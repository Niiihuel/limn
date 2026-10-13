# Pager

Two arrows around a position label such as "3 / 12", for stepping through a
sequence: pages, search matches, changes.

## Import

```rust
use limn::{Component, Icon, Pager, PagerSizing, TooltipSpec};
```

## Usage

The host owns the position: it writes the label and says which ends can be
taken. Each arrow reports `UiAction::Clicked` with the ID it was given.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Pager::new(format!("{} / {}", state.page, state.pages), palette)
        .previous("page-previous", state.page > 1)
        .next("page-next", state.page < state.pages)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "page-previous" => state.page -= 1,
            "page-next" => state.page += 1,
            _ => {}
        }
    }
}
```

`Pager::new` is the label alone; `previous` and `next` each add one arrow. An
arrow passed `false` is passive: it has no hit target and keeps its hint.

## Label width

```rust
Pager::new("3 / 12", palette)
    .previous("page-previous", true)
    .next("page-next", true)
    .label_width(52.0)
    .render()
```

`label_width` holds the label, centered, in a slot of that width, so the
arrows do not move as the position changes. A label that does not fit is cut
with an ellipsis.

## Icons

```rust
Pager::new("3 / 12", palette)
    .previous("change-previous", true)
    .next("change-next", true)
    .icons(Icon::ArrowUp, Icon::ArrowDown)
    .render()
```

The arrows point left and right. `icons` gives other glyphs to the arrows
already added, such as up and down for a sequence that runs down the page, so
call it after `previous` and `next`.

## Hints

```rust
Pager::new("3 / 12", palette)
    .previous("page-previous", true)
    .next("page-next", true)
    .tooltips(
        TooltipSpec::new("Previous page").shortcut("Shift+F7"),
        TooltipSpec::new("Next page").shortcut("F7"),
    )
    .label_tooltip("page-where", TooltipSpec::new("Page 3 of 12"))
    .render()
```

`tooltips` takes one `TooltipSpec` per arrow and, like `icons`, applies to the
arrows already added. `label_tooltip` puts a hint on the label; its ID must be
unique in the window.

## Sizing

```rust
Pager::new("3 / 12", palette)
    .previous("page-previous", true)
    .next("page-next", true)
    .sizing(PagerSizing { gap: 0.0 })
    .render()
```

`PagerSizing` has one field, `gap`: the space between each arrow and the
label. `PagerSizing::DEFAULT` is 4.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | An enabled arrow is activated, with the ID passed to `previous` or `next`. |

## Accessibility

Each arrow is a button named by the title of its hint, so give the arrows
`tooltips`: without them they have no name. An end that cannot be taken is
there and says it cannot be used.
