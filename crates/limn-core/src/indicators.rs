//! How a progress ring and a spinner are drawn, from the commands every
//! backend paints: polylines for arcs and spokes, and fully rounded squares
//! for dots and for the round ends a polyline does not have.

use crate::{Color, DrawCommand, Point, Rect, SpinnerVariant};
use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::Arc;

/// Width of a progress ring's circle and arc, in logical pixels.
const RING_WIDTH: f32 = 1.7;

/// `segments` chords along the circle of `radius` around `center`, from
/// `start` radians through `sweep`. 0 is three o'clock, and a positive sweep
/// runs clockwise, since y grows downwards.
fn arc(center: Point, radius: f32, start: f32, sweep: f32, segments: usize) -> Arc<[Point]> {
    (0..=segments)
        .map(|step| {
            let theta = start + sweep * step as f32 / segments as f32;
            Point::new(center.x + radius * theta.cos(), center.y + radius * theta.sin())
        })
        .collect()
}

fn line(points: Arc<[Point]>, color: Color, width: f32) -> DrawCommand {
    DrawCommand::Polyline { points, color, width, dashed: false }
}

/// A disc of `diameter` around `center`: a square rounded to a circle.
fn disc(center: Point, diameter: f32, color: Color) -> DrawCommand {
    DrawCommand::FillRect {
        rect: Rect::new(center.x - diameter / 2.0, center.y - diameter / 2.0, diameter, diameter),
        color,
        radius: diameter / 2.0,
    }
}

/// A polyline with a disc on each end: the round ends a polyline does not
/// draw.
fn rounded_line(commands: &mut Vec<DrawCommand>, points: Arc<[Point]>, color: Color, width: f32) {
    let ends = [points[0], points[points.len() - 1]];
    commands.push(line(points, color, width));
    commands.extend(ends.map(|end| disc(end, width, color)));
}

/// A spinner in `rect` at `angle` radians, centered in the shorter side.
/// `Circle` is five sixths of a circle from twelve o'clock clockwise, turned
/// by `angle`; `Spokes` eight strokes from about half the radius to its end,
/// the first at `angle` from three o'clock; both have round ends and a stroke
/// 12% of the side, from 1.5 to 2.8 pixels, a pixel inside the side.
/// `Dots` reads `angle` as its place in the cycle; see `dots`. A side that
/// is not above 0, or not finite, draws nothing.
pub(crate) fn spinner(commands: &mut Vec<DrawCommand>, rect: Rect, color: Color, angle: f32, variant: SpinnerVariant) {
    let side = rect.width.min(rect.height);
    if !(rect.width.is_finite() && rect.height.is_finite()) || side <= 0.0 {
        return;
    }
    let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let width = (side * 0.12).clamp(1.5, 2.8);
    let reach = (side / 2.0 - width / 2.0 - 1.0).max(0.0);
    match variant {
        SpinnerVariant::Circle => {
            // A chord about every 1.4 pixels of the arc.
            let sweep = TAU * 5.0 / 6.0;
            let segments = ((reach * sweep / 1.4).ceil() as usize).clamp(16, 80);
            rounded_line(commands, arc(center, reach, angle - FRAC_PI_2, sweep, segments), color, width);
        }
        SpinnerVariant::Spokes => {
            for index in 0..8 {
                let theta = angle + TAU * index as f32 / 8.0;
                let at = |distance: f32| Point::new(center.x + theta.cos() * distance, center.y + theta.sin() * distance);
                rounded_line(commands, Arc::new([at(reach * 0.55), at(reach)]), color, width);
            }
        }
        SpinnerVariant::Dots => {
            for dot in dots(angle, side) {
                commands.push(disc(Point::new(center.x + dot.x, center.y + dot.y), side * 0.22, color));
            }
        }
    }
}

/// The centers of a `Dots` spinner's four dots, from the middle of a side
/// of `side`. `angle` runs a full turn per cycle: in the first half two
/// opposite dots slide onto the diagonal of the other two and back, in the
/// second half the other two.
fn dots(angle: f32, side: f32) -> [Point; 4] {
    let half = side * 0.31;
    let turn = (angle / TAU).rem_euclid(1.0) * 2.0;
    let phase = turn.fract();
    let smooth = |t: f32| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    // Out over a fifth of the half, held, then back; at rest the rest of it.
    let reach = smooth(phase / 0.2) - smooth((phase - 0.35) / 0.2);
    let corner = |x: f32, y: f32| Point::new(x * half, y * half);
    let toward = |from: Point, to: Point| Point::new(from.x + (to.x - from.x) * reach, from.y + (to.y - from.y) * reach);
    let third = 1.0 / 3.0;
    if turn < 1.0 {
        // "/": the top right and bottom left stay; the others slide inward.
        [
            corner(1.0, -1.0),
            corner(-1.0, 1.0),
            toward(corner(-1.0, -1.0), corner(third, -third)),
            toward(corner(1.0, 1.0), corner(-third, third)),
        ]
    } else {
        // "\": the top left and bottom right stay.
        [
            corner(-1.0, -1.0),
            corner(1.0, 1.0),
            toward(corner(1.0, -1.0), corner(-third, -third)),
            toward(corner(-1.0, 1.0), corner(third, third)),
        ]
    }
}

/// A progress ring in `rect`: a circle of `track`, and over it an arc of
/// `fill` for `fraction` of a turn, from twelve o'clock clockwise. Both are
/// centered, a pixel inside the shorter side. A fraction that is `None`, not
/// finite or not above 0 draws the circle alone; a side of 2 or less, or
/// one that is not finite, draws nothing.
pub(crate) fn progress_ring(commands: &mut Vec<DrawCommand>, rect: Rect, fraction: Option<f32>, track: Color, fill: Color) {
    let side = rect.width.min(rect.height);
    if !(rect.width.is_finite() && rect.height.is_finite()) || side <= 2.0 {
        return;
    }
    let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    // A whole turn is 64 chords, an arc its share of them.
    let turn = |share: f32| arc(center, side / 2.0 - 1.0, -FRAC_PI_2, TAU * share, (share * 64.0).ceil().max(2.0) as usize);
    commands.push(line(turn(1.0), track, RING_WIDTH));
    if let Some(share) = fraction
        .filter(|share| share.is_finite())
        .map(|share| share.clamp(0.0, 1.0))
        .filter(|share| *share > 0.0)
    {
        commands.push(line(turn(share), fill, RING_WIDTH));
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    pub(crate) fn distance(a: Point, b: Point) -> f32 {
        (a.x - b.x).hypot(a.y - b.y)
    }

    /// How far round from twelve o'clock, clockwise, in turns: 0.25 is three o'clock.
    pub(crate) fn turns(center: Point, point: Point) -> f32 {
        ((point.y - center.y).atan2(point.x - center.x) + FRAC_PI_2).rem_euclid(TAU) / TAU
    }

    /// Every step from one point to the next turns clockwise about `center`.
    pub(crate) fn clockwise(center: Point, points: &[Point]) -> bool {
        points.windows(2).all(|pair| {
            let (from, to) = (Point::new(pair[0].x - center.x, pair[0].y - center.y), Point::new(pair[1].x - center.x, pair[1].y - center.y));
            from.x * to.y - from.y * to.x > 0.0
        })
    }

    pub(crate) fn lines(commands: &[DrawCommand]) -> Vec<(&[Point], Color, f32)> {
        commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Polyline { points, color, width, dashed: false } => Some((&points[..], *color, *width)),
                _ => None,
            })
            .collect()
    }

    /// The square a `Circle` spinner was drawn in, and its angle, read back
    /// from its arc: the first polyline of three points or more.
    pub(crate) fn circle_spinner(commands: &[DrawCommand]) -> Option<(Rect, f32)> {
        let (points, _, width) = lines(commands).into_iter().find(|line| line.0.len() >= 3)?;
        // The circle through three of its points.
        let a = points[0];
        let (b, c) = (points[points.len() / 2], points[points.len() - 1]);
        let (b, c) = (Point::new(b.x - a.x, b.y - a.y), Point::new(c.x - a.x, c.y - a.y));
        let (d, bb, cc) = (2.0 * (b.x * c.y - b.y * c.x), b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
        let center = Point::new(a.x + (c.y * bb - b.y * cc) / d, a.y + (b.x * cc - c.x * bb) / d);
        let side = 2.0 * (distance(a, center) + width / 2.0 + 1.0);
        let square = Rect::new(center.x - side / 2.0, center.y - side / 2.0, side, side);
        Some((square, turns(center, a) * TAU))
    }

    fn inside(command: &DrawCommand, rect: Rect) -> bool {
        command.bounds().is_some_and(|drawn| {
            drawn.x >= rect.x && drawn.y >= rect.y && drawn.x + drawn.width <= rect.x + rect.width && drawn.y + drawn.height <= rect.y + rect.height
        })
    }

    fn is_disc(command: &DrawCommand, center: Point, diameter: f32, ink: Color) -> bool {
        matches!(command, DrawCommand::FillRect { rect, color, radius }
            if near(rect.x + rect.width / 2.0, center.x) && near(rect.y + rect.height / 2.0, center.y)
                && near(rect.width, diameter) && near(rect.height, diameter) && near(*radius, diameter / 2.0) && *color == ink)
    }

    #[test]
    fn a_circle_spinner_is_an_open_arc_with_round_ends_turned_by_its_angle() {
        let ink = Color::rgb(220, 220, 220);
        // Taller than wide: sized to the width, and centered.
        let rect = Rect::new(10.0, 20.0, 32.0, 40.0);
        let center = Point::new(26.0, 40.0);
        // 12% of 32 is past the widest stroke, 2.8; the arc keeps a pixel and half of it inside.
        let (width, reach) = (2.8, 16.0 - 1.4 - 1.0);
        let at = |theta: f32| Point::new(center.x + reach * theta.cos(), center.y + reach * theta.sin());
        for angle in [0.0, FRAC_PI_2, 2.0] {
            let mut commands = Vec::new();
            spinner(&mut commands, rect, ink, angle, SpinnerVariant::Circle);
            assert_eq!(commands.len(), 3);
            let (points, color, stroke) = lines(&commands)[0];
            assert_eq!((color, stroke), (ink, width));
            // A chord about every 1.4 pixels of the arc.
            assert_eq!(points.len(), 52);
            assert!(points.iter().all(|point| near(distance(*point, center), reach)));
            assert!(clockwise(center, points));
            // From twelve o'clock turned by the angle, five sixths of the way round.
            let (first, last) = (points[0], points[51]);
            assert!(near(first.x, at(angle - FRAC_PI_2).x) && near(first.y, at(angle - FRAC_PI_2).y), "{angle}");
            assert!(near(last.x, at(angle - FRAC_PI_2 + TAU * 5.0 / 6.0).x) && near(last.y, at(angle - FRAC_PI_2 + TAU * 5.0 / 6.0).y));
            assert!(is_disc(&commands[1], first, width, ink) && is_disc(&commands[2], last, width, ink), "round ends");
            assert!(commands.iter().all(|command| inside(command, rect)));
            assert_eq!(circle_spinner(&commands).map(|(square, _)| (near(square.x, 10.0), near(square.y, 24.0), near(square.width, 32.0))), Some((true, true, true)));
        }
        // The thinnest stroke and the fewest chords on a small one.
        let mut small = Vec::new();
        spinner(&mut small, Rect::new(0.0, 0.0, 8.0, 8.0), ink, 0.0, SpinnerVariant::Circle);
        assert_eq!((lines(&small)[0].0.len(), lines(&small)[0].2), (17, 1.5));
        let mut none = Vec::new();
        for rect in [Rect::new(0.0, 0.0, 0.0, 16.0), Rect::new(0.0, 0.0, f32::NAN, 16.0), Rect::new(0.0, 0.0, f32::INFINITY, 16.0)] {
            spinner(&mut none, rect, ink, 0.0, SpinnerVariant::Circle);
        }
        assert!(none.is_empty());
    }

    #[test]
    fn spokes_are_eight_round_ended_strokes_from_about_half_the_radius_out() {
        let ink = Color::WHITE;
        let rect = Rect::new(0.0, 0.0, 16.0, 16.0);
        let center = Point::new(8.0, 8.0);
        // 12% of 16, and the spokes' ends a pixel and half of it inside.
        let (width, reach) = (1.92, 8.0 - 0.96 - 1.0);
        let mut commands = Vec::new();
        spinner(&mut commands, rect, ink, 0.3, SpinnerVariant::Spokes);
        assert_eq!(commands.len(), 24);
        for (index, spoke) in commands.chunks(3).enumerate() {
            let theta = 0.3 + TAU * index as f32 / 8.0;
            let at = |distance: f32| Point::new(center.x + theta.cos() * distance, center.y + theta.sin() * distance);
            let (points, color, stroke) = lines(&spoke[..1])[0];
            assert_eq!((points.len(), color), (2, ink));
            assert!(near(stroke, width));
            assert!(near(points[0].x, at(reach * 0.55).x) && near(points[0].y, at(reach * 0.55).y));
            assert!(near(points[1].x, at(reach).x) && near(points[1].y, at(reach).y));
            assert!(is_disc(&spoke[1], points[0], stroke, ink) && is_disc(&spoke[2], points[1], stroke, ink));
        }
        assert!(commands.iter().all(|command| inside(command, rect)));
    }

    fn corners(angle: f32) -> Vec<(i32, i32)> {
        let mut dots: Vec<_> = dots(angle, 16.0).iter().map(|dot| (dot.x.round() as i32, dot.y.round() as i32)).collect();
        dots.sort();
        dots
    }

    #[test]
    fn dots_rest_on_a_square_and_line_up_on_each_diagonal() {
        let square = vec![(-5, -5), (-5, 5), (5, -5), (5, 5)];
        assert_eq!(corners(0.0), square);
        assert_eq!(corners(TAU * 0.4), square);
        // A quarter into each half the moving dots sit on the other diagonal.
        assert_eq!(corners(TAU * 0.14), vec![(-5, 5), (-2, 2), (2, -2), (5, -5)]);
        assert_eq!(corners(TAU * 0.64), vec![(-5, -5), (-2, -2), (2, 2), (5, 5)]);
        // Each is a disc 22% of the side across, around the middle.
        let mut commands = Vec::new();
        spinner(&mut commands, Rect::new(4.0, 4.0, 16.0, 16.0), Color::WHITE, TAU * 0.14, SpinnerVariant::Dots);
        assert_eq!(commands.len(), 4);
        for (command, dot) in commands.iter().zip(dots(TAU * 0.14, 16.0)) {
            assert!(is_disc(command, Point::new(12.0 + dot.x, 12.0 + dot.y), 3.52, Color::WHITE));
        }
    }

    #[test]
    fn a_laid_out_spinner_turns_with_the_clock_and_stands_still_under_reduced_motion() {
        use web_time::{Duration, Instant};
        let start = Instant::now();
        let viewport = crate::Size::new(32.0, 32.0);
        let mut tree = crate::UiTree::new(crate::Element::spinner("busy", Color::WHITE, SpinnerVariant::Circle).size(Some(32.0), Some(32.0)));
        let at = |tree: &mut crate::UiTree, ms: u64| tree.layout_at(viewport, start + Duration::from_millis(ms)).commands;
        let first = at(&mut tree, 0);
        // A quarter of a turn of 0.9 s later, the arc starts at three o'clock.
        let later = at(&mut tree, 225);
        assert_ne!(first, later);
        let reach = 16.0 - 1.4 - 1.0;
        assert!(near(lines(&first)[0].0[0].x, 16.0) && near(lines(&first)[0].0[0].y, 16.0 - reach));
        assert!(near(lines(&later)[0].0[0].x, 16.0 + reach) && near(lines(&later)[0].0[0].y, 16.0));
        tree.set_reduced_motion(true);
        let still = at(&mut tree, 300);
        assert_eq!(still, at(&mut tree, 1300));
        assert!(near(lines(&still)[0].0[0].x, 16.0) && near(lines(&still)[0].0[0].y, 16.0 - reach), "at twelve o'clock");
    }

    #[test]
    fn a_spinner_and_a_ring_are_drawn_with_shapes_that_name_no_widget() {
        let shapes = |element: crate::Element| {
            let frame = crate::UiTree::new(element.size(Some(24.0), Some(24.0))).layout(crate::Size::new(24.0, 24.0));
            assert!(!frame.commands.is_empty());
            frame.commands.iter().all(|command| matches!(command, DrawCommand::Polyline { .. } | DrawCommand::FillRect { .. }))
        };
        for variant in [SpinnerVariant::Circle, SpinnerVariant::Spokes, SpinnerVariant::Dots] {
            assert!(shapes(crate::Element::spinner("busy", Color::WHITE, variant)), "{variant:?}");
        }
        assert!(shapes(crate::Element::progress_ring(Some(0.4), Color::rgb(60, 60, 60), Color::WHITE)));
    }

    #[test]
    fn a_ring_is_a_circle_of_its_track_under_an_arc_of_its_fill_from_twelve_o_clock_clockwise() {
        let (track, fill) = (Color::rgb(60, 60, 60), Color::WHITE);
        // Wider than tall: sized to the height, and centered.
        let mut commands = Vec::new();
        progress_ring(&mut commands, Rect::new(10.0, 20.0, 40.0, 16.0), Some(0.25), track, fill);
        let center = Point::new(30.0, 28.0);
        let lines = lines(&commands);
        assert_eq!((commands.len(), lines.len()), (2, 2), "two polylines and nothing else");
        for (points, _, width) in &lines {
            assert_eq!(*width, 1.7);
            assert!(points.iter().all(|point| near(distance(*point, center), 7.0)));
            assert!(near(points[0].x, 30.0) && near(points[0].y, 21.0), "from twelve o'clock");
            assert!(clockwise(center, points));
        }
        let (circle, color, _) = lines[0];
        assert_eq!((circle.len(), color), (65, track));
        assert!(distance(circle[0], circle[64]) < 1e-3, "the circle closes");
        // Its 64 chords keep within a twentieth of a pixel of the circle.
        assert!(circle.windows(2).all(|pair| distance(Point::new((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0), center) > 7.0 - 0.05));
        let (arc, color, _) = lines[1];
        assert_eq!((arc.len(), color), (17, fill));
        assert!(near(turns(center, arc[16]), 0.25), "a quarter of the way round");
    }

    #[test]
    fn a_ring_with_no_share_is_its_track_alone_and_one_too_small_draws_nothing() {
        let (track, fill) = (Color::rgb(60, 60, 60), Color::WHITE);
        let ring = |rect: Rect, fraction: Option<f32>| {
            let mut commands = Vec::new();
            progress_ring(&mut commands, rect, fraction, track, fill);
            commands
        };
        let square = Rect::new(0.0, 0.0, 32.0, 32.0);
        for fraction in [None, Some(0.0), Some(-1.0), Some(f32::NAN)] {
            let commands = ring(square, fraction);
            assert_eq!(lines(&commands).iter().map(|line| line.1).collect::<Vec<_>>(), vec![track], "{fraction:?}");
            assert_eq!(commands.len(), 1);
        }
        // A share past the whole is the whole circle in the fill.
        let full = ring(square, Some(7.0));
        let full = lines(&full);
        assert_eq!((full[1].0.len(), full[1].1), (65, fill));
        assert!(distance(full[1].0[0], full[1].0[64]) < 1e-3);
        assert!(ring(Rect::new(0.0, 0.0, 2.0, 40.0), Some(0.5)).is_empty());
        assert!(ring(Rect::new(0.0, 0.0, f32::INFINITY, 16.0), Some(0.5)).is_empty());
        assert!(ring(Rect::new(0.0, 0.0, f32::NAN, 16.0), Some(0.5)).is_empty());
    }

    #[test]
    fn a_laid_out_ring_paints_polylines_in_its_square() {
        let frame = crate::UiTree::new(crate::Element::progress_ring(Some(0.5), Color::rgb(60, 60, 60), Color::WHITE).size(Some(24.0), Some(24.0)))
            .layout(crate::Size::new(24.0, 24.0));
        let mut commands = Vec::new();
        progress_ring(&mut commands, Rect::new(0.0, 0.0, 24.0, 24.0), Some(0.5), Color::rgb(60, 60, 60), Color::WHITE);
        assert_eq!(frame.commands, commands);
    }
}
