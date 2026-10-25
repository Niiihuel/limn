# TerminalView

A terminal's screen and its scrollback, painted from a grid of cells the host
keeps. limn runs no shell and reads no escape sequences: the host's pseudo
terminal and emulator do, and hand the view what they would show.

## Import

```rust
use limn::{Component, TerminalCell, TerminalCursor, TerminalCursorShape, TerminalScreen, TerminalView};
```

## Usage

The host keeps a `TerminalScreen` in its state, as an `Arc`, and passes a
clone of it each frame.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    TerminalView::new("shell", state.screen.clone(), palette).render()
}
```

A `TerminalScreen` is `cols` by `rows` cells, row by row from the top, the
lines scrolled off above it in `scrollback`, oldest first, and the cursor. A
`TerminalCell` is a character, its ink and fill, and whether it is bold,
underlined or inverse. Colors arrive resolved: the host turns the 16, 256 or
true colors a program asks for into `Color`s.

The view fills the box it is given. Give it a height, or `flex(1.0)` in a
column.

## Keyboard

A press on the view, or Tab, gives it the keyboard focus under its ID. It
reports nothing: while `tree.focused_id()` is its ID, the host writes the
keys to its terminal instead of handing them to the tree, and hands the tree
only what it keeps for itself, such as Ctrl+Shift+C to copy.

| Key | Effect |
| --- | --- |
| Ctrl+C, Ctrl+Shift+C | Given to the tree with text selected, reports `CopyRequested` with it. |
| Tab, Shift+Tab | Given to the tree, moves the focus on. A host that wants Tab for its shell keeps it. |

## Size of the grid

`TerminalView::grid_size(size, font_size)` gives the columns and rows of cells
that fit in a size, at least one of each. Call it when the view's box changes
and resize the pseudo terminal to it.

```rust
let (cols, rows) = TerminalView::grid_size(Size::new(width, height), 13.0);
```

A cell is as wide as a character of Hack, the monospace face limn bundles, and
a row is 1.3 times the font size. `font_size(size)` sets the text, 13 logical
pixels by default and kept within 1 to 96.

## Cursor

`TerminalCursor` gives the cursor's column and row on the screen and its
shape: a `Block` over the cell, the character drawn in the cell's fill on it,
or a 2 px `Bar` at its left edge. It is drawn in the palette's `text`, only
while the view has the focus. A cursor of `None`, or out of the screen, is
not drawn.

## Selection

Dragging over the text selects it, across lines; a double press selects a
word. `CopyRequested` carries the selected text with a line break between
rows and without the blank cells that end a row. A change to the screen's
text drops the selection.

## Scrolling

The screen and its scrollback scroll together in one area, whose ID is
`{id}:scroll`. The view starts at the top; to follow the output, ask the
tree before the screen changes whether the reader is at the end, and keep
them there after it:

```rust
let follow = tree.is_scroll_at_end("shell:scroll", 4.0);
state.screen = Arc::new(next_screen);
if follow {
    tree.scroll_to_end("shell:scroll");
}
```

## Colors

`background(color)` fills behind the cells that have no fill of their own;
unset, the palette's `surface`. An inverse cell draws its character in its
fill, or the background, on a fill of its ink.

`min_contrast(ratio)` moves each character's ink toward white or black until
it stands at that contrast on its fill, as a program's navy on black is made
readable. It is kept within 1 to 21.

## Motion

The cursor blinks on the clock text fields' carets share: shown for 500 ms,
hidden for 500 ms, and shown again at once when it moves. Under reduced
motion it stays shown.

## Actions

| Action | When |
| --- | --- |
| `CopyRequested { text }` | Ctrl+C reaches the tree with text selected. |

## Limits

- A bold cell is drawn in the regular face: limn has no bold monospace face.
  A host that brightens bold text does so in the cell's ink.
- Every character takes one cell. A wide character, such as a CJK one or an
  emoji, is drawn wider than its cell and pushes the rest of its row right.
- The grid assumes Hack. A host that gives the tree another monospace face
  gets text that drifts from the fills and the cursor.
- The whole scrollback is laid out each frame as one text. Keep it to a few
  thousand lines.
