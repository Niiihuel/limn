use super::{caption, row, section, Cx, Section};
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

pub fn view(cx: &Cx) -> Vec<Section> {
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
    vec![
        section(
            cx,
            "Escala",
            std::iter::once(caption(cx, "Text toma el tamaño de un TextRole y el color de la paleta."))
                .chain(ROLES.iter().map(|(role, name)| sample(*role, name)))
                .collect(),
        ),
        section(cx, "Fuera de la escala", vec![
            caption(cx, ".size(px) para un paso que el diseño del host tiene entre dos roles"),
            Text::new("Small · 11", palette).role(TextRole::Small).muted().render(),
            Text::new(".size(11.5): entre Small y Caption", palette).size(11.5).muted().render(),
            Text::new("Caption · 12", palette).muted().render(),
            Text::new("   42", palette).mono().size(10.5).muted().render(),
        ]),
        section(cx, "Tinta y familia", vec![
            Text::eyebrow("EYEBROW", palette).render(),
            Text::new("Texto principal", palette).render(),
            Text::new("Texto secundario con muted()", palette).muted().render(),
            Text::new("Tinta semántica con ink()", palette).ink(AlertColors::for_tone(AlertTone::Error, palette).accent).render(),
            Text::new("cargo test -p limn", palette).mono().render(),
        ]),
        section(cx, "Pesos", [
            ("Regular", limn::FontFamily::Sans),
            ("Medium", limn::FontFamily::SansMedium),
            ("Bold", limn::FontFamily::SansBold),
        ].into_iter().map(|(name, family)| {
            Element::text(format!("{name} · 175.6K de 200K tokens"))
                .font_family(family)
                .font_size(15.0)
                .foreground(palette.text)
        }).collect()),
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
        section(cx, "Fuera de foco", vec![
            Element::column(
                [0.0, 1.0, 2.0, 3.5]
                    .into_iter()
                    .enumerate()
                    .map(|(index, radius)| Text::new("La línea que todavía no se canta", palette).size(20.0).render().soft_focus(format!("typography-soft:{index}"), radius))
                    .collect(),
            )
            .gap(8.0),
            caption(cx, "soft_focus: nítido, y a 1, 2 y 3,5 de radio. El radio va al nuevo con un resorte."),
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
    ]
}
