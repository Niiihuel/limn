# TextField

A single line of text: a name, a path, a password. For several lines use
[TextArea](text-area.md), and for a query use [SearchField](search-field.md).

## Import

```rust
use limn::{Component, ControlSize, TextField};
```

## Usage

A text field is controlled: pass its value each frame and store the one it
reports. It takes an ID, the value, a placeholder and the palette.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    TextField::new("project-name", &state.name, "Project name", palette)
        .width(360.0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "project-name" => {
            state.name = value.clone();
        }
        UiAction::Submitted { id, .. } if id == "project-name" => state.create_project(),
        _ => {}
    }
}
```

## Password

```rust
TextField::new("password", &state.password, "Password", palette)
    .password(true)
    .render()
```

`password(true)` hides the characters that are typed.

## Monospace

```rust
TextField::new("path", &state.path, "src/main.rs", palette)
    .monospace(true)
    .render()
```

`monospace(true)` sets the value in the monospaced family, for a path, an
identifier or a number.

## Size

```rust
TextField::new("server", &state.server, "e.g. filesystem", palette)
    .size(ControlSize::Compact)
    .width(360.0)
    .height(40.0)
    .render()
```

`size` takes a `ControlSize`; the default is `Regular`. `height` is for a
form whose inputs are taller than a control size: the field is never shorter
than its control size. A width or height that is not finite is ignored.

## Placeholder

`persistent_placeholder(true)` keeps the placeholder visible while the field
is focused and empty.

## Disabled

```rust
TextField::new("owner", "Not editable", "", palette).disabled().render()
TextField::new("owner", &state.owner, "", palette).enabled(state.can_edit).render()
```

A disabled field shows its value in the muted ink, or its placeholder when
the value is empty, and is not an input: it reports nothing.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Reports `Submitted` with the value |
| Home, End | Move the caret to either end |
| Ctrl+A | Selects everything |

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text changes. |
| `UiAction::Submitted { id, value }` | Enter is pressed in the field. |
