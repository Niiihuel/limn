# TextLayout

Text that limn shapes, breaks into lines and places itself. Where the caret
goes, what a selection covers and the glyphs a backend draws all come from
one layout, so they always agree: no backend lays the text out again.

In a tree told to with `UiTree::set_text_layout(true)`, which
`limn_egui::attach_fonts` does, every `Element::text` and
`Element::formatted_text` is laid out in a `TextLayout`, and selected,
underlined and pressed from it: so are `Markdown` and every component made of
them. Text fields, text areas and the code editor still measure their text
through the backend; they move to it one at a time.
An application also draws text with it in an [`Element::drawing`](drawing.md).

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
font, placed from `origin`. A monochrome glyph is drawn in `color`. A layout
and its clones give the same glyphs every time, so a backend can keep what it
drew for them while they stay in place.

## Style

`TextStyle::new(size, family)` sets the size in logical pixels and the
`FontFamily`, which carries the weight and the slant. `.line_height(px)` sets
every line that far apart; without it the font's own ascent, descent and line
gap decide. A size that is not finite is 14 and one below zero is zero; a
line height that is not finite or below zero is ignored.

`.leading(Leading)` says where the glyphs stand in a line taller than they
are: `Leading::Even`, the default, sets half the spare room above them, as
CSS does; `Leading::Below` sets them at the top of the line, the room under
them, as limn's own text is set. A tree lays `Element::text` out with
`Below` and the line height of its metrics, so text sits where it did when
the backend laid it out.

## Wrapping

```rust
let paragraph = TextLayout::new(text, style).wrapped(360.0);
```

`wrapped(width)` breaks the same text into lines no wider than `width`, where
the language allows a break, without shaping it again: a layout made once can
follow its container's width. A word longer than `width` is broken between
graphemes, so no line runs past it. A width that is not finite does not wrap.

`centered(width)` breaks it as `wrapped` does and centers each line across
`width`.

`lines()` gives each line's box, its baseline and the bytes it holds. A line's
box leaves out the spaces that hang past its end where it wraps.

## Formatted text

```rust
let spans = [
    TextSpan::new("Read ", ink, 14.0, FontFamily::Sans),
    TextSpan::new("the guide", link, 14.0, FontFamily::Sans).underline(),
    TextSpan::new(" first.", ink, 14.0, FontFamily::Sans),
];
let paragraph = TextStyle::new(14.0, FontFamily::Sans).line_height(18.0);
let layout = TextLayout::formatted(&spans, paragraph).wrapped(320.0);
```

`TextLayout::formatted(spans, paragraph)` sets each `TextSpan` in its own
size, family and ink, all on one baseline, with the lines, background, icon
or picture it asks for, as `Element::formatted_text` draws them. The
paragraph's line height is the least height of every line, and a line whose
glyphs need more takes more; its family draws nothing.

A span with a background stays on one line unless it is wider than any. An
icon goes to the next line with the first word after it. A picture stands in
for its span's text, which a selection copies whole or not at all.

`max_lines(n)` keeps at most `n` lines. When more would follow, the last ends
in «…» in the style of the text before it; no offset reaches the «…».

`span_bounds(index)` gives one box for each line a span is on, as tall as
the line. A span's `action` and `tooltip` belong to the tree, which puts
their targets and hints there.

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
Ubuntu for `Sans` in regular, medium and bold, Hack for `Monospace`, Noto Sans
Math and Noto Sans Symbols 2 behind both, and Noto Emoji for emoji. `.face(family,
font)` replaces a family's face; `.fallback(font)` adds a face tried, in
order, for characters the families lack; `.emoji(font)` sets the face tried
last, so it never takes a symbol a text face has. A family without a face of
its own takes `Sans`: `SansItalic` slanted and `SansBold` thickened. Behind
`Sans` come the fallbacks, then `Monospace`, then the emoji face; behind
`Monospace` come `Sans`, the fallbacks, then the emoji face. Only the faces given are used, never the system's, so text is laid
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

- Only `Element::text` and `Element::formatted_text` are laid out with it
  yet, and only in a tree told to.
- No pointer position reaches a drawing, so selecting with the pointer inside
  one is the host's.
- The bundled faces have no Arabic, Hebrew or CJK glyphs: such text is laid
  out in its own direction and broken by its own rules, but drawn with each
  face's missing-glyph mark until a face that has it is given.
- Tabs take the width of a space.
