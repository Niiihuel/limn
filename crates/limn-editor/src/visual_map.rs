//! Physical-to-visual rows shared by wrap, folding, hit testing and cursor movement.
use crate::{BufferSnapshot, EditorError};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// One row as drawn: a whole physical line, or one wrapped segment of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualRow {
    /// Zero-based line of the document this row shows.
    pub physical_row: usize,
    /// Characters shown on this row, without the line terminator.
    pub range: Range<usize>,
    /// Display cells from the start of the physical line to the row's first cell and, in
    /// `end_column`, past its last. Both are 0 when wrapping is off.
    pub start_column: usize,
    /// Display cells from the start of the physical line to the end of this row. 0 when
    /// wrapping is off.
    pub end_column: usize,
    /// True for every wrapped segment after the first of its line.
    pub continuation: bool,
    /// True on the first row of a line whose fold hides the lines that follow it.
    pub folded: bool,
}
/// The visual rows of one snapshot, in order, after wrapping and folding.
#[derive(Clone, Debug)]
pub struct VisualMap {
    /// Every visible row, top to bottom. Lines hidden by a fold have no row.
    pub rows: Vec<VisualRow>,
    /// `document_id` of the snapshot the rows were laid out from.
    pub document_id: u64,
    /// Revision of that snapshot. `update` rebuilds the whole map when it is given another one.
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
        // Lines are read in sequence; only a fold makes the cursor jump.
        let seek = |row: usize| {
            let row = row.min(snapshot.line_count());
            (snapshot.rope.lines_at(row), snapshot.rope.line_to_char(row))
        };
        let (mut lines, mut line_start) = seek(physical);
        let mut scratch = String::new();
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
            let Some(line) = lines.next() else {
                return Err(EditorError::InvalidLine {
                    row: physical,
                    line_count: snapshot.line_count(),
                });
            };
            let range = line_start..line_start + crate::content_char_count(line);
            line_start += line.len_chars();
            let first_row = rows.len();
            if let Some(columns) = wrap_columns {
                let columns = columns.max(1);
                let mut offset = range.start;
                let mut start = offset;
                let mut column = 0;
                let mut start_column = 0;
                // A line of ordinary length is segmented as one string, copied
                // first when it straddles two pieces of the rope.
                let content = match line.as_str() {
                    Some(text) => Some(text),
                    None if line.len_bytes() <= 65_536 => {
                        scratch.clear();
                        scratch.extend(line.chunks());
                        Some(scratch.as_str())
                    }
                    None => None,
                }
                .map(|text| {
                    let end = text
                        .char_indices()
                        .nth(range.len())
                        .map_or(text.len(), |(index, _)| index);
                    &text[..end]
                });
                // In printable ASCII every byte is one grapheme and one cell.
                let ascii = content.map(str::as_bytes).filter(|bytes| {
                    bytes
                        .iter()
                        .all(|byte| *byte == b'\t' || (0x20..0x7f).contains(byte))
                });
                let mut graphemes = content.map(|text| text.graphemes(true));
                while offset < range.end {
                    let (next, width) = if let Some(bytes) = ascii {
                        let width = if bytes[offset - range.start] == b'\t' {
                            tab_width - column % tab_width
                        } else {
                            1
                        };
                        (offset + 1, width)
                    } else if let Some(grapheme) = graphemes.as_mut().and_then(Iterator::next) {
                        let width = if grapheme == "\t" {
                            tab_width - column % tab_width
                        } else {
                            UnicodeWidthStr::width(grapheme)
                        };
                        (offset + grapheme.chars().count(), width)
                    } else {
                        let next = snapshot.next_grapheme(offset)?.min(range.end);
                        let grapheme = snapshot.slice(offset..next)?;
                        let width = if grapheme == "\t" {
                            tab_width - column % tab_width
                        } else if let Some(text) = grapheme.as_str() {
                            UnicodeWidthStr::width(text)
                        } else if grapheme.len_bytes() > 65_536 {
                            grapheme
                                .chars()
                                .filter_map(unicode_width::UnicodeWidthChar::width)
                                .max()
                                .unwrap_or(0)
                        } else {
                            UnicodeWidthStr::width(grapheme.to_string().as_str())
                        };
                        (next, width)
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
                (lines, line_start) = seek(hidden_end);
            }
            physical = hidden_end;
        }
        Ok(Self {
            rows,
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
        })
    }
    /// Lays out only the changed physical lines again, keeping the rows before
    /// and after them. `old_folds` are the folds this map was built with. The
    /// whole map is rebuilt when the folds away from the changed lines are not
    /// the same ones, when a fold encloses those lines, or when the folds that
    /// start on them hide a different number of the rows that follow.
    pub fn update(
        &self,
        previous: &BufferSnapshot,
        next: &BufferSnapshot,
        wrap_columns: Option<usize>,
        tab_width: usize,
        old_folds: &[Range<usize>],
        folds: &[Range<usize>],
    ) -> Result<Self, EditorError> {
        let rebuild = || Self::new(next, wrap_columns, tab_width, folds);
        if previous.document_id() != self.document_id
            || next.document_id() != self.document_id
            || previous.revision() != self.revision
        {
            return rebuild();
        }
        let (prefix, old_end, new_end) = crate::changed_rows(previous, next)?;
        // Folds on each side of the change, as they sit in the new text, and
        // how many rows after it are hidden by folds that start inside it.
        let around = |folds: &[Range<usize>], lines: usize, end: usize, shift: usize| {
            let mut sides = Vec::new();
            let mut reach = 0;
            for fold in folds
                .iter()
                .filter(|fold| fold.start < fold.end && fold.start < lines)
            {
                if fold.end <= prefix {
                    sides.push(fold.clone());
                } else if fold.start >= end {
                    sides.push(fold.start + shift - end..fold.end.min(lines) + shift - end);
                } else if fold.start >= prefix {
                    reach = reach.max(fold.end.min(lines).saturating_sub(end));
                } else {
                    return None;
                }
            }
            sides.sort_by_key(|fold| (fold.start, fold.end));
            Some((sides, reach))
        };
        let before = around(old_folds, previous.line_count(), old_end, new_end);
        if before.is_none() || before != around(folds, next.line_count(), new_end, new_end) {
            return rebuild();
        }
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .take_while(|r| r.physical_row < prefix)
            .cloned()
            .collect();
        rows.extend(Self::build_range(next, wrap_columns, tab_width, folds, prefix..new_end)?.rows);
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
    /// Index of the visual row that shows `offset`. An offset on a wrap boundary belongs to the
    /// following row, and an offset hidden by a fold maps to the last row before it.
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
            map = map.update(&previous, &next, Some(6), 4, &[], &[]).unwrap();
            assert_eq!(
                map.rows,
                VisualMap::new(&next, Some(6), 4, &[]).unwrap().rows
            );
            previous = next;
        }
        buffer.undo().unwrap();
        let next = buffer.snapshot();
        let updated = map.update(&previous, &next, Some(6), 4, &[], &[]).unwrap();
        assert_eq!(
            updated.rows,
            VisualMap::new(&next, Some(6), 4, &[]).unwrap().rows
        );
    }
    #[test]
    fn incremental_layout_with_wrap_and_folds_matches_a_full_layout() {
        use crate::{remap_rows, EditKind, Selection};
        let text: String = (0..240)
            .map(|row| match row % 9 {
                0 => "\r\n".to_owned(),
                1 => format!("\tñandú {row} 終 wide wide wide wide\r\n"),
                2 => format!("a\tb\tc {row} plain ascii line that wraps\n"),
                3 => "ctl\u{1}x\n".to_owned(),
                _ => format!("line {row}\n"),
            })
            .collect();
        let pieces = ["", "x", "\n", "\t", "é\n", "a\nb\nc", "long long long long long"];
        let mut seed = 0x51ed_270b_u64;
        let mut random = |bound: usize| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (seed >> 33) as usize % bound.max(1)
        };
        for columns in [None, Some(12)] {
            let mut buffer = TextBuffer::new(&text).unwrap();
            let mut previous = buffer.snapshot();
            let mut folds = vec![3..9, 20..24, 22..30, 100..140, 200..206];
            let mut map = VisualMap::new(&previous, columns, 4, &folds).unwrap();
            let mut kept = 0;
            for step in 0..300 {
                let length = previous.char_count();
                let start = random(length + 1);
                let end = (start + random(3) * random(25)).min(length);
                let piece = pieces[random(pieces.len())];
                let caret = Selection::caret(start + piece.chars().count());
                buffer.edit(start..end, piece, caret, EditKind::Typing, 0).unwrap();
                let next = buffer.snapshot();
                let mut moved = remap_rows(&previous, &next, &folds).unwrap();
                if step % 40 == 39 {
                    let row = random(next.line_count());
                    moved.push(row..row + 2 + random(6));
                }
                if step % 7 == 3 {
                    let row = next.position_of(caret.head).unwrap().row;
                    moved.push(row..row + 3 + random(4));
                }
                let full = VisualMap::new(&next, columns, 4, &moved).unwrap();
                kept += usize::from(moved.len() == folds.len());
                map = map.update(&previous, &next, columns, 4, &folds, &moved).unwrap();
                assert_eq!(map.rows, full.rows, "step {step} with {columns:?}");
                (previous, folds) = (next, moved);
            }
            assert!(kept > 60);
        }
    }
    #[test]
    fn typing_on_the_first_line_of_a_folded_block_keeps_it_folded() {
        use crate::{EditKind, Selection};
        let text: String = (0..60).map(|row| format!("line {row}\n")).collect();
        let mut buffer = TextBuffer::new(&text).unwrap();
        let folds = [10..20, 15..25, 40..44];
        let mut previous = buffer.snapshot();
        let mut map = VisualMap::new(&previous, Some(8), 4, &folds).unwrap();
        for _ in 0..3 {
            let at = previous.line_range(10).unwrap().end;
            buffer
                .edit(at..at, " and more", Selection::caret(at + 9), EditKind::Typing, 0)
                .unwrap();
            let next = buffer.snapshot();
            map = map.update(&previous, &next, Some(8), 4, &folds, &folds).unwrap();
            assert_eq!(map.rows, VisualMap::new(&next, Some(8), 4, &folds).unwrap().rows);
            previous = next;
        }
        let header: Vec<_> = map.rows.iter().filter(|row| row.physical_row == 10).collect();
        assert!(header.len() > 1 && header[0].folded);
        assert!(!map.rows.iter().any(|row| (11..25).contains(&row.physical_row)));
        assert!(map.rows.iter().any(|row| row.physical_row == 25));
    }
    #[test]
    fn wrapping_matches_grapheme_by_grapheme_layout_over_the_rope() {
        let samples = [
            "plain ascii that is long enough to wrap twice at least",
            "\ttabs\tin\tthe\tmiddle and a control \u{1} byte",
            "ñandú café e\u{301}e\u{301} 東京東京東京 🧑‍💻🧑‍💻 fin de línea",
            "",
            "a\u{200d}b \u{1f1e6}\u{1f1f7}\u{1f1e6}\u{1f1f7} flags",
        ];
        let endings = ["\n", "\r\n", "\r", "\u{2028}", "\u{85}"];
        let text: String = (0..900)
            .map(|row| format!("{}{}", samples[row % samples.len()], endings[row % endings.len()]))
            .collect();
        let snapshot = TextBuffer::new(&text).unwrap().snapshot();
        for (columns, tab_width) in [(1, 4), (7, 4), (16, 8), (200, 2)] {
            let mut expected = Vec::new();
            for physical in 0..snapshot.line_count() {
                let range = snapshot.line_range(physical).unwrap();
                let (mut offset, mut start, mut column, mut start_column) =
                    (range.start, range.start, 0, 0);
                while offset < range.end {
                    let next = snapshot.next_grapheme(offset).unwrap().min(range.end);
                    let grapheme = snapshot.slice(offset..next).unwrap().to_string();
                    let width = if grapheme == "\t" {
                        tab_width - column % tab_width
                    } else {
                        UnicodeWidthStr::width(grapheme.as_str())
                    };
                    if offset > start && column + width - start_column > columns {
                        expected.push((physical, start..offset, start_column, column));
                        (start, start_column) = (offset, column);
                    }
                    column += width;
                    offset = next;
                }
                expected.push((physical, start..range.end, start_column, column));
            }
            let rows: Vec<_> = VisualMap::new(&snapshot, Some(columns), tab_width, &[])
                .unwrap()
                .rows
                .into_iter()
                .map(|row| (row.physical_row, row.range, row.start_column, row.end_column))
                .collect();
            assert_eq!(rows, expected, "{columns} columns");
        }
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
