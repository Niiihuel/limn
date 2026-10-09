//! A moving highlight painted over measured text, with no layout changes.

use crate::{Color, Component, ComponentPalette, Element, FontFamily};

pub struct TextShimmer {
    id: String,
    text: String,
    base: Color,
    highlight: Color,
}

impl TextShimmer {
    pub fn new(id: impl Into<String>, text: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            base: palette.muted,
            highlight: palette.text,
        }
    }

    pub fn colors(mut self, base: Color, highlight: Color) -> Self {
        self.base = base;
        self.highlight = highlight;
        self
    }
}

impl Component for TextShimmer {
    fn render(self) -> Element {
        Element::shimmer_text(self.id, self.text, self.highlight)
            .foreground(self.base)
            .font_family(FontFamily::Sans)
            .font_size(13.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Key, Point, ScrollArea, Size, UiAction, UiEvent, UiTree};
    use std::time::{Duration, Instant};

    #[test]
    fn shimmer_moves_over_measured_text_without_restarting_or_changing_selection() {
        let label = "Pensando · acción ñ 終";
        let make = || {
            TextShimmer::new("model-request-1", label, ComponentPalette::neutral_dark()).render()
        };
        let start = Instant::now();
        let mut tree = UiTree::new(make());
        tree.set_text_measurer(|value, _, _| {
            value
                .chars()
                .map(|ch| if ch == 'i' { 3.0 } else { 8.0 })
                .sum()
        });
        let viewport = Size::new(300.0, 40.0);
        let first = tree.layout_at(viewport, start);
        let moving = tree.layout_at(viewport, start + Duration::from_millis(640));
        assert_ne!(first.commands, moving.commands);
        assert_eq!(first.text_layouts(), moving.text_layouts());
        assert!(tree.is_animating());
        tree.set_root(make());
        let rebuilt = tree.layout_at(viewport, start + Duration::from_millis(640));
        assert_eq!(
            moving.commands, rebuilt.commands,
            "stable ID retains the phase across rebuilds"
        );
        assert!(rebuilt.regions.is_empty());
        let area = rebuilt.text_layouts()[0].runs[0].rect;
        tree.handle_event(UiEvent::PointerDown(Point::new(area.x, area.y + 4.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(
            area.x + area.width + 1.0,
            area.y + 4.0,
        )));
        tree.handle_event(UiEvent::PointerUp(Point::new(
            area.x + area.width + 1.0,
            area.y + 4.0,
        )));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::C,
                ctrl: true,
                shift: false
            }),
            vec![UiAction::CopyRequested { text: label.into() }]
        );
    }

    #[test]
    fn shimmer_reduced_motion_and_offscreen_content_stop_repainting() {
        let start = Instant::now();
        let palette = ComponentPalette::neutral_dark();
        let mut tree =
            UiTree::new(TextShimmer::new("reading", "Leyendo archivos", palette).render());
        tree.set_reduced_motion(true);
        let static_first = tree.layout_at(Size::new(200.0, 40.0), start);
        let static_later = tree.layout_at(Size::new(200.0, 40.0), start + Duration::from_secs(2));
        assert_eq!(static_first.commands, static_later.commands);
        assert!(!tree.is_animating());
        assert_eq!(
            static_first
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count(),
            1
        );
        tree.set_reduced_motion(false);
        tree.set_root(
            ScrollArea::new(
                "timeline",
                Element::column(vec![
                    Element::spacer().size(None, Some(400.0)),
                    TextShimmer::new("hidden", "Leyendo archivos", palette).render(),
                ]),
            )
            .render()
            .size(None, Some(60.0)),
        );
        tree.layout_at(Size::new(200.0, 60.0), start + Duration::from_secs(3));
        assert!(
            !tree.is_animating(),
            "offscreen loading text has no repaint loop"
        );
        tree.set_root(Element::text("Respuesta completada"));
        tree.layout_at(Size::new(200.0, 40.0), start + Duration::from_secs(4));
        assert!(!tree.is_animating());
    }
}
