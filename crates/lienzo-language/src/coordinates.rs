use lienzo_editor::BufferSnapshot;
use serde::{Deserialize, Serialize};

/// LSP UTF-16 coordinates. A position inside a surrogate pair is invalid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

const MAX_INDEX_LINES: usize = 1_000_000;
struct LineIndex {
    starts: Vec<usize>,
}
type CachedIndex = (u64, u64, std::sync::Arc<LineIndex>);
fn index(snapshot: &BufferSnapshot) -> Option<std::sync::Arc<LineIndex>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::VecDeque<CachedIndex>>> =
        std::sync::OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::VecDeque::new()));
    let key = (snapshot.document_id(), snapshot.revision());
    if let Ok(mut cache) = cache.lock() {
        if let Some(at) = cache
            .iter()
            .position(|(id, revision, _)| (*id, *revision) == key)
        {
            let entry = cache.remove(at)?;
            let index = entry.2.clone();
            cache.push_front(entry);
            return Some(index);
        }
    }
    // Ropey's Unicode line separators differ from the LSP reference implementation.
    // Build outside the cache lock; only CR, LF and CRLF create protocol lines.
    let source = snapshot.slice(0..snapshot.char_count()).ok()?;
    let mut chars = source.chars().enumerate().peekable();
    let mut starts = vec![0];
    while let Some((offset, ch)) = chars.next() {
        let next = match ch {
            '\r' if chars.peek().is_some_and(|(_, ch)| *ch == '\n') => {
                chars.next();
                Some(offset + 2)
            }
            '\r' | '\n' => Some(offset + 1),
            _ => None,
        };
        if let Some(next) = next {
            if starts.len() >= MAX_INDEX_LINES {
                return None;
            }
            starts.push(next);
        }
    }
    let index = std::sync::Arc::new(LineIndex { starts });
    if let Ok(mut cache) = cache.lock() {
        cache.retain(|(id, _, _)| *id != key.0);
        cache.push_front((key.0, key.1, index.clone()));
        cache.truncate(4);
    }
    Some(index)
}
pub(crate) fn warm_index(snapshot: &BufferSnapshot) {
    let _ = index(snapshot);
}
fn line_content(
    snapshot: &BufferSnapshot,
    index: &LineIndex,
    row: usize,
) -> Option<std::ops::Range<usize>> {
    let start = *index.starts.get(row)?;
    let mut end = index
        .starts
        .get(row + 1)
        .copied()
        .unwrap_or(snapshot.char_count());
    let slice = snapshot.slice(start..end).ok()?;
    let count = slice.len_chars();
    if count > 0 && matches!(slice.char(count - 1), '\n' | '\r') {
        end -= 1;
        if slice.char(count - 1) == '\n' && count > 1 && slice.char(count - 2) == '\r' {
            end -= 1;
        }
    }
    Some(start..end)
}
pub fn offset_to_utf16(snapshot: &BufferSnapshot, offset: usize) -> Option<Position> {
    if offset > snapshot.char_count() {
        return None;
    }
    let index = index(snapshot)?;
    let row = index
        .starts
        .partition_point(|start| *start <= offset)
        .saturating_sub(1);
    let line = line_content(snapshot, &index, row)?;
    let character = snapshot
        .slice(line.start..offset.min(line.end))
        .ok()?
        .len_utf16_cu();
    Some(Position {
        line: row.try_into().ok()?,
        character: character.try_into().ok()?,
    })
}
pub fn utf16_to_offset(snapshot: &BufferSnapshot, position: Position) -> Option<usize> {
    let index = index(snapshot)?;
    let line = line_content(snapshot, &index, position.line as usize)?;
    let slice = snapshot.slice(line.clone()).ok()?;
    let units = position.character as usize;
    if units > slice.len_utf16_cu() {
        return None;
    }
    let chars = slice.utf16_cu_to_char(units);
    (slice.char_to_utf16_cu(chars) == units).then_some(line.start + chars)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_roundtrips_non_bmp_combining_and_crlf_without_splitting_surrogates() {
        let snapshot = lienzo_editor::TextBuffer::new("a😀e\u{301}\r\nniño")
            .unwrap()
            .snapshot();
        for offset in [0, 1, 2, 3, 4, 6, 7, 8, 9, 10] {
            let position = offset_to_utf16(&snapshot, offset).unwrap();
            assert_eq!(utf16_to_offset(&snapshot, position), Some(offset));
        }
        assert_eq!(
            utf16_to_offset(
                &snapshot,
                Position {
                    line: 0,
                    character: 2
                }
            ),
            None
        );
        assert_eq!(
            utf16_to_offset(
                &snapshot,
                Position {
                    line: 1,
                    character: 99
                }
            ),
            None
        );
    }
    #[test]
    fn protocol_lines_use_cr_lf_not_unicode_separator_rows() {
        let snapshot = lienzo_editor::TextBuffer::new("a\u{2028}b\u{85}c\r😀\r\nz\n")
            .unwrap()
            .snapshot();
        assert_eq!(
            offset_to_utf16(&snapshot, 5),
            Some(Position {
                line: 0,
                character: 5
            })
        );
        assert_eq!(
            offset_to_utf16(&snapshot, 6),
            Some(Position {
                line: 1,
                character: 0
            })
        );
        assert_eq!(
            offset_to_utf16(&snapshot, 7),
            Some(Position {
                line: 1,
                character: 2
            })
        );
        assert_eq!(
            offset_to_utf16(&snapshot, 9),
            Some(Position {
                line: 2,
                character: 0
            })
        );
        assert_eq!(
            utf16_to_offset(
                &snapshot,
                Position {
                    line: 0,
                    character: 4
                }
            ),
            Some(4)
        );
        assert_eq!(
            utf16_to_offset(
                &snapshot,
                Position {
                    line: 1,
                    character: 1
                }
            ),
            None
        );
        assert_eq!(
            utf16_to_offset(
                &snapshot,
                Position {
                    line: 2,
                    character: 1
                }
            ),
            Some(10)
        );
        assert_eq!(
            offset_to_utf16(&snapshot, 11),
            Some(Position {
                line: 3,
                character: 0
            })
        );
    }
}
