use super::{caption, section, Cx};
use limn::{
    AlertColors, AlertTone, Button, ButtonVariant, Component, ComponentPalette, ControlSize, DiffStat,
    Element, Icon, IconButton, ListRow, RadiusRole, RowSizing, ScrollArea, Tooltip, TooltipPlacement,
    TooltipSpec,
    TreeGuides, TreeRow,
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

/// The palette of the row the host calls selected: its fill is the list's
/// selected color.
fn selected(cx: &Cx) -> ComponentPalette {
    ComponentPalette { surface: cx.palette.selected, hover: cx.palette.selected, ..cx.palette }
}

fn tree_header(cx: &Cx, palette: ComponentPalette, id: &str, title: &str, count: usize, open: bool, width: f32) -> Element {
    let id = id.to_owned();
    TreeRow::new(
        move |content| Element::clickable(id, content),
        Element::icon(limn::folder_icon(title, open)).foreground(cx.palette.muted),
        Element::truncated_text(title)
            .foreground(cx.palette.text)
            .font_family(limn::FontFamily::Sans)
            .font_size(12.0),
        TreeGuides::root(),
        palette,
    )
    .count(count)
    .disclosure(open)
    .width(width)
    .render()
}

fn task(cx: &Cx, palette: ComponentPalette, id: &str, title: &str, filled: bool, width: f32) -> Element {
    let more = format!("{id}-more");
    let row = ListRow::new(id, title, palette)
        .icon(Icon::Terminal)
        .title_fade(true)
        .trailing(
            IconButton::new(&more, Icon::More, cx.palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .render()
                .centered_y(),
        )
        .hover_with(more)
        .radius(RadiusRole::Surface)
        .width(width);
    if filled { row.filled() } else { row }.render()
}

fn change(cx: &Cx, palette: ComponentPalette, id: &str, name: &str, status: &str, enabled: bool, width: f32) -> Element {
    let stage = format!("{id}-stage");
    ListRow::new(id, name, palette)
        .icon(Icon::File)
        .meta(status)
        .enabled(enabled)
        .trailing_action(
            &stage,
            // No fill of its own: the row's shared surface is its hover.
            Element::clickable(&stage, Element::icon(Icon::Add).foreground(cx.palette.muted))
                .size(Some(24.0), None),
        )
        .tooltip_with(TooltipSpec::new(format!("Abrir {name}")).placement(TooltipPlacement::Right))
        .width(width)
        .render()
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
        section(cx, "Tinta de la segunda línea, contorno y texto de ancho fijo", vec![
            caption(cx, "La segunda línea lleva un color de estado; la fila contorneada es una tarjeta; la última guarda 160 px para su texto."),
            ListRow::new("list-account", "Cuenta y uso", cx.palette)
                .icon(Icon::Chat)
                .subtitle("Proveedor · Sin actualizar")
                .subtitle_ink(colors(AlertTone::Warning))
                .trailing_icon(Icon::ChevronRight)
                .filled()
                .width(width)
                .render(),
            ListRow::new("list-citation", "Documentación de limn", cx.palette)
                .icon(Icon::Search)
                .subtitle("https://example.com/limn/list-row")
                .subtitle_mono(true)
                .filled()
                .outlined()
                .width(width)
                .render(),
            ListRow::new("list-budget", "Un título que se corta a los 160 px que la fila le reserva", on_card)
                .icon(Icon::Folder)
                .subtitle("El resto de la fila queda libre")
                .trailing_icon(Icon::ChevronDown)
                .copy_width(160.0)
                .filled()
                .width(width)
                .render(),
        ]),
        section(cx, "Barra lateral", vec![
            caption(cx, "Cabeceras con recuento y pliegue; títulos que se desvanecen; el botón Más mantiene el hover de su fila."),
            card(cx, width, vec![
                tree_header(cx, on_card, "list-tree-open", "limn", 3, true, inner),
                task(cx, on_card, "list-task-a", "Separar la presentación de la aplicación en componentes", false, inner),
                task(cx, selected(cx), "list-task-b", "Filas de la barra lateral", true, inner),
                tree_header(cx, on_card, "list-tree-folded", "documentos", 128, false, inner),
            ]),
        ]),
        section(cx, "Acciones propias bajo un mismo hover", vec![
            caption(cx, "Abrir y preparar son dos destinos; el relleno es uno solo."),
            card(cx, width, vec![
                change(cx, on_card, "list-change-a", "list_row.rs", "M", true, inner),
                change(cx, on_card, "list-change-b", "tree_view.rs", "A", true, inner),
                change(cx, on_card, "list-change-c", "bloqueada.rs", "D", false, inner),
            ]),
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
        section(cx, "Llevar una fila a la vista", vec![
            caption(cx, "UiTree::reveal(id) desplaza lo justo para mostrarla entera; las filas no miden lo mismo."),
            Element::row(vec![
                // The gallery's host calls `reveal` with what follows the prefix.
                Button::new("reveal:list-reveal-17", "Mostrar la fila 18", cx.palette).variant(ButtonVariant::Subtle).render(),
                Button::new("reveal:list-reveal-0", "Volver a la primera", cx.palette).variant(ButtonVariant::Ghost).render(),
                Element::spacer().flex(1.0),
            ])
            .gap(8.0),
            card(cx, width, vec![ScrollArea::new(
                "list-reveal",
                Element::column(
                    (0..24)
                        .map(|index| {
                            let row = ListRow::new(format!("list-reveal-{index}"), format!("Fila {}", index + 1), on_card);
                            if index % 3 == 0 { row.subtitle("Con una segunda línea") } else { row }.render()
                        })
                        .collect(),
                )
                .gap(2.0),
            )
            .render()
            .size(None, Some(150.0))]),
        ]),
    ])
    .gap(32.0)
}
