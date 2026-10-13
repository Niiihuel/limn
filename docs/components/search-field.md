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

## Size

`size` takes a `ControlSize`; the default is `Spacious`. `width` sets the
width, and a value that is not finite is ignored.

## Disabled

```rust
SearchField::new("search", "", "Search", palette).disabled().render()
SearchField::new("search", &state.query, "Search", palette).enabled(state.indexed).render()
```

A disabled field keeps its magnifier and its shortcut hint, shows the query
or the placeholder in the muted ink, and is not an input.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Reports `Submitted` with the query |

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The query changes. |
| `UiAction::Submitted { id, value }` | Enter is pressed in the field. |

## Accessibility

The platform is told a search field named by its placeholder, and the query
it holds. Where the placeholder is not its name, give one with
`accessibility_label`. A disabled field is still there and says it cannot be
used.

```rust
SearchField::new("search", query, "Type to filter", palette).accessibility_label("Search files")
```
