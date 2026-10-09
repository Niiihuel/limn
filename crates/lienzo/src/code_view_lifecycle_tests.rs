use crate::{
    CodeDiff, CodeDiffLine, CodeDiffLineKind, CodeEditor, Component, EditorController, Element,
    Size, UiTree,
};
use lienzo_editor::{EditKind, TextBuffer};
use std::sync::Arc;

#[test]
fn visiting_hundreds_of_diffs_keeps_a_bounded_cache() {
    let mut tree = UiTree::new(Element::spacer());
    for index in 0..256 {
        let lines: Arc<[CodeDiffLine]> = Arc::from([CodeDiffLine {
            old_line: Some(1),
            new_line: Some(1),
            text: format!("fn file_{index}() {{}}"),
            kind: CodeDiffLineKind::Context,
        }]);
        tree.set_root(CodeDiff::new(format!("git-diff:{index}"), lines).render());
        tree.layout(Size::new(420.0, 320.0));
        assert!(tree.retained_code_view_counts().1 <= 64);
    }
}

#[test]
fn evicting_views_keeps_host_owned_unsaved_buffers_intact() {
    let mut controllers = Vec::new();
    let mut tree = UiTree::new(Element::spacer());
    for index in 0..40 {
        let mut buffer = TextBuffer::new("original").unwrap();
        buffer.insert("changed ", EditKind::Typing, 1).unwrap();
        let controller = EditorController::new(buffer);
        tree.set_root(CodeEditor::new(format!("document:{index}"), controller.clone()).render());
        tree.layout(Size::new(420.0, 320.0));
        assert!(tree.retained_code_view_counts().0 <= 16);
        controllers.push(controller);
    }
    for controller in controllers {
        let snapshot = controller.snapshot().unwrap();
        assert!(snapshot.is_dirty());
        assert_eq!(
            snapshot.text_range(0..snapshot.char_count()).unwrap(),
            "changed original"
        );
    }
}
