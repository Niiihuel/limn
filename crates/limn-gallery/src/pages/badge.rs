use super::{caption, row, section, Cx, Section};
use limn::{
    Access, AlertColors, AlertTone, Avatar, Badge, Button, ButtonVariant, Component, Element, Icon, IconButton, Role,
};

/// An icon button of the demo, named for a screen reader.
fn tool(cx: &Cx, id: &str, icon: Icon, name: &str) -> Element {
    IconButton::new(id, icon, cx.palette).variant(ButtonVariant::Outline).accessibility_label(name).render()
}

/// What a count says to a screen reader. The words are the host's.
fn unread(count: u32) -> String {
    format!("{count} unread")
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let count: u32 = cx.state.text("badge-count").parse().unwrap_or(3);
    let urgent = AlertColors::for_tone(AlertTone::Error, palette).accent;
    let bell = |id: &str| tool(cx, id, Icon::Bell, "Notifications");
    let counted = |id: &str, count: u32| Badge::new(bell(id), palette).count(count).description(unread(count));
    vec![
        section(cx, "Count", vec![
            row(vec![
                counted("badge-bell", count).render(),
                Button::new(format!("badge-count={}", count.saturating_add(1)), "Add one", palette)
                    .variant(ButtonVariant::Secondary)
                    .render(),
                Button::new(format!("badge-count={}", count.saturating_mul(10)), "Times ten", palette)
                    .variant(ButtonVariant::Secondary)
                    .render(),
                Button::new("badge-count=0", "Read all", palette).variant(ButtonVariant::Secondary).render(),
            ]),
            caption(cx, &format!("Count: {count}. Zero shows no mark; above 99 it reads 99+.")),
        ]),
        section(cx, "Dot", vec![
            row(vec![
                Badge::new(bell("badge-dot"), palette).dot().description("New activity").render(),
                Badge::new(bell("badge-dot-urgent"), palette).dot().color(urgent).description("Needs attention").render(),
            ]),
            caption(cx, "Something is new, without saying how many."),
        ]),
        section(cx, "Maximum and zero", vec![
            row(vec![
                counted("badge-max-9", 12).max(9).render(),
                counted("badge-max-99", 120).render(),
                counted("badge-max-999", 1200).max(999).render(),
                counted("badge-zero", 0).show_zero(true).render(),
                counted("badge-none", 0).render(),
            ]),
            caption(cx, "max(9), the default 99, max(999), a zero that is shown, and a zero that is not."),
        ]),
        section(cx, "Color", vec![
            row(vec![
                counted("badge-default", 5).render(),
                counted("badge-urgent", 5).color(urgent).render(),
                counted("badge-warning", 5).color(AlertColors::for_tone(AlertTone::Warning, palette).accent).render(),
            ]),
            caption(cx, "The palette's text color unless the host passes one, such as an alert color for urgency."),
        ]),
        section(cx, "On an avatar", vec![
            row(vec![
                // An avatar is no control: it is given a node, so the mark has something to describe.
                Badge::new(
                    Avatar::new("Ada Lovelace", palette)
                        .render()
                        .accessible(Access::new(Role::Image).id("badge-avatar").name("Ada Lovelace")),
                    palette,
                )
                .dot()
                .color(AlertColors::for_tone(AlertTone::Success, palette).accent)
                .description("Online")
                .render(),
                Badge::new(
                    Avatar::new("Grace Hopper", palette)
                        .size(48.0)
                        .render()
                        .accessible(Access::new(Role::Image).id("badge-avatar-large").name("Grace Hopper")),
                    palette,
                )
                .count(7)
                .description(unread(7))
                .render(),
            ]),
        ]),
        section(cx, "In a toolbar", vec![
            row(vec![
                tool(cx, "badge-row-search", Icon::Search, "Search"),
                Badge::new(tool(cx, "badge-row-bell", Icon::Bell, "Notifications"), palette)
                    .count(if cx.state.flag("badge-row-bell") { 0 } else { 4 })
                    .description(unread(4))
                    .render(),
                tool(cx, "badge-row-settings", Icon::Settings, "Settings"),
            ]),
            caption(cx, "Click the bell: the mark goes and no button moves."),
        ]),
        section(cx, "Disabled", vec![
            row(vec![Badge::new(
                IconButton::new("badge-disabled", Icon::Bell, palette)
                    .variant(ButtonVariant::Outline)
                    .accessibility_label("Notifications")
                    .disabled()
                    .render(),
                palette,
            )
            .count(2)
            .description(unread(2))
            .render()]),
            caption(cx, "The badge has no state of its own: a disabled control keeps its mark."),
        ]),
    ]
}
