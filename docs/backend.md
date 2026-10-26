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

Call `tree.set_text_layout(true)` once the backend draws `DrawCommand::Glyphs`:
every `Element::text` is then laid out by limn itself, in the faces of
`limn::fonts()`, and drawn as glyph runs, so it needs no measuring. Other text
is still measured as below until it moves too. `limn_egui::attach_fonts` turns
it on.

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
| `PasteCaptured { target, text }` | An unreleased delayed text reply bound to a previously captured editable state. |
| `ImePreedit`, `ImeCommit`, `ImeCancelled`, `ImeDeleteSurrounding` | Input method composition, in a code editor or a text field. |
| `FilesHovered { position }`, `FilesDropped { position, paths }` | Files dragged from another application. |

External file events preserve the supplied list; they do not read file
contents. The tree resolves drop targets within floating surfaces and modal
boundaries. Hosts validate targets and paths before starting file I/O.
`limn_egui::events` delivers file hover/drop after the frame's ordinary input,
so a host can apply a click or key that changes its view before admitting files.
egui groups file metadata separately; this is not an OS event chronology.
The touch wrapper can append derived touch actions afterward. Hosts must
revalidate their current view and prevent an editor submission in a frame
containing new files when their product requires preparation before sending.

`handle_event` returns the `UiAction`s the event produced. Call
`UiTree::layout` before delivering pointer events: they are resolved against
the regions of the last layout.

### Captured paste replies (unreleased)

Use `UiTree::capture_paste_target()` after resolving focus and laying out the
current view. Keep that opaque target with the host's delayed request, then
apply `UiEvent::PasteCaptured { target, text }`. The target carries no clipboard
or window resource. It is consumed once on acceptance, including an accepted
empty/sanitized-empty result or an insertion error; clones cannot repeat it.
A foreign/stale target is rejected before claiming, while busy-buffer or
pending-layout attempts do not consume it and may be retried explicitly.

Forward native focus through `UiTree::set_window_focused`, and preserve the
position of paste markers relative to navigation, clicks and host-owned keys.
Resolve earlier state changes, pending focus and layout before authorizing the
text read or acknowledging a deferred clipboard reservation. Do not pre-process
all navigation for a frame ahead of the marker. If a host changes a document,
account or endpoint association outside the tree's observations, call
`invalidate_paste_targets()` at the change, including a temporary A→B→A mapping.

An active IME composition cannot be captured; its start/cancel lifecycle cuts
old targets even without text changes. Vim mode transitions and observed
read-only/removal/modal changes also invalidate old targets. Equivalent reflow
can retain one after layout. These checks neither implement a clipboard owner
nor make a synchronous clipboard getter asynchronous. The host owns transfer
limits, reservation/ACK/completion and ordered SDK event mapping.

`PasteTo` remains available with its previous ID/focus contract. The new
`UiEvent` variant requires changes to exhaustive event matches. See
[Captured paste targets](captured-paste.md) for complete examples, shared-buffer
stamps, single-use semantics and the limits of tree observation.

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
| `FillHorizontalGradient { .. }` | A gradient from left to right, with square corners: the length of a bar that lies down. |
| `FillMesh { points, colors, indices }` | Triangles with a color at each corner, blended across each: the area under a line, a slice of a pie, a radar's shape. Its edges need no smoothing; the tree strokes them. |
| `FillTriangle { points, color }` | A triangle, used for tooltip pointers. |
| `Polyline { points, color, width, dashed }` | An open stroked path centered on its points, dashed in short marks when asked: the connectors and the decision outline of a `Flowchart`, the lifelines and messages of a `SequenceDiagram`. |
| `Shadow { rect, shadow, radius }` | A blurred shadow under a rounded rectangle. |
| `Text { position, text, color, font_size, font_family }` | One run of text from its top-left corner. |
| `Glyphs(GlyphRun)` | Glyphs limn has already shaped and placed: draw each from `run.font` at `run.size`, at its place from `run.origin`, tinted `run.color`; slant by `run.skew` degrees and thicken when `run.embolden`. Lay nothing out again. See [TextLayout](components/text-layout.md#drawing-the-glyphs). |
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
| `events_indexed(&ctx, origin)` | Unreleased indexed-input contract: input with its source index for an ordered host loop. See below. |
| `paint(ui, &frame)`, `paint_at(ui, &frame, origin)` | Paints a frame. |
| `report_ime(&ctx, &tree, origin)` | Tells the platform where an input method belongs: by the caret of the focused code editor or field, from `UiTree::ime_area()`. Each frame, after the layout. Without it egui turns input methods off. A password asks for no composition. |
| `set_image_cache_entries(n)` | How many decoded images stay as textures; a screen of thumbnails needs more than the default 16. Native decoding uses two workers per UI thread, with at most 32 queued sources and 16 MiB of queued encoded data. A full queue retries on a later paint pass. |
| `clear_image_cache()` | Release this UI thread's raster textures and queued sources, for example on sign-out. Up to two active decodes may finish; their old generation cannot publish a texture. Remove the old account's sources from the UI tree/model too, or the next paint requests them again. |

The native ImageViewer gallery's **Reload image cache** button exercises this
reset. It preserves the visible synthetic sources, so their textures are
decoded again. There is no filesystem or network I/O in the decoder. Limits
on source dimensions and decoder allocations are unchanged. Browsers retain
the existing synchronous decode path because that backend has no native
workers; clearing still releases its raster cache.
| `SurfaceFrame::from_rgb(size, rgb, scale)`, `from_rgba` | One frame for a `Surface`, converted off the UI thread. |
| `SurfaceFrame::from_image(image, scale)` | One frame already in egui's form (`ColorImage`), taken without converting it. |
| `SurfaceFrame::within(frame, at)` | Makes those pixels the rectangle at `at` of a frame of `frame` pixels the surface already shows. |
| `present_surface(&ctx, id, frame)` | Shows a frame on a surface. A frame of the size already shown is written into the same texture, and a part of one writes only its rectangle; a frame larger than the GPU's textures, or a part of a frame the surface does not show, is refused and returns `false`. |
| `forget_surface(id)` | Frees a surface's texture. |

### Indexed input

`events_indexed` returns `Vec<IndexedUiEvent>`, with public fields
`source_index: Option<usize>` and `event: UiEvent`. The host retains ownership
of clipboard requests, cancellation and application context.

Run `normalize_keyboard_input` first. `Some(i)` refers to `input.events[i]`
in the **same normalized snapshot and viewport**, not the earlier RawInput or
the physical order of operating-system input. Keep that snapshot stable during
translation and consumption; do not normalize it again or switch viewports.
Translations retain source order, and several may share one index.

An ordered host traverses **every source event in that snapshot**, including unknown events
that produce no `UiEvent`. At each index, process its associated translations
in order. Read physical_key, pressed, repeat and modifiers (including Alt)
from that source Key event, rather than a whole-frame key prepass. Apply earlier
host actions, pending focus and layout before capturing a later paste target.
Draining only translated events, or processing all markers at the end, loses
their place among navigation and clicks.

`None` identifies frame-derived input without an `egui::Event` source. RawInput
file-hover/drop metadata comes after every indexed event, as in `events`, with no
invented source index or native chronology: process it once the source loop is
done. The legacy `events` function shares the converter and discards the indices.

Deduplication changes from whole-frame searches to forward, contiguous,
non-overlapping pairs: pressed Enter followed by Text exactly `\n`, `\r` or
`\r\n` keeps the Key; pressed Ctrl/Command+C/X followed by Copy/Cut keeps the
Key and its modifiers; pressed Ctrl/Command+V followed by Paste keeps the
unchanged Paste payload and its index. Each event belongs to at most one pair.
Any intervening event, even an ignored one, breaks the pair. Reverse orders
and other combinations translate individually. This behavior change applies
to `events` too, so a later shortcut cannot suppress an earlier event across
an unknown marker.

Indices do not make every input property source-exact. Initial pointer
position and double/triple-click detection still use whole-frame pointer
state; MouseWheel can use that initial position until a preceding PointerMoved.
Copy/Cut without a Key has no source modifiers and retains the final-frame
Shift fallback. These are compatibility limits, not reconstructed per-event
click counts, modifiers or operating-system timing. `events_with_touch` and
its hold timers remain unchanged and are not part of the indexed API.

The `captured_paste` Gallery example adds `--ordered-input` to show two fixed
synthetic batches: navigation before the marker delivers to B; navigation
after capturing A rejects its late reply. The visible panels show the second
sequence's final state, with both historical results below. Use `--small` for
600 × 580; `--verify`, theme/corner flags and new-path PNG capture are shared
with the original mode. This demonstration uses a separate synthetic egui
context and the visible limn tree, without requesting clipboard I/O.

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

A surface that takes input, such as an embedded page, sits in
`Element::focus_area(id, surface)`: a press or Tab gives the region the focus
and the tree reports nothing for it. While `tree.focused_id()` is `id` the host
sends the keys, text and input method events it receives to its source
instead of `handle_event`; wherever `tree.frame().press_region(point)` is the
region it sends the pointer and the wheel, relative to the region's corner,
and keeps sending moves to it until the button that pressed there is
released. It still passes the press to the tree, which focuses the region.

A host may instead put a native view of its own where the surface is laid out,
such as an embedded browser's window, which limn cannot paint over. While
`frame.drawn_over(id)` holds, something the tree paints after the surface
lands on its visible part: a menu, a tooltip, a dialog, a panel sliding in.
The host hides its view then and presents the view's last frame in the
surface, and shows the view again once nothing covers it.

## See also

- [Concepts](concepts.md) for the model a backend serves.
- [Captured paste targets](captured-paste.md) for delayed replies and host lifetime cuts.
- [Getting started](getting-started.md) for a complete host.
