//! A value dragged along a track: a seek position, a volume, any share of a range.

use crate::{Color, Component, ComponentPalette, CornerRadii, Element};

/// Height of the pointer target. The track is thinner and centered in it.
const HEIGHT: f32 = 16.0;
const TRACK: f32 = 4.0;
const THUMB: f32 = 12.0;

/// The host owns the value: it passes `fraction` on every rebuild and applies
/// each `SliderChanged`. `committed` is false while the pointer is down and
/// true on release and for keyboard steps, so a seek bar can wait for the
/// release while a volume control follows the drag. Without a width the
/// slider fills the available horizontal space.
pub struct Slider {
    id: String,
    fraction: f32,
    palette: ComponentPalette,
    width: Option<f32>,
    steps: usize,
    enabled: bool,
    ink: Option<Color>,
}

impl Slider {
    /// `fraction` runs from 0 to 1; values outside are clamped and NaN is 0.
    pub fn new(id: impl Into<String>, fraction: f32, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            fraction: if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) },
            palette,
            width: None,
            steps: 0,
            enabled: true,
            ink: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(THUMB));
        }
        self
    }

    /// Snap to this many evenly spaced stops, the ends included. Below 2 the
    /// slider is continuous, which is the default.
    pub fn steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    /// A disabled slider shows its value and takes no pointer or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Color of the filled part and the thumb, in place of `palette.text`.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }
}

impl Component for Slider {
    fn render(self) -> Element {
        let ink = match (self.enabled, self.ink) {
            (false, _) => self.palette.muted,
            (true, Some(color)) => color,
            (true, None) => self.palette.text,
        };
        let round = TRACK / 2.0;
        let segment = |share: f32, color: Color, radii: CornerRadii| {
            Element::panel(Element::spacer())
                .background(color)
                .corner_radii(radii)
                .size(None, Some(TRACK))
                .flex(share)
                .centered_y()
        };
        // The thumb sits between the two segments, so its center travels from
        // half a thumb inside one end to half a thumb inside the other: the
        // inset the tree is told about below.
        let mut pieces = Vec::with_capacity(3);
        if self.fraction > 0.0 {
            pieces.push(segment(
                self.fraction,
                ink,
                CornerRadii { top_left: round, bottom_left: round, top_right: 0.0, bottom_right: 0.0 },
            ));
        }
        if self.enabled {
            pieces.push(
                Element::spacer()
                    .size(Some(THUMB), Some(THUMB))
                    .background(ink)
                    .radius(THUMB / 2.0)
                    .centered_y(),
            );
        }
        if self.fraction < 1.0 {
            pieces.push(segment(
                1.0 - self.fraction,
                self.palette.selected,
                CornerRadii { top_left: 0.0, bottom_left: 0.0, top_right: round, bottom_right: round },
            ));
        }
        let track = Element::row(pieces).size(self.width, Some(HEIGHT));
        if !self.enabled {
            return Element::panel(track).size(self.width, Some(HEIGHT));
        }
        Element::clickable(self.id, track)
            .slider(self.fraction, self.steps, THUMB / 2.0)
            .size(self.width, Some(HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Key, Point, Size, UiAction, UiEvent, UiTree};

    const WIDTH: f32 = 212.0;
    const TRAVEL: f32 = WIDTH - THUMB;

    fn tree(slider: Slider) -> UiTree {
        // The root fills the window; a row keeps the slider at its own width.
        let mut tree = UiTree::new(Element::row(vec![slider.render(), Element::spacer().flex(1.0)]));
        tree.layout(Size::new(400.0, HEIGHT));
        tree
    }

    fn thumb_center(fraction: f32) -> f32 {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(Slider::new("s", fraction, palette).width(WIDTH)).layout(Size::new(400.0, HEIGHT));
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillCornerRect { rect, .. } | DrawCommand::FillRect { rect, .. }
                    if (rect.width - THUMB).abs() < 0.01 && (rect.height - THUMB).abs() < 0.01 =>
                {
                    Some(rect.x + rect.width / 2.0)
                }
                _ => None,
            })
            .expect("a thumb is painted")
    }

    fn changed(actions: &[UiAction]) -> (f32, bool) {
        match actions {
            [UiAction::SliderChanged { id, fraction, committed }] if id == "s" => (*fraction, *committed),
            other => panic!("expected one SliderChanged, got {other:?}"),
        }
    }

    #[test]
    fn the_thumb_sits_at_the_fraction_and_bad_values_are_clamped() {
        for (fraction, expected) in [(0.0, 0.0), (0.25, 0.25), (1.0, 1.0), (7.0, 1.0), (-1.0, 0.0), (f32::NAN, 0.0)] {
            let center = thumb_center(fraction);
            assert!(
                (center - (THUMB / 2.0 + expected * TRAVEL)).abs() < 0.6,
                "fraction {fraction}: thumb at {center}"
            );
        }
    }

    #[test]
    fn a_drag_previews_and_the_release_commits() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.0, palette).width(WIDTH));
        let at = |share: f32| Point::new(THUMB / 2.0 + share * TRAVEL, HEIGHT / 2.0);

        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerDown(at(0.5))));
        assert!((fraction - 0.5).abs() < 0.01 && !committed);
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerMove(at(0.8))));
        assert!((fraction - 0.8).abs() < 0.01 && !committed);
        // Past the end of the track the value stops at 1.
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerUp(Point::new(900.0, 2.0))));
        assert!((fraction - 1.0).abs() < 0.001 && committed);
        assert!(tree.handle_event(UiEvent::PointerMove(at(0.2))).is_empty(), "nothing follows the release");
    }

    #[test]
    fn a_plain_slider_leaves_the_window_idle() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.3, palette).width(WIDTH));
        let point = Point::new(100.0, HEIGHT / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point));
        tree.handle_event(UiEvent::PointerLeave);
        tree.set_root(Element::row(vec![
            Slider::new("s", 0.3, palette).width(WIDTH).render(),
            Element::spacer().flex(1.0),
        ]));
        tree.layout(Size::new(400.0, HEIGHT));
        // A player bar is on screen for hours: a control that kept the frame
        // loop running after being touched would never let the app sleep.
        std::thread::sleep(std::time::Duration::from_millis(400));
        tree.layout(Size::new(400.0, HEIGHT));
        assert_eq!(tree.animation_repaint_after(), None);
    }

    #[test]
    fn arrows_home_and_end_move_it_from_the_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.5, palette).width(WIDTH));
        assert!(tree.focus("s"));
        let key = |key| UiEvent::Key { key, shift: false, ctrl: false };
        let (fraction, committed) = changed(&tree.handle_event(key(Key::ArrowRight)));
        assert!((fraction - 0.55).abs() < 0.001 && committed);
        assert!((changed(&tree.handle_event(key(Key::ArrowLeft))).0 - 0.45).abs() < 0.001);
        assert_eq!(changed(&tree.handle_event(key(Key::Home))).0, 0.0);
        assert_eq!(changed(&tree.handle_event(key(Key::End))).0, 1.0);
        assert!(tree.handle_event(key(Key::Enter)).is_empty(), "a slider has no click");
    }

    #[test]
    fn stops_snap_on_release() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.0, palette).width(WIDTH).steps(5));
        let point = Point::new(THUMB / 2.0 + 0.6 * TRAVEL, HEIGHT / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerUp(point)));
        assert!((fraction - 0.5).abs() < 0.001 && committed, "0.6 of five stops settles on 0.5, got {fraction}");
    }

    #[test]
    fn a_disabled_slider_takes_no_pointer_or_focus() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.4, palette).width(WIDTH).enabled(false));
        assert!(tree.layout(Size::new(400.0, HEIGHT)).regions.is_empty());
        assert!(!tree.focus("s"));
        assert!(tree.handle_event(UiEvent::PointerDown(Point::new(60.0, HEIGHT / 2.0))).is_empty());
    }

    #[test]
    fn a_width_narrower_than_the_thumb_is_raised_to_it() {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(Slider::new("s", 0.5, palette).width(2.0)).layout(Size::new(400.0, HEIGHT));
        let region = frame.regions.iter().find(|region| region.id == "s").expect("a region");
        assert!((region.rect.width - THUMB).abs() < 0.01);
    }
}
