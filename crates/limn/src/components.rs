//! Composable, controlled controls built from the same backend-neutral tree.
//! State such as the selected tab or whether a menu is open belongs to the
//! caller; stable action IDs let [`crate::UiTree`] retain focus and hover while
//! these controls are rebuilt on every frame.

use crate::{
    Color, Element, ElementKind, FontFamily, Icon, Point, PopoverPlacement, TooltipItem,
    RadiusRole, TooltipPlacement, TooltipSection, TooltipStyle, ICON_SIZE,
};
use crate::{DropdownMenu, DropdownMenuEntry, DropdownMenuItem, MenuMotion};

/// A component can render to an ordinary [`Element`] and be used anywhere in
/// the tree. App-specific views and generic controls follow the same contract.
pub trait Component {
    fn render(self) -> Element;
}

pub(crate) fn hover_ms() -> u16 {
    crate::tokens().motion.hover_ms
}
// Discrete corners for editor chrome. Circular controls (for example the
// switch thumb or a host's avatar) opt into their own semantic shape.
pub(crate) fn control_radius() -> f32 {
    crate::tokens().radius.control
}

pub(crate) fn surface_radius() -> f32 {
    crate::tokens().radius.surface
}

/// A small, viewport-aware hint anchored to a control or a passive element.
/// It appears after a short hover delay or immediately with keyboard focus,
/// without adding layout size or stealing keyboard input. Passive anchors
/// receive a hover region, without adding a click action or keyboard stop.
pub struct Tooltip {
    anchor: Element,
    anchor_id: String,
    title: String,
    shortcut: Option<String>,
    sections: Vec<TooltipSection>,
    max_width: f32,
    placement: TooltipPlacement,
    palette: ComponentPalette,
}

impl Tooltip {
    pub fn new(
        anchor: Element,
        anchor_id: impl Into<String>,
        title: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            anchor_id: anchor_id.into(),
            title: title.into(),
            shortcut: None,
            sections: Vec::new(),
            max_width: 360.0,
            placement: TooltipPlacement::Above,
            palette,
        }
    }

    /// The hint of something that stands for a file: its name as the title
    /// and the directory that holds it beneath, in a hint no wider than 320.
    /// A bare name has no second line, and a trailing separator is ignored.
    pub fn for_path(
        anchor: Element,
        anchor_id: impl Into<String>,
        path: &str,
        palette: ComponentPalette,
    ) -> Self {
        const MAX_WIDTH: f32 = 320.0;
        // A trailing separator belongs to the directory it closes; a path of
        // nothing else is the root, which names itself.
        let trimmed = path.trim_end_matches(['/', '\\']);
        let path = if trimmed.is_empty() { path } else { trimmed };
        let (parent, name) = match path.rfind(['/', '\\']) {
            Some(separator) if separator + 1 < path.len() => {
                (Some(&path[..separator.max(1)]), &path[separator + 1..])
            }
            _ => (None, path),
        };
        let tooltip = Self::new(anchor, anchor_id, name, palette).max_width(MAX_WIDTH);
        match parent {
            Some(parent) => tooltip.description(parent),
            None => tooltip,
        }
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Add a paragraph beneath the title, with measured word wrapping.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        if !text.trim().is_empty() {
            self.sections.push(TooltipSection::Description(text));
        }
        self
    }

    /// Add a compact row of labeled icons. Items wrap within the tooltip and
    /// remain informational, without click targets or nested tooltips.
    pub fn icon_row(
        mut self,
        label: impl Into<String>,
        items: impl IntoIterator<Item = TooltipItem>,
    ) -> Self {
        let items: Vec<_> = items
            .into_iter()
            .filter(|item| !item.label.trim().is_empty())
            .collect();
        if !items.is_empty() {
            self.sections.push(TooltipSection::IconRow {
                label: label.into(),
                items,
            });
        }
        self
    }

    /// Divide the preceding and following tooltip sections with a hairline.
    pub fn separator(mut self) -> Self {
        if !matches!(self.sections.last(), Some(TooltipSection::Separator)) {
            self.sections.push(TooltipSection::Separator);
        }
        self
    }

    /// Add secondary information such as authentication or a source note.
    pub fn footer(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        if !text.trim().is_empty() {
            self.sections.push(TooltipSection::Footer(text));
        }
        self
    }

    /// The viewport remains the upper bound even when a wider hint is requested.
    pub fn max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_width = width.max(32.0);
        }
        self
    }

    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }
}

impl Component for Tooltip {
    fn render(self) -> Element {
        let mut element = Element::tooltip(
            self.anchor,
            self.anchor_id,
            self.title,
            self.shortcut,
            self.placement,
            self.palette.tooltip_style(),
        );
        if let ElementKind::Tooltip {
            sections,
            max_width,
            ..
        } = &mut element.kind
        {
            *sections = self.sections;
            *max_width = self.max_width;
        }
        element
    }
}

/// What the hint of a [`Button`] or an [`IconButton`] says and where it
/// opens. The button supplies the anchor and its ID.
#[derive(Clone, Debug)]
pub struct TooltipSpec {
    title: Option<String>,
    description: Option<String>,
    shortcut: Option<String>,
    placement: TooltipPlacement,
    palette: Option<ComponentPalette>,
    max_width: Option<f32>,
    suppressed: bool,
}

impl TooltipSpec {
    /// A hint with this title, opening above its button.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            ..Self::none()
        }
    }

    fn none() -> Self {
        Self {
            title: None,
            description: None,
            shortcut: None,
            placement: TooltipPlacement::Above,
            palette: None,
            max_width: None,
            suppressed: false,
        }
    }

    /// Add a paragraph beneath the title.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Show the keys that trigger the action beside the title. An empty
    /// shortcut, such as an unbound action, is ignored.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        let shortcut = shortcut.into();
        if !shortcut.trim().is_empty() {
            self.shortcut = Some(shortcut);
        }
        self
    }

    /// The side of the button the hint prefers.
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Color the hint from this palette instead of the button's own, for a
    /// button whose action palette differs from the surrounding theme.
    pub fn palette(mut self, palette: ComponentPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Limit the hint's width. A value that is not finite is ignored.
    pub fn max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_width = Some(width);
        }
        self
    }

    /// Render the button without its hint, for example while the menu it
    /// opens is showing.
    pub fn suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    // Wrap a rendered button, enabled or passive, in the hint.
    pub(crate) fn wrap(self, anchor: Element, id: String, button_palette: ComponentPalette) -> Element {
        let Some(title) = self.title.filter(|_| !self.suppressed) else {
            return anchor;
        };
        let mut tooltip = Tooltip::new(anchor, id, title, self.palette.unwrap_or(button_palette))
            .placement(self.placement);
        if let Some(shortcut) = self.shortcut {
            tooltip = tooltip.shortcut(shortcut);
        }
        if let Some(text) = self.description {
            tooltip = tooltip.description(text);
        }
        if let Some(width) = self.max_width {
            tooltip = tooltip.max_width(width);
        }
        tooltip.render()
    }
}

/// How an active [`Button`] without a persistent fill answers the pointer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActiveHover {
    /// Hover and press lighten the selected fill.
    #[default]
    Lift,
    /// Hover and press use the palette's own hover and selected colors, as
    /// the inactive button does.
    Palette,
}

/// Shared control heights in logical pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlSize {
    Dense,
    Compact,
    Regular,
    Spacious,
}

impl ControlSize {
    pub const fn height(self) -> f32 {
        match self {
            Self::Dense => 24.0,
            Self::Compact => 28.0,
            Self::Regular => 32.0,
            Self::Spacious => 36.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ControlState {
    #[default]
    Enabled,
    Disabled,
}

/// Semantic colors shared by tabs, dropdowns, and floating surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentPalette {
    pub surface: Color,
    pub raised: Color,
    pub hover: Color,
    pub selected: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub shadow: Color,
}

impl ComponentPalette {
    /// Apply the common action surface without changing its layout or focus
    /// target. Tooltips and popovers forward this to their trigger; disabled
    /// panels remain passive. Callers retain their contextual palette and can
    /// override the selected treatment after applying this style.
    pub fn interactive_surface(self, mut element: Element) -> Element {
        self.style_interactive_surface(&mut element, control_radius());
        element
    }

    /// A contiguous cell in a breadcrumb or status bar. The host supplies the
    /// full bar height; hover, press and keyboard focus keep square corners.
    /// Floating menus and tooltips retain their own surface treatment.
    pub fn bar_item_surface(self, mut element: Element) -> Element {
        self.style_interactive_surface(&mut element, 0.0);
        element
    }

    fn style_interactive_surface(self, element: &mut Element, radius: f32) {
        match &mut element.kind {
            ElementKind::Tooltip { anchor, .. } | ElementKind::Popover { anchor, .. } => {
                self.style_interactive_surface(anchor, radius);
            }
            ElementKind::Button { .. }
            | ElementKind::IconButton { .. }
            | ElementKind::Clickable { .. } => {
                element.style.hover_background = Some(self.hover);
                element.style.pressed_background = Some(self.selected);
                element.style.radius = radius;
                element.style.transition_ms = hover_ms();
            }
            _ => {}
        }
    }

    pub const fn neutral_dark() -> Self {
        Self {
            surface: Color::rgb(24, 24, 24),
            raised: Color::rgb(38, 38, 38),
            hover: Color::rgb(48, 48, 48),
            selected: Color::rgb(58, 58, 58),
            border: Color::rgb(63, 63, 63),
            text: Color::rgb(225, 225, 225),
            muted: Color::rgb(153, 153, 153),
            shadow: Color::rgba(0, 0, 0, 100),
        }
    }

    fn is_dark(self) -> bool {
        u16::from(self.border.r) + u16::from(self.border.g) + u16::from(self.border.b) < 3 * 128
    }

    /// Ink for links: blue, in the shade that reads on this palette.
    pub fn link(self) -> Color {
        if self.is_dark() {
            Color::rgb(68, 147, 248)
        } else {
            Color::rgb(9, 105, 218)
        }
    }

    pub(crate) fn tooltip_style(self) -> TooltipStyle {
        // Buttons may use an action palette with a light fill or muted ink.
        // The shared tooltip still belongs to the surrounding theme: its
        // border and muted text are stable semantic tokens across actions.
        // Dark surfaces need a clearer edge and a darker interior than a
        // regular raised control. Light themes retain their own colors.
        if self.is_dark() {
            TooltipStyle {
                background: darker(self.border, 19),
                border: lighter(self.border, 18),
                keycap: crate::shortcut::cap_fill(lighter(self.muted, 76)),
                text: lighter(self.muted, 76),
                muted: self.muted,
                shadow: self.shadow,
            }
        } else {
            TooltipStyle {
                background: self.raised,
                border: self.border,
                keycap: crate::shortcut::cap_fill(self.text),
                text: self.text,
                muted: self.muted,
                shadow: self.shadow,
            }
        }
    }
}

/// Paint treatment for an action. All variants use the same geometry for a
/// given [`ControlSize`]; the visual state only changes color and border.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonVariant {
    /// Light primary action with dark text.
    #[default]
    Default,
    /// Quiet filled action on a raised surface.
    Secondary,
    /// Filled action for irreversible operations.
    Destructive,
    /// Filled action for cautionary operations.
    Warning,
    /// Filled action for a successful or affirmative operation.
    Success,
    /// A toolbar action without a persistent fill.
    Ghost,
    /// A text action with an underline on hover and keyboard focus.
    Link,
    /// A navigation row on its parent surface.
    Subtle,
    /// A control raised above its parent surface.
    Raised,
    /// An outlined control on its parent surface.
    Outline,
    /// A prominent, neutral action.
    Solid,
}

/// Sizes matching the reference button family. Existing `ControlSize` values
/// remain available for compact chrome and navigation controls.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonSize {
    #[default]
    Default,
    Small,
    Large,
    Icon,
    IconSmall,
    IconExtraSmall,
    /// A 22 px action inside a dense row, such as "more" in a file tree.
    Mini,
}

impl ButtonSize {
    pub fn sizing(self) -> ButtonSizing {
        let (height, font_size, horizontal_padding) = match self {
            Self::Default => (36.0, 14.0, 16.0),
            Self::Small => (32.0, 13.0, 12.0),
            Self::Large => (40.0, 14.0, 24.0),
            Self::Icon => (36.0, 14.0, 10.0),
            Self::IconSmall => (32.0, 13.0, 8.0),
            Self::IconExtraSmall => (24.0, 12.0, 4.0),
            Self::Mini => (22.0, 11.0, 3.0),
        };
        ButtonSizing {
            height,
            font_size,
            icon_size: ICON_SIZE,
            horizontal_padding,
            inner_spacing: 8.0,
        }
    }
}

/// The visible content of one action. This keeps icon-only, text-only, and
/// combined buttons on the same rendering and interaction path.
#[derive(Clone, Debug)]
pub enum ButtonContent {
    Label(String),
    Icon(Icon),
    IconAndLabel(Icon, String),
}

impl ButtonContent {
    fn parts(self) -> (Option<Icon>, String) {
        match self {
            Self::Label(label) => (None, label),
            Self::Icon(icon) => (Some(icon), String::new()),
            Self::IconAndLabel(icon, label) => (Some(icon), label),
        }
    }
}

/// Shared button geometry in logical pixels. The current renderer uses one
/// canonical 16 px icon size, while density controls the hit box and type.
/// The radius is not part of it: it is a token, chosen with
/// [`Button::radius`]. A host keeps its own densities as consts of this type
/// and passes one with [`Button::sizing`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonSizing {
    /// Height of the button, and the width of an icon-only one.
    pub height: f32,
    /// Size of the label.
    pub font_size: f32,
    /// Room kept for a glyph; the glyph itself is drawn at 16 px.
    pub icon_size: f32,
    /// Space between the button's edge and its content.
    pub horizontal_padding: f32,
    /// Space between the icons and the label.
    pub inner_spacing: f32,
}

impl ButtonSizing {
    // A profile is plain data from the host; a bad number stops here.
    fn sanitized(self) -> Self {
        let px = crate::page::px;
        Self {
            height: px(self.height),
            font_size: px(self.font_size),
            icon_size: px(self.icon_size),
            horizontal_padding: px(self.horizontal_padding),
            inner_spacing: px(self.inner_spacing),
        }
    }
}

impl ControlSize {
    pub fn button_sizing(self) -> ButtonSizing {
        let height = self.height();
        ButtonSizing {
            height,
            font_size: match self {
                Self::Dense | Self::Compact => 12.0,
                Self::Regular => 13.0,
                Self::Spacious => 14.0,
            },
            icon_size: ICON_SIZE,
            horizontal_padding: (height - ICON_SIZE) / 2.0,
            inner_spacing: 6.0,
        }
    }
}

/// A semantic action with one stable ID. Selection and availability belong to
/// the caller, while hover/press/focus are retained by [`crate::UiTree`].
pub struct Button {
    action_id: String,
    content: ButtonContent,
    trailing_icon: Option<Icon>,
    variant: ButtonVariant,
    size: ControlSize,
    button_size: Option<ButtonSize>,
    sizing: Option<ButtonSizing>,
    width: Option<f32>,
    active: bool,
    active_hover: ActiveHover,
    state: ControlState,
    loading: bool,
    ink: Option<Color>,
    hover_ink: Option<Color>,
    pressed_ink: Option<Color>,
    gap: Option<f32>,
    radius: RadiusRole,
    round: bool,
    palette: ComponentPalette,
    tooltip: TooltipSpec,
    dispatch_as: Option<String>,
}

impl Button {
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self::with_content(action_id, ButtonContent::Label(label.into()), palette)
    }

    pub fn with_content(
        action_id: impl Into<String>,
        content: ButtonContent,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            content,
            trailing_icon: None,
            variant: ButtonVariant::default(),
            size: ControlSize::Regular,
            button_size: None,
            sizing: None,
            width: None,
            active: false,
            active_hover: ActiveHover::default(),
            state: ControlState::Enabled,
            loading: false,
            ink: None,
            hover_ink: None,
            pressed_ink: None,
            gap: None,
            radius: RadiusRole::Control,
            round: false,
            palette,
            tooltip: TooltipSpec::none(),
            dispatch_as: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        let (_, label) = self.content.parts();
        self.content = if label.is_empty() {
            ButtonContent::Icon(icon)
        } else {
            ButtonContent::IconAndLabel(icon, label)
        };
        self
    }

    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.tooltip.title = Some(title.into());
        self
    }

    pub fn tooltip_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.tooltip.shortcut = Some(shortcut.into());
        self
    }

    pub fn tooltip_placement(mut self, placement: TooltipPlacement) -> Self {
        self.tooltip.placement = placement;
        self
    }

    /// Set the whole hint at once. A disabled button keeps its hint.
    pub fn tooltip_with(mut self, spec: TooltipSpec) -> Self {
        self.tooltip = spec;
        self
    }

    /// Report this action when activated, while the button keeps its own ID
    /// for focus, hover and its hint.
    pub fn dispatch_as(mut self, action_id: impl Into<String>) -> Self {
        self.dispatch_as = Some(action_id.into());
        self
    }

    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self.button_size = None;
        self.sizing = None;
        self
    }

    pub fn button_size(mut self, size: ButtonSize) -> Self {
        self.button_size = Some(size);
        self.sizing = None;
        self
    }

    /// Take the density from a profile of the host's, in place of a named
    /// size. A number that is negative or not finite is read as zero.
    pub fn sizing(mut self, sizing: ButtonSizing) -> Self {
        self.sizing = Some(sizing.sanitized());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Choose how an active Ghost, Subtle, Raised or Outline button hovers.
    pub fn active_hover(mut self, active_hover: ActiveHover) -> Self {
        self.active_hover = active_hover;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }

    /// Replace the leading icon with the standard animated spinner. A busy
    /// action keeps its label and height, but cannot be focused or activated.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Draw the label and icons in a semantic color, such as the host's
    /// destructive ink. Hover and press keep it unless `hover_ink` or
    /// `pressed_ink` say otherwise; a disabled button stays muted.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    pub fn hover_ink(mut self, color: Color) -> Self {
        self.hover_ink = Some(color);
        self
    }

    pub fn pressed_ink(mut self, color: Color) -> Self {
        self.pressed_ink = Some(color);
        self
    }

    /// Set the space between the icons and the label, in place of the
    /// size's own. A value that is not finite is ignored.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = Some(gap.max(0.0));
        }
        self
    }

    /// Round the button with another radius token. The default is `Control`.
    pub fn radius(mut self, radius: RadiusRole) -> Self {
        self.radius = radius;
        self
    }

    /// Round the ends fully: an icon button becomes a circle and a labelled
    /// one a pill, at any size. It takes precedence over `radius`.
    pub fn round(mut self) -> Self {
        self.round = true;
        self
    }

    fn colors(&self) -> (Color, Color, Color, Color, bool) {
        let palette = self.palette;
        let (base, hover, pressed, ink) = match self.variant {
            ButtonVariant::Default => (
                palette.text,
                darker(palette.text, 16),
                darker(palette.text, 30),
                palette.surface,
            ),
            ButtonVariant::Secondary => (
                palette.raised,
                lighter(palette.raised, 10),
                lighter(palette.raised, 18),
                palette.text,
            ),
            ButtonVariant::Destructive => (
                Color::rgb(161, 67, 71),
                Color::rgb(178, 77, 81),
                Color::rgb(139, 54, 58),
                Color::WHITE,
            ),
            ButtonVariant::Warning => (
                Color::rgb(209, 154, 55),
                Color::rgb(225, 169, 65),
                Color::rgb(183, 132, 42),
                Color::rgb(32, 27, 19),
            ),
            ButtonVariant::Success => (
                Color::rgb(47, 140, 94),
                Color::rgb(57, 157, 105),
                Color::rgb(36, 120, 78),
                Color::WHITE,
            ),
            ButtonVariant::Ghost => (
                Color::TRANSPARENT,
                palette.hover,
                palette.selected,
                palette.text,
            ),
            ButtonVariant::Link => (
                Color::TRANSPARENT,
                Color::TRANSPARENT,
                Color::TRANSPARENT,
                palette.link(),
            ),
            ButtonVariant::Subtle | ButtonVariant::Outline => (
                palette.surface,
                palette.hover,
                palette.selected,
                palette.text,
            ),
            ButtonVariant::Raised => (
                palette.raised,
                if palette.hover == palette.raised {
                    lighter(palette.raised, 8)
                } else {
                    palette.hover
                },
                palette.selected,
                palette.text,
            ),
            ButtonVariant::Solid => (
                palette.selected,
                lighter(palette.selected, 10),
                lighter(palette.selected, 18),
                palette.text,
            ),
        };
        let (base, hover, pressed) = if self.active
            && matches!(
                self.variant,
                ButtonVariant::Subtle
                    | ButtonVariant::Raised
                    | ButtonVariant::Outline
                    | ButtonVariant::Ghost
            ) {
            match self.active_hover {
                ActiveHover::Lift => (
                    palette.selected,
                    lighter(palette.selected, 10),
                    lighter(palette.selected, 18),
                ),
                ActiveHover::Palette => (palette.selected, palette.hover, palette.selected),
            }
        } else if self.active {
            (hover, pressed, pressed)
        } else {
            (base, hover, pressed)
        };
        // A muted label on a light or saturated fill loses contrast. Disabled
        // controls use the quiet raised surface while retaining their size.
        let base = if self.state == ControlState::Disabled
            && matches!(
                self.variant,
                ButtonVariant::Default
                    | ButtonVariant::Destructive
                    | ButtonVariant::Warning
                    | ButtonVariant::Success
            ) {
            palette.raised
        } else {
            base
        };
        let ink = if self.state == ControlState::Disabled {
            palette.muted
        } else {
            self.ink.unwrap_or(ink)
        };
        (
            base,
            hover,
            pressed,
            ink,
            self.variant == ButtonVariant::Outline,
        )
    }
}

impl Component for Button {
    fn render(self) -> Element {
        let (base, hover, pressed, ink, outline) = self.colors();
        let sizing = self
            .sizing
            .or(self.button_size.map(ButtonSize::sizing))
            .unwrap_or_else(|| self.size.button_sizing());
        let padding = sizing.horizontal_padding;
        let spacing = self.gap.unwrap_or(sizing.inner_spacing);
        let (icon, label) = self.content.parts();
        let icon_only = (icon.is_some() || self.loading) && label.is_empty();
        let width = self.width.or(icon_only.then_some(sizing.height));
        // A square icon button keeps room for its glyph at every size.
        let padding = if icon_only {
            padding.min(((width.unwrap_or(sizing.height) - sizing.icon_size) / 2.0).max(0.0))
        } else {
            padding
        };
        let height = sizing.height;
        // Half the height is a shape, not a token.
        let radius = if self.round { height / 2.0 } else { self.radius.px() };

        if self.state == ControlState::Disabled || self.loading {
            // The engine's Button element always registers a hit target. A
            // disabled action is instead ordinary painted content: it remains
            // visible, but cannot be focused or activated by any input path.
            let has_label = !label.is_empty();
            let icon_space = if icon.is_some() || self.loading {
                sizing.icon_size + spacing
            } else {
                0.0
            };
            let trailing_space = if self.trailing_icon.is_some() {
                sizing.icon_size + spacing
            } else {
                0.0
            };
            let label_width =
                width.map(|value| (value - padding * 2.0 - icon_space - trailing_space).max(0.0));
            let content_height = (sizing.font_size * 18.0 / 14.0).max(sizing.icon_size);
            let center_icon = |child: Element| {
                Element::column(vec![
                    Element::spacer().flex(1.0),
                    child,
                    Element::spacer().flex(1.0),
                ])
                .size(Some(sizing.icon_size), Some(content_height))
            };
            let leading = if self.loading {
                Some(center_icon(
                    crate::Spinner::new(format!("{}:loading", self.action_id), ink)
                        .size(sizing.icon_size)
                        .render(),
                ))
            } else {
                icon.map(|icon| {
                    center_icon(
                        Element::icon(icon)
                            .foreground(ink)
                            .size(Some(sizing.icon_size), Some(sizing.icon_size)),
                    )
                })
            };
            let label = Element::truncated_text(label)
                .foreground(ink)
                .font_family(FontFamily::Sans)
                .font_size(sizing.font_size)
                .size(label_width, Some(content_height));
            let visual = if icon_only && self.trailing_icon.is_none() {
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    leading.expect("icon or spinner"),
                    Element::spacer().flex(1.0),
                ])
            } else if let Some(leading) = leading {
                let mut children = vec![leading];
                if has_label {
                    children.push(label);
                }
                if let Some(trailing) = self.trailing_icon {
                    children.push(center_icon(Element::icon(trailing).foreground(ink)));
                }
                Element::row(children).gap(spacing)
            } else if let Some(trailing) = self.trailing_icon {
                Element::row(vec![
                    label,
                    center_icon(Element::icon(trailing).foreground(ink)),
                ])
                .gap(spacing)
            } else {
                label
            };
            // Button padding is horizontal. Center the content vertically
            // without clipping large button sizes or the spinner's slot.
            let vertical_padding = ((height - content_height) / 2.0).max(0.0);
            let extra_horizontal = (padding - vertical_padding).max(0.0);
            let mut element = Element::panel(Element::row(vec![
                Element::spacer().size(Some(extra_horizontal), None),
                visual.size(None, Some(content_height)).flex(1.0),
                Element::spacer().size(Some(extra_horizontal), None),
            ]))
            .padding(vertical_padding)
            .background(base)
            .radius(radius)
            .size(width, Some(height));
            if outline {
                element = element.border(self.palette.border, 1.0);
            }
            return self.tooltip.wrap(element, self.action_id, self.palette);
        }

        let mut element = if let Some(icon) = icon {
            Element::icon_button(self.action_id.clone(), icon, label)
        } else {
            Element::button(self.action_id.clone(), label)
        };
        if let Some(trailing) = self.trailing_icon {
            element = element.trailing_icon(trailing);
        }
        element = element
            .background(base)
            .hover_background(hover)
            .pressed_background(pressed)
            .hover_foreground(self.hover_ink.unwrap_or(ink))
            .pressed_foreground(self.pressed_ink.or(self.hover_ink).unwrap_or(ink))
            .transition_ms(hover_ms())
            .foreground(ink)
            .font_family(FontFamily::Sans)
            .font_size(sizing.font_size)
            .padding(padding)
            .gap(spacing)
            .radius(radius)
            .size(width, Some(height));
        if matches!(
            self.variant,
            ButtonVariant::Default
                | ButtonVariant::Secondary
                | ButtonVariant::Destructive
                | ButtonVariant::Warning
                | ButtonVariant::Success
                | ButtonVariant::Ghost
                | ButtonVariant::Outline
                | ButtonVariant::Link
        ) {
            element = element.center_content();
        }
        if self.variant == ButtonVariant::Link {
            element = element.underline_on_hover();
        }
        if outline {
            element = element.border(self.palette.border, 1.0);
        }
        if let Some(action) = self.dispatch_as {
            element = element.dispatch_as(action);
        }
        self.tooltip.wrap(element, self.action_id, self.palette)
    }
}

/// A text action that hugs its label: link ink, underlined under the pointer
/// and keyboard focus, with no hover fill. Emits `UiAction::Clicked`.
pub struct Link {
    action_id: String,
    label: String,
    color: Color,
    font_size: f32,
}

impl Link {
    /// A text action that reports `Clicked` with this ID.
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            color: palette.link(),
            font_size: 13.0,
        }
    }

    /// Draw the link in this color, in place of the palette's link ink.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the size of the text. A size that is not finite or not above
    /// zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size;
        }
        self
    }
}

impl Component for Link {
    fn render(self) -> Element {
        Element::button(self.action_id, self.label)
            .foreground(self.color)
            .font_family(FontFamily::Sans)
            .font_size(self.font_size)
            .padding(0.0)
            .underline_on_hover()
    }
}

/// A button whose leading icon is mandatory. Without a label it uses a square
/// hit target, while the glyph remains 16 logical pixels at every density.
pub struct IconButton(Button);

impl IconButton {
    pub fn new(action_id: impl Into<String>, icon: Icon, palette: ComponentPalette) -> Self {
        Self(Button::with_content(
            action_id,
            ButtonContent::Icon(icon),
            palette,
        ))
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        let (icon, _) = self.0.content.parts();
        self.0.content = ButtonContent::IconAndLabel(icon.expect("IconButton icon"), label.into());
        self
    }

    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.0 = self.0.tooltip(title);
        self
    }

    pub fn tooltip_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.0 = self.0.tooltip_shortcut(shortcut);
        self
    }

    pub fn tooltip_placement(mut self, placement: TooltipPlacement) -> Self {
        self.0 = self.0.tooltip_placement(placement);
        self
    }

    /// Set the whole hint at once. A disabled button keeps its hint.
    pub fn tooltip_with(mut self, spec: TooltipSpec) -> Self {
        self.0 = self.0.tooltip_with(spec);
        self
    }

    /// Report this action when activated, while the button keeps its own ID.
    pub fn dispatch_as(mut self, action_id: impl Into<String>) -> Self {
        self.0 = self.0.dispatch_as(action_id);
        self
    }

    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.0.trailing_icon = Some(icon);
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.0.variant = variant;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.0 = self.0.size(size);
        self
    }

    pub fn button_size(mut self, size: ButtonSize) -> Self {
        self.0 = self.0.button_size(size);
        self
    }

    /// Take the density from a profile of the host's, as [`Button::sizing`].
    pub fn sizing(mut self, sizing: ButtonSizing) -> Self {
        self.0 = self.0.sizing(sizing);
        self
    }

    /// Round the button with another radius token, as [`Button::radius`].
    pub fn radius(mut self, radius: RadiusRole) -> Self {
        self.0 = self.0.radius(radius);
        self
    }

    /// A circle at any size, as [`Button::round`].
    pub fn round(mut self) -> Self {
        self.0 = self.0.round();
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.0 = self.0.width(width);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.0.active = active;
        self
    }

    /// Choose how an active button without a persistent fill hovers.
    pub fn active_hover(mut self, active_hover: ActiveHover) -> Self {
        self.0.active_hover = active_hover;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.0.state = ControlState::Disabled;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0 = self.0.enabled(enabled);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.0 = self.0.loading(loading);
        self
    }

    /// Draw the icon and label in this color at rest; see [`Button::ink`].
    pub fn ink(mut self, color: Color) -> Self {
        self.0.ink = Some(color);
        self
    }

    pub fn hover_ink(mut self, color: Color) -> Self {
        self.0.hover_ink = Some(color);
        self
    }

    pub fn pressed_ink(mut self, color: Color) -> Self {
        self.0.pressed_ink = Some(color);
        self
    }
}

impl Component for IconButton {
    fn render(self) -> Element {
        self.0.render()
    }
}

/// Passive VS Code-style keycaps sharing the painter used by search and tooltips.
pub struct Kbd {
    keys: String,
    palette: ComponentPalette,
    max_width: Option<f32>,
    menu: bool,
    alternatives: Vec<String>,
    caption: Option<String>,
}

impl Kbd {
    pub fn new(keys: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            keys: keys.into(),
            palette,
            max_width: None,
            menu: false,
            alternatives: Vec::new(),
            caption: None,
        }
    }

    /// Another key that does the same, shown after a slash: Enter / Tab.
    pub fn alternative(mut self, keys: impl Into<String>) -> Self {
        self.alternatives.push(keys.into());
        self
    }

    /// Say what the keys do, in a small muted caption after them: the hint
    /// in the footer of a palette or a suggestion list.
    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

    /// Render the same combination as a plain, right-aligned action-menu
    /// accelerator. Instructional hints keep the default outlined keycaps.
    pub fn menu(mut self) -> Self {
        self.menu = true;
        self
    }

    /// Omit the complete hint, including alternatives and caption, if its
    /// measured width cannot fit this budget.
    /// A truncated combination would imply a different keyboard shortcut.
    pub fn max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_width = Some(width.max(0.0));
        }
        self
    }
}

impl Component for Kbd {
    fn render(mut self) -> Element {
        if self.caption.is_some() || !self.alternatives.is_empty() {
            let palette = self.palette;
            let small = crate::TextRole::Small.size();
            let caption = self.caption.take();
            let mut parts = Vec::new();
            for keys in std::iter::once(std::mem::take(&mut self.keys))
                .chain(std::mem::take(&mut self.alternatives))
            {
                if !parts.is_empty() {
                    parts.push(Element::text("/").font_size(small).foreground(palette.muted));
                }
                parts.push(
                    Kbd { keys, menu: self.menu, ..Kbd::new("", palette) }.render(),
                );
            }
            if let Some(caption) = caption {
                parts.push(
                    Element::text(caption)
                        .font_family(FontFamily::Sans)
                        .font_size(small)
                        .foreground(palette.muted),
                );
            }
            // One pixel of air above and below the keycaps.
            return Element::shortcut_hint(Element::row(parts).gap(4.0), self.max_width)
                .size(None, Some(crate::shortcut::HEIGHT + 2.0));
        }
        if crate::shortcut::label(&self.keys).is_empty() {
            return Element::spacer().size(Some(0.0), Some(0.0));
        }
        let element = if self.menu {
            Element::menu_shortcut(self.keys, self.max_width)
                .foreground(self.palette.muted)
                .font_size(crate::shortcut::MENU_FONT_SIZE)
        } else {
            Element::keyboard_shortcut(
                self.keys,
                crate::shortcut::cap_fill(self.palette.text),
                self.max_width,
            )
            .foreground(self.palette.text)
            .font_size(crate::shortcut::FONT_SIZE)
        };
        element
            .font_family(FontFamily::Sans)
            .size(None, Some(crate::shortcut::HEIGHT))
    }
}

/// A single-line input with the same density, focus treatment and palette as
/// the shared search field. The host owns its value through `InputChanged`.
pub struct TextField {
    id: String,
    value: String,
    placeholder: String,
    persistent_placeholder: bool,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
    height: Option<f32>,
    state: ControlState,
    password: bool,
    monospace: bool,
}

impl TextField {
    /// A field with its current value and the placeholder shown while it is
    /// empty. The host stores what `InputChanged` reports under this ID.
    pub fn new(
        id: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            persistent_placeholder: false,
            palette,
            size: ControlSize::Regular,
            width: None,
            height: None,
            state: ControlState::Enabled,
            password: false,
            monospace: false,
        }
    }

    /// Take the height and text size of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Set the value in the monospaced family, for a path, an identifier or
    /// a number.
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    /// Mask what is typed when `hidden`, as a password field does.
    pub fn password(mut self, hidden: bool) -> Self {
        self.password = hidden;
        self
    }

    /// Keep the placeholder visible while this field is focused and empty.
    pub fn persistent_placeholder(mut self, visible: bool) -> Self {
        self.persistent_placeholder = visible;
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// A height of the host's own, for a form whose inputs are taller than
    /// a [`ControlSize`]. The field is never shorter than its control size;
    /// a value that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height);
        }
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for TextField {
    fn render(self) -> Element {
        let least = self.size.height().max(28.0);
        let height = self.height.map_or(least, |height| height.max(least));
        let family = if self.monospace {
            FontFamily::Monospace
        } else {
            FontFamily::Sans
        };
        if self.state == ControlState::Disabled {
            let text = if self.password && !self.value.is_empty() {
                "•".repeat(self.value.chars().count())
            } else if self.value.is_empty() {
                self.placeholder
            } else {
                self.value
            };
            return Element::panel(
                Element::truncated_text(text)
                    .foreground(self.palette.muted)
                    .font_family(family)
                    .font_size(13.0),
            )
            .padding(8.0)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(input_radius(self.size))
            .size(self.width, Some(height));
        }
        input_surface(
            Element::text_input(self.id, self.value, self.placeholder)
                .password(self.password)
                .persistent_placeholder(self.persistent_placeholder),
            self.palette,
            self.size,
            self.width,
        )
        .size(self.width, Some(height))
        .font_family(family)
    }
}

/// A multi-line editor using the same palette and focus treatment as
/// [`TextField`]. The host owns its text through `InputChanged`.
pub struct TextArea {
    id: String,
    value: String,
    placeholder: String,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
    radius: RadiusRole,
    state: ControlState,
    preserve_line_endings: bool,
}

impl TextArea {
    /// A multi-line field with its current value and the placeholder shown
    /// while it is empty. The host stores what `InputChanged` reports under this ID.
    pub fn new(
        id: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            palette,
            width: None,
            height: 96.0,
            radius: RadiusRole::Control,
            state: ControlState::Enabled,
            preserve_line_endings: false,
        }
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Set the height, in logical pixels, never below 64. A height that is not
    /// finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(64.0);
        }
        self
    }

    /// Round the box with another radius token. The default is `Control`.
    pub fn radius(mut self, radius: RadiusRole) -> Self {
        self.radius = radius;
        self
    }

    /// Opt in to retaining CRLF/CR bytes through editing and clipboard events.
    pub fn preserve_line_endings(mut self, preserve: bool) -> Self {
        self.preserve_line_endings = preserve;
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for TextArea {
    fn render(self) -> Element {
        if self.state == ControlState::Disabled {
            let text = if self.value.is_empty() {
                self.placeholder
            } else {
                self.value
            };
            return Element::panel(
                Element::text(text)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0),
            )
            .padding(8.0)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(self.radius.px())
            .size(self.width, Some(self.height));
        }
        Element::text_area(self.id, self.value, self.placeholder)
            .text_area_preserve_line_endings(self.preserve_line_endings)
            .background(self.palette.raised)
            .hover_background(lighter(self.palette.raised, 5))
            .border(self.palette.border, 1.0)
            .focus_border(lighter(self.palette.border, 70), 1.5)
            .foreground(self.palette.text)
            .font_family(FontFamily::Sans)
            .font_size(13.0)
            .padding(8.0)
            .radius(self.radius.px())
            .min_size(80.0, self.height)
            .size(self.width, Some(self.height))
    }
}

fn input_radius(_size: ControlSize) -> f32 {
    control_radius()
}

fn input_surface(
    input: Element,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
) -> Element {
    let height = size.height().max(28.0);
    input
        .background(palette.raised)
        .hover_background(lighter(palette.raised, 5))
        .border(palette.border, 1.0)
        .focus_border(lighter(palette.border, 70), 1.5)
        .foreground(palette.text)
        .font_family(FontFamily::Sans)
        .font_size(13.0)
        .padding(8.0)
        .radius(input_radius(size))
        .min_size(80.0, height)
        .size(width, Some(height))
}

/// An editable search control. Its magnifier, text, caret, selection and
/// focus border all live in the same hit box, so pointer and keyboard focus
/// cannot disagree about the field's bounds. The host owns the query and
/// handles `UiAction::InputChanged` for `id`.
pub struct SearchField {
    id: String,
    value: String,
    placeholder: String,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
    state: ControlState,
    shortcut: Option<String>,
}

/// The application's shared search input; the existing `SearchField` name
/// remains available to current callers.
pub type Search = SearchField;

impl SearchField {
    /// A search field with its current query and the placeholder shown while
    /// it is empty. The host stores what `InputChanged` reports under this ID.
    pub fn new(
        id: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            palette,
            size: ControlSize::Spacious,
            width: None,
            state: ControlState::Enabled,
            shortcut: None,
        }
    }

    /// Take the height and text size of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Show the keys of a shortcut in the field, as a hint. The field does
    /// not bind them: the host does.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for SearchField {
    fn render(self) -> Element {
        let height = self.size.height().max(28.0);
        let radius = input_radius(self.size);
        let surface = self.palette.raised;
        if self.state == ControlState::Disabled {
            let (value, ink) = if self.value.is_empty() {
                (self.placeholder, self.palette.muted)
            } else {
                (self.value, self.palette.muted)
            };
            let mut children = vec![
                Element::icon(Icon::Search)
                    .foreground(self.palette.muted)
                    .size(Some(ICON_SIZE), Some(height)),
                Element::truncated_text(value)
                    .foreground(ink)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0)
                    .size(None, Some(height))
                    .flex(1.0),
            ];
            if let Some(shortcut) = self.shortcut {
                let mut hint = Kbd::new(shortcut, self.palette);
                if let Some(width) = self.width {
                    hint = hint.max_width((width - 16.0 - ICON_SIZE - 16.0 - 24.0).max(0.0));
                }
                let hint = hint.render().size(None, Some(height));
                children.push(if self.width.is_some() {
                    hint
                } else {
                    hint.flex(1.0)
                });
            }
            return Element::panel(Element::row(vec![
                Element::spacer().size(Some(8.0), None),
                Element::row(children).gap(8.0).flex(1.0),
                Element::spacer().size(Some(8.0), None),
            ]))
            .background(surface)
            .border(self.palette.border, 1.0)
            .radius(radius)
            .size(self.width, Some(height));
        }
        let mut input = input_surface(
            Element::text_input(self.id, self.value, self.placeholder)
                .leading_icon(Icon::Search)
                .leading_icon_tint(self.palette.muted),
            self.palette,
            self.size,
            self.width,
        );
        if let Some(shortcut) = self.shortcut {
            input = input
                .input_shortcut(shortcut)
                .input_shortcut_fill(crate::shortcut::cap_fill(self.palette.text));
        }
        input
    }
}

/// A caller-controlled switch with a single focusable target. The thumb is
/// painted inside its track and moves only when the caller passes a new
/// `checked` value after receiving `UiAction::Clicked`.
pub struct Switch {
    action_id: String,
    checked: bool,
    state: ControlState,
    palette: ComponentPalette,
}

impl Switch {
    /// An on/off control that shows `checked` and reports `Clicked` with this
    /// ID. The host flips its own value.
    pub fn new(action_id: impl Into<String>, checked: bool, palette: ComponentPalette) -> Self {
        Self {
            action_id: action_id.into(),
            checked,
            state: ControlState::Enabled,
            palette,
        }
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for Switch {
    fn render(self) -> Element {
        const TRACK_WIDTH: f32 = 36.0;
        const TRACK_HEIGHT: f32 = 20.0;
        const THUMB: f32 = 14.0;
        let thumb_color = if self.checked {
            self.palette.surface
        } else {
            self.palette.text
        };
        let track_color = if self.checked {
            self.palette.text
        } else {
            self.palette.selected
        };
        // The thumb is the strip's selection indicator: it slides between the
        // two slots when `checked` changes and keeps its place across rebuilds.
        let slot = |name: &str| {
            Element::spacer()
                .size(Some(THUMB), Some(THUMB))
                .indicator_slot(name)
        };
        let contents = Element::tab_strip(
            vec![
                slot("off"),
                Element::spacer().size(Some(TRACK_WIDTH - THUMB * 2.0 - 6.0), Some(THUMB)),
                slot("on"),
            ],
            Vec::new(),
        )
        .tab_selection(
            format!("{}:thumb", self.action_id),
            if self.checked { "on" } else { "off" },
            thumb_color,
            THUMB / 2.0,
            crate::tokens().motion.move_ms,
        );
        let enabled = self.state == ControlState::Enabled;
        let track = if enabled {
            Element::clickable(self.action_id, contents)
                .hover_background(lighter(track_color, 10))
                .pressed_background(lighter(track_color, 16))
                .transition_ms(hover_ms())
        } else {
            Element::panel(contents)
        };
        track
            .background(track_color)
            .padding(3.0)
            .radius(TRACK_HEIGHT / 2.0)
            .size(Some(TRACK_WIDTH), Some(TRACK_HEIGHT))
    }
}

pub(crate) const fn lighter(color: Color, amount: u8) -> Color {
    Color::rgba(
        color.r.saturating_add(amount),
        color.g.saturating_add(amount),
        color.b.saturating_add(amount),
        color.a,
    )
}

pub(crate) const fn darker(color: Color, amount: u8) -> Color {
    Color::rgba(
        color.r.saturating_sub(amount),
        color.g.saturating_sub(amount),
        color.b.saturating_sub(amount),
        color.a,
    )
}

/// One selectable tab. `action_id` is a stable ID handled by the host's
/// `UiAction::Clicked`; it can represent any domain value (file, view, task).
pub struct Tab {
    pub action_id: String,
    pub label: String,
    pub icon: Option<Icon>,
    pub active: bool,
    pub width: f32,
    pub state: ControlState,
}

impl Tab {
    /// A tab of this width that reports `Clicked` with this ID.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>, width: f32) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            active: false,
            // Never under 40, and 40 when it is not a number.
            width: if width.is_finite() { width.max(40.0) } else { 40.0 },
            state: ControlState::Enabled,
        }
    }

    /// Show an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark this tab as the chosen one.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Disable the tab: it keeps its place and cannot be chosen.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled { ControlState::Enabled } else { ControlState::Disabled };
        self
    }
}

/// A compact tab strip. It owns appearance, while the host owns navigation.
const TAB_GAP: f32 = 2.0;
const TAB_TRACK_PADDING: f32 = 2.0;
const TAB_ITEM_PADDING: f32 = 4.0;
const TAB_ICON_GAP: f32 = 4.0;
const TAB_FONT_SIZE: f32 = 11.5;

/// A row of [`Tab`]s with a selection that slides to the active one.
pub struct Tabs {
    pub items: Vec<Tab>,
    pub palette: ComponentPalette,
    pub height: f32,
    pub gap: f32,
    pub id: Option<String>,
    pub animation_ms: u16,
}

impl Tabs {
    /// A row of these tabs, in order.
    pub fn new(items: Vec<Tab>, palette: ComponentPalette) -> Self {
        Self {
            items,
            palette,
            height: ControlSize::Dense.height(),
            gap: TAB_GAP,
            id: None,
            animation_ms: crate::tokens().motion.move_ms,
        }
    }

    /// A stable ID for the strip's selection animation. Useful when tabs are
    /// dynamically inserted, removed, or reordered.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// How long the selection takes to slide to another tab, in milliseconds,
    /// in place of the `move_ms` token.
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    /// Set the height, in logical pixels, never below 24. A height that is
    /// not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(24.0);
        }
        self
    }

    /// Take the height of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.height = size.height();
        self
    }

    /// Give every tab the same width so the track is `total` wide, whatever
    /// width each tab was built with. A total that is not finite is ignored.
    pub fn fill_width(mut self, total: f32) -> Self {
        if total.is_finite() && !self.items.is_empty() {
            let count = self.items.len() as f32;
            let width = (total - TAB_TRACK_PADDING * 2.0 - self.gap * (count - 1.0)) / count;
            for item in &mut self.items {
                // The same floor as `Tab::new`.
                item.width = width.max(40.0);
            }
        }
        self
    }
}

impl Component for Tabs {
    fn render(self) -> Element {
        let selected_id = self
            .items
            .iter()
            .find(|item| item.active && item.state == ControlState::Enabled)
            .map(|item| item.action_id.clone());
        let enabled_ids = self
            .items
            .iter()
            .filter(|item| item.state == ControlState::Enabled)
            .map(|item| item.action_id.clone())
            .collect();
        let group_id = self.id.unwrap_or_else(|| {
            self.items
                .first()
                .map(|item| format!("tabs:{}", item.action_id))
                .unwrap_or_else(|| "tabs:empty".to_owned())
        });
        let items = self
            .items
            .into_iter()
            .map(|item| {
                let ink = if item.active {
                    self.palette.text
                } else {
                    self.palette.muted
                };
                // The shared indicator paints below the controls and moves
                // between them. Individual surfaces stay transparent so they
                // cannot hide its intermediate position.
                let fill = Color::TRANSPARENT;
                // A tab is a text treatment above the shared selection
                // indicator. Hover changes ink only; another fill would
                // obscure the sliding indicator and make the track look
                // uneven. The active tab retains its selected surface.
                let hover_ink = if item.active {
                    lighter(self.palette.text, 12)
                } else {
                    self.palette.text
                };
                if item.state == ControlState::Disabled {
                    let label = Element::text(item.label)
                        .foreground(self.palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(TAB_FONT_SIZE);
                    let visual = if let Some(icon) = item.icon {
                        Element::row(vec![
                            Element::icon(icon).foreground(self.palette.muted),
                            label,
                        ])
                        .gap(TAB_ICON_GAP)
                    } else {
                        label
                    };
                    return Element::row(vec![
                        Element::spacer().flex(1.0),
                        visual,
                        Element::spacer().flex(1.0),
                    ])
                    .padding(TAB_ITEM_PADDING)
                    .background(Color::TRANSPARENT)
                    .radius(control_radius())
                    .size(Some(item.width), Some(self.height));
                }
                let button = if let Some(icon) = item.icon {
                    Element::icon_button(item.action_id, icon, item.label)
                } else {
                    Element::button(item.action_id, item.label)
                };
                button
                    .background(fill)
                    .hover_foreground(hover_ink)
                    .pressed_foreground(hover_ink)
                    .foreground(ink)
                    .font_family(FontFamily::Sans)
                    .font_size(TAB_FONT_SIZE)
                    .padding(TAB_ITEM_PADDING)
                    .gap(TAB_ICON_GAP)
                    .radius(control_radius())
                    .center_content()
                    .size(Some(item.width), Some(self.height))
            })
            .collect();
        let strip = Element::tab_strip(items, enabled_ids)
            .gap(self.gap)
            .padding(TAB_TRACK_PADDING)
            .background(self.palette.surface)
            .radius(surface_radius())
            .size(None, Some(self.height + TAB_TRACK_PADDING * 2.0));
        if let Some(selected_id) = selected_id {
            strip.tab_selection(
                group_id,
                selected_id,
                self.palette.selected,
                control_radius(),
                self.animation_ms,
            )
        } else {
            strip
        }
    }
}

/// Tabs in one rounded track. Each item remains a normal tab action, so the
/// existing keyboard navigation and focus model also applies to segmented
/// selectors. The caller controls the selected item and every visual color.
pub struct SegmentedControl {
    items: Vec<Tab>,
    palette: ComponentPalette,
    height: f32,
}

impl SegmentedControl {
    /// A control of these segments, in order. Each is a [`Tab`].
    pub fn new(items: Vec<Tab>, palette: ComponentPalette) -> Self {
        Self {
            items,
            palette,
            height: 28.0,
        }
    }

    /// Set the height, in logical pixels, never below 28. A height that is
    /// not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(28.0);
        }
        self
    }
}

impl Component for SegmentedControl {
    fn render(self) -> Element {
        let inner_width = self.items.iter().map(|item| item.width).sum::<f32>()
            + TAB_GAP * self.items.len().saturating_sub(1) as f32;
        Tabs::new(self.items, self.palette)
            .height(self.height - TAB_TRACK_PADDING * 2.0)
            .render()
            .size(
                Some(inner_width + TAB_TRACK_PADDING * 2.0),
                Some(self.height),
            )
    }
}

/// A floating slot anchored to any element. The child may itself be a
/// dropdown, command list, form, or another application-specific component.
/// While open, Tab stays among visible controls in the floating surface;
/// Escape or backdrop dismissal returns focus to the first focusable anchor.
pub struct Popover {
    pub anchor: Element,
    pub content: Element,
    pub open: bool,
    pub dismiss_id: String,
    pub placement: PopoverPlacement,
    pub width: f32,
    pub palette: ComponentPalette,
    pub animation_ms: u16,
    pub close_animation_ms: Option<u16>,
    pub open_scale: f32,
    pub view_key: Option<String>,
}

impl Popover {
    pub fn new(
        anchor: Element,
        content: Element,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            content,
            open,
            dismiss_id: dismiss_id.into(),
            placement: PopoverPlacement::BottomStart,
            width: 240.0,
            palette,
            animation_ms: crate::tokens().motion.enter_ms,
            close_animation_ms: None,
            open_scale: 1.0,
            view_key: None,
        }
    }

    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    pub fn close_animation_ms(mut self, duration: u16) -> Self {
        self.close_animation_ms = Some(duration);
        self
    }

    pub fn open_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.open_scale = scale.clamp(0.5, 1.25);
        }
        self
    }

    pub fn view_key(mut self, key: impl Into<String>) -> Self {
        self.view_key = Some(key.into());
        self
    }
}

impl Component for Popover {
    fn render(self) -> Element {
        if !self.open {
            return self.anchor;
        }
        let floating = Element::panel(self.content)
            .padding(6.0)
            .size(Some(self.width), None)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(surface_radius())
            .shadow(self.palette.shadow, Point::new(0.0, 9.0), 22.0, 0.0);
        let mut popover = Element::popover(self.anchor, floating, self.dismiss_id, self.placement)
            .popover_animation_ms(self.animation_ms)
            .popover_open_scale(self.open_scale);
        if let Some(duration) = self.close_animation_ms {
            popover = popover.popover_close_animation_ms(duration);
        }
        if let Some(key) = self.view_key {
            popover = popover.popover_view_key(key);
        }
        popover
    }
}

/// One option in a dropdown list; its ID is the host action to execute.
pub struct DropdownOption {
    pub action_id: String,
    pub label: String,
    pub icon: Option<Icon>,
    pub selected: bool,
    pub state: ControlState,
}

impl DropdownOption {
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            state: ControlState::Enabled,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled { ControlState::Enabled } else { ControlState::Disabled };
        self
    }
}

/// A dropdown with caller-owned selection/open state and stable action IDs.
/// The anchor is a slot, so callers can supply an icon button, field, or any
/// other visual trigger without changing the list behavior. Arrow keys,
/// Home/End and Tab move through enabled options and reveal offscreen rows;
/// Enter activates the focused option. The host still owns the open state and
/// handles the resulting ID.
pub struct DropdownList {
    pub anchor: Element,
    pub options: Vec<DropdownOption>,
    pub open: bool,
    pub dismiss_id: String,
    pub width: f32,
    pub placement: PopoverPlacement,
    pub palette: ComponentPalette,
    pub motion: MenuMotion,
    pub sizing: Option<crate::SelectorSizing>,
}

impl DropdownList {
    pub fn new(
        anchor: Element,
        options: Vec<DropdownOption>,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            options,
            open,
            dismiss_id: dismiss_id.into(),
            width: 240.0,
            placement: PopoverPlacement::BottomStart,
            palette,
            motion: MenuMotion::default(),
            sizing: None,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Share the width, row density and scroll budget of other selectors.
    /// Call before `width` to override the preferred width for the host.
    pub fn sizing(mut self, sizing: crate::SelectorSizing) -> Self {
        self.width = sizing.preferred_width;
        self.sizing = Some(sizing);
        self
    }

    pub fn motion(mut self, motion: MenuMotion) -> Self {
        self.motion = motion;
        self
    }
}

impl Component for DropdownList {
    fn render(self) -> Element {
        let entries = self
            .options
            .into_iter()
            .map(|option| {
                let mut item =
                    DropdownMenuItem::new(option.action_id, option.label).selected(option.selected);
                if let Some(icon) = option.icon {
                    item = item.icon(icon);
                }
                if option.state == ControlState::Disabled {
                    item = item.disabled();
                }
                DropdownMenuEntry::Item(item)
            })
            .collect();
        let mut menu = DropdownMenu::new(
            self.anchor,
            entries,
            self.open,
            self.dismiss_id,
            self.palette,
        );
        if let Some(sizing) = self.sizing {
            menu = menu.sizing(sizing);
        }
        menu.width(self.width)
            .placement(self.placement)
            .motion(self.motion)
            .render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, ElementKind, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn a_captioned_shortcut_uses_one_budget_for_keys_separators_and_caption() {
        let palette = ComponentPalette::neutral_dark();
        for menu in [false, true] {
            let key_width = |keys: &str| {
                if menu {
                    crate::shortcut::menu_label(keys).chars().count() as f32 * 7.0
                } else {
                    crate::shortcut::width(keys, |key| key.chars().count() as f32 * 7.0)
                }
            };
            let width = key_width("Enter") + key_width("Tab") + 7.0 + 42.0 + 12.0;
            for (budget, visible) in [(0.0, false), (width - 1.0, false), (width, true)] {
                let mut hint = Kbd::new("Enter", palette)
                    .alternative("Tab")
                    .caption("Accept")
                    .max_width(budget);
                if menu {
                    hint = hint.menu();
                }
                let mut tree = UiTree::new(
                    Element::row(vec![hint.render(), Element::text("after")]).gap(4.0),
                );
                tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
                let frame = tree.layout(Size::new(400.0, 28.0));
                let texts: Vec<_> = frame.commands.iter()
                    .filter_map(|command| match command {
                        DrawCommand::Text { text, position, .. } => Some((text.as_str(), position.x)),
                        _ => None,
                    }).collect();
                assert_eq!(texts.iter().any(|(text, _)| *text == "Accept"), visible);
                assert_eq!(texts.iter().any(|(text, _)| *text == "/"), visible);
                assert_eq!(texts.iter().any(|(text, _)| *text == "Enter"), visible);
                assert_eq!(texts.iter().any(|(text, _)| *text == "Tab"), visible);
                assert_eq!(
                    texts.iter().find(|(text, _)| *text == "after").unwrap().1,
                    if visible { width + 4.0 } else { 4.0 },
                );
            }
        }
    }

    #[test]
    fn a_captioned_shortcut_disappears_and_returns_as_the_row_resizes() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Element::row(vec![
                Element::text("Action").min_size(60.0, 0.0).flex(1.0),
                Kbd::new("Enter", palette).alternative("Tab").caption("Accept").render(),
            ]).gap(4.0),
        );
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
        for (width, visible) in [(400.0, true), (100.0, false), (400.0, true)] {
            let frame = tree.layout(Size::new(width, 28.0));
            let texts: Vec<_> = frame.commands.iter().filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            }).collect();
            assert_eq!(texts.contains(&"Accept"), visible);
            assert_eq!(texts.contains(&"/"), visible);
            assert_eq!(texts.contains(&"Enter"), visible);
            assert_eq!(texts.contains(&"Tab"), visible);
            assert!(texts.contains(&"Action"));
        }
    }

    #[test]
    fn a_transition_asks_for_the_next_refresh_and_a_spinner_for_a_slower_cadence() {
        use std::time::Duration;
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(200.0, 60.0);

        let mut resting = UiTree::new(Button::new("save", "Save", palette).render());
        resting.layout(size);
        assert_eq!(resting.animation_repaint_after(), None, "an idle window costs nothing");

        // Hovering starts a fill transition: every display refresh until it ends.
        resting.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
        resting.layout(size);
        assert_eq!(resting.animation_repaint_after(), Some(Duration::ZERO));

        // A spinner never finishes, so it does not run at display rate.
        let mut spinning = UiTree::new(crate::Spinner::new("busy", palette.text).render());
        spinning.layout(size);
        assert_eq!(spinning.animation_repaint_after(), Some(Duration::from_millis(16)));

        // With movement switched off the hover no longer animates.
        let mut still = UiTree::new(Button::new("save", "Save", palette).render());
        still.set_reduced_motion(true);
        still.layout(size);
        still.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
        still.layout(size);
        assert_eq!(still.animation_repaint_after(), None);
    }

    #[test]
    fn large_buttons_center_their_label_and_keep_their_icon() {
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(Element::row(vec![
            Button::new("large", "Large", palette).button_size(ButtonSize::Large).render(),
            Button::new("default", "Default", palette).button_size(ButtonSize::Default).render(),
            IconButton::new("large-icon", Icon::Add, palette)
                .button_size(ButtonSize::Large)
                .render(),
            Element::spacer().flex(1.0),
        ]))
        .layout(Size::new(400.0, 40.0));
        let label = |wanted: &str| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { position, text, .. } if text == wanted => Some(position.y),
                    _ => None,
                })
                .expect("label")
        };
        // Padding deeper than half the height used to push the label down:
        // both labels sit the same distance above their button's middle.
        assert!((label("Large") - 20.0 - (label("Default") - 18.0)).abs() < 0.1);
        let icon = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon { icon: Icon::Add, rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("icon");
        assert_eq!((icon.width, icon.height), (ICON_SIZE, ICON_SIZE));
        assert!((icon.y + icon.height / 2.0 - 20.0).abs() < 0.5);
    }

    #[test]
    fn search_field_taller_than_its_line_centers_the_text_and_disabled_labels_fit() {
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(Element::column(vec![
            SearchField::new("search", "", "Buscar", palette).shortcut("Ctrl+K").width(300.0).render(),
            Element::row(vec![
                Button::new("off", "Destructive", palette)
                    .variant(ButtonVariant::Destructive)
                    .disabled()
                    .render(),
                Element::spacer().flex(1.0),
            ]),
        ]))
        .layout(Size::new(400.0, 200.0));
        let field = frame.regions.iter().find(|region| region.id == "search").expect("field").rect;
        let text = |wanted: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == wanted => Some(*position),
                _ => None,
            })
        };
        let placeholder = text("Buscar").expect("placeholder");
        let line = 14.0 * 13.0 / 14.0;
        assert!(field.height > 32.0);
        assert!(
            (placeholder.y - field.y - (field.height - line) / 2.0).abs() < 4.0
                && placeholder.y - field.y > 8.5,
            "placeholder at {} in a field of {}",
            placeholder.y - field.y,
            field.height
        );
        assert!(text("Destructive").is_some(), "disabled label was truncated");
    }

    #[test]
    fn shared_hover_reaches_tooltip_trigger_without_changing_hits_or_enabling_disabled_actions() {
        use web_time::{Duration, Instant};
        let palette = ComponentPalette::neutral_dark();
        let action = |id, enabled| {
            palette.interactive_surface(
                Button::new(id, "", palette)
                    .icon(Icon::Refresh)
                    .button_size(ButtonSize::IconSmall)
                    .variant(ButtonVariant::Ghost)
                    .enabled(enabled)
                    .tooltip("Actualizar")
                    .render(),
            )
        };
        let mut tree =
            UiTree::new(Element::row(vec![action("ready", true), action("busy", false)]).gap(8.0));
        let viewport = Size::new(160.0, 48.0);
        let start = Instant::now();
        let initial = tree.layout_at(viewport, start);
        let bounds = initial
            .regions
            .iter()
            .find(|region| region.id == "ready")
            .unwrap()
            .rect;
        let point = Point::new(
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        assert_eq!(bounds.width, 32.0);
        assert_eq!(bounds.height, 32.0);
        assert!(!tree.focus("busy"));
        tree.handle_event(UiEvent::PointerMove(point));
        tree.layout_at(viewport, start);
        let hovered = tree.layout_at(viewport, start + Duration::from_millis(200));
        assert_eq!(hovered.hit_test(point), Some("ready"));
        assert!(hovered.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, radius }
                if *rect == bounds && *color == palette.hover && *radius == control_radius()
        )));
        tree.handle_event(UiEvent::PointerDown(point));
        tree.layout_at(viewport, start + Duration::from_millis(200));
        let pressed = tree.layout_at(viewport, start + Duration::from_millis(400));
        assert!(pressed.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. } if *rect == bounds && *color == palette.selected
        )));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "ready".into() }]
        );
        assert!(!tree.focus("busy"));
    }

    #[test]
    fn compact_hover_shape_and_hit_targets_remain_separate_from_neighbors_and_circles() {
        let palette = ComponentPalette::neutral_dark();
        let manual = palette.interactive_surface(
            Tooltip::new(
                Element::icon_button("manual", Icon::Refresh, "")
                    .foreground(palette.text)
                    .size(Some(24.0), Some(24.0)),
                "manual",
                "Actualizar",
                palette,
            )
            .render(),
        );
        let native = IconButton::new("native", Icon::Settings, palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .render();
        // A deliberate circle remains available to avatars, send controls,
        // and other host-specific shapes after applying the common control.
        let circle = IconButton::new("circle", Icon::ArrowUp, palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .render()
            .radius(12.0);
        let mut tree = UiTree::new(Element::row(vec![manual, native, circle]).gap(6.0));
        tree.set_reduced_motion(true);
        let viewport = Size::new(140.0, 48.0);
        let idle = tree.layout(viewport);
        let targets: Vec<_> = ["manual", "native", "circle"]
            .into_iter()
            .map(|id| {
                let rect = idle
                    .regions
                    .iter()
                    .find(|region| region.id == id)
                    .unwrap()
                    .rect;
                assert_eq!((rect.width, rect.height), (24.0, 24.0));
                (id, rect)
            })
            .collect();
        for pair in targets.windows(2) {
            assert!(pair[0].1.x + pair[0].1.width < pair[1].1.x);
        }
        for (id, target) in &targets {
            let point = Point::new(
                target.x + target.width / 2.0,
                target.y + target.height / 2.0,
            );
            tree.handle_event(UiEvent::PointerMove(point));
            let hovered = tree.layout(viewport);
            assert_eq!(hovered.regions, idle.regions);
            assert_eq!(hovered.hit_test(point), Some(*id));
            let fills: Vec<_> = hovered
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *color == palette.hover => Some((*rect, *radius)),
                    _ => None,
                })
                .collect();
            assert_eq!(
                fills.len(),
                1,
                "moving across controls must leave one hover"
            );
            assert_eq!(fills[0].0, *target);
            if *id == "circle" {
                assert_eq!(fills[0].1, target.height / 2.0);
            } else {
                assert!(
                    fills[0].1 > 0.0 && fills[0].1 < target.height / 4.0,
                    "compact chrome keeps straight edges"
                );
            }
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: (*id).into() }]
            );
        }
        let gap = Point::new(
            targets[0].1.x + targets[0].1.width + 3.0,
            targets[0].1.y + 12.0,
        );
        assert!(tree.layout(viewport).hit_test(gap).is_none());
    }

    #[test]
    fn loading_button_animates_in_the_icon_slot_and_blocks_duplicate_actions() {
        let palette = ComponentPalette::neutral_dark();
        let button = |loading| {
            Button::new("refresh", "Actualizar modelos", palette)
                .icon(Icon::Refresh)
                .variant(ButtonVariant::Outline)
                .size(ControlSize::Compact)
                .loading(loading)
                .render()
        };
        let root = |loading| Element::row(vec![button(loading), Element::spacer().flex(1.0)]);
        let viewport = Size::new(400.0, 40.0);
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(root(false));
        let idle = tree.layout_at(viewport, start);
        let rect = idle
            .regions
            .iter()
            .find(|region| region.id == "refresh")
            .unwrap()
            .rect;
        tree.set_root(root(true));
        let busy = tree.layout_at(viewport, start);
        assert!(!busy.regions.iter().any(|region| region.id == "refresh"));
        assert!(!tree.focus("refresh"));
        assert!(busy.commands.iter().any(|command| matches!(command,
            DrawCommand::StrokeRect { rect: bounds, .. } if *bounds == rect
        )));
        assert!(busy.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Actualizar modelos"
        )));
        assert!(!busy.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                icon: Icon::Refresh,
                ..
            }
        )));
        let phase = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Spinner {
                        rect: spinner,
                        angle,
                        variant: crate::SpinnerVariant::Circle,
                        ..
                    } => {
                        assert_eq!(spinner.width, 16.0);
                        assert_eq!(spinner.height, 16.0);
                        assert!(
                            spinner.x >= rect.x && spinner.x + spinner.width <= rect.x + rect.width
                        );
                        assert!(
                            spinner.y >= rect.y
                                && spinner.y + spinner.height <= rect.y + rect.height
                        );
                        Some(*angle)
                    }
                    _ => None,
                })
                .expect("standard circular loading spinner")
        };
        let first = phase(&busy);
        assert!(tree.animation_repaint_after().is_some());
        let later = tree.layout_at(viewport, start + std::time::Duration::from_millis(240));
        assert_ne!(phase(&later), first);
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        // Move outside before restoring the button so a hover transition does
        // not contribute repaint requests after the spinner is removed.
        tree.handle_event(UiEvent::PointerMove(Point::new(399.0, 39.0)));
        tree.set_root(root(false));
        let restored = tree.layout_at(viewport, start + std::time::Duration::from_secs(1));
        assert_eq!(
            restored
                .regions
                .iter()
                .find(|region| region.id == "refresh")
                .unwrap()
                .rect,
            rect
        );
        assert!(!restored
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Spinner { .. })));
        assert_eq!(tree.animation_repaint_after(), None);
        assert!(tree.focus("refresh"));
    }

    #[test]
    fn reference_variants_have_distinct_semantic_treatments() {
        let palette = ComponentPalette::neutral_dark();
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Secondary,
            ButtonVariant::Outline,
            ButtonVariant::Ghost,
            ButtonVariant::Destructive,
            ButtonVariant::Link,
            ButtonVariant::Warning,
            ButtonVariant::Success,
        ] {
            let element = Button::new("action", "Action", palette)
                .variant(variant)
                .render();
            assert!(element.style.center_content);
            assert_eq!(
                element.style.underline_on_hover,
                variant == ButtonVariant::Link
            );
            assert_eq!(
                element.style.border.is_some(),
                variant == ButtonVariant::Outline
            );
            assert_eq!(
                element.style.background,
                Some(match variant {
                    ButtonVariant::Default => palette.text,
                    ButtonVariant::Secondary => palette.raised,
                    ButtonVariant::Outline => palette.surface,
                    ButtonVariant::Ghost | ButtonVariant::Link => Color::TRANSPARENT,
                    ButtonVariant::Destructive => Color::rgb(161, 67, 71),
                    ButtonVariant::Warning => Color::rgb(209, 154, 55),
                    ButtonVariant::Success => Color::rgb(47, 140, 94),
                    _ => unreachable!(),
                })
            );
        }
    }

    #[test]
    fn a_path_hint_names_the_file_over_the_directory_that_holds_it() {
        let hint = |path: &str| {
            let palette = ComponentPalette::neutral_dark();
            match Tooltip::for_path(Element::spacer(), "file", path, palette).render().kind {
                ElementKind::Tooltip { anchor_id, title, sections, max_width, .. } => {
                    assert_eq!((anchor_id.as_str(), max_width), ("file", 320.0));
                    (title, sections)
                }
                _ => panic!("a tooltip"),
            }
        };
        let described = |text: &str| vec![TooltipSection::Description(text.into())];
        assert_eq!(hint("crates/limn/src/tree.rs"), ("tree.rs".into(), described("crates/limn/src")));
        assert_eq!(hint("C:\\src\\main.rs"), ("main.rs".into(), described("C:\\src")));
        // The root is its own directory, and a bare name has none.
        assert_eq!(hint("/etc"), ("etc".into(), described("/")));
        assert_eq!(hint("Cargo.toml"), ("Cargo.toml".into(), Vec::new()));
        assert_eq!(hint(""), (String::new(), Vec::new()));
        // A directory written with its closing separator is still named.
        assert_eq!(hint("crates/limn/src/"), ("src".into(), described("crates/limn")));
        assert_eq!(hint("C:\\dir\\"), ("dir".into(), described("C:")));
        assert_eq!(hint("/"), ("/".into(), Vec::new()));
    }

    #[test]
    fn tooltip_spec_fields_reach_the_hint() {
        let palette = ComponentPalette::neutral_dark();
        let other = ComponentPalette {
            border: Color::rgb(90, 90, 90),
            ..palette
        };
        let spec = TooltipSpec::new("Guardar")
            .description("Escribe el archivo")
            .shortcut("Ctrl+S")
            .shortcut(" ")
            .placement(TooltipPlacement::Below)
            .palette(other)
            .max_width(f32::NAN)
            .max_width(200.0);
        let element = IconButton::new("save", Icon::Check, palette)
            .tooltip_with(spec.clone())
            .render();
        let ElementKind::Tooltip {
            anchor_id,
            title,
            shortcut,
            sections,
            max_width,
            placement,
            colors,
            ..
        } = &element.kind
        else {
            panic!("a button with a hint renders a tooltip");
        };
        assert_eq!((anchor_id.as_str(), title.as_str()), ("save", "Guardar"));
        assert_eq!(shortcut.as_deref(), Some("Ctrl+S"));
        assert_eq!(
            sections,
            &vec![crate::TooltipSection::Description("Escribe el archivo".into())]
        );
        assert_eq!(*max_width, 200.0);
        assert_eq!(*placement, TooltipPlacement::Below);
        assert_eq!(*colors, other.tooltip_style());
        assert_ne!(*colors, palette.tooltip_style());

        // Suppressed, the button is rendered bare.
        let bare = Button::new("save", "Guardar", palette)
            .tooltip_with(spec.suppressed(true))
            .render();
        assert!(matches!(bare.kind, ElementKind::Button { .. }));
    }

    #[test]
    fn tooltip_shorthands_build_the_same_hint_as_a_spec() {
        let palette = ComponentPalette::neutral_dark();
        let short = Button::new("run", "Run", palette)
            .tooltip_shortcut("F5")
            .tooltip_placement(TooltipPlacement::Right)
            .tooltip("Ejecutar")
            .render();
        let spec = Button::new("run", "Run", palette)
            .tooltip_with(
                TooltipSpec::new("Ejecutar")
                    .shortcut("F5")
                    .placement(TooltipPlacement::Right),
            )
            .render();
        assert_eq!(format!("{short:?}"), format!("{spec:?}"));
    }

    #[test]
    fn a_disabled_button_keeps_its_hint_without_a_click_target() {
        let palette = ComponentPalette::neutral_dark();
        for loading in [false, true] {
            let element = IconButton::new("new", Icon::Add, palette)
                .tooltip("Nueva tarea")
                .enabled(loading)
                .loading(loading)
                .render();
            let ElementKind::Tooltip { anchor, title, .. } = &element.kind else {
                panic!("the passive button keeps its hint");
            };
            assert_eq!(title, "Nueva tarea");
            assert!(matches!(anchor.kind, ElementKind::Panel { .. }));
            let mut tree = UiTree::new(Element::row(vec![element]));
            let frame = tree.layout(Size::new(120.0, 32.0));
            assert_eq!(frame.regions.len(), 1);
            assert_eq!(frame.regions[0].kind, crate::HitKind::Hover);
            assert!(!tree.focus("new"));
            let point = Point::new(16.0, 16.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        }
    }

    #[test]
    fn dispatch_as_reports_the_action_and_keeps_the_widget_id() {
        let palette = ComponentPalette::neutral_dark();
        for with_hint in [false, true] {
            let mut button = IconButton::new("toolbar_close", Icon::Close, palette).dispatch_as("close");
            if with_hint {
                button = button.tooltip("Cerrar");
            }
            let mut tree = UiTree::new(Element::row(vec![button.render()]));
            let frame = tree.layout(Size::new(120.0, 32.0));
            assert_eq!(frame.regions[0].id, "toolbar_close");
            let point = Point::new(16.0, 16.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "close".into() }]
            );
        }
    }

    #[test]
    fn active_hover_lifts_by_default_and_can_follow_the_palette() {
        let palette = ComponentPalette::neutral_dark();
        for variant in [
            ButtonVariant::Ghost,
            ButtonVariant::Subtle,
            ButtonVariant::Raised,
            ButtonVariant::Outline,
        ] {
            let lift = IconButton::new("toggle", Icon::PanelLeft, palette)
                .variant(variant)
                .active(true)
                .render();
            assert_eq!(lift.style.background, Some(palette.selected));
            assert_eq!(lift.style.hover_background, Some(lighter(palette.selected, 10)));
            assert_eq!(lift.style.pressed_background, Some(lighter(palette.selected, 18)));

            let flat = IconButton::new("toggle", Icon::PanelLeft, palette)
                .variant(variant)
                .active(true)
                .active_hover(ActiveHover::Palette)
                .render();
            assert_eq!(flat.style.background, Some(palette.selected));
            assert_eq!(flat.style.hover_background, Some(palette.hover));
            assert_eq!(flat.style.pressed_background, Some(palette.selected));
        }
        // An inactive button is the same in both modes.
        let idle = Button::new("a", "A", palette).variant(ButtonVariant::Ghost);
        let idle_flat = Button::new("a", "A", palette)
            .variant(ButtonVariant::Ghost)
            .active_hover(ActiveHover::Palette);
        assert_eq!(format!("{:?}", idle.render()), format!("{:?}", idle_flat.render()));
    }

    #[test]
    fn icon_only_buttons_are_squares_with_a_centered_glyph() {
        let palette = ComponentPalette::neutral_dark();
        for (size, side) in [
            (ControlSize::Dense, 24.0),
            (ControlSize::Compact, 28.0),
            (ControlSize::Regular, 32.0),
        ] {
            let element = IconButton::new("action", Icon::Add, palette)
                .variant(ButtonVariant::Ghost)
                .size(size)
                .render();
            assert_eq!(element.style.radius, control_radius());
            let frame = UiTree::new(Element::row(vec![element])).layout(Size::new(100.0, side));
            let region = frame.regions[0].rect;
            assert_eq!((region.width, region.height), (side, side));
            let icon = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .expect("glyph");
            assert_eq!((icon.width, icon.height), (ICON_SIZE, ICON_SIZE));
            let inset = (side - ICON_SIZE) / 2.0;
            assert_eq!((icon.x - region.x, icon.y - region.y), (inset, inset));
        }
    }

    #[test]
    fn a_ghost_icon_button_already_wears_the_interactive_surface() {
        let palette = ComponentPalette::neutral_dark();
        let plain = IconButton::new("action", Icon::Add, palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .tooltip("Nuevo")
            .render();
        let ElementKind::Tooltip { anchor, .. } = &plain.kind else {
            panic!("tooltip");
        };
        assert_eq!(anchor.style.hover_background, Some(palette.hover));
        assert_eq!(anchor.style.pressed_background, Some(palette.selected));
        assert_eq!(anchor.style.radius, control_radius());
        assert_eq!(anchor.style.transition_ms, hover_ms());
        let wrapped = palette.interactive_surface(plain.clone());
        assert_eq!(format!("{wrapped:?}"), format!("{plain:?}"));
    }

    #[test]
    fn reference_sizes_define_button_and_icon_hit_bounds() {
        let palette = ComponentPalette::neutral_dark();
        for (size, expected) in [
            (ButtonSize::Default, 36.0),
            (ButtonSize::Small, 32.0),
            (ButtonSize::Large, 40.0),
            (ButtonSize::Icon, 36.0),
            (ButtonSize::IconSmall, 32.0),
            (ButtonSize::IconExtraSmall, 24.0),
        ] {
            let element = IconButton::new("action", Icon::Add, palette)
                .button_size(size)
                .render();
            assert_eq!(element.style.height, Some(expected));
            assert_eq!(element.style.width, Some(expected));
        }
    }

    #[test]
    fn mini_is_a_22_px_action_with_an_11_px_label() {
        let palette = ComponentPalette::neutral_dark();
        let label = Button::new("more", "More", palette)
            .button_size(ButtonSize::Mini)
            .width(36.0)
            .render();
        assert_eq!(label.style.height, Some(22.0));
        assert_eq!(label.style.width, Some(36.0));
        assert_eq!(label.style.font_size, 11.0);
        assert_eq!(label.style.padding, 3.0);
        // The square icon form still has room for the 16 px glyph.
        let icon = IconButton::new("more", Icon::Add, palette)
            .button_size(ButtonSize::Mini)
            .render();
        assert_eq!(icon.style.width, Some(22.0));
        assert_eq!(icon.style.padding, 3.0);
    }

    #[test]
    fn sizing_takes_a_host_profile_until_a_named_size_replaces_it() {
        let palette = ComponentPalette::neutral_dark();
        const NAV: ButtonSizing = ButtonSizing {
            height: 30.0,
            font_size: 14.0,
            icon_size: 16.0,
            horizontal_padding: 5.0,
            inner_spacing: 4.0,
        };
        let label = Button::new("panel", "Panel", palette).sizing(NAV).render();
        assert_eq!(label.style.height, Some(30.0));
        assert_eq!(label.style.font_size, 14.0);
        assert_eq!(label.style.padding, 5.0);
        assert_eq!(label.style.gap, 4.0);
        assert_eq!(label.style.radius, control_radius());
        // A square of the profile's height, with room for the 16 px glyph.
        let icon = IconButton::new("close", Icon::Close, palette)
            .sizing(ButtonSizing { height: 20.0, horizontal_padding: 6.0, ..NAV })
            .render();
        assert_eq!((icon.style.width, icon.style.height), (Some(20.0), Some(20.0)));
        assert_eq!(icon.style.padding, 2.0);
        // A cell flush inside a row has no corners, enabled or passive.
        let cell = || IconButton::new("stage", Icon::Add, palette).radius(RadiusRole::Square);
        assert_eq!(cell().render().style.radius, 0.0);
        assert_eq!(cell().disabled().render().style.radius, 0.0);
        // The passive form keeps the profile's box and label size.
        let mut tree = UiTree::new(Element::row(vec![
            Button::new("panel", "Panel", palette).sizing(NAV).disabled().render(),
            Element::spacer().flex(1.0),
        ]));
        let frame = tree.layout(Size::new(240.0, 30.0));
        assert!(frame.regions.is_empty());
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, .. } if text == "Panel" && *font_size == 14.0)));

        let named = Button::new("panel", "Panel", palette).sizing(NAV).size(ControlSize::Dense).render();
        assert_eq!(named.style.height, Some(24.0));
        let bad = ButtonSizing {
            height: f32::NAN,
            font_size: f32::INFINITY,
            icon_size: -16.0,
            horizontal_padding: -4.0,
            inner_spacing: f32::NEG_INFINITY,
        };
        let bad = Button::new("panel", "Panel", palette).sizing(bad).render();
        assert_eq!(bad.style.height, Some(0.0));
        assert_eq!((bad.style.padding, bad.style.gap), (0.0, 0.0));
        // The element refuses a zero font size and keeps its own.
        assert!(bad.style.font_size.is_finite() && bad.style.font_size > 0.0);
    }

    #[test]
    fn ink_colors_every_enabled_state_and_leaves_disabled_muted() {
        let palette = ComponentPalette::neutral_dark();
        let red = Color::rgb(255, 92, 92);
        let button = |build: fn(Button) -> Button| {
            build(Button::new("drop", "Drop", palette).variant(ButtonVariant::Ghost).ink(Color::rgb(255, 92, 92)))
                .render()
        };
        let plain = button(|button| button);
        assert_eq!(plain.style.foreground, red);
        assert_eq!(plain.style.hover_foreground, Some(red));
        assert_eq!(plain.style.pressed_foreground, Some(red));
        let hovering = button(|button| button.hover_ink(Color::WHITE));
        assert_eq!(hovering.style.foreground, red);
        assert_eq!(hovering.style.hover_foreground, Some(Color::WHITE));

        let mut tree = UiTree::new(button(Button::disabled));
        let frame = tree.layout(Size::new(200.0, 40.0));
        assert!(frame.regions.is_empty());
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "Drop" && *color == palette.muted)));
    }

    #[test]
    fn gap_replaces_the_icon_spacing_and_ignores_bad_input() {
        let palette = ComponentPalette::neutral_dark();
        let button = || Button::new("connect", "Connect", palette).icon(Icon::Add).size(ControlSize::Dense);
        assert_eq!(button().render().style.gap, 6.0);
        assert_eq!(button().gap(8.0).render().style.gap, 8.0);
        assert_eq!(button().gap(f32::NAN).render().style.gap, 6.0);
        assert_eq!(button().gap(-2.0).render().style.gap, 0.0);

        // The busy form keeps the same distance between spinner and label.
        let label_x = |gap: f32| {
            let mut tree = UiTree::new(button().gap(gap).width(160.0).loading(true).render());
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(160.0, 24.0));
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == "Connect" => Some(position.x),
                    _ => None,
                })
                .expect("label")
        };
        assert_eq!(label_x(8.0) - label_x(6.0), 2.0);
    }

    #[test]
    fn radius_takes_a_token_role_and_defaults_to_control() {
        let palette = ComponentPalette::neutral_dark();
        let button = || Button::new("save", "Save", palette);
        assert_eq!(button().render().style.radius, RadiusRole::Control.px());
        for role in [RadiusRole::Surface, RadiusRole::Card, RadiusRole::Panel] {
            assert_eq!(button().radius(role).render().style.radius, role.px());
            assert_eq!(button().radius(role).disabled().render().style.radius, role.px());
        }
    }

    #[test]
    fn round_button_is_a_circle_or_a_pill_at_every_size_and_state() {
        let palette = ComponentPalette::neutral_dark();
        for size in [ControlSize::Compact, ControlSize::Regular] {
            let height = size.button_sizing().height;
            let icon = || IconButton::new("send", Icon::ArrowUp, palette).size(size).round();
            for element in [icon().render(), icon().disabled().render()] {
                assert_eq!(element.style.radius, height / 2.0);
                assert_eq!((element.style.width, element.style.height), (Some(height), Some(height)));
            }
            let pill = Button::new("save", "Save", palette).size(size).radius(RadiusRole::Card).round();
            assert_eq!(pill.render().style.radius, height / 2.0);
        }
    }

    #[test]
    fn search_hint_stays_inside_the_editors_single_hit_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(280.0)
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 36.0));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "query");
        for key in ["Ctrl", "K"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_family: FontFamily::Sans, .. } if text == key)));
        }
        assert!(tree.focus("query"));
    }

    #[test]
    fn search_shortcut_yields_space_to_focus_and_typed_text() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(280.0)
                .render(),
            Element::button("outside", "Otro"),
        ]));
        let has_hint = |frame: &crate::Frame| {
            ["Ctrl", "K"].iter().all(|key| {
                frame
                    .commands
                    .iter()
                    .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == key))
            })
        };
        assert!(has_hint(&tree.layout(Size::new(280.0, 72.0))));
        assert!(tree.focus("query"));
        assert!(!has_hint(&tree.layout(Size::new(280.0, 72.0))));
        tree.handle_event(UiEvent::Text("Rust".to_owned()));
        assert!(tree.focus("outside"));
        assert!(!has_hint(&tree.layout(Size::new(280.0, 72.0))));
        tree.set_input_value("query", "");
        assert!(has_hint(&tree.layout(Size::new(280.0, 72.0))));
    }

    #[test]
    fn shared_text_editors_disable_without_leaving_input_targets() {
        let palette = ComponentPalette::neutral_dark();
        let enabled = UiTree::new(Element::column(vec![
            TextField::new("name", "", "Nombre", palette).render(),
            TextArea::new("description", "", "Descripción", palette)
                .height(72.0)
                .render(),
        ]))
        .layout(Size::new(320.0, 120.0));
        assert!(enabled.regions.iter().any(|region| region.id == "name"));
        assert!(enabled
            .regions
            .iter()
            .any(|region| region.id == "description"));

        let disabled = UiTree::new(Element::column(vec![
            TextField::new("name", "", "Nombre", palette)
                .disabled()
                .render(),
            TextArea::new("description", "", "Descripción", palette)
                .disabled()
                .render(),
        ]))
        .layout(Size::new(320.0, 140.0));
        assert!(disabled.regions.is_empty());
    }

    #[test]
    fn a_text_area_takes_another_radius_token_enabled_and_disabled() {
        let palette = ComponentPalette::neutral_dark();
        let area = || TextArea::new("description", "", "Descripción", palette);
        assert_eq!(area().render().style.radius, RadiusRole::Control.px());
        for area in [area(), area().disabled()] {
            let rounded = area.radius(RadiusRole::Surface).render();
            assert_eq!(rounded.style.radius, RadiusRole::Surface.px());
        }
    }

    #[test]
    fn keyboard_shortcuts_preserve_chords_and_literal_keys_without_hit_targets() {
        let palette = ComponentPalette::neutral_dark();
        for (keys, expected) in [
            ("Ctrl++", vec!["Ctrl", "+"]),
            ("Ctrl+Shift+M", vec!["Ctrl", "Shift", "M"]),
            ("Ctrl+K Ctrl+C", vec!["Ctrl", "K", "Ctrl", "C"]),
            ("Esc", vec!["Esc"]),
            ("Alt+←", vec!["Alt", "←"]),
            ("↑↓", vec!["↑", "↓"]),
            ("", vec![]),
        ] {
            let kbd = Kbd::new(keys, palette).render();
            let frame = UiTree::new(Element::row(vec![kbd])).layout(Size::new(180.0, 28.0));
            let shown: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    // The plus joining two keys is drawn fainter than a key.
                    DrawCommand::Text { color, .. }
                        if *color == crate::shortcut::separator_ink(palette.text) => None,
                    DrawCommand::Text {
                        text, font_family, ..
                    } => {
                        assert_eq!(*font_family, FontFamily::Sans);
                        Some(text.as_str())
                    }
                    DrawCommand::Icon { icon: Icon::ArrowUp, .. } => Some("↑"),
                    DrawCommand::Icon { icon: Icon::ArrowDown, .. } => Some("↓"),
                    DrawCommand::Icon { icon: Icon::ArrowLeft, .. } => Some("←"),
                    DrawCommand::Icon { icon: Icon::ArrowRight, .. } => Some("→"),
                    _ => None,
                })
                .collect();
            assert_eq!(shown, expected);
            assert!(frame.regions.is_empty());
        }
    }

    #[test]
    fn arrow_shortcuts_use_centered_icons_independent_of_font_glyphs() {
        let palette = ComponentPalette::neutral_dark();
        for (keys, expected) in [
            ("↑↓", vec![Icon::ArrowUp, Icon::ArrowDown]),
            ("Alt+←", vec![Icon::ArrowLeft]),
            ("→", vec![Icon::ArrowRight]),
        ] {
            let mut tree = UiTree::new(Element::row(vec![
                Kbd::new(keys, palette).max_width(100.0).render(),
                Element::button("next", "Next").size(Some(40.0), Some(28.0)),
            ]));
            tree.set_text_measurer(|text, _, _| {
                assert!(!text.chars().any(|character| matches!(character, '↑' | '↓' | '←' | '→')));
                text.chars().count() as f32 * 6.0
            });
            let frame = tree.layout(Size::new(160.0, 28.0));
            let caps: Vec<_> = frame.commands.iter().filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if *color == crate::shortcut::cap_fill(palette.text) => Some(*rect),
                _ => None,
            }).collect();
            let icons: Vec<_> = frame.commands.iter().filter_map(|command| match command {
                DrawCommand::Icon { icon, rect, tint } => {
                    assert_eq!(*tint, palette.text);
                    let cap = caps.iter().find(|cap| {
                        rect.x >= cap.x && rect.x + rect.width <= cap.x + cap.width
                    }).expect("arrow belongs to a complete keycap");
                    assert!((rect.x + rect.width / 2.0 - cap.x - cap.width / 2.0).abs() < 0.01);
                    assert!((rect.y + rect.height / 2.0 - cap.y - cap.height / 2.0).abs() < 0.01);
                    Some(*icon)
                }
                _ => None,
            }).collect();
            assert_eq!(icons, expected);
            let next = frame.regions.iter().find(|region| region.id == "next").unwrap();
            assert_eq!(frame.regions.len(), 1);
            assert!(caps.iter().all(|cap| cap.x + cap.width <= next.rect.x));
        }
    }

    #[test]
    fn a_captioned_hint_is_keycaps_a_slash_between_alternatives_and_a_small_muted_caption() {
        let palette = ComponentPalette::neutral_dark();
        let hint = Kbd::new("Enter", palette)
            .alternative("Tab")
            .caption("Accept")
            .render();
        assert_eq!(hint.style.height, Some(20.0));
        let frame = UiTree::new(Element::row(vec![hint, Element::spacer().flex(1.0)]))
            .layout(Size::new(300.0, 20.0));
        let texts: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text,
                    position,
                    color,
                    font_size,
                    ..
                } => Some((text.as_str(), position.x, *color, *font_size)),
                _ => None,
            })
            .collect();
        let labels: Vec<_> = texts.iter().map(|text| text.0).collect();
        assert_eq!(labels, ["Enter", "/", "Tab", "Accept"]);
        assert!(texts.windows(2).all(|pair| pair[0].1 < pair[1].1));
        for (label, _, color, size) in &texts {
            if ["/", "Accept"].contains(label) {
                assert_eq!((*color, *size), (palette.muted, 11.0));
            }
        }
        let caps = frame
            .commands
            .iter()
            .filter(|command| matches!(command,
                DrawCommand::FillRect { color, .. }
                    if *color == crate::shortcut::cap_fill(palette.text)))
            .count();
        assert_eq!(caps, 2);
        assert!(frame.regions.is_empty(), "a hint is passive");
    }

    #[test]
    fn menu_keyboard_shortcuts_are_plain_measured_and_all_or_nothing() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::row(vec![
            Element::spacer().flex(1.0),
            Kbd::new("Ctrl+K Ctrl+C", palette).menu().render(),
        ]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        let frame = tree.layout(Size::new(200.0, 26.0));
        assert!(frame.regions.is_empty());
        assert_eq!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(command,
                    DrawCommand::Text { text, font_size, .. }
                        if text == "Ctrl+K Ctrl+C" && *font_size == crate::shortcut::MENU_FONT_SIZE
                ))
                .count(),
            1
        );
        assert!(!frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { .. } | DrawCommand::StrokeRect { .. }
        )));
        tree.set_root(
            Kbd::new("Ctrl+K Ctrl+C", palette)
                .menu()
                .max_width(30.0)
                .render(),
        );
        assert!(tree.layout(Size::new(200.0, 26.0)).commands.is_empty());
    }

    #[test]
    fn bounded_keyboard_shortcuts_omit_the_whole_chord_and_empty_badges() {
        let palette = ComponentPalette::neutral_dark();
        for keys in ["Ctrl+Shift+M", ""] {
            let frame = UiTree::new(Kbd::new(keys, palette).max_width(35.0).render())
                .layout(Size::new(200.0, 28.0));
            assert!(
                frame.commands.is_empty(),
                "unexpected partial badge: {keys}"
            );
        }
    }

    #[test]
    fn disabled_search_retains_complete_caps_within_compact_control() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .size(ControlSize::Regular)
                .width(280.0)
                .disabled()
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 32.0));
        for key in ["Ctrl", "K"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == key)));
        }
        for command in &frame.commands {
            if let DrawCommand::FillRect { rect, color, .. } = command {
                if *color == crate::shortcut::cap_fill(palette.text) {
                    assert!(rect.y >= 0.0 && rect.y + rect.height <= 32.0);
                }
            }
        }
        assert!(frame.regions.is_empty());
        assert!(!tree.focus("query"));
        let narrow = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(80.0)
                .disabled()
                .render(),
        )
        .layout(Size::new(80.0, 36.0));
        assert!(!narrow.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K")));
    }

    #[test]
    fn disabled_search_without_explicit_width_bounds_shortcuts_to_parent() {
        let palette = ComponentPalette::neutral_dark();
        for (width, hint_visible) in [(80.0, false), (280.0, true)] {
            let frame = UiTree::new(
                Search::new("query", "", "Buscar", palette)
                    .shortcut("Ctrl+K")
                    .disabled()
                    .render(),
            )
            .layout(Size::new(width, 36.0));
            for key in ["Ctrl", "K"] {
                assert_eq!(
                    frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { text, .. } if text == key)),
                    hint_visible
                );
            }
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text.starts_with('B'))));
            for command in &frame.commands {
                if let DrawCommand::FillRect { rect, color, .. } = command {
                    if *color == crate::shortcut::cap_fill(palette.text) {
                        assert!(rect.x >= 8.0 && rect.x + rect.width <= width - 8.0);
                    }
                }
            }
            assert!(frame.regions.is_empty());
        }
    }

    #[test]
    fn all_tabs_share_the_segmented_track() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(vec![Tab::new("files", "Archivos", 88.0)], palette).render();
        assert_eq!(tabs.style.background, Some(palette.surface));
        assert_eq!(tabs.style.radius, surface_radius());
        assert_eq!(tabs.style.padding, 2.0);
        assert_eq!(tabs.style.height, Some(28.0));
        assert_eq!(tabs.style.gap, 2.0);
        let ElementKind::TabStrip { children, .. } = tabs.kind else {
            panic!("Tabs must render one strip");
        };
        assert_eq!(children[0].style.height, Some(24.0));
        assert_eq!(children[0].style.padding, 4.0);
        assert_eq!(children[0].style.font_size, 11.5);
    }

    #[test]
    fn every_tab_centers_its_label_or_icon_label_group() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("all", "Todo", 120.0).active(true),
                Tab::new("actions", "Acciones", 120.0).icon(Icon::Code),
                Tab::new("disabled", "Archivos", 120.0).disabled(),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(tabs);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 10.0);
        let frame = tree.layout(Size::new(368.0, 28.0));
        let text_x = |name: &str| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == name => Some(position.x),
                    _ => None,
                })
                .unwrap()
        };
        let icon_x = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon {
                    icon: Icon::Code,
                    rect,
                    ..
                } => Some(rect.x),
                _ => None,
            })
            .unwrap();
        let center = |left: f32, right: f32| (left + right) / 2.0;
        assert!((center(text_x("Todo"), text_x("Todo") + 40.0) - 62.0).abs() < 0.1);
        assert!((center(icon_x, text_x("Acciones") + 80.0) - 184.0).abs() < 0.1);
        assert!((center(text_x("Archivos"), text_x("Archivos") + 80.0) - 306.0).abs() < 0.1);
    }

    #[test]
    fn explicit_tab_height_preserves_files_and_project_switchers() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("files", "Archivos", 88.0).active(true),
                Tab::new("projects", "Proyectos", 92.0),
            ],
            palette,
        )
        .height(32.0)
        .render();
        assert_eq!(tabs.style.height, Some(36.0));
        let ElementKind::TabStrip {
            children,
            selection,
            ..
        } = tabs.kind
        else {
            panic!("Tabs must render one strip");
        };
        assert_eq!(children[0].style.height, Some(32.0));
        assert_eq!(children[1].style.height, Some(32.0));
        assert_eq!(selection.unwrap().selected_id, "files");
    }

    #[test]
    fn button_content_and_tooltip_share_one_action_target() {
        let palette = ComponentPalette::neutral_dark();
        let button = Button::with_content(
            "inspect",
            ButtonContent::IconAndLabel(Icon::Code, "Inspeccionar".into()),
            palette,
        )
        .tooltip("Ver detalles")
        .tooltip_shortcut("Ctrl+I")
        .render();
        let ElementKind::Tooltip {
            anchor,
            anchor_id,
            title,
            shortcut,
            ..
        } = &button.kind
        else {
            panic!("button should own its tooltip");
        };
        assert_eq!(anchor_id, "inspect");
        assert_eq!(title, "Ver detalles");
        assert_eq!(shortcut.as_deref(), Some("Ctrl+I"));
        assert!(matches!(anchor.kind, ElementKind::IconButton { .. }));

        let mut tree = UiTree::new(button);
        let frame = tree.layout(Size::new(200.0, 40.0));
        assert_eq!(frame.regions.len(), 1);
        let center = Point::new(
            frame.regions[0].rect.x + frame.regions[0].rect.width / 2.0,
            frame.regions[0].rect.y + frame.regions[0].rect.height / 2.0,
        );
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked {
                id: "inspect".into()
            }]
        );
    }

    #[test]
    fn dark_tooltips_share_one_surface_across_manual_and_button_actions() {
        let palette = ComponentPalette {
            surface: Color::rgb(22, 22, 22),
            raised: Color::rgb(43, 43, 43),
            hover: Color::rgb(52, 52, 52),
            selected: Color::rgb(54, 54, 54),
            border: Color::rgb(46, 46, 46),
            text: Color::rgb(212, 212, 212),
            muted: Color::rgb(136, 136, 136),
            shadow: Color::rgba(0, 0, 0, 125),
        };
        let action_palette = ComponentPalette {
            surface: Color::WHITE,
            raised: Color::WHITE,
            selected: Color::WHITE,
            text: Color::rgb(0, 0, 0),
            ..palette
        };
        let manual = Tooltip::new(
            Element::button("manual", "Abrir"),
            "manual",
            "Abrir",
            palette,
        )
        .render();
        let button = Button::new("button", "Abrir", action_palette)
            .tooltip("Abrir")
            .render();
        let icon_button = IconButton::new("icon", Icon::Add, action_palette)
            .tooltip("Agregar")
            .render();
        let tooltip_style = |element: Element| {
            let ElementKind::Tooltip { colors, .. } = element.kind else {
                panic!("expected the shared tooltip");
            };
            colors
        };
        let expected = tooltip_style(manual);
        assert_eq!(expected, tooltip_style(button));
        assert_eq!(expected, tooltip_style(icon_button));
        assert_eq!(expected.background, Color::rgb(27, 27, 27));
        assert_eq!(expected.border, Color::rgb(64, 64, 64));
        assert_eq!(expected.keycap, crate::shortcut::cap_fill(expected.text));
        assert_eq!(expected.text, Color::rgb(212, 212, 212));
    }

    #[test]
    fn light_tooltip_keeps_palette_colors() {
        let palette = ComponentPalette {
            surface: Color::rgb(250, 250, 250),
            raised: Color::rgb(246, 246, 246),
            hover: Color::rgb(235, 235, 235),
            selected: Color::rgb(225, 225, 225),
            border: Color::rgb(205, 205, 205),
            text: Color::rgb(25, 25, 25),
            muted: Color::rgb(95, 95, 95),
            shadow: Color::rgba(0, 0, 0, 40),
        };
        let tooltip =
            Tooltip::new(Element::button("help", "Ayuda"), "help", "Ayuda", palette).render();
        let ElementKind::Tooltip { colors, .. } = tooltip.kind else {
            panic!("expected the shared tooltip");
        };
        assert_eq!(colors.background, palette.raised);
        assert_eq!(colors.border, palette.border);
        assert_eq!(colors.text, palette.text);
    }

    #[test]
    fn disabled_bright_variants_keep_muted_ink_readable() {
        let palette = ComponentPalette::neutral_dark();
        for variant in [
            ButtonVariant::Default,
            ButtonVariant::Destructive,
            ButtonVariant::Warning,
            ButtonVariant::Success,
        ] {
            let element = Button::new("save", "Guardar", palette)
                .variant(variant)
                .disabled()
                .render();
            assert_eq!(element.style.background, Some(palette.raised));
            let frame = UiTree::new(element).layout(Size::new(180.0, 36.0));
            assert!(frame.regions.is_empty());
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::Text { color, .. } if *color == palette.muted
            )));
        }
    }

    #[test]
    fn label_button_trailing_icon_preserves_width_when_disabled() {
        let palette = ComponentPalette::neutral_dark();
        let enabled = Button::new("more", "Más opciones", palette)
            .trailing_icon(Icon::ChevronDown)
            .variant(ButtonVariant::Raised)
            .render();
        let disabled = Button::new("more", "Más opciones", palette)
            .trailing_icon(Icon::ChevronDown)
            .variant(ButtonVariant::Raised)
            .disabled()
            .render();
        let enabled_frame = UiTree::new(enabled).layout(Size::new(300.0, 32.0));
        let disabled_frame = UiTree::new(disabled).layout(Size::new(300.0, 32.0));
        let enabled_width = enabled_frame.regions[0].rect.width;
        let disabled_width = disabled_frame.commands.iter().find_map(|command| {
            if let DrawCommand::FillRect { rect, color, .. } = command {
                (*color == palette.raised).then_some(rect.width)
            } else {
                None
            }
        });
        assert_eq!(disabled_width, Some(enabled_width));
        assert!(enabled_frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Icon { .. })));
        assert!(disabled_frame.regions.is_empty());
    }

    #[test]
    fn semantic_button_preserves_one_action_for_both_icons() {
        let palette = ComponentPalette::neutral_dark();
        let button = Button::new("backend:codex", "Codex", palette)
            .icon(Icon::Code)
            .trailing_icon(Icon::ArrowDown)
            .variant(ButtonVariant::Raised)
            .width(180.0)
            .render();
        let mut tree = UiTree::new(button);
        let frame = tree.layout(Size::new(180.0, 32.0));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "backend:codex");
        assert_eq!(frame.regions[0].rect.width, 180.0);
        assert_eq!(
            frame.hit_test(Point::new(172.0, 16.0)),
            Some("backend:codex")
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(172.0, 16.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(172.0, 16.0))),
            vec![UiAction::Clicked {
                id: "backend:codex".into()
            }]
        );
    }

    #[test]
    fn disabled_button_keeps_its_box_but_has_no_action() {
        let palette = ComponentPalette::neutral_dark();
        let enabled = IconButton::new("new", Icon::Add, palette)
            .label("Nueva tarea")
            .variant(ButtonVariant::Raised)
            .width(168.0)
            .render();
        let disabled = IconButton::new("new", Icon::Add, palette)
            .label("Nueva tarea")
            .variant(ButtonVariant::Raised)
            .width(168.0)
            .disabled()
            .render();
        let enabled_frame = UiTree::new(enabled).layout(Size::new(168.0, 32.0));
        let disabled_frame = UiTree::new(disabled).layout(Size::new(168.0, 32.0));
        assert_eq!(enabled_frame.regions[0].rect.width, 168.0);
        assert_eq!(enabled_frame.regions[0].rect.height, 32.0);
        assert!(disabled_frame.regions.is_empty());
        assert!(disabled_frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if rect.width == 168.0 && rect.height == 32.0 && *color == palette.raised
        )));
        assert!(disabled_frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon { tint, .. } if *tint == palette.muted
        )));
    }

    #[test]
    fn disabled_intrinsic_width_matches_enabled_width() {
        let palette = ComponentPalette::neutral_dark();
        let enabled = Button::new("save", "Guardar", palette)
            .variant(ButtonVariant::Raised)
            .render();
        let disabled = Button::new("save", "Guardar", palette)
            .variant(ButtonVariant::Raised)
            .disabled()
            .render();
        let enabled_frame = UiTree::new(enabled).layout(Size::new(300.0, 32.0));
        let disabled_frame = UiTree::new(disabled).layout(Size::new(300.0, 32.0));
        let enabled_width = enabled_frame.regions[0].rect.width;
        let painted_width = disabled_frame.commands.iter().find_map(|command| {
            if let DrawCommand::FillRect { rect, color, .. } = command {
                (*color == palette.raised).then_some(rect.width)
            } else {
                None
            }
        });
        assert_eq!(painted_width, Some(enabled_width));
        assert!(disabled_frame.regions.is_empty());
    }

    #[test]
    fn disabled_button_truncates_long_label_inside_fixed_width() {
        let palette = ComponentPalette::neutral_dark();
        let button = Button::new(
            "project",
            "Proyecto con un nombre demasiado largo para el control",
            palette,
        )
        .icon(Icon::Folder)
        .width(140.0)
        .disabled()
        .render();
        let mut tree = UiTree::new(button);
        tree.set_text_measurer(|value, _, _| value.chars().count() as f32 * 7.0);
        let frame = tree.layout(Size::new(140.0, 32.0));
        let label = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, position, .. } => Some((text, position.x)),
            _ => None,
        });
        let (text, x) = label.expect("button label");
        assert!(text.ends_with('…'));
        assert!(x + text.chars().count() as f32 * 7.0 <= 140.0);
    }

    #[test]
    fn icon_button_sizes_keep_the_same_glyph_and_hit_bounds_during_press() {
        let palette = ComponentPalette::neutral_dark();
        for size in [
            ControlSize::Compact,
            ControlSize::Regular,
            ControlSize::Spacious,
        ] {
            let button = IconButton::new("settings", Icon::Settings, palette)
                .variant(ButtonVariant::Subtle)
                .size(size)
                .render();
            let extent = size.height();
            let mut tree = UiTree::new(button);
            tree.set_reduced_motion(true);
            let base = tree.layout(Size::new(extent, extent));
            let hit = base.regions[0].rect;
            assert_eq!(hit.width, extent);
            assert_eq!(hit.height, extent);
            let glyph = base.commands.iter().find_map(|command| match command {
                DrawCommand::Icon { rect, .. } => Some(*rect),
                _ => None,
            });
            assert_eq!(glyph.unwrap().width, ICON_SIZE);
            tree.handle_event(UiEvent::PointerMove(Point::new(4.0, 4.0)));
            let hover = tree.layout(Size::new(extent, extent));
            assert!(hover.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } if *color == palette.hover
            )));
            tree.handle_event(UiEvent::PointerDown(Point::new(4.0, 4.0)));
            let pressed = tree.layout(Size::new(extent, extent));
            assert!(pressed.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } if *color == palette.selected
            )));
            assert_eq!(base.regions, hover.regions);
            assert_eq!(hover.regions, pressed.regions);
        }
    }

    #[test]
    fn active_state_changes_only_paint() {
        let palette = ComponentPalette::neutral_dark();
        let regular = Button::new("session", "Sesión", palette)
            .variant(ButtonVariant::Subtle)
            .width(140.0)
            .render();
        let active = Button::new("session", "Sesión", palette)
            .variant(ButtonVariant::Subtle)
            .width(140.0)
            .active(true)
            .render();
        let regular_frame = UiTree::new(regular).layout(Size::new(140.0, 32.0));
        let active_frame = UiTree::new(active).layout(Size::new(140.0, 32.0));
        assert_eq!(regular_frame.regions, active_frame.regions);
        assert!(regular_frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == palette.surface
        )));
        assert!(active_frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == palette.selected
        )));
    }

    #[test]
    fn tabs_preserve_domain_action_ids() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("files", "Archivos", 100.0).icon(Icon::Folder),
                Tab::new("sessions", "Sesiones", 100.0).active(true),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(tabs);
        let frame = tree.layout(Size::new(240.0, 40.0));
        assert!(frame.regions.iter().any(|region| region.id == "files"));
        assert!(frame.regions.iter().any(|region| region.id == "sessions"));
    }

    #[test]
    fn segmented_control_keeps_the_tab_keyboard_contract_and_fixed_track() {
        let palette = ComponentPalette::neutral_dark();
        let segment = SegmentedControl::new(
            vec![
                Tab::new("group", "Grupo", 70.0).icon(Icon::Hash),
                Tab::new("project", "Proyecto", 78.0)
                    .icon(Icon::Folder)
                    .active(true),
            ],
            palette,
        )
        .render();
        assert_eq!(segment.style.radius, surface_radius());
        assert_eq!(segment.style.width, Some(154.0));
        let mut tree = UiTree::new(segment);
        let frame = tree.layout(Size::new(154.0, 28.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, radius }
                if *color == palette.surface && rect.width == 154.0 && rect.height == 28.0 && *radius == surface_radius()
        )));
        assert!(tree.focus("group"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::ArrowRight,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "project".into()
            }]
        );
    }

    #[test]
    fn active_tab_hover_preserves_selected_treatment() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("group", "Grupo", 80.0),
                Tab::new("project", "Proyecto", 80.0).active(true),
            ],
            palette,
        )
        .render();
        let ElementKind::TabStrip { children, .. } = tabs.kind else {
            panic!("Tabs must render a tab strip");
        };
        assert_eq!(children[0].style.hover_background, None);
        assert_eq!(children[1].style.hover_background, None);
        assert_eq!(children[0].style.hover_foreground, Some(palette.text));
        assert_eq!(
            children[1].style.hover_foreground,
            Some(lighter(palette.text, 12))
        );
    }

    #[test]
    fn search_field_keeps_icon_editor_and_focus_ring_in_one_hit_box() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            SearchField::new("query", "", "Buscar", palette)
                .width(280.0)
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 40.0));
        let region = frame
            .regions
            .iter()
            .find(|region| region.id == "query")
            .unwrap();
        assert_eq!(region.rect.width, 280.0);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon { icon: Icon::Search, rect, .. }
                if rect.x >= region.rect.x && rect.x + rect.width <= region.rect.x + region.rect.width
        )));
        let hit_box = region.rect;
        assert!(tree.focus("query"));
        let focused = tree.layout(Size::new(280.0, 40.0));
        assert_eq!(
            focused
                .regions
                .iter()
                .find(|region| region.id == "query")
                .unwrap()
                .rect,
            hit_box
        );
        assert!(focused.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { rect, .. } if *rect == hit_box
        )));
    }

    #[test]
    fn tooltip_keeps_switch_track_and_thumb_geometry_in_a_tall_row() {
        let palette = ComponentPalette::neutral_dark();
        for checked in [false, true] {
            let toggle = Tooltip::new(
                Switch::new("toggle", checked, palette).render(),
                "toggle",
                "Cambiar habilitación",
                palette,
            )
            .render();
            let mut tree = UiTree::new(
                Element::row(vec![toggle, Element::spacer().flex(1.0)]).size(None, Some(50.0)),
            );
            tree.set_reduced_motion(true);
            let viewport = Size::new(300.0, 160.0);
            let frame = tree.layout(viewport);
            let track = frame
                .regions
                .iter()
                .find(|region| region.id == "toggle")
                .expect("switch target")
                .rect;
            assert_eq!(track.width, 36.0);
            assert_eq!(track.height, 20.0);
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .filter(|region| region.id == "toggle")
                    .count(),
                1
            );
            let thumb = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, radius, .. }
                        if rect.width == 14.0 && rect.height == 14.0 && *radius == 7.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("switch thumb");
            assert_eq!(thumb.x - track.x, if checked { 19.0 } else { 3.0 });
            assert_eq!(thumb.y - track.y, 3.0);
            let point = Point::new(track.x + 8.0, track.y + 8.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "toggle".into()
                }]
            );
            assert!(tree.focus("toggle"));
            let focused = tree.layout(viewport);
            assert!(focused.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "Cambiar habilitación"
            )));
            assert_eq!(
                focused
                    .regions
                    .iter()
                    .find(|region| region.id == "toggle")
                    .unwrap()
                    .rect,
                track
            );
        }
    }

    #[test]
    fn dropdown_keeps_its_compact_anchor_geometry_when_open_in_a_tall_row() {
        let palette = ComponentPalette::neutral_dark();
        let build = |open| {
            let anchor = Button::new("add_origin", "Agregar origen", palette)
                .icon(Icon::Add)
                .size(ControlSize::Compact)
                .render();
            let menu = DropdownMenu::new(
                anchor,
                vec![
                    crate::DropdownMenuItem::new("origin:local", "Carpeta local").into(),
                    crate::DropdownMenuItem::new("origin:github", "Repositorio GitHub").into(),
                ],
                open,
                "dismiss_origin",
                palette,
            )
            .width(200.0)
            .render();
            Element::row(vec![Element::spacer().flex(1.0), menu]).size(None, Some(50.0))
        };
        let viewport = Size::new(420.0, 260.0);
        let mut tree = UiTree::new(build(false));
        tree.set_reduced_motion(true);
        let closed = tree.layout(viewport);
        let anchor = closed
            .regions
            .iter()
            .find(|region| region.id == "add_origin")
            .expect("closed anchor")
            .rect;
        assert_eq!(anchor.height, 28.0);
        assert!(tree.focus("add_origin"));
        tree.set_root(build(true));
        let open = tree.layout(viewport);
        assert_eq!(
            open.regions
                .iter()
                .find(|region| region.id == "add_origin")
                .unwrap()
                .rect,
            anchor
        );
        for id in ["origin:local", "origin:github"] {
            assert!(open.regions.iter().any(|region| region.id == id));
        }
        assert!(tree.focus("origin:local"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        assert_eq!(
            tree.handle_event(key(crate::Key::Enter)),
            vec![UiAction::Clicked {
                id: "origin:local".into()
            }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss_origin".into()
            }]
        );
        tree.set_root(build(false));
        let closed_again = tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("add_origin"));
        assert_eq!(
            closed_again
                .regions
                .iter()
                .find(|region| region.id == "add_origin")
                .unwrap()
                .rect,
            anchor
        );
    }

    #[test]
    fn floating_wrappers_preserve_anchor_constraints_and_flex_without_repeating_paint() {
        let palette = ComponentPalette::neutral_dark();
        let fill = Color::rgb(63, 83, 101);
        for popover in [false, true] {
            let decorate = |anchor: Element| {
                if popover {
                    Element::popover(
                        anchor,
                        Element::button("choice", "Elegir").size(Some(100.0), Some(28.0)),
                        "dismiss",
                        PopoverPlacement::BottomStart,
                    )
                } else {
                    Element::tooltip(
                        anchor,
                        "layout_anchor",
                        "Ayuda",
                        None,
                        TooltipPlacement::Above,
                        palette.tooltip_style(),
                    )
                }
            };
            let anchor = || {
                Element::button("layout_anchor", "Acción")
                    .size(None, Some(28.0))
                    .min_size(140.0, 24.0)
                    .flex(2.0)
                    .padding(4.0)
                    .background(fill)
                    .border(palette.border, 1.0)
                    .radius(6.0)
                    .center_content()
            };
            let wrapper = decorate(anchor());
            assert_eq!(wrapper.style.width, None);
            assert_eq!(wrapper.style.height, Some(28.0));
            assert_eq!(wrapper.style.min_width, 140.0);
            assert_eq!(wrapper.style.min_height, 24.0);
            assert_eq!(wrapper.style.flex, 2.0);
            assert_eq!(wrapper.style.padding, 0.0);
            assert_eq!(wrapper.style.background, None);
            assert_eq!(wrapper.style.border, None);
            assert!(!wrapper.style.center_content);
            let mut tree = UiTree::new(
                Element::row(vec![Element::spacer().flex(1.0), wrapper]).size(None, Some(50.0)),
            );
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(300.0, 200.0));
            let hit = frame
                .regions
                .iter()
                .find(|region| region.id == "layout_anchor")
                .expect("original anchor target");
            assert_eq!(hit.rect.width, 200.0);
            assert_eq!(hit.rect.height, 28.0);
            assert_eq!(hit.rect.x, 100.0);
            assert_eq!(
                frame
                    .commands
                    .iter()
                    .filter(|command| matches!(command,
                        DrawCommand::FillRect { color, .. } if *color == fill
                    ))
                    .count(),
                1
            );

            let overridden = decorate(anchor()).size(Some(180.0), Some(34.0));
            assert_eq!(overridden.style.width, Some(180.0));
            assert_eq!(overridden.style.height, Some(34.0));
        }
    }

    #[test]
    fn switch_is_one_keyboard_and_pointer_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Switch::new("keep_awake", false, palette).render());
        let frame = tree.layout(Size::new(36.0, 20.0));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "keep_awake");
        assert_eq!(frame.regions[0].rect.width, 36.0);
        assert_eq!(frame.regions[0].rect.height, 20.0);
        assert!(tree.focus("keep_awake"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Enter,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "keep_awake".into()
            }]
        );
        tree.set_root(
            Switch::new("keep_awake", false, palette)
                .disabled()
                .render(),
        );
        let disabled = tree.layout(Size::new(36.0, 20.0));
        assert!(disabled.regions.is_empty());
    }

    #[test]
    fn tab_arrows_activate_visible_enabled_tabs_and_wrap_within_their_strip() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("sessions", "Sesiones", 100.0),
                Tab::new("unavailable", "Pronto", 100.0).disabled(),
                Tab::new("files", "Archivos", 100.0),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(Element::column(vec![
            tabs,
            Element::button("outside", "Otro"),
        ]));
        tree.layout(Size::new(340.0, 100.0));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        assert!(tree.focus("sessions"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: crate::Key::ArrowRight,
                shift: true,
                ctrl: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("sessions"));
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert_eq!(tree.focused_id(), Some("files"));
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked {
                id: "sessions".into()
            }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowLeft)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::Home)),
            vec![UiAction::Clicked {
                id: "sessions".into()
            }]
        );
        assert!(tree.handle_event(key(crate::Key::Home)).is_empty());
        assert_eq!(
            tree.handle_event(key(crate::Key::End)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert!(!tree.focus("unavailable"));
        assert!(tree.focus("outside"));
        assert!(tree.handle_event(key(crate::Key::ArrowLeft)).is_empty());
        assert_eq!(tree.focused_id(), Some("outside"));
    }

    #[test]
    fn dropdown_uses_shared_popover_and_keeps_item_ids() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("backend:codex", "Codex").selected(true),
                DropdownOption::new("backend:claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        let frame = tree.layout(Size::new(400.0, 300.0));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:codex"));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:claude"));
        assert_eq!(frame.hit_test(Point::new(390.0, 290.0)), Some("dismiss"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Escape,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn dropdown_list_sizing_preserves_compact_rows_and_keyboard_scroll() {
        let sizing = crate::SelectorSizing {
            max_visible_items: 2,
            ..crate::SelectorSizing::COMPACT
        };
        let dropdown = DropdownList::new(
            Element::button("trigger", "Choice").size(Some(160.0), Some(36.0)),
            (0..5)
                .map(|index| {
                    DropdownOption::new(format!("choice:{index}"), format!("Choice {index}"))
                        .icon(Icon::Folder)
                        .selected(index == 0)
                })
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .sizing(sizing)
        .width(220.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(360.0, 280.0));
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap();
        assert_eq!(surface.rect.width, 220.0);
        let first = frame
            .regions
            .iter()
            .find(|region| region.id == "choice:0")
            .unwrap();
        assert_eq!(first.rect.height, sizing.row_height);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, .. }
                if text == "Choice 0" && *font_size == sizing.label_size
        )));
        assert!(tree.focus("choice:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::End,
            shift: false,
            ctrl: false,
        });
        let final_frame = tree.layout(Size::new(360.0, 280.0));
        assert_eq!(tree.focused_id(), Some("choice:4"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(final_frame
            .regions
            .iter()
            .any(|region| region.id == "choice:4"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Enter,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "choice:4".into()
            }]
        );
    }

    #[test]
    fn fill_width_shares_the_total_between_the_tabs_and_ignores_a_bad_total() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |total: f32| {
            Tabs::new(
                vec![Tab::new("a", "A", 50.0), Tab::new("b", "B", 90.0), Tab::new("c", "C", 70.0)],
                palette,
            )
            .fill_width(total)
        };
        for total in [240.0, 301.0, 520.0] {
            let mut tree = UiTree::new(Element::column(vec![
                Element::row(vec![tabs(total).render(), Element::spacer().flex(1.0)]),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(800.0, 100.0));
            let rect = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
            assert_eq!(rect("a").width, rect("b").width);
            assert_eq!(rect("b").width, rect("c").width);
            assert!((rect("a").x - TAB_TRACK_PADDING).abs() < 0.01);
            let end = rect("c").x + rect("c").width + TAB_TRACK_PADDING;
            assert!((end - total).abs() < 0.01, "{total}: track ends at {end}");
        }
        for bad in [f32::NAN, f32::INFINITY] {
            let widths: Vec<_> = tabs(bad).items.iter().map(|item| item.width).collect();
            assert_eq!(widths, [50.0, 90.0, 70.0]);
        }
        assert!(tabs(-10.0).items.iter().all(|item| item.width == 40.0));
        assert!(Tabs::new(Vec::new(), palette).fill_width(200.0).items.is_empty());
    }

    #[test]
    fn monospace_text_field_changes_only_the_family_enabled_and_disabled() {
        let palette = ComponentPalette::neutral_dark();
        for disabled in [false, true] {
            let field = |mono| {
                let field = TextField::new("path", "src/main.rs", "", palette)
                    .monospace(mono)
                    .enabled(!disabled);
                UiTree::new(field.render()).layout(Size::new(300.0, 32.0))
            };
            let family = |frame: &crate::Frame| {
                frame.commands.iter().find_map(|command| match command {
                    DrawCommand::Text { font_family, .. } => Some(*font_family),
                    _ => None,
                })
            };
            assert_eq!(family(&field(false)), Some(FontFamily::Sans));
            assert_eq!(family(&field(true)), Some(FontFamily::Monospace));
            assert_eq!(field(false).regions, field(true).regions);
        }
    }

    #[test]
    fn a_text_field_takes_a_height_of_the_host_and_never_less_than_its_size() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let style = |height: f32| {
                TextField::new("path", "", "", palette)
                    .size(ControlSize::Compact)
                    .enabled(enabled)
                    .height(height)
                    .render()
                    .style
            };
            let plain = TextField::new("path", "", "", palette)
                .size(ControlSize::Compact)
                .enabled(enabled)
                .render()
                .style;
            assert_eq!(style(40.0).height, Some(40.0));
            assert_eq!(style(40.0).min_height, plain.min_height);
            for bad in [f32::NAN, f32::INFINITY, -8.0, 4.0] {
                assert_eq!(style(bad).height, plain.height, "{bad}");
            }
        }
    }

    #[test]
    fn selected_dropdown_option_keeps_selected_hover_color() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agent"),
            vec![
                DropdownOption::new("codex", "Codex").selected(true),
                DropdownOption::new("claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let ElementKind::Popover { floating, .. } = dropdown.kind else {
            panic!("open dropdown must have a popover");
        };
        let ElementKind::Panel(content) = floating.kind else {
            panic!("popover must contain a panel");
        };
        let ElementKind::Column(options) = content.kind else {
            panic!("short dropdown must contain an option column");
        };
        assert_eq!(
            options[0].style.hover_background,
            Some(lighter(palette.selected, 8))
        );
        assert_eq!(options[1].style.hover_background, Some(palette.hover));
    }

    #[test]
    fn dropdown_rows_keep_one_hit_target_and_stable_icon_columns() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("codex", "Codex")
                    .icon(Icon::Chat)
                    .selected(true),
                DropdownOption::new("native", "Nativo"),
                DropdownOption::new("offline", "No instalado").disabled(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 300.0));
        let rows: Vec<_> = frame
            .regions
            .iter()
            .filter(|region| region.id == "codex" || region.id == "native")
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rect.width, rows[1].rect.width);
        assert_eq!(rows[0].rect.height, 36.0);
        assert_eq!(rows[1].rect.height, 36.0);
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
        assert_eq!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    DrawCommand::Icon {
                        icon: Icon::Check,
                        ..
                    }
                ))
                .count(),
            1
        );
        tree.handle_event(UiEvent::PointerMove(Point::new(
            rows[1].rect.x + 4.0,
            rows[1].rect.y + 4.0,
        )));
        let hovered = tree.layout(Size::new(400.0, 300.0));
        assert!(hovered.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == rows[1].rect && *color == palette.hover
        )));
    }

    #[test]
    fn dropdown_keyboard_stays_in_menu_skips_disabled_and_returns_to_trigger() {
        let palette = ComponentPalette::neutral_dark();
        let make_dropdown = || {
            DropdownList::new(
                Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
                vec![
                    DropdownOption::new("codex", "Codex"),
                    DropdownOption::new("offline", "No instalado").disabled(),
                    DropdownOption::new("claude", "Claude"),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(170.0)
            .render()
        };
        let key = |key, shift| UiEvent::Key {
            key,
            shift,
            ctrl: false,
        };
        let mut tree = UiTree::new(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("trigger"));

        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::End, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Home, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::Tab, true));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Enter, false)),
            vec![UiAction::Clicked { id: "codex".into() }]
        );

        tree.set_root(Element::button("trigger", "Agente"));
        tree.layout(Size::new(400.0, 300.0));
        assert_eq!(tree.focused_id(), Some("trigger"));

        tree.set_root(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Escape, false)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn long_dropdown_reveals_options_reached_by_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let options = (0..12)
            .map(|index| {
                let option =
                    DropdownOption::new(format!("option:{index}"), format!("Opción {index}"));
                if index == 5 {
                    option.disabled()
                } else {
                    option
                }
            })
            .collect();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            options,
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        let viewport = Size::new(400.0, 400.0);
        tree.layout(viewport);
        assert!(tree.focus("trigger"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        for _ in 0..9 {
            tree.handle_event(key(crate::Key::ArrowDown));
        }
        assert_eq!(tree.focused_id(), Some("option:9"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        let scrolled = tree.layout(viewport);
        assert!(scrolled
            .regions
            .iter()
            .any(|region| region.id == "option:9"));
        tree.handle_event(key(crate::Key::End));
        assert_eq!(tree.focused_id(), Some("option:11"));
        let end = tree.layout(viewport);
        assert!(end.regions.iter().any(|region| region.id == "option:11"));
        tree.handle_event(key(crate::Key::Home));
        assert_eq!(tree.focused_id(), Some("option:0"));
        assert_eq!(tree.scroll_offset("dismiss:options"), 0.0);
        let home = tree.layout(viewport);
        assert!(home.regions.iter().any(|region| region.id == "option:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("option:11"));
        let wrapped = tree.layout(viewport);
        assert!(wrapped
            .regions
            .iter()
            .any(|region| region.id == "option:11"));
    }

    #[test]
    fn clicking_outside_dropdown_returns_focus_before_dismiss_action() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![DropdownOption::new("codex", "Codex")],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("codex"));
        let outside = Point::new(390.0, 290.0);
        tree.handle_event(UiEvent::PointerDown(outside));
        assert_eq!(tree.focused_id(), Some("trigger"));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(outside)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn disabled_options_are_visible_without_click_regions() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![Tab::new("soon", "Próximamente", 140.0).disabled()],
            palette,
        )
        .render();
        let frame = UiTree::new(tabs).layout(Size::new(180.0, 40.0));
        assert!(!frame.regions.iter().any(|region| region.id == "soon"));
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente"),
            vec![DropdownOption::new("offline", "No instalado").disabled()],
            true,
            "dismiss",
            palette,
        )
        .render();
        let frame = UiTree::new(dropdown).layout(Size::new(300.0, 200.0));
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
    }
}
