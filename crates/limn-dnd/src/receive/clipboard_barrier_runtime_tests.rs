//! Invalidation cuts a resolved clipboard seal and requires a fresh native fence.
//! This does not claim coverage of an old physical callback still in flight.

use super::*;

#[test]
fn invalidate_cuts_the_resolved_seal_and_open_reader_until_a_fresh_clipboard_barrier() {
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
    let (endpoint, old, source_open, old_fd) = first_request(&mut case, 851);
    let control = endpoint.control();
    drain_prefix(&mut case, &source_open, &old_fd);
    let before = focused(&case, Some(endpoint.id())).unwrap();
    assert!(before.focus.unwrap().sealed);
    let seals = observed.clipboard_seals.load(Ordering::SeqCst);
    assert!(seals > 0);
    let enter_dispatch = observed.keyboard_enter_dispatch.load(Ordering::SeqCst);
    assert!(enter_dispatch > 0);
    let old_epoch = control.header().epoch;
    let new_epoch = control.invalidate_pending().unwrap();
    assert_ne!(old_epoch, new_epoch);
    {
        let header = control.header();
        assert_eq!(header.epoch, new_epoch);
        assert!(!header.armed);
        assert!(header.text_sealed.is_none());
    }
    assert!(old_fd.upgrade().is_some());
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(old.id())));
    assert_eq!(poll_text(&case, &old), Some(Err(ClipboardError::Cancelled)));
    assert_eq!(poll_text(&case, &old), None);
    assert!(matches!(
        case.clipboard()
            .reserve(&control, endpoint.id(), Instant::now()),
        Err(ClipboardError::Unavailable)
    ));

    // Do not serve the server here: no fresh Arm/clipboard Sync may receive its
    // real Done before the disarmed and watcher-retirement assertions.
    case.ping.ping();
    retirement_entered
        .recv_timeout(WAIT)
        .expect("invalidation retires the actual text reader without EOF");
    assert!(old_fd.upgrade().is_some());
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(old.id())));
    assert!(control.incoming() == Some(IncomingLease::Text(old.id())));
    assert_eq!(poll_text(&case, &old), None);
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), seals);
    assert!(
        !focused(&case, Some(endpoint.id()))
            .unwrap()
            .focus
            .unwrap()
            .sealed
    );
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert!(!control.header().armed);
    // A final post-flush check can retire the pipe before the next drive emits
    // Arm. The current pump was already counted before this gate was entered.
    let pumps_at_retirement = observed.completed_pumps.load(Ordering::SeqCst);
    release.release();
    wait(
        || case.fixture().host_read(),
        || {
            assert_eq!(poll_text(&case, &old), None);
            let incoming = case.lifecycle.incoming();
            if incoming.is_none() {
                assert!(old_fd.upgrade().is_none());
            }
            incoming.is_none()
                && old_fd.upgrade().is_none()
                && observed.completed_pumps.load(Ordering::SeqCst) > pumps_at_retirement
        },
    );
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    assert!(matches!(
        case.clipboard()
            .reserve(&control, endpoint.id(), Instant::now()),
        Err(ClipboardError::Unavailable)
    ));
    assert!(case.lifecycle.incoming().is_none());

    // One server pass answers Arm. The host alone reads that Done. Hold every
    // subsequent server pass so the distinct fresh clipboard Sync stays pending.
    case.fixture_mut().serve();
    case.fixture().host_read();
    case.ping.ping();
    wait(
        || case.fixture().host_read(),
        || {
            let header = control.header();
            header.epoch == new_epoch && header.armed
        },
    );
    assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), seals);
    assert!(control.header().text_sealed.is_none());
    let pending = focused(&case, Some(endpoint.id())).unwrap();
    assert!(!pending.focus.unwrap().sealed);
    assert!(pending.selection.unwrap().id == before.selection.unwrap().id);
    assert!(matches!(
        case.clipboard()
            .reserve(&control, endpoint.id(), Instant::now()),
        Err(ClipboardError::Unavailable)
    ));
    assert!(case.lifecycle.incoming().is_none());
    assert_eq!(case.fixture().text_receive_count(851, UTF8), 1);

    drive_text(&mut case, |case| {
        observed.clipboard_seals.load(Ordering::SeqCst) > seals
            && focused(case, Some(endpoint.id())).is_some_and(|seat| seat.focus.unwrap().sealed)
    });
    let after = focused(&case, Some(endpoint.id())).unwrap();
    assert!(after.selection.unwrap().id == before.selection.unwrap().id);
    assert!(after.focus.unwrap().generation == before.focus.unwrap().generation);
    assert_eq!(control.header().epoch, new_epoch);
    assert_eq!(
        observed.keyboard_enter_dispatch.load(Ordering::SeqCst),
        enter_dispatch
    );
    assert!(
        control.poll().is_empty(),
        "no native DnD Enter is needed to rearm clipboard text"
    );
    let fresh = reserve_text(&mut case, &endpoint);
    assert!(fresh.id() != old.id());
    old.cancel();
    assert_eq!(poll_text(&case, &old), None);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(fresh.id())));
    assert!(control.incoming() == Some(IncomingLease::Text(fresh.id())));
    fresh.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(&mut case, |case| {
        case.fixture().text_receive_count(851, UTF8) == 2
    });
    let fresh_fd = watched_text(&observed, &fresh);
    assert!(!Weak::ptr_eq(&old_fd, &fresh_fd));
    let writer = case.fixture_mut().take_writer(851);
    write_chunk(&writer, "barrera nueva á😀\r\n".as_bytes());
    drop(writer);
    let mut outcome = None;
    drive_text(&mut case, |case| {
        outcome = poll_text(case, &fresh);
        if matches!(outcome, Some(Ok(_))) {
            assert!(fresh_fd.upgrade().is_none());
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok("barrera nueva á😀\r\n".to_owned())));
    assert_eq!(poll_text(&case, &fresh), None);
    assert_eq!(poll_text(&case, &old), None);
    assert!(case.lifecycle.incoming().is_none());
    assert!(old_fd.upgrade().is_none());
    // The invalidated source remains open through the whole recovered transfer.
    drop(source_open);
    assert_no_dnd_requests(&case, 851);
    assert_eq!(
        observed.keyboard_enter_dispatch.load(Ordering::SeqCst),
        enter_dispatch
    );
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
    assert_eq!(control.status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    case.close();
    assert!(endpoint.is_detached());
    text_host_still_works(&mut case);
}
