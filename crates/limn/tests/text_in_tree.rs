//! `Element::text` laid out in a `TextLayout` once a tree is told to with
//! `UiTree::set_text_layout`, as limn-egui's `attach_fonts` tells it.

// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use limn::{
    Color, DrawCommand, Element, FontFamily, Frame, GlyphRun, Point, Size, TextLayout, TextStyle, UiEvent, UiTree,
};

const INK: Color = Color::rgb(10, 20, 30);
const PROSE: &str = "Text the tree lays out itself, broken where the language allows.";

fn tree(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_text_layout(true);
    tree
}

fn runs(frame: &Frame) -> Vec<&GlyphRun> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run),
            _ => None,
        })
        .collect()
}

#[test]
fn text_is_drawn_as_glyphs_and_sized_as_its_layout() {
    let mut tree = tree(Element::column(vec![
        Element::text("limn").foreground(INK).font_family(FontFamily::Sans),
        Element::spacer().flex(1.0),
    ]));
    let frame = tree.layout(Size::new(300.0, 200.0));
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
    let runs = runs(&frame);
    assert_eq!(runs.len(), 1);
    assert_eq!((&*runs[0].text, runs[0].color, runs[0].origin), ("limn", INK, Point::new(0.0, 0.0)));
    let blocks = frame.text_layouts();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].source, "limn");
    let width = TextLayout::new("limn", TextStyle::new(14.0, FontFamily::Sans)).size().width;
    assert!((blocks[0].runs[0].rect.width - width).abs() < 0.01);
}

#[test]
fn text_given_back_its_own_width_keeps_every_letter_on_its_line() {
    // A row hands text that shrinks to fit the width it measured, by sums
    // that can come out a rounding error short of it.
    for label in ["Compact", "Regular", "Spacious", "Unchecked", "Indeterminate", "Send me the summary"] {
        for size in [12.0, 13.0, 14.0, 15.0] {
            for width in (240..=420).step_by(9) {
                let text = Element::text(label).font_family(FontFamily::Sans).font_size(size).shrink_to_fit();
                let row = Element::row(vec![Element::spacer().size(Some(16.0), None), text]).gap(8.0).padding(4.0);
                let mut tree = tree(Element::column(vec![Element::row(vec![row, Element::spacer().flex(1.0)])]));
                let frame = tree.layout(Size::new(width as f32, 200.0));
                let lines = &frame.text_layouts()[0].runs;
                assert_eq!(lines.len(), 1, "{label:?} at {size} in {width}: {lines:?}");
            }
        }
    }
}

#[test]
fn wrapped_text_keeps_the_line_height_of_the_metrics_and_never_passes_its_width() {
    let mut tree = tree(Element::row(vec![
        Element::text(PROSE).font_family(FontFamily::Sans).size(Some(140.0), None),
        Element::spacer().flex(1.0),
    ]));
    let frame = tree.layout(Size::new(400.0, 400.0));
    let lines = &frame.text_layouts()[0].runs;
    assert!(lines.len() >= 3, "{lines:?}");
    for (index, line) in lines.iter().enumerate() {
        assert!(line.rect.width <= 140.0 + 0.01, "{line:?}");
        // The tree's default metrics: 18 pixels a line at 14.
        assert!((line.rect.y - index as f32 * 18.0).abs() < 0.01, "{line:?}");
        assert!((line.rect.height - 18.0).abs() < 0.01);
    }
    let shown: String = lines.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(shown, PROSE);
}

#[test]
fn centered_text_stands_in_the_middle_of_its_box() {
    let mut tree = tree(Element::row(vec![
        Element::text("limn").font_family(FontFamily::Sans).size(Some(200.0), None).center_content(),
        Element::spacer().flex(1.0),
    ]));
    let frame = tree.layout(Size::new(300.0, 100.0));
    let width = TextLayout::new("limn", TextStyle::new(14.0, FontFamily::Sans)).size().width;
    let line = &frame.text_layouts()[0].runs[0];
    assert!((line.rect.x - (200.0 - width) / 2.0).abs() < 0.01, "{line:?}");
    let first = runs(&frame)[0].glyphs[0];
    assert!((runs(&frame)[0].origin.x + first.x - line.rect.x).abs() < 1.0);
}

#[test]
fn a_drag_across_text_selects_what_its_glyphs_show() {
    let mut tree = tree(Element::column(vec![Element::text("one two three").font_family(FontFamily::Sans)]));
    let frame = tree.layout(Size::new(300.0, 100.0));
    let layout = TextLayout::new("one two three", TextStyle::new(14.0, FontFamily::Sans));
    // From the start of "two" to the end of it, by where the layout puts them.
    let (start, end) = (layout.caret(4), layout.caret(7));
    tree.handle_event(UiEvent::PointerDown(Point::new(start.x + 0.5, 9.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(end.x - 0.5, 9.0)));
    assert_eq!(tree.selected_text(), Some("two"));
    // The selection is painted under the glyphs, over the same span.
    let frame_after = tree.layout(Size::new(300.0, 100.0));
    let fill = frame_after.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, .. } if *color == Color::rgba(180, 180, 180, 84) => Some(*rect),
        _ => None,
    });
    let fill = fill.expect("a selection fill");
    assert!((fill.x - start.x).abs() < 0.01 && (fill.x + fill.width - end.x).abs() < 0.01, "{fill:?}");
    assert_eq!(runs(&frame).len(), runs(&frame_after).len());
}

#[test]
fn a_link_under_the_pointer_is_underlined_across_its_text() {
    let link = Element::clickable("docs", Element::text("Read the guide").foreground(INK).font_family(FontFamily::Sans))
        .underline_on_hover();
    let mut tree = tree(Element::column(vec![link]));
    tree.layout(Size::new(300.0, 100.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(5.0, 5.0)));
    let frame = tree.layout(Size::new(300.0, 100.0));
    let width = TextLayout::new("Read the guide", TextStyle::new(14.0, FontFamily::Sans)).size().width;
    let underline = frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, .. } if *color == INK && rect.height == 1.0 => Some(*rect),
        _ => None,
    });
    let underline = underline.expect("an underline in the text's ink");
    assert!((underline.width - width).abs() < 0.01, "{underline:?}");
    assert!((underline.y - 16.0).abs() < 0.01, "two pixels above the line's foot: {underline:?}");
}

#[test]
fn text_that_stays_keeps_its_glyphs_from_one_layout_to_the_next() {
    let mut tree = tree(Element::column(vec![Element::text("limn").font_family(FontFamily::Sans)]));
    let first = tree.layout(Size::new(300.0, 100.0));
    let again = tree.layout(Size::new(300.0, 100.0));
    assert!(std::sync::Arc::ptr_eq(&runs(&first)[0].glyphs, &runs(&again)[0].glyphs));
    // A tree that is not told keeps measuring through the text measurer.
    let mut measured = UiTree::new(Element::column(vec![Element::text("limn")]));
    let frame = measured.layout(Size::new(300.0, 100.0));
    assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
    assert!(runs(&frame).is_empty());
}

#[test]
fn a_long_word_wraps_inside_its_box() {
    let address = "Pneumonoultramicroscopicsilicovolcanoconiosisandmorelettersstill";
    let mut tree = tree(Element::row(vec![
        Element::text(address).font_family(FontFamily::Sans).size(Some(120.0), None),
        Element::spacer().flex(1.0),
    ]));
    let frame = tree.layout(Size::new(400.0, 400.0));
    let lines = &frame.text_layouts()[0].runs;
    assert!(lines.len() > 1, "{lines:?}");
    assert!(lines.iter().all(|line| line.rect.x + line.rect.width <= 120.0 + 0.01), "{lines:?}");
}

mod formatted {
    use super::*;
    use limn::{Checkbox, Component, ComponentPalette, ControlSize, HitKind, Key, Markdown, TextSpan, UiAction};
    use std::time::{Duration, Instant};

    const LINK: Color = Color::rgb(40, 110, 220);

    fn span(text: &str) -> TextSpan {
        TextSpan::new(text, INK, 14.0, FontFamily::Sans)
    }

    fn paragraph(spans: Vec<TextSpan>) -> UiTree {
        tree(Element::column(vec![Element::formatted_text(spans).size(Some(220.0), None)]))
    }

    #[test]
    fn a_label_given_back_its_own_width_keeps_every_letter_on_its_line() {
        // A row hands a label that shrinks to fit the width it measured, by
        // sums that can come out a rounding error short of it.
        let palette = ComponentPalette::neutral_dark();
        for label in ["Compact", "Regular", "Spacious", "Unchecked", "Indeterminate", "Send me the summary"] {
            for size in [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious] {
                for width in (240..=420).step_by(9) {
                    let checkbox = Checkbox::new("box", false, palette).label(label).size(size).render();
                    let mut tree = tree(Element::column(vec![Element::row(vec![checkbox, Element::spacer().flex(1.0)])]));
                    let frame = tree.layout(Size::new(width as f32, 200.0));
                    let lines = &frame.text_layouts()[0].runs;
                    assert_eq!(lines.len(), 1, "{label:?} {size:?} at {width}: {lines:?}");
                }
            }
        }
    }

    #[test]
    fn spans_are_drawn_as_glyphs_in_their_own_inks() {
        let mut tree = paragraph(vec![span("Plain and "), TextSpan::new("linked", LINK, 14.0, FontFamily::SansBold)]);
        let frame = tree.layout(Size::new(300.0, 200.0));
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
        let inks: Vec<_> = runs(&frame).iter().map(|run| (run.text.to_string(), run.color)).collect();
        assert_eq!(inks, vec![("Plain and ".to_owned(), INK), ("linked".to_owned(), LINK)]);
        assert_eq!(frame.text_layouts()[0].source, "Plain and linked");
    }

    #[test]
    fn an_inline_action_is_pressed_reached_by_tab_and_underlined_under_the_pointer() {
        let spans = vec![span("Read "), TextSpan::new("the guide", LINK, 14.0, FontFamily::Sans).action("guide"), span(" first.")];
        let mut tree = paragraph(spans);
        let frame = tree.layout(Size::new(300.0, 200.0));
        let target = frame.regions.iter().find(|region| region.id == "guide" && region.kind == HitKind::Button).expect("a target").rect;
        let layout = TextLayout::new("Read ", TextStyle::new(14.0, FontFamily::Sans));
        assert!((target.x - layout.size().width).abs() < 0.5, "{target:?}");
        let center = Point::new(target.x + target.width / 2.0, target.y + target.height / 2.0);
        tree.handle_event(UiEvent::PointerMove(center));
        tree.handle_event(UiEvent::PointerDown(center));
        assert!(tree.handle_event(UiEvent::PointerUp(center)).contains(&UiAction::Clicked { id: "guide".into() }));
        let hovered = tree.layout(Size::new(300.0, 200.0));
        let underline = hovered.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. } if *color == LINK && rect.height == 1.0 && (rect.x - target.x).abs() < 0.01));
        assert!(underline, "the action under the pointer is underlined");
        tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false });
        assert_eq!(tree.focused_id(), Some("guide"));
    }

    #[test]
    fn a_runs_hint_shows_over_it_after_the_hover_delay() {
        let palette = ComponentPalette::neutral_dark();
        let spans = vec![span("Sent "), span("yesterday").tooltip("Tuesday at 22:13", palette)];
        let mut tree = paragraph(spans);
        let start = Instant::now();
        let frame = tree.layout_at(Size::new(300.0, 200.0), start);
        let target = frame.regions.iter().find(|region| region.kind == HitKind::Hover).expect("a hint target").rect;
        let center = Point::new(target.x + target.width / 2.0, target.y + target.height / 2.0);
        tree.handle_event_at(UiEvent::PointerMove(center), start);
        let shown = |frame: &Frame| frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text.contains("Tuesday"))
            || matches!(command, DrawCommand::Glyphs(run) if run.text.contains("Tuesday")));
        assert!(!shown(&tree.layout_at(Size::new(300.0, 200.0), start)));
        assert!(shown(&tree.layout_at(Size::new(300.0, 200.0), start + Duration::from_secs(1))));
    }

    #[test]
    fn a_drag_selects_across_spans_by_their_glyphs() {
        let mut tree = paragraph(vec![span("one "), TextSpan::new("two", INK, 14.0, FontFamily::SansBold), span(" three")]);
        tree.layout(Size::new(300.0, 200.0));
        let bold = TextLayout::formatted(
            &[span("one "), TextSpan::new("two", INK, 14.0, FontFamily::SansBold), span(" three")],
            TextStyle::new(14.0, FontFamily::Sans).line_height(18.0),
        );
        let (from, to) = (bold.caret(2), bold.caret(10));
        tree.handle_event(UiEvent::PointerDown(Point::new(from.x + 0.3, 9.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(to.x - 0.3, 9.0)));
        assert_eq!(tree.selected_text(), Some("e two th"));
    }

    #[test]
    fn a_paragraph_cut_to_its_lines_ends_in_an_ellipsis_it_does_not_select() {
        let long = "A channel's topic, long enough to run past the lines it is given in a narrow column of the window.";
        let mut tree = tree(Element::column(vec![Element::formatted_text(vec![span(long)]).size(Some(160.0), None).max_lines(2)]));
        let frame = tree.layout(Size::new(300.0, 200.0));
        let lines = &frame.text_layouts()[0].runs;
        assert_eq!(lines.len(), 2);
        let drawn: String = runs(&frame).iter().map(|run| &*run.text).collect();
        assert!(drawn.ends_with('…'), "{drawn:?}");
        assert!(lines.iter().all(|line| !line.text.contains('…')));
    }

    #[test]
    fn a_picture_is_drawn_in_place_of_its_spans_text() {
        let picture = limn::RasterImage::new("a".repeat(64), limn::RasterImageFormat::Png, 4, std::sync::Arc::from("AAAA"), "dusk").unwrap();
        let mut tree = paragraph(vec![span("Nice "), span(":dusk:").image(picture)]);
        let frame = tree.layout(Size::new(300.0, 200.0));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::RasterImage { rect, .. } if rect.width == 14.0 * limn::INLINE_IMAGE_SCALE)));
        assert!(runs(&frame).iter().all(|run| !run.text.contains("dusk")));
        assert_eq!(frame.text_layouts()[0].source, "Nice :dusk:");
    }

    #[test]
    fn markdown_is_laid_out_by_the_tree_too() {
        let mut tree = tree(Markdown::new("A **bold** word and `code`.", ComponentPalette::neutral_dark()).width(400.0).render().element);
        let frame = tree.layout(Size::new(400.0, 300.0));
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
        let drawn: String = runs(&frame).iter().map(|run| &*run.text).collect();
        assert_eq!(drawn, "A bold word and code.");
    }
}
