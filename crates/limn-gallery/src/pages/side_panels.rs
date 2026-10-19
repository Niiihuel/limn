use super::{caption, row, section, Cx, Section};
use limn::{
    ButtonVariant, Color, Component, ControlSize, Element, Icon, IconButton, ListRow, PanelHeader, PeekPanel,
    RadiusRole, Side, SidebarPanel, SideRail, SideRailItem, TooltipPlacement, TooltipSpec,
};

/// A library: an ID, a title, the line under it, a cover's color, and whether it is a person.
const LIBRARY: &[(&str, &str, &str, (u8, u8, u8), bool)] = &[
    ("liked", "Liked Songs", "Playlist · 312 songs", (80, 56, 200), false),
    ("discover", "Discover Weekly", "Playlist · Made for you", (30, 120, 90), false),
    ("nina", "Nina Simone", "Artist", (170, 110, 60), true),
    ("blue", "Kind of Blue", "Album · Miles Davis", (40, 70, 140), false),
    ("radio", "Daily Mix 1", "Playlist · Made for you", (150, 40, 70), false),
];

const FRAME_HEIGHT: f32 = 360.0;

fn cover(rgb: (u8, u8, u8)) -> Element {
    Element::spacer().background(Color::rgb(rgb.0, rgb.1, rgb.2))
}

fn rail(cx: &Cx, id: &str, count: usize) -> SideRail {
    let items = (0..count)
        .map(|index| {
            let (key, title, subtitle, rgb, person) = LIBRARY[index % LIBRARY.len()];
            let key = if index < LIBRARY.len() { key.to_owned() } else { format!("{key}-{index}") };
            SideRailItem::new(format!("{id}-current={key}"), title).subtitle(subtitle).picture(cover(rgb)).round(person)
        })
        .collect();
    let current = format!("{id}-current={}", cx.state.choice(&format!("{id}-current"), "discover"));
    SideRail::new(id, items, cx.palette)
        .expand("Open Your Library")
        .create("Create a playlist")
        .current(current)
        .playing(format!("{id}-current=blue"))
}

/// The same library open: a header whose button folds it, and a row for each item.
fn open_library(cx: &Cx, fold: &str) -> Element {
    let palette = cx.palette;
    let button = IconButton::new(fold, Icon::PanelLeftClose, palette)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Dense)
        .tooltip_with(TooltipSpec::new("Collapse Your Library").placement(TooltipPlacement::Below))
        .render();
    let rows = LIBRARY.iter().map(|(key, title, subtitle, rgb, person)| {
        let radius = if *person { 20.0 } else { RadiusRole::Control.px() };
        ListRow::new(format!("{fold}-{key}"), *title, palette)
            .subtitle(*subtitle)
            .leading(cover(*rgb).size(Some(40.0), Some(40.0)).radius(radius))
            .render()
    });
    let mut children = vec![PanelHeader::new("Your Library", palette).trailing(vec![button]).render()];
    children.extend(rows);
    SidebarPanel::new(children, palette).size(280.0, FRAME_HEIGHT).render()
}

/// A window's main area: what stands beside the side panels.
fn main_area(cx: &Cx, text: &str) -> Element {
    Element::panel(caption(cx, text).centered()).background(cx.palette.raised).flex(1.0)
}

/// A queue, as the right panel shows it open, with these tools in its header.
fn queue(cx: &Cx, prefix: &str, tools: Vec<Element>) -> Element {
    let palette = cx.palette;
    let rows = LIBRARY.iter().map(|(key, title, subtitle, rgb, _)| {
        ListRow::new(format!("{prefix}-{key}"), *title, palette)
            .subtitle(*subtitle)
            .leading(cover(*rgb).size(Some(40.0), Some(40.0)).radius(RadiusRole::Control.px()))
            .render()
    });
    let mut children = vec![PanelHeader::new("Queue", palette).trailing(tools).render()];
    children.extend(rows);
    SidebarPanel::new(children, palette).render()
}

/// A rail at `height`: the rail sets its own width.
fn rail_at(rail: SideRail, height: f32) -> Element {
    Element::column(vec![rail.render()]).size(Some(SideRail::WIDTH), Some(height))
}

/// A window of the gallery's width and a fixed height, clipped at its edges.
fn window(cx: &Cx, children: Vec<Element>) -> Element {
    Element::row(children)
        .gap(8.0)
        .size(Some(cx.width.min(720.0)), Some(FRAME_HEIGHT))
        .radius(RadiusRole::Surface.px())
        .clip_children()
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let unfolded = cx.state.flag("side-rail:expand");
    let library = if unfolded { open_library(cx, "side-rail:expand") } else { rail(cx, "side-rail", 5).render() };
    let peek_open = cx.state.flag("side-peek");
    let right = if peek_open {
        let close = IconButton::new("side-peek", Icon::PanelRightClose, cx.palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .tooltip_with(TooltipSpec::new("Hide the queue").placement(TooltipPlacement::Below))
            .render();
        queue(cx, "side-queue-open", vec![close]).size(Some(280.0), None)
    } else {
        PeekPanel::new("side-peek", queue(cx, "side-queue-peek", Vec::new()), cx.palette)
            .content_width(280.0)
            .accessibility_label("Show the queue")
            .render()
    };
    vec![
        section(cx, "Folded library", vec![
            caption(cx, "A rail of pictures where the library was. Its hints open at once at their side; the button at its head unfolds it."),
            window(cx, vec![library, main_area(cx, "Main view")]),
            caption(cx, "current(id) keeps the page shown filled; playing(id) marks what plays. Folded and open side by side:"),
            row(vec![rail_at(rail(cx, "side-both", 5), FRAME_HEIGHT), open_library(cx, "side-both-fold")]),
        ]),
        section(cx, "A long rail", vec![
            caption(cx, "More items than fit: they scroll under the head."),
            row(vec![rail_at(rail(cx, "side-long", 24), 240.0)]),
        ]),
        section(cx, "Peek at the right edge", vec![
            caption(cx, "The closed queue leaves a strip: hover it to see it reach out, click it to open the queue."),
            window(cx, vec![main_area(cx, "Main view"), right]),
        ]),
        section(cx, "Peek at the left edge", vec![
            caption(cx, "On the left the strip reaches over what comes after it, so it is stacked over the window's row."),
            window(cx, vec![Element::stack(vec![
                Element::row(vec![Element::spacer().size(Some(40.0), None), main_area(cx, "Main view")]).gap(8.0),
                Element::row(vec![
                    PeekPanel::new("side-peek-left", queue(cx, "side-queue-left", Vec::new()), cx.palette)
                        .side(Side::Left)
                        .content_width(280.0)
                        .accessibility_label("Show the library")
                        .render(),
                    Element::spacer().flex(1.0),
                ]),
            ])
            .flex(1.0)]),
        ]),
    ]
}
