# RadioGroup

One choice among several, all of them in view: a circle for each option, with
a dot in the chosen one. Use it for two to about seven options that deserve to
be read before choosing; for a few short ones in little space, a
[SegmentedControl](segmented-control.md) does the same, and for many, a
[Select](select.md).

## Import

```rust
use limn::{Component, RadioGroup, RadioOption};
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
