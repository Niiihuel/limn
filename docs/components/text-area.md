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

## Highlights

```rust
Element::text_area("message", &state.message, "Message").text_area_highlights(vec![TextHighlight {
    range: 5..9,
    color: palette.link(),
    italic: false,
    background: Some(Color::rgba(88, 101, 242, 40)),
}])
```

`text_area_highlights` repaints ranges of a bare `Element::text_area`, by
Unicode character offsets, and `Composer::editor_highlights` those of the
composer's editor: `color` inks the range, `italic` slants it and
`background` fills behind it with rounded corners, under the selection and
the text, such as a mention drawn as the pill it becomes once sent. The fill
covers the range's characters and the line's height. In a tree that lays
its text out (`UiTree::set_text_layout`, as `limn_egui::attach_fonts` does)
a highlight is a span of the text: an italic range is set in the italic
face, and the caret and the wrapping follow it. Otherwise the range is
painted over the plain text, whose advances the caret and the wrapping
keep. Highlights follow the host's text; while an edit has not reached the
host yet, the text paints plain.
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

## Input methods

In a tree that lays its text out (`UiTree::set_text_layout`, as
`limn_egui::attach_fonts` does), what an input method is composing shows at
the caret, underlined, in place of any selection. It becomes part of the
value, and reports `InputChanged`, only when the input method commits it;
a cancelled composition leaves the value and the selection as they were.
`limn_egui::report_ime` tells the platform where the caret is, once a
frame, so the candidate window opens beside it.

## Pointer

A press puts the caret nearest it, and dragging selects from there.
Shift and a press extend the selection of a field that has the focus. A
double press selects the word under the pointer, or the run of spaces or
the mark; a triple press selects its line, up to the break. Dragging past
the top or the bottom scrolls the text to the caret.

## Keyboard

| Key | Effect |
| --- | --- |
| Enter | Adds a line; with `enter_submits(true)`, reports `Submitted` with the text |
| Shift+Enter | Adds a line |
| Ctrl+Enter | Reports `Submitted` with the text |
| Left, Right | Move the caret a character, an emoji with its modifiers whole; a selection collapses to the edge the arrow points at. With Shift, extend the selection |
| Ctrl+Left, Ctrl+Right | Move the caret to the start of the word before it or the end of the word after it |
| Backspace, Delete | Delete the selection, or the character before or after the caret; with Ctrl, the word |
| Up, Down | Move the caret a line, as near its column as the line allows |
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
