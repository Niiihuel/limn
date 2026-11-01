//! A controlled sheet with fixed actions, scrolling and handle dismissal.
//! Run `cargo run -p limn-egui --example bottom_sheet`.
//! Optional arguments: phone width (default 390), then a PNG capture path.

use eframe::egui;
use limn::{BottomSheet, BottomSheetDrag, Component, Element, Size, Theme, UiAction, UiTree};

struct App {
    tree: UiTree,
    open: bool,
    drag: BottomSheetDrag,
    attached: bool,
    capture: Option<String>,
    frames: u32,
    shot: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<egui::ColorImage>>>>,
}

impl App {
    fn view(&self, size: Size) -> Element {
        let palette = Theme::DARK.palette();
        let base = Element::panel(
            Element::column(vec![
                Element::text("Library").font_size(24.0),
                Element::button("open", "Song actions").size(None, Some(44.0)),
            ])
            .gap(24.0)
            .padding(24.0),
        )
        .background(Theme::DARK.background)
        .size(Some(size.width), Some(size.height));
        BottomSheet::new(
            base,
            self.open,
            "Song actions",
            "Choose what to do with this song.",
            "close",
            palette,
        )
        .max_height(500.0)
        .safe_area_bottom(34.0)
        .body(
            Element::column(
                [
                    "Play next",
                    "Add to queue",
                    "Save to library",
                    "Add to playlist",
                    "View album",
                    "View artist",
                    "Share song",
                    "Song information",
                ]
                .into_iter()
                .enumerate()
                .map(|(index, label)| {
                    Element::button(format!("action:{index}"), label).size(None, Some(44.0))
                })
                .collect(),
            )
            .gap(8.0),
        )
        .footer(Element::button("close", "Done").size(None, Some(44.0)))
        .return_focus("open")
        .render()
    }

    fn update(&mut self, action: &UiAction) {
        if self.drag.handle_action(action) {
            self.open = false;
        }
        if matches!(action, UiAction::Clicked { id } if id == "open") {
            self.drag.reset();
            self.open = true;
        }
    }

    fn capture(&mut self, ctx: &egui::Context) {
        let Some(path) = &self.capture else {
            return;
        };
        if self.frames == 20 {
            let shot = self.shot.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(
                move |image| *shot.lock().expect("screenshot slot") = Some(image),
            )));
        }
        if let Some(image) = self.shot.lock().expect("screenshot slot").take() {
            image::save_buffer(
                path,
                image.as_raw(),
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .expect("save sheet capture");
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.attached {
            limn_egui::attach_fonts(&mut self.tree, &ctx);
            self.attached = true;
        }
        limn_egui::normalize_keyboard_input(&ctx);
        let area = ui.max_rect();
        let size = Size::new(area.width(), area.height());
        self.tree.set_root(self.view(size));
        self.tree.layout(size);
        for event in limn_egui::events(&ctx, area.min) {
            for action in self.tree.handle_event(event) {
                self.update(&action);
            }
        }
        for action in limn_egui::apply_access_requests(&ctx, &mut self.tree) {
            self.update(&action);
        }
        self.tree.set_root(self.view(size));
        let frame = self.tree.layout(size);
        limn_egui::paint_at(ui, &frame, area.min);
        if let Some(after) = self.tree.animation_repaint_after() {
            ctx.request_repaint_after(after);
        }
        self.frames += 1;
        self.capture(&ctx);
    }
}

fn main() -> eframe::Result {
    let mut arguments = std::env::args().skip(1);
    let width = arguments
        .next()
        .and_then(|width| width.parse::<f32>().ok())
        .filter(|width| width.is_finite() && *width > 0.0)
        .unwrap_or(390.0);
    let capture = arguments.next();
    let mut viewport = egui::ViewportBuilder::default().with_inner_size([width, 720.0]);
    if capture.is_some() {
        // A tiling compositor otherwise overrides the requested phone width.
        viewport = viewport
            .with_min_inner_size([width, 720.0])
            .with_max_inner_size([width, 720.0]);
    }
    eframe::run_native(
        "Bottom sheet",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport,
            ..Default::default()
        },
        Box::new(move |creation| {
            limn_egui::install_fonts(&creation.egui_ctx);
            let mut tree = UiTree::new(Element::spacer());
            tree.set_reduced_motion(capture.is_some());
            Ok(Box::new(App {
                tree,
                open: true,
                drag: BottomSheetDrag::new("close"),
                attached: false,
                capture,
                frames: 0,
                shot: Default::default(),
            }))
        }),
    )
}
