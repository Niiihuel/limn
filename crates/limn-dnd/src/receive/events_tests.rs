use super::super::{OfferId, ReceiveEpoch};
use super::ReceiveMailbox;
use crate::{ReceiveError, ReceiveEvent, SurfacePoint};
use std::path::PathBuf;

fn id(value: u64) -> OfferId {
    OfferId(value)
}

fn point(value: f64) -> SurfacePoint {
    SurfacePoint {
        x: value,
        y: value + 1.0,
    }
}

fn started(value: u64) -> ReceiveEvent {
    ReceiveEvent::DropStarted {
        id: id(value),
        epoch: ReceiveEpoch(1),
        point: point(10.0),
    }
}

#[test]
fn ready_debug_reports_count_without_exposing_paths_or_file_names() {
    let event = ReceiveEvent::DropReady {
        id: id(1),
        paths: vec![
            PathBuf::from("/private/secret-one"),
            PathBuf::from("/private/secret-two"),
        ],
    };
    let debug = format!("{event:?}");
    assert!(debug.contains("file_count: 2"), "{debug}");
    assert!(
        !debug.contains("private") && !debug.contains("secret"),
        "{debug}"
    );
}

#[test]
fn reliable_events_keep_started_ready_ended_order_and_payload_ownership() {
    let mut mailbox = ReceiveMailbox::new();
    mailbox.push(started(1)).unwrap();
    mailbox
        .push(ReceiveEvent::DropReady {
            id: id(1),
            paths: vec![PathBuf::from("/tmp/received")],
        })
        .unwrap();
    mailbox
        .push(ReceiveEvent::DropEnded {
            id: id(1),
            result: Ok(()),
        })
        .unwrap();
    let mut events = mailbox.poll().into_iter();
    assert!(
        matches!(events.next(), Some(ReceiveEvent::DropStarted { id: actual, .. }) if actual == id(1))
    );
    let Some(ReceiveEvent::DropReady { id: actual, paths }) = events.next() else {
        panic!("the complete payload follows Started");
    };
    assert_eq!(actual, id(1));
    assert_eq!(paths[0].as_os_str(), "/tmp/received");
    assert!(
        matches!(events.next(), Some(ReceiveEvent::DropEnded { id: actual, result: Ok(()) }) if actual == id(1))
    );
    assert!(events.next().is_none());
}

#[test]
fn a_hover_storm_retains_only_the_latest_pointer_position() {
    let mut mailbox = ReceiveMailbox::new();
    for position in 0..1000 {
        mailbox
            .push(ReceiveEvent::Hovered {
                id: id(1),
                point: point(position as f64),
            })
            .unwrap();
    }
    let events = mailbox.poll();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], ReceiveEvent::Hovered { id: actual, point: actual_point }
        if *actual == id(1) && *actual_point == point(999.0))
    );
}

#[test]
fn hover_leave_and_reentry_share_one_projection_slot() {
    let mut mailbox = ReceiveMailbox::new();
    mailbox
        .push(ReceiveEvent::Hovered {
            id: id(1),
            point: point(1.0),
        })
        .unwrap();
    mailbox.push(ReceiveEvent::Left { id: id(1) }).unwrap();
    mailbox
        .push(ReceiveEvent::Hovered {
            id: id(2),
            point: point(2.0),
        })
        .unwrap();
    let events = mailbox.poll();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], ReceiveEvent::Hovered { id: actual, .. } if *actual == id(2)));
}

#[test]
fn started_retires_its_queued_hover_instead_of_delivering_a_stale_position() {
    let mut mailbox = ReceiveMailbox::new();
    mailbox
        .push(ReceiveEvent::Hovered {
            id: id(1),
            point: point(1.0),
        })
        .unwrap();
    mailbox.push(started(1)).unwrap();
    let events = mailbox.poll();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], ReceiveEvent::DropStarted { id: actual, .. } if *actual == id(1)));
}

#[test]
fn ended_retires_its_queued_hover_without_losing_the_terminal() {
    let mut mailbox = ReceiveMailbox::new();
    mailbox
        .push(ReceiveEvent::Hovered {
            id: id(1),
            point: point(1.0),
        })
        .unwrap();
    mailbox
        .push(ReceiveEvent::DropEnded {
            id: id(1),
            result: Err(ReceiveError::Cancelled),
        })
        .unwrap();
    let events = mailbox.poll();
    assert_eq!(events.len(), 1);
    assert!(
        matches!(&events[0], ReceiveEvent::DropEnded { id: actual, result: Err(ReceiveError::Cancelled) }
        if *actual == id(1))
    );
}

#[test]
fn five_reliable_events_and_one_hover_are_the_complete_poll_budget() {
    let mut mailbox = ReceiveMailbox::new();
    for value in 1..=5 {
        mailbox.push(started(value)).unwrap();
    }
    mailbox
        .push(ReceiveEvent::Hovered {
            id: id(99),
            point: point(99.0),
        })
        .unwrap();
    let events = mailbox.poll();
    assert_eq!(events.len(), 6);
    for (index, event) in events[..5].iter().enumerate() {
        assert!(
            matches!(event, ReceiveEvent::DropStarted { id: actual, .. } if *actual == id(index as u64 + 1))
        );
    }
    assert!(matches!(&events[5], ReceiveEvent::Hovered { id: actual, .. } if *actual == id(99)));
}

#[test]
fn a_full_reliable_mailbox_returns_ended_intact_for_explicit_retry() {
    let mut mailbox = ReceiveMailbox::new();
    for value in 1..=5 {
        mailbox.push(started(value)).unwrap();
    }
    let terminal = mailbox
        .push(ReceiveEvent::DropEnded {
            id: id(1),
            result: Err(ReceiveError::Cancelled),
        })
        .expect_err("the caller must keep a terminal rejected by the full mailbox");
    assert!(
        matches!(&terminal, ReceiveEvent::DropEnded { id: actual, result: Err(ReceiveError::Cancelled) }
        if *actual == id(1))
    );
    assert_eq!(mailbox.poll().len(), 5);
    mailbox.push(terminal).unwrap();
    let events = mailbox.poll();
    assert!(
        matches!(events.as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Err(ReceiveError::Cancelled) }]
        if *actual == id(1))
    );
}

#[test]
fn reliable_saturation_returns_the_whole_ready_payload_without_silent_loss() {
    let mut mailbox = ReceiveMailbox::new();
    for value in 1..=5 {
        mailbox.push(started(value)).unwrap();
    }
    let rejected = mailbox
        .push(ReceiveEvent::DropReady {
            id: id(1),
            paths: vec![PathBuf::from("/tmp/not-lost")],
        })
        .expect_err("a full mailbox cannot silently dispose of the payload");
    let ReceiveEvent::DropReady { id: actual, paths } = rejected else {
        panic!("the caller must receive the original event");
    };
    assert_eq!(actual, id(1));
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].as_os_str(), "/tmp/not-lost");
    assert_eq!(mailbox.poll().len(), 5);
}

#[test]
fn polling_consumes_the_current_batch_without_replaying_events() {
    let mut mailbox = ReceiveMailbox::new();
    mailbox.push(ReceiveEvent::Left { id: id(1) }).unwrap();
    assert_eq!(mailbox.poll().len(), 1);
    assert!(mailbox.poll().is_empty());
}
