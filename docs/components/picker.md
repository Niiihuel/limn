# Catalog menus

A menu that lists more than actions: a selector with a searchable catalog,
folding sections and two-line rows. There is no `Picker` type; it is a
[DropdownMenu](dropdown-menu.md) with a `MenuHeader`, `DropdownMenuSection`s
and items that have a description. `MenuItem` is the same row on its own, and
`PopoverPanel` the layout of a [Popover](popover.md) that is not a menu.

## Import

```rust
use limn::{
    Component, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownMenuSection,
    MenuHeader, MenuItem, PopoverPanel, SelectorSizing,
};
```

## Usage

The menu wraps its trigger where the trigger is built, and the host owns the
open flag, the query and the choice. An item's ID is the action it reports.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let width = SelectorSizing::CATALOG.preferred_width;
    let entries = state
        .models_matching(&state.query)
        .map(|model| {
            DropdownMenuItem::new(format!("model={}", model.id), &model.name)
                .description(&model.id)
                .detail_mono(true)
                .selected(model.id == state.model)
                .into()
        })
        .collect();
    DropdownMenu::new(
        Button::new("models", "Model", palette).render(),
        entries,
        state.models_open,
        "models:dismiss",
        palette,
    )
    .header(
        MenuHeader::new(palette)
            .title("Model")
            .search("models:search", &state.query, "Search models")
            .width(width - 8.0)
            .render(),
    )
    .context_menu_style()
    .width(width)
    .max_visible_items(6)
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "models:search" => state.query = value.clone(),
        UiAction::Clicked { id } if id == "models" => state.models_open = true,
        UiAction::Clicked { id } if id == "models:dismiss" => state.models_open = false,
        UiAction::Clicked { id } => {
            if let Some(model) = id.strip_prefix("model=") {
                state.choose_model(model);
            }
        }
        _ => {}
    }
}
```

The host filters the entries itself: the menu shows what it is given.

## Two-line items

```rust
DropdownMenuItem::new("model=north/large-2", "North Large")
    .icon(Icon::Server)
    .description("north/large-2")
    .detail_mono(true)
    .status("Reviewed")
    .status_width(84.0)
    .selected(true)
    .into()
```

With `description` an item is a two-line row, `SelectorSizing::detail_row_height`
(40) tall, with a column for the check. It takes the menu's `sizing` and the
sides, icon slot, label size and gap of its one-line rows, so a menu with a
profile of its own gets two-line rows that line up with its other rows; with
`SelectorSizing::COMPACT` or `CATALOG` the row is as it always was. `detail_mono` sets the second line in
monospace, `status` puts short muted text at its end and `status_width` fixes
that slot so the statuses start in a column. `chevron()` ends the row with a
chevron instead of the check, for a row that opens another view; with a
`submenu` the row ends with a chevron and opens it. A two-line item ignores
`shortcut`, `annotation` and `destructive`.

`truncate()` cuts a long label to one line in a menu with
`context_menu_style()`, where it would otherwise wrap; pair it with
`tooltip_with` to say the rest.

## Header and footer

```rust
MenuHeader::new(palette)
    .back("models:back", "Model")
    .trailing(refresh_button)
    .search("models:search", &state.query, "Search models")
    .render()
```

`DropdownMenu::header` keeps an element above the options while they scroll,
and `footer` keeps one under them, below a hairline. `MenuHeader` is the usual
header: a `title`, or a `back` button that reports its ID, one square
`trailing` tool and a `search` field under the row. `width` fixes its width.

## Sections

```rust
DropdownMenuSection::new("fold:north", "North")
    .icon(Icon::Server)
    .count(2)
    .collapsed(state.north_folded)
    .into()
```

A section is a heading that folds. It reports its ID when activated; the host
flips its own flag and leaves the group's items out of the entries while the
section is collapsed.

## Loading, empty and error

```rust
DropdownMenuEntry::Loading { id: "branches:loading".into(), rows: 3 }
DropdownMenuEntry::Message("No branches yet.".into())
DropdownMenuEntry::Error { id: "branches:error".into(), text: error.to_string() }
```

`Loading` shows skeleton rows, `Message` a muted sentence and `Error` a
failure in a short scroll area of its own.

## Shared sizes

`SelectorSizing::COMPACT` (256 wide, 8 rows before scrolling) is for short
lists of actions and `SelectorSizing::CATALOG` (320 wide, 6 rows) for
searchable catalogs. Read a field such as `preferred_width`, or pass the
profile to `DropdownMenu::sizing`.

## Rows on their own

```rust
MenuItem::new("terminal=1", "Terminal 1", palette)
    .icon(Icon::Terminal)
    .trailing("Active")
    .trailing_action("terminal-1-close", Icon::Trash, TooltipSpec::new("Close terminal"))
    .trailing_dispatch_as("terminal:close=1")
    .width(300.0)
    .render()
```

`MenuItem` is a menu row for a list the host lays out itself. It takes `icon`,
`trailing` text or a `shortcut`, `destructive`, `tooltip_with`, and the
two-line options above: `description`, `detail_mono`, `status`, `status_width`,
`chevron` and `selected`. It is 220 wide unless `width` is set.
`enabled(false)` makes it passive. `trailing_action` adds a second square
action at the end of the row, reached by Tab and not by the arrow keys;
`trailing_dispatch_as` makes it report another ID.

## A panel in a popover

```rust
Popover::new(
    trigger,
    PopoverPanel::new(palette)
        .header(PanelHeader::new("Account", palette).dismiss("account:dismiss", "Close").render())
        .body(ScrollArea::new("account:body", rows).render())
        .body_max_height(180.0)
        .footer(Button::new("account:details", "See usage", palette).render())
        .render(),
    state.account_open,
    "account:dismiss",
    palette,
)
.render()
```

`PopoverPanel` stacks a header, a body and a footer with hairlines between
them. The body gives way first in a short window; `body_max_height` caps it,
for a body that is a scroll area. `sizing(PopoverPanelSizing { gap, padding,
header_rule })` sets another rhythm; `PopoverPanelSizing::DEFAULT` is 4, 0 and
a hairline under the header. The popover keeps placement, width and surface.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's or a row's ID | The item is activated. |
| `UiAction::Clicked { id }` with a section's ID | Its heading is activated. |
| `UiAction::Clicked { id }` with the back ID | The header's back button. |
| `UiAction::InputChanged { id, value }` with the search ID | The query changes. |
| `UiAction::Clicked { id }` with the dismiss ID | Escape, or a click outside the menu. |

## Accessibility

A `MenuItem` is a menu item named by its label. Its trailing text or its
description is said after the name, a two-line row given `selected` says
whether it is, and a disabled one is there and cannot be used. A trailing
action is a button named by the title of its hint.
