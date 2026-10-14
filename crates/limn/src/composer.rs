//! Reusable composition surface. The host owns text and actions; this
//! component owns the editor card and its responsive slot layout.

use crate::components::{Component, ComponentPalette};
use crate::{Color, CornerRadii, Element, FontFamily, Point, TextHighlight};

const STRIP_INSET: f32 = 8.0;
const STRIP_PADDING: f32 = 4.0;
const STRIP_ROW: f32 = 28.0;

/// A message editor in a card with slots around it: attachments above the editor, a link
/// preview and a footer below it, and a context element outside the card. The editor is a
/// text area with the ID `editor_id`.
pub struct Composer {
    editor_id: String,
    prompt: String,
    placeholder: String,
    width: f32,
    editor_height: f32,
    auto_grow: Option<(f32, f32)>,
    context: Option<Element>,
    context_height: f32,
    attachments: Vec<Element>,
    footer: Option<Element>,
    pill_controls: Option<(Element, Element)>,
    inline_controls: bool,
    editor_font_size: f32,
    enter_submits: bool,
    editor_action: Option<Element>,
    editor_highlights: Vec<TextHighlight>,
    link_preview: Option<Element>,
    header_strip: Option<(Element, ComponentPalette)>,
    palette: ComponentPalette,
    border_color: Color,
    shadow_color: Color,
    card_radius: f32,
    enabled: bool,
}

impl Composer {
    /// A composer `width` logical pixels wide whose editor shows `prompt`, or `placeholder`
    /// while that is empty. A width that is negative or not finite is zero. The editor is
    /// 64 tall until `editor_height` or `auto_grow` says otherwise.
    pub fn new(
        editor_id: impl Into<String>,
        prompt: impl Into<String>,
        placeholder: impl Into<String>,
        width: f32,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            editor_id: editor_id.into(),
            prompt: prompt.into(),
            placeholder: placeholder.into(),
            width: if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            },
            editor_height: 64.0,
            auto_grow: None,
            context: None,
            context_height: 28.0,
            attachments: Vec::new(),
            footer: None,
            pill_controls: None,
            inline_controls: false,
            editor_font_size: 13.0,
            enter_submits: false,
            editor_action: None,
            editor_highlights: Vec::new(),
            link_preview: None,
            header_strip: None,
            palette,
            border_color: palette.border,
            shadow_color: palette.shadow,
            card_radius: crate::tokens().radius.panel,
            enabled: true,
        }
    }

    /// Fix the editor's height in logical pixels, never below 24 (64 by default), and turn
    /// `auto_grow` off. A value that is not finite is ignored.
    pub fn editor_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.editor_height = height.max(24.0);
            self.auto_grow = None;
        }
        self
    }

    /// Grow from the actual wrapped input lines within these outer heights.
    pub fn auto_grow(mut self, min_height: f32, max_height: f32) -> Self {
        if min_height.is_finite() && max_height.is_finite() {
            let minimum = min_height.max(24.0);
            self.auto_grow = Some((minimum, max_height.max(minimum)));
        }
        self
    }

    /// It never changed what is drawn. `inline_controls` is the compact form.
    #[deprecated(note = "it has no effect; use `inline_controls` for the compact form")]
    pub fn compact(self, _compact: bool) -> Self {
        self
    }

    /// An element outside the card: centered in a row above it, or under the pill when
    /// `pill_controls` is set.
    pub fn context(mut self, context: Element) -> Self {
        self.context = Some(context);
        self
    }

    /// The height of the row that holds `context` above the card (28 by default). The pill
    /// layout does not use it. A height that is not finite is ignored and a negative one is
    /// zero.
    pub fn context_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.context_height = height.max(0.0);
        }
        self
    }

    /// Elements stacked above the editor inside the card, such as `Attachment` cards.
    pub fn attachments(mut self, attachments: Vec<Element>) -> Self {
        self.attachments = attachments;
        self
    }

    /// An element at the bottom of the card, under the editor and the link preview, such as
    /// a row of actions.
    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Put a full-width editor above leading and trailing actions in one
    /// rounded composer pill, with attachments above the editor. In this layout
    /// `context` is the metadata row below it.
    pub fn pill_controls(mut self, leading: Element, trailing: Element) -> Self {
        self.pill_controls = Some((leading, trailing));
        self
    }

    /// Put an empty editor between its actions for a compact initial surface.
    /// The host switches back to the full-width editor when a draft is entered.
    pub fn inline_controls(mut self, inline: bool) -> Self {
        self.inline_controls = inline;
        self
    }

    /// Set the editor's text size in logical pixels (13 by default). A size that is zero,
    /// negative or not finite is ignored.
    pub fn editor_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.editor_font_size = size;
        }
        self
    }

    /// Make Enter send what was written and Shift+Enter break the line, as a chat
    /// does: the editor reports `Submitted` on Enter. Ctrl+Enter sends either way.
    /// Off by default: Enter breaks the line.
    pub fn enter_submits(mut self, submits: bool) -> Self {
        self.enter_submits = submits;
        self
    }

    /// An action beside the editor, with separate bounds so it cannot cover text.
    pub fn editor_action(mut self, action: Element) -> Self {
        self.editor_action = Some(action);
        self
    }

    /// Highlights for ranges of the editor's text. A disabled composer draws none.
    pub fn editor_highlights(mut self, highlights: Vec<TextHighlight>) -> Self {
        self.editor_highlights = highlights;
        self
    }

    /// A bounded link strip below the editable source. It never replaces the
    /// draft text, so selecting, copying and sending retain the original URLs.
    pub fn link_preview(mut self, preview: Element) -> Self {
        self.link_preview = Some(preview);
        self
    }

    /// The height [`Self::header_strip`] adds above the card.
    pub const HEADER_STRIP_HEIGHT: f32 = STRIP_ROW + 4.0;

    /// The width a header strip's content gets in a composer `width` wide.
    pub fn header_strip_width(width: f32) -> f32 {
        ((width - 2.0 * STRIP_INSET).max(0.0) - 2.0 * STRIP_PADDING).max(0.0)
    }

    /// A strip of metadata joined to the top of the card, such as the
    /// project and branch a message goes to. It is set in from the card's
    /// sides, filled with `palette.surface` and rounded at the top with the
    /// card's radius. Build `strip` [`Self::header_strip_width`] wide.
    pub fn header_strip(mut self, strip: Element, palette: ComponentPalette) -> Self {
        self.header_strip = Some((strip, palette));
        self
    }

    /// The color of the card's border; the palette's border by default.
    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    /// The color of the card's shadow; the palette's shadow by default. The pill layout
    /// draws no shadow.
    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    /// The card's corner radius; the `panel` radius token by default. A radius that is not
    /// finite is ignored and a negative one is zero.
    pub fn card_radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.card_radius = radius.max(0.0);
        }
        self
    }

    /// A disabled composer shows its draft, or its placeholder, as muted
    /// text in the editor's place: it has no hit region, takes no focus and
    /// no typing, and draws no highlights. The elements the host passes in,
    /// such as the footer, the attachments and the controls, are the host's
    /// to disable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Composer {
    fn render(mut self) -> Element {
        let Some((content, palette)) = self.header_strip.take() else {
            return self.card();
        };
        let (width, radius) = (self.width, self.card_radius);
        let strip_width = (width - 2.0 * STRIP_INSET).max(0.0);
        let strip = Element::column(vec![
            Element::spacer().size(None, Some(2.0)),
            Element::row(vec![
                Element::spacer().size(Some(STRIP_PADDING), None),
                content,
                Element::spacer().size(Some(STRIP_PADDING), None),
            ])
            .size(Some(strip_width), Some(STRIP_ROW)),
            Element::spacer().size(None, Some(2.0)),
        ])
        .background(palette.surface)
        .corner_radii(CornerRadii::top(radius))
        .size(Some(strip_width), Some(Self::HEADER_STRIP_HEIGHT));
        let strip = Element::row(vec![
            Element::spacer().size(Some(STRIP_INSET), None),
            strip,
            Element::spacer().size(Some(STRIP_INSET), None),
        ])
        .size(Some(width), Some(Self::HEADER_STRIP_HEIGHT));
        Element::column(vec![strip, self.card()]).size(Some(width), None)
    }
}

impl Composer {
    fn card(self) -> Element {
        let (id, prompt, placeholder) = (self.editor_id, self.prompt, self.placeholder);
        let (highlights, palette, enabled) = (self.editor_highlights, self.palette, self.enabled);
        let (font_size, height, auto_grow) = (self.editor_font_size, self.editor_height, self.auto_grow);
        let enter_submits = self.enter_submits;
        let editor = move |padding: f32| {
            if !enabled {
                // Text in the editor's box, as a disabled `TextArea` is: no
                // region, so no focus, caret or typing. It is still there for
                // the platform, as the editor is: named by its placeholder.
                let mut said = crate::Access::new(crate::Role::MultilineTextInput)
                    .id(id)
                    .value_text(prompt.clone())
                    .disabled(true);
                if !placeholder.is_empty() {
                    said = said.name(placeholder.clone());
                }
                let mut text = Element::panel(
                    Element::text(if prompt.is_empty() { placeholder } else { prompt })
                        .foreground(palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(font_size),
                )
                .background(palette.selected)
                .padding(padding)
                .radius(crate::tokens().radius.surface)
                .clip_children()
                .size(None, Some(height))
                .accessible(said);
                if let Some((minimum, maximum)) = auto_grow {
                    text = text.size(None, None).min_size(0.0, minimum);
                    text.style.max_height = Some(maximum);
                }
                return text;
            }
            let editor = Element::text_area(id, prompt, placeholder)
                .text_area_highlights(highlights)
                .text_area_enter_submits(enter_submits)
                .background(palette.selected)
                .foreground(palette.text)
                .font_family(FontFamily::Sans)
                .font_size(font_size)
                .padding(padding)
                .radius(crate::tokens().radius.surface)
                .size(None, Some(height));
            if let Some((minimum, maximum)) = auto_grow {
                editor.text_area_auto_grow(minimum, maximum)
            } else {
                editor
            }
        };
        if let Some((leading, trailing)) = self.pill_controls {
            let editor = editor(if self.inline_controls { 5.0 } else { 6.0 });
            let mut contents = self.attachments;
            if self.inline_controls {
                contents.push(
                    Element::row(vec![leading, editor.flex(1.0), trailing])
                        .gap(6.0)
                        .size(None, Some(28.0)),
                );
            } else {
                let editor = if let Some(action) = self.editor_action {
                    Element::row(vec![editor.flex(1.0), action]).gap(4.0)
                } else {
                    editor
                };
                let controls = Element::row(vec![leading, Element::spacer().flex(1.0), trailing])
                    .gap(6.0)
                    .size(None, Some(28.0));
                let mut editor_content = vec![editor];
                if let Some(preview) = self.link_preview {
                    editor_content.push(preview);
                }
                editor_content.push(controls);
                contents.push(Element::column(editor_content).gap(4.0));
            }
            if let Some(footer) = self.footer {
                contents.push(footer);
            }
            let body = Element::column(contents).gap(6.0);
            let pill = Element::panel(body)
                .background(self.palette.selected)
                .border(self.border_color, 1.0)
                .radius(self.card_radius)
                .padding(8.0)
                .size(Some(self.width), None);
            return if let Some(context) = self.context {
                Element::column(vec![pill, context])
                    .gap(4.0)
                    .size(Some(self.width), None)
            } else {
                pill
            };
        }
        let editor = editor(8.0);
        let mut contents = self.attachments;
        contents.push(editor);
        if let Some(preview) = self.link_preview {
            contents.push(preview);
        }
        if let Some(footer) = self.footer {
            contents.push(footer);
        }
        let card = Element::panel(Element::column(contents).gap(4.0))
            .background(self.palette.selected)
            .border(self.border_color, 1.0)
            .radius(self.card_radius)
            .shadow(self.shadow_color, Point::new(0.0, 5.0), 18.0, 0.0)
            .padding(8.0)
            .size(Some(self.width), None);
        let Some(context) = self.context else {
            return card;
        };
        let context = Element::row(vec![
            Element::spacer().flex(1.0),
            context,
            Element::spacer().flex(1.0),
        ])
        .size(Some(self.width), Some(self.context_height));
        Element::column(vec![context, card])
            .gap(0.0)
            .size(Some(self.width), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn the_editor_is_named_by_its_placeholder_and_gives_its_draft_enabled_or_not() {
        for enabled in [true, false] {
            let composer = Composer::new("prompt", "Borrador", "Escribe una tarea", 480.0, ComponentPalette::neutral_dark())
                .footer(Element::button("send", "Enviar"))
                .enabled(enabled)
                .render();
            let frame = UiTree::new(composer).layout(Size::new(480.0, 250.0));
            let said = frame.access.iter().find(|node| node.id == "prompt").unwrap();
            assert_eq!((said.role, said.name.as_str()), (crate::Role::MultilineTextInput, "Escribe una tarea"));
            assert_eq!(said.value_text.as_deref(), Some("Borrador"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
    }

    #[test]
    fn composer_keeps_context_outside_editor_and_routes_text_to_one_id() {
        let composer = Composer::new(
            "prompt",
            "",
            "Escribe una tarea",
            480.0,
            ComponentPalette::neutral_dark(),
        )
        .context(Element::button("project", "Proyecto"))
        .footer(Element::button("send", "Enviar"))
        .render();
        let mut tree = UiTree::new(composer);
        let frame = tree.layout(Size::new(480.0, 250.0));
        let project = frame
            .regions
            .iter()
            .find(|region| region.id == "project")
            .unwrap();
        let prompt = frame
            .regions
            .iter()
            .find(|region| region.id == "prompt")
            .unwrap();
        assert!(project.rect.y + project.rect.height <= prompt.rect.y);
        assert_eq!(
            frame
                .regions
                .iter()
                .filter(|region| region.id == "prompt")
                .count(),
            1
        );
        assert!(tree.focus("prompt"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("hola".into())),
            vec![UiAction::InputChanged {
                id: "prompt".into(),
                value: "hola".into(),
            }]
        );
        let enter = tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: false,
        });
        assert_eq!(
            enter,
            vec![UiAction::InputChanged {
                id: "prompt".into(),
                value: "hola\n".into(),
            }]
        );
    }

    #[test]
    fn a_disabled_composer_shows_its_draft_takes_no_typing_and_keeps_its_layout() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(480.0, 400.0);
        let draft = "primera línea\nsegunda línea";
        type Build = fn(Composer) -> Composer;
        let layouts: [Build; 4] = [
            |composer| composer,
            |composer| composer.auto_grow(28.0, 200.0),
            |composer| composer.pill_controls(Element::button("add", "+"), Element::button("go", "Ir")),
            |composer| {
                composer
                    .pill_controls(Element::button("add", "+"), Element::button("go", "Ir"))
                    .inline_controls(true)
                    .auto_grow(28.0, 200.0)
            },
        ];
        for (index, build) in layouts.into_iter().enumerate() {
            for prompt in [draft, ""] {
                let frame_of = |enabled: bool| {
                    let composer = build(Composer::new("prompt", prompt, "Escribe una tarea", 480.0, palette))
                        .footer(Element::button("send", "Enviar"))
                        .enabled(enabled)
                        .render();
                    let mut tree = UiTree::new(Element::column(vec![composer, Element::spacer().flex(1.0)]));
                    tree.set_reduced_motion(true);
                    let frame = tree.layout(size);
                    (tree, frame)
                };
                let (mut tree, frame) = frame_of(false);
                assert!(frame.regions.iter().all(|region| region.id != "prompt"), "{:?}", frame.regions);
                assert!(!tree.focus("prompt"));
                let (_, enabled) = frame_of(true);
                let region = |frame: &crate::Frame, id: &str| {
                    frame.regions.iter().find(|region| region.id == id).unwrap().rect
                };
                let editor = region(&enabled, "prompt");
                let point = crate::Point::new(editor.x + 20.0, editor.y + 10.0);
                assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
                assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
                assert!(tree.handle_event(UiEvent::Text("hola".into())).is_empty());
                let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
                assert!(tree.handle_event(enter).is_empty());
                // The draft, or the placeholder, is there in muted ink.
                let shown = frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        crate::DrawCommand::Text { text, position, color, .. }
                            if *color == palette.muted =>
                        {
                            Some((text.as_str(), *position))
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let lines: Vec<&str> = if prompt.is_empty() {
                    vec!["Escribe una tarea"]
                } else {
                    prompt.lines().collect()
                };
                assert_eq!(shown.iter().map(|line| line.0).collect::<Vec<_>>(), lines, "layout {index}");
                assert!(shown.iter().all(|line| line.1.x >= editor.x && line.1.y >= editor.y));
                // The host's controls stand where they did.
                assert_eq!(region(&frame, "send"), region(&enabled, "send"), "layout {index}");
                if index >= 2 {
                    assert_eq!(region(&frame, "go"), region(&enabled, "go"), "layout {index}");
                }
            }
        }
    }

    #[test]
    fn header_strip_sits_above_the_card_set_in_from_its_sides() {
        let palette = ComponentPalette::neutral_dark();
        let strip = ComponentPalette {
            surface: Color::rgb(1, 2, 3),
            ..palette
        };
        for width in [240.0, 480.0, 0.0, f32::NAN] {
            let shown = if width.is_finite() { width } else { 0.0 };
            let inner = Composer::header_strip_width(shown);
            let composer = Composer::new("prompt", "", "Escribe", width, palette)
                .pill_controls(Element::button("add", "+"), Element::button("send", "Enviar"))
                .card_radius(10.0)
                .header_strip(
                    Element::button("project", "limn").size(Some(inner), Some(28.0)),
                    strip,
                )
                .render();
            let mut tree = UiTree::new(Element::column(vec![composer, Element::spacer().flex(1.0)]));
            let frame = tree.layout(Size::new(shown.max(1.0), 300.0));
            if shown == 0.0 {
                continue;
            }
            let (fill, radii) = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    crate::DrawCommand::FillCornerRect { rect, color, radii, .. }
                        if *color == strip.surface =>
                    {
                        Some((*rect, *radii))
                    }
                    _ => None,
                })
                .unwrap();
            assert_eq!((fill.x, fill.y), (8.0, 0.0));
            assert_eq!((fill.width, fill.height), (width - 16.0, Composer::HEADER_STRIP_HEIGHT));
            assert_eq!(radii, CornerRadii::top(10.0));
            let region = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
            let project = region("project");
            assert_eq!((project.x, project.y, project.width), (12.0, 2.0, inner));
            assert_eq!(inner, width - 24.0);
            // The card starts where the strip ends.
            assert!(region("prompt").y >= Composer::HEADER_STRIP_HEIGHT);
        }
        assert_eq!(Composer::header_strip_width(10.0), 0.0);
    }
}
