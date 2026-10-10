use super::*;
use crate::{CodeEditor, Component, EditorController};
use limn_editor::{EditorCommand, Selection, TextBuffer};

fn controller(text: &str) -> EditorController {
    EditorController::new(TextBuffer::new(text).unwrap())
}
fn text(controller: &EditorController) -> String {
    let snapshot = controller.snapshot().unwrap();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}
fn root(a: &EditorController, b: &EditorController) -> Element {
    Element::column(vec![
        CodeEditor::new("a", a.clone()).height(160.0).render(),
        CodeEditor::new("b", b.clone()).height(160.0).render(),
    ])
}
fn layout(tree: &mut UiTree) -> Frame {
    tree.layout(Size::new(500.0, 380.0))
}

#[test]
fn native_ime_commit_keeps_its_owner_across_focus_document_and_window_changes() {
    let a = controller("alpha");
    let b = controller("beta");
    let mut tree = UiTree::new(root(&a, &b));
    layout(&mut tree);
    tree.focus("a");
    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    tree.focus("b");
    tree.handle_event(UiEvent::ImePreedit(String::new()));
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&a), "alpha");
    assert_eq!(text(&b), "beta");

    tree.handle_event(UiEvent::ImePreedit("かな".into()));
    tree.handle_event(UiEvent::ImePreedit(String::new()));
    assert!(tree
        .handle_event(UiEvent::ImeCommit("語".into()))
        .iter()
        .any(|action| matches!(action, UiAction::EditorChanged { id, .. } if id == "b")));
    assert_eq!(text(&b), "語beta");

    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    tree.set_window_focused(false);
    tree.set_window_focused(true);
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&b), "語beta");

    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    let replacement = controller("replacement");
    tree.set_root(root(&a, &replacement));
    layout(&mut tree);
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&replacement), "replacement");
}

#[test]
fn pointer_focus_and_context_menu_cancel_composition_without_redirecting_it() {
    let a = controller("alpha");
    let b = controller("beta");
    let mut tree = UiTree::new(root(&a, &b));
    let frame = layout(&mut tree);
    let b_rect = frame
        .regions
        .iter()
        .find(|region| region.id == "b")
        .unwrap()
        .rect;
    tree.focus("a");
    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    let point = Point::new(b_rect.x + 60.0, b_rect.y + 8.0);
    tree.handle_event(UiEvent::PointerDown(point));
    tree.handle_event(UiEvent::PointerUp(point));
    assert!(tree
        .handle_event(UiEvent::ImeDeleteSurrounding {
            before_chars: 2,
            after_chars: 2
        })
        .is_empty());
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&a), "alpha");
    assert_eq!(text(&b), "beta");
    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    layout(&mut tree);
    assert!(tree.context_menu_open());
    assert!(tree.editor_ime_rects().is_none());
    tree.dismiss_context_menu();
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&b), "beta");
}

#[test]
fn input_composition_cannot_commit_into_an_editor_that_gained_focus() {
    let editor = controller("untouched");
    let mut tree = UiTree::new(Element::column(vec![
        Element::text_input("query", "", "").size(None, Some(32.0)),
        CodeEditor::new("code", editor.clone())
            .height(180.0)
            .render(),
    ]));
    layout(&mut tree);
    tree.focus("query");
    tree.handle_event(UiEvent::ImePreedit("仮".into()));
    tree.focus("code");
    assert!(tree
        .handle_event(UiEvent::ImeCommit("stale".into()))
        .is_empty());
    assert_eq!(text(&editor), "untouched");
    assert_eq!(tree.input_value("query"), Some(""));
    tree.focus("query");
    tree.handle_event(UiEvent::ImePreedit("かな".into()));
    tree.handle_event(UiEvent::ImeCommit("語".into()));
    assert_eq!(tree.input_value("query"), Some("語"));
}

#[test]
fn escape_reaches_real_editor_snippets_and_multicursors_after_dismissing_a_menu() {
    let controller = controller("foo foo");
    controller
        .execute(EditorCommand::SetSelections(vec![
            Selection::caret(0),
            Selection::caret(4),
        ]))
        .unwrap();
    let mut tree = UiTree::new(CodeEditor::new("code", controller.clone()).render());
    layout(&mut tree);
    tree.focus("code");
    let escape = UiEvent::Key {
        key: Key::Escape,
        ctrl: false,
        shift: false,
    };
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    layout(&mut tree);
    tree.handle_event(escape.clone());
    assert!(!tree.context_menu_open());
    assert_eq!(controller.snapshot().unwrap().selections().count(), 2);
    tree.handle_event(escape.clone());
    assert_eq!(controller.snapshot().unwrap().selections().count(), 1);
    controller
        .execute(EditorCommand::InsertSnippet(
            "${1:name} ${2:value}$0".into(),
        ))
        .unwrap();
    assert!(controller.shared_buffer().lock().unwrap().has_snippet());
    tree.handle_event(escape);
    assert!(!controller.shared_buffer().lock().unwrap().has_snippet());
    assert_eq!(text(&controller), "name valuefoo foo");
}
