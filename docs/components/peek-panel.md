# PeekPanel

What is left of a side panel the user has closed: a strip at the window's
edge through which the panel's content can barely be seen, that reaches out
under the pointer and opens the panel when it is clicked.

## Import

```rust
use limn::{Component, PeekPanel, Side};
```

## Usage

Render the strip where the panel was while it is closed, with the panel's
content. The host owns the window's layout and whether the panel is open: a
click on the strip reports its ID, and the host opens the panel.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let side = if state.queue_open {
        queue(state, palette).size(Some(state.queue_width), None)
    } else {
        PeekPanel::new("queue-peek", queue(state, palette), palette)
            .content_width(state.queue_width)
            .accessibility_label("Show the queue")
            .render()
    };
    Element::row(vec![main_view(state, palette).flex(1.0), side]).gap(8.0)
}

fn update(state: &mut State, action: &UiAction) {
    if matches!(action, UiAction::Clicked { id } if id == "queue-peek") {
        state.queue_open = true;
    }
}
```

## The strip

It is 40 px wide unless `width(px)` says otherwise, on the palette's
`surface`. The corners that face the window are rounded to the `panel`
radius and the others are square, so it runs to the window's edge.

The content is laid out at `content_width(px)`, the open panel's width (320
unless set), from the strip's left edge and clipped to the strip, so what
shows is how the panel begins. A veil of the surface at 0.9 opacity covers
it. The content is only painted there: its controls take no pointer, no
focus and no reader until the panel is open.

## Side

`side(Side::Right)` is the default. `side(Side::Left)` puts the rounded
corners on the right and reaches out to the right.

## Under the pointer

While the pointer is over the strip, or it has keyboard focus, it reaches 12
px further into the window on a spring that overshoots a little, the veil
lifts to 0.8 and a small chevron, muted at rest, turns to full ink. The
pointer over the part it reached keeps it out. Leaving reverses it.

The strip takes the room it reaches into, as a music app's does: what is
beside it in a row gives way, a flexible main area narrowing by the 12 px and
keeping its gap from the strip, instead of being covered. Put it in a row
beside the main area, on either side:

```rust
Element::row(vec![main_view.flex(1.0), peek.render()]).gap(8.0)
```

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Focuses the strip, which reaches out as under the pointer. |
| Enter, Space | Report the strip's ID. |

## Motion

The reach is the tree's: an `Element::push_on_hover` spring with a
damping ratio of 0.6, which overshoots by about a tenth and settles within
three `move_ms`. The veil and the chevron fade over `enter_ms` and
`exit_ms`. With reduced motion the strip reaches out and back at once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the strip's ID | The strip is clicked, or Enter or Space is pressed on it. |

## Accessibility

The strip is a button with no text: name it with `accessibility_label`,
such as "Show the queue".
