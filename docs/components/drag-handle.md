# DragHandle

The grip on a card or a row that the pointer takes it by to move it among
its siblings. It reports the drag; the host, which owns the order, moves the
item.

## Import

```rust
use limn::{Component, DragHandle};
```

## Usage

`DragHandle::new(id, palette)` draws two columns of dots
([`Icon::DragHandle`](../components.md)), muted at rest and on the palette's
`hover` under the pointer. The pointer is the platform's own open hand over
it (`Cursor::Grab`) and its closed hand while the host says it is dragged
(`.dragging(true)`, `Cursor::Grabbing`), when the handle also fills with
`selected`. A host applies the shape with
`limn_egui::set_cursor(ctx, tree.cursor())` after the layout, which asks the
system for its own cursor: nothing of it is drawn by limn.

It is controlled, as `ResizeHandle` is: it reports `DragStarted`, `DragMoved`
and `Dropped` or `DragCancelled` on its ID, and nothing follows the pointer.
Make each item, or a control of each such as its header, a
`drop_target()`: `DragMoved` names the one under the pointer in `over`. Put
the dragged item in that one's place as the pointer goes, and with
`animate_move` on each item the others glide aside. Keep the order the drag
began with to put back on `DragCancelled`.

```rust
fn card(state: &State, id: &str, palette: ComponentPalette) -> Element {
    let head = Element::row(vec![
        Text::new(id).render().flex(1.0),
        DragHandle::new(format!("move:{id}"), palette)
            .on_hover(true)
            .dragging(state.dragging.as_deref() == Some(id))
            .accessibility_label(format!("Move {id}"))
            .render(),
    ]);
    Element::column(vec![Element::clickable(format!("card:{id}"), head).drop_target(), body(id)])
        .hover_region()
        .animate_move(format!("card:{id}:move"))
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::DragStarted { id, .. } => {
            if let Some(card) = id.strip_prefix("move:") {
                state.dragging = Some(card.to_owned());
                state.before = state.order.clone();
            }
        }
        UiAction::DragMoved { over: Some(over), .. } => {
            if let (Some(card), Some(target)) = (&state.dragging, over.strip_prefix("card:")) {
                let to = state.order.iter().position(|c| c == target).unwrap();
                state.order.retain(|c| c != card);
                state.order.insert(to, card.clone());
            }
        }
        UiAction::Dropped { .. } => state.dragging = None,
        UiAction::DragCancelled { .. } => {
            state.dragging = None;
            state.order = std::mem::take(&mut state.before);
        }
        _ => {}
    }
}
```

## Options

| Method | What it does |
|---|---|
| `.on_hover(bool)` | Shown only while the pointer is over the `hover_region` around it (its item), while it has the focus or while it is dragged. It keeps its place, so nothing moves when it appears. |
| `.dragging(bool)` | The host says a drag from it is under way: the closed hand, the `selected` fill, always shown. |
| `.size(px)` | Its side and pointer target: 24, never under 16. |
| `.accessibility_label(name)` | What the platform calls it, such as "Move Context". It shows no text. |
| `.enabled(bool)`, `.disabled()` | A disabled handle keeps its place, draws nothing and cannot be dragged or focused. |

## Accessibility

It is a button named by `accessibility_label`. Enter on it, or a press
released without moving, is a `Clicked` the host may ignore: there is no
keyboard move yet, so an order the person can only change by dragging should
also be reachable another way.
