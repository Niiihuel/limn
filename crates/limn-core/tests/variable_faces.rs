//! A variable face at the instance its `Font` names. `set_fonts` changes the
//! faces for the whole process, so this file holds one test.
// Noto Emoji is the variable face limn bundles: a weight axis from 300 to
// 700, 400 by default, and no other.
#![cfg(feature = "bundled-fonts")]

use limn_core::{fonts, set_fonts, Color, DrawCommand, FontFamily, GlyphRun, Point, TextLayout, TextStyle};

fn runs(text: &str, family: FontFamily) -> Vec<GlyphRun> {
    TextLayout::new(text, TextStyle::new(14.0, family))
        .draw(Point::new(0.0, 0.0), Color::WHITE)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run),
            _ => None,
        })
        .collect()
}

/// The weight axis of the one run of `text`, as 2.14 fixed point.
fn weight(text: &str, family: FontFamily) -> Vec<i16> {
    let runs = runs(text, family);
    assert_eq!(runs.len(), 1, "{text}");
    runs[0].coords.to_vec()
}

#[test]
fn a_variable_face_is_laid_out_and_drawn_at_the_instance_it_names() {
    let bundled = fonts();
    let emoji = bundled.emoji_face().unwrap().clone();
    let at = |weight: f32| emoji.clone().with_variations([(*b"wght", weight)]);
    const LIGHTEST: i16 = -16384;
    const HEAVIEST: i16 = 16384;

    // As before, an instance named by no face: emoji at their default in
    // regular text and at 700 in bold, as CSS sets a variable face.
    assert_eq!(weight("🦀", FontFamily::Sans), Vec::<i16>::new());
    assert_eq!(weight("🦀", FontFamily::SansBold), [HEAVIEST]);

    set_fonts(
        bundled
            .clone()
            .face(FontFamily::Sans, at(300.0))
            .face(FontFamily::SansMedium, at(560.0))
            .face(FontFamily::SansBold, at(300.0))
            .face(FontFamily::Custom(5), at(900.0))
            .emoji(at(300.0)),
    );
    assert_eq!(weight("🦀", FontFamily::Sans), [LIGHTEST]);
    let medium = weight("🦀", FontFamily::SansMedium)[0];
    assert!(0 < medium && medium < HEAVIEST, "560 is between the default and the heaviest: {medium}");
    assert_eq!(weight("🦀", FontFamily::SansBold), [LIGHTEST], "the instance named, not the family's 700");
    assert_eq!(weight("🦀", FontFamily::SansItalic), [LIGHTEST], "a family with no face of its own takes the one it falls to");
    assert_eq!(weight("🦀", FontFamily::Custom(5)), [HEAVIEST], "past the axis, its end");
    // Every run of a line is set at the instance of its own family.
    let spans = [
        limn_core::TextSpan::new("🦀", Color::WHITE, 14.0, FontFamily::SansMedium),
        limn_core::TextSpan::new("🦀", Color::WHITE, 14.0, FontFamily::Sans),
    ];
    let formatted: Vec<Vec<i16>> = TextLayout::formatted(&spans, TextStyle::new(14.0, FontFamily::SansMedium))
        .draw(Point::new(0.0, 0.0), Color::WHITE)
        .into_iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run.coords.to_vec()),
            _ => None,
        })
        .collect();
    assert_eq!(formatted, [vec![medium], vec![LIGHTEST]]);

    // A static face has no axes to set.
    let ubuntu = bundled.face_of(FontFamily::Sans).unwrap().clone();
    set_fonts(bundled.clone().face(FontFamily::Sans, ubuntu.with_variations([(*b"wght", 300.0)])));
    assert_eq!(weight("limn", FontFamily::Sans), Vec::<i16>::new());

    set_fonts(bundled);
    assert_eq!(weight("🦀", FontFamily::SansBold), [HEAVIEST]);
}
