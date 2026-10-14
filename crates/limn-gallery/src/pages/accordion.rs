use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{
    Accordion, AccordionItem, Button, ButtonVariant, Component, ControlSize, Element, FontFamily, Icon, Switch,
    UiAction,
};

/// The items of the demo in which one is open at a time: an ID, a title, the
/// text inside and whether it is open before any click.
const ONE: [(&str, &str, &str, bool); 3] = [
    (
        "accordion-one-shipping",
        "How long does shipping take?",
        "Orders leave the warehouse within two working days and arrive in three to five more.",
        true,
    ),
    (
        "accordion-one-returns",
        "Can I return an order?",
        "Within thirty days of delivery, unused and in its box. The refund goes to the card that paid.",
        false,
    ),
    (
        "accordion-one-invoice",
        "Where is my invoice?",
        "It is sent to the address of the account when the order ships, and stays under Orders.",
        false,
    ),
];

/// Whether an item of the page is open: a click flips its flag, and
/// `initially` is what it shows before any click.
fn open(state: &State, id: &str, initially: bool) -> bool {
    state.flag(id) != initially
}

/// One open at a time: a click on an item of that demo closes the others, and
/// the click itself then flips the one it was on, as it does every flag.
pub fn update(state: &mut State, action: &UiAction) {
    let UiAction::Clicked { id } = action else { return };
    if ONE.iter().any(|(item, ..)| item == id) {
        for (other, _, _, initially) in ONE.iter().filter(|(item, ..)| item != id) {
            state.set(other, if *initially { "on" } else { "off" });
        }
    }
}

/// A paragraph of content, as an item of a list of questions holds.
fn words(cx: &Cx, text: &str) -> Element {
    Element::text(text).font_family(FontFamily::Sans).font_size(13.0).foreground(cx.palette.muted)
}

/// Three live items named after `prefix`, the first open before any click.
fn three(cx: &Cx, prefix: &str) -> Vec<AccordionItem> {
    [("General", true), ("Appearance", false), ("Advanced", false)]
        .into_iter()
        .map(|(title, initially)| {
            let id = format!("{prefix}-{}", title.to_lowercase());
            let text = format!("The {} settings of the workspace.", title.to_lowercase());
            AccordionItem::new(id.clone(), title, words(cx, &text)).open(open(cx.state, &id, initially))
        })
        .collect()
}

/// An accordion at the width of a column of text, or the page's when narrower.
fn column(cx: &Cx, accordion: Accordion) -> Element {
    row(vec![accordion.render().size(Some(cx.width.min(520.0)), None)])
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let one = ONE
        .iter()
        .map(|(id, title, text, initially)| {
            AccordionItem::new(*id, *title, words(cx, text)).open(open(state, id, *initially))
        })
        .collect();
    let sizes = [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious];
    let wifi = "accordion-form-wifi";
    vec![
        section(cx, "Accordion", vec![
            column(cx, Accordion::new("accordion-one", one, palette).accessibility_label("Questions about orders")),
            caption(cx, "One open at a time: the host closes the others when a header reports its click."),
        ]),
        section(cx, "Several open", vec![
            column(cx, Accordion::new("accordion-many", three(cx, "accordion-many"), palette)
                .accessibility_label("Settings, several open")),
            caption(cx, "Each item is open or closed by itself: the host flips only the one that was clicked."),
        ]),
        section(cx, "Bordered", vec![
            column(cx, Accordion::new("accordion-bordered", three(cx, "accordion-bordered"), palette)
                .bordered(true)
                .accessibility_label("Settings, bordered")),
            caption(cx, "One outlined surface with hairlines between the items, against the plain list above."),
        ]),
        section(cx, "Icon and description", vec![
            column(cx, Accordion::new(
                "accordion-icons",
                vec![
                    AccordionItem::new("accordion-icons-alerts", "Notifications", words(cx, "Email and push, by channel."))
                        .icon(Icon::Bell)
                        .description("3 channels on")
                        .open(open(state, "accordion-icons-alerts", false)),
                    AccordionItem::new("accordion-icons-files", "Files", words(cx, "Where downloads and exports are kept."))
                        .icon(Icon::Folder)
                        .description("2 folders")
                        .open(open(state, "accordion-icons-files", true)),
                    AccordionItem::new("accordion-icons-shell", "Terminal", words(cx, "The shell and its font."))
                        .icon(Icon::Terminal)
                        .open(open(state, "accordion-icons-shell", false)),
                ],
                palette,
            )
            .bordered(true)
            .accessibility_label("Settings with icons")),
            caption(cx, "An icon before the title and a muted description beside it."),
        ]),
        section(cx, "Sizes", sizes
            .iter()
            .map(|size| {
                let prefix = format!("accordion-size-{}", size.height());
                let items = ["First", "Second"]
                    .into_iter()
                    .map(|title| {
                        let id = format!("{prefix}-{}", title.to_lowercase());
                        AccordionItem::new(id.clone(), format!("{title}, {} px", size.height() + 8.0), words(cx, "Content."))
                            .open(open(state, &id, false))
                    })
                    .collect();
                column(cx, Accordion::new(prefix.clone(), items, palette)
                    .size(*size)
                    .bordered(true)
                    .accessibility_label(format!("Size {}", size.height())))
            })
            .chain([caption(cx, "Dense, Compact, Regular and Spacious: the header's height and its text.")])
            .collect()),
        section(cx, "Disabled", vec![
            column(cx, Accordion::new(
                "accordion-locked",
                vec![
                    AccordionItem::new("accordion-locked-plan", "Plan", words(cx, "Team, billed yearly."))
                        .open(open(state, "accordion-locked-plan", false)),
                    AccordionItem::new("accordion-locked-owner", "Owner", words(cx, "Only the owner can change this."))
                        .description("Locked")
                        .open(true)
                        .disabled(),
                    AccordionItem::new("accordion-locked-region", "Region", words(cx, "Set when the workspace was made."))
                        .disabled(),
                ],
                palette,
            )
            .bordered(true)
            .accessibility_label("Workspace, two items locked")),
            caption(cx, "A disabled item stays open or closed as the host says; its header takes no click and no focus."),
            column(cx, Accordion::new("accordion-off", three(cx, "accordion-off"), palette)
                .bordered(true)
                .disabled()
                .accessibility_label("Settings, disabled")),
            caption(cx, "A whole accordion disabled."),
        ]),
        section(cx, "Edge cases", vec![
            column(cx, Accordion::new(
                "accordion-form",
                vec![AccordionItem::new(
                    "accordion-form-network",
                    "Network",
                    Element::column(vec![
                        row(vec![
                            Switch::new(wifi, state.flag(wifi), palette).accessibility_label("Use Wi-Fi").render(),
                            words(cx, "Use Wi-Fi").centered_y(),
                        ]),
                        row(vec![Button::new("accordion-form-reset", "Reset network settings", palette)
                            .variant(ButtonVariant::Outline)
                            .render()]),
                    ])
                    .gap(12.0),
                )
                .open(open(state, "accordion-form-network", true))],
                palette,
            )
            .bordered(true)
            .accessibility_label("Network")),
            caption(cx, "Controls inside an item are reached by Tab only while it is open."),
            row(vec![Accordion::new(
                "accordion-narrow",
                vec![
                    AccordionItem::new(
                        "accordion-narrow-long",
                        "A title that is much longer than the width it has been given",
                        words(cx, "The title is cut with an ellipsis and the chevron keeps its place."),
                    )
                    .icon(Icon::File)
                    .description("And a description")
                    .open(open(state, "accordion-narrow-long", false)),
                    AccordionItem::new(
                        "accordion-narrow-nested",
                        "Nested",
                        Accordion::new(
                            "accordion-inner",
                            vec![
                                AccordionItem::new("accordion-inner-a", "Child one", words(cx, "Content."))
                                    .open(open(state, "accordion-inner-a", false)),
                                AccordionItem::new("accordion-inner-b", "Child two", words(cx, "Content."))
                                    .open(open(state, "accordion-inner-b", false)),
                            ],
                            palette,
                        )
                        .size(ControlSize::Dense)
                        .render(),
                    )
                    .open(open(state, "accordion-narrow-nested", true)),
                ],
                palette,
            )
            .bordered(true)
            .accessibility_label("Narrow")
            .render()
            .size(Some(cx.width.min(260.0)), None)]),
            caption(cx, "In 260 pixels a long title is cut. An item's content can be another accordion."),
        ]),
    ]
}
