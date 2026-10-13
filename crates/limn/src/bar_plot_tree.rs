//! How the tree draws a [`BarPlot`]: its axes, its bars, and what the
//! pointer does to them.

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
/// for about four of them across `span`.
fn tick_step(span: f64) -> f64 {
    let raw = span / 4.0;
    let magnitude = 10f64.powf(raw.log10().floor());
    let share = raw / magnitude;
    magnitude * if share <= 1.0 { 1.0 } else if share <= 2.0 { 2.0 } else if share <= 5.0 { 5.0 } else { 10.0 }
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
        let step = tick_step(high - low);
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
        let text = |value: &str, color: Color, family: FontFamily| {
            Element::text(value).font_family(family).font_size(12.0).foreground(color)
        };
        let row = |swatch: Option<Color>, name: &str, value: String| {
            Element::row(vec![
                Element::spacer().size(Some(8.0), Some(8.0)).background(swatch.unwrap_or(Color::TRANSPARENT)).radius(2.0).centered_y(),
                text(name, plot.tooltip.muted, FontFamily::Sans),
                Element::spacer().flex(1.0).min_size(16.0, 0.0),
                text(&value, plot.tooltip.text, FontFamily::Sans),
            ])
            .gap(6.0)
        };
        let mut rows = Vec::new();
        if let Some(name) = plot.categories.get(category).filter(|name| !name.is_empty()) {
            rows.push(text(name, plot.tooltip.text, FontFamily::SansMedium));
        }
        for (index, series) in plot.series.iter().enumerate() {
            let color = series.values.get(category).and_then(|bar| bar.color).unwrap_or(series.color);
            let name = series.values.get(category).and_then(|bar| bar.name.as_deref()).unwrap_or(&series.name);
            rows.push(row(Some(color), name, written_for(index, category)));
        }
        if let Some(total) = plot.total.as_deref().filter(|_| plot.series.len() > 1) {
            let sum: f64 = (0..plot.series.len()).map(|series| value(series, category)).sum();
            rows.push(row(None, total, written(sum)));
        }
        let tip = Element::panel(Element::column(rows).gap(5.0))
            .padding(9.0)
            .background(plot.tooltip.background)
            .border(plot.tooltip.border, 1.0)
            .radius(crate::tokens().radius.control)
            .shadow(plot.tooltip.shadow, Point::new(0.0, 6.0), 18.0, 0.0);
        let size = self.measure_uncached(&tip, Size::new(280.0, 4096.0));
        let size = Size::new(size.width.max(150.0), size.height);
        // Beside the pointer, on the side of it where the plot has more room.
        let x = if pointer.x < area.x + area.width / 2.0 { pointer.x + 10.0 } else { pointer.x - 10.0 - size.width };
        let y = if pointer.y < area.y + area.height / 2.0 { pointer.y + 10.0 } else { pointer.y - 10.0 - size.height };
        self.overlays.push_back(DeferredOverlay::Free {
            floating: tip,
            area: Rect::new(x.round(), y.round(), size.width, size.height),
        });
    }
}
