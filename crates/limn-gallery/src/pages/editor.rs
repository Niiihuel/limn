use super::{caption, row, section, Cx, Section};
use limn::{
    Breadcrumb, BreadcrumbItem, Button, ButtonVariant, Collapsible, Color, Component, Composer,
    ComponentPalette, ControlSize, DictationButton, DictationState, DictationWaveform, Element, FindBar, FindIds,
    FindLabels, FindOptions, Icon, IconButton, Marker,
};

const DANGER: Color = Color::rgb(229, 83, 75);
const WIDTHS: [&str; 3] = ["260", "480", "520"];
const PATH: &str = "crates/limn/src/find_bar.rs";
/// Two seconds of someone speaking, a level every 80 ms.
const SPEECH: [f32; 24] = [
    0.0, 0.05, 0.3, 0.55, 0.4, 0.7, 0.9, 0.6, 0.35, 0.1, 0.0, 0.0, 0.2, 0.5, 0.8, 1.0, 0.75, 0.45, 0.6, 0.3, 0.15, 0.4, 0.65, 0.35,
];

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
    // Each click moves the session on: idle, starting, listening,
    // transcribing, idle.
    let (state, next) = match cx.state.choice("editor-dictation", "idle") {
        "starting" => (DictationState::Starting, "listening"),
        "listening" => (DictationState::Listening, "transcribing"),
        "transcribing" => (DictationState::Transcribing, "idle"),
        _ => (DictationState::Idle, "starting"),
    };
    // While the microphone is open its level stands beside it; before and after, it is silence.
    let levels: &[f32] = if state == DictationState::Listening { &SPEECH } else { &[] };
    let mut trailing = vec![
        DictationButton::new(format!("editor-dictation={next}"), state, palette)
            .size(ControlSize::Compact)
            .render(),
        tool("editor-composer-send", Icon::Send),
    ];
    if state != DictationState::Idle {
        trailing.insert(0, DictationWaveform::new("editor-composer-levels", levels, palette).size(ControlSize::Compact).width(48.0).render());
    }
    let trailing = Element::row(trailing).gap(4.0);
    Composer::new("editor-composer", cx.state.text("editor-composer"), "Describe una tarea", width, palette)
        .pill_controls(tool("editor-composer-add", Icon::Add), trailing)
        .auto_grow(28.0, 120.0)
        .header_strip(strip, ComponentPalette { surface: palette.raised, ..palette })
        .render()
}

pub fn view(cx: &Cx) -> Vec<Section> {
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
    vec![
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
                caption(cx, "El micrófono del compositor avanza con cada clic: en reposo, iniciando, escuchando, transcribiendo."),
                row(vec![
                    DictationButton::new("editor-dictation-idle", DictationState::Idle, palette).render(),
                    DictationButton::new("editor-dictation-starting", DictationState::Starting, palette).render(),
                    DictationButton::new("editor-dictation-listening", DictationState::Listening, palette).render(),
                    DictationButton::new("editor-dictation-accent", DictationState::Listening, palette)
                        .live_palette(ComponentPalette { surface: DANGER, text: Color::WHITE, ..palette })
                        .render(),
                    DictationButton::new("editor-dictation-off", DictationState::Listening, palette)
                        .enabled(false)
                        .render(),
                ]),
                caption(cx, "DictationButton: reposo, iniciando, escuchando, escuchando con la paleta de la app, y sin dictado disponible."),
                row(vec![
                    DictationWaveform::new(
                        "editor-waveform",
                        match cx.state.choice("editor-levels", "silence") {
                            "speech" => &SPEECH,
                            "loud" => &[1.0; 24],
                            _ => &[],
                        },
                        palette,
                    )
                    .render(),
                    Button::new("editor-levels=silence", "Silencio", palette).variant(ButtonVariant::Outline).render(),
                    Button::new("editor-levels=speech", "Voz", palette).variant(ButtonVariant::Outline).render(),
                    Button::new("editor-levels=loud", "Tope", palette).variant(ButtonVariant::Outline).render(),
                ]),
                caption(cx, "DictationWaveform: el nivel del micrófono, lo más nuevo al final. El silencio son puntos, y cada barra va a su nivel con un resorte."),
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
    ]
}
