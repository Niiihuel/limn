//! Text fields and text areas laid out in a `TextLayout` once a tree is told
//! to with `UiTree::set_text_layout`: what they draw, where the caret and the
//! selection stand, and where a click or a key puts the caret.

// They lay text out in the bundled faces.
#![cfg(feature = "bundled-fonts")]

use limn::{
    Color, DrawCommand, Element, FontFamily, Frame, ImeArea, Key, Point, Rect, Size, TextHighlight, TextLayout,
    TextStyle, UiAction, UiEvent, UiTree,
};

const INK: Color = Color::rgb(10, 20, 30);
const WINDOW: Size = Size::new(400.0, 300.0);

fn tree(field: Element) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![field, Element::spacer().flex(1.0)]));
    tree.set_text_layout(true);
    tree.set_reduced_motion(true);
    tree
}

fn input(value: &str, width: f32) -> UiTree {
    tree(
        Element::text_input("field", value, "Search")
            .font_family(FontFamily::Sans)
            .foreground(INK)
            .size(Some(width), None),
    )
}

fn area(value: &str, width: f32) -> UiTree {
    tree(
        Element::text_area("notes", value, "Write")
            .font_family(FontFamily::Sans)
            .foreground(INK)
            .padding(0.0)
            .size(Some(width), Some(90.0)),
    )
}

/// Paint once, so the field is there to focus, then focus it.
fn focus(tree: &mut UiTree, id: &str) {
    tree.layout(WINDOW);
    assert!(tree.focus(id), "{id} takes the focus");
}

fn key(tree: &mut UiTree, key: Key) {
    tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false });
}

/// The text each glyph run draws, in order.
fn drawn(frame: &Frame) -> Vec<String> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run.text.to_string()),
            _ => None,
        })
        .collect()
}

/// The caret: the one fill a pixel and a half wide.
fn caret(frame: &Frame) -> Option<Rect> {
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, .. } if rect.width == 1.5 => Some(*rect),
        _ => None,
    })
}

/// Where the field's text starts, from the origin of its first glyph run.
fn text_origin(frame: &Frame) -> Point {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run.origin),
            _ => None,
        })
        .expect("a glyph run")
}

fn style() -> TextStyle {
    // A tree's default metrics: 18 pixels a line at 14.
    TextStyle::new(14.0, FontFamily::Sans).line_height(18.0)
}

#[test]
fn a_field_draws_its_value_as_glyphs_and_its_caret_where_the_layout_puts_it() {
    let mut tree = input("limn", 300.0);
    focus(&mut tree, "field");
    let frame = tree.layout(WINDOW);
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
    assert_eq!(drawn(&frame), ["limn"]);
    let caret = caret(&frame).expect("a caret");
    let end = TextLayout::new("limn", style()).caret(4);
    let origin = text_origin(&frame);
    assert!((caret.x - (origin.x + end.x)).abs() < 0.01, "{caret:?}");
    assert_eq!(caret.height, 18.0);
}

#[test]
fn a_click_puts_the_caret_at_the_nearest_boundary_of_the_layout() {
    let mut tree = input("interfaces", 300.0);
    let frame = tree.layout(WINDOW);
    let origin = text_origin(&frame);
    let layout = TextLayout::new("interfaces", style());
    // Just right of the middle of the fifth letter's box.
    let (before, after) = (layout.caret(4).x, layout.caret(5).x);
    let x = origin.x + before + (after - before) * 0.6;
    tree.handle_event(UiEvent::PointerDown(Point::new(x, origin.y + 9.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(x, origin.y + 9.0)));
    assert_eq!(tree.focused_id(), Some("field"));
    assert_eq!(tree.input_cursor("field"), Some(5));
}

#[test]
fn a_long_value_scrolls_to_keep_the_caret_in_view_and_back_to_its_start() {
    let value = "a value far longer than the narrow field it is typed into";
    let mut tree = input(value, 120.0);
    focus(&mut tree, "field");
    let frame = tree.layout(WINDOW);
    let caret_at_end = caret(&frame).expect("a caret");
    let field = frame.regions.iter().find(|region| region.id == "field").expect("the field").rect;
    assert!(caret_at_end.x <= field.x + field.width, "{caret_at_end:?} past {field:?}");
    assert!(text_origin(&frame).x < field.x, "scrolled left to show the end");
    key(&mut tree, Key::Home);
    let frame = tree.layout(WINDOW);
    let caret_at_start = caret(&frame).expect("a caret");
    assert!((caret_at_start.x - text_origin(&frame).x).abs() < 0.01, "back at the start");
    assert!(text_origin(&frame).x >= field.x);
}

#[test]
fn a_password_draws_a_dot_for_each_character_and_never_its_value() {
    let mut tree = tree(Element::text_input("secret", "hunter2", "").password(true).font_family(FontFamily::Sans));
    let frame = tree.layout(WINDOW);
    assert_eq!(drawn(&frame), ["•••••••"]);
}

#[test]
fn an_empty_field_draws_its_placeholder_faint() {
    let mut tree = input("", 300.0);
    let frame = tree.layout(WINDOW);
    let run = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Glyphs(run) => Some(run),
            _ => None,
        })
        .expect("the placeholder");
    assert_eq!((&*run.text, run.color.a), ("Search", 110));
}

#[test]
fn typed_text_is_laid_out_again_before_the_next_paint() {
    let mut tree = input("ab", 300.0);
    focus(&mut tree, "field");
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::Text("c".into()));
    let frame = tree.layout(WINDOW);
    assert_eq!(drawn(&frame), ["abc"]);
    let end = TextLayout::new("abc", style()).caret(3);
    assert!((caret(&frame).expect("a caret").x - (text_origin(&frame).x + end.x)).abs() < 0.01);
}

#[test]
fn a_text_area_breaks_its_lines_where_the_layout_does() {
    let value = "Text the tree lays out itself, broken where the language allows.";
    let mut tree = area(value, 160.0);
    let frame = tree.layout(WINDOW);
    let layout = TextLayout::new(value, style()).wrapped(160.0);
    let lines: Vec<String> = layout.lines().map(|line| value[line.range].to_owned()).collect();
    assert!(lines.len() >= 3, "{lines:?}");
    assert_eq!(drawn(&frame).concat(), value);
    assert_eq!(frame.text_layouts().len(), 0, "a field is not a selectable static text");
}

#[test]
fn up_and_down_keep_the_column_in_pixels_across_shorter_lines() {
    let mut tree = area("wide wide wide\nmm\nwide wide wide", 400.0);
    focus(&mut tree, "notes");
    tree.layout(WINDOW);
    key(&mut tree, Key::Home);
    // From the end of the first line's third word to the short line, and on.
    tree.handle_event(UiEvent::Key { key: Key::Home, shift: false, ctrl: true });
    for _ in 0..12 {
        key(&mut tree, Key::ArrowRight);
    }
    let first = caret(&tree.layout(WINDOW)).expect("a caret");
    key(&mut tree, Key::ArrowDown);
    let short = caret(&tree.layout(WINDOW)).expect("a caret");
    assert!(short.x < first.x, "the short line ends before the column");
    assert_eq!(tree.input_cursor("notes"), Some(17), "at the end of \"mm\"");
    key(&mut tree, Key::ArrowDown);
    let third = caret(&tree.layout(WINDOW)).expect("a caret");
    assert!((third.x - first.x).abs() < 4.0, "back at the column: {third:?} {first:?}");
    assert!(third.y > short.y && short.y > first.y);
}

#[test]
fn home_and_end_go_to_the_edges_of_the_wrapped_line_the_caret_is_on() {
    let value = "one two three four five six seven eight nine ten";
    let mut tree = area(value, 120.0);
    focus(&mut tree, "notes");
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::Key { key: Key::Home, shift: false, ctrl: true });
    key(&mut tree, Key::ArrowDown);
    let line = caret(&tree.layout(WINDOW)).expect("a caret").y;
    key(&mut tree, Key::End);
    let end = caret(&tree.layout(WINDOW)).expect("a caret");
    assert_eq!(end.y, line, "End stays on the wrapped line");
    let at = tree.input_cursor("notes").expect("a caret");
    assert_ne!(value.chars().nth(at.saturating_sub(1)), Some(' '), "before the space the line hangs");
    key(&mut tree, Key::Home);
    let home = caret(&tree.layout(WINDOW)).expect("a caret");
    assert_eq!(home.y, line);
    assert!(home.x < end.x);
}

#[test]
fn a_text_area_scrolls_to_its_caret_and_draws_only_the_lines_in_view() {
    let value: String = (1..=30).map(|line| format!("line {line}\n")).collect();
    let mut tree = area(&value, 200.0);
    focus(&mut tree, "notes");
    let frame = tree.layout(WINDOW);
    let caret = caret(&frame).expect("a caret at the end");
    let field = frame.regions.iter().find(|region| region.id == "notes").expect("the area").rect;
    assert!(caret.y >= field.y && caret.y + caret.height <= field.y + field.height + 0.01, "{caret:?} in {field:?}");
    let lines = drawn(&frame);
    assert!(lines.len() <= 7, "only what fits 90 pixels, and a line either side: {lines:?}");
    assert!(lines.iter().any(|line| line.contains("line 30")));
}

#[test]
fn an_area_that_grows_is_as_tall_as_its_wrapped_lines() {
    let value = "Text the tree lays out itself, broken where the language allows.";
    let mut tree = tree(
        Element::text_area("notes", value, "")
            .text_area_auto_grow(18.0, 400.0)
            .font_family(FontFamily::Sans)
            .padding(0.0)
            .size(Some(160.0), None),
    );
    let frame = tree.layout(WINDOW);
    let field = frame.regions.iter().find(|region| region.id == "notes").expect("the area").rect;
    let lines = TextLayout::new(value, style()).wrapped(160.0).lines().count();
    assert_eq!(field.height, lines as f32 * 18.0);
}

#[test]
fn highlights_are_spans_in_their_inks_and_slants_over_their_fills() {
    let red = Color::rgb(200, 30, 30);
    let pill = Color::rgb(40, 60, 90);
    let value = "ping @ada about it";
    let mut tree = tree(
        Element::text_area("notes", value, "")
            .text_area_highlights(vec![
                TextHighlight { range: 5..9, color: red, italic: false, background: Some(pill) },
                TextHighlight { range: 10..15, color: INK, italic: true, background: None },
            ])
            .font_family(FontFamily::Sans)
            .foreground(INK)
            .padding(0.0)
            .size(Some(300.0), Some(60.0)),
    );
    let frame = tree.layout(WINDOW);
    let runs: Vec<(String, Color)> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Glyphs(run) => Some((run.text.to_string(), run.color)),
            _ => None,
        })
        .collect();
    assert!(runs.contains(&("@ada".to_owned(), red)), "{runs:?}");
    let slanted = frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Glyphs(run) if &*run.text == "about" && (run.skew.is_some() || run.font.id() != frame_font(&frame, "ping "))));
    assert!(slanted, "the italic range is set in the italic face");
    let fill = frame.commands.iter().position(|command| matches!(command, DrawCommand::FillRect { color, radius, .. } if *color == pill && *radius == 3.0));
    let text = frame.commands.iter().position(|command| matches!(command, DrawCommand::Glyphs(_)));
    assert!(fill.expect("the pill's fill") < text.expect("the text"), "the fill goes under the text");
}

/// The font a run of `text` is drawn from.
fn frame_font(frame: &Frame, text: &str) -> u64 {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Glyphs(run) if &*run.text == text => Some(run.font.id()),
            _ => None,
        })
        .expect("a run")
}

/// The lines under what an input method composes: fills a pixel high.
fn underlines(frame: &Frame) -> Vec<Rect> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if rect.height == 1.0 && *color == INK => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn a_composition_is_shown_at_the_caret_underlined_and_is_not_the_value_yet() {
    let mut tree = input("ab", 300.0);
    focus(&mut tree, "field");
    key(&mut tree, Key::ArrowLeft);
    assert!(tree.handle_event(UiEvent::ImePreedit("xyz".into())).is_empty());
    let frame = tree.layout(WINDOW);
    assert_eq!(drawn(&frame), ["axyzb"]);
    assert_eq!(tree.input_value("field"), Some("ab"));
    let layout = TextLayout::new("axyzb", style());
    let origin = text_origin(&frame);
    let lines = underlines(&frame);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!((lines[0].x - (origin.x + layout.caret(1).x)).abs() < 0.01);
    assert!((lines[0].width - (layout.caret(4).x - layout.caret(1).x)).abs() < 0.01);
    let caret = caret(&frame).expect("a caret");
    assert!((caret.x - (origin.x + layout.caret(4).x)).abs() < 0.01, "after what is composed");
}

#[test]
fn a_commit_takes_the_composition_into_the_value_and_a_cancel_drops_it() {
    let mut tree = input("ab", 300.0);
    focus(&mut tree, "field");
    tree.handle_event(UiEvent::ImePreedit("x".into()));
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::ImeCancelled);
    let frame = tree.layout(WINDOW);
    assert_eq!((drawn(&frame), tree.input_value("field")), (vec!["ab".to_owned()], Some("ab")));
    assert!(underlines(&frame).is_empty());
    tree.handle_event(UiEvent::ImePreedit("yz".into()));
    tree.layout(WINDOW);
    let actions = tree.handle_event(UiEvent::ImeCommit("yz".into()));
    assert!(actions.contains(&UiAction::InputChanged { id: "field".into(), value: "abyz".into() }), "{actions:?}");
    let frame = tree.layout(WINDOW);
    assert_eq!(drawn(&frame), ["abyz"]);
    assert!(underlines(&frame).is_empty());
}

#[test]
fn a_composition_stands_in_for_the_selection_until_it_commits() {
    let mut tree = input("abc", 300.0);
    focus(&mut tree, "field");
    tree.handle_event(UiEvent::Key { key: Key::ArrowLeft, shift: true, ctrl: false });
    tree.handle_event(UiEvent::Key { key: Key::ArrowLeft, shift: true, ctrl: false });
    tree.handle_event(UiEvent::ImePreedit("z".into()));
    let frame = tree.layout(WINDOW);
    assert_eq!(drawn(&frame), ["az"]);
    let selection = Color::rgba(75, 130, 210, 130);
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == selection)), "no selection over it");
    tree.handle_event(UiEvent::ImeCancelled);
    let frame = tree.layout(WINDOW);
    assert_eq!(tree.input_selection("field"), Some((1, 3)), "the selection is back");
    assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == selection)));
    tree.handle_event(UiEvent::ImePreedit("z".into()));
    tree.handle_event(UiEvent::ImeCommit("z".into()));
    assert_eq!(tree.input_value("field"), Some("az"));
}

#[test]
fn an_input_method_deletes_around_the_caret() {
    let mut tree = input("abcd", 300.0);
    focus(&mut tree, "field");
    key(&mut tree, Key::ArrowLeft);
    let actions = tree.handle_event(UiEvent::ImeDeleteSurrounding { before_chars: 2, after_chars: 1 });
    assert_eq!(actions, [UiAction::InputChanged { id: "field".into(), value: "a".into() }]);
    assert_eq!(tree.input_cursor("field"), Some(1));
}

#[test]
fn a_text_area_composes_on_the_line_its_caret_is_on() {
    let value = "one two three four five six seven eight";
    let mut tree = area(value, 120.0);
    focus(&mut tree, "notes");
    tree.handle_event(UiEvent::Key { key: Key::Home, shift: false, ctrl: true });
    key(&mut tree, Key::ArrowDown);
    let line = caret(&tree.layout(WINDOW)).expect("a caret").y;
    tree.handle_event(UiEvent::ImePreedit("zz".into()));
    let frame = tree.layout(WINDOW);
    let lines = underlines(&frame);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!((lines[0].y - (line + 17.0)).abs() < 0.01, "at the bottom of the caret's line");
    assert_eq!(tree.input_value("notes"), Some(value));
}

#[test]
fn an_input_method_belongs_by_the_focused_fields_caret() {
    let mut tree = input("ab", 300.0);
    tree.layout(WINDOW);
    assert_eq!(tree.ime_area(), None, "nothing has the focus");
    focus(&mut tree, "field");
    let frame = tree.layout(WINDOW);
    let field = frame.regions.iter().find(|region| region.id == "field").expect("the field").rect;
    let ImeArea { area, caret: at, secret } = tree.ime_area().expect("an area");
    assert_eq!(at, caret(&frame).expect("a caret"));
    assert!(area.x >= field.x && area.x + area.width <= field.x + field.width);
    assert!(!secret);
    tree.set_window_focused(false);
    assert_eq!(tree.ime_area(), None, "the window is not focused");

    let mut tree = tree_of(Element::text_input("secret", "", "").password(true).font_family(FontFamily::Sans));
    focus(&mut tree, "secret");
    tree.layout(WINDOW);
    assert!(tree.ime_area().expect("an area").secret, "a password composes nothing");
}

fn tree_of(field: Element) -> UiTree {
    tree(field)
}

/// Where the caret before the character `at` of `value` stands in a field
/// painted in `frame`, a little below the top of its line.
fn point_at(frame: &Frame, value: &str, at: usize) -> Point {
    let layout = TextLayout::new(value, style());
    let byte = value.char_indices().nth(at).map_or(value.len(), |(byte, _)| byte);
    let origin = text_origin(frame);
    Point::new(origin.x + layout.caret(byte).x, origin.y + 6.0)
}

fn press(tree: &mut UiTree, at: Point, shift: bool, click_count: u8) {
    tree.handle_event(UiEvent::PointerDownEditor { position: at, shift, alt: false, ctrl: false, click_count });
    tree.handle_event(UiEvent::PointerUp(at));
}

fn ctrl(tree: &mut UiTree, key: Key, shift: bool) {
    tree.handle_event(UiEvent::Key { key, shift, ctrl: true });
}

#[test]
fn a_drag_selects_from_where_it_was_pressed() {
    let value = "interfaces";
    let mut tree = input(value, 300.0);
    let frame = tree.layout(WINDOW);
    let (from, to) = (point_at(&frame, value, 7), point_at(&frame, value, 2));
    tree.handle_event(UiEvent::PointerDown(from));
    tree.handle_event(UiEvent::PointerMove(point_at(&frame, value, 4)));
    tree.handle_event(UiEvent::PointerMove(to));
    tree.handle_event(UiEvent::PointerUp(to));
    assert_eq!(tree.input_selection("field"), Some((2, 7)));
    // The press is over: moving again selects nothing more.
    tree.handle_event(UiEvent::PointerMove(point_at(&frame, value, 0)));
    assert_eq!(tree.input_selection("field"), Some((2, 7)));
    assert_eq!(tree.input_cursor("field"), Some(2), "the caret is where the drag ended");
}

#[test]
fn shift_and_a_click_extend_the_selection_of_the_focused_field() {
    let value = "one two three";
    let mut tree = input(value, 300.0);
    let frame = tree.layout(WINDOW);
    press(&mut tree, point_at(&frame, value, 4), false, 1);
    tree.handle_event(UiEvent::PointerDownModified { position: point_at(&frame, value, 9), shift: true });
    assert_eq!(tree.input_selection("field"), Some((4, 9)));
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_its_line() {
    let value = "first line\nsecond line here";
    let mut tree = area(value, 300.0);
    focus(&mut tree, "notes");
    let frame = tree.layout(WINDOW);
    let origin = text_origin(&frame);
    let layout = TextLayout::new(value, style()).wrapped(300.0);
    let in_second = |at: usize| Point::new(origin.x + layout.caret(at).x + 1.0, origin.y + 18.0 + 6.0);
    press(&mut tree, in_second(13), false, 2);
    assert_eq!(tree.input_selection("notes"), Some((11, 17)), "\"second\"");
    // Just after a word, the word before.
    press(&mut tree, Point::new(origin.x + layout.caret(5).x, origin.y + 6.0), false, 2);
    assert_eq!(tree.input_selection("notes"), Some((0, 5)), "\"first\"");
    press(&mut tree, in_second(13), false, 3);
    assert_eq!(tree.input_selection("notes"), Some((11, 27)), "the line, without its break");
}

#[test]
fn ctrl_and_an_arrow_step_by_word_and_ctrl_backspace_deletes_one() {
    let mut tree = input("one two, three", 300.0);
    focus(&mut tree, "field");
    ctrl(&mut tree, Key::Home, false);
    ctrl(&mut tree, Key::ArrowRight, false);
    assert_eq!(tree.input_cursor("field"), Some(3));
    ctrl(&mut tree, Key::ArrowRight, false);
    assert_eq!(tree.input_cursor("field"), Some(7), "the end of \"two\", before its comma");
    ctrl(&mut tree, Key::ArrowRight, false);
    ctrl(&mut tree, Key::ArrowLeft, false);
    assert_eq!(tree.input_cursor("field"), Some(9), "the start of \"three\"");
    ctrl(&mut tree, Key::ArrowLeft, true);
    assert_eq!(tree.input_selection("field"), Some((4, 9)), "Shift selects the step");
    ctrl(&mut tree, Key::End, false);
    let actions = tree.handle_event(UiEvent::Key { key: Key::Backspace, shift: false, ctrl: true });
    assert_eq!(actions, [UiAction::InputChanged { id: "field".into(), value: "one two, ".into() }]);
}

#[test]
fn the_caret_steps_over_a_grapheme_whole_and_an_arrow_collapses_a_selection() {
    let mut tree = input("a👍🏽b", 300.0);
    focus(&mut tree, "field");
    ctrl(&mut tree, Key::Home, false);
    key(&mut tree, Key::ArrowRight);
    key(&mut tree, Key::ArrowRight);
    assert_eq!(tree.input_cursor("field"), Some(3), "past the thumb and its skin tone");
    let actions = tree.handle_event(UiEvent::Key { key: Key::Backspace, shift: false, ctrl: false });
    assert_eq!(actions, [UiAction::InputChanged { id: "field".into(), value: "ab".into() }]);
    ctrl(&mut tree, Key::A, false);
    key(&mut tree, Key::ArrowLeft);
    assert_eq!(tree.input_selection("field"), Some((0, 0)), "to the selection's start");
    ctrl(&mut tree, Key::A, false);
    key(&mut tree, Key::ArrowRight);
    assert_eq!(tree.input_selection("field"), Some((2, 2)), "to its end");
}

#[test]
fn a_password_has_no_words_to_step_by_or_select() {
    let mut tree = tree_of(Element::text_input("secret", "two words", "").password(true).font_family(FontFamily::Sans));
    focus(&mut tree, "secret");
    ctrl(&mut tree, Key::ArrowLeft, false);
    assert_eq!(tree.input_cursor("secret"), Some(0));
    ctrl(&mut tree, Key::ArrowRight, false);
    assert_eq!(tree.input_cursor("secret"), Some(9));
    let frame = tree.layout(WINDOW);
    press(&mut tree, Point::new(text_origin(&frame).x + 2.0, text_origin(&frame).y + 6.0), false, 2);
    assert_eq!(tree.input_selection("secret"), Some((0, 9)));
}

#[test]
fn dragging_below_a_text_area_scrolls_it_to_the_caret() {
    let value: String = (1..=30).map(|line| format!("line {line}\n")).collect();
    let mut tree = area(&value, 200.0);
    focus(&mut tree, "notes");
    ctrl(&mut tree, Key::Home, false);
    let frame = tree.layout(WINDOW);
    let field = frame.regions.iter().find(|region| region.id == "notes").expect("the area").rect;
    assert!(drawn(&frame).iter().any(|line| line.contains("line 1\n") || line == "line 1"));
    let start = Point::new(field.x + 0.5, field.y + 6.0);
    tree.handle_event(UiEvent::PointerDown(start));
    for step in 1..=3 {
        tree.handle_event(UiEvent::PointerMove(Point::new(start.x, field.y + field.height + 10.0 * step as f32)));
        tree.layout(WINDOW);
    }
    let frame = tree.layout(WINDOW);
    let (start, end) = tree.input_selection("notes").expect("a selection");
    assert_eq!(start, 0);
    assert!(end > 5 * 7, "past the lines in view: {end}");
    let caret_line = value[..end].matches('\n').count() + 1;
    assert!(drawn(&frame).iter().any(|line| line.contains(&format!("line {caret_line}"))), "scrolled to the caret's line {caret_line}");
}
