# StatusBar

The bar at the foot of a window: cells of status, of which the trailing ones
give way by priority when the window is narrow.

## Import

```rust
use limn::{Component, DropdownMenuItem, Icon, StatusBar, StatusBarItem, StatusBarItemSizing, TooltipSpec};
```

## Usage

A bar takes its leading cells, its trailing cells and the palette. A
`StatusBarItem` takes an ID, a width and the palette, and reports
`UiAction::Clicked` with its ID.

```rust
fn view(state: &State, width: f32, palette: ComponentPalette) -> Element {
    StatusBar::new(
        vec![StatusBarItem::new("branch", 64.0, palette).icon(Icon::Github).label("main").strong()],
        vec![
            StatusBarItem::new("cursor", 76.0, palette).label("Ln 12, Col 4").priority(2),
            StatusBarItem::new("encoding", 44.0, palette).label("UTF-8").passive().priority(1),
        ],
        palette,
    )
    .width(width)
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "cursor" {
            state.open_go_to_line();
        }
    }
}
```

The host passes every width: the bar adds up the widths it is given and
measures no text. The bar is `StatusBar::HEIGHT` (21) high and its cells are
`StatusBar::GAP` (4) apart.

## Cell content

```rust
StatusBarItem::new("problems", 56.0, palette)
    .tinted_icon(Icon::ErrorCircle, error_color)
    .count(2)
    .tinted_icon(Icon::WarningTriangle, warning_color)
    .count(7)
    .sizing(StatusBarItemSizing::COUNTS)
```

`icon`, `label` and `count` are shown in the order they are called; `icon` and
`count` add one more each time. A label is muted, takes the width left and
fades where it does not fit. `tinted_icon` adds a glyph in a color of its own.

`StatusBarItemSizing::LABEL`, the default, sets glyphs 12 wide and 4 from
their label; `COUNTS` sets them 3 from their counts, so each pair reads as one
figure.

## Emphasis and ink

```rust
StatusBarItem::new("branch", 64.0, palette).icon(Icon::Github).label("main").strong().ink(palette.text)
StatusBarItem::new("bell", 42.0, palette).icon(Icon::Bell).count(3).ink(palette.text).centered()
```

`strong` sets the label in the text color instead of the muted one, for the
one value the bar is read for. `ink` colors the glyphs only. `centered`
centers the content in the cell instead of starting it at the leading edge.

## Passive cells and dividers

```rust
StatusBarItem::new("encoding", 44.0, palette).label("UTF-8").passive()
StatusBarItem::divider("rule", palette)
```

A `passive` cell is information without an action: it has no hit target and
no hover, and it keeps its hint. `divider` is a passive hairline one pixel
wide between two groups of cells.

## Hint

```rust
StatusBarItem::new("cursor", 76.0, palette)
    .label("Ln 12, Col 4")
    .tooltip_with(TooltipSpec::new("Go to line and column"))
```

## Fitting the width

```rust
StatusBar::new(leading, trailing, palette)
    .width(width)
    .overflow(StatusBarItem::new("more", 20.0, palette).icon(Icon::More))
    .render()
```

With `width`, the leading cells always stand and the trailing ones are
offered the room that is left, the highest `priority` first. One that does not
fit is left out, and a narrower one of lower priority may still stay. Those
kept are shown in the order given. Without `width` every cell is shown.

`overflow` is a cell shown at the end only while some trailing cell does not
fit; its width is set aside before the others are fitted. The host fills its
menu. `trailing_budget(px)` replaces the room the bar would work out with a
width the host reserves by its own rule.

## Menus

```rust
StatusBarItem::new("ending", 24.0, palette).label("LF").menu(
    vec![
        DropdownMenuItem::new("ending-lf", "LF").selected(true).into(),
        DropdownMenuItem::new("ending-crlf", "CRLF").into(),
    ],
    state.ending_open,
    "ending-close",
    palette,
)
```

`menu(entries, open, dismiss_id, menu_palette)` opens a menu upward from the
cell's trailing edge. The host owns `open`: it sets it on the cell's click and
clears it when `dismiss_id` is reported.

## Top hairline

```rust
StatusBar::new(leading, trailing, palette).width(width).top_hairline(true).render()
```

For a bar that closes a panel of its own: a hairline above it and the
palette's `surface` behind it, one pixel taller in all.

## A cell the host builds

`hosted(element)` stands an element the host built in the cell's place, such
as the cell rendered and wrapped in a popover; the bar keeps the width and the
priority. `dispatch_as(action)` reports another ID while the cell keeps its
own as its focus target.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a cell's ID, or the one given to `dispatch_as` | A cell that is not passive is activated. |
| `UiAction::Clicked { id }` with a menu's `dismiss_id` | The cell's open menu is left. |
