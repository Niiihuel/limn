//! Snapshot-bound review decorations. Presentation rows never become buffer rows.
use super::*;
use crate::{Button, ButtonVariant, ControlSize, Icon, IconButton, TooltipPlacement};
use std::ops::Range;
use std::time::Duration;

const MAX_HUNKS: usize = 4096;
const MAX_REMOVED_ROWS: usize = 65_536;
const MAX_REMOVED_BYTES: usize = 4 * 1024 * 1024;
pub(super) const ADDED_FILL: Color = Color::rgba(72, 142, 92, 28);
pub(super) const REMOVED_FILL: Color = Color::rgba(170, 76, 85, 28);
pub(super) const ADDED_MARKER: Color = Color::rgb(105, 185, 135);
pub(super) const REMOVED_MARKER: Color = Color::rgb(210, 115, 122);

/// One exact diff hunk. Current rows are zero-based and end-exclusive. A pure
/// deletion has an empty `new_rows` range; its start anchors the removed zone
/// before that current row, or after the document when equal to `line_count`.
/// Each removed string is one old physical line, with an optional final CR/LF.
#[derive(Clone, Debug)]
pub struct CodeReviewHunk {
    /// Rows of the current text this hunk added or changed; they get the added tint. Hunks
    /// must come in ascending order without overlapping, inside the document.
    pub new_rows: Range<usize>,
    /// Zero-based row of the first removed line in the old text; the removed rows are numbered
    /// from it, one-based, in the gutter.
    pub old_start_row: usize,
    /// The old lines this hunk removed, drawn as extra rows above `new_rows`. Empty for a pure
    /// addition.
    pub removed_lines: Arc<[String]>,
}

/// Both IDs must identify the owner, immutable review version and hunk.
/// Acceptance and rejection execute in the host, never in this renderer.
#[derive(Clone, Debug, Default)]
pub struct CodeReviewHunkActions {
    /// Identifier of the button that keeps the hunk. Not empty, at most 1024 bytes, no control
    /// characters and different from `reject_id`, or the hunk gets no controls.
    pub accept_id: String,
    /// Identifier of the button that undoes the hunk, under the same rules as `accept_id`.
    pub reject_id: String,
    /// Optional, snapshot-scoped navigation actions. Keep them distinct from
    /// the header's IDs: both controls can be visible in the same UI tree.
    pub previous_id: Option<String>,
    /// Identifier of the arrow to the next hunk. The stepper is drawn only when this and
    /// `previous_id` are both set, valid and different from each other and from the two above.
    pub next_id: Option<String>,
    /// Whether the keep button and the two arrows respond. False in `Default`.
    pub enabled: bool,
    /// Whether the undo button responds, independently of `enabled`. False in `Default`.
    pub reject_enabled: bool,
    /// Tooltip of a disabled keep or undo button. `None` uses the library's
    /// `review_unavailable` label.
    pub disabled_reason: Option<String>,
}

/// Identity of the pill and counter derived from one review layout. Hosts must
/// compare accept_id with the active review token before publishing its index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeReviewPosition {
    /// `document_id` of the buffer snapshot the review was laid out for.
    pub document_id: u64,
    /// Revision of that snapshot.
    pub revision: u64,
    /// Zero-based index of the active hunk in the editor's review hunks.
    pub index: usize,
    /// `accept_id` of that hunk's actions.
    pub accept_id: String,
}

pub(super) struct Interaction {
    active: Option<usize>,
    requested: Option<usize>,
    caret: Option<(u64, u64, usize)>,
    generation: u64,
    pointer: Option<Point>,
    now: Instant,
    hover: Option<(usize, Instant, bool)>,
}

impl Interaction {
    /// No hunk is active and no pointer is known; `now` is the time of that reading.
    pub fn new(now: Instant) -> Self {
        Self {
            active: None,
            requested: None,
            caret: None,
            generation: 0,
            pointer: None,
            now,
            hover: None,
        }
    }

    /// Where the pointer rests over this editor, and the time of that reading.
    pub fn pointer(&self) -> Option<(Point, Instant)> {
        self.pointer.map(|point| (point, self.now))
    }

    /// True while the pointer rests on a hunk whose hover has not fired yet.
    pub fn hover_pending(&self) -> bool {
        self.hover.is_some_and(|(_, _, fired)| !fired)
    }
}

#[derive(Clone)]
struct Zone {
    visual: usize,
    display: usize,
    end: usize,
    cumulative: usize,
    hunk: usize,
    fragments: Vec<Fragment>,
}

#[derive(Clone)]
struct Fragment {
    line: usize,
    bytes: Range<usize>,
    column: usize,
}

#[derive(Default)]
pub(super) struct Layout {
    key: Option<(u64, u64, u64, usize)>,
    owner: Option<Arc<[CodeReviewHunk]>>,
    active: bool,
    valid: bool,
    zones: Vec<Zone>,
    added: Vec<Range<usize>>,
    hunk_ranges: Vec<Option<Range<usize>>>,
    real_rows: usize,
    pub generation: u64,
    pub rows: usize,
}

impl Layout {
    /// Lay the review hunks out again for this snapshot, visual map and wrap width; nothing is
    /// done when none of them changed. Hunks that are not for this exact snapshot, or that are
    /// invalid or over the limits, leave the layout without review rows.
    pub fn update(
        &mut self,
        editor: &CodeEditor,
        snapshot: &BufferSnapshot,
        map: Option<&VisualMap>,
        map_generation: u64,
        wrap_columns: usize,
    ) {
        let key = (
            snapshot.document_id(),
            snapshot.revision(),
            map_generation,
            wrap_columns,
        );
        let active = editor.review_document_id == Some(snapshot.document_id())
            && editor.review_revision == Some(snapshot.revision());
        let real_rows = map.map_or(snapshot.line_count(), |map| map.rows.len());
        if self.key == Some(key)
            && self.active == active
            && self.real_rows == real_rows
            && self
                .owner
                .as_ref()
                .is_some_and(|owner| Arc::ptr_eq(owner, &editor.review_hunks))
        {
            return;
        }
        self.key = Some(key);
        self.active = active;
        self.valid = false;
        self.owner = Some(editor.review_hunks.clone());
        self.real_rows = real_rows;
        self.rows = real_rows;
        self.zones.clear();
        self.added.clear();
        self.hunk_ranges.clear();
        self.generation = self.generation.wrapping_add(1);
        if !active || editor.review_hunks.len() > MAX_HUNKS {
            return;
        }
        let mut removed_rows = 0usize;
        let mut removed_bytes = 0usize;
        let mut previous_end = 0;
        for hunk in editor.review_hunks.iter() {
            removed_rows = removed_rows.saturating_add(hunk.removed_lines.len());
            if removed_rows > MAX_REMOVED_ROWS {
                return;
            }
            for line in hunk.removed_lines.iter() {
                removed_bytes = removed_bytes.saturating_add(line.len());
                if removed_bytes > MAX_REMOVED_BYTES || source_line(line).contains(['\n', '\r']) {
                    return;
                }
            }
            if hunk.new_rows.start < previous_end
                || hunk.new_rows.start > hunk.new_rows.end
                || hunk.new_rows.end > snapshot.line_count()
                || hunk
                    .old_start_row
                    .checked_add(hunk.removed_lines.len())
                    .is_none()
                || removed_rows > MAX_REMOVED_ROWS
                || removed_bytes > MAX_REMOVED_BYTES
            {
                return;
            }
            previous_end = hunk.new_rows.end;
        }
        let mut cumulative = 0;
        for (index, hunk) in editor.review_hunks.iter().enumerate() {
            if !hunk.new_rows.is_empty() {
                self.added.push(hunk.new_rows.clone());
            }
            let visual = map.map_or(hunk.new_rows.start, |map| {
                map.rows
                    .partition_point(|row| row.physical_row < hunk.new_rows.start)
            });
            // Folding hides zones anchored inside the hidden source range.
            let visible = hunk.new_rows.start == snapshot.line_count()
                || map.is_none_or(|map| {
                    map.rows
                        .get(visual)
                        .is_some_and(|row| row.physical_row == hunk.new_rows.start)
                });
            if visible && !hunk.removed_lines.is_empty() {
                let mut fragments = Vec::new();
                for (line, text) in hunk.removed_lines.iter().enumerate() {
                    let text = source_line(text);
                    let mut start = 0;
                    let mut column = 0;
                    let mut start_column = 0;
                    if wrap_columns > 0 {
                        for (offset, grapheme) in text.grapheme_indices(true) {
                            let width = if grapheme == "\t" {
                                editor.tab_width - column % editor.tab_width
                            } else {
                                UnicodeWidthStr::width(grapheme)
                            };
                            if offset > start && column + width - start_column > wrap_columns {
                                fragments.push(Fragment {
                                    line,
                                    bytes: start..offset,
                                    column: start_column,
                                });
                                start = offset;
                                start_column = column;
                            }
                            column += width;
                            if cumulative + fragments.len() > MAX_REMOVED_ROWS {
                                self.zones.clear();
                                self.added.clear();
                                return;
                            }
                        }
                    }
                    fragments.push(Fragment {
                        line,
                        bytes: start..text.len(),
                        column: start_column,
                    });
                }
                if cumulative + fragments.len() > MAX_REMOVED_ROWS {
                    self.zones.clear();
                    self.added.clear();
                    return;
                }
                let display = visual + cumulative;
                cumulative += fragments.len();
                self.zones.push(Zone {
                    visual,
                    display,
                    end: visual + cumulative,
                    cumulative,
                    hunk: index,
                    fragments,
                });
            }
        }
        self.rows = real_rows + cumulative;
        self.valid = true;
        self.hunk_ranges = (0..editor.review_hunks.len())
            .map(|index| self.compute_hunk_rows(index, editor, map))
            .collect();
    }

    /// The row on screen of visual row `visual`: the rows of removed lines shown above it are
    /// counted in.
    pub fn display_row(&self, visual: usize) -> usize {
        let index = self.zones.partition_point(|zone| zone.visual <= visual);
        visual
            + index
                .checked_sub(1)
                .map_or(0, |index| self.zones[index].cumulative)
    }

    /// How many of `count` rows starting at `visual` come before the next zone of removed lines.
    pub fn contiguous_rows(&self, visual: usize, count: usize) -> usize {
        let next = self.zones.partition_point(|zone| zone.visual <= visual);
        self.zones
            .get(next)
            .map_or(count, |zone| count.min(zone.visual - visual))
    }

    /// The visual row for a row on screen. A row inside a zone of removed lines maps to the
    /// visual row the zone sits above.
    pub fn visual_row(&self, display: usize) -> usize {
        let index = self.zones.partition_point(|zone| zone.display <= display);
        let visual = match index.checked_sub(1).map(|index| &self.zones[index]) {
            Some(zone) if display < zone.end => zone.visual,
            Some(zone) => display.saturating_sub(zone.cumulative),
            None => display,
        };
        visual.min(self.real_rows)
    }

    /// The hunk, and the index of its removed line, shown at screen row `display`. `None` when
    /// that row is not a removed line.
    pub fn zone_row(&self, display: usize) -> Option<(usize, usize)> {
        self.zone_fragment(display)
            .map(|(hunk, fragment)| (hunk, fragment.line))
    }

    fn zone_fragment(&self, display: usize) -> Option<(usize, &Fragment)> {
        let index = self
            .zones
            .partition_point(|zone| zone.display <= display)
            .checked_sub(1)?;
        let zone = &self.zones[index];
        zone.fragments
            .get(display - zone.display)
            .map(|fragment| (zone.hunk, fragment))
    }

    /// The first screen row of hunk `index`: its removed lines when it has any, otherwise its
    /// first current row. `None` when the layout is not valid or there is no such hunk.
    pub fn hunk_row(
        &self,
        index: usize,
        editor: &CodeEditor,
        map: Option<&VisualMap>,
    ) -> Option<usize> {
        if !self.valid {
            return None;
        }
        let hunk = editor.review_hunks.get(index)?;
        if let Ok(zone) = self.zones.binary_search_by_key(&index, |zone| zone.hunk) {
            let zone = &self.zones[zone];
            return Some(zone.display);
        }
        let visual = map.map_or(hunk.new_rows.start, |map| {
            map.rows
                .partition_point(|row| row.physical_row < hunk.new_rows.start)
        });
        Some(self.display_row(visual))
    }

    fn compute_hunk_rows(
        &self,
        index: usize,
        editor: &CodeEditor,
        map: Option<&VisualMap>,
    ) -> Option<Range<usize>> {
        if !self.valid {
            return None;
        }
        let hunk = editor.review_hunks.get(index)?;
        let start = self.hunk_row(index, editor, map)?;
        let end = if hunk.new_rows.is_empty() {
            self.zones
                .binary_search_by_key(&index, |zone| zone.hunk)
                .map_or(start + 1, |zone| self.zones[zone].end)
        } else {
            let visual = map.map_or(hunk.new_rows.end, |map| {
                map.rows
                    .partition_point(|row| row.physical_row < hunk.new_rows.end)
            });
            // A following hunk's deleted zone does not belong to this block.
            let before = self.zones.partition_point(|zone| zone.visual < visual);
            visual
                + before
                    .checked_sub(1)
                    .map_or(0, |index| self.zones[index].cumulative)
        };
        (end > start).then_some(start..end)
    }

    /// Whether physical row `physical` is one of the current rows of a hunk.
    pub fn added(&self, physical: usize) -> bool {
        self.added
            .partition_point(|range| range.start <= physical)
            .checked_sub(1)
            .is_some_and(|index| self.added[index].contains(&physical))
    }
}

fn source_line(text: &str) -> &str {
    text.strip_suffix("\r\n")
        .or_else(|| text.strip_suffix('\n'))
        .or_else(|| text.strip_suffix('\r'))
        .unwrap_or(text)
}

impl EditorView {
    pub(super) fn ensure_review_layout(&mut self, snapshot: &BufferSnapshot) {
        self.review_layout.update(
            &self.editor,
            snapshot,
            self.visual_map.as_ref().map(|(_, _, _, _, map)| map),
            self.visual_map_generation,
            self.visual_map
                .as_ref()
                .map_or(0, |(_, columns, _, _, _)| *columns),
        );
    }

    pub(crate) fn reveal_review_hunk(&mut self, index: usize) -> bool {
        let Ok(snapshot) = self.editor.controller.snapshot() else {
            return false;
        };
        if self.editor.review_document_id != Some(snapshot.document_id())
            || self.editor.review_revision != Some(snapshot.revision())
            || index >= self.editor.review_hunks.len()
        {
            return false;
        }
        self.review_reveal = Some((snapshot.document_id(), snapshot.revision(), index));
        self.review_interaction.requested = Some(index);
        true
    }

    pub(crate) fn set_review_pointer(
        &mut self,
        hovered: Option<&str>,
        point: Option<Point>,
        now: Instant,
    ) {
        self.review_interaction.now = now;
        self.review_interaction.pointer = (hovered == Some(self.editor.id.as_str()))
            .then_some(point)
            .flatten();
    }

    /// Last laid-out block, scoped to the exact current buffer snapshot.
    pub(crate) fn current_review_hunk(&self) -> Option<CodeReviewPosition> {
        let snapshot = self.editor.controller.snapshot().ok()?;
        if !self.review_layout.valid
            || self.editor.review_document_id != Some(snapshot.document_id())
            || self.editor.review_revision != Some(snapshot.revision())
            || !self
                .review_layout
                .owner
                .as_ref()
                .is_some_and(|hunks| Arc::ptr_eq(hunks, &self.editor.review_hunks))
        {
            return None;
        }
        let index = self
            .review_interaction
            .requested
            .or(self.review_interaction.active)
            .filter(|index| *index < self.editor.review_hunks.len())?;
        Some(CodeReviewPosition {
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
            index,
            accept_id: self.editor.review_actions.get(index)?.accept_id.clone(),
        })
    }

    pub(super) fn update_review_active(&mut self, snapshot: &BufferSnapshot) {
        if !self.review_layout.valid || self.editor.review_hunks.is_empty() {
            self.review_interaction.active = None;
            self.review_interaction.requested = None;
            self.review_interaction.hover = None;
            self.review_interaction.caret = None;
            return;
        }
        let visible = self.content_visible;
        let top = (visible.y - self.viewport.body.y + self.viewport.scroll_y) / LINE_HEIGHT;
        let bottom = top + visible.height / LINE_HEIGHT;
        let ranges = &self.review_layout.hunk_ranges;
        let is_visible = |index: usize| {
            ranges
                .get(index)
                .and_then(|range| range.as_ref())
                .is_some_and(|range| (range.start as f32) < bottom && range.end as f32 > top)
        };
        let caret = (
            snapshot.document_id(),
            snapshot.revision(),
            snapshot.selection().head,
        );
        let changed_caret = self.review_interaction.caret != Some(caret);
        let changed_layout = self.review_interaction.generation != self.review_layout.generation;
        let requested = self
            .review_interaction
            .requested
            .take()
            .filter(|index| *index < self.editor.review_hunks.len());
        if let Some(index) = requested {
            self.review_interaction.active = Some(index);
        } else if changed_caret {
            let physical = snapshot
                .position_of(caret.2)
                .map_or(0, |position| position.row);
            self.review_interaction.active = self
                .editor
                .review_hunks
                .iter()
                .enumerate()
                .min_by_key(|(_, hunk)| {
                    if physical < hunk.new_rows.start {
                        hunk.new_rows.start - physical
                    } else {
                        physical.saturating_sub(
                            hunk.new_rows.end.saturating_sub(1).max(hunk.new_rows.start),
                        )
                    }
                })
                .map(|(index, _)| index);
        } else if changed_layout
            || self
                .review_interaction
                .active
                .is_none_or(|index| !is_visible(index))
        {
            let target = top + (bottom - top) * 0.35;
            self.review_interaction.active = ranges
                .iter()
                .enumerate()
                .filter_map(|(index, range)| {
                    range.as_ref().map(|range| {
                        let distance = if target < range.start as f32 {
                            range.start as f32 - target
                        } else {
                            (target - range.end as f32).max(0.0)
                        };
                        (index, distance)
                    })
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(index, _)| index);
        }
        self.review_interaction.caret = Some(caret);
        self.review_interaction.generation = self.review_layout.generation;
        let hovered = self
            .review_interaction
            .pointer
            .filter(|point| visible.contains(*point))
            .and_then(|point| {
                let row = ((point.y - self.viewport.body.y + self.viewport.scroll_y).max(0.0)
                    / LINE_HEIGHT)
                    .floor() as usize;
                ranges
                    .iter()
                    .position(|range| range.as_ref().is_some_and(|range| range.contains(&row)))
            });
        if self.review_interaction.hover.map(|(index, _, _)| index) != hovered {
            self.review_interaction.hover =
                hovered.map(|index| (index, self.review_interaction.now, false));
        }
        if let Some((index, started, fired)) = &mut self.review_interaction.hover {
            if !*fired
                && self
                    .review_interaction
                    .now
                    .saturating_duration_since(*started)
                    >= Duration::from_secs(1)
            {
                self.review_interaction.active = Some(*index);
                *fired = true;
            }
        }
    }

    pub(super) fn build_review_controls(&mut self, snapshot: &BufferSnapshot, clip: Rect) {
        let Some(position) = self.current_review_hunk() else {
            return;
        };
        let index = position.index;
        if position.document_id != snapshot.document_id()
            || position.revision != snapshot.revision()
            || self.editor.review_actions.len() != self.editor.review_hunks.len()
        {
            return;
        }
        let actions = &self.editor.review_actions[index];
        let valid_id =
            |id: &str| !id.is_empty() && id.len() <= 1024 && !id.chars().any(char::is_control);
        if !valid_id(&actions.accept_id)
            || !valid_id(&actions.reject_id)
            || actions.accept_id == actions.reject_id
        {
            return;
        }
        let Some(rows) = self
            .review_layout
            .hunk_ranges
            .get(index)
            .and_then(|rows| rows.as_ref())
        else {
            return;
        };
        let body = self.viewport.body;
        let Some(visible) = self.content_visible.intersection(clip) else {
            return;
        };
        // A floating find surface may not clip all source text, but controls
        // still stay below its reserved reveal band and fixed scope headers.
        let controls_top = visible
            .y
            .max(body.y + self.editor.reveal_top_inset.max(0.0));
        let visible = Rect::new(
            visible.x,
            controls_top,
            visible.width,
            (visible.y + visible.height - controls_top).max(0.0),
        );
        let top = body.y + rows.start as f32 * LINE_HEIGHT - self.viewport.scroll_y;
        let bottom = body.y + rows.end as f32 * LINE_HEIGHT - self.viewport.scroll_y;
        if bottom <= visible.y || top >= visible.y + visible.height {
            return;
        }
        let height = ControlSize::Dense.height();
        let gap = 4.0;
        let accept_width = 80.0;
        let reject_width = 80.0;
        let decision_width = accept_width + reject_width + gap;
        if visible.width < decision_width || visible.height < height {
            return;
        }
        let count = format!("{}/{}", index + 1, self.editor.review_hunks.len());
        let count_width = (count.len() as f32 * 7.0).max(40.0);
        let navigation_width = height * 2.0 + count_width;
        let navigation = actions
            .previous_id
            .as_deref()
            .zip(actions.next_id.as_deref())
            .filter(|(previous, next)| {
                previous != next
                    && [*previous, *next].iter().all(|id| {
                        valid_id(id) && *id != actions.accept_id && *id != actions.reject_id
                    })
                    && visible.width >= decision_width + gap + navigation_width
            });
        let width = decision_width + navigation.map_or(0.0, |_| gap + navigation_width);
        // Native Buttons stay inside the editor's text viewport. Like Monaco's
        // content widget, prefer below the block, then above, then the nearest
        // visible edge when the block itself is taller than the viewport.
        let below = bottom + 4.0;
        let above = top - height - 4.0;
        let y = if below + height <= visible.y + visible.height {
            below
        } else if above >= visible.y {
            above
        } else {
            visible.y + visible.height - height
        };
        let x = (body.x + body.width - width - 14.0)
            .max(visible.x)
            .min(visible.x + visible.width - width);
        if let Some((previous, next)) = navigation {
            // The counter and buttons form one quiet, rounded stepper. Each
            // arrow keeps its own owner for focus and wheel routing.
            let counter = Element::truncated_text(count)
                .font_family(FontFamily::Sans)
                .font_size(11.0)
                .foreground(self.editor.palette.text)
                .center_content()
                .size(Some(count_width), Some(height));
            self.structure_controls.push(structure::StructureControl {
                action_id: String::new(),
                element: Element::row(vec![
                    Element::spacer().size(Some(height), Some(height)),
                    counter,
                    Element::spacer().size(Some(height), Some(height)),
                ])
                .size(Some(navigation_width), Some(height))
                .background(self.editor.palette.surface)
                .border(self.editor.palette.border, 1.0)
                .radius(8.0),
                rect: Rect::new(x, y, navigation_width, height),
                clip: visible,
            });
            for (id, icon, hint, left, enabled) in [
                (previous, Icon::ArrowUp, crate::labels().previous_change, x, index > 0),
                (
                    next,
                    Icon::ArrowDown,
                    crate::labels().next_change,
                    x + height + count_width,
                    index + 1 < self.editor.review_hunks.len(),
                ),
            ] {
                let element = IconButton::new(id, icon, self.editor.palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Dense)
                    .enabled(enabled && actions.enabled)
                    .tooltip(hint)
                    .tooltip_placement(TooltipPlacement::Above)
                    .render();
                self.structure_controls.push(structure::StructureControl {
                    action_id: id.to_owned(),
                    element,
                    rect: Rect::new(left, y, height, height),
                    clip: visible,
                });
            }
        }
        let decisions_x = x + navigation.map_or(0.0, |_| navigation_width + gap);
        for (id, label, enabled, left, button_width, variant) in [
            (
                &actions.reject_id,
                crate::labels().undo,
                actions.reject_enabled,
                decisions_x,
                reject_width,
                ButtonVariant::Outline,
            ),
            (
                &actions.accept_id,
                crate::labels().keep,
                actions.enabled,
                decisions_x + reject_width + gap,
                accept_width,
                ButtonVariant::Default,
            ),
        ] {
            let hint = if enabled {
                (if *id == actions.accept_id { crate::labels().keep_block } else { crate::labels().undo_block }).to_owned()
            } else {
                actions
                    .disabled_reason
                    .clone()
                    .unwrap_or_else(|| crate::labels().review_unavailable.into())
            };
            let rect = Rect::new(left, y, button_width, height);
            let element = Button::new(id, label, self.editor.palette)
                .variant(variant)
                .size(ControlSize::Dense)
                .width(button_width)
                .enabled(enabled)
                .tooltip(hint)
                .tooltip_placement(TooltipPlacement::Above)
                .render();
            self.structure_controls.push(structure::StructureControl {
                action_id: id.clone(),
                element,
                rect,
                clip: visible,
            });
        }
    }

    pub(super) fn review_page_target(
        &self,
        current: usize,
        amount: usize,
        up: bool,
        count: usize,
    ) -> usize {
        let display = self.review_layout.display_row(current);
        let target = if up {
            display.saturating_sub(amount)
        } else {
            display.saturating_add(amount)
        };
        let visual = self
            .review_layout
            .visual_row(target)
            .min(count.saturating_sub(1));
        // A page landing inside removed text still progresses to real text.
        if up && visual >= current {
            current.saturating_sub(1)
        } else {
            visual
        }
    }

    pub(super) fn review_zone_at(&self, point: Point) -> bool {
        if !self.viewport.rect.contains(point) || point.y < self.viewport.body.y {
            return false;
        }
        let row = ((point.y - self.viewport.body.y + self.viewport.scroll_y).max(0.0) / LINE_HEIGHT)
            .floor() as usize;
        self.review_layout.zone_row(row).is_some()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_removed_review_rows(
        &self,
        rect: Rect,
        visible_body: Rect,
        clip: Rect,
        first: usize,
        end: usize,
        fold_lane: f32,
        measure: &dyn Fn(&str) -> f32,
        text_height: f32,
        commands: &mut Vec<DrawCommand>,
    ) -> f32 {
        let body = self.viewport.body;
        let mut longest = body.width;
        let rows_clip = Rect::new(rect.x, visible_body.y, rect.width, visible_body.height);
        let Some(rows_clip) = rows_clip.intersection(clip) else {
            return longest;
        };
        commands.push(DrawCommand::PushClip(rows_clip));
        for row in first..end.min(self.review_layout.rows) {
            let Some((hunk, fragment)) = self.review_layout.zone_fragment(row) else {
                continue;
            };
            let hunk = &self.editor.review_hunks[hunk];
            let removed = fragment.line;
            let y = body.y + row as f32 * LINE_HEIGHT - self.viewport.scroll_y;
            let text_y = centered_text_y(y, LINE_HEIGHT, text_height);
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(rect.x, y, rect.width, LINE_HEIGHT),
                color: REMOVED_FILL,
                radius: 0.0,
            });
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(rect.x, y, 2.0, LINE_HEIGHT),
                color: REMOVED_MARKER,
                radius: 0.0,
            });
            if self.editor.line_numbers && fragment.bytes.start == 0 {
                let number = format!("−{}", hunk.old_start_row + removed + 1);
                commands.push(text_command(
                    Point::new(
                        body.x - fold_lane - structure::NUMBER_TRAILING_PADDING - measure(&number),
                        text_y,
                    ),
                    number,
                    REMOVED_MARKER,
                ));
            }
            let origin = fragment.column as f32 * measure(" ").max(1.0);
            let mut display = DisplayLine::window_at(
                &source_line(&hunk.removed_lines[removed])[fragment.bytes.clone()],
                self.editor.tab_width,
                self.viewport.scroll_x + origin,
                body.width,
                fragment.column,
                measure,
            );
            if self.editor.soft_wrap {
                for cluster in &mut display.clusters {
                    cluster.x -= origin;
                }
                display.width = display
                    .clusters
                    .last()
                    .map_or(0.0, |cluster| cluster.x + cluster.width);
                display.truncated = false;
            }
            longest = longest.max(display.width + if display.truncated { body.width } else { 0.0 });
            commands.push(DrawCommand::PushClip(
                visible_body
                    .intersection(clip)
                    .unwrap_or(Rect::new(body.x, body.y, 0.0, 0.0)),
            ));
            display.paint(
                commands,
                Point::new(body.x - self.viewport.scroll_x, text_y),
                (self.viewport.scroll_x, body.width),
                self.editor.palette.text,
                0,
                &[],
            );
            commands.push(DrawCommand::PopClip);
        }
        commands.push(DrawCommand::PopClip);
        longest
    }
}

#[cfg(test)]
#[path = "code_editor_review_tests.rs"]
mod tests;
