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
| `Key { key, shift, ctrl, alt }` | A non-text key: arrows, Tab, Enter, Escape, F1 to F12, `/` for shortcuts, and so on. The tree reads `alt` only for context menu shortcuts; the host reads it for its own, such as Alt+Left. |
| `PointerBack(point)`, `PointerForward(point)` | The pointer's side buttons (mouse buttons 4 and 5). The tree ignores them; a host reads them to move through its own history. |
| `Paste(string)`, `PasteTo { id, text }` | Clipboard text, to the focused control or a named one. |
| `PasteCaptured { target, text }` | An unreleased delayed text reply bound to a previously captured editable state. |
| `ImePreedit`, `ImeCommit`, `ImeCancelled`, `ImeDeleteSurrounding` | Input method composition. |
| `FilesHovered { position }`, `FilesDropped { position, paths }` | Files dragged from another application. |

External file events preserve the supplied list; they do not read file
contents. The tree resolves drop targets within floating surfaces and modal
boundaries. Hosts validate targets and paths before starting file I/O.
`limn_egui::events_indexed` places this frame-derived metadata before ordinary
input with `source_index: None`; `events` projects the same converter. This
supersedes the RUNE integration's earlier artificial after-input ordering.
egui groups file metadata separately; neither position is an OS chronology.
The touch wrapper can append derived touch actions afterward. Hosts must
explicitly defer or reject unpositioned files, revalidate their current view,
and prevent editor submission in that frame when preparation must precede sending.

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
| `events_indexed(&ctx, origin)` | Unreleased indexed-input contract: input with its source index for an ordered host loop. See below. |
| `paint(ui, &frame)`, `paint_at(ui, &frame, origin)` | Paints a frame. |
| `GlassRenderer::new(&render_state)` | Creates the native wgpu optical renderer. Supply the backdrop using `set_backdrop_rgba` or `set_backdrop_texture`, then call its `paint_at`. |
| `GlassRenderer::paint_at_surface(ui, &frame, origin, backdrop_rect, surface_id)` | Displays the shared backdrop texture at a named surface's paint depth and renders the glass above it. This opt-in path stretches the texture to the explicit backdrop rectangle; ordinary host-fed surfaces retain their pixel mapping. See [Native glass navigation](liquid-glass.md). |
| `set_image_cache_entries(n)` | How many decoded images stay as textures; a screen of thumbnails needs more than the default 16. Native decoding uses two workers per UI thread, with at most 32 queued sources and 16 MiB of queued encoded data. A full queue retries on a later paint pass. |
| `set_color_emoji(Some(source))` | Paints the host's color pictures over the glyphs of emoji in every text run. See [Color emoji](#color-emoji). |
| `clear_image_cache()` | Release this UI thread's raster textures and queued sources, for example on sign-out. Up to two active decodes may finish; their old generation cannot publish a texture. Remove the old account's sources from the UI tree/model too, or the next paint requests them again. |

The native ImageViewer gallery's **Reload image cache** button exercises this
reset. It preserves the visible synthetic sources, so their textures are
decoded again. There is no filesystem or network I/O in the decoder. Limits
on source dimensions and decoder allocations are unchanged. Browsers retain
the existing synchronous decode path because that backend has no native
workers; clearing still releases its raster cache.
| `SurfaceFrame::from_rgb(size, rgb, scale)`, `from_rgba` | One frame for a `Surface`, converted off the UI thread. A clone shares the converted pixels: keep an animation's frames and present clones as it loops. |
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
in order. A translated `UiEvent::Key` carries Shift, Ctrl and Alt; read
physical_key, pressed, repeat and any other modifier from that source Key
event, rather than a whole-frame key prepass. Apply earlier
host actions, pending focus and layout before capturing a later paste target.
Draining only translated events, or processing all markers at the end, loses
their place among navigation and clicks.

`None` identifies frame-derived input without an `egui::Event` source. Existing
RawInput file-hover/drop metadata keeps its compatibility position before
ordinary events, with no invented source index or native chronology. The
legacy `events` function shares the converter and discards the indices.

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

### Color emoji

limn bundles only monochrome Noto Emoji. A host with color artwork installs
`set_color_emoji(Some(source))` on the UI thread, where `source` is a
`fn(&str) -> Option<&'static [u8]>`: given one extended grapheme cluster of
painted text, it returns a PNG or `None`. Skin tones, joined sequences, flags
and keycaps arrive as the one cluster they form, with any variation selector,
so the host decides presentation (for example drawing `❤️` but not `❤` or
`❤︎`). All-ASCII text is never asked about.

Nothing about the text changes: the tree measures and wraps with the font,
the glyph keeps its place and is painted transparent, and the picture is
fitted to that place, between 1 and 1.2 ems wide, centered on the line and
tinted with the text's alpha. Selection, copied text and accessibility keep
the original characters. This covers every `DrawCommand::Text`: labels,
formatted text, tooltips and editors alike.

Pictures are decoded on the UI thread, downscaled to the size drawn in steps
of 4 pixels, and cached per egui context (1024 textures). A paint pass
decodes at most 24 new ones and requests another pass for the rest, which
show their glyph meanwhile. A source that is not a PNG, is larger than
256 KiB, or exceeds 512 pixels a side keeps the glyph. Replacing or
removing the source clears the cache. The Text gallery page installs three
synthetic discs.

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

`Element::surface(..).surface_fit(fit)` places the pixels as a raster image
places its own: `SurfaceFit::Contain` scales them up or down to fit the
rectangle, centered, and `SurfaceFit::Cover` fills it and crops around the
center. Either ignores the frame's scale, so an animation can stand where its
still first frame stood, at the same size. `.radius(r)` rounds the corners of
the pixels shown in any fit. `SurfaceFit::Native`, the default, is the
one-to-one placement above.

An animation converts each frame once, into a `SurfaceFrame` it keeps, and
presents a clone of the one that is due: a clone shares the pixels, so
looping copies nothing more on the host side.

A source that knows what changed sends only that: `frame.within(size, at)`
turns the pixels of one rectangle into a part of the frame of `size` pixels
that the surface shows, and presenting it uploads that rectangle and leaves
the rest of the texture as it was. A part is refused, with `false` and no
effect, when the surface shows no frame, a frame of another size or one at
another scale; the host then sends a whole frame, which always replaces
everything. Parts are applied in the order they are presented.

## See also

- [Concepts](concepts.md) for the model a backend serves.
- [Captured paste targets](captured-paste.md) for delayed replies and host lifetime cuts.
- [Getting started](getting-started.md) for a complete host.
