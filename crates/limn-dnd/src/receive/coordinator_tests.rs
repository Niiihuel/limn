use super::super::{control::ReceiveControl, OfferId, ReceiveEpoch};
use super::Coordinator;
use crate::{DropDecision, ReceiveError, ReceiveEvent, ReceiveLimits, SurfacePoint};
use std::time::{Duration, Instant};

const DATA: &[u8] = b"file:///tmp/received\r\n";

fn id(value: u64) -> OfferId {
    OfferId(value)
}

fn point(x: f64) -> SurfacePoint {
    SurfacePoint { x, y: x + 1.0 }
}

fn setup() -> (Coordinator, Instant) {
    let limits = ReceiveLimits {
        transfer_timeout: Duration::from_millis(5),
        decision_timeout: Duration::from_millis(2),
        ..ReceiveLimits::default()
    };
    let control = ReceiveControl::new(ReceiveEpoch(17)).unwrap();
    let mut coordinator = Coordinator::with_control(limits, control, || {})
        .expect("receive-only coordinator is available");
    let now = Instant::now();
    let initial = coordinator.next_sync(now).unwrap().unwrap();
    coordinator.sync_done(initial.epoch, now).unwrap();
    (coordinator, now)
}

fn enter_copy(coordinator: &mut Coordinator, value: u64) {
    coordinator.enter(id(value), point(10.0), 3, true).unwrap();
    coordinator.action(id(value), true).unwrap();
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
fn started_keeps_enter_epoch_and_freezes_the_last_predrop_position() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.motion(id(1), point(20.0)).unwrap();
    coordinator.start(id(1), now).unwrap();
    coordinator.motion(id(1), point(99.0)).unwrap();
    coordinator.leave(id(1)).unwrap();
    let events = coordinator.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropStarted { id: actual, epoch, point: actual_point }]
        if *actual == id(1) && *epoch == ReceiveEpoch(17) && *actual_point == point(20.0))
    );
    coordinator.complete(id(1), DATA, now).unwrap();
    assert!(
        matches!(coordinator.poll().as_slice(), [ReceiveEvent::DropReady { id: actual, .. }] if *actual == id(1))
    );
}

#[test]
fn another_offer_and_its_callbacks_cannot_replace_the_active_offer() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    assert_eq!(
        coordinator.enter(id(2), point(200.0), 3, true),
        Err(ReceiveError::Busy)
    );
    assert_eq!(
        coordinator.action(id(2), true),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(
        coordinator.motion(id(2), point(200.0)),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(coordinator.leave(id(2)), Err(ReceiveError::StaleOffer));
    coordinator.cancel(id(2)).unwrap();
    coordinator.start(id(1), now).unwrap();
    assert_eq!(
        coordinator.complete(id(2), DATA, now),
        Err(ReceiveError::StaleOffer)
    );
    coordinator.complete(id(1), DATA, now).unwrap();
    let mut finishes = Vec::new();
    coordinator
        .decide(id(1), DropDecision::Accept, now, |actual| {
            finishes.push(actual);
            Ok(())
        })
        .unwrap();
    assert_eq!(finishes, vec![id(1)]);
    let events = coordinator.poll();
    assert!(
        matches!(&events[0], ReceiveEvent::DropStarted { point: actual, .. } if *actual == point(10.0))
    );
    assert_terminal(&events, 1, Ok(()));
}

#[test]
fn callbacks_from_a_drained_offer_cannot_affect_the_next_offer() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.cancel(id(1)).unwrap();
    coordinator.poll();
    enter_copy(&mut coordinator, 2);
    coordinator.cancel(id(1)).unwrap();
    assert_eq!(
        coordinator.complete(id(1), DATA, now),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(
        coordinator.action(id(1), false),
        Err(ReceiveError::StaleOffer)
    );
    coordinator.start(id(2), now).unwrap();
    assert!(
        matches!(coordinator.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id(2))
    );
}

#[test]
fn incompatible_protocol_or_mime_is_rejected_before_started() {
    let (mut coordinator, _) = setup();
    for version in [0, 1, 2] {
        assert_eq!(
            coordinator.enter(id(1), point(1.0), version, true),
            Err(ReceiveError::Unavailable)
        );
    }
    assert_eq!(
        coordinator.enter(id(1), point(1.0), 3, false),
        Err(ReceiveError::InvalidUriList)
    );
    assert!(coordinator.poll().is_empty());
    enter_copy(&mut coordinator, 2);
}

#[test]
fn a_drop_requires_the_exact_copy_selection() {
    for selected in [None, Some(false)] {
        let (mut coordinator, now) = setup();
        coordinator.enter(id(1), point(1.0), 3, true).unwrap();
        if let Some(selected) = selected {
            coordinator.action(id(1), selected).unwrap();
        }
        assert_eq!(
            coordinator.start(id(1), now),
            Err(ReceiveError::InvalidPhase)
        );
        let events = coordinator.poll();
        assert!(events.len() <= 1);
        assert!(events
            .iter()
            .all(|event| matches!(event, ReceiveEvent::Left { id: actual } if *actual == id(1))));
        assert_eq!(
            coordinator.action(id(1), true),
            Err(ReceiveError::StaleOffer)
        );
        assert_eq!(coordinator.start(id(1), now), Err(ReceiveError::StaleOffer));
        enter_copy(&mut coordinator, 2);
        coordinator.start(id(2), now).unwrap();
        assert!(
            matches!(coordinator.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id(2))
        );
    }
}

#[test]
fn premature_accept_does_not_consume_the_offer_and_eof_does_not_finish_it() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    let mut finishes = Vec::new();
    assert_eq!(
        coordinator.decide(id(1), DropDecision::Accept, now, |actual| {
            finishes.push(actual);
            Ok(())
        }),
        Err(ReceiveError::InvalidPhase)
    );
    coordinator.complete(id(1), DATA, now).unwrap();
    assert!(finishes.is_empty());
    coordinator
        .decide(id(1), DropDecision::Accept, now, |actual| {
            finishes.push(actual);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        coordinator.decide(id(1), DropDecision::Accept, now, |actual| {
            finishes.push(actual);
            Ok(())
        }),
        Err(ReceiveError::StaleOffer)
    );
    coordinator.cancel(id(1)).unwrap();
    assert_eq!(finishes, vec![id(1)]);
    assert_terminal(&coordinator.poll(), 1, Ok(()));
}

#[test]
fn losing_copy_before_accept_cannot_issue_finish() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.complete(id(1), DATA, now).unwrap();
    coordinator.action(id(1), false).unwrap();
    let mut finishes = 0;
    assert_eq!(
        coordinator.decide(id(1), DropDecision::Accept, now, |_| {
            finishes += 1;
            Ok(())
        }),
        Err(ReceiveError::InvalidPhase)
    );
    assert_eq!(finishes, 0);
    coordinator.cancel(id(1)).unwrap();
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Cancelled));
}

#[test]
fn a_typed_finish_error_produces_one_error_terminal_without_success() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.complete(id(1), DATA, now).unwrap();
    let mut finishes = Vec::new();
    assert_eq!(
        coordinator.decide(id(1), DropDecision::Accept, now, |actual| {
            finishes.push(actual);
            Err(ReceiveError::Io)
        }),
        Err(ReceiveError::Io)
    );
    assert_eq!(finishes, vec![id(1)]);
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Io));
    assert!(coordinator.poll().is_empty());
}

#[test]
fn cancel_during_reading_suppresses_late_ready_and_keeps_one_terminal() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.cancel(id(1)).unwrap();
    coordinator.cancel(id(1)).unwrap();
    assert_eq!(
        coordinator.complete(id(1), DATA, now),
        Err(ReceiveError::StaleOffer)
    );
    let events = coordinator.poll();
    assert!(matches!(
        events.as_slice(),
        [
            ReceiveEvent::DropStarted { .. },
            ReceiveEvent::DropEnded { .. }
        ]
    ));
    assert_terminal(&events, 1, Err(ReceiveError::Cancelled));
}

#[test]
fn cancelling_queued_ready_retires_only_payload_and_waits_for_terminal_drain() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.complete(id(1), DATA, now).unwrap();
    coordinator.cancel(id(1)).unwrap();
    assert_eq!(
        coordinator.enter(id(2), point(2.0), 3, true),
        Err(ReceiveError::Busy)
    );
    let events = coordinator.poll();
    assert!(matches!(
        events.as_slice(),
        [
            ReceiveEvent::DropStarted { .. },
            ReceiveEvent::DropEnded { .. }
        ]
    ));
    assert_terminal(&events, 1, Err(ReceiveError::Cancelled));
    enter_copy(&mut coordinator, 2);
}

#[test]
fn predrop_leave_releases_hover_without_inventing_a_terminal() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.leave(id(1)).unwrap();
    assert!(
        matches!(coordinator.poll().as_slice(), [ReceiveEvent::Left { id: actual }] if *actual == id(1))
    );
    assert_eq!(coordinator.start(id(1), now), Err(ReceiveError::StaleOffer));
    enter_copy(&mut coordinator, 2);
}

#[test]
fn predrop_cancel_clears_hover_without_reserving_a_terminal() {
    let (mut coordinator, _) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.cancel(id(1)).unwrap();
    let events = coordinator.poll();
    assert!(matches!(events.as_slice(), [ReceiveEvent::Left { id: actual }] if *actual == id(1)));
    enter_copy(&mut coordinator, 2);
}

#[test]
fn reject_can_cancel_reading_before_ready_without_invoking_finish() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    let mut finishes = 0;
    coordinator
        .decide(id(1), DropDecision::Reject, now, |_| {
            finishes += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(finishes, 0);
    assert_eq!(
        coordinator.complete(id(1), DATA, now),
        Err(ReceiveError::StaleOffer)
    );
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Cancelled));
}

#[test]
fn transfer_expiration_is_exact_and_motion_does_not_extend_it() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.expire(now + Duration::from_millis(4)).unwrap();
    coordinator.motion(id(1), point(20.0)).unwrap();
    assert!(!coordinator
        .poll()
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropEnded { .. })));
    coordinator.expire(now + Duration::from_millis(5)).unwrap();
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Timeout));
}

#[test]
fn complete_checks_the_absolute_deadline_without_a_prior_expire_call() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    assert_eq!(
        coordinator.complete(id(1), DATA, now + Duration::from_millis(5)),
        Err(ReceiveError::Timeout)
    );
    let events = coordinator.poll();
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
    assert_terminal(&events, 1, Err(ReceiveError::Timeout));
}

#[test]
fn accept_checks_the_decision_deadline_without_a_prior_expire_call() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator
        .complete(id(1), DATA, now + Duration::from_millis(4))
        .unwrap();
    coordinator.action(id(1), true).unwrap();
    let mut finishes = 0;
    assert_eq!(
        coordinator.decide(
            id(1),
            DropDecision::Accept,
            now + Duration::from_millis(6),
            |_| {
                finishes += 1;
                Ok(())
            }
        ),
        Err(ReceiveError::Timeout)
    );
    assert_eq!(finishes, 0);
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Timeout));
}

#[test]
fn reject_and_close_never_finish_and_close_prevents_new_offers() {
    for close in [false, true] {
        let (mut coordinator, now) = setup();
        enter_copy(&mut coordinator, 1);
        coordinator.start(id(1), now).unwrap();
        coordinator.complete(id(1), DATA, now).unwrap();
        let mut finishes = 0;
        if close {
            coordinator.close().unwrap();
            coordinator.close().unwrap();
        } else {
            coordinator
                .decide(id(1), DropDecision::Reject, now, |_| {
                    finishes += 1;
                    Ok(())
                })
                .unwrap();
        }
        assert_eq!(finishes, 0);
        let events = coordinator.poll();
        assert_terminal(&events, 1, Err(ReceiveError::Cancelled));
        assert!(!events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
        if close {
            assert_eq!(
                coordinator.enter(id(2), point(2.0), 3, true),
                Err(ReceiveError::Closed)
            );
        }
    }
}

#[test]
fn a_complete_drop_reserves_its_terminal_before_any_poll() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.complete(id(1), DATA, now).unwrap();
    coordinator
        .decide(id(1), DropDecision::Accept, now, |_| Ok(()))
        .unwrap();
    assert_eq!(
        coordinator.enter(id(2), point(2.0), 3, true),
        Err(ReceiveError::Busy)
    );
    let events = coordinator.poll();
    assert!(matches!(
        events.as_slice(),
        [
            ReceiveEvent::DropStarted { .. },
            ReceiveEvent::DropReady { .. },
            ReceiveEvent::DropEnded { .. }
        ]
    ));
    assert_terminal(&events, 1, Ok(()));
    assert!(events.len() <= 5);
    enter_copy(&mut coordinator, 2);
}

#[test]
fn polling_started_and_ready_does_not_release_the_offer_before_ended() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    coordinator.complete(id(1), DATA, now).unwrap();
    let events = coordinator.poll();
    assert!(matches!(
        events.as_slice(),
        [
            ReceiveEvent::DropStarted { .. },
            ReceiveEvent::DropReady { .. }
        ]
    ));
    assert_eq!(
        coordinator.enter(id(2), point(2.0), 3, true),
        Err(ReceiveError::Busy)
    );
    coordinator.cancel(id(1)).unwrap();
    assert_terminal(&coordinator.poll(), 1, Err(ReceiveError::Cancelled));
    enter_copy(&mut coordinator, 2);
}

#[test]
fn an_invalid_complete_list_ends_the_drop_without_publishing_a_prefix() {
    let (mut coordinator, now) = setup();
    enter_copy(&mut coordinator, 1);
    coordinator.start(id(1), now).unwrap();
    assert_eq!(
        coordinator.complete(
            id(1),
            b"file:///tmp/valid\nhttps://example.invalid/not-local\n",
            now
        ),
        Err(ReceiveError::InvalidUriList)
    );
    let events = coordinator.poll();
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
    assert_terminal(&events, 1, Err(ReceiveError::InvalidUriList));
}
