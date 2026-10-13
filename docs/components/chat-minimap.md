# ChatMinimap

A conversation's turns as a rail of short marks beside it, to see where you
are in a long one and jump to another turn.

## Import

```rust
use limn::{ChatMinimap, ChatMinimapMark, Component};
```

## Usage

Give it one mark for each turn, and the mark of the turn in view. A click on
a mark reports its ID, and the host scrolls that turn into view.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let marks = state
        .turns
        .iter()
        .map(|turn| ChatMinimapMark::new(format!("turn={}", turn.index), &turn.question).preview(&turn.answer))
        .collect();
    ChatMinimap::new("minimap", marks, palette)
        .active(format!("turn={}", state.turn_in_view))
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(turn) = id.strip_prefix("turn=") {
            state.scroll_to_turn(turn);
        }
    }
}
```

## Marks

`ChatMinimapMark::new(id, title)` is one turn, and `preview` a line of what
it says. The mark under the pointer or with the focus grows and its
neighbours step down, and it shows the title and the preview in a tooltip at
once, without the wait of other hints.

## The turn in view

`active(id)` marks the turn the conversation shows now. The host passes it
each frame as the conversation scrolls.

## Keyboard

| Key | Effect |
| --- | --- |
| Up, Down | Move the focus to the next mark and report it. They stop at the ends of the rail. |
| Home | Moves to the first mark and reports it. |

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a mark's ID | A mark is clicked, or reached with the keyboard. |

## Accessibility

The marks are drawn by the tree, not built as controls, so the platform is
told nothing of them yet.
