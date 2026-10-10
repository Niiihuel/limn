use super::{caption, section, Cx};
use limn::{
    AlertColors, AlertTone, Button, ButtonVariant, Component, ComponentPalette, ControlSize, DiffStat,
    Element, Icon, ListRow, RowSizing, Tooltip, TooltipPlacement, TooltipSpec,
};

const FRAMES: [(&str, &str); 4] = [
    ("render", "list_row.rs:214"),
    ("place_flex", "tree.rs:8816"),
    ("layout_at_with_hover_refresh", "tree.rs:2566"),
    ("main", "main.rs:12"),
];

/// Rows on a card, as a list in a panel has them.
fn card(cx: &Cx, width: f32, rows: Vec<Element>) -> Element {
    Element::column(rows)
        .gap(2.0)
        .padding(6.0)
        .background(cx.palette.raised)
        .radius(limn::RadiusRole::Card.px())
        .size(Some(width), None)
}

pub fn view(cx: &Cx) -> Element {
    let width = cx.width.min(360.0);
    let inner = width - 12.0;
    // A row hovers one step above the surface it sits on.
    let on_card = ComponentPalette { surface: cx.palette.raised, hover: cx.palette.selected, ..cx.palette };
    let colors = |tone| AlertColors::for_tone(tone, cx.palette).accent;
    let stat = || {
        DiffStat::new(128, 17, cx.palette)
            .inks(colors(AlertTone::Success), colors(AlertTone::Error))
            .render()
    };
    let hint = |title: &str| TooltipSpec::new(title).placement(TooltipPlacement::Right);
    Element::column(vec![
        section(cx, "Compacta", vec![card(cx, width, vec![
            ListRow::new("list-branch", "ui-separation", on_card)
                .icon(Icon::GitBranch)
                .trailing_icon(Icon::ChevronDown)
                .tooltip_with(hint("Cambiar de rama"))
                .width(inner)
                .render(),
            ListRow::new("list-changes", "12 archivos", on_card)
                .icon(Icon::Code)
                .trailing(stat())
                .width(inner)
                .render(),
            ListRow::new("list-source", "list_row.rs", on_card)
                .icon(Icon::File)
                .subtitle("crates/limn/src/list_row.rs")
                .subtitle_mono(true)
                .meta("hace 2 min")
                .trailing_icon(Icon::ChevronRight)
                .width(inner)
                .render(),
            ListRow::new("list-long", "Un título demasiado largo para caber en el ancho de esta lista", on_card)
                .icon(Icon::Book)
                .subtitle("Y una segunda línea que tampoco cabe y se corta con puntos suspensivos")
                .meta_element(stat())
                .width(inner)
                .render(),
            ListRow::new("list-locked", "Bloqueada mientras se guarda", on_card)
                .icon(Icon::Folder)
                .subtitle("Sin región de clic ni foco")
                .trailing_icon(Icon::ChevronDown)
                .enabled(false)
                .width(inner)
                .render(),
        ])]),
        section(cx, "Densa", vec![card(
            cx,
            width,
            FRAMES
                .iter()
                .enumerate()
                .map(|(index, (name, place))| {
                    ListRow::new(format!("list-frame-{index}"), *name, on_card)
                        .icon(Icon::Terminal)
                        .meta(*place)
                        .sizing(RowSizing::DENSE)
                        .width(inner)
                        .render()
                })
                .collect(),
        )]),
        section(cx, "Con elemento inicial, rellena y sin ancho fijo", vec![
            caption(cx, "El elemento inicial ocupa un cuadrado arriba; la fila toma el ancho que le dan."),
            Element::row(vec![
                ListRow::new("list-run", "Ejecutar el proyecto", cx.palette)
                    .leading(Element::spacer().background(cx.palette.selected).radius(limn::RadiusRole::Control.px()))
                    .subtitle("cargo run -p limn-gallery")
                    .subtitle_mono(true)
                    .filled()
                    .render()
                    .flex(1.0),
                Button::new("list-run-edit", "Editar", cx.palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
        ]),
        section(cx, "Recuento de cambios y pista de ruta", vec![Element::row(vec![
            stat(),
            DiffStat::new(3, 0, cx.palette).render(),
            Tooltip::for_path(
                Button::new("list-path", "tree.rs", cx.palette).variant(ButtonVariant::Ghost).render(),
                "list-path",
                "crates/limn/src/tree.rs",
                cx.palette,
            )
            .placement(TooltipPlacement::Right)
            .render(),
            Element::spacer().flex(1.0),
        ])
        .gap(16.0)]),
    ])
    .gap(32.0)
}
