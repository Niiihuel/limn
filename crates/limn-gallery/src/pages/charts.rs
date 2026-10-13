use super::{caption, row, section, Cx, Section};
use limn::{
    AlertColors, AlertTone, BarChart, ChartAxisLabel, ChartDatum, ChartLegend, Color, ColumnChart, Component,
    HeatmapCell, HeatmapChart, HeatmapScale,
};

const LANGUAGES: [(&str, f64); 5] = [("Rust", 48.0), ("TypeScript", 31.0), ("Python", 17.0), ("Shell", 6.0), ("Nix", 0.0)];
const MONTHS: [(usize, &str); 5] = [(0, "Jan"), (5, "Feb"), (9, "Mar"), (13, "Apr"), (18, "May")];
const WEEKS: usize = 20;

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
        section(cx, "Bar chart", vec![
            BarChart::new("charts-bars", languages(), palette).width(width).render(),
            caption(cx, "One linear scale from zero: the longest bar fills the track, and a zero draws none."),
            BarChart::new(
                "charts-bars-ink",
                vec![
                    ChartDatum::new("passed", "Passed", 412.0),
                    ChartDatum::new("skipped", "Skipped", 37.0).value_label("37 (8 %)").tooltip("Skipped: 37 tests behind a feature flag"),
                    ChartDatum::new("failed", "Failed", 9.0).color(AlertColors::for_tone(AlertTone::Error, palette).accent),
                ],
                palette,
            )
            .width(width)
            .color(accent)
            .render(),
            caption(cx, ".color(accent) on the chart, and on one datum its own color, value label and tooltip."),
        ]),
        section(cx, "Column chart", vec![
            BarChart::new("charts-columns", languages(), palette).width(width).vertical().render(),
            caption(cx, "BarChart::vertical(): the same data as columns, with whole-number ticks."),
            ColumnChart::new(
                "charts-columns-seconds",
                vec![
                    ChartDatum::new("parse", "Parse", 0.3),
                    ChartDatum::new("check", "Check", 1.4),
                    ChartDatum::new("link", "Link", 0.8),
                ],
                palette,
            )
            .width(width)
            .height(180.0)
            .max_bar_width(28.0)
            .integer_ticks(false)
            .color(accent)
            .render(),
            caption(cx, ".integer_ticks(false) for a measure with fractions, 180 px high with columns of 28 px at most."),
            ColumnChart::new(
                "charts-columns-dense",
                (1..=16).map(|day| ChartDatum::new(day.to_string(), format!("Day {day}"), ((day * 7) % 11) as f64)).collect(),
                palette,
            )
            .width(width)
            .height(180.0)
            .render(),
            caption(cx, "More categories than fit: the columns scroll sideways and the value axis stays."),
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
