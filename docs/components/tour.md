# Tour

A guided tour points at controls that already exist and explains them one at
a time. `CoachMark` is one step: a bubble with a caret towards a control and
a ring around it. `Tour` is a sequence of them. [Tours](../tours.md) covers
both in depth.

## Import

```rust
use limn::{CoachAlign, CoachSide, Component, Tour, TourStep};
```

`CoachMark` is imported the same way, for a single hint.

## Usage

A tour wraps the window's root element, built last, around everything else.
Each step names its target by the ID of a control built anywhere under that
root; where the control is built does not change. The host owns the index of
the current step.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let root = window_content(state, palette);
    let steps = vec![
        TourStep::new("new-task", "Start here", "Each task is a conversation with an agent."),
        TourStep::new("search", "Find it again", "Type part of a name to filter the list.")
            .placement(CoachSide::Below, CoachAlign::Center),
    ];
    Tour::new(root, "tour", steps, state.tour_step, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::Clicked { id } if id == "tour:next" => {
            state.tour_step = state.tour_step.map(|step| step + 1).filter(|step| *step < 2);
        }
        UiAction::Clicked { id } if id == "tour:back" => {
            state.tour_step = state.tour_step.map(|step| step.saturating_sub(1));
        }
        UiAction::Clicked { id } if id == "tour" || id == "tour:skip" => state.tour_step = None,
        _ => {}
    }
}
```

With the step at `None`, or past the end, `Tour` renders the root alone.
While a step shows, the rest of the window takes no input and a click outside
the bubble does nothing.

## A single step

```rust
CoachMark::new(root, "hint", "new-task", palette)
    .title("Start here")
    .body("Each task is a conversation with an agent.")
    .placement(CoachSide::Below, CoachAlign::Start)
    .step(0, 3)
    .render()
```

`CoachMark::new(base, id, target, palette)` is one bubble. `step(index,
count)` shows the counter, hides the back action on the first step and labels
the primary action as done on the last. `on_next`, `on_back` and `on_skip`
report an action of the host's instead of `{id}:next`, `{id}:back` and
`{id}:skip`.

## Accent, dim and skip

```rust
Tour::new(root, "tour", steps, state.tour_step, palette)
    .coach_mark()
    .accent(brand, Color::WHITE)
    .dim(Color::rgba(0, 0, 0, 110))
    .skip("Skip tour")
    .render()
```

`Tour::coach_mark()` returns the current step as a `CoachMark`, to set what
the whole tour shares. `accent(fill, ink)` paints the bubble, its caret and
the ring in a color of the host's, `dim` dims the window around the target,
and `skip` adds an action to leave the tour on every step but the last.

## Keyboard

| Key | Effect |
| --- | --- |
| Right, Left | Report the next and the back action. |
| Enter | Activates the focused button; a new step focuses the primary action. |
| Escape | Reports `id`. |

## Motion

The bubble and its ring fade in and out, and on a new step both travel from
the last target to the new one. See [Tours](../tours.md#motion).

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The close button, or Escape. |
| `UiAction::Clicked { id }` with `{id}:next` | The primary action, or the Right arrow. On the last step the tour is over. |
| `UiAction::Clicked { id }` with `{id}:back` | The secondary action, or the Left arrow. |
| `UiAction::Clicked { id }` with `{id}:skip` | The skip action. |

## Accessibility

The bubble is a dialog named by the step's title and described by its body,
known as `{id}:dialog`. Its close button is named by `labels().close`, and the
other actions by their labels.
