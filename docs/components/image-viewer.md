# ImageViewer

A picture shown over the whole window, alone or as one of a gallery, zoomed
and panned by the person. Use it when a thumbnail or an attachment is opened.

## Import

```rust
use limn::{Component, ImageViewer, RasterImage, Size};
```

## Usage

The viewer wraps the root of the window. Closed, it is that root unchanged;
open, it covers the window with a dark backdrop that lets the window show
faintly through, with the picture in the middle, the title and the controls
above it and the gallery's place below. It is controlled: the host says
whether it is open and which picture it shows, and closes it when the close
action arrives. The tree keeps the zoom.

```rust
fn view(state: &State, window: Size, palette: ComponentPalette) -> Element {
    let viewer = ImageViewer::new(root(state, palette), state.viewing, "dusk.png", "viewer-close", palette)
        .viewport(window)
        .subtitle("2400 × 1600 · 1.2 MB · PNG")
        .picture_size(Size::new(2400.0, 1600.0))
        .zoom_actions("viewer-out", "viewer-fit", "viewer-in")
        .return_focus("thumbnail");
    match &state.picture {
        Some(picture) => viewer.image(picture.clone()),
        None => viewer,
    }
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "thumbnail" => state.viewing = true,
            "viewer-close" => state.viewing = false,
            _ => {}
        }
    }
}
```

The arguments are the root, whether the viewer is open, the title in its
header, the ID of its close action and the palette. `viewport` takes the size
of the window; without it the viewer assumes 900 by 620. `return_focus` names
the control that takes the focus back when the viewer closes, and the one the
picture grows out of when it opens. `initial_focus` names the control that
has the focus when it opens, the close button unless set. `subtitle` puts a
line under the title, such as the picture's size and weight.

The viewer is dark whatever the window's theme: a dark palette keeps its
colors, and a light one gives way to `ComponentPalette::neutral_dark`.

## Picture, loading and error

```rust
viewer.image(picture).picture_size(Size::new(300.0, 340.0))
viewer.placeholder(thumbnail)
viewer.error("The picture could not be decoded.")
```

`image` takes a `RasterImage`, decoded whole so it can be zoomed into, with a
checkerboard behind its transparent pixels. `picture_size` gives its own size
in pixels: the picture then fits the window without being enlarged past it,
as GNOME's Loupe, Discord and Telegram show a small picture, and one of its
pixels is a logical pixel at 100 %. Without it the picture fills the room at
its proportions.

While there is no image, `placeholder` shows a stand-in at the picture's size,
such as the thumbnail already on screen, with a spinner over it; without one
the viewer shows a spinner and a line saying the image is loading. `error`
replaces both with the host's message.

## Zoom

```rust
viewer.zoom_actions("viewer-out", "viewer-fit", "viewer-in")
viewer.zoom_level(scale, fitted)
```

The picture zooms and pans as a [zoom view](#zoom-view) does, so the wheel,
a pinch, a drag, a double click and the keys work whether or not the header
has controls. `zoom_actions` adds them: zoom out, zoom in, fit the window and
a quarter turn clockwise (for a picture, not a surface). The viewer answers
them itself; their clicks are still reported with the IDs given, the turn's
as `{close_id}:rotate`.

Each change of scale is reported as `UiAction::Zoomed { id, scale, fitted }`
and shown for a moment as a percentage at the bottom of the picture. Pass the
last one back with `zoom_level` and the header shows the percentage between
zoom out and zoom in, as a button that fits the picture or zooms in two and a
half times; zoom out and fit are then disabled while it is fitted, and zoom
in at the most. A host can also use the scale to fetch a sharper picture.

## Animation or video

```rust
viewer.surface("animation:loop", "A looping animation").picture_size(Size::new(96.0, 64.0))
viewer.footer(controls).initial_focus("video:play")
```

`surface` shows the pixels the host presents under that ID, as
`Element::surface` does, in place of a picture: an animated GIF, a video.
They are fitted and zoomed as a picture is, but do not turn. The host
presents the first frame before it opens the viewer on it and the next ones
as they fall due; see [Host-fed surfaces](../backend.md#host-fed-surfaces).
`footer` puts the host's own content under the picture, such as a video's
controls, and `initial_focus` can give its play button the focus.

## Gallery

```rust
ImageViewer::new(root, state.viewing, &state.names[state.index], "viewer-close", palette)
    .gallery(state.index, state.names.len())
    .previous_action("viewer-previous")
    .next_action("viewer-next")
    .thumbnail("viewer-show:0", Some(first), "dusk.png")
    .thumbnail("viewer-show:1", None, "tide.png")
```

`gallery(index, total)` says which picture of how many is shown. With more
than one, `previous_action` and `next_action` put an arrow at each side of
the picture; the arrow at an end of the gallery is drawn disabled. Below the
picture, `2 of 5` says where it is, or, with two or more `thumbnail`s, a
strip of them does: each cropped to a square (a plain square while it has no
picture), the shown one ringed, reporting its ID when clicked. The host
changes the index and passes the picture that goes with it.

The arrows are named "Previous image" and "Next image". For a gallery of
something else, such as the pages of a document, name them:

```rust
viewer.navigation_labels("Previous page", "Next page")
```

## Lightbox layout

```rust
viewer.layout(ViewerLayout::Lightbox)
    .lead(Element::text("Ana · 12:04"))
    .tool(menu_button)
    .scrim(Color::rgba(0, 0, 0, 232))
```

`layout` picks the arrangement: `ViewerLayout::Header`, the default and
today's look, or `ViewerLayout::Lightbox`, as a chat's media viewer draws it.
A lightbox has:

- the `lead` at the top left, such as who sent the picture and when, at most
  184 pixels wide; without one, the title is there. The title is the
  dialog's name either way;
- one floating group at the top right: the zoom toggle first, then the host's
  actions, with `tool` last: a control of the host's own, such as a button
  that opens a menu;
- close alone in the top right corner, and chevron arrows at the sides that
  wrap around;
- thumbnails under the picture, 40 pixels square, with no "2 of 5".

The zoom toggle draws `Icon::ZoomIn` while the picture fits and
`Icon::ZoomOut` while it is zoomed. It is drawn for a still picture only: a
surface, such as an animation or a video, has none. A click on a still picture
zooms it two and a half times about the point, and the next click fits it
again, as the toggle does about the middle; a double click counts as one click.
A surface does not zoom on a click;
its wheel, pinch, drag and keys still zoom and move it, as in a zoom view.
While the picture is zoomed in, the arrows and the thumbnails hide; the group
stays, so a keyboard user who reached its toggle can zoom back out, and close
and the lead stay. `scrim` sets the backdrop's color, 232 black unless set, in
either layout.

The margins are 24 pixels from the sides and 36 from the top and the bottom.
The glyphs are 16 pixels, as an icon button's always are.

## Host actions

```rust
viewer
    .action("viewer-copy", Icon::Copy, "Copy image")
    .action("viewer-save", Icon::ArrowDown, "Save as…")
```

Each `action` is a round icon button of the host's own, as large as the close
button, with its label as name and tooltip, and it reports its ID with
`UiAction::Clicked` when clicked or activated by the keyboard, for example to
save the picture shown. Each call adds one, in order; an empty ID adds nothing.

With `zoom_actions` the header is a toolbar, and the actions sit in it before
the zoom controls; in `ViewerLayout::Lightbox` they sit in its group. Otherwise,
and in a `lightbox(true)`, the close button leads them in one column just
outside the picture's top-right corner, 6 pixels from it. When the right of the
picture has no room the column moves inside that corner instead. The column is
a non-modal popover anchored to the picture, so it takes no backdrop of its own.

## Keyboard

| Key | Does |
| --- | --- |
| Left, Right | Reports the previous or the next action, while there is a picture that way |
| `+`, `=`, `-` | Zooms in or out by 1.5, about the middle |
| `0` | Fits the picture |
| `1`, `2`, `3` | Shows it at 100, 200 or 300 % |
| Ctrl + an arrow | Moves a zoomed picture by 15 % of the window |
| Ctrl+R, Ctrl+Shift+R | Turns a picture a quarter clockwise, or back |
| Escape | Reports the close action |

The zoom keys work wherever the focus is, unless a text field has it.

## Pointer

| Gesture | Does |
| --- | --- |
| Mouse wheel | Zooms by 1.3 a notch about the pointer, in 120 ms |
| Ctrl with a touchpad's scroll, a pinch | Zooms about the fingers as they move |
| A touchpad's scroll | Moves a zoomed picture |
| Drag | Moves a zoomed picture; let go moving, it glides on and slows down |
| Double click on the picture | Zooms to two and a half times its fit about the pointer, and back |
| Click beside the picture | Reports the close action |

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the close ID | The close button, Escape, or a click beside the picture or on the backdrop. |
| `UiAction::Clicked { id }` with the previous or next ID | An arrow is clicked, or Left or Right is pressed. |
| `UiAction::Clicked { id }` with a thumbnail's ID | That thumbnail is clicked. |
| `UiAction::Clicked { id }` with a zoom ID | One of the zoom controls is clicked; the viewer has already zoomed. |
| `UiAction::Zoomed { id, scale, fitted }` | The picture's scale changed, by any of the above. |
| `UiAction::Clicked { id }` with a host action's ID | That control is clicked or activated. |

## Origin

```rust
viewer.origin(thumbnail_rect)
```

`origin` is the rectangle the picture comes from, in window coordinates. The
picture grows from it to its place over 350 ms, its corner going from 8 to its
own (6 in a lightbox), and the scrim and the rest fade in; closing runs it back
in 250 ms, and the viewer stays on screen until the picture has returned. It
takes the place of the growth out of the `return_focus` control. Without an
origin a `lightbox(true)` fades over 180 ms. Under reduced motion, as
`UiTree::set_reduced_motion` sets it, both happen at once.

## Lightbox

```rust
viewer
    .lightbox(true)
    .natural_size(1200.0, 800.0)
    .caption("dusk.png")
```

`lightbox(true)` shows the picture as a lightbox, as a pasted image is shown in
a desktop chat app: the window dims behind it to 62.5 % black, and there is no
panel, no header and no count for one picture. The picture keeps its aspect
ratio, is at most 640 pixels wide and the window's height less 64, and is never
enlarged past `natural_size`. Its close control and each `action` stand in a
column just outside its top-right corner, 6 pixels from it, aligned with its
top; when the window has no room to its right, the column moves inside that
corner, 6 pixels in. `caption` is the white line under the picture, and a
gallery of more than one adds its count under that. A click on the backdrop or
on the picture closes, as Escape does. With `origin`, the picture grows from its
thumbnail to its fitted place. Zoom is not offered in a lightbox.

`natural_size(width, height)` is the picture's size in pixels, which the host
knows: `RasterImage` does not carry it. Without it the picture's box is the
whole of the room (640 wide, the window's height less 64) and the picture is
fitted inside that. It is used only in lightbox mode; the panel already shows
the picture whole at its own proportions.

## Accessibility

The viewer is a dialog named by its title, known to the platform as
`{close_id}:dialog`. Every control is a button with a name from the
library's labels, such as "Close image", and one that cannot be used is
still there as disabled. The picture is an image named by its text
alternative and described by its scale; a thumbnail is a button named by its
label, selected when it is the one shown.

## Zoom view

```rust
Element::zoom_view("viewer:view:dusk", Element::raster_image(picture.full_resolution()))
    .zoom_content(Size::new(2400.0, 1600.0))
    .zoom_control("viewer-in", ZoomControl::In)
    .zoom_dismiss("viewer-close")
    .zoom_opened_from("thumbnail")
    .shadow(palette.shadow, Point::new(0.0, 12.0), 40.0, 0.0)
```

What the viewer zooms with, for a host that lays a picture out itself. The
element fills its box; the picture, a raster image or a surface, is drawn
filling the rectangle its zoom gives it, clipped to the box. The tree keeps
the zoom under the ID, which also names a picture: under a new ID a picture
opens fitted, and a view that is not laid out is forgotten.

- **Fit.** The whole picture, centered, never enlarged past `zoom_content`
  unless `zoom_upscale(most)` allows it. A fitted view follows the box when
  the window changes size; a zoomed one keeps the pixel at the middle of the
  box there.
- **Limits.** The scale goes from the fit (or the picture's own size, when
  that is smaller) to `ZOOM_VIEW_MAX_SCALE`, 20 (2000 %, as Loupe and Zed),
  and a zoom within a quarter of a step of the fit rests on it. A side of the
  picture smaller than the box is centered; a larger one always covers it,
  so the picture is never lost off the box.
- **Motion.** A control, a key or a double click zooms in 200 ms (Loupe's
  and Telegram's), a wheel notch in 120 ms, keeping the pixel under the
  pointer where it is all the way; a fling slows as e^(−t/0.25 s), GTK's
  deceleration. The first time it is laid out, a view with
  `zoom_opened_from` grows out of that control's place. Under reduced motion
  everything happens at once.
- **Controls.** `zoom_control(id, control)` makes a click reported with that
  ID move the view too: `ZoomControl::In`, `Out`, `Fit`, `Actual`, `Toggle`,
  `RotateClockwise`, `RotateCounterclockwise`. `zoom_dismiss(id)` reports a
  click beside the picture as that ID.
- **Clicks.** `zoom_click_toggles()` makes a click on the picture zoom it two
  and a half times about the point, and the next click fit it again, as a
  lightbox does; a double click zooms it once, not in and back out. A drag
  still pans, and the wheel zooms as it does without the option. Without the
  option a single click does nothing, and a double click zooms.
- **Shadow.** The element's `shadow` is cast by the picture, not by the box.
- **Reading it.** `UiTree::zoom_scale(id)` gives the scale where it rests;
  `UiAction::Zoomed` reports each change; the scale shows as a percentage at
  the bottom of the box for a moment after it changes.

The wheel, a pinch and a press reach the topmost zoom view in the surface on
top, unless a scrolling area or a field lies over it there; the keys reach
the last one laid out.
