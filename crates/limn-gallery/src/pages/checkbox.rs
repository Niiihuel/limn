use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{Checkbox, Component, ControlSize, Element, UiAction};

/// The list under the parent box of the indeterminate demo: a flag's ID and its label.
const CHANNELS: [(&str, &str); 3] = [
    ("check-channel-mail", "Email"),
    ("check-channel-push", "Push notifications"),
    ("check-channel-digest", "Weekly digest"),
];
const ALL: &str = "check-channels";

/// The parent box chooses the whole list, or none of it once all are chosen.
pub fn update(state: &mut State, action: &UiAction) {
    if matches!(action, UiAction::Clicked { id } if id == ALL) {
        let value = if CHANNELS.iter().all(|(id, _)| state.flag(id)) { "off" } else { "on" };
        for (id, _) in CHANNELS {
            state.set(id, value);
        }
    }
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let live = |id: &str, label: &str| Checkbox::new(id, state.flag(id), palette).label(label);
    let chosen = CHANNELS.iter().filter(|(id, _)| state.flag(id)).count();
    let mut channels = vec![row(vec![Checkbox::new(ALL, chosen == CHANNELS.len(), palette)
        .label("All channels")
        .indeterminate(chosen > 0 && chosen < CHANNELS.len())
        .render()])];
    channels.extend(CHANNELS.iter().map(|(id, label)| {
        row(vec![Element::spacer().size(Some(24.0), None), live(id, label).render()])
    }));
    vec![
        section(cx, "Checkbox", vec![
            row(vec![live("check-terms", "I accept the terms").render()]),
            row(vec![live("check-updates", "Send me product updates").render()]),
            caption(cx, &format!(
                "The box and its label are one target. Terms: {}. Updates: {}.",
                if state.flag("check-terms") { "accepted" } else { "not accepted" },
                if state.flag("check-updates") { "on" } else { "off" },
            )),
        ]),
        section(cx, "Indeterminate", vec![
            Element::column(channels).gap(8.0),
            caption(cx, &format!(
                "{chosen} of {} chosen. The parent shows a dash while only some are, and a click on it chooses all or none.",
                CHANNELS.len(),
            )),
        ]),
        section(cx, "Sizes", vec![
            Element::column(vec![
                row(vec![live("check-dense", "Dense").size(ControlSize::Dense).render()]),
                row(vec![live("check-compact", "Compact").size(ControlSize::Compact).render()]),
                row(vec![live("check-regular", "Regular").size(ControlSize::Regular).render()]),
                row(vec![live("check-spacious", "Spacious").size(ControlSize::Spacious).render()]),
            ])
            .gap(8.0),
            caption(cx, "A size sets the box and the label's text. Regular is the default."),
        ]),
        section(cx, "Disabled", vec![
            row(vec![
                Checkbox::new("check-off-empty", false, palette).label("Unchecked").disabled().render(),
                Checkbox::new("check-off-checked", true, palette).label("Checked").disabled().render(),
                Checkbox::new("check-off-mixed", false, palette).label("Indeterminate").indeterminate(true).disabled().render(),
            ]),
            caption(cx, "It shows its value in the muted color and takes no pointer or focus."),
        ]),
        section(cx, "Long label", vec![
            live(
                "check-long",
                "Keep a copy of every message on this device, including attachments, so that they can be read without a connection",
            )
            .render()
            .size(Some(cx.width.min(280.0)), None),
            caption(cx, "In 280 pixels the label wraps and the box stays on its first line."),
        ]),
        section(cx, "Without a label", vec![
            row(vec![
                live("check-row-1", "").accessibility_label("Select row 1").render(),
                live("check-row-2", "").accessibility_label("Select row 2").render(),
                Checkbox::new("check-row-3", true, palette).accessibility_label("Select row 3").disabled().render(),
            ]),
            caption(cx, "A box alone, as in a table's row, is named with `accessibility_label`."),
        ]),
    ]
}
