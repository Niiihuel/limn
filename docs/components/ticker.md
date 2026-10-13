# Ticker

A fixed-height, clipped slot that shows the latest of a series of lines, such
as the current step of running work. The slot never changes height, so
content below it does not move.

## Import

```rust
use limn::{Component, Ticker};
```

## Usage

A ticker needs an ID, a key and the line to show. The key names the content:
the line travels only when the key changes.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Ticker::new(
        "step",
        &state.step,
        Element::truncated_text(&state.step)
            .foreground(palette.muted)
            .size(None, Some(20.0)),
    )
    .render()
}
```

The line is any element. The ticker is passive and reports nothing.

## Height

```rust
Ticker::new("step", &state.step, line).height(40.0).render()
```

The slot is 20 px high unless set. The line rests at its top, so a slot
taller than the line shows the next one arriving before it takes its place.

## Motion

When the key changes, the new line rises into place from its own height below
while the previous one moves up by the same distance and fades, clipped by
the slot, over `move_ms`. With reduced motion the lines swap at once.

`Element::animate_replace(id, key)` is the movement alone, for a parent that
clips already. See [Animation](../animation.md#an-element-that-replaces-another).
