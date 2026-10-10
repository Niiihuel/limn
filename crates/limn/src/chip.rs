//! Passive inline counts and labels with a fixed-height pill surface.

use crate::{Component, ComponentPalette, Element, FontFamily};

/// A compact informational label. Place it in an inline row so its width follows
/// the measured text; a one-digit ASCII count uses a circular surface instead.
/// The chip never registers an action, receives focus or adds a dismissal.
pub struct Chip {
    label: String,
    palette: ComponentPalette,
    height: f32,
    outline: bool,
}

impl Chip {
    pub fn new(label: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            label: label.into(),
            palette,
            height: 22.0,
            outline: false,
        }
    }

    /// Draw the border only, with no fill, for a label that should sit
    /// quietly on its surface.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the fixed height, in logical pixels (22 by default, at least 16).
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(16.0);
        }
        self
    }
}

impl Component for Chip {
    fn render(self) -> Element {
        let circular_count = self.label.len() == 1 && self.label.as_bytes()[0].is_ascii_digit();
        let centered = Element::text(self.label)
            .font_family(FontFamily::Sans)
            .font_size(11.0)
            .foreground(self.palette.text)
            .center_content()
            .centered_y();
        let content = if circular_count {
            centered
        } else {
            // Keep the label at its measured width. Reconstructing that width
            // by subtracting padding from a flex allocation can lose an f32
            // unit and wrap a label which fitted during intrinsic measurement.
            Element::row(vec![
                Element::spacer().min_size(8.0, 0.0).flex(1.0),
                centered,
                Element::spacer().min_size(8.0, 0.0).flex(1.0),
            ])
        };
        let mut surface = Element::panel(content);
        if !self.outline {
            surface = surface.background(self.palette.selected);
        }
        surface
            .border(self.palette.border, 1.0)
            .radius(self.height / 2.0)
            .min_size(self.height, self.height)
            .size(circular_count.then_some(self.height), Some(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, FontMetrics, Key, Point, Size, UiEvent, UiTree};

    #[test]
    fn installed_labels_remain_complete_on_one_line_inside_a_centered_row() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["Instalada · Usuario", "Instalada · Proyecto"] {
            for row_height in [28.0, 44.0] {
                let control = Chip::new(label, palette).render();
                let centered = Element::column(vec![
                    Element::spacer().flex(1.0),
                    control,
                    Element::spacer().flex(1.0),
                ]);
                let mut tree = UiTree::new(
                    Element::row(vec![centered, Element::spacer().flex(1.0)])
                        .size(None, Some(row_height)),
                );
                let frame = tree.layout(Size::new(920.0, row_height));
                let surface = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, color, .. } if *color == palette.selected => {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                let blocks = frame.text_layouts();
                let block = blocks.iter().find(|block| block.source == label).unwrap();
                assert_eq!(block.runs.len(), 1, "{label} at row height {row_height}");
                let text = &block.runs[0];
                assert_eq!(text.text, label);
                assert_eq!(surface.height, 22.0);
                assert!((surface.width - text.rect.width - 16.0).abs() < 0.01);
                assert!((text.rect.x - surface.x - 8.0).abs() < 0.01);
                assert!(text.rect.y >= surface.y);
                assert!(text.rect.y + text.rect.height <= surface.y + surface.height);
                assert!((surface.y + surface.height / 2.0 - row_height / 2.0).abs() < 0.01);
                assert!(
                    (text.rect.y + text.rect.height / 2.0 - surface.y - surface.height / 2.0).abs()
                        < 0.01
                );
            }
        }
    }

    #[test]
    fn counts_and_unicode_labels_use_measured_width_and_centered_copy() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["1", "12", "999", "Reparación · 模型 🦀", "W", "終"] {
            for row_height in [28.0, 44.0] {
                let mut tree = UiTree::new(
                    Element::row(vec![Chip::new(label, palette).render()])
                        .size(None, Some(row_height)),
                );
                // Exercise the host's proportional measurement rather than
                // calculating a label width from a guessed character count.
                tree.set_text_measurer(|text, size, _| {
                    text.chars()
                        .map(|character| match character {
                            '1' => 4.0,
                            'W' => 10.0,
                            '終' | '模' | '型' | '🦀' => 11.0,
                            _ => 6.0,
                        })
                        .sum::<f32>()
                        * size
                        / 11.0
                });
                tree.set_sans_metrics(FontMetrics {
                    char_width: 7.0,
                    line_height: 18.0,
                });
                let frame = tree.layout(Size::new(600.0, row_height));
                let (surface, radius) = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect {
                            rect,
                            color,
                            radius,
                        } if *color == palette.selected => Some((*rect, *radius)),
                        _ => None,
                    })
                    .unwrap();
                let blocks = frame.text_layouts();
                let block = blocks.iter().find(|block| block.source == label).unwrap();
                assert_eq!(block.runs.len(), 1);
                let text = &block.runs[0];
                assert_eq!(text.text, label);
                assert_eq!(surface.height, 22.0);
                assert_eq!(radius, surface.height / 2.0);
                if label == "1" {
                    assert_eq!(surface.width, surface.height);
                } else {
                    assert!((surface.width - (text.rect.width + 16.0).max(22.0)).abs() < 0.1);
                }
                assert!(
                    (text.rect.x + text.rect.width / 2.0 - surface.x - surface.width / 2.0).abs()
                        < 0.1
                );
                assert!(
                    (text.rect.y + text.rect.height / 2.0 - surface.y - surface.height / 2.0).abs()
                        < 0.1
                );
                assert!(text.visible.is_some());
                assert!(frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { font_family: FontFamily::Sans, font_size, .. } if *font_size == 11.0
                )));
            }
        }
    }

    #[test]
    fn outline_keeps_the_border_and_geometry_without_a_fill() {
        let palette = ComponentPalette::neutral_dark();
        let frame_of = |chip: Chip| {
            UiTree::new(Element::row(vec![chip.height(20.0).render()])).layout(Size::new(300.0, 32.0))
        };
        let filled = frame_of(Chip::new("Plan Pro", palette));
        let outline = frame_of(Chip::new("Plan Pro", palette).outline());
        assert!(!outline.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { .. })));
        let without_fill: Vec<_> = filled
            .commands
            .iter()
            .filter(|command| !matches!(command, DrawCommand::FillRect { .. }))
            .cloned()
            .collect();
        assert_eq!(outline.commands, without_fill);
        assert!(outline.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { rect, color, radius, .. }
                if *color == palette.border && *radius == 10.0 && rect.height == 20.0
        )));
        assert!(outline.regions.is_empty());
    }

    #[test]
    fn chip_remains_passive_under_pointer_and_keyboard_events() {
        let mut tree = UiTree::new(Element::row(vec![Chip::new(
            "1",
            ComponentPalette::neutral_dark(),
        )
        .render()]));
        let frame = tree.layout(Size::new(120.0, 44.0));
        assert!(frame.regions.is_empty());
        assert_eq!(tree.focused_id(), None);
        assert!(!tree.focus("chip"));
        let point = Point::new(11.0, 11.0);
        for event in [
            UiEvent::PointerMove(point),
            UiEvent::PointerDown(point),
            UiEvent::PointerUp(point),
            UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
            },
            UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
            },
        ] {
            assert!(tree.handle_event(event).is_empty());
        }
        assert_eq!(tree.focused_id(), None);
        assert!(!tree.is_animating());
        assert!(tree.layout(Size::new(120.0, 44.0)).regions.is_empty());
    }

    #[test]
    fn custom_height_keeps_a_real_pill_and_does_not_stretch_with_the_row() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["1", "12", "Nombre Unicode 終わり"] {
            let mut tree = UiTree::new(Element::row(vec![Chip::new(label, palette)
                .height(28.0)
                .render()]));
            let frame = tree.layout(Size::new(600.0, 44.0));
            let (rect, radius) = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *color == palette.selected => Some((*rect, *radius)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(rect.height, 28.0);
            assert_eq!(radius, 14.0);
            assert!(rect.width >= rect.height);
            if label == "1" {
                assert_eq!(rect.width, 28.0);
            }
            assert_eq!(frame.text_layouts()[0].source, label);
        }
    }
}
