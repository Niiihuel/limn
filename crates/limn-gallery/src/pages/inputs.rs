use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{
    Color, Component, ControlSize, DropdownOption, Element, FormField, FormFieldSizing, Icon, PopoverPlacement, RadiusRole,
    SearchField, Select, SelectVariant, SelectorSizing, Slider, Switch, SwitchVariant, TextArea, TextField, TextRole, UiAction,
};

/// The search fields with a clear button, and the query each starts with.
const CLEARABLE: [(&str, &str); 2] = [
    ("input-search", ""),
    ("input-search-filled", "release notes"),
];

/// A clear button empties its field's query.
pub fn update(state: &mut State, action: &UiAction) {
    let UiAction::Clicked { id } = action else {
        return;
    };
    if let Some((field, _)) = CLEARABLE
        .iter()
        .find(|(field, _)| id.strip_suffix(":clear") == Some(field))
    {
        state.set(field, "");
    }
}

/// The search fields a clear button changes, each with its query now. A
/// field keeps the text typed in it, so after a clear the tree is given these.
pub fn fields(state: &State) -> Vec<(&'static str, String)> {
    CLEARABLE
        .iter()
        .map(|(field, start)| (*field, state.choice(field, start).to_owned()))
        .collect()
}

const SHELLS: [(&str, &str); 4] = [("bash", "Bash"), ("zsh", "Zsh"), ("fish", "Fish"), ("nu", "Nushell")];
// Each band's gain as a share of its slider: 0.5 is 0 dB.
const EQUALISER: [(&str, f32); 5] = [("60", 0.68), ("250", 0.58), ("1k", 0.5), ("4k", 0.38), ("12k", 0.72)];

/// A slider's value as the gallery keeps it, or `initial` before it is moved.
fn fraction(cx: &Cx, id: &str, initial: f32) -> f32 {
    cx.state.text(id).parse().unwrap_or(initial)
}

/// A select over the demo's shells; `group` keeps its choice and open state.
fn shell_select(cx: &Cx, group: &str) -> Select {
    let chosen = cx.state.choice(group, SHELLS[0].0);
    let label = SHELLS.iter().find(|(id, _)| *id == chosen).map_or(SHELLS[0].1, |(_, label)| label);
    let options = SHELLS
        .iter()
        .map(|(id, label)| {
            let option = DropdownOption::new(format!("{group}={id}&{group}-open=no"), *label)
                .selected(*id == chosen);
            if *id == "nu" {
                option.disabled()
            } else {
                option
            }
        })
        .collect();
    Select::new(
        format!("{group}-open=yes"),
        label,
        options,
        cx.state.text(&format!("{group}-open")) == "yes",
        format!("{group}-open=no"),
        cx.palette,
    )
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let width = cx.width.min(360.0);
    vec![
        section(cx, "Text field", vec![
            TextField::new("input-name", state.text("input-name"), "Project name", palette)
                .width(width)
                .render(),
            caption(cx, &format!("Value: {}", state.text("input-name"))),
            TextField::new("input-password", state.text("input-password"), "Password", palette)
                .password(true)
                .width(width)
                .render(),
            TextField::new("input-disabled", "Read-only", "", palette)
                .accessibility_label("License")
                .disabled()
                .width(width)
                .render(),
            TextField::new("input-path", state.text("input-path"), "src/main.rs", palette)
                .monospace(true)
                .width(width)
                .render(),
            caption(cx, "Monospaced, for paths and identifiers."),
            TextField::new("input-tall", state.text("input-tall"), "e.g. filesystem", palette)
                .size(ControlSize::Compact)
                .width(width)
                .height(40.0)
                .render(),
            caption(cx, "Form height: 40 px on a compact control."),
            Element::column(vec![
                TextField::new("input-channel", state.text("input-channel"), "new-channel", palette)
                    .icon(Icon::Hash)
                    .accessibility_label("Channel name")
                    .width(width)
                    .render(),
                TextField::new("input-channel-disabled", "general", "", palette)
                    .icon(Icon::Hash)
                    .accessibility_label("Channel name")
                    .disabled()
                    .width(width)
                    .render(),
            ])
            .gap(8.0),
            caption(cx, "With an icon before the text, muted until the field has the focus; disabled, it stays."),
        ]),
        section(cx, "Form field", vec![
            Element::column(vec![
                FormField::new(
                    "Name",
                    TextField::new("field-name", state.text("field-name"), "My project", palette).render(),
                    palette,
                )
                .render(),
                FormField::new(
                    "Executable",
                    TextField::new("field-program", state.text("field-program"), "/usr/bin/env", palette)
                        .monospace(true)
                        .render(),
                    palette,
                )
                .hint("Absolute path to an existing file. Runs without a shell.")
                .hint("The agent cannot change this path.")
                .hint_above(true)
                .render(),
                FormField::new(
                    "Path within the project",
                    TextField::new("field-path", state.text("field-path"), "src/new.rs", palette).render(),
                    palette,
                )
                .hint("Missing folders are created.")
                .error("A file already exists at this path.", Color::rgb(229, 83, 75))
                .render(),
                FormField::new(
                    "Instructions",
                    TextArea::new("field-notes", state.text("field-notes"), "Describe the task", palette)
                        .height(72.0)
                        .render(),
                    palette,
                )
                .hint("Muted label on an 18 px line, as in a setup wizard.")
                .sizing(FormFieldSizing {
                    label_muted: true,
                    label_height: 18.0,
                    hint_role: TextRole::Caption,
                    gap: 8.0,
                    ..FormFieldSizing::DEFAULT
                })
                .render(),
            ])
            .gap(16.0)
            .size(Some(width), None),
            caption(cx, "With `rows`, two fields share the form column, which sets their spacing."),
            Element::column({
                let mut rows = FormField::new(
                    "Program",
                    TextField::new("flat-program", state.text("flat-program"), "e.g. npx", palette).render(),
                    palette,
                )
                .hint("Starts without a shell.")
                .hint_above(true)
                .rows();
                rows.extend(
                    FormField::new(
                        "Working directory",
                        TextField::new("flat-cwd", state.text("flat-cwd"), "e.g. tools/mcp", palette).render(),
                        palette,
                    )
                    .rows(),
                );
                rows
            })
            .gap(7.0)
            .size(Some(width), None),
        ]),
        section(cx, "Select", vec![
            row(vec![
                shell_select(cx, "select-plain").render(),
                shell_select(cx, "select-off").enabled(false).render(),
            ]),
            caption(cx, "The list matches the button width. When disabled, it cannot open or receive focus."),
            row(vec![shell_select(cx, "select-compact")
                .sizing(SelectorSizing::COMPACT)
                .icon(Icon::Terminal)
                .placement(PopoverPlacement::BottomStart)
                .render()]),
            caption(cx, "With a sizing profile: compact rows and an icon, with the list at its preferred width."),
            shell_select(cx, "select-wide")
                .width(width)
                .placement(PopoverPlacement::BottomStart)
                .render(),
            caption(cx, "As wide as the form, with the list aligned below the leading edge."),
            Element::column(vec![
                TextField::new("select-field-name", state.text("select-field-name"), "Name", palette).render(),
                shell_select(cx, "select-field")
                    .variant(SelectVariant::Field)
                    .accessibility_label("Shell")
                    .width(width)
                    .menu_width(width)
                    .placement(PopoverPlacement::BottomStart)
                    .render(),
                shell_select(cx, "select-field-icon")
                    .variant(SelectVariant::Field)
                    .icon(Icon::Terminal)
                    .accessibility_label("Shell")
                    .width(width)
                    .menu_width(width)
                    .placement(PopoverPlacement::BottomStart)
                    .render(),
                shell_select(cx, "select-field-off")
                    .variant(SelectVariant::Field)
                    .accessibility_label("Shell")
                    .enabled(false)
                    .width(width)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
            caption(cx, "As a field: a text field's fill, border and height, the choice at its start; open, the border of a focused field. With an icon, and disabled."),
            Element::column(vec![
                TextField::new("select-tall-name", state.text("select-tall-name"), "Name", palette)
                    .size(ControlSize::Compact)
                    .height(40.0)
                    .render(),
                shell_select(cx, "select-field-tall")
                    .variant(SelectVariant::Field)
                    .accessibility_label("Shell")
                    .size(ControlSize::Compact)
                    .height(40.0)
                    .width(width)
                    .menu_width(width)
                    .placement(PopoverPlacement::BottomStart)
                    .render(),
            ])
            .gap(8.0)
            .size(Some(width), None),
            caption(cx, "Form height: 40 px, beside a text field of the same height."),
            shell_select(cx, "select-field-accent")
                .variant(SelectVariant::Field)
                .accessibility_label("Shell")
                .font_size(16.0)
                .max_visible_items(2)
                .ink(cx.state.theme().accent)
                .height(40.0)
                .width(width)
                .menu_width(width)
                .placement(PopoverPlacement::BottomStart)
                .render(),
            caption(cx, ".font_size(16.0), .max_visible_items(2) and .ink(accent): open, two rows show, the chevron points up and the border takes the accent."),
            Element::row(vec![
                Element::spacer().flex(1.0),
                shell_select(cx, "select-quiet")
                    .variant(SelectVariant::Plain)
                    .accessibility_label("Shell")
                    .placement(PopoverPlacement::BottomEnd)
                    .render(),
            ])
            .size(Some(width), None),
            caption(cx, "Plain: the choice and its chevron, filled only under the pointer, at the end of a settings row."),
        ]),
        section(cx, "Search", vec![
            SearchField::new("input-search", state.choice(CLEARABLE[0].0, CLEARABLE[0].1), "Search", palette)
                .shortcut("Ctrl+K")
                .clearable(true)
                .width(width)
                .render(),
            caption(cx, "The shortcut while it is empty; type, and a button at its end clears it."),
            Element::column(vec![
                SearchField::new("input-search-filled", state.choice(CLEARABLE[1].0, CLEARABLE[1].1), "Search", palette)
                    .clearable(true)
                    .width(width)
                    .render(),
                SearchField::new("input-search-disabled", "release notes", "Search", palette)
                    .clearable(true)
                    .disabled()
                    .width(width)
                    .render(),
            ])
            .gap(8.0),
            caption(cx, "With a query, and disabled: the button stays in its place and cannot be used."),
        ]),
        section(cx, "Text area", vec![
            TextArea::new("input-notes", state.text("input-notes"), "Notes", palette)
                .width(width)
                .height(120.0)
                .render(),
            caption(cx, ".enter_submits(true): Enter sends, Shift+Enter adds a line"),
            TextArea::new("input-notes-chat", state.text("input-notes-chat"), "Message #general", palette)
                .width(width)
                .height(64.0)
                .enter_submits(true)
                .render(),
            caption(cx, ".radius(RadiusRole::Surface), and disabled"),
            TextArea::new("input-notes-surface", state.text("input-notes-surface"), "Message", palette)
                .width(width)
                .height(64.0)
                .radius(RadiusRole::Surface)
                .render(),
            TextArea::new("input-notes-disabled", "", "No permission to write", palette)
                .width(width)
                .height(64.0)
                .radius(RadiusRole::Surface)
                .disabled()
                .render(),
            caption(cx, ".text_area_highlights: a mention in the accent over a fill, as the pill it becomes once sent"),
            {
                let value = state.choice("input-notes-mentions", "Ask @ada about #general before Friday");
                Element::text_area("input-notes-mentions", value, "Message")
                    .text_area_highlights(mentions(value, palette.link()))
                    .font_family(limn::FontFamily::Sans)
                    .foreground(palette.text)
                    .padding(10.0)
                    .background(palette.raised)
                    .radius(RadiusRole::Surface.px())
                    .size(Some(width), Some(44.0))
            },
        ]),
        section(cx, "Switch", vec![
            row(vec![
                Switch::new("input-switch", state.flag("input-switch"), palette).accessibility_label("Notifications").render(),
                Switch::new("input-switch-disabled", true, palette).accessibility_label("Automatic updates").disabled().render(),
            ]),
            caption(cx, "A switch, and one disabled: it keeps its value."),
            row([
                ("Dense", ControlSize::Dense),
                ("Compact", ControlSize::Compact),
                ("Regular", ControlSize::Regular),
                ("Spacious", ControlSize::Spacious),
            ]
            .into_iter()
            .map(|(name, size)| {
                let id = format!("input-switch-{}", name.to_lowercase());
                Switch::new(id.as_str(), state.flag(&id), palette).size(size).accessibility_label(name).render()
            })
            .collect()),
            caption(cx, ".size(ControlSize): Dense, Compact, Regular and Spacious."),
            row(vec![
                Switch::new("input-switch-accent", !state.flag("input-switch-accent"), palette)
                    .variant(SwitchVariant::Accent)
                    .size(ControlSize::Spacious)
                    .accessibility_label("Show embeds")
                    .render(),
                Switch::new("input-switch-accent-off", state.flag("input-switch-accent-off"), palette)
                    .variant(SwitchVariant::Accent)
                    .size(ControlSize::Spacious)
                    .accessibility_label("Autoplay GIFs")
                    .render(),
                Switch::new("input-switch-accent-ink", !state.flag("input-switch-accent-ink"), palette)
                    .variant(SwitchVariant::Accent)
                    .ink(cx.state.theme().accent)
                    .size(ControlSize::Spacious)
                    .accessibility_label("Compact mode")
                    .render(),
                Switch::new("input-switch-accent-disabled", true, palette)
                    .variant(SwitchVariant::Accent)
                    .size(ControlSize::Spacious)
                    .accessibility_label("Sync")
                    .disabled()
                    .render(),
            ]),
            caption(cx, "SwitchVariant::Accent: a light thumb on the link color, on the theme's accent with .ink, and disabled."),
            row(vec![
                Switch::new("input-switch-track", !state.flag("input-switch-track"), palette)
                    .variant(SwitchVariant::Accent)
                    .ink(cx.state.theme().accent)
                    .track_size(50.0, 26.0)
                    .accessibility_label("Developer mode")
                    .render(),
                Switch::new("input-switch-track-off", state.flag("input-switch-track-off"), palette)
                    .variant(SwitchVariant::Accent)
                    .track_size(50.0, 26.0)
                    .accessibility_label("Streamer mode")
                    .render(),
            ]),
            caption(cx, ".track_size(50.0, 26.0): a track of the host's own size, the thumb 3 px inside it."),
        ]),
        section(cx, "Slider", vec![
            Slider::new("input-slider", fraction(cx, "input-slider", 0.35), palette)
                .accessibility_label("Volume")
                .width(280.0)
                .render(),
            caption(cx, &format!("Continuous: {:.0} %", fraction(cx, "input-slider", 0.35) * 100.0)),
            Slider::new("input-slider-steps", fraction(cx, "input-slider-steps", 0.5), palette)
                .steps(5)
                .accessibility_label("Quality")
                .width(280.0)
                .render(),
            caption(cx, ".steps(5): snaps to one of five stops on release."),
            Slider::new("input-slider-disabled", 0.6, palette)
                .accessibility_label("Brightness")
                .enabled(false)
                .width(280.0)
                .render(),
            caption(cx, "Disabled: shows the value without pointer input or focus."),
            Slider::new("input-slider-marks", fraction(cx, "input-slider-marks", 0.25), palette)
                .steps(5)
                .marks()
                .accessibility_label("Zoom")
                .width(280.0)
                .render(),
            caption(cx, ".marks(): a hairline at each stop, behind the thumb."),
            Slider::new("input-slider-sizes", fraction(cx, "input-slider-sizes", 0.5), palette)
                .steps(7)
                .mark_labels(["12px", "14px", "15px", "16px", "18px", "20px", "24px"])
                .default_stop(3)
                .ink(cx.state.theme().accent)
                .accessibility_label("Font size")
                .width(280.0)
                .render(),
            caption(cx, ".mark_labels(..) names the stops and .default_stop(3) shows 16px in the ink."),
            Slider::new("input-slider-sizes-disabled", 0.5, palette)
                .steps(3)
                .mark_labels(["Small", "Medium", "Large"])
                .default_stop(1)
                .accessibility_label("Density")
                .enabled(false)
                .width(280.0)
                .render(),
            caption(cx, "Disabled, every label is muted."),
            {
                let threshold = fraction(cx, "input-slider-level", 0.4);
                let level = 0.55;
                let theme = cx.state.theme();
                Slider::new("input-slider-level", threshold, palette)
                    .ink(theme.accent)
                    .level(level, if level >= threshold { Color::rgb(35, 165, 90) } else { Color::rgb(240, 178, 50) })
                    .accessibility_label("Input sensitivity")
                    .width(280.0)
                    .render()
            },
            caption(cx, ".level(0.55, color): a microphone's input along the track, green over the threshold the thumb sets and yellow under it."),
            row(EQUALISER
                .iter()
                .map(|(band, gain)| {
                    let id = format!("input-slider-eq-{band}");
                    Slider::new(&id, fraction(cx, &id, *gain), palette)
                        .vertical()
                        .origin(0.5)
                        .accessibility_label(format!("{band} Hz"))
                        .width(120.0)
                        .render()
                })
                .collect()),
            caption(cx, ".vertical().origin(0.5): an equaliser's bands. Each stands up, 120 px tall, and fills from its middle, the 0 dB of a gain."),
        ]),
    ]
}

/// Each `@name` and `#name` of `value`, inked and filled in `accent`, by
/// character offsets.
fn mentions(value: &str, accent: Color) -> Vec<limn::TextHighlight> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, character) in value.chars().chain([' ']).enumerate() {
        match (start, character) {
            (None, '@' | '#') => start = Some(index),
            (Some(from), character) if !character.is_alphanumeric() && index > from + 1 => {
                found.push(limn::TextHighlight {
                    range: from..index,
                    color: accent,
                    italic: false,
                    background: Some(Color::rgba(accent.r, accent.g, accent.b, 40)),
                });
                start = None;
            }
            (Some(_), character) if !character.is_alphanumeric() => start = None,
            _ => {}
        }
    }
    found
}
