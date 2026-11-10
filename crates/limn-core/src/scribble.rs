//! A working indicator: a pen writes a stroke, the stroke fades, and it starts again.

use crate::{Access, Color, Component, CornerRadii, DrawCommand, Element, Point, Rect, Role};
use std::f32::consts::PI;
use std::sync::Arc;
use web_time::Instant;

/// Seconds the pen takes to write every line, shared by their lengths, with an ease-in-out.
const WRITE: f64 = 1.0;
/// Seconds the pen takes to lift from where it is to the next mark.
const HOP: f64 = 0.16;
/// Seconds a drawn pen takes to come down onto the paper, and to lift away at the end.
const PEN_IN: f64 = 0.22;
const PEN_OUT: f64 = 0.32;
/// Seconds the finished marks are held, then faded, before the pen starts again.
const HOLD: f64 = 0.2;
const FADE: f64 = 0.4;
/// Seconds a drawn pen rests between the ink fading and its next stroke.
const REST: f64 = 0.2;
/// Points on the flourish, two loops and a tail, sampled evenly in its parameter.
const POINTS: usize = 48;

/// One thing the pen writes, in logical pixels from the scribble's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// A line drawn as a smooth curve through these points.
    Line(Vec<(f32, f32)>),
    /// A dot the pen taps.
    Dot {
        /// Its center.
        at: (f32, f32),
        /// How wide it is, in logical pixels.
        diameter: f32,
    },
}

/// What a scribble writes, and the pen it is written with, when they are not the defaults;
/// made by [`Scribble::marks`] and [`Scribble::pen`].
#[derive(Debug, Default)]
pub struct Drawing {
    marks: Vec<Mark>,
    pen: Vec<Vec<Point>>,
    /// Written once and kept, rather than faded and written again.
    once: bool,
    /// Seconds the lines take, in place of [`WRITE`].
    write: Option<f64>,
}

/// A working indicator, a pen writing a loopy flourish. Reuse the ID while
/// rebuilding the tree so the stroke does not restart.
pub struct Scribble {
    id: String,
    color: Color,
    width: f32,
    height: f32,
    stroke: f32,
    drawing: Drawing,
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
            drawing: Drawing::default(),
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

    /// Write these marks, in order, in place of the flourish: an app's own signature.
    pub fn marks(mut self, marks: Vec<Mark>) -> Self {
        self.drawing.marks = marks;
        self
    }

    /// Write the marks once and keep them: the pen lifts away and the ink stays, as a
    /// signature or a greeting does. The tree stops animating once the pen is gone.
    pub fn once(mut self) -> Self {
        self.drawing.once = true;
        self
    }

    /// Take this many seconds to write every line, shared by their lengths; 1 by default. A
    /// value that is not finite or not above zero is ignored.
    pub fn duration(mut self, seconds: f64) -> Self {
        if seconds.is_finite() && seconds > 0.0 {
            self.drawing.write = Some(seconds);
        }
        self
    }

    /// Draw the pen that writes: lines in logical pixels with its nib at (0, 0), drawn as
    /// thick as the stroke, as an icon's lines are. It comes down onto the paper, rides the tip, hops
    /// between marks and lifts away before the ink fades; still, it rests at the end. Its
    /// lines may reach past the scribble's box, which does not clip them.
    pub fn pen(mut self, lines: Vec<Vec<(f32, f32)>>) -> Self {
        self.drawing.pen = lines
            .into_iter()
            .map(|line| line.into_iter().map(|(x, y)| Point::new(x, y)).collect())
            .collect();
        self
    }
}

impl Component for Scribble {
    fn render(self) -> Element {
        let said = Access::new(Role::ProgressIndicator).id(self.id.clone()).name("Working");
        let custom = !self.drawing.marks.is_empty()
            || !self.drawing.pen.is_empty()
            || self.drawing.once
            || self.drawing.write.is_some();
        let drawing = custom.then(|| Arc::new(self.drawing));
        Element::scribble_drawing(self.id, self.color, self.stroke, drawing)
            .size(Some(self.width), Some(self.height))
            .accessible(said)
    }
}

/// The flourish in a unit box: a pen writing two cursive "l"s joined, up from a low lead-in
/// and out in a low tail (a prolate cycloid whose loops stand up), stretched to fill it.
fn flourish() -> Vec<(f32, f32)> {
    let (from, to) = (0.6 * PI, 4.6 * PI);
    let raw: Vec<(f32, f32)> = (0..POINTS)
        .map(|i| {
            let theta = from + (to - from) * i as f32 / (POINTS - 1) as f32;
            // A third of a unit of travel per radian against a unit wheel: round, open loops
            // that stand, which stay legible at a small size.
            (0.32 * theta - theta.sin(), -theta.cos())
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

/// A Catmull-Rom curve through `points`, a few samples between each pair.
fn smooth(points: &[Point]) -> Vec<Point> {
    const SAMPLES: usize = 5;
    if points.len() < 3 {
        return points.to_vec();
    }
    let at = |i: isize| points[i.clamp(0, points.len() as isize - 1) as usize];
    let mut out = Vec::with_capacity(points.len() * SAMPLES);
    for i in 0..points.len() as isize - 1 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        for k in 0..SAMPLES {
            let t = k as f32 / SAMPLES as f32;
            let (t2, t3) = (t * t, t * t * t);
            let axis = |a: f32, b: f32, c: f32, d: f32| {
                0.5 * (2.0 * b + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3)
            };
            out.push(Point::new(axis(p0.x, p1.x, p2.x, p3.x), axis(p0.y, p1.y, p2.y, p3.y)));
        }
    }
    out.push(points[points.len() - 1]);
    out
}

fn length(points: &[Point]) -> f32 {
    points.windows(2).map(|pair| (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y)).sum()
}

/// The start of `points` up to `share` of its length, ending exactly there.
fn written(points: &[Point], share: f32) -> Vec<Point> {
    let goal = length(points) * share.clamp(0.0, 1.0);
    let mut out = vec![points[0]];
    let mut run = 0.0;
    for pair in points.windows(2) {
        let step = (pair[1].x - pair[0].x).hypot(pair[1].y - pair[0].y);
        if run + step >= goal {
            let t = if step > 0.0 { (goal - run) / step } else { 0.0 };
            out.push(Point::new(pair[0].x + (pair[1].x - pair[0].x) * t, pair[0].y + (pair[1].y - pair[0].y) * t));
            return out;
        }
        run += step;
        out.push(pair[1]);
    }
    out
}

fn ease_in_out(t: f64) -> f32 {
    (0.5 - 0.5 * (std::f64::consts::PI * t.clamp(0.0, 1.0)).cos()) as f32
}

fn ease_out(t: f64) -> f32 {
    (1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)) as f32
}

/// A mark placed in the frame: a line's curve, or a dot's center and width.
enum Placed {
    Line(Vec<Point>),
    Dot(Point, f32),
}

impl Placed {
    fn start(&self) -> Point {
        match self {
            Placed::Line(points) => points[0],
            Placed::Dot(at, _) => *at,
        }
    }

    fn end(&self) -> Point {
        match self {
            Placed::Line(points) => points[points.len() - 1],
            Placed::Dot(at, _) => *at,
        }
    }
}

/// The commands that paint the marks at `now`, given the time `started` at which the
/// pen began, and whether they still move. Under reduced motion the whole drawing is still,
/// its pen resting at the end.
pub(crate) fn paint(
    rect: Rect,
    color: Color,
    stroke: f32,
    drawing: Option<&Drawing>,
    started: Instant,
    now: Instant,
    reduced_motion: bool,
) -> (Vec<DrawCommand>, bool) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return (Vec::new(), false);
    }
    let custom = drawing.filter(|d| !d.marks.is_empty());
    let placed: Vec<Placed> = match custom {
        Some(drawing) => drawing
            .marks
            .iter()
            .filter_map(|mark| match mark {
                Mark::Line(points) if !points.is_empty() => {
                    let points: Vec<Point> = points.iter().map(|(x, y)| Point::new(rect.x + x, rect.y + y)).collect();
                    Some(Placed::Line(smooth(&points)))
                }
                Mark::Dot { at: (x, y), diameter } => Some(Placed::Dot(Point::new(rect.x + x, rect.y + y), *diameter)),
                Mark::Line(_) => None,
            })
            .collect(),
        None => {
            // Inset by half the stroke so the round caps stay inside the rect.
            let inset = stroke / 2.0;
            let (width, height) = ((rect.width - stroke).max(0.0), (rect.height - stroke).max(0.0));
            vec![Placed::Line(
                flourish().into_iter().map(|(x, y)| Point::new(rect.x + inset + x * width, rect.y + inset + y * height)).collect(),
            )]
        }
    };
    if placed.is_empty() {
        return (Vec::new(), false);
    }
    let pen = drawing.map(|d| d.pen.as_slice()).filter(|pen| !pen.is_empty());

    // The schedule: the pen comes down, writes each line in a share of WRITE by its length,
    // hops to each mark after the first, and lifts away; the ink holds, then fades.
    let write = drawing.and_then(|d| d.write).unwrap_or(WRITE);
    let once = drawing.is_some_and(|d| d.once);
    let total: f32 = placed.iter().map(|m| if let Placed::Line(p) = m { length(p) } else { 0.0 }).sum();
    let arrive = if pen.is_some() { PEN_IN } else { 0.0 };
    let mut spans = Vec::with_capacity(placed.len());
    let mut clock = arrive;
    for (i, mark) in placed.iter().enumerate() {
        let hop = if i > 0 { HOP } else { 0.0 };
        let draw = match mark {
            Placed::Line(points) if total > 0.0 => write * f64::from(length(points) / total),
            Placed::Line(_) => 0.0,
            Placed::Dot(..) if i == 0 => HOP,
            Placed::Dot(..) => 0.0,
        };
        spans.push((clock, clock + hop, clock + hop + draw));
        clock += hop + draw;
    }
    let done = clock;
    let fade_from = done + if pen.is_some() { PEN_OUT.max(HOLD) } else { HOLD };
    let cycle = fade_from + FADE + if pen.is_some() { REST } else { 0.0 };
    let elapsed = now.saturating_duration_since(started).as_secs_f64();
    // Written once, the clock stops where the pen has gone and nothing fades.
    let (t, fade_from) = match (reduced_motion, once) {
        (true, _) => (done, fade_from),
        (false, true) => (elapsed.min(done + PEN_OUT), f64::INFINITY),
        (false, false) => (elapsed % cycle, fade_from),
    };

    // How much of each mark is drawn, where the nib is, how high it is lifted and how
    // opaque the pen and the ink are.
    let alpha = if reduced_motion || t < fade_from { 1.0 } else { (1.0 - (t - fade_from) / FADE).max(0.0) as f32 };
    // A mark not yet begun has a share below 0; a line begun is drawn from its first point.
    let mut shares = vec![-1.0f32; placed.len()];
    let (mut nib, mut lift, mut pen_alpha, mut writing) = (placed[0].start(), 0.0f32, 1.0f32, false);
    if t < arrive {
        let u = ease_out(t / PEN_IN);
        (lift, pen_alpha) = ((1.0 - u) * 3.0, u);
    }
    for (i, (mark, &(from, drawn_from, to))) in placed.iter().zip(&spans).enumerate() {
        if t >= to {
            shares[i] = 1.0;
            nib = mark.end();
            continue;
        }
        if t < from {
            break;
        }
        if t < drawn_from {
            // Lifted between marks, on an arc.
            let u = (t - from) / (drawn_from - from);
            let (a, b, e) = (placed[i - 1].end(), mark.start(), ease_in_out(u));
            nib = Point::new(a.x + (b.x - a.x) * e, a.y + (b.y - a.y) * e);
            lift = (PI * u as f32).sin() * 2.2;
        } else if let Placed::Line(points) = mark {
            shares[i] = ease_in_out((t - drawn_from) / (to - drawn_from));
            nib = written(points, shares[i]).last().copied().unwrap_or(nib);
            writing = true;
        } else {
            nib = mark.start();
        }
        break;
    }
    if t >= done && !reduced_motion {
        let u = ease_out((t - done) / PEN_OUT);
        (lift, pen_alpha) = (u * 4.0, 1.0 - u);
    }

    let ink = Color { a: (f32::from(color.a) * alpha).round() as u8, ..color };
    let disc = |center: Point, diameter: f32, color: Color| DrawCommand::FillCornerRect {
        rect: Rect::new(center.x - diameter / 2.0, center.y - diameter / 2.0, diameter, diameter),
        color,
        radii: CornerRadii::same(diameter / 2.0),
    };
    let mut commands = Vec::new();
    for (mark, share) in placed.iter().zip(&shares) {
        match mark {
            Placed::Line(points) if *share >= 0.0 => {
                let trail = written(points, *share);
                commands.push(DrawCommand::Polyline { points: trail.clone().into(), color: ink, width: stroke, dashed: false });
                // Round caps at both ends.
                commands.push(disc(trail[0], stroke, ink));
                commands.push(disc(trail[trail.len() - 1], stroke, ink));
            }
            Placed::Dot(at, diameter) if *share >= 1.0 => commands.push(disc(*at, *diameter, ink)),
            _ => {}
        }
    }
    match pen {
        Some(lines) if pen_alpha > 0.0 => {
            let color = Color { a: (f32::from(color.a) * pen_alpha).round() as u8, ..color };
            for line in lines {
                let points: Vec<Point> = line.iter().map(|p| Point::new(nib.x + p.x, nib.y - lift + p.y)).collect();
                commands.push(DrawCommand::Polyline { points: smooth(&points).into(), color, width: stroke, dashed: false });
            }
        }
        // Without a pen drawn, its tip leads, a little larger than the line.
        None if writing => commands.push(disc(nib, stroke * 1.6, ink)),
        _ => {}
    }
    let settled = reduced_motion || (once && elapsed >= done + PEN_OUT);
    let moving = !settled;
    (commands, moving)
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
    fn a_drawn_pen_writes_the_marks_hops_to_the_dots_and_rests_at_the_end_when_still() {
        let marks = vec![Mark::Line(vec![(2.0, 10.0), (10.0, 8.0), (18.0, 10.0)]), Mark::Dot { at: (22.0, 8.0), diameter: 3.0 }];
        let pen = vec![vec![(0.0, 0.0), (6.0, -6.0)]];
        let scribble = || Scribble::new("working", Color::WHITE).size(28.0, 12.0).marks(marks.clone()).pen(pen.clone()).render();
        let polylines = |frame: &crate::Frame| frame.commands.iter().filter(|c| matches!(c, DrawCommand::Polyline { .. })).count();
        let dot = |frame: &crate::Frame| {
            frame.commands.iter().any(|c| matches!(c, DrawCommand::FillCornerRect { rect, .. } if (rect.width - 3.0).abs() < 1e-3))
        };
        let mut tree = UiTree::new(scribble());
        let start = Instant::now();
        // Coming down, the pen alone; writing, the line and the pen; once it hopped, the dot.
        tree.layout_at(VIEW, start);
        assert_eq!(polylines(&tree.layout_at(VIEW, start + Duration::from_secs_f64(0.1))), 1);
        let writing = tree.layout_at(VIEW, start + Duration::from_secs_f64(PEN_IN + 0.5));
        assert_eq!(polylines(&writing), 2);
        assert!(!dot(&writing));
        assert!(dot(&tree.layout_at(VIEW, start + Duration::from_secs_f64(PEN_IN + WRITE + HOP + 0.01))));

        let mut still = UiTree::new(scribble());
        still.set_reduced_motion(true);
        let frame = still.layout_at(VIEW, start);
        assert!(dot(&frame) && polylines(&frame) == 2);
    }

    #[test]
    fn written_once_the_ink_stays_the_pen_goes_and_the_tree_rests() {
        let marks = vec![Mark::Line(vec![(2.0, 10.0), (10.0, 8.0), (18.0, 10.0)])];
        let pen = vec![vec![(0.0, 0.0), (6.0, -6.0)]];
        let mut tree = UiTree::new(
            Scribble::new("hello", Color::WHITE).size(28.0, 12.0).marks(marks).pen(pen).once().duration(2.0).render(),
        );
        let start = Instant::now();
        let at = |tree: &mut UiTree, seconds: f64| tree.layout_at(VIEW, start + Duration::from_secs_f64(seconds));
        at(&mut tree, 0.0);
        let polylines = |frame: &crate::Frame| frame.commands.iter().filter(|c| matches!(c, DrawCommand::Polyline { .. })).count();
        // Halfway through its two seconds the pen writes; long after, the ink alone, opaque.
        assert_eq!(polylines(&at(&mut tree, PEN_IN + 1.0)), 2);
        assert!(tree.is_animating());
        let late = at(&mut tree, 30.0);
        assert_eq!(polylines(&late), 1);
        let ink = late.commands.iter().find_map(|c| match c {
            DrawCommand::Polyline { color, .. } => Some(color.a),
            _ => None,
        });
        assert_eq!(ink, Some(255));
        assert!(!tree.is_animating());
        // Kept, it is not written again.
        assert_eq!(polylines(&at(&mut tree, 31.0)), 1);
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
