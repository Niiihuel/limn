use crate::{BufferSnapshot, EditorError, Position};
use ropey::RopeSlice;
use std::ops::Range;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const MAX_VIEWPORT_ROWS: usize = 1024;
const MAX_WINDOW_COLUMNS: usize = 8192;
const MAX_WINDOW_BYTES: usize = 64 * 1024;

/// The physical lines a view asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    /// Zero-based first visible line. Past the last line, the result is empty or only overscan.
    pub first_row: usize,
    /// Number of visible lines, from `first_row` down.
    pub row_count: usize,
    /// Extra rows included before and after the `row_count` visible ones.
    pub overscan: usize,
}

/// One line of a viewport: its row and its characters, without the line terminator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleLine {
    /// Zero-based line of the document.
    pub row: usize,
    /// Character offsets of the line's content in the snapshot.
    pub char_range: Range<usize>,
}

/// A bounded visual window. Tabs are expanded, graphemes stay intact unless
/// one exceeds the byte budget (then the text is clipped and `truncated` is set).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayLine {
    /// The text to paint, with each tab replaced by the spaces that reach its tab stop.
    pub text: String,
    /// Character offsets in the snapshot of the graphemes in `text`. Empty at the end of the
    /// line when the window shows nothing.
    pub char_range: Range<usize>,
    /// Display cell of the first grapheme in `text`, from the start of the line. It is less
    /// than the requested first column when a wide grapheme or a tab straddles it.
    pub start_column: usize,
    /// Display cell just past the last grapheme in `text`.
    pub end_column: usize,
    /// True when the line continues after `char_range`, because the window ended or a
    /// grapheme went over the 64 KiB text budget.
    pub truncated: bool,
}

impl BufferSnapshot {
    /// The lines of `viewport` with its overscan, clipped to the document. Fails with
    /// `ViewportTooLarge` when `row_count` plus twice the overscan exceeds 1024.
    pub fn visible_lines(&self, viewport: Viewport) -> Result<Vec<VisibleLine>, EditorError> {
        let count = viewport
            .row_count
            .checked_add(viewport.overscan.saturating_mul(2))
            .ok_or(EditorError::ViewportTooLarge)?;
        if count > MAX_VIEWPORT_ROWS {
            return Err(EditorError::ViewportTooLarge);
        }
        let first = viewport
            .first_row
            .min(self.line_count())
            .saturating_sub(viewport.overscan);
        let end = viewport
            .first_row
            .saturating_add(viewport.row_count)
            .saturating_add(viewport.overscan)
            .min(self.line_count());
        (first..end)
            .map(|row| {
                Ok(VisibleLine {
                    row,
                    char_range: self.line_range(row)?,
                })
            })
            .collect()
    }

    /// Visual column of `offset` on its line, in display cells: a tab advances to the next
    /// multiple of `tab_width` and a wide grapheme counts its display width. `tab_width` must be
    /// within 1..=32.
    pub fn display_column(&self, offset: usize, tab_width: usize) -> Result<usize, EditorError> {
        validate_tab_width(tab_width)?;
        let position = self.position_of(offset)?;
        let range = self.line_range(position.row)?;
        let end = offset.min(range.end);
        let mut column = 0usize;
        let mut current = range.start;
        while current < end {
            let next = self.next_grapheme(current)?.min(range.end);
            column = column.saturating_add(grapheme_width(
                self.rope.slice(current..next),
                column,
                tab_width,
            ));
            current = next;
        }
        Ok(column)
    }

    /// Closest grapheme boundary to a visual column; ties choose its start.
    pub fn offset_for_display_column(
        &self,
        row: usize,
        desired: usize,
        tab_width: usize,
    ) -> Result<usize, EditorError> {
        validate_tab_width(tab_width)?;
        let range = self.line_range(row)?;
        let mut current = range.start;
        let mut column = 0usize;
        while current < range.end {
            let next = self.next_grapheme(current)?.min(range.end);
            let width = grapheme_width(self.rope.slice(current..next), column, tab_width);
            let end = column.saturating_add(width);
            if desired <= end {
                return Ok(
                    if desired.saturating_sub(column) > end.saturating_sub(desired) {
                        next
                    } else {
                        current
                    },
                );
            }
            column = end;
            current = next;
        }
        Ok(range.end)
    }

    /// Physical row/column, clipped to a line's content and snapped to a grapheme.
    pub fn clipped_offset_of(&self, position: Position) -> Result<usize, EditorError> {
        let row = position.row.min(self.line_count() - 1);
        let range = self.line_range(row)?;
        self.snap_grapheme_backward(range.start + position.column.min(range.len()))
    }

    /// Expands only the requested horizontal window, preserving source text in
    /// the snapshot. No full document or full line String is needed for painting.
    pub fn display_line_window(
        &self,
        row: usize,
        first_column: usize,
        columns: usize,
        tab_width: usize,
    ) -> Result<DisplayLine, EditorError> {
        validate_tab_width(tab_width)?;
        if columns > MAX_WINDOW_COLUMNS {
            return Err(EditorError::ViewportTooLarge);
        }
        let range = self.line_range(row)?;
        let last_column = first_column.saturating_add(columns);
        let mut current = range.start;
        let mut column = 0usize;
        let mut result = DisplayLine {
            text: String::new(),
            char_range: range.end..range.end,
            start_column: 0,
            end_column: 0,
            truncated: false,
        };
        if columns == 0 {
            return Ok(result);
        }
        while current < range.end {
            let next = self.next_grapheme(current)?.min(range.end);
            let grapheme = self.rope.slice(current..next);
            let width = grapheme_width(grapheme, column, tab_width);
            let end_column = column.saturating_add(width);
            if end_column > first_column || width == 0 && column >= first_column {
                if column >= last_column {
                    break;
                }
                if result.text.is_empty() {
                    result.char_range.start = current;
                    result.start_column = column;
                }
                if grapheme == "\t" {
                    result.text.extend(std::iter::repeat_n(' ', width));
                } else {
                    for character in grapheme.chars() {
                        if result.text.len() + character.len_utf8() > MAX_WINDOW_BYTES {
                            result.truncated = true;
                            break;
                        }
                        result.text.push(character);
                    }
                }
                result.char_range.end = next;
                result.end_column = end_column;
                if result.truncated {
                    break;
                }
            }
            current = next;
            column = end_column;
        }
        if result.text.is_empty() {
            result.start_column = column;
            result.end_column = column;
        }
        result.truncated |= result.char_range.end < range.end;
        Ok(result)
    }
}

fn validate_tab_width(width: usize) -> Result<(), EditorError> {
    if (1..=32).contains(&width) {
        Ok(())
    } else {
        Err(EditorError::InvalidTabWidth)
    }
}

fn grapheme_width(grapheme: RopeSlice<'_>, column: usize, tab_width: usize) -> usize {
    if grapheme == "\t" {
        return tab_width - column % tab_width;
    }
    if let Some(text) = grapheme.as_str() {
        UnicodeWidthStr::width(text)
    }
    // A maliciously huge combining/ZWJ cluster must not allocate a whole line
    // to calculate its paint width. The bounded window flags clipped output.
    else if grapheme.len_bytes() > MAX_WINDOW_BYTES {
        grapheme
            .chars()
            .filter_map(UnicodeWidthChar::width)
            .max()
            .unwrap_or(0)
    } else {
        UnicodeWidthStr::width(grapheme.to_string().as_str())
    }
}
