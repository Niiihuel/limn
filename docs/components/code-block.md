# CodeBlock

The card that holds a block of code inside prose: wrapped, monospaced and
passive, under an optional label such as the language.

## Import

```rust
use limn::{CodeBlock, CodeBlockSizing, Component, FontFamily, TextSpan};
```

## Usage

A code block needs its text and the palette. It reports no actions.

```rust
fn view(palette: ComponentPalette) -> Element {
    CodeBlock::new("cargo test -p limn", palette).label("SH").render()
}
```

The code is set on the palette's `raised` fill inside a hairline in its
border color.

## Label

```rust
CodeBlock::new(source, palette).label("RUST").render()
```

`label` adds a muted line above the code. The host writes it in the case it
wants. Without it the block is the code alone.

## Sizing

```rust
CodeBlock::new(source, palette)
    .label("rust")
    .sizing(CodeBlockSizing::DOCUMENT)
    .render()
```

`CodeBlockSizing::CHAT`, the default, is for a conversation: a small
monospaced label over 12 px code. `DOCUMENT` is a sans label over 13 px code.
`CodeBlockSizing` is a struct of the label's text role, whether the label is
monospaced, and the code's text role, so a host can write a profile of its
own. The code is always monospaced.

## Highlighted code

```rust
let spans = vec![
    TextSpan::new("fn ", palette.muted, 13.0, FontFamily::Monospace),
    TextSpan::new("main", palette.text, 13.0, FontFamily::Monospace),
];
CodeBlock::new("", palette).spans(spans).render()
```

`spans` paints text the host's own highlighter already colored, in place of
the plain text. Each span carries its own color, size and family.

## Width

```rust
CodeBlock::new(source, palette).width(180.0).render()
```

`width` fixes the width and wraps the code inside it. Without it the block
fills the width it is given. Values that are negative or not finite are
ignored.
