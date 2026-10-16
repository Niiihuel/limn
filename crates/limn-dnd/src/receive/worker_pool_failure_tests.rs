use std::{
    panic::panic_any,
    sync::mpsc,
    time::{Duration, Instant},
};

use super::*;
use crate::receive::ReceiveEpoch;

const WAIT: Duration = Duration::from_secs(3);

fn key(surface: usize) -> SurfaceKey {
    SurfaceKey {
        display: 1,
        surface,
    }
}

fn control() -> ReceiveControl {
    ReceiveControl::new(ReceiveEpoch(1)).unwrap()
}

fn receive<T>(receiver: &mpsc::Receiver<T>) -> T {
    receiver.recv_timeout(WAIT).expect("reaper progress")
}

#[derive(Default)]
struct Gate {
    open: Mutex<bool>,
    changed: Condvar,
}

impl Gate {
    fn wait(&self) -> bool {
        let guard = self.open.lock().unwrap_or_else(|error| error.into_inner());
        let (guard, _) = self
            .changed
            .wait_timeout_while(guard, WAIT * 2, |open| !*open)
            .unwrap_or_else(|error| error.into_inner());
        *guard
    }
}

struct Release(Arc<Gate>);

impl Drop for Release {
    fn drop(&mut self) {
        *self
            .0
            .open
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = true;
        self.0.changed.notify_all();
    }
}

struct PanicOnDrop(mpsc::Sender<()>);

impl Drop for PanicOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(());
        panic!("synthetic destructor failure");
    }
}

struct DropNotice(mpsc::Sender<thread::ThreadId>);

impl Drop for DropNotice {
    fn drop(&mut self) {
        let _ = self.0.send(thread::current().id());
    }
}

#[test]
fn reaper_failure_closes_other_owned_controls_and_rejects_new_workers() {
    let pool = WorkerPool::isolated();
    let held_control = control();
    held_control.header().status = ReceiveStatus::Ready;
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let gate = Arc::new(Gate::default());
    let release = Release(gate.clone());
    let (started_tx, started_rx) = mpsc::channel();
    let (opened_tx, opened_rx) = mpsc::channel();
    pool.reserve_owned(key(1), owner, held_control.clone())
        .unwrap()
        .spawn(move |exit| {
            let _ = started_tx.send(());
            let _ = opened_tx.send(gate.wait());
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(|| {}),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    receive(&started_rx);

    // This reservation exists before the crash, but has not started a worker.
    let previous_control = control();
    let (retired_tx, retired_rx) = mpsc::channel();
    let previous_owner = Arc::new(DropNotice(retired_tx.clone()));
    let previous_weak = Arc::downgrade(&previous_owner);
    let previous = pool
        .reserve_owned(key(4), previous_owner, previous_control.clone())
        .unwrap();

    let (drop_tx, drop_rx) = mpsc::channel();
    pool.reserve_owned(key(2), Arc::new(()), control())
        .unwrap()
        .spawn(move |exit| {
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(|| {}),
                cause: None,
                payload: Some(Box::new(PanicOnDrop(drop_tx))),
            }
        })
        .unwrap();
    receive(&drop_rx);
    let deadline = Instant::now() + WAIT;
    while held_control.status() != ReceiveStatus::Closing {
        assert!(
            Instant::now() < deadline,
            "a dead reaper left another control {:?}",
            held_control.status()
        );
        thread::yield_now();
    }
    assert!(
        weak.upgrade().is_some(),
        "unconfirmed worker retains its owner"
    );
    assert!(matches!(
        pool.reserve_legacy(key(1)),
        Err(ReceiveError::Busy)
    ));
    let (executed_tx, executed_rx) = mpsc::channel();
    let previous_job = DropNotice(retired_tx);
    let result = previous.spawn(move |exit| {
        let _retained = previous_job;
        let _ = executed_tx.send(());
        exit.guest_cleaned();
        WorkerExit {
            wake: Box::new(|| {}),
            cause: None,
            payload: None,
        }
    });
    assert_eq!(result, Err(ReceiveError::Unavailable));
    assert_eq!(previous_control.status(), ReceiveStatus::Closing);
    assert!(previous_weak.upgrade().is_some());
    assert!(matches!(
        executed_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert!(
        matches!(retired_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
        "a dead reaper retains the previous owner and job instead of dropping them in UI"
    );
    assert!(matches!(
        pool.reserve_owned(key(3), Arc::new(()), control()),
        Err(ReceiveError::Unavailable)
    ));
    drop(release);
    assert!(
        receive(&opened_rx),
        "the test released the held worker explicitly"
    );
}

#[test]
fn final_wake_panic_payload_drop_is_isolated_and_the_same_reaper_remains_usable() {
    let pool = WorkerPool::isolated();
    let (first_tx, first_rx) = mpsc::channel();
    let (panic_tx, panic_rx) = mpsc::channel();
    pool.reserve_owned(key(10), Arc::new(()), control())
        .unwrap()
        .spawn(move |exit| {
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(move || {
                    let _ = first_tx.send(thread::current().id());
                    panic_any(PanicOnDrop(panic_tx.clone()));
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    let first_thread = receive(&first_rx);
    receive(&panic_rx);

    let next_control = control();
    let callback_control = next_control.clone();
    let (next_tx, next_rx) = mpsc::channel();
    pool.reserve_owned(key(11), Arc::new(()), next_control)
        .unwrap()
        .spawn(move |exit| {
            exit.guest_cleaned();
            WorkerExit {
                wake: Box::new(move || {
                    let _ = next_tx.send((thread::current().id(), callback_control.status()));
                }),
                cause: None,
                payload: None,
            }
        })
        .unwrap();
    let (next_thread, status) = receive(&next_rx);
    assert_eq!(status, ReceiveStatus::Closed(None));
    assert_eq!(
        next_thread, first_thread,
        "the original fixed reaper survives"
    );
}
