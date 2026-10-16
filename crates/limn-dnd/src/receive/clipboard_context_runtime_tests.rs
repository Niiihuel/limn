//! Native context cuts through actual Keyboard/Selection events and FD readers.
//! Wire only as a child of clipboard_runtime_tests; no new fixture seam.

use super::*;

fn selection_receives(case: &LiveOwnerCase, key: usize) -> Vec<&str> {
    // The existing one-MIME counter stays strict. Here a single real offer is
    // deliberately received once as Plain and then as UTF8, in that order.
    case.fixture()
        .server
        .requests
        .iter()
        .filter_map(|request| match request {
            OfferRequest::Receive(actual, mime, _) | OfferRequest::ReceiveTaken(actual, mime)
                if *actual == key =>
            {
                Some(mime.as_str())
            }
            _ => None,
        })
        .collect()
}

fn selection_was_destroyed(case: &LiveOwnerCase, key: usize) -> bool {
    case.fixture()
        .server
        .requests
        .iter()
        .any(|request| matches!(request, OfferRequest::Destroy(actual) if *actual == key))
}

fn assert_context_incoming(
    case: &LiveOwnerCase,
    endpoint: &SurfaceLease,
    request: &TextRequestControl,
) {
    let exact = IncomingLease::Text(request.id());
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(endpoint.control().incoming() == Some(exact));
}

fn finish_context_text(
    case: &mut LiveOwnerCase,
    request: &TextRequestControl,
    fd: &Weak<OwnedFd>,
    writer: OwnedFd,
    raw: &str,
) {
    write_chunk(&writer, raw.as_bytes());
    drop(writer);
    let mut outcome = None;
    drive_text(case, |case| {
        outcome = poll_text(case, request);
        if matches!(outcome, Some(Ok(_))) {
            assert!(
                fd.upgrade().is_none(),
                "success requires the exact Generic FD gone"
            );
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok(raw.to_owned())));
    assert_eq!(poll_text(case, request), None);
    assert!(fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
}

fn await_context_cancellation(
    case: &mut LiveOwnerCase,
    request: &TextRequestControl,
    fd: Option<&Weak<OwnedFd>>,
) {
    let mut outcome = None;
    drive_text(case, |case| {
        outcome = poll_text(case, request);
        let incoming = case.lifecycle.incoming();
        if incoming.is_none() {
            if let Some(fd) = fd {
                assert!(
                    fd.upgrade().is_none(),
                    "Incoming free requires the exact FD gone"
                );
            }
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Err(ClipboardError::Cancelled)));
    drive_text(case, |case| {
        assert_eq!(poll_text(case, request), None);
        let incoming = case.lifecycle.incoming();
        if incoming.is_none() {
            if let Some(fd) = fd {
                assert!(
                    fd.upgrade().is_none(),
                    "Incoming free requires the exact FD gone"
                );
            }
        }
        incoming.is_none() && fd.is_none_or(|fd| fd.upgrade().is_none())
    });
    assert_eq!(poll_text(case, request), None);
}

#[test]
fn keyboard_loss_without_leave_retires_the_open_reader_and_requires_a_new_selection() {
    // Published-but-unpolled is left to the TTL slice. This case observes an
    // actual Reading pipe, source writer open, and the existing retirement gate.
    let observed = testing::Observations::default();
    let (entered, retirement_entered) = mpsc::sync_channel(1);
    let (release, retirement_release) = mpsc::sync_channel(1);
    let mut case = LiveOwnerCase::with_text(
        testing::Hooks {
            observed: observed.clone(),
            before_retire_sync: Some((entered, retirement_release)),
            ..Default::default()
        },
        ReceiveLimits::default(),
        TextClipboardLimits::default(),
    );
    let mut release = OpenOnDrop(Some(release));
    let (endpoint, old, source_open, old_fd) = first_request(&mut case, 601);
    drain_prefix(&mut case, &source_open, &old_fd);
    let before_loss = focused(&case, Some(endpoint.id())).unwrap();
    let original_selection = before_loss.selection.unwrap().id;
    let original_focus = before_loss.focus.unwrap().generation;
    assert_context_incoming(&case, &endpoint, &old);
    assert!(old_fd.upgrade().is_some());
    assert_eq!(poll_text(&case, &old), None);

    // There is deliberately no keyboard_leave call and no source EOF.
    case.fixture_mut().keyboard_capability(0, false);
    drive_text(&mut case, |_| retirement_entered.try_recv().is_ok());
    let lost = case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.seat == before_loss.seat)
        .unwrap();
    assert!(lost.keyboard_generation != before_loss.keyboard_generation);
    assert!(lost.focus.is_none());
    assert!(lost.selection.is_none());
    assert!(old_fd.upgrade().is_some());
    assert_context_incoming(&case, &endpoint, &old);
    assert_eq!(poll_text(&case, &old), Some(Err(ClipboardError::Cancelled)));
    assert_eq!(poll_text(&case, &old), None);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    release.release();
    drive_text(&mut case, |case| {
        assert_eq!(poll_text(case, &old), None);
        old_fd.upgrade().is_none()
            && case.lifecycle.incoming().is_none()
            && selection_was_destroyed(case, 601)
            && case.fixture().server.released_keyboards == 1
    });
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    assert!(!case.fixture().has_keyboard(0));
    assert_eq!(selection_receives(&case, 601), vec![UTF8]);
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::Unavailable)
    ));

    case.fixture_mut().keyboard_capability(0, true);
    drive_text(&mut case, |case| case.fixture().has_keyboard(0));
    case.fixture_mut().keyboard_enter(0, 0);
    drive_text(&mut case, |case| {
        focused(case, Some(endpoint.id())).is_some_and(|seat| seat.focus.unwrap().sealed)
    });
    let recovered = focused(&case, Some(endpoint.id())).unwrap();
    assert!(recovered.seat == before_loss.seat);
    assert!(recovered.keyboard_generation != before_loss.keyboard_generation);
    assert!(recovered.keyboard_generation != lost.keyboard_generation);
    assert!(recovered.focus.unwrap().generation != original_focus);
    assert!(
        recovered.selection.is_none(),
        "a new Enter cannot revive the destroyed Selection"
    );
    assert!(case.lifecycle.incoming().is_none());
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::NoSelection)
    ));
    assert_eq!(selection_receives(&case, 601), vec![UTF8]);

    case.fixture().text_selection(0, 602, true, true);
    drive_text(&mut case, |case| {
        focused(case, Some(endpoint.id())).is_some_and(|seat| {
            seat.focus.unwrap().sealed
                && seat
                    .selection
                    .is_some_and(|selection| selection.id != original_selection)
        })
    });
    let fresh = reserve_text(&mut case, &endpoint);
    assert!(fresh.id() != old.id());
    old.cancel();
    assert_eq!(poll_text(&case, &old), None);
    assert_context_incoming(&case, &endpoint, &fresh);
    assert!(selection_receives(&case, 602).is_empty());
    fresh.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        selection_receives(case, 602) == vec![UTF8]
    });
    let fresh_fd = watched_text(&observed, &fresh);
    assert!(!Weak::ptr_eq(&old_fd, &fresh_fd));
    assert!(old_fd.upgrade().is_none());
    let writer = case.fixture_mut().take_writer(602);
    finish_context_text(
        &mut case,
        &fresh,
        &fresh_fd,
        writer,
        "teclado recuperado á😀\r\n",
    );
    // The cancelled source's writer stayed open through the entire new transfer.
    drop(source_open);
    assert_eq!(poll_text(&case, &old), None);
    assert_eq!(selection_receives(&case, 601), vec![UTF8]);
    assert_eq!(selection_receives(&case, 602), vec![UTF8]);
    assert_no_dnd_requests(&case, 601);
    assert_no_dnd_requests(&case, 602);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    case.close();
    assert!(endpoint.is_detached());
    host_still_works(case.fixture_mut());
    assert_eq!(case.fixture().server.released_keyboards, 2);
    assert!(case.fixture().server.keyboards.is_empty());
}

#[test]
fn late_mime_is_frozen_at_reserve_and_selection_replace_or_null_cancels_only_its_request() {
    let observed = testing::Observations::default();
    let mut case = LiveOwnerCase::with_text(
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        ReceiveLimits::default(),
        TextClipboardLimits::default(),
    );
    drive_text(&mut case, |case| {
        case.lifecycle.status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
    });
    let same_offer = case.fixture().text_selection(0, 701, false, true);
    case.fixture_mut().keyboard_enter(0, 0);
    drive_text(&mut case, |case| {
        focused(case, None).is_some_and(|seat| {
            seat.selection
                .is_some_and(|selection| selection.plain && !selection.utf8)
        })
    });
    let original_selection = focused(&case, None).unwrap().selection.unwrap().id;
    let (endpoint, _) = case.register(0, || {});
    let plain = reserve_text(&mut case, &endpoint);
    let reserved_focus = focused(&case, Some(endpoint.id()))
        .unwrap()
        .focus
        .unwrap()
        .generation;
    assert_context_incoming(&case, &endpoint, &plain);
    assert!(selection_receives(&case, 701).is_empty());

    // Adversarial late MIME on the same native offer: the core protocol normally
    // advertises MIME just after DataOffer, before Selection. The v4 robustness
    // policy also freezes the admitted MIME if an event arrives after Reserve.
    same_offer.offer(UTF8.to_owned());
    drive_text(&mut case, |case| {
        focused(case, Some(endpoint.id())).is_some_and(|seat| {
            seat.selection.is_some_and(|selection| {
                selection.id == original_selection && selection.utf8 && selection.plain
            })
        })
    });
    let after_mime = focused(&case, Some(endpoint.id())).unwrap();
    assert!(after_mime.focus.unwrap().generation == reserved_focus);
    assert!(after_mime.focus.unwrap().sealed);
    assert_eq!(poll_text(&case, &plain), None);
    assert_context_incoming(&case, &endpoint, &plain);
    assert!(selection_receives(&case, 701).is_empty());
    plain.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        selection_receives(case, 701) == vec!["text/plain"]
    });
    let plain_fd = watched_text(&observed, &plain);
    let writer = case.fixture_mut().take_writer(701);
    finish_context_text(
        &mut case,
        &plain,
        &plain_fd,
        writer,
        "Plain reservado á😀\r\n",
    );
    assert!(!selection_was_destroyed(&case, 701));

    // The next request sees the accumulated UTF8 preference on the same offer.
    let utf8 = reserve_text(&mut case, &endpoint);
    assert!(utf8.id() != plain.id());
    assert!(
        focused(&case, Some(endpoint.id()))
            .unwrap()
            .selection
            .unwrap()
            .id
            == original_selection
    );
    plain.cancel();
    assert_eq!(poll_text(&case, &plain), None);
    assert_context_incoming(&case, &endpoint, &utf8);
    utf8.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        selection_receives(case, 701) == vec!["text/plain", UTF8]
    });
    let utf8_fd = watched_text(&observed, &utf8);
    assert!(!Weak::ptr_eq(&plain_fd, &utf8_fd));
    let source_open = case.fixture_mut().take_writer(701);
    drain_prefix(&mut case, &source_open, &utf8_fd);
    assert!(utf8_fd.upgrade().is_some());
    assert_context_incoming(&case, &endpoint, &utf8);
    assert_eq!(poll_text(&case, &utf8), None);

    // Replace the selection while its exact reader and source writer are open.
    case.fixture().text_selection(0, 702, true, true);
    await_context_cancellation(&mut case, &utf8, Some(&utf8_fd));
    drive_text(&mut case, |case| {
        selection_was_destroyed(case, 701)
            && focused(case, Some(endpoint.id())).is_some_and(|seat| {
                seat.focus.unwrap().sealed
                    && seat
                        .selection
                        .is_some_and(|selection| selection.id != original_selection)
            })
    });
    let replacement_selection = focused(&case, Some(endpoint.id()))
        .unwrap()
        .selection
        .unwrap()
        .id;
    assert_eq!(selection_receives(&case, 701), vec!["text/plain", UTF8]);
    assert!(selection_receives(&case, 702).is_empty());
    assert!(!selection_was_destroyed(&case, 702));

    // NULL cuts a Reserved request before ACK: it must never issue Receive.
    let null_cut = reserve_text(&mut case, &endpoint);
    assert!(null_cut.id() != utf8.id() && null_cut.id() != plain.id());
    utf8.cancel();
    assert_eq!(poll_text(&case, &utf8), None);
    assert_context_incoming(&case, &endpoint, &null_cut);
    case.fixture().selection_none(0);
    await_context_cancellation(&mut case, &null_cut, None);
    drive_text(&mut case, |case| {
        selection_was_destroyed(case, 702)
            && focused(case, Some(endpoint.id()))
                .is_some_and(|seat| seat.focus.unwrap().sealed && seat.selection.is_none())
    });
    assert!(selection_receives(&case, 702).is_empty());
    assert!(utf8_fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::NoSelection)
    ));

    case.fixture().text_selection(0, 703, true, true);
    drive_text(&mut case, |case| {
        focused(case, Some(endpoint.id())).is_some_and(|seat| {
            seat.focus.unwrap().sealed
                && seat.selection.is_some_and(|selection| {
                    selection.id != original_selection && selection.id != replacement_selection
                })
        })
    });
    let fresh = reserve_text(&mut case, &endpoint);
    assert!(fresh.id() != plain.id() && fresh.id() != utf8.id() && fresh.id() != null_cut.id());
    plain.cancel();
    utf8.cancel();
    null_cut.cancel();
    assert_eq!(poll_text(&case, &plain), None);
    assert_eq!(poll_text(&case, &utf8), None);
    assert_eq!(poll_text(&case, &null_cut), None);
    assert_context_incoming(&case, &endpoint, &fresh);
    assert!(selection_receives(&case, 703).is_empty());
    fresh.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        selection_receives(case, 703) == vec![UTF8]
    });
    let fresh_fd = watched_text(&observed, &fresh);
    assert!(!Weak::ptr_eq(&utf8_fd, &fresh_fd));
    let writer = case.fixture_mut().take_writer(703);
    finish_context_text(
        &mut case,
        &fresh,
        &fresh_fd,
        writer,
        "Selection nueva á😀\r\n",
    );
    // Keeping this writer open prevents EOF from masquerading as replacement.
    drop(source_open);
    assert_eq!(selection_receives(&case, 701), vec!["text/plain", UTF8]);
    assert!(selection_receives(&case, 702).is_empty());
    assert_eq!(selection_receives(&case, 703), vec![UTF8]);
    for key in [701, 702, 703] {
        assert_no_dnd_requests(&case, key);
    }
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    case.close();
    assert!(endpoint.is_detached());
    text_host_still_works(&mut case);
}
