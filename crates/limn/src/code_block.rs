//! The labelled card that holds a block of code inside prose.

use crate::{Component, ComponentPalette, Element, FontFamily, RadiusRole, TextRole, TextSpan};

/// The type of a [`CodeBlock`]: the label's role and family, and the role of
/// the code, which is always monospaced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeBlockSizing {
    pub label: TextRole,
    pub label_mono: bool,
    pub code: TextRole,
}

impl CodeBlockSizing {
    /// A conversation: a small monospaced label over 12 px code.
    pub const CHAT: Self = Self {
        label: TextRole::Micro,
        label_mono: true,
        code: TextRole::Caption,
    };
    /// A document: a sans label over 13 px code.
    pub const DOCUMENT: Self = Self {
        label: TextRole::Small,
        label_mono: false,
        code: TextRole::Label,
    };
}

/// Wrapped, passive code on the palette's `raised` fill, under an optional
/// label such as the language. The host writes the label in the case it
/// wants, and may pass the text already colored by its own highlighter.
pub struct CodeBlock {
    text: String,
    palette: ComponentPalette,
    label: Option<String>,
    spans: Option<Vec<TextSpan>>,
    sizing: CodeBlockSizing,
    width: Option<f32>,
}

impl CodeBlock {
    pub fn new(text: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            text: text.into(),
            palette,
            label: None,
            spans: None,
            sizing: CodeBlockSizing::CHAT,
            width: None,
        }
    }

    /// A muted line above the code.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Paint these spans in place of the plain text. Each span carries its
    /// own color, size and family.
    pub fn spans(mut self, spans: Vec<TextSpan>) -> Self {
        self.spans = Some(spans);
        self
    }

    /// The type of the label and the code ([`CodeBlockSizing::CHAT`] unless set).
    pub fn sizing(mut self, sizing: CodeBlockSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// Fix the width, in logical pixels, and wrap the code inside it. Without
    /// it the block fills the width it is given. Values that are negative or
    /// not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width >= 0.0 {
            self.width = Some(width);
        }
        self
    }
}

const PADDING: f32 = 12.0;

impl Component for CodeBlock {
    fn render(self) -> Element {
        let mut children = Vec::new();
        if let Some(label) = self.label {
            children.push(
                Element::text(label)
                    .font_family(if self.sizing.label_mono {
                        FontFamily::Monospace
                    } else {
                        FontFamily::Sans
                    })
                    .font_size(self.sizing.label.size())
                    .foreground(self.palette.muted),
            );
        }
        let code = match self.spans {
            Some(spans) => Element::formatted_text(spans),
            None => Element::text(self.text)
                .font_family(FontFamily::Monospace)
                .font_size(self.sizing.code.size())
                .foreground(self.palette.text),
        };
        children.push(match self.width {
            Some(width) => code.size(Some((width - 2.0 * PADDING).max(0.0)), None),
            None => code,
        });
        let block = Element::panel(Element::column(children).gap(8.0))
            .padding(PADDING)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(RadiusRole::Card.px());
        match self.width {
            Some(width) => block.size(Some(width), None),
            None => block,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};

    fn frame_of(block: CodeBlock, width: f32) -> crate::Frame {
        UiTree::new(Element::column(vec![block.render(), Element::spacer().flex(1.0)]))
            .layout(Size::new(width, 400.0))
    }

    fn text(frame: &crate::Frame, value: &str) -> (f32, FontFamily, crate::Color) {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, font_size, font_family, color, .. } if text == value => {
                    Some((*font_size, *font_family, *color))
                }
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn both_presets_set_the_label_and_the_code_in_their_own_type() {
        let palette = ComponentPalette::neutral_dark();
        let chat = frame_of(CodeBlock::new("ls", palette).label("SH"), 300.0);
        assert_eq!(text(&chat, "SH"), (10.0, FontFamily::Monospace, palette.muted));
        assert_eq!(text(&chat, "ls"), (12.0, FontFamily::Monospace, palette.text));
        let document = frame_of(
            CodeBlock::new("ls", palette).label("sh").sizing(CodeBlockSizing::DOCUMENT),
            300.0,
        );
        assert_eq!(text(&document, "sh"), (11.0, FontFamily::Sans, palette.muted));
        assert_eq!(text(&document, "ls"), (13.0, FontFamily::Monospace, palette.text));
        for frame in [&chat, &document] {
            assert!(frame.regions.is_empty());
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, radius, .. }
                    if *color == palette.raised && *radius == RadiusRole::Card.px()
            )));
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::StrokeRect { color, width, .. } if *color == palette.border && *width == 1.0
            )));
        }
    }

    #[test]
    fn the_label_is_optional_and_spans_replace_the_plain_text() {
        let palette = ComponentPalette::neutral_dark();
        let ink = palette.muted;
        let frame = frame_of(
            CodeBlock::new("plain", palette)
                .spans(vec![TextSpan::new("let", ink, 13.0, FontFamily::Monospace)]),
            300.0,
        );
        assert_eq!(text(&frame, "let"), (13.0, FontFamily::Monospace, ink));
        let texts = frame.commands.iter().filter(|command| matches!(command, DrawCommand::Text { .. })).count();
        assert_eq!(texts, 1);
    }

    #[test]
    fn code_wraps_inside_a_narrow_width_and_bad_widths_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        let source = "cargo test --workspace --no-fail-fast --all-features";
        let fill = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == palette.raised => Some(*rect),
                    _ => None,
                })
                .unwrap()
        };
        let narrow = frame_of(CodeBlock::new(source, palette).width(140.0), 400.0);
        assert_eq!(fill(&narrow).width, 140.0);
        let layouts = narrow.text_layouts();
        let block = layouts.iter().find(|layout| layout.source == source).unwrap();
        assert!(block.runs.len() > 1);
        for run in &block.runs {
            assert!(run.rect.x + run.rect.width <= 140.0 - PADDING + 0.5);
        }
        for bad in [f32::NAN, f32::INFINITY, -1.0] {
            let frame = frame_of(CodeBlock::new(source, palette).width(bad), 400.0);
            assert_eq!(fill(&frame).width, 400.0);
        }
    }
}
