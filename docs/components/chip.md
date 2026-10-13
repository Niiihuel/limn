# Chip

A passive pill for a count or a label: a tag, a status, a plan name. It takes
no pointer or focus and has no dismiss button.

## Import

```rust
use limn::{Chip, Component};
```

## Usage

A chip needs a label and the palette. Place it in a row, so its width follows
its text.

```rust
fn view(palette: ComponentPalette) -> Element {
    Element::row(vec![
        Chip::new("rust", palette).render(),
        Chip::new("draft", palette).render(),
    ])
    .gap(8.0)
}
```

The fill is `palette.selected` with a border of `palette.border`. A label
that is a single digit is drawn as a circle.

## Outline

```rust
Chip::new("Pro plan", palette).outline().render()
```

`outline` draws the border only, with no fill, for a label that should sit
quietly on its surface.

## Height

```rust
Chip::new("3", palette).height(18.0).render()
```

A chip is 22 px high unless set, and never lower than 16 px.

## Related

`Avatar` and `Kbd` are the other passive labels shown beside it in the
gallery: an account photo or the initial of a name, and the keycaps of a
shortcut.
