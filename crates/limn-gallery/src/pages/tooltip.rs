use super::{caption, row, section, Cx, Section};
use limn::{Button, ButtonVariant, Color, Component, FontFamily, Icon, IconButton, Tooltip, TooltipFace, TooltipPlacement};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let anchor = |id: &str, label: &str| {
        Button::new(id, label, palette).variant(ButtonVariant::Outline).render()
    };
    vec![
        caption(cx, "They appear after the pointer stays still for 350 ms, or immediately on keyboard focus.").into(),
        section(cx, "Title and shortcut", vec![row(vec![
            Button::new("tooltip-plain", "Title only", palette)
                .variant(ButtonVariant::Outline)
                .tooltip("Save changes")
                .render(),
            Button::new("tooltip-shortcut", "With shortcut", palette)
                .variant(ButtonVariant::Outline)
                .tooltip("Save changes")
                .tooltip_shortcut("Ctrl+S")
                .render(),
            IconButton::new("tooltip-icon", Icon::Refresh, palette)
                .tooltip("Refresh")
                .tooltip_shortcut("F5")
                .render(),
        ])]),
        section(cx, "With description", vec![row(vec![
            Tooltip::new(anchor("tooltip-rich", "Description"), "tooltip-rich", "Plan mode", palette)
                .shortcut("Shift+Tab")
                .description("The agent proposes steps and waits for your approval before editing files.")
                .separator()
                .footer("Available in any conversation")
                .render(),
        ])]),
        section(cx, "Faces", vec![row(vec![
            Tooltip::new(anchor("tooltip-faces", "In a call"), "tooltip-faces", "Rust workshop", palette)
                .placement(TooltipPlacement::Right)
                .faces(
                    Some(Icon::Volume),
                    [("Ada", (226, 99, 99)), ("Bruno", (88, 148, 226)), ("Carla", (214, 160, 96)), ("Dario", (168, 112, 214)), ("Elena", (96, 190, 170))]
                        .into_iter()
                        .map(|(name, (r, g, b))| TooltipFace::new(name, Color::rgb(r, g, b))),
                    4,
                )
                .faces(Some(Icon::Screen), [TooltipFace::new("Elena", Color::rgb(96, 190, 170))], 0)
                .render(),
        ])]),
        section(cx, "Title face", vec![row(vec![
            Tooltip::new(anchor("tooltip-sans", "Sans"), "tooltip-sans", "Rust workshop", palette)
                .placement(TooltipPlacement::Right)
                .render(),
            Tooltip::new(anchor("tooltip-semibold", "Semibold"), "tooltip-semibold", "Rust workshop", palette)
                .placement(TooltipPlacement::Right)
                .title_font_family(FontFamily::SansSemibold)
                .render(),
        ])]),
        section(cx, "Delay, title, pointer and fill", vec![
            row(vec![
                Tooltip::new(anchor("tooltip-at-once", "At once"), "tooltip-at-once", "Rust workshop", palette)
                    .placement(TooltipPlacement::Right)
                    .delay_ms(0)
                    .title_font_size(16.0)
                    .title_font_family(FontFamily::SansMedium)
                    .background(Color::rgb(10, 10, 12))
                    .render(),
                Tooltip::new(anchor("tooltip-no-arrow", "No pointer"), "tooltip-no-arrow", "Without a pointer", palette)
                    .arrow(false)
                    .render(),
                Tooltip::new(anchor("tooltip-slow", "After a second"), "tooltip-slow", "A slower hint", palette)
                    .delay_ms(1000)
                    .render(),
            ]),
            caption(cx, "A rail's hint opens at once with a 16 px title on a fill of its own; another hides its pointer or waits longer."),
        ]),
        section(cx, "Placement", vec![row(vec![
            Tooltip::new(anchor("tooltip-above", "Above"), "tooltip-above", "Above", palette)
                .placement(TooltipPlacement::Above)
                .render(),
            Tooltip::new(anchor("tooltip-below", "Below"), "tooltip-below", "Below", palette)
                .placement(TooltipPlacement::Below)
                .render(),
            Tooltip::new(anchor("tooltip-left", "Left"), "tooltip-left", "Left", palette)
                .placement(TooltipPlacement::Left)
                .render(),
            Tooltip::new(anchor("tooltip-right", "Right"), "tooltip-right", "Right", palette)
                .placement(TooltipPlacement::Right)
                .render(),
        ])]),
    ]
}
