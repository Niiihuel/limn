//! `TextLayout`: text shaped, broken into lines and placed by limn, in the
//! bundled faces.
// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use limn::{fonts, Color, DrawCommand, FontFamily, GlyphRun, Point, TextLayout, TextStyle};

const INK: Color = Color::rgb(10, 20, 30);
const PROSE: &str = "Text laid out by limn itself: shaped, broken into lines and placed.";

fn sans(text: &str) -> TextLayout {
    TextLayout::new(text, TextStyle::new(14.0, FontFamily::Sans))
}

fn runs(layout: &TextLayout) -> Vec<GlyphRun> {
    layout
        .draw(Point::new(0.0, 0.0), INK)
        .into_iter()
        .map(|command| match command {
            DrawCommand::Glyphs(run) => run,
            other => panic!("a text layout draws glyphs, not {other:?}"),
        })
        .collect()
}

fn boundaries(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.char_indices().map(|(index, _)| index).chain([text.len()])
}

#[test]
fn text_is_as_wide_as_its_glyphs_and_empty_text_is_one_line_tall() {
    let word = sans("limn").size();
    assert!(word.width > 20.0 && word.width < 40.0, "{word:?}");
    assert!(word.height > 14.0 && word.height < 20.0, "{word:?}");
    let empty = sans("");
    assert_eq!(empty.size().width, 0.0);
    assert_eq!(empty.size().height, word.height);
    assert_eq!(empty.lines().map(|line| line.range).collect::<Vec<_>>(), vec![0..0]);
    assert!(empty.draw(Point::new(0.0, 0.0), INK).is_empty());
    // Two lines are twice as tall as one.
    assert_eq!(sans("one\ntwo").size().height, 2.0 * word.height);
}

#[test]
fn shaping_kerns_pairs_that_a_sum_of_widths_would_not() {
    let pair = sans("AV").size().width;
    let apart = sans("A").size().width + sans("V").size().width;
    assert!(pair < apart - 0.5, "AV is {pair} wide, A and V apart {apart}");
}

#[test]
fn wrapping_breaks_between_words_without_losing_or_repeating_text() {
    let layout = sans(PROSE);
    assert_eq!(layout.lines().count(), 1);
    let wrapped = layout.wrapped(120.0);
    let lines: Vec<_> = wrapped.lines().collect();
    assert!(lines.len() >= 4, "{lines:?}");
    for line in &lines {
        assert!(line.rect.width <= 120.0 + 0.01, "{line:?}");
        assert!(line.baseline > line.rect.y && line.baseline < line.rect.y + line.rect.height);
    }
    // The lines cover the text once, in order.
    assert_eq!(lines.first().unwrap().range.start, 0);
    assert_eq!(lines.last().unwrap().range.end, PROSE.len());
    for pair in lines.windows(2) {
        assert_eq!(pair[0].range.end, pair[1].range.start);
        assert!(pair[1].rect.y >= pair[0].rect.y + pair[0].rect.height - 0.01);
    }
    assert_eq!(wrapped.text(), PROSE);
    // A width that is not a number does not wrap.
    assert_eq!(layout.wrapped(f32::NAN).lines().count(), 1);
}

#[test]
fn the_caret_moves_right_through_a_line_and_a_click_finds_it_again() {
    let text = "Wavy line, fine";
    let layout = sans(text);
    let mut last = -1.0;
    for index in boundaries(text) {
        let caret = layout.caret(index);
        assert!(caret.x > last, "caret {index} at {caret:?}");
        assert_eq!(caret.width, 0.0);
        assert!(caret.height > 14.0);
        last = caret.x;
        let middle = Point::new(caret.x, caret.y + caret.height / 2.0);
        assert_eq!(layout.index_at(middle), index, "a click at the caret before byte {index}");
    }
    assert!((layout.caret(text.len()).x - layout.size().width).abs() < 0.01);
    // Past the end is the end; inside a character is its start.
    assert_eq!(layout.caret(usize::MAX), layout.caret(text.len()));
    let accented = sans("né");
    assert_eq!(accented.caret(2), accented.caret(1));
    assert_eq!(layout.index_at(Point::new(f32::NAN, 0.0)), 0);
    assert_eq!(layout.index_at(Point::new(-50.0, 5.0)), 0);
    assert_eq!(layout.index_at(Point::new(1.0e6, 5.0)), text.len());
}

#[test]
fn a_selection_covers_each_line_it_touches() {
    let wrapped = sans(PROSE).wrapped(120.0);
    let lines: Vec<_> = wrapped.lines().collect();
    assert!(wrapped.selection(3..3).is_empty());
    // A range given backwards, as a drag to the left gives one, covers nothing.
    let (from, to) = (9, 3);
    assert!(wrapped.selection(from..to).is_empty());
    let first = wrapped.selection(0..4);
    assert_eq!(first.len(), 1);
    assert!((first[0].x - wrapped.caret(0).x).abs() < 0.01);
    assert!((first[0].width - (wrapped.caret(4).x - wrapped.caret(0).x)).abs() < 0.01);
    let across = wrapped.selection(2..lines[2].range.start + 2);
    assert_eq!(across.len(), 3);
    for (rect, line) in across.iter().zip(&lines) {
        assert!((rect.y - line.rect.y).abs() < 0.01, "{rect:?} on {line:?}");
    }
}

#[test]
fn a_character_the_face_lacks_comes_from_a_fallback() {
    let layout = sans("sum 𝑥 😀");
    let runs = runs(&layout);
    let sans = &runs[0].font;
    assert_eq!(&*runs[0].text, "sum ");
    assert!(runs.iter().any(|run| run.font != *sans && run.text.contains('𝑥')), "{runs:?}");
    assert!(runs.iter().any(|run| run.font != *sans && run.text.contains('😀')), "{runs:?}");
    assert!(runs.iter().all(|run| run.glyphs.iter().all(|glyph| glyph.id != 0)), "no missing glyphs");
    let shown: String = runs.iter().map(|run| &*run.text).collect();
    assert_eq!(shown, "sum 𝑥 😀");
}

#[test]
fn a_run_is_drawn_from_the_origin_in_the_ink_given() {
    let layout = sans("limn");
    let origin = Point::new(30.0, 40.0);
    let commands = layout.draw(origin, INK);
    let [DrawCommand::Glyphs(run)] = commands.as_slice() else {
        panic!("one run: {commands:?}");
    };
    assert_eq!((run.origin, run.color, run.size), (origin, INK, 14.0));
    assert_eq!(run.glyphs.len(), 4);
    let baseline = layout.lines().next().unwrap().baseline;
    assert!(run.glyphs.iter().all(|glyph| glyph.y == baseline));
    assert!(run.glyphs.windows(2).all(|pair| pair[0].x < pair[1].x));
    assert_eq!((run.embolden, run.skew), (false, None));
    // A bundled face is compiled in, so a backend can keep it without a copy.
    assert!(run.font.static_bytes().is_some_and(|bytes| bytes == run.font.bytes()));
}

#[test]
fn weights_have_faces_of_their_own_and_italic_is_slanted() {
    let regular = TextLayout::new("Bold", TextStyle::new(14.0, FontFamily::Sans));
    let bold = TextLayout::new("Bold", TextStyle::new(14.0, FontFamily::SansBold));
    let bundled = fonts();
    let bold_face = bundled.face_of(FontFamily::SansBold).unwrap();
    assert_eq!(&runs(&bold)[0].font, bold_face);
    assert!(!runs(&bold)[0].embolden, "a real bold face, not a thickened regular one");
    assert!(bold.size().width > regular.size().width);
    let italic = runs(&TextLayout::new("Slant", TextStyle::new(14.0, FontFamily::SansItalic)));
    assert_eq!(&italic[0].font, bundled.face_of(FontFamily::Sans).unwrap());
    assert!(italic[0].skew.is_some_and(|degrees| degrees > 0.0), "{:?}", italic[0].skew);
    // Bold italic, without a face of its own, is the bold face slanted.
    let both = runs(&TextLayout::new("Both", TextStyle::new(14.0, FontFamily::SansBoldItalic)));
    assert_eq!(&both[0].font, bold_face);
    assert!(!both[0].embolden && both[0].skew.is_some());
}

#[test]
fn code_is_set_in_a_monospace_face() {
    let narrow = TextLayout::new("iiii", TextStyle::new(14.0, FontFamily::Monospace)).size().width;
    let wide = TextLayout::new("WWWW", TextStyle::new(14.0, FontFamily::Monospace)).size().width;
    assert!((narrow - wide).abs() < 0.01);
}

#[test]
fn numbers_that_are_not_finite_never_reach_the_layout() {
    let style = TextStyle::new(f32::NAN, FontFamily::Sans).line_height(f32::INFINITY);
    assert_eq!(style.size, 14.0);
    assert_eq!(style.line_height, None);
    assert_eq!(TextStyle::new(-3.0, FontFamily::Sans).size, 0.0);
    let spaced = TextLayout::new("one\ntwo", TextStyle::new(14.0, FontFamily::Sans).line_height(30.0));
    assert_eq!(spaced.size().height, 60.0);
}

#[test]
fn a_right_to_left_run_is_placed_right_to_left_and_a_tab_is_a_space() {
    let text = "abc אבג";
    let layout = sans(text);
    let (alef, bet) = (text.find('א').unwrap(), text.find('ב').unwrap());
    // The first letter of the Hebrew word stands rightmost; its end is at its left.
    assert!(layout.caret(alef).x > layout.caret(bet).x);
    assert!(layout.caret(bet).x > layout.caret(text.len()).x);
    assert_eq!(sans("a\tb").size().width, sans("a b").size().width);
}

#[test]
fn centered_lines_stand_in_the_middle_of_the_width_with_their_glyphs() {
    let width = sans("limn").size().width;
    let centered = sans("limn").centered(200.0);
    let line = centered.lines().next().unwrap();
    assert!((line.rect.x - (200.0 - width) / 2.0).abs() < 0.01, "{line:?}");
    assert!((runs(&centered)[0].glyphs[0].x - line.rect.x).abs() < 1.0);
    assert!((centered.caret(0).x - line.rect.x).abs() < 0.01);
}

#[test]
fn a_layout_and_its_clones_draw_the_same_glyphs_every_time() {
    let layout = sans("limn");
    let (first, again) = (runs(&layout), runs(&layout.clone()));
    assert!(std::sync::Arc::ptr_eq(&first[0].glyphs, &again[0].glyphs));
    // Rewrapped, it is another layout with glyphs of its own.
    assert!(!std::sync::Arc::ptr_eq(&first[0].glyphs, &runs(&layout.wrapped(10.0))[0].glyphs));
}

#[test]
fn leading_below_sets_the_glyphs_at_the_top_of_a_tall_line() {
    let tall = |leading| {
        let layout = TextLayout::new("limn", TextStyle::new(14.0, FontFamily::Sans).line_height(40.0).leading(leading));
        let line = layout.lines().next().unwrap();
        (line.baseline, runs(&layout)[0].glyphs[0].y, line.rect)
    };
    let (even, even_glyph, even_line) = tall(limn::Leading::Even);
    let (below, below_glyph, below_line) = tall(limn::Leading::Below);
    assert_eq!(even_line, below_line, "the line takes the same room either way");
    assert_eq!((even_glyph, below_glyph), (even, below), "the glyphs stand on the baseline told");
    // Half of the 40 - 16 spare pixels moves from above the glyphs to below them.
    let natural = sans("limn").size().height;
    assert!((even - below - (40.0 - natural) / 2.0).abs() < 1.0, "{even} {below} {natural}");
}
