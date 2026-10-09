use super::*;
use crate::{Size, UiTree};
use std::time::Duration;

fn key(key: Key, ctrl: bool, shift: bool) -> UiEvent {
    UiEvent::Key { key, ctrl, shift }
}
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 8.0
}
fn text(controller: &EditorController) -> String {
    let snapshot = controller.snapshot().unwrap();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}

#[test]
fn individually_typed_search_replacement_saves_and_undoes_as_one_gesture() {
    let now = Instant::now();
    let controller = EditorController::new(TextBuffer::new("café").unwrap());
    let mut tree = UiTree::new(CodeEditor::new("code", controller.clone()).render());
    tree.layout_at(Size::new(400.0, 250.0), now);
    tree.focus("code");
    let found = controller
        .search("café", &SearchOptions::default())
        .unwrap();
    let range = found.matches[0].clone();
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: range.start,
            head: range.end,
        }))
        .unwrap();
    tree.handle_event_at(key(Key::Escape, false, false), now);
    for (index, character) in "mundo".chars().enumerate() {
        tree.handle_event_at(
            UiEvent::Text(character.to_string()),
            now + Duration::from_millis(index as u64 * 40),
        );
    }
    assert_eq!(text(&controller), "mundo");
    assert!(tree
        .handle_event_at(key(Key::S, true, false), now + Duration::from_millis(220))
        .iter()
        .any(|action| matches!(action, UiAction::EditorSaveRequested { id, .. } if id == "code")));
    let saved = controller.snapshot().unwrap();
    controller
        .shared_buffer()
        .lock()
        .unwrap()
        .mark_saved_snapshot(&saved)
        .unwrap();
    tree.handle_event_at(key(Key::Z, true, false), now + Duration::from_millis(260));
    assert_eq!(text(&controller), "café");
    assert_eq!(controller.snapshot().unwrap().selection().range(), 0..4);
    tree.handle_event_at(key(Key::Y, true, false), now + Duration::from_millis(300));
    assert_eq!(text(&controller), "mundo");
    assert!(!controller.snapshot().unwrap().is_dirty());
}

#[test]
fn ime_deletes_whole_graphemes_and_empty_preedit_allows_commit_with_visible_anchor() {
    let now = Instant::now();
    let controller = EditorController::new(TextBuffer::new("a e\u{301}東京 z").unwrap());
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(4)))
        .unwrap();
    let mut view = EditorView::new(CodeEditor::new("code", controller.clone()), now);
    paint(&mut view, 360.0);
    let (body, before) = view
        .ime_rects()
        .expect("focused editable cursor anchors IME");
    assert!(body.contains(Point::new(before.x, before.y)));
    assert_eq!(before.height, LINE_HEIGHT);
    view.composition(Some("候補"), None, now);
    paint(&mut view, 360.0);
    assert!(view.ime_rects().unwrap().1.x > before.x);
    let changed = view.delete_surrounding(1, 1, now);
    assert!(changed
        .iter()
        .any(|action| matches!(action, UiAction::EditorChanged { .. })));
    assert_eq!(
        text(&controller),
        "a 京 z",
        "one scalar before expands to the whole e + accent grapheme"
    );
    view.composition(Some(""), None, now);
    view.composition(None, Some("語"), now);
    assert_eq!(text(&controller), "a 語京 z");
    controller.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&controller), "a 京 z");
    controller.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&controller), "a e\u{301}東京 z");
    view.editor.read_only = true;
    paint(&mut view, 360.0);
    assert!(view.ime_rects().is_none());
    assert!(view.delete_surrounding(100, 100, now).is_empty());
    assert_eq!(text(&controller), "a e\u{301}東京 z");
}

#[test]
fn diagnostic_markers_use_wrapped_visible_ranges_and_reject_stale_snapshots() {
    let controller = EditorController::new(TextBuffer::new("0123456789abcdefghij\nother").unwrap());
    let snapshot = controller.snapshot().unwrap();
    let diagnostic_color = Color::rgb(248, 70, 99);
    let diagnostics: Arc<[CodeDiagnostic]> = Arc::from([CodeDiagnostic {
        range: 2..18,
        color: diagnostic_color,
        message: "example diagnostic".into(),
    }]);
    let mut view = EditorView::new(
        CodeEditor::new("code", controller.clone())
            .soft_wrap(true)
            .diagnostics(snapshot.document_id(), snapshot.revision(), diagnostics),
        Instant::now(),
    );
    let commands = paint(&mut view, 120.0);
    let underlines: Vec<_> = commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if *color == diagnostic_color && rect.height == 1.5 =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert!(
        underlines.len() >= 2,
        "a diagnostic crossing wrapped rows must underline each visible segment"
    );
    assert!(underlines.windows(2).all(|pair| pair[1].y > pair[0].y));
    assert!(commands.iter().any(|command| matches!(command, DrawCommand::FillRect { rect, color, .. } if *color == diagnostic_color && rect.width == 5.0 && rect.height == 5.0)));
    controller
        .execute(EditorCommand::InsertText("new".into()))
        .unwrap();
    assert!(!paint(&mut view, 120.0).iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == diagnostic_color)));
    view.editor.diagnostic_revision = Some(controller.snapshot().unwrap().revision());
    view.editor.diagnostic_document_id = Some(snapshot.document_id().wrapping_add(1));
    assert!(!paint(&mut view, 120.0).iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == diagnostic_color)));
}

#[test]
fn multiple_cursors_and_snippet_tabstops_edit_through_the_real_tree() {
    use crate::{Size, UiTree};
    let controller = EditorController::new(TextBuffer::new("foo\r\nfoo").unwrap());
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 0,
            head: 3,
        }))
        .unwrap();
    let mut tree = UiTree::new(CodeEditor::new("code", controller.clone()).render());
    tree.layout(Size::new(400.0, 250.0));
    assert!(tree.focus("code"));
    tree.handle_event(key(Key::D, true, false));
    assert_eq!(controller.snapshot().unwrap().selections().count(), 2);
    let actions = tree.handle_event(UiEvent::Text("東京".into()));
    assert!(actions
        .iter()
        .any(|a| matches!(a, UiAction::EditorChanged { .. })));
    assert_eq!(text(&controller), "東京\r\n東京");
    tree.handle_event(key(Key::Z, true, false));
    assert_eq!(text(&controller), "foo\r\nfoo");
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(0)))
        .unwrap();
    controller
        .execute(EditorCommand::InsertSnippet(
            "${1:name}($1, ${2:value})$0".into(),
        ))
        .unwrap();
    tree.layout(Size::new(400.0, 250.0));
    tree.handle_event(UiEvent::Text("x".into()));
    tree.handle_event(key(Key::Tab, false, false));
    tree.handle_event(UiEvent::Text("42".into()));
    tree.handle_event(key(Key::Tab, false, false));
    assert_eq!(text(&controller), "x(x, 42)foo\r\nfoo");
    assert!(!controller.shared_buffer().lock().unwrap().has_snippet());
}

#[test]
fn wrapped_and_folded_rows_use_the_same_coordinates_for_paint_clicks_and_arrows() {
    let controller =
        EditorController::new(TextBuffer::new("abcdefghijklmno\nhidden\nlast").unwrap());
    let mut view = EditorView::new(
        CodeEditor::new("code", controller.clone()).soft_wrap(true),
        Instant::now(),
    );
    paint(&mut view, 100.0);
    let map = &view.visual_map.as_ref().unwrap().4;
    assert!(map.rows[1].continuation);
    let expected = map.rows[1].range.start;
    let point = Point::new(
        view.viewport.body.x,
        view.viewport.body.y + LINE_HEIGHT + 5.0,
    );
    view.point(point, false, false, &measure);
    assert_eq!(controller.snapshot().unwrap().selection().head, expected);
    view.input(&key(Key::ArrowDown, false, false), Instant::now(), &measure);
    assert!(controller.snapshot().unwrap().selection().head > expected);
    controller
        .execute(EditorCommand::Goto(Position { row: 0, column: 0 }))
        .unwrap();
    view.update(CodeEditor::new("code", controller.clone()).folds(std::slice::from_ref(&(0..2))));
    let commands = paint(&mut view, 350.0);
    assert!(!contains(&commands, "hidden"));
    assert!(contains(&commands, "last"));
    let point = Point::new(
        view.viewport.body.x,
        view.viewport.body.y + LINE_HEIGHT + 5.0,
    );
    view.point(point, false, false, &measure);
    assert_eq!(
        controller
            .snapshot()
            .unwrap()
            .position_of(controller.snapshot().unwrap().selection().head)
            .unwrap()
            .row,
        2
    );
    controller
        .execute(EditorCommand::Goto(Position { row: 1, column: 0 }))
        .unwrap();
    view.viewport.reveal_caret = true;
    let commands = paint(&mut view, 350.0);
    assert!(
        contains(&commands, "hidden"),
        "goto inside a fold reveals its target"
    );
}

#[test]
fn context_click_preserves_selection_inside_and_repositions_outside_it() {
    let controller = EditorController::new(TextBuffer::new("first\nsecond").unwrap());
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 0,
            head: 5,
        }))
        .unwrap();
    let mut view = EditorView::new(CodeEditor::new("code", controller.clone()), Instant::now());
    paint(&mut view, 350.0);
    view.context_point(
        Point::new(view.viewport.body.x + 8.0, view.viewport.body.y + 5.0),
        &measure,
    );
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection { anchor: 0, head: 5 }
    );
    view.context_point(
        Point::new(
            view.viewport.body.x,
            view.viewport.body.y + LINE_HEIGHT + 5.0,
        ),
        &measure,
    );
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection::caret(6)
    );
    view.input(&UiEvent::Paste("X".into()), Instant::now(), &measure);
    assert_eq!(text(&controller), "first\nXsecond");
}

#[test]
fn pointer_release_keeps_added_cursors_and_whole_word_selection() {
    let controller = EditorController::new(TextBuffer::new("first second\nthird").unwrap());
    let mut view = EditorView::new(CodeEditor::new("code", controller.clone()), Instant::now());
    paint(&mut view, 350.0);
    let point = Point::new(
        view.viewport.body.x,
        view.viewport.body.y + LINE_HEIGHT + 5.0,
    );
    view.pointer_modified(point, false, true, false, 1, true, &measure);
    view.point(point, true, false, &measure);
    assert_eq!(controller.snapshot().unwrap().selections().count(), 2);
    let word = Point::new(view.viewport.body.x + 16.0, view.viewport.body.y + 5.0);
    view.pointer_modified(word, false, false, false, 2, true, &measure);
    view.point(word, true, false, &measure);
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection { anchor: 0, head: 5 }
    );
}

#[test]
fn ime_preedit_is_visible_without_mutation_and_commit_is_one_undoable_edit() {
    let controller = EditorController::new(TextBuffer::new("hello").unwrap());
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 0,
            head: 5,
        }))
        .unwrap();
    let mut view = EditorView::new(CodeEditor::new("code", controller.clone()), Instant::now());
    paint(&mut view, 350.0);
    view.composition(Some("とう"), None, Instant::now());
    assert_eq!(text(&controller), "hello");
    assert_eq!(controller.snapshot().unwrap().revision(), 0);
    assert!(contains(&paint(&mut view, 350.0), "とう"));
    let actions = view.composition(None, Some("東京"), Instant::now());
    assert_eq!(text(&controller), "東京");
    assert_eq!(
        actions
            .iter()
            .filter(|a| matches!(a, UiAction::EditorChanged { .. }))
            .count(),
        1
    );
    controller.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&controller), "hello");
    view.composition(Some("仮"), None, Instant::now());
    controller
        .execute(EditorCommand::Goto(Position { row: 0, column: 5 }))
        .unwrap();
    paint(&mut view, 350.0); // A repaint may hide a cancelled preedit before Commit arrives.
    view.composition(Some(""), None, Instant::now()); // Dismissal cannot reauthorize the stale commit.
    assert!(view
        .composition(None, Some("stale"), Instant::now())
        .is_empty());
    assert_eq!(text(&controller), "hello");
}

#[test]
fn search_matches_are_revision_scoped_and_replacement_is_one_undo_step() {
    let controller = EditorController::new(TextBuffer::new("café CAFE café").unwrap());
    let options = SearchOptions::default();
    let found = controller.search("café", &options).unwrap();
    let mut view = EditorView::new(
        CodeEditor::new("code", controller.clone()).search_matches(found.revision, found.matches),
        Instant::now(),
    );
    let highlight = |commands: &[DrawCommand]| {
        commands.iter().filter(|c|matches!(c,DrawCommand::FillRect{color,..} if *color==Color::rgba(218,173,54,70))).count()
    };
    assert_eq!(highlight(&paint(&mut view, 350.0)), 2);
    controller
        .execute(EditorCommand::ReplaceAll {
            query: "café".into(),
            replacement: "東京".into(),
            options,
        })
        .unwrap();
    assert_eq!(text(&controller), "東京 CAFE 東京");
    assert_eq!(highlight(&paint(&mut view, 350.0)), 0);
    controller.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&controller), "café CAFE café");
}
fn paint(view: &mut EditorView, width: f32) -> Vec<DrawCommand> {
    let rect = Rect::new(0.0, 0.0, width, 220.0);
    view.paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
}

#[test]
fn editor_width_wrap_reflows_to_the_available_viewport_without_changing_source_lines() {
    let source = format!("{}\nfin\n", "a ".repeat(120));
    let controller = EditorController::new(TextBuffer::new(&source).unwrap());
    let before = controller.snapshot().unwrap();
    let mut view = EditorView::new(
        CodeEditor::new("code", controller.clone()).soft_wrap(true),
        Instant::now(),
    );
    paint(&mut view, 320.0);
    let narrow_width = view.viewport.body.width;
    let narrow_rows = view.visual_map.as_ref().unwrap().4.rows.len();
    paint(&mut view, 2080.0);
    let wide_rows = view.visual_map.as_ref().unwrap().4.rows.len();
    assert_eq!(view.viewport.body.width - narrow_width, 1760.0);
    assert!(narrow_rows > wide_rows);
    assert_eq!(
        wide_rows,
        before.line_count(),
        "wide panes are not capped at a preferred column"
    );
    assert_eq!(view.viewport.scroll_x, 0.0);
    let body = view.viewport.body;
    view.point(
        Point::new(body.x + body.width - 2.0, body.y + 2.0),
        false,
        false,
        &measure,
    );
    let after = controller.snapshot().unwrap();
    assert_eq!(
        after.position_of(after.selection().head).unwrap(),
        Position {
            row: 0,
            column: 240
        }
    );
    assert_eq!(after.revision(), before.revision());
    assert_eq!(after.line_count(), before.line_count());
    assert_eq!(text(&controller), source);
    paint(&mut view, 320.0);
    assert_eq!(view.visual_map.as_ref().unwrap().4.rows.len(), narrow_rows);
}
fn contains(commands: &[DrawCommand], expected: &str) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text.contains(expected)))
}
fn wait_syntax(view: &mut EditorView) -> BufferSnapshot {
    let snapshot = view.editor.controller.snapshot().unwrap();
    let started = Instant::now();
    loop {
        view.syntax.update(&snapshot, view.editor.language);
        if !view.syntax.pending() {
            return snapshot;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "syntax worker timeout"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn native_word_keys_smart_home_tab_and_undo_use_engine_commands() {
    let controller = EditorController::new(TextBuffer::new("  café_東京 tail").unwrap());
    let mut view = EditorView::new(
        CodeEditor::new("editor", controller.clone()),
        Instant::now(),
    );
    let now = Instant::now();
    view.input(&key(Key::End, true, false), now, &measure);
    view.input(&key(Key::ArrowLeft, true, true), now, &measure);
    let snapshot = controller.snapshot().unwrap();
    assert_eq!(
        snapshot.text_range(snapshot.selection().range()).unwrap(),
        "tail"
    );
    view.input(&key(Key::Backspace, true, false), now, &measure);
    assert_eq!(text(&controller), "  café_東京 ");
    view.input(&key(Key::Z, true, false), now, &measure);
    assert_eq!(text(&controller), "  café_東京 tail");
    view.input(&key(Key::Home, false, false), now, &measure);
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection::caret(2)
    );
    view.input(&key(Key::Home, false, false), now, &measure);
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection::caret(0)
    );
    view.input(&key(Key::ArrowRight, false, false), now, &measure);
    view.input(&key(Key::Tab, false, false), now, &measure);
    assert!(text(&controller).starts_with("     café"));
    view.input(&key(Key::Home, true, false), now, &measure);
    view.input(&key(Key::Delete, true, false), now, &measure);
    assert_eq!(text(&controller), " tail");
}

#[test]
fn readonly_editor_navigates_by_word_but_never_indents_or_deletes() {
    let controller = EditorController::new(TextBuffer::new("alpha beta").unwrap());
    let mut view = EditorView::new(
        CodeEditor::new("editor", controller.clone()).read_only(true),
        Instant::now(),
    );
    let now = Instant::now();
    view.input(&key(Key::ArrowRight, true, false), now, &measure);
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        Selection::caret(5)
    );
    for event in [
        key(Key::Backspace, true, false),
        key(Key::Delete, true, false),
        key(Key::Tab, false, false),
    ] {
        assert!(view.input(&event, now, &measure).is_empty());
    }
    assert_eq!(text(&controller), "alpha beta");
    assert_eq!(controller.snapshot().unwrap().revision(), 0);
}

#[test]
fn status_is_live_for_keyboard_and_pointer_without_rebuilding_the_host_root() {
    let controller = EditorController::new(TextBuffer::new("fn main() {}\r\nnext").unwrap());
    let mut tree = UiTree::new(
        CodeEditor::new("editor", controller.clone())
            .language(CodeLanguage::Rust)
            .status_bar(true)
            .height(220.0)
            .render(),
    );
    let size = Size::new(760.0, 220.0);
    tree.layout(size);
    tree.focus("editor");
    assert!(contains(&tree.layout(size).commands, "Ln 1, Col 1"));
    assert!(tree.handle_event(key(Key::End, true, false)).is_empty());
    let frame = tree.layout(size);
    assert!(contains(&frame.commands, "Ln 2, Col 5"));
    assert!(contains(&frame.commands, "Rust · UTF-8 · CRLF"));
    tree.handle_event(key(Key::ArrowLeft, true, true));
    assert!(contains(
        &tree.layout(size).commands,
        "Ln 2, Col 1 · 4 sel."
    ));
    tree.handle_event(UiEvent::PointerDown(Point::new(44.0, 7.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(44.0, 7.0)));
    assert!(contains(&tree.layout(size).commands, "Ln 1, Col"));
    assert_eq!(controller.snapshot().unwrap().revision(), 0);
}

#[test]
fn narrow_status_keeps_cursor_metadata_inside_its_own_viewport() {
    let controller = EditorController::new(TextBuffer::new("sample").unwrap());
    for width in [220.0, 360.0, 900.0] {
        let mut view = EditorView::new(
            CodeEditor::new("editor", controller.clone())
                .language(CodeLanguage::TypeScript)
                .status_bar(true)
                .read_only(true),
            Instant::now(),
        );
        let commands = paint(&mut view, width);
        assert!(contains(&commands, "Ln 1, Col 1"));
        for command in &commands {
            if let DrawCommand::Text { position, text, .. } = command {
                if position.y > 190.0 {
                    assert!(position.x >= 8.0);
                    assert!(position.x + measure(text) <= width - 7.9);
                }
            }
        }
        let before = controller.snapshot().unwrap().selection();
        view.point(Point::new(80.0, 210.0), false, true, &measure);
        view.point(Point::new(80.0, 210.0), true, false, &measure);
        assert_eq!(controller.snapshot().unwrap().selection(), before);
    }
}

#[test]
fn gutter_numbers_are_right_aligned_and_active_row_is_distinct() {
    let controller = EditorController::new(TextBuffer::new(&"line\n".repeat(25)).unwrap());
    let mut view = EditorView::new(CodeEditor::new("editor", controller), Instant::now());
    let commands = paint(&mut view, 360.0);
    let number = |value: &str| {
        commands
            .iter()
            .find_map(|command| {
                if let DrawCommand::Text {
                    position,
                    text,
                    color,
                    ..
                } = command
                {
                    (text == value).then_some((position.x + measure(text), *color))
                } else {
                    None
                }
            })
            .unwrap()
    };
    assert_eq!(number("9").0, number("10").0);
    assert_eq!(number("1").1, view.editor.palette.text);
    assert_eq!(number("2").1, view.editor.palette.muted);
}

#[test]
fn replacing_a_horizontally_scrolled_long_line_resets_extent_and_undo_restores_it() {
    let source = "x".repeat(2048);
    let controller = EditorController::new(TextBuffer::new(&source).unwrap());
    let mut view = EditorView::new(
        CodeEditor::new("editor", controller.clone()),
        Instant::now(),
    );
    paint(&mut view, 360.0);
    let now = Instant::now();
    view.input(&key(Key::End, true, false), now, &measure);
    paint(&mut view, 360.0);
    assert!(view.viewport.scroll_x > 1000.0);
    view.input(&key(Key::A, true, false), now, &measure);
    view.input(&UiEvent::Text("short".into()), now, &measure);
    let commands = paint(&mut view, 360.0);
    assert_eq!(view.viewport.scroll_x, 0.0);
    assert_eq!(view.viewport.max_x, 0.0);
    assert!(contains(&commands, "short"));
    view.input(&key(Key::Z, true, false), now, &measure);
    paint(&mut view, 360.0);
    assert_eq!(text(&controller), source);
    assert!(view.viewport.scroll_x > 1000.0);
    assert!(view.viewport.max_x > 1000.0);
}

#[test]
fn joining_lines_invalidates_syntax_on_the_actual_preceding_row() {
    for ctrl in [false, true] {
        let controller = EditorController::new(
            TextBuffer::new("// prefix\nlet value = 7;\nfn tail() {}").unwrap(),
        );
        let mut view = EditorView::new(
            CodeEditor::new("editor", controller.clone()).language(CodeLanguage::Rust),
            Instant::now(),
        );
        let before = wait_syntax(&mut view);
        assert_eq!(
            view.syntax.line(&before, CodeLanguage::Rust, 0)[0].range,
            0..9
        );
        let line = before.line_range(1).unwrap();
        controller
            .shared_buffer()
            .lock()
            .unwrap()
            .set_selection(Selection::caret(line.start))
            .unwrap();
        view.input(&key(Key::Backspace, ctrl, false), Instant::now(), &measure);
        let after = wait_syntax(&mut view);
        assert_eq!(text(&controller), "// prefixlet value = 7;\nfn tail() {}");
        let comment = view.syntax.line(&after, CodeLanguage::Rust, 0);
        assert_eq!(comment.len(), 1);
        assert_eq!(comment[0].range, after.line_range(0).unwrap());
        assert_eq!(comment[0].color, Color::rgb(126, 151, 119));
    }
}
