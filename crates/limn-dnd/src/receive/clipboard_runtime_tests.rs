//! Text consumers of the actual admitted owner, protocol queue and FD watcher.

use super::*;
use crate::receive::{
    clipboard_control::{ClipboardStamp, TextRequestControl, TextSeatSnapshot},
    control::IncomingLease,
};
use crate::{ClipboardError, TextClipboardLimits};

const UTF8: &str = "text/plain;charset=utf-8";
const PREFIX: &[u8] = b"primera linea\r\n";
const SUFFIX: &str = "segunda á😀\r\n";

fn drive_text(case: &mut LiveOwnerCase, mut done: impl FnMut(&LiveOwnerCase) -> bool) {
    let deadline = Instant::now() + WAIT;
    loop {
        case.fixture_mut().serve();
        case.fixture().host_read();
        if done(case) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "real clipboard runtime made no progress"
        );
        thread::yield_now();
    }
}

fn focused(
    case: &LiveOwnerCase,
    registration: Option<crate::receive::worker_pool::SurfaceId>,
) -> Option<TextSeatSnapshot> {
    case.clipboard()
        .observed_seats()
        .into_iter()
        .flatten()
        .find(|seat| {
            seat.focus
                .is_some_and(|focus| focus.registration == registration)
        })
}

fn assert_no_dnd_requests(case: &LiveOwnerCase, key: usize) {
    assert!(case.fixture().server.requests.iter().all(|request| !matches!(request,
        OfferRequest::Accept(actual, _) | OfferRequest::Actions(actual, _, _) | OfferRequest::Finish(actual)
        if *actual == key)));
    assert_eq!(case.fixture().finish_count(key), 0);
}

fn text_host_still_works(case: &mut LiveOwnerCase) {
    host_still_works(case.fixture_mut());
    assert_eq!(case.fixture().server.released_keyboards, 1);
    assert!(case.fixture().server.keyboards.is_empty());
}

fn reserve_text(case: &mut LiveOwnerCase, endpoint: &SurfaceLease) -> TextRequestControl {
    let mut reserved = None;
    drive_text(case, |case| {
        match case
            .clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now())
        {
            Ok(request) => {
                reserved = Some(request);
                true
            }
            Err(ClipboardError::Unavailable) => false,
            Err(error) => panic!("unexpected real text admission result: {error:?}"),
        }
    });
    reserved.unwrap()
}

fn text_is_busy(case: &LiveOwnerCase, endpoint: &SurfaceLease) {
    // This vertical case attempts Text admission. A competing native DnD
    // gesture during the live text reader remains a separate required case.
    assert!(matches!(
        case.clipboard()
            .reserve(&endpoint.control(), endpoint.id(), Instant::now()),
        Err(ClipboardError::Busy)
    ));
}

fn poll_text(
    case: &LiveOwnerCase,
    request: &TextRequestControl,
) -> Option<Result<String, ClipboardError>> {
    let outcome = request.poll(Instant::now());
    if outcome.is_some() {
        case.ping.ping();
    }
    outcome
}

fn watched_text(observed: &testing::Observations, request: &TextRequestControl) -> Weak<OwnedFd> {
    let mut result = None;
    wait(
        || {},
        || {
            if let Some((tag, fd)) = observed.watched_pipe.lock().unwrap().as_ref() {
                if *tag == ReadTag::Text(request.id()) && fd.upgrade().is_some() {
                    result = Some(fd.clone());
                }
            }
            result.is_some()
        },
    );
    result.unwrap()
}

fn prepare_unregistered_selection(case: &mut LiveOwnerCase, key: usize) -> ClipboardStamp {
    drive_text(case, |case| {
        case.lifecycle.status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
    });
    assert_eq!(case.fixture().server.devices, 1);
    assert!(case.lifecycle.incoming().is_none());
    case.fixture().text_selection(0, key, true, true);
    case.fixture_mut().keyboard_enter(0, 0);
    drive_text(case, |case| {
        focused(case, None).is_some_and(|seat| seat.selection.is_some())
    });
    let before_registration = focused(case, None).unwrap();
    assert!(before_registration.focus.unwrap().registration.is_none());
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);
    before_registration.selection.unwrap().id
}

fn first_request(
    case: &mut LiveOwnerCase,
    key: usize,
) -> (
    Arc<SurfaceLease>,
    TextRequestControl,
    OwnedFd,
    Weak<OwnedFd>,
) {
    let selection = prepare_unregistered_selection(case, key);
    let (endpoint, _) = case.register(0, || {});
    let request = reserve_text(case, &endpoint);
    let after_registration = focused(case, Some(endpoint.id())).unwrap();
    assert!(after_registration.selection.unwrap().id == selection);
    assert!(after_registration.focus.unwrap().sealed);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert_eq!(case.fixture().text_receive_count(key, UTF8), 0);
    assert_eq!(request.poll(Instant::now()), None);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    assert_no_dnd_requests(case, key);
    request.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(case, |case| {
        case.fixture().text_receive_count(key, UTF8) == 1
    });
    let fd = watched_text(&case.observed, &request);
    let writer = case.fixture_mut().take_writer(key);
    assert_no_dnd_requests(case, key);
    (endpoint, request, writer, fd)
}

fn drain_prefix(case: &mut LiveOwnerCase, writer: &OwnedFd, fd: &Weak<OwnedFd>) {
    write_chunk(writer, PREFIX);
    drive_text(case, |_| {
        // This temporary Arc only observes the exact installed FD. It is gone
        // before EOF, cancellation or any retirement assertion below.
        let current = fd.upgrade().expect("the live prefix reader still exists");
        rustix::io::ioctl_fionread(current.as_ref()).unwrap() == 0
    });
}

fn finish_b_on_same_selection(
    case: &mut LiveOwnerCase,
    endpoint: &SurfaceLease,
    old: &TextRequestControl,
    key: usize,
) {
    let next = reserve_text(case, endpoint);
    assert!(old.id() != next.id());
    old.cancel();
    assert_eq!(poll_text(case, old), None);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(next.id())));
    next.acknowledge(Instant::now()).unwrap();
    case.ping.ping();
    drive_text(case, |case| {
        case.fixture().text_receive_count(key, UTF8) == 2
    });
    let fd = watched_text(&case.observed, &next);
    let writer = case.fixture_mut().take_writer(key);
    write_chunk(&writer, b"nuevo B\r\n");
    drop(writer);
    let mut outcome = None;
    drive_text(case, |case| {
        outcome = poll_text(case, &next);
        if matches!(outcome, Some(Ok(_))) {
            assert!(
                fd.upgrade().is_none(),
                "observed text success requires the actual watcher FD gone"
            );
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok("nuevo B\r\n".to_owned())));
    assert_eq!(poll_text(case, &next), None);
    assert!(case.lifecycle.incoming().is_none());
    assert_no_dnd_requests(case, key);
}

#[test]
fn text_selection_precedes_registration_and_eof_waits_for_the_actual_generic_retirement() {
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
    let (endpoint, request, writer, fd) = first_request(&mut case, 101);
    drain_prefix(&mut case, &writer, &fd);
    assert!(matches!(
        retirement_entered.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert_eq!(poll_text(&case, &request), None);
    text_is_busy(&case, &endpoint);
    assert!(fd.upgrade().is_some());
    write_chunk(&writer, SUFFIX.as_bytes());
    drop(writer);
    retirement_entered
        .recv_timeout(WAIT)
        .expect("EOF staged before real Generic removal");
    assert!(fd.upgrade().is_some());
    assert_eq!(poll_text(&case, &request), None);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    text_is_busy(&case, &endpoint);
    release.release();
    let mut outcome = None;
    drive_text(&mut case, |case| {
        outcome = poll_text(case, &request);
        if matches!(outcome, Some(Ok(_))) {
            assert!(
                fd.upgrade().is_none(),
                "observed text success requires the actual watcher FD gone"
            );
        }
        outcome.is_some()
    });
    assert!(fd.upgrade().is_none());
    assert_eq!(
        outcome,
        Some(Ok(format!(
            "{}{}",
            std::str::from_utf8(PREFIX).unwrap(),
            SUFFIX
        )))
    );
    assert_eq!(poll_text(&case, &request), None);
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    finish_b_on_same_selection(&mut case, &endpoint, &request, 101);
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    case.close();
    assert!(endpoint.is_detached());
    text_host_still_works(&mut case);
}

fn cancellation_at_real_watcher_gate(leave_writer_open: bool) {
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
    let (endpoint, request, writer, fd) = first_request(&mut case, 102);
    drain_prefix(&mut case, &writer, &fd);
    assert!(matches!(
        retirement_entered.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert_eq!(poll_text(&case, &request), None);
    let mut writer = Some(writer);
    if leave_writer_open {
        request.cancel();
        case.ping.ping();
    } else {
        drop(writer.take());
    }
    retirement_entered
        .recv_timeout(WAIT)
        .expect("the real text reader retires at the runtime boundary");
    if !leave_writer_open {
        request.cancel();
    }
    assert!(fd.upgrade().is_some());
    assert_eq!(
        poll_text(&case, &request),
        Some(Err(ClipboardError::Cancelled))
    );
    assert_eq!(poll_text(&case, &request), None);
    assert!(case.lifecycle.incoming() == Some(IncomingLease::Text(request.id())));
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert!(!endpoint.is_detached());
    text_is_busy(&case, &endpoint);
    release.release();
    drive_text(&mut case, |case| {
        assert_eq!(poll_text(case, &request), None);
        fd.upgrade().is_none() && case.lifecycle.incoming().is_none()
    });
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    finish_b_on_same_selection(&mut case, &endpoint, &request, 102);
    // C keeps the cancelled source's writer open through B's real full transfer.
    assert_eq!(writer.is_some(), leave_writer_open);
    drop(writer);
    assert_no_dnd_requests(&case, 102);
    case.close();
    assert!(endpoint.is_detached());
    text_host_still_works(&mut case);
}

#[test]
fn text_cancel_after_eof_staging_cannot_publish_before_generic_cleanup() {
    cancellation_at_real_watcher_gate(false);
}

#[test]
fn text_cancel_during_an_open_drained_prefix_retires_without_waiting_for_eof() {
    cancellation_at_real_watcher_gate(true);
}
#[path = "clipboard_prefix_runtime_tests.rs"]
mod prefix;

#[path = "clipboard_fence_runtime_tests.rs"]
mod fence;

#[path = "clipboard_dnd_runtime_tests.rs"]
mod dnd;

#[path = "clipboard_payload_runtime_tests.rs"]
mod payload;

#[path = "clipboard_context_runtime_tests.rs"]
mod context;

#[path = "clipboard_seat_runtime_tests.rs"]
mod seats;

#[path = "clipboard_barrier_runtime_tests.rs"]
mod barrier;

#[path = "clipboard_quota_runtime_tests.rs"]
mod quota;

#[path = "clipboard_fairness_runtime_tests.rs"]
mod fairness;

#[path = "clipboard_cleanup_runtime_tests.rs"]
mod cleanup;

#[path = "clipboard_callback_cut_runtime_tests.rs"]
mod callback_cut;

#[path = "clipboard_tombstone_runtime_tests.rs"]
mod tombstone;
