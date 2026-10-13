# PanelHeader

The header row of a panel: a title that takes the width left, the panel's
tools after it, and optional back and close buttons at the ends.

## Import

```rust
use limn::{Component, PanelHeaderSizing, PanelHeader};
```

## Usage

A header needs a title and the palette. The title is cut to one line before
the tools. The header decides the height, the inset, the gap and where the
tools stand; it paints no fill.

```rust
fn view(palette: ComponentPalette) -> Element {
    PanelHeader::new("Source control", palette)
        .trailing(vec![
            IconButton::new("refresh", Icon::Refresh, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .render(),
        ])
        .render()
}
```

## Tools

`trailing` takes the panel's tools as a vector of elements, placed after the
title. Each tool reports its own action. A rule between two groups is a
`Separator` with a length, in the vector:

```rust
PanelHeader::new("Source control", palette)
    .trailing(vec![
        refresh,
        Separator::vertical(palette).length(14.0).centered_in(24.0).margin(4.0).render(),
        add,
    ])
    .render()
```

## Back and dismiss

`back(id, tooltip)` puts a back arrow before the title and
`dismiss(id, tooltip)` a close button after the tools. Each reports
`UiAction::Clicked` with its ID. Their hints open downward; an empty tooltip
shows none.

```rust
fn view(palette: ComponentPalette) -> Element {
    PanelHeader::new("Commit", palette)
        .back("commit-back", "Back to changes")
        .dismiss("commit-close", "Close")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "commit-back" => state.show_changes(),
            "commit-close" => state.close_panel(),
            _ => {}
        }
    }
}
```

## An element in place of the title

```rust
PanelHeader::with_leading(
    TextField::new("address", &state.address, "Address…", palette)
        .size(ControlSize::Compact)
        .render(),
    palette,
)
    .sizing(PanelHeaderSizing { height: 36.0, ..PanelHeaderSizing::PANE })
    .render()
```

`leading` puts an element in place of the title, such as a strip of tabs, a
breadcrumb or a field. It takes the width the tools leave, and the title is
not drawn.

## Sizing

```rust
PanelHeader::new("Commit", palette).sizing(PanelHeaderSizing::CARD).render()
```

| Profile | For | Height | Inset | Gap | Title |
| --- | --- | --- | --- | --- | --- |
| `PanelHeaderSizing::SIDEBAR` | The heading of a sidebar; the default | 26 | 0 | 2 | `TextRole::Caption` |
| `PanelHeaderSizing::PANE` | The bar over a pane, as tall as a tab strip | 32 | 4 | 4 | `TextRole::Caption` |
| `PanelHeaderSizing::CARD` | The heading of a card | 28 | 2 | 4 | `TextRole::Body` |
| `PanelHeaderSizing::POPOVER` | The heading of a popover | 24 | 0 | 6 | `TextRole::Caption` |

The fields are `height`, `padding`, `gap`, `title_role` and `center_tools`.
Every profile centers the tools in the header's height;
`PanelHeaderSizing { center_tools: false, ..PanelHeaderSizing::SIDEBAR }` hangs them
from the top inset instead.

## Width

`width` fixes the header's width. A non-finite value is ignored.

## A tint behind the header

The header paints no fill, so a color behind it belongs to its container.
`Element::top_background_fade(color, height)` paints a color from the
container's top edge down that gives way to the surface below it, behind the
container's children. `bottom_background_fade` does the same from the bottom
edge.

```rust
Element::column(vec![header, body])
    .top_background_fade(cover_color, 96.0)
```

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The back arrow or the close button is clicked, with the ID given to `back` or `dismiss`. |

## Accessibility

The back and close buttons are named by the tooltips given to `back` and
`dismiss`; with an empty tooltip they have no name. With an `id` the title is
a heading the platform knows by that ID.

```rust
PanelHeader::new("Files", palette)
    .id("files-heading")
    .dismiss("files-close", "Close")
    .render()
```
