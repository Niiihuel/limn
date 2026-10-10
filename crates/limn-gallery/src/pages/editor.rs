use super::{caption, row, section, Cx};
use limn::{
    Breadcrumb, BreadcrumbItem, Button, ButtonVariant, Collapsible, Color, Component, Composer,
    ComponentPalette, ControlSize, Element, FindBar, FindIds, FindLabels, FindOptions, Icon,
    IconButton, Marker,
};

const DANGER: Color = Color::rgb(229, 83, 75);
const WIDTHS: [&str; 3] = ["260", "480", "520"];
const PATH: &str = "crates/limn/src/find_bar.rs";

fn find_labels() -> FindLabels {
    FindLabels {
        query: "Buscar en el archivo".into(),
        replacement: "Reemplazar por".into(),
        case: "Distinguir mayúsculas y minúsculas".into(),
        word: "Sólo palabras completas".into(),
        regex: "Expresión regular".into(),
        in_selection: "Buscar dentro de la selección".into(),
        toggle_replace: "Mostrar u ocultar reemplazo".into(),
        previous: "Coincidencia anterior".into(),
        next: "Coincidencia siguiente".into(),
        dismiss: "Cerrar búsqueda".into(),
        replace_one: "Reemplazar coincidencia".into(),
        replace_all: "Reemplazar todas las coincidencias".into(),
        count: "Coincidencia actual / total".into(),
        ..FindLabels::default()
    }
}

// The gallery flips a flag per click, so the option IDs are the flags.
fn find_bar(cx: &Cx, width: f32) -> Element {
    let state = cx.state;
    let query = state.text("editor-find-query");
    let found = !query.is_empty() && !query.contains("zzz");
    let regex = state.flag("editor-find-regex");
    FindBar::new("editor-find", query, cx.palette)
        .ids(FindIds {
            query: "editor-find-query".into(),
            replacement: "editor-find-replacement".into(),
            case: "editor-find-case".into(),
            word: "editor-find-word".into(),
            regex: "editor-find-regex".into(),
            in_selection: "editor-find-scope".into(),
            toggle_replace: "editor-find-replace".into(),
            ..FindIds::new("editor-find")
        })
        .labels(find_labels())
        .replacement(Some(state.text("editor-find-replacement")))
        .replace_open(state.flag("editor-find-replace"))
        .options(FindOptions {
            case: state.flag("editor-find-case"),
            word: state.flag("editor-find-word"),
            regex,
            in_selection: state.flag("editor-find-scope"),
        })
        .count_label(match (query.is_empty(), found) {
            (true, _) => "",
            (false, true) => "2 / 14",
            (false, false) => "0 / 0",
        })
        .has_matches(found)
        .has_current(found)
        .no_matches(!query.is_empty() && !found)
        .error((regex && query.contains('(') && !query.contains(')')).then_some("Paréntesis sin cerrar en la expresión"))
        .ink(DANGER)
        .width(width)
        .render()
}

fn composer(cx: &Cx, width: f32) -> Element {
    let palette = cx.palette;
    let strip = Element::row(vec![
        Button::new("editor-composer-project", "limn", palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Compact)
            .icon(Icon::Folder)
            .render(),
        Button::new("editor-composer-branch", "ui-separation", palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Compact)
            .icon(Icon::GitBranch)
            .render(),
        Element::spacer().flex(1.0),
    ])
    .gap(4.0)
    .size(Some(Composer::header_strip_width(width)), Some(28.0));
    let tool = |id: &str, icon| {
        IconButton::new(id, icon, palette).variant(ButtonVariant::Ghost).size(ControlSize::Compact).render()
    };
    Composer::new("editor-composer", cx.state.text("editor-composer"), "Describe una tarea", width, palette)
        .pill_controls(tool("editor-composer-add", Icon::Add), tool("editor-composer-send", Icon::Send))
        .auto_grow(28.0, 120.0)
        .header_strip(strip, ComponentPalette { surface: palette.raised, ..palette })
        .render()
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let chosen = state.choice("editor-find-width", "520");
    let width: f32 = chosen.parse().unwrap_or(520.0);
    let widths = WIDTHS
        .iter()
        .map(|option| {
            Button::new(format!("editor-find-width={option}"), *option, palette)
                .variant(if *option == chosen { ButtonVariant::Secondary } else { ButtonVariant::Outline })
                .render()
        })
        .collect();
    let crumb_width = cx.width.min(420.0);
    let linked = |id: &str, dirty: bool| {
        let folder = format!("{id}-folder");
        let file = format!("{id}-file");
        Breadcrumb::from_path(id, PATH, palette)
            .dirty(dirty)
            .segment_action(move |segment| Some(if segment == PATH { file.clone() } else { format!("{folder}={segment}") }))
            .width(crumb_width)
    };
    Element::column(vec![
        section(
            cx,
            "Buscar y reemplazar",
            vec![
                caption(cx, "La app busca; la barra muestra la consulta, las opciones y el recuento. «zzz» no encuentra nada."),
                row(widths),
                row(vec![find_bar(cx, width.min(cx.width))]),
                caption(cx, "Por debajo de 480, o con el reemplazo abierto, las opciones bajan a su propia fila."),
            ],
        ),
        section(
            cx,
            "Ruta de un archivo",
            vec![
                caption(cx, "Pasiva: sólo muestra dónde está el archivo."),
                Breadcrumb::from_path("editor-path", PATH, palette).width(crumb_width).render(),
                caption(cx, "Con cambios sin guardar y con las acciones que da la app a cada segmento."),
                linked("editor-path-dirty", true).render(),
                caption(cx, "Un símbolo del archivo, añadido tras la ruta."),
                linked("editor-path-symbol", false)
                    .item(BreadcrumbItem::new("render").action("editor-path-symbol-render"))
                    .render(),
            ],
        ),
        section(
            cx,
            "Compositor con franja superior",
            vec![
                caption(cx, "La franja lleva los metadatos del mensaje, unida al borde superior de la tarjeta."),
                row(vec![composer(cx, cx.width.min(480.0))]),
            ],
        ),
        section(
            cx,
            "Tinta de estado",
            vec![
                caption(cx, "Un marcador y un plegable con el color de fallo de la app en su texto secundario."),
                Marker::new("editor-marker", "Respuesta recibida", palette).icon(Icon::Check).render(),
                Marker::new("editor-marker-failed", "Solicitud al modelo fallida", palette)
                    .icon(Icon::Code)
                    .ink(DANGER)
                    .render(),
                Collapsible::new(
                    "editor-collapsible-failed",
                    "Exploración · 5",
                    caption(cx, "crates/limn/src/missing.rs: no existe"),
                    palette,
                )
                .icon(Icon::Search)
                .detail("3 lecturas · 2 fallidas")
                .ink(DANGER)
                .expanded(state.flag("editor-collapsible-failed"))
                .render(),
            ],
        ),
    ])
    .gap(28.0)
}
