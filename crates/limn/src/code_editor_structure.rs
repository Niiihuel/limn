//! Native, revision-scoped editor structure. Host analysis supplies candidates;
//! the view never parses source or consults the filesystem while painting.
use super::*;
use crate::{Icon, Tooltip, TooltipPlacement};

/// A marker's actual debugger acknowledgement, independent of source edits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeBreakpointState {
    Pending,
    Verified,
    Rejected,
    Disabled,
}
/// A breakpoint as the debugger reports it, on a zero-based row.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeBreakpoint {
    pub row: usize,
    pub state: CodeBreakpointState,
    /// Added to the marker's tooltip when present and not empty.
    pub message: Option<String>,
}
/// A row the debugger is stopped at, zero-based.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CodeExecutionLine {
    pub row: usize,
    /// The currently executing frame, rather than another inspected stack frame.
    pub primary: bool,
}

const MAX_STRUCTURE_ENTRIES: usize = 16_384;
pub(super) const GLYPH_LANE: f32 = 18.0;
pub(super) const BREAKPOINT_LANE: f32 = 14.0;
pub(super) const NUMBER_TRAILING_PADDING: f32 = 8.0;

/// A range of rows the gutter offers to fold; its control reports `action_id`.
#[derive(Clone, Debug)]
pub struct CodeFoldCandidate {
    /// Physical, zero-based rows [start_row, end_row).
    pub start_row: usize,
    pub end_row: usize,
    pub action_id: String,
}

/// A scope whose first row is offered as a sticky header; its control reports `action_id`.
#[derive(Clone, Debug)]
pub struct CodeStickyScope {
    /// Physical, zero-based rows [start_row, end_row).
    pub start_row: usize,
    pub end_row: usize,
    pub action_id: String,
}

#[derive(Default)]
pub(super) struct StructureCache {
    key: Option<(u64, u64)>,
    source_folds: Arc<[CodeFoldCandidate]>,
    source_scopes: Arc<[CodeStickyScope]>,
    folds: Vec<CodeFoldCandidate>,
    scopes: Vec<CodeStickyScope>,
    scope_max_end: Vec<usize>,
    breakpoint_key: Option<(u64, u64)>,
    source_breakpoints: Arc<[usize]>,
    breakpoints: std::collections::HashSet<usize>,
    debugger_key: Option<(u64, u64)>,
    debugger_source: Arc<[CodeBreakpoint]>,
    debugger_markers: std::collections::HashMap<usize, CodeBreakpoint>,
}

impl StructureCache {
    pub(super) fn update(&mut self, editor: &CodeEditor, snapshot: &BufferSnapshot) {
        let breakpoint_key = (snapshot.document_id(), snapshot.revision());
        if self.breakpoint_key != Some(breakpoint_key)
            || !Arc::ptr_eq(&self.source_breakpoints, &editor.breakpoint_rows)
        {
            self.breakpoint_key = Some(breakpoint_key);
            self.source_breakpoints = editor.breakpoint_rows.clone();
            self.breakpoints = editor
                .breakpoint_rows
                .iter()
                .take(MAX_STRUCTURE_ENTRIES)
                .copied()
                .filter(|row| *row < snapshot.line_count())
                .collect();
        }
        let debugger_valid = editor.debugger_document_id == Some(snapshot.document_id())
            && editor.debugger_revision == Some(snapshot.revision());
        if !debugger_valid {
            self.debugger_key = None;
            self.debugger_markers.clear();
        } else if self.debugger_key != Some(breakpoint_key)
            || !Arc::ptr_eq(&self.debugger_source, &editor.debugger_breakpoints)
        {
            self.debugger_key = Some(breakpoint_key);
            self.debugger_source = editor.debugger_breakpoints.clone();
            self.debugger_markers = editor
                .debugger_breakpoints
                .iter()
                .take(MAX_STRUCTURE_ENTRIES)
                .filter(|marker| marker.row < snapshot.line_count())
                .cloned()
                .map(|marker| (marker.row, marker))
                .collect();
        }
        if editor.structure_document_id != Some(snapshot.document_id())
            || editor.structure_revision != Some(snapshot.revision())
        {
            self.key = None;
            self.folds.clear();
            self.scopes.clear();
            self.scope_max_end.clear();
            return;
        }
        let key = (snapshot.document_id(), snapshot.revision());
        if self.key == Some(key)
            && Arc::ptr_eq(&self.source_folds, &editor.fold_candidates)
            && Arc::ptr_eq(&self.source_scopes, &editor.sticky_scopes)
        {
            return;
        }
        self.key = Some(key);
        self.source_folds = editor.fold_candidates.clone();
        self.source_scopes = editor.sticky_scopes.clone();
        let valid = |start: usize, end: usize, id: &str| {
            start < end.saturating_sub(1)
                && end <= snapshot.line_count()
                && !id.is_empty()
                && id.len() <= 512
        };
        self.folds = editor
            .fold_candidates
            .iter()
            .take(MAX_STRUCTURE_ENTRIES)
            .filter(|item| valid(item.start_row, item.end_row, &item.action_id))
            .cloned()
            .collect();
        self.folds
            .sort_by_key(|item| (item.start_row, std::cmp::Reverse(item.end_row)));
        self.folds.dedup_by_key(|item| item.start_row);
        self.scopes = editor
            .sticky_scopes
            .iter()
            .take(MAX_STRUCTURE_ENTRIES)
            .filter(|item| valid(item.start_row, item.end_row, &item.action_id))
            .cloned()
            .collect();
        self.scopes
            .sort_by_key(|item| (item.start_row, std::cmp::Reverse(item.end_row)));
        self.scopes
            .dedup_by_key(|item| (item.start_row, item.end_row));
        self.scope_max_end = vec![0; self.scopes.len().saturating_mul(4).max(1)];
        if !self.scopes.is_empty() {
            self.build_index(1, 0, self.scopes.len());
        }
    }
    fn build_index(&mut self, node: usize, start: usize, end: usize) -> usize {
        let maximum = if end - start == 1 {
            self.scopes[start].end_row
        } else {
            let mid = start + (end - start) / 2;
            self.build_index(node * 2, start, mid)
                .max(self.build_index(node * 2 + 1, mid, end))
        };
        self.scope_max_end[node] = maximum;
        maximum
    }
    pub(super) fn debugger_marker(&self, row: usize) -> Option<&CodeBreakpoint> {
        self.debugger_markers.get(&row)
    }
    pub(super) fn has_breakpoint(&self, row: usize) -> bool {
        // An exact debugger projection is the complete visible set. In
        // particular, adapter-relocated markers must not leave a second pending
        // dot at their original requested line.
        if self.debugger_key.is_some() {
            self.debugger_markers.contains_key(&row)
        } else {
            self.breakpoints.contains(&row)
        }
    }
    pub(super) fn fold_at(&self, row: usize) -> Option<&CodeFoldCandidate> {
        self.folds
            .binary_search_by_key(&row, |item| item.start_row)
            .ok()
            .map(|index| &self.folds[index])
    }
    /// Interval index prunes completed sibling scopes, keeping scrolling
    /// proportional to relevant ancestors rather than all preceding symbols.
    pub(super) fn scopes_at(&self, first: usize, last: usize) -> Vec<CodeStickyScope> {
        let mut result = Vec::new();
        if !self.scopes.is_empty() {
            self.query(1, 0, self.scopes.len(), first, last, &mut result);
        }
        result
    }
    fn query(
        &self,
        node: usize,
        start: usize,
        end: usize,
        first: usize,
        last: usize,
        out: &mut Vec<CodeStickyScope>,
    ) {
        if self.scope_max_end[node] <= first || self.scopes[start].start_row > last {
            return;
        }
        if end - start == 1 {
            out.push(self.scopes[start].clone());
        } else {
            let mid = start + (end - start) / 2;
            self.query(node * 2, start, mid, first, last, out);
            self.query(node * 2 + 1, mid, end, first, last, out);
        }
    }
}

#[derive(Clone)]
pub(crate) struct StructureControl {
    pub rect: Rect,
    pub clip: Rect,
    pub element: Element,
    pub action_id: String,
}

fn control(
    id: &str,
    content: Element,
    hint: &str,
    detail: &str,
    rect: Rect,
    clip: Rect,
    palette: ComponentPalette,
) -> StructureControl {
    let button = Element::clickable(id, content)
        .hover_background(Color::rgba(
            palette.text.r,
            palette.text.g,
            palette.text.b,
            14,
        ))
        .pressed_background(Color::rgba(
            palette.text.r,
            palette.text.g,
            palette.text.b,
            24,
        ))
        .transition_ms(crate::tokens().motion.hover_ms)
        .radius(3.0)
        .size(Some(rect.width), Some(rect.height));
    StructureControl {
        rect,
        clip,
        action_id: id.into(),
        element: Tooltip::new(button, id, hint, palette)
            .description(detail)
            .placement(TooltipPlacement::Right)
            .render(),
    }
}

pub(super) fn fold_control(
    candidate: &CodeFoldCandidate,
    folded: bool,
    rect: Rect,
    clip: Rect,
    palette: ComponentPalette,
) -> StructureControl {
    control(
        &candidate.action_id,
        Element::icon(if folded {
            Icon::ChevronRight
        } else {
            Icon::ChevronDown
        })
        .foreground(palette.muted)
        .size(Some(12.0), Some(12.0))
        .padding(3.0),
        if folded {
            crate::labels().expand_block
        } else {
            crate::labels().fold_block
        },
        &format!("{} {}–{}", crate::labels().lines, candidate.start_row + 1, candidate.end_row),
        rect,
        clip,
        palette,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn breakpoint_control(
    prefix: &str,
    row: usize,
    pending: bool,
    marker: Option<&CodeBreakpoint>,
    hovered: bool,
    rect: Rect,
    clip: Rect,
    palette: ComponentPalette,
) -> StructureControl {
    let state = marker
        .map(|marker| marker.state)
        .or(pending.then_some(CodeBreakpointState::Pending));
    let red = Color::rgb(206, 113, 113);
    let (icon, title, explanation) = match state {
        Some(CodeBreakpointState::Verified) => (
            Element::spacer()
                .size(Some(8.0), Some(8.0))
                .background(red)
                .radius(4.0),
            crate::labels().breakpoint_verified,
            crate::labels().breakpoint_verified_detail,
        ),
        Some(CodeBreakpointState::Rejected) => (
            Element::icon(Icon::WarningTriangle)
                .foreground(Color::rgb(221, 170, 80))
                .size(Some(8.0), Some(8.0)),
            crate::labels().breakpoint_rejected,
            crate::labels().breakpoint_rejected_detail,
        ),
        Some(CodeBreakpointState::Disabled) => (
            Element::spacer()
                .size(Some(8.0), Some(8.0))
                .border(palette.muted, 1.0)
                .radius(4.0),
            crate::labels().breakpoint_disabled,
            crate::labels().breakpoint_disabled_detail,
        ),
        Some(CodeBreakpointState::Pending) => (
            Element::spacer()
                .size(Some(8.0), Some(8.0))
                .border(red, 1.5)
                .radius(4.0),
            crate::labels().breakpoint_pending,
            if marker.is_some() {
                crate::labels().breakpoint_pending_detail
            } else {
                crate::labels().breakpoint_local_detail
            },
        ),
        None => (
            if hovered {
                Element::spacer()
                    .size(Some(8.0), Some(8.0))
                    .background(Color::rgba(red.r, red.g, red.b, 90))
                    .radius(4.0)
            } else {
                Element::spacer().size(Some(8.0), Some(8.0))
            },
            crate::labels().breakpoint_add,
            crate::labels().breakpoint_add_detail,
        ),
    };
    let mut detail = explanation.to_string();
    if let Some(message) = marker
        .and_then(|marker| marker.message.as_deref())
        .filter(|text| !text.is_empty())
    {
        detail.push('\n');
        detail.extend(message.chars().take(2048));
    }
    let mut control = control(
        &format!("{prefix}:{row}"),
        Element::row(vec![
            Element::spacer().flex(1.0),
            Element::column(vec![
                Element::spacer().flex(1.0),
                icon,
                Element::spacer().flex(1.0),
            ])
            .size(Some(8.0), None),
            Element::spacer().flex(1.0),
        ]),
        title,
        &detail,
        rect,
        clip,
        palette,
    );
    if let crate::ElementKind::Tooltip {
        anchor, shortcut, ..
    } = &mut control.element.kind
    {
        // The circle is the affordance; a second rectangular hover tile would
        // compete with the gutter and make the point appear against its edge.
        anchor.style.hover_background = None;
        anchor.style.pressed_background = Some(Color::rgba(red.r, red.g, red.b, 20));
        *shortcut = Some("F9".into());
    }
    control
}

pub(super) fn sticky_control(
    scope: &CodeStickyScope,
    rect: Rect,
    clip: Rect,
    palette: ComponentPalette,
) -> StructureControl {
    control(
        &scope.action_id,
        Element::spacer(),
        crate::labels().block_start,
        &format!("{} {}", crate::labels().line, scope.start_row + 1),
        rect,
        clip,
        palette,
    )
}

impl EditorView {
    fn structure_visual_row(&self, row: usize, line_count: usize) -> usize {
        let visual = self
            .visual_map
            .as_ref()
            .map_or(row.min(line_count), |(_, _, _, _, map)| {
                map.rows.partition_point(|entry| entry.physical_row < row)
            });
        self.review_layout.display_row(visual)
    }
    fn structure_physical_row(&self, visual: usize, line_count: usize) -> usize {
        let visual = self.review_layout.visual_row(visual);
        self.visual_map.as_ref().map_or(
            visual.min(line_count.saturating_sub(1)),
            |(_, _, _, _, map)| {
                map.rows
                    .get(visual)
                    .map_or(line_count, |entry| entry.physical_row)
            },
        )
    }
    pub(super) fn sticky_rows(
        &self,
        snapshot: &BufferSnapshot,
        body: Rect,
        find_inset: f32,
    ) -> Vec<CodeStickyScope> {
        let count =
            (((body.height - find_inset).max(0.0) * 0.25 / LINE_HEIGHT).floor() as usize).min(3);
        if count == 0 {
            return Vec::new();
        }
        let anchor = (self.viewport.scroll_y + find_inset).max(0.0);
        let first_visual = (anchor / LINE_HEIGHT).floor() as usize;
        let first = self.structure_physical_row(first_visual, snapshot.line_count());
        let last =
            self.structure_physical_row(first_visual.saturating_add(count), snapshot.line_count());
        let mut result: Vec<CodeStickyScope> = Vec::with_capacity(count);
        for scope in self.structure_cache.scopes_at(first, last) {
            if result.last().is_some_and(|parent| {
                scope.end_row > parent.end_row || scope.start_row == parent.start_row
            }) {
                continue;
            }
            let start = self.structure_visual_row(scope.start_row, snapshot.line_count());
            // A scope inside a folded range is not visible and cannot acquire a header.
            if self.structure_physical_row(start, snapshot.line_count()) != scope.start_row {
                continue;
            }
            let end = self.structure_visual_row(scope.end_row, snapshot.line_count());
            let edge = anchor + result.len() as f32 * LINE_HEIGHT;
            if start as f32 * LINE_HEIGHT < edge && end as f32 * LINE_HEIGHT > edge + LINE_HEIGHT {
                result.push(scope);
                if result.len() == count {
                    break;
                }
            }
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_sticky_rows(
        &mut self,
        scopes: &[CodeStickyScope],
        snapshot: &BufferSnapshot,
        rect: Rect,
        body: Rect,
        clip: Rect,
        find_inset: f32,
        fold_lane: f32,
        measure: &dyn Fn(&str) -> f32,
        text_height: f32,
        commands: &mut Vec<DrawCommand>,
    ) {
        for (index, scope) in scopes.iter().enumerate() {
            let y = body.y + find_inset + index as f32 * LINE_HEIGHT;
            let header = Rect::new(rect.x, y, (rect.width - 8.0).max(0.0), LINE_HEIGHT);
            let Some(header_clip) = header.intersection(clip) else {
                continue;
            };
            commands.push(DrawCommand::PushClip(header_clip));
            commands.push(DrawCommand::FillRect {
                rect: header,
                color: self.editor.palette.surface,
                radius: 0.0,
            });
            let text_y = centered_text_y(y, LINE_HEIGHT, text_height);
            if self.editor.line_numbers {
                let number = (scope.start_row + 1).to_string();
                commands.push(text_command(
                    Point::new(
                        body.x - fold_lane - NUMBER_TRAILING_PADDING - measure(&number),
                        text_y,
                    ),
                    number,
                    self.editor.palette.muted,
                ));
            }
            let (display, range, _) = visible_display_line(
                snapshot,
                scope.start_row,
                self.viewport.scroll_x,
                body.width,
                self.editor.tab_width,
                measure,
            );
            let highlights = self.highlight_index.lookup(
                &self.editor,
                &self.syntax,
                snapshot,
                scope.start_row,
                range.clone(),
            );
            commands.push(DrawCommand::PushClip(Rect::new(
                body.x,
                y,
                body.width,
                LINE_HEIGHT,
            )));
            display.paint(
                commands,
                Point::new(body.x - self.viewport.scroll_x, text_y),
                (self.viewport.scroll_x, body.width),
                self.editor.palette.text,
                range.start,
                &highlights,
            );
            commands.push(DrawCommand::PopClip);
            commands.push(DrawCommand::PopClip);
            self.structure_controls.push(sticky_control(
                scope,
                header,
                header_clip,
                self.editor.palette,
            ));
        }
        if !scopes.is_empty() {
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(
                    rect.x,
                    body.y + find_inset + scopes.len() as f32 * LINE_HEIGHT - 1.0,
                    (rect.width - 8.0).max(0.0),
                    1.0,
                ),
                color: self.editor.palette.border,
                radius: 0.0,
            });
        }
    }
}
