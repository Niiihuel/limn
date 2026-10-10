use super::{caption, section, Cx};
use limn::{
    Button, ButtonSize, ButtonVariant, CardGrid, Component, Element, Icon, IconButton, PageColumn, PageHeader,
    PageHeaderSizing, RadiusRole, ScrollbarAppearance, SectionHeader, SectionSizing, Text,
};

/// A catalog's heading: one cut line each, on the row of its actions.
const CATALOG: PageHeaderSizing = PageHeaderSizing {
    title_size: 20.0,
    copy_gap: 2.0,
    row_gap: 6.0,
    height: 40.0,
    ..PageHeaderSizing::DEFAULT
};

fn card(cx: &Cx, title: &str, width: f32) -> Element {
    Element::panel(
        Element::column(vec![
            Text::new(title, cx.palette).render(),
            Text::new("Descripción de la tarjeta.", cx.palette).muted().render(),
        ])
        .gap(4.0),
    )
    .background(cx.palette.raised)
    .border(cx.palette.border, 1.0)
    .radius(RadiusRole::Card.px())
    .padding(12.0)
    .size(Some(width), None)
}

fn grid(cx: &Cx, width: f32, count: usize) -> Element {
    let grid = CardGrid::new(width, 360.0);
    let item_width = grid.item_width();
    grid.items((1..=count).map(|index| card(cx, &format!("Tarjeta {index}"), item_width)).collect())
        .render()
}

/// A whole page in a frame, so its gutter and its own scrolling show.
fn page(cx: &Cx, id: &str, width: f32) -> Element {
    let page = PageColumn::new(id, width, cx.palette).reserve_rail(true);
    let content_width = page.content_width();
    let page = page.children(vec![
        PageHeader::new("Automatizaciones", cx.palette)
            .description("Tareas programadas para este proyecto")
            .trailing(Button::new(format!("{id}-new"), "Nueva tarea", cx.palette).icon(Icon::Add).render())
            .stacked(content_width < 440.0)
            .render(),
        SectionHeader::new("Plantillas", cx.palette).count(5).render(),
        grid(cx, content_width, 5),
    ]);
    Element::panel(page.render())
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Panel.px())
        .clip_children()
        .size(Some(width), Some(260.0))
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    // This page overflows, and the gallery's scrollbar then takes its rail.
    let width = (cx.width - ScrollbarAppearance::default().rail_width).min(720.0);
    let action = |id: &str| Button::new(id, "Actualizar", palette).variant(ButtonVariant::Outline).render();
    let open = !cx.state.flag("page-section-fold");
    Element::column(vec![
        section(cx, "Encabezado de página", vec![
            PageHeader::new("Proveedores", palette)
                .description("Cuentas conectadas y modelos disponibles para el agente.")
                .trailing(action("page-header-row"))
                .render()
                .size(Some(width), None),
            caption(cx, "Apilado, para una página estrecha"),
            PageHeader::new("Proveedores", palette)
                .description("Cuentas conectadas y modelos disponibles para el agente.")
                .trailing(action("page-header-stacked"))
                .stacked(true)
                .render()
                .size(Some(width.min(320.0)), None),
            caption(cx, "Con elemento inicial, una línea cortada y alto fijo"),
            PageHeader::new("Marketplace de complementos del agente", palette)
                .description("Skills para tu agente, por usuario o por proyecto")
                .leading(IconButton::new("page-header-back", Icon::ArrowLeft, palette).variant(ButtonVariant::Ghost).render())
                .trailing(action("page-header-cut"))
                .truncate(true)
                .sizing(CATALOG)
                .render()
                .size(Some(width.min(360.0)), None),
            caption(cx, "Sólo título"),
            PageHeader::new("Atajos de teclado", palette).render(),
        ]),
        section(cx, "Encabezados de sección", vec![
            Element::column(vec![
                SectionHeader::new("Recomendadas", palette).count(7).render(),
                SectionHeader::new("Instaladas", palette).count(128).render(),
                SectionHeader::new("Cambios", palette)
                    .disclosure(Some(open))
                    .detail(if open { "3 archivos" } else { "oculto" })
                    .action("page-section-fold")
                    .sizing(SectionSizing::LIST)
                    .render(),
                SectionHeader::new("Fuentes", palette)
                    .detail("12")
                    .sizing(SectionSizing::LABEL)
                    .render(),
                SectionHeader::new("Herramientas", palette)
                    .trailing(
                        IconButton::new("page-section-add", Icon::Add, palette)
                            .variant(ButtonVariant::Ghost)
                            .button_size(ButtonSize::Mini)
                            .render(),
                    )
                    .sizing(SectionSizing::LIST)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width.min(420.0)), None),
        ]),
        section(cx, "Rejilla de tarjetas", vec![
            grid(cx, width, 3),
            caption(cx, "Por debajo del corte, una sola columna"),
            grid(cx, width.min(300.0), 2),
        ]),
        section(cx, "Columna de página", vec![
            page(cx, "page-column-wide", width),
            page(cx, "page-column-narrow", width.min(300.0)),
        ]),
    ])
    .gap(32.0)
}
