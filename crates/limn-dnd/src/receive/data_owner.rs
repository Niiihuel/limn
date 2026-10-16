//! Explicit connection ownership and independently retired receive surfaces.

use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
#[cfg(target_os = "linux")]
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
#[cfg(target_os = "linux")]
use sctk::reexports::calloop::ping::Ping;

use super::{
    control::ReceiveControl, worker_pool::SurfaceLease, DropDecision, OfferId, ReceiveEpoch,
    ReceiveEvent, ReceiveStatus,
};
#[cfg(target_os = "linux")]
use super::{
    runtime,
    worker_pool::{OwnerId, SurfaceKey, WorkerPool},
};
use crate::{ReceiveError, ReceiveLimits};

/// Lifecycle of one registered receiving surface, independent of its worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EndpointStatus {
    /// The surface is waiting for its first context barrier.
    Registering,
    /// Registration completed; context invalidation may still disarm offers.
    Ready,
    /// Retirement was requested, but local resources remain unconfirmed.
    Detaching,
    /// The registration's resources and surface permit have been retired.
    ///
    /// This is not proof that the connection worker has closed or joined.
    Detached(Option<ReceiveError>),
}

/// One receive-only Wayland data-device owner shared by registered surfaces.
///
/// An owner uses one worker and one device per compatible seat. Registered
/// surfaces share its bounded URI receiver and one active drop lease. The
/// process permits four workers and four registered surfaces, shared with
/// [`crate::FileDropReceiver`] and the legacy [`crate::FileDnd`].
///
/// This API discards Selection and does not bind Primary. It is not a clipboard
/// backend. The integrator must not create another clipboard/data-device owner
/// on this display in parallel. Dropping this owner requests closure without
/// joining; it does not establish that the foreign display can be destroyed.
pub struct WaylandDataOwner {
    control: ReceiveControl,
    #[cfg(target_os = "linux")]
    pool: Arc<WorkerPool>,
    #[cfg(target_os = "linux")]
    owner: OwnerId,
    #[cfg(target_os = "linux")]
    ping: Ping,
    #[cfg(target_os = "linux")]
    display: usize,
}

/// One receiving surface registered with an explicit [`WaylandDataOwner`].
///
/// Dropping the handle requests retirement without waiting or joining. The
/// pool retains the admitted window until [`EndpointStatus::Detached`], even
/// when the caller has already dropped this handle. Other surfaces and the
/// connection worker remain usable after this registration is detached.
pub struct FileDropEndpoint {
    lease: SurfaceLease,
    #[cfg(target_os = "linux")]
    ping: Ping,
}

impl WaylandDataOwner {
    /// Attach to a display owner without requiring a window handle.
    ///
    /// Limits are validated before querying handles. Unsupported platforms or
    /// non-Wayland handles return `Ok(None)` without a fallback. Admission
    /// returns before protocol discovery; `Ready` confirms devices, not that any
    /// surface has registered or any drag has been received. `wake` is a unique
    /// `Send` callback invoked on a service thread outside control guards.
    ///
    /// # Safety
    ///
    /// The caller must keep the exact native display and its event-loop owner
    /// valid, and preserve coordinated SDK reading and dispatch until this
    /// owner's status is [`ReceiveStatus::Closed`]. Retaining `Arc<W>` does not
    /// prove those lifetimes or progress. Readers must coordinate through the
    /// SDK/libwayland; no external reader may consume raw display-socket bytes.
    /// This worker never reads or prepares a read on the foreign display.
    /// Requesting close, or observing every endpoint Detached, is insufficient:
    /// the owner must reach Closed before the host event loop exits.
    pub unsafe fn attach_owned<W>(
        display_owner: Arc<W>,
        limits: ReceiveLimits,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Option<Self>, ReceiveError>
    where
        W: HasDisplayHandle + Send + Sync + 'static,
    {
        limits.validate()?;
        let display = display_owner
            .display_handle()
            .map_err(|_| ReceiveError::Unavailable)?
            .as_raw();
        #[cfg(target_os = "linux")]
        {
            let RawDisplayHandle::Wayland(display) = display else {
                return Ok(None);
            };
            let display = display.display.as_ptr() as usize;
            let pool = WorkerPool::global().clone();
            let control = ReceiveControl::new(ReceiveEpoch(1))?;
            // Admission retains this exact owner before enabling foreign IO.
            let (owner, ping) = unsafe {
                runtime::spawn_owner(
                    pool.clone(),
                    display,
                    display_owner,
                    control.clone(),
                    limits,
                    Box::new(wake),
                )
            }?;
            Ok(Some(Self {
                control,
                pool,
                owner,
                ping,
                display,
            }))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (display_owner, display, wake);
            Ok(None)
        }
    }

    /// Register a surface on this exact display without another device or worker.
    ///
    /// The global surface permit and exact window owner are retained before
    /// returning. `wake` is independent of the owner's callback. The first Ready
    /// deadline starts at admission, including time behind another barrier.
    /// Unsupported or mismatched handles return [`ReceiveError::Unavailable`];
    /// a closed owner rejects admission rather than creating a new connection.
    ///
    /// # Safety
    ///
    /// Both native handles must belong to the same live connection as this owner.
    /// The caller must preserve the exact surface until this registration reaches
    /// [`EndpointStatus::Detached`], and preserve display validity and coordinated
    /// host reading until the owner reaches [`ReceiveStatus::Closed`]. `Arc<W>`
    /// alone does not prove these obligations. The worker borrows the surface's
    /// identity without changing its userdata or event queue.
    pub unsafe fn register_surface_owned<W>(
        &self,
        window: Arc<W>,
        wake: impl Fn() + Send + 'static,
    ) -> Result<FileDropEndpoint, ReceiveError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let display = window
            .display_handle()
            .map_err(|_| ReceiveError::Unavailable)?
            .as_raw();
        let surface = window
            .window_handle()
            .map_err(|_| ReceiveError::Unavailable)?
            .as_raw();
        #[cfg(target_os = "linux")]
        {
            let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) =
                (display, surface)
            else {
                return Err(ReceiveError::Unavailable);
            };
            let display = display.display.as_ptr() as usize;
            if display != self.display {
                return Err(ReceiveError::Unavailable);
            }
            let lease = self.pool.reserve_surface(
                self.owner,
                SurfaceKey {
                    display,
                    surface: surface.surface.as_ptr() as usize,
                },
                window,
                Box::new(wake),
            )?;
            let endpoint = FileDropEndpoint {
                lease,
                ping: self.ping.clone(),
            };
            self.ping.ping();
            Ok(endpoint)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (window, display, surface, wake);
            Err(ReceiveError::Unavailable)
        }
    }

    /// Observe the worker lifecycle; Closed requires certified cleanup and join.
    pub fn status(&self) -> ReceiveStatus {
        self.control.status()
    }

    /// Cut admission and all endpoint claims synchronously, without joining.
    ///
    /// A finish claim that already won may complete its authorized local attempt.
    /// Actual cleanup and thread exit remain asynchronous; keep the host alive
    /// until [`Self::status`] reports [`ReceiveStatus::Closed`].
    pub fn close(&self) {
        #[cfg(target_os = "linux")]
        {
            self.pool.close_owner(self.owner, None);
            self.ping.ping();
        }
        #[cfg(not(target_os = "linux"))]
        let _ = self.control.close();
    }
}

impl Drop for WaylandDataOwner {
    fn drop(&mut self) {
        self.close();
    }
}

impl FileDropEndpoint {
    /// Drain at most six events; returned path metadata belongs to the caller.
    ///
    /// Retired handles retain their terminal events. Draining an old handle
    /// never acknowledges or releases a replacement registration's drop lease.
    pub fn poll(&self) -> Vec<ReceiveEvent> {
        let events = self.lease.control().poll();
        if events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
        {
            self.wake_worker();
        }
        events
    }

    /// Queue a decision for this exact offer, without making a finish claim.
    ///
    /// Cancel, invalidation, detach or owner close can still win before claim.
    /// `DropEnded(Ok(()))` records only a successful typed local finish attempt,
    /// not transport, compositor acknowledgement, file preparation or sending.
    pub fn decide(&self, id: OfferId, decision: DropDecision) -> Result<(), ReceiveError> {
        let result = self.lease.control().decide(id, decision);
        self.wake_worker();
        result
    }

    /// Cancel one exact offer without waiting for worker progress.
    pub fn cancel(&self, id: OfferId) {
        let _ = self.lease.control().cancel(id);
        self.wake_worker();
    }

    /// Disarm this endpoint and advance its checked context epoch.
    ///
    /// Rearming requires its current guest-queue barrier and a new Enter.
    pub fn invalidate_pending(&self) -> Result<ReceiveEpoch, ReceiveError> {
        let result = self.lease.control().invalidate_pending();
        self.wake_worker();
        result
    }

    /// Request local retirement without closing other surfaces or joining.
    pub fn detach(&self) {
        self.lease.request_retire();
        self.wake_worker();
    }

    /// Observe local retirement independently of the connection's worker status.
    pub fn status(&self) -> EndpointStatus {
        let control = self.lease.control();
        let header = control.header();
        if self.lease.is_detached() {
            return EndpointStatus::Detached(header.close_cause);
        }
        match header.status {
            ReceiveStatus::Starting => EndpointStatus::Registering,
            ReceiveStatus::Ready => EndpointStatus::Ready,
            ReceiveStatus::Closing | ReceiveStatus::Closed(_) => EndpointStatus::Detaching,
        }
    }

    fn wake_worker(&self) {
        // Public operations release every header guard before sending Ping.
        #[cfg(target_os = "linux")]
        self.ping.ping();
    }
}

impl Drop for FileDropEndpoint {
    fn drop(&mut self) {
        self.detach();
    }
}
