# DropdownMenu

A menu of actions that opens from a trigger: items with icons and shortcuts,
headings, separators and submenus. To choose one value of several use a
[DropdownList](dropdown-list.md).

## Import

```rust
use limn::{Component, DropdownMenu, DropdownMenuEntry, DropdownMenuItem};
```

## Usage

The menu wraps its trigger where the trigger is built. It is controlled: the
host owns whether it is open. An item's ID is the action it reports; the menu
does not close itself, so the host closes it when it handles an item or the
dismiss ID.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    DropdownMenu::new(
        Button::new("file", "File", palette).render(),
        vec![
            DropdownMenuEntry::Heading("File".into()),
            DropdownMenuItem::new("file:rename", "Rename").icon(Icon::Edit).shortcut("F2").into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("file:delete", "Delete").icon(Icon::Trash).into(),
        ],
        state.file_menu_open,
        "file:dismiss",
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "file" => state.file_menu_open = true,
            "file:dismiss" => state.file_menu_open = false,
            "file:rename" | "file:delete" => {
                state.file_menu_open = false;
                state.run(id);
            }
            _ => {}
        }
    }
}
```

While closed, `render` returns the trigger alone.

## Items

```rust
DropdownMenuItem::new("pin", "Pin").icon(Icon::Pin).disabled().into()
DropdownMenuItem::new("wrap", "Word wrap").selected(state.wrap).into()
DropdownMenuItem::new("delete", "Delete").destructive(danger).into()
DropdownMenuItem::new("sync", "Sync").annotation("2 pending").into()
```

`shortcut` shows keycaps at the end of the row and `annotation` plain status
text. `selected` checks the item, `disabled` (or `enabled(false)`) takes it
out of reach, and `destructive` draws it in an ink of the host's.
`tooltip_with(TooltipSpec)` adds a hint. `description` and the two-line row
are in [Catalog menus](picker.md).

## Submenus

```rust
DropdownMenuItem::new("settings", "Settings")
    .submenu(vec![
        DropdownMenuItem::new("account", "Account").into(),
        DropdownMenuItem::new("privacy", "Privacy").into(),
    ])
    .into()
```

The second menu opens beside its row while the pointer is over the row or the
submenu, or with the keyboard.

## Other entries

`DropdownMenuEntry::Heading(text)` names a group and `Separator` divides two.
`DropdownMenuEntry::custom(element)` puts a control of the host's in the menu.
`Message(text)`, `Loading { id, rows }` and `Error { id, text }` stand in for
items that are not there, and `DropdownMenuSection` is a heading that folds;
see [Catalog menus](picker.md).

## Size and placement

`width` is 240 unless set and never under 96. `max_visible_items` (8) is how
many rows show before the options scroll. `placement` takes a
`PopoverPlacement`; `BottomStart` unless set. `sizing(SelectorSizing)` takes a
shared profile for width, row density and scroll budget.

## Style

`context_menu_style()` gives the menu the surface and density of a
[ContextMenu](context-menu.md). `hints_in_tooltips(true)` moves shortcuts and
annotations into each row's hint, for a narrow menu. `inline(true)` renders
the options in the layout under the trigger instead of floating them.

## Keyboard

| Key | Effect |
| --- | --- |
| Up, Down, Home, End, Tab | Move through the enabled items; headings, separators and disabled items are skipped. |
| Right, Left | Enter a submenu and return to its row. |
| Enter | Activates the focused item. |
| Escape | Reports the dismiss ID. |

## Motion

The menu fades in and settles into place, and fades out; see
[Animation](../animation.md). `motion(MenuMotion { open_ms, item_ms })`,
`close_animation_ms` and `open_scale` change it. With reduced motion it is in
place at once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's ID | The item is activated. |
| `UiAction::Clicked { id }` with the dismiss ID | Escape, or a click outside the menu. |
