//! Marks drawn over a picture or any area, for annotating it.

use std::f32::consts::TAU;
use std::sync::Arc;

use crate::{Color, Component, DrawCommand, Element, Point, Rect};

/// Vertices in a flattened ellipse.
const ELLIPSE_SEGMENTS: usize = 64;

/// Which mark a press and drag make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SketchTool {
    /// A freehand polyline. A press without a drag is a dot.
    Pen,
    /// A straight line from the press to the release.
    Line,
    /// A line with a filled head at its end.
    Arrow,
    /// An outlined rectangle spanned by the press and the release.
    Rectangle,
    /// An outlined ellipse inside the box spanned by the press and the release.
    Ellipse,
}

/// A finished mark. Positions are shares of the sketch's width and height,
/// from 0 to 1, so the marks survive a resize. Widths are in logical pixels
/// at the size the sketch is shown at.
#[derive(Clone, Debug, PartialEq)]
pub enum SketchMark {
    /// A pen stroke, in order. One point is a dot.
    Stroke {
        /// The points, as shares of the sketch.
        points: Vec<Point>,
        /// Stroke color.
        color: Color,
        /// Stroke width in logical pixels.
        width: f32,
    },
    /// A straight line.
    Line {
        /// Where the press was.
        from: Point,
        /// Where the pointer was released.
        to: Point,
        /// Stroke color.
        color: Color,
        /// Stroke width in logical pixels.
        width: f32,
    },
    /// A line with a filled head at `to`.
    Arrow {
        /// The tail.
        from: Point,
        /// The tip.
        to: Point,
        /// Color of the shaft and the head.
        color: Color,
        /// Stroke width in logical pixels; the head is four times as long, or
        /// as long as the line.
        width: f32,
    },
    /// An outlined rectangle with corners `from` and `to`.
    Rectangle {
        /// One corner.
        from: Point,
        /// The opposite corner.
        to: Point,
        /// Stroke color.
        color: Color,
        /// Stroke width in logical pixels.
        width: f32,
    },
    /// An outlined ellipse inside the box with corners `from` and `to`.
    Ellipse {
        /// One corner of its box.
        from: Point,
        /// The opposite corner.
        to: Point,
        /// Stroke color.
        color: Color,
        /// Stroke width in logical pixels.
        width: f32,
    },
}

/// The style a sketch paints its marks and its draft with, as a tree reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SketchStyle {
    pub(crate) tool: SketchTool,
    pub(crate) color: Color,
    pub(crate) width: f32,
}

/// Marks over an area. The host owns the marks and passes them on every
/// rebuild; a press and drag over the sketch make a mark of `tool`, which the
/// sketch reports as `UiAction::Sketched` on release. The mark in progress is
/// drawn by the tree, not the host.
pub struct Sketch {
    id: String,
    marks: Vec<SketchMark>,
    style: SketchStyle,
    size: Option<(f32, f32)>,
}

impl Sketch {
    /// A sketch under `id`, showing `marks`; a press and drag make a mark of
    /// `tool` in `color` and `width`. A width that is not finite is 2 and a
    /// negative one is 0. Without [`Sketch::size`] it takes the size its
    /// parent gives it.
    pub fn new(
        id: impl Into<String>,
        marks: &[SketchMark],
        tool: SketchTool,
        color: Color,
        width: f32,
    ) -> Self {
        Self {
            id: id.into(),
            marks: marks.to_vec(),
            style: SketchStyle {
                tool,
                color,
                width: if width.is_finite() { width.max(0.0) } else { 2.0 },
            },
            size: None,
        }
    }

    /// The sketch's size in logical pixels. A side that is not finite is
    /// ignored and a negative one is zero.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() {
            self.size = Some((width.max(0.0), height.max(0.0)));
        }
        self
    }
}

impl Component for Sketch {
    fn render(self) -> Element {
        let marks = self.marks;
        let (width, height) = self.size.map_or((None, None), |(w, h)| (Some(w), Some(h)));
        Element::clickable(
            self.id,
            Element::drawing(move |size| {
                let rect = Rect::new(0.0, 0.0, size.width, size.height);
                marks.iter().flat_map(|mark| mark_commands(mark, rect)).collect()
            }),
        )
        .size(width, height)
        .sketch(self.style)
    }
}

/// The in-progress mark of a press that has not been released.
pub(crate) struct SketchDraft {
    pub(crate) id: String,
    rect: Rect,
    style: SketchStyle,
    start: Point,
    end: Point,
    points: Vec<Point>,
}

impl SketchDraft {
    pub(crate) fn new(id: String, rect: Rect, style: SketchStyle, at: Point) -> Self {
        let start = share(at, rect);
        Self {
            id,
            rect,
            style,
            start,
            end: start,
            points: vec![start],
        }
    }

    /// The pointer moved to `at`. A pen adds a point unless it is where the
    /// last one was.
    pub(crate) fn extend(&mut self, at: Point) {
        self.end = share(at, self.rect);
        if self.style.tool == SketchTool::Pen && self.points.last() != Some(&self.end) {
            self.points.push(self.end);
        }
    }

    /// The mark as it stands: the finished one on release, or the one to draw.
    pub(crate) fn mark(&self) -> SketchMark {
        let (from, to) = (self.start, self.end);
        let (color, width) = (self.style.color, self.style.width);
        match self.style.tool {
            SketchTool::Pen => SketchMark::Stroke {
                points: self.points.clone(),
                color,
                width,
            },
            SketchTool::Line => SketchMark::Line { from, to, color, width },
            SketchTool::Arrow => SketchMark::Arrow { from, to, color, width },
            SketchTool::Rectangle => SketchMark::Rectangle { from, to, color, width },
            SketchTool::Ellipse => SketchMark::Ellipse { from, to, color, width },
        }
    }
}

/// A position as a share of `rect`, clamped to its edges.
fn share(at: Point, rect: Rect) -> Point {
    let part = |value: f32, start: f32, length: f32| {
        if length > 0.0 {
            ((value - start) / length).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    Point::new(
        part(at.x, rect.x, rect.width),
        part(at.y, rect.y, rect.height),
    )
}

/// Where a share of the sketch lies inside `rect`.
fn place(share: Point, rect: Rect) -> Point {
    Point::new(rect.x + share.x * rect.width, rect.y + share.y * rect.height)
}

/// A filled disc, drawn as a fully rounded square.
fn dot(center: Point, radius: f32, color: Color) -> DrawCommand {
    DrawCommand::FillRect {
        rect: Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0),
        color,
        radius,
    }
}

/// A polyline with a dot at each vertex: the round caps and joins a polyline
/// does not draw.
fn stroke(points: Vec<Point>, color: Color, width: f32) -> Vec<DrawCommand> {
    let radius = width / 2.0;
    let mut commands: Vec<DrawCommand> = points.iter().map(|point| dot(*point, radius, color)).collect();
    if points.len() > 1 {
        commands.insert(
            0,
            DrawCommand::Polyline {
                points: Arc::from(points),
                color,
                width,
                dashed: false,
            },
        );
    }
    commands
}

/// The draw commands of `mark` inside `rect`, in the same coordinates as `rect`.
pub(crate) fn mark_commands(mark: &SketchMark, rect: Rect) -> Vec<DrawCommand> {
    match mark {
        SketchMark::Stroke { points, color, width } => stroke(
            points.iter().map(|point| place(*point, rect)).collect(),
            *color,
            *width,
        ),
        SketchMark::Line { from, to, color, width } => {
            stroke(vec![place(*from, rect), place(*to, rect)], *color, *width)
        }
        SketchMark::Arrow { from, to, color, width } => {
            let (tail, tip) = (place(*from, rect), place(*to, rect));
            let (dx, dy) = (tip.x - tail.x, tip.y - tail.y);
            let length = dx.hypot(dy);
            if length == 0.0 {
                return stroke(vec![tail, tip], *color, *width);
            }
            let (ux, uy) = (dx / length, dy / length);
            // The head is four times the width long, and never longer than the line.
            let head = (width * 4.0).min(length);
            let base = Point::new(tip.x - ux * head, tip.y - uy * head);
            let half = head / 2.0;
            let mut commands = stroke(vec![tail, base], *color, *width);
            commands.push(DrawCommand::FillTriangle {
                points: [
                    tip,
                    Point::new(base.x - uy * half, base.y + ux * half),
                    Point::new(base.x + uy * half, base.y - ux * half),
                ],
                color: *color,
            });
            commands
        }
        SketchMark::Rectangle { from, to, color, width } => {
            let (a, b) = (place(*from, rect), place(*to, rect));
            let half = width / 2.0;
            // The outline is centered on its path, as the rasterizer draws it.
            let outline = Rect::new(
                a.x.min(b.x) - half,
                a.y.min(b.y) - half,
                (a.x - b.x).abs() + width,
                (a.y - b.y).abs() + width,
            );
            let mut commands = vec![DrawCommand::StrokeRect {
                rect: outline,
                color: *color,
                width: *width,
                radius: 0.0,
            }];
            // Square corners; a dot at each one stands in for a round join.
            for corner in [Point::new(a.x, a.y), Point::new(b.x, a.y), Point::new(b.x, b.y), Point::new(a.x, b.y)] {
                commands.push(dot(corner, half, *color));
            }
            commands
        }
        SketchMark::Ellipse { from, to, color, width } => {
            let (a, b) = (place(*from, rect), place(*to, rect));
            let (cx, cy) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let (rx, ry) = ((a.x - b.x).abs() / 2.0, (a.y - b.y).abs() / 2.0);
            let path: Vec<Point> = (0..=ELLIPSE_SEGMENTS)
                .map(|step| {
                    let angle = TAU * step as f32 / ELLIPSE_SEGMENTS as f32;
                    Point::new(cx + rx * angle.cos(), cy + ry * angle.sin())
                })
                .collect();
            vec![DrawCommand::Polyline {
                points: Arc::from(path),
                color: *color,
                width: *width,
                dashed: false,
            }]
        }
    }
}

/// Paints `marks` onto an RGBA8 buffer of the full-size image, `width` by
/// `height` pixels, row by row from the top-left pixel. `scale` is the pixels per logical pixel of the sketch: the
/// buffer's width divided by the width the sketch is displayed at. Each mark
/// is drawn over the ones before it. Edges are not anti-aliased: a pixel is
/// colored when its center is inside the stroke.
pub fn rasterize(marks: &[SketchMark], width: u32, height: u32, rgba: &mut [u8], scale: f32) {
    let (w, h) = (width as f32, height as f32);
    if !scale.is_finite() || scale <= 0.0 || rgba.len() < (width as usize) * (height as usize) * 4 {
        return;
    }
    for mark in marks {
        let (shapes, color) = raster_shapes(mark, w, h, scale);
        let mut bounds: Option<[f32; 4]> = None;
        for shape in &shapes {
            let r = shape.bounds();
            let b = [r.x, r.y, r.x + r.width, r.y + r.height];
            bounds = Some(match bounds {
                None => b,
                Some(a) => [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])],
            });
        }
        let Some([left, top, right, bottom]) = bounds else {
            continue;
        };
        let (x0, y0) = (left.floor().max(0.0) as u32, top.floor().max(0.0) as u32);
        let (x1, y1) = (right.ceil().min(w).max(0.0) as u32, bottom.ceil().min(h).max(0.0) as u32);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        // Each shape tests only the pixels of its own box, so a long stroke costs
        // its length, not its area times its segments.
        let span = (x1 - x0) as usize;
        let mut covered = vec![false; span * (y1 - y0) as usize];
        for shape in &shapes {
            let r = shape.bounds();
            let (sx0, sy0) = (r.x.floor().max(x0 as f32) as u32, r.y.floor().max(y0 as f32) as u32);
            let (sx1, sy1) = ((r.x + r.width).ceil().min(x1 as f32).max(0.0) as u32, (r.y + r.height).ceil().min(y1 as f32).max(0.0) as u32);
            for y in sy0..sy1 {
                for x in sx0..sx1 {
                    let index = (y - y0) as usize * span + (x - x0) as usize;
                    if !covered[index] && shape.contains(Point::new(x as f32 + 0.5, y as f32 + 0.5)) {
                        covered[index] = true;
                    }
                }
            }
        }
        // One blend per pixel, however many of the mark's shapes cover it.
        for (index, _) in covered.iter().enumerate().filter(|(_, covered)| **covered) {
            let at = ((y0 + (index / span) as u32) * width + x0 + (index % span) as u32) as usize * 4;
            let alpha = f32::from(color.a) / 255.0;
            for (channel, source) in [color.r, color.g, color.b].into_iter().enumerate() {
                let below = f32::from(rgba[at + channel]);
                rgba[at + channel] = (f32::from(source) * alpha + below * (1.0 - alpha)).round() as u8;
            }
            let below = f32::from(rgba[at + 3]) / 255.0;
            rgba[at + 3] = ((alpha + below * (1.0 - alpha)) * 255.0).round() as u8;
        }
    }
}

/// A region of pixels a mark covers: the union of its shapes.
enum Shape {
    /// Points within `radius` of the segment from `a` to `b`.
    Capsule { a: Point, b: Point, radius: f32 },
    /// A filled triangle.
    Triangle([Point; 3]),
}

impl Shape {
    fn bounds(&self) -> Rect {
        let (min, max) = match self {
            Shape::Capsule { a, b, radius } => (
                Point::new(a.x.min(b.x) - radius, a.y.min(b.y) - radius),
                Point::new(a.x.max(b.x) + radius, a.y.max(b.y) + radius),
            ),
            Shape::Triangle(points) => (
                Point::new(
                    points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                    points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
                ),
                Point::new(
                    points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                    points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
                ),
            ),
        };
        Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
    }

    fn contains(&self, p: Point) -> bool {
        match self {
            Shape::Capsule { a, b, radius } => {
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let length_sq = dx * dx + dy * dy;
                let t = if length_sq > 0.0 {
                    (((p.x - a.x) * dx + (p.y - a.y) * dy) / length_sq).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let (nx, ny) = (a.x + dx * t - p.x, a.y + dy * t - p.y);
                nx * nx + ny * ny <= radius * radius
            }
            Shape::Triangle([a, b, c]) => {
                let side = |p1: Point, p2: Point| (p2.x - p1.x) * (p.y - p1.y) - (p2.y - p1.y) * (p.x - p1.x);
                let (s1, s2, s3) = (side(*a, *b), side(*b, *c), side(*c, *a));
                // A degenerate triangle covers nothing.
                if s1 == 0.0 && s2 == 0.0 && s3 == 0.0 {
                    return false;
                }
                let negative = s1 < 0.0 || s2 < 0.0 || s3 < 0.0;
                let positive = s1 > 0.0 || s2 > 0.0 || s3 > 0.0;
                !(negative && positive)
            }
        }
    }
}

/// The shapes of `mark` in pixels of a `w` by `h` buffer, and its color.
fn raster_shapes(mark: &SketchMark, w: f32, h: f32, scale: f32) -> (Vec<Shape>, Color) {
    let px = |p: &Point| Point::new(p.x * w, p.y * h);
    let capsules = |points: &[Point], radius: f32| -> Vec<Shape> {
        points
            .windows(2)
            .map(|pair| Shape::Capsule { a: pair[0], b: pair[1], radius })
            .collect()
    };
    match mark {
        SketchMark::Stroke { points, color, width } => {
            let radius = width * scale / 2.0;
            let pixels: Vec<Point> = points.iter().map(px).collect();
            let mut shapes = capsules(&pixels, radius);
            if pixels.len() == 1 {
                shapes.push(Shape::Capsule { a: pixels[0], b: pixels[0], radius });
            }
            (shapes, *color)
        }
        SketchMark::Line { from, to, color, width } => {
            let radius = width * scale / 2.0;
            (capsules(&[px(from), px(to)], radius), *color)
        }
        SketchMark::Arrow { from, to, color, width } => {
            let radius = width * scale / 2.0;
            let (tail, tip) = (px(from), px(to));
            let (dx, dy) = (tip.x - tail.x, tip.y - tail.y);
            let length = dx.hypot(dy);
            let mut shapes = capsules(&[tail, tip], radius);
            if length > 0.0 {
                let (ux, uy) = (dx / length, dy / length);
                let head = (width * scale * 4.0).min(length);
                let base = Point::new(tip.x - ux * head, tip.y - uy * head);
                let half = head / 2.0;
                shapes.push(Shape::Triangle([
                    tip,
                    Point::new(base.x - uy * half, base.y + ux * half),
                    Point::new(base.x + uy * half, base.y - ux * half),
                ]));
            }
            (shapes, *color)
        }
        SketchMark::Rectangle { from, to, color, width } => {
            let radius = width * scale / 2.0;
            let (a, b) = (px(from), px(to));
            let corners = [a, Point::new(b.x, a.y), b, Point::new(a.x, b.y), a];
            (capsules(&corners, radius), *color)
        }
        SketchMark::Ellipse { from, to, color, width } => {
            let radius = width * scale / 2.0;
            let (a, b) = (px(from), px(to));
            let (cx, cy) = ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let (rx, ry) = ((a.x - b.x).abs() / 2.0, (a.y - b.y).abs() / 2.0);
            let path: Vec<Point> = (0..=ELLIPSE_SEGMENTS)
                .map(|step| {
                    let angle = TAU * step as f32 / ELLIPSE_SEGMENTS as f32;
                    Point::new(cx + rx * angle.cos(), cy + ry * angle.sin())
                })
                .collect();
            (capsules(&path, radius), *color)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, UiAction, UiEvent, UiTree};

    const INK: Color = Color::rgb(255, 0, 0);

    fn sketch_tree(tool: SketchTool) -> UiTree {
        let sketch = Sketch::new("sketch", &[], tool, INK, 4.0).size(100.0, 100.0).render();
        let mut tree = UiTree::new(Element::row(vec![sketch]));
        tree.layout(Size::new(200.0, 200.0));
        tree
    }

    fn sketched(actions: &[UiAction]) -> Vec<SketchMark> {
        actions
            .iter()
            .filter_map(|action| match action {
                UiAction::Sketched { mark, .. } => Some(mark.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_pen_drag_reports_one_stroke_whose_points_follow_the_drag() {
        let mut tree = sketch_tree(SketchTool::Pen);
        let mut actions = tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
        actions.extend(tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 10.0))));
        actions.extend(tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 110.0))));
        actions.extend(tree.handle_event(UiEvent::PointerUp(Point::new(150.0, 50.0))));
        assert_eq!(
            sketched(&actions),
            [SketchMark::Stroke {
                points: vec![
                    Point::new(0.1, 0.1),
                    Point::new(0.6, 0.1),
                    Point::new(0.6, 1.0),
                    Point::new(1.0, 0.5),
                ],
                color: INK,
                width: 4.0,
            }]
        );
    }

    #[test]
    fn a_pen_click_without_a_drag_is_a_dot() {
        let mut tree = sketch_tree(SketchTool::Pen);
        tree.handle_event(UiEvent::PointerDown(Point::new(30.0, 40.0)));
        let actions = tree.handle_event(UiEvent::PointerUp(Point::new(30.0, 40.0)));
        assert_eq!(
            sketched(&actions),
            [SketchMark::Stroke {
                points: vec![Point::new(0.3, 0.4)],
                color: INK,
                width: 4.0,
            }]
        );
    }

    #[test]
    fn a_rectangle_reports_its_corners_and_no_click() {
        let mut tree = sketch_tree(SketchTool::Rectangle);
        tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 20.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(50.0, 80.0)));
        let actions = tree.handle_event(UiEvent::PointerUp(Point::new(90.0, 60.0)));
        assert_eq!(
            actions,
            [UiAction::Sketched {
                id: "sketch".to_owned(),
                mark: SketchMark::Rectangle {
                    from: Point::new(0.1, 0.2),
                    to: Point::new(0.9, 0.6),
                    color: INK,
                    width: 4.0,
                },
            }]
        );
    }

    #[test]
    fn the_mark_in_progress_is_drawn_before_release() {
        let mut tree = sketch_tree(SketchTool::Line);
        tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(90.0, 10.0)));
        let frame = tree.layout(Size::new(200.0, 200.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Polyline { points, .. } if points.len() == 2 && points[1].x > points[0].x
        )));
    }

    #[test]
    fn rendering_marks_draws_their_commands_inside_the_sketch() {
        let marks = [
            SketchMark::Arrow { from: Point::new(0.1, 0.1), to: Point::new(0.9, 0.5), color: INK, width: 4.0 },
            SketchMark::Ellipse { from: Point::new(0.2, 0.2), to: Point::new(0.8, 0.8), color: INK, width: 2.0 },
        ];
        let element = Sketch::new("sketch", &marks, SketchTool::Pen, INK, 2.0).size(100.0, 100.0).render();
        let frame = UiTree::new(Element::row(vec![element])).layout(Size::new(200.0, 200.0));
        let kinds: Vec<&str> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillTriangle { .. } => Some("triangle"),
                DrawCommand::Polyline { .. } => Some("polyline"),
                _ => None,
            })
            .collect();
        assert!(kinds.contains(&"triangle"));
        assert_eq!(kinds.iter().filter(|kind| **kind == "polyline").count(), 2);
        for command in &frame.commands {
            if let DrawCommand::Polyline { points, .. } = command {
                assert!(points.iter().all(|p| p.x >= 0.0 && p.x <= 100.0 && p.y >= 0.0 && p.y <= 100.0));
            }
        }
    }

    fn painted(marks: &[SketchMark]) -> Vec<u8> {
        let mut rgba = vec![0u8; 20 * 20 * 4];
        rasterize(marks, 20, 20, &mut rgba, 1.0);
        rgba
    }

    fn red(rgba: &[u8], x: usize, y: usize) -> bool {
        let at = (y * 20 + x) * 4;
        rgba[at] == 255 && rgba[at + 3] == 255
    }

    #[test]
    fn a_horizontal_line_colors_its_pixels_and_leaves_the_corners() {
        let rgba = painted(&[SketchMark::Line {
            from: Point::new(0.1, 0.5),
            to: Point::new(0.9, 0.5),
            color: INK,
            width: 4.0,
        }]);
        assert!(red(&rgba, 10, 10) && red(&rgba, 2, 10) && red(&rgba, 17, 10));
        assert!(!red(&rgba, 10, 5) && !red(&rgba, 0, 0) && !red(&rgba, 19, 19));
    }

    #[test]
    fn a_rectangle_border_is_colored_and_its_inside_is_not() {
        let rgba = painted(&[SketchMark::Rectangle {
            from: Point::new(0.2, 0.2),
            to: Point::new(0.8, 0.8),
            color: INK,
            width: 4.0,
        }]);
        assert!(red(&rgba, 4, 10) && red(&rgba, 10, 4) && red(&rgba, 15, 15));
        assert!(!red(&rgba, 10, 10) && !red(&rgba, 0, 0));
    }

    #[test]
    fn a_dot_is_a_filled_disc() {
        let rgba = painted(&[SketchMark::Stroke {
            points: vec![Point::new(0.5, 0.5)],
            color: INK,
            width: 8.0,
        }]);
        assert!(red(&rgba, 10, 10) && red(&rgba, 7, 10) && red(&rgba, 12, 12));
        assert!(!red(&rgba, 10, 2) && !red(&rgba, 0, 0));
    }

    #[test]
    fn the_scale_widens_the_stroke() {
        let mark = [SketchMark::Line {
            from: Point::new(0.1, 0.5),
            to: Point::new(0.9, 0.5),
            color: INK,
            width: 2.0,
        }];
        let mut rgba = vec![0u8; 20 * 20 * 4];
        rasterize(&mark, 20, 20, &mut rgba, 2.0);
        assert!(red(&rgba, 10, 8) && red(&rgba, 10, 11));
    }
}
