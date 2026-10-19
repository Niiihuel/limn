# Carousel

A shelf of items that scrolls sideways inside a page that scrolls down, under
an optional heading: the rows of a music app's home page. It has no arrow
buttons; the pointer moves it.

## Import

```rust
use limn::{Carousel, Component, MediaTile};
```

## Usage

A carousel takes an ID, the items in order and the palette. The items are
the host's own elements and report their own actions.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let tiles = state
        .albums
        .iter()
        .map(|album| MediaTile::new(format!("album={}", album.id), &album.title, palette).subtitle(&album.artist).render())
        .collect();
    Carousel::new("home-albums", tiles, palette).title("Jump back in").render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(album) = id.strip_prefix("album=") {
            state.open_album(album);
        }
    }
}
```

The row fills the width it is given and is as tall as its tallest item, with
8 px under the items for its indicator.

## Moving it

| Input | Effect |
| --- | --- |
| A drag anywhere on the row, on an item too | The row follows the pointer. A press on an item becomes a drag after 4 px of travel sideways, and the item is then not clicked. |
| A press that does not travel | Clicks the item under it, as anywhere else. |
| Letting go of a drag with speed | The row glides on and slows to rest, stopping at either end. |
| A press while it glides | Catches the row where it is, and clicks nothing. |
| Trackpad sideways, or Shift with the wheel | Pans the row. |
| The wheel | Scrolls the page; the row does not move. |

The tree keeps where the row stands under the carousel's ID, across rebuilds
and routes. `tree.horizontal_scroll_offset(id)` reads it, and
`tree.set_horizontal_scroll_offset(id, 0.0)` puts it back to the start, for
a row whose content was replaced.

## Heading

```rust
Carousel::new("mixes", tiles, palette)
    .eyebrow("Made for Ana")
    .title("Your top mixes")
    .trailing(Link::new("mixes:all", "Show all", palette).render())
    .render()
```

| Builder | What it adds |
| --- | --- |
| `title(text)` | The heading, 24 px bold on one line, cut with an ellipsis when it does not fit. |
| `eyebrow(text)` | A muted line of 14 px over the title: what the row is to the reader, over its name. |
| `leading(element)` | A picture beside the heading, held round in 48 × 48, such as the artist a row is about. |
| `trailing(element)` | An element at the far end of the heading, such as a "Show all" link. |

Without any of them the carousel is the row alone.

## Gap and inset

```rust
Carousel::new("long", tiles, palette).gap(16.0).inset(32.0)
```

`gap` is the space between items, 24 unless set. `inset` is the space before
the first item and after the last, and on both sides of the heading, zero
unless set. Give the carousel the page's full width and the page's gutter as
its inset: the first item lines up with the rest of the page, and the row
still scrolls out under the gutter to the page's edges. Both take logical
pixels; a negative value is zero and one that is not finite is ignored.

## Edges

Where items are hidden past an edge, that edge fades into the palette's
`surface` over the inset and two gaps, 48 px at the defaults. An edge with
nothing past it does not fade. Give the carousel the color of the page it
stands on:

```rust
Carousel::new("home-albums", tiles, ComponentPalette { surface: page_color, ..palette })
```

A thin indicator in the palette's muted color shows where the row stands
while it moves and fades out after it stops.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Moves through the items, as through any controls. A focused item is scrolled clear of the edges' fades. |
| Enter, Space | Activates the focused item. |

## Motion

| What | How |
| --- | --- |
| A released drag | Glides on at the pointer's speed over its last 100 ms, at most 8000 px/s, slowing as e^(-t/0.325 s) until under 20 px/s. It follows elapsed time, so it covers the same ground at any frame rate. The tree asks for a frame on every refresh while it glides. |
| The indicator | Holds for four `move_ms` after the row stops, then fades over `exit_ms`. |

Under reduced motion a released row stays where it was let go, and the
indicator hides without fading.

## Actions

The carousel reports nothing of its own: each item reports its own, such as
`UiAction::Clicked { id }` from a `MediaTile`.

## Accessibility

The title is a heading for the platform, with the eyebrow as its
description. Each item says what it is itself.
