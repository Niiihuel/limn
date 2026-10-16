// Wire as a child of wayland::tests::offers::runtime to reuse its real fixture.
use super::*;
use crate::receive::{
    control::AttemptGate,
    runtime::spawn_owner,
    worker_pool::{OwnerId, SurfaceLease},
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{OnceLock, Weak},
};

struct DisplayOwner {
    _connection: Connection,
}

struct RegisteredWindow {
    _connection: Connection,
    _surface: client::protocol::wl_surface::WlSurface,
}

struct LiveOwnerCase {
    fixture: Option<Fixture>,
    pool: Arc<WorkerPool>,
    owner: OwnerId,
    lifecycle: ReceiveControl,
    ping: Ping,
    observed: testing::Observations,
    display_owner: Weak<DisplayOwner>,
    closed_wakes: Arc<AtomicUsize>,
}

impl LiveOwnerCase {
    fn new(hooks: testing::Hooks) -> Self {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let pool = WorkerPool::isolated();
        let lifecycle = control();
        // This owner deliberately holds no Arc<RegisteredWindow>.
        let owner = Arc::new(DisplayOwner {
            _connection: fixture.host.clone(),
        });
        let display_owner = Arc::downgrade(&owner);
        let observed = hooks.observed.clone();
        let closed_wakes = Arc::new(AtomicUsize::new(0));
        let callback_closed = closed_wakes.clone();
        let callback_control = lifecycle.clone();
        let (owner, ping) = testing::with_hooks(hooks, || {
            // The retained display owner and fixture preserve the actual native
            // connection and coordinated host reader through cleanup and join.
            unsafe {
                spawn_owner(
                    pool.clone(),
                    fixture.host.backend().display_ptr() as usize,
                    owner,
                    lifecycle.clone(),
                    ReceiveLimits::default(),
                    Box::new(move || {
                        if matches!(callback_control.status(), ReceiveStatus::Closed(_)) {
                            callback_closed.fetch_add(1, Ordering::SeqCst);
                        }
                    }),
                )
            }
        })
        .expect("the real shared-owner runtime is admitted");
        Self {
            fixture: Some(fixture),
            pool,
            owner,
            lifecycle,
            ping,
            observed,
            display_owner,
            closed_wakes,
        }
    }

    fn fixture(&self) -> &Fixture {
        self.fixture.as_ref().unwrap()
    }
    fn fixture_mut(&mut self) -> &mut Fixture {
        self.fixture.as_mut().unwrap()
    }

    fn drive(&mut self, done: impl FnMut(&Fixture) -> bool) {
        super::drive(self.fixture.as_mut().unwrap(), done);
    }

    fn key(&self, surface: usize) -> SurfaceKey {
        SurfaceKey {
            display: self.fixture().host.backend().display_ptr() as usize,
            surface: self.fixture().host_surfaces[surface].id().as_ptr() as usize,
        }
    }

    fn register(
        &self,
        surface: usize,
        wake: impl Fn() + Send + 'static,
    ) -> (Arc<SurfaceLease>, Weak<RegisteredWindow>) {
        let window = Arc::new(RegisteredWindow {
            _connection: self.fixture().host.clone(),
            _surface: self.fixture().host_surfaces[surface].clone(),
        });
        let weak = Arc::downgrade(&window);
        let lease = self
            .pool
            .reserve_surface(self.owner, self.key(surface), window, Box::new(wake))
            .unwrap();
        self.ping.ping();
        (Arc::new(lease), weak)
    }

    fn start(
        &mut self,
        surface: usize,
        offer_key: usize,
        control: &ReceiveControl,
    ) -> (OfferId, OwnedFd) {
        let epoch = control.header().epoch;
        let point = SurfacePoint {
            x: 10.0 + surface as f64,
            y: 20.0 + surface as f64,
        };
        let offer = self.fixture().offer(0, offer_key, "text/uri-list");
        offer.action(wl_data_device_manager::DndAction::Copy);
        self.fixture().enter(0, &offer, surface, point.x, point.y);
        self.fixture().server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        let mut events = Vec::new();
        let ping = self.ping.clone();
        self.drive(|fixture| {
            events.extend(poll(control, &ping));
            fixture.received(offer_key) == 1
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
        assert_eq!(started.len(), 1);
        let (id, actual_epoch, actual_point) = started[0];
        assert_eq!(actual_epoch, epoch);
        assert_eq!(actual_point, point);
        assert!(events.iter().all(|event| matches!(event,
            ReceiveEvent::Hovered { id: actual, .. } | ReceiveEvent::DropStarted { id: actual, .. } if *actual == id)));
        (id, self.fixture_mut().take_writer(offer_key))
    }

    fn close(&mut self) {
        self.pool.close_owner(self.owner, None);
        self.ping.ping();
        let lifecycle = self.lifecycle.clone();
        self.drive(|_| matches!(lifecycle.status(), ReceiveStatus::Closed(_)));
    }
}

impl Drop for LiveOwnerCase {
    fn drop(&mut self) {
        self.pool.close_owner(self.owner, None);
        self.ping.ping();
        let deadline = Instant::now() + WAIT;
        while !matches!(self.lifecycle.status(), ReceiveStatus::Closed(_))
            && !self.observed.cleanup_failed.load(Ordering::SeqCst)
            && Instant::now() < deadline
        {
            let progress = catch_unwind(AssertUnwindSafe(|| {
                self.fixture_mut().serve();
                self.fixture().host_read();
            }));
            if progress.is_err() {
                break;
            }
            thread::yield_now();
        }
        if !matches!(self.lifecycle.status(), ReceiveStatus::Closed(_)) {
            // Unknown SDK cleanup must not destroy this foreign fixture on unwind.
            if let Some(fixture) = self.fixture.take() {
                std::mem::forget(fixture);
            }
        }
    }
}

fn watched_fd(observed: &testing::Observations, id: OfferId) -> Weak<OwnedFd> {
    let mut result = None;
    wait(
        || {},
        || {
            if let Some((actual, fd)) = observed.watched_pipe.lock().unwrap().as_ref() {
                if *actual == id && fd.upgrade().is_some() {
                    result = Some(fd.clone());
                }
            }
            result.is_some()
        },
    );
    result.unwrap()
}

#[derive(Debug)]
struct RetirementWake {
    calls: u8,
    thread: ThreadId,
    detached: bool,
    old_fd_gone: bool,
    exact_window_gone: bool,
    owner_status: ReceiveStatus,
}

#[test]
fn shared_runtime_removes_the_actual_watcher_and_window_before_detached_wake_then_b_uses_the_same_device(
) {
    let observed = testing::Observations::default();
    let (entered, retirement_entered) = mpsc::sync_channel(1);
    let (release, retirement_release) = mpsc::sync_channel(1);
    let mut case = LiveOwnerCase::new(testing::Hooks {
        observed: observed.clone(),
        before_retire_sync: Some((entered, retirement_release)),
        ..Default::default()
    });
    let mut release = OpenOnDrop(Some(release));
    let lifecycle = case.lifecycle.clone();
    case.drive(|_| lifecycle.status() == ReceiveStatus::Ready);
    let witness = Arc::new(OnceLock::<(Arc<SurfaceLease>, Weak<RegisteredWindow>)>::new());
    let old_fd = Arc::new(OnceLock::<Weak<OwnedFd>>::new());
    let callback_witness = witness.clone();
    let callback_fd = old_fd.clone();
    let callback_lifecycle = case.lifecycle.clone();
    let (wake, retired) = mpsc::channel();
    let calls = Cell::new(0_u8); // Fn + Send, intentionally not Sync.
    let (a, a_window) = case.register(0, move || {
        let Some((lease, window)) = callback_witness.get() else {
            return;
        };
        if lease.is_detached() {
            calls.set(calls.get() + 1);
            let _ = wake.send(RetirementWake {
                calls: calls.get(),
                thread: thread::current().id(),
                detached: true,
                old_fd_gone: callback_fd.get().is_some_and(|fd| fd.upgrade().is_none()),
                exact_window_gone: window.upgrade().is_none(),
                owner_status: callback_lifecycle.status(),
            });
        }
    });
    assert!(witness.set((a.clone(), a_window.clone())).is_ok());
    let (b, b_window) = case.register(1, || {});
    let a_control = a.control();
    let b_control = b.control();
    case.drive(|_| {
        a_control.status() == ReceiveStatus::Ready && b_control.status() == ReceiveStatus::Ready
    });
    let (a_id, writer_a) = case.start(0, 1, &a_control);
    let a_watcher_fd = watched_fd(&observed, a_id);
    assert!(old_fd.set(a_watcher_fd.clone()).is_ok());
    a.request_retire();
    case.ping.ping();
    retirement_entered
        .recv_timeout(WAIT)
        .expect("the actual worker pauses after pump and before watcher retirement");
    assert!(!a.is_detached());
    assert!(
        a_watcher_fd.upgrade().is_some(),
        "the obsolete Generic still owns the exact read FD"
    );
    assert!(
        a_window.upgrade().is_some(),
        "the pool retains this exact registered window until the watcher is gone"
    );
    assert!(matches!(
        case.pool
            .reserve_surface(case.owner, case.key(0), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Busy)
    ));
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    release.release();
    let final_wake = retired
        .recv_timeout(WAIT)
        .expect("A's final wake observes actual retirement");
    assert_eq!(final_wake.calls, 1);
    assert_ne!(final_wake.thread, thread::current().id());
    assert!(final_wake.detached);
    assert!(final_wake.old_fd_gone);
    assert!(final_wake.exact_window_gone);
    assert_eq!(final_wake.owner_status, ReceiveStatus::Ready);
    assert!(matches!(
        retired.recv_timeout(WAIT),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));

    // A's source writer stays open throughout actual watcher/window retirement.
    let (b_id, writer_b) = case.start(1, 2, &b_control);
    let b_watcher_fd = watched_fd(&observed, b_id);
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));
    let ping = case.ping.clone();
    assert!(
        matches!(poll(&a_control, &ping).as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == a_id)
    );
    let before_ack = observed.completed_pumps.load(Ordering::SeqCst);
    case.drive(|_| observed.completed_pumps.load(Ordering::SeqCst) > before_ack);
    let after_ack_fd = watched_fd(&observed, b_id);
    assert!(Weak::ptr_eq(&b_watcher_fd, &after_ack_fd));
    assert!(b_watcher_fd.upgrade().is_some());
    assert!(poll(&b_control, &ping).is_empty());
    {
        let header = b_control.header();
        let current = header.offer.unwrap();
        assert_eq!(current.id, b_id);
        assert!(matches!(current.gate, AttemptGate::Open));
    }

    let (fresh_a, _) = case.register(0, || {});
    let fresh_a_control = fresh_a.control();
    case.drive(|_| fresh_a_control.status() == ReceiveStatus::Ready);
    let busy = case.fixture().offer(0, 3, "text/uri-list");
    busy.action(wl_data_device_manager::DndAction::Copy);
    case.fixture().enter(0, &busy, 0, 50.0, 60.0);
    case.fixture().server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    case.drive(|fixture| {
        fixture
            .server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Destroy(3)))
    });
    assert!(poll(&fresh_a_control, &ping).is_empty());
    assert_eq!(case.fixture().received(3), 0);
    assert!(b_watcher_fd.upgrade().is_some());
    assert!(Weak::ptr_eq(&b_watcher_fd, &watched_fd(&observed, b_id)));
    assert!(poll(&b_control, &ping).is_empty());

    write_chunk(&writer_b, DATA);
    drop(writer_b);
    let mut ready = Vec::new();
    case.drive(|_| {
        ready.extend(poll(&b_control, &ping));
        ready
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    assert!(
        matches!(ready.as_slice(), [ReceiveEvent::DropReady { id, paths }]
        if *id == b_id && *paths == vec![std::path::PathBuf::from("/tmp/received")])
    );
    b_control.decide(b_id, DropDecision::Accept).unwrap();
    case.ping.ping();
    let mut ended = Vec::new();
    case.drive(|_| {
        ended.extend(poll(&b_control, &ping));
        ended
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
    });
    assert!(
        matches!(ended.as_slice(), [ReceiveEvent::DropEnded { id, result: Ok(()) }] if *id == b_id)
    );
    case.drive(|fixture| fixture.finish_count(2) == 1);
    assert_eq!(case.fixture().finish_count(1), 0);
    assert_eq!(case.fixture().finish_count(3), 0);
    assert_eq!(case.fixture().server.devices, 1);
    drop(writer_a);
    case.close();
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Closed(None));
    assert!(case.display_owner.upgrade().is_none());
    assert!(b_window.upgrade().is_none());
    assert!(observed.cleaned.load(Ordering::SeqCst));
    host_still_works(case.fixture_mut());
}

#[test]
fn uncertain_shared_cleanup_with_a_live_pipe_retains_detaching_and_display_exclusion() {
    let observed = testing::Observations::default();
    let mut case = LiveOwnerCase::new(testing::Hooks {
        observed: observed.clone(),
        panic_cleanup: true,
        ..Default::default()
    });
    let lifecycle = case.lifecycle.clone();
    case.drive(|_| lifecycle.status() == ReceiveStatus::Ready);
    let detached_wakes = Arc::new(AtomicUsize::new(0));
    let callback_detached = detached_wakes.clone();
    let panic_armed = Arc::new(AtomicBool::new(false));
    let callback_panic = panic_armed.clone();
    let wake_panics = Arc::new(AtomicUsize::new(0));
    let callback_panics = wake_panics.clone();
    let lease_ref = Arc::new(OnceLock::<Arc<SurfaceLease>>::new());
    let callback_lease = lease_ref.clone();
    let (a, a_window) = case.register(0, move || {
        if let Some(lease) = callback_lease.get() {
            if lease.is_detached() {
                callback_detached.fetch_add(1, Ordering::SeqCst);
            }
            let ended = lease
                .control()
                .header()
                .offer
                .is_some_and(|offer| matches!(offer.gate, AttemptGate::Ended { .. }));
            if ended && callback_panic.swap(false, Ordering::SeqCst) {
                callback_panics.fetch_add(1, Ordering::SeqCst);
                // This legitimate wake interrupts reconciliation before it can
                // drop the Guest pipe. It does not drain the terminal mailbox.
                panic!("private terminal wake interrupts before offer cleanup");
            }
        }
    });
    assert!(lease_ref.set(a.clone()).is_ok());
    let a_control = a.control();
    case.drive(|_| a_control.status() == ReceiveStatus::Ready);
    let (a_id, writer) = case.start(0, 1, &a_control);
    let fd = watched_fd(&observed, a_id);
    assert!(fd.upgrade().is_some());
    panic_armed.store(true, Ordering::SeqCst);
    // Cut parent and endpoint admission together before requesting another pump.
    // A pre-pump exit keeps the live pipe directly. If a pump already started,
    // the one armed terminal wake interrupts before cleanup_offers can drop it.
    // Both paths then reach the existing bounded cleanup panic hook.
    case.pool.close_owner(case.owner, None);
    case.ping.ping();
    wait(|| {}, || observed.cleanup_failed.load(Ordering::SeqCst));
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Closing);
    assert_eq!(a_control.status(), ReceiveStatus::Closing);
    assert!(!a.is_detached());
    assert!(!observed.cleaned.load(Ordering::SeqCst));
    assert!(
        fd.upgrade().is_some(),
        "the retained uncertain guest still owns the exact pipe FD"
    );
    assert!(a_window.upgrade().is_some());
    assert!(case.display_owner.upgrade().is_some());
    assert!(matches!(
        case.pool.reserve_owner(
            case.key(0).display,
            Arc::new(()),
            control(),
            Duration::from_secs(5)
        ),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        case.pool.reserve_legacy(case.key(0)),
        Err(ReceiveError::Busy)
    ));
    assert_eq!(detached_wakes.load(Ordering::SeqCst), 0);
    assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
    assert!(wake_panics.load(Ordering::SeqCst) <= 1);
    drop(writer);
    host_sync(case.fixture_mut());
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Closing);
    assert!(!a.is_detached());
    assert!(fd.upgrade().is_some());
    assert!(case.display_owner.upgrade().is_some());
    assert_eq!(detached_wakes.load(Ordering::SeqCst), 0);
    assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
    // LiveOwnerCase retains the real fixture instead of destroying unconfirmed IO.
}
