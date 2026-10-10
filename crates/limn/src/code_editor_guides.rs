//! Indentation guides use physical rows, but paint only inside the current
//! visual viewport. Work is bounded even for whitespace-only or giant files.
use super::*;
use std::{collections::HashMap, ops::Range};

const MAX_COLUMNS: usize = 512;
const NEIGHBOR_ROWS: usize = 256;
const CACHE_ROWS: usize = 4096;

#[derive(Default)]
pub(super) struct IndentGuideCache {
    key: Option<(u64, u64, usize)>,
    indents: HashMap<usize, Option<usize>>,
    depths: HashMap<usize, usize>,
    off_side: bool,
    active_row: Option<usize>,
    active: Option<(Range<usize>, usize)>,
}

impl IndentGuideCache {
    fn prepare(&mut self, snapshot: &BufferSnapshot, tabs: usize) {
        let key = (snapshot.document_id(), snapshot.revision(), tabs);
        if self.key != Some(key) {
            self.key = Some(key);
            self.indents.clear();
            self.depths.clear();
            self.active_row = None;
            self.active = None;
        }
    }

    fn indent(&mut self, snapshot: &BufferSnapshot, row: usize, tabs: usize) -> Option<usize> {
        if let Some(indent) = self.indents.get(&row) {
            return *indent;
        }
        let range = snapshot.line_range(row).ok()?;
        let slice = snapshot.slice(range).ok()?;
        let mut columns = 0;
        let mut result = None;
        for character in slice.chars().take(MAX_COLUMNS + 1) {
            match character {
                ' ' => columns += 1,
                '\t' => columns += tabs - columns % tabs,
                _ => {
                    result = Some(columns);
                    break;
                }
            }
            if columns >= MAX_COLUMNS {
                // An unexamined suffix could be all whitespace. Treat this
                // bounded prefix as a guide boundary, rather than inventing
                // a deeply nested scope or scanning an arbitrarily long line.
                result = Some(0);
                break;
            }
        }
        // A viewport-sized cache avoids retaining a document-wide line map.
        if self.indents.len() >= CACHE_ROWS {
            self.indents.clear();
        }
        self.indents.insert(row, result);
        result
    }

    fn depth(&mut self, snapshot: &BufferSnapshot, row: usize, tabs: usize) -> usize {
        if let Some(depth) = self.depths.get(&row) {
            return *depth;
        }
        if let Some(indent) = self.indent(snapshot, row, tabs) {
            return indent.div_ceil(tabs);
        }
        // Empty lines connect their surrounding scopes, but leading/trailing
        // whitespace and unbounded blank runs must not invent a scope.
        let above = (row.saturating_sub(NEIGHBOR_ROWS)..row)
            .rev()
            .find_map(|line| self.indent(snapshot, line, tabs));
        let below = (row + 1
            ..snapshot
                .line_count()
                .min(row.saturating_add(NEIGHBOR_ROWS + 1)))
            .find_map(|line| self.indent(snapshot, line, tabs));
        let depth = match (above, below) {
            (Some(a), Some(b)) if a == b => a.div_ceil(tabs),
            (Some(a), Some(b)) if self.off_side && a > b => b.div_ceil(tabs),
            (Some(a), Some(b)) => a.min(b) / tabs + 1,
            _ => 0,
        };
        if self.depths.len() >= CACHE_ROWS {
            self.depths.clear();
        }
        self.depths.insert(row, depth);
        depth
    }

    fn active_guide(&mut self, snapshot: &BufferSnapshot, row: usize, tabs: usize) {
        if self.active_row == Some(row) {
            return;
        }
        self.active_row = Some(row);
        let depth = self.depth(snapshot, row, tabs);
        if depth == 0 {
            self.active = None;
            return;
        }
        let mut start = row;
        let mut end = row + 1;
        while start > row.saturating_sub(NEIGHBOR_ROWS)
            && self.depth(snapshot, start - 1, tabs) >= depth
        {
            start -= 1;
        }
        while end
            < snapshot
                .line_count()
                .min(row.saturating_add(NEIGHBOR_ROWS + 1))
            && self.depth(snapshot, end, tabs) >= depth
        {
            end += 1;
        }
        self.active = Some((start..end, depth - 1));
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_row(
        &mut self,
        snapshot: &BufferSnapshot,
        row: usize,
        continuation: bool,
        body: Rect,
        y: f32,
        scroll_x: f32,
        tab_width: usize,
        space_width: f32,
        cursor_row: usize,
        focused: bool,
        language: CodeLanguage,
        color: Color,
        commands: &mut Vec<DrawCommand>,
    ) {
        // Wrapped continuations start with code, rather than indentation.
        if continuation {
            return;
        }
        let tabs = tab_width.clamp(1, 32);
        let off_side = language == CodeLanguage::Python;
        if self.off_side != off_side {
            self.key = None;
            self.off_side = off_side;
        }
        self.prepare(snapshot, tabs);
        let depth = self.depth(snapshot, row, tabs);
        if focused {
            self.active_guide(snapshot, cursor_row, tabs);
        }
        for level in 0..depth.min(MAX_COLUMNS / tabs) {
            let x = body.x + (level * tabs) as f32 * space_width - scroll_x;
            if x < body.x || x + 1.0 > body.x + body.width {
                continue;
            }
            let active = focused
                && self
                    .active
                    .as_ref()
                    .is_some_and(|(rows, active)| *active == level && rows.contains(&row));
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(x, y, 1.0, LINE_HEIGHT),
                color: Color::rgba(color.r, color.g, color.b, if active { 140 } else { 52 }),
                radius: 0.0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(text: &str) -> BufferSnapshot {
        TextBuffer::new(text).unwrap().snapshot()
    }

    #[test]
    fn spaces_tabs_and_blank_scope_bridges_share_visual_columns() {
        let snapshot = snapshot("fn main() {\n    outer();\n\t\tinner();\n\n    }\n}\n\n");
        let mut cache = IndentGuideCache::default();
        cache.prepare(&snapshot, 4);
        assert_eq!(
            (0..8)
                .map(|row| cache.depth(&snapshot, row, 4))
                .collect::<Vec<_>>(),
            [0, 1, 2, 2, 1, 0, 0, 0]
        );
    }

    #[test]
    fn guides_clip_horizontally_and_do_not_cross_wrapped_code() {
        let snapshot = snapshot("        nested();\n");
        let mut cache = IndentGuideCache::default();
        let mut commands = Vec::new();
        let body = Rect::new(60.0, 4.0, 200.0, 300.0);
        cache.paint_row(
            &snapshot,
            0,
            false,
            body,
            4.0,
            10.0,
            4,
            7.0,
            0,
            true,
            CodeLanguage::Rust,
            Color::rgb(140, 140, 140),
            &mut commands,
        );
        assert!(
            matches!(commands.as_slice(), [DrawCommand::FillRect { rect, color, .. }]
            if *rect == Rect::new(78.0, 4.0, 1.0, 20.0) && color.a == 140)
        );
        commands.clear();
        cache.paint_row(
            &snapshot,
            0,
            true,
            body,
            24.0,
            0.0,
            4,
            7.0,
            0,
            true,
            CodeLanguage::Rust,
            Color::rgb(140, 140, 140),
            &mut commands,
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn revisions_documents_and_tab_width_invalidate_cached_indentation() {
        let mut buffer = TextBuffer::new("\tvalue\n").unwrap();
        let mut cache = IndentGuideCache::default();
        let first = buffer.snapshot();
        cache.prepare(&first, 4);
        assert_eq!(cache.indent(&first, 0, 4), Some(4));
        cache.prepare(&first, 8);
        assert_eq!(cache.indent(&first, 0, 8), Some(8));
        buffer
            .edit(0..1, "", Selection::caret(0), EditKind::Replace, 0)
            .unwrap();
        let second = buffer.snapshot();
        cache.prepare(&second, 8);
        assert_eq!(cache.indent(&second, 0, 8), Some(0));
        let other = snapshot("        value\n");
        cache.prepare(&other, 4);
        assert_eq!(cache.depth(&other, 0, 4), 2);
    }

    #[test]
    fn python_blank_line_before_dedent_does_not_extend_finished_scope() {
        let snapshot = snapshot("def first():\n    pass\n\ndef second():\n    pass\n");
        let mut cache = IndentGuideCache {
            off_side: true,
            ..Default::default()
        };
        cache.prepare(&snapshot, 4);
        assert_eq!(cache.depth(&snapshot, 2, 4), 0);
    }

    #[test]
    fn huge_lines_and_blank_runs_have_bounded_work_and_no_invented_scope() {
        let snapshot = snapshot(&format!(
            "{}x\n{}\n",
            " ".repeat(100_000),
            "\n".repeat(10_000)
        ));
        let mut cache = IndentGuideCache::default();
        cache.prepare(&snapshot, 4);
        assert_eq!(cache.indent(&snapshot, 0, 4), Some(0));
        assert_eq!(cache.depth(&snapshot, 5_000, 4), 0);
        assert!(cache.indents.len() <= 2 * NEIGHBOR_ROWS + 2);
        for row in 0..snapshot.line_count() {
            cache.indent(&snapshot, row, 4);
        }
        assert!(cache.indents.len() <= CACHE_ROWS);
        let whitespace = super::tests::snapshot(&" ".repeat(MAX_COLUMNS * 2));
        cache.prepare(&whitespace, 4);
        assert_eq!(cache.depth(&whitespace, 0, 4), 0);
    }
}
