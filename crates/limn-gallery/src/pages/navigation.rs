use super::{caption, row, section, Cx};
use limn::{
    Breadcrumb, BreadcrumbItem, Collapsible, Component, Element, Icon, Link, SegmentedControl, Step,
    Stepper, Tab, Tabs,
};

const VIEWS: [(&str, &str, Icon); 3] = [
    ("files", "Archivos", Icon::Folder),
    ("changes", "Cambios", Icon::GitBranch),
    ("history", "Historial", Icon::History),
];

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
