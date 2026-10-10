use super::{caption, row, section, Cx};
use limn::{AlertColors, AlertTone, Component, Element, Icon, IconButton, Separator, Text, TextRole};

const ROLES: [(TextRole, &str); 8] = [
    (TextRole::Display, "Display"),
    (TextRole::Heading, "Heading"),
    (TextRole::Body, "Body"),
    (TextRole::Label, "Label"),
    (TextRole::Caption, "Caption"),
    (TextRole::Small, "Small"),
    (TextRole::Micro, "Micro"),
    (TextRole::Code, "Code"),
];

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(520.0);
    let long = "Un título bastante más largo que la columna que lo contiene, para ver cómo termina";
    let tool = |id: &str, icon: Icon| IconButton::new(id, icon, palette).render();
    let sample = |role: TextRole, name: &str| {
        Element::row(vec![
            Text::new(format!("{name} · {}", role.size()), palette)
                .muted()
                .render()
                .centered_y()
                .size(Some(96.0), None),
            Text::new("El zorro veloz salta sobre el perro", palette).role(role).render().centered_y(),
        ])
        .gap(12.0)
    };
    Element::column(vec![
        caption(cx, "Text toma el tamaño de un TextRole y el color de la paleta; no acepta píxeles."),
        section(cx, "Escala", ROLES.iter().map(|(role, name)| sample(*role, name)).collect()),
        section(cx, "Tinta y familia", vec![
            Text::eyebrow("EYEBROW", palette).render(),
            Text::new("Texto principal", palette).render(),
            Text::new("Texto secundario con muted()", palette).muted().render(),
            Text::new("Tinta semántica con ink()", palette).ink(AlertColors::for_tone(AlertTone::Error, palette).accent).render(),
            Text::new("cargo test -p limn", palette).mono().render(),
        ]),
        section(cx, "Texto largo", vec![
            Element::column(vec![
                caption(cx, "truncate() con tooltip()"),
                Text::new(long, palette).truncate().tooltip("typography-truncated").render(),
                caption(cx, "fade()"),
                Text::new(long, palette).fade(32.0).render(),
                caption(cx, "Sin nada: el párrafo se parte"),
                Text::new(long, palette).line_height(20.0).render(),
            ])
            .gap(6.0)
            .size(Some(width.min(280.0)), None),
        ]),
        section(cx, "Separador", vec![
            Element::column(vec![
                Text::new("Arriba", palette).render(),
                Separator::horizontal(palette).render(),
                Text::new("Abajo, con margen", palette).render(),
                Separator::horizontal(palette).margin(8.0).render(),
                Text::new("Largo fijo", palette).render(),
                Separator::horizontal(palette).length(96.0).render(),
                Text::new("Con ink()", palette).render(),
                Separator::horizontal(palette).ink(palette.muted).render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
            row(vec![
                tool("typography-back", Icon::ArrowLeft),
                tool("typography-forward", Icon::ArrowRight),
                Separator::vertical(palette).length(16.0).centered_in(28.0).margin(4.0).render(),
                tool("typography-refresh", Icon::Refresh),
                Separator::vertical(palette).render(),
                tool("typography-archive", Icon::Archive),
            ]),
            caption(cx, "Vertical: centrado en la barra con margen, y sin opciones (llena el alto)."),
        ]),
    ])
    .gap(32.0)
}
