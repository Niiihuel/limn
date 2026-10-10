use super::*;
use unicode_segmentation::UnicodeSegmentation;

fn text(buffer: &TextBuffer) -> String {
    let snapshot = buffer.snapshot();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}

fn char_byte(text: &str, offset: usize) -> usize {
    text.char_indices()
        .nth(offset)
        .map_or(text.len(), |(byte, _)| byte)
}

#[test]
fn smoke_edit_and_snapshot() {
    let mut buffer = TextBuffer::new("hello\r\nworld").unwrap();
    let snapshot = buffer.snapshot();
    buffer.set_selection(Selection::caret(5)).unwrap();
    buffer.insert("!", EditKind::Typing, 0).unwrap();
    assert_eq!(
        snapshot.text_range(0..snapshot.char_count()).unwrap(),
        "hello\r\nworld"
    );
    assert_eq!(buffer.snapshot().line_text(0).unwrap(), "hello!");
    assert!(buffer.is_dirty());
    assert!(buffer.undo().unwrap());
    assert!(!buffer.is_dirty());
    assert!(buffer.redo().unwrap());
}

#[test]
fn selections_and_range_replacement_restore_exactly() {
    let mut buffer = TextBuffer::new("a😀b界c").unwrap();
    let before = Selection { anchor: 4, head: 1 };
    buffer.set_selection(before).unwrap();
    buffer.insert("é", EditKind::Paste, 1).unwrap();
    assert_eq!(text(&buffer), "aéc");
    assert_eq!(buffer.selection(), Selection::caret(2));
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "a😀b界c");
    assert_eq!(buffer.selection(), before);
    buffer.redo().unwrap();
    assert_eq!(buffer.selection(), Selection::caret(2));
}

#[test]
fn typing_coalesces_but_timeout_movement_paste_and_save_break_groups() {
    let mut buffer = TextBuffer::new("").unwrap();
    for (time, value) in [(0, "a"), (100, "b"), (400, "c")] {
        buffer.insert(value, EditKind::Typing, time).unwrap();
    }
    assert_eq!(buffer.history_transaction_count(), 1);
    buffer.insert("d", EditKind::Typing, 1000).unwrap();
    assert_eq!(buffer.history_transaction_count(), 2);
    buffer.set_selection(Selection::caret(0)).unwrap();
    buffer.insert("e", EditKind::Typing, 1010).unwrap();
    buffer.insert("f", EditKind::Paste, 1020).unwrap();
    buffer.insert("g", EditKind::Typing, 1030).unwrap();
    buffer.mark_saved();
    buffer.insert("h", EditKind::Typing, 1040).unwrap();
    assert_eq!(buffer.history_transaction_count(), 6);
    buffer.undo().unwrap();
    assert!(!buffer.is_dirty());
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "efabcd");
}

#[test]
fn typing_over_a_directed_selection_is_one_undo_gesture_and_save_still_breaks_groups() {
    for selection in [
        Selection { anchor: 0, head: 4 },
        Selection { anchor: 4, head: 0 },
    ] {
        let mut buffer = TextBuffer::new("café").unwrap();
        buffer.set_selection(selection).unwrap();
        for (index, character) in "mundo".chars().enumerate() {
            buffer
                .insert(&character.to_string(), EditKind::Typing, index as u64 * 40)
                .unwrap();
        }
        assert_eq!(text(&buffer), "mundo");
        assert_eq!(buffer.history_transaction_count(), 1);
        let saved = buffer.snapshot();
        buffer.mark_saved_snapshot(&saved).unwrap();
        buffer.insert("!", EditKind::Typing, 220).unwrap();
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), "mundo");
        assert!(!buffer.is_dirty());
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), "café");
        assert_eq!(buffer.selection(), selection);
        buffer.redo().unwrap();
        assert_eq!(text(&buffer), "mundo");
        assert!(!buffer.is_dirty());
    }
}

#[test]
fn paste_after_typing_over_a_selection_remains_a_separate_undo_gesture() {
    let mut buffer = TextBuffer::new("café").unwrap();
    buffer
        .set_selection(Selection { anchor: 0, head: 4 })
        .unwrap();
    buffer.insert("m", EditKind::Typing, 0).unwrap();
    buffer.insert("u", EditKind::Typing, 40).unwrap();
    buffer.insert("nd", EditKind::Paste, 60).unwrap();
    buffer.insert("o", EditKind::Typing, 80).unwrap();
    assert_eq!(text(&buffer), "mundo");
    assert_eq!(buffer.history_transaction_count(), 3);
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "mund");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "mu");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "café");
}

#[test]
fn maximum_group_length_is_bounded() {
    let limits = EditorLimits {
        max_grouped_edits: 2,
        ..EditorLimits::default()
    };
    let mut buffer = TextBuffer::with_limits("", limits).unwrap();
    for time in 0..5 {
        buffer.insert("x", EditKind::Typing, time).unwrap();
    }
    assert_eq!(buffer.history_transaction_count(), 3);
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "xxxx");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "xx");
}

#[test]
fn backspace_and_delete_group_without_splitting_graphemes() {
    let mut buffer = TextBuffer::new("a👩‍💻e\u{301}🇦🇷").unwrap();
    buffer
        .set_selection(Selection::caret(buffer.snapshot().char_count()))
        .unwrap();
    buffer.delete_backward(0).unwrap();
    assert_eq!(text(&buffer), "a👩‍💻e\u{301}");
    buffer.delete_backward(1).unwrap();
    assert_eq!(text(&buffer), "a👩‍💻");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "a👩‍💻e\u{301}🇦🇷");
    buffer.set_selection(Selection::caret(1)).unwrap();
    buffer.delete_forward(2).unwrap();
    assert_eq!(text(&buffer), "ae\u{301}🇦🇷");
    buffer.delete_forward(3).unwrap();
    assert_eq!(text(&buffer), "a🇦🇷");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "a👩‍💻e\u{301}🇦🇷");
}

#[test]
fn deletion_expands_partial_programmatic_grapheme_selection() {
    for backward in [true, false] {
        let mut buffer = TextBuffer::new("a👩‍💻b").unwrap();
        buffer.set_selection(Selection::caret(2)).unwrap();
        if backward {
            buffer.delete_backward(0).unwrap();
        } else {
            buffer.delete_forward(0).unwrap();
        }
        assert_eq!(text(&buffer), "ab");
        buffer.undo().unwrap();
        buffer
            .set_selection(Selection { anchor: 3, head: 2 })
            .unwrap();
        buffer.delete_backward(1).unwrap();
        assert_eq!(text(&buffer), "ab");
    }
}

#[test]
fn crlf_deletion_and_mixed_line_endings_are_preserved() {
    let original = "a\r\nb\nc\rd\r\n";
    let mut buffer = TextBuffer::new(original).unwrap();
    assert_eq!(buffer.preferred_line_ending(), LineEnding::CrLf);
    buffer.set_selection(Selection::caret(3)).unwrap();
    buffer.delete_backward(0).unwrap();
    assert_eq!(text(&buffer), "ab\nc\rd\r\n");
    buffer.undo().unwrap();
    let mut bytes = Vec::new();
    buffer.snapshot().write_to(&mut bytes).unwrap();
    assert_eq!(bytes, original.as_bytes());
    assert_eq!(buffer.snapshot().chunks().collect::<String>(), original);
}

#[test]
fn newline_auto_indent_uses_original_convention_without_normalizing_paste() {
    for (original, ending) in [
        ("\t  abc\r\ndef", "\r\n"),
        ("\t  abc\ndef", "\n"),
        ("\t  abc\rdef", "\r"),
    ] {
        let mut buffer = TextBuffer::new(original).unwrap();
        buffer.set_selection(Selection::caret(6)).unwrap();
        buffer.insert_newline(0).unwrap();
        assert_eq!(text(&buffer), format!("\t  abc{ending}\t  {ending}def"));
        buffer.insert("x\ny\r\nz", EditKind::Paste, 1).unwrap();
        assert!(text(&buffer).contains("x\ny\r\nz"));
        buffer.undo().unwrap();
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), original);
    }
}

#[test]
fn line_indices_and_positions_handle_unicode_empty_final_lines_and_crlf() {
    let snapshot = TextBuffer::new("😀é\r\n界\n").unwrap().snapshot();
    assert_eq!(snapshot.line_count(), 3);
    assert_eq!(snapshot.line_range(0).unwrap(), 0..2);
    assert_eq!(snapshot.line_range(1).unwrap(), 4..5);
    assert_eq!(snapshot.line_range(2).unwrap(), 6..6);
    assert_eq!(
        snapshot.position_of(3).unwrap(),
        Position { row: 0, column: 2 }
    );
    assert_eq!(
        snapshot.position_of(4).unwrap(),
        Position { row: 1, column: 0 }
    );
    assert_eq!(
        snapshot.offset_of(Position { row: 1, column: 1 }).unwrap(),
        5
    );
    assert_eq!(
        snapshot.offset_of(Position { row: 2, column: 0 }).unwrap(),
        6
    );
    for offset in 0..=snapshot.char_count() {
        let byte = snapshot.char_to_byte(offset).unwrap();
        assert_eq!(snapshot.byte_to_char(byte).unwrap(), offset);
    }
    assert_eq!(snapshot.byte_to_char(1).unwrap(), 0);
    assert!(snapshot.offset_of(Position { row: 1, column: 2 }).is_err());
    assert!(snapshot.line_text(3).is_err());
}

#[test]
fn unicode_line_separators_remain_exact() {
    let value = "a\u{85}b\u{2028}c\u{2029}d\u{b}e\u{c}f";
    let snapshot = TextBuffer::new(value).unwrap().snapshot();
    assert_eq!(snapshot.line_count(), 6);
    for (row, expected) in ["a", "b", "c", "d", "e", "f"].into_iter().enumerate() {
        assert_eq!(snapshot.line_text(row).unwrap(), expected);
    }
    assert_eq!(snapshot.chunks().collect::<String>(), value);
}

#[test]
fn grapheme_cursor_agrees_with_unicode_segmentation_at_rope_chunk_seams() {
    let value = format!(
        "{}{}{}",
        "abcd界".repeat(250),
        "👩‍💻e\u{301}🇦🇷क्‍ष\r\n".repeat(100),
        "x\u{301}".repeat(700)
    );
    let snapshot = TextBuffer::new(&value).unwrap().snapshot();
    assert!(snapshot.chunks().count() > 3);
    let boundaries: Vec<usize> = value
        .grapheme_indices(true)
        .map(|(byte, _)| value[..byte].chars().count())
        .chain(std::iter::once(value.chars().count()))
        .collect();
    for offset in 0..=snapshot.char_count() {
        assert_eq!(
            snapshot.is_grapheme_boundary(offset).unwrap(),
            boundaries.binary_search(&offset).is_ok(),
            "boundary {offset}"
        );
        let previous = boundaries
            .iter()
            .copied()
            .take_while(|boundary| *boundary < offset)
            .last()
            .unwrap_or(0);
        let next = boundaries
            .iter()
            .copied()
            .find(|boundary| *boundary > offset)
            .unwrap_or(snapshot.char_count());
        assert_eq!(
            snapshot.prev_grapheme(offset).unwrap(),
            previous,
            "previous {offset}"
        );
        assert_eq!(
            snapshot.next_grapheme(offset).unwrap(),
            next,
            "next {offset}"
        );
    }
}

#[test]
fn giant_combining_cluster_spans_chunks_without_partial_deletion() {
    let value = format!("a{}b", "\u{301}".repeat(3000));
    let mut buffer = TextBuffer::new(&value).unwrap();
    assert_eq!(buffer.snapshot().next_grapheme(0).unwrap(), 3001);
    assert_eq!(buffer.snapshot().prev_grapheme(3001).unwrap(), 0);
    buffer.set_selection(Selection::caret(3001)).unwrap();
    buffer.delete_backward(0).unwrap();
    assert_eq!(text(&buffer), "b");
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), value);
}

#[test]
fn revision_is_monotonic_while_saved_content_identity_tracks_undo_and_redo() {
    let mut buffer = TextBuffer::new("a").unwrap();
    buffer.set_selection(Selection::caret(1)).unwrap();
    buffer.insert("b", EditKind::Paste, 1).unwrap();
    buffer.mark_saved();
    let saved_revision = buffer.saved_revision();
    buffer.insert("c", EditKind::Paste, 2).unwrap();
    assert!(buffer.is_dirty());
    let before_undo = buffer.revision();
    buffer.undo().unwrap();
    assert!(buffer.revision() > before_undo);
    assert!(!buffer.is_dirty());
    assert_eq!(buffer.saved_revision(), saved_revision);
    buffer.undo().unwrap();
    assert!(buffer.is_dirty());
    buffer.redo().unwrap();
    assert!(!buffer.is_dirty());
    buffer.redo().unwrap();
    assert!(buffer.is_dirty());
}

#[test]
fn asynchronous_save_acknowledges_old_snapshot_and_rejects_foreign_documents() {
    let mut buffer = TextBuffer::new("a").unwrap();
    buffer.set_selection(Selection::caret(1)).unwrap();
    buffer.insert("b", EditKind::Paste, 0).unwrap();
    let saved = buffer.snapshot();
    buffer.insert("c", EditKind::Paste, 1).unwrap();
    buffer.mark_saved_snapshot(&saved).unwrap();
    assert!(buffer.is_dirty());
    buffer.undo().unwrap();
    assert!(!buffer.is_dirty());
    assert_eq!(text(&buffer), "ab");
    let foreign = TextBuffer::new("ab").unwrap().snapshot();
    assert!(matches!(
        buffer.mark_saved_snapshot(&foreign),
        Err(EditorError::WrongDocument { .. })
    ));
}

#[test]
fn new_edits_invalidate_redo_and_noops_preserve_revision_and_redo() {
    let mut buffer = TextBuffer::new("x").unwrap();
    buffer.set_selection(Selection::caret(1)).unwrap();
    buffer.insert("y", EditKind::Paste, 0).unwrap();
    buffer.undo().unwrap();
    let revision = buffer.revision();
    assert!(!buffer
        .edit(0..1, "x", Selection::caret(1), EditKind::Replace, 1)
        .unwrap());
    assert_eq!(buffer.revision(), revision);
    assert!(buffer.can_redo());
    buffer.insert("z", EditKind::Typing, 2).unwrap();
    assert!(!buffer.can_redo());
    assert_eq!(text(&buffer), "xz");
}

#[test]
fn empty_document_and_boundary_deletions_are_noops() {
    let mut buffer = TextBuffer::new("").unwrap();
    assert_eq!(buffer.snapshot().line_count(), 1);
    assert_eq!(buffer.snapshot().line_range(0).unwrap(), 0..0);
    assert!(!buffer.delete_backward(0).unwrap());
    assert!(!buffer.delete_forward(0).unwrap());
    assert!(!buffer.undo().unwrap());
    assert!(!buffer.redo().unwrap());
    assert_eq!(buffer.revision(), 0);
}

#[test]
fn invalid_edits_limits_and_selections_do_not_mutate_buffer() {
    let limits = EditorLimits {
        max_document_bytes: 4,
        ..EditorLimits::default()
    };
    let mut buffer = TextBuffer::with_limits("abc", limits).unwrap();
    for (range, replacement, selection) in [
        (Range { start: 2, end: 1 }, "", Selection::caret(0)),
        (0..4, "", Selection::caret(0)),
        (1..1, "😀", Selection::caret(2)),
        (1..1, "x", Selection::caret(5)),
    ] {
        assert!(buffer
            .edit(range, replacement, selection, EditKind::Replace, 0)
            .is_err());
        assert_eq!(text(&buffer), "abc");
        assert_eq!(buffer.revision(), 0);
        assert_eq!(buffer.history_transaction_count(), 0);
    }
    assert!(buffer.set_selection(Selection::caret(4)).is_err());
    assert!(TextBuffer::with_limits("abcde", limits).is_err());
    assert!(TextBuffer::with_limits(
        "",
        EditorLimits {
            max_grouped_edits: 0,
            ..limits
        }
    )
    .is_err());
}

#[test]
fn revision_exhaustion_is_reported_before_mutation() {
    let mut buffer = TextBuffer::new("abc").unwrap();
    buffer.revision = u64::MAX;
    assert_eq!(
        buffer.insert("x", EditKind::Typing, 0),
        Err(EditorError::RevisionExhausted)
    );
    assert_eq!(text(&buffer), "abc");
}

#[test]
fn history_budget_and_disabled_history_are_enforced() {
    let limits = EditorLimits {
        max_history_transactions: 2,
        max_history_bytes: 10_000,
        ..EditorLimits::default()
    };
    let mut buffer = TextBuffer::with_limits("", limits).unwrap();
    for time in 0..20 {
        buffer.insert("x", EditKind::Paste, time).unwrap();
    }
    assert_eq!(buffer.history_transaction_count(), 2);
    assert!(buffer.history_bytes() <= limits.max_history_bytes);
    buffer.undo().unwrap();
    buffer.undo().unwrap();
    assert!(!buffer.undo().unwrap());
    assert_eq!(text(&buffer), "x".repeat(18));
    let mut disabled = TextBuffer::with_limits(
        "",
        EditorLimits {
            max_history_bytes: 0,
            ..limits
        },
    )
    .unwrap();
    disabled.insert("x", EditKind::Paste, 0).unwrap();
    assert!(!disabled.can_undo());
    assert_eq!(disabled.history_bytes(), 0);
}

#[test]
fn multi_edit_transaction_is_atomic_and_uses_original_offsets() {
    let mut buffer = TextBuffer::new("abcdef").unwrap();
    let edits = [
        TextEdit {
            range: 4..5,
            replacement: "EE".into(),
        },
        TextEdit {
            range: 1..2,
            replacement: "BB".into(),
        },
    ];
    buffer
        .apply_transaction(&edits, Selection::caret(8), EditKind::Indent, 0)
        .unwrap();
    assert_eq!(text(&buffer), "aBBcdEEf");
    assert_eq!(buffer.history_transaction_count(), 1);
    buffer.undo().unwrap();
    assert_eq!(text(&buffer), "abcdef");
    buffer.redo().unwrap();
    assert_eq!(text(&buffer), "aBBcdEEf");
    let revision = buffer.revision();
    let invalid = [
        TextEdit {
            range: 1..3,
            replacement: "x".into(),
        },
        TextEdit {
            range: 2..4,
            replacement: "y".into(),
        },
    ];
    assert_eq!(
        buffer.apply_transaction(&invalid, Selection::caret(0), EditKind::Replace, 1),
        Err(EditorError::OverlappingEdits)
    );
    assert_eq!(text(&buffer), "aBBcdEEf");
    assert_eq!(buffer.revision(), revision);
}

#[test]
fn transaction_limits_and_resulting_selection_are_checked_before_mutation() {
    let mut buffer = TextBuffer::new("abc").unwrap();
    let invalid = [TextEdit {
        range: 0..1,
        replacement: "x".into(),
    }];
    assert!(buffer
        .apply_transaction(&invalid, Selection::caret(99), EditKind::Replace, 0)
        .is_err());
    let duplicate = [
        TextEdit {
            range: 1..1,
            replacement: "x".into(),
        },
        TextEdit {
            range: 1..1,
            replacement: "y".into(),
        },
    ];
    assert!(matches!(
        buffer.apply_transaction(&duplicate, Selection::caret(0), EditKind::Replace, 0),
        Err(EditorError::OverlappingEdits)
    ));
    assert_eq!(text(&buffer), "abc");
    let too_many = vec![
        TextEdit {
            range: 0..0,
            replacement: String::new()
        };
        4097
    ];
    assert!(matches!(
        buffer.apply_transaction(&too_many, Selection::caret(0), EditKind::Replace, 0),
        Err(EditorError::TransactionTooLarge { .. })
    ));
}

#[test]
fn randomized_scalar_edits_match_reference_string_and_all_undo_states() {
    let replacements = ["", "a", "界", "😀", "e\u{301}", "\r\n", "👩‍💻", "\txy\n"];
    for seed in 1..=16u64 {
        let mut rng = seed;
        let mut reference = "base\r\n界".to_string();
        let mut buffer = TextBuffer::new(&reference).unwrap();
        let mut states = vec![reference.clone()];
        for time in 0..256u64 {
            let count = reference.chars().count();
            let start = random(&mut rng) as usize % (count + 1);
            let end = start + random(&mut rng) as usize % (count - start + 1);
            let replacement = replacements[random(&mut rng) as usize % replacements.len()];
            let changed = buffer
                .edit(
                    start..end,
                    replacement,
                    Selection::caret(start + replacement.chars().count()),
                    EditKind::Replace,
                    time,
                )
                .unwrap();
            let start_byte = char_byte(&reference, start);
            let end_byte = char_byte(&reference, end);
            let expected_change = &reference[start_byte..end_byte] != replacement;
            reference.replace_range(start_byte..end_byte, replacement);
            assert_eq!(changed, expected_change);
            assert_eq!(text(&buffer), reference, "seed {seed} time {time}");
            if changed {
                states.push(reference.clone());
            }
            let snapshot = buffer.snapshot();
            assert_eq!(snapshot.byte_count(), reference.len());
            assert_eq!(snapshot.char_count(), reference.chars().count());
            assert_eq!(snapshot.chunks().collect::<String>(), reference);
        }
        for state in states[..states.len() - 1].iter().rev() {
            assert!(buffer.undo().unwrap());
            assert_eq!(&text(&buffer), state);
        }
        assert!(!buffer.undo().unwrap());
        for state in states.iter().skip(1) {
            assert!(buffer.redo().unwrap());
            assert_eq!(&text(&buffer), state);
        }
        assert!(!buffer.redo().unwrap());
    }
}

fn random(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn display_columns_tabs_wide_emoji_and_combining_clusters_map_round_trip() {
    let snapshot = TextBuffer::new("\tA界👩‍💻e\u{301}").unwrap().snapshot();
    for (offset, column) in [(0, 0), (1, 4), (2, 5), (3, 7), (6, 9), (8, 10)] {
        assert_eq!(snapshot.display_column(offset, 4).unwrap(), column);
        assert_eq!(
            snapshot.offset_for_display_column(0, column, 4).unwrap(),
            offset
        );
    }
    assert_eq!(snapshot.offset_for_display_column(0, 2, 4).unwrap(), 0);
    assert_eq!(snapshot.offset_for_display_column(0, 3, 4).unwrap(), 1);
    assert_eq!(snapshot.offset_for_display_column(0, 999, 4).unwrap(), 8);
    assert!(snapshot.display_column(0, 0).is_err());
    assert!(snapshot.offset_for_display_column(0, 1, 33).is_err());
}

#[test]
fn horizontal_window_is_bounded_and_preserves_grapheme_and_physical_ranges() {
    let snapshot = TextBuffer::new("\tA界👩‍💻e\u{301}").unwrap().snapshot();
    let window = snapshot.display_line_window(0, 5, 4, 4).unwrap();
    assert_eq!(window.text, "界👩‍💻");
    assert_eq!(window.char_range, 2..6);
    assert_eq!((window.start_column, window.end_column), (5, 9));
    assert!(window.truncated);
    let window = snapshot.display_line_window(0, 0, 20, 4).unwrap();
    assert_eq!(window.text, "    A界👩‍💻e\u{301}");
    assert!(!window.truncated);
    assert!(snapshot.display_line_window(0, 0, 8193, 4).is_err());
}

#[test]
fn viewport_lookup_does_not_copy_full_document_or_scan_all_rows() {
    let document = "0123456789abcdef\n".repeat(100_000);
    let snapshot = TextBuffer::new(&document).unwrap().snapshot();
    let rows = snapshot
        .visible_lines(Viewport {
            first_row: 90_000,
            row_count: 30,
            overscan: 2,
        })
        .unwrap();
    assert_eq!(rows.len(), 34);
    assert_eq!(rows[0].row, 89_998);
    assert_eq!(rows.last().unwrap().row, 90_031);
    assert_eq!(rows[0].char_range.len(), 16);
    assert!(snapshot
        .visible_lines(Viewport {
            first_row: 0,
            row_count: 1025,
            overscan: 0
        })
        .is_err());
    assert!(snapshot
        .visible_lines(Viewport {
            first_row: 0,
            row_count: 1,
            overscan: usize::MAX
        })
        .is_err());
}

#[test]
fn giant_line_window_caps_render_allocation_but_keeps_complete_source() {
    let value = format!("x{}tail", "\u{301}".repeat(40_000));
    let snapshot = TextBuffer::new(&value).unwrap().snapshot();
    let window = snapshot.display_line_window(0, 0, 10, 4).unwrap();
    assert!(window.text.len() <= 64 * 1024);
    assert!(window.truncated);
    assert_eq!(snapshot.chunks().collect::<String>(), value);
}

#[test]
fn snapshots_are_send_sync_and_can_be_saved_after_buffer_changes() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<BufferSnapshot>();
    let mut buffer = TextBuffer::new("original\r\n").unwrap();
    let snapshot = buffer.snapshot();
    let worker = std::thread::spawn(move || {
        let mut output = Vec::new();
        snapshot.write_to(&mut output).unwrap();
        output
    });
    buffer.insert("changed", EditKind::Paste, 0).unwrap();
    assert_eq!(worker.join().unwrap(), b"original\r\n");
}

#[test]
#[ignore = "opt-in timing benchmark; run with --ignored --nocapture"]
fn benchmark_large_document_snapshots_edits_and_visible_rows() {
    use std::hint::black_box;
    use std::time::Instant;
    let source = "    let value = 123; // Unicode 界😀\r\n".repeat(150_000);
    let start = Instant::now();
    let mut buffer = TextBuffer::new(&source).unwrap();
    let load = start.elapsed();
    let start = Instant::now();
    for _ in 0..10_000 {
        black_box(buffer.snapshot());
    }
    let snapshots = start.elapsed();
    let start = Instant::now();
    for time in 0..1000 {
        let offset = buffer
            .snapshot()
            .offset_of(Position {
                row: 100_000,
                column: 5,
            })
            .unwrap();
        buffer
            .edit(
                offset..offset + 1,
                if time % 2 == 0 { "x" } else { "y" },
                Selection::caret(offset + 1),
                EditKind::Replace,
                time,
            )
            .unwrap();
    }
    let edits = start.elapsed();
    let snapshot = buffer.snapshot();
    let start = Instant::now();
    for _ in 0..1000 {
        black_box(
            snapshot
                .visible_lines(Viewport {
                    first_row: 120_000,
                    row_count: 40,
                    overscan: 2,
                })
                .unwrap(),
        );
    }
    let visible = start.elapsed();
    eprintln!(
        "editor core debug: bytes={} rows={} load={load:?} snapshots10000={snapshots:?} edits1000={edits:?} visible44rows1000={visible:?}",
        source.len(),
        snapshot.line_count()
    );
}
