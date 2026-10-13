use super::{caption, section, Cx, Section};
use limn::{
    Button, ButtonSize, ButtonVariant, CardGrid, Component, Element, Icon, IconButton, PageColumn, PageHeader,
    PageHeaderSizing, RadiusRole, ScrollbarAppearance, SectionHeader, SectionHeaderSizing, Text,
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
            Text::new("Card description.", cx.palette).muted().render(),
        ])
        .gap(4.0),
    )
    .background(cx.palette.raised)
    .border(cx.palette.border, 1.0)
    .radius(RadiusRole::Card.px())
    .padding(12.0)
    .size(Some(width), None)
}

fn grid(cx: &Cx, width: f32, breakpoint: f32, count: usize) -> Element {
    let grid = CardGrid::new(width, breakpoint);
    let item_width = grid.item_width();
    grid.items((1..=count).map(|index| card(cx, &format!("Card {index}"), item_width)).collect())
        .render()
}

/// A whole page in a frame, so its gutter and its own scrolling show.
fn page(cx: &Cx, id: &str, width: f32, tint: bool) -> Element {
    // Breakpoints scaled down to the frames, so the gallery shows both states.
    let page = PageColumn::new(id, width, cx.palette)
        .gutter(14.0, 28.0, 360.0)
        .reserve_rail(true)
        .tint_scrollbar(tint);
    let content_width = page.content_width();
    let page = page.children(vec![
        PageHeader::new("Automations", cx.palette)
            .description("Scheduled tasks for this project")
            .trailing(Button::new(format!("{id}-new"), "New task", cx.palette).icon(Icon::Add).render())
            .stacked(content_width < 300.0)
            .render(),
        SectionHeader::new("Templates", cx.palette).count(5).render(),
        grid(cx, content_width, 300.0, 5),
    ]);
    Element::panel(page.render())
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Panel.px())
        .clip_children()
        .size(Some(width), Some(260.0))
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    // This page overflows, and the gallery's scrollbar then takes its rail.
    let width = (cx.width - ScrollbarAppearance::default().rail_width).min(720.0);
    let action = |id: &str| Button::new(id, "Refresh", palette).variant(ButtonVariant::Outline).render();
    let open = !cx.state.flag("page-section-fold");
    vec![
        section(cx, "Page header", vec![
            PageHeader::new("Providers", palette)
                .description("Connected accounts and models available to the agent.")
                .trailing(action("page-header-row"))
                .render()
                .size(Some(width), None),
            caption(cx, "Stacked, for a narrow page"),
            PageHeader::new("Providers", palette)
                .description("Connected accounts and models available to the agent.")
                .trailing(action("page-header-stacked"))
                .stacked(true)
                .render()
                .size(Some(width.min(320.0)), None),
            caption(cx, "Leading element, truncated line, and fixed height"),
            PageHeader::new("Agent plugin marketplace", palette)
                .description("Skills for your agent, per user or per project")
                .leading(IconButton::new("page-header-back", Icon::ArrowLeft, palette).variant(ButtonVariant::Ghost).render())
                .trailing(action("page-header-cut"))
                .truncate(true)
                .sizing(CATALOG)
                .render()
                .size(Some(width.min(360.0)), None),
            caption(cx, "Trailing element on the title line, without centering"),
            PageHeader::new("Automations", palette)
                .description("Scheduled tasks for this project, with a description spanning multiple lines on a narrow page.")
                .trailing(action("page-header-top"))
                .trailing_top(true)
                .render()
                .size(Some(width.min(420.0)), None),
            caption(cx, "Title only"),
            PageHeader::new("Keyboard shortcuts", palette).render(),
        ]),
        section(cx, "Section headers", vec![
            Element::column(vec![
                SectionHeader::new("Recommended", palette).count(7).render(),
                SectionHeader::new("Installed", palette).count(128).render(),
                SectionHeader::new("Changes", palette)
                    .disclosure(Some(open))
                    .detail(if open { "3 files" } else { "hidden" })
                    .action("page-section-fold")
                    .sizing(SectionHeaderSizing::LIST)
                    .render(),
                SectionHeader::new("Sources", palette)
                    .detail("12")
                    .sizing(SectionHeaderSizing::LABEL)
                    .render(),
                SectionHeader::new("A section title too long to fit on one line", palette)
                    .detail("128")
                    .truncate(true)
                    .sizing(SectionHeaderSizing::LIST)
                    .width(280.0)
                    .render(),
                SectionHeader::new("Tools", palette)
                    .trailing(
                        IconButton::new("page-section-add", Icon::Add, palette)
                            .variant(ButtonVariant::Ghost)
                            .button_size(ButtonSize::Mini)
                            .render(),
                    )
                    .sizing(SectionHeaderSizing::LIST)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width.min(420.0)), None),
        ]),
        section(cx, "Card grid", vec![
            grid(cx, width, 360.0, 3),
            caption(cx, "Below the breakpoint, a single column"),
            grid(cx, width.min(300.0), 360.0, 2),
        ]),
        section(cx, "Page column", vec![
            page(cx, "page-column-wide", width, true),
            caption(cx, "Narrow: smaller gutter, stacked header, and one column"),
            page(cx, "page-column-narrow", width.min(300.0), true),
            caption(cx, "With the scrollbar's own color, without tinting"),
            page(cx, "page-column-plain", width.min(300.0), false),
        ]),
    ]
}
