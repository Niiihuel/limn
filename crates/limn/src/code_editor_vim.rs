//! A native, deliberately bounded modal layer over the existing Unicode buffer.
//! Printable commands arrive as Text, never Key: desktop backends emit both.
use super::*;
use limn_editor::EditorError;
use std::ops::Range;

const MAX_COUNT: usize = 1_000;
const MAX_WORD_SCAN: usize = 100_000;

/// The mode of modal editing. It starts in `Normal`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VimMode {
    /// Typed characters are commands and the caret is a block on a character.
    #[default]
    Normal,
    /// Typed characters are inserted, as without modal editing. Escape returns to `Normal`.
    Insert,
    /// A character-wise selection, entered with `v` or by selecting with the pointer.
    Visual,
    /// A selection of whole lines, entered with `V`.
    VisualLine,
}
impl VimMode {
    /// The mode's name in capitals, as the editor's mode indicator writes it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "VISUAL LINE",
        }
    }
}

#[derive(Default)]
pub(super) struct VimState {
    pub mode: VimMode,
    count: usize,
    operator: Option<char>,
    operator_count: usize,
    pending_g: bool,
    visual_anchor: usize,
    pub cursor: Option<usize>,
    desired_column: Option<usize>,
    register: String,
    linewise_register: bool,
    // An insert gesture can create several engine history entries (newline,
    // indentation, a pause while typing). Undo it as the user's one gesture.
    insert_history_start: Option<Option<u64>>,
    undo_groups: Vec<(u64, Option<u64>)>,
    redo_groups: Vec<usize>,
    observed_revision: Option<u64>,
    observed_selection: Option<Selection>,
}

struct Motion {
    target: usize,
    inclusive: bool,
    linewise: bool,
}

impl EditorView {
    pub(crate) fn vim_mode(&self) -> Option<VimMode> {
        self.editor.vim_mode.then_some(self.vim.mode)
    }
    pub(crate) fn vim_consumes(&self, key: Key, ctrl: bool) -> bool {
        self.editor.vim_mode
            && (key == Key::Escape
                || (self.vim.mode != VimMode::Insert
                    && ctrl
                    && matches!(
                        key,
                        Key::B | Key::F | Key::U | Key::D | Key::J | Key::N | Key::P | Key::R
                    )))
    }
    pub(super) fn vim_input(&mut self, event: &UiEvent, now: Instant) -> Option<Vec<UiAction>> {
        if !self.editor.vim_mode {
            return None;
        }
        let controller = self.editor.controller.clone();
        let shared = controller.shared_buffer();
        let Ok(mut buffer) = shared.lock() else {
            return Some(vec![self.fail("Editor buffer lock is unavailable")]);
        };
        self.vim.observe_external_edit(&buffer);
        let escape = matches!(
            event,
            UiEvent::Key {
                key: Key::Escape,
                ..
            }
        );
        if self.vim.mode == VimMode::Insert && !escape {
            return None;
        }
        // Save/copy remain host/native actions; all other plain key presses
        // wait for the corresponding Text event or are consumed below.
        if matches!(
            event,
            UiEvent::Key {
                key: Key::S | Key::C,
                ctrl: true,
                ..
            }
        ) {
            return None;
        }
        self.cancel_composition();
        let before = buffer.snapshot();
        let timestamp = now.saturating_duration_since(self.started).as_millis() as u64;
        let page = (self.viewport.body.height / LINE_HEIGHT).floor().max(1.0) as usize;
        let mut copied = None;
        let result = self.vim.handle(
            event,
            &mut buffer,
            !self.editor.read_only,
            self.editor.tab_width,
            page,
            timestamp,
            &mut copied,
        );
        let after = buffer.snapshot();
        self.vim.observe_local_edit(&buffer);
        if before.revision() != after.revision() {
            self.syntax.invalidate_from(0);
        }
        let mut actions = Vec::new();
        if let Err(error) = result {
            actions.push(self.fail(error));
        }
        if let Some(text) = copied {
            actions.push(UiAction::CopyRequested { text });
        }
        if before.revision() != after.revision() {
            actions.push(UiAction::EditorChanged {
                id: self.editor.id.clone(),
                document_id: after.document_id(),
                revision: after.revision(),
            });
        }
        self.viewport.reveal_caret = true;
        Some(actions)
    }
    pub(super) fn paint_vim_mode(&self, rect: Rect, commands: &mut Vec<DrawCommand>) {
        if !self.editor.vim_mode || rect.width < 80.0 || rect.height < 22.0 {
            return;
        }
        let label = self.vim.mode.label();
        let width = label.len() as f32 * 7.0 + 12.0;
        let badge = Rect::new(rect.x + 6.0, rect.y + rect.height - 24.0, width, 18.0);
        commands.push(DrawCommand::FillRect {
            rect: badge,
            color: self.editor.palette.surface,
            radius: 4.0,
        });
        commands.push(DrawCommand::Text {
            position: Point::new(badge.x + 6.0, badge.y + 2.0),
            text: label.into(),
            color: self.editor.palette.muted,
            font_size: 11.0,
            font_family: FontFamily::Monospace,
        });
    }
}

impl VimState {
    fn observe_external_edit(&mut self, buffer: &TextBuffer) {
        let snapshot = buffer.snapshot();
        let selection_changed = self
            .observed_selection
            .is_some_and(|selection| selection != snapshot.selection());
        if self
            .observed_revision
            .is_some_and(|revision| revision != buffer.revision())
        {
            // Undo history positions can be reused after external undo/edit.
            // Never combine somebody else's transaction with a Vim gesture.
            self.undo_groups.clear();
            self.redo_groups.clear();
            if self.mode == VimMode::Insert {
                self.insert_history_start = Some(buffer.undo_transaction_id());
            }
            self.reset_pending();
            self.pointer_selection(&snapshot);
        } else if selection_changed {
            self.pointer_selection(&snapshot);
        }
        self.observe_local_edit(buffer);
    }
    pub(super) fn observe_local_edit(&mut self, buffer: &TextBuffer) {
        self.observed_revision = Some(buffer.revision());
        self.observed_selection = Some(buffer.selection());
    }
    pub(super) fn pointer_selection(&mut self, snapshot: &BufferSnapshot) {
        self.reset_pending();
        self.desired_column = None;
        if self.mode == VimMode::Insert {
            return;
        }
        let selected = snapshot.selection();
        if selected.is_empty() {
            self.mode = VimMode::Normal;
            self.cursor = None;
        } else {
            self.mode = VimMode::Visual;
            self.visual_anchor = if selected.anchor < selected.head {
                selected.anchor
            } else {
                snapshot
                    .prev_grapheme(selected.anchor)
                    .unwrap_or(selected.anchor)
            };
            self.cursor = Some(if selected.head > selected.anchor {
                snapshot
                    .prev_grapheme(selected.head)
                    .unwrap_or(selected.head)
            } else {
                selected.head
            });
        }
    }
    fn reset_pending(&mut self) {
        self.count = 0;
        self.operator = None;
        self.operator_count = 0;
        self.pending_g = false;
    }
    fn record_undo(&mut self, before: Option<u64>, after: Option<u64>) {
        if let Some(after) = after.filter(|after| Some(*after) != before) {
            self.undo_groups.push((after, before));
            if self.undo_groups.len() > 512 {
                self.undo_groups.remove(0);
            }
            self.redo_groups.clear();
        }
    }
    fn normal(&mut self, buffer: &mut TextBuffer, cursor: usize) -> Result<(), EditorError> {
        self.mode = VimMode::Normal;
        self.cursor = None;
        self.reset_pending();
        let target = normal_offset(&buffer.snapshot(), cursor)?;
        buffer.set_selection(Selection::caret(target))
    }
    fn begin_insert(
        &mut self,
        buffer: &mut TextBuffer,
        position: usize,
        history: Option<u64>,
    ) -> Result<(), EditorError> {
        self.mode = VimMode::Insert;
        self.cursor = None;
        self.reset_pending();
        self.insert_history_start = Some(history);
        buffer.set_selection(Selection::caret(position))?;
        buffer.break_undo_group();
        Ok(())
    }
    fn select(&mut self, buffer: &mut TextBuffer, target: usize) -> Result<(), EditorError> {
        let snapshot = buffer.snapshot();
        let target = normal_offset(&snapshot, target)?;
        if matches!(self.mode, VimMode::Visual | VimMode::VisualLine) {
            self.cursor = Some(target);
            let anchor = normal_offset(&snapshot, self.visual_anchor.min(snapshot.char_count()))?;
            let range = if self.mode == VimMode::VisualLine {
                full_lines(&snapshot, anchor, target)?
            } else {
                anchor.min(target)..snapshot.next_grapheme(anchor.max(target))?
            };
            buffer.set_selection(if target >= anchor {
                Selection {
                    anchor: range.start,
                    head: range.end,
                }
            } else {
                Selection {
                    anchor: range.end,
                    head: range.start,
                }
            })
        } else {
            buffer.set_selection(Selection::caret(target))
        }
    }
    fn handle(
        &mut self,
        event: &UiEvent,
        buffer: &mut TextBuffer,
        editable: bool,
        tab_width: usize,
        page: usize,
        now: u64,
        copied: &mut Option<String>,
    ) -> Result<(), EditorError> {
        if matches!(
            event,
            UiEvent::Key {
                key: Key::Escape,
                ..
            }
        ) {
            let snapshot = buffer.snapshot();
            let mut cursor = self.cursor.unwrap_or(snapshot.selection().head);
            if self.mode == VimMode::Insert {
                let range = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
                if cursor > range.start {
                    cursor = snapshot.prev_grapheme(cursor)?;
                }
                if let Some(start) = self.insert_history_start.take() {
                    self.record_undo(start, buffer.undo_transaction_id());
                }
            }
            buffer.cancel_snippet();
            buffer.break_undo_group();
            return self.normal(buffer, cursor);
        }
        if let UiEvent::Text(text) = event {
            // Bound a malformed backend Text batch; a mode-changing command
            // consumes its whole event, never inserts trailing command text.
            for character in text.chars().take(64) {
                self.character(character, buffer, editable, tab_width, now, copied)?;
                if self.mode == VimMode::Insert {
                    break;
                }
            }
            return Ok(());
        }
        let UiEvent::Key { key, ctrl, .. } = event else {
            return Ok(());
        };
        if *ctrl {
            let cursor = self.cursor.unwrap_or(buffer.selection().head);
            let count = self.count.max(1);
            let (direction, amount) = match key {
                Key::J | Key::N => (1, count),
                Key::P => (-1, count),
                Key::B => (-1, page.saturating_mul(count)),
                Key::F => (1, page.saturating_mul(count)),
                Key::U => (-1, (page / 2).max(1).saturating_mul(count)),
                Key::D => (1, (page / 2).max(1).saturating_mul(count)),
                Key::R if editable => {
                    for _ in 0..count {
                        let before = buffer.undo_transaction_id();
                        let steps = self.redo_groups.pop().unwrap_or(1);
                        for _ in 0..steps {
                            if !buffer.redo()? {
                                break;
                            }
                        }
                        let after = buffer.undo_transaction_id();
                        if let Some(after) = after.filter(|after| Some(*after) != before) {
                            self.undo_groups.push((after, before));
                        }
                    }
                    return self.normal(buffer, buffer.selection().head);
                }
                _ => return Ok(()),
            };
            let target = self.vertical(&buffer.snapshot(), cursor, direction, amount, tab_width)?;
            self.reset_pending();
            return self.select(buffer, target);
        }
        let command = match key {
            Key::ArrowLeft => 'h',
            Key::ArrowRight => 'l',
            Key::ArrowUp => 'k',
            Key::ArrowDown => 'j',
            Key::Home => '0',
            Key::End => '$',
            Key::Delete => 'x',
            _ => return Ok(()),
        };
        self.character(command, buffer, editable, tab_width, now, copied)
    }
    fn character(
        &mut self,
        character: char,
        buffer: &mut TextBuffer,
        editable: bool,
        tab_width: usize,
        now: u64,
        copied: &mut Option<String>,
    ) -> Result<(), EditorError> {
        let before = buffer.undo_transaction_id();
        let revision = buffer.revision();
        let result = self.character_inner(character, buffer, editable, tab_width, now, copied);
        if buffer.revision() != revision && self.mode != VimMode::Insert && character != 'u' {
            self.record_undo(before, buffer.undo_transaction_id());
        }
        result
    }
    fn character_inner(
        &mut self,
        character: char,
        buffer: &mut TextBuffer,
        editable: bool,
        tab_width: usize,
        now: u64,
        copied: &mut Option<String>,
    ) -> Result<(), EditorError> {
        if character.is_ascii_digit() && (character != '0' || self.count > 0) {
            self.count = (self.count.saturating_mul(10) + character.to_digit(10).unwrap() as usize)
                .min(MAX_COUNT);
            return Ok(());
        }
        let snapshot = buffer.snapshot();
        let cursor = normal_offset(
            &snapshot,
            self.cursor
                .unwrap_or(snapshot.selection().head)
                .min(snapshot.char_count()),
        )?;
        let explicit_count = self.count > 0 || self.operator_count > 0;
        let count = self
            .count
            .max(1)
            .saturating_mul(self.operator_count.max(1))
            .min(MAX_COUNT);
        if character == 'g' && !self.pending_g {
            self.pending_g = true;
            return Ok(());
        }
        let go_first = self.pending_g && character == 'g';
        if self.pending_g && !go_first {
            self.reset_pending();
            return Ok(());
        }
        let visual = matches!(self.mode, VimMode::Visual | VimMode::VisualLine);
        if matches!(character, 'd' | 'c' | 'y') {
            if visual {
                let linewise = self.mode == VimMode::VisualLine;
                let range = snapshot.selection().range();
                return self.operate(character, range, linewise, buffer, editable, now, copied);
            }
            if self.operator == Some(character) {
                let row = snapshot.position_of(cursor)?.row;
                let target = snapshot
                    .line_range((row + count - 1).min(snapshot.line_count() - 1))?
                    .start;
                return self.operate(
                    character,
                    full_lines(&snapshot, cursor, target)?,
                    true,
                    buffer,
                    editable,
                    now,
                    copied,
                );
            }
            if self.operator.is_none() {
                self.operator = Some(character);
                self.operator_count = self.count.max(1);
                self.count = 0;
                return Ok(());
            }
            self.reset_pending();
            return Ok(());
        }
        if let Some(motion) = self.motion(
            character,
            go_first,
            explicit_count,
            count,
            &snapshot,
            cursor,
            tab_width,
        )? {
            if let Some(operator) = self.operator {
                let mut range = if motion.linewise {
                    full_lines(&snapshot, cursor, motion.target)?
                } else {
                    let end = cursor.max(motion.target);
                    cursor.min(motion.target)..if motion.inclusive {
                        snapshot.next_grapheme(end)?
                    } else {
                        end
                    }
                };
                // cw leaves the following whitespace in place, even when
                // the word under the caret is only a single grapheme.
                if operator == 'c' && character == 'w' && class(&snapshot, cursor)? != 0 {
                    while range.end > range.start {
                        let previous = snapshot.prev_grapheme(range.end)?;
                        if class(&snapshot, previous)? != 0 {
                            break;
                        }
                        range.end = previous;
                    }
                }
                return self.operate(
                    operator,
                    range,
                    motion.linewise,
                    buffer,
                    editable,
                    now,
                    copied,
                );
            }
            self.reset_pending();
            return self.select(buffer, motion.target);
        }
        if self.operator.is_some() {
            self.reset_pending();
            return Ok(());
        }
        let history = buffer.undo_transaction_id();
        match character {
            'v' | 'V' => {
                let mode = if character == 'v' {
                    VimMode::Visual
                } else {
                    VimMode::VisualLine
                };
                if self.mode == mode {
                    return self.normal(buffer, cursor);
                }
                if !visual {
                    self.visual_anchor = cursor;
                }
                self.mode = mode;
                self.reset_pending();
                return self.select(buffer, cursor);
            }
            'i' | 'a' | 'I' | 'A' if editable => {
                let range = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
                let position = match character {
                    'a' => snapshot.next_grapheme(cursor)?.min(range.end),
                    'I' => first_nonblank(&snapshot, cursor)?,
                    'A' => range.end,
                    _ => cursor,
                };
                return self.begin_insert(buffer, position, history);
            }
            'o' | 'O' if editable => {
                let range = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
                let indent = snapshot
                    .text_range(range.clone())?
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect::<String>();
                let newline = snapshot.preferred_line_ending().as_str();
                let (position, text, offset) = if character == 'o' {
                    (
                        range.end,
                        format!("{newline}{indent}"),
                        newline.chars().count() + indent.chars().count(),
                    )
                } else {
                    (
                        range.start,
                        format!("{indent}{newline}"),
                        indent.chars().count(),
                    )
                };
                buffer.edit(
                    position..position,
                    &text,
                    Selection::caret(position + offset),
                    EditKind::Replace,
                    now,
                )?;
                return self.begin_insert(buffer, position + offset, history);
            }
            'x' | 'X' => {
                if visual {
                    return self.operate(
                        'd',
                        snapshot.selection().range(),
                        self.mode == VimMode::VisualLine,
                        buffer,
                        editable,
                        now,
                        copied,
                    );
                }
                let line = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
                let mut end = cursor;
                for _ in 0..count {
                    end = if character == 'x' {
                        snapshot.next_grapheme(end)?.min(line.end)
                    } else {
                        snapshot.prev_grapheme(end)?.max(line.start)
                    };
                }
                return self.operate(
                    'd',
                    cursor.min(end)..cursor.max(end),
                    false,
                    buffer,
                    editable,
                    now,
                    copied,
                );
            }
            'D' | 'C' => {
                let end = snapshot.line_range(snapshot.position_of(cursor)?.row)?.end;
                return self.operate(
                    if character == 'D' { 'd' } else { 'c' },
                    cursor..end,
                    false,
                    buffer,
                    editable,
                    now,
                    copied,
                );
            }
            'p' | 'P' if editable => {
                self.paste(buffer, cursor, character == 'P', count, now)?;
            }
            'u' if editable => {
                for _ in 0..count {
                    let available = buffer.undo_transaction_id();
                    let boundary = if self
                        .undo_groups
                        .last()
                        .is_some_and(|(after, _)| Some(*after) == available)
                    {
                        Some(self.undo_groups.pop().unwrap().1)
                    } else {
                        None
                    };
                    let mut undone = 0;
                    for _ in 0..MAX_COUNT {
                        if !buffer.undo()? {
                            break;
                        }
                        undone += 1;
                        if boundary.is_none() || buffer.undo_transaction_id() == boundary.unwrap() {
                            break;
                        }
                    }
                    if undone > 0 {
                        self.redo_groups.push(undone);
                    }
                }
                return self.normal(buffer, buffer.selection().head);
            }
            _ => {}
        }
        self.reset_pending();
        Ok(())
    }
    fn vertical(
        &mut self,
        snapshot: &BufferSnapshot,
        cursor: usize,
        direction: isize,
        count: usize,
        tab_width: usize,
    ) -> Result<usize, EditorError> {
        let position = snapshot.position_of(cursor)?;
        let column = *self
            .desired_column
            .get_or_insert(snapshot.display_column(cursor, tab_width)?);
        let row = if direction < 0 {
            position.row.saturating_sub(count)
        } else {
            position
                .row
                .saturating_add(count)
                .min(snapshot.line_count() - 1)
        };
        snapshot.offset_for_display_column(row, column, tab_width)
    }
    fn motion(
        &mut self,
        command: char,
        first: bool,
        explicit: bool,
        count: usize,
        snapshot: &BufferSnapshot,
        cursor: usize,
        tab_width: usize,
    ) -> Result<Option<Motion>, EditorError> {
        let mut target = cursor;
        let mut inclusive = false;
        let mut linewise = false;
        let line = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
        if !matches!(command, 'j' | 'k') {
            self.desired_column = None;
        }
        match command {
            'h' => {
                for _ in 0..count {
                    target = snapshot.prev_grapheme(target)?.max(line.start);
                }
            }
            'l' => {
                for _ in 0..count {
                    target = snapshot.next_grapheme(target)?.min(line.end);
                }
            }
            'j' | 'k' => {
                target = self.vertical(
                    snapshot,
                    cursor,
                    if command == 'j' { 1 } else { -1 },
                    count,
                    tab_width,
                )?;
                linewise = true;
            }
            'w' | 'b' | 'e' => {
                let mut budget = MAX_WORD_SCAN;
                for _ in 0..count {
                    target = word_motion(snapshot, target, command, &mut budget)?;
                }
                inclusive = command == 'e';
            }
            '0' => target = line.start,
            '^' => target = first_nonblank(snapshot, cursor)?,
            '$' => {
                let row =
                    (snapshot.position_of(cursor)?.row + count - 1).min(snapshot.line_count() - 1);
                let line = snapshot.line_range(row)?;
                target = if line.is_empty() {
                    line.start
                } else {
                    snapshot.prev_grapheme(line.end)?
                };
                inclusive = true;
            }
            'g' if first => {
                target = snapshot
                    .line_range(
                        if explicit { count - 1 } else { 0 }.min(snapshot.line_count() - 1),
                    )?
                    .start;
                linewise = true;
            }
            'G' => {
                target = snapshot
                    .line_range(
                        if explicit {
                            count - 1
                        } else {
                            snapshot.line_count() - 1
                        }
                        .min(snapshot.line_count() - 1),
                    )?
                    .start;
                linewise = true;
            }
            _ => return Ok(None),
        }
        Ok(Some(Motion {
            target,
            inclusive,
            linewise,
        }))
    }
    fn operate(
        &mut self,
        operator: char,
        mut range: Range<usize>,
        linewise: bool,
        buffer: &mut TextBuffer,
        editable: bool,
        now: u64,
        copied: &mut Option<String>,
    ) -> Result<(), EditorError> {
        self.reset_pending();
        if !editable && operator != 'y' {
            return Ok(());
        }
        let snapshot = buffer.snapshot();
        let history = buffer.undo_transaction_id();
        if range.len() > CLIPBOARD_CHAR_LIMIT {
            return Ok(());
        }
        self.register = snapshot.text_range(range.clone())?;
        self.linewise_register = linewise;
        if linewise && !self.register.ends_with(['\n', '\r']) {
            self.register
                .push_str(snapshot.preferred_line_ending().as_str());
        }
        *copied = Some(self.register.clone());
        if operator == 'y' {
            let cursor = if matches!(self.mode, VimMode::Visual | VimMode::VisualLine) {
                range.start
            } else {
                snapshot.selection().head
            };
            return self.normal(buffer, cursor);
        }
        if operator == 'c' && range.is_empty() {
            return self.begin_insert(buffer, range.start, history);
        }
        let mut replacement = String::new();
        if linewise {
            let row = snapshot.position_of(range.start)?.row;
            if operator == 'c' {
                // Keep the final separator, leaving one editable physical line.
                let last = snapshot.position_of(range.end.saturating_sub(1))?.row;
                range.end = snapshot.line_range(last)?.end;
                replacement = snapshot
                    .line_text(row)?
                    .chars()
                    .take_while(|c| *c == ' ' || *c == '\t')
                    .collect();
            } else if range.end == snapshot.char_count() && range.start > 0 {
                // Deleting the final unterminated line consumes its preceding
                // line ending too, without putting that separator in the yank.
                let last = snapshot.line_range(snapshot.line_count() - 1)?;
                if !last.is_empty() || range.start == snapshot.char_count() {
                    range.start = snapshot.line_range(row - 1)?.end;
                }
            }
        }
        let cursor = range.start + replacement.chars().count();
        buffer.break_undo_group();
        buffer.edit(
            range,
            &replacement,
            Selection::caret(cursor),
            EditKind::Replace,
            now,
        )?;
        if operator == 'c' {
            self.begin_insert(buffer, cursor, history)
        } else {
            self.normal(buffer, cursor)
        }
    }
    fn paste(
        &mut self,
        buffer: &mut TextBuffer,
        cursor: usize,
        before: bool,
        count: usize,
        now: u64,
    ) -> Result<(), EditorError> {
        if self.register.is_empty()
            || self.register.len().saturating_mul(count) > CLIPBOARD_CHAR_LIMIT
        {
            return Ok(());
        }
        let snapshot = buffer.snapshot();
        let mut text = self.register.repeat(count);
        let visual = matches!(self.mode, VimMode::Visual | VimMode::VisualLine);
        let (range, offset) = if visual {
            let mut range = snapshot.selection().range();
            if self.mode == VimMode::VisualLine && !self.linewise_register && !range.is_empty() {
                let last = snapshot.position_of(range.end - 1)?.row;
                range.end = snapshot.line_range(last)?.end;
            }
            (range, 0)
        } else if self.linewise_register {
            let row = snapshot.position_of(cursor)?.row;
            let line = snapshot.line_range(row)?;
            if before {
                (line.start..line.start, 0)
            } else if row + 1 < snapshot.line_count() {
                let next = snapshot.line_range(row + 1)?.start;
                (next..next, 0)
            } else {
                let newline = snapshot.preferred_line_ending().as_str();
                let suffix = text.strip_suffix(newline).unwrap_or(&text);
                text = format!("{newline}{suffix}");
                (line.end..line.end, newline.chars().count())
            }
        } else {
            let position = if before {
                cursor
            } else {
                snapshot
                    .next_grapheme(cursor)?
                    .min(snapshot.line_range(snapshot.position_of(cursor)?.row)?.end)
            };
            (position..position, text.chars().count().saturating_sub(1))
        };
        let target = range.start + offset;
        buffer.break_undo_group();
        buffer.edit(range, &text, Selection::caret(target), EditKind::Paste, now)?;
        self.normal(buffer, target)
    }
}

fn normal_offset(snapshot: &BufferSnapshot, offset: usize) -> Result<usize, EditorError> {
    let offset = snapshot.snap_grapheme_backward(offset.min(snapshot.char_count()))?;
    let range = snapshot.line_range(snapshot.position_of(offset)?.row)?;
    if offset >= range.end && !range.is_empty() {
        snapshot.prev_grapheme(range.end)
    } else {
        Ok(offset)
    }
}
fn first_nonblank(snapshot: &BufferSnapshot, cursor: usize) -> Result<usize, EditorError> {
    let range = snapshot.line_range(snapshot.position_of(cursor)?.row)?;
    Ok(range.start
        + snapshot
            .text_range(range)?
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .count())
}
fn full_lines(snapshot: &BufferSnapshot, a: usize, b: usize) -> Result<Range<usize>, EditorError> {
    let first = snapshot.position_of(a.min(b))?.row;
    let last = snapshot.position_of(a.max(b))?.row;
    Ok(
        snapshot.line_range(first)?.start..if last + 1 < snapshot.line_count() {
            snapshot.line_range(last + 1)?.start
        } else {
            snapshot.char_count()
        },
    )
}
fn class(snapshot: &BufferSnapshot, offset: usize) -> Result<u8, EditorError> {
    let next = snapshot.next_grapheme(offset)?;
    let text = snapshot.text_range(offset..next)?;
    Ok(match text.chars().next() {
        None => 0,
        Some(c) if c.is_whitespace() => 0,
        Some(c) if c.is_alphanumeric() || c == '_' => 1,
        _ => 2,
    })
}
fn word_motion(
    snapshot: &BufferSnapshot,
    cursor: usize,
    command: char,
    budget: &mut usize,
) -> Result<usize, EditorError> {
    let mut at = cursor;
    if command == 'b' {
        at = snapshot.prev_grapheme(at)?;
        while at > 0 && class(snapshot, at)? == 0 && *budget > 0 {
            at = snapshot.prev_grapheme(at)?;
            *budget -= 1;
        }
        let kind = class(snapshot, at)?;
        while at > 0 && *budget > 0 {
            let previous = snapshot.prev_grapheme(at)?;
            if class(snapshot, previous)? != kind {
                break;
            }
            at = previous;
            *budget -= 1;
        }
        return Ok(at);
    }
    if command == 'w' {
        let kind = class(snapshot, at)?;
        while at < snapshot.char_count() && class(snapshot, at)? == kind && *budget > 0 {
            at = snapshot.next_grapheme(at)?;
            *budget -= 1;
        }
        while at < snapshot.char_count() && class(snapshot, at)? == 0 && *budget > 0 {
            at = snapshot.next_grapheme(at)?;
            *budget -= 1;
        }
        return Ok(at);
    }
    // e is inclusive and advances when already on a word's final grapheme.
    at = snapshot.next_grapheme(at)?;
    while at < snapshot.char_count() && class(snapshot, at)? == 0 && *budget > 0 {
        at = snapshot.next_grapheme(at)?;
        *budget -= 1;
    }
    let kind = class(snapshot, at)?;
    while at < snapshot.char_count() && *budget > 0 {
        let next = snapshot.next_grapheme(at)?;
        if next >= snapshot.char_count() || class(snapshot, next)? != kind {
            break;
        }
        at = next;
        *budget -= 1;
    }
    Ok(at)
}
