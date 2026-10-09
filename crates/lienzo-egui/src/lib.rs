use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect as EguiRect, Stroke, StrokeKind,
};
use lienzo::{
    Color, DrawCommand, FontFamily, Frame, Icon, Key, Point, RasterImage, RasterImageFormat, Rect,
    SpinnerVariant, UiEvent, ICON_SIZE,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;

mod provider_icons;

/// Colors of the effort slider, the only command without colors of its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffortPalette {
    pub input: Color,
    pub foreground_subtle: Color,
    pub blue: Color,
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

enum RasterCacheEntry {
    Ready(egui::TextureHandle),
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

fn raster_texture(ui: &egui::Ui, image: &RasterImage) -> Option<(egui::TextureId, [usize; 2])> {
    RASTER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.context.as_ref() != Some(ui.ctx()) {
            cache.entries.clear();
            cache.order.clear();
            cache.context = Some(ui.ctx().clone());
        }
        let key = raster_cache_key(image);
        if let Some(entry) = cache.entries.get(&key) {
            return match entry {
                RasterCacheEntry::Ready(texture) => Some((texture.id(), texture.size())),
                RasterCacheEntry::Unavailable => None,
            };
        }
        let entry =
            decode_raster_thumbnail(image).map_or(RasterCacheEntry::Unavailable, |bitmap| {
                RasterCacheEntry::Ready(ui.ctx().load_texture(
                    format!("user-image:{}", image.sha256()),
                    bitmap,
                    egui::TextureOptions::LINEAR,
                ))
            });
        let result = match &entry {
            RasterCacheEntry::Ready(texture) => Some((texture.id(), texture.size())),
            RasterCacheEntry::Unavailable => None,
        };
        cache.entries.insert(key.clone(), entry);
        cache.order.push_back(key);
        let full = image.is_full_resolution();
        let limit = if full {
            FULL_IMAGE_TEXTURE_CACHE_ENTRIES
        } else {
            IMAGE_TEXTURE_CACHE_ENTRIES
        };
        while cache.order.iter().filter(|key| key.2 == full).count() > limit {
            if let Some(index) = cache.order.iter().position(|key| key.2 == full) {
                let oldest = cache.order.remove(index).expect("cache entry index");
                cache.entries.remove(&oldest);
            }
        }
        result
    })
}

fn fit_image(area: EguiRect, pixels: [usize; 2]) -> EguiRect {
    let width = pixels[0] as f32;
    let height = pixels[1] as f32;
    let scale = (area.width() / width).min(area.height() / height).min(1.0);
    let size = egui::vec2(width * scale, height * scale);
    EguiRect::from_center_size(area.center(), size)
}

fn icon_svg(icon: Icon) -> &'static str {
    match icon {
        Icon::PanelLeft => include_str!("../assets/solar/panel-left.svg"),
        Icon::PanelLeftOpen => include_str!("../assets/solar/panel-left-open.svg"),
        Icon::PanelLeftClose => include_str!("../assets/solar/panel-left-close.svg"),
        Icon::PanelBottom => include_str!("../assets/solar/panel-bottom.svg"),
        Icon::PanelRight => include_str!("../assets/solar/panel-right.svg"),
        Icon::PanelRightOpen => include_str!("../assets/solar/panel-right-open.svg"),
        Icon::PanelRightClose => include_str!("../assets/solar/panel-right-close.svg"),
        Icon::Add => include_str!("../assets/solar/add.svg"),
        Icon::Microphone => include_str!("../assets/openide/microphone.svg"),
        Icon::Text => include_str!("../assets/solar/text-square.svg"),
        Icon::Image => include_str!("../assets/solar/gallery.svg"),
        Icon::Audio => include_str!("../assets/solar/soundwave.svg"),
        Icon::Video => include_str!("../assets/solar/videocamera.svg"),
        Icon::File => include_str!("../assets/solar/file-text.svg"),
        Icon::FileType(file) => file.svg(),
        Icon::Infinity => include_str!("../assets/openide/infinity.svg"),
        Icon::Paperclip => include_str!("../assets/openide/paperclip.svg"),
        Icon::Target => include_str!("../assets/openide/target.svg"),
        Icon::Plan => include_str!("../assets/openide/plan.svg"),
        Icon::Brain => include_str!("../assets/openide/brain.svg"),
        Icon::ContextRing => include_str!("../assets/openide/context-ring.svg"),
        Icon::Check => include_str!("../assets/solar/check.svg"),
        Icon::InfoCircle => include_str!("../assets/solar/info-circle.svg"),
        Icon::CheckCircle => include_str!("../assets/solar/check-circle.svg"),
        Icon::WarningTriangle => include_str!("../assets/solar/danger-triangle.svg"),
        Icon::ErrorCircle => include_str!("../assets/solar/close-circle.svg"),
        Icon::Settings => include_str!("../assets/solar/settings.svg"),
        Icon::Github => include_str!("../assets/octicons/github-mark.svg"),
        Icon::Codex
        | Icon::OpenAi
        | Icon::OpenRouter
        | Icon::OpenCode
        | Icon::Nvidia
        | Icon::MiniMax
        | Icon::Zai
        | Icon::Anthropic
        | Icon::Groq
        | Icon::Mistral
        | Icon::Xai
        | Icon::DeepSeek => crate::provider_icons::svg_for_icon(icon)
            .expect("every provider icon has embedded artwork"),
        Icon::Chat => include_str!("../assets/solar/chat-round-dots.svg"),
        Icon::Folder => include_str!("../assets/solar/folder.svg"),
        Icon::Laptop => include_str!("../assets/solar/laptop-minimalistic.svg"),
        Icon::Globe => include_str!("../assets/openide/browser.svg"),
        Icon::GitBranch => include_str!("../assets/solar/git-branch.svg"),
        Icon::Grid => include_str!("../assets/solar/widget.svg"),
        Icon::Palette => include_str!("../assets/openide/palette.svg"),
        Icon::Users => include_str!("../assets/openide/users.svg"),
        Icon::Puzzle => include_str!("../assets/openide/puzzle.svg"),
        Icon::Book => include_str!("../assets/openide/book.svg"),
        Icon::Server => include_str!("../assets/openide/server.svg"),
        Icon::Bolt => include_str!("../assets/openide/bolt.svg"),
        Icon::ChartBar => include_str!("../assets/openide/chart-bar.svg"),
        Icon::Keyboard => include_str!("../assets/openide/keyboard.svg"),
        Icon::Hash => include_str!("../assets/solar/hashtag.svg"),
        Icon::Code => include_str!("../assets/solar/code.svg"),
        Icon::Terminal => include_str!("../assets/openide/terminal.svg"),
        Icon::ArrowUp => include_str!("../assets/solar/arrow-up.svg"),
        Icon::Send => include_str!("../assets/openide/send-arrow.svg"),
        Icon::ArrowDown => include_str!("../assets/solar/arrow-down.svg"),
        Icon::ArrowLeft => include_str!("../assets/solar/arrow-left.svg"),
        Icon::ArrowRight => include_str!("../assets/solar/arrow-right.svg"),
        Icon::ChevronDown => include_str!("../assets/solar/alt-arrow-down.svg"),
        Icon::ChevronRight => include_str!("../assets/solar/alt-arrow-right.svg"),
        Icon::Search => include_str!("../assets/solar/magnifier.svg"),
        Icon::Replace => include_str!("../assets/openide/replace.svg"),
        Icon::ReplaceAll => include_str!("../assets/openide/replace-all.svg"),
        Icon::Stop => include_str!("../assets/solar/stop.svg"),
        Icon::DebugStart => include_str!("../assets/openide/debug-start.svg"),
        Icon::DebugPause => include_str!("../assets/openide/debug-pause.svg"),
        Icon::DebugStepOver => include_str!("../assets/openide/debug-step-over.svg"),
        Icon::DebugStepInto => include_str!("../assets/openide/debug-step-into.svg"),
        Icon::DebugStepOut => include_str!("../assets/openide/debug-step-out.svg"),

        Icon::History => include_str!("../assets/solar/history.svg"),
        Icon::Close => include_str!("../assets/solar/close.svg"),
        Icon::CloseBold => include_str!("../assets/solar/close-bold.svg"),
        Icon::CloseCircle => include_str!("../assets/solar/close-circle.svg"),
        Icon::Refresh => include_str!("../assets/solar/refresh.svg"),
        Icon::Menu => include_str!("../assets/solar/hamburger-menu.svg"),
        Icon::More => include_str!("../assets/menu-dots-filled.svg"),
        Icon::Pin => include_str!("../assets/solar/pin.svg"),
        Icon::Archive => include_str!("../assets/solar/archive.svg"),
        Icon::ArchiveUp => include_str!("../assets/solar/archive-up.svg"),
        Icon::Trash => include_str!("../assets/solar/trash-bin-minimalistic.svg"),
        Icon::Edit => include_str!("../assets/solar/pen.svg"),
        Icon::Unread => include_str!("../assets/solar/chat-unread.svg"),
        Icon::Filter => include_str!("../assets/solar/filter.svg"),
        Icon::Sort => include_str!("../assets/solar/sort.svg"),
        Icon::Collapse => include_str!("../assets/solar/minimize.svg"),
        Icon::Expand => include_str!("../assets/solar/maximize.svg"),
        Icon::Clock => include_str!("../assets/solar/clock-circle.svg"),
    }
}

fn monochrome_svg(source: &str) -> String {
    // Solar Linear draws with currentColor. Rasterize a white coverage mask
    // once per icon and physical size, then tint it per DrawCommand. This
    // keeps theme and hover colors out of the texture cache, like WarpUI's
    // monochrome Icon element does for its size-cached image.
    source.replace("currentColor", "#FFFFFF")
}

fn raster_icon(key: IconKey) -> Option<egui::ColorImage> {
    let source = if matches!(key.icon, Icon::FileType(_)) {
        std::borrow::Cow::Borrowed(icon_svg(key.icon))
    } else {
        std::borrow::Cow::Owned(monochrome_svg(icon_svg(key.icon)))
    };
    let tree = resvg::usvg::Tree::from_str(&source, &resvg::usvg::Options::default()).ok()?;
    let pixels = u32::from(key.pixels);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(pixels, pixels)?;
    let transform = if crate::provider_icons::svg_for_icon(key.icon).is_some() {
        // Provider marks have different source aspect ratios. Contain the
        // artwork in the same 16px budget without stretching NVIDIA or Z.ai.
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
            format!("lienzo-icon:{:?}:{}", key.icon, key.pixels),
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
fn aligned_icon_rect(area: EguiRect, pixels_per_point: f32) -> Option<(EguiRect, u16)> {
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    // The tree reserves a 16 point box. Keep that upper bound when painting
    // DrawCommands supplied by another caller of the backend, too.
    let side = area.width().min(area.height()).min(ICON_SIZE);
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

pub fn paint(ui: &egui::Ui, frame: &Frame) {
    paint_at(ui, frame, ui.max_rect().min);
}

/// Paint a tree laid out in its own coordinate space at a window position.
pub fn paint_at(ui: &egui::Ui, frame: &Frame, origin: Pos2) {
    let mut clips = vec![ui.clip_rect()];
    for command in &frame.commands {
        match command {
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
                        "lienzo-raster-image",
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
                if let Some((texture, pixels)) = raster_texture(ui, image) {
                    painter.add(
                        egui::epaint::RectShape::filled(
                            fit_image(area, pixels),
                            CornerRadius::same(radius.clamp(0.0, 255.0) as u8),
                            Color32::from_white_alpha((opacity.clamp(0.0, 1.0) * 255.0) as u8),
                        )
                        .with_texture(
                            texture,
                            EguiRect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        ),
                    );
                } else {
                    painter.rect_filled(area, CornerRadius::same(6), Color32::from_gray(37));
                    painter.text(
                        area.center(),
                        Align2::CENTER_CENTER,
                        "Vista previa no disponible",
                        FontId::proportional(12.0),
                        Color32::from_gray(190),
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
                egui::Event::Text(value) => {
                    if !(enter_pressed && matches!(value.as_str(), "\n" | "\r" | "\r\n")) {
                        events.push(UiEvent::Text(value.clone()));
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
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
    fn effort_particles_are_deterministic_and_flow_leftward() {
        let track = EguiRect::from_min_size(Pos2::new(20.0, 30.0), egui::vec2(220.0, 28.0));
        let first = effort_particle(4, 1.25, track);
        assert_eq!(first, effort_particle(4, 1.25, track));
        assert!((track.left() - 3.0..=track.right() + 3.0).contains(&first.0.x));
        assert_ne!(first, effort_particle(4, 1.5, track));
        assert_eq!(effort_color(10, 20, 30, 200, 0.5).a(), 100);
    }

    #[test]
    fn bundled_solar_linear_icons_rasterize_as_monochrome_masks() {
        let icons = [
            Icon::PanelLeft,
            Icon::PanelLeftOpen,
            Icon::PanelLeftClose,
            Icon::PanelBottom,
            Icon::PanelRight,
            Icon::PanelRightOpen,
            Icon::PanelRightClose,
            Icon::Add,
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
            Icon::ChevronRight,
            Icon::Search,
            Icon::Stop,
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
            Icon::Unread,
            Icon::Filter,
            Icon::Sort,
            Icon::Collapse,
            Icon::Expand,
            Icon::Clock,
        ];
        for icon in icons {
            let source = icon_svg(icon);
            assert!(
                source.contains("viewBox=\"0 0 24 24\""),
                "Solar source needs a canonical square viewBox: {icon:?}"
            );
            if icon == Icon::More {
                assert!(
                    source.contains("fill=\"currentColor\""),
                    "menu dots must be solid"
                );
            } else if icon == Icon::CloseBold {
                assert!(source.contains("-bold"), "modal close must use the bold X");
            } else {
                assert!(source.contains("-linear"), "wrong Solar style: {icon:?}");
            }
            assert!(!source.contains("duotone"), "wrong Solar style: {icon:?}");
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
    fn provider_marks_rasterize_as_tintable_monochrome_masks() {
        for icon in crate::provider_icons::PROVIDER_ICONS {
            let source = icon_svg(icon);
            assert!(source.contains("viewBox="), "{icon:?}");
            assert!(source.contains("fill=\"currentColor\""), "{icon:?}");
            let image = raster_icon(IconKey { icon, pixels: 48 })
                .unwrap_or_else(|| panic!("invalid provider SVG: {icon:?}"));
            assert_eq!(image.size, [48, 48]);
            assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
            assert!(image
                .pixels
                .iter()
                .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
        }
    }

    #[test]
    fn openai_mark_uses_the_native_icon_budget_instead_of_upstream_empty_margins() {
        for pixels in [16, 32] {
            let image = raster_icon(IconKey {
                icon: Icon::OpenAi,
                pixels,
            }).unwrap();
            let side = usize::from(pixels);
            let mut bounds = (side, side, 0, 0);
            for (index, pixel) in image.pixels.iter().enumerate() {
                if pixel.a() < 32 {
                    continue;
                }
                let (x, y) = (index % side, index / side);
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
            let width = bounds.2 + 1 - bounds.0;
            let height = bounds.3 + 1 - bounds.1;
            assert!(
                width >= side - 2 && height >= side - 2,
                "OpenAI underfills its native icon budget at {pixels}px: {bounds:?}"
            );
            assert!(
                width.abs_diff(height) <= 1,
                "OpenAI must preserve its proportions"
            );
        }
    }

    #[test]
    fn provider_marks_fit_native_budget_without_squashing_non_square_artwork() {
        for pixels in [16, 32] {
            for (icon, expected_ratio) in [
                (Icon::Nvidia, 351.46 / 258.785),
                (Icon::OpenCode, 256.0 / 320.0),
            ] {
                let image = raster_icon(IconKey { icon, pixels }).unwrap();
                let side = usize::from(pixels);
                let mut bounds = (side, side, 0, 0);
                for (index, pixel) in image.pixels.iter().enumerate() {
                    if pixel.a() < 32 {
                        continue;
                    }
                    let (x, y) = (index % side, index / side);
                    bounds.0 = bounds.0.min(x);
                    bounds.1 = bounds.1.min(y);
                    bounds.2 = bounds.2.max(x);
                    bounds.3 = bounds.3.max(y);
                }
                let width = (bounds.2 + 1 - bounds.0) as f32;
                let height = (bounds.3 + 1 - bounds.1) as f32;
                assert!(
                    (width - height * expected_ratio).abs() <= 2.0,
                    "{icon:?} distorted at {pixels}px: {bounds:?}"
                );
                assert!(
                    (bounds.0 + bounds.2 + 1).abs_diff(side) <= 1,
                    "{icon:?} not centered horizontally: {bounds:?}"
                );
                assert!(
                    (bounds.1 + bounds.3 + 1).abs_diff(side) <= 1,
                    "{icon:?} not centered vertically: {bounds:?}"
                );
                // A theme tint must never reveal an opaque app-tile background.
                assert_eq!(image.pixels[0].a(), 0);
                assert_eq!(image.pixels[side * side - 1].a(), 0);
            }
        }
    }

    #[test]
    fn send_arrow_rasterizes_with_a_centered_stroke() {
        let source = icon_svg(Icon::Send);
        assert!(source.contains("viewBox=\"0 0 24 24\""));
        let image = raster_icon(IconKey {
            icon: Icon::Send,
            pixels: 48,
        })
        .expect("send arrow SVG");
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
    }

    #[test]
    fn original_icons_rasterize_as_tintable_monochrome_masks() {
        for icon in [
            Icon::Microphone,
            Icon::Infinity,
            Icon::Paperclip,
            Icon::Target,
            Icon::Plan,
            Icon::Brain,
            Icon::ContextRing,
            Icon::Replace,
            Icon::ReplaceAll,
        ] {
            let source = icon_svg(icon);
            assert!(source.contains("viewBox=\"0 0 24 24\""), "{icon:?}");
            let image = raster_icon(IconKey { icon, pixels: 48 })
                .unwrap_or_else(|| panic!("invalid original SVG: {icon:?}"));
            assert_eq!(image.size, [48, 48]);
            assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
            assert!(image
                .pixels
                .iter()
                .all(|pixel| pixel.r() == pixel.g() && pixel.g() == pixel.b()));
        }
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
        let area = EguiRect::from_min_size(Pos2::new(3.2, 4.4), egui::Vec2::splat(32.0));
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
