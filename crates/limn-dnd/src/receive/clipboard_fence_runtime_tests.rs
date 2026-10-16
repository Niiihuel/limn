//! A real callback reaches A1 sealing, then waits across its original deadline.

use super::*;

#[test]
fn a_clipboard_fence_blocked_on_root_cannot_seal_after_its_actual_emitted_deadline() {
    let observed = testing::Observations::default();
    let (entered, seal_entered) = mpsc::sync_channel(1);
    let (release, seal_release) = mpsc::sync_channel(1);
    let mut case = LiveOwnerCase::with_text(
        testing::Hooks {
            observed: observed.clone(),
            before_clipboard_seal: Some((entered, seal_release)),
            ..Default::default()
        },
        ReceiveLimits {
            startup_timeout: Duration::from_millis(500),
            ..ReceiveLimits::default()
        },
        TextClipboardLimits::default(),
    );
    let mut release = OpenOnDrop(Some(release));
    prepare_unregistered_selection(&mut case, 301);
    let (endpoint, _) = case.register(0, || {});
    let mut actual_deadline = None;
    drive_text(&mut case, |_| match seal_entered.try_recv() {
        Ok(deadline) => {
            actual_deadline = Some(deadline);
            true
        }
        Err(mpsc::TryRecvError::Empty) => false,
        Err(mpsc::TryRecvError::Disconnected) => panic!("real clipboard seal gate was dropped"),
    });
    let deadline = actual_deadline.unwrap();
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(case.lifecycle.incoming().is_none());
    assert_eq!(case.fixture().text_receive_count(301, UTF8), 0);
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::Unavailable)
    ));
    let authority = case.lifecycle.clone();
    let guard = authority.header();
    let before = observed.clipboard_seals.load(Ordering::SeqCst);
    assert_eq!(before, 0);
    assert!(
        Instant::now() < deadline,
        "the actual callback passed its preliminary clock check"
    );
    // The real State precheck passed and this root stays locked past expiry.
    // Thread scheduling is not an arrival witness for a hypothetical extra
    // pre-lock check inside A1. Review must verify fresh time AFTER both locks.
    release.release();
    let (_held_sender, receiver) = mpsc::channel::<()>();
    assert_eq!(
        receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), before);
    assert_eq!(case.closed_wakes.load(Ordering::SeqCst), 0);
    assert!(!observed.cleaned.load(Ordering::SeqCst));
    assert_eq!(guard.status, ReceiveStatus::Ready);
    drop(guard);
    drive_text(&mut case, |case| {
        matches!(
            case.lifecycle.status(),
            ReceiveStatus::Closed(Some(ReceiveError::Timeout))
        )
    });
    assert_eq!(
        observed.clipboard_seals.load(Ordering::SeqCst),
        before,
        "no transient late seal may succeed"
    );
    assert!(observed.cleaned.load(Ordering::SeqCst));
    assert!(endpoint.is_detached());
    assert_no_dnd_requests(&case, 301);
    text_host_still_works(&mut case);
}
