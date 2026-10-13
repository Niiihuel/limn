# Card

A raised surface that sets one group of content into the page. The host
builds the content; the card adds the padding, the hairline and the corners.

## Import

```rust
use limn::{Card, CardDensity, Component, RadiusRole};
```

## Usage

A card takes its content and the palette. It fills with the palette's
`raised` under a 1 px hairline of its `border`, and its height follows the
content. It is passive: it reports nothing.

```rust
fn view(palette: ComponentPalette) -> Element {
    Card::new(
        Element::column(vec![
            Text::new("Proposed plan", palette).render(),
            Text::new("Three steps", palette).role(TextRole::Small).muted().render(),
        ])
        .gap(6.0),
        palette,
    )
    .render()
}
```

To give a card another fill, pass a palette whose `raised` is that color.

## Density

```rust
Card::new(content, palette).density(CardDensity::Roomy).render()
```

`density` sets the padding around the content: 8 px for `Compact`, 10 for
`Regular`, which is the default, and 12 for `Roomy`. `Flush` pads nothing,
for content whose rows run to the card's edge and bring their own inset.

## Border and radius

```rust
Card::new(content, palette)
    .border(false)
    .radius(RadiusRole::Surface)
    .render()
```

`border(false)` drops the hairline. `radius` takes another radius token;
`RadiusRole::Card` is the default.

## Width

```rust
Card::new(content, palette).width(200.0).render()
```

`width` fixes the width in logical pixels. Without it the card fills the
width it is given. A negative or non-finite value is ignored.
