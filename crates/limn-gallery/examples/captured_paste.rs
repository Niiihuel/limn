//! Synthetic captured-paste scenarios; this code does not request clipboard I/O.
//!
//! `cargo run -p limn-gallery --example captured_paste -- --verify`
//! Add `--ordered-input` for synthetic source-order scenarios, or `--small` for 600×580.
//! Add `--light`, `--square`, or `--screenshot new.png`. Screenshots imply verify.
//! The logic deadline needs ticks; use an external watchdog for startup/starvation.
//! eframe may still initialize its default clipboard backend.

use eframe::egui;
use image::ImageEncoder;
use limn::{
    Button, CodeEditor, Color, Component, EditorController, Element, Radius, ScrollArea, Size,
    Theme, Tokens, UiAction, UiEvent, UiTree,
};
use limn_editor::{EditKind, Selection, TextBuffer};
use std::{
    cell::Cell,
    collections::VecDeque,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

const FIELD: &str = "captured-paste-field";
const EDITOR: &str = "captured-paste-editor";
const FIELD_TEXT: &str = "Hola ñ · λ";
const CODE_TEXT: &str = "let saludo = \"ñ λ\";\n";
const REPLY: &str = "\nλ · café";
const DEADLINE: Duration = Duration::from_secs(10);
const LIGHT: Theme = Theme {
    background: Color::rgb(255, 255, 255),
    foreground: Color::rgb(24, 26, 31),
    accent: Color::rgb(9, 105, 218),
};

#[derive(Default)]
struct Options {
    verify: bool,
    screenshot: Option<PathBuf>,
    light: bool,
    square: bool,
    ordered_input: bool,
    small: bool,
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut options = Self::default();
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--verify" => options.verify = true,
                "--ordered-input" => options.ordered_input = true,
                "--small" => options.small = true,
                "--light" => options.light = true,
                "--square" => options.square = true,
                "--screenshot" => {
                    options.screenshot = Some(PathBuf::from(
                        arguments
                            .next()
                            .ok_or("--screenshot needs a new PNG path")?,
                    ));
                    options.verify = true;
                }
                _ => return Err("Use --verify, --ordered-input, --small, --light, --square or --screenshot new.png".into()),
            }
        }
        Ok(options)
    }
}

#[derive(Default)]
struct Completion {
    verified: Cell<bool>,
    saved: Cell<bool>,
    failed: Cell<bool>,
}

#[derive(Clone, Copy)]
struct Checks {
    valid: bool,
    stale: bool,
    empty: bool,
}

impl Checks {
    fn passed(self) -> bool {
        self.valid && self.stale && self.empty
    }

    fn label(self) -> String {
        let mark = |ok| if ok { "PASS" } else { "FAIL" };
        format!(
            "Valid: {} · Stale: {} · Empty once: {}",
            mark(self.valid),
            mark(self.stale),
            mark(self.empty)
        )
    }
}

struct Report {
    field: Checks,
    editor: Checks,
}

impl Report {
    fn passed(&self) -> bool {
        self.field.passed() && self.editor.passed()
    }
}

struct OrderedResult {
    sequence: &'static str,
    visited: String,
    translated: String,
    captured: &'static str,
    passed: bool,
    field: String,
    editor: String,
}

enum DemoReport {
    Captured(Report),
    Ordered(Option<[OrderedResult; 2]>),
}

impl DemoReport {
    fn passed(&self) -> bool {
        match self {
            Self::Captured(report) => report.passed(),
            Self::Ordered(Some(rows)) => rows.iter().all(|row| row.passed),
            Self::Ordered(None) => false,
        }
    }
}

// This context has no window/backend; only its normalized synthetic snapshot is used.
fn indexed_fixture(
    navigation_first: bool,
) -> Result<(Vec<egui::Event>, Vec<limn_egui::IndexedUiEvent>), String> {
    let navigation = egui::Event::Key {
        key: egui::Key::ArrowDown,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            alt: true,
            ..Default::default()
        },
    };
    let marker = egui::Event::Zoom(1.0);
    let events = if navigation_first {
        vec![navigation, marker]
    } else {
        vec![marker, navigation]
    };
    let ctx = egui::Context::default();
    let mut observed = None;
    let mut output = ctx.run_ui(
        egui::RawInput {
            events,
            ..Default::default()
        },
        |ui| {
            limn_egui::normalize_keyboard_input(ui.ctx());
            let source = ui.ctx().input(|input| input.events.clone());
            observed = Some((
                source,
                limn_egui::events_indexed(ui.ctx(), egui::Pos2::ZERO),
            ));
        },
    );
    output.textures_delta.clear();
    observed.ok_or_else(|| "Synthetic input snapshot was not produced".into())
}

fn shown(value: &str) -> String {
    value
        .chars()
        .take(96)
        .collect::<String>()
        .replace('\n', "\\n")
}

fn controller() -> EditorController {
    EditorController::new(TextBuffer::new(CODE_TEXT).expect("small synthetic document"))
}

fn editor_text(editor: &EditorController) -> Result<String, String> {
    let shared = editor.shared_buffer();
    let buffer = shared
        .try_lock()
        .map_err(|_| "Synthetic buffer is busy".to_owned())?;
    let snapshot = buffer.snapshot();
    snapshot
        .text_range(0..snapshot.char_count())
        .map_err(|error| error.to_string())
}

fn current_text(tree: &UiTree, editor: &EditorController, id: &str) -> Result<String, String> {
    if id == FIELD {
        tree.input_value(id)
            .map(str::to_owned)
            .ok_or_else(|| "Synthetic field is missing".into())
    } else {
        editor_text(editor)
    }
}

fn check_destination(
    tree: &mut UiTree,
    editor: &EditorController,
    id: &str,
    initial: &str,
) -> Result<Checks, String> {
    if !tree.focus(id) {
        return Err("Synthetic destination is not focusable".into());
    }
    if id == FIELD {
        tree.set_input_cursor(id, initial.chars().count());
    } else {
        let shared = editor.shared_buffer();
        let mut buffer = shared
            .try_lock()
            .map_err(|_| "Synthetic buffer is busy".to_owned())?;
        buffer
            .set_selection(Selection::caret(initial.chars().count()))
            .map_err(|error| error.to_string())?;
    }
    let target = tree
        .capture_paste_target()
        .ok_or("Synthetic destination cannot be captured")?;
    let actions = tree.handle_event(UiEvent::PasteCaptured {
        target,
        text: REPLY.into(),
    });
    let expected = format!("{initial}{REPLY}");
    let valid_action = if id == FIELD {
        matches!(actions.as_slice(), [UiAction::InputChanged { id: changed, value }] if changed == id && value == &expected)
    } else {
        let shared = editor.shared_buffer();
        let buffer = shared
            .try_lock()
            .map_err(|_| "Synthetic buffer is busy".to_owned())?;
        let snapshot = buffer.snapshot();
        matches!(actions.as_slice(), [UiAction::EditorChanged { id: changed, document_id, revision }]
            if changed == id && *document_id == snapshot.document_id() && *revision == snapshot.revision())
    };
    let valid = valid_action && current_text(tree, editor, id)? == expected;

    let stale = tree
        .capture_paste_target()
        .ok_or("Synthetic stale capture is missing")?;
    tree.invalidate_paste_targets();
    let stale_actions = tree.handle_event(UiEvent::PasteCaptured {
        target: stale,
        text: "STALE".into(),
    });
    let stale = stale_actions.is_empty() && current_text(tree, editor, id)? == expected;

    let empty = tree
        .capture_paste_target()
        .ok_or("Synthetic empty capture is missing")?;
    let replay = empty.clone();
    let empty_actions = tree.handle_event(UiEvent::PasteCaptured {
        target: empty,
        text: String::new(),
    });
    let replay_actions = tree.handle_event(UiEvent::PasteCaptured {
        target: replay,
        text: "REPLAY".into(),
    });
    let empty = empty_actions.is_empty()
        && replay_actions.is_empty()
        && current_text(tree, editor, id)? == expected;
    Ok(Checks {
        valid,
        stale,
        empty,
    })
}

fn run_scenario(tree: &mut UiTree, editor: &EditorController) -> Result<Report, String> {
    let field = check_destination(tree, editor, FIELD, FIELD_TEXT)?;
    let code = check_destination(tree, editor, EDITOR, CODE_TEXT)?;
    Ok(Report {
        field,
        editor: code,
    })
}

fn panel(title: &str, content: Element, status: Option<String>, theme: Theme) -> Element {
    let mut children = vec![
        Element::text(title)
            .font_size(18.0)
            .foreground(theme.text()),
        content,
    ];
    children.push(
        Element::text(status.unwrap_or_else(|| "Waiting for the scenario".into()))
            .font_size(13.0)
            .foreground(theme.text_sub()),
    );
    Element::column(children)
        .gap(10.0)
        .padding(16.0)
        .background(theme.surface(1))
        .border(theme.outline(), 1.0)
        .radius(limn::tokens().radius.card)
}

fn view(
    value: &str,
    editor: &EditorController,
    report: Option<&DemoReport>,
    notes: &VecDeque<String>,
    window: Size,
    light: bool,
    square: bool,
) -> Element {
    let theme = if light { LIGHT } else { Theme::DARK };
    let ordered = matches!(report, Some(DemoReport::Ordered(_)));
    let legacy = match report {
        Some(DemoReport::Captured(report)) => Some(report),
        _ => None,
    };
    let final_state = matches!(report, Some(DemoReport::Ordered(Some(_))))
        .then(|| "Final state after sequence 2; historical results below".to_owned());
    let field = Element::text_area(FIELD, value, "Synthetic text")
        .size(None, Some(if ordered { 52.0 } else { 88.0 }))
        .background(theme.surface(2))
        .foreground(theme.text())
        .border(theme.outline(), 1.0)
        .radius(limn::tokens().radius.control);
    let code = CodeEditor::new(EDITOR, editor.clone())
        .palette(theme.palette())
        .height(if ordered { 84.0 } else { 150.0 })
        .render();
    let mut details = vec![
        panel(if ordered { "A · TextArea" } else { "TextArea" }, field,
            legacy.map(|r| r.field.label()).or_else(|| final_state.clone()), theme),
        panel(if ordered { "B · CodeEditor" } else { "CodeEditor" }, code,
            legacy.map(|r| r.editor.label()).or(final_state), theme),
        Element::text(if ordered {
            "Two synthetic normalized batches; Zoom is a fixture marker, not an SDK paste request. No clipboard, token or ACK. Panels show only the final state of sequence 2."
        } else {
            "A stored capture is delivered with synthetic Unicode text; an explicitly invalidated capture and an empty replay must leave it unchanged."
        }).foreground(theme.text_sub()),
    ];
    if let Some(DemoReport::Ordered(Some(rows))) = report {
        for row in rows {
            details.push(
                Element::column(vec![
                    Element::text(format!(
                        "{} · {}",
                        row.sequence,
                        if row.passed { "PASS" } else { "FAIL" }
                    ))
                    .foreground(theme.text()),
                    Element::text(format!(
                        "{} · translations: {} · captured {}",
                        row.visited, row.translated, row.captured
                    ))
                    .font_size(12.0)
                    .foreground(theme.text_sub()),
                    Element::text(format!("Historical A: {}", shown(&row.field)))
                        .font_size(12.0)
                        .foreground(theme.text()),
                    Element::text(format!("Historical B: {}", shown(&row.editor)))
                        .font_size(12.0)
                        .foreground(theme.text()),
                ])
                .gap(6.0)
                .padding(12.0)
                .background(theme.surface(1))
                .border(theme.outline(), 1.0)
                .radius(limn::tokens().radius.card),
            );
        }
    }
    for note in notes {
        details.push(
            Element::text(note)
                .font_size(12.0)
                .foreground(theme.text_sub()),
        );
    }
    Element::column(vec![
        Element::text(if ordered {
            "Indexed input order"
        } else {
            "Captured paste targets"
        })
        .font_size(24.0)
        .foreground(theme.text()),
        Element::text(if ordered {
            "Simulated indexed input · no clipboard reads or writes"
        } else {
            "Simulated delayed text · no clipboard reads or writes"
        })
        .foreground(theme.accent_text()),
        ScrollArea::new("captured-paste-details", Element::column(details).gap(14.0))
            .render()
            .flex(1.0),
        Element::row(vec![
            Button::new("run", "Run", theme.palette()).render(),
            Button::new(
                "theme",
                if light { "Dark theme" } else { "Light theme" },
                theme.palette(),
            )
            .render(),
            Button::new(
                "corners",
                if square {
                    "Default corners"
                } else {
                    "Square corners"
                },
                theme.palette(),
            )
            .render(),
            Button::new("close", "Close", theme.palette()).render(),
        ])
        .gap(8.0),
    ])
    .gap(14.0)
    .padding(20.0)
    .size(Some(window.width), Some(window.height))
    .background(theme.background)
    .foreground(theme.text())
}

struct App {
    tree: UiTree,
    editor: EditorController,
    value: String,
    report: Option<DemoReport>,
    notes: VecDeque<String>,
    options: Options,
    completion: Rc<Completion>,
    fonts: bool,
    pending_run: bool,
    opened: Instant,
    settled_frames: u8,
    capture_requested: bool,
    shot: std::sync::Arc<std::sync::Mutex<Option<std::sync::Arc<egui::ColorImage>>>>,
}

impl App {
    fn new(
        creation: &eframe::CreationContext<'_>,
        options: Options,
        completion: Rc<Completion>,
    ) -> Self {
        limn_egui::install_fonts(&creation.egui_ctx);
        Self {
            tree: UiTree::new(Element::spacer()),
            editor: controller(),
            value: FIELD_TEXT.into(),
            report: options.ordered_input.then_some(DemoReport::Ordered(None)),
            notes: VecDeque::new(),
            options,
            completion,
            fonts: false,
            pending_run: true,
            opened: Instant::now(),
            settled_frames: 0,
            capture_requested: false,
            shot: Default::default(),
        }
    }

    fn note(&mut self, line: &str) {
        if self.notes.len() == 8 {
            self.notes.pop_front();
        }
        self.notes.push_back(line.into());
    }

    fn reset(&mut self) {
        self.tree.invalidate_paste_targets();
        self.tree.set_input_value(FIELD, FIELD_TEXT);
        if !self.options.ordered_input {
            self.editor = controller();
        }
        self.value = FIELD_TEXT.into();
        self.report = self
            .options
            .ordered_input
            .then_some(DemoReport::Ordered(None));
        self.pending_run = true;
        self.opened = Instant::now();
        self.settled_frames = 0;
        self.capture_requested = false;
        self.shot = Default::default();
        self.completion.verified.set(false);
        self.completion.saved.set(false);
        self.completion.failed.set(false);
        self.note("Fresh synthetic scenario requested");
    }

    fn layout(&mut self, size: Size) -> limn::Frame {
        limn::set_tokens(Tokens {
            radius: if self.options.square {
                Radius::SQUARE
            } else {
                Radius::DEFAULT
            },
            ..Tokens::DEFAULT
        });
        self.tree.set_root(view(
            &self.value,
            &self.editor,
            self.report.as_ref(),
            &self.notes,
            size,
            self.options.light,
            self.options.square,
        ));
        self.tree.layout(size)
    }

    fn ordered_sequence(
        &mut self,
        size: Size,
        navigation_first: bool,
    ) -> Result<OrderedResult, String> {
        self.tree.invalidate_paste_targets();
        self.tree.set_input_value(FIELD, FIELD_TEXT);
        self.tree
            .set_input_cursor(FIELD, FIELD_TEXT.chars().count());
        self.value = FIELD_TEXT.into();
        {
            let shared = self.editor.shared_buffer();
            let mut buffer = shared.try_lock().map_err(|_| "Synthetic buffer is busy")?;
            let count = buffer.snapshot().char_count();
            buffer
                .edit(
                    0..count,
                    CODE_TEXT,
                    Selection::caret(CODE_TEXT.chars().count()),
                    EditKind::Replace,
                    0,
                )
                .map_err(|error| error.to_string())?;
        }
        self.layout(size);
        if !self.tree.focus(FIELD) {
            return Err("Synthetic A cannot be focused".into());
        }
        self.tree.layout(size);
        let (source, indexed) = indexed_fixture(navigation_first)?;
        let navigation_index = usize::from(!navigation_first);
        let provenance = indexed.len() == 1
            && indexed[0].source_index == Some(navigation_index)
            && matches!(
                indexed[0].event,
                UiEvent::Key {
                    key: limn::Key::ArrowDown,
                    ctrl: false,
                    shift: false,
                    ..
                }
            );
        let translated = format!(
            "{:?}",
            indexed
                .iter()
                .map(|item| item.source_index)
                .collect::<Vec<_>>()
        );
        let mut indexed = indexed.into_iter().peekable();
        let mut target = None;
        let mut captured = "missing";
        let mut visited = Vec::with_capacity(2);
        while indexed
            .peek()
            .is_some_and(|item| item.source_index.is_none())
        {
            let item = indexed.next().ok_or("Synthetic derived event is missing")?;
            if !self.tree.handle_event(item.event).is_empty() {
                return Err("Unexpected action from synthetic derived input".into());
            }
            self.layout(size);
        }
        for (index, event) in source.iter().enumerate() {
            let consumed = match event {
                egui::Event::Key {
                    key: egui::Key::ArrowDown,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if modifiers.alt => {
                    if !self.tree.focus(EDITOR) {
                        return Err("Synthetic B cannot be focused".into());
                    }
                    self.layout(size);
                    visited.push(format!("{index}: Alt+Down → B"));
                    true
                }
                egui::Event::Zoom(_) => {
                    self.layout(size);
                    captured = match self.tree.focused_id() {
                        Some(FIELD) => "A",
                        Some(EDITOR) => "B",
                        _ => return Err("Synthetic marker has no editable destination".into()),
                    };
                    target = self.tree.capture_paste_target();
                    visited.push(format!("{index}: marker → {captured}"));
                    false
                }
                _ => return Err("Unexpected source in the fixed synthetic batch".into()),
            };
            while indexed
                .peek()
                .is_some_and(|item| item.source_index == Some(index))
            {
                let item = indexed.next().ok_or("Synthetic translation is missing")?;
                if !consumed && !self.tree.handle_event(item.event).is_empty() {
                    return Err("Unexpected action before synthetic completion".into());
                }
                self.layout(size);
            }
        }
        if indexed.next().is_some() {
            return Err("Synthetic translation has no source index".into());
        }
        let actions = self.tree.handle_event(UiEvent::PasteCaptured {
            target: target.ok_or("Synthetic marker was not captured")?,
            text: REPLY.into(),
        });
        let field = current_text(&self.tree, &self.editor, FIELD)?;
        let editor = editor_text(&self.editor)?;
        let delivery = if navigation_first {
            let shared = self.editor.shared_buffer();
            let buffer = shared.try_lock().map_err(|_| "Synthetic buffer is busy")?;
            let snapshot = buffer.snapshot();
            matches!(actions.as_slice(), [UiAction::EditorChanged { id, document_id, revision }]
                if id == EDITOR && *document_id == snapshot.document_id() && *revision == snapshot.revision())
                && captured == "B"
                && editor == format!("{CODE_TEXT}{REPLY}")
        } else {
            actions.is_empty() && captured == "A" && editor == CODE_TEXT
        };
        self.value = field.clone();
        Ok(OrderedResult {
            sequence: if navigation_first {
                "1 · Alt+Down → marker · delivery to B"
            } else {
                "2 · marker → Alt+Down · stale reply rejected"
            },
            visited: visited.join("; "),
            translated,
            captured,
            passed: provenance
                && source.len() == 2
                && visited.len() == 2
                && self.tree.focused_id() == Some(EDITOR)
                && field == FIELD_TEXT
                && delivery,
            field,
            editor,
        })
    }

    fn run_ordered(&mut self, size: Size) -> Result<DemoReport, String> {
        let first = self.ordered_sequence(size, true)?;
        let second = self.ordered_sequence(size, false)?;
        Ok(DemoReport::Ordered(Some([first, second])))
    }

    fn action(&mut self, action: UiAction, ctx: &egui::Context) {
        match action {
            UiAction::InputChanged { id, value } if id == FIELD => self.value = value,
            UiAction::Clicked { id } => match id.as_str() {
                "run" => self.reset(),
                "theme" => self.options.light = !self.options.light,
                "corners" => self.options.square = !self.options.square,
                "close" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                _ => {}
            },
            _ => {}
        }
    }

    fn fail(&mut self, ctx: &egui::Context, reason: &str) {
        self.completion.failed.set(true);
        self.note(reason);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn tick(&mut self, ctx: &egui::Context) {
        let waiting = !self.completion.verified.get()
            || (self.options.screenshot.is_some() && !self.completion.saved.get());
        if waiting && self.opened.elapsed() >= DEADLINE {
            self.fail(
                ctx,
                "Verification deadline reached; external watchdog also required",
            );
        } else if waiting {
            ctx.request_repaint_after(Duration::from_millis(20));
        }
    }

    fn finish_frame(&mut self, ctx: &egui::Context) {
        if !self.completion.verified.get() || self.completion.failed.get() {
            return;
        }
        self.settled_frames = self.settled_frames.saturating_add(1);
        if self.settled_frames < 2 {
            ctx.request_repaint();
            return;
        }
        if let Some(path) = self.options.screenshot.as_ref() {
            if !self.capture_requested {
                self.capture_requested = true;
                let shot = self.shot.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::ScreenshotCallback::new(
                    move |image| *shot.lock().expect("screenshot slot") = Some(image),
                )));
            }
            let image = self.shot.lock().expect("screenshot slot").take();
            if let Some(image) = image {
                let saved = (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                    let output = std::fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(path)?;
                    let [width, height] = image.size;
                    image::codecs::png::PngEncoder::new(output).write_image(
                        image.as_raw(),
                        width.try_into()?,
                        height.try_into()?,
                        image::ExtendedColorType::Rgba8,
                    )?;
                    Ok(())
                })();
                if saved.is_err() {
                    self.fail(ctx, "Could not create the new screenshot PNG");
                    return;
                }
                self.completion.saved.set(true);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if self.options.verify {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
        self.tree.set_reduced_motion(true);
        self.tree
            .set_window_focused(ctx.input(|input| input.focused));
        limn_egui::normalize_keyboard_input(&ctx);
        let area = ui.max_rect();
        let size = Size::new(area.width(), area.height());
        self.layout(size);
        for event in limn_egui::events(&ctx, area.min) {
            let actions = self.tree.handle_event(event);
            for action in actions {
                self.action(action, &ctx);
            }
            self.layout(size);
        }
        for action in limn_egui::apply_access_requests(&ctx, &mut self.tree) {
            self.action(action, &ctx);
        }
        self.layout(size);
        if self.pending_run && ctx.input(|input| input.focused) {
            self.pending_run = false;
            let scenario = if self.options.ordered_input {
                self.run_ordered(size)
            } else {
                run_scenario(&mut self.tree, &self.editor).map(DemoReport::Captured)
            };
            match scenario {
                Ok(report) => {
                    let passed = report.passed();
                    self.value = self
                        .tree
                        .input_value(FIELD)
                        .unwrap_or(FIELD_TEXT)
                        .to_owned();
                    self.report = Some(report);
                    self.completion.verified.set(passed);
                    if passed {
                        self.note(if self.options.ordered_input {
                            "Verified synthetic indexed order: delivery to B, then stale A rejected"
                        } else {
                            "Verified field/editor valid, stale and empty-once scenarios"
                        });
                    } else {
                        self.fail(&ctx, "A synthetic scenario failed verification");
                    }
                }
                Err(_) => self.fail(&ctx, "Could not resolve the synthetic editable targets"),
            }
        }
        let frame = self.layout(size);
        limn_egui::paint_at(ui, &frame, area.min);
        limn_egui::name_window(
            &ctx,
            if self.options.ordered_input {
                "Indexed input order — simulated"
            } else {
                "Captured paste targets — simulated"
            },
        );
        if let Some(after) = self.tree.animation_repaint_after() {
            ctx.request_repaint_after(after);
        }
        self.finish_frame(&ctx);
        self.tick(&ctx);
    }
}

fn failure(message: &str) -> eframe::Error {
    eframe::Error::AppCreation(std::io::Error::other(message).into())
}

fn main() -> eframe::Result {
    let options = Options::parse().map_err(|error| failure(&error))?;
    let screenshot_required = options.screenshot.is_some();
    let ordered_input = options.ordered_input;
    let initial_size = if options.small {
        [600.0, 580.0]
    } else {
        [760.0, 820.0]
    };
    let completion = Rc::new(Completion::default());
    let observed = Rc::clone(&completion);
    eframe::run_native(
        if ordered_input {
            "Indexed input order — simulated"
        } else {
            "Captured paste targets — simulated"
        },
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(initial_size)
                .with_min_inner_size([600.0, 580.0]),
            ..Default::default()
        },
        Box::new(move |creation| Ok(Box::new(App::new(creation, options, observed)))),
    )?;
    if completion.failed.get()
        || !completion.verified.get()
        || (screenshot_required && !completion.saved.get())
    {
        return Err(failure(
            "Captured-paste verification or screenshot did not complete",
        ));
    }
    if ordered_input {
        println!("Indexed input verified: navigation-before-marker delivers B; marker-before-navigation rejects A; synthetic only");
    } else {
        println!(
            "Captured paste verified: valid/stale/empty-once; synthetic field and editor only"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> (UiTree, EditorController, Report) {
        let editor = controller();
        let mut tree = UiTree::new(view(
            FIELD_TEXT,
            &editor,
            None,
            &VecDeque::new(),
            Size::new(760.0, 820.0),
            false,
            false,
        ));
        tree.set_reduced_motion(true);
        tree.set_window_focused(true);
        tree.layout(Size::new(760.0, 820.0));
        let report = run_scenario(&mut tree, &editor).expect("synthetic scenario");
        (tree, editor, report)
    }

    #[test]
    fn scenario_delivers_unicode_to_the_displayed_field_and_editor() {
        let (tree, editor, report) = run();
        assert!(report.field.valid && report.editor.valid);
        assert_eq!(
            tree.input_value(FIELD),
            Some(format!("{FIELD_TEXT}{REPLY}").as_str())
        );
        assert_eq!(editor_text(&editor).unwrap(), format!("{CODE_TEXT}{REPLY}"));
    }

    #[test]
    fn scenario_rejects_the_simulated_late_reply_after_invalidation() {
        let (tree, editor, report) = run();
        assert!(report.field.stale && report.editor.stale);
        assert!(!tree.input_value(FIELD).unwrap().contains("STALE"));
        assert!(!editor_text(&editor).unwrap().contains("STALE"));
    }

    #[test]
    fn scenario_consumes_an_accepted_empty_reply_before_replay() {
        let (tree, editor, report) = run();
        assert!(report.field.empty && report.editor.empty);
        assert!(!tree.input_value(FIELD).unwrap().contains("REPLAY"));
        assert!(!editor_text(&editor).unwrap().contains("REPLAY"));
    }
}
