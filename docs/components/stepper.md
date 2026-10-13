# Stepper

The steps of a short, ordered form: a round badge for each step, a rule from
each badge to the next, and the step's label.

## Import

```rust
use limn::{Icon, Step, Stepper};
```

## Usage

A stepper is controlled: pass the steps and the index of the active one, and
change the index when a step reports a click. A `Step` takes an ID and a
label.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Stepper::new(
        vec![
            Step::new("step=0", "Content"),
            Step::new("step=1", "Schedule"),
            Step::new("step=2", "Summary"),
        ],
        state.step,
        palette,
    )
    .width(520.0)
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(index) = id.strip_prefix("step=") {
            state.step = index.parse().unwrap_or(0);
        }
    }
}
```

The steps up to the active one have a filled badge, and the rules between
them are filled too. The steps still to come are quiet and lift under the
pointer. An index past the end counts as the last step. Every step but the
last takes an equal share of `width`.

## Icons

```rust
Step::new("step=1", "Schedule").icon(Icon::Clock)
```

`icon` puts a glyph in the badge. Without one the badge shows the step's
number.

## Description

```rust
Step::new("step=1", "Schedule").description("When the task runs.")
```

`description` adds a muted line under the label.

## Vertical

```rust
Stepper::new(steps, state.step, palette).vertical().width(280.0).render()
```

`vertical()` stacks the steps, each with its label and description beside its
badge and the rule running down to the next.

## Disabled

```rust
Step::new("step=2", "Summary").disabled()
```

A disabled step keeps its place, its badge and its rule, with a muted label.
It takes no pointer or focus and reports nothing. `enabled(bool)` is the
same for a state the host computes.

## Motion

The fills of the badges and the rules cross-fade over the `move_ms` motion
token when the active step changes.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A step is clicked, on its badge, its rule or its label. The ID is the step's. |

## Accessibility

A step is a button named by its label, with its description said after it.
It says whether it is the selected one, the active step, and a disabled step
is there and cannot be used.
