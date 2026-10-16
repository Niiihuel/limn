//! Selection, real Copy and unassigned native offers share the existing cap of 8.
//! Unassigned-offer accumulation is adversarial resource pressure, not normal
//! compositor sequencing or evidence of live-compositor interoperability.

use super::*;

#[test]
fn eight_mixed_native_offers_remain_intact_and_the_ninth_is_destroyed_before_cleanup() {
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
    let selection_offer = case.fixture().text_selection(0, 901, true, true);
    case.fixture_mut().keyboard_enter(0, 0);
    drive_text(&mut case, |case| {
        focused(case, None).is_some_and(|seat| seat.selection.is_some())
    });
    let selection = focused(&case, None).unwrap().selection.unwrap().id;
    let (a, _) = case.register(0, || {});
    let (b, _) = case.register(1, || {});
    let a_control = a.control();
    let b_control = b.control();
    drive_text(&mut case, |case| {
        a_control.status() == ReceiveStatus::Ready
            && b_control.status() == ReceiveStatus::Ready
            && focused(case, Some(a.id())).is_some_and(|seat| seat.focus.unwrap().sealed)
    });
    let copy_offer = case.fixture().offer(0, 902, "text/uri-list");
    copy_offer.action(wl_data_device_manager::DndAction::Copy);
    case.fixture().enter(0, &copy_offer, 1, 61.0, 71.0);
    let mut hovered = Vec::new();
    drive_text(&mut case, |_| {
        hovered.extend(b_control.poll());
        !hovered.is_empty()
    });
    let id = match hovered.as_slice() {
        [ReceiveEvent::Hovered { id, point }] => {
            assert_eq!(*point, SurfacePoint { x: 61.0, y: 71.0 });
            *id
        }
        _ => panic!("the real native Copy publishes one Hovered"),
    };
    let exact = IncomingLease::Drop {
        registration: Some(b.id()),
        id,
    };
    assert!(case.lifecycle.incoming() == Some(exact));
    let pending: Vec<_> = (903..=908)
        .map(|key| {
            let offer = case.fixture().introduce_offer(0, key);
            offer.offer(UTF8.to_owned());
            offer
        })
        .collect();
    assert_eq!(pending.len(), 6);
    // A real Motion in the SAME native queue follows all six DataOffer/MIME
    // pairs. Its exact Hovered is a positive witness of that protocol prefix.
    case.fixture().server.device_resources[0].motion(62, 62.0, 72.0);
    let mut suffix = Vec::new();
    drive_text(&mut case, |_| {
        suffix.extend(b_control.poll());
        !suffix.is_empty()
    });
    assert!(
        matches!(suffix.as_slice(), [ReceiveEvent::Hovered { id: actual, point }]
        if *actual == id && *point == SurfacePoint { x: 62.0, y: 72.0 })
    );
    // Observe a completed pump after the suffix publication, then receive its
    // already-flushed client requests before asserting that no offer was evicted.
    let pump_after_suffix = observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || case.fixture().host_read(),
        || observed.completed_pumps.load(Ordering::SeqCst) > pump_after_suffix,
    );
    case.fixture_mut().serve();
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert_eq!(b_control.status(), ReceiveStatus::Ready);
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(a_control.incoming() == Some(exact) && b_control.incoming() == Some(exact));
    assert!(focused(&case, Some(a.id())).unwrap().selection.unwrap().id == selection);
    assert!(selection_offer.is_alive() && copy_offer.is_alive());
    assert!(pending.iter().all(|offer| offer.is_alive()));
    assert!(case
        .fixture()
        .server
        .requests
        .iter()
        .all(|request| !matches!(request,
        OfferRequest::Destroy(key) if (901..=908).contains(key))));
    assert_eq!(case.fixture().text_receive_count(901, UTF8), 0);
    assert_eq!(case.fixture().received(902), 0);
    assert!(matches!(
        case.clipboard().reserve(&a_control, a.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
    assert!(observed.watched_pipe.lock().unwrap().is_none());

    // The same surviving Copy now creates its real typed receive and Generic.
    case.fixture().server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    let mut started = Vec::new();
    drive_text(&mut case, |case| {
        started.extend(b_control.poll());
        case.fixture().received(902) == 1 && !started.is_empty()
    });
    assert!(
        matches!(started.as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id)
    );
    let fd = watched_fd(&observed, id);
    let source_open = case.fixture_mut().take_writer(902);
    write_chunk(&source_open, b"file:///tmp/eight-offers-still-reading\r\n");
    drive_text(&mut case, |_| {
        let current = fd
            .upgrade()
            .expect("the original Copy reader remains installed");
        rustix::io::ioctl_fionread(current.as_ref()).unwrap() == 0
    });
    assert!(fd.upgrade().is_some());
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(a_control.incoming() == Some(exact) && b_control.incoming() == Some(exact));
    assert!(
        b_control.poll().is_empty(),
        "open source without EOF cannot publish DropReady"
    );
    assert_eq!(case.fixture().received(902), 1);
    assert_eq!(case.fixture().finish_count(902), 0);
    assert!(selection_offer.is_alive() && copy_offer.is_alive());
    assert!(pending.iter().all(|offer| offer.is_alive()));
    assert!(case
        .fixture()
        .server
        .requests
        .iter()
        .all(|request| !matches!(request,
        OfferRequest::Destroy(key) if (901..=908).contains(key))));
    let before_overflow = case.fixture().server.requests.len();
    let ninth = case.fixture().introduce_offer(0, 909);
    drive_text(&mut case, |case| {
        let incoming = case.lifecycle.incoming();
        if incoming.is_none() {
            assert!(fd.upgrade().is_none());
        } else {
            assert!(incoming == Some(exact));
        }
        matches!(
            case.lifecycle.status(),
            ReceiveStatus::Closed(Some(ReceiveError::ResourceLimit))
        )
    });
    assert!(observed.cleaned.load(Ordering::SeqCst));
    assert!(!observed.cleanup_failed.load(Ordering::SeqCst));
    assert!(fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert!(a.is_detached() && b.is_detached());
    // Root Closed follows normal cleanup/join. Keep the real host/display alive
    // and collect the native destructor requests before inspecting their order.
    text_host_still_works(&mut case);
    let destroyed: Vec<_> = case.fixture().server.requests[before_overflow..]
        .iter()
        .filter_map(|request| {
            if let OfferRequest::Destroy(key) = request {
                Some(*key)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(destroyed.first(), Some(&909));
    let mut all_destroyed = destroyed.clone();
    all_destroyed.sort_unstable();
    assert_eq!(all_destroyed, (901..=909).collect::<Vec<_>>());
    assert!(!ninth.is_alive());
    assert!(!selection_offer.is_alive() && !copy_offer.is_alive());
    assert!(pending.iter().all(|offer| !offer.is_alive()));
    assert_eq!(case.fixture().received(902), 1);
    assert_eq!(case.fixture().finish_count(902), 0);
    assert_eq!(case.fixture().text_receive_count(901, UTF8), 0);
    assert_no_dnd_requests(&case, 901);
    for key in 903..=909 {
        assert_no_dnd_requests(&case, key);
        assert!(case.fixture().server.requests.iter().all(|request| !matches!(request,
            OfferRequest::Receive(actual, _, _) | OfferRequest::ReceiveTaken(actual, _) if *actual == key)));
    }
    assert!(matches!(poll(&b_control, &case.ping).as_slice(),
        [ReceiveEvent::DropEnded { id: actual, result: Err(ReceiveError::ResourceLimit) }] if *actual == id));
    assert!(poll(&b_control, &case.ping).is_empty());
    // EOF cannot account for failure or cleanup: this writer remained open.
    drop(source_open);
}
