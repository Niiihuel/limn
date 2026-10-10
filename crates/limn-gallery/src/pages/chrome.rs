use super::{caption, row, section, Cx};
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
            "Archivo",
            60.0,
            vec![
                DropdownMenuItem::new(format!("{group}-file-revert"), "Revertir").disabled().into(),
                DropdownMenuItem::new(format!("{group}-file-new"), "Nuevo archivo…").shortcut("Ctrl+N").into(),
                DropdownMenuItem::new(format!("{group}-file-open"), "Abrir…").shortcut("Ctrl+O").into(),
                DropdownMenuEntry::Separator,
                DropdownMenuItem::new(format!("{group}-file-save"), "Guardar").shortcut("Ctrl+S").into(),
            ],
        ),
        menu(
            "edit",
            "Editar",
            52.0,
            vec![
                DropdownMenuItem::new(format!("{group}-edit-undo"), "Deshacer").shortcut("Ctrl+Z").into(),
                DropdownMenuItem::new(format!("{group}-edit-redo"), "Rehacer").shortcut("Ctrl+Shift+Z").into(),
            ],
        ),
        menu(
            "view",
            "Ver",
            36.0,
            vec![
                DropdownMenuItem::new(format!("{group}-view-wrap"), "Ajuste de línea").selected(true).into(),
                item(&format!("{group}-view-zoom"), "Ampliar"),
            ],
        ),
    ]
}

/// A bar in an outline of the width it was fitted to.
fn framed(cx: &Cx, width: f32, bar: Element) -> Element {
    Element::panel(bar)
        .background(cx.palette.surface)
        .border(cx.palette.border, 1.0)
        .radius(RadiusRole::Surface.px())
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
            StatusBarItem::new(id("branch"), 64.0, palette)
                .icon(Icon::GitBranch)
                .label("main")
                .tooltip_with(TooltipSpec::new("main · Ramas del repositorio")),
            StatusBarItem::new(id("problems"), 56.0, palette)
                .icon(Icon::ErrorCircle)
                .count(2)
                .icon(Icon::WarningTriangle)
                .count(7)
                .sizing(StatusBarItemSizing::COUNTS)
                .tooltip_with(TooltipSpec::new("2 errores · 7 advertencias")),
        ],
        vec![
            cell("saved", "Guardado", 60.0, "Guardado · Guardar archivo").passive().priority(1),
            cell("cursor", "Ln 12, Col 4", 76.0, "Ir a línea y columna").priority(6),
            cell("indent", "Espacios: 4", 72.0, "Sangría").priority(4),
            cell("encoding", "UTF-8", 44.0, "Codificación del archivo").passive().priority(2),
            cell("ending", "LF", 24.0, "Finales de línea").priority(3).menu(
                vec![
                    DropdownMenuEntry::Heading("Finales de línea".into()),
                    DropdownMenuItem::new(format!("{ending}=lf"), "LF · Unix").selected(true).into(),
                    item(&format!("{ending}=crlf"), "CRLF · Windows"),
                ],
                cx.state.text(&ending) == "open",
                format!("{ending}=closed"),
                palette,
            ),
            cell("language", "Rust", 40.0, "Modo de lenguaje").priority(5),
        ],
        palette,
    )
    .width(width)
    .overflow(
        StatusBarItem::new(format!("{more}=open"), 20.0, palette)
            .icon(Icon::More)
            .tooltip_with(TooltipSpec::new("Más estado"))
            .menu(
                vec![item(&id("more-cursor"), "Ln 12, Col 4"), item(&id("more-indent"), "Espacios: 4")],
                cx.state.text(&more) == "open",
                format!("{more}=closed"),
                palette,
            ),
    )
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let open_id = |group: &str| {
        let open = cx.state.choice(group, "none");
        (open != "none").then(|| format!("{group}={open}"))
    };
    let menu_bar = MenuBar::new(menus("chrome-menu"), open_id("chrome-menu").as_deref(), palette);
    let compact = MenuBar::new(menus("chrome-compact"), open_id("chrome-compact").as_deref(), palette).compact(
        "chrome-compact=all",
        "chrome-compact=none",
        TooltipSpec::new("Menú principal").placement(TooltipPlacement::Below),
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
        .tooltip_with(TooltipSpec::new("3 notificaciones nuevas"));

    Element::column(vec![
        section(
            cx,
            "Barra de menús",
            vec![
                caption(cx, "Cada menú recibe su ancho; el abierto enfoca su primera acción disponible."),
                row(vec![menu_bar.render()]),
                caption(cx, "Compacta: los mismos menús como submenús de un botón."),
                row(vec![compact.render()]),
            ],
        ),
        section(
            cx,
            "Barra de estado",
            vec![
                caption(cx, "Las celdas finales ceden por prioridad; lo que no entra queda tras «Más»."),
                row(widths),
                framed(cx, width, status(cx, "chrome-status", width).render()),
                caption(cx, "Con filete superior y superficie propia, para cerrar un panel."),
                status(cx, "chrome-ruled", width).top_hairline(true).render(),
                caption(cx, "Celda centrada con tinta propia en el glifo."),
                row(vec![bell.render()]),
            ],
        ),
    ])
    .gap(28.0)
}
