//! An unserved physical Clipboard callback keeps its original clock after a cut.
//! Wire only as a child of clipboard_runtime_tests. No new observer or gate.

use super::*;

#[test]
fn a_tombstoned_clipboard_callback_times_out_without_renewal_while_the_host_is_paused() {
    let budget = Duration::from_secs(2);
    let observed = testing::Observations::default();
    let mut case = LiveOwnerCase::with_text(
        testing::Hooks {
            observed: observed.clone(),
            ..Default::default()
        },
        ReceiveLimits {
            startup_timeout: budget,
            ..ReceiveLimits::default()
        },
        TextClipboardLimits::default(),
    );
    drive_text(&mut case, |case| {
        case.lifecycle.status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
    });
    let (endpoint, window) = case.register(0, || {});
    let control = endpoint.control();
    drive_text(&mut case, |_| {
        let header = control.header();
        header.status == ReceiveStatus::Ready && header.armed
    });
    let epoch = control.header().epoch;
    let key = 1301;
    case.fixture().text_selection(0, key, true, true);
    case.fixture_mut().display.flush_clients().unwrap();
    case.fixture().host_read();
    wait(
        || case.fixture().host_read(),
        || {
            case.clipboard()
                .observed_seats()
                .into_iter()
                .flatten()
                .any(|seat| seat.focus.is_none() && seat.selection.is_some())
        },
    );
    let selection = case
        .clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| seat.focus.is_none() && seat.selection.is_some())
        .unwrap()
        .selection
        .unwrap()
        .id;
    // Finish initialization before Enter. Ready+armed consumed the initial Arm;
    // these later completed pumps also pass State's first_ready assignments.
    let before_enter = observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || {},
        || {
            observed
                .completed_pumps
                .load(Ordering::SeqCst)
                .saturating_sub(before_enter)
                >= 2
        },
    );
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert!(control.header().armed);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(case.lifecycle.incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert!(window.upgrade().is_some());
    assert!(case.display_owner.upgrade().is_some());
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);

    let t0 = Instant::now();
    case.fixture_mut().keyboard_enter(0, 0);
    case.fixture_mut().display.flush_clients().unwrap();
    case.fixture().host_read();
    wait(
        || case.fixture().host_read(),
        || {
            focused(&case, Some(endpoint.id()))
                .is_some_and(|seat| seat.selection.is_some_and(|actual| actual.id == selection))
        },
    );
    // Stop ALL server dispatch and host reads now. No server pass has occurred
    // since Enter, so its new physical Sync cannot receive a Done. The first
    // completion may belong to Enter's current pump; a second includes a later
    // whole pump and its normal connection.flush. This is an emission bracket
    // plus source review, not an observation of the private deadline itself.
    let after_enter = observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || {},
        || {
            observed
                .completed_pumps
                .load(Ordering::SeqCst)
                .saturating_sub(after_enter)
                >= 2
        },
    );
    let t1 = Instant::now();
    assert!(
        t1.duration_since(t0) < budget / 8,
        "need a narrow real emission bracket"
    );
    let entered = focused(&case, Some(endpoint.id())).unwrap();
    assert!(!entered.focus.unwrap().sealed);
    assert!(control.header().text_sealed.is_none());
    assert!(control.header().armed);
    assert_eq!(control.header().epoch, epoch);
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(case.lifecycle.incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);

    // Time a real protocol-callback budget, never infer focus from a sleep.
    // The held sender makes each bounded recv_timeout expire without input.
    let (_time_sender, time_receiver) = mpsc::channel::<()>();
    let cut_target = t1.checked_add(budget / 2).unwrap();
    while let Some(remaining) = cut_target.checked_duration_since(Instant::now()) {
        assert_eq!(
            time_receiver.recv_timeout(remaining),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
    }
    let c0 = Instant::now();
    let new_epoch = control.invalidate_pending().unwrap();
    assert_ne!(new_epoch, epoch);
    case.ping.ping();
    // Sample AFTER the cut; two more completions include a pump started after
    // it. State refreshes the endpoint epoch/cut before incrementing this count.
    let after_cut = observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || {},
        || {
            observed
                .completed_pumps
                .load(Ordering::SeqCst)
                .saturating_sub(after_cut)
                >= 2
                || matches!(
                    case.lifecycle.status(),
                    ReceiveStatus::Closing | ReceiveStatus::Closed(_)
                )
        },
    );
    assert!(
        Instant::now() < t0.checked_add(budget).unwrap(),
        "cut must be observed before the earliest original expiry"
    );
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert_eq!(control.header().epoch, new_epoch);
    assert!(!control.header().armed);
    assert!(control.header().text_sealed.is_none());
    let cut = focused(&case, Some(endpoint.id())).unwrap();
    assert!(cut.seat == entered.seat);
    assert!(cut.keyboard_generation == entered.keyboard_generation);
    assert!(cut.focus.unwrap().generation == entered.focus.unwrap().generation);
    assert!(cut.selection.unwrap().id == selection);
    assert!(!cut.focus.unwrap().sealed);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(case.lifecycle.incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);
    assert!(control.poll().is_empty());
    assert!(window.upgrade().is_some());
    assert!(case.display_owner.upgrade().is_some());

    // Original emission lies in [t0,t1]; its expiry is at most t1+budget.
    // A fresh renewal from this cut cannot expire before c0+budget. Keep a
    // genuine observation window between those bounds without timing teardown.
    let original_latest = t1.checked_add(budget).unwrap();
    let decision_limit = original_latest.checked_add(budget / 4).unwrap();
    let renewed_earliest = c0.checked_add(budget).unwrap();
    assert!(decision_limit < renewed_earliest);
    let mut decided = None;
    wait(
        || {},
        || {
            let (status, cause) = {
                let header = case.lifecycle.header();
                (header.status, header.close_cause)
            };
            let seen = Instant::now();
            if matches!(status, ReceiveStatus::Closing | ReceiveStatus::Closed(_)) {
                assert_eq!(
                    cause,
                    Some(ReceiveError::Timeout),
                    "the root must decide Timeout"
                );
                decided = Some(seen);
                true
            } else {
                assert_eq!(status, ReceiveStatus::Ready);
                seen >= decision_limit
            }
        },
    );
    let decided = decided
        .expect("the original physical callback clock must fail the owner before a renewed budget");
    assert!(decided < decision_limit);
    assert!(decided < renewed_earliest);
    // No request, DnD gesture, endpoint intake or initial Arm was outstanding.
    // Root startup and endpoint first-ready clocks were disabled before Enter;
    // after the cut any newly emitted Arm would have a later budget. Review of
    // pending preservation/check/next_deadline attributes this root decision to
    // the unserved Clipboard callback, not to another active operation timer.
    assert!(case.lifecycle.incoming().is_none());
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);

    // Only now wait separately for normal cleanup and actual join. This WAIT
    // is a private-fixture progress bound, not a promised SDK teardown SLA.
    wait(
        || {},
        || {
            case.lifecycle.status() == ReceiveStatus::Closed(Some(ReceiveError::Timeout))
                && case.closed_wakes.load(Ordering::SeqCst) == 1
        },
    );
    assert!(observed.cleaned.load(Ordering::SeqCst));
    assert!(!observed.cleanup_failed.load(Ordering::SeqCst));
    assert!(endpoint.is_detached());
    assert!(window.upgrade().is_none());
    assert!(case.display_owner.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert!(control.poll().is_empty());
    // Neither server nor host read has run since Enter was observed. Native
    // Release requests are still unprocessed; Closed certifies local teardown.
    assert_eq!(case.fixture().server.released_devices, 0);
    assert_eq!(case.fixture().server.released_seats, 0);
    assert_eq!(case.fixture().server.released_keyboards, 0);
    text_host_still_works(&mut case);
    assert_eq!(case.fixture().server.released_keyboards, 1);
    assert!(case.fixture().server.keyboards.is_empty());
    assert_eq!(
        case.lifecycle.status(),
        ReceiveStatus::Closed(Some(ReceiveError::Timeout))
    );
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
    assert!(observed.watched_pipe.lock().unwrap().is_none());
    assert_no_dnd_requests(&case, key);
}
