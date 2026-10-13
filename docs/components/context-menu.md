# ContextMenu

A menu opened by a secondary click on an element, for the actions that apply
to that element.

## Import

```rust
use limn::{Component, ContextMenu, ContextMenuEntry, ContextMenuItem};
```

## Usage

A context menu wraps the element it belongs to, where that element is built.
It adds no layout and no primary-click target, and the child keeps its own
focus and selection. The tree opens and closes the menu, so the host keeps no
open flag: it only handles the item's ID.

```rust
fn view(palette: ComponentPalette) -> Element {
    ContextMenu::new(
        "file-row",
        file_row(palette),
        vec![
            ContextMenuEntry::Item(
                ContextMenuItem::new("file:copy", "Copy").icon(Icon::File).shortcut("Ctrl+C"),
            ),
            ContextMenuEntry::Separator,
            ContextMenuEntry::Item(ContextMenuItem::new("file:delete", "Delete").icon(Icon::Trash)),
        ],
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "file:copy" => state.copy(),
            "file:delete" => state.delete(),
            _ => {}
        }
    }
}
```

The first argument names the area the menu belongs to. The menu opens when the
window's backend sends `UiEvent::ContextMenu { position }` over that area, or
`UiEvent::ContextMenuKeyboard` while a control inside it has focus; see
[Backend](../backend.md).

## Items

```rust
ContextMenuItem::new("paste", "Paste").enabled(state.can_paste)
```

`icon` puts a glyph before the label and `shortcut` keycaps after it.
`enabled(false)` draws the item muted and takes it out of reach. A long label
wraps; in a very narrow window the shortcut is left out whole, never cut.

## Keyboard

| Key | Effect |
| --- | --- |
| Up, Down, Home, End | Move through the enabled items, wrapping at the ends. |
| Enter | Activates the selected item. |
| An item's shortcut | Activates that item while the menu is open. The modifiers must match. |
| Escape, Tab | Close the menu. |

## Motion

The menu fades in and settles, and fades out; see
[Animation](../animation.md).

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's ID | The item is activated by pointer, Enter or its shortcut. |

A click outside the menu closes it and reports nothing: it does not reach the
control under the pointer.

## Accessibility

While the menu is open the platform is told each item is a menu item named by
its label, and that a disabled one is there and cannot be used. A separator
says nothing.
