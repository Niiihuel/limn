use std::{
    cell::Cell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use raw_window_handle::{DisplayHandle, HandleError, HasDisplayHandle};

use crate::{EndpointStatus, FileDropEndpoint, ReceiveError, ReceiveLimits, WaylandDataOwner};

// Neither fixture implements HasWindowHandle or invents a native Wayland pointer.
struct UnavailableDisplay(Arc<AtomicUsize>);

impl HasDisplayHandle for UnavailableDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(HandleError::Unavailable)
    }
}

struct NonWaylandDisplay(Arc<AtomicUsize>);

impl HasDisplayHandle for NonWaylandDisplay {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        // The empty Web handle is safe to construct and borrows no foreign object.
        Ok(DisplayHandle::web())
    }
}

#[test]
fn public_owner_validates_limits_before_querying_the_display() {
    let queried = Arc::new(AtomicUsize::new(0));
    let display = Arc::new(UnavailableDisplay(queried.clone()));
    let wakes = Arc::new(AtomicUsize::new(0));
    let callback_wakes = wakes.clone();
    let limits = ReceiveLimits {
        max_files: 0,
        ..ReceiveLimits::default()
    };

    // No native handle exists in this fixture, even if validation is misplaced.
    let result = unsafe {
        WaylandDataOwner::attach_owned(display, limits, move || {
            callback_wakes.fetch_add(1, Ordering::SeqCst);
        })
    };

    assert!(matches!(result, Err(ReceiveError::InvalidLimits)));
    assert_eq!(queried.load(Ordering::SeqCst), 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn public_owner_reports_an_unavailable_display_handle() {
    let queried = Arc::new(AtomicUsize::new(0));
    let display = Arc::new(UnavailableDisplay(queried.clone()));
    let wakes = Arc::new(AtomicUsize::new(0));
    let callback_wakes = wakes.clone();

    // The fixture only returns a HandleError, so no foreign lifetime is borrowed.
    let result = unsafe {
        WaylandDataOwner::attach_owned(display, ReceiveLimits::default(), move || {
            callback_wakes.fetch_add(1, Ordering::SeqCst);
        })
    };

    assert!(matches!(result, Err(ReceiveError::Unavailable)));
    assert!(queried.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn public_owner_accepts_display_only_owners_and_send_only_wakes_without_a_fallback() {
    let queried = Arc::new(AtomicUsize::new(0));
    let display = Arc::new(NonWaylandDisplay(queried.clone()));
    let wakes = Arc::new(AtomicUsize::new(0));
    let callback_wakes = wakes.clone();
    let calls = Cell::new(0_u8); // Fn + Send, intentionally not Sync.

    // This signature also keeps the new endpoint types reachable at the crate root.
    let _endpoint_status: fn(&FileDropEndpoint) -> EndpointStatus = FileDropEndpoint::status;
    // The handle is pointer-free and non-Wayland; admission must not start a worker.
    let result: Result<Option<WaylandDataOwner>, ReceiveError> = unsafe {
        WaylandDataOwner::attach_owned(display, ReceiveLimits::default(), move || {
            calls.set(calls.get() + 1);
            callback_wakes.fetch_add(1, Ordering::SeqCst);
        })
    };

    assert!(matches!(result, Ok(None)));
    assert!(queried.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}
