//! A working indicator: a pen writes a stroke, the stroke fades, and it starts again.

use crate::{Access, Color, Component, CornerRadii, DrawCommand, Element, Point, Rect, Role};
use std::f32::consts::{PI, TAU};
use web_time::Instant;

/// Seconds the pen takes to write the stroke, with an ease-in-out on its length.
const WRITE: f64 = 1.0;
/// Seconds the finished stroke is held before it fades.
const HOLD_END: f64 = 1.2;
/// Seconds in one cycle: write, hold, then a fade of 0.4 s.
const CYCLE: f64 = 1.6;
/// Points on the flourish, two loops and a tail, sampled evenly in its parameter.
const POINTS: usize = 40;

/// A working indicator, a pen writing a loopy flourish. Reuse the ID while
/// rebuilding the tree so the stroke does not restart.
pub struct Scribble {
    id: String,
    color: Color,
    width: f32,
    height: f32,
    stroke: f32,
}

impl Scribble {
    /// A flourish 28 by 12 logical pixels, 2 pixels thick, drawn in `color`. The platform
    /// knows it by `id` as a progress indicator named "Working"; name it differently
    /// with [`Element::accessibility_label`].
    pub fn new(id: impl Into<String>, color: Color) -> Self {
        Self {
            id: id.into(),
            color,
            width: 28.0,
            height: 12.0,
            stroke: 2.0,
        }
    }

    /// Set the size in logical pixels. A value that is not finite is ignored.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() {
            self.width = width.max(1.0);
            self.height = height.max(1.0);
        }
        self
    }

    /// Set the stroke width in logical pixels. A value that is not finite is ignored.
    pub fn stroke(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.stroke = width.max(0.5);
        }
        self
    }
}

impl Component for Scribble {
    fn render(self) -> Element {
        let said = Access::new(Role::ProgressIndicator).id(self.id.clone()).name("Working");
        Element::scribble(self.id, self.color, self.stroke)
            .size(Some(self.width), Some(self.height))
            .accessible(said)
    }
}

/// The flourish in a unit box: two prolate-cycloid loops, stretched to fill it.
fn flourish() -> Vec<(f32, f32)> {
    let raw: Vec<(f32, f32)> = (0..POINTS)
        .map(|i| {
            let theta = 2.0 * TAU * i as f32 / (POINTS - 1) as f32;
            // The ratio of the two constants makes the loops about three times as wide as tall.
            (0.477 * theta - theta.sin(), -theta.cos())
        })
        .collect();
    let span = |axis: fn(&(f32, f32)) -> f32| {
        let low = raw.iter().map(axis).fold(f32::INFINITY, f32::min);
        let high = raw.iter().map(axis).fold(f32::NEG_INFINITY, f32::max);
        (low, high - low)
    };
    let (min_x, span_x) = span(|p| p.0);
    let (min_y, span_y) = span(|p| p.1);
    raw.iter().map(|(x, y)| ((x - min_x) / span_x, (y - min_y) / span_y)).collect()
}

/// The commands that paint the stroke at `now`, given the time `started` at which the
/// pen began. Under reduced motion the whole stroke is drawn still.
pub(crate) fn paint(
    rect: Rect,
    color: Color,
    stroke: f32,
    started: Instant,
    now: Instant,
    reduced_motion: bool,
) -> Vec<DrawCommand> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return Vec::new();
    }
    // The share of the stroke written, the opacity, and whether the pen is on the paper.
    let (written, alpha, writing) = if reduced_motion {
        (1.0, 1.0, false)
    } else {
        let t = now.saturating_duration_since(started).as_secs_f64() % CYCLE;
        if t < WRITE {
            let eased = 0.5 - 0.5 * (PI as f64 * t / WRITE).cos();
            (eased as f32, 1.0, true)
        } else if t < HOLD_END {
            (1.0, 1.0, false)
        } else {
            (1.0, (1.0 - (t - HOLD_END) / (CYCLE - HOLD_END)) as f32, false)
        }
    };
    // Inset by half the stroke so the round caps stay inside the rect.
    let inset = stroke / 2.0;
    let (width, height) = ((rect.width - stroke).max(0.0), (rect.height - stroke).max(0.0));
    let points: Vec<Point> = flourish()
        .into_iter()
        .map(|(x, y)| Point::new(rect.x + inset + x * width, rect.y + inset + y * height))
        .collect();

    let last = points.len() - 1;
    let at = written * last as f32;
    let whole = (at.floor() as usize).min(last);
    let next = (whole + 1).min(last);
    let share = (at - whole as f32).clamp(0.0, 1.0);
    let tip = Point::new(
        points[whole].x + (points[next].x - points[whole].x) * share,
        points[whole].y + (points[next].y - points[whole].y) * share,
    );
    let mut trail = points[..=whole].to_vec();
    trail.push(tip);

    let ink = Color { a: (f32::from(color.a) * alpha).round() as u8, ..color };
    let dot = |center: Point, diameter: f32| DrawCommand::FillCornerRect {
        rect: Rect::new(center.x - diameter / 2.0, center.y - diameter / 2.0, diameter, diameter),
        color: ink,
        radii: CornerRadii::same(diameter / 2.0),
    };
    let line = DrawCommand::Polyline { points: trail.clone().into(), color: ink, width: stroke, dashed: false };
    let mut commands = vec![line];
    // A dot at each vertex gives round joins and caps.
    commands.extend(trail.iter().map(|point| dot(*point, stroke)));
    if writing {
        // The pen's tip leads, a little larger than the line.
        commands.push(dot(tip, stroke * 1.6));
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, UiTree};
    use web_time::Duration;

    const VIEW: Size = Size::new(28.0, 12.0);

    /// The drawn length of the stroke and its opacity at `at` seconds after the first layout.
    fn stroke_at(tree: &mut UiTree, start: Instant, seconds: f64) -> (f32, u8) {
        let frame = tree.layout_at(VIEW, start + Duration::from_secs_f64(seconds));
        let (points, color) = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Polyline { points, color, .. } => Some((points.clone(), *color)),
                _ => None,
            })
            .expect("a stroke is drawn");
        let length = points.windows(2).map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y)).sum();
        (length, color.a)
    }

    fn scribble_tree() -> UiTree {
        UiTree::new(Scribble::new("working", Color::WHITE).render())
    }

    #[test]
    fn the_pen_writes_the_stroke_then_it_fades_and_starts_again() {
        let mut tree = scribble_tree();
        let start = Instant::now();
        tree.layout_at(VIEW, start);
        let (full, opaque) = stroke_at(&mut tree, start, 1.0);
        assert!(full > 0.0 && opaque == 255);

        let (nothing, _) = stroke_at(&mut tree, start, 0.0);
        assert!(nothing < 1e-3, "nothing is written at the start, {nothing}");
        let (early, _) = stroke_at(&mut tree, start, 0.3);
        let (later, _) = stroke_at(&mut tree, start, 0.6);
        assert!(0.0 < early && early < later && later < full, "{early} {later} {full}");

        let (held, held_alpha) = stroke_at(&mut tree, start, 1.1);
        assert!((held - full).abs() < 1e-3 && held_alpha == 255);
        let (_, fading) = stroke_at(&mut tree, start, 1.3);
        let (_, faded) = stroke_at(&mut tree, start, 1.5);
        assert!(faded < fading && fading < 255, "{fading} {faded}");

        let (again, again_alpha) = stroke_at(&mut tree, start, 1.6 + 0.3);
        assert!((again - early).abs() < 1e-3 && again_alpha == 255);
        assert!(tree.is_animating());
    }

    #[test]
    fn reduced_motion_draws_the_whole_stroke_still() {
        let mut tree = scribble_tree();
        tree.set_reduced_motion(true);
        let start = Instant::now();
        let (full, alpha) = stroke_at(&mut tree, start, 0.0);
        assert_eq!(alpha, 255);
        assert_eq!(stroke_at(&mut tree, start, 0.5), (full, alpha));
        assert_eq!(stroke_at(&mut tree, start, 1.4), (full, alpha));
        assert!(!tree.is_animating());
    }

    #[test]
    fn scribble_is_a_progress_indicator_named_working() {
        let access = UiTree::new(Scribble::new("working", Color::WHITE).render())
            .layout(VIEW)
            .access;
        let node = &access[0];
        assert_eq!((node.id.as_str(), node.role, node.name.as_str()), ("working", Role::ProgressIndicator, "Working"));
    }
}
