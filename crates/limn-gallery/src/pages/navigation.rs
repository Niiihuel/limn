use super::{caption, row, section, Cx};
use limn::{
    file_icon, Breadcrumb, BreadcrumbItem, ButtonVariant, Collapsible, Component, ControlSize,
    Element, Icon, IconButton, Link, SegmentedControl, Step, Stepper, Tab, TabItem, TabStrip, Tabs,
    TooltipPlacement,
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
                .size(ControlSize::Dense)
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
    Element::column(vec![
        TabStrip::header(TabStrip::new(group, items, palette).render(), tools, width, palette)
            .render(),
        Element::spacer().background(palette.surface).size(Some(width), Some(36.0)),
    ])
    .size(Some(width), None)
}

pub fn view(cx: &Cx) -> Element {
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
    Element::column(vec![
        section(cx, "Pestañas", vec![row(vec![Tabs::new(tabs("nav-tabs"), palette).id("nav-tabs").render()])]),
        section(cx, "Pestañas de recursos", vec![
            file_tabs(cx, "nav-files-1", 1),
            file_tabs(cx, "nav-files-3", 3),
            file_tabs(cx, "nav-files-12", 12),
            caption(cx, "La pestaña activa se abre sobre el cuerpo; la franja se desplaza hasta mostrarla."),
        ]),
        section(cx, "Control segmentado", vec![row(vec![
            SegmentedControl::new(tabs("nav-segments"), palette).render(),
        ])]),
        section(cx, "Pasos", vec![
            Stepper::new(
                vec![
                    Step::new("nav-step=0", "Contenido"),
                    Step::new("nav-step=1", "Horario"),
                    Step::new("nav-step=2", "Agente"),
                ],
                step,
                palette,
            )
            .width(cx.width.min(420.0))
            .render(),
            // Its own ID: sharing the step's would share its hover and fill.
            row(vec![limn::Button::new("nav-next", "Siguiente", palette)
                .render()
                .dispatch_as(format!("nav-step={}", (step + 1) % 3))]),
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
    ])
    .gap(32.0)
}
