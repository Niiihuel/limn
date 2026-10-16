//! Public cancellation/context cuts after actual unconsumed text publication.
//! Wire only as a child of public_text_owner_tests; no fixture or source seam.

use super::*;

#[test]
fn public_cuts_discard_published_text_once_and_isolate_the_current_successor() {
    #[derive(Clone, Copy, Debug)]
    enum Cut {
        Cancel,
        CloseOwner,
        KeyboardLoss,
        SelectionReplace,
    }

    for (index, cut) in [
        Cut::Cancel,
        Cut::CloseOwner,
        Cut::KeyboardLoss,
        Cut::SelectionReplace,
    ]
    .into_iter()
    .enumerate()
    {
        let ttl = Duration::from_secs(10);
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
        let key_a = 601 + index * 2;
        let new_selection = key_a + 1;
        let a = focus_selection(&mut case, &endpoint, key_a);
        a.acknowledge().unwrap();
        let (writer_a, fd_a) = received_writer(&mut case, key_a, 1);
        // Ready and the only seal notify before this same worker installs the
        // real reader. No context event is sent before publication is observed.
        let before_wake = case.wakes.load(Ordering::SeqCst);
        drain_public_prefix(
            &mut case,
            &writer_a,
            &fd_a,
            "A no se entrega á😀\r\n".as_bytes(),
        );
        assert!(fd_a.upgrade().is_some());
        let earliest_expiry = Instant::now().checked_add(ttl).unwrap();
        drop(writer_a);
        case.until(|case| {
            assert_eq!(case.owner().status(), ReceiveStatus::Ready);
            assert_eq!(endpoint.status(), EndpointStatus::Ready);
            if case.wakes.load(Ordering::SeqCst) <= before_wake || fd_a.upgrade().is_some() {
                return false;
            }
            // Error retirement releases Incoming before its notification.
            // In this stable unique context this conjunction means Published.
            assert!(matches!(
                case.owner().reserve_text_paste(&endpoint),
                Err(ClipboardError::Busy)
            ));
            true
        });
        assert!(
            Instant::now() < earliest_expiry,
            "{cut:?}: cut precedes TTL expiry"
        );
        assert!(fd_a.upgrade().is_none());
        assert!(endpoint.poll().is_empty());
        assert!(!case.fixture().server.requests.iter().any(|request| {
            matches!(request, OfferRequest::Destroy(actual) if *actual == key_a)
        }));
        assert_eq!(case.fixture().server.released_keyboards, 0);

        // No A poll occurs before the actual worker-side cut evidence below.
        let (b, key_b, count_b, keyboards) = match cut {
            Cut::Cancel => {
                a.cancel();
                // Successful public B reservation proves worker cleanup of A;
                // merely sending cancel is not treated as cleanup evidence.
                let b = reserve_after_cleanup(&mut case, &endpoint);
                assert!(Instant::now() < earliest_expiry);
                (b, key_a, 2, 1)
            }
            Cut::CloseOwner => {
                // A stays alive and unpolled through confirmed join/native
                // cleanup and the existing real host-liveness roundtrip.
                close_public_text(&mut case, 1);
                assert!(Instant::now() < earliest_expiry);
                assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
                assert!(window.upgrade().is_none());
                assert!(fd_a.upgrade().is_none());
                assert_eq!(a.poll(), Some(Err(ClipboardError::Closed)));
                assert_eq!(a.poll(), None);
                a.cancel();
                assert_eq!(a.poll(), None);
                assert_eq!(case.fixture().text_receive_count(key_a, UTF8), 1);
                no_text_dnd_requests(&case, key_a);
                continue;
            }
            Cut::KeyboardLoss => {
                // Deliberately no Leave. The server disabling its resource is
                // not evidence: wait for the worker's real Release and Destroy.
                case.fixture_mut().keyboard_capability(0, false);
                case.until(|case| {
                    assert_eq!(case.owner().status(), ReceiveStatus::Ready);
                    assert_eq!(endpoint.status(), EndpointStatus::Ready);
                    case.fixture().server.released_keyboards == 1
                        && !case.fixture().has_keyboard(0)
                        && case.fixture().server.requests.iter().any(|request| {
                            matches!(request, OfferRequest::Destroy(actual) if *actual == key_a)
                        })
                });
                assert!(Instant::now() < earliest_expiry);
                assert!(case.fixture().server.keyboards.is_empty());
                assert!(matches!(
                    case.owner().reserve_text_paste(&endpoint),
                    Err(ClipboardError::Unavailable)
                ));
                case.fixture_mut().keyboard_capability(0, true);
                ready_text(&mut case); // Requires the replacement real GetKeyboard.
                assert_eq!(case.fixture().server.released_keyboards, 1);
                assert_eq!(case.fixture().server.keyboards.len(), 1);
                // This new resource has no focus. Announce a fresh Selection
                // and legal Enter, then await the actual new successful seal.
                let b = focus_selection(&mut case, &endpoint, new_selection);
                (b, new_selection, 1, 2)
            }
            Cut::SelectionReplace => {
                case.fixture().text_selection(0, new_selection, true, true);
                case.until(|case| {
                    assert_eq!(case.owner().status(), ReceiveStatus::Ready);
                    assert_eq!(endpoint.status(), EndpointStatus::Ready);
                    // State publishes/cancels the old snapshot before it
                    // destroys that exact previous SDK Selection proxy.
                    case.fixture().server.requests.iter().any(|request| {
                        matches!(request, OfferRequest::Destroy(actual) if *actual == key_a)
                    })
                });
                assert!(Instant::now() < earliest_expiry);
                let b = reserve_after_cleanup(&mut case, &endpoint);
                (b, new_selection, 1, 1)
            }
        };

        // The asynchronous cut has definitely been observed, and B's reserve
        // additionally proves admitted A body cleanup. Do not mask a missing
        // context cancellation by manually cancelling A before its first poll.
        assert_eq!(a.poll(), Some(Err(ClipboardError::Cancelled)), "{cut:?}");
        assert_eq!(a.poll(), None);
        assert!(fd_a.upgrade().is_none());
        a.cancel();
        assert_eq!(a.poll(), None);
        b.acknowledge().unwrap();
        let (writer_b, fd_b) = received_writer(&mut case, key_b, count_b);
        assert!(!Weak::ptr_eq(&fd_a, &fd_b));
        let raw_b = "B conserva á😀\r\n";
        drain_public_prefix(&mut case, &writer_b, &fd_b, raw_b.as_bytes());
        a.cancel();
        assert_eq!(a.poll(), None);
        assert!(fd_a.upgrade().is_none());
        assert!(
            fd_b.upgrade().is_some(),
            "{cut:?}: old A cannot retire B's reader"
        );
        assert_eq!(b.poll(), None, "the successor still requires its own EOF");
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
        assert_eq!(case.fixture().text_receive_count(key_b, UTF8), count_b);
        if key_b != key_a {
            assert_eq!(case.fixture().text_receive_count(key_a, UTF8), 1);
            no_text_dnd_requests(&case, key_b);
        }
        no_text_dnd_requests(&case, key_a);
        assert!(endpoint.poll().is_empty());
        close_public_text(&mut case, keyboards);
        assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
        assert!(window.upgrade().is_none());
    }
}
