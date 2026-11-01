# CodeDiff

A read-only view of the rows of a diff: line numbers, a mark for each added
or removed row, and the text. It scrolls and builds only the rows in view.

## Import

```rust
use limn::{CodeDiff, CodeDiffLine, CodeDiffLineKind, CodeLanguage, Component, DiffColors, DiffFileHeader};
```

## Usage

The host supplies the rows; the view never computes or applies a patch, and
it shows each row's text exactly as given. A row is a `CodeDiffLine`: its
kind, its number on the old side and on the new side, and its text.

```rust
fn rows() -> Arc<[CodeDiffLine]> {
    vec![
        CodeDiffLine { kind: CodeDiffLineKind::Context, old_line: Some(41), new_line: Some(41), text: "fn total() -> u32 {".into() },
        CodeDiffLine { kind: CodeDiffLineKind::Removed, old_line: Some(42), new_line: None, text: "    0".into() },
        CodeDiffLine { kind: CodeDiffLineKind::Added, old_line: None, new_line: Some(42), text: "    1".into() },
    ]
    .into()
}

fn view(state: &State, palette: ComponentPalette) -> Element {
    CodeDiff::new("review", state.rows.clone()).palette(palette).render()
}
```

Keep the rows in the state as an `Arc<[CodeDiffLine]>` and pass a clone of it
each frame. The view colors its rows once for each allocation, so rows
rebuilt every frame are colored again every frame. Without `palette` the
diff uses the neutral dark palette.

The diff fills the width it is given. It reports an action only for a folded
row; see [Folded rows](#folded-rows).

## Height

```rust
CodeDiff::new("review", rows).height(240.0).render()
```

`height` is in logical pixels, never under 24; the default is 320. Rows past
it scroll under the wheel or the scrollbar.

`font_size(size)` changes the text, gutter and row layout together, 13 logical
pixels by default. Invalid numbers and nonpositive sizes are ignored; valid
sizes are kept within 1 to 96. `diff_backgrounds(false)` hides the full row
tints while preserving the +/- marks and colored edge bands.

## Changed words

```rust
CodeDiff::new("review", state.rows.clone()).word_ranges(state.words.clone()).render()
```

`word_ranges` takes the changed words of each row, as an `Arc<[Vec<Range<usize>>]>`:
entry `i` holds the ranges of `lines[i]`, in Unicode scalar offsets into its
text, end-exclusive. On added and removed rows they are tinted more strongly
than the row; on other rows, and past the end of the list, nothing is drawn.
Keep them in the state beside the rows.

## Colors and the margin bar

```rust
CodeDiff::new("review", rows).colors(DiffColors { add_row, ..DiffColors::default() }).margin_bar(false).render()
```

`DiffColors` holds the tint of an added and a removed row (`add_row`,
`del_row`), of their changed words (`add_word`, `del_word`), and the color of
their marker and margin bar (`add_bar`, `del_bar`). The default is the diff's
own green and red. `margin_bar(false)` hides the 2 px bar at the left edge of
changed rows; it is shown by default.

## Folded rows

```rust
CodeDiffLine { kind: CodeDiffLineKind::Folded, old_line: None, new_line: None, text: "Show 12 more lines".into() }
```

A row of kind `Folded` stands for unchanged lines the host left out. It shows
its text as a label, without line numbers, across a band of the palette's
`hover`. Activating it reports `Clicked { id: "{id}:expand:{row}" }`, where
`row` is its index in `lines`; the host replaces it with the lines. Syntax
coloring starts again after it.

## File header

```rust
DiffFileHeader::new("file:src/main.rs", "src/main.rs", palette)
    .stat(state.added, state.removed)
    .collapsed(state.collapsed)
    .render()
```

`DiffFileHeader` is the bar above a file's diff: a chevron, the path with the
file's name in the text color and its folder muted, and, with `stat`, `+N` in
`add_bar` and `−M` in `del_bar`. `colors` takes the same `DiffColors` as the
diff. The chevron points down, or right with `collapsed(true)`. Activating
the bar reports its ID; the host flips `collapsed` and leaves out the diff.

## Language

```rust
CodeDiff::new("review", rows).language(CodeLanguage::Rust).render()
```

`language` chooses how the text is colored: comments, strings, numbers and
the keywords of that language. The default, `CodeLanguage::Plain`, colors
nothing. `tab_width` is how many columns a tab takes, from 1 to 16; the
default is 4.

## One gutter

```rust
CodeDiff::new("captured", rows).single_gutter(true).render()
```

`single_gutter(true)` draws one column of line numbers, the new number or
else the old one, and no column of marks. Added and removed rows keep their
colors.

## Focus

```rust
CodeDiff::new("document", rows).focusable(true).render()
```

A diff takes no focus by default, as a preview inside a conversation.
`focusable(true)` lets a click or Tab focus it, for a diff that stands as a
document. It stays read-only: typing, pasting and deleting change nothing.

## Following the last row

```rust
CodeDiff::new("stream", rows).follow_tail(true).render()
```

With `follow_tail(true)` a diff that is at its last row stays there as rows
are appended. Scrolling up stops it, and scrolling back to the end resumes
it.

## Actions

| Action | When |
| --- | --- |
| `Clicked { id: "{id}:expand:{row}" }` | A `Folded` row is activated. |
| `Clicked { id }` on a `DiffFileHeader` | The header is activated. |

## Limits

The control of a folded row is laid over the diff as it is painted, as the
fold controls of a `CodeEditor` are: it takes the pointer, but not Tab.
