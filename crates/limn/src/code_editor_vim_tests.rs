use super::*;
use crate::{Size, UiTree};

struct Modal {
    view: EditorView,
    controller: EditorController,
    now: Instant,
}
impl Modal {
    fn new(text: &str) -> Self {
        let controller = EditorController::new(TextBuffer::new(text).unwrap());
        let now = Instant::now();
        let view = EditorView::new(
            CodeEditor::new("vim", controller.clone()).vim_mode(true),
            now,
        );
        Self {
            view,
            controller,
            now,
        }
    }
    fn event(&mut self, event: UiEvent) {
        let actions = self.view.input(&event, self.now, &measure);
        assert!(
            !actions
                .iter()
                .any(|a| matches!(a, UiAction::EditorFailed { .. })),
            "{actions:?}"
        );
        self.now += std::time::Duration::from_millis(20);
    }
    fn command(&mut self, text: &str) {
        for c in text.chars() {
            self.event(UiEvent::Text(c.to_string()));
        }
    }
    fn key(&mut self, key: Key, ctrl: bool) {
        self.event(UiEvent::Key {
            key,
            ctrl,
            shift: false,
        });
    }
    fn escape(&mut self) {
        self.key(Key::Escape, false);
    }
    fn text(&self) -> String {
        let snapshot = self.controller.snapshot().unwrap();
        snapshot.text_range(0..snapshot.char_count()).unwrap()
    }
    fn cursor(&self) -> usize {
        self.controller.snapshot().unwrap().selection().head
    }
    fn set_cursor(&mut self, at: usize) {
        self.controller
            .execute(EditorCommand::SetSelection(Selection::caret(at)))
            .unwrap();
    }
    fn paint(&mut self) -> Vec<DrawCommand> {
        let rect = Rect::new(0.0, 0.0, 400.0, 220.0);
        self.view
            .paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
    }
}
fn measure(text: &str) -> f32 {
    text.width() as f32 * 8.0
}

#[test]
fn native_key_plus_text_enters_insert_once_and_escape_returns_to_a_grapheme() {
    let mut modal = Modal::new("abc");
    modal.key(Key::I, false);
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
    modal.command("i");
    modal.event(UiEvent::Text("e\u{301}🧑‍💻".into()));
    assert_eq!(modal.text(), "e\u{301}🧑‍💻abc");
    modal.escape();
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
    assert_eq!(modal.cursor(), 2);
    modal.key(Key::H, false);
    modal.command("h");
    assert_eq!(modal.cursor(), 0, "Key and Text must not move twice");
    modal.command("x");
    assert_eq!(modal.text(), "🧑‍💻abc");
    modal.command("u");
    assert_eq!(modal.text(), "e\u{301}🧑‍💻abc");
    modal.key(Key::R, true);
    assert_eq!(modal.text(), "🧑‍💻abc");
}

#[test]
fn operators_counts_and_linewise_registers_preserve_crlf_and_last_line() {
    let mut modal = Modal::new("one two three\r\nfour\r\nfive");
    modal.command("2dw");
    assert_eq!(modal.text(), "three\r\nfour\r\nfive");
    modal.command("u");
    modal.command("2dd");
    assert_eq!(modal.text(), "five");
    modal.command("P");
    assert_eq!(modal.text(), "one two three\r\nfour\r\nfive");
    modal.command("Gdd");
    assert_eq!(modal.text(), "one two three\r\nfour");
    modal.command("p");
    assert_eq!(modal.text(), "one two three\r\nfour\r\nfive");
}

#[test]
fn change_and_open_line_undo_the_whole_insert_gesture_and_redo_it() {
    let mut modal = Modal::new("  alpha beta\nlast");
    modal.command("^cw");
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Insert));
    modal.command("new");
    modal.escape();
    assert_eq!(modal.text(), "  new beta\nlast");
    modal.command("u");
    assert_eq!(modal.text(), "  alpha beta\nlast");
    modal.key(Key::R, true);
    assert_eq!(modal.text(), "  new beta\nlast");
    modal.command("o");
    modal.command("child");
    modal.escape();
    assert_eq!(modal.text(), "  new beta\n  child\nlast");
    modal.command("u");
    assert_eq!(modal.text(), "  new beta\nlast");
}

#[test]
fn external_undo_and_replacement_do_not_reuse_an_old_modal_undo_group() {
    let mut modal = Modal::new("alpha beta");
    modal.command("cwnew");
    modal.escape();
    assert_eq!(modal.text(), "new beta");
    {
        let shared = modal.controller.shared_buffer();
        let mut buffer = shared.lock().unwrap();
        // Partial undo removes the insertion, leaving cw's deletion. Another
        // producer then reuses that same stack length with unrelated content.
        buffer.undo().unwrap();
        assert_eq!(
            buffer
                .snapshot()
                .text_range(0..buffer.snapshot().char_count())
                .unwrap(),
            " beta"
        );
        buffer.insert("external", EditKind::Replace, 1000).unwrap();
    }
    modal.command("u");
    assert_eq!(modal.text(), " beta", "only the external edit is undone");
    modal.command("u");
    assert_eq!(modal.text(), "alpha beta");
}

#[test]
fn history_eviction_never_makes_x_undo_part_of_an_older_change_gesture() {
    let mut modal = Modal::new("alpha beta");
    let buffer = TextBuffer::with_limits(
        "alpha beta",
        limn_editor::EditorLimits {
            max_history_transactions: 2,
            ..Default::default()
        },
    )
    .unwrap();
    modal.controller = EditorController::new(buffer);
    modal.view = EditorView::new(
        CodeEditor::new("vim", modal.controller.clone()).vim_mode(true),
        modal.now,
    );
    modal.command("cwnew");
    modal.escape();
    assert_eq!(modal.text(), "new beta");
    modal.command("x");
    assert_eq!(modal.text(), "ne beta");
    modal.command("u");
    assert_eq!(
        modal.text(),
        "new beta",
        "x is a fresh transaction although history length remained two"
    );
}

#[test]
fn batched_undo_text_keeps_the_complete_redo_gesture() {
    let mut modal = Modal::new("alpha beta");
    modal.command("cwnew");
    modal.escape();
    modal.event(UiEvent::Text("2u".into()));
    assert_eq!(modal.text(), "alpha beta");
    modal.key(Key::R, true);
    assert_eq!(modal.text(), "new beta");
}

#[test]
fn external_search_navigation_cancels_the_previous_visual_anchor() {
    let mut modal = Modal::new("first\nsecond\nthird");
    modal.command("vll");
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Visual));
    modal.set_cursor(6);
    modal.command("j");
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
    assert_eq!(modal.cursor(), 13);
    assert!(modal.controller.snapshot().unwrap().selection().is_empty());
}

#[test]
fn visual_line_paste_of_character_register_keeps_the_next_line_separate() {
    for newline in ["\n", "\r\n"] {
        let mut modal = Modal::new(&format!("a{newline}bb{newline}cc"));
        modal.command("vyjVp");
        assert_eq!(modal.text(), format!("a{newline}a{newline}cc"));
        modal.command("u");
        assert_eq!(modal.text(), format!("a{newline}bb{newline}cc"));
    }
}

#[test]
fn changing_and_deleting_the_empty_final_line_never_inverts_a_range() {
    for newline in ["\n", "\r\n"] {
        let original = format!("first{newline}");
        let mut modal = Modal::new(&original);
        modal.command("Gcc");
        modal.command("last");
        modal.escape();
        assert_eq!(modal.text(), format!("first{newline}last"));
        modal.command("u");
        assert_eq!(modal.text(), original);
        modal.command("Gdd");
        assert_eq!(modal.text(), "first");
        modal.command("u");
        assert_eq!(modal.text(), original);
    }
    let mut modal = Modal::new("");
    modal.command("cc");
    modal.command("new");
    modal.escape();
    assert_eq!(modal.text(), "new");
    modal.command("u");
    assert_eq!(modal.text(), "");
}

#[test]
fn visual_character_and_line_selection_are_inclusive_and_escape_cancels() {
    let mut modal = Modal::new("a🧑‍💻e\u{301}\nnext\nlast");
    modal.command("vl");
    assert_eq!(
        modal.controller.snapshot().unwrap().selection().range(),
        0..4
    );
    modal.command("x");
    assert_eq!(modal.text(), "e\u{301}\nnext\nlast");
    modal.command("u");
    modal.command("ggVj");
    modal.command("y");
    assert_eq!(modal.text(), "a🧑‍💻e\u{301}\nnext\nlast");
    modal.command("Gp");
    assert_eq!(modal.text(), "a🧑‍💻e\u{301}\nnext\nlast\na🧑‍💻e\u{301}\nnext");
    modal.command("v");
    modal.escape();
    assert!(modal.controller.snapshot().unwrap().selection().is_empty());
}

#[test]
fn motions_respect_words_graphemes_and_desired_visual_column() {
    let mut modal = Modal::new("alpha beta\nq\n0123456789\n");
    modal.command("w");
    assert_eq!(modal.cursor(), 6);
    modal.command("e");
    assert_eq!(modal.cursor(), 9);
    modal.command("b");
    assert_eq!(modal.cursor(), 6);
    modal.command("j");
    assert_eq!(modal.cursor(), 11);
    modal.command("j");
    assert_eq!(modal.cursor(), 19);
    modal.command("gg$");
    assert_eq!(modal.cursor(), 9);
    modal.command("0");
    assert_eq!(modal.cursor(), 0);
    modal.command("3G");
    assert_eq!(modal.cursor(), 13);
    modal.command("9999999999999999999999999999999h");
    assert_eq!(modal.cursor(), 13);
}

#[test]
fn read_only_allows_navigation_yank_and_visual_but_never_mutations_or_ime() {
    let mut modal = Modal::new("first\nsecond");
    modal.view.editor.read_only = true;
    for command in ["i", "a", "o", "O", "dd", "cc", "x", "p", "u"] {
        modal.command(command);
    }
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
    modal.command("j0vy");
    assert_eq!(modal.cursor(), 6);
    modal.view.composition(Some("候補"), None, modal.now);
    modal.view.composition(None, Some("語"), modal.now);
    modal.view.delete_surrounding(1, 1, modal.now);
    assert_eq!(modal.text(), "first\nsecond");
    assert_eq!(
        modal
            .controller
            .shared_buffer()
            .lock()
            .unwrap()
            .undo_transaction_count(),
        0
    );
}

#[test]
fn modal_state_survives_same_document_frames_and_resets_for_new_documents() {
    let mut modal = Modal::new("abc");
    modal.command("i");
    modal
        .view
        .update(CodeEditor::new("vim", modal.controller.clone()).vim_mode(true));
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Insert));
    modal
        .view
        .update(CodeEditor::new("vim", modal.controller.clone()));
    assert_eq!(modal.view.vim_mode(), None);
    modal
        .view
        .update(CodeEditor::new("vim", modal.controller.clone()).vim_mode(true));
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
    modal.command("i");
    let other = EditorController::new(TextBuffer::new("other").unwrap());
    modal
        .view
        .update(CodeEditor::new("vim", other).vim_mode(true));
    assert_eq!(modal.view.vim_mode(), Some(VimMode::Normal));
}

#[test]
fn focused_modal_editor_owns_control_navigation_and_escape_before_host_shortcuts() {
    let controller = EditorController::new(TextBuffer::new("first\nsecond\nthird").unwrap());
    let mut tree = UiTree::new(
        CodeEditor::new("vim", controller.clone())
            .vim_mode(true)
            .render(),
    );
    tree.layout(Size::new(400.0, 220.0));
    tree.focus("vim");
    for key in [
        Key::B,
        Key::F,
        Key::U,
        Key::D,
        Key::J,
        Key::N,
        Key::P,
        Key::R,
    ] {
        assert!(tree.focused_code_editor_vim_consumes(key, true));
    }
    tree.handle_event(UiEvent::Key {
        key: Key::J,
        ctrl: true,
        shift: false,
    });
    assert_eq!(controller.snapshot().unwrap().selection().head, 6);
    tree.handle_event(UiEvent::Text("i".into()));
    assert_eq!(tree.focused_code_editor_vim_mode(), Some(VimMode::Insert));
    assert!(!tree.focused_code_editor_vim_consumes(Key::J, true));
    tree.handle_event(UiEvent::Key {
        key: Key::Escape,
        ctrl: false,
        shift: false,
    });
    assert_eq!(tree.code_editor_vim_mode("vim"), Some(VimMode::Normal));
}

#[test]
fn mode_badge_and_grapheme_sized_block_cursor_do_not_add_an_editor_row() {
    let mut modal = Modal::new("界abc");
    let commands = modal.paint();
    let body = modal.view.viewport.body;
    assert!(commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "NORMAL")));
    assert!(commands.iter().any(|c| matches!(c, DrawCommand::FillRect { rect, color, .. } if rect.width == 16.0 && rect.height == LINE_HEIGHT && color.a == 96)));
    modal.command("i");
    let commands = modal.paint();
    assert_eq!(modal.view.viewport.body, body);
    assert!(commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "INSERT")));
    assert!(commands.iter().any(|c| matches!(c, DrawCommand::FillRect { rect, .. } if rect.width == 1.5 && rect.height == LINE_HEIGHT)));
    assert!(modal.view.ime_rects().is_some());
    modal.escape();
    modal.paint();
    assert!(modal.view.ime_rects().is_none());
}

#[test]
fn vim_is_opt_in_and_normal_mode_ignores_paste_and_ime_commits() {
    let mut modal = Modal::new("abc");
    modal.event(UiEvent::Paste("bad".into()));
    modal.view.composition(None, Some("bad"), modal.now);
    assert_eq!(modal.text(), "abc");
    modal.view.editor.vim_mode = false;
    modal.command("x");
    assert_eq!(modal.text(), "xabc");
}

#[test]
fn a_host_that_shows_the_mode_itself_turns_the_badge_off() {
    let mut modal = Modal::new("abc");
    let editor = CodeEditor::new("vim", modal.controller.clone())
        .vim_mode(true)
        .vim_mode_label(false);
    modal.view = EditorView::new(editor, modal.now);
    let commands = modal.paint();
    assert!(!commands
        .iter()
        .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "NORMAL")));
}
