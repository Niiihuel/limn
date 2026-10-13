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
        DropdownMenuItem::new(format!("{id}-plain"), "Copy as plain text").icon(Icon::Text).into(),
        DropdownMenuItem::new(format!("{id}-activity"), "View turn activity").icon(Icon::History).into(),
        DropdownMenuEntry::Separator,
        DropdownMenuItem::new(format!("{id}-retry"), "Retry").icon(Icon::Refresh).disabled().into(),
    ]
}

/// The row under an assistant's message. A click on copy flips its done
/// state here; a host shows it for `MessageActionDone::SHOWN_FOR`.
fn assistant(cx: &Cx, id: &str, visibility: MessageActionsVisibility) -> MessageActions {
    // The trigger and the dismissal are two IDs, and each click flips one.
    let open = cx.state.flag(&format!("{id}:more")) != cx.state.flag(&format!("{id}:more:dismiss"));
    let copy = format!("{id}-copy");
    MessageActions::new(id, cx.palette)
        .action(MessageAction::new(&copy, Icon::Copy, "Copy").done(cx.state.flag(&copy)))
        .action(MessageAction::new(format!("{id}-retry"), Icon::Refresh, "Retry"))
        .overflow(overflow(id), open, "More actions")
        .caption("5 min ago · 14 s · opus-5.5")
        .visibility(visibility)
}

fn bubble(cx: &Cx, id: &str, text: &str, visibility: MessageActionsVisibility) -> Element {
    let actions = MessageActions::new(id, cx.palette)
        .action(MessageAction::new(format!("{id}-copy"), Icon::Copy, "Copy"))
        .action(MessageAction::new(format!("{id}-edit"), Icon::Edit, "Edit").shortcut("Enter"))
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
        ("Why does the content jump on hover?", "The bug is in the spacing calculation: the row reserves its height even when it is not drawn."),
        ("Always show the actions.", "Done: the row stays visible on the last message and appears on hover for the others."),
        ("Check the narrow layout too.", "At 360 px, the caption is cut before the buttons, which keep their size."),
        ("Add a test for the narrow layout.", "I added one that measures the row at 360 and 720 px and compares the message height."),
        ("The hover fill does not reach the card's rounded corners.", "I fixed the fill: it now uses the card radius minus the inner padding."),
        ("Add it to the gallery.", "This page has a new section showing all three visibility modes."),
        ("How long does the animation take?", "170 ms with a spring that does not bounce: changing its target mid-flight preserves its velocity."),
        ("Try it with the light theme.", "In the light theme, muted ink has 60 % opacity and active ink uses the text color."),
        ("Commit it.", "Done: one commit with the component, its tests, and the gallery page."),
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
    let answer = "The bug is in the spacing calculation: the row reserves its height even when not drawn, \
                  so the message stays the same size when the actions appear.";
    let sections: Vec<Section> = vec![
        section(cx, "Assistant message", vec![
            caption(cx, "24 px buttons with 2 px gaps and muted ink at rest. The row reserves its space even when it is not drawn."),
            caption(cx, "Always visible: the last message"),
            Element::column(vec![prose(cx, answer), assistant(cx, "messages-last", MessageActionsVisibility::Always).render()])
                .gap(4.0),
            caption(cx, "On message hover, or while the row has keyboard focus"),
            Element::column(vec![prose(cx, answer), assistant(cx, "messages-old", MessageActionsVisibility::Hover).render()])
                .gap(4.0)
                .hover_region(),
            caption(cx, "While streaming: reserved space only"),
            Element::column(vec![
                prose(cx, "The bug is in the calculation of"),
                assistant(cx, "messages-live", MessageActionsVisibility::Hidden).render(),
            ])
            .gap(4.0),
        ]),
        section(cx, "User message", vec![
            bubble(cx, "messages-user", "Why does the content jump on hover?", MessageActionsVisibility::Hover),
            bubble(cx, "messages-user-shown", "Always show the actions.", MessageActionsVisibility::Always),
            caption(cx, "Below the bubble, aligned to the trailing edge"),
            Element::column(vec![
                Card::new(prose(cx, "Check the narrow layout too."), palette).border(false).render(),
                MessageActions::new("messages-under", palette)
                    .caption("14:32")
                    .action(MessageAction::new("messages-under-copy", Icon::Copy, "Copy"))
                    .action(
                        MessageAction::new("messages-under-edit", Icon::Edit, "Edit")
                            .disabled("Wait for the agent to finish before editing an earlier message."),
                    )
                    .align_end()
                    .render(),
            ])
            .gap(4.0),
        ]),
        section(cx, "Toggles and branches", vec![
            Element::row(vec![
                Pager::new("2 / 3", palette)
                    .previous("messages-branch-previous", true)
                    .next("messages-branch-next", true)
                    .render(),
                MessageActions::new("messages-toggles", palette)
                    .action(
                        MessageAction::new("messages-pin", Icon::Pin, "Pin to context")
                            .active(cx.state.flag("messages-pin")),
                    )
                    .action(MessageAction::new("messages-toggles-copy", Icon::Copy, "Copy"))
                    .render()
                    .flex(1.0),
            ])
            .gap(8.0),
            Text::new("A `Pager` alongside shows the position among versions of a response.", palette)
                .role(TextRole::Small)
                .muted()
                .render(),
        ]),
        section(cx, "Conversation minimap", vec![
            caption(cx, "One mark per turn. The hovered mark grows and its neighbors taper away; it shows the turn immediately. Up and Down move through the marks."),
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
