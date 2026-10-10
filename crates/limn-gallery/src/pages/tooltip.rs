use super::{caption, row, section, Cx};
use limn::{Button, ButtonVariant, Component, Element, Icon, IconButton, Tooltip, TooltipPlacement};

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let anchor = |id: &str, label: &str| {
        Button::new(id, label, palette).variant(ButtonVariant::Outline).render()
    };
    Element::column(vec![
        caption(cx, "Aparecen tras 350 ms de puntero quieto, o al instante con el foco del teclado."),
        section(cx, "Título y atajo", vec![row(vec![
            Button::new("tooltip-plain", "Sólo título", palette)
                .variant(ButtonVariant::Outline)
                .tooltip("Guardar cambios")
                .render(),
            Button::new("tooltip-shortcut", "Con atajo", palette)
                .variant(ButtonVariant::Outline)
                .tooltip("Guardar cambios")
                .tooltip_shortcut("Ctrl+S")
                .render(),
            IconButton::new("tooltip-icon", Icon::Refresh, palette)
                .tooltip("Actualizar")
                .tooltip_shortcut("F5")
                .render(),
        ])]),
        section(cx, "Con descripción", vec![row(vec![
            Tooltip::new(anchor("tooltip-rich", "Descripción"), "tooltip-rich", "Modo plan", palette)
                .shortcut("Shift+Tab")
                .description("El agente propone los pasos y espera tu aprobación antes de editar archivos.")
                .separator()
                .footer("Disponible en cualquier conversación")
                .render(),
        ])]),
        section(cx, "Posición", vec![row(vec![
            Tooltip::new(anchor("tooltip-above", "Arriba"), "tooltip-above", "Arriba", palette)
                .placement(TooltipPlacement::Above)
                .render(),
            Tooltip::new(anchor("tooltip-below", "Abajo"), "tooltip-below", "Abajo", palette)
                .placement(TooltipPlacement::Below)
                .render(),
            Tooltip::new(anchor("tooltip-left", "Izquierda"), "tooltip-left", "Izquierda", palette)
                .placement(TooltipPlacement::Left)
                .render(),
            Tooltip::new(anchor("tooltip-right", "Derecha"), "tooltip-right", "Derecha", palette)
                .placement(TooltipPlacement::Right)
                .render(),
        ])]),
    ])
    .gap(32.0)
}
