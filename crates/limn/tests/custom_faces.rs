//! A face a host gives for `FontFamily::Custom`. `set_fonts` changes the
//! faces for the whole process, so this file holds one test.
// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use limn::{
    fonts, set_fonts, DrawCommand, Element, Font, FontFamily, Frame, Point, Size, TextLayout,
    TextStyle, UiTree,
};

const NAME: FontFamily = FontFamily::Custom(7);

fn layout(text: &str, family: FontFamily) -> TextLayout {
    TextLayout::new(text, TextStyle::new(14.0, family))
}

fn width(text: &str, family: FontFamily) -> f32 {
    layout(text, family).size().width
}

/// The faces each run of `text` in `family` is drawn from, by id.
fn faces(text: &str, family: FontFamily) -> Vec<u64> {
    layout(text, family)
        .draw(Point::new(0.0, 0.0), limn::Color::WHITE)
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run.font.id()),
            _ => None,
        })
        .collect()
}

/// Where the text after a truncated name starts in a row: the name's width.
fn after_name(frame: &Frame) -> f32 {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { text, position, .. } if text == "|" => Some(position.x),
            _ => None,
        })
        .unwrap()
}

#[test]
fn a_custom_face_is_laid_out_once_given_and_is_sans_until_then() {
    let bundled = fonts();
    let sans = bundled.face_of(FontFamily::Sans).unwrap().id();
    // Not given: Sans, measured and drawn alike.
    assert_eq!(
        layout("Ada iiii", NAME).size(),
        layout("Ada iiii", FontFamily::Sans).size()
    );
    assert_eq!(faces("Ada", NAME), vec![sans]);

    // A tree measuring through the host, as limn-egui's does, measures again once the faces change.
    let mut tree = UiTree::new(Element::row(vec![
        Element::truncated_text("iiii").font_family(NAME),
        Element::text("|").font_family(FontFamily::Sans),
    ]));
    tree.set_text_measurer(|text, size, family| {
        TextLayout::new(text, TextStyle::new(size, family))
            .size()
            .width
    });
    let before = after_name(&tree.layout(Size::new(300.0, 40.0)));
    assert!((before - width("iiii", FontFamily::Sans)).abs() < 0.01);

    // Hack, a monospace face, given as the custom one.
    let hack = bundled.face_of(FontFamily::Monospace).unwrap();
    let face = Font::new(hack.bytes().to_vec()).unwrap();
    set_fonts(bundled.clone().face(NAME, face.clone()));
    assert!(
        (width("iiii", NAME) - width("WWWW", NAME)).abs() < 0.01,
        "every letter as wide"
    );
    assert_eq!(
        width("iiii", FontFamily::Sans),
        width("iiii", FontFamily::Sans)
    );
    assert!(width("iiii", NAME) > width("iiii", FontFamily::Sans));
    assert_eq!(faces("Ada", NAME), vec![face.id()]);
    // What the face lacks is drawn as Sans text draws it: a mathematical
    // letter from the math fallback, emoji from the emoji face.
    let math = bundled.fallbacks()[0].id();
    let emoji = bundled.emoji_face().unwrap().id();
    assert_eq!(faces("Ada 𝓡", NAME), vec![face.id(), math]);
    assert_eq!(faces("Ada 🦀", NAME), vec![face.id(), emoji]);
    // Another number still has no face.
    assert_eq!(faces("Ada", FontFamily::Custom(8)), vec![sans]);
    assert!((after_name(&tree.layout(Size::new(300.0, 40.0))) - width("iiii", NAME)).abs() < 0.01);

    // Laid out by the tree itself, the text takes the face too.
    let mut laid_out = UiTree::new(Element::column(vec![
        Element::text("iiii").font_family(NAME)
    ]));
    laid_out.set_text_layout(true);
    let frame = laid_out.layout(Size::new(300.0, 100.0));
    assert!((frame.text_layouts()[0].runs[0].rect.width - width("iiii", NAME)).abs() < 0.01);

    set_fonts(bundled);
    assert_eq!(
        layout("Ada iiii", NAME).size(),
        layout("Ada iiii", FontFamily::Sans).size()
    );
}
