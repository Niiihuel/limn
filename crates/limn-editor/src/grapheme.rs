use crate::{validate_range, EditorError};
use ropey::Rope;
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

pub(crate) fn boundary(rope: &Rope, offset: usize, forward: bool) -> Result<usize, EditorError> {
    validate_range(&(offset..offset), rope.len_chars())?;
    if forward && offset == rope.len_chars() || !forward && offset == 0 {
        return Ok(offset);
    }
    let byte = rope.char_to_byte(offset);
    let mut cursor = GraphemeCursor::new(byte, rope.len_bytes(), true);
    let (mut chunk, mut start, _, _) = rope.chunk_at_byte(if forward { byte } else { byte - 1 });
    loop {
        let result = if forward {
            cursor.next_boundary(chunk, start)
        } else {
            cursor.prev_boundary(chunk, start)
        };
        match result {
            Ok(Some(byte)) => return Ok(rope.byte_to_char(byte)),
            Ok(None) => return Ok(if forward { rope.len_chars() } else { 0 }),
            Err(GraphemeIncomplete::PreContext(end)) => provide_context(rope, &mut cursor, end)?,
            Err(GraphemeIncomplete::PrevChunk) if start > 0 => {
                (chunk, start, _, _) = rope.chunk_at_byte(start - 1);
            }
            Err(GraphemeIncomplete::NextChunk) if start + chunk.len() < rope.len_bytes() => {
                (chunk, start, _, _) = rope.chunk_at_byte(start + chunk.len());
            }
            Err(_) => return Err(EditorError::InvalidGraphemeBoundary),
        }
    }
}

pub(crate) fn is_boundary(rope: &Rope, offset: usize) -> Result<bool, EditorError> {
    validate_range(&(offset..offset), rope.len_chars())?;
    if offset == 0 || offset == rope.len_chars() {
        return Ok(true);
    }
    let byte = rope.char_to_byte(offset);
    let (chunk, start, _, _) = rope.chunk_at_byte(byte);
    let mut cursor = GraphemeCursor::new(byte, rope.len_bytes(), true);
    loop {
        match cursor.is_boundary(chunk, start) {
            Ok(value) => return Ok(value),
            Err(GraphemeIncomplete::PreContext(end)) => provide_context(rope, &mut cursor, end)?,
            Err(_) => return Err(EditorError::InvalidGraphemeBoundary),
        }
    }
}

fn provide_context(
    rope: &Rope,
    cursor: &mut GraphemeCursor,
    end: usize,
) -> Result<(), EditorError> {
    if end == 0 || end > rope.len_bytes() {
        return Err(EditorError::InvalidGraphemeBoundary);
    }
    let (chunk, start, _, _) = rope.chunk_at_byte(end - 1);
    let prefix = chunk
        .get(..end - start)
        .ok_or(EditorError::InvalidGraphemeBoundary)?;
    cursor.provide_context(prefix, start);
    Ok(())
}
