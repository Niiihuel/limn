# DescriptionList

Pairs of a label and its value, for the details of one thing: an order, a
server, a person. Use it for a handful of facts that are read, not edited; for
many things with the same facts, use a [Table](table.md).

## Import

```rust
use limn::{Component, DescriptionItem, DescriptionList};
```

## Usage

The host owns the pairs and passes them each frame. The list reports nothing,
so there is no action of its own to handle: a control given as a value reports
its own.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    DescriptionList::new(
        vec![
            DescriptionItem::new("Order", state.order.number.clone()),
            DescriptionItem::new("Total", state.order.total.clone()),
            DescriptionItem::with("Invoice", Link::new("open-invoice", "INV-2026-0931", palette).render()),
        ],
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "open-invoice" {
            state.invoice_open = true;
        }
    }
}
```

Each pair is a row at least 32 high: the label in the palette's muted color
in a column 120 wide, its value beside it in the text color, and a hairline in
the border color between rows.

## Values

```rust
DescriptionItem::new("Total", "1,204.50 EUR")
DescriptionItem::with("Status", Chip::new("Shipped", palette).render())
DescriptionItem::with("Invoice", Link::new("open-invoice", "INV-2026-0931", palette).render())
```

`new` takes the value as text. A number given as text is text: it starts at
the left edge of the value's place like any other.

`with` takes an element the host built, such as a [Chip](chip.md), a
[Link](link.md) or a [Clipboard](clipboard.md). It keeps its measured size, its
hit region, its focus and its actions, and starts at the left edge of the
value's place. An element that flexes is given the whole width of that place.
One no higher than a row's content (24 at the regular size) is centered on the
row's first line; a higher one makes its row higher.

## Label width

```rust
DescriptionList::new(items, palette).label_width(180.0).render()
```

The width of the label column, 120 unless set. A negative width becomes zero
and one that is not finite is ignored. A stacked list has no label column and
ignores it.

## Stacked

```rust
DescriptionList::new(items, palette).stacked().render()
```

`stacked` puts each label over its value, both starting at the same edge, for
long values or a narrow pane.

## Columns

```rust
DescriptionList::new(
    vec![
        DescriptionItem::new("Name", "atlas-01"),
        DescriptionItem::new("Region", "eu-west-3"),
        DescriptionItem::new("Memory", "16 GB"),
        DescriptionItem::new("Role", "Serves the public API").span(3),
        DescriptionItem::new("Image", "debian-12").span(2),
        DescriptionItem::new("Uptime", "41 days"),
    ],
    palette,
)
.columns(3)
.render()
```

`columns` lays the pairs in columns of equal width, from 1 to 4; a number
outside that is held to it. `span` makes a pair take several columns: at least
1, and no more than the list has. A pair that does not fit in what its row has
left starts the next row, and the last pair of a row takes the columns the row
has left, so no row ends in an empty place.

Every column has its own label column of `label_width`. With several columns,
set a narrower one or stack the list.

## Bordered and dividers

```rust
DescriptionList::new(items, palette).bordered(true).render()
DescriptionList::new(items, palette).dividers(false).render()
```

`bordered(true)` draws the list as an outlined surface in the palette's
surface color, with the card radius of the tokens and hairlines between rows,
between columns and between each label and its value. Stacked, the hairline
between a label and its value runs across.

A plain list has only the hairlines between rows, and `dividers(false)`
removes those. A bordered list always has its hairlines.

## Sizes

```rust
DescriptionList::new(items, palette).size(ControlSize::Dense).render()
```

| Size | Row height | Text |
| --- | --- | --- |
| `Dense` | 24 | 12 |
| `Compact` | 28 | 13 |
| `Regular`, the default | 32 | 13 |
| `Spacious` | 36 | 14 |

The height is the least a row has when the label is beside its value. A value
that wraps makes its row higher.

## Width and long text

```rust
DescriptionList::new(items, palette).width(320.0).render()
```

Without `width` the list fills the width its parent gives it. A negative width
becomes zero and one that is not finite is ignored.

A long value wraps under itself, and the label stays on the value's first
line. A long label is cut to one line with an ellipsis.

## Disabled

There is nothing to disable: the list is not a control. A control given as a
value is disabled by the host, as it is anywhere else.

## Keyboard

None of its own. The list adds no tab stop; Tab, Enter and Space reach a
control given as a value as they do outside the list.

## Actions

None. A click on a control given as a value reports that control's action.

## Accessibility

The list and its pairs have no node. A passive element is a node only with an
ID to be known by, and a list takes none; and no role says "a term and its
value" today: `Role::Label` is text that names another control, and
`Role::ListItem` is a row of a list that is chosen from. The nodes of a frame
are also flat, so a pair that was a node would not hold the link inside it.
The platform is therefore told nothing of the text of a list, as it is told
nothing of other passive text.

A control given as a value keeps its node, with its role, its name and its
state. Its label in the list does not name it, so give it a name that reads
alone: a link that reads "INV-2026-0931", not "Open".

## Limits

- The label column has one width for the whole list, set by the host: it is
  not measured from the labels.
- A `label_width` wider than a column leaves its value no room.
- There is no separator item to group pairs: use two lists under two headings.
- A row's height is not fixed: a wrapped value or a tall element makes it higher.
