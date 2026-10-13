# OutputBlock

Read-only monospaced output of a command or a tool, bounded however much text
it is given.

## Import

```rust
use limn::{Component, OutputBlock};
```

## Usage

An output block needs an ID, the text and the palette. The ID names its
scroll area and must be unique in the window. It reports no actions.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    OutputBlock::new("test-output", &state.output, palette).render()
}
```

Each line of the text is one clipped row, in a scroll area that builds only
the rows in view. The fill is the palette's `surface`, so pass a palette whose
surface is the host's code well.

## Limits

The text is cut to 48 KiB, 600 lines and 500 characters per line before any of
it is shaped. A line that was cut ends with an ellipsis.

```rust
OutputBlock::new("test-output", &state.output, palette)
    .truncation_note("Partial view: the output is too long")
    .render()
```

`truncation_note` adds a muted line under the output, shown only when lines or
bytes were cut.

## Height

```rust
OutputBlock::new("test-output", &state.output, palette).max_height(136.0).render()
```

The block is as tall as its lines up to `max_height`, then it scrolls. The
default is 200, and the least is one line. Values that are not finite are
ignored.

## Ink

```rust
OutputBlock::new("stderr", &state.errors, palette).ink(error_color).render()
```

`ink` paints the lines in a color that carries meaning the host owns, such as
an error stream, in place of the palette's text.

## Width

```rust
OutputBlock::new("test-output", &state.output, palette).width(520.0).render()
```

`width` fixes the width. Without it the block fills the width it is given.
Values that are negative or not finite are ignored.
