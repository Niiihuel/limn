//! Account photos with an accessible, name-based fallback.

use crate::{Component, ComponentPalette, Element, FontFamily, RasterImage};

pub struct Avatar {
    name: String,
    image: Option<RasterImage>,
    size: f32,
    palette: ComponentPalette,
}

impl Avatar {
    pub fn new(name: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            name: name.into(),
            image: None,
            size: 32.0,
            palette,
        }
    }

    /// Provide a square photo, decoded or cropped by the host's image worker.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.size = size.clamp(16.0, 256.0);
        }
        self
    }
}

impl Component for Avatar {
    fn render(self) -> Element {
        if let Some(image) = self.image {
            return Element::raster_image(image)
                .size(Some(self.size), Some(self.size))
                .radius(self.size / 2.0);
        }
        let initial = self
            .name
            .trim()
            .chars()
            .next()
            .map(|character| character.to_uppercase().collect::<String>())
            .unwrap_or_else(|| "?".to_owned());
        Element::panel(Element::column(vec![
            Element::spacer().flex(1.0),
            Element::row(vec![
                Element::spacer().flex(1.0),
                Element::text(initial)
                    .font_family(FontFamily::Sans)
                    .font_size(self.size * 0.42)
                    .foreground(self.palette.text),
                Element::spacer().flex(1.0),
            ]),
            Element::spacer().flex(1.0),
        ]))
        .size(Some(self.size), Some(self.size))
        .background(self.palette.selected)
        .radius(self.size / 2.0)
    }
}
