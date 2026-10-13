# Breadcrumb

A path through a hierarchy: folders, then the current resource. Segments with
an action are links, and the last segment is the current one.

## Import

```rust
use limn::{Breadcrumb, BreadcrumbItem, Component, Icon};
```

## Usage

A breadcrumb takes an ID, its segments and the palette. A segment with an
action reports `UiAction::Clicked` with that action's ID; a segment without
one is passive. The host supplies every label and action: the breadcrumb
infers no navigation.

```rust
fn view(palette: ComponentPalette) -> Element {
    Breadcrumb::new(
        "path",
        vec![
            BreadcrumbItem::new("limn").action("crumb=root").icon(Icon::Folder),
            BreadcrumbItem::new("crates").action("crumb=crates"),
            BreadcrumbItem::new("empty.rs").tooltip("crates/limn/src/empty.rs"),
        ],
        palette,
    )
    .width(420.0)
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(folder) = id.strip_prefix("crumb=") {
            state.open_folder(folder);
        }
    }
}
```

The bar is 320 wide and 28 tall by default; `width` and `height` change
that, and the height is never less than 20.

## Segments

`BreadcrumbItem::new(label)` takes `action` to make it a link, `icon` for a
glyph before the label, and `tooltip` for the hint's description, such as the
segment's full path.

## A file path

```rust
Breadcrumb::from_path("file-path", "crates/limn/src/find_bar.rs", palette)
    .dirty(state.unsaved)
    .segment_action(|segment| Some(format!("open={segment}")))
    .width(420.0)
    .render()
```

`from_path` splits a file path into one segment per folder and the file last,
with its file icon when the bar is at least 76 wide. Each segment's tooltip is
its own path. The segments are passive until `segment_action` gives them
actions: it is called with each folder's path from the root and, for the
file, with the whole path as it was given, and returning `None` leaves that
segment passive. `dirty(true)` marks unsaved changes with a dot after the
file's name.

```rust
Breadcrumb::from_path("file-path", path, palette)
    .item(BreadcrumbItem::new("render").action("symbol=render"))
    .render()
```

`item` adds one more segment after the last, such as a symbol inside the
file. It becomes the current resource.

## Collapsed segments

When the path does not fit, middle segments collapse into an ellipsis whose
hint is the complete path. If one of the hidden segments has an action, the
ellipsis opens a menu of them, and the host owns whether it is open:

```rust
Breadcrumb::new("path", items, palette)
    .overflow_open(state.path_menu_open)
    .render()
```

The ellipsis reports `{id}:overflow_toggle`, and dismissing the menu reports
`{id}:overflow_dismiss`, where `{id}` is the breadcrumb's ID. A choice in the
menu reports the segment's own action.

## Search and editing

```rust
Breadcrumb::new("path", items, palette).search_action("path-search").render()
Breadcrumb::new("path", items, palette)
    .editing("path-input", &state.location, "Go to a path")
    .render()
```

`search_action` ends the bar with a search button that reports the action;
the path gives up the button's width, so the bar keeps its size. `editing`
shows a one-line field in place of the path, for typing a location or a
search. The host owns its text through `InputChanged` and `Submitted` for the
input ID, and leaves this mode by not calling `editing`.

The button's hint shows `Ctrl+F`; `search_shortcut(keys)` shows the keys the
host binds instead, and an empty string none.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a segment's action | A link segment, or its entry in the menu, is activated. |
| `UiAction::Clicked { id }` with `{id}:overflow_toggle` | The ellipsis of collapsed links is activated. |
| `UiAction::Clicked { id }` with `{id}:overflow_dismiss` | The menu of collapsed segments is dismissed. |
| `UiAction::Clicked { id }` with the search action | The search button is activated. |
| `UiAction::InputChanged`, `UiAction::Submitted` with the input ID | The field shown by `editing` changes or is submitted. |
