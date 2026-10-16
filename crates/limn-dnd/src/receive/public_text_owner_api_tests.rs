//! Public text API checks without any foreign pointer or native admission.

use super::*;
use crate::{ClipboardError, TextClipboardLimits, TextPasteRequest};

#[test]
fn public_text_policies_precede_handles_and_unavailable_or_web_remain_distinct() {
    for (receive, text) in [
        (
            ReceiveLimits {
                max_files: 0,
                ..ReceiveLimits::default()
            },
            TextClipboardLimits::default(),
        ),
        (
            ReceiveLimits::default(),
            TextClipboardLimits {
                max_text_bytes: 0,
                ..TextClipboardLimits::default()
            },
        ),
        (
            ReceiveLimits::default(),
            TextClipboardLimits {
                completion_ttl: std::time::Duration::ZERO,
                ..TextClipboardLimits::default()
            },
        ),
    ] {
        let queried = Arc::new(AtomicUsize::new(0));
        let wakes = Arc::new(AtomicUsize::new(0));
        let callback_wakes = wakes.clone();
        // This object returns only HandleError even if validation is misplaced.
        let result = unsafe {
            WaylandDataOwner::attach_owned_with_text_clipboard(
                Arc::new(UnavailableDisplay(queried.clone())),
                receive,
                text,
                move || {
                    callback_wakes.fetch_add(1, Ordering::SeqCst);
                },
            )
        };
        assert!(matches!(result, Err(ClipboardError::InvalidLimits)));
        assert_eq!(queried.load(Ordering::SeqCst), 0);
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }

    let queried = Arc::new(AtomicUsize::new(0));
    let wakes = Arc::new(AtomicUsize::new(0));
    let callback_wakes = wakes.clone();
    let unavailable = unsafe {
        WaylandDataOwner::attach_owned_with_text_clipboard(
            Arc::new(UnavailableDisplay(queried.clone())),
            ReceiveLimits::default(),
            TextClipboardLimits::default(),
            move || {
                callback_wakes.fetch_add(1, Ordering::SeqCst);
            },
        )
    };
    assert!(matches!(unavailable, Err(ClipboardError::Unavailable)));
    assert!(queried.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);

    let queried = Arc::new(AtomicUsize::new(0));
    let callback_wakes = wakes.clone();
    let calls = Cell::new(0_u8); // Fn + Send, deliberately not Sync.
                                 // DisplayHandle::web() is pointer-free and borrows no foreign event loop.
    let non_wayland: Result<Option<WaylandDataOwner>, ClipboardError> = unsafe {
        WaylandDataOwner::attach_owned_with_text_clipboard(
            Arc::new(NonWaylandDisplay(queried.clone())),
            ReceiveLimits::default(),
            TextClipboardLimits::default(),
            move || {
                calls.set(calls.get() + 1);
                callback_wakes.fetch_add(1, Ordering::SeqCst);
            },
        )
    };
    assert!(matches!(non_wayland, Ok(None)));
    assert!(queried.load(Ordering::SeqCst) > 0);
    assert_eq!(wakes.load(Ordering::SeqCst), 0);
}

#[test]
fn public_text_request_has_no_clone_or_default_and_exposes_only_opaque_operations() {
    // If either bound starts to hold, inference becomes ambiguous and this
    // consumer test no longer compiles. Copy also requires Clone in Rust.
    trait AmbiguousIfClone<A> {
        fn marker() {}
    }
    struct CloneImplemented;
    impl<T: ?Sized> AmbiguousIfClone<()> for T {}
    impl<T: Clone> AmbiguousIfClone<CloneImplemented> for T {}
    let _ = <TextPasteRequest as AmbiguousIfClone<_>>::marker;

    trait AmbiguousIfDefault<A> {
        fn marker() {}
    }
    struct DefaultImplemented;
    impl<T: ?Sized> AmbiguousIfDefault<()> for T {}
    impl<T: Default> AmbiguousIfDefault<DefaultImplemented> for T {}
    let _ = <TextPasteRequest as AmbiguousIfDefault<_>>::marker;

    let _reserve: fn(
        &WaylandDataOwner,
        &FileDropEndpoint,
    ) -> Result<TextPasteRequest, ClipboardError> = WaylandDataOwner::reserve_text_paste;
    let _ack: fn(&TextPasteRequest) -> Result<(), ClipboardError> = TextPasteRequest::acknowledge;
    let _poll: fn(&TextPasteRequest) -> Option<Result<String, ClipboardError>> =
        TextPasteRequest::poll;
    let _cancel: fn(&TextPasteRequest) = TextPasteRequest::cancel;
    // Field privacy, absent public ID/constructor and absent serialization are
    // additionally reviewed in source. No unresolved-serde-import false RED.
}
