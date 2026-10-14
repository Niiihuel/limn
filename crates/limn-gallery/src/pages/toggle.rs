use super::{caption, row, section, Cx, Section};
use limn::{Component, ControlSize, Icon, Toggle, ToggleGroup, ToggleItem, ToggleVariant};

/// Whether a toggle of the page is pressed: a click flips its flag, and
/// `initially` is what it shows before any click.
fn pressed(cx: &Cx, id: &str, initially: bool) -> bool {
    cx.state.flag(id) != initially
}

/// The words for what is pressed among `names`, for a caption.
fn listed(cx: &Cx, names: &[(&str, &str, bool)]) -> String {
    let on: Vec<&str> =
        names.iter().filter(|(id, _, initially)| pressed(cx, id, *initially)).map(|(_, name, _)| *name).collect();
    if on.is_empty() {
        "Pressed: none".to_owned()
    } else {
        format!("Pressed: {}", on.join(", "))
    }
}

const STYLES: [(&str, &str, bool); 3] =
    [("toggle-bold", "Bold", true), ("toggle-italic", "Italic", false), ("toggle-underline", "Underline", true)];

/// Bold, italic and underline as one group of one-letter toggles; `prefix`
/// keeps the IDs of one demo apart from another's.
fn styles(cx: &Cx, prefix: &str, size: ControlSize) -> ToggleGroup {
    let items = STYLES
        .iter()
        .map(|(id, name, initially)| {
            let id = format!("{prefix}{id}");
            ToggleItem::new(id.clone(), &name[..1], pressed(cx, &id, *initially))
                .width(size.height())
                .accessibility_label(*name)
                .tooltip(*name)
        })
        .collect();
    ToggleGroup::new(format!("{prefix}toggle-styles"), items, cx.palette).size(size).accessibility_label("Text style")
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let sizes = [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious];
    vec![
        section(cx, "Toggle", vec![
            row(vec![
                Toggle::new("toggle-wrap", "Wrap lines", pressed(cx, "toggle-wrap", true), palette).render(),
                Toggle::new("toggle-filter", "Unread only", pressed(cx, "toggle-filter", false), palette)
                    .icon(Icon::Filter)
                    .render(),
                Toggle::icon_only("toggle-pin", Icon::Pin, pressed(cx, "toggle-pin", false), palette)
                    .tooltip("Pin the panel")
                    .render(),
            ]),
            caption(cx, &listed(cx, &[
                ("toggle-wrap", "Wrap lines", true),
                ("toggle-filter", "Unread only", false),
                ("toggle-pin", "Pin the panel", false),
            ])),
            caption(cx, "A label, an icon before a label, and an icon alone, which its hint names."),
        ]),
        section(cx, "Variants", vec![
            row(vec![
                Toggle::new("toggle-ghost", "Ghost", pressed(cx, "toggle-ghost", false), palette).render(),
                Toggle::new("toggle-ghost-on", "Ghost", pressed(cx, "toggle-ghost-on", true), palette).render(),
                Toggle::new("toggle-outline", "Outline", pressed(cx, "toggle-outline", false), palette)
                    .variant(ToggleVariant::Outline)
                    .render(),
                Toggle::new("toggle-outline-on", "Outline", pressed(cx, "toggle-outline-on", true), palette)
                    .variant(ToggleVariant::Outline)
                    .render(),
            ]),
            caption(cx, "Each at rest and pressed. Ghost is for a toolbar; Outline stands alone on a page."),
        ]),
        section(cx, "Sizes", vec![
            row(sizes
                .iter()
                .map(|size| {
                    let id = format!("toggle-size-{}", size.height());
                    Toggle::new(id.clone(), format!("{} px", size.height()), pressed(cx, &id, true), palette)
                        .icon(Icon::Bell)
                        .variant(ToggleVariant::Outline)
                        .size(*size)
                        .render()
                })
                .collect()),
            row(sizes
                .iter()
                .map(|size| {
                    let id = format!("toggle-icon-size-{}", size.height());
                    Toggle::icon_only(id.clone(), Icon::Heart, pressed(cx, &id, false), palette)
                        .variant(ToggleVariant::Outline)
                        .size(*size)
                        .accessibility_label(format!("Favorite, {} pixels", size.height()))
                        .render()
                })
                .collect()),
            caption(cx, "Dense, Compact, Regular and Spacious."),
        ]),
        section(cx, "Toggle group", vec![
            row(vec![styles(cx, "", ControlSize::Regular).render()]),
            caption(cx, &listed(cx, &STYLES)),
            row(vec![ToggleGroup::new(
                "toggle-show",
                vec![
                    ToggleItem::new("toggle-show-files", "Files", pressed(cx, "toggle-show-files", true)).icon(Icon::Folder),
                    ToggleItem::new("toggle-show-terminal", "Terminal", pressed(cx, "toggle-show-terminal", false))
                        .icon(Icon::Terminal),
                    ToggleItem::new("toggle-show-chat", "Chat", pressed(cx, "toggle-show-chat", true)).icon(Icon::Chat),
                ],
                palette,
            )
            .accessibility_label("Panels shown")
            .render()]),
            caption(cx, "Any number may be pressed at once, and each toggle reports its own ID."),
            row(sizes.iter().map(|size| styles(cx, &format!("s{}-", size.height()), *size).render()).collect()),
            caption(cx, "The four sizes: the height is the group's, outline included."),
        ]),
        section(cx, "Disabled", vec![
            row(vec![
                Toggle::new("toggle-off-rest", "Wrap lines", false, palette).disabled().render(),
                Toggle::new("toggle-off-pressed", "Wrap lines", true, palette).disabled().render(),
                Toggle::new("toggle-off-outline", "Unread only", true, palette)
                    .icon(Icon::Filter)
                    .variant(ToggleVariant::Outline)
                    .disabled()
                    .render(),
                Toggle::icon_only("toggle-off-pin", Icon::Pin, true, palette).tooltip("Pin the panel").disabled().render(),
            ]),
            caption(cx, "A disabled toggle still shows whether it is pressed, and keeps its hint."),
            row(vec![
                styles(cx, "off-", ControlSize::Regular).disabled().render(),
                ToggleGroup::new(
                    "toggle-one-off",
                    vec![
                        ToggleItem::new("toggle-one-a", "Files", pressed(cx, "toggle-one-a", true)),
                        ToggleItem::new("toggle-one-b", "Terminal", true).disabled(),
                        ToggleItem::new("toggle-one-c", "Chat", pressed(cx, "toggle-one-c", false)),
                    ],
                    palette,
                )
                .accessibility_label("Panels shown, one locked")
                .render(),
            ]),
            caption(cx, "A whole group disabled, and a group with only its middle toggle disabled."),
        ]),
        section(cx, "Edge cases", vec![
            row(vec![
                ToggleGroup::new(
                    "toggle-single",
                    vec![ToggleItem::icon_only("toggle-single-a", Icon::Grid, pressed(cx, "toggle-single-a", true))
                        .tooltip("Grid")],
                    palette,
                )
                .render(),
                Toggle::new(
                    "toggle-long",
                    "Include archived conversations",
                    pressed(cx, "toggle-long", false),
                    palette,
                )
                .variant(ToggleVariant::Outline)
                .render(),
                Toggle::new("toggle-fixed", "Include archived conversations", pressed(cx, "toggle-fixed", true), palette)
                    .variant(ToggleVariant::Outline)
                    .width(140.0)
                    .render(),
            ]),
            caption(cx, "A group of one keeps its four corners. A long label widens the toggle, or is cut at a fixed width."),
        ]),
    ]
}
