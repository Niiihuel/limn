use super::{caption, row, section, Cx, Section};
use limn::{
    ActiveHover, Button, ButtonGroup, ButtonGroupSizing, ButtonSize, ButtonSizing, ButtonVariant, Clipboard,
    Color, Component, ControlSize, Element, Icon, IconButton, RadiusRole, TooltipPlacement, TooltipSpec,
};

/// A host's own densities: a navigation button with a larger label, and a
/// close button for a 20 px tab.
const NAVIGATION: ButtonSizing = ButtonSizing {
    height: 32.0,
    font_size: 14.0,
    icon_size: 16.0,
    horizontal_padding: 5.0,
    inner_spacing: 6.0,
};
const TAB_CLOSE: ButtonSizing = ButtonSizing {
    height: 20.0,
    horizontal_padding: 2.0,
    ..NAVIGATION
};
const WIDE_GROUP: ButtonGroupSizing = ButtonGroupSizing {
    gap: 12.0,
    stacked_gap: 10.0,
};

const VARIANTS: [(ButtonVariant, &str); 11] = [
    (ButtonVariant::Default, "Default"),
    (ButtonVariant::Secondary, "Secondary"),
    (ButtonVariant::Outline, "Outline"),
    (ButtonVariant::Ghost, "Ghost"),
    (ButtonVariant::Subtle, "Subtle"),
    (ButtonVariant::Raised, "Raised"),
    (ButtonVariant::Solid, "Solid"),
    (ButtonVariant::Link, "Link"),
    (ButtonVariant::Success, "Success"),
    (ButtonVariant::Warning, "Warning"),
    (ButtonVariant::Destructive, "Destructive"),
];

// A status ink is the host's to choose; the gallery has no theme of its own.
const DANGER: Color = Color::rgb(255, 92, 92);

fn group(prefix: &str, palette: limn::ComponentPalette) -> Vec<Element> {
    vec![
        Button::new(format!("{prefix}-cancel"), "Cancel", palette)
            .variant(ButtonVariant::Outline)
            .render(),
        Button::new(format!("{prefix}-save"), "Save", palette).render(),
    ]
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let variants = |prefix: &str, build: fn(Button) -> Button| {
        // Two rows, so the page also fits a narrow window.
        Element::column(
            VARIANTS
                .chunks(6)
                .map(|chunk| {
                    row(chunk
                        .iter()
                        .map(|(variant, name)| {
                            build(Button::new(format!("{prefix}-{name}"), *name, palette).variant(*variant))
                                .render()
                        })
                        .collect())
                })
                .collect(),
        )
        .gap(12.0)
    };
    vec![
        section(cx, "Variants", vec![variants("button", |button| button)]),
        section(cx, "Disabled", vec![variants("button-disabled", Button::disabled)]),
        section(cx, "Loading", vec![variants("button-loading", |button| button.loading(true))]),
        section(cx, "Sizes", vec![row(vec![
            Button::new("button-mini", "More", palette)
                .variant(ButtonVariant::Ghost)
                .button_size(ButtonSize::Mini)
                .render(),
            Button::new("button-small", "Small", palette).button_size(ButtonSize::Small).render(),
            Button::new("button-default", "Default", palette).button_size(ButtonSize::Default).render(),
            Button::new("button-large", "Large", palette).button_size(ButtonSize::Large).render(),
            IconButton::new("button-large-icon", Icon::Add, palette)
                .button_size(ButtonSize::Large)
                .render(),
        ])]),
        section(cx, "Application sizing", vec![
            row(vec![
                Button::new("button-sizing", "Panel", palette)
                    .variant(ButtonVariant::Subtle)
                    .icon(Icon::PanelRightOpen)
                    .sizing(NAVIGATION)
                    .radius(RadiusRole::Card)
                    .render(),
                Button::new("button-sizing-disabled", "Panel", palette)
                    .variant(ButtonVariant::Subtle)
                    .icon(Icon::PanelRightOpen)
                    .sizing(NAVIGATION)
                    .radius(RadiusRole::Card)
                    .disabled()
                    .render(),
                IconButton::new("button-sizing-close", Icon::Close, palette)
                    .variant(ButtonVariant::Ghost)
                    .sizing(TAB_CLOSE)
                    .render(),
            ]),
            caption(cx, "Custom sizing instead of a named size: 32 px with 14 px text, and a 20 px close button."),
        ]),
        section(cx, "With icon", vec![row(vec![
            Button::new("button-icon", "New", palette).icon(Icon::Add).render(),
            Button::new("button-trailing", "Branch", palette)
                .variant(ButtonVariant::Outline)
                .icon(Icon::GitBranch)
                .trailing_icon(Icon::ChevronDown)
                .render(),
            IconButton::new("button-settings", Icon::Settings, palette).tooltip("Settings").render(),
            IconButton::new("button-trash", Icon::Trash, palette)
                .variant(ButtonVariant::Destructive)
                .tooltip("Delete")
                .render(),
        ])]),
        section(cx, "With picture", vec![
            caption(cx, "A picture in the icon's place, such as a custom emoji; it keeps its own colors when disabled."),
            row(vec![
                Button::new("button-picture", "Dusk", palette).image(super::media::picture(0)).render(),
                Button::new("button-picture-outline", "Forest", palette)
                    .variant(ButtonVariant::Outline)
                    .image(super::media::picture(1))
                    .render(),
                Button::new("button-picture-disabled", "Tide", palette)
                    .variant(ButtonVariant::Secondary)
                    .image(super::media::picture(2))
                    .disabled()
                    .render(),
                Button::new("button-picture-alone", "", palette)
                    .variant(ButtonVariant::Ghost)
                    .image(super::media::picture(0))
                    .render(),
            ]),
        ]),
        section(cx, "Clipboard", vec![
            caption(cx, "The icon becomes a checkmark while the host reports it (1.5 s with `ClipboardCopied`), and a toast announces it. Clicking here toggles it."),
            row(vec![
                Clipboard::new("button-clipboard-link", "Copy link", palette)
                    .copied(cx.state.flag("button-clipboard-link"))
                    .render(),
                Clipboard::new("button-clipboard-label", "Copy link", palette)
                    .label(if cx.state.flag("button-clipboard-label") { "Link copied" } else { "Copy link" })
                    .variant(ButtonVariant::Outline)
                    .copied(cx.state.flag("button-clipboard-label"))
                    .render(),
            ]),
        ]),
        section(cx, "Toolbar", vec![
            row(vec![
                IconButton::new("button-tool-dense", Icon::Add, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Dense)
                    .tooltip_with(
                        TooltipSpec::new("New")
                            .description("Creates a file in the open folder.")
                            .shortcut("Ctrl+N")
                            .placement(TooltipPlacement::Below),
                    )
                    .render(),
                IconButton::new("button-tool-minus", Icon::Minus, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .dispatch_as("button-tool-dense")
                    .tooltip_with(TooltipSpec::new("Remove").placement(TooltipPlacement::Below))
                    .render(),
                IconButton::new("button-tool-disabled", Icon::Trash, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .disabled()
                    .tooltip_with(
                        TooltipSpec::new("Nothing to delete").placement(TooltipPlacement::Below),
                    )
                    .render(),
                IconButton::new("button-tool-lift", Icon::PanelLeft.filled(), palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .active(true)
                    .tooltip("Active: Lift")
                    .render(),
                IconButton::new("button-tool-palette", Icon::PanelRight.filled(), palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Regular)
                    .active(true)
                    .active_hover(ActiveHover::Palette)
                    .tooltip("Active: Palette")
                    .render(),
            ]),
            caption(cx, "Dense, Compact and Regular are 24, 28 and 32 px squares. The disabled button keeps its tooltip."),
        ]),
        section(cx, "Ink, spacing and radius", vec![
            row(vec![
                Button::new("button-ink", "Discard", palette)
                    .variant(ButtonVariant::Ghost)
                    .ink(DANGER)
                    .render(),
                Button::new("button-ink-disabled", "Discard", palette)
                    .variant(ButtonVariant::Ghost)
                    .ink(DANGER)
                    .disabled()
                    .render(),
                Button::new("button-gap", "Connect", palette)
                    .variant(ButtonVariant::Outline)
                    .icon(Icon::Github)
                    .gap(12.0)
                    .render(),
                Button::new("button-radius", "Card", palette)
                    .variant(ButtonVariant::Outline)
                    .radius(RadiusRole::Card)
                    .render(),
                IconButton::new("button-radius-square", Icon::Add, palette)
                    .variant(ButtonVariant::Outline)
                    .radius(RadiusRole::Square)
                    .tooltip("Radius: Square")
                    .render(),
            ]),
            row(vec![
                IconButton::new("button-round-send", Icon::ArrowUp, palette)
                    .round()
                    .tooltip("round: send")
                    .render(),
                IconButton::new("button-round-stop", Icon::StopFill, palette)
                    .round()
                    .tooltip("round: stop")
                    .render(),
                Button::new("button-round-pill", "Pill", palette)
                    .variant(ButtonVariant::Secondary)
                    .round()
                    .render(),
            ]),
            caption(cx, "The application supplies the ink; when disabled, the button returns to its muted tone. round makes an icon button circular and a text button pill-shaped."),
        ]),
        section(cx, "Button group", vec![
            ButtonGroup::new(group("button-group-row", palette)).render(),
            ButtonGroup::new(group("button-group-end", palette))
                .align_end()
                .leading(
                    Button::new("button-group-help", "Help", palette)
                        .variant(ButtonVariant::Ghost)
                        .render(),
                )
                .render(),
            ButtonGroup::new(group("button-group-stacked", palette))
                .stacked(true)
                .reverse_when_stacked(true)
                .leading(
                    Button::new("button-group-stacked-help", "Help", palette)
                        .variant(ButtonVariant::Ghost)
                        .render(),
                )
                .render()
                .size(Some(220.0), None),
            caption(cx, "In a row, aligned at the end with a leading element, and stacked: the leading element goes above and the primary action comes first."),
            ButtonGroup::new(group("button-group-wide", palette)).sizing(WIDE_GROUP).render(),
            ButtonGroup::new(group("button-group-wide-stacked", palette))
                .sizing(WIDE_GROUP)
                .stacked(true)
                .render()
                .size(Some(220.0), None),
            caption(cx, "With custom sizing: 12 px between actions in a row and 10 in a column."),
        ]),
        section(cx, "Active", vec![
            row(vec![Button::new("button-toggle", "Toggle", palette)
                .variant(ButtonVariant::Ghost)
                .active(cx.state.flag("button-toggle"))
                .render()]),
            caption(cx, "The button keeps its fill while its state is active."),
        ]),
    ]
}
