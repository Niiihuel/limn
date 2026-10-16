//! Editing commands over rope snapshots; no GUI or filesystem authority.
use crate::{BufferSnapshot, EditKind, EditorError, Selection, TextBuffer, TextEdit};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Space,
    Newline,
    Identifier,
    Punctuation,
}

fn word_class(character: char) -> WordClass {
    if matches!(
        character,
        '\r' | '\n' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    ) {
        WordClass::Newline
    } else if character.is_whitespace() {
        WordClass::Space
    } else if character.is_alphanumeric() || character == '_' || character.width() == Some(0) {
        // Combining marks belong to their identifier; final targets are also
        // snapped to grapheme boundaries (emoji joins cannot be split).
        WordClass::Identifier
    } else {
        WordClass::Punctuation
    }
}

impl BufferSnapshot {
    /// Start of the preceding identifier/punctuation run. Horizontal whitespace
    /// is crossed, but a line break is a distinct stop rather than deleting code
    /// from the previous line together with the current indentation.
    pub fn previous_word_start(&self, offset: usize) -> Result<usize, EditorError> {
        self.word_target(offset, false)
    }

    /// End of the next identifier/punctuation run, on a grapheme boundary.
    pub fn next_word_end(&self, offset: usize) -> Result<usize, EditorError> {
        self.word_target(offset, true)
    }

    fn word_target(&self, offset: usize, forward: bool) -> Result<usize, EditorError> {
        let start = if forward {
            self.snap_grapheme_forward(offset)?
        } else {
            self.snap_grapheme_backward(offset)?
        };
        let mut chars = self.rope.chars_at(start);
        let mut cursor = start;
        let mut group = None;
        while let Some(character) = if forward { chars.next() } else { chars.prev() } {
            let class = word_class(character);
            if class == WordClass::Newline && cursor != start {
                break;
            }
            if let Some(group) = group {
                if class != group {
                    break;
                }
            } else if class != WordClass::Space {
                group = Some(class);
            }
            cursor = if forward { cursor + 1 } else { cursor - 1 };
            if class == WordClass::Newline {
                break;
            }
        }
        if forward {
            self.snap_grapheme_forward(cursor)
        } else {
            self.snap_grapheme_backward(cursor)
        }
    }

    /// Home first visits the first non-indent character; a second Home visits
    /// column zero. Ctrl+Home remains a separate document-level command.
    pub fn smart_line_start(&self, offset: usize) -> Result<usize, EditorError> {
        let range = self.line_range(self.position_of(offset)?.row)?;
        let indentation = self
            .rope
            .slice(range.clone())
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .count();
        let content = range.start + indentation;
        Ok(if offset == content {
            range.start
        } else {
            content
        })
    }
}

impl TextBuffer {
    /// Delete from the caret back to `previous_word_start`. A non-empty selection is deleted
    /// instead, as `delete_backward` does.
    pub fn delete_word_backward(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        if self.selections().count() > 1 {
            return self.delete_at_selections(false, true, timestamp_ms);
        }
        if !self.selection().is_empty() {
            return self.delete_backward(timestamp_ms);
        }
        let snapshot = self.snapshot();
        let end = snapshot.snap_grapheme_forward(self.selection().head)?;
        let start = snapshot.previous_word_start(end)?;
        self.edit(
            start..end,
            "",
            Selection::caret(start),
            EditKind::Replace,
            timestamp_ms,
        )
    }

    /// Delete from the caret forward to `next_word_end`. A non-empty selection is deleted
    /// instead, as `delete_forward` does.
    pub fn delete_word_forward(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        if self.selections().count() > 1 {
            return self.delete_at_selections(true, true, timestamp_ms);
        }
        if !self.selection().is_empty() {
            return self.delete_forward(timestamp_ms);
        }
        let snapshot = self.snapshot();
        let start = snapshot.snap_grapheme_backward(self.selection().head)?;
        let end = snapshot.next_word_end(start)?;
        self.edit(
            start..end,
            "",
            Selection::caret(start),
            EditKind::Replace,
            timestamp_ms,
        )
    }

    /// Insert spaces to the next visual tab stop for an empty selection, or
    /// indent/outdent the selected physical lines as one undo transaction.
    /// Only line prefixes are edited; original LF/CRLF/CR bytes remain exact.
    pub fn indent_selection(
        &mut self,
        reverse: bool,
        tab_width: usize,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        if !(1..=32).contains(&tab_width) {
            return Err(EditorError::InvalidTabWidth);
        }
        let snapshot = self.snapshot();
        let selection = self.selection();
        if selection.is_empty() && !reverse {
            let column = snapshot.display_column(selection.head, tab_width)?;
            let spaces = tab_width - column % tab_width;
            return self.insert(&" ".repeat(spaces), EditKind::Indent, timestamp_ms);
        }
        let range = selection.range();
        let first = snapshot.position_of(range.start)?.row;
        let mut last = snapshot.position_of(range.end)?.row;
        // A selection ending at the next line's column zero excludes that line.
        if !range.is_empty() && snapshot.line_range(last)?.start == range.end {
            last = last.saturating_sub(1);
        }
        let count = last - first + 1;
        if count > 4096 {
            return Err(EditorError::TransactionTooLarge {
                edits: count,
                limit: 4096,
            });
        }
        let mut edits = Vec::with_capacity(count);
        for row in first..=last {
            let line = snapshot.line_range(row)?;
            if reverse {
                let mut characters = snapshot.rope.slice(line.clone()).chars();
                let remove = match characters.next() {
                    Some('\t') => 1,
                    Some(' ') => {
                        let spaces =
                            1 + characters.take_while(|character| *character == ' ').count();
                        let remainder = spaces % tab_width;
                        if remainder == 0 {
                            tab_width
                        } else {
                            remainder
                        }
                    }
                    _ => 0,
                };
                if remove > 0 {
                    edits.push(TextEdit {
                        range: line.start..line.start + remove,
                        replacement: String::new(),
                    });
                }
            } else {
                edits.push(TextEdit {
                    range: line.start..line.start,
                    replacement: " ".repeat(tab_width),
                });
            }
        }
        if edits.is_empty() {
            return Ok(false);
        }
        // Map each directed endpoint through original-coordinate edits. A
        // caret inside removed indent clips to the remaining line's start.
        let mapped = |offset: usize| {
            let mut delta = 0isize;
            for edit in &edits {
                if offset < edit.range.start {
                    break;
                }
                let inserted = edit.replacement.len(); // Prefixes are ASCII spaces.
                if offset <= edit.range.end {
                    return edit.range.start.saturating_add_signed(delta) + inserted;
                }
                delta += inserted as isize - edit.range.len() as isize;
            }
            offset.saturating_add_signed(delta)
        };
        let after = Selection {
            anchor: mapped(selection.anchor),
            head: mapped(selection.head),
        };
        self.apply_transaction(&edits, after, EditKind::Indent, timestamp_ms)
    }

    /// Remove the spaces and tabs that end each line, as one undo step; line terminators stay
    /// exact and every selection keeps its place in its line. Returns `Ok(false)` when no line
    /// ends in either. Past 4096 changed lines it is one replacement spanning all of them, and
    /// only the primary selection is kept.
    pub fn trim_trailing_whitespace(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        let snapshot = self.snapshot();
        let mut edits = Vec::new();
        for row in 0..snapshot.line_count() {
            let line = snapshot.line_range(row)?;
            let text = snapshot.line_text(row)?;
            let trailing = text.chars().count() - text.trim_end_matches([' ', '\t']).chars().count();
            if trailing > 0 {
                edits.push(TextEdit {
                    range: line.end - trailing..line.end,
                    replacement: String::new(),
                });
            }
        }
        if edits.is_empty() {
            return Ok(false);
        }
        // An endpoint inside removed whitespace moves to where that whitespace began.
        let mapped = |offset: usize| {
            let mut removed = 0;
            for edit in edits.iter().take_while(|edit| edit.range.start < offset) {
                removed += offset.min(edit.range.end) - edit.range.start;
            }
            offset - removed
        };
        let map = |selection: Selection| Selection {
            anchor: mapped(selection.anchor),
            head: mapped(selection.head),
        };
        if edits.len() <= 4096 {
            let selections = self.selections().map(map).collect();
            return self.apply_transaction_selections(&edits, selections, EditKind::Replace, timestamp_ms);
        }
        let span = edits[0].range.start..edits[edits.len() - 1].range.end;
        let mut replacement = String::new();
        let mut cursor = span.start;
        for edit in &edits {
            replacement.push_str(&snapshot.text_range(cursor..edit.range.start)?);
            cursor = edit.range.end;
        }
        self.edit(span, &replacement, map(self.selection()), EditKind::Replace, timestamp_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(buffer: &TextBuffer) -> String {
        let snapshot = buffer.snapshot();
        snapshot.text_range(0..snapshot.char_count()).unwrap()
    }

    #[test]
    fn word_navigation_preserves_unicode_clusters_and_line_break_stops() {
        let buffer = TextBuffer::new("cafe\u{301}_東京 :: 🧑‍💻\r\n  next").unwrap();
        let snapshot = buffer.snapshot();
        let word = "cafe\u{301}_東京".chars().count();
        assert_eq!(snapshot.next_word_end(0).unwrap(), word);
        assert_eq!(snapshot.previous_word_start(word).unwrap(), 0);
        assert_eq!(snapshot.next_word_end(word).unwrap(), word + 3);
        let emoji_end = word + 7;
        assert_eq!(snapshot.next_word_end(word + 3).unwrap(), emoji_end);
        assert_eq!(snapshot.previous_word_start(emoji_end).unwrap(), word + 4);
        assert_eq!(snapshot.next_word_end(emoji_end).unwrap(), emoji_end + 2);
        assert_eq!(
            snapshot.previous_word_start(emoji_end + 4).unwrap(),
            emoji_end + 2
        );
        assert_eq!(
            snapshot.previous_word_start(emoji_end + 2).unwrap(),
            emoji_end
        );
        for offset in 0..=snapshot.char_count() {
            assert!(snapshot
                .is_grapheme_boundary(snapshot.previous_word_start(offset).unwrap())
                .unwrap());
            assert!(snapshot
                .is_grapheme_boundary(snapshot.next_word_end(offset).unwrap())
                .unwrap());
        }
        assert!(snapshot.next_word_end(snapshot.char_count() + 1).is_err());
    }

    #[test]
    fn word_deletion_is_one_undoable_edit_and_partial_selections_expand_graphemes() {
        let mut buffer = TextBuffer::new("alpha cafe\u{301}_東京").unwrap();
        let end = buffer.snapshot().char_count();
        buffer.set_selection(Selection::caret(end)).unwrap();
        buffer.delete_word_backward(10).unwrap();
        assert_eq!(text(&buffer), "alpha ");
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), "alpha cafe\u{301}_東京");
        assert_eq!(buffer.selection(), Selection::caret(end));
        buffer.set_selection(Selection::caret(0)).unwrap();
        buffer.delete_word_forward(20).unwrap();
        assert_eq!(text(&buffer), " cafe\u{301}_東京");
        buffer.undo().unwrap();
        buffer
            .set_selection(Selection {
                anchor: 10,
                head: 9,
            })
            .unwrap();
        buffer.delete_word_forward(30).unwrap();
        assert_eq!(text(&buffer), "alpha caf_東京");
    }

    #[test]
    fn smart_home_toggles_indent_start_without_changing_document() {
        let buffer = TextBuffer::new("  \tfn main()\r\n    \rnext").unwrap();
        let snapshot = buffer.snapshot();
        assert_eq!(snapshot.smart_line_start(8).unwrap(), 3);
        assert_eq!(snapshot.smart_line_start(3).unwrap(), 0);
        assert_eq!(snapshot.smart_line_start(0).unwrap(), 3);
        let empty = snapshot.line_range(1).unwrap();
        assert_eq!(snapshot.smart_line_start(empty.start).unwrap(), empty.end);
        assert_eq!(snapshot.smart_line_start(empty.end).unwrap(), empty.start);
    }

    #[test]
    fn tabs_follow_visual_stops_including_wide_and_combined_characters() {
        for (source, spaces) in [("ab", 2), ("終", 2), ("e\u{301}", 3), ("\t", 4)] {
            let mut buffer = TextBuffer::new(source).unwrap();
            buffer
                .set_selection(Selection::caret(source.chars().count()))
                .unwrap();
            buffer.indent_selection(false, 4, 1).unwrap();
            assert_eq!(text(&buffer), format!("{source}{}", " ".repeat(spaces)));
            buffer.undo().unwrap();
            assert_eq!(text(&buffer), source);
        }
    }

    #[test]
    fn multiline_indent_preserves_crlf_cr_selection_direction_and_undo() {
        let original = "  uno\r\n\tdos\rtres\ncuatro";
        for reversed in [false, true] {
            let mut buffer = TextBuffer::new(original).unwrap();
            let end = buffer.snapshot().line_range(3).unwrap().start;
            let selection = if reversed {
                Selection {
                    anchor: end,
                    head: 0,
                }
            } else {
                Selection {
                    anchor: 0,
                    head: end,
                }
            };
            buffer.set_selection(selection).unwrap();
            buffer.indent_selection(false, 4, 1).unwrap();
            assert_eq!(text(&buffer), "      uno\r\n    \tdos\r    tres\ncuatro");
            assert_eq!(buffer.revision(), 1);
            assert_eq!(buffer.history_transaction_count(), 1);
            assert_eq!(
                buffer.selection().anchor > buffer.selection().head,
                reversed
            );
            buffer.undo().unwrap();
            assert_eq!(text(&buffer), original);
            assert_eq!(buffer.selection(), selection);
            buffer.redo().unwrap();
            assert_eq!(text(&buffer), "      uno\r\n    \tdos\r    tres\ncuatro");
        }
        let mut buffer = TextBuffer::new("      a\r\n\tb\rc").unwrap();
        buffer
            .set_selection(Selection {
                anchor: 0,
                head: buffer.snapshot().char_count(),
            })
            .unwrap();
        buffer.indent_selection(true, 4, 1).unwrap();
        assert_eq!(text(&buffer), "    a\r\nb\rc");
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), "      a\r\n\tb\rc");
    }

    #[test]
    fn outdent_clips_a_caret_in_indent_and_noop_keeps_revision() {
        let mut buffer = TextBuffer::new("    first\nnext").unwrap();
        buffer.set_selection(Selection::caret(2)).unwrap();
        buffer.indent_selection(true, 4, 1).unwrap();
        assert_eq!(text(&buffer), "first\nnext");
        assert_eq!(buffer.selection(), Selection::caret(0));
        let revision = buffer.revision();
        assert!(!buffer.indent_selection(true, 4, 2).unwrap());
        assert_eq!(buffer.revision(), revision);
        assert!(buffer.indent_selection(false, 0, 3).is_err());
        assert_eq!(text(&buffer), "first\nnext");
    }

    #[test]
    fn oversized_indentation_transaction_is_rejected_without_partial_edits() {
        let source = "line\n".repeat(4097);
        let mut buffer = TextBuffer::new(&source).unwrap();
        let selection = Selection {
            anchor: 0,
            head: buffer.snapshot().char_count(),
        };
        buffer.set_selection(selection).unwrap();
        assert!(matches!(
            buffer.indent_selection(false, 4, 1),
            Err(EditorError::TransactionTooLarge { .. })
        ));
        assert_eq!(text(&buffer), source);
        assert_eq!(buffer.selection(), selection);
        assert_eq!(buffer.revision(), 0);
    }

    #[test]
    fn trailing_whitespace_is_trimmed_as_one_undo_step_keeping_terminators_and_selections() {
        let mut buffer = TextBuffer::new("a  \r\n\tb\t\nc\n  ").unwrap();
        buffer
            .set_selections(vec![Selection::caret(2), Selection { anchor: 6, head: 10 }])
            .unwrap();
        assert!(buffer.trim_trailing_whitespace(1).unwrap());
        assert_eq!(text(&buffer), "a\r\n\tb\nc\n");
        let selections: Vec<_> = buffer.selections().collect();
        assert_eq!(selections, [Selection::caret(1), Selection { anchor: 4, head: 7 }]);
        assert!(!buffer.trim_trailing_whitespace(2).unwrap());
        buffer.undo().unwrap();
        assert_eq!(text(&buffer), "a  \r\n\tb\t\nc\n  ");

        let source = "x \n".repeat(5000);
        let mut large = TextBuffer::new(&source).unwrap();
        large.set_selection(Selection::caret(source.chars().count())).unwrap();
        assert!(large.trim_trailing_whitespace(1).unwrap());
        assert_eq!(text(&large), "x\n".repeat(5000));
        assert_eq!(large.selection(), Selection::caret(10_000));
    }
}
