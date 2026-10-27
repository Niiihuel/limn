//! Controlled image gallery. The host owns the images and navigation state.

use crate::{
    Access, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily,
    Icon, IconButton, Point, PopoverPlacement, RasterImage, Rect, Role, Size, Spinner, Tooltip,
};

/// The corner radius a thumbnail's picture starts with, and the one the full picture
/// rests at.
const ORIGIN_RADIUS: f32 = 8.0;
const PICTURE_RADIUS: f32 = 6.0;
/// How long the picture takes from its origin, and back, in milliseconds.
const MORPH_OPEN_MS: u16 = 350;
const MORPH_CLOSE_MS: u16 = 250;
/// How long a viewer without an origin takes to fade in and out, in milliseconds.
const FADE_MS: u16 = 180;
/// The gap between the action column and the picture's corner, in logical pixels.
const ACTION_GAP: f32 = 6.0;
/// A lightbox's picture is at most this wide, and its window's height less this tall.
const LIGHTBOX_MAX_WIDTH: f32 = 640.0;
const LIGHTBOX_HEIGHT_MARGIN: f32 = 64.0;
/// The caption and count lines under a lightbox's picture, and the gap above them.
const CAPTION_LINE: f32 = 20.0;
const CAPTION_GAP: f32 = 8.0;

/// A viewer over the whole window for one picture of a gallery: a title, a close control,
/// the picture or a loading or error message, and the picture's place in the gallery.
pub struct ImageViewer {
    base: Element,
    open: bool,
    title: String,
    close_id: String,
    palette: ComponentPalette,
    viewport: Size,
    image: Option<RasterImage>,
    error: Option<String>,
    index: usize,
    total: usize,
    previous_action: Option<String>,
    next_action: Option<String>,
    return_focus: Option<String>,
    zoom: f32,
    zoom_actions: Option<[String; 3]>,
    origin: Option<Rect>,
    actions: Vec<(String, Icon, String)>,
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
            close_id: close_id.into(),
            palette,
            viewport: Size::new(900.0, 620.0),
            image: None,
            error: None,
            index: 0,
            total: 1,
            previous_action: None,
            next_action: None,
            return_focus: None,
            zoom: 1.0,
            zoom_actions: None,
            origin: None,
            actions: Vec::new(),
            lightbox: false,
            natural: None,
            caption: None,
        }
    }

    /// The window's size in logical pixels; the viewer leaves 8 on every side (900 by 620
    /// by default). A size with a side that is not finite is ignored and a negative side is
    /// zero.
    pub fn viewport(mut self, size: Size) -> Self {
        if size.width.is_finite() && size.height.is_finite() {
            self.viewport = Size::new(size.width.max(0.0), size.height.max(0.0));
        }
        self
    }

    /// The picture to show. It is drawn at full resolution.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    /// A message shown in place of the spinner and the loading label while there is no
    /// image. It is not shown once `image` is set.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// The place of the picture: `index` counts from zero and is clamped to the last of
    /// `total`, which is at least 1. The footer shows both, and the arrows appear only when
    /// `total` is above 1.
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

    /// The control that takes focus when the viewer closes.
    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.return_focus = Some(id.into());
        self
    }

    /// How many times larger than its fit to the window the picture is
    /// shown, from 1 to [`ImageViewer::MAX_ZOOM`]. The host owns the level.
    /// Past 1 the picture is panned: the wheel moves it down, a drag or a
    /// sideways gesture moves it across.
    pub fn zoom(mut self, zoom: f32) -> Self {
        if zoom.is_finite() {
            self.zoom = zoom.clamp(1.0, Self::MAX_ZOOM);
        }
        self
    }

    /// Controls in the header to zoom out, fit the window and zoom in. The
    /// host answers them by passing another [`ImageViewer::zoom`].
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
    /// closing shrinks it back over 250 ms. Without an origin the viewer fades in
    /// over 180 ms. Under reduced motion both are at once.
    pub fn origin(mut self, rect: Rect) -> Self {
        if [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite()) {
            self.origin = Some(rect);
        }
        self
    }

    /// Add a round icon button, `label` as its name and tooltip, that reports `id`. The
    /// buttons stand in a column under the close button, just outside the picture's top-right
    /// corner, or on the picture's own corner when there is no room beside it.
    pub fn action(mut self, id: impl Into<String>, icon: Icon, label: impl Into<String>) -> Self {
        self.actions.push((id.into(), icon, label.into()));
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

    /// The largest level [`ImageViewer::zoom`] accepts.
    pub const MAX_ZOOM: f32 = 8.0;
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
                body.push(arrow(id, Icon::ArrowLeft, labels.previous_image, "←", self.index > 0, controls));
            }
        }
        body.push(picture);
        if self.total > 1 {
            if let Some(id) = &self.next_action {
                body.push(arrow(id, Icon::ArrowRight, labels.next_image, "→", self.index + 1 < self.total, controls));
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
fn arrow(id: &str, icon: Icon, label: &str, key: &str, enabled: bool, palette: ComponentPalette) -> Element {
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

impl Component for ImageViewer {
    fn render(mut self) -> Element {
        if !self.open {
            return self.base;
        }
        if self.lightbox {
            let viewer = self.lightbox_viewer();
            return self.float(viewer, Color::rgba(0, 0, 0, 159));
        }
        let width = (self.viewport.width - 16.0).max(0.0);
        let height = (self.viewport.height - 16.0).max(0.0);
        let mut controls = self.palette;
        controls.surface = self.palette.raised;
        let mut header = vec![Element::truncated_text(&self.title)
            .font_family(FontFamily::Sans)
            .font_size(14.0)
            .foreground(self.palette.text)
            .flex(1.0)
            .size(None, Some(36.0))];
        let zoomed = self.zoom > 1.0;
        if let Some([out, fit, zoom_in]) = &self.zoom_actions {
            let labels = crate::labels();
            header.push(control(out, Icon::Minus, labels.zoom_out, labels.zoom_out, zoomed, controls));
            header.push(control(fit, Icon::Collapse, labels.fit_to_window, labels.fit_to_window, zoomed, controls));
            header.push(control(zoom_in, Icon::Add, labels.zoom_in, labels.zoom_in, self.zoom < Self::MAX_ZOOM, controls));
        }
        // With actions the close button leads their column beside the picture instead.
        let close = || close_control(&self.close_id, controls);
        if self.actions.is_empty() {
            header.push(close());
        }
        let header = Element::row(header).gap(12.0).size(None, Some(36.0));
        // What is left for the picture inside the panel's padding, between
        // the header and the count, and between the gallery's arrows.
        let arrows = if self.total > 1 { 2.0 * (36.0 + 12.0) } else { 0.0 };
        let room = Size::new(
            (width - 32.0 - arrows).max(0.0),
            (height - 32.0 - 36.0 - 24.0 - 24.0).max(0.0),
        );
        let image = if let Some(image) = self.image.take() {
            let picture = Element::raster_image(image.full_resolution()).radius(PICTURE_RADIUS);
            if zoomed {
                // The picture fits a box this many times its room, and the
                // box is panned both ways, as a diagram is.
                Element::scroll(
                    format!("{}:down", self.close_id),
                    crate::diagram::pannable(
                        picture.size(Some(room.width * self.zoom), Some(room.height * self.zoom)),
                        format!("{}:across", self.close_id),
                        self.palette.surface,
                        self.palette.muted,
                    ),
                )
            } else {
                picture
            }
        } else {
            pending(&self.close_id, self.error.as_deref(), width, self.palette)
        }
        .flex(1.0);
        let image = if self.actions.is_empty() {
            image
        } else {
            let mut column = vec![close()];
            for (id, icon, label) in &self.actions {
                column.push(control(id, *icon, label, label, true, controls));
            }
            // Non-modal: the picture keeps its own focus scope, and the column takes no backdrop.
            Element::popover(
                image,
                Element::column(column).gap(ACTION_GAP).size(Some(36.0), None),
                format!("{}:actions", self.close_id),
                PopoverPlacement::RightCorner,
            )
            .popover_gap(ACTION_GAP)
            .popover_non_modal()
            .flex(1.0)
        };
        let mut body = Vec::new();
        if self.total > 1 {
            if let Some(id) = &self.previous_action {
                body.push(arrow(id, Icon::ArrowLeft, crate::labels().previous_image, "←", self.index > 0, controls));
            }
        }
        body.push(image);
        if self.total > 1 {
            if let Some(id) = &self.next_action {
                body.push(arrow(id, Icon::ArrowRight, crate::labels().next_image, "→", self.index + 1 < self.total, controls));
            }
        }
        let footer = Element::truncated_text(format!("{} {} {}", self.index + 1, crate::labels().of, self.total))
            .font_family(FontFamily::Sans)
            .font_size(12.0)
            .foreground(self.palette.muted)
            .center_content()
            .size(None, Some(24.0));
        let viewer = Element::panel(
            Element::column(vec![header, Element::row(body).gap(12.0).flex(1.0), footer]).gap(12.0),
        )
        .padding(16.0)
        .background(self.palette.surface)
        .radius(crate::tokens().radius.panel)
        .clip_children()
        .size(Some(width), Some(height))
        // Known by an ID of its own: `close_id` is its close control's.
        .accessible(Access::new(Role::Dialog).id(format!("{}:dialog", self.close_id)).name(&self.title));
        self.float(viewer, Color::rgba(0, 0, 0, 230))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, RasterImageFormat, Rect, UiAction, UiEvent, UiTree};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    const WINDOW: Size = Size::new(900.0, 620.0);

    fn viewer(zoom: f32) -> Element {
        let picture = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
        ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
            .viewport(WINDOW)
            .image(picture)
            .zoom(zoom)
            .zoom_actions("out", "fit", "in")
            .render()
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

    fn click(tree: &mut UiTree, frame: &Frame, id: &str) -> Vec<UiAction> {
        let Some(region) = frame.regions.iter().find(|region| region.id == id) else {
            return Vec::new();
        };
        let center = Point::new(region.rect.x + region.rect.width / 2.0, region.rect.y + region.rect.height / 2.0);
        tree.handle_event(UiEvent::PointerMove(center));
        tree.handle_event(UiEvent::PointerDown(center));
        tree.handle_event(UiEvent::PointerUp(center))
    }

    #[test]
    fn the_viewer_is_a_dialog_named_by_its_title_and_its_icons_are_named_enabled_or_not() {
        let mut tree = UiTree::new(viewer(1.0));
        tree.set_reduced_motion(true);
        let frame = tree.layout(WINDOW);
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
        ] {
            let said = node(id);
            assert_eq!((said.role, said.name.as_str()), (Role::Button, name));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled), "{id}");
        }
    }

    #[test]
    fn the_arrow_keys_do_what_the_hints_say_and_stop_at_the_ends() {
        let gallery = |index: usize| {
            let picture = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
            ImageViewer::new(Element::spacer(), true, "picture.png", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .image(picture)
                .gallery(index, 3)
                .previous_action("before")
                .next_action("after")
                .render()
        };
        let press = |index: usize, key: crate::Key| {
            let mut tree = UiTree::new(gallery(index));
            tree.set_reduced_motion(true);
            tree.layout(WINDOW);
            tree.handle_event(crate::UiEvent::Key { key, shift: false, ctrl: false })
        };
        let clicked = |id: &str| vec![UiAction::Clicked { id: id.into() }];
        assert_eq!(press(1, crate::Key::ArrowLeft), clicked("before"));
        assert_eq!(press(1, crate::Key::ArrowRight), clicked("after"));
        // Nothing before the first, nothing after the last.
        assert!(press(0, crate::Key::ArrowLeft).is_empty());
        assert!(press(2, crate::Key::ArrowRight).is_empty());
    }

    #[test]
    fn a_fitted_picture_offers_only_to_zoom_in() {
        let mut tree = UiTree::new(viewer(1.0));
        tree.set_reduced_motion(true);
        let frame = tree.layout(WINDOW);
        let fitted = picture(&frame);
        assert!(fitted.width <= WINDOW.width && fitted.height <= WINDOW.height);
        // Nothing to zoom out of or to fit again.
        assert!(click(&mut tree, &frame, "out").is_empty());
        assert!(click(&mut tree, &frame, "fit").is_empty());
        assert_eq!(click(&mut tree, &frame, "in"), vec![UiAction::Clicked { id: "in".into() }]);
    }

    #[test]
    fn a_zoomed_picture_is_larger_than_its_room_and_can_be_brought_back() {
        let mut tree = UiTree::new(viewer(1.0));
        tree.set_reduced_motion(true);
        let fitted = picture(&tree.layout(WINDOW));
        tree.set_root(viewer(2.0));
        let frame = tree.layout(WINDOW);
        let zoomed = picture(&frame);
        // Twice its room both ways, so it no longer fits and is panned.
        assert_eq!((zoomed.width, zoomed.height), (fitted.width * 2.0, fitted.height * 2.0));
        assert_eq!(click(&mut tree, &frame, "out"), vec![UiAction::Clicked { id: "out".into() }]);
        assert_eq!(click(&mut tree, &frame, "fit"), vec![UiAction::Clicked { id: "fit".into() }]);
        // A level outside the range is brought into it.
        tree.set_root(viewer(50.0));
        let most = picture(&tree.layout(WINDOW));
        assert_eq!(most.width, fitted.width * ImageViewer::MAX_ZOOM);
        let frame = tree.layout(WINDOW);
        assert!(click(&mut tree, &frame, "in").is_empty());
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
