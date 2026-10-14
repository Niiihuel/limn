use super::{caption, section, Cx, Section};
use limn::{Component, ControlSize, Element, RadioGroup, RadioOption};

const THEMES: [(&str, &str); 3] = [("light", "Light"), ("dark", "Dark"), ("system", "Match the system")];
const PLANS: [(&str, &str, &str); 3] = [
    ("free", "Free", "Three projects, one seat."),
    ("team", "Team", "Unlimited projects, ten seats."),
    ("company", "Company", "Single sign-on and an audit log."),
];

/// The options of `group`, each reporting `group=value`, which the gallery
/// takes as a choice.
fn options(group: &str, items: &[(&str, &str)]) -> Vec<RadioOption> {
    items.iter().map(|(value, label)| RadioOption::new(format!("{group}={value}"), *label)).collect()
}

/// A group over `items` whose choice the gallery keeps under `group`; none is
/// chosen until one is clicked unless `default` names one.
fn group(cx: &Cx, group: &str, name: &str, items: Vec<RadioOption>, default: &str) -> RadioGroup {
    let chosen = format!("{group}={}", cx.state.choice(group, default));
    RadioGroup::new(group, items, Some(&chosen), cx.palette).accessibility_label(name)
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let themes = |id: &str| options(id, &THEMES);
    let plans = |id: &str| -> Vec<RadioOption> {
        PLANS
            .iter()
            .map(|(value, label, description)| {
                RadioOption::new(format!("{id}={value}"), *label).description(*description)
            })
            .collect()
    };
    vec![
        section(cx, "Radio group", vec![
            group(cx, "radio-theme", "Theme", themes("radio-theme"), "dark").render(),
            caption(cx, &format!("Chosen: {}", cx.state.choice("radio-theme", "dark"))),
        ]),
        section(cx, "Descriptions", vec![
            group(cx, "radio-plan", "Plan", plans("radio-plan"), "team").render(),
            caption(cx, "The description is clicked with its label."),
        ]),
        section(cx, "Horizontal", vec![
            group(cx, "radio-row", "Theme", themes("radio-row"), "light").horizontal().render(),
            caption(cx, "Left and Right move the choice; Home and End go to the ends."),
        ]),
        section(
            cx,
            "Sizes",
            [
                ("Dense", ControlSize::Dense),
                ("Compact", ControlSize::Compact),
                ("Regular", ControlSize::Regular),
                ("Spacious", ControlSize::Spacious),
            ]
            .into_iter()
            .flat_map(|(name, size)| {
                let id = format!("radio-size-{}", name.to_lowercase());
                [
                    group(cx, &id, &format!("{name} theme"), themes(&id), "light").size(size).horizontal().render(),
                    caption(cx, name),
                ]
            })
            .collect(),
        ),
        section(cx, "Disabled", vec![
            group(cx, "radio-off", "Theme, disabled", themes("radio-off"), "dark").disabled().render(),
            caption(cx, "A disabled group shows its choice and takes no pointer or focus."),
            group(
                cx,
                "radio-part",
                "Plan",
                plans("radio-part").into_iter().enumerate().map(|(index, plan)| plan.enabled(index != 2)).collect(),
                "free",
            )
            .render(),
            caption(cx, "One option disabled: the arrows pass over it."),
        ]),
        section(cx, "Edge cases", vec![
            group(cx, "radio-none", "Theme, not yet chosen", themes("radio-none"), "").render(),
            caption(cx, "No option chosen until one is clicked."),
            Element::column(vec![group(
                cx,
                "radio-long",
                "Retention",
                vec![
                    RadioOption::new("radio-long=keep", "Keep every run until someone deletes it by hand")
                        .description("Storage grows without limit, and old runs stay searchable for audits."),
                    RadioOption::new("radio-long=month", "Thirty days"),
                ],
                "keep",
            )
            .render()])
            .size(Some(cx.width.min(240.0)), None),
            caption(cx, "In 240 px the label and the description wrap; the circle centers on the label."),
        ]),
    ]
}
