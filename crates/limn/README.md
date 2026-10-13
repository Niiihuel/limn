# limn

A retained-state UI library for Rust. An application describes its interface
as an `Element` tree; `UiTree` lays it out, keeps control state by stable IDs,
and produces backend-neutral drawing commands. `limn-egui` provides an
optional renderer and input adapter for egui and wgpu.

[Documentation and live component gallery](https://limn-omega.vercel.app/).

```rust
use limn::{Button, Component, Element, Theme};

fn view(count: u32) -> Element {
    let palette = Theme::DARK.palette();
    Element::column(vec![
        Element::text(format!("Count: {count}")).foreground(palette.text),
        Button::new("increment", "Increase", palette).render(),
    ])
    .gap(16.0)
}
```

Keep one `UiTree` per window. Each frame, describe the interface, lay it out,
deliver input, apply the resulting `UiAction` values to your application
state, and paint the resulting `Frame`.

## Getting started

The [getting started guide](https://limn-omega.vercel.app/docs/getting-started/)
walks through a complete counter application. Rust 1.95 or later is required.
From a repository checkout, run `cargo run -p limn-egui --example counter`.

## Language

Built-in labels default to English. To keep an existing application in
Spanish, call this once during startup, before building the first UI tree:

```rust
limn::set_labels(limn::Labels::SPANISH);
```

See [Language](https://limn-omega.vercel.app/docs/getting-started/#language)
for the process-wide setting and component-specific overrides.

## Features

The default `syntax` feature enables tree-sitter highlighting through
`limn-language`. With `default-features = false`, code views use the line
lexer instead.

## Used by

**openADE**, a native Rust development environment for working with coding agents.

## License

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE),
at your option.
