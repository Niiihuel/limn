//! How a progress ring and a spinner are drawn, from the commands every
//! backend paints: polylines for arcs and spokes, and fully rounded squares
//! for dots and for the round ends a polyline does not have.

use crate::{Color, DrawCommand, Point, Rect};
use std::f32::consts::{FRAC_PI_2, TAU};

/// Width of a progress ring's circle and arc, in logical pixels.
const RING_WIDTH: f32 = 1.7;

/// `segments` chords along the circle of `radius` around `center`, from
/// `start` radians through `sweep`. 0 is three o'clock, and a positive sweep
/// runs clockwise, since y grows downwards.
fn arc(center: Point, radius: f32, start: f32, sweep: f32, segments: usize) -> Vec<Point> {
    (0..=segments)
        .map(|step| {
            let theta = start + sweep * step as f32 / segments as f32;
            Point::new(center.x + radius * theta.cos(), center.y + radius * theta.sin())
        })
        .collect()
}

fn line(points: Vec<Point>, color: Color, width: f32) -> DrawCommand {
    DrawCommand::Polyline { points: points.into(), color, width, dashed: false }
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
