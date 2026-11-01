# Sketch

Marks drawn over a picture or any area, for annotating a screenshot. The host
keeps the marks; the sketch draws them, and reports each new one.

## Import

```rust
use limn::{rasterize_sketch, Component, Sketch, SketchMark, SketchTool};
```

## Usage

The host owns the marks. It passes them on every rebuild, and appends each
one it receives. Positions are shares of the sketch's size, from 0 to 1, so
the marks keep their place when the sketch is resized.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Sketch::new("annotate", &state.marks, state.tool, state.ink, 4.0)
        .size(640.0, 400.0)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Sketched { id, mark } = action {
        if id == "annotate" {
            state.marks.push(mark.clone());
        }
    }
}
```

## Tools

`SketchTool` chooses the mark a press and drag make. Each one is reported as a
`SketchMark` of the same name.

| Tool | Mark | Drawn as |
| --- | --- | --- |
| `Pen` | `Stroke { points, color, width }`: the points in order | A polyline with a dot at each point. One point, from a press without a drag, is a dot. |
| `Line` | `Line { from, to, color, width }` | A stroked line. |
| `Arrow` | `Arrow { from, to, color, width }` | A shaft with a filled head at `to`, as long as four times the width or the line, whichever is shorter. |
| `Rectangle` | `Rectangle { from, to, color, width }` | An outlined rectangle with corners at `from` and `to`. |
| `Ellipse` | `Ellipse { from, to, color, width }` | An outlined ellipse inside the box with corners at `from` and `to`. |

`color` and `width` come from the sketch: `Sketch::new(id, marks, tool, color, width)`.
Widths are in logical pixels at the size the sketch is shown at. A width that
is not finite is 2, and a negative one is 0.

## Size

`.size(width, height)` fixes the sketch in logical pixels. A side that is not
finite is ignored and a negative one is zero. Without it the sketch takes the
size its parent gives it.

## Saving the marks onto the picture

`rasterize_sketch` paints the marks into the pixels of the full-size image, so
the host can save an annotated copy. The buffer is RGBA8, row by row from the
top left; `scale` is the buffer's width divided by the width the sketch is
shown at, which scales the widths.

```rust
// `rgba` holds the picture's pixels, `image_width` by `image_height`.
rasterize_sketch(&state.marks, image_width, image_height, &mut rgba, image_width as f32 / 640.0);
```

Pixels are not anti-aliased: a pixel is colored when its center is inside a
mark.

## Drawing while the pointer is down

The mark in progress is drawn by the tree, from the press to the pointer. The
host hears nothing until the release, when it receives one `Sketched` action.
A pen point is added when the pointer moves, unless it is where the last one
was. The pointer may leave the sketch: points are clamped to its edges.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves focus to the sketch. |
| Enter, Space | Reports `Clicked` with the sketch's ID. The sketch has no keyboard drawing; ignore the action. |

## Actions

| Action | When |
| --- | --- |
| `Sketched { id, mark }` | The pointer was released after a press over the sketch, or left the window while it was down. The mark is in the sketch's shares. |

## Limits

- Joins and caps are approximated in the sketch: a dot at each vertex of a
  pen stroke and at the corners of a rectangle stand in for round joins, and
  an ellipse is 64 segments. The rasterizer draws real round joins.
- The rasterizer does not anti-alias edges.
- A sketch has no undo. The host removes the last mark when it wants one.
