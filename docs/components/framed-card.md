# FramedCard

A card that stands on a frame a shade apart from it, with a foot on the
frame under the card. Use it when a status and an action belong to the whole
card and not to its last row.

## Import

```rust
use limn::{Component, FramedCard};
```

## Usage

A card is built from parts, top to bottom: `section` for content that keeps
to the card's inset, `rows` for rows whose hover fill reaches past it, and
`foot` for what sits on the frame below.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    FramedCard::new(palette)
        .width(320.0)
        .section(Element::text("Production"))
        .rows(FramedCard::row(
            "branch:fill",
            Element::clickable("branch", Element::row(vec![Element::text("Branch")])),
            ["branch"],
            palette,
        ))
        .foot(Element::text("Deployed 2 minutes ago"))
        .render()
}
```

The card reports nothing of its own: the controls the host puts in it report
their actions. Without `width` the card fills the space it is given; a width
is never under 64. Without `foot` the frame closes under the card.

## Sections and space

`section(element)` adds content inset by `FramedCard::INSET` (16) from the
component's edge: a header, a figure, a paragraph. `space(height)` leaves
room between two parts.

## Rows

```rust
FramedCard::row(
    "region:fill",
    Element::row(vec![Element::clickable("region", label).flex(1.0), more_button]),
    ["region", "region-more"],
    palette,
)
```

`FramedCard::row(id, content, actions, palette)` builds one row,
`FramedCard::ROW_HEIGHT` (30) high. Its content starts at the same inset as
a section, and its hover fill reaches `FramedCard::REACH` (6) past the
content on each side. `actions` are the IDs of the controls inside the row:
hovering or pressing any of them paints the one fill. `id` names that fill
and must differ from the action IDs.

`rows(element)` adds a row, or a block of them, to the card. Two `rows`
calls one after the other are kept `FramedCard::ROW_GAP` (2) apart, so their
fills never touch.

## A row that opens a list

```rust
FramedCard::block(
    "sources-list",
    FramedCard::row("sources:fill", sources_row, ["sources"], palette),
    state.sources_open.then(|| list),
    palette,
)
```

`FramedCard::block(id, row, listed, palette)` is a row and what it opens to
list. While `listed` is `Some` the two share one fill and read as one block.
Render the block on every rebuild and pass `listed` only while the row is
open, so the list opens and closes in place. Pass the block to `rows`.

## On a surface of the host's

```rust
let (body, foot) = FramedCard::new(palette).flat().section(header).rows(list).foot(status).into_parts();
```

`flat()` is for a host whose surface already is the card, such as a popover:
no frame and no card are drawn, and the parts keep the same insets from the
host's edge. Call it before adding parts. `into_parts()` then gives the body
and the foot apart, for a host that scrolls one and pins the other;
`render()` gives the body with the foot under it.
