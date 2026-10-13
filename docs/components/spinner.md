# Spinner

A rotating indicator for work of unknown duration. When the completed share is
known, use [Progress](progress.md).

## Import

```rust
use limn::{Component, Spinner, SpinnerVariant};
```

## Usage

A spinner needs an ID and a color. Reuse the ID while rebuilding the tree, so
the rotation does not restart.

```rust
fn view(palette: ComponentPalette) -> Element {
    Spinner::new("search-progress", palette.text).render()
}
```

It is passive: it takes no pointer or focus and reports nothing.

## Size

```rust
Spinner::new("search-progress", palette.text).size(24.0).render()
```

The spinner is a square of 16 px unless set, and never smaller than 8 px.

## Variants

```rust
Spinner::new("loading", palette.text)
    .variant(SpinnerVariant::Spokes)
    .render()
```

`Circle`, the default, is an incomplete circular stroke. `Spokes` is eight
short radial strokes.

## Motion

The rotation is continuous, not a transition, and keeps the tree animating
while the spinner is in it. With reduced motion it holds still. See
[Animation](../animation.md#keeping-the-window-painting) for how the host
keeps the window painting.
