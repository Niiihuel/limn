# PageColumn / CardGrid

The layout of a page, in two types: `PageColumn` is a scrolling page whose
content is a centered column of bounded width, and `CardGrid` lays cards of
equal width in rows inside it. Both are passive.

## Import

```rust
use limn::{CardGrid, Component, PageColumn};
```

## Usage

A `PageColumn` takes an ID for its scroll position, the width available to
the page and the palette. Build it, read `content_width()` to size the
content, then pass the content with `children`.

```rust
fn view(available: f32, palette: ComponentPalette) -> Element {
    let page = PageColumn::new("automations", available, palette);
    let width = page.content_width();
    page.children(vec![
        PageHeader::new("Automations", palette).render(),
        SectionHeader::new("Templates", palette).count(5).render(),
        templates(width, palette),
    ])
    .render()
}
```

The children are the sections of the page, top to bottom. The page fills its
parent and scrolls on its own. [PageHeader](page-header.md) and
[SectionHeader](section-header.md) are its usual headings.

## Column width and gutter

```rust
PageColumn::new("settings", available, palette)
    .max_width(720.0)
    .gutter(14.0, 28.0, 640.0)
    .gap(16.0)
```

`max_width` is the widest the column grows (960 unless set) and `min_width`
the narrowest it shrinks, even when the page is narrower (0 unless set).
`gutter(narrow, wide, breakpoint)` sets the padding around the column:
`narrow` on a page under `breakpoint` wide and `wide` from there on, 14 and
28 at 640 unless set. `gap` is the space between sections, 10 unless set.

`gutter_width()` returns the padding in force at the page's width, and
`content_width()` the width of the column after the gutter and the bounds.

## Scrollbar

```rust
PageColumn::new("settings", available, palette)
    .reserve_rail(true)
    .tint_scrollbar(false)
```

`reserve_rail(true)` keeps a scrollbar rail's width out of the column before
the page overflows, so the content keeps its width when the scrollbar
arrives. The scrollbar is painted in the palette's muted color;
`tint_scrollbar(false)` leaves it the scroll area's own.

## A grid of cards

A `CardGrid` takes its width and a breakpoint. Read `item_width()`, build
each card at that width, then pass the cards with `items` in reading order.

```rust
fn templates(width: f32, palette: ComponentPalette) -> Element {
    let grid = CardGrid::new(width, 360.0);
    let item_width = grid.item_width();
    grid.items(
        ["Daily summary", "Review", "Release notes"]
            .into_iter()
            .map(|name| Card::new(Text::new(name, palette).render(), palette).width(item_width).render())
            .collect(),
    )
    .render()
}
```

At the breakpoint or wider the grid has two columns; below it, one. The gap
between cards is 8, and a short last row keeps its cards in their columns
instead of stretching them. The cards here are [Card](card.md), but any
element built at `item_width()` will do.

## Columns

```rust
CardGrid::new(width, 360.0).columns(3)
```

`columns` sets the number of columns on a wide page: 2 unless set and at
least 1. Set it before reading `item_width()`.
