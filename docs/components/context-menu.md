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

## Measures

A context menu is 352 logical pixels wide, with 26 pixel rows and 13 pixel labels.
`UiTree::set_context_menu_sizing(SelectorSizing)` draws every context menu of the
tree, the code editor's and the text's included, at the measures of the host's
other menus: its width from `preferred_width`, and the rows' `row_height`,
`label_size`, `icon_size`, `padding`, `gap` and `scrollbar_gutter`.
`UiTree::set_context_menu_palette` matches their colors likewise.

```rust
tree.set_context_menu_sizing(SelectorSizing {
    preferred_width: 240.0,
    row_height: 28.0,
    label_size: 14.0,
    ..SelectorSizing::COMPACT
});
```

## A look of your own

A view is the function that draws a context menu: it reads a `ContextMenuView`
(the entries, the colors, the width, the tallest the menu may be, a way to
measure text and the measures in force) and returns the menu's element and its
size. `ContextMenu::view` gives one menu its view.
`UiTree::set_context_menu_view` gives one to every menu without its own, the
code editor's and the text's included. With neither, limn's look draws it.

```rust
fn plain_menu() -> ContextMenuViewFn {
    ContextMenuViewFn::new(|view| {
        if view.touch.is_some() {
            return view.default_element(); // limn's edit bar on a touch screen
        }
        let row = |entry: &ContextMenuEntry| if matches!(entry, ContextMenuEntry::Separator) { 9.0 } else { 32.0 };
        let rows = view.entries.iter().enumerate().map(|(index, entry)| match entry {
            ContextMenuEntry::Item(item) if item.enabled => {
                Element::clickable(ContextMenuView::row_id(index), Element::text(&item.label))
                    .action_menu_item()
                    .hover_background(Color::rgb(60, 60, 72))
                    .pressed_background(Color::rgb(80, 80, 96))
                    .size(Some(view.width), Some(row(entry)))
            }
            ContextMenuEntry::Item(item) => Element::text(&item.label).size(Some(view.width), Some(row(entry))),
            ContextMenuEntry::Separator => Element::spacer().size(Some(view.width), Some(row(entry))),
        });
        let height = view.entries.iter().map(row).sum::<f32>().min(view.max_height);
        let menu = Element::scroll(ContextMenuView::scroll_id(), Element::column(rows.collect()))
            .background(Color::rgb(30, 30, 36))
            .size(Some(view.width), Some(height));
        (menu, Size::new(view.width, height))
    })
}

tree.set_context_menu_view(Some(plain_menu()));
```

The tree keeps the rest: it opens the menu, places it inside the window,
lights its active row, moves that row with the pointer and the keyboard,
scrolls it, runs its shortcuts and closes it. It knows the menu's parts by
their IDs, so a view owes it this:

- Entry `i` of `view.entries`, separators counted, is row `i`. An enabled
  entry is one `Element::clickable` with the ID `ContextMenuView::row_id(i)`,
  with `action_menu_item()`. The row shows it is lit, under the pointer or as
  the keyboard's row, with its `hover_background`, and pressed with its
  `pressed_background`.
- A disabled entry and a separator take no pointer. The tree never chooses
  them, but it would light one under the pointer that had a row's ID.
- Rows that do not fit in `view.max_height` scroll in one scroll area with the
  ID `ContextMenuView::scroll_id()`. The tree starts it at its top each time
  the menu opens and lets its rail be dragged; under another ID a press on the
  rail closes the menu.
- A press on no row and not on that rail closes the menu, as one outside
  does, and what was pressed is not told. Only a press where nothing in the
  menu has a hit region leaves it open; a scroll area has one.
- The size returned is the element's: the tree places the element in a
  rectangle of that size, and the wheel there scrolls the menu.

`view.touch` is set for a menu of text on a touch screen
(`set_touch_text_menu`), which the tree centres on the press, over what was
held. limn's look for it is the edit bar, as wide as its actions.

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
control under the pointer. A wheel or trackpad scroll outside it closes it
too, and scrolls what is under the pointer.

## Accessibility

While the menu is open the platform is told each item is a menu item named by
its label, and that a disabled one is there and cannot be used. A separator
says nothing.

## Limits

- Its items are actions in one list: no submenus, check boxes, radio items or
  a second line that says why an item is disabled. For those, the host opens a
  [DropdownMenu](dropdown-menu.md#at-a-point) with `at(point)` where the
  secondary click was, and catches the menu key itself.
