//! The content of a popover that is more than a menu: a head, a body that
//! scrolls and a foot that stays.

use crate::{Component, ComponentPalette, Element, Separator};

/// Pass the rendered panel to `Popover::new` as its content. The popover
/// keeps the placement, the width and the surface; the panel only stacks
/// the parts, so a body the host built as a scroll area gives way first
/// when the window is short and the header and footer stay whole.
pub struct PopoverPanel {
    palette: ComponentPalette,
    header: Option<Element>,
    body: Option<Element>,
    body_max_height: Option<f32>,
    footer: Option<Element>,
}

impl PopoverPanel {
    /// An empty panel with hairlines between its parts and no padding of
    /// its own.
    pub fn new(palette: ComponentPalette) -> Self {
        Self {
            palette,
            header: None,
            body: None,
            body_max_height: None,
            footer: None,
        }
    }

    /// The head of the panel, usually a `PanelHeader`.
    pub fn header(mut self, header: Element) -> Self {
        self.header = Some(header);
        self
    }

    /// The content between header and footer. It takes the height it needs,
    /// and what is left when the popover is given less.
    pub fn body(mut self, body: Element) -> Self {
        self.body = Some(body);
        self
    }

    /// Cap the body's own height; pair it with a scroll area as the body.
    /// Values that are not finite are ignored.
    pub fn body_max_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.body_max_height = Some(height.max(0.0));
        }
        self
    }

    /// The foot of the panel, such as the action that leads further.
    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }
}

impl Component for PopoverPanel {
    fn render(self) -> Element {
        let rule = || Separator::horizontal(self.palette).render();
        let mut parts = Vec::with_capacity(5);
        let has_body = self.body.is_some();
        if let Some(header) = self.header {
            parts.push(header);
            if has_body || self.footer.is_some() {
                parts.push(rule());
            }
        }
        if let Some(body) = self.body {
            let body = match self.body_max_height {
                Some(height) => body.max_height(height),
                None => body,
            };
            parts.push(body.flex(1.0));
        }
        if let Some(footer) = self.footer {
            if has_body {
                parts.push(rule());
            }
            parts.push(footer);
        }
        // Four pixels: the parts read as one surface, not as stacked cards.
        Element::column(parts).gap(4.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Popover, Rect, ScrollArea, Size, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn rules(frame: &Frame) -> Vec<Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if *color == palette().border && rect.height == 1.0 =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    fn text_y(frame: &Frame, label: &str) -> f32 {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == label => Some(position.y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"))
    }

    fn panel(lines: usize) -> PopoverPanel {
        PopoverPanel::new(palette())
            .header(Element::text("Head").size(None, Some(28.0)))
            .body(
                ScrollArea::new(
                    "body",
                    Element::column(
                        (0..lines)
                            .map(|line| Element::text(format!("Line {line}")).size(None, Some(20.0)))
                            .collect(),
                    ),
                )
                .render(),
            )
            .footer(Element::text("Foot").size(None, Some(32.0)))
    }

    fn frame_of(panel: PopoverPanel, height: f32) -> Frame {
        let mut tree = UiTree::new(
            Popover::new(
                Element::button("anchor", "Open").size(Some(80.0), Some(28.0)),
                panel.render(),
                true,
                "dismiss",
                palette(),
            )
            .width(280.0)
            .render(),
        );
        tree.set_reduced_motion(true);
        tree.layout(Size::new(400.0, height))
    }

    #[test]
    fn a_short_body_takes_its_own_height_between_two_hairlines() {
        let frame = frame_of(panel(3).body_max_height(320.0), 700.0);
        let rules = rules(&frame);
        assert_eq!(rules.len(), 2);
        // Gap, three 20 px lines, gap: nothing is stretched to the cap.
        assert_eq!(rules[1].y - (rules[0].y + 1.0), 4.0 + 60.0 + 4.0);
        assert_eq!(text_y(&frame, "Foot") - (rules[1].y + 1.0), 4.0);
        assert_eq!(rules[0].y - text_y(&frame, "Head"), 28.0 + 4.0);
    }

    #[test]
    fn a_long_body_stops_at_its_cap_and_the_footer_stays_in_a_short_window() {
        let frame = frame_of(panel(40).body_max_height(320.0), 700.0);
        let rules = rules(&frame);
        assert_eq!(rules[1].y - (rules[0].y + 1.0), 4.0 + 320.0 + 4.0);

        let short = frame_of(panel(40).body_max_height(320.0), 240.0);
        let foot = text_y(&short, "Foot");
        assert!(foot + 32.0 <= 240.0, "the footer is inside the window");
        assert!(text_y(&short, "Head") >= 0.0);
    }

    #[test]
    fn a_bad_cap_is_ignored() {
        let capped = |height: f32| {
            let ElementKind::Column(mut parts) =
                PopoverPanel::new(palette())
                    .body(Element::text("Body"))
                    .body_max_height(height)
                    .render()
                    .kind
            else {
                panic!("the panel is a column");
            };
            parts.remove(0).style.max_height
        };
        assert_eq!(capped(f32::NAN), None);
        assert_eq!(capped(-8.0), Some(0.0));
    }

    use crate::ElementKind;
}
