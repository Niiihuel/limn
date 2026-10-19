# TextField

A single line of text: a name, a path, a password. For several lines use
[TextArea](text-area.md), and for a query use [SearchField](search-field.md).

## Import

```rust
use limn::{Component, ControlSize, Icon, TextField};
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

## Icon

```rust
TextField::new("channel-name", &state.channel, "new-channel", palette)
    .icon(Icon::Hash)
    .render()
```

`icon` puts an icon before the text, inside the field, as a
[SearchField](search-field.md) has its magnifier. It is decorative: it is
muted until the field has the focus, takes the text's ink while it does, and
the text, the caret and the placeholder start 8 after it. A disabled field
keeps it where it was, muted.

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

## Input methods

In a tree that lays its text out (`UiTree::set_text_layout`, as
`limn_egui::attach_fonts` does), what an input method is composing shows at
the caret, underlined, in place of any selection. It becomes part of the
value, and reports `InputChanged`, only when the input method commits it;
a cancelled composition leaves the value and the selection as they were.
`limn_egui::report_ime` tells the platform where the caret is, once a
frame, so the candidate window opens beside it.

A password composes nothing: the platform is asked for no composition or
prediction in it.

## Pointer

A press puts the caret nearest it, and dragging selects from there.
Shift and a press extend the selection of a field that has the focus. A
double press selects the word under the pointer, or the run of spaces or
the mark; a triple press selects the whole value. A password has no words:
Ctrl and an arrow go to its ends, and a double press selects it whole.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Reports `Submitted` with the value |
| Left, Right | Move the caret a character, an emoji with its modifiers whole; a selection collapses to the edge the arrow points at. With Shift, extend the selection |
| Ctrl+Left, Ctrl+Right | Move the caret to the start of the word before it or the end of the word after it |
| Backspace, Delete | Delete the selection, or the character before or after the caret; with Ctrl, the word |
| Home, End | Move the caret to either end |
| Ctrl+A | Selects everything |

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text changes. |
| `UiAction::Submitted { id, value }` | Enter is pressed in the field. |

## Accessibility

The platform is told a text field, or a password field, named by its
placeholder, and the text it holds; what is typed into a password field is
never told. A placeholder that is an example is not a name: give one with
`accessibility_label`, or with the label of a [Form field](form-field.md).
A disabled field is still there and says it cannot be used.

```rust
TextField::new("project", value, "e.g. my-project", palette).accessibility_label("Project name")
```
