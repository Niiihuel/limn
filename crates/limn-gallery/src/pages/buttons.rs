use super::{caption, row, section, Cx};
use limn::{
    ActiveHover, Button, ButtonGroup, ButtonGroupSizing, ButtonSize, ButtonSizing, ButtonVariant, Color,
    Component, ControlSize, Element, Icon, IconButton, RadiusRole, TooltipPlacement, TooltipSpec,
};

/// A host's own densities: a navigation button with a larger label, and a
/// close button for a 20 px tab.
const NAVIGATION: ButtonSizing = ButtonSizing {
    height: 32.0,
    font_size: 14.0,
    icon_size: 16.0,
    horizontal_padding: 5.0,
    inner_spacing: 6.0,
};
const TAB_CLOSE: ButtonSizing = ButtonSizing {
    height: 20.0,
    horizontal_padding: 2.0,
    ..NAVIGATION
};
const WIDE_GROUP: ButtonGroupSizing = ButtonGroupSizing {
    gap: 12.0,
    stacked_gap: 10.0,
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

// A status ink is the host's to choose; the gallery has no theme of its own.
const DANGER: Color = Color::rgb(255, 92, 92);

fn group(prefix: &str, palette: limn::ComponentPalette) -> Vec<Element> {
    vec![
        Button::new(format!("{prefix}-cancel"), "Cancelar", palette)
            .variant(ButtonVariant::Outline)
            .render(),
        Button::new(format!("{prefix}-save"), "Guardar", palette).render(),
    ]
}

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
            Button::new("button-mini", "Más", palette)
                .variant(ButtonVariant::Ghost)
                .button_size(ButtonSize::Mini)
                .render(),
            Button::new("button-small", "Small", palette).button_size(ButtonSize::Small).render(),
            Button::new("button-default", "Default", palette).button_size(ButtonSize::Default).render(),
            Button::new("button-large", "Large", palette).button_size(ButtonSize::Large).render(),
            IconButton::new("button-large-icon", Icon::Add, palette)
                .button_size(ButtonSize::Large)
                .render(),
        ])]),
        section(cx, "Perfil de la aplicación", vec![
            row(vec![
                Button::new("button-sizing", "Panel", palette)
                    .variant(ButtonVariant::Subtle)
                    .icon(Icon::PanelRightOpen)
                    .sizing(NAVIGATION)
                    .radius(RadiusRole::Card)
                    .render(),
                Button::new("button-sizing-disabled", "Panel", palette)
                    .variant(ButtonVariant::Subtle)
                    .icon(Icon::PanelRightOpen)
                    .sizing(NAVIGATION)
                    .radius(RadiusRole::Card)
                    .disabled()
                    .render(),
                IconButton::new("button-sizing-close", Icon::Close, palette)
                    .variant(ButtonVariant::Ghost)
                    .sizing(TAB_CLOSE)
                    .render(),
            ]),
            caption(cx, "Un perfil propio en lugar de un tamaño con nombre: 32 px con texto de 14, y un cierre de 20 px."),
        ]),
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
        section(cx, "Tinta, separación y radio", vec![
            row(vec![
                Button::new("button-ink", "Descartar", palette)
                    .variant(ButtonVariant::Ghost)
                    .ink(DANGER)
                    .render(),
                Button::new("button-ink-disabled", "Descartar", palette)
                    .variant(ButtonVariant::Ghost)
                    .ink(DANGER)
                    .disabled()
                    .render(),
                Button::new("button-gap", "Conectar", palette)
                    .variant(ButtonVariant::Outline)
                    .icon(Icon::Github)
                    .gap(12.0)
                    .render(),
                Button::new("button-radius", "Tarjeta", palette)
                    .variant(ButtonVariant::Outline)
                    .radius(RadiusRole::Card)
                    .render(),
                IconButton::new("button-radius-square", Icon::Add, palette)
                    .variant(ButtonVariant::Outline)
                    .radius(RadiusRole::Square)
                    .tooltip("Radio: Square")
                    .render(),
            ]),
            row(vec![
                IconButton::new("button-round-send", Icon::ArrowUp, palette)
                    .round()
                    .tooltip("round: enviar")
                    .render(),
                IconButton::new("button-round-stop", Icon::StopFill, palette)
                    .round()
                    .tooltip("round: detener")
                    .render(),
                Button::new("button-round-pill", "Píldora", palette)
                    .variant(ButtonVariant::Secondary)
                    .round()
                    .render(),
            ]),
            caption(cx, "La tinta la pasa la aplicación; deshabilitado, el botón vuelve al tono apagado. round hace un círculo de un botón de icono y una píldora de uno con texto."),
        ]),
        section(cx, "Grupo de botones", vec![
            ButtonGroup::new(group("button-group-row", palette)).render(),
            ButtonGroup::new(group("button-group-end", palette))
                .align_end()
                .leading(
                    Button::new("button-group-help", "Ayuda", palette)
                        .variant(ButtonVariant::Ghost)
                        .render(),
                )
                .render(),
            ButtonGroup::new(group("button-group-stacked", palette))
                .stacked(true)
                .reverse_when_stacked(true)
                .leading(
                    Button::new("button-group-stacked-help", "Ayuda", palette)
                        .variant(ButtonVariant::Ghost)
                        .render(),
                )
                .render()
                .size(Some(220.0), None),
            caption(cx, "En fila, alineado al final con un elemento inicial, y apilado: el elemento inicial arriba y la acción principal primero."),
            ButtonGroup::new(group("button-group-wide", palette)).sizing(WIDE_GROUP).render(),
            ButtonGroup::new(group("button-group-wide-stacked", palette))
                .sizing(WIDE_GROUP)
                .stacked(true)
                .render()
                .size(Some(220.0), None),
            caption(cx, "Con un perfil: 12 px entre acciones en fila y 10 en columna."),
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
