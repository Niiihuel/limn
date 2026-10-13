# Contributing a component

The steps to add a component, and the conventions the existing ones follow.
`crates/limn/src/progress.rs` is the shortest complete example: read it
alongside this page.

## 1. Decide whether it is a component

A component earns its place when it packages a decision that should be made
once: a layout that is easy to get wrong, a set of states, a keyboard
behavior. If it is only two elements in a row, leave it to the caller.

Before writing anything new, check [Components](components.md). Many requests
are an option on something that exists.

## 2. Write it

Create `crates/limn/src/<name>.rs`.

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

Then declare it in `crates/limn/src/lib.rs`: a `mod` line and a `pub use`.

### Conventions

| Rule | Reason |
| --- | --- |
| `new` takes what is required: an ID if interactive, the content, the palette last. | Every component reads the same way. |
| Everything optional is a builder method returning `Self`. | Callers set only what they need. |
| The component is controlled. It takes its value and reports actions; it stores nothing between frames. | The window stays a function of the host's state. |
| Validate numbers in builders: ignore values that are not finite, clamp ranges. A profile struct passes each number through `crate::page::px` where it is used. | A bad value from a caller must not reach a renderer. |
| Derive every ID the component creates from the caller's ID, such as `format!("{id}:body")`. | IDs must be unique across the window. |
| Take colors from the palette, radii and durations from `crate::tokens()`. Never write a color, a radius or a duration as a literal. | Themes and tokens must reach every component. |
| A disabled state renders passive elements, not a button that ignores clicks. | It must be unreachable by keyboard as well as pointer. |
| One name for one idea: `ink` for the color of the content, `font_size` for the size of text, `width` and `height` in logical pixels, `size(ControlSize)` for a control's size, `sizing(<Component>Sizing)` for a density profile, `accessibility_label` for a name. | A caller who knows one component can guess the next. |
| A name that changes keeps its old form, marked `#[deprecated]` with the new name in its note. | An application built on an earlier version still compiles, and is told what to change. |
| What can be disabled has both `disabled()` and `enabled(bool)`. | The first reads well for a fixed case, the second takes a state the host computes; a caller should not have to remember which one a component has. |

`crates/limn/tests/conventions.rs` holds the components to the rules that can
be checked across all of them: both spellings of disabling, and sizes that
are not numbers. A component that joins a family joins its row there.

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

Every component has a page of its own in the gallery. Write its sections in a
module of `crates/limn-gallery/src/pages/`, one `section` for each aspect:
variants, sizes, states. Show each state, not one happy example: disabled,
loading, empty, long text. Then list the component in `PAGES`, with the titles
of the sections that are its own. See
[Gallery and hot reload](gallery.md#adding-a-page).

Then look at it, in both themes and with square corners. Tests do not catch a
label that sits two pixels low.

## 5. Document it

Add a row to the right table in [Components](components.md), and a doc comment
on the struct and on every public method.

Then write its guide, `docs/components/<id>.md`, named after its page in the
gallery. The site shows the guide under the component running, so the two are
read together. Every guide follows this outline, in this order, omitting what
does not apply after `Usage`:

```markdown
# ComponentName

One or two sentences: what it is for and when to use it.

## Import
The `use` line a file needs.

## Usage
The smallest complete use: the view, and the action handled.

## One heading for each thing it can do
Variants, sizes, icons, steps: a snippet and what it changes.

## Keyboard
| Key | Effect |

## Motion
What animates, and with which duration token.

## Actions
| Action | When |
```

A test in the gallery checks that a guide belongs to a page, starts with the
component's name and has `Import` and `Usage`. Snippets use `palette` and
`state` as a reader's own view would have them; they are fragments, not
programs. [Button](components/button.md), [Slider](components/slider.md) and
[Switch](components/switch.md) are the models.

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
- [ ] Tests pass: `cargo test -p limn -p limn-egui -p limn-gallery`.
- [ ] Shown in the gallery, checked in both themes and with square corners.
- [ ] Row in `docs/components.md`; doc comments on the public API.
