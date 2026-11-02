//! Selectors, popover panels and suggestion lists: what a menu needs once it
//! lists more than actions.

use super::{caption, row, section, Cx, Section};
use limn::{
    Avatar, Button, ButtonVariant, Color, Component, ComponentPalette, ContextMenuMetrics, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
    DropdownMenuSection, Element, FontFamily, PanelHeaderSizing, Icon, IconButton, Kbd, MenuHeader,
    MenuItem, PanelHeader, Popover, PopoverPanel, PopoverPanelSizing, ScrollArea, SelectorSizing,
    SuggestionItem, SuggestionList, TextField, TooltipPlacement, TooltipSpec,
};

const MODELS: [(&str, &str, &str, &str); 5] = [
    ("north", "North Large", "north/large-2", "Reviewed"),
    ("north", "North Small", "north/small-2", ""),
    ("harbor", "Harbor Pro", "harbor/pro-latest", "Unverified"),
    ("harbor", "Harbor Flash", "harbor/flash", "Declared"),
    ("harbor", "Harbor Legacy", "harbor/legacy-1", "Unsupported"),
];

const COMMANDS: [(&str, &str, &str, Icon); 6] = [
    ("Commands", "/plan", "Describe the steps before editing", Icon::Plan),
    ("Commands", "/review", "Review uncommitted changes", Icon::Search),
    ("Commands", "/files", "Attach project files", Icon::File),
    ("Templates", "/docs", "Write documentation for a module", Icon::Text),
    ("Templates", "/tests", "Write tests for the selection", Icon::Code),
    ("Templates", "/release", "Prepare release notes", Icon::Archive),
];

fn trigger(cx: &Cx, id: &str, label: &str) -> Element {
    Button::new(id, label, cx.palette)
        .variant(ButtonVariant::Outline)
        .trailing_icon(Icon::ChevronDown)
        .render()
}

/// The first view of a selector: what it sets and a chevron into each list.
fn selector(cx: &Cx) -> Element {
    DropdownMenu::new(
        trigger(cx, "menus-selector=open", "Model and effort"),
        vec![
            DropdownMenuEntry::Heading("For the next message".into()),
            DropdownMenuItem::new("menus-selector=model", "Model")
                .icon(Icon::Server)
                .description("North Large")
                .chevron()
                .into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("menus-selector=effort", "Effort")
                .icon(Icon::Brain)
                .description("Automatic")
                .chevron()
                .into(),
        ],
        cx.state.text("menus-selector") == "open",
        "menus-selector=closed",
        cx.palette,
    )
    .context_menu_style()
    .width(SelectorSizing::COMPACT.preferred_width)
    .render()
}

/// A searchable catalog: fixed header, folding sections, two-line rows and a
/// footer that stays while the options scroll.
fn catalog(cx: &Cx) -> Element {
    let palette = cx.palette;
    let sizing = SelectorSizing::CATALOG;
    let width = sizing.preferred_width;
    let query = cx.state.text("menus-search").to_lowercase();
    let chosen = cx.state.choice("menus-model", MODELS[0].2);
    let mut entries = Vec::new();
    for (provider, name) in [("north", "North"), ("harbor", "Harbor · In your account")] {
        let models: Vec<_> = MODELS
            .iter()
            .filter(|model| model.0 == provider && model.1.to_lowercase().contains(&query))
            .collect();
        if models.is_empty() {
            continue;
        }
        if !entries.is_empty() {
            entries.push(DropdownMenuEntry::Separator);
        }
        let toggle = format!("menus-folded-{provider}");
        let collapsed = cx.state.flag(&toggle);
        entries.push(
            DropdownMenuSection::new(toggle, name)
                .icon(Icon::Server)
                .count(models.len())
                .collapsed(collapsed)
                .into(),
        );
        if collapsed {
            continue;
        }
        for (_, title, id, status) in models {
            entries.push(
                DropdownMenuItem::new(format!("menus-model={id}"), *title)
                    .icon(Icon::Server)
                    .description(*id)
                    .detail_mono(true)
                    .status(*status)
                    // One slot for every status, so they start in a column.
                    .status_width(84.0)
                    .selected(*id == chosen)
                    .enabled(*status != "Unsupported")
                    .tooltip_with(TooltipSpec::new(*title).description(format!("Identifier: {id}")))
                    .into(),
            );
        }
    }
    if entries.is_empty() {
        entries.push(DropdownMenuEntry::Message("No results. Try another name.".into()));
    }
    DropdownMenu::new(
        trigger(cx, "menus-catalog=open", "Model catalog"),
        entries,
        cx.state.text("menus-catalog") == "open",
        "menus-catalog=closed",
        palette,
    )
    .header(
        MenuHeader::new(palette)
            .back("menus-catalog=closed", "Model")
            .trailing(
                IconButton::new("menus-refresh", Icon::Refresh, palette)
                    .variant(ButtonVariant::Ghost)
                    .tooltip_with(TooltipSpec::new("Refresh the catalog"))
                    .render(),
            )
            .search("menus-search", cx.state.text("menus-search"), "Search models")
            .width(width - 8.0)
            .render(),
    )
    .footer(
        Button::new("menus-connect", "Connect provider", palette)
            .icon(Icon::Add)
            .variant(ButtonVariant::Ghost)
            .width(width - 8.0)
            .render(),
    )
    .context_menu_style()
    .width(width)
    .max_visible_items(6)
    .render()
}

/// What a menu shows in place of its items.
fn states(cx: &Cx) -> Element {
    let state = cx.state.choice("menus-state", "loading");
    let mut entries = match state {
        "loading" => vec![DropdownMenuEntry::Loading { id: "menus-skeleton".into(), rows: 3 }],
        "error" => vec![
            DropdownMenuEntry::Error {
                id: "menus-error".into(),
                text: "Could not read the repository: the working directory is missing or \
                       inaccessible. Check the path, read permissions, and whether the \
                       repository has moved; then refresh the list again."
                    .into(),
            },
        ],
        _ => vec![DropdownMenuEntry::Message("This repository has no branches yet.".into())],
    };
    entries.push(DropdownMenuEntry::Separator);
    for (id, label) in [("loading", "Loading"), ("empty", "Empty"), ("error", "Error")] {
        entries.push(
            DropdownMenuItem::new(format!("menus-state={id}"), label)
                .selected(state == id)
                .into(),
        );
    }
    DropdownMenu::new(
        trigger(cx, "menus-states=open", "States"),
        entries,
        cx.state.text("menus-states") == "open",
        "menus-states=closed",
        cx.palette,
    )
    .header(MenuHeader::new(cx.palette).title("Switch branch").render())
    .context_menu_style()
    .width(280.0)
    .render()
}

/// Menu rows on their own, as a host lays them out in a custom list.
fn rows(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = 300.0;
    Element::panel(
        Element::column(vec![
            MenuItem::new("menus-row=one", "Terminal 1", palette)
                .icon(Icon::Terminal)
                .trailing("Active")
                .tooltip_with(TooltipSpec::new("Terminal 1").description("Initial directory: ~/project"))
                .trailing_action("menus-row-close-1", Icon::Trash, TooltipSpec::new("Close terminal"))
                .trailing_dispatch_as("menus-row=closed-one")
                .width(width)
                .render(),
            MenuItem::new("menus-row=two", "Terminal 2", palette)
                .icon(Icon::Terminal)
                .trailing_action("menus-row-close-2", Icon::Trash, TooltipSpec::new("Close terminal"))
                .trailing_dispatch_as("menus-row=closed-two")
                .width(width)
                .render(),
            MenuItem::new("menus-row=project", "limn", palette)
                .icon(Icon::Folder)
                .description("~/projects/personal/limn")
                .selected(true)
                .width(width)
                .render(),
            MenuItem::new("menus-row=gone", "Project moved", palette)
                .icon(Icon::Folder)
                .description("~/projects/old")
                .selected(false)
                .enabled(false)
                .width(width)
                .render(),
        ])
        .gap(2.0),
    )
    .padding(4.0)
    .background(palette.surface)
    .border(palette.border, 1.0)
    .radius(limn::tokens().radius.surface)
}

/// Actions whose names are cut to one line; the hint says the rest. `dense` draws
/// them with 32 px rows and 14 px labels.
fn actions(cx: &Cx, dense: bool) -> Element {
    let key = if dense {
        "menus-dense"
    } else {
        "menus-actions"
    };
    let action = |id: &str, icon: Icon, label: &str, description: &str| {
        DropdownMenuItem::new(format!("menus-action={id}"), label)
            .icon(icon)
            .truncate()
            .tooltip_with(
                TooltipSpec::new(label)
                    .description(description)
                    .placement(TooltipPlacement::Left),
            )
            .into()
    };
    DropdownMenu::new(
        trigger(
            cx,
            &format!("{key}=open"),
            if dense { "Dense actions" } else { "Actions" },
        ),
        vec![
            action("copy", Icon::File, "Copy address", "Copy the visible address to the clipboard."),
            action(
                "settings",
                Icon::Code,
                "Copy connection settings",
                "Copy the settings for connecting tools to this page.",
            ),
            DropdownMenuEntry::Separator,
            action("restart", Icon::Refresh, "Restart", "Open a new context."),
        ],
        cx.state.text(key) == "open",
        format!("{key}=closed"),
        cx.palette,
    )
    .context_menu_style()
    .context_metrics(if dense {
        ContextMenuMetrics {
            row_height: 32.0,
            label_size: 14.0,
            inset: 8.0,
            separator_height: 17.0,
            ..ContextMenuMetrics::default()
        }
    } else {
        ContextMenuMetrics::default()
    })
    .width(192.0)
    .render()
}

/// A popover that is more than a menu: head, scrolling body, fixed foot.
/// `padded` is the same panel in a looser profile with one hairline.
fn panel(cx: &Cx, padded: bool) -> Element {
    let palette = cx.palette;
    let key = if padded { "menus-padded" } else { "menus-panel" };
    let line = |label: &str, value: &str| {
        Element::row(vec![
            caption(cx, label).flex(1.0),
            Element::text(value)
                .font_family(FontFamily::Sans)
                .font_size(12.0)
                .foreground(palette.text),
        ])
        .size(None, Some(24.0))
    };
    let accounts = (1..=14)
        .map(|index| line(&format!("Account {index}"), &format!("{} %", index * 7 % 100)))
        .collect();
    Popover::new(
        trigger(cx, &format!("{key}=open"), if padded { "With padding" } else { "Account and usage" }),
        PopoverPanel::new(palette)
            .header(
                PanelHeader::new("Account and usage", palette)
                    .dismiss(format!("{key}=closed"), "Close")
                    .sizing(PanelHeaderSizing::POPOVER)
                    .render(),
            )
            .body(ScrollArea::new(format!("{key}-body"), Element::column(accounts)).render())
            .body_max_height(180.0)
            .footer(
                Button::new(format!("{key}=details"), "View usage and providers", palette)
                    .trailing_icon(Icon::ArrowRight)
                    .variant(ButtonVariant::Ghost)
                    .render(),
            )
            .sizing(if padded {
                PopoverPanelSizing { gap: 8.0, padding: 10.0, header_rule: false }
            } else {
                PopoverPanelSizing::DEFAULT
            })
            .render(),
        cx.state.text(key) == "open",
        format!("{key}=closed"),
        palette,
    )
    .width(300.0)
    .render()
}

/// Completions over an input. The gallery opens them with the button, since
/// it has no editor to watch; a host opens them on a trigger character.
fn suggestions(cx: &Cx) -> Element {
    let palette = cx.palette;
    let empty = cx.state.flag("menus-suggest-empty");
    let mut section = "";
    let items = COMMANDS
        .iter()
        .filter(|_| !empty)
        .map(|(group, label, description, icon)| {
            let item = SuggestionItem::new(format!("menus-suggest-pick={label}&menus-suggest=closed"), *label)
                .icon(*icon)
                .description(*description);
            if section != *group {
                section = group;
                item.section(*group)
            } else {
                item
            }
        })
        .collect();
    SuggestionList::new(
        TextField::new("menus-suggest-input", cx.state.text("menus-suggest-input"), "Type / to see commands", palette)
            .width(360.0)
            .render(),
        "menus-suggest-options",
        items,
        1,
        cx.state.text("menus-suggest") == "open",
        "menus-suggest=closed",
        palette,
    )
    .title("Commands and templates")
    .empty("No matching commands.")
    .hints(hints(cx))
    .note("Limited results")
    .width(cx.width)
    .placement(limn::PopoverPlacement::BottomStart)
    .render()
}

/// People to mention: a face, the name in a role's color and the handle at
/// the end of the line.
fn people(cx: &Cx) -> Element {
    let palette = cx.palette;
    let items = [
        ("Ada Lovelace", "ada", Color::rgb(226, 99, 99), Some(Color::rgb(46, 204, 113))),
        ("Bruno Díaz", "bruno", Color::rgb(88, 148, 226), None),
        ("Carla Pérez", "carla", Color::rgb(214, 160, 96), Some(Color::rgb(155, 89, 182))),
    ]
    .into_iter()
    .map(|(name, handle, color, role)| {
        let face = Avatar::new(name, ComponentPalette { selected: color, ..palette }).size(24.0).render();
        let item = SuggestionItem::new(format!("menus-mention-pick={handle}&menus-mention=closed"), name)
            .leading(face)
            .detail(handle);
        match role {
            Some(role) => item.ink(role),
            None => item,
        }
    })
    .collect();
    SuggestionList::new(
        TextField::new("menus-mention-input", cx.state.text("menus-mention-input"), "Type @ to mention", palette)
            .width(360.0)
            .render(),
        "menus-mention-options",
        items,
        0,
        cx.state.text("menus-mention") == "open",
        "menus-mention=closed",
        palette,
    )
    .title("Members")
    .width(cx.width)
    .placement(limn::PopoverPlacement::BottomStart)
    .render()
}

fn hints(cx: &Cx) -> Vec<Element> {
    let hints = vec![
        Kbd::new("↑↓", cx.palette).caption("Choose").render(),
        Kbd::new("Enter", cx.palette).alternative("Tab").caption("Accept").render(),
        Kbd::new("Esc", cx.palette).caption("Close").render(),
    ];
    if cx.width < 360.0 {
        vec![Element::column(hints).gap(8.0)]
    } else {
        hints
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
    vec![
        section(cx, "Select and catalog", vec![
            row(vec![selector(cx), catalog(cx), states(cx)]),
            row(vec![actions(cx, false), actions(cx, true)]),
            caption(cx, &format!("Model: {}", cx.state.choice("menus-model", MODELS[0].2))),
        ]),
        section(cx, "Menu rows", vec![
            row(vec![rows(cx)]),
            caption(cx, &format!("Last row: {}", cx.state.text("menus-row"))),
        ]),
        section(cx, "Popover panel", vec![row(vec![panel(cx, false), panel(cx, true)])]),
        section(cx, "Suggestion list", vec![
            row(vec![
                suggestions(cx),
                Button::new("menus-suggest=open", "Show", cx.palette).variant(ButtonVariant::Outline).render(),
                Button::new("menus-suggest-empty", "Clear", cx.palette).variant(ButtonVariant::Ghost).render(),
            ]),
            caption(cx, &format!("Selected: {}", cx.state.text("menus-suggest-pick"))),
            row(vec![
                people(cx),
                Button::new("menus-mention=open", "Show", cx.palette).variant(ButtonVariant::Outline).render(),
            ]),
            caption(cx, ".leading(avatar), .ink(role color) and .detail(handle): one line in the middle of the row."),
        ]),
        section(cx, "Keys with captions", vec![row(hints(cx))]),
    ]
}
