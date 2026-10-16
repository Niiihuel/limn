use super::*;
use crate::color_emoji::*;
use std::sync::OnceLock;

/// A square PNG of one color, `side` pixels a side.
fn png(side: u32, rgba: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(side, side, image::Rgba(rgba));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

fn smile() -> &'static [u8] {
    static PNG: OnceLock<Vec<u8>> = OnceLock::new();
    PNG.get_or_init(|| png(72, [255, 200, 0, 255]))
}

/// Draws the smile, a family, a flag, a keycap, a thumb with a skin tone and
/// a heart only when it asks for emoji presentation; anything else is
/// garbage bytes, which must keep the glyph.
fn source(cluster: &str) -> Option<&'static [u8]> {
    match cluster {
        "🙂" | "👨\u{200d}👩\u{200d}👧\u{200d}👦" | "🇦🇷" | "1\u{fe0f}\u{20e3}" | "👍🏽"
        | "❤\u{fe0f}" => Some(smile()),
        "🦀" => Some(b"not a png"),
        _ => None,
    }
}

#[test]
fn runs_are_whole_clusters_the_source_draws() {
    let text = "a🙂 👨\u{200d}👩\u{200d}👧\u{200d}👦🇦🇷 1\u{fe0f}\u{20e3} 👍🏽 ❤\u{fe0f} ❤ é 1";
    let found: Vec<&str> = runs(text, source)
        .into_iter()
        .map(|(range, _)| &text[range])
        .collect();
    assert_eq!(
        found,
        [
            "🙂",
            "👨\u{200d}👩\u{200d}👧\u{200d}👦",
            "🇦🇷",
            "1\u{fe0f}\u{20e3}",
            "👍🏽",
            "❤\u{fe0f}"
        ],
        "a text-presentation heart, a letter and a digit stay text"
    );
    assert!(runs("plain ascii :) 1 #", source).is_empty());
}

#[test]
fn a_picture_fills_the_room_of_its_glyphs_between_one_and_a_fifth_more_ems() {
    let at = |x: f32| EguiRect::from_min_size(Pos2::new(x, 10.0), egui::vec2(0.0, 20.0));
    let fitted = picture_rect(at(5.0), at(23.0), 16.0);
    assert_eq!(fitted.width(), 18.0);
    assert_eq!(fitted.center(), Pos2::new(14.0, 20.0));
    // A narrow replacement glyph still gets an em; an unjoined sequence of
    // four glyphs, no more than 1.2.
    assert_eq!(picture_rect(at(5.0), at(10.0), 16.0).width(), 16.0);
    assert!((picture_rect(at(0.0), at(80.0), 16.0).width() - 19.2).abs() < 1e-4);
}

fn painted(text: &str) -> Vec<egui::epaint::ClippedShape> {
    let ctx = egui::Context::default();
    crate::install_fonts(&ctx);
    let mut frame = Frame::default();
    frame.commands = vec![DrawCommand::Text {
        position: Point::new(10.0, 20.0),
        text: text.into(),
        color: Color::WHITE,
        font_size: 16.0,
        font_family: FontFamily::Sans,
    }];
    let mut output = ctx.run_ui(Default::default(), |ui| crate::paint_at(ui, &frame, Pos2::ZERO));
    output.textures_delta.clear();
    output.shapes
}

fn pictures(shapes: &[egui::epaint::ClippedShape]) -> Vec<EguiRect> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) if mesh.texture_id != egui::TextureId::default() => {
                Some(mesh.calc_bounds())
            }
            _ => None,
        })
        .collect()
}

fn galley_width(shapes: &[egui::epaint::ClippedShape]) -> f32 {
    shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some(text.galley.rect.width()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn installed_artwork_covers_emoji_without_moving_the_text() {
    set_color_emoji(None);
    let text = "a🙂b 🦀 ❤";
    let before = painted(text);
    assert!(pictures(&before).is_empty(), "nothing installed: glyphs only");
    set_color_emoji(Some(source));
    let after = painted(text);
    let drawn = pictures(&after);
    assert_eq!(drawn.len(), 1, "the crab's bytes are not a PNG: its glyph stays");
    assert!((galley_width(&before) - galley_width(&after)).abs() < 0.01);
    let picture = drawn[0];
    assert!((16.0..=19.2).contains(&picture.width()), "{picture:?}");
    assert!(picture.min.x > 10.0 && picture.min.y > 15.0 && picture.max.y < 45.0, "{picture:?}");
    set_color_emoji(None);
}

#[test]
fn a_pass_decodes_a_bounded_number_of_pictures() {
    fn many(cluster: &str) -> Option<&'static [u8]> {
        static PNGS: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
        let pngs = PNGS.get_or_init(|| (0..40).map(|index| png(8, [index, 0, 0, 255])).collect());
        let index = cluster.chars().next()? as usize;
        (0x1F600..0x1F600 + 40)
            .contains(&index)
            .then(|| pngs[index - 0x1F600].as_slice())
    }
    set_color_emoji(Some(many));
    let text: String = (0x1F600..0x1F600 + 40).filter_map(char::from_u32).collect();
    assert_eq!(pictures(&painted(&text)).len(), DECODES_PER_PASS);
    set_color_emoji(None);
}
