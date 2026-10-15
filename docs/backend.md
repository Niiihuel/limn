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

External file events preserve the supplied list; they do not read file
contents. The tree resolves drop targets within floating surfaces and modal
boundaries. Hosts validate targets and paths before starting file I/O.

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
| `Glass { rect, radius, material, opacity }` | A rounded surface sampling an explicit host backdrop. Paint its blur, refractive edge and tint before foreground controls. A backend without a backdrop uses an opaque tint, multiplied by the command's animation opacity. |
| `FillCornerRect { rect, color, radii }` | A filled rectangle with a radius per corner. |
| `StrokeRect { rect, color, width, radius }` | An outline drawn inside the rectangle. |
| `StrokeCornerRect { .. }` | An outline with a radius per corner. |
| `FillVerticalGradient { .. }` | A vertical gradient, used for fades at scroll edges. |
| `FillHorizontalGradient { .. }` | A gradient from left to right, with square corners: the length of a bar that lies down. |
| `FillMesh { points, colors, indices }` | Triangles with a color at each corner, blended across each: the area under a line, a slice of a pie, a radar's shape. Its edges need no smoothing; the tree strokes them. |
| `FillTriangle { points, color }` | A triangle, used for tooltip pointers. |
| `Polyline { points, color, width, dashed }` | An open stroked path centered on its points, dashed in short marks when asked: the connectors and the decision outline of a `Flowchart`, the lifelines and messages of a `SequenceDiagram`. |
| `Shadow { rect, shadow, radius }` | A blurred shadow under a rounded rectangle. |
| `Text { position, text, color, font_size, font_family }` | One run of text from its top-left corner. |
| `Icon { icon, rect, tint }` | A semantic icon. The backend chooses the artwork. |
| `RasterImage { .. }` | A decoded image. |
| `Surface { rect, id, label, opacity }` | Pixels the host hands to the backend under `id`, outside the frame. They start at the rectangle's top-left corner, one pixel per physical pixel when rendered for this display, and are clipped to the rectangle, never stretched to it. |
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
| `GlassRenderer::new(&render_state)` | Creates the native wgpu optical renderer. Supply the backdrop using `set_backdrop_rgba` or `set_backdrop_texture`, then call its `paint_at`. |
| `GlassRenderer::paint_at_surface(ui, &frame, origin, backdrop_rect, surface_id)` | Displays the shared backdrop texture at a named surface's paint depth and renders the glass above it. This opt-in path stretches the texture to the explicit backdrop rectangle; ordinary host-fed surfaces retain their pixel mapping. See [Native glass navigation](liquid-glass.md). |
| `set_image_cache_entries(n)` | How many decoded images stay as textures; a screen of thumbnails needs more than the default 16. Native decoding uses two workers per UI thread, with at most 32 queued sources and 16 MiB of queued encoded data. A full queue retries on a later paint pass. |
| `clear_image_cache()` | Release this UI thread's raster textures and queued sources, for example on sign-out. Up to two active decodes may finish; their old generation cannot publish a texture. Remove the old account's sources from the UI tree/model too, or the next paint requests them again. |

The native ImageViewer gallery's **Reload image cache** button exercises this
reset. It preserves the visible synthetic sources, so their textures are
decoded again. There is no filesystem or network I/O in the decoder. Limits
on source dimensions and decoder allocations are unchanged. Browsers retain
the existing synchronous decode path because that backend has no native
workers; clearing still releases its raster cache.
| `SurfaceFrame::from_rgb(size, rgb, scale)`, `from_rgba` | One frame for a `Surface`, converted off the UI thread. |
| `SurfaceFrame::within(frame, at)` | Makes those pixels the rectangle at `at` of a frame of `frame` pixels the surface already shows. |
| `present_surface(&ctx, id, frame)` | Shows a frame on a surface. A frame of the size already shown is written into the same texture, and a part of one writes only its rectangle; a frame larger than the GPU's textures, or a part of a frame the surface does not show, is refused and returns `false`. |
| `forget_surface(id)` | Frees a surface's texture. |

### Host-fed surfaces

`Element::surface(id, label)` reserves a rectangle for pixels that change
without the tree: a remote screen, a video, an embedded page. The tree lays it
out and names it; the host decodes frames on its own thread, builds a
`SurfaceFrame` there, and calls `present_surface` on the UI thread followed by
a repaint. The frame carries the display scale its pixels were rendered for.
At the window's scale they are painted one to one from the top-left corner,
snapped to the pixel grid; a frame a pixel larger or smaller than the
rectangle is clipped or leaves a gap, and is not resampled. At another scale
it keeps its logical size. Each side is at most 8192 pixels and a frame at
most 16 megapixels. A surface holds its last frame until `forget_surface`.

A source that knows what changed sends only that: `frame.within(size, at)`
turns the pixels of one rectangle into a part of the frame of `size` pixels
that the surface shows, and presenting it uploads that rectangle and leaves
the rest of the texture as it was. A part is refused, with `false` and no
effect, when the surface shows no frame, a frame of another size or one at
another scale; the host then sends a whole frame, which always replaces
everything. Parts are applied in the order they are presented.

## See also

- [Concepts](concepts.md) for the model a backend serves.
- [Getting started](getting-started.md) for a complete host.
