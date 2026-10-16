//! Configuration and redacted failures for receive-only file drops.

use std::time::{Duration, Instant};

/// Resource limits for a receive-only file-drop connection.
///
/// Byte limits count the incoming URI list and decoded Unix path bytes, not
/// characters. Each limit must be positive. The supported ceilings are 1 MiB
/// per list, 256 files, 64 KiB per path, and 1 MiB for all decoded paths.
/// Timeouts must be positive, at most 60 seconds, and fit an absolute deadline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReceiveLimits {
    /// Maximum URI-list payload, including comments and line separators.
    pub max_list_bytes: usize,
    /// Maximum number of files in one complete drop.
    pub max_files: usize,
    /// Maximum decoded byte length of one absolute path.
    pub max_path_bytes: usize,
    /// Maximum sum of decoded path byte lengths in one drop.
    pub max_total_path_bytes: usize,
    /// Absolute deadline for initializing the receive-only connection.
    pub startup_timeout: Duration,
    /// Absolute deadline for reading one complete URI list.
    pub transfer_timeout: Duration,
    /// Absolute deadline for the consumer's decision after a validated drop.
    pub decision_timeout: Duration,
}

impl ReceiveLimits {
    /// Reject unsupported resource limits or unrepresentable phase deadlines.
    pub fn validate(&self) -> Result<(), ReceiveError> {
        if !(1..=1024 * 1024).contains(&self.max_list_bytes)
            || !(1..=256).contains(&self.max_files)
            || !(1..=64 * 1024).contains(&self.max_path_bytes)
            || !(1..=1024 * 1024).contains(&self.max_total_path_bytes)
        {
            return Err(ReceiveError::InvalidLimits);
        }
        let now = Instant::now();
        for timeout in [
            self.startup_timeout,
            self.transfer_timeout,
            self.decision_timeout,
        ] {
            if timeout.is_zero()
                || timeout > Duration::from_secs(60)
                || now.checked_add(timeout).is_none()
            {
                return Err(ReceiveError::InvalidLimits);
            }
        }
        Ok(())
    }
}

impl Default for ReceiveLimits {
    fn default() -> Self {
        Self {
            max_list_bytes: 64 * 1024,
            max_files: 32,
            max_path_bytes: 4096,
            max_total_path_bytes: 64 * 1024,
            startup_timeout: Duration::from_secs(5),
            transfer_timeout: Duration::from_secs(5),
            decision_timeout: Duration::from_secs(2),
        }
    }
}

/// A receive-only failure that never contains a file name, path, or URI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiveError {
    /// The platform or required protocol is unavailable.
    Unavailable,
    /// A temporary worker, surface, or offer reservation is occupied.
    Busy,
    /// A non-renewable process resource budget has been exhausted.
    ResourceLimit,
    /// The configured limits or deadlines are unsupported.
    InvalidLimits,
    /// The complete URI list is empty, malformed, ambiguous, or non-local.
    InvalidUriList,
    /// The URI list or decoded paths exceed a byte limit.
    TooLarge,
    /// The complete list contains more files than permitted.
    TooMany,
    /// The consumer rejected or cancelled the operation.
    Cancelled,
    /// An absolute phase deadline expired.
    Timeout,
    /// A protocol or pipe I/O operation failed.
    Io,
    /// The offer ID is no longer current.
    StaleOffer,
    /// The requested decision is invalid in the current phase.
    InvalidPhase,
    /// The receive-only connection is closing or closed.
    Closed,
}
