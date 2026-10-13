use std::sync::Arc;

/// A bounded encoded image which the host paints after validating and decoding.
/// The UI tree keeps its journal payload; no machine-local path is needed.
#[derive(Clone, PartialEq, Eq)]
pub struct RasterImage {
    sha256: String,
    format: RasterImageFormat,
    size_bytes: u32,
    data_base64: Arc<str>,
    alt: String,
    full_resolution: bool,
    cover: bool,
}

impl std::fmt::Debug for RasterImage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RasterImage")
            .field("sha256", &self.sha256)
            .field("format", &self.format)
            .field("size_bytes", &self.size_bytes)
            .field("alt", &self.alt)
            .field("full_resolution", &self.full_resolution)
            .field("cover", &self.cover)
            .finish()
    }
}

/// The encoding of an image's bytes, as the host declares it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RasterImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
}

impl RasterImage {
    /// The host must recompute the digest and inspect the format before decoding.
    /// Construction alone does not attest to the bytes.
    pub fn new(
        sha256: impl Into<String>,
        format: RasterImageFormat,
        size_bytes: u32,
        data_base64: Arc<str>,
        alt: impl Into<String>,
    ) -> Option<Self> {
        let sha256 = sha256.into();
        let alt = alt.into();
        if size_bytes == 0
            || size_bytes as usize > 8 * 1024 * 1024
            || data_base64.is_empty()
            || data_base64.len() > (8 * 1024 * 1024_usize).div_ceil(3) * 4 + 4
            || sha256.len() != 64
            || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || alt.is_empty()
            || alt.len() > 256
        {
            return None;
        }
        Some(Self {
            sha256,
            format,
            size_bytes,
            data_base64,
            alt,
            full_resolution: false,
            cover: false,
        })
    }

    /// The SHA-256 digest the image was constructed with: 64 hexadecimal characters.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// The encoding the image was constructed with.
    pub fn format(&self) -> RasterImageFormat {
        self.format
    }

    /// The declared size of the image's bytes, from 1 to 8 MiB.
    pub fn size_bytes(&self) -> u32 {
        self.size_bytes
    }

    /// The image's bytes in base64.
    pub fn data_base64(&self) -> &str {
        &self.data_base64
    }

    /// The text alternative: never empty and at most 256 bytes.
    pub fn alt(&self) -> &str {
        &self.alt
    }

    /// Request the host's bounded full image decoder for a detail viewer.
    pub fn full_resolution(mut self) -> Self {
        self.full_resolution = true;
        self
    }

    /// Whether [`full_resolution`](Self::full_resolution) was requested.
    pub fn is_full_resolution(&self) -> bool {
        self.full_resolution
    }

    /// Fill the element's box, cropping what does not fit around the center,
    /// as artwork does in a square whatever shape it arrived in. Without it
    /// the whole picture is shown inside the box at its own proportions.
    pub fn cover(mut self) -> Self {
        self.cover = true;
        self
    }

    /// Whether [`cover`](Self::cover) was requested.
    pub fn is_cover(&self) -> bool {
        self.cover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Element, Size, UiTree};

    #[test]
    fn raster_image_layout_keeps_alt_and_bounded_payload() {
        let image = RasterImage::new(
            "a".repeat(64),
            RasterImageFormat::Png,
            3,
            Arc::from("cG5n"),
            "Imagen adjunta 1 de 1",
        )
        .unwrap();
        let frame = UiTree::new(Element::column(vec![Element::raster_image(image.clone())]))
            .layout(Size::new(180.0, 220.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::RasterImage { rect, image: painted, .. }
                if painted == &image && painted.alt() == "Imagen adjunta 1 de 1"
                    && rect.width <= 180.0 && rect.height <= 180.0
        )));
        assert!(RasterImage::new(
            "a".repeat(64),
            RasterImageFormat::Png,
            8 * 1024 * 1024 + 1,
            Arc::from("cG5n"),
            "Imagen",
        )
        .is_none());
    }
}
