use super::{caption, section, Cx};
use limn::{
    Card, Component, DropdownMenuEntry, DropdownMenuItem, Element, Icon, MessageAction,
    MessageActions, MessageActionsVisibility, Pager, Text, TextRole,
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

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let answer = "El fallo está en el cálculo del margen: la fila reserva su alto aunque no se pinte, \
                  así que el mensaje no cambia de tamaño cuando aparecen las acciones.";
    Element::column(vec![
        caption(cx, "Botones de 24 px separados por 2, tinta atenuada en reposo. La fila ocupa su lugar aunque no se pinte."),
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
    ])
    .gap(32.0)
    .size(Some(width), None)
}
