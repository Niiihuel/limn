use super::{caption, row, section, Cx, Section};
use limn::{
    AlertColors, AlertTone, ButtonVariant, Component, ControlSize, Element, HeaderSizing, Icon, IconButton, ListRow, Pager,
    PagerSizing, PanelHeader, RadiusRole, ResizeHandle, SearchField, Separator, SidebarPanel, Text, TextField,
    TextRole, TooltipPlacement, TooltipSpec,
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
            TooltipSpec::new("Previous page").shortcut("Shift+F7").placement(TooltipPlacement::Below),
            TooltipSpec::new("Next page").shortcut("F7").placement(TooltipPlacement::Below),
        )
        .label_width(52.0)
        .label_tooltip(format!("{group}-where"), TooltipSpec::new(format!("Page {page} of {PAGES}")))
        .sizing(sizing);
    if vertical {
        pager.icons(Icon::ArrowUp, Icon::ArrowDown).render()
    } else {
        pager.render()
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
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
        headers.push(caption(cx, &format!("HeaderSizing::{name}: {} px high", sizing.height)));
        headers.push(framed(
            cx,
            width,
            PanelHeader::new("Source control", palette)
                .trailing(vec![
                    tool(cx, &format!("panel-{name}-refresh"), Icon::Refresh, "Refresh"),
                    tool(cx, &format!("panel-{name}-more"), Icon::More, "More actions"),
                ])
                .sizing(sizing)
                .render(),
        ));
    }
    vec![
        section(cx, "Panel header", headers),
        section(cx, "Back, close, and long title", vec![
            framed(
                cx,
                width,
                PanelHeader::new("A title too long to fit alongside its tools", palette)
                    .back("panel-back", "Back to changes")
                    .trailing(vec![
                        tool(cx, "panel-long-refresh", Icon::Refresh, "Refresh"),
                        Separator::vertical(palette).length(14.0).centered_in(24.0).margin(4.0).render(),
                        tool(cx, "panel-long-add", Icon::Add, "Add"),
                    ])
                    .dismiss("panel-dismiss", "Close")
                    .sizing(HeaderSizing::CARD)
                    .render(),
            ),
            caption(cx, "Title only, and tools aligned to the top edge (center_tools: false):"),
            framed(cx, width, PanelHeader::new("Debugging", palette).render()),
            framed(
                cx,
                width,
                PanelHeader::new("Debugging", palette)
                    .trailing(vec![tool(cx, "panel-top-refresh", Icon::Refresh, "Refresh")])
                    .sizing(HeaderSizing { center_tools: false, ..HeaderSizing::SIDEBAR })
                    .render(),
            ),
        ]),
        section(cx, "A field in place of the title", vec![framed(
            cx,
            width,
            PanelHeader::with_leading(
                TextField::new("panel-address", cx.state.text("panel-address"), "Address…", palette)
                    .size(ControlSize::Compact)
                    .render(),
                palette,
            )
                .trailing(vec![tool(cx, "panel-field-more", Icon::More, "More actions")])
                // A profile of the host's own: room for a 28 px field.
                .sizing(HeaderSizing { height: 36.0, ..HeaderSizing::PANE })
                .render(),
        )]),
        section(cx, "Sidebar panel", vec![
            SidebarPanel::new(
                vec![
                    PanelHeader::new("Search", palette)
                        .trailing(vec![tool(cx, "panel-side-refresh", Icon::Refresh, "Refresh")])
                        .render(),
                    SearchField::new("panel-side-query", cx.state.text("panel-side-query"), "Search the project…", palette)
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
            caption(cx, "6 px padding and gaps on the palette surface."),
        ]),
        section(cx, "Resize handle between panels", vec![
            caption(cx, "The strip between two panels is the resize handle: it lights up on hover, and the host moves the boundary while dragging."),
            {
                let pane = |label: &str| {
                    Element::panel(caption(cx, label)).padding(12.0).background(palette.surface).radius(RadiusRole::Surface.px()).size(Some(180.0), Some(96.0))
                };
                row(vec![Element::row(vec![
                    pane("Library"),
                    ResizeHandle::vertical("panel-resize-demo", palette).render(),
                    pane("Content"),
                ])])
            },
        ]),
        section(cx, "Pager", vec![
            row(vec![
                pager(cx, "panel-pager", PagerSizing::DEFAULT, false),
                pager(cx, "panel-pager-tight", PagerSizing { gap: 0.0 }, true),
            ]),
            caption(cx, "An unavailable direction stays inactive and keeps its tooltip."),
            row(vec![Pager::new("PDF", palette)
                .previous("panel-pager-off-prev", false)
                .next("panel-pager-off-next", false)
                .label_width(52.0)
                .render()]),
        ]),
        section(cx, "A tint behind the header", vec![
            framed(
                cx,
                width,
                Element::column(vec![
                    Text::new("An album", palette).role(TextRole::Heading).render(),
                    Text::new("The cover color fades down from the edge into the surface.", palette).muted().render(),
                    Element::spacer().size(None, Some(72.0)),
                ])
                .gap(4.0)
                .padding(12.0)
                .top_background_fade(AlertColors::for_tone(AlertTone::Info, palette).accent, 96.0),
            )
            .clip_children(),
            caption(cx, "top_background_fade(color, height): paint only, behind the container's content. bottom_background_fade does the same from below."),
        ]),
    ]
}
