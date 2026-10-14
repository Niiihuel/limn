# Chip

An inline pill for a count, a label or a state: a tag, a plan name, "Online".
It is passive, with no pointer and no focus, unless you give it a remove
button, which is then its only control.

## Import

```rust
use limn::{AlertTone, Chip, Component, ControlSize, Icon};
```

## Usage

A chip needs a label and the palette. Place it in a row, so its width follows
its text.

```rust
fn view(palette: ComponentPalette) -> Element {
    Element::row(vec![
        Chip::new("rust", palette).render(),
        Chip::new("draft", palette).render(),
    ])
    .gap(8.0)
}
```

The fill is `palette.selected` with a border of `palette.border`. A label
that is a single digit, with nothing before or after it, is drawn as a circle.
A chip is as wide as its label: it does not wrap or truncate.

A removable chip reports a click, and the host drops the tag:

```rust
// view
for (index, tag) in state.tags.iter().enumerate() {
    row.push(Chip::new(tag, palette).remove_action(format!("tag-remove:{index}")).render());
}

// update
UiAction::Clicked { id } => {
    if let Some(index) = id.strip_prefix("tag-remove:").and_then(|index| index.parse::<usize>().ok()) {
        state.tags.remove(index);
    }
}
```

## Outline

```rust
Chip::new("Pro plan", palette).outline().render()
```

`outline` draws the border only, with no fill, for a label that should sit
quietly on its surface.

## Tones

```rust
Chip::new("Deployed", palette).tone(AlertTone::Success).render()
Chip::new("Failed", palette).tone(AlertTone::Error).outline().render()
```

`tone` says a state. Its color is the one an [Alert](alert.md) of the same
tone gives its icon, `AlertColors::for_tone(tone, palette).accent`, so the
two agree in every theme.

| Chip | Fill | Border | Label |
| --- | --- | --- | --- |
| Toned | A tint of the color over `palette.surface` | A stronger tint | The color, at a contrast of 4.5 or more on the fill |
| Toned, `outline` | None | The color | The color, at a contrast of 4.5 or more on `palette.surface` |

`AlertTone::Neutral` is the chip without a tone. The color alone does not say
the state to everyone: let the label say it.

## Color

```rust
Chip::new("Design", palette).color(Color::rgb(138, 92, 246)).render()
```

`color` takes a color of your own and draws it as a tone is drawn. The label
is moved toward white or black until it reads on the fill, and the border of
an outline chip until it shows on the surface, so one color serves both
themes. The later of `tone` and `color` is the one used.

## Sizes

```rust
Chip::new("beta", palette).size(ControlSize::Dense).render()
```

`size` sets the height and the text together.

| Size | Height | Text |
| --- | --- | --- |
| `Dense` | 18 | 10 |
| `Compact` | 20 | 11 |
| `Regular` (the chip without a size) | 22 | 11 |
| `Spacious` | 26 | 12 |

## Height

```rust
Chip::new("3", palette).height(18.0).render()
```

`height` sets the height alone, never lower than 16 px, and leaves the text
as it is. Called after `size`, it changes that size's height.

## Icon and dot

```rust
Chip::new("main", palette).icon(Icon::GitBranch).render()
Chip::new("Online", palette).tone(AlertTone::Success).dot().render()
```

`icon` draws an icon before the label, and `dot` a circle 6 px across: the
mark of a state such as "online" or "failed". Both take the label's color, so
a toned chip has a toned dot. A chip has one or the other: the later call is
the one drawn.

## Removable

```rust
Chip::new("rust", palette).remove_action("tag-remove:rust").render()
```

`remove_action` adds a round close button after the label that reports
`UiAction::Clicked` with the ID you give. The chip does not remove itself:
leave it out of the next view. Only the button is a target; a click on the
label does nothing.

## Disabled

```rust
Chip::new("rust", palette).remove_action("tag-remove:rust").disabled().render()
Chip::new("rust", palette).remove_action("tag-remove:rust").enabled(state.can_edit).render()
```

A disabled chip is drawn in `palette.muted`, keeping its fill and border. Its
remove button stays in place and is passive: no hit region, no focus, nothing
reported. A chip without a remove button is only muted.

## Many chips

```rust
Element::row(tags).gap(6.0).wrap()
```

A row with `wrap` moves the chips that do not fit its width to the next line,
so a long list of tags or roles grows downward instead of out of its box.
`line_gap` sets the space between the lines. See
[Layout](../layout.md#wrapping-rows).

## Keyboard

A chip without a remove button takes no keys.

| Key | Effect |
| --- | --- |
| Tab | Moves the focus to the remove button, and on from it |
| Enter, Space | Reports the remove action |

## Motion

The remove button's fill under the pointer changes over
`tokens().motion.hover_ms`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The remove button is clicked or activated with Enter or Space; `id` is the one given to `remove_action` |

## Accessibility

The remove button is a button named `Labels::remove` followed by the chip's
label, such as "Remove rust", in the language of the labels you set. Disabled,
it is still announced, as disabled. The chip itself has no node: its label is
text like any other, and a dot or an icon says nothing to the platform, so the
label must carry the state.

## Chip or Badge

A chip is a pill in the flow of a row: it has its own place and its own
label. A [Badge](badge.md) is a mark over the corner of another element, such
as an unread count on a bell: it takes no room and is nothing without the
element it marks. Use a chip to label a thing beside it, a badge to mark a
control.

## Related

`Avatar` and `Kbd` are the other passive labels shown beside it in the
gallery: an account photo or the initial of a name, and the keycaps of a
shortcut.
