//! Account photos, with the initial of the name where there is no photo.

use crate::{Color, Component, ComponentPalette, Element, FontFamily, RasterImage};

/// A round account picture. Without an image it shows the first character of the trimmed
/// name, in upper case, in the palette's text on its selected fill, or on [`Avatar::color`];
/// a blank name shows "?".
pub struct Avatar {
    name: String,
    image: Option<RasterImage>,
    size: f32,
    color: Option<Color>,
    palette: ComponentPalette,
}

impl Avatar {
    /// An avatar 32 logical pixels across for `name`, with no image.
    pub fn new(name: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            name: name.into(),
            image: None,
            size: 32.0,
            color: None,
            palette,
        }
    }

    /// Provide a square photo, decoded or cropped by the host's image worker.
    pub fn image(mut self, image: RasterImage) -> Self {
        self.image = Some(image);
        self
    }

    /// The fill behind the initial, such as the color an application gives
    /// each person, in place of the palette's selected fill. A photo covers
    /// it.
    pub fn color(mut self, fill: Color) -> Self {
        self.color = Some(fill);
        self
    }

    /// Set the diameter in logical pixels, clamped to 16 to 256. A value that is not finite
    /// is ignored.
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
        .background(self.color.unwrap_or(self.palette.selected))
        .radius(self.size / 2.0)
    }
}
