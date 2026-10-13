# Separator

A passive one-pixel rule in the palette's border color, between two regions
or two groups of tools.

## Import

```rust
use limn::{Component, Separator};
```

## Usage

A horizontal rule goes between rows, a vertical one between columns or
tools. Without a length it fills the axis it runs along.

```rust
fn view(palette: ComponentPalette) -> Element {
    Element::column(vec![
        Text::new("Above", palette).render(),
        Separator::horizontal(palette).render(),
        Text::new("Below", palette).render(),
    ])
}
```

## Length

```rust
Separator::horizontal(palette).length(96.0).render()
```

`length` fixes the rule's length instead of filling the container.

## Margin

```rust
Separator::horizontal(palette).margin(8.0).render()
```

`margin` adds empty space on both sides, across the rule: above and below a
horizontal one, left and right of a vertical one. The default is none.

## In a toolbar

```rust
Separator::vertical(palette)
    .length(16.0)
    .centered_in(28.0)
    .margin(4.0)
    .render()
```

`centered_in(height)` centers a vertical rule in a slot of that height, for
a bar taller than the rule. A horizontal rule ignores it.

## Ink

```rust
Separator::horizontal(palette).ink(palette.muted).render()
```

`ink` paints the rule in a color the host owns, such as a tree guide,
instead of the palette's border.
