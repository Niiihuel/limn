//! Small block minimap drawn from real visual rows and read-only review zones. Work and draw
//! commands are bounded independently of document size; scrolling reuses the
//! revision cache instead of shaping text or walking the document again.
use super::*;
use unicode_width::UnicodeWidthChar;

const WIDTH: f32 = 72.0;
const MIN_SOURCE_WIDTH: f32 = 300.0;
const ROW_HEIGHT: f32 = 2.0;
const CELL_WIDTH: f32 = 0.75;
const MAX_ROWS: usize = 512;
const MAX_SCALARS: usize = 128;
const MAX_RUNS: usize = 8;
const MAX_HIGHLIGHTS: usize = 32_768;

pub(super) fn reserved_width(enabled: bool, available: f32) -> f32 {
    if enabled && available >= WIDTH + MIN_SOURCE_WIDTH + 8.0 {
        WIDTH
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    pub rect: Rect,
    pub content_height: f32,
    rows: usize,
}

impl Layout {
    pub fn new(rect: Rect, rows: usize) -> Self {
        Self {
            rect,
            content_height: (rows.max(1) as f32 * ROW_HEIGHT).min(rect.height),
            rows: rows.max(1),
        }
    }

    pub fn thumb(self, viewport: &CodeViewport) -> Rect {
        let range = viewport.max_y - viewport.min_y;
        let height = (self.content_height * viewport.body.height
            / (viewport.body.height + range).max(1.0))
        .max(8.0)
        .min(self.content_height);
        let fraction = if range > 0.0 {
            ((viewport.scroll_y - viewport.min_y) / range).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Rect::new(
            self.rect.x,
            self.rect.y + fraction * (self.content_height - height),
            self.rect.width,
            height,
        )
    }
    fn row_y(self, row: usize) -> f32 {
        row.min(self.rows.saturating_sub(1)) as f32 / self.rows as f32 * self.content_height
    }
}

#[derive(PartialEq)]
struct CacheKey {
    document: u64,
    revision: u64,
    map_generation: u64,
    review_generation: u64,
    language: CodeLanguage,
    tab_width: usize,
    rows: usize,
    width: u32,
    height: u32,
    text: Color,
    highlight_identity: Option<usize>,
    syntax_pending: bool,
}

struct Run {
    rect: Rect,
    color: Color,
}

#[derive(Default)]
pub(super) struct Cache {
    key: Option<CacheKey>,
    runs: Vec<Run>,
    // Retain the owner so a freed Arc allocation cannot reuse a cache identity.
    highlight_owner: Option<Arc<[CodeHighlight]>>,
    #[cfg(test)]
    builds: usize,
    #[cfg(test)]
    sampled_scalars: usize,
}

impl EditorView {
    pub(super) fn paint_minimap(
        &mut self,
        snapshot: &BufferSnapshot,
        external_syntax: bool,
        commands: &mut Vec<DrawCommand>,
    ) {
        let Some(layout) = self.viewport.minimap else {
            return;
        };
        let key = CacheKey {
            document: snapshot.document_id(),
            revision: snapshot.revision(),
            map_generation: self.visual_map_generation,
            review_generation: self.review_layout.generation,
            language: self.editor.language,
            tab_width: self.editor.tab_width,
            rows: layout.rows,
            width: layout.rect.width.to_bits(),
            height: layout.content_height.to_bits(),
            text: self.editor.palette.text,
            highlight_identity: external_syntax
                .then(|| Arc::as_ptr(&self.editor.highlights) as *const CodeHighlight as usize),
            syntax_pending: !external_syntax && self.syntax.pending(),
        };
        if self.minimap_cache.key.as_ref() != Some(&key) {
            self.rebuild_minimap(snapshot, external_syntax, layout);
            self.minimap_cache.key = Some(key);
        }
        commands.push(DrawCommand::PushClip(layout.rect));
        commands.push(DrawCommand::FillRect {
            rect: layout.rect,
            color: self.editor.palette.surface,
            radius: 0.0,
        });
        self.paint_minimap_selections(snapshot, layout, commands);
        for run in &self.minimap_cache.runs {
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(
                    layout.rect.x + run.rect.x,
                    layout.rect.y + run.rect.y,
                    run.rect.width,
                    run.rect.height,
                ),
                color: run.color,
                radius: 0.0,
            });
        }
        let thumb = layout.thumb(&self.viewport);
        let active = self.minimap_hovered || self.viewport.minimap_drag;
        commands.push(DrawCommand::FillRect {
            rect: thumb,
            color: with_alpha(self.editor.palette.muted, if active { 58 } else { 26 }),
            radius: 1.0,
        });
        commands.push(DrawCommand::StrokeRect {
            rect: thumb,
            color: with_alpha(self.editor.palette.muted, if active { 100 } else { 42 }),
            width: 1.0,
            radius: 1.0,
        });
        // Selection/caret markers are dynamic; they never invalidate the text
        // cache and use the same physical-to-visual mapping as the source.
        for selection in snapshot.selections().take(32) {
            let row_for = |offset| {
                self.visual_map.as_ref().map_or_else(
                    || {
                        snapshot
                            .position_of(offset)
                            .map_or(0, |position| position.row)
                    },
                    |(_, _, _, _, map)| map.row_for_offset(offset),
                )
            };
            let range = selection.range();
            let start = layout.row_y(self.review_layout.display_row(row_for(range.start)));
            let end = layout.row_y(self.review_layout.display_row(row_for(range.end)));
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(
                    layout.rect.x + layout.rect.width - 3.0,
                    layout.rect.y + start,
                    2.0,
                    (end - start + 1.5).max(1.5),
                ),
                color: with_alpha(self.editor.palette.text, 160),
                radius: 0.0,
            });
        }
        commands.push(DrawCommand::PopClip);
    }

    fn paint_minimap_selections(
        &self,
        snapshot: &BufferSnapshot,
        layout: Layout,
        commands: &mut Vec<DrawCommand>,
    ) {
        let selections: Vec<_> = snapshot
            .selections()
            .filter(|selection| !selection.is_empty())
            .take(32)
            .collect();
        // The overlay is independent of the revision cache. Bound the total
        // rows and scalar scans even for select-all in a very large document.
        let painted_rows = MAX_ROWS.min(layout.content_height.ceil().max(1.0) as usize);
        let row_budget = (painted_rows / selections.len().max(1)).max(1);
        let row_height = layout.content_height / layout.rows as f32;
        let inner_width = (layout.rect.width - 8.0).max(0.0);
        for selected in selections {
            let selected = selected.range();
            let row_for = |offset| {
                self.visual_map.as_ref().map_or_else(
                    || {
                        snapshot
                            .position_of(offset)
                            .map_or(0, |position| position.row)
                    },
                    |(_, _, _, _, map)| map.row_for_offset(offset),
                )
            };
            let first = row_for(selected.start).min(layout.rows - 1);
            let last = row_for(selected.end.saturating_sub(1)).min(layout.rows - 1);
            let step = (last - first + 1).div_ceil(row_budget).max(1);
            for visual in (first..=last).step_by(step) {
                let (range, column_origin) = if let Some((_, _, _, _, map)) = &self.visual_map {
                    let row = &map.rows[visual];
                    (row.range.clone(), row.start_column)
                } else {
                    let Ok(range) = snapshot.line_range(visual) else {
                        continue;
                    };
                    (range, 0)
                };
                let start = selected.start.max(range.start).min(range.end) - range.start;
                let end = selected.end.min(range.end).max(range.start) - range.start;
                let includes_newline = selected.start <= range.end && selected.end > range.end;
                if start >= end && !includes_newline {
                    continue;
                }
                let Ok(source) = snapshot
                    .slice(range.start..range.end.min(range.start.saturating_add(MAX_SCALARS)))
                else {
                    continue;
                };
                let mut column = column_origin;
                let mut left = (start == 0).then_some(0.0);
                let mut right = (end == 0).then_some(0.0);
                for (index, character) in source.chars().enumerate() {
                    column += if character == '\t' {
                        self.editor.tab_width - column % self.editor.tab_width
                    } else {
                        UnicodeWidthChar::width(character).unwrap_or(0)
                    };
                    let x = ((column - column_origin) as f32 * CELL_WIDTH).min(inner_width);
                    if index + 1 == start {
                        left = Some(x);
                    }
                    if index + 1 == end {
                        right = Some(x);
                    }
                    if x >= inner_width || index + 1 >= end {
                        break;
                    }
                }
                let left = left.unwrap_or(inner_width);
                let right = right.unwrap_or(inner_width);
                let right = if includes_newline {
                    right.max(left + CELL_WIDTH).min(inner_width)
                } else {
                    right
                };
                if right > left {
                    let display = self.review_layout.display_row(visual);
                    let y = layout.row_y(display);
                    let until_zone = self
                        .review_layout
                        .contiguous_rows(visual, (last - visual + 1).min(step));
                    let height = (until_zone as f32 * row_height)
                        .max(1.0)
                        .min(layout.content_height - y);
                    commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            layout.rect.x + 4.0 + left,
                            layout.rect.y + y,
                            right - left,
                            height,
                        ),
                        color: selection::COLOR,
                        radius: 0.0,
                    });
                }
            }
        }
    }

    fn rebuild_minimap(
        &mut self,
        snapshot: &BufferSnapshot,
        external_syntax: bool,
        layout: Layout,
    ) {
        self.minimap_cache.runs.clear();
        self.minimap_cache.highlight_owner =
            external_syntax.then(|| self.editor.highlights.clone());
        #[cfg(test)]
        {
            self.minimap_cache.builds += 1;
            self.minimap_cache.sampled_scalars = 0;
        }
        // Host spans need not arrive sorted. Index a bounded, evenly sampled
        // subset; omitted spans fall back to the normal text color.
        let mut external: Vec<&CodeHighlight> = if external_syntax {
            self.editor
                .highlights
                .iter()
                .step_by(self.editor.highlights.len().div_ceil(MAX_HIGHLIGHTS).max(1))
                .collect()
        } else {
            Vec::new()
        };
        external.sort_unstable_by_key(|span| span.range.start);
        let samples = layout
            .rows
            .min(MAX_ROWS)
            .min(layout.content_height.ceil().max(1.0) as usize);
        let sampled_height = layout.content_height / samples.max(1) as f32;
        for sample in 0..samples {
            let display = sample.saturating_mul(layout.rows) / samples;
            if self.review_layout.zone_row(display).is_some() {
                self.minimap_cache.runs.push(Run {
                    rect: Rect::new(
                        4.0,
                        sample as f32 * sampled_height,
                        (layout.rect.width - 8.0).max(0.0),
                        sampled_height.min(1.4),
                    ),
                    color: with_alpha(review::REMOVED_MARKER, 110),
                });
                continue;
            }
            let visual = self.review_layout.visual_row(display);
            let (physical, range, column_origin) = if let Some((_, _, _, _, map)) = &self.visual_map
            {
                let row = &map.rows[visual.min(map.rows.len().saturating_sub(1))];
                (row.physical_row, row.range.clone(), row.start_column)
            } else {
                let Ok(range) = snapshot.line_range(visual) else {
                    continue;
                };
                (visual, range, 0)
            };
            if self.review_layout.added(physical) {
                self.minimap_cache.runs.push(Run {
                    rect: Rect::new(
                        0.0,
                        sample as f32 * sampled_height,
                        2.0,
                        sampled_height.max(1.0),
                    ),
                    color: review::ADDED_MARKER,
                });
            }
            let Ok(source) =
                snapshot.slice(range.start..range.end.min(range.start.saturating_add(MAX_SCALARS)))
            else {
                continue;
            };
            let spans = self.highlight_index.lookup(
                &self.editor,
                &self.syntax,
                snapshot,
                physical,
                range.clone(),
            );
            let mut column = column_origin;
            let mut last: Option<Run> = None;
            let mut runs = 0;
            for (index, character) in source.chars().enumerate() {
                #[cfg(test)]
                {
                    self.minimap_cache.sampled_scalars += 1;
                }
                let x = (column - column_origin) as f32 * CELL_WIDTH + 4.0;
                if x >= layout.rect.width - 4.0 {
                    break;
                }
                let cells = if character == '\t' {
                    self.editor.tab_width - column % self.editor.tab_width
                } else {
                    UnicodeWidthChar::width(character).unwrap_or(0)
                };
                column += cells;
                if cells == 0 || character.is_whitespace() {
                    continue;
                }
                let offset = range.start + index;
                let color = if external_syntax {
                    let end = external.partition_point(|span| span.range.start <= offset);
                    external[..end]
                        .iter()
                        .rev()
                        .take(32)
                        .find(|span| span.range.contains(&offset))
                        .map(|span| span.color)
                } else {
                    spans
                        .iter()
                        .take(256)
                        .find(|span| span.range.contains(&offset))
                        .map(|span| span.color)
                }
                .unwrap_or(self.editor.palette.text);
                let color = with_alpha(color, 145);
                let width = (cells as f32 * CELL_WIDTH).min(layout.rect.width - 4.0 - x);
                if let Some(run) = last.as_mut().filter(|run| {
                    run.color == color && (run.rect.x + run.rect.width - x).abs() < 0.01
                }) {
                    run.rect.width += width;
                    continue;
                }
                if let Some(run) = last.take() {
                    self.minimap_cache.runs.push(run);
                    runs += 1;
                    if runs >= MAX_RUNS {
                        break;
                    }
                }
                last = Some(Run {
                    rect: Rect::new(
                        x,
                        sample as f32 * sampled_height,
                        width,
                        sampled_height.min(1.4),
                    ),
                    color,
                });
            }
            if let Some(run) = last {
                self.minimap_cache.runs.push(run);
            }
        }
    }
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color::rgba(color.r, color.g, color.b, alpha)
}

#[cfg(test)]
#[path = "code_editor_minimap_tests.rs"]
mod tests;
