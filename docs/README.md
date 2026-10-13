# limn documentation

limn is a native UI library for Rust. An application describes its window as
a tree of elements; the library lays it out, keeps the state of every control,
and produces drawing commands for a backend to paint.

The pages are organized by what you are trying to do.

## Learn

Start here if you have never used the library.

| Page | What you get from it |
| --- | --- |
| [Getting started](getting-started.md) | A window with a working counter, built step by step. |

## Understand

Background that explains why the library behaves as it does.

| Page | What it explains |
| --- | --- |
| [Concepts](concepts.md) | Elements, the tree, frames, IDs, and who owns state. |
| [Animation](animation.md) | What moves, for how long, and how it is switched off. |
| [Native glass navigation](liquid-glass.md) | The Ricelin-inspired bar, optical material, research and Wayland integration boundary. |

## Do a specific task

| Page | Task |
| --- | --- |
| [Layout](layout.md) | Arrange, size, center and wrap elements. |
| [Theming](theming.md) | Choose colors, corner radii and pace. |
| [Guided tours](tours.md) | Point at existing controls and explain them step by step. |
| [Testing](testing.md) | Test a component or a screen without a window. |
| [Gallery and hot reload](gallery.md) | Browse components, edit them live, capture screenshots. |
| [Contributing a component](contributing.md) | Add a component, its test, its demo and its documentation. |

## Look something up

| Page | Contents |
| --- | --- |
| [Tokens](tokens.md) | Every radius and duration, with defaults. |
| [Components](components.md) | Every component, by category, with its purpose. |
| [Backend contract](backend.md) | Draw commands, events and fonts, for writing a renderer. |
| API reference | `cargo doc -p limn --open` lists every type and method. |

## Conventions used in these pages

- Sizes are logical pixels. The backend applies the display's scale factor.
- Durations are milliseconds.
- "The host" is the application that owns a `UiTree`.
- Code samples assume `use limn::*;` unless they show their own imports.
