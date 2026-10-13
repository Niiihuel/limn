# ButtonGroup

A set of actions that sits in a row when there is room and in a column when
there is not, such as the foot of a dialog. The group owns the order, spacing
and alignment of the buttons it is given.

## Import

```rust
use limn::{Button, ButtonGroup, ButtonVariant, Component};
```

## Usage

Pass rendered buttons in reading order, the primary action last. They sit in
a row, 6 px apart. Each button still reports its own `UiAction::Clicked`.

```rust
fn view(palette: ComponentPalette) -> Element {
    ButtonGroup::new(vec![
        Button::new("cancel", "Cancel", palette)
            .variant(ButtonVariant::Outline)
            .render(),
        Button::new("save", "Save", palette).render(),
    ])
    .render()
}
```

## Stacked

```rust
ButtonGroup::new(buttons)
    .stacked(narrow)
    .reverse_when_stacked(true)
    .render()
```

`stacked(true)` lays the actions in a column, each as wide as the group. The
host decides when: the group does not measure itself. `reverse_when_stacked`
reverses the actions in the column so the primary one comes first; in a row
it does nothing.

## Aligned to the end

```rust
ButtonGroup::new(buttons).align_end().render()
```

`align_end` pushes the actions to the far edge of the row.

## A leading element

```rust
ButtonGroup::new(buttons)
    .align_end()
    .leading(
        Button::new("help", "Help", palette)
            .variant(ButtonVariant::Ghost)
            .render(),
    )
    .render()
```

`leading` keeps one element at the near edge, before the actions: at the
start of the row, and first in the column whatever the actions' order.

## Sizing

```rust
const WIDE: ButtonGroupSizing = ButtonGroupSizing { gap: 12.0, stacked_gap: 10.0 };

ButtonGroup::new(buttons).sizing(WIDE).render()
```

A `ButtonGroupSizing` has one gap for the row and one for the column.
`ButtonGroupSizing::DEFAULT` is 6 and 6.
