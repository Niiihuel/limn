//! Controlled image gallery. The host owns the images and navigation state.

use crate::{
    ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon,
    IconButton, PopoverPlacement, RasterImage, Size, Spinner, Tooltip,
};

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
}

impl ImageViewer {
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
        }
    }

    pub fn viewport(mut self, size: Size) -> Self {
        if size.width.is_finite() && size.height.is_finite() {
            self.viewport = Size::new(size.width.max(0.0), size.height.max(0.0));
        }
        self
    }

    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn gallery(mut self, index: usize, total: usize) -> Self {
        self.total = total.max(1);
        self.index = index.min(self.total - 1);
        self
    }

    pub fn previous_action(mut self, id: impl Into<String>) -> Self {
        self.previous_action = Some(id.into());
        self
    }

    pub fn next_action(mut self, id: impl Into<String>) -> Self {
        self.next_action = Some(id.into());
        self
    }

    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.return_focus = Some(id.into());
        self
    }
}

fn control(id: &str, icon: Icon, title: &str, enabled: bool, palette: ComponentPalette) -> Element {
    let button = IconButton::new(id, icon, palette)
        .variant(ButtonVariant::Subtle)
        .size(ControlSize::Spacious)
        .enabled(enabled)
        .render()
        .radius(18.0)
        .size(Some(36.0), Some(36.0));
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
        let header = Element::row(vec![
            Element::truncated_text(&self.title)
                .font_family(FontFamily::Sans)
                .font_size(14.0)
                .foreground(self.palette.text)
                .flex(1.0)
                .size(None, Some(36.0)),
            control(
                &self.close_id,
                Icon::Close,
                "Cerrar imagen · Esc",
                true,
                controls,
            ),
        ])
        .gap(12.0)
        .size(None, Some(36.0));
        let image = if let Some(image) = self.image {
            Element::raster_image(image.full_resolution())
        } else {
            let message = self.error.as_deref().unwrap_or("Cargando imagen…");
            let mut content = vec![Element::spacer().flex(1.0)];
            if self.error.is_none() {
                content.push(centered(
                    Spinner::new(format!("{}:loading", self.close_id), self.palette.muted)
                        .size(32.0)
                        .render(),
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
                            "Imagen anterior · ←",
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
                            "Imagen siguiente · →",
                            self.index + 1 < self.total,
                            controls,
                        ),
                        Element::spacer().flex(1.0),
                    ])
                    .size(Some(36.0), None),
                );
            }
        }
        let footer = Element::truncated_text(format!("{} de {}", self.index + 1, self.total))
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
        .radius(12.0)
        .clip_children()
        .size(Some(width), Some(height));
        let mut popover =
            Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
                .popover_scrim(Color::rgba(0, 0, 0, 230))
                .popover_animation_ms(120)
                .popover_backdrop_dismiss(true)
                .popover_initial_focus(&self.close_id);
        if let Some(id) = self.return_focus {
            popover = popover.popover_return_focus(id);
        }
        popover
    }
}
