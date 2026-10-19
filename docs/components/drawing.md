# Drawing

What the application draws itself, for a shape no component makes: a track's
waveform, a meter, a sketch. It draws in the frame's own commands, the ones
every component is drawn in, so any backend paints it.

## Import

```rust
use limn::{Color, DrawCommand, Element, Rect};
```

## Usage

`Element::drawing` takes a function of the size the element is laid out at
and returns the commands to draw at it, placed from the element's top-left
corner.

```rust
fn waveform(levels: Vec<f32>, played: f32, ink: Color, rest: Color) -> Element {
    Element::drawing(move |size| {
        let pitch = size.width / levels.len().max(1) as f32;
        levels
            .iter()
            .enumerate()
            .map(|(index, level)| {
                let x = index as f32 * pitch;
                let height = size.height * level;
                DrawCommand::FillRect {
                    rect: Rect::new(x, (size.height - height) / 2.0, pitch - 2.0, height),
                    color: if x < size.width * played { ink } else { rest },
                    radius: 1.5,
                }
            })
            .collect()
    })
    .size(None, Some(64.0))
}
```

The function runs each time the tree lays the window out, so the drawing
follows its size. It owns what it draws from: move the values in, as `levels`
is above.

## Size

Give it a size. Alone it takes no height, and only the width a column
stretches it to. Padding is kept around the drawing: the size the function is
given is the box inside it.

## Clipping

The tree moves the commands into place and clips them to the element's box.
A drawing may push clips of its own: one it leaves open is closed at its end,
and a `PopClip` with no `PushClip` before it is left out, so no drawing can
change the clipping of what comes after it.

## Pressing it

A drawing is passive. To press it, wrap it as any element is wrapped:
`Element::clickable(id, drawing)` reports `Clicked` with `id`, and
`Tooltip::new(drawing, id, title, palette)` gives it a hint.

## Accessibility

A drawing tells the platform nothing on its own. Name what it shows with
`.accessible(Access::new(Role::Image).id("waveform").name("Waveform"))`: a passive element
has no ID of its own, so the node takes the one given here.
