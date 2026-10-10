//! A raised surface that sets one group of content into the page.

use crate::{Component, ComponentPalette, Element, RadiusRole};

/// How far a [`Card`] holds its content from its edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardDensity {
    /// 8 px, for a card set inside another surface.
    Compact,
    /// 10 px.
    Regular,
    /// 12 px, for a card that stands alone in a reading column.
    Roomy,
}

impl CardDensity {
    pub(crate) const fn padding(self) -> f32 {
        match self {
            Self::Compact => 8.0,
            Self::Regular => 10.0,
            Self::Roomy => 12.0,
        }
    }
}

/// The host builds the content and picks the palette whose `raised` is the
/// fill it wants. The card is passive: it adds the padding, the hairline and
/// the corners, and follows its content's height.
pub struct Card {
    content: Element,
    palette: ComponentPalette,
    density: CardDensity,
    border: bool,
    radius: RadiusRole,
    width: Option<f32>,
}

impl Card {
    /// A bordered card around `content`, with the default density and radius.
    pub fn new(content: Element, palette: ComponentPalette) -> Self {
        Self {
            content,
            palette,
            density: CardDensity::Regular,
            border: true,
            radius: RadiusRole::Card,
            width: None,
        }
    }

    /// The padding around the content ([`CardDensity::Regular`] unless set).
    pub fn density(mut self, density: CardDensity) -> Self {
        self.density = density;
        self
    }

    /// Whether the palette's border outlines the card (it does unless set).
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// The corners ([`RadiusRole::Card`] unless set).
    pub fn radius(mut self, role: RadiusRole) -> Self {
        self.radius = role;
        self
    }

    /// Fix the width, in logical pixels. Without it the card fills the width
    /// it is given. Values that are negative or not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width >= 0.0 {
            self.width = Some(width);
        }
        self
    }
}

impl Component for Card {
    fn render(self) -> Element {
        let mut card = Element::panel(self.content)
            .padding(self.density.padding())
            .background(self.palette.raised)
            .radius(self.radius.px());
        if self.border {
            card = card.border(self.palette.border, 1.0);
        }
        match self.width {
            Some(width) => card.size(Some(width), None),
            None => card,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Rect, Size, UiTree};

    fn frame_of(card: Card) -> crate::Frame {
        let root = Element::row(vec![
            Element::column(vec![card.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]);
        UiTree::new(root).layout(Size::new(400.0, 200.0))
    }

    fn content() -> Element {
        Element::spacer().size(Some(100.0), Some(40.0))
    }

    fn fill(frame: &crate::Frame, palette: ComponentPalette) -> (Rect, f32) {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } if *color == palette.raised => {
                    Some((*rect, *radius))
                }
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn each_density_pads_the_content_and_the_card_is_passive() {
        let palette = ComponentPalette::neutral_dark();
        for (density, padding) in [
            (CardDensity::Compact, 8.0),
            (CardDensity::Regular, 10.0),
            (CardDensity::Roomy, 12.0),
        ] {
            let frame = frame_of(Card::new(content(), palette).density(density));
            let (rect, radius) = fill(&frame, palette);
            assert_eq!(rect, Rect::new(0.0, 0.0, 100.0 + 2.0 * padding, 40.0 + 2.0 * padding));
            assert_eq!(radius, RadiusRole::Card.px());
            assert!(frame.regions.is_empty());
        }
        let default = frame_of(Card::new(content(), palette));
        assert_eq!(default.commands, frame_of(Card::new(content(), palette).density(CardDensity::Regular)).commands);
    }

    #[test]
    fn the_border_can_be_dropped_and_the_radius_follows_its_role() {
        let palette = ComponentPalette::neutral_dark();
        let stroke = |frame: &crate::Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::StrokeRect { color, width, radius, .. } => Some((*color, *width, *radius)),
                _ => None,
            })
        };
        let bordered = frame_of(Card::new(content(), palette));
        assert_eq!(stroke(&bordered), Some((palette.border, 1.0, RadiusRole::Card.px())));
        let plain = frame_of(Card::new(content(), palette).border(false).radius(RadiusRole::Surface));
        assert_eq!(stroke(&plain), None);
        assert_eq!(fill(&plain, palette).1, RadiusRole::Surface.px());
    }

    #[test]
    fn width_is_fixed_and_bad_values_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        let width = |card: Card| fill(&frame_of(card), palette).0.width;
        assert_eq!(width(Card::new(content(), palette).width(240.0)), 240.0);
        for bad in [f32::NAN, f32::INFINITY, -4.0] {
            assert_eq!(width(Card::new(content(), palette).width(bad)), 120.0);
        }
    }
}
