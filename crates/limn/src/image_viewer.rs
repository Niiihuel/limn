//! Controlled image gallery. The host owns the images and navigation state;
//! the tree keeps the zoom.

use crate::{
    Access, Button, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon,
    IconButton, Point, PopoverPlacement, RasterImage, Role, Size, Spinner, Tooltip, ZoomControl,
};

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

    /// One more control in the header, before the zoom controls: `icon`,
    /// named and hinted `name`, reporting `id` when activated, such as saving
    /// the picture. Each call adds one; they keep the order of the calls. An
    /// empty `id` adds nothing.
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

impl Component for ImageViewer {
    fn render(self) -> Element {
        if !self.open {
            return self.base;
        }
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
        let mut header = vec![Element::column(titles).gap(2.0).flex(1.0).centered_y()];
        let mut tools = Vec::new();
        for (id, icon, name) in &self.actions {
            tools.push(control(id, *icon, name, name, true, controls));
        }
        let picture_shown = self.surface.is_some() || self.image.is_some() || self.placeholder.is_some();
        let mut zoom_controls = Vec::new();
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
        if !tools.is_empty() {
            tools.push(Element::spacer().size(Some(8.0), Some(36.0)));
        }
        tools.push(control(
            &self.close_id,
            Icon::Close,
            labels.close_image,
            &format!("{} · Esc", labels.close_image),
            true,
            controls,
        ));
        header.push(
            Element::panel(Element::row(tools).gap(2.0))
                .padding(4.0)
                .background(chrome.surface)
                .radius(crate::tokens().radius.panel * 1.5 + 4.0)
                .centered_y(),
        );
        let header = Element::row(header).gap(16.0).size(None, Some(HEADER));
        let picture = match (&self.surface, &self.image, &self.placeholder) {
            (Some((id, label)), _, _) => Some(Element::surface(id, label)),
            (None, Some(image), _) => Some(Element::raster_image(image.clone().full_resolution().checkerboard())),
            (None, None, Some(stand_in)) => Some(Element::raster_image(stand_in.clone())),
            (None, None, None) => None,
        };
        let waiting = self.surface.is_none() && self.image.is_none();
        let body = if let Some(picture) = picture {
            let mut view = Element::zoom_view(&view_id, picture)
                .zoom_dismiss(&self.close_id)
                .shadow(chrome.shadow, Point::new(0.0, 12.0), 40.0, 0.0);
            if let Some(size) = self.picture_size {
                view = view.zoom_content(size);
            }
            if let Some(id) = &self.return_focus {
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
        let mut over = Vec::new();
        if self.total > 1 {
            if let Some(id) = &self.previous_action {
                over.push(arrow(id, Icon::ArrowLeft, &previous_label, "←", self.index > 0));
            }
            over.push(Element::spacer().flex(1.0));
            if let Some(id) = &self.next_action {
                over.push(arrow(id, Icon::ArrowRight, &next_label, "→", self.index + 1 < self.total));
            }
        }
        let body = if over.is_empty() { body } else { Element::stack(vec![body, Element::row(over)]) };
        let mut rows = vec![header, body.flex(1.0)];
        if let Some(footer) = self.footer {
            rows.push(footer);
        }
        if self.thumbnails.len() > 1 {
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
        let viewer = Element::column(rows)
            .gap(12.0)
            .padding(12.0)
            // A popover keeps 8 from the window's edges.
            .size(Some((self.viewport.width - 16.0).max(0.0)), Some((self.viewport.height - 16.0).max(0.0)))
            // Known by an ID of its own: `close_id` is its close control's.
            .accessible(Access::new(Role::Dialog).id(format!("{}:dialog", self.close_id)).name(&self.title));
        let initial = self.initial_focus.clone().unwrap_or_else(|| self.close_id.clone());
        let mut popover = Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
            // The window's content stays faintly there behind the picture.
            .popover_scrim(Color::rgba(0, 0, 0, 232))
            .popover_animation_ms(crate::tokens().motion.enter_ms)
            // The picture grows out of its thumbnail; the rest fades in.
            .popover_open_scale(1.0)
            .popover_backdrop_dismiss(true)
            .popover_initial_focus(initial)
            // The arrows its hints show: back while there is one before, on while there is one after.
            .popover_arrows(
                self.previous_action.filter(|_| self.index > 0),
                self.next_action.filter(|_| self.index + 1 < self.total),
            );
        if let Some(id) = self.return_focus {
            popover = popover.popover_return_focus(id);
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
}
