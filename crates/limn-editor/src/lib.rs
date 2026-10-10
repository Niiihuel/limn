//! Independent, renderer-neutral editor core.
//!
//! Offsets and columns are Unicode scalar (Rust `char`) indices, never UTF-8
//! byte offsets. Interactive movement and deletion use extended graphemes.
//! Snapshots share Ropey storage and can be read or saved off the UI thread.

mod commands;
mod display;
mod editing;
mod grapheme;
mod row_map;
mod search;
mod visual_map;

use ropey::{Rope, RopeSlice};
use std::collections::VecDeque;
use std::fmt;
use std::io::{self, Write};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub use display::{DisplayLine, Viewport, VisibleLine};
pub use editing::{EditorCommand, IndentStyle};
pub use row_map::{changed_rows, remap_rows};
pub use search::{SearchError, SearchOptions, SearchResults};
pub use visual_map::{VisualMap, VisualRow};

pub type DocumentId = u64;
static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);
const GROUP_INTERVAL_MS: u64 = 500;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    pub const fn caret(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    pub fn range(self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Backspace,
    Delete,
    Paste,
    Replace,
    Indent,
    Newline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorLimits {
    pub max_document_bytes: usize,
    /// Changed text plus a conservative per-edit storage charge, not allocator RSS.
    pub max_history_bytes: usize,
    pub max_history_transactions: usize,
    pub max_grouped_edits: usize,
}

impl Default for EditorLimits {
    fn default() -> Self {
        Self {
            max_document_bytes: 8 * 1024 * 1024,
            max_history_bytes: 32 * 1024 * 1024,
            max_history_transactions: 512,
            max_grouped_edits: 64,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorError {
    InvalidLimits,
    InvalidRange {
        start: usize,
        end: usize,
        char_count: usize,
    },
    InvalidSelection {
        selection: Selection,
        char_count: usize,
    },
    InvalidLine {
        row: usize,
        line_count: usize,
    },
    InvalidPosition {
        position: Position,
    },
    DocumentTooLarge {
        bytes: usize,
        limit: usize,
    },
    OverlappingEdits,
    TransactionTooLarge {
        edits: usize,
        limit: usize,
    },
    RevisionExhausted,
    WrongDocument {
        expected: DocumentId,
        actual: DocumentId,
    },
    InvalidTabWidth,
    ViewportTooLarge,
    InvalidGraphemeBoundary,
    InvalidCommand(String),
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimits => write!(
                f,
                "editor limits must permit a document and at least one grouped edit"
            ),
            Self::InvalidRange {
                start,
                end,
                char_count,
            } => write!(
                f,
                "character range {start}..{end} is outside document length {char_count}"
            ),
            Self::InvalidSelection {
                selection,
                char_count,
            } => write!(
                f,
                "selection {selection:?} is outside document length {char_count}"
            ),
            Self::InvalidLine { row, line_count } => {
                write!(f, "line {row} is outside document line count {line_count}")
            }
            Self::InvalidPosition { position } => {
                write!(f, "position {position:?} is outside line contents")
            }
            Self::DocumentTooLarge { bytes, limit } => {
                write!(f, "document contains {bytes} bytes; limit is {limit}")
            }
            Self::OverlappingEdits => {
                write!(f, "transaction edits overlap or insert at the same offset")
            }
            Self::TransactionTooLarge { edits, limit } => {
                write!(f, "transaction contains {edits} edits; limit is {limit}")
            }
            Self::RevisionExhausted => write!(f, "document revision counter exhausted"),
            Self::WrongDocument { expected, actual } => write!(
                f,
                "snapshot belongs to document {actual}, expected {expected}"
            ),
            Self::InvalidTabWidth => write!(f, "tab width must be between 1 and 32 columns"),
            Self::ViewportTooLarge => {
                write!(f, "viewport exceeds its bounded row or column budget")
            }
            Self::InvalidGraphemeBoundary => {
                write!(f, "grapheme cursor requested an inconsistent rope chunk")
            }
            Self::InvalidCommand(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for EditorError {}

/// All ranges refer to the pre-transaction document, and must not overlap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    before: Rope,
    after: Rope,
    before_selection: Selection,
    after_selection: Selection,
    before_secondary: Arc<[Selection]>,
    after_secondary: Arc<[Selection]>,
    before_line_ending: LineEnding,
    after_line_ending: LineEnding,
    before_state: u64,
    after_state: u64,
    kind: EditKind,
    timestamp_ms: u64,
    charge: usize,
    edits: usize,
    groupable: bool,
}

/// Host-owned buffer. A controller may put this behind an `Arc<Mutex<_>>`;
/// take a snapshot under that lock, then render or save without the lock.
#[derive(Debug)]
pub struct TextBuffer {
    rope: Rope,
    document_id: DocumentId,
    revision: u64,
    state: u64,
    saved_state: u64,
    saved_revision: u64,
    externally_unsaved: bool,
    selection: Selection,
    secondary: Arc<[Selection]>,
    snippet: Option<editing::SnippetSession>,
    line_ending: LineEnding,
    limits: EditorLimits,
    undo: VecDeque<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    group_open: bool,
}

impl TextBuffer {
    pub fn new(text: &str) -> Result<Self, EditorError> {
        Self::with_limits(text, EditorLimits::default())
    }

    pub fn with_limits(text: &str, limits: EditorLimits) -> Result<Self, EditorError> {
        if limits.max_document_bytes == 0 || limits.max_grouped_edits == 0 {
            return Err(EditorError::InvalidLimits);
        }
        if text.len() > limits.max_document_bytes {
            return Err(EditorError::DocumentTooLarge {
                bytes: text.len(),
                limit: limits.max_document_bytes,
            });
        }
        Ok(Self {
            rope: Rope::from_str(text),
            document_id: NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            state: 0,
            saved_state: 0,
            saved_revision: 0,
            externally_unsaved: false,
            selection: Selection::default(),
            secondary: Arc::from([]),
            snippet: None,
            line_ending: detect_line_ending(text),
            limits,
            undo: VecDeque::new(),
            redo: Vec::new(),
            group_open: false,
        })
    }

    /// Recover an unsaved draft without claiming that its contents are on disk.
    pub fn restore_unsaved(
        text: &str,
        selection: Selection,
        limits: EditorLimits,
    ) -> Result<Self, EditorError> {
        let mut buffer = Self::with_limits(text, limits)?;
        buffer.set_selection(selection)?;
        buffer.state = 1;
        buffer.revision = 1;
        Ok(buffer)
    }

    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn saved_revision(&self) -> u64 {
        self.saved_revision
    }
    pub fn is_dirty(&self) -> bool {
        self.externally_unsaved || self.state != self.saved_state
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn limits(&self) -> EditorLimits {
        self.limits
    }
    pub fn preferred_line_ending(&self) -> LineEnding {
        self.line_ending
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn history_transaction_count(&self) -> usize {
        self.undo.len() + self.redo.len()
    }

    /// Applied transactions, excluding redo history. Modal editing uses this
    /// boundary to undo a change command and its inserted text as one gesture.
    pub fn undo_transaction_count(&self) -> usize {
        self.undo.len()
    }
    /// Version of the newest undo transaction. Every edit, including a
    /// coalesced edit, exposes a fresh state; eviction never recycles a value.
    pub fn undo_transaction_id(&self) -> Option<u64> {
        self.undo.back().map(|entry| entry.after_state)
    }
    pub fn history_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(self.redo.iter())
            .fold(0usize, |total, entry| total.saturating_add(entry.charge))
    }

    pub fn snapshot(&self) -> BufferSnapshot {
        BufferSnapshot {
            rope: self.rope.clone(),
            document_id: self.document_id,
            revision: self.revision,
            state: self.state,
            saved_state: self.saved_state,
            externally_unsaved: self.externally_unsaved,
            selection: self.selection,
            secondary: self.secondary.clone(),
            line_ending: self.line_ending,
        }
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditorError> {
        validate_selection(selection, self.rope.len_chars())?;
        if selection != self.selection || !self.secondary.is_empty() {
            self.break_undo_group();
        }
        self.selection = selection;
        self.secondary = Arc::from([]);
        Ok(())
    }

    pub fn break_undo_group(&mut self) {
        self.group_open = false;
    }

    pub fn mark_saved(&mut self) {
        self.externally_unsaved = false;
        self.saved_state = self.state;
        self.saved_revision = self.revision;
        self.break_undo_group();
    }

    /// A completed asynchronous save acknowledges exactly the snapshot written.
    /// Later edits stay dirty. Undoing to that snapshot's state becomes clean.
    pub fn mark_saved_snapshot(&mut self, snapshot: &BufferSnapshot) -> Result<(), EditorError> {
        if self.document_id != snapshot.document_id {
            return Err(EditorError::WrongDocument {
                expected: self.document_id,
                actual: snapshot.document_id,
            });
        }
        self.externally_unsaved = false;
        self.saved_state = snapshot.state;
        self.saved_revision = snapshot.revision;
        self.break_undo_group();
        Ok(())
    }

    /// Disk deletion invalidates the saved state without rebuilding the rope,
    /// losing cursor/history, or consuming an edit revision.
    pub fn mark_unsaved(&mut self) {
        self.externally_unsaved = true;
        self.break_undo_group();
    }

    /// One edit. The explicit resulting selection is validated before mutation.
    /// Scalar-index edits can split graphemes; interactive navigation/deletion cannot.
    pub fn edit(
        &mut self,
        range: Range<usize>,
        replacement: &str,
        selection_after: Selection,
        kind: EditKind,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        validate_range(&range, self.rope.len_chars())?;
        let after_chars =
            self.rope.len_chars() - (range.end - range.start) + replacement.chars().count();
        validate_selection(selection_after, after_chars)?;
        let removed_bytes = self.rope.slice(range.clone()).len_bytes();
        let bytes = (self.rope.len_bytes() - removed_bytes)
            .checked_add(replacement.len())
            .ok_or(EditorError::DocumentTooLarge {
                bytes: usize::MAX,
                limit: self.limits.max_document_bytes,
            })?;
        self.validate_size(bytes)?;
        if self.rope.slice(range.clone()) == replacement {
            self.set_selection(selection_after)?;
            return Ok(false);
        }
        let next_revision = self.next_revision()?;
        let before = self.rope.clone();
        let before_selection = self.selection;
        let before_secondary = self.secondary.clone();
        let before_state = self.state;
        let groupable = self.secondary.is_empty()
            && is_groupable(
                kind,
                &range,
                replacement,
                before_selection,
                selection_after,
                &self.rope,
            );
        let charge = removed_bytes
            .saturating_add(replacement.len())
            .saturating_add(2048);
        self.rope.remove(range.clone());
        self.rope.insert(range.start, replacement);
        self.remap_snippet(&[TextEdit {
            range,
            replacement: replacement.into(),
        }]);
        self.selection = selection_after;
        self.secondary = Arc::from([]);
        self.state = next_revision;
        self.revision = next_revision;
        self.record_history(HistoryEntry {
            before,
            after: self.rope.clone(),
            before_selection,
            after_selection: selection_after,
            before_secondary,
            after_secondary: self.secondary.clone(),
            before_line_ending: self.line_ending,
            after_line_ending: self.line_ending,
            before_state,
            after_state: self.state,
            kind,
            timestamp_ms,
            charge,
            edits: 1,
            groupable,
        });
        Ok(true)
    }

    pub fn insert(
        &mut self,
        text: &str,
        kind: EditKind,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        if !self.secondary.is_empty() {
            return self.insert_at_selections(text, kind, timestamp_ms);
        }
        let range = self.selection.range();
        let offset = range.start + text.chars().count();
        self.edit(range, text, Selection::caret(offset), kind, timestamp_ms)
    }

    pub fn replace_selection(
        &mut self,
        text: &str,
        kind: EditKind,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        self.insert(text, kind, timestamp_ms)
    }

    pub fn delete_backward(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        if !self.secondary.is_empty() {
            return self.delete_at_selections(false, false, timestamp_ms);
        }
        let mut range = self.selection.range();
        let snapshot = self.snapshot();
        if range.is_empty() {
            range.end = snapshot.snap_grapheme_forward(range.end)?;
            range.start = snapshot.prev_grapheme(range.end)?;
        } else {
            range.start = snapshot.snap_grapheme_backward(range.start)?;
            range.end = snapshot.snap_grapheme_forward(range.end)?;
        }
        self.edit(
            range.clone(),
            "",
            Selection::caret(range.start),
            EditKind::Backspace,
            timestamp_ms,
        )
    }

    pub fn delete_forward(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        if !self.secondary.is_empty() {
            return self.delete_at_selections(true, false, timestamp_ms);
        }
        let mut range = self.selection.range();
        let snapshot = self.snapshot();
        if range.is_empty() {
            range.start = snapshot.snap_grapheme_backward(range.start)?;
            range.end = snapshot.next_grapheme(range.start)?;
        } else {
            range.start = snapshot.snap_grapheme_backward(range.start)?;
            range.end = snapshot.snap_grapheme_forward(range.end)?;
        }
        self.edit(
            range.clone(),
            "",
            Selection::caret(range.start),
            EditKind::Delete,
            timestamp_ms,
        )
    }

    /// Uses the document's first LF/CRLF/CR convention and copies leading indent
    /// up to the insertion column. Existing and pasted line endings are untouched.
    pub fn insert_newline(&mut self, timestamp_ms: u64) -> Result<bool, EditorError> {
        if !self.secondary.is_empty() {
            return self.newline_at_selections(timestamp_ms);
        }
        let snapshot = self.snapshot();
        let start = self.selection.range().start;
        let row = snapshot.position_of(start)?.row;
        let range = snapshot.line_range(row)?;
        let indent: String = snapshot
            .rope
            .slice(range.start..start.min(range.end))
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .collect();
        let text = format!("{}{indent}", self.line_ending.as_str());
        self.insert(&text, EditKind::Newline, timestamp_ms)
    }

    /// Atomic multi-edit transaction. Ranges address the original snapshot.
    /// Validation completes before any mutation; descending application preserves offsets.
    pub fn apply_transaction(
        &mut self,
        edits: &[TextEdit],
        selection_after: Selection,
        kind: EditKind,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        self.apply_transaction_selections(edits, vec![selection_after], kind, timestamp_ms)
    }

    /// Atomic edits and the complete resulting directed selection set.
    pub fn apply_transaction_selections(
        &mut self,
        edits: &[TextEdit],
        selections_after: Vec<Selection>,
        kind: EditKind,
        timestamp_ms: u64,
    ) -> Result<bool, EditorError> {
        let (sorted, selections_after, charge) =
            self.prepare_transaction(edits, selections_after)?;
        let selection_after = selections_after[0];
        if sorted
            .iter()
            .all(|edit| self.rope.slice(edit.range.clone()) == edit.replacement.as_str())
        {
            self.set_selections(selections_after)?;
            return Ok(false);
        }
        let next_revision = self.next_revision()?;
        let before = self.rope.clone();
        let before_selection = self.selection;
        let before_secondary = self.secondary.clone();
        let before_state = self.state;
        let mut after = self.rope.clone();
        for edit in sorted.iter().rev() {
            after.remove(edit.range.clone());
            after.insert(edit.range.start, &edit.replacement);
        }
        self.rope = after;
        self.remap_snippet(edits);
        self.selection = selection_after;
        self.secondary = selections_after[1..].to_vec().into();
        self.state = next_revision;
        self.revision = next_revision;
        self.record_history(HistoryEntry {
            before,
            after: self.rope.clone(),
            before_selection,
            after_selection: selection_after,
            before_secondary,
            after_secondary: self.secondary.clone(),
            before_line_ending: self.line_ending,
            after_line_ending: self.line_ending,
            before_state,
            after_state: self.state,
            kind,
            timestamp_ms,
            charge,
            edits: sorted.len(),
            groupable: false,
        });
        Ok(true)
    }

    /// Validate against this buffer's real limits while retaining its lock.
    /// Workspace transactions can preflight every target before any mutation.
    pub fn validate_transaction(
        &self,
        edits: &[TextEdit],
        selection_after: Selection,
    ) -> Result<(), EditorError> {
        self.prepare_transaction(edits, vec![selection_after])
            .map(|_| ())
    }

    fn prepare_transaction<'a>(
        &self,
        edits: &'a [TextEdit],
        selections_after: Vec<Selection>,
    ) -> Result<(Vec<&'a TextEdit>, Vec<Selection>, usize), EditorError> {
        const MAX_TRANSACTION_EDITS: usize = 4096;
        if edits.len() > MAX_TRANSACTION_EDITS {
            return Err(EditorError::TransactionTooLarge {
                edits: edits.len(),
                limit: MAX_TRANSACTION_EDITS,
            });
        }
        let mut sorted: Vec<&TextEdit> = edits.iter().collect();
        sorted.sort_by_key(|edit| (edit.range.start, edit.range.end));
        let mut bytes = self.rope.len_bytes();
        let mut chars = self.rope.len_chars();
        let mut charge = 0usize;
        let mut previous: Option<&TextEdit> = None;
        for edit in &sorted {
            validate_range(&edit.range, self.rope.len_chars())?;
            if let Some(previous) = previous {
                if previous.range.end > edit.range.start || previous.range.start == edit.range.start
                {
                    return Err(EditorError::OverlappingEdits);
                }
            }
            let removed = self.rope.slice(edit.range.clone());
            bytes = bytes
                .checked_sub(removed.len_bytes())
                .and_then(|value| value.checked_add(edit.replacement.len()))
                .ok_or(EditorError::DocumentTooLarge {
                    bytes: usize::MAX,
                    limit: self.limits.max_document_bytes,
                })?;
            chars = chars - removed.len_chars() + edit.replacement.chars().count();
            charge = charge
                .saturating_add(removed.len_bytes())
                .saturating_add(edit.replacement.len())
                .saturating_add(2048);
            previous = Some(edit);
        }
        let selections_after = editing::normalize_selections(selections_after, chars)?;
        self.validate_size(bytes)?;
        if sorted
            .iter()
            .any(|edit| self.rope.slice(edit.range.clone()) != edit.replacement.as_str())
        {
            self.next_revision()?;
        }
        Ok((sorted, selections_after, charge))
    }

    pub fn undo(&mut self) -> Result<bool, EditorError> {
        if self.undo.is_empty() {
            return Ok(false);
        }
        let revision = self.next_revision()?;
        let entry = self.undo.pop_back().expect("checked history");
        self.rope = entry.before.clone();
        self.snippet = None;
        self.selection = entry.before_selection;
        self.secondary = entry.before_secondary.clone();
        self.line_ending = entry.before_line_ending;
        self.state = entry.before_state;
        self.revision = revision;
        self.redo.push(entry);
        self.break_undo_group();
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditorError> {
        if self.redo.is_empty() {
            return Ok(false);
        }
        let revision = self.next_revision()?;
        let entry = self.redo.pop().expect("checked history");
        self.rope = entry.after.clone();
        self.snippet = None;
        self.selection = entry.after_selection;
        self.secondary = entry.after_secondary.clone();
        self.line_ending = entry.after_line_ending;
        self.state = entry.after_state;
        self.revision = revision;
        self.undo.push_back(entry);
        self.break_undo_group();
        Ok(true)
    }

    fn validate_size(&self, bytes: usize) -> Result<(), EditorError> {
        if bytes > self.limits.max_document_bytes {
            Err(EditorError::DocumentTooLarge {
                bytes,
                limit: self.limits.max_document_bytes,
            })
        } else {
            Ok(())
        }
    }

    fn next_revision(&self) -> Result<u64, EditorError> {
        self.revision
            .checked_add(1)
            .ok_or(EditorError::RevisionExhausted)
    }

    fn record_history(&mut self, entry: HistoryEntry) {
        self.redo.clear();
        let merge = self.group_open
            && entry.groupable
            && self.undo.back().is_some_and(|previous| {
                previous.groupable
                    && previous.kind == entry.kind
                    && previous.after_selection == entry.before_selection
                    && entry.timestamp_ms >= previous.timestamp_ms
                    && entry.timestamp_ms - previous.timestamp_ms <= GROUP_INTERVAL_MS
                    && previous.edits < self.limits.max_grouped_edits
            });
        if merge {
            let previous = self.undo.back_mut().expect("merge requires previous entry");
            previous.after = entry.after;
            previous.after_selection = entry.after_selection;
            previous.after_secondary = entry.after_secondary;
            previous.after_line_ending = entry.after_line_ending;
            previous.after_state = entry.after_state;
            previous.timestamp_ms = entry.timestamp_ms;
            previous.charge = previous.charge.saturating_add(entry.charge);
            previous.edits += 1;
        } else {
            self.undo.push_back(entry);
        }
        while self.undo.len() > self.limits.max_history_transactions
            || self.history_bytes() > self.limits.max_history_bytes
        {
            self.undo.pop_front();
        }
        self.group_open = self.undo.back().is_some_and(|entry| entry.groupable);
    }
}

#[derive(Clone, Debug)]
pub struct BufferSnapshot {
    pub(crate) rope: Rope,
    document_id: DocumentId,
    revision: u64,
    state: u64,
    saved_state: u64,
    externally_unsaved: bool,
    selection: Selection,
    secondary: Arc<[Selection]>,
    line_ending: LineEnding,
}

impl BufferSnapshot {
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn is_dirty(&self) -> bool {
        self.externally_unsaved || self.state != self.saved_state
    }
    pub fn selection(&self) -> Selection {
        self.selection
    }
    pub fn preferred_line_ending(&self) -> LineEnding {
        self.line_ending
    }
    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }
    pub fn byte_count(&self) -> usize {
        self.rope.len_bytes()
    }
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }
    pub fn chunks(&self) -> ropey::iter::Chunks<'_> {
        self.rope.chunks()
    }
    pub fn write_to<W: Write>(&self, writer: W) -> io::Result<()> {
        self.rope.write_to(writer)
    }

    /// Borrowed line including its original terminator, without allocating.
    pub fn line(&self, row: usize) -> Result<RopeSlice<'_>, EditorError> {
        self.rope.get_line(row).ok_or(EditorError::InvalidLine {
            row,
            line_count: self.line_count(),
        })
    }

    /// Character range excluding the line terminator; trailing empty line is valid.
    pub fn line_range(&self, row: usize) -> Result<Range<usize>, EditorError> {
        let line = self.line(row)?;
        let start = self.rope.line_to_char(row);
        Ok(start..start + content_char_count(line))
    }

    pub fn line_text(&self, row: usize) -> Result<String, EditorError> {
        self.text_range(self.line_range(row)?)
    }

    pub fn text_range(&self, range: Range<usize>) -> Result<String, EditorError> {
        validate_range(&range, self.char_count())?;
        Ok(self.rope.slice(range).to_string())
    }

    pub fn slice(&self, range: Range<usize>) -> Result<RopeSlice<'_>, EditorError> {
        validate_range(&range, self.char_count())?;
        Ok(self.rope.slice(range))
    }

    pub fn char_to_byte(&self, offset: usize) -> Result<usize, EditorError> {
        validate_range(&(offset..offset), self.char_count())?;
        Ok(self.rope.char_to_byte(offset))
    }

    /// Bytes inside a scalar round down to that scalar's offset, as Ropey does.
    pub fn byte_to_char(&self, offset: usize) -> Result<usize, EditorError> {
        if offset > self.byte_count() {
            return Err(EditorError::InvalidRange {
                start: offset,
                end: offset,
                char_count: self.byte_count(),
            });
        }
        Ok(self.rope.byte_to_char(offset))
    }

    /// Offsets within a line terminator map to that line's final content column.
    /// The row containing `offset`, without measuring its column.
    pub fn row_of(&self, offset: usize) -> Result<usize, EditorError> {
        validate_range(&(offset..offset), self.char_count())?;
        Ok(self.rope.char_to_line(offset))
    }

    pub fn position_of(&self, offset: usize) -> Result<Position, EditorError> {
        validate_range(&(offset..offset), self.char_count())?;
        let row = self.rope.char_to_line(offset);
        let range = self.line_range(row)?;
        Ok(Position {
            row,
            column: offset.min(range.end) - range.start,
        })
    }

    pub fn offset_of(&self, position: Position) -> Result<usize, EditorError> {
        let range = self.line_range(position.row)?;
        if position.column > range.len() {
            return Err(EditorError::InvalidPosition { position });
        }
        Ok(range.start + position.column)
    }

    pub fn prev_grapheme(&self, offset: usize) -> Result<usize, EditorError> {
        grapheme::boundary(&self.rope, offset, false)
    }
    pub fn next_grapheme(&self, offset: usize) -> Result<usize, EditorError> {
        grapheme::boundary(&self.rope, offset, true)
    }
    pub fn is_grapheme_boundary(&self, offset: usize) -> Result<bool, EditorError> {
        grapheme::is_boundary(&self.rope, offset)
    }
    pub fn snap_grapheme_backward(&self, offset: usize) -> Result<usize, EditorError> {
        if self.is_grapheme_boundary(offset)? {
            Ok(offset)
        } else {
            self.prev_grapheme(offset)
        }
    }
    pub fn snap_grapheme_forward(&self, offset: usize) -> Result<usize, EditorError> {
        if self.is_grapheme_boundary(offset)? {
            Ok(offset)
        } else {
            self.next_grapheme(offset)
        }
    }
}

fn validate_range(range: &Range<usize>, char_count: usize) -> Result<(), EditorError> {
    if range.start > range.end || range.end > char_count {
        Err(EditorError::InvalidRange {
            start: range.start,
            end: range.end,
            char_count,
        })
    } else {
        Ok(())
    }
}

fn validate_selection(selection: Selection, char_count: usize) -> Result<(), EditorError> {
    if selection.anchor > char_count || selection.head > char_count {
        Err(EditorError::InvalidSelection {
            selection,
            char_count,
        })
    } else {
        Ok(())
    }
}

fn detect_line_ending(text: &str) -> LineEnding {
    for (index, byte) in text.bytes().enumerate() {
        match byte {
            b'\r' if text.as_bytes().get(index + 1) == Some(&b'\n') => return LineEnding::CrLf,
            b'\r' => return LineEnding::Cr,
            b'\n' => return LineEnding::Lf,
            _ => {}
        }
    }
    LineEnding::Lf
}

fn content_char_count(line: RopeSlice<'_>) -> usize {
    let count = line.len_chars();
    let Some(last) = count.checked_sub(1).and_then(|index| line.get_char(index)) else {
        return 0;
    };
    if last == '\n' && count >= 2 && line.char(count - 2) == '\r' {
        count - 2
    } else if matches!(
        last,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    ) {
        count - 1
    } else {
        count
    }
}

fn is_groupable(
    kind: EditKind,
    range: &Range<usize>,
    replacement: &str,
    before: Selection,
    after: Selection,
    rope: &Rope,
) -> bool {
    if !after.is_empty() || replacement.contains(['\n', '\r']) {
        return false;
    }
    match kind {
        EditKind::Typing => {
            // Replacing a selection starts the same continuous typing gesture
            // as its following characters. It cannot merge backwards because
            // the previous grouped edit ended with an empty selection.
            *range == before.range()
                && !replacement.is_empty()
                && after.head == range.start + replacement.chars().count()
        }
        EditKind::Backspace => {
            before.is_empty()
                && replacement.is_empty()
                && range.end == before.head
                && after.head == range.start
                && !rope
                    .slice(range.clone())
                    .chars()
                    .any(|character| matches!(character, '\n' | '\r'))
        }
        EditKind::Delete => {
            before.is_empty()
                && replacement.is_empty()
                && range.start == before.head
                && after.head == range.start
                && !rope
                    .slice(range.clone())
                    .chars()
                    .any(|character| matches!(character, '\n' | '\r'))
        }
        _ => false,
    }
}

#[cfg(test)]
mod advanced_tests;
#[cfg(test)]
mod tests;
