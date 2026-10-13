use super::{caption, row, section, Cx, Section};
use limn::{
    Alert, AlertTone, Avatar, Chip, Collapse, Component, Element, FontFamily, Icon, Kbd, Button, ButtonVariant, Marker, MarkerVariant, Progress, Skeleton, SkeletonGroup,
    Spinner, SpinnerVariant, Ticker,
};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let alert = |tone: AlertTone, title: &str, description: &str| {
        Element::panel(Alert::new(title, description, palette).tone(tone).render()).size(Some(width), None)
    };
    let percent: u32 = cx.state.choice("feedback-progress", "66").parse().unwrap_or(66);
    let progress = percent as f32 / 100.0;
    const STEPS: [&str; 4] = [
        "Buscando **/*.{md,txt} en presupuestacion",
        "Leyendo README.md",
        "Leyendo un-archivo-con-un-nombre-muy-largo-que-no-entra-en-el-ancho-disponible.md",
        "Listando src/componentes",
    ];
    let step: usize = cx.state.choice("feedback-ticker", "0").parse().unwrap_or(0) % STEPS.len();
    let collapse_open = cx.state.choice("feedback-collapse", "open") == "open";
    vec![
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
            Progress::new(progress, palette).width(width.min(336.0)).flat().render(),
            caption(cx, "Plana: extremos rectos, para el medidor de una lista densa."),
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
            Element::panel(
                SkeletonGroup::rows("feedback-skeleton-rows", palette, 4).icon(true).render(),
            )
            .size(Some(width.min(336.0)), None),
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
        section(cx, "Teletipo", vec![
            Element::panel(
                Ticker::new(
                    "feedback-ticker-line",
                    STEPS[step],
                    Element::truncated_text(STEPS[step])
                        .foreground(palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(13.0)
                        .size(None, Some(20.0)),
                )
                .height(40.0)
                .render(),
            )
            .size(Some(width.min(336.0)), None),
            row(vec![Button::new(format!("feedback-ticker={}", (step + 1) % STEPS.len()), "Siguiente paso", palette)
                .variant(ButtonVariant::Outline)
                .render()]),
            caption(cx, "El hueco mide siempre dos líneas: lo de abajo no se mueve."),
        ]),
        section(cx, "Hueco que se cierra", vec![
            // No gap around the slot: a gap would vanish in one frame at the end.
            Element::column(vec![
                Element::text("Explorando el proyecto").foreground(palette.text).font_family(FontFamily::Sans).font_size(13.0),
                Collapse::new(
                    "feedback-collapse-slot",
                    collapse_open.then(|| {
                        Element::text("Leyendo README.md\nListando src/componentes")
                            .foreground(palette.muted)
                            .font_family(FontFamily::Sans).font_size(13.0)
                            .padding(6.0)
                    }),
                )
                .expand()
                .render(),
                Element::text("Lo que sigue acompaña al hueco.").foreground(palette.text).font_family(FontFamily::Sans).font_size(13.0),
            ])
            .size(Some(width.min(336.0)), None),
            row(vec![Button::new(
                format!("feedback-collapse={}", if collapse_open { "closed" } else { "open" }),
                if collapse_open { "Quitar el contenido" } else { "Mostrar el contenido" },
                palette,
            )
            .variant(ButtonVariant::Outline)
            .render()]),
            caption(cx, "Al quitarlo, su última imagen se desvanece mientras el hueco se cierra."),
        ]),
        section(cx, "Etiquetas", vec![row(vec![
            Chip::new("rust", palette).render(),
            Chip::new("borrador", palette).render(),
            Chip::new("Plan Pro", palette).outline().render(),
            Avatar::new("Ada Lovelace", palette).size(32.0).render(),
            Kbd::new("Ctrl+Shift+P", palette).render(),
        ])]),
    ]
}
