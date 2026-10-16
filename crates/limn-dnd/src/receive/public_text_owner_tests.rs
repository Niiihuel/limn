//! Public-only consumer operations with real retained display/window fixtures.

use super::*;
use crate::receive::read_tag::ReadTag;
use crate::{ClipboardError, TextClipboardLimits, TextPasteRequest};

const UTF8: &str = "text/plain;charset=utf-8";

fn text_case() -> PublicOwnerCase {
    PublicOwnerCase::with_text(
        testing::Hooks::default(),
        ReceiveLimits::default(),
        TextClipboardLimits {
            max_text_bytes: 256,
            ..TextClipboardLimits::default()
        },
    )
}

fn ready_text(case: &mut PublicOwnerCase) {
    case.until(|case| {
        case.owner().status() == ReceiveStatus::Ready && case.fixture().has_keyboard(0)
    });
}

fn reserve_after_cleanup(
    case: &mut PublicOwnerCase,
    endpoint: &FileDropEndpoint,
) -> TextPasteRequest {
    let mut outcome = None;
    case.until(|case| match case.owner().reserve_text_paste(endpoint) {
        Ok(request) => {
            outcome = Some(request);
            true
        }
        Err(ClipboardError::Busy) => false,
        Err(error) => panic!("unexpected public reserve after cleanup: {error:?}"),
    });
    outcome.unwrap()
}

fn focus_selection(
    case: &mut PublicOwnerCase,
    endpoint: &FileDropEndpoint,
    key: usize,
) -> TextPasteRequest {
    let before = case.observed.clipboard_seals.load(Ordering::SeqCst);
    case.fixture().text_selection(0, key, true, true);
    case.fixture_mut().keyboard_enter(0, 0);
    case.until(|case| {
        endpoint.status() == EndpointStatus::Ready
            && case.observed.clipboard_seals.load(Ordering::SeqCst) > before
    });
    case.owner()
        .reserve_text_paste(endpoint)
        .expect("the real public context is sealed")
}

fn redacted(request: &TextPasteRequest) -> String {
    let debug = format!("{request:?}");
    assert!(debug.contains("TextPasteRequest"));
    assert!(!debug.contains("0x"));
    debug
}

fn real_text_fd(case: &mut PublicOwnerCase) -> Weak<OwnedFd> {
    let mut result = None;
    case.until(|case| {
        if let Some((ReadTag::Text(_), fd)) = case.observed.watched_pipe.lock().unwrap().as_ref() {
            if fd.upgrade().is_some() {
                result = Some(fd.clone());
            }
        }
        result.is_some()
    });
    result.unwrap()
}

fn received_writer(
    case: &mut PublicOwnerCase,
    key: usize,
    count: usize,
) -> (OwnedFd, Weak<OwnedFd>) {
    case.until(|case| case.fixture().text_receive_count(key, UTF8) == count);
    let fd = real_text_fd(case);
    (case.fixture_mut().take_writer(key), fd)
}

fn write_public(writer: &OwnedFd, mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let count = rustix::io::write(writer, bytes).unwrap();
        assert!(count > 0);
        bytes = &bytes[count..];
    }
}

fn drain_public_prefix(
    case: &mut PublicOwnerCase,
    writer: &OwnedFd,
    fd: &Weak<OwnedFd>,
    bytes: &[u8],
) {
    write_public(writer, bytes);
    case.until(|_| {
        // A temporary observational upgrade never crosses EOF/cancel below.
        let actual = fd.upgrade().expect("the real public reader remains alive");
        rustix::io::ioctl_fionread(actual.as_ref()).unwrap() == 0
    });
}

fn public_result(
    case: &mut PublicOwnerCase,
    request: &TextPasteRequest,
    fd: &Weak<OwnedFd>,
) -> Result<String, ClipboardError> {
    let mut result = None;
    case.until(|_| {
        // Poll every iteration; the consumer must not conceal early publication
        // by deciding to poll only after the watcher disappeared.
        result = request.poll();
        if matches!(result, Some(Ok(_))) {
            assert!(
                fd.upgrade().is_none(),
                "observed public success requires actual watcher retirement"
            );
        }
        result.is_some()
    });
    result.unwrap()
}

fn no_text_dnd_requests(case: &PublicOwnerCase, key: usize) {
    assert!(case.fixture().server.requests.iter().all(|request| !matches!(request,
        OfferRequest::Accept(actual, _) | OfferRequest::Actions(actual, _, _) | OfferRequest::Finish(actual)
        if *actual == key)));
    assert_eq!(case.fixture().finish_count(key), 0);
}

fn close_public_text(case: &mut PublicOwnerCase, keyboards: usize) {
    case.owner().close();
    case.until(|case| case.owner().status() == ReceiveStatus::Closed(None));
    assert!(case.display.upgrade().is_none());
    let mut queue = case.fixture().host.new_event_queue::<PublicHostSync>();
    let mut alive = PublicHostSync::default();
    case.fixture().host.display().sync(&queue.handle(), ());
    case.fixture().host.flush().unwrap();
    case.until(|_| {
        queue.dispatch_pending(&mut alive).unwrap();
        alive.0
    });
    assert_eq!(
        case.fixture().server.released_devices,
        case.fixture().server.devices
    );
    assert_eq!(
        case.fixture().server.released_seats,
        case.fixture().server.seat_versions.len()
    );
    assert_eq!(case.fixture().server.released_keyboards, keyboards);
    assert!(case.fixture().server.keyboards.is_empty());
    assert!(case
        .fixture()
        .server
        .surfaces
        .iter()
        .all(|surface| surface.is_alive()));
}

#[test]
fn public_text_owner_transfers_raw_and_empty_once_without_exposing_request_identity() {
    let mut case = text_case();
    ready_text(&mut case);
    let (endpoint, window) = case.register(0);
    let initial = focus_selection(&mut case, &endpoint, 401);
    let mut initial = Some(initial);
    for (index, raw) in ["privado á😀\r\nsegunda línea\r\n", ""]
        .into_iter()
        .enumerate()
    {
        let request = initial
            .take()
            .unwrap_or_else(|| reserve_after_cleanup(&mut case, &endpoint));
        let opaque = redacted(&request);
        if !raw.is_empty() {
            assert!(!opaque.contains(raw));
        }
        assert_eq!(request.poll(), None);
        assert_eq!(case.fixture().text_receive_count(401, UTF8), index);
        request.acknowledge().unwrap();
        assert_eq!(request.acknowledge(), Err(ClipboardError::InvalidPhase));
        assert_eq!(redacted(&request), opaque);
        let (writer, fd) = received_writer(&mut case, 401, index + 1);
        if !raw.is_empty() {
            drain_public_prefix(&mut case, &writer, &fd, raw.as_bytes());
        }
        assert!(fd.upgrade().is_some());
        assert_eq!(request.poll(), None);
        assert!(matches!(
            case.owner().reserve_text_paste(&endpoint),
            Err(ClipboardError::Busy)
        ));
        let before_wake = case.wakes.load(Ordering::SeqCst);
        drop(writer);
        assert_eq!(public_result(&mut case, &request, &fd), Ok(raw.to_owned()));
        assert_eq!(request.poll(), None);
        assert_eq!(redacted(&request), opaque);
        request.cancel();
        assert_eq!(request.poll(), None);
        case.until(|case| case.wakes.load(Ordering::SeqCst) > before_wake);
        no_text_dnd_requests(&case, 401);
    }
    assert_eq!(case.fixture().server.devices, 1);
    assert_eq!(case.fixture().text_receive_count(401, UTF8), 2);
    close_public_text(&mut case, 1);
    assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
    assert!(window.upgrade().is_none());
}

#[test]
fn public_text_cancel_and_drop_release_reserved_and_open_reading_requests() {
    for drop_reading in [false, true] {
        let mut case = text_case();
        ready_text(&mut case);
        let (endpoint, _) = case.register(0);
        let reserved = focus_selection(&mut case, &endpoint, 402);
        let opaque = redacted(&reserved);
        reserved.cancel();
        assert_eq!(reserved.poll(), Some(Err(ClipboardError::Cancelled)));
        assert_eq!(reserved.poll(), None);
        assert_eq!(redacted(&reserved), opaque);
        let abandoned = reserve_after_cleanup(&mut case, &endpoint);
        drop(abandoned); // Public Drop cancels a reserved request without reading.
        let active = reserve_after_cleanup(&mut case, &endpoint);
        assert_eq!(case.fixture().text_receive_count(402, UTF8), 0);
        active.acknowledge().unwrap();
        let (writer_a, fd_a) = received_writer(&mut case, 402, 1);
        drain_public_prefix(&mut case, &writer_a, &fd_a, b"prefijo abierto\r\n");
        assert_eq!(active.poll(), None);
        let active = if drop_reading {
            drop(active);
            None
        } else {
            active.cancel();
            assert_eq!(active.poll(), Some(Err(ClipboardError::Cancelled)));
            assert_eq!(active.poll(), None);
            Some(active)
        };
        let mut next = None;
        case.until(|case| {
            if let Some(active) = active.as_ref() {
                assert_eq!(active.poll(), None);
            }
            match case.owner().reserve_text_paste(&endpoint) {
                Ok(request) => {
                    assert!(
                        fd_a.upgrade().is_none(),
                        "public successor needs real old-reader retirement"
                    );
                    next = Some(request);
                    true
                }
                Err(ClipboardError::Busy) => false,
                Err(error) => panic!("unexpected public cancellation cleanup: {error:?}"),
            }
        });
        let next = next.unwrap();
        reserved.cancel();
        assert_eq!(reserved.poll(), None);
        if let Some(active) = active.as_ref() {
            active.cancel();
            assert_eq!(active.poll(), None);
        }
        next.acknowledge().unwrap();
        let (writer_b, fd_b) = received_writer(&mut case, 402, 2);
        write_public(&writer_b, b"sucesor B\r\n");
        drop(writer_b);
        assert_eq!(
            public_result(&mut case, &next, &fd_b),
            Ok("sucesor B\r\n".to_owned())
        );
        assert_eq!(next.poll(), None);
        // The cancelled first source remained open through B's entire transfer.
        drop(writer_a);
        no_text_dnd_requests(&case, 402);
        close_public_text(&mut case, 1);
        assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
    }
    // These public operations make progress without a private Ping call here.
    // The existing 20-ms tick can also drive them; specific Ping emission still
    // requires source review after the guards, not a functional claim here.
}

#[test]
fn public_text_owner_keeps_default_mode_busy_foreign_and_closed_errors_distinct() {
    {
        let mut default = PublicOwnerCase::new();
        default.until(|case| case.owner().status() == ReceiveStatus::Ready);
        let (endpoint, _) = default.register(0);
        default.until(|_| endpoint.status() == EndpointStatus::Ready);
        assert!(matches!(
            default.owner().reserve_text_paste(&endpoint),
            Err(ClipboardError::Unavailable)
        ));
        let wakes = Arc::new(AtomicUsize::new(0));
        let callback_wakes = wakes.clone();
        // The exact real display is already owned; do not reacquire the shared
        // test mutex or manufacture a second native pointer for this attempt.
        let duplicate = unsafe {
            WaylandDataOwner::attach_owned_with_text_clipboard(
                Arc::new(FixtureDisplay {
                    connection: default.fixture().host.clone(),
                }),
                ReceiveLimits::default(),
                TextClipboardLimits::default(),
                move || {
                    callback_wakes.fetch_add(1, Ordering::SeqCst);
                },
            )
        };
        assert!(matches!(duplicate, Err(ClipboardError::Busy)));
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
        assert_eq!(default.owner().status(), ReceiveStatus::Ready);
        close_public_text(&mut default, 0);
    }

    let mut case = text_case();
    ready_text(&mut case);
    assert!(case._public_pool_guard.is_some());
    // The first case retains the one shared test guard for both real displays.
    let mut foreign = PublicOwnerCase::with_mode(
        testing::Hooks::default(),
        ReceiveLimits::default(),
        Some(TextClipboardLimits::default()),
        None,
    );
    ready_text(&mut foreign);
    let (other, _) = foreign.register(0);
    foreign.until(|_| other.status() == EndpointStatus::Ready);
    let (endpoint, window) = case.register(0);
    let before = case.observed.clipboard_seals.load(Ordering::SeqCst);
    case.fixture().selection_none(0);
    case.fixture_mut().keyboard_enter(0, 0);
    case.until(|case| {
        endpoint.status() == EndpointStatus::Ready
            && case.observed.clipboard_seals.load(Ordering::SeqCst) > before
    });
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::NoSelection)
    ));
    case.fixture().text_selection(0, 403, true, true);
    let mut request = None;
    // Only this explicitly announced asynchronous NULL->Selection transition
    // retries NoSelection. It does not hide the preceding settled assertion.
    case.until(|case| match case.owner().reserve_text_paste(&endpoint) {
        Ok(current) => {
            request = Some(current);
            true
        }
        Err(ClipboardError::NoSelection) => false,
        Err(error) => panic!("unexpected public selection replacement: {error:?}"),
    });
    let request = request.unwrap();
    assert!(matches!(
        case.owner().reserve_text_paste(&other),
        Err(ClipboardError::StaleRequest)
    ));
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::Busy)
    ));
    assert_eq!(case.fixture().text_receive_count(403, UTF8), 0);
    case.owner().close();
    assert!(matches!(
        case.owner().reserve_text_paste(&endpoint),
        Err(ClipboardError::Closed)
    ));
    assert_eq!(request.poll(), Some(Err(ClipboardError::Closed)));
    assert_eq!(request.poll(), None);
    close_public_text(&mut case, 1);
    close_public_text(&mut foreign, 1);
    assert_eq!(endpoint.status(), EndpointStatus::Detached(None));
    assert_eq!(other.status(), EndpointStatus::Detached(None));
    assert!(
        window.upgrade().is_none(),
        "the still-live opaque request retains no native window owner"
    );
}

#[path = "public_text_ttl_runtime_tests.rs"]
mod ttl;

#[path = "public_text_cut_runtime_tests.rs"]
mod cuts;
