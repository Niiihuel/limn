# Scribble

A working indicator that looks like a pen writing. It suits a line of text
that says what is going on, where a [Spinner](spinner.md) would be too
mechanical. For work whose completed share is known, use
[Progress](progress.md).

## Import

```rust
use limn::{Component, Scribble};
```

## Usage

A scribble needs an ID and a color. Reuse the ID while rebuilding the tree,
so the stroke does not restart.

```rust
fn view(palette: ComponentPalette) -> Element {
    Scribble::new("save-progress", palette.text).render()
}
```

It is passive: it takes no pointer or focus and reports nothing.

## Size and stroke

```rust
Scribble::new("save-progress", palette.text)
    .size(56.0, 24.0)
    .stroke(3.0)
    .render()
```

The default is 28 by 12 logical pixels with a 2 px stroke. The stroke is
drawn with round ends and joins.

## Motion

Each cycle takes 1.6 seconds:

- The pen writes the flourish in 1 second, starting slowly, speeding up and
  slowing again. Its tip is a dot a little wider than the line.
- The finished flourish holds for 0.2 seconds.
- It fades for 0.4 seconds, then the pen starts again.

The motion keeps the tree animating while the scribble is in it. With reduced
motion it draws the whole flourish, still and fully opaque. See
[Animation](../animation.md#keeping-the-window-painting) for how the host
keeps the window painting.

## Accessibility

The platform is told of a progress indicator without a value, known by the
scribble's ID, and named "Working". Name it on what it means with
`accessibility_label`:

```rust
Scribble::new("save-progress", palette.text)
    .render()
    .accessibility_label("Saving")
```
