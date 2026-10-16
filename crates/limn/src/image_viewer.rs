//! Controlled image gallery. The host owns the images and navigation state.

use crate::{
    Access, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily,
    Icon, IconButton, PopoverPlacement, RasterImage, Role, Size, Spinner, SurfaceFit, Tooltip,
};

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
    surface: Option<(String, String)>,
    error: Option<String>,
    index: usize,
    total: usize,
    previous_action: Option<String>,
    next_action: Option<String>,
    return_focus: Option<String>,
    zoom: f32,
    zoom_actions: Option<[String; 3]>,
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
            surface: None,
            error: None,
            index: 0,
            total: 1,
            previous_action: None,
            next_action: None,
            return_focus: None,
            zoom: 1.0,
            zoom_actions: None,
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

    /// The largest level [`ImageViewer::zoom`] accepts.
    pub const MAX_ZOOM: f32 = 8.0;
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
    fn render(self) -> Element {
        if !self.open {
            return self.base;
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
        header.push(control(
            &self.close_id,
            Icon::Close,
            crate::labels().close_image,
            &format!("{} · Esc", crate::labels().close_image),
            true,
            controls,
        ));
        let header = Element::row(header).gap(12.0).size(None, Some(36.0));
        // What is left for the picture inside the panel's padding, between
        // the header and the count, and between the gallery's arrows.
        let arrows = if self.total > 1 { 2.0 * (36.0 + 12.0) } else { 0.0 };
        let room = Size::new(
            (width - 32.0 - arrows).max(0.0),
            (height - 32.0 - 36.0 - 24.0 - 24.0).max(0.0),
        );
        let picture = match (self.surface, self.image) {
            (Some((id, label)), _) => Some(Element::surface(id, label).surface_fit(SurfaceFit::Contain)),
            (None, Some(image)) => Some(Element::raster_image(image.full_resolution())),
            (None, None) => None,
        };
        let image = if let Some(picture) = picture {
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
            let message = self.error.as_deref().unwrap_or(crate::labels().loading_image);
            let mut content = vec![Element::spacer().flex(1.0)];
            if self.error.is_none() {
                content.push(centered(
                    Spinner::new(format!("{}:loading", self.close_id), self.palette.muted)
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
                    .foreground(self.palette.muted)
                    .size(Some((width - 144.0).clamp(80.0, 520.0)), None),
                64.0,
            ));
            content.push(Element::spacer().flex(1.0));
            Element::column(content).gap(12.0)
        }
        .flex(1.0);
        let mut body = Vec::new();
        if self.total > 1 {
            if let Some(id) = &self.previous_action {
                body.push(
                    Element::column(vec![
                        Element::spacer().flex(1.0),
                        control(
                            id,
                            Icon::ArrowLeft,
                            crate::labels().previous_image,
                            &format!("{} · ←", crate::labels().previous_image),
                            self.index > 0,
                            controls,
                        ),
                        Element::spacer().flex(1.0),
                    ])
                    .size(Some(36.0), None),
                );
            }
        }
        body.push(image);
        if self.total > 1 {
            if let Some(id) = &self.next_action {
                body.push(
                    Element::column(vec![
                        Element::spacer().flex(1.0),
                        control(
                            id,
                            Icon::ArrowRight,
                            crate::labels().next_image,
                            &format!("{} · →", crate::labels().next_image),
                            self.index + 1 < self.total,
                            controls,
                        ),
                        Element::spacer().flex(1.0),
                    ])
                    .size(Some(36.0), None),
                );
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
        let mut popover =
            Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
                .popover_scrim(Color::rgba(0, 0, 0, 230))
                .popover_animation_ms(crate::tokens().motion.enter_ms)
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
        popover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, RasterImageFormat, Rect, UiAction, UiEvent, UiTree};
    use std::sync::Arc;

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
            tree.handle_event(crate::UiEvent::Key { key, shift: false, ctrl: false, alt: false })
        };
        let clicked = |id: &str| vec![UiAction::Clicked { id: id.into() }];
        assert_eq!(press(1, crate::Key::ArrowLeft), clicked("before"));
        assert_eq!(press(1, crate::Key::ArrowRight), clicked("after"));
        // Nothing before the first, nothing after the last.
        assert!(press(0, crate::Key::ArrowLeft).is_empty());
        assert!(press(2, crate::Key::ArrowRight).is_empty());
    }

    #[test]
    fn a_surface_is_shown_fitted_and_zoomed_as_a_picture_is() {
        let shown = |zoom: f32, image: bool| {
            let mut viewer = ImageViewer::new(Element::spacer(), true, "loop.gif", "close", ComponentPalette::neutral_dark())
                .viewport(WINDOW)
                .zoom(zoom)
                .surface("animation:a", "A loop");
            if image {
                let still = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
                viewer = viewer.image(still);
            }
            let mut tree = UiTree::new(viewer.render());
            tree.set_reduced_motion(true);
            let frame = tree.layout(WINDOW);
            assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::RasterImage { .. })), "the surface takes the picture's place");
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Surface { rect, id, label, fit, .. } => Some((*rect, id.clone(), label.clone(), *fit)),
                    _ => None,
                })
                .expect("the surface is drawn")
        };
        let (fitted, id, label, fit) = shown(1.0, true);
        assert_eq!((id.as_str(), label.as_str(), fit), ("animation:a", "A loop", SurfaceFit::Contain));
        // The same room a picture gets, and twice it zoomed.
        let mut tree = UiTree::new(viewer(1.0));
        tree.set_reduced_motion(true);
        assert_eq!(fitted, picture(&tree.layout(WINDOW)));
        let zoomed = shown(2.0, false).0;
        assert_eq!((zoomed.width, zoomed.height), (fitted.width * 2.0, fitted.height * 2.0));
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
}
