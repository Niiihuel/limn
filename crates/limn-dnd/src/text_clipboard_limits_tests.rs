use std::time::Duration;

use super::{ClipboardError, TextClipboardLimits};

type SetTimeout = fn(&mut TextClipboardLimits, Duration);

const TIMEOUT_FIELDS: [(&str, SetTimeout); 3] = [
    ("acknowledgement_timeout", |limits, value| {
        limits.acknowledgement_timeout = value;
    }),
    ("read_timeout", |limits, value| {
        limits.read_timeout = value;
    }),
    ("completion_ttl", |limits, value| {
        limits.completion_ttl = value;
    }),
];

#[test]
fn text_clipboard_defaults_match_the_public_budget() {
    let limits = TextClipboardLimits::default();

    assert_eq!(limits.max_text_bytes, 65_536);
    assert_eq!(limits.acknowledgement_timeout, Duration::from_secs(2));
    assert_eq!(limits.read_timeout, Duration::from_secs(5));
    assert_eq!(limits.completion_ttl, Duration::from_secs(2));
    assert_eq!(limits.validate(), Ok(()));
}

#[test]
fn text_clipboard_byte_limits_include_both_edges() {
    for (max_text_bytes, expected) in [
        (0, Err(ClipboardError::InvalidLimits)),
        (1, Ok(())),
        (1_048_576, Ok(())),
        (1_048_577, Err(ClipboardError::InvalidLimits)),
        (usize::MAX, Err(ClipboardError::InvalidLimits)),
    ] {
        let limits = TextClipboardLimits {
            max_text_bytes,
            ..TextClipboardLimits::default()
        };
        let before = limits;

        assert_eq!(limits.validate(), expected, "bytes={max_text_bytes}");
        assert_eq!(limits, before, "validation changed bytes={max_text_bytes}");
    }
}

#[test]
fn text_clipboard_validates_each_timeout_without_changing_the_config() {
    for (field, set_timeout) in TIMEOUT_FIELDS {
        for timeout in [
            Duration::ZERO,
            Duration::new(60, 1),
            Duration::from_secs(61),
            Duration::MAX,
        ] {
            let mut limits = TextClipboardLimits::default();
            set_timeout(&mut limits, timeout);
            let before = limits;

            assert_eq!(
                limits.validate(),
                Err(ClipboardError::InvalidLimits),
                "{field}={timeout:?}"
            );
            assert_eq!(limits, before, "validation changed {field}={timeout:?}");
        }
    }
}

#[test]
fn text_clipboard_accepts_custom_limits_at_each_timeout_edge() {
    for (field, set_timeout) in TIMEOUT_FIELDS {
        for timeout in [Duration::from_nanos(1), Duration::from_secs(60)] {
            let mut limits = TextClipboardLimits {
                max_text_bytes: 1_048_576,
                ..TextClipboardLimits::default()
            };
            set_timeout(&mut limits, timeout);
            let before = limits;

            assert_eq!(limits.validate(), Ok(()), "{field}={timeout:?}");
            assert_eq!(limits, before, "validation changed {field}={timeout:?}");
        }
    }

    let limits = TextClipboardLimits {
        max_text_bytes: 7,
        acknowledgement_timeout: Duration::from_nanos(1),
        read_timeout: Duration::from_secs(60),
        completion_ttl: Duration::from_millis(13),
    };
    let before = limits;
    assert_eq!(limits.validate(), Ok(()));
    assert_eq!(limits, before);
}
