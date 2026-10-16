//! Explicit editor commands, multiple selections and bounded snippet sessions.
use crate::{
    validate_selection, BufferSnapshot, EditKind, EditorError, LineEnding, Position, SearchOptions,
    Selection, TextBuffer, TextEdit,
};
use std::{collections::BTreeMap, ops::Range, sync::Arc};

const CURSOR_LIMIT: usize = 256;
/// What one indent level inserts. The number is its width in columns: the spaces written, or
/// the distance between tab stops. The default is four spaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    /// One level is this many spaces. Must be within 1..=32.
    Spaces(usize),
    /// One level is one tab character, counted as this many columns. Must be within 1..=32.
    Tabs(usize),
}
impl Default for IndentStyle {
    fn default() -> Self {
        Self::Spaces(4)
    }
}
impl IndentStyle {
    /// Columns in one indent level.
    pub fn width(self) -> usize {
        match self {
            Self::Spaces(n) | Self::Tabs(n) => n,
        }
    }
    fn unit(self) -> Result<String, EditorError> {
        if !(1..=32).contains(&self.width()) {
            return Err(EditorError::InvalidTabWidth);
        }
        Ok(match self {
            Self::Spaces(n) => " ".repeat(n),
            Self::Tabs(_) => "\t".into(),
        })
    }
}
/// An editing operation for `TextBuffer::execute_command`.
#[derive(Clone, Debug)]
pub enum EditorCommand {
    /// Make this the only selection, dropping every secondary one.
    SetSelection(Selection),
    /// Replace all selections; the first is the primary. At most 256, not empty. Selections
    /// that overlap are merged, and so is a caret at the end of another selection.
    SetSelections(Vec<Selection>),
    /// Put a single caret at a row and column, clipped to the last line and the line's length.
    Goto(Position),
    /// Add a caret on the line below or above the primary one, at the same visual column.
    AddCursorVertical {
        /// True for the next line, false for the previous one.
        below: bool,
        /// Columns between tab stops, within 1..=32, used to find the visual column.
        tab_width: usize,
    },
    /// With an empty selection, select the word at the caret. Otherwise add the next occurrence
    /// of the selected text (case-sensitive, wrapping around), or select every one when `all`.
    SelectNextOccurrence {
        /// Select every occurrence in the document instead of only the next one.
        all: bool,
    },
    /// Put one selection on each line between two corners, from the column of one corner to
    /// the column of the other. Fails when the corners are 256 or more rows apart.
    SelectRectangle {
        /// The corner where the drag began. Its `column` counts display cells, not characters.
        anchor: Position,
        /// The opposite corner, with `column` in display cells too. Its row may be past the
        /// last line.
        head: Position,
        /// Columns between tab stops, within 1..=32.
        tab_width: usize,
    },
    /// Replace each selection with this text, recorded as a paste.
    InsertText(String),
    /// Insert a snippet; see `TextBuffer::insert_snippet` for the syntax.
    InsertSnippet(String),
    /// Move to the next snippet tab stop, or the previous one when `backward`.
    NextSnippet {
        /// Go to the previous tab stop instead of the next.
        backward: bool,
    },
    /// Insert a copy of every line a selection touches, below it.
    DuplicateLines,
    /// Swap the selected lines with the line above or below them. Fails when the selections
    /// do not cover one contiguous run of lines; does nothing at the edge of the document.
    MoveLines {
        /// True to move toward the end of the document.
        down: bool,
    },
    /// Remove every line a selection touches, terminator included.
    DeleteLines,
    /// Comment the selected lines that are not blank, or uncomment them when every one of
    /// them already is commented.
    ToggleLineComment {
        /// Opening marker, such as `//`: not empty, on one line, at most 16 bytes. It is
        /// written after the line's indentation, followed by a space.
        prefix: String,
        /// Closing marker for languages that only have block comments.
        suffix: String,
    },
    /// Indent, or outdent when `reverse`; see `TextBuffer::indent_with_style`.
    Indent {
        /// Remove one indent level instead of adding one.
        reverse: bool,
        /// What one level is.
        style: IndentStyle,
    },
    /// Rewrite every line terminator in the text and use this one for new lines.
    SetLineEnding(LineEnding),
    /// Revert the newest undo step, as `TextBuffer::undo`.
    Undo,
    /// Apply the last undone step again, as `TextBuffer::redo`.
    Redo,
    /// Replace every match of a search as one undo step. A search error comes back as
    /// `InvalidCommand` with its message.
    ReplaceAll {
        /// What to search for; empty finds nothing.
        query: String,
        /// The new text. With `options.regex` it may refer to capture groups, as `$1`.
        replacement: String,
        /// How `query` is read and where it is searched.
        options: SearchOptions,
    },
    /// Replace one match of a search. A search error comes back as `InvalidCommand`.
    ReplaceMatch {
        /// What to search for.
        query: String,
        /// The new text, as in `ReplaceAll`.
        replacement: String,
        /// How `query` is read and where it is searched.
        options: SearchOptions,
        /// Character range of the match to replace. Nothing changes unless the search
        /// still finds a match with exactly this range.
        range: Range<usize>,
    },
}
#[derive(Clone, Debug)]
pub(crate) struct SnippetSession {
    groups: Vec<Vec<Range<usize>>>,
    current: usize,
    end: usize,
}

pub(crate) fn normalize_selections(
    mut selections: Vec<Selection>,
    chars: usize,
) -> Result<Vec<Selection>, EditorError> {
    if selections.is_empty() {
        return Err(EditorError::InvalidCommand(
            "At least one cursor is required".into(),
        ));
    }
    if selections.len() > CURSOR_LIMIT {
        return Err(EditorError::TransactionTooLarge {
            edits: selections.len(),
            limit: CURSOR_LIMIT,
        });
    }
    for selection in &selections {
        validate_selection(*selection, chars)?;
    }
    let primary = selections[0];
    selections.sort_by_key(|s| (s.range().start, s.range().end));
    let mut result: Vec<Selection> = Vec::new();
    for selection in selections {
        if let Some(last) = result.last_mut() {
            let a = last.range();
            let b = selection.range();
            if a.end > b.start || a == b || (selection.is_empty() && b.start == a.end) {
                let start = a.start.min(b.start);
                let end = a.end.max(b.end);
                let backwards = if a.contains(&primary.head) || *last == primary {
                    primary.anchor > primary.head
                } else {
                    last.anchor > last.head
                };
                *last = if backwards {
                    Selection {
                        anchor: end,
                        head: start,
                    }
                } else {
                    Selection {
                        anchor: start,
                        head: end,
                    }
                };
                continue;
            }
        }
        result.push(selection);
    }
    if let Some(index) = result.iter().position(|s| {
        s.range().start <= primary.range().start && s.range().end >= primary.range().end
    }) {
        result.swap(0, index);
    }
    Ok(result)
}

pub(crate) fn map_offset(offset: usize, edits: &[TextEdit], right_bias: bool) -> usize {
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|e| e.range.start);
    let mut delta = 0isize;
    for edit in sorted {
        if offset < edit.range.start || (offset == edit.range.start && !right_bias) {
            break;
        }
        let inserted = edit.replacement.chars().count();
        if offset < edit.range.end || (edit.range.is_empty() && offset == edit.range.start) {
            return edit.range.start.saturating_add_signed(delta)
                + if right_bias { inserted } else { 0 };
        }
        delta += inserted as isize - edit.range.len() as isize;
    }
    offset.saturating_add_signed(delta)
}
impl BufferSnapshot {
    /// The primary selection first, then the secondary ones.
    pub fn selections(&self) -> impl Iterator<Item = Selection> + Clone + '_ {
        std::iter::once(self.selection).chain(self.secondary.iter().copied())
    }
}
impl TextBuffer {
    /// The primary selection first, then the secondary ones.
    pub fn selections(&self) -> impl Iterator<Item = Selection> + Clone + '_ {
        std::iter::once(self.selection).chain(self.secondary.iter().copied())
    }
    /// Replace all selections; the first one given is the primary. Selections that overlap are
    /// merged. An empty list, more than 256 selections or an end past the document is an error.
    pub fn set_selections(&mut self, selections: Vec<Selection>) -> Result<(), EditorError> {
        let selections = normalize_selections(selections, self.rope.len_chars())?;
        if self.selection != selections[0] || self.secondary.as_ref() != &selections[1..] {
            self.selection_epoch = Arc::new(());
        }
        self.break_undo_group();
        self.selection = selections[0];
        self.secondary = selections[1..].to_vec().into();
        Ok(())
    }
    /// Put a single caret at a zero-based row and character column, clipped to the last line and
    /// to the line's length, and end any snippet session.
    pub fn goto_line(&mut self, row: usize, column: usize) -> Result<(), EditorError> {
        let offset = self
            .snapshot()
            .clipped_offset_of(Position { row, column })?;
        self.snippet = None;
        self.set_selection(Selection::caret(offset))
    }
    /// Rectangular columns are visual columns (tabs/wide graphemes), rows are physical.
    pub fn select_rectangle(
        &mut self,
        anchor: Position,
        head: Position,
        tab_width: usize,
    ) -> Result<(), EditorError> {
        let snapshot = self.snapshot();
        let first = anchor.row.min(head.row);
        let last = anchor.row.max(head.row).min(snapshot.line_count() - 1);
        if last.saturating_sub(first) >= CURSOR_LIMIT {
            return Err(EditorError::TransactionTooLarge {
                edits: last - first + 1,
                limit: CURSOR_LIMIT,
            });
        }
        let mut selections = Vec::new();
        for row in first..=last {
            selections.push(Selection {
                anchor: snapshot.offset_for_display_column(row, anchor.column, tab_width)?,
                head: snapshot.offset_for_display_column(row, head.column, tab_width)?,
            });
        }
        if head.row >= anchor.row {
            selections.reverse();
        }
        self.set_selections(selections)
    }
    fn replacement_transaction(
        &mut self,
        replacements: Vec<(Selection, String)>,
        kind: EditKind,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        let edits: Vec<_> = replacements
            .iter()
            .map(|(s, text)| TextEdit {
                range: s.range(),
                replacement: text.clone(),
            })
            .collect();
        let after = edits
            .iter()
            .map(|e| Selection::caret(map_offset(e.range.end, &edits, true)))
            .collect();
        self.apply_transaction_selections(&edits, after, kind, timestamp)
    }
    pub(crate) fn insert_at_selections(
        &mut self,
        text: &str,
        kind: EditKind,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        self.replacement_transaction(
            self.selections().map(|s| (s, text.to_owned())).collect(),
            kind,
            timestamp,
        )
    }
    pub(crate) fn delete_at_selections(
        &mut self,
        forward: bool,
        word: bool,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        let snapshot = self.snapshot();
        let mut ranges = Vec::new();
        for selection in self.selections() {
            let mut r = selection.range();
            if r.is_empty() {
                if forward {
                    r.start = snapshot.snap_grapheme_backward(r.start)?;
                    r.end = if word {
                        snapshot.next_word_end(r.start)?
                    } else {
                        snapshot.next_grapheme(r.start)?
                    };
                } else {
                    r.end = snapshot.snap_grapheme_forward(r.end)?;
                    r.start = if word {
                        snapshot.previous_word_start(r.end)?
                    } else {
                        snapshot.prev_grapheme(r.end)?
                    };
                }
            } else {
                r.start = snapshot.snap_grapheme_backward(r.start)?;
                r.end = snapshot.snap_grapheme_forward(r.end)?;
            }
            ranges.push(Selection {
                anchor: r.start,
                head: r.end,
            });
        }
        let ranges = normalize_selections(ranges, snapshot.char_count())?;
        self.replacement_transaction(
            ranges.into_iter().map(|s| (s, String::new())).collect(),
            EditKind::Replace,
            timestamp,
        )
    }
    pub(crate) fn newline_at_selections(&mut self, timestamp: u64) -> Result<bool, EditorError> {
        let snapshot = self.snapshot();
        let mut replacements = Vec::new();
        for selection in self.selections() {
            let start = selection.range().start;
            let line = snapshot.line_range(snapshot.position_of(start)?.row)?;
            let indent: String = snapshot
                .rope
                .slice(line.start..start.min(line.end))
                .chars()
                .take_while(|c| matches!(c, ' ' | '\t'))
                .collect();
            replacements.push((selection, format!("{}{indent}", self.line_ending.as_str())));
        }
        self.replacement_transaction(replacements, EditKind::Newline, timestamp)
    }
    /// Insert typed text. With `auto_pairs`, typing an opening bracket, a quote or a backtick
    /// also inserts its closing character, around the selection when there is one, and typing a
    /// closing character that already follows every caret steps over it. A quote typed right
    /// after a letter or digit is inserted alone.
    pub fn type_text(
        &mut self,
        text: &str,
        auto_pairs: bool,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        if !auto_pairs || text.chars().count() != 1 {
            return self.insert(text, EditKind::Typing, timestamp);
        }
        let typed = text.chars().next().unwrap();
        let close = match typed {
            '(' => Some(')'),
            '[' => Some(']'),
            '{' => Some('}'),
            '"' => Some('"'),
            '\'' => Some('\''),
            '`' => Some('`'),
            _ => None,
        };
        let snapshot = self.snapshot();
        // Skip a closing delimiter only when every caret is already before it.
        if matches!(typed, ')' | ']' | '}' | '"' | '\'' | '`')
            && self
                .selections()
                .all(|s| s.is_empty() && snapshot.rope.get_char(s.head) == Some(typed))
        {
            self.set_selections(
                self.selections()
                    .map(|s| Selection::caret(s.head + 1))
                    .collect(),
            )?;
            return Ok(false);
        }
        let Some(close) = close else {
            return self.insert(text, EditKind::Typing, timestamp);
        };
        let selections: Vec<_> = self.selections().collect();
        let mut edits = Vec::new();
        for s in &selections {
            let r = s.range();
            // Quotes inside identifiers are literal; no language-specific guessing.
            if matches!(typed, '\'' | '"' | '`')
                && s.is_empty()
                && r.start > 0
                && snapshot.rope.char(r.start - 1).is_alphanumeric()
            {
                return self.insert(text, EditKind::Typing, timestamp);
            }
            edits.push(TextEdit {
                range: r.clone(),
                replacement: format!("{typed}{}{close}", snapshot.text_range(r)?),
            });
        }
        let after = selections
            .iter()
            .map(|s| {
                let start = map_offset(s.range().start, &edits, false) + 1;
                if s.is_empty() {
                    Selection::caret(start)
                } else if s.anchor > s.head {
                    Selection {
                        anchor: start + s.range().len(),
                        head: start,
                    }
                } else {
                    Selection {
                        anchor: start,
                        head: start + s.range().len(),
                    }
                }
            })
            .collect();
        self.apply_transaction_selections(&edits, after, EditKind::Replace, timestamp)
    }
    /// Backspace that removes both characters when every caret sits inside an empty bracket or
    /// quote pair; otherwise the same as `delete_backward`.
    pub fn delete_pair_backward(&mut self, timestamp: u64) -> Result<bool, EditorError> {
        let snapshot = self.snapshot();
        let paired = self.selections().all(|s| {
            s.is_empty()
                && s.head > 0
                && matches!(
                    (
                        snapshot.rope.get_char(s.head - 1),
                        snapshot.rope.get_char(s.head)
                    ),
                    (Some('('), Some(')'))
                        | (Some('['), Some(']'))
                        | (Some('{'), Some('}'))
                        | (Some('"'), Some('"'))
                        | (Some('\''), Some('\''))
                        | (Some('`'), Some('`'))
                )
        });
        if !paired {
            return self.delete_backward(timestamp);
        }
        self.replacement_transaction(
            self.selections()
                .map(|s| {
                    (
                        Selection {
                            anchor: s.head - 1,
                            head: s.head + 1,
                        },
                        String::new(),
                    )
                })
                .collect(),
            EditKind::Replace,
            timestamp,
        )
    }
    fn selected_rows(&self) -> Result<Vec<usize>, EditorError> {
        let snapshot = self.snapshot();
        let mut rows = std::collections::BTreeSet::new();
        for s in self.selections() {
            let r = s.range();
            let first = snapshot.position_of(r.start)?.row;
            let mut last = snapshot.position_of(r.end)?.row;
            if !r.is_empty() && snapshot.line_range(last)?.start == r.end {
                last = last.saturating_sub(1);
            }
            if last - first >= 4096 {
                return Err(EditorError::TransactionTooLarge {
                    edits: last - first + 1,
                    limit: 4096,
                });
            }
            rows.extend(first..=last);
        }
        Ok(rows.into_iter().collect())
    }
    /// Indent, or outdent when `reverse`. Indenting with only carets inserts a tab, or spaces up
    /// to the next indent stop, at each caret. Every other case adds or removes one indent level
    /// at the start of each selected line. Fails with `InvalidTabWidth` when the style's width
    /// is outside 1..=32.
    pub fn indent_with_style(
        &mut self,
        reverse: bool,
        style: IndentStyle,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        let unit = style.unit()?;
        let snapshot = self.snapshot();
        let selections: Vec<_> = self.selections().collect();
        if !reverse && selections.iter().all(|s| s.is_empty()) {
            let values = selections
                .iter()
                .map(|s| {
                    Ok((
                        *s,
                        match style {
                            IndentStyle::Tabs(_) => unit.clone(),
                            IndentStyle::Spaces(width) => {
                                " ".repeat(width - snapshot.display_column(s.head, width)? % width)
                            }
                        },
                    ))
                })
                .collect::<Result<Vec<_>, EditorError>>()?;
            return self.replacement_transaction(values, EditKind::Indent, timestamp);
        }
        let mut edits = Vec::new();
        for row in self.selected_rows()? {
            let line = snapshot.line_range(row)?;
            if reverse {
                let mut chars = snapshot.rope.slice(line.clone()).chars();
                let remove = match chars.next() {
                    Some('\t') => 1,
                    Some(' ') => {
                        let count = 1 + chars.take_while(|c| *c == ' ').count();
                        let rest = count % style.width();
                        if rest == 0 {
                            style.width()
                        } else {
                            rest
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
                    replacement: unit.clone(),
                });
            }
        }
        self.apply_preserving_selections(&edits, EditKind::Indent, timestamp)
    }
    fn apply_preserving_selections(
        &mut self,
        edits: &[TextEdit],
        kind: EditKind,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        let after = self
            .selections()
            .map(|s| Selection {
                anchor: map_offset(s.anchor, edits, true),
                head: map_offset(s.head, edits, true),
            })
            .collect();
        self.apply_transaction_selections(edits, after, kind, timestamp)
    }
    fn toggle_comment(
        &mut self,
        prefix: &str,
        suffix: &str,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        if prefix.is_empty()
            || prefix.contains(['\r', '\n'])
            || prefix.len() > 16
            || suffix.contains(['\r', '\n'])
            || suffix.len() > 16
        {
            return Err(EditorError::InvalidCommand(
                "A nonempty single-line comment prefix is required".into(),
            ));
        }
        let snapshot = self.snapshot();
        let rows = self.selected_rows()?;
        let mut starts = Vec::new();
        for row in rows {
            let r = snapshot.line_range(row)?;
            let indent = snapshot
                .rope
                .slice(r.clone())
                .chars()
                .take_while(|c| matches!(c, ' ' | '\t'))
                .count();
            let trailing = snapshot
                .rope
                .slice(r.clone())
                .chars_at(r.len())
                .reversed()
                .take_while(|c| matches!(c, ' ' | '\t'))
                .count();
            if r.start + indent < r.end {
                starts.push((r.start + indent, r.end - trailing));
            }
        }
        let markers = prefix.chars().count() + suffix.chars().count();
        let remove = starts.iter().all(|(start, end)| {
            let line = snapshot.rope.slice(*start..*end).to_string();
            end - start >= markers && line.starts_with(prefix) && line.ends_with(suffix)
        });
        let mut edits = Vec::new();
        for (start, end) in starts {
            if remove {
                let mut count = prefix.chars().count();
                let mut closing = end - suffix.chars().count();
                if start + count < closing && snapshot.rope.char(start + count) == ' ' {
                    count += 1;
                }
                if !suffix.is_empty()
                    && closing > start + count
                    && snapshot.rope.char(closing - 1) == ' '
                {
                    closing -= 1;
                }
                edits.push(TextEdit {
                    range: start..start + count,
                    replacement: String::new(),
                });
                if closing < end {
                    edits.push(TextEdit {
                        range: closing..end,
                        replacement: String::new(),
                    });
                }
            } else {
                edits.push(TextEdit {
                    range: start..start,
                    replacement: format!("{prefix} "),
                });
                if !suffix.is_empty() {
                    edits.push(TextEdit {
                        range: end..end,
                        replacement: format!(" {suffix}"),
                    });
                }
            }
        }
        self.apply_preserving_selections(&edits, EditKind::Replace, timestamp)
    }
    fn line_command(
        &mut self,
        command: &EditorCommand,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        let snapshot = self.snapshot();
        let rows = self.selected_rows()?;
        let Some(first) = rows.first().copied() else {
            return Ok(false);
        };
        let last = *rows.last().unwrap();
        // Disjoint line groups retain their independent meaning for delete/duplicate.
        let mut groups = Vec::<Range<usize>>::new();
        for row in rows {
            if let Some(last) = groups.last_mut().filter(|r| r.end == row) {
                last.end += 1;
            } else {
                groups.push(row..row + 1);
            }
        }
        if matches!(command, EditorCommand::MoveLines { .. }) && groups.len() != 1 {
            return Err(EditorError::InvalidCommand(
                "Move lines requires one contiguous line selection".into(),
            ));
        }
        let mut edits = Vec::new();
        for group in groups {
            let start = snapshot.line_range(group.start)?.start;
            let end = if group.end < snapshot.line_count() {
                snapshot.line_range(group.end)?.start
            } else {
                snapshot.char_count()
            };
            match command {
                EditorCommand::DeleteLines => {
                    let start = if group.end == snapshot.line_count() && group.start > 0 {
                        snapshot.line_range(group.start - 1)?.end
                    } else {
                        start
                    };
                    edits.push(TextEdit {
                        range: start..end,
                        replacement: String::new(),
                    });
                }
                EditorCommand::DuplicateLines => {
                    let text = snapshot.text_range(start..end)?;
                    let replacement =
                        if end == snapshot.char_count() && !text.ends_with(['\n', '\r']) {
                            format!("{}{text}", self.line_ending.as_str())
                        } else {
                            text
                        };
                    edits.push(TextEdit {
                        range: end..end,
                        replacement,
                    });
                }
                EditorCommand::MoveLines { down } => {
                    if (*down && group.end >= snapshot.line_count()) || (!*down && group.start == 0)
                    {
                        return Ok(false);
                    }
                    let neighbour = if *down { group.end } else { group.start - 1 };
                    let mut contents = Vec::new();
                    let from = group.start.min(neighbour);
                    let to = (group.end - 1).max(neighbour);
                    for row in from..=to {
                        contents.push(snapshot.line_text(row)?);
                    }
                    if *down {
                        contents.rotate_right(1);
                    } else {
                        contents.rotate_left(1);
                    }
                    // Preserve each original row's terminator, including an unterminated EOF.
                    let mut replacement = String::new();
                    for (index, row) in (from..=to).enumerate() {
                        replacement.push_str(&contents[index]);
                        let content = snapshot.line_range(row)?;
                        let full = snapshot.line(row)?;
                        replacement.push_str(&full.slice(content.len()..).to_string());
                    }
                    let range = snapshot.line_range(from)?.start
                        ..(if to + 1 < snapshot.line_count() {
                            snapshot.line_range(to + 1)?.start
                        } else {
                            snapshot.char_count()
                        });
                    let delta = if *down { 1isize } else { -1 };
                    let mut temp = TextBuffer::new(&replacement)?; // Bounded by the existing document limit.
                    temp.line_ending = self.line_ending;
                    let temp_snapshot = temp.snapshot();
                    let after = self
                        .selections()
                        .map(|s| {
                            let map = |offset| -> Result<usize, EditorError> {
                                let p = snapshot.position_of(offset)?;
                                let row = p.row.saturating_add_signed(delta).saturating_sub(from);
                                Ok(range.start
                                    + temp_snapshot.clipped_offset_of(Position {
                                        row,
                                        column: p.column,
                                    })?)
                            };
                            Ok(Selection {
                                anchor: map(s.anchor)?,
                                head: map(s.head)?,
                            })
                        })
                        .collect::<Result<Vec<_>, EditorError>>()?;
                    return self.apply_transaction_selections(
                        &[TextEdit { range, replacement }],
                        after,
                        EditKind::Replace,
                        timestamp,
                    );
                }
                _ => unreachable!(),
            }
        }
        let _ = (first, last);
        self.apply_preserving_selections(&edits, EditKind::Replace, timestamp)
    }
    /// Run one command. Returns `Ok(true)` when the text changed; commands that only move
    /// selections return `Ok(false)`.
    pub fn execute_command(
        &mut self,
        command: EditorCommand,
        timestamp: u64,
    ) -> Result<bool, EditorError> {
        match command {
            EditorCommand::SetSelection(s) => {
                self.set_selection(s)?;
                Ok(false)
            }
            EditorCommand::SetSelections(s) => {
                self.set_selections(s)?;
                Ok(false)
            }
            EditorCommand::Goto(p) => {
                self.goto_line(p.row, p.column)?;
                Ok(false)
            }
            EditorCommand::SelectRectangle {
                anchor,
                head,
                tab_width,
            } => {
                self.select_rectangle(anchor, head, tab_width)?;
                Ok(false)
            }
            EditorCommand::AddCursorVertical { below, tab_width } => {
                let snapshot = self.snapshot();
                let s = self.selection();
                let p = snapshot.position_of(s.head)?;
                let row = if below {
                    (p.row + 1).min(snapshot.line_count() - 1)
                } else {
                    p.row.saturating_sub(1)
                };
                let col = snapshot.display_column(s.head, tab_width)?;
                let offset = snapshot.offset_for_display_column(row, col, tab_width)?;
                let mut selections = vec![Selection::caret(offset)];
                selections.extend(self.selections());
                self.set_selections(selections)?;
                Ok(false)
            }
            EditorCommand::SelectNextOccurrence { all } => {
                let snapshot = self.snapshot();
                let selection = self.selection();
                if selection.is_empty() {
                    let start = snapshot.previous_word_start(selection.head)?;
                    let end = snapshot.next_word_end(start)?;
                    self.set_selection(Selection {
                        anchor: start,
                        head: end,
                    })?;
                    return Ok(false);
                }
                let query = snapshot.text_range(selection.range())?;
                let results = snapshot
                    .search(
                        &query,
                        &SearchOptions {
                            case_sensitive: true,
                            ..Default::default()
                        },
                    )
                    .map_err(|e| EditorError::InvalidCommand(e.to_string()))?;
                let mut selections: Vec<_> = self.selections().collect();
                if all {
                    selections = results
                        .matches
                        .into_iter()
                        .map(|r| Selection {
                            anchor: r.start,
                            head: r.end,
                        })
                        .collect();
                } else {
                    let after = selection.range().end;
                    let found = results
                        .matches
                        .iter()
                        .filter(|r| r.start >= after)
                        .chain(results.matches.iter().filter(|r| r.start < after))
                        .find(|r| !selections.iter().any(|s| s.range() == **r));
                    if let Some(r) = found {
                        selections.insert(
                            0,
                            Selection {
                                anchor: r.start,
                                head: r.end,
                            },
                        );
                    }
                }
                self.set_selections(selections)?;
                Ok(false)
            }
            EditorCommand::InsertText(text) => self.insert(&text, EditKind::Paste, timestamp),
            EditorCommand::InsertSnippet(source) => self.insert_snippet(&source, timestamp),
            EditorCommand::NextSnippet { backward } => self.next_snippet(backward),
            EditorCommand::Indent { reverse, style } => {
                self.indent_with_style(reverse, style, timestamp)
            }
            EditorCommand::ToggleLineComment { prefix, suffix } => {
                self.toggle_comment(&prefix, &suffix, timestamp)
            }
            EditorCommand::DuplicateLines
            | EditorCommand::MoveLines { .. }
            | EditorCommand::DeleteLines => self.line_command(&command, timestamp),
            EditorCommand::SetLineEnding(ending) => {
                let snapshot = self.snapshot();
                let mut edits = Vec::new();
                for row in 0..snapshot.line_count() {
                    let range = snapshot.line_range(row)?;
                    let full = snapshot.line(row)?;
                    if full.len_chars() > range.len() {
                        let terminator = full.slice(range.len()..).to_string();
                        if terminator != ending.as_str() {
                            edits.push(TextEdit {
                                range: range.end..range.start + full.len_chars(),
                                replacement: ending.as_str().into(),
                            });
                        }
                    }
                }
                let changed =
                    self.apply_preserving_selections(&edits, EditKind::Replace, timestamp)?;
                self.line_ending = ending;
                if changed {
                    if let Some(entry) = self.undo.back_mut() {
                        entry.after_line_ending = ending;
                    }
                }
                Ok(changed)
            }
            EditorCommand::Undo => self.undo(),
            EditorCommand::Redo => self.redo(),
            EditorCommand::ReplaceAll {
                query,
                replacement,
                options,
            } => self
                .replace_all(&query, &replacement, &options, timestamp)
                .map(|n| n > 0)
                .map_err(|e| EditorError::InvalidCommand(e.to_string())),
            EditorCommand::ReplaceMatch {
                query,
                replacement,
                options,
                range,
            } => self
                .replace_match(&query, &replacement, &options, range, timestamp)
                .map(|n| n > 0)
                .map_err(|e| EditorError::InvalidCommand(e.to_string())),
        }
    }
    /// True while a snippet with tab stops is being filled in.
    pub fn has_snippet(&self) -> bool {
        self.snippet.is_some()
    }
    /// End the snippet session, leaving text and selections as they are.
    pub fn cancel_snippet(&mut self) {
        self.snippet = None;
    }
    pub(crate) fn remap_snippet(&mut self, edits: &[TextEdit]) {
        if let Some(s) = self.snippet.as_mut() {
            for group in &mut s.groups {
                for range in group {
                    let owns = edits
                        .iter()
                        .any(|e| e.range.start >= range.start && e.range.end <= range.end);
                    range.start = map_offset(range.start, edits, !owns);
                    range.end = map_offset(range.end, edits, true);
                }
            }
            s.end = map_offset(s.end, edits, true);
        }
    }
    /// Select the next tab stop of the active snippet, or the previous one when `backward`. Past
    /// the last stop the caret moves to the snippet's end and the session ends. Always returns
    /// `Ok(false)`, because the text is not changed.
    pub fn next_snippet(&mut self, backward: bool) -> Result<bool, EditorError> {
        let Some(mut snippet) = self.snippet.take() else {
            return Ok(false);
        };
        if backward {
            snippet.current = snippet.current.saturating_sub(1);
        } else {
            snippet.current += 1;
        }
        if snippet.current >= snippet.groups.len() {
            self.set_selection(Selection::caret(snippet.end))?;
            return Ok(false);
        }
        let selections = snippet.groups[snippet.current]
            .iter()
            .map(|r| Selection {
                anchor: r.start,
                head: r.end,
            })
            .collect();
        self.set_selections(selections)?;
        self.snippet = Some(snippet);
        Ok(false)
    }
    /// Insert a snippet at the selection and select its first tab stop. Tab stops are written
    /// `$1`, `${1}` or `${1:default}`, `$0` marks where the caret ends up and a backslash
    /// escapes the next character; stops with the same number are edited together. Fails with
    /// more than one selection, a source over 64 KiB or any other `$` form.
    pub fn insert_snippet(&mut self, source: &str, timestamp: u64) -> Result<bool, EditorError> {
        if source.len() > 64 * 1024 {
            return Err(EditorError::InvalidCommand("Snippet exceeds 64 KiB".into()));
        }
        if !self.secondary.is_empty() {
            return Err(EditorError::InvalidCommand(
                "Insert a snippet at one cursor; linked tabstops create their own cursors".into(),
            ));
        }
        let (text, groups, end) = parse_snippet(source)?;
        let start = self.selection.range().start;
        let changed = self.insert(&text, EditKind::Replace, timestamp)?;
        let groups: Vec<Vec<Range<usize>>> = groups
            .into_values()
            .map(|ranges| {
                ranges
                    .into_iter()
                    .map(|r| start + r.start..start + r.end)
                    .collect()
            })
            .collect();
        if let Some(first) = groups.first() {
            self.set_selections(
                first
                    .iter()
                    .map(|r| Selection {
                        anchor: r.start,
                        head: r.end,
                    })
                    .collect(),
            )?;
            self.snippet = Some(SnippetSession {
                groups,
                current: 0,
                end: start + end,
            });
        } else {
            self.set_selection(Selection::caret(start + end))?;
        }
        // Selection navigation is part of the insertion's undo state.
        if changed {
            if let Some(entry) = self.undo.back_mut() {
                entry.after_selection = self.selection;
                entry.after_secondary = self.secondary.clone();
            }
        }
        Ok(changed)
    }
}
type ParsedSnippet = (String, BTreeMap<u32, Vec<Range<usize>>>, usize);

fn parse_snippet(source: &str) -> Result<ParsedSnippet, EditorError> {
    enum Token {
        Text(String),
        Stop(u32, Option<String>),
    }
    let mut tokens = Vec::new();
    let mut chars = source.chars().peekable();
    let mut literal = String::new();
    let mut defaults = BTreeMap::new();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(c) = chars.next() {
                literal.push(c);
            } else {
                literal.push('\\');
            }
            continue;
        }
        if c != '$' {
            literal.push(c);
            continue;
        }
        let braces = chars.peek() == Some(&'{');
        if braces {
            chars.next();
        }
        let mut digits = String::new();
        while chars.peek().is_some_and(|c| c.is_ascii_digit()) {
            digits.push(chars.next().unwrap());
        }
        if digits.is_empty() {
            return Err(EditorError::InvalidCommand(
                "Snippets support numbered tabstops and literal text only".into(),
            ));
        }
        let id = digits
            .parse::<u32>()
            .map_err(|_| EditorError::InvalidCommand("Invalid snippet tabstop".into()))?;
        let default = if braces && chars.peek() == Some(&':') {
            chars.next();
            let mut text = String::new();
            loop {
                match chars.next() {
                    Some('}') => break,
                    Some('\\') => {
                        if let Some(c) = chars.next() {
                            text.push(c);
                        }
                    }
                    Some(c) => text.push(c),
                    None => {
                        return Err(EditorError::InvalidCommand(
                            "Unclosed snippet tabstop".into(),
                        ));
                    }
                }
            }
            Some(text)
        } else {
            if braces && chars.next() != Some('}') {
                return Err(EditorError::InvalidCommand(
                    "Unclosed snippet tabstop".into(),
                ));
            }
            None
        };
        if let Some(value) = &default {
            defaults.entry(id).or_insert_with(|| value.clone());
        }
        tokens.push(Token::Text(std::mem::take(&mut literal)));
        tokens.push(Token::Stop(id, default));
    }
    tokens.push(Token::Text(literal));
    let mut text = String::new();
    let mut offset = 0;
    let mut groups: BTreeMap<u32, Vec<Range<usize>>> = BTreeMap::new();
    let mut end = None;
    for token in tokens {
        match token {
            Token::Text(value) => {
                offset += value.chars().count();
                text.push_str(&value);
            }
            Token::Stop(id, _default) => {
                let value = defaults.get(&id).map(String::as_str).unwrap_or("");
                let start = offset;
                offset += value.chars().count();
                text.push_str(value);
                if id == 0 {
                    end = Some(start);
                } else {
                    groups.entry(id).or_default().push(start..offset);
                }
            }
        }
    }
    if groups.values().map(Vec::len).sum::<usize>() > CURSOR_LIMIT {
        return Err(EditorError::InvalidCommand(
            "Too many snippet tabstops".into(),
        ));
    }
    Ok((text, groups, end.unwrap_or(offset)))
}
