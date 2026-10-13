use super::{caption, row, section, Cx, Section};
use limn::{
    Button, ButtonVariant, Component, DropdownMenuEntry, DropdownMenuItem, Element, Icon, MenuBar,
    MenuBarMenu, RadiusRole, StatusBar, StatusBarItem, StatusBarItemSizing, TooltipPlacement,
    TooltipSpec,
};

const WIDTHS: [&str; 4] = ["200", "260", "340", "420"];

fn item(id: &str, label: &str) -> DropdownMenuEntry {
    DropdownMenuItem::new(id, label).into()
}

// `group` keeps the two demo bars and their items apart.
fn menus(group: &str) -> Vec<MenuBarMenu> {
    let menu = |key: &str, label: &str, width: f32, entries| {
        MenuBarMenu::new(format!("{group}={key}"), label, width, format!("{group}=none"), entries)
    };
    vec![
        menu(
            "file",
            "File",
            60.0,
            vec![
                DropdownMenuItem::new(format!("{group}-file-revert"), "Revert").disabled().into(),
                DropdownMenuItem::new(format!("{group}-file-new"), "New file…").shortcut("Ctrl+N").into(),
                DropdownMenuItem::new(format!("{group}-file-open"), "Open…").shortcut("Ctrl+O").into(),
                DropdownMenuEntry::Separator,
                DropdownMenuItem::new(format!("{group}-file-save"), "Save").shortcut("Ctrl+S").into(),
            ],
        ),
        menu(
            "edit",
            "Edit",
            52.0,
            vec![
                DropdownMenuItem::new(format!("{group}-edit-undo"), "Undo").shortcut("Ctrl+Z").into(),
                DropdownMenuItem::new(format!("{group}-edit-redo"), "Redo").shortcut("Ctrl+Shift+Z").into(),
            ],
        ),
        menu(
            "view",
            "View",
            36.0,
            vec![
                DropdownMenuItem::new(format!("{group}-view-wrap"), "Word wrap").selected(true).into(),
                item(&format!("{group}-view-zoom"), "Zoom in"),
            ],
        ),
    ]
}

/// A bar in an outline of the width it was fitted to.
fn framed(cx: &Cx, width: f32, bar: Element) -> Element {
    // A bar's cells are square, as at the foot of a window: the frame's
    // corners stay round over them.
    Element::panel(bar)
        .background(cx.palette.surface)
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Surface.px())
        .corner_mask(super::ground(cx))
        .clip_children()
        .size(Some(width), None)
}

fn status(cx: &Cx, prefix: &str, width: f32) -> StatusBar {
    let palette = cx.palette;
    let id = |name: &str| format!("{prefix}-{name}");
    let cell = |name: &str, label: &str, width: f32, hint: &str| {
        StatusBarItem::new(id(name), width, palette).label(label).tooltip_with(TooltipSpec::new(hint))
    };
    let ending = id("ending");
    let more = id("more");
    StatusBar::new(
        vec![
            // The one value the bar is read for stands out, under its host's mark.
            StatusBarItem::new(id("branch"), 64.0, palette)
                .icon(Icon::Github)
                .label("main")
                .strong()
                .ink(palette.text)
                .tooltip_with(TooltipSpec::new("main · Repository branches")),
            StatusBarItem::divider(id("rule"), palette),
            StatusBarItem::new(id("problems"), 56.0, palette)
                .tinted_icon(Icon::ErrorCircle, limn::Color::rgb(229, 83, 75))
                .count(2)
                .tinted_icon(Icon::WarningTriangle, limn::Color::rgb(224, 179, 106))
                .count(7)
                .sizing(StatusBarItemSizing::COUNTS)
                .tooltip_with(TooltipSpec::new("2 errors · 7 warnings")),
        ],
        vec![
            cell("saved", "Saved", 60.0, "Saved · Save file").passive().priority(1),
            cell("cursor", "Ln 12, Col 4", 76.0, "Go to line and column").priority(6),
            cell("indent", "Spaces: 4", 72.0, "Indentation").priority(4),
            cell("encoding", "UTF-8", 44.0, "File encoding").passive().priority(2),
            cell("ending", "LF", 24.0, "Line endings").priority(3).menu(
                vec![
                    DropdownMenuEntry::Heading("Line endings".into()),
                    DropdownMenuItem::new(format!("{ending}=lf"), "LF · Unix").selected(true).into(),
                    item(&format!("{ending}=crlf"), "CRLF · Windows"),
                ],
                cx.state.text(&ending) == "open",
                format!("{ending}=closed"),
                palette,
            ),
            cell("language", "Rust", 40.0, "Language mode").priority(5),
        ],
        palette,
    )
    .width(width)
    .overflow(
        StatusBarItem::new(format!("{more}=open"), 20.0, palette)
            .icon(Icon::More)
            .tooltip_with(TooltipSpec::new("More status"))
            .menu(
                vec![item(&id("more-cursor"), "Ln 12, Col 4"), item(&id("more-indent"), "Spaces: 4")],
                cx.state.text(&more) == "open",
                format!("{more}=closed"),
                palette,
            ),
    )
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let open_id = |group: &str| {
        let open = cx.state.choice(group, "none");
        (open != "none").then(|| format!("{group}={open}"))
    };
    let menu_bar = MenuBar::new(menus("chrome-menu"), open_id("chrome-menu").as_deref(), palette);
    let compact = MenuBar::new(menus("chrome-compact"), open_id("chrome-compact").as_deref(), palette).compact(
        "chrome-compact=all",
        "chrome-compact=none",
        TooltipSpec::new("Main menu").placement(TooltipPlacement::Below),
    );

    let chosen = cx.state.choice("chrome-width", "420");
    let width: f32 = chosen.parse().unwrap_or(420.0);
    let width = width.min(cx.width);
    let widths = WIDTHS
        .iter()
        .map(|option| {
            Button::new(format!("chrome-width={option}"), *option, palette)
                .variant(if *option == chosen { ButtonVariant::Secondary } else { ButtonVariant::Outline })
                .render()
        })
        .collect();
    let bell = StatusBarItem::new("chrome-bell", 42.0, palette)
        .icon(Icon::Bell)
        .count(3)
        .ink(palette.text)
        .centered()
        .sizing(StatusBarItemSizing::COUNTS)
        .tooltip_with(TooltipSpec::new("3 new notifications"));

    vec![
        section(
            cx,
            "Menu bar",
            vec![
                caption(cx, "Each menu receives its width; the open menu focuses its first available action."),
                row(vec![menu_bar.render()]),
                caption(cx, "Compact: the same menus appear as submenus of one button."),
                row(vec![compact.render()]),
            ],
        ),
        section(
            cx,
            "Status bar",
            vec![
                caption(cx, "Trailing cells yield by priority; items that do not fit move behind “More”."),
                row(widths),
                framed(cx, width, status(cx, "chrome-status", width).render()),
                caption(cx, "With a top hairline and its own surface, for the bottom of a panel."),
                status(cx, "chrome-ruled", width).top_hairline(true).render(),
                caption(cx, "Centered cell with custom glyph ink."),
                row(vec![bell.render()]),
            ],
        ),
    ]
}
