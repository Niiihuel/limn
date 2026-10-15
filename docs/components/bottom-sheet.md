# BottomSheet

`BottomSheet` is a controlled modal for phone actions and short forms. It reuses
`Modal`'s scroll composition and `Popover`'s backdrop, focus scope, restoration,
interruptible transitions and reduced-motion behavior. It meets the bottom of
the host's viewport, with full width on phones and a preferred width of 560 on
larger windows. It leaves at least eight logical pixels clear above it.

## Import

```rust
use limn::{BottomSheet, BottomSheetDrag, Component, Element};
```

## Usage

```rust
// Keep this beside the host's `open` state, rather than rebuilding it per frame.
let mut drag = BottomSheetDrag::new("close_actions");

// Rebuild this tree from the host's current state each frame.
let view = BottomSheet::new(base, open, "Song actions", "", "close_actions", palette)
    .body(actions)
    .max_height(500.0)
    .return_focus("song_more")
    .render();

// Apply each action produced by UiTree to the controlled state.
if drag.handle_action(&action) {
    open = false;
}
```

The visible close button has a 44-point target. The title names the accessible
dialog; an optional description is both visible in the scrolling body and spoken. The header and
optional `.footer(...)` stay in place; only `.body(...)` scrolls, under the ID
`modal_body:{dismiss_id}`. `.max_height(...)` caps the whole sheet without
forcing short content to fill it (the requested cap is kept at least 240).
The title, drag target and close control share a compact header, so a short
keyboard-reduced viewport gives space to the fixed footer first.
`.initial_focus(...)` overrides the default
close-control focus; `.return_focus(...)` can name a stable control when a
temporary opening menu item disappears.

Escape, tapping the close control or handle, and tapping the scrim request the
same `UiAction::Clicked { id: dismiss_id }`. `.backdrop_dismiss(false)` keeps
outside taps from closing while still blocking the underlying view. To dismiss
by dragging, feed actions to `BottomSheetDrag`: a downward handle release of
at least 64 logical pixels closes, while short, upward, sideways or cancelled
drags do not. Scrolling the body does not drag the handle. The handle uses
limn's existing in-place drag API: the sheet stays in its layout while held,
then closes through the shared overlay transition. It does not follow the
finger. Call `.reset()` when closing/replacing it by another host-owned path.

The viewport supplied to `UiTree` should exclude system safe areas. If a host
deliberately uses a full-screen viewport instead, `.safe_area_bottom(inset)`
reserves extra content space above the home indicator. Leave it at zero when
the viewport is already safe to avoid counting the inset twice.

The [`bottom_sheet` example](../../crates/limn-egui/examples/bottom_sheet.rs)
shows a complete host, scrollable actions and dismissal handling:

```sh
cargo run -p limn-egui --example bottom_sheet
# Capture a phone-width native render, using reduced motion for a stable frame.
cargo run -p limn-egui --example bottom_sheet -- 390 /tmp/bottom-sheet.png
```
