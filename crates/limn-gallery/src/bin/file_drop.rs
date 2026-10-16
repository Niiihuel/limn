//! Receive-only Wayland demo: `cargo run -p limn-gallery --bin file_drop`.
//! Only URI-list metadata is inspected; no selected file is opened.

#[cfg(target_os = "linux")]
mod demo {
    use eframe::egui;
    use limn::{Component, Element, Point, Rect, ScrollArea, Size, Theme, UiAction, UiTree};
    use limn_dnd::{
        DropDecision, FileDropReceiver, OfferId, ReceiveEpoch, ReceiveEvent, ReceiveLimits,
        ReceiveStatus, SurfacePoint,
    };
    use std::{collections::VecDeque, time::Duration};

    const TARGET: &str = "file-drop-target";

    fn target(theme: Theme) -> Element {
        Element::clickable(TARGET, Element::spacer())
            .size(None, Some(152.0))
            .background(theme.surface(1))
            .border(theme.accent, 1.0)
            .drop_target()
    }

    #[derive(Clone, Copy, PartialEq)]
    struct Geometry {
        target: Rect,
        origin: Point,
        viewport: Size,
        native_scale: f64,
        pixels_per_point: f32,
        zoom: f32,
    }

    impl Geometry {
        fn inside(self, point: SurfacePoint) -> bool {
            let ratio = self.native_scale / f64::from(self.pixels_per_point);
            let x = point.x * ratio - f64::from(self.origin.x);
            let y = point.y * ratio - f64::from(self.origin.y);
            ratio.is_finite()
                && ratio > 0.0
                && x.is_finite()
                && y.is_finite()
                && x.abs() <= f64::from(f32::MAX)
                && y.abs() <= f64::from(f32::MAX)
                && self.target.contains(Point::new(x as f32, y as f32))
        }
    }

    struct Pending {
        id: OfferId,
        epoch: ReceiveEpoch,
        point: SurfacePoint,
        geometry: Geometry,
        ready: bool,
        decided: bool,
    }

    pub(super) struct App {
        receiver: Option<FileDropReceiver>,
        native_scale: Box<dyn Fn() -> f64>,
        tree: UiTree,
        fonts: bool,
        epoch: Option<ReceiveEpoch>,
        geometry: Option<Geometry>,
        pending: Option<Pending>,
        names: Vec<String>,
        log: VecDeque<String>,
        unavailable: Option<String>,
        closing: bool,
    }

    impl App {
        fn new(creation: &eframe::CreationContext<'_>) -> Self {
            limn_egui::install_fonts(&creation.egui_ctx);
            let mut app = Self {
                receiver: None,
                native_scale: Box::new(|| 1.0),
                tree: UiTree::new(Element::spacer()),
                fonts: false,
                epoch: None,
                geometry: None,
                pending: None,
                names: Vec::new(),
                log: VecDeque::new(),
                unavailable: None,
                closing: false,
            };
            if let Some(window) = creation.winit_window().cloned() {
                let scale_window = window.clone();
                app.native_scale = Box::new(move || scale_window.scale_factor());
                let repaint = creation.egui_ctx.clone();
                // This app keeps the root event loop dispatching until Closed, including
                // hidden-window logic and repeated native close requests. No FileDnd is attached.
                match unsafe {
                    FileDropReceiver::attach_owned(
                        window,
                        ReceiveLimits {
                            max_files: 8,
                            max_total_path_bytes: 32 * 1024,
                            // This demo asks a person to decide; the library default stays 2 s.
                            decision_timeout: Duration::from_secs(30),
                            ..ReceiveLimits::default()
                        },
                        move || repaint.request_repaint(),
                    )
                } {
                    Ok(Some(receiver)) => app.receiver = Some(receiver),
                    Ok(None) => {
                        app.unavailable = Some(
                            "Unavailable: this window is not Wayland. No legacy fallback.".into(),
                        )
                    }
                    Err(error) => {
                        app.unavailable = Some(format!("Receiver unavailable: {error:?}"))
                    }
                }
            } else {
                app.unavailable = Some("Unavailable: no owned root window.".into());
            }
            app.cut("initial context");
            app
        }

        fn note(&mut self, line: String) {
            if self.log.len() == 8 {
                self.log.pop_front();
            }
            self.log.push_back(line);
        }

        fn cut(&mut self, reason: &str) {
            self.pending = None;
            self.names.clear();
            if self.closing {
                return;
            }
            if let Some(receiver) = &self.receiver {
                match receiver.invalidate_pending() {
                    Ok(epoch) => self.epoch = Some(epoch),
                    Err(error) => {
                        self.epoch = None;
                        self.closing = true;
                        receiver.close();
                        self.note(format!("Context cut failed: {error:?}"));
                        return;
                    }
                }
                self.note(format!("Context cut: {reason}"));
            }
        }

        fn begin_close(&mut self) {
            if !self.closing {
                self.cut("window closing");
                self.closing = true;
                if let Some(receiver) = &self.receiver {
                    receiver.close();
                }
            }
        }

        fn closed(&self) -> bool {
            self.receiver
                .as_ref()
                .is_none_or(|receiver| matches!(receiver.status(), ReceiveStatus::Closed(_)))
        }

        fn drain(&mut self) {
            let events = self
                .receiver
                .as_ref()
                .map(FileDropReceiver::poll)
                .unwrap_or_default();
            for event in events {
                let redacted = format!("{event:?}");
                if !matches!(
                    event,
                    ReceiveEvent::Hovered { .. } | ReceiveEvent::Left { .. }
                ) {
                    println!("{redacted}");
                }
                self.note(redacted);
                match event {
                    ReceiveEvent::DropStarted { id, epoch, point } => {
                        let geometry = self.geometry.filter(|geometry| geometry.inside(point));
                        if !self.closing && self.epoch == Some(epoch) && geometry.is_some() {
                            self.names.clear();
                            self.pending = geometry.map(|geometry| Pending {
                                id,
                                epoch,
                                point,
                                geometry,
                                ready: false,
                                decided: false,
                            });
                        } else if let Some(receiver) = &self.receiver {
                            receiver.cancel(id);
                        }
                    }
                    ReceiveEvent::DropReady { id, paths } => {
                        let valid = self.pending.as_ref().is_some_and(|pending| {
                            pending.id == id
                                && self.epoch == Some(pending.epoch)
                                && self.geometry == Some(pending.geometry)
                                && !self.closing
                        });
                        if valid {
                            self.names = paths
                                .iter()
                                .filter_map(|path| path.file_name())
                                .map(|name| {
                                    name.to_string_lossy()
                                        .chars()
                                        .filter(|c| !c.is_control())
                                        .take(120)
                                        .collect()
                                })
                                .collect();
                            println!("metadata file_count={} names={:?}", paths.len(), self.names);
                            if let Some(pending) = &mut self.pending {
                                pending.ready = true;
                            }
                        } else if let Some(receiver) = &self.receiver {
                            receiver.cancel(id);
                        }
                    }
                    ReceiveEvent::DropEnded { id, .. }
                        if self
                            .pending
                            .as_ref()
                            .is_some_and(|pending| pending.id == id) =>
                    {
                        self.pending = None;
                    }
                    _ => {}
                }
            }
        }

        fn tick(&mut self, ctx: &egui::Context) {
            if ctx.input(|input| input.viewport().close_requested()) && !self.closed() {
                // Repeat this for every close request while cleanup or TLS is still pending.
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.begin_close();
            }
            if self.geometry.is_some_and(|geometry| {
                geometry.native_scale != (self.native_scale)()
                    || geometry.pixels_per_point != ctx.pixels_per_point()
                    || geometry.zoom != ctx.zoom_factor()
            }) {
                self.cut("scale or zoom changed");
                self.geometry = None;
            }
            if self.closing {
                if self.closed() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                } else {
                    ctx.request_repaint_after(Duration::from_millis(20));
                }
            }
        }

        fn view(&self, window: Size) -> Element {
            let summary = self.pending.as_ref().map(|pending| {
                format!(
                    "{:?} / {:?} / surface ({:.1}, {:.1}) / {} files",
                    pending.id,
                    pending.epoch,
                    pending.point.x,
                    pending.point.y,
                    self.names.len()
                )
            });
            let details = self
                .pending
                .as_ref()
                .zip(summary.as_deref())
                .map(|(pending, summary)| (summary, pending.ready, pending.decided));
            self.view_content(window, details)
        }

        fn view_content(&self, window: Size, details: Option<(&str, bool, bool)>) -> Element {
            let theme = Theme::DARK;
            let status = self.unavailable.clone().unwrap_or_else(|| {
                format!(
                    "Receiver: {:?}",
                    self.receiver.as_ref().map(FileDropReceiver::status)
                )
            });
            let mut diagnostics = vec![
                Element::text("After EOF, choose Accept or Reject within 30 seconds. Accept authorizes one local Copy finish attempt; it does not prepare or send files.")
                    .foreground(theme.text_sub()),
            ];
            if let Some((summary, ready, decided)) = details {
                diagnostics.push(Element::text(summary));
                if ready && !decided && !self.closing {
                    diagnostics.push(
                        Element::row(vec![
                            Element::button("accept", "Accept Copy"),
                            Element::button("reject", "Reject"),
                        ])
                        .gap(8.0),
                    );
                } else {
                    diagnostics.push(Element::text(if decided {
                        "Decision admitted; waiting for the local result."
                    } else {
                        "Reading the complete URI list…"
                    }));
                }
            }
            for name in &self.names {
                diagnostics.push(Element::text(name));
            }
            diagnostics
                .push(Element::text("Recent events (paths redacted)").foreground(theme.text_sub()));
            for line in &self.log {
                diagnostics.push(Element::text(line).font_size(12.0));
            }
            // Only metadata and diagnostics scroll: a new log entry must not move
            // the target whose geometry was frozen by DropStarted.
            let children = vec![
                Element::text("Native file receive").font_size(24.0),
                Element::text(status).foreground(theme.text_sub()),
                Element::text("Drop your two fixture files inside the outlined rectangle. No file content is opened."),
                target(theme),
                ScrollArea::new(
                    "file-drop-details",
                    Element::column(diagnostics).gap(12.0),
                )
                .render()
                .flex(1.0),
                Element::button(
                    "close",
                    if self.closing {
                        "Waiting for worker cleanup…"
                    } else {
                        "Close safely"
                    },
                ),
            ];
            Element::panel(Element::column(children).gap(12.0).padding(24.0))
                .foreground(theme.text())
                .background(theme.background)
                .size(Some(window.width), Some(window.height))
        }

        fn layout(&mut self, ctx: &egui::Context, area: egui::Rect) -> limn::Frame {
            let window = Size::new(area.width(), area.height());
            self.tree.set_root(self.view(window));
            let frame = self.tree.layout(window);
            let geometry = frame
                .regions
                .iter()
                .find(|region| region.id == TARGET)
                .and_then(|region| {
                    region
                        .rect
                        .intersection(Rect::new(0.0, 0.0, window.width, window.height))
                })
                .map(|target| Geometry {
                    target,
                    origin: Point::new(area.min.x, area.min.y),
                    viewport: window,
                    native_scale: (self.native_scale)(),
                    pixels_per_point: ctx.pixels_per_point(),
                    zoom: ctx.zoom_factor(),
                });
            if geometry != self.geometry {
                self.cut("target layout changed");
                self.geometry = geometry;
            }
            frame
        }

        fn action(&mut self, action: UiAction) {
            if let UiAction::Clicked { id } = action {
                if id == "close" {
                    self.begin_close();
                    return;
                }
                let decision = match id.as_str() {
                    "accept" => DropDecision::Accept,
                    "reject" => DropDecision::Reject,
                    _ => return,
                };
                let valid = self.pending.as_ref().is_some_and(|pending| {
                    pending.ready
                        && !pending.decided
                        && self.epoch == Some(pending.epoch)
                        && self.geometry == Some(pending.geometry)
                        && !self.closing
                });
                if valid {
                    if let (Some(receiver), Some(pending)) = (&self.receiver, &mut self.pending) {
                        match receiver.decide(pending.id, decision) {
                            Ok(()) => pending.decided = true,
                            Err(error) => {
                                self.pending = None;
                                self.note(format!("Decision not admitted: {error:?}"));
                            }
                        }
                    }
                }
            }
        }
    }

    impl eframe::App for App {
        fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
            self.tick(ctx);
        }

        fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
            let ctx = ui.ctx().clone();
            if !self.fonts {
                limn_egui::attach_fonts(&mut self.tree, &ctx);
                self.fonts = true;
            }
            limn_egui::normalize_keyboard_input(&ctx);
            let area = ui.max_rect();
            self.layout(&ctx, area);
            self.drain();
            for event in limn_egui::events(&ctx, area.min) {
                let actions = self.tree.handle_event(event);
                self.layout(&ctx, area); // Scroll/geometry cuts precede any decision action.
                for action in actions {
                    self.action(action);
                }
            }
            let actions = limn_egui::apply_access_requests(&ctx, &mut self.tree);
            self.layout(&ctx, area);
            for action in actions {
                self.action(action);
            }
            let frame = self.layout(&ctx, area);
            limn_egui::paint_at(ui, &frame, area.min);
            limn_egui::name_window(&ctx, "Native file receive");
            self.tick(&ctx);
        }
    }

    pub(super) fn run() -> eframe::Result {
        eframe::run_native(
            "limn file receive",
            eframe::NativeOptions {
                renderer: eframe::Renderer::Wgpu,
                viewport: egui::ViewportBuilder::default()
                    .with_inner_size([700.0, 760.0])
                    .with_min_inner_size([520.0, 440.0]),
                ..Default::default()
            },
            Box::new(|creation| Ok(Box::new(App::new(creation)))),
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use limn::UiEvent;

        fn ready_metadata_keeps_target_stable(window: Size) {
            // Presentation state only: protocol identities remain opaque and are not fabricated.
            let mut app = App {
                receiver: None,
                native_scale: Box::new(|| 1.0),
                tree: UiTree::new(Element::spacer()),
                fonts: false,
                epoch: None,
                geometry: None,
                pending: None,
                names: Vec::new(),
                log: VecDeque::new(),
                unavailable: Some("Receiver: Ready".into()),
                closing: false,
            };
            let mut tree = UiTree::new(app.view_content(window, None));
            let initial = tree.layout(window);
            let target_rect = initial
                .regions
                .iter()
                .find(|region| region.id == TARGET)
                .expect("initial target")
                .rect;
            assert!(target_rect.width > 0.0);
            assert_eq!(target_rect.height, 152.0);
            assert!(target_rect.y >= 0.0 && target_rect.y + target_rect.height <= window.height);

            app.names = vec!["first-fixture.txt".into(), "second-fixture.txt".into()];
            for index in 0..8 {
                app.note(format!(
                    "Event {index}: {}",
                    "bounded receiver diagnostic with no paths or file content; ".repeat(12)
                ));
            }
            let details = Some((
                "Received offer / current epoch / surface (100.0, 200.0) / 2 files",
                true,
                false,
            ));
            tree.set_root(app.view_content(window, details));
            let ready = tree.layout(window);
            let ready_rect = ready
                .regions
                .iter()
                .find(|region| region.id == TARGET)
                .expect("target remains available while awaiting a Copy decision")
                .rect;
            assert!(ready.regions.iter().any(|region| region.id == "accept"));
            assert!(ready.regions.iter().any(|region| region.id == "reject"));
            assert_eq!(
                ready_rect, target_rect,
                "received metadata and bounded diagnostics must not invalidate the frozen target"
            );
        }

        #[test]
        fn demo_ready_metadata_keeps_target_stable_700x760() {
            ready_metadata_keeps_target_stable(Size::new(700.0, 760.0));
        }

        #[test]
        fn demo_ready_metadata_keeps_target_stable_980x800() {
            ready_metadata_keeps_target_stable(Size::new(980.0, 800.0));
        }

        #[test]
        fn demo_target_has_a_real_hit_region_and_accepts_drop_hit_testing() {
            let mut tree = UiTree::new(Element::column(vec![target(Theme::DARK)]).padding(24.0));
            let frame = tree.layout(Size::new(700.0, 400.0));
            let region = frame
                .regions
                .iter()
                .find(|region| region.id == TARGET)
                .expect("the visible drop target must participate in actual limn hit testing");
            let rect = region.rect;
            assert!(rect.width > 0.0);
            assert_eq!(rect.height, 152.0);
            let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            assert_eq!(
                tree.handle_event(UiEvent::FilesHovered { position: center }),
                vec![UiAction::FilesHovered {
                    position: center,
                    over: Some(TARGET.into())
                }],
            );
            let outside = Point::new(rect.x + rect.width + 1.0, rect.y + rect.height / 2.0);
            assert_eq!(
                tree.handle_event(UiEvent::FilesHovered { position: outside }),
                vec![UiAction::FilesHovered {
                    position: outside,
                    over: None
                }],
            );
        }
    }
}

#[cfg(target_os = "linux")]
fn main() -> eframe::Result {
    demo::run()
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("The native file-receive demo is available on Linux/Wayland only.");
}
