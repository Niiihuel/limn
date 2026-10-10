use super::Cx;
use lienzo::{
    ActionRow, Button, ButtonVariant, Chip, Component, Element, Icon, PreferencesGroup, Switch,
};

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let toggle = |id: &str| Switch::new(id, state.flag(id), palette).render();
    let plugin = |id: &str, name: &str, skills: &str| {
        ActionRow::new(name, palette).subtitle(skills).suffix(toggle(id)).render()
    };
    Element::column(vec![
        PreferencesGroup::new(
            "Plugins",
            vec![
                ActionRow::new("Skills de plugins", palette)
                    .icon(Icon::InfoCircle)
                    .subtitle("Al activar un plugin, el agente recibe el nombre y la descripción de cada skill y puede leer sus archivos cuando la tarea lo pida.")
                    .render(),
                plugin("preferences-agent-skills", "agent-skills", "25 skills"),
                plugin("preferences-ponytail", "ponytail", "6 skills"),
                plugin("preferences-vercel", "vercel", "37 skills"),
            ],
            palette,
        )
        .description("Instrucciones instaladas para el agente.")
        .render(),
        PreferencesGroup::new(
            "Marketplace",
            vec![
                ActionRow::new("Skills instaladas", palette)
                    .subtitle("Instrucciones disponibles por usuario o por proyecto.")
                    .suffix(Chip::new("0 activas", palette).render())
                    .render(),
                ActionRow::new("Explorar y administrar", palette)
                    .subtitle("Busca skills, agrega fuentes y administra instalaciones.")
                    .suffix(
                        Button::new("preferences-marketplace", "Abrir Marketplace", palette)
                            .variant(ButtonVariant::Outline)
                            .icon(Icon::ArrowRight)
                            .render(),
                    )
                    .render(),
            ],
            palette,
        )
        .render(),
        PreferencesGroup::new(
            "Filas que son acciones",
            vec![
                ActionRow::new("Cuenta", palette)
                    .icon(Icon::Users)
                    .subtitle("Sesión, plan y facturación.")
                    .suffix(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-account")
                    .render(),
                ActionRow::new("Atajos de teclado", palette)
                    .icon(Icon::Keyboard)
                    .suffix(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-shortcuts")
                    .render(),
                ActionRow::new("Datos", palette)
                    .icon(Icon::Archive)
                    .suffix(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-data")
                    .render(),
            ],
            palette,
        )
        .render(),
    ])
    .gap(28.0)
}
