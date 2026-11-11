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
    painted_commands(vec![measured(text, Color::WHITE)])
}

/// `text` at 16 px from (10, 20), as measured text.
fn measured(text: &str, ink: Color) -> DrawCommand {
    DrawCommand::Text { position: Point::new(10.0, 20.0), text: text.into(), color: ink, font_size: 16.0, font_family: FontFamily::Sans }
}

/// `text` at 16 px from (10, 20), as text limn lays out.
fn laid_out(text: &str, ink: Color) -> Vec<DrawCommand> {
    limn::TextLayout::new(text, limn::TextStyle::new(16.0, FontFamily::Sans)).draw(Point::new(10.0, 20.0), ink)
}

fn painted_commands(commands: Vec<DrawCommand>) -> Vec<egui::epaint::ClippedShape> {
    let ctx = egui::Context::default();
    crate::install_fonts(&ctx);
    let mut frame = Frame::default();
    frame.commands = commands;
    let mut output = ctx.run_ui(Default::default(), |ui| crate::paint_at(ui, &frame, Pos2::ZERO));
    output.textures_delta.clear();
    output.shapes
}

/// A picture is drawn whole from its texture; a glyph from the atlas is a
/// small part of a page.
fn is_picture(clipped: &egui::epaint::ClippedShape) -> bool {
    matches!(&clipped.shape, egui::Shape::Mesh(mesh) if mesh.vertices.iter().any(|vertex| vertex.uv == Pos2::new(1.0, 1.0)))
}

/// The pictures among `shapes`, and the glyphs drawn from limn's atlas,
/// each as the rectangle it covers.
fn pictures_and_glyphs(shapes: &[egui::epaint::ClippedShape]) -> (Vec<EguiRect>, Vec<EguiRect>) {
    let (mut pictures, mut glyphs) = (Vec::new(), Vec::new());
    for clipped in shapes {
        if let egui::Shape::Mesh(mesh) = &clipped.shape {
            if is_picture(clipped) {
                pictures.push(mesh.calc_bounds());
            } else {
                glyphs.extend(mesh.vertices.chunks(4).map(|quad| EguiRect::from_two_pos(quad[0].pos, quad[3].pos)));
            }
        }
    }
    (pictures, glyphs)
}

#[test]
fn an_emoji_in_text_limn_lays_out_is_a_picture_where_its_glyph_was_and_the_text_around_it_stays() {
    set_color_emoji(None);
    let commands = laid_out("a🙂b", Color::WHITE);
    let (none, before) = pictures_and_glyphs(&painted_commands(commands.clone()));
    assert!(none.is_empty(), "nothing installed: glyphs only");
    assert_eq!(before.len(), 3, "a, the smile's monochrome glyph and b");
    set_color_emoji(Some(source));
    let (drawn, after) = pictures_and_glyphs(&painted_commands(commands.clone()));
    assert_eq!(drawn.len(), 1);
    assert_eq!(after, [before[0], before[2]], "a and b as they were, and no glyph under the picture");
    let (picture, glyph) = (drawn[0], before[1]);
    assert!((16.0..=19.2 + 1e-4).contains(&picture.width()) && (picture.width() - picture.height()).abs() < 1e-4, "{picture:?}");
    assert!(picture.contains(glyph.center()) && (picture.center() - glyph.center()).length() < 0.5, "{picture:?} over the glyph's {glyph:?}");
    // Sized as measured text sizes it: by the room the same glyph takes in its line.
    let (measured, _) = pictures_and_glyphs(&painted_commands(vec![measured("a🙂b", Color::WHITE)]));
    assert!((measured[0].width() - picture.width()).abs() < 0.01, "{measured:?} and {picture:?}");
    // A heart with VS16 is drawn from Noto Sans Symbols 2, whose box reaches far below its ink,
    // and VS16 is an empty glyph of no advance: the picture stands at the height of the heart's
    // ink, across the room the heart takes, short of the y.
    let drawn = pictures_and_glyphs(&painted_commands(laid_out("x❤\u{fe0f}y", Color::WHITE))).0;
    set_color_emoji(None);
    let glyphs = pictures_and_glyphs(&painted_commands(laid_out("x❤\u{fe0f}y", Color::WHITE))).1;
    let (heart, y) = (glyphs[1], glyphs[2]);
    assert!(drawn[0].contains_rect(heart) && (drawn[0].center().y - heart.center().y).abs() < 0.75, "{drawn:?} over the glyph's {heart:?}");
    assert!(drawn[0].max.x <= y.min.x + 0.5, "{drawn:?} reaches into the y's {y:?}");
    assert_eq!(pictures_and_glyphs(&painted_commands(commands)), (Vec::new(), before), "removed, the glyph is back");
}

#[test]
fn a_joined_sequence_a_flag_and_a_keycap_laid_out_by_limn_are_one_picture_each() {
    let text = "👨\u{200d}👩\u{200d}👧\u{200d}👦 🇦🇷 1\u{fe0f}\u{20e3} 🦀";
    set_color_emoji(None);
    let (_, before) = pictures_and_glyphs(&painted_commands(laid_out(text, Color::WHITE)));
    set_color_emoji(Some(source));
    let (drawn, after) = pictures_and_glyphs(&painted_commands(laid_out(text, Color::WHITE)));
    assert_eq!(drawn.len(), 3, "{drawn:?}");
    assert!(drawn.windows(2).all(|pair| pair[0].max.x <= pair[1].min.x), "in order, apart: {drawn:?}");
    assert!(drawn.iter().all(|picture| (16.0..=19.2 + 1e-4).contains(&picture.width())), "{drawn:?}");
    assert_eq!(after, [*before.last().unwrap()], "only the crab, whose bytes are not a PNG, keeps its glyph");
    set_color_emoji(None);
}

#[test]
fn a_picture_in_text_limn_lays_out_takes_the_ink_s_alpha_and_the_clip_as_measured_text_does() {
    set_color_emoji(Some(source));
    let faded = Color::rgba(255, 255, 255, 128);
    let clipped = |text: Vec<DrawCommand>| {
        let mut commands = vec![DrawCommand::PushClip(limn::Rect::new(0.0, 0.0, 30.0, 100.0))];
        commands.extend(text);
        commands.push(DrawCommand::PopClip);
        let shapes = painted_commands(commands);
        let picture = shapes.into_iter().find(is_picture).expect("a picture");
        let egui::Shape::Mesh(mesh) = &picture.shape else { unreachable!() };
        (picture.clip_rect, mesh.vertices.iter().map(|vertex| vertex.color).collect::<Vec<_>>())
    };
    let (clip, tints) = clipped(laid_out("🙂", faded));
    assert_eq!(clip, EguiRect::from_min_size(Pos2::ZERO, egui::vec2(30.0, 100.0)));
    assert!(tints.iter().all(|tint| *tint == Color32::from_white_alpha(128)), "{tints:?}");
    assert_eq!((clip, tints), clipped(vec![measured("🙂", faded)]));
    set_color_emoji(None);
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
