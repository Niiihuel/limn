# SidebarPanel

The body of a sidebar: its blocks in a column, inset from the edges, on the
palette's `surface`.

## Import

```rust
use limn::{Component, SidebarPanel};
```

## Usage

A panel takes its blocks from top to bottom and the palette. It puts an inset
of 6 around them and a gap of 6 between them. It is passive: the blocks
report their own actions.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    SidebarPanel::new(
        vec![
            PanelHeader::new("Search", palette).render(),
            SearchField::new("query", &state.query, "Search the project…", palette)
                .size(ControlSize::Compact)
                .render(),
            results(state, palette),
        ],
        palette,
    )
    .render()
}
```

A [PanelHeader](panel-header.md) is its usual first block.

## Size

```rust
SidebarPanel::new(blocks, palette).size(320.0, 190.0).render()
```

Without `size` the panel fills whatever holds it. `size(width, height)` fixes
both; a non-finite value leaves that axis to the container.
