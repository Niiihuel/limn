use super::{caption, row, section, Cx};
use limn::{
    ButtonVariant, Component, ControlSize, Element, HeaderSizing, Icon, IconButton, ListRow, Pager,
    PagerSizing, PanelHeader, RadiusRole, ResizeHandle, SearchField, Separator, SidebarPanel, TextField,
    TooltipPlacement, TooltipSpec,
};

const PAGES: usize = 12;

fn tool(cx: &Cx, id: &str, icon: Icon, hint: &str) -> Element {
    IconButton::new(id, icon, cx.palette)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Dense)
        .tooltip_with(TooltipSpec::new(hint).placement(TooltipPlacement::Below))
        .render()
}

/// A header in an outline that shows its height.
fn framed(cx: &Cx, width: f32, header: Element) -> Element {
    // On the surface, not a card: the palette's border has the card's own color.
    Element::panel(header)
        .background(cx.palette.surface)
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Surface.px())
        .size(Some(width), None)
}

/// A pager over `PAGES` pages that the gallery's state steps through.
fn pager(cx: &Cx, group: &str, sizing: PagerSizing, vertical: bool) -> Element {
    let page: usize = cx.state.choice(group, "1").parse().unwrap_or(1).clamp(1, PAGES);
    let pager = Pager::new(format!("{page} / {PAGES}"), cx.palette)
        .previous(format!("{group}={}", page.saturating_sub(1).max(1)), page > 1)
        .next(format!("{group}={}", (page + 1).min(PAGES)), page < PAGES)
        .tooltips(
            TooltipSpec::new("Página anterior").shortcut("Shift+F7").placement(TooltipPlacement::Below),
            TooltipSpec::new("Página siguiente").shortcut("F7").placement(TooltipPlacement::Below),
        )
        .label_width(52.0)
        .label_tooltip(format!("{group}-where"), TooltipSpec::new(format!("Página {page} de {PAGES}")))
        .sizing(sizing);
    if vertical {
        pager.icons(Icon::ArrowUp, Icon::ArrowDown).render()
    } else {
        pager.render()
    }
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(320.0);
    let profiles = [
        ("SIDEBAR", HeaderSizing::SIDEBAR),
        ("PANE", HeaderSizing::PANE),
        ("CARD", HeaderSizing::CARD),
        ("POPOVER", HeaderSizing::POPOVER),
    ];
    let mut headers = Vec::new();
    for (name, sizing) in profiles {
        headers.push(caption(cx, &format!("HeaderSizing::{name}: {} de alto", sizing.height)));
        headers.push(framed(
            cx,
            width,
            PanelHeader::new("Control de versiones", palette)
                .trailing(vec![
                    tool(cx, &format!("panel-{name}-refresh"), Icon::Refresh, "Actualizar"),
                    tool(cx, &format!("panel-{name}-more"), Icon::More, "Más acciones"),
                ])
                .sizing(sizing)
                .render(),
        ));
    }
    Element::column(vec![
        section(cx, "Cabecera de panel", headers),
        section(cx, "Volver, cerrar y título largo", vec![
            framed(
                cx,
                width,
                PanelHeader::new("Un título demasiado largo para caber junto a sus herramientas", palette)
                    .back("panel-back", "Volver a los cambios")
                    .trailing(vec![
                        tool(cx, "panel-long-refresh", Icon::Refresh, "Actualizar"),
                        Separator::vertical(palette).length(14.0).centered_in(24.0).margin(4.0).render(),
                        tool(cx, "panel-long-add", Icon::Add, "Añadir"),
                    ])
                    .dismiss("panel-dismiss", "Cerrar")
                    .sizing(HeaderSizing::CARD)
                    .render(),
            ),
            caption(cx, "Sólo título, y herramientas colgadas del borde superior (center_tools: false):"),
            framed(cx, width, PanelHeader::new("Depuración", palette).render()),
            framed(
                cx,
                width,
                PanelHeader::new("Depuración", palette)
                    .trailing(vec![tool(cx, "panel-top-refresh", Icon::Refresh, "Actualizar")])
                    .sizing(HeaderSizing { center_tools: false, ..HeaderSizing::SIDEBAR })
                    .render(),
            ),
        ]),
        section(cx, "Un campo en lugar del título", vec![framed(
            cx,
            width,
            PanelHeader::new("", palette)
                .leading(
                    TextField::new("panel-address", cx.state.text("panel-address"), "Dirección…", palette)
                        .size(ControlSize::Compact)
                        .render(),
                )
                .trailing(vec![tool(cx, "panel-field-more", Icon::More, "Más acciones")])
                // A profile of the host's own: room for a 28 px field.
                .sizing(HeaderSizing { height: 36.0, ..HeaderSizing::PANE })
                .render(),
        )]),
        section(cx, "Panel lateral", vec![
            SidebarPanel::new(
                vec![
                    PanelHeader::new("Buscar", palette)
                        .trailing(vec![tool(cx, "panel-side-refresh", Icon::Refresh, "Actualizar")])
                        .render(),
                    SearchField::new("panel-side-query", cx.state.text("panel-side-query"), "Buscar en el proyecto…", palette)
                        .size(ControlSize::Compact)
                        .render(),
                    Element::column(
                        ["main.rs", "lib.rs", "Cargo.toml"]
                            .into_iter()
                            .map(|name| ListRow::new(format!("panel-side-{name}"), name, palette).icon(Icon::File).render())
                            .collect(),
                    )
                    .gap(2.0),
                ],
                palette,
            )
            .size(width, 190.0)
            .render(),
            caption(cx, "Margen y separación de 6 sobre la superficie de la paleta."),
        ]),
        section(cx, "Tirador entre paneles", vec![
            caption(cx, "La franja entre dos paneles es el tirador: se ilumina bajo el puntero y el anfitrión mueve el límite con el arrastre."),
            {
                let pane = |label: &str| {
                    Element::panel(caption(cx, label)).padding(12.0).background(palette.surface).radius(RadiusRole::Surface.px()).size(Some(180.0), Some(96.0))
                };
                row(vec![Element::row(vec![
                    pane("Biblioteca"),
                    ResizeHandle::vertical("panel-resize-demo", palette).render(),
                    pane("Contenido"),
                ])])
            },
        ]),
        section(cx, "Paginador", vec![
            row(vec![
                pager(cx, "panel-pager", PagerSizing::DEFAULT, false),
                pager(cx, "panel-pager-tight", PagerSizing { gap: 0.0 }, true),
            ]),
            caption(cx, "El extremo que no se puede tomar es pasivo y conserva su pista."),
            row(vec![Pager::new("PDF", palette)
                .previous("panel-pager-off-prev", false)
                .next("panel-pager-off-next", false)
                .label_width(52.0)
                .render()]),
        ]),
    ])
    .gap(32.0)
}
