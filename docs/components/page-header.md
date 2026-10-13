# PageHeader

The heading of a page: a title, an optional description, and elements the
host owns before and after them.

## Import

```rust
use limn::{Component, PageHeader, PageHeaderSizing};
```

## Usage

A header needs a title and the palette. It is passive: the elements the host
puts in it report their own actions.

```rust
fn view(palette: ComponentPalette) -> Element {
    PageHeader::new("Providers", palette)
        .description("Connected accounts and the models they offer.")
        .trailing(Button::new("refresh", "Refresh", palette).render())
        .render()
}
```

The title is in the palette's text color and the description under it in the
muted color. Both wrap unless `truncate` is set.

## Leading and trailing

```rust
PageHeader::new("Marketplace", palette)
    .leading(IconButton::new("back", Icon::ArrowLeft, palette).render())
    .trailing(Button::new("refresh", "Refresh", palette).render())
    .render()
```

`leading` puts an element before the copy, such as a back button, and
`trailing` one after it at its own width. Both are centered on the header.

`trailing_top(true)` keeps the trailing element on the title's line instead
of centering it, for a header whose description runs over several lines.

## Stacked

```rust
PageHeader::new("Providers", palette)
    .trailing(Button::new("refresh", "Refresh", palette).render())
    .stacked(content_width < 300.0)
    .render()
```

`stacked(true)` moves the trailing element under the copy, for a narrow page.

## Truncate

`truncate(true)` cuts the title and the description to one line each instead
of wrapping them.

## Sizing

```rust
const CATALOG: PageHeaderSizing = PageHeaderSizing {
    title_size: 20.0,
    height: 40.0,
    ..PageHeaderSizing::DEFAULT
};

PageHeader::new("Marketplace", palette).truncate(true).sizing(CATALOG).render()
```

A `PageHeaderSizing` holds the header's density in logical pixels:

| Field | Sets | `DEFAULT` |
| --- | --- | --- |
| `title_size` | Font size of the title | 18 |
| `description_size` | Font size of the description | 11 |
| `copy_gap` | Space between the title and the description | 4 |
| `row_gap` | Space between the leading element, the copy and the trailing element | 12 |
| `stacked_gap` | Space between the copy and a trailing element under it | 8 |
| `height` | Fixed height of the header while it is a row; zero follows the copy | 0 |

`PageHeaderSizing::DEFAULT` is the heading of a settings page.
