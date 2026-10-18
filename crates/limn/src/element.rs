use crate::{Color, GlassMaterial, Point, RasterImage, ScrollMotion, ScrollbarAppearance, Size};
use std::sync::Arc;

/// Text size, in logical pixels, of an element that sets none.
pub const DEFAULT_FONT_SIZE: f32 = 14.0;
/// Canonical visual size of every chrome icon, in logical pixels.
/// The host rasterizes this size at the current display scale.
pub const ICON_SIZE: f32 = 16.0;

/// Preferred side of an anchored floating surface. The layout engine flips
/// vertically when the preferred side would leave the viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopoverPlacement {
    /// Below the anchor, their left edges aligned.
    BottomStart,
    /// Below the anchor, their right edges aligned.
    BottomEnd,
    /// Above the anchor, their left edges aligned.
    TopStart,
    /// Above the anchor, their right edges aligned.
    TopEnd,
    /// To the right of the anchor, their tops aligned: a submenu. It goes to
    /// the left instead when the right has no room.
    RightStart,
    /// To the left of the anchor, their tops aligned. It goes to the right
    /// instead when the left has no room.
    LeftStart,
    /// Centered in the viewport, wherever the anchor is: a modal. The gap is
    /// not used.
    Center,
    /// Centered horizontally against the viewport's bottom edge: a sheet.
    /// Ignores the anchor and gap, fills up to the viewport width, and keeps
    /// eight logical pixels clear above it. The host supplies a safe viewport.
    Bottom,
}

/// A shape for the pointer other than the platform's own, asked for with
/// [`Element::cursor`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cursor {
    /// Two arrows pointing sideways: the boundary between two columns.
    ResizeColumn,
    /// Two arrows pointing up and down: the boundary between two rows.
    ResizeRow,
}

/// Side of its target a coach mark sits on. The engine takes the opposite
/// side, then the other axis, when the preferred one has no room.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoachSide {
    /// Over the target, the caret pointing down at it.
    Above,
    /// Under the target, the caret pointing up at it.
    Below,
    /// To the left of the target, the caret pointing right at it.
    Left,
    /// To the right of the target, the caret pointing left at it.
    Right,
}

/// Where a coach mark sits along its target's edge.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoachAlign {
    /// Left edges together above or below the target; top edges together
    /// beside it.
    Start,
    /// Centered on the target along that edge.
    Center,
    /// Right edges together above or below the target; bottom edges together
    /// beside it.
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
    /// Over the anchor, centered on it horizontally.
    Above,
    /// Under the anchor, centered on it horizontally.
    Below,
    /// To the left of the anchor, centered on it vertically. Inside a popover
    /// it stands clear of the whole surface.
    Left,
    /// To the right of the anchor, centered on it vertically. Inside a
    /// popover it stands clear of the whole surface.
    Right,
}

/// Read-only blocks beneath a tooltip's title. Paragraphs wrap to the
/// available width; separators divide related pieces of information.
#[derive(Clone, Debug, PartialEq)]
pub enum TooltipSection {
    /// A wrapped paragraph in the tooltip's text color. Blank text is left
    /// out.
    Description(String),
    /// A caption followed by passive items, which wrap onto further lines.
    IconRow {
        /// Caption in the muted color: beside the items, or over them in a
        /// narrow tooltip. Blank draws none.
        label: String,
        /// The items in order. One with a blank label is skipped.
        items: Vec<TooltipItem>,
    },
    /// A hairline in the border color before the next block. A repeated or
    /// trailing one is ignored.
    Separator,
    /// An optional icon, then people's faces as one group: round pictures
    /// [`TOOLTIP_FACE`] wide that overlap by [`TOOLTIP_FACE_OVERLAP`], each
    /// cut from the one before by a ring of the tooltip's fill, and «+N»
    /// for `more` not shown. Without faces nothing is drawn.
    Faces {
        /// Drawn before the faces, [`TOOLTIP_FACE_ICON`] wide.
        icon: Option<Icon>,
        /// In order; the ones that do not fit the row join `more`.
        faces: Vec<TooltipFace>,
        /// People not among `faces`, said as «+N»; 0 says nothing.
        more: usize,
    },
    /// A wrapped paragraph in the muted color, slightly smaller. Blank text
    /// is left out.
    Footer(String),
}

/// A labeled, passive item in a tooltip row. Icons keep their canonical size.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipItem {
    /// Text of the item, shortened to the row's width. An item with a blank
    /// label is not drawn.
    pub label: String,
    /// Drawn before the label, the two inside a hairline box. `None` draws
    /// the label alone, in the muted color.
    pub icon: Option<Icon>,
    /// Draw the icon without the label; set by [`Self::icon_only`]. It has no
    /// effect on an item without an icon.
    pub icon_only: bool,
}

impl TooltipItem {
    /// An item drawn as its label, without an icon.
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

    /// Give the item an icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// The side of a face in a tooltip's row of faces.
pub const TOOLTIP_FACE: f32 = 24.0;
/// How far a face of that row overlaps the one before it.
pub const TOOLTIP_FACE_OVERLAP: f32 = 6.0;
/// The side of the icon that leads that row; 8 more stand before the faces.
pub const TOOLTIP_FACE_ICON: f32 = 20.0;

/// Someone in a tooltip's row of faces: their picture or, while it loads or
/// when they have none, their initial on their color.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipFace {
    /// Who they are. Its first letter stands in for the picture.
    pub name: String,
    /// Their picture, drawn round. A host passes it already fitted (cover).
    pub image: Option<RasterImage>,
    /// The ground of their initial.
    pub color: Color,
}

impl TooltipFace {
    /// Someone shown by their initial on `color`.
    pub fn new(name: impl Into<String>, color: Color) -> Self {
        Self {
            name: name.into(),
            image: None,
            color,
        }
    }

    /// Show their picture instead of the initial.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }
}

/// The colors a tooltip is painted with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TooltipStyle {
    /// Fill of the body and of its pointer.
    pub background: Color,
    /// The one pixel outline, the separators and the boxes around icon
    /// items.
    pub border: Color,
    /// Fill of the shortcut's key caps.
    pub keycap: Color,
    /// Ink of the title, descriptions, shortcut and icon items.
    pub text: Color,
    /// Ink of footers, row captions and items without an icon.
    pub muted: Color,
    /// Color of the shadow under the body.
    pub shadow: Color,
}

/// Semantic icon names. A host chooses the concrete artwork for each name.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Icon {
    /// A window with a side panel on its left: the left sidebar.
    PanelLeft,
    /// The left panel filled in: the sidebar while shown. It is what
    /// [`Self::filled`] gives for [`Self::PanelLeft`].
    PanelLeftOpen,
    /// Hiding the left sidebar. The bundled host draws it as
    /// [`Self::PanelLeftOpen`].
    PanelLeftClose,
    /// A window with a strip along its bottom: the bottom panel.
    PanelBottom,
    /// A window with a side panel on its right: the right sidebar.
    PanelRight,
    /// The right panel filled in: the sidebar while shown. It is what
    /// [`Self::filled`] gives for [`Self::PanelRight`].
    PanelRightOpen,
    /// Hiding the right sidebar. The bundled host draws it as
    /// [`Self::PanelRightOpen`].
    PanelRightClose,
    /// A plus: add or create.
    Add,
    /// A horizontal dash: remove, collapse or unstage.
    Minus,
    /// A microphone: voice input.
    Microphone,
    /// A card with lines of text: plain text.
    Text,
    /// A framed picture: an image file.
    Image,
    /// A sound wave: an audio file.
    Audio,
    /// A video camera: a video file.
    Video,
    /// A sheet with a folded corner and lines: a file of no known type.
    File,
    /// Embedded file/language/folder artwork; hosts preserve its native colors.
    FileType(crate::file_icons::FileIcon),
    /// The infinity sign.
    Infinity,
    /// A paperclip: an attachment.
    Paperclip,
    /// A bullseye.
    Target,
    /// A list with check marks: a plan and its steps.
    Plan,
    /// Thought or reasoning. The bundled host draws a light bulb.
    Brain,
    /// A check mark: done, or the chosen item.
    Check,
    /// A circled `i`: information.
    InfoCircle,
    /// A circled check mark: success.
    CheckCircle,
    /// An exclamation mark in a triangle: a warning.
    WarningTriangle,
    /// A circled cross: an error.
    ErrorCircle,
    /// A gear: settings.
    Settings,
    /// The GitHub mark.
    Github,
    /// Artwork the application supplies: the source of an SVG drawn with
    /// `currentColor`. It is fitted to the icon's square without distortion
    /// and tinted like any other icon, so a brand mark or a product glyph
    /// follows the theme.
    Custom(&'static str),
    /// A speech bubble with three dots: a conversation.
    Chat,
    /// An outlined folder.
    Folder,
    /// A laptop.
    Laptop,
    /// A globe.
    Globe,
    /// A git branch. The bundled host draws the Git logo.
    GitBranch,
    /// Four squares: a grid of items.
    Grid,
    /// A painter's palette.
    Palette,
    /// A group of people.
    Users,
    /// A puzzle piece.
    Puzzle,
    /// An open book.
    Book,
    /// Stacked drives: a server.
    Server,
    /// A lightning bolt.
    Bolt,
    /// A bar chart.
    ChartBar,
    /// A keyboard.
    Keyboard,
    /// A hash sign.
    Hash,
    /// A slash between angle brackets: code.
    Code,
    /// A terminal window with a prompt.
    Terminal,
    /// An arrow pointing up.
    ArrowUp,
    /// Send a message. The bundled host draws the same arrow as
    /// [`Self::ArrowUp`].
    Send,
    /// An arrow pointing down.
    ArrowDown,
    /// An arrow pointing left.
    ArrowLeft,
    /// An arrow pointing right.
    ArrowRight,
    /// A chevron pointing down.
    ChevronDown,
    /// A chevron pointing up: a menu that is open, where a closed one
    /// shows [`Self::ChevronDown`]. A section that opens in place shows
    /// [`Self::ChevronDown`] when open and [`Self::ChevronRight`] when closed.
    ChevronUp,
    /// A chevron pointing right.
    ChevronRight,
    /// A magnifying glass: search.
    Search,
    /// An outlined square: stop.
    Stop,
    /// A solid rounded square: stop, on a filled button.
    StopFill,
    /// An outlined triangle pointing right: play.
    Play,
    /// Two bars: pause.
    Pause,
    /// A clock inside a counterclockwise arrow: history.
    History,
    /// A thin cross: close or dismiss.
    Close,
    /// A shorter, heavier cross than [`Self::Close`].
    CloseBold,
    /// A circled cross: clear or remove. The bundled host draws it as
    /// [`Self::ErrorCircle`].
    CloseCircle,
    /// A clockwise circular arrow: reload or retry.
    Refresh,
    /// Three horizontal lines: a menu.
    Menu,
    /// Three dots in a row: further actions.
    More,
    /// A tilted pushpin: pin.
    Pin,
    /// A storage box: archive.
    Archive,
    /// A box with an arrow leaving it upwards.
    ArchiveUp,
    /// A box with an arrow leaving it up and to the right: a link that
    /// opens outside the application.
    ExternalLink,
    /// A waste bin: delete.
    Trash,
    /// A pencil: edit or rename.
    Edit,
    /// Two overlapping sheets: copy to the clipboard.
    Copy,
    /// A speech bubble holding three dots, its tail at the left.
    Unread,
    /// A funnel: filter.
    Filter,
    /// Lines of decreasing length beside a down arrow: sort.
    Sort,
    /// Two diagonal arrows pointing inwards: fit or shrink.
    Collapse,
    /// Two diagonal arrows pointing outwards: enlarge.
    Expand,
    /// Two arrows closing on a line: making something take less room.
    Compact,
    /// Four corners: showing something over the whole window.
    Fullscreen,
    /// A clock face.
    Clock,
    /// Filled counterparts, for the chosen item of a navigation list.
    FolderFill,
    /// The solid form of [`Self::Play`].
    PlayFill,
    /// The solid form of [`Self::Grid`].
    GridFill,
    /// The solid form of [`Self::Settings`].
    SettingsFill,
    /// The solid form of [`Self::Palette`].
    PaletteFill,
    /// The solid form of [`Self::Laptop`].
    LaptopFill,
    /// The solid form of [`Self::Terminal`].
    TerminalFill,
    /// The solid form of [`Self::Keyboard`].
    KeyboardFill,
    /// The solid form of [`Self::Chat`].
    ChatFill,
    /// The solid form of [`Self::Users`].
    UsersFill,
    /// The solid form of [`Self::Puzzle`].
    PuzzleFill,
    /// The solid form of [`Self::Server`].
    ServerFill,
    /// The solid form of [`Self::Book`].
    BookFill,
    /// The solid form of [`Self::Clock`].
    ClockFill,
    /// The solid form of [`Self::Bolt`].
    BoltFill,
    /// The solid form of [`Self::Archive`].
    ArchiveFill,
    /// The solid form of [`Self::ChartBar`].
    ChartBarFill,
    /// A bell: notifications.
    Bell,
    /// The solid form of [`Self::Bell`].
    BellFill,
    /// A bell struck through: notifications off.
    BellSlash,
    /// A solid bar and a triangle pointing left: the previous track.
    SkipBack,
    /// A solid triangle pointing right and a bar: the next track.
    SkipForward,
    /// The solid form of [`Self::Pause`].
    PauseFill,
    /// Two crossing arrows: shuffle.
    Shuffle,
    /// Two arrows in a loop: repeat.
    Repeat,
    /// The repeat loop with a 1: repeat one track.
    RepeatOne,
    /// A loudspeaker with sound waves: volume.
    Volume,
    /// A loudspeaker with a cross: muted.
    VolumeMuted,
    /// An outlined heart: like or favorite.
    Heart,
    /// The solid form of [`Self::Heart`].
    HeartFill,
    /// Two beamed notes: music, or a track.
    MusicNote,
    /// A note beside a list: a playlist.
    Playlist,
    /// A house: home.
    Home,
    /// A stack of frames: a collection.
    Library,
    /// Three faders: an equalizer, or any set of fine adjustments.
    Sliders,
    /// A microphone with a slash: the microphone is off.
    MicrophoneMuted,
    /// Headphones: what is heard, or listening.
    Headphones,
    /// A smiling face: an emoji, or adding a reaction.
    Emoji,
    /// An arrow turning back: a reply.
    Reply,
    /// An at sign: a mention.
    At,
    /// A handset: a call.
    Phone,
    /// A handset with a cross: leaving or declining a call.
    HangUp,
    /// A monitor: a screen, or sharing one.
    Screen,
    /// A video camera with a slash: the camera is off.
    VideoOff,
    /// A closed padlock: private or locked.
    Lock,
    /// A diagonal stroke: neither on nor off, such as a setting left as
    /// it is inherited.
    Neutral,
    /// A person with a plus: inviting or adding someone.
    Invite,
    /// An arrow leaving a box to the left: leaving a group, or signing out.
    Leave,
    /// Two columns of dots: a handle to drag an item by.
    DragHandle,
    /// An empty ring: an option not chosen, as an unselected radio or check
    /// mark in a menu, beside [`Self::CheckCircle`] for the chosen one.
    Circle,
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
            Self::Pause => Self::PauseFill,
            Self::Heart => Self::HeartFill,
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

/// The face a text is measured and painted in. The host chooses the font
/// for each. The default is `Monospace`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FontFamily {
    /// The host's fixed-width face.
    #[default]
    Monospace,
    /// The host's proportional face, at its regular weight.
    Sans,
    /// Sans slanted by the host. It measures exactly as [`Self::Sans`].
    SansItalic,
    /// The medium and bold faces of the sans family: real faces the host
    /// provides, each measured as itself. A host without one paints Sans.
    SansMedium,
    /// The bold face of the sans family; see [`Self::SansMedium`].
    SansBold,
    /// [`Self::SansBold`] slanted by the host. It measures exactly as
    /// `SansBold`.
    SansBoldItalic,
}

/// One inline style run in a paragraph. Width is measured by the same host
/// text engine that paints it, so changing family or size does not require
/// callers to guess a character advance.
#[derive(Clone, Debug, PartialEq)]
pub struct TextSpan {
    /// The run's text. A line feed in it starts a new line.
    pub text: String,
    /// Ink of the text, and the tint of `icon`.
    pub color: Color,
    /// Text size in logical pixels. [`Self::new`] replaces one that is not
    /// finite or not above zero.
    pub font_size: f32,
    /// Face the run is measured and painted in.
    pub font_family: FontFamily,
    /// Fill behind the run, with rounded corners and 3 pixels of room on
    /// each side of the text. `None` paints none.
    pub background: Option<Color>,
    /// Optional inline action. Wrapped fragments share one keyboard target and
    /// dispatch `UiAction::Clicked` using their exact painted text bounds.
    pub action: Option<String>,
    /// Decorative leading glyph. It contributes width, never copyable text.
    pub icon: Option<Icon>,
    /// A picture painted in place of the text, such as a custom emoji; see
    /// [`Self::image`]. `None` paints the text.
    pub image: Option<InlineImage>,
    /// A line under the text, in its ink; see [`Self::underline`].
    pub underline: bool,
    /// A line through the text, in its ink; see [`Self::strikethrough`].
    pub strikethrough: bool,
    /// A hint shown over the run while the pointer rests on it; see
    /// [`Self::tooltip`]. `None` shows none.
    pub tooltip: Option<TextHint>,
}

/// The hint of a [`TextSpan`]: a title in a tooltip's colors.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHint {
    /// What the hint says. It wraps when it holds a line break.
    pub title: String,
    /// The colors it is painted with.
    pub colors: TooltipStyle,
}

/// A picture in a line of text, painted in place of its [`TextSpan`]'s
/// text. The text stays what is copied, selected and read aloud.
#[derive(Clone, Debug, PartialEq)]
pub enum InlineImage {
    /// A still picture, painted with [`DrawCommand::RasterImage`](crate::DrawCommand::RasterImage).
    Raster(RasterImage),
    /// A surface the host feeds by this ID, such as the current frame of an
    /// animation, painted with [`DrawCommand::Surface`](crate::DrawCommand::Surface)
    /// and fitted inside its square ([`SurfaceFit::Contain`]).
    Surface(String),
}

/// The side of an inline picture, as a multiple of its span's text size:
/// the size chat clients give custom emoji in a line of text.
pub const INLINE_IMAGE_SCALE: f32 = 1.375;

impl TextSpan {
    /// A run with no background, action or icon. A `font_size` that is not
    /// finite or not above zero becomes [`DEFAULT_FONT_SIZE`].
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
            image: None,
            underline: false,
            strikethrough: false,
            tooltip: None,
        }
    }

    /// Paint this color behind the run.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Make the run an inline action that reports `UiAction::Clicked` with
    /// this ID.
    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }

    /// Draw a decorative icon before the text. It adds to the width and is
    /// not part of the text that can be copied.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Paint `image` in place of the text: a square [`INLINE_IMAGE_SCALE`]
    /// times `font_size` on a side, centered on its line and never split
    /// across lines. The text, such as `:name:` for an emoji, is still what
    /// a selection copies and what accessibility reads; it is not painted.
    /// `background` and `icon` are not painted on such a run. A span with
    /// no text shows nothing.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(InlineImage::Raster(image));
        self
    }

    /// Paint the host-fed surface `id` in place of the text, as
    /// [`Self::image`] paints a picture: for an animation the host presents
    /// on that surface. The surface is fitted inside the square.
    pub fn surface(mut self, id: impl Into<String>) -> Self {
        self.image = Some(InlineImage::Surface(id.into()));
        self
    }

    /// Draw a line under the text, in its ink, one pixel thick for every
    /// 14 of `font_size`. It follows the text across wrapped lines and is
    /// not drawn under a picture or an icon.
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Draw a line through the middle of the text, in its ink, as thick as
    /// [`Self::underline`]'s.
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }

    /// Show `title` in a hint over the run while the pointer rests on it, in
    /// the colors of `palette`'s tooltips, as [`crate::Tooltip`] shows one
    /// over a control: after the same delay, above the part of the run under
    /// the pointer. A run with an [`Self::action`] also shows it while it has
    /// the keyboard focus. A blank title shows none.
    pub fn tooltip(mut self, title: impl Into<String>, palette: crate::ComponentPalette) -> Self {
        let title = title.into();
        self.tooltip = (!title.trim().is_empty()).then(|| TextHint {
            title,
            colors: palette.tooltip_style(),
        });
        self
    }
}

/// Paint-only emphasis in editable text, using Unicode character offsets.
/// Geometry, selection, clipboard and the stored value are unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct TextHighlight {
    /// Unicode character offsets into the text area's value, the end
    /// excluded.
    pub range: std::ops::Range<usize>,
    /// Ink the text in the range is repainted with.
    pub color: Color,
    /// Slant the range, as for text that is not committed yet.
    pub italic: bool,
    /// Fill behind the range, with rounded corners, such as a mention drawn
    /// as the pill it becomes once sent. It covers the characters' advances
    /// and the line's height. `None` paints none.
    pub background: Option<Color>,
}

/// The color of an outline and its width in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// Color of the stroke.
    pub color: Color,
    /// Stroke width in logical pixels. The stroke is drawn inside the
    /// element's bounds.
    pub width: f32,
}

/// Independent corner radii for backgrounds, solid borders and keyboard focus.
/// They affect paint and, with [`Element::clip_children`], the fills and
/// images of descendants; hit regions remain rectangular.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    /// Radius of the top left corner, in logical pixels.
    pub top_left: f32,
    /// Radius of the top right corner, in logical pixels.
    pub top_right: f32,
    /// Radius of the bottom right corner, in logical pixels.
    pub bottom_right: f32,
    /// Radius of the bottom left corner, in logical pixels.
    pub bottom_left: f32,
}

impl CornerRadii {
    /// The same radius on the four corners.
    pub const fn same(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Round the two top corners; the bottom ones stay square.
    pub const fn top(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: 0.0,
            bottom_left: 0.0,
        }
    }

    /// Round the two bottom corners; the top ones stay square.
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
    /// Color of the shadow. [`Element::shadow`] sets none when it is fully
    /// transparent.
    pub color: Color,
    /// How far the shadow is moved from the element's bounds.
    pub offset: Point,
    /// Blur radius; [`Self::new`] keeps it within 0..=64.
    pub blur: f32,
    /// Spread; [`Self::new`] keeps it within 0..=32.
    pub spread: f32,
}

/// Paint-only selection marker for a reusable tab strip. The group ID remains
/// stable when children are rebuilt or reordered.
#[derive(Clone, Debug)]
pub struct TabSelection {
    /// Stable ID under which the tree keeps where the indicator is.
    pub group_id: String,
    /// ID of the tab button, or of the [`Element::indicator_slot`] child, the
    /// indicator rests on. Without a match none is drawn.
    pub selected_id: String,
    /// Fill of the indicator, painted under the tabs.
    pub color: Color,
    /// Corner radius of the indicator, in logical pixels.
    pub radius: f32,
    /// Time the indicator takes to settle on a newly selected tab, in
    /// milliseconds. Zero moves it at once.
    pub animation_ms: u16,
}

impl BoxShadow {
    /// A shadow with its numbers made safe: an offset that is not finite
    /// becomes zero, `blur` is clamped to 0..=64 and `spread` to 0..=32, and
    /// either is zero when it is not finite.
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
    /// Painted except while the region is hovered; `presence` holds its ID.
    Concealed,
}

/// Rare paint effects, a row's wrapping, and the IDs under which the tree
/// keeps an element's ink and a column's children across rebuilds.
#[derive(Clone, Debug, Default)]
pub(crate) struct Kept {
    /// Stored off the common element path: most elements have no material.
    pub(crate) glass: Option<GlassMaterial>,
    pub(crate) foreground: Option<String>,
    pub(crate) leaving: Option<String>,
    /// The ID a soft focus is kept under, and its radius.
    pub(crate) soft: Option<(String, f32)>,
    /// The ID a paint scale is kept under, its factor, and the point it grows
    /// from as shares of the element's width and height.
    pub(crate) scale: Option<(String, f32, Point)>,
    /// Set by [`Element::wrap`]: a row moves children onto more lines.
    pub(crate) wrap: bool,
    /// Set by [`Element::line_gap`]; `None` takes the row's `gap`.
    pub(crate) line_gap: Option<f32>,
    /// Width a text input leaves free at its end, for a control laid over it.
    pub(crate) input_trailing_space: f32,
    /// A picture an icon button draws in its icon's place.
    pub(crate) leading_image: Option<crate::RasterImage>,
    /// Evenly spaced colors painted across the element, and whether they run
    /// down it rather than across.
    pub(crate) gradient: Option<(Arc<[Color]>, bool)>,
    /// Set by [`Element::foreground_gradient`]: colors a text's ink runs
    /// through from the left of each line to its right.
    pub(crate) ink_gradient: Option<Arc<[Color]>>,
    pub(crate) scrollbar_visibility: crate::ScrollbarVisibility,
    pub(crate) translation: Option<Point>,
    pub(crate) pointer_blocker: Option<String>,
}

/// Where a slider's value sits and how the pointer maps onto it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SliderTravel {
    pub(crate) fraction: f32,
    pub(crate) steps: usize,
    pub(crate) inset: f32,
    /// The share of the height from the top, for a slider over an area.
    pub(crate) y: Option<f32>,
    /// Measured up the height from the bottom instead of along the width.
    pub(crate) vertical: bool,
}

/// A transient visual description. Interaction state belongs to [`crate::UiTree`].
#[derive(Clone, Debug)]
pub struct Element {
    /// What the element is, with the content it holds.
    pub kind: ElementKind,
    /// Its layout and paint properties.
    pub style: Style,
    /// Another action whose pointer hover should also highlight this element.
    /// Useful when a row contains a more specific action such as its menu.
    pub hover_peer: Option<String>,
    /// Makes a clickable report [`UiAction::SliderChanged`](crate::UiAction::SliderChanged)
    /// instead of a click. Boxed: few elements have one.
    pub(crate) slider: Option<Box<SliderTravel>>,
    /// What the element says of itself to the platform, over what the tree
    /// works out. Boxed: few elements say anything.
    pub(crate) access: Option<Box<crate::Access>>,
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
    pub(crate) drag_in_place: bool,
    pub(crate) cursor: Option<Cursor>,
    pub(crate) sticky_top: Option<StickyTop>,
    /// Names a passive element for `UiTree::reveal` and `anchor_offset`.
    pub(crate) anchor: Option<String>,
    /// Set by [`Element::animate_height`]: the height follows `presence`'s
    /// ID, and whether it also opens from zero. One byte of existing padding.
    pub(crate) animate_height: Option<bool>,
    /// Rare paint and retained effects, including [`Element::glass`]. Boxed
    /// together so ordinary elements do not grow in cached trees.
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
    /// Stable ID under which the tree keeps the scroll position; the one
    /// `UiTree::scroll_offset` takes.
    pub id: String,
    /// Number of rows. `build_row` is called with indices below it.
    pub row_count: usize,
    /// Height of every row in logical pixels. [`Element::uniform_list`] makes
    /// it at least 1.
    pub row_height: f32,
    /// Builds the row at a zero-based index, on every layout that shows it.
    pub build_row: Arc<dyn Fn(usize) -> Element + Send + Sync>,
    /// Scrollbar policy; set by [`Element::scrollbar_appearance`].
    pub appearance: ScrollbarAppearance,
    /// How a wheel step is followed; set by [`Element::scroll_motion`].
    pub motion: ScrollMotion,
    /// What one row opens under itself: see [`Element::list_expansion`].
    pub expansion: Option<(usize, Arc<Element>)>,
    /// Rows of their own heights, measured as they come into view, instead
    /// of `row_height` each: see [`Element::measured_list`]. `row_height` is
    /// then the estimate for a row not measured yet.
    pub measured: Option<MeasuredRows>,
}

/// What makes a [`UniformList`] a list of rows of their own heights. The
/// tree measures each row it builds, keeps the height under the row's key,
/// and holds the row at the top of the viewport still when rows above it are
/// measured, grow, shrink, or are added.
#[derive(Clone)]
pub struct MeasuredRows {
    /// The stable key of the row at a zero-based index. The height a row was
    /// measured at, and the place the view holds, follow the key when rows
    /// are added or removed before it. Hash a key that is not a number.
    pub key: Arc<dyn Fn(usize) -> u64 + Send + Sync>,
    /// While the list is scrolled to its end, keep it there when rows are
    /// added at the end or the last ones grow: a conversation. Set by
    /// [`Element::list_follow_end`].
    pub follow_end: bool,
}

impl std::fmt::Debug for UniformList {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("UniformList")
            .field("id", &self.id)
            .field("row_count", &self.row_count)
            .field("row_height", &self.row_height)
            .field("measured", &self.measured.is_some())
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
    /// Stable ID; with each part's `key`, the name the tree keeps that
    /// part's count under.
    pub id: String,
    /// Cells in a row.
    pub columns: u16,
    /// Rows of cells. Cells past the last one are not drawn.
    pub rows: u16,
    /// Side of a cell and the space between two.
    pub cell: f32,
    /// Space between two cells, in logical pixels.
    pub gap: f32,
    /// Corner radius of each cell, in logical pixels.
    pub radius: f32,
    /// The parts, in the order they take their cells.
    pub parts: Vec<UnitGridPart>,
    /// Color of a cell that no part takes.
    pub empty: Color,
    /// Index of the part that stands out, if one does.
    pub focus: Option<usize>,
}

/// One part of a [`UnitGrid`]: how many cells it takes and their color.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitGridPart {
    /// Names the part across rebuilds, whatever its position.
    pub key: String,
    /// How many cells it takes; below zero counts as zero. A cell taken in
    /// part shows that share of the color.
    pub cells: f32,
    /// Color of its cells.
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

/// A row of level bars, as many as fit its width: one slot per `bar + gap`,
/// the newest level in the last slot and the ones before it in the slots
/// before. A slot without a level, or with a level of zero, is a dot `bar`
/// wide in `rest`; a level of one stands the element's whole height in
/// `color`. UiTree keeps each slot's height by `id` and springs it to a new
/// level, so a level that moves on to the next slot is followed there. A bar
/// rises at the pace of a hover and falls back over two moves: a peak shows
/// at once and does not flicker on its way down.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelBars {
    /// Stable ID under which the tree keeps each slot's height.
    pub id: String,
    /// From 0 to 1, oldest first.
    pub levels: Vec<f32>,
    /// Width of a bar and the space between two.
    pub bar: f32,
    /// Space between two bars, in logical pixels.
    pub gap: f32,
    /// Color of a bar that stands up.
    pub color: Color,
    /// Color of the dot a slot at rest shows.
    pub rest: Color,
}

/// One value of a [`BarSeries`]: a bar, or a segment of a stacked one.
#[derive(Clone, Debug, PartialEq)]
pub struct BarValue {
    /// What the bar measures. Not finite counts as zero; a negative one grows the other way.
    pub value: f64,
    /// A color of its own, in place of its series'.
    pub color: Option<Color>,
    /// What is written for the value, in place of the number.
    pub label: Option<String>,
    /// What the tooltip calls it, in place of its series' name.
    pub name: Option<String>,
}

/// One series of a [`BarPlot`]: a value for each category, a name and a color.
#[derive(Clone, Debug, PartialEq)]
pub struct BarSeries {
    /// Said beside its values in the plot's tooltip. Empty for a plot of one unnamed series.
    pub name: String,
    /// Fill of its bars.
    pub color: Color,
    /// One for each category, in their order; a missing one counts as zero.
    pub values: Vec<BarValue>,
}

/// Bars over categories, drawn and kept by the tree: a value axis with round
/// ticks, a dashed grid, and one bar for each series of each category, side
/// by side or stacked. UiTree keeps the plot's motion by `id`: the bars grow
/// from the zero line when the plot first appears, and under the pointer the
/// category it is over is lit by a band that glides from one to the next
/// while the others recede, and a tooltip that follows the pointer lists its
/// values.
#[derive(Clone, Debug, PartialEq)]
pub struct BarPlot {
    /// Stable ID under which the tree keeps the plot's motion and its hover.
    pub id: String,
    /// The name of each category, along the band axis.
    pub categories: Vec<String>,
    /// The series, in the order they are drawn and listed.
    pub series: Vec<BarSeries>,
    /// True to pile a category's bars on one another; false to set them side by side.
    pub stacked: bool,
    /// True for bars that run from left to right, with the categories down the side.
    pub horizontal: bool,
    /// True to fade each bar from a third of its color at its foot to all of it at its tip.
    pub gradient: bool,
    /// Radius of a bar's tip, in logical pixels. Its foot is square.
    pub radius: f32,
    /// The widest a bar is drawn, in logical pixels.
    pub max_bar: f32,
    /// True to draw a dashed line at each tick of the value axis.
    pub grid: bool,
    /// True to write the ticks of the value axis.
    pub value_axis: bool,
    /// True to write each bar's value at its tip.
    pub value_labels: bool,
    /// Ink of value labels.
    pub text: Color,
    /// Ink of the ticks and of the categories' names.
    pub muted: Color,
    /// Color of the grid and of the zero line.
    pub line: Color,
    /// Colors of the tooltip.
    pub tooltip: TooltipStyle,
    /// The name of a last row that sums a stacked category in the tooltip, if it has one.
    pub total: Option<String>,
}

/// One series of a [`LinePlot`] or a [`RadarPlot`]: a name, a color and a value
/// for each point or each axis.
#[derive(Clone, Debug, PartialEq)]
pub struct PlotSeries {
    /// Said beside its values in the plot's tooltip.
    pub name: String,
    /// Color of its line, of its dots and of its fill.
    pub color: Color,
    /// One for each point, in their order. A missing or non-finite one counts as zero.
    pub values: Vec<f64>,
}

/// Lines over points along an axis, drawn and kept by the tree: a value axis
/// with round ticks, a dashed grid and a line for each series, with the area
/// under it filled if asked. UiTree keeps the plot's motion by `id`: the lines
/// are drawn from left to right when the plot first appears, and under the
/// pointer a line marks the nearest point and glides to the next, with a dot
/// on each series and a tooltip that follows the pointer and lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct LinePlot {
    /// Stable ID under which the tree keeps the plot's motion and its hover.
    pub id: String,
    /// The name of each point along the axis.
    pub points: Vec<String>,
    /// The series, in the order they are drawn and listed.
    pub series: Vec<PlotSeries>,
    /// True to round each line through its points; false to join them with straight strokes.
    pub smooth: bool,
    /// True to fill under each line, from its color down to nothing at the zero line.
    pub area: bool,
    /// True to draw a dot at every point of every line.
    pub dots: bool,
    /// True to draw a dashed line at each tick of the value axis.
    pub grid: bool,
    /// True to write the ticks of the value axis.
    pub value_axis: bool,
    /// True to write the names of the points under the plot.
    pub point_axis: bool,
    /// A fixed value axis from the first to the second, with ticks at its ends
    /// and at zero, instead of one fitted to the values.
    pub range: Option<(f64, f64)>,
    /// True to dash the zero line.
    pub dashed_zero: bool,
    /// A point marked on every line with a dot, as the pointer marks one.
    pub mark: Option<usize>,
    /// Ink of the pointer's line.
    pub text: Color,
    /// Ink of the ticks and of the points' names.
    pub muted: Color,
    /// Color of the grid and of the zero line.
    pub line: Color,
    /// What is behind the plot: the ring around a dot under the pointer.
    pub ground: Color,
    /// Colors of the tooltip.
    pub tooltip: TooltipStyle,
}

/// One cell of a [`HeatPlot`].
#[derive(Clone, Debug, PartialEq)]
pub struct HeatCell {
    /// Fill of the cell.
    pub color: Color,
    /// What the plot's tooltip says of it. Empty for a cell with no tooltip.
    pub name: String,
}

/// A grid of square cells, one column after another, drawn and kept by the
/// tree. UiTree keeps the plot's motion by `id`: the columns come in one
/// after another when the plot first appears, and the cell under the pointer
/// grows over its neighbours while the others recede, with a tooltip that
/// follows the pointer and says what the cell is.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatPlot {
    /// Stable ID under which the tree keeps the plot's motion and its hover.
    pub id: String,
    /// One vector for each column, top row first. `None` is a hole.
    pub columns: Vec<Vec<Option<HeatCell>>>,
    /// Side of a cell, in logical pixels.
    pub side: f32,
    /// Space between two cells, in logical pixels.
    pub gap: f32,
    /// Colors of the tooltip.
    pub tooltip: TooltipStyle,
}

/// One slice of a [`PiePlot`].
#[derive(Clone, Debug, PartialEq)]
pub struct PieSlice {
    /// Said in the plot's tooltip.
    pub name: String,
    /// Its share of the whole, as any positive number. Negative or non-finite counts as zero.
    pub value: f64,
    /// Fill of the slice.
    pub color: Color,
    /// What is written for the value in the tooltip, in place of the number.
    pub label: Option<String>,
}

/// A circle shared out among slices, or a ring of them, drawn and kept by the
/// tree. UiTree keeps the plot's motion by `id`: the slices sweep round from
/// twelve o'clock when the plot first appears, and the one under the pointer
/// stands out from the others, which recede, while a tooltip that follows the
/// pointer says its value and its share.
#[derive(Clone, Debug, PartialEq)]
pub struct PiePlot {
    /// Stable ID under which the tree keeps the plot's motion and its hover.
    pub id: String,
    /// The slices, clockwise from twelve o'clock.
    pub slices: Vec<PieSlice>,
    /// The hole in the middle as a share of the radius, from 0 for a whole pie to 0.9.
    pub hole: f32,
    /// What is written in the middle of a ring, over `caption`. Empty for nothing.
    pub title: String,
    /// What is written under `title` in the middle of a ring. Empty for nothing.
    pub caption: String,
    /// Ink of the title.
    pub text: Color,
    /// Ink of the caption.
    pub muted: Color,
    /// Colors of the tooltip.
    pub tooltip: TooltipStyle,
}

/// Series over axes that spread from a center, drawn and kept by the tree:
/// rings and spokes for a grid, and a shape for each series whose corners
/// stand on the axes at the series' values. UiTree keeps the plot's motion by
/// `id`: the shapes open from the center when the plot first appears, and the
/// axis nearest the pointer is marked with a dot on each series and listed in
/// a tooltip that follows the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct RadarPlot {
    /// Stable ID under which the tree keeps the plot's motion and its hover.
    pub id: String,
    /// The name of each axis, clockwise from twelve o'clock.
    pub axes: Vec<String>,
    /// The series, in the order they are drawn and listed.
    pub series: Vec<PlotSeries>,
    /// The value at the rim. `None` takes a round number over the greatest value.
    pub maximum: Option<f64>,
    /// Ink of the pointer's marks.
    pub text: Color,
    /// Ink of the axes' names.
    pub muted: Color,
    /// Color of the rings and the spokes.
    pub line: Color,
    /// What is behind the plot: the ring around a dot under the pointer.
    pub ground: Color,
    /// Colors of the tooltip.
    pub tooltip: TooltipStyle,
}

/// A column of short marks, one for each part of a long view, such as the
/// turns of a conversation. The mark under the pointer or the keyboard focus
/// grows to `peak` and its neighbours step down to `rest` over `reach` marks,
/// each on a spring UiTree keeps by stable ID. That mark shows its title and
/// preview in a tooltip at once, and reports a click like a button. Up and
/// Down move the focus between marks and activate the one they reach.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkRail {
    /// Stable ID under which the tree keeps each mark's length.
    pub id: String,
    /// The marks, from top to bottom.
    pub marks: Vec<RailMark>,
    /// Index of the mark for what the view shows now, if one is.
    pub active: Option<usize>,
    /// Height each mark takes; less when they do not all fit.
    pub pitch: f32,
    /// Thickness of a mark in logical pixels, never more than the height it
    /// takes.
    pub thickness: f32,
    /// Length of a mark at rest, in logical pixels. The `active` mark rests
    /// three tenths of the way to `peak`.
    pub rest: f32,
    /// Length of the mark under the pointer or the focus, in logical pixels;
    /// also the rail's natural width.
    pub peak: f32,
    /// How many marks away the last raised neighbour is.
    pub reach: u8,
    /// Color of a mark at rest.
    pub ink: Color,
    /// Color of the `active` mark at rest.
    pub active_ink: Color,
    /// Color a mark takes as it reaches `peak`.
    pub lead_ink: Color,
    /// Colors of the tooltip the raised mark shows.
    pub tooltip: TooltipStyle,
}

/// One mark of a [`MarkRail`], with the title and preview its tooltip
/// shows.
#[derive(Clone, Debug, PartialEq)]
pub struct RailMark {
    /// Reported as `UiAction::Clicked`.
    pub id: String,
    /// Title of the mark's tooltip.
    pub title: String,
    /// Muted text under the title. Empty shows the title alone.
    pub preview: String,
}

impl MarkRail {
    /// Length a mark tends to, `distance` marks from the raised one.
    pub(crate) fn length(&self, index: usize, lead: Option<usize>) -> f32 {
        let rest = if self.active == Some(index) {
            self.rest + (self.peak - self.rest) * 0.3
        } else {
            self.rest
        };
        let Some(lead) = lead else { return rest };
        let steps = f32::from(self.reach) + 1.0;
        let lift = (1.0 - index.abs_diff(lead) as f32 / steps).max(0.0);
        rest.max(self.rest + (self.peak - self.rest) * lift)
    }
}

/// How a surface's pixels fill the rectangle laid out for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SurfaceFit {
    /// At the density they were rendered for, from the top-left corner,
    /// clipped to the rectangle and never scaled: a remote screen, a page.
    #[default]
    Native,
    /// Scaled up or down to fit inside the rectangle, centered, keeping its
    /// proportions: as large as a picture is shown in the same rectangle.
    Contain,
    /// Filling the rectangle, cropped around the center, as a cover picture
    /// ([`RasterImage::cover`](crate::RasterImage::cover)) is.
    Cover,
}

/// What an element is: a container, a control or passive content. The
/// constructors of [`Element`] build each kind.
#[derive(Clone, Debug)]
pub enum ElementKind {
    /// Children from top to bottom; see [`Element::column`].
    Column(Vec<Element>),
    /// Children side by side; see [`Element::row`].
    Row(Vec<Element>),
    /// A row of related, enabled tab actions. Arrow keys move between these
    /// actions and activate the newly focused tab.
    TabStrip {
        /// The tabs and whatever sits among them, laid out as a row.
        children: Vec<Element>,
        /// IDs of the enabled tabs, in the order the arrow keys walk them.
        action_ids: Vec<String>,
        /// The sliding selection surface; set by [`Element::tab_selection`].
        selection: Option<TabSelection>,
        /// True when Up and Down step it as Left and Right do; set by
        /// [`Element::every_arrow`].
        every_arrow: bool,
    },
    /// A box around one child, placed inside this element's padding; see
    /// [`Element::panel`].
    Panel(Box<Element>),
    /// Children laid one over another in the same box; see
    /// [`Element::stack`].
    Stack(Vec<Element>),
    /// One interactive surface whose children can contain more specific
    /// controls. Child hit regions take precedence over this row's region.
    Clickable {
        /// Reported as `UiAction::Clicked`, with `UiAction::SliderChanged`
        /// after [`Element::slider`] or with `UiAction::AreaSliderChanged`
        /// after [`Element::area_slider`]; also its focus ID.
        id: String,
        /// The content the surface wraps.
        child: Box<Element>,
    },
    /// Paints a floating subtree above the ordinary scene. It shares the
    /// base element's bounds, but may escape clips created by ancestors.
    Overlay {
        /// Laid out in this element's box; it alone gives it its size.
        base: Box<Element>,
        /// Placed in the same box and painted after the ordinary scene, or in
        /// its turn when this element has `clip_children`.
        floating: Box<Element>,
    },
    /// A floating surface tied to its anchor's actual laid-out rectangle.
    /// A transparent dismissal region is inserted behind the surface.
    Popover {
        /// Laid out in the flow as if the popover were not there; the surface
        /// is placed against its rectangle.
        anchor: Box<Element>,
        /// The surface, painted above the ordinary scene and never wider than
        /// the viewport less 16 pixels.
        floating: Box<Element>,
        /// Stable ID of the surface. The tree reports it as `UiAction::Clicked`
        /// for the host to close it: on a click of the backdrop, or on Escape
        /// while the surface has the keyboard.
        dismiss_id: String,
        /// Preferred side and alignment against the anchor.
        placement: PopoverPlacement,
        /// Distance between anchor and surface in logical pixels, 5 by default;
        /// see [`Element::popover_gap`].
        gap: f32,
        /// Duration of the entrance in milliseconds; zero shows the surface at
        /// once. The `enter_ms` motion token by default.
        animation_ms: u16,
        /// Duration of the exit in milliseconds. `None` takes `animation_ms`, up
        /// to the `exit_ms` motion token.
        close_animation_ms: Option<u16>,
        /// Scale the entrance starts from and the exit ends at; 1 does not
        /// scale. [`Element::popover_open_scale`] keeps it within 0.5..=1.25.
        open_scale: f32,
        /// Names the view the surface shows. When it changes under the same
        /// `dismiss_id`, the old content cross-fades into the new.
        view_key: Option<String>,
        /// Show the surface only while its trigger or its own content is
        /// hovered or focused; set by [`Element::popover_on_hover`].
        hover_open: bool,
        /// Whether a region over the whole viewport sits behind the surface and
        /// keeps the pointer from what is under it.
        backdrop: bool,
        /// Whether this surface owns menu focus and keyboard navigation.
        focus_scope: bool,
        /// Whether a click on the backdrop reports `dismiss_id`. Without it the
        /// backdrop only blocks.
        backdrop_dismiss: bool,
        /// Color painted over the viewport behind the surface, fading with it.
        /// With one the surface is modal for keyboard focus.
        scrim: Option<Color>,
        /// Control of the surface that takes the focus when it opens; without
        /// one a modal focuses its first control.
        initial_focus: Option<String>,
        /// Control that takes the focus back when a modal closes; `None` is the
        /// one that had it before.
        return_focus: Option<String>,
        /// Anchors the surface to existing controls instead of `anchor`.
        coach: Option<Box<CoachAnchor>>,
        /// The actions the Left and Right arrow keys report while this
        /// surface has the keyboard: what goes back and what goes on.
        arrows: Option<Box<(Option<String>, Option<String>)>>,
        /// A point of the window the surface is placed against instead of
        /// the anchor's box; set by [`Element::popover_at`].
        at: Option<Point>,
    },
    /// A transient, non-interactive overlay whose anchor remains in normal
    /// layout. The `anchor_id` identifies an action or a passive hover target.
    Tooltip {
        /// The element the hint describes, laid out as if the tooltip were not
        /// there.
        anchor: Box<Element>,
        /// ID whose hover or keyboard focus shows the hint. When no control in
        /// `anchor` has it, the anchor's box becomes a hover target under it.
        anchor_id: String,
        /// First line. Shortened to one line unless it holds a line break or
        /// there are sections; then it wraps.
        title: String,
        /// Keys drawn as key caps at the title's right. Left out when they
        /// would leave the title less than 24 pixels.
        shortcut: Option<String>,
        /// Blocks under the title, from top to bottom.
        sections: Vec<TooltipSection>,
        /// Widest the hint may be in logical pixels: at least 32, and never
        /// more than the viewport less 16.
        max_width: f32,
        /// Preferred side of the anchor.
        placement: TooltipPlacement,
        /// The colors it is painted with.
        colors: TooltipStyle,
    },
    /// A vertical scrolling viewport over one child; see [`Element::scroll`].
    Scroll {
        /// Stable ID under which the tree keeps the scroll position; the one
        /// `UiTree::scroll_offset` takes.
        id: String,
        /// The content, laid out at the viewport's width.
        child: Box<Element>,
        /// Scrollbar policy; set by [`Element::scrollbar_appearance`].
        appearance: ScrollbarAppearance,
        /// How a wheel step is followed; set by [`Element::scroll_motion`].
        motion: ScrollMotion,
    },
    /// A clipped horizontal viewport; selection changes reveal the target.
    HorizontalScroll {
        /// Stable ID under which the tree keeps the scroll position.
        id: String,
        /// The content, laid out at its natural width.
        child: Box<Element>,
        /// ID of a control inside to bring into view when it changes; set by
        /// [`Element::reveal_horizontal_item`].
        reveal: Option<String>,
        /// Tab strips translate a vertical wheel into horizontal motion;
        /// two-dimensional document canvases leave it to their vertical parent.
        vertical_wheel: bool,
        /// Let a drag pan the content and show a thin bar where it sits; set by
        /// [`Element::drag_to_pan`].
        drag_pan: bool,
    },
    /// A virtual list of rows of one height; see [`UniformList`].
    UniformList(UniformList),
    /// A code editor; see `CodeEditor`.
    CodeEditor(crate::CodeEditor),
    /// A code diff view; see `CodeDiff`.
    CodeDiff(crate::CodeDiff),
    /// A passive diagram at its natural size, measured with the tree's text.
    Flowchart(crate::Flowchart),
    /// A passive sequence diagram at its natural size.
    SequenceDiagram(crate::SequenceDiagram),
    /// Plain text wrapped to the width available. The pointer can select
    /// it.
    Text(String),
    /// Passive, individually measured key caps. A bounded shortcut is omitted
    /// as a group when it cannot fit; a partial chord would be misleading.
    KeyboardShortcut {
        /// The shortcut as text, such as `Ctrl+K`. A space separates the chords
        /// of a sequence.
        keys: String,
        /// Fill of the key caps. Not used by a `menu` shortcut.
        fill: Color,
        /// Width in logical pixels over which the whole shortcut is left out;
        /// `None` leaves only the room it is given as a limit.
        max_width: Option<f32>,
        /// Draw a plain accelerator label in the foreground instead of key
        /// caps; set by [`Element::menu_shortcut`].
        menu: bool,
    },
    /// Passive keys, alternatives and caption that fit or disappear together.
    ShortcutHint {
        /// The hint, measured at its natural width.
        child: Box<Element>,
        /// Width in logical pixels over which the child is not drawn; `None`
        /// leaves only the room it is given as a limit.
        max_width: Option<f32>,
    },
    /// A single measured line for navigation labels. It uses an ellipsis when
    /// the available width is shorter than the painted text.
    TruncatedText(String),
    /// A measured single line whose overflowing right edge fades to transparent.
    /// The title remains unchanged; fading affects paint, never hit regions.
    FadedText {
        /// The line. It is never wrapped or shortened.
        text: String,
        /// Width of the fade at the right edge in logical pixels, used only
        /// when the text overflows.
        fade_width: f32,
    },
    /// Text with a paint-only moving highlight. Wrapped paragraphs remain
    /// selectable; a truncated label is passive and stays on one measured line.
    ShimmerText {
        /// Stable ID under which the tree keeps when the sweep began.
        id: String,
        /// The text, painted in the foreground.
        text: String,
        /// Color of the moving highlight. Under reduced motion there is none.
        highlight: Color,
        /// One line ended with an ellipsis instead of wrapped, selectable text.
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
        /// Names the pixels; the frame carries it in `DrawCommand::Surface`.
        id: String,
        /// Names the surface for assistive technology.
        label: String,
        /// How the pixels fill the rectangle; see [`Element::surface_fit`].
        fit: SurfaceFit,
    },
    /// An icon centered in the element's box, [`ICON_SIZE`] on a side unless
    /// [`Style::icon_side`] says otherwise.
    Icon(Icon),
    /// A text button; see [`Element::button`].
    Button {
        /// Reported as `UiAction::Clicked`; also its focus ID.
        id: String,
        /// The text it shows.
        label: String,
        /// Icon at the far edge; set by [`Element::trailing_icon`].
        trailing_icon: Option<Icon>,
    },
    /// An icon and an optional label in one control; see
    /// [`Element::icon_button`].
    IconButton {
        /// Reported as `UiAction::Clicked`; also its focus ID.
        id: String,
        /// The icon, drawn [`ICON_SIZE`] on a side.
        icon: Icon,
        /// Text after the icon. Empty makes an icon-only button.
        label: String,
        /// Second icon at the far edge; set by [`Element::trailing_icon`].
        trailing_icon: Option<Icon>,
    },
    /// A single-line text field; see [`Element::text_input`].
    TextInput {
        /// Stable ID under which the tree keeps the value, the caret and the
        /// selection; the one `UiTree::input_value` takes.
        id: String,
        /// The value the first time this ID is laid out. Later the tree's own
        /// value stands.
        initial_value: String,
        /// Shown while the value is empty and the field does not have the
        /// focus.
        placeholder: String,
        /// Keep the placeholder while the field is focused and still empty.
        persistent_placeholder: bool,
        /// Decorative icon before the text, inside the field; set by
        /// [`Element::leading_icon`].
        leading_icon: Option<Icon>,
        /// Tint of the leading icon while the field is not focused. `None`, or
        /// the focus, gives it the foreground.
        leading_icon_tint: Option<Color>,
        /// Keys drawn as key caps at the right edge while the field is empty
        /// and not focused, when they fit.
        shortcut: Option<String>,
        /// Fill of those key caps. `None` takes a faint neutral from the
        /// foreground.
        shortcut_fill: Option<Color>,
        /// Paint a bullet for each character and keep the value from being
        /// copied.
        password: bool,
    },
    /// A multi-line editor; see [`Element::text_area`].
    TextArea {
        /// Stable ID under which the tree keeps the value, the caret and the
        /// selection; the one `UiTree::input_value` takes.
        id: String,
        /// The value the first time this ID is laid out. Later the tree's own
        /// value stands.
        initial_value: String,
        /// Shown while the value is empty and the editor does not have the
        /// focus.
        placeholder: String,
        /// Maximum outer height when content determines the editor's height.
        auto_grow: Option<f32>,
        /// Keep CRLF/CR bytes in editable values instead of normalizing to LF.
        preserve_line_endings: bool,
        /// Enter submits and Shift+Enter breaks the line, as in a chat
        /// composer, instead of the other way round; see
        /// [`Element::text_area_enter_submits`].
        enter_submits: bool,
        /// Paint-only ranges over `initial_value`, 64 at most. None is painted
        /// while the edited value differs from it.
        highlights: Vec<TextHighlight>,
    },
    /// Passive loading placeholder. UiTree animates its paint by stable ID.
    Skeleton {
        /// Stable ID under which the tree keeps when the pulse began.
        id: String,
        /// Fill at rest, and the only one under reduced motion.
        base: Color,
        /// Fill the surface eases to and back from.
        highlight: Color,
    },
    /// Passive progress indicator. Its phase is retained by stable ID in UiTree.
    Spinner {
        /// Stable ID under which the tree keeps when it began to turn.
        id: String,
        /// Color of the strokes.
        color: Color,
        /// Which outline is drawn.
        variant: SpinnerVariant,
    },
    /// A record behind its sleeve. Its slide and its turn are retained by
    /// stable ID in UiTree.
    Record {
        /// Stable ID under which the tree keeps the slide and the angle.
        id: String,
        /// Picture on the sleeve. `None` fills the sleeve with the style's
        /// background.
        image: Option<RasterImage>,
        /// Whether the record is drawn out of the sleeve.
        out: bool,
        /// Whether the record turns. It does not under reduced motion.
        spinning: bool,
        /// A control whose hover also draws the record out.
        peek: Option<String>,
    },
    /// Passive finite progress. Unknown capacity has no filled arc.
    ProgressRing {
        /// Share of the ring that is filled, from 0 to 1. `None` fills no arc.
        fraction: Option<f32>,
        /// Color of the whole ring.
        track: Color,
        /// Color of the filled arc.
        fill: Color,
    },
    /// An interactive reasoning-effort track. `fraction` is the selected
    /// position; `steps` is the current model's number of available levels.
    EffortSlider {
        /// Reported with `UiAction::SliderChanged`; also its focus ID.
        id: String,
        /// The selected position, from 0 to 1.
        fraction: f32,
        /// Number of stops; [`Element::effort_slider`] makes it at least 2.
        steps: usize,
        /// The final advertised level is Ultra. Other providers can end at
        /// High or Max without receiving the Ultra treatment.
        ultra_available: bool,
    },
    /// A figure that counts from the value it showed to the one it is given.
    /// UiTree keeps the shown value by stable ID and lays the text out as it
    /// reads on each frame, so what follows it keeps to its side.
    RollingNumber {
        /// Stable ID under which the tree keeps the value it shows.
        id: String,
        /// The value to count to.
        value: f32,
        /// Writes the value shown on a frame as the text to draw.
        format: fn(f32) -> String,
    },
    /// A field of cells shared out among parts; see [`UnitGrid`].
    UnitGrid(UnitGrid),
    /// A row of level bars; see [`LevelBars`].
    LevelBars(LevelBars),
    /// A bar chart; see [`BarPlot`].
    BarPlot(Box<BarPlot>),
    /// A line or area chart; see [`LinePlot`].
    LinePlot(Box<LinePlot>),
    /// A pie or ring chart; see [`PiePlot`].
    PiePlot(Box<PiePlot>),
    /// A radar chart; see [`RadarPlot`].
    RadarPlot(Box<RadarPlot>),
    /// A grid of cells; see [`HeatPlot`].
    HeatPlot(Box<HeatPlot>),
    /// A column of marks that rise around the pointer; see [`MarkRail`].
    MarkRail(MarkRail),
    /// Nothing of its own to draw and no natural size; see
    /// [`Element::spacer`].
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

/// Layout and paint properties every kind of element has. The builder
/// methods of [`Element`] set them; lengths are in logical pixels.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    /// Fixed outer width, padding included. `None` leaves it to the content
    /// and the parent. Set by [`Element::size`].
    pub width: Option<f32>,
    /// Fixed outer height, padding included. `None` leaves it to the content
    /// and the parent. Set by [`Element::size`].
    pub height: Option<f32>,
    /// Lower bound of the measured width; zero by default.
    pub min_width: f32,
    /// Lower bound of the measured height; zero by default.
    pub min_height: f32,
    /// Upper bound for intrinsic width. An explicit minimum takes precedence.
    pub max_width: Option<f32>,
    /// Upper bound for intrinsic height. An explicit minimum takes precedence.
    pub max_height: Option<f32>,
    /// Space between the element's edges and its content, the same on the
    /// four sides. A button's applies to its left and right only.
    pub padding: f32,
    /// Space between the children of a row, a column or a tab strip, and
    /// between the icon, label and trailing icon of a button.
    pub gap: f32,
    /// Share of the space a row or column has left, among its flexible
    /// children. Zero, the default, keeps the natural size.
    pub flex: f32,
    /// In a row, give up width the row cannot spare instead of overflowing.
    pub shrink: bool,
    /// In a column, give up height the column cannot spare. Off by default;
    /// set by [`Element::shrink_height_to_fit`].
    pub shrink_height: bool,
    /// Fill of the element's bounds. `None` paints none.
    pub background: Option<Color>,
    /// Paint-only scrim above children, opaque at the top and transparent at
    /// its bottom edge. Useful where scrolling content passes under a header.
    pub top_fade: Option<(Color, f32)>,
    /// Paint-only scrim above children, transparent at its top edge and
    /// tinted at the bottom. Scroll areas show it only above hidden content.
    pub bottom_fade: Option<(Color, f32)>,
    /// Paint-only distances from the top and bottom edges to the corresponding
    /// fade bands. Opaque floating bars can occupy this space; defaults to zero.
    pub fade_insets: (f32, f32),
    /// Passive bottom background behind children, for dedicated footer space.
    pub bottom_background_fade: Option<(Color, f32)>,
    /// Passive top background behind children: the color at the top edge,
    /// transparent this many pixels below it.
    pub top_background_fade: Option<(Color, f32)>,
    /// Paint-only overflow cues at a horizontal scroll viewport's edges.
    pub horizontal_edge_fade: Option<(Color, f32)>,
    /// Background of a button, clickable or text field while the pointer is
    /// over it, and of a `hover_surface` while one of its actions is.
    pub hover_background: Option<Color>,
    /// Background while pressed. `None` falls back to `hover_background`.
    pub pressed_background: Option<Color>,
    /// Ink for an interactive control while hovered or pressed. Unlike a
    /// background treatment this never changes its hit bounds or geometry.
    pub hover_foreground: Option<Color>,
    /// Ink while pressed. `None` falls back to `hover_foreground`.
    pub pressed_foreground: Option<Color>,
    /// Border painted while an editor owns focus, including pointer focus,
    /// or while another control without a focus ring has the keyboard focus.
    pub focus_border: Option<Border>,
    /// Editors that paint their own caret may suppress the outer keyboard ring.
    pub focus_ring: bool,
    /// Action menus share one pointer/keyboard highlight instead of a button ring.
    pub action_menu_item: bool,
    /// Duration of interactive background transitions, in milliseconds.
    pub transition_ms: u16,
    /// Outline drawn inside the bounds; set by [`Element::border`] or
    /// [`Element::dashed_border`].
    pub border: Option<Border>,
    /// Draw `border` as dashes instead of a solid line.
    pub dashed_border: bool,
    /// Shadow painted under the background, rounded by `radius`. It does not
    /// change the layout.
    pub shadow: Option<BoxShadow>,
    /// Clip child paint and hit regions to the content rectangle. The parent
    /// background, border and shadow still paint at their normal bounds.
    pub clip_children: bool,
    /// Center lines of text, or a button's content, across the width instead
    /// of starting them at the left edge.
    pub center_content: bool,
    /// Underline the text of a button or clickable while it is hovered or
    /// has the keyboard focus.
    pub underline_on_hover: bool,
    /// Color of the text and icons, white by default. A scroll area's
    /// scrollbar takes it too.
    pub foreground: Color,
    /// Corner radius in logical pixels for the background, the border and
    /// the shadow. Zero is square.
    pub radius: f32,
    /// Radii for each corner, used instead of `radius` by the background, a
    /// solid border and the focus ring.
    pub corner_radii: Option<CornerRadii>,
    /// Surrounding surface painted outside rounded corners after children.
    /// This is passive paint; use `clip_children` for rectangular child bounds.
    pub corner_mask: Option<Color>,
    /// Text size in logical pixels; [`DEFAULT_FONT_SIZE`] by default.
    pub font_size: f32,
    /// Face of the text; `FontFamily::Monospace` by default.
    pub font_family: FontFamily,
    /// Minimum line box for formatted paragraphs, in logical pixels. Glyphs
    /// retain their natural metrics; controls and editors are unaffected.
    pub line_height: Option<f32>,
    /// Side of a standalone icon; `None` is the standard 16 px.
    pub icon_side: Option<f32>,
    /// Most lines a formatted text shows; set by [`Element::max_lines`].
    /// `None` shows them all.
    pub max_lines: Option<usize>,
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
            shrink_height: false,
            background: None,
            top_fade: None,
            bottom_fade: None,
            fade_insets: (0.0, 0.0),
            bottom_background_fade: None,
            top_background_fade: None,
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
            max_lines: None,
        }
    }
}

/// A length a builder was given: never negative, and `current` where it is
/// not a number a layout can use.
fn length(value: f32, current: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        current
    }
}

impl Element {
    /// A code editor; see `CodeEditor`.
    pub fn code_editor(editor: crate::CodeEditor) -> Self {
        Self::new(ElementKind::CodeEditor(editor))
    }
    /// A code diff view; see `CodeDiff`.
    pub fn code_diff(diff: crate::CodeDiff) -> Self {
        Self::new(ElementKind::CodeDiff(diff))
    }
    /// A passive flowchart at its natural size; see `Flowchart`.
    pub fn flowchart(flowchart: crate::Flowchart) -> Self {
        Self::new(ElementKind::Flowchart(flowchart))
    }
    /// A sequence diagram; see `SequenceDiagram`.
    pub fn sequence_diagram(diagram: crate::SequenceDiagram) -> Self {
        Self::new(ElementKind::SequenceDiagram(diagram))
    }
    fn new(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            hover_peer: None,
            slider: None,
            access: None,
            presence: None,
            replacement: None,
            hover_surface: None,
            indicator_slot: None,
            action_alias: None,
            menu_secondary_action: false,
            draggable: false,
            drag_in_place: false,
            cursor: None,
            sticky_top: None,
            anchor: None,
            animate_height: None,
            kept: None,
            hover_reveal: HoverReveal::None,
            drop_target: false,
            menu_navigation: false,
            context_menu: None,
        }
    }

    /// Name this element as a place to scroll to, without making it a
    /// control: `UiTree::reveal(id)` brings it into view and
    /// `UiTree::anchor_offset(id)` says where it is inside its scroll area.
    /// Static branches retain anchor geometry when clipped outside the viewport;
    /// a virtual row must have been built before its anchors can be located.
    pub fn anchor(mut self, id: impl Into<String>) -> Self {
        self.anchor = Some(id.into());
        self
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

    /// Lay children out from top to bottom, `gap` apart. A child with `flex`
    /// shares the height the others leave.
    pub fn column(children: Vec<Self>) -> Self {
        Self::new(ElementKind::Column(children))
    }

    /// Lay children out side by side, `gap` apart. A child with `flex` shares
    /// the width the others leave.
    pub fn row(children: Vec<Self>) -> Self {
        Self::new(ElementKind::Row(children))
    }

    /// A row of tabs. `action_ids` are the IDs of its enabled tabs: arrow keys
    /// move the focus between them and activate the tab they reach.
    pub fn tab_strip(children: Vec<Self>, action_ids: Vec<String>) -> Self {
        Self::new(ElementKind::TabStrip {
            children,
            action_ids,
            selection: None,
            every_arrow: false,
        })
    }

    /// Let Up and Down step a tab strip as Left and Right do, whichever way
    /// it is laid out: what a group of choices does, where a strip of tabs
    /// leaves Up and Down to the page.
    pub fn every_arrow(mut self) -> Self {
        if let ElementKind::TabStrip { every_arrow, .. } = &mut self.kind {
            *every_arrow = true;
        }
        self
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

    /// Translate this element and its descendants without changing their slot
    /// in the parent's flow. Paint, pointer, accessibility and scroll geometry
    /// move together; ancestor clips remain fixed. Non-finite offsets are ignored.
    /// The host owns the offset, including direct manipulation and animation.
    /// Viewport popovers retain their placement; translate their floating
    /// surface itself when it should move independently of its anchor.
    pub fn translate(mut self, offset: Point) -> Self {
        if offset.x.is_finite() && offset.y.is_finite() {
            self.kept.get_or_insert_default().translation = Some(offset);
        }
        self
    }

    /// Block presses from reaching elements beneath this element's visible
    /// bounds, while controls inside it keep their own targets. The unique, stable
    /// `id` names a passive hit region, with no action, focus or accessibility
    /// node. Clipping and translation apply normally. Scroll events still
    /// reach scroll areas underneath; the host owns touch gesture arbitration.
    pub fn block_pointer(mut self, id: impl Into<String>) -> Self {
        self.kept.get_or_insert_default().pointer_blocker = Some(id.into());
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

    /// Paint this element's text out of focus by `radius` logical pixels, the
    /// way lines away from the one being read are held back. The radius is
    /// kept under `id` and springs to a new one in one move, or changes at
    /// once under reduced motion. Zero is sharp; a radius that is not finite
    /// counts as zero and none goes over 8.
    pub fn soft_focus(mut self, id: impl Into<String>, radius: f32) -> Self {
        let radius = if radius.is_finite() { radius.clamp(0.0, 8.0) } else { 0.0 };
        self.kept.get_or_insert_default().soft = Some((id.into(), radius));
        self
    }

    /// Paint this element and what is inside it at `factor` times its size,
    /// growing from the middle of its left edge. Layout, hit targets and line
    /// breaks stay those of factor one, so the line being read can grow
    /// without re-wrapping or moving its neighbours. The factor is kept under
    /// `id` and springs to a new one in one move, or changes at once under
    /// reduced motion. A factor that is not finite or not above zero is ignored.
    pub fn animate_scale(self, id: impl Into<String>, factor: f32) -> Self {
        self.animate_scale_about(id, factor, Point::new(0.0, 0.5))
    }

    /// As [`Element::animate_scale`], growing from `anchor`, given as shares
    /// of the element's width and height: `(0.5, 0.5)` is its center. An
    /// anchor that is not finite is ignored with the rest.
    pub fn animate_scale_about(mut self, id: impl Into<String>, factor: f32, anchor: Point) -> Self {
        if factor.is_finite() && factor > 0.0 && anchor.x.is_finite() && anchor.y.is_finite() {
            self.kept.get_or_insert_default().scale = Some((id.into(), factor, anchor));
        }
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

    /// Fill the element with `stops` spread evenly from its left edge to its
    /// right edge, blending from each to the next, behind its children and
    /// over its background. The corners are rounded by its `radius`. One
    /// stop is a plain fill, and none paints nothing.
    pub fn horizontal_gradient(mut self, stops: impl Into<Arc<[Color]>>) -> Self {
        self.kept.get_or_insert_default().gradient = Some((stops.into(), false));
        self
    }

    /// The same as [`horizontal_gradient`](Self::horizontal_gradient), from
    /// the top edge to the bottom edge.
    pub fn vertical_gradient(mut self, stops: impl Into<Arc<[Color]>>) -> Self {
        self.kept.get_or_insert_default().gradient = Some((stops.into(), true));
        self
    }

    /// Let a tab strip's selection rest on this child when `id` is selected.
    pub fn indicator_slot(mut self, id: impl Into<String>) -> Self {
        self.indicator_slot = Some(id.into());
        self
    }

    /// A box around one child, as large as the child plus this element's
    /// padding. It carries a background, border or radius.
    pub fn panel(child: Self) -> Self {
        Self::new(ElementKind::Panel(Box::new(child)))
    }

    /// Make `child` one interactive surface with this ID. Controls inside it
    /// keep their own hit regions, which take precedence. Its hover and press
    /// backgrounds change over the hover duration of the motion tokens.
    pub fn clickable(id: impl Into<String>, child: Self) -> Self {
        Self::new(ElementKind::Clickable {
            id: id.into(),
            child: Box::new(child),
        })
        .transition_ms(crate::tokens().motion.hover_ms)
    }

    /// Turn a clickable into a slider. Pressing or dragging over it reports
    /// `SliderChanged` with the pointer's share of its width, measured
    /// between `inset` pixels from each end; arrows, Home and End move it
    /// from the keyboard. `steps` below 2 is continuous, otherwise the value
    /// snaps to that many evenly spaced stops. The element paints nothing of
    /// its own: the child draws the track at the `fraction` it is given.
    pub fn slider(mut self, fraction: f32, steps: usize, inset: f32) -> Self {
        self.slider = Some(Box::new(SliderTravel {
            fraction: if fraction.is_finite() { fraction.clamp(0.0, 1.0) } else { 0.0 },
            steps,
            inset: if inset.is_finite() { inset.max(0.0) } else { 0.0 },
            y: None,
            vertical: false,
        }));
        self
    }

    /// Turn a clickable into a slider that stands up: the same as
    /// [`Element::slider`], but the share is of its height measured from the
    /// bottom, so dragging up raises it, and Up and Down step it as well as
    /// Right and Left.
    pub fn vertical_slider(mut self, fraction: f32, steps: usize, inset: f32) -> Self {
        self = self.slider(fraction, steps, inset);
        if let Some(travel) = self.slider.as_mut() {
            travel.vertical = true;
        }
        self
    }

    /// Turn a clickable into a slider over its area. Pressing or dragging
    /// over it reports `AreaSliderChanged` with the pointer's share of its
    /// width from the left, `x`, and of its height from the top, `y`, each
    /// measured between `inset` pixels from its edges. Left and Right move
    /// `x` by a twentieth, Up and Down move `y`, and Home and End take `x`
    /// to its ends. Shares outside 0 to 1 are clamped and one that is not
    /// finite is 0. The element paints nothing of its own: the child draws
    /// the mark at `x` and `y`.
    pub fn area_slider(mut self, x: f32, y: f32, inset: f32) -> Self {
        let share = |share: f32| {
            if share.is_finite() {
                share.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        self.slider = Some(Box::new(SliderTravel {
            fraction: share(x),
            steps: 0,
            inset: if inset.is_finite() {
                inset.max(0.0)
            } else {
                0.0
            },
            y: Some(share(y)),
            vertical: false,
        }));
        self
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

    /// The shape the pointer takes over this button or clickable, and keeps
    /// for as long as a drag that started on it lasts. The tree reports it
    /// through `UiTree::cursor`; the host's backend applies it.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    /// Let a button or clickable start a drag once the pointer leaves the
    /// click tolerance. A press released inside it is still a click.
    pub fn draggable(mut self) -> Self {
        self.draggable = true;
        self
    }

    /// As [`draggable`](Self::draggable), without a copy of the element
    /// following the pointer: for a handle the host itself moves as the drag
    /// goes on, which stays where the host puts it.
    pub fn draggable_in_place(mut self) -> Self {
        self.draggable = true;
        self.drag_in_place = true;
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

    /// The opposite of [`reveal_on_hover`](Self::reveal_on_hover): painted
    /// except while the pointer is over the
    /// [`hover_region`](Self::hover_region) around it, fading the same way.
    /// In a [`stack`](Self::stack) with a revealed element, one gives way to
    /// the other. `id` must be unique in the window.
    pub fn conceal_on_hover(mut self, id: impl Into<String>) -> Self {
        self.presence = Some((id.into(), false));
        self.hover_reveal = HoverReveal::Concealed;
        self
    }

    /// Accept drops. Applies to buttons, clickables and scroll surfaces
    /// (including virtual lists), which are the elements with a pointer region.
    pub fn drop_target(mut self) -> Self {
        self.drop_target = true;
        self
    }

    /// Lay children one over another. Each takes the whole of the stack's
    /// box, which is as large as the largest of them unless it is sized.
    /// They are painted in the order given, the last on top, inside whatever
    /// clips the stack; where two take the pointer, the later one has it.
    pub fn stack(children: Vec<Self>) -> Self {
        Self::new(ElementKind::Stack(children))
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
                arrows: None,
                at: None,
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

    /// Let the Left and Right arrow keys report these actions while the
    /// surface has the keyboard: the previous and the next of a gallery, a
    /// carousel, a wizard. `None` for an end there is nothing beyond.
    pub fn popover_arrows(mut self, back: Option<String>, next: Option<String>) -> Self {
        if let ElementKind::Popover { arrows, .. } = &mut self.kind {
            *arrows = Some(Box::new((back, next)));
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

    /// Place the surface against a point of the window instead of the
    /// anchor's box, as a context menu opens where the secondary click was.
    /// The anchor keeps its place in the layout, and the arrow keys reach the
    /// surface wherever the focus is. A point that is not finite is ignored.
    pub fn popover_at(mut self, point: Point) -> Self {
        if let ElementKind::Popover { at, .. } = &mut self.kind {
            if point.x.is_finite() && point.y.is_finite() {
                *at = Some(point);
            }
        }
        self
    }

    /// Show a submenu while its trigger or content is hovered or focused,
    /// or a submenu nested in it is.
    pub fn popover_on_hover(mut self) -> Self {
        if let ElementKind::Popover {
            anchor,
            hover_open,
            backdrop,
            gap,
            ..
        } = &mut self.kind
        {
            // It opens under the pointer, which only the tree knows: whoever
            // built it cannot say whether it is open.
            if let Some(said) = anchor.access.as_mut() {
                said.expanded = None;
            }
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

    /// A scrolling viewport over `child`, with the default scrollbar appearance
    /// and wheel motion. `id` is the one `UiTree::scroll_offset` and the
    /// `scroll_to_*` methods take.
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
            expansion: None,
            measured: None,
        }))
    }

    /// A scrollable list whose rows each take the height they need, such as
    /// the messages of a conversation. Like [`Element::uniform_list`], only
    /// the rows in view are built, from their index. Each one is measured at
    /// the width of the list when it is built, and `estimated_row_height`
    /// stands for a row not measured yet, so the scrollbar is exact for the
    /// rows seen and close for the rest.
    ///
    /// `row_key` names the row at an index. Under its key the tree keeps the
    /// height it measured, and the row at the top of the viewport stays where
    /// it is when rows above it are measured, change height or are added: a
    /// page of older messages loaded above the ones being read does not move
    /// them. [`Element::list_follow_end`] keeps the end in view as rows
    /// arrive. A measured list has no [`Element::list_expansion`]: a row that
    /// opens grows itself. Set `.flex(1.0)` when it fills the remaining height
    /// of a column. An estimate that is not finite or under 1 is read as 1.
    pub fn measured_list(
        id: impl Into<String>,
        row_count: usize,
        estimated_row_height: f32,
        row_key: impl Fn(usize) -> u64 + Send + Sync + 'static,
        build_row: impl Fn(usize) -> Self + Send + Sync + 'static,
    ) -> Self {
        let mut list = Self::uniform_list(id, row_count, estimated_row_height, build_row);
        if let ElementKind::UniformList(list) = &mut list.kind {
            list.measured = Some(MeasuredRows {
                key: Arc::new(row_key),
                follow_end: false,
            });
        }
        list
    }

    /// Keep a [`Element::measured_list`] at its end while it is scrolled
    /// there: rows added at the end, or a last row that grows, scroll into
    /// view, the way a conversation follows new messages. Scrolled away from
    /// the end, the list holds its place. Does nothing on another element.
    pub fn list_follow_end(mut self, follow: bool) -> Self {
        if let ElementKind::UniformList(UniformList {
            measured: Some(measured),
            ..
        }) = &mut self.kind
        {
            measured.follow_end = follow;
        }
        self
    }

    /// Show `content` under row `index` of a uniform list, at the height it
    /// takes: what an item of the list opens in place. It scrolls with the
    /// rows, the rows after it move down by its height, and the list still
    /// builds only what is in view. One row is open at a time; pass
    /// `content` on every rebuild while it is. A [`crate::Collapse`] as
    /// `content` opens and closes it on the spring. Does nothing on an
    /// element that is not a uniform list.
    pub fn list_expansion(mut self, index: usize, content: Self) -> Self {
        if let ElementKind::UniformList(list) = &mut self.kind {
            list.expansion = Some((index, Arc::new(content)));
        }
        self
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

    /// Show an ordinary, virtual or horizontal scroll indicator only while
    /// active, or keep its idle appearance visible. Does not change layout or
    /// scrolling. A horizontal indicator requires [`Element::drag_to_pan`];
    /// hiding it never disables the viewport's pointer or scroll targets.
    pub fn scrollbar_visibility(mut self, visibility: crate::ScrollbarVisibility) -> Self {
        if matches!(self.kind, ElementKind::Scroll { .. } | ElementKind::UniformList(_) | ElementKind::HorizontalScroll { .. }) {
            self.kept.get_or_insert_default().scrollbar_visibility = visibility;
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

    /// Plain text in the element's foreground, font and size, wrapped to the
    /// width available.
    pub fn text(value: impl Into<String>) -> Self {
        Self::new(ElementKind::Text(value.into()))
    }

    /// Wrapped text with a paint-only highlight in `highlight` that moves
    /// across it. See `shimmer_truncated_text` for a single line.
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

    /// A passive ring filled up to `fraction`, which is clamped to 0..=1.
    /// `None`, or a fraction that is not finite, fills no arc.
    pub fn progress_ring(fraction: Option<f32>, track: Color, fill: Color) -> Self {
        Self::new(ElementKind::ProgressRing {
            fraction: fraction
                .filter(|value| value.is_finite())
                .map(|value| value.clamp(0.0, 1.0)),
            track,
            fill,
        })
    }

    /// Passive key caps for `keys`, filled with `fill`. With a `max_width` the
    /// whole shortcut is left out when it does not fit, never a part of it.
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

    pub(crate) fn shortcut_hint(child: Element, max_width: Option<f32>) -> Self {
        Self::new(ElementKind::ShortcutHint {
            child: Box::new(child),
            max_width,
        })
    }

    /// One line of text, ended with an ellipsis when the width available is
    /// shorter than the text.
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

    /// A paragraph of inline style runs, with explicit line breaks.
    pub fn formatted_text(spans: Vec<TextSpan>) -> Self {
        Self::new(ElementKind::FormattedText(spans))
    }

    /// An encoded raster image. The host validates and decodes it; see
    /// `RasterImage`.
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
            fit: SurfaceFit::Native,
        })
    }

    /// How a surface's pixels fill its rectangle: [`SurfaceFit::Contain`]
    /// and [`SurfaceFit::Cover`] scale them as a raster image is scaled, so
    /// an animation can take the place of its still first frame. Its
    /// [`radius`](Self::radius) rounds the corners of the pixels shown. Has
    /// no effect on an element that is not a surface.
    pub fn surface_fit(mut self, fit: SurfaceFit) -> Self {
        if let ElementKind::Surface { fit: own, .. } = &mut self.kind {
            *own = fit;
        }
        self
    }

    /// An icon on its own, [`ICON_SIZE`] on a side unless `large_icon` says
    /// otherwise.
    pub fn icon(icon: Icon) -> Self {
        Self::new(ElementKind::Icon(icon))
    }

    /// A text button with this ID. It starts with 8 pixels of padding, and its
    /// hover and press backgrounds change over the hover duration of the
    /// motion tokens.
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

    /// A single-line text field with 8 pixels of padding. `initial_value` is
    /// the text the first time this ID is laid out; from then on UiTree keeps
    /// the value, which `UiTree::input_value` reads.
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

    /// Tint a text input's leading icon. Without it the icon takes the
    /// foreground. Other elements ignore it.
    pub fn leading_icon_tint(mut self, color: Color) -> Self {
        if let ElementKind::TextInput {
            leading_icon_tint, ..
        } = &mut self.kind
        {
            *leading_icon_tint = Some(color);
        }
        self
    }

    /// Draw `image` in an icon button's icon slot, [`ICON_SIZE`] on a side,
    /// in place of its icon. Other elements ignore it.
    pub(crate) fn leading_image(mut self, image: crate::RasterImage) -> Self {
        if matches!(self.kind, ElementKind::IconButton { .. }) {
            self.kept.get_or_insert_default().leading_image = Some(image);
        }
        self
    }

    /// Keep `width` logical pixels at the end of a text input free of text,
    /// caret and hint, for a control a component lays over it there. Other
    /// elements ignore it; a width that is not finite is ignored and a
    /// negative one is zero.
    pub(crate) fn input_trailing_space(mut self, width: f32) -> Self {
        if width.is_finite() && matches!(self.kind, ElementKind::TextInput { .. }) {
            self.kept.get_or_insert_default().input_trailing_space = width.max(0.0);
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
    /// [`Element::text_area_enter_submits`] turns it into a chat composer's.
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
            enter_submits: false,
            highlights: Vec::new(),
        })
        .padding(8.0)
    }

    /// Make Enter submit a text area and Shift+Enter break the line, as a
    /// chat composer does; Ctrl+Enter still submits. Without it Enter breaks
    /// the line and Ctrl+Enter submits. Does nothing on another element.
    pub fn text_area_enter_submits(mut self, submits: bool) -> Self {
        if let ElementKind::TextArea { enter_submits, .. } = &mut self.kind {
            *enter_submits = submits;
        }
        self
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

    /// An empty element with no size of its own. Give it `flex` to push its
    /// siblings apart, or `size` for a fixed space.
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

    /// Levels as bars that spring to their height; see [`LevelBars`]. Give
    /// it the size it draws in.
    pub fn level_bars(bars: LevelBars) -> Self {
        Self::new(ElementKind::LevelBars(bars))
    }

    /// Bars over categories that the tree draws, animates and answers the
    /// pointer for; see [`BarPlot`]. Give it a size: left alone it takes the
    /// width it is offered and 240 pixels of height.
    pub fn bar_plot(plot: BarPlot) -> Self {
        Self::new(ElementKind::BarPlot(Box::new(plot)))
    }

    /// Lines over points that the tree draws, animates and answers the
    /// pointer for; see [`LinePlot`]. Sized as a [`BarPlot`] is.
    pub fn line_plot(plot: LinePlot) -> Self {
        Self::new(ElementKind::LinePlot(Box::new(plot)))
    }

    /// A pie or a ring that the tree draws, animates and answers the pointer
    /// for; see [`PiePlot`]. Sized as a [`BarPlot`] is; the circle takes the
    /// lesser of its width and its height.
    pub fn pie_plot(plot: PiePlot) -> Self {
        Self::new(ElementKind::PiePlot(Box::new(plot)))
    }

    /// A radar that the tree draws, animates and answers the pointer for; see
    /// [`RadarPlot`]. Sized as a [`BarPlot`] is.
    pub fn radar_plot(plot: RadarPlot) -> Self {
        Self::new(ElementKind::RadarPlot(Box::new(plot)))
    }

    /// A grid of cells that the tree draws, animates and answers the pointer
    /// for; see [`HeatPlot`]. It is as large as its cells and their gaps.
    pub fn heat_plot(plot: HeatPlot) -> Self {
        Self::new(ElementKind::HeatPlot(Box::new(plot)))
    }

    /// A column of marks that rise around the pointer. It is as wide as its
    /// longest mark and as tall as its marks, or as the room it is given;
    /// see [`MarkRail`].
    pub fn mark_rail(rail: MarkRail) -> Self {
        Self::new(ElementKind::MarkRail(rail))
    }

    /// A non-interactive loading surface with paint-only motion.
    pub fn skeleton(id: impl Into<String>, base: Color, highlight: Color) -> Self {
        Self::new(ElementKind::Skeleton {
            id: id.into(),
            base,
            highlight,
        })
    }

    /// A passive spinner in `color`, whose phase UiTree keeps by `id`. It has
    /// no size of its own: give it one with `size`.
    pub fn spinner(id: impl Into<String>, color: Color, variant: SpinnerVariant) -> Self {
        Self::new(ElementKind::Spinner {
            id: id.into(),
            color,
            variant,
        })
    }

    /// A record in its sleeve. `image` is the sleeve, a square on the
    /// element's height that fills with the picture, or with the style's
    /// background without one; the style's radius rounds it. The record
    /// waits behind it. `out` draws the record out towards the end, where it
    /// shows a fifth of the sleeve's side beyond it: leave it that room.
    /// `spinning` turns it, a turn every three seconds, and stopped it keeps
    /// the angle it reached. It is passive: wrap it in a clickable to play
    /// what it stands for.
    pub fn record(id: impl Into<String>, image: Option<RasterImage>, out: bool, spinning: bool) -> Self {
        Self::new(ElementKind::Record { id: id.into(), image, out, spinning, peek: None })
    }

    /// Also draw a record out while the pointer is over the control `id`,
    /// usually the clickable that holds it, as an invitation to play it.
    pub fn record_peek(mut self, id: impl Into<String>) -> Self {
        if let ElementKind::Record { peek, .. } = &mut self.kind {
            *peek = Some(id.into());
        }
        self
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

    /// Space between the element's edges and its content, in logical pixels.
    /// A negative value becomes zero; one that is not finite is ignored.
    pub fn padding(mut self, value: f32) -> Self {
        self.style.padding = length(value, self.style.padding);
        self
    }

    /// Space between the children of a row or column, in logical pixels. A
    /// negative value becomes zero; one that is not finite is ignored.
    pub fn gap(mut self, value: f32) -> Self {
        self.style.gap = length(value, self.style.gap);
        self
    }

    /// Let a row move its children onto more lines when they do not fit its
    /// width, as tags or role pills do. Each child keeps its natural width
    /// and `gap` from the one before it on its line. A line is as tall as
    /// its tallest child, and the others stretch to it as they do in a row;
    /// the next line starts [`line_gap`](Self::line_gap) under it. A child
    /// wider than the row has a line of its own, at the row's width. The
    /// row is as tall as its lines, its `max_width` is a width it wraps at,
    /// and the children keep their order for
    /// Tab and for assistive technology. `flex` and `shrink_to_fit` on its
    /// children change nothing. Only a row wraps; elsewhere it does nothing.
    pub fn wrap(mut self) -> Self {
        self.kept.get_or_insert_default().wrap = true;
        self
    }

    /// Space between the lines of a [`wrap`](Self::wrap)ping row, in logical
    /// pixels; its `gap` until this is called. A negative value becomes zero;
    /// one that is not finite is ignored.
    pub fn line_gap(mut self, value: f32) -> Self {
        if value.is_finite() {
            self.kept.get_or_insert_default().line_gap = Some(value.max(0.0));
        }
        self
    }

    /// The space between the lines of a wrapping row; `None` when this is
    /// not one.
    pub(crate) fn wrapping_line_gap(&self) -> Option<f32> {
        let kept = self.kept.as_deref().filter(|kept| kept.wrap)?;
        matches!(self.kind, ElementKind::Row(_)).then(|| kept.line_gap.unwrap_or(self.style.gap))
    }

    /// Fill the element's bounds with this color.
    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    /// Add a backdrop material behind the element's background and children.
    /// Leave the background transparent for the sampled scene to show through.
    pub fn glass(mut self, material: GlassMaterial) -> Self {
        self.kept.get_or_insert_default().glass = Some(material.normalized());
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

    /// Inset the top and bottom fade bands from the viewport edges, for content
    /// beneath opaque floating bars. Fade heights remain the visible transition
    /// outside the bars. This changes paint only; the host still reserves content
    /// space for its bars. Negative or non-finite insets become zero.
    pub fn fade_insets(mut self, top: f32, bottom: f32) -> Self {
        let inset = |value: f32| if value.is_finite() { value.max(0.0) } else { 0.0 };
        self.style.fade_insets = (inset(top), inset(bottom));
        self
    }

    /// Add a quiet bottom background behind children. Use dedicated footer
    /// space so the tint never covers the last line or the collapse control.
    pub fn bottom_background_fade(mut self, color: Color, height: f32) -> Self {
        self.style.bottom_background_fade =
            (height.is_finite() && height > 0.0).then_some((color, height));
        self
    }

    /// The same from the top edge down: a colour behind a page's header
    /// that gives way to the surface below it.
    pub fn top_background_fade(mut self, color: Color, height: f32) -> Self {
        self.style.top_background_fade =
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

    /// Ink of an interactive element while the pointer is over it. Its hit
    /// bounds and geometry do not change.
    pub fn hover_foreground(mut self, color: Color) -> Self {
        self.style.hover_foreground = Some(color);
        self
    }

    /// Ink of an interactive element while it is pressed.
    pub fn pressed_foreground(mut self, color: Color) -> Self {
        self.style.pressed_foreground = Some(color);
        self
    }

    /// Border a text input or text area shows in place of its own while it has
    /// the focus. Another control shows it while it has the keyboard focus if
    /// its [focus ring](Self::focus_ring) is off, as its mark of that focus.
    /// A width that is not finite or not above zero removes it.
    pub fn focus_border(mut self, color: Color, width: f32) -> Self {
        self.style.focus_border =
            (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self
    }

    /// Let the pointer drag a horizontal viewport's content sideways, and
    /// show a thin bar where it sits while there is more than fits. The bar
    /// takes the viewport's foreground; pressing a control inside still
    /// presses that control. [`Element::scrollbar_visibility`] can make the
    /// indicator transient without disabling drag input.
    pub fn drag_to_pan(mut self) -> Self {
        if let ElementKind::HorizontalScroll { drag_pan, .. } = &mut self.kind {
            *drag_pan = true;
        }
        self
    }

    /// A clipped horizontal viewport over `child`. A vertical wheel moves it
    /// sideways, as a tab strip needs; inside a document that scrolls
    /// vertically use `horizontal_scroll_2d`.
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

    /// Whether the keyboard focus ring is drawn on this element. It is by
    /// default.
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

    /// Color of the element's text and icons. The default is white.
    pub fn foreground(mut self, color: Color) -> Self {
        self.style.foreground = color;
        self
    }

    /// Ink a text with `stops` spread evenly from the left of each painted
    /// line to its right, blending from each to the next: a name in the
    /// colors of a role. For [`Element::text`] and
    /// [`Element::truncated_text`]; the alpha of the foreground scales each
    /// stop's. Fewer than two stops paint the plain foreground. Measuring,
    /// wrapping, truncation and selection are those of the plain text.
    pub fn foreground_gradient(mut self, stops: impl Into<Arc<[Color]>>) -> Self {
        let stops = stops.into();
        self.kept.get_or_insert_default().ink_gradient = (stops.len() > 1).then_some(stops);
        self
    }

    /// Center the text, or a button's content, across the element's width
    /// instead of starting it at the edge.
    /// Single-line inputs center values that fit, with matching caret,
    /// selection and pointer geometry; overflowing values scroll normally.
    pub fn center_content(mut self) -> Self {
        self.style.center_content = true;
        self
    }

    /// Underline the text while the pointer is over the element or it has the
    /// keyboard focus.
    pub fn underline_on_hover(mut self) -> Self {
        self.style.underline_on_hover = true;
        self
    }

    /// Draw a solid border inside the element's bounds without changing its
    /// layout size. It replaces any border set before, dashed or not; a width
    /// that is not finite or not above zero removes the border.
    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.style.border = (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self.style.dashed_border = false;
        self
    }

    /// Draw the border as dashes. It replaces any border set before; a width
    /// that is not finite or not above zero removes the border.
    pub fn dashed_border(mut self, color: Color, width: f32) -> Self {
        self.style.border = (width > 0.0 && width.is_finite()).then_some(Border { color, width });
        self.style.dashed_border = self.style.border.is_some();
        self
    }

    /// Add depth to a card or floating surface without changing its layout.
    pub fn shadow(mut self, color: Color, offset: Point, blur: f32, spread: f32) -> Self {
        self.style.shadow = (color.a > 0).then_some(BoxShadow::new(color, offset, blur, spread));
        self
    }

    /// Keep descendants inside this element's content rectangle. Useful for
    /// a dock whose width changes while its children retain intrinsic sizes.
    /// On a rounded element the clip is rounded too, concentric with its
    /// corners less its padding: fills and images that reach a corner take
    /// it (see [`crate::DrawCommand::RoundClip`]). Text is clipped to the
    /// rectangle only.
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

    /// Say what this element is to the platform: its role, its name and its
    /// state, as a screen reader will give them. What is left out the tree
    /// works out from the element; said twice, the later call adds to the
    /// earlier and wins where they differ.
    pub fn accessible(mut self, access: crate::Access) -> Self {
        self.access = Some(Box::new(match self.access.take() {
            Some(earlier) => access.over(*earlier),
            None => access,
        }));
        self
    }

    /// Name this element for the platform: what a screen reader says for a
    /// control whose text does not say it, such as an icon alone.
    pub fn accessibility_label(self, name: impl Into<String>) -> Self {
        self.accessible(crate::Access::named(name))
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

    /// Choose the face the element's text is measured and painted in. The
    /// default is `FontFamily::Monospace`.
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

    /// Keep the natural height while the column has room, and give up height
    /// before pushing fixed siblings out. Only a direct, non-flex child of a
    /// column uses this flag; rows and unbounded columns leave it alone.
    /// Multiple children give up height in order, down to their minimums first
    /// and then below them if necessary. Flexible siblings keep their minimum
    /// heights. Put overflowing content in a scroll area or clip it explicitly.
    /// A child with unbounded natural height starts at the column's finite
    /// height before yielding room to its siblings.
    pub fn shrink_height_to_fit(mut self) -> Self {
        self.style.shrink_height = true;
        self
    }

    /// Take a share of the space a row or column has left, in proportion to
    /// this factor among the flexible children. Zero, the default, keeps the
    /// natural size; a negative factor becomes zero, and one that is not
    /// finite is ignored.
    pub fn flex(mut self, factor: f32) -> Self {
        self.style.flex = length(factor, self.style.flex);
        self
    }

    /// Fix the width and the height in logical pixels. `None` leaves that
    /// side to the content and to the parent; a negative value becomes zero,
    /// and one that is not finite leaves that side as it was.
    pub fn size(mut self, width: Option<f32>, height: Option<f32>) -> Self {
        let side = |value: Option<f32>, current: Option<f32>| match value {
            Some(value) if !value.is_finite() => current,
            value => value.map(|value| value.max(0.0)),
        };
        self.style.width = side(width, self.style.width);
        self.style.height = side(height, self.style.height);
        self
    }

    /// Lower bounds for the width and the height, in logical pixels. They take
    /// precedence over `max_width` and `max_height`. A negative value becomes
    /// zero; one that is not finite is ignored.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.style.min_width = length(width, self.style.min_width);
        self.style.min_height = length(height, self.style.min_height);
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

    /// Show at most `lines` lines of a formatted text, at least one: what
    /// follows is left out and the last line shown ends in «…», in the ink,
    /// size and face of the text before it, giving up the end of that line
    /// to make room. The «…» is not part of the text a selection copies.
    /// Other elements ignore it.
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.style.max_lines = Some(lines.max(1));
        self
    }

    /// Round the four corners by this radius, in logical pixels. It replaces
    /// any `corner_radii`; a negative value becomes zero, and one that is not
    /// finite is ignored. A painted radius never exceeds half the shorter side.
    pub fn radius(mut self, radius: f32) -> Self {
        self.style.radius = length(radius, self.style.radius);
        self.style.corner_radii = None;
        self
    }

    /// Round the ends fully: half the shorter side of wherever the element is
    /// laid out, so a pill or a circle stays one when its size changes later.
    /// It replaces any `radius` or `corner_radii`.
    pub fn round(mut self) -> Self {
        // Painting clamps every radius to the rectangle it rounds.
        self.style.radius = f32::INFINITY;
        self.style.corner_radii = None;
        self
    }

    /// Override the uniform radius for backgrounds, solid borders and focus.
    /// Shadows keep the uniform radius; a clip of descendants follows these.
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
                DrawCommand::Surface { rect, id, label, opacity, .. } => Some((*rect, id.as_str(), label.as_str(), *opacity)),
                _ => None,
            })
            .collect();
        // The frame carries the name only: new pixels never rebuild the tree.
        assert_eq!(surfaces, [(Rect::new(0.0, 20.0, 300.0, 200.0), "page-pixels", "Página", 1.0)]);
        assert!(frame.regions.iter().any(|region| region.id == "page" && region.rect == surfaces[0].0));
    }

    #[test]
    fn a_surface_carries_its_fit_and_radius_and_other_elements_ignore_the_fit() {
        let shown = |element: Element| {
            let frame = UiTree::new(element).layout(Size::new(120.0, 80.0));
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Surface { fit, radius, .. } => Some((*fit, *radius)),
                _ => None,
            })
        };
        assert_eq!(shown(Element::surface("a", "A")), Some((SurfaceFit::Native, 0.0)));
        let covered = Element::surface("a", "A").surface_fit(SurfaceFit::Cover).radius(8.0);
        assert_eq!(shown(covered), Some((SurfaceFit::Cover, 8.0)));
        assert!(shown(Element::spacer().surface_fit(SurfaceFit::Cover)).is_none());
    }
}
