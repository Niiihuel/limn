use super::{caption, Cx, Section, titled};
use crate::glass_drag::{
    self, DEFAULT_X, DEFAULT_Y, HANDLE, HANDLE_GAP, HANDLE_HEIGHT, PREVIEW_HEIGHT,
};
use limn::{
    Button, ButtonVariant, Collapse, Color, Component, Element, FontFamily, GlassMaterial, Icon,
    NavigationAction, NavigationBar, NavigationWorkspace, Rect, Slider, Switch,
};

pub const BACKDROP: &str = "glass-demo-backdrop";

const WIFI: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0"/><circle cx="12" cy="20" r="1" fill="currentColor"/></svg>"#;
const BATTERY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="1" y="6" width="19" height="12" rx="2"/><path d="M22 10v4"/><rect x="4" y="9" width="13" height="6" rx=".5" fill="currentColor" stroke="none"/></svg>"#;
const CLOUD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M6 19a5 5 0 0 1-1-10 7 7 0 0 1 13-1 5.5 5.5 0 0 1 .5 11Z"/></svg>"#;
const POWER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M12 2v10M7 5a9 9 0 1 0 10 0"/></svg>"#;

fn value(cx: &Cx, key: &str, default: f32) -> f32 {
    glass_drag::fraction(cx.state, key, default)
}

fn slider(cx: &Cx, id: &str, label: &str, default: f32, maximum: f32) -> Element {
    let fraction = value(cx, id, default);
    let amount = if maximum == 100.0 {
        format!("{:.0}%", fraction * maximum)
    } else {
        format!("{:.1}", fraction * maximum)
    };
    let label_width = 135.0_f32.min(cx.width * 0.45);
    Element::row(vec![
        caption(cx, &format!("{label} · {amount}")).size(Some(label_width), None),
        Slider::new(id, fraction, cx.palette)
            .width((cx.width - label_width - 12.0).clamp(64.0, 340.0))
            .render(),
    ])
    .gap(12.0)
}

fn toggle(cx: &Cx, id: &str, label: &str) -> Element {
    Element::row(vec![
        Switch::new(id, cx.state.flag(id), cx.palette).render(),
        caption(cx, label),
    ])
    .gap(12.0)
}

fn controls(cx: &Cx) -> Element {
    let columns = if cx.width >= 900.0 {
        3
    } else if cx.width >= 620.0 {
        2
    } else {
        1
    };
    let column_width = (cx.width - (columns - 1) as f32 * 20.0) / columns as f32;
    let column = Cx {
        state: cx.state,
        palette: cx.palette,
        width: column_width,
    };
    let cx = &column;
    let palette = cx.palette;
    let position = titled(
        cx,
        "Position",
        vec![
            slider(cx, "glass-x", "Horizontal", DEFAULT_X, 100.0),
            slider(cx, "glass-y", "Vertical", DEFAULT_Y, 100.0),
            Button::new("glass-x=0.5&glass-y=0.25", "Reset position", palette)
                .variant(ButtonVariant::Outline)
                .render(),
            toggle(cx, "glass-compact", "Compact bar"),
        ],
    )
    .size(Some(column_width), None);

    let scenes = [
        ("amber", "Amber"),
        ("ocean", "Ocean"),
        ("grid", "Lines"),
        ("checker", "Checkerboard"),
        ("rings", "Rings"),
        ("content", "Content"),
    ];
    let scene_columns = if column_width < 280.0 { 2 } else { 3 };
    let scene_width = (column_width - (scene_columns - 1) as f32 * 8.0) / scene_columns as f32;
    let mut backgrounds: Vec<Element> = scenes
        .chunks(scene_columns)
        .map(|scenes| {
            Element::row(
                scenes
                    .iter()
                    .map(|(id, label)| {
                        Button::new(format!("glass-scene={id}"), *label, palette)
                            .variant(ButtonVariant::Outline)
                            .active(cx.state.choice("glass-scene", "amber") == *id)
                            .width(scene_width)
                            .render()
                    })
                    .collect(),
            )
            .gap(8.0)
        })
        .collect();
    let animate = cx.state.choice("glass-animate", "on") != "off";
    backgrounds.push(
        Element::row(vec![
            Switch::new("glass-animate", animate, palette)
                .render()
                .dispatch_as(if animate {
                    "glass-animate=off"
                } else {
                    "glass-animate=on"
                }),
            caption(cx, "Animate background"),
        ])
        .gap(12.0),
    );
    backgrounds.push(slider(cx, "glass-speed", "Speed", 0.5, 100.0));
    let backgrounds = titled(cx, "Background", backgrounds).size(Some(column_width), None);

    let mut material: Vec<_> = [
        ("glass-blur", "Blur", 0.125, 64.0),
        ("glass-refraction", "Refraction", 1.0, 32.0),
        ("glass-dispersion", "Dispersion", 0.2, 2.0),
        ("glass-tint", "Tint", 0.6, 100.0),
    ]
    .into_iter()
    .map(|(id, label, default, maximum)| slider(cx, id, label, default, maximum))
    .collect();
    material.push(toggle(cx, "glass-opaque", "Opaque surface"));
    material.push(toggle(cx, "tokens-reduced", "Reduced motion"));
    let material = titled(cx, "Material", material).size(Some(column_width), None);
    match columns {
        3 => Element::row(vec![position, backgrounds, material]).gap(20.0),
        2 => Element::column(vec![
            Element::row(vec![position, backgrounds]).gap(20.0),
            material,
        ])
        .gap(20.0),
        _ => Element::column(vec![position, backgrounds, material]).gap(20.0),
    }
}

fn material(cx: &Cx) -> GlassMaterial {
    let mut material = if cx.state.flag("light") {
        GlassMaterial::light()
    } else {
        GlassMaterial::dark()
    };
    // iOS 27 lets the user take glass from ultra clear to fully tinted.
    material.tint.a = (value(cx, "glass-tint", 0.6) * 255.0) as u8;
    material
        .blur(value(cx, "glass-blur", 0.125) * 64.0)
        .refraction(value(cx, "glass-refraction", 1.0) * 32.0)
        .dispersion(value(cx, "glass-dispersion", 0.2) * 2.0)
}

fn detail(cx: &Cx, width: f32) -> Option<Element> {
    let panel = cx.state.choice("glass-panel", "none");
    let (title, message) = match panel {
        "calendar" => (
            "Tuesday, October 13",
            "11:06 · Your next event is at 14:30.",
        ),
        "weather" => ("Córdoba · 15°", "Partly cloudy · High of 22°."),
        "network" => ("Connectivity", "Wi-Fi connected · Bluetooth available."),
        "battery" => (
            "Battery · 100%",
            "Plugged in · Fully charged.",
        ),
        "notifications" => ("Notifications", "You have two unread notifications."),
        "audio" => ("Mixer", "Output: speakers · Volume: 65%."),
        "system" => ("System", "CPU 12% · Memory 6.4 / 32 GB."),
        "containers" => ("Containers", "Two active services in the local environment."),
        "recorder" => ("Recording", "Ready to record a region of the screen."),
        "settings" => ("Settings", "Try the material with the controls below."),
        "power" => (
            "Session",
            "Demo view: system actions are not executed.",
        ),
        _ => return None,
    };
    let palette = cx.palette;
    // One solid panel keeps text legible without stacking another glass layer.
    Some(
        Element::column(vec![
            Element::row(vec![
                Element::text(title)
                    .font_size(16.0)
                    .font_family(FontFamily::Sans)
                    .foreground(palette.text)
                    .flex(1.0),
                Button::new("glass-panel=none", "Close", palette)
                    .variant(ButtonVariant::Ghost)
                    .render(),
            ]),
            Element::text(message)
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .foreground(palette.muted),
        ])
        .gap(8.0)
        .padding(16.0)
        .background(palette.surface)
        .border(palette.border, 1.0)
        .radius(limn::tokens().radius.surface)
        .size(Some(width.min(420.0)), None)
        .centered_x(),
    )
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width;
    let position = glass_drag::geometry(cx.state, Rect::new(0.0, 0.0, width, PREVIEW_HEIGHT));
    let bar_width = position.bar.width;
    let selected = cx.state.choice("glass-workspace", "5");
    let panel = cx.state.choice("glass-panel", "none");
    let actions = [
        ("weather", Icon::Custom(CLOUD), "Weather", "15°"),
        ("network", Icon::Custom(WIFI), "Connectivity", ""),
        ("battery", Icon::Custom(BATTERY), "Battery", "100%"),
        ("notifications", Icon::Bell, "Notifications", ""),
        ("audio", Icon::Sliders, "Mixer", ""),
        ("system", Icon::Laptop, "System", ""),
        ("containers", Icon::Server, "Containers", ""),
        ("recorder", Icon::Video, "Recording", ""),
        ("settings", Icon::Settings, "Settings", ""),
        ("power", Icon::Custom(POWER), "Session", ""),
    ]
    .into_iter()
    .map(|(key, icon, label, value)| {
        NavigationAction::new(format!("glass-panel={key}"), icon, label)
            .value(value)
            .active(panel == key)
    })
    .collect();
    let bar = NavigationBar::new("glass-nav", palette)
        .workspaces(
            (1..=5)
                .map(|number| {
                    NavigationWorkspace::new(
                        format!("glass-workspace={number}"),
                        format!("Workspace {number}"),
                    )
                    .selected(selected == number.to_string())
                    .occupied(number == 1 || number == 3)
                })
                .collect(),
        )
        .clock("11:06:46", "TUE OCT 13")
        .clock_action("glass-panel=calendar")
        .actions(actions)
        .accent(Color::rgb(244, 168, 123))
        .width(bar_width);
    let bar = if cx.state.flag("glass-opaque") {
        bar
    } else {
        bar.glass(material(cx))
    };
    let handle = Element::button(HANDLE, "Drag bar")
        .draggable_in_place()
        .font_family(FontFamily::Sans)
        .font_size(12.0)
        .foreground(palette.text)
        .background(Color {
            a: 190,
            ..palette.surface
        })
        .hover_background(palette.hover)
        .pressed_background(palette.selected)
        .transition_ms(limn::tokens().motion.hover_ms)
        .border(palette.border, 1.0)
        .radius(HANDLE_HEIGHT / 2.0)
        .center_content()
        .size(Some(146.0_f32.min(bar_width)), Some(HANDLE_HEIGHT));
    let movable = Element::column(vec![
        bar.render(),
        Element::spacer().size(None, Some(HANDLE_GAP)),
        handle.centered_x(),
    ])
    .size(
        Some(bar_width),
        Some(NavigationBar::HEIGHT + HANDLE_GAP + HANDLE_HEIGHT),
    );
    let preview = Element::stack(vec![
        Element::surface(BACKDROP, "Glass demo background"),
        Element::column(vec![
            Element::spacer().size(None, Some(position.bar.y)),
            Element::row(vec![
                Element::spacer().size(Some(position.bar.x), None),
                movable,
                Element::spacer().flex(1.0),
            ]),
            Element::spacer().flex(1.0),
        ]),
        Element::column(vec![
            Element::spacer().flex(1.0),
            Element::text("RICELIN  /  LIENZO")
                .font_family(FontFamily::Sans)
                .font_size(11.0)
                .foreground(Color::rgba(255, 255, 255, 210))
                .centered_x(),
            Element::spacer().size(None, Some(16.0)),
        ]),
    ])
    .size(Some(width), Some(PREVIEW_HEIGHT))
    .clip_children();

    vec![
        caption(cx, "Drag the bar across the background to explore the glass. Its controls still work; the data is simulated.").into(),
        preview.into(),
        Collapse::new("glass-detail", detail(cx, bar_width)).expand().render().into(),
        controls(cx).into(),
        caption(cx, "This demo uses its own background. Desktop integration is planned for a later stage.").into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{self, State};
    use limn::{DrawCommand, Point, Size, UiTree};

    fn heading(frame: &limn::Frame, label: &str) -> Point {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == label => Some(*position),
                _ => None,
            })
            .unwrap()
    }

    #[test]
    fn preview_and_all_adjustments_fit_on_a_wide_laptop_screen() {
        let state = State::starting_at("glass");
        let size = Size::new(1440.0, 830.0);
        let mut tree = UiTree::new(app::view(&state, size));
        tree.set_reduced_motion(true);
        let frame = tree.layout(size);
        let preview = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Surface { rect, id, .. } if id == BACKDROP => Some(*rect),
                _ => None,
            })
            .unwrap();
        assert_eq!(preview.height, 340.0);
        for id in [
            "glass-x",
            "glass-y",
            "glass-speed",
            "glass-blur",
            "glass-refraction",
            "glass-dispersion",
            "glass-tint",
            "glass-compact",
            "glass-animate",
            "glass-opaque",
            "tokens-reduced",
            "glass-scene=amber",
            "glass-scene=content",
        ] {
            let target = frame
                .regions
                .iter()
                .find(|region| region.id == id)
                .unwrap_or_else(|| panic!("{id} missing"));
            assert!(
                target.rect.y >= preview.y + preview.height,
                "{id} overlaps preview"
            );
            assert!(
                target.rect.y + target.rect.height <= size.height,
                "{id} is offscreen"
            );
        }
        let position = heading(&frame, "Position");
        let background = heading(&frame, "Background");
        let material = heading(&frame, "Material");
        assert_eq!((position.y, background.y), (background.y, material.y));
        assert!(position.x < background.x && background.x < material.x);
    }

    #[test]
    fn adjustments_use_two_columns_then_stack_as_the_page_narrows() {
        let state = State::default();
        for (width, columns) in [(760.0, 2), (360.0, 1)] {
            let cx = Cx {
                state: &state,
                palette: state.theme().palette(),
                width,
            };
            let frame = UiTree::new(controls(&cx)).layout(Size::new(width, 1000.0));
            let position = heading(&frame, "Position");
            let background = heading(&frame, "Background");
            let material = heading(&frame, "Material");
            if columns == 2 {
                assert_eq!(position.y, background.y);
                assert!(background.x > position.x && material.y > background.y);
            } else {
                assert!(position.y < background.y && background.y < material.y);
                assert_eq!((position.x, background.x), (background.x, material.x));
            }
            assert!(frame
                .regions
                .iter()
                .all(|region| region.rect.x >= 0.0 && region.rect.x + region.rect.width <= width));
        }
    }
}
