use super::{section, Cx};
use lienzo::{ButtonVariant, Component, Element, Empty, Icon};

pub fn view(cx: &Cx) -> Element {
    let width = cx.width.min(720.0);
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
        section(cx, "Sólo título", vec![
            Empty::new("Nada por aquí", "", cx.palette).width(width.min(360.0)).render(),
        ]),
    ])
    .gap(32.0)
}
