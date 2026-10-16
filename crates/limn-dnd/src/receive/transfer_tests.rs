use super::*;
use crate::DropDecision;
use std::os::unix::ffi::OsStrExt;

const DATA: &[u8] = b"file:///tmp/received\r\n";

impl Fixture {
    pub(super) fn take_writer(&mut self, key: usize) -> OwnedFd {
        let index = self
            .server
            .requests
            .iter()
            .position(
                |request| matches!(request, OfferRequest::Receive(actual, _, _) if *actual == key),
            )
            .expect("server received the exact transfer descriptor");
        match self.server.requests.remove(index) {
            OfferRequest::Receive(actual, mime, fd) => {
                self.server
                    .requests
                    .insert(index, OfferRequest::ReceiveTaken(actual, mime));
                fd
            }
            _ => unreachable!("the selected request still owns its writer"),
        }
    }

    pub(super) fn finish_count(&self, key: usize) -> usize {
        self.server
            .requests
            .iter()
            .filter(|request| matches!(request, OfferRequest::Finish(actual) if *actual == key))
            .count()
    }
}

fn configured_fixture(limits: ReceiveLimits) -> (Fixture, GuestStartup, ReceiveControl, Instant) {
    let mut fixture = Fixture::new(Some(3), &[5, 5]);
    fixture.create_surfaces();
    let control = control();
    let now = Instant::now();
    // The fixture retains the display and both surfaces until guest cleanup.
    let backend = unsafe { Backend::from_foreign_display(fixture.host.backend().display_ptr()) };
    let mut guest = GuestStartup::for_surface(
        Connection::from_backend(backend),
        fixture.host_surfaces[0].id(),
        control.clone(),
        limits,
        now,
        || {},
    )
    .unwrap();
    fixture.until_ready(&mut guest, &control, now);
    (fixture, guest, control, now)
}

fn start(
    fixture: &mut Fixture,
    guest: &mut GuestStartup,
    control: &ReceiveControl,
    now: Instant,
) -> (wl_data_offer::WlDataOffer, OfferId, OwnedFd) {
    let (offer, id) = copy_enter(fixture, guest, control, 0, 1, now);
    fixture.server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    fixture.deliver(guest, now).unwrap();
    assert!(
        matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id)
    );
    fixture.serve();
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

fn ready_events(
    guest: &mut GuestStartup,
    control: &ReceiveControl,
    now: Instant,
) -> Vec<ReceiveEvent> {
    let mut events = Vec::new();
    for _ in 0..8 {
        guest.pump(now).unwrap();
        events.extend(control.poll());
        if events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
        {
            return events;
        }
    }
    panic!("a complete private URI transfer never became Ready: {events:?}");
}

fn ended_events(
    guest: &mut GuestStartup,
    control: &ReceiveControl,
    now: Instant,
) -> Vec<ReceiveEvent> {
    let mut events = Vec::new();
    for _ in 0..8 {
        guest.pump(now).unwrap();
        events.extend(control.poll());
        if events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
        {
            return events;
        }
    }
    panic!("the exact private transfer never ended: {events:?}");
}

fn assert_ready(events: &[ReceiveEvent], id: OfferId) {
    assert!(
        matches!(events, [ReceiveEvent::DropReady { id: actual, paths }] if *actual == id && paths.len() == 1 && paths[0].as_os_str().as_bytes() == b"/tmp/received")
    );
}

fn assert_ended(events: &[ReceiveEvent], id: OfferId, expected: Result<(), ReceiveError>) {
    assert!(
        matches!(events, [ReceiveEvent::DropEnded { id: actual, result }] if *actual == id && *result == expected)
    );
}

#[test]
fn a_valid_prefix_waits_for_eof_and_ready_alone_never_finishes_copy() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    for _ in 0..3 {
        guest.pump(now).unwrap();
        assert!(control.poll().is_empty());
    }
    assert!(matches!(
        guest.state.coordinator.offer_phase(id),
        Some(Phase::Reading { .. })
    ));
    fixture.serve();
    assert_eq!(fixture.received(1), 1);
    assert!(!fixture.finished());
    drop(writer);
    assert_ready(&ready_events(&mut guest, &control, now), id);
    fixture.serve();
    assert!(!fixture.finished());
    assert!(control.poll().is_empty());
}

#[test]
fn accepting_a_finishes_only_a_once_while_b_is_rejected() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, a, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    drop(writer);
    assert_ready(&ready_events(&mut guest, &control, now), a);
    let b = fixture.offer(1, 2, "text/uri-list");
    fixture.enter(1, &b, 0, 100.0, 200.0);
    b.action(wl_data_device_manager::DndAction::Move);
    fixture.deliver(&mut guest, now).unwrap();
    assert!(control.poll().is_empty());
    control.decide(a, DropDecision::Accept).unwrap();
    assert_ended(&ended_events(&mut guest, &control, now), a, Ok(()));
    fixture.serve();
    assert_eq!(fixture.finish_count(1), 1);
    assert_eq!(fixture.finish_count(2), 0);
    assert_eq!(fixture.received(1), 1);
    assert_eq!(fixture.received(2), 0);
    assert_eq!(
        control.decide(a, DropDecision::Accept),
        Err(ReceiveError::StaleOffer)
    );
    guest.pump(now).unwrap();
    fixture.serve();
    assert_eq!(fixture.finish_count(1), 1);
    assert!(control.poll().is_empty());
}

#[test]
fn rejection_or_cancel_with_pending_bytes_has_one_cancelled_terminal() {
    for reject in [false, true] {
        let (mut fixture, mut guest, control, now) = ready_fixture();
        let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
        write_chunk(&writer, &DATA[..8]);
        guest.pump(now).unwrap();
        assert!(control.poll().is_empty());
        if reject {
            control.decide(id, DropDecision::Reject).unwrap();
        } else {
            control.cancel(id).unwrap();
        }
        assert_ended(
            &ended_events(&mut guest, &control, now),
            id,
            Err(ReceiveError::Cancelled),
        );
        drop(writer);
        for _ in 0..3 {
            guest.pump(now).unwrap();
            assert!(control.poll().is_empty());
        }
        fixture.serve();
        assert!(!fixture.finished());
    }
}

#[test]
fn limit_plus_one_and_the_original_deadline_reject_complete_prefixes() {
    let limits = ReceiveLimits {
        max_list_bytes: DATA.len(),
        ..ReceiveLimits::default()
    };
    let (mut fixture, mut guest, control, now) = configured_fixture(limits);
    let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    write_chunk(&writer, b"x");
    assert_ended(
        &ended_events(&mut guest, &control, now),
        id,
        Err(ReceiveError::TooLarge),
    );
    drop(writer);
    fixture.serve();
    assert!(!fixture.finished());

    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
    let deadline = control.header().offer.unwrap().deadline.unwrap();
    write_chunk(&writer, &DATA[..8]);
    guest.pump(deadline - Duration::from_millis(1)).unwrap();
    assert!(control.poll().is_empty());
    write_chunk(&writer, &DATA[8..]);
    drop(writer);
    assert_ended(
        &ended_events(&mut guest, &control, deadline),
        id,
        Err(ReceiveError::Timeout),
    );
    fixture.serve();
    assert!(!fixture.finished());
}

#[test]
fn revoking_copy_after_started_cancels_that_drop_before_ready_or_finish() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (offer, id, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    offer.action(wl_data_device_manager::DndAction::Move);
    fixture.deliver(&mut guest, now).unwrap();
    assert_ended(&control.poll(), id, Err(ReceiveError::Cancelled));
    drop(writer);
    for _ in 0..3 {
        guest.pump(now).unwrap();
        assert!(control.poll().is_empty());
    }
    fixture.serve();
    assert!(!fixture.finished());
    let (_, b) = copy_enter(&mut fixture, &mut guest, &control, 1, 2, now);
    assert_ne!(id, b);
}

#[test]
fn an_invalid_exact_proxy_after_ready_has_one_io_terminal_without_finish() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    drop(writer);
    assert_ready(&ready_events(&mut guest, &control, now), id);
    guest
        .state
        .offer_proxy(id)
        .unwrap()
        .send_request(client::protocol::wl_data_offer::Request::Destroy {})
        .unwrap();
    control.decide(id, DropDecision::Accept).unwrap();
    assert_ended(
        &ended_events(&mut guest, &control, now),
        id,
        Err(ReceiveError::Io),
    );
    fixture.serve();
    assert!(!fixture.finished());
    guest.pump(now).unwrap();
    assert!(control.poll().is_empty());
}

#[test]
fn a_mixed_invalid_uri_list_never_publishes_its_valid_first_path() {
    let (mut fixture, mut guest, control, now) = ready_fixture();
    let (_, id, writer) = start(&mut fixture, &mut guest, &control, now);
    write_chunk(&writer, DATA);
    write_chunk(&writer, b"https://example.invalid/file\r\n");
    drop(writer);
    assert_ended(
        &ended_events(&mut guest, &control, now),
        id,
        Err(ReceiveError::InvalidUriList),
    );
    fixture.serve();
    assert!(!fixture.finished());
    guest.pump(now).unwrap();
    assert!(control.poll().is_empty());
}
