//! Color pictures painted over the glyphs of emoji in text; see [`set_color_emoji`].

use eframe::egui::{self, Color32, FontId, Pos2, Rect};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// A host's color emoji artwork: given one extended grapheme cluster of the
/// text being painted, the PNG to draw in its place, or `None` to keep the
/// font's glyph. It decides what counts as an emoji: a skin tone, a joined
/// sequence, a flag or a keycap arrives as the one cluster it forms, with any
/// variation selector it carries.
pub type ColorEmoji = fn(&str) -> Option<&'static [u8]>;

thread_local! {
    static SOURCE: Cell<Option<ColorEmoji>> = const { Cell::new(None) };
    // Textures belong to an egui context on this UI thread, like icons.
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

/// Textures kept at once: a picker's worth at a few sizes, about 6 KiB each
/// at the sizes text is drawn.
const ENTRIES: usize = 1024;
/// Pictures decoded in one paint pass; the rest keep their glyph until the
/// next pass, so opening a full picker spreads its cost over a few frames.
pub(crate) const DECODES_PER_PASS: usize = 24;
const MAX_SOURCE_BYTES: usize = 256 * 1024;
const MAX_SOURCE_SIDE: u32 = 512;
/// Pictures are drawn at most this many ems wide, and at least one.
const MAX_EMS: f32 = 1.2;

/// One picture at one size in physical pixels, keyed by where its bytes live.
type Key = (usize, usize, u32);

#[derive(Default)]
struct Cache {
    context: Option<egui::Context>,
    textures: HashMap<Key, Option<egui::TextureHandle>>,
    order: VecDeque<Key>,
    pass: u64,
    decoded: usize,
}

/// Install or remove this UI thread's color emoji artwork.
///
/// Text keeps its layout, width, selection and copied characters: the font's
/// glyph still takes its place in the line and is painted transparent under
/// the picture, which is fitted to that place and centered on the line. A
/// cluster the source does not draw, or whose picture is still to be decoded
/// or is not a PNG of at most 512 pixels a side and 256 KiB, keeps its glyph.
/// Text that is all ASCII is not asked about.
///
/// Replacing or removing the source clears the cached pictures and requests
/// a repaint.
pub fn set_color_emoji(source: Option<ColorEmoji>) {
    SOURCE.with(|cell| cell.set(source));
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.textures.clear();
        cache.order.clear();
        if let Some(ctx) = &cache.context {
            ctx.request_repaint();
        }
    });
}

/// The byte ranges of `text`'s clusters that `source` draws, with their PNGs.
pub(crate) fn runs(text: &str, source: ColorEmoji) -> Vec<(Range<usize>, &'static [u8])> {
    if text.is_ascii() {
        return Vec::new();
    }
    text.grapheme_indices(true)
        .filter(|(_, cluster)| !cluster.is_ascii())
        .filter_map(|(at, cluster)| source(cluster).map(|png| (at..at + cluster.len(), png)))
        .collect()
}

fn decode(png: &[u8], pixels: u32) -> Option<egui::ColorImage> {
    if png.is_empty() || png.len() > MAX_SOURCE_BYTES {
        return None;
    }
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(png), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_SOURCE_SIDE);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let side = decoded.width().max(decoded.height());
    // Downscaled once per size: egui samples without mipmaps.
    let decoded = if pixels < side {
        decoded.thumbnail(pixels, pixels)
    } else {
        decoded
    };
    let rgba = decoded.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

/// The texture of `png` at `pixels` a side, if it is ready or this pass may
/// still decode it.
fn texture(ctx: &egui::Context, png: &'static [u8], pixels: u32) -> Option<egui::TextureId> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.context.as_ref() != Some(ctx) {
            *cache = Cache {
                context: Some(ctx.clone()),
                ..Cache::default()
            };
        }
        let key = (png.as_ptr() as usize, png.len(), pixels);
        if let Some(entry) = cache.textures.get(&key) {
            return entry.as_ref().map(egui::TextureHandle::id);
        }
        let pass = ctx.cumulative_pass_nr();
        if cache.pass != pass {
            cache.pass = pass;
            cache.decoded = 0;
        }
        if cache.decoded >= DECODES_PER_PASS {
            ctx.request_repaint();
            return None;
        }
        cache.decoded += 1;
        let handle = decode(png, pixels).map(|image| {
            ctx.load_texture(
                format!("color-emoji:{:x}:{pixels}", key.0),
                image,
                egui::TextureOptions::LINEAR,
            )
        });
        let id = handle.as_ref().map(egui::TextureHandle::id);
        cache.textures.insert(key, handle);
        cache.order.push_back(key);
        while cache.order.len() > ENTRIES {
            if let Some(oldest) = cache.order.pop_front() {
                cache.textures.remove(&oldest);
            }
        }
        id
    })
}

/// Where a picture goes over the glyphs between `start` and `end`: the room
/// they take in the line, with its width fitted between one em and
/// [`MAX_EMS`], centered on the line.
pub(crate) fn picture_rect(start: Rect, end: Rect, font_size: f32) -> Rect {
    let room = (end.min.x - start.min.x).max(0.0);
    let side = room.clamp(font_size, font_size * MAX_EMS);
    Rect::from_center_size(
        Pos2::new((start.min.x + end.min.x) / 2.0, start.center().y),
        egui::vec2(side, side),
    )
}

/// Paint `text` from its top-left corner `at` as `Painter::text` would, with
/// the clusters the installed source draws as pictures. False, having
/// painted nothing, when there is nothing to draw: the caller paints the text.
pub(crate) fn paint(
    painter: &egui::Painter,
    at: Pos2,
    text: &str,
    font: &FontId,
    italics: bool,
    ink: Color32,
) -> bool {
    let Some(source) = SOURCE.with(Cell::get) else {
        return false;
    };
    let runs = runs(text, source);
    if runs.is_empty() {
        return false;
    }
    let ctx = painter.ctx();
    let side = font.size * MAX_EMS * ctx.pixels_per_point();
    // Sizes in steps of 4 pixels share textures.
    let pixels = (side.ceil().clamp(4.0, MAX_SOURCE_SIDE as f32) as u32).div_ceil(4) * 4;
    let ready: Vec<(Range<usize>, egui::TextureId)> = runs
        .into_iter()
        .filter_map(|(range, png)| texture(ctx, png, pixels).map(|id| (range, id)))
        .collect();
    if ready.is_empty() {
        return false;
    }
    let format = egui::TextFormat {
        font_id: font.clone(),
        color: ink,
        italics,
        ..Default::default()
    };
    let hidden = egui::TextFormat {
        color: Color32::TRANSPARENT,
        ..format.clone()
    };
    let mut job = egui::text::LayoutJob::default();
    let mut from = 0;
    for (range, _) in &ready {
        if from < range.start {
            job.append(&text[from..range.start], 0.0, format.clone());
        }
        job.append(&text[range.clone()], 0.0, hidden.clone());
        from = range.end;
    }
    if from < text.len() {
        job.append(&text[from..], 0.0, format);
    }
    let galley = painter.layout_job(job);
    let tint = Color32::from_white_alpha(ink.a());
    let whole = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    for (range, id) in &ready {
        let start = text[..range.start].chars().count();
        let end = start + text[range.clone()].chars().count();
        let cursor = |index| galley.pos_from_cursor(egui::text::CCursor::new(index));
        let place = picture_rect(cursor(start), cursor(end), font.size).translate(at.to_vec2());
        painter.image(*id, place, whole, tint);
    }
    painter.galley(at, galley, ink);
    true
}
