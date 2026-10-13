# Link

A text action that hugs its label: link ink, underlined under the pointer
and with keyboard focus, and no hover fill.

## Import

```rust
use limn::{Component, Link};
```

## Usage

A link needs an ID, a label and the palette. It reports `UiAction::Clicked`
with its ID; the host opens the page or the view.

```rust
fn view(palette: ComponentPalette) -> Element {
    Link::new("docs", "Read the documentation", palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "docs" {
            state.open_docs();
        }
    }
}
```

The ink is `palette.link()`: a blue in the shade that reads on that palette,
dark or light.

## Color

```rust
Link::new("more", "Show more", palette).color(palette.muted).render()
```

`color` replaces the link ink with a color of the host's.

## Size

```rust
Link::new("terms", "Terms", palette).font_size(11.0).render()
```

`font_size` sets the label's size in pixels. The default is 13.

## Disabled

```rust
Link::new("docs", "Read the documentation", palette).enabled(false).render()
```

A disabled link is its label in the palette's muted, whatever its `color`,
in the same place. It takes no pointer or focus and reports nothing.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The link is activated by pointer, Enter or Space. |
