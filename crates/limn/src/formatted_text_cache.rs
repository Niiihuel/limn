//! Exact, bounded paragraph layouts retained across measurement and paint.
//! A streamed append invalidates its paragraph, not every completed message.

use super::{FormattedLine, InlineImage, TextSpan};
use std::collections::{hash_map::DefaultHasher, HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

const MAX_ENTRIES: usize = 2048;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_ENTRY_BYTES: usize = 256 * 1024;

struct Entry {
    spans: Box<[TextSpan]>,
    width: u32,
    line_height: u32,
    max_lines: Option<usize>,
    lines: Arc<[FormattedLine]>,
    bytes: usize,
}

#[derive(Default)]
pub(super) struct FormattedTextCache {
    entries: HashMap<u64, Entry>,
    order: VecDeque<u64>,
    bytes: usize,
}

impl FormattedTextCache {
    fn key(spans: &[TextSpan], width: f32, line_height: f32, max_lines: Option<usize>) -> u64 {
        let mut hash = DefaultHasher::new();
        width.to_bits().hash(&mut hash);
        line_height.to_bits().hash(&mut hash);
        max_lines.hash(&mut hash);
        spans.len().hash(&mut hash);
        for span in spans {
            span.text.hash(&mut hash);
            span.font_size.to_bits().hash(&mut hash);
            (span.font_family as u8).hash(&mut hash);
            [span.color.r, span.color.g, span.color.b, span.color.a].hash(&mut hash);
            span.background
                .map(|color| [color.r, color.g, color.b, color.a])
                .hash(&mut hash);
            span.action.hash(&mut hash);
            span.icon.hash(&mut hash);
            (span.underline, span.strikethrough).hash(&mut hash);
            span.tooltip.as_ref().map(|hint| &hint.title).hash(&mut hash);
            match &span.image {
                None => 0_u8.hash(&mut hash),
                Some(InlineImage::Raster(image)) => {
                    1_u8.hash(&mut hash);
                    image.sha256().hash(&mut hash);
                }
                Some(InlineImage::Surface(id)) => {
                    2_u8.hash(&mut hash);
                    id.hash(&mut hash);
                }
            }
        }
        hash.finish()
    }

    pub(super) fn get(
        &self,
        spans: &[TextSpan],
        width: f32,
        line_height: f32,
        max_lines: Option<usize>,
    ) -> Option<Arc<[FormattedLine]>> {
        let entry = self
            .entries
            .get(&Self::key(spans, width, line_height, max_lines))?;
        // The hash is only an index. Exact source/style equality prevents a
        // collision from displaying content belonging to another paragraph.
        (entry.width == width.to_bits()
            && entry.line_height == line_height.to_bits()
            && entry.max_lines == max_lines
            && entry.spans.as_ref() == spans)
            .then(|| entry.lines.clone())
    }

    pub(super) fn insert(
        &mut self,
        spans: &[TextSpan],
        width: f32,
        line_height: f32,
        max_lines: Option<usize>,
        lines: Arc<[FormattedLine]>,
    ) {
        let bytes = std::mem::size_of::<Entry>()
            + std::mem::size_of_val(spans)
            + spans
                .iter()
                .map(|span| {
                    // A picture's bytes stay alive while its entry does.
                    let image = match &span.image {
                        Some(InlineImage::Raster(image)) => image.data_base64().len(),
                        Some(InlineImage::Surface(id)) => id.len(),
                        None => 0,
                    };
                    span.text.len() + span.action.as_ref().map_or(0, String::len) + image
                })
                .sum::<usize>()
            + std::mem::size_of_val(lines.as_ref())
            + lines
                .iter()
                .map(|line| {
                    line.fragments.capacity() * std::mem::size_of::<super::FormattedFragment>()
                        + line
                            .fragments
                            .iter()
                            .map(|fragment| {
                                fragment.text.capacity()
                                    + fragment.action.as_ref().map_or(0, String::capacity)
                            })
                            .sum::<usize>()
                })
                .sum::<usize>();
        if bytes > MAX_ENTRY_BYTES {
            return;
        }
        let key = Self::key(spans, width, line_height, max_lines);
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes -= previous.bytes;
            self.order.retain(|candidate| *candidate != key);
        }
        while self.order.len() >= MAX_ENTRIES || self.bytes + bytes > MAX_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(previous) = self.entries.remove(&oldest) {
                self.bytes -= previous.bytes;
            }
        }
        self.entries.insert(
            key,
            Entry {
                spans: spans.to_vec().into_boxed_slice(),
                width: width.to_bits(),
                line_height: line_height.to_bits(),
                max_lines,
                lines,
                bytes,
            },
        );
        self.order.push_back(key);
        self.bytes += bytes;
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, FontFamily};

    fn spans(text: &str) -> Vec<TextSpan> {
        vec![TextSpan::new(
            text,
            Color::rgb(100, 120, 140),
            14.0,
            FontFamily::Sans,
        )]
    }

    #[test]
    fn paragraph_cache_matches_exact_text_style_width_and_shares_the_layout() {
        let mut cache = FormattedTextCache::default();
        let source = spans("Árbol 👩🏽‍💻 completo");
        let lines: Arc<[FormattedLine]> = vec![FormattedLine::new(18.0)].into();
        cache.insert(&source, 300.0, 18.0, None, lines.clone());
        assert!(Arc::ptr_eq(
            &cache.get(&source, 300.0, 18.0, None).unwrap(),
            &lines
        ));
        assert!(cache.get(&source, 301.0, 18.0, None).is_none());
        assert!(cache.get(&source, 300.0, 20.0, None).is_none());
        assert!(cache.get(&source, 300.0, 18.0, Some(1)).is_none(), "a limit of lines is another layout");
        let mut changed = source.clone();
        changed[0].text.push('!');
        assert!(cache.get(&changed, 300.0, 18.0, None).is_none());
        changed = source.clone();
        changed[0].color = Color::rgb(10, 20, 30);
        assert!(cache.get(&changed, 300.0, 18.0, None).is_none());
        // Simulate a colliding index: equality must still reject the entry.
        cache
            .entries
            .get_mut(&FormattedTextCache::key(&source, 300.0, 18.0, None))
            .unwrap()
            .spans[0]
            .text
            .push('?');
        assert!(cache.get(&source, 300.0, 18.0, None).is_none());
    }

    #[test]
    fn paragraph_cache_tells_inline_pictures_apart() {
        let picture = |digest: char| {
            crate::RasterImage::new(
                digest.to_string().repeat(64),
                crate::RasterImageFormat::Png,
                4,
                Arc::from("AAAA"),
                "wave",
            )
            .unwrap()
        };
        let with = |span: TextSpan| vec![span];
        let base = TextSpan::new(":wave:", Color::WHITE, 14.0, FontFamily::Sans);
        let mut cache = FormattedTextCache::default();
        let still = with(base.clone().image(picture('a')));
        cache.insert(&still, 300.0, 18.0, None, vec![FormattedLine::new(18.0)].into());
        assert!(cache.get(&still, 300.0, 18.0, None).is_some());
        for other in [
            with(base.clone()),
            with(base.clone().image(picture('c'))),
            with(base.clone().surface("animation:a")),
        ] {
            assert_ne!(
                FormattedTextCache::key(&other, 300.0, 18.0, None),
                FormattedTextCache::key(&still, 300.0, 18.0, None)
            );
            assert!(cache.get(&other, 300.0, 18.0, None).is_none());
        }
    }

    #[test]
    fn paragraph_cache_bounds_retained_sources_and_layouts() {
        let mut cache = FormattedTextCache::default();
        for index in 0..MAX_ENTRIES * 2 {
            cache.insert(
                &spans(&format!("{index}: {}", "contenido ".repeat(200))),
                300.0,
                18.0,
                None,
                vec![FormattedLine::new(18.0)].into(),
            );
            assert!(cache.bytes <= MAX_BYTES);
            assert!(cache.entries.len() <= MAX_ENTRIES);
            assert_eq!(cache.order.len(), cache.entries.len());
        }
        let previous = cache.bytes;
        cache.insert(
            &spans(&"x".repeat(MAX_ENTRY_BYTES)),
            300.0,
            18.0,
            None,
            vec![FormattedLine::new(18.0)].into(),
        );
        assert_eq!(cache.bytes, previous);
        cache.clear();
        assert_eq!(cache.bytes, 0);
        assert!(cache.entries.is_empty() && cache.order.is_empty());
    }

    #[test]
    fn tree_reuses_paragraphs_without_stale_text_font_or_metrics() {
        use crate::{Element, FontMetrics, Size, UiTree};
        let source = "Texto completo con ñ y 👩🏽‍💻 junto a su resultado.";
        let root = || Element::formatted_text(spans(source));
        let mut tree = UiTree::new(root());
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 3.0);
        let viewport = Size::new(180.0, 500.0);
        let first = tree.layout(viewport).text_layouts();
        tree.set_root(root());
        assert_eq!(tree.layout(viewport).text_layouts(), first);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 9.0);
        let wider = tree.layout(viewport).text_layouts();
        assert_eq!(wider[0].source, source);
        assert!(wider[0].runs.len() > first[0].runs.len());
        tree.set_sans_metrics(FontMetrics {
            char_width: 9.0,
            line_height: 32.0,
        });
        let taller = tree.layout(viewport).text_layouts();
        assert!(taller[0].runs[0].rect.height > wider[0].runs[0].rect.height);
        tree.set_root(Element::formatted_text(spans("Respuesta final distinta.")));
        let final_frame = tree.layout(viewport);
        assert_eq!(
            final_frame.text_layouts()[0].source,
            "Respuesta final distinta."
        );
        assert!(!tree.is_animating());
    }
}
