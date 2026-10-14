use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{
    Button, ButtonSize, ButtonVariant, Component, ControlSize, Element, Icon, Text, TextRole, Toolbar, ToolbarItem,
    UiAction,
};

/// The bar of the overflow demo, whose menu the page opens and closes.
const FIT: &str = "toolbar-fit";

/// Whether a toggle of the page is pressed: a click flips its flag, and
/// `initially` is what it shows before any click.
fn pressed(cx: &Cx, id: &str, initially: bool) -> bool {
    cx.state.flag(id) != initially
}

/// The trigger and the dismissal are two IDs, and each click flips one.
fn menu_open(state: &State) -> bool {
    state.flag(&format!("{FIT}:overflow_toggle")) != state.flag(&format!("{FIT}:overflow_dismiss"))
}

/// A command chosen in the overflow menu closes it, as a host does.
pub fn update(state: &mut State, action: &UiAction) {
    if matches!(action, UiAction::Clicked { id } if id.starts_with("toolbar-fit-")) && menu_open(state) {
        let dismiss = format!("{FIT}:overflow_dismiss");
        state.set(&dismiss, if state.flag(&dismiss) { "off" } else { "on" });
    }
}

/// File commands, text styles and, at the far end, the view: the bar most of
/// the page shows. `prefix` keeps the IDs of one demo apart from another's.
fn document(cx: &Cx, prefix: &str) -> Toolbar {
    let id = |name: &str| format!("{prefix}-{name}");
    let style = |name: &str, initially: bool| {
        ToolbarItem::toggle(id(name), &name[..1], pressed(cx, &id(name), initially))
            .accessibility_label(name)
            .tooltip(name)
    };
    Toolbar::new(prefix, cx.palette)
        .group(vec![
            ToolbarItem::button(id("new"), "New").icon(Icon::Add),
            ToolbarItem::icon_button(id("open"), Icon::Folder, "Open"),
            ToolbarItem::icon_button(id("copy"), Icon::Copy, "Copy"),
        ])
        .group(vec![style("Bold", true), style("Italic", false), style("Underline", false)])
        .spacer()
        .group(vec![
            ToolbarItem::icon_toggle(id("pin"), Icon::Pin, "Pin the panel", pressed(cx, &id("pin"), false)),
            ToolbarItem::icon_button(id("settings"), Icon::Settings, "Settings"),
        ])
        .accessibility_label("Document")
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let wide = cx.width.min(520.0);
    let sizes = [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious];
    let room = cx.state.choice("toolbar-room", "260").parse::<f32>().unwrap_or(260.0);
    let fitted = Toolbar::new(FIT, palette)
        .group(vec![
            ToolbarItem::icon_button("toolbar-fit-new", Icon::Add, "New").priority(2),
            ToolbarItem::icon_button("toolbar-fit-open", Icon::Folder, "Open"),
            ToolbarItem::icon_button("toolbar-fit-copy", Icon::Copy, "Copy"),
        ])
        .group(vec![
            ToolbarItem::icon_toggle("toolbar-fit-pin", Icon::Pin, "Pin the panel", pressed(cx, "toolbar-fit-pin", true)),
            ToolbarItem::icon_button("toolbar-fit-refresh", Icon::Refresh, "Refresh"),
            ToolbarItem::icon_button("toolbar-fit-trash", Icon::Trash, "Delete").disabled(),
        ])
        .group(vec![
            ToolbarItem::icon_button("toolbar-fit-search", Icon::Search, "Search").priority(1),
            ToolbarItem::icon_button("toolbar-fit-settings", Icon::Settings, "Settings"),
        ])
        .surface(true)
        .bordered(true)
        .width(room.min(cx.width))
        .overflow(menu_open(cx.state), "More")
        .accessibility_label("Files");
    let rooms = [("320", "Wide"), ("260", "Medium"), ("170", "Narrow"), ("90", "Tight")]
        .into_iter()
        .map(|(value, name)| {
            Button::new(format!("toolbar-room={value}"), name, palette)
                .variant(ButtonVariant::Outline)
                .button_size(ButtonSize::Small)
                .active(room.to_string() == value)
                .render()
        })
        .collect();
    let saved = Text::new("Saved", palette).role(TextRole::Small).muted().render();
    vec![
        section(cx, "Toolbar", vec![
            document(cx, "toolbar-main").surface(true).bordered(true).width(wide).render(),
            caption(cx, "Three groups with a rule between the first two, and a spacer that sends the last to the far end. Each control reports its own ID; the styles and the pin are toggles."),
        ]),
        section(cx, "On its container", vec![
            document(cx, "toolbar-plain").width(wide).render(),
            caption(cx, "Without surface and bordered the bar paints nothing of its own and is as high as its controls: for the header of a panel that already has a ground."),
        ]),
        section(cx, "Sizes", vec![
            Element::column(
                sizes
                    .iter()
                    .map(|size| {
                        document(cx, &format!("toolbar-size-{}", size.height()))
                            .size(*size)
                            .bordered(true)
                            .width(wide)
                            .render()
                    })
                    .collect(),
            )
            .gap(12.0),
            caption(cx, "Dense, Compact, Regular and Spacious: controls of 24, 28, 32 and 36 px, and 8 more for the bar that draws an outline."),
        ]),
        section(cx, "Vertical", vec![
            row(vec![
                Toolbar::new("toolbar-side", palette)
                    .group(vec![
                        ToolbarItem::icon_toggle("toolbar-side-files", Icon::File, "Files", pressed(cx, "toolbar-side-files", true)),
                        ToolbarItem::icon_toggle("toolbar-side-search", Icon::Search, "Search", pressed(cx, "toolbar-side-search", false)),
                        ToolbarItem::icon_toggle("toolbar-side-code", Icon::Code, "Source", pressed(cx, "toolbar-side-code", false)),
                    ])
                    .group(vec![ToolbarItem::icon_button("toolbar-side-settings", Icon::Settings, "Settings")])
                    .vertical()
                    .surface(true)
                    .bordered(true)
                    .accessibility_label("Views")
                    .render(),
                Toolbar::new("toolbar-side-plain", palette)
                    .group(vec![
                        ToolbarItem::icon_button("toolbar-side-add", Icon::Add, "New"),
                        ToolbarItem::icon_button("toolbar-side-edit", Icon::Edit, "Edit"),
                    ])
                    .group(vec![ToolbarItem::icon_button("toolbar-side-trash", Icon::Trash, "Delete")])
                    .vertical()
                    .size(ControlSize::Compact)
                    .accessibility_label("Tools")
                    .render(),
            ]),
            caption(cx, "Down a side the rule between groups lies flat. The second sits on its container at the compact size."),
        ]),
        section(cx, "Overflow", vec![
            row(rooms),
            fitted.render(),
            caption(cx, "With a width and overflow, what does not fit moves behind More, the lowest priority first: New stays longest, then Search. A group that leaves takes its rule with it."),
        ]),
        section(cx, "Disabled", vec![
            document(cx, "toolbar-disabled").bordered(true).width(wide).disabled().render(),
            row(vec![Toolbar::new("toolbar-partly", palette)
                .group(vec![
                    ToolbarItem::button("toolbar-partly-save", "Save").icon(Icon::Check),
                    ToolbarItem::button("toolbar-partly-undo", "Undo").disabled().tooltip("Nothing to undo"),
                ])
                .bordered(true)
                .accessibility_label("Edit")
                .render()]),
            caption(cx, "The whole bar disabled: every control keeps its place and none can be focused. Below, one control disabled alone keeps its hint."),
        ]),
        section(cx, "Edge cases", vec![
            Toolbar::new("toolbar-custom", palette)
                .group(vec![
                    ToolbarItem::button("toolbar-custom-export", "Export as PDF").icon(Icon::File),
                    ToolbarItem::toggle("toolbar-custom-wrap", "Wrap lines", pressed(cx, "toolbar-custom-wrap", true)),
                ])
                .spacer()
                .group(vec![ToolbarItem::element(saved, 40.0)])
                .surface(true)
                .bordered(true)
                .width(wide)
                .accessibility_label("Export")
                .render(),
            caption(cx, "Labels, a labelled toggle, and an element the host built: the bar centers it and leaves it as it is."),
            document(cx, "toolbar-cut").bordered(true).width(150.0_f32.min(cx.width)).render(),
            caption(cx, "Narrower than its controls and without an overflow menu, the bar cuts what falls outside."),
            row(vec![Toolbar::new("toolbar-one", palette)
                .group(vec![ToolbarItem::icon_button("toolbar-one-refresh", Icon::Refresh, "Refresh")])
                .bordered(true)
                .accessibility_label("Refresh")
                .render()]),
            caption(cx, "One control alone: no rule. Without a width the bar takes what its container gives it, here its content's in a row."),
        ]),
    ]
}
