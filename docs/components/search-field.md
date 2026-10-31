# SearchField

A text field with a magnifier and an optional shortcut hint, for a query the
host searches with.

## Import

```rust
use limn::{Component, SearchField};
```

`Search` is another name for the same type.

## Usage

A search field is controlled: pass the query each frame and store the one it
reports. It takes an ID, the query, a placeholder and the palette.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    SearchField::new("search", &state.query, "Search", palette)
        .width(360.0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::InputChanged { id, value } = action {
        if id == "search" {
            state.query = value.clone();
        }
    }
}
```

## Shortcut

```rust
SearchField::new("search", &state.query, "Search", palette)
    .shortcut("Ctrl+K")
    .render()
```

`shortcut` shows the keys that reach the field, inside it. It is a hint: the
host handles the shortcut itself.

## Clear button

```rust
SearchField::new("search", &state.query, "Search", palette)
    .clearable(true)
    .render()
```

`clearable(true)` shows a circled cross at the end of the field while the
query is not empty. It is a button of its own, reached by Tab after the
field, and it reports `Clicked` with the field's ID and `:clear`. The field
does not empty itself: the host clears its query, gives the field its new
text, since a field keeps the text typed in it, and may give it the focus
back.

```rust
fn update(state: &mut State, tree: &mut UiTree, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "search:clear" {
            state.query.clear();
            tree.set_input_value("search", "");
            tree.focus("search");
        }
    }
}
```

The text stops before the button, as it starts after the magnifier, and the
field stays lit while the pointer is on the button. A shortcut hint shows
only while the field is empty, so the two never meet.

## Size

`size` takes a `ControlSize`; the default is `Spacious`. `width` sets the
width, and a value that is not finite is ignored.

## Height and placeholder

```rust
SearchField::new("search", &state.query, "Search", palette)
    .height(40.0)
    .persistent_placeholder(true)
    .render()
```

`height` sets a height in logical pixels for a field taller than its
`ControlSize`, as a settings page's 40 is. The field is never shorter than
its control size, and a value that is not finite is ignored.
`persistent_placeholder(true)` keeps the placeholder visible while the field
is focused and empty; by default it goes at the first key, as in a text field.

## Disabled

```rust
SearchField::new("search", "", "Search", palette).disabled().render()
SearchField::new("search", &state.query, "Search", palette).enabled(state.indexed).render()
```

A disabled field shows what the enabled one shows away from the focus: its
magnifier, the query or the placeholder in the muted ink, the shortcut hint
while there is no query and the clear button while there is one. It is not
an input, and the button is passive too.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Reports `Submitted` with the query |
| Tab | From the field, moves to its clear button while it has one |
| Enter, Space | On the clear button, report its click |

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The query changes. |
| `UiAction::Submitted { id, value }` | Enter is pressed in the field. |
| `UiAction::Clicked { id }` with `"{id}:clear"` | The clear button of a `clearable` field is activated. |

## Accessibility

The platform is told a search field named by its placeholder, and the query
it holds. Where the placeholder is not its name, give one with
`accessibility_label`. The clear button is a button named by
`Labels::clear_search`, "Clear search" unless the host sets its labels. A disabled field is still there and says it cannot be
used.

```rust
SearchField::new("search", query, "Type to filter", palette).accessibility_label("Search files")
```
