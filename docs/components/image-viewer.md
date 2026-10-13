# ImageViewer

A picture shown over the whole window, alone or as one of a gallery, with
optional zoom. Use it when a thumbnail or an attachment is opened.

## Import

```rust
use limn::{Component, ImageViewer, RasterImage, Size};
```

## Usage

The viewer wraps the root of the window. Closed, it is that root unchanged;
open, it floats a panel over it on a dark scrim. It is controlled: the host
says whether it is open and which picture it shows, and closes it when the
close action arrives.

```rust
fn view(state: &State, window: Size, palette: ComponentPalette) -> Element {
    let viewer = ImageViewer::new(root(state, palette), state.viewing, "dusk.png", "viewer-close", palette)
        .viewport(window)
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
of the window, and the panel is that size less a margin of 8 on each side;
without it the viewer assumes 900 by 620. `return_focus` names the control
that takes the focus back when the viewer closes.

## Picture, loading and error

```rust
viewer.image(picture)
viewer.error("The picture could not be decoded.")
```

`image` takes a `RasterImage` and shows it whole, at its own proportions, in
the room the panel leaves. Without one the viewer shows a spinner and a line
saying the image is loading. `error` replaces both with the host's message.

## Gallery

```rust
ImageViewer::new(root, state.viewing, &state.names[state.index], "viewer-close", palette)
    .gallery(state.index, state.names.len())
    .previous_action("viewer-previous")
    .next_action("viewer-next")
```

`gallery(index, total)` says which picture of how many is shown, and the
footer reads `2 of 5`. With more than one, `previous_action` and
`next_action` put an arrow at each side of the picture. The arrow at an end
of the gallery is drawn disabled. The host changes the index and passes the
picture that goes with it.

## Zoom

```rust
viewer.zoom(state.zoom).zoom_actions("viewer-out", "viewer-fit", "viewer-in")
```

`zoom` is how many times larger than its fit the picture is shown, from 1 to
`ImageViewer::MAX_ZOOM`, which is 8; a level outside that range is brought
into it. Past 1 the picture is panned: the wheel moves it down, and a drag or
a sideways gesture moves it across. `zoom_actions` adds three controls to the
header, to zoom out, to fit the window and to zoom in. The host owns the
level and answers each control by passing another `zoom`. Zoom out and fit
are disabled at 1, and zoom in at the maximum.

## Keyboard

| Key | Does |
| --- | --- |
| Left, Right | Reports the previous or the next action, while there is a picture that way |
| Escape | Reports the close action |

The close button has the focus when the viewer opens.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the close ID | The close button, Escape, or a click on the scrim. |
| `UiAction::Clicked { id }` with the previous or next ID | An arrow is clicked, or Left or Right is pressed. |
| `UiAction::Clicked { id }` with a zoom ID | One of the three zoom controls is clicked. |

## Accessibility

The panel is a dialog named by its title, known to the platform as
`{close_id}:dialog`. Every control is a button with a name from the
library's labels, such as "Close image", and one that cannot be used is
still there as disabled.
