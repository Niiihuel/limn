//! Body ownership tests. No fixture installs a native pipe or runtime watcher.

use super::super::{TextRequestControl, TextRequestId};
use super::*;

fn claim_text(
    case: &mut Case,
    endpoint: &ReceiveControl,
    registration: SurfaceId,
    now: Instant,
) -> TextRequestControl {
    let request = case.clipboard.reserve(endpoint, registration, now).unwrap();
    request.acknowledge(now).unwrap();
    let current = case.worker.current_request().unwrap();
    assert!(current.0 == request.id() && current.1 == registration);
    let claim = case
        .worker
        .claim_receive(request.id(), endpoint, registration, now)
        .unwrap();
    assert!(claim.request == request.id());
    request
}

fn staged_is(worker: &ClipboardWorker, request: TextRequestId, expected: &str) {
    assert!(worker
        .staged
        .as_ref()
        .is_some_and(|(id, text)| *id == request && text == expected));
    assert!(worker.body.lock().unwrap().body.is_none());
}

fn body_is(worker: &ClipboardWorker, request: TextRequestId, expected: &str) {
    assert!(worker.staged.is_none());
    assert!(worker
        .body
        .lock()
        .unwrap()
        .body
        .as_ref()
        .is_some_and(|(id, text)| *id == request && text == expected));
}

fn empty_body(worker: &ClipboardWorker) {
    assert!(worker.staged.is_none());
    assert!(worker.body.lock().unwrap().body.is_none());
}

fn b_is_busy(case: &Case) {
    assert!(matches!(
        case.clipboard
            .reserve(&case.b.control(), case.b.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
}

// Only this fixture's never-installed-watcher path authorizes this finalizer.
// B/C separately has to prove the real Generic/OwnedFd retirement boundary.
fn finish_without_watcher(
    case: &mut Case,
    request: TextRequestId,
    endpoint: &ReceiveControl,
    registration: SurfaceId,
    now: Instant,
) -> Result<bool, ClipboardError> {
    case.worker
        .finish_after_watcher_removed(request, Some(endpoint), registration, now)
}

#[test]
fn clipboard_a2_stage_is_private_and_publication_keeps_raw_single_use_text() {
    bounded(|| {
        for raw in ["primero\r\nsegundo 😀", "", "éé"] {
            let limits = TextClipboardLimits {
                max_text_bytes: raw.len().max(1),
                ..TextClipboardLimits::default()
            };
            let mut case = Case::new(limits);
            let a = case.a.control();
            let registration = case.a.id();
            let anchor = Instant::now() + Duration::from_secs(10);
            let request = claim_text(&mut case, &a, registration, anchor);
            let read_deadline = anchor + limits.read_timeout;
            assert_eq!(case.worker.next_deadline(), Some(read_deadline));
            case.worker
                .stage_validated(
                    request.id(),
                    &a,
                    registration,
                    raw.to_owned(),
                    anchor + Duration::from_millis(100),
                )
                .unwrap();
            staged_is(&case.worker, request.id(), raw);
            assert_eq!(request.poll(anchor + Duration::from_secs(1)), None);
            assert_eq!(case.worker.next_deadline(), Some(read_deadline));
            case.incoming_is(IncomingLease::Text(request.id()));
            b_is_busy(&case);
            assert_eq!(
                case.worker.stage_validated(
                    request.id(),
                    &a,
                    registration,
                    "replacement".to_owned(),
                    anchor + Duration::from_secs(1),
                ),
                Err(ClipboardError::InvalidPhase)
            );
            staged_is(&case.worker, request.id(), raw);
            let published = anchor + Duration::from_secs(2);
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), &a, registration, published),
                Ok(true)
            );
            body_is(&case.worker, request.id(), raw);
            case.incoming_is(IncomingLease::Text(request.id()));
            b_is_busy(&case);
            let expires = published + limits.completion_ttl;
            assert_eq!(case.worker.next_deadline(), Some(expires));
            assert_eq!(
                finish_without_watcher(
                    &mut case,
                    request.id(),
                    &a,
                    registration,
                    published + Duration::from_millis(100),
                ),
                Ok(false)
            );
            assert_eq!(case.worker.next_deadline(), Some(expires));
            assert!(!case.worker.fail_receive(request.id(), ClipboardError::Io));
            assert_eq!(request.poll(published), Some(Ok(raw.to_owned())));
            assert_eq!(request.poll(published), None);
            assert!(case.root.incoming().is_none());
            assert!(case.worker.current_request().is_none());
            assert_eq!(case.worker.next_deadline(), None);
            empty_body(&case.worker);
            request.cancel();
            assert!(!case
                .worker
                .fail_receive(request.id(), ClipboardError::InvalidText));
            assert_eq!(request.poll(published), None);
            let b = case.b.control();
            let b_sid = case.b.id();
            let next = case.clipboard.reserve(&b, b_sid, published).unwrap();
            next.cancel();
            assert_eq!(
                finish_without_watcher(&mut case, next.id(), &b, b_sid, published),
                Ok(true)
            );
        }

        // The boundary is bytes, including multibyte UTF-8, rather than chars.
        let mut case = Case::new(TextClipboardLimits {
            max_text_bytes: 4,
            ..TextClipboardLimits::default()
        });
        let a = case.a.control();
        let registration = case.a.id();
        let request = claim_text(&mut case, &a, registration, Instant::now());
        assert_eq!(
            case.worker.stage_validated(
                request.id(),
                &a,
                registration,
                "ééa".to_owned(),
                Instant::now(),
            ),
            Err(ClipboardError::TooLarge)
        );
        empty_body(&case.worker);
        case.incoming_is(IncomingLease::Text(request.id()));
        assert_eq!(
            request.poll(Instant::now()),
            Some(Err(ClipboardError::TooLarge))
        );
        assert_eq!(request.poll(Instant::now()), None);
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
            Ok(true)
        );
    });
}

#[test]
fn clipboard_a2_publication_revalidates_authority_and_original_read_deadline() {
    bounded(|| {
        for at_stage in [false, true] {
            let limits = TextClipboardLimits {
                read_timeout: Duration::from_millis(60),
                ..TextClipboardLimits::default()
            };
            let mut case = Case::new(limits);
            let a = case.a.control();
            let registration = case.a.id();
            let old_now = Instant::now();
            let request = claim_text(&mut case, &a, registration, old_now);
            if !at_stage {
                case.worker
                    .stage_validated(request.id(), &a, registration, "staged".to_owned(), old_now)
                    .unwrap();
            }
            let expires = case.worker.next_deadline().unwrap();
            let authority = case.root.clone();
            let guard = authority.header();
            let (attempting, attempted) = mpsc::channel();
            let (finished, result) = mpsc::channel();
            let probe = thread::spawn(move || {
                let _ = attempting.send(());
                let staged = if at_stage {
                    case.worker.stage_validated(
                        request.id(),
                        &a,
                        registration,
                        "too late".to_owned(),
                        old_now,
                    )
                } else {
                    Ok(())
                };
                let outcome =
                    finish_without_watcher(&mut case, request.id(), &a, registration, old_now);
                let _ = finished.send((case, request, staged, outcome));
            });
            let entered = attempted.recv_timeout(WAIT);
            wait_until(expires);
            drop(guard);
            assert_eq!(entered, Ok(()));
            let (case, request, staged, outcome) = result.recv_timeout(WAIT).unwrap();
            probe.join().unwrap();
            assert_eq!(
                staged,
                if at_stage {
                    Err(ClipboardError::Timeout)
                } else {
                    Ok(())
                }
            );
            assert_eq!(outcome, Ok(true));
            assert_eq!(request.poll(old_now), Some(Err(ClipboardError::Timeout)));
            assert_eq!(request.poll(old_now), None);
            empty_body(&case.worker);
            assert!(case.root.incoming().is_none());
        }

        for cut in 0..7 {
            let mut case = Case::new(TextClipboardLimits::default());
            let a = case.a.control();
            let registration = case.a.id();
            let request = claim_text(&mut case, &a, registration, Instant::now());
            case.worker
                .stage_validated(
                    request.id(),
                    &a,
                    registration,
                    "private".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            let mut seat = case.seat_a;
            match cut {
                0 => {
                    a.invalidate_pending().unwrap();
                }
                1 => {
                    case.root.close().unwrap();
                }
                2..=4 => {
                    match cut {
                        2 => seat.selection = next_clipboard_stamp().unwrap(),
                        3 => seat.keyboard = next_clipboard_stamp().unwrap(),
                        _ => seat.focus = next_clipboard_stamp().unwrap(),
                    }
                    case.worker
                        .observe_seat(seat.id, Some(seat.snapshot(registration, true)))
                        .unwrap();
                }
                5 => case.worker.observe_seat(seat.id, None).unwrap(),
                _ => {} // The missing State endpoint is supplied only at finish.
            }
            let endpoint = (cut != 6).then_some(&a);
            assert_eq!(
                case.worker.finish_after_watcher_removed(
                    request.id(),
                    endpoint,
                    registration,
                    Instant::now(),
                ),
                Ok(true)
            );
            let error = if cut == 1 {
                ClipboardError::Closed
            } else {
                ClipboardError::Cancelled
            };
            assert_eq!(request.poll(Instant::now()), Some(Err(error)));
            assert_eq!(request.poll(Instant::now()), None);
            empty_body(&case.worker);
            assert!(case.root.incoming().is_none());
        }

        let mut case = Case::new(TextClipboardLimits::default());
        let a = case.a.control();
        let b = case.b.control();
        let b_sid = case.b.id();
        let registration = case.a.id();
        let request = claim_text(&mut case, &a, registration, Instant::now());
        case.worker
            .stage_validated(
                request.id(),
                &a,
                registration,
                "exact A".to_owned(),
                Instant::now(),
            )
            .unwrap();
        for (endpoint, sid) in [(&b, registration), (&a, b_sid), (&b, b_sid)] {
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), endpoint, sid, Instant::now()),
                Err(ClipboardError::StaleRequest)
            );
            staged_is(&case.worker, request.id(), "exact A");
            case.incoming_is(IncomingLease::Text(request.id()));
            assert_eq!(request.poll(Instant::now()), None);
        }
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
            Ok(true)
        );
        assert_eq!(request.poll(Instant::now()), Some(Ok("exact A".to_owned())));
    });
}

#[test]
fn clipboard_a2_poll_uses_fresh_ttl_after_root_and_body_mutex_contention() {
    bounded(|| {
        for hold_body in [false, true] {
            let mut case = Case::new(TextClipboardLimits {
                completion_ttl: Duration::from_millis(60),
                ..TextClipboardLimits::default()
            });
            let a = case.a.control();
            let registration = case.a.id();
            let request = claim_text(&mut case, &a, registration, Instant::now());
            case.worker
                .stage_validated(
                    request.id(),
                    &a,
                    registration,
                    "not for UI".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
                Ok(true)
            );
            let expires = case.worker.next_deadline().unwrap();
            let old_now = Instant::now();
            let root_guard = (!hold_body).then(|| case.root.header());
            let body_guard = hold_body.then(|| case.worker.body.lock().unwrap());
            let (attempting, attempted) = mpsc::channel();
            let (finished, result) = mpsc::channel();
            let probe = thread::spawn(move || {
                let _ = attempting.send(());
                let polled = request.poll(old_now);
                let _ = finished.send((request, polled));
            });
            let entered = attempted.recv_timeout(WAIT);
            wait_until(expires);
            drop(body_guard);
            drop(root_guard);
            assert_eq!(entered, Ok(()));
            let (request, polled) = result.recv_timeout(WAIT).unwrap();
            probe.join().unwrap();
            assert_eq!(polled, Some(Err(ClipboardError::Timeout)));
            assert_eq!(request.poll(old_now), None);
            body_is(&case.worker, request.id(), "not for UI");
            case.incoming_is(IncomingLease::Text(request.id()));
            b_is_busy(&case);
            assert!(!case.worker.retire_after_cleanup(request.id()));
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
                Ok(true)
            );
            empty_body(&case.worker);
            assert!(case.root.incoming().is_none());
        }

        for cut in 0..7 {
            let mut case = Case::new(TextClipboardLimits::default());
            let a = case.a.control();
            let registration = case.a.id();
            let request = claim_text(&mut case, &a, registration, Instant::now());
            case.worker
                .stage_validated(
                    request.id(),
                    &a,
                    registration,
                    "cut after publication".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
                Ok(true)
            );
            let mut seat = case.seat_a;
            match cut {
                0 => request.cancel(),
                1 => {
                    case.root.close().unwrap();
                }
                2 => {
                    a.invalidate_pending().unwrap();
                }
                3..=5 => {
                    match cut {
                        3 => seat.selection = next_clipboard_stamp().unwrap(),
                        4 => seat.keyboard = next_clipboard_stamp().unwrap(),
                        _ => seat.focus = next_clipboard_stamp().unwrap(),
                    }
                    case.worker
                        .observe_seat(seat.id, Some(seat.snapshot(registration, true)))
                        .unwrap();
                }
                _ => case.worker.observe_seat(seat.id, None).unwrap(),
            }
            let error = if cut == 1 {
                ClipboardError::Closed
            } else {
                ClipboardError::Cancelled
            };
            assert_eq!(request.poll(Instant::now()), Some(Err(error)));
            assert_eq!(request.poll(Instant::now()), None);
            body_is(&case.worker, request.id(), "cut after publication");
            case.incoming_is(IncomingLease::Text(request.id()));
            assert!(!case.worker.retire_after_cleanup(request.id()));
            assert_eq!(
                finish_without_watcher(&mut case, request.id(), &a, registration, Instant::now()),
                Ok(true)
            );
            empty_body(&case.worker);
            assert!(case.root.incoming().is_none());
        }

        // Scheduler expiry also returns the slot when the UI never polls A.
        let mut case = Case::new(TextClipboardLimits::default());
        let a = case.a.control();
        let b = case.b.control();
        let b_sid = case.b.id();
        let registration = case.a.id();
        let anchor = Instant::now() + Duration::from_secs(10);
        let request = claim_text(&mut case, &a, registration, anchor);
        case.worker
            .stage_validated(
                request.id(),
                &a,
                registration,
                "unpolled".to_owned(),
                anchor,
            )
            .unwrap();
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, anchor),
            Ok(true)
        );
        let expires = case.worker.next_deadline().unwrap();
        case.worker.reconcile(expires);
        assert_eq!(case.worker.next_deadline(), None);
        body_is(&case.worker, request.id(), "unpolled");
        assert!(!case.worker.retire_after_cleanup(request.id()));
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, expires),
            Ok(true)
        );
        let replacement = case.clipboard.reserve(&b, b_sid, expires).unwrap();
        assert_eq!(request.poll(anchor), Some(Err(ClipboardError::Timeout)));
        assert_eq!(request.poll(anchor), None);
        case.incoming_is(IncomingLease::Text(replacement.id()));
        replacement.cancel();
        assert_eq!(
            finish_without_watcher(&mut case, replacement.id(), &b, b_sid, expires),
            Ok(true)
        );

        // Exact >= expiry is tested independently of scheduler reconciliation.
        let request = claim_text(&mut case, &a, registration, anchor);
        case.worker
            .stage_validated(request.id(), &a, registration, "edge".to_owned(), anchor)
            .unwrap();
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, anchor),
            Ok(true)
        );
        let expires = case.worker.next_deadline().unwrap();
        assert_eq!(request.poll(expires), Some(Err(ClipboardError::Timeout)));
        body_is(&case.worker, request.id(), "edge");
        assert_eq!(
            finish_without_watcher(&mut case, request.id(), &a, registration, expires),
            Ok(true)
        );
    });
}

#[test]
fn clipboard_a2_terminal_errors_and_old_handles_cannot_take_a_reused_slot() {
    bounded(|| {
        for error in [
            ClipboardError::Io,
            ClipboardError::TooLarge,
            ClipboardError::InvalidText,
            ClipboardError::Timeout,
        ] {
            let mut case = Case::new(TextClipboardLimits::default());
            let a = case.a.control();
            let b = case.b.control();
            let a_sid = case.a.id();
            let b_sid = case.b.id();
            let old = case.clipboard.reserve(&a, a_sid, Instant::now()).unwrap();
            assert!(!case.worker.fail_receive(old.id(), error));
            old.acknowledge(Instant::now()).unwrap();
            assert!(!case.worker.fail_receive(old.id(), error));
            case.worker
                .claim_receive(old.id(), &a, a_sid, Instant::now())
                .unwrap();
            case.worker
                .stage_validated(
                    old.id(),
                    &a,
                    a_sid,
                    "abandoned A".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            for invalid in [
                ClipboardError::Unavailable,
                ClipboardError::Busy,
                ClipboardError::ResourceLimit,
                ClipboardError::InvalidLimits,
                ClipboardError::DeadlineOverflow,
                ClipboardError::NoSelection,
                ClipboardError::AmbiguousSeat,
                ClipboardError::StaleRequest,
                ClipboardError::InvalidPhase,
                ClipboardError::Cancelled,
                ClipboardError::Closed,
            ] {
                assert!(!case.worker.fail_receive(old.id(), invalid));
                staged_is(&case.worker, old.id(), "abandoned A");
                assert_eq!(old.poll(Instant::now()), None);
            }
            assert!(case.worker.fail_receive(old.id(), error));
            assert!(!case.worker.fail_receive(old.id(), ClipboardError::Io));
            old.cancel();
            assert!(!case.worker.retire_after_cleanup(old.id()));
            staged_is(&case.worker, old.id(), "abandoned A");
            b_is_busy(&case);
            assert_eq!(
                finish_without_watcher(&mut case, old.id(), &a, a_sid, Instant::now()),
                Ok(true)
            );
            empty_body(&case.worker);

            let slot = Arc::downgrade(&case.worker.body);
            let request_b = claim_text(&mut case, &b, b_sid, Instant::now());
            case.worker
                .stage_validated(
                    request_b.id(),
                    &b,
                    b_sid,
                    "only B".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(
                finish_without_watcher(&mut case, request_b.id(), &b, b_sid, Instant::now()),
                Ok(true)
            );
            assert!(std::sync::Weak::ptr_eq(
                &slot,
                &Arc::downgrade(&case.worker.body)
            ));
            body_is(&case.worker, request_b.id(), "only B");
            assert_eq!(old.poll(Instant::now()), Some(Err(error)));
            assert_eq!(old.poll(Instant::now()), None);
            old.cancel();
            assert!(!case.worker.fail_receive(old.id(), ClipboardError::Io));
            assert!(!case.worker.retire_after_cleanup(old.id()));
            assert_eq!(
                finish_without_watcher(&mut case, old.id(), &a, a_sid, Instant::now()),
                Err(ClipboardError::StaleRequest)
            );
            drop(old);
            body_is(&case.worker, request_b.id(), "only B");
            case.incoming_is(IncomingLease::Text(request_b.id()));
            assert!(!case.worker.fail_receive(request_b.id(), ClipboardError::Io));
            assert_eq!(
                request_b.poll(Instant::now()),
                Some(Ok("only B".to_owned()))
            );
            assert!(!case
                .worker
                .fail_receive(request_b.id(), ClipboardError::InvalidText));
            assert_eq!(request_b.poll(Instant::now()), None);
        }
    });
}

#[test]
fn clipboard_a2_worker_drop_empties_the_body_before_a_temporary_ui_arc_is_last() {
    bounded(|| {
        // As in A1's lifetime probe, borrow a real pool SID but keep the metadata
        // subject separate from pool/reaper ownership, which otherwise retains it.
        let source = Case::new(TextClipboardLimits::default());
        let registration = source.a.id();
        let (delivered, delivery) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let (finished, completion) = mpsc::channel();
        let probe = thread::spawn(move || {
            let root = ReceiveControl::new(ReceiveEpoch(1)).unwrap();
            let alive = root.lifetime_observer();
            let endpoint =
                ReceiveControl::new_for_surface(ReceiveEpoch(1), root.clone(), registration)
                    .unwrap();
            ready(&root);
            ready(&endpoint);
            let (clipboard, mut worker) =
                ClipboardControl::new(root.clone(), TextClipboardLimits::default()).unwrap();
            let seat = Seat::new();
            worker
                .observe_seat(seat.id, Some(seat.snapshot(registration, true)))
                .unwrap();
            worker
                .seal_endpoint(&endpoint, registration, seat.seal(&endpoint, registration))
                .unwrap();
            let request = clipboard
                .reserve(&endpoint, registration, Instant::now())
                .unwrap();
            request.acknowledge(Instant::now()).unwrap();
            worker
                .claim_receive(request.id(), &endpoint, registration, Instant::now())
                .unwrap();
            worker
                .stage_validated(
                    request.id(),
                    &endpoint,
                    registration,
                    "worker-owned body".to_owned(),
                    Instant::now(),
                )
                .unwrap();
            assert_eq!(
                worker.finish_after_watcher_removed(
                    request.id(),
                    Some(&endpoint),
                    registration,
                    Instant::now()
                ),
                Ok(true)
            );
            let body = Arc::downgrade(&worker.body);
            assert!(delivered.send((request, body, root.clone(), alive)).is_ok());
            assert_eq!(released.recv_timeout(WAIT), Ok(()));
            // This path exercises Drop draining, not certified request cleanup.
            // It must not free Incoming or synthesize OwnerClosed.
            drop(worker);
            drop(clipboard);
            drop(endpoint);
            drop(root);
            let _ = finished.send(());
        });
        let (request, weak_body, root, alive) = delivery.recv_timeout(WAIT).unwrap();
        let id = request.id();
        let temporary_ui_arc = weak_body.upgrade().unwrap();
        assert!(temporary_ui_arc
            .lock()
            .unwrap()
            .body
            .as_ref()
            .is_some_and(|(current, text)| { *current == id && text == "worker-owned body" }));
        assert_eq!(root.status(), ReceiveStatus::Ready);
        release.send(()).unwrap();
        let completed = completion.recv_timeout(WAIT);
        if !matches!(completed, Err(mpsc::RecvTimeoutError::Timeout)) {
            probe.join().unwrap();
        }
        assert_eq!(completed, Ok(()));
        assert!(temporary_ui_arc.lock().unwrap().body.is_none());
        assert!(root.incoming() == Some(IncomingLease::Text(id)));
        assert_eq!(root.status(), ReceiveStatus::Ready);
        assert!(alive());
        request.cancel();
        assert_eq!(
            request.poll(Instant::now()),
            Some(Err(ClipboardError::Cancelled))
        );
        assert_eq!(request.poll(Instant::now()), None);
        drop(request);
        drop(root);
        assert!(
            !alive(),
            "Root/Core must not cycle through the body slot or request"
        );
        drop(temporary_ui_arc);
        assert!(weak_body.upgrade().is_none());
        // Slot lifetime and an executed worker Drop path are observed here.
        // This does not instrument String's allocator or prove its drop thread.
    });
}
