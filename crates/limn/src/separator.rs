//! A one-pixel rule between two regions.

use crate::{Color, Component, ComponentPalette, Element};

/// A passive hairline in the palette's border color. Without a length it
/// fills the axis it runs along, so it spans whatever container holds it.
pub struct Separator {
    vertical: bool,
    color: Color,
    length: Option<f32>,
    margin: f32,
    centered_in: Option<f32>,
}

impl Separator {
    /// A rule one pixel tall, for stacking between rows.
    pub fn horizontal(palette: ComponentPalette) -> Self {
        Self::new(false, palette)
    }

    /// A rule one pixel wide, for standing between columns or tools.
    pub fn vertical(palette: ComponentPalette) -> Self {
        Self::new(true, palette)
    }

    fn new(vertical: bool, palette: ComponentPalette) -> Self {
        Self {
            vertical,
            color: palette.border,
            length: None,
            margin: 0.0,
            centered_in: None,
        }
    }

    /// Fix the rule's length instead of filling the container. A negative
    /// length is none at all; one that is not finite is ignored.
    pub fn length(mut self, length: f32) -> Self {
        if length.is_finite() {
            self.length = Some(length.max(0.0));
        }
        self
    }

    /// Empty space on both sides, across the rule. The default is none.
    /// Values that are not finite are ignored.
    pub fn margin(mut self, margin: f32) -> Self {
        if margin.is_finite() {
            self.margin = margin.max(0.0);
        }
        self
    }

    /// Center a vertical rule in a slot of this height, as between the tools
    /// of a bar taller than the rule. A horizontal rule ignores it, as are
    /// values that are not finite.
    pub fn centered_in(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.centered_in = Some(height.max(0.0));
        }
        self
    }

    /// Paint the rule in a color the host owns, such as a tree guide,
    /// instead of the palette's border.
    pub fn ink(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Component for Separator {
    fn render(self) -> Element {
        let line = Element::spacer().background(self.color);
        if !self.vertical {
            let line = line.size(self.length, Some(1.0));
            return if self.margin > 0.0 {
                Element::column(vec![
                    Element::spacer().size(None, Some(self.margin)),
                    line,
                    Element::spacer().size(None, Some(self.margin)),
                ])
            } else {
                line
            };
        }
        let mut line = line.size(Some(1.0), self.length);
        if let Some(height) = self.centered_in {
            line = line.centered_y().size(Some(1.0), Some(height));
        }
        if self.margin > 0.0 {
            Element::row(vec![
                Element::spacer().size(Some(self.margin), None),
                line,
                Element::spacer().size(Some(self.margin), None),
            ])
        } else {
            line
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Rect, Size, UiTree};

    fn rule(element: Element, color: Color, size: Size) -> Rect {
        let frame = UiTree::new(element).layout(size);
        assert!(frame.regions.is_empty(), "a separator is passive");
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
                _ => None,
            })
            .expect("the rule is painted")
    }

    #[test]
    fn a_rule_is_one_pixel_and_fills_its_axis_unless_given_a_length() {
        let palette = ComponentPalette::neutral_dark();
        for width in [180.0, 900.0] {
            let column = |separator: Separator| {
                Element::column(vec![separator.render(), Element::spacer().flex(1.0)])
            };
            let size = Size::new(width, 40.0);
            let full = rule(column(Separator::horizontal(palette)), palette.border, size);
            assert_eq!((full.x, full.width, full.height), (0.0, width, 1.0));
            let fixed = rule(column(Separator::horizontal(palette).length(64.0)), palette.border, size);
            assert_eq!((fixed.width, fixed.height), (64.0, 1.0));
            let padded = rule(column(Separator::horizontal(palette).margin(6.0)), palette.border, size);
            assert_eq!((padded.y, padded.height), (6.0, 1.0));

            let row = |separator: Separator| {
                Element::row(vec![separator.render(), Element::spacer().flex(1.0)])
            };
            let tall = rule(row(Separator::vertical(palette)), palette.border, size);
            assert_eq!((tall.x, tall.width, tall.height), (0.0, 1.0, 40.0));
            // Margin and centering are off unless asked for: a bare rule adds one pixel.
            let bare = rule(row(Separator::vertical(palette).length(16.0)), palette.border, size);
            assert_eq!((bare.x, bare.width, bare.height), (0.0, 1.0, 16.0));
        }
    }

    #[test]
    fn a_vertical_rule_centers_in_its_slot_between_its_margins() {
        let palette = ComponentPalette::neutral_dark();
        let ink = Color::rgb(72, 72, 72);
        let separator = Separator::vertical(palette)
            .length(14.0)
            .centered_in(24.0)
            .margin(4.0)
            .ink(ink)
            .render();
        let root = Element::column(vec![
            Element::row(vec![separator, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]);
        let line = rule(root, ink, Size::new(200.0, 80.0));
        assert_eq!((line.x, line.y, line.width, line.height), (4.0, 5.0, 1.0, 14.0));
    }

    #[test]
    fn values_that_cannot_be_drawn_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        for bad in [f32::NAN, f32::INFINITY] {
            let plain = format!("{:?}", Separator::vertical(palette).render());
            let built = Separator::vertical(palette)
                .length(bad)
                .margin(bad)
                .centered_in(bad)
                .render();
            assert_eq!(format!("{built:?}"), plain);
        }
        // A negative length is clamped, as Element::size does.
        assert_eq!(Separator::horizontal(palette).length(-4.0).render().style.width, Some(0.0));
        assert_eq!(
            format!("{:?}", Separator::vertical(palette).margin(-4.0).render()),
            format!("{:?}", Separator::vertical(palette).render()),
        );
        // A horizontal rule has no slot to be centered in.
        assert_eq!(
            format!("{:?}", Separator::horizontal(palette).centered_in(24.0).render()),
            format!("{:?}", Separator::horizontal(palette).render()),
        );
    }
}
