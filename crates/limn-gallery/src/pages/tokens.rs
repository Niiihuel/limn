use super::{caption, row, section, Cx, Section};
use limn::{
    Alert, AlertTone, Button, ButtonVariant, Component, CornerRadii, Element, Icon, IconButton,
    IconKeyframe, IconMotion, Point, SegmentedControl, Switch, Tab, TextField,
};

/// A segmented choice stored under `group`.
fn choice(cx: &Cx, group: &str, options: &[(&str, &str)]) -> Element {
    let chosen = cx.state.choice(group, "default");
    SegmentedControl::new(
        options
            .iter()
            .map(|(id, label)| Tab::new(format!("{group}={id}"), *label, 110.0).active(*id == chosen))
            .collect(),
        cx.palette,
    )
    .render()
}

/// Icons that move under the pointer, as a voice panel's settings, mute and
/// deafen buttons do.
fn moving_icons(cx: &Cx) -> Element {
    let button = |id: &str, icon: Icon, label: &str, motion: IconMotion| {
        IconButton::new(id, icon, cx.palette).accessibility_label(label).render().hover_motion(format!("{id}:motion"), motion)
    };
    row(vec![
        button("tokens-gear", Icon::Settings, "Settings", IconMotion::new()
            .keyframe(IconKeyframe::at(120))
            .keyframe(IconKeyframe::at(1070).turn(180.0))),
        button("tokens-mute", Icon::MicrophoneMuted, "Unmute", IconMotion::new()
            .keyframe(IconKeyframe::at(160).turn(-5.0))
            .keyframe(IconKeyframe::at(400).turn(5.0))
            .keyframe(IconKeyframe::at(650))),
        button("tokens-deafen", Icon::Headphones, "Deafen", IconMotion::new()
            .keyframe(IconKeyframe::at(150).scale(1.06, 0.88))
            .keyframe(IconKeyframe::at(350).scale(1.0, 1.04))
            .keyframe(IconKeyframe::at(600))),
    ])
}

/// Two servers on a rail: the pill at the edge of one with unread messages
/// grows from 8 to 20 under the pointer; on the other it slides in.
fn rail_pills(cx: &Cx) -> Element {
    // Rounded on its right end only: a radius of its width is a shape, not a token.
    let ends = CornerRadii { top_left: 0.0, top_right: 4.0, bottom_right: 4.0, bottom_left: 0.0 };
    let bar = |height: f32| {
        Element::panel(Element::spacer()).background(cx.palette.text).corner_radii(ends).size(Some(4.0), Some(height)).centered_y()
    };
    let server = |pill: Element| {
        Element::row(vec![
            pill.size(Some(4.0), Some(48.0)),
            Element::panel(Element::spacer()).background(cx.palette.selected).radius(limn::tokens().radius.card).size(Some(48.0), Some(48.0)),
        ])
        .gap(8.0)
        .hover_region()
    };
    row(vec![
        server(Element::stack(vec![
            bar(8.0).conceal_on_hover("tokens-pill-unread"),
            bar(20.0).reveal_on_hover("tokens-pill-hover").grow_from(8.0),
        ])),
        server(bar(20.0).reveal_on_hover("tokens-pill-new").enter_from(Point::new(-4.0, 0.0))),
    ])
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let tokens = limn::tokens();
    let width = cx.width.min(520.0);
    vec![
        caption(cx, "One call to set_tokens changes the shape and pace of the whole library.").into(),
        section(cx, "Corners", vec![
            row(vec![choice(cx, "tokens-radius", &[("square", "Square"), ("default", "Default"), ("soft", "Soft"), ("round", "Round")])]),
            caption(cx, &format!(
                "control {} · surface {} · card {} · panel {}",
                tokens.radius.control, tokens.radius.surface, tokens.radius.card, tokens.radius.panel
            )),
        ]),
        section(cx, "Motion", vec![
            row(vec![choice(cx, "tokens-motion", &[("fast", "Fast"), ("default", "Normal"), ("slow", "Slow")])]),
            caption(cx, &format!(
                "hover {} ms · enter {} ms · exit {} ms · move {} ms",
                tokens.motion.hover_ms, tokens.motion.enter_ms, tokens.motion.exit_ms, tokens.motion.move_ms
            )),
            row(vec![
                Switch::new("tokens-reduced", cx.state.flag("tokens-reduced"), palette).render(),
                caption(cx, "No animations (set_reduced_motion)"),
            ]),
            moving_icons(cx),
            caption(cx, "hover_motion: under the pointer the gear turns half way round, the microphone shakes and the headphones squash."),
            rail_pills(cx),
            caption(cx, "grow_from and enter_from: a rail's pill grows from 8 to 20 px on hover, or slides in from 4 px to the left."),
        ]),
        section(cx, "Sample", vec![
            row(vec![
                Button::new("tokens-primary", "Save", palette).icon(Icon::Check).render(),
                Button::new("tokens-outline", "Cancel", palette).variant(ButtonVariant::Outline).render(),
                TextField::new("tokens-field", cx.state.text("tokens-field"), "Name", palette)
                    .width(180.0)
                    .render(),
            ]),
            Element::panel(
                Alert::new("Card", "Alerts use the card radius.", palette)
                    .tone(AlertTone::Info)
                    .render(),
            )
            .size(Some(width), None),
            Element::panel(caption(cx, "Panel: dialogs and previews use the panel radius."))
                .padding(32.0)
                .background(palette.surface)
                .border(palette.border, 1.0)
                .radius(tokens.radius.panel)
                .size(Some(width), None),
        ]),
    ]
}
