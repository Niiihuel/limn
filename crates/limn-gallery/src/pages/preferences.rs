use super::Cx;
use limn::{
    ActionRow, ActionRowSizing, Button, ButtonVariant, Chip, Component, Element, Icon, PreferencesGroup, Switch,
};

/// A note's density: a smaller icon column and no minimum height.
const NOTE: ActionRowSizing = ActionRowSizing {
    icon_column: 16.0,
    copy_gap: 5.0,
    min_height: 0.0,
    min_height_with_subtitle: 0.0,
    ..ActionRowSizing::DEFAULT
};

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let toggle = |id: &str| Switch::new(id, state.flag(id), palette).render();
    let plugin = |id: &str, name: &str, skills: &str| {
        ActionRow::new(name, palette).subtitle(skills).trailing(toggle(id)).render()
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
                    .trailing(Chip::new("0 activas", palette).render())
                    .render(),
                ActionRow::new("Explorar y administrar", palette)
                    .subtitle("Busca skills, agrega fuentes y administra instalaciones.")
                    .trailing(
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
            "Valores, notas y filas apiladas",
            vec![
                ActionRow::new("Contexto de la conversación", palette)
                    .subtitle("Último porcentaje de contexto restante.")
                    .value("82% restante")
                    .render(),
                ActionRow::new("Solo CLIs iniciadas desde la aplicación", palette)
                    .subtitle("Cada sesión recibe un acceso propio que deja de valer al cerrarla. Una CLI abierta por fuera no puede conectarse.")
                    .icon(Icon::Terminal)
                    .icon_muted(true)
                    .align_top(true)
                    .wrap_title(true)
                    .sizing(NOTE)
                    .render(),
                ActionRow::new("Carpeta de trabajo de los agentes en segundo plano", palette)
                    .subtitle("Apilada: el control baja bajo el texto y el título se parte en vez de cortarse.")
                    .wrap_title(true)
                    .align_top(true)
                    .trailing(
                        Button::new("preferences-folder", "Elegir carpeta", palette)
                            .variant(ButtonVariant::Outline)
                            .render(),
                    )
                    .stacked(true)
                    .render(),
                ActionRow::new("Reducir movimiento", palette)
                    .subtitle("Apilada también, pero un interruptor se queda en su línea.")
                    .trailing(toggle("preferences-motion"))
                    .stacked(true)
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
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-account")
                    .render(),
                ActionRow::new("Atajos de teclado", palette)
                    .icon(Icon::Keyboard)
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-shortcuts")
                    .render(),
                ActionRow::new("Datos", palette)
                    .icon(Icon::Archive)
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-data")
                    .render(),
            ],
            palette,
        )
        .render(),
    ])
    .gap(28.0)
}
