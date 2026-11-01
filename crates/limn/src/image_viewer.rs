//! Controlled image gallery. The host owns the images and navigation state;
//! the tree keeps the zoom.

use crate::{
    Access, Button, ButtonVariant, Color, Component, ComponentPalette, ControlSize, CornerRadii, Element, FontFamily,
    Icon, IconButton, Point, PopoverPlacement, RasterImage, Rect, Role, Size, Spinner, Tooltip, TooltipPlacement,
    ZoomControl,
};

/// The corner radius a thumbnail's picture starts with, and the one a lightbox's picture
/// rests at.
const ORIGIN_RADIUS: f32 = 8.0;
const PICTURE_RADIUS: f32 = 6.0;
/// How long the picture takes from its origin, and back, in milliseconds.
const MORPH_OPEN_MS: u16 = 350;
const MORPH_CLOSE_MS: u16 = 250;
/// How long a lightbox without an origin takes to fade in and out, in milliseconds.
const FADE_MS: u16 = 180;
/// The gap between the action column and the picture's corner, in logical pixels.
const ACTION_GAP: f32 = 6.0;
/// A lightbox's picture is at most this wide, and its window's height less this tall.
const LIGHTBOX_MAX_WIDTH: f32 = 640.0;
const LIGHTBOX_HEIGHT_MARGIN: f32 = 64.0;
/// The caption and count lines under a lightbox's picture, and the gap above them.
const CAPTION_LINE: f32 = 20.0;
const CAPTION_GAP: f32 = 8.0;

/// How an [`ImageViewer`] arranges its controls around the picture.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ViewerLayout {
    /// A header with the title and one group of controls that ends with close; arrows at the
    /// sides; thumbnails, or the place in the gallery, under the picture. Today's look.
    #[default]
    Header,
    /// A lightbox: the host's lead ([`ImageViewer::lead`]) at the top left, or the title
    /// there when there is none; the controls in a floating group at the top right, its zoom
    /// toggle first, which shows zoom in, or zoom out while the picture is zoomed; close on its
    /// own in the corner; chevron arrows at the window's sides that wrap around; small
    /// thumbnails under the picture; no "2 of 5". A click zooms a still picture, not a surface
    /// such as a video. While the picture is zoomed in, the arrows and the thumbnails hide; the
    /// group, close and the lead stay.
    Lightbox,
}

/// A picture over the whole window, alone or as one of a gallery: the
/// picture on a dark backdrop, zoomed and panned as [`Element::zoom_view`]
/// does, with a title and the host's controls above it and the gallery's
/// place, or its thumbnails, below.
pub struct ImageViewer {
    base: Element,
    open: bool,
    title: String,
    subtitle: Option<String>,
    close_id: String,
    palette: ComponentPalette,
    viewport: Size,
    image: Option<RasterImage>,
    placeholder: Option<RasterImage>,
    surface: Option<(String, String)>,
    picture_size: Option<Size>,
    error: Option<String>,
    index: usize,
    total: usize,
    previous_action: Option<String>,
    next_action: Option<String>,
    navigation_labels: Option<[String; 2]>,
    thumbnails: Vec<(String, Option<RasterImage>, String)>,
    return_focus: Option<String>,
    initial_focus: Option<String>,
    level: Option<(f32, bool)>,
    zoom_actions: Option<[String; 3]>,
    actions: Vec<(String, Icon, String)>,
    footer: Option<Element>,
    layout: ViewerLayout,
    lead: Option<Element>,
    tool: Option<Element>,
    scrim: Option<Color>,
    origin: Option<Rect>,
    lightbox: bool,
    natural: Option<(f32, f32)>,
    caption: Option<String>,
}

impl ImageViewer {
    /// A viewer over `base`, drawn only while `open`; closed, it renders `base` alone. The
    /// close control reports `close_id`, which also prefixes the viewer's own IDs, such as
    /// `{close_id}:dialog`. Until `image` is set it shows a spinner and
    /// `Labels::loading_image`.
    pub fn new(
        base: Element,
        open: bool,
        title: impl Into<String>,
        close_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            open,
            title: title.into(),
            subtitle: None,
            close_id: close_id.into(),
            palette,
            viewport: Size::new(900.0, 620.0),
            image: None,
            placeholder: None,
            surface: None,
            picture_size: None,
            error: None,
            index: 0,
            total: 1,
            previous_action: None,
            next_action: None,
            navigation_labels: None,
            thumbnails: Vec::new(),
            return_focus: None,
            initial_focus: None,
            level: None,
            zoom_actions: None,
            actions: Vec::new(),
            footer: None,
            layout: ViewerLayout::Header,
            lead: None,
            tool: None,
            scrim: None,
            origin: None,
            lightbox: false,
            natural: None,
            caption: None,
        }
    }

    /// The window's size in logical pixels, which the viewer covers (900 by
    /// 620 by default). A size with a side that is not finite is ignored and
    /// a negative side is zero.
    pub fn viewport(mut self, size: Size) -> Self {
        if size.width.is_finite() && size.height.is_finite() {
            self.viewport = Size::new(size.width.max(0.0), size.height.max(0.0));
        }
        self
    }

    /// A line under the title, such as the picture's size and weight:
    /// `"300 × 340 · 47 KB · PNG"`. An empty one shows nothing.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into()).filter(|said| !said.is_empty());
        self
    }

    /// The picture to show, decoded whole so it can be zoomed into, and with
    /// a checkerboard behind its transparent pixels.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    /// Something to show in the picture's place, at its size, while `image`
    /// has not arrived, such as the thumbnail already on screen or a blurred
    /// stand-in: the picture then loads over it instead of over a spinner.
    pub fn placeholder(mut self, image: RasterImage) -> Self {
        self.placeholder = Some(image);
        self
    }

    /// The picture's own size in pixels. With it the picture fits the window
    /// without being enlarged past that size, one of its pixels is a logical
    /// pixel at 100 %, and zooming shows its pixels; without it, it fills the
    /// room at its proportions. A size with a side that is not finite or not
    /// above zero is ignored.
    pub fn picture_size(mut self, size: Size) -> Self {
        let valid = |side: f32| side.is_finite() && side > 0.0;
        if valid(size.width) && valid(size.height) {
            self.picture_size = Some(size);
        }
        self
    }

    /// Show the pixels the host presents under `id` (see [`Element::surface`]) in place
    /// of a picture, fitted and zoomed as a picture is: an animation, a video. `label`
    /// names it for assistive technology. It takes the place of `image` when both are
    /// set; the host presents its first frame before it opens the viewer on it.
    pub fn surface(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.surface = Some((id.into(), label.into()));
        self
    }

    /// A message shown in place of the spinner and the loading label while there is no
    /// image. It is not shown once `image` or `surface` is set.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// The place of the picture: `index` counts from zero and is clamped to the last of
    /// `total`, which is at least 1. Below the picture, `2 of 5` says it when there is
    /// more than one and no thumbnails, and the arrows appear only when `total` is above 1.
    pub fn gallery(mut self, index: usize, total: usize) -> Self {
        self.total = total.max(1);
        self.index = index.min(self.total - 1);
        self
    }

    /// Add an arrow before the picture that reports `id`. It is disabled on the first
    /// picture.
    pub fn previous_action(mut self, id: impl Into<String>) -> Self {
        self.previous_action = Some(id.into());
        self
    }

    /// Add an arrow after the picture that reports `id`. It is disabled on the last
    /// picture.
    pub fn next_action(mut self, id: impl Into<String>) -> Self {
        self.next_action = Some(id.into());
        self
    }

    /// What the arrows are called, for a gallery of something other than images, such
    /// as the pages of a document: the names a screen reader says and the hints show,
    /// in place of `Labels::previous_image` and `Labels::next_image`.
    pub fn navigation_labels(mut self, previous: impl Into<String>, next: impl Into<String>) -> Self {
        self.navigation_labels = Some([previous.into(), next.into()]);
        self
    }

    /// One more thumbnail in the strip under the picture, reporting `id` when
    /// clicked: `image` cropped to a square, or a plain square while it has
    /// none, named `label`. The one at the gallery's index is ringed. Each
    /// call adds one, in order; the strip shows with two or more, in place of
    /// `2 of 5`. An empty `id` adds nothing.
    pub fn thumbnail(mut self, id: impl Into<String>, image: Option<RasterImage>, label: impl Into<String>) -> Self {
        let id = id.into();
        if !id.is_empty() {
            self.thumbnails.push((id, image, label.into()));
        }
        self
    }

    /// The control that takes focus when the viewer closes. The picture also
    /// grows out of it when the viewer opens, unless motion is reduced.
    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.return_focus = Some(id.into());
        self
    }

    /// The control that has the focus when the viewer opens, such as a
    /// video's play button; the close control unless set.
    pub fn initial_focus(mut self, id: impl Into<String>) -> Self {
        self.initial_focus = Some(id.into());
        self
    }

    /// Ignored: the tree keeps the zoom now. Pass what
    /// [`UiAction::Zoomed`](crate::UiAction::Zoomed) reports to
    /// [`ImageViewer::zoom_level`] to show it.
    #[deprecated(note = "the tree keeps the zoom; pass UiAction::Zoomed to zoom_level")]
    pub fn zoom(self, _zoom: f32) -> Self {
        self
    }

    /// What the last [`UiAction::Zoomed`](crate::UiAction::Zoomed) of this
    /// viewer said: the header then shows the scale as a percentage between
    /// zoom out and zoom in, which fits the picture or zooms in two and a
    /// half times; zoom out and fit are disabled while `fitted`, and zoom in
    /// at the most. A scale that is not finite is ignored.
    pub fn zoom_level(mut self, scale: f32, fitted: bool) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.level = Some((scale, fitted));
        }
        self
    }

    /// Controls in the header to zoom out, fit the window and zoom in, and
    /// one to turn a picture a quarter clockwise. The viewer answers them
    /// itself; their clicks are still reported with these IDs, the turn's as
    /// `{close_id}:rotate`.
    pub fn zoom_actions(
        mut self,
        out: impl Into<String>,
        fit: impl Into<String>,
        zoom_in: impl Into<String>,
    ) -> Self {
        self.zoom_actions = Some([out.into(), fit.into(), zoom_in.into()]);
        self
    }

    /// Where the picture comes from: the rectangle of its thumbnail in window
    /// coordinates. Opening grows the picture from it to its place over 350 ms, and
    /// closing shrinks it back over 250 ms. Without an origin a lightbox fades in
    /// over 180 ms, and the other layouts grow the picture out of the
    /// [`ImageViewer::return_focus`] control. Under reduced motion both are at once.
    pub fn origin(mut self, rect: Rect) -> Self {
        if [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite()) {
            self.origin = Some(rect);
        }
        self
    }

    /// Add a control that reports `id` when activated, `name` its name and hint,
    /// such as saving the picture. Each call adds one; they keep the order of the
    /// calls. An empty `id` adds nothing. With [`ImageViewer::zoom_actions`] they sit
    /// in the header, before the zoom controls, and in [`ViewerLayout::Lightbox`] in its
    /// group. Otherwise, and in a [`ImageViewer::lightbox`], they stand in a column under
    /// the close button, just outside the picture's top-right corner, or on its own corner
    /// when there is no room beside it.
    pub fn action(mut self, id: impl Into<String>, icon: Icon, name: impl Into<String>) -> Self {
        let id = id.into();
        if !id.is_empty() {
            self.actions.push((id, icon, name.into()));
        }
        self
    }

    /// The host's own content under the picture, above the gallery's place,
    /// such as a video's controls. It is as wide as the window less a margin.
    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }

    /// How the viewer arranges its controls: [`ViewerLayout::Header`] unless set.
    pub fn layout(mut self, layout: ViewerLayout) -> Self {
        self.layout = layout;
        self
    }

    /// The host's content at the top left in the lightbox layout, such as who sent the
    /// picture and when, at most 184 pixels wide; the title is then only the dialog's name.
    /// The header layout does not show it.
    pub fn lead(mut self, lead: Element) -> Self {
        self.lead = Some(lead);
        self
    }

    /// One more control of the host's own, such as a button that opens a menu: after the
    /// actions in the header, last in the lightbox's group. Give it the controls' size, 36
    /// pixels in the header and 32 in the lightbox.
    pub fn tool(mut self, tool: Element) -> Self {
        self.tool = Some(tool);
        self
    }

    /// The backdrop's color, behind the picture in either layout: 232 black unless set.
    pub fn scrim(mut self, color: Color) -> Self {
        self.scrim = Some(color);
        self
    }

    /// A lightbox: no panel, no header and no scrim of the panel's, the picture alone over
    /// a dimmed window, its close control and actions in a column beside its top-right
    /// corner and its caption under it. Zoom is not offered; the picture is fitted to at most
    /// 640 pixels wide and the window's height less 64, and never enlarged past its natural
    /// size (see [`ImageViewer::natural_size`]). Off by default.
    pub fn lightbox(mut self, lightbox: bool) -> Self {
        self.lightbox = lightbox;
        self
    }

    /// The picture's size in pixels, which a lightbox keeps to its aspect ratio. Without it
    /// a lightbox's picture box is the whole of its bound and the picture is fitted inside it.
    /// A size that is not finite or not above zero is ignored.
    pub fn natural_size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            self.natural = Some((width, height));
        }
        self
    }

    /// The line under a lightbox's picture, such as the file name. It is white and centred.
    pub fn caption(mut self, text: impl Into<String>) -> Self {
        self.caption = Some(text.into());
        self
    }

    /// Formerly the largest zoom the host could pass; the tree's limit is
    /// [`ZOOM_VIEW_MAX_SCALE`](crate::ZOOM_VIEW_MAX_SCALE).
    #[deprecated(note = "the tree keeps the zoom; its limit is ZOOM_VIEW_MAX_SCALE")]
    pub const MAX_ZOOM: f32 = 8.0;

    /// The zoom view's ID: one of its own for each picture, so another
    /// picture opens fitted.
    fn view_id(&self) -> String {
        format!("{}:view:{}:{}", self.close_id, self.index, self.title)
    }
}

impl ImageViewer {
    /// The floating surface over the base, dimmed by `scrim`: its entrance and exit, and the
    /// dismiss, focus and arrows of the gallery.
    fn float(self, viewer: Element, scrim: Color) -> Element {
        let (enter, exit) = if self.origin.is_some() { (MORPH_OPEN_MS, MORPH_CLOSE_MS) } else { (FADE_MS, FADE_MS) };
        let mut popover = Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
            .popover_scrim(scrim)
            .popover_animation_ms(enter)
            .popover_close_animation_ms(exit)
            .popover_backdrop_dismiss(true)
            .popover_initial_focus(&self.close_id)
            // The arrows its hints show: back while there is one before, on while there is one after.
            .popover_arrows(
                self.previous_action.filter(|_| self.index > 0),
                self.next_action.filter(|_| self.index + 1 < self.total),
            );
        if let Some(id) = self.return_focus {
            popover = popover.popover_return_focus(id);
        }
        if let Some(origin) = self.origin {
            popover = popover.popover_morph(origin, ORIGIN_RADIUS);
        }
        popover
    }

    /// The lightbox's content: the picture, its close control and actions beside its corner,
    /// and its caption and count under it. Centred by its own height, so the picture is.
    fn lightbox_viewer(&mut self) -> Element {
        let fit = lightbox_fit(self.natural, self.viewport);
        let mut controls = self.palette;
        controls.surface = self.palette.raised;
        let labels = crate::labels();
        let picture = match self.image.take() {
            Some(image) => Element::raster_image(image.full_resolution()).radius(PICTURE_RADIUS),
            None => pending(&self.close_id, self.error.as_deref(), fit.width, self.palette),
        };
        // A panel carries the shadow, and the click that closes the viewer is the picture's own.
        let shadow = Element::panel(picture.size(Some(fit.width), Some(fit.height)))
            .radius(PICTURE_RADIUS)
            .shadow(Color::rgba(0, 0, 0, 77), Point::new(0.0, 4.0), 32.0, 0.0);
        let anchor = Element::clickable(&self.close_id, shadow).size(Some(fit.width), Some(fit.height));
        let mut column = vec![close_control(&self.close_id, controls)];
        for (id, icon, label) in &self.actions {
            column.push(control(id, *icon, label, label, true, controls));
        }
        let picture = Element::popover(
            anchor,
            Element::column(column).gap(ACTION_GAP).size(Some(36.0), None),
            format!("{}:actions", self.close_id),
            PopoverPlacement::RightCorner,
        )
        .popover_gap(ACTION_GAP)
        .popover_non_modal();
        let mut body = Vec::new();
        if self.total > 1 {
            if let Some(id) = &self.previous_action {
                body.push(side_arrow(id, Icon::ArrowLeft, labels.previous_image, "←", self.index > 0, controls));
            }
        }
        body.push(picture);
        if self.total > 1 {
            if let Some(id) = &self.next_action {
                body.push(side_arrow(id, Icon::ArrowRight, labels.next_image, "→", self.index + 1 < self.total, controls));
            }
        }
        let width = fit.width + (body.len() - 1) as f32 * (36.0 + 12.0);
        let line = |text: String| {
            Element::truncated_text(text)
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .foreground(Color::WHITE)
                .center_content()
                .size(Some(width), Some(CAPTION_LINE))
        };
        let mut below = vec![line(self.caption.clone().unwrap_or_default())];
        if self.total > 1 {
            below.push(line(format!("{} {} {}", self.index + 1, labels.of, self.total)));
        }
        // The top spacer is as tall as the caption block, so the picture sits at the centre.
        let below_height = CAPTION_LINE * below.len() as f32;
        Element::column(vec![
            Element::spacer().size(Some(width), Some(below_height)),
            Element::row(body).gap(12.0).size(Some(width), Some(fit.height)),
            Element::column(below).size(Some(width), Some(below_height)),
        ])
        .gap(CAPTION_GAP)
        .size(Some(width), None)
        // Known by an ID of its own, as the panel's is: `close_id` is its close control's.
        .accessible(Access::new(Role::Dialog).id(format!("{}:dialog", self.close_id)).name(&self.title))
    }
}

/// The picture's box in a lightbox: its natural size fitted to 640 wide, or the window's width
/// less the margins if that is less, and the window's height less 64. Never enlarged.
fn lightbox_fit(natural: Option<(f32, f32)>, viewport: Size) -> Size {
    let bound = Size::new(
        (viewport.width - 16.0).clamp(0.0, LIGHTBOX_MAX_WIDTH),
        (viewport.height - LIGHTBOX_HEIGHT_MARGIN).max(0.0),
    );
    match natural {
        Some((width, height)) => {
            let scale = (bound.width / width).min(bound.height / height).min(1.0);
            // The clamp keeps a side at its bound exactly, not a rounding past it.
            Size::new((width * scale).min(bound.width), (height * scale).min(bound.height))
        }
        None => bound,
    }
}

/// The close control, whose hint names the Escape key that does the same.
fn close_control(close_id: &str, palette: ComponentPalette) -> Element {
    let labels = crate::labels();
    control(close_id, Icon::Close, labels.close_image, &format!("{} · Esc", labels.close_image), true, palette)
}

/// A gallery arrow at a side of the picture, its hint naming the key `key` that does the same.
fn side_arrow(id: &str, icon: Icon, label: &str, key: &str, enabled: bool, palette: ComponentPalette) -> Element {
    Element::column(vec![
        Element::spacer().flex(1.0),
        control(id, icon, label, &format!("{label} · {key}"), enabled, palette),
        Element::spacer().flex(1.0),
    ])
    .size(Some(36.0), None)
}

/// What shows while the picture loads, or the host's error in its place.
fn pending(close_id: &str, error: Option<&str>, width: f32, palette: ComponentPalette) -> Element {
    let message = error.unwrap_or(crate::labels().loading_image);
    let mut content = vec![Element::spacer().flex(1.0)];
    if error.is_none() {
        content.push(centered(
            Spinner::new(format!("{close_id}:loading"), palette.muted)
                .size(32.0)
                .render()
                .accessibility_label(crate::labels().loading_image),
            32.0,
        ));
    }
    content.push(centered(
        Element::text(message)
            .font_family(FontFamily::Sans)
            .font_size(14.0)
            .foreground(palette.muted)
            .size(Some((width - 144.0).clamp(80.0, 520.0)), None),
        64.0,
    ));
    content.push(Element::spacer().flex(1.0));
    Element::column(content).gap(12.0)
}

/// An icon alone: `name` is what it is called, `title` its hint, which may
/// add a shortcut.
pub(crate) fn control(
    id: &str,
    icon: Icon,
    name: &str,
    title: &str,
    enabled: bool,
    palette: ComponentPalette,
) -> Element {
    let said = Access::new(Role::Button).name(name);
    // Drawn disabled it is passive, and still there for the platform.
    let said = if enabled { said } else { said.id(id).disabled(true) };
    let button = IconButton::new(id, icon, palette)
        .variant(ButtonVariant::Subtle)
        .size(ControlSize::Spacious)
        .enabled(enabled)
        .render()
        // Half as round again as a panel, as a full-window preview.
        .radius(crate::tokens().radius.panel * 1.5)
        .size(Some(36.0), Some(36.0))
        .accessible(said);
    Tooltip::new(button, id, title, palette)
        .render()
        .size(Some(36.0), Some(36.0))
}

fn centered(child: Element, height: f32) -> Element {
    Element::row(vec![
        Element::spacer().flex(1.0),
        child,
        Element::spacer().flex(1.0),
    ])
    .size(None, Some(height))
}

/// A thumbnail's side in the strip.
const THUMBNAIL: f32 = 48.0;
/// The header's height, and the bar the thumbnails sit in.
const HEADER: f32 = 48.0;
const STRIP: f32 = THUMBNAIL + 16.0;
/// The lightbox's measures in logical pixels: the margins from the window's edges, the top
/// bar, the gaps between its rows, the squares of close and the arrows, the buttons of its
/// group, the widest lead, the room the group leaves for close, and the thumbnails.
const LIGHTBOX_MARGIN_X: f32 = 24.0;
const LIGHTBOX_MARGIN_Y: f32 = 36.0;
const LIGHTBOX_BAR: f32 = 40.0;
const LIGHTBOX_GAP: f32 = 12.0;
const LIGHTBOX_SQUARE: f32 = 40.0;
const LIGHTBOX_CONTROL: f32 = 32.0;
const LIGHTBOX_LEAD: f32 = 184.0;
/// From the group's right edge to the content box's: the close square and the gap before it.
const LIGHTBOX_CLOSE_ROOM: f32 = LIGHTBOX_SQUARE + LIGHTBOX_GAP;
const LIGHTBOX_THUMBNAIL: f32 = 40.0;
const LIGHTBOX_THUMBNAIL_GAP: f32 = 2.0;
/// The veil over a thumbnail that is not the one shown: black at this alpha, and at
/// `VEIL_HOVER` while the pointer is over it.
const VEIL: u8 = 128;
const VEIL_HOVER: u8 = 38;

/// A lightbox button: `side` on a side with its glyph at 16 pixels, a ghost until the
/// pointer is over it, rounded by `radius`, and its hint below.
fn lightbox_button(
    id: &str,
    icon: Icon,
    name: &str,
    title: &str,
    palette: ComponentPalette,
    side: f32,
    radius: f32,
) -> Element {
    let button = IconButton::new(id, icon, palette)
        .variant(ButtonVariant::Ghost)
        .ink(palette.muted)
        .hover_ink(palette.text)
        .render()
        .radius(radius)
        .size(Some(side), Some(side))
        .accessible(Access::new(Role::Button).name(name));
    Tooltip::new(button, id, title, palette)
        .placement(TooltipPlacement::Below)
        .render()
        .size(Some(side), Some(side))
}

/// The lightbox's close and arrows: a lightbox button on a raised square with a 1 px border.
fn lightbox_square(id: &str, icon: Icon, name: &str, title: &str, palette: ComponentPalette) -> Element {
    let radius = crate::tokens().radius.panel;
    Element::panel(lightbox_button(id, icon, name, title, palette, LIGHTBOX_SQUARE, radius))
        .background(palette.raised)
        .border(palette.border, 1.0)
        .radius(radius)
        .size(Some(LIGHTBOX_SQUARE), Some(LIGHTBOX_SQUARE))
}

impl Component for ImageViewer {
    fn render(mut self) -> Element {
        if !self.open {
            return self.base;
        }
        if self.lightbox {
            let viewer = self.lightbox_viewer();
            return self.float(viewer, Color::rgba(0, 0, 0, 159));
        }
        let lightbox = self.layout == ViewerLayout::Lightbox;
        // A header without zoom controls leaves the host's actions to a column, led by close,
        // beside the picture's top-right corner.
        let column = !lightbox && !self.actions.is_empty() && self.zoom_actions.is_none();
        // Zoomed in, the lightbox hides its arrows and thumbnails; its group stays, for the zoom toggle.
        let zoomed = lightbox && matches!(self.level, Some((_, false)));
        let labels = crate::labels();
        // The viewer is dark whatever the window's theme, as a picture is
        // best seen on dark; a dark host palette keeps its own colors.
        let chrome = if self.palette.is_dark() { self.palette } else { ComponentPalette::neutral_dark() };
        let mut controls = chrome;
        controls.surface = chrome.raised;
        let view_id = self.view_id();
        let mut titles = vec![Element::truncated_text(&self.title)
            .font_family(FontFamily::SansMedium)
            .font_size(15.0)
            .foreground(chrome.text)
            .size(None, Some(20.0))];
        if let Some(subtitle) = &self.subtitle {
            titles.push(
                Element::truncated_text(subtitle)
                    .font_family(FontFamily::Sans)
                    .font_size(12.0)
                    .foreground(chrome.muted)
                    .size(None, Some(16.0)),
            );
        }
        let mut tools = Vec::new();
        let mut zoom_controls = Vec::new();
        // The lightbox's zoom toggle leads its group, before the host's actions. It is drawn for
        // a still picture only: zoom in while the picture fits, zoom out while it is zoomed.
        if lightbox {
            if let Some([_, _, zoom_in]) = &self.zoom_actions {
                if self.surface.is_none() {
                    let (icon, name, hint) = if zoomed {
                        (Icon::ZoomOut, labels.zoom_out, format!("{} · −", labels.zoom_out))
                    } else {
                        (Icon::ZoomIn, labels.zoom_in, format!("{} · +", labels.zoom_in))
                    };
                    let button = lightbox_button(
                        zoom_in,
                        icon,
                        name,
                        &hint,
                        controls,
                        LIGHTBOX_CONTROL,
                        crate::tokens().radius.card,
                    );
                    tools.push(button);
                    zoom_controls.push((zoom_in.clone(), ZoomControl::Toggle));
                }
            }
        }
        for (id, icon, name) in self.actions.iter().filter(|_| !column) {
            tools.push(if lightbox {
                lightbox_button(id, *icon, name, name, controls, LIGHTBOX_CONTROL, crate::tokens().radius.card)
            } else {
                control(id, *icon, name, name, true, controls)
            });
        }
        let picture_shown = self.surface.is_some() || self.image.is_some() || self.placeholder.is_some();
        if lightbox {
            // The host's own tool comes last in the group.
            tools.extend(self.tool);
        } else {
            tools.extend(self.tool);
            if let Some([out, fit, zoom_in]) = &self.zoom_actions {
                let (fitted, most) = match self.level {
                    Some((scale, fitted)) => (fitted, scale >= crate::ZOOM_VIEW_MAX_SCALE - 0.001),
                    None => (false, false),
                };
                if !tools.is_empty() {
                    tools.push(Element::spacer().size(Some(8.0), Some(36.0)));
                }
                tools.push(control(out, Icon::Minus, labels.zoom_out, &format!("{} · −", labels.zoom_out), !fitted, controls));
                zoom_controls.push((out.clone(), ZoomControl::Out));
                if let Some((scale, _)) = self.level {
                    let id = format!("{}:zoom-level", self.close_id);
                    let percent = format!("{} %", crate::zoom_percent(scale));
                    let button = Button::new(&id, &percent, controls)
                        .variant(ButtonVariant::Subtle)
                        .size(ControlSize::Spacious)
                        .accessibility_label(format!("{} {percent}", labels.zoom_level))
                        .render()
                        .size(Some(64.0), Some(36.0));
                    tools.push(Tooltip::new(button, &id, labels.fit_or_zoom, controls).render().size(Some(64.0), Some(36.0)));
                    zoom_controls.push((id, ZoomControl::Toggle));
                }
                tools.push(control(zoom_in, Icon::Add, labels.zoom_in, &format!("{} · +", labels.zoom_in), !most, controls));
                zoom_controls.push((zoom_in.clone(), ZoomControl::In));
                tools.push(control(fit, Icon::Collapse, labels.fit_to_window, &format!("{} · 0", labels.fit_to_window), !fitted, controls));
                zoom_controls.push((fit.clone(), ZoomControl::Fit));
                // Only a picture turns; an animation or a video keeps its way up.
                if self.surface.is_none() && picture_shown {
                    let id = format!("{}:rotate", self.close_id);
                    tools.push(control(&id, Icon::Refresh, labels.rotate, &format!("{} · Ctrl+R", labels.rotate), true, controls));
                    zoom_controls.push((id, ZoomControl::RotateClockwise));
                }
            }
        }
        let picture = match (&self.surface, &self.image, &self.placeholder) {
            (Some((id, label)), _, _) => Some(Element::surface(id, label)),
            (None, Some(image), _) => Some(Element::raster_image(image.clone().full_resolution().checkerboard())),
            (None, None, Some(stand_in)) => Some(Element::raster_image(stand_in.clone())),
            (None, None, None) => None,
        };
        let waiting = self.surface.is_none() && self.image.is_none();
        let body = if let Some(picture) = picture {
            let mut view = Element::zoom_view(&view_id, picture).zoom_dismiss(&self.close_id);
            if lightbox {
                // A still picture zooms on a click; a surface, such as a video, does not. A lightbox
                // casts no shadow.
                if self.surface.is_none() {
                    view = view.zoom_click_toggles();
                }
            } else {
                view = view.shadow(chrome.shadow, Point::new(0.0, 12.0), 40.0, 0.0);
            }
            if let Some(size) = self.picture_size {
                view = view.zoom_content(size);
            }
            // A host's origin morphs the picture instead; see below.
            if let (Some(id), None) = (&self.return_focus, self.origin) {
                view = view.zoom_opened_from(id);
            }
            for (id, control) in zoom_controls {
                view = view.zoom_control(id, control);
            }
            let mut layers = vec![view];
            // The picture loads over its stand-in.
            if waiting && self.error.is_none() {
                layers.push(Element::column(vec![
                    Element::spacer().flex(1.0),
                    centered(
                        Spinner::new(format!("{}:loading", self.close_id), chrome.text)
                            .size(28.0)
                            .render()
                            .accessibility_label(labels.loading_image),
                        28.0,
                    ),
                    Element::spacer().flex(1.0),
                ]));
            }
            Element::stack(layers)
        } else {
            let message = self.error.as_deref().unwrap_or(labels.loading_image);
            let mut content = vec![Element::spacer().flex(1.0)];
            if self.error.is_none() {
                content.push(centered(
                    Spinner::new(format!("{}:loading", self.close_id), chrome.muted)
                        .size(32.0)
                        .render()
                        .accessibility_label(labels.loading_image),
                    32.0,
                ));
            }
            content.push(centered(
                Element::text(message)
                    .font_family(FontFamily::Sans)
                    .font_size(14.0)
                    .foreground(chrome.muted)
                    .size(Some((self.viewport.width - 144.0).clamp(80.0, 520.0)), None),
                64.0,
            ));
            content.push(Element::spacer().flex(1.0));
            Element::column(content).gap(12.0)
        };
        let [previous_label, next_label] = self
            .navigation_labels
            .clone()
            .unwrap_or_else(|| [labels.previous_image.into(), labels.next_image.into()]);
        let arrow = |id: &str, icon: Icon, label: &str, key: &str, enabled: bool| {
            Element::column(vec![
                Element::spacer().flex(1.0),
                Element::panel(control(id, icon, label, &format!("{label} · {key}"), enabled, controls))
                    .padding(4.0)
                    .background(chrome.surface)
                    .radius(crate::tokens().radius.panel * 1.5 + 4.0),
                Element::spacer().flex(1.0),
            ])
            .size(Some(44.0), None)
        };
        // The lightbox's arrows are never disabled: the gallery wraps around.
        let lightbox_arrow = |id: &str, icon: Icon, label: &str, key: &str| {
            Element::column(vec![
                Element::spacer().flex(1.0),
                lightbox_square(id, icon, label, &format!("{label} · {key}"), controls),
                Element::spacer().flex(1.0),
            ])
            .size(Some(LIGHTBOX_SQUARE), None)
        };
        let mut over = Vec::new();
        if self.total > 1 && !zoomed {
            if lightbox {
                if let Some(id) = &self.previous_action {
                    over.push(lightbox_arrow(id, Icon::ChevronLeft, &previous_label, "←"));
                }
                over.push(Element::spacer().flex(1.0));
                if let Some(id) = &self.next_action {
                    over.push(lightbox_arrow(id, Icon::ChevronRight, &next_label, "→"));
                }
            } else {
                if let Some(id) = &self.previous_action {
                    over.push(arrow(id, Icon::ArrowLeft, &previous_label, "←", self.index > 0));
                }
                over.push(Element::spacer().flex(1.0));
                if let Some(id) = &self.next_action {
                    over.push(arrow(id, Icon::ArrowRight, &next_label, "→", self.index + 1 < self.total));
                }
            }
        }
        let body = if column {
            let mut column = vec![close_control(&self.close_id, controls)];
            for (id, icon, label) in &self.actions {
                column.push(control(id, *icon, label, label, true, controls));
            }
            // Non-modal: the picture keeps its own focus scope, and the column takes no backdrop.
            Element::popover(
                body,
                Element::column(column).gap(ACTION_GAP).size(Some(36.0), None),
                format!("{}:actions", self.close_id),
                PopoverPlacement::RightCorner,
            )
            .popover_gap(ACTION_GAP)
            .popover_non_modal()
        } else {
            body
        };
        let body = if over.is_empty() { body } else { Element::stack(vec![body, Element::row(over)]) };
        let top = if lightbox {
            let left = match self.lead {
                Some(lead) => Element::row(vec![lead])
                    .max_width(LIGHTBOX_LEAD)
                    .clip_children()
                    .centered_y()
                    .flex(1.0),
                None => Element::column(titles).gap(2.0).flex(1.0).centered_y(),
            };
            let mut bar = vec![left];
            if !tools.is_empty() {
                bar.push(
                    Element::panel(Element::row(tools).gap(4.0))
                        .padding(3.0)
                        .background(chrome.raised)
                        .border(chrome.border, 1.0)
                        .radius(crate::tokens().radius.panel)
                        .centered_y(),
                );
            }
            bar.push(Element::spacer().size(Some(LIGHTBOX_CLOSE_ROOM), None));
            Element::row(bar).size(None, Some(LIGHTBOX_BAR))
        } else {
            let mut header = vec![Element::column(titles).gap(2.0).flex(1.0).centered_y()];
            // With a column of actions the close button leads it beside the picture instead.
            if !column {
                if !tools.is_empty() {
                    tools.push(Element::spacer().size(Some(8.0), Some(36.0)));
                }
                tools.push(close_control(&self.close_id, controls));
            }
            if !tools.is_empty() {
                header.push(
                    Element::panel(Element::row(tools).gap(2.0))
                        .padding(4.0)
                        .background(chrome.surface)
                        .radius(crate::tokens().radius.panel * 1.5 + 4.0)
                        .centered_y(),
                );
            }
            Element::row(header).gap(16.0).size(None, Some(HEADER))
        };
        let mut rows = vec![top, body.flex(1.0)];
        if let Some(footer) = self.footer {
            rows.push(footer);
        }
        if lightbox {
            if self.thumbnails.len() > 1 && !zoomed {
                let count = self.thumbnails.len();
                let strip: Vec<Element> = self
                    .thumbnails
                    .iter()
                    .enumerate()
                    .map(|(at, (id, image, label))| {
                        let face = match image {
                            Some(image) => Element::raster_image(image.clone().cover()),
                            None => Element::spacer().background(chrome.surface),
                        }
                        .size(Some(LIGHTBOX_THUMBNAIL), Some(LIGHTBOX_THUMBNAIL));
                        // The strip's outer corners are the larger, and its inner ones barely round.
                        let (outer, inner) = (crate::tokens().radius.card, crate::tokens().radius.control / 2.0);
                        let left = if at == 0 { outer } else { inner };
                        let right = if at + 1 == count { outer } else { inner };
                        let corners = CornerRadii {
                            top_left: left,
                            bottom_left: left,
                            top_right: right,
                            bottom_right: right,
                        };
                        let mut layers = vec![face];
                        if at != self.index {
                            let veil = Element::spacer()
                                .background(Color::rgba(0, 0, 0, VEIL))
                                .dim_on_hover(format!("{id}:veil"), f32::from(VEIL_HOVER) / f32::from(VEIL));
                            layers.push(veil);
                        }
                        let stack = Element::stack(layers).corner_radii(corners).clip_children().hover_region();
                        Element::clickable(id, stack)
                            .size(Some(LIGHTBOX_THUMBNAIL), Some(LIGHTBOX_THUMBNAIL))
                            .accessible(Access::new(Role::Button).name(label).selected(at == self.index))
                    })
                    .collect();
                rows.push(centered(
                    Element::horizontal_scroll(
                        format!("{}:thumbnails", self.close_id),
                        Element::row(strip).gap(LIGHTBOX_THUMBNAIL_GAP),
                    ),
                    LIGHTBOX_THUMBNAIL,
                ));
            }
        } else if self.thumbnails.len() > 1 {
            let strip: Vec<Element> = self
                .thumbnails
                .iter()
                .enumerate()
                .map(|(at, (id, image, label))| {
                    let face = match image {
                        Some(image) => Element::raster_image(image.clone().cover()),
                        None => Element::spacer().background(chrome.hover),
                    }
                    .radius(crate::tokens().radius.control)
                    .size(Some(THUMBNAIL), Some(THUMBNAIL));
                    let ring = if at == self.index { chrome.text } else { Color::TRANSPARENT };
                    Element::clickable(id, Element::panel(face).padding(2.0).border(ring, 2.0).radius(crate::tokens().radius.control + 2.0))
                        .size(Some(THUMBNAIL + 4.0), Some(THUMBNAIL + 4.0))
                        .accessible(Access::new(Role::Button).name(label).selected(at == self.index))
                })
                .collect();
            rows.push(centered(
                Element::horizontal_scroll(format!("{}:thumbnails", self.close_id), Element::row(strip).gap(6.0)),
                STRIP,
            ));
        } else if self.total > 1 {
            rows.push(
                Element::truncated_text(format!("{} {} {}", self.index + 1, labels.of, self.total))
                    .font_family(FontFamily::Sans)
                    .font_size(12.0)
                    .foreground(chrome.muted)
                    .center_content()
                    .size(None, Some(24.0)),
            );
        }
        // Known by an ID of its own: `close_id` is its close control's.
        let dialog = Access::new(Role::Dialog).id(format!("{}:dialog", self.close_id)).name(&self.title);
        let viewer = if lightbox {
            // The lightbox's close stands alone in the content's top right corner, over the rows.
            let close = Element::column(vec![
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    lightbox_square(
                        &self.close_id,
                        Icon::Close,
                        labels.close_image,
                        &format!("{} · Esc", labels.close_image),
                        controls,
                    ),
                ])
                .size(None, Some(LIGHTBOX_SQUARE)),
                Element::spacer().flex(1.0),
            ]);
            Element::stack(vec![Element::column(rows).gap(LIGHTBOX_GAP), close])
                .size(
                    Some((self.viewport.width - 2.0 * LIGHTBOX_MARGIN_X).max(0.0)),
                    Some((self.viewport.height - 2.0 * LIGHTBOX_MARGIN_Y).max(0.0)),
                )
                .accessible(dialog)
        } else {
            Element::column(rows)
                .gap(12.0)
                .padding(12.0)
                // A popover keeps 8 from the window's edges.
                .size(Some((self.viewport.width - 16.0).max(0.0)), Some((self.viewport.height - 16.0).max(0.0)))
                .accessible(dialog)
        };
        let initial = self.initial_focus.clone().unwrap_or_else(|| self.close_id.clone());
        // The arrows its hints show: back while there is one before, on while there is one after.
        // A lightbox wraps around, so both report whenever there is more than one picture.
        let (back, next) = if lightbox {
            (self.previous_action.filter(|_| self.total > 1), self.next_action.filter(|_| self.total > 1))
        } else {
            (
                self.previous_action.filter(|_| self.index > 0),
                self.next_action.filter(|_| self.index + 1 < self.total),
            )
        };
        let mut popover = Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
            // The window's content stays faintly there behind the picture.
            .popover_scrim(self.scrim.unwrap_or(Color::rgba(0, 0, 0, 232)))
            .popover_animation_ms(crate::tokens().motion.enter_ms)
            // The picture grows out of its thumbnail; the rest fades in.
            .popover_open_scale(1.0)
            .popover_backdrop_dismiss(true)
            .popover_initial_focus(initial)
            .popover_arrows(back, next);
        if let Some(id) = self.return_focus {
            popover = popover.popover_return_focus(id);
        }
        if let Some(origin) = self.origin {
            popover = popover
                .popover_morph(origin, ORIGIN_RADIUS)
                .popover_animation_ms(MORPH_OPEN_MS)
                .popover_close_animation_ms(MORPH_CLOSE_MS);
        }
        popover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Key, RasterImageFormat, Rect, UiAction, UiEvent, UiTree};
    use std::sync::Arc;

    const WINDOW: Size = Size::new(900.0, 620.0);

    fn still() -> RasterImage {
        RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap()
    }

    /// A viewer on a picture of `size` pixels, with the zoom controls.
    fn sized(size: Size) -> ImageViewer {
        ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
            .viewport(WINDOW)
            .image(still())
            .picture_size(size)
            .zoom_actions("out", "fit", "in")
    }

    fn tree(viewer: ImageViewer) -> (UiTree, Frame) {
        let mut tree = UiTree::new(viewer.render());
        tree.set_reduced_motion(true);
        let frame = tree.layout(WINDOW);
        (tree, frame)
    }

    fn picture(frame: &Frame) -> Rect {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RasterImage { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("the picture is drawn")
    }

    /// The box the picture zooms in.
    fn room(frame: &Frame) -> Rect {
        frame.regions.iter().find(|region| region.kind == crate::HitKind::Zoom).expect("a zoom view").rect
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn click(tree: &mut UiTree, frame: &Frame, id: &str) -> Vec<UiAction> {
        let Some(region) = frame.regions.iter().find(|region| region.id == id) else {
            return Vec::new();
        };
        let at = center(region.rect);
        tree.handle_event(UiEvent::PointerMove(at));
        tree.handle_event(UiEvent::PointerDown(at));
        tree.handle_event(UiEvent::PointerUp(at))
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.05
    }

    fn zoomed(actions: &[UiAction]) -> Option<(f32, bool)> {
        actions.iter().find_map(|action| match action {
            UiAction::Zoomed { scale, fitted, .. } => Some((*scale, *fitted)),
            _ => None,
        })
    }

    #[test]
    fn the_viewer_is_a_dialog_named_by_its_title_and_its_icons_are_named_enabled_or_not() {
        let (_, frame) = tree(sized(Size::new(2400.0, 1600.0)).zoom_level(0.3, true));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let dialog = node("close:dialog");
        assert_eq!((dialog.role, dialog.name.as_str()), (Role::Dialog, "picture.png"));
        assert!(!dialog.actionable);
        let labels = crate::labels();
        // Fitted, there is nothing to zoom out of: those two are drawn disabled.
        for (id, name, enabled) in [
            ("out", labels.zoom_out, false),
            ("fit", labels.fit_to_window, false),
            ("in", labels.zoom_in, true),
            ("close", labels.close_image, true),
            ("close:rotate", labels.rotate, true),
        ] {
            let said = node(id);
            assert_eq!((said.role, said.name.as_str()), (Role::Button, name));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled), "{id}");
        }
        // The level the host passed back, as a percentage that toggles.
        assert_eq!(node("close:zoom-level").name, format!("{} 30 %", labels.zoom_level));
        // The picture itself, named by its text alternative.
        let shown = frame.access.iter().find(|node| node.role == Role::Image).unwrap();
        assert_eq!(shown.name, "A picture");
    }

    #[test]
    fn host_actions_sit_before_the_zoom_controls_and_report_their_ids() {
        let viewer = sized(Size::new(400.0, 300.0))
            .action("save", Icon::ArrowDown, "Save as…")
            .action("", Icon::Copy, "Ignored")
            .action("copy", Icon::Copy, "Copy link");
        let (mut tree, frame) = tree(viewer);
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        assert_eq!((node("save").role, node("save").name.as_str()), (Role::Button, "Save as…"));
        assert!(node("save").actionable && !node("save").disabled);
        assert!(!frame.access.iter().any(|node| node.name == "Ignored"));
        let left = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect.x;
        assert!(left("save") < left("copy") && left("copy") < left("out"));
        assert_eq!(click(&mut tree, &frame, "save"), vec![UiAction::Clicked { id: "save".into() }]);
    }

    #[test]
    fn a_small_picture_keeps_its_own_size_centered_and_a_large_one_fits() {
        let (_, frame) = tree(sized(Size::new(300.0, 340.0)));
        let small = picture(&frame);
        assert_eq!((small.width, small.height), (300.0, 340.0), "never enlarged");
        let middle = center(small);
        assert!(close(middle.x, WINDOW.width / 2.0), "{small:?}");
        let (_, frame) = tree(sized(Size::new(2400.0, 1600.0)));
        let large = picture(&frame);
        assert!(close(large.width / large.height, 1.5), "at its proportions");
        // It fills its room one way and fits inside it the other.
        let room = self::room(&frame);
        assert!(close(large.height, room.height) && large.width <= room.width, "{large:?} {room:?}");
    }

    #[test]
    fn the_wheel_zooms_about_the_pointer_and_reports_the_scale() {
        let (mut tree, frame) = tree(sized(Size::new(2400.0, 1600.0)));
        let before = picture(&frame);
        let at = Point::new(before.x + before.width * 0.25, before.y + before.height * 0.75);
        let pixel = |rect: Rect| ((at.x - rect.x) / rect.width, (at.y - rect.y) / rect.height);
        let actions = tree.handle_event(UiEvent::Scroll { position: at, delta_y: 30.0 });
        let (scale, fitted) = zoomed(&actions).expect("a zoom is reported");
        assert!(!fitted && close(scale, before.width / 2400.0 * 1.3));
        let after = picture(&tree.layout(WINDOW));
        assert!(close(after.width, before.width * 1.3));
        let (a, b) = (pixel(before), pixel(after));
        assert!((a.0 - b.0).abs() < 0.001 && (a.1 - b.1).abs() < 0.001, "the same pixel under the pointer");
        // A notch the other way comes back to the fit.
        tree.handle_event(UiEvent::Scroll { position: at, delta_y: -30.0 });
        assert_eq!(picture(&tree.layout(WINDOW)), before);
        // A touchpad's scroll moves a fitted picture nowhere; a pinch zooms.
        tree.handle_event(UiEvent::ScrollDirect { position: at, delta_y: 40.0 });
        assert_eq!(picture(&tree.layout(WINDOW)), before);
        assert!(zoomed(&tree.handle_event(UiEvent::Zoom { position: at, factor: 2.0 })).is_some());
        assert!(close(picture(&tree.layout(WINDOW)).width, before.width * 2.0));
    }

    #[test]
    fn a_drag_moves_a_zoomed_picture_but_never_off_an_edge() {
        let (mut tree, frame) = tree(sized(Size::new(2400.0, 1600.0)));
        let fitted = picture(&frame);
        let mut frame = frame;
        for _ in 0..3 {
            let actions = click(&mut tree, &frame, "in");
            assert!(actions.contains(&UiAction::Clicked { id: "in".into() }));
            assert!(zoomed(&actions).is_some());
            frame = tree.layout(WINDOW);
        }
        let zoomed_in = picture(&frame);
        assert!(close(zoomed_in.width, fitted.width * 1.5f32.powi(3)));
        let start = center(zoomed_in);
        tree.handle_event(UiEvent::PointerMove(start));
        tree.handle_event(UiEvent::PointerDown(start));
        tree.handle_event(UiEvent::PointerMove(Point::new(start.x + 40.0, start.y)));
        tree.handle_event(UiEvent::PointerMove(Point::new(start.x + 100.0, start.y + 20.0)));
        let moved = picture(&tree.layout(WINDOW));
        assert!(close(moved.x, zoomed_in.x + 100.0) && close(moved.y, zoomed_in.y + 20.0));
        // Far to the right: its left edge stops at the room's.
        tree.handle_event(UiEvent::PointerMove(Point::new(start.x + 5_000.0, start.y)));
        let stopped = picture(&tree.layout(WINDOW));
        assert!(close(stopped.x, room(&frame).x), "{stopped:?}");
        // Letting go of a drag closes nothing.
        assert!(tree.handle_event(UiEvent::PointerUp(Point::new(start.x + 5_000.0, start.y))).is_empty());
    }

    #[test]
    fn a_click_beside_the_picture_closes_and_a_double_click_on_it_zooms() {
        let (mut tree, frame) = tree(sized(Size::new(300.0, 340.0)));
        let shown = picture(&frame);
        let beside = Point::new(shown.x - 60.0, center(shown).y);
        tree.handle_event(UiEvent::PointerMove(beside));
        tree.handle_event(UiEvent::PointerDown(beside));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(beside)), vec![UiAction::Clicked { id: "close".into() }]);
        // A click on the picture itself does not close it.
        let on = center(shown);
        tree.handle_event(UiEvent::PointerDown(on));
        assert!(tree.handle_event(UiEvent::PointerUp(on)).is_empty());
        let double = UiEvent::PointerDownEditor { position: on, shift: false, alt: false, ctrl: false, click_count: 2 };
        assert_eq!(zoomed(&tree.handle_event(double.clone())), Some((2.5, false)));
        tree.handle_event(UiEvent::PointerUp(on));
        assert!(close(picture(&tree.layout(WINDOW)).width, 750.0));
        assert_eq!(zoomed(&tree.handle_event(double)), Some((1.0, true)));
    }

    #[test]
    fn keys_zoom_fit_show_its_own_size_and_turn_it() {
        let (mut tree, _) = tree(sized(Size::new(2400.0, 1600.0)));
        let press = |tree: &mut UiTree, key: Key, ctrl: bool| {
            tree.handle_event(UiEvent::Key { key, shift: false, ctrl, alt: false })
        };
        let fit = tree.zoom_scale("close:view:0:picture.png").unwrap();
        assert!(zoomed(&press(&mut tree, Key::Plus, false)).is_some());
        assert!(close(tree.zoom_scale("close:view:0:picture.png").unwrap(), fit * 1.5));
        press(&mut tree, Key::Num1, false);
        assert_eq!(tree.zoom_scale("close:view:0:picture.png"), Some(1.0));
        press(&mut tree, Key::Num0, false);
        assert_eq!(tree.zoom_scale("close:view:0:picture.png"), Some(fit));
        // A quarter turn: taller than wide now, and fitted again.
        press(&mut tree, Key::R, true);
        let turned = picture(&tree.layout(WINDOW));
        assert!(close(turned.height / turned.width, 1.5), "{turned:?}");
        let turns = tree.frame().commands.iter().find_map(|command| match command {
            DrawCommand::RasterImage { image, .. } => Some(image.quarter_turns()),
            _ => None,
        });
        assert_eq!(turns, Some(1));
    }

    #[test]
    fn a_zoom_is_animated_unless_motion_is_reduced() {
        let mut tree = UiTree::new(sized(Size::new(2400.0, 1600.0)).render());
        let start = web_time::Instant::now();
        let fitted = picture(&tree.layout_at(WINDOW, start));
        let frame = tree.frame().clone();
        let region = frame.regions.iter().find(|region| region.id == "in").unwrap().rect;
        tree.handle_event_at(UiEvent::PointerMove(center(region)), start);
        tree.handle_event_at(UiEvent::PointerDown(center(region)), start);
        tree.handle_event_at(UiEvent::PointerUp(center(region)), start);
        let halfway = picture(&tree.layout_at(WINDOW, start + web_time::Duration::from_millis(60)));
        assert!(halfway.width > fitted.width && halfway.width < fitted.width * 1.5 - 1.0, "{halfway:?}");
        assert!(tree.is_animating());
        let there = picture(&tree.layout_at(WINDOW, start + web_time::Duration::from_secs(1)));
        assert!(close(there.width, fitted.width * 1.5));
    }

    #[test]
    fn the_picture_grows_out_of_its_thumbnail_only_when_the_viewer_opens() {
        let base = Element::column(vec![
            Element::clickable("thumb", Element::spacer()).size(Some(40.0), Some(30.0)),
            Element::spacer().flex(1.0),
        ]);
        let viewer = |index: usize| {
            ImageViewer::new(base.clone(), true, "picture.png", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .image(still())
                .picture_size(Size::new(2400.0, 1600.0))
                .gallery(index, 2)
                .return_focus("thumb")
                .render()
        };
        let start = web_time::Instant::now();
        let mut tree = UiTree::new(viewer(0));
        let opening = picture(&tree.layout_at(WINDOW, start));
        assert!(opening.width < 40.5 && opening.x < 40.0 && opening.y < 30.0, "at its thumbnail: {opening:?}");
        let later = start + web_time::Duration::from_secs(1);
        let settled = picture(&tree.layout_at(WINDOW, later));
        assert!(settled.width > 400.0);
        // The next picture of the gallery is at its fit at once.
        tree.set_root(viewer(1));
        assert_eq!(picture(&tree.layout_at(WINDOW, later)), settled);
        // Under reduced motion it opens fitted.
        let mut still_tree = UiTree::new(viewer(0));
        still_tree.set_reduced_motion(true);
        assert_eq!(picture(&still_tree.layout_at(WINDOW, start)), settled);
    }

    #[test]
    fn the_arrow_keys_do_what_the_hints_say_and_stop_at_the_ends() {
        let gallery = |index: usize| {
            ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .image(still())
                .gallery(index, 3)
                .previous_action("before")
                .next_action("after")
        };
        let press = |index: usize, key: Key| {
            let (mut tree, _) = tree(gallery(index));
            tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false })
        };
        let clicked = |id: &str| vec![UiAction::Clicked { id: id.into() }];
        assert_eq!(press(1, Key::ArrowLeft), clicked("before"));
        assert_eq!(press(1, Key::ArrowRight), clicked("after"));
        // Nothing before the first, nothing after the last.
        assert!(press(0, Key::ArrowLeft).is_empty());
        assert!(press(2, Key::ArrowRight).is_empty());
    }

    #[test]
    fn thumbnails_take_the_place_of_the_count_and_report_their_ids() {
        let viewer = |thumbnails: bool| {
            let mut viewer = sized(Size::new(400.0, 300.0)).gallery(1, 3).previous_action("before").next_action("after");
            if thumbnails {
                for at in 0..3 {
                    viewer = viewer.thumbnail(format!("thumb:{at}"), (at != 2).then(still), format!("Picture {at}"));
                }
            }
            viewer
        };
        let (_, frame) = tree(viewer(false));
        let counted = |frame: &Frame| {
            frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text.starts_with("2 ")))
        };
        assert!(counted(&frame));
        let (mut tree, frame) = tree(viewer(true));
        assert!(!counted(&frame));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().clone();
        assert_eq!(node("thumb:1").selected, Some(true));
        assert_eq!((node("thumb:0").selected, node("thumb:0").name.as_str()), (Some(false), "Picture 0"));
        assert_eq!(click(&mut tree, &frame, "thumb:2"), vec![UiAction::Clicked { id: "thumb:2".into() }]);
    }

    #[test]
    fn a_surface_is_shown_and_zoomed_as_a_picture_is_but_does_not_turn() {
        let viewer = ImageViewer::new(Element::spacer(), true, "loop.gif", "close", ComponentPalette::neutral_dark())
            .viewport(WINDOW)
            .picture_size(Size::new(300.0, 340.0))
            .zoom_actions("out", "fit", "in")
            .surface("animation:a", "A loop")
            .image(still());
        let (mut tree, frame) = tree(viewer);
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::RasterImage { .. })));
        let surface = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Surface { rect, id, fit, .. } => Some((*rect, id.clone(), *fit)),
                    _ => None,
                })
                .expect("the surface is drawn")
        };
        let (rect, id, fit) = surface(&frame);
        assert_eq!((rect.width, rect.height, id.as_str(), fit), (300.0, 340.0, "animation:a", crate::SurfaceFit::Contain));
        assert!(!frame.access.iter().any(|node| node.id == "close:rotate"));
        click(&mut tree, &frame, "in");
        assert!(close(surface(&tree.layout(WINDOW)).0.width, 450.0));
    }

    #[test]
    fn the_arrows_are_named_for_images_unless_the_host_names_them() {
        let names = |labels: Option<[&str; 2]>| {
            let mut viewer = ImageViewer::new(Element::spacer(), true, "report.pdf", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .gallery(1, 3)
                .previous_action("before")
                .next_action("after");
            if let Some([previous, next]) = labels {
                viewer = viewer.navigation_labels(previous, next);
            }
            let (_, frame) = tree(viewer);
            let name = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().name.clone();
            [name("before"), name("after")]
        };
        let labels = crate::labels();
        assert_eq!(names(None), [labels.previous_image, labels.next_image]);
        assert_eq!(names(Some(["Previous page", "Next page"])), ["Previous page", "Next page"]);
    }

    #[test]
    fn a_stand_in_shows_at_the_pictures_size_until_the_picture_arrives() {
        let viewer = ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
            .viewport(WINDOW)
            .picture_size(Size::new(300.0, 340.0))
            .placeholder(still());
        let (_, frame) = tree(viewer);
        assert_eq!(picture(&frame).width, 300.0);
        assert!(frame.access.iter().any(|node| node.name == crate::labels().loading_image));
    }

    /// A lightbox of `size` pixels with its controls: a save action, the zoom toggle and a tool.
    fn lightbox(size: Size) -> ImageViewer {
        sized(size).layout(ViewerLayout::Lightbox).action("save", Icon::ArrowDown, "Save as…")
    }

    /// The glyph drawn in the control `id`: the icon whose box lies within the control's.
    fn glyph(frame: &Frame, id: &str) -> Option<Icon> {
        let rect = frame.access.iter().find(|node| node.id == id)?.rect;
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { icon, rect: at, .. } if rect.contains(center(*at)) => Some(*icon),
            _ => None,
        })
    }

    #[test]
    fn the_lightbox_puts_close_in_the_corner_and_its_group_before_it() {
        let (_, frame) = tree(lightbox(Size::new(400.0, 300.0)));
        // The backdrop's region takes the close ID too, so a control's own box is read from its node.
        let rect = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().rect;
        let close = rect("close");
        assert_eq!((close.width, close.height), (40.0, 40.0));
        assert_eq!((WINDOW.width - close.x - close.width, close.y), (24.0, 36.0));
        // The group's buttons are 32 square, and the group ends 52 short of the content's edge:
        // its padding of 3 stands after its last button, the host's action. The zoom toggle leads.
        let (zoom, save) = (rect("in"), rect("save"));
        assert_eq!((save.width, save.height, zoom.width, zoom.height), (32.0, 32.0, 32.0, 32.0));
        assert!(zoom.x < save.x);
        assert_eq!(save.x + save.width + 3.0, WINDOW.width - 24.0 - 52.0);
        // Fitted, the zoom toggle offers zoom in.
        assert_eq!(glyph(&frame, "in"), Some(Icon::ZoomIn));
    }

    #[test]
    fn a_lead_sits_at_the_top_left_and_the_title_is_only_the_dialogs_name() {
        let viewer = lightbox(Size::new(400.0, 300.0)).lead(Element::text("Ana · 12:04"));
        let (_, frame) = tree(viewer);
        let position = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "Ana · 12:04" => Some(*position),
                _ => None,
            })
            .expect("the lead is drawn");
        assert!(close(position.x, 24.0) && (36.0..76.0).contains(&position.y), "{position:?}");
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "picture.png")));
        let dialog = frame.access.iter().find(|node| node.id == "close:dialog").unwrap();
        assert_eq!((dialog.role, dialog.name.as_str()), (Role::Dialog, "picture.png"));
    }

    #[test]
    fn a_lightbox_gallery_keeps_both_arrows_and_squares_its_thumbnails_without_a_count() {
        let gallery = |thumbnails: bool| {
            let mut viewer = lightbox(Size::new(400.0, 300.0)).gallery(0, 3).previous_action("before").next_action("after");
            if thumbnails {
                for at in 0..3 {
                    viewer = viewer.thumbnail(format!("thumb:{at}"), (at != 2).then(still), format!("Picture {at}"));
                }
            }
            viewer
        };
        let (_, frame) = tree(gallery(false));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().clone();
        assert!(!node("before").disabled && !node("after").disabled, "the arrows wrap around");
        assert_eq!((glyph(&frame, "before"), glyph(&frame, "after")), (Some(Icon::ChevronLeft), Some(Icon::ChevronRight)));
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text.starts_with("1 "))));
        let (_, frame) = tree(gallery(true));
        for at in 0..3 {
            let rect = frame.regions.iter().find(|region| region.id == format!("thumb:{at}")).unwrap().rect;
            assert_eq!((rect.width, rect.height), (40.0, 40.0));
        }
    }

    #[test]
    fn a_zoomed_lightbox_keeps_its_group_close_and_lead_and_offers_zoom_out() {
        let viewer = lightbox(Size::new(2400.0, 1600.0))
            .lead(Element::text("Ana"))
            .gallery(1, 3)
            .previous_action("before")
            .next_action("after")
            .thumbnail("thumb:0", None, "Picture 0")
            .thumbnail("thumb:1", None, "Picture 1")
            .zoom_level(2.5, false);
        let (_, frame) = tree(viewer);
        let shown = |id: &str| frame.regions.iter().any(|region| region.id == id);
        // The group stays, so keyboard users who reach its toggle can zoom out again.
        assert!(shown("close") && shown("save") && shown("in"));
        assert_eq!(glyph(&frame, "in"), Some(Icon::ZoomOut));
        assert_eq!(frame.access.iter().find(|node| node.id == "in").unwrap().name, crate::labels().zoom_out);
        for id in ["before", "after", "thumb:0"] {
            assert!(!shown(id), "{id}");
        }
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Ana")));
    }

    #[test]
    fn a_tool_is_the_last_control_of_the_group_in_either_layout() {
        let menu = || Element::clickable("menu", Element::spacer()).size(Some(32.0), Some(32.0));
        let left = |frame: &Frame, id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect.x;
        let (_, frame) = tree(sized(Size::new(400.0, 300.0)).action("save", Icon::ArrowDown, "Save as…").tool(menu()));
        assert!(left(&frame, "save") < left(&frame, "menu") && left(&frame, "menu") < left(&frame, "out"));
        let (_, frame) = tree(lightbox(Size::new(400.0, 300.0)).tool(menu()));
        assert!(left(&frame, "in") < left(&frame, "save") && left(&frame, "save") < left(&frame, "menu"));
    }

    #[test]
    fn the_scrim_paints_the_backdrop_in_its_color() {
        let scrim = Color::rgba(10, 20, 30, 200);
        let (_, frame) = tree(sized(Size::new(400.0, 300.0)).scrim(scrim));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == scrim)));
    }

    #[test]
    fn a_click_on_a_lightbox_picture_zooms_it_and_the_next_fits_it() {
        let (mut tree, frame) = tree(lightbox(Size::new(2400.0, 1600.0)));
        let first = click(&mut tree, &frame, "close:view:0:picture.png");
        assert_eq!(zoomed(&first).map(|(_, fitted)| fitted), Some(false));
        let frame = tree.layout(WINDOW);
        let second = click(&mut tree, &frame, "close:view:0:picture.png");
        assert!(matches!(zoomed(&second), Some((_, true))), "{second:?}");
    }

    #[test]
    fn a_click_does_not_zoom_a_surface_in_the_lightbox_but_the_wheel_still_does() {
        // A video or an animation shown as a surface keeps its fit on a click, and has no zoom toggle.
        let video = ImageViewer::new(Element::spacer(), true, "loop.gif", "close", ComponentPalette::neutral_dark())
            .viewport(WINDOW)
            .picture_size(Size::new(300.0, 340.0))
            .zoom_actions("out", "fit", "in")
            .surface("animation:a", "A loop")
            .layout(ViewerLayout::Lightbox);
        let (mut tree, frame) = tree(video);
        let view = "close:view:0:loop.gif";
        let fitted = tree.zoom_scale(view).unwrap();
        assert!(zoomed(&click(&mut tree, &frame, view)).is_none());
        assert_eq!(tree.zoom_scale(view), Some(fitted));
        assert!(!frame.regions.iter().any(|region| region.id == "in"));
        // The wheel still zooms it, as a zoom view's does.
        let at = center(frame.regions.iter().find(|region| region.id == view).unwrap().rect);
        assert!(zoomed(&tree.handle_event(UiEvent::Scroll { position: at, delta_y: 30.0 })).is_some());
    }

    /// The action column, the origin's morph and the lightbox of `ImageViewer::lightbox`.
    mod actions_and_lightbox {
        use super::super::*;
        use crate::{DrawCommand, Frame, Point, RasterImageFormat, Rect, UiAction, UiEvent, UiTree};
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        const WINDOW: Size = Size::new(900.0, 620.0);

        fn picture(frame: &Frame) -> Rect {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::RasterImage { rect, .. } => Some(*rect),
                    _ => None,
                })
                .expect("the picture is drawn")
        }

        fn click(tree: &mut UiTree, frame: &Frame, id: &str) -> Vec<UiAction> {
            let Some(region) = frame.regions.iter().find(|region| region.id == id) else {
                return Vec::new();
            };
            let center = Point::new(region.rect.x + region.rect.width / 2.0, region.rect.y + region.rect.height / 2.0);
            tree.handle_event(UiEvent::PointerMove(center));
            tree.handle_event(UiEvent::PointerDown(center));
            tree.handle_event(UiEvent::PointerUp(center))
        }

        /// The viewer over `WINDOW` with `picture` and the given origin, and its two actions.
        fn with_actions(origin: Option<Rect>, gallery: bool) -> Element {
            let picture = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
            let mut viewer = ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .image(picture)
                .action("copy", Icon::Copy, "Copy image")
                .action("annotate", Icon::Edit, "Annotate image");
            if gallery {
                viewer = viewer.gallery(1, 3).previous_action("before").next_action("after");
            }
            if let Some(origin) = origin {
                viewer = viewer.origin(origin);
            }
            viewer.render()
        }

        /// The control's own region: the viewer's backdrop shares the close control's ID and is painted first.
        fn region(frame: &Frame, id: &str) -> Rect {
            frame.regions.iter().rev().find(|region| region.id == id).expect("the control is in the frame").rect
        }

        fn radius(frame: &Frame) -> f32 {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::RasterImage { radius, .. } => Some(*radius),
                    _ => None,
                })
                .expect("the picture is drawn")
        }

        fn close_enough(rect: Rect, other: Rect) -> bool {
            [(rect.x, other.x), (rect.y, other.y), (rect.width, other.width), (rect.height, other.height)]
                .iter()
                .all(|(a, b)| (a - b).abs() < 0.01)
        }

        #[test]
        fn the_actions_stand_in_a_column_under_the_close_button_just_outside_the_picture_corner() {
            let mut tree = UiTree::new(with_actions(None, true));
            tree.set_reduced_motion(true);
            let frame = tree.layout(WINDOW);
            let (close, copy, annotate) = (region(&frame, "close"), region(&frame, "copy"), region(&frame, "annotate"));
            // Close first, then each action below it.
            assert!(close.y < copy.y && copy.y < annotate.y, "{close:?} {copy:?} {annotate:?}");
            assert_eq!((copy.x, annotate.x), (close.x, close.x));
            // The picture stops short of the next arrow, so the column fits to its right with a gap.
            let corner = picture(&frame);
            assert_eq!(close.x, corner.x + corner.width + ACTION_GAP);
            assert_eq!(close.y, corner.y);
            assert!(close.x + close.width <= WINDOW.width - 8.0);
        }

        #[test]
        fn with_no_room_beside_the_picture_the_actions_lie_inside_its_top_right_corner() {
            let mut tree = UiTree::new(with_actions(None, false));
            tree.set_reduced_motion(true);
            let frame = tree.layout(WINDOW);
            let corner = picture(&frame);
            let close = region(&frame, "close");
            assert_eq!(close.x, corner.x + corner.width - ACTION_GAP - close.width);
            assert_eq!(close.y, corner.y + ACTION_GAP);
        }

        #[test]
        fn an_action_reports_its_id_when_it_is_clicked() {
            let mut tree = UiTree::new(with_actions(None, false));
            tree.set_reduced_motion(true);
            let frame = tree.layout(WINDOW);
            assert_eq!(click(&mut tree, &frame, "annotate"), vec![UiAction::Clicked { id: "annotate".into() }]);
            let node = frame.access.iter().find(|node| node.id == "copy").unwrap();
            assert_eq!((node.role, node.name.as_str()), (Role::Button, "Copy image"));
        }

        #[test]
        fn with_an_origin_the_picture_grows_from_its_thumbnail_and_settles_in_350_ms() {
            let origin = Rect::new(40.0, 500.0, 120.0, 90.0);
            let mut settled = UiTree::new(with_actions(Some(origin), false));
            settled.set_reduced_motion(true);
            let end = picture(&settled.layout(WINDOW));

            let start = Instant::now();
            let mut tree = UiTree::new(with_actions(Some(origin), false));
            let first = tree.layout_at(WINDOW, start);
            assert!(close_enough(picture(&first), origin), "{:?}", picture(&first));
            assert_eq!(radius(&first), ORIGIN_RADIUS);

            let middle = picture(&tree.layout_at(WINDOW, start + Duration::from_millis(175)));
            assert!(middle.width > origin.width && middle.width < end.width, "{middle:?}");

            let done = tree.layout_at(WINDOW, start + Duration::from_millis(350));
            assert!(close_enough(picture(&done), end), "{:?} != {end:?}", picture(&done));
            assert_eq!(radius(&done), PICTURE_RADIUS);
        }

        #[test]
        fn closing_shrinks_the_picture_back_over_250_ms_and_the_viewer_stays_until_it_has() {
            let origin = Rect::new(40.0, 500.0, 120.0, 90.0);
            let mut tree = UiTree::new(with_actions(Some(origin), false));
            let start = Instant::now();
            tree.layout_at(WINDOW, start);
            let end = picture(&tree.layout_at(WINDOW, start + Duration::from_millis(500)));

            let closing = start + Duration::from_millis(500);
            // The host takes the viewer away: the tree keeps painting its last frame while it closes.
            tree.set_root(Element::spacer());
            let gone = tree.layout_at(WINDOW, closing);
            assert!(gone.commands.iter().any(|command| matches!(command, DrawCommand::RasterImage { .. })));
            let halfway = picture(&tree.layout_at(WINDOW, closing + Duration::from_millis(125)));
            assert!(halfway.width < end.width && halfway.width > origin.width, "{halfway:?}");
            assert!(tree.layout_at(WINDOW, closing + Duration::from_millis(400)).commands.iter().all(|command| !matches!(command, DrawCommand::RasterImage { .. })));
        }

        #[test]
        fn without_an_origin_the_viewer_only_fades_and_reduced_motion_is_at_once() {
            let mut plain = UiTree::new(with_actions(None, false));
            let start = Instant::now();
            let fresh = plain.layout_at(WINDOW, start);
            let mut settled = UiTree::new(with_actions(None, false));
            settled.set_reduced_motion(true);
            let end = picture(&settled.layout(WINDOW));
            // The surface's own 4 px entrance slide is all that differs from the settled place.
            let slid = picture(&fresh);
            assert!(close_enough(Rect::new(slid.x, end.y, slid.width, slid.height), end) && (slid.y - end.y).abs() <= 4.0, "{slid:?} {end:?}");
            assert_eq!(radius(&fresh), PICTURE_RADIUS);

            let mut still = UiTree::new(with_actions(Some(Rect::new(40.0, 500.0, 120.0, 90.0)), false));
            still.set_reduced_motion(true);
            let frame = still.layout_at(WINDOW, start);
            assert!(close_enough(picture(&frame), end), "{:?}", picture(&frame));
            assert_eq!(radius(&frame), PICTURE_RADIUS);
        }

        /// A lightbox of a picture `natural` pixels large in `window`, with a caption, two actions and an origin if given.
        fn lightbox(window: Size, natural: (f32, f32), origin: Option<Rect>) -> Element {
            let picture = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
            let mut viewer = ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
                .viewport(window)
                .image(picture)
                .natural_size(natural.0, natural.1)
                .lightbox(true)
                .caption("dusk.png")
                .action("copy", Icon::Copy, "Copy image")
                .action("annotate", Icon::Edit, "Annotate image");
            if let Some(origin) = origin {
                viewer = viewer.origin(origin);
            }
            viewer.render()
        }

        fn tap(tree: &mut UiTree, at: Point) -> Vec<UiAction> {
            tree.handle_event(UiEvent::PointerMove(at));
            tree.handle_event(UiEvent::PointerDown(at));
            tree.handle_event(UiEvent::PointerUp(at))
        }

        #[test]
        fn a_lightbox_fits_its_picture_to_640_wide_at_its_aspect_and_centres_it_but_never_enlarges() {
            let window = Size::new(1408.0, 844.0);
            let mut tree = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            tree.set_reduced_motion(true);
            let fitted = picture(&tree.layout(window));
            assert!((fitted.width - 640.0).abs() < 0.01 && (fitted.height - 426.67).abs() < 0.01, "{fitted:?}");
            assert!((fitted.x + fitted.width / 2.0 - 704.0).abs() < 0.01 && (fitted.y + fitted.height / 2.0 - 422.0).abs() < 0.01, "{fitted:?}");

            let mut small = UiTree::new(lightbox(window, (300.0, 200.0), None));
            small.set_reduced_motion(true);
            let shown = picture(&small.layout(window));
            assert_eq!((shown.width, shown.height), (300.0, 200.0));
        }

        #[test]
        fn a_lightbox_column_of_close_and_actions_sits_six_pixels_beside_the_picture_top_aligned() {
            let window = Size::new(1408.0, 844.0);
            let mut tree = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            tree.set_reduced_motion(true);
            let frame = tree.layout(window);
            let corner = picture(&frame);
            let (close, copy) = (region(&frame, "close"), region(&frame, "copy"));
            assert_eq!(close.x, corner.x + corner.width + ACTION_GAP);
            assert_eq!(close.y, corner.y);
            assert!(close.y < copy.y && copy.x == close.x, "{close:?} {copy:?}");
        }

        #[test]
        fn a_lightbox_in_a_narrow_window_puts_its_column_inside_the_picture_corner() {
            let window = Size::new(700.0, 844.0);
            let mut tree = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            tree.set_reduced_motion(true);
            let frame = tree.layout(window);
            let corner = picture(&frame);
            let close = region(&frame, "close");
            assert_eq!(corner.width, 640.0);
            assert_eq!(close.x, corner.x + corner.width - ACTION_GAP - close.width);
            assert_eq!(close.y, corner.y + ACTION_GAP);
        }

        #[test]
        fn a_lightbox_draws_its_caption_under_the_picture_and_no_header_or_count_for_one_picture() {
            let window = Size::new(1408.0, 844.0);
            let mut tree = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            tree.set_reduced_motion(true);
            let frame = tree.layout(window);
            let corner = picture(&frame);
            let texts: Vec<(String, Point)> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. } => Some((text.clone(), *position)),
                    _ => None,
                })
                .collect();
            let (_, at) = texts.iter().find(|(text, _)| text == "dusk.png").expect("the caption is drawn");
            assert!(at.y >= corner.y + corner.height, "{at:?} {corner:?}");
            assert!(texts.iter().all(|(text, _)| text != "picture.png" && !text.contains(" of ")), "{texts:?}");
        }

        #[test]
        fn a_click_on_the_backdrop_or_on_the_picture_closes_a_lightbox() {
            let window = Size::new(1408.0, 844.0);
            let closed = vec![UiAction::Clicked { id: "close".into() }];
            let mut backdrop = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            backdrop.set_reduced_motion(true);
            backdrop.layout(window);
            assert_eq!(tap(&mut backdrop, Point::new(5.0, 5.0)), closed);

            let mut on_picture = UiTree::new(lightbox(window, (1200.0, 800.0), None));
            on_picture.set_reduced_motion(true);
            let corner = picture(&on_picture.layout(window));
            assert_eq!(tap(&mut on_picture, Point::new(corner.x + 100.0, corner.y + 100.0)), closed);
        }

        #[test]
        fn a_lightbox_morphs_its_picture_from_the_thumbnail_to_the_fitted_rectangle_over_350_ms() {
            let window = Size::new(1408.0, 844.0);
            let origin = Rect::new(40.0, 500.0, 120.0, 90.0);
            let mut settled = UiTree::new(lightbox(window, (1200.0, 800.0), Some(origin)));
            settled.set_reduced_motion(true);
            let end = picture(&settled.layout(window));

            let start = Instant::now();
            let mut tree = UiTree::new(lightbox(window, (1200.0, 800.0), Some(origin)));
            let first = picture(&tree.layout_at(window, start));
            assert!(close_enough(first, origin), "{first:?}");
            let done = picture(&tree.layout_at(window, start + Duration::from_millis(350)));
            assert!(close_enough(done, end), "{done:?} != {end:?}");
        }
    }
}
