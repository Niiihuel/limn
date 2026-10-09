//! A scroll surface whose content, scrollbar layout and scrollbar colors are
//! configured independently. The same appearance is used by ordinary scroll
//! content and lazily built uniform lists.

use crate::{Color, Component, Element, ScrollMotion};

/// Where the scrollbar sits relative to scroll content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollbarPlacement {
    /// Paint over the content without reducing its layout width.
    Overlay,
    /// Reserve a separate rail for content with controls at its far edge.
    #[default]
    Reserved,
}

/// Visual and pointer policy for a vertical scrollbar, in logical pixels.
///
/// The default reserves a separate rail while content overflows, keeping
/// scrollbar arrows and pointer targets clear of row controls and hovers.
/// The idle track is transparent. An explicit overlay can keep the content
/// width unchanged when the surface leaves room for the scrollbar itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarAppearance {
    pub placement: ScrollbarPlacement,
    /// Show compact up/down controls at the ends of an overflowing rail.
    pub buttons: bool,
    pub rail_width: f32,
    pub thumb_width: f32,
    pub min_thumb_length: f32,
    /// `None` uses the scroll element's foreground color, so a theme can tint
    /// the scrollbar through its ordinary typography token.
    pub tint: Option<Color>,
    pub idle_thumb_opacity: u8,
    pub hover_thumb_opacity: u8,
    pub active_thumb_opacity: u8,
    pub hover_track_opacity: u8,
}

impl Default for ScrollbarAppearance {
    fn default() -> Self {
        Self {
            placement: ScrollbarPlacement::Reserved,
            buttons: true,
            rail_width: 14.0,
            thumb_width: 5.0,
            min_thumb_length: 24.0,
            tint: None,
            idle_thumb_opacity: 72,
            hover_thumb_opacity: 118,
            active_thumb_opacity: 164,
            hover_track_opacity: 14,
        }
    }
}

impl ScrollbarAppearance {
    pub fn overlay() -> Self {
        Self {
            placement: ScrollbarPlacement::Overlay,
            ..Self::default()
        }
    }

    pub fn reserved() -> Self {
        Self::default()
    }

    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn buttons(mut self, enabled: bool) -> Self {
        self.buttons = enabled;
        self
    }

    pub fn rail_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.rail_width = width.clamp(4.0, 24.0);
        }
        self
    }

    pub fn thumb_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.thumb_width = width.clamp(2.0, 12.0);
        }
        self
    }

    /// Shared scrollbar ink for native editors and ordinary scroll surfaces.
    pub(crate) fn thumb_color(self, foreground: Color, hovered: bool, active: bool) -> Color {
        self.color_with_opacity(
            foreground,
            if active {
                self.active_thumb_opacity
            } else if hovered {
                self.hover_thumb_opacity
            } else {
                self.idle_thumb_opacity
            },
        )
    }

    pub(crate) fn track_color(self, foreground: Color) -> Color {
        self.color_with_opacity(foreground, self.hover_track_opacity)
    }

    fn color_with_opacity(self, foreground: Color, opacity: u8) -> Color {
        let ink = self.tint.unwrap_or(foreground);
        Color::rgba(
            ink.r,
            ink.g,
            ink.b,
            ((u16::from(ink.a) * u16::from(opacity)) / 255) as u8,
        )
    }

    pub(crate) fn effective_rail_width(self, viewport_width: f32) -> f32 {
        let width = if self.rail_width.is_finite() {
            self.rail_width
        } else {
            Self::default().rail_width
        };
        width.clamp(1.0, 24.0).min(viewport_width / 3.0)
    }

    pub(crate) fn content_width(self, viewport_width: f32, overflowing: bool) -> f32 {
        if overflowing && self.placement == ScrollbarPlacement::Reserved {
            viewport_width - self.effective_rail_width(viewport_width)
        } else {
            viewport_width
        }
    }
}

/// Reusable wrapper for a normal or virtualized scroll surface. It preserves
/// the stable element ID used for retained scroll position.
pub struct ScrollArea {
    element: Element,
}

impl ScrollArea {
    pub fn new(id: impl Into<String>, child: Element) -> Self {
        Self {
            element: Element::scroll(id, child),
        }
    }

    pub fn uniform_list(
        id: impl Into<String>,
        row_count: usize,
        row_height: f32,
        build_row: impl Fn(usize) -> Element + Send + Sync + 'static,
    ) -> Self {
        Self {
            element: Element::uniform_list(id, row_count, row_height, build_row),
        }
    }

    pub fn appearance(mut self, appearance: ScrollbarAppearance) -> Self {
        self.element = self.element.scrollbar_appearance(appearance);
        self
    }

    pub fn motion(mut self, motion: ScrollMotion) -> Self {
        self.element = self.element.scroll_motion(motion);
        self
    }

    pub fn buttons(mut self, enabled: bool) -> Self {
        match &mut self.element.kind {
            crate::ElementKind::Scroll { appearance, .. } => appearance.buttons = enabled,
            crate::ElementKind::UniformList(list) => list.appearance.buttons = enabled,
            _ => unreachable!("ScrollArea always contains a scroll element"),
        }
        self
    }

    pub fn tint(mut self, color: Color) -> Self {
        match &mut self.element.kind {
            crate::ElementKind::Scroll { appearance, .. } => appearance.tint = Some(color),
            crate::ElementKind::UniformList(list) => list.appearance.tint = Some(color),
            _ => unreachable!("ScrollArea always contains a scroll element"),
        }
        self
    }
}

impl Component for ScrollArea {
    fn render(self) -> Element {
        self.element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Easing, ElementKind};

    fn policies(element: &Element) -> (ScrollbarAppearance, ScrollMotion) {
        match &element.kind {
            ElementKind::Scroll {
                appearance, motion, ..
            } => (*appearance, *motion),
            ElementKind::UniformList(list) => (list.appearance, list.motion),
            _ => panic!("expected a scroll surface"),
        }
    }

    #[test]
    fn appearance_applies_to_both_scroll_content_types() {
        let appearance = ScrollbarAppearance::reserved().tint(Color::rgb(1, 2, 3));
        let ordinary = ScrollArea::new("ordinary", Element::text("content"))
            .appearance(appearance)
            .render();
        let virtualized = ScrollArea::uniform_list("virtual", 2, 24.0, |_| Element::text("row"))
            .appearance(appearance)
            .render();
        assert!(
            matches!(ordinary.kind, ElementKind::Scroll { appearance: a, .. } if a == appearance)
        );
        assert!(
            matches!(virtualized.kind, ElementKind::UniformList(list) if list.appearance == appearance)
        );
    }

    #[test]
    fn overlay_keeps_content_width_when_overflow_changes() {
        let overlay = ScrollbarAppearance::overlay();
        let reserved = ScrollbarAppearance::reserved();
        assert_eq!(overlay.content_width(100.0, true), 100.0);
        assert_eq!(reserved.content_width(100.0, true), 86.0);
        assert_eq!(reserved.content_width(100.0, false), 100.0);
    }

    #[test]
    fn tint_does_not_replace_a_previous_layout_policy() {
        let element = ScrollArea::new("list", Element::text("content"))
            .appearance(ScrollbarAppearance::reserved())
            .tint(Color::rgb(1, 2, 3))
            .render();
        assert!(
            matches!(element.kind, ElementKind::Scroll { appearance, .. }
            if appearance.placement == ScrollbarPlacement::Reserved
                && appearance.tint == Some(Color::rgb(1, 2, 3)))
        );
    }

    #[test]
    fn motion_and_button_overrides_preserve_each_other_on_both_content_types() {
        let tint = Color::rgb(1, 2, 3);
        let appearance = ScrollbarAppearance::reserved().tint(tint);
        let motion = ScrollMotion {
            duration_ms: 210,
            easing: Easing::InOutCubic,
        };
        let ordinary = ScrollArea::new("ordinary", Element::text("content"))
            .appearance(appearance)
            .motion(motion)
            .buttons(false)
            .render();
        let virtualized = ScrollArea::uniform_list("virtual", 2, 24.0, |_| Element::text("row"))
            .appearance(appearance)
            .buttons(false)
            .motion(motion)
            .render();
        let expected = (appearance.buttons(false), motion);
        assert_eq!(policies(&ordinary), expected);
        assert_eq!(policies(&virtualized), expected);
    }

    #[test]
    fn low_level_elements_can_disable_motion_without_replacing_appearance() {
        let appearance = ScrollbarAppearance::overlay()
            .buttons(false)
            .rail_width(18.0);
        let motion = ScrollMotion {
            duration_ms: 0,
            ..ScrollMotion::default()
        };
        for surface in [
            Element::scroll("ordinary", Element::text("content")),
            Element::uniform_list("virtual", 3, 24.0, |_| Element::text("row")),
        ] {
            let element = surface
                .scroll_motion(motion)
                .scrollbar_appearance(appearance);
            assert_eq!(policies(&element), (appearance, motion));
        }
    }
}
