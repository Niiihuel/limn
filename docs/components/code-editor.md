# CodeEditor

A code editor over a text buffer the host owns: carets and selections, undo,
line numbers, syntax colors, and optional soft wrap, minimap, status bar and
Vim mode. The host opens and saves files; the editor never touches one.

## Import

```rust
use limn::{CodeEditor, CodeLanguage, Component, EditorController};
use limn_editor::TextBuffer;
```

The buffer comes from the `limn-editor` crate, which the host adds to its
dependencies beside `limn`.

## Usage

The text lives in a `TextBuffer` behind an `EditorController`. Create the
controller once, keep it in the state, and pass a clone of it to the editor
each frame. The editor edits the buffer in place and reports that it did.

```rust
fn open(text: &str) -> Result<EditorController, limn_editor::EditorError> {
    Ok(EditorController::new(TextBuffer::new(text)?))
}

fn view(state: &State, palette: ComponentPalette) -> Element {
    CodeEditor::new("source", state.controller.clone())
        .palette(palette)
        .language(CodeLanguage::Rust)
        .height(420.0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::EditorChanged { id, revision, .. } if id == "source" => state.edited(*revision),
        UiAction::EditorSaveRequested { id, .. } if id == "source" => {
            if let Ok(snapshot) = state.controller.snapshot() {
                state.save(&snapshot);
            }
        }
        UiAction::EditorFailed { id, error } if id == "source" => state.report(error),
        _ => {}
    }
}
```

`TextBuffer::new` fails for a text larger than the buffer's limit. Without
`palette` the editor uses the neutral dark palette. It fills the width it is
given, and the tree keeps its scroll position under its ID.

## Reading the text

`controller.snapshot()` gives a `BufferSnapshot`: the text at one revision,
which the host can read without holding the buffer's lock.
`snapshot.write_to(writer)` writes it out, `snapshot.line_text(row)` gives
one line, and `snapshot.document_id()` and `snapshot.revision()` are the
numbers the editor's actions carry. After a save,
`controller.shared_buffer()` gives the buffer to lock, and
`mark_saved_snapshot(&snapshot)` on it records that revision as the saved
one.

## Language and height

`language` takes a `CodeLanguage`, which chooses the syntax colors; the
default, `CodeLanguage::Plain`, colors nothing. `height` is in logical
pixels, never under 24, and 320 by default.

## Options

```rust
CodeEditor::new("source", controller)
    .line_numbers(false)
    .soft_wrap(true)
    .minimap(true)
    .status_bar(true)
    .tab_width(2)
    .render()
```

| Method | Default | What it does |
| --- | --- | --- |
| `line_numbers(bool)` | on | The column of line numbers |
| `soft_wrap(bool)` | off | Wraps long lines to the editor's width |
| `minimap(bool)` | off | An overview of the document to navigate by; a narrow editor hides it |
| `status_bar(bool)` | off | A bar under the text with the caret, the language, the line ending and whether the document is saved |
| `tab_width(usize)` | 4 | Columns of a tab and of an indent level, from 1 to 16 |
| `indent_style(IndentStyle)` | `IndentStyle::Spaces(4)` | Whether Tab indents with spaces or with tabs, and by how many columns |
| `auto_pairs(bool)` | on | Typing an opening bracket or quote also inserts its closing one |
| `vim_mode(bool)` | off | Modal editing with Vim's keys |

`IndentStyle` is in `limn_editor`.

## Read-only

```rust
CodeEditor::new("source", controller).read_only(true).render()
```

A read-only editor still moves the caret, selects and copies. Typing, pasting, deleting, Enter, undo and redo change nothing, and
it does not take Tab.

## Decorations

The host can draw its own analysis over the text: `highlights` for syntax
colors of its own, `diagnostics` for ranges with a message that shows when
the pointer rests on one, `search_matches` for the matches of a search,
`structure` for folds and sticky scopes, `breakpoints` and `debugger` for
the gutter, and `review` for the hunks of a change. Each takes the document
ID and the revision of the snapshot it was computed for, or the revision
alone for `search_matches`: that is how the editor tells whether a result
belongs to the text it is showing. `context_actions` takes the entries of
the editor's [context menu](context-menu.md).

For find and replace, `controller.search(query, &options)` searches the
buffer, and [Editing tools](editor.md) has the bar that goes with it.
`reveal_top_inset` keeps the caret below a bar that floats over the top of
the editor.

## Keyboard

| Key | Does |
| --- | --- |
| Arrows | Move the caret; with Ctrl, Left and Right move by a word |
| Home, End | To the first character of the line after its indentation, then to its first column on a second Home, and to its end; with Ctrl, to the start and the end of the document |
| Page Up, Page Down | Move by the lines the editor shows |
| Shift with any of those | Extends the selection |
| Ctrl+A | Selects everything |
| Ctrl+D | Also selects the next occurrence of the selection; with Shift, every occurrence |
| Tab, Shift+Tab | Indent and outdent |
| Backspace, Delete | Delete a character; with Ctrl, a word |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| Ctrl+C, Ctrl+X | Report `CopyRequested` with the selected text; Ctrl+X also deletes it |
| Ctrl+S | Reports `EditorSaveRequested` |

## Actions

| Action | When |
| --- | --- |
| `UiAction::EditorChanged { id, document_id, revision }` | An edit changed the text; `revision` is the buffer's new revision. |
| `UiAction::EditorSaveRequested { id, document_id, revision }` | Ctrl+S is pressed. The host saves. |
| `UiAction::CopyRequested { text }` | Ctrl+C or Ctrl+X with a selection. |
| `UiAction::EditorFailed { id, error }` | An edit could not be applied, or a selection is too large to copy. |
| `UiAction::Clicked { id }` with `{prefix}:{row}` | A row of the breakpoint gutter is clicked, with the prefix given to `breakpoints` and the row counted from zero. |

## Accessibility

The platform is told the editor is a multiline text field.
