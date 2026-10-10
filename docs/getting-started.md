# Getting started

This tutorial builds a window with a counter and two buttons. By the end you
will have seen the whole loop a lienzo application runs: describe, lay out,
handle input, paint.

The finished program is
[`crates/lienzo-egui/examples/counter.rs`](../crates/lienzo-egui/examples/counter.rs).
Run it at any point to compare:

```sh
cargo run -p lienzo-egui --example counter
```

## Before you begin

You need Rust 1.95 or later, a C compiler, and the windowing libraries of your
platform. On NixOS, `nix-shell` at the repository root provides all of them.

## 1. Add the dependencies

lienzo is used from a checkout. Point your `Cargo.toml` at it:

```toml
[dependencies]
lienzo = { path = "../lienzo/crates/lienzo" }
lienzo-egui = { path = "../lienzo/crates/lienzo-egui" }
eframe = { version = "0.36.2", default-features = false, features = ["wgpu", "default_fonts", "x11", "wayland"] }
```

`lienzo` is the library. `lienzo-egui` paints it and reads input through egui;
`eframe` opens the window.

## 2. Hold the state

Everything the window shows is derived from one value that your application
owns:

```rust
#[derive(Default)]
struct State {
    count: i32,
}
```

## 3. Describe the window

Write a function from the state to a tree of elements. It runs every frame, so
it holds no state of its own.

```rust
use lienzo::{Button, ButtonVariant, Component, Element, FontFamily, Size, Theme};

fn view(state: &State, window: Size) -> Element {
    let theme = Theme::DARK;
    let palette = theme.palette();
    Element::panel(
        Element::column(vec![
            Element::text(format!("Count: {}", state.count))
                .font_family(FontFamily::Sans)
                .font_size(24.0)
                .foreground(palette.text),
            Element::row(vec![
                Button::new("decrement", "Decrease", palette)
                    .variant(ButtonVariant::Outline)
                    .render(),
                Button::new("increment", "Increase", palette).render(),
                Element::spacer().flex(1.0),
            ])
            .gap(8.0),
        ])
        .gap(16.0)
        .padding(32.0),
    )
    .background(theme.background)
    .size(Some(window.width), Some(window.height))
}
```

Three things to notice:

- `Element::column` and `Element::row` arrange children; `gap` and `padding`
  space them. See [Layout](layout.md).
- `Button::new("increment", ...)` gives the button an **ID**. The ID is how
  the library remembers the button between frames and how you recognize its
  click. See [Concepts](concepts.md#ids).
- A component is configured with builder methods and turned into an element
  with `.render()`.

## 4. React to actions

Input comes back as `UiAction` values. A click carries the ID of the control:

```rust
use lienzo::UiAction;

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "increment" => state.count += 1,
            "decrement" => state.count -= 1,
            _ => {}
        }
    }
}
```

## 5. Run the loop

Keep one `UiTree` for the life of the window. Each frame does four things:

```rust
fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let ctx = ui.ctx().clone();
    if !self.fonts_attached {
        lienzo_egui::attach_fonts(&mut self.tree, &ctx);
        self.fonts_attached = true;
    }
    lienzo_egui::normalize_keyboard_input(&ctx);
    let area = ui.max_rect();
    let window = Size::new(area.width(), area.height());

    // 1. Describe, 2. lay out.
    self.tree.set_root(view(&self.state, window));
    self.tree.layout(window);
    // 3. Deliver input and apply the actions it produces.
    for event in lienzo_egui::events(&ctx, area.min) {
        for action in self.tree.handle_event(event) {
            update(&mut self.state, &action);
        }
    }
    // 4. Describe the new state and paint it.
    self.tree.set_root(view(&self.state, window));
    let frame = self.tree.layout(window);
    lienzo_egui::paint_at(ui, &frame, area.min);

    // Keep painting while a transition is running, then go idle.
    if let Some(after) = self.tree.animation_repaint_after() {
        ctx.request_repaint_after(after);
    }
}
```

The tree is laid out before input is delivered because a click is resolved
against the positions from the last layout.

## 6. Open the window

Install the UI font once, when the window is created:

```rust
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "counter",
        options,
        Box::new(|creation| {
            lienzo_egui::install_fonts(&creation.egui_ctx);
            Ok(Box::new(App {
                tree: UiTree::new(Element::spacer()),
                state: State::default(),
                fonts_attached: false,
            }))
        }),
    )
}
```

Run it. The buttons change the number, darken under the pointer, and can be
reached with Tab and pressed with Enter or Space. You wrote none of that
behavior: the tree keeps hover, press and focus for each ID.

## Where to go next

- [Concepts](concepts.md) explains the model you just used.
- [Components](components.md) lists what you can put in the tree.
- [Theming](theming.md) changes how it looks.
- [Gallery and hot reload](gallery.md) shows every component running.
