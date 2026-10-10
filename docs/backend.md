# Backend contract

What a renderer must do to host limn. Read this to write a backend for a
toolkit other than egui, or to understand what `limn-egui` does.

The library has no dependency on any window system or graphics API. It turns
an element tree into a `Frame`; a backend paints the frame and feeds input
back.

## Responsibilities

| The backend must | Through |
| --- | --- |
| Tell the tree how wide text is | `set_text_measurer`, `set_text_height_measurer`, `set_metrics`, `set_sans_metrics` |
| Translate input into events | `UiTree::handle_event(UiEvent)` |
| Paint the frame's commands in order | `Frame::commands` |
| Repaint while something is animating | `UiTree::animation_repaint_after()` |

## Text measurement

Layout depends on text widths, so the tree must measure with the same fonts
the backend paints with. Register them once, and again if the fonts change:

```rust
tree.set_metrics(mono_metrics);       // monospace, at 14 px
tree.set_sans_metrics(sans_metrics);  // UI font, at 14 px
tree.set_text_measurer(|text, size, family| width_in_logical_pixels);
tree.set_text_height_measurer(|size, family| line_height);
```

Without a measurer the tree estimates from the calibrated metrics.
`limn_egui::attach_fonts(&mut tree, &ctx)` does all of this for egui.

## Events

`UiEvent` is the input vocabulary. Positions are logical pixels relative to
the tree's origin.

| Event | When |
| --- | --- |
| `PointerMove(point)`, `PointerLeave` | The pointer moved, or left the window. |
| `PointerDown(point)`, `PointerUp(point)` | Primary button. |
| `PointerDownModified { position, shift }` | Primary button with Shift. |
| `PointerDownEditor { position, shift, alt, ctrl, click_count }` | Primary button in a code editor, with modifiers and click count. |
| `ContextMenu { position }`, `ContextMenuKeyboard` | Secondary button, or the keyboard shortcut that opens a context menu. |
| `Scroll { position, delta_y }`, `ScrollDirect { .. }`, `Scroll2D { .. }` | Wheel, precise touchpad, and two-axis scrolling. |
| `Text(string)` | Typed text. |
| `Key { key, shift, ctrl }` | A non-text key: arrows, Tab, Enter, Escape, and so on. |
| `Paste(string)`, `PasteTo { id, text }` | Clipboard text, to the focused control or a named one. |
| `ImePreedit`, `ImeCommit`, `ImeCancelled`, `ImeDeleteSurrounding` | Input method composition. |
| `FilesHovered { position }`, `FilesDropped { position, paths }` | Files dragged from another application. |

`handle_event` returns the `UiAction`s the event produced. Call
`UiTree::layout` before delivering pointer events: they are resolved against
the regions of the last layout.

## The frame

`UiTree::layout(size)` returns a `Frame`.

- `frame.commands` is the list of `DrawCommand`s, painted in order.
- `frame.regions` is the list of `HitRegion`s: the ID, rectangle and kind of
  every interactive area. A backend does not need it to paint; tests and
  tooling do.

### Draw commands

| Command | Paints |
| --- | --- |
| `FillRect { rect, color, radius }` | A filled rectangle with uniformly rounded corners. |
| `FillCornerRect { rect, color, radii }` | A filled rectangle with a radius per corner. |
| `StrokeRect { rect, color, width, radius }` | An outline drawn inside the rectangle. |
| `StrokeCornerRect { .. }` | An outline with a radius per corner. |
| `FillVerticalGradient { .. }` | A vertical gradient, used for fades at scroll edges. |
| `FillTriangle { points, color }` | A triangle, used for tooltip pointers. |
| `Shadow { rect, shadow, radius }` | A blurred shadow under a rounded rectangle. |
| `Text { position, text, color, font_size, font_family }` | One run of text from its top-left corner. |
| `Icon { icon, rect, tint }` | A semantic icon. The backend chooses the artwork. |
| `RasterImage { .. }` | A decoded image. |
| `Spinner { .. }`, `ProgressRing { .. }` | Vector progress indicators. |
| `SelectionContour { .. }`, `CornerMask { .. }` | Text selection outlines and rounded clipping of images. |
| `EffortSlider { .. }` | A specialized slider track. |
| `PushClip(rect)`, `PopClip` | Begin and end a clipping rectangle. |

Rules a renderer must follow:

- **Order is depth.** Later commands paint over earlier ones.
- **Clips nest.** A `PushClip` inside another clips to their intersection.
- **Colors are straight alpha**, not premultiplied.
- **Sizes are logical pixels.** Apply the display scale when rasterizing.
- **Icons are 16 px** unless the command's rectangle says otherwise.

## The frame loop

```rust
tree.set_root(view(&state, size));
tree.layout(size);
for event in input {
    for action in tree.handle_event(event) {
        update(&mut state, &action);
    }
}
tree.set_root(view(&state, size));
let frame = tree.layout(size);
paint(&frame);
if let Some(after) = tree.animation_repaint_after() {
    schedule_repaint(after);
}
```

## `limn-egui`

The egui backend exposes:

| Function | Purpose |
| --- | --- |
| `install_fonts(&ctx)` | Installs the UI font. Once, at window creation. |
| `attach_fonts(&mut tree, &ctx)` | Makes a tree measure with egui's fonts. Once per tree. |
| `normalize_keyboard_input(&ctx)` | Fixes key and text events on remapped layouts. Each frame, before `events`. |
| `events(&ctx, origin)` | This frame's input as `UiEvent`s. |
| `paint(ui, &frame)`, `paint_at(ui, &frame, origin)` | Paints a frame. |
| `set_image_cache_entries(n)` | Size of the decoded image cache. |

## See also

- [Concepts](concepts.md) for the model a backend serves.
- [Getting started](getting-started.md) for a complete host.
