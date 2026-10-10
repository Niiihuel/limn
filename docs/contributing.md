# Contributing a component

The steps to add a component, and the conventions the existing ones follow.
`crates/lienzo/src/progress.rs` is the shortest complete example: read it
alongside this page.

## 1. Decide whether it is a component

A component earns its place when it packages a decision that should be made
once: a layout that is easy to get wrong, a set of states, a keyboard
behavior. If it is only two elements in a row, leave it to the caller.

Before writing anything new, check [Components](components.md). Many requests
are an option on something that exists.

## 2. Write it

Create `crates/lienzo/src/<name>.rs`.

```rust
//! One line saying what this is for.

use crate::{Component, ComponentPalette, Element};

/// What the host owns, and what the component reports.
pub struct Progress {
    fraction: f32,
    palette: ComponentPalette,
    width: Option<f32>,
}

impl Progress {
    pub fn new(fraction: f32, palette: ComponentPalette) -> Self { /* ... */ }

    pub fn width(mut self, width: f32) -> Self { /* ... */ }
}

impl Component for Progress {
    fn render(self) -> Element { /* ... */ }
}
```

Then declare it in `crates/lienzo/src/lib.rs`: a `mod` line and a `pub use`.

### Conventions

| Rule | Reason |
| --- | --- |
| `new` takes what is required: an ID if interactive, the content, the palette last. | Every component reads the same way. |
| Everything optional is a builder method returning `Self`. | Callers set only what they need. |
| The component is controlled. It takes its value and reports actions; it stores nothing between frames. | The window stays a function of the host's state. |
| Validate numbers in builders: ignore values that are not finite, clamp ranges. | A bad value from a caller must not reach a renderer. |
| Derive every ID the component creates from the caller's ID, such as `format!("{id}:body")`. | IDs must be unique across the window. |
| Take colors from the palette, radii and durations from `crate::tokens()`. Never write a color, a radius or a duration as a literal. | Themes and tokens must reach every component. |
| A disabled state renders passive elements, not a button that ignores clicks. | It must be unreachable by keyboard as well as pointer. |

Pills and circles are the exception to the radius rule: a radius of half the
height is a shape, not a token.

### Choosing tokens

| For | Use |
| --- | --- |
| A control | `tokens().radius.control` |
| A floating surface | `tokens().radius.surface` |
| A group set into the page | `tokens().radius.card` |
| A large container | `tokens().radius.panel` |
| A hover or press fill | `tokens().motion.hover_ms` |
| Something arriving or leaving | `tokens().motion.enter_ms`, `exit_ms` |
| Something moving in place | `tokens().motion.move_ms` |

See [Animation](animation.md#adding-animation-to-a-component) before adding
movement.

## 3. Test it

Add a `#[cfg(test)]` module in the same file. Cover at least:

- the geometry or paint that defines the component;
- every action it reports, by pointer;
- bad input to each numeric builder;
- a narrow width, if it contains text.

See [Testing](testing.md).

## 4. Show it in the gallery

Add it to an existing page in `crates/lienzo-gallery/src/pages/`, or add a
page. Show each state, not one happy example: disabled, loading, empty, long
text. See [Gallery and hot reload](gallery.md#adding-a-page).

Then look at it, in both themes and with square corners. Tests do not catch a
label that sits two pixels low.

## 5. Document it

Add a row to the right table in [Components](components.md), and a doc comment
on the struct and on every public method.

If the component needs more than a row, give it a section that follows this
template, in this order, omitting what does not apply:

```markdown
## ComponentName

One sentence: what it is for and when to use it.

### Example
The smallest useful snippet.

### Options
| Builder | Effect |

### Actions
| Action | When |

### States
Disabled, loading, empty, selected: how each looks and behaves.

### Keyboard
| Key | Effect |

### Motion
What animates, and with which duration token.

### Notes
Constraints and anything the host must do.
```

## Writing style for these pages

- English, present tense, second person for instructions.
- One idea per sentence. Say what something does before how.
- Tables for anything with more than two parallel items.
- Every code sample must compile against the current API. Prefer pointing to
  a file that is built by Cargo.
- State limits plainly, in a section of their own.

## Checklist

- [ ] Module, `mod` and `pub use` added.
- [ ] No literal colors, radii or durations.
- [ ] Tests pass: `cargo test -p lienzo -p lienzo-egui -p lienzo-gallery`.
- [ ] Shown in the gallery, checked in both themes and with square corners.
- [ ] Row in `docs/components.md`; doc comments on the public API.
