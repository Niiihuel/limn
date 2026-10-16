//! How the tree draws the plots: bars, lines, pies and radars, their axes,
//! and what the pointer does to them.

use super::{
    paint_corner_rect, paint_fade, AnimatedScalar, CornerRadii, DeferredOverlay, DrawCommand, HitKind, LayoutBuilder,
    Point, Rect, Size,
};
use crate::{BarPlot, Color, Element, FontFamily, ScalarTransition};

use std::time::Duration;

/// Size of the ticks and of the categories' names.
const AXIS_TEXT: f32 = 10.0;
/// Room under or beside the plot for one line of that text.
const AXIS_GUTTER: f32 = 18.0;
/// The share of a category's room its bars take.
const BAND_FILL: f32 = 0.6;
/// How far a bar that is not under the pointer recedes.
const DIM: f32 = 0.45;
/// How much of its color a bar with a gradient keeps at its foot.
const FOOT: f32 = 0.3;

/// The side a bar grows towards.
#[derive(Clone, Copy, PartialEq)]
enum Tip {
    Up,
    Down,
    Right,
    Left,
}

/// A step between ticks that reads well: 1, 2 or 5 times a power of ten,
/// for about four of them across `span`. Values that are all whole, such as
/// counts, never get a step under 1: an axis of tasks has no half task.
fn tick_step(span: f64, whole: bool) -> f64 {
    let raw = span / 4.0;
    let magnitude = 10f64.powf(raw.log10().floor());
    let share = raw / magnitude;
    let step = magnitude * if share <= 1.0 { 1.0 } else if share <= 2.0 { 2.0 } else if share <= 5.0 { 5.0 } else { 10.0 };
    if whole { step.max(1.0) } else { step }
}

fn is_whole(value: f64) -> bool {
    value.fract() == 0.0
}

/// A value as it is written: whole numbers bare, the rest to one decimal.
fn written(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn faded(color: Color, share: f32) -> Color {
    Color { a: (f32::from(color.a) * share.clamp(0.0, 1.0)).round() as u8, ..color }
}

/// One bar: square at its foot, round at its tip, and with a gradient along
/// its length when it has one.
fn paint_bar(commands: &mut Vec<DrawCommand>, rect: Rect, color: Color, tip: Tip, radius: f32, gradient: bool) {
    // On whole pixels: the rows under a round tip would otherwise overlap by a fraction and show as seams.
    let (left, top) = (rect.x.round(), rect.y.round());
    let rect = Rect::new(left, top, (rect.x + rect.width).round() - left, (rect.y + rect.height).round() - top);
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let across = if matches!(tip, Tip::Up | Tip::Down) { rect.width } else { rect.height };
    let along = if matches!(tip, Tip::Up | Tip::Down) { rect.height } else { rect.width };
    let radius = radius.min(across / 2.0).min(along).max(0.0);
    if !gradient {
        let (top, bottom, left, right) = (tip == Tip::Up, tip == Tip::Down, tip == Tip::Left, tip == Tip::Right);
        let corner = |round: bool| if round { radius } else { 0.0 };
        let radii = CornerRadii {
            top_left: corner(top || left),
            top_right: corner(top || right),
            bottom_right: corner(bottom || right),
            bottom_left: corner(bottom || left),
        };
        paint_corner_rect(commands, rect, color, None, 0.0, Some(radii));
        return;
    }
    let foot = faded(color, FOOT);
    match tip {
        Tip::Up => paint_fade(commands, rect, color, foot, radius, true),
        Tip::Down => paint_fade(commands, rect, foot, color, radius, false),
        Tip::Right | Tip::Left => {
            // The straight part is one gradient; under the tip's corners it
            // is columns a pixel wide, each as tall as the corner leaves it.
            let rounded = radius.floor();
            let towards_right = tip == Tip::Right;
            let at = |x: f32| {
                let share = ((x - rect.x) / rect.width).clamp(0.0, 1.0);
                super::mix(foot, color, if towards_right { share } else { 1.0 - share })
            };
            let straight = if towards_right {
                Rect::new(rect.x, rect.y, rect.width - rounded, rect.height)
            } else {
                Rect::new(rect.x + rounded, rect.y, rect.width - rounded, rect.height)
            };
            if straight.width > 0.0 {
                commands.push(DrawCommand::FillHorizontalGradient {
                    rect: straight,
                    left: at(straight.x),
                    right: at(straight.x + straight.width),
                });
            }
            for column in 0..rounded as usize {
                let depth = column as f32 + 0.5;
                let inset = radius - (radius * radius - (radius - depth).powi(2)).max(0.0).sqrt();
                let x = if towards_right { rect.x + rect.width - column as f32 - 1.0 } else { rect.x + column as f32 };
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(x, rect.y + inset, 1.0, (rect.height - 2.0 * inset).max(0.0)),
                    color: at(x + 0.5),
                    radius: 0.0,
                });
            }
        }
    }
}

/// A path that passes through every knot and bends between them without
/// overshooting any: a monotone cubic, cut into short strokes.
fn rounded_through(knots: &[Point]) -> Vec<Point> {
    if knots.len() < 3 {
        return knots.to_vec();
    }
    // The slope of each stretch, and at each knot a slope that keeps the curve between its neighbours.
    let slope: Vec<f32> = knots.windows(2).map(|pair| (pair[1].y - pair[0].y) / (pair[1].x - pair[0].x).max(1e-3)).collect();
    let mut tangent = vec![0.0f32; knots.len()];
    tangent[0] = slope[0];
    tangent[knots.len() - 1] = slope[slope.len() - 1];
    for knot in 1..knots.len() - 1 {
        let (before, after) = (slope[knot - 1], slope[knot]);
        tangent[knot] = if before * after <= 0.0 { 0.0 } else { 2.0 * before * after / (before + after) };
    }
    const STROKES: usize = 14;
    let mut path = Vec::with_capacity((knots.len() - 1) * STROKES + 1);
    for (stretch, pair) in knots.windows(2).enumerate() {
        let width = pair[1].x - pair[0].x;
        for stroke in 0..STROKES {
            let t = stroke as f32 / STROKES as f32;
            let (t2, t3) = (t * t, t * t * t);
            let y = (2.0 * t3 - 3.0 * t2 + 1.0) * pair[0].y
                + (t3 - 2.0 * t2 + t) * width * tangent[stretch]
                + (-2.0 * t3 + 3.0 * t2) * pair[1].y
                + (t3 - t2) * width * tangent[stretch + 1];
            path.push(Point::new(pair[0].x + width * t, y));
        }
    }
    path.push(knots[knots.len() - 1]);
    path
}

impl LayoutBuilder<'_> {
    /// From nothing to one over four moves, starting when the key is first
    /// seen: what a plot's bars grow by as it appears.
    fn appearing(&self, key: &str) -> f32 {
        let mut scalars = self.scalars.borrow_mut();
        let state = scalars.entry(key.to_owned()).or_insert_with(|| {
            let mut value = ScalarTransition::new(0.0, self.now);
            let duration = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 4);
            value.retarget_with_motion(1.0, self.now, duration, self.reduced_motion);
            AnimatedScalar { value, seen: self.now }
        });
        state.seen = self.now;
        state.value.value(self.now)
    }

    /// One line of axis text at `x`, `y`: `align` is 0 to start there, 0.5
    /// to be centered on it and 1 to end there.
    fn axis_text(&mut self, text: &str, color: Color, x: f32, y: f32, align: f32, clip: Option<Rect>) {
        let label = Element::text(text).font_family(FontFamily::Sans).font_size(AXIS_TEXT).foreground(color);
        // Not the cached measure: that one knows an element by where it is, and these are made and dropped in place.
        let size = self.measure_uncached(&label, Size::new(4096.0, 4096.0));
        self.place(&label, Rect::new((x - size.width * align).round(), y.round(), size.width, size.height), clip);
    }

    fn axis_text_width(&self, text: &str) -> f32 {
        let label = Element::text(text).font_family(FontFamily::Sans).font_size(AXIS_TEXT);
        self.measure_uncached(&label, Size::new(4096.0, 4096.0)).width
    }

    /// The tooltip of a plot: a title over a row for each series, with its
    /// color, its name and its value. It floats beside the pointer, on the
    /// side of it where `around` has more room, and never out of the window.
    fn plot_tooltip(
        &mut self,
        style: crate::TooltipStyle,
        title: &str,
        rows: Vec<(Option<Color>, String, String)>,
        around: Rect,
        pointer: Point,
    ) {
        let text = |value: &str, color: Color, family: FontFamily| {
            Element::text(value).font_family(family).font_size(12.0).foreground(color)
        };
        let mut lines = Vec::new();
        if !title.is_empty() {
            lines.push(text(title, style.text, FontFamily::SansMedium));
        }
        // Rows with a value keep it at the far edge of a tooltip wide enough to read across.
        let valued = rows.iter().any(|(_, _, value)| !value.is_empty());
        lines.extend(rows.into_iter().map(|(swatch, name, value)| {
            let mut row = vec![
                Element::spacer().size(Some(8.0), Some(8.0)).background(swatch.unwrap_or(Color::TRANSPARENT)).radius(2.0).centered_y(),
                text(&name, if valued { style.muted } else { style.text }, FontFamily::Sans),
            ];
            if valued {
                row.extend([Element::spacer().flex(1.0).min_size(16.0, 0.0), text(&value, style.text, FontFamily::Sans)]);
            }
            Element::row(row).gap(6.0)
        }));
        let tip = Element::panel(Element::column(lines).gap(5.0))
            .padding(9.0)
            .background(style.background)
            .border(style.border, 1.0)
            .radius(crate::tokens().radius.control)
            .shadow(style.shadow, Point::new(0.0, 6.0), 18.0, 0.0);
        let size = self.measure_uncached(&tip, Size::new(280.0, 4096.0));
        let size = Size::new(size.width.max(if valued { 150.0 } else { 0.0 }), size.height);
        let x = if pointer.x < around.x + around.width / 2.0 { pointer.x + 10.0 } else { pointer.x - 10.0 - size.width };
        let y = if pointer.y < around.y + around.height / 2.0 { pointer.y + 10.0 } else { pointer.y - 10.0 - size.height };
        let inside = |at: f32, length: f32, from: f32, room: f32| at.min(from + room - length - 4.0).max(from + 4.0);
        let window = self.viewport;
        self.overlays.push_back(DeferredOverlay::Free {
            floating: tip,
            area: Rect::new(
                inside(x, size.width, window.x, window.width).round(),
                inside(y, size.height, window.y, window.height).round(),
                size.width,
                size.height,
            ),
        });
    }

    /// A filled circle.
    fn plot_dot(&mut self, center: Point, diameter: f32, color: Color) {
        self.frame.commands.push(DrawCommand::FillRect {
            rect: Rect::new(center.x - diameter / 2.0, center.y - diameter / 2.0, diameter, diameter),
            color,
            radius: diameter / 2.0,
        });
    }

    /// The dot a series shows where the pointer reads it: a soft ring, a rim
    /// of the ground, and the dot.
    fn plot_mark(&mut self, center: Point, color: Color, ground: Color, focus: f32) {
        self.plot_dot(center, 8.0 + 12.0 * focus, faded(color, 0.2 * focus));
        self.plot_dot(center, 10.0, faded(ground, focus));
        self.plot_dot(center, 8.0, faded(color, focus));
    }

    pub(super) fn paint_line_plot(&mut self, plot: &crate::LinePlot, content: Rect, clip: Option<Rect>) {
        let count = plot.points.len().max(plot.series.iter().map(|series| series.values.len()).max().unwrap_or(0));
        if count == 0 || plot.series.is_empty() || content.width <= 0.0 || content.height <= 0.0 {
            return;
        }
        let value = |series: usize, point: usize| {
            plot.series[series].values.get(point).copied().filter(|value| value.is_finite()).unwrap_or(0.0)
        };
        let (low, high, step, ticks) = match plot.range.filter(|(low, high)| low < high) {
            Some((low, high)) => {
                let ticks = if low < 0.0 && high > 0.0 { vec![low, 0.0, high] } else { vec![low, high] };
                (low, high, high - low, ticks)
            }
            None => {
                let (mut low, mut high) = (0.0f64, 0.0f64);
                for series in 0..plot.series.len() {
                    for point in 0..count {
                        low = low.min(value(series, point));
                        high = high.max(value(series, point));
                    }
                }
                if high - low <= 0.0 {
                    high = 1.0;
                }
                let whole = (0..plot.series.len()).all(|series| (0..count).all(|point| is_whole(value(series, point))));
                let step = tick_step(high - low, whole);
                let (low, high) = ((low / step).floor() * step, (high / step).ceil() * step);
                let ticks = (0..=((high - low) / step).round() as usize).map(|index| low + index as f64 * step).collect();
                (low, high, step, ticks)
            }
        };
        let signed = plot.range.is_some() && low < 0.0;
        let tick_text = |tick: f64| if signed && tick > 0.0 { format!("+{}", written(tick)) } else { written(tick) };

        let names: f32 = if plot.point_axis {
            plot.points.iter().map(|name| self.axis_text_width(name)).fold(0.0, f32::max)
        } else {
            0.0
        };
        let left = if plot.value_axis {
            ticks.iter().map(|tick| self.axis_text_width(&tick_text(*tick))).fold(0.0, f32::max) + 8.0
        } else {
            8.0
        };
        // Half a name of room at the end, for the last point's.
        let area = Rect::new(
            content.x + left,
            content.y + 8.0,
            (content.width - left - (names / 2.0).max(8.0)).max(1.0),
            (content.height - 8.0 - if plot.point_axis { AXIS_GUTTER } else { 8.0 }).max(1.0),
        );
        let pitch = if count > 1 { area.width / (count - 1) as f32 } else { 0.0 };
        let x_of = |point: f32| if count > 1 { area.x + point * pitch } else { area.x + area.width / 2.0 };
        let y_of = |value: f64| area.y + area.height - ((value - low) / (high - low)) as f32 * area.height;

        let appear = self.appearing(&format!("{}/appear", plot.id));
        let over = self
            .pointer
            .filter(|pointer| self.hovered == Some(plot.id.as_str()) && area.contains(*pointer) && appear >= 0.999)
            .map(|pointer| if count > 1 { ((pointer.x - area.x) / pitch).round().clamp(0.0, count as f32 - 1.0) as usize } else { 0 });
        let hover = crate::tokens().motion.hover_ms;
        let focus = self.spring_paced(&format!("{}/focus", plot.id), if over.is_some() { 1.0 } else { 0.0 }, hover, hover).clamp(0.0, 1.0);
        let at_key = format!("{}/at", plot.id);
        let resting = self.springs.borrow().get(&at_key).map(|spring| spring.value.target());
        let read = over.map_or(resting.unwrap_or(0.0), |index| index as f32);
        let glide = self.spring_paced(&at_key, read, hover, hover);

        for tick in &ticks {
            let line = y_of(*tick).round() + 0.5;
            if plot.grid && tick.abs() > step * 1e-6 {
                let points = [Point::new(area.x, line), Point::new(area.x + area.width, line)];
                self.frame.commands.push(DrawCommand::Polyline { points: points.into(), color: plot.line, width: 1.0, dashed: true });
            }
            if plot.value_axis {
                self.axis_text(&tick_text(*tick), plot.muted, area.x - 8.0, line - AXIS_TEXT * 0.7, 1.0, clip);
            }
        }
        let zero = y_of(0.0).round() + 0.5;
        let base = [Point::new(area.x, zero), Point::new(area.x + area.width, zero)];
        self.frame.commands.push(DrawCommand::Polyline { points: base.into(), color: plot.line, width: 1.0, dashed: plot.dashed_zero });

        // The lines are drawn from the left as the plot appears: a clip that widens.
        let shown = Rect::new(area.x - 6.0, content.y, (area.width + 12.0) * appear, content.height);
        let shown = clip.map_or(Some(shown), |clip| shown.intersection(clip));
        if let Some(shown) = shown {
            self.frame.commands.push(DrawCommand::PushClip(shown));
            for (index, series) in plot.series.iter().enumerate() {
                let knots: Vec<Point> = (0..count).map(|point| Point::new(x_of(point as f32), y_of(value(index, point)))).collect();
                let path = if plot.smooth { rounded_through(&knots) } else { knots.clone() };
                if plot.area && path.len() > 1 {
                    // A strip under each stroke of the line, from its color down to nothing at the zero line.
                    let mut points = Vec::with_capacity(path.len() * 2);
                    let mut colors = Vec::with_capacity(path.len() * 2);
                    let mut indices = Vec::with_capacity(path.len() * 6);
                    for (at, point) in path.iter().enumerate() {
                        points.extend([*point, Point::new(point.x, zero)]);
                        colors.extend([faded(series.color, 0.32), faded(series.color, 0.02)]);
                        if at > 0 {
                            let corner = (at * 2) as u32;
                            indices.extend([corner - 2, corner - 1, corner, corner - 1, corner + 1, corner]);
                        }
                    }
                    self.frame.commands.push(DrawCommand::FillMesh { points: points.into(), colors: colors.into(), indices: indices.into() });
                }
                self.frame.commands.push(DrawCommand::Polyline { points: path.into(), color: series.color, width: 2.0, dashed: false });
                if plot.dots {
                    for knot in &knots {
                        self.plot_dot(*knot, 7.0, plot.ground);
                        self.plot_dot(*knot, 5.0, series.color);
                    }
                }
                if let Some(knot) = plot.mark.and_then(|point| knots.get(point)) {
                    self.plot_mark(*knot, series.color, plot.ground, 1.0);
                }
            }
            self.frame.commands.push(DrawCommand::PopClip);
        }

        let every = ((names + 12.0) / pitch.max(1.0)).ceil().max(1.0) as usize;
        let named = if plot.point_axis { plot.points.len() } else { 0 };
        for (point, name) in plot.points.iter().take(named).enumerate().filter(|(point, _)| point % every == 0) {
            self.axis_text(name, plot.muted, x_of(point as f32), area.y + area.height + 5.0, 0.5, clip);
        }

        // Under the pointer: a line at the point it reads, and a dot on each series there.
        if focus > 0.01 {
            let x = x_of(glide).round() + 0.5;
            let line = [Point::new(x, area.y), Point::new(x, area.y + area.height)];
            self.frame.commands.push(DrawCommand::Polyline { points: line.into(), color: faded(plot.text, 0.35 * focus), width: 1.0, dashed: true });
            let point = read.round().clamp(0.0, count as f32 - 1.0) as usize;
            for (index, series) in plot.series.iter().enumerate() {
                self.plot_mark(Point::new(x_of(glide), y_of(value(index, point))), series.color, plot.ground, focus);
            }
        }
        self.add_region(&plot.id, area, HitKind::Hover, clip);
        let (Some(point), Some(pointer)) = (over, self.pointer) else { return };
        let rows = plot
            .series
            .iter()
            .enumerate()
            .map(|(index, series)| (Some(series.color), series.name.clone(), written(value(index, point))))
            .collect();
        self.plot_tooltip(plot.tooltip, plot.points.get(point).map_or("", String::as_str), rows, area, pointer);
    }

    pub(super) fn paint_pie_plot(&mut self, plot: &crate::PiePlot, content: Rect, clip: Option<Rect>) {
        let value = |slice: &crate::PieSlice| if slice.value.is_finite() { slice.value.max(0.0) } else { 0.0 };
        let total: f64 = plot.slices.iter().map(value).sum();
        if total <= 0.0 || content.width <= 0.0 || content.height <= 0.0 {
            return;
        }
        // Room around the circle for the slice under the pointer to stand out.
        const LIFT: f32 = 6.0;
        let center = Point::new(content.x + content.width / 2.0, content.y + content.height / 2.0);
        let outer = (content.width.min(content.height) / 2.0 - LIFT - 2.0).max(4.0);
        let inner = outer * plot.hole.clamp(0.0, 0.9);
        let appear = self.appearing(&format!("{}/appear", plot.id));
        let turn = std::f32::consts::TAU;
        // Each slice's start and end, as shares of the whole turn.
        let mut spans = Vec::with_capacity(plot.slices.len());
        let mut from = 0.0f32;
        for slice in &plot.slices {
            let share = (value(slice) / total) as f32;
            spans.push((from, from + share));
            from += share;
        }
        let over = self.pointer.filter(|_| self.hovered == Some(plot.id.as_str()) && appear >= 0.999).and_then(|pointer| {
            let (dx, dy) = (pointer.x - center.x, pointer.y - center.y);
            let distance = dx.hypot(dy);
            // Clockwise from twelve o'clock, as the slices are laid.
            let share = (dx.atan2(-dy) / turn).rem_euclid(1.0);
            (distance >= inner && distance <= outer + LIFT)
                .then(|| spans.iter().position(|(from, to)| share >= *from && share < *to))
                .flatten()
        });
        let hover = crate::tokens().motion.hover_ms;
        let focus = self.spring_paced(&format!("{}/focus", plot.id), if over.is_some() { 1.0 } else { 0.0 }, hover, hover).clamp(0.0, 1.0);
        let gap = if plot.slices.iter().filter(|slice| value(slice) > 0.0).count() > 1 { 1.0 / outer } else { 0.0 };
        for (index, (slice, (from, to))) in plot.slices.iter().zip(&spans).enumerate() {
            let lifted = self
                .spring_paced(&format!("{}/slice/{index}", plot.id), if over == Some(index) { 1.0 } else { 0.0 }, hover, hover * 2)
                .clamp(0.0, 1.2);
            // The plot sweeps round as it appears, and slices part by a hair.
            let (from, to) = (from * appear * turn + gap, to * appear * turn - gap);
            if to <= from {
                continue;
            }
            let middle = (from + to) / 2.0;
            let center = Point::new(center.x + middle.sin() * LIFT * lifted, center.y - middle.cos() * LIFT * lifted);
            let color = faded(slice.color, 1.0 - 0.4 * focus * (1.0 - lifted).clamp(0.0, 1.0));
            let steps = (((to - from) * outer / 3.0).ceil() as usize).clamp(2, 240);
            let at = |radius: f32, angle: f32| Point::new(center.x + angle.sin() * radius, center.y - angle.cos() * radius);
            let mut points = Vec::with_capacity((steps + 1) * 2);
            let mut indices = Vec::with_capacity(steps * 6);
            for step in 0..=steps {
                let angle = from + (to - from) * step as f32 / steps as f32;
                points.extend([at(outer, angle), at(inner, angle)]);
                if step > 0 {
                    let corner = (step * 2) as u32;
                    indices.extend([corner - 2, corner - 1, corner, corner - 1, corner + 1, corner]);
                }
            }
            // The mesh has hard edges: a stroke of the same color round it softens them.
            let mut rim: Vec<Point> = points.iter().step_by(2).copied().collect();
            rim.extend(points.iter().skip(1).step_by(2).rev().copied());
            rim.push(points[0]);
            let colors = vec![color; points.len()];
            self.frame.commands.push(DrawCommand::FillMesh { points: points.into(), colors: colors.into(), indices: indices.into() });
            self.frame.commands.push(DrawCommand::Polyline { points: rim.into(), color, width: 1.0, dashed: false });
        }
        // What a ring says in its middle.
        if inner > 0.0 && appear > 0.0 {
            let title = Element::text(&plot.title).font_family(FontFamily::SansMedium).font_size((inner * 0.42).clamp(12.0, 28.0)).foreground(faded(plot.text, appear));
            let size = self.measure_uncached(&title, Size::new(4096.0, 4096.0));
            let caption_height = if plot.caption.is_empty() { 0.0 } else { AXIS_TEXT + 6.0 };
            let top = center.y - (size.height + caption_height) / 2.0;
            if !plot.title.is_empty() {
                self.place(&title, Rect::new((center.x - size.width / 2.0).round(), top.round(), size.width, size.height), clip);
            }
            if !plot.caption.is_empty() {
                self.axis_text(&plot.caption, faded(plot.muted, appear), center.x, top + size.height + 4.0, 0.5, clip);
            }
        }
        let bounds = Rect::new(center.x - outer - LIFT, center.y - outer - LIFT, 2.0 * (outer + LIFT), 2.0 * (outer + LIFT));
        self.add_region(&plot.id, bounds, HitKind::Hover, clip);
        let (Some(index), Some(pointer)) = (over, self.pointer) else { return };
        let slice = &plot.slices[index];
        let said = slice.label.clone().unwrap_or_else(|| written(value(slice)));
        let share = value(slice) / total * 100.0;
        self.plot_tooltip(plot.tooltip, "", vec![(Some(slice.color), slice.name.clone(), format!("{said} ({share:.1} %)"))], bounds, pointer);
    }

    pub(super) fn paint_radar_plot(&mut self, plot: &crate::RadarPlot, content: Rect, clip: Option<Rect>) {
        let count = plot.axes.len();
        if count < 3 || plot.series.is_empty() || content.width <= 0.0 || content.height <= 0.0 {
            return;
        }
        let value = |series: usize, axis: usize| {
            plot.series[series].values.get(axis).copied().filter(|value| value.is_finite()).map_or(0.0, |value| value.max(0.0))
        };
        let greatest = (0..plot.series.len()).flat_map(|series| (0..count).map(move |axis| value(series, axis))).fold(0.0, f64::max);
        let maximum = plot.maximum.filter(|maximum| maximum.is_finite() && *maximum > 0.0).unwrap_or_else(|| {
            let step = tick_step(greatest.max(1e-9), false);
            (greatest / step).ceil().max(1.0) * step
        });
        let names: f32 = plot.axes.iter().map(|name| self.axis_text_width(name)).fold(0.0, f32::max);
        let center = Point::new(content.x + content.width / 2.0, content.y + content.height / 2.0);
        let radius = ((content.width / 2.0 - names - 12.0).min(content.height / 2.0 - AXIS_TEXT - 14.0)).max(8.0);
        let turn = std::f32::consts::TAU;
        let at = |axis: f32, reach: f32| {
            let angle = axis / count as f32 * turn;
            Point::new(center.x + angle.sin() * radius * reach, center.y - angle.cos() * radius * reach)
        };
        // Rings and spokes.
        const RINGS: usize = 4;
        for ring in 1..=RINGS {
            let mut points: Vec<Point> = (0..count).map(|axis| at(axis as f32, ring as f32 / RINGS as f32)).collect();
            points.push(points[0]);
            self.frame.commands.push(DrawCommand::Polyline { points: points.into(), color: plot.line, width: 1.0, dashed: false });
        }
        for axis in 0..count {
            let spoke = [center, at(axis as f32, 1.0)];
            self.frame.commands.push(DrawCommand::Polyline { points: spoke.into(), color: plot.line, width: 1.0, dashed: false });
        }
        for (axis, name) in plot.axes.iter().enumerate() {
            let rim = at(axis as f32, 1.0);
            let side = ((rim.x - center.x) / radius).clamp(-1.0, 1.0);
            // Beside the rim, on the side of the center it is on.
            let align = if side.abs() < 0.2 { 0.5 } else if side > 0.0 { 0.0 } else { 1.0 };
            let up = (center.y - rim.y) / radius;
            self.axis_text(name, plot.muted, rim.x + side * 8.0, rim.y - AXIS_TEXT * 0.7 - up * (AXIS_TEXT * 0.7 + 4.0), align, clip);
        }

        let appear = self.appearing(&format!("{}/appear", plot.id));
        let reach = |series: usize, axis: usize| ((value(series, axis) / maximum) as f32).clamp(0.0, 1.0) * appear;
        for (index, series) in plot.series.iter().enumerate() {
            // A fan of triangles from the center to each side of the shape.
            let mut points = vec![center];
            points.extend((0..count).map(|axis| at(axis as f32, reach(index, axis))));
            let indices: Vec<u32> = (0..count as u32).flat_map(|axis| [0, axis + 1, (axis + 1) % count as u32 + 1]).collect();
            let colors = vec![faded(series.color, 0.24); points.len()];
            let mut outline: Vec<Point> = points[1..].to_vec();
            outline.push(points[1]);
            self.frame.commands.push(DrawCommand::FillMesh { points: points.into(), colors: colors.into(), indices: indices.into() });
            self.frame.commands.push(DrawCommand::Polyline { points: outline.into(), color: series.color, width: 2.0, dashed: false });
        }

        let bounds = Rect::new(center.x - radius - 12.0, center.y - radius - 12.0, 2.0 * radius + 24.0, 2.0 * radius + 24.0);
        let over = self.pointer.filter(|pointer| self.hovered == Some(plot.id.as_str()) && bounds.contains(*pointer) && appear >= 0.999).map(|pointer| {
            let share = ((pointer.x - center.x).atan2(-(pointer.y - center.y)) / turn).rem_euclid(1.0);
            (share * count as f32).round() as usize % count
        });
        let hover = crate::tokens().motion.hover_ms;
        let focus = self.spring_paced(&format!("{}/focus", plot.id), if over.is_some() { 1.0 } else { 0.0 }, hover, hover).clamp(0.0, 1.0);
        let at_key = format!("{}/at", plot.id);
        let resting = self.springs.borrow().get(&at_key).map(|spring| spring.value.target());
        let read = over.map_or(resting.unwrap_or(0.0), |axis| axis as f32).round().clamp(0.0, count as f32 - 1.0) as usize;
        self.spring_paced(&at_key, read as f32, hover, hover);
        if focus > 0.01 {
            let spoke = [center, at(read as f32, 1.0)];
            self.frame.commands.push(DrawCommand::Polyline { points: spoke.into(), color: faded(plot.text, 0.4 * focus), width: 1.0, dashed: false });
            for (index, series) in plot.series.iter().enumerate() {
                self.plot_mark(at(read as f32, reach(index, read)), series.color, plot.ground, focus);
            }
        }
        self.add_region(&plot.id, bounds, HitKind::Hover, clip);
        let (Some(axis), Some(pointer)) = (over, self.pointer) else { return };
        let rows = plot
            .series
            .iter()
            .enumerate()
            .map(|(index, series)| (Some(series.color), series.name.clone(), written(value(index, axis))))
            .collect();
        self.plot_tooltip(plot.tooltip, &plot.axes[axis], rows, bounds, pointer);
    }

    pub(super) fn paint_bar_plot(&mut self, plot: &BarPlot, content: Rect, clip: Option<Rect>) {
        let count = plot.categories.len().max(plot.series.iter().map(|series| series.values.len()).max().unwrap_or(0));
        if count == 0 || plot.series.is_empty() || content.width <= 0.0 || content.height <= 0.0 {
            return;
        }
        let value = |series: usize, category: usize| {
            plot.series[series].values.get(category).map_or(0.0, |bar| if bar.value.is_finite() { bar.value } else { 0.0 })
        };
        let written_for = |series: usize, category: usize| {
            let bar = plot.series[series].values.get(category);
            bar.and_then(|bar| bar.label.clone()).unwrap_or_else(|| written(value(series, category)))
        };

        // The values' reach: zero is always in it, and it ends on a tick.
        let (mut low, mut high) = (0.0f64, 0.0f64);
        for category in 0..count {
            let (mut above, mut below) = (0.0, 0.0);
            for series in 0..plot.series.len() {
                let value = value(series, category);
                if plot.stacked {
                    if value >= 0.0 { above += value } else { below += value }
                } else {
                    above = f64::max(above, value);
                    below = f64::min(below, value);
                }
            }
            high = high.max(above);
            low = low.min(below);
        }
        if high - low <= 0.0 {
            high = 1.0;
        }
        let whole = (0..plot.series.len()).all(|series| (0..count).all(|category| is_whole(value(series, category))));
        let step = tick_step(high - low, whole);
        let (low, high) = ((low / step).floor() * step, (high / step).ceil() * step);
        let ticks: Vec<f64> = (0..=((high - low) / step).round() as usize).map(|index| low + index as f64 * step).collect();

        // The plot's own rectangle, inside the room the axes' text takes.
        let vertical = !plot.horizontal;
        let names: f32 = plot.categories.iter().map(|name| self.axis_text_width(name)).fold(0.0, f32::max);
        let tick_room = ticks.iter().map(|tick| self.axis_text_width(&written(*tick))).fold(0.0, f32::max) + 8.0;
        let left = match (vertical, plot.value_axis) {
            (true, true) => tick_room,
            (true, false) => 0.0,
            (false, _) => names.min(content.width * 0.3) + 8.0,
        };
        let bottom = if vertical || plot.value_axis { AXIS_GUTTER } else { 0.0 };
        let top = if vertical && plot.value_labels { 16.0 } else { 6.0 };
        let right = if !vertical && plot.value_labels {
            (0..count).map(|category| self.axis_text_width(&written_for(0, category))).fold(0.0, f32::max) + 8.0
        } else if !vertical {
            tick_room / 2.0
        } else {
            0.0
        };
        let area = Rect::new(
            content.x + left,
            content.y + top,
            (content.width - left - right).max(1.0),
            (content.height - top - bottom).max(1.0),
        );
        let reach = if vertical { area.height } else { area.width };
        let share = |value: f64| ((value - low) / (high - low)) as f32;
        // Where a value stands along the value axis.
        let at = |value: f64| if vertical { area.y + area.height - share(value) * reach } else { area.x + share(value) * reach };
        let band = (if vertical { area.width } else { area.height }) / count as f32;
        let band_start = |category: f32| (if vertical { area.x } else { area.y }) + category * band;

        // Motion: the bars grow as the plot appears, and the band under the pointer glides.
        let appear = self.appearing(&format!("{}/appear", plot.id));
        let over = self
            .pointer
            .filter(|pointer| self.hovered == Some(plot.id.as_str()) && area.contains(*pointer) && appear >= 0.999)
            .map(|pointer| (((if vertical { pointer.x } else { pointer.y }) - band_start(0.0)) / band).floor().clamp(0.0, count as f32 - 1.0) as usize);
        let hover = crate::tokens().motion.hover_ms;
        let focus = self.spring_paced(&format!("{}/focus", plot.id), if over.is_some() { 1.0 } else { 0.0 }, hover, hover).clamp(0.0, 1.0);
        let band_key = format!("{}/band", plot.id);
        let resting = self.springs.borrow().get(&band_key).map(|spring| spring.value.target());
        let lit = self.spring_paced(&band_key, over.map_or(resting.unwrap_or(0.0), |index| index as f32), hover, hover);

        // The grid and the ticks.
        for tick in &ticks {
            let line = at(*tick).round() + 0.5;
            if plot.grid && tick.abs() > step * 1e-6 {
                let points = if vertical {
                    [Point::new(area.x, line), Point::new(area.x + area.width, line)]
                } else {
                    [Point::new(line, area.y), Point::new(line, area.y + area.height)]
                };
                self.frame.commands.push(DrawCommand::Polyline { points: points.into(), color: plot.line, width: 1.0, dashed: true });
            }
            if plot.value_axis {
                if vertical {
                    self.axis_text(&written(*tick), plot.muted, area.x - 8.0, line - AXIS_TEXT * 0.7, 1.0, clip);
                } else {
                    self.axis_text(&written(*tick), plot.muted, line, area.y + area.height + 5.0, 0.5, clip);
                }
            }
        }

        // The band of the category under the pointer, behind its bars.
        let group = (band * BAND_FILL).max(1.0);
        if focus > 0.01 {
            let start = band_start(lit) + (band - group) / 2.0 - 4.0;
            let rect = if vertical {
                Rect::new(start, area.y, group + 8.0, area.height)
            } else {
                Rect::new(area.x, start, area.width, group + 8.0)
            };
            self.frame.commands.push(DrawCommand::FillRect { rect, color: faded(plot.text, 0.07 * focus), radius: 4.0 });
        }

        // The bars.
        let zero = at(0.0);
        let beside = if plot.stacked { 1 } else { plot.series.len() };
        let gap = if beside > 1 { 2.0 } else { 0.0 };
        let bar = ((group - gap * (beside as f32 - 1.0)) / beside as f32).clamp(1.0, plot.max_bar.max(1.0));
        let drawn = bar * beside as f32 + gap * (beside as f32 - 1.0);
        for category in 0..count {
            let dim = 1.0 - DIM * focus * (category as f32 - lit).abs().min(1.0);
            let first = band_start(category as f32) + (band - drawn) / 2.0;
            let mut piles = (0.0f64, 0.0f64);
            // The last series to add to a pile is the one whose tip is round.
            let last = |positive: bool| (0..plot.series.len()).rev().find(|series| {
                let value = value(*series, category);
                value != 0.0 && (value > 0.0) == positive
            });
            for series in 0..plot.series.len() {
                let value = value(series, category);
                let color = plot.series[series].values.get(category).and_then(|bar| bar.color).unwrap_or(plot.series[series].color);
                let (from, to) = if plot.stacked {
                    let pile = if value >= 0.0 { &mut piles.0 } else { &mut piles.1 };
                    let from = *pile;
                    *pile += value;
                    (from, *pile)
                } else {
                    (0.0, value)
                };
                // Grown from the zero line by how far the plot has appeared.
                let (from, to) = (zero + (at(from) - zero) * appear, zero + (at(to) - zero) * appear);
                let across = first + if plot.stacked { 0.0 } else { series as f32 * (bar + gap) };
                let length = (to - from).abs();
                let rect = if vertical {
                    Rect::new(across, from.min(to), bar, length)
                } else {
                    Rect::new(from.min(to), across, length, bar)
                };
                let tip = match (vertical, value >= 0.0) {
                    (true, true) => Tip::Up,
                    (true, false) => Tip::Down,
                    (false, true) => Tip::Right,
                    (false, false) => Tip::Left,
                };
                let round = !plot.stacked || last(value > 0.0) == Some(series);
                paint_bar(&mut self.frame.commands, rect, faded(color, dim), tip, if round { plot.radius } else { 0.0 }, plot.gradient);
                if plot.value_labels && !plot.stacked && appear > 0.0 {
                    let ink = faded(plot.text, appear * dim);
                    if vertical {
                        let y = if value >= 0.0 { to - AXIS_TEXT - 5.0 } else { to + 3.0 };
                        self.axis_text(&written_for(series, category), ink, across + bar / 2.0, y, 0.5, clip);
                    } else {
                        let (x, align) = if value >= 0.0 { (to + 6.0, 0.0) } else { (to - 6.0, 1.0) };
                        self.axis_text(&written_for(series, category), ink, x, across + bar / 2.0 - AXIS_TEXT * 0.7, align, clip);
                    }
                }
            }
            if plot.value_labels && plot.stacked && appear > 0.0 {
                let end = zero + (at(piles.0) - zero) * appear;
                let ink = faded(plot.text, appear * dim);
                if vertical {
                    self.axis_text(&written(piles.0 + piles.1), ink, first + bar / 2.0, end - AXIS_TEXT - 5.0, 0.5, clip);
                } else {
                    self.axis_text(&written(piles.0 + piles.1), ink, end + 6.0, first + bar / 2.0 - AXIS_TEXT * 0.7, 0.0, clip);
                }
            }
        }

        // The zero line, over the bars' feet.
        let line = zero.round() + 0.5;
        let points = if vertical {
            [Point::new(area.x, line), Point::new(area.x + area.width, line)]
        } else {
            [Point::new(line, area.y), Point::new(line, area.y + area.height)]
        };
        self.frame.commands.push(DrawCommand::Polyline { points: points.into(), color: plot.line, width: 1.0, dashed: false });

        // The categories' names: as many as fit without touching.
        if vertical {
            let every = ((names + 10.0) / band).ceil().max(1.0) as usize;
            for (category, name) in plot.categories.iter().enumerate().filter(|(category, _)| category % every == 0) {
                self.axis_text(name, plot.muted, band_start(category as f32) + band / 2.0, area.y + area.height + 5.0, 0.5, clip);
            }
        } else {
            let every = ((AXIS_TEXT + 6.0) / band).ceil().max(1.0) as usize;
            for (category, name) in plot.categories.iter().enumerate().filter(|(category, _)| category % every == 0) {
                self.axis_text(name, plot.muted, area.x - 8.0, band_start(category as f32) + band / 2.0 - AXIS_TEXT * 0.7, 1.0, clip);
            }
        }

        // The plot answers the pointer over its whole area, not bar by bar.
        self.add_region(&plot.id, area, HitKind::Hover, clip);
        let (Some(category), Some(pointer)) = (over, self.pointer) else { return };
        let mut rows = Vec::new();
        for (index, series) in plot.series.iter().enumerate() {
            let color = series.values.get(category).and_then(|bar| bar.color).unwrap_or(series.color);
            let name = series.values.get(category).and_then(|bar| bar.name.as_deref()).unwrap_or(&series.name);
            rows.push((Some(color), name.to_owned(), written_for(index, category)));
        }
        if let Some(total) = plot.total.as_deref().filter(|_| plot.series.len() > 1) {
            let sum: f64 = (0..plot.series.len()).map(|series| value(series, category)).sum();
            rows.push((None, total.to_owned(), written(sum)));
        }
        let title = plot.categories.get(category).map_or("", String::as_str);
        self.plot_tooltip(plot.tooltip, title, rows, area, pointer);
    }

    pub(super) fn paint_heat_plot(&mut self, plot: &crate::HeatPlot, content: Rect, clip: Option<Rect>) {
        let count = plot.columns.len();
        let pitch = plot.side + plot.gap;
        if count == 0 || plot.side <= 0.0 {
            return;
        }
        let appear = self.appearing(&format!("{}/appear", plot.id));
        let cell_at = |column: usize, row: usize| plot.columns.get(column).and_then(|cells| cells.get(row)).and_then(Option::as_ref);
        // The cell the pointer is in, counting the gap around it as its own.
        let over = self
            .pointer
            .filter(|pointer| self.hovered == Some(plot.id.as_str()) && content.contains(*pointer) && appear >= 0.999)
            .map(|pointer| (((pointer.x - content.x + plot.gap / 2.0) / pitch) as usize, ((pointer.y - content.y + plot.gap / 2.0) / pitch) as usize))
            .filter(|(column, row)| cell_at(*column, *row).is_some());
        let hover = crate::tokens().motion.hover_ms;
        let focus = self.spring_paced(&format!("{}/focus", plot.id), if over.is_some() { 1.0 } else { 0.0 }, hover, hover).clamp(0.0, 1.0);
        let mut raised = Vec::new();
        for (column, cells) in plot.columns.iter().enumerate() {
            // One column after another: each waits its turn, then comes in over the same time.
            let turn = (appear * 1.5 - column as f32 / count as f32 * 0.5).clamp(0.0, 1.0);
            for (row, cell) in cells.iter().enumerate() {
                let Some(cell) = cell else { continue };
                let key = format!("{}/cell/{column}/{row}", plot.id);
                let under = over == Some((column, row));
                // Only the cells the pointer has been over have a height to come down from.
                let lift = if under || self.springs.borrow().contains_key(&key) {
                    let lift = self.spring_paced(&key, if under { 1.0 } else { 0.0 }, hover, hover * 2);
                    if under || lift.abs() > 0.004 { lift } else { 0.0 }
                } else {
                    0.0
                };
                let side = plot.side * (0.6 + 0.4 * turn) * (1.0 + 0.45 * lift);
                let center = Point::new(content.x + column as f32 * pitch + plot.side / 2.0, content.y + row as f32 * pitch + plot.side / 2.0);
                let strength = turn * (1.0 - 0.4 * focus * (1.0 - lift).clamp(0.0, 1.0));
                let command = DrawCommand::FillRect {
                    rect: Rect::new(center.x - side / 2.0, center.y - side / 2.0, side, side),
                    color: faded(cell.color, strength),
                    radius: (side * 0.2).min(3.5),
                };
                // The cell that stands over its neighbours is drawn after them.
                if lift > 0.0 { raised.push(command) } else { self.frame.commands.push(command) }
            }
        }
        self.frame.commands.extend(raised);
        self.add_region(&plot.id, content, HitKind::Hover, clip);
        let (Some((column, row)), Some(pointer)) = (over, self.pointer) else { return };
        let Some(cell) = cell_at(column, row).filter(|cell| !cell.name.is_empty()) else { return };
        self.plot_tooltip(plot.tooltip, "", vec![(Some(cell.color), cell.name.clone(), String::new())], content, pointer);
    }
}
