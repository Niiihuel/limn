//! `set_fonts` changes the faces for the whole process, so this file holds
//! one test: nothing else in its process lays text out meanwhile.
// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use limn::{fonts, set_fonts, DrawCommand, Font, FontFamily, Fonts, Point, TextLayout, TextStyle};

fn width(text: &str, family: FontFamily) -> f32 {
    TextLayout::new(text, TextStyle::new(14.0, family)).size().width
}

#[test]
fn the_faces_given_are_the_ones_text_is_laid_out_in_from_then_on() {
    let bundled = fonts();
    let sans = width("iiii", FontFamily::Sans);
    assert!(sans < width("iiii", FontFamily::Monospace));

    // With no faces at all, text takes no room and draws nothing.
    set_fonts(Fonts::new());
    let none = TextLayout::new("limn", TextStyle::new(14.0, FontFamily::Sans));
    assert_eq!(none.size().width, 0.0);
    assert!(none.draw(Point::new(0.0, 0.0), limn::Color::WHITE).iter().all(|command| match command {
        DrawCommand::Glyphs(run) => run.glyphs.iter().all(|glyph| glyph.id == 0),
        _ => false,
    }));

    // A face of the host's own: here Hack for sans, so it is as wide as code.
    let hack = bundled.face_of(FontFamily::Monospace).unwrap().clone();
    let bytes: Vec<u8> = hack.bytes().to_vec();
    set_fonts(Fonts::new().face(FontFamily::Sans, Font::new(bytes).unwrap()));
    assert!((width("iiii", FontFamily::Sans) - width("WWWW", FontFamily::Sans)).abs() < 0.01);

    set_fonts(bundled);
    assert_eq!(width("iiii", FontFamily::Sans), sans);
}
