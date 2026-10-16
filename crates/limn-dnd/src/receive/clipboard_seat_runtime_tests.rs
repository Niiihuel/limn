//! Two real seats must not select an arbitrary clipboard focus.
//! Requires only the separately proposed test constructor with a seat list.

use super::*;

#[test]
fn two_sealed_seats_are_ambiguous_and_losing_one_preserves_the_other_selection() {
    let observed = testing::Observations::default();
    let mut case = LiveOwnerCase::with_text_seats(
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        ReceiveLimits::default(),
        TextClipboardLimits::default(),
        &[5, 5],
    );
    drive_text(&mut case, |case| {
        case.lifecycle.status() == ReceiveStatus::Ready
            && case.fixture().has_keyboard(0)
            && case.fixture().has_keyboard(1)
    });
    assert_eq!(case.fixture().server.devices, 2);
    let first_offer = case.fixture().text_selection(0, 801, true, true);
    case.fixture_mut().keyboard_enter(0, 0);
    drive_text(&mut case, |case| {
        focused(case, None).is_some_and(|seat| seat.selection.is_some())
    });
    let first = focused(&case, None).unwrap();
    let second_offer = case.fixture().text_selection(1, 802, true, true);
    case.fixture_mut().keyboard_enter(1, 0);
    drive_text(&mut case, |case| {
        case.clipboard()
            .observed_seats()
            .into_iter()
            .flatten()
            .filter(|seat| {
                seat.focus.is_some_and(|focus| focus.registration.is_none())
                    && seat.selection.is_some()
            })
            .count()
            == 2
    });
    let second = case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.seat != first.seat && seat.selection.is_some())
        .unwrap();
    assert!(first.selection.unwrap().id != second.selection.unwrap().id);
    let (endpoint, _) = case.register(0, || {});
    drive_text(&mut case, |case| {
        case.clipboard()
            .observed_seats()
            .into_iter()
            .flatten()
            .filter(|seat| {
                seat.focus
                    .is_some_and(|focus| focus.registration == Some(endpoint.id()) && focus.sealed)
            })
            .count()
            == 2
    });
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(observed.clipboard_seals.load(Ordering::SeqCst) >= 2);
    assert!(case.lifecycle.incoming().is_none());
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::AmbiguousSeat)
    ));
    assert!(case.lifecycle.incoming().is_none());
    assert!(endpoint.control().incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert_eq!(case.fixture().text_receive_count(801, UTF8), 0);
    assert_eq!(case.fixture().text_receive_count(802, UTF8), 0);
    let survivor_before = case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.seat == second.seat)
        .unwrap();

    // The first real keyboard disappears without a Leave; the other seat's
    // unchanged native selection and resolved focus must remain usable.
    case.fixture_mut().keyboard_capability(0, false);
    drive_text(&mut case, |case| {
        case.clipboard()
            .observed_seats()
            .into_iter()
            .flatten()
            .find(|seat| seat.seat == first.seat)
            .is_some_and(|seat| seat.focus.is_none() && seat.selection.is_none())
            && case.fixture().server.released_keyboards == 1
            && case
                .fixture()
                .server
                .requests
                .iter()
                .any(|request| matches!(request, OfferRequest::Destroy(801)))
    });
    let survivor_after = case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.seat == second.seat)
        .unwrap();
    assert!(survivor_after.keyboard_generation == survivor_before.keyboard_generation);
    assert!(survivor_after.selection.unwrap().id == survivor_before.selection.unwrap().id);
    assert!(survivor_after.focus.unwrap().generation == survivor_before.focus.unwrap().generation);
    assert!(survivor_after.focus.unwrap().registration == Some(endpoint.id()));
    assert!(survivor_after.focus.unwrap().sealed);
    assert!(!first_offer.is_alive());
    assert!(second_offer.is_alive());
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    assert!(case.lifecycle.incoming().is_none());

    let request = reserve_text(&mut case, &endpoint);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    assert!(endpoint.control().incoming() == Some(IncomingLease::Text(request.id())));
    request.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        case.fixture().text_receive_count(802, UTF8) == 1
    });
    assert_eq!(case.fixture().text_receive_count(801, UTF8), 0);
    let fd = watched_text(&observed, &request);
    let writer = case.fixture_mut().take_writer(802);
    write_chunk(&writer, "segundo seat intacto á😀\r\n".as_bytes());
    drop(writer);
    let mut outcome = None;
    drive_text(&mut case, |case| {
        outcome = poll_text(case, &request);
        if matches!(outcome, Some(Ok(_))) {
            assert!(fd.upgrade().is_none());
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok("segundo seat intacto á😀\r\n".to_owned())));
    assert_eq!(poll_text(&case, &request), None);
    assert!(fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert_eq!(case.fixture().text_receive_count(801, UTF8), 0);
    assert_eq!(case.fixture().text_receive_count(802, UTF8), 1);
    assert_no_dnd_requests(&case, 801);
    assert_no_dnd_requests(&case, 802);
    assert_eq!(case.fixture().server.devices, 2);
    case.close();
    assert!(endpoint.is_detached());
    host_still_works(case.fixture_mut());
    assert_eq!(case.fixture().server.released_keyboards, 2);
    assert!(case.fixture().server.keyboards.is_empty());
}
