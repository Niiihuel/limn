//! Text laid out by limn itself: shaped, broken into lines and placed, so the
//! caret, the selection and the glyphs a backend paints all come from one
//! layout. parley does the work; nothing of it is in this module's API.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use parley::fontique::{Collection, CollectionOptions, FontInfoOverride, SourceCache};
use parley::{
    Affinity, Alignment, AlignmentOptions, BoundingBox, Cursor, FontContext, FontFamilyName, FontStyle,
    FontWeight, LayoutContext, LineHeight, PositionedLayoutItem, Selection, StyleProperty,
};

use crate::fonts::{self, Font, Fonts};
use crate::{Color, DrawCommand, FontFamily, Point, Rect, Size};

/// Where a line's glyphs stand when the line is taller than they are.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Leading {
    /// Half the extra room above the glyphs and half below, as CSS sets a
    /// line.
    #[default]
    Even,
    /// The glyphs at the top of the line and the extra room under them, as
    /// limn's measured text and egui set a line.
    Below,
}

/// How a piece of text is set: its size and family, and the height of its
/// lines.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextStyle {
    /// The font size in logical pixels.
    pub size: f32,
    /// The family, and with it the weight and the slant.
    pub family: FontFamily,
    /// Each line's height in logical pixels; `None` takes the font's own,
    /// its ascent, descent and line gap.
    pub line_height: Option<f32>,
    /// Where the glyphs stand in a line taller than they are. `Even` by
    /// default.
    pub leading: Leading,
}

impl TextStyle {
    /// `size` pixels in `family`, at the font's own line height. A size that
    /// is not finite is the default 14; one below zero is zero.
    pub fn new(size: f32, family: FontFamily) -> Self {
        let size = if size.is_finite() { size.max(0.0) } else { crate::DEFAULT_FONT_SIZE };
        Self { size, family, line_height: None, leading: Leading::Even }
    }

    /// Set every line `height` pixels apart. A height that is not finite or
    /// below zero is ignored.
    pub fn line_height(mut self, height: f32) -> Self {
        if height.is_finite() && height >= 0.0 {
            self.line_height = Some(height);
        }
        self
    }

    /// Set where the glyphs stand in a line taller than they are.
    pub fn leading(mut self, leading: Leading) -> Self {
        self.leading = leading;
        self
    }
}

/// One line of a [`TextLayout`].
#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    /// The line's box, from the layout's top-left corner: as wide as its
    /// content and as tall as the line.
    pub rect: Rect,
    /// How far down the layout the line's baseline is.
    pub baseline: f32,
    /// The bytes of the text it holds.
    pub range: Range<usize>,
}

/// A glyph of a [`GlyphRun`]: which one of the run's font, and where its
/// origin is on the baseline, from the run's origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glyph {
    /// The glyph's index in the font.
    pub id: u32,
    /// How far right of the run's origin it starts.
    pub x: f32,
    /// How far down from the run's origin its baseline is.
    pub y: f32,
}

/// Glyphs of one font that limn has shaped and placed. A backend draws each
/// from [`GlyphRun::font`] at [`GlyphRun::size`], tinted `color`; it lays
/// nothing out again.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GlyphRun {
    /// The top-left corner of the layout the run belongs to; every glyph is
    /// placed from it.
    pub origin: Point,
    /// The face to draw the glyphs from.
    pub font: Font,
    /// The font size in logical pixels.
    pub size: f32,
    /// The instance of a variable font, as normalized coordinates in 2.14
    /// fixed point, one for each axis; empty for a static font.
    pub coords: Arc<[i16]>,
    /// Thicken the outlines: a bold the face has no weight for.
    pub embolden: bool,
    /// Slant the outlines this many degrees: an italic the face has no
    /// style for.
    pub skew: Option<f32>,
    /// The glyphs in the order they are drawn.
    pub glyphs: Arc<[Glyph]>,
    /// The ink. A monochrome glyph is drawn in it; a color one keeps its own.
    pub color: Color,
    /// The text the run shows: for tests, tools and accessibility, never
    /// drawn.
    pub text: Arc<str>,
}

/// Text that limn has shaped, broken into lines and placed. Where the caret
/// goes, what a selection covers, and the glyphs a backend draws all come
/// from it, so they always agree. Clones share the layout.
///
/// Offsets into the text are bytes. One inside a character is taken as the
/// start of that character.
#[derive(Clone)]
pub struct TextLayout {
    text: Arc<str>,
    style: TextStyle,
    layout: Arc<parley::Layout<()>>,
    /// The faces it was laid out in, which its runs are drawn from.
    fonts: Arc<Fonts>,
    /// Its runs, made on the first draw and shared by every clone, so a
    /// backend sees the same glyphs from one frame to the next.
    runs: Arc<OnceLock<Vec<GlyphRun>>>,
}

impl TextLayout {
    /// `text` set in `style`, one line for each line it has, however long.
    pub fn new(text: impl Into<Arc<str>>, style: TextStyle) -> Self {
        let text = text.into();
        let (mut layout, fonts) = with_system(|system| (system.shape(&text, &style), system.given.clone()));
        place(&mut layout, None, Alignment::Start);
        Self { text, style, layout: Arc::new(layout), fonts, runs: Arc::default() }
    }

    /// The same text broken into lines no wider than `width`, where the
    /// language allows a break, without shaping it again. A word longer than
    /// `width` keeps its own line. A width that is not finite does not wrap;
    /// one below zero is zero.
    pub fn wrapped(&self, width: f32) -> Self {
        self.placed(width, Alignment::Start)
    }

    /// The same text broken into lines as [`Self::wrapped`] breaks it, each
    /// centered across `width`. A width that is not finite centers each line
    /// across the widest.
    pub fn centered(&self, width: f32) -> Self {
        self.placed(width, Alignment::Center)
    }

    fn placed(&self, width: f32, alignment: Alignment) -> Self {
        let width = width.is_finite().then(|| width.max(0.0));
        let mut layout = parley::Layout::clone(&self.layout);
        place(&mut layout, width, alignment);
        Self {
            text: self.text.clone(),
            style: self.style,
            layout: Arc::new(layout),
            fonts: self.fonts.clone(),
            runs: Arc::default(),
        }
    }

    /// The text laid out.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The style it is set in.
    pub fn style(&self) -> TextStyle {
        self.style
    }

    /// As wide as its widest line and as tall as its lines. Empty text is
    /// one line tall.
    pub fn size(&self) -> Size {
        // Empty text is laid out as a space, for its line's height.
        let width = if self.text.is_empty() { 0.0 } else { self.layout.width() };
        Size::new(width, self.layout.height())
    }

    /// Its lines, top to bottom. A line's box leaves out the spaces that
    /// hang past its end where it wraps.
    pub fn lines(&self) -> impl Iterator<Item = TextLine> + '_ {
        let length = self.text.len();
        self.layout.lines().map(move |line| {
            let metrics = line.metrics();
            let width = if length == 0 { 0.0 } else { metrics.advance - metrics.hanging_advance };
            let range = line.text_range();
            TextLine {
                rect: Rect::new(
                    metrics.offset,
                    metrics.block_min_coord,
                    width,
                    metrics.block_max_coord - metrics.block_min_coord,
                ),
                baseline: metrics.baseline + self.lift(metrics),
                range: range.start.min(length)..range.end.min(length),
            }
        })
    }

    /// Where a caret before the byte at `index` stands: zero wide, as tall as
    /// its line. At a line that wraps, the end of one line is the start of
    /// the next, and the caret stands at the start of the next. An index
    /// past the end is the end.
    pub fn caret(&self, index: usize) -> Rect {
        let cursor = Cursor::from_byte_index(&self.layout, self.boundary(index), Affinity::Downstream);
        rect(cursor.geometry(&self.layout, 0.0))
    }

    /// The offset of the caret position nearest `point`, from the layout's
    /// top-left corner: a click there puts the caret here. Never inside a
    /// grapheme. A point that is not finite is the start.
    pub fn index_at(&self, point: Point) -> usize {
        if !point.x.is_finite() || !point.y.is_finite() {
            return 0;
        }
        Cursor::from_point(&self.layout, point.x, point.y).index()
    }

    /// The boxes a selection of `range` covers, one for each line it
    /// touches, in the text's order. An empty range covers nothing.
    pub fn selection(&self, range: Range<usize>) -> Vec<Rect> {
        let (start, end) = (self.boundary(range.start), self.boundary(range.end));
        if start >= end {
            return Vec::new();
        }
        let anchor = Cursor::from_byte_index(&self.layout, start, Affinity::Downstream);
        let focus = Cursor::from_byte_index(&self.layout, end, Affinity::Upstream);
        Selection::new(anchor, focus)
            .geometry(&self.layout)
            .into_iter()
            .map(|(bounds, _)| rect(bounds))
            .collect()
    }

    /// The commands that draw the text with its top-left corner at `origin`,
    /// in `color`: one [`DrawCommand::Glyphs`] for each run of one font. The
    /// runs of one layout, and of its clones, share their glyphs from one
    /// call to the next.
    pub fn draw(&self, origin: Point, color: Color) -> Vec<DrawCommand> {
        self.runs()
            .iter()
            .map(|run| DrawCommand::Glyphs(GlyphRun { origin, color, ..run.clone() }))
            .collect()
    }

    fn runs(&self) -> &[GlyphRun] {
        self.runs.get_or_init(|| {
            if self.text.is_empty() {
                return Vec::new();
            }
            let mut runs = Vec::new();
            for line in self.layout.lines() {
                let lift = self.lift(line.metrics());
                for item in line.items() {
                    let PositionedLayoutItem::GlyphRun(placed) = item else {
                        continue;
                    };
                    let run = placed.run();
                    let synthesis = run.synthesis();
                    runs.push(GlyphRun {
                        origin: Point::new(0.0, 0.0),
                        font: self.fonts.find(run.font()),
                        size: run.font_size(),
                        coords: run.normalized_coords().iter().map(|coord| coord.to_bits()).collect(),
                        embolden: synthesis.embolden(),
                        skew: synthesis.skew(),
                        glyphs: placed
                            .positioned_glyphs()
                            .map(|glyph| Glyph { id: glyph.id, x: glyph.x, y: glyph.y + lift })
                            .collect(),
                        color: Color::rgba(0, 0, 0, 0),
                        text: self.text.get(run.text_range()).unwrap_or_default().into(),
                    });
                }
            }
            runs
        })
    }

    /// How far down a line's glyphs move from where parley sets them.
    fn lift(&self, metrics: &parley::LineMetrics) -> f32 {
        match self.style.leading {
            Leading::Even => 0.0,
            Leading::Below => metrics.block_min_coord - metrics.content_block_min_coord,
        }
    }

    fn boundary(&self, index: usize) -> usize {
        self.text.floor_char_boundary(index.min(self.text.len()))
    }
}

impl fmt::Debug for TextLayout {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextLayout")
            .field("text", &self.text)
            .field("style", &self.style)
            .field("size", &self.size())
            .finish()
    }
}

fn place(layout: &mut parley::Layout<()>, width: Option<f32>, alignment: Alignment) {
    layout.break_all_lines(width);
    layout.align(alignment, AlignmentOptions::default());
}

fn rect(bounds: BoundingBox) -> Rect {
    Rect::new(bounds.x0 as f32, bounds.y0 as f32, (bounds.x1 - bounds.x0) as f32, (bounds.y1 - bounds.y0) as f32)
}

// The family each face is registered under, and what it is asked for with.
const FAMILIES: [(FontFamily, &str, f32, FontStyle); 6] = [
    (FontFamily::Monospace, "limn-mono", 400.0, FontStyle::Normal),
    (FontFamily::Sans, "limn-sans", 400.0, FontStyle::Normal),
    (FontFamily::SansItalic, "limn-sans-italic", 400.0, FontStyle::Italic),
    (FontFamily::SansMedium, "limn-sans-medium", 500.0, FontStyle::Normal),
    (FontFamily::SansBold, "limn-sans-bold", 700.0, FontStyle::Normal),
    (FontFamily::SansBoldItalic, "limn-sans-bold-italic", 700.0, FontStyle::Italic),
];

fn slot(family: FontFamily) -> usize {
    FAMILIES.iter().position(|(given, ..)| *given == family).expect("every family has a slot")
}

/// What one thread lays text out with, rebuilt when the fonts change.
struct System {
    generation: u64,
    given: Arc<Fonts>,
    fonts: FontContext,
    layouts: LayoutContext<()>,
    /// For each family, the faces tried in order.
    stacks: [Vec<FontFamilyName<'static>>; FAMILIES.len()],
}

thread_local! {
    static SYSTEM: RefCell<Option<System>> = const { RefCell::new(None) };
}

fn with_system<R>(work: impl FnOnce(&mut System) -> R) -> R {
    let generation = fonts::generation();
    SYSTEM.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.as_ref().is_none_or(|system| system.generation != generation) {
            *slot = Some(System::new(generation));
        }
        work(slot.as_mut().expect("just made"))
    })
}

impl System {
    fn new(generation: u64) -> Self {
        let given = fonts::current();
        // Only the given faces: text is laid out alike on every machine.
        let mut collection = Collection::new(CollectionOptions { shared: false, system_fonts: false });
        let mut register = |font: &Font, name: &str, weight: Option<f32>, style: Option<FontStyle>| {
            let data = font.data();
            let only = FontInfoOverride {
                family_name: Some(name),
                weight: weight.map(FontWeight::new),
                style,
                ..FontInfoOverride::default()
            };
            collection.register_fonts(data.data.clone(), Some(only));
        };
        for (family, name, weight, style) in FAMILIES {
            if let Some(font) = given.face_of(family) {
                register(font, name, Some(weight), Some(style));
            }
        }
        let fallbacks: Vec<String> = (0..given.fallbacks().len()).map(|index| format!("limn-fallback-{index}")).collect();
        for (font, name) in given.fallbacks().iter().zip(&fallbacks) {
            register(font, name, None, None);
        }
        if let Some(font) = given.emoji_face() {
            register(font, "limn-emoji", None, None);
        }
        let named = |name: &str| FontFamilyName::Named(Cow::Owned(name.to_owned()));
        let stack = |family: FontFamily| {
            let own = FAMILIES[slot(family)].1;
            let mut names = vec![named(own)];
            match family {
                FontFamily::Monospace => {
                    names.push(named("limn-sans"));
                    names.extend(fallbacks.iter().map(|name| named(name)));
                }
                _ => {
                    // Without a face of its own, a bold italic is the bold face slanted.
                    if family == FontFamily::SansBoldItalic {
                        names.push(named("limn-sans-bold"));
                    }
                    if family != FontFamily::Sans {
                        names.push(named("limn-sans"));
                    }
                    names.extend(fallbacks.iter().map(|name| named(name)));
                    names.push(named("limn-mono"));
                }
            }
            // Last, so it never takes a symbol a text face has.
            names.push(named("limn-emoji"));
            names
        };
        Self {
            generation,
            given: given.clone(),
            fonts: FontContext { collection, source_cache: SourceCache::default() },
            layouts: LayoutContext::new(),
            stacks: FAMILIES.map(|(family, ..)| stack(family)),
        }
    }

    fn shape(&mut self, text: &str, style: &TextStyle) -> parley::Layout<()> {
        let (_, _, weight, slant) = FAMILIES[slot(style.family)];
        let mut builder = self.layouts.ranged_builder(&mut self.fonts, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(parley::FontFamily::List(Cow::Borrowed(
            &self.stacks[slot(style.family)],
        ))));
        builder.push_default(StyleProperty::FontSize(style.size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
        builder.push_default(StyleProperty::FontStyle(slant));
        builder.push_default(StyleProperty::LineHeight(match style.line_height {
            Some(height) => LineHeight::Absolute(height),
            None => LineHeight::MetricsRelative(1.0),
        }));
        builder.build(text)
    }
}
