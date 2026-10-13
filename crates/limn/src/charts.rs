//! Read-only charts composed from the ordinary, backend-neutral element tree.
//! The host supplies data, labels and colors; charts never fetch or invent data.

use crate::{Color, Component, ComponentPalette, CornerRadii, Element, FontFamily, Tooltip};

fn label(value: impl Into<String>, color: Color, size: f32) -> Element {
    Element::truncated_text(value)
        .foreground(color)
        .font_family(FontFamily::Sans)
        .font_size(size)
}

/// A five-step intensity scale. Level zero represents a measured zero;
/// missing observations are represented by `None` in a heatmap column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatmapScale {
    pub colors: [Color; 5],
}

impl HeatmapScale {
    pub const fn new(colors: [Color; 5]) -> Self {
        Self { colors }
    }

    pub fn color(self, level: u8) -> Color {
        self.colors[usize::from(level.min(4))]
    }

    pub fn neutral(palette: ComponentPalette) -> Self {
        Self::new([
            palette.raised,
            palette.selected,
            palette.muted,
            palette.text,
            Color::WHITE,
        ])
    }
}

/// A compact intensity legend with labels at its two ends.
pub struct ChartLegend {
    colors: Vec<Color>,
    low: String,
    high: String,
    palette: ComponentPalette,
}

impl ChartLegend {
    pub fn new(
        colors: impl IntoIterator<Item = Color>,
        low: impl Into<String>,
        high: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            colors: colors.into_iter().collect(),
            low: low.into(),
            high: high.into(),
            palette,
        }
    }
}

impl Component for ChartLegend {
    fn render(self) -> Element {
        let mut items = vec![label(self.low, self.palette.muted, 11.0).size(None, Some(16.0))];
        items.extend(self.colors.into_iter().map(|color| {
            Element::column(vec![
                Element::spacer().size(None, Some(3.0)),
                Element::spacer()
                    .size(Some(10.0), Some(10.0))
                    .background(color)
                    .radius(2.0),
            ])
            .size(Some(10.0), Some(16.0))
        }));
        items.push(label(self.high, self.palette.muted, 11.0).size(None, Some(16.0)));
        Element::row(items).gap(5.0).size(None, Some(16.0))
    }
}

/// One observed heatmap cell with a stable, chart-local identity.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapCell {
    pub id: String,
    pub level: u8,
    pub tooltip: String,
}

impl HeatmapCell {
    pub fn new(id: impl Into<String>, level: u8, tooltip: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            level: level.min(4),
            tooltip: tooltip.into(),
        }
    }
}

/// A label anchored to a column, such as a month above a calendar's weeks.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartAxisLabel {
    pub column: usize,
    pub label: String,
}

impl ChartAxisLabel {
    pub fn new(column: usize, label: impl Into<String>) -> Self {
        Self {
            column,
            label: label.into(),
        }
    }
}

/// A column-major heatmap, including calendars and other discrete matrices.
/// Every column uses the same row count. `None` leaves a genuine hole (for
/// instance, days outside a calendar's period), while level zero paints a cell.
/// Set `width` to the host's content width when the viewport changes.
pub struct HeatmapChart {
    id: String,
    columns: Vec<Vec<Option<HeatmapCell>>>,
    row_labels: Vec<String>,
    column_labels: Vec<ChartAxisLabel>,
    palette: ComponentPalette,
    scale: HeatmapScale,
    width: f32,
    cell_size: f32,
    gap: f32,
}

impl HeatmapChart {
    pub fn new(
        id: impl Into<String>,
        columns: Vec<Vec<Option<HeatmapCell>>>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            columns,
            row_labels: Vec::new(),
            column_labels: Vec::new(),
            palette,
            scale: HeatmapScale::neutral(palette),
            width: 720.0,
            cell_size: 14.0,
            gap: 3.0,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }

    pub fn cell_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.cell_size = size.clamp(1.0, 40.0);
        }
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.clamp(0.0, 16.0);
        }
        self
    }

    pub fn scale(mut self, scale: HeatmapScale) -> Self {
        self.scale = scale;
        self
    }

    pub fn row_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.row_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    pub fn column_labels(mut self, labels: Vec<ChartAxisLabel>) -> Self {
        self.column_labels = labels;
        self
    }
}

impl Component for HeatmapChart {
    fn render(self) -> Element {
        let count = self.columns.len();
        let rows = self
            .columns
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or_default()
            .max(self.row_labels.len());
        if count == 0 || rows == 0 {
            return Element::spacer().size(Some(self.width), Some(0.0));
        }
        // Keep the whole period visible. At narrow widths reduce both cells
        // and gutters; omit the secondary row axis when it cannot be legible.
        let show_rows = self.row_labels.iter().any(|label| !label.is_empty())
            && self.width / count as f32 >= 11.0;
        let axis_width = if show_rows { 32.0 } else { 0.0 };
        let available = (self.width - axis_width).max(0.0);
        let gap = self.gap.min(available / count as f32 * 0.2);
        let side = ((available - gap * count.saturating_sub(1) as f32) / count as f32)
            .clamp(0.0, self.cell_size);
        let pitch = side + gap;
        let plot_width = side * count as f32 + gap * count.saturating_sub(1) as f32;

        let grid = Element::row(
            self.columns
                .into_iter()
                .map(|column| {
                    let cells = (0..rows)
                        .map(|row| {
                            let square = Element::spacer().size(Some(side), Some(side));
                            let Some(cell) = column.get(row).and_then(Option::as_ref) else {
                                return square;
                            };
                            let square = square
                                .background(self.scale.color(cell.level))
                                .radius((side * 0.2).min(2.5));
                            if cell.tooltip.is_empty() {
                                square
                            } else {
                                Tooltip::new(
                                    square,
                                    format!("{}:{}", self.id, cell.id),
                                    &cell.tooltip,
                                    self.palette,
                                )
                                .render()
                                .size(Some(side), Some(side))
                            }
                        })
                        .collect();
                    Element::column(cells).gap(gap).size(Some(side), None)
                })
                .collect(),
        )
        .gap(gap)
        .size(Some(plot_width), None);

        let grid = if show_rows {
            let axis = Element::column(
                (0..rows)
                    .map(|row| {
                        label(
                            self.row_labels.get(row).cloned().unwrap_or_default(),
                            self.palette.muted,
                            side.min(11.0),
                        )
                        .size(Some(axis_width), Some(side))
                    })
                    .collect(),
            )
            .gap(gap)
            .size(Some(axis_width), None);
            Element::row(vec![axis, grid]).size(Some(self.width), None)
        } else {
            grid
        };

        let mut labels = self.column_labels;
        labels.sort_by_key(|label| label.column);
        labels.dedup_by_key(|label| label.column);
        let mut visible: Vec<ChartAxisLabel> = Vec::new();
        for label in labels {
            let x = label.column as f32 * pitch;
            if label.column < count
                && plot_width - x >= 24.0
                && visible
                    .last()
                    .is_none_or(|previous| (label.column - previous.column) as f32 * pitch >= 32.0)
            {
                visible.push(label);
            }
        }
        let mut body = Vec::new();
        if let Some(first) = visible.first() {
            let mut axis =
                vec![Element::spacer()
                    .size(Some(axis_width + first.column as f32 * pitch), Some(18.0))];
            for (index, tick) in visible.iter().enumerate() {
                let end = visible
                    .get(index + 1)
                    .map(|next| next.column as f32 * pitch)
                    .unwrap_or(plot_width);
                axis.push(label(&tick.label, self.palette.muted, 11.0).size(
                    Some((end - tick.column as f32 * pitch).max(0.0)),
                    Some(18.0),
                ));
            }
            body.push(Element::row(axis).size(Some(self.width), Some(18.0)));
        }
        body.push(grid);
        Element::column(body).gap(5.0).size(Some(self.width), None)
    }
}

/// One non-negative observation in a bar chart. Values that are negative or
/// non-finite are displayed as zero. IDs are scoped to their chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartDatum {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub value_label: String,
    pub tooltip: Option<String>,
    pub color: Option<Color>,
}

impl ChartDatum {
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        let value = non_negative(value);
        Self {
            id: id.into(),
            label: label.into(),
            value,
            value_label: if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                format!("{value:.1}")
            },
            tooltip: None,
            color: None,
        }
    }

    pub fn value_label(mut self, label: impl Into<String>) -> Self {
        self.value_label = label.into();
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

fn non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Horizontal bars sharing a zero baseline and a single linear scale. Labels
/// and values remain visible without hovering. No click or focus stops are added.
pub struct BarChart {
    id: String,
    data: Vec<ChartDatum>,
    palette: ComponentPalette,
    width: f32,
    color: Color,
}

impl BarChart {
    pub fn new(id: impl Into<String>, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            data,
            palette,
            width: 480.0,
            color: palette.text,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Compare discrete categories with columns instead of horizontal bars.
    /// All existing data, width, palette and color settings are retained.
    pub fn vertical(self) -> ColumnChart {
        ColumnChart::new(self.id, self.data, self.palette)
            .width(self.width)
            .color(self.color)
    }
}

impl Component for BarChart {
    fn render(self) -> Element {
        let maximum = self
            .data
            .iter()
            .map(|datum| non_negative(datum.value))
            .fold(0.0, f64::max);
        let maximum = if maximum > 0.0 { maximum } else { 1.0 };
        let label_width = (self.width * 0.3).min(140.0);
        let value_width = (self.width * 0.13).min(56.0);
        let gap = 10.0_f32.min(self.width * 0.05);
        let track_width = (self.width - label_width - value_width - 2.0 * gap).max(0.0);
        let rows = self
            .data
            .into_iter()
            .map(|datum| {
                let bar_width = (non_negative(datum.value) / maximum) as f32 * track_width;
                let track = Element::row(vec![
                    Element::spacer()
                        .size(Some(bar_width), Some(10.0))
                        .background(datum.color.unwrap_or(self.color))
                        .radius(3.0),
                    Element::spacer().flex(1.0),
                ])
                .background(self.palette.raised)
                .radius(3.0)
                .size(Some(track_width), Some(10.0));
                let row = Element::row(vec![
                    label(&datum.label, self.palette.muted, 12.0)
                        .padding(4.0)
                        .size(Some(label_width), Some(28.0)),
                    Element::column(vec![Element::spacer().size(None, Some(9.0)), track])
                        .size(Some(track_width), Some(28.0)),
                    label(&datum.value_label, self.palette.text, 12.0)
                        .padding(4.0)
                        .size(Some(value_width), Some(28.0)),
                ])
                .gap(gap)
                .size(Some(self.width), Some(28.0));
                Tooltip::new(
                    row,
                    format!("{}:{}", self.id, datum.id),
                    datum
                        .tooltip
                        .unwrap_or_else(|| format!("{}: {}", datum.label, datum.value_label)),
                    self.palette,
                )
                .render()
                .size(Some(self.width), Some(28.0))
            })
            .collect();
        Element::column(rows).gap(6.0).size(Some(self.width), None)
    }
}

/// Native categorical columns with a zero baseline. Counts use integer ticks
/// by default. The host supplies observations; measured zero draws no bar.
/// Dense categories scroll inside the plot while the value axis stays fixed.
pub struct ColumnChart {
    id: String,
    data: Vec<ChartDatum>,
    palette: ComponentPalette,
    width: f32,
    height: f32,
    color: Color,
    max_bar_width: f32,
    integer_ticks: bool,
}

impl ColumnChart {
    pub fn new(id: impl Into<String>, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            data,
            palette,
            width: 480.0,
            height: 240.0,
            color: palette.text,
            max_bar_width: 48.0,
            integer_ticks: true,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.clamp(120.0, 1200.0);
        }
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn max_bar_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_bar_width = width.clamp(1.0, 120.0);
        }
        self
    }

    pub fn integer_ticks(mut self, integer: bool) -> Self {
        self.integer_ticks = integer;
        self
    }
}

struct ColumnScale {
    maximum: f64,
    ticks: Vec<f64>,
}

impl ColumnScale {
    fn new(maximum: f64, integer: bool) -> Self {
        let maximum = non_negative(maximum);
        if maximum == 0.0 {
            return Self {
                maximum: 1.0,
                ticks: vec![0.0, 1.0],
            };
        }
        let raw = maximum / 4.0;
        let magnitude = 10_f64.powf(raw.log10().floor());
        let normalized = raw / magnitude;
        let factor = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };
        let mut step = factor * magnitude;
        if integer {
            step = step.max(1.0).ceil();
        }
        let count = (maximum / step).ceil().clamp(1.0, 5.0) as usize;
        let bound = step * count as f64;
        if !step.is_finite() || step <= 0.0 || !bound.is_finite() || bound < maximum {
            // Finite extreme/subnormal observations must not create NaN geometry.
            return Self {
                maximum,
                ticks: vec![0.0, maximum],
            };
        }
        Self {
            maximum: bound,
            ticks: (0..=count).map(|index| step * index as f64).collect(),
        }
    }
}

fn tick_text(value: f64, integer: bool) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if !(0.01..1_000_000.0).contains(&value) {
        return format!("{value:.0e}");
    }
    if integer || value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

/// Grid pieces use ordinary passive elements. Splitting only the empty space
/// around a single solid bar avoids layered popovers or alpha seams in bars.
fn column_grid(from: f32, to: f32, width: f32, ticks: &[f32], color: Color) -> Element {
    let mut children = Vec::new();
    let mut y = from;
    for &tick in ticks {
        if tick < from || tick >= to {
            continue;
        }
        if tick > y {
            children.push(Element::spacer().size(Some(width), Some(tick - y)));
        }
        let line_height = (to - tick).min(1.0);
        children.push(
            Element::spacer()
                .size(Some(width), Some(line_height))
                .background(color),
        );
        y = tick + line_height;
    }
    if to > y {
        children.push(Element::spacer().size(Some(width), Some(to - y)));
    }
    Element::column(children).size(Some(width), Some((to - from).max(0.0)))
}

impl Component for ColumnChart {
    fn render(self) -> Element {
        if self.data.is_empty() || self.width == 0.0 {
            return Element::spacer().size(Some(self.width), Some(0.0));
        }
        let maximum = self
            .data
            .iter()
            .map(|datum| non_negative(datum.value))
            .fold(0.0, f64::max);
        let scale = ColumnScale::new(maximum, self.integer_ticks);
        let top = 24.0;
        let category_height = 32.0;
        let baseline = self.height - category_height;
        let plot_height = baseline - top;
        let tick_labels: Vec<_> = scale
            .ticks
            .iter()
            .map(|value| tick_text(*value, self.integer_ticks))
            .collect();
        let widest = tick_labels.iter().map(String::len).max().unwrap_or(1) as f32;
        let axis_width = (widest * 7.0 + 12.0)
            .clamp(28.0, 68.0)
            .min(self.width * 0.3);
        let plot_width = (self.width - axis_width).max(0.0);
        let content_width = plot_width.max(self.data.len() as f32 * 64.0);
        let cell_width = content_width / self.data.len() as f32;
        let bar_width = self.max_bar_width.min(cell_width * 0.56);
        let side_width = (cell_width - bar_width) * 0.5;
        let grid_color = Color {
            a: ((u16::from(self.palette.border.a) * 3) / 5) as u8,
            ..self.palette.border
        };
        let mut grid_y: Vec<_> = scale
            .ticks
            .iter()
            .map(|value| baseline - (value / scale.maximum) as f32 * plot_height)
            .collect();
        grid_y.reverse();

        let mut axis = Vec::new();
        let mut axis_y = 0.0;
        for (&y, text) in grid_y.iter().zip(tick_labels.iter().rev()) {
            let label_y = y - 8.0;
            axis.push(Element::spacer().size(Some(axis_width), Some((label_y - axis_y).max(0.0))));
            axis.push(
                label(text, self.palette.muted, 11.0)
                    .center_content()
                    .size(Some(axis_width), Some(16.0)),
            );
            axis_y = label_y + 16.0;
        }
        axis.push(Element::spacer().size(Some(axis_width), Some((self.height - axis_y).max(0.0))));
        let axis = Element::column(axis)
            .size(Some(axis_width), Some(self.height))
            .clip_children();

        let columns = self
            .data
            .into_iter()
            .map(|datum| {
                let height = (non_negative(datum.value) / scale.maximum) as f32 * plot_height;
                let bar_top = baseline - height;
                let value_height = 20.0;
                let value_top = (bar_top - value_height).max(0.0);
                let bar = Element::spacer()
                    .size(Some(bar_width), Some(height))
                    .background(datum.color.unwrap_or(self.color))
                    .corner_radii(CornerRadii::top(3.0));
                let body = Element::row(vec![
                    column_grid(bar_top, baseline, side_width, &grid_y, grid_color),
                    bar,
                    column_grid(bar_top, baseline, side_width, &grid_y, grid_color),
                ])
                .size(Some(cell_width), Some(height));
                let cell = Element::column(vec![
                    column_grid(0.0, value_top, cell_width, &grid_y, grid_color),
                    label(&datum.value_label, self.palette.text, 12.0)
                        .center_content()
                        .size(Some(cell_width), Some(value_height)),
                    body,
                    Element::spacer()
                        .size(Some(cell_width), Some(1.0))
                        .background(self.palette.border),
                    Element::spacer().size(Some(cell_width), Some(6.0)),
                    Element::row(vec![
                        Element::spacer().size(Some(4.0), Some(18.0)),
                        label(&datum.label, self.palette.muted, 12.0)
                            .center_content()
                            .size(Some(cell_width - 8.0), Some(18.0)),
                        Element::spacer().size(Some(4.0), Some(18.0)),
                    ])
                    .size(Some(cell_width), Some(18.0)),
                ])
                .size(Some(cell_width), Some(self.height));
                let description = match datum.tooltip {
                    Some(tooltip) if tooltip.contains(&datum.label) => tooltip,
                    Some(tooltip) => format!("{}\n{tooltip}", datum.label),
                    None => format!("{}: {}", datum.label, datum.value_label),
                };
                Tooltip::new(
                    cell,
                    format!("{}:{}", self.id, datum.id),
                    &datum.value_label,
                    self.palette,
                )
                .description(description)
                .max_width(320.0)
                .render()
                .size(Some(cell_width), Some(self.height))
            })
            .collect();
        let plot = Element::row(columns).size(Some(content_width), Some(self.height));
        let plot = if content_width > plot_width + 0.5 {
            Element::horizontal_scroll(format!("{}:columns", self.id), plot)
                .horizontal_edge_fade(self.palette.surface, 10.0)
                .size(Some(plot_width), Some(self.height))
        } else {
            // A chart which already fits must not consume page wheel events.
            plot.clip_children()
        };
        Element::row(vec![axis, plot]).size(Some(self.width), Some(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};
    use std::time::{Duration, Instant};

    fn column_bars(frame: &crate::Frame, color: Color) -> Vec<crate::Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillCornerRect {
                    rect, color: fill, ..
                } if *fill == color => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn columns_share_a_zero_baseline_integer_grid_and_capped_width() {
        let color = Color::rgb(71, 142, 231);
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(
            BarChart::new(
                "tasks",
                vec![
                    ChartDatum::new("a", "Alpha", 6.0),
                    ChartDatum::new("b", "Beta", 3.0),
                    ChartDatum::new("c", "Zero", 0.0),
                ],
                palette,
            )
            .width(720.0)
            .color(color)
            .vertical()
            .height(240.0)
            .max_bar_width(42.0)
            .render(),
        )
        .layout(Size::new(720.0, 240.0));
        let bars = column_bars(&frame, color);
        // The plot clip correctly culls the zero-height observation; its label
        // and passive tooltip still exist, without inventing a visible bar.
        assert_eq!(bars.len(), 2);
        assert!((bars[0].height - bars[1].height * 2.0).abs() < 0.01);
        let zero = frame
            .regions
            .iter()
            .find(|region| region.id == "tasks:c")
            .unwrap()
            .rect;
        assert!(!bars.iter().any(|bar| zero.contains(Point::new(
            bar.x + bar.width * 0.5,
            bar.y + bar.height * 0.5
        ))));
        assert!(bars
            .iter()
            .all(|bar| (bar.y + bar.height - 208.0).abs() < 0.01 && bar.width <= 42.0));
        for value in ["0", "2", "4", "6"] {
            assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Text {text,position,..} if text == value && position.x < 28.0)), "missing tick {value}");
        }
        assert!(frame
            .regions
            .iter()
            .all(|region| region.kind == HitKind::Hover));
    }

    #[test]
    fn count_scales_keep_unique_integer_ticks_and_extreme_data_finite() {
        for maximum in [
            0.0,
            1.0,
            2.0,
            3.0,
            7.0,
            11.0,
            103.0,
            8_003.0,
            f64::MAX,
            f64::NAN,
        ] {
            let scale = ColumnScale::new(maximum, true);
            assert!(scale.maximum.is_finite() && scale.maximum > 0.0);
            assert!(scale.maximum >= non_negative(maximum));
            assert_eq!(scale.ticks[0], 0.0);
            assert!(scale.ticks.len() <= 6);
            assert!(scale
                .ticks
                .iter()
                .all(|tick| tick.is_finite() && tick.fract() == 0.0));
            assert!(scale.ticks.windows(2).all(|pair| pair[0] < pair[1]));
        }
        let scale = ColumnScale::new(0.4, false);
        assert!(scale.ticks.iter().any(|tick| tick.fract() != 0.0));
        assert!(tick_text(0.2, false) == "0.2");
    }

    #[test]
    fn columns_scroll_categories_without_moving_axis_or_overflowing_hover_bounds() {
        let palette = ComponentPalette::neutral_dark();
        for width in [90.0, 180.0, 320.0, 960.0] {
            let chart = ColumnChart::new(
                "dense",
                (0..12)
                    .map(|index| {
                        ChartDatum::new(
                            index.to_string(),
                            format!("Agent {index}"),
                            (index + 1) as f64,
                        )
                    })
                    .collect(),
                palette,
            )
            .width(width)
            .render();
            let mut tree = UiTree::new(chart);
            tree.set_reduced_motion(true);
            let size = Size::new(width, 240.0);
            let before = tree.layout(size);
            let axis = before
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == "0" => Some(*position),
                    _ => None,
                })
                .unwrap();
            assert!(before
                .regions
                .iter()
                .all(|region| region.rect.x >= 0.0
                    && region.rect.x + region.rect.width <= width + 0.01));
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(width - 5.0, 100.0),
                delta_y: -10_000.0,
            });
            let after = tree.layout(size);
            assert!(after.regions.iter().any(|region| region.id == "dense:11"));
            assert!(after.commands.iter().any(|command| matches!(command, DrawCommand::Text {text,position,..} if text == "0" && *position == axis)));
            assert!(after
                .regions
                .iter()
                .all(|region| region.rect.x >= 0.0
                    && region.rect.x + region.rect.width <= width + 0.01));
        }
    }

    #[test]
    fn zero_column_tooltip_is_passive_wrapped_and_does_not_capture_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let full_name = "Agent with a deliberately long complete category name";
        let chart = ColumnChart::new(
            "hover",
            vec![ChartDatum::new("zero", full_name, 0.0)
                .tooltip("No tasks in the selected interval")],
            palette,
        )
        .width(240.0)
        .render();
        let mut tree = UiTree::new(Element::column(vec![
            chart,
            Element::button("next", "Next").size(None, Some(32.0)),
        ]));
        let size = Size::new(240.0, 400.0);
        let now = Instant::now();
        let frame = tree.layout_at(size, now);
        let region = frame
            .regions
            .iter()
            .find(|region| region.id == "hover:zero")
            .unwrap();
        let point = Point::new(region.rect.x + region.rect.width * 0.5, 210.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        tree.handle_event(UiEvent::PointerMove(point));
        let hovered = tree.layout_at(size, now + Duration::from_secs(2));
        let text = hovered
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(normalized.contains(full_name), "{normalized}");
        assert!(normalized.contains("No tasks in the selected interval"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("next"));
    }

    #[test]
    fn columns_use_supplied_light_and_dark_palettes_and_reject_nan_geometry() {
        let dark = ComponentPalette::neutral_dark();
        let light = ComponentPalette {
            surface: Color::rgb(250, 250, 250),
            border: Color::rgb(200, 205, 212),
            text: Color::rgb(25, 30, 40),
            muted: Color::rgb(90, 95, 105),
            ..dark
        };
        for palette in [dark, light] {
            let mut invalid = ChartDatum::new("invalid", "Invalid", 0.0);
            invalid.value = f64::NAN;
            let chart = ColumnChart::new(
                "finite",
                vec![invalid, ChartDatum::new("huge", "Huge", f64::MAX)],
                palette,
            )
            .width(f32::NAN)
            .height(f32::INFINITY)
            .max_bar_width(f32::NAN)
            .render();
            let frame = UiTree::new(chart).layout(Size::new(480.0, 240.0));
            let bars = column_bars(&frame, palette.text);
            assert_eq!(
                bars.len(),
                1,
                "invalid observation must not paint a visible bar"
            );
            let invalid = frame
                .regions
                .iter()
                .find(|region| region.id == "finite:invalid")
                .unwrap()
                .rect;
            assert!(!invalid.contains(Point::new(
                bars[0].x + bars[0].width * 0.5,
                bars[0].y + bars[0].height * 0.5
            )));
            assert!(bars
                .iter()
                .all(|rect| [rect.x, rect.y, rect.width, rect.height]
                    .iter()
                    .all(|value| value.is_finite())));
            assert!(frame.commands.iter().any(
                |command| matches!(command, DrawCommand::Text {color,..} if *color == palette.muted)
            ));
        }
    }

    #[test]
    fn heatmap_preserves_missing_days_and_zero_observations() {
        let palette = ComponentPalette::neutral_dark();
        let chart = HeatmapChart::new(
            "calendar",
            vec![
                vec![None, Some(HeatmapCell::new("mon", 0, "0 contributions"))],
                vec![Some(HeatmapCell::new("sun", 4, "8 contributions")), None],
            ],
            palette,
        )
        .width(90.0)
        .cell_size(12.0)
        .render();
        let frame = UiTree::new(chart).layout(Size::new(90.0, 60.0));
        let monday = frame
            .regions
            .iter()
            .find(|r| r.id == "calendar:mon")
            .unwrap();
        let sunday = frame
            .regions
            .iter()
            .find(|r| r.id == "calendar:sun")
            .unwrap();
        assert_eq!(monday.rect.y, 15.0);
        assert_eq!(sunday.rect.y, 0.0);
        assert_eq!(sunday.rect.x, 15.0);
        assert_eq!(frame.regions.len(), 2);
        assert!(frame.commands.iter().any(|command| matches!(
            command, DrawCommand::FillRect { color, .. } if *color == palette.raised
        )));
    }

    #[test]
    fn chart_tooltips_do_not_activate_or_add_keyboard_stops() {
        let palette = ComponentPalette::neutral_dark();
        let chart = HeatmapChart::new(
            "calendar",
            vec![vec![Some(HeatmapCell::new(
                "today",
                2,
                "3 contributions today",
            ))]],
            palette,
        )
        .render();
        let mut tree = UiTree::new(Element::column(vec![
            chart,
            Element::button("next", "Next"),
        ]));
        let now = Instant::now();
        let frame = tree.layout_at(Size::new(720.0, 100.0), now);
        assert_eq!(frame.regions[0].kind, HitKind::Hover);
        let point = Point::new(5.0, 5.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        tree.handle_event(UiEvent::PointerMove(point));
        let frame = tree.layout_at(Size::new(720.0, 100.0), now + Duration::from_secs(2));
        assert!(frame.commands.iter().any(|command| matches!(
            command, DrawCommand::Text { text, .. } if text == "3 contributions today"
        )));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked { id: "next".into() }]
        );
    }

    #[test]
    fn a_full_year_fits_narrow_viewports_and_bars_share_one_scale() {
        let palette = ComponentPalette::neutral_dark();
        for width in [180.0, 320.0, 720.0, 960.0] {
            let columns = (0..54)
                .map(|week| {
                    (0..7)
                        .map(|day| Some(HeatmapCell::new(format!("{week}-{day}"), 2, "day")))
                        .collect()
                })
                .collect();
            let frame = UiTree::new(
                HeatmapChart::new("year", columns, palette)
                    .width(width)
                    .row_labels(["", "Mon", "", "Wed", "", "Fri", ""])
                    .render(),
            )
            .layout(Size::new(width, 180.0));
            assert_eq!(frame.regions.len(), 378);
            assert!(frame
                .regions
                .iter()
                .all(|r| r.rect.x >= 0.0 && r.rect.x + r.rect.width <= width + 0.01));
        }
        let color = Color::rgb(80, 140, 220);
        let frame = UiTree::new(
            BarChart::new(
                "bars",
                vec![
                    ChartDatum::new("half", "Half", 5.0),
                    ChartDatum::new("full", "Full", 10.0),
                    ChartDatum::new("invalid", "Invalid", f64::NAN),
                ],
                palette,
            )
            .width(320.0)
            .color(color)
            .render(),
        )
        .layout(Size::new(320.0, 160.0));
        let widths: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect {
                    rect, color: fill, ..
                } if *fill == color => Some(rect.width),
                _ => None,
            })
            .collect();
        assert_eq!(widths.len(), 3);
        assert_eq!(widths[0] * 2.0, widths[1]);
        assert_eq!(widths[2], 0.0);
        assert!(frame.regions.iter().all(|r| r.kind == HitKind::Hover));
    }
}
