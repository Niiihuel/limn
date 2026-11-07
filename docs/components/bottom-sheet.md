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
by dragging, feed actions to `BottomSheetDrag`. Held by its handle (the header
row, or a grabber strip over a headless sheet's body), the sheet follows the
finger down, and up only as a rubber band of at most 48 points. Let go past a
third of its height (kept between 60 and 180 points), or flung down at 700
points a second or more, the tree reports the handle `Dropped` with the
sheet's `dismiss_id` as its target. `BottomSheetDrag` turns that into a close,
and the sheet goes on down from where the finger left it as it leaves. Let go
short, the tree reports `DragCancelled` and the sheet springs back with a
little bounce. A drop a host synthesizes without that target still closes at
64 points down. Scrolling the body does not drag the handle. A sheet whose
content changes its height moves to the new height on the same spring rather
than jumping. Call `.reset()` when closing/replacing it by another host-owned
path.

### iOS 26's sheet

`.ios(SheetSpec { .. })` draws and moves the sheet as Apple's. The spec gives:

- the heights it rests at, smallest first: `SheetDetent::Fit` (as tall as what it holds), `Medium` (about half), `Large` and `Height(points)`;
- where the screen's safe area starts (`safe_top`), since the large height stops 10 points under it;
- the display's corner radius (`screen_corner`);
- whether what it was opened over recedes, and whether a drag may send it away.

The tree gives the sheet its size each frame:

- **Partial heights:** it floats 8 points in from the sides and foot, with corners concentric with the display's.
- **Near its large height:** it eases onto the screen's edges and its glass grows opaque. What it was opened over recedes behind it, smaller and rounded.
- **Over the keyboard** (`keyboard`, how much of the screen's foot it covers): it grows by the keyboard's height, up to its large one, keeping its sides' inset. Its content ends above the keyboard and its glass reaches down behind it. Taken by its bar, it lets go of a field's focus first, which puts the keyboard away.
- **Corners:** its glass corners are round arcs, not a pane's squircle, and its rim bends what is behind it only 12 points deep.

Its bar:

- the grabber;
- a cross that reports `dismiss_id`;
- the title, centred at 17 points semibold;
- with `.confirm(id, label, enabled)`, a prominent check.

It moves:

- **Held by its bar:** it follows the finger one to one between its heights, and past its largest as UIScrollView's rubber band.
- **Held by its body:** when the content is at its top, pulling it down moves the sheet. Short of its largest height, pushing up grows the sheet before the content scrolls.
- **Let go:** it rests at the height its speed carries it nearest to, projected as UIKit does (deceleration 0.998). If that is past its smallest height, it leaves: the tree reports `UiAction::Clicked { id: dismiss_id }` and the sheet slides on down from where it was.
- **Tapped grabber:** it steps through its heights.

Hosts that turn touch pans into `Scroll2D` events send `UiEvent::ScrollEnded` when the finger lifts.

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
