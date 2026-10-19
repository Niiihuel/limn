//! Draws the glyphs limn lays out, from an atlas of its own. Glyphs are
//! rasterized as epaint rasterizes its own (skrifa outlines, the same
//! hinting, vello_cpu, the theme's coverage curve), so text limn lays out
//! looks as egui's does.

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::sync::Arc;

use ahash::HashMap;

use eframe::egui;
use egui::epaint::text::HintingTarget;
use egui::epaint::FontColorTransferFunction;
use egui::{Color32, ColorImage, Pos2, TextureHandle, TextureOptions};
use limn::{Glyph, GlyphRun};
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::{DrawSettings, HintingInstance, OutlinePen};
use skrifa::{GlyphId, MetadataProvider as _};
use vello_cpu::kurbo::{self, Affine, BezPath, Shape as _};

/// A page's side in pixels. Pages never grow: a full one is followed by
/// another, so the texture coordinates of what is already drawn stay valid.
const PAGE: usize = 1024;
/// Past this many pages the atlas starts again empty.
const MAX_PAGES: usize = 8;
/// Clear space around each glyph, so sampling never bleeds into a neighbour.
const GUTTER: usize = 1;

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct GlyphKey {
    font: u64,
    index: u32,
    glyph: u32,
    /// The size in physical pixels, as bits.
    pixels: u32,
    /// The quarter of a pixel the glyph starts at.
    bin: u8,
    coords: u64,
    embolden: bool,
    skew: u32,
}

#[derive(Clone, Copy)]
struct Placed {
    page: usize,
    min: [usize; 2],
    size: [usize; 2],
    /// From the pen's position on the baseline to the image's top-left
    /// corner, in physical pixels.
    offset: egui::Vec2,
}

struct Page {
    texture: TextureHandle,
    /// The shelf being filled: its top, its height, and how far along it is.
    shelf: [usize; 3],
}

/// A run drawn before: the same glyphs at the same place in pixels.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct MeshKey {
    glyphs: usize,
    x: u32,
    y: u32,
    pixels_per_point: u32,
    ink: [u8; 4],
}

struct DrawnRun {
    /// Held, so no other run takes this one's address while it is kept.
    _glyphs: Arc<[Glyph]>,
    meshes: Vec<Arc<egui::Mesh>>,
}

#[derive(Default)]
struct Atlas {
    context: Option<egui::Context>,
    /// What the glyphs were drawn with; a change draws them again.
    style: Option<(FontColorTransferFunction, bool)>,
    pages: Vec<Page>,
    /// `None` for a glyph with nothing to draw, such as a space.
    glyphs: HashMap<GlyphKey, Option<Placed>>,
    hinting: HashMap<(u64, u32, u32, u64), HintingInstance>,
    /// Runs drawn this frame and the one before, reused while they stay put,
    /// as egui reuses a galley.
    drawn: HashMap<MeshKey, DrawnRun>,
    drawn_before: HashMap<MeshKey, DrawnRun>,
    frame: u64,
    /// How many times the pages were started again.
    restarts: u64,
}

thread_local! {
    // Textures belong to an egui context on this UI thread, as the icons do.
    static ATLAS: RefCell<Atlas> = RefCell::new(Atlas::default());
}

/// What every run of a frame is drawn with, read once a frame.
#[derive(Clone, Copy)]
pub(crate) struct Display {
    pub(crate) pixels_per_point: f32,
    pub(crate) options: egui::epaint::text::TextOptions,
}

impl Display {
    pub(crate) fn of(ctx: &egui::Context) -> Self {
        Self { pixels_per_point: ctx.pixels_per_point(), options: ctx.global_style().visuals.text_options }
    }
}

/// Draw `run`, laid out at `origin` in the window, through `painter`.
pub(crate) fn paint(painter: &egui::Painter, run: &GlyphRun, origin: Pos2, display: Display) {
    let Display { pixels_per_point, options } = display;
    let ink = Color32::from_rgba_unmultiplied(run.color.r, run.color.g, run.color.b, run.color.a);
    let pixels = run.size * pixels_per_point;
    // The same for every glyph of the run.
    let base = GlyphKey {
        font: run.font.id(),
        index: run.font.index(),
        glyph: 0,
        pixels: pixels.to_bits(),
        bin: 0,
        coords: ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&run.coords),
        embolden: run.embolden,
        skew: run.skew.unwrap_or(0.0).to_bits(),
    };
    ATLAS.with(|atlas| {
        let mut atlas = atlas.borrow_mut();
        atlas.attach(painter.ctx(), (options.color_transfer_function, options.font_hinting));
        let frame = painter.ctx().cumulative_frame_nr();
        if frame != atlas.frame {
            atlas.drawn_before = std::mem::take(&mut atlas.drawn);
            atlas.frame = frame;
        }
        let place = MeshKey {
            glyphs: Arc::as_ptr(&run.glyphs).cast::<()>() as usize,
            x: ((origin.x + run.origin.x) * pixels_per_point).to_bits(),
            y: ((origin.y + run.origin.y) * pixels_per_point).to_bits(),
            pixels_per_point: pixels_per_point.to_bits(),
            ink: ink.to_array(),
        };
        let kept = match atlas.drawn.get(&place) {
            Some(kept) => Some(kept.meshes.clone()),
            None => atlas.drawn_before.remove(&place).map(|kept| {
                let meshes = kept.meshes.clone();
                atlas.drawn.insert(place, kept);
                meshes
            }),
        };
        if let Some(meshes) = kept {
            for mesh in meshes {
                painter.add(egui::Shape::Mesh(mesh));
            }
            return;
        }
        let restarts = atlas.restarts;
        // Read only when a glyph is not in the atlas yet.
        let mut source: Option<(skrifa::FontRef<'_>, Vec<NormalizedCoord>)> = None;
        let mut meshes: Vec<egui::Mesh> = Vec::new();
        for glyph in run.glyphs.iter() {
            let x = (origin.x + run.origin.x + glyph.x) * pixels_per_point;
            let y = ((origin.y + run.origin.y + glyph.y) * pixels_per_point).round();
            // Quarter-pixel bins as epaint's, so neighbours keep even spacing.
            let (whole, bin) = if options.subpixel_binning {
                let quarters = (x * 4.0).round();
                ((quarters / 4.0).floor(), (quarters.rem_euclid(4.0)) as u8)
            } else {
                (x.round(), 0)
            };
            let key = GlyphKey { glyph: glyph.id, bin, ..base };
            let placed = match atlas.glyphs.get(&key) {
                Some(placed) => *placed,
                None => {
                    if source.is_none() {
                        let Ok(font) = skrifa::FontRef::from_index(run.font.bytes(), run.font.index()) else {
                            return;
                        };
                        source = Some((font, run.coords.iter().map(|coord| NormalizedCoord::from_bits(*coord)).collect()));
                    }
                    let (font, coords) = source.as_ref().expect("just read");
                    atlas.place(key, font, coords, run, pixels, options.font_hinting)
                }
            };
            let Some(placed) = placed else {
                continue;
            };
            let texture = atlas.pages[placed.page].texture.id();
            let top_left = Pos2::new(whole + placed.offset.x, y + placed.offset.y) / pixels_per_point;
            let rect = egui::Rect::from_min_size(
                top_left,
                egui::vec2(placed.size[0] as f32, placed.size[1] as f32) / pixels_per_point,
            );
            let uv = egui::Rect::from_min_size(
                Pos2::new(placed.min[0] as f32, placed.min[1] as f32) / PAGE as f32,
                egui::vec2(placed.size[0] as f32, placed.size[1] as f32) / PAGE as f32,
            );
            let mesh = match meshes.iter().position(|mesh| mesh.texture_id == texture) {
                Some(index) => &mut meshes[index],
                None => {
                    let mut mesh = egui::Mesh::with_texture(texture);
                    mesh.reserve_vertices(run.glyphs.len() * 4);
                    mesh.reserve_triangles(run.glyphs.len() * 2);
                    meshes.push(mesh);
                    meshes.last_mut().expect("just pushed")
                }
            };
            mesh.add_rect_with_uv(rect, uv, ink);
        }
        let meshes: Vec<Arc<egui::Mesh>> = meshes.into_iter().map(Arc::new).collect();
        for mesh in &meshes {
            painter.add(egui::Shape::Mesh(mesh.clone()));
        }
        // A run whose glyphs started the pages again holds some of the old ones.
        if atlas.restarts == restarts {
            atlas.drawn.insert(place, DrawnRun { _glyphs: run.glyphs.clone(), meshes });
        }
    });
}

impl Atlas {
    fn attach(&mut self, ctx: &egui::Context, style: (FontColorTransferFunction, bool)) {
        if self.context.as_ref() != Some(ctx) || self.style != Some(style) {
            *self = Self { context: Some(ctx.clone()), style: Some(style), ..Self::default() };
        }
    }

    fn place(
        &mut self,
        key: GlyphKey,
        font: &skrifa::FontRef<'_>,
        coords: &[NormalizedCoord],
        run: &GlyphRun,
        pixels: f32,
        hinted: bool,
    ) -> Option<Placed> {
        let drawn = self.rasterize(key, font, coords, run, pixels, hinted);
        let placed = drawn.and_then(|(image, offset)| self.allocate(image, offset));
        self.glyphs.insert(key, placed);
        placed
    }

    /// The glyph's coverage, as the atlas keeps it, and where its top-left
    /// corner is from the pen; `None` when there is nothing to draw.
    fn rasterize(
        &mut self,
        key: GlyphKey,
        font: &skrifa::FontRef<'_>,
        coords: &[NormalizedCoord],
        run: &GlyphRun,
        pixels: f32,
        hinted: bool,
    ) -> Option<(ColorImage, egui::Vec2)> {
        let outlines = font.outline_glyphs();
        let outline = outlines.get(GlyphId::new(key.glyph))?;
        let location = LocationRef::new(coords);
        let mut path = BezPath::new();
        let mut pen = Pen { path: &mut path, x_offset: f64::from(key.bin) / 4.0 };
        let size = Size::new(pixels);
        if hinted {
            let instance = match self.hinting.entry((key.font, key.index, key.pixels, key.coords)) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => {
                    let target: skrifa::outline::Target = HintingTarget::default().into();
                    entry.insert(HintingInstance::new(&outlines, size, location, target).ok()?)
                }
            };
            outline.draw(DrawSettings::hinted(instance, false), &mut pen).ok()?;
        } else {
            outline.draw(DrawSettings::unhinted(size, location), &mut pen).ok()?;
        }
        if let Some(degrees) = run.skew {
            // y grows downward here, so a slant to the right shears by -y.
            path.apply_affine(Affine::new([1.0, 0.0, -f64::from(degrees).to_radians().tan(), 1.0, 0.0, 0.0]));
        }
        // A bold the face lacks: the outline stroked as well as filled.
        let stroke = run.embolden.then(|| f64::from(pixels) / 24.0);
        let bounds = path.bounding_box().inflate(stroke.unwrap_or(0.0), stroke.unwrap_or(0.0)).expand();
        let (width, height) = (bounds.width() as u16, bounds.height() as u16);
        if width == 0 || height == 0 {
            return None;
        }
        let mut ctx = vello_cpu::RenderContext::new(width, height);
        ctx.set_transform(Affine::translate((-bounds.x0, -bounds.y0)));
        ctx.set_paint(vello_cpu::color::OpaqueColor::<vello_cpu::color::Srgb>::WHITE);
        ctx.fill_path(&path);
        if let Some(width) = stroke {
            ctx.set_stroke(kurbo::Stroke::new(width));
            ctx.stroke_path(&path);
        }
        let mut pixmap = vello_cpu::Pixmap::new(width, height);
        ctx.render(&mut pixmap, &mut vello_cpu::Resources::new());
        let transfer = self.style.map_or(FontColorTransferFunction::default(), |(transfer, _)| transfer);
        let pixels = pixmap
            .data_as_u8_slice()
            .chunks_exact(4)
            .map(|pixel| transfer.to_atlas_color(Color32::from_rgba_premultiplied(pixel[0], pixel[1], pixel[2], pixel[3])))
            .collect();
        let image = ColorImage { size: [width as usize, height as usize], pixels, ..ColorImage::default() };
        Some((image, egui::vec2(bounds.x0 as f32, bounds.y0 as f32)))
    }

    fn allocate(&mut self, image: ColorImage, offset: egui::Vec2) -> Option<Placed> {
        let [width, height] = image.size;
        if width + GUTTER > PAGE || height + GUTTER > PAGE {
            return None;
        }
        let fits = |page: &Page| {
            let [top, tall, along] = page.shelf;
            (along + width + GUTTER <= PAGE && top + height.max(tall) + GUTTER <= PAGE)
                || top + tall + GUTTER + height + GUTTER <= PAGE
        };
        if !self.pages.last().is_some_and(fits) {
            if self.pages.len() == MAX_PAGES {
                // Start again; what is drawn this frame keeps its pages until egui frees them.
                self.pages.clear();
                self.glyphs.clear();
                self.hinting.clear();
                self.drawn.clear();
                self.drawn_before.clear();
                self.restarts += 1;
            }
            let ctx = self.context.clone()?;
            let blank = ColorImage::filled([PAGE, PAGE], Color32::TRANSPARENT);
            let texture = ctx.load_texture(format!("limn-glyphs-{}", self.pages.len()), blank, TextureOptions::LINEAR);
            self.pages.push(Page { texture, shelf: [0, 0, 0] });
        }
        let index = self.pages.len() - 1;
        let page = &mut self.pages[index];
        let [top, tall, along] = page.shelf;
        let (x, y) = if along + width + GUTTER <= PAGE && top + height.max(tall) + GUTTER <= PAGE {
            page.shelf = [top, tall.max(height), along + width + GUTTER];
            (along, top)
        } else {
            let next = top + tall + GUTTER;
            page.shelf = [next, height, width + GUTTER];
            (0, next)
        };
        page.texture.set_partial([x, y], image, TextureOptions::LINEAR);
        Some(Placed { page: index, min: [x, y], size: [width, height], offset })
    }
}

/// Turns an outline into a path in pixels, y downward, moved right by a
/// fraction of a pixel.
struct Pen<'a> {
    path: &'a mut BezPath,
    x_offset: f64,
}

impl OutlinePen for Pen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to((f64::from(x) + self.x_offset, -f64::from(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((f64::from(x) + self.x_offset, -f64::from(y)));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.path.quad_to(
            (f64::from(cx0) + self.x_offset, -f64::from(cy0)),
            (f64::from(x) + self.x_offset, -f64::from(y)),
        );
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(
            (f64::from(cx0) + self.x_offset, -f64::from(cy0)),
            (f64::from(cx1) + self.x_offset, -f64::from(cy1)),
            (f64::from(x) + self.x_offset, -f64::from(y)),
        );
    }

    fn close(&mut self) {
        self.path.close_path();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atlas() -> Atlas {
        let mut atlas = Atlas::default();
        atlas.attach(&egui::Context::default(), (FontColorTransferFunction::default(), true));
        atlas
    }

    fn glyph(width: usize, height: usize) -> ColorImage {
        ColorImage::filled([width, height], Color32::WHITE)
    }

    #[test]
    fn a_run_is_painted_as_one_mesh_from_the_atlas_in_its_ink() {
        let ctx = egui::Context::default();
        let layout = limn::TextLayout::new("limn", limn::TextStyle::new(14.0, limn::FontFamily::Sans));
        let commands = layout.draw(limn::Point::new(0.0, 0.0), limn::Color::rgb(200, 100, 50));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for command in &commands {
                if let limn::DrawCommand::Glyphs(run) = command {
                    paint(ui.painter(), run, Pos2::new(10.0, 20.0), Display::of(ui.ctx()));
                }
            }
        });
        let meshes: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(meshes.len(), 1);
        let mesh = &meshes[0];
        assert_eq!(mesh.vertices.len(), 4 * 4, "four glyphs of four corners");
        assert_ne!(mesh.texture_id, egui::TextureId::default(), "limn's atlas, not egui's font texture");
        assert!(mesh.vertices.iter().all(|vertex| vertex.color == Color32::from_rgb(200, 100, 50)));
        let left = mesh.vertices.iter().map(|vertex| vertex.pos.x).fold(f32::MAX, f32::min);
        let top = mesh.vertices.iter().map(|vertex| vertex.pos.y).fold(f32::MAX, f32::min);
        assert!((10.0..12.0).contains(&left), "{left}");
        assert!((20.0..24.0).contains(&top), "{top}");
        assert!(!output.textures_delta.set.is_empty(), "the glyphs went up to the texture");
        // This probe has no renderer to take the texture updates.
        output.textures_delta.clear();
    }

    #[test]
    fn glyphs_never_overlap_and_a_full_page_is_followed_by_another() {
        let mut atlas = atlas();
        let placed: Vec<Placed> = (0..4000)
            .map(|index| atlas.allocate(glyph(10 + index % 23, 12 + index % 17), egui::Vec2::ZERO).unwrap())
            .collect();
        assert!(atlas.pages.len() > 1, "4000 glyphs fill more than one page");
        for (index, a) in placed.iter().enumerate() {
            assert!(a.min[0] + a.size[0] <= PAGE && a.min[1] + a.size[1] <= PAGE);
            for b in &placed[index + 1..] {
                let apart = a.page != b.page
                    || a.min[0] + a.size[0] + GUTTER <= b.min[0]
                    || b.min[0] + b.size[0] + GUTTER <= a.min[0]
                    || a.min[1] + a.size[1] + GUTTER <= b.min[1]
                    || b.min[1] + b.size[1] + GUTTER <= a.min[1];
                assert!(apart, "{:?} and {:?}", (a.page, a.min, a.size), (b.page, b.min, b.size));
            }
        }
    }

    #[test]
    fn past_the_last_page_the_atlas_starts_again_and_a_glyph_too_large_is_not_kept() {
        let mut atlas = atlas();
        let big = || glyph(PAGE / 2, PAGE / 2);
        for _ in 0..MAX_PAGES * 4 {
            atlas.allocate(big(), egui::Vec2::ZERO).unwrap();
        }
        assert_eq!(atlas.pages.len(), MAX_PAGES);
        atlas.glyphs.insert(
            GlyphKey { font: 1, index: 0, glyph: 1, pixels: 0, bin: 0, coords: 0, embolden: false, skew: 0 },
            None,
        );
        atlas.allocate(big(), egui::Vec2::ZERO).unwrap();
        assert_eq!(atlas.pages.len(), 1);
        assert!(atlas.glyphs.is_empty(), "every glyph is drawn again into the new pages");
        assert!(atlas.allocate(glyph(PAGE, 8), egui::Vec2::ZERO).is_none());
    }
}

#[cfg(test)]
mod reuse_tests {
    use super::*;

    fn shapes(ctx: &egui::Context, commands: &[limn::DrawCommand], at: Pos2) -> Vec<Arc<egui::Mesh>> {
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            for command in commands {
                if let limn::DrawCommand::Glyphs(run) = command {
                    paint(ui.painter(), run, at, Display::of(ui.ctx()));
                }
            }
        });
        output.textures_delta.clear();
        output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_run_that_stays_put_reuses_its_mesh_and_one_that_moves_is_drawn_again() {
        let ctx = egui::Context::default();
        let layout = limn::TextLayout::new("limn", limn::TextStyle::new(14.0, limn::FontFamily::Sans));
        let commands = layout.draw(limn::Point::new(0.0, 0.0), limn::Color::WHITE);
        let first = shapes(&ctx, &commands, Pos2::new(10.0, 20.0));
        let again = shapes(&ctx, &layout.draw(limn::Point::new(0.0, 0.0), limn::Color::WHITE), Pos2::new(10.0, 20.0));
        assert!(Arc::ptr_eq(&first[0], &again[0]), "the same glyphs in the same place");
        let moved = shapes(&ctx, &commands, Pos2::new(11.0, 20.0));
        assert!(!Arc::ptr_eq(&first[0], &moved[0]));
        assert_eq!(moved[0].vertices[0].pos.x, first[0].vertices[0].pos.x + 1.0);
    }
}
