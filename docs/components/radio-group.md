# RadioGroup

One choice among several, all of them in view: a circle for each option, with
a dot in the chosen one. Use it for two to about seven options that deserve to
be read before choosing; for a few short ones in little space, a
[SegmentedControl](segmented-control.md) does the same, and for many, a
[Select](select.md).

## Import

```rust
use limn::{Component, Icon, RadioGroup, RadioOption};
```

## Usage

A radio group is controlled: pass the chosen option's value each frame, and
keep the value an option reports when it is clicked. Each option's value is
the ID of its `UiAction::Clicked`, as a tab's `action_id` is, so it must be
unique in the window: prefix it with the group's name.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    RadioGroup::new(
        "theme",
        vec![
            RadioOption::new("theme:light", "Light"),
            RadioOption::new("theme:dark", "Dark"),
            RadioOption::new("theme:system", "Match the system"),
        ],
        Some(&state.theme),
        palette,
    )
    .accessibility_label("Theme")
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id.starts_with("theme:") {
            state.theme = id.clone();
        }
    }
}
```

`selected` is `None`, or a value no option has, while nothing is chosen yet:
every circle is empty.

The circle, the label and the description are one click target. The empty
circle lights while the pointer is anywhere on its option.

## Description

```rust
RadioOption::new("plan:team", "Team").description("Unlimited projects, ten seats.")
```

A muted line under the label. It is clicked with the label, and the circle
stays centered on the label.

## Icon

```rust
RadioOption::new("kind:text", "Text").icon(Icon::Hash).description("Messages, images and links.")
```

The icon stands between the circle and the label, in the label's color and
at the 16 pixels of every icon in a control. It is clicked with the label, and
a description starts under the label, past the icon.

## Cards

```rust
RadioGroup::new("plan", options, Some(&state.plan), palette).variant(RadioVariant::Card)
```

Each option is a rounded card with its circle at the left, as a settings
panel shows its choices: `raised`, with a 1 pixel `border`. The chosen card
takes the palette's `selected` fill and its border turns `text`, so nothing moves.
The whole card is the click target, and hover lights it. A disabled card is
muted and takes no pointer. Horizontal, the cards share the row's width
equally. Cards are 8 pixels apart; the keyboard and the accessibility tree are
those of the plain group.

## Horizontal

```rust
RadioGroup::new("theme", options, Some(&state.theme), palette).horizontal()
```

The options stand side by side, 16 pixels apart, instead of one under the
other, 8 pixels apart.

## Size

```rust
RadioGroup::new("theme", options, Some(&state.theme), palette).size(ControlSize::Compact)
```

| Size | Circle | Label text |
| --- | --- | --- |
| `Dense` | 14 | 12 |
| `Compact` | 16 | 13 |
| `Regular`, the default | 16 | 14 |
| `Spacious` | 18 | 16 |

These are a [Checkbox](checkbox.md)'s, so the two line up in one form. A
description is one pixel smaller than its label. An option is as high as its
text, not as a control of that size.

## Metrics

```rust
RadioGroup::new("plan", options, Some(&state.plan), palette).metrics(RadioMetrics {
    circle: 20.0,
    option_gap: 16.0,
    description_size: 14.0,
    description_gap: 4.0,
    label_size: 16.0,
})
```

`metrics` takes the measures of a plain group in place of its control size's:
the circle, the label and description text, the space between options stacked
one under the other, and the space between a label and its description. The
values above are Discord's settings list. Side by side, options keep their 16
pixels, and a card keeps its control size's measures. A number that is not
finite counts as zero.

## Narrow widths

A label or a description that does not fit wraps inside the width the group is
given. The circle centers on its label, so beside a label of two lines it
stands between them.

## Disabled

```rust
RadioGroup::new("theme", options, Some(&state.theme), palette).disabled()
RadioGroup::new("theme", options, Some(&state.theme), palette).enabled(state.signed_in)
RadioOption::new("plan:company", "Company").disabled()
RadioOption::new("plan:company", "Company").enabled(state.may_upgrade)
```

A disabled group shows its choice in the muted color and takes no pointer or
focus. A single disabled option does the same while the others work. Nothing
moves: a disabled option is laid out as an enabled one.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Move the focus to the next or the previous option; every enabled option is a stop. |
| Enter, Space | Choose the focused option. |
| Up, Left, Down, Right | Move the focus to the previous or the next enabled option and choose it, going round at the ends, however the group is laid out. |
| Home, End | Move to the first or the last enabled option and choose it. |

## Motion

The circle's fill and its dot cross-fade over `hover_ms` when the choice
changes, as the empty circle does under the pointer. With reduced motion they
change at once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }`, where `id` is the option's value | An option is chosen by pointer, Enter or Space, or reached with Left, Right, Home or End. Choosing the option that is already chosen reports it again. |

## Accessibility

Each option is told to the platform as a radio button named by its label, with
whether it is the selected one, its description, and whether it is disabled. A
disabled option is still there, known by its value.

The options do not say what they are options of. Name the group, and it
becomes a node of its own around them, known by the group's ID:

```rust
RadioGroup::new("theme", options, Some(&state.theme), palette).accessibility_label("Theme")
```
