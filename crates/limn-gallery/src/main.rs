//! Component gallery for limn.
//!
//! `cargo run -p limn-gallery` opens the gallery.
//! `dx serve --hot-patch -p limn-gallery --features hot` patches views
//! while it runs. `--page <id>` chooses the first page and
//! `--screenshot <file.png>` saves that page and exits; `--hover <id>` holds
//! the pointer over a control first, to capture its hover state or tooltip,
//! `--end` scrolls the page to its end and `--set <id=value>` starts as if
//! that control had been clicked, to capture an open menu or dialog.

mod app;
mod glass_content;
mod glass_demo;
mod glass_drag;
mod pages;

use eframe::egui;
use limn::{Element, Point, Size, UiAction, UiEvent, UiTree};
use std::path::PathBuf;
use web_time::{Duration, Instant};

/// Run through the hot-patch jump table when patching is enabled, so the
/// next frame after a patch calls the new code.
fn hot<T>(call: impl FnMut() -> T) -> T {
    #[cfg(feature = "hot")]
    return subsecond::call(call);
    #[cfg(not(feature = "hot"))]
    {
        let mut call = call;
        call()
    }
}

struct Host {
    glass: Option<glass_demo::GlassDemo>,
    tree: UiTree,
    state: app::State,
    started: bool,
    screenshot: Option<PathBuf>,
    hover: Option<String>,
    scroll_end: bool,
    opened: Instant,
    requested: bool,
}

/// Where the gallery keeps its state between runs while hot reloading.
fn session_file() -> PathBuf {
    std::env::temp_dir().join("limn-gallery-session")
}

impl Host {
    /// Under hot reload a change that cannot be patched restarts the gallery.
    /// Saving the state makes that restart land on the same page with the
    /// same values, like a patch would.
    fn remember(&self) {
        if cfg!(feature = "hot") && self.screenshot.is_none() {
            // Best effort: a lost session only costs a click.
            let _ = std::fs::write(session_file(), self.state.serialize());
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        limn_egui::attach_fonts(&mut self.tree, ctx);
        #[cfg(feature = "hot")]
        {
            let wake = ctx.clone();
            subsecond::register_handler(std::sync::Arc::new(move || wake.request_repaint()));
        }
    }

    /// Ask for the frame once fonts and icons have settled, save it, exit.
    fn capture(&mut self, ctx: &egui::Context, path: &PathBuf) {
        // Long enough for fonts and icons to settle and a tooltip to open.
        if !self.requested && self.opened.elapsed() > Duration::from_millis(900) {
            self.requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [width, height] = image.size;
            let saved = image::save_buffer(
                path,
                image.as_raw(),
                width as u32,
                height as u32,
                image::ColorType::Rgba8,
            );
            if let Err(error) = saved {
                eprintln!("could not save {}: {error}", path.display());
                std::process::exit(1);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl eframe::App for Host {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            self.start(&ctx);
        }
        limn_egui::normalize_keyboard_input(&ctx);
        self.tree
            .set_reduced_motion(self.screenshot.is_some() || self.state.flag("tokens-reduced"));
        let area = ui.max_rect();
        let window = Size::new(area.width(), area.height());

        // A page may answer the control under the pointer, as a host mirrors
        // it; a screenshot keeps the one it was given with `--set`.
        if self.screenshot.is_none() {
            let hovered = self.tree.hovered_id().unwrap_or_default().to_owned();
            if self.state.text("hovered") != hovered {
                self.state.set("hovered", &hovered);
            }
        }
        self.tree.set_root(hot(|| app::view(&self.state, window)));
        self.tree.layout(window);
        let mut changed = false;
        for event in limn_egui::events(&ctx, area.min) {
            for action in self.tree.handle_event(event) {
                if let UiAction::CopyRequested { text } = &action {
                    ctx.copy_text(text.clone());
                }
                // A demo asks for a control to be brought into view.
                if let UiAction::Clicked { id } = &action {
                    if let Some(target) = id.strip_prefix("reveal:") {
                        self.tree.reveal(target);
                    }
                }
                if !glass_drag::handle(&mut self.state, &action, self.tree.frame()) {
                    hot(|| app::update(&mut self.state, &action));
                }
                changed = true;
            }
        }
        if changed {
            self.remember();
        }
        if changed {
            self.tree.set_root(hot(|| app::view(&self.state, window)));
        }
        if self.scroll_end {
            self.tree.scroll_to_end("page-scroll");
        }
        if let Some(id) = &self.hover {
            let frame = self.tree.layout(window);
            if let Some(region) = frame.regions.iter().find(|region| &region.id == id) {
                let rect = region.rect;
                let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                self.tree.handle_event(UiEvent::PointerMove(center));
            }
        }
        let frame = self.tree.layout(window);
        if let Some(glass) = &mut self.glass {
            let animate = self.screenshot.is_none()
                && !self.state.flag("tokens-reduced")
                && self.state.choice("glass-animate", "on") != "off";
            let speed = self.state.text("glass-speed").parse().unwrap_or(0.5);
            glass.paint(
                ui,
                &frame,
                area.min,
                self.state.choice("glass-scene", "amber"),
                animate,
                speed,
            );
        } else {
            limn_egui::paint_at(ui, &frame, area.min);
        }

        if let Some(path) = self.screenshot.clone() {
            self.capture(&ctx, &path);
        } else if let Some(after) = self.tree.animation_repaint_after() {
            ctx.request_repaint_after(after);
        }
    }
}

/// In a page the gallery fills the canvas `limn`; the address's fragment
/// (`#glass`) chooses the page, so a site can link to one component, and
/// `?embed` leaves only that page, `?light` in the light theme.
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    let window = eframe::web_sys::window().expect("a window");
    let page = window.location().hash().unwrap_or_default();
    let page = page.trim_start_matches('#');
    let page = pages::PAGES
        .iter()
        .find(|known| known.id == page)
        .map_or(pages::PAGES[0].id, |known| known.id);
    let mut state = app::State::starting_at(page);
    let query = window.location().search().unwrap_or_default();
    for flag in ["light", "embed"] {
        if query.contains(flag) {
            state.set(flag, "on");
        }
    }
    let canvas = window
        .document()
        .and_then(|document| document.get_element_by_id("limn"))
        .and_then(|element| element.dyn_into::<eframe::web_sys::HtmlCanvasElement>().ok())
        .expect("a canvas with the id `limn`");
    wasm_bindgen_futures::spawn_local(async move {
        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(move |creation| {
                    limn_egui::install_fonts(&creation.egui_ctx);
                    Ok(Box::new(Host {
                        glass: creation
                            .wgpu_render_state
                            .as_ref()
                            .map(glass_demo::GlassDemo::new),
                        tree: UiTree::new(Element::spacer()),
                        state,
                        started: false,
                        screenshot: None,
                        hover: None,
                        scroll_end: false,
                        opened: Instant::now(),
                        requested: false,
                    }))
                }),
            )
            .await
            .expect("the gallery starts");
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    #[cfg(feature = "hot")]
    dioxus_devtools::connect_subsecond();

    let mut arguments = std::env::args().skip(1);
    let mut page = pages::PAGES[0].id.to_owned();
    // An explicit page starts clean instead of resuming the last session.
    let mut page_chosen = false;
    let mut light = false;
    let mut screenshot = None;
    let mut hover = None;
    let mut scroll_end = false;
    let mut presets = Vec::new();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--page" => {
                page = arguments.next().unwrap_or(page);
                page_chosen = true;
            }
            "--light" => light = true,
            "--screenshot" => screenshot = arguments.next().map(PathBuf::from),
            "--hover" => hover = arguments.next(),
            "--end" => scroll_end = true,
            "--set" => presets.extend(arguments.next()),
            other => {
                eprintln!("unknown argument {other}; expected --page <id>, --light, --end, --set <id=value>, --hover <id> or --screenshot <file.png>");
                std::process::exit(2);
            }
        }
    }
    let restored = (cfg!(feature = "hot") && !page_chosen && screenshot.is_none())
        .then(|| std::fs::read_to_string(session_file()).ok())
        .flatten()
        .map(|text| app::State::deserialize(&text));
    let mut state = restored.unwrap_or_else(|| app::State::starting_at(&page));
    if light {
        state.set("light", "on");
    }
    for preset in presets {
        // The same path a click on that ID takes.
        app::update(&mut state, &UiAction::Clicked { id: preset });
    }

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 820.0])
            .with_min_inner_size([520.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "limn-gallery",
        options,
        Box::new(move |creation| {
            limn_egui::install_fonts(&creation.egui_ctx);
            Ok(Box::new(Host {
                glass: creation
                    .wgpu_render_state
                    .as_ref()
                    .map(glass_demo::GlassDemo::new),
                tree: UiTree::new(Element::spacer()),
                state,
                started: false,
                screenshot,
                hover,
                scroll_end,
                opened: Instant::now(),
                requested: false,
            }))
        }),
    )
}
