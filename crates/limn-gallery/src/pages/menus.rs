//! Selectors, popover panels and suggestion lists: what a menu needs once it
//! lists more than actions.

use super::{caption, row, section, Cx};
use limn::{
    Button, ButtonVariant, Component, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
    DropdownMenuSection, Element, FontFamily, HeaderSizing, Icon, IconButton, Kbd, MenuHeader,
    MenuItem, PanelHeader, Popover, PopoverPanel, ScrollArea, SelectorSizing, SuggestionItem,
    SuggestionList, TextField, TooltipSpec,
};

const MODELS: [(&str, &str, &str, &str); 5] = [
    ("north", "North Large", "north/large-2", "Revisado"),
    ("north", "North Small", "north/small-2", ""),
    ("harbor", "Harbor Pro", "harbor/pro-latest", "Sin verificar"),
    ("harbor", "Harbor Flash", "harbor/flash", "Declarado"),
    ("harbor", "Harbor Legacy", "harbor/legacy-1", "No compatible"),
];

const COMMANDS: [(&str, &str, &str, Icon); 6] = [
    ("Comandos", "/plan", "Describe los pasos antes de editar", Icon::Plan),
    ("Comandos", "/review", "Revisa los cambios sin confirmar", Icon::Search),
    ("Comandos", "/files", "Adjunta archivos del proyecto", Icon::File),
    ("Plantillas", "/docs", "Redacta la documentación de un módulo", Icon::Text),
    ("Plantillas", "/tests", "Escribe pruebas para la selección", Icon::Code),
    ("Plantillas", "/release", "Prepara las notas de la versión", Icon::Archive),
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
        trigger(cx, "menus-selector=open", "Modelo y esfuerzo"),
        vec![
            DropdownMenuEntry::Heading("Para el próximo mensaje".into()),
            DropdownMenuItem::new("menus-selector=model", "Modelo")
                .icon(Icon::Server)
                .description("North Large")
                .chevron()
                .into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("menus-selector=effort", "Esfuerzo")
                .icon(Icon::Brain)
                .description("Automático")
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
    for (provider, name) in [("north", "North"), ("harbor", "Harbor · En tu cuenta")] {
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
                    .enabled(*status != "No compatible")
                    .tooltip_with(TooltipSpec::new(*title).description(format!("Identificador: {id}")))
                    .into(),
            );
        }
    }
    if entries.is_empty() {
        entries.push(DropdownMenuEntry::Message("Sin resultados. Prueba otro nombre.".into()));
    }
    DropdownMenu::new(
        trigger(cx, "menus-catalog=open", "Catálogo de modelos"),
        entries,
        cx.state.text("menus-catalog") == "open",
        "menus-catalog=closed",
        palette,
    )
    .header(
        MenuHeader::new(palette)
            .back("menus-catalog=closed", "Modelo")
            .trailing(
                IconButton::new("menus-refresh", Icon::Refresh, palette)
                    .variant(ButtonVariant::Ghost)
                    .tooltip_with(TooltipSpec::new("Actualizar el catálogo"))
                    .render(),
            )
            .search("menus-search", cx.state.text("menus-search"), "Buscar modelos")
            .width(width - 8.0)
            .render(),
    )
    .footer(
        Button::new("menus-connect", "Conectar proveedor", palette)
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
                text: "No se pudo leer el repositorio: el directorio de trabajo no existe o no \
                       es accesible. Comprueba la ruta, los permisos de lectura y que el \
                       repositorio no se haya movido; después vuelve a actualizar la lista."
                    .into(),
            },
        ],
        _ => vec![DropdownMenuEntry::Message("Todavía no hay ramas en este repositorio.".into())],
    };
    entries.push(DropdownMenuEntry::Separator);
    for (id, label) in [("loading", "Cargando"), ("empty", "Vacío"), ("error", "Error")] {
        entries.push(
            DropdownMenuItem::new(format!("menus-state={id}"), label)
                .selected(state == id)
                .into(),
        );
    }
    DropdownMenu::new(
        trigger(cx, "menus-states=open", "Estados"),
        entries,
        cx.state.text("menus-states") == "open",
        "menus-states=closed",
        cx.palette,
    )
    .header(MenuHeader::new(cx.palette).title("Cambiar rama").render())
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
                .trailing("Activa")
                .tooltip_with(TooltipSpec::new("Terminal 1").description("Directorio inicial: ~/proyecto"))
                .trailing_action("menus-row-close-1", Icon::Trash, TooltipSpec::new("Cerrar terminal"))
                .trailing_dispatch_as("menus-row=closed-one")
                .width(width)
                .render(),
            MenuItem::new("menus-row=two", "Terminal 2", palette)
                .icon(Icon::Terminal)
                .trailing_action("menus-row-close-2", Icon::Trash, TooltipSpec::new("Cerrar terminal"))
                .trailing_dispatch_as("menus-row=closed-two")
                .width(width)
                .render(),
            MenuItem::new("menus-row=project", "limn", palette)
                .icon(Icon::Folder)
                .description("~/proyectos/personal/limn")
                .selected(true)
                .width(width)
                .render(),
            MenuItem::new("menus-row=gone", "Proyecto movido", palette)
                .icon(Icon::Folder)
                .description("~/proyectos/antiguo")
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

/// A popover that is more than a menu: head, scrolling body, fixed foot.
fn panel(cx: &Cx) -> Element {
    let palette = cx.palette;
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
        .map(|index| line(&format!("Cuenta {index}"), &format!("{} %", index * 7 % 100)))
        .collect();
    Popover::new(
        trigger(cx, "menus-panel=open", "Cuenta y uso"),
        PopoverPanel::new(palette)
            .header(
                PanelHeader::new("Cuenta y uso", palette)
                    .dismiss("menus-panel=closed", "Cerrar")
                    .sizing(HeaderSizing::POPOVER)
                    .render(),
            )
            .body(ScrollArea::new("menus-panel-body", Element::column(accounts)).render())
            .body_max_height(180.0)
            .footer(
                Button::new("menus-panel=details", "Ver uso y proveedores", palette)
                    .trailing_icon(Icon::ArrowRight)
                    .variant(ButtonVariant::Ghost)
                    .render(),
            )
            .render(),
        cx.state.text("menus-panel") == "open",
        "menus-panel=closed",
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
        TextField::new("menus-suggest-input", cx.state.text("menus-suggest-input"), "Escribe / para ver comandos", palette)
            .width(360.0)
            .render(),
        "menus-suggest-options",
        items,
        1,
        cx.state.text("menus-suggest") == "open",
        "menus-suggest=closed",
        palette,
    )
    .title("Comandos y plantillas")
    .empty("No hay comandos que coincidan.")
    .hints(hints(cx))
    .note("Resultados limitados")
    .width(cx.width)
    .placement(limn::PopoverPlacement::BottomStart)
    .render()
}

fn hints(cx: &Cx) -> Vec<Element> {
    vec![
        Kbd::new("↑↓", cx.palette).caption("Elegir").render(),
        Kbd::new("Enter", cx.palette).alternative("Tab").caption("Aceptar").render(),
        Kbd::new("Esc", cx.palette).caption("Cerrar").render(),
    ]
}

pub fn view(cx: &Cx) -> Element {
    Element::column(vec![
        section(cx, "Selector y catálogo", vec![
            row(vec![selector(cx), catalog(cx), states(cx)]),
            caption(cx, &format!("Modelo: {}", cx.state.choice("menus-model", MODELS[0].2))),
        ]),
        section(cx, "Filas de menú", vec![
            row(vec![rows(cx)]),
            caption(cx, &format!("Última fila: {}", cx.state.text("menus-row"))),
        ]),
        section(cx, "Panel en popover", vec![row(vec![panel(cx)])]),
        section(cx, "Lista de sugerencias", vec![
            row(vec![
                suggestions(cx),
                Button::new("menus-suggest=open", "Mostrar", cx.palette).variant(ButtonVariant::Outline).render(),
                Button::new("menus-suggest-empty", "Vaciar", cx.palette).variant(ButtonVariant::Ghost).render(),
            ]),
            caption(cx, &format!("Elegido: {}", cx.state.text("menus-suggest-pick"))),
        ]),
        section(cx, "Teclas con leyenda", vec![row(hints(cx))]),
    ])
    .gap(32.0)
}
