//! Read-only charts composed from the ordinary, backend-neutral element tree.
//! The host supplies data, labels and colors; charts never fetch or invent data.

use crate::{Color, Component, ComponentPalette, Element, FontFamily};

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
    /// The five colors, level 0 first.
    pub colors: [Color; 5],
}

impl HeatmapScale {
    /// A scale from these colors, level 0 first.
    pub const fn new(colors: [Color; 5]) -> Self {
        Self { colors }
    }

    /// The color for `level`. Levels above 4 take the last color.
    pub fn color(self, level: u8) -> Color {
        self.colors[usize::from(level.min(4))]
    }

    /// A scale from the palette's own colors: raised, selected, muted and text, then white
    /// for the highest level.
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
    /// A legend with one swatch for each color, in order, between the `low` and `high`
    /// labels.
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
        let mut items = vec![label(self.low, self.palette.muted, 11.0).size(None, Some(16.0)).shrink_to_fit()];
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
        items.push(label(self.high, self.palette.muted, 11.0).size(None, Some(16.0)).shrink_to_fit());
        Element::row(items).gap(5.0).size(None, Some(16.0))
    }
}

/// One observed heatmap cell with a stable, chart-local identity.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapCell {
    /// Scopes the cell's tooltip within its chart; unique among the chart's cells.
    /// A cell with no tooltip does not use it.
    pub id: String,
    /// From 0 to 4.
    pub level: u8,
    /// Empty for a cell with no tooltip.
    pub tooltip: String,
}

impl HeatmapCell {
    /// A cell at `level`, clamped to 0 to 4. `id` scopes the cell's tooltip within its
    /// chart.
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
    /// The column the label starts at, counted from zero. A label at or past the
    /// number of columns is not drawn.
    pub column: usize,
    /// The text. It is left out when it starts under 32 pixels after the label
    /// before it, and of two labels on one column only one is drawn.
    pub label: String,
}

impl ChartAxisLabel {
    /// A label that starts at `column`, counted from zero.
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
    /// A heatmap 720 wide with cells of at most 14, a gap of 3, the neutral scale and no
    /// labels. `columns` holds one vector for each column, top row first; `id` scopes the
    /// tooltips of its cells.
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

    /// Set the total width in logical pixels (720 by default). Cells shrink so every column
    /// stays inside it. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(0.0);
        }
        self
    }

    /// Set the largest side of a cell, clamped to 1 to 40 (14 by default). Cells are
    /// smaller when the width requires it. A value that is not finite is ignored.
    pub fn cell_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.cell_size = size.clamp(1.0, 40.0);
        }
        self
    }

    /// Set the space between cells, clamped to 0 to 16 (3 by default). When drawn it is
    /// also at most a fifth of each column's share of the width. A value that is not finite
    /// is ignored.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.clamp(0.0, 16.0);
        }
        self
    }

    /// Replace the neutral color scale.
    pub fn scale(mut self, scale: HeatmapScale) -> Self {
        self.scale = scale;
        self
    }

    /// Label the rows, top first, in an axis 32 wide at the leading edge. The axis is left
    /// out when every label is empty or when the width gives each column less than 11.
    pub fn row_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.row_labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Label columns above the grid. Labels are sorted by column and one is kept for each
    /// column. A label is dropped when its column does not exist, when it starts within 24
    /// of the plot's end or within 32 of the label before it.
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

        // The cells are one plot the tree draws: it grows the one under the pointer and says what it is.
        let scale = self.scale;
        let grid = Element::heat_plot(crate::HeatPlot {
            id: self.id,
            columns: self
                .columns
                .into_iter()
                .map(|column| {
                    (0..rows)
                        .map(|row| {
                            column.get(row).and_then(Option::as_ref).map(|cell| crate::HeatCell { color: scale.color(cell.level), name: cell.tooltip.clone() })
                        })
                        .collect()
                })
                .collect(),
            side,
            gap,
            tooltip: self.palette.tooltip_style(),
        })
        .size(Some(plot_width), Some(side * rows as f32 + gap * rows.saturating_sub(1) as f32));

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

/// One observation in a bar chart. A negative one grows the other way from the
/// zero line; one that is not finite counts as zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartDatum {
    /// Scopes the datum's tooltip within its chart; unique among the chart's data.
    pub id: String,
    /// The name of what was observed.
    pub label: String,
    /// The observation. `new` stores one that is not finite as zero.
    pub value: f64,
    /// The text shown for the value. `new` writes the value with no decimals when
    /// it is whole and with one otherwise.
    pub value_label: String,
    /// `None` shows the label and the value label.
    pub tooltip: Option<String>,
    /// `None` takes the chart's color.
    pub color: Option<Color>,
}

impl ChartDatum {
    /// An observation with no tooltip and no color of its own. A `value` that is not
    /// finite becomes zero. The value label is the value with no decimals when it is
    /// whole and with one otherwise.
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        let value = finite(value);
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

    /// Replace the text shown for the value, to add a unit or a format of the host's.
    pub fn value_label(mut self, label: impl Into<String>) -> Self {
        self.value_label = label.into();
        self
    }

    /// What the tooltip calls this observation, in place of its label.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Draw this observation in a color of its own instead of the chart's.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// One series of a chart of several: a name, a value for each category, and
/// a color of its own if it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartSeries {
    /// Said beside its values in the tooltip.
    pub name: String,
    /// One for each category, in their order. A missing or non-finite one counts as zero.
    pub values: Vec<f64>,
    /// `None` takes the next step of the chart's color.
    pub color: Option<Color>,
}

impl ChartSeries {
    /// A series with no color of its own.
    pub fn new(name: impl Into<String>, values: impl IntoIterator<Item = f64>) -> Self {
        Self { name: name.into(), values: values.into_iter().collect(), color: None }
    }

    /// Draw this series in a color of its own instead of a step of the chart's.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

/// What a bar chart and a column chart share: the same plot, lying or standing.
#[derive(Clone)]
struct Bars {
    id: String,
    categories: Vec<String>,
    series: Vec<ChartSeries>,
    /// A color, a written value and a name in the tooltip, for each value of a chart built from data.
    marks: Vec<(Option<Color>, Option<String>, Option<String>)>,
    palette: ComponentPalette,
    width: Option<f32>,
    height: Option<f32>,
    color: Option<Color>,
    stacked: bool,
    gradient: bool,
    radius: f32,
    max_bar: f32,
    grid: bool,
    value_axis: bool,
    value_labels: bool,
    total: Option<String>,
    track: Option<Color>,
}

impl Bars {
    fn new(id: String, palette: ComponentPalette) -> Self {
        Self {
            id,
            categories: Vec::new(),
            series: Vec::new(),
            marks: Vec::new(),
            palette,
            width: None,
            height: None,
            color: None,
            stacked: false,
            gradient: false,
            radius: 4.0,
            max_bar: 32.0,
            grid: true,
            value_axis: true,
            value_labels: false,
            total: None,
            track: None,
        }
    }

    fn from_data(id: String, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        let mut bars = Self::new(id, palette);
        bars.categories = data.iter().map(|datum| datum.label.clone()).collect();
        bars.series = vec![ChartSeries::new("", data.iter().map(|datum| datum.value))];
        bars.marks = data
            .into_iter()
            .map(|datum| (datum.color, Some(datum.value_label), datum.tooltip))
            .collect();
        // One series is read bar by bar: each says its value.
        bars.value_labels = true;
        bars
    }

    fn render(self, horizontal: bool) -> Element {
        let palette = self.palette;
        let base = self.color.unwrap_or(palette.text);
        let several = self.series.len();
        let series = self
            .series
            .into_iter()
            .enumerate()
            .map(|(index, series)| crate::BarSeries {
                name: series.name,
                color: series_color(series.color, base, index, several),
                values: series
                    .values
                    .into_iter()
                    .enumerate()
                    .map(|(category, value)| {
                        let (color, label, name) = self.marks.get(category).cloned().unwrap_or_default();
                        crate::BarValue { value: finite(value), color, label, name }
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let count = self.categories.len().max(1) as f32;
        let height = self.height.unwrap_or(if horizontal { count * 34.0 + 24.0 } else { 240.0 });
        Element::bar_plot(crate::BarPlot {
            id: self.id,
            categories: self.categories,
            series,
            stacked: self.stacked,
            horizontal,
            gradient: self.gradient,
            radius: self.radius,
            max_bar: self.max_bar,
            grid: self.grid,
            value_axis: self.value_axis,
            value_labels: self.value_labels,
            text: palette.text,
            muted: palette.muted,
            line: Color { a: (f32::from(palette.border.a) * 0.7) as u8, ..palette.border },
            tooltip: palette.tooltip_style(),
            total: self.total,
            track: self.track,
        })
        .size(self.width, Some(height))
    }
}

/// The builders a bar chart and a column chart have in common.
macro_rules! bars_builders {
    () => {
        /// Fix the width in logical pixels, never below 160. Without one the chart takes
        /// the width it is offered. A value that is not finite is ignored.
        pub fn width(mut self, width: f32) -> Self {
            if width.is_finite() {
                self.0.width = Some(width.max(160.0));
            }
            self
        }

        /// Fix the height in logical pixels, from 80 to 1200. A value that is not finite
        /// is ignored.
        pub fn height(mut self, height: f32) -> Self {
            if height.is_finite() {
                self.0.height = Some(height.clamp(80.0, 1200.0));
            }
            self
        }

        /// The color of the bars: the first series takes it whole and each one after it
        /// a lighter step. The palette's text color by default.
        pub fn color(mut self, color: Color) -> Self {
            self.0.color = Some(color);
            self
        }

        /// Pile each category's series on one another instead of setting them side by
        /// side. The tooltip lists each and, with `total`, their sum.
        pub fn stacked(mut self) -> Self {
            self.0.stacked = true;
            self
        }

        /// Fade each bar from a third of its color at its foot to all of it at its tip.
        pub fn gradient(mut self, gradient: bool) -> Self {
            self.0.gradient = gradient;
            self
        }

        /// Round the tip of each bar by this radius, 4 by default; its foot stays square.
        /// A negative value is zero and one that is not finite is ignored.
        pub fn radius(mut self, radius: f32) -> Self {
            if radius.is_finite() {
                self.0.radius = radius.max(0.0);
            }
            self
        }

        /// The widest a bar is drawn, 32 by default and never below 2. A value that is
        /// not finite is ignored.
        pub fn max_bar_width(mut self, width: f32) -> Self {
            if width.is_finite() {
                self.0.max_bar = width.max(2.0);
            }
            self
        }

        /// Lay a rail of `color` behind each category's bars, from the zero line to the
        /// end of the value axis, so how full each one is reads at a glance.
        pub fn track(mut self, color: Color) -> Self {
            self.0.track = Some(color);
            self
        }

        /// Draw or leave out the dashed line at each tick of the value axis. Drawn by default.
        pub fn grid(mut self, grid: bool) -> Self {
            self.0.grid = grid;
            self
        }

        /// Write or leave out the ticks of the value axis. Written by default.
        pub fn value_axis(mut self, axis: bool) -> Self {
            self.0.value_axis = axis;
            self
        }

        /// Write each bar's value at its tip, or a stack's sum at its end. A chart built
        /// from data writes them by default; a chart of several series does not.
        pub fn value_labels(mut self, labels: bool) -> Self {
            self.0.value_labels = labels;
            self
        }

        /// Add a last row to the tooltip of a chart of several series that sums the
        /// category, under this name.
        pub fn total(mut self, name: impl Into<String>) -> Self {
            self.0.total = Some(name.into());
            self
        }
    };
}

/// Horizontal bars over a value axis with round ticks: one bar for each
/// datum, or one for each series of each category, side by side or stacked.
/// The bars grow from the zero line when the chart appears. Under the
/// pointer, the category it is over is lit and the others recede, and a
/// tooltip that follows the pointer lists its values. It takes no clicks and
/// adds no tab stops.
#[derive(Clone)]
pub struct BarChart(Bars);

impl BarChart {
    /// A chart of one bar for each datum, each written with its value. `id` is the
    /// stable ID under which the tree keeps the chart's motion and its hover.
    pub fn new(id: impl Into<String>, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        Self(Bars::from_data(id.into(), data, palette))
    }

    /// A chart of several series over the same categories: a value of each series for
    /// each category, side by side unless the chart is `stacked`.
    pub fn series(
        id: impl Into<String>,
        categories: impl IntoIterator<Item = impl Into<String>>,
        series: Vec<ChartSeries>,
        palette: ComponentPalette,
    ) -> Self {
        let mut bars = Bars::new(id.into(), palette);
        bars.categories = categories.into_iter().map(Into::into).collect();
        bars.series = series;
        Self(bars)
    }

    /// The same chart as columns that stand on the category axis.
    pub fn vertical(self) -> ColumnChart {
        ColumnChart(self.0)
    }

    bars_builders!();
}

impl Component for BarChart {
    fn render(self) -> Element {
        self.0.render(true)
    }
}

/// Columns that stand on a category axis, with the same series, motion and
/// tooltip as a [`BarChart`]. Without a width it takes the one it is
/// offered; it is 240 pixels high unless it is given a height.
#[derive(Clone)]
pub struct ColumnChart(Bars);

impl ColumnChart {
    /// A chart of one column for each datum, each written with its value. `id` is the
    /// stable ID under which the tree keeps the chart's motion and its hover.
    pub fn new(id: impl Into<String>, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        Self(Bars::from_data(id.into(), data, palette))
    }

    /// A chart of several series over the same categories: a value of each series for
    /// each category, side by side unless the chart is `stacked`.
    pub fn series(
        id: impl Into<String>,
        categories: impl IntoIterator<Item = impl Into<String>>,
        series: Vec<ChartSeries>,
        palette: ComponentPalette,
    ) -> Self {
        Self(BarChart::series(id, categories, series, palette).0)
    }

    /// It made ticks whole numbers. Data that are all whole numbers now get whole ticks on
    /// their own, and the rest round steps of 1, 2 or 5.
    #[deprecated(note = "whole-number data get whole ticks on their own; this has no effect")]
    pub fn integer_ticks(self, _integer: bool) -> Self {
        self
    }

    bars_builders!();
}

impl Component for ColumnChart {
    fn render(self) -> Element {
        self.0.render(false)
    }
}

/// The color of the series at `index` of `count`: its own, or a step of the
/// chart's, lighter for each one after the first.
fn series_color(own: Option<Color>, base: Color, index: usize, count: usize) -> Color {
    own.unwrap_or(Color { a: (f32::from(base.a) * (1.0 - 0.7 * index as f32 / count.max(2) as f32)).round() as u8, ..base })
}

/// Lines over the points of an axis, one for each series, on a value axis
/// with round ticks. The lines are drawn from left to right when the chart
/// appears. Under the pointer, a line marks the nearest point with a dot on
/// each series, and a tooltip that follows the pointer lists their values.
/// It takes no clicks and adds no tab stops. An [`AreaChart`] is the same
/// chart with the area under each line filled.
#[derive(Clone)]
pub struct LineChart {
    id: String,
    points: Vec<String>,
    series: Vec<ChartSeries>,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
    color: Option<Color>,
    smooth: bool,
    area: bool,
    dots: bool,
    grid: bool,
    value_axis: bool,
    point_axis: bool,
    range: Option<(f64, f64)>,
    dashed_zero: bool,
    mark: Option<usize>,
    stacked: bool,
}

impl LineChart {
    /// A chart of one line for each series over the same points. `id` is the stable
    /// ID under which the tree keeps the chart's motion and its hover.
    pub fn new(
        id: impl Into<String>,
        points: impl IntoIterator<Item = impl Into<String>>,
        series: Vec<ChartSeries>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            points: points.into_iter().map(Into::into).collect(),
            series,
            palette,
            width: None,
            height: 240.0,
            color: None,
            smooth: true,
            area: false,
            dots: false,
            grid: true,
            value_axis: true,
            point_axis: true,
            range: None,
            dashed_zero: false,
            mark: None,
            stacked: false,
        }
    }

    /// Fix the width in logical pixels, never below 160. Without one the chart takes
    /// the width it is offered. A value that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(160.0));
        }
        self
    }

    /// Fix the height in logical pixels, from 80 to 1200; 240 by default. A value
    /// that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.clamp(80.0, 1200.0);
        }
        self
    }

    /// The color of the lines: the first series takes it whole and each one after it
    /// a lighter step. The palette's text color by default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Round each line through its points, without passing over or under any. On by
    /// default; `smooth(false)` joins the points with straight strokes.
    pub fn smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }

    /// Fill under each line, from its color down to nothing at the zero line.
    pub fn area(mut self, area: bool) -> Self {
        self.area = area;
        self
    }

    /// Draw a dot at every point of every line.
    pub fn dots(mut self, dots: bool) -> Self {
        self.dots = dots;
        self
    }

    /// Draw or leave out the dashed line at each tick of the value axis. Drawn by default.
    pub fn grid(mut self, grid: bool) -> Self {
        self.grid = grid;
        self
    }

    /// Write or leave out the ticks of the value axis. Written by default.
    pub fn value_axis(mut self, axis: bool) -> Self {
        self.value_axis = axis;
        self
    }

    /// Write or leave out the names of the points under the plot, and the room
    /// they take. Written by default; the tooltip says them either way.
    pub fn point_axis(mut self, axis: bool) -> Self {
        self.point_axis = axis;
        self
    }

    /// Fix the value axis from `low` to `high` instead of fitting it to the
    /// values, with ticks at both ends and at zero between them; a positive
    /// tick of an axis that runs below zero is written with its sign, as a
    /// gain is. Values past it are drawn past it. Ignored unless both are
    /// finite and `low` is below `high`.
    pub fn range(mut self, low: f64, high: f64) -> Self {
        if low.is_finite() && high.is_finite() && low < high {
            self.range = Some((low, high));
        }
        self
    }

    /// Dash the zero line instead of drawing it solid.
    pub fn dashed_zero(mut self, dashed: bool) -> Self {
        self.dashed_zero = dashed;
        self
    }

    /// Mark this point, counted from 0, on every line with a dot, as the
    /// pointer marks the one it reads. A point past the last marks nothing.
    pub fn mark(mut self, point: usize) -> Self {
        self.mark = Some(point);
        self
    }
    /// Set each series on the ones before it, as a stacked area of ECharts' is:
    /// each line is the sum so far, and the area under it is filled from the line
    /// below in its color, strong at the top and lighter at the foot. The tooltip
    /// still says each series' own value.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }
}

impl Component for LineChart {
    fn render(self) -> Element {
        let palette = self.palette;
        let base = self.color.unwrap_or(palette.text);
        let count = self.series.len();
        Element::line_plot(crate::LinePlot {
            id: self.id,
            points: self.points,
            series: self
                .series
                .into_iter()
                .enumerate()
                .map(|(index, series)| crate::PlotSeries {
                    name: series.name,
                    color: series_color(series.color, base, index, count),
                    values: series.values.into_iter().map(finite).collect(),
                })
                .collect(),
            smooth: self.smooth,
            area: self.area,
            dots: self.dots,
            grid: self.grid,
            value_axis: self.value_axis,
            point_axis: self.point_axis,
            range: self.range,
            dashed_zero: self.dashed_zero,
            mark: self.mark,
            stacked: self.stacked,
            text: palette.text,
            muted: palette.muted,
            line: Color { a: (f32::from(palette.border.a) * 0.7) as u8, ..palette.border },
            ground: palette.surface,
            tooltip: palette.tooltip_style(),
        })
        .size(self.width, Some(self.height))
    }
}

/// A [`LineChart`] with the area under each line filled: the same builders,
/// the same motion and the same tooltip.
#[derive(Clone)]
pub struct AreaChart;

impl AreaChart {
    /// A line chart whose lines stand on a fill that fades down to the zero line.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        id: impl Into<String>,
        points: impl IntoIterator<Item = impl Into<String>>,
        series: Vec<ChartSeries>,
        palette: ComponentPalette,
    ) -> LineChart {
        LineChart::new(id, points, series, palette).area(true)
    }
}

/// A circle shared out among its data, or a ring of them with something said
/// in its middle. The slices sweep round from twelve o'clock when the chart
/// appears. The slice under the pointer stands out and the others recede,
/// and a tooltip that follows the pointer says its value and its share. It
/// takes no clicks and adds no tab stops.
#[derive(Clone)]
pub struct PieChart {
    id: String,
    data: Vec<ChartDatum>,
    palette: ComponentPalette,
    size: f32,
    color: Option<Color>,
    hole: f32,
    title: String,
    caption: String,
}

impl PieChart {
    /// A pie of one slice for each datum, clockwise from twelve o'clock in their
    /// order. `id` is the stable ID under which the tree keeps the chart's motion.
    pub fn new(id: impl Into<String>, data: Vec<ChartDatum>, palette: ComponentPalette) -> Self {
        Self { id: id.into(), data, palette, size: 220.0, color: None, hole: 0.0, title: String::new(), caption: String::new() }
    }

    /// The side of the square the chart takes, in logical pixels, from 80 to 1200;
    /// 220 by default. A value that is not finite is ignored.
    pub fn size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.size = size.clamp(80.0, 1200.0);
        }
        self
    }

    /// The color of the slices without one of their own: the first takes it whole
    /// and each one after it a lighter step. The palette's text color by default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Make it a ring: the hole as a share of the radius, from 0 to 0.9. A value that
    /// is not finite is ignored.
    pub fn donut(mut self, hole: f32) -> Self {
        if hole.is_finite() {
            self.hole = hole.clamp(0.0, 0.9);
        }
        self
    }

    /// What a ring says in its middle: a figure, over a line that says what it is.
    pub fn center(mut self, title: impl Into<String>, caption: impl Into<String>) -> Self {
        self.title = title.into();
        self.caption = caption.into();
        self
    }
}

impl Component for PieChart {
    fn render(self) -> Element {
        let palette = self.palette;
        let base = self.color.unwrap_or(palette.text);
        let count = self.data.len();
        Element::pie_plot(crate::PiePlot {
            id: self.id,
            slices: self
                .data
                .into_iter()
                .enumerate()
                .map(|(index, datum)| crate::PieSlice {
                    name: datum.tooltip.unwrap_or(datum.label),
                    value: datum.value,
                    color: series_color(datum.color, base, index, count),
                    label: Some(datum.value_label),
                })
                .collect(),
            hole: self.hole,
            title: self.title,
            caption: self.caption,
            text: palette.text,
            muted: palette.muted,
            tooltip: palette.tooltip_style(),
        })
        .size(Some(self.size), Some(self.size))
    }
}

/// Series compared over several axes that spread from a center: each is a
/// shape whose corners stand on the axes at its values. The shapes open from
/// the center when the chart appears. The axis nearest the pointer is marked
/// with a dot on each series, and a tooltip that follows the pointer lists
/// their values. It takes no clicks and adds no tab stops.
#[derive(Clone)]
pub struct RadarChart {
    id: String,
    axes: Vec<String>,
    series: Vec<ChartSeries>,
    palette: ComponentPalette,
    width: f32,
    height: f32,
    color: Option<Color>,
    maximum: Option<f64>,
}

impl RadarChart {
    /// A radar of one shape for each series over the same axes, clockwise from
    /// twelve o'clock. With fewer than three axes nothing is drawn. `id` is the
    /// stable ID under which the tree keeps the chart's motion and its hover.
    pub fn new(
        id: impl Into<String>,
        axes: impl IntoIterator<Item = impl Into<String>>,
        series: Vec<ChartSeries>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            axes: axes.into_iter().map(Into::into).collect(),
            series,
            palette,
            width: 320.0,
            height: 260.0,
            color: None,
            maximum: None,
        }
    }

    /// The width and the height in logical pixels, each from 120 to 1200; 320 by 260
    /// by default. A value that is not finite leaves that side as it was.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() {
            self.width = width.clamp(120.0, 1200.0);
        }
        if height.is_finite() {
            self.height = height.clamp(120.0, 1200.0);
        }
        self
    }

    /// The color of the shapes: the first series takes it whole and each one after
    /// it a lighter step. The palette's text color by default.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// The value at the rim. Without one it is a round number over the greatest
    /// value. One that is not finite or not above zero is ignored.
    pub fn maximum(mut self, maximum: f64) -> Self {
        if maximum.is_finite() && maximum > 0.0 {
            self.maximum = Some(maximum);
        }
        self
    }
}

impl Component for RadarChart {
    fn render(self) -> Element {
        let palette = self.palette;
        let base = self.color.unwrap_or(palette.text);
        let count = self.series.len();
        Element::radar_plot(crate::RadarPlot {
            id: self.id,
            axes: self.axes,
            series: self
                .series
                .into_iter()
                .enumerate()
                .map(|(index, series)| crate::PlotSeries {
                    name: series.name,
                    color: series_color(series.color, base, index, count),
                    values: series.values.into_iter().map(finite).collect(),
                })
                .collect(),
            maximum: self.maximum,
            text: palette.text,
            muted: palette.muted,
            line: Color { a: (f32::from(palette.border.a) * 0.7) as u8, ..palette.border },
            ground: palette.surface,
            tooltip: palette.tooltip_style(),
        })
        .size(Some(self.width), Some(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};

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
        let window = Size::new(90.0, 60.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(chart);
        tree.set_reduced_motion(true);
        let cells = |frame: &crate::Frame| -> Vec<(crate::Rect, Color)> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if rect.width >= 10.0 && rect.width <= 20.0 && rect.height == rect.width => Some((*rect, *color)),
                    _ => None,
                })
                .collect()
        };
        let rest = cells(&tree.layout_at(window, start));
        // A hole draws nothing; a day with nothing is a cell at level zero.
        assert_eq!(rest.len(), 2);
        assert_eq!((rest[0].0.x, rest[0].0.y, rest[0].0.width, rest[0].1), (0.0, 15.0, 12.0, palette.raised));
        assert_eq!((rest[1].0.x, rest[1].0.y), (15.0, 0.0));

        // Under the pointer a cell grows about its middle and says what it is, and the other recedes.
        tree.handle_event(crate::UiEvent::PointerMove(crate::Point::new(21.0, 6.0)));
        tree.layout_at(window, start);
        let frame = tree.layout_at(window, start);
        let lit = cells(&frame);
        let (monday, sunday) = (lit[0], lit[1]);
        assert!(sunday.0.width > 12.0 && (sunday.0.x + sunday.0.width / 2.0 - 21.0).abs() < 0.01, "{sunday:?}");
        assert!(monday.0.width == 12.0 && monday.1.a < palette.raised.a, "{monday:?}");
        assert!(texts(&frame).contains(&"8 contributions".to_owned()));
        assert!(frame.regions.iter().all(|region| region.kind == crate::HitKind::Hover));
    }

    /// The rectangles drawn in `color`, at any strength: the bars of a series.
    fn bars(frame: &crate::Frame, color: Color) -> Vec<(crate::Rect, u8)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillCornerRect { rect, color: fill, .. } | DrawCommand::FillRect { rect, color: fill, .. }
                    if (fill.r, fill.g, fill.b) == (color.r, color.g, color.b) && rect.width > 2.0 && rect.height > 2.0 =>
                {
                    Some((*rect, fill.a))
                }
                _ => None,
            })
            .collect()
    }

    fn texts(frame: &crate::Frame) -> Vec<String> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    const INK: Color = Color::rgb(10, 200, 90);
    const OTHER: Color = Color::rgb(200, 40, 220);

    fn grouped() -> ColumnChart {
        ColumnChart::series(
            "sales",
            ["Jan", "Feb", "Mar"],
            vec![ChartSeries::new("Desktop", [10.0, 40.0, 20.0]).color(INK), ChartSeries::new("Mobile", [5.0, 20.0, 30.0]).color(OTHER)],
            ComponentPalette::neutral_dark(),
        )
        .width(400.0)
        .height(240.0)
    }

    #[test]
    fn bars_grow_from_the_zero_line_as_the_chart_appears_and_stand_at_once_without_motion() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(grouped().render());
        assert!(bars(&tree.layout_at(window, start), INK).is_empty(), "nothing stands on the first frame");
        let half = bars(&tree.layout_at(window, start + web_time::Duration::from_millis(200)), INK);
        let full = bars(&tree.layout_at(window, start + web_time::Duration::from_secs(3)), INK);
        assert_eq!((half.len(), full.len()), (3, 3));
        for ((growing, _), (grown, _)) in half.iter().zip(&full) {
            assert!(growing.height < grown.height, "{growing:?} against {grown:?}");
            // They stand on one line, whatever their height.
            assert_eq!(growing.y + growing.height, grown.y + grown.height);
        }
        // Heights are in proportion to the values: 10, 40 and 20.
        assert!((full[1].0.height / full[0].0.height - 4.0).abs() < 0.05, "{full:?}");
        assert!((full[2].0.height / full[0].0.height - 2.0).abs() < 0.05, "{full:?}");

        let mut still = UiTree::new(grouped().render());
        still.set_reduced_motion(true);
        assert_eq!(bars(&still.layout_at(window, start), INK), full);
    }

    #[test]
    fn the_category_under_the_pointer_is_lit_listed_in_a_tooltip_and_the_others_recede() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(grouped().render());
        tree.set_reduced_motion(true);
        let rest = tree.layout_at(window, start);
        let columns = bars(&rest, INK);
        assert!(columns.iter().all(|(_, strength)| *strength == 255));
        assert!(!texts(&rest).contains(&"Desktop".to_owned()), "no tooltip without the pointer");

        // Over February: twice, since what is under the pointer is known from the frame before.
        let february = columns[1].0;
        tree.handle_event(crate::UiEvent::PointerMove(crate::Point::new(february.x + 2.0, 100.0)));
        tree.layout_at(window, start);
        let lit = tree.layout_at(window, start);
        let strengths: Vec<u8> = bars(&lit, INK).into_iter().map(|(_, strength)| strength).collect();
        assert!(strengths[1] == 255 && strengths[0] < 255 && strengths[2] < 255, "{strengths:?}");
        let said = texts(&lit);
        for wanted in ["Feb", "Desktop", "40", "Mobile", "20"] {
            assert!(said.contains(&wanted.to_owned()), "{wanted} is not in {said:?}");
        }
        // It takes no click and no focus.
        assert!(lit.regions.iter().all(|region| region.kind == crate::HitKind::Hover));
    }

    #[test]
    fn a_stack_piles_its_series_and_a_negative_value_grows_the_other_way() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(grouped().stacked().total("Total").value_labels(true).render());
        tree.set_reduced_motion(true);
        let frame = tree.layout_at(window, start);
        let (below, above) = (bars(&frame, INK), bars(&frame, OTHER));
        for ((first, _), (second, _)) in below.iter().zip(&above) {
            assert_eq!(first.x, second.x);
            assert!((second.y + second.height - first.y).abs() < 0.01, "{second:?} does not stand on {first:?}");
        }
        // The sum of each pile is written over it.
        for sum in ["15", "60", "50"] {
            assert!(texts(&frame).contains(&sum.to_owned()), "{sum}");
        }

        let signed = ColumnChart::new(
            "change",
            vec![ChartDatum::new("up", "Up", 4.0).color(INK), ChartDatum::new("down", "Down", -2.0).color(OTHER)],
            ComponentPalette::neutral_dark(),
        )
        .width(400.0);
        let mut tree = UiTree::new(signed.render());
        tree.set_reduced_motion(true);
        let frame = tree.layout_at(window, start);
        let (up, down) = (bars(&frame, INK)[0].0, bars(&frame, OTHER)[0].0);
        assert!((up.y + up.height - down.y).abs() < 0.01, "{up:?} and {down:?} do not meet at zero");
        assert!((up.height / down.height - 2.0).abs() < 0.05);
    }

    #[test]
    fn a_bar_chart_lies_down_and_bad_numbers_never_reach_the_layout() {
        let window = Size::new(400.0, 240.0);
        let data = vec![
            ChartDatum::new("a", "Alpha", 30.0).color(INK),
            ChartDatum::new("b", "Beta", f64::NAN).color(INK),
            ChartDatum::new("c", "Gamma", 60.0).color(INK),
        ];
        let chart = BarChart::new("lying", data, ComponentPalette::neutral_dark())
            .width(f32::NAN)
            .height(f32::INFINITY)
            .radius(f32::NAN)
            .max_bar_width(f32::NAN);
        let mut tree = UiTree::new(chart.render());
        tree.set_reduced_motion(true);
        let frame = tree.layout_at(window, web_time::Instant::now());
        let drawn = bars(&frame, INK);
        assert_eq!(drawn.len(), 2, "what is not a number is no bar: {drawn:?}");
        assert!((drawn[1].0.width / drawn[0].0.width - 2.0).abs() < 0.05);
        assert_eq!(drawn[0].0.x, drawn[1].0.x);
        for command in &frame.commands {
            if let DrawCommand::FillRect { rect, .. } | DrawCommand::FillCornerRect { rect, .. } = command {
                assert!([rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite()));
            }
        }
    }

    fn meshes(frame: &crate::Frame) -> Vec<Vec<crate::Point>> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillMesh { points, .. } => Some(points.to_vec()),
                _ => None,
            })
            .collect()
    }

    fn lines() -> LineChart {
        LineChart::new(
            "visits",
            ["Jan", "Feb", "Mar", "Apr"],
            vec![ChartSeries::new("Desktop", [10.0, 40.0, 20.0, 30.0]).color(INK), ChartSeries::new("Mobile", [5.0, 20.0, 30.0, 10.0]).color(OTHER)],
            ComponentPalette::neutral_dark(),
        )
        .width(400.0)
    }

    #[test]
    fn stacked_series_stand_on_those_before_and_fill_from_their_line() {
        let window = Size::new(400.0, 240.0);
        let frame_of = |stacked: bool| {
            let mut tree = UiTree::new(lines().area(true).smooth(false).stacked(stacked).height(240.0).render());
            tree.set_reduced_motion(true);
            tree.layout_at(window, web_time::Instant::now())
        };
        let top = |frame: &crate::Frame| texts(frame).iter().filter_map(|text| text.parse::<f64>().ok()).fold(0.0, f64::max);
        // Feb's two together reach 60, past either alone.
        assert!(top(&frame_of(false)) < 60.0 && top(&frame_of(true)) >= 60.0);
        let stacked = meshes(&frame_of(true));
        assert_eq!(stacked.len(), 2);
        // The second's foot is the first's line, not the zero line.
        let first_tops: Vec<f32> = stacked[0].iter().step_by(2).map(|point| point.y).collect();
        let second_feet: Vec<f32> = stacked[1].iter().skip(1).step_by(2).map(|point| point.y).collect();
        assert_eq!(first_tops, second_feet);
    }

    #[test]
    fn whole_number_data_get_whole_ticks_and_fractions_keep_half_steps() {
        let ticks = |value: f64| {
            let chart = ColumnChart::new("count", vec![ChartDatum::new("a", "A", value)], ComponentPalette::neutral_dark())
                .width(400.0)
                .height(200.0)
                .value_labels(false);
            let mut tree = UiTree::new(chart.render());
            tree.set_reduced_motion(true);
            let frame = tree.layout_at(Size::new(400.0, 200.0), web_time::Instant::now());
            texts(&frame).into_iter().filter(|text| text != "A").collect::<Vec<_>>()
        };
        assert_eq!(ticks(1.0), ["0", "1"]);
        assert!(ticks(1.5).contains(&"0.5".to_owned()));
    }

    #[test]
    fn a_fixed_range_a_dashed_zero_line_a_mark_and_no_point_names() {
        let window = Size::new(400.0, 160.0);
        let chart = || {
            LineChart::new("gain", ["31", "1K", "16K"], vec![ChartSeries::new("Gain", [3.0, -2.0, 20.0]).color(INK)], ComponentPalette::neutral_dark())
                .width(400.0)
                .height(160.0)
                .grid(false)
                .range(-12.0, 12.0)
        };
        let frame_of = |chart: LineChart| {
            let mut tree = UiTree::new(chart.render());
            tree.set_reduced_motion(true);
            tree.layout_at(window, web_time::Instant::now())
        };
        let frame = frame_of(chart());
        // The ends and zero, a gain's sign on the positive one, whatever the values reach.
        let ticks: Vec<String> = texts(&frame).into_iter().filter(|text| !["31", "1K", "16K"].contains(&text.as_str())).collect();
        assert_eq!(ticks, ["-12", "0", "+12"]);
        assert_eq!(texts(&frame).len(), 6, "the points are named by default");
        let zero_dashed = |frame: &crate::Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Polyline { points, dashed, width, .. } if *width == 1.0 && points.len() == 2 => Some(*dashed),
                _ => None,
            })
        };
        assert_eq!(zero_dashed(&frame), Some(false));
        let marked = |frame: &crate::Frame| frame.commands.iter().filter(|command| matches!(command, DrawCommand::FillRect { color, rect, .. } if *color == INK && rect.width == 8.0)).count();
        assert_eq!(marked(&frame), 0);

        let frame = frame_of(chart().point_axis(false).dashed_zero(true).mark(1).mark(1));
        assert_eq!(texts(&frame), ["-12", "0", "+12"]);
        assert_eq!(zero_dashed(&frame), Some(true));
        assert_eq!(marked(&frame), 1, "one dot on the one line");
        // A range that is not one is ignored; a mark past the last point marks nothing.
        let frame = frame_of(chart().range(5.0, 5.0).mark(9).point_axis(false));
        assert_eq!(marked(&frame), 0);
        assert!(texts(&frame).contains(&"+12".to_owned()));
    }

    #[test]
    fn lines_are_drawn_from_the_left_as_the_chart_appears_and_an_area_stands_on_the_zero_line() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let clip = |frame: &crate::Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::PushClip(rect) => Some(rect.width),
                _ => None,
            })
        };
        let mut tree = UiTree::new(lines().render());
        tree.layout_at(window, start);
        let part = clip(&tree.layout_at(window, start + web_time::Duration::from_millis(150))).unwrap();
        let whole = clip(&tree.layout_at(window, start + web_time::Duration::from_secs(3))).unwrap();
        assert!(part > 0.0 && part < whole, "{part} of {whole}");

        // A line passes through each of its points, rounded or not.
        let through = |smooth: bool| {
            let mut tree = UiTree::new(lines().smooth(smooth).render());
            tree.set_reduced_motion(true);
            let frame = tree.layout_at(window, start);
            let strokes: Vec<Vec<crate::Point>> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Polyline { points, color, width, .. } if *color == INK && *width == 2.0 => Some(points.to_vec()),
                    _ => None,
                })
                .collect();
            assert_eq!(strokes.len(), 1);
            strokes[0].clone()
        };
        let (straight, rounded) = (through(false), through(true));
        assert_eq!(straight.len(), 4);
        assert!(rounded.len() > straight.len());
        for knot in &straight {
            assert!(rounded.iter().any(|point| (point.x - knot.x).abs() < 0.01 && (point.y - knot.y).abs() < 0.01), "{knot:?}");
        }
        // 40 is twice as far from the zero line as 20.
        let zero = straight[0].y + (straight[0].y - straight[1].y) / 3.0;
        assert!(((zero - straight[1].y) / (zero - straight[2].y) - 2.0).abs() < 0.05);

        let mut area = UiTree::new(AreaChart::new("area", ["a", "b", "c"], vec![ChartSeries::new("One", [1.0, 3.0, 2.0])], ComponentPalette::neutral_dark()).width(400.0).render());
        area.set_reduced_motion(true);
        let fills = meshes(&area.layout_at(window, start));
        assert_eq!(fills.len(), 1);
        let foot = fills[0].iter().map(|point| point.y).fold(f32::MIN, f32::max);
        assert!(fills[0].iter().skip(1).step_by(2).all(|point| point.y == foot), "every strip stands on one line");
        assert!(meshes(&UiTree::new(lines().render()).layout_at(window, start + web_time::Duration::from_secs(9))).is_empty());
    }

    #[test]
    fn the_point_nearest_the_pointer_is_marked_and_listed() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(lines().render());
        tree.set_reduced_motion(true);
        let rest = tree.layout_at(window, start);
        let area = rest.regions.iter().find(|region| region.id == "visits").unwrap().rect;
        // Nearer March, the third of four points, than April.
        tree.handle_event(crate::UiEvent::PointerMove(crate::Point::new(area.x + area.width * 0.7, area.y + 30.0)));
        tree.layout_at(window, start);
        let said = texts(&tree.layout_at(window, start));
        for wanted in ["Mar", "Desktop", "20", "Mobile", "30"] {
            assert!(said.iter().filter(|text| *text == wanted).count() >= 1, "{wanted} is not in {said:?}");
        }
    }

    #[test]
    fn a_pie_shares_the_circle_by_value_and_sweeps_round_as_it_appears() {
        // The root takes the whole window, so the window is the chart's square.
        let window = Size::new(220.0, 220.0);
        let start = web_time::Instant::now();
        let data = || vec![ChartDatum::new("a", "Alpha", 3.0), ChartDatum::new("b", "Beta", 1.0), ChartDatum::new("c", "None", 0.0), ChartDatum::new("d", "Bad", f64::NAN)];
        let pie = || PieChart::new("pie", data(), ComponentPalette::neutral_dark()).size(220.0);
        let mut tree = UiTree::new(pie().render());
        assert!(meshes(&tree.layout_at(window, start)).is_empty(), "nothing is swept on the first frame");
        let grown = meshes(&tree.layout_at(window, start + web_time::Duration::from_secs(3)));
        // A slice of nothing is not drawn.
        assert_eq!(grown.len(), 2);
        // Three quarters of the turn against one: three times the strokes along the rim.
        let share = grown[0].len() as f32 / grown[1].len() as f32;
        assert!((2.6..=3.3).contains(&share), "{share}");
        let center = crate::Point::new(110.0, 110.0);
        let reach = |points: &[crate::Point]| points.iter().map(|point| (point.x - center.x).hypot(point.y - center.y)).fold(0.0, f32::max);
        // A whole pie reaches its center, a ring does not.
        let nearest = |points: &[crate::Point]| points.iter().map(|point| (point.x - center.x).hypot(point.y - center.y)).fold(f32::MAX, f32::min);
        assert!(nearest(&grown[0]) < 0.5);
        let mut ring = UiTree::new(pie().donut(0.6).center("4", "Things").render());
        ring.set_reduced_motion(true);
        let frame = ring.layout_at(window, start);
        let holed = meshes(&frame);
        assert!((nearest(&holed[0]) / reach(&holed[0]) - 0.6).abs() < 0.02);
        assert!(texts(&frame).contains(&"4".to_owned()) && texts(&frame).contains(&"Things".to_owned()));

        // Over the first slice, to the right of twelve o'clock: it is named with its share.
        ring.handle_event(crate::UiEvent::PointerMove(crate::Point::new(center.x + 60.0, center.y - 50.0)));
        ring.layout_at(window, start);
        let said = texts(&ring.layout_at(window, start));
        assert!(said.contains(&"Alpha".to_owned()) && said.contains(&"3 (75.0 %)".to_owned()), "{said:?}");
    }

    #[test]
    fn a_radar_opens_from_its_center_and_needs_three_axes() {
        let window = Size::new(320.0, 260.0);
        let start = web_time::Instant::now();
        let radar = |axes: &[&str]| {
            RadarChart::new("radar", axes.iter().copied(), vec![ChartSeries::new("One", [50.0, 100.0, 25.0, 75.0]).color(INK)], ComponentPalette::neutral_dark())
                .maximum(100.0)
        };
        let mut two = UiTree::new(radar(&["a", "b"]).render());
        two.set_reduced_motion(true);
        assert!(two.layout_at(window, start).commands.iter().all(|command| !matches!(command, DrawCommand::FillMesh { .. } | DrawCommand::Polyline { .. })));

        let mut tree = UiTree::new(radar(&["North", "East", "South", "West"]).render());
        let closed = meshes(&tree.layout_at(window, start));
        let open = meshes(&tree.layout_at(window, start + web_time::Duration::from_secs(3)));
        let center = open[0][0];
        let reach = |point: &crate::Point| (point.x - center.x).hypot(point.y - center.y);
        assert!(closed[0].iter().all(|point| reach(point) < 0.01), "it starts shut");
        // The corners stand on their axes in proportion: 50, 100, 25 and 75.
        let corners: Vec<f32> = open[0][1..].iter().map(reach).collect();
        assert!((corners[1] / corners[0] - 2.0).abs() < 0.02 && (corners[0] / corners[2] - 2.0).abs() < 0.02, "{corners:?}");
        // North is up and East to the right.
        assert!(open[0][1].y < center.y && (open[0][1].x - center.x).abs() < 0.01);
        assert!(open[0][2].x > center.x && (open[0][2].y - center.y).abs() < 0.01);
    }

    #[test]
    fn a_chart_tooltip_stays_inside_the_window() {
        let window = Size::new(400.0, 240.0);
        let start = web_time::Instant::now();
        let long = "A series with a name long enough to make the tooltip wide";
        let chart = ColumnChart::series("edge", ["Only"], vec![ChartSeries::new(long, [10.0])], ComponentPalette::neutral_dark()).width(400.0);
        let mut tree = UiTree::new(chart.render());
        tree.set_reduced_motion(true);
        let rest = tree.layout_at(window, start);
        let area = rest.regions.iter().find(|region| region.id == "edge").unwrap().rect;
        for point in [crate::Point::new(area.x + 2.0, area.y + 2.0), crate::Point::new(area.x + area.width - 2.0, area.y + area.height - 2.0)] {
            tree.handle_event(crate::UiEvent::PointerMove(point));
            tree.layout_at(window, start);
            let frame = tree.layout_at(window, start);
            let tip = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text.starts_with("A series") => Some(*position),
                    _ => None,
                })
                .next()
                .expect("the tooltip names the series");
            assert!(tip.x >= 0.0 && tip.x < window.width && tip.y >= 0.0 && tip.y < window.height, "{tip:?}");
            for command in &frame.commands {
                if let DrawCommand::StrokeRect { rect, .. } | DrawCommand::FillRect { rect, .. } = command {
                    assert!(rect.x >= -0.5 && rect.x + rect.width <= window.width + 0.5, "{rect:?}");
                }
            }
        }
    }
}
