use super::*;
use std::cell::Cell;

thread_local! { static CALLS: Cell<usize> = const { Cell::new(0) }; }

struct Restore(Option<IconRasterizer>);

impl Restore {
    fn install(rasterizer: Option<IconRasterizer>) -> Self {
        let old = ICON_RASTERIZER.with(Cell::get);
        set_icon_rasterizer(rasterizer);
        CALLS.with(|calls| calls.set(0));
        Self(old)
    }
}

impl Drop for Restore {
    fn drop(&mut self) { set_icon_rasterizer(self.0); }
}

fn native_search(icon: Icon, pixels: u16) -> Option<egui::ColorImage> {
    (icon == Icon::Search).then(|| {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        egui::ColorImage::filled([usize::from(pixels); 2], Color32::WHITE)
    })
}

#[test]
fn provider_receives_physical_size_and_unmapped_icons_keep_svg_fallback() {
    let _restore = Restore::install(None);
    let search = IconKey { icon: Icon::Search, pixels: 48 };
    let trash = IconKey { icon: Icon::Trash, pixels: 48 };
    let fallback_search = raster_icon(search).unwrap();
    let fallback_trash = raster_icon(trash).unwrap();
    set_icon_rasterizer(Some(native_search));
    let native = raster_icon(search).unwrap();
    assert!(native == egui::ColorImage::filled([48, 48], Color32::WHITE), "the provider's native mask is used at the requested physical size");
    assert!(native != fallback_search);
    assert!(raster_icon(trash).unwrap() == fallback_trash);
    assert_eq!(CALLS.with(Cell::get), 1);
}

#[test]
fn malformed_provider_images_fall_back_before_texture_upload() {
    let _restore = Restore::install(None);
    let key = IconKey { icon: Icon::Search, pixels: 32 };
    let fallback = raster_icon(key).unwrap();
    set_icon_rasterizer(Some(|_, pixels| Some(egui::ColorImage::filled([usize::from(pixels), 1], Color32::WHITE))));
    assert!(raster_icon(key).unwrap() == fallback, "wrong dimensions cannot enter the texture cache");
    set_icon_rasterizer(Some(|_, pixels| {
        let mut image = egui::ColorImage::filled([usize::from(pixels); 2], Color32::WHITE);
        image.pixels.pop();
        Some(image)
    }));
    assert!(raster_icon(key).unwrap() == fallback, "truncated pixel data cannot enter the texture cache");
}

#[test]
fn custom_artwork_can_opt_in_but_file_types_keep_their_native_palette() {
    const OWN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect width="20" height="10" fill="currentColor"/></svg>"#;
    let _restore = Restore::install(None);
    let key = IconKey { icon: limn::file_icon("example.rs"), pixels: 32 };
    let file = raster_icon(key).unwrap();
    set_icon_rasterizer(Some(|_, pixels| {
        CALLS.with(|calls| calls.set(calls.get() + 1));
        Some(egui::ColorImage::filled([usize::from(pixels); 2], Color32::WHITE))
    }));
    assert!(raster_icon(IconKey { icon: Icon::Custom(OWN), pixels: 32 }).unwrap() == egui::ColorImage::filled([32, 32], Color32::WHITE), "custom artwork can explicitly choose the native provider");
    assert!(raster_icon(key).unwrap() == file);
    assert_eq!(CALLS.with(Cell::get), 1, "file artwork does not call the monochrome provider");
}

struct Upload {
    id: egui::TextureId,
    image: Option<egui::ColorImage>,
    freed: Vec<egui::TextureId>,
}

fn texture(ctx: &egui::Context, key: IconKey) -> Upload {
    let mut id = None;
    let mut output = ctx.run_ui(Default::default(), |ui| id = icon_texture(ui, key));
    let id = id.unwrap();
    let image = output.textures_delta.set.get(&id).and_then(|deltas| deltas.last()).map(|delta| match &delta.image {
        egui::ImageData::Color(image) => image.as_ref().clone(),
    });
    let freed = output.textures_delta.free.iter().copied().collect();
    output.textures_delta.clear();
    Upload { id, image, freed }
}

#[test]
fn cached_native_icons_render_once_and_provider_changes_replace_the_painted_texture() {
    let _restore = Restore::install(Some(native_search));
    let ctx = egui::Context::default();
    let key = IconKey { icon: Icon::Search, pixels: 32 };
    let first = texture(&ctx, key);
    assert!(first.image.is_some());
    let second = texture(&ctx, key);
    assert_eq!(first.id, second.id);
    assert!(second.image.is_none());
    assert_eq!(CALLS.with(Cell::get), 1, "repaints reuse the rasterized mask");

    set_icon_rasterizer(None);
    let fallback = texture(&ctx, key);
    assert_ne!(fallback.id, first.id, "removing the provider must not keep its old texture");
    assert!(fallback.image.unwrap() == raster_icon(key).unwrap());
    assert!(fallback.freed.contains(&first.id));
}
