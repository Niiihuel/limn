//! State identities distinguish editor changes even when text and selections return.
use super::*;

fn text(buffer: &TextBuffer) -> String {
    let snapshot = buffer.snapshot();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}

#[test]
fn edit_state_tokens_are_opaque_cloneable_and_distinct_between_buffers() {
    fn assert_traits<T: Clone + std::fmt::Debug + Eq>() {}
    assert_traits::<EditStateToken>();
    let a = TextBuffer::new("private document body").unwrap();
    let b = TextBuffer::new("private document body").unwrap();
    let target = a.edit_state_token();
    assert_eq!(target, target.clone());
    assert_ne!(target, b.edit_state_token());
    assert!(!format!("{target:?}").contains("private document body"));
}

#[test]
fn edit_state_tokens_preserve_noop_selections_but_reject_primary_and_secondary_aba() {
    let mut buffer = TextBuffer::new("alpha").unwrap();
    let target = buffer.edit_state_token();
    buffer.set_selection(Selection::caret(0)).unwrap();
    assert_eq!(target, buffer.edit_state_token());
    buffer.set_selections(vec![Selection::caret(0)]).unwrap();
    assert_eq!(target, buffer.edit_state_token());
    assert!(buffer.set_selection(Selection::caret(100)).is_err());
    assert_eq!(target, buffer.edit_state_token());

    buffer.set_selection(Selection::caret(2)).unwrap();
    buffer.set_selection(Selection::caret(0)).unwrap();
    assert_eq!(buffer.revision(), 0);
    assert_ne!(target, buffer.edit_state_token());

    let selections = vec![Selection::caret(0), Selection::caret(4)];
    buffer.set_selections(selections.clone()).unwrap();
    let target = buffer.edit_state_token();
    buffer.set_selections(selections.clone()).unwrap();
    assert_eq!(target, buffer.edit_state_token());
    buffer
        .set_selections(vec![Selection::caret(0), Selection::caret(3)])
        .unwrap();
    buffer.set_selections(selections).unwrap();
    assert_eq!(buffer.revision(), 0);
    assert_ne!(target, buffer.edit_state_token());
}

#[test]
fn edit_state_tokens_do_not_recover_after_edit_undo_redo_or_buffer_replacement() {
    let mut buffer = TextBuffer::new("alpha").unwrap();
    let original = buffer.edit_state_token();
    buffer.insert("!", EditKind::Paste, 0).unwrap();
    let edited = buffer.edit_state_token();
    assert_ne!(original, edited);
    assert!(buffer.undo().unwrap());
    let undone = buffer.edit_state_token();
    assert_eq!(text(&buffer), "alpha");
    assert_eq!(buffer.selection(), Selection::caret(0));
    assert_ne!(original, undone);
    assert!(buffer.redo().unwrap());
    assert_eq!(text(&buffer), "!alpha");
    assert_ne!(edited, buffer.edit_state_token());
    assert_ne!(undone, buffer.edit_state_token());

    buffer = TextBuffer::new("alpha").unwrap();
    assert_eq!(buffer.revision(), 0);
    assert_ne!(original, buffer.edit_state_token());
}
