use super::{caption, row, section, Cx};
use lienzo::{
    Alert, AlertTone, Button, ButtonVariant, Component, Element, Empty, Icon, SegmentedControl,
    Switch, Tab, TextField,
};

/// A segmented choice stored under `group`.
fn choice(cx: &Cx, group: &str, options: &[(&str, &str)]) -> Element {
    let chosen = cx.state.choice(group, "default");
    SegmentedControl::new(
        options
            .iter()
            .map(|(id, label)| Tab::new(format!("{group}={id}"), *label, 110.0).active(*id == chosen))
            .collect(),
        cx.palette,
    )
    .render()
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let tokens = lienzo::tokens();
    let width = cx.width.min(520.0);
    Element::column(vec![
        caption(cx, "Una llamada a set_tokens cambia la forma y el ritmo de toda la biblioteca."),
        section(cx, "Esquinas", vec![
            row(vec![choice(cx, "tokens-radius", &[("square", "Rectas"), ("default", "Normales"), ("round", "Redondas")])]),
            caption(cx, &format!(
                "control {} · superficie {} · tarjeta {} · panel {}",
                tokens.radius.control, tokens.radius.surface, tokens.radius.card, tokens.radius.panel
            )),
        ]),
        section(cx, "Ritmo", vec![
            row(vec![choice(cx, "tokens-motion", &[("fast", "Rápido"), ("default", "Normal"), ("slow", "Lento")])]),
            caption(cx, &format!(
                "hover {} ms · entrada {} ms · salida {} ms · movimiento {} ms",
                tokens.motion.hover_ms, tokens.motion.enter_ms, tokens.motion.exit_ms, tokens.motion.move_ms
            )),
            row(vec![
                Switch::new("tokens-reduced", cx.state.flag("tokens-reduced"), palette).render(),
                caption(cx, "Sin animaciones (set_reduced_motion)"),
            ]),
        ]),
        section(cx, "Muestra", vec![
            row(vec![
                Button::new("tokens-primary", "Guardar", palette).icon(Icon::Check).render(),
                Button::new("tokens-outline", "Cancelar", palette).variant(ButtonVariant::Outline).render(),
                TextField::new("tokens-field", cx.state.text("tokens-field"), "Nombre", palette)
                    .width(180.0)
                    .render(),
            ]),
            Element::panel(
                Alert::new("Tarjeta", "Los avisos usan el radio de tarjeta.", palette)
                    .tone(AlertTone::Info)
                    .render(),
            )
            .size(Some(width), None),
            Empty::new("Panel", "Los estados vacíos y los diálogos usan el radio de panel.", palette)
                .icon(Icon::Grid)
                .compact()
                .width(width)
                .render(),
        ]),
    ])
    .gap(32.0)
}
