use super::{caption, section, Cx, Section};
use limn::{
    Card, ChatMinimap, ChatMinimapMark, Component, DropdownMenuEntry, DropdownMenuItem, Element,
    Icon, MessageAction, MessageActions, MessageActionsVisibility, Pager, Text, TextRole,
};

fn prose(cx: &Cx, text: &str) -> Element {
    Text::new(text, cx.palette).render()
}

fn overflow(id: &str) -> Vec<DropdownMenuEntry> {
    vec![
        DropdownMenuItem::new(format!("{id}-plain"), "Copiar como texto sin formato").icon(Icon::Text).into(),
        DropdownMenuItem::new(format!("{id}-activity"), "Ver actividad del turno").icon(Icon::History).into(),
        DropdownMenuEntry::Separator,
        DropdownMenuItem::new(format!("{id}-retry"), "Reintentar").icon(Icon::Refresh).disabled().into(),
    ]
}

/// The row under an assistant's message. A click on copy flips its done
/// state here; a host shows it for `MessageActionDone::SHOWN_FOR`.
fn assistant(cx: &Cx, id: &str, visibility: MessageActionsVisibility) -> MessageActions {
    // The trigger and the dismissal are two IDs, and each click flips one.
    let open = cx.state.flag(&format!("{id}:more")) != cx.state.flag(&format!("{id}:more:dismiss"));
    let copy = format!("{id}-copy");
    MessageActions::new(id, cx.palette)
        .action(MessageAction::new(&copy, Icon::Copy, "Copiar").done(cx.state.flag(&copy)))
        .action(MessageAction::new(format!("{id}-retry"), Icon::Refresh, "Reintentar"))
        .overflow(overflow(id), open, "Más acciones")
        .caption("hace 5 min · 14 s · opus-5.5")
        .visibility(visibility)
}

fn bubble(cx: &Cx, id: &str, text: &str, visibility: MessageActionsVisibility) -> Element {
    let actions = MessageActions::new(id, cx.palette)
        .action(MessageAction::new(format!("{id}-copy"), Icon::Copy, "Copiar"))
        .action(MessageAction::new(format!("{id}-edit"), Icon::Edit, "Editar").shortcut("Enter"))
        .visibility(visibility)
        .render();
    Card::new(Element::row(vec![prose(cx, text).flex(1.0), actions]).gap(8.0), cx.palette)
        .border(false)
        .render()
        .hover_region()
}

/// A conversation's turns as marks; a click on one puts that turn in view.
fn minimap(cx: &Cx) -> Element {
    const TURNS: [(&str, &str); 9] = [
        ("¿Por qué salta el contenido al pasar el puntero?", "El fallo está en el cálculo del margen: la fila reserva su alto aunque no se pinte."),
        ("Mostrá las acciones siempre.", "Listo: la fila queda visible en el último mensaje y aparece con el puntero en los demás."),
        ("Revisá también el modo estrecho.", "En 360 px la leyenda se corta antes que los botones, que conservan su tamaño."),
        ("Agregá una prueba para el modo estrecho.", "Agregué una que mide la fila a 360 y a 720 px y compara el alto del mensaje."),
        ("El hover no llega a las curvas de la tarjeta.", "Corregí el relleno: ahora usa el radio de la tarjeta menos el margen interior."),
        ("Pasalo a la galería.", "Hay una sección nueva en esta página con los tres modos de visibilidad."),
        ("¿Cuánto tarda la animación?", "170 ms en un resorte sin rebote: si cambia de destino a mitad de camino conserva la velocidad."),
        ("Probalo con el tema claro.", "En claro la tinta atenuada baja a 60 % de opacidad y la activa usa el color del texto."),
        ("Hacé el commit.", "Hecho: un commit con el componente, sus pruebas y la página de la galería."),
    ];
    let marks = TURNS
        .iter()
        .enumerate()
        .map(|(index, (title, preview))| {
            ChatMinimapMark::new(format!("messages-turn={index}"), *title).preview(preview)
        })
        .collect();
    let chosen = format!("messages-turn={}", cx.state.choice("messages-turn", "6"));
    Element::row(vec![
        ChatMinimap::new("messages-minimap", marks, cx.palette).active(chosen).render(),
        Element::spacer().flex(1.0),
    ])
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let answer = "El fallo está en el cálculo del margen: la fila reserva su alto aunque no se pinte, \
                  así que el mensaje no cambia de tamaño cuando aparecen las acciones.";
    let sections: Vec<Section> = vec![
        caption(cx, "Botones de 24 px separados por 2, tinta atenuada en reposo. La fila ocupa su lugar aunque no se pinte.").into(),
        section(cx, "Mensaje del asistente", vec![
            caption(cx, "Siempre visible: el último mensaje"),
            Element::column(vec![prose(cx, answer), assistant(cx, "messages-last", MessageActionsVisibility::Always).render()])
                .gap(4.0),
            caption(cx, "Al pasar el puntero por el mensaje, o con el foco del teclado en la fila"),
            Element::column(vec![prose(cx, answer), assistant(cx, "messages-old", MessageActionsVisibility::Hover).render()])
                .gap(4.0)
                .hover_region(),
            caption(cx, "Mientras se escribe: sólo el lugar"),
            Element::column(vec![
                prose(cx, "El fallo está en el cálculo del"),
                assistant(cx, "messages-live", MessageActionsVisibility::Hidden).render(),
            ])
            .gap(4.0),
        ]),
        section(cx, "Mensaje del usuario", vec![
            bubble(cx, "messages-user", "¿Por qué salta el contenido al pasar el puntero?", MessageActionsVisibility::Hover),
            bubble(cx, "messages-user-shown", "Mostrá las acciones siempre.", MessageActionsVisibility::Always),
            caption(cx, "Debajo de la burbuja, contra el borde final"),
            Element::column(vec![
                Card::new(prose(cx, "Revisá también el modo estrecho."), palette).border(false).render(),
                MessageActions::new("messages-under", palette)
                    .caption("14:32")
                    .action(MessageAction::new("messages-under-copy", Icon::Copy, "Copiar"))
                    .action(
                        MessageAction::new("messages-under-edit", Icon::Edit, "Editar")
                            .disabled("Esperá a que termine el agente para editar un mensaje anterior."),
                    )
                    .align_end()
                    .render(),
            ])
            .gap(4.0),
        ]),
        section(cx, "Conmutadores y ramas", vec![
            Element::row(vec![
                Pager::new("2 / 3", palette)
                    .previous("messages-branch-previous", true)
                    .next("messages-branch-next", true)
                    .render(),
                MessageActions::new("messages-toggles", palette)
                    .action(
                        MessageAction::new("messages-pin", Icon::Pin, "Fijar al contexto")
                            .active(cx.state.flag("messages-pin")),
                    )
                    .action(MessageAction::new("messages-toggles-copy", Icon::Copy, "Copiar"))
                    .render()
                    .flex(1.0),
            ])
            .gap(8.0),
            Text::new("Un `Pager` al lado da la posición entre versiones de una respuesta.", palette)
                .role(TextRole::Small)
                .muted()
                .render(),
        ]),
        section(cx, "Minimapa de la conversación", vec![
            caption(cx, "Una marca por turno. La del puntero crece y sus vecinas bajan en escalera; muestra el turno sin espera. Arriba y Abajo recorren las marcas."),
            minimap(cx),
        ]),
    ];
    // Every message is as wide as the conversation it is in.
    sections
        .into_iter()
        .map(|section| Section {
            demos: vec![Element::column(section.demos).gap(12.0).size(Some(width), None)],
            ..section
        })
        .collect()
}
