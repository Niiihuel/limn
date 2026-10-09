//! Physical-to-visual rows shared by wrap, folding, hit testing and cursor movement.
use crate::{BufferSnapshot, EditorError};
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualRow {
    pub physical_row: usize,
    pub range: Range<usize>,
    pub start_column: usize,
    pub end_column: usize,
    pub continuation: bool,
    pub folded: bool,
}
#[derive(Clone, Debug)]
pub struct VisualMap {
    pub rows: Vec<VisualRow>,
    pub document_id: u64,
    pub revision: u64,
}
impl VisualMap {
    /// Folds retain the start row and hide start+1..end. Overlaps merge naturally.
    /// Wrap columns count display cells; no wrap boundary splits a grapheme or CRLF.
    pub fn new(
        snapshot: &BufferSnapshot,
        wrap_columns: Option<usize>,
        tab_width: usize,
        folds: &[Range<usize>],
    ) -> Result<Self, EditorError> {
        Self::build_range(
            snapshot,
            wrap_columns,
            tab_width,
            folds,
            0..snapshot.line_count(),
        )
    }
    fn build_range(
        snapshot: &BufferSnapshot,
        wrap_columns: Option<usize>,
        tab_width: usize,
        folds: &[Range<usize>],
        physical_range: Range<usize>,
    ) -> Result<Self, EditorError> {
        if !(1..=32).contains(&tab_width) {
            return Err(EditorError::InvalidTabWidth);
        }
        let mut folds: Vec<_> = folds
            .iter()
            .filter(|r| r.start < r.end && r.start < snapshot.line_count())
            .cloned()
            .collect();
        folds.sort_by_key(|r| r.start);
        let mut rows = Vec::new();
        let mut physical = physical_range.start;
        let mut fold_index = 0;
        while physical < physical_range.end {
            while fold_index < folds.len() && folds[fold_index].end <= physical + 1 {
                fold_index += 1;
            }
            let mut hidden_end = physical + 1;
            if let Some(fold) = folds.get(fold_index).filter(|r| r.start == physical) {
                hidden_end = fold.end.min(snapshot.line_count());
                while fold_index + 1 < folds.len() && folds[fold_index + 1].start < hidden_end {
                    fold_index += 1;
                    hidden_end = hidden_end.max(folds[fold_index].end.min(snapshot.line_count()));
                }
            }
            let range = snapshot.line_range(physical)?;
            let first_row = rows.len();
            if let Some(columns) = wrap_columns {
                let columns = columns.max(1);
                let mut offset = range.start;
                let mut start = offset;
                let mut column = 0;
                let mut start_column = 0;
                while offset < range.end {
                    let next = snapshot.next_grapheme(offset)?.min(range.end);
                    let grapheme = snapshot.slice(offset..next)?;
                    let width = if grapheme == "\t" {
                        tab_width - column % tab_width
                    } else {
                        if let Some(text) = grapheme.as_str() {
                            UnicodeWidthStr::width(text)
                        } else if grapheme.len_bytes() > 65_536 {
                            grapheme
                                .chars()
                                .filter_map(unicode_width::UnicodeWidthChar::width)
                                .max()
                                .unwrap_or(0)
                        } else {
                            UnicodeWidthStr::width(grapheme.to_string().as_str())
                        }
                    };
                    if offset > start && column + width - start_column > columns {
                        rows.push(VisualRow {
                            physical_row: physical,
                            range: start..offset,
                            start_column,
                            end_column: column,
                            continuation: start != range.start,
                            folded: false,
                        });
                        start = offset;
                        start_column = column;
                    }
                    column += width;
                    offset = next;
                }
                rows.push(VisualRow {
                    physical_row: physical,
                    range: start..range.end,
                    start_column,
                    end_column: column,
                    continuation: start != range.start,
                    folded: false,
                });
            } else {
                rows.push(VisualRow {
                    physical_row: physical,
                    range: range.clone(),
                    start_column: 0,
                    end_column: 0,
                    continuation: false,
                    folded: false,
                });
            }
            if hidden_end > physical + 1 {
                rows[first_row].folded = true;
            }
            physical = hidden_end;
        }
        Ok(Self {
            rows,
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
        })
    }
    /// Rewrap only changed physical lines, retaining prefix/suffix layout. The
    /// rope comparison is bounded by document size and does not allocate text.
    pub fn update_unfolded(
        &self,
        previous: &BufferSnapshot,
        next: &BufferSnapshot,
        columns: usize,
        tab_width: usize,
    ) -> Result<Self, EditorError> {
        if previous.document_id() != self.document_id
            || next.document_id() != self.document_id
            || previous.revision() != self.revision
        {
            return Self::new(next, Some(columns), tab_width, &[]);
        }
        let mut prefix = 0;
        while prefix < previous.line_count().min(next.line_count())
            && previous.line(prefix)? == next.line(prefix)?
        {
            prefix += 1;
        }
        let (mut old_end, mut new_end) = (previous.line_count(), next.line_count());
        while old_end > prefix
            && new_end > prefix
            && previous.line(old_end - 1)? == next.line(new_end - 1)?
        {
            old_end -= 1;
            new_end -= 1;
        }
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .take_while(|r| r.physical_row < prefix)
            .cloned()
            .collect();
        rows.extend(Self::build_range(next, Some(columns), tab_width, &[], prefix..new_end)?.rows);
        let old_offset = if old_end < previous.line_count() {
            previous.line_range(old_end)?.start
        } else {
            previous.char_count()
        };
        let new_offset = if new_end < next.line_count() {
            next.line_range(new_end)?.start
        } else {
            next.char_count()
        };
        let char_delta = new_offset as isize - old_offset as isize;
        let row_delta = new_end as isize - old_end as isize;
        rows.extend(
            self.rows
                .iter()
                .filter(|r| r.physical_row >= old_end)
                .cloned()
                .map(|mut row| {
                    row.physical_row = row.physical_row.saturating_add_signed(row_delta);
                    row.range = row.range.start.saturating_add_signed(char_delta)
                        ..row.range.end.saturating_add_signed(char_delta);
                    row
                }),
        );
        Ok(Self {
            rows,
            document_id: next.document_id(),
            revision: next.revision(),
        })
    }
    pub fn row_for_offset(&self, offset: usize) -> usize {
        let index = self
            .rows
            .partition_point(|r| r.range.start <= offset)
            .saturating_sub(1);
        // A soft-wrap boundary belongs to the following visual row.
        index.min(self.rows.len().saturating_sub(1))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextBuffer;
    #[test]
    fn incremental_wrap_matches_full_layout_after_insert_delete_and_undo() {
        use crate::{EditKind, Selection};
        let mut buffer = TextBuffer::new("prefix\r\n東京 abcdefghijkl\r\nsuffix").unwrap();
        let mut previous = buffer.snapshot();
        let mut map = VisualMap::new(&previous, Some(6), 4, &[]).unwrap();
        for replacement in ["X\nY", "", "🧑‍💻"] {
            let range = buffer.snapshot().line_range(1).unwrap();
            buffer
                .set_selection(Selection {
                    anchor: range.start,
                    head: range.end,
                })
                .unwrap();
            buffer.insert(replacement, EditKind::Replace, 0).unwrap();
            let next = buffer.snapshot();
            map = map.update_unfolded(&previous, &next, 6, 4).unwrap();
            assert_eq!(
                map.rows,
                VisualMap::new(&next, Some(6), 4, &[]).unwrap().rows
            );
            previous = next;
        }
        buffer.undo().unwrap();
        let next = buffer.snapshot();
        let updated = map.update_unfolded(&previous, &next, 6, 4).unwrap();
        assert_eq!(
            updated.rows,
            VisualMap::new(&next, Some(6), 4, &[]).unwrap().rows
        );
    }
    #[test]
    fn wrap_fold_roundtrips_graphemes_and_physical_lines() {
        let b = TextBuffer::new("a\t東京e\u{301}🧑‍💻\r\nhidden\r\nlast").unwrap();
        let s = b.snapshot();
        let map = VisualMap::new(&s, Some(5), 4, std::slice::from_ref(&(0..2))).unwrap();
        assert!(map.rows[0].folded);
        assert!(!map.rows.iter().any(|r| r.physical_row == 1));
        assert_eq!(map.rows.last().unwrap().physical_row, 2);
        for (index, row) in map.rows.iter().enumerate() {
            assert!(s.is_grapheme_boundary(row.range.start).unwrap());
            assert!(s.is_grapheme_boundary(row.range.end).unwrap());
            assert_eq!(map.row_for_offset(row.range.start), index);
        }
        assert!(map.rows.iter().any(|r| r.continuation));
    }
}
