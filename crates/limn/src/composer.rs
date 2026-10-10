//! Reusable composition surface. The host owns text and actions; this
//! component owns the editor card and its responsive slot layout.

use crate::components::{Component, ComponentPalette};
use crate::{Color, Element, FontFamily, Point, TextHighlight};

pub struct Composer {
    editor_id: String,
    prompt: String,
    placeholder: String,
    width: f32,
    editor_height: f32,
    auto_grow: Option<(f32, f32)>,
    compact: bool,
    context: Option<Element>,
    context_height: f32,
    attachments: Vec<Element>,
    footer: Option<Element>,
    pill_controls: Option<(Element, Element)>,
    inline_controls: bool,
    editor_font_size: f32,
    editor_action: Option<Element>,
    editor_highlights: Vec<TextHighlight>,
    link_preview: Option<Element>,
    palette: ComponentPalette,
    border_color: Color,
    shadow_color: Color,
    card_radius: f32,
}

impl Composer {
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
            compact: false,
            context: None,
            context_height: 28.0,
            attachments: Vec::new(),
            footer: None,
            pill_controls: None,
            inline_controls: false,
            editor_font_size: 13.0,
            editor_action: None,
            editor_highlights: Vec::new(),
            link_preview: None,
            palette,
            border_color: palette.border,
            shadow_color: palette.shadow,
            card_radius: crate::tokens().radius.panel,
        }
    }

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

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn context(mut self, context: Element) -> Self {
        self.context = Some(context);
        self
    }

    pub fn context_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.context_height = height.max(0.0);
        }
        self
    }

    pub fn attachments(mut self, attachments: Vec<Element>) -> Self {
        self.attachments = attachments;
        self
    }

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

    pub fn editor_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.editor_font_size = size;
        }
        self
    }

    /// An action beside the editor, with separate bounds so it cannot cover text.
    pub fn editor_action(mut self, action: Element) -> Self {
        self.editor_action = Some(action);
        self
    }

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

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn shadow_color(mut self, color: Color) -> Self {
        self.shadow_color = color;
        self
    }

    pub fn card_radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.card_radius = radius.max(0.0);
        }
        self
    }
}

impl Component for Composer {
    fn render(self) -> Element {
        if let Some((leading, trailing)) = self.pill_controls {
            let editor = Element::text_area(self.editor_id, self.prompt, self.placeholder)
                .text_area_highlights(self.editor_highlights)
                .background(self.palette.selected)
                .foreground(self.palette.text)
                .font_family(FontFamily::Sans)
                .font_size(self.editor_font_size)
                .padding(if self.inline_controls { 5.0 } else { 6.0 })
                .radius(crate::tokens().radius.surface)
                .size(None, Some(self.editor_height));
            let editor = if let Some((minimum, maximum)) = self.auto_grow {
                editor.text_area_auto_grow(minimum, maximum)
            } else {
                editor
            };
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
        let editor = Element::text_area(self.editor_id, self.prompt, self.placeholder)
            .text_area_highlights(self.editor_highlights)
            .background(self.palette.selected)
            .foreground(self.palette.text)
            .font_family(FontFamily::Sans)
            .font_size(self.editor_font_size)
            .padding(8.0)
            .radius(crate::tokens().radius.surface)
            .size(None, Some(self.editor_height));
        let editor = if let Some((minimum, maximum)) = self.auto_grow {
            editor.text_area_auto_grow(minimum, maximum)
        } else {
            editor
        };
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
}
