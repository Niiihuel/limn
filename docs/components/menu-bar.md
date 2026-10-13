# MenuBar

The application menus across the top of a window: a row of triggers, one of
which may be open. In a narrow window the same menus fold behind one button.

## Import

```rust
use limn::{Component, DropdownMenuEntry, DropdownMenuItem, MenuBar, MenuBarMenu, TooltipSpec};
```

## Usage

The host owns which menu is open. Each `MenuBarMenu` takes the ID its trigger
reports, a label, the trigger's width, the ID reported when the open menu is
left, and its entries. `MenuBar::new` takes the ID of the open menu, if any.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let menus = vec![
        MenuBarMenu::new("menu-file", "File", 40.0, "menu-close", vec![
            DropdownMenuItem::new("file-new", "New file…").shortcut("Ctrl+N").into(),
            DropdownMenuItem::new("file-open", "Open…").shortcut("Ctrl+O").into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("file-save", "Save").shortcut("Ctrl+S").into(),
        ]),
        MenuBarMenu::new("menu-edit", "Edit", 44.0, "menu-close", vec![
            DropdownMenuItem::new("edit-undo", "Undo").shortcut("Ctrl+Z").into(),
        ]),
    ];
    MenuBar::new(menus, state.open_menu.as_deref(), palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "menu-file" | "menu-edit" => state.open_menu = Some(id.clone()),
            "menu-close" => state.open_menu = None,
            "file-save" => state.save(),
            _ => {}
        }
    }
}
```

The host passes every trigger's width, since it knows the label's language. A
width that is not finite is none. The trigger of the open menu is filled with
the palette's `selected`, and only the open menu's items exist.

## Compact

```rust
MenuBar::new(menus, state.open_menu.as_deref(), palette)
    .compact("menu-all", "menu-close", TooltipSpec::new("Main menu"))
    .render()
```

`compact(id, dismiss_id, tooltip)` folds the bar into one button that opens
every menu as a submenu. The button reports `id`; it is open while any
`open_id` is set, and reports `dismiss_id` when it is left. Each menu's row in
it has the ID `{id}:compact`, where `{id}` is the menu's own, unless
`MenuBarMenu::compact_id` names it.

## Menu palette

```rust
MenuBar::new(menus, open, palette).menu_palette(popover_palette).render()
```

`menu_palette` colors the open menus from another palette than the bar's.

## Width

```rust
let bar = MenuBar::new(menus, open, palette);
let width = bar.width();
```

`width()` is what the bar takes: its triggers and the gaps between them, or
`MenuBar::COMPACT_WIDTH` (32) when compact. The bar is `MenuBar::HEIGHT` (24)
high and its triggers are `MenuBar::GAP` (2) apart.

## Disabled

```rust
MenuBarMenu::new("menu-edit", "Edit", 44.0, "menu-close", entries).enabled(state.has_document)
```

A disabled menu keeps its trigger's place with a muted label and cannot be
opened, even when `open_id` names it. In the compact bar it is a disabled row.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Activates the item in focus. An open menu starts with focus on its first enabled item. |
| Down | Moves to the next item. |
| Right | In the compact bar, opens the submenu of the menu in focus. |
| Escape | Leaves the menu, which reports its `dismiss_id`. |

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a menu's `id` | Its trigger is activated. |
| `UiAction::Clicked { id }` with a menu's `dismiss_id` | The open menu is left. |
| `UiAction::Clicked { id }` with an item's ID | An item of the open menu is activated. |
| `UiAction::Clicked { id }` with the IDs given to `compact` | The compact button is activated; its menu is left. |

## Accessibility

Each trigger is a button named by its label that says whether its menu is
expanded; a disabled one is there and cannot be used. The compact button is
named by the title of its hint. The items are those of a
[dropdown menu](dropdown-menu.md).
