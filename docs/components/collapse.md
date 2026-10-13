# Collapse

A slot in the flow that closes when its content leaves, so what follows
glides up instead of jumping by the content's height. Use it for content that
comes and goes with the application's state, such as the live output of
running work.

## Import

```rust
use limn::{Collapse, Component};
```

## Usage

Render the `Collapse` every frame and pass the content while there is any.
When the content stops being passed, the slot stays in the flow and closes.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::column(vec![
        Element::text("Exploring the project"),
        Collapse::new("run-live", state.running.then(|| live_output(state, palette))).render(),
        Element::text("What follows moves with the slot."),
    ])
}
```

The ID names the slot and must stay the same across rebuilds. The slot is
passive and reports nothing. While it closes, the content's last paint is
paint only: its controls stop taking input the moment it leaves.

## Expand

```rust
Collapse::new("run-live", content).expand().render()
```

Without `expand`, arriving content takes its height at once. With it, content
that arrives in a slot that was rendered empty opens from zero height and
fades in. Content already there when the slot first appears is shown at its
height either way.

## Spacing

A `gap` on the parent column still surrounds the empty slot. Keep the slot
out of a column with a gap and put the spacing inside the content, where it
closes with it.

## Motion

The slot opens and closes on a spring with no overshoot, which covers 99% of
its way in one and a half `move_ms`. While it closes, the last paint of the
content fades, clipped to what is left of the slot. Content that returns
while the slot is closing reopens from the height it has reached. A change of
height while the content is present is followed at once. With reduced motion
the slot opens and closes at once.

`Element::animate_height(id, expand)` is the same for an element of your own
whose height becomes zero. See
[Animation](../animation.md#content-in-the-flow-that-leaves).
