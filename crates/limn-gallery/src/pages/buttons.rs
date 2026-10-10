use super::{caption, row, section, Cx};
use limn::{
    ActiveHover, Button, ButtonSize, ButtonVariant, Component, ControlSize, Element, Icon, IconButton,
    TooltipPlacement, TooltipSpec,
};

const VARIANTS: [(ButtonVariant, &str); 11] = [
    (ButtonVariant::Default, "Default"),
    (ButtonVariant::Secondary, "Secondary"),
    (ButtonVariant::Outline, "Outline"),
    (ButtonVariant::Ghost, "Ghost"),
    (ButtonVariant::Subtle, "Subtle"),
    (ButtonVariant::Raised, "Raised"),
    (ButtonVariant::Solid, "Solid"),
    (ButtonVariant::Link, "Link"),
    (ButtonVariant::Success, "Success"),
    (ButtonVariant::Warning, "Warning"),
    (ButtonVariant::Destructive, "Destructive"),
];

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let variants = |prefix: &str, build: fn(Button) -> Button| {
        // Two rows, so the page also fits a narrow window.
        Element::column(
            VARIANTS
                .chunks(6)
                .map(|chunk| {
                    row(chunk
                        .iter()
                        .map(|(variant, name)| {
                            build(Button::new(format!("{prefix}-{name}"), *name, palette).variant(*variant))
                                .render()
                        })
                        .collect())
                })
                .collect(),
        )
        .gap(12.0)
    };
    Element::column(vec![
        section(cx, "Variantes", vec![variants("button", |button| button)]),
        section(cx, "Deshabilitados", vec![variants("button-disabled", Button::disabled)]),
        section(cx, "Cargando", vec![variants("button-loading", |button| button.loading(true))]),
        section(cx, "Tamaños", vec![row(vec![
            Button::new("button-small", "Small", palette).button_size(ButtonSize::Small).render(),
            Button::new("button-default", "Default", palette).button_size(ButtonSize::Default).render(),
            Button::new("button-large", "Large", palette).button_size(ButtonSize::Large).render(),
            IconButton::new("button-large-icon", Icon::Add, palette)
                .button_size(ButtonSize::Large)
                .render(),
        ])]),
        section(cx, "Con icono", vec![row(vec![
            Button::new("button-icon", "Nuevo", palette).icon(Icon::Add).render(),
            Button::new("button-trailing", "Rama", palette)
                .variant(ButtonVariant::Outline)
                .icon(Icon::GitBranch)
                .trailing_icon(Icon::ChevronDown)
                .render(),
            IconButton::new("button-settings", Icon::Settings, palette).tooltip("Ajustes").render(),
            IconButton::new("button-trash", Icon::Trash, palette)
                .variant(ButtonVariant::Destructive)
                .tooltip("Eliminar")
                .render(),
        ])]),
        section(cx, "Barra de herramientas", vec![
            row(vec![
                IconButton::new("button-tool-dense", Icon::Add, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Dense)
                    .tooltip_with(
                        TooltipSpec::new("Nuevo")
                            .description("Crea un archivo en la carpeta abierta.")
                            .shortcut("Ctrl+N")
                            .placement(TooltipPlacement::Below),
                    )
                    .render(),
                IconButton::new("button-tool-minus", Icon::Minus, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .dispatch_as("button-tool-dense")
                    .tooltip_with(TooltipSpec::new("Quitar").placement(TooltipPlacement::Below))
                    .render(),
                IconButton::new("button-tool-disabled", Icon::Trash, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .disabled()
                    .tooltip_with(
                        TooltipSpec::new("Nada que eliminar").placement(TooltipPlacement::Below),
                    )
                    .render(),
                IconButton::new("button-tool-lift", Icon::PanelLeft.filled(), palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .active(true)
                    .tooltip("Activo: Lift")
                    .render(),
                IconButton::new("button-tool-palette", Icon::PanelRight.filled(), palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .active(true)
                    .active_hover(ActiveHover::Palette)
                    .tooltip("Activo: Palette")
                    .render(),
            ]),
            caption(cx, "Dense, Compact y Regular son cuadrados de 24, 28 y 32. El botón deshabilitado conserva su ayuda."),
        ]),
        section(cx, "Activo", vec![
            row(vec![Button::new("button-toggle", "Alternar", palette)
                .variant(ButtonVariant::Ghost)
                .active(cx.state.flag("button-toggle"))
                .render()]),
            caption(cx, "El botón conserva su relleno mientras el estado está activo."),
        ]),
    ])
    .gap(32.0)
}
