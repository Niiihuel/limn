# SplitView

A row or a column of panes with a [ResizeHandle](resize-handle.md) between
each pair, such as a chat beside a diff, or a diff over a terminal. The host
owns each pane's share of the room and moves it when a handle is dragged.

## Import

```rust
use limn::{Component, Pane, SplitView};
```

## Usage

`SplitView::row(id, panes, palette)` puts the panes side by side and
`SplitView::column` one over the other. `Pane::fraction` is a pane's share of
the room the handles leave, from 0 to 1; a pane without one (`Pane::new`, or
`.flex()`) takes what the others leave, shared evenly with any other such
pane. Each pane clips its content.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    SplitView::row(
        "work",
        vec![Pane::new(chat(state, palette)).fraction(state.chat), Pane::new(diff(state, palette)).flex()],
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::DragStarted { id, position } if id == "work:handle:0" => {
            state.drag_from = Some((position.x, state.chat));
        }
        UiAction::DragMoved { id, position, .. } if id == "work:handle:0" => {
            if let Some((x, chat)) = state.drag_from {
                // The room the panes share: the window's width less the handle.
                let fractions = SplitView::resized(&[chat, 1.0 - chat], 0, position.x - x, state.width - 8.0, &[240.0, 320.0]);
                state.chat = fractions[0];
            }
        }
        UiAction::Dropped { id, .. } if id == "work:handle:0" => state.drag_from = None,
        UiAction::DragCancelled { id } if id == "work:handle:0" => {
            if let Some((_, chat)) = state.drag_from.take() {
                state.chat = chat;
            }
        }
        _ => {}
    }
}
```

## Handles

The handle after pane `i` reports `DragStarted`, `DragMoved` and `Dropped` or
`DragCancelled` on `{id}:handle:{i}`, with the pointer's position, as a
`ResizeHandle` does: the host reads `position.x` in a row and `position.y` in
a column. `.handle_thickness(px)` is the room each handle takes, 8 unless set
and never under 2.

## Resizing within minimums

`SplitView::resized(fractions, handle, delta_px, total_px, mins)` is the new
set of fractions after the handle after pane `handle` has moved `delta_px`
toward the end. `total_px` is the room the panes share: the split's length
less its handles. Only the two panes beside the handle change, and neither
goes below its minimum in `mins`, in pixels. Pass the fractions the panes had
when the drag began and the pointer's whole travel since, so the boundary
stops at a minimum and comes back with the pointer.

Called with a `delta_px` of 0 it only fits the fractions to the minimums:
call it when the split's length changes, since the layout shrinks the panes
with it and knows no minimum. Panes whose minimums do not fit share the room
in proportion to them. The result always sums to 1.

A host that keeps a pane's size in pixels, such as a sidebar that keeps its
width when the window grows, passes `width / total_px` as its fraction and
reads its width back from `resized` the same way.

## Limits

The layout does not hold a pane to its minimum by itself; `resized` does,
when the host calls it. The handles have no keyboard resize and no name;
give the panes names in their own content.

## Actions

| Action | When |
| --- | --- |
| `UiAction::DragStarted { id, position }` | The pointer pressed on `{id}:handle:{i}` leaves the click tolerance. |
| `UiAction::DragMoved { id, position, over }` | The pointer moves during the drag. |
| `UiAction::Dropped { id, position, target }` | The pointer is released. |
| `UiAction::DragCancelled { id }` | Escape or the pointer leaving the window ended the drag. |
| `UiAction::Clicked { id }` | A press on a handle is released without moving. The host may ignore it. |
