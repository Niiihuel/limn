use super::{section, Cx};
use limn::{
    AlertColors, AlertTone, Button, ButtonSize, ButtonVariant, Component, ComponentPalette, Element, Empty,
    Icon, Notice, NoticeSizing,
};

pub fn view(cx: &Cx) -> Element {
    let width = cx.width.min(720.0);
    // A framed notice takes its fill from the palette it is given.
    let raised = ComponentPalette { surface: cx.palette.raised, ..cx.palette };
    let bar = |status: &str, failed: bool| {
        let notice = Notice::new(status, raised)
            .trailing(
                Button::new(format!("notice-restart-{failed}"), "Reiniciar", cx.palette)
                    .variant(ButtonVariant::Ghost)
                    .render(),
            )
            .framed()
            .sizing(BAR)
            .width(width.min(420.0));
        if failed {
            notice.ink(AlertColors::for_tone(AlertTone::Error, cx.palette).accent).render()
        } else {
            notice.render()
        }
    };
    Element::column(vec![
        section(cx, "Con icono y acción", vec![
            Empty::new(
                "Bienvenido a la wiki",
                "Las wikis ordenan la hoja de ruta del proyecto, muestran su estado y documentan el software.",
                cx.palette,
            )
            .icon(Icon::Book)
            .action("empty-create", "Crear la primera página", ButtonVariant::Default)
            .width(width)
            .render(),
        ]),
        section(cx, "Dos acciones", vec![
            Empty::new("Sin proyectos", "Crea un proyecto o importa uno existente.", cx.palette)
                .icon(Icon::Folder)
                .action("empty-new", "Crear proyecto", ButtonVariant::Default)
                .action("empty-import", "Importar", ButtonVariant::Outline)
                .width(width)
                .render(),
        ]),
        section(cx, "Compacto", vec![
            Empty::new("Sin resultados", "Prueba otro término.", cx.palette)
                .icon(Icon::Search)
                .compact()
                .width(width.min(360.0))
                .render(),
        ]),
        section(cx, "Enmarcado", vec![
            Empty::new("Todavía no hay plugins", "Elige una carpeta de plugin para instalarla.", cx.palette)
                .icon(Icon::Puzzle)
                .compact()
                .framed()
                .width(width)
                .render(),
        ]),
        section(cx, "Sólo título", vec![
            Empty::new("Nada por aquí", "", cx.palette).width(width.min(360.0)).render(),
        ]),
        section(cx, "Aviso en línea", vec![
            Notice::new("Todavía no instalaste skills", raised)
                .icon(Icon::Grid)
                .description("Elige una del catálogo y dónde instalarla.")
                .action(Element::row(vec![
                    Button::new("notice-explore", "Explorar catálogo", cx.palette)
                        .variant(ButtonVariant::Outline)
                        .button_size(ButtonSize::Small)
                        .render(),
                    Element::spacer().flex(1.0),
                ]))
                .framed()
                .width(width)
                .render(),
            Notice::new("Sin resultados", cx.palette)
                .icon(Icon::Search)
                .description("Prueba otra búsqueda o cambia el filtro.")
                .width(width)
                .render(),
        ]),
        section(cx, "Aviso apilado", vec![
            Notice::new("Tu agente, junto al código", cx.palette)
                .icon(Icon::Chat)
                .description("Explora el proyecto, pide cambios o resuelve una tarea.")
                .stacked()
                .width(width.min(360.0))
                .render(),
        ]),
        section(cx, "Aviso compacto", vec![
            Notice::new("Sin problemas en este archivo", cx.palette)
                .icon(Icon::CheckCircle)
                .description("Los diagnósticos están actualizados.")
                .sizing(NoticeSizing::COMPACT)
                .width(width.min(360.0))
                .render(),
            Notice::new("Carpeta vacía", cx.palette)
                .description("Abre un proyecto para ver sus archivos. El texto se parte en varias líneas cuando el panel es estrecho.")
                .action(
                    Button::new("notice-reload", "Volver a cargar", cx.palette)
                        .variant(ButtonVariant::Ghost)
                        .button_size(ButtonSize::Small)
                        .render(),
                )
                .sizing(NoticeSizing::COMPACT)
                .width(width.min(240.0))
                .render(),
        ]),
        section(cx, "Aviso de alto fijo", vec![
            bar("El proceso terminó con un error que no cabe en una sola línea y se corta con puntos suspensivos", true),
            bar("Proceso finalizado · código 0", false),
        ]),
    ])
    .gap(32.0)
}

/// A one-line bar: the title cut and centered beside its action.
const BAR: NoticeSizing = NoticeSizing {
    padding: 4.0,
    height: Some(40.0),
    title_height: Some(32.0),
    ..NoticeSizing::COMPACT
};
