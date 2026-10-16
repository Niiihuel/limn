//! Native DnD/Text exclusion through the real owner, protocol offers and watcher.
//! Wire only as a child of clipboard_runtime_tests; no fixture changes required.
use super::*;
use crate::receive::control::AttemptGate;
use crate::DropDecision;

const URI: &[u8] = b"file:///tmp/rune-native-crossing\r\n";
const URI_PATH: &str = "/tmp/rune-native-crossing";

fn assert_same_native_watcher(case: &LiveOwnerCase, tag: ReadTag, fd: &Weak<OwnedFd>) {
    let observed = case.observed.watched_pipe.lock().unwrap();
    let (actual, observed_fd) = observed.as_ref().expect("the actual Generic was installed");
    assert!(*actual == tag);
    assert!(
        Weak::ptr_eq(observed_fd, fd),
        "a competing gesture cannot replace the exact FD"
    );
}

fn assert_copy_incoming(case: &LiveOwnerCase, endpoint: &SurfaceLease, id: OfferId) {
    let exact = IncomingLease::Drop {
        registration: Some(endpoint.id()),
        id,
    };
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(endpoint.control().incoming() == Some(exact));
}

fn drain_copy_ready(case: &mut LiveOwnerCase, endpoint: &SurfaceLease, id: OfferId) {
    let control = endpoint.control();
    let ping = case.ping.clone();
    let mut ready = Vec::new();
    drive_text(case, |_| {
        ready.extend(poll(&control, &ping));
        ready
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropReady { .. }))
    });
    assert!(
        matches!(ready.as_slice(), [ReceiveEvent::DropReady { id: actual, paths }]
        if *actual == id && *paths == vec![std::path::PathBuf::from(URI_PATH)])
    );
    assert_copy_incoming(case, endpoint, id);
}

fn accept_copy_without_draining_terminal(
    case: &mut LiveOwnerCase,
    endpoint: &SurfaceLease,
    id: OfferId,
    key: usize,
) {
    let control = endpoint.control();
    control.decide(id, DropDecision::Accept).unwrap();
    case.ping.ping();
    drive_text(case, |case| {
        // Drop the child header guard before any parent/context observation.
        let ended = {
            let header = control.header();
            header.offer.is_some_and(|offer| {
                offer.id == id && matches!(offer.gate, AttemptGate::Ended { drained: false })
            })
        };
        ended && case.fixture().finish_count(key) == 1
    });
    assert_copy_incoming(case, endpoint, id);
    assert_eq!(case.fixture().received(key), 1);
    assert_eq!(case.fixture().finish_count(key), 1);
}

fn drain_copy_terminal(case: &mut LiveOwnerCase, endpoint: &SurfaceLease, id: OfferId, key: usize) {
    let control = endpoint.control();
    assert!(matches!(poll(&control, &case.ping).as_slice(),
        [ReceiveEvent::DropEnded { id: actual, result: Ok(()) }] if *actual == id));
    drive_text(case, |case| case.lifecycle.incoming().is_none());
    assert!(poll(&control, &case.ping).is_empty());
    assert_eq!(case.fixture().received(key), 1);
    assert_eq!(case.fixture().finish_count(key), 1);
}

#[test]
fn reserved_and_reading_text_reject_native_copy_without_replacing_the_transfer() {
    // Published-but-unpolled Text is deliberately left for the TTL/publication
    // slice; this test adds no observation seam to peek at or consume its body.
    for reading in [false, true] {
        let observed = testing::Observations::default();
        let mut case = LiveOwnerCase::with_text(
            testing::Hooks {
                observed: observed.clone(),
                ..Default::default()
            },
            ReceiveLimits::default(),
            TextClipboardLimits::default(),
        );
        let selection = prepare_unregistered_selection(&mut case, 401);
        let (a, _) = case.register(0, || {});
        let (b, _) = case.register(1, || {});
        let a_control = a.control();
        let b_control = b.control();
        drive_text(&mut case, |_| {
            a_control.status() == ReceiveStatus::Ready && b_control.status() == ReceiveStatus::Ready
        });
        let request = reserve_text(&mut case, &a);
        assert!(focused(&case, Some(a.id())).unwrap().selection.unwrap().id == selection);
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
        assert!(b_control.incoming() == Some(IncomingLease::Text(request.id())));
        assert_eq!(case.fixture().text_receive_count(401, UTF8), 0);
        let mut writer = None;
        let mut text_fd = None;
        if reading {
            request.acknowledge(Instant::now()).unwrap();
            case.ping.ping();
            drive_text(&mut case, |case| {
                case.fixture().text_receive_count(401, UTF8) == 1
            });
            let fd = watched_text(&observed, &request);
            let source = case.fixture_mut().take_writer(401);
            drain_prefix(&mut case, &source, &fd);
            assert!(fd.upgrade().is_some());
            assert_same_native_watcher(&case, ReadTag::Text(request.id()), &fd);
            writer = Some(source);
            text_fd = Some(fd);
        } else {
            assert!(observed.watched_pipe.lock().unwrap().is_none());
        }

        // This is a real native gesture on B, not a control-only Drop reservation.
        let rejected = case.fixture().offer(0, 402, "text/uri-list");
        rejected.action(wl_data_device_manager::DndAction::Copy);
        case.fixture().enter(0, &rejected, 1, 21.0, 31.0);
        case.fixture().server.device_resources[0]
            .send_event(wl_data_device::Event::Drop {})
            .unwrap();
        drive_text(&mut case, |case| {
            assert!(
                b_control.poll().is_empty(),
                "busy native Copy publishes no Drop event"
            );
            assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
            case.fixture()
                .server
                .requests
                .iter()
                .any(|event| matches!(event, OfferRequest::Destroy(402)))
        });
        assert!(b_control.poll().is_empty());
        assert_eq!(case.fixture().received(402), 0);
        assert_eq!(case.fixture().finish_count(402), 0);
        assert_eq!(
            case.fixture()
                .server
                .requests
                .iter()
                .filter(|event| matches!(event, OfferRequest::Accept(402, None)))
                .count(),
            1
        );
        assert!(case
            .fixture()
            .server
            .requests
            .iter()
            // Refusal offers no action; it never selects Copy.
            .all(|event| !matches!(event, OfferRequest::Actions(402, actions, _)
                if *actions != WEnum::Value(wl_data_device_manager::DndAction::None))));
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
        assert!(a_control.incoming() == Some(IncomingLease::Text(request.id())));
        assert!(b_control.incoming() == Some(IncomingLease::Text(request.id())));
        assert_eq!(poll_text(&case, &request), None);
        assert!(focused(&case, Some(a.id())).unwrap().selection.unwrap().id == selection);
        if let Some(fd) = text_fd.as_ref() {
            assert!(fd.upgrade().is_some());
            assert_same_native_watcher(&case, ReadTag::Text(request.id()), fd);
            assert_eq!(case.fixture().text_receive_count(401, UTF8), 1);
        } else {
            assert!(observed.watched_pipe.lock().unwrap().is_none());
            assert_eq!(case.fixture().text_receive_count(401, UTF8), 0);
            request.acknowledge(Instant::now()).unwrap();
            case.ping.ping();
            drive_text(&mut case, |case| {
                case.fixture().text_receive_count(401, UTF8) == 1
            });
            text_fd = Some(watched_text(&observed, &request));
            writer = Some(case.fixture_mut().take_writer(401));
        }
        write_chunk(writer.as_ref().unwrap(), b"texto preservado\r\n");
        drop(writer.take());
        let fd = text_fd.unwrap();
        let mut outcome = None;
        drive_text(&mut case, |case| {
            outcome = poll_text(case, &request);
            if matches!(outcome, Some(Ok(_))) {
                assert!(fd.upgrade().is_none());
            }
            outcome.is_some()
        });
        let expected = if reading {
            format!(
                "{}texto preservado\r\n",
                std::str::from_utf8(PREFIX).unwrap()
            )
        } else {
            "texto preservado\r\n".to_owned()
        };
        assert_eq!(outcome, Some(Ok(expected)));
        assert_eq!(poll_text(&case, &request), None);
        assert!(fd.upgrade().is_none());
        assert!(case.lifecycle.incoming().is_none());
        assert_no_dnd_requests(&case, 401);

        // A genuine Copy works after Text consumption; rejection was exclusion,
        // not a disabled or permanently damaged DnD protocol path.
        let (copy, copy_writer) = case.start(1, 403, &b_control);
        let copy_fd = watched_fd(&observed, copy);
        assert!(copy_fd.upgrade().is_some());
        assert_copy_incoming(&case, &b, copy);
        assert_same_native_watcher(&case, ReadTag::Drop(copy), &copy_fd);
        write_chunk(&copy_writer, URI);
        drop(copy_writer);
        drain_copy_ready(&mut case, &b, copy);
        accept_copy_without_draining_terminal(&mut case, &b, copy, 403);
        drain_copy_terminal(&mut case, &b, copy, 403);
        assert!(copy_fd.upgrade().is_none());
        assert_eq!(case.fixture().received(402), 0);
        assert_eq!(case.fixture().finish_count(402), 0);
        assert_no_dnd_requests(&case, 401);
        assert_eq!(case.fixture().server.devices, 1);
        assert_eq!(a_control.status(), ReceiveStatus::Ready);
        assert_eq!(b_control.status(), ReceiveStatus::Ready);
        case.close();
        assert!(a.is_detached() && b.is_detached());
        text_host_still_works(&mut case);
    }
}

#[test]
fn native_copy_hover_read_decision_and_undrained_terminal_exclude_text_then_release_it() {
    let observed = testing::Observations::default();
    let mut case = LiveOwnerCase::with_text(
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        ReceiveLimits::default(),
        TextClipboardLimits::default(),
    );
    let selection = prepare_unregistered_selection(&mut case, 501);
    let (a, _) = case.register(0, || {});
    let (b, _) = case.register(1, || {});
    let a_control = a.control();
    let b_control = b.control();
    drive_text(&mut case, |case| {
        a_control.status() == ReceiveStatus::Ready
            && b_control.status() == ReceiveStatus::Ready
            && focused(case, Some(a.id())).is_some_and(|seat| seat.focus.unwrap().sealed)
    });
    assert!(case.lifecycle.incoming().is_none());
    assert!(focused(&case, Some(a.id())).unwrap().selection.unwrap().id == selection);

    let offer = case.fixture().offer(0, 502, "text/uri-list");
    offer.action(wl_data_device_manager::DndAction::Copy);
    case.fixture().enter(0, &offer, 1, 41.0, 51.0);
    let mut hovered = Vec::new();
    drive_text(&mut case, |_| {
        hovered.extend(b_control.poll());
        !hovered.is_empty()
    });
    let id = match hovered.as_slice() {
        [ReceiveEvent::Hovered { id, point }] => {
            assert_eq!(*point, SurfacePoint { x: 41.0, y: 51.0 });
            *id
        }
        _ => panic!("the actual native Copy must publish exactly one Hovered"),
    };
    assert_copy_incoming(&case, &b, id);
    assert!(
        a_control.incoming()
            == Some(IncomingLease::Drop {
                registration: Some(b.id()),
                id
            })
    );
    assert!(matches!(
        case.clipboard().reserve(&a_control, a.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
    assert_eq!(case.fixture().received(502), 0);
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 0);
    assert!(observed.watched_pipe.lock().unwrap().is_none());

    case.fixture().server.device_resources[0]
        .send_event(wl_data_device::Event::Drop {})
        .unwrap();
    let mut started = Vec::new();
    drive_text(&mut case, |case| {
        started.extend(b_control.poll());
        case.fixture().received(502) == 1 && !started.is_empty()
    });
    assert!(
        matches!(started.as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id)
    );
    let fd = watched_fd(&observed, id);
    let writer = case.fixture_mut().take_writer(502);
    write_chunk(&writer, URI);
    drive_text(&mut case, |_| {
        let current = fd
            .upgrade()
            .expect("the native Copy source writer remains open");
        rustix::io::ioctl_fionread(current.as_ref()).unwrap() == 0
    });
    assert!(
        b_control.poll().is_empty(),
        "URI content without EOF is not DropReady"
    );
    assert!(fd.upgrade().is_some());
    assert_same_native_watcher(&case, ReadTag::Drop(id), &fd);
    assert_copy_incoming(&case, &b, id);
    assert!(matches!(
        case.clipboard().reserve(&a_control, a.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 0);
    assert_eq!(case.fixture().finish_count(502), 0);

    drop(writer);
    drain_copy_ready(&mut case, &b, id);
    drive_text(&mut case, |_| fd.upgrade().is_none());
    // AwaitingDecision holds Incoming even after the actual FD watcher is gone.
    assert_copy_incoming(&case, &b, id);
    assert!(matches!(
        case.clipboard().reserve(&a_control, a.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 0);
    assert_eq!(case.fixture().finish_count(502), 0);
    accept_copy_without_draining_terminal(&mut case, &b, id, 502);
    assert!(fd.upgrade().is_none());
    assert!(matches!(
        case.clipboard().reserve(&a_control, a.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
    assert_eq!(case.fixture().text_receive_count(501, UTF8), 0);
    // Only this actual terminal drain acknowledges the accepted native Copy.
    drain_copy_terminal(&mut case, &b, id, 502);

    let request = reserve_text(&mut case, &a);
    assert!(focused(&case, Some(a.id())).unwrap().selection.unwrap().id == selection);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    request.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        case.fixture().text_receive_count(501, UTF8) == 1
    });
    let text_fd = watched_text(&observed, &request);
    assert!(text_fd.upgrade().is_some());
    assert_same_native_watcher(&case, ReadTag::Text(request.id()), &text_fd);
    assert!(poll(&b_control, &case.ping).is_empty());
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    assert!(a_control.incoming() == Some(IncomingLease::Text(request.id())));
    assert!(b_control.incoming() == Some(IncomingLease::Text(request.id())));
    assert_eq!(case.fixture().finish_count(502), 1);
    let text_writer = case.fixture_mut().take_writer(501);
    write_chunk(&text_writer, b"texto despues de Copy\r\n");
    drop(text_writer);
    let mut outcome = None;
    drive_text(&mut case, |case| {
        outcome = poll_text(case, &request);
        if matches!(outcome, Some(Ok(_))) {
            assert!(text_fd.upgrade().is_none());
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok("texto despues de Copy\r\n".to_owned())));
    assert_eq!(poll_text(&case, &request), None);
    assert!(text_fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert_eq!(case.fixture().received(502), 1);
    assert_eq!(case.fixture().finish_count(502), 1);
    assert_no_dnd_requests(&case, 501);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(a_control.status(), ReceiveStatus::Ready);
    assert_eq!(b_control.status(), ReceiveStatus::Ready);
    case.close();
    assert!(a.is_detached() && b.is_detached());
    text_host_still_works(&mut case);
}
