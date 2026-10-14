# Checkbox

A box that is checked or not, with a label beside it: one of several options
that can each be chosen, or a statement to agree to. For a setting that takes
effect at once, use a [Switch](switch.md).

## Import

```rust
use limn::{Checkbox, Component};
```

## Usage

A checkbox is controlled: pass whether it is checked each frame, and flip your
own value when it reports a click.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Checkbox::new("terms", state.terms, palette).label("I accept the terms").render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "terms" {
            state.terms = !state.terms;
        }
    }
}
```

A checked box is filled with the palette's text color and its check is drawn
in the surface color. An unchecked one is an empty box with a border.

## Label

```rust
Checkbox::new("updates", state.updates, palette).label("Send me product updates")
```

The label is part of the target: a click anywhere on it is a click on the
checkbox, and the empty box lights while the pointer is over either. Without
a label the checkbox is the box alone.

A label that does not fit its width wraps; it is never cut. The box stays on
the first line. In a row the checkbox takes the width of its label. In a
column it takes the column's width, and the room beside a short label is part
of the target. Give it a width, or put it in a column that has one, where it
should wrap:

```rust
Checkbox::new("offline", state.offline, palette)
    .label("Keep a copy of every message on this device, including attachments")
    .render()
    .size(Some(280.0), None)
```

## Indeterminate

```rust
let chosen = state.channels.iter().filter(|channel| channel.on).count();
Checkbox::new("all-channels", chosen == state.channels.len(), palette)
    .label("All channels")
    .indeterminate(chosen > 0 && chosen < state.channels.len())
```

An indeterminate box is filled and shows a dash, whatever `checked` is. It is
for the parent of a list of which only some are chosen. It reports a click as
any other; the host decides what that means, usually choosing all, or none
once all are chosen.

## Sizes

```rust
Checkbox::new("dense", on, palette).label("Dense").size(ControlSize::Dense)
```

| Size | Box | Label text | Line |
| --- | --- | --- | --- |
| `ControlSize::Dense` | 14 | 12 | 16 |
| `ControlSize::Compact` | 16 | 13 | 18 |
| `ControlSize::Regular`, the default | 16 | 14 | 20 |
| `ControlSize::Spacious` | 18 | 16 | 24 |

A checkbox is as high as its label, one line for each line of text, not as
high as a button of the same size. A box alone is one line high.

## Disabled

```rust
Checkbox::new("sync", true, palette).label("Sync").disabled().render()
Checkbox::new("sync", state.sync, palette).label("Sync").enabled(state.signed_in).render()
```

A disabled checkbox shows its value in the muted color, keeps its place and
takes no pointer or focus.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves the focus to the checkbox. A disabled one is skipped. |
| Enter, Space | Reports the click. |

## Motion

The fill cross-fades between empty and filled over `hover_ms`, as it does
under the pointer, and the mark's ink fades with it over `move_ms`. With
reduced motion both change at once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The box or its label is clicked, or Enter or Space is pressed while it has the focus. |

## Accessibility

The platform is told a check box, named by its label, whether it is on, and
whether it is disabled. A box without a label has no name until the host gives
one, which is also said in place of a label when both are given:

```rust
Checkbox::new("row-3", selected, palette).accessibility_label("Select row 3")
```

An indeterminate box is told as mixed: neither on nor off.
