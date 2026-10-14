use super::{caption, row, section, Cx, Section};
use limn::{
    file_icon, Breadcrumb, BreadcrumbItem, ButtonSizing, ButtonVariant, Collapsible, Component, ContextMenu,
    ContextMenuEntry, ContextMenuItem, ControlSize, Element, FontFamily, Icon, IconButton, Link,
    SegmentedControl, Step, Stepper, Tab, TabItem, TabStrip, Tabs, TabsVariant, TextRole, TooltipPlacement,
};

const VIEWS: [(&str, &str, Icon); 3] = [
    ("files", "Files", Icon::Folder),
    ("changes", "Changes", Icon::GitBranch),
    ("history", "History", Icon::History),
];

const FILES: [&str; 12] = [
    "main.rs",
    "lib.rs",
    "native_streaming_app_tests.rs",
    "Cargo.toml",
    "README.md",
    "tab_strip.rs",
    "a.c",
    "horizontal_scroll_tree.rs",
    "shell.nix",
    "components.md",
    "index.html",
    "code_editor_structure_tests.rs",
];

/// A bar of `count` file tabs over a strip of the body they open onto.
fn file_tabs(cx: &Cx, group: &str, count: usize) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(640.0);
    let chosen = cx.state.choice(group, "0").parse().unwrap_or(0);
    let items = FILES[..count]
        .iter()
        .enumerate()
        .map(|(index, name)| {
            // The host owns the width: here, a rough fit to the name.
            let tab = (68.0 + name.len() as f32 * 7.8).clamp(112.0, 208.0);
            let close = IconButton::new(format!("{group}-close-{index}"), Icon::Close, palette)
                .variant(ButtonVariant::Ghost)
                // 20 px, so the hover stays inside the tab's rounded corner.
                .sizing(ButtonSizing {
                    height: 20.0,
                    horizontal_padding: 2.0,
                    ..ControlSize::Dense.button_sizing()
                })
                .tooltip(format!("Close {name}"))
                .render();
            TabItem::new(
                format!("{group}={index}"),
                format!("{group}-extent-{index}"),
                file_icon(name),
                *name,
                tab,
            )
            .tooltip(format!("crates/limn/src/{name}"))
            .tooltip_placement(TooltipPlacement::Below)
            .active(index == chosen)
            .close(close)
        })
        .collect();
    let tools = IconButton::new(format!("{group}-menu"), Icon::Menu, palette)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Dense)
        .tooltip("All tabs")
        .render();
    let strip = TabStrip::new(group, items, palette).each_tab(|index, tab| {
        let entries = vec![ContextMenuEntry::Item(
            ContextMenuItem::new(format!("{group}-close-{index}"), "Close").icon(Icon::Close),
        )];
        ContextMenu::new(format!("{group}-context-{index}"), tab, entries, palette).render()
    });
    Element::column(vec![
        TabStrip::header(strip.render(), tools, width, palette).render(),
        Element::spacer().background(palette.surface).size(Some(width), Some(36.0)),
    ])
    .size(Some(width), None)
}

/// File tabs whose titles take the ink of a state the host knows, and one
/// whose close slot holds the mark of unsaved changes.
fn status_tabs(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(640.0);
    let ink = |tone| limn::AlertColors::for_tone(tone, palette).accent;
    let dot = Element::spacer()
        .background(palette.text)
        .radius(4.0)
        .size(Some(8.0), Some(8.0));
    let items = vec![
        TabItem::new("nav-status=0", "nav-status-extent-0", file_icon("lib.rs"), "lib.rs", 112.0)
            .title_ink(ink(limn::AlertTone::Warning))
            .tooltip("lib.rs · Modified")
            .active(true),
        TabItem::new("nav-status=1", "nav-status-extent-1", file_icon("notes.md"), "notes.md", 112.0)
            .title_ink(ink(limn::AlertTone::Success))
            .tooltip("notes.md · Untracked"),
        TabItem::new("nav-status=2", "nav-status-extent-2", file_icon("main.rs"), "main.rs", 112.0)
            .tooltip("main.rs · Unsaved")
            .close(dot),
    ];
    let strip = TabStrip::new("nav-status", items, palette).render();
    Element::column(vec![
        TabStrip::header(strip, Element::spacer().size(Some(0.0), None), width, palette).render(),
        Element::spacer().background(palette.surface).size(Some(width), Some(36.0)),
    ])
    .size(Some(width), None)
}

/// A bar of label-only and icon-only tabs, tight to its edges.
fn section_tabs(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(640.0);
    let chosen = cx.state.choice("nav-sections", "problems");
    let tab = |id: &str, content: Element, tab_width: f32| {
        TabStrip::surface(&format!("nav-sections={id}"), content, id == chosen, tab_width, palette)
    };
    let label = |id: &str, title: &str| {
        let ink = if id == chosen { palette.text } else { palette.muted };
        // Faded text is the kind that centers on both axes.
        let text = Element::faded_text(title, 14.0)
            .font_family(FontFamily::Sans)
            .font_size(TextRole::Label.size())
            .foreground(ink)
            .center_content();
        tab(id, text, 96.0)
    };
    let tabs = Element::row(vec![
        label("problems", "Problems"),
        label("output", "Output"),
        label("terminal", "Terminal"),
        tab("history", Element::icon(Icon::History).foreground(palette.muted).centered(), 32.0),
    ])
    .size(Some(320.0), Some(TabStrip::TAB_HEIGHT));
    let leading = Element::row(vec![TabStrip::align(tabs), Element::spacer().flex(1.0)]).flex(1.0);
    let tools = IconButton::new("nav-sections-close", Icon::Close, palette)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Dense)
        .tooltip("Close panel")
        .render();
    bar(TabStrip::header(leading, tools, width, palette).side_inset(2.0).gap(2.0).render(), cx)
}

/// A bar for a group that shows one resource: the tab alone, with no strip.
fn single_tab(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(640.0);
    let tab = TabItem::new("nav-single", "nav-single-extent", Icon::File, "Preview · README.md", 208.0)
        .tooltip("docs/README.md")
        .tooltip_placement(TooltipPlacement::Below)
        .active(true)
        .render(palette);
    let leading = Element::row(vec![TabStrip::align(tab), Element::spacer().flex(1.0)]).flex(1.0);
    let tools = Element::spacer().size(Some(0.0), None);
    bar(TabStrip::header(leading, tools, width, palette).render(), cx)
}

/// A header over a strip of the body its active tab opens onto.
fn bar(header: Element, cx: &Cx) -> Element {
    let width = cx.width.min(640.0);
    Element::column(vec![
        header,
        Element::spacer().background(cx.palette.surface).size(Some(width), Some(36.0)),
    ])
    .size(Some(width), None)
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let tabs = |group: &str| -> Vec<Tab> {
        let chosen = state.choice(group, VIEWS[0].0);
        VIEWS
            .iter()
            .map(|(id, label, icon)| {
                Tab::new(format!("{group}={id}"), *label, 120.0).icon(*icon).active(*id == chosen)
            })
            .collect()
    };
    let step = state.choice("nav-step", "0").parse().unwrap_or(0);
    vec![
        section(cx, "Tabs", vec![
            row(vec![Tabs::new(tabs("nav-tabs"), palette).id("nav-tabs").render()]),
            row(vec![Tabs::new(tabs("nav-fill"), palette)
                .id("nav-fill")
                .fill_width(cx.width.min(520.0))
                .render()]),
            caption(cx, "With fill_width, the tabs share the width supplied by the host."),
        ]),
        section(cx, "Underlined tabs", vec![
            row(vec![Tabs::new(tabs("nav-under"), palette)
                .variant(TabsVariant::Underlined)
                .accent(state.theme().accent_text())
                .gap(16.0)
                .id("nav-under")
                .render()]),
            caption(cx, "No track: the underline slides to the active tab in the accent, over a hairline in the border color."),
            Tabs::new(
                VIEWS
                    .iter()
                    .map(|(id, label, _)| {
                        let chosen = state.choice("nav-under-text", VIEWS[0].0);
                        Tab::new(format!("nav-under-text={id}"), *label, 72.0)
                            .active(*id == chosen)
                            .enabled(*id != "history")
                    })
                    .collect(),
                palette,
            )
            .variant(TabsVariant::Underlined)
            .gap(24.0)
            .id("nav-under-text")
            .render(),
            caption(cx, "Text alone, without an accent: the active tab takes the text color. History is disabled; in a column the hairline runs the full width."),
        ]),
        section(cx, "Resource tabs", vec![
            file_tabs(cx, "nav-files-1", 1),
            file_tabs(cx, "nav-files-3", 3),
            file_tabs(cx, "nav-files-12", 12),
            caption(cx, "The active tab opens above the body; the strip scrolls to reveal it. Right-click a tab to open its menu."),
            section_tabs(cx),
            single_tab(cx),
            caption(cx, "Text-only or icon-only tabs, and a single tab for a group containing one resource."),
            status_tabs(cx),
            caption(cx, "title_ink colors the title with a status supplied by the host; the close slot can hold an unsaved-changes mark."),
        ]),
        section(cx, "Segmented control", vec![row(vec![
            SegmentedControl::new(tabs("nav-segments"), palette).render(),
        ])]),
        section(cx, "Steps", vec![
            caption(cx, "Badges and lines are filled up to the active step; remaining steps are muted. Clicking a step reports it."),
            Stepper::new(
                vec![
                    Step::new("nav-step=0", "Content").icon(Icon::File),
                    Step::new("nav-step=1", "Schedule").icon(Icon::Clock),
                    Step::new("nav-step=2", "Agent").icon(Icon::Settings),
                    Step::new("nav-step=3", "Summary").icon(Icon::InfoCircle),
                ],
                step,
                palette,
            )
            .width(cx.width.min(520.0))
            .render(),
            // Its own ID: sharing the step's would share its hover and fill.
            row(vec![limn::Button::new("nav-next", "Next", palette)
                .render()
                .dispatch_as(format!("nav-step={}", (step + 1) % 4))]),
            caption(cx, "In a column, with the description alongside; without an icon, the badge shows the step number."),
            Stepper::new(
                (0..4)
                    .map(|index| {
                        let step = Step::new(format!("nav-column-step={index}"), format!("Step {}", index + 1))
                            .description(format!("Description of step {}.", index + 1));
                        if index == 3 { step.icon(Icon::CheckCircle) } else { step }
                    })
                    .collect(),
                cx.state.choice("nav-column-step", "2").parse().unwrap_or(2),
                palette,
            )
            .vertical()
            .width(cx.width.min(280.0))
            .render(),
        ]),
        section(cx, "Links", vec![row(vec![
            Link::new("nav-link", "View documentation", palette).render(),
            Link::new("nav-link-muted", "Custom color", palette).ink(palette.muted).render(),
        ])]),
        section(cx, "Path", vec![Breadcrumb::new(
            "nav-path",
            vec![
                BreadcrumbItem::new("limn").action("nav-crumb=root").icon(Icon::Folder),
                BreadcrumbItem::new("crates").action("nav-crumb=crates"),
                BreadcrumbItem::new("limn").action("nav-crumb=limn"),
                BreadcrumbItem::new("src").action("nav-crumb=src"),
                BreadcrumbItem::new("empty.rs").tooltip("crates/limn/src/empty.rs"),
            ],
            palette,
        )
        .width(cx.width.min(420.0))
        .render()]),
        section(cx, "Collapsible", vec![
            Collapsible::new(
                "nav-collapsible",
                "Modified files",
                Element::column(
                    ["crates/limn/src/empty.rs", "crates/limn/src/stepper.rs", "crates/limn/README.md"]
                        .into_iter()
                        .map(|path| caption(cx, path))
                        .collect(),
                )
                .gap(6.0),
                palette,
            )
            .icon(Icon::Folder)
            .detail("3 files")
            .expanded(state.flag("nav-collapsible"))
            .render(),
            Collapsible::new(
                "nav-collapsible-plain",
                "No icon or detail",
                caption(cx, "The body is mounted only while open."),
                palette,
            )
            .expanded(state.flag("nav-collapsible-plain"))
            .render(),
        ]),
        section(cx, "Selected item icon", vec![
            row(
                [Icon::Folder, Icon::Settings, Icon::Chat, Icon::Bell, Icon::Search]
                    .into_iter()
                    .flat_map(|icon| [
                        Element::icon(icon).foreground(palette.muted),
                        Element::icon(icon.filled()).foreground(palette.text),
                        Element::spacer().size(Some(8.0), None),
                    ])
                    .collect(),
            ),
            caption(cx, "Icon::filled() returns the solid variant; icons without one, such as the magnifier, stay the same."),
        ]),
        section(cx, "Servers and settings", vec![
            row(
                [
                    (Icon::Lock, "Lock"),
                    (Icon::Neutral, "Neutral"),
                    (Icon::ChevronUp, "ChevronUp"),
                    (Icon::Invite, "Invite"),
                    (Icon::Leave, "Leave"),
                    (Icon::DragHandle, "DragHandle"),
                ]
                .into_iter()
                .flat_map(|(icon, name)| [
                    Element::icon(icon).foreground(palette.text),
                    caption(cx, name),
                    Element::spacer().size(Some(8.0), None),
                ])
                .collect(),
            ),
            caption(cx, "A private channel, a permission left as inherited, an open menu, an invitation, leaving a server and a handle to drag a row by."),
        ]),
    ]
}
