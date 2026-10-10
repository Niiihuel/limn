use std::ops::Range;

use crate::{BufferSnapshot, EditorError};

/// Locates the difference between two snapshots by the lines they share at
/// the start and at the end: the number of leading rows in common, then the
/// row where the common tail starts in each snapshot.
pub fn changed_rows(
    previous: &BufferSnapshot,
    next: &BufferSnapshot,
) -> Result<(usize, usize, usize), EditorError> {
    let (old, new) = (&previous.rope, &next.rope);
    let limit = old.len_bytes().min(new.len_bytes());
    let head = shared_bytes(old, new, limit, false);
    let tail = shared_bytes(old, new, limit - head, true);
    // Whole chunks are compared first, so the line scans below only have to
    // settle the rows next to the change. One row of slack on each side keeps
    // a line break split by the change out of the estimate.
    let shared = previous.line_count().min(next.line_count());
    let mut prefix = old
        .byte_to_line(head)
        .min(new.byte_to_line(head))
        .saturating_sub(1)
        .min(shared);
    while prefix < shared && previous.line(prefix)? == next.line(prefix)? {
        prefix += 1;
    }
    let rows_after = |rope: &ropey::Rope, lines: usize| {
        lines.saturating_sub(rope.byte_to_line(rope.len_bytes() - tail) + 2)
    };
    let kept = rows_after(old, previous.line_count())
        .min(rows_after(new, next.line_count()))
        .min(shared - prefix);
    let (mut old_end, mut new_end) = (previous.line_count() - kept, next.line_count() - kept);
    while old_end > prefix
        && new_end > prefix
        && previous.line(old_end - 1)? == next.line(new_end - 1)?
    {
        old_end -= 1;
        new_end -= 1;
    }
    Ok((prefix, old_end, new_end))
}

/// Bytes two ropes have in common at the start, or at the end when
/// `from_end`, up to `limit`.
fn shared_bytes(left: &ropey::Rope, right: &ropey::Rope, limit: usize, from_end: bool) -> usize {
    fn cursor(rope: &ropey::Rope, from_end: bool) -> ropey::iter::Chunks<'_> {
        if from_end {
            rope.chunks_at_byte(rope.len_bytes()).0.reversed()
        } else {
            rope.chunks()
        }
    }
    let (mut left, mut right) = (cursor(left, from_end), cursor(right, from_end));
    let (mut a, mut b): (&[u8], &[u8]) = (&[], &[]);
    let mut shared = 0;
    while shared < limit {
        if a.is_empty() {
            match left.next() {
                Some(chunk) => a = chunk.as_bytes(),
                None => break,
            }
        }
        if b.is_empty() {
            match right.next() {
                Some(chunk) => b = chunk.as_bytes(),
                None => break,
            }
        }
        let count = a.len().min(b.len());
        let (x, y) = if from_end {
            (&a[a.len() - count..], &b[b.len() - count..])
        } else {
            (&a[..count], &b[..count])
        };
        if x.as_ptr() != y.as_ptr() && x != y {
            let pairs = x.iter().zip(y);
            shared += if from_end {
                pairs.rev().take_while(|(x, y)| x == y).count()
            } else {
                pairs.take_while(|(x, y)| x == y).count()
            };
            break;
        }
        shared += count;
        (a, b) = if from_end {
            (&a[..a.len() - count], &b[..b.len() - count])
        } else {
            (&a[count..], &b[count..])
        };
    }
    shared.min(limit)
}

/// Carries end-exclusive row ranges from one snapshot of a document to a later
/// one. The lines both snapshots share at the start and at the end locate the
/// change; a range before it is kept, a range after it moves with it, and a
/// range that encloses it grows or shrinks by the same number of rows. A range
/// whose first or last row was itself rewritten is dropped, except when only
/// its first row changed and no line was added or removed.
pub fn remap_rows(
    previous: &BufferSnapshot,
    next: &BufferSnapshot,
    ranges: &[Range<usize>],
) -> Result<Vec<Range<usize>>, EditorError> {
    if ranges.is_empty() || previous.document_id() != next.document_id() {
        return Ok(Vec::new());
    }
    let (prefix, old_end, new_end) = changed_rows(previous, next)?;
    let shift = |row: usize| row - old_end + new_end;
    let header_only = new_end == old_end && old_end == prefix + 1;
    Ok(ranges
        .iter()
        .filter_map(|range| {
            let moved = if range.end <= prefix {
                range.clone()
            } else if range.start >= old_end {
                shift(range.start)..shift(range.end)
            } else if range.start < prefix && range.end > old_end {
                range.start..shift(range.end)
            } else if header_only && range.start == prefix {
                range.clone()
            } else {
                return None;
            };
            (moved.start + 1 < moved.end && moved.end <= next.line_count()).then_some(moved)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditKind, Selection, TextBuffer};

    fn lines(count: usize) -> String {
        (0..count).map(|row| format!("line {row}\n")).collect()
    }

    fn edited(text: &str, range: Range<usize>, replacement: &str) -> (BufferSnapshot, BufferSnapshot) {
        let mut buffer = TextBuffer::new(text).unwrap();
        let before = buffer.snapshot();
        let caret = Selection::caret(range.start + replacement.chars().count());
        buffer
            .edit(range, replacement, caret, EditKind::Typing, 0)
            .unwrap();
        (before, buffer.snapshot())
    }

    fn row_start(text: &str, row: usize) -> usize {
        text.split_inclusive('\n').take(row).map(|line| line.chars().count()).sum()
    }

    fn line_by_line(previous: &BufferSnapshot, next: &BufferSnapshot) -> (usize, usize, usize) {
        let shared = previous.line_count().min(next.line_count());
        let mut prefix = 0;
        while prefix < shared && previous.line(prefix).unwrap() == next.line(prefix).unwrap() {
            prefix += 1;
        }
        let (mut old_end, mut new_end) = (previous.line_count(), next.line_count());
        while old_end > prefix
            && new_end > prefix
            && previous.line(old_end - 1).unwrap() == next.line(new_end - 1).unwrap()
        {
            old_end -= 1;
            new_end -= 1;
        }
        (prefix, old_end, new_end)
    }

    #[test]
    fn changed_rows_match_a_line_by_line_comparison_for_arbitrary_edits() {
        let text: String = (0..700)
            .map(|row| match row % 7 {
                0 => "\r\n".to_owned(),
                1 => format!("ñandú {row} 終\r\n"),
                2 => "same\n".to_owned(),
                _ => format!("line {row}\n"),
            })
            .collect();
        let pieces = ["", "x", "\n", "\r", "\r\n", "same\n", "é", "a\nb\nc", "\n\n"];
        let mut seed = 0x9e37_79b9_u64;
        let mut random = |bound: usize| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            (seed >> 33) as usize % bound.max(1)
        };
        let mut buffer = TextBuffer::new(&text).unwrap();
        let original = buffer.snapshot();
        let mut earlier = original.clone();
        for step in 0..160 {
            let length = buffer.snapshot().char_count();
            let start = match step % 4 {
                0 => random(40),
                1 => length.saturating_sub(random(40)),
                _ => random(length + 1),
            };
            let end = (start + random(6) * random(30)).min(length);
            let piece = pieces[random(pieces.len())];
            let caret = Selection::caret(start + piece.chars().count());
            buffer.edit(start..end, piece, caret, EditKind::Typing, 0).unwrap();
            let now = buffer.snapshot();
            for before in [&original, &earlier] {
                assert_eq!(changed_rows(before, &now).unwrap(), line_by_line(before, &now));
                assert_eq!(changed_rows(&now, before).unwrap(), line_by_line(&now, before));
            }
            earlier = now;
        }
        assert_eq!(changed_rows(&original, &original).unwrap(), line_by_line(&original, &original));
        let empty = TextBuffer::new("").unwrap().snapshot();
        assert_eq!(changed_rows(&empty, &original).unwrap(), line_by_line(&empty, &original));
    }

    #[test]
    fn ranges_before_stay_ranges_after_move_and_an_enclosing_range_resizes() {
        let text = lines(40);
        let at = row_start(&text, 20);
        let (before, after) = edited(&text, at..at, "new a\nnew b\n");
        let ranges = [2..6, 10..30, 25..31, 34..40];
        assert_eq!(
            remap_rows(&before, &after, &ranges).unwrap(),
            vec![2..6, 10..32, 27..33, 36..42]
        );

        let (before, after) = edited(&text, row_start(&text, 20)..row_start(&text, 23), "");
        assert_eq!(
            remap_rows(&before, &after, &[2..6, 10..30, 25..31, 34..40]).unwrap(),
            vec![2..6, 10..27, 22..28, 31..37]
        );
    }

    #[test]
    fn a_range_whose_edge_was_rewritten_is_dropped_but_a_retyped_header_keeps_its_fold() {
        let text = lines(40);
        let header = row_start(&text, 10);
        let (before, after) = edited(&text, header..header + 4, "LINE");
        assert_eq!(remap_rows(&before, &after, &[10..20, 22..30]).unwrap(), vec![10..20, 22..30]);

        // A line added above the header leaves the block whole, one row lower.
        let (before, after) = edited(&text, header..header, "extra\n");
        assert_eq!(remap_rows(&before, &after, &[10..20, 22..30]).unwrap(), vec![11..21, 23..31]);

        // The header itself was split in two: the fold no longer starts where it did.
        let (before, after) = edited(&text, header + 4..header + 4, "\n");
        assert_eq!(remap_rows(&before, &after, &[10..20, 22..30]).unwrap(), vec![23..31]);

        // The closing row of the first range and the opening row of the second changed.
        let last = row_start(&text, 19);
        let (before, after) = edited(&text, last..row_start(&text, 23), "x\n");
        assert_eq!(remap_rows(&before, &after, &[10..20, 22..30, 32..36]).unwrap(), vec![29..33]);
    }

    #[test]
    fn nothing_survives_another_document_or_falls_outside_the_new_text() {
        let text = lines(12);
        let other = TextBuffer::new(&text).unwrap().snapshot();
        let mine = TextBuffer::new(&text).unwrap().snapshot();
        assert!(remap_rows(&mine, &other, &[2..6]).unwrap().is_empty());
        assert_eq!(remap_rows(&mine, &mine, &[2..6]).unwrap(), vec![2..6]);
        assert!(remap_rows(&mine, &mine, &[]).unwrap().is_empty());

        let (before, after) = edited(&text, row_start(&text, 4)..text.chars().count(), "");
        assert_eq!(remap_rows(&before, &after, &[0..3, 6..10, 2..12]).unwrap(), vec![0..3]);

        let (before, after) = edited(&text, 0..text.chars().count(), "entirely\ndifferent\n");
        assert!(remap_rows(&before, &after, &[0..3, 6..10]).unwrap().is_empty());
    }
}
