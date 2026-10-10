use super::{row, section, Cx};
use limn::{
    Alert, AlertTone, Avatar, Chip, Component, Element, Icon, Kbd, Button, ButtonVariant, Marker, MarkerVariant, Progress, Skeleton,
    Spinner, SpinnerVariant,
};

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let alert = |tone: AlertTone, title: &str, description: &str| {
        Element::panel(Alert::new(title, description, palette).tone(tone).render()).size(Some(width), None)
    };
    let percent: u32 = cx.state.choice("feedback-progress", "66").parse().unwrap_or(66);
    let progress = percent as f32 / 100.0;
    Element::column(vec![
        section(cx, "Avisos", vec![
            alert(AlertTone::Neutral, "Nota", "Un aviso sin urgencia."),
            alert(AlertTone::Info, "Información", "Hay una versión nueva disponible."),
            alert(AlertTone::Success, "Listo", "Los cambios se guardaron."),
            alert(AlertTone::Warning, "Atención", "Quedan cambios sin guardar."),
            alert(AlertTone::Error, "Pago rechazado", "No se pudo procesar el pago. Revisa el método de pago e inténtalo de nuevo."),
        ]),
        section(cx, "Avisos con acción", vec![
            Element::panel(
                Alert::new("El modo oscuro ya está disponible", "Actívalo desde la configuración del perfil.", palette)
                    .action(Button::new("feedback-alert-enable", "Activar", palette).variant(ButtonVariant::Outline).render())
                    .render(),
            )
            .size(Some(width), None),
            if cx.state.flag("feedback-alert-dismiss") {
                Element::row(vec![
                    Button::new("feedback-alert-dismiss", "Volver a mostrar", palette)
                        .variant(ButtonVariant::Outline)
                        .render(),
                    Element::spacer().flex(1.0),
                ])
            } else {
                Element::panel(
                    Alert::new("La suscripción vence en 3 días", "Renuévala para no perder el acceso.", palette)
                        .tone(AlertTone::Warning)
                        .dismiss("feedback-alert-dismiss")
                        .render(),
                )
                .size(Some(width), None)
            },
        ]),
        section(cx, "Progreso", vec![
            Progress::new(progress, palette).width(width.min(336.0)).render(),
            row(vec![
                Button::new(format!("feedback-progress={}", (percent + 85) % 100), "Retroceder", palette)
                    .variant(ButtonVariant::Outline)
                    .render(),
                Button::new(format!("feedback-progress={}", (percent + 15).min(100)), "Avanzar", palette)
                    .variant(ButtonVariant::Outline)
                    .render(),
            ]),
            Progress::new(1.0, palette).width(width.min(336.0)).height(8.0).render(),
        ]),
        section(cx, "Carga", vec![
            row(vec![
                Spinner::new("feedback-spinner", palette.text).size(24.0).render(),
                Spinner::new("feedback-spokes", palette.text)
                    .size(24.0)
                    .variant(SpinnerVariant::Spokes)
                    .render(),
            ]),
            row(vec![
                Skeleton::circle("feedback-skeleton-avatar", palette, 40.0).render(),
                Element::column(vec![
                    Skeleton::line("feedback-skeleton-title", palette, 180.0).render(),
                    Skeleton::line("feedback-skeleton-text", palette, 260.0).render(),
                ])
                .gap(8.0),
            ]),
        ]),
        section(cx, "Marcadores", vec![
            Marker::new("feedback-marker", "Rama creada", palette).icon(Icon::GitBranch).render(),
            Marker::new("feedback-marker-loading", "Ejecutando pruebas", palette).loading(true).render(),
            Marker::new("feedback-marker-thinking", "Pensando", palette).thinking(true).render(),
            Element::panel(
                Marker::new("feedback-marker-separator", "Hoy", palette)
                    .variant(MarkerVariant::Separator)
                    .render(),
            )
            .size(Some(width), None),
        ]),
        section(cx, "Etiquetas", vec![row(vec![
            Chip::new("rust", palette).render(),
            Chip::new("borrador", palette).render(),
            Avatar::new("Ada Lovelace", palette).size(32.0).render(),
            Kbd::new("Ctrl+Shift+P", palette).render(),
        ])]),
    ])
    .gap(32.0)
}
