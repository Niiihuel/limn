# TextShimmer

A line of text with a highlight moving across it, for work in progress that
has a name: "Reading files", "Thinking".

## Import

```rust
use limn::{Component, TextShimmer};
```

## Usage

A shimmer needs an ID, its text and the palette. Reuse the ID while
rebuilding the tree, so the highlight does not start again.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    TextShimmer::new("request-status", &state.status, palette).render()
}
```

The text is muted and the highlight is the palette's text color. It is 13 px
sans text, takes no focus and reports nothing. The text can be selected and
copied like any other.

## Colors

```rust
TextShimmer::new("build-status", "Waiting for the build", palette)
    .colors(palette.muted, accent)
    .render()
```

`colors(base, highlight)` sets the color of the text and of the highlight
that passes over it.

## Motion

The highlight is paint only: the text is laid out once and does not move.
With reduced motion the highlight does not move and nothing repaints, and a
shimmer scrolled out of view does not repaint either.
