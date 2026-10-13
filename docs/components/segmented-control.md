# SegmentedControl

Tabs in one rounded track, for choosing one of a few options that are all
visible at once.

## Import

```rust
use limn::{Component, Icon, SegmentedControl, Tab};
```

## Usage

A segmented control is controlled: mark the chosen item each frame and change
it when an item reports a click. Its items are the `Tab`s of
[Tabs](tabs.md): an action ID, a label and a width.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let items = vec![
        Tab::new("layout=list", "List", 96.0).active(state.layout == "list"),
        Tab::new("layout=grid", "Grid", 96.0).active(state.layout == "grid"),
    ];
    SegmentedControl::new(items, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(layout) = id.strip_prefix("layout=") {
            state.layout = layout.to_owned();
        }
    }
}
```

The track is as wide as its items: the sum of their widths and the gaps
between them.

## Items

```rust
Tab::new("layout=list", "List", 96.0).icon(Icon::Menu)
Tab::new("layout=board", "Board", 96.0).disabled()
```

An item takes an icon before its label. A disabled item is drawn muted and
cannot be chosen.

## Height

```rust
SegmentedControl::new(items, palette).height(32.0).render()
```

The track is 28 tall by default and never less.

## Keyboard

With the focus on an item:

| Key | Effect |
| --- | --- |
| Left, Right | Chooses the item before or after, wrapping at the ends |
| Home, End | Chooses the first or the last item |

## Motion

The selection slides to the chosen item over the `move_ms` motion token.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | An item is chosen by pointer or keyboard. The ID is the item's. |

## Accessibility

Each item is told to the platform as a tab named by its label, and says
whether it is the chosen one. A disabled item is still there and says it
cannot be used.
