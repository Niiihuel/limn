# TextArea

Several lines of text: notes, instructions, a message. For one line use
[TextField](text-field.md).

## Import

```rust
use limn::{Component, RadiusRole, TextArea};
```

## Usage

A text area is controlled: pass its text each frame and store the one it
reports. It takes an ID, the text, a placeholder and the palette.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    TextArea::new("notes", &state.notes, "Notes", palette)
        .width(360.0)
        .height(120.0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "notes" => {
            state.notes = value.clone();
        }
        UiAction::Submitted { id, .. } if id == "notes" => state.save_notes(),
        _ => {}
    }
}
```

## Size

`height` is 96 by default and never less than 64. A width or height that is
not finite is ignored.

## Radius

```rust
TextArea::new("message", &state.message, "Message", palette)
    .radius(RadiusRole::Surface)
    .render()
```

`radius` rounds the box with another radius token. The default is
`RadiusRole::Control`.

## Sending on Enter

```rust
TextArea::new("message", &state.message, "Message #general", palette)
    .enter_submits(true)
    .render()
```

`enter_submits(true)` makes the area a chat composer: Enter reports
`Submitted` with the text and Shift+Enter adds a line. Ctrl+Enter submits
either way. The host clears the field after sending with
`tree.set_input_value(id, "")`. `Composer::enter_submits` does the same for
the composer's editor, and `Element::text_area_enter_submits` for a bare
`Element::text_area`.

## Line endings

`preserve_line_endings(true)` keeps CRLF and CR line endings through editing
and clipboard events.

## Disabled

```rust
TextArea::new("message", "", "No permission to write", palette).disabled().render()
TextArea::new("message", &state.message, "Message", palette).enabled(state.can_write).render()
```

A disabled area shows its text in the muted ink, or its placeholder when the
text is empty, and is not an input: it reports nothing.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Adds a line; with `enter_submits(true)`, reports `Submitted` with the text |
| Shift+Enter | Adds a line |
| Ctrl+Enter | Reports `Submitted` with the text |
| Home, End | Move the caret to either edge of its line; with Ctrl, of the text |
| Ctrl+A | Selects everything |

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text changes. |
| `UiAction::Submitted { id, value }` | Ctrl+Enter is pressed in the area, or Enter with `enter_submits(true)`. |

## Accessibility

The platform is told a multi-line field named by its placeholder, and the
text it holds. Where the placeholder is not its name, give one with
`accessibility_label`, or with the label of a [Form field](form-field.md).
A disabled area is still there and says it cannot be used.

```rust
TextArea::new("notes", value, "What changed?", palette).accessibility_label("Release notes")
```
