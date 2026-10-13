# Skeleton

A pulsing placeholder in the shape of content that is loading. Use it when
the shape is known before the data; for work with no shape to show, use a
[Spinner](spinner.md). `SkeletonGroup` is the same for the rows of a list.

## Import

```rust
use limn::{Component, Skeleton, SkeletonGroup};
```

## Usage

A skeleton needs an ID and the palette. Reuse the ID while rebuilding the
tree, so the pulse keeps its phase.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    match &state.title {
        Some(title) => Element::text(title),
        None => Skeleton::line("title-loading", palette, 180.0).render(),
    }
}
```

It is passive: it takes no pointer or focus and reports nothing.

## Shapes

```rust
Skeleton::line("title", palette, 180.0).render()
Skeleton::block("preview", palette, 250.0, 125.0).render()
Skeleton::circle("avatar", palette, 40.0).render()
```

`line` is one line of text, 12 px high, of the width given. `block` is a
card, an image or a preview, with a radius of 12. `circle` is an avatar or an
icon of the diameter given.

## Size and radius

```rust
Skeleton::new("bar", palette).height(20.0).radius(4.0).render()
Skeleton::new("cell", palette).width(96.0).render()
```

`Skeleton::new` is 12 px high with a radius of 6, and fills the available
horizontal space until `width` is set. `width`, `height` and `radius` also
adjust any of the shapes.

## Rows of a list

```rust
SkeletonGroup::rows("files-loading", palette, 4).icon(true).row_height(36.0).render()
```

`SkeletonGroup::rows(id, palette, count)` is `count` rows with one line each.
The lines fill different shares of the width, so they read as text.

- `row_height` is 28 unless set; match the rows that will replace these.
- `icon(true)` leads each row with an icon-sized square.
- `fractions(&[0.8, 0.5])` sets the share of the row each line fills, from 0
  to 1, repeated in order.

`into_rows()` gives the rows as a `Vec<Element>` instead of one column, for a
host that places each row itself, as in a virtualized list.

## Motion

The fill pulses between the palette's `raised` and its `hover`. In a palette
whose `hover` is its `raised`, the second color is `raised` moved an eighth
of the way to `text`. With reduced motion the fill stays at `raised` and
nothing repaints.
