use super::super::{coordinator::Coordinator, OfferId, ReceiveEpoch};
use super::ReceiveControl;
use crate::{DropDecision, ReceiveError, ReceiveEvent, ReceiveLimits, ReceiveStatus, SurfacePoint};
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const DATA: &[u8] = b"file:///tmp/received\r\n";
const PHASE: Duration = Duration::from_secs(5);

fn id(value: u64) -> OfferId {
    OfferId(value)
}

fn limits() -> ReceiveLimits {
    ReceiveLimits {
        startup_timeout: PHASE,
        transfer_timeout: PHASE,
        decision_timeout: PHASE,
        ..ReceiveLimits::default()
    }
}

fn fixture(seed: u64) -> (Coordinator, ReceiveControl, Instant) {
    let control =
        ReceiveControl::new(ReceiveEpoch(seed)).expect("shared control capability is available");
    let coordinator = Coordinator::with_control(limits(), control.clone(), || {}).unwrap();
    (coordinator, control, Instant::now())
}

fn arm(coordinator: &mut Coordinator, now: Instant) -> ReceiveEpoch {
    let sync = coordinator
        .next_sync(now)
        .unwrap()
        .expect("a disarmed context requests its barrier");
    coordinator.sync_done(sync.epoch, now).unwrap();
    sync.epoch
}

fn enter_copy(coordinator: &mut Coordinator, value: u64) {
    coordinator
        .enter(id(value), SurfacePoint { x: 10.0, y: 20.0 }, 3, true)
        .unwrap();
    coordinator.action(id(value), true).unwrap();
}

fn ready(coordinator: &mut Coordinator, value: u64, now: Instant) {
    enter_copy(coordinator, value);
    coordinator.start(id(value), now).unwrap();
    coordinator.complete(id(value), DATA, now).unwrap();
}

fn assert_terminal(events: &[ReceiveEvent], value: u64, expected: Result<(), ReceiveError>) {
    let terminals: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ReceiveEvent::DropEnded { id, result } => Some((*id, *result)),
            _ => None,
        })
        .collect();
    assert_eq!(terminals, vec![(id(value), expected)]);
}

#[test]
fn invalidate_reconciles_ready_in_worker_or_leaves_a_transferred_old_epoch() {
    for poll_wins in [false, true] {
        let (mut coordinator, control, now) = fixture(17);
        let epoch_a = arm(&mut coordinator, now);
        ready(&mut coordinator, 1, now);
        let epoch_b = control.invalidate_pending().unwrap();
        assert_ne!(epoch_a, epoch_b);

        // Invalidating only changes the header. Poll may already own the old batch.
        let transferred = if poll_wins {
            control.poll()
        } else {
            Vec::new()
        };
        if poll_wins {
            assert!(
                matches!(transferred.as_slice(), [ReceiveEvent::DropStarted { epoch, .. }, ReceiveEvent::DropReady { .. }]
                if *epoch == epoch_a && *epoch != epoch_b)
            );
        }
        coordinator.reconcile(now).unwrap();
        let events = control.poll();
        assert!(!events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
        assert_terminal(&events, 1, Err(ReceiveError::Cancelled));
        if !poll_wins {
            assert!(
                matches!(events.as_slice(), [ReceiveEvent::DropStarted { epoch, .. }, ReceiveEvent::DropEnded { .. }]
                if *epoch == epoch_a)
            );
        }
        assert!(control.poll().is_empty());
    }
}

#[test]
fn queued_accept_loses_to_invalidation_and_claim_rechecks_its_deadline() {
    for invalidate in [false, true] {
        let (mut coordinator, control, now) = fixture(17);
        arm(&mut coordinator, now);
        ready(&mut coordinator, 1, now);
        control.poll();
        control.decide(id(1), DropDecision::Accept).unwrap();
        let mut calls = 0;
        let expected = if invalidate {
            control.invalidate_pending().unwrap();
            coordinator.reconcile(now).unwrap();
            assert_eq!(control.take_decision(), None);
            assert_eq!(
                coordinator.decide(id(1), DropDecision::Accept, now, |_| {
                    calls += 1;
                    Ok(())
                }),
                Err(ReceiveError::StaleOffer)
            );
            ReceiveError::Cancelled
        } else {
            let (actual, decision) = control.take_decision().expect("one pending decision");
            assert_eq!(actual, id(1));
            assert_eq!(decision, DropDecision::Accept);
            assert_eq!(control.take_decision(), None);
            let deadline = {
                let header = control.header();
                header.offer.unwrap().deadline.unwrap()
            };
            assert_eq!(
                coordinator.decide(actual, decision, deadline, |_| {
                    calls += 1;
                    Ok(())
                }),
                Err(ReceiveError::Timeout)
            );
            ReceiveError::Timeout
        };
        assert_eq!(calls, 0);
        assert_terminal(&control.poll(), 1, Err(expected));
    }

    // The worker may already own Accept when invalidation changes the header.
    // Claim must read that header even without a preceding reconciliation tick.
    let (mut coordinator, control, now) = fixture(37);
    let epoch_a = arm(&mut coordinator, now);
    ready(&mut coordinator, 1, now);
    control.poll();
    control.decide(id(1), DropDecision::Accept).unwrap();
    let (actual, decision) = control.take_decision().unwrap();
    control.invalidate_pending().unwrap();
    assert_eq!(
        control.claim(actual, epoch_a, now),
        Err(ReceiveError::StaleOffer)
    );
    let mut calls = 0;
    assert_eq!(
        coordinator.decide(actual, decision, now, |_| {
            calls += 1;
            Ok(())
        }),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(calls, 0);
    coordinator.reconcile(now).unwrap();
    assert_terminal(&control.poll(), 1, Err(ReceiveError::Cancelled));

    // A real wake consumer may poll and enqueue Accept immediately. The header
    // must already describe Ready/Ended, without holding either publication lock.
    let (done_tx, done_rx) = mpsc::channel();
    let probe = thread::spawn(move || {
        let control = ReceiveControl::new(ReceiveEpoch(47)).unwrap();
        let callback_control = control.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let (ended_tx, ended_rx) = mpsc::channel();
        let mut coordinator = Coordinator::with_control(limits(), control.clone(), move || {
            for event in callback_control.poll() {
                match event {
                    ReceiveEvent::DropReady { id, .. } => {
                        ready_tx
                            .send(callback_control.decide(id, DropDecision::Accept))
                            .unwrap();
                    }
                    ReceiveEvent::DropEnded { id, result } => {
                        ended_tx.send((id, result)).unwrap();
                    }
                    _ => {}
                }
            }
        })
        .unwrap();
        let now = Instant::now();
        arm(&mut coordinator, now);
        ready(&mut coordinator, 1, now);
        assert_eq!(ready_rx.try_recv().unwrap(), Ok(()));
        let (actual, decision) = control.take_decision().unwrap();
        coordinator
            .decide(actual, decision, now, |_| {
                assert_eq!(control.status(), ReceiveStatus::Ready);
                Ok(())
            })
            .unwrap();
        assert_eq!(ended_rx.try_recv().unwrap(), (id(1), Ok(())));
        coordinator.reconcile(now).unwrap();
        enter_copy(&mut coordinator, 2);
        done_tx.send(()).unwrap();
    });
    let completed = done_rx.recv_timeout(Duration::from_secs(2));
    if !matches!(completed, Err(mpsc::RecvTimeoutError::Timeout)) {
        probe.join().unwrap();
    }
    assert_eq!(
        completed,
        Ok(()),
        "publication callbacks may reenter control"
    );
}

#[test]
fn a_claimed_sdk_attempt_does_not_block_invalidate_close_or_poll() {
    for close in [false, true] {
        for outcome in [Ok(()), Err(ReceiveError::Io)] {
            let (mut coordinator, control, now) = fixture(17);
            arm(&mut coordinator, now);
            ready(&mut coordinator, 1, now);
            control.poll();
            control.decide(id(1), DropDecision::Accept).unwrap();
            let (actual, decision) = control.take_decision().unwrap();
            let (entered_tx, entered_rx) = mpsc::channel();
            let (release_tx, release_rx) = mpsc::channel();
            let worker = thread::spawn(move || {
                let result = coordinator.decide(actual, decision, now, |actual| {
                    assert_eq!(actual, id(1));
                    entered_tx.send(()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(10)).unwrap();
                    outcome
                });
                coordinator.reconcile(now).unwrap();
                result
            });
            entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            let observer_control = control.clone();
            let (observed_tx, observed_rx) = mpsc::channel();
            let observer = thread::spawn(move || {
                if close {
                    observer_control.close().unwrap();
                } else {
                    observer_control.invalidate_pending().unwrap();
                }
                let status = observer_control.status();
                observed_tx.send((status, observer_control.poll())).unwrap();
            });
            let observed = observed_rx.recv_timeout(Duration::from_secs(2));
            // Release even if observation failed, so the regression cannot strand the join.
            release_tx.send(()).unwrap();
            let (status, early_events) =
                observed.expect("controls return while SDK remains stopped");
            assert_eq!(worker.join().unwrap(), outcome);
            observer.join().unwrap();
            assert!(!early_events
                .iter()
                .any(|event| matches!(event, ReceiveEvent::DropEnded { .. })));
            if close {
                assert_eq!(status, ReceiveStatus::Closing);
            }
            assert_terminal(&control.poll(), 1, outcome);
            assert!(control.poll().is_empty());
        }
    }
}

#[test]
fn barriers_coalesce_epochs_and_only_a_new_enter_after_the_current_cut_is_valid() {
    let (mut coordinator, control, now) = fixture(17);
    assert_eq!(control.status(), ReceiveStatus::Starting);
    assert_eq!(
        coordinator.enter(id(1), SurfacePoint { x: 1.0, y: 2.0 }, 3, true),
        Err(ReceiveError::InvalidPhase)
    );
    let epoch_a = arm(&mut coordinator, now);
    enter_copy(&mut coordinator, 1);
    let epoch_b = control.invalidate_pending().unwrap();
    coordinator.reconcile(now).unwrap();
    let sync_b = coordinator.next_sync(now).unwrap().unwrap();
    assert_eq!(sync_b.epoch, epoch_b);
    let epoch_c = control.invalidate_pending().unwrap();
    assert_ne!(epoch_c, epoch_a);
    assert_ne!(epoch_c, epoch_b);
    assert_eq!(coordinator.next_sync(now).unwrap(), None);
    coordinator.sync_done(epoch_b, now).unwrap();
    assert_eq!(
        coordinator.enter(id(2), SurfacePoint { x: 1.0, y: 2.0 }, 3, true),
        Err(ReceiveError::InvalidPhase)
    );
    let sync_c = coordinator.next_sync(now).unwrap().unwrap();
    assert_eq!(sync_c.epoch, epoch_c);
    assert_eq!(
        coordinator.sync_done(epoch_b, now),
        Err(ReceiveError::InvalidPhase)
    );
    coordinator.sync_done(epoch_c, now).unwrap();
    assert_eq!(coordinator.start(id(1), now), Err(ReceiveError::StaleOffer));
    enter_copy(&mut coordinator, 3);
    coordinator.start(id(3), now).unwrap();
    let events = control.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropStarted { epoch, .. }] if *epoch == epoch_c)
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropEnded { .. })));
}

#[test]
fn barrier_deadlines_do_not_renew_and_postdecode_time_is_checked_before_ready() {
    let (mut coordinator, control, now) = fixture(17);
    arm(&mut coordinator, now);
    let epoch_b = control.invalidate_pending().unwrap();
    coordinator.reconcile(now).unwrap();
    let sync_b = coordinator.next_sync(now).unwrap().unwrap();
    assert_eq!(sync_b.deadline, now + PHASE);
    control.invalidate_pending().unwrap();
    assert_eq!(
        coordinator
            .next_sync(now + PHASE - Duration::from_millis(1))
            .unwrap(),
        None
    );
    coordinator.reconcile(now + PHASE).unwrap();
    assert_eq!(control.status(), ReceiveStatus::Closing);
    assert_eq!(
        coordinator.next_sync(now + PHASE),
        Err(ReceiveError::Closed)
    );
    assert!(coordinator.sync_done(epoch_b, now + PHASE).is_err());

    let (mut failed, failed_control, now) = fixture(27);
    let initial = failed.next_sync(now).unwrap().unwrap();
    failed.sync_failed(initial.epoch, ReceiveError::Io).unwrap();
    assert_eq!(failed_control.status(), ReceiveStatus::Closing);
    assert_eq!(failed.next_sync(now), Err(ReceiveError::Closed));

    let (mut coordinator, control, now) = fixture(37);
    arm(&mut coordinator, now);
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    let paths = crate::uri_list::decode_received(DATA, &limits()).unwrap();
    assert_eq!(
        coordinator.complete_validated(id(1), paths, now + PHASE),
        Err(ReceiveError::Timeout)
    );
    let events = control.poll();
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
    assert_terminal(&events, 1, Err(ReceiveError::Timeout));
}

#[test]
fn terminal_ack_is_exact_and_epoch_exhaustion_requests_closing_without_wrap() {
    let (mut coordinator, control, now) = fixture(17);
    arm(&mut coordinator, now);
    ready(&mut coordinator, 1, now);
    coordinator
        .decide(id(1), DropDecision::Accept, now, |_| Ok(()))
        .unwrap();
    assert_terminal(&control.poll(), 1, Ok(()));
    coordinator.reconcile(now).unwrap();
    enter_copy(&mut coordinator, 2);
    coordinator.start(id(2), now).unwrap();
    control.acknowledge_ended(id(1));
    coordinator.reconcile(now).unwrap();
    assert_eq!(
        coordinator.enter(id(3), SurfacePoint { x: 1.0, y: 2.0 }, 3, true),
        Err(ReceiveError::Busy)
    );
    coordinator.complete(id(2), DATA, now).unwrap();
    control.decide(id(2), DropDecision::Accept).unwrap();
    assert_eq!(
        control.decide(id(1), DropDecision::Reject),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(control.take_decision(), Some((id(2), DropDecision::Accept)));
    control.cancel(id(2)).unwrap();
    coordinator.reconcile(now).unwrap();
    assert_terminal(&control.poll(), 2, Err(ReceiveError::Cancelled));
    coordinator.reconcile(now).unwrap();
    enter_copy(&mut coordinator, 3);

    let (mut exhausted, exhausted_control, now) = fixture(u64::MAX);
    arm(&mut exhausted, now);
    enter_copy(&mut exhausted, 1);
    exhausted.start(id(1), now).unwrap();
    assert_eq!(
        exhausted_control.invalidate_pending(),
        Err(ReceiveError::ResourceLimit)
    );
    assert_eq!(exhausted_control.status(), ReceiveStatus::Closing);
    assert_eq!(
        exhausted_control.invalidate_pending(),
        Err(ReceiveError::Closed)
    );
    exhausted.reconcile(now).unwrap();
    let events = exhausted_control.poll();
    assert!(
        matches!(&events[0], ReceiveEvent::DropStarted { epoch, .. } if *epoch == ReceiveEpoch(u64::MAX))
    );
    assert_terminal(&events, 1, Err(ReceiveError::Cancelled));
    assert_eq!(exhausted.next_sync(now), Err(ReceiveError::Closed));
    assert_eq!(exhausted_control.status(), ReceiveStatus::Closing);
}

#[test]
fn a_stale_caller_timestamp_cannot_claim_after_the_real_decision_deadline() {
    let control = ReceiveControl::new(ReceiveEpoch(17)).unwrap();
    let limits = ReceiveLimits {
        decision_timeout: Duration::from_millis(20),
        ..limits()
    };
    let mut coordinator = Coordinator::with_control(limits, control.clone(), || {}).unwrap();
    let now = Instant::now();
    arm(&mut coordinator, now);
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    let before_deadline = Instant::now();
    coordinator.complete(id(1), DATA, before_deadline).unwrap();
    control.poll();
    control.decide(id(1), DropDecision::Accept).unwrap();
    let (actual, decision) = control
        .take_decision()
        .expect("Accept was admitted before its deadline");
    let deadline = {
        let header = control.header();
        header.offer.unwrap().deadline.unwrap()
    };
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining);
    }
    let mut calls = 0;
    assert_eq!(
        coordinator.decide(actual, decision, before_deadline, |_| {
            calls += 1;
            Ok(())
        }),
        Err(ReceiveError::Timeout)
    );
    assert_eq!(calls, 0);
    assert_terminal(&control.poll(), 1, Err(ReceiveError::Timeout));
    assert!(control.poll().is_empty());
}

#[test]
fn a_stale_postdecode_timestamp_cannot_publish_ready_after_the_real_transfer_deadline() {
    let control = ReceiveControl::new(ReceiveEpoch(17)).unwrap();
    let limits = ReceiveLimits {
        transfer_timeout: Duration::from_millis(20),
        ..limits()
    };
    let paths = crate::uri_list::decode_received(DATA, &limits).unwrap();
    let mut coordinator = Coordinator::with_control(limits, control.clone(), || {}).unwrap();
    let now = Instant::now();
    arm(&mut coordinator, now);
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    let deadline = {
        let header = control.header();
        header.offer.unwrap().deadline.unwrap()
    };
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining);
    }

    assert_eq!(
        coordinator.complete_validated(id(1), paths, now),
        Err(ReceiveError::Timeout)
    );
    let events = control.poll();
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
    assert_terminal(&events, 1, Err(ReceiveError::Timeout));
    assert!(control.poll().is_empty());
}
