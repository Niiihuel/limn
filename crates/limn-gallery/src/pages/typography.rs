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
    let long = "A title much longer than the column holding it, to show how it ends";
    let tool = |id: &str, icon: Icon| IconButton::new(id, icon, palette).render();
    let sample = |role: TextRole, name: &str| {
        Element::row(vec![
            Text::new(format!("{name} · {}", role.size()), palette)
                .muted()
                .render()
                .centered_y()
                .size(Some(96.0), None),
            Text::new("The quick brown fox jumps over the lazy dog", palette).role(role).render().centered_y(),
        ])
        .gap(12.0)
    };
    vec![
        section(
            cx,
            "Scale",
            std::iter::once(caption(cx, "Text takes its size from a TextRole and its color from the palette."))
                .chain(ROLES.iter().map(|(role, name)| sample(*role, name)))
                .collect(),
        ),
        section(cx, "Outside the scale", vec![
            caption(cx, ".font_size(px) for a size the host's design places between two roles"),
            Text::new("Small · 11", palette).role(TextRole::Small).muted().render(),
            Text::new(".font_size(11.5): between Small and Caption", palette).font_size(11.5).muted().render(),
            Text::new("Caption · 12", palette).muted().render(),
            Text::new("   42", palette).mono().font_size(10.5).muted().render(),
        ]),
        section(cx, "Ink and family", vec![
            Text::eyebrow("EYEBROW", palette).render(),
            Text::new("Primary text", palette).render(),
            Text::new("Secondary text with muted()", palette).muted().render(),
            Text::new("Semantic ink with ink()", palette).ink(AlertColors::for_tone(AlertTone::Error, palette).accent).render(),
            Text::new("cargo test -p limn", palette).mono().render(),
        ]),
        section(cx, "Weights", [
            ("Regular", limn::FontFamily::Sans),
            ("Medium", limn::FontFamily::SansMedium),
            ("Bold", limn::FontFamily::SansBold),
        ].into_iter().map(|(name, family)| {
            Element::text(format!("{name} · 175.6K of 200K tokens"))
                .font_family(family)
                .font_size(15.0)
                .foreground(palette.text)
        }).collect()),
        section(cx, "Long text", vec![
            Element::column(vec![
                caption(cx, "truncate() with tooltip()"),
                Text::new(long, palette).truncate().tooltip("typography-truncated").render(),
                caption(cx, "fade()"),
                Text::new(long, palette).fade(32.0).render(),
                caption(cx, "No options: the paragraph wraps"),
                Text::new(long, palette).line_height(20.0).render(),
            ])
            .gap(6.0)
            .size(Some(width.min(280.0)), None),
        ]),
        section(cx, "Soft focus", vec![
            Element::column(
                [0.0, 1.0, 2.0, 3.5]
                    .into_iter()
                    .enumerate()
                    .map(|(index, radius)| Text::new("The line that has not been sung yet", palette).font_size(20.0).render().soft_focus(format!("typography-soft:{index}"), radius))
                    .collect(),
            )
            .gap(8.0),
            caption(cx, "soft_focus: sharp, then at radii of 1, 2 and 3.5. The radius springs to its new value."),
        ]),
        section(cx, "Separator", vec![
            Element::column(vec![
                Text::new("Above", palette).render(),
                Separator::horizontal(palette).render(),
                Text::new("Below, with margin", palette).render(),
                Separator::horizontal(palette).margin(8.0).render(),
                Text::new("Fixed length", palette).render(),
                Separator::horizontal(palette).length(96.0).render(),
                Text::new("With ink()", palette).render(),
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
            caption(cx, "Vertical: centered in the toolbar with margin, and with no options (fills the height)."),
        ]),
    ]
}
