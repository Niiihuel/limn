# Testing

How to test a component or a whole screen. Nothing here needs a window or a
GPU: a test lays out a tree and inspects the frame.

## The basic test

```rust
use lienzo::{Button, Component, ComponentPalette, Element, Size, UiAction, UiEvent, UiTree, Point};

#[test]
fn clicking_the_button_reports_its_id() {
    let palette = ComponentPalette::neutral_dark();
    let mut tree = UiTree::new(Element::row(vec![
        Button::new("save", "Save", palette).render(),
        Element::spacer().flex(1.0),
    ]));
    let frame = tree.layout(Size::new(400.0, 40.0));

    let button = frame.regions.iter().find(|region| region.id == "save").unwrap().rect;
    let center = Point::new(button.x + button.width / 2.0, button.y + button.height / 2.0);

    tree.handle_event(UiEvent::PointerDown(center));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(center)),
        vec![UiAction::Clicked { id: "save".into() }]
    );
}
```

The root element is stretched to the window. Put a component in a row or
column beside a flexible spacer to test it at its own size.

## What to inspect

| To check | Look at |
| --- | --- |
| Where an interactive control is, and that it exists | `frame.regions`: each has an `id`, a `rect` and a `kind`. |
| That something is passive | No region with its ID. |
| What is painted, and where | `frame.commands`: match on `DrawCommand::Text`, `FillRect`, `Icon`, and so on. |
| What a click would hit | `frame.hit_test(point)`. |
| What an event does | The `Vec<UiAction>` returned by `tree.handle_event`. |
| A builder's effect on an element | `element.style` before it goes in a tree. |

## Text widths

Without a backend the tree estimates text width from default metrics. For a
test that depends on exact widths, register a simple measurer:

```rust
tree.set_text_measurer(|text, _size, _family| text.chars().count() as f32 * 8.0);
```

## Animation

A frame laid out the instant something appears is the first frame of its
entrance: transparent and slightly offset. A test of geometry or color would
measure that instead of the settled result. Choose one:

**Settle everything.** For tests that are not about animation:

```rust
tree.set_reduced_motion(true);
```

**Drive the clock.** For tests that are about animation, lay out at chosen
instants with `layout_at`:

```rust
use std::time::{Duration, Instant};

let start = Instant::now();
let arriving = tree.layout_at(size, start);
let settled = tree.layout_at(size, start + Duration::from_millis(200));
```

`tree.is_animating()` and `animation_repaint_after()` use the real clock, not
the instant passed to `layout_at`, so do not assert on them in such a test.

## Tokens

`set_tokens` changes the whole process, and the unit tests of a crate share
one process and run in parallel. A unit test that changes tokens would reshape
every other test while it ran.

Put token-changing tests in an integration test, which Cargo builds as its own
executable, and keep them in a single test function so they run in order.
`crates/lienzo/tests/tokens.rs` is the example.

## Covering more than one size

Layout bugs tend to appear at the extremes. Loop over a few widths:

```rust
for width in [180.0, 360.0, 900.0] {
    let frame = tree.layout(Size::new(width, 240.0));
    // assert nothing escapes `width`
}
```

## Running the tests

```sh
cargo test -p lienzo -p lienzo-egui -p lienzo-gallery
```

The gallery's tests lay out every page in both themes at three widths and fail
if two controls on a page share an ID.

## Seeing a result

A test proves geometry, not appearance. To look at a component, capture it
from the gallery. See [Gallery and hot reload](gallery.md#capturing-a-page).

## See also

- [Concepts](concepts.md) for regions, actions and IDs.
- [Animation](animation.md) for what a first frame contains.
