//! Explicit connection ownership and independently retired receive surfaces.

use std::{sync::Arc, time::Instant};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
#[cfg(target_os = "linux")]
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
#[cfg(target_os = "linux")]
use sctk::reexports::calloop::ping::Ping;

#[cfg(target_os = "linux")]
use super::{
    clipboard_control::ClipboardControl,
    runtime,
    worker_pool::{OwnerId, SurfaceKey, WorkerPool},
};
use super::{
    clipboard_control::{from_receive_error, TextRequestControl},
    control::ReceiveControl,
    worker_pool::SurfaceLease,
    DropDecision, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveStatus,
};
use crate::{ClipboardError, ReceiveError, ReceiveLimits, TextClipboardLimits};

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
/// surfaces share its bounded URI receiver and one incoming reservation. The
/// process permits four workers and four registered surfaces, shared with
/// [`crate::FileDropReceiver`] and the legacy [`crate::FileDnd`].
///
/// [`Self::attach_owned`] discards Selection. Explicitly opt into incoming text
/// with [`Self::attach_owned_with_text_clipboard`]. Neither mode binds Primary or
/// publishes copied text/images; this is not a complete clipboard backend. The
/// integrator must not create another clipboard/data-device owner on this display
/// in parallel. Dropping this owner requests closure without
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
    #[cfg(target_os = "linux")]
    clipboard: Option<ClipboardControl>,
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

/// One opaque, single-use reservation for incoming Selection text.
///
/// It retains only control metadata and a lightweight worker signal, not a
/// native display/window, protocol proxy, callback or internal text body owner.
/// Dropping it cancels the reservation without waiting for cleanup or joining.
/// The owner and registered endpoint have independent native lifetime duties.
pub struct TextPasteRequest {
    request: TextRequestControl,
    #[cfg(target_os = "linux")]
    ping: Ping,
}

impl std::fmt::Debug for TextPasteRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TextPasteRequest { .. }")
    }
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
                clipboard: None,
            }))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (display_owner, display, wake);
            Ok(None)
        }
    }

    /// Attach the same bounded owner with incoming Selection text enabled.
    ///
    /// Both policies are validated before querying display handles or admitting
    /// resources. Unsupported platforms and non-Wayland handles return `Ok(None)`;
    /// invalid handles return [`ClipboardError::Unavailable`]. Admission failures
    /// retain their exact category, including Busy and ResourceLimit.
    ///
    /// This mode uses the owner's existing worker, guest queue and one device per
    /// seat. It retains Selection before surface registration and observes native
    /// keyboard focus without processing keys or keymaps. File drops and text
    /// share one incoming reservation. Primary and copy publication are absent;
    /// installing a full clipboard backend remains the integrator's responsibility.
    /// `wake` runs on a service thread outside control guards.
    ///
    /// # Safety
    ///
    /// The exact display and its coordinated host reader must remain valid and
    /// make progress until this owner reaches [`ReceiveStatus::Closed`], which
    /// requires certified cleanup and actual worker join. Retaining `Arc<W>`,
    /// dropping requests, requesting close or detaching every endpoint does not
    /// prove those obligations. No external reader may consume raw display-socket
    /// bytes. Registered surfaces must additionally obey
    /// [`Self::register_surface_owned`]'s lifetime requirements.
    pub unsafe fn attach_owned_with_text_clipboard<W>(
        display_owner: Arc<W>,
        receive: ReceiveLimits,
        text: TextClipboardLimits,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Option<Self>, ClipboardError>
    where
        W: HasDisplayHandle + Send + Sync + 'static,
    {
        receive.validate().map_err(from_receive_error)?;
        text.validate()?;
        let display = display_owner
            .display_handle()
            .map_err(|_| ClipboardError::Unavailable)?
            .as_raw();
        #[cfg(target_os = "linux")]
        {
            let RawDisplayHandle::Wayland(display) = display else {
                return Ok(None);
            };
            let display = display.display.as_ptr() as usize;
            let pool = WorkerPool::global().clone();
            let control = ReceiveControl::new(ReceiveEpoch(1)).map_err(from_receive_error)?;
            // The same admission retains this exact display before foreign SDK IO.
            let (owner, ping, clipboard) = unsafe {
                runtime::spawn_owner_with_text(
                    pool.clone(),
                    display,
                    display_owner,
                    control.clone(),
                    receive,
                    text,
                    Box::new(wake),
                )
            }?;
            Ok(Some(Self {
                control,
                pool,
                owner,
                ping,
                display,
                clipboard: Some(clipboard),
            }))
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (display_owner, display, wake);
            Ok(None)
        }
    }

    /// Reserve the current focused Selection for this exact registered endpoint.
    ///
    /// Reservation performs no clipboard read, pipe creation or protocol Receive.
    /// The caller captures its destination before acknowledging the returned
    /// request. The owner/endpoint, current context barrier, native seat focus,
    /// Selection and deadlines must remain valid through publication and polling.
    /// An old or foreign endpoint cannot act on this owner's current reservation.
    /// Text-disabled owners return [`ClipboardError::Unavailable`].
    pub fn reserve_text_paste(
        &self,
        endpoint: &FileDropEndpoint,
    ) -> Result<TextPasteRequest, ClipboardError> {
        #[cfg(target_os = "linux")]
        {
            let clipboard = self.clipboard.as_ref().ok_or(ClipboardError::Unavailable)?;
            let control = endpoint.lease.control();
            let registration = control.registration().ok_or(ClipboardError::StaleRequest)?;
            let request = clipboard.reserve(&control, registration, Instant::now())?;
            Ok(TextPasteRequest {
                request,
                ping: self.ping.clone(),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = endpoint;
            Err(ClipboardError::Unavailable)
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

impl TextPasteRequest {
    /// Queue the one read acknowledgement after the host captures its destination.
    ///
    /// This validates metadata and signals the worker; it does not make a
    /// synchronous SDK call. A duplicate acknowledgement is InvalidPhase, and
    /// cancellation/context loss may still prevent a read or successful delivery.
    pub fn acknowledge(&self) -> Result<(), ClipboardError> {
        let result = self.request.acknowledge(Instant::now());
        self.wake_worker();
        result
    }

    /// Transfer one complete, freshly validated raw UTF-8 result, or one error.
    ///
    /// `None` means pending or already consumed. An empty String is a successful
    /// result. CRLF is preserved; no trimming, lossy decoding or normalization is
    /// performed. Success is unavailable until the actual reader watcher is gone
    /// and expires at the configured completion TTL even without UI polling.
    /// An invalid poll returns metadata only; its internal body is retired on the
    /// worker. A successfully returned String belongs to the caller thereafter.
    pub fn poll(&self) -> Option<Result<String, ClipboardError>> {
        let result = self.request.poll(Instant::now());
        self.wake_worker();
        result
    }

    /// Cut this exact reservation without blocking, joining or releasing it early.
    ///
    /// An already authorized local Receive attempt may finish, but cancellation
    /// prevents later delivery. Native watcher/body retirement remains on worker.
    pub fn cancel(&self) {
        self.request.cancel();
        self.wake_worker();
    }

    fn wake_worker(&self) {
        // Every internal operation has returned and released its guards first.
        #[cfg(target_os = "linux")]
        self.ping.ping();
    }
}

impl Drop for TextPasteRequest {
    fn drop(&mut self) {
        self.cancel();
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
