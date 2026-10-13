//! A conversation's turns as a rail of marks beside it: one mark per turn,
//! raised around the pointer, with the turn's opening and reply in a tooltip.

use crate::{ComponentPalette, Element, MarkRail, RailMark};

pub struct ChatMinimapMark {
    id: String,
    title: String,
    preview: String,
}

impl ChatMinimapMark {
    /// `id` is reported as `UiAction::Clicked`; `title` is how the turn opens.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: clipped(&title.into(), 72),
            preview: String::new(),
        }
    }

    /// How the reply starts, under the title.
    pub fn preview(mut self, preview: impl AsRef<str>) -> Self {
        self.preview = clipped(preview.as_ref(), 180);
        self
    }
}

/// One line of at most `limit` characters, ending in an ellipsis when cut.
fn clipped(text: &str, limit: usize) -> String {
    let mut words = text.split_whitespace();
    let mut line = words.next().unwrap_or_default().to_owned();
    for word in words {
        line.push(' ');
        line.push_str(word);
    }
    if line.chars().count() > limit {
        line = line.chars().take(limit - 1).collect::<String>().trim_end().to_owned();
        line.push('…');
    }
    line
}

pub struct ChatMinimap {
    id: String,
    marks: Vec<ChatMinimapMark>,
    active: Option<String>,
    palette: ComponentPalette,
}

impl ChatMinimap {
    /// A rail of these marks, one for each turn, first to last.
    pub fn new(
        id: impl Into<String>,
        marks: Vec<ChatMinimapMark>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            marks,
            active: None,
            palette,
        }
    }

    /// The ID of the mark for the turn the conversation shows now.
    pub fn active(mut self, id: impl Into<String>) -> Self {
        self.active = Some(id.into());
        self
    }

    /// The rail as an element; the same as [`Component::render`](crate::Component::render).
    pub fn render(self) -> Element {
        let active = self
            .active
            .and_then(|id| self.marks.iter().position(|mark| mark.id == id));
        let mut ink = self.palette.muted;
        ink.a = (f32::from(ink.a) * 0.6) as u8;
        Element::mark_rail(MarkRail {
            id: self.id,
            marks: self
                .marks
                .into_iter()
                .map(|mark| RailMark {
                    id: mark.id,
                    title: mark.title,
                    preview: mark.preview,
                })
                .collect(),
            active,
            pitch: 10.0,
            thickness: 2.0,
            rest: 8.0,
            peak: 26.0,
            reach: 4,
            ink,
            active_ink: self.palette.text,
            lead_ink: self.palette.text,
            tooltip: self.palette.tooltip_style(),
        })
    }
}

impl crate::Component for ChatMinimap {
    fn render(self) -> Element {
        ChatMinimap::render(self)
    }
}

#[cfg(test)]
#[path = "chat_minimap_tests.rs"]
mod tests;
