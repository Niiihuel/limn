//! File drag and drop between a window and other applications.
//!
//! winit does not speak the Wayland data-device protocol, so a window on
//! Wayland neither receives dropped files nor can start a drag. This crate
//! attaches a second event queue to the window's own `wl_display`, the same
//! technique `smithay-clipboard` uses for the clipboard, and owns a data
//! device per seat. The compositor then reports the pointer position on every
//! motion of an incoming drag, and accepts drags started from the window.
//!
//! X11, Windows and macOS are served by winit's own file-drop events;
//! `attach` returns `None` there.
#![warn(missing_docs)]

use std::path::PathBuf;

mod receive;
mod receive_limits;
mod text_clipboard_limits;
mod uri_list;
#[cfg(target_os = "linux")]
mod wayland;

pub use receive::{
    DropDecision, EndpointStatus, FileDropEndpoint, FileDropReceiver, OfferId, ReceiveEpoch,
    ReceiveEvent, ReceiveStatus, SurfacePoint, TextPasteRequest, WaylandDataOwner,
};
pub use receive_limits::{ReceiveError, ReceiveLimits};
pub use text_clipboard_limits::{ClipboardError, TextClipboardLimits};

#[cfg(test)]
mod text_clipboard_limits_tests;

/// Positions are in logical pixels relative to the window's surface.
#[derive(Clone, Debug, PartialEq)]
pub enum DndEvent {
    /// Files from any application are over the window.
    Hovered {
        /// Pointer position from the left edge of the surface.
        x: f32,
        /// Pointer position from the top edge of the surface.
        y: f32,
    },
    /// The drag left the window without dropping.
    Left,
    /// Files were dropped on the window.
    Dropped {
        /// Last pointer position of the drag, from the left edge of the surface.
        x: f32,
        /// Last pointer position of the drag, from the top edge of the surface.
        y: f32,
        /// The local files of the drop, in the order of the source's URI list. Never empty.
        paths: Vec<PathBuf>,
    },
    /// A drag started with `start_drag` was dropped or cancelled.
    DragEnded,
}

/// A window's connection to the Wayland drag-and-drop protocol. Dropping it stops and joins
/// its worker thread.
pub struct FileDnd {
    #[cfg(target_os = "linux")]
    inner: wayland::Worker,
}

impl FileDnd {
    /// Attach to a live window. `wake` runs on a worker thread whenever an
    /// event is ready, typically to request a repaint.
    ///
    /// This legacy API borrows its handles and joins its worker synchronously on drop.
    ///
    /// # Safety
    ///
    /// On Wayland, the caller must keep the native display, connection and surface
    /// valid until dropping the returned value has finished joining its worker.
    /// Keeping only a Rust window wrapper alive does not establish that lifetime;
    /// the display's event-loop owner must not disconnect or destroy it first.
    /// Readers of the shared connection must coordinate through libwayland and its
    /// SDK; an external reader must not consume raw bytes from the display socket.
    ///
    /// Attaching borrowed native handles requires an explicit unsafe call:
    ///
    /// ```compile_fail,E0133
    /// use limn_dnd::FileDnd;
    /// use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
    ///
    /// fn attach_without_unsafe(window: &(impl HasWindowHandle + HasDisplayHandle)) {
    ///     let _ = FileDnd::attach(window, || {});
    /// }
    /// ```
    pub unsafe fn attach(
        window: &(impl raw_window_handle::HasWindowHandle + raw_window_handle::HasDisplayHandle),
        wake: impl Fn() + Send + 'static,
    ) -> Option<Self> {
        #[cfg(target_os = "linux")]
        {
            use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
            let (RawDisplayHandle::Wayland(display), RawWindowHandle::Wayland(surface)) = (
                window.display_handle().ok()?.as_raw(),
                window.window_handle().ok()?.as_raw(),
            ) else {
                return None;
            };
            // Safety: the caller keeps both borrowed handles and their connection
            // valid through the worker's synchronous join, as required above.
            let inner = unsafe {
                wayland::Worker::spawn(
                    display.display.as_ptr(),
                    surface.surface.as_ptr(),
                    Box::new(wake),
                )
            }?;
            Some(Self { inner })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (window, wake);
            None
        }
    }

    /// Events since the last call, oldest first.
    pub fn poll(&self) -> Vec<DndEvent> {
        #[cfg(target_os = "linux")]
        return self.inner.events.try_iter().collect();
        #[cfg(not(target_os = "linux"))]
        Vec::new()
    }

    /// Hand `paths` to the compositor as a drag. Call while the pointer button
    /// that started the gesture is still held; the compositor rejects a drag
    /// without a recent press on this window. From here the compositor owns
    /// the pointer: the window sees it leave, and a drop back onto the window
    /// arrives as `Dropped`.
    pub fn start_drag(&self, paths: Vec<PathBuf>) {
        #[cfg(target_os = "linux")]
        self.inner.start_drag(uri_list::encode(&paths));
        #[cfg(not(target_os = "linux"))]
        let _ = paths;
    }
}
