use super::{caption, row, section, Cx, Section};
use limn::{
    Alert, AlertTone, Button, ButtonVariant, Component, Element, Icon, SegmentedControl,
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

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let tokens = limn::tokens();
    let width = cx.width.min(520.0);
    vec![
        caption(cx, "One call to set_tokens changes the shape and pace of the whole library.").into(),
        section(cx, "Corners", vec![
            row(vec![choice(cx, "tokens-radius", &[("square", "Square"), ("default", "Default"), ("round", "Round")])]),
            caption(cx, &format!(
                "control {} · surface {} · card {} · panel {}",
                tokens.radius.control, tokens.radius.surface, tokens.radius.card, tokens.radius.panel
            )),
        ]),
        section(cx, "Motion", vec![
            row(vec![choice(cx, "tokens-motion", &[("fast", "Fast"), ("default", "Normal"), ("slow", "Slow")])]),
            caption(cx, &format!(
                "hover {} ms · enter {} ms · exit {} ms · move {} ms",
                tokens.motion.hover_ms, tokens.motion.enter_ms, tokens.motion.exit_ms, tokens.motion.move_ms
            )),
            row(vec![
                Switch::new("tokens-reduced", cx.state.flag("tokens-reduced"), palette).render(),
                caption(cx, "No animations (set_reduced_motion)"),
            ]),
        ]),
        section(cx, "Sample", vec![
            row(vec![
                Button::new("tokens-primary", "Save", palette).icon(Icon::Check).render(),
                Button::new("tokens-outline", "Cancel", palette).variant(ButtonVariant::Outline).render(),
                TextField::new("tokens-field", cx.state.text("tokens-field"), "Name", palette)
                    .width(180.0)
                    .render(),
            ]),
            Element::panel(
                Alert::new("Card", "Alerts use the card radius.", palette)
                    .tone(AlertTone::Info)
                    .render(),
            )
            .size(Some(width), None),
            Element::panel(caption(cx, "Panel: dialogs and previews use the panel radius."))
                .padding(32.0)
                .background(palette.surface)
                .border(palette.border, 1.0)
                .radius(tokens.radius.panel)
                .size(Some(width), None),
        ]),
    ]
}
