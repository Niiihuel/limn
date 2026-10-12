use crate::{Color, Point, RasterImage, ScrollMotion, ScrollbarAppearance, Size};
use std::sync::Arc;

pub const DEFAULT_FONT_SIZE: f32 = 14.0;
/// Canonical visual size of every chrome icon, in logical pixels.
/// The host rasterizes this size at the current display scale.
pub const ICON_SIZE: f32 = 16.0;

/// Preferred side of an anchored floating surface. The layout engine flips
/// vertically when the preferred side would leave the viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopoverPlacement {
    BottomStart,
    BottomEnd,
    TopStart,
    TopEnd,
    RightStart,
    LeftStart,
    Center,
}

/// Side of its target a coach mark sits on. The engine takes the opposite
/// side, then the other axis, when the preferred one has no room.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoachSide {
    Above,
    Below,
    Left,
    Right,
}

/// Where a coach mark sits along its target's edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoachAlign {
    Start,
    Center,
    End,
}

/// Ties a popover to controls that already exist in the tree, by their IDs,
/// instead of to the element it wraps. Built by `CoachMark`.
#[derive(Clone, Debug, PartialEq)]
pub struct CoachAnchor {
    /// The bubble points at the union of the regions with these IDs.
    pub(crate) targets: Vec<String>,
    pub(crate) side: CoachSide,
    pub(crate) align: CoachAlign,
    /// Names the step: when it changes the bubble travels to the new target.
    pub(crate) key: String,
    /// The bubble's fill, shared by its caret and the ring on the target.
    pub(crate) fill: Color,
    /// Dims everything but the target.
    pub(crate) dim: Option<Color>,
    /// Actions reported by the Left and Right arrow keys.
    pub(crate) back: Option<String>,
    pub(crate) next: Option<String>,
}

/// Preferred side of a non-interactive tooltip. The engine flips to the
/// opposite side when the preferred one would leave the viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TooltipPlacement {
    Above,
    Below,
    Left,
    Right,
}

/// Read-only blocks beneath a tooltip's title. Paragraphs wrap to the
/// available width; separators divide related pieces of information.
#[derive(Clone, Debug, PartialEq)]
pub enum TooltipSection {
    Description(String),
    IconRow {
        label: String,
        items: Vec<TooltipItem>,
    },
    Separator,
    Footer(String),
}

/// A labeled, passive item in a tooltip row. Icons keep their canonical size.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipItem {
    pub label: String,
    pub icon: Option<Icon>,
    pub icon_only: bool,
}

impl TooltipItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            icon_only: false,
        }
    }

    /// Draw only the icon, so a row of several stays on one line. The label
    /// still names the item for hosts that describe it elsewhere, and is
    /// drawn after all when the icon does not fit.
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipStyle {
    pub background: Color,
    pub border: Color,
    pub keycap: Color,
    pub text: Color,
    pub muted: Color,
    pub shadow: Color,
}

/// Semantic icon names. A host chooses the concrete artwork for each name.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Icon {
    PanelLeft,
    PanelLeftOpen,
    PanelLeftClose,
    PanelBottom,
    PanelRight,
    PanelRightOpen,
    PanelRightClose,
    Add,
    /// A horizontal dash: remove, collapse or unstage.
    Minus,
    Microphone,
    Text,
    Image,
    Audio,
    Video,
    File,
    /// Embedded file/language/folder artwork; hosts preserve its native colors.
    FileType(crate::file_icons::FileIcon),
    Infinity,
    Paperclip,
    Target,
    Plan,
    Brain,
    Check,
    InfoCircle,
    CheckCircle,
    WarningTriangle,
    ErrorCircle,
    Settings,
    Github,
    /// Artwork the application supplies: the source of an SVG drawn with
    /// `currentColor`. It is fitted to the icon's square without distortion
    /// and tinted like any other icon, so a brand mark or a product glyph
    /// follows the theme.
    Custom(&'static str),
    Chat,
    Folder,
    Laptop,
    Globe,
    GitBranch,
    Grid,
    Palette,
    Users,
    Puzzle,
    Book,
    Server,
    Bolt,
    ChartBar,
    Keyboard,
    Hash,
    Code,
    Terminal,
    ArrowUp,
    Send,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ChevronDown,
    ChevronRight,
    Search,
    Stop,
    /// A solid rounded square: stop, on a filled button.
    StopFill,
    Play,
    Pause,
    History,
    Close,
    CloseBold,
    CloseCircle,
    Refresh,
    Menu,
    More,
    Pin,
    Archive,
    ArchiveUp,
    Trash,
    Edit,
    /// Two overlapping sheets: copy to the clipboard.
    Copy,
    Unread,
    Filter,
    Sort,
    Collapse,
    Expand,
    /// Two arrows closing on a line: making something take less room.
    Compact,
    /// Four corners: showing something over the whole window.
    Fullscreen,
    Clock,
    /// Filled counterparts, for the chosen item of a navigation list.
    FolderFill,
    PlayFill,
    GridFill,
    SettingsFill,
    PaletteFill,
    LaptopFill,
    TerminalFill,
    KeyboardFill,
    ChatFill,
    UsersFill,
    PuzzleFill,
    ServerFill,
    BookFill,
    ClockFill,
    BoltFill,
    ArchiveFill,
    ChartBarFill,
    Bell,
    BellFill,
    BellSlash,
}

impl Icon {
    /// The solid form of an icon, for the active or chosen item of a list.
    /// An icon without one is returned as it is.
    pub fn filled(self) -> Self {
        match self {
            Self::PanelLeft => Self::PanelLeftOpen,
            Self::PanelRight => Self::PanelRightOpen,
            Self::Bell => Self::BellFill,
            Self::Folder => Self::FolderFill,
            Self::Play => Self::PlayFill,
            Self::Grid => Self::GridFill,
            Self::Settings => Self::SettingsFill,
            Self::Palette => Self::PaletteFill,
            Self::Laptop => Self::LaptopFill,
            Self::Terminal => Self::TerminalFill,
            Self::Keyboard => Self::KeyboardFill,
            Self::Chat => Self::ChatFill,
            Self::Users => Self::UsersFill,
            Self::Puzzle => Self::PuzzleFill,
            Self::Server => Self::ServerFill,
            Self::Book => Self::BookFill,
            Self::Clock => Self::ClockFill,
            Self::Bolt => Self::BoltFill,
            Self::Archive => Self::ArchiveFill,
            Self::ChartBar => Self::ChartBarFill,
            other => other,
        }
    }
}

/// Outline forms of the passive loading indicator.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SpinnerVariant {
    /// An incomplete circular stroke, similar to a rotating LoaderCircle.
    #[default]
    Circle,
    /// Eight short radial strokes, similar to a rotating Loader.
    Spokes,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FontFamily {
    #[default]
    Monospace,
    Sans,
    /// Sans slanted by the host. It measures exactly as [`Self::Sans`].
    SansItalic,
    /// The medium and bold faces of the sans family: real faces the host
    /// provides, each measured as itself. A host without one paints Sans.
    SansMedium,
    SansBold,
}

/// One inline style run in a paragraph. Width is measured by the same host
/// text engine that paints it, so changing family or size does not require
/// callers to guess a character advance.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub font_family: FontFamily,
    pub background: Option<Color>,
    /// Optional inline action. Wrapped fragments share one keyboard target and
    /// dispatch `UiAction::Clicked` using their exact painted text bounds.
    pub action: Option<String>,
    /// Decorative leading glyph. It contributes width, never copyable text.
    pub icon: Option<Icon>,
}

impl TextSpan {
    pub fn new(
        text: impl Into<String>,
        color: Color,
        font_size: f32,
        font_family: FontFamily,
    ) -> Self {
        Self {
            text: text.into(),
            color,
            font_size: if font_size.is_finite() && font_size > 0.0 {
                font_size
            } else {
                DEFAULT_FONT_SIZE
            },
            font_family,
            background: None,
            action: None,
            icon: None,
        }
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// Paint-only emphasis in editable text, using Unicode character offsets.
/// Geometry, selection, clipboard and the stored value are unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
    pub range: std::ops::Range<usize>,
    pub color: Color,
    /// Slant the range, as for text that is not committed yet.
    pub italic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    pub color: Color,
    pub width: f32,
}

/// Independent corner radii for backgrounds, solid borders and keyboard focus.
/// They affect paint only; child clipping and hit regions remain rectangular.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    pub const fn same(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    pub const fn bottom(radius: f32) -> Self {
        Self {
            top_left: 0.0,
            top_right: 0.0,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    fn normalized(self) -> Self {
        let clean = |value: f32| {
            if value.is_finite() {
                value.max(0.0)
            } else {
                0.0
            }
        };
        Self {
            top_left: clean(self.top_left),
            top_right: clean(self.top_right),
            bottom_right: clean(self.bottom_right),
            bottom_left: clean(self.bottom_left),
        }
    }
}

/// Visual depth for a surface. Values are logical pixels and never affect
/// layout or pointer hit bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    pub color: Color,
    pub offset: Point,
    pub blur: f32,
    pub spread: f32,
}

/// Paint-only selection marker for a reusable tab strip. The group ID remains
/// stable when children are rebuilt or reordered.
#[derive(Clone, Debug)]
pub struct TabSelection {
    pub group_id: String,
    pub selected_id: String,
    pub color: Color,
    pub radius: f32,
    pub animation_ms: u16,
}

impl BoxShadow {
    pub fn new(color: Color, offset: Point, blur: f32, spread: f32) -> Self {
        Self {
            color,
            offset: Point::new(
                if offset.x.is_finite() { offset.x } else { 0.0 },
                if offset.y.is_finite() { offset.y } else { 0.0 },
            ),
            blur: if blur.is_finite() {
                blur.clamp(0.0, 64.0)
            } else {
                0.0
            },
            spread: if spread.is_finite() {
                spread.clamp(0.0, 32.0)
            } else {
                0.0
            },
        }
    }
}

/// The part an element plays in revealing controls under the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum HoverReveal {
    #[default]
    None,
    /// The pointer anywhere over this element reveals what it contains.
    Region,
    /// Painted only while revealed; `presence` holds its ID.
    Revealed,
}

/// The IDs under which the tree keeps an element's ink and a column's
/// children across rebuilds.
#[derive(Clone, Debug, Default)]
pub(crate) struct Kept {
    pub(crate) foreground: Option<String>,
    pub(crate) leaving: Option<String>,
}

/// A transient visual description. Interaction state belongs to [`crate::UiTree`].
#[derive(Clone, Debug)]
pub struct Element {
    pub kind: ElementKind,
    pub style: Style,
    /// Another action whose pointer hover should also highlight this element.
    /// Useful when a row contains a more specific action such as its menu.
    pub hover_peer: Option<String>,
    /// ID under which this element fades in when it is first placed, and
    /// whether its last paint is left behind to fade out once it is gone.
    pub presence: Option<(String, bool)>,
    /// ID and content key under which this element rises into place when the
    /// key changes, while its previous paint moves up and fades. Boxed: few
    /// elements have one, and every element pays for the field's size.
    pub replacement: Option<Box<(String, String)>>,
    /// A paint-only surface keyed independently from the actions it follows.
    /// It adds no pointer target or keyboard focus stop.
    /// Boxed: few elements have one, and every element pays for the field.
    pub hover_surface: Option<Box<(String, Vec<String>)>>,
    /// Names this passive child of a tab strip as a place where the strip's
    /// selection indicator can rest, such as either end of a switch.
    pub indicator_slot: Option<String>,
    /// Optional action dispatched by an interactive element whose focus ID
    /// must remain distinct from another control with the same action.
    pub action_alias: Option<String>,
    pub(crate) menu_secondary_action: bool,
    pub(crate) draggable: bool,
    pub(crate) sticky_top: Option<StickyTop>,
    /// Set by [`Element::animate_height`]: the height follows `presence`'s
    /// ID, and whether it also opens from zero. One byte of existing padding.
    pub(crate) animate_height: Option<bool>,
    /// Set by [`Element::animate_foreground`] and [`Element::keep_leaving`].
    /// Boxed together: few elements have either, and every element pays for
    /// the field in the caches that hold built trees.
    pub(crate) kept: Option<Box<Kept>>,
    /// Set by [`Element::hover_region`] and [`Element::reveal_on_hover`];
    /// the latter keeps its ID in `presence`. One byte of existing padding.
    pub(crate) hover_reveal: HoverReveal,
    pub(crate) drop_target: bool,
    /// Arrow-key navigation for an inline menu. Its first focusable child is
    /// the trigger; subsequent buttons are options. This does not trap Tab.
    pub menu_navigation: bool,
    pub(crate) context_menu: Option<crate::context_menu::ContextMenuDefinition>,
}

/// A fixed-height, lazily built list. The row factory is called only for rows
/// intersecting the viewport (and for a row reached through keyboard focus).
/// Its stable `id` retains the scroll position across tree rebuilds.
#[derive(Clone)]
pub struct UniformList {
    pub id: String,
    pub row_count: usize,
    pub row_height: f32,
    pub build_row: Arc<dyn Fn(usize) -> Element + Send + Sync>,
    pub appearance: ScrollbarAppearance,
    pub motion: ScrollMotion,
}

impl std::fmt::Debug for UniformList {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UniformList")
            .field("id", &self.id)
            .field("row_count", &self.row_count)
            .field("row_height", &self.row_height)
            .finish_non_exhaustive()
    }
}

/// A field of equal cells shared out among parts, read in rows from the top
/// left: each part takes as many cells as its `cells`, one part after
/// another, and the rest stay `empty`. UiTree keeps each part's count by
/// stable ID and moves it to a new one, so a part that shrinks gives its
/// cells up from its end and the parts after it follow. With `focus`, the
/// other parts recede towards `empty`.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitGrid {
    pub id: String,
    pub columns: u16,
    pub rows: u16,
    /// Side of a cell and the space between two.
    pub cell: f32,
    pub gap: f32,
    pub radius: f32,
    pub parts: Vec<UnitGridPart>,
    pub empty: Color,
    /// Index of the part that stands out, if one does.
    pub focus: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnitGridPart {
    /// Names the part across rebuilds, whatever its position.
    pub key: String,
    pub cells: f32,
    pub color: Color,
}

impl UnitGrid {
    pub(crate) fn size(&self) -> Size {
        let side = |count: u16| {
            f32::from(count) * self.cell + f32::from(count.saturating_sub(1)) * self.gap
        };
        Size::new(side(self.columns), side(self.rows))
    }
}

#[derive(Clone, Debug)]
pub enum ElementKind {
    Column(Vec<Element>),
    Row(Vec<Element>),
    /// A row of related, enabled tab actions. Arrow keys move between these
    /// actions and activate the newly focused tab.
    TabStrip {
        children: Vec<Element>,
        action_ids: Vec<String>,
        selection: Option<TabSelection>,
    },
    Panel(Box<Element>),
    /// One interactive surface whose children can contain more specific
    /// controls. Child hit regions take precedence over this row's region.
    Clickable {
        id: String,
        child: Box<Element>,
    },
    /// Paints a floating subtree above the ordinary scene. It shares the
    /// base element's bounds, but may escape clips created by ancestors.
    Overlay {
        base: Box<Element>,
        floating: Box<Element>,
    },
    /// A floating surface tied to its anchor's actual laid-out rectangle.
    /// A transparent dismissal region is inserted behind the surface.
    Popover {
        anchor: Box<Element>,
        floating: Box<Element>,
        dismiss_id: String,
        placement: PopoverPlacement,
        gap: f32,
        animation_ms: u16,
        close_animation_ms: Option<u16>,
        open_scale: f32,
        view_key: Option<String>,
        hover_open: bool,
        backdrop: bool,
        /// Whether this surface owns menu focus and keyboard navigation.
        focus_scope: bool,
        backdrop_dismiss: bool,
        scrim: Option<Color>,
        initial_focus: Option<String>,
        return_focus: Option<String>,
        /// Anchors the surface to existing controls instead of `anchor`.
        coach: Option<Box<CoachAnchor>>,
    },
    /// A transient, non-interactive overlay whose anchor remains in normal
    /// layout. The `anchor_id` identifies an action or a passive hover target.
    Tooltip {
        anchor: Box<Element>,
        anchor_id: String,
        title: String,
        shortcut: Option<String>,
        sections: Vec<TooltipSection>,
        max_width: f32,
        placement: TooltipPlacement,
        colors: TooltipStyle,
    },
    Scroll {
        id: String,
        child: Box<Element>,
        appearance: ScrollbarAppearance,
        motion: ScrollMotion,
    },
    /// A clipped horizontal viewport; selection changes reveal the target.
    HorizontalScroll {
        id: String,
        child: Box<Element>,
        reveal: Option<String>,
        /// Tab strips translate a vertical wheel into horizontal motion;
        /// two-dimensional document canvases leave it to their vertical parent.
        vertical_wheel: bool,
        drag_pan: bool,
    },
    UniformList(UniformList),
    CodeEditor(crate::CodeEditor),
    CodeDiff(crate::CodeDiff),
    /// A passive diagram at its natural size, measured with the tree's text.
    Flowchart(crate::Flowchart),
    SequenceDiagram(crate::SequenceDiagram),
    Text(String),
    /// Passive, individually measured key caps. A bounded shortcut is omitted
    /// as a group when it cannot fit; a partial chord would be misleading.
    KeyboardShortcut {
        keys: String,
        fill: Color,
        max_width: Option<f32>,
        menu: bool,
    },
    /// A single measured line for navigation labels. It uses an ellipsis when
    /// the available width is shorter than the painted text.
    TruncatedText(String),
    /// A measured single line whose overflowing right edge fades to transparent.
    /// The title remains unchanged; fading affects paint, never hit regions.
    FadedText {
        text: String,
        fade_width: f32,
    },
    /// Text with a paint-only moving highlight. Wrapped paragraphs remain
    /// selectable; a truncated label is passive and stays on one measured line.
    ShimmerText {
        id: String,
        text: String,
        highlight: Color,
        truncated: bool,
    },
    /// A paragraph containing inline styles and explicit line breaks.
    FormattedText(Vec<TextSpan>),
    /// Bounded encoded raster image. The host validates its digest and format
    /// before decoding and exposes `alt` to its accessibility backend.
    RasterImage(RasterImage),
    /// A rectangle whose pixels the host hands to the backend under `id`,
    /// outside the frame: a remote screen, a video, an embedded page.
    Surface {
        id: String,
        label: String,
    },
    Icon(Icon),
    Button {
        id: String,
        label: String,
        trailing_icon: Option<Icon>,
    },
    IconButton {
        id: String,
        icon: Icon,
        label: String,
        trailing_icon: Option<Icon>,
    },
    TextInput {
        id: String,
        initial_value: String,
        placeholder: String,
        persistent_placeholder: bool,
        leading_icon: Option<Icon>,
        leading_icon_tint: Option<Color>,
        shortcut: Option<String>,
        shortcut_fill: Option<Color>,
        password: bool,
    },
    TextArea {
        id: String,
        initial_value: String,
        placeholder: String,
        /// Maximum outer height when content determines the editor's height.
        auto_grow: Option<f32>,
        /// Keep CRLF/CR bytes in editable values instead of normalizing to LF.
        preserve_line_endings: bool,
        highlights: Vec<TextHighlight>,
    },
    /// Passive loading placeholder. UiTree animates its paint by stable ID.
    Skeleton {
        id: String,
        base: Color,
        highlight: Color,
    },
    /// Passive progress indicator. Its phase is retained by stable ID in UiTree.
    Spinner {
        id: String,
        color: Color,
        variant: SpinnerVariant,
    },
    /// Passive finite progress. Unknown capacity has no filled arc.
    ProgressRing {
        fraction: Option<f32>,
        track: Color,
        fill: Color,
    },
    /// An interactive reasoning-effort track. `fraction` is the selected
    /// position; `steps` is the current model's number of available levels.
    EffortSlider {
        id: String,
        fraction: f32,
        steps: usize,
        /// The final advertised level is Ultra. Other providers can end at
        /// High or Max without receiving the Ultra treatment.
        ultra_available: bool,
    },
    /// A figure that counts from the value it showed to the one it is given.
    /// UiTree keeps the shown value by stable ID and lays the text out as it
    /// reads on each frame, so what follows it keeps to its side.
    RollingNumber {
        id: String,
        value: f32,
        format: fn(f32) -> String,
    },
    UnitGrid(UnitGrid),
    Spacer,
}

/// How a `sticky_top` child of a column sits while pinned. Whole pixels, so
/// the option fits in the padding `Element` already has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StickyTop {
    /// `MAX` is no cap.
    pub(crate) max_height: std::num::NonZeroU16,
    pub(crate) inset: u8,
    /// Height of the fade under it while pinned; zero is none.
    pub(crate) fade: u8,
}

impl Default for StickyTop {
    fn default() -> Self {
        Self { max_height: std::num::NonZeroU16::MAX, inset: 0, fade: 0 }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: f32,
    pub min_height: f32,
    /// Upper bound for intrinsic width. An explicit minimum takes precedence.
    pub max_width: Option<f32>,
    /// Upper bound for intrinsic height. An explicit minimum takes precedence.
    pub max_height: Option<f32>,
    pub padding: f32,
    pub gap: f32,
    pub flex: f32,
    /// In a row, give up width the row cannot spare instead of overflowing.
    pub shrink: bool,
    pub background: Option<Color>,
    /// Paint-only scrim above children, opaque at the top and transparent at
    /// its bottom edge. Useful where scrolling content passes under a header.
    pub top_fade: Option<(Color, f32)>,
    /// Paint-only scrim above children, transparent at its top edge and
    /// tinted at the bottom. Scroll areas show it only above hidden content.
    pub bottom_fade: Option<(Color, f32)>,
    /// Passive bottom background behind children, for dedicated footer space.
    pub bottom_background_fade: Option<(Color, f32)>,
    /// Paint-only overflow cues at a horizontal scroll viewport's edges.
    pub horizontal_edge_fade: Option<(Color, f32)>,
    pub hover_background: Option<Color>,
    pub pressed_background: Option<Color>,
    /// Ink for an interactive control while hovered or pressed. Unlike a
    /// background treatment this never changes its hit bounds or geometry.
    pub hover_foreground: Option<Color>,
    pub pressed_foreground: Option<Color>,
    /// Border painted while an editor owns focus, including pointer focus.
    pub focus_border: Option<Border>,
    /// Editors that paint their own caret may suppress the outer keyboard ring.
    pub focus_ring: bool,
    /// Action menus share one pointer/keyboard highlight instead of a button ring.
    pub action_menu_item: bool,
    /// Duration of interactive background transitions, in milliseconds.
    pub transition_ms: u16,
    pub border: Option<Border>,
    pub dashed_border: bool,
    pub shadow: Option<BoxShadow>,
    /// Clip child paint and hit regions to the content rectangle. The parent
    /// background, border and shadow still paint at their normal bounds.
    pub clip_children: bool,
    pub center_content: bool,
    pub underline_on_hover: bool,
    pub foreground: Color,
    pub radius: f32,
    pub corner_radii: Option<CornerRadii>,
    /// Surrounding surface painted outside rounded corners after children.
    /// This is passive paint; use `clip_children` for rectangular child bounds.
    pub corner_mask: Option<Color>,
    pub font_size: f32,
    pub font_family: FontFamily,
    /// Minimum line box for formatted paragraphs, in logical pixels. Glyphs
    /// retain their natural metrics; controls and editors are unaffected.
    pub line_height: Option<f32>,
    /// Side of a standalone icon; `None` is the standard 16 px.
    pub icon_side: Option<f32>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            min_width: 0.0,
            min_height: 0.0,
            max_width: None,
            max_height: None,
            padding: 0.0,
            gap: 0.0,
            flex: 0.0,
            shrink: false,
            background: None,
            top_fade: None,
            bottom_fade: None,
            bottom_background_fade: None,
            horizontal_edge_fade: None,
            hover_background: None,
            pressed_background: None,
            hover_foreground: None,
            pressed_foreground: None,
            focus_border: None,
            focus_ring: true,
            action_menu_item: false,
            transition_ms: 0,
            border: None,
            dashed_border: false,
            shadow: None,
            clip_children: false,
            center_content: false,
            underline_on_hover: false,
            foreground: Color::WHITE,
            radius: 0.0,
            corner_radii: None,
            corner_mask: None,
            font_size: DEFAULT_FONT_SIZE,
            font_family: FontFamily::Monospace,
            line_height: None,
            icon_side: None,
        }
    }
}

impl Element {
    pub fn code_editor(editor: crate::CodeEditor) -> Self {
        Self::new(ElementKind::CodeEditor(editor))
    }
    pub fn code_diff(diff: crate::CodeDiff) -> Self {
        Self::new(ElementKind::CodeDiff(diff))
    }
    pub fn flowchart(flowchart: crate::Flowchart) -> Self {
        Self::new(ElementKind::Flowchart(flowchart))
    }
    pub fn sequence_diagram(diagram: crate::SequenceDiagram) -> Self {
        Self::new(ElementKind::SequenceDiagram(diagram))
    }
    fn new(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            hover_peer: None,
            presence: None,
            replacement: None,
            hover_surface: None,
            indicator_slot: None,
            action_alias: None,
            menu_secondary_action: false,
            draggable: false,
            sticky_top: None,
            animate_height: None,
            kept: None,
            hover_reveal: HoverReveal::None,
            drop_target: false,
            menu_navigation: false,
            context_menu: None,
        }
    }

    /// Group a menu trigger, optional search input and its inline options.
    /// Arrow keys move focus; Enter activates through the usual button path.
    pub fn menu_navigation(mut self) -> Self {
        self.menu_navigation = true;
        self
    }

    // Floating decorations are transparent to their anchor's layout. Keep
    // constraints and flex participation at the parent-facing node; padding
    // and paint still belong exclusively to the original anchor.
    fn with_anchor_layout(kind: ElementKind, anchor: Style) -> Self {
        let mut element = Self::new(kind);
        element.style.width = anchor.width;
        element.style.height = anchor.height;
        element.style.min_width = anchor.min_width;
        element.style.min_height = anchor.min_height;
        element.style.max_width = anchor.max_width;
        element.style.max_height = anchor.max_height;
        element.style.flex = anchor.flex;
        element
    }

    pub fn column(children: Vec<Self>) -> Self {
        Self::new(ElementKind::Column(children))
    }

    pub fn row(children: Vec<Self>) -> Self {
        Self::new(ElementKind::Row(children))
    }

    pub fn tab_strip(children: Vec<Self>, action_ids: Vec<String>) -> Self {
        Self::new(ElementKind::TabStrip {
            children,
            action_ids,
            selection: None,
        })
    }

    /// Animate one tab's selected surface to another without changing either
    /// tab's layout or hit region.
    pub fn tab_selection(
        mut self,
        group_id: impl Into<String>,
        selected_id: impl Into<String>,
        color: Color,
        radius: f32,
        animation_ms: u16,
    ) -> Self {
        if let ElementKind::TabStrip { selection, .. } = &mut self.kind {
            *selection = Some(TabSelection {
                group_id: group_id.into(),
                selected_id: selected_id.into(),
                color,
                radius: radius.max(0.0),
                animation_ms,
            });
        }
        self
    }

    /// Fade and settle this element in when it first appears. For content
    /// in the page's flow, which leaves at once when it is removed.
    pub fn animate_enter(mut self, id: impl Into<String>) -> Self {
        self.presence = Some((id.into(), false));
        self
    }

    /// As [`Element::animate_enter`], and fade out in place once removed.
    /// For floating content such as a toast: nothing else moves into the
    /// space it leaves, so its last paint can linger there.
    pub fn animate_presence(mut self, id: impl Into<String>) -> Self {
        self.presence = Some((id.into(), true));
        self
    }

    /// Animate this element taking the place of an earlier one. When `key`
    /// changes under the same `id`, it rises into place from its own height
    /// below while the previous paint moves up by the same distance and
    /// fades. Layout does not change. Place it in a parent that clips, as
    /// [`crate::Ticker`] does, so neither paint leaves the slot.
    pub fn animate_replace(mut self, id: impl Into<String>, key: impl Into<String>) -> Self {
        self.replacement = Some(Box::new((id.into(), key.into())));
        self
    }

    /// Keep this element's slot in the flow while its content leaves. When
    /// its natural height becomes zero, such as a column whose only child is
    /// no longer rendered, the slot closes over `exit_ms` while the last
    /// paint fades, clipped to what is left and taking no pointer input.
    /// With `expand` it also opens from zero over `enter_ms` when content
    /// arrives in a slot that was rendered empty. Siblings follow the height on every frame. The element must
    /// stay in the tree, empty, for as long as it may animate: see
    /// [`crate::Collapse`]. Replaces `animate_enter` on the same element.
    pub fn animate_height(mut self, id: impl Into<String>, expand: bool) -> Self {
        self.presence = Some((id.into(), false));
        self.animate_height = Some(expand);
        self
    }

    /// Cross-fade the foreground when the colour given under `id` changes,
    /// over `move_ms`. For a text or an icon whose colour carries a state.
    pub fn animate_foreground(mut self, id: impl Into<String>) -> Self {
        self.kept.get_or_insert_default().foreground = Some(id.into());
        self
    }

    /// On a column: a child with [`Element::animate_height`] that stops being
    /// passed closes where it was, its last paint fading, instead of
    /// vanishing with a jump. The tree remembers the children under `id`
    /// and keeps the slot of one that left until it has closed. The child's
    /// own ID names it, so it must stay the same across rebuilds.
    pub fn keep_leaving(mut self, id: impl Into<String>) -> Self {
        self.kept.get_or_insert_default().leaving = Some(id.into());
        self
    }

    /// Let a tab strip's selection rest on this child when `id` is selected.
    pub fn indicator_slot(mut self, id: impl Into<String>) -> Self {
        self.indicator_slot = Some(id.into());
        self
    }

    pub fn panel(child: Self) -> Self {
        Self::new(ElementKind::Panel(Box::new(child)))
    }

    pub fn clickable(id: impl Into<String>, child: Self) -> Self {
        Self::new(ElementKind::Clickable {
            id: id.into(),
            child: Box::new(child),
        })
        .transition_ms(crate::tokens().motion.hover_ms)
    }

    /// Keep one visual hover surface across a parent action and one nested
    /// control while their click targets stay independent.
    pub fn hover_with(mut self, action_id: impl Into<String>) -> Self {
        self.hover_peer = Some(action_id.into());
        self
    }

    /// Share hover and pressed paint across independent actions without
    /// expanding their hit targets. Apply to a passive row or panel; `id`
    /// identifies its color transition and must be distinct from action IDs.
    pub fn hover_surface(
        mut self,
        id: impl Into<String>,
        actions: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.hover_surface = Some(Box::new((id.into(), actions.into_iter().map(Into::into).collect())));
        self
    }

    /// Keep a unique focus target while dispatching an existing action ID.
    /// For example, a dialog's close icon and Cancel button can both cancel
    /// without collapsing into one keyboard stop.
    pub fn dispatch_as(mut self, action_id: impl Into<String>) -> Self {
        self.action_alias = Some(action_id.into());
        self
    }

    /// Keep an auxiliary popover action accessible by Tab and activation,
    /// while Up/Down/Home/End navigate only the menu's primary actions.
    /// Apply to the interactive element before wrapping it in a tooltip.
    pub fn menu_secondary_action(mut self) -> Self {
        self.menu_secondary_action = true;
        self
    }

    /// Let a button or clickable start a drag once the pointer leaves the
    /// click tolerance. A press released inside it is still a click.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// Keep this child of a column in view while its section scrolls. Once
    /// the scroll area around the column has carried it past the top, it is
    /// drawn at the top of the viewport, above the content passing under it
    /// and above the scroll area's fades, until the next `sticky_top` child
    /// of the same column pushes it out. It takes clicks where it is drawn.
    /// The column's height and the scroll range do not change. Give it an
    /// opaque `background`.
    pub fn sticky_top(mut self) -> Self {
        self.sticky_top.get_or_insert(StickyTop::default());
        self
    }

    /// A [`sticky_top`](Self::sticky_top) child that gives up height while
    /// pinned: its bottom edge follows the scroll until `height` is left,
    /// so a long one cannot cover the viewport. What no longer fits is
    /// clipped and fades into the element's `background`.
    pub fn sticky_top_max_height(mut self, height: f32) -> Self {
        // Whole pixels: a float-to-integer cast saturates, and zero is not a cap.
        if let Some(height) = std::num::NonZeroU16::new(height.round() as u16) {
            self.sticky_top.get_or_insert(StickyTop::default()).max_height = height;
        }
        self
    }

    /// A [`sticky_top`](Self::sticky_top) child pinned this far below the
    /// top of the viewport instead of against it.
    pub fn sticky_top_inset(mut self, inset: f32) -> Self {
        if inset.is_finite() && inset >= 0.0 {
            self.sticky_top.get_or_insert(StickyTop::default()).inset = inset.round() as u8;
        }
        self
    }

    /// A [`sticky_top`](Self::sticky_top) child with a fade under it while
    /// pinned: the rows passing under it fade out over `height` below its
    /// bottom edge, in the color of the scroll area's `top_fade`, instead of
    /// being cut at the edge. It arrives as the child pins and gives way to
    /// the next sticky child. It is paint only; without a `top_fade` on the
    /// scroll area nothing is drawn.
    pub fn sticky_top_fade(mut self, height: f32) -> Self {
        if height.is_finite() && height >= 1.0 {
            self.sticky_top.get_or_insert(StickyTop::default()).fade = height.round() as u8;
        }
        self
    }

    /// Make this element the area whose hover reveals the
    /// [`reveal_on_hover`](Self::reveal_on_hover) elements inside it: a
    /// message around its row of actions. It is passive: it adds no pointer
    /// target, no focus stop and no paint, and the host mirrors nothing.
    pub fn hover_region(mut self) -> Self {
        self.hover_reveal = HoverReveal::Region;
        self
    }

    /// Paint this element only while the pointer is over the
    /// [`hover_region`](Self::hover_region) around it, while one of its
    /// controls has keyboard focus, or while a popover inside it is open.
    /// It keeps its place in the layout and its controls stay in the Tab
    /// order, so nothing moves when it appears and Tab reaches it. It fades
    /// over `enter_ms` and `exit_ms`, or switches at once with reduced
    /// motion. `id` must be unique in the window. It replaces
    /// [`animate_enter`](Self::animate_enter) on the same element.
    pub fn reveal_on_hover(mut self, id: impl Into<String>) -> Self {
        self.presence = Some((id.into(), false));
        self.hover_reveal = HoverReveal::Revealed;
        self
    }

    /// Accept drops. Applies to buttons, clickables and scroll surfaces
    /// (including virtual lists), which are the elements with a pointer region.
    pub fn drop_target(mut self) -> Self {
        self.drop_target = true;
        self
    }

    /// Layer a popover or modal over a base tree. The base determines layout;
    /// floating is painted after the ordinary scene and receives pointer hits
    /// first. The window viewport remains its outer clip.
    pub fn overlay(base: Self, floating: Self) -> Self {
        Self::new(ElementKind::Overlay {
            base: Box::new(base),
            floating: Box::new(floating),
        })
    }

    /// Float content above surrounding rows and scroll clips. The anchor
    /// determines normal layout; `floating` is placed against it in the
    /// viewport and receives pointer hits before the underlying tree.
    pub fn popover(
        anchor: Self,
        floating: Self,
        dismiss_id: impl Into<String>,
        placement: PopoverPlacement,
    ) -> Self {
        let layout = anchor.style;
        Self::with_anchor_layout(
            ElementKind::Popover {
                anchor: Box::new(anchor),
                floating: Box::new(floating),
                dismiss_id: dismiss_id.into(),
                placement,
                gap: 5.0,
                animation_ms: crate::tokens().motion.enter_ms,
                close_animation_ms: None,
                open_scale: 1.0,
                view_key: None,
                hover_open: false,
                backdrop: true,
                focus_scope: true,
                backdrop_dismiss: true,
                scrim: None,
                initial_focus: None,
                return_focus: None,
                coach: None,
            },
            layout,
        )
    }

    /// Place the surface against existing controls, found by their IDs once
    /// the anchor has been laid out, with a caret and a ring on them.
    pub(crate) fn popover_coach(mut self, value: CoachAnchor) -> Self {
        if let ElementKind::Popover { coach, .. } = &mut self.kind {
            *coach = Some(Box::new(value));
        }
        self
    }

    /// Prefer a control when a modal first opens. The target must be a
    /// focusable descendant of the floating subtree.
    pub fn popover_initial_focus(mut self, id: impl Into<String>) -> Self {
        if let ElementKind::Popover { initial_focus, .. } = &mut self.kind {
            *initial_focus = Some(id.into());
        }
        self
    }

    /// Restore focus to a stable control when a modal closes, including when
    /// the action that opened it belonged to a menu that has since closed.
    pub fn popover_return_focus(mut self, id: impl Into<String>) -> Self {
        if let ElementKind::Popover { return_focus, .. } = &mut self.kind {
            *return_focus = Some(id.into());
        }
        self
    }

    /// Show a submenu while its trigger or content is hovered or focused.
    pub fn popover_on_hover(mut self) -> Self {
        if let ElementKind::Popover {
            hover_open,
            backdrop,
            gap,
            ..
        } = &mut self.kind
        {
            *hover_open = true;
            *backdrop = false;
            *gap = 0.0;
        }
        self
    }

    /// Float a host-controlled suggestion surface while leaving the editor
    /// and surrounding controls interactive. The host owns keyboard selection
    /// and dismissal; this surface never captures or restores keyboard focus.
    pub fn popover_non_modal(mut self) -> Self {
        if let ElementKind::Popover {
            hover_open,
            backdrop,
            focus_scope,
            scrim,
            initial_focus,
            return_focus,
            ..
        } = &mut self.kind
        {
            *hover_open = false;
            *backdrop = false;
            *focus_scope = false;
            *scrim = None;
            *initial_focus = None;
            *return_focus = None;
        }
        self
    }

    /// Distance between the anchor and the floating surface, 5 unless set.
    /// With 0 under a hairline anchor the surface starts where the anchor
    /// is, so it can stand in for the row that follows. Give the anchor some
    /// height: a scroll area does not place an element that has none.
    pub fn popover_gap(mut self, value: f32) -> Self {
        if let ElementKind::Popover { gap, .. } = &mut self.kind {
            if value.is_finite() && value >= 0.0 {
                *gap = value;
            }
        }
        self
    }

    /// Dim the viewport behind an open modal surface.
    pub fn popover_scrim(mut self, color: Color) -> Self {
        if let ElementKind::Popover { scrim, .. } = &mut self.kind {
            *scrim = Some(color);
        }
        self
    }

    /// Keep a modal open when its backdrop is clicked while still blocking
    /// interaction with the content underneath.
    pub fn popover_backdrop_dismiss(mut self, enabled: bool) -> Self {
        if let ElementKind::Popover {
            backdrop_dismiss, ..
        } = &mut self.kind
        {
            *backdrop_dismiss = enabled;
        }
        self
    }

    /// Set the entrance animation for a floating surface. Visual movement does
    /// not change the surface's stable hit bounds or keyboard focus order.
    pub fn popover_animation_ms(mut self, duration: u16) -> Self {
        if let ElementKind::Popover { animation_ms, .. } = &mut self.kind {
            *animation_ms = duration;
        }
        self
    }

    /// Set the exit duration independently of entrance. By default exits use
    /// the established short fade (at most 110 ms).
    pub fn popover_close_animation_ms(mut self, duration: u16) -> Self {
        if let ElementKind::Popover {
            close_animation_ms, ..
        } = &mut self.kind
        {
            *close_animation_ms = Some(duration);
        }
        self
    }

    /// Start entrance at this visual scale while keeping final hit geometry.
    pub fn popover_open_scale(mut self, scale: f32) -> Self {
        if let ElementKind::Popover { open_scale, .. } = &mut self.kind {
            if scale.is_finite() {
                *open_scale = scale.clamp(0.5, 1.25);
            }
        }
        self
    }

    /// Animate content when a retained popover switches between keyed views.
    pub fn popover_view_key(mut self, key: impl Into<String>) -> Self {
        if let ElementKind::Popover { view_key, .. } = &mut self.kind {
            *view_key = Some(key.into());
        }
        self
    }

    /// Override the preferred side of an already rendered tooltip, for example
    /// a toolbar helper reused in a sidebar. This only changes the outer hint;
    /// it leaves its anchor, layout, focus and shortcut unchanged.
    pub fn tooltip_placement(mut self, preferred: TooltipPlacement) -> Self {
        if let ElementKind::Tooltip { placement, .. } = &mut self.kind {
            *placement = preferred;
        }
        self
    }

    pub(crate) fn tooltip(
        anchor: Self,
        anchor_id: impl Into<String>,
        title: impl Into<String>,
        shortcut: Option<String>,
        placement: TooltipPlacement,
        colors: TooltipStyle,
    ) -> Self {
        let layout = anchor.style;
        Self::with_anchor_layout(
            ElementKind::Tooltip {
                anchor: Box::new(anchor),
                anchor_id: anchor_id.into(),
                title: title.into(),
                shortcut,
                sections: Vec::new(),
                max_width: 360.0,
                placement,
                colors,
            },
            layout,
        )
    }

    pub fn scroll(id: impl Into<String>, child: Self) -> Self {
        Self::new(ElementKind::Scroll {
            id: id.into(),
            child: Box::new(child),
            appearance: ScrollbarAppearance::default(),
            motion: ScrollMotion::default(),
        })
    }

    /// A scrollable list for one fixed-height sequence, including section
    /// headers if desired. Build each row from its index instead of eagerly
    /// constructing every row. Set `.flex(1.0)` when it fills the remaining
    /// height of a column.
    pub fn uniform_list(
        id: impl Into<String>,
        row_count: usize,
        row_height: f32,
        build_row: impl Fn(usize) -> Self + Send + Sync + 'static,
    ) -> Self {
        Self::new(ElementKind::UniformList(UniformList {
            id: id.into(),
            row_count,
            row_height: if row_height.is_finite() {
                row_height.max(1.0)
            } else {
                1.0
            },
            build_row: Arc::new(build_row),
            appearance: ScrollbarAppearance::default(),
            motion: ScrollMotion::default(),
        }))
    }

    /// Use the same scrollbar policy for ordinary scroll content and virtual
    /// lists. The default reserves a rail on overflow, leaving row controls
    /// and their hover surfaces clear of scrollbar pointer targets.
    pub fn scrollbar_appearance(mut self, appearance: ScrollbarAppearance) -> Self {
        match &mut self.kind {
            ElementKind::Scroll {
                appearance: current,
                ..
            } => *current = appearance,
            ElementKind::UniformList(list) => list.appearance = appearance,
            _ => {}
        }
        self
    }

    /// Configure wheel and scrollbar-button interpolation for ordinary
    /// scroll content or a virtual list without changing scrollbar appearance.
    pub fn scroll_motion(mut self, motion: ScrollMotion) -> Self {
        match &mut self.kind {
            ElementKind::Scroll {
                motion: current, ..
            } => *current = motion,
            ElementKind::UniformList(list) => list.motion = motion,
            _ => {}
        }
        self
    }

    pub fn text(value: impl Into<String>) -> Self {
        Self::new(ElementKind::Text(value.into()))
    }

    pub fn shimmer_text(id: impl Into<String>, text: impl Into<String>, highlight: Color) -> Self {
        Self::new(ElementKind::ShimmerText {
            id: id.into(),
            text: text.into(),
            highlight,
            truncated: false,
        })
    }

    /// A single measured line with a paint-only moving highlight. The text
    /// uses the same ellipsis and vertical alignment as `truncated_text`.
    pub fn shimmer_truncated_text(
        id: impl Into<String>,
        text: impl Into<String>,
        highlight: Color,
    ) -> Self {
        Self::new(ElementKind::ShimmerText {
            id: id.into(),
            text: text.into(),
            highlight,
            truncated: true,
        })
    }

    pub fn progress_ring(fraction: Option<f32>, track: Color, fill: Color) -> Self {
        Self::new(ElementKind::ProgressRing {
            fraction: fraction
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(0.0, 1.0)),
            track,
            fill,
        })
    }

    pub fn keyboard_shortcut(keys: impl Into<String>, fill: Color, max_width: Option<f32>) -> Self {
        Self::new(ElementKind::KeyboardShortcut {
            keys: keys.into(),
            fill,
            max_width,
            menu: false,
        })
    }

    /// Plain, measured menu accelerator. Width limits omit the full shortcut
    /// instead of showing a misleading partial chord.
    pub fn menu_shortcut(keys: impl Into<String>, max_width: Option<f32>) -> Self {
        Self::new(ElementKind::KeyboardShortcut {
            keys: keys.into(),
            fill: Color::rgba(0, 0, 0, 0),
            max_width,
            menu: true,
        })
    }

    pub fn truncated_text(value: impl Into<String>) -> Self {
        Self::new(ElementKind::TruncatedText(value.into()))
    }

    /// Keep a navigation title on one line, fading only when its painted
    /// width exceeds the available space. No ellipsis or background mask.
    pub fn faded_text(value: impl Into<String>, fade_width: f32) -> Self {
        Self::new(ElementKind::FadedText {
            text: value.into(),
            fade_width: if fade_width.is_finite() {
                fade_width.max(0.0)
            } else {
                24.0
            },
        })
    }

    pub fn formatted_text(spans: Vec<TextSpan>) -> Self {
        Self::new(ElementKind::FormattedText(spans))
    }

    pub fn raster_image(image: RasterImage) -> Self {
        Self::new(ElementKind::RasterImage(image))
    }

    /// Reserve a rectangle for pixels the host presents to the backend under
    /// `id` as they change, without rebuilding the tree. `label` names it for
    /// assistive technology.
    pub fn surface(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ElementKind::Surface {
            id: id.into(),
            label: label.into(),
        })
    }

    pub fn icon(icon: Icon) -> Self {
        Self::new(ElementKind::Icon(icon))
    }

    pub fn button(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(ElementKind::Button {
            id: id.into(),
            label: label.into(),
            trailing_icon: None,
        })
        .padding(8.0)
        .transition_ms(crate::tokens().motion.hover_ms)
    }

    /// Pass an empty label for an icon-only button.
    pub fn icon_button(id: impl Into<String>, icon: Icon, label: impl Into<String>) -> Self {
        Self::new(ElementKind::IconButton {
            id: id.into(),
            icon,
            label: label.into(),
            trailing_icon: None,
        })
        .padding(4.0)
        .gap(6.0)
        .transition_ms(crate::tokens().motion.hover_ms)
    }

    /// Place a second canonical-size icon at the far edge of an icon button.
    /// The entire control keeps one action ID and one hit region.
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        match &mut self.kind {
            ElementKind::Button { trailing_icon, .. }
            | ElementKind::IconButton { trailing_icon, .. } => {
                *trailing_icon = Some(icon);
            }
            _ => {}
        }
        self
    }

    pub fn text_input(
        id: impl Into<String>,
        initial_value: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self::new(ElementKind::TextInput {
            id: id.into(),
            initial_value: initial_value.into(),
            placeholder: placeholder.into(),
            persistent_placeholder: false,
            leading_icon: None,
            leading_icon_tint: None,
            shortcut: None,
            shortcut_fill: None,
            password: false,
        })
        .padding(8.0)
    }

    /// Keep a single-line input's placeholder visible while focused and empty.
    pub fn persistent_placeholder(mut self, visible: bool) -> Self {
        if let ElementKind::TextInput {
            persistent_placeholder,
            ..
        } = &mut self.kind
        {
            *persistent_placeholder = visible;
        }
        self
    }

    /// Place a canonical-size icon inside an input's own hit and focus box.
    /// Text, caret and selection reserve its width; the icon is decorative.
    pub fn leading_icon(mut self, icon: Icon) -> Self {
        if let ElementKind::TextInput { leading_icon, .. } = &mut self.kind {
            *leading_icon = Some(icon);
        }
        self
    }

    /// Hide editable secrets in paint and prevent copying them to the clipboard.
    pub fn password(mut self, hidden: bool) -> Self {
        if let ElementKind::TextInput { password, .. } = &mut self.kind {
            *password = hidden;
        }
        self
    }

    pub fn leading_icon_tint(mut self, color: Color) -> Self {
        if let ElementKind::TextInput {
            leading_icon_tint, ..
        } = &mut self.kind
        {
            *leading_icon_tint = Some(color);
        }
        self
    }

    /// A decorative keyboard hint inside the input's focus and hit bounds.
    pub fn input_shortcut(mut self, value: impl Into<String>) -> Self {
        if let ElementKind::TextInput { shortcut, .. } = &mut self.kind {
            *shortcut = Some(value.into());
        }
        self
    }

    /// Background of the passive keyboard hint inside a text input.
    pub fn input_shortcut_fill(mut self, color: Color) -> Self {
        if let ElementKind::TextInput { shortcut_fill, .. } = &mut self.kind {
            *shortcut_fill = Some(color);
        }
        self
    }

    /// A multi-line editor. Enter inserts a line break; Ctrl+Enter submits.
    pub fn text_area(
        id: impl Into<String>,
        initial_value: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self::new(ElementKind::TextArea {
            id: id.into(),
            initial_value: initial_value.into(),
            placeholder: placeholder.into(),
            auto_grow: None,
            preserve_line_endings: false,
            highlights: Vec::new(),
        })
        .padding(8.0)
    }

    /// Size a multiline editor from its wrapped lines and the host's font
    /// metrics. Beyond the maximum height, its existing cursor scroll applies.
    pub fn text_area_auto_grow(mut self, min_height: f32, max_height: f32) -> Self {
        if min_height.is_finite() && max_height.is_finite() {
            if let ElementKind::TextArea { auto_grow, .. } = &mut self.kind {
                let minimum = min_height.max(1.0);
                *auto_grow = Some(max_height.max(minimum));
                self.style.min_height = minimum;
                self.style.height = None;
            }
        }
        self
    }

    /// Highlights refer to `initial_value`; stale host ranges are skipped until
    /// that value catches up with the authoritative input buffer.
    pub fn text_area_highlights(mut self, values: Vec<TextHighlight>) -> Self {
        if let ElementKind::TextArea { highlights, .. } = &mut self.kind {
            *highlights = values.into_iter().take(64).collect();
        }
        self
    }

    /// Preserve exact editable UTF-8 while displaying CRLF/CR as line breaks.
    pub fn text_area_preserve_line_endings(mut self, preserve: bool) -> Self {
        if let ElementKind::TextArea {
            preserve_line_endings,
            ..
        } = &mut self.kind
        {
            *preserve_line_endings = preserve;
        }
        self
    }

    pub fn spacer() -> Self {
        Self::new(ElementKind::Spacer)
    }

    /// A figure that counts to `value` from the one it showed under the same
    /// `id`, written by `format` on every frame. The first value is simply
    /// shown. Style it as a text.
    pub fn rolling_number(id: impl Into<String>, value: f32, format: fn(f32) -> String) -> Self {
        Self::new(ElementKind::RollingNumber {
            id: id.into(),
            value: if value.is_finite() { value } else { 0.0 },
            format,
        })
    }

    /// A field of cells shared out among parts. It takes the size of its
    /// cells; see [`UnitGrid`].
    pub fn unit_grid(grid: UnitGrid) -> Self {
        Self::new(ElementKind::UnitGrid(grid))
    }

    /// A non-interactive loading surface with paint-only motion.
    pub fn skeleton(id: impl Into<String>, base: Color, highlight: Color) -> Self {
        Self::new(ElementKind::Skeleton {
            id: id.into(),
            base,
            highlight,
        })
    }

    pub fn spinner(id: impl Into<String>, color: Color, variant: SpinnerVariant) -> Self {
        Self::new(ElementKind::Spinner {
            id: id.into(),
            color,
            variant,
        })
    }

    /// Build a reasoning-effort slider with model-specific stops.
    /// Dragging previews a continuous fraction; release snaps to a stop.
    pub fn effort_slider(id: impl Into<String>, fraction: f32, steps: usize) -> Self {
        Self::new(ElementKind::EffortSlider {
            id: id.into(),
            fraction: if fraction.is_finite() {
                fraction.clamp(0.0, 1.0)
            } else {
                0.0
            },
            steps: steps.max(2),
            ultra_available: false,
        })
    }

    /// Enable the violet finish and celebration only for a model whose last
    /// advertised reasoning level is Ultra.
    pub fn effort_ultra_available(mut self, available: bool) -> Self {
        if let ElementKind::EffortSlider {
            ultra_available, ..
        } = &mut self.kind
        {
            *ultra_available = available;
        }
        self
    }

    pub fn padding(mut self, value: f32) -> Self {
        self.style.padding = value.max(0.0);
        self
    }

    pub fn gap(mut self, value: f32) -> Self {
        self.style.gap = value.max(0.0);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    /// Fade content at this element's top edge without changing its layout,
    /// clipping, scroll range, or pointer regions.
    pub fn top_fade(mut self, color: Color, height: f32) -> Self {
        self.style.top_fade = (height.is_finite() && height > 0.0).then_some((color, height));
        self
    }

    /// Fade content at this element's bottom edge without changing layout or
    /// pointer regions. Scroll areas leave their final visible row unobscured.
    pub fn bottom_fade(mut self, color: Color, height: f32) -> Self {
        self.style.bottom_fade = (height.is_finite() && height > 0.0).then_some((color, height));
        self
    }

    /// Add a quiet bottom background behind children. Use dedicated footer
    /// space so the tint never covers the last line or the collapse control.
    pub fn bottom_background_fade(mut self, color: Color, height: f32) -> Self {
        self.style.bottom_background_fade =
            (height.is_finite() && height > 0.0).then_some((color, height));
        self
    }

    /// Fade only the horizontal edges with hidden scroll content. The scrims
    /// are passive; selection/focus reveal reserves this margin when it fits.
    pub fn horizontal_edge_fade(mut self, color: Color, width: f32) -> Self {
        self.style.horizontal_edge_fade =
            (width.is_finite() && width > 0.0).then_some((color, width));
        self
    }

    /// Background shown when the pointer is over an interactive element.
    pub fn hover_background(mut self, color: Color) -> Self {
        self.style.hover_background = Some(color);
        self
    }

    /// Background shown while an interactive element is pressed.
    pub fn pressed_background(mut self, color: Color) -> Self {
        self.style.pressed_background = Some(color);
        self
    }

    pub fn hover_foreground(mut self, color: Color) -> Self {
        self.style.hover_foreground = Some(color);
        self
    }

    pub fn pressed_foreground(mut self, color: Color) -> Self {
        self.style.pressed_foreground = Some(color);
        self
    }

    pub fn focus_border(mut self, color: Color, width: f32) -> Self {
        self.style.focus_border =
            (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self
    }

    /// Let the pointer drag a horizontal viewport's content sideways, and
    /// show a thin bar where it sits while there is more than fits. The bar
    /// takes the viewport's foreground; pressing a control inside still
    /// presses that control.
    pub fn drag_to_pan(mut self) -> Self {
        if let ElementKind::HorizontalScroll { drag_pan, .. } = &mut self.kind {
            *drag_pan = true;
        }
        self
    }

    pub fn horizontal_scroll(id: impl Into<String>, child: Element) -> Self {
        Self::new(ElementKind::HorizontalScroll {
            id: id.into(),
            child: Box::new(child),
            reveal: None,
            vertical_wheel: true,
            drag_pan: false,
        })
    }

    /// A horizontal viewport inside a vertically scrolling document. Trackpad
    /// X and Shift+wheel pan horizontally; ordinary wheel Y reaches the parent.
    pub fn horizontal_scroll_2d(id: impl Into<String>, child: Element) -> Self {
        Self::new(ElementKind::HorizontalScroll {
            id: id.into(),
            child: Box::new(child),
            reveal: None,
            vertical_wheel: false,
            drag_pan: false,
        })
    }

    /// Keep this item visible when selection or the viewport changes. Manual
    /// scrolling stays where the user left it while selection remains stable.
    pub fn reveal_horizontal_item(mut self, id: impl Into<String>) -> Self {
        if let ElementKind::HorizontalScroll { reveal, .. } = &mut self.kind {
            *reveal = Some(id.into());
        }
        self
    }

    pub fn focus_ring(mut self, visible: bool) -> Self {
        self.style.focus_ring = visible;
        self
    }

    /// Opt an enabled action row into menu focus tracking. Checked state is
    /// independent; the current pointer or keyboard row owns the highlight.
    pub fn action_menu_item(mut self) -> Self {
        self.style.action_menu_item = true;
        self.style.focus_ring = false;
        self.style.transition_ms = 0;
        self
    }

    /// Smooths hover and press backgrounds without changing layout or hit bounds.
    /// Set to zero for an immediate response.
    pub fn transition_ms(mut self, duration: u16) -> Self {
        self.style.transition_ms = duration;
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.style.foreground = color;
        self
    }

    pub fn center_content(mut self) -> Self {
        self.style.center_content = true;
        self
    }

    pub fn underline_on_hover(mut self) -> Self {
        self.style.underline_on_hover = true;
        self
    }

    /// Draw a border inside the element's bounds without changing its layout size.
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.style.border = (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self
    }

    pub fn dashed_border(mut self, color: Color, width: f32) -> Self {
        self.style.border = (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self.style.dashed_border = true;
        self
    }

    /// Add depth to a card or floating surface without changing its layout.
    pub fn shadow(mut self, color: Color, offset: Point, blur: f32, spread: f32) -> Self {
        self.style.shadow = (color.a > 0).then_some(BoxShadow::new(color, offset, blur, spread));
        self
    }

    /// Keep descendants inside this element's content rectangle. Useful for
    /// a dock whose width changes while its children retain intrinsic sizes.
    pub fn clip_children(mut self) -> Self {
        self.style.clip_children = true;
        self
    }

    /// Center this element vertically in whatever height its parent gives
    /// it. The wrapper takes over the element's width and flex, so it stands
    /// in the same place in a row; the element keeps its own height.
    pub fn centered_y(mut self) -> Self {
        let width = self.style.width;
        let flex = self.style.flex;
        self.style.flex = 0.0;
        Self::column(vec![Self::spacer().flex(1.0), self, Self::spacer().flex(1.0)])
            .size(width, None)
            .flex(flex)
    }

    /// Center this element horizontally in the width its parent gives it.
    pub fn centered_x(self) -> Self {
        Self::row(vec![Self::spacer().flex(1.0), self, Self::spacer().flex(1.0)])
    }

    /// Center this element on both axes in the space its parent gives it.
    pub fn centered(self) -> Self {
        Self::column(vec![
            Self::spacer().flex(1.0),
            self.centered_x(),
            Self::spacer().flex(1.0),
        ])
    }

    /// Hold this element in a fixed box that does not flex and hides what
    /// falls outside, for content that keeps its own width while the box
    /// around it is resized or animated.
    pub fn clip_to(self, width: f32, height: f32) -> Self {
        // A row keeps the child's width on its main axis; a column would
        // clamp it to the box and reflow the contents.
        let finite = |value: f32| if value.is_finite() { value } else { 0.0 };
        Self::row(vec![self])
            .size(Some(finite(width)), Some(finite(height)))
            .flex(0.0)
            .clip_children()
    }

    /// Set text size in logical pixels. Hosts should measure with the same
    /// font family and size that they use to paint each run.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.style.font_size = size;
        }
        self
    }

    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.style.font_family = family;
        self
    }

    /// Give formatted paragraphs additional leading without enlarging glyphs.
    pub fn line_height(mut self, height: f32) -> Self {
        if height.is_finite() && height > 0.0 {
            self.style.line_height = Some(height);
        }
        self
    }

    /// Kept for source compatibility while callers migrate to [`ICON_SIZE`].
    /// Icons inside controls use one size; this request is ignored.
    pub fn icon_size(self, _size: f32) -> Self {
        self
    }

    /// Draw a standalone [`Element::icon`] larger than the control size, for
    /// thumbnails and empty states. Controls keep the standard 16 px.
    pub fn large_icon(mut self, side: f32) -> Self {
        if side.is_finite() {
            self.style.icon_side = Some(side.clamp(ICON_SIZE, 128.0));
        }
        self
    }

    /// Keep the natural width while the row has room, and wrap within what
    /// is left once it does not. Flex siblings keep their minimum widths.
    pub fn shrink_to_fit(mut self) -> Self {
        self.style.shrink = true;
        self
    }

    pub fn flex(mut self, factor: f32) -> Self {
        self.style.flex = factor.max(0.0);
        self
    }

    pub fn size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        self.style.width = width.map(|value| value.max(0.0));
        self.style.height = height.map(|value| value.max(0.0));
        self
    }

    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.style.min_width = width.max(0.0);
        self.style.min_height = height.max(0.0);
        self
    }

    /// Cap intrinsic width without forcing short content to fill the budget.
    /// This limits the measured width, not the constraints used to measure
    /// descendants. Pair with `clip_children` to contain their overflow.
    /// Flex parents may still assign a different width, as with explicit `size`.
    pub fn max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.style.max_width = Some(width.max(0.0));
        }
        self
    }

    /// Cap intrinsic height without forcing short content to fill the budget.
    /// Pair with a scroll area to keep overflow reachable. Flex parents may
    /// still assign a different height, just as with an explicit `size`.
    pub fn max_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.style.max_height = Some(height.max(0.0));
        }
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.style.radius = radius.max(0.0);
        self.style.corner_radii = None;
        self
    }

    /// Override the uniform radius for backgrounds, solid borders and focus.
    /// Shadows and descendant clipping retain their existing behavior.
    pub fn corner_radii(mut self, radii: CornerRadii) -> Self {
        self.style.corner_radii = Some(radii.normalized());
        self
    }

    /// Preserve rounded outer corners above opaque children. The supplied
    /// color is the surrounding surface; solid borders finish above the mask.
    /// Floating menus remain above this local, non-interactive decoration.
    pub fn corner_mask(mut self, color: Color) -> Self {
        self.style.corner_mask = Some(color);
        self
    }
}

#[cfg(test)]
mod icon_tests {
    use super::Icon;

    #[test]
    fn filled_maps_an_outline_to_its_solid_form_and_leaves_the_rest() {
        assert_eq!(Icon::Settings.filled(), Icon::SettingsFill);
        assert_eq!(Icon::PanelLeft.filled(), Icon::PanelLeftOpen);
        assert_eq!(Icon::Bell.filled(), Icon::BellFill);
        // No solid form, and already solid.
        assert_eq!(Icon::Search.filled(), Icon::Search);
        assert_eq!(Icon::SettingsFill.filled(), Icon::SettingsFill);
    }
}

#[cfg(test)]
mod centering_tests {
    use super::*;
    use crate::{Color, DrawCommand, Rect, Size, UiTree};

    const INK: Color = Color::rgb(200, 40, 40);

    fn mark(width: f32, height: f32) -> Element {
        Element::spacer().background(INK).size(Some(width), Some(height))
    }

    fn painted(root: Element, size: Size) -> Rect {
        UiTree::new(root)
            .layout(size)
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == INK => Some(*rect),
                _ => None,
            })
            .expect("the mark is painted")
    }

    #[test]
    fn centered_y_is_the_spacer_sandwich_and_takes_over_width_and_flex() {
        let wrapper = mark(40.0, 10.0).flex(2.0).centered_y();
        assert_eq!((wrapper.style.width, wrapper.style.height, wrapper.style.flex), (Some(40.0), None, 2.0));
        let ElementKind::Column(children) = &wrapper.kind else {
            panic!("centered_y wraps in a column");
        };
        assert_eq!(children.len(), 3);
        assert_eq!((children[0].style.flex, children[1].style.flex, children[2].style.flex), (1.0, 0.0, 1.0));
        assert_eq!(children[1].style.width, Some(40.0));

        for width in [120.0, 600.0] {
            let row = Element::row(vec![mark(40.0, 10.0).centered_y(), Element::spacer().flex(1.0)]);
            let rect = painted(row, Size::new(width, 50.0));
            assert_eq!((rect.x, rect.y, rect.width, rect.height), (0.0, 20.0, 40.0, 10.0));
        }
    }

    #[test]
    fn centered_x_and_centered_place_the_element_in_the_middle() {
        for width in [120.0, 600.0] {
            let size = Size::new(width, 50.0);
            let across = painted(
                Element::column(vec![mark(40.0, 10.0).centered_x(), Element::spacer().flex(1.0)]),
                size,
            );
            assert_eq!((across.x, across.y), ((width - 40.0) / 2.0, 0.0));
            let both = painted(mark(40.0, 10.0).centered(), size);
            assert_eq!((both.x, both.y, both.width, both.height), ((width - 40.0) / 2.0, 20.0, 40.0, 10.0));
        }
    }

    #[test]
    fn clip_to_is_a_fixed_box_that_hides_what_overflows() {
        let boxed = mark(300.0, 20.0).clip_to(80.0, 20.0);
        assert_eq!((boxed.style.width, boxed.style.height, boxed.style.flex), (Some(80.0), Some(20.0), 0.0));
        assert!(boxed.style.clip_children);
        let bad = mark(300.0, 20.0).clip_to(f32::NAN, f32::INFINITY);
        assert_eq!((bad.style.width, bad.style.height), (Some(0.0), Some(0.0)));

        let frame = UiTree::new(Element::row(vec![boxed, Element::spacer().flex(1.0)]))
            .layout(Size::new(400.0, 20.0));
        let clip = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::PushClip(rect) => Some(*rect),
                _ => None,
            })
            .expect("the box clips its child");
        // The box clips to its 80 px while the child keeps its own 300.
        assert_eq!(clip.width, 80.0);
        assert_eq!(painted(Element::row(vec![mark(300.0, 20.0).clip_to(80.0, 20.0)]), Size::new(400.0, 20.0)).width, 300.0);
    }
}

#[cfg(test)]
mod faded_text_tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};

    #[test]
    fn fade_preserves_shaping_and_clips_alpha_without_ellipsis() {
        let title = "Unicode 👩‍💻 e\u{301} titulo muy largo";
        let mut tree = UiTree::new(Element::faded_text(title, 24.0).size(Some(80.0), Some(32.0)));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(Size::new(80.0, 32.0));
        let mut previous_alpha = u8::MAX;
        let mut clips = Vec::new();
        let mut painted = 0;
        for command in &frame.commands {
            match command {
                DrawCommand::PushClip(rect) => clips.push(*rect),
                DrawCommand::PopClip => {
                    clips.pop();
                }
                DrawCommand::Text {
                    text,
                    color,
                    position,
                    ..
                } => {
                    assert_eq!(text, title);
                    assert_eq!(position.x, 0.0);
                    assert!(color.a <= previous_alpha);
                    previous_alpha = color.a;
                    let clip = clips.last().expect("fading text must be clipped");
                    assert!(clip.x >= 0.0 && clip.x + clip.width <= 80.0);
                    painted += 1;
                }
                _ => {}
            }
        }
        assert_eq!(painted, 17);
        assert!(previous_alpha < 16);
        assert!(frame.regions.is_empty());
    }

    #[test]
    fn fitting_title_is_not_faded_or_repeated() {
        let mut tree = UiTree::new(Element::faded_text("Task", 24.0).size(Some(80.0), Some(32.0)));
        tree.set_text_measurer(|text, _, _| text.len() as f32 * 8.0);
        let frame = tree.layout(Size::new(80.0, 32.0));
        let text = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text.as_str(), color.a)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(text, [("Task", u8::MAX)]);
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;
    use crate::{DrawCommand, Rect, Size, UiTree};

    #[test]
    fn a_surface_names_its_pixels_and_fills_the_rectangle_it_is_given() {
        let root = Element::column(vec![
            Element::spacer().size(None, Some(20.0)),
            Element::clickable("page", Element::surface("page-pixels", "Página")).flex(1.0),
        ]);
        let frame = UiTree::new(root).layout(Size::new(300.0, 220.0));
        let surfaces: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Surface { rect, id, label, opacity } => Some((*rect, id.as_str(), label.as_str(), *opacity)),
                _ => None,
            })
            .collect();
        // The frame carries the name only: new pixels never rebuild the tree.
        assert_eq!(surfaces, [(Rect::new(0.0, 20.0, 300.0, 200.0), "page-pixels", "Página", 1.0)]);
        assert!(frame.regions.iter().any(|region| region.id == "page" && region.rect == surfaces[0].0));
    }
}
