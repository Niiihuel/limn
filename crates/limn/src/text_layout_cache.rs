//! The text layouts a tree made, kept while it uses them: those of this
//! layout pass and of the one before. Text that stays costs no shaping, and
//! text that only changes width is broken into lines again without it.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::{InlineImage, TextLayout, TextSpan, TextStyle};

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct Key {
    text: u64,
    size: u32,
    family: u8,
    line_height: Option<u32>,
    leading: crate::Leading,
    width: Option<u32>,
    centered: bool,
}

impl Key {
    fn new(text: &str, style: TextStyle, width: Option<f32>, centered: bool) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Self {
            text: hasher.finish(),
            size: style.size.to_bits(),
            family: style.family as u8,
            line_height: style.line_height.map(f32::to_bits),
            leading: style.leading,
            width: width.map(f32::to_bits),
            centered,
        }
    }
}

/// A formatted text's key: its spans, its paragraph, the width it is broken
/// at and its limit of lines.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct FormattedKey {
    spans: u64,
    paragraph: Key,
    max_lines: Option<usize>,
}

/// A formatted layout and the spans it was made from, to tell spans with one
/// hash apart.
struct Formatted {
    spans: Vec<TextSpan>,
    layout: TextLayout,
}

/// Everything about spans that changes how they are laid out or drawn.
fn hash_spans(spans: &[TextSpan]) -> u64 {
    let mut hasher = DefaultHasher::new();
    spans.len().hash(&mut hasher);
    for span in spans {
        span.text.hash(&mut hasher);
        span.font_size.to_bits().hash(&mut hasher);
        (span.font_family as u8).hash(&mut hasher);
        [span.color.r, span.color.g, span.color.b, span.color.a].hash(&mut hasher);
        span.background.map(|color| [color.r, color.g, color.b, color.a]).hash(&mut hasher);
        span.icon.hash(&mut hasher);
        (span.underline, span.strikethrough).hash(&mut hasher);
        match &span.image {
            None => 0_u8.hash(&mut hasher),
            Some(InlineImage::Raster(image)) => {
                1_u8.hash(&mut hasher);
                image.sha256().hash(&mut hasher);
            }
            Some(InlineImage::Surface(id)) => {
                2_u8.hash(&mut hasher);
                id.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

#[derive(Default)]
pub(crate) struct TextLayoutCache {
    current: HashMap<Key, TextLayout>,
    previous: HashMap<Key, TextLayout>,
    formatted: HashMap<FormattedKey, Formatted>,
    formatted_before: HashMap<FormattedKey, Formatted>,
    /// The fonts the layouts were made with.
    fonts: u64,
}

impl TextLayoutCache {
    /// Begin a layout pass: what the pass before did not use is dropped, and
    /// everything is, once the fonts have changed.
    pub(crate) fn begin_pass(&mut self) {
        let fonts = crate::fonts::generation();
        if fonts != self.fonts {
            self.current.clear();
            self.formatted.clear();
            self.fonts = fonts;
        }
        self.previous = std::mem::take(&mut self.current);
        self.formatted_before = std::mem::take(&mut self.formatted);
    }

    /// Formatted `spans` set over `paragraph`, broken into lines no wider
    /// than `width` when it is finite, and cut to `max_lines` when given.
    pub(crate) fn formatted(
        &mut self,
        spans: &[TextSpan],
        paragraph: TextStyle,
        width: f32,
        max_lines: Option<usize>,
    ) -> TextLayout {
        let finite = width.is_finite().then(|| width.max(0.0));
        let key = FormattedKey { spans: hash_spans(spans), paragraph: Key::new("", paragraph, finite, false), max_lines };
        if let Some(found) = self.formatted.get(&key).filter(|found| found.spans == spans) {
            return found.layout.clone();
        }
        if let Some(found) = self.formatted_before.remove(&key).filter(|found| found.spans == spans) {
            let layout = found.layout.clone();
            self.formatted.insert(key, found);
            return layout;
        }
        let layout = match (finite, max_lines) {
            (None, None) => TextLayout::formatted(spans, paragraph),
            (_, None) => self.formatted(spans, paragraph, f32::INFINITY, None).wrapped(width),
            (_, Some(lines)) => self.formatted(spans, paragraph, width, None).max_lines(lines),
        };
        self.formatted.insert(key, Formatted { spans: spans.to_vec(), layout: layout.clone() });
        layout
    }

    /// `text` set in `style`, broken into lines no wider than `width` when it
    /// is finite, and each centered across it when asked.
    pub(crate) fn get(&mut self, text: &str, style: TextStyle, width: f32, centered: bool) -> TextLayout {
        let width = width.is_finite().then(|| width.max(0.0));
        let key = Key::new(text, style, width, centered);
        if let Some(layout) = self.find(key, text) {
            return layout;
        }
        let layout = if width.is_none() && !centered {
            TextLayout::new(text, style)
        } else {
            // Shaped once for every width it is broken at.
            let shaped = self.get(text, style, f32::INFINITY, false);
            let width = width.unwrap_or(f32::INFINITY);
            if centered { shaped.centered(width) } else { shaped.wrapped(width) }
        };
        self.current.insert(key, layout.clone());
        layout
    }

    fn find(&mut self, key: Key, text: &str) -> Option<TextLayout> {
        if let Some(layout) = self.current.get(&key).filter(|layout| layout.text() == text) {
            return Some(layout.clone());
        }
        let layout = self.previous.remove(&key).filter(|layout| layout.text() == text)?;
        self.current.insert(key, layout.clone());
        Some(layout)
    }
}

#[cfg(all(test, feature = "bundled-fonts"))]
mod tests {
    use super::*;
    use crate::{DrawCommand, FontFamily, Point};

    fn glyphs(layout: &TextLayout) -> std::sync::Arc<[crate::Glyph]> {
        match &layout.draw(Point::new(0.0, 0.0), crate::Color::WHITE)[0] {
            DrawCommand::Glyphs(run) => run.glyphs.clone(),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn text_used_in_a_pass_is_kept_for_the_next_and_dropped_after_one_without_it() {
        let style = TextStyle::new(14.0, FontFamily::Sans);
        let mut cache = TextLayoutCache::default();
        cache.begin_pass();
        let first = glyphs(&cache.get("limn", style, 200.0, false));
        cache.begin_pass();
        assert!(std::sync::Arc::ptr_eq(&first, &glyphs(&cache.get("limn", style, 200.0, false))));
        // Another width breaks the same shaping again; centered is another layout.
        assert_eq!(cache.get("limn", style, 10.0, false).text(), "limn");
        assert!(cache.get("limn", style, 200.0, true).lines().next().unwrap().rect.x > 50.0);
        cache.begin_pass();
        cache.begin_pass();
        assert!(cache.current.is_empty() && cache.previous.is_empty());
        assert!(!std::sync::Arc::ptr_eq(&first, &glyphs(&cache.get("limn", style, 200.0, false))));
    }
}
