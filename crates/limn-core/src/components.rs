//! Composable, controlled controls built from the same backend-neutral tree.
//! State such as the selected tab or whether a menu is open belongs to the
//! caller; stable action IDs let [`crate::UiTree`] retain focus and hover while
//! these controls are rebuilt on every frame.

use crate::{
    Access, Color, Element, ElementKind, FontFamily, Icon, InlineImage, RasterImage, TooltipItem,
    RadiusRole, Size, TextSpan, TooltipOptions, TooltipPlacement, TooltipSection, TooltipShape, TooltipStyle, ICON_SIZE,
};
use crate::Role;

/// A component can render to an ordinary [`Element`] and be used anywhere in
/// the tree. App-specific views and generic controls follow the same contract.
pub trait Component {
    /// Consume the component and build the element that draws it.
    fn render(self) -> Element;
}

#[doc(hidden)]
pub fn hover_ms() -> u16 {
    crate::tokens().motion.hover_ms
}
// Discrete corners for editor chrome. Circular controls (for example the
// switch thumb or a host's avatar) opt into their own semantic shape.
#[doc(hidden)]
pub fn control_radius() -> f32 {
    crate::tokens().radius.control
}

#[doc(hidden)]
pub fn surface_radius() -> f32 {
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
    content: Option<Box<Element>>,
}

impl Tooltip {
    /// A hint on `anchor` with this title. `anchor_id` is the ID of the control
    /// the anchor is, or the one a passive anchor's hover region takes. It
    /// opens above the anchor, is at most 360 pixels wide and starts from
    /// the tokens' `tooltip_options`.
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
            options: crate::tokens().tooltip_options,
            background: None,
            content: None,
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

    /// Show a keyboard shortcut in the hint, as key caps, or as muted text
    /// under `ShortcutStyle::Text`.
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
    /// milliseconds: the tokens' `tooltip_options` unless set (350 by
    /// default), and 0 shows it as soon as the pointer is over it, for a
    /// strip of marks read by moving along it. Keyboard focus shows it at
    /// once either way.
    pub fn delay_ms(mut self, delay: u16) -> Self {
        self.options.delay_ms = delay;
        self
    }

    /// The title's text size in logical pixels, the tokens' unless set (12
    /// by default). A one-line
    /// hint grows by what a larger title adds. A size that is not finite or
    /// not above zero is ignored.
    pub fn title_font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.options.title_font_size = size;
        }
        self
    }

    /// The title's face, the tokens' unless set (the regular one by
    /// default).
    pub fn title_font_family(mut self, family: FontFamily) -> Self {
        self.options.title_font_family = family;
        self
    }

    /// Whether a small pointer joins the hint to its anchor: the tokens'
    /// unless set, on by default.
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

    /// Add a picture in a box `size` large, such as a badge's artwork; see
    /// [`TooltipSection::Picture`]. A size that is not finite or not above
    /// zero adds nothing.
    pub fn picture(mut self, image: RasterImage, size: Size) -> Self {
        if size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0 {
            self.sections.push(TooltipSection::Picture { image, size });
        }
        self
    }

    /// Add lines one under the other, each in its span's size, face and
    /// ink; see [`TooltipSection::Lines`]. A hint with an empty title
    /// describes its anchor with these lines.
    pub fn lines(mut self, spans: impl IntoIterator<Item = TextSpan>) -> Self {
        let spans: Vec<_> = spans.into_iter().filter(|span| !span.text.trim().is_empty()).collect();
        if !spans.is_empty() {
            self.sections.push(TooltipSection::Lines(spans));
        }
        self
    }

    /// Center every line and block across the hint, as a card that shows a
    /// picture over what it means.
    pub fn centered(mut self) -> Self {
        self.options.centered = true;
        self
    }

    /// Draw this hint's body and pointer to these measures instead of the
    /// tokens', such as a roomier padding around a picture.
    pub fn shape(mut self, shape: TooltipShape) -> Self {
        self.options.shape = Some(shape);
        self
    }

    /// Lay the body out as `content`, an element of the caller's own, in
    /// place of the title, shortcut and sections. The body is sized to it,
    /// at most `max_width` wide, plus the padding of the shape (8 at the
    /// sides and 6 above and below without one), and keeps its fill, border,
    /// shadow, pointer, placement and entrance. The content is painted and
    /// nothing more: nothing in it can be clicked, focused or read, so the
    /// title is still what the platform is told.
    pub fn content(mut self, content: Element) -> Self {
        self.content = Some(Box::new(content));
        self
    }
}

impl Component for Tooltip {
    fn render(self) -> Element {
        // A hint is not seen by who cannot hover: its title describes the
        // control it is anchored to.
        // Without a title, its lines say what it says.
        let said = if self.title.trim().is_empty() {
            self.sections
                .iter()
                .filter_map(|section| match section {
                    TooltipSection::Lines(spans) => Some(spans.iter().map(|span| span.text.as_str())),
                    _ => None,
                })
                .flatten()
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            self.title.clone()
        };
        let anchor = if self.described && !said.trim().is_empty() {
            self.anchor.accessible(Access::hinted(said))
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
            content,
            ..
        } = &mut element.kind
        {
            *sections = self.sections;
            *max_width = self.max_width;
            *options = self.options;
            *content = self.content;
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
    content: Option<Box<Element>>,
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
            content: None,
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

    /// Lay the hint's body out as `content`, in place of its title,
    /// description and shortcut; see [`Tooltip::content`].
    pub fn content(mut self, content: Element) -> Self {
        self.content = Some(Box::new(content));
        self
    }

    /// Render the button without its hint, for example while the menu it
    /// opens is showing.
    pub fn suppressed(mut self, suppressed: bool) -> Self {
        self.suppressed = suppressed;
        self
    }

    // Wrap a rendered button, enabled or passive, in the hint.
    #[doc(hidden)]
    pub fn wrap(self, anchor: Element, id: String, button_palette: ComponentPalette) -> Element {
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
        tooltip.content = self.content;
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

    #[doc(hidden)]
    pub fn is_dark(self) -> bool {
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

    #[doc(hidden)]
    pub fn tooltip_style(self) -> TooltipStyle {
        if let Some(style) = crate::tokens().tooltip {
            return style;
        }
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

/// Shared button geometry in logical pixels: the hit box, the type and the
/// side its glyph is drawn at.
/// The radius is not part of it: it is a token, chosen with
/// [`Button::radius`]. A host keeps its own densities as consts of this type
/// and passes one with [`Button::sizing`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonSizing {
    /// Height of the button, and the width of an icon-only one.
    pub height: f32,
    /// Size of the label.
    pub font_size: f32,
    /// Room kept for a glyph and the side it is drawn at, from 8 to 128.
    pub icon_size: f32,
    /// Space between the button's edge and its content.
    pub horizontal_padding: f32,
    /// Space between the icons and the label.
    pub inner_spacing: f32,
}

impl ButtonSizing {
    // A profile is plain data from the host; a bad number stops here.
    fn sanitized(self) -> Self {
        let px = crate::px;
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
    image: Option<InlineImage>,
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
    family: FontFamily,
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
            family: FontFamily::Sans,
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
            .or_else(|| match &self.image {
                Some(InlineImage::Raster(image)) if label.is_empty() => Some(image.alt().to_owned()),
                _ => None,
            });
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
        self.image = Some(InlineImage::Raster(image));
        self
    }

    /// Put the host-fed surface `id` before the label, as [`Self::image`]
    /// puts a picture: for an animation the host presents on that surface,
    /// such as an animated custom emoji, fitted inside the slot. The
    /// surface is named by the button's label; a button of a surface alone
    /// needs an [`accessibility_label`](Self::accessibility_label).
    pub fn surface(mut self, id: impl Into<String>) -> Self {
        self.image = Some(InlineImage::Surface(id.into()));
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

    /// Set the label in `family`, such as a bold face for a host whose
    /// buttons speak louder than its text.
    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    /// Round the ends fully: an icon button becomes a circle and a labelled
    /// one a pill, at any size. It takes precedence over `radius`.
    pub fn round(mut self) -> Self {
        self.round = true;
        self
    }

    #[doc(hidden)]
    pub fn colors(&self) -> (Color, Color, Color, Color, bool) {
        let palette = self.palette;
        let (base, hover, pressed, ink) = match self.variant {
            ButtonVariant::Default => (
                palette.text,
                darker(palette.text, 16),
                darker(palette.text, 30),
                palette.surface,
            ),
            // Hover and press step away from the surface: a lighter step on a
            // light palette would fade the fill into the card.
            ButtonVariant::Secondary if palette.is_dark() => (
                palette.raised,
                lighter(palette.raised, 10),
                lighter(palette.raised, 18),
                palette.text,
            ),
            ButtonVariant::Secondary => (
                palette.raised,
                darker(palette.raised, 10),
                darker(palette.raised, 18),
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

/// The sizing's icon side is the glyph's, not only its box's.
fn icon_side(glyph: Element, side: f32) -> Element {
    if side > ICON_SIZE {
        glyph.large_icon(side)
    } else {
        glyph.small_icon(side)
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
            } else if let Some(image) = &image {
                let name = self.accessibility_label.as_deref().unwrap_or(&label);
                Some(center_icon(image.element(name).size(Some(ICON_SIZE), Some(ICON_SIZE)).centered_x()))
            } else {
                icon.map(|icon| {
                    let glyph = icon_side(Element::icon(icon).foreground(ink), sizing.icon_size);
                    center_icon(glyph.size(Some(sizing.icon_size), Some(sizing.icon_size)))
                })
            };
            let label = Element::truncated_text(label)
                .foreground(ink)
                .font_family(self.family)
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
            icon_side(
                Element::icon_button(self.action_id.clone(), icon, label),
                sizing.icon_size,
            )
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
            .font_family(self.family)
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
        let element = if self.menu || crate::tokens().shortcut_style == crate::ShortcutStyle::Text {
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

#[doc(hidden)]
pub const fn lighter(color: Color, amount: u8) -> Color {
    Color::rgba(
        color.r.saturating_add(amount),
        color.g.saturating_add(amount),
        color.b.saturating_add(amount),
        color.a,
    )
}

#[doc(hidden)]
pub const fn darker(color: Color, amount: u8) -> Color {
    Color::rgba(
        color.r.saturating_sub(amount),
        color.g.saturating_sub(amount),
        color.b.saturating_sub(amount),
        color.a,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, ElementKind, Point, PopoverPlacement, Size, UiAction, UiEvent, UiTree};

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
    fn a_hint_without_a_title_describes_its_control_with_its_lines() {
        let palette = ComponentPalette::neutral_dark();
        let line = |text: &str| TextSpan::new(text, Color::WHITE, 14.0, FontFamily::SansMedium);
        let hinted = Tooltip::new(Button::new("badge", "Badge", palette).render(), "badge", "", palette)
            .lines([line("Account age"), line(" "), line("Bloom")]);
        let said = access_node(hinted.render(), "badge");
        assert_eq!(said.description.as_deref(), Some("Account age\nBloom"));
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
    fn secondary_hover_and_press_step_away_from_the_surface_on_light_and_dark_palettes() {
        let dark = ComponentPalette::neutral_dark();
        let light = ComponentPalette {
            surface: Color::WHITE,
            raised: Color::rgb(242, 242, 242),
            border: Color::rgb(221, 221, 221),
            text: Color::rgb(25, 25, 25),
            ..dark
        };
        for (palette, hover, pressed) in [
            (dark, Color::rgb(48, 48, 48), Color::rgb(56, 56, 56)),
            (light, Color::rgb(232, 232, 232), Color::rgb(224, 224, 224)),
        ] {
            let style = Button::new("a", "A", palette)
                .variant(ButtonVariant::Secondary)
                .render()
                .style;
            assert_eq!(style.background, Some(palette.raised));
            assert_eq!(style.hover_background, Some(hover));
            assert_eq!(style.pressed_background, Some(pressed));
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
    fn an_icon_button_draws_its_glyph_at_the_sizing_side_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let glyph = |button: IconButton| {
            let frame =
                UiTree::new(Element::row(vec![button.render()])).layout(Size::new(100.0, 32.0));
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .expect("glyph")
        };
        for (asked, drawn) in [(20.0, 20.0), (ICON_SIZE, ICON_SIZE), (12.0, 12.0)] {
            let button = || {
                IconButton::new("action", Icon::Add, palette)
                    .variant(ButtonVariant::Ghost)
                    .sizing(ButtonSizing {
                        height: 32.0,
                        font_size: 14.0,
                        icon_size: asked,
                        horizontal_padding: 9.0,
                        inner_spacing: 6.0,
                    })
            };
            let (enabled, disabled) = (glyph(button()), glyph(button().disabled()));
            assert_eq!(
                (enabled.width, enabled.height),
                (drawn, drawn),
                "asked {asked}"
            );
            assert_eq!(enabled, disabled, "asked {asked}");
            let inset = (32.0 - drawn) / 2.0;
            assert_eq!((enabled.x, enabled.y), (inset, inset));
        }
        // A preset keeps the standard glyph.
        let preset =
            glyph(IconButton::new("action", Icon::Add, palette).button_size(ButtonSize::IconSmall));
        assert_eq!((preset.width, preset.height), (ICON_SIZE, ICON_SIZE));
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
                DrawCommand::Icon { icon, rect, tint, .. } => {
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
    fn a_surface_takes_the_picture_slot_enabled_or_not_fitted_and_named_by_the_label() {
        let palette = ComponentPalette::neutral_dark();
        let surfaced = |frame: &crate::Frame| {
            let found: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Surface { rect, id, label, fit, .. } => Some((*rect, id.clone(), label.clone(), *fit)),
                    _ => None,
                })
                .collect();
            assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { .. })), "no glyph");
            assert_eq!(found.len(), 1, "one surface");
            found[0].clone()
        };
        let button = || Button::new("join", "Join", palette).surface("animation:party").variant(ButtonVariant::Raised);
        for enabled in [true, false] {
            let mut tree = UiTree::new(Element::row(vec![button().enabled(enabled).render()]));
            let frame = tree.layout(Size::new(300.0, 32.0));
            let (rect, id, label, fit) = surfaced(&frame);
            assert_eq!((rect.width, rect.height), (ICON_SIZE, ICON_SIZE));
            assert_eq!((id.as_str(), label.as_str(), fit), ("animation:party", "Join", crate::SurfaceFit::Contain));
            if enabled {
                tree.handle_event(UiEvent::PointerDown(Point::new(rect.x + 1.0, rect.y + 1.0)));
                assert_eq!(
                    tree.handle_event(UiEvent::PointerUp(Point::new(rect.x + 1.0, rect.y + 1.0))),
                    vec![UiAction::Clicked { id: "join".into() }]
                );
            }
        }
        // A surface alone is as square as a picture alone and is called
        // what the host says.
        let alone = || Button::new("party", "", palette).surface("animation:party").accessibility_label("Party");
        assert_eq!(access_node(alone().render(), "party").name, "Party");
        let frame = UiTree::new(Element::row(vec![alone().render()])).layout(Size::new(100.0, 32.0));
        assert_eq!((frame.regions[0].rect.width, frame.regions[0].rect.height), (32.0, 32.0));
        assert_eq!(surfaced(&frame).0.width, ICON_SIZE);
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
}
