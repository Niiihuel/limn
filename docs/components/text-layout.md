# TextLayout

Text that limn shapes, breaks into lines and places itself. Where the caret
goes, what a selection covers and the glyphs a backend draws all come from
one layout, so they always agree: no backend lays the text out again.

limn's components still lay their text out the older way, measuring widths
through the backend; they move to `TextLayout` one at a time. Today it is
what an application draws text with in an [`Element::drawing`](drawing.md).

## Import

```rust
use limn::{Color, Element, FontFamily, Point, TextLayout, TextStyle};
```

## Usage

```rust
let layout = TextLayout::new("Interfaces, drawn in Rust.", TextStyle::new(22.0, FontFamily::Sans));
let size = layout.size();
Element::drawing(move |_| layout.draw(Point::new(0.0, 0.0), ink))
    .size(Some(size.width), Some(size.height))
```

`TextLayout::new(text, style)` shapes the text: kerning, ligatures, marks,
right-to-left runs, and a fallback face for each character the family lacks.
It keeps one line for each line the text has. `size()` is as wide as its
widest line and as tall as its lines; empty text is one line tall.

`draw(origin, color)` returns one `DrawCommand::Glyphs` for each run of one
font, placed from `origin`. A monochrome glyph is drawn in `color`.

## Style

`TextStyle::new(size, family)` sets the size in logical pixels and the
`FontFamily`, which carries the weight and the slant. `.line_height(px)` sets
every line that far apart; without it the font's own ascent, descent and line
gap decide. A size that is not finite is 14 and one below zero is zero; a
line height that is not finite or below zero is ignored.

## Wrapping

```rust
let paragraph = TextLayout::new(text, style).wrapped(360.0);
```

`wrapped(width)` breaks the same text into lines no wider than `width`, where
the language allows a break, without shaping it again: a layout made once can
follow its container's width. A word longer than `width` keeps a line of its
own. A width that is not finite does not wrap.

`lines()` gives each line's box, its baseline and the bytes it holds. A line's
box leaves out the spaces that hang past its end where it wraps.

## Caret and selection

```rust
let caret = layout.caret(index);            // zero wide, as tall as its line
let index = layout.index_at(pointer);       // where a click puts the caret
let boxes = layout.selection(start..end);   // one box for each line it touches
```

Offsets are bytes into the text. One inside a character is taken as the
start of that character, and the caret never lands inside a grapheme. At a
line that wraps, the end of one line and the start of the
next are the same offset: the caret stands at the start of the next. An
offset past the end is the end, and a point that is not finite is the start.

## Fonts

The faces text is laid out in are one set for the whole process, as the
tokens are:

```rust
let ui = limn::Font::new(std::fs::read("Inter-Regular.ttf")?)?;
limn::set_fonts(limn::Fonts::bundled().face(FontFamily::Sans, ui));
```

`Fonts::bundled()`, with the default `bundled-fonts` feature, is limn's own:
Ubuntu for `Sans` in regular, medium and bold, Hack for `Monospace`, and Noto
Sans Math, Noto Sans Symbols 2 and Noto Emoji behind both. `.face(family,
font)` replaces a family's face; `.fallback(font)` adds a face tried, in
order, for characters the families lack. A family without a face of its own
takes `Sans`: `SansItalic` slanted and `SansBold` thickened. Behind `Sans`
come the fallbacks, then `Monospace`; behind `Monospace` come `Sans`, then the
fallbacks. Only the faces given are used, never the system's, so text is laid
out alike on every machine and in a browser.

`Font::new(bytes)` refuses bytes that hold no face; `Font::from_static` keeps a
face compiled into the program, which a backend can then use without copying.

## Drawing the glyphs

A backend draws each `GlyphRun` from `run.font.bytes()` at `run.size`, glyph
by glyph where the run places it, slanting the outlines by `run.skew` degrees
and thickening them when `run.embolden` asks. `run.text` is the text the run
shows, for tests and tools. `limn-egui` draws them from an atlas of its own,
rasterized as egui rasterizes its own text, so both look alike.

## Limits

- Text in a component is not laid out with it yet.
- No pointer position reaches a drawing, so selecting with the pointer inside
  one is the host's.
- The bundled faces have no Arabic, Hebrew or CJK glyphs: such text is laid
  out in its own direction and broken by its own rules, but drawn with each
  face's missing-glyph mark until a face that has it is given.
- Tabs take the width of a space.
