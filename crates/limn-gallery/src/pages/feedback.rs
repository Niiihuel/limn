use super::{caption, row, section, Cx, Section};
use limn::{
    Alert, AlertTone, Avatar, Chip, Collapse, Color, Component, ControlSize, Element, FontFamily, Icon, Kbd, Button, ButtonVariant, Marker, MarkerVariant, Progress, Skeleton, SkeletonGroup,
    Spinner, SpinnerVariant, StatusDot, Ticker,
};

const CHIP_TONES: [(AlertTone, &str); 5] = [
    (AlertTone::Neutral, "Neutral"),
    (AlertTone::Info, "Information"),
    (AlertTone::Success, "Success"),
    (AlertTone::Warning, "Warning"),
    (AlertTone::Error, "Error"),
];
/// The tags of the removable demo, before any is removed.
const CHIP_TAGS: &str = "rust,urgent,review";
/// The tags of the wrapping demo, before any is removed.
const CHIP_WRAP_TAGS: &str = "rust,review,urgent,good first issue,ui,layout";
/// Roles of a member, in their colors, for the wrapping demo.
const CHIP_ROLES: [(&str, Color); 8] = [
    ("Admin", Color::rgb(230, 76, 60)),
    ("Moderator", Color::rgb(52, 152, 219)),
    ("Rust", Color::rgb(222, 120, 60)),
    ("Design", Color::rgb(138, 92, 246)),
    ("Night owls", Color::rgb(20, 160, 150)),
    ("Events", Color::rgb(241, 196, 15)),
    ("Verified", Color::rgb(46, 204, 113)),
    ("Helpers", Color::rgb(155, 89, 182)),
];

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
    let roles = || {
        CHIP_ROLES
            .iter()
            .map(|(label, color)| Chip::new(*label, palette).color(*color).dot().render())
            .collect()
    };
    let boxed = |content: Element, side: f32| {
        Element::panel(content)
            .padding(8.0)
            .background(palette.raised)
            .radius(limn::RadiusRole::Card.px())
            .size(Some(width.min(side)), None)
    };
    // As in the Labels demo, each remove button chooses the list without its own tag.
    let shown: Vec<&str> = cx
        .state
        .choice("chip-wrap-tags", CHIP_WRAP_TAGS)
        .split(',')
        .filter(|tag| !tag.is_empty())
        .collect();
    let mut wrap_tags: Vec<Element> = shown
        .iter()
        .map(|tag| {
            let rest: Vec<&str> = shown.iter().copied().filter(|other| other != tag).collect();
            let chip = Chip::new(*tag, palette)
                .remove_action(format!("chip-wrap-tags={}", rest.join(",")));
            if *tag == "urgent" {
                chip.tone(AlertTone::Warning).disabled()
            } else {
                chip
            }
            .render()
        })
        .collect();
    wrap_tags.push(
        Button::new(
            format!("chip-wrap-tags={CHIP_WRAP_TAGS}"),
            "Restore",
            palette,
        )
        .variant(ButtonVariant::Outline)
        .size(ControlSize::Dense)
        .render(),
    );
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
            row(vec![
                Progress::new(progress, palette).id("feedback-progress-ring").ring(16.0).render().accessibility_label("Context used").centered_y(),
                Progress::new(progress, palette).id("feedback-progress-ring-large").ring(32.0).render().accessibility_label("Context used"),
            ]),
            caption(cx, "Ring: the share used of a budget, such as a model's context."),
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
        section(cx, "Status dot", {
            // Status inks are the host's to choose; the gallery has no theme of its own.
            let (green, amber, red) = (Color::rgb(64, 192, 87), Color::rgb(230, 162, 60), Color::rgb(230, 76, 60));
            let pulsing = cx.state.text("feedback-dot-pulse") != "off";
            let labelled = |dot: StatusDot, label: &str| {
                Element::row(vec![dot.render().centered_y(), Element::text(label).font_family(FontFamily::Sans).foreground(palette.text)]).gap(8.0)
            };
            vec![
                row(vec![
                    labelled(StatusDot::new(green).pulse(pulsing).id("feedback-dot-running").description("Running"), "Running"),
                    labelled(StatusDot::new(amber).id("feedback-dot-waiting").description("Waiting"), "Waiting"),
                    labelled(StatusDot::new(red).id("feedback-dot-failed").description("Failed"), "Failed"),
                    labelled(StatusDot::new(green).pulse(pulsing).size(10.0), "10 px"),
                ]),
                row(vec![Button::new(
                    if pulsing { "feedback-dot-pulse=off" } else { "feedback-dot-pulse=on" },
                    if pulsing { "Stop the pulse" } else { "Pulse" },
                    palette,
                )
                .variant(ButtonVariant::Outline)
                .render()]),
                caption(cx, "A pulsing dot fades to 0.3 and back every 1.2 s, in step with every other; it holds still under reduced motion."),
            ]
        }),
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
        section(cx, "Labels", vec![
            row(vec![
                Chip::new("rust", palette).render(),
                Chip::new("draft", palette).render(),
                Chip::new("Pro plan", palette).outline().render(),
                Avatar::new("Ada Lovelace", palette).size(32.0).render(),
                Kbd::new("Ctrl+Shift+P", palette).render(),
            ]),
            row(CHIP_TONES.iter().map(|(tone, label)| Chip::new(*label, palette).tone(*tone).render()).collect()),
            row(CHIP_TONES.iter().map(|(tone, label)| Chip::new(*label, palette).tone(*tone).outline().render()).collect()),
            caption(cx, "A tone takes the color an alert of that tone gives its icon. Outlined, it is the edge and the label."),
            row(vec![
                Chip::new("Design", palette).color(Color::rgb(138, 92, 246)).render(),
                Chip::new("Research", palette).color(Color::rgb(20, 160, 150)).render(),
                Chip::new("Design", palette).color(Color::rgb(138, 92, 246)).outline().render(),
                Chip::new("Research", palette).color(Color::rgb(20, 160, 150)).outline().render(),
            ]),
            caption(cx, "A color of the host's own is drawn as a tone is."),
            row(vec![
                Chip::new("Dense", palette).size(ControlSize::Dense).render().centered_y(),
                Chip::new("Compact", palette).size(ControlSize::Compact).render().centered_y(),
                Chip::new("Regular", palette).size(ControlSize::Regular).render().centered_y(),
                Chip::new("Spacious", palette).size(ControlSize::Spacious).render().centered_y(),
                Chip::new("3", palette).size(ControlSize::Dense).render().centered_y(),
                Chip::new("3", palette).size(ControlSize::Spacious).render().centered_y(),
            ]),
            row(vec![
                Chip::new("Online", palette).tone(AlertTone::Success).dot().render(),
                Chip::new("Failed", palette).tone(AlertTone::Error).dot().outline().render(),
                Chip::new("Idle", palette).dot().render(),
                Chip::new("main", palette).icon(Icon::GitBranch).render(),
                Chip::new("Verified", palette).tone(AlertTone::Info).icon(Icon::Check).render(),
            ]),
            caption(cx, "A dot or an icon before the label takes the label's color."),
            row({
                // The tags still shown are one choice; each remove button chooses the list without its own.
                let shown: Vec<&str> = cx.state.choice("chip-tags", CHIP_TAGS).split(',').filter(|tag| !tag.is_empty()).collect();
                let mut tags: Vec<Element> = shown
                    .iter()
                    .map(|tag| {
                        let rest: Vec<&str> = shown.iter().copied().filter(|other| other != tag).collect();
                        let chip = Chip::new(*tag, palette).remove_action(format!("chip-tags={}", rest.join(",")));
                        match *tag {
                            "urgent" => chip.tone(AlertTone::Warning),
                            "review" => chip.outline(),
                            _ => chip,
                        }
                        .render()
                        .centered_y()
                    })
                    .collect();
                tags.push(Button::new(format!("chip-tags={CHIP_TAGS}"), "Restore", palette).variant(ButtonVariant::Outline).size(ControlSize::Dense).render());
                tags
            }),
            caption(cx, "The remove button reports its ID; the host drops the tag. Tab reaches it, Enter and Space press it."),
            row(vec![
                Chip::new("rust", palette).remove_action("chip-disabled-rust").disabled().render(),
                Chip::new("urgent", palette).tone(AlertTone::Warning).remove_action("chip-disabled-urgent").disabled().render(),
                Chip::new("Offline", palette).dot().outline().disabled().render(),
            ]),
            caption(cx, "Disabled: muted, and the remove button keeps its place without answering."),
            row(vec![
                Chip::new("7", palette).render(),
                Chip::new("128", palette).render(),
                Chip::new("Reparaci\u{f3}n \u{b7} a label that is long for a chip", palette).tone(AlertTone::Info).remove_action("chip-long").render(),
            ]),
            caption(cx, "One digit alone is a circle. A chip is as wide as its label; it does not truncate."),
        ]),
        section(cx, "Wrapping", vec![
            boxed(Element::row(roles()).gap(6.0).wrap(), 240.0),
            caption(cx, "Element::row(chips).gap(6).wrap(): a chip that does not fit starts the next line, and the box grows with its lines."),
            boxed(Element::row(roles()).gap(6.0).line_gap(12.0).wrap(), 360.0),
            caption(cx, "line_gap(12) spaces the lines apart from the chips; without it they are gap apart."),
            boxed(Element::row(wrap_tags).gap(6.0).wrap(), 240.0),
            caption(cx, "Removing a tag pulls the ones after it back; Tab walks the remove buttons line after line. The disabled chip keeps its place."),
        ]),
    ]
}
