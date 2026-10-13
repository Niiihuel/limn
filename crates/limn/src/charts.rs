//! Read-only charts composed from the ordinary, backend-neutral element tree.
//! The host supplies data, labels and colors; charts never fetch or invent data.

use crate::{Color, Component, ComponentPalette, Element, FontFamily, Tooltip};

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
                // One hue, a step lighter for each series after the first.
                color: series.color.unwrap_or(Color {
                    a: (f32::from(base.a) * (1.0 - 0.7 * index as f32 / several.max(2) as f32)).round() as u8,
                    ..base
                }),
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

    /// It made ticks whole numbers. Ticks are now always round steps of 1, 2 or 5.
    #[deprecated(note = "ticks are always round steps; this has no effect")]
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
}
