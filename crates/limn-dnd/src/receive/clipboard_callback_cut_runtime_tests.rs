//! Native callbacks already in flight cannot certify a later clipboard context.
//! Original tombstone deadline expiry with the host reader paused is separate.

use super::*;

fn pump_without_serving(case: &LiveOwnerCase) {
    // The first observed completion might belong to a pump already in progress
    // when the caller changed metadata. Two completions include a subsequent one.
    let before = case.observed.completed_pumps.load(Ordering::SeqCst);
    wait(
        || case.fixture().host_read(),
        || {
            case.observed
                .completed_pumps
                .load(Ordering::SeqCst)
                .saturating_sub(before)
                >= 2
                || matches!(case.lifecycle.status(), ReceiveStatus::Closed(_))
        },
    );
    assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
}

fn answer_one_prefix(case: &mut LiveOwnerCase) {
    let mut queue = case.fixture().host.new_event_queue::<HostSync>();
    let mut marker = HostSync::default();
    case.fixture().host.display().sync(&queue.handle(), ());
    case.fixture().host.flush().unwrap();
    // The guest's preceding callback is already flushed. A later guest callback
    // cannot receive Done until another explicit server pass below.
    case.fixture_mut().serve();
    let deadline = Instant::now() + WAIT;
    loop {
        case.fixture().host_read();
        queue.dispatch_pending(&mut marker).unwrap();
        if marker.0 {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the real native prefix marker did not arrive"
        );
        thread::yield_now();
    }
    pump_without_serving(case);
}

fn answer_fresh_fence(case: &mut LiveOwnerCase, seals_before: usize) {
    // One seat/endpoint can require Arm followed by Clipboard. Answer at most
    // those two actual prefixes, then assert the outcome; never wait for a seal
    // that would hide a wrong old-Done decision behind a generic progress timeout.
    for _ in 0..2 {
        answer_one_prefix(case);
        if case.observed.clipboard_seals.load(Ordering::SeqCst) > seals_before {
            break;
        }
    }
    assert_eq!(
        case.observed.clipboard_seals.load(Ordering::SeqCst),
        seals_before + 1
    );
}

#[derive(Clone, Copy, Debug)]
enum Cut {
    Intake,
    Epoch,
    Sid,
}

#[test]
fn old_clipboard_done_cannot_seal_after_intake_epoch_or_sid_change() {
    for cut in [Cut::Intake, Cut::Epoch, Cut::Sid] {
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
        let mut endpoint = if matches!(cut, Cut::Intake) {
            None
        } else {
            let (endpoint, _) = case.register(0, || {});
            let control = endpoint.control();
            drive_text(&mut case, |_| {
                let header = control.header();
                header.status == ReceiveStatus::Ready && header.armed
            });
            Some(endpoint)
        };
        case.fixture().text_selection(0, 1101, true, true);
        case.fixture_mut().keyboard_enter(0, 0);
        case.fixture_mut().display.flush_clients().unwrap();
        case.fixture().host_read();
        let registration = endpoint.as_ref().map(|endpoint| endpoint.id());
        wait(
            || case.fixture().host_read(),
            || focused(&case, registration).is_some_and(|seat| seat.selection.is_some()),
        );
        pump_without_serving(&case);
        let before = focused(&case, registration).unwrap();
        assert!(!before.focus.unwrap().sealed);
        assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
        assert!(case.lifecycle.incoming().is_none());
        assert_eq!(case.fixture().text_receive_count(1101, UTF8), 0);
        let enter_dispatch = observed.keyboard_enter_dispatch.load(Ordering::SeqCst);
        assert!(enter_dispatch > 0);

        // No server pass has answered the actual Clipboard callback born above.
        let mut former = None;
        match cut {
            Cut::Intake => {
                endpoint = Some(case.register(0, || {}).0);
            }
            Cut::Epoch => {
                let control = endpoint.as_ref().unwrap().control();
                let old_epoch = control.header().epoch;
                let new_epoch = control.invalidate_pending().unwrap();
                assert_ne!(new_epoch, old_epoch);
                case.ping.ping();
            }
            Cut::Sid => {
                let old = endpoint.take().unwrap();
                old.request_retire();
                case.ping.ping();
                wait(
                    || case.fixture().host_read(),
                    || {
                        old.is_detached()
                            || matches!(case.lifecycle.status(), ReceiveStatus::Closed(_))
                    },
                );
                assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
                assert!(old.is_detached());
                let replacement = case.register(0, || {}).0;
                assert!(replacement.id() != old.id());
                former = Some(old);
                endpoint = Some(replacement);
            }
        }
        pump_without_serving(&case);
        let endpoint = endpoint.unwrap();
        let control = endpoint.control();
        let epoch = control.header().epoch;
        let cut_snapshot = focused(&case, Some(endpoint.id())).unwrap();
        assert!(cut_snapshot.seat == before.seat);
        assert!(cut_snapshot.keyboard_generation == before.keyboard_generation);
        assert!(cut_snapshot.focus.unwrap().generation == before.focus.unwrap().generation);
        assert!(cut_snapshot.selection.unwrap().id == before.selection.unwrap().id);
        assert!(!cut_snapshot.focus.unwrap().sealed);
        assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), 0);
        assert!(case.lifecycle.incoming().is_none());
        assert!(matches!(
            case.clipboard()
                .reserve(&control, endpoint.id(), Instant::now()),
            Err(ClipboardError::Unavailable)
        ));

        // Positive marker Done plus a later completed guest pump consumes the
        // old physical reply. Only then inspect whether it wrongly sealed A.
        answer_one_prefix(&mut case);
        assert_eq!(
            observed.clipboard_seals.load(Ordering::SeqCst),
            0,
            "old Done across {cut:?}"
        );
        let after_old = focused(&case, Some(endpoint.id())).unwrap();
        assert!(!after_old.focus.unwrap().sealed);
        assert!(after_old.selection.unwrap().id == before.selection.unwrap().id);
        assert_eq!(control.header().epoch, epoch);
        assert!(control.header().text_sealed.is_none());
        assert!(case.lifecycle.incoming().is_none());
        assert!(control.incoming().is_none());
        assert!(matches!(
            case.clipboard()
                .reserve(&control, endpoint.id(), Instant::now()),
            Err(ClipboardError::Unavailable)
        ));
        assert_eq!(case.fixture().text_receive_count(1101, UTF8), 0);
        assert!(observed.watched_pipe.lock().unwrap().is_none());
        if let Some(old) = former.as_ref() {
            assert!(old.is_detached());
            assert!(matches!(
                case.clipboard()
                    .reserve(&old.control(), old.id(), Instant::now()),
                Err(ClipboardError::Closed)
            ));
        }

        answer_fresh_fence(&mut case, 0);
        let after_fresh = focused(&case, Some(endpoint.id())).unwrap();
        assert!(after_fresh.focus.unwrap().sealed);
        assert!(after_fresh.selection.unwrap().id == before.selection.unwrap().id);
        assert!(after_fresh.focus.unwrap().generation == before.focus.unwrap().generation);
        assert_eq!(control.header().epoch, epoch);
        assert_eq!(
            observed.keyboard_enter_dispatch.load(Ordering::SeqCst),
            enter_dispatch
        );
        let request = case
            .clipboard()
            .reserve(&control, endpoint.id(), Instant::now())
            .unwrap();
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
        assert!(control.incoming() == Some(IncomingLease::Text(request.id())));
        request.acknowledge(Instant::now()).unwrap();
        case.ping.ping();
        drive_text(&mut case, |case| {
            case.fixture().text_receive_count(1101, UTF8) == 1
        });
        let fd = watched_text(&observed, &request);
        let writer = case.fixture_mut().take_writer(1101);
        write_chunk(&writer, "callback fresco á😀\r\n".as_bytes());
        drop(writer);
        let mut outcome = None;
        drive_text(&mut case, |case| {
            outcome = poll_text(case, &request);
            if matches!(outcome, Some(Ok(_))) {
                assert!(fd.upgrade().is_none());
            }
            outcome.is_some()
        });
        assert_eq!(outcome, Some(Ok("callback fresco á😀\r\n".to_owned())));
        assert_eq!(poll_text(&case, &request), None);
        assert!(case.lifecycle.incoming().is_none());
        assert!(fd.upgrade().is_none());
        assert_no_dnd_requests(&case, 1101);
        assert_eq!(case.fixture().text_receive_count(1101, UTF8), 1);
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
        assert_eq!(control.status(), ReceiveStatus::Ready);
        case.close();
        assert!(endpoint.is_detached());
        text_host_still_works(&mut case);
    }
}

#[test]
fn a_b_a_quarantine_done_preserves_current_selection_without_reviving_old_a() {
    for replace_selection in [false, true] {
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
        let (a, old, writer_a, old_fd) = first_request(&mut case, 1201);
        drain_prefix(&mut case, &writer_a, &old_fd);
        let a_control = a.control();
        let (b, _) = case.register(1, || {});
        let b_control = b.control();
        drive_text(&mut case, |_| b_control.status() == ReceiveStatus::Ready);
        let original = focused(&case, Some(a.id())).unwrap();
        let a_epoch = a_control.header().epoch;
        let b_epoch = b_control.header().epoch;
        let seals = observed.clipboard_seals.load(Ordering::SeqCst);
        let old_quarantine = observed
            .clipboard_quarantine_dispatch
            .load(Ordering::SeqCst);
        let old_enter = observed.keyboard_enter_dispatch.load(Ordering::SeqCst);
        case.fixture_mut().keyboard_leave(0, 0);
        case.fixture_mut().display.flush_clients().unwrap();
        case.fixture().host_read();
        case.ping.ping();
        retirement_entered
            .recv_timeout(WAIT)
            .expect("Leave retires A with its actual source still open");
        assert!(old_fd.upgrade().is_some());
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(old.id())));
        assert_eq!(poll_text(&case, &old), Some(Err(ClipboardError::Cancelled)));
        assert_eq!(poll_text(&case, &old), None);

        // Quarantine's actual callback is in flight. Return through B to A before
        // serving it. Replacing the selection is a second explicit v4 variant.
        case.fixture_mut().keyboard_enter(0, 1);
        case.fixture_mut().keyboard_leave(0, 1);
        if replace_selection {
            case.fixture().text_selection(0, 1202, true, true);
        }
        case.fixture_mut().keyboard_enter(0, 0);
        case.fixture_mut().display.flush_clients().unwrap();
        case.fixture().host_read();
        release.release();
        wait(
            || case.fixture().host_read(),
            || {
                assert_eq!(poll_text(&case, &old), None);
                let incoming = case.lifecycle.incoming();
                if incoming.is_none() {
                    assert!(old_fd.upgrade().is_none());
                }
                focused(&case, Some(a.id())).is_some_and(|seat| {
                    seat.focus.unwrap().generation != original.focus.unwrap().generation
                })
            },
        );
        pump_without_serving(&case);
        assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
        let returned = focused(&case, Some(a.id())).unwrap();
        assert!(returned.focus.unwrap().generation != original.focus.unwrap().generation);
        assert!(returned.keyboard_generation == original.keyboard_generation);
        assert!(!returned.focus.unwrap().sealed);
        assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), seals);
        assert_eq!(
            observed
                .clipboard_quarantine_dispatch
                .load(Ordering::SeqCst),
            old_quarantine
        );
        assert!(observed.keyboard_enter_dispatch.load(Ordering::SeqCst) > old_enter);
        let current_selection = returned.selection.unwrap().id;
        if replace_selection {
            assert!(current_selection != original.selection.unwrap().id);
        } else {
            assert!(current_selection == original.selection.unwrap().id);
        }
        assert!(old_fd.upgrade().is_none());
        assert!(case.lifecycle.incoming().is_none());
        assert_eq!(poll_text(&case, &old), None);
        assert_eq!(a_control.header().epoch, a_epoch);
        assert_eq!(b_control.header().epoch, b_epoch);

        answer_one_prefix(&mut case);
        // This counter is updated only after physical ObjectId+full-tag match,
        // before logical validation, so a stale Quarantine is positively seen.
        assert!(
            observed
                .clipboard_quarantine_dispatch
                .load(Ordering::SeqCst)
                > old_quarantine
        );
        assert_eq!(observed.clipboard_seals.load(Ordering::SeqCst), seals);
        let after_old = focused(&case, Some(a.id())).unwrap();
        assert!(after_old.focus.unwrap().generation == returned.focus.unwrap().generation);
        assert!(after_old.selection.unwrap().id == current_selection);
        assert!(!after_old.focus.unwrap().sealed);
        assert!(case.lifecycle.incoming().is_none());
        assert!(matches!(
            case.clipboard().reserve(&a_control, a.id(), Instant::now()),
            Err(ClipboardError::Unavailable)
        ));
        let key = if replace_selection { 1202 } else { 1201 };
        assert!(case
            .fixture()
            .server
            .requests
            .iter()
            .all(|request| !matches!(request, OfferRequest::Destroy(actual) if *actual == key)));
        assert_eq!(case.fixture().text_receive_count(1201, UTF8), 1);
        if replace_selection {
            assert_eq!(case.fixture().text_receive_count(1202, UTF8), 0);
            assert_eq!(
                case.fixture()
                    .server
                    .requests
                    .iter()
                    .filter(|request| matches!(request, OfferRequest::Destroy(1201)))
                    .count(),
                1
            );
        }

        answer_fresh_fence(&mut case, seals);
        let fresh = case
            .clipboard()
            .reserve(&a_control, a.id(), Instant::now())
            .unwrap();
        assert!(fresh.id() != old.id());
        old.cancel();
        assert_eq!(poll_text(&case, &old), None);
        assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(fresh.id())));
        assert!(a_control.incoming() == Some(IncomingLease::Text(fresh.id())));
        fresh.acknowledge(Instant::now()).unwrap();
        case.ping.ping();
        let count = if replace_selection { 1 } else { 2 };
        drive_text(&mut case, |case| {
            case.fixture().text_receive_count(key, UTF8) == count
        });
        let fresh_fd = watched_text(&observed, &fresh);
        assert!(!Weak::ptr_eq(&old_fd, &fresh_fd));
        assert!(old_fd.upgrade().is_none());
        let writer = case.fixture_mut().take_writer(key);
        write_chunk(&writer, "A vuelve con request nuevo á😀\r\n".as_bytes());
        drop(writer);
        let mut outcome = None;
        drive_text(&mut case, |case| {
            outcome = poll_text(case, &fresh);
            if matches!(outcome, Some(Ok(_))) {
                assert!(fresh_fd.upgrade().is_none());
            }
            outcome.is_some()
        });
        assert_eq!(
            outcome,
            Some(Ok("A vuelve con request nuevo á😀\r\n".to_owned()))
        );
        assert_eq!(poll_text(&case, &fresh), None);
        assert_eq!(poll_text(&case, &old), None);
        assert!(case.lifecycle.incoming().is_none());
        assert!(fresh_fd.upgrade().is_none());
        // The first source stayed open through A/B/A and the full new A transfer.
        drop(writer_a);
        assert_eq!(
            case.fixture().text_receive_count(1201, UTF8),
            if replace_selection { 1 } else { 2 }
        );
        assert_no_dnd_requests(&case, 1201);
        if replace_selection {
            assert_no_dnd_requests(&case, 1202);
        }
        assert_eq!(a_control.header().epoch, a_epoch);
        assert_eq!(b_control.header().epoch, b_epoch);
        assert_eq!(a_control.status(), ReceiveStatus::Ready);
        assert_eq!(b_control.status(), ReceiveStatus::Ready);
        assert_eq!(case.lifecycle.status(), ReceiveStatus::Ready);
        case.close();
        assert!(a.is_detached() && b.is_detached());
        text_host_still_works(&mut case);
    }
}
