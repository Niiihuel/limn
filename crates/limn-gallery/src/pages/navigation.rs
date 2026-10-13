use super::{caption, row, section, Cx, Section};
use limn::{
    file_icon, Breadcrumb, BreadcrumbItem, ButtonSizing, ButtonVariant, Collapsible, Component, ContextMenu,
    ContextMenuEntry, ContextMenuItem, ControlSize, Element, FontFamily, Icon, IconButton, Link,
    SegmentedControl, Step, Stepper, Tab, TabItem, TabStrip, Tabs, TextRole, TooltipPlacement,
};

const VIEWS: [(&str, &str, Icon); 3] = [
    ("files", "Archivos", Icon::Folder),
    ("changes", "Cambios", Icon::GitBranch),
    ("history", "Historial", Icon::History),
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
                .tooltip(format!("Cerrar {name}"))
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
        .tooltip("Todas las pestañas")
        .render();
    let strip = TabStrip::new(group, items, palette).each_tab(|index, tab| {
        let entries = vec![ContextMenuEntry::Item(
            ContextMenuItem::new(format!("{group}-close-{index}"), "Cerrar").icon(Icon::Close),
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
            .tooltip("lib.rs · Modificado")
            .active(true),
        TabItem::new("nav-status=1", "nav-status-extent-1", file_icon("notes.md"), "notes.md", 112.0)
            .title_ink(ink(limn::AlertTone::Success))
            .tooltip("notes.md · Sin seguimiento"),
        TabItem::new("nav-status=2", "nav-status-extent-2", file_icon("main.rs"), "main.rs", 112.0)
            .tooltip("main.rs · Sin guardar")
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
        label("problems", "Problemas"),
        label("output", "Salida"),
        label("terminal", "Terminal"),
        tab("history", Element::icon(Icon::History).foreground(palette.muted).centered(), 32.0),
    ])
    .size(Some(320.0), Some(TabStrip::TAB_HEIGHT));
    let leading = Element::row(vec![TabStrip::align(tabs), Element::spacer().flex(1.0)]).flex(1.0);
    let tools = IconButton::new("nav-sections-close", Icon::Close, palette)
        .variant(ButtonVariant::Ghost)
        .size(ControlSize::Dense)
        .tooltip("Cerrar panel")
        .render();
    bar(TabStrip::header(leading, tools, width, palette).side_inset(2.0).gap(2.0).render(), cx)
}

/// A bar for a group that shows one resource: the tab alone, with no strip.
fn single_tab(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(640.0);
    let tab = TabItem::new("nav-single", "nav-single-extent", Icon::File, "Vista previa · README.md", 208.0)
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
        section(cx, "Pestañas", vec![
            row(vec![Tabs::new(tabs("nav-tabs"), palette).id("nav-tabs").render()]),
            row(vec![Tabs::new(tabs("nav-fill"), palette)
                .id("nav-fill")
                .fill_width(cx.width.min(520.0))
                .render()]),
            caption(cx, "Con fill_width las pestañas se reparten el ancho que da el anfitrión."),
        ]),
        section(cx, "Pestañas de recursos", vec![
            file_tabs(cx, "nav-files-1", 1),
            file_tabs(cx, "nav-files-3", 3),
            file_tabs(cx, "nav-files-12", 12),
            caption(cx, "La pestaña activa se abre sobre el cuerpo; la franja se desplaza hasta mostrarla. Clic derecho en una pestaña abre su menú."),
            section_tabs(cx),
            single_tab(cx),
            caption(cx, "Pestañas de sólo texto o icono, y una pestaña suelta para un grupo de un solo recurso."),
            status_tabs(cx),
            caption(cx, "title_ink tiñe el título con un estado que conoce el anfitrión; el hueco de cierre puede llevar la marca de cambios sin guardar."),
        ]),
        section(cx, "Control segmentado", vec![row(vec![
            SegmentedControl::new(tabs("nav-segments"), palette).render(),
        ])]),
        section(cx, "Pasos", vec![
            caption(cx, "Los pasos hasta el activo tienen la insignia y la línea llenas; los que faltan, apagadas. Un clic en un paso lo informa."),
            Stepper::new(
                vec![
                    Step::new("nav-step=0", "Contenido").icon(Icon::File),
                    Step::new("nav-step=1", "Horario").icon(Icon::Clock),
                    Step::new("nav-step=2", "Agente").icon(Icon::Settings),
                    Step::new("nav-step=3", "Resumen").icon(Icon::InfoCircle),
                ],
                step,
                palette,
            )
            .width(cx.width.min(520.0))
            .render(),
            // Its own ID: sharing the step's would share its hover and fill.
            row(vec![limn::Button::new("nav-next", "Siguiente", palette)
                .render()
                .dispatch_as(format!("nav-step={}", (step + 1) % 4))]),
            caption(cx, "En columna, con la descripción al lado; sin icono, la insignia lleva el número del paso."),
            Stepper::new(
                (0..4)
                    .map(|index| {
                        let step = Step::new(format!("nav-column-step={index}"), format!("Paso {}", index + 1))
                            .description(format!("Descripción del paso {}.", index + 1));
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
        section(cx, "Enlaces", vec![row(vec![
            Link::new("nav-link", "Ver documentación", palette).render(),
            Link::new("nav-link-muted", "Color propio", palette).color(palette.muted).render(),
        ])]),
        section(cx, "Ruta", vec![Breadcrumb::new(
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
        section(cx, "Plegable", vec![
            Collapsible::new(
                "nav-collapsible",
                "Archivos modificados",
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
            .detail("3 archivos")
            .expanded(state.flag("nav-collapsible"))
            .render(),
            Collapsible::new(
                "nav-collapsible-plain",
                "Sin icono ni detalle",
                caption(cx, "El cuerpo sólo se monta mientras está abierto."),
                palette,
            )
            .expanded(state.flag("nav-collapsible-plain"))
            .render(),
        ]),
        section(cx, "Icono del elemento elegido", vec![
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
            caption(cx, "Icon::filled() da la forma sólida; un icono sin ella, como la lupa, queda igual."),
        ]),
    ]
}
