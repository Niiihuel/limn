//! A small dot that says the state of something beside it, and may pulse
//! while that state is live.

use crate::{Access, Color, Component, Element, Role};

// Every pulsing dot keeps its phase under this one key, so dots pulse in step.
const PULSE_ID: &str = "limn:status-dot:pulse";
// One cycle, from full ink to `FADED` and back, eased in and out.
const PULSE_MS: u16 = 1_200;
const FADED: f32 = 0.3;

/// A round mark in a status color, such as green beside a session that is
/// running. The host passes the color and whether it pulses on every
/// rebuild; the dot is passive and reports nothing.
pub struct StatusDot {
    color: Color,
    pulse: bool,
    side: f32,
    id: Option<String>,
    description: Option<String>,
}

impl StatusDot {
    /// A still dot 6 px across, in `color`.
    pub fn new(color: Color) -> Self {
        Self { color, pulse: false, side: 6.0, id: None, description: None }
    }

    /// While `true`, the dot's opacity eases from 1 to 0.3 and back every
    /// 1.2 s; every pulsing dot in a window is in step. Under reduced motion
    /// it stays at full opacity.
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    /// Set the diameter in logical pixels, never below 1 (6 by default). A
    /// value that is not finite is ignored.
    pub fn size(mut self, side: f32) -> Self {
        if side.is_finite() {
            self.side = side.max(1.0);
        }
        self
    }

    /// Make the dot known to the platform by this ID, as a picture named by
    /// its [`description`](Self::description). Without one it is only paint.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// What the dot means, such as "Running": the name the platform is told
    /// for a dot with an [`id`](Self::id).
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }
}

impl Component for StatusDot {
    fn render(self) -> Element {
        let dot = if self.pulse {
            let faded = Color { a: (f32::from(self.color.a) * FADED).round() as u8, ..self.color };
            Element::skeleton(PULSE_ID, self.color, faded).pulse_period(PULSE_MS)
        } else {
            Element::spacer().background(self.color)
        }
        .round()
        .size(Some(self.side), Some(self.side));
        match self.id {
            Some(id) => {
                let said = Access::new(Role::Image).id(id);
                dot.accessible(match self.description {
                    Some(text) => said.name(text),
                    None => said,
                })
            }
            None => dot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};
    use web_time::{Duration, Instant};

    const GREEN: Color = Color::rgb(64, 192, 87);

    fn tree(dot: StatusDot) -> UiTree {
        UiTree::new(Element::row(vec![dot.render(), Element::spacer().flex(1.0)]))
    }

    // The dot's fill at `ms` after it first appeared: its rectangle and color.
    fn fill_at(tree: &mut UiTree, start: Instant, ms: u64) -> (crate::Rect, Color) {
        let frame = tree.layout_at(Size::new(100.0, 20.0), start + Duration::from_millis(ms));
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .expect("a fill")
    }

    #[test]
    fn a_still_dot_is_a_six_pixel_circle_in_its_color() {
        let start = Instant::now();
        let mut still = tree(StatusDot::new(GREEN));
        let (rect, color) = fill_at(&mut still, start, 0);
        assert_eq!((rect.width, rect.height, color), (6.0, 6.0, GREEN));
        assert_eq!(fill_at(&mut still, start, 600).1, GREEN);
        let round = still.layout(Size::new(100.0, 20.0)).commands.iter().any(|command| {
            matches!(command, DrawCommand::FillRect { radius, .. } if *radius == 3.0)
        });
        assert!(round);
        assert!(still.layout(Size::new(100.0, 20.0)).regions.is_empty());
    }

    #[test]
    fn a_pulsing_dot_fades_to_three_tenths_and_back_in_one_point_two_seconds() {
        let start = Instant::now();
        let mut pulsing = tree(StatusDot::new(GREEN).pulse(true));
        assert_eq!(fill_at(&mut pulsing, start, 0).1, GREEN);
        let faded = fill_at(&mut pulsing, start, 600).1;
        assert_eq!((faded.r, faded.g, faded.b, faded.a), (GREEN.r, GREEN.g, GREEN.b, 77));
        let between = fill_at(&mut pulsing, start, 300).1.a;
        assert!(between > 77 && between < 255, "{between}");
        assert_eq!(fill_at(&mut pulsing, start, 1_200).1, GREEN);
        assert!(pulsing.is_animating());
    }

    #[test]
    fn under_reduced_motion_a_pulsing_dot_holds_still_at_full_ink() {
        let start = Instant::now();
        let mut pulsing = tree(StatusDot::new(GREEN).pulse(true));
        pulsing.set_reduced_motion(true);
        for ms in [0, 300, 600, 900] {
            assert_eq!(fill_at(&mut pulsing, start, ms).1, GREEN);
        }
        assert!(!pulsing.is_animating());
    }

    #[test]
    fn size_sets_the_diameter_and_ignores_bad_values() {
        let start = Instant::now();
        let side = |dot: StatusDot| fill_at(&mut tree(dot), start, 0).0.width;
        assert_eq!(side(StatusDot::new(GREEN).size(10.0)), 10.0);
        assert_eq!(side(StatusDot::new(GREEN).size(-4.0)), 1.0);
        assert_eq!(side(StatusDot::new(GREEN).size(f32::NAN)), 6.0);
    }

    #[test]
    fn with_an_id_the_dot_is_a_picture_named_by_its_description() {
        let mut named = tree(StatusDot::new(GREEN).pulse(true).id("session:state").description("Running"));
        let frame = named.layout(Size::new(100.0, 20.0));
        let node = frame.access.iter().find(|node| node.id == "session:state").unwrap();
        assert_eq!((node.role, node.name.as_str()), (Role::Image, "Running"));
        assert!(!node.actionable);
        // Without an ID there is nothing to know it by.
        assert!(tree(StatusDot::new(GREEN).description("Running")).layout(Size::new(100.0, 20.0)).access.is_empty());
    }
}
