//! File and image cards. The host owns payloads, metadata and removal actions.

use crate::{
    Access, ButtonVariant, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon, IconButton,
    RasterImage, Role, Tooltip,
};

/// How an [`Attachment`] card lays out its media and its caption.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AttachmentOrientation {
    /// The media beside the caption, in a card `Attachment::FILE_HEIGHT` tall.
    #[default]
    Horizontal,
    /// The media above the caption, in a card `Attachment::IMAGE_HEIGHT` tall.
    Vertical,
}

/// A card for one attached file or image: an icon or a preview, the title and a description
/// line under it. The card reports only the open and remove actions it is given.
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
    /// The height of a horizontal card, in logical pixels.
    pub const FILE_HEIGHT: f32 = 64.0;
    /// The height of a vertical card with its caption, in logical pixels.
    pub const IMAGE_HEIGHT: f32 = 140.0;
    /// A side for a square `preview_only` card, in logical pixels. The library does not
    /// apply it; pass it to `width`.
    pub const PREVIEW_SIZE: f32 = 96.0;

    /// A horizontal card 220 wide with the file icon, an empty description and no actions.
    /// `id` prefixes the ID of the card's tooltip, `{id}:name`.
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

    /// The muted line under the title, such as a size or a type. It is also the tooltip's
    /// description unless `help` is set.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Text for the tooltip and for the card's description to the platform, in place of the
    /// description line.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// The icon drawn while there is no preview; the file icon by default.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// Draw this image in the media slot instead of the icon.
    pub fn preview(mut self, image: RasterImage) -> Self {
        self.preview = Some(image);
        self
    }

    /// Add a remove button that reports `id`. Its name is `Labels::remove` followed by the
    /// title.
    pub fn remove_action(mut self, id: impl Into<String>) -> Self {
        self.remove_action = Some(id.into());
        self
    }

    /// Make the whole card a button that reports `id`, named by the title.
    pub fn open_action(mut self, id: impl Into<String>) -> Self {
        self.open_action = Some(id.into());
        self
    }

    /// Choose the layout. This also brings back the caption that `preview_only` hides.
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

    /// Set the card's width in logical pixels, never below 80 (220 by default). A
    /// `preview_only` card is as tall as it is wide. A value that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(80.0);
        }
        self
    }
}

impl Component for Attachment {
    fn render(self) -> Element {
        // Previews keep their concentric corners in proportion to a panel.
        let panel = crate::tokens().radius.panel;
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
        .radius(if preview_only { panel * 14.0 / 12.0 } else { crate::tokens().radius.surface })
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
                .radius(panel)
                .size(Some(24.0), Some(24.0))
                // An icon alone: its hint is its name.
                .accessibility_label(format!("{} {}", crate::labels().remove, self.title));
            Tooltip::new(button, id, format!("{} {}", crate::labels().remove, self.title), self.palette)
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
            .radius(if preview_only { panel * 16.0 / 12.0 } else { panel * 10.0 / 12.0 })
            .size(Some(self.width), Some(height));
        let card = if let Some(id) = self.open_action {
            // Named by its title whatever the card shows: a preview alone has no text.
            let mut said = Access::new(Role::Button).name(&self.title);
            let about = self.help.as_deref().unwrap_or(&self.description);
            if !about.is_empty() {
                said = said.description(about);
            }
            Tooltip::new(Element::clickable(&id, card).accessible(said), id, &self.title, self.palette)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, UiTree};

    #[test]
    fn the_remove_button_is_named_for_what_it_removes_and_the_card_by_its_title() {
        let palette = ComponentPalette::neutral_dark();
        for attachment in [
            Attachment::new("file", "notas.md", palette).description("2 KB"),
            Attachment::new("file", "notas.md", palette).description("2 KB").preview_only(),
        ] {
            let element = attachment.open_action("open").remove_action("remove").render();
            let frame = UiTree::new(Element::row(vec![element])).layout(Size::new(400.0, 300.0));
            let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
            let remove = node("remove");
            assert_eq!((remove.role, remove.name.clone()), (Role::Button, format!("{} notas.md", crate::labels().remove)));
            assert_eq!((node("open").role, node("open").name.as_str()), (Role::Button, "notas.md"));
            assert!(remove.actionable && node("open").actionable);
        }
    }
}
