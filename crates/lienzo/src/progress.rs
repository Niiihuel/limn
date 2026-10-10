//! A thin bar for work whose completed share is known.

use crate::{Component, ComponentPalette, CornerRadii, Element};

/// The host owns the value and passes it on every rebuild. Without a width
/// the bar fills the available horizontal space.
pub struct Progress {
    fraction: f32,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
}

impl Progress {
    /// `fraction` runs from 0 to 1; values outside are clamped and NaN is empty.
    pub fn new(fraction: f32, palette: ComponentPalette) -> Self {
        Self {
            fraction: if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) },
            palette,
            width: None,
            height: 4.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(1.0);
        }
        self
    }
}

impl Component for Progress {
    fn render(self) -> Element {
        let round = self.height / 2.0;
        // The fill ends flat against the remaining track until it is complete.
        let end = if self.fraction >= 1.0 { round } else { 0.0 };
        let fill = Element::panel(Element::spacer())
            .background(self.palette.text)
            .corner_radii(CornerRadii {
                top_left: round,
                top_right: end,
                bottom_right: end,
                bottom_left: round,
            })
            .size(None, Some(self.height))
            .flex(self.fraction);
        Element::panel(Element::row(vec![
            fill,
            Element::spacer().flex(1.0 - self.fraction),
        ]))
        .background(self.palette.selected)
        .radius(round)
        .size(self.width, Some(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};

    fn fill_width(fraction: f32) -> f32 {
        let palette = ComponentPalette::neutral_dark();
        let bar = Progress::new(fraction, palette).width(200.0).render();
        // The root fills the window; a row keeps the bar at its own width.
        let root = Element::row(vec![bar, Element::spacer().flex(1.0)]);
        let frame = UiTree::new(root).layout(Size::new(400.0, 40.0));
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillCornerRect { rect, color, .. } | DrawCommand::FillRect { rect, color, .. }
                    if *color == palette.text =>
                {
                    Some(rect.width)
                }
                _ => None,
            })
            .unwrap_or(0.0)
    }

    #[test]
    fn fill_covers_its_share_of_the_track_and_clamps_bad_values() {
        assert!((fill_width(0.25) - 50.0).abs() < 0.5);
        assert!((fill_width(1.0) - 200.0).abs() < 0.5);
        assert!(fill_width(0.0) < 0.5);
        assert!((fill_width(7.0) - 200.0).abs() < 0.5);
        assert!(fill_width(-1.0) < 0.5);
        assert!(fill_width(f32::NAN) < 0.5);
    }
}
