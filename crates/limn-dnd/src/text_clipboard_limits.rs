//! Resource policy for explicit Selection text-paste operations.

use std::time::{Duration, Instant};

/// Bounds for one explicitly acknowledged UTF-8 Selection transfer.
///
/// These limits do not enable a clipboard backend or authorize reading a
/// selection. Text and URI-list transfers have separate byte policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextClipboardLimits {
    /// Maximum complete text payload in bytes, from 1 through 1 MiB.
    pub max_text_bytes: usize,
    /// Absolute time from reservation to acknowledgement admission.
    pub acknowledgement_timeout: Duration,
    /// Absolute time from acknowledgement to validated result publication.
    ///
    /// This includes waiting for the worker and retiring its pipe watcher.
    pub read_timeout: Duration,
    /// Maximum time a published text result may await consumption.
    pub completion_ttl: Duration,
}

impl TextClipboardLimits {
    /// Reject unsupported byte bounds or unrepresentable phase deadlines.
    ///
    /// Durations must be positive and at most 60 seconds. Runtime transitions
    /// must independently check deadline arithmetic using their current clock.
    pub fn validate(&self) -> Result<(), ClipboardError> {
        if !(1..=1_048_576).contains(&self.max_text_bytes) {
            return Err(ClipboardError::InvalidLimits);
        }
        let now = Instant::now();
        for duration in [
            self.acknowledgement_timeout,
            self.read_timeout,
            self.completion_ttl,
        ] {
            if duration.is_zero()
                || duration > Duration::from_secs(60)
                || now.checked_add(duration).is_none()
            {
                return Err(ClipboardError::InvalidLimits);
            }
        }
        Ok(())
    }
}

impl Default for TextClipboardLimits {
    fn default() -> Self {
        Self {
            max_text_bytes: 65_536,
            acknowledgement_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(5),
            completion_ttl: Duration::from_secs(2),
        }
    }
}

/// A clipboard operation's outcome without clipboard contents or native handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClipboardError {
    /// The platform, protocol, MIME format, or current focus is unavailable.
    Unavailable,
    /// The shared incoming-transfer reservation is occupied.
    Busy,
    /// A bounded resource or non-reusable identity budget was exhausted.
    ResourceLimit,
    /// A configured byte limit or duration is unsupported.
    InvalidLimits,
    /// A runtime phase deadline cannot be represented.
    DeadlineOverflow,
    /// The seat has no current Selection offer.
    NoSelection,
    /// More than one seat has focus on the receiving endpoint.
    AmbiguousSeat,
    /// The request no longer names its original live context.
    StaleRequest,
    /// The operation is invalid in the request's current phase.
    InvalidPhase,
    /// The consumer cancelled or invalidated the request.
    Cancelled,
    /// An absolute request, transfer, or result deadline expired.
    Timeout,
    /// The complete incoming payload exceeds its text byte limit.
    TooLarge,
    /// The complete incoming payload is not valid UTF-8.
    InvalidText,
    /// A protocol or pipe operation failed.
    Io,
    /// The owner or endpoint is closing or closed.
    Closed,
}

impl std::fmt::Display for ClipboardError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "clipboard operation unavailable",
            Self::Busy => "incoming transfer reservation occupied",
            Self::ResourceLimit => "clipboard resource limit reached",
            Self::InvalidLimits => "invalid clipboard limits",
            Self::DeadlineOverflow => "clipboard deadline cannot be represented",
            Self::NoSelection => "no current clipboard selection",
            Self::AmbiguousSeat => "clipboard seat is ambiguous",
            Self::StaleRequest => "clipboard request is stale",
            Self::InvalidPhase => "invalid clipboard request phase",
            Self::Cancelled => "clipboard request cancelled",
            Self::Timeout => "clipboard deadline expired",
            Self::TooLarge => "clipboard text exceeds its byte limit",
            Self::InvalidText => "clipboard text is not valid UTF-8",
            Self::Io => "clipboard I/O failed",
            Self::Closed => "clipboard owner or endpoint is closed",
        })
    }
}

impl std::error::Error for ClipboardError {}
