# GlassMaterial

A refracting surface over a scene you own. Use it for floating navigation,
player controls, action menus and sheets. Keep foreground text separate
from the page backdrop; a selection lens captures its own track and labels.

## Import

```rust
use limn::{Component, Element, GlassMaterial, Switch};
use limn_egui::GlassRenderer;
```

## Usage

```rust
let material = GlassMaterial::new(limn::Color { a: palette.surface.a / 2, ..palette.surface });
let surface = Element::panel(Element::text("Now playing"))
    .glass(material)
    .radius(limn::tokens().radius.surface);
```

Choose a translucent tint from your theme. An opaque tint hides the source.
Create one `GlassRenderer` from eframe's wgpu render state, then supply a
sampleable scene with `set_backdrop_texture`, or render an ordinary page
with `set_backdrop_frame`. Paint the resulting frame with `paint_at` rather
than the ordinary fallback painter. The built
[`glass_qa` example](../../crates/limn-egui/examples/glass_qa.rs) shows the
complete offscreen rendering and capture path.

## Lenses

```rust
let control = Switch::new("playback:normalize", enabled, palette)
    .track_size(64.0, 28.0)
    .thumb_inset(2.0)
    .thumb_width(40.0)
    .glass(material.blur(0.0).magnify(1.1))
    .render();
```

`Switch::glass` makes its thumb a lens. The control remains controlled:
handle its click and rebuild the checked value. Pressing or dragging lifts
the thumb, and releasing settles it. Use `ink` to supply your theme's on
color.

For tabs or a segmented control, apply `glass_selection(material)` to its
rendered element. The lens samples a small local capture containing the
track and labels, excluding the lens itself. Its travel retains velocity
when interrupted. Lift and deformation settle independently of position.
A capsule radius keeps the selection's ends circular.

## Floating navigation

`FloatingGlassBar` accepts `FloatingGlassTab` items, a search control and an
optional player element. Search remains a separate circle. Keep one
`FloatingBarMotion` in host state, observe content movement with `scroll_by`
(positive means down), and pass its `value` to `collapse`. Repaint while
`is_active` returns true. The same player subtree moves between expanded
and inline positions, retaining its action IDs.

## Menus and sheets

`GlassMenu` reuses dropdown focus, keyboard navigation, action IDs and
dismissal. Supply the trigger's laid-out rectangle as its morph origin.
Dismissal reports `{id}:dismiss`. For a held row, wrap the row in
`ContextMenu::new(...).open_action(...)`, handle that request by opening a
`GlassMenu`, and use the row's bounds as its origin. The touch input bridge
already cancels a hold that turns into a scroll.

`Modal::glass` and `BottomSheet::glass` apply the material to the floating
shell. They leave the base page and scrim separate.

## Motion

`UiTree::set_reduced_motion(true)` snaps built-in motion. Pass the same
preference to host-owned `FloatingBarMotion` updates. Use theme motion
tokens for application changes. Keep IDs stable across rebuilds.

## Limits

A callback cannot sample egui's window surface. An external video must
supply a sampleable GPU texture or join an explicitly composited scene.
`set_backdrop_frame` rejects surfaces and nested glass rather than silently
capturing an incomplete page. The local selection capture uses the tint
fallback for a glass track; it does not recursively render optical layers.
The shader currently uses one radius for all corners, including a sheet's
bottom corners. See [Liquid glass](../liquid-glass.md) for the reference
analysis, integration steps, captures and measured frame costs.
