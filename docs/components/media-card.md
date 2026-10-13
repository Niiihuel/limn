# MediaCard

A track as a card: its record sleeve beside its title and artist, over a tint
the host takes from the artwork. The sleeve is the control that plays it.

## Import

```rust
use limn::{Component, MediaCard};
```

## Usage

A card needs an ID, a title and the palette. It is controlled and keeps no
state: the sleeve reports `UiAction::Clicked` with the card's ID, and the host
answers by playing or pausing and passing `playing`.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    MediaCard::new("track", "Kind of Blue", palette)
        .artist("Miles Davis")
        .playing(state.playing)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "track" {
            state.playing = !state.playing;
        }
    }
}
```

Only the sleeve takes the pointer, not the whole card.

## Artist

`artist` is a muted line under the title, cut to one line. An empty one is no
line at all. The title fades at its end when it does not fit.

## Artwork and tint

```rust
MediaCard::new("track", "Kind of Blue", palette)
    .artwork(state.cover.clone())
    .tint(state.cover_color)
    .render()
```

`artwork` takes a `RasterImage` for the sleeve, cropped to its square. `tint`
is the card's fill, a `Color` the host takes from the artwork; without it the
card is the palette's `raised`. The fill darkens towards the foot of the card
so the copy reads on any tint.

## Mark

```rust
MediaCard::new("track", "Kind of Blue", palette)
    .mark(Element::icon(Icon::Audio).foreground(palette.muted))
    .render()
```

`mark` puts an element in the top corner at the end of the card: a mark, or a
link out.

## Width

`width` sets the card's width: 325 unless set and never under 220. A
non-finite value is ignored. A long title does not widen the card.

## Disabled

```rust
MediaCard::new("track", "Unavailable", palette).enabled(false).render()
```

A disabled card is passive: its title is muted, its sleeve takes no pointer
and its record stays in, even with `playing(true)`.

## Motion

While `playing` is true the record is out of its sleeve and turning, one turn
every three seconds. When it stops the record goes back in and keeps the angle
it reached. Under the pointer the record comes out without turning. With
reduced motion a playing record shows out and holds still.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The sleeve is clicked. `id` is the card's ID. |
