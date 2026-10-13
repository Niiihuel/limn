use super::{caption, row, section, Cx, Section};
use limn::{
    Button, ButtonVariant, CoachAlign, CoachMark, CoachSide, Color, Component, Element, Icon, Switch,
    TextField,
};

/// Target, title, body and placement of each step.
const STEPS: [(&str, &str, &str, CoachSide, CoachAlign); 3] = [
    (
        "tour-new",
        "Empieza por aquí",
        "Cada tarea es una conversación con un agente sobre tu proyecto.",
        CoachSide::Below,
        CoachAlign::Start,
    ),
    (
        "tour-search",
        "Encuentra lo que buscas",
        "Escribe parte del nombre de una tarea o de un archivo para filtrar la lista.",
        CoachSide::Below,
        CoachAlign::Center,
    ),
    (
        "tour-notify",
        "Entérate cuando termine",
        "Con los avisos activados no hace falta quedarse mirando: te avisamos cuando el agente te necesite.",
        CoachSide::Left,
        CoachAlign::Center,
    ),
];

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let toolbar = Element::row(vec![
        Button::new("tour-new", "Nueva tarea", palette).icon(Icon::Add).render(),
        TextField::new("tour-search", state.text("tour-search"), "Buscar", palette)
            .render()
            .size(Some(220.0), None),
        Element::spacer().flex(1.0),
        caption(cx, "Avisos"),
        Switch::new("tour-notify", state.flag("tour-notify"), palette).render(),
    ])
    .gap(12.0);
    let option = |id: &str, label: &str| {
        Element::row(vec![Switch::new(id, state.flag(id), palette).render(), caption(cx, label)]).gap(8.0)
    };
    vec![
        section(
            cx,
            "Recorrido de tres pasos",
            vec![
                toolbar,
                row(vec![Button::new("tour-step=0", "Iniciar recorrido", palette)
                    .variant(ButtonVariant::Outline)
                    .render()]),
                caption(
                    cx,
                    "Flechas izquierda y derecha o Enter para avanzar, Esc para cerrar. \
                     El resto de la ventana no recibe clics mientras dura.",
                ),
            ],
        ),
        section(
            cx,
            "Variantes",
            vec![
                option("tour-accent", "Color de acento de la aplicación"),
                option("tour-dim", "Atenuar la ventana alrededor del control"),
            ],
        ),
    ]
}

/// Wrap the window in the step the page is on.
pub fn layer(cx: &Cx, base: Element) -> Element {
    let state = cx.state;
    let Some(index) = state.text("tour-step").parse::<usize>().ok().filter(|index| *index < STEPS.len())
    else {
        return base;
    };
    let (target, title, body, side, align) = STEPS[index];
    let mut mark = CoachMark::new(base, "tour-step=closed", target, cx.palette)
        .title(title)
        .body(body)
        .placement(side, align)
        .step(index, STEPS.len())
        .skip("Omitir recorrido")
        .on_skip("tour-step=skipped")
        .on_next(format!("tour-step={}", index + 1))
        .on_back(format!("tour-step={}", index.saturating_sub(1)));
    if state.flag("tour-accent") {
        mark = mark.accent(Color::rgb(9, 105, 218), Color::WHITE);
    }
    if state.flag("tour-dim") {
        mark = mark.dim(Color::rgba(0, 0, 0, 110));
    }
    mark.render()
}
