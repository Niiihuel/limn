use super::{section, Cx, Section};
use limn::{
    AlertColors, AlertTone, Button, ButtonSize, ButtonVariant, Component, ComponentPalette, Element, Empty,
    Icon, Notice, NoticeSizing,
};

pub fn view(cx: &Cx) -> Vec<Section> {
    let width = cx.width.min(720.0);
    // A framed notice takes its fill from the palette it is given.
    let raised = ComponentPalette { surface: cx.palette.raised, ..cx.palette };
    let bar = |status: &str, failed: bool| {
        let notice = Notice::new(status, raised)
            .trailing(
                Button::new(format!("notice-restart-{failed}"), "Restart", cx.palette)
                    .variant(ButtonVariant::Ghost)
                    .render(),
            )
            .framed()
            .sizing(BAR)
            .width(width.min(420.0));
        if failed {
            notice.ink(AlertColors::for_tone(AlertTone::Error, cx.palette).accent).render()
        } else {
            notice.render()
        }
    };
    vec![
        section(cx, "With icon and action", vec![
            Empty::new(
                "Welcome to the wiki",
                "Wikis organize the project roadmap, show its status and document the software.",
                cx.palette,
            )
            .icon(Icon::Book)
            .action("empty-create", "Create the first page", ButtonVariant::Default)
            .width(width)
            .render(),
        ]),
        section(cx, "Two actions", vec![
            Empty::new("No projects", "Create a project or import an existing one.", cx.palette)
                .icon(Icon::Folder)
                .action("empty-new", "Create project", ButtonVariant::Default)
                .action("empty-import", "Import", ButtonVariant::Outline)
                .width(width)
                .render(),
        ]),
        section(cx, "Compact", vec![
            Empty::new("No results", "Try another term.", cx.palette)
                .icon(Icon::Search)
                .compact()
                .width(width.min(360.0))
                .render(),
        ]),
        section(cx, "Framed", vec![
            Empty::new("No plugins yet", "Choose a plugin folder to install.", cx.palette)
                .icon(Icon::Puzzle)
                .compact()
                .framed()
                .width(width)
                .render(),
        ]),
        section(cx, "Title only", vec![
            Empty::new("Nothing here", "", cx.palette).width(width.min(360.0)).render(),
        ]),
        section(cx, "Inline notice", vec![
            Notice::new("No skills installed yet", raised)
                .icon(Icon::Grid)
                .description("Choose one from the catalog and where to install it.")
                .action(Element::row(vec![
                    Button::new("notice-explore", "Explore catalog", cx.palette)
                        .variant(ButtonVariant::Outline)
                        .button_size(ButtonSize::Small)
                        .render(),
                    Element::spacer().flex(1.0),
                ]))
                .framed()
                .width(width)
                .render(),
            Notice::new("No results", cx.palette)
                .icon(Icon::Search)
                .description("Try another search or change the filter.")
                .width(width)
                .render(),
        ]),
        section(cx, "Stacked notice", vec![
            Notice::new("Your agent, beside your code", cx.palette)
                .icon(Icon::Chat)
                .description("Explore the project, request changes or complete a task.")
                .stacked()
                .width(width.min(360.0))
                .render(),
        ]),
        section(cx, "Compact notice", vec![
            Notice::new("No problems in this file", cx.palette)
                .icon(Icon::CheckCircle)
                .description("Diagnostics are up to date.")
                .sizing(NoticeSizing::COMPACT)
                .width(width.min(360.0))
                .render(),
            Notice::new("Empty folder", cx.palette)
                .description("Open a project to see its files. Text wraps across multiple lines when the panel is narrow.")
                .action(
                    Button::new("notice-reload", "Reload", cx.palette)
                        .variant(ButtonVariant::Ghost)
                        .button_size(ButtonSize::Small)
                        .render(),
                )
                .sizing(NoticeSizing::COMPACT)
                .width(width.min(240.0))
                .render(),
        ]),
        section(cx, "Fixed-height notice", vec![
            bar("The process ended with an error that does not fit on one line and is truncated with an ellipsis", true),
            bar("Process finished · exit code 0", false),
        ]),
    ]
}

/// A one-line bar: the title cut and centered beside its action.
const BAR: NoticeSizing = NoticeSizing {
    padding: 4.0,
    height: Some(40.0),
    title_height: Some(32.0),
    ..NoticeSizing::COMPACT
};
