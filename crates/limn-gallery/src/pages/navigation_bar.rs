use super::{caption, Cx, Section};
use limn::{
    Button, ButtonVariant, Collapse, Color, Component, Element, FontFamily, Icon, NavigationAction,
    NavigationBar, NavigationWorkspace,
};

const WIFI: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"><path d="M2 8a16 16 0 0 1 20 0M5 12a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0"/><circle cx="12" cy="20" r="1" fill="currentColor"/></svg>"#;
const BATTERY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"><rect x="1" y="6" width="19" height="12" rx="2"/><path d="M22 10v4"/><rect x="4" y="9" width="13" height="6" rx=".5" fill="currentColor" stroke="none"/></svg>"#;
const CLOUD: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7"><path d="M6 19a5 5 0 0 1-1-10 7 7 0 0 1 13-1 5.5 5.5 0 0 1 .5 11Z"/></svg>"#;
const POWER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M12 2v10M7 5a9 9 0 1 0 10 0"/></svg>"#;

fn detail(cx: &Cx, width: f32) -> Option<Element> {
    let panel = cx.state.choice("nav-panel", "none");
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
        "settings" => ("Settings", "Preferences would open here."),
        "power" => (
            "Session",
            "Demo view: system actions are not executed.",
        ),
        _ => return None,
    };
    let palette = cx.palette;
    Some(
        Element::column(vec![
            Element::row(vec![
                Element::text(title)
                    .font_size(16.0)
                    .font_family(FontFamily::Sans)
                    .foreground(palette.text)
                    .flex(1.0),
                Button::new("nav-panel=none", "Close", palette)
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
    let selected = cx.state.choice("nav-workspace", "5");
    let panel = cx.state.choice("nav-panel", "none");
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
        NavigationAction::new(format!("nav-panel={key}"), icon, label)
            .value(value)
            .active(panel == key)
    })
    .collect();
    let bar = NavigationBar::new("desktop", palette)
        .workspaces(
            (1..=5)
                .map(|number| {
                    NavigationWorkspace::new(
                        format!("nav-workspace={number}"),
                        format!("Workspace {number}"),
                    )
                    .selected(selected == number.to_string())
                    .occupied(number == 1 || number == 3)
                })
                .collect(),
        )
        .clock("11:06:46", "TUE OCT 13")
        .clock_action("nav-panel=calendar")
        .actions(actions)
        .accent(Color::rgb(244, 168, 123))
        .width(width);

    vec![
        caption(cx, "Its controls work; the data is simulated. A bar narrower than its content drops the date and the values and scrolls.").into(),
        bar.render().into(),
        Collapse::new("nav-detail", detail(cx, width)).expand().render().into(),
    ]
}
