use super::{caption, section, Cx, Section};
use limn::{
    guides_for_depths, AlertColors, AlertTone, Button, ButtonSize, ButtonVariant, Component, ControlSize, Element,
    FontFamily, FramedCard, Icon, IconButton, ListRow, RadiusRole, ScrollArea, ScrollMotion, ScrollbarAppearance,
    Separator, TreeGuides, TreeRow,
};

/// The words the measured list's messages are made of.
const WORDS: [&str; 12] = [
    "the", "list", "measures", "each", "row", "as", "it", "comes", "into", "view", "and", "keeps",
];

/// A project as its rows in depth-first order: name, depth, and whether it is a folder.
const TREE: [(&str, usize, bool); 11] = [
    ("limn", 0, true),
    ("src", 1, true),
    ("components", 2, true),
    ("button.rs", 3, false),
    ("slider.rs", 3, false),
    ("lib.rs", 2, false),
    ("tree_view.rs", 2, false),
    ("docs", 1, true),
    ("components.md", 2, false),
    ("Cargo.toml", 1, false),
    ("README.md", 0, false),
];

fn label(cx: &Cx, text: &str, size: f32) -> Element {
    Element::truncated_text(text).font_family(FontFamily::Sans).font_size(size).foreground(cx.palette.text)
}

/// The project above: a click folds a folder or selects a file.
fn tree(cx: &Cx, width: f32) -> Element {
    let palette = cx.palette;
    let folded = |name: &str| cx.state.flag(&format!("surfaces-fold-{name}"));
    let selected = cx.state.choice("surfaces-tree", "lib.rs");
    // What is under a folded folder is not a row, so it draws no guide either.
    let mut visible = Vec::new();
    let mut hidden_below = None;
    for (index, &(name, depth, folder)) in TREE.iter().enumerate() {
        if hidden_below.is_some_and(|limit| depth > limit) {
            continue;
        }
        hidden_below = (folder && folded(name)).then_some(depth);
        visible.push(index);
    }
    let depths: Vec<usize> = visible.iter().map(|&index| TREE[index].1).collect();
    let rows = visible
        .into_iter()
        .zip(guides_for_depths(&depths))
        .map(|(index, guides)| {
            let (name, depth, folder) = TREE[index];
            let title = label(cx, name, 12.0);
            if folder {
                let open = !folded(name);
                let held = TREE[index + 1..]
                    .iter()
                    .take_while(|entry| entry.1 > depth)
                    .filter(|entry| entry.1 == depth + 1)
                    .count();
                let id = format!("surfaces-fold-{name}");
                TreeRow::new(
                    move |content| Element::clickable(id, content),
                    Element::icon(limn::folder_icon(name, open)).foreground(palette.muted),
                    title,
                    guides,
                    palette,
                )
                .count(held)
                .disclosure(open)
                .width(width)
                .render()
            } else {
                let id = format!("surfaces-tree={name}");
                let row = TreeRow::new(
                    move |content| Element::clickable(id, content),
                    Element::icon(limn::file_icon(name)).foreground(palette.muted),
                    title,
                    guides,
                    palette,
                )
                .selected(name == selected)
                .width(width);
                // The host's own mark, here a file with changes.
                if name == "tree_view.rs" {
                    row.trailing(Element::text("M").font_family(FontFamily::Sans).font_size(11.0).foreground(palette.muted))
                } else {
                    row
                }
                .render()
            }
        })
        .collect();
    Element::column(rows).gap(0.0)
}

/// One row on its own, at a depth the caller draws by hand.
fn lone_row(cx: &Cx, id: &'static str, title: &str, guides: TreeGuides, width: f32) -> TreeRow<impl FnOnce(Element) -> Element> {
    TreeRow::new(
        move |content| Element::clickable(id, content),
        Element::icon(Icon::File).foreground(cx.palette.muted),
        label(cx, title, 12.0),
        guides,
        cx.palette,
    )
    .width(width)
}

fn framed_cards(cx: &Cx, width: f32) -> Vec<Element> {
    let palette = cx.palette;
    let ink = |text: &str, size: f32, color| Element::truncated_text(text).font_family(FontFamily::Sans).font_size(size).foreground(color);
    let glyph = |icon: Icon| Element::icon(icon).foreground(palette.muted).size(Some(14.0), Some(14.0)).centered_y();
    let line = |id: &str, mark: Icon, name: &str, trailing: Element| {
        FramedCard::row(
            format!("{id}:fill"),
            Element::clickable(
                id,
                Element::row(vec![glyph(mark), ink(name, 13.0, palette.text).flex(1.0).centered_y(), trailing]).gap(10.0),
            ),
            [id],
            palette,
        )
    };
    let value = |text: &str| ink(text, 12.0, palette.muted).centered_y();
    let header = |title: &str| ink(title, 13.0, palette.text).size(None, Some(28.0));
    let foot = |id: &str| {
        Element::row(vec![
            ink("Deployed 2 minutes ago", 12.0, palette.muted).flex(1.0).centered_y(),
            Button::new(id, "Deploy", palette).variant(ButtonVariant::Outline).button_size(ButtonSize::Small).render(),
        ])
        .gap(8.0)
    };

    let open = cx.state.flag("surfaces-card-variables");
    let listed = open.then(|| {
        Element::column(
            ["DATABASE_URL", "REGION", "LOG_LEVEL"]
                .into_iter()
                .map(|name| {
                    Element::row(vec![
                        Element::spacer().size(Some(FramedCard::REACH + 24.0), None),
                        ink(name, 12.0, palette.muted).flex(1.0).centered_y(),
                    ])
                    .size(None, Some(24.0))
                })
                .chain([Element::spacer().size(None, Some(4.0))])
                .collect(),
        )
    });
    let card = FramedCard::new(palette)
        .width(width)
        .section(header("Production"))
        .rows(line("surfaces-card-branch", Icon::GitBranch, "Branch", value("main")))
        // Two controls on one row share its fill.
        .rows(FramedCard::row(
            "surfaces-card-region:fill",
            Element::row(vec![
                Element::clickable(
                    "surfaces-card-region",
                    Element::row(vec![glyph(Icon::Laptop), ink("Region", 13.0, palette.text).flex(1.0).centered_y(), value("eu-west")])
                        .gap(10.0),
                )
                .flex(1.0),
                IconButton::new("surfaces-card-region-more", Icon::More, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Dense)
                    .render()
                    .centered_y(),
            ])
            .gap(4.0),
            ["surfaces-card-region", "surfaces-card-region-more"],
            palette,
        ))
        .space(6.0)
        .rows(FramedCard::block(
            "surfaces-card-variables-list",
            line(
                "surfaces-card-variables",
                Icon::Book,
                "Variables",
                glyph(if open { Icon::ChevronDown } else { Icon::ChevronRight }),
            ),
            listed,
            palette,
        ))
        .foot(foot("surfaces-card-deploy"))
        .render();

    // The host's surface is the card: it scrolls the body and keeps the foot.
    let (body, pinned) = (0..8)
        .fold(FramedCard::new(palette).flat().section(header("Deployments")), |card, index| {
            card.rows(line(
                &format!("surfaces-flat-{index}"),
                Icon::GitBranch,
                &format!("Deploy {}", 48 - index),
                value(&format!("{} h ago", index * 3 + 1)),
            ))
        })
        .foot(foot("surfaces-flat-deploy"))
        .into_parts();
    let mut parts = vec![
        ScrollArea::new("surfaces-flat-scroll", body)
            // Over the rows, so they keep the card's insets.
            .appearance(ScrollbarAppearance::overlay())
            .render()
            .size(None, Some(140.0)),
        Separator::horizontal(palette).render(),
    ];
    parts.extend(pinned);
    let flat = Element::panel(Element::column(parts))
        .background(palette.surface)
        .border(palette.border, 1.0)
        .radius(RadiusRole::Panel.px())
        .clip_children()
        .size(Some(width), None);

    vec![
        caption(cx, "Sections keep to the inset; a row's fill reaches past it, and the foot sits on the frame under the card. The second row holds two controls that share one fill; the last opens a list in place."),
        card,
        caption(cx, "flat(): no frame and no card, on a surface of the host's. into_parts() gives the body and the foot apart, so the host scrolls one and pins the other."),
        flat,
        caption(cx, "Without a width the card fills the space it is given, and without a foot the frame closes under it."),
        FramedCard::new(palette)
            .section(header("Preview"))
            .section(ink("No deployment yet.", 12.0, palette.muted).size(None, Some(24.0)))
            .render()
            .size(Some(cx.width.min(480.0)), None),
    ]
}

fn scroll_areas(cx: &Cx, width: f32) -> Vec<Element> {
    let palette = cx.palette;
    let card = |content: Element| {
        Element::panel(content).padding(8.0).background(palette.surface).radius(RadiusRole::Card.px()).size(Some(width), None)
    };
    let rows = |id: &str, count: usize| {
        Element::column(
            (0..count)
                .map(|index| ListRow::new(format!("{id}-{index}"), format!("Row {}", index + 1), palette).render())
                .collect(),
        )
        .gap(2.0)
    };
    let open: Option<usize> = cx.state.text("surfaces-list-open").parse().ok();
    let list = ScrollArea::uniform_list("surfaces-list", 10_000, 28.0, move |index| {
        // The open row closes on a second click.
        let id = if open == Some(index) { "surfaces-list-open=none".to_owned() } else { format!("surfaces-list-open={index}") };
        ListRow::new(id, format!("Row {}", index + 1), palette).render()
    });
    let list = match open {
        Some(index) => list.expansion(
            index,
            Element::panel(caption(cx, &format!("What row {} opens. It scrolls with the rows, and the rows after it move down.", index + 1)))
                .padding(8.0),
        ),
        None => list,
    };
    let messages: usize = cx.state.text("surfaces-measured").parse().unwrap_or(2_000);
    let measured = ScrollArea::measured_list(
        "surfaces-measured-list",
        messages,
        44.0,
        |index| index as u64,
        move |index| {
            // Messages of one to five lines, so no two neighbours agree.
            let words = 3 + index * 37 % 41;
            let body: Vec<&str> = (0..words).map(|word| WORDS[(index + word) % WORDS.len()]).collect();
            Element::column(vec![
                Element::text(format!("Message {}", index + 1))
                    .font_family(FontFamily::SansMedium)
                    .font_size(12.0)
                    .foreground(palette.text),
                Element::text(body.join(" ")).font_family(FontFamily::Sans).font_size(12.0).foreground(palette.muted),
            ])
            .gap(2.0)
            .padding(6.0)
        },
    )
    .follow_end(true);
    vec![
        caption(cx, "The default: while the content overflows, a rail beside it with a thumb and a button at each end."),
        card(ScrollArea::new("surfaces-scroll", rows("surfaces-scroll", 24)).render().size(None, Some(150.0))),
        caption(cx, "ScrollbarAppearance::overlay(): the thumb is drawn over the content, which keeps its width."),
        card(
            ScrollArea::new("surfaces-scroll-overlay", rows("surfaces-scroll-overlay", 24))
                .appearance(ScrollbarAppearance::overlay())
                .render()
                .size(None, Some(150.0)),
        ),
        caption(cx, "No buttons, a tint, a wider rail and thumb, and wheel steps that land at once."),
        card(
            ScrollArea::new("surfaces-scroll-custom", rows("surfaces-scroll-custom", 24))
                .appearance(ScrollbarAppearance::reserved().rail_width(20.0).thumb_width(10.0))
                .buttons(false)
                .tint(AlertColors::for_tone(AlertTone::Info, palette).accent)
                .motion(ScrollMotion { duration_ms: 0, ..ScrollMotion::default() })
                .render()
                .size(None, Some(150.0)),
        ),
        caption(cx, "Content that fits: no scrollbar and no rail."),
        card(ScrollArea::new("surfaces-scroll-short", rows("surfaces-scroll-short", 3)).render().size(None, Some(150.0))),
        caption(cx, "uniform_list: 10,000 rows of one height, built only while in view. Click a row to open something under it."),
        card(list.render().size(None, Some(180.0))),
        caption(cx, "measured_list: messages of their own heights, measured as they come into view, following the end as new ones arrive. Scroll up and add one: the view holds its place."),
        card(Element::column(vec![
            measured.render().size(None, Some(220.0)),
            Element::row(vec![
                Button::new(format!("surfaces-measured={}", messages + 1), "Add a message", palette)
                    .variant(ButtonVariant::Outline)
                    .size(ControlSize::Compact)
                    .render(),
                Element::spacer().flex(1.0),
            ]),
        ])
        .gap(8.0)),
    ]
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let width = cx.width.min(320.0);
    let accent = AlertColors::for_tone(AlertTone::Info, cx.palette).accent;
    let deep = |continues: bool| TreeGuides { depth: 3, ancestor_continuations: vec![true, true], next_sibling: continues };
    vec![
        section(cx, "Tree rows", vec![
            caption(cx, "A folder shows how many items it holds and a chevron; click it to fold it. Click a file to select it. The guides are computed from the depths of the visible rows."),
            tree(cx, width),
            caption(cx, "A taller row, a guide color, and a row three levels deep that draws only its nearest lane with max_guide_depth(1)."),
            Element::column(vec![
                lone_row(cx, "surfaces-row-tall", "height(36.0)", TreeGuides::root(), width).height(36.0).render(),
                lone_row(cx, "surfaces-row-ink", "guide_color(accent)", deep(true), width).guide_color(accent).render(),
                lone_row(cx, "surfaces-row-shallow", "max_guide_depth(1)", deep(false), width).max_guide_depth(1).render(),
            ])
            .gap(0.0),
        ]),
        section(cx, "Framed card", framed_cards(cx, width)),
        section(cx, "Scroll area", scroll_areas(cx, width)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::State;
    use limn::{ComponentPalette, Size, UiTree};

    #[test]
    fn folding_a_folder_hides_what_is_under_it_and_nothing_else() {
        let rows = |state: &State| {
            let cx = Cx { state, palette: ComponentPalette::neutral_dark(), width: 320.0 };
            let frame = UiTree::new(Element::row(vec![tree(&cx, 320.0)])).layout(Size::new(320.0, 600.0));
            frame.regions.iter().map(|region| region.id.clone()).collect::<Vec<_>>()
        };
        let mut state = State::default();
        assert_eq!(rows(&state).len(), TREE.len());
        state.set("surfaces-fold-src", "on");
        let folded = rows(&state);
        assert_eq!(folded.len(), TREE.len() - 5);
        assert!(!folded.contains(&"surfaces-tree=lib.rs".to_owned()) && folded.contains(&"surfaces-tree=components.md".to_owned()));
    }
}
