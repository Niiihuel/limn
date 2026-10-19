# Charts

Read-only charts for data the host already has: `BarChart` and `ColumnChart`
compare categories, `LineChart` and `AreaChart` follow a measure along an
axis, `PieChart` shares out a whole, `RadarChart` compares series over
several axes, `HeatmapChart` shows intensity over a grid such as a calendar,
and `ChartLegend` explains a color scale. They take no clicks and
add no tab stops; the pointer over a chart marks what it is nearest and
shows its values.

## Import

```rust
use limn::{
    AreaChart, BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ChartSeries, ColumnChart,
    Component, HeatmapCell, HeatmapChart, HeatmapScale, LineChart, PieChart, RadarChart,
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

A bar chart takes the width it is offered unless `width` is set; a heatmap
is 720 px wide. Charts report nothing.

## Data

```rust
ChartDatum::new("skipped", "Skipped", 37.0)
    .value_label("37 (8 %)")
    .tooltip("Behind a feature flag")
    .color(warning)
```

The value is an `f64`. A negative one grows the other way from the zero line;
one that is not finite counts as zero. `value_label` replaces the number
written at the bar's tip, which is otherwise the value with no decimals when
it is whole and one when it is not. `tooltip` is what the tooltip calls this
bar's row, and `color` paints this one bar instead of the chart's color.

## Several series

```rust
ColumnChart::series(
    "traffic",
    ["Jan", "Feb", "Mar"],
    vec![
        ChartSeries::new("Desktop", [186.0, 305.0, 237.0]),
        ChartSeries::new("Mobile", [80.0, 200.0, 120.0]),
    ],
    palette,
)
.render()
```

`series` takes the categories and a `ChartSeries` for each measure: a name
and a value for each category. A category's bars stand side by side.
`stacked()` piles them on one another, and `total("Total")` adds a last row
to the tooltip with their sum. Each series is a lighter step of the chart's
color unless it is given one with `ChartSeries::color`.

## BarChart and ColumnChart

```rust
BarChart::new("tests", data, palette).width(480.0).color(accent).render()
ColumnChart::new("timings", data, palette).height(180.0).gradient(true).render()
```

A `ColumnChart` stands its bars on the category axis; a `BarChart` lays them
down, with the categories' names at the side. `BarChart::vertical()` turns
one into the other. Both have the same builders:

| Builder | Default | What it does |
| --- | --- | --- |
| `width(f32)` | the width offered | Fixes the width, never below 160 |
| `height(f32)` | 240, or 34 for each bar of a `BarChart` | Fixes the height, from 80 to 1200 |
| `color(Color)` | the palette's text | The color of the first series; the others are lighter steps of it |
| `stacked()` | side by side | Piles each category's series |
| `gradient(bool)` | off | Fades each bar from a third of its color at its foot to all of it at its tip |
| `radius(f32)` | 4 | Rounds the tip of each bar; the foot stays square |
| `max_bar_width(f32)` | 32 | The widest a bar is drawn |
| `grid(bool)` | on | The dashed line at each tick |
| `value_axis(bool)` | on | The ticks of the value axis |
| `value_labels(bool)` | on for a chart built from data | Each bar's value at its tip, or a stack's sum |
| `total(name)` | none | A row in the tooltip that sums a category |

The value axis always includes zero and ends on a tick; ticks are steps of 1,
2 or 5 times a power of ten. A number that is not finite is ignored by every
builder.

## LineChart and AreaChart

```rust
LineChart::new("visits", ["Jan", "Feb", "Mar"], series, palette).render()
AreaChart::new("visits", ["Jan", "Feb", "Mar"], series, palette).color(accent).render()
```

A line for each `ChartSeries` over the points of an axis. `AreaChart::new`
gives the same chart with the area under each line filled, from its color
down to nothing at the zero line; it is `LineChart::area(true)`.

| Builder | Default | What it does |
| --- | --- | --- |
| `width(f32)`, `height(f32)` | the width offered, 240 | As for a bar chart |
| `color(Color)` | the palette's text | The color of the first series; the others are lighter steps of it |
| `smooth(bool)` | on | Rounds each line through its points without passing over or under any; off joins them with straight strokes |
| `area(bool)` | off | Fills under each line |
| `dots(bool)` | off | A dot at every point |
| `grid(bool)`, `value_axis(bool)` | on | The dashed lines and the ticks |
| `point_axis(bool)` | on | The names of the points under the plot, and the room they take; the tooltip says them either way |
| `range(low, high)` | fitted to the values | A fixed value axis with ticks at both ends and at zero between them; a positive tick of an axis that runs below zero is written with its sign, as a gain is. Values past it are drawn past it |
| `dashed_zero(bool)` | off | Dashes the zero line |
| `mark(usize)` | none | A dot on every line at this point, counted from 0, as the pointer marks the one it reads |

```rust
LineChart::new("response", ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"], gain, palette)
    .range(-12.0, 12.0)
    .dashed_zero(true)
    .point_axis(false)
    .mark(state.band)
    .render()
```

## PieChart

```rust
PieChart::new("browsers", data, palette).render()
PieChart::new("browsers", data, palette).donut(0.62).center("1,125", "Visitors").render()
```

One slice for each `ChartDatum`, clockwise from twelve o'clock in their
order. `size` is the side of the square it takes, 220 by default. `donut`
makes it a ring, with the hole as a share of the radius up to 0.9, and
`center` writes a figure and a caption in the hole. A datum's `color` paints
its slice; the others are steps of the chart's `color`. A datum of zero, or
one that is negative or not finite, has no slice.

## RadarChart

```rust
RadarChart::new("profile", ["Speed", "Memory", "Startup", "Size"], series, palette)
    .maximum(100.0)
    .render()
```

Each `ChartSeries` is a shape whose corners stand on the axes at its values,
over four rings and a spoke for each axis. `maximum` is the value at the rim,
otherwise a round number over the greatest value; `size(width, height)` is
320 by 260 by default. It needs three axes or more.

## Motion

When a chart first appears it is drawn in: bars grow from the zero line,
lines are drawn from left to right, a pie sweeps round from twelve o'clock
a radar opens from its center and the columns of a heatmap come in one after
another. Under the pointer, a bar chart lights the
category with a band that glides to the next while the others recede, a line
chart marks the nearest point with a line and a dot on each series, a pie
stands the slice out, a radar marks the nearest axis, and a heatmap grows the
cell over its neighbours while the others recede. A chart keeps this
by its ID, so a chart that is rebuilt every frame does not start again.
`UiTree::set_reduced_motion(true)` draws a chart at once and moves its marks
without gliding.

## Tooltip

The tooltip is the charts' own, not the one of the other components: it
follows the pointer, on the side of it where the chart has more room and
never out of the window, and shows what the pointer reads over a row for each
series with its color, its name and its value. A pie's says the slice's value
and its share. It appears as soon as the pointer is over the chart, and
anywhere over a category, not only over its bars.

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
