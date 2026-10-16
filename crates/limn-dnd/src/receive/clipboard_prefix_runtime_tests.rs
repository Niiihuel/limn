//! Both orders cross one real guest dispatch, observed by cfg(test) counters.

use super::*;

#[test]
fn clipboard_quarantine_done_and_enter_in_one_dispatch_preserve_the_protocol_prefix_order() {
    for enter_before_done in [false, true] {
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
        let (a, old, writer_a, old_fd) = first_request(&mut case, 201);
        let selection = focused(&case, Some(a.id())).unwrap().selection.unwrap().id;
        let (b, _) = case.register(1, || {});
        let b_control = b.control();
        drive_text(&mut case, |_| b_control.status() == ReceiveStatus::Ready);
        assert!(matches!(
            retirement_entered.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        let previous_done = observed
            .clipboard_quarantine_dispatch
            .load(Ordering::SeqCst);
        let previous_enter = observed.keyboard_enter_dispatch.load(Ordering::SeqCst);
        let previous_dispatch = observed.guest_dispatches.load(Ordering::SeqCst);

        // Deliver Leave alone. Do not serve client requests while waiting: that
        // would answer the newly emitted Quarantine Sync before choosing order.
        case.fixture_mut().keyboard_leave(0, 0);
        case.fixture_mut().display.flush_clients().unwrap();
        case.fixture().host_read();
        case.ping.ping();
        retirement_entered
            .recv_timeout(WAIT)
            .expect("Leave retires the real active text pipe");
        assert!(old_fd.upgrade().is_some());
        assert_eq!(poll_text(&case, &old), Some(Err(ClipboardError::Cancelled)));
        assert_eq!(poll_text(&case, &old), None);
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(old.id())));

        // The worker is paused after its real Sync was flushed. The server's
        // built-in Sync handler produces the actual Done; no callback is forged.
        if enter_before_done {
            case.fixture_mut().keyboard_enter(0, 1);
            case.fixture_mut().serve();
        } else {
            case.fixture_mut().serve();
            case.fixture_mut().keyboard_enter(0, 1);
            case.fixture_mut().display.flush_clients().unwrap();
        }
        case.fixture().host_read();
        release.release();
        case.ping.ping();
        drive_text(&mut case, |case| {
            assert_eq!(poll_text(case, &old), None);
            observed
                .clipboard_quarantine_dispatch
                .load(Ordering::SeqCst)
                > previous_done
                && observed.keyboard_enter_dispatch.load(Ordering::SeqCst) > previous_enter
                && focused(case, Some(b.id())).is_some_and(|seat| seat.focus.unwrap().sealed)
                && case.lifecycle.incoming().is_none()
        });
        let done_dispatch = observed
            .clipboard_quarantine_dispatch
            .load(Ordering::SeqCst);
        let enter_dispatch = observed.keyboard_enter_dispatch.load(Ordering::SeqCst);
        assert!(
            done_dispatch > previous_done && done_dispatch > previous_dispatch && done_dispatch > 0
        );
        assert!(
            enter_dispatch > previous_enter
                && enter_dispatch > previous_dispatch
                && enter_dispatch > 0
        );
        assert_eq!(
            done_dispatch, enter_dispatch,
            "both real events must run in one dispatch_pending call"
        );
        assert!(old_fd.upgrade().is_none());
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
        assert!(!a.is_detached() && !b.is_detached());
        if !enter_before_done {
            drive_text(&mut case, |case| {
                case.fixture()
                    .server
                    .requests
                    .iter()
                    .any(|request| matches!(request, OfferRequest::Destroy(key) if *key == 201))
            });
        }
        let after = focused(&case, Some(b.id())).unwrap();
        let key = if enter_before_done {
            assert!(after
                .selection
                .is_some_and(|current| current.id == selection));
            201
        } else {
            assert!(
                after.selection.is_none(),
                "Done ended the old Selection prefix before Enter"
            );
            assert!(matches!(
                case.clipboard().reserve(&b_control, b.id(), Instant::now()),
                Err(ClipboardError::NoSelection)
            ));
            case.fixture().text_selection(0, 202, true, true);
            // Selection is asynchronous. Keep the prior NoSelection assertion,
            // then wait for the worker's actual replacement snapshot before
            // Reserve; do not retry every NoSelection in the general helper.
            drive_text(&mut case, |case| {
                focused(case, Some(b.id())).is_some_and(|seat| {
                    seat.selection
                        .is_some_and(|current| current.id != selection)
                })
            });
            202
        };
        let fresh = reserve_text(&mut case, &b);
        old.cancel();
        assert_eq!(poll_text(&case, &old), None);
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(fresh.id())));
        fresh.acknowledge(Instant::now()).unwrap();
        case.ping.ping();
        let expected_receives = if enter_before_done { 2 } else { 1 };
        drive_text(&mut case, |case| {
            case.fixture().text_receive_count(key, UTF8) == expected_receives
        });
        let fresh_fd = watched_text(&observed, &fresh);
        let writer_b = case.fixture_mut().take_writer(key);
        // A's source writer remains open through B's real admission and Receive.
        drop(writer_a);
        write_chunk(&writer_b, b"prefijo B\r\n");
        drop(writer_b);
        let mut outcome = None;
        drive_text(&mut case, |case| {
            outcome = poll_text(case, &fresh);
            if matches!(outcome, Some(Ok(_))) {
                assert!(
                    fresh_fd.upgrade().is_none(),
                    "observed text success requires the actual watcher FD gone"
                );
            }
            outcome.is_some()
        });
        assert_eq!(outcome, Some(Ok("prefijo B\r\n".to_owned())));
        assert_eq!(poll_text(&case, &fresh), None);
        assert_no_dnd_requests(&case, 201);
        assert_no_dnd_requests(&case, key);
        assert_eq!(case.fixture().server.devices, 1);
        case.close();
        assert!(a.is_detached() && b.is_detached());
        text_host_still_works(&mut case);
    }
}
