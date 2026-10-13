use super::{Cx, Section};
use limn::{
    ActionRow, ActionRowSizing, Button, ButtonVariant, Chip, Component, Element, Icon, Kbd, PreferencesGroup,
    Switch,
};

/// A note's density: a smaller icon column and no minimum height.
const NOTE: ActionRowSizing = ActionRowSizing {
    icon_column: 16.0,
    copy_gap: 5.0,
    min_height: 0.0,
    min_height_with_subtitle: 0.0,
    ..ActionRowSizing::DEFAULT
};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let toggle = |id: &str| Switch::new(id, state.flag(id), palette).render();
    let plugin = |id: &str, name: &str, skills: &str| {
        ActionRow::new(name, palette).subtitle(skills).trailing(toggle(id)).render()
    };
    vec![
        PreferencesGroup::new(
            "Plugins",
            vec![
                ActionRow::new("Plugin skills", palette)
                    .icon(Icon::InfoCircle)
                    .subtitle("When a plugin is enabled, the agent receives each skill's name and description and can read its files when a task needs them.")
                    .render(),
                plugin("preferences-agent-skills", "agent-skills", "25 skills"),
                plugin("preferences-ponytail", "ponytail", "6 skills"),
                plugin("preferences-vercel", "vercel", "37 skills"),
            ],
            palette,
        )
        .description("Instructions installed for the agent.")
        .render().into(),
        PreferencesGroup::new(
            "Marketplace",
            vec![
                ActionRow::new("Installed skills", palette)
                    .subtitle("Instructions available for a user or a project.")
                    .trailing(Chip::new("0 active", palette).render())
                    .render(),
                ActionRow::new("Browse and manage", palette)
                    .subtitle("Find skills, add sources and manage installations.")
                    .trailing(
                        Button::new("preferences-marketplace", "Open Marketplace", palette)
                            .variant(ButtonVariant::Outline)
                            .icon(Icon::ArrowRight)
                            .render(),
                    )
                    .render(),
            ],
            palette,
        )
        .render().into(),
        PreferencesGroup::new(
            "Values, notes and stacked rows",
            vec![
                ActionRow::new("Conversation context", palette)
                    .subtitle("Latest percentage of remaining context.")
                    .value("82% remaining")
                    .render(),
                ActionRow::new("Only CLIs started from the application", palette)
                    .subtitle("Each session gets its own access, which expires when it closes. A CLI started elsewhere cannot connect.")
                    .icon(Icon::Terminal)
                    .icon_muted(true)
                    .align_top(true)
                    .wrap_title(true)
                    .sizing(NOTE)
                    .render(),
                ActionRow::new("Working folder for background agents", palette)
                    .subtitle("Stacked: the control moves below the text and the title wraps instead of being cut off.")
                    .wrap_title(true)
                    .align_top(true)
                    .trailing(
                        Button::new("preferences-folder", "Choose folder", palette)
                            .variant(ButtonVariant::Outline)
                            .render(),
                    )
                    .stacked(true)
                    .render(),
                ActionRow::new("A shortcut beside text that wraps onto several lines in a narrow column: the key stays on the first line", palette)
                    .wrap_title(true)
                    .trailing(Kbd::new("Ctrl+K", palette).render())
                    .trailing_top(true)
                    .render(),
                ActionRow::new("Reduce motion", palette)
                    .subtitle("Also stacked, but a switch stays on the same line.")
                    .trailing(toggle("preferences-motion"))
                    .stacked(true)
                    .render(),
            ],
            palette,
        )
        .render().into(),
        PreferencesGroup::new(
            "Rows that act as actions",
            vec![
                ActionRow::new("Account", palette)
                    .icon(Icon::Users)
                    .subtitle("Session, plan and billing.")
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-account")
                    .render(),
                ActionRow::new("Keyboard shortcuts", palette)
                    .icon(Icon::Keyboard)
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-shortcuts")
                    .render(),
                ActionRow::new("Data", palette)
                    .icon(Icon::Archive)
                    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
                    .action("preferences-data")
                    .render(),
            ],
            palette,
        )
        .render().into(),
    ]
}
