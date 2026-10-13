# Charts

Read-only charts for data the host already has: `BarChart` and `ColumnChart`
compare categories, `HeatmapChart` shows intensity over a grid such as a
calendar, and `ChartLegend` explains a color scale. They take no clicks and
add no tab stops; hovering a bar or a cell shows a tooltip.

## Import

```rust
use limn::{
    BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ColumnChart, Component, HeatmapCell,
    HeatmapChart, HeatmapScale,
};
```

## Usage

A chart takes an ID, its data and the palette. A `ChartDatum` is an ID within
the chart, a label and a value.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let data = state
        .languages
        .iter()
        .map(|language| ChartDatum::new(&language.id, &language.name, language.files))
        .collect();
    BarChart::new("languages", data, palette).width(state.content_width).render()
}
```

A chart is 480 px wide unless `width` is set (a heatmap, 720), so pass the
width of its container when the window changes. Charts report nothing.

## Data

```rust
ChartDatum::new("skipped", "Skipped", 37.0)
    .value_label("37 (8 %)")
    .tooltip("Skipped: 37 tests behind a feature flag")
    .color(warning)
```

The value is an `f64`. A negative or non-finite value is drawn as zero.
`value_label` replaces the number shown, which is otherwise the value with no
decimals when it is whole and one when it is not. `tooltip` is the text shown
on hover, which is otherwise "label: value", and `color` paints this one bar
instead of the chart's color.

## BarChart

```rust
BarChart::new("tests", data, palette).width(480.0).color(accent).render()
```

Horizontal bars, one row per datum: the label, the bar and the value. The
bars share one linear scale from zero, on which the largest value fills the
track. `color` is the bars' color, the palette's text unless set.

## ColumnChart

```rust
ColumnChart::new("timings", data, palette)
    .width(480.0)
    .height(180.0)
    .max_bar_width(28.0)
    .integer_ticks(false)
    .render()
```

Vertical columns from a zero baseline, with a value axis at the left, each
value over its column and each label under it. `BarChart::vertical()` gives
the same chart from a bar chart, keeping its data, width and color.

- `height` is 240 unless set, between 120 and 1200.
- `max_bar_width` is 48 unless set, between 1 and 120.
- The axis has whole-number ticks, for counts. `integer_ticks(false)` allows
  fractions.

A column's tooltip is headed by its value, over the datum's tooltip with its
label.

Each category takes at least 64 px. When they do not fit in the width, the
columns scroll sideways and the value axis stays in place.

## HeatmapChart

```rust
let weeks: Vec<Vec<Option<HeatmapCell>>> = state
    .weeks
    .iter()
    .map(|week| {
        week.iter()
            .map(|day| day.as_ref().map(|day| HeatmapCell::new(&day.date, day.level, &day.summary)))
            .collect()
    })
    .collect();

HeatmapChart::new("activity", weeks, palette)
    .width(state.content_width)
    .row_labels(["", "Tue", "", "Thu", "", "Sat", ""])
    .column_labels(vec![ChartAxisLabel::new(0, "Jan"), ChartAxisLabel::new(5, "Feb")])
    .render()
```

The data is a list of columns, each a list of cells from the top. A
`HeatmapCell` is an ID within the chart, a level from 0 to 4 and the text of
its tooltip; a cell with empty text has no tooltip. `None` leaves a hole,
for a day outside the period, while level zero paints a cell.

`row_labels` names the rows at the left and `column_labels` names columns
above the grid, each `ChartAxisLabel` at the column it is given. A label
that would run into the one before it, or past the end, is left out.

Cells are 14 px squares 3 px apart unless `cell_size` (1 to 40) and `gap`
(0 to 16) say otherwise. When the columns do not fit in the width, the cells
and gaps shrink so that the whole period stays visible, and the row labels
are dropped when the columns become too narrow for them.

## Scale and legend

```rust
let scale = HeatmapScale::new([none, low, medium, high, highest]);

HeatmapChart::new("activity", weeks, palette).scale(scale).render()
ChartLegend::new(scale.colors, "Less", "More", palette).render()
```

A `HeatmapScale` is the five colors of levels 0 to 4. Without `scale` a
heatmap uses `HeatmapScale::neutral(palette)`, built from the palette.

`ChartLegend::new(colors, low, high, palette)` draws a square for each color
between a label for each end. It takes any colors, so it shows the scale the
chart was given.
