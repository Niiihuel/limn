//! Composable, controlled controls built from the same backend-neutral tree.
//! State such as the selected tab or whether a menu is open belongs to the
//! caller; stable action IDs let [`crate::UiTree`] retain focus and hover while
//! these controls are rebuilt on every frame.

use crate::{
    Access, Color, Element, ElementKind, FontFamily, Icon, Point, PopoverPlacement, RasterImage, TooltipItem,
    RadiusRole, TooltipOptions, TooltipPlacement, TooltipSection, TooltipStyle, ICON_SIZE,
};
use crate::{DropdownMenu, DropdownMenuEntry, DropdownMenuItem, MenuMotion, Role};

/// A component can render to an ordinary [`Element`] and be used anywhere in
/// the tree. App-specific views and generic controls follow the same contract.
pub trait Component {
    /// Consume the component and build the element that draws it.
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
    described: bool,
    options: TooltipOptions,
    background: Option<Color>,
}

impl Tooltip {
    /// A hint on `anchor` with this title. `anchor_id` is the ID of the control
    /// the anchor is, or the one a passive anchor's hover region takes. It
    /// opens above the anchor and is at most 360 pixels wide.
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
            described: true,
            options: TooltipOptions::default(),
            background: None,
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

    /// Show a keyboard shortcut in the hint, as key caps.
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

    /// Add a row of people's faces after an optional icon, as who is in a
    /// call or a room: round pictures that overlap as a group, then «+N» for
    /// `more` people not passed. Faces that do not fit the tooltip's width
    /// are counted in «+N». Passive, as the rest of the hint; without faces
    /// nothing is added.
    pub fn faces(
        mut self,
        icon: Option<Icon>,
        faces: impl IntoIterator<Item = crate::TooltipFace>,
        more: usize,
    ) -> Self {
        let faces: Vec<_> = faces.into_iter().collect();
        if !faces.is_empty() {
            self.sections.push(TooltipSection::Faces { icon, faces, more });
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

    /// The side of the anchor it prefers. The default is above.
    pub fn placement(mut self, placement: TooltipPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// How long the pointer rests on the anchor before the hint shows, in
    /// milliseconds: 350 unless set, and 0 shows it as soon as the pointer
    /// is over it, for a strip of marks read by moving along it. Keyboard
    /// focus shows it at once either way.
    pub fn delay_ms(mut self, delay: u16) -> Self {
        self.options.delay_ms = delay;
        self
    }

    /// The title's text size in logical pixels, 12 unless set. A one-line
    /// hint grows by what a larger title adds. A size that is not finite or
    /// not above zero is ignored.
    pub fn title_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.options.title_font_size = size;
        }
        self
    }

    /// The title's face, the regular one unless set.
    pub fn title_font_family(mut self, family: FontFamily) -> Self {
        self.options.title_font_family = family;
        self
    }

    /// Whether a small pointer joins the hint to its anchor. On unless set.
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.options.arrow = arrow;
        self
    }

    /// Fill the hint and its pointer with this color instead of the
    /// palette's, such as a darker one than the menus have.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }
}

impl Component for Tooltip {
    fn render(self) -> Element {
        // A hint is not seen by who cannot hover: its title describes the
        // control it is anchored to.
        let anchor = if self.described && !self.title.trim().is_empty() {
            self.anchor.accessible(Access::hinted(self.title.clone()))
        } else {
            self.anchor
        };
        let mut colors = self.palette.tooltip_style();
        if let Some(background) = self.background {
            colors.background = background;
        }
        let mut element = Element::tooltip(
            anchor,
            self.anchor_id,
            self.title,
            self.shortcut,
            self.placement,
            colors,
        );
        if let ElementKind::Tooltip {
            sections,
            max_width,
            options,
            ..
        } = &mut element.kind
        {
            *sections = self.sections;
            *max_width = self.max_width;
            *options = self.options;
        }
        element
    }
}

impl Tooltip {
    // The hint alone, for a button that has already said what its hint says.
    fn hint(self) -> Element {
        Component::render(Tooltip { described: false, ..self })
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
        tooltip.hint()
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
    /// 24 pixels high.
    Dense,
    /// 28 pixels high.
    Compact,
    /// 32 pixels high.
    Regular,
    /// 36 pixels high.
    Spacious,
}

impl ControlSize {
    /// The height in logical pixels: 24, 28, 32 or 36.
    pub const fn height(self) -> f32 {
        match self {
            Self::Dense => 24.0,
            Self::Compact => 28.0,
            Self::Regular => 32.0,
            Self::Spacious => 36.0,
        }
    }
}

/// Whether a control can be used.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ControlState {
    #[default]
    /// It answers the pointer and the keyboard.
    Enabled,
    /// It keeps its place and cannot be used.
    Disabled,
}

/// Semantic colors shared by tabs, dropdowns, and floating surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentPalette {
    /// The fill a component sits on, such as the track of a tab strip.
    pub surface: Color,
    /// The fill of what stands above the surface, such as a popover.
    pub raised: Color,
    /// The fill under the pointer. `interactive_surface` sets it as the hover
    /// background.
    pub hover: Color,
    /// The fill of the chosen item, such as the active tab, and of a pressed
    /// control.
    pub selected: Color,
    /// Hairlines around and between surfaces. Its lightness also tells a dark
    /// palette from a light one, for `link` and the tooltips.
    pub border: Color,
    /// Primary ink.
    pub text: Color,
    /// Ink for secondary labels and for disabled content.
    pub muted: Color,
    /// The shadow under floating surfaces: popovers, modals, toasts, tooltips.
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

    /// A dark palette of neutral grays.
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

    pub(crate) fn is_dark(self) -> bool {
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
    /// 36 pixels high, with a 14 pixel label and 16 pixels of padding at each side.
    Default,
    /// 32 pixels high, with a 13 pixel label and 12 pixels of padding at each side.
    Small,
    /// 40 pixels high, with a 14 pixel label and 24 pixels of padding at each side.
    Large,
    /// A 36 pixel square around an icon alone.
    Icon,
    /// A 32 pixel square around an icon alone.
    IconSmall,
    /// A 24 pixel square around an icon alone, with a 12 pixel label if it has one.
    IconExtraSmall,
    /// A 22 px action inside a dense row, such as "more" in a file tree.
    Mini,
}

impl ButtonSize {
    /// The geometry of this size: 22 to 40 pixels high, with 8 pixels between
    /// the icons and the label.
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
    /// Text alone.
    Label(String),
    /// An icon alone. The button has no label and, unless a width is set, is
    /// as wide as it is high.
    Icon(Icon),
    /// An icon before a label.
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
    /// The geometry of a button of this height: a label of 12 to 14 pixels,
    /// 6 pixels between the icons and the label, and padding that centers the
    /// icon in a square button.
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
    image: Option<RasterImage>,
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
    accessibility_label: Option<String>,
}

impl Button {
    /// A button with this label that reports `action_id`. It starts as the
    /// default variant at `ControlSize::Regular`.
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self::with_content(action_id, ButtonContent::Label(label.into()), palette)
    }

    /// A button with a label, an icon or both, as [`Self::new`].
    pub fn with_content(
        action_id: impl Into<String>,
        content: ButtonContent,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            content,
            image: None,
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
            accessibility_label: None,
        }
    }

    /// What the button is called to the platform, in place of its label. A
    /// button of an icon alone is called by its hint's title without one, and
    /// one of a picture alone by the picture's text alternative.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    // What the platform is told: the name of an icon alone, and the hint's
    // title as a description where it is not already the name.
    fn access(&self) -> Access {
        let (_, label) = self.content.clone().parts();
        let title = self.tooltip.title.clone().filter(|title| !title.trim().is_empty());
        let name = self
            .accessibility_label
            .clone()
            .or_else(|| if label.is_empty() { title.clone() } else { None })
            .or_else(|| self.image.as_ref().filter(|_| label.is_empty()).map(|image| image.alt().to_owned()));
        let spoken = name.clone().unwrap_or(label);
        let mut access = Access::default();
        if let Some(name) = name {
            access = access.name(name);
        }
        match title {
            Some(title) if title != spoken => access.description(title),
            _ => access,
        }
    }

    /// Put an icon before the label. With an empty label the button shows
    /// the icon alone and is as wide as it is high.
    pub fn icon(mut self, icon: Icon) -> Self {
        let (_, label) = self.content.parts();
        self.content = if label.is_empty() {
            ButtonContent::Icon(icon)
        } else {
            ButtonContent::IconAndLabel(icon, label)
        };
        self
    }

    /// Put a picture before the label, such as a custom emoji, in place of
    /// any icon: it takes the icon's slot, [`ICON_SIZE`] on a side, fitted at
    /// its own proportions and in its own colors, so ink, hover and a disabled
    /// state do not tint it. With an empty label the button shows the picture
    /// alone and is as wide as it is high. `loading` still shows the spinner.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    /// Give the button a hint with this title. A button of an icon alone is
    /// also called by it to the platform.
    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.tooltip.title = Some(title.into());
        self
    }

    /// Set the keyboard shortcut the hint shows.
    pub fn tooltip_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.tooltip.shortcut = Some(shortcut.into());
        self
    }

    /// The side of the button the hint prefers.
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

    /// Add a second icon after the label. It shares the button's action and
    /// hit region.
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    /// Choose the fill, the ink and the hover treatment; see
    /// [`ButtonVariant`].
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Size the button from a shared control height. It drops a `button_size`
    /// or `sizing` set before.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self.button_size = None;
        self.sizing = None;
        self
    }

    /// Size the button from the button family. It takes precedence over
    /// `size` and drops a `sizing` set before.
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

    /// Fix the width in logical pixels, in place of the content's. A negative
    /// width becomes zero; one that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Draw the button as the current choice. Ghost, Subtle, Raised and
    /// Outline take the palette's selected fill; the other variants rest on
    /// their hover fill.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Choose how an active Ghost, Subtle, Raised or Outline button hovers.
    pub fn active_hover(mut self, active_hover: ActiveHover) -> Self {
        self.active_hover = active_hover;
        self
    }

    /// Draw the button muted, as passive content: it keeps its size and its
    /// hint, and cannot be focused or activated.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it from a state the host computes; `enabled(false)`
    /// is [`Self::disabled`].
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

    /// Ink under the pointer, and while pressed unless `pressed_ink` is set.
    pub fn hover_ink(mut self, color: Color) -> Self {
        self.hover_ink = Some(color);
        self
    }

    /// Ink while pressed.
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
        let access = self.access();
        let sizing = self
            .sizing
            .or(self.button_size.map(ButtonSize::sizing))
            .unwrap_or_else(|| self.size.button_sizing());
        let padding = sizing.horizontal_padding;
        let spacing = self.gap.unwrap_or(sizing.inner_spacing);
        let (icon, label) = self.content.parts();
        let image = self.image;
        let icon_only = (icon.is_some() || image.is_some() || self.loading) && label.is_empty();
        let width = self.width.or(icon_only.then_some(sizing.height));
        // A square icon button keeps room for its glyph at every size.
        let padding = if icon_only {
            padding.min(((width.unwrap_or(sizing.height) - sizing.icon_size) / 2.0).max(0.0))
        } else {
            padding
        };
        let height = sizing.height;
        // A shape, not a token: half the height of whatever size it is laid out at.
        let (round, radius) = (self.round, self.radius.px());
        let shape = move |element: Element| if round { element.round() } else { element.radius(radius) };

        if self.state == ControlState::Disabled || self.loading {
            // The engine's Button element always registers a hit target. A
            // disabled action is instead ordinary painted content: it remains
            // visible, but cannot be focused or activated by any input path.
            let has_label = !label.is_empty();
            // Passive content has no name of its own: the label is said.
            let access = Access::new(Role::Button).id(self.action_id.clone()).disabled(true).over(access);
            let access = if has_label { access.over(Access::named(label.clone())) } else { access };
            let icon_space = if icon.is_some() || image.is_some() || self.loading {
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
                        .decorative()
                        .render(),
                ))
            } else if let Some(image) = image {
                Some(center_icon(
                    Element::raster_image(image).size(Some(ICON_SIZE), Some(ICON_SIZE)).centered_x(),
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
                // In the middle of a button wider than its label, where an enabled one has it.
                label.center_content()
            };
            // Button padding is horizontal. Center the content vertically
            // without clipping large button sizes or the spinner's slot.
            let vertical_padding = ((height - content_height) / 2.0).max(0.0);
            let extra_horizontal = (padding - vertical_padding).max(0.0);
            let mut element = shape(Element::panel(Element::row(vec![
                Element::spacer().size(Some(extra_horizontal), None),
                visual.size(None, Some(content_height)).flex(1.0),
                Element::spacer().size(Some(extra_horizontal), None),
            ]))
            .padding(vertical_padding)
            .background(base))
            .size(width, Some(height));
            if outline {
                element = element.border(self.palette.border, 1.0);
            }
            return self.tooltip.wrap(element.accessible(access), self.action_id, self.palette);
        }

        let mut element = if let Some(image) = image {
            // The icon is never drawn: the picture takes its slot.
            Element::icon_button(self.action_id.clone(), icon.unwrap_or(Icon::Add), label).leading_image(image)
        } else if let Some(icon) = icon {
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
            .gap(spacing);
        element = shape(element).size(width, Some(height));
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
        if access != Access::default() {
            element = element.accessible(access);
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
    muted: Color,
    font_size: f32,
    enabled: bool,
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
            muted: palette.muted,
            font_size: 13.0,
            enabled: true,
        }
    }

    /// Draw the link in this ink, in place of the palette's link ink.
    pub fn ink(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The former name of [`ink`](Self::ink).
    #[deprecated(since = "0.1.39", note = "renamed to `ink`, the name every other component uses")]
    pub fn color(self, color: Color) -> Self {
        self.ink(color)
    }

    /// Set the size of the text. A size that is not finite or not above
    /// zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size;
        }
        self
    }

    /// A disabled link is passive text: its label in the palette's muted, in
    /// place of any `color`, with no underline, hit region or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Link {
    fn render(self) -> Element {
        if !self.enabled {
            // The engine's Button element always registers a hit target, so
            // a disabled link is its label alone, on one line as a button's.
            let access = Access::new(Role::Link).id(self.action_id).name(self.label.clone()).disabled(true);
            return Element::truncated_text(self.label)
                .foreground(self.muted)
                .font_family(FontFamily::Sans)
                .font_size(self.font_size)
                .accessible(access);
        }
        Element::button(self.action_id, self.label)
            .foreground(self.color)
            .font_family(FontFamily::Sans)
            .font_size(self.font_size)
            .padding(0.0)
            .underline_on_hover()
            .accessible(Access::new(Role::Link))
    }
}

/// A button whose leading icon is mandatory. Without a label it uses a square
/// hit target, while the glyph remains 16 logical pixels at every density.
pub struct IconButton(Button);

impl IconButton {
    /// A button of this icon alone that reports `action_id`.
    pub fn new(action_id: impl Into<String>, icon: Icon, palette: ComponentPalette) -> Self {
        Self(Button::with_content(
            action_id,
            ButtonContent::Icon(icon),
            palette,
        ))
    }

    /// Add a label after the icon. The button is then as wide as its content
    /// in place of square.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let (icon, _) = self.0.content.parts();
        self.0.content = ButtonContent::IconAndLabel(icon.expect("IconButton icon"), label.into());
        self
    }

    /// Give the button a hint with this title, which also names an icon
    /// alone to the platform; as [`Button::tooltip`].
    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.0 = self.0.tooltip(title);
        self
    }

    /// Set the keyboard shortcut the hint shows.
    pub fn tooltip_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.0 = self.0.tooltip_shortcut(shortcut);
        self
    }

    /// The side of the button the hint prefers.
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

    /// What the button is called to the platform. An icon alone has no name:
    /// without this it is called by its hint's title, and by nothing without
    /// a hint.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.0 = self.0.accessibility_label(name);
        self
    }

    /// Add a second icon at the far edge, as [`Button::trailing_icon`].
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.0.trailing_icon = Some(icon);
        self
    }

    /// Choose the fill, the ink and the hover treatment; see
    /// [`ButtonVariant`].
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.0.variant = variant;
        self
    }

    /// Size the button from a shared control height, as [`Button::size`].
    pub fn size(mut self, size: ControlSize) -> Self {
        self.0 = self.0.size(size);
        self
    }

    /// Size the button from the button family, as [`Button::button_size`].
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

    /// Fix the width in logical pixels, as [`Button::width`].
    pub fn width(mut self, width: f32) -> Self {
        self.0 = self.0.width(width);
        self
    }

    /// Draw the button as the current choice, as [`Button::active`].
    pub fn active(mut self, active: bool) -> Self {
        self.0.active = active;
        self
    }

    /// Choose how an active button without a persistent fill hovers.
    pub fn active_hover(mut self, active_hover: ActiveHover) -> Self {
        self.0.active_hover = active_hover;
        self
    }

    /// Draw the button muted and passive, as [`Button::disabled`].
    pub fn disabled(mut self) -> Self {
        self.0.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it from a state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.0 = self.0.enabled(enabled);
        self
    }

    /// Show the standard spinner in place of the icon; a busy button cannot
    /// be focused or activated. As [`Button::loading`].
    pub fn loading(mut self, loading: bool) -> Self {
        self.0 = self.0.loading(loading);
        self
    }

    /// Draw the icon and label in this color at rest; see [`Button::ink`].
    pub fn ink(mut self, color: Color) -> Self {
        self.0.ink = Some(color);
        self
    }

    /// Ink under the pointer, and while pressed unless `pressed_ink` is set.
    pub fn hover_ink(mut self, color: Color) -> Self {
        self.0.hover_ink = Some(color);
        self
    }

    /// Ink while pressed.
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
    /// Key caps for the combination `keys`.
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
                    parts.push(Element::text("/").font_size(small).foreground(palette.muted).centered_y());
                }
                parts.push(
                    Kbd { keys, menu: self.menu, ..Kbd::new("", palette) }.render().centered_y(),
                );
            }
            if let Some(caption) = caption {
                parts.push(
                    // On the keycaps' middle line, not hung from their top.
                    Element::text(caption)
                        .font_family(FontFamily::Sans)
                        .font_size(small)
                        .foreground(palette.muted)
                        .centered_y(),
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
    icon: Option<Icon>,
    accessibility_label: Option<String>,
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
            icon: None,
            accessibility_label: None,
        }
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Take the height and text size of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Put an icon before the text, inside the field, as a search field has
    /// its magnifier: a `#` before a channel's name, an at sign before an
    /// address. It is decorative, in the muted ink until the field has the
    /// focus, and the text starts after it.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
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
            let access = disabled_field(
                if self.password { Role::PasswordInput } else { Role::TextInput },
                self.id,
                self.accessibility_label,
                &self.placeholder,
                // What is typed into a password field is nobody's to read out.
                (!self.password).then(|| self.value.clone()),
            );
            let text = if self.password && !self.value.is_empty() {
                "•".repeat(self.value.chars().count())
            } else if self.value.is_empty() {
                self.placeholder
            } else {
                self.value
            };
            let text = Element::truncated_text(text)
                .foreground(self.palette.muted)
                .font_family(family)
                .font_size(13.0);
            // Where the enabled field draws them: the icon in the padding's
            // corner and the text 8 after it, both on the middle line.
            let content = match self.icon {
                Some(icon) => Element::row(vec![
                    Element::icon(icon)
                        .foreground(self.palette.muted)
                        .size(Some(ICON_SIZE), Some(ICON_SIZE))
                        .centered_y(),
                    text.flex(1.0),
                ])
                .gap(8.0),
                None => text,
            };
            return Element::panel(content)
                .padding(8.0)
                .background(self.palette.raised)
                .border(self.palette.border, 1.0)
                .radius(input_radius(self.size))
                .size(self.width, Some(height))
                .accessible(access);
        }
        let mut input = Element::text_input(self.id, self.value, self.placeholder)
            .password(self.password)
            .persistent_placeholder(self.persistent_placeholder);
        if let Some(icon) = self.icon {
            input = input
                .leading_icon(icon)
                .leading_icon_tint(self.palette.muted);
        }
        let input = input_surface(input, self.palette, self.size, self.width)
            .size(self.width, Some(height))
            .font_family(family);
        match self.accessibility_label {
            Some(name) => input.accessibility_label(name),
            None => input,
        }
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
    enter_submits: bool,
    accessibility_label: Option<String>,
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
            enter_submits: false,
            accessibility_label: None,
        }
    }

    /// Make Enter report `Submitted` and Shift+Enter break the line, as a chat
    /// composer does; Ctrl+Enter submits either way. Off by default: Enter
    /// breaks the line.
    pub fn enter_submits(mut self, submits: bool) -> Self {
        self.enter_submits = submits;
        self
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
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
            let access = disabled_field(
                Role::MultilineTextInput,
                self.id,
                self.accessibility_label,
                &self.placeholder,
                Some(self.value.clone()),
            );
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
            .size(self.width, Some(self.height))
            .accessible(access);
        }
        let area = Element::text_area(self.id, self.value, self.placeholder)
            .text_area_preserve_line_endings(self.preserve_line_endings)
            .text_area_enter_submits(self.enter_submits)
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
            .size(self.width, Some(self.height));
        match self.accessibility_label {
            Some(name) => area.accessibility_label(name),
            None => area,
        }
    }
}

// What a field drawn disabled still says: the node its input would have been.
fn disabled_field(
    role: Role,
    id: String,
    name: Option<String>,
    placeholder: &str,
    value: Option<String>,
) -> Access {
    let mut access = Access::new(role).id(id).disabled(true);
    if let Some(name) = name.or_else(|| (!placeholder.is_empty()).then(|| placeholder.to_owned())) {
        access = access.name(name);
    }
    match value {
        Some(value) => access.value_text(value),
        None => access,
    }
}

fn input_radius(_size: ControlSize) -> f32 {
    control_radius()
}

// The fill, border, radius, ink and height every text field shares; a
// select drawn as a field takes it too.
pub(crate) fn input_surface(
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
    clearable: bool,
    accessibility_label: Option<String>,
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
            clearable: false,
            accessibility_label: None,
        }
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
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

    /// Show a button at the end of the field while the query is not empty,
    /// which reports `Clicked` with `"{id}:clear"`. The field does not empty
    /// itself: the host clears its query and gives the field its new text
    /// with `UiTree::set_input_value`, and may give it the focus back.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
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
        let clear =
            (self.clearable && !self.value.is_empty()).then(|| format!("{}:clear", self.id));
        let height = self.size.height().max(28.0);
        let radius = input_radius(self.size);
        let surface = self.palette.raised;
        if self.state == ControlState::Disabled {
            let access = disabled_field(
                Role::SearchInput,
                self.id,
                self.accessibility_label,
                &self.placeholder,
                Some(self.value.clone()),
            );
            // What the enabled field shows away from the focus: the hint only
            // while there is no query, so it never meets the clear button.
            let shortcut = self.shortcut.filter(|_| self.value.is_empty());
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
            if let Some(shortcut) = shortcut {
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
            let mut row = vec![
                Element::spacer().size(Some(8.0), None),
                Element::row(children).gap(8.0).flex(1.0),
            ];
            match clear {
                // Where the enabled one is, passive and still known by its ID.
                Some(id) => row.extend([
                    clear_button(id, self.palette, false).centered_y(),
                    Element::spacer().size(Some(CLEAR_INSET), None),
                ]),
                None => row.push(Element::spacer().size(Some(8.0), None)),
            }
            return Element::panel(Element::row(row))
                .background(surface)
                .border(self.palette.border, 1.0)
                .radius(radius)
                .size(self.width, Some(height))
                .accessible(access);
        }
        let mut access = Access::new(Role::SearchInput);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
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
        let input = input.accessible(access);
        let Some(clear) = clear else { return input };
        // The button lies over the field's end, which the text leaves free as
        // it leaves the magnifier's room at the start; the field stays lit
        // while the pointer is on the button.
        Element::stack(vec![
            input
                .input_trailing_space(ICON_SIZE + 8.0)
                .hover_with(clear.clone()),
            Element::row(vec![
                Element::spacer().flex(1.0),
                clear_button(clear, self.palette, true).centered_y(),
                Element::spacer().size(Some(CLEAR_INSET), None),
            ]),
        ])
        // As wide as the field alone, so typing does not narrow it.
        .min_size(80.0, height)
        .size(self.width, Some(height))
    }
}

/// How a [`Switch`] paints its track and its thumb.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchVariant {
    /// On, a thumb of the palette's surface on a track of its text, or of
    /// `ink`; off, a thumb of the palette's text on its selected fill.
    #[default]
    Default,
    /// A thumb of the palette's lightest color at full strength, the text of
    /// a dark palette and the surface of a light one, on and off: on a track
    /// of the ink, which is the palette's `link` color unless `ink` says
    /// otherwise, and off on the palette's muted color. Either track is
    /// darkened where needed until the thumb stands at a contrast of 3 or
    /// more on it. This is how chat applications draw a switch.
    Accent,
}

/// Side of a search field's clear button: its glyph and 4 around it.
const CLEAR_SIDE: f32 = ICON_SIZE + 8.0;
/// From the clear button to the field's end, so that its glyph is as far
/// from that end as the magnifier is from the start.
const CLEAR_INSET: f32 = 8.0 - (CLEAR_SIDE - ICON_SIZE) / 2.0;

// A circled cross that empties a search field; drawn disabled it is passive.
fn clear_button(id: String, palette: ComponentPalette, enabled: bool) -> Element {
    // An icon alone: it is named for what it does.
    let said = Access::new(Role::Button).name(crate::labels().clear_search);
    if enabled {
        Element::icon_button(id, Icon::CloseCircle, "")
            .foreground(palette.muted)
            .hover_foreground(palette.text)
            .pressed_foreground(palette.text)
            .hover_background(palette.hover)
            .pressed_background(palette.selected)
            .accessible(said)
    } else {
        Element::panel(
            Element::icon(Icon::CloseCircle)
                .foreground(palette.muted)
                .size(Some(ICON_SIZE), Some(ICON_SIZE))
                .centered(),
        )
        .accessible(said.id(id).disabled(true))
    }
    // Half the side is a shape, not a token.
    .radius(CLEAR_SIDE / 2.0)
    .size(Some(CLEAR_SIDE), Some(CLEAR_SIDE))
}

/// A caller-controlled switch with a single focusable target. The thumb is
/// painted inside its track and moves only when the caller passes a new
/// `checked` value after receiving `UiAction::Clicked`.
pub struct Switch {
    action_id: String,
    checked: bool,
    state: ControlState,
    palette: ComponentPalette,
    accessibility_label: Option<String>,
    size: ControlSize,
    track: Option<(f32, f32)>,
    variant: SwitchVariant,
    ink: Option<Color>,
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
            accessibility_label: None,
            size: ControlSize::Regular,
            track: None,
            variant: SwitchVariant::Default,
            ink: None,
        }
    }

    /// The track's width and height and the thumb's side: 28, 16 and 10
    /// pixels for `Dense`, 32, 18 and 12 for `Compact`, 36, 20 and 14 for
    /// `Regular`, which is the switch without a size, and 44, 24 and 18 for
    /// `Spacious`. The thumb is always 3 pixels inside the track.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self.track = None;
        self
    }

    /// A track of the host's own size, in logical pixels, in place of a
    /// [`ControlSize`]'s, such as a settings page's 50 by 26: the thumb is
    /// as tall as the track less 6, 3 pixels inside it. The height is at
    /// least 12 and the width leaves room for the thumb to travel, at least
    /// twice the height less 6. Values that are not finite are ignored.
    pub fn track_size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() {
            let height = height.max(12.0);
            self.track = Some((width.max(height * 2.0 - 6.0), height));
        }
        self
    }

    /// How the track and the thumb are painted; see [`SwitchVariant`].
    pub fn variant(mut self, variant: SwitchVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The track's color while the switch is on, in place of the palette's
    /// text, or of its `link` color for `SwitchVariant::Accent`, which
    /// darkens it where needed for its light thumb to read on it.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// What the switch is called to the platform. It shows no text of its
    /// own, so without this, or a `FormField` around it, it has no name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
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

impl Component for Switch {
    fn render(self) -> Element {
        const INSET: f32 = 3.0;
        let (track_width, track_height, thumb) = match (self.track, self.size) {
            (Some((width, height)), _) => (width, height, height - INSET * 2.0),
            (None, ControlSize::Dense) => (28.0, 16.0, 10.0),
            (None, ControlSize::Compact) => (32.0, 18.0, 12.0),
            (None, ControlSize::Regular) => (36.0, 20.0, 14.0),
            (None, ControlSize::Spacious) => (44.0, 24.0, 18.0),
        };
        let palette = self.palette;
        let (thumb_color, track_color) = match (self.variant, self.checked) {
            (SwitchVariant::Default, true) => (palette.surface, self.ink.unwrap_or(palette.text)),
            (SwitchVariant::Default, false) => (palette.text, palette.selected),
            (SwitchVariant::Accent, checked) => {
                // At full strength, and on a track darkened until it reads at 3:1, as state must.
                let light = if palette.is_dark() { palette.text } else { palette.surface };
                let light = Color::rgb(light.r, light.g, light.b);
                let track = if checked { self.ink.unwrap_or_else(|| palette.link()) } else { palette.muted };
                (light, crate::chip::readable_ink(track, light, 3.0))
            }
        };
        // The thumb is the strip's selection indicator: it slides between the
        // two slots when `checked` changes and keeps its place across rebuilds.
        let slot = |name: &str| {
            Element::spacer()
                .size(Some(thumb), Some(thumb))
                .indicator_slot(name)
        };
        let contents = Element::tab_strip(
            vec![
                slot("off"),
                Element::spacer().size(Some(track_width - thumb * 2.0 - INSET * 2.0), Some(thumb)),
                slot("on"),
            ],
            Vec::new(),
        )
        .tab_selection(
            format!("{}:thumb", self.action_id),
            if self.checked { "on" } else { "off" },
            thumb_color,
            thumb / 2.0,
            crate::tokens().motion.move_ms,
        );
        let enabled = self.state == ControlState::Enabled;
        // Drawn disabled it is passive, and known by the ID it would have had.
        let mut access = Access::new(Role::Switch)
            .id(self.action_id.clone())
            .toggled(self.checked)
            .disabled(!enabled);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
        }
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
            .padding(INSET)
            .radius(track_height / 2.0)
            .size(Some(track_width), Some(track_height))
            .accessible(access)
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
    /// Reported in `UiAction::Clicked` when the tab is chosen. A disabled tab is
    /// known to the platform by it.
    pub action_id: String,
    /// The tab's text.
    pub label: String,
    /// An icon before the label. `None` by default.
    pub icon: Option<Icon>,
    /// Whether it is the chosen tab: it takes the accent of [`Tabs`], or the
    /// text color without one, and the sliding selection rests under it,
    /// unless it is disabled.
    pub active: bool,
    /// Width in logical pixels. `new` keeps it at 40 or more and
    /// `Tabs::fill_width` replaces it.
    pub width: f32,
    /// `Disabled` keeps the tab in its place, in the muted color, and it cannot
    /// be chosen.
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

const TAB_GAP: f32 = 2.0;
const TAB_TRACK_PADDING: f32 = 2.0;
const TAB_ITEM_PADDING: f32 = 4.0;
const TAB_ICON_GAP: f32 = 4.0;
const TAB_FONT_SIZE: f32 = 11.5;
// The underline fills the track's bottom padding, so both variants keep one size.
const TAB_UNDERLINE: f32 = TAB_TRACK_PADDING;
const TAB_HAIRLINE: f32 = 1.0;

/// How [`Tabs`] mark the active tab. Both take the same size, report the same
/// actions and answer the same keys.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabsVariant {
    /// A filled selection on a rounded track: the palette's `selected` on its
    /// `surface`.
    #[default]
    Segmented,
    /// Text tabs with no track: a 2 px underline in the accent under the
    /// active tab, over a 1 px hairline in the palette's `border` as wide as
    /// the strip is laid out.
    Underlined,
}

/// A row of [`Tab`]s with a selection that slides to the active one.
pub struct Tabs {
    /// The tabs, in the order they are drawn.
    pub items: Vec<Tab>,
    /// `surface` fills the track and `selected` the sliding selection; `text` is
    /// the ink of the active tab unless [`accent`](Self::accent) is set, and
    /// `muted` of the others. Underlined, `border` draws the hairline and
    /// neither fill is used.
    pub palette: ComponentPalette,
    /// Height of a tab in logical pixels, 24 by default. The track is 4 taller.
    pub height: f32,
    /// Space between two tabs in logical pixels, 2 by default.
    pub gap: f32,
    /// Keys the selection's animation. `None`, the default, derives it from the
    /// first tab's `action_id`.
    pub id: Option<String>,
    /// How long the selection takes to slide to another tab, in milliseconds.
    /// The default is the `move_ms` token.
    pub animation_ms: u16,
    /// How the active tab is marked. [`TabsVariant::Segmented`] by default.
    pub variant: TabsVariant,
    /// Ink of the active tab, and color of the underline. `None`, the default,
    /// takes the palette's `text`.
    pub accent: Option<Color>,
    /// Corners of the selection, in logical pixels; the segmented track
    /// rounds 2 more so the two stay concentric. `None`, the default, takes
    /// the `control` radius token for the selection and `surface` for the track.
    pub radius: Option<f32>,
    /// Size of the labels in logical pixels, 11.5 by default.
    pub font_size: f32,
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
            variant: TabsVariant::Segmented,
            accent: None,
            radius: None,
            font_size: TAB_FONT_SIZE,
        }
    }

    /// Round the selection by `radius` logical pixels and the segmented track
    /// by 2 more, concentric with it, in place of the radius tokens. Half the
    /// height or more makes both a capsule. A negative radius becomes zero;
    /// one that is not finite is ignored.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = Some(radius.max(0.0));
        }
        self
    }

    /// Set the label size in logical pixels, in place of 11.5. A size that is
    /// not finite or not above zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size;
        }
        self
    }

    /// Choose how the active tab is marked; see [`TabsVariant`].
    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Ink the active tab, and its underline, in this color in place of the
    /// palette's `text`, such as a theme's `accent_text()`.
    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = Some(accent);
        self
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

    /// Set the space between two tabs, in logical pixels, never below 0. A gap
    /// that is not finite is ignored. Set it before
    /// [`fill_width`](Self::fill_width), which reads it.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
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
        let accent = self.accent.unwrap_or(self.palette.text);
        let selection_radius = self.radius.unwrap_or_else(control_radius);
        let track_radius = self.radius.map_or_else(surface_radius, |radius| radius + TAB_TRACK_PADDING);
        let font_size = self.font_size;
        // Underlined, a strip of slots under the tabs gives the sliding
        // underline its thin target while the tabs keep their full height.
        let slots = (self.variant == TabsVariant::Underlined).then(|| {
            self.items
                .iter()
                .map(|item| {
                    Element::spacer()
                        .size(Some(item.width), Some(TAB_UNDERLINE))
                        .indicator_slot(item.action_id.clone())
                })
                .collect::<Vec<_>>()
        });
        let items = self
            .items
            .into_iter()
            .map(|item| {
                let ink = if item.active {
                    accent
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
                    lighter(accent, 12)
                } else {
                    self.palette.text
                };
                let access = Access::new(Role::Tab).selected(item.active);
                if item.state == ControlState::Disabled {
                    let access = access.id(item.action_id).name(item.label.clone()).disabled(true);
                    let label = Element::text(item.label)
                        .foreground(self.palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(font_size);
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
                        // At the height of an enabled tab's label.
                        visual.centered_y(),
                        Element::spacer().flex(1.0),
                    ])
                    .padding(TAB_ITEM_PADDING)
                    .background(Color::TRANSPARENT)
                    .radius(selection_radius)
                    .size(Some(item.width), Some(self.height))
                    .accessible(access);
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
                    .font_size(font_size)
                    .padding(TAB_ITEM_PADDING)
                    .gap(TAB_ICON_GAP)
                    .radius(selection_radius)
                    .center_content()
                    .size(Some(item.width), Some(self.height))
                    .accessible(access)
            })
            .collect();
        let strip = Element::tab_strip(items, enabled_ids)
            .gap(self.gap)
            .padding(TAB_TRACK_PADDING)
            .size(None, Some(self.height + TAB_TRACK_PADDING * 2.0));
        if let Some(slots) = slots {
            let mut underline = Element::tab_strip(slots, Vec::new())
                .gap(self.gap)
                .size(None, Some(TAB_UNDERLINE));
            if let Some(selected_id) = selected_id {
                underline =
                    underline.tab_selection(group_id, selected_id, accent, 0.0, self.animation_ms);
            }
            let strip_height = self.height + TAB_TRACK_PADDING * 2.0;
            // Anchored to the bottom of the strip, not of the box: a box taller
            // than the strip stretches the stack, never the tabs.
            let foot = |line: Element, height: f32| {
                Element::column(vec![
                    Element::spacer().size(None, Some(strip_height - height)),
                    line,
                ])
            };
            return Element::stack(vec![
                foot(
                    Element::spacer()
                        .size(None, Some(TAB_HAIRLINE))
                        .background(self.palette.border),
                    TAB_HAIRLINE,
                ),
                foot(
                    Element::row(vec![
                        Element::spacer().size(Some(TAB_TRACK_PADDING), Some(TAB_UNDERLINE)),
                        underline,
                    ]),
                    TAB_UNDERLINE,
                ),
                strip,
            ])
            .size(None, Some(strip_height));
        }
        let strip = strip
            .background(self.palette.surface)
            .radius(track_radius);
        if let Some(selected_id) = selected_id {
            strip.tab_selection(
                group_id,
                selected_id,
                self.palette.selected,
                selection_radius,
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
    /// The element it opens from, told to the platform as expanded or not. It
    /// is all that is drawn while the popover is closed.
    pub anchor: Element,
    /// What the floating surface holds, inside 6 pixels of padding.
    pub content: Element,
    /// Whether the surface shows. The host owns it.
    pub open: bool,
    /// Reported in `UiAction::Clicked` when the popover is dismissed, for the
    /// host to close it.
    pub dismiss_id: String,
    /// The side of the anchor it prefers. `BottomStart` by default.
    pub placement: PopoverPlacement,
    /// Width of the surface in logical pixels, 240 by default. The `width`
    /// builder keeps it at 96 or more.
    pub width: f32,
    /// `raised` fills the surface, `border` outlines it and `shadow` falls under it.
    pub palette: ComponentPalette,
    /// Duration of the entrance in milliseconds. The default is the `enter_ms`
    /// token.
    pub animation_ms: u16,
    /// Duration of the exit in milliseconds. `None`, the default, keeps the
    /// engine's own.
    pub close_animation_ms: Option<u16>,
    /// The scale the entrance starts at. 1.0, the default, does not scale; the
    /// `open_scale` builder keeps it within 0.5..=1.25.
    pub open_scale: f32,
    /// Names the view the content shows: a change of key while the popover stays
    /// open is animated. `None` by default.
    pub view_key: Option<String>,
}

impl Popover {
    /// A popover on `anchor` that shows `content` while `open` is true.
    /// Escape reports `UiAction::Clicked` with `dismiss_id`, for the host to
    /// close it. It opens under the start of the anchor, 240 pixels wide.
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

    /// The side of the anchor it prefers. The default is `BottomStart`.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Width of the floating surface in logical pixels, at least 96. A width
    /// that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// Duration of the entrance, in milliseconds. The default is the enter
    /// duration of the motion tokens.
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    /// Duration of the exit, in milliseconds, in place of the default short
    /// fade.
    pub fn close_animation_ms(mut self, duration: u16) -> Self {
        self.close_animation_ms = Some(duration);
        self
    }

    /// The scale the entrance starts at, clamped to 0.5..=1.25. The default,
    /// 1.0, does not scale; a value that is not finite is ignored.
    pub fn open_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.open_scale = scale.clamp(0.5, 1.25);
        }
        self
    }

    /// Name the view the content shows, so that a change of key while the
    /// popover stays open is animated.
    pub fn view_key(mut self, key: impl Into<String>) -> Self {
        self.view_key = Some(key.into());
        self
    }
}

impl Component for Popover {
    fn render(self) -> Element {
        // What the anchor opens is open or it is not; said of the anchor
        // alone, so it cannot reach a control of the content.
        let anchor = self.anchor.accessible(Access::default().expanded(self.open));
        if !self.open {
            return anchor;
        }
        let floating = Element::panel(self.content)
            .padding(6.0)
            .size(Some(self.width), None)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(surface_radius())
            .shadow(self.palette.shadow, Point::new(0.0, 9.0), 22.0, 0.0);
        let mut popover = Element::popover(anchor, floating, self.dismiss_id, self.placement)
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
    /// What the host receives when the option is chosen.
    pub action_id: String,
    /// The option's text, and its name for the platform.
    pub label: String,
    /// A leading icon. `None` by default.
    pub icon: Option<Icon>,
    /// Whether it is the current choice.
    pub selected: bool,
    /// Whether it can be chosen. A disabled option stays in the list.
    pub state: ControlState,
}

impl DropdownOption {
    /// An enabled option, not selected, that reports `action_id`.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            state: ControlState::Enabled,
        }
    }

    /// Give the option an icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark the option as the current choice.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disable the option.
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
    /// The element the list opens from: a button, a field or any other trigger.
    pub anchor: Element,
    /// The options, in the order they are listed.
    pub options: Vec<DropdownOption>,
    /// Whether the list shows. The host owns it.
    pub open: bool,
    /// Reported when the list is dismissed, for the host to close it.
    pub dismiss_id: String,
    /// Width of the list in logical pixels, 240 by default and 96 at least when
    /// it is drawn. `sizing` replaces it with the profile's preferred width.
    pub width: f32,
    /// The side of the anchor it prefers. `BottomStart` by default.
    pub placement: PopoverPlacement,
    /// The colors of the list's surface and of its rows.
    pub palette: ComponentPalette,
    /// Durations of the opening and of the item feedback.
    pub motion: MenuMotion,
    /// A selector profile for the row density and the scroll budget. `None`, the
    /// default, keeps the menu's own.
    pub sizing: Option<crate::SelectorSizing>,
}

impl DropdownList {
    /// A list of `options` on `anchor`, shown while `open` is true and
    /// dismissed with `dismiss_id`. It opens under the start of the anchor,
    /// 240 pixels wide.
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

    /// Width of the list in logical pixels, at least 96. A width that is not
    /// finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// The side of the anchor it prefers. The default is `BottomStart`.
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

    /// Durations of the opening and of the item feedback; see `MenuMotion`.
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
    use crate::{AccessNode, DrawCommand, ElementKind, Point, Size, UiAction, UiEvent, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    #[test]
    fn a_button_is_named_by_its_label_and_described_by_its_hint_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let save = Button::new("save", "Save", palette).tooltip("Save every open file").enabled(enabled);
            let said = access_node(save.render(), "save");
            assert_eq!((said.role, said.name.as_str()), (Role::Button, "Save"));
            assert_eq!(said.description.as_deref(), Some("Save every open file"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            // A hint that repeats the label is not said twice.
            let same = access_node(Button::new("save", "Save", palette).tooltip("Save").enabled(enabled).render(), "save");
            assert_eq!((same.name.as_str(), same.description), ("Save", None));
            let named = Button::new("save", "Save", palette).accessibility_label("Save the file").enabled(enabled);
            assert_eq!(access_node(named.render(), "save").name, "Save the file");
        }
        let busy = access_node(Button::new("save", "Save", palette).loading(true).render(), "save");
        assert!(busy.disabled && !busy.actionable, "a busy button cannot be used");
    }

    #[test]
    fn an_icon_button_is_named_by_its_hint_or_by_the_name_it_is_given() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let button = || IconButton::new("close", Icon::Close, palette).enabled(enabled);
            let bare = access_node(button().render(), "close");
            assert_eq!((bare.role, bare.name.as_str(), bare.disabled), (Role::Button, "", !enabled));
            let hinted = access_node(button().tooltip("Close").render(), "close");
            assert_eq!((hinted.name.as_str(), hinted.description), ("Close", None));
            // A suppressed hint is not drawn; it still names the button.
            let quiet = button().tooltip_with(TooltipSpec::new("Close").suppressed(true));
            assert_eq!(access_node(quiet.render(), "close").name, "Close");
            let named = access_node(button().tooltip("Esc").accessibility_label("Close").render(), "close");
            assert_eq!((named.name.as_str(), named.description.as_deref()), ("Close", Some("Esc")));
            let labelled = access_node(button().label("Close tab").render(), "close");
            assert_eq!(labelled.name, "Close tab");
        }
    }

    #[test]
    fn a_link_is_a_link_named_by_its_label_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let said = access_node(Link::new("docs", "Read the docs", palette).enabled(enabled).render(), "docs");
            assert_eq!((said.role, said.name.as_str()), (Role::Link, "Read the docs"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
    }

    #[test]
    fn a_switch_says_whether_it_is_on_and_takes_its_name_from_the_host() {
        let palette = ComponentPalette::neutral_dark();
        for (checked, enabled) in [(true, true), (false, true), (true, false), (false, false)] {
            let switch = Switch::new("wifi", checked, palette).enabled(enabled).accessibility_label("Wi-Fi");
            let said = access_node(switch.render(), "wifi");
            assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::Switch, "Wi-Fi", Some(checked)));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
        assert_eq!(access_node(Switch::new("wifi", true, palette).render(), "wifi").name, "");
    }

    #[test]
    fn a_text_field_is_named_by_its_placeholder_or_the_name_it_is_given_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let field = || TextField::new("name", "limn", "e.g. my-project", palette).enabled(enabled);
            let said = access_node(field().render(), "name");
            assert_eq!((said.role, said.name.as_str()), (Role::TextInput, "e.g. my-project"));
            assert_eq!(said.value_text.as_deref(), Some("limn"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(field().accessibility_label("Project name").render(), "name").name, "Project name");
            let secret = access_node(field().password(true).render(), "name");
            assert_eq!((secret.role, secret.value_text), (Role::PasswordInput, None));
        }
    }

    #[test]
    fn a_text_area_is_a_multiline_field_with_its_name_and_value_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let area = || TextArea::new("notes", "First line", "Notes", palette).enabled(enabled);
            let said = access_node(area().render(), "notes");
            assert_eq!((said.role, said.name.as_str()), (Role::MultilineTextInput, "Notes"));
            assert_eq!(said.value_text.as_deref(), Some("First line"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(area().accessibility_label("Release notes").render(), "notes").name, "Release notes");
        }
    }

    #[test]
    fn a_search_field_is_a_search_input_with_its_name_and_query_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let field = || SearchField::new("search", "tabs", "Search", palette).shortcut("Ctrl+K").enabled(enabled);
            let said = access_node(field().render(), "search");
            assert_eq!((said.role, said.name.as_str()), (Role::SearchInput, "Search"));
            assert_eq!(said.value_text.as_deref(), Some("tabs"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(field().accessibility_label("Search files").render(), "search").name, "Search files");
        }
    }

    #[test]
    fn tabs_and_segments_are_tabs_and_the_chosen_one_says_so() {
        let palette = ComponentPalette::neutral_dark();
        let items = || {
            vec![
                Tab::new("files", "Files", 80.0).active(true),
                Tab::new("search", "Search", 80.0).icon(Icon::Search),
                Tab::new("git", "Git", 80.0).disabled(),
            ]
        };
        for strip in [Tabs::new(items(), palette).render(), SegmentedControl::new(items(), palette).render()] {
            let row = Element::row(vec![strip, Element::spacer().flex(1.0)]);
            let nodes = UiTree::new(row).layout(Size::new(480.0, 60.0)).access;
            let said: Vec<_> = nodes
                .iter()
                .map(|node| (node.id.as_str(), node.role, node.name.as_str(), node.selected, node.disabled))
                .collect();
            assert_eq!(
                said,
                [
                    ("files", Role::Tab, "Files", Some(true), false),
                    ("search", Role::Tab, "Search", Some(false), false),
                    ("git", Role::Tab, "Git", Some(false), true),
                ]
            );
            assert!(nodes[0].actionable && !nodes[2].actionable);
        }
    }

    #[test]
    fn a_popover_tells_its_anchor_whether_it_is_open() {
        let palette = ComponentPalette::neutral_dark();
        for open in [false, true] {
            let popover = Popover::new(
                Element::panel(Button::new("more", "More", palette).render()),
                Button::new("inside", "Inside", palette).render(),
                open,
                "dismiss",
                palette,
            );
            let nodes = UiTree::new(Element::column(vec![popover.render(), Element::spacer().flex(1.0)]))
                .layout(Size::new(480.0, 320.0))
                .access;
            let anchor = nodes.iter().find(|node| node.id == "more").expect("the anchor");
            assert_eq!((anchor.role, anchor.name.as_str(), anchor.expanded), (Role::Button, "More", Some(open)));
            // What is said of the anchor is not said of what it opens.
            let inside = nodes.iter().find(|node| node.id == "inside");
            assert_eq!(inside.map(|node| node.expanded), open.then_some(None));
        }
    }

    #[test]
    fn a_tooltip_describes_the_control_it_is_anchored_to() {
        let palette = ComponentPalette::neutral_dark();
        let hinted = Tooltip::new(Button::new("run", "Run", palette).render(), "run", "Run the tests", palette);
        let said = access_node(hinted.render(), "run");
        assert_eq!((said.name.as_str(), said.description.as_deref()), ("Run", Some("Run the tests")));
        // A passive anchor is not made a node by its hint.
        let passive = Tooltip::new(Element::text("12 files"), "count", "In this folder", palette).render();
        assert!(UiTree::new(passive).layout(Size::new(200.0, 60.0)).access.is_empty());
    }

    #[test]
    fn a_dropdown_list_says_which_option_is_the_chosen_one() {
        let palette = ComponentPalette::neutral_dark();
        let list = DropdownList::new(
            Button::new("open", "Shell", palette).render(),
            vec![
                DropdownOption::new("bash", "Bash").selected(true),
                DropdownOption::new("zsh", "Zsh"),
                DropdownOption::new("off", "Off").disabled(),
            ],
            true,
            "dismiss",
            palette,
        );
        let mut tree = UiTree::new(Element::column(vec![list.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let nodes = tree.layout(Size::new(480.0, 320.0)).access;
        let said = |id: &str| {
            let node = nodes.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id} among {nodes:?}"));
            (node.role, node.name.as_str(), node.selected, node.disabled)
        };
        assert_eq!(said("bash"), (Role::MenuItem, "Bash", Some(true), false));
        assert_eq!(said("zsh"), (Role::MenuItem, "Zsh", Some(false), false));
        assert_eq!(said("off"), (Role::MenuItem, "Off", Some(false), true));
    }

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
                assert_eq!((element.style.width, element.style.height), (Some(height), Some(height)));
                assert_eq!(painted_fill(element), (height, height / 2.0));
            }
            let pill = || Button::new("save", "Save", palette).size(size).radius(RadiusRole::Card).round();
            assert_eq!(painted_fill(pill().render()), (height, height / 2.0));
            // Given another height after it is built, it is still a pill.
            assert_eq!(painted_fill(pill().render().size(None, Some(44.0))), (44.0, 22.0));
            assert_eq!(painted_fill(pill().disabled().render().size(None, Some(45.0))), (45.0, 22.5));
        }
        // A radius larger than the shape is painted as the half it can be.
        let square = Button::new("save", "Save", palette).radius(RadiusRole::Panel).render().size(None, Some(10.0));
        assert_eq!(painted_fill(square), (10.0, 5.0));
    }

    /// The height and radius of the first fill a lone element paints.
    fn painted_fill(element: Element) -> (f32, f32) {
        let mut tree = UiTree::new(Element::column(vec![element]));
        tree.layout(Size::new(400.0, 200.0))
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. } => Some((rect.height, *radius)),
                _ => None,
            })
            .expect("a fill")
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

    // A field at the top left of a window, at its own size.
    fn field_tree(field: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![field, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    #[test]
    fn a_text_field_icon_sits_before_its_text_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let drawn = |field: TextField| {
            let frame = field_tree(field.width(240.0).render()).layout(Size::new(400.0, 120.0));
            let icon = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Icon {
                    rect,
                    icon: Icon::Hash,
                    tint,
                } => Some((*rect, *tint)),
                _ => None,
            });
            let text = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == "general" => Some(*position),
                _ => None,
            });
            (icon.expect("the icon"), text.expect("the text"))
        };
        let field = || TextField::new("name", "general", "Name", palette).icon(Icon::Hash);
        let ((icon, tint), text) = drawn(field());
        // In the padding's corner, muted, the text 8 after it and the line
        // in the middle of the field.
        assert_eq!((icon.x, icon.width, tint), (8.0, ICON_SIZE, palette.muted));
        assert_eq!(icon.y + icon.height / 2.0, 16.0);
        assert_eq!(text.x, 8.0 + ICON_SIZE + 8.0);
        // Disabled, the two stay where they were, both muted.
        let ((still, muted), moved) = drawn(field().disabled());
        assert_eq!((still, muted, moved.x), (icon, palette.muted, text.x));
        // Without one the text starts in the padding's corner, as before.
        let frame = field_tree(TextField::new("name", "general", "Name", palette).render())
            .layout(Size::new(400.0, 120.0));
        assert!(!frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Icon { .. })));
        // Focused, the icon takes the text's ink, as a search field's magnifier does.
        let mut tree = field_tree(field().render());
        tree.layout(Size::new(400.0, 120.0));
        assert!(tree.focus("name"));
        let focused = tree.layout(Size::new(400.0, 120.0));
        assert!(focused.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::Hash, tint, .. } if *tint == palette.text)));
    }

    #[test]
    fn a_text_field_icon_stays_inside_a_narrow_field() {
        let palette = ComponentPalette::neutral_dark();
        for width in [40.0, 80.0, 180.0] {
            for field in [
                TextField::new("name", "a long value", "Name", palette)
                    .icon(Icon::Hash)
                    .width(width),
                TextField::new("name", "a long value", "Name", palette)
                    .icon(Icon::Hash)
                    .width(width)
                    .disabled(),
            ] {
                let frame = field_tree(field.render()).layout(Size::new(400.0, 120.0));
                for command in &frame.commands {
                    if let DrawCommand::Icon { rect, .. } = command {
                        assert!(
                            rect.x >= 0.0 && rect.x + rect.width <= width,
                            "{width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_clearable_search_field_has_its_button_only_while_there_is_a_query() {
        let palette = ComponentPalette::neutral_dark();
        let window = Size::new(400.0, 120.0);
        let field = |query: &str| {
            SearchField::new("query", query, "Search", palette)
                .clearable(true)
                .width(280.0)
        };
        let ids = |tree: &mut UiTree| -> Vec<String> {
            tree.layout(window)
                .regions
                .iter()
                .map(|region| region.id.clone())
                .collect()
        };
        // Empty, or with a query but not clearable, it is one input as before.
        assert_eq!(ids(&mut field_tree(field("").render())), ["query"]);
        let plain = SearchField::new("query", "rust", "Search", palette).width(280.0);
        assert_eq!(ids(&mut field_tree(plain.render())), ["query"]);

        let mut tree = field_tree(field("rust").render());
        let frame = tree.layout(window);
        let rect = |id: &str| {
            frame
                .regions
                .iter()
                .find(|region| region.id == id)
                .map(|region| region.rect)
        };
        let (input, clear) = (
            rect("query").unwrap(),
            rect("query:clear").expect("the clear button"),
        );
        // A circle at the end whose glyph is as far from it as the magnifier
        // is from the start, on the field's middle line.
        assert_eq!((input.width, input.height), (280.0, 36.0));
        assert_eq!((clear.width, clear.height), (24.0, 24.0));
        assert_eq!(input.x + input.width - (clear.x + clear.width), 4.0);
        assert_eq!(clear.y + clear.height / 2.0, input.y + input.height / 2.0);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::CloseCircle, rect, tint }
                if *tint == palette.muted && rect.x + rect.width == input.x + input.width - 8.0)));
        // The text, its caret and its selection are drawn before the button.
        let text = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::PushClip(clip) if clip.x == input.x + 8.0 + ICON_SIZE + 8.0 => {
                    Some(*clip)
                }
                _ => None,
            })
            .expect("the text's clip");
        assert!(
            text.x + text.width < clear.x,
            "{text:?} runs under {clear:?}"
        );
        assert_eq!(
            frame.hit_test(Point::new(clear.x + 12.0, clear.y + 12.0)),
            Some("query:clear")
        );

        // The pointer on the button keeps the field lit and lights the button.
        let middle = Point::new(clear.x + 12.0, clear.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(middle));
        let lit = tree.layout(window);
        let fill = |rect: crate::Rect| {
            lit.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect {
                    rect: drawn, color, ..
                } if *drawn == rect => Some(*color),
                _ => None,
            })
        };
        assert_eq!(fill(input), Some(lighter(palette.raised, 5)));
        assert_eq!(fill(clear), Some(palette.hover));

        // A click reports it; the host empties its query and the field.
        tree.handle_event(UiEvent::PointerDown(middle));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(middle)),
            vec![UiAction::Clicked {
                id: "query:clear".into()
            }]
        );
        tree.set_input_value("query", "");
        tree.set_root(Element::column(vec![
            Element::row(vec![field("").render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        assert_eq!(ids(&mut tree), ["query"]);
        assert_eq!(tree.input_value("query"), Some(""));
    }

    #[test]
    fn typing_into_a_clearable_search_field_keeps_its_focus_and_caret_as_the_button_comes() {
        let palette = ComponentPalette::neutral_dark();
        let window = Size::new(400.0, 120.0);
        let view = |query: &str| {
            Element::column(vec![
                Element::row(vec![
                    SearchField::new("query", query, "Search", palette)
                        .clearable(true)
                        .width(280.0)
                        .render(),
                    Element::spacer().flex(1.0),
                ]),
                Element::spacer().flex(1.0),
            ])
        };
        let mut tree = UiTree::new(view(""));
        tree.layout(window);
        assert!(tree.focus("query"));
        let reported = tree.handle_event(UiEvent::Text("ru".into()));
        assert_eq!(
            reported,
            vec![UiAction::InputChanged {
                id: "query".into(),
                value: "ru".into()
            }]
        );
        // The host stores the query; the field is rebuilt with its button.
        tree.set_root(view("ru"));
        let frame = tree.layout(window);
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "query:clear"));
        assert_eq!(tree.focused_id(), Some("query"));
        assert_eq!(
            (tree.input_value("query"), tree.input_cursor("query")),
            (Some("ru"), Some(2))
        );
        tree.handle_event(UiEvent::Text("st".into()));
        assert_eq!(tree.input_value("query"), Some("rust"));
    }

    #[test]
    fn a_disabled_clearable_search_field_keeps_a_passive_clear_button_in_place() {
        let palette = ComponentPalette::neutral_dark();
        let field = || {
            SearchField::new("query", "rust", "Search", palette)
                .clearable(true)
                .width(280.0)
        };
        let node = |element: Element| {
            let frame = field_tree(element).layout(Size::new(400.0, 120.0));
            assert!(frame
                .regions
                .iter()
                .all(|region| region.id != "query:clear"));
            frame
                .access
                .iter()
                .find(|node| node.id == "query:clear")
                .cloned()
                .expect("its node")
        };
        let said = access_node(field().render(), "query:clear");
        assert_eq!(
            (said.role, said.name.as_str(), said.disabled),
            (Role::Button, "Clear search", false)
        );
        let disabled = node(field().disabled().render());
        assert_eq!(
            (disabled.role, disabled.name.as_str(), disabled.disabled),
            (Role::Button, "Clear search", true)
        );
        assert_eq!(disabled.rect, said.rect);
        // Without a query there is nothing to clear, enabled or not.
        let empty = field_tree(
            SearchField::new("query", "", "Search", palette)
                .clearable(true)
                .disabled()
                .render(),
        )
        .layout(Size::new(400.0, 120.0));
        assert!(empty.access.iter().all(|node| node.id != "query:clear"));
    }

    #[test]
    fn a_clearable_search_field_keeps_its_button_inside_at_every_width_and_size() {
        let palette = ComponentPalette::neutral_dark();
        for size in [
            ControlSize::Dense,
            ControlSize::Compact,
            ControlSize::Regular,
            ControlSize::Spacious,
        ] {
            for width in [60.0, 120.0, 360.0] {
                let field =
                    SearchField::new("query", "a query longer than the field", "Search", palette)
                        .clearable(true)
                        .size(size)
                        .width(width);
                let frame = field_tree(field.render()).layout(Size::new(400.0, 120.0));
                let rect = |id: &str| {
                    frame
                        .regions
                        .iter()
                        .find(|region| region.id == id)
                        .unwrap()
                        .rect
                };
                let (input, clear) = (rect("query"), rect("query:clear"));
                assert!(
                    clear.x >= input.x
                        && clear.x + clear.width <= input.x + input.width
                        && clear.y >= input.y
                        && clear.y + clear.height <= input.y + input.height,
                    "{size:?} at {width}: {clear:?} outside {input:?}"
                );
                // As wide as it is empty: never under a field's 80.
                let empty = field_tree(
                    SearchField::new("query", "", "Search", palette)
                        .clearable(true)
                        .size(size)
                        .width(width)
                        .render(),
                )
                .layout(Size::new(400.0, 120.0));
                assert_eq!(
                    input.width, empty.regions[0].rect.width,
                    "{size:?} at {width}"
                );
                assert_eq!(input.width, f32::max(width, 80.0));
            }
        }
        // The narrowest field still has room for what is typed in it.
        let frame = field_tree(
            SearchField::new("query", "a", "Search", palette)
                .clearable(true)
                .width(60.0)
                .render(),
        )
        .layout(Size::new(400.0, 120.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "a")));
    }

    #[test]
    fn a_disabled_search_field_shows_its_hint_only_without_a_query() {
        let palette = ComponentPalette::neutral_dark();
        let drawn = |query: &str, clearable: bool| {
            let frame = field_tree(
                SearchField::new("query", query, "Search", palette)
                    .clearable(clearable)
                    .shortcut("Ctrl+K")
                    .width(140.0)
                    .disabled()
                    .render(),
            )
            .layout(Size::new(400.0, 120.0));
            let texts: Vec<String> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            (
                texts.iter().any(|text| text == "Ctrl"),
                texts.iter().any(|text| text == "rust"),
            )
        };
        // As the enabled field away from the focus: the hint while it is
        // empty, the query and its clear button once it is not.
        assert_eq!(drawn("", true), (true, false));
        assert_eq!(drawn("rust", true), (false, true));
        assert_eq!(drawn("rust", false), (false, true));
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
    fn a_picture_takes_the_icon_slot_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let picture = || {
            RasterImage::new("a".repeat(64), crate::RasterImageFormat::Png, 4, std::sync::Arc::from("AAAA"), "party")
                .unwrap()
        };
        let pictured = |frame: &crate::Frame| {
            let rects: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::RasterImage { rect, .. } => Some(*rect),
                    _ => None,
                })
                .collect();
            assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { .. })), "no glyph");
            assert_eq!(rects.len(), 1, "one picture");
            rects[0]
        };
        let button = || Button::new("join", "Join", palette).image(picture()).variant(ButtonVariant::Raised);
        let mut tree = UiTree::new(Element::row(vec![button().render()]));
        let frame = tree.layout(Size::new(300.0, 32.0));
        let region = frame.regions[0].rect;
        let slot = pictured(&frame);
        assert_eq!((slot.width, slot.height), (ICON_SIZE, ICON_SIZE));
        assert_eq!(slot.y - region.y, (region.height - ICON_SIZE) / 2.0);
        // The plain button is narrower by the slot and its gap.
        let plain = UiTree::new(Element::row(vec![Button::new("join", "Join", palette).variant(ButtonVariant::Raised).render()]))
            .layout(Size::new(300.0, 32.0));
        let gap = ControlSize::Regular.button_sizing().inner_spacing;
        assert_eq!(region.width, plain.regions[0].rect.width + ICON_SIZE + gap);
        tree.handle_event(UiEvent::PointerDown(Point::new(slot.x + 1.0, slot.y + 1.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(slot.x + 1.0, slot.y + 1.0))),
            vec![UiAction::Clicked { id: "join".into() }]
        );

        let disabled = UiTree::new(Element::row(vec![button().disabled().render()])).layout(Size::new(300.0, 32.0));
        assert!(disabled.regions.is_empty());
        let still = pictured(&disabled);
        assert_eq!((still.width, still.height), (ICON_SIZE, ICON_SIZE));
        assert!(disabled.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. } if rect.width == region.width && *color == palette.raised
        )));
        assert!(disabled.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { color, .. } if *color == palette.muted
        )));

        // A picture alone: square, and called by its text alternative.
        for enabled in [true, false] {
            let alone = Button::new("party", "", palette).image(picture()).enabled(enabled);
            assert_eq!(access_node(alone.render(), "party").name, "party");
        }
        let alone = UiTree::new(Element::row(vec![Button::new("party", "", palette).image(picture()).render()]))
            .layout(Size::new(100.0, 32.0));
        let (square, slot) = (alone.regions[0].rect, pictured(&alone));
        assert_eq!((square.width, square.height), (32.0, 32.0));
        assert_eq!((slot.x - square.x, slot.y - square.y), (8.0, 8.0));
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
                ctrl: false,
                alt: false
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
            alt: false,
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
                alt: false,
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

    /// A switch at its own size, settled: its track's rectangle and color,
    /// then its thumb's.
    fn switch_paint(switch: Switch) -> ((crate::Rect, Color), (crate::Rect, Color)) {
        let mut tree = UiTree::new(Element::row(vec![switch.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 40.0));
        let fills: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        // The track is the pill the others stand in; the thumb is the circle inside it.
        let [(track, track_color, _), (thumb, thumb_color, thumb_radius)] = fills.as_slice() else {
            panic!("a track and a thumb, got {fills:?}");
        };
        assert_eq!(*thumb_radius, thumb.width / 2.0);
        ((*track, *track_color), (*thumb, *thumb_color))
    }

    #[test]
    fn a_switch_size_sets_its_track_and_its_thumb_three_pixels_inside_it() {
        let palette = ComponentPalette::neutral_dark();
        for (size, width, height, side) in [
            (ControlSize::Dense, 28.0, 16.0, 10.0),
            (ControlSize::Compact, 32.0, 18.0, 12.0),
            (ControlSize::Regular, 36.0, 20.0, 14.0),
            (ControlSize::Spacious, 44.0, 24.0, 18.0),
        ] {
            for checked in [false, true] {
                let ((track, _), (thumb, _)) = switch_paint(Switch::new("s", checked, palette).size(size));
                assert_eq!((track.width, track.height), (width, height), "{size:?}");
                assert_eq!((thumb.width, thumb.height), (side, side), "{size:?}");
                let x = if checked { width - 3.0 - side } else { 3.0 };
                assert_eq!((thumb.x - track.x, thumb.y - track.y), (x, 3.0), "{size:?}, checked {checked}");
            }
        }
        // Without a size it is the regular one.
        assert_eq!(switch_paint(Switch::new("s", true, palette)).0 .0.width, 36.0);
    }

    #[test]
    fn a_track_of_its_own_size_keeps_the_thumb_three_pixels_inside_it() {
        let palette = ComponentPalette::neutral_dark();
        for (asked, (width, height)) in [((50.0, 26.0), (50.0, 26.0)), ((10.0, 26.0), (46.0, 26.0)), ((30.0, 4.0), (30.0, 12.0))] {
            for checked in [false, true] {
                let switch = Switch::new("s", checked, palette)
                    .size(ControlSize::Dense)
                    .track_size(asked.0, asked.1)
                    .track_size(f32::NAN, 40.0);
                let ((track, _), (thumb, _)) = switch_paint(switch);
                let side = height - 6.0;
                assert_eq!((track.width, track.height), (width, height), "{asked:?}");
                assert_eq!((thumb.width, thumb.height), (side, side), "{asked:?}");
                let x = if checked { width - 3.0 - side } else { 3.0 };
                assert_eq!((thumb.x - track.x, thumb.y - track.y), (x, 3.0), "{asked:?}, checked {checked}");
            }
        }
        // A size set after it takes its place again.
        let ((track, _), _) = switch_paint(Switch::new("s", true, palette).track_size(50.0, 26.0).size(ControlSize::Compact));
        assert_eq!((track.width, track.height), (32.0, 18.0));
    }

    #[test]
    fn the_accent_variant_keeps_a_light_thumb_on_the_link_color_or_the_ink_and_on_the_muted_color_off() {
        let dark = ComponentPalette::neutral_dark();
        let light = ComponentPalette {
            surface: Color::rgb(255, 255, 255),
            raised: Color::rgb(250, 250, 250),
            hover: Color::rgb(235, 235, 235),
            selected: Color::rgb(220, 220, 220),
            border: Color::rgb(205, 205, 205),
            text: Color::rgb(32, 32, 32),
            muted: Color::rgb(100, 100, 100),
            shadow: Color::rgba(0, 0, 0, 40),
        };
        let ink = Color::rgb(88, 101, 242);
        // A theme's text and muted color are translucent; its thumb is the foreground at full strength.
        let theme = crate::Theme::DARK.palette();
        // The thumb is the palette's lightest color: its text on a dark one, its surface on a light one.
        for (palette, thumb) in [(dark, dark.text), (light, light.surface), (theme, Color::rgb(255, 255, 255))] {
            let accent = |checked| Switch::new("s", checked, palette).variant(SwitchVariant::Accent);
            let paint = |switch: Switch| {
                let ((_, track), (_, thumb)) = switch_paint(switch);
                (track, thumb)
            };
            // A pale ink, such as the gallery theme's accent, is darkened like the link color.
            let pale = Color::rgb(25, 170, 216);
            for switch in [accent(true), accent(false), accent(true).ink(pale)] {
                let (track, drawn) = paint(switch);
                assert_eq!(drawn, thumb);
                let contrast = crate::chip::contrast(thumb, track);
                assert!(contrast >= 3.0, "{palette:?}: {track:?} at {contrast}");
            }
            for checked in [true, false] {
                assert_eq!(paint(accent(checked).disabled()), paint(accent(checked)), "disabled, it keeps its paint");
            }
            assert_eq!(paint(accent(true).ink(ink)), (ink, thumb), "an ink that reads is kept");
            assert_eq!(paint(accent(false).ink(ink)), paint(accent(false)));
        }
        // Where the link and muted colors already read, they are the track.
        let paint = |checked| switch_paint(Switch::new("s", checked, light).variant(SwitchVariant::Accent)).0 .1;
        assert_eq!((paint(true), paint(false)), (light.link(), light.muted));
        // The default variant takes the ink for its track and keeps its own thumb.
        let paint = |checked| {
            let ((_, track), (_, thumb)) = switch_paint(Switch::new("s", checked, dark).ink(ink));
            (track, thumb)
        };
        assert_eq!((paint(true), paint(false)), ((ink, dark.surface), (dark.selected, dark.text)));
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
            alt: false,
        };
        assert!(tree.focus("sessions"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: crate::Key::ArrowRight,
                shift: true,
                ctrl: false,
                alt: false,
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
                alt: false,
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
            alt: false,
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
                alt: false,
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

    fn underlined(active: &str, accent: Color) -> Element {
        Tabs::new(
            ["general", "roles", "emoji"]
                .iter()
                .map(|id| Tab::new(*id, id.to_uppercase(), 80.0).active(*id == active))
                .chain([Tab::new("audit", "Audit", 80.0).disabled()])
                .collect(),
            ComponentPalette::neutral_dark(),
        )
        .variant(TabsVariant::Underlined)
        .accent(accent)
        .gap(16.0)
        .id("settings")
        .render()
    }

    fn underlined_frame(tree: &mut UiTree) -> crate::Frame {
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
        // Taller than the strip's 28: the lines must stay under the tabs.
        tree.layout(Size::new(600.0, 40.0))
    }

    fn fills(frame: &crate::Frame, of: Color) -> Vec<crate::Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == of => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn underlined_tabs_draw_the_accent_under_the_active_tab_over_a_hairline() {
        let palette = ComponentPalette::neutral_dark();
        let accent = Color::rgb(25, 170, 216);
        let mut tree = UiTree::new(underlined("roles", accent));
        let frame = underlined_frame(&mut tree);
        let roles = frame
            .regions
            .iter()
            .find(|region| region.id == "roles")
            .unwrap()
            .rect;
        assert_eq!(
            fills(&frame, accent),
            [crate::Rect::new(roles.x, 26.0, roles.width, 2.0)]
        );
        // The hairline runs the width it is given, under the underline.
        assert_eq!(
            fills(&frame, palette.border),
            [crate::Rect::new(0.0, 27.0, 600.0, 1.0)]
        );
        assert!(
            fills(&frame, palette.surface).is_empty() && fills(&frame, palette.selected).is_empty()
        );
        let ink = |name: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == name => Some(*color),
                _ => None,
            })
        };
        assert_eq!(ink("ROLES"), Some(accent));
        assert_eq!(ink("GENERAL"), Some(palette.muted));
        assert_eq!(ink("Audit"), Some(palette.muted));
        // The disabled label sits at the height of the enabled ones.
        let top = |name: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == name => Some(position.y),
                _ => None,
            })
        };
        assert!(top("Audit").is_some());
        assert_eq!(top("Audit"), top("GENERAL"));
        // Without an accent the active tab takes the palette's text, as segmented.
        let plain = Tabs::new(vec![Tab::new("a", "A", 60.0).active(true)], palette)
            .variant(TabsVariant::Underlined)
            .render();
        let mut tree = UiTree::new(plain);
        assert_eq!(fills(&underlined_frame(&mut tree), palette.text).len(), 1);
    }

    #[test]
    fn underlined_tabs_keep_the_segmented_geometry_and_hover_ink() {
        let palette = ComponentPalette::neutral_dark();
        let items = || {
            vec![
                Tab::new("a", "A", 70.0).active(true),
                Tab::new("b", "B", 90.0),
            ]
        };
        let regions = |variant| {
            let tabs = Tabs::new(items(), palette)
                .variant(variant)
                .fill_width(300.0)
                .render();
            let mut tree = UiTree::new(Element::row(vec![tabs, Element::spacer().flex(1.0)]));
            let frame = underlined_frame(&mut tree);
            frame
                .regions
                .iter()
                .map(|region| (region.id.clone(), region.rect))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            regions(TabsVariant::Underlined),
            regions(TabsVariant::Segmented)
        );
        let ElementKind::Stack(layers) = Tabs::new(items(), palette)
            .variant(TabsVariant::Underlined)
            .accent(palette.border)
            .render()
            .kind
        else {
            panic!("Underlined tabs must stack the strip over its lines");
        };
        let ElementKind::TabStrip { children, .. } = &layers[2].kind else {
            panic!("The tabs must be the top layer");
        };
        assert_eq!(
            children[0].style.hover_foreground,
            Some(lighter(palette.border, 12))
        );
        assert_eq!(children[1].style.hover_foreground, Some(palette.text));
        assert_eq!(children[0].style.hover_background, None);
    }

    #[test]
    fn underlined_tabs_report_clicks_and_keys_and_the_underline_follows() {
        let accent = Color::rgb(25, 170, 216);
        let mut tree = UiTree::new(underlined("general", accent));
        let frame = underlined_frame(&mut tree);
        let emoji = frame
            .regions
            .iter()
            .find(|region| region.id == "emoji")
            .unwrap()
            .rect;
        let center = Point::new(emoji.x + emoji.width / 2.0, emoji.y + emoji.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked { id: "emoji".into() }]
        );
        assert!(tree.focus("general"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked { id: "roles".into() }]
        );
        // The disabled tab is stepped over: from the last enabled one, Right wraps.
        assert_eq!(
            tree.handle_event(key(crate::Key::End)),
            vec![UiAction::Clicked { id: "emoji".into() }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked {
                id: "general".into()
            }]
        );
        assert!(!underlined_frame(&mut tree)
            .regions
            .iter()
            .any(|region| region.id == "audit"));
        tree.set_root(underlined("emoji", accent));
        let frame = underlined_frame(&mut tree);
        assert_eq!(
            fills(&frame, accent),
            [crate::Rect::new(emoji.x, 26.0, emoji.width, 2.0)]
        );
        // A disabled tab marked active draws no underline.
        tree.set_root(underlined("audit", accent));
        assert!(fills(&underlined_frame(&mut tree), accent).is_empty());
    }

    #[test]
    fn tabs_gap_ignores_bad_input_and_underlined_tabs_fill_a_narrow_width() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = || Tabs::new(vec![Tab::new("a", "A", 60.0)], palette);
        assert_eq!(tabs().gap(f32::NAN).gap, TAB_GAP);
        assert_eq!(tabs().gap(f32::INFINITY).gap, TAB_GAP);
        assert_eq!(tabs().gap(-4.0).gap, 0.0);
        let accent = Color::rgb(25, 170, 216);
        let narrow = Tabs::new(
            vec![
                Tab::new("a", "Alpha", 80.0).active(true),
                Tab::new("b", "Beta", 80.0),
            ],
            palette,
        )
        .variant(TabsVariant::Underlined)
        .accent(accent)
        .gap(16.0)
        .fill_width(120.0)
        .render();
        let mut tree = UiTree::new(narrow);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(120.0, 28.0));
        assert_eq!(
            fills(&frame, palette.border),
            [crate::Rect::new(0.0, 27.0, 120.0, 1.0)]
        );
        // Nothing escapes the width: neither a tab nor the underline.
        let inside = |rect: &crate::Rect| rect.x >= 0.0 && rect.x + rect.width <= 120.0;
        assert_eq!(frame.regions.len(), 2);
        assert!(frame.regions.iter().all(|region| inside(&region.rect)));
        let underline = fills(&frame, accent);
        assert_eq!(underline.len(), 1);
        assert!(inside(&underline[0]));
    }

    #[test]
    fn capsule_tabs_are_concentric_stadiums_with_labels_of_their_own_size() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |radius: Option<f32>| {
            let tab = |index: usize| Tab::new(format!("t{index}"), "Tab", 90.0).active(index == 0);
            let tabs = Tabs::new(vec![tab(0), tab(1)], palette).height(40.0).font_size(15.0);
            match radius { Some(radius) => tabs.radius(radius), None => tabs }.render()
        };
        let painted = |radius| {
            let mut tree = UiTree::new(Element::column(vec![tabs(radius)]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(300.0, 60.0));
            let fill = |color| frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color: fill, radius } if *fill == color => Some((rect.height, *radius)),
                _ => None,
            }).unwrap();
            let size = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { font_size, .. } => Some(*font_size),
                _ => None,
            });
            (fill(palette.surface), fill(palette.selected), size)
        };
        // A radius of half the height or more: both ends are semicircles.
        assert_eq!(painted(Some(100.0)), ((44.0, 22.0), (40.0, 20.0), Some(15.0)));
        // A smaller one keeps the track concentric, 2 rounder.
        assert_eq!(painted(Some(10.0)), ((44.0, 12.0), (40.0, 10.0), Some(15.0)));
        assert_eq!(painted(None), ((44.0, surface_radius()), (40.0, control_radius()), Some(15.0)));
        let plain = Tabs::new(Vec::new(), palette);
        assert_eq!((plain.radius, plain.font_size), (None, TAB_FONT_SIZE));
        let ignored = Tabs::new(Vec::new(), palette).radius(f32::NAN).font_size(0.0).font_size(f32::INFINITY);
        assert_eq!((ignored.radius, ignored.font_size), (None, TAB_FONT_SIZE));
        assert_eq!(Tabs::new(Vec::new(), palette).radius(-3.0).radius, Some(0.0));
    }

    #[test]
    fn a_tab_selection_sets_off_on_the_first_frame_after_the_change() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |chosen: usize| {
            let tab = |index: usize| Tab::new(format!("t{index}"), "Tab", 80.0).active(index == chosen);
            Tabs::new(vec![tab(0), tab(1), tab(2)], palette).id("tabs").render()
        };
        let size = Size::new(300.0, 40.0);
        let start = web_time::Instant::now() - web_time::Duration::from_secs(10);
        let frame = web_time::Duration::from_micros(16_667);
        let mut tree = UiTree::new(tabs(0));
        let from = fills(&tree.layout_at(size, start), palette.selected)[0].x;
        tree.set_root(tabs(2));
        assert_eq!(fills(&tree.layout_at(size, start), palette.selected)[0].x, from);
        let to = {
            let mut rest = UiTree::new(tabs(2));
            fills(&rest.layout(size), palette.selected)[0].x
        };
        // An ease-out over the move token covers a quarter of the way in one
        // frame; a spring from standstill covered under a seventh.
        let first = fills(&tree.layout_at(size, start + frame), palette.selected)[0].x;
        assert!((first - from) / (to - from) > 0.25, "{first} between {from} and {to}");
        let settled = start + web_time::Duration::from_secs(2);
        assert_eq!(fills(&tree.layout_at(size, settled), palette.selected)[0].x, to);
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
            alt: false,
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
            alt: false,
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
            alt: false,
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

    #[test]
    fn a_disabled_link_is_muted_text_with_no_target_in_the_place_of_the_enabled_one() {
        let palette = ComponentPalette::neutral_dark();
        let tree_of = |enabled| {
            let link = Link::new("docs", "Read the documentation", palette)
                .ink(palette.text)
                .font_size(11.0)
                .enabled(enabled);
            let mut tree = UiTree::new(Element::column(vec![
                Element::row(vec![link.render(), Element::text("after")]).gap(4.0),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            tree
        };
        let texts = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, color, .. } => {
                        Some((text.clone(), *position, *color))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let size = Size::new(400.0, 80.0);
        let mut tree = tree_of(true);
        let enabled = tree.layout(size);
        let [region] = enabled.regions.as_slice() else {
            panic!("one region, got {:?}", enabled.regions);
        };
        let point = Point::new(region.rect.x + 4.0, region.rect.y + 4.0);

        let mut tree = tree_of(false);
        let disabled = tree.layout(size);
        assert!(disabled.regions.is_empty());
        tree.handle_event(UiEvent::PointerMove(point));
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(!tree.focus("docs"));
        let enter = UiEvent::Key { key: crate::Key::Enter, shift: false, ctrl: false, alt: false };
        assert!(tree.handle_event(enter).is_empty());
        // Hovered, it is still the same muted line with no underline.
        assert_eq!(tree.layout(size).commands, disabled.commands);
        let (was, now) = (texts(&enabled), texts(&disabled));
        assert_eq!((was[0].2, now[0].2), (palette.text, palette.muted));
        // The label and what follows it stay where they were.
        let places = |lines: &[(String, Point, Color)]| {
            lines.iter().map(|line| (line.0.clone(), line.1)).collect::<Vec<_>>()
        };
        assert_eq!(places(&now), places(&was));
    }
}
