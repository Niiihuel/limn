# ResizeHandle

The strip between two panes that the pointer drags to move their boundary.
It reports the drag; the host, which owns the panes' sizes, moves them.

## Import

```rust
use limn::{Component, ResizeHandle};
```

## Usage

`ResizeHandle::vertical(id, palette)` stands between two columns and fills
the height of the row that holds it. The handle is controlled: it reports
`DragStarted`, `DragMoved` and `Dropped` or `DragCancelled` on its ID, and
the host adds how far the pointer has travelled to the width the pane had
when the drag began, within its own limits.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::row(vec![
        library(state, palette).size(Some(state.library_width), None),
        ResizeHandle::vertical("split", palette).render(),
        content(state, palette).flex(1.0),
    ])
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::DragStarted { id, position } if id == "split" => {
            state.drag_from = Some((position.x, state.library_width));
        }
        UiAction::DragMoved { id, position, .. } if id == "split" => {
            if let Some((x, width)) = state.drag_from {
                state.library_width = (width + position.x - x).clamp(160.0, 480.0);
            }
        }
        UiAction::Dropped { id, .. } if id == "split" => state.drag_from = None,
        UiAction::DragCancelled { id } if id == "split" => {
            if let Some((_, width)) = state.drag_from.take() {
                state.library_width = width;
            }
        }
        _ => {}
    }
}
```

The handle draws nothing at rest, so the gap between two panes can be the
handle itself. Under the pointer it fills with the palette's `hover`, and
while held with its `selected`. Nothing follows the pointer: the handle
stays where the host's panes put it, so a boundary that has reached its
limit shows where it stopped.

## Between two rows

```rust
Element::column(vec![
    editor,
    ResizeHandle::horizontal("terminal-split", palette).render(),
    terminal,
])
```

`ResizeHandle::horizontal` lies between two rows, moves up and down and
fills the width of the column that holds it. The host reads `position.y`.

## Thickness

```rust
ResizeHandle::vertical("split", palette).thickness(12.0).render()
```

`thickness` is the room the handle takes across the boundary and its pointer
target: 8 unless set and never under 2. A non-finite value is ignored.

## Cursor

Over the handle and through the drag the pointer takes the column-resize
shape (`Cursor::ResizeColumn`) for a vertical handle and the row-resize
shape (`Cursor::ResizeRow`) for a horizontal one. The tree reports it
through `UiTree::cursor` and the host's backend applies it; with egui that
is `limn_egui::set_cursor(ctx, tree.cursor())` once a frame, after the
layout.

## Disabled

```rust
ResizeHandle::vertical("split", palette).enabled(state.resizable).render()
```

A disabled handle keeps its place and its thickness, so the panes do not
move. It cannot be dragged or focused, and the pointer keeps its shape.

## Actions

| Action | When |
| --- | --- |
| `UiAction::DragStarted { id, position }` | The pressed pointer leaves the click tolerance. `position` is where it was pressed. |
| `UiAction::DragMoved { id, position, over }` | The pointer moves during the drag, on the handle or away from it. |
| `UiAction::Dropped { id, position, target }` | The pointer is released. |
| `UiAction::DragCancelled { id }` | Escape or the pointer leaving the window ended the drag. |
| `UiAction::Clicked { id }` | A press is released without moving. The host may ignore it. |

The handle has no keyboard resize.
