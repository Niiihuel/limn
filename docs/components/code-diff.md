# CodeDiff

A read-only view of the rows of a diff: line numbers, a mark for each added
or removed row, and the text. It scrolls and builds only the rows in view.

## Import

```rust
use limn::{CodeDiff, CodeDiffLine, CodeDiffLineKind, CodeLanguage, Component};
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

The diff fills the width it is given. It reports no actions.

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
