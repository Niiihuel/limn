use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{mpsc, Arc, Barrier, Condvar, Mutex},
    thread::{self, ThreadId},
    time::{Duration, Instant},
};

use super::*;
use crate::receive::{ReceiveEpoch, ReceiveStatus};

const WAIT: Duration = Duration::from_secs(3);

fn key(surface: usize) -> SurfaceKey {
    SurfaceKey {
        display: 1,
        surface,
    }
}

fn new_control() -> ReceiveControl {
    ReceiveControl::new(ReceiveEpoch(1)).unwrap()
}

fn receive<T>(receiver: &mpsc::Receiver<T>) -> T {
    receiver.recv_timeout(WAIT).expect("lifecycle progress")
}

#[derive(Debug)]
struct Trace {
    label: &'static str,
    thread: ThreadId,
    status: Option<ReceiveStatus>,
}

struct DropProbe {
    label: &'static str,
    sent: mpsc::Sender<Trace>,
    control: Option<ReceiveControl>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        let _ = self.sent.send(Trace {
            label: self.label,
            thread: thread::current().id(),
            status: self.control.as_ref().map(ReceiveControl::status),
        });
    }
}

#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) -> bool {
        // A failed assertion must not leave a detached test thread waiting forever.
        let guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, WAIT, |open| !*open)
            .unwrap_or_else(|error| error.into_inner());
        *guard
    }

    fn release(&self) {
        *self.open.lock().unwrap_or_else(|error| error.into_inner()) = true;
        self.changed.notify_all();
    }
}

struct ReleaseOnDrop(Arc<Gate>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

struct TlsExit {
    gate: Arc<Gate>,
    entered: mpsc::Sender<()>,
    opened: mpsc::Sender<bool>,
}

impl Drop for TlsExit {
    fn drop(&mut self) {
        let _ = self.entered.send(());
        let _ = self.opened.send(self.gate.wait());
    }
}

thread_local! {
    static TLS_EXIT: RefCell<Option<TlsExit>> = const { RefCell::new(None) };
}

fn wait_status(control: &ReceiveControl, expected: ReceiveStatus) {
    let deadline = Instant::now() + WAIT;
    while control.status() != expected {
        assert!(
            Instant::now() < deadline,
            "status {:?}, expected {expected:?}",
            control.status()
        );
        thread::yield_now();
    }
}

#[test]
fn pool_reservations_share_four_slots_and_exact_surface_exclusion_with_legacy() {
    let pool = WorkerPool::isolated();
    let legacy = pool
        .reserve_legacy(key(10))
        .expect("legacy reserves one shared slot");
    assert!(matches!(
        pool.reserve_owned(key(10), Arc::new(()), new_control()),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_legacy(key(10)),
        Err(ReceiveError::Busy)
    ));

    // Surface pointer equality on different displays is not the same identity.
    let other_display = SurfaceKey {
        display: 2,
        surface: 10,
    };
    let one = pool
        .reserve_owned(other_display, Arc::new(()), new_control())
        .unwrap();
    let two = pool
        .reserve_owned(key(11), Arc::new(()), new_control())
        .unwrap();
    let three = pool
        .reserve_owned(key(12), Arc::new(()), new_control())
        .unwrap();
    assert!(matches!(
        pool.reserve_legacy(key(13)),
        Err(ReceiveError::Busy)
    ));
    assert!(matches!(
        pool.reserve_owned(key(13), Arc::new(()), new_control()),
        Err(ReceiveError::Busy)
    ));
    // The legacy caller releases this guard only after its existing synchronous join.
    drop(legacy);
    let reused = pool
        .reserve_legacy(key(10))
        .expect("joined legacy returns its slot");
    drop((one, two, three, reused));
}

#[test]
fn pool_cleanup_signal_before_handle_handoff_remains_pending_until_real_join() {
    let pool = WorkerPool::isolated();
    let control = new_control();
    let reservation = pool
        .reserve_owned(key(20), Arc::new(()), control.clone())
        .unwrap();
    // No guest exists in this fixture: early certification is safe and deliberately
    // precedes handle handoff. It must not be lost or make an empty slot reusable.
    reservation.exit_signal().guest_cleaned();
    assert!(!matches!(control.status(), ReceiveStatus::Closed(_)));
    assert!(matches!(
        pool.reserve_legacy(key(20)),
        Err(ReceiveError::Busy)
    ));
    let gate = Arc::new(Gate::default());
    let release = ReleaseOnDrop(gate.clone());
    let (started_tx, started_rx) = mpsc::channel();
    let (closed_tx, closed_rx) = mpsc::channel();
    let (opened_tx, opened_rx) = mpsc::channel();
    let callback_control = control.clone();
    reservation
        .spawn(move |_| {
            let _ = started_tx.send(());
            let _ = opened_tx.send(gate.wait());
            WorkerExit {
                wake: Box::new(move || {
                    let _ = closed_tx.send(callback_control.status());
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    receive(&started_rx);
    control.close().unwrap();
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(matches!(
        pool.reserve_legacy(key(20)),
        Err(ReceiveError::Busy)
    ));
    release.0.release();
    assert_eq!(receive(&closed_rx), ReceiveStatus::Closed(None));
    assert!(
        receive(&opened_rx),
        "the gate opened before its safety timeout"
    );
    let _reused = pool.reserve_legacy(key(20)).unwrap();
}

#[test]
fn pool_tls_join_precedes_payload_owner_release_closed_and_send_only_wake() {
    let ui_thread = thread::current().id();
    let pool = WorkerPool::isolated();
    let control = new_control();
    let (trace_tx, trace_rx) = mpsc::channel();
    let owner = Arc::new(DropProbe {
        label: "owner",
        sent: trace_tx.clone(),
        control: Some(control.clone()),
    });
    let weak = Arc::downgrade(&owner);
    let reservation = pool.reserve_owned(key(30), owner, control.clone()).unwrap();
    let gate = Arc::new(Gate::default());
    let release = ReleaseOnDrop(gate.clone());
    let (tls_tx, tls_rx) = mpsc::channel();
    let (opened_tx, opened_rx) = mpsc::channel();
    let callback_control = control.clone();
    let callback_pool = pool.clone();
    // Cell is Send but not Sync. This exact box must move worker -> reaper.
    let calls = Cell::new(0u8);
    let callback_trace = trace_tx.clone();
    let callback_drop = DropProbe {
        label: "wake drop",
        sent: trace_tx.clone(),
        control: Some(control.clone()),
    };
    let wake: Wake = Box::new(move || {
        let _retained = &callback_drop;
        calls.set(calls.get() + 1);
        let status = callback_control.status();
        if calls.get() == 2 {
            let _guard = callback_pool
                .reserve_legacy(key(30))
                .expect("slot released before final wake");
        }
        let _ = callback_trace.send(Trace {
            label: if calls.get() == 1 {
                "worker wake"
            } else {
                "closed wake"
            },
            thread: thread::current().id(),
            status: Some(status),
        });
    });
    let payload = DropProbe {
        label: "payload",
        sent: trace_tx,
        control: Some(control.clone()),
    };
    reservation
        .spawn(move |exit| {
            TLS_EXIT.with(|slot| {
                *slot.borrow_mut() = Some(TlsExit {
                    gate,
                    entered: tls_tx,
                    opened: opened_tx,
                })
            });
            wake();
            exit.guest_cleaned();
            WorkerExit {
                wake,
                cause: None,
                payload: Some(Box::new(payload)),
            }
        })
        .unwrap();
    let worker_wake = receive(&trace_rx);
    assert_eq!(worker_wake.label, "worker wake");
    receive(&tls_rx);
    control.close().unwrap();
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert!(
        weak.upgrade().is_some(),
        "owner survives the completed worker body and TLS"
    );
    assert!(matches!(
        pool.reserve_legacy(key(30)),
        Err(ReceiveError::Busy)
    ));
    // Joining one worker must not hold the pool/control lock or block another slot.
    let _unrelated = pool.reserve_legacy(key(31)).unwrap();
    let second_control = new_control();
    let callback_control = second_control.clone();
    let (second_tx, second_rx) = mpsc::channel();
    pool.reserve_owned(key(32), Arc::new(()), second_control)
        .unwrap()
        .spawn(move |exit| {
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(move || {
                    let _ = second_tx.send((thread::current().id(), callback_control.status()));
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    release.0.release();
    let payload = receive(&trace_rx);
    let owner = receive(&trace_rx);
    let closed = receive(&trace_rx);
    let wake_drop = receive(&trace_rx);
    assert_eq!(
        (payload.label, owner.label, closed.label),
        ("payload", "owner", "closed wake")
    );
    assert_eq!(payload.status, Some(ReceiveStatus::Closing));
    assert_eq!(owner.status, Some(ReceiveStatus::Closing));
    assert_eq!(closed.status, Some(ReceiveStatus::Closed(None)));
    assert_eq!(wake_drop.label, "wake drop");
    assert_eq!(wake_drop.status, Some(ReceiveStatus::Closed(None)));
    assert_eq!(payload.thread, owner.thread);
    assert_eq!(owner.thread, closed.thread);
    assert_eq!(closed.thread, wake_drop.thread);
    assert_ne!(closed.thread, worker_wake.thread);
    assert_ne!(closed.thread, ui_thread);
    assert!(weak.upgrade().is_none());
    assert!(
        receive(&opened_rx),
        "TLS left through release, never the safety timeout"
    );
    let (second_thread, second_status) = receive(&second_rx);
    assert_eq!(second_status, ReceiveStatus::Closed(None));
    assert_eq!(
        second_thread, closed.thread,
        "owned reservations share one fixed reaper"
    );
}

#[test]
fn pool_caught_startup_panic_recovers_wake_but_unconfirmed_panic_keeps_owner_and_slot() {
    let pool = WorkerPool::isolated();
    let control = new_control();
    let (closed_tx, closed_rx) = mpsc::channel();
    let callback_control = control.clone();
    let reservation = pool
        .reserve_owned(key(40), Arc::new(()), control.clone())
        .unwrap();
    reservation
        .spawn(move |exit| {
            // This box stands for State/Coordinator outside the pump catch. Startup
            // fails, but its still-owned wake and certified cleanup survive unwind.
            let wake: Wake = Box::new(move || {
                let _ = closed_tx.send(callback_control.status());
            });
            let failure = catch_unwind(AssertUnwindSafe(|| panic!("synthetic startup failure")));
            exit.guest_cleaned();
            WorkerExit {
                wake,
                cause: Some(ReceiveError::Io),
                payload: failure.err(),
            }
        })
        .unwrap();
    assert_eq!(
        receive(&closed_rx),
        ReceiveStatus::Closed(Some(ReceiveError::Io))
    );
    let _reused = pool.reserve_legacy(key(40)).unwrap();

    let unconfirmed = new_control();
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let reservation = pool
        .reserve_owned(key(41), owner, unconfirmed.clone())
        .unwrap();
    let (unwind_tx, unwind_rx) = mpsc::channel();
    reservation
        .spawn(move |_| {
            struct Unwound(mpsc::Sender<()>);
            impl Drop for Unwound {
                fn drop(&mut self) {
                    let _ = self.0.send(());
                }
            }
            let _unwound = Unwound(unwind_tx);
            panic!("cleanup was not certified");
        })
        .unwrap();
    receive(&unwind_rx);
    wait_status(&unconfirmed, ReceiveStatus::Closing);
    assert!(weak.upgrade().is_some());
    assert!(matches!(
        pool.reserve_legacy(key(41)),
        Err(ReceiveError::Busy)
    ));
    // No test-only release can pretend that unknown guest cleanup succeeded.
}

#[test]
fn pool_spawn_failures_retire_jobs_off_ui_and_registry_credit_is_not_refunded() {
    let ui_thread = thread::current().id();
    let unavailable = WorkerPool::isolated();
    unavailable.fail_reaper_start();
    assert!(matches!(
        unavailable.reserve_owned(key(50), Arc::new(()), new_control()),
        Err(ReceiveError::Unavailable)
    ));
    for _ in 0..64 {
        assert_eq!(unavailable.consume_registry(), Ok(()));
    }
    assert_eq!(
        unavailable.consume_registry(),
        Err(ReceiveError::ResourceLimit)
    );

    let pool = WorkerPool::isolated();
    let (drops_tx, drops_rx) = mpsc::channel();
    let owner = Arc::new(DropProbe {
        label: "owner",
        sent: drops_tx.clone(),
        control: None,
    });
    let reservation = pool.reserve_owned(key(51), owner, new_control()).unwrap();
    pool.fail_next_worker_spawn();
    let job = DropProbe {
        label: "job",
        sent: drops_tx.clone(),
        control: None,
    };
    let callback_drop = DropProbe {
        label: "wake",
        sent: drops_tx,
        control: None,
    };
    let (called_tx, called_rx) = mpsc::channel();
    let wake: Wake = Box::new(move || {
        let _retained = &callback_drop;
        let _ = called_tx.send(());
    });
    let result = reservation.spawn(move |exit| {
        let _job = job;
        exit.guest_cleaned();
        WorkerExit {
            wake,
            cause: None,
            payload: None,
        }
    });
    assert_eq!(result, Err(ReceiveError::Unavailable));
    let first_drop = receive(&drops_rx);
    let second_drop = receive(&drops_rx);
    let owner_drop = receive(&drops_rx);
    let mut jobs = [first_drop.label, second_drop.label];
    jobs.sort();
    assert_eq!(jobs, ["job", "wake"]);
    assert_eq!(owner_drop.label, "owner");
    assert_ne!(first_drop.thread, ui_thread);
    assert_eq!(first_drop.thread, second_drop.thread);
    assert_eq!(second_drop.thread, owner_drop.thread);
    assert!(matches!(
        called_rx.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
    // The reaper drops the owner before it releases the slot: wait for the release.
    let deadline = Instant::now() + WAIT;
    let retry = loop {
        match pool.reserve_legacy(key(51)) {
            Err(ReceiveError::Busy) if Instant::now() < deadline => thread::yield_now(),
            result => break result.expect("the failed spawn releases its slot"),
        }
    };

    // Reservations and a failed spawn precede get_registry and spend no credit.
    for _ in 0..64 {
        assert_eq!(pool.consume_registry(), Ok(()));
    }
    assert_eq!(pool.consume_registry(), Err(ReceiveError::ResourceLimit));
    drop(retry);
    let _after_cleanup = pool.reserve_legacy(key(52)).unwrap();
    assert_eq!(pool.consume_registry(), Err(ReceiveError::ResourceLimit));

    let concurrent = WorkerPool::isolated();
    let barrier = Arc::new(Barrier::new(9));
    let mut workers = Vec::new();
    for _ in 0..8 {
        let pool = concurrent.clone();
        let barrier = barrier.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            (0..16).filter(|_| pool.consume_registry().is_ok()).count()
        }));
    }
    barrier.wait();
    let admitted: usize = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .sum();
    assert_eq!(admitted, 64);
    assert_eq!(
        concurrent.consume_registry(),
        Err(ReceiveError::ResourceLimit)
    );
}
