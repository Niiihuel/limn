use super::{bytes, decode_received};
use crate::{ReceiveError, ReceiveLimits};
use std::{path::PathBuf, time::Duration};

fn decode(data: &[u8]) -> Result<Vec<PathBuf>, ReceiveError> {
    decode_received(data, &ReceiveLimits::default())
}

fn assert_path_bytes(actual: Vec<PathBuf>, expected: &[&[u8]]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(bytes(actual).as_slice(), *expected);
    }
}

#[test]
fn defaults_keep_generic_receive_budgets_and_absolute_phase_timeouts() {
    let limits = ReceiveLimits::default();
    assert_eq!(limits.max_list_bytes, 64 * 1024);
    assert_eq!(limits.max_files, 32);
    assert_eq!(limits.max_path_bytes, 4096);
    assert_eq!(limits.max_total_path_bytes, 64 * 1024);
    assert_eq!(limits.startup_timeout, Duration::from_secs(5));
    assert_eq!(limits.transfer_timeout, Duration::from_secs(5));
    assert_eq!(limits.decision_timeout, Duration::from_secs(2));
}

#[test]
fn accepts_local_files_comments_and_line_separators_in_source_order() {
    assert_path_bytes(
        decode(b"# source comment\r\n\r\nfile:///tmp/first\nfile://localhost/tmp/second\rfile:///tmp/third\r\n").unwrap(),
        &[b"/tmp/first", b"/tmp/second", b"/tmp/third"],
    );
}

#[test]
fn decodes_once_and_preserves_literal_path_bytes_without_normalization() {
    assert_path_bytes(
        decode(b"file:///tmp//%252e/foo../.../a%20b/%23%3F%25%5C\r\n").unwrap(),
        &[b"/tmp//%2e/foo../.../a b/#?%\\"],
    );
}

#[cfg(unix)]
#[test]
fn preserves_percent_escaped_non_utf8_unix_names() {
    assert_path_bytes(
        decode(b"file:///tmp/%FF%FE.bin\n").unwrap(),
        &[b"/tmp/\xff\xfe.bin"],
    );
}

#[test]
fn rejects_an_empty_effective_list() {
    for data in [b"".as_slice(), b"\r\n\n", b"# comment\r\n"] {
        assert_eq!(decode(data), Err(ReceiveError::InvalidUriList));
    }
}

#[test]
fn rejects_the_whole_batch_when_any_entry_is_non_local_or_relative() {
    for invalid in [
        "https://example.invalid/a",
        "file://server/share/a",
        "file://localhost.evil/tmp/a",
        "file:relative",
        "/tmp/not-a-uri",
    ] {
        let data = format!("file:///tmp/valid\r\n{invalid}\r\n");
        assert_eq!(
            decode(data.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn rejects_malformed_percent_escapes_instead_of_treating_them_as_names() {
    for invalid in ["file:///tmp/%", "file:///tmp/%2", "file:///tmp/%zz"] {
        let data = format!("file:///tmp/valid\n{invalid}\n");
        assert_eq!(
            decode(data.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn rejects_decoded_nul_and_control_bytes() {
    for invalid in ["%00", "%01", "%09", "%0A", "%0D", "%7F"] {
        let data = format!("file:///tmp/{invalid}name\n");
        assert_eq!(
            decode(data.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn rejects_raw_control_bytes_inside_a_uri() {
    assert_eq!(
        decode(b"file:///tmp/a\tb\n"),
        Err(ReceiveError::InvalidUriList)
    );
}

#[test]
fn rejects_authority_credentials_ports_and_unescaped_query_or_fragment() {
    for invalid in [
        "file://user@localhost/tmp/a",
        "file://localhost:80/tmp/a",
        "file:///tmp/a?download=true",
        "file:///tmp/a#fragment",
    ] {
        assert_eq!(
            decode(invalid.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn rejects_raw_and_percent_escaped_dot_segments() {
    for invalid in [
        "/tmp/./a",
        "/tmp/../a",
        "/tmp/%2e/a",
        "/tmp/%2E%2e/a",
        "/tmp/.%2E/a",
        "/tmp/%2e./a",
        "/tmp/.",
        "/tmp/%2e%2e",
    ] {
        let data = format!("file://{invalid}\n");
        assert_eq!(
            decode(data.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn encoded_separators_cannot_hide_dot_segments() {
    for invalid in ["/tmp/link%2F..%2Fchosen", "/tmp/link%2f.%2fchosen"] {
        let data = format!("file://{invalid}\n");
        assert_eq!(
            decode(data.as_bytes()),
            Err(ReceiveError::InvalidUriList),
            "{invalid}"
        );
    }
}

#[test]
fn rejects_duplicate_decoded_paths_instead_of_importing_the_same_file_twice() {
    for data in [
        b"file:///tmp/a\nfile:///tmp/a\n".as_slice(),
        b"file:///tmp/a\nfile://localhost/tmp/%61\n",
    ] {
        assert_eq!(decode(data), Err(ReceiveError::InvalidUriList));
    }
}

#[test]
fn uri_payload_byte_limit_includes_comments_and_separators() {
    let data = b"# counted\r\nfile:///tmp/a\r\n";
    let mut limits = ReceiveLimits {
        max_list_bytes: data.len(),
        ..ReceiveLimits::default()
    };
    assert_path_bytes(decode_received(data, &limits).unwrap(), &[b"/tmp/a"]);
    limits.max_list_bytes -= 1;
    assert_eq!(decode_received(data, &limits), Err(ReceiveError::TooLarge));
}

#[test]
fn rejects_one_file_beyond_the_configured_count_without_a_valid_prefix() {
    let limits = ReceiveLimits {
        max_files: 2,
        ..ReceiveLimits::default()
    };
    assert_path_bytes(
        decode_received(b"file:///a\nfile:///b\n", &limits).unwrap(),
        &[b"/a", b"/b"],
    );
    assert_eq!(
        decode_received(b"file:///a\nfile:///b\nfile:///c\n", &limits),
        Err(ReceiveError::TooMany),
    );
}

#[test]
fn decoded_path_limit_counts_bytes_rather_than_uri_escape_characters() {
    let data = b"file:///tmp/%C3%B1\n";
    let mut limits = ReceiveLimits {
        max_path_bytes: 7,
        ..ReceiveLimits::default()
    };
    assert_path_bytes(decode_received(data, &limits).unwrap(), &[b"/tmp/\xc3\xb1"]);
    limits.max_path_bytes = 6;
    assert_eq!(decode_received(data, &limits), Err(ReceiveError::TooLarge));
}

#[test]
fn aggregate_decoded_path_limit_rejects_the_complete_batch_at_plus_one() {
    let data = b"file:///a\nfile:///bc\n";
    let mut limits = ReceiveLimits {
        max_total_path_bytes: 5,
        ..ReceiveLimits::default()
    };
    assert_path_bytes(decode_received(data, &limits).unwrap(), &[b"/a", b"/bc"]);
    limits.max_total_path_bytes = 4;
    assert_eq!(decode_received(data, &limits), Err(ReceiveError::TooLarge));
}

#[test]
fn rejects_zero_or_excessive_resource_limits_before_decoding() {
    let defaults = ReceiveLimits::default();
    let invalid = [
        ReceiveLimits {
            max_list_bytes: 0,
            ..defaults
        },
        ReceiveLimits {
            max_list_bytes: 1024 * 1024 + 1,
            ..defaults
        },
        ReceiveLimits {
            max_files: 0,
            ..defaults
        },
        ReceiveLimits {
            max_files: 257,
            ..defaults
        },
        ReceiveLimits {
            max_path_bytes: 0,
            ..defaults
        },
        ReceiveLimits {
            max_path_bytes: 64 * 1024 + 1,
            ..defaults
        },
        ReceiveLimits {
            max_total_path_bytes: 0,
            ..defaults
        },
        ReceiveLimits {
            max_total_path_bytes: usize::MAX,
            ..defaults
        },
    ];
    for limits in invalid {
        assert_eq!(
            decode_received(b"file:///a\n", &limits),
            Err(ReceiveError::InvalidLimits),
            "{limits:?}"
        );
    }
}

#[test]
fn rejects_zero_or_excessive_phase_deadlines_before_decoding() {
    let defaults = ReceiveLimits::default();
    for duration in [Duration::ZERO, Duration::from_secs(61), Duration::MAX] {
        for limits in [
            ReceiveLimits {
                startup_timeout: duration,
                ..defaults
            },
            ReceiveLimits {
                transfer_timeout: duration,
                ..defaults
            },
            ReceiveLimits {
                decision_timeout: duration,
                ..defaults
            },
        ] {
            assert_eq!(
                decode_received(b"file:///a\n", &limits),
                Err(ReceiveError::InvalidLimits),
                "{limits:?}"
            );
        }
    }
}
