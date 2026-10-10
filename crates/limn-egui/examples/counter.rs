//! The smallest complete limn application: one window, one counter.
//! Run it with `cargo run -p limn-egui --example counter`.
//! `docs/getting-started.md` walks through this file.

use eframe::egui;
use limn::{
    Button, ButtonVariant, Component, Element, FontFamily, Size, Theme, UiAction, UiTree,
};

/// Everything the window shows is derived from this.
#[derive(Default)]
struct State {
    count: i32,
}

/// Describe the window for the current state. Called every frame.
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

/// Apply one action to the state.
fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "increment" => state.count += 1,
            "decrement" => state.count -= 1,
            _ => {}
        }
    }
}

struct App {
    tree: UiTree,
    state: State,
    fonts_attached: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.fonts_attached {
            // Measure text with the fonts egui paints with.
            limn_egui::attach_fonts(&mut self.tree, &ctx);
            self.fonts_attached = true;
        }
        limn_egui::normalize_keyboard_input(&ctx);
        let area = ui.max_rect();
        let window = Size::new(area.width(), area.height());

        // 1. Describe, 2. lay out.
        self.tree.set_root(view(&self.state, window));
        self.tree.layout(window);
        // 3. Deliver input and apply the actions it produces.
        for event in limn_egui::events(&ctx, area.min) {
            for action in self.tree.handle_event(event) {
                update(&mut self.state, &action);
            }
        }
        // 4. Describe the new state and paint it.
        self.tree.set_root(view(&self.state, window));
        let frame = self.tree.layout(window);
        limn_egui::paint_at(ui, &frame, area.min);

        // Keep painting while a transition is running, then go idle.
        if let Some(after) = self.tree.animation_repaint_after() {
            ctx.request_repaint_after(after);
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_inner_size([420.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "counter",
        options,
        Box::new(|creation| {
            limn_egui::install_fonts(&creation.egui_ctx);
            Ok(Box::new(App {
                tree: UiTree::new(Element::spacer()),
                state: State::default(),
                fonts_attached: false,
            }))
        }),
    )
}
