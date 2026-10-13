use super::{caption, row, section, Cx, Section};
use limn::{
    Button, ButtonVariant, CoachAlign, CoachMark, CoachSide, Color, Component, Element, Icon, Switch,
    TextField,
};

/// Target, title, body and placement of each step.
const STEPS: [(&str, &str, &str, CoachSide, CoachAlign); 3] = [
    (
        "tour-new",
        "Start here",
        "Each task is a conversation with an agent about your project.",
        CoachSide::Below,
        CoachAlign::Start,
    ),
    (
        "tour-search",
        "Find what you need",
        "Type part of a task or file name to filter the list.",
        CoachSide::Below,
        CoachAlign::Center,
    ),
    (
        "tour-notify",
        "Know when it finishes",
        "With notifications enabled, you do not need to keep watching: we let you know when the agent needs you.",
        CoachSide::Left,
        CoachAlign::Center,
    ),
];

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let toolbar = Element::row(vec![
        Button::new("tour-new", "New task", palette).icon(Icon::Add).render(),
        TextField::new("tour-search", state.text("tour-search"), "Search", palette)
            .render()
            .size(Some(220.0), None),
        Element::spacer().flex(1.0),
        caption(cx, "Notifications"),
        Switch::new("tour-notify", state.flag("tour-notify"), palette).render(),
    ])
    .gap(12.0);
    let option = |id: &str, label: &str| {
        Element::row(vec![Switch::new(id, state.flag(id), palette).render(), caption(cx, label)]).gap(8.0)
    };
    vec![
        section(
            cx,
            "Three-step tour",
            vec![
                toolbar,
                row(vec![Button::new("tour-step=0", "Start tour", palette)
                    .variant(ButtonVariant::Outline)
                    .render()]),
                caption(
                    cx,
                    "Left and Right arrow keys or Enter to advance, Esc to close. \
                     The rest of the window does not receive clicks during the tour.",
                ),
            ],
        ),
        section(
            cx,
            "Variants",
            vec![
                option("tour-accent", "Application accent color"),
                option("tour-dim", "Dim the window around the control"),
            ],
        ),
    ]
}

/// Wrap the window in the step the page is on.
pub fn layer(cx: &Cx, base: Element) -> Element {
    let state = cx.state;
    let Some(index) = state.text("tour-step").parse::<usize>().ok().filter(|index| *index < STEPS.len())
    else {
        return base;
    };
    let (target, title, body, side, align) = STEPS[index];
    let mut mark = CoachMark::new(base, "tour-step=closed", target, cx.palette)
        .title(title)
        .body(body)
        .placement(side, align)
        .step(index, STEPS.len())
        .skip("Skip tour")
        .on_skip("tour-step=skipped")
        .on_next(format!("tour-step={}", index + 1))
        .on_back(format!("tour-step={}", index.saturating_sub(1)));
    if state.flag("tour-accent") {
        mark = mark.accent(Color::rgb(9, 105, 218), Color::WHITE);
    }
    if state.flag("tour-dim") {
        mark = mark.dim(Color::rgba(0, 0, 0, 110));
    }
    mark.render()
}
