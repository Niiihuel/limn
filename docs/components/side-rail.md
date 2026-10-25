# SideRail

A sidebar's library folded into a narrow column of pictures, for an app
whose sidebar the user can fold away: one picture for each playlist, album
or person, named by a hint at its side.

## Import

```rust
use limn::{Component, SideRail, SideRailItem};
```

## Usage

Give it the items, each with its picture and title, and render it where the
open sidebar was. The host owns whether the sidebar is folded: the button at
the rail's head reports `{id}:expand`, and the host puts the open sidebar
back.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let items = state
        .library
        .iter()
        .map(|entry| {
            SideRailItem::new(format!("open={}", entry.id), &entry.name)
                .subtitle(&entry.kind)
                .picture(Element::raster_image(entry.cover.clone()))
                .round(entry.is_artist)
        })
        .collect();
    SideRail::new("library", items, palette)
        .expand("Open Your Library")
        .create("Create a playlist")
        .current(format!("open={}", state.page))
        .playing(format!("open={}", state.playing))
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "library:expand" => state.library_folded = false,
            "library:create" => state.new_playlist(),
            id => {
                if let Some(entry) = id.strip_prefix("open=") {
                    state.open(entry);
                }
            }
        }
    }
}
```

## Geometry

`SideRail::WIDTH` is 72: a 48 px picture, 8 px of padding around it, which
the hover fill covers, and 4 px between the item and the rail's edges. The
rail is as tall as its parent makes it and paints the palette's `surface`.
Its items scroll under its head when there are more than fit; the scroll
area is `{id}:items`.

## Items

`SideRailItem::new(id, title)` is one entry. `picture(element)` is the host's
picture, held in a 48 px square with the `control` radius; without one the
item draws a muted note on the palette's `selected`. `round(true)` makes the
picture a circle, for a person. `subtitle(text)` is the muted second line of
its hint, such as "Playlist · 42 songs".

The rail draws no text. Each item is a button named by its title and
described by its subtitle, so a screen reader reads what the hint shows.

## Hints

An item's hint opens to its right as soon as the pointer is over it, with no
wait, and moves with the pointer from item to item: the rail is read by
moving along it. It shows the title and, under it, the subtitle in the muted
color. Keyboard focus shows it too.

## Head

`expand(tooltip)` heads the rail with a 40 px button holding a 24 px library
icon; `tooltip` names it, such as "Open Your Library". `create(tooltip)`
adds a round button with a plus under it. Without either the items start at
the top. `library_icon(icon)` and `create_icon(icon)` draw the host's own
glyphs in place of the library and the plus.

## Current and playing

`current(id)` keeps the item for the page shown filled with the palette's
`selected`, and announces it as selected. `playing(id)` puts a small
loudspeaker over the bottom right corner of that item's picture.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves through the head's buttons, then the items. |
| Enter, Space | Activate the focused button or item. |

## Motion

The hover fill cross-fades over `hover_ms`, and hints fade in over
`enter_ms` and out over `exit_ms`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's ID | The item is clicked or activated. |
| `UiAction::Clicked { id: "{id}:expand" }` | The button that unfolds the sidebar. |
| `UiAction::Clicked { id: "{id}:create" }` | The create button. |
