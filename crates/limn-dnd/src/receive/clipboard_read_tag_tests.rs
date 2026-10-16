//! PRIVATE REPLAY of retired typed readiness against a later real pipe.
//! Wire only as a child of owner_wayland_tests after the separate constructor
//! proposal is reviewed. This sequential fixture NEVER installs Generic: no
//! native late callback is claimed, and no ReadTag or domain ID is fabricated.

use super::*;
use crate::receive::{clipboard_control::TextRequestControl, control::IncomingLease};
use crate::{ClipboardError, TextClipboardLimits};

#[test]
fn replayed_old_drop_and_text_tags_cannot_read_the_current_real_pipe() {
    #[derive(Clone, Copy, Debug)]
    enum Route {
        DropToText,
        TextToDrop,
        TextToText,
    }

    for (index, route) in [Route::DropToText, Route::TextToDrop, Route::TextToText]
        .into_iter()
        .enumerate()
    {
        let (mut case, clipboard) =
            OwnerCase::with_text_policy(Some(TextClipboardLimits::default()));
        let clipboard = clipboard.unwrap();
        case.until(|case| {
            case.lifecycle.status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
        });
        let endpoint = case.register(0);
        let control = endpoint.control();
        case.until(|_| control.status() == ReceiveStatus::Ready);
        let selection_key = 1001 + index * 3;
        let drop_a_key = selection_key + 1;
        let drop_b_key = selection_key + 2;
        case.fixture().text_selection(0, selection_key, true, true);
        case.fixture_mut().keyboard_enter(0, 0);
        case.deliver();
        case.until(|_| {
            clipboard
                .observed_seats()
                .into_iter()
                .flatten()
                .any(|seat| {
                    seat.selection.is_some()
                        && seat.focus.is_some_and(|focus| {
                            focus.registration == Some(endpoint.id()) && focus.sealed
                        })
                })
        });
        assert!(case.lifecycle.incoming().is_none());

        let (old_text, old_drop, writer_a): (Option<TextRequestControl>, Option<OfferId>, OwnedFd) =
            if matches!(route, Route::DropToText) {
                let _offer = case.enter(0, drop_a_key, 11.0, 21.0);
                let id = hover_at(&control, 11.0, 21.0);
                case.drop_gesture();
                assert!(
                    matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id)
                );
                case.fixture_mut().serve();
                assert_eq!(case.fixture().received(drop_a_key), 1);
                (None, Some(id), case.fixture_mut().take_writer(drop_a_key))
            } else {
                let request = clipboard
                    .reserve(&control, endpoint.id(), Instant::now())
                    .unwrap();
                request.acknowledge(Instant::now()).unwrap();
                case.guest
                    .as_mut()
                    .unwrap()
                    .pump_readable(case.now, None)
                    .unwrap();
                case.fixture_mut().serve();
                assert_eq!(
                    case.fixture()
                        .text_receive_count(selection_key, "text/plain;charset=utf-8"),
                    1
                );
                (
                    Some(request),
                    None,
                    case.fixture_mut().take_writer(selection_key),
                )
            };
        let (tag_a, actual_a) = case.guest().pipe_fd().expect("A owns a real Receive pipe");
        match tag_a {
            ReadTag::Drop(id) => assert_eq!(Some(id), old_drop),
            ReadTag::Text(id) => {
                assert!(old_text.as_ref().is_some_and(|request| request.id() == id))
            }
        }
        let fd_a = Arc::downgrade(&actual_a);
        assert_eq!(rustix::io::write(&writer_a, b"old A").unwrap(), 5);
        assert_eq!(rustix::io::ioctl_fionread(actual_a.as_ref()).unwrap(), 5);
        drop(actual_a);
        if let Some(request) = &old_text {
            request.cancel();
        }
        if let Some(id) = old_drop {
            control.cancel(id).unwrap();
        }
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, None)
            .unwrap();
        if let Some(request) = &old_text {
            assert_eq!(
                request.poll(Instant::now()),
                Some(Err(ClipboardError::Cancelled))
            );
            assert_eq!(request.poll(Instant::now()), None);
        } else {
            assert!(
                matches!(control.poll().as_slice(), [ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) }] if Some(*id) == old_drop)
            );
            assert!(control.poll().is_empty());
        }
        // Retire A completely before admitting B. There has never been an
        // external watcher, so the existing sequential finalizer is permitted.
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, None)
            .unwrap();
        case.guest.as_mut().unwrap().finish_retirements().unwrap();
        case.fixture_mut().serve();
        assert!(fd_a.upgrade().is_none());
        assert!(case.guest().pipe_fd().is_none());
        assert!(case.lifecycle.incoming().is_none());
        assert_eq!(case.fixture().finish_count(drop_a_key), 0);

        let (text_b, drop_b, writer_b): (Option<TextRequestControl>, Option<OfferId>, OwnedFd) = if matches!(
            route,
            Route::TextToDrop
        ) {
            let _offer = case.enter(0, drop_b_key, 12.0, 22.0);
            let id = hover_at(&control, 12.0, 22.0);
            case.drop_gesture();
            assert!(
                matches!(control.poll().as_slice(), [ReceiveEvent::DropStarted { id: actual, .. }] if *actual == id)
            );
            case.fixture_mut().serve();
            assert_eq!(case.fixture().received(drop_b_key), 1);
            (None, Some(id), case.fixture_mut().take_writer(drop_b_key))
        } else {
            let request = clipboard
                .reserve(&control, endpoint.id(), Instant::now())
                .unwrap();
            request.acknowledge(Instant::now()).unwrap();
            case.guest
                .as_mut()
                .unwrap()
                .pump_readable(case.now, None)
                .unwrap();
            case.fixture_mut().serve();
            let expected_receives = if old_text.is_some() { 2 } else { 1 };
            assert_eq!(
                case.fixture()
                    .text_receive_count(selection_key, "text/plain;charset=utf-8"),
                expected_receives
            );
            (
                Some(request),
                None,
                case.fixture_mut().take_writer(selection_key),
            )
        };
        let (tag_b, actual_b) = case
            .guest()
            .pipe_fd()
            .expect("B owns a different real Receive pipe");
        assert_ne!(tag_a, tag_b);
        let incoming_b = match tag_b {
            ReadTag::Drop(id) => {
                assert_eq!(Some(id), drop_b);
                IncomingLease::Drop {
                    registration: Some(endpoint.id()),
                    id,
                }
            }
            ReadTag::Text(id) => {
                assert!(text_b.as_ref().is_some_and(|request| request.id() == id));
                IncomingLease::Text(id)
            }
        };
        let fd_b = Arc::downgrade(&actual_b);
        assert!(!std::sync::Weak::ptr_eq(&fd_a, &fd_b));
        let payload: &[u8] = if text_b.is_some() {
            "B conserva á😀\r\n".as_bytes()
        } else {
            b"file:///tmp/read-tag-b\r\n"
        };
        assert!(payload.len() < 1024);
        assert_eq!(
            rustix::io::write(&writer_b, payload).unwrap(),
            payload.len()
        );
        assert_eq!(
            rustix::io::ioctl_fionread(actual_b.as_ref()).unwrap() as usize,
            payload.len()
        );
        drop(actual_b);

        // This is deliberate PRIVATE REPLAY, not a native callback surviving
        // into B. Incoming already required A's actual reader to be dead.
        // Do not call case.pump/step/deliver here: they intentionally select
        // the current tag, which would mask a broken stale-readiness guard.
        for replay in [Some(tag_a), None, Some(tag_a)] {
            case.guest
                .as_mut()
                .unwrap()
                .pump_readable(case.now, replay)
                .unwrap();
            case.guest.as_mut().unwrap().finish_retirements().unwrap();
            let (same_tag, same_fd) = case.guest().pipe_fd().expect("replayed A cannot retire B");
            assert_eq!(same_tag, tag_b);
            assert!(std::sync::Weak::ptr_eq(&fd_b, &Arc::downgrade(&same_fd)));
            assert_eq!(
                rustix::io::ioctl_fionread(same_fd.as_ref()).unwrap() as usize,
                payload.len()
            );
            drop(same_fd);
            assert!(case.lifecycle.incoming() == Some(incoming_b));
            assert!(fd_a.upgrade().is_none());
            if let Some(request) = &text_b {
                assert_eq!(request.poll(Instant::now()), None);
            }
            assert!(control.poll().is_empty());
        }
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, Some(tag_b))
            .unwrap();
        {
            let (same_tag, same_fd) = case
                .guest()
                .pipe_fd()
                .expect("B still requires its own EOF");
            assert_eq!(same_tag, tag_b);
            assert!(std::sync::Weak::ptr_eq(&fd_b, &Arc::downgrade(&same_fd)));
            assert_eq!(rustix::io::ioctl_fionread(same_fd.as_ref()).unwrap(), 0);
        }
        assert!(case.lifecycle.incoming() == Some(incoming_b));
        if let Some(request) = &text_b {
            assert_eq!(request.poll(Instant::now()), None);
        }
        assert!(control.poll().is_empty());
        drop(writer_b);
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, Some(tag_b))
            .unwrap();
        assert!(case.guest().pipe_fd().is_none());
        assert!(
            fd_b.upgrade().is_none(),
            "no Generic ever existed; all observer Arcs were released"
        );
        // Exactly the never-installed-watcher precondition. This does not
        // exercise Runtime::sync_pipe or claim native watcher-retirement proof.
        case.guest.as_mut().unwrap().finish_retirements().unwrap();
        if let Some(request) = &text_b {
            assert_eq!(
                request.poll(Instant::now()),
                Some(Ok(std::str::from_utf8(payload).unwrap().to_owned()))
            );
            assert_eq!(request.poll(Instant::now()), None);
        } else {
            let id = drop_b.unwrap();
            assert!(
                matches!(control.poll().as_slice(), [ReceiveEvent::DropReady { id: actual, paths }]
                if *actual == id && paths == &vec![std::path::PathBuf::from("/tmp/read-tag-b")])
            );
            assert!(control.poll().is_empty());
            control.decide(id, DropDecision::Accept).unwrap();
            case.guest
                .as_mut()
                .unwrap()
                .pump_readable(case.now, None)
                .unwrap();
            assert!(
                matches!(control.poll().as_slice(), [ReceiveEvent::DropEnded { id: actual, result: Ok(()) }] if *actual == id)
            );
            assert!(control.poll().is_empty());
        }
        case.guest
            .as_mut()
            .unwrap()
            .pump_readable(case.now, None)
            .unwrap();
        case.guest.as_mut().unwrap().finish_retirements().unwrap();
        assert!(case.lifecycle.incoming().is_none());
        assert!(fd_a.upgrade().is_none());
        assert!(fd_b.upgrade().is_none());
        case.fixture_mut().serve();
        assert_eq!(case.fixture().finish_count(drop_a_key), 0);
        assert_eq!(
            case.fixture().finish_count(drop_b_key),
            usize::from(drop_b.is_some())
        );
        assert_eq!(case.fixture().finish_count(selection_key), 0);
        assert!(case.fixture().server.requests.iter().all(|request| !matches!(request,
            OfferRequest::Accept(key, _) | OfferRequest::Actions(key, _, _) | OfferRequest::Finish(key)
            if *key == selection_key)));
        if let Some(request) = &old_text {
            assert_eq!(request.poll(Instant::now()), None);
        }
        assert!(control.poll().is_empty());
        // A's writer stayed open for B's entire transfer and exact result.
        drop(writer_a);
    }
}
