use super::ReceivePipe;
use crate::{ReceiveError, ReceiveEvent, ReceiveLimits, SurfacePoint};
use std::{
    os::fd::OwnedFd,
    time::{Duration, Instant},
};

use super::super::{control::ReceiveControl, coordinator::Coordinator, OfferId, ReceiveEpoch};

const DATA: &[u8] = b"file:///tmp/received\r\n";
const PHASE: Duration = Duration::from_secs(5);

fn id(value: u64) -> OfferId {
    OfferId(value)
}

fn write_all(writer: &OwnedFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let count = rustix::io::write(writer, bytes).unwrap();
        assert!(count > 0);
        bytes = &bytes[count..];
    }
}

fn fixture() -> (Coordinator, ReceiveControl, Instant) {
    let limits = ReceiveLimits {
        transfer_timeout: PHASE,
        decision_timeout: PHASE,
        ..ReceiveLimits::default()
    };
    let control = ReceiveControl::new(ReceiveEpoch(17)).unwrap();
    let mut coordinator = Coordinator::with_control(limits, control.clone(), || {}).unwrap();
    let now = Instant::now();
    let sync = coordinator.next_sync(now).unwrap().unwrap();
    coordinator.sync_done(sync.epoch, now).unwrap();
    coordinator
        .enter(id(1), SurfacePoint { x: 10.0, y: 20.0 }, 3, true)
        .unwrap();
    coordinator.action(id(1), true).unwrap();
    coordinator.start(id(1), now).unwrap();
    (coordinator, control, now)
}

fn assert_failed_once(events: &[ReceiveEvent], expected: ReceiveError) {
    assert!(
        matches!(events.first(), Some(ReceiveEvent::DropStarted { id: actual, .. }) if *actual == id(1))
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, ReceiveEvent::DropReady { .. })));
    let terminals: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ReceiveEvent::DropEnded { id, result } => Some((*id, *result)),
            _ => None,
        })
        .collect();
    assert_eq!(terminals, vec![(id(1), Err(expected))]);
}

#[test]
fn only_the_reader_is_nonblocking_and_both_descriptors_are_close_on_exec() {
    use rustix::{
        fs::{fcntl_getfl, OFlags},
        io::{fcntl_getfd, FdFlags},
    };
    let (_, deadline) = (Instant::now(), Instant::now() + PHASE);
    let (pipe, writer) = ReceivePipe::new(id(1), DATA.len(), deadline).unwrap();
    assert!(fcntl_getfl(&pipe.reader)
        .unwrap()
        .contains(OFlags::NONBLOCK));
    assert!(!fcntl_getfl(&writer).unwrap().contains(OFlags::NONBLOCK));
    assert!(fcntl_getfd(&pipe.reader)
        .unwrap()
        .contains(FdFlags::CLOEXEC));
    assert!(fcntl_getfd(&writer).unwrap().contains(FdFlags::CLOEXEC));
}

#[test]
fn a_valid_prefix_is_not_complete_until_the_writer_reaches_eof() {
    let now = Instant::now();
    let (mut pipe, writer) = ReceivePipe::new(id(1), 64, now + PHASE).unwrap();
    write_all(&writer, DATA);
    assert_eq!(pipe.read_available(now), Ok(None));
    assert_eq!(pipe.read_available(now), Ok(None));
    drop(writer);
    assert_eq!(pipe.read_available(now), Ok(Some(DATA.to_vec())));
    assert_eq!(pipe.read_available(now), Err(ReceiveError::InvalidPhase));
}

#[test]
fn an_empty_eof_remains_an_invalid_uri_list_instead_of_a_fabricated_path() {
    let now = Instant::now();
    let (mut pipe, writer) = ReceivePipe::new(id(1), 64, now + PHASE).unwrap();
    drop(writer);
    let bytes = pipe.read_available(now).unwrap().unwrap();
    assert!(bytes.is_empty());
    assert_eq!(
        crate::uri_list::decode_received(&bytes, &ReceiveLimits::default()),
        Err(ReceiveError::InvalidUriList)
    );
}

#[test]
fn an_exact_limit_waits_for_eof_and_then_preserves_every_byte() {
    let now = Instant::now();
    let (mut pipe, writer) = ReceivePipe::new(id(1), DATA.len(), now + PHASE).unwrap();
    write_all(&writer, DATA);
    assert_eq!(pipe.read_available(now), Ok(None));
    drop(writer);
    assert_eq!(pipe.read_available(now), Ok(Some(DATA.to_vec())));
}

#[test]
fn limit_plus_one_fails_without_waiting_for_eof_or_returning_a_prefix() {
    let now = Instant::now();
    let (mut pipe, writer) = ReceivePipe::new(id(1), DATA.len(), now + PHASE).unwrap();
    write_all(&writer, DATA);
    write_all(&writer, b"x");
    assert_eq!(pipe.read_available(now), Err(ReceiveError::TooLarge));
    assert_eq!(pipe.read_available(now), Err(ReceiveError::InvalidPhase));
}

#[test]
fn again_is_pending_and_later_chunks_keep_the_original_order() {
    let now = Instant::now();
    let (mut pipe, writer) = ReceivePipe::new(id(1), 64, now + PHASE).unwrap();
    assert_eq!(pipe.read_available(now), Ok(None));
    write_all(&writer, &DATA[..8]);
    assert_eq!(pipe.read_available(now), Ok(None));
    assert_eq!(pipe.read_available(now), Ok(None));
    write_all(&writer, &DATA[8..]);
    assert_eq!(pipe.read_available(now), Ok(None));
    drop(writer);
    assert_eq!(pipe.read_available(now), Ok(Some(DATA.to_vec())));
}

#[test]
fn partial_data_never_renews_the_absolute_transfer_deadline() {
    let now = Instant::now();
    let deadline = now + PHASE;
    let (mut pipe, writer) = ReceivePipe::new(id(1), 64, deadline).unwrap();
    write_all(&writer, &DATA[..8]);
    assert_eq!(
        pipe.read_available(deadline - Duration::from_millis(1)),
        Ok(None)
    );
    write_all(&writer, &DATA[8..]);
    drop(writer);
    assert_eq!(pipe.read_available(deadline), Err(ReceiveError::Timeout));
    assert_eq!(
        pipe.read_available(deadline),
        Err(ReceiveError::InvalidPhase)
    );
}

#[test]
fn unsupported_pipe_limits_fail_without_creating_a_transfer() {
    let deadline = Instant::now() + PHASE;
    for limit in [0, 1024 * 1024 + 1, usize::MAX] {
        assert!(matches!(
            ReceivePipe::new(id(1), limit, deadline),
            Err(ReceiveError::InvalidLimits)
        ));
    }
}

#[test]
fn the_pipe_uses_the_same_reading_deadline_as_started_without_renewal() {
    let (mut coordinator, control, now) = fixture();
    let deadline = coordinator.reading_deadline(id(1), now).unwrap();
    assert_eq!(deadline, now + PHASE);
    assert_eq!(
        coordinator.reading_deadline(id(2), now),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(
        coordinator.reading_deadline(id(1), now + Duration::from_secs(1)),
        Ok(deadline)
    );
    assert_eq!(
        coordinator.reading_deadline(id(1), deadline),
        Err(ReceiveError::Timeout)
    );
    assert_failed_once(&control.poll(), ReceiveError::Timeout);
    assert!(control.poll().is_empty());
}

#[test]
fn an_exact_pipe_failure_preserves_its_error_and_does_not_close_the_receiver() {
    for error in [
        ReceiveError::Io,
        ReceiveError::TooLarge,
        ReceiveError::Timeout,
    ] {
        let (mut coordinator, control, now) = fixture();
        assert_eq!(
            coordinator.fail(id(2), error),
            Err(ReceiveError::StaleOffer)
        );
        coordinator.fail(id(1), error).unwrap();
        assert_eq!(
            coordinator.fail(id(1), error),
            Err(ReceiveError::StaleOffer)
        );
        assert_eq!(
            coordinator.complete(id(1), DATA, now),
            Err(ReceiveError::StaleOffer)
        );
        assert_failed_once(&control.poll(), error);
        assert!(control.poll().is_empty());
        coordinator.reconcile(now).unwrap();
        assert!(coordinator
            .enter(id(2), SurfacePoint { x: 1.0, y: 2.0 }, 3, true)
            .is_ok());
    }
}

#[test]
fn bytes_that_finish_after_cancel_cannot_publish_a_ready_or_second_terminal() {
    let (mut coordinator, control, now) = fixture();
    let deadline = coordinator.reading_deadline(id(1), now).unwrap();
    let (mut pipe, writer) = ReceivePipe::new(id(1), 64, deadline).unwrap();
    write_all(&writer, &DATA[..8]);
    assert_eq!(pipe.read_available(now), Ok(None));
    control.cancel(id(1)).unwrap();
    coordinator.reconcile(now).unwrap();
    assert_failed_once(&control.poll(), ReceiveError::Cancelled);
    write_all(&writer, &DATA[8..]);
    assert_eq!(pipe.read_available(now), Ok(None));
    drop(writer);
    let bytes = pipe.read_available(now).unwrap().unwrap();
    let paths = crate::uri_list::decode_received(&bytes, &ReceiveLimits::default()).unwrap();
    assert_eq!(
        coordinator.complete_validated(id(1), paths, now),
        Err(ReceiveError::StaleOffer)
    );
    assert_eq!(
        coordinator.fail(id(1), ReceiveError::Io),
        Err(ReceiveError::StaleOffer)
    );
    assert!(control.poll().is_empty());
}

#[test]
fn an_os_read_error_ends_the_exact_drop_once_without_a_ready() {
    let (mut coordinator, control, now) = fixture();
    let (mut pipe, _writer) = ReceivePipe::new(id(1), 64, now + PHASE).unwrap();
    let (_other_reader, other_writer) =
        rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC).unwrap();
    // A valid write-only descriptor fails read without unsafe code or a signal.
    pipe.reader = other_writer.into();
    let error = pipe.read_available(now).unwrap_err();
    assert_eq!(error, ReceiveError::Io);
    coordinator.fail(id(1), error).unwrap();
    assert_eq!(pipe.read_available(now), Err(ReceiveError::InvalidPhase));
    assert_failed_once(&control.poll(), ReceiveError::Io);
    assert!(control.poll().is_empty());
}

#[test]
fn interleaved_chunks_larger_than_the_read_buffer_preserve_order_until_eof() {
    let now = Instant::now();
    let bytes: Vec<u8> = (0..(3 * 4096 + 123))
        .map(|index| (index % 251) as u8)
        .collect();
    let (mut pipe, writer) = ReceivePipe::new(id(1), bytes.len(), now + PHASE).unwrap();
    for chunk in bytes.chunks(1024) {
        write_all(&writer, chunk);
        // Each write fits the pipe and is drained before the following write.
        assert_eq!(pipe.read_available(now), Ok(None));
    }
    assert_eq!(pipe.read_available(now), Ok(None));
    drop(writer);
    assert_eq!(pipe.read_available(now), Ok(Some(bytes)));
}
