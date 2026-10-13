use super::{caption, row, section, Cx, Section};
use limn::{
    Color, Component, ControlSize, DropdownOption, Element, FormField, FormFieldSizing, Icon, PopoverPlacement, RadiusRole,
    SearchField, Select, SelectorSizing, Slider, Switch, TextArea, TextField, TextRole,
};

const SHELLS: [(&str, &str); 4] = [("bash", "Bash"), ("zsh", "Zsh"), ("fish", "Fish"), ("nu", "Nushell")];

/// A slider's value as the gallery keeps it, or `initial` before it is moved.
fn fraction(cx: &Cx, id: &str, initial: f32) -> f32 {
    cx.state.text(id).parse().unwrap_or(initial)
}

/// A select over the demo's shells; `group` keeps its choice and open state.
fn shell_select(cx: &Cx, group: &str) -> Select {
    let chosen = cx.state.choice(group, SHELLS[0].0);
    let label = SHELLS.iter().find(|(id, _)| *id == chosen).map_or(SHELLS[0].1, |(_, label)| label);
    let options = SHELLS
        .iter()
        .map(|(id, label)| {
            let option = DropdownOption::new(format!("{group}={id}&{group}-open=no"), *label)
                .selected(*id == chosen);
            if *id == "nu" {
                option.disabled()
            } else {
                option
            }
        })
        .collect();
    Select::new(
        format!("{group}-open=yes"),
        label,
        options,
        cx.state.text(&format!("{group}-open")) == "yes",
        format!("{group}-open=no"),
        cx.palette,
    )
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let width = cx.width.min(360.0);
    vec![
        section(cx, "Campo de texto", vec![
            TextField::new("input-name", state.text("input-name"), "Nombre del proyecto", palette)
                .width(width)
                .render(),
            caption(cx, &format!("Valor: {}", state.text("input-name"))),
            TextField::new("input-password", state.text("input-password"), "Contraseña", palette)
                .password(true)
                .width(width)
                .render(),
            TextField::new("input-disabled", "No editable", "", palette).disabled().width(width).render(),
            TextField::new("input-path", state.text("input-path"), "src/main.rs", palette)
                .monospace(true)
                .width(width)
                .render(),
            caption(cx, "Monoespaciado, para rutas e identificadores."),
            TextField::new("input-tall", state.text("input-tall"), "ej. filesystem", palette)
                .size(ControlSize::Compact)
                .width(width)
                .height(40.0)
                .render(),
            caption(cx, "Con la altura del formulario: 40 sobre un control compacto."),
        ]),
        section(cx, "Campo de formulario", vec![
            Element::column(vec![
                FormField::new(
                    "Nombre",
                    TextField::new("field-name", state.text("field-name"), "Mi proyecto", palette).render(),
                    palette,
                )
                .render(),
                FormField::new(
                    "Ejecutable",
                    TextField::new("field-program", state.text("field-program"), "/usr/bin/env", palette)
                        .monospace(true)
                        .render(),
                    palette,
                )
                .hint("Ruta absoluta a un archivo existente. Se ejecuta sin shell.")
                .hint("El agente no puede cambiar esta ruta.")
                .hint_above(true)
                .render(),
                FormField::new(
                    "Ruta dentro del proyecto",
                    TextField::new("field-path", state.text("field-path"), "src/nuevo.rs", palette).render(),
                    palette,
                )
                .hint("Las carpetas que falten se crean.")
                .error("Ya existe un archivo con esa ruta.", Color::rgb(229, 83, 75))
                .render(),
                FormField::new(
                    "Instrucciones",
                    TextArea::new("field-notes", state.text("field-notes"), "Describe la tarea", palette)
                        .height(72.0)
                        .render(),
                    palette,
                )
                .hint("Etiqueta apagada sobre una línea de 18, como en un asistente por pasos.")
                .sizing(FormFieldSizing {
                    label_muted: true,
                    label_height: 18.0,
                    hint_role: TextRole::Caption,
                    gap: 8.0,
                    ..FormFieldSizing::DEFAULT
                })
                .render(),
            ])
            .gap(16.0)
            .size(Some(width), None),
            caption(cx, "Con `rows`, las líneas de dos campos en la columna del formulario, que pone la distancia."),
            Element::column({
                let mut rows = FormField::new(
                    "Programa",
                    TextField::new("flat-program", state.text("flat-program"), "ej. npx", palette).render(),
                    palette,
                )
                .hint("Se inicia sin shell.")
                .hint_above(true)
                .rows();
                rows.extend(
                    FormField::new(
                        "Directorio de trabajo",
                        TextField::new("flat-cwd", state.text("flat-cwd"), "ej. tools/mcp", palette).render(),
                        palette,
                    )
                    .rows(),
                );
                rows
            })
            .gap(7.0)
            .size(Some(width), None),
        ]),
        section(cx, "Selector", vec![
            row(vec![
                shell_select(cx, "select-plain").render(),
                shell_select(cx, "select-off").enabled(false).render(),
            ]),
            caption(cx, "La lista toma el ancho del botón. Desactivado no se abre ni recibe el foco."),
            row(vec![shell_select(cx, "select-compact")
                .sizing(SelectorSizing::COMPACT)
                .icon(Icon::Terminal)
                .placement(PopoverPlacement::BottomStart)
                .render()]),
            caption(cx, "Con un perfil: filas compactas y la lista en su ancho preferido, con icono."),
            shell_select(cx, "select-wide")
                .width(width)
                .placement(PopoverPlacement::BottomStart)
                .render(),
            caption(cx, "Tan ancho como su formulario, con la lista bajo el borde inicial."),
        ]),
        section(cx, "Búsqueda", vec![
            SearchField::new("input-search", state.text("input-search"), "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(width)
                .render(),
        ]),
        section(cx, "Área de texto", vec![
            TextArea::new("input-notes", state.text("input-notes"), "Notas", palette)
                .width(width)
                .height(120.0)
                .render(),
            caption(cx, ".radius(RadiusRole::Surface), y deshabilitada"),
            TextArea::new("input-notes-surface", state.text("input-notes-surface"), "Mensaje", palette)
                .width(width)
                .height(64.0)
                .radius(RadiusRole::Surface)
                .render(),
            TextArea::new("input-notes-disabled", "", "Sin permiso para escribir", palette)
                .width(width)
                .height(64.0)
                .radius(RadiusRole::Surface)
                .disabled()
                .render(),
        ]),
        section(cx, "Interruptor", vec![row(vec![
            Switch::new("input-switch", state.flag("input-switch"), palette).render(),
            Switch::new("input-switch-disabled", true, palette).disabled().render(),
        ])]),
        section(cx, "Deslizador", vec![
            Slider::new("input-slider", fraction(cx, "input-slider", 0.35), palette).width(280.0).render(),
            caption(cx, &format!("Continuo: {:.0} %", fraction(cx, "input-slider", 0.35) * 100.0)),
            Slider::new("input-slider-steps", fraction(cx, "input-slider-steps", 0.5), palette)
                .steps(5)
                .width(280.0)
                .render(),
            caption(cx, ".steps(5): al soltar queda en una de cinco paradas."),
            Slider::new("input-slider-disabled", 0.6, palette).enabled(false).width(280.0).render(),
            caption(cx, "Deshabilitado: muestra el valor y no recibe puntero ni foco."),
        ]),
    ]
}
