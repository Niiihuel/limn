//! Text teardown with its real host reader paused and source writer still open.
//! Wire only as a child of clipboard_runtime_tests; no new fixture or runtime hook.

use super::*;

#[test]
fn paused_host_text_cleanup_closes_normally_or_retains_uncertain_native_resources() {
    for uncertain in [false, true] {
        let observed = testing::Observations::default();
        let (entered, retirement_entered) = mpsc::sync_channel(1);
        let (release, retirement_release) = mpsc::sync_channel(1);
        let mut case = LiveOwnerCase::with_text(
            testing::Hooks {
                observed: observed.clone(),
                before_retire_sync: Some((entered, retirement_release)),
                panic_cleanup: uncertain,
                ..Default::default()
            },
            ReceiveLimits::default(),
            TextClipboardLimits::default(),
        );
        // Declared after case so assertion unwind opens the bounded gate before
        // LiveOwnerCase performs its existing safe cleanup/retention fallback.
        let mut release = OpenOnDrop(Some(release));
        let key = if uncertain { 902 } else { 901 };
        prepare_unregistered_selection(&mut case, key);
        // Keep the existing registration's actual Window Weak; first_request
        // intentionally discards it and cannot witness native-owner retirement.
        let (endpoint, window) = case.register(0, || {});
        let request = reserve_text(&mut case, &endpoint);
        request.acknowledge(Instant::now()).unwrap();
        case.ping.ping();
        drive_text(&mut case, |case| {
            case.fixture().text_receive_count(key, UTF8) == 1
        });
        let fd = watched_text(&observed, &request);
        let writer = case.fixture_mut().take_writer(key);
        drain_prefix(&mut case, &writer, &fd);
        assert_eq!(poll_text(&case, &request), None);
        assert!(fd.upgrade().is_some());
        assert!(window.upgrade().is_some());
        assert!(case.display_owner.upgrade().is_some());
        assert_eq!(case.fixture().server.devices, 1);
        assert_eq!(case.fixture().server.keyboards.len(), 1);
        assert_eq!(case.fixture().server.released_devices, 0);
        assert_eq!(case.fixture().server.released_seats, 0);
        assert_eq!(case.fixture().server.released_keyboards, 0);
        assert!(matches!(
            retirement_entered.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));

        // Pause both server dispatch and coordinated host reading here. The
        // fixture/display remain alive. The source writer stays open through
        // the final host-liveness roundtrip below; no EOF or new bytes can help.
        request.cancel();
        case.ping.ping();
        retirement_entered.recv_timeout(WAIT).expect(
            "text cancellation reaches the real watcher-retirement gate without host reads",
        );
        assert!(
            fd.upgrade().is_some(),
            "Generic still owns the exact retired State reader"
        );
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
        assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
        assert!(!endpoint.is_detached());
        text_is_busy(&case, &endpoint);
        assert_eq!(
            poll_text(&case, &request),
            Some(Err(ClipboardError::Cancelled))
        );
        assert_eq!(poll_text(&case, &request), None);

        // Cut root/endpoint authority while the exact Generic is still held by
        // the existing gate. Native cleanup has not run and Incoming stays held.
        case.pool.close_owner(case.owner, None);
        case.ping.ping();
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Closing);
        assert_eq!(endpoint.control().status(), ReceiveStatus::Closing);
        assert!(!endpoint.is_detached());
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
        assert!(fd.upgrade().is_some());
        assert!(window.upgrade().is_some());
        assert!(case.display_owner.upgrade().is_some());
        assert!(!observed.cleaned.load(Ordering::SeqCst));
        assert!(!observed.cleanup_failed.load(Ordering::SeqCst));
        assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
        release.release();

        // No serve/host_read/drive helper executes while waiting for either
        // outcome. Both waits use the existing bounded WAIT implementation.
        if uncertain {
            wait(|| {}, || observed.cleanup_failed.load(Ordering::SeqCst));
            assert_eq!(case.lifecycle.status(), ReceiveStatus::Closing);
            assert_eq!(endpoint.control().status(), ReceiveStatus::Closing);
            assert!(!endpoint.is_detached());
            assert!(!observed.cleaned.load(Ordering::SeqCst));
            assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
            assert!(window.upgrade().is_some());
            assert!(case.display_owner.upgrade().is_some());
            // The gate was released BEFORE panic_cleanup. sync_pipe removed
            // Generic and the text finalizer released only this Incoming.
            // Dead FD/empty Incoming do not certify SDK cleanup or native join.
            assert!(fd.upgrade().is_none());
            assert!(case.lifecycle.incoming().is_none());
            assert!(matches!(
                case.pool.reserve_owner(
                    case.key(0).display,
                    Arc::new(()),
                    control(),
                    Duration::from_secs(5),
                ),
                Err(ReceiveError::Busy)
            ));
            assert!(matches!(
                case.pool.reserve_legacy(case.key(0)),
                Err(ReceiveError::Busy)
            ));
        } else {
            wait(
                || {},
                || {
                    case.lifecycle.status() == ReceiveStatus::Closed(None)
                        && case.closed_wakes.load(Ordering::SeqCst) == 1
                },
            );
            assert!(observed.cleaned.load(Ordering::SeqCst));
            assert!(!observed.cleanup_failed.load(Ordering::SeqCst));
            assert!(endpoint.is_detached());
            assert!(fd.upgrade().is_none());
            assert!(case.lifecycle.incoming().is_none());
            assert!(window.upgrade().is_none());
            assert!(case.display_owner.upgrade().is_none());
        }
        assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
        assert_eq!(poll_text(&case, &request), None);
        assert!(endpoint.control().poll().is_empty());
        // The server was paused, so even a locally certified close is not yet
        // evidence that it processed the queued native Release requests.
        assert_eq!(case.fixture().server.released_devices, 0);
        assert_eq!(case.fixture().server.released_seats, 0);
        assert_eq!(case.fixture().server.released_keyboards, 0);

        if uncertain {
            // Resume only now. A genuine host sync is not a repair/cleanup ACK
            // for the retained guest whose cleanup hook failed before entering
            // GuestStartup::cleanup. Keyboard/device/seat remain unreleased.
            host_sync(case.fixture_mut());
            assert_eq!(case.fixture().server.released_devices, 0);
            assert_eq!(case.fixture().server.released_seats, 0);
            assert_eq!(case.fixture().server.released_keyboards, 0);
            assert_eq!(case.fixture().server.keyboards.len(), 1);
            assert!(case
                .fixture()
                .server
                .device_resources
                .iter()
                .all(|device| device.is_alive()));
            assert_eq!(case.lifecycle.status(), ReceiveStatus::Closing);
            assert!(!endpoint.is_detached());
            assert!(window.upgrade().is_some());
            assert!(case.display_owner.upgrade().is_some());
            assert!(fd.upgrade().is_none());
            assert!(case.lifecycle.incoming().is_none());
            assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
            assert!(!observed.cleaned.load(Ordering::SeqCst));
            assert!(matches!(
                case.pool.reserve_legacy(case.key(0)),
                Err(ReceiveError::Busy)
            ));
            assert!(matches!(
                case.pool.reserve_owner(
                    case.key(0).display,
                    Arc::new(()),
                    control(),
                    Duration::from_secs(5),
                ),
                Err(ReceiveError::Busy)
            ));
            // LiveOwnerCase's existing Drop retains the real fixture while
            // cleanup remains uncertain; this test never synthesizes Closed.
        } else {
            // Processes real Release requests, checks their exact counts, and
            // performs the existing host_sync only after local cleanup/join.
            text_host_still_works(&mut case);
            assert_eq!(case.fixture().server.released_keyboards, 1);
            assert!(case.fixture().server.keyboards.is_empty());
            assert_eq!(case.lifecycle.status(), ReceiveStatus::Closed(None));
        }
        assert_eq!(case.fixture().text_receive_count(key, UTF8), 1);
        assert_no_dnd_requests(&case, key);
        assert_eq!(poll_text(&case, &request), None);
        drop(writer);
    }
}
