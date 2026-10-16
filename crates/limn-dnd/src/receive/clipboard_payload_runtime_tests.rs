//! Real text reader bounds and EOF/error cleanup; wire below clipboard_runtime_tests.
use super::*;

const EXACT_SEVEN: &[u8] = b"a\r\n\xf0\x9f\x98\x80";
const EXACT_SEVEN_TEXT: &str = "a\r\n😀";

fn payload_busy(case: &LiveOwnerCase, endpoint: &SurfaceLease, request: &TextRequestControl) {
    let exact = IncomingLease::Text(request.id());
    assert!(case.lifecycle.incoming() == Some(exact));
    assert!(endpoint.control().incoming() == Some(exact));
    text_is_busy(case, endpoint);
}

fn finish_seven_byte_b(
    case: &mut LiveOwnerCase,
    endpoint: &SurfaceLease,
    old: &TextRequestControl,
    key: usize,
) {
    let next = reserve_text(case, endpoint);
    assert!(next.id() != old.id());
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
    assert_eq!(EXACT_SEVEN.len(), 7);
    write_chunk(&writer, EXACT_SEVEN);
    drop(writer);
    let mut outcome = None;
    drive_text(case, |case| {
        outcome = poll_text(case, &next);
        if matches!(outcome, Some(Ok(_))) {
            assert!(fd.upgrade().is_none());
        }
        outcome.is_some()
    });
    assert_eq!(outcome, Some(Ok(EXACT_SEVEN_TEXT.to_owned())));
    assert_eq!(poll_text(case, &next), None);
    assert!(fd.upgrade().is_none());
    assert!(case.lifecycle.incoming().is_none());
    assert_no_dnd_requests(case, key);
}

#[test]
fn real_text_reader_enforces_seven_bytes_and_strict_utf8_with_exact_cleanup() {
    let cases: [(usize, &[u8], Option<ClipboardError>, bool); 4] = [
        (601, b"", None, false),
        (602, EXACT_SEVEN, None, false),
        (
            603,
            b"a\r\n\xf0\x9f\x98\x80b",
            Some(ClipboardError::TooLarge),
            true,
        ),
        (
            604,
            b"\xf0\x28\x8c\x28",
            Some(ClipboardError::InvalidText),
            false,
        ),
    ];
    for (key, bytes, error, leave_source_open) in cases {
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
            TextClipboardLimits {
                max_text_bytes: 7,
                ..TextClipboardLimits::default()
            },
        );
        let mut release = OpenOnDrop(Some(release));
        let (endpoint, request, writer, fd) = first_request(&mut case, key);
        let selection = focused(&case, Some(endpoint.id()))
            .unwrap()
            .selection
            .unwrap()
            .id;
        assert_eq!(case.fixture().text_receive_count(key, UTF8), 1);
        assert_eq!(poll_text(&case, &request), None);
        let mut writer = Some(writer);
        if !bytes.is_empty() {
            write_chunk(writer.as_ref().unwrap(), bytes);
        }
        if !leave_source_open {
            drop(writer.take());
        }
        if leave_source_open {
            assert_eq!(bytes.len(), 8);
            assert_eq!(error, Some(ClipboardError::TooLarge));
            assert!(
                writer.is_some(),
                "reader must reject limit+1 WITHOUT source EOF"
            );
        }
        retirement_entered
            .recv_timeout(WAIT)
            .expect("the real pipe reached EOF or rejected its byte cap");
        assert!(
            fd.upgrade().is_some(),
            "the exact Generic still owns A's read FD at retirement gate"
        );
        payload_busy(&case, &endpoint, &request);
        if let Some(error) = error {
            // The real pipe/decoder failure already won before this later cut.
            request.cancel();
            case.ping.ping();
            assert_eq!(
                poll_text(&case, &request),
                Some(Err(error)),
                "payload case {key}"
            );
            assert_eq!(poll_text(&case, &request), None);
        } else {
            assert_eq!(
                poll_text(&case, &request),
                None,
                "EOF only staged private text"
            );
        }
        payload_busy(&case, &endpoint, &request);
        assert_eq!(writer.is_some(), leave_source_open);
        assert_no_dnd_requests(&case, key);
        assert_eq!(
            case.fixture().text_receive_count(key, UTF8),
            1,
            "no retry or Plain fallback"
        );
        release.release();
        if error.is_none() {
            let mut outcome = None;
            drive_text(&mut case, |case| {
                outcome = poll_text(case, &request);
                if matches!(outcome, Some(Ok(_))) {
                    assert!(fd.upgrade().is_none());
                }
                outcome.is_some()
            });
            let expected = std::str::from_utf8(bytes).unwrap().to_owned();
            assert_eq!(outcome, Some(Ok(expected)), "payload case {key}");
            assert_eq!(poll_text(&case, &request), None);
            assert!(fd.upgrade().is_none());
            assert!(case.lifecycle.incoming().is_none());
        } else {
            drive_text(&mut case, |case| {
                assert_eq!(poll_text(case, &request), None);
                let incoming = case.lifecycle.incoming();
                if incoming.is_none() {
                    assert!(
                        fd.upgrade().is_none(),
                        "A's lease cannot free while its actual watcher lives"
                    );
                }
                incoming.is_none() && fd.upgrade().is_none()
            });
        }
        assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
        assert!(
            focused(&case, Some(endpoint.id()))
                .unwrap()
                .selection
                .unwrap()
                .id
                == selection
        );
        assert_eq!(writer.is_some(), leave_source_open);
        // The oversized source stays OPEN through B's actual Receive/EOF/result,
        // proving A did not need EOF to free the sole incoming-reader slot.
        finish_seven_byte_b(&mut case, &endpoint, &request, key);
        assert_eq!(writer.is_some(), leave_source_open);
        drop(writer);
        assert_eq!(case.fixture().text_receive_count(key, UTF8), 2);
        assert_eq!(case.fixture().finish_count(key), 0);
        assert_no_dnd_requests(&case, key);
        assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
        assert!(!endpoint.is_detached());
        assert_eq!(case.fixture().server.devices, 1);
        case.close();
        assert!(endpoint.is_detached());
        text_host_still_works(&mut case);
    }
}

#[test]
fn real_text_read_timeout_retires_an_open_drained_prefix_without_source_eof() {
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
        TextClipboardLimits {
            max_text_bytes: 7,
            read_timeout: Duration::from_secs(1),
            ..TextClipboardLimits::default()
        },
    );
    let mut release = OpenOnDrop(Some(release));
    let (endpoint, request, writer, fd) = first_request(&mut case, 605);
    let selection = focused(&case, Some(endpoint.id()))
        .unwrap()
        .selection
        .unwrap()
        .id;
    write_chunk(&writer, b"a\r\n");
    drive_text(&mut case, |_| {
        // This temporary Arc is released before waiting for actual timeout.
        let current = fd
            .upgrade()
            .expect("the real partial reader is still active");
        rustix::io::ioctl_fionread(current.as_ref()).unwrap() == 0
    });
    assert!(matches!(
        retirement_entered.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    assert_eq!(
        poll_text(&case, &request),
        None,
        "a drained prefix is not completion"
    );
    payload_busy(&case, &endpoint, &request);
    // No EOF, no additional bytes, no injected clock or sleeps. The real worker's
    // existing absolute deadline/tick must cut this still-open incoming read.
    retirement_entered
        .recv_timeout(WAIT)
        .expect("the actual read deadline retires the open pipe");
    assert!(fd.upgrade().is_some());
    payload_busy(&case, &endpoint, &request);
    request.cancel();
    case.ping.ping();
    assert_eq!(
        poll_text(&case, &request),
        Some(Err(ClipboardError::Timeout))
    );
    assert_eq!(
        poll_text(&case, &request),
        None,
        "later cancel cannot replace the first Timeout"
    );
    assert_eq!(case.fixture().text_receive_count(605, UTF8), 1);
    assert_no_dnd_requests(&case, 605);
    release.release();
    drive_text(&mut case, |case| {
        assert_eq!(poll_text(case, &request), None);
        let incoming = case.lifecycle.incoming();
        if incoming.is_none() {
            assert!(fd.upgrade().is_none());
        }
        incoming.is_none() && fd.upgrade().is_none()
    });
    assert!(observed.retirement_gate_released.load(Ordering::SeqCst));
    assert!(
        focused(&case, Some(endpoint.id()))
            .unwrap()
            .selection
            .unwrap()
            .id
            == selection
    );
    finish_seven_byte_b(&mut case, &endpoint, &request, 605);
    // The original source remains open through the replacement's real result.
    drop(writer);
    assert_eq!(case.fixture().text_receive_count(605, UTF8), 2);
    assert_no_dnd_requests(&case, 605);
    assert_eq!(endpoint.control().status(), ReceiveStatus::Ready);
    assert_eq!(case.fixture().server.devices, 1);
    case.close();
    assert!(endpoint.is_detached());
    text_host_still_works(&mut case);
}
