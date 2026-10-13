# Guided tours

How to point at controls that already exist and explain them one at a time:
a first-run tour, or a single hint on a new feature.

## CoachMark

A bubble with a caret towards a control and a ring around it. Use it to teach;
for a hint on hover use a `Tooltip`, and for a question use an `AlertDialog`.

### Example

```rust
let root = CoachMark::new(root, "tour", "new-task", palette)
    .title("Nueva tarea")
    .body("Cada tarea es una conversación con un agente sobre tu proyecto.")
    .placement(CoachSide::Right, CoachAlign::Start)
    .step(0, 3)
    .render();
```

It wraps the window's root, like `Modal`. The target is the ID of a button,
a clickable, an input or a scroll area built anywhere under that root. The
place where the control is built does not change.

### Options

| Builder | Effect |
| --- | --- |
| `.title(text)`, `.body(text)` | The heading and one or two sentences. The body wraps and is never cut. |
| `.illustration(element)` | A picture above the title. |
| `.step(index, count)` | Counts from zero. Shows "2 of 5", hides the secondary action on the first step and labels the primary action "Got it" on the last. |
| `.placement(side, align)` | `CoachSide::{Above, Below, Left, Right}` and `CoachAlign::{Start, Center, End}`. Below and start unless set. |
| `.also(id)` | Rings the area the targets cover together, such as a group of buttons. |
| `.accent(fill, ink)` | Paints the bubble, the caret and the ring in a statement color of the host. |
| `.dim(color)` | Dims the window around the target. |
| `.skip(label)` | Adds an action to leave the tour, on every step but the last. |
| `.next_label`, `.done_label`, `.back_label`, `.counter(text)` | Replace "Next", "Got it", "Back" and the counter. Without them the words come from the library's [labels](theming.md#language). |
| `.on_next(action)`, `.on_back(action)`, `.on_skip(action)` | Report another action. The controls keep their own IDs. |
| `.width(px)` | 320 unless set, never under 220. |
| `.key(text)` | Names the step when two steps share a target. |
| `.open(bool)` | Renders the root alone while `false`. |

Without `.accent` the bubble takes the palette's solid role: `palette.text`
as its fill and `palette.surface` as its ink. It is light in a dark theme and
dark in a light one, so it never reads as a menu or a tooltip.

### Actions

| Action | When |
| --- | --- |
| `Clicked { id }` | The close button, or Escape. |
| `Clicked { "{id}:next" }` | The primary action, Enter, or the Right arrow. On the last step it means the tour is over. |
| `Clicked { "{id}:back" }` | The secondary action, or the Left arrow. Not on the first step. |
| `Clicked { "{id}:skip" }` | The skip action. |

### Keyboard

| Key | Effect |
| --- | --- |
| Escape | Reports `id`. |
| Right, Left | Report the next and the back action, whichever button has focus. |
| Enter, Space | Activate the focused button; a new step focuses the primary action. |
| Tab | Moves among the bubble's own buttons. |

### Motion

| What | Duration |
| --- | --- |
| The bubble and the ring appear and leave like any popover. | `enter_ms`, `exit_ms` |
| On a new step both spring from the last target to the new one. | `move_ms` |

Buttons take input at the new place at once. A resize moves the ring with its
target without a trail. With reduced motion a step is in place on its first
frame.

### Notes

- While a step shows, the rest of the window takes no input, and a click
  outside the bubble does nothing. A stray click neither loses the tour nor
  acts on what is behind it. The close button and Escape always end it.
- Focus returns to the control that had it when the bubble goes.
- A target inside a scroll area is scrolled into view when its step starts.
- The ring takes the target's own corner radius.
- When no control with the target's ID is laid out in view, nothing is
  painted, the window stays usable, and the frame names the target. See
  [A target that is not there](#a-target-that-is-not-there).
- When no side of the target has room, the bubble stays whole inside the
  window, covers part of the target and has no caret.

## Tour

A sequence of coach marks. The host owns the index of the current step.

```rust
let steps = vec![
    TourStep::new("new-task", "Nueva tarea", "Cada tarea es una conversación."),
    TourStep::new("composer", "Escribe aquí", "Cuéntale al agente qué necesitas.")
        .placement(CoachSide::Above, CoachAlign::Center),
];
let root = Tour::new(root, "tour", steps, state.tour_step, palette).render();
```

```rust
match action {
    UiAction::Clicked { id } if id == "tour:next" => {
        state.tour_step = state.tour_step.map(|step| step + 1).filter(|step| *step < STEPS);
    }
    UiAction::Clicked { id } if id == "tour:back" => {
        state.tour_step = state.tour_step.map(|step| step.saturating_sub(1));
    }
    UiAction::Clicked { id } if id == "tour" || id == "tour:skip" => state.tour_step = None,
    _ => {}
}
```

With the step at `None`, or past the end, `Tour` renders the root alone.
`Tour::coach_mark()` returns the current step as a `CoachMark`, to set what
the whole tour shares: `.accent`, `.dim`, `.skip` and the labels.

## A target that is not there

A step may point at a control the window is not showing: a closed panel, a
button a narrow window hides. After each layout, ask the frame:

```rust
let frame = tree.layout(size);
if let Some(target) = frame.missing_coach_target() {
    // Open what holds the control, or move on in the direction of travel.
    state.tour_step = state.tour_step.map(|step| step + 1);
}
```

Nothing is painted for that frame, so a skipped step never flashes a bubble
in a corner.

## Writing a tour

- One idea per step. Say what the control is for, not what it is called.
- Keep the body under about 140 characters. Two short lines read at a glance.
- Do not start a tour over work in progress. Offer it, and keep a way to
  start it again later.
- Remember that a tour was finished or dismissed, so it is offered once.

## See also

- [Components](components.md#overlays) for the other floating surfaces.
- [Animation](animation.md) for how surfaces enter, leave and travel.
- [Layout](layout.md#scrolling-from-the-host) for `UiTree::reveal`.
