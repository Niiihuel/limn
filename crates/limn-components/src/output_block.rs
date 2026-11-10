//! Read-only monospaced output, bounded however much text it is given.

use crate::{Color, Component, ComponentPalette, Element, FontFamily, RadiusRole, ScrollArea, Text, TextRole};

// A result can hold megabytes; nothing past these reaches layout.
const MAX_BYTES: usize = 48 * 1024;
const MAX_LINES: usize = 600;
const MAX_LINE_CHARS: usize = 500;
const LINE_HEIGHT: f32 = 17.0;
const PADDING: f32 = 6.0;

/// The output of a command or a tool: one clipped line per row, in a scroll
/// area that builds only the rows in view. The text is cut to 48 KiB, 600
/// lines and 500 characters per line before any of it is shaped. The fill is
/// the palette's `surface`; the host passes a palette whose surface is its
/// code well.
pub struct OutputBlock {
    id: String,
    text: String,
    palette: ComponentPalette,
    ink: Color,
    max_height: f32,
    width: Option<f32>,
    truncation_note: Option<String>,
}

impl OutputBlock {
    /// `id` names the scroll area and must be unique in the window.
    pub fn new(id: impl Into<String>, text: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            palette,
            ink: palette.text,
            max_height: 200.0,
            width: None,
            truncation_note: None,
        }
    }

    /// Paint the lines in a color that carries meaning the host owns, such
    /// as an error stream.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = color;
        self
    }

    /// The tallest the lines may grow before they scroll (200 unless set, at
    /// least one line). Values that are not finite are ignored.
    pub fn max_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.max_height = height.max(LINE_HEIGHT);
        }
        self
    }

    /// Fix the width, in logical pixels. Without it the block fills the
    /// width it is given. Values that are negative or not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width >= 0.0 {
            self.width = Some(width);
        }
        self
    }

    /// A muted line under the output, shown only when text was cut.
    pub fn truncation_note(mut self, note: impl Into<String>) -> Self {
        self.truncation_note = Some(note.into());
        self
    }
}

impl Component for OutputBlock {
    fn render(self) -> Element {
        let source = self.text.as_str();
        let mut byte_end = source.len().min(MAX_BYTES);
        while !source.is_char_boundary(byte_end) {
            byte_end -= 1;
        }
        let mut source_lines = source[..byte_end].lines();
        let lines: Vec<String> = source_lines
            .by_ref()
            .take(MAX_LINES)
            .map(|line| {
                let mut chars = line.chars();
                let mut text: String = chars.by_ref().take(MAX_LINE_CHARS).collect();
                if chars.next().is_some() {
                    text.push('…');
                }
                // An empty row would have no line box to keep its height.
                if text.is_empty() {
                    text.push(' ');
                }
                text
            })
            .collect();
        let truncated = byte_end < source.len() || source_lines.next().is_some();
        let count = lines.len();
        let ink = self.ink;
        // The scrollbar keeps its lane inside the padding.
        let line_width = self.width.map(|width| (width - 20.0).max(100.0));
        let body = ScrollArea::uniform_list(self.id, count, LINE_HEIGHT, move |index| {
            Element::text(&lines[index])
                .foreground(ink)
                .font_family(FontFamily::Monospace)
                .font_size(TextRole::Small.size())
                .size(line_width, Some(LINE_HEIGHT))
                .clip_children()
        })
        .render()
        .top_fade(self.palette.surface, 12.0)
        .size(
            self.width.map(|width| (width - 2.0 * PADDING).max(120.0)),
            Some((count.max(1) as f32 * LINE_HEIGHT).min(self.max_height)),
        );
        let mut children = vec![body];
        if let Some(note) = self.truncation_note.filter(|_| truncated) {
            children.push(Text::new(note, self.palette).role(TextRole::Micro).muted().render());
        }
        let block = Element::column(children)
            .gap(4.0)
            .padding(PADDING)
            .background(self.palette.surface)
            .radius(RadiusRole::Control.px());
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

    fn frame_of(block: OutputBlock) -> crate::Frame {
        UiTree::new(Element::column(vec![block.render(), Element::spacer().flex(1.0)]))
            .layout(Size::new(400.0, 300.0))
    }

    fn surface_height(frame: &crate::Frame, palette: ComponentPalette) -> f32 {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius }
                    if *color == palette.surface && *radius == RadiusRole::Control.px() =>
                {
                    Some(rect.height)
                }
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn large_outputs_build_only_visible_rows_and_say_they_were_cut() {
        let palette = ComponentPalette::neutral_dark();
        let source: String = (0..100_000).map(|n| format!("line {n}\n")).collect();
        let frame = frame_of(
            OutputBlock::new("output", source, palette)
                .width(380.0)
                .max_height(120.0)
                .truncation_note("Partial view"),
        );
        assert!(frame.commands.len() < 100);
        let layouts = frame.text_layouts();
        assert!(layouts.iter().any(|layout| layout.source == "Partial view"));
        assert!(layouts.iter().any(|layout| layout.source == "line 0"));
        assert!(!layouts.iter().any(|layout| layout.source == "line 599"));
    }

    #[test]
    fn bytes_lines_and_line_length_are_bounded_on_character_boundaries() {
        let palette = ComponentPalette::neutral_dark();
        // Multi-byte text whose 48 KiB cut falls inside a character.
        let wide = "漢".repeat(40_000);
        let frame = frame_of(OutputBlock::new("wide", wide, palette).truncation_note("cut"));
        let layouts = frame.text_layouts();
        let line = layouts.iter().find(|layout| layout.source.starts_with('漢')).unwrap();
        assert_eq!(line.source.chars().count(), MAX_LINE_CHARS + 1);
        assert!(line.source.ends_with('…'));
        assert!(layouts.iter().any(|layout| layout.source == "cut"));

        let many = "x\n".repeat(MAX_LINES + 1);
        let cut = frame_of(OutputBlock::new("many", many, palette).truncation_note("cut"));
        assert!(cut.text_layouts().iter().any(|layout| layout.source == "cut"));
    }

    #[test]
    fn short_output_follows_its_lines_and_shows_no_note() {
        let palette = ComponentPalette::neutral_dark();
        let frame = frame_of(OutputBlock::new("short", "one\n\nthree", palette).truncation_note("cut"));
        assert!(!frame.text_layouts().iter().any(|layout| layout.source == "cut"));
        assert_eq!(surface_height(&frame, palette), 3.0 * LINE_HEIGHT + 2.0 * PADDING);
        // No text still holds one line.
        let empty = frame_of(OutputBlock::new("empty", "", palette));
        assert_eq!(surface_height(&empty, palette), LINE_HEIGHT + 2.0 * PADDING);
    }

    #[test]
    fn ink_colors_the_lines_and_bad_numbers_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        let ink = palette.muted;
        let frame = frame_of(OutputBlock::new("err", "failed", palette).ink(ink));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, font_family: FontFamily::Monospace, font_size, .. }
                if text == "failed" && *color == ink && *font_size == 11.0
        )));
        let tall = "x\n".repeat(40);
        let default = surface_height(&frame_of(OutputBlock::new("a", tall.clone(), palette)), palette);
        assert_eq!(default, 200.0 + 2.0 * PADDING);
        for bad in [f32::NAN, f32::INFINITY] {
            let block = OutputBlock::new("a", tall.clone(), palette).max_height(bad).width(bad);
            assert_eq!(surface_height(&frame_of(block), palette), default);
        }
        let floor = OutputBlock::new("a", tall, palette).max_height(-5.0);
        assert_eq!(surface_height(&frame_of(floor), palette), LINE_HEIGHT + 2.0 * PADDING);
    }
}
