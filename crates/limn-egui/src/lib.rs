//! The egui and eframe backend of limn.
//!
//! [`paint`] draws a `Frame` of draw commands with an egui painter and publishes the frame's
//! controls to the platform's accessibility tree through AccessKit. [`events`] turns egui's
//! input into `UiEvent`s and [`apply_access_requests`] applies what a screen reader asked for.
//! The rest is fonts, icon packs, the image cache and externally rendered surfaces.
#![warn(missing_docs)]
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect as EguiRect, Stroke, StrokeKind,
};
use limn::{
    Color, DrawCommand, FontFamily, Frame, Icon, Key, Point, RasterImage, RasterImageFormat, Rect,
    SpinnerVariant, UiEvent,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;

mod glass;
pub use glass::GlassRenderer;

/// Colors of the effort slider, the only command without colors of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffortPalette {
    /// The track behind the filled part. Default `rgb(43, 43, 43)`.
    pub input: Color,
    /// The step dots to the right of the filled part. Default `rgb(136, 136, 136)`.
    pub foreground_subtle: Color,
    /// The filled part of the track, and the left end of its gradient. Default
    /// `rgb(80, 145, 255)`.
    pub blue: Color,
    /// The right end of the gradient, which is drawn over the filled part with the command's
    /// `ultra_mix` as its opacity. Default
    /// `rgb(190, 126, 255)`.
    pub ultra: Color,
}

impl Default for EffortPalette {
    fn default() -> Self {
        Self {
            input: Color::rgb(43, 43, 43),
            foreground_subtle: Color::rgb(136, 136, 136),
            blue: Color::rgb(80, 145, 255),
            ultra: Color::rgb(190, 126, 255),
        }
    }
}

thread_local! {
    static EFFORT_PALETTE: std::cell::Cell<EffortPalette> = std::cell::Cell::new(EffortPalette::default());
}

/// Set once by the host on the UI thread to match its theme.
pub fn set_effort_palette(palette: EffortPalette) {
    EFFORT_PALETTE.with(|cell| cell.set(palette));
}

fn effort_palette() -> EffortPalette {
    EFFORT_PALETTE.with(std::cell::Cell::get)
}

/// Whether the person asked their system for less motion, where that can be
/// asked. Give it to [`UiTree::set_reduced_motion`](limn::UiTree::set_reduced_motion)
/// at launch, with your own setting over it:
///
/// ```no_run
/// # let mut tree = limn::UiTree::new(limn::Element::spacer());
/// # let setting: Option<bool> = None;
/// tree.set_reduced_motion(setting.or(limn_egui::prefers_reduced_motion()).unwrap_or(false));
/// ```
///
/// In a browser it is the `prefers-reduced-motion` media query. On the
/// desktop it is `None`: nothing here reads the platform's setting yet.
pub fn prefers_reduced_motion() -> Option<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        let query = eframe::web_sys::window()?.match_media("(prefers-reduced-motion: reduce)").ok()??;
        Some(query.matches())
    }
    #[cfg(not(target_arch = "wasm32"))]
    None
}

/// Tell the platform what the window is called. egui gives a screen reader
/// a window with no name; call this each frame with the window's title.
pub fn name_window(ctx: &egui::Context, name: &str) {
    ctx.accesskit_node_builder(egui::accesskit_root_id(), |window| window.set_label(name));
}

/// Paint again when a hot patch lands, so an idle window shows the new code
/// at once. Call it once, when the window is created. It does nothing in a
/// build without the `hot` feature; see [`limn::hot`].
pub fn repaint_on_patch(ctx: &egui::Context) {
    let ctx = ctx.clone();
    limn::hot::on_patch(move || ctx.request_repaint());
}

/// The artwork of another icon set: the source of an SVG drawn with
/// `currentColor` for each icon it has, and `None` for the ones it leaves to
/// Bootstrap Icons.
pub type IconPack = fn(Icon) -> Option<&'static str>;

thread_local! {
    static ICON_PACK: std::cell::Cell<Option<IconPack>> = const { std::cell::Cell::new(None) };
}

/// Draw the semantic icons from another set. Bootstrap Icons stay for
/// whatever the pack does not have, so a pack can replace a few or all.
/// Artwork of any proportion is fitted to the icon's square. File type
/// icons and `Icon::Custom` are not asked for.
///
/// Call it once on the UI thread when the window is created, before the
/// first frame: an icon already drawn stays as it was cached.
pub fn set_icon_pack(pack: IconPack) {
    ICON_PACK.with(|cell| cell.set(Some(pack)));
}

fn packed_svg(icon: Icon) -> Option<&'static str> {
    if matches!(icon, Icon::FileType(_) | Icon::Custom(_)) {
        return None;
    }
    ICON_PACK.with(std::cell::Cell::get).and_then(|pack| pack(icon))
}

const MAX_IMAGE_SOURCE_BYTES: usize = 8 * 1024 * 1024;

#[path = "corner_mask_render.rs"]
mod corner_mask_render;

#[path = "selection_contour_render.rs"]
mod selection_contour_render;

#[path = "keyboard_input.rs"]
mod keyboard_input;
pub use keyboard_input::normalize_keyboard_input;


#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct IconKey {
    icon: Icon,
    pixels: u16,
}

#[derive(Default)]
struct IconCache {
    context: Option<egui::Context>,
    textures: HashMap<IconKey, egui::TextureHandle>,
    order: VecDeque<IconKey>,
}

const ICON_TEXTURE_CACHE_ENTRIES: usize = 512;

thread_local! {
    // Textures belong to an egui context on this UI thread. Keying the
    // rasterization keeps SVG parsing out of subsequent paint passes.
    static ICON_CACHE: RefCell<IconCache> = RefCell::new(IconCache::default());
    static RASTER_CACHE: RefCell<RasterCache> = RefCell::new(RasterCache::default());
}

const MAX_IMAGE_SOURCE_SIDE: u32 = 4096;
const MAX_IMAGE_SOURCE_PIXELS: u64 = 12 * 1024 * 1024;
const MAX_IMAGE_DECODE_ALLOCATION: u64 = 64 * 1024 * 1024;
const IMAGE_THUMBNAIL_SIDE: u32 = 384;
const IMAGE_TEXTURE_CACHE_ENTRIES: usize = 16;
const FULL_IMAGE_TEXTURE_CACHE_ENTRIES: usize = 2;

thread_local! {
    static IMAGE_CACHE_ENTRIES: std::cell::Cell<usize> =
        const { std::cell::Cell::new(IMAGE_TEXTURE_CACHE_ENTRIES) };
}

/// How many decoded images stay on the GPU at once. The default suits a
/// conversation with a few attachments; a host showing a grid of thumbnails
/// raises it so scrolling does not decode the same images again and again.
pub fn set_image_cache_entries(entries: usize) {
    IMAGE_CACHE_ENTRIES.with(|cell| cell.set(entries.clamp(1, 1024)));
}

enum RasterCacheEntry {
    Decoding(std::sync::mpsc::Receiver<Option<egui::ColorImage>>),
    Ready(egui::TextureHandle),
    Unavailable,
}

/// What a paint pass has of an image.
#[derive(Debug, PartialEq)]
enum RasterTexture {
    Ready(egui::TextureId, [usize; 2]),
    /// Still being decoded; a repaint is asked for when it lands.
    Decoding,
    Unavailable,
}

#[derive(Default)]
struct RasterCache {
    context: Option<egui::Context>,
    entries: HashMap<(String, RasterImageFormat, bool), RasterCacheEntry>,
    order: VecDeque<(String, RasterImageFormat, bool)>,
}

fn raster_format(format: RasterImageFormat) -> image::ImageFormat {
    match format {
        RasterImageFormat::Png => image::ImageFormat::Png,
        RasterImageFormat::Jpeg => image::ImageFormat::Jpeg,
        RasterImageFormat::Webp => image::ImageFormat::WebP,
        RasterImageFormat::Gif => image::ImageFormat::Gif,
    }
}

/// Only this path may turn journal image bytes into pixels. It verifies the
/// digest and declared format again at the render boundary, then checks
/// dimensions before asking a decoder to allocate the full image.
fn decode_raster_thumbnail(source: &RasterImage) -> Option<egui::ColorImage> {
    use base64::Engine as _;
    use sha2::Digest as _;
    let size = source.size_bytes() as usize;
    if size == 0 || size > MAX_IMAGE_SOURCE_BYTES {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.data_base64())
        .ok()?;
    if bytes.len() != size || format!("{:x}", sha2::Sha256::digest(&bytes)) != source.sha256() {
        return None;
    }
    let format = raster_format(source.format());
    if image::guess_format(&bytes).ok()? != format {
        return None;
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_SOURCE_SIDE);
    limits.max_image_height = Some(MAX_IMAGE_SOURCE_SIDE);
    limits.max_alloc = Some(MAX_IMAGE_DECODE_ALLOCATION);
    let mut dimensions_reader =
        image::ImageReader::with_format(Cursor::new(bytes.as_slice()), format);
    dimensions_reader.limits(limits.clone());
    let (width, height) = dimensions_reader.into_dimensions().ok()?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_IMAGE_SOURCE_PIXELS {
        return None;
    }
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes.as_slice()), format);
    reader.limits(limits);
    let decoded = reader.decode().ok()?;
    let thumbnail = if source.is_full_resolution()
        || (width <= IMAGE_THUMBNAIL_SIDE && height <= IMAGE_THUMBNAIL_SIDE)
    {
        decoded.to_rgba8()
    } else {
        decoded
            .thumbnail(IMAGE_THUMBNAIL_SIDE, IMAGE_THUMBNAIL_SIDE)
            .to_rgba8()
    };
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [thumbnail.width() as usize, thumbnail.height() as usize],
        thumbnail.as_raw(),
    ))
}

fn raster_cache_key(image: &RasterImage) -> (String, RasterImageFormat, bool) {
    (
        image.sha256().to_owned(),
        image.format(),
        image.is_full_resolution(),
    )
}

fn raster_texture(ui: &egui::Ui, image: &RasterImage) -> RasterTexture {
    RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.context.as_ref() != Some(ui.ctx()) {
            cache.entries.clear();
            cache.order.clear();
            cache.context = Some(ui.ctx().clone());
        }
        let key = raster_cache_key(image);
        if let Some(entry) = cache.entries.get_mut(&key) {
            if let RasterCacheEntry::Decoding(decoded) = entry {
                match decoded.try_recv() {
                    Ok(Some(bitmap)) => {
                        *entry = RasterCacheEntry::Ready(ui.ctx().load_texture(
                            format!("user-image:{}", image.sha256()),
                            bitmap,
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    Ok(None) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        *entry = RasterCacheEntry::Unavailable;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            }
            return match entry {
                RasterCacheEntry::Ready(texture) => {
                    RasterTexture::Ready(texture.id(), texture.size())
                }
                RasterCacheEntry::Decoding(_) => RasterTexture::Decoding,
                RasterCacheEntry::Unavailable => RasterTexture::Unavailable,
            };
        }
        // Decoding costs milliseconds an image: on the UI thread, a row of
        // pictures scrolling into view drops a frame for each of them.
        let (landed, decoded) = std::sync::mpsc::channel();
        let (source, ctx) = (image.clone(), ui.ctx().clone());
        let decode = move || {
            let _ = landed.send(decode_raster_thumbnail(&source));
            ctx.request_repaint();
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(decode);
        // A browser has no thread to give; the image lands on the next frame.
        #[cfg(target_arch = "wasm32")]
        decode();
        cache
            .entries
            .insert(key.clone(), RasterCacheEntry::Decoding(decoded));
        cache.order.push_back(key);
        let full = image.is_full_resolution();
        let limit = if full {
            FULL_IMAGE_TEXTURE_CACHE_ENTRIES
        } else {
            IMAGE_CACHE_ENTRIES.with(std::cell::Cell::get)
        };
        while cache.order.iter().filter(|key| key.2 == full).count() > limit {
            if let Some(index) = cache.order.iter().position(|key| key.2 == full) {
                let oldest = cache.order.remove(index).expect("cache entry index");
                cache.entries.remove(&oldest);
            }
        }
        RasterTexture::Decoding
    })
}

const MAX_SURFACE_SIDE: usize = 8192;
const MAX_SURFACE_PIXELS: usize = 16 * 1024 * 1024;
const SURFACE_TEXTURE: egui::TextureOptions = egui::TextureOptions::LINEAR;

/// One frame of a [`DrawCommand::Surface`], or the part of one that changed.
/// Build it where the pixels are decoded, away from the UI thread: converting
/// them is the costly part.
pub struct SurfaceFrame {
    image: egui::ColorImage,
    pixels_per_point: f32,
    /// The size of the frame these pixels are a part of and where they go in
    /// it, when they are not all of it.
    part: Option<([usize; 2], [usize; 2])>,
}

impl std::fmt::Debug for SurfaceFrame {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SurfaceFrame")
            .field("size", &self.image.size)
            .field("pixels_per_point", &self.pixels_per_point)
            .field("part", &self.part)
            .finish()
    }
}

impl SurfaceFrame {
    /// Rows of straight-alpha RGBA, top to bottom. `pixels_per_point` is the
    /// display scale the pixels were rendered for.
    pub fn from_rgba(size: [usize; 2], rgba: &[u8], pixels_per_point: f32) -> Option<Self> {
        Self::bounded(size, rgba.len(), 4, pixels_per_point).then(|| Self {
            image: egui::ColorImage::from_rgba_unmultiplied(size, rgba),
            pixels_per_point,
            part: None,
        })
    }

    /// Rows of opaque RGB, as a JPEG decodes: no alpha channel is carried.
    pub fn from_rgb(size: [usize; 2], rgb: &[u8], pixels_per_point: f32) -> Option<Self> {
        Self::bounded(size, rgb.len(), 3, pixels_per_point).then(|| Self {
            image: egui::ColorImage::from_rgb(size, rgb),
            pixels_per_point,
            part: None,
        })
    }

    fn bounded(size: [usize; 2], bytes: usize, channels: usize, pixels_per_point: f32) -> bool {
        let [width, height] = size;
        (1..=MAX_SURFACE_SIDE).contains(&width)
            && (1..=MAX_SURFACE_SIDE).contains(&height)
            && width * height <= MAX_SURFACE_PIXELS
            && bytes == width * height * channels
            && pixels_per_point.is_finite()
            && pixels_per_point > 0.0
    }

    /// Width and height in pixels.
    pub fn size(&self) -> [usize; 2] {
        self.image.size
    }

    /// Makes these pixels the rectangle at `at` of a frame of `frame` pixels
    /// that a surface already shows: presenting them writes only that
    /// rectangle of its texture. `None` when they do not fit in such a frame.
    pub fn within(mut self, frame: [usize; 2], at: [usize; 2]) -> Option<Self> {
        let fits = (0..2).all(|axis| at[axis].checked_add(self.image.size[axis]).is_some_and(|end| end <= frame[axis]));
        self.part = Some((frame, at));
        fits.then_some(self)
    }

    /// The size of the frame these pixels are a part of and where they go in
    /// it. `None` for a whole frame.
    pub fn part(&self) -> Option<([usize; 2], [usize; 2])> {
        self.part
    }
}

struct SurfaceTexture {
    texture: egui::TextureHandle,
    pixels_per_point: f32,
}

#[derive(Default)]
struct SurfaceCache {
    context: Option<egui::Context>,
    entries: HashMap<String, SurfaceTexture>,
}

thread_local! {
    static SURFACE_CACHE: RefCell<SurfaceCache> = RefCell::new(SurfaceCache::default());
}

/// Show `frame` wherever a [`DrawCommand::Surface`] names `id`. Called on the
/// UI thread; a frame of the size already shown is written into the texture
/// the GPU has, so a stream of frames allocates nothing, and a part of one
/// ([`SurfaceFrame::within`]) uploads only its own pixels. The surface keeps
/// its last frame until [`forget_surface`]. A frame wider or taller than the
/// GPU's textures is refused, and so is a part of any frame but the one the
/// surface shows, at its size and scale: `false` says so, and the surface is
/// left as it was. The host makes up for a refused part with a whole frame.
pub fn present_surface(ctx: &egui::Context, id: &str, frame: SurfaceFrame) -> bool {
    let side = ctx.input(|input| input.max_texture_side);
    if frame.image.size.iter().any(|pixels| *pixels > side) {
        return false;
    }
    SURFACE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.context.as_ref() != Some(ctx) {
            cache.entries.clear();
            cache.context = Some(ctx.clone());
        }
        let SurfaceFrame { image, pixels_per_point, part } = frame;
        match (cache.entries.get_mut(id), part) {
            (Some(entry), Some((frame, at))) => {
                if entry.texture.size() != frame || entry.pixels_per_point != pixels_per_point {
                    return false;
                }
                entry.texture.set_partial(at, image, SURFACE_TEXTURE);
            }
            // There is no frame for it to be a part of.
            (None, Some(_)) => return false,
            (Some(entry), None) => {
                if entry.texture.size() == image.size {
                    entry.texture.set_partial([0, 0], image, SURFACE_TEXTURE);
                } else {
                    entry.texture.set(image, SURFACE_TEXTURE);
                }
                entry.pixels_per_point = pixels_per_point;
            }
            (None, None) => {
                let texture = ctx.load_texture(format!("surface:{id}"), image, SURFACE_TEXTURE);
                cache.entries.insert(id.to_owned(), SurfaceTexture { texture, pixels_per_point });
            }
        }
        true
    })
}

/// Free the texture of a surface the host no longer shows.
pub fn forget_surface(id: &str) {
    SURFACE_CACHE.with(|cache| cache.borrow_mut().entries.remove(id));
}

fn surface_texture(ui: &egui::Ui, id: &str) -> Option<(egui::TextureId, [usize; 2], f32)> {
    SURFACE_CACHE.with(|cache| {
        let cache = cache.borrow();
        let entry = cache.entries.get(id).filter(|_| cache.context.as_ref() == Some(ui.ctx()))?;
        Some((entry.texture.id(), entry.texture.size(), entry.pixels_per_point))
    })
}

/// Where a surface's pixels meet the screen: from the area's top-left corner,
/// snapped to the pixel grid, at the density they were rendered for. On the
/// display they were rendered for each one covers exactly one physical pixel,
/// whatever the area measures; on another they keep their logical size.
fn surface_rect(area: EguiRect, pixels: [usize; 2], density: f32, pixels_per_point: f32) -> EguiRect {
    let snap = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
    // A scale that crossed a process boundary as text is still this display.
    let density = if (density - pixels_per_point).abs() < 0.01 {
        pixels_per_point
    } else {
        density
    };
    EguiRect::from_min_size(
        Pos2::new(snap(area.min.x), snap(area.min.y)),
        egui::vec2(pixels[0] as f32 / density, pixels[1] as f32 / density),
    )
}

fn fit_image(area: EguiRect, pixels: [usize; 2]) -> EguiRect {
    let width = pixels[0] as f32;
    let height = pixels[1] as f32;
    let scale = (area.width() / width).min(area.height() / height).min(1.0);
    let size = egui::vec2(width * scale, height * scale);
    EguiRect::from_center_size(area.center(), size)
}

/// The centered part of an image that fills `area` without distortion, as
/// texture coordinates.
fn cover_uv(area: EguiRect, pixels: [usize; 2]) -> EguiRect {
    let image = pixels[0] as f32 / pixels[1] as f32;
    let wanted = area.width() / area.height();
    let shown = if !(image.is_finite() && wanted.is_finite()) {
        egui::vec2(1.0, 1.0)
    } else if image > wanted {
        egui::vec2(wanted / image, 1.0)
    } else {
        egui::vec2(1.0, image / wanted)
    };
    EguiRect::from_center_size(Pos2::new(0.5, 0.5), shown)
}

fn icon_svg(icon: Icon) -> &'static str {
    match icon {
        Icon::PanelLeft => include_str!("../assets/bootstrap/layout-sidebar.svg"),
        Icon::PanelLeftOpen => include_str!("../assets/bootstrap/layout-sidebar-inset.svg"),
        Icon::PanelLeftClose => include_str!("../assets/bootstrap/layout-sidebar-inset.svg"),
        Icon::PanelBottom => include_str!("../assets/bootstrap/window-dock.svg"),
        Icon::PanelRight => include_str!("../assets/bootstrap/layout-sidebar-reverse.svg"),
        Icon::PanelRightOpen => include_str!("../assets/bootstrap/layout-sidebar-inset-reverse.svg"),
        Icon::PanelRightClose => include_str!("../assets/bootstrap/layout-sidebar-inset-reverse.svg"),
        Icon::Add => include_str!("../assets/bootstrap/plus-lg.svg"),
        Icon::Minus => include_str!("../assets/bootstrap/dash.svg"),
        Icon::Microphone => include_str!("../assets/bootstrap/mic.svg"),
        Icon::Text => include_str!("../assets/bootstrap/card-text.svg"),
        Icon::Image => include_str!("../assets/bootstrap/image.svg"),
        Icon::Audio => include_str!("../assets/bootstrap/soundwave.svg"),
        Icon::Video => include_str!("../assets/bootstrap/camera-video.svg"),
        Icon::File => include_str!("../assets/bootstrap/file-earmark-text.svg"),
        Icon::FileType(file) => file.svg(),
        Icon::Infinity => include_str!("../assets/bootstrap/infinity.svg"),
        Icon::Paperclip => include_str!("../assets/bootstrap/paperclip.svg"),
        Icon::Target => include_str!("../assets/bootstrap/bullseye.svg"),
        Icon::Plan => include_str!("../assets/bootstrap/list-check.svg"),
        Icon::Brain => include_str!("../assets/bootstrap/lightbulb.svg"),
        Icon::Check => include_str!("../assets/bootstrap/check-lg.svg"),
        Icon::InfoCircle => include_str!("../assets/bootstrap/info-circle.svg"),
        Icon::CheckCircle => include_str!("../assets/bootstrap/check-circle.svg"),
        Icon::WarningTriangle => include_str!("../assets/bootstrap/exclamation-triangle.svg"),
        Icon::ErrorCircle => include_str!("../assets/bootstrap/x-circle.svg"),
        Icon::Settings => include_str!("../assets/bootstrap/gear.svg"),
        Icon::Github => include_str!("../assets/bootstrap/github.svg"),
        Icon::Custom(source) => source,
        Icon::Chat => include_str!("../assets/bootstrap/chat-dots.svg"),
        Icon::Folder => include_str!("../assets/bootstrap/folder.svg"),
        Icon::Laptop => include_str!("../assets/bootstrap/laptop.svg"),
        Icon::Globe => include_str!("../assets/bootstrap/globe.svg"),
        Icon::GitBranch => include_str!("../assets/bootstrap/git.svg"),
        Icon::Grid => include_str!("../assets/bootstrap/grid.svg"),
        Icon::Palette => include_str!("../assets/bootstrap/palette.svg"),
        Icon::Users => include_str!("../assets/bootstrap/people.svg"),
        Icon::Puzzle => include_str!("../assets/bootstrap/puzzle.svg"),
        Icon::Book => include_str!("../assets/bootstrap/book.svg"),
        Icon::Server => include_str!("../assets/bootstrap/hdd-stack.svg"),
        Icon::Bolt => include_str!("../assets/bootstrap/lightning-charge.svg"),
        Icon::ChartBar => include_str!("../assets/bootstrap/bar-chart.svg"),
        Icon::Keyboard => include_str!("../assets/bootstrap/keyboard.svg"),
        Icon::Hash => include_str!("../assets/bootstrap/hash.svg"),
        Icon::Code => include_str!("../assets/bootstrap/code-slash.svg"),
        Icon::Terminal => include_str!("../assets/bootstrap/terminal.svg"),
        Icon::ArrowUp => include_str!("../assets/bootstrap/arrow-up.svg"),
        Icon::Send => include_str!("../assets/bootstrap/arrow-up.svg"),
        Icon::ArrowDown => include_str!("../assets/bootstrap/arrow-down.svg"),
        Icon::ArrowLeft => include_str!("../assets/bootstrap/arrow-left.svg"),
        Icon::ArrowRight => include_str!("../assets/bootstrap/arrow-right.svg"),
        Icon::ChevronDown => include_str!("../assets/bootstrap/chevron-down.svg"),
        Icon::ChevronUp => include_str!("../assets/bootstrap/chevron-up.svg"),
        Icon::ChevronRight => include_str!("../assets/bootstrap/chevron-right.svg"),
        Icon::Search => include_str!("../assets/bootstrap/search.svg"),
        Icon::Stop => include_str!("../assets/bootstrap/stop.svg"),
        Icon::StopFill => include_str!("../assets/bootstrap/stop-fill.svg"),
        Icon::Play => include_str!("../assets/bootstrap/play.svg"),
        Icon::Pause => include_str!("../assets/bootstrap/pause.svg"),

        Icon::History => include_str!("../assets/bootstrap/clock-history.svg"),
        Icon::Close => include_str!("../assets/bootstrap/x-lg.svg"),
        Icon::CloseBold => include_str!("../assets/bootstrap/x.svg"),
        Icon::CloseCircle => include_str!("../assets/bootstrap/x-circle.svg"),
        Icon::Refresh => include_str!("../assets/bootstrap/arrow-clockwise.svg"),
        Icon::Menu => include_str!("../assets/bootstrap/list.svg"),
        Icon::More => include_str!("../assets/bootstrap/three-dots.svg"),
        Icon::Pin => include_str!("../assets/bootstrap/pin-angle.svg"),
        Icon::Archive => include_str!("../assets/bootstrap/archive.svg"),
        Icon::ArchiveUp => include_str!("../assets/bootstrap/box-arrow-up.svg"),
        Icon::Trash => include_str!("../assets/bootstrap/trash3.svg"),
        Icon::Edit => include_str!("../assets/bootstrap/pencil.svg"),
        Icon::Copy => include_str!("../assets/bootstrap/copy.svg"),
        Icon::Unread => include_str!("../assets/bootstrap/chat-left-dots.svg"),
        Icon::Filter => include_str!("../assets/bootstrap/funnel.svg"),
        Icon::Sort => include_str!("../assets/bootstrap/sort-down.svg"),
        Icon::Collapse => include_str!("../assets/bootstrap/arrows-angle-contract.svg"),
        Icon::Expand => include_str!("../assets/bootstrap/arrows-angle-expand.svg"),
        Icon::Compact => include_str!("../assets/bootstrap/arrows-collapse.svg"),
        Icon::Fullscreen => include_str!("../assets/bootstrap/fullscreen.svg"),
        Icon::Clock => include_str!("../assets/bootstrap/clock.svg"),
        Icon::Bell => include_str!("../assets/bootstrap/bell.svg"),
        Icon::BellFill => include_str!("../assets/bootstrap/bell-fill.svg"),
        Icon::BellSlash => include_str!("../assets/bootstrap/bell-slash.svg"),
        Icon::SkipBack => include_str!("../assets/bootstrap/skip-start-fill.svg"),
        Icon::SkipForward => include_str!("../assets/bootstrap/skip-end-fill.svg"),
        Icon::PauseFill => include_str!("../assets/bootstrap/pause-fill.svg"),
        Icon::Shuffle => include_str!("../assets/bootstrap/shuffle.svg"),
        Icon::Repeat => include_str!("../assets/bootstrap/repeat.svg"),
        Icon::RepeatOne => include_str!("../assets/bootstrap/repeat-1.svg"),
        Icon::Volume => include_str!("../assets/bootstrap/volume-up.svg"),
        Icon::VolumeMuted => include_str!("../assets/bootstrap/volume-mute.svg"),
        Icon::Heart => include_str!("../assets/bootstrap/heart.svg"),
        Icon::HeartFill => include_str!("../assets/bootstrap/heart-fill.svg"),
        Icon::MusicNote => include_str!("../assets/bootstrap/music-note-beamed.svg"),
        Icon::Playlist => include_str!("../assets/bootstrap/music-note-list.svg"),
        Icon::Home => include_str!("../assets/bootstrap/house.svg"),
        Icon::Library => include_str!("../assets/bootstrap/collection.svg"),
        Icon::Sliders => include_str!("../assets/bootstrap/sliders.svg"),
        Icon::MicrophoneMuted => include_str!("../assets/bootstrap/mic-mute.svg"),
        Icon::Headphones => include_str!("../assets/bootstrap/headphones.svg"),
        Icon::Emoji => include_str!("../assets/bootstrap/emoji-smile.svg"),
        Icon::Reply => include_str!("../assets/bootstrap/reply.svg"),
        Icon::At => include_str!("../assets/bootstrap/at.svg"),
        Icon::Phone => include_str!("../assets/bootstrap/telephone.svg"),
        Icon::HangUp => include_str!("../assets/bootstrap/telephone-x.svg"),
        Icon::Screen => include_str!("../assets/bootstrap/display.svg"),
        Icon::VideoOff => include_str!("../assets/bootstrap/camera-video-off.svg"),
        Icon::Lock => include_str!("../assets/bootstrap/lock-fill.svg"),
        Icon::Neutral => include_str!("../assets/bootstrap/slash-lg.svg"),
        Icon::Invite => include_str!("../assets/bootstrap/person-plus.svg"),
        Icon::Leave => include_str!("../assets/bootstrap/box-arrow-left.svg"),
        Icon::DragHandle => include_str!("../assets/bootstrap/grip-vertical.svg"),
        Icon::FolderFill => include_str!("../assets/bootstrap/folder-fill.svg"),
        Icon::PlayFill => include_str!("../assets/bootstrap/play-fill.svg"),
        Icon::GridFill => include_str!("../assets/bootstrap/grid-fill.svg"),
        Icon::SettingsFill => include_str!("../assets/bootstrap/gear-fill.svg"),
        Icon::PaletteFill => include_str!("../assets/bootstrap/palette-fill.svg"),
        Icon::LaptopFill => include_str!("../assets/bootstrap/laptop-fill.svg"),
        Icon::TerminalFill => include_str!("../assets/bootstrap/terminal-fill.svg"),
        Icon::KeyboardFill => include_str!("../assets/bootstrap/keyboard-fill.svg"),
        Icon::ChatFill => include_str!("../assets/bootstrap/chat-dots-fill.svg"),
        Icon::UsersFill => include_str!("../assets/bootstrap/people-fill.svg"),
        Icon::PuzzleFill => include_str!("../assets/bootstrap/puzzle-fill.svg"),
        Icon::ServerFill => include_str!("../assets/bootstrap/hdd-stack-fill.svg"),
        Icon::BookFill => include_str!("../assets/bootstrap/book-fill.svg"),
        Icon::ClockFill => include_str!("../assets/bootstrap/clock-fill.svg"),
        Icon::BoltFill => include_str!("../assets/bootstrap/lightning-charge-fill.svg"),
        Icon::ArchiveFill => include_str!("../assets/bootstrap/archive-fill.svg"),
        Icon::ChartBarFill => include_str!("../assets/bootstrap/bar-chart-fill.svg"),
    }
}

fn monochrome_svg(source: &str) -> String {
    // The icons draw with currentColor. Rasterize a white coverage mask
    // once per icon and physical size, then tint it per DrawCommand. This
    // keeps theme and hover colors out of the texture cache, like WarpUI's
    // monochrome Icon element does for its size-cached image.
    source.replace("currentColor", "#FFFFFF")
}

fn raster_icon(key: IconKey) -> Option<egui::ColorImage> {
    let packed = packed_svg(key.icon);
    let source = if matches!(key.icon, Icon::FileType(_)) {
        std::borrow::Cow::Borrowed(icon_svg(key.icon))
    } else {
        std::borrow::Cow::Owned(monochrome_svg(packed.unwrap_or_else(|| icon_svg(key.icon))))
    };
    let tree = resvg::usvg::Tree::from_str(&source, &resvg::usvg::Options::default()).ok()?;
    let pixels = u32::from(key.pixels);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels)?;
    let transform = if packed.is_some() || matches!(key.icon, Icon::Custom(_)) {
        // Application artwork and another set's come in any aspect ratio.
        // Contain it in the icon's square instead of stretching it.
        let scale = pixels as f32 / tree.size().width().max(tree.size().height());
        resvg::tiny_skia::Transform::from_row(
            scale,
            0.0,
            0.0,
            scale,
            (pixels as f32 - tree.size().width() * scale) / 2.0,
            (pixels as f32 - tree.size().height() * scale) / 2.0,
        )
    } else {
        resvg::tiny_skia::Transform::from_scale(
            pixels as f32 / tree.size().width(),
            pixels as f32 / tree.size().height(),
        )
    };
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(egui::ColorImage::from_rgba_premultiplied(
        [pixels as usize, pixels as usize],
        pixmap.data(),
    ))
}

fn icon_texture(ui: &egui::Ui, key: IconKey) -> Option<egui::TextureId> {
    ICON_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.context.as_ref() != Some(ui.ctx()) {
            cache.textures.clear();
            cache.order.clear();
            cache.context = Some(ui.ctx().clone());
        }
        if let Some(texture) = cache.textures.get(&key) {
            return Some(texture.id());
        }
        let image = raster_icon(key)?;
        let texture = ui.ctx().load_texture(
            format!("limn-icon:{:?}:{}", key.icon, key.pixels),
            image,
            egui::TextureOptions::NEAREST,
        );
        let id = texture.id();
        cache.textures.insert(key, texture);
        cache.order.push_back(key);
        while cache.order.len() > ICON_TEXTURE_CACHE_ENTRIES {
            if let Some(oldest) = cache.order.pop_front() {
                cache.textures.remove(&oldest);
            }
        }
        Some(id)
    })
}

fn icon_tint(icon: Icon, tint: Color) -> Color32 {
    if matches!(icon, Icon::FileType(_)) {
        // Colored file artwork keeps its palette; disabled and fade opacity
        // still follow the surrounding surface. UI glyphs remain tintable.
        Color32::from_white_alpha(tint.a)
    } else {
        color(tint)
    }
}

fn color(value: Color) -> Color32 {
    Color32::from_rgba_unmultiplied(value.r, value.g, value.b, value.a)
}

fn pos(origin: Pos2, value: Point) -> Pos2 {
    Pos2::new(origin.x + value.x, origin.y + value.y)
}

fn rect(origin: Pos2, value: Rect) -> EguiRect {
    EguiRect::from_min_size(
        Pos2::new(origin.x + value.x, origin.y + value.y),
        egui::vec2(value.width, value.height),
    )
}

fn spinner_arc_points(area: EguiRect, angle: f32, stroke_width: f32) -> Vec<Pos2> {
    let radius = (area.width().min(area.height()) / 2.0 - stroke_width / 2.0 - 1.0).max(0.0);
    let sweep = std::f32::consts::TAU * 5.0 / 6.0;
    let steps = ((radius * sweep / 1.4).ceil() as usize).clamp(16, 80);
    (0..=steps)
        .map(|step| {
            let theta = angle - std::f32::consts::FRAC_PI_2 + sweep * step as f32 / steps as f32;
            Pos2::new(
                area.center().x + radius * theta.cos(),
                area.center().y + radius * theta.sin(),
            )
        })
        .collect()
}

fn paint_progress_ring(
    painter: &egui::Painter,
    area: EguiRect,
    fraction: Option<f32>,
    track: Color32,
    fill: Color32,
) {
    let side = area.width().min(area.height());
    if !side.is_finite() || side <= 2.0 {
        return;
    }
    let center = area.center();
    let radius = (side / 2.0 - 1.0).max(0.0);
    painter.circle_stroke(center, radius, Stroke::new(1.7, track));
    if let Some(fraction) = fraction
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .filter(|value| *value > 0.0)
    {
        if fraction == 1.0 {
            painter.circle_stroke(center, radius, Stroke::new(1.7, fill));
        } else {
            let count = (fraction * 64.0).ceil().max(2.0) as usize;
            let points = (0..=count)
                .map(|index| {
                    let angle = -std::f32::consts::FRAC_PI_2
                        + fraction * std::f32::consts::TAU * index as f32 / count as f32;
                    center + egui::vec2(angle.cos() * radius, angle.sin() * radius)
                })
                .collect();
            painter.add(egui::Shape::line(points, Stroke::new(1.7, fill)));
        }
    }
}

/// A vinyl record behind `sleeve`: at `out` 0 it is centered on the sleeve
/// and hidden by it, at 1 it shows `RECORD_REACH` of the sleeve's side
/// beyond its end. The sheen turns with `angle`; the grooves are the same
/// at any angle.
fn paint_record(painter: &egui::Painter, sleeve: EguiRect, out: f32, angle: f32, opacity: f32) {
    let side = sleeve.width();
    if !side.is_finite() || side <= 0.0 {
        return;
    }
    let radius = side * limn::RECORD_SHARE / 2.0;
    let rest = sleeve.center().x;
    let drawn = sleeve.max.x + side * limn::RECORD_REACH - radius;
    let center = Pos2::new(rest + (drawn - rest) * out.clamp(0.0, 1.0), sleeve.center().y);
    let opacity = opacity.clamp(0.0, 1.0);
    let white = |alpha: f32| Color32::from_white_alpha((alpha * opacity * 255.0) as u8);
    let dark = |value: u8| Color32::from_gray(value).gamma_multiply(opacity);
    painter.circle_filled(center, radius, dark(8));
    for share in [0.93, 0.87, 0.8, 0.7, 0.6] {
        painter.circle_stroke(center, radius * share, Stroke::new(0.6, white(0.16)));
    }
    // Four soft wedges of light, a quarter turn apart.
    let spread = 0.17;
    for quarter in 0..4 {
        let theta = angle + std::f32::consts::FRAC_PI_2 * quarter as f32;
        let edge = |theta: f32| center + egui::vec2(theta.cos(), theta.sin()) * radius;
        painter.add(egui::Shape::convex_polygon(vec![center, edge(theta - spread), edge(theta + spread)], white(0.08), Stroke::NONE));
    }
    painter.circle_filled(center, radius * 0.14, dark(54));
    painter.circle_filled(center, radius * 0.05, dark(8));
}

fn paint_spinner(
    painter: &egui::Painter,
    area: EguiRect,
    ink: Color32,
    angle: f32,
    variant: SpinnerVariant,
) {
    let side = area.width().min(area.height());
    if !side.is_finite() || side <= 0.0 {
        return;
    }
    let width = (side * 0.12).clamp(1.5, 2.8);
    let stroke = Stroke::new(width, ink);
    match variant {
        SpinnerVariant::Circle => {
            let points = spinner_arc_points(area, angle, width);
            if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
                painter.add(egui::Shape::line(points, stroke));
                painter.circle_filled(first, width / 2.0, ink);
                painter.circle_filled(last, width / 2.0, ink);
            }
        }
        SpinnerVariant::Spokes => {
            let outer = (side / 2.0 - width / 2.0 - 1.0).max(0.0);
            let inner = outer * 0.55;
            for index in 0..8 {
                let theta = angle + std::f32::consts::TAU * index as f32 / 8.0;
                let direction = egui::vec2(theta.cos(), theta.sin());
                let from = area.center() + direction * inner;
                let to = area.center() + direction * outer;
                painter.line_segment([from, to], stroke);
                painter.circle_filled(from, width / 2.0, ink);
                painter.circle_filled(to, width / 2.0, ink);
            }
        }
    }
}

fn effort_color(r: u8, g: u8, b: u8, alpha: u8, opacity: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        r,
        g,
        b,
        (f32::from(alpha) * opacity.clamp(0.0, 1.0)).round() as u8,
    )
}

fn paint_effort_gradient(painter: &egui::Painter, area: EguiRect, opacity: f32) {
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return;
    }
    let radius = area.height() / 2.0;
    let middle = Color::rgb(
        (f32::from(effort_palette().blue.r) * 0.35 + f32::from(effort_palette().ultra.r) * 0.65).round()
            as u8,
        (f32::from(effort_palette().blue.g) * 0.35 + f32::from(effort_palette().ultra.g) * 0.65).round()
            as u8,
        (f32::from(effort_palette().blue.b) * 0.35 + f32::from(effort_palette().ultra.b) * 0.65).round()
            as u8,
    );
    let stops = [
        (0.0, effort_palette().blue),
        (0.65, middle),
        (1.0, effort_palette().ultra),
    ];
    let left = area.left() + radius;
    let right = area.right() - radius;
    if right <= left {
        painter.rect_filled(
            area,
            CornerRadius::same(radius as u8),
            effort_color(
                effort_palette().blue.r,
                effort_palette().blue.g,
                effort_palette().blue.b,
                255,
                opacity,
            ),
        );
        return;
    }
    painter.circle_filled(
        Pos2::new(left, area.center().y),
        radius,
        effort_color(
            effort_palette().blue.r,
            effort_palette().blue.g,
            effort_palette().blue.b,
            255,
            opacity,
        ),
    );
    painter.circle_filled(
        Pos2::new(right, area.center().y),
        radius,
        effort_color(
            effort_palette().ultra.r,
            effort_palette().ultra.g,
            effort_palette().ultra.b,
            255,
            opacity,
        ),
    );
    let mut mesh = egui::Mesh::default();
    for (fraction, shade) in stops {
        let x = left + (right - left) * fraction;
        let color = effort_color(shade.r, shade.g, shade.b, 255, opacity);
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(x, area.top()),
            uv: egui::epaint::WHITE_UV,
            color,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: Pos2::new(x, area.bottom()),
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    for index in 0..stops.len() - 1 {
        let first = (index * 2) as u32;
        mesh.indices
            .extend([first, first + 1, first + 2, first + 1, first + 3, first + 2]);
    }
    painter.add(egui::Shape::mesh(mesh));
}

fn effort_noise(index: usize, salt: u32) -> f32 {
    let mut value = (index as u32)
        .wrapping_mul(0x9E37_79B9)
        .wrapping_add(salt.wrapping_mul(0x85EB_CA6B));
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    (value as f64 / u32::MAX as f64) as f32
}

/// A seed lives across the full rail, even while the selected fill is short.
/// The fill clips this leftward stream without reseeding it on every stop.
fn effort_particle(index: usize, phase: f32, track: EguiRect) -> (Pos2, f32, u8) {
    let width = track.width().max(1.0);
    let flow = 85.0 + 50.0 * effort_noise(index, 4);
    let x = track.left()
        + ((effort_noise(index, 1) * (width + 6.0) - phase * flow).rem_euclid(width + 6.0))
        - 3.0;
    let y = track.top() + 4.0 + effort_noise(index, 2) * (track.height() - 8.0).max(0.0);
    let radius = 0.8 + 0.9 * effort_noise(index, 3);
    let twinkle = 2.5 + 4.5 * effort_noise(index, 5);
    let seed_phase = effort_noise(index, 6) * std::f32::consts::TAU;
    let pulse = 0.5 + 0.5 * (phase * twinkle + seed_phase).sin();
    let alpha = ((0.06 + 0.74 * pulse * pulse) * 255.0).round() as u8;
    (Pos2::new(x, y), radius, alpha)
}

// These values mirror one DrawCommand::EffortSlider exactly at the render edge.
#[allow(clippy::too_many_arguments)]
fn paint_effort_slider(
    painter: &egui::Painter,
    area: EguiRect,
    fraction: f32,
    steps: usize,
    phase: f32,
    dragging: bool,
    reduced_motion: bool,
    ultra_mix: f32,
    burst_age: Option<f32>,
    opacity: f32,
) {
    if area.width() <= 34.0 || area.height() < 34.0 || opacity <= 0.0 {
        return;
    }
    // The Web Component's rail is 28px, with 34px of travel inset for the
    // white knob. Keep those proportions in the native renderer.
    let track = EguiRect::from_center_size(area.center(), egui::vec2(area.width(), 28.0));
    let thumb_x = area.left() + 17.0 + fraction.clamp(-0.05, 1.05) * (area.width() - 34.0);
    let thumb = Pos2::new(thumb_x, track.center().y);

    painter.rect_filled(
        track,
        CornerRadius::same(14),
        effort_color(effort_palette().input.r, effort_palette().input.g, effort_palette().input.b, 255, opacity),
    );
    let fill_right = (thumb_x + 17.0).clamp(track.left(), track.right());
    let fill = EguiRect::from_min_max(track.left_top(), Pos2::new(fill_right, track.bottom()));
    painter.rect_filled(
        fill,
        CornerRadius::same(14),
        effort_color(
            effort_palette().blue.r,
            effort_palette().blue.g,
            effort_palette().blue.b,
            255,
            opacity,
        ),
    );
    if ultra_mix > 0.001 {
        paint_effort_gradient(&painter.with_clip_rect(fill), fill, opacity * ultra_mix);
    }

    if (2..=12).contains(&steps) {
        let travel = area.width() - 34.0;
        for index in 0..steps {
            let x = area.left() + 17.0 + travel * index as f32 / (steps - 1) as f32;
            if x > fill_right + 2.5 {
                painter.circle_filled(
                    Pos2::new(x, track.center().y),
                    2.5,
                    effort_color(
                        effort_palette().foreground_subtle.r,
                        effort_palette().foreground_subtle.g,
                        effort_palette().foreground_subtle.b,
                        255,
                        opacity,
                    ),
                );
            }
        }
    }

    let effects = painter.with_clip_rect(fill.intersect(track));
    for index in 0..((track.width() / 24.0).round() as usize).max(6) {
        let (center, radius, alpha) = effort_particle(index, phase, track);
        if center.x >= fill.left() && center.x <= fill.right() {
            effects.circle_filled(center, radius, effort_color(255, 255, 255, alpha, opacity));
        }
    }

    // Layered low-alpha circles stand in for the reference's soft 2D shadow.
    painter.circle_filled(
        thumb + egui::vec2(0.0, 2.0),
        20.0,
        effort_color(26, 29, 33, 15, opacity),
    );
    painter.circle_filled(
        thumb + egui::vec2(0.0, 1.0),
        18.5,
        effort_color(26, 29, 33, 20, opacity),
    );
    let pulse = burst_age
        .filter(|_| !reduced_motion && !dragging)
        .map_or(0.0, |age| {
            (age / 0.18).clamp(0.0, 1.0) * std::f32::consts::PI
        });
    let knob_radius =
        17.0 * if dragging && !reduced_motion {
            1.04
        } else {
            1.0
        } * (1.0 + pulse.sin().max(0.0) * 0.05);
    painter.circle_filled(
        thumb,
        knob_radius,
        effort_color(255, 255, 255, 255, opacity),
    );
    painter.circle_stroke(
        thumb,
        knob_radius,
        Stroke::new(0.5, effort_color(26, 29, 33, 16, opacity)),
    );

    if let Some(age) = burst_age.filter(|_| !reduced_motion) {
        let colors = [
            (201, 176, 240),
            (191, 165, 242),
            (212, 195, 247),
            (183, 158, 245),
        ];
        for index in 0..14 {
            let lifetime = 0.2 + 0.08 * effort_noise(index, 17);
            if age >= lifetime {
                continue;
            }
            let angle = index as f32 * std::f32::consts::TAU / 14.0
                + (effort_noise(index, 18) - 0.5) * 0.35;
            let speed = 105.0 + 45.0 * effort_noise(index, 19);
            let travel = speed * (1.0 - (-6.0 * age).exp()) / 6.0;
            let direction = egui::vec2(angle.cos(), angle.sin());
            let center = thumb + direction * (17.0 + travel) + egui::vec2(0.0, -25.0 * age);
            let radius = (4.5 + effort_noise(index, 20)) / 2.0;
            let fade = (1.0 - age / lifetime).powf(1.5);
            let (r, g, b) = colors[index % colors.len()];
            painter.circle_filled(center, radius, effort_color(r, g, b, 255, opacity * fade));
        }
    }
}

// A texture rendered at its final physical size needs integer pixel edges.
// Fractional UI layout coordinates otherwise make the GPU resample a sharp
// raster even when the texture itself has exactly the right resolution.
/// Largest icon side the tree can request, in logical pixels.
const MAX_ICON_SIDE: f32 = 128.0;

fn aligned_icon_rect(area: EguiRect, pixels_per_point: f32) -> Option<(EguiRect, u16)> {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    // The tree reserves 16 points for control icons and up to 128 for a
    // standalone `large_icon`; nothing may paint beyond that.
    let side = area.width().min(area.height()).min(MAX_ICON_SIDE);
    if !side.is_finite() || side <= 0.0 {
        return None;
    }
    // Decimal DPI factors and egui rectangles can land just below a half
    // pixel through f32 rounding (for example 14 * 1.25 as 17.499998).
    let pixels = (side * pixels_per_point + 0.0001).round().clamp(1.0, 256.0) as u16;
    let center = area.center();
    let x = (center.x * pixels_per_point - f32::from(pixels) / 2.0).round();
    let y = (center.y * pixels_per_point - f32::from(pixels) / 2.0).round();
    let min = Pos2::new(x / pixels_per_point, y / pixels_per_point);
    let size = egui::Vec2::splat(f32::from(pixels) / pixels_per_point);
    Some((EguiRect::from_min_size(min, size), pixels))
}

/// Paint a frame with its origin at the top-left corner of the space `ui` has available.
pub fn paint(ui: &egui::Ui, frame: &Frame) {
    paint_at(ui, frame, ui.max_rect().min);
}

/// Give the pointer the shape the tree asks for: `set_cursor(ctx, tree.cursor())`
/// once a frame, after the layout. egui goes back to its own shape every
/// frame, so `None` needs nothing done.
pub fn set_cursor(ctx: &egui::Context, cursor: Option<limn::Cursor>) {
    if let Some(cursor) = cursor {
        ctx.set_cursor_icon(match cursor {
            limn::Cursor::ResizeColumn => egui::CursorIcon::ResizeColumn,
            limn::Cursor::ResizeRow => egui::CursorIcon::ResizeRow,
        });
    }
}

/// Paint a tree laid out in its own coordinate space at a window position.
pub fn paint_at(ui: &egui::Ui, frame: &Frame, origin: Pos2) {
    paint_at_impl(ui, frame, origin, None, None);
}

/// The ID a limn control has in the platform's accessibility tree.
fn access_id(id: &str) -> egui::Id {
    egui::Id::new(("limn-access", id))
}

/// Tell the platform what is in the frame: each control with its role, its
/// name and its state, where it is. Nothing is built unless something on
/// the platform is listening.
fn publish_access(ui: &egui::Ui, frame: &Frame, origin: Pos2) {
    use egui::accesskit::{Action, Role as Platform, Toggled};
    use limn::Role;
    for node in &frame.access {
        let bounds = rect(origin, node.rect);
        ui.ctx().accesskit_node_builder(access_id(&node.id), |built| {
            built.set_role(match node.role {
                Role::Button => Platform::Button,
                Role::Link => Platform::Link,
                Role::CheckBox => Platform::CheckBox,
                Role::Switch => Platform::Switch,
                Role::RadioButton => Platform::RadioButton,
                Role::Slider => Platform::Slider,
                Role::TextInput => Platform::TextInput,
                Role::PasswordInput => Platform::PasswordInput,
                Role::MultilineTextInput => Platform::MultilineTextInput,
                Role::SearchInput => Platform::SearchInput,
                Role::ComboBox => Platform::ComboBox,
                Role::Tab => Platform::Tab,
                Role::MenuItem => Platform::MenuItem,
                Role::MenuItemCheckBox => Platform::MenuItemCheckBox,
                Role::MenuItemRadio => Platform::MenuItemRadio,
                Role::ListItem => Platform::ListItem,
                Role::TreeItem => Platform::TreeItem,
                Role::Dialog => Platform::Dialog,
                Role::AlertDialog => Platform::AlertDialog,
                Role::Alert => Platform::Alert,
                Role::Status => Platform::Status,
                Role::Tooltip => Platform::Tooltip,
                Role::ProgressIndicator => Platform::ProgressIndicator,
                Role::Image => Platform::Image,
                Role::Heading => Platform::Heading,
                Role::Label => Platform::Label,
                // A role this backend does not know yet is still a thing with a name.
                _ => Platform::Group,
            });
            built.set_bounds(egui::accesskit::Rect {
                x0: bounds.min.x.into(),
                y0: bounds.min.y.into(),
                x1: bounds.max.x.into(),
                y1: bounds.max.y.into(),
            });
            if !node.name.is_empty() {
                built.set_label(node.name.as_str());
            }
            if let Some(description) = &node.description {
                built.set_description(description.as_str());
            }
            if node.disabled {
                built.set_disabled();
            }
            if node.mixed {
                built.set_toggled(Toggled::Mixed);
            } else if let Some(on) = node.toggled {
                built.set_toggled(if on { Toggled::True } else { Toggled::False });
            }
            if let Some(selected) = node.selected {
                built.set_selected(selected);
            }
            if let Some(expanded) = node.expanded {
                built.set_expanded(expanded);
            }
            if let Some(value) = node.value {
                built.set_numeric_value(f64::from(value));
                built.set_min_numeric_value(0.0);
                built.set_max_numeric_value(1.0);
            }
            if let Some(text) = &node.value_text {
                built.set_value(text.as_str());
            }
            if node.actionable {
                built.add_action(Action::Focus);
                built.add_action(Action::Click);
            }
        });
        if node.focused {
            ui.ctx().memory_mut(|memory| memory.request_focus(access_id(&node.id)));
        }
    }
}

/// Do what the platform asked of the controls in the tree's last frame: a
/// screen reader's "activate" is a click on the control, and its "focus"
/// gives it the keyboard. Call it each frame beside [`events`], and apply
/// the actions it returns as you do the tree's own.
pub fn apply_access_requests(ctx: &egui::Context, tree: &mut limn::UiTree) -> Vec<limn::UiAction> {
    use egui::accesskit::Action;
    let asked: Vec<(String, limn::Rect, bool)> = ctx.input(|input| {
        tree.frame()
            .access
            .iter()
            .filter(|node| node.actionable)
            .filter_map(|node| {
                let id = access_id(&node.id);
                let click = input.has_accesskit_action_request(id, Action::Click);
                (click || input.has_accesskit_action_request(id, Action::Focus)).then(|| (node.id.clone(), node.rect, click))
            })
            .collect()
    });
    let mut actions = Vec::new();
    for (id, rect, click) in asked {
        if click {
            // The same path a pointer takes, so a control answers it as it answers a click.
            let middle = limn::Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            actions.extend(tree.handle_event(UiEvent::PointerDown(middle)));
            actions.extend(tree.handle_event(UiEvent::PointerUp(middle)));
        } else {
            tree.focus(&id);
        }
    }
    actions
}

#[derive(Clone, Copy)]
struct BackdropSurface<'a> {
    id: &'a str,
    texture: egui::TextureId,
    rect: EguiRect,
}

fn paint_at_impl(
    ui: &egui::Ui,
    frame: &Frame,
    origin: Pos2,
    glass: Option<(&GlassRenderer, EguiRect)>,
    backdrop_surface: Option<BackdropSurface<'_>>,
) {
    publish_access(ui, frame, origin);
    let mut clips = vec![ui.clip_rect()];
    for command in &frame.commands {
        match command {
            DrawCommand::Glass {
                rect: area,
                radius,
                material,
                opacity,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let area = rect(origin, *area);
                let painted = glass.is_some_and(|(renderer, backdrop)| {
                    renderer.paint_glass(ui, &painter, area, *radius, *material, *opacity, backdrop)
                });
                if !painted {
                    let tint = material.tint;
                    painter.rect_filled(
                        area,
                        CornerRadius::same(*radius as u8),
                        Color32::from_rgba_unmultiplied(
                            tint.r,
                            tint.g,
                            tint.b,
                            (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
                        ),
                    );
                }
            }
            DrawCommand::PushClip(area) => {
                let next = clips
                    .last()
                    .copied()
                    .unwrap_or(ui.clip_rect())
                    .intersect(rect(origin, *area));
                clips.push(next);
            }
            DrawCommand::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            DrawCommand::Shadow {
                rect: area,
                shadow,
                radius,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let offset = [
                    shadow
                        .offset
                        .x
                        .round()
                        .clamp(i8::MIN as f32, i8::MAX as f32) as i8,
                    shadow
                        .offset
                        .y
                        .round()
                        .clamp(i8::MIN as f32, i8::MAX as f32) as i8,
                ];
                let shape = egui::epaint::Shadow {
                    offset,
                    blur: shadow.blur.round().clamp(0.0, u8::MAX as f32) as u8,
                    spread: shadow.spread.round().clamp(0.0, u8::MAX as f32) as u8,
                    color: color(shadow.color),
                }
                .as_shape(rect(origin, *area), CornerRadius::same(*radius as u8));
                painter.add(shape);
            }
            DrawCommand::FillRect {
                rect: area,
                color: fill,
                radius,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.rect_filled(
                    rect(origin, *area),
                    CornerRadius::same(*radius as u8),
                    color(*fill),
                );
            }
            DrawCommand::FillCornerRect {
                rect: area,
                color: fill,
                radii,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.rect_filled(
                    rect(origin, *area),
                    CornerRadius {
                        nw: radii.top_left.clamp(0.0, 255.0) as u8,
                        ne: radii.top_right.clamp(0.0, 255.0) as u8,
                        se: radii.bottom_right.clamp(0.0, 255.0) as u8,
                        sw: radii.bottom_left.clamp(0.0, 255.0) as u8,
                    },
                    color(*fill),
                );
            }
            DrawCommand::CornerMask {
                rect: area,
                color: fill,
                radii,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.add(egui::Shape::mesh(corner_mask_render::mesh(
                    rect(origin, *area),
                    color(*fill),
                    *radii,
                    ui.ctx().pixels_per_point(),
                )));
            }
            DrawCommand::SelectionContour { points, color: fill } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.add(egui::Shape::mesh(selection_contour_render::mesh(
                    points,
                    origin,
                    color(*fill),
                    ui.ctx().pixels_per_point(),
                )));
            }
            DrawCommand::FillVerticalGradient {
                rect: area,
                top,
                bottom,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let area = rect(origin, *area);
                let mut mesh = egui::Mesh::default();
                let uv = egui::epaint::WHITE_UV;
                mesh.vertices.extend([
                    egui::epaint::Vertex {
                        pos: area.left_top(),
                        uv,
                        color: color(*top),
                    },
                    egui::epaint::Vertex {
                        pos: area.right_top(),
                        uv,
                        color: color(*top),
                    },
                    egui::epaint::Vertex {
                        pos: area.right_bottom(),
                        uv,
                        color: color(*bottom),
                    },
                    egui::epaint::Vertex {
                        pos: area.left_bottom(),
                        uv,
                        color: color(*bottom),
                    },
                ]);
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                painter.add(egui::Shape::mesh(mesh));
            }
            DrawCommand::FillHorizontalGradient {
                rect: area,
                left,
                right,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let area = rect(origin, *area);
                let mut mesh = egui::Mesh::default();
                let uv = egui::epaint::WHITE_UV;
                mesh.vertices.extend([
                    egui::epaint::Vertex {
                        pos: area.left_top(),
                        uv,
                        color: color(*left),
                    },
                    egui::epaint::Vertex {
                        pos: area.right_top(),
                        uv,
                        color: color(*right),
                    },
                    egui::epaint::Vertex {
                        pos: area.right_bottom(),
                        uv,
                        color: color(*right),
                    },
                    egui::epaint::Vertex {
                        pos: area.left_bottom(),
                        uv,
                        color: color(*left),
                    },
                ]);
                mesh.indices.extend([0, 1, 2, 0, 2, 3]);
                painter.add(egui::Shape::mesh(mesh));
            }
            DrawCommand::StrokeRect {
                rect: area,
                color: ink,
                width,
                radius,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.rect_stroke(
                    rect(origin, *area),
                    CornerRadius::same(*radius as u8),
                    Stroke::new(*width, color(*ink)),
                    StrokeKind::Inside,
                );
            }
            DrawCommand::StrokeCornerRect {
                rect: area,
                color: ink,
                width,
                radii,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.rect_stroke(
                    rect(origin, *area),
                    CornerRadius {
                        nw: radii.top_left.clamp(0.0, 255.0) as u8,
                        ne: radii.top_right.clamp(0.0, 255.0) as u8,
                        se: radii.bottom_right.clamp(0.0, 255.0) as u8,
                        sw: radii.bottom_left.clamp(0.0, 255.0) as u8,
                    },
                    Stroke::new(*width, color(*ink)),
                    StrokeKind::Inside,
                );
            }
            DrawCommand::FillTriangle {
                points,
                color: fill,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                painter.add(egui::Shape::convex_polygon(
                    points.iter().map(|point| pos(origin, *point)).collect(),
                    color(*fill),
                    Stroke::NONE,
                ));
            }
            DrawCommand::FillMesh { points, colors, indices } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let mut mesh = egui::Mesh::default();
                mesh.vertices.extend(points.iter().zip(colors.iter()).map(|(point, ink)| egui::epaint::Vertex {
                    pos: pos(origin, *point),
                    uv: egui::epaint::WHITE_UV,
                    color: color(*ink),
                }));
                // A corner that is not there would be read past the end.
                if indices.iter().all(|index| (*index as usize) < mesh.vertices.len()) {
                    mesh.indices.extend(indices.iter().copied());
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            DrawCommand::Polyline {
                points,
                color: ink,
                width,
                dashed,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let path: Vec<egui::Pos2> = points.iter().map(|point| pos(origin, *point)).collect();
                let stroke = Stroke::new(*width, color(*ink));
                if *dashed {
                    painter.extend(egui::Shape::dashed_line(&path, stroke, 3.0, 3.0));
                } else {
                    painter.add(egui::Shape::line(path, stroke));
                }
            }
            DrawCommand::Text {
                position,
                text,
                color: ink,
                font_size,
                font_family,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let font = match font_family {
                    FontFamily::Monospace => FontId::monospace(*font_size),
                    FontFamily::Sans => FontId::proportional(*font_size),
                    FontFamily::SansMedium | FontFamily::SansBold => {
                        FontId::new(*font_size, egui_family(ui.ctx(), *font_family))
                    }
                    FontFamily::SansItalic => {
                        // egui slants the regular face, so advances match Sans.
                        let format = egui::TextFormat {
                            font_id: FontId::proportional(*font_size),
                            color: color(*ink),
                            italics: true,
                            ..Default::default()
                        };
                        let job = egui::text::LayoutJob::single_section(text.clone(), format);
                        painter.galley(pos(origin, *position), painter.layout_job(job), color(*ink));
                        continue;
                    }
                };
                painter.text(
                    pos(origin, *position),
                    Align2::LEFT_TOP,
                    text,
                    font,
                    color(*ink),
                );
            }
            DrawCommand::Icon {
                rect: area,
                icon,
                tint,
            } => {
                if let Some((aligned, pixels)) =
                    aligned_icon_rect(rect(origin, *area), ui.ctx().pixels_per_point())
                {
                    let key = IconKey {
                        icon: *icon,
                        pixels,
                    };
                    if let Some(texture) = icon_texture(ui, key) {
                        let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                        painter.image(
                            texture,
                            aligned,
                            EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            icon_tint(*icon, *tint),
                        );
                    }
                }
            }
            DrawCommand::RasterImage {
                rect: area,
                image,
                opacity,
                radius,
            } => {
                let area = rect(origin, *area);
                let response = ui.interact(
                    area,
                    egui::Id::new((
                        "limn-raster-image",
                        image.sha256(),
                        area.min.x.to_bits(),
                        area.min.y.to_bits(),
                    )),
                    egui::Sense::hover(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Image, true, image.alt())
                });
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let texture = raster_texture(ui, image);
                if let RasterTexture::Ready(texture, pixels) = texture {
                    let (target, uv) = if image.is_cover() {
                        (area, cover_uv(area, pixels))
                    } else {
                        (fit_image(area, pixels), EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)))
                    };
                    painter.add(
                        egui::epaint::RectShape::filled(
                            target,
                            CornerRadius::same(radius.clamp(0.0, 255.0) as u8),
                            Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8),
                        )
                        .with_texture(texture, uv),
                    );
                } else if texture == RasterTexture::Unavailable {
                    painter.rect_filled(area, CornerRadius::same(6), Color32::from_gray(37));
                    painter.text(
                        area.center(),
                        Align2::CENTER_CENTER,
                        limn::labels().preview_unavailable,
                        FontId::proportional(12.0),
                        Color32::from_gray(190),
                    );
                }
            }
            DrawCommand::Surface {
                rect: area,
                id,
                label,
                opacity,
            } => {
                let area = rect(origin, *area);
                let response = ui.interact(
                    area,
                    egui::Id::new(("limn-surface", id)),
                    egui::Sense::hover(),
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Image, true, label)
                });
                let painter = ui
                    .painter()
                    .with_clip_rect(clips.last().unwrap().intersect(area));
                if let Some(backdrop) = backdrop_surface.filter(|backdrop| backdrop.id == id) {
                    painter.image(
                        backdrop.texture,
                        backdrop.rect,
                        EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8),
                    );
                } else if let Some((texture, pixels, density)) = surface_texture(ui, id) {
                    painter.image(
                        texture,
                        surface_rect(area, pixels, density, ui.ctx().pixels_per_point()),
                        EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8),
                    );
                }
            }
            DrawCommand::ProgressRing {
                rect: area,
                fraction,
                track,
                fill,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                paint_progress_ring(
                    &painter,
                    rect(origin, *area),
                    *fraction,
                    color(*track),
                    color(*fill),
                );
            }
            DrawCommand::Record {
                rect: area,
                image,
                fill,
                out,
                angle,
                radius,
                opacity,
            } => {
                let area = rect(origin, *area);
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                let side = area.width().min(area.height());
                let sleeve = EguiRect::from_min_size(area.min, egui::vec2(side, side));
                paint_record(&painter, sleeve, *out, *angle, *opacity);
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
                let corner = CornerRadius::same(radius.clamp(0.0, 255.0) as u8);
                match image.as_ref().map(|image| raster_texture(ui, image)) {
                    Some(RasterTexture::Ready(texture, pixels)) => {
                        painter.add(
                            egui::epaint::RectShape::filled(sleeve, corner, Color32::from_white_alpha(alpha))
                                .with_texture(texture, cover_uv(sleeve, pixels)),
                        );
                    }
                    _ => {
                        painter.rect_filled(sleeve, corner, color(*fill).gamma_multiply(opacity.clamp(0.0, 1.0)));
                    }
                }
            }
            DrawCommand::Spinner {
                rect: area,
                color: ink,
                angle,
                variant,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                paint_spinner(&painter, rect(origin, *area), color(*ink), *angle, *variant);
            }
            DrawCommand::EffortSlider {
                rect: area,
                fraction,
                steps,
                phase,
                dragging,
                reduced_motion,
                ultra_mix,
                burst_age,
                opacity,
            } => {
                let painter = ui.painter().with_clip_rect(*clips.last().unwrap());
                paint_effort_slider(
                    &painter,
                    rect(origin, *area),
                    *fraction,
                    *steps,
                    *phase,
                    *dragging,
                    *reduced_motion,
                    *ultra_mix,
                    *burst_age,
                    *opacity,
                );
            }
        }
    }
}

fn relative(origin: Pos2, position: Pos2) -> Point {
    Point::new(position.x - origin.x, position.y - origin.y)
}

/// Translate this frame's egui input into limn events, with pointer positions made
/// relative to `origin`. A primary press with Alt, Ctrl or Command held, or a double or
/// triple click, becomes `PointerDownEditor`; wheel input is dropped while the pointer is
/// outside the window; only the keys limn knows are forwarded, with Command counted as Ctrl.
pub fn events(ctx: &egui::Context, origin: Pos2) -> Vec<UiEvent> {
    ctx.input(|input| {
        let mut events = Vec::new();
        let mut pointer = input.pointer.hover_pos();
        let native_paste = input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Paste(_)));
        let clipboard_key = |target| {
            input.events.iter().any(|event| {
                matches!(event,
                    egui::Event::Key { key, pressed: true, modifiers, .. }
                        if *key == target && (modifiers.ctrl || modifiers.command)
                )
            })
        };
        let enter_pressed = input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    ..
                }
            )
        });
        // The OS owns the pointer during an external drag, so the position is
        // the last one egui saw; it can lag until the drop on some platforms.
        if let Some(position) = input.pointer.latest_pos().map(|pos| relative(origin, pos)) {
            let paths: Vec<_> = input
                .raw
                .dropped_files
                .iter()
                .map(|file| file.path().to_path_buf())
                .filter(|path| !path.as_os_str().is_empty())
                .collect();
            if !paths.is_empty() {
                events.push(UiEvent::FilesDropped { position, paths });
            } else if !input.raw.hovered_files.is_empty() {
                events.push(UiEvent::FilesHovered { position });
            }
        }
        for event in &input.events {
            match event {
                egui::Event::PointerMoved(pos) => {
                    pointer = Some(*pos);
                    events.push(UiEvent::PointerMove(relative(origin, *pos)));
                }
                egui::Event::PointerGone => {
                    pointer = None;
                    events.push(UiEvent::PointerLeave);
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: true,
                    ..
                } => {
                    pointer = Some(*pos);
                    events.push(UiEvent::ContextMenu {
                        position: relative(origin, *pos),
                    });
                }
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers,
                } => {
                    pointer = Some(*pos);
                    events.push(
                        if *pressed
                            && (modifiers.alt
                                || modifiers.ctrl
                                || modifiers.command
                                || input
                                    .pointer
                                    .button_double_clicked(egui::PointerButton::Primary)
                                || input
                                    .pointer
                                    .button_triple_clicked(egui::PointerButton::Primary))
                        {
                            UiEvent::PointerDownEditor {
                                position: relative(origin, *pos),
                                shift: modifiers.shift,
                                alt: modifiers.alt,
                                ctrl: modifiers.ctrl || modifiers.command,
                                click_count: if input
                                    .pointer
                                    .button_triple_clicked(egui::PointerButton::Primary)
                                {
                                    3
                                } else if input
                                    .pointer
                                    .button_double_clicked(egui::PointerButton::Primary)
                                {
                                    2
                                } else {
                                    1
                                },
                            }
                        } else if *pressed && modifiers.shift {
                            UiEvent::PointerDownModified {
                                position: relative(origin, *pos),
                                shift: true,
                            }
                        } else if *pressed {
                            UiEvent::PointerDown(relative(origin, *pos))
                        } else {
                            UiEvent::PointerUp(relative(origin, *pos))
                        },
                    );
                }
                egui::Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => {
                    if let Some(pos) = pointer {
                        let scale = match unit {
                            egui::MouseWheelUnit::Point => 1.0,
                            egui::MouseWheelUnit::Line => 30.0,
                            egui::MouseWheelUnit::Page => 500.0,
                        };
                        let position = relative(origin, pos);
                        let delta_y = delta.y * scale;
                        events.push(if delta.x != 0.0 || modifiers.shift {
                            UiEvent::Scroll2D {
                                position,
                                delta_x: delta.x * scale,
                                delta_y,
                                direct: *unit == egui::MouseWheelUnit::Point,
                                shift: modifiers.shift,
                            }
                        } else if *unit == egui::MouseWheelUnit::Point {
                            UiEvent::ScrollDirect { position, delta_y }
                        } else {
                            UiEvent::Scroll { position, delta_y }
                        });
                    }
                }
                egui::Event::Text(value)
                    if !(enter_pressed && matches!(value.as_str(), "\n" | "\r" | "\r\n")) => {
                        events.push(UiEvent::Text(value.clone()));
                    }
                egui::Event::Ime(egui::ImeEvent::Preedit { text: value, .. }) => {
                    events.push(UiEvent::ImePreedit(value.clone()))
                }
                egui::Event::Ime(egui::ImeEvent::DeleteSurrounding {
                    before_chars,
                    after_chars,
                }) => events.push(UiEvent::ImeDeleteSurrounding {
                    before_chars: *before_chars,
                    after_chars: *after_chars,
                }),
                egui::Event::Ime(egui::ImeEvent::Commit(value)) => {
                    events.push(UiEvent::ImeCommit(value.clone()))
                }
                #[allow(deprecated)]
                egui::Event::Ime(egui::ImeEvent::Enabled | egui::ImeEvent::Disabled) => {
                    events.push(UiEvent::ImeCancelled)
                }
                egui::Event::Paste(value) => {
                    events.push(UiEvent::Paste(value.clone()));
                }
                egui::Event::Copy if !clipboard_key(egui::Key::C) => events.push(UiEvent::Key {
                    key: Key::C,
                    shift: input.modifiers.shift,
                    ctrl: true,
                }),
                egui::Event::Cut if !clipboard_key(egui::Key::X) => events.push(UiEvent::Key {
                    key: Key::X,
                    shift: input.modifiers.shift,
                    ctrl: true,
                }),
                egui::Event::Key {
                    key: egui::Key::F10,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.shift && !modifiers.ctrl && !modifiers.alt && !modifiers.command => {
                    events.push(UiEvent::ContextMenuKeyboard);
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Some backends report both the shortcut and the clipboard
                    // event. Forward a paste once, using the delivered payload.
                    if *key == egui::Key::V && (modifiers.ctrl || modifiers.command) && native_paste
                    {
                        continue;
                    }
                    let mapped = match key {
                        egui::Key::Escape => Some(Key::Escape),
                        egui::Key::F2 => Some(Key::F2),
                        egui::Key::F5 => Some(Key::F5),
                        egui::Key::F9 => Some(Key::F9),
                        egui::Key::F10 => Some(Key::F10),
                        egui::Key::F11 => Some(Key::F11),
                        egui::Key::F12 => Some(Key::F12),
                        egui::Key::ArrowLeft => Some(Key::ArrowLeft),
                        egui::Key::ArrowRight => Some(Key::ArrowRight),
                        egui::Key::ArrowUp => Some(Key::ArrowUp),
                        egui::Key::ArrowDown => Some(Key::ArrowDown),
                        egui::Key::Home => Some(Key::Home),
                        egui::Key::End => Some(Key::End),
                        egui::Key::PageUp => Some(Key::PageUp),
                        egui::Key::PageDown => Some(Key::PageDown),
                        egui::Key::Backspace => Some(Key::Backspace),
                        egui::Key::Delete => Some(Key::Delete),
                        egui::Key::Enter => Some(Key::Enter),
                        egui::Key::Tab => Some(Key::Tab),
                        egui::Key::Space => Some(Key::Space),
                        egui::Key::A => Some(Key::A),
                        egui::Key::B => Some(Key::B),
                        egui::Key::C => Some(Key::C),
                        egui::Key::D => Some(Key::D),
                        egui::Key::E => Some(Key::E),
                        egui::Key::F => Some(Key::F),
                        egui::Key::G => Some(Key::G),
                        egui::Key::H => Some(Key::H),
                        egui::Key::I => Some(Key::I),
                        egui::Key::J => Some(Key::J),
                        egui::Key::K => Some(Key::K),
                        egui::Key::L => Some(Key::L),
                        egui::Key::M => Some(Key::M),
                        egui::Key::N => Some(Key::N),
                        egui::Key::O => Some(Key::O),
                        egui::Key::P => Some(Key::P),
                        egui::Key::Q => Some(Key::Q),
                        egui::Key::R => Some(Key::R),
                        egui::Key::S => Some(Key::S),
                        egui::Key::T => Some(Key::T),
                        egui::Key::U => Some(Key::U),
                        egui::Key::V => Some(Key::V),
                        egui::Key::W => Some(Key::W),
                        egui::Key::X => Some(Key::X),
                        egui::Key::Y => Some(Key::Y),
                        egui::Key::Z => Some(Key::Z),
                        _ => None,
                    };
                    if let Some(key) = mapped {
                        events.push(UiEvent::Key {
                            key,
                            shift: modifiers.shift,
                            ctrl: modifiers.ctrl || modifiers.command,
                        });
                    }
                }
                _ => {}
            }
        }
        events
    })
}

/// Use Ubuntu Regular for UI text. egui's default proportional face is
/// Ubuntu Light, whose thin strokes read as blurry at small sizes. Call once
/// when the window is created, before the first frame and [`attach_fonts`];
/// egui's Unicode, emoji and monospace fallbacks are kept.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let name = "Ubuntu-Regular".to_owned();
    fonts.font_data.insert(
        name.clone(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Ubuntu-Regular.ttf")).into(),
    );
    // The weights are faces of their own, with the same fallbacks behind them.
    let fallbacks = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (family, face, bytes) in [
        (SANS_MEDIUM, "Ubuntu-Medium", &include_bytes!("../assets/fonts/Ubuntu-Medium.ttf")[..]),
        (SANS_BOLD, "Ubuntu-Bold", &include_bytes!("../assets/fonts/Ubuntu-Bold.ttf")[..]),
    ] {
        fonts
            .font_data
            .insert(face.to_owned(), egui::FontData::from_static(bytes).into());
        let mut faces = vec![face.to_owned()];
        faces.extend(fallbacks.iter().cloned());
        fonts.families.insert(egui::FontFamily::Name(family.into()), faces);
    }
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, name);
    ctx.set_fonts(fonts);
}

const SANS_MEDIUM: &str = "limn-sans-medium";
const SANS_BOLD: &str = "limn-sans-bold";

/// The egui family a limn family is painted and measured with. A host that
/// set its own fonts without the weighted faces gets the regular one.
fn egui_family(ctx: &egui::Context, family: FontFamily) -> egui::FontFamily {
    let weighted = match family {
        FontFamily::Monospace => return egui::FontFamily::Monospace,
        FontFamily::Sans | FontFamily::SansItalic => return egui::FontFamily::Proportional,
        FontFamily::SansMedium => SANS_MEDIUM,
        FontFamily::SansBold => SANS_BOLD,
    };
    let named = egui::FontFamily::Name(weighted.into());
    if ctx.fonts(|fonts| fonts.definitions().families.contains_key(&named)) {
        named
    } else {
        egui::FontFamily::Proportional
    }
}

/// Make a tree measure text with the fonts egui paints with. Call once per
/// tree, and again after changing the context's fonts.
pub fn attach_fonts(tree: &mut limn::UiTree, ctx: &egui::Context) {
    let mono = egui::FontId::monospace(14.0);
    let sans = egui::FontId::proportional(14.0);
    let (metrics, sans_metrics) = ctx.fonts_mut(|fonts| {
        (
            limn::FontMetrics {
                char_width: fonts.glyph_width(&mono, 'M'),
                line_height: fonts.row_height(&mono),
            },
            limn::FontMetrics {
                char_width: ('a'..='z')
                    .map(|character| fonts.glyph_width(&sans, character))
                    .sum::<f32>()
                    / 26.0,
                line_height: fonts.row_height(&sans),
            },
        )
    });
    tree.set_metrics(metrics);
    tree.set_sans_metrics(sans_metrics);
    let heights = ctx.clone();
    // The family is resolved before the fonts are locked to measure with it.
    tree.set_text_height_measurer(move |size, font| {
        let family = egui_family(&heights, font);
        heights.fonts_mut(|fonts| fonts.row_height(&egui::FontId::new(size, family)))
    });
    let widths = ctx.clone();
    tree.set_text_measurer(move |text, size, font| {
        let family = egui_family(&widths, font);
        widths.fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(
                    text.to_owned(),
                    egui::FontId::new(size, family),
                    Color32::WHITE,
                )
                .size()
                .x
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_covering_image_shows_its_centered_part_at_the_box_proportions() {
        let square = EguiRect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0));
        let wide = cover_uv(square, [300, 100]);
        assert!((wide.width() - 1.0 / 3.0).abs() < 1e-6 && wide.height() == 1.0, "a banner loses its sides");
        assert!((wide.center().x - 0.5).abs() < 1e-6);
        let tall = cover_uv(square, [100, 400]);
        assert!(tall.width() == 1.0 && (tall.height() - 0.25).abs() < 1e-6, "a portrait loses top and bottom");
        assert_eq!(cover_uv(square, [64, 64]), EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
        let empty = EguiRect::from_min_size(Pos2::ZERO, egui::vec2(0.0, 0.0));
        assert_eq!(cover_uv(empty, [64, 0]), EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), "nothing to divide by");
    }
    use image::ImageEncoder as _;
    use std::sync::Arc;

    #[test]
    fn debugger_function_keys_keep_shift_f10_as_the_context_menu_shortcut() {
        let ctx = egui::Context::default();
        let mut forwarded = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput {
            events: [(egui::Key::F5, false), (egui::Key::F10, false), (egui::Key::F11, false),
                (egui::Key::F11, true), (egui::Key::F10, true)].into_iter().map(|(key, shift)| egui::Event::Key {
                    key, physical_key: None, pressed: true, repeat: false,
                    modifiers: egui::Modifiers { shift, ..Default::default() },
                }).collect(), ..Default::default()
        }, |ui| forwarded = events(ui.ctx(), Pos2::ZERO));
        output.textures_delta.clear();
        assert_eq!(forwarded, vec![UiEvent::Key {key: Key::F5, ctrl: false, shift: false},
            UiEvent::Key {key: Key::F10, ctrl: false, shift: false},
            UiEvent::Key {key: Key::F11, ctrl: false, shift: false},
            UiEvent::Key {key: Key::F11, ctrl: false, shift: true}, UiEvent::ContextMenuKeyboard]);
    }

    #[test]
    fn native_context_and_clipboard_events_do_not_replace_the_primary_selection() {
        let ctx = egui::Context::default();
        let mut forwarded = Vec::new();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerButton {
                        pos: Pos2::new(140.0, 80.0),
                        button: egui::PointerButton::Secondary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                    egui::Event::PointerButton {
                        pos: Pos2::new(140.0, 80.0),
                        button: egui::PointerButton::Secondary,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    },
                    egui::Event::Copy,
                    egui::Event::Cut,
                    egui::Event::Key {
                        key: egui::Key::F10,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers {
                            shift: true,
                            ..Default::default()
                        },
                    },
                ],
                ..Default::default()
            },
            |ui| forwarded = events(ui.ctx(), Pos2::new(100.0, 50.0)),
        );
        output.textures_delta.clear();
        assert_eq!(
            forwarded,
            vec![
                UiEvent::ContextMenu {
                    position: Point::new(40.0, 30.0)
                },
                UiEvent::Key {
                    key: Key::C,
                    ctrl: true,
                    shift: false
                },
                UiEvent::Key {
                    key: Key::X,
                    ctrl: true,
                    shift: false
                },
                UiEvent::ContextMenuKeyboard,
            ]
        );
    }

    #[test]
    fn native_clipboard_events_do_not_duplicate_keys_or_lose_terminal_modifiers() {
        let ctx = egui::Context::default();
        let mut forwarded = Vec::new();
        let shortcut = |key, shift| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                ctrl: true,
                shift,
                ..Default::default()
            },
        };
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![
                    shortcut(egui::Key::C, true),
                    egui::Event::Copy,
                    shortcut(egui::Key::X, false),
                    egui::Event::Cut,
                    shortcut(egui::Key::V, false),
                    egui::Event::Paste("ñ 東京".into()),
                ],
                ..Default::default()
            },
            |ui| forwarded = events(ui.ctx(), Pos2::ZERO),
        );
        output.textures_delta.clear();
        assert_eq!(
            forwarded,
            vec![
                UiEvent::Key {
                    key: Key::C,
                    ctrl: true,
                    shift: true
                },
                UiEvent::Key {
                    key: Key::X,
                    ctrl: true,
                    shift: false
                },
                UiEvent::Paste("ñ 東京".into()),
            ]
        );
    }

    #[test]
    fn native_wheel_units_preserve_direct_touchpad_input_and_scroll_direction() {
        let ctx = egui::Context::default();
        let origin = Pos2::new(100.0, 50.0);
        let mut forwarded = Vec::new();
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(Pos2::new(140.0, 80.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -2.5),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::default(),
                    },
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line,
                        delta: egui::vec2(0.0, -2.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::default(),
                    },
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Page,
                        delta: egui::vec2(0.0, 1.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
            |ui| forwarded = events(ui.ctx(), origin),
        );
        // This input-only probe has no renderer to apply texture updates.
        output.textures_delta.clear();
        assert!(forwarded.iter().any(|event| matches!(event,
            UiEvent::ScrollDirect { position, delta_y }
                if *position == Point::new(40.0, 30.0) && *delta_y == -2.5)));
        assert!(forwarded.iter().any(|event| matches!(event,
            UiEvent::Scroll { position, delta_y }
                if *position == Point::new(40.0, 30.0) && *delta_y == -60.0)));
        assert!(forwarded.iter().any(|event| matches!(event,
            UiEvent::Scroll { delta_y, .. } if *delta_y == 500.0)));
    }

    fn test_png(width: u32, height: u32) -> Vec<u8> {
        let mut png = Vec::new();
        let rgba = vec![80; (width * height * 4) as usize];
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&rgba, width, height, image::ExtendedColorType::Rgba8)
            .unwrap();
        png
    }

    fn raster(bytes: Vec<u8>, format: RasterImageFormat) -> RasterImage {
        use base64::Engine as _;
        use sha2::Digest as _;
        RasterImage::new(
            format!("{:x}", sha2::Sha256::digest(&bytes)),
            format,
            bytes.len() as u32,
            Arc::from(base64::engine::general_purpose::STANDARD.encode(&bytes)),
            "Imagen de prueba",
        )
        .unwrap()
    }

    #[test]
    fn image_decoder_requires_digest_format_and_bounded_dimensions() {
        let png = test_png(2, 1);
        let image = raster(png.clone(), RasterImageFormat::Png);
        let thumbnail = decode_raster_thumbnail(&image).unwrap();
        assert_eq!(thumbnail.size, [2, 1]);
        assert_eq!(thumbnail.pixels.len(), 2);

        let wrong_format = raster(png.clone(), RasterImageFormat::Jpeg);
        assert_ne!(raster_cache_key(&image), raster_cache_key(&wrong_format));
        assert!(decode_raster_thumbnail(&wrong_format).is_none());
        let wrong_digest = RasterImage::new(
            "0".repeat(64),
            RasterImageFormat::Png,
            image.size_bytes(),
            Arc::from(image.data_base64()),
            "Imagen de prueba",
        )
        .unwrap();
        assert!(decode_raster_thumbnail(&wrong_digest).is_none());

        // A narrow PNG crosses the dimension limit without a large allocation.
        let oversized = raster(
            test_png(MAX_IMAGE_SOURCE_SIDE + 1, 1),
            RasterImageFormat::Png,
        );
        assert!(decode_raster_thumbnail(&oversized).is_none());
    }

    #[test]
    fn an_image_is_decoded_away_from_the_paint_pass_and_lands_on_a_later_one() {
        let ctx = egui::Context::default();
        let image = raster(test_png(2, 1), RasterImageFormat::Png);
        let broken = raster(b"not a picture".to_vec(), RasterImageFormat::Png);
        let paint = |image: &RasterImage| {
            let mut texture = RasterTexture::Unavailable;
            let mut output = ctx.run_ui(Default::default(), |ui| texture = raster_texture(ui, image));
            output.textures_delta.clear();
            texture
        };
        assert_eq!(paint(&image), RasterTexture::Decoding, "the first pass does not wait for it");
        assert_eq!(paint(&broken), RasterTexture::Decoding);
        let settled = |image: &RasterImage| {
            for _ in 0..400 {
                let texture = paint(image);
                if texture != RasterTexture::Decoding {
                    return texture;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            RasterTexture::Decoding
        };
        assert!(matches!(settled(&image), RasterTexture::Ready(_, [2, 1])));
        assert_eq!(settled(&broken), RasterTexture::Unavailable);
    }

    #[test]
    fn surface_pixels_land_one_to_one_on_the_pixel_grid_at_every_scale() {
        for scale in [1.0_f32, 1.5, 2.0] {
            // A layout rectangle off the pixel grid, as fractional UI sizes give.
            let area = EguiRect::from_min_size(Pos2::new(10.3, 33.7), egui::vec2(300.4, 200.2));
            for extra in [-1_i32, 0, 1] {
                // Chromium rounds a fractional surface up; the frame may differ
                // from the area by a pixel and must not be stretched to it.
                let pixels = [
                    ((300.4 * scale).round() as i32 + extra) as usize,
                    ((200.2 * scale).round() as i32 + extra) as usize,
                ];
                let painted = surface_rect(area, pixels, scale, scale);
                for edge in [painted.min.x, painted.min.y, painted.max.x, painted.max.y] {
                    let physical = edge * scale;
                    assert!((physical - physical.round()).abs() < 1e-3, "{edge} at {scale}");
                }
                assert!((painted.width() * scale - pixels[0] as f32).abs() < 1e-3);
                assert!((painted.height() * scale - pixels[1] as f32).abs() < 1e-3);
                assert!((painted.min.x - area.min.x).abs() <= 0.5 / scale + 1e-4);
                assert!((painted.min.y - area.min.y).abs() <= 0.5 / scale + 1e-4);
            }
            // The scale travelled as text: 1.1 against a display at 1.0999999.
            let near = surface_rect(area, [330, 220], scale + 0.004, scale);
            assert!((near.width() * scale - 330.0).abs() < 1e-3);
        }
        // Rendered for another display, a frame keeps its logical size.
        let area = EguiRect::from_min_size(Pos2::new(8.0, 8.0), egui::vec2(640.0, 480.0));
        assert_eq!(surface_rect(area, [640, 480], 1.0, 2.0).size(), egui::vec2(640.0, 480.0));
        assert_eq!(surface_rect(area, [1280, 960], 2.0, 1.0).size(), egui::vec2(640.0, 480.0));
    }

    #[test]
    fn a_painted_surface_is_a_quad_with_one_texel_per_physical_pixel() {
        for scale in [1.0_f32, 1.5, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
            // Layout puts the surface off the pixel grid, in a box a little
            // smaller than the frame the host has for it.
            let root = limn::Element::column(vec![
                limn::Element::spacer().size(None, Some(33.7)),
                limn::Element::row(vec![
                    limn::Element::spacer().size(Some(10.3), None),
                    limn::Element::surface("page", "Página").size(Some(300.4), Some(200.2)),
                ]),
            ]);
            let frame = limn::UiTree::new(root).layout(limn::Size::new(400.0, 300.0));
            let pixels = [(300.4 * scale).ceil() as usize, (200.2 * scale).ceil() as usize];
            let mut texture = None;
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let image = vec![120; pixels[0] * pixels[1] * 3];
                assert!(present_surface(ui.ctx(), "page", SurfaceFrame::from_rgb(pixels, &image, scale).unwrap()));
                texture = surface_texture(ui, "page").map(|(id, ..)| id);
                paint_at(ui, &frame, Pos2::new(0.25, 0.4));
            });
            output.textures_delta.clear();
            assert_eq!(output.pixels_per_point, scale);
            let quads: Vec<_> = ctx
                .tessellate(output.shapes, output.pixels_per_point)
                .into_iter()
                .filter_map(|clipped| match clipped.primitive {
                    egui::epaint::Primitive::Mesh(mesh) if Some(mesh.texture_id) == texture => {
                        Some((clipped.clip_rect, mesh))
                    }
                    _ => None,
                })
                .collect();
            let [(clip, mesh)] = quads.as_slice() else {
                panic!("one textured quad at {scale}");
            };
            let bounds = mesh.calc_bounds();
            for edge in [bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y] {
                let physical = edge * scale;
                assert!((physical - physical.round()).abs() < 1e-3, "{edge} at {scale}");
            }
            assert!((bounds.width() * scale - pixels[0] as f32).abs() < 1e-3);
            assert!((bounds.height() * scale - pixels[1] as f32).abs() < 1e-3);
            let uv = mesh.vertices.iter().fold(EguiRect::NOTHING, |seen, vertex| seen.union(EguiRect::from_min_max(vertex.uv, vertex.uv)));
            assert_eq!(uv, EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
            // The pixel beyond the box is clipped away, not squeezed into it.
            assert!(clip.width() <= 300.4 + 1e-3 && clip.height() <= 200.2 + 1e-3);
            forget_surface("page");
            ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        }
    }

    #[test]
    fn a_surface_reuses_one_texture_across_frames_and_bounds_what_it_accepts() {
        let frame = |size: [usize; 2], value: u8| {
            SurfaceFrame::from_rgb(size, &vec![value; size[0] * size[1] * 3], 2.0).unwrap()
        };
        let ctx = egui::Context::default();
        // What reached the GPU in one pass: per texture, where and how large.
        let writes = |ctx: &egui::Context| {
            let mut delta = ctx.run_ui(Default::default(), |_| {}).textures_delta;
            let set: HashMap<_, Vec<_>> = delta
                .set
                .iter()
                .map(|(id, parts)| (*id, parts.iter().map(|part| (part.pos, part.image.size())).collect()))
                .collect();
            let free = delta.free.clone();
            delta.clear();
            (set, free)
        };
        writes(&ctx);
        assert!(present_surface(&ctx, "page", frame([64, 48], 10)));
        let id = SURFACE_CACHE.with(|cache| cache.borrow().entries["page"].texture.id());
        assert_eq!(writes(&ctx).0[&id], [(None, [64, 48])]);
        let allocated = ctx.tex_manager().read().num_allocated();

        // Frames of the same size are written into that texture in place.
        for value in [20, 30, 40] {
            present_surface(&ctx, "page", frame([64, 48], value));
            let (set, free) = writes(&ctx);
            assert_eq!(set[&id], [(Some([0, 0]), [64, 48])]);
            assert!(free.is_empty());
        }
        // A resized frame replaces the pixels and keeps the texture's identity.
        present_surface(&ctx, "page", frame([80, 48], 50));
        assert_eq!(writes(&ctx).0[&id], [(None, [80, 48])]);
        assert_eq!(SURFACE_CACHE.with(|cache| cache.borrow().entries["page"].texture.id()), id);
        assert_eq!(ctx.tex_manager().read().num_allocated(), allocated);

        // A frame the GPU has no texture for is refused; the last one stays.
        let side = ctx.input(|input| input.max_texture_side);
        assert!(!present_surface(&ctx, "page", frame([side + 1, 1], 60)));
        assert!(writes(&ctx).0.is_empty());

        forget_surface("page");
        assert!(writes(&ctx).1.contains(&id));
        assert_eq!(SurfaceFrame::from_rgba([2, 1], &[0; 8], 1.0).unwrap().size(), [2, 1]);
        assert!(SurfaceFrame::from_rgba([2, 1], &[0; 7], 1.0).is_none());
        assert!(SurfaceFrame::from_rgb([0, 4], &[], 1.0).is_none());
        assert!(SurfaceFrame::from_rgb([MAX_SURFACE_SIDE + 1, 1], &[], 1.0).is_none());
        assert!(SurfaceFrame::from_rgb([1, 1], &[0; 3], f32::NAN).is_none());
    }

    #[test]
    fn a_part_of_a_frame_is_written_over_the_frame_a_surface_shows_and_nowhere_else() {
        let solid = |size: [usize; 2], value: u8| {
            SurfaceFrame::from_rgb(size, &vec![value; size[0] * size[1] * 3], 2.0).unwrap()
        };
        let ctx = egui::Context::default();
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
        // Nothing is shown yet: a part has no frame to be a part of.
        assert!(!present_surface(&ctx, "page", solid([2, 2], 9).within([8, 6], [1, 1]).unwrap()));
        assert!(SURFACE_CACHE.with(|cache| cache.borrow().entries.is_empty()));
        assert!(present_surface(&ctx, "page", solid([8, 6], 10)));
        let id = SURFACE_CACHE.with(|cache| cache.borrow().entries["page"].texture.id());

        // What the GPU's texture holds: the writes of each pass, replayed in
        // the order egui hands them to a renderer. Returns where they went.
        let mut texture = egui::ColorImage::filled([0, 0], Color32::TRANSPARENT);
        let upload = |texture: &mut egui::ColorImage| {
            let mut delta = ctx.run_ui(Default::default(), |_| {}).textures_delta;
            let mut written = Vec::new();
            for write in delta.set.get(&id).into_iter().flatten() {
                let egui::ImageData::Color(image) = &write.image;
                written.push((write.pos, image.size));
                match write.pos {
                    None => *texture = (**image).clone(),
                    Some([left, top]) => {
                        for (row, pixels) in image.pixels.chunks_exact(image.size[0]).enumerate() {
                            let start = (top + row) * texture.size[0] + left;
                            texture.pixels[start..start + pixels.len()].copy_from_slice(pixels);
                        }
                    }
                }
            }
            delta.clear();
            written
        };
        let grey = |value: u8| Color32::from_gray(value);
        assert_eq!(upload(&mut texture), [(None, [8, 6])]);

        // Only the part reaches the GPU, and the rest of the frame stays.
        let part = solid([3, 2], 200).within([8, 6], [4, 3]).unwrap();
        assert_eq!((part.size(), part.part()), ([3, 2], Some(([8, 6], [4, 3]))));
        assert!(present_surface(&ctx, "page", part));
        assert_eq!(upload(&mut texture), [(Some([4, 3]), [3, 2])]);
        for (index, pixel) in texture.pixels.iter().enumerate() {
            let (x, y) = (index % 8, index / 8);
            let inside = (4..7).contains(&x) && (3..5).contains(&y);
            assert_eq!(*pixel, grey(if inside { 200 } else { 10 }), "({x}, {y})");
        }
        // Parts that touch the frame's edges fit; several in a pass all land.
        assert!(present_surface(&ctx, "page", solid([8, 1], 30).within([8, 6], [0, 5]).unwrap()));
        assert!(present_surface(&ctx, "page", solid([1, 1], 40).within([8, 6], [7, 0]).unwrap()));
        assert_eq!(upload(&mut texture), [(Some([0, 5]), [8, 1]), (Some([7, 0]), [1, 1])]);
        assert_eq!((texture[(0, 5)], texture[(7, 5)], texture[(7, 0)], texture[(6, 0)]), (grey(30), grey(30), grey(40), grey(10)));

        // A part that would stick out of its frame is not one.
        for (size, at) in [([3, 2], [6, 3]), ([3, 2], [4, 5]), ([9, 1], [0, 0]), ([1, 1], [8, 0]), ([1, 1], [usize::MAX, 0])] {
            assert!(solid(size, 1).within([8, 6], at).is_none(), "{size:?} at {at:?}");
        }
        // Nor is one shown that belongs to a frame of another size or scale
        // than the surface's, or to a surface that shows nothing.
        let before = texture.clone();
        assert!(!present_surface(&ctx, "page", solid([3, 2], 1).within([9, 6], [0, 0]).unwrap()));
        assert!(!present_surface(&ctx, "page", solid([3, 2], 1).within([4, 3], [0, 0]).unwrap()));
        let coarse = SurfaceFrame::from_rgb([1, 1], &[1; 3], 1.0).unwrap();
        assert!(!present_surface(&ctx, "page", coarse.within([8, 6], [0, 0]).unwrap()));
        assert!(!present_surface(&ctx, "other", solid([3, 2], 1).within([8, 6], [0, 0]).unwrap()));
        assert!(upload(&mut texture).is_empty());
        assert_eq!(texture, before);

        // A whole frame after parts replaces every pixel, also one of another
        // size, whose parts are then the ones that fit.
        assert!(present_surface(&ctx, "page", solid([8, 6], 77)));
        assert_eq!(upload(&mut texture), [(Some([0, 0]), [8, 6])]);
        assert!(texture.pixels.iter().all(|pixel| *pixel == grey(77)));
        assert!(present_surface(&ctx, "page", solid([4, 4], 90)));
        assert!(!present_surface(&ctx, "page", solid([1, 1], 1).within([8, 6], [0, 0]).unwrap()));
        assert!(present_surface(&ctx, "page", solid([1, 1], 5).within([4, 4], [3, 3]).unwrap()));
        assert_eq!(upload(&mut texture), [(None, [4, 4]), (Some([3, 3]), [1, 1])]);
        assert_eq!((texture.size, texture[(3, 3)], texture[(2, 3)]), ([4, 4], grey(5), grey(90)));

        // Forgotten, the surface has no frame again.
        forget_surface("page");
        assert!(!present_surface(&ctx, "page", solid([1, 1], 1).within([4, 4], [0, 0]).unwrap()));
        ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
    }

    #[test]
    fn effort_particles_are_deterministic_and_flow_leftward() {
        let track = EguiRect::from_min_size(Pos2::new(20.0, 30.0), egui::vec2(220.0, 28.0));
        let first = effort_particle(4, 1.25, track);
        assert_eq!(first, effort_particle(4, 1.25, track));
        assert!((track.left() - 3.0..=track.right() + 3.0).contains(&first.0.x));
        assert_ne!(first, effort_particle(4, 1.5, track));
        assert_eq!(effort_color(10, 20, 30, 200, 0.5).a(), 100);
    }

    #[test]
    fn bundled_bootstrap_icons_rasterize_as_monochrome_masks() {
        let icons = [
            Icon::PanelLeft,
            Icon::PanelLeftOpen,
            Icon::PanelLeftClose,
            Icon::PanelBottom,
            Icon::PanelRight,
            Icon::PanelRightOpen,
            Icon::PanelRightClose,
            Icon::Add,
            Icon::Minus,
            Icon::Check,
            Icon::InfoCircle,
            Icon::CheckCircle,
            Icon::WarningTriangle,
            Icon::ErrorCircle,
            Icon::Settings,
            Icon::Chat,
            Icon::Folder,
            Icon::Laptop,
            Icon::GitBranch,
            Icon::Grid,
            Icon::Hash,
            Icon::Code,
            Icon::ArrowUp,
            Icon::ArrowDown,
            Icon::ArrowLeft,
            Icon::ArrowRight,
            Icon::ChevronDown,
            Icon::ChevronUp,
            Icon::ChevronRight,
            Icon::Search,
            Icon::Stop,
            Icon::StopFill,
            Icon::Play,
            Icon::Pause,
            Icon::History,
            Icon::Close,
            Icon::CloseBold,
            Icon::CloseCircle,
            Icon::Refresh,
            Icon::Menu,
            Icon::More,
            Icon::Pin,
            Icon::Archive,
            Icon::ArchiveUp,
            Icon::Trash,
            Icon::Edit,
            Icon::Copy,
            Icon::Unread,
            Icon::Filter,
            Icon::Sort,
            Icon::Collapse,
            Icon::Expand,
            Icon::Compact,
            Icon::Fullscreen,
            Icon::Clock,
            Icon::Bell,
            Icon::BellFill,
            Icon::BellSlash,
            Icon::SkipBack,
            Icon::SkipForward,
            Icon::PauseFill,
            Icon::Shuffle,
            Icon::Repeat,
            Icon::RepeatOne,
            Icon::Volume,
            Icon::VolumeMuted,
            Icon::Heart,
            Icon::HeartFill,
            Icon::MusicNote,
            Icon::Playlist,
            Icon::Home,
            Icon::Library,
            Icon::Sliders,
            Icon::MicrophoneMuted,
            Icon::Headphones,
            Icon::Emoji,
            Icon::Reply,
            Icon::At,
            Icon::Phone,
            Icon::HangUp,
            Icon::Screen,
            Icon::VideoOff,
            Icon::Lock,
            Icon::Neutral,
            Icon::Invite,
            Icon::Leave,
            Icon::DragHandle,
            Icon::FolderFill,
            Icon::PlayFill,
            Icon::GridFill,
            Icon::SettingsFill,
            Icon::PaletteFill,
            Icon::LaptopFill,
            Icon::TerminalFill,
            Icon::KeyboardFill,
            Icon::ChatFill,
            Icon::UsersFill,
            Icon::PuzzleFill,
            Icon::ServerFill,
            Icon::BookFill,
            Icon::ClockFill,
            Icon::BoltFill,
            Icon::ArchiveFill,
            Icon::ChartBarFill,
        ];
        for icon in icons {
            let source = icon_svg(icon);
            // Bootstrap Icons: a 16 px square drawn with the current color.
            assert!(source.contains("class=\"bi bi-"), "not a Bootstrap icon: {icon:?}");
            assert!(source.contains("viewBox=\"0 0 16 16\""), "{icon:?}");
            assert!(source.contains("fill=\"currentColor\""), "{icon:?}");
            let key = IconKey { icon, pixels: 48 };
            let svg = monochrome_svg(source);
            assert!(svg.contains("#FFFFFF"), "missing white mask: {icon:?}");
            assert!(!svg.contains("currentColor"), "unresolved color: {icon:?}");
            let image = raster_icon(key).unwrap_or_else(|| panic!("invalid SVG: {icon:?}"));
            assert_eq!(image.size, [48, 48]);
            assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
            assert!(
                image
                    .pixels
                    .iter()
                    .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()),
                "mask contains chromatic pixels: {icon:?}"
            );
        }
    }

    #[test]
    fn github_mark_rasterizes_as_a_tintable_monochrome_mask() {
        let source = icon_svg(Icon::Github);
        assert!(source.contains("viewBox=\"0 0 16 16\""));
        let image = raster_icon(IconKey {
            icon: Icon::Github,
            pixels: 48,
        })
        .expect("GitHub mark should render");
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
        assert!(image
            .pixels
            .iter()
            .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
    }

    #[test]
    fn semantic_settings_icons_rasterize_at_native_sizes_as_distinct_masks() {
        for pixels in [16, 32] {
            let mut masks = Vec::new();
            for icon in [Icon::Palette, Icon::Users, Icon::Puzzle, Icon::Book, Icon::Server, Icon::Bolt, Icon::ChartBar, Icon::Keyboard] {
                let image = raster_icon(IconKey { icon, pixels }).unwrap_or_else(|| panic!("invalid semantic SVG: {icon:?}"));
                assert_eq!(image.size, [usize::from(pixels); 2]);
                assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
                assert!(image.pixels.iter().all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
                let alpha = image.pixels.iter().map(|pixel| pixel.a()).collect::<Vec<_>>();
                assert!(!masks.contains(&alpha), "semantic icon reused another shape: {icon:?}");
                masks.push(alpha);
            }
        }
    }

    #[test]
    fn the_platform_is_told_each_control_with_its_role_name_and_state() {
        use limn::{Access, Button, Component, ComponentPalette, Element, Role, Size, UiTree};
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(Element::column(vec![
            Button::new("save", "Save changes", palette).render(),
            Element::clickable("wifi", Element::text("Wi-Fi")).accessible(Access::new(Role::Switch).toggled(true)),
            Element::clickable("mute", Element::text("Mute"))
                .accessible(Access::new(Role::MenuItemCheckBox).toggled(false)),
            Element::clickable("all", Element::text("All"))
                .accessible(Access::new(Role::MenuItemRadio).toggled(true)),
        ]))
        .layout(Size::new(320.0, 200.0));
        let context = egui::Context::default();
        context.enable_accesskit();
        let origin = egui::pos2(10.0, 20.0);
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))),
                ..Default::default()
            },
            |ui| paint_at(ui, &frame, origin),
        );
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.take().expect("a tree for the platform");
        let node = |id: &str| {
            let wanted = access_id(id).accesskit_id();
            update.nodes.iter().find(|(node, _)| *node == wanted).map(|(_, node)| node).unwrap_or_else(|| panic!("no node {id}"))
        };
        let save = node("save");
        assert_eq!(save.role(), egui::accesskit::Role::Button);
        assert_eq!(save.label(), Some("Save changes"));
        assert!(save.supports_action(egui::accesskit::Action::Click));
        // Where it is on the window, not in the frame.
        let placed = frame.access.iter().find(|node| node.id == "save").unwrap().rect;
        let bounds = save.bounds().unwrap();
        assert_eq!((bounds.x0 as f32, bounds.y0 as f32), (placed.x + origin.x, placed.y + origin.y));
        let wifi = node("wifi");
        assert_eq!(wifi.role(), egui::accesskit::Role::Switch);
        assert_eq!(wifi.label(), Some("Wi-Fi"));
        assert_eq!(wifi.toggled(), Some(egui::accesskit::Toggled::True));
        // A menu's check box and radio item are what they are, on or off.
        use egui::accesskit::{Role as Platform, Toggled};
        let (mute, all) = (node("mute"), node("all"));
        assert_eq!(mute.role(), Platform::MenuItemCheckBox);
        assert_eq!(mute.toggled(), Some(Toggled::False));
        assert_eq!(all.role(), Platform::MenuItemRadio);
        assert_eq!(all.toggled(), Some(Toggled::True));
    }

    #[test]
    fn an_icon_pack_replaces_the_icons_it_has_and_leaves_the_rest() {
        const WIDE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"><rect width="200" height="100" fill="currentColor"/></svg>"#;
        let raster = |icon| raster_icon(IconKey { icon, pixels: 16 }).unwrap();
        let (search, trash) = (raster(Icon::Search), raster(Icon::Trash));
        set_icon_pack(|icon| (icon == Icon::Search).then_some(WIDE));
        // The pack's artwork is fitted like an application's own.
        assert!(raster(Icon::Search) == raster(Icon::Custom(WIDE)));
        assert!(raster(Icon::Search) != search);
        assert!(raster(Icon::Trash) == trash);
    }

    #[test]
    fn custom_artwork_is_contained_centered_and_tintable_whatever_its_shape() {
        // Twice as wide as tall, drawn with the current color.
        const WIDE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100"><rect width="200" height="100" fill="currentColor"/></svg>"#;
        for pixels in [16, 32] {
            let image = raster_icon(IconKey { icon: Icon::Custom(WIDE), pixels }).unwrap();
            let side = usize::from(pixels);
            let mut bounds = (side, side, 0, 0);
            for (index, pixel) in image.pixels.iter().enumerate() {
                if pixel.a() < 32 {
                    continue;
                }
                let (x, y) = (index % side, index / side);
                bounds = (bounds.0.min(x), bounds.1.min(y), bounds.2.max(x), bounds.3.max(y));
            }
            let width = bounds.2 + 1 - bounds.0;
            let height = bounds.3 + 1 - bounds.1;
            assert_eq!(width, side, "fills the square's width at {pixels}px");
            assert!(width.abs_diff(height * 2) <= 2, "kept its shape: {bounds:?}");
            assert!((bounds.1 + bounds.3 + 1).abs_diff(side) <= 1, "centered: {bounds:?}");
            // A white mask, so the theme's tint decides the color.
            assert!(image
                .pixels
                .iter()
                .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
            assert_eq!(image.pixels[0].a(), 0, "nothing painted outside the artwork");
        }
    }

    #[test]
    fn send_icon_rasterizes() {
        let source = icon_svg(Icon::Send);
        assert!(source.contains("viewBox=\"0 0 16 16\""));
        let image = raster_icon(IconKey {
            icon: Icon::Send,
            pixels: 48,
        })
        .expect("send arrow SVG");
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
    }

    #[test]
    fn linear_icon_mask_preserves_stroke_coverage() {
        let image = raster_icon(IconKey {
            icon: Icon::PanelLeft,
            pixels: 48,
        })
        .expect("panel-left SVG should render");
        assert!(image.pixels.iter().any(|pixel| pixel.a() == 255));
        assert!(image
            .pixels
            .iter()
            .any(|pixel| (1..255).contains(&pixel.a())));
        assert!(image.pixels.iter().any(|pixel| pixel.a() == 0));
    }

    #[test]
    fn icon_texture_and_destination_share_physical_pixel_grid() {
        let area = EguiRect::from_min_size(Pos2::new(11.3, 20.7), egui::vec2(14.0, 14.0));
        let (aligned, pixels) = aligned_icon_rect(area, 1.25).expect("valid icon rectangle");
        assert_eq!(pixels, 18);
        for edge in [aligned.min.x, aligned.min.y, aligned.max.x, aligned.max.y] {
            let physical = edge * 1.25;
            assert!((physical - physical.round()).abs() < 0.0001);
        }
        assert!((aligned.center().x - area.center().x).abs() <= 0.8);
        assert!((aligned.center().y - area.center().y).abs() <= 0.8);
        assert!(aligned_icon_rect(area, 0.0).is_none());
    }

    #[test]
    fn canonical_logical_icon_uses_current_display_scale() {
        let area = EguiRect::from_min_size(Pos2::new(3.2, 4.4), egui::Vec2::splat(limn::ICON_SIZE));
        // A standalone large icon paints at its own side, up to the cap.
        let side = |points: f32| {
            let area = EguiRect::from_min_size(Pos2::new(3.2, 4.4), egui::Vec2::splat(points));
            aligned_icon_rect(area, 1.0).unwrap().1
        };
        assert_eq!((side(64.0), side(500.0)), (64, 128));
        for (scale, expected_pixels) in [(1.0, 16), (1.25, 20), (1.5, 24), (2.0, 32)] {
            let (aligned, pixels) = aligned_icon_rect(area, scale).unwrap();
            assert_eq!(pixels, expected_pixels);
            assert!((aligned.width() * scale - f32::from(expected_pixels)).abs() < 0.0001);
            assert!((aligned.height() * scale - f32::from(expected_pixels)).abs() < 0.0001);
        }
    }

    #[test]
    fn spinner_arc_is_open_rotates_and_stays_inside_its_square() {
        let area = EguiRect::from_min_size(Pos2::new(10.0, 20.0), egui::Vec2::splat(32.0));
        let initial = spinner_arc_points(area, 0.0, 2.4);
        let rotated = spinner_arc_points(area, std::f32::consts::FRAC_PI_2, 2.4);
        assert!(initial.len() >= 17);
        assert_ne!(initial.first(), initial.last());
        assert_ne!(initial.first(), rotated.first());
        for point in initial.into_iter().chain(rotated) {
            assert!(point.x > area.left() && point.x < area.right());
            assert!(point.y > area.top() && point.y < area.bottom());
        }
    }
}
