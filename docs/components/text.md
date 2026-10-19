# Text

A line or a paragraph of passive text, sized by a role of the type scale and
colored from the palette.

## Import

```rust
use limn::{Component, Text, TextRole};
```

## Usage

Text needs its value and the palette. Without anything else it is
caption-sized sans text in the palette's text color, and it wraps.

```rust
fn view(palette: ComponentPalette) -> Element {
    Text::new("Saved a moment ago", palette).render()
}
```

## Roles

```rust
Text::new("Projects", palette).role(TextRole::Heading).render()
```

| Role | Size |
| --- | --- |
| `Display` | 18 |
| `Heading` | 16 |
| `Body` | 14 |
| `Label` | 13 |
| `Caption` | 12 |
| `Small` | 11 |
| `Micro` | 10 |
| `Code` | 14, monospaced |

## A size off the scale

```rust
Text::new("42", palette).mono().font_size(10.5).render()
```

`font_size(px)` is for a step the host's design has between two roles. The role
still chooses the family. Prefer a role.

## Ink and family

```rust
Text::new("Last run failed", palette).ink(error).render()
Text::new("3 files changed", palette).muted().render()
Text::new("cargo test -p limn", palette).mono().render()
Text::eyebrow("RECENT", palette).render()
```

`muted` paints in the palette's muted color, for secondary copy. `ink` takes
a color that carries meaning the host owns, such as an error or a warning.
`mono` uses the monospaced family whatever the role. `Text::eyebrow` is the
small monospaced muted label set above the group it names.

## Long text

```rust
Text::new(title, palette).truncate().tooltip("file-title").render()
Text::new(title, palette).fade(32.0).render()
Text::new(paragraph, palette).line_height(20.0).render()
```

`truncate` keeps the text to one line and ends it in an ellipsis when the
width runs out. `tooltip(id)` shows the whole value in a tooltip, wrapped;
the ID must be unique in the window, and it is ignored without `truncate`.
`fade(width)` keeps to one line and fades out over the last `width` pixels
instead; it takes precedence over `truncate`. `line_height` sets the leading
of a wrapped paragraph.

## Weights

`Text` has no weight. For the medium and bold faces, build the element and
choose the family:

```rust
Element::text("175.6K of 200K tokens")
    .font_family(FontFamily::SansBold)
    .font_size(15.0)
    .foreground(palette.text)
```

`FontFamily::SansMedium`, `FontFamily::SansSemibold` and `FontFamily::SansBold`
are faces the host provides; a host without one paints `Sans`. Semibold sits
between the other two, for a title that should stand out from medium text
without the weight of bold.

## Out of focus

```rust
Text::new(line, palette).font_size(20.0).render().soft_focus("lyric:3", 2.0)
```

`soft_focus(id, radius)` is a method of the rendered element. It paints the
text out of focus by `radius` logical pixels, from 0 (sharp) to 8.

## Motion

The radius of `soft_focus` is kept under its ID and springs to a new value;
under reduced motion it changes at once.
