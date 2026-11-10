//! Faces tried after every other, emoji's included. `set_fonts` changes the
//! faces for the whole process, so this file holds one test.
#![cfg(feature = "bundled-fonts")]

use limn_core::{fonts, set_fonts, Color, DrawCommand, FontFamily, Fonts, Point, TextLayout, TextStyle};

/// The face each run of `text` in `family` is drawn from, by id, and whether
/// every glyph of it is a real one rather than the missing-glyph mark.
fn faces(text: &str, family: FontFamily) -> Vec<(u64, bool)> {
    TextLayout::new(text, TextStyle::new(14.0, family))
        .draw(Point::new(0.0, 0.0), Color::WHITE)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some((run.font.id(), run.glyphs.iter().all(|glyph| glyph.id != 0))),
            _ => None,
        })
        .collect()
}

#[test]
fn a_last_resort_draws_only_what_no_other_face_has() {
    let bundled = fonts();
    let symbols = bundled.fallbacks()[1].clone();
    let emoji = bundled.emoji_face().unwrap().clone();
    let hack = bundled.face_of(FontFamily::Monospace).unwrap().clone();
    // Symbols for text, which has no letters, and Noto Emoji, which has `©` as Hack does.
    let given = Fonts::new().face(FontFamily::Sans, symbols.clone()).emoji(emoji.clone());
    set_fonts(given.clone());
    assert_eq!(faces("a", FontFamily::Sans), [(symbols.id(), false)], "no face has it: the missing-glyph mark");

    set_fonts(given.last_resort(hack.clone()));
    assert_eq!(fonts().last_resorts(), std::slice::from_ref(&hack));
    assert_eq!(faces("a", FontFamily::Sans), [(hack.id(), true)], "drawn by the last resort");
    assert_eq!(faces("a", FontFamily::Monospace), [(hack.id(), true)], "behind every family");
    assert_eq!(faces("©", FontFamily::Sans), [(emoji.id(), true)], "what the emoji face has stays its");
    assert_eq!(faces("✦", FontFamily::Sans), [(symbols.id(), true)]);

    set_fonts(bundled);
}
