use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{
    Color, ColorPicker, Component, ControlSize, Hsv, Popover, Swatch, SwatchPicker, UiAction,
};

/// Ten colors to choose among, each with the name the platform is told.
const COLORS: [(&str, Color, &str); 10] = [
    ("teal", Color::rgb(26, 188, 156), "Teal"),
    ("green", Color::rgb(46, 204, 113), "Green"),
    ("blue", Color::rgb(52, 152, 219), "Blue"),
    ("purple", Color::rgb(155, 89, 182), "Purple"),
    ("pink", Color::rgb(233, 30, 99), "Pink"),
    ("yellow", Color::rgb(241, 196, 15), "Yellow"),
    ("orange", Color::rgb(230, 126, 34), "Orange"),
    ("red", Color::rgb(231, 76, 60), "Red"),
    ("gray", Color::rgb(149, 165, 166), "Gray"),
    ("navy", Color::rgb(52, 73, 94), "Navy"),
];

/// The picker the custom swatch opens.
const CUSTOM: &str = "swatch-custom-picker";

/// Every live picker: its ID, its hex field's and the color it starts on.
/// The gallery keeps each color as `hue,saturation,value`.
const PICKERS: [(&str, &str, &str); 4] = [
    ("color-main", "color-main:hex", "#3498db"),
    ("color-narrow", "color-narrow:hex", "#e91e63"),
    ("color-labels", "color-labels:hex", "#2ecc71"),
    (CUSTOM, "swatch-custom-picker:hex", "#f5d76e"),
];

fn color(state: &State, id: &str) -> Hsv {
    let start = PICKERS
        .iter()
        .find(|picker| picker.0 == id)
        .map_or("#ffffff", |picker| picker.2);
    let mut kept = state
        .text(id)
        .split(',')
        .filter_map(|part| part.parse::<f32>().ok());
    match (kept.next(), kept.next(), kept.next()) {
        (Some(hue), Some(saturation), Some(value)) => Hsv::new(hue, saturation, value),
        _ => Hsv::from(ColorPicker::parse_hex(start).unwrap_or(Color::WHITE)),
    }
}

/// A picker's action becomes its next color; the custom swatch opens its
/// picker, and a color chosen there makes the custom swatch the choice.
pub fn update(state: &mut State, action: &UiAction) {
    if matches!(action, UiAction::Clicked { id } if id == "swatch-custom:custom") {
        state.set("swatch-custom-open", "open");
    }
    for (id, _, _) in PICKERS {
        if let Some(next) = ColorPicker::color_after(id, color(state, id), action) {
            state.set(
                id,
                &format!("{},{},{}", next.hue, next.saturation, next.value),
            );
            if id == CUSTOM {
                state.set("swatch-custom", "custom");
            }
        }
    }
}

/// Each picker's hex field and the color it stands for: after a drag the
/// tree is given these, since a field keeps the text typed in it.
pub fn fields(state: &State) -> Vec<(&'static str, String)> {
    PICKERS
        .iter()
        .map(|(id, hex, _)| (*hex, ColorPicker::hex(color(state, id).into())))
        .collect()
}

/// The first `count` colors as swatches of `group`, each reporting
/// `group=name`, which the gallery takes as a choice.
fn swatches(group: &str, count: usize) -> Vec<Swatch> {
    COLORS
        .iter()
        .take(count)
        .map(|(value, color, name)| Swatch::new(format!("{group}={value}"), *color, *name))
        .collect()
}

/// A picker over the first `count` colors whose choice the gallery keeps under `group`.
fn picker(cx: &Cx, group: &str, count: usize, default: &str) -> SwatchPicker {
    let selected = format!("{group}={}", cx.state.choice(group, default));
    SwatchPicker::new(group, swatches(group, count), Some(&selected), cx.palette)
        .accessibility_label("Accent color")
}

fn described(hsv: Hsv) -> String {
    format!(
        "{}: hue {}°, saturation {}%, value {}%",
        ColorPicker::hex(hsv.into()),
        hsv.hue.round(),
        (hsv.saturation * 100.0).round(),
        (hsv.value * 100.0).round()
    )
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;

    let custom = color(state, CUSTOM);
    let chosen = match state.choice("swatch-custom", "blue") {
        "custom" => "swatch-custom:custom".to_owned(),
        value => format!("swatch-custom={value}"),
    };
    let with_custom = Popover::new(
        SwatchPicker::new(
            "swatch-custom",
            swatches("swatch-custom", 6),
            Some(&chosen),
            palette,
        )
        .custom(custom.into(), "Custom color")
        .accessibility_label("Role color")
        .render(),
        ColorPicker::new(CUSTOM, custom, palette)
            .render()
            .padding(12.0),
        state.text("swatch-custom-open") == "open",
        "swatch-custom-open=closed",
        palette,
    )
    .width(244.0)
    .render();

    let sizes = [
        ("Dense", ControlSize::Dense),
        ("Compact", ControlSize::Compact),
        ("Regular", ControlSize::Regular),
        ("Spacious", ControlSize::Spacious),
    ];
    vec![
        section(cx, "Swatch picker", vec![
            row(vec![picker(cx, "swatch-accent", 8, "blue").render()]),
            caption(cx, &format!("Chosen: {}. The arrows move the choice; Home and End go to the ends.", state.choice("swatch-accent", "blue"))),
        ]),
        section(cx, "Custom color", vec![
            row(vec![with_custom]),
            caption(cx, &format!(
                "The swatch with a pencil opens a color picker; a color chosen there makes it the choice. Chosen: {}.",
                if state.choice("swatch-custom", "blue") == "custom" { described(custom) } else { state.choice("swatch-custom", "blue").to_owned() }
            )),
        ]),
        section(cx, "Rows", vec![
            row(vec![picker(cx, "swatch-rows", 10, "purple").columns(5).render()]),
            caption(cx, "Ten colors in rows of five. The arrows run on from the end of one row to the start of the next."),
        ]),
        section(
            cx,
            "Sizes",
            sizes
                .into_iter()
                .flat_map(|(name, size)| {
                    let group = format!("swatch-size-{}", name.to_lowercase());
                    [row(vec![picker(cx, &group, 6, "teal").size(size).render()]), caption(cx, name)]
                })
                .collect(),
        ),
        section(cx, "Disabled", vec![
            row(vec![picker(cx, "swatch-off", 6, "green").custom(Color::rgb(245, 215, 110), "Custom color").disabled().render()]),
            caption(cx, "A disabled picker shows every color and its choice, ringed in the muted ink, and takes no pointer or focus."),
        ]),
        section(cx, "Edge cases", vec![
            row(vec![picker(cx, "swatch-none", 6, "").render()]),
            caption(cx, "No swatch chosen until one is clicked."),
            row(vec![SwatchPicker::new(
                "swatch-plain",
                vec![
                    Swatch::new("swatch-plain=surface", palette.surface, "Background"),
                    Swatch::new("swatch-plain=text", palette.text, "Text"),
                ],
                Some(&format!("swatch-plain={}", state.choice("swatch-plain", "surface"))),
                palette,
            )
            .render()]),
            caption(cx, "A swatch of the page's own color keeps a hairline edge, so it still shows."),
        ]),
        section(cx, "Color picker", vec![
            row(vec![ColorPicker::new("color-main", color(state, "color-main"), palette).render()]),
            caption(cx, &format!(
                "{}. Drag in the field for saturation and value, along the bar for the hue, or type six or three hex digits.",
                described(color(state, "color-main"))
            )),
        ]),
        section(cx, "Width", vec![
            row(vec![ColorPicker::new("color-narrow", color(state, "color-narrow"), palette).width(160.0).render()]),
            caption(cx, &format!("160 wide; the field stays three quarters as high. {}.", described(color(state, "color-narrow")))),
        ]),
        section(cx, "Disabled picker", vec![
            row(vec![ColorPicker::new("color-off", Hsv::new(280.0, 0.6, 0.7), palette).disabled().render()]),
            caption(cx, "Disabled: the color shows, the thumbs are ringed in the muted ink, and nothing takes the pointer or focus."),
        ]),
        section(cx, "Part labels", vec![
            row(vec![ColorPicker::new("color-labels", color(state, "color-labels"), palette)
                .part_labels("Saturación y brillo", "Matiz", "Hexadecimal")
                .render()]),
            caption(cx, "The field, the bar and the hex field named to the platform in words of the host's own. Without part_labels they follow set_labels."),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_and_typed_hex_change_the_kept_color_and_the_field_follows() {
        let mut state = State::default();
        assert_eq!(fields(&state)[0], ("color-main:hex", "#3498db".to_owned()));
        update(
            &mut state,
            &UiAction::AreaSliderChanged {
                id: "color-main:area".into(),
                x: 1.0,
                y: 0.0,
                committed: true,
            },
        );
        assert_eq!(fields(&state)[0].1, "#0099ff");
        update(
            &mut state,
            &UiAction::InputChanged {
                id: "color-main:hex".into(),
                value: "#fa0".into(),
            },
        );
        assert_eq!(fields(&state)[0].1, "#ffaa00");
        // Half typed, the color stays.
        update(
            &mut state,
            &UiAction::InputChanged {
                id: "color-main:hex".into(),
                value: "#12".into(),
            },
        );
        assert_eq!(fields(&state)[0].1, "#ffaa00");
    }

    #[test]
    fn the_custom_swatch_opens_its_picker_and_a_color_from_it_becomes_the_choice() {
        let mut state = State::default();
        update(
            &mut state,
            &UiAction::Clicked {
                id: "swatch-custom:custom".into(),
            },
        );
        assert_eq!(state.text("swatch-custom-open"), "open");
        assert_eq!(state.choice("swatch-custom", "blue"), "blue");
        update(
            &mut state,
            &UiAction::SliderChanged {
                id: "swatch-custom-picker:hue".into(),
                fraction: 0.5,
                committed: false,
            },
        );
        assert_eq!(state.choice("swatch-custom", "blue"), "custom");
    }
}
