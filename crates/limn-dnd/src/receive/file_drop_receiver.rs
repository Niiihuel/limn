//! The public receive-only boundary and its nonblocking control notifications.

use std::sync::Arc;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
#[cfg(target_os = "linux")]
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
#[cfg(target_os = "linux")]
use sctk::reexports::calloop::ping::Ping;

use super::{
    control::ReceiveControl, DropDecision, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveStatus,
};
#[cfg(target_os = "linux")]
use super::{
    runtime,
    worker_pool::{SurfaceKey, WorkerPool},
};
use crate::{ReceiveError, ReceiveLimits};

/// One bounded, receive-only connection to a window's native file-drop protocol.
///
/// This type never opens the selected files or sends their contents. It shares
/// surface exclusion and worker admission with the legacy [`crate::FileDnd`].
/// Dropping it requests closure without joining its worker on the caller.
pub struct FileDropReceiver {
    control: ReceiveControl,
    #[cfg(target_os = "linux")]
    ping: Ping,
}

impl FileDropReceiver {
    /// Attach to the exact display and surface owned by `window`.
    ///
    /// Limits are checked before querying native handles on every platform.
    /// Non-Wayland handles return `Ok(None)` without a fallback. Successful
    /// admission returns before asynchronous protocol discovery completes.
    /// `wake` runs on a service thread when events or lifecycle changes need polling.
    ///
    /// # Safety
    ///
    /// The caller must keep the native display, surface and their event-loop owner
    /// valid, and preserve coordinated SDK reading and dispatch until status is
    /// [`ReceiveStatus::Closed`]. Retaining `Arc<W>` does not by itself prove the
    /// lifetime of those foreign resources. Existing readers must coordinate through
    /// libwayland/SDK; no external reader may consume raw display-socket bytes.
    /// The receiver itself never reads or prepares a read of the shared display.
    /// Requesting close before destroying the host event loop is insufficient.
    ///
    /// ```compile_fail,E0133
    /// use std::sync::Arc;
    /// use limn_dnd::{FileDropReceiver, ReceiveLimits};
    /// use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    ///
    /// fn attach_without_unsafe<W>(window: Arc<W>)
    /// where W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static {
    ///     let _ = FileDropReceiver::attach_owned(window, ReceiveLimits::default(), || {});
    /// }
    /// ```
    pub unsafe fn attach_owned<W>(
        window: Arc<W>,
        limits: ReceiveLimits,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Option<Self>, ReceiveError>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        limits.validate()?;
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
                return Ok(None);
            };
            let key = SurfaceKey {
                display: display.display.as_ptr() as usize,
                surface: surface.surface.as_ptr() as usize,
            };
            let control = ReceiveControl::new(ReceiveEpoch(1))?;
            // The unsafe caller preserves the foreign lifetimes and reader progress.
            // The runtime retains this exact owner before enabling its worker.
            let ping = unsafe {
                runtime::spawn_owned(
                    WorkerPool::global().clone(),
                    key,
                    window,
                    control.clone(),
                    limits,
                    Box::new(wake),
                )
            }?;
            Ok(Some(Self { control, ping }))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (window, wake, display, surface);
            Ok(None)
        }
    }

    /// Drain at most six events; returned path metadata belongs to the caller.
    pub fn poll(&self) -> Vec<ReceiveEvent> {
        let events = self.control.poll();
        if events
            .iter()
            .any(|event| matches!(event, ReceiveEvent::DropEnded { .. }))
        {
            self.wake_worker();
        }
        events
    }

    /// Admit a decision for the worker; success does not establish a finish attempt.
    ///
    /// Cancel, invalidation or close may still win before the worker's atomic claim.
    /// Only `DropEnded(Ok(()))` establishes the typed local finish attempt, without
    /// proving transport, compositor receipt, file preparation or sending.
    pub fn decide(&self, id: OfferId, decision: DropDecision) -> Result<(), ReceiveError> {
        let result = self.control.decide(id, decision);
        self.wake_worker();
        result
    }

    /// Cancel the exact offer without waiting for queue space or worker progress.
    pub fn cancel(&self, id: OfferId) {
        let _ = self.control.cancel(id);
        self.wake_worker();
    }

    /// Disarm synchronously and advance the checked context epoch.
    ///
    /// The worker rearms only after the current guest-queue barrier and a new Enter.
    pub fn invalidate_pending(&self) -> Result<ReceiveEpoch, ReceiveError> {
        let result = self.control.invalidate_pending();
        self.wake_worker();
        result
    }

    /// Request closure without joining; `Closed` requires actual cleanup and join.
    pub fn close(&self) {
        let _ = self.control.close();
        self.wake_worker();
    }

    /// Observe lifecycle without treating a close request as completed cleanup.
    pub fn status(&self) -> ReceiveStatus {
        self.control.status()
    }

    fn wake_worker(&self) {
        // Every caller has released the control/header guards before notifying.
        #[cfg(target_os = "linux")]
        self.ping.ping();
    }
}

impl Drop for FileDropReceiver {
    fn drop(&mut self) {
        self.close();
    }
}
