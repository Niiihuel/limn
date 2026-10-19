//! `TextLayout::formatted`: spans set in their own sizes, families and inks,
//! with lines, backgrounds, icons and pictures, in the bundled faces.

// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use std::sync::Arc;

use limn::{
    Color, DrawCommand, FontFamily, GlyphRun, Icon, Point, RasterImage, RasterImageFormat, Rect, TextLayout, TextSpan,
    TextStyle,
};

const INK: Color = Color::rgb(10, 20, 30);
const RED: Color = Color::rgb(200, 30, 30);
const CODE: Color = Color::rgb(60, 60, 60);

fn span(text: &str) -> TextSpan {
    TextSpan::new(text, INK, 14.0, FontFamily::Sans)
}

fn paragraph() -> TextStyle {
    TextStyle::new(14.0, FontFamily::Sans).line_height(18.0)
}

fn drawn(layout: &TextLayout) -> Vec<DrawCommand> {
    layout.draw(Point::new(0.0, 0.0), Color::WHITE)
}

fn runs(commands: &[DrawCommand]) -> Vec<&GlyphRun> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run),
            _ => None,
        })
        .collect()
}

fn fills(commands: &[DrawCommand], color: Color) -> Vec<Rect> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
            _ => None,
        })
        .collect()
}

fn picture() -> RasterImage {
    RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "dusk").unwrap()
}

#[test]
fn each_span_is_drawn_in_its_own_ink_and_size_on_one_baseline() {
    let layout = TextLayout::formatted(
        &[span("Plain "), TextSpan::new("red", RED, 20.0, FontFamily::SansBold), span(" again")],
        paragraph(),
    );
    let commands = drawn(&layout);
    let runs = runs(&commands);
    let inks: Vec<_> = runs.iter().map(|run| (&*run.text, run.color, run.size)).collect();
    assert_eq!(inks, vec![("Plain ", INK, 14.0), ("red", RED, 20.0), (" again", INK, 14.0)]);
    let baselines: Vec<f32> = runs.iter().map(|run| run.glyphs[0].y).collect();
    assert!(baselines.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01), "{baselines:?}");
    // The larger span makes its line taller than the paragraph's 18.
    assert!(layout.size().height > 18.0);
    assert_eq!(layout.text(), "Plain red again");
}

#[test]
fn a_line_is_at_least_the_paragraphs_height() {
    let layout = TextLayout::formatted(&[span("one\ntwo")], TextStyle::new(14.0, FontFamily::Sans).line_height(30.0));
    let lines: Vec<_> = layout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| (line.rect.height - 30.0).abs() < 0.01), "{lines:?}");
}

#[test]
fn the_paragraphs_family_draws_nothing_and_makes_no_line_taller() {
    // An element that never chose a family is Monospace, whose face sits
    // lower than the spans' on a line of the same height.
    for family in [FontFamily::Sans, FontFamily::Monospace] {
        let layout = TextLayout::formatted(&[span("one\ntwo")], TextStyle::new(14.0, family).line_height(18.0));
        assert!((layout.size().height - 36.0).abs() < 0.01, "{family:?}: {:?}", layout.size());
    }
}

#[test]
fn underline_and_strikethrough_follow_their_runs_in_their_ink() {
    let layout = TextLayout::formatted(
        &[span("Plain "), TextSpan::new("under", RED, 14.0, FontFamily::Sans).underline().strikethrough()],
        paragraph(),
    );
    let commands = drawn(&layout);
    let lines = fills(&commands, RED);
    assert_eq!(lines.len(), 2, "{lines:?}");
    let under = runs(&commands).into_iter().find(|run| &*run.text == "under").unwrap();
    let baseline = under.glyphs[0].y;
    let (strike, low) = if lines[0].y < lines[1].y { (lines[0], lines[1]) } else { (lines[1], lines[0]) };
    assert!(low.y > baseline - 1.0 && strike.y < baseline - 2.0, "{low:?} {strike:?} on {baseline}");
    assert!(lines.iter().all(|line| (line.height - 1.0).abs() < 0.01 && (line.x - layout.caret(6).x).abs() < 0.5));
}

#[test]
fn a_background_leaves_room_around_its_span_and_keeps_it_on_one_line() {
    let spans = [span("Run "), TextSpan::new("cargo test --workspace", INK, 14.0, FontFamily::Monospace).background(CODE), span(" now")];
    let layout = TextLayout::formatted(&spans, paragraph());
    let fill = fills(&drawn(&layout), CODE);
    assert_eq!(fill.len(), 1);
    let (start, end) = (layout.caret(4).x, layout.caret(26).x);
    assert!((fill[0].x - start).abs() < 0.5, "{:?} from {start}", fill[0]);
    // Three pixels of room on each side: from where the span starts to past its text.
    assert!((fill[0].x + fill[0].width - end - 3.0).abs() < 0.5, "{:?} to {end}", fill[0]);
    // Narrower than the whole line, the code moves down whole.
    let narrow = layout.wrapped(220.0);
    let lines: Vec<_> = narrow.lines().map(|line| narrow.text()[line.range].to_owned()).collect();
    assert!(lines.iter().any(|line| line.contains("cargo test --workspace")), "{lines:?}");
    // Narrower than the code itself, it breaks rather than run past the line.
    let tight = layout.wrapped(90.0);
    assert!(tight.lines().all(|line| line.rect.width <= 90.0 + 0.5), "{:?}", tight.lines().collect::<Vec<_>>());
}

#[test]
fn an_icon_stands_before_its_text() {
    let plain = TextLayout::formatted(&[span("Done")], paragraph());
    let iconned = TextLayout::formatted(&[span("Done").icon(Icon::Check)], paragraph());
    let commands = drawn(&iconned);
    let icon = commands.iter().find_map(|command| match command {
        DrawCommand::Icon { rect, icon: Icon::Check, tint } => Some((*rect, *tint)),
        _ => None,
    });
    let (rect, tint) = icon.expect("the icon");
    assert_eq!((rect.x, rect.width, tint), (0.0, limn::ICON_SIZE, INK));
    assert!((iconned.size().width - plain.size().width - limn::ICON_SIZE - 4.0).abs() < 0.5);
    assert!((runs(&commands)[0].glyphs[0].x - limn::ICON_SIZE - 4.0).abs() < 1.0);
}

#[test]
fn a_picture_takes_its_spans_place_and_is_selected_whole() {
    let spans = [span("Nice "), span(":dusk:").image(picture()), span(" one")];
    let layout = TextLayout::formatted(&spans, paragraph());
    let commands = drawn(&layout);
    assert!(runs(&commands).iter().all(|run| !run.text.contains("dusk")), "its text is never drawn");
    let side = 14.0 * limn::INLINE_IMAGE_SCALE;
    let area = commands.iter().find_map(|command| match command {
        DrawCommand::RasterImage { rect, .. } => Some(*rect),
        _ => None,
    });
    let area = area.expect("the picture");
    assert_eq!((area.width, area.height), (side, side));
    assert!((area.x - layout.caret(5).x).abs() < 0.5);
    // Its text is what a selection takes, and a selection takes it whole.
    assert_eq!(layout.text(), "Nice :dusk: one");
    let covered = layout.selection(5..11);
    assert_eq!(covered.len(), 1);
    assert!((covered[0].width - side).abs() < 0.5, "{covered:?}");
    assert!(layout.size().height >= side);
}

#[test]
fn max_lines_ends_the_last_line_in_an_ellipsis_outside_the_text() {
    let spans = [span("A channel's topic, "), TextSpan::new("with marks", INK, 14.0, FontFamily::SansBold), span(", long enough to run past the lines it is given in a narrow column.")];
    let layout = TextLayout::formatted(&spans, paragraph()).wrapped(200.0);
    assert!(layout.lines().count() > 2);
    let cut = layout.max_lines(2);
    let lines: Vec<_> = cut.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.rect.width <= 200.0 + 0.5), "{lines:?}");
    assert!(cut.text().ends_with('…'));
    let commands = drawn(&cut);
    let shown: String = runs(&commands).iter().map(|run| &*run.text).collect();
    assert!(shown.ends_with('…') && !shown.ends_with(" …"), "{shown:?}");
    // No offset reaches the ellipsis.
    let end = cut.text().len() - '…'.len_utf8();
    assert!(lines.iter().all(|line| line.range.end <= end));
    assert_eq!(cut.index_at(Point::new(1.0e6, 30.0)), end);
    let selected = cut.selection(0..cut.text().len());
    let last = selected.last().expect("a selection");
    assert!(last.x + last.width <= cut.caret(end).x + 0.5, "{selected:?}");
    // A text that fits is left as it is.
    assert_eq!(TextLayout::formatted(&[span("Short")], paragraph()).max_lines(2).text(), "Short");
}

#[test]
fn a_span_is_found_on_each_line_it_runs_onto() {
    let spans = [span("Read the "), span("guide on writing components well").action("guide"), span(".")];
    let unwrapped = TextLayout::formatted(&spans, paragraph());
    assert_eq!(unwrapped.span_bounds(1).len(), 1);
    // Broken again from a layout whose spans were already found.
    let layout = unwrapped.wrapped(160.0);
    let boxes = layout.span_bounds(1);
    assert!(boxes.len() >= 2, "{boxes:?}");
    for (bounds, line) in boxes.iter().zip(layout.lines()) {
        assert!((bounds.height - line.rect.height).abs() < 0.01);
    }
    assert!(layout.span_bounds(9).is_empty());
}

#[test]
fn an_icon_goes_to_the_next_line_with_its_first_word() {
    let spans = [span("details are in the "), span("guide").icon(Icon::Globe), span(" and more")];
    let whole = TextLayout::formatted(&spans, paragraph());
    let start = "details are in the ".len();
    // Just too narrow for the icon and its word on the first line.
    let wrapped = whole.wrapped(whole.caret(start).x + limn::ICON_SIZE + 4.0 + 10.0);
    let commands = drawn(&wrapped);
    let icon = commands.iter().find_map(|command| match command {
        DrawCommand::Icon { rect, .. } => Some(*rect),
        _ => None,
    });
    let icon = icon.expect("the icon");
    let guide = runs(&commands).into_iter().find(|run| &*run.text == "guide").expect("its word");
    let lines: Vec<_> = wrapped.lines().collect();
    assert!(lines.len() >= 2);
    // Both on the second line, the icon first.
    assert!(icon.y > lines[0].rect.y + lines[0].rect.height - 1.0, "{icon:?} {lines:?}");
    assert!((guide.glyphs[0].y - lines[1].baseline).abs() < 0.5);
    assert_eq!(icon.x, 0.0);
}
