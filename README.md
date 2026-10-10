# limn

A native UI library for Rust. An application describes its window as a tree
of elements; limn lays it out, keeps the state of every control by its ID,
and produces drawing commands for a backend to paint.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::column(vec![
        Element::text(format!("Count: {}", state.count)).foreground(palette.text),
        Button::new("increment", "Increase", palette).render(),
    ])
    .gap(16.0)
}
```

- **Declarative.** The window is a function of your state, rebuilt every frame.
- **Backend-neutral.** The core has no window or graphics dependency. An egui
  backend is included.
- **Animated by default.** Menus, dialogs, tooltips and controls move without
  application code, and one switch turns all of it off.
- **Themeable.** Three colors define a palette; tokens set corner radii and
  pace for the whole application.
- **Testable.** A test lays out a tree and inspects the result. No window.

## Quick start

```sh
nix-shell                                               # compiler and window libraries
cargo run -p limn-egui --example counter              # the smallest application
cargo run -p limn-gallery                             # every component
dx serve --hot-patch -p limn-gallery --features hot   # the gallery with hot reload
cargo test -p limn -p limn-egui -p limn-gallery   # the tests
```

## Documentation

Everything is in [`docs/`](docs/README.md).

| If you want to | Read |
| --- | --- |
| Build your first window | [Getting started](docs/getting-started.md) |
| Understand the model | [Concepts](docs/concepts.md) |
| Arrange elements | [Layout](docs/layout.md) |
| See what components exist | [Components](docs/components.md) |
| Change colors, corners and pace | [Theming](docs/theming.md), [Tokens](docs/tokens.md) |
| Know what animates and how to stop it | [Animation](docs/animation.md) |
| Test without a window | [Testing](docs/testing.md) |
| Browse and edit components live | [Gallery and hot reload](docs/gallery.md) |
| Add a component | [Contributing a component](docs/contributing.md) |
| Write a renderer | [Backend contract](docs/backend.md) |

## Crates

| Crate | Purpose |
| --- | --- |
| `limn` | The library: elements, layout, events, animation and components. No backend dependency. |
| `limn-egui` | Backend: paints a frame with egui and wgpu and translates its input. |
| `limn-editor` | The text document behind the code editor: rope, selection, search. |
| `limn-language` | Syntax highlighting with tree-sitter and an LSP client. Optional, behind the `syntax` feature. |
| `limn-dnd` | Dragging files to and from other applications. |
| `limn-gallery` | The component gallery. Not published. |

## Status

Version 0.1. The API changes without notice. The library is used from a
checkout, not from crates.io.
