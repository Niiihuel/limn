//! A collapsed side panel as a strip at the window's edge: a glimpse of its
//! content behind a veil, which reaches out under the pointer and opens the
//! panel when it is clicked.

use crate::{Access, Color, Component, ComponentPalette, CornerRadii, Element, Icon, Point, Role};

/// Width of the strip at rest, in logical pixels, unless set.
const WIDTH: f32 = 40.0;
/// Width the content is laid out at, unless set: an open panel's.
const CONTENT_WIDTH: f32 = 320.0;
/// How much further the strip reaches out under the pointer.
const REACH: f32 = 12.0;
/// Opacity of the veil over the content at rest, and under the pointer.
const VEIL_AT_REST: f32 = 0.9;
const VEIL_UNDER_POINTER: f32 = 0.8;

/// The edge of the window a side panel stands against.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Side {
    /// The left edge.
    Left,
    /// The right edge.
    #[default]
    Right,
}

/// What is left of a side panel the host has closed: a strip, 40 px wide
/// unless set, on the palette's surface, with the corners that face the
/// window rounded to the `panel` radius. Through a veil almost as opaque as
/// the surface it shows the first pixels of the panel's content, laid out
/// at the panel's whole width. While the pointer is over it, or it has
/// keyboard focus, it reaches 12 px further out on a spring that overshoots
/// a little, the veil lifts a little and its chevron turns to full ink. The
/// host owns whether the panel is open: the strip reports `UiAction::Clicked`
/// with its ID, for a click, Enter or Space.
pub struct PeekPanel {
    id: String,
    content: Element,
    side: Side,
    width: f32,
    content_width: f32,
    label: Option<String>,
    palette: ComponentPalette,
}

impl PeekPanel {
    /// A strip at the right edge over this content, which is only painted:
    /// its controls take no pointer, focus or reader there.
    pub fn new(id: impl Into<String>, content: Element, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            content,
            side: Side::Right,
            width: WIDTH,
            content_width: CONTENT_WIDTH,
            label: None,
            palette,
        }
    }

    /// The edge of the window the strip stands against: the right one
    /// unless set. Its other edge is the one that is rounded and reaches out.
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// The strip's width at rest in logical pixels, 40 unless set. A
    /// negative value is read as zero and one that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }

    /// The width the content is laid out at, in logical pixels: the open
    /// panel's, 320 unless set, so that what shows is how the panel begins.
    /// A negative value is read as zero and one that is not finite is
    /// ignored.
    pub fn content_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.content_width = width.max(0.0);
        }
        self
    }

    /// What the platform calls the strip, such as "Show the queue".
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.label = Some(name.into());
        self
    }
}

impl Component for PeekPanel {
    fn render(self) -> Element {
        let palette = self.palette;
        let radius = crate::tokens().radius.panel;
        let square = CornerRadii::same(0.0);
        let (corners, reach, chevron) = match self.side {
            Side::Right => (CornerRadii { top_left: radius, bottom_left: radius, ..square }, -REACH, Icon::ChevronLeft),
            Side::Left => (CornerRadii { top_right: radius, bottom_right: radius, ..square }, REACH, Icon::ChevronRight),
        };
        let veil = |opacity: f32| {
            let color = Color { a: (f32::from(palette.surface.a) * opacity).round() as u8, ..palette.surface };
            Element::spacer().background(color)
        };
        // The veil under the pointer, and over it what makes up the veil at rest.
        let rest = 1.0 - (1.0 - VEIL_AT_REST) / (1.0 - VEIL_UNDER_POINTER);
        let chevron = |ink: Color| Element::icon(chevron).foreground(ink);
        let glimpse = Element::stack(vec![
            Element::row(vec![self.content.size(Some(self.content_width), None)]).inert(),
            veil(VEIL_UNDER_POINTER),
            veil(rest).conceal_on_hover(format!("{}:veil", self.id)),
            chevron(palette.muted).conceal_on_hover(format!("{}:chevron", self.id)),
            chevron(palette.text).reveal_on_hover(format!("{}:chevron-lit", self.id)),
        ]);
        let mut said = Access::new(Role::Button);
        if let Some(label) = self.label {
            said = said.name(label);
        }
        Element::clickable(self.id.clone(), glimpse)
            .background(palette.surface)
            .corner_radii(corners)
            .clip_children()
            .size(Some(self.width), None)
            .hover_region()
            .push_on_hover(format!("{}:reach", self.id), Point::new(reach, 0.0))
            .accessible(said)
    }
}

#[cfg(test)]
#[path = "peek_panel_tests.rs"]
mod tests;
