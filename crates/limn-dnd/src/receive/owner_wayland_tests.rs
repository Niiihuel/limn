use super::*;
use crate::receive::{
    clipboard_control::ClipboardControl,
    control::AttemptGate,
    read_tag::ReadTag,
    worker_pool::{OwnerId, Reservation, SurfaceKey, SurfaceLease, WorkerPool},
};
use crate::DropDecision;
use std::panic::{catch_unwind, AssertUnwindSafe};

impl GuestStartup {
    fn for_owner(
        connection: Connection,
        pool: Arc<WorkerPool>,
        owner: OwnerId,
        control: ReceiveControl,
        limits: ReceiveLimits,
        now: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Self, ReceiveError> {
        limits.validate()?;
        let mut guest =
            Self::dormant_owner(pool, owner, control, limits, now, Box::new(wake), None);
        guest.install(connection, None);
        Ok(guest)
    }
}

struct OwnerCase {
    guest: Option<GuestStartup>,
    reservation: Option<Reservation>,
    fixture: Option<Fixture>,
    pool: Arc<WorkerPool>,
    owner: OwnerId,
    lifecycle: ReceiveControl,
    limits: ReceiveLimits,
    now: Instant,
    original_registries: usize,
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

impl OwnerCase {
    fn new() -> Self {
        Self::with_text_policy(None).0
    }

    // Optional metadata and protocol mode for sequential real-Guest fixtures.
    // This constructor never installs a Runtime/Generic readiness source.
    fn with_text_policy(
        text: Option<crate::TextClipboardLimits>,
    ) -> (Self, Option<ClipboardControl>) {
        let mut fixture = if text.is_some() {
            Fixture::new_text(Some(3), &[5])
        } else {
            Fixture::new(Some(3), &[5])
        };
        fixture.create_surfaces();
        let original_registries = registry_count(&fixture);
        let pool = WorkerPool::isolated();
        let lifecycle = control();
        let limits = ReceiveLimits::default();
        let (clipboard, text_worker) = match text {
            Some(policy) => {
                let (control, worker) = ClipboardControl::new(lifecycle.clone(), policy).unwrap();
                (Some(control), Some(worker))
            }
            None => (None, None),
        };
        let display = fixture.host.backend().display_ptr() as usize;
        let retained = Arc::new((fixture.host.clone(), fixture.host_surfaces.clone()));
        let (owner, reservation) = pool
            .reserve_owner(display, retained, lifecycle.clone(), limits.startup_timeout)
            .unwrap();
        let now = Instant::now();
        // The fixture owns the actual display and its coordinated host reader.
        let backend =
            unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
        let connection = Connection::from_backend(backend);
        let guest = if let Some(worker) = text_worker {
            limits.validate().unwrap();
            let mut guest = GuestStartup::dormant_owner(
                pool.clone(),
                owner,
                lifecycle.clone(),
                limits,
                now,
                Box::new(|| {}),
                Some(worker),
            );
            guest.install(connection, None);
            guest
        } else {
            GuestStartup::for_owner(
                connection,
                pool.clone(),
                owner,
                lifecycle.clone(),
                limits,
                now,
                || {},
            )
            .unwrap()
        };
        (
            Self {
                guest: Some(guest),
                reservation: Some(reservation),
                fixture: Some(fixture),
                pool,
                owner,
                lifecycle,
                limits,
                now,
                original_registries,
            },
            clipboard,
        )
    }

    fn fixture(&self) -> &Fixture {
        self.fixture.as_ref().unwrap()
    }
    fn fixture_mut(&mut self) -> &mut Fixture {
        self.fixture.as_mut().unwrap()
    }
    fn guest(&self) -> &GuestStartup {
        self.guest.as_ref().unwrap()
    }

    fn pump(&mut self) {
        self.guest.as_mut().unwrap().pump(self.now).unwrap();
    }

    fn step(&mut self) {
        self.fixture
            .as_mut()
            .unwrap()
            .step(self.guest.as_mut().unwrap(), self.now)
            .unwrap();
    }

    fn deliver(&mut self) {
        self.fixture
            .as_mut()
            .unwrap()
            .deliver(self.guest.as_mut().unwrap(), self.now)
            .unwrap();
    }

    fn until(&mut self, mut done: impl FnMut(&Self) -> bool) {
        for _ in 0..32 {
            self.pump();
            self.step();
            if done(self) {
                return;
            }
        }
        panic!("the real private owner protocol made no progress");
    }

    fn try_register(&self, surface: usize) -> Result<SurfaceLease, ReceiveError> {
        let fixture = self.fixture();
        let proxy = fixture.host_surfaces[surface].clone();
        let key = SurfaceKey {
            display: fixture.host.backend().display_ptr() as usize,
            surface: proxy.id().as_ptr() as usize,
        };
        self.pool.reserve_surface(
            self.owner,
            key,
            Arc::new((fixture.host.clone(), proxy)),
            Box::new(|| {}),
        )
    }

    fn register(&self, surface: usize) -> SurfaceLease {
        self.try_register(surface).unwrap()
    }

    fn enter(&mut self, surface: usize, key: usize, x: f64, y: f64) -> wl_data_offer::WlDataOffer {
        let offer = self.fixture().offer(0, key, "text/uri-list");
        offer.action(wl_data_device_manager::DndAction::Copy);
        self.fixture().enter(0, &offer, surface, x, y);
        self.deliver();
        offer
    }

    fn drop_gesture(&mut self) {
        self.fixture().server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        self.deliver();
    }
}

impl Drop for OwnerCase {
    fn drop(&mut self) {
        if let Some(mut guest) = self.guest.take() {
            let cleaned = catch_unwind(AssertUnwindSafe(|| guest.cleanup()));
            if !matches!(cleaned, Ok(Ok(()))) {
                // Never destroy an unconfirmed foreign display during assertion unwind.
                std::mem::forget(guest);
                if let Some(fixture) = self.fixture.take() {
                    std::mem::forget(fixture);
                }
                if let Some(reservation) = self.reservation.take() {
                    std::mem::forget(reservation);
                }
                assert!(
                    std::thread::panicking(),
                    "the private guest failed cleanup; foreign resources retained"
                );
                return;
            }
            drop(guest);
        }
        drop(self.reservation.take());
    }
}

fn hover_at(control: &ReceiveControl, x: f64, y: f64) -> OfferId {
    let events = control.poll();
    match events.as_slice() {
        [ReceiveEvent::Hovered { id, point }] if *point == (SurfacePoint { x, y }) => *id,
        _ => panic!("expected only the exact surface hover: {events:?}"),
    }
}

fn destroyed(fixture: &Fixture, key: usize) -> usize {
    fixture
        .server
        .requests
        .iter()
        .filter(|request| matches!(request, OfferRequest::Destroy(actual) if *actual == key))
        .count()
}

fn rejected(fixture: &Fixture, key: usize) -> bool {
    fixture
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Accept(actual, None) if *actual == key))
}

fn busy_b(case: &mut OwnerCase, control: &ReceiveControl, key: usize) {
    let _offer = case.enter(1, key, 100.0, 200.0);
    case.drop_gesture();
    case.fixture_mut().serve();
    assert!(
        control.poll().is_empty(),
        "a busy endpoint must not publish another drop"
    );
    assert!(rejected(case.fixture(), key));
    assert_eq!(destroyed(case.fixture(), key), 1);
    assert_eq!(case.fixture().received(key), 0);
    assert_eq!(case.fixture().finish_count(key), 0);
}

#[test]
fn empty_owner_discovers_once_then_routes_two_surfaces_on_the_same_first_device() {
    let mut case = OwnerCase::new();
    assert!(case
        .pool
        .take_pending_surfaces(case.owner)
        .unwrap()
        .is_empty());
    case.until(|case| case.lifecycle.status() == ReceiveStatus::Ready);
    assert_eq!(case.fixture().server.manager_versions, vec![3]);
    assert_eq!(case.fixture().server.seat_versions, vec![5]);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(registry_count(case.fixture()), case.original_registries + 1);

    let a = case.register(0);
    let a_control = a.control();
    case.until(|_| a_control.status() == ReceiveStatus::Ready);
    // Surface B exists in the host but has not been registered with this owner.
    let _foreign = case.enter(1, 1, 90.0, 91.0);
    case.fixture_mut().serve();
    assert!(a_control.poll().is_empty());
    assert!(rejected(case.fixture(), 1));
    assert_eq!(destroyed(case.fixture(), 1), 1);
    assert_eq!(case.fixture().received(1), 0);

    let b = case.register(1);
    let b_control = b.control();
    case.until(|_| b_control.status() == ReceiveStatus::Ready);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(registry_count(case.fixture()), case.original_registries + 1);
    let _a_offer = case.enter(0, 2, 10.0, 20.0);
    let a_id = hover_at(&a_control, 10.0, 20.0);
    assert!(b_control.poll().is_empty());

    let selection = case.fixture().introduce_offer(0, 3);
    case.fixture().server.device_resources[0].selection(Some(&selection));
    selection.offer("text/uri-list".into());
    selection.offer("text/plain".into());
    case.fixture().server.device_resources[0].selection(None);
    case.deliver();
    case.fixture_mut().serve();
    assert!(a_control.poll().is_empty());
    assert!(b_control.poll().is_empty());
    assert_eq!(destroyed(case.fixture(), 3), 1);
    assert_eq!(destroyed(case.fixture(), 2), 0);
    assert_eq!(case.fixture().received(3), 0);
    assert_eq!(case.fixture().finish_count(3), 0);

    case.fixture().server.device_resources[0].leave();
    case.deliver();
    assert!(matches!(a_control.poll().as_slice(), [ReceiveEvent::Left { id }] if *id == a_id));
    let _b_offer = case.enter(1, 4, 30.0, 40.0);
    let b_id = hover_at(&b_control, 30.0, 40.0);
    assert_ne!(a_id, b_id);
    assert!(a_control.poll().is_empty());
    assert_eq!(case.fixture().server.devices, 1);
    assert!(!case.fixture().finished());
}

#[test]
fn one_owner_drop_lease_survives_eof_and_an_undrained_terminal_without_replacing_a() {
    let mut case = OwnerCase::new();
    case.until(|case| case.lifecycle.status() == ReceiveStatus::Ready);
    let a = case.register(0);
    let b = case.register(1);
    let a_control = a.control();
    let b_control = b.control();
    case.until(|_| {
        a_control.status() == ReceiveStatus::Ready && b_control.status() == ReceiveStatus::Ready
    });
    let _a_offer = case.enter(0, 10, 11.0, 21.0);
    let a_id = hover_at(&a_control, 11.0, 21.0);
    let a_epoch = a_control.header().epoch;
    case.drop_gesture();
    assert!(
        matches!(a_control.poll().as_slice(), [ReceiveEvent::DropStarted { id, epoch, point }]
        if *id == a_id && *epoch == a_epoch && *point == SurfacePoint { x: 11.0, y: 21.0 })
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().received(10), 1);
    let writer = case.fixture_mut().take_writer(10);
    let payload = b"file:///tmp/owner-a-first\r\nfile:///tmp/owner-a-second\r\n";
    let mut remaining = payload.as_slice();
    while !remaining.is_empty() {
        let count = rustix::io::write(&writer, remaining).unwrap();
        assert!(count > 0);
        remaining = &remaining[count..];
    }
    for _ in 0..3 {
        case.pump();
        assert!(a_control.poll().is_empty());
    }
    let (pipe_id, pipe_fd) = case.guest().pipe_fd().expect("A still waits for real EOF");
    assert_eq!(pipe_id, ReadTag::Drop(a_id));
    busy_b(&mut case, &b_control, 11);
    let (same_id, same_fd) = case
        .guest()
        .pipe_fd()
        .expect("Busy B cannot replace A's pipe");
    assert_eq!(same_id, ReadTag::Drop(a_id));
    assert!(Arc::ptr_eq(&pipe_fd, &same_fd));
    assert_eq!(destroyed(case.fixture(), 10), 0);
    drop(pipe_fd);
    drop(same_fd);
    drop(writer);

    let mut ready = Vec::new();
    for _ in 0..8 {
        case.pump();
        ready.extend(a_control.poll());
        if ready
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
        {
            break;
        }
    }
    assert!(
        matches!(ready.as_slice(), [ReceiveEvent::DropReady { id, paths }]
        if *id == a_id && *paths == vec![std::path::PathBuf::from("/tmp/owner-a-first"), std::path::PathBuf::from("/tmp/owner-a-second")])
    );
    assert!(case.guest().pipe_fd().is_none());
    busy_b(&mut case, &b_control, 12);
    assert!(a_control.poll().is_empty());
    assert_eq!(case.fixture().finish_count(10), 0);

    a_control.decide(a_id, DropDecision::Accept).unwrap();
    case.pump();
    case.fixture_mut().serve();
    assert_eq!(case.fixture().finish_count(10), 1);
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));
    busy_b(&mut case, &b_control, 13);
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));
    assert!(
        matches!(a_control.poll().as_slice(), [ReceiveEvent::DropEnded { id, result: Ok(()) }] if *id == a_id)
    );
    case.pump();

    let _b_offer = case.enter(1, 14, 31.0, 41.0);
    let b_id = hover_at(&b_control, 31.0, 41.0);
    assert_ne!(b_id, a_id);
    case.drop_gesture();
    assert!(
        matches!(b_control.poll().as_slice(), [ReceiveEvent::DropStarted { id, point, .. }]
        if *id == b_id && *point == SurfacePoint { x: 31.0, y: 41.0 })
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().received(14), 1);
    b_control.cancel(b_id).unwrap();
    case.pump();
    assert!(
        matches!(b_control.poll().as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == b_id)
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().finish_count(10), 1);
    assert_eq!(case.fixture().finish_count(14), 0);
    assert!(a_control.poll().is_empty());

    // Complete detach releases A's lease even while its exact terminal stays
    // undrained in the old control; that later acknowledgement cannot touch B.
    let _old_a_offer = case.enter(0, 15, 12.0, 22.0);
    let old_a_id = hover_at(&a_control, 12.0, 22.0);
    case.drop_gesture();
    assert!(
        matches!(a_control.poll().as_slice(), [ReceiveEvent::DropStarted { id, .. }] if *id == old_a_id)
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().received(15), 1);
    let old_writer = case.fixture_mut().take_writer(15);
    a_control.cancel(old_a_id).unwrap();
    case.pump();
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));
    drop(old_writer);
    a.request_retire();
    case.guest
        .as_mut()
        .unwrap()
        .pump_readable(case.now, None)
        .unwrap();
    assert!(!a.is_detached());
    case.guest.as_mut().unwrap().finish_retirements().unwrap();
    assert!(a.is_detached());
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Ended { drained: false }
    ));

    let _new_b_offer = case.enter(1, 16, 33.0, 43.0);
    let new_b_id = hover_at(&b_control, 33.0, 43.0);
    case.drop_gesture();
    assert!(
        matches!(b_control.poll().as_slice(), [ReceiveEvent::DropStarted { id, .. }] if *id == new_b_id)
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().received(16), 1);
    let (before_id, before_fd) = case
        .guest()
        .pipe_fd()
        .expect("B owns its actual transfer pipe before old A is polled");
    assert_eq!(before_id, ReadTag::Drop(new_b_id));
    assert!(
        matches!(a_control.poll().as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == old_a_id)
    );
    assert!(
        a_control.poll().is_empty(),
        "the detached control keeps one terminal only"
    );
    case.pump();
    let (after_id, after_fd) = case
        .guest()
        .pipe_fd()
        .expect("old A's ACK cannot release B's transfer");
    assert_eq!(after_id, ReadTag::Drop(new_b_id));
    assert!(Arc::ptr_eq(&before_fd, &after_fd));
    {
        let header = b_control.header();
        let current = header.offer.unwrap();
        assert_eq!(current.id, new_b_id);
        assert!(matches!(current.gate, AttemptGate::Open));
    }
    assert!(
        b_control.poll().is_empty(),
        "old A's ACK cannot cancel or finish B"
    );

    // A fresh endpoint makes a prematurely released global lease observable:
    // its offer must still be rejected while B owns the same untouched pipe.
    let fresh_a = case.register(0);
    let fresh_a_control = fresh_a.control();
    case.until(|_| fresh_a_control.status() == ReceiveStatus::Ready);
    let _busy_a_offer = case.enter(0, 17, 13.0, 23.0);
    case.drop_gesture();
    case.fixture_mut().serve();
    assert!(fresh_a_control.poll().is_empty());
    assert!(rejected(case.fixture(), 17));
    assert_eq!(case.fixture().received(17), 0);
    let (still_id, still_fd) = case
        .guest()
        .pipe_fd()
        .expect("rejected A cannot replace B after the stale ACK");
    assert_eq!(still_id, ReadTag::Drop(new_b_id));
    assert!(Arc::ptr_eq(&before_fd, &still_fd));
    assert!(b_control.poll().is_empty());
    drop(before_fd);
    drop(after_fd);
    drop(still_fd);
    b_control.cancel(new_b_id).unwrap();
    case.pump();
    assert!(
        matches!(b_control.poll().as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if *id == new_b_id)
    );
    case.fixture_mut().serve();
    assert_eq!(case.fixture().finish_count(15), 0);
    assert_eq!(case.fixture().finish_count(16), 0);
    assert_eq!(case.fixture().finish_count(17), 0);
}

#[test]
fn retired_arm_callback_is_a_tombstone_and_cannot_rearm_a_replacement_or_renew_its_deadline() {
    for expire_replacement in [false, true] {
        let mut case = OwnerCase::new();
        case.until(|case| case.lifecycle.status() == ReceiveStatus::Ready);
        let b = case.register(1);
        let b_control = b.control();
        case.until(|_| b_control.status() == ReceiveStatus::Ready);
        let a = case.register(0);
        let a_control = a.control();
        case.pump();
        let old = case
            .guest()
            .pending_sync()
            .expect("A owns the pending physical callback");
        assert_eq!(old.1, SyncPhase::Arm(a_control.header().epoch));
        // Send the real reply into the socket, but deliberately do not run host_read.
        case.fixture_mut().serve();
        case.pump();
        assert_eq!(case.guest().pending_sync(), Some(old.clone()));
        assert_eq!(a_control.status(), ReceiveStatus::Starting);
        a.request_retire();
        // No Runtime watcher exists in this sequential fixture. The production
        // read pump may retire local SDK routing, but cannot acknowledge detach
        // until its caller has removed the old watcher and finishes retirement.
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, None)
            .unwrap();
        assert!(!a.is_detached());
        assert!(matches!(case.try_register(0), Err(ReceiveError::Busy)));
        case.guest.as_mut().unwrap().finish_retirements().unwrap();
        assert!(a.is_detached());
        assert_eq!(case.guest().pending_sync(), Some(old.clone()));
        let replacement = case.register(0);
        // Sample after admission: this upper bound cannot precede its actual deadline.
        let expires_at = Instant::now() + case.limits.startup_timeout;
        let replacement_control = replacement.control();
        assert!(replacement.id() != a.id());
        case.pump();
        assert_eq!(case.guest().pending_sync(), Some(old.clone()));
        assert!(!replacement_control.header().armed);
        assert_eq!(b_control.status(), ReceiveStatus::Ready);

        if expire_replacement {
            case.now = case.now.max(expires_at);
            case.pump();
            assert!(replacement.is_detached());
            assert_eq!(
                replacement_control.header().close_cause,
                Some(ReceiveError::Timeout)
            );
            assert_eq!(case.guest().pending_sync(), Some(old.clone()));
        }

        // Exactly one reader pass and one pump: it can retire the old callback
        // and send a new Arm, but cannot read that replacement callback's reply.
        case.fixture().host_read();
        case.pump();
        assert!(!replacement_control.header().armed);
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
        assert_eq!(b_control.status(), ReceiveStatus::Ready);
        if expire_replacement {
            assert!(case.guest().pending_sync().is_none());
            assert!(replacement.is_detached());
        } else {
            let fresh = case
                .guest()
                .pending_sync()
                .expect("the replacement needs its own callback");
            assert_ne!(old.0, fresh.0);
            assert_eq!(fresh.1, SyncPhase::Arm(replacement_control.header().epoch));
            assert_eq!(replacement_control.status(), ReceiveStatus::Starting);
            case.until(|_| replacement_control.status() == ReceiveStatus::Ready);
            assert!(replacement_control.header().armed);
        }
        let _b_offer = case.enter(1, 30, 32.0, 42.0);
        let _b_id = hover_at(&b_control, 32.0, 42.0);
        assert!(replacement_control.poll().is_empty());
        assert_eq!(case.fixture().server.devices, 1);
        assert_eq!(registry_count(case.fixture()), case.original_registries + 1);
        assert!(!case.fixture().finished());
    }
}

impl OwnerCase {
    fn with_limits(limits: ReceiveLimits) -> Self {
        let mut fixture = Fixture::new(Some(3), &[5]);
        fixture.create_surfaces();
        let original_registries = registry_count(&fixture);
        let pool = WorkerPool::isolated();
        let lifecycle = control();
        let display = fixture.host.backend().display_ptr() as usize;
        let retained = Arc::new((fixture.host.clone(), fixture.host_surfaces.clone()));
        let (owner, reservation) = pool
            .reserve_owner(display, retained, lifecycle.clone(), limits.startup_timeout)
            .unwrap();
        let now = Instant::now();
        // This is the same real fixture/reader and production guest as OwnerCase::new.
        let backend =
            unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
        let guest = GuestStartup::for_owner(
            Connection::from_backend(backend),
            pool.clone(),
            owner,
            lifecycle.clone(),
            limits,
            now,
            || {},
        )
        .unwrap();
        Self {
            guest: Some(guest),
            reservation: Some(reservation),
            fixture: Some(fixture),
            pool,
            owner,
            lifecycle,
            limits,
            now,
            original_registries,
        }
    }

    fn register_with_wake(&self, surface: usize, wake: impl Fn() + Send + 'static) -> SurfaceLease {
        let fixture = self.fixture();
        let proxy = fixture.host_surfaces[surface].clone();
        let key = SurfaceKey {
            display: fixture.host.backend().display_ptr() as usize,
            surface: proxy.id().as_ptr() as usize,
        };
        self.pool
            .reserve_surface(
                self.owner,
                key,
                Arc::new((fixture.host.clone(), proxy)),
                Box::new(wake),
            )
            .unwrap()
    }
}

fn wait_for_exact_instant(receiver: &std::sync::mpsc::Receiver<()>, target: Instant) {
    while let Some(remaining) = target.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        assert_eq!(
            receiver.recv_timeout(remaining),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        );
    }
}

#[test]
fn endpoint_first_ready_cannot_publish_after_admission_expires_during_post_dispatch_reconcile() {
    use std::sync::{mpsc, OnceLock};

    let limits = ReceiveLimits {
        startup_timeout: Duration::from_millis(800),
        ..ReceiveLimits::default()
    };
    let mut case = OwnerCase::with_limits(limits);
    case.until(|case| case.lifecycle.status() == ReceiveStatus::Ready);
    let a_shared = Arc::new(OnceLock::<ReceiveControl>::new());
    let a_hover = Arc::new(OnceLock::<OfferId>::new());
    let deadlines = Arc::new(OnceLock::<(Instant, Instant)>::new());
    let gate_times = Arc::new(Mutex::new(None::<(Instant, Instant)>));
    let motion_cancelled = Arc::new(AtomicBool::new(false));
    let (gate_sender, gate_receiver) = mpsc::channel();
    let callback_a = a_shared.clone();
    let callback_hover = a_hover.clone();
    let callback_deadlines = deadlines.clone();
    let callback_times = gate_times.clone();
    let callback_cancelled = motion_cancelled.clone();
    let a = case.register_with_wake(0, move || {
        let Some(control) = callback_a.get() else { return; };
        for event in control.poll() {
            match event {
                ReceiveEvent::Hovered { id, point: SurfacePoint { x: 15.0, y: 25.0 } } => {
                    // Motion's real wake cancels only its own hover. Motion does
                    // not reconcile again, so Left is emitted by the POST check.
                    control.cancel(id).unwrap();
                    callback_cancelled.store(true, Ordering::SeqCst);
                }
                ReceiveEvent::Hovered { id, .. } => { let _ = callback_hover.set(id); }
                ReceiveEvent::Left { id } if callback_hover.get() == Some(&id)
                    && callback_cancelled.load(Ordering::SeqCst) => {
                    let (admission, uncapped_budget) = *callback_deadlines.get().expect("B's admission deadline and the old uncapped budget were observed before dispatch");
                    let entered = Instant::now();
                    assert!(entered < admission, "the gate must start before B expires");
                    wait_for_exact_instant(&gate_receiver, admission);
                    let released = Instant::now();
                    assert!(released < uncapped_budget, "the old uncapped sync budget must still be valid");
                    *callback_times.lock().unwrap() = Some((entered, released));
                }
                _ => {}
            }
        }
    });
    let a_control = a.control();
    assert!(a_shared.set(a_control.clone()).is_ok());
    case.until(|_| a_control.status() == ReceiveStatus::Ready);
    let _hover_offer = case.enter(0, 40, 10.0, 20.0);
    assert!(a_hover.get().is_some());

    let b_shared = Arc::new(OnceLock::<ReceiveControl>::new());
    let ready_publications = Arc::new(Mutex::new(Vec::<Instant>::new()));
    let callback_b = b_shared.clone();
    let callback_ready = ready_publications.clone();
    let b = case.register_with_wake(1, move || {
        if callback_b
            .get()
            .is_some_and(|control| control.status() == ReceiveStatus::Ready)
        {
            callback_ready.lock().unwrap().push(Instant::now());
        }
    });
    let registered = Instant::now();
    let b_control = b.control();
    assert!(b_shared.set(b_control.clone()).is_ok());
    // Deliberately separate admission and sync issuance using an exact bounded
    // channel deadline; no guessed sleep determines whether the bug reproduces.
    let (delay_sender, delay_receiver) = mpsc::channel();
    wait_for_exact_instant(&delay_receiver, registered + limits.startup_timeout / 2);
    drop(delay_sender);
    let issued_not_before = Instant::now();
    case.pump();
    let uncapped_sync_budget = issued_not_before + limits.startup_timeout;
    assert_eq!(
        case.guest().pending_sync().unwrap().1,
        SyncPhase::Arm(b_control.header().epoch)
    );
    let endpoint = case
        .guest()
        .state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.registration == Some(b.id()))
        .unwrap();
    let admission = endpoint.ready_deadline.unwrap();
    // This lower bound describes the old renewed budget, without requiring a
    // correct coordinator to keep it instead of capping at admission.
    assert!(uncapped_sync_budget >= admission + limits.startup_timeout / 3);
    assert!(Instant::now() < admission);
    assert!(deadlines.set((admission, uncapped_sync_budget)).is_ok());

    // Motion is queued before the server processes B's already-issued Sync.
    // The one actual reader pass then delivers Motion followed by Done(B).
    case.fixture().server.device_resources[0].motion(41, 15.0, 25.0);
    case.fixture_mut().serve();
    case.fixture().host_read();
    case.pump();
    let (entered, released) = gate_times
        .lock()
        .unwrap()
        .expect("Left wake blocked inside the real POST-dispatch reconcile");
    assert!(entered < admission);
    assert!(released >= admission);
    assert!(released < uncapped_sync_budget);
    assert!(motion_cancelled.load(Ordering::SeqCst));
    let published = ready_publications.lock().unwrap().clone();
    assert!(published.is_empty(), "first Ready was published after its admission deadline: {published:?}, deadline {admission:?}, old uncapped budget lower bound {uncapped_sync_budget:?}");
    assert!(b.is_detached());
    assert_eq!(b_control.header().close_cause, Some(ReceiveError::Timeout));
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert!(!case.fixture().finished());
    assert_eq!(case.fixture().received(40), 0);
    drop(gate_sender);
}

#[test]
fn a_reinvalidated_first_endpoint_cannot_take_the_next_arm_ahead_of_an_unserved_endpoint() {
    let mut case = OwnerCase::new();
    case.until(|case| case.lifecycle.status() == ReceiveStatus::Ready);
    let a = case.register(0);
    let b = case.register(1);
    let a_control = a.control();
    let b_control = b.control();
    case.pump();
    let first = case
        .guest()
        .pending_sync()
        .expect("A has the first real arm callback");
    assert_eq!(first.1, SyncPhase::Arm(a_control.header().epoch));
    let a_next = a_control.invalidate_pending().unwrap();
    assert_ne!(a_next, b_control.header().epoch);
    case.fixture_mut().serve();
    case.fixture().host_read();
    case.pump();
    let next = case
        .guest()
        .pending_sync()
        .expect("the unserved B owns the next callback");
    assert_ne!(next.0, first.0);
    assert_eq!(next.1, SyncPhase::Arm(b_control.header().epoch));
    case.fixture_mut().serve();
    case.fixture().host_read();
    case.pump();
    assert_eq!(b_control.status(), ReceiveStatus::Ready);
    assert_eq!(a_control.status(), ReceiveStatus::Starting);
    assert!(!a_control.header().armed);
    assert_eq!(
        case.guest().pending_sync().unwrap().1,
        SyncPhase::Arm(a_next)
    );
}

#[path = "clipboard_read_tag_tests.rs"]
mod read_tags;
