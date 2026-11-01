use super::{caption, section, Cx, Section};
use limn::{
    AlertColors, AlertTone, Button, ButtonVariant, Component, ComponentPalette, ControlSize, DiffStat,
    Element, Icon, IconButton, ListRow, RadiusRole, ListRowSizing, ScrollArea, Tooltip, TooltipPlacement,
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
        .tooltip_with(TooltipSpec::new(format!("Open {name}")).placement(TooltipPlacement::Right))
        .width(width)
        .render()
}

pub fn view(cx: &Cx) -> Vec<Section> {
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
    // A mark over a picture that turns into what a click would do while the pointer is on its row.
    let palette = cx.palette;
    let over = |content: Element| Element::panel(content.centered()).background(limn::Color::rgba(0, 0, 0, 150)).radius(limn::tokens().radius.control);
    let picture = Element::panel(Element::spacer()).background(palette.selected).radius(limn::tokens().radius.control);
    let stacked = Element::stack(vec![
        picture,
        over(Element::icon(Icon::Audio).foreground(palette.text)).conceal_on_hover("lists-stack-mark"),
        over(Element::icon(Icon::PauseFill).foreground(palette.text)).reveal_on_hover("lists-stack-does"),
    ])
    .size(Some(40.0), Some(40.0));
    let stacked_row = ListRow::new("lists-stack-row", "A row with cover art", palette)
        .subtitle("The mark makes way on hover")
        .leading(stacked)
        .render()
        .hover_region_id("lists-stack-row");
    // A layer over the row's text that only dims under the pointer, so the text reads through it.
    let veil = Element::panel(Element::spacer()).background(limn::Color::rgba(120, 90, 220, 160)).radius(limn::tokens().radius.control);
    let dimmed_row = Element::overlay(
        ListRow::new("lists-dim-row", "A row under a decoration", palette).subtitle("It dims to 35 % on hover").render(),
        veil.dim_on_hover("lists-dim-veil", 0.35),
    )
    .hover_region_id("lists-dim-row");
    let under = match cx.state.text("hovered-region") {
        "" => "Under the pointer: no named row".to_owned(),
        region => format!("Under the pointer: {region}"),
    };
    vec![
        section(cx, "Layers in the same space", vec![
            card(cx, width, vec![stacked_row, dimmed_row]),
            caption(cx, "Element::stack with conceal_on_hover and reveal_on_hover: the mark at rest, the click action on hover."),
            caption(cx, "dim_on_hover fades a layer to an opacity instead of away; each row is a hover_region_id, read back with UiTree::hovered_region."),
            caption(cx, &under),
        ]),
        section(cx, "Compact", vec![card(cx, width, vec![
            ListRow::new("list-branch", "ui-separation", on_card)
                .icon(Icon::GitBranch)
                .trailing_icon(Icon::ChevronDown)
                .tooltip_with(hint("Switch branch"))
                .width(inner)
                .render(),
            ListRow::new("list-changes", "12 files", on_card)
                .icon(Icon::Code)
                .trailing(stat())
                .width(inner)
                .render(),
            ListRow::new("list-source", "list_row.rs", on_card)
                .icon(Icon::File)
                .subtitle("crates/limn/src/list_row.rs")
                .subtitle_mono(true)
                .meta("2 min ago")
                .trailing_icon(Icon::ChevronRight)
                .width(inner)
                .render(),
            ListRow::new("list-long", "A title too long to fit the width of this list", on_card)
                .icon(Icon::Book)
                .subtitle("And a second line that also overflows and ends with an ellipsis")
                .meta_element(stat())
                .width(inner)
                .render(),
            ListRow::new("list-locked", "Locked while saving", on_card)
                .icon(Icon::Folder)
                .subtitle("No click target or focus")
                .trailing_icon(Icon::ChevronDown)
                .enabled(false)
                .width(inner)
                .render(),
        ])]),
        section(cx, "Dense", vec![card(
            cx,
            width,
            FRAMES
                .iter()
                .enumerate()
                .map(|(index, (name, place))| {
                    ListRow::new(format!("list-frame-{index}"), *name, on_card)
                        .icon(Icon::Terminal)
                        .meta(*place)
                        .sizing(ListRowSizing::DENSE)
                        .width(inner)
                        .render()
                })
                .collect(),
        )]),
        section(cx, "Leading element, filled, with no fixed width", vec![
            caption(cx, "The leading element occupies a square at the top; the row takes the available width."),
            Element::row(vec![
                ListRow::new("list-run", "Run the project", cx.palette)
                    .leading(Element::spacer().background(cx.palette.selected).radius(limn::RadiusRole::Control.px()))
                    .subtitle("cargo run -p limn-gallery")
                    .subtitle_mono(true)
                    .filled()
                    .render()
                    .flex(1.0),
                Button::new("list-run-edit", "Edit", cx.palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
        ]),
        section(cx, "Subtitle color, outline, and fixed text width", vec![
            caption(cx, "The subtitle uses a status color; the outlined row is a card; the last reserves 160 px for text."),
            ListRow::new("list-account", "Account and usage", cx.palette)
                .icon(Icon::Chat)
                .subtitle("Provider · Not updated")
                .subtitle_ink(colors(AlertTone::Warning))
                .trailing_icon(Icon::ChevronRight)
                .filled()
                .width(width)
                .render(),
            ListRow::new("list-citation", "limn documentation", cx.palette)
                .icon(Icon::Search)
                .subtitle("https://example.com/limn/list-row")
                .subtitle_mono(true)
                .filled()
                .outlined()
                .width(width)
                .render(),
            ListRow::new("list-budget", "A title cut to the 160 px reserved by the row", on_card)
                .icon(Icon::Folder)
                .subtitle("The rest of the row remains available")
                .trailing_icon(Icon::ChevronDown)
                .copy_width(160.0)
                .filled()
                .width(width)
                .render(),
        ]),
        section(cx, "Sidebar", vec![
            caption(cx, "Headers with counts and disclosure controls; fading titles; the More button shares its row's hover."),
            card(cx, width, vec![
                tree_header(cx, on_card, "list-tree-open", "limn", 3, true, inner),
                task(cx, on_card, "list-task-a", "Split the application view into components", false, inner),
                task(cx, selected(cx), "list-task-b", "Sidebar rows", true, inner),
                tree_header(cx, on_card, "list-tree-folded", "documents", 128, false, inner),
            ]),
        ]),
        section(cx, "Separate actions sharing one hover", vec![
            caption(cx, "Open and stage are separate targets sharing one fill."),
            card(cx, width, vec![
                change(cx, on_card, "list-change-a", "list_row.rs", "M", true, inner),
                change(cx, on_card, "list-change-b", "tree_view.rs", "A", true, inner),
                change(cx, on_card, "list-change-c", "locked.rs", "D", false, inner),
            ]),
        ]),
        section(cx, "Change counts and path tooltip", vec![Element::row(vec![
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
        section(cx, "Bring a row into view", vec![
            caption(cx, "UiTree::reveal(id) scrolls just enough to show the whole row; the rows have different heights."),
            Element::row(vec![
                // The gallery's host calls `reveal` with what follows the prefix.
                Button::new("reveal:list-reveal-17", "Show row 18", cx.palette).variant(ButtonVariant::Subtle).render(),
                Button::new("reveal:list-reveal-0", "Back to the first", cx.palette).variant(ButtonVariant::Ghost).render(),
                Element::spacer().flex(1.0),
            ])
            .gap(8.0),
            card(cx, width, vec![ScrollArea::new(
                "list-reveal",
                Element::column(
                    (0..24)
                        .map(|index| {
                            let row = ListRow::new(format!("list-reveal-{index}"), format!("Row {}", index + 1), on_card);
                            if index % 3 == 0 { row.subtitle("With a second line") } else { row }.render()
                        })
                        .collect(),
                )
                .gap(2.0),
            )
            .render()
            .size(None, Some(150.0))]),
        ]),
        section(cx, "Sticky header while scrolling", vec![
            caption(cx, "sticky_top keeps each group's header at the top until the next arrives; a tall header shrinks to its limit, and sticky_top_fade fades rows passing beneath it."),
            card(cx, width, vec![ScrollArea::new(
                "list-sticky",
                Element::column(
                    (0..3)
                        .flat_map(|group| {
                            let title = if group == 1 {
                                "Group 2, with a long header that spans several lines in place and shrinks while pinned to the top".to_owned()
                            } else {
                                format!("Group {}", group + 1)
                            };
                            let header = Element::panel(limn::Text::new(title, on_card).render())
                                .padding(8.0)
                                .background(cx.palette.surface)
                                .radius(limn::RadiusRole::Control.px())
                                .sticky_top_max_height(36.0)
                                .sticky_top_fade(16.0);
                            std::iter::once(header).chain((0..8).map(move |index| {
                                ListRow::new(format!("list-sticky-{group}-{index}"), format!("Row {}", index + 1), on_card).render()
                            }))
                        })
                        .collect(),
                )
                .gap(2.0),
            )
            .render()
            .top_fade(cx.palette.raised, 6.0)
            .size(None, Some(180.0))]),
        ]),
    ]
}
