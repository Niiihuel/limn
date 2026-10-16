use super::*;
use crate::{
    receive::{
        runtime::{spawn_owned, testing},
        worker_pool::{SurfaceKey, WorkerPool},
    },
    DropDecision,
};
use sctk::reexports::calloop::ping::Ping;
use std::{
    cell::Cell,
    sync::mpsc,
    thread::{self, ThreadId},
};

const WAIT: Duration = Duration::from_secs(3);
const DATA: &[u8] = b"file:///tmp/received\r\n";

struct HostOwner {
    _connection: Connection,
    _surface: client::protocol::wl_surface::WlSurface,
    dropped: Option<mpsc::Sender<(ThreadId, bool)>>,
    observed: testing::Observations,
}

impl Drop for HostOwner {
    fn drop(&mut self) {
        if let Some(sent) = &self.dropped {
            let _ = sent.send((
                thread::current().id(),
                self.observed.cleaned.load(Ordering::SeqCst),
            ));
        }
    }
}

fn key(fixture: &Fixture) -> SurfaceKey {
    SurfaceKey {
        display: fixture.host.backend().display_ptr() as usize,
        surface: fixture.host_surfaces[0].id().as_ptr() as usize,
    }
}

fn owner(fixture: &Fixture, observed: testing::Observations) -> Arc<HostOwner> {
    Arc::new(HostOwner {
        _connection: fixture.host.clone(),
        _surface: fixture.host_surfaces[0].clone(),
        dropped: None,
        observed,
    })
}

fn spawn(
    fixture: &Fixture,
    pool: &Arc<WorkerPool>,
    control: &ReceiveControl,
    limits: ReceiveLimits,
    hooks: testing::Hooks,
    wake: impl Fn() + Send + 'static,
) -> Ping {
    let owner = owner(fixture, hooks.observed.clone());
    testing::with_hooks(hooks, || {
        // The retained owner owns the exact display and surface used by this fixture.
        unsafe {
            spawn_owned(
                pool.clone(),
                key(fixture),
                owner,
                control.clone(),
                limits,
                Box::new(wake),
            )
        }
    })
    .expect("the owned receive runtime is available")
}

// An assertion cannot leave a test receiver running or a dispatch gate closed.
struct StopOnDrop(ReceiveControl, Ping);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let _ = self.0.close();
        self.1.ping();
    }
}

struct OpenOnDrop(Option<mpsc::SyncSender<()>>);

impl OpenOnDrop {
    fn release(&mut self) {
        if let Some(sent) = self.0.take() {
            let _ = sent.try_send(());
        }
    }
}

impl Drop for OpenOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

fn wait(mut step: impl FnMut(), mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    loop {
        step();
        if done() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "private runtime made no progress"
        );
        thread::yield_now();
    }
}

fn drive(fixture: &mut Fixture, mut done: impl FnMut(&Fixture) -> bool) {
    let deadline = Instant::now() + WAIT;
    loop {
        fixture.serve();
        fixture.host_read();
        if done(fixture) {
            return;
        }
        assert!(Instant::now() < deadline, "private host made no progress");
        thread::yield_now();
    }
}

fn poll(control: &ReceiveControl, ping: &Ping) -> Vec<ReceiveEvent> {
    let events = control.poll();
    if events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    {
        ping.ping();
    }
    events
}

fn terminal(control: &ReceiveControl, ping: &Ping, id: OfferId, error: ReceiveError) {
    let mut events = Vec::new();
    wait(
        || {},
        || {
            events.extend(poll(control, ping));
            events
                .iter()
                .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
        },
    );
    assert!(matches!(
        events.as_slice(),
        [ReceiveEvent::DropEnded { id: actual, result: Err(actual_error) }]
        if *actual == id && *actual_error == error
    ));
    assert!(poll(control, ping).is_empty());
}

fn stop(control: &ReceiveControl, ping: &Ping) {
    control.close().unwrap();
    ping.ping();
    wait(
        || {},
        || matches!(control.status(), ReceiveStatus::Closed(_)),
    );
}

fn start_copy(
    fixture: &mut Fixture,
    control: &ReceiveControl,
    ping: &Ping,
) -> (wl_data_offer::WlDataOffer, OfferId, OwnedFd) {
    let offer = fixture.offer(0, 1, "text/uri-list");
    offer.offer("text/plain".into());
    fixture.enter(0, &offer, 0, 10.0, 20.0);
    offer.action(wl_data_device_manager::DndAction::Copy);
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    let mut events = Vec::new();
    drive(fixture, |fixture| {
        events.extend(poll(control, ping));
        fixture.received(1) == 1
            && events
                .iter()
                .any(|event| matches!(event, ReceiveEvent::DropStarted { .. }))
    });
    let started: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ReceiveEvent::DropStarted { id, epoch, point } => Some((*id, *epoch, *point)),
            _ => None,
        })
        .collect();
    assert_eq!(started.len(), 1, "expected exact Started: {events:?}");
    let (id, epoch, point) = started[0];
    assert_eq!(epoch, ReceiveEpoch(17));
    assert_eq!(point, SurfacePoint { x: 10.0, y: 20.0 });
    assert!(events.iter().all(|event| matches!(
        event,
        ReceiveEvent::Hovered { id: actual, .. } | ReceiveEvent::DropStarted { id: actual, .. }
        if *actual == id
    )));
    (offer, id, fixture.take_writer(1))
}

fn write_chunk(writer: &OwnedFd, mut bytes: &[u8]) {
    assert!(bytes.len() <= 1024);
    while !bytes.is_empty() {
        let count = rustix::io::write(writer, bytes).unwrap();
        assert!(count > 0);
        bytes = &bytes[count..];
    }
}

#[derive(Default)]
struct HostSync(bool);

impl client::Dispatch<client::protocol::wl_callback::WlCallback, ()> for HostSync {
    fn event(
        state: &mut Self,
        _: &client::protocol::wl_callback::WlCallback,
        _: client::protocol::wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &client::QueueHandle<Self>,
    ) {
        state.0 = true;
    }
}

fn host_still_works(fixture: &mut Fixture) {
    // Flush the shared host only after Closed, then observe the real protocol
    // destructors. An internal cleanup flag alone does not establish teardown.
    fixture.host.flush().unwrap();
    fixture.serve();
    assert_eq!(fixture.server.released_devices, fixture.server.devices);
    assert_eq!(
        fixture.server.released_seats,
        fixture.server.seat_versions.len()
    );
    assert!(fixture
        .server
        .device_resources
        .iter()
        .all(|device| !device.is_alive()));
    host_sync(fixture);
}

fn host_sync(fixture: &mut Fixture) {
    let mut queue = fixture.host.new_event_queue::<HostSync>();
    let mut state = HostSync::default();
    fixture.host.display().sync(&queue.handle(), ());
    fixture.host.flush().unwrap();
    drive(fixture, |_| {
        queue.dispatch_pending(&mut state).unwrap();
        state.0
    });
}

fn registry_count(fixture: &Fixture) -> usize {
    let handle = fixture.display.handle().backend_handle();
    let client = fixture.server.surfaces[0].client().unwrap().id();
    let mut count = 0;
    handle
        .with_all_objects_for(client, |id| {
            if id.interface().name == "wl_registry" {
                count += 1;
            }
        })
        .unwrap();
    count
}

#[test]
fn closing_from_the_ready_wake_preserves_a_normal_close() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let pool = WorkerPool::isolated();
    let control = control();
    let closing_control = control.clone();
    let ready_wakes = wakes();
    let counted = ready_wakes.clone();
    let observed = testing::Observations::default();
    let ping = spawn(
        &fixture,
        &pool,
        &control,
        ReceiveLimits::default(),
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        move || {
            if closing_control.status() == ReceiveStatus::Ready {
                counted.fetch_add(1, Ordering::SeqCst);
                closing_control.close().unwrap();
            }
        },
    );
    let _stop = StopOnDrop(control.clone(), ping);
    drive(&mut fixture, |_| {
        matches!(control.status(), ReceiveStatus::Closed(_))
    });
    assert_eq!(ready_wakes.load(Ordering::SeqCst), 1);
    assert!(observed.cleaned.load(Ordering::SeqCst));
    assert_eq!(control.status(), ReceiveStatus::Closed(None));
    host_still_works(&mut fixture);
}

#[test]
fn the_runtime_needs_its_host_reader_and_a_paused_host_expires() {
    for resume in [false, true] {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let pool = WorkerPool::isolated();
        let control = control();
        let observed = testing::Observations::default();
        let wakes = wakes();
        let callback_wakes = wakes.clone();
        let ping = spawn(
            &fixture,
            &pool,
            &control,
            ReceiveLimits {
                startup_timeout: if resume {
                    Duration::from_secs(5)
                } else {
                    Duration::from_millis(30)
                },
                ..ReceiveLimits::default()
            },
            testing::Hooks {
                observed: observed.clone(),
                ..Default::default()
            },
            move || {
                callback_wakes.fetch_add(1, Ordering::SeqCst);
            },
        );
        let _stop = StopOnDrop(control.clone(), ping.clone());
        if resume {
            // Serve requests, but do not read the host display yet.
            wait(
                || fixture.serve(),
                || observed.completed_pumps.load(Ordering::SeqCst) >= 2,
            );
            assert_eq!(control.status(), ReceiveStatus::Starting);
            assert!(fixture.server.manager_versions.is_empty());
            drive(&mut fixture, |_| control.status() == ReceiveStatus::Ready);
            stop(&control, &ping);
            assert_eq!(control.status(), ReceiveStatus::Closed(None));
        } else {
            wait(
                || fixture.serve(),
                || matches!(control.status(), ReceiveStatus::Closed(_)),
            );
            assert_eq!(
                control.status(),
                ReceiveStatus::Closed(Some(ReceiveError::Timeout))
            );
        }
        assert!(observed.cleaned.load(Ordering::SeqCst));
        assert!(wakes.load(Ordering::SeqCst) > 0);
        host_still_works(&mut fixture);
    }

    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let pool = WorkerPool::isolated();
    for _ in 0..64 {
        pool.consume_registry().unwrap();
    }
    assert_eq!(pool.consume_registry(), Err(ReceiveError::ResourceLimit));
    let before = registry_count(&fixture);
    let control = control();
    let observed = testing::Observations::default();
    // Reservation/spawn still succeeds: the credit belongs to the actual request.
    let ping = spawn(
        &fixture,
        &pool,
        &control,
        ReceiveLimits::default(),
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        || {},
    );
    let _stop = StopOnDrop(control.clone(), ping.clone());
    drive(&mut fixture, |_| {
        matches!(control.status(), ReceiveStatus::Closed(_))
    });
    assert_eq!(
        control.status(),
        ReceiveStatus::Closed(Some(ReceiveError::ResourceLimit))
    );
    fixture.host.flush().unwrap();
    fixture.serve();
    assert_eq!(registry_count(&fixture), before);
    assert!(fixture.server.manager_versions.is_empty());
    assert!(observed.cleaned.load(Ordering::SeqCst));
    host_still_works(&mut fixture);
}

#[test]
fn level_readiness_drains_more_than_one_chunk_but_waits_for_eof_and_accept() {
    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let pool = WorkerPool::isolated();
    let control = control();
    let observed = testing::Observations::default();
    let ping = spawn(
        &fixture,
        &pool,
        &control,
        ReceiveLimits::default(),
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        || {},
    );
    let _stop = StopOnDrop(control.clone(), ping.clone());
    drive(&mut fixture, |_| control.status() == ReceiveStatus::Ready);
    // A Ready receiver already spent one credit and needs no registry per drop.
    for _ in 0..63 {
        pool.consume_registry().unwrap();
    }
    assert_eq!(pool.consume_registry(), Err(ReceiveError::ResourceLimit));
    let (_offer, id, writer) = start_copy(&mut fixture, &control, &ping);
    let expected: Vec<_> = (0..24)
        .map(|i| format!("/tmp/{i:02}-{}", "x".repeat(512)))
        .collect();
    let bytes = expected
        .iter()
        .map(|path| format!("file://{path}\r\n"))
        .collect::<String>()
        .into_bytes();
    assert!(bytes.len() > 3 * 4096);
    for chunk in bytes.chunks(1024) {
        let before = observed.pipe_callbacks.load(Ordering::SeqCst);
        write_chunk(&writer, chunk);
        drive(&mut fixture, |_| {
            observed.pipe_callbacks.load(Ordering::SeqCst) > before
        });
        // The callback can precede its pump's read. Wait for a subsequent pump
        // before writing another bounded chunk into the blocking source pipe.
        let after_callback = observed.completed_pumps.load(Ordering::SeqCst);
        drive(&mut fixture, |_| {
            observed.completed_pumps.load(Ordering::SeqCst) > after_callback
        });
        assert!(poll(&control, &ping).is_empty());
    }
    assert!(observed.pipe_callbacks.load(Ordering::SeqCst) > 3);
    assert_eq!(fixture.finish_count(1), 0);
    drop(writer);
    let mut events = Vec::new();
    drive(&mut fixture, |_| {
        events.extend(poll(&control, &ping));
        events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    match events.as_slice() {
        [ReceiveEvent::DropReady { id: actual, paths }] => {
            assert_eq!(*actual, id);
            assert_eq!(
                paths,
                &expected
                    .iter()
                    .map(std::path::PathBuf::from)
                    .collect::<Vec<_>>()
            );
        }
        _ => panic!("expected the complete ordered batch: {events:?}"),
    }
    assert_eq!(fixture.finish_count(1), 0);
    control.decide(id, DropDecision::Accept).unwrap();
    ping.ping();
    let mut events = Vec::new();
    drive(&mut fixture, |_| {
        events.extend(poll(&control, &ping));
        events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Ok(()) }] if *actual == id)
    );
    drive(&mut fixture, |fixture| fixture.finish_count(1) == 1);
    assert!(poll(&control, &ping).is_empty());
    stop(&control, &ping);
}

#[test]
fn ping_controls_cancel_an_open_writer_without_compositor_or_new_bytes() {
    for operation in ["cancel", "reject", "close"] {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let pool = WorkerPool::isolated();
        let control = control();
        let observed = testing::Observations::default();
        let ping = spawn(
            &fixture,
            &pool,
            &control,
            ReceiveLimits::default(),
            testing::Hooks {
                observed: observed.clone(),
                ..Default::default()
            },
            || {},
        );
        let _stop = StopOnDrop(control.clone(), ping.clone());
        drive(&mut fixture, |_| control.status() == ReceiveStatus::Ready);
        let (_offer, id, writer) = start_copy(&mut fixture, &control, &ping);
        write_chunk(&writer, &DATA[..8]);
        drive(&mut fixture, |_| {
            observed.pipe_callbacks.load(Ordering::SeqCst) > 0
        });
        assert!(poll(&control, &ping).is_empty());
        match operation {
            "cancel" => control.cancel(id).unwrap(),
            "reject" => control.decide(id, DropDecision::Reject).unwrap(),
            "close" => control.close().unwrap(),
            _ => unreachable!(),
        }
        ping.ping();
        // No server dispatch or host read supplies a protocol event after this point.
        terminal(&control, &ping, id, ReceiveError::Cancelled);
        drop(writer);
        stop(&control, &ping);
        fixture.serve();
        assert_eq!(fixture.finish_count(1), 0);
        assert!(poll(&control, &ping).is_empty());
        host_still_works(&mut fixture);
    }
}

struct WakeDrop(mpsc::Sender<(ThreadId, ReceiveStatus)>, ReceiveControl);

impl Drop for WakeDrop {
    fn drop(&mut self) {
        let _ = self.0.send((thread::current().id(), self.1.status()));
    }
}

#[test]
fn startup_or_wake_panic_keeps_the_send_only_wake_until_guest_cleanup() {
    for panic_startup in [false, true] {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let pool = WorkerPool::isolated();
        let control = control();
        let observed = testing::Observations::default();
        let (dropped_tx, dropped_rx) = mpsc::channel();
        let drop_probe = WakeDrop(dropped_tx, control.clone());
        let (called_tx, called_rx) = mpsc::channel();
        let callback_control = control.clone();
        // Cell deliberately makes this Fn + Send closure non-Sync.
        let calls = Cell::new(0);
        let ping = spawn(
            &fixture,
            &pool,
            &control,
            ReceiveLimits::default(),
            testing::Hooks {
                observed: observed.clone(),
                panic_startup,
                ..Default::default()
            },
            move || {
                let _keep_until_wake_drop = &drop_probe;
                let next = calls.get() + 1;
                calls.set(next);
                if !panic_startup && next == 1 {
                    panic!("private first wake panic");
                }
                let _ = called_tx.send(callback_control.status());
            },
        );
        let _stop = StopOnDrop(control.clone(), ping.clone());
        drive(&mut fixture, |_| {
            matches!(control.status(), ReceiveStatus::Closed(_))
        });
        assert_eq!(
            control.status(),
            ReceiveStatus::Closed(Some(ReceiveError::Io))
        );
        assert!(observed.cleaned.load(Ordering::SeqCst));
        let (drop_thread, at_drop) = dropped_rx
            .recv_timeout(WAIT)
            .expect("the recovered wake is dropped by the reaper");
        assert_ne!(drop_thread, thread::current().id());
        assert_eq!(at_drop, ReceiveStatus::Closed(Some(ReceiveError::Io)));
        assert!(called_rx
            .try_iter()
            .any(|status| status == ReceiveStatus::Closed(Some(ReceiveError::Io))));
        host_still_works(&mut fixture);
    }

    let mut fixture = Fixture::new(Some(3), &[5]);
    fixture.create_surfaces();
    let pool = WorkerPool::isolated();
    let control = control();
    let observed = testing::Observations::default();
    let (owner_tx, owner_rx) = mpsc::channel();
    let lease = Arc::new(HostOwner {
        _connection: fixture.host.clone(),
        _surface: fixture.host_surfaces[0].clone(),
        dropped: Some(owner_tx),
        observed: observed.clone(),
    });
    let weak = Arc::downgrade(&lease);
    let (wake_drop_tx, wake_drop_rx) = mpsc::channel();
    let wake_drop = WakeDrop(wake_drop_tx, control.clone());
    let closed_wakes = Arc::new(AtomicUsize::new(0));
    let callback_closed = closed_wakes.clone();
    let callback_control = control.clone();
    let ping = testing::with_hooks(
        testing::Hooks {
            observed: observed.clone(),
            panic_cleanup: true,
            ..Default::default()
        },
        || {
            // The owner remains the live host lease if cleanup cannot be confirmed.
            unsafe {
                spawn_owned(
                    pool.clone(),
                    key(&fixture),
                    lease,
                    control.clone(),
                    ReceiveLimits::default(),
                    Box::new(move || {
                        let _keep_same_wake = &wake_drop;
                        if matches!(callback_control.status(), ReceiveStatus::Closed(_)) {
                            callback_closed.fetch_add(1, Ordering::SeqCst);
                        }
                    }),
                )
            }
        },
    )
    .expect("cleanup-failure fixture starts the actual runtime");
    let _stop = StopOnDrop(control.clone(), ping.clone());
    drive(&mut fixture, |_| control.status() == ReceiveStatus::Ready);
    control.close().unwrap();
    ping.ping();
    wait(|| {}, || observed.cleanup_failed.load(Ordering::SeqCst));
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(!observed.cleaned.load(Ordering::SeqCst));
    assert!(weak.upgrade().is_some());
    assert!(matches!(
        owner_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert!(matches!(
        wake_drop_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    let other_control = super::super::control();
    assert_eq!(
        // Uncertain cleanup cannot reopen the same surface or release its lease.
        unsafe {
            spawn_owned(
                pool.clone(),
                key(&fixture),
                owner(&fixture, Default::default()),
                other_control,
                ReceiveLimits::default(),
                Box::new(|| {}),
            )
        }
        .unwrap_err(),
        ReceiveError::Busy
    );
    host_sync(&mut fixture);
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert_eq!(closed_wakes.load(Ordering::SeqCst), 0);
    assert!(weak.upgrade().is_some());
    assert!(matches!(
        wake_drop_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
}

#[test]
fn a_dispatch_gate_keeps_the_owner_surface_and_four_slots_until_real_exit() {
    let pool = WorkerPool::isolated();
    let mut fixtures = Vec::new();
    let mut controls = Vec::new();
    let mut pings = Vec::new();
    let mut stops = Vec::new();
    let mut gates = Vec::new();
    let mut owners = Vec::new();
    let mut drops = Vec::new();
    let mut observations = Vec::new();
    for _ in 0..4 {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let control = control();
        let observed = testing::Observations::default();
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let gate = OpenOnDrop(Some(release_tx));
        let (drop_tx, drop_rx) = mpsc::channel();
        let owner = Arc::new(HostOwner {
            _connection: fixture.host.clone(),
            _surface: fixture.host_surfaces[0].clone(),
            dropped: Some(drop_tx),
            observed: observed.clone(),
        });
        let weak = Arc::downgrade(&owner);
        let ping = testing::with_hooks(
            testing::Hooks {
                observed: observed.clone(),
                before_dispatch: Some((entered_tx, release_rx)),
                ..Default::default()
            },
            || {
                // Each owner retains this exact private display and target surface.
                unsafe {
                    spawn_owned(
                        pool.clone(),
                        key(&fixture),
                        owner,
                        control.clone(),
                        ReceiveLimits::default(),
                        Box::new(|| {}),
                    )
                }
            },
        )
        .expect("a free runtime slot is available");
        stops.push(StopOnDrop(control.clone(), ping.clone()));
        entered_rx
            .recv_timeout(WAIT)
            .expect("worker reached its SDK dispatch gate");
        fixtures.push(fixture);
        controls.push(control);
        pings.push(ping);
        gates.push(gate);
        owners.push(weak);
        drops.push(drop_rx);
        observations.push(observed);
    }
    controls[0].close().unwrap();
    pings[0].ping();
    assert_eq!(controls[0].status(), ReceiveStatus::Closing);
    assert!(owners[0].upgrade().is_some());
    assert!(matches!(
        drops[0].try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    let mut fifth = Fixture::new(Some(3), &[5]);
    fifth.create_surfaces();
    let fifth_registries = registry_count(&fifth);
    for fixture in [&fixtures[0], &fifth] {
        let control = control();
        // Rejected attachment never transfers ownership of any foreign display.
        assert_eq!(
            unsafe {
                spawn_owned(
                    pool.clone(),
                    key(fixture),
                    owner(fixture, Default::default()),
                    control.clone(),
                    ReceiveLimits::default(),
                    Box::new(|| {}),
                )
            }
            .unwrap_err(),
            ReceiveError::Busy
        );
        assert_eq!(control.status(), ReceiveStatus::Starting);
    }
    fifth.serve();
    assert_eq!(registry_count(&fifth), fifth_registries);
    for (control, ping) in controls.iter().zip(&pings) {
        control.close().unwrap();
        ping.ping();
    }
    for gate in &mut gates {
        gate.release();
    }
    for (index, control) in controls.iter().enumerate() {
        wait(
            || {},
            || matches!(control.status(), ReceiveStatus::Closed(_)),
        );
        assert!(observations[index].gate_released.load(Ordering::SeqCst));
        assert!(owners[index].upgrade().is_none());
        let (drop_thread, cleaned) = drops[index].recv_timeout(WAIT).unwrap();
        assert_ne!(drop_thread, thread::current().id());
        assert!(cleaned, "owner cannot be released before guest teardown");
        host_still_works(&mut fixtures[index]);
    }
}

#[path = "owner_runtime_tests.rs"]
mod shared_owner;
