use super::*;
fn text(buffer: &TextBuffer) -> String {
    let s = buffer.snapshot();
    s.text_range(0..s.char_count()).unwrap()
}

#[test]
fn multi_edit_is_atomic_and_undo_restores_every_directed_selection() {
    let mut b = TextBuffer::new("cafe\u{301}\r\n東京\r\nlast").unwrap();
    let original = vec![
        Selection { anchor: 5, head: 0 },
        Selection { anchor: 7, head: 9 },
    ];
    b.set_selections(original.clone()).unwrap();
    b.insert("🧑‍💻", EditKind::Typing, 1).unwrap();
    assert_eq!(text(&b), "🧑‍💻\r\n🧑‍💻\r\nlast");
    assert_eq!(b.selections().count(), 2);
    assert_eq!(b.history_transaction_count(), 1);
    b.undo().unwrap();
    assert_eq!(text(&b), "cafe\u{301}\r\n東京\r\nlast");
    assert_eq!(b.selections().collect::<Vec<_>>(), original);
    b.redo().unwrap();
    assert!(b.selections().all(Selection::is_empty));
}
#[test]
fn rectangular_selection_respects_tabs_wide_text_and_combining_clusters() {
    let mut b = TextBuffer::new("\t東京\r\nab\tcd\r\ne\u{301}xy").unwrap();
    b.select_rectangle(
        Position { row: 0, column: 4 },
        Position { row: 2, column: 6 },
        4,
    )
    .unwrap();
    let before = b.selections().collect::<Vec<_>>();
    assert_eq!(before.len(), 3);
    for s in &before {
        assert!(b.snapshot().is_grapheme_boundary(s.anchor).unwrap());
        assert!(b.snapshot().is_grapheme_boundary(s.head).unwrap());
    }
    b.insert("Q", EditKind::Typing, 1).unwrap();
    assert_eq!(b.selections().count(), 3);
    b.undo().unwrap();
    assert_eq!(b.selections().collect::<Vec<_>>(), before);
    assert_eq!(text(&b), "\t東京\r\nab\tcd\r\ne\u{301}xy");
}
#[test]
fn snippets_link_tabstops_track_replacements_and_finish_at_zero() {
    let mut b = TextBuffer::new("").unwrap();
    b.insert_snippet("let ${1:name} = $1; ${2:value}$0", 1)
        .unwrap();
    assert_eq!(text(&b), "let name = name; value");
    assert_eq!(b.selections().count(), 2);
    b.type_text("東京", true, 2).unwrap();
    assert_eq!(text(&b), "let 東京 = 東京; value");
    b.next_snippet(false).unwrap();
    assert_eq!(
        b.snapshot().text_range(b.selection().range()).unwrap(),
        "value"
    );
    b.insert("42", EditKind::Typing, 3).unwrap();
    b.next_snippet(false).unwrap();
    assert!(!b.has_snippet());
    assert_eq!(b.selection(), Selection::caret(b.snapshot().char_count()));
    assert_eq!(text(&b), "let 東京 = 東京; 42");
    b.undo().unwrap();
    assert_eq!(text(&b), "let 東京 = 東京; value");
}
#[test]
fn autopairs_wrap_reverse_selections_skip_closing_and_delete_empty_pairs() {
    let mut b = TextBuffer::new("東京").unwrap();
    b.set_selection(Selection { anchor: 2, head: 0 }).unwrap();
    b.type_text("(", true, 0).unwrap();
    assert_eq!(text(&b), "(東京)");
    assert_eq!(b.selection(), Selection { anchor: 3, head: 1 });
    b.undo().unwrap();
    assert_eq!(b.selection(), Selection { anchor: 2, head: 0 });
    b.set_selection(Selection::caret(2)).unwrap();
    b.type_text("[", true, 1).unwrap();
    assert_eq!(text(&b), "東京[]");
    b.delete_pair_backward(2).unwrap();
    assert_eq!(text(&b), "東京");
    b.type_text("(", true, 3).unwrap();
    let revision = b.revision();
    b.type_text(")", true, 4).unwrap();
    assert_eq!(b.revision(), revision);
    assert_eq!(b.selection().head, 4);
}
#[test]
fn hard_tabs_comments_line_moves_and_eol_are_undoable() {
    let mut b = TextBuffer::new("  α\r\n  β\r\nlast").unwrap();
    b.set_selection(Selection {
        anchor: 10,
        head: 0,
    })
    .unwrap();
    b.execute_command(
        EditorCommand::ToggleLineComment {
            prefix: "//".into(),
        },
        0,
    )
    .unwrap();
    assert_eq!(text(&b), "  // α\r\n  // β\r\nlast");
    b.execute_command(
        EditorCommand::ToggleLineComment {
            prefix: "//".into(),
        },
        1,
    )
    .unwrap();
    assert_eq!(text(&b), "  α\r\n  β\r\nlast");
    b.execute_command(
        EditorCommand::Indent {
            reverse: false,
            style: IndentStyle::Tabs(4),
        },
        2,
    )
    .unwrap();
    assert_eq!(text(&b), "\t  α\r\n\t  β\r\nlast");
    b.undo().unwrap();
    b.set_selection(Selection::caret(2)).unwrap();
    b.execute_command(EditorCommand::MoveLines { down: true }, 3)
        .unwrap();
    assert_eq!(text(&b), "  β\r\n  α\r\nlast");
    assert_eq!(
        b.snapshot().position_of(b.selection().head).unwrap(),
        Position { row: 1, column: 2 }
    );
    b.undo().unwrap();
    b.execute_command(EditorCommand::SetLineEnding(LineEnding::Cr), 4)
        .unwrap();
    assert_eq!(text(&b), "  α\r  β\rlast");
    assert_eq!(b.preferred_line_ending(), LineEnding::Cr);
    b.undo().unwrap();
    assert_eq!(b.preferred_line_ending(), LineEnding::CrLf);
    assert_eq!(text(&b), "  α\r\n  β\r\nlast");
}
#[test]
fn failed_multi_edit_and_invalid_snippet_leave_text_history_and_selections_intact() {
    let mut b = TextBuffer::with_limits(
        "abc",
        EditorLimits {
            max_document_bytes: 4,
            ..Default::default()
        },
    )
    .unwrap();
    b.set_selections(vec![Selection::caret(0), Selection::caret(3)])
        .unwrap();
    let before = b.selections().collect::<Vec<_>>();
    assert!(b.insert("XX", EditKind::Paste, 0).is_err());
    assert_eq!(text(&b), "abc");
    assert_eq!(b.selections().collect::<Vec<_>>(), before);
    assert_eq!(b.revision(), 0);
    assert!(!b.can_undo());
    b.set_selection(Selection::caret(0)).unwrap();
    assert!(b.insert_snippet("${bad}", 0).is_err());
    assert_eq!(text(&b), "abc");
}

#[test]
fn inserting_at_multiple_empty_cursors_maps_later_offsets_past_earlier_insertions() {
    let mut buffer = TextBuffer::new("a\nb").unwrap();
    buffer
        .set_selections(vec![Selection::caret(0), Selection::caret(2)])
        .unwrap();
    buffer.insert("東京", EditKind::Typing, 0).unwrap();
    assert_eq!(text(&buffer), "東京a\n東京b");
    assert_eq!(
        buffer.selections().collect::<Vec<_>>(),
        vec![Selection::caret(2), Selection::caret(6)]
    );
    buffer.insert("!", EditKind::Typing, 1).unwrap();
    assert_eq!(text(&buffer), "東京!a\n東京!b");
}
#[test]
fn goto_and_next_occurrence_preserve_unicode_and_do_not_modify_document() {
    let mut b = TextBuffer::new("東京 foo 東京\r\nnext").unwrap();
    b.goto_line(500, 500).unwrap();
    assert_eq!(b.selection().head, b.snapshot().char_count());
    b.set_selection(Selection { anchor: 0, head: 2 }).unwrap();
    b.execute_command(EditorCommand::SelectNextOccurrence { all: false }, 0)
        .unwrap();
    assert_eq!(b.selections().count(), 2);
    assert_eq!(b.revision(), 0);
    b.type_text("x", true, 0).unwrap();
    assert_eq!(text(&b), "x foo x\r\nnext");
    b.undo().unwrap();
    assert_eq!(b.selections().count(), 2);
}
#[test]
fn missing_disk_state_preserves_document_history_and_empty_recovery_is_dirty() {
    let mut b = TextBuffer::new("a").unwrap();
    b.insert("b", EditKind::Typing, 0).unwrap();
    b.mark_saved();
    let id = b.document_id();
    let revision = b.revision();
    b.mark_unsaved();
    assert!(b.is_dirty());
    assert!(b.snapshot().is_dirty());
    assert_eq!((b.document_id(), b.revision()), (id, revision));
    assert!(b.can_undo());
    b.undo().unwrap();
    assert!(b.is_dirty());
    let saved = b.snapshot();
    b.mark_saved_snapshot(&saved).unwrap();
    assert!(!b.is_dirty());
    let empty =
        TextBuffer::restore_unsaved("", Selection::caret(0), EditorLimits::default()).unwrap();
    assert!(empty.is_dirty());
    assert_eq!(empty.revision(), 1);
    assert!(!empty.can_undo());
    assert!(TextBuffer::restore_unsaved("", Selection::caret(1), EditorLimits::default()).is_err());
}
