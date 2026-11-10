//! Shaping measurements shared across layout frames, per font and text.
//! Lookups borrow the source text; unchanged labels do not allocate a key.
//! A width lasts while layouts read it: what the last layout did not read is
//! forgotten when the next one begins. The cache holds what the window shows,
//! however many texts that is, and nothing it stopped showing; a fixed cap
//! smaller than one frame's texts would make every lookup miss.

use std::cell::Cell;

use crate::{FastMap, FontFamily};

const MAX_TEXT_BYTES: usize = 256;
type FontKey = (u32, FontFamily);

#[derive(Default)]
pub(crate) struct TextWidthCache {
    /// Each width, and the layout that last read it.
    fonts: FastMap<FontKey, FastMap<String, (f32, Cell<u64>)>>,
    /// The layout under way, counted from the first.
    pass: u64,
}

impl TextWidthCache {
    fn font_key(size: f32, family: FontFamily) -> FontKey {
        (size.to_bits(), family)
    }

    /// Start a layout: forget the widths the last one did not read.
    pub(crate) fn begin_pass(&mut self) {
        let ended = self.pass;
        for entries in self.fonts.values_mut() {
            entries.retain(|_, (_, read)| read.get() >= ended);
        }
        self.fonts.retain(|_, entries| !entries.is_empty());
        self.pass += 1;
    }

    pub(crate) fn get(&self, text: &str, size: f32, family: FontFamily) -> Option<f32> {
        if text.len() > MAX_TEXT_BYTES {
            return None;
        }
        let (width, read) = self.fonts.get(&Self::font_key(size, family))?.get(text)?;
        read.set(self.pass);
        Some(*width)
    }

    pub(crate) fn insert(&mut self, text: &str, size: f32, family: FontFamily, width: f32) {
        if text.len() > MAX_TEXT_BYTES || !width.is_finite() {
            return;
        }
        let entries = self.fonts.entry(Self::font_key(size, family)).or_default();
        if !entries.contains_key(text) {
            entries.insert(text.to_owned(), (width, Cell::new(self.pass)));
        }
    }

    pub(crate) fn clear(&mut self) {
        self.fonts.clear();
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.fonts.values().map(FastMap::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measurements_are_separate_for_font_and_size() {
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
        cache.insert(&"x".repeat(MAX_TEXT_BYTES + 1), 12.0, FontFamily::Sans, 400.0);
        assert_eq!(cache.len(), 2, "a long text is not kept");
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn a_width_lasts_while_layouts_read_it_however_many_one_layout_reads() {
        let mut cache = TextWidthCache::default();
        cache.begin_pass();
        // More texts than the old cap of 4,096: every one is still there.
        for index in 0..10_000 {
            cache.insert(&format!("row {index}"), 12.0, FontFamily::Sans, 40.0);
        }
        cache.insert("tooltip", 12.0, FontFamily::Sans, 50.0);
        cache.begin_pass();
        for index in 0..10_000 {
            assert_eq!(cache.get(&format!("row {index}"), 12.0, FontFamily::Sans), Some(40.0));
        }
        assert_eq!(cache.len(), 10_001, "the layout that measured them ended; they are kept for the next");
        cache.begin_pass();
        assert_eq!(cache.get("tooltip", 12.0, FontFamily::Sans), None, "a layout went by without it");
        assert_eq!(cache.get("row 9999", 12.0, FontFamily::Sans), Some(40.0));
        cache.begin_pass();
        cache.begin_pass();
        assert_eq!(cache.len(), 0, "nothing read for a layout is forgotten");
    }
}
