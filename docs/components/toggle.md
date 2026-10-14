# Toggle

A button that stays pressed while something is on: a filter, a pinned panel,
a text style. `ToggleGroup` joins several in one row, of which any number may
be pressed at once.

## Import

```rust
use limn::{Component, Toggle, ToggleGroup, ToggleItem, ToggleVariant};
```

## Usage

A toggle is controlled: pass whether it is pressed each frame, and flip your
own value when it reports a click.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Toggle::new("wrap", "Wrap lines", state.wrap, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "wrap" {
            state.wrap = !state.wrap;
        }
    }
}
```

## Icons

```rust
Toggle::new("unread", "Unread only", state.unread, palette).icon(Icon::Filter)
Toggle::icon_only("pin", Icon::Pin, state.pinned, palette).tooltip("Pin the panel")
```

`.icon` puts an icon before the label. `Toggle::icon_only` shows the icon
alone, in a square as wide as the toggle is high.

## Variants

```rust
Toggle::new("wrap", "Wrap lines", state.wrap, palette).variant(ToggleVariant::Outline)
```

| Variant | At rest | Use |
| --- | --- | --- |
| `Ghost`, the default | No fill and no outline. | A toolbar. |
| `Outline` | The palette's `surface` inside its `border`. | A toggle that stands alone on a page. |

Pressed, both take the palette's `selected` fill, as an active `Button` of the
same variant does. The outline keeps its border.

## Sizes

```rust
Toggle::new("wrap", "Wrap lines", state.wrap, palette).size(ControlSize::Compact)
```

`Dense` is 24 pixels high, `Compact` 28, `Regular` 32 and `Spacious` 36.
`Regular` is the default.

## Width

```rust
Toggle::new("bold", "B", state.bold, palette).width(32.0).accessibility_label("Bold")
```

A toggle is as wide as its content. `.width(px)` fixes it: a square for a
label of one letter, or a column of toggles of one width. A label that does
not fit is cut with an ellipsis. A negative width becomes zero and one that is
not finite is ignored.

## Tooltip

`.tooltip(title)` gives the toggle a hint. A disabled toggle keeps it.

## Toggle group

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    ToggleGroup::new(
        "style",
        vec![
            ToggleItem::new("bold", "B", state.bold).width(30.0).accessibility_label("Bold"),
            ToggleItem::new("italic", "I", state.italic).width(30.0).accessibility_label("Italic"),
            ToggleItem::icon_only("pin", Icon::Pin, state.pinned).tooltip("Pin"),
        ],
        palette,
    )
    .accessibility_label("Text style")
    .render()
}
```

The toggles sit inside one outline, a hairline apart, and only the corners at
the ends of the row are round. Each `ToggleItem` is pressed or not on its own
and reports its own ID; the group reports nothing. Handle each ID as you
would a single toggle's.

A `ToggleItem` takes `.icon`, `.width`, `.tooltip`, `.accessibility_label`,
`.disabled()` and `.enabled(bool)`, as a `Toggle` does. The group takes
`.size(ControlSize)`: the height is the group's, outline included, so a group
lines up with a button of the same size.

## Toggle, SegmentedControl or Switch

| Use | When |
| --- | --- |
| `Toggle`, `ToggleGroup` | Each option is on or off by itself, and any number may be on: bold and italic, filters, panels shown. |
| [`SegmentedControl`](segmented-control.md) | Exactly one of several is chosen: a view mode, a time range. |
| [`Switch`](switch.md) | A setting that takes effect at once and reads as on or off, in a form or a preferences row. |

## Disabled

```rust
Toggle::new("wrap", "Wrap lines", true, palette).disabled().render()
Toggle::new("wrap", "Wrap lines", state.wrap, palette).enabled(state.editable).render()

ToggleGroup::new("style", items, palette).disabled().render()
ToggleItem::new("italic", "I", state.italic).disabled()
```

A disabled toggle is drawn muted and still shows whether it is pressed. It
keeps its place and its size, takes no click and no focus, and reports
nothing. A disabled group disables every toggle in it, whatever each one
says; one disabled item leaves the others in use.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Moves to the next or the previous toggle. Each toggle of a group is a stop. |
| Enter, Space | Reports the focused toggle's `Clicked`. |

## Motion

The fill under the pointer and while pressing cross-fades over
`tokens().motion.hover_ms`, as a button's does, and stands at once under
reduced motion.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A toggle, alone or in a group, is activated by pointer, Enter or Space. `id` is that toggle's. |

## Accessibility

The platform is told a button and whether it is pressed, disabled or not. A
toggle is named by its label. `.accessibility_label(name)` names it in place
of the label, which a label of one letter needs. A toggle of an icon alone
has no name of its own: give it an `accessibility_label`, or a `tooltip`,
whose title it is then called by. A tooltip on a toggle that has a name is
its description.

A group with `.accessibility_label("Text style")` is told as a group of that
name, known by the group's ID. Without one the platform is told of the
toggles alone.
