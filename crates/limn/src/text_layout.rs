//! Text laid out by limn itself: shaped, broken into lines and placed, so the
//! caret, the selection and the glyphs a backend paints all come from one
//! layout. parley does the work; nothing of it is in this module's API.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use parley::fontique::{Collection, CollectionOptions, FontInfoOverride, SourceCache};
use parley::{
    Affinity, Alignment, AlignmentOptions, BoundingBox, Cursor, FontContext, FontFamilyName, FontStyle,
    FontWeight, InlineBox, InlineBoxKind, LayoutContext, LineHeight, OverflowWrap, PositionedLayoutItem,
    Selection, StyleProperty, TextWrapMode, VerticalAlign, YieldData,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::fonts::{self, Font, Fonts};
use crate::{Color, DrawCommand, FontFamily, Icon, InlineImage, Point, Rect, Size, TextSpan, ICON_SIZE};

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

// Where a line under or through the text stands, as a share of its natural
// line height down from the top of its glyphs; the same as limn's measured
// formatted text draws them.
const UNDERLINE_AT: f32 = 0.86;
const STRIKETHROUGH_AT: f32 = 0.55;
// The room a span's background leaves on each side of its text.
const BACKGROUND_INSET: f32 = 3.0;
const ELLIPSIS: &str = "…";

/// A span of formatted text as a layout keeps it.
#[derive(Clone, Debug, PartialEq)]
struct Span {
    /// Its bytes in the layout's text.
    range: Range<usize>,
    /// `None` takes the ink `draw` is given.
    color: Option<Color>,
    size: f32,
    family: FontFamily,
    underline: bool,
    strikethrough: bool,
    background: Option<Color>,
    icon: Option<Icon>,
    image: Option<InlineImage>,
    /// Kept on one line, as a run with a background is, unless it is wider
    /// than any line.
    whole: bool,
}

/// The parley brush of a run: the span it belongs to.
type Brush = Option<u32>;

// What an inline box stands for, in the low bits of its ID; the span is above.
const ICON_BOX: u64 = 0;
const PICTURE_BOX: u64 = 1;
const PAD_BOX: u64 = 2;

/// A piece of what a layout draws, from its top-left corner. `None` inks
/// take the color `draw` is given.
#[derive(Clone, Debug)]
enum Part {
    Glyphs(GlyphRun, Option<Color>),
    Fill(Rect, Option<Color>, f32),
    Icon(Rect, Icon, Option<Color>),
    Picture(Rect, InlineImage, Arc<str>),
}

/// Text that limn has shaped, broken into lines and placed. Where the caret
/// goes, what a selection covers, and the glyphs a backend draws all come
/// from it, so they always agree. Clones share the layout.
///
/// Offsets into the text are bytes. One inside a character is taken as the
/// start of that character.
#[derive(Clone)]
pub struct TextLayout {
    /// What is laid out: the text, or the part of it kept by
    /// [`Self::max_lines`] followed by «…».
    text: Arc<str>,
    /// How many bytes of `text` are the text's own; past them is the «…».
    shown: usize,
    style: TextStyle,
    spans: Arc<[Span]>,
    /// Each span's width on one line, background included.
    widths: Arc<[f32]>,
    layout: Arc<parley::Layout<Brush>>,
    width: Option<f32>,
    alignment: Alignment,
    /// The faces it was laid out in, which its runs are drawn from.
    fonts: Arc<Fonts>,
    /// What it draws, made on the first draw and shared by every clone, so a
    /// backend sees the same glyphs from one frame to the next.
    parts: Arc<OnceLock<Vec<Part>>>,
    /// Where each span is, found the first time it is asked for and shared
    /// as `parts` is: a tree asks every frame for the spans with an action.
    bounds: Arc<[OnceLock<Vec<Rect>>]>,
}

impl TextLayout {
    /// `text` set in `style`, one line for each line it has, however long.
    pub fn new(text: impl Into<Arc<str>>, style: TextStyle) -> Self {
        let text: Arc<str> = text.into();
        Self::shaped(text.clone(), text.len(), style, Vec::new().into(), None, Alignment::Start)
    }

    /// Formatted text: each span set in its own size, family and ink, with
    /// the lines, background, icon or picture it asks for, as
    /// `Element::formatted_text` draws them. A span with a background stays
    /// on one line unless it is wider than any. `paragraph` sets the least
    /// height of every line, where the glyphs stand in a taller one, and the
    /// line empty text takes. A span's `action` and `tooltip` are the
    /// tree's; [`Self::span_bounds`] says where a span is.
    pub fn formatted(spans: &[TextSpan], paragraph: TextStyle) -> Self {
        let mut text = String::new();
        let mut kept = Vec::with_capacity(spans.len());
        for span in spans {
            let start = text.len();
            text.push_str(&span.text);
            let image = span.image.clone().filter(|_| !span.text.is_empty());
            kept.push(Span {
                range: start..text.len(),
                color: Some(span.color),
                size: if span.font_size.is_finite() && span.font_size > 0.0 {
                    span.font_size
                } else {
                    crate::DEFAULT_FONT_SIZE
                },
                family: span.font_family,
                // Lines are not drawn under a picture, nor is a background.
                underline: span.underline && image.is_none(),
                strikethrough: span.strikethrough && image.is_none(),
                background: span.background.filter(|_| image.is_none() && !span.text.is_empty()),
                icon: span.icon.filter(|_| image.is_none() && !span.text.is_empty()),
                whole: span.background.is_some() && image.is_none(),
                image,
            });
        }
        let length = text.len();
        Self::shaped(text.into(), length, paragraph, kept.into(), None, Alignment::Start)
    }

    fn shaped(
        text: Arc<str>,
        shown: usize,
        style: TextStyle,
        spans: Arc<[Span]>,
        width: Option<f32>,
        alignment: Alignment,
    ) -> Self {
        let (mut layout, fonts) = with_system(|system| (system.shape(&text, &style, &spans), system.given.clone()));
        // The width of each span on one line, before it is ever broken.
        place(&mut layout, None, Alignment::Start, &[]);
        let widths = span_widths(&layout, &spans);
        place(&mut layout, width, alignment, &spans);
        let bounds = unknown(spans.len());
        Self {
            text,
            shown,
            style,
            spans,
            widths,
            layout: Arc::new(layout),
            width,
            alignment,
            fonts,
            parts: Arc::default(),
            bounds,
        }
    }

    /// The same text broken into lines no wider than `width`, where the
    /// language allows a break, without shaping it again. A word longer than
    /// `width` is broken between graphemes, so no line runs past it. A width
    /// that is not finite does not wrap; one below zero is zero.
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
        // A span kept whole that cannot fit a line is let break, as text is.
        let limit = width.unwrap_or(f32::INFINITY);
        if self.spans.iter().zip(self.widths.iter()).any(|(span, wide)| span.whole && *wide > limit + SLACK) {
            let spans: Arc<[Span]> = self
                .spans
                .iter()
                .zip(self.widths.iter())
                .map(|(span, wide)| Span { whole: span.whole && *wide <= limit + SLACK, ..span.clone() })
                .collect();
            return Self::shaped(self.text.clone(), self.shown, self.style, spans, width, alignment);
        }
        let mut layout = parley::Layout::clone(&self.layout);
        place(&mut layout, width, alignment, &self.spans);
        Self {
            layout: Arc::new(layout),
            width,
            alignment,
            parts: Arc::default(),
            bounds: unknown(self.spans.len()),
            ..self.clone()
        }
    }

    /// At most `lines` lines, at least one. When more would follow, the last
    /// one ends in «…» in the style of the text before it, giving up
    /// graphemes, then whole pictures and icons, and the spaces before the
    /// «…», until it fits the width the text is broken at. The «…» is drawn
    /// but is not part of the text: no offset reaches it.
    pub fn max_lines(&self, lines: usize) -> Self {
        let lines = lines.max(1);
        let all: Vec<TextLine> = self.lines().collect();
        if all.len() <= lines {
            return self.clone();
        }
        let last = &all[lines - 1];
        // Where the text may be cut: between graphemes of the last line kept,
        // never inside a picture's span.
        let mut cuts: Vec<usize> = self.text[last.range.clone()]
            .grapheme_indices(true)
            .map(|(at, _)| last.range.start + at)
            .chain([last.range.end])
            .filter(|at| !self.spans.iter().any(|span| span.image.is_some() && span.range.start < *at && *at < span.range.end))
            .collect();
        cuts.dedup();
        let fits = |at: usize| {
            let cut = self.cut(at);
            (cut.lines().count() <= lines).then_some(cut)
        };
        // More text never takes fewer lines: the longest cut that fits.
        let (mut low, mut high) = (0, cuts.len());
        let mut best = None;
        while low < high {
            let middle = (low + high) / 2;
            match fits(cuts[middle]) {
                Some(cut) => {
                    best = Some(cut);
                    low = middle + 1;
                }
                None => high = middle,
            }
        }
        best.unwrap_or_else(|| self.cut(last.range.start))
    }

    /// The text up to `at`, without the spaces before it, followed by «…».
    fn cut(&self, at: usize) -> Self {
        let kept = self.text[..at.min(self.shown)].trim_end();
        let end = kept.len();
        let mut spans: Vec<Span> = self
            .spans
            .iter()
            .filter(|span| span.range.start < end)
            .map(|span| Span { range: span.range.start..span.range.end.min(end), ..span.clone() })
            .collect();
        // The «…» takes the size, family and ink of what it follows.
        let before = spans.last().cloned();
        let style = before.as_ref().map_or((None, self.style.size, self.style.family), |span| {
            (span.color, span.size, span.family)
        });
        let mut text = kept.to_owned();
        text.push_str(ELLIPSIS);
        if !self.spans.is_empty() || before.is_some() {
            spans.push(Span {
                range: end..text.len(),
                color: style.0,
                size: style.1,
                family: style.2,
                underline: false,
                strikethrough: false,
                background: None,
                icon: None,
                image: None,
                whole: false,
            });
        }
        Self::shaped(text.into(), end, self.style, spans.into(), self.width, self.alignment)
    }

    /// The text laid out: when it was cut by [`Self::max_lines`], the part
    /// kept and the «…».
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The style it is set in; for formatted text, the paragraph's.
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
    /// hang past its end where it wraps. A range never reaches the «…» of a
    /// text cut to its lines.
    pub fn lines(&self) -> impl Iterator<Item = TextLine> + '_ {
        let (length, shown) = (self.text.len(), self.shown);
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
                range: range.start.min(shown)..range.end.min(shown),
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
    /// grapheme, nor past the text into a «…». A point that is not finite is
    /// the start.
    pub fn index_at(&self, point: Point) -> usize {
        if !point.x.is_finite() || !point.y.is_finite() {
            return 0;
        }
        Cursor::from_point(&self.layout, point.x, point.y).index().min(self.shown)
    }

    /// The boxes a selection of `range` covers, one for each line it
    /// touches, in the text's order. A picture is covered whole. An empty
    /// range covers nothing.
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

    /// Where the span at `index` of [`Self::formatted`] is: one box for each
    /// line it is on, as tall as the line, its icon, picture and background
    /// included. A span past the text, or cut away, has none.
    pub fn span_bounds(&self, index: usize) -> Vec<Rect> {
        let Some(span) = self.spans.get(index).filter(|span| span.range.start < self.shown) else {
            return Vec::new();
        };
        self.bounds[index]
            .get_or_init(|| {
                let mut boxes = self.selection(span.range.start..span.range.end.min(self.shown));
                if span.background.is_some() {
                    // The room after the text belongs to it too.
                    for bounds in &mut boxes {
                        bounds.width += BACKGROUND_INSET;
                    }
                }
                boxes
            })
            .clone()
    }

    /// The commands that draw the text with its top-left corner at `origin`:
    /// backgrounds, then each run of one font as a [`DrawCommand::Glyphs`]
    /// with its lines, icons and pictures. Text with no ink of its own, as
    /// all of [`Self::new`]'s, is drawn in `color`. The runs of one layout,
    /// and of its clones, share their glyphs from one call to the next.
    pub fn draw(&self, origin: Point, color: Color) -> Vec<DrawCommand> {
        let moved = |area: Rect| Rect::new(area.x + origin.x, area.y + origin.y, area.width, area.height);
        self.parts()
            .iter()
            .map(|part| match part {
                Part::Glyphs(run, ink) => DrawCommand::Glyphs(GlyphRun { origin, color: ink.unwrap_or(color), ..run.clone() }),
                Part::Fill(area, ink, radius) => {
                    DrawCommand::FillRect { rect: moved(*area), color: ink.unwrap_or(color), radius: *radius }
                }
                Part::Icon(area, icon, ink) => DrawCommand::Icon { rect: moved(*area), icon: *icon, tint: ink.unwrap_or(color) },
                Part::Picture(area, InlineImage::Raster(image), _) => {
                    DrawCommand::RasterImage { rect: moved(*area), image: image.clone(), opacity: 1.0, radius: 0.0 }
                }
                Part::Picture(area, InlineImage::Surface(id), label) => DrawCommand::Surface {
                    rect: moved(*area),
                    id: id.clone(),
                    label: label.to_string(),
                    opacity: 1.0,
                },
            })
            .collect()
    }

    fn parts(&self) -> &[Part] {
        self.parts.get_or_init(|| {
            if self.text.is_empty() {
                return Vec::new();
            }
            let mut backgrounds = Vec::new();
            let mut parts = Vec::new();
            for line in self.layout.lines() {
                let metrics = line.metrics();
                let lift = self.lift(metrics);
                let middle = (metrics.block_min_coord + metrics.block_max_coord) / 2.0;
                // Each background's extent on this line, from its first box to its last.
                let mut extents: Vec<(usize, f32, f32)> = Vec::new();
                let mut extend = |index: usize, from: f32, to: f32| match extents.iter_mut().find(|(span, ..)| *span == index) {
                    Some((_, start, end)) => {
                        *start = start.min(from);
                        *end = end.max(to);
                    }
                    None => extents.push((index, from, to)),
                };
                for item in line.items() {
                    match item {
                        PositionedLayoutItem::GlyphRun(placed) => {
                            let index = placed.style().brush.map(|index| index as usize);
                            let span = index.and_then(|index| self.spans.get(index));
                            if let (Some(index), Some(span)) = (index, span) {
                                if span.background.is_some() {
                                    extend(index, placed.offset(), placed.offset() + placed.advance());
                                }
                            }
                            if span.is_some_and(|span| span.image.is_some()) || placed.glyphs().next().is_none() {
                                continue;
                            }
                            let run = placed.run();
                            let synthesis = run.synthesis();
                            let ink = span.and_then(|span| span.color);
                            parts.push(Part::Glyphs(
                                GlyphRun {
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
                                    // Spans of one font are shaped together; the brush says whose text this is.
                                    text: {
                                        let shaped = run.text_range();
                                        let own = span.map_or(shaped.clone(), |span| {
                                            shaped.start.max(span.range.start)..shaped.end.min(span.range.end)
                                        });
                                        self.text.get(own).unwrap_or_default().into()
                                    },
                                },
                                ink,
                            ));
                            if let Some(span) = span.filter(|span| span.underline || span.strikethrough) {
                                // As the measured text draws them: from the top of the glyphs.
                                let font = run.font_metrics();
                                let top = placed.baseline() + lift - font.ascent;
                                let natural = font.ascent + font.descent + font.leading;
                                let thickness = (span.size / 14.0).max(1.0);
                                for (drawn, at) in [(span.underline, UNDERLINE_AT), (span.strikethrough, STRIKETHROUGH_AT)] {
                                    if drawn {
                                        parts.push(Part::Fill(
                                            Rect::new(placed.offset(), top + natural * at - thickness / 2.0, placed.advance(), thickness),
                                            ink,
                                            0.0,
                                        ));
                                    }
                                }
                            }
                        }
                        PositionedLayoutItem::InlineBox(placed) => {
                            let index = (placed.id >> 2) as usize;
                            let Some(span) = self.spans.get(index) else {
                                continue;
                            };
                            match placed.id & 0b11 {
                                ICON_BOX => {
                                    if let Some(icon) = span.icon {
                                        let area = Rect::new(placed.x, middle - ICON_SIZE / 2.0, ICON_SIZE, ICON_SIZE);
                                        parts.push(Part::Icon(area, icon, span.color));
                                    }
                                }
                                PICTURE_BOX => {
                                    if let Some(image) = &span.image {
                                        let side = placed.width;
                                        let area = Rect::new(placed.x, middle - side / 2.0, side, side);
                                        let label = self.text.get(span.range.clone()).unwrap_or_default().into();
                                        parts.push(Part::Picture(area, image.clone(), label));
                                    }
                                }
                                _ => extend(index, placed.x, placed.x + placed.width),
                            }
                        }
                    }
                }
                for (index, start, end) in extents {
                    let span = &self.spans[index];
                    let height = with_system(|system| system.natural(span.size, span.family));
                    let area = Rect::new(start, middle - height / 2.0, end - start, height);
                    backgrounds.push(Part::Fill(area, span.background, BACKGROUND_INSET));
                }
            }
            backgrounds.extend(parts);
            backgrounds
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
        self.text.floor_char_boundary(index.min(self.shown))
    }
}

impl fmt::Debug for TextLayout {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TextLayout")
            .field("text", &self.text)
            .field("style", &self.style)
            .field("spans", &self.spans.len())
            .field("size", &self.size())
            .finish()
    }
}

/// How far past its width a line may run. A box sized to its text and handed
/// that width back by sums can come out a rounding error short of it; its
/// last letter would go to a line of its own.
const SLACK: f32 = 0.01;

/// Break `layout` into lines no wider than `width` and align them. A span's
/// icon never ends a line its text does not start on: such a line is broken
/// again before the icon, which goes down with the span's first word.
fn place(layout: &mut parley::Layout<Brush>, width: Option<f32>, alignment: Alignment, spans: &[Span]) {
    let max = width.map_or(f32::MAX, |width| width + SLACK);
    // Lines broken narrower than `max`, and how narrow.
    let mut narrower: Vec<(usize, f32)> = Vec::new();
    loop {
        let mut breaker = layout.break_lines();
        breaker.state_mut().set_layout_max_advance(max);
        let mut line = 0;
        loop {
            let limit = narrower.iter().find(|(at, _)| *at == line).map_or(max, |(_, limit)| *limit);
            breaker.state_mut().set_line_max_advance(limit);
            match breaker.break_next() {
                Some(YieldData::LineBreak(_)) => line += 1,
                // Only in-flow boxes are made here; place any other where it is, as parley does.
                Some(YieldData::InlineBoxBreak(_)) => {
                    let x = breaker.state().line_x();
                    breaker.state_mut().append_inline_box_to_line(x, 0.0, 0.0);
                }
                Some(YieldData::MaxHeightExceeded(_)) | None => break,
            }
        }
        breaker.finish();
        match stranded_icon(layout, spans) {
            Some((line, x)) if !narrower.iter().any(|(at, _)| *at == line) => narrower.push((line, x)),
            _ => break,
        }
    }
    layout.align(alignment, AlignmentOptions::default());
}

/// The first line, but the last, that ends in a span's icon while the span's
/// text starts on the next line, and where that icon starts.
fn stranded_icon(layout: &parley::Layout<Brush>, spans: &[Span]) -> Option<(usize, f32)> {
    let count = layout.len();
    layout.lines().enumerate().take(count.saturating_sub(1)).find_map(|(index, line)| {
        let PositionedLayoutItem::InlineBox(placed) = line.items().last()? else {
            return None;
        };
        let span = spans.get((placed.id >> 2) as usize)?;
        (placed.id & 0b11 == ICON_BOX && span.range.start >= line.text_range().end && placed.x > 0.0)
            .then_some((index, placed.x))
    })
}

/// Each span's advance on one line: its runs and its boxes.
fn span_widths(layout: &parley::Layout<Brush>, spans: &[Span]) -> Arc<[f32]> {
    let mut widths = vec![0.0; spans.len()];
    for line in layout.lines() {
        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(placed) => {
                    if let Some(index) = placed.style().brush {
                        if let Some(width) = widths.get_mut(index as usize) {
                            *width += placed.advance();
                        }
                    }
                }
                PositionedLayoutItem::InlineBox(placed) => {
                    if let Some(width) = widths.get_mut((placed.id >> 2) as usize) {
                        *width += placed.width;
                    }
                }
            }
        }
    }
    widths.into()
}

/// A place for the bounds of each of `spans` spans, none found yet.
fn unknown(spans: usize) -> Arc<[OnceLock<Vec<Rect>>]> {
    (0..spans).map(|_| OnceLock::new()).collect()
}

fn rect(bounds: BoundingBox) -> Rect {
    Rect::new(bounds.x0 as f32, bounds.y0 as f32, (bounds.x1 - bounds.x0) as f32, (bounds.y1 - bounds.y0) as f32)
}

// The family each face is registered under, and what it is asked for with.
const FAMILIES: [(FontFamily, &str, f32, FontStyle); 8] = [
    (FontFamily::Monospace, "limn-mono", 400.0, FontStyle::Normal),
    (FontFamily::MonospaceBold, "limn-mono-bold", 700.0, FontStyle::Normal),
    (FontFamily::Sans, "limn-sans", 400.0, FontStyle::Normal),
    (FontFamily::SansItalic, "limn-sans-italic", 400.0, FontStyle::Italic),
    (FontFamily::SansMedium, "limn-sans-medium", 500.0, FontStyle::Normal),
    (FontFamily::SansBold, "limn-sans-bold", 700.0, FontStyle::Normal),
    (FontFamily::SansBoldItalic, "limn-sans-bold-italic", 700.0, FontStyle::Italic),
    (FontFamily::Display, "limn-display", 400.0, FontStyle::Normal),
];

fn slot(family: FontFamily) -> usize {
    FAMILIES.iter().position(|(given, ..)| *given == family).expect("every family has a slot")
}

/// The vertical measures of a family at a size.
#[derive(Clone, Copy)]
struct Metrics {
    ascent: f32,
    descent: f32,
    /// Its ascent, descent and line gap: the height of a line of it.
    natural: f32,
}

impl Metrics {
    /// The baseline of a box `height` tall, from its top, that centers it
    /// where the text's lines center: the middle of the glyphs' box.
    fn centered(&self, height: f32) -> f32 {
        height / 2.0 + (self.ascent - self.descent) / 2.0
    }
}

/// What one thread lays text out with, rebuilt when the fonts change.
struct System {
    generation: u64,
    given: Arc<Fonts>,
    fonts: FontContext,
    layouts: LayoutContext<Brush>,
    /// For each family, the faces tried in order.
    stacks: [Vec<FontFamilyName<'static>>; FAMILIES.len()],
    /// A family's ascent, descent and natural line height at a size, by its bits.
    naturals: HashMap<(u32, usize), Metrics>,
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
                FontFamily::Monospace | FontFamily::MonospaceBold => {
                    // Without a face of its own, a bold monospace is the regular one.
                    if family == FontFamily::MonospaceBold {
                        names.push(named("limn-mono"));
                    }
                    names.push(named("limn-sans"));
                    names.extend(fallbacks.iter().map(|name| named(name)));
                }
                _ => {
                    // Without a face of its own, a bold italic is the bold face slanted.
                    // Without a face of its own, a bold italic is the bold face slanted, and a display the bold face.
                    if matches!(family, FontFamily::SansBoldItalic | FontFamily::Display) {
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
            naturals: HashMap::new(),
        }
    }

    /// A family's own line height at `size`: its ascent, descent and gap.
    fn natural(&mut self, size: f32, family: FontFamily) -> f32 {
        self.metrics(size, family).natural
    }

    fn metrics(&mut self, size: f32, family: FontFamily) -> Metrics {
        if let Some(metrics) = self.naturals.get(&(size.to_bits(), slot(family))) {
            return *metrics;
        }
        let style = TextStyle::new(size, family);
        let mut layout = self.shape("x", &style, &[]);
        place(&mut layout, None, Alignment::Start, &[]);
        let font = layout.lines().next().and_then(|line| {
            line.runs().next().map(|run| (run.font_metrics().ascent, run.font_metrics().descent))
        });
        let (ascent, descent) = font.unwrap_or((size * 0.8, size * 0.2));
        let metrics = Metrics { ascent, descent, natural: layout.height() };
        self.naturals.insert((size.to_bits(), slot(family)), metrics);
        metrics
    }

    fn shape(&mut self, text: &str, style: &TextStyle, spans: &[Span]) -> parley::Layout<Brush> {
        // A span's line is the paragraph's height, or its own when taller.
        let measures: Vec<Metrics> = spans.iter().map(|span| self.metrics(span.size, span.family)).collect();
        let heights: Vec<Option<f32>> = measures
            .iter()
            .map(|metrics| style.line_height.map(|least| least.max(metrics.natural)))
            .collect();
        // Every line holds the default style's box (CSS's strut). Set in the
        // first drawn span's face, it stands inside that span's own box, so
        // the paragraph gives the lines their least height and nothing more.
        let (size, family) = spans
            .iter()
            .find(|span| span.image.is_none())
            .map_or((style.size, style.family), |span| (span.size, span.family));
        let (_, _, weight, slant) = FAMILIES[slot(family)];
        let mut builder = self.layouts.ranged_builder(&mut self.fonts, text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(parley::FontFamily::List(Cow::Borrowed(
            &self.stacks[slot(family)],
        ))));
        builder.push_default(StyleProperty::FontSize(size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
        builder.push_default(StyleProperty::FontStyle(slant));
        // A word that cannot fit a line is broken, as limn has always broken it.
        builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
        builder.push_default(StyleProperty::LineHeight(match style.line_height {
            Some(height) => LineHeight::Absolute(height),
            None => LineHeight::MetricsRelative(1.0),
        }));
        for (index, span) in spans.iter().enumerate() {
            let range = span.range.clone();
            let (_, _, weight, slant) = FAMILIES[slot(span.family)];
            builder.push(
                StyleProperty::FontFamily(parley::FontFamily::List(Cow::Borrowed(&self.stacks[slot(span.family)]))),
                range.clone(),
            );
            // A picture's text is read and copied, never drawn: it takes no room.
            builder.push(StyleProperty::FontSize(if span.image.is_some() { 0.0 } else { span.size }), range.clone());
            builder.push(StyleProperty::FontWeight(FontWeight::new(weight)), range.clone());
            builder.push(StyleProperty::FontStyle(slant), range.clone());
            builder.push(StyleProperty::Brush(Some(index as u32)), range.clone());
            if span.image.is_some() {
                // Its unseen text gives the line no height; the picture does.
                builder.push(StyleProperty::LineHeight(LineHeight::Absolute(0.0)), range.clone());
            } else if let Some(height) = heights[index] {
                builder.push(StyleProperty::LineHeight(LineHeight::Absolute(height)), range.clone());
            }
            if span.whole {
                builder.push(StyleProperty::TextWrapMode(TextWrapMode::NoWrap), range.clone());
            }
            let id = (index as u64) << 2;
            // An icon or a picture is centered where the text's lines center,
            // so its line is as tall as the taller of the two, as before.
            let metrics = measures[index];
            let inline = |id: u64, at: usize, width: f32, height: f32| InlineBox {
                id,
                kind: InlineBoxKind::InFlow,
                index: at,
                width,
                height,
                baseline: Some(metrics.centered(height)),
                vertical_align: VerticalAlign::BASELINE,
            };
            if span.icon.is_some() {
                // The icon and the gap after it, before the text; `place`
                // keeps it on the line its text starts on.
                builder.push_inline_box(inline(id | ICON_BOX, range.start, ICON_SIZE + 4.0, ICON_SIZE));
            }
            if let Some(side) = span.image.as_ref().map(|_| span.size * crate::INLINE_IMAGE_SCALE) {
                builder.push_inline_box(inline(id | PICTURE_BOX, range.start, side, side));
            }
            if span.background.is_some() {
                builder.push_inline_box(inline(id | PAD_BOX, range.start, BACKGROUND_INSET, 0.0));
                builder.push_inline_box(inline(id | PAD_BOX, range.end, BACKGROUND_INSET, 0.0));
            }
        }
        builder.build(text)
    }
}
