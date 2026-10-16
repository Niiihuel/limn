use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use super::{
    allocate_text_request_id, next_clipboard_stamp, ClipboardControl, ClipboardStamp,
    ClipboardWorker, TextFocus, TextFocusSeal, TextMime, TextSeatSnapshot, TextSelection,
};
use crate::receive::{
    control::{IncomingLease, ReceiveControl},
    coordinator::Coordinator,
    worker_pool::{OwnerId, Reservation, SurfaceId, SurfaceKey, SurfaceLease, WorkerPool},
    OfferId, ReceiveEpoch,
};
use crate::{
    ClipboardError, ReceiveError, ReceiveEvent, ReceiveLimits, ReceiveStatus, SurfacePoint,
    TextClipboardLimits,
};

const WAIT: Duration = Duration::from_secs(3);

// A regression that holds an authority through reentry must fail within a bound.
// Never join a blocked probe after that bound, including on assertion unwind.
fn bounded(test: impl FnOnce() + Send + 'static) {
    let (done, completed) = mpsc::channel();
    let probe = thread::spawn(move || {
        test();
        let _ = done.send(());
    });
    let result = completed.recv_timeout(WAIT);
    if !matches!(result, Err(mpsc::RecvTimeoutError::Timeout)) {
        probe.join().unwrap();
    }
    assert_eq!(result, Ok(()), "metadata authority probe did not finish");
}

fn wait_until(deadline: Instant) {
    let (_held_sender, receiver) = mpsc::channel::<()>();
    assert_eq!(
        receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())),
        Err(mpsc::RecvTimeoutError::Timeout)
    );
}

fn ready(control: &ReceiveControl) {
    let mut coordinator =
        Coordinator::with_control(ReceiveLimits::default(), control.clone(), || {}).unwrap();
    let now = Instant::now();
    let sync = coordinator.next_sync(now).unwrap().unwrap();
    coordinator.sync_done(sync.epoch, now).unwrap();
    assert_eq!(control.status(), ReceiveStatus::Ready);
}

#[derive(Clone, Copy)]
struct Seat {
    id: ClipboardStamp,
    keyboard: ClipboardStamp,
    focus: ClipboardStamp,
    selection: ClipboardStamp,
}

impl Seat {
    fn new() -> Self {
        Self {
            id: next_clipboard_stamp().unwrap(),
            keyboard: next_clipboard_stamp().unwrap(),
            focus: next_clipboard_stamp().unwrap(),
            selection: next_clipboard_stamp().unwrap(),
        }
    }

    fn snapshot(self, registration: SurfaceId, utf8: bool) -> TextSeatSnapshot {
        TextSeatSnapshot {
            seat: self.id,
            keyboard_generation: self.keyboard,
            focus: Some(TextFocus {
                generation: self.focus,
                registration: Some(registration),
                sealed: true,
            }),
            selection: Some(TextSelection {
                id: self.selection,
                utf8,
                plain: true,
            }),
        }
    }

    fn seal(self, endpoint: &ReceiveControl, registration: SurfaceId) -> TextFocusSeal {
        TextFocusSeal {
            seat: self.id,
            keyboard_generation: self.keyboard,
            focus_generation: self.focus,
            registration,
            epoch: endpoint.header().epoch,
            fence_deadline: None,
        }
    }
}

struct Case {
    pool: Arc<WorkerPool>,
    owner: OwnerId,
    _reservation: Reservation,
    root: ReceiveControl,
    a: SurfaceLease,
    b: SurfaceLease,
    clipboard: Arc<ClipboardControl>,
    worker: ClipboardWorker,
    seat_a: Seat,
}

impl Case {
    fn new(limits: TextClipboardLimits) -> Self {
        let pool = WorkerPool::isolated();
        let root = ReceiveControl::new(ReceiveEpoch(1)).unwrap();
        let (owner, reservation) = pool
            .reserve_owner(1, Arc::new(()), root.clone(), Duration::from_secs(5))
            .unwrap();
        let a = pool
            .reserve_surface(
                owner,
                SurfaceKey {
                    display: 1,
                    surface: 1,
                },
                Arc::new(()),
                Box::new(|| {}),
            )
            .unwrap();
        let b = pool
            .reserve_surface(
                owner,
                SurfaceKey {
                    display: 1,
                    surface: 2,
                },
                Arc::new(()),
                Box::new(|| {}),
            )
            .unwrap();
        ready(&root);
        ready(&a.control());
        ready(&b.control());
        let (clipboard, mut worker) = ClipboardControl::new(root.clone(), limits).unwrap();
        let seat_a = Seat::new();
        let seat_b = Seat::new();
        for (seat, lease) in [(seat_a, &a), (seat_b, &b)] {
            worker
                .observe_seat(seat.id, Some(seat.snapshot(lease.id(), true)))
                .unwrap();
            worker
                .seal_endpoint(
                    &lease.control(),
                    lease.id(),
                    seat.seal(&lease.control(), lease.id()),
                )
                .unwrap();
        }
        Self {
            pool,
            owner,
            _reservation: reservation,
            root,
            a,
            b,
            clipboard: Arc::new(clipboard),
            worker,
            seat_a,
        }
    }

    fn incoming_is(&self, expected: IncomingLease) {
        assert!(self.root.incoming() == Some(expected));
        assert!(self.a.control().incoming() == Some(expected));
        assert!(self.b.control().incoming() == Some(expected));
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        // This metadata fixture never installs SDK objects, pipes or watchers.
        drop(self.pool.take_pending_surfaces(self.owner));
        let _ = self.pool.retire_surface(self.owner, self.a.id());
        let _ = self.pool.retire_surface(self.owner, self.b.id());
    }
}

#[test]
fn clipboard_a1_checked_ids_and_registration_authority_precede_busy() {
    bounded(|| {
        let counter = Arc::new(AtomicU64::new(u64::MAX - 4));
        let probes: Vec<_> = (0..8)
            .map(|_| {
                let counter = counter.clone();
                thread::spawn(move || allocate_text_request_id(&counter))
            })
            .collect();
        let mut ids = Vec::new();
        let mut exhausted = 0;
        for probe in probes {
            match probe.join().unwrap() {
                Ok(id) => ids.push(id.0),
                Err(error) => {
                    assert_eq!(error, ClipboardError::ResourceLimit);
                    exhausted += 1;
                }
            }
        }
        ids.sort_unstable();
        assert_eq!(ids.len(), 4);
        assert_eq!(exhausted, 4);
        assert!(ids.windows(2).all(|pair| pair[0] != pair[1]));
        assert!(ids.iter().all(|id| *id >= u64::MAX - 4));
        assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
        assert!(matches!(
            allocate_text_request_id(&counter),
            Err(ClipboardError::ResourceLimit)
        ));

        let mut case = Case::new(TextClipboardLimits::default());
        let a = case.a.control();
        assert!(a.registration() == Some(case.a.id()));
        assert!(case.b.control().registration() == Some(case.b.id()));
        let mut no_selection = case.seat_a.snapshot(case.a.id(), true);
        no_selection.selection = None;
        case.worker
            .observe_seat(case.seat_a.id, Some(no_selection))
            .unwrap();
        case.worker
            .seal_endpoint(&a, case.a.id(), case.seat_a.seal(&a, case.a.id()))
            .unwrap();
        assert!(matches!(
            case.clipboard.reserve(&a, case.a.id(), Instant::now()),
            Err(ClipboardError::NoSelection)
        ));
        assert!(case.root.incoming().is_none());
        case.worker
            .observe_seat(
                case.seat_a.id,
                Some(case.seat_a.snapshot(case.a.id(), true)),
            )
            .unwrap();
        case.worker
            .seal_endpoint(&a, case.a.id(), case.seat_a.seal(&a, case.a.id()))
            .unwrap();

        let overlapping = Seat::new();
        case.worker
            .observe_seat(
                overlapping.id,
                Some(overlapping.snapshot(case.a.id(), true)),
            )
            .unwrap();
        case.worker
            .seal_endpoint(&a, case.a.id(), overlapping.seal(&a, case.a.id()))
            .unwrap();
        assert!(matches!(
            case.clipboard.reserve(&a, case.a.id(), Instant::now()),
            Err(ClipboardError::AmbiguousSeat)
        ));
        assert!(case.root.incoming().is_none());
        case.worker.observe_seat(overlapping.id, None).unwrap();
        case.worker
            .seal_endpoint(&a, case.a.id(), case.seat_a.seal(&a, case.a.id()))
            .unwrap();

        a.header().status = ReceiveStatus::Starting;
        assert!(matches!(
            case.clipboard.reserve(&a, case.a.id(), Instant::now()),
            Err(ClipboardError::Unavailable)
        ));
        assert!(case.root.incoming().is_none());
        a.header().status = ReceiveStatus::Ready;
        let obsolete_seal = case.seat_a.seal(&a, case.a.id());
        a.invalidate_pending().unwrap();
        case.b.control().invalidate_pending().unwrap();
        assert_eq!(a.header().epoch, case.b.control().header().epoch);
        assert!(case
            .worker
            .seal_endpoint(&a, case.a.id(), obsolete_seal)
            .is_err());
        assert!(matches!(
            case.clipboard.reserve(&a, case.a.id(), Instant::now()),
            Err(ClipboardError::Unavailable)
        ));
        assert!(case.root.incoming().is_none());
        case.worker
            .seal_endpoint(&a, case.a.id(), case.seat_a.seal(&a, case.a.id()))
            .unwrap();
        let request = case
            .clipboard
            .reserve(&a, case.a.id(), Instant::now())
            .unwrap();
        let lease = IncomingLease::Text(request.id());
        case.incoming_is(lease);

        let other_root = ReceiveControl::new(ReceiveEpoch(1)).unwrap();
        ready(&other_root);
        let foreign =
            ReceiveControl::new_for_surface(a.header().epoch, other_root, case.a.id()).unwrap();
        ready(&foreign);
        for (endpoint, registration) in [(&a, case.b.id()), (&foreign, case.a.id())] {
            let error = case
                .clipboard
                .reserve(endpoint, registration, Instant::now())
                .err();
            assert!(error.is_some() && error != Some(ClipboardError::Busy));
            assert!(case
                .worker
                .seal_endpoint(
                    endpoint,
                    registration,
                    case.seat_a.seal(endpoint, registration)
                )
                .is_err());
            case.incoming_is(lease);
        }
        let root_as_endpoint = case
            .clipboard
            .reserve(&case.root, case.a.id(), Instant::now())
            .err();
        assert!(root_as_endpoint.is_some() && root_as_endpoint != Some(ClipboardError::Busy));
        request.cancel();
        case.worker.reconcile(Instant::now());
        assert!(case.worker.retire_after_cleanup(request.id()));
        assert!(case.root.incoming().is_none());
    });
}

#[test]
fn clipboard_a1_incoming_is_shared_through_hover_decision_and_undrained_terminal() {
    bounded(|| {
        let mut case = Case::new(TextClipboardLimits::default());
        let a = case.a.control();
        let clipboard = case.clipboard.clone();
        let callback_a = a.clone();
        let a_id = case.a.id();
        let (observed, observations) = mpsc::channel();
        let mut coordinator =
            Coordinator::with_control(ReceiveLimits::default(), a.clone(), move || {
                let error = clipboard.reserve(&callback_a, a_id, Instant::now()).err();
                let _ = observed.send(error);
            })
            .unwrap();
        let offer = OfferId(81);
        let drop_lease = IncomingLease::Drop {
            registration: Some(a_id),
            id: offer,
        };
        a.reserve_incoming_drop(Some(a_id), offer).unwrap();
        coordinator
            .enter(offer, SurfacePoint { x: 1.0, y: 2.0 }, 3, true)
            .unwrap();
        assert_eq!(
            observations.recv_timeout(WAIT).unwrap(),
            Some(ClipboardError::Busy)
        );
        case.incoming_is(drop_lease);
        coordinator.action(offer, true).unwrap();
        coordinator.start(offer, Instant::now()).unwrap();
        coordinator
            .complete(offer, b"file:///tmp/a1\r\n", Instant::now())
            .unwrap();
        assert!(matches!(
            case.clipboard.reserve(&a, a_id, Instant::now()),
            Err(ClipboardError::Busy)
        ));
        coordinator.cancel(offer).unwrap();
        case.incoming_is(drop_lease);
        assert!(matches!(
            case.clipboard.reserve(&a, a_id, Instant::now()),
            Err(ClipboardError::Busy)
        ));
        assert!(!a.release_incoming_after_cleanup(IncomingLease::Drop {
            registration: Some(case.b.id()),
            id: offer,
        }));
        assert!(!a.release_incoming_after_cleanup(IncomingLease::Drop {
            registration: Some(a_id),
            id: OfferId(82),
        }));
        case.incoming_is(drop_lease);
        let events = a.poll();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event,
            ReceiveEvent::DropEnded { id, result: Err(ReceiveError::Cancelled) } if *id == offer))
                .count(),
            1
        );
        assert!(a.release_incoming_after_cleanup(drop_lease));
        let text = case.clipboard.reserve(&a, a_id, Instant::now()).unwrap();
        case.incoming_is(IncomingLease::Text(text.id()));
        assert_eq!(
            case.b
                .control()
                .reserve_incoming_drop(Some(case.b.id()), OfferId(83)),
            Err(ReceiveError::Busy)
        );
        text.cancel();
        case.worker.reconcile(Instant::now());
        assert!(case.worker.retire_after_cleanup(text.id()));

        // The same helper resolves a standalone root once, without self-locking.
        let standalone = ReceiveControl::new(ReceiveEpoch(1)).unwrap();
        ready(&standalone);
        standalone.reserve_incoming_drop(None, offer).unwrap();
        assert!(
            standalone.incoming()
                == Some(IncomingLease::Drop {
                    registration: None,
                    id: offer
                })
        );
        assert!(
            standalone.release_incoming_after_cleanup(IncomingLease::Drop {
                registration: None,
                id: offer
            })
        );
        assert!(standalone.incoming().is_none());
    });
}

#[test]
fn clipboard_a1_ack_claim_freezes_mime_and_respects_direct_authority_cuts() {
    bounded(|| {
        let mut case = Case::new(TextClipboardLimits::default());
        let a = case.a.control();
        case.seat_a.selection = next_clipboard_stamp().unwrap();
        case.worker
            .observe_seat(
                case.seat_a.id,
                Some(case.seat_a.snapshot(case.a.id(), false)),
            )
            .unwrap();
        case.worker
            .seal_endpoint(&a, case.a.id(), case.seat_a.seal(&a, case.a.id()))
            .unwrap();
        let request = case
            .clipboard
            .reserve(&a, case.a.id(), Instant::now())
            .unwrap();
        let before_ack = Instant::now();
        request.acknowledge(before_ack).unwrap();
        let after_ack = Instant::now();
        assert_eq!(
            request.acknowledge(after_ack),
            Err(ClipboardError::InvalidPhase)
        );
        let (pending, registration) = case.worker.pending_request().unwrap();
        assert!(pending == request.id() && registration == case.a.id());
        // Later MIME flags on the SAME Selection do not change the reserved MIME.
        case.worker
            .observe_seat(
                case.seat_a.id,
                Some(case.seat_a.snapshot(case.a.id(), true)),
            )
            .unwrap();
        assert!(case
            .worker
            .claim_receive(pending, &a, case.b.id(), Instant::now())
            .is_err());
        let claim = case
            .worker
            .claim_receive(pending, &a, registration, Instant::now())
            .unwrap();
        assert!(claim.request == request.id());
        assert!(claim.seat == case.seat_a.id && claim.selection == case.seat_a.selection);
        assert!(claim.mime == TextMime::Plain);
        let timeout = TextClipboardLimits::default().read_timeout;
        assert!(claim.deadline >= before_ack + timeout && claim.deadline <= after_ack + timeout);
        // Successful claim returns with root/endpoint/request guards all released.
        a.invalidate_pending().unwrap();
        case.root.close().unwrap();
        request.cancel();
        assert!(claim.request == request.id());
        case.worker.reconcile(Instant::now());
        assert!(case.worker.retire_after_cleanup(request.id()));

        for cut in 0..3 {
            let mut case = Case::new(TextClipboardLimits::default());
            let a = case.a.control();
            let request = case
                .clipboard
                .reserve(&a, case.a.id(), Instant::now())
                .unwrap();
            request.acknowledge(Instant::now()).unwrap();
            let (pending, registration) = case.worker.pending_request().unwrap();
            match cut {
                0 => request.cancel(),
                1 => case.root.close().unwrap(),
                _ => {
                    a.invalidate_pending().unwrap();
                }
            }
            // No worker sweep precedes claim: the controls themselves arbitrate.
            let denied = case
                .worker
                .claim_receive(pending, &a, registration, Instant::now());
            assert!(denied.is_err());
            if cut == 0 {
                assert!(matches!(denied, Err(ClipboardError::Cancelled)));
            }
            case.incoming_is(IncomingLease::Text(request.id()));
            case.worker.reconcile(Instant::now());
            assert!(case.worker.retire_after_cleanup(request.id()));
        }

        let mut case = Case::new(TextClipboardLimits::default());
        let request = case
            .clipboard
            .reserve(&case.a.control(), case.a.id(), Instant::now())
            .unwrap();
        request.acknowledge(Instant::now()).unwrap();
        assert!(
            case.worker
                .claim_receive(request.id(), &case.a.control(), case.a.id(), Instant::now())
                .unwrap()
                .mime
                == TextMime::Utf8
        );
    });
}

#[test]
fn clipboard_a1_deadlines_use_fresh_locked_time_and_old_handles_cannot_release_b() {
    bounded(|| {
        let limits = TextClipboardLimits {
            acknowledgement_timeout: Duration::from_millis(60),
            read_timeout: Duration::from_millis(60),
            ..TextClipboardLimits::default()
        };
        let mut case = Case::new(limits);
        let a = case.a.control();
        let old_now = Instant::now();
        let request = case.clipboard.reserve(&a, case.a.id(), old_now).unwrap();
        let expires_no_later_than = Instant::now() + limits.acknowledgement_timeout;
        let guard = case.root.header();
        let (attempting, attempted) = mpsc::channel();
        let (finished, result) = mpsc::channel();
        let probe = thread::spawn(move || {
            let _ = attempting.send(());
            let ack = request.acknowledge(old_now);
            let _ = finished.send((request, ack));
        });
        let entered = attempted.recv_timeout(WAIT);
        wait_until(expires_no_later_than);
        drop(guard);
        assert_eq!(entered, Ok(()));
        let (old, ack) = result.recv_timeout(WAIT).unwrap();
        probe.join().unwrap();
        assert_eq!(ack, Err(ClipboardError::Timeout));
        case.incoming_is(IncomingLease::Text(old.id()));
        case.worker.reconcile(Instant::now());
        assert!(case.worker.retire_after_cleanup(old.id()));

        // Use future transition instants for the exact >= edges, without a clock seam.
        let anchor = Instant::now() + Duration::from_secs(10);
        let exact_ack = case.clipboard.reserve(&a, case.a.id(), anchor).unwrap();
        assert_eq!(
            exact_ack.acknowledge(anchor + limits.acknowledgement_timeout),
            Err(ClipboardError::Timeout)
        );
        case.worker
            .reconcile(anchor + limits.acknowledgement_timeout);
        assert!(case.worker.retire_after_cleanup(exact_ack.id()));
        let exact_read = case.clipboard.reserve(&a, case.a.id(), anchor).unwrap();
        exact_read.acknowledge(anchor).unwrap();
        assert_eq!(
            exact_read.acknowledge(anchor + Duration::from_millis(1)),
            Err(ClipboardError::InvalidPhase)
        );
        assert!(matches!(
            case.worker.claim_receive(
                exact_read.id(),
                &a,
                case.a.id(),
                anchor + limits.read_timeout
            ),
            Err(ClipboardError::Timeout)
        ));
        case.worker.reconcile(anchor + limits.read_timeout);
        assert!(case.worker.retire_after_cleanup(exact_read.id()));

        let b = case.b.control();
        let request_b = case
            .clipboard
            .reserve(&b, case.b.id(), Instant::now())
            .unwrap();
        assert!(old.id() != request_b.id());
        assert!(old.acknowledge(Instant::now()).is_err());
        old.cancel();
        case.worker.reconcile(Instant::now());
        assert!(!case.worker.retire_after_cleanup(old.id()));
        assert!(!a.release_incoming_after_cleanup(IncomingLease::Text(old.id())));
        case.incoming_is(IncomingLease::Text(request_b.id()));
        request_b.acknowledge(Instant::now()).unwrap();
        let stale_now = Instant::now();
        let read_expires_no_later_than = stale_now + limits.read_timeout;
        let authority = case.root.clone();
        let guard = authority.header();
        let registration = case.b.id();
        let (attempting, attempted) = mpsc::channel();
        let (finished, result) = mpsc::channel();
        let probe = thread::spawn(move || {
            let _ = attempting.send(());
            let claim = case
                .worker
                .claim_receive(request_b.id(), &b, registration, stale_now);
            let _ = finished.send((case, request_b, claim));
        });
        let entered = attempted.recv_timeout(WAIT);
        wait_until(read_expires_no_later_than);
        drop(guard);
        assert_eq!(entered, Ok(()));
        let (mut case, request_b, claim) = result.recv_timeout(WAIT).unwrap();
        probe.join().unwrap();
        assert!(matches!(claim, Err(ClipboardError::Timeout)));
        case.incoming_is(IncomingLease::Text(request_b.id()));
        case.worker.reconcile(Instant::now());
        assert!(case.worker.retire_after_cleanup(request_b.id()));
    });
}

#[test]
fn clipboard_a1_focus_selection_epoch_and_lifetime_cuts_keep_exact_identity() {
    bounded(|| {
        for cut in 0..4 {
            let mut case = Case::new(TextClipboardLimits::default());
            let a = case.a.control();
            let request = case
                .clipboard
                .reserve(&a, case.a.id(), Instant::now())
                .unwrap();
            request.acknowledge(Instant::now()).unwrap();
            let (pending, registration) = case.worker.pending_request().unwrap();
            let mut fresh = case.seat_a;
            match cut {
                0 => {
                    fresh.focus = next_clipboard_stamp().unwrap();
                }
                1 => {
                    fresh.selection = next_clipboard_stamp().unwrap();
                }
                2 => {
                    fresh.keyboard = next_clipboard_stamp().unwrap();
                }
                _ => {
                    a.invalidate_pending().unwrap();
                }
            }
            case.worker
                .observe_seat(fresh.id, Some(fresh.snapshot(case.a.id(), true)))
                .unwrap();
            assert!(case
                .worker
                .claim_receive(pending, &a, registration, Instant::now())
                .is_err());
            case.incoming_is(IncomingLease::Text(request.id()));
            case.worker.reconcile(Instant::now());
            assert!(case.worker.retire_after_cleanup(request.id()));
            case.worker
                .seal_endpoint(&a, case.a.id(), fresh.seal(&a, case.a.id()))
                .unwrap();
            // Clipboard resealing is independent of a fresh DnD Enter/armed bit.
            let replacement = case
                .clipboard
                .reserve(&a, case.a.id(), Instant::now())
                .unwrap();
            replacement.acknowledge(Instant::now()).unwrap();
            let (pending, registration) = case.worker.pending_request().unwrap();
            case.a.request_retire();
            assert!(case
                .worker
                .claim_receive(pending, &a, registration, Instant::now())
                .is_err());
            replacement.cancel();
            case.worker.reconcile(Instant::now());
            assert!(case.worker.retire_after_cleanup(replacement.id()));
        }

        // The SID is real, but the metadata subject root is distinct from the
        // pool's root so its lifetime is not obscured by reaper/slot retention.
        let case = Case::new(TextClipboardLimits::default());
        let registration = case.a.id();
        let root = ReceiveControl::new(ReceiveEpoch(1)).unwrap();
        let alive = root.lifetime_observer();
        let endpoint =
            ReceiveControl::new_for_surface(ReceiveEpoch(1), root.clone(), registration).unwrap();
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
        assert!(alive());
        drop(request);
        drop(worker);
        drop(clipboard);
        drop(endpoint);
        drop(root);
        assert!(
            !alive(),
            "Root -> current Core must not cycle through a strong child control"
        );
    });
}
#[path = "clipboard_body_tests.rs"]
mod body;
