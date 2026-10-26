# RollingNumber

A number whose digits roll to their new values, such as the lines added and
removed in a working tree as they change. Every digit is a column of 0 to 9
seen through a window one line tall; other characters, such as a sign or a
thousands separator, stand still beside them.

## Import

```rust
use limn::{Component, RollingNumber};
```

## Usage

A rolling number needs an ID and its text. Reuse the ID while rebuilding the
tree, so each digit rolls from where it was instead of starting again.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    RollingNumber::new("lines-added", format!("+{}", state.added), palette)
        .color(green)
        .render()
}
```

The text is the host's to format: limn rolls whatever digits it holds. A
digit is known by its place from the right, so "99" becoming "100" rolls the
two nines down to zero and the new one simply appears. It is passive and
reports nothing; the platform is told the whole text as its name.

## Text

```rust
RollingNumber::new("count", "1,204", palette)
    .font_size(14.0)
    .font_family(FontFamily::Monospace)
    .render()
```

13 px in the sans family in the palette's text color unless set. The window
is a quarter taller than the text.

## Motion

A digit that changes rolls up towards a greater one and down towards a
lesser one, over three `move_ms`, long enough to see the digits pass. A
change midway turns it from where it is. With reduced motion the new value
shows at once.

`Element::animate_slide(id, y)` is the movement alone: it places an element
`y` below its slot, sprung under `id`, for a parent that clips.
