//! Receive-only events and their bounded delivery boundary.

use std::{fmt, path::PathBuf};

use crate::ReceiveError;

mod control;
mod coordinator;
mod data_owner;
mod events;
mod file_drop_receiver;
#[cfg(target_os = "linux")]
mod pipe;
#[cfg(target_os = "linux")]
mod runtime;
#[cfg(target_os = "linux")]
mod wayland;
pub(crate) mod worker_pool;

pub use data_owner::{EndpointStatus, FileDropEndpoint, WaylandDataOwner};
pub use file_drop_receiver::FileDropReceiver;

#[cfg(test)]
#[path = "receive/data_owner_api_tests.rs"]
mod data_owner_api_tests;

/// An opaque offer identity, never reused by receive-only connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OfferId(u64);

/// An opaque context revision associated with newly admitted offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ReceiveEpoch(u64);

/// The consumer's decision about a complete, validated Copy drop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropDecision {
    /// Authorize a local finish attempt for this exact offer.
    Accept,
    /// Reject the offer without issuing finish.
    Reject,
}

/// Lifecycle of a receive-only connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReceiveStatus {
    /// Initialization has not completed.
    Starting,
    /// Initialization completed; context barriers may still disarm new offers.
    Ready,
    /// Closure was requested, but actual worker cleanup has not been confirmed.
    Closing,
    /// The worker and its resources have actually finished cleanup.
    Closed(Option<ReceiveError>),
}

/// A point in the originating `wl_surface` coordinate system.
///
/// These coordinates have not been converted for a consumer's DPI or zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint {
    /// Distance from the surface's left edge.
    pub x: f64,
    /// Distance from the surface's top edge.
    pub y: f64,
}

/// A receive-only event associated with one exact offer.
///
/// Events cannot be cloned. Debug output omits file names and paths.
#[non_exhaustive]
pub enum ReceiveEvent {
    /// A local-file offer is over the receiving surface.
    Hovered {
        /// The current offer identity.
        id: OfferId,
        /// The latest surface-relative pointer position.
        point: SurfacePoint,
    },
    /// A hover left the surface before its drop started.
    Left {
        /// The offer that left.
        id: OfferId,
    },
    /// A drop gesture occurred, before the URI-list payload is received.
    DropStarted {
        /// The offer associated with this gesture.
        id: OfferId,
        /// The context revision fixed when this offer entered the surface.
        epoch: ReceiveEpoch,
        /// The position frozen at the drop gesture.
        point: SurfacePoint,
    },
    /// The entire URI list reached EOF and passed validation.
    DropReady {
        /// The same offer as the corresponding Started event.
        id: OfferId,
        /// Validated paths in source order; ownership transfers to the consumer.
        paths: Vec<PathBuf>,
    },
    /// The unique terminal event for an admitted drop.
    DropEnded {
        /// The offer whose drop ended.
        id: OfferId,
        /// A local finish attempt succeeded, or the drop ended with a redacted error.
        /// Success does not acknowledge storage or transmission by libwayland,
        /// receipt by the compositor, or preparation of the files by the consumer.
        result: Result<(), ReceiveError>,
    },
}

impl fmt::Debug for ReceiveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hovered { id, point } => f
                .debug_struct("Hovered")
                .field("id", id)
                .field("point", point)
                .finish(),
            Self::Left { id } => f.debug_struct("Left").field("id", id).finish(),
            Self::DropStarted { id, epoch, point } => f
                .debug_struct("DropStarted")
                .field("id", id)
                .field("epoch", epoch)
                .field("point", point)
                .finish(),
            Self::DropReady { id, paths } => f
                .debug_struct("DropReady")
                .field("id", id)
                .field("file_count", &paths.len())
                .finish(),
            Self::DropEnded { id, result } => f
                .debug_struct("DropEnded")
                .field("id", id)
                .field("result", result)
                .finish(),
        }
    }
}
