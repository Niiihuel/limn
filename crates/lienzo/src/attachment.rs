//! File and image cards. The host owns payloads, metadata and removal actions.

use crate::{
    ButtonVariant, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon, IconButton,
    RasterImage, Tooltip,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AttachmentOrientation {
    #[default]
    Horizontal,
    Vertical,
}

pub struct Attachment {
    id: String,
    title: String,
    description: String,
    help: Option<String>,
    icon: Icon,
    preview: Option<RasterImage>,
    remove_action: Option<String>,
    open_action: Option<String>,
    orientation: AttachmentOrientation,
    show_caption: bool,
    width: f32,
    palette: ComponentPalette,
}

impl Attachment {
    pub const FILE_HEIGHT: f32 = 64.0;
    pub const IMAGE_HEIGHT: f32 = 140.0;
    pub const PREVIEW_SIZE: f32 = 96.0;

    pub fn new(id: impl Into<String>, title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            help: None,
            icon: Icon::File,
            preview: None,
            remove_action: None,
            open_action: None,
            orientation: AttachmentOrientation::Horizontal,
            show_caption: true,
            width: 220.0,
            palette,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    pub fn preview(mut self, image: RasterImage) -> Self {
        self.preview = Some(image);
        self
    }

    pub fn remove_action(mut self, id: impl Into<String>) -> Self {
        self.remove_action = Some(id.into());
        self
    }

    pub fn open_action(mut self, id: impl Into<String>) -> Self {
        self.open_action = Some(id.into());
        self
    }

    pub fn orientation(mut self, orientation: AttachmentOrientation) -> Self {
        self.orientation = orientation;
        self.show_caption = true;
        self
    }

    /// A square image preview with its name and metadata in the hover tooltip.
    pub fn preview_only(mut self) -> Self {
        self.orientation = AttachmentOrientation::Vertical;
        self.show_caption = false;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(80.0);
        }
        self
    }
}

impl Component for Attachment {
    fn render(self) -> Element {
        let vertical = self.orientation == AttachmentOrientation::Vertical;
        let preview_only = vertical && !self.show_caption;
        let padding = if preview_only { 2.0 } else { 8.0 };
        let inner_width = (self.width - 2.0 * padding).max(0.0);
        let gap = if inner_width < 144.0 { 4.0 } else { 8.0 };
        let file_media_width = if inner_width < 144.0 { 32.0 } else { 40.0 };
        let caption_width = if vertical {
            inner_width
        } else {
            (inner_width
                - file_media_width
                - gap
                - self.remove_action.as_ref().map_or(0.0, |_| 24.0 + gap))
            .max(0.0)
        };
        let caption = Element::column(vec![
            Element::truncated_text(&self.title)
                .font_family(FontFamily::Sans)
                .font_size(12.5)
                .foreground(self.palette.text)
                .size(Some(caption_width), Some(18.0)),
            Element::truncated_text(&self.description)
                .font_family(FontFamily::Sans)
                .font_size(11.0)
                .foreground(self.palette.muted)
                .size(Some(caption_width), Some(16.0)),
        ])
        .size(Some(caption_width), Some(34.0));
        let caption = if self.open_action.is_none() {
            Tooltip::new(
                caption,
                format!("{}:name", self.id),
                &self.title,
                self.palette,
            )
            .description(self.help.as_deref().unwrap_or(&self.description))
            .render()
        } else {
            caption
        };
        let media_width = if vertical {
            inner_width
        } else {
            file_media_width
        };
        let media_height = if preview_only {
            inner_width
        } else if vertical {
            84.0
        } else {
            40.0
        };
        let media = if let Some(image) = self.preview {
            Element::raster_image(image)
        } else {
            Element::icon(self.icon).foreground(self.palette.muted)
        }
        .background(self.palette.raised)
        .radius(if preview_only { 14.0 } else { 6.0 })
        .size(Some(media_width), Some(media_height));
        let remove = self.remove_action.map(|id| {
            let button = IconButton::new(&id, Icon::Close, self.palette)
                .variant(if vertical {
                    ButtonVariant::Default
                } else {
                    ButtonVariant::Subtle
                })
                .size(ControlSize::Dense)
                .render()
                .radius(12.0)
                .size(Some(24.0), Some(24.0));
            Tooltip::new(button, id, format!("Quitar {}", self.title), self.palette)
                .render()
                .size(Some(24.0), Some(24.0))
        });
        let contents = if preview_only {
            if self.open_action.is_none() {
                Tooltip::new(
                    media,
                    format!("{}:name", self.id),
                    &self.title,
                    self.palette,
                )
                .description(self.help.as_deref().unwrap_or(&self.description))
                .render()
                .size(Some(inner_width), Some(media_height))
            } else {
                media
            }
        } else if vertical {
            Element::column(vec![media, caption]).gap(6.0)
        } else {
            let mut row = vec![media, caption];
            if let Some(remove) = remove.clone() {
                row.push(remove);
            }
            Element::row(row).gap(gap)
        };
        let height = if preview_only {
            self.width
        } else if vertical {
            Self::IMAGE_HEIGHT
        } else {
            Self::FILE_HEIGHT
        };
        let card = Element::panel(contents)
            .padding(padding)
            .background(self.palette.surface)
            .border(self.palette.border, 1.0)
            .radius(if preview_only { 16.0 } else { 10.0 })
            .size(Some(self.width), Some(height));
        let card = if let Some(id) = self.open_action {
            Tooltip::new(Element::clickable(&id, card), id, &self.title, self.palette)
                .description(self.help.as_deref().unwrap_or(&self.description))
                .render()
                .size(Some(self.width), Some(height))
        } else {
            card
        };
        if vertical {
            if let Some(remove) = remove {
                return Element::overlay(
                    card,
                    Element::column(vec![
                        Element::row(vec![Element::spacer().flex(1.0), remove])
                            .padding(6.0)
                            .size(Some(self.width), Some(36.0)),
                        Element::spacer().flex(1.0),
                    ]),
                )
                .clip_children()
                .size(Some(self.width), Some(height));
            }
        }
        card
    }
}
