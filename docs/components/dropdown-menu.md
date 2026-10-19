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

`image(picture)` draws a picture in the icon's slot, in place of any icon,
at most 16 px on a side and untinted, and `surface(id)` a host-fed surface
there, fitted inside and named by the label, for an animation the host
presents. Both keep the slot for the menu's other rows.

## Check boxes and radio items

```rust
DropdownMenuItem::new("unread", "Show unread only").checkbox(state.unread_only).into()
DropdownMenuItem::new("level=all", "All messages").radio(state.level == Level::All).into()
DropdownMenuItem::new("level=mentions", "Only @mentions").radio(state.level == Level::Mentions).into()
```

`checkbox(checked)` draws a box at the end of the row, filled with a check
while it is ticked; `radio(checked)` draws a circle, with a dot in the chosen
one. The menu keeps the column for every row. Choosing either reports its ID,
as any item does: the host flips its own value, or chooses the radio item's
value. The mark is drawn as a `Checkbox` and a `RadioGroup` draw theirs, and is
muted while the item is disabled.

## Two lines, and why an item is disabled

```rust
DropdownMenuItem::new("notify", "Notifications").description("Only @mentions").into()
DropdownMenuItem::new("edit", "Edit channel")
    .description("Needs the Manage Channels permission")
    .tooltip_with(TooltipSpec::new("Needs the Manage Channels permission"))
    .disabled()
    .into()
```

`description(text)` puts a quiet second line under the label: the value a
setting has, or, on a disabled item, why it cannot be chosen. The row is as
tall as the menu's `sizing` says for two lines (`detail_row_height`, 40
without one) and lines up with the menu's one-line rows: it has their sides,
icon slot, label size and gap, and keeps their check and chevron columns, so
labels, marks and chevrons stand in the same columns in every style. The
second line is at the profile's `detail_size` (12 without one, and in
`context_menu_style`). The padding above and below the two lines gives way
when they need the room. The second line is cut to one line, so a hint says a
long reason whole. The platform hears it after the label, disabled or not.

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
submenu, or with the keyboard. A submenu can hold submenus of its own: each
stays open while the pointer or the focus is in one nested in it. One the
pointer opened closes when the pointer moves to another row of its parent;
one the keyboard opened closes when the pointer moves to a row that opens a
submenu, or to any row of a menu with `context_menu_style`.

An item with a submenu can also have a `description`: it is a two-line row
that ends with a chevron, such as a setting that shows its value under its
name and opens the list of values.

```rust
DropdownMenuItem::new("notify", "Notifications")
    .description("Only @mentions")
    .submenu(vec![
        DropdownMenuItem::new("level=all", "All messages").radio(false).into(),
        DropdownMenuItem::new("level=mentions", "Only @mentions").radio(true).into(),
    ])
    .into()
```

## At a point

A context menu opens where the secondary click was. `at(point)` places the
menu with its corner on a point of the window instead of against its anchor:
below and after the point where there is room, above it or moved in where
there is not. The anchor keeps its place in the layout; pass the element the
menu belongs to. The menu has every kind of item, submenus included, which a
[ContextMenu](context-menu.md) does not.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let row = channel_row(state, palette);
    match state.channel_menu_at {
        Some(point) => DropdownMenu::new(row, channel_entries(state), true, "channel:dismiss", palette)
            .context_menu_style()
            .at(point)
            .render(),
        None => row,
    }
}

// Before the tree sees the event: the host keeps where the menu opens.
fn event(state: &mut State, tree: &mut UiTree, event: UiEvent) -> Vec<UiAction> {
    match event {
        // The open menu's backdrop covers the row, so look for the row's own region.
        UiEvent::ContextMenu { position }
            if tree.frame().regions.iter().any(|region| region.id == "channel" && region.rect.contains(position)) =>
        {
            state.channel_menu_at = Some(position);
            Vec::new()
        }
        // The menu key and Shift+F10 open it under the row, where the focus is.
        UiEvent::ContextMenuKeyboard if tree.focused_id() == Some("channel") => {
            let row = tree.frame().regions.iter().rev().find(|region| region.id == "channel");
            state.channel_menu_at = row.map(|row| Point::new(row.rect.x, row.rect.y + row.rect.height));
            Vec::new()
        }
        event => tree.handle_event(event),
    }
}
```

The point is in the tree's coordinates, as `UiEvent::ContextMenu` gives it.
The host clears it on the dismiss ID and on the items' IDs. The arrow keys
reach the menu wherever the focus is while it is open. A secondary click
outside the menu reports the dismiss ID, as a primary one does; one on the
row the menu belongs to reaches the host first and moves the menu there.

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
| Up, Down, Home, End, Tab | Move through the enabled items; headings, separators and disabled items are skipped. In a menu at a point, Up and Down enter it from wherever the focus is. |
| Right, Left | Enter a submenu and return to its row, at any depth. |
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
| `UiAction::Clicked { id }` with the dismiss ID | Escape, or a click outside the menu, primary or secondary. |

## Accessibility

The platform is told each item is a menu item named by its label, with its
annotation or its second line as a description, and that a disabled item is
there and cannot be used. In a menu where an item is `selected`, every item
says whether it is selected; in one without, none does. A `checkbox` item is a
check box of a menu and a `radio` item a radio item of a menu, each saying
whether it is on. The trigger is the host's: the menu adds to it whether it is
open, and a section's heading says whether it is expanded.

## Limits

- A menu at a point does not open itself. The host catches
  `UiEvent::ContextMenu` and `UiEvent::ContextMenuKeyboard` before the tree,
  keeps the point and passes it to `at`, as in [At a point](#at-a-point).
- A [ContextMenu](context-menu.md), which does open itself, has one flat list
  of actions: no submenus, check boxes, radio items or second lines.
- A two-line item ignores `shortcut`, `annotation` and `destructive`, and its
  second line is one line, cut with an ellipsis.
- The menu reports IDs; it keeps no value. A check box or a radio item shows
  the value the host passes, and the host changes it.
