//! The faces text is laid out in, one set for the whole process, shared by
//! the layout and by every backend that paints its glyphs.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

use parley::fontique::{Blob, Collection, CollectionOptions};
use parley::FontData;

use crate::FontFamily;

/// One face of a font file: what limn lays text out in, and what a backend
/// draws the glyphs of a [`crate::GlyphRun`] from. Clones share the bytes
/// and the [`id`](Self::id).
#[derive(Clone)]
pub struct Font {
    data: FontData,
    compiled: Option<&'static [u8]>,
}

impl Font {
    /// The first face of a TrueType or OpenType file, or an error if the
    /// bytes hold no face limn can lay text out in.
    pub fn new(bytes: impl Into<Arc<[u8]>>) -> Result<Self, FontError> {
        Self::checked(Blob::new(Arc::new(bytes.into())), 0, None)
    }

    /// The face at `index` of a font collection (`.ttc`); 0 is the first.
    pub fn in_collection(bytes: impl Into<Arc<[u8]>>, index: u32) -> Result<Self, FontError> {
        Self::checked(Blob::new(Arc::new(bytes.into())), index, None)
    }

    /// The first face of a font compiled into the program, as
    /// `include_bytes!` gives one.
    pub fn from_static(bytes: &'static [u8]) -> Result<Self, FontError> {
        Self::checked(Blob::new(Arc::new(bytes)), 0, Some(bytes))
    }

    fn checked(blob: Blob<u8>, index: u32, compiled: Option<&'static [u8]>) -> Result<Self, FontError> {
        // The collection reads the face's tables as layout will.
        let faces = Collection::new(CollectionOptions { shared: false, system_fonts: false })
            .register_fonts(blob.clone(), None);
        let found = faces.iter().flat_map(|(_, faces)| faces).any(|face| face.index() == index);
        found.then(|| Self { data: FontData::new(blob, index), compiled }).ok_or(FontError)
    }

    /// The whole file the face is in.
    pub fn bytes(&self) -> &[u8] {
        self.data.data.data()
    }

    /// The whole file, when it is compiled into the program, as
    /// [`Font::from_static`] takes one: a backend can keep these bytes
    /// without copying them.
    pub fn static_bytes(&self) -> Option<&'static [u8]> {
        self.compiled
    }

    /// Which face of the file this is; 0 unless it is a collection.
    pub fn index(&self) -> u32 {
        self.data.index
    }

    /// The same for every clone of this font and different for any other
    /// file: what a backend keys the glyphs it has drawn by.
    pub fn id(&self) -> u64 {
        self.data.data.id()
    }

    pub(crate) fn data(&self) -> &FontData {
        &self.data
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id() && self.index() == other.index()
    }
}

impl fmt::Debug for Font {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Font")
            .field("id", &self.id())
            .field("index", &self.index())
            .field("bytes", &self.bytes().len())
            .finish()
    }
}

/// Bytes that hold no font face limn can lay text out in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontError;

impl fmt::Display for FontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("the bytes hold no font face limn can lay text out in")
    }
}

impl std::error::Error for FontError {}

/// The faces text is laid out in: a face for each [`FontFamily`] that has
/// one, fallbacks tried in order for the characters it lacks, and a face for
/// emoji, tried last.
///
/// A family without a face of its own takes the one it is a style of:
/// `SansItalic`, `SansMedium` and `SansBold` take `Sans`, slanted or
/// emboldened. Behind `Sans` comes every fallback, then `Monospace`, then
/// the emoji face; behind `Monospace` comes `Sans`, every fallback, then the
/// emoji face. The emoji face is last so it never takes a symbol a text face
/// has.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fonts {
    faces: Vec<(FontFamily, Font)>,
    fallbacks: Vec<Font>,
    emoji: Option<Font>,
}

impl Fonts {
    /// No faces at all: text takes no room and draws nothing until faces
    /// are given.
    pub fn new() -> Self {
        Self::default()
    }

    /// limn's own faces: Ubuntu for `Sans` in regular, medium and bold, Hack
    /// for `Monospace`, Noto Sans Math and Noto Sans Symbols 2 behind both,
    /// and Noto Emoji for emoji. `SansItalic` is `Sans` slanted. Their
    /// licenses are in `assets/fonts/NOTICE.md`.
    #[cfg(feature = "bundled-fonts")]
    pub fn bundled() -> Self {
        let font = |bytes: &'static [u8]| Font::from_static(bytes).expect("a bundled face is a font");
        Self::new()
            .face(FontFamily::Sans, font(include_bytes!("../assets/fonts/Ubuntu-Regular.ttf")))
            .face(FontFamily::SansMedium, font(include_bytes!("../assets/fonts/Ubuntu-Medium.ttf")))
            .face(FontFamily::SansBold, font(include_bytes!("../assets/fonts/Ubuntu-Bold.ttf")))
            .face(FontFamily::Monospace, font(include_bytes!("../assets/fonts/Hack-Regular.ttf")))
            .fallback(font(include_bytes!("../assets/fonts/NotoSansMath-Regular.ttf")))
            .fallback(font(include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.otf")))
            .emoji(font(include_bytes!("../assets/fonts/NotoEmoji.ttf")))
    }

    /// Lay `family` out in `font`, in place of the face it had.
    pub fn face(mut self, family: FontFamily, font: Font) -> Self {
        self.faces.retain(|(given, _)| *given != family);
        self.faces.push((family, font));
        self
    }

    /// Try `font` for characters the faces lack, after the fallbacks
    /// already given.
    pub fn fallback(mut self, font: Font) -> Self {
        self.fallbacks.push(font);
        self
    }

    /// Lay emoji out in `font`, tried after every other face, in place of the
    /// emoji face there was.
    pub fn emoji(mut self, font: Font) -> Self {
        self.emoji = Some(font);
        self
    }

    /// The face `family` has of its own, if any.
    pub fn face_of(&self, family: FontFamily) -> Option<&Font> {
        self.faces.iter().find(|(given, _)| *given == family).map(|(_, font)| font)
    }

    /// The fallbacks, in the order they are tried.
    pub fn fallbacks(&self) -> &[Font] {
        &self.fallbacks
    }

    /// The face for emoji, if any.
    pub fn emoji_face(&self) -> Option<&Font> {
        self.emoji.as_ref()
    }

    /// The given face a layout's run names by its data.
    pub(crate) fn find(&self, data: &FontData) -> Font {
        let mut all = self.faces.iter().map(|(_, font)| font).chain(&self.fallbacks).chain(&self.emoji);
        all.find(|font| font.id() == data.data.id() && font.index() == data.index)
            .cloned()
            .unwrap_or_else(|| Font { data: data.clone(), compiled: None })
    }
}

static CURRENT: RwLock<Option<Arc<Fonts>>> = RwLock::new(None);
// Counts replacements, so a thread's layout context knows when it is stale.
static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The faces in force: those given to [`set_fonts`], or else
/// [`Fonts::bundled`] (with the `bundled-fonts` feature) or none.
pub fn fonts() -> Fonts {
    current().as_ref().clone()
}

/// Replace the faces for the whole process. Every [`crate::TextLayout`] made
/// from here on uses them; one made before keeps the faces it was made with.
pub fn set_fonts(fonts: Fonts) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(fonts));
    GENERATION.fetch_add(1, Ordering::Release);
}

pub(crate) fn current() -> Arc<Fonts> {
    if let Some(fonts) = CURRENT.read().unwrap_or_else(PoisonError::into_inner).as_ref() {
        return fonts.clone();
    }
    let mut slot = CURRENT.write().unwrap_or_else(PoisonError::into_inner);
    slot.get_or_insert_with(|| Arc::new(default_fonts())).clone()
}

pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::Acquire)
}

fn default_fonts() -> Fonts {
    #[cfg(feature = "bundled-fonts")]
    return Fonts::bundled();
    #[cfg(not(feature = "bundled-fonts"))]
    return Fonts::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_that_are_not_a_font_are_refused() {
        assert_eq!(Font::new(vec![0_u8; 64]), Err(FontError));
        assert_eq!(Font::from_static(b"not a font"), Err(FontError));
    }

    #[cfg(feature = "bundled-fonts")]
    #[test]
    fn a_face_replaces_the_one_its_family_had_and_fallbacks_keep_their_order() {
        let bundled = Fonts::bundled();
        let hack = bundled.face_of(FontFamily::Monospace).unwrap().clone();
        let replaced = bundled.clone().face(FontFamily::Sans, hack.clone());
        assert_eq!(replaced.face_of(FontFamily::Sans), Some(&hack));
        assert_eq!(replaced.faces.len(), bundled.faces.len());
        assert_eq!(bundled.fallbacks().len(), 2);
        assert!(bundled.emoji_face().is_some());
        assert_eq!(bundled.face_of(FontFamily::SansItalic), None);
        // Clones share the bytes, so a backend keys one face once.
        assert_eq!(hack.id(), bundled.face_of(FontFamily::Monospace).unwrap().id());
    }
}
