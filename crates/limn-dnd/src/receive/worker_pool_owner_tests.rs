use std::{
    cell::RefCell,
    sync::mpsc,
    thread::{self, ThreadId},
    time::{Duration, Instant},
};

use super::*;
use crate::receive::{control::AttemptGate, coordinator::Coordinator, OfferId, ReceiveEpoch};
use crate::{ReceiveLimits, SurfacePoint};

const WAIT: Duration = Duration::from_secs(3);
const ENDPOINT_TIMEOUT: Duration = Duration::from_secs(5);

fn key(display: usize, surface: usize) -> SurfaceKey {
    SurfaceKey { display, surface }
}

fn control() -> ReceiveControl {
    ReceiveControl::new(ReceiveEpoch(1)).unwrap()
}

fn owner(pool: &Arc<WorkerPool>, display: usize) -> (OwnerId, Reservation, ReceiveControl) {
    let control = control();
    let (id, reservation) = pool
        .reserve_owner(display, Arc::new(()), control.clone(), ENDPOINT_TIMEOUT)
        .unwrap();
    (id, reservation, control)
}

fn surface(pool: &Arc<WorkerPool>, owner: OwnerId, key: SurfaceKey) -> SurfaceLease {
    pool.reserve_surface(owner, key, Arc::new(()), Box::new(|| {}))
        .unwrap()
}

fn receive<T>(receiver: &mpsc::Receiver<T>) -> T {
    receiver
        .recv_timeout(WAIT)
        .expect("owner lifecycle progress")
}

fn retire_pending(pool: &WorkerPool, owner: OwnerId) {
    for pending in pool.take_pending_surfaces(owner).unwrap() {
        let id = pending.id;
        drop(pending); // This fixture has no SDK objects or active transfer.
        pool.retire_surface(owner, id).unwrap();
    }
}

#[test]
fn owner_pool_separates_four_workers_from_four_global_surface_permits() {
    let pool = WorkerPool::isolated();
    let (a, _a_worker, _) = owner(&pool, 1);
    let (b, _b_worker, _) = owner(&pool, 2);
    let admitted = Instant::now();
    let surfaces = [
        surface(&pool, a, key(1, 10)),
        surface(&pool, a, key(1, 11)),
        surface(&pool, b, key(2, 10)),
        surface(&pool, b, key(2, 11)),
    ];
    let last_admitted = Instant::now();
    let pending_a = pool.take_pending_surfaces(a).unwrap();
    let pending_b = pool.take_pending_surfaces(b).unwrap();
    assert_eq!(pending_a.len(), 2);
    assert_eq!(pending_b.len(), 2);
    assert!(pool.take_pending_surfaces(a).unwrap().is_empty());
    for pending in pending_a.iter().chain(&pending_b) {
        assert!(pending.ready_deadline >= admitted + ENDPOINT_TIMEOUT);
        assert!(pending.ready_deadline <= last_admitted + ENDPOINT_TIMEOUT);
        assert!(surfaces.iter().any(|surface| surface.id() == pending.id));
    }
    assert!(matches!(
        pool.reserve_surface(a, key(1, 12), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_legacy(key(3, 1)),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_owned(key(3, 1), Arc::new(()), control()),
        Err(ReceiveError::Busy)
    ));

    // Full surface admission leaves the two unused worker permits available.
    let (_, _c_worker, _) = owner(&pool, 3);
    let (_, _d_worker, _) = owner(&pool, 4);
    assert!(matches!(
        pool.reserve_owner(5, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    for (id, pending) in [(a, pending_a), (b, pending_b)] {
        for seed in pending {
            let surface = seed.id;
            drop(seed);
            pool.retire_surface(id, surface).unwrap();
        }
    }
    assert!(surfaces.iter().all(SurfaceLease::is_detached));

    // Legacy and standalone receivers consume the same global surface budget.
    let mixed = WorkerPool::isolated();
    let (id, _worker, _) = owner(&mixed, 10);
    let _a = surface(&mixed, id, key(10, 1));
    let _b = surface(&mixed, id, key(10, 2));
    let _legacy = mixed.reserve_legacy(key(11, 1)).unwrap();
    let _receiver = mixed
        .reserve_owned(key(12, 1), Arc::new(()), control())
        .unwrap();
    assert!(matches!(
        mixed.reserve_surface(id, key(10, 3), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Busy)
    ));
    let (_, _empty_worker, _) = owner(&mixed, 13);
    retire_pending(&mixed, id);
}

#[test]
fn owner_pool_excludes_displays_bidirectionally_but_preserves_legacy_surface_rules() {
    let pool = WorkerPool::isolated();
    let (a, _a_worker, _) = owner(&pool, 1);
    let _surface = surface(&pool, a, key(1, 10));
    assert!(matches!(
        pool.reserve_owner(1, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_legacy(key(1, 99)),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_owned(key(1, 99), Arc::new(()), control()),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_surface(a, key(1, 10), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_surface(a, key(2, 10), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Unavailable)
    ));
    let (b, _b_worker, _) = owner(&pool, 2);
    let _other_display = surface(&pool, b, key(2, 10));
    retire_pending(&pool, a);
    retire_pending(&pool, b);

    let legacy = WorkerPool::isolated();
    let _one = legacy.reserve_legacy(key(3, 10)).unwrap();
    let _two = legacy.reserve_legacy(key(3, 11)).unwrap();
    assert!(matches!(
        legacy.reserve_legacy(key(3, 10)),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        legacy.reserve_owner(3, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    let (_, _unrelated, _) = owner(&legacy, 4);

    let standalone = WorkerPool::isolated();
    let _one = standalone
        .reserve_owned(key(5, 10), Arc::new(()), control())
        .unwrap();
    let _two = standalone
        .reserve_owned(key(5, 11), Arc::new(()), control())
        .unwrap();
    assert!(matches!(
        standalone.reserve_owner(5, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    let (_, _unrelated, _) = owner(&standalone, 6);
}

struct WindowDrop {
    pool: Arc<WorkerPool>,
    sent: mpsc::Sender<(ThreadId, bool, bool)>,
}

impl Drop for WindowDrop {
    fn drop(&mut self) {
        let unlocked = self.pool.shared.state.try_lock().is_ok();
        // Avoid a deadlocked fixture if a regression runs Drop under the pool lock.
        let still_reserved = unlocked
            && matches!(
                self.pool.reserve_legacy(key(2, 90)),
                Err(ReceiveError::Busy)
            );
        let _ = self
            .sent
            .send((thread::current().id(), unlocked, still_reserved));
    }
}

#[test]
fn owner_pool_retirement_drops_window_off_caller_and_reuses_only_the_surface_permit() {
    let caller = thread::current().id();
    let pool = WorkerPool::isolated();
    let (id, worker, lifecycle) = owner(&pool, 1);
    let (dropped, drop_result) = mpsc::channel();
    let window = Arc::new(WindowDrop {
        pool: pool.clone(),
        sent: dropped,
    });
    let retained = Arc::downgrade(&window);
    let a = pool
        .reserve_surface(id, key(1, 10), window, Box::new(|| {}))
        .unwrap();
    let b = surface(&pool, id, key(1, 11));
    // Keep all four surface permits occupied while A's final destructor runs.
    let c = surface(&pool, id, key(1, 12));
    let d = surface(&pool, id, key(1, 13));
    let b_control = b.control();
    let old = a.id();
    a.request_retire();
    assert_eq!(a.control().status(), ReceiveStatus::Closing);
    assert!(!a.is_detached());
    assert!(retained.upgrade().is_some());
    assert!(b_control.invalidate_pending().is_ok());
    let (retired, retire_result) = mpsc::channel();
    let (replacement, next_surface) = mpsc::sync_channel(1);
    let (closed, close_result) = mpsc::channel();
    let worker_pool = pool.clone();
    let worker_control = lifecycle.clone();
    let b_id = b.id();
    let c_id = c.id();
    let d_id = d.id();
    worker
        .spawn(move |exit| {
            let seeds = worker_pool.take_pending_surfaces(id).unwrap();
            assert_eq!(seeds.len(), 4);
            drop(seeds);
            worker_pool.retire_surface(id, old).unwrap();
            let _ = retired.send(thread::current().id());
            let fresh = next_surface.recv_timeout(WAIT).unwrap();
            let seeds = worker_pool.take_pending_surfaces(id).unwrap();
            assert_eq!(seeds.len(), 1);
            assert!(seeds[0].id == fresh);
            let fresh_control = seeds[0].control.clone();
            drop(seeds);
            let _ = worker_pool.retire_surface(id, old);
            assert!(!fresh_control.header().is_closing());
            assert!(b_control.invalidate_pending().is_ok());
            worker_pool.retire_surface(id, fresh).unwrap();
            worker_pool.retire_surface(id, b_id).unwrap();
            worker_pool.retire_surface(id, c_id).unwrap();
            worker_pool.retire_surface(id, d_id).unwrap();
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(move || {
                    let _ = closed.send(worker_control.status());
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    let retiring_thread = receive(&retire_result);
    let (drop_thread, unlocked, still_reserved) = receive(&drop_result);
    assert_ne!(drop_thread, caller);
    assert_eq!(drop_thread, retiring_thread);
    assert!(
        unlocked,
        "the admitted window's final Drop runs outside the pool mutex"
    );
    assert!(
        still_reserved,
        "the surface remains occupied through its final Drop"
    );
    assert!(retained.upgrade().is_none());
    assert!(a.is_detached());
    assert!(!b.is_detached());
    assert!(!lifecycle.header().is_closing());
    assert!(matches!(
        pool.reserve_legacy(key(1, 12)),
        Err(ReceiveError::Busy)
    ));
    let fresh = surface(&pool, id, key(1, 10));
    assert!(
        fresh.id() != old,
        "a reused surface gets a fresh registration identity"
    );
    replacement.send(fresh.id()).unwrap();
    assert_eq!(receive(&close_result), ReceiveStatus::Closed(None));
    assert!(fresh.is_detached());
    assert!(b.is_detached());
}

struct TlsPause {
    entered: mpsc::Sender<ThreadId>,
    release: mpsc::Receiver<()>,
    opened: mpsc::Sender<bool>,
}

impl Drop for TlsPause {
    fn drop(&mut self) {
        let _ = self.entered.send(thread::current().id());
        let _ = self.opened.send(self.release.recv_timeout(WAIT).is_ok());
    }
}

thread_local! {
    static TLS_PAUSE: RefCell<Option<TlsPause>> = const { RefCell::new(None) };
}

struct ReleaseOnDrop(Option<mpsc::SyncSender<()>>);

impl ReleaseOnDrop {
    fn release(&mut self) {
        if let Some(sent) = self.0.take() {
            let _ = sent.try_send(());
        }
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

#[test]
fn detached_endpoint_does_not_return_owner_worker_or_display_before_actual_tls_join() {
    let pool = WorkerPool::isolated();
    let lifecycle = control();
    let display_owner = Arc::new(());
    let retained = Arc::downgrade(&display_owner);
    let (id, worker) = pool
        .reserve_owner(1, display_owner, lifecycle.clone(), ENDPOINT_TIMEOUT)
        .unwrap();
    let endpoint = surface(&pool, id, key(1, 10));
    let (entered, tls_entered) = mpsc::channel();
    let (release, tls_release) = mpsc::sync_channel(1);
    let (opened, tls_opened) = mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release));
    let (closed, close_result) = mpsc::channel();
    let worker_pool = pool.clone();
    let worker_control = lifecycle.clone();
    worker
        .spawn(move |exit| {
            for pending in worker_pool.take_pending_surfaces(id).unwrap() {
                let surface = pending.id;
                drop(pending);
                worker_pool.retire_surface(id, surface).unwrap();
            }
            TLS_PAUSE.with(|slot| {
                *slot.borrow_mut() = Some(TlsPause {
                    entered,
                    release: tls_release,
                    opened,
                });
            });
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(move || {
                    let _ = closed.send(worker_control.status());
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    receive(&tls_entered);
    assert!(endpoint.is_detached());
    assert_eq!(lifecycle.status(), ReceiveStatus::Closing);
    assert!(retained.upgrade().is_some());
    assert!(matches!(
        pool.reserve_owner(1, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_legacy(key(1, 10)),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_owned(key(1, 10), Arc::new(()), control()),
        Err(ReceiveError::Busy)
    ));
    // Reaper join holds no pool mutex; the three other worker slots remain useful.
    let (_, _b, _) = owner(&pool, 2);
    let (_, _c, _) = owner(&pool, 3);
    let (_, _d, _) = owner(&pool, 4);
    assert!(matches!(
        pool.reserve_owner(5, Arc::new(()), control(), ENDPOINT_TIMEOUT),
        Err(ReceiveError::Busy)
    ));
    release.release();
    assert!(
        receive(&tls_opened),
        "TLS was released explicitly, never by its safety timeout"
    );
    assert_eq!(receive(&close_result), ReceiveStatus::Closed(None));
    assert!(retained.upgrade().is_none());
    let (_, _reused, _) = owner(&pool, 1);
}

fn ready(control: ReceiveControl, value: u64) -> (Coordinator, OfferId, ReceiveEpoch) {
    let mut coordinator =
        Coordinator::with_control(ReceiveLimits::default(), control.clone(), || {}).unwrap();
    let now = Instant::now();
    let barrier = coordinator.next_sync(now).unwrap().unwrap();
    coordinator.sync_done(barrier.epoch, now).unwrap();
    let id = OfferId(value);
    coordinator
        .enter(id, SurfacePoint { x: 1.0, y: 2.0 }, 3, true)
        .unwrap();
    coordinator.action(id, true).unwrap();
    coordinator.start(id, now).unwrap();
    coordinator
        .complete(id, b"file:///tmp/owner-received\r\n", now)
        .unwrap();
    control.poll();
    (coordinator, id, barrier.epoch)
}

#[test]
fn owner_close_cuts_admission_and_unclaimed_finish_authority_before_endpoint_sweep() {
    let pool = WorkerPool::isolated();
    let (id, _worker, lifecycle) = owner(&pool, 1);
    lifecycle.header().status = ReceiveStatus::Ready;
    let a = surface(&pool, id, key(1, 10));
    let b = surface(&pool, id, key(1, 11));
    let a_control = a.control();
    let b_control = b.control();
    let seeds = pool.take_pending_surfaces(id).unwrap();
    assert_eq!(seeds.len(), 2);
    drop(seeds);
    let (_a_coordinator, a_offer, a_epoch) = ready(a_control.clone(), 1);
    let (_b_coordinator, b_offer, b_epoch) = ready(b_control.clone(), 2);
    assert_eq!(a_control.claim(a_offer, a_epoch, Instant::now()), Ok(()));

    // Registration must pass through the same parent authority, even when it
    // already obtained the owner's pool entry before closure won that gate.
    let mut parent = lifecycle.header();
    let (entered, registration_entered) = mpsc::channel();
    let (registered, registration_result) = mpsc::channel();
    let registering_pool = pool.clone();
    let registrar = thread::spawn(move || {
        let _ = entered.send(());
        let result =
            registering_pool.reserve_surface(id, key(1, 12), Arc::new(()), Box::new(|| {}));
        let _ = registered.send(result.map(drop));
    });
    receive(&registration_entered);
    let before_close = registration_result.recv_timeout(Duration::from_millis(100));
    parent.request_close(None);
    drop(parent);
    let after_close = if matches!(before_close, Err(mpsc::RecvTimeoutError::Timeout)) {
        receive(&registration_result)
    } else {
        before_close.expect("registration did not disconnect")
    };
    registrar.join().unwrap();
    assert_eq!(before_close, Err(mpsc::RecvTimeoutError::Timeout));
    assert_eq!(after_close, Err(ReceiveError::Closed));

    // Parent authority is closed before any endpoint header has been swept.
    assert_eq!(b_control.header().status, ReceiveStatus::Ready);
    assert_eq!(
        b_control.claim(b_offer, b_epoch, Instant::now()),
        Err(ReceiveError::StaleOffer)
    );
    assert!(matches!(
        pool.reserve_surface(id, key(1, 12), Arc::new(()), Box::new(|| {})),
        Err(ReceiveError::Closed)
    ));
    pool.close_owner(id, Some(ReceiveError::Io));
    assert_eq!(lifecycle.status(), ReceiveStatus::Closing);
    assert_eq!(a_control.status(), ReceiveStatus::Closing);
    assert_eq!(b_control.status(), ReceiveStatus::Closing);
    assert_eq!(lifecycle.header().close_cause, Some(ReceiveError::Io));
    assert!(matches!(
        a_control.header().offer.unwrap().gate,
        AttemptGate::Claimed
    ));
    assert!(matches!(
        b_control.header().offer.unwrap().gate,
        AttemptGate::Cancelled(_)
    ));
    // No SDK objects or finish callback exist in this authority-only fixture.
    // The unspawned reservation's existing rejection path owns final cleanup.
}

#[test]
fn an_unclaimed_endpoint_seed_wakes_once_after_detached_before_its_off_caller_final_drop() {
    use std::cell::Cell;

    #[derive(Debug)]
    struct WakeTrace {
        stage: &'static str,
        calls: u8,
        thread: ThreadId,
        detached: bool,
        pool_unlocked: bool,
        endpoint_status: Option<ReceiveStatus>,
        owner_status: ReceiveStatus,
    }

    struct WakeProbe {
        calls: Cell<u8>,
        sent: mpsc::Sender<WakeTrace>,
        endpoint: Arc<OnceLock<(Arc<AtomicBool>, ReceiveControl)>>,
        pool: Arc<WorkerPool>,
        owner_control: ReceiveControl,
    }

    impl WakeProbe {
        fn record(&self, stage: &'static str) {
            let (detached, endpoint_status) =
                self.endpoint.get().map_or((false, None), |(ack, control)| {
                    (ack.load(Ordering::Acquire), Some(control.status()))
                });
            let pool_unlocked = self.pool.shared.state.try_lock().is_ok();
            let owner_status = self.owner_control.status();
            let _ = self.sent.send(WakeTrace {
                stage,
                calls: self.calls.get(),
                thread: thread::current().id(),
                detached,
                pool_unlocked,
                endpoint_status,
                owner_status,
            });
        }
    }

    impl Drop for WakeProbe {
        fn drop(&mut self) {
            self.record("drop");
        }
    }

    for fail_spawn in [false, true] {
        let caller = thread::current().id();
        let pool = WorkerPool::isolated();
        let lifecycle = control();
        let display_owner = Arc::new(());
        let retained = Arc::downgrade(&display_owner);
        let (id, reservation) = pool
            .reserve_owner(1, display_owner, lifecycle.clone(), ENDPOINT_TIMEOUT)
            .unwrap();
        let endpoint = Arc::new(OnceLock::new());
        let (sent, traces) = mpsc::channel();
        let probe = WakeProbe {
            calls: Cell::new(0), // Fn + Send, intentionally not Sync.
            sent,
            endpoint: endpoint.clone(),
            pool: pool.clone(),
            owner_control: lifecycle.clone(),
        };
        let lease = pool
            .reserve_surface(
                id,
                key(1, 10),
                Arc::new(()),
                Box::new(move || {
                    probe.calls.set(probe.calls.get() + 1);
                    probe.record("invoke");
                }),
            )
            .unwrap();
        assert!(endpoint
            .set((lease.detached.clone(), lease.control()))
            .is_ok());
        assert!(!lease.is_detached());
        assert!(retained.upgrade().is_some());

        // Never take the pending seed: the endpoint's sole wake stays admitted
        // in the pool when startup is cancelled or its worker cannot be spawned.
        if fail_spawn {
            pool.fail_next_worker_spawn();
            assert_eq!(
                reservation.spawn(|exit| {
                    exit.guest_cleaned();
                    WorkerExit {
                        wake: Box::new(|| {}),
                        cause: None,
                        payload: None,
                    }
                }),
                Err(ReceiveError::Unavailable)
            );
        } else {
            drop(reservation);
        }

        let invocation = receive(&traces);
        assert_eq!(
            invocation.stage, "invoke",
            "dropping the endpoint wake is not a Detached notification"
        );
        let final_drop = receive(&traces);
        assert_eq!(final_drop.stage, "drop");
        for trace in [&invocation, &final_drop] {
            assert_eq!(
                trace.calls, 1,
                "the admitted endpoint receives exactly one retirement wake"
            );
            assert!(
                trace.detached,
                "wake invocation and destruction follow the exact Detached acknowledgement"
            );
            assert_ne!(trace.thread, caller);
            assert!(trace.pool_unlocked);
            assert_eq!(trace.endpoint_status, Some(ReceiveStatus::Closing));
            assert!(matches!(
                trace.owner_status,
                ReceiveStatus::Closing | ReceiveStatus::Closed(_)
            ));
        }
        assert_eq!(invocation.thread, final_drop.thread);
        assert!(matches!(
            traces.recv_timeout(WAIT),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ));
        let deadline = Instant::now() + WAIT;
        while !matches!(lifecycle.status(), ReceiveStatus::Closed(_)) {
            assert!(Instant::now() < deadline, "owner cleanup did not complete");
            thread::yield_now();
        }
        assert!(lease.is_detached());
        assert!(retained.upgrade().is_none());
    }
}
