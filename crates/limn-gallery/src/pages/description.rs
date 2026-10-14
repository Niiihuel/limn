use super::{caption, section, Cx, Section};
use limn::{Chip, Clipboard, Component, ControlSize, DescriptionItem, DescriptionList, Link};

const LONG: &str = "Serves the public API and the nightly exports. It is restarted by the deploy job, never by hand, \
                    and keeps its logs for thirty days.";

/// The details of one server, as text.
fn server() -> Vec<DescriptionItem> {
    vec![
        DescriptionItem::new("Name", "atlas-01"),
        DescriptionItem::new("Region", "eu-west-3"),
        DescriptionItem::new("Memory", "16 GB"),
        DescriptionItem::new("Uptime", "41 days"),
    ]
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let width = cx.width.min(560.0);
    let wide = cx.width.min(760.0);
    let list = |items: Vec<DescriptionItem>| DescriptionList::new(items, palette).width(width);
    let opened = if state.flag("description-repo") { "opened" } else { "not opened" };
    vec![
        section(cx, "DescriptionList", vec![
            list(vec![
                DescriptionItem::new("Order", "#48213"),
                DescriptionItem::with("Status", Chip::new("Shipped", palette).render()),
                DescriptionItem::new("Total", "1,204.50 EUR"),
                DescriptionItem::with("Invoice", Link::new("description-repo", "INV-2026-0931", palette).render()),
                DescriptionItem::with(
                    "Tracking",
                    Clipboard::new("description-copy", "Copy tracking number", palette)
                        .label("LX 4821 7730 ES")
                        .size(ControlSize::Dense)
                        .copied(state.flag("description-copy"))
                        .render(),
                ),
            ])
            .render(),
            caption(cx, &format!(
                "A value is text or an element. The list takes no click; the link and the copy button keep theirs. Invoice: {opened}."
            )),
        ]),
        section(cx, "Stacked", vec![
            list(server()).stacked().render(),
            caption(cx, "The label over its value, for long values or a narrow pane."),
        ]),
        section(cx, "Columns", vec![
            DescriptionList::new(
                vec![
                    DescriptionItem::new("Name", "atlas-01"),
                    DescriptionItem::new("Region", "eu-west-3"),
                    DescriptionItem::new("Memory", "16 GB"),
                    DescriptionItem::new("Role", LONG).span(3),
                    DescriptionItem::new("Image", "debian-12-2026.09").span(2),
                    DescriptionItem::new("Uptime", "41 days"),
                ],
                palette,
            )
            .columns(3)
            .label_width(64.0)
            .width(wide)
            .render(),
            caption(cx, "Three columns of equal width: Role spans three and Image two."),
            DescriptionList::new(server().into_iter().chain([DescriptionItem::new("Kernel", "6.12")]).collect(), palette)
                .columns(2)
                .stacked()
                .width(wide)
                .render(),
            caption(cx, "Two columns, stacked. The last pair of a row takes the columns the row has left."),
        ]),
        section(cx, "Bordered", vec![
            list(server()).bordered(true).render(),
            DescriptionList::new(
                vec![
                    DescriptionItem::new("Name", "Ada Lovelace"),
                    DescriptionItem::new("Team", "Engines"),
                    DescriptionItem::with("Email", Link::new("description-mail", "ada@example.com", palette).render()),
                    DescriptionItem::with("State", Chip::new("Active", palette).outline().render()),
                    DescriptionItem::new("Notes", LONG).span(2),
                ],
                palette,
            )
            .bordered(true)
            .columns(2)
            .label_width(72.0)
            .width(wide)
            .render(),
            list(server()).bordered(true).stacked().columns(2).render(),
            caption(cx, "An outlined surface with a rule between rows, between columns and between a label and its value."),
        ]),
        section(cx, "Dividers", vec![
            list(server()).dividers(false).render(),
            caption(cx, "A plain list without the rules between its rows."),
        ]),
        section(cx, "Sizes", vec![
            list(server()).size(ControlSize::Dense).render(),
            caption(cx, "Dense: rows 24 high, text of 12."),
            list(server()).size(ControlSize::Compact).render(),
            caption(cx, "Compact: rows 28 high, text of 13."),
            list(server()).render(),
            caption(cx, "Regular, the default: rows 32 high, text of 13."),
            list(server()).size(ControlSize::Spacious).render(),
            caption(cx, "Spacious: rows 36 high, text of 14."),
        ]),
        section(cx, "Long text", vec![
            DescriptionList::new(
                vec![
                    DescriptionItem::new("Role", LONG),
                    DescriptionItem::new("A label too long for the column it is given", "eu-west-3"),
                    DescriptionItem::new("Memory", "16 GB"),
                ],
                palette,
            )
            .width(cx.width.min(320.0))
            .render(),
            caption(cx, "A long value wraps under itself; a long label is cut with an ellipsis."),
        ]),
    ]
}
