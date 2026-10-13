//! A thin bar for work whose completed share is known.

use crate::{Access, Component, ComponentPalette, CornerRadii, Element, Role};

/// The host owns the value and passes it on every rebuild. Without a width
/// the bar fills the available horizontal space.
pub struct Progress {
    id: Option<String>,
    fraction: f32,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
    flat: bool,
}

impl Progress {
    /// `fraction` runs from 0 to 1; values outside are clamped and NaN is empty.
    pub fn new(fraction: f32, palette: ComponentPalette) -> Self {
        Self {
            id: None,
            fraction: if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) },
            palette,
            width: None,
            height: 4.0,
            flat: false,
        }
    }

    /// Make the bar known to the platform by this ID, as a progress
    /// indicator with its value. Without one it is only paint. Name it on
    /// what it renders, with [`Element::accessibility_label`].
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a
    /// negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Set the bar's thickness in logical pixels, never below 1 (4 by default). A value
    /// that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(1.0);
        }
        self
    }

    /// Square ends, with the fill and the rest side by side as two pieces
    /// instead of a fill over a track: the meter of a dense list, where a
    /// rounded cap would read as a control. The fill takes
    /// `palette.text` and the rest `palette.selected`, so a host passes a
    /// status color by naming it in the palette.
    pub fn flat(mut self) -> Self {
        self.flat = true;
        self
    }
}

impl Component for Progress {
    fn render(self) -> Element {
        let said = self.id.map(|id| Access::new(Role::ProgressIndicator).id(id).value(self.fraction));
        let say = |bar: Element| match said {
            Some(said) => bar.accessible(said),
            None => bar,
        };
        if self.flat {
            let mut pieces = Vec::with_capacity(2);
            if self.fraction > 0.0 {
                pieces.push(Element::spacer().flex(self.fraction).background(self.palette.text));
            }
            if self.fraction < 1.0 {
                pieces.push(
                    Element::spacer()
                        .flex(1.0 - self.fraction)
                        .background(self.palette.selected),
                );
            }
            // Clipped so a rounding error in the split cannot paint past the bar.
            return say(Element::row(pieces)
                .size(self.width, Some(self.height))
                .clip_children());
        }
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
        say(Element::panel(Element::row(vec![
            fill,
            Element::spacer().flex(1.0 - self.fraction),
        ]))
        .background(self.palette.selected)
        .radius(round)
        .size(self.width, Some(self.height)))
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

    #[test]
    fn a_flat_bar_is_two_square_pieces_side_by_side() {
        let palette = ComponentPalette::neutral_dark();
        let pieces = |fraction: f32| {
            let bar = Progress::new(fraction, palette).width(200.0).flat().render();
            let root = Element::row(vec![bar, Element::spacer().flex(1.0)]);
            let frame = UiTree::new(root).layout(Size::new(400.0, 40.0));
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } => Some((rect.x, rect.width, *color)),
                    DrawCommand::FillCornerRect { .. } => panic!("a flat bar has no rounded corner"),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(pieces(0.25), vec![(0.0, 50.0, palette.text), (50.0, 150.0, palette.selected)]);
        assert_eq!(pieces(0.0), vec![(0.0, 200.0, palette.selected)]);
        assert_eq!(pieces(1.0), vec![(0.0, 200.0, palette.text)]);
        assert_eq!(pieces(f32::NAN), pieces(0.0));
    }

    #[test]
    fn a_bar_with_an_id_is_a_progress_indicator_with_its_value_and_the_name_it_is_given() {
        let palette = ComponentPalette::neutral_dark();
        for flat in [false, true] {
            let bar = Progress::new(0.25, palette).id("upload").width(200.0);
            let bar = if flat { bar.flat() } else { bar };
            let root = Element::row(vec![bar.render().accessibility_label("Uploading")]);
            let frame = UiTree::new(root).layout(Size::new(400.0, 40.0));
            let node = frame.access.iter().find(|node| node.id == "upload").unwrap();
            assert_eq!((node.role, node.name.as_str(), node.value), (Role::ProgressIndicator, "Uploading", Some(0.25)));
            assert!(!node.actionable && node.rect.width == 200.0);
        }
        // Without an ID there is nothing to know it by.
        let frame = UiTree::new(Progress::new(0.25, palette).render()).layout(Size::new(400.0, 40.0));
        assert!(frame.access.is_empty());
    }
}
