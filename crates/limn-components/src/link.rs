//! A text action that hugs its label.

use crate::{Access, Color, Component, ComponentPalette, Element, FontFamily, Role};

/// A text action that hugs its label: link ink, underlined under the pointer
/// and keyboard focus, with no hover fill. Emits `UiAction::Clicked`.
pub struct Link {
    action_id: String,
    label: String,
    color: Color,
    muted: Color,
    font_size: f32,
    enabled: bool,
}

impl Link {
    /// A text action that reports `Clicked` with this ID.
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            color: palette.link(),
            muted: palette.muted,
            font_size: 13.0,
            enabled: true,
        }
    }

    /// Draw the link in this ink, in place of the palette's link ink.
    pub fn ink(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The former name of [`ink`](Self::ink).
    #[deprecated(since = "0.1.39", note = "renamed to `ink`, the name every other component uses")]
    pub fn color(self, color: Color) -> Self {
        self.ink(color)
    }

    /// Set the size of the text. A size that is not finite or not above
    /// zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size;
        }
        self
    }

    /// A disabled link is passive text: its label in the palette's muted, in
    /// place of any `color`, with no underline, hit region or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Link {
    fn render(self) -> Element {
        if !self.enabled {
            // The engine's Button element always registers a hit target, so
            // a disabled link is its label alone, on one line as a button's.
            let access = Access::new(Role::Link).id(self.action_id).name(self.label.clone()).disabled(true);
            return Element::truncated_text(self.label)
                .foreground(self.muted)
                .font_family(FontFamily::Sans)
                .font_size(self.font_size)
                .accessible(access);
        }
        Element::button(self.action_id, self.label)
            .foreground(self.color)
            .font_family(FontFamily::Sans)
            .font_size(self.font_size)
            .padding(0.0)
            .underline_on_hover()
            .accessible(Access::new(Role::Link))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, Point, Size, UiEvent, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    #[test]
    fn a_link_is_a_link_named_by_its_label_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let said = access_node(Link::new("docs", "Read the docs", palette).enabled(enabled).render(), "docs");
            assert_eq!((said.role, said.name.as_str()), (Role::Link, "Read the docs"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
    }

    #[test]
    fn a_disabled_link_is_muted_text_with_no_target_in_the_place_of_the_enabled_one() {
        let palette = ComponentPalette::neutral_dark();
        let tree_of = |enabled| {
            let link = Link::new("docs", "Read the documentation", palette)
                .ink(palette.text)
                .font_size(11.0)
                .enabled(enabled);
            let mut tree = UiTree::new(Element::column(vec![
                Element::row(vec![link.render(), Element::text("after")]).gap(4.0),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            tree
        };
        let texts = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, color, .. } => {
                        Some((text.clone(), *position, *color))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let size = Size::new(400.0, 80.0);
        let mut tree = tree_of(true);
        let enabled = tree.layout(size);
        let [region] = enabled.regions.as_slice() else {
            panic!("one region, got {:?}", enabled.regions);
        };
        let point = Point::new(region.rect.x + 4.0, region.rect.y + 4.0);

        let mut tree = tree_of(false);
        let disabled = tree.layout(size);
        assert!(disabled.regions.is_empty());
        tree.handle_event(UiEvent::PointerMove(point));
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(!tree.focus("docs"));
        let enter = UiEvent::Key { key: crate::Key::Enter, shift: false, ctrl: false, alt: false };
        assert!(tree.handle_event(enter).is_empty());
        // Hovered, it is still the same muted line with no underline.
        assert_eq!(tree.layout(size).commands, disabled.commands);
        let (was, now) = (texts(&enabled), texts(&disabled));
        assert_eq!((was[0].2, now[0].2), (palette.text, palette.muted));
        // The label and what follows it stay where they were.
        let places = |lines: &[(String, Point, Color)]| {
            lines.iter().map(|line| (line.0.clone(), line.1)).collect::<Vec<_>>()
        };
        assert_eq!(places(&now), places(&was));
    }
}
