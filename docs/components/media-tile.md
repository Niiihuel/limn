# MediaTile

A tile of a grid or a shelf of albums and artists: square artwork over a
one-line title and a muted second line, all under one hover surface.

## Import

```rust
use limn::{Component, MediaTile};
```

## Usage

A tile needs an ID, a title and the palette. A click anywhere on it, artwork
or copy, reports `UiAction::Clicked` with its ID.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    MediaTile::new("album=kind-of-blue", "Kind of Blue", palette)
        .artwork(Element::raster_image(state.cover.clone()))
        .subtitle("Miles Davis")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "album=kind-of-blue" {
            state.open_album();
        }
    }
}
```

The tile hovers to the palette's `hover` and presses to its `selected` as a
whole. Its height follows the copy.

## Artwork

`artwork` takes an element the host owns and holds it in a square of the
tile's width, with the `Card` radius. A picture of another shape fills the
square when its `RasterImage` is marked `.cover()`, which crops it around its
center; otherwise it is shown whole inside the square.

Without artwork the tile draws a muted music note on the palette's
`selected`, as for a cover that has not loaded.

## Subtitle

```rust
MediaTile::new("album", "Kind of Blue", palette).subtitle("Miles Davis").render()
```

`subtitle` is a muted second line under the title. An empty one is no line at
all. Both lines are cut with an ellipsis and never widen the tile.

## Width

```rust
MediaTile::new("album", "Kind of Blue", palette).width(144.0).render()
```

`width` is the tile's width and the side of its artwork: 160 unless set and
never under 48. A non-finite value is ignored.

## Round

```rust
MediaTile::new("artist", "Nina Simone", palette)
    .subtitle("Artist")
    .round(true)
    .render()
```

`round(true)` makes the artwork a circle and centers the copy: an artist
rather than an album.

## Disabled

```rust
MediaTile::new("album", "Unavailable", palette).enabled(false).render()
```

A disabled tile is passive: a muted title, no pointer and no focus. It keeps
the layout of an enabled one, so a shelf does not move.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The tile is clicked, on its artwork or its copy. |

## Accessibility

The tile is a button named by its title and described by its second line. A
disabled tile is still there for the platform, as disabled.
