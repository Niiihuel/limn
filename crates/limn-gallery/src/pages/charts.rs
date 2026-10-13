use super::{caption, row, section, Cx, Section};
use limn::{
    AlertColors, AlertTone, BarChart, ChartAxisLabel, ChartDatum, ChartLegend, ChartSeries, Color, ColumnChart, Component,
    HeatmapCell, HeatmapChart, HeatmapScale,
};

const LANGUAGES: [(&str, f64); 5] = [("Rust", 48.0), ("TypeScript", 31.0), ("Python", 17.0), ("Shell", 6.0), ("Nix", 0.0)];
const MONTHS: [(usize, &str); 5] = [(0, "Jan"), (5, "Feb"), (9, "Mar"), (13, "Apr"), (18, "May")];
const WEEKS: usize = 20;
const MONTHS_HALF: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

fn traffic() -> Vec<ChartSeries> {
    vec![
        ChartSeries::new("Desktop", [186.0, 305.0, 237.0, 73.0, 209.0, 214.0]),
        ChartSeries::new("Mobile", [80.0, 200.0, 120.0, 190.0, 130.0, 140.0]),
    ]
}

fn languages() -> Vec<ChartDatum> {
    LANGUAGES.iter().map(|(name, files)| ChartDatum::new(name.to_lowercase(), *name, *files)).collect()
}

/// Twenty weeks of made-up counts. The period starts on a Wednesday and ends
/// on a Thursday, so the days outside it are holes, not zeros.
fn weeks() -> Vec<Vec<Option<HeatmapCell>>> {
    (0..WEEKS)
        .map(|week| {
            (0..7)
                .map(|day| {
                    if (week == 0 && day < 2) || (week == WEEKS - 1 && day > 3) {
                        return None;
                    }
                    let count = (week * 3 + day * 5) % 9;
                    let level = count.div_ceil(2) as u8;
                    Some(HeatmapCell::new(format!("{week}-{day}"), level, format!("{count} commits")))
                })
                .collect()
        })
        .collect()
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(480.0);
    let accent = AlertColors::for_tone(AlertTone::Info, palette).accent;
    let tinted = HeatmapScale::new([
        palette.raised,
        Color { a: 70, ..accent },
        Color { a: 125, ..accent },
        Color { a: 190, ..accent },
        accent,
    ]);
    vec![
        section(cx, "Column chart", vec![
            ColumnChart::new("charts-columns", languages(), palette).width(width).gradient(true).render(),
            caption(cx, "One column for each datum, written with its value. gradient(true) fades each from its foot to its tip. Move the pointer over the chart."),
            ColumnChart::series("charts-grouped", MONTHS_HALF, traffic(), palette).width(width).gradient(true).radius(6.0).render(),
            caption(cx, "ColumnChart::series: two series over the same months, side by side. The tooltip lists both."),
            ColumnChart::series("charts-stacked", MONTHS_HALF, traffic(), palette)
                .width(width)
                .stacked()
                .total("Total")
                .value_labels(true)
                .color(accent)
                .render(),
            caption(cx, ".stacked().total(\"Total\").color(accent): one pile for each month, its sum over it and in the tooltip."),
            ColumnChart::new(
                "charts-negative",
                [("Mon", 4.0), ("Tue", -2.0), ("Wed", 6.5), ("Thu", -4.5), ("Fri", 3.0)]
                    .into_iter()
                    .map(|(day, change)| {
                        let datum = ChartDatum::new(day, day, change);
                        if change < 0.0 { datum.color(AlertColors::for_tone(AlertTone::Error, palette).accent) } else { datum }
                    })
                    .collect(),
                palette,
            )
            .width(width)
            .height(200.0)
            .render(),
            caption(cx, "A negative value grows down from the zero line; here each has a color of its own."),
        ]),
        section(cx, "Bar chart", vec![
            BarChart::new("charts-bars", languages(), palette).width(width).gradient(true).render(),
            caption(cx, "The same data lying down: BarChart, with the categories' names down the side."),
            BarChart::new(
                "charts-bars-ink",
                vec![
                    ChartDatum::new("passed", "Passed", 412.0),
                    ChartDatum::new("skipped", "Skipped", 37.0).value_label("37 (8 %)").tooltip("Behind a feature flag"),
                    ChartDatum::new("failed", "Failed", 9.0).color(AlertColors::for_tone(AlertTone::Error, palette).accent),
                ],
                palette,
            )
            .width(width)
            .color(accent)
            .value_axis(false)
            .grid(false)
            .render(),
            caption(cx, ".color(accent).value_axis(false).grid(false), and on one datum its own color, value label and name in the tooltip."),
            BarChart::series("charts-bars-stacked", ["Engine", "Components", "Gallery"], vec![
                ChartSeries::new("Code", [16.6, 21.4, 6.2]),
                ChartSeries::new("Tests", [8.1, 9.7, 0.4]),
                ChartSeries::new("Docs", [1.9, 3.3, 0.2]),
            ], palette)
            .width(width)
            .stacked()
            .total("Thousand lines")
            .render(),
            caption(cx, "Three series stacked, each a lighter step of the chart's color."),
        ]),
        section(cx, "Heatmap chart", vec![
            HeatmapChart::new("charts-heat", weeks(), palette)
                .width(width)
                .row_labels(["", "Tue", "", "Thu", "", "Sat", ""])
                .column_labels(MONTHS.iter().map(|(column, month)| ChartAxisLabel::new(*column, *month)).collect())
                .render(),
            caption(cx, "A column per week. The days outside the period are holes; a day with nothing is a cell at level zero."),
            HeatmapChart::new("charts-heat-tinted", weeks(), palette)
                .width(width.min(260.0))
                .cell_size(10.0)
                .gap(2.0)
                .scale(tinted)
                .render(),
            caption(cx, ".scale(HeatmapScale::new(colors)) with .cell_size(10.0) and .gap(2.0), in 260 px."),
        ]),
        section(cx, "Chart legend", vec![
            row(vec![ChartLegend::new(HeatmapScale::neutral(palette).colors, "Less", "More", palette).render()]),
            row(vec![ChartLegend::new(tinted.colors, "0", "8 commits", palette).render()]),
            caption(cx, "The colors of a scale between a label for each end."),
        ]),
    ]
}
