use super::{caption, row, section, Cx, Section};
use limn::{
    Alert, AlertTone, Avatar, Chip, Collapse, Component, Element, FontFamily, Icon, Kbd, Button, ButtonVariant, Marker, MarkerVariant, Progress, Skeleton, SkeletonGroup,
    Spinner, SpinnerVariant, Ticker,
};

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let alert = |tone: AlertTone, title: &str, description: &str| {
        // An ID makes the alert itself known to a screen reader, not only its controls.
        let id = format!("feedback-alert-{tone:?}");
        Element::panel(Alert::new(title, description, palette).tone(tone).id(id).render()).size(Some(width), None)
    };
    let percent: u32 = cx.state.choice("feedback-progress", "66").parse().unwrap_or(66);
    let progress = percent as f32 / 100.0;
    const STEPS: [&str; 4] = [
        "Searching **/*.{md,txt} in budgeting",
        "Reading README.md",
        "Reading a-file-with-a-very-long-name-that-does-not-fit-in-the-available-width.md",
        "Listing src/components",
    ];
    let step: usize = cx.state.choice("feedback-ticker", "0").parse().unwrap_or(0) % STEPS.len();
    let collapse_open = cx.state.choice("feedback-collapse", "open") == "open";
    vec![
        section(cx, "Alerts", vec![
            alert(AlertTone::Neutral, "Note", "A notice without urgency."),
            alert(AlertTone::Info, "Information", "A new version is available."),
            alert(AlertTone::Success, "Done", "Changes were saved."),
            alert(AlertTone::Warning, "Warning", "There are unsaved changes."),
            alert(AlertTone::Error, "Payment declined", "The payment could not be processed. Check your payment method and try again."),
        ]),
        section(cx, "Alerts with actions", vec![
            Element::panel(
                Alert::new("Dark mode is now available", "Enable it in your profile settings.", palette)
                    .id("feedback-alert-dark-mode")
                    .action(Button::new("feedback-alert-enable", "Enable", palette).variant(ButtonVariant::Outline).render())
                    .render(),
            )
            .size(Some(width), None),
            if cx.state.flag("feedback-alert-dismiss") {
                Element::row(vec![
                    Button::new("feedback-alert-dismiss", "Show again", palette)
                        .variant(ButtonVariant::Outline)
                        .render(),
                    Element::spacer().flex(1.0),
                ])
            } else {
                Element::panel(
                    Alert::new("Your subscription expires in 3 days", "Renew it to keep access.", palette)
                        .tone(AlertTone::Warning)
                        .id("feedback-alert-subscription")
                        .dismiss("feedback-alert-dismiss")
                        .render(),
                )
                .size(Some(width), None)
            },
        ]),
        section(cx, "Progress", vec![
            Progress::new(progress, palette)
                .id("feedback-progress-bar")
                .width(width.min(336.0))
                .render()
                .accessibility_label("Upload"),
            row(vec![
                Button::new(format!("feedback-progress={}", (percent + 85) % 100), "Back", palette)
                    .variant(ButtonVariant::Outline)
                    .render(),
                Button::new(format!("feedback-progress={}", (percent + 15).min(100)), "Forward", palette)
                    .variant(ButtonVariant::Outline)
                    .render(),
            ]),
            Progress::new(1.0, palette)
                .id("feedback-progress-done")
                .width(width.min(336.0))
                .height(8.0)
                .render()
                .accessibility_label("Download"),
            Progress::new(progress, palette)
                .id("feedback-progress-flat")
                .width(width.min(336.0))
                .flat()
                .render()
                .accessibility_label("Disk space used"),
            caption(cx, "Flat: square ends for a meter in a dense list."),
        ]),
        section(cx, "Loading", vec![
            row(vec![
                Spinner::new("feedback-spinner", palette.text).size(24.0).render(),
                Spinner::new("feedback-spokes", palette.text)
                    .size(24.0)
                    .variant(SpinnerVariant::Spokes)
                    .render(),
            ]),
            row(vec![
                Skeleton::circle("feedback-skeleton-avatar", palette, 40.0).render(),
                Element::column(vec![
                    Skeleton::line("feedback-skeleton-title", palette, 180.0).render(),
                    Skeleton::line("feedback-skeleton-text", palette, 260.0).render(),
                ])
                .gap(8.0),
            ]),
            Element::panel(
                SkeletonGroup::rows("feedback-skeleton-rows", palette, 4).icon(true).render(),
            )
            .size(Some(width.min(336.0)), None),
        ]),
        section(cx, "Markers", vec![
            Marker::new("feedback-marker", "Branch created", palette).icon(Icon::GitBranch).render(),
            Marker::new("feedback-marker-loading", "Running tests", palette).loading(true).render(),
            Marker::new("feedback-marker-thinking", "Thinking", palette).thinking(true).render(),
            Element::panel(
                Marker::new("feedback-marker-separator", "Today", palette)
                    .variant(MarkerVariant::Separator)
                    .render(),
            )
            .size(Some(width), None),
        ]),
        section(cx, "Ticker", vec![
            Element::panel(
                Ticker::new(
                    "feedback-ticker-line",
                    STEPS[step],
                    Element::truncated_text(STEPS[step])
                        .foreground(palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(13.0)
                        .size(None, Some(20.0)),
                )
                .height(40.0)
                .render(),
            )
            .size(Some(width.min(336.0)), None),
            row(vec![Button::new(format!("feedback-ticker={}", (step + 1) % STEPS.len()), "Next step", palette)
                .variant(ButtonVariant::Outline)
                .render()]),
            caption(cx, "The slot always measures two lines: content below it stays in place."),
        ]),
        section(cx, "Closing slot", vec![
            // No gap around the slot: a gap would vanish in one frame at the end.
            Element::column(vec![
                Element::text("Exploring the project").foreground(palette.text).font_family(FontFamily::Sans).font_size(13.0),
                Collapse::new(
                    "feedback-collapse-slot",
                    collapse_open.then(|| {
                        Element::text("Reading README.md\nListing src/components")
                            .foreground(palette.muted)
                            .font_family(FontFamily::Sans).font_size(13.0)
                            .padding(6.0)
                    }),
                )
                .expand()
                .render(),
                Element::text("Following content moves with the slot.").foreground(palette.text).font_family(FontFamily::Sans).font_size(13.0),
            ])
            .size(Some(width.min(336.0)), None),
            row(vec![Button::new(
                format!("feedback-collapse={}", if collapse_open { "closed" } else { "open" }),
                if collapse_open { "Remove content" } else { "Show content" },
                palette,
            )
            .variant(ButtonVariant::Outline)
            .render()]),
            caption(cx, "When removed, its last image fades while the slot closes."),
        ]),
        section(cx, "Labels", vec![row(vec![
            Chip::new("rust", palette).render(),
            Chip::new("draft", palette).render(),
            Chip::new("Pro plan", palette).outline().render(),
            Avatar::new("Ada Lovelace", palette).size(32.0).render(),
            Kbd::new("Ctrl+Shift+P", palette).render(),
        ])]),
    ]
}
