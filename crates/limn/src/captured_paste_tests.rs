//! Captured replies keep the editable state that received the original request.
use crate::*;
use limn_editor::{EditorCommand, EditorLimits, Selection, TextBuffer};
use std::sync::mpsc;
use std::time::Duration;

const VIEWPORT: Size = Size::new(500.0, 380.0);

fn fields() -> Element {
    Element::column(vec![
        Element::text_input("a", "alpha", "").size(None, Some(32.0)),
        Element::text_input("b", "beta", "").size(None, Some(32.0)),
    ])
    .gap(8.0)
}

fn field_tree() -> UiTree {
    let mut tree = UiTree::new(fields());
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    tree
}

fn editor_root(controller: &EditorController, read_only: bool) -> Element {
    CodeEditor::new("a", controller.clone())
        .read_only(read_only)
        .height(180.0)
        .render()
}

fn editor_tree(controller: &EditorController) -> UiTree {
    let mut tree = UiTree::new(editor_root(controller, false));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    tree
}

fn controller() -> EditorController {
    EditorController::new(TextBuffer::new("alpha").unwrap())
}

fn text(controller: &EditorController) -> String {
    let snapshot = controller.snapshot().unwrap();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}

fn capture(tree: &mut UiTree) -> PasteTarget {
    tree.capture_paste_target()
        .expect("a writable, laid-out target")
}

fn paste(tree: &mut UiTree, target: PasteTarget, text: &str) -> Vec<UiAction> {
    tree.handle_event(UiEvent::PasteCaptured {
        target,
        text: text.into(),
    })
}

fn key(tree: &mut UiTree, key: Key) {
    tree.handle_event(UiEvent::Key {
        key,
        ctrl: false,
        shift: false,
    });
}

#[test]
fn captured_paste_inserts_at_the_captured_field_or_editor_once() {
    for multiline in [false, true] {
        let root = if multiline {
            Element::text_area("a", "alpha", "").size(None, Some(88.0))
        } else {
            Element::text_input("a", "alpha", "").size(None, Some(32.0))
        };
        let mut tree = UiTree::new(root);
        tree.layout(VIEWPORT);
        assert!(tree.focus("a"));
        tree.set_input_cursor("a", 2);
        let target = capture(&mut tree);
        assert_eq!(target, target.clone());
        assert!(!format!("{target:?}").contains("alpha"));
        assert_eq!(
            paste(&mut tree, target.clone(), "!"),
            vec![UiAction::InputChanged {
                id: "a".into(),
                value: "al!pha".into(),
            }]
        );
        assert!(paste(&mut tree, target, "again").is_empty());
        assert_eq!(tree.input_value("a"), Some("al!pha"));

        let empty = capture(&mut tree);
        let independent = capture(&mut tree);
        assert_ne!(empty, independent);
        assert!(paste(&mut tree, empty.clone(), "").is_empty());
        assert!(paste(&mut tree, empty, "late").is_empty());
        assert_eq!(tree.input_value("a"), Some("al!pha"));
        assert!(!paste(&mut tree, independent, "!").is_empty());
        assert_eq!(tree.input_value("a"), Some("al!!pha"));

        tree.set_input_value("a", "alpha");
        tree.set_input_cursor("a", 2);
        let target = capture(&mut tree);
        let expected = if multiline {
            "alCafé\npha"
        } else {
            "alCafépha"
        };
        assert_eq!(
            paste(&mut tree, target, "Café\r\n"),
            vec![UiAction::InputChanged {
                id: "a".into(),
                value: expected.into()
            }]
        );
        assert_eq!(tree.input_value("a"), Some(expected));
        if !multiline {
            let sanitized_empty = capture(&mut tree);
            assert!(paste(&mut tree, sanitized_empty.clone(), "\r\n").is_empty());
            assert!(paste(&mut tree, sanitized_empty, "late").is_empty());
            assert_eq!(tree.input_value("a"), Some(expected));
        }
    }

    let editor = controller();
    editor
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 4,
            head: 1,
        }))
        .unwrap();
    let mut tree = editor_tree(&editor);
    let target = capture(&mut tree);
    let actions = paste(&mut tree, target.clone(), "!");
    let snapshot = editor.snapshot().unwrap();
    assert_eq!(
        actions,
        vec![UiAction::EditorChanged {
            id: "a".into(),
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
        }]
    );
    assert_eq!(text(&editor), "a!a");
    assert!(paste(&mut tree, target, "again").is_empty());
    editor.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor), "alpha");
    editor
        .execute(EditorCommand::SetSelection(Selection::caret(0)))
        .unwrap();
    let empty = capture(&mut tree);
    assert!(paste(&mut tree, empty.clone(), "").is_empty());
    assert!(paste(&mut tree, empty, "late").is_empty());
    assert_eq!(text(&editor), "alpha");
    editor
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 1,
            head: 4,
        }))
        .unwrap();
    let target = capture(&mut tree);
    assert!(
        matches!(paste(&mut tree, target, "ñ").as_slice(), [UiAction::EditorChanged { id, .. }] if id == "a")
    );
    assert_eq!(text(&editor), "aña");

    let limited = EditorController::new(
        TextBuffer::with_limits(
            "alpha",
            EditorLimits {
                max_document_bytes: 5,
                ..EditorLimits::default()
            },
        )
        .unwrap(),
    );
    limited
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 1,
            head: 4,
        }))
        .unwrap();
    let mut tree = editor_tree(&limited);
    let before = limited.snapshot().unwrap();
    let target = capture(&mut tree);
    assert!(
        matches!(paste(&mut tree, target.clone(), "123456").as_slice(), [UiAction::EditorFailed { id, error }] if id == "a" && !error.is_empty())
    );
    let after = limited.snapshot().unwrap();
    assert_eq!(text(&limited), "alpha");
    assert_eq!(after.revision(), before.revision());
    assert_eq!(
        after.selections().collect::<Vec<_>>(),
        before.selections().collect::<Vec<_>>()
    );
    assert!(paste(&mut tree, target, "X").is_empty());
    assert_eq!(text(&limited), "alpha");
    let fresh = capture(&mut tree);
    assert!(
        matches!(paste(&mut tree, fresh, "X").as_slice(), [UiAction::EditorChanged { id, .. }] if id == "a")
    );
    assert_eq!(text(&limited), "aXa");
}

#[test]
fn captured_paste_cannot_recover_after_focus_window_menu_or_modal_aba() {
    for transition in 0..7 {
        let mut tree = field_tree();
        let target = capture(&mut tree);
        match transition {
            0 => {
                assert!(tree.focus("b"));
                assert!(tree.focus("a"));
            }
            1 => {
                key(&mut tree, Key::Tab);
                assert_eq!(tree.focused_id(), Some("b"));
                key(&mut tree, Key::Tab);
            }
            2 => {
                tree.set_window_focused(false);
                assert!(tree.capture_paste_target().is_none());
                tree.set_window_focused(true);
            }
            3 => {
                tree.handle_event(UiEvent::ContextMenuKeyboard);
                assert!(tree.context_menu_open());
                assert!(tree.capture_paste_target().is_none());
                assert!(tree.dismiss_context_menu());
            }
            4 => {
                tree.set_root(
                    Modal::new(
                        fields(),
                        true,
                        "Dialog",
                        "",
                        "dismiss",
                        ComponentPalette::neutral_dark(),
                    )
                    .body(Element::text_input("modal", "", ""))
                    .render(),
                );
                tree.layout(VIEWPORT);
                assert_ne!(tree.focused_id(), Some("a"));
                tree.set_root(fields());
                tree.layout(VIEWPORT);
                assert!(tree.focus("a"));
            }
            5 => {
                let rect = tree
                    .frame()
                    .regions
                    .iter()
                    .find(|hit| hit.id == "b")
                    .unwrap()
                    .rect;
                let point = Point::new(rect.x + 4.0, rect.y + 4.0);
                tree.handle_event(UiEvent::PointerDown(point));
                tree.handle_event(UiEvent::PointerUp(point));
                assert_eq!(tree.focused_id(), Some("b"));
                assert!(tree.focus("a"));
            }
            _ => {
                assert!(tree.clear_focus());
                assert!(tree.capture_paste_target().is_none());
                assert!(tree.focus("a"));
            }
        }
        assert_eq!(tree.focused_id(), Some("a"), "transition {transition}");
        assert!(
            paste(&mut tree, target, "stale").is_empty(),
            "transition {transition}"
        );
        assert_eq!(tree.input_value("a"), Some("alpha"));
        assert_eq!(tree.input_value("b"), Some("beta"));
    }

    let editor = controller();
    let mut tree = UiTree::new(Element::column(vec![
        editor_root(&editor, false),
        Element::text_input("b", "beta", "").size(None, Some(32.0)),
    ]));
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    let target = capture(&mut tree);
    assert!(tree.focus("b"));
    assert!(tree.focus("a"));
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&editor), "alpha");

    for code_editor in [false, true] {
        let editor = controller();
        let mut tree = if code_editor {
            editor_tree(&editor)
        } else {
            field_tree()
        };
        let before = editor.snapshot().unwrap();
        let input_selection = tree.input_selection("a");
        let target = capture(&mut tree);
        assert!(tree
            .handle_event(UiEvent::ImePreedit("仮".into()))
            .is_empty());
        assert!(tree.capture_paste_target().is_none());
        assert!(tree.handle_event(UiEvent::ImeCancelled).is_empty());
        if code_editor {
            let after = editor.snapshot().unwrap();
            assert_eq!(after.revision(), before.revision());
            assert_eq!(
                after.selections().collect::<Vec<_>>(),
                before.selections().collect::<Vec<_>>()
            );
            assert_eq!(text(&editor), "alpha");
        } else {
            assert_eq!(tree.input_value("a"), Some("alpha"));
            assert_eq!(tree.input_selection("a"), input_selection);
        }
        assert!(paste(&mut tree, target, "stale").is_empty());
        let fresh = capture(&mut tree);
        let actions = paste(&mut tree, fresh, "!");
        if code_editor {
            assert!(
                matches!(actions.as_slice(), [UiAction::EditorChanged { id, .. }] if id == "a")
            );
            assert_eq!(text(&editor), "!alpha");
        } else {
            assert_eq!(
                actions,
                vec![UiAction::InputChanged {
                    id: "a".into(),
                    value: "alpha!".into()
                }]
            );
            assert_eq!(tree.input_value("a"), Some("alpha!"));
        }
    }
}

#[test]
fn captured_paste_detects_selection_aba_in_fields_and_shared_multiselections() {
    let mut tree = field_tree();
    tree.set_input_cursor("a", 2);
    let unchanged = capture(&mut tree);
    tree.set_input_cursor("a", 2);
    assert!(!paste(&mut tree, unchanged, "!").is_empty());
    assert_eq!(tree.input_value("a"), Some("al!pha"));

    let target = capture(&mut tree);
    tree.set_input_cursor("a", 0);
    tree.set_input_cursor("a", 3);
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(tree.input_value("a"), Some("al!pha"));

    let mut tree = field_tree();
    let target = capture(&mut tree);
    key(&mut tree, Key::Home);
    key(&mut tree, Key::End);
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(tree.input_value("a"), Some("alpha"));

    let editor = controller();
    let shared = editor.shared_buffer();
    let selections = vec![Selection::caret(0), Selection::caret(4)];
    shared
        .lock()
        .unwrap()
        .set_selections(selections.clone())
        .unwrap();
    let mut tree = editor_tree(&editor);
    let target = capture(&mut tree);
    let revision = editor.snapshot().unwrap().revision();
    {
        let mut buffer = shared.lock().unwrap();
        buffer
            .set_selections(vec![Selection::caret(0), Selection::caret(3)])
            .unwrap();
        buffer.set_selections(selections).unwrap();
        assert_eq!(buffer.revision(), revision);
    }
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&editor), "alpha");

    let target = capture(&mut tree);
    {
        let mut buffer = shared.lock().unwrap();
        buffer.set_selection(Selection::caret(2)).unwrap();
        buffer
            .set_selections(vec![Selection::caret(0), Selection::caret(4)])
            .unwrap();
    }
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&editor), "alpha");
}

#[test]
fn captured_paste_rejects_text_undo_and_controller_or_buffer_replacement() {
    let mut tree = field_tree();
    let target = capture(&mut tree);
    tree.set_input_value("a", "different");
    tree.set_input_value("a", "alpha");
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(tree.input_value("a"), Some("alpha"));

    let editor = controller();
    let mut tree = editor_tree(&editor);
    let target = capture(&mut tree);
    editor
        .execute(EditorCommand::InsertText("temporary".into()))
        .unwrap();
    editor.execute(EditorCommand::Undo).unwrap();
    assert_eq!(text(&editor), "alpha");
    assert!(paste(&mut tree, target, "stale").is_empty());

    let target = capture(&mut tree);
    let replacement = controller();
    tree.set_root(editor_root(&replacement, false));
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&editor), "alpha");
    assert_eq!(text(&replacement), "alpha");

    let target = capture(&mut tree);
    *replacement.shared_buffer().lock().unwrap() = TextBuffer::new("alpha").unwrap();
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&replacement), "alpha");

    let target = capture(&mut tree);
    {
        let shared = replacement.shared_buffer();
        let mut buffer = shared.lock().unwrap();
        let original = std::mem::replace(&mut *buffer, TextBuffer::new("temporary").unwrap());
        *buffer = original;
    }
    // A raw remapping restored between tree observations needs the host's explicit cut.
    tree.invalidate_paste_targets();
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&replacement), "alpha");
}

#[test]
fn captured_paste_requires_fresh_layout_but_survives_equivalent_reflow() {
    let mut kind_tree = field_tree();
    let old_kind = capture(&mut kind_tree);
    kind_tree.set_root(Element::text_area("a", "alpha", "").size(None, Some(88.0)));
    kind_tree.layout(VIEWPORT);
    assert_eq!(kind_tree.focused_id(), Some("a"));
    kind_tree.set_root(fields());
    kind_tree.layout(VIEWPORT);
    assert_eq!(kind_tree.focused_id(), Some("a"));
    assert!(paste(&mut kind_tree, old_kind, "stale").is_empty());
    assert_eq!(kind_tree.input_value("a"), Some("alpha"));
    let fresh_kind = capture(&mut kind_tree);
    assert!(!paste(&mut kind_tree, fresh_kind, "!").is_empty());
    assert_eq!(kind_tree.input_value("a"), Some("alpha!"));

    let mut tree = field_tree();
    let target = capture(&mut tree);
    tree.set_root(fields());
    assert!(tree.capture_paste_target().is_none());
    assert!(paste(&mut tree, target.clone(), "stale").is_empty());
    tree.layout(Size::new(310.0, 420.0));
    assert_eq!(
        paste(&mut tree, target, "!"),
        vec![UiAction::InputChanged {
            id: "a".into(),
            value: "alpha!".into()
        }]
    );

    let target = capture(&mut tree);
    tree.set_root(Element::text("No editor"));
    tree.layout(VIEWPORT);
    assert!(tree.capture_paste_target().is_none());
    tree.set_root(fields());
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(tree.input_value("a"), Some("alpha!"));

    let editor = controller();
    let mut tree = editor_tree(&editor);
    let target = capture(&mut tree);
    tree.set_root(editor_root(&editor, false));
    assert!(tree.capture_paste_target().is_none());
    assert!(paste(&mut tree, target.clone(), "stale").is_empty());
    tree.layout(Size::new(260.0, 420.0));
    assert!(matches!(
        paste(&mut tree, target, "!").as_slice(),
        [UiAction::EditorChanged { .. }]
    ));
    assert_eq!(text(&editor), "!alpha");
    editor.execute(EditorCommand::Undo).unwrap();
    let target = capture(&mut tree);
    tree.set_root(editor_root(&editor, true));
    tree.layout(VIEWPORT);
    assert!(tree.capture_paste_target().is_none());
    tree.set_root(editor_root(&editor, false));
    tree.layout(VIEWPORT);
    assert!(paste(&mut tree, target, "stale").is_empty());
    assert_eq!(text(&editor), "alpha");

    let target = capture(&mut tree);
    tree.set_root(
        CodeEditor::new("a", editor.clone())
            .vim_mode(true)
            .height(180.0)
            .render(),
    );
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    assert_eq!(tree.focused_code_editor_vim_mode(), Some(VimMode::Normal));
    assert!(tree.capture_paste_target().is_none());
    assert!(paste(&mut tree, target, "stale").is_empty());
    key(&mut tree, Key::Escape);
    assert_eq!(text(&editor), "alpha");

    assert!(tree.handle_event(UiEvent::Text("i".into())).is_empty());
    assert_eq!(tree.focused_code_editor_vim_mode(), Some(VimMode::Insert));
    let before = editor.snapshot().unwrap();
    assert_eq!(before.selection(), Selection::caret(0));
    let target = capture(&mut tree);
    key(&mut tree, Key::Escape);
    assert_eq!(tree.focused_code_editor_vim_mode(), Some(VimMode::Normal));
    assert!(tree.capture_paste_target().is_none());
    assert!(tree.handle_event(UiEvent::Text("i".into())).is_empty());
    assert_eq!(tree.focused_code_editor_vim_mode(), Some(VimMode::Insert));
    let after = editor.snapshot().unwrap();
    assert_eq!(after.revision(), before.revision());
    assert_eq!(
        after.selections().collect::<Vec<_>>(),
        before.selections().collect::<Vec<_>>()
    );
    assert_eq!(text(&editor), "alpha");
    assert!(paste(&mut tree, target, "stale").is_empty());
    let fresh = capture(&mut tree);
    assert!(
        matches!(paste(&mut tree, fresh, "!").as_slice(), [UiAction::EditorChanged { id, .. }] if id == "a")
    );
    assert_eq!(text(&editor), "!alpha");

    let mut disabled = UiTree::new(
        Composer::new("a", "alpha", "", 400.0, ComponentPalette::neutral_dark())
            .enabled(false)
            .render(),
    );
    disabled.layout(VIEWPORT);
    assert!(!disabled.focus("a"));
    assert!(disabled.capture_paste_target().is_none());
}

#[test]
fn captured_paste_rejects_foreign_invalidated_and_busy_buffer_targets() {
    let mut first = field_tree();
    let mut other = field_tree();
    let target = capture(&mut first);
    assert!(paste(&mut other, target.clone(), "foreign").is_empty());
    assert_eq!(other.input_value("a"), Some("alpha"));
    first.invalidate_paste_targets();
    assert!(paste(&mut first, target, "stale").is_empty());
    assert_eq!(first.input_value("a"), Some("alpha"));
    let foreign = capture(&mut other);
    assert!(paste(&mut first, foreign.clone(), "foreign").is_empty());
    assert_eq!(
        paste(&mut other, foreign, "!"),
        vec![UiAction::InputChanged {
            id: "a".into(),
            value: "alpha!".into()
        }]
    );

    let editor = controller();
    let mut tree = editor_tree(&editor);
    let target = capture(&mut tree);
    let shared = editor.shared_buffer();
    let (locked_tx, locked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let guard = shared.lock().unwrap();
        locked_tx.send(()).unwrap();
        let explicitly_released = release_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        drop(guard);
        explicitly_released
    });
    locked_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    // The escape timeout makes a blocking implementation fail assertions instead of hanging.
    let blocked_capture = tree.capture_paste_target();
    let actions = paste(&mut tree, target.clone(), "must not wait then insert");
    let _ = release_tx.send(());
    let explicitly_released = worker.join().unwrap();
    assert!(blocked_capture.is_none());
    assert!(actions.is_empty());
    assert!(
        explicitly_released,
        "the operation waited until the escape released the buffer"
    );
    assert_eq!(text(&editor), "alpha");
    assert!(
        matches!(paste(&mut tree, target, "!").as_slice(), [UiAction::EditorChanged { id, .. }] if id == "a")
    );
    assert_eq!(text(&editor), "!alpha");
}
