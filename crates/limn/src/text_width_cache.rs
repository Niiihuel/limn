//! Bounded, per-surface shaping measurements shared across layout frames.
//! Lookups borrow the source text; unchanged labels do not allocate a key.

use std::collections::{HashMap, VecDeque};

use crate::FontFamily;

const MAX_ENTRIES: usize = 4096;
const MAX_TEXT_BYTES: usize = 256;
type FontKey = (u32, u8);

#[derive(Default)]
pub(crate) struct TextWidthCache {
    fonts: HashMap<FontKey, HashMap<String, f32>>,
    order: VecDeque<(FontKey, String)>,
}

impl TextWidthCache {
    fn font_key(size: f32, family: FontFamily) -> FontKey {
        (size.to_bits(), family as u8)
    }

    pub(crate) fn get(&self, text: &str, size: f32, family: FontFamily) -> Option<f32> {
        if text.len() > MAX_TEXT_BYTES {
            return None;
        }
        self.fonts
            .get(&Self::font_key(size, family))?
            .get(text)
            .copied()
    }

    pub(crate) fn insert(&mut self, text: &str, size: f32, family: FontFamily, width: f32) {
        if text.len() > MAX_TEXT_BYTES || !width.is_finite() {
            return;
        }
        let key = Self::font_key(size, family);
        if self
            .fonts
            .get(&key)
            .is_some_and(|entries| entries.contains_key(text))
        {
            return;
        }
        if self.order.len() == MAX_ENTRIES {
            if let Some((old_font, old_text)) = self.order.pop_front() {
                if let Some(entries) = self.fonts.get_mut(&old_font) {
                    entries.remove(&old_text);
                    if entries.is_empty() {
                        self.fonts.remove(&old_font);
                    }
                }
            }
        }
        self.fonts
            .entry(key)
            .or_default()
            .insert(text.to_owned(), width);
        self.order.push_back((key, text.to_owned()));
    }

    pub(crate) fn clear(&mut self) {
        self.fonts.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurements_are_separate_for_font_and_size_and_memory_stays_bounded() {
        let mut cache = TextWidthCache::default();
        cache.insert("hola", 12.0, FontFamily::Sans, 23.0);
        assert_eq!(cache.get("hola", 12.0, FontFamily::Sans), Some(23.0));
        assert_eq!(cache.get("hola", 14.0, FontFamily::Sans), None);
        assert_eq!(cache.get("hola", 12.0, FontFamily::Monospace), None);
        // Every face measures apart: a bold label is never given a monospace width.
        for family in [FontFamily::SansItalic, FontFamily::SansMedium, FontFamily::SansBold] {
            assert_eq!(cache.get("hola", 12.0, family), None);
        }
        cache.insert("hola", 12.0, FontFamily::Monospace, 32.0);
        assert_eq!(cache.get("hola", 12.0, FontFamily::SansBold), None);
        for index in 0..MAX_ENTRIES * 2 {
            cache.insert(&format!("token-{index}"), 12.0, FontFamily::Sans, 40.0);
        }
        assert_eq!(cache.order.len(), MAX_ENTRIES);
        assert_eq!(
            cache.fonts.values().map(HashMap::len).sum::<usize>(),
            MAX_ENTRIES
        );
        assert_eq!(cache.get("hola", 12.0, FontFamily::Sans), None);
        assert_eq!(cache.get("token-8191", 12.0, FontFamily::Sans), Some(40.0));
        cache.insert(
            &"x".repeat(MAX_TEXT_BYTES + 1),
            12.0,
            FontFamily::Sans,
            400.0,
        );
        assert_eq!(cache.order.len(), MAX_ENTRIES);
        cache.clear();
        assert!(cache.fonts.is_empty());
        assert!(cache.order.is_empty());
    }
}
