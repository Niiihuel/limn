//! Public TTL progress through the actual worker and private Wayland server.
//! Wire only as a child of public_text_owner_tests; no new observations required.

use super::*;

#[test]
fn public_unpolled_text_excludes_native_copy_then_expires_before_same_selection_successor() {
    let ttl = Duration::from_secs(1);
    let mut case = PublicOwnerCase::with_text(
        testing::Hooks::default(),
        ReceiveLimits::default(),
        TextClipboardLimits {
            max_text_bytes: 256,
            completion_ttl: ttl,
            ..TextClipboardLimits::default()
        },
    );
    ready_text(&mut case);
    let (endpoint, window) = case.register(0);
    let a = focus_selection(&mut case, &endpoint, 501);
    a.acknowledge().unwrap();
    let (writer_a, fd_a) = received_writer(&mut case, 501, 1);
    // Ready and the only focus seal notify on this same worker before it can
    // install this real reader. No context changes occur during publication.
    let before_wake = case.wakes.load(Ordering::SeqCst);
    drain_public_prefix(&mut case, &writer_a, &fd_a, b"publicado A\r\n");
    assert!(fd_a.upgrade().is_some());
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::Busy)
    ));
    // Publication cannot precede EOF. This is a conservative lower bound on
    // its expiry, not an injected clock or an observation of a private phase.
    let before_eof = Instant::now();
    let earliest_expiry = before_eof.checked_add(ttl).unwrap();
    drop(writer_a);
    case.until(|case| {
        assert_eq!(case.owner().status(), ReceiveStatus::Ready);
        assert_eq!(endpoint.status(), EndpointStatus::Ready);
        if case.wakes.load(Ordering::SeqCst) <= before_wake || fd_a.upgrade().is_some() {
            return false;
        }
        // Error cleanup releases Incoming before it wakes. With this stable
        // context, wake + actual FD death + Busy identifies publication.
        assert!(matches!(
            case.owner().reserve_text_paste(&endpoint),
            Err(ClipboardError::Busy)
        ));
        true
    });
    // A remains entirely unpolled and uncancelled until the worker expires it
    // and a public successor reservation below actually succeeds.
    assert!(endpoint.poll().is_empty());
    let rejected = case.fixture().offer(0, 502, "text/uri-list");
    rejected.action(wl_data_device_manager::DndAction::Copy);
    case.fixture().enter(0, &rejected, 0, 23.0, 37.0);
    case.fixture().server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    case.until(|case| {
        assert!(
            endpoint.poll().is_empty(),
            "occupied Text publishes no Drop event"
        );
        case.fixture()
            .server
            .requests
            .iter()
            .any(|request| matches!(request, OfferRequest::Destroy(502)))
    });
    assert!(
        Instant::now() < earliest_expiry,
        "the real rejection must be observed while Published is still unexpired"
    );
    assert_eq!(case.fixture().received(502), 0);
    assert_eq!(case.fixture().finish_count(502), 0);
    assert_eq!(
        case.fixture()
            .server
            .requests
            .iter()
            .filter(|request| { matches!(request, OfferRequest::Accept(502, None)) })
            .count(),
        1
    );
    assert!(case
        .fixture()
        .server
        .requests
        .iter()
        .all(|request| !matches!(
            request,
            OfferRequest::Accept(502, Some(_))
        ) && !matches!(request, OfferRequest::Actions(502, actions, _)
            if *actions != WEnum::Value(wl_data_device_manager::DndAction::None))));
    assert!(endpoint.poll().is_empty());
    assert!(fd_a.upgrade().is_none());
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::Busy)
    ));
    no_text_dnd_requests(&case, 501);

    // Only worker deadline progress can free A: no public A poll/cancel or new
    // Selection/focus event is sent before this Busy -> successful reservation.
    let b = reserve_after_cleanup(&mut case, &endpoint);
    assert!(
        Instant::now() >= earliest_expiry,
        "TTL must not release A early"
    );
    assert!(fd_a.upgrade().is_none());
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 1);
    // Cancel after worker expiry must preserve Timeout and must not cancel B.
    a.cancel();
    assert_eq!(a.poll(), Some(Err(ClipboardError::Timeout)));
    assert_eq!(a.poll(), None);
    b.acknowledge().unwrap();
    let (writer_b, fd_b) = received_writer(&mut case, 501, 2);
    assert!(!Weak::ptr_eq(&fd_a, &fd_b));
    assert!(fd_b.upgrade().is_some());
    let raw_b = "sucesor B á😀\r\n";
    drain_public_prefix(&mut case, &writer_b, &fd_b, raw_b.as_bytes());
    a.cancel();
    assert_eq!(a.poll(), None);
    assert!(fd_a.upgrade().is_none());
    assert!(
        fd_b.upgrade().is_some(),
        "old A cannot retire B's actual reader"
    );
    assert_eq!(b.poll(), None, "B still requires its own EOF");
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::Busy)
    ));
    drop(writer_b);
    assert_eq!(public_result(&mut case, &b, &fd_b), Ok(raw_b.to_owned()));
    assert!(fd_b.upgrade().is_none());
    assert_eq!(b.poll(), None);
    a.cancel();
    assert_eq!(a.poll(), None);
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 2);
    no_text_dnd_requests(&case, 501);
    assert!(endpoint.poll().is_empty());
    close_public_text(&mut case, 1);
    assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
    assert!(window.upgrade().is_none());
}
