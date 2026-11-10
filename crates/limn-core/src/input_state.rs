use super::*;

/// A field's value as limn lays it out, in a tree told to
/// (`UiTree::set_text_layout`), and where it was last painted.
#[derive(Clone)]
pub(super) struct FieldLayout {
    pub(super) style: crate::TextStyle,
    /// The width a text area breaks its lines at; a single line breaks none.
    pub(super) width: Option<f32>,
    /// The value as shown, a password masked: never the placeholder.
    pub(super) layout: crate::TextLayout,
    /// Its top-left corner in the tree, scrolled.
    pub(super) origin: Point,
}

pub(super) struct InputState {
    pub(super) paste_epoch: Arc<()>,
    pub(super) paste_kind: Option<HitKind>,
    pub(super) value: String,
    pub(super) cursor: usize,
    pub(super) anchor: Option<usize>,
    pub(super) view_start: usize,
    pub(super) content_x: f32,
    pub(super) content_y: f32,
    pub(super) char_width: f32,
    pub(super) content_width: f32,
    pub(super) font_size: f32,
    pub(super) font_family: FontFamily,
    pub(super) measurer: Option<Arc<TextMeasurer>>,
    pub(super) line_height: f32,
    pub(super) visible_lines: usize,
    pub(super) scroll_line: usize,
    /// The scroll position last painted and when it last moved, for the
    /// scrollbar that shows while a text area scrolls.
    pub(super) painted_scroll_line: usize,
    pub(super) scrolled_at: Option<Instant>,
    pub(super) keep_cursor_visible: bool,
    pub(super) desired_column: Option<usize>,
    /// Laid out by limn: what it painted, and where Up and Down aim.
    pub(super) laid_out: Option<FieldLayout>,
    pub(super) desired_x: Option<f32>,
    /// What an input method is composing at the caret, shown in place of
    /// the selection until it commits: not yet part of the value.
    pub(super) preedit: Option<String>,
    /// Where it was last painted with the focus, for an input method: its
    /// visible part and its caret.
    pub(super) ime: Option<(Rect, Rect)>,
    /// How far a single line laid out by limn is scrolled to show the caret.
    pub(super) scroll_x: f32,
    pub(super) preserve_line_endings: bool,
    /// Enter submits and Shift+Enter breaks the line.
    pub(super) enter_submits: bool,
    pub(super) password: bool,
}

impl InputState {
    pub(super) fn new(value: String) -> Self {
        let cursor = value.chars().count();
        Self {
            paste_epoch: Arc::new(()),
            paste_kind: None,
            value,
            cursor,
            anchor: None,
            view_start: 0,
            content_x: 0.0,
            content_y: 0.0,
            char_width: FontMetrics::default().char_width,
            content_width: 8.0,
            font_size: DEFAULT_FONT_SIZE,
            font_family: FontFamily::Monospace,
            measurer: None,
            line_height: FontMetrics::default().line_height,
            visible_lines: 1,
            scroll_line: 0,
            painted_scroll_line: 0,
            scrolled_at: None,
            keep_cursor_visible: true,
            desired_column: None,
            laid_out: None,
            desired_x: None,
            preedit: None,
            ime: None,
            scroll_x: 0.0,
            preserve_line_endings: false,
            enter_submits: false,
            password: false,
        }
    }

    pub(super) fn selected_range(&self) -> Option<(usize, usize)> {
        self.anchor
            .filter(|anchor| *anchor != self.cursor)
            .map(|anchor| (anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    pub(super) fn set_selection(&mut self, cursor: usize, anchor: Option<usize>) {
        if self.cursor != cursor || self.anchor != anchor {
            self.paste_epoch = Arc::new(());
            self.cursor = cursor;
            self.anchor = anchor;
        }
    }

    pub(super) fn observe_paste_kind(&mut self, kind: HitKind) {
        if self.paste_kind != Some(kind) {
            self.paste_epoch = Arc::new(());
            self.paste_kind = Some(kind);
        }
    }

    pub(super) fn width(&self, value: &str) -> f32 {
        let masked;
        let value = if self.password {
            masked = "•".repeat(value.chars().count());
            masked.as_str()
        } else {
            value
        };
        measured_width(
            self.measurer.as_deref(),
            value,
            self.font_size,
            self.font_family,
            self.char_width,
        )
    }

    /// The value as it is shown: a password as one dot a character.
    pub(super) fn shown(&self) -> Cow<'_, str> {
        if self.password {
            Cow::Owned("•".repeat(self.value.chars().count()))
        } else {
            Cow::Borrowed(&self.value)
        }
    }

    /// The value laid out as last painted, laid out again when an edit has
    /// changed it since, and where it was painted. `None` while the tree
    /// measures text through the host.
    fn layout(&self) -> Option<(crate::TextLayout, Point)> {
        let field = self.laid_out.as_ref()?;
        let shown = self.shown();
        if field.layout.text() == shown {
            return Some((field.layout.clone(), field.origin));
        }
        let layout = crate::TextLayout::new(shown.into_owned(), field.style);
        Some((field.width.map_or_else(|| layout.clone(), |width| layout.wrapped(width)), field.origin))
    }

    /// The value with what an input method composes in place of the
    /// selection, and the composition's bytes in it, the caret at their end.
    /// `None` while nothing is composed, and in a password.
    pub(super) fn composed(&self) -> Option<(String, std::ops::Range<usize>)> {
        let preedit = self.preedit.as_deref().filter(|_| !self.password)?;
        let (start, end) = self.selected_range().unwrap_or((self.cursor, self.cursor));
        let (start, end) = (byte_index(&self.value, start), byte_index(&self.value, end));
        let text = [&self.value[..start], preedit, &self.value[end..]].concat();
        Some((text, start..start + preedit.len()))
    }

    /// Delete what an input method asks around the caret: `before`
    /// characters before it and `after` after it.
    pub(super) fn delete_around(&mut self, before: usize, after: usize) -> bool {
        let start = self.cursor.saturating_sub(before);
        let end = (self.cursor + after).min(self.value.chars().count());
        if start == end {
            return false;
        }
        let bytes = byte_index(&self.value, start)..byte_index(&self.value, end);
        self.value.replace_range(bytes, "");
        self.paste_epoch = Arc::new(());
        self.cursor = start;
        self.anchor = None;
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }

    /// The line of `layout` the caret is on, and the caret.
    pub(super) fn caret_line(&self, layout: &crate::TextLayout) -> (usize, Rect) {
        let caret = layout.caret(byte_index(&self.shown(), self.cursor));
        let row = layout.lines().take_while(|line| line.rect.y <= caret.y + caret.height / 2.0).count();
        (row.saturating_sub(1), caret)
    }

    pub(super) fn visual_lines(&self) -> Vec<VisualLine> {
        visual_lines_measured_endings(
            &self.value,
            self.content_width,
            self.preserve_line_endings,
            |value| self.width(value),
        )
    }

    pub(super) fn move_cursor(&mut self, next: usize, shift: bool) {
        let anchor = shift.then_some(self.anchor.unwrap_or(self.cursor));
        self.set_selection(next.min(self.value.chars().count()), anchor);
        self.desired_column = None;
        self.desired_x = None;
        self.keep_cursor_visible = true;
    }

    /// The character before the byte at `index` of the value as shown.
    fn character_at(&self, index: usize) -> usize {
        let shown = self.shown();
        shown[..index.min(shown.len())].chars().count()
    }

    pub(super) fn move_vertical(&mut self, direction: i32, shift: bool) {
        if let Some((layout, _)) = self.layout() {
            let (row, caret) = self.caret_line(&layout);
            let goal = self.desired_x.unwrap_or(caret.x);
            let lines: Vec<_> = layout.lines().collect();
            let line = &lines[(row as i64 + i64::from(direction)).clamp(0, lines.len() as i64 - 1) as usize];
            let index = layout.index_at(Point::new(goal, line.rect.y + line.rect.height / 2.0));
            self.move_cursor(self.character_at(index), shift);
            self.desired_x = Some(goal);
            return;
        }
        let lines = self.visual_lines();
        let (row, column) = visual_cursor(&lines, self.cursor);
        let desired = self.desired_column.unwrap_or(column);
        let next_row =
            (row as i64 + direction as i64).clamp(0, lines.len().saturating_sub(1) as i64) as usize;
        let next = lines[next_row].start + desired.min(lines[next_row].end - lines[next_row].start);
        self.move_cursor(next, shift);
        self.desired_column = Some(desired);
    }

    pub(super) fn move_line_edge(&mut self, end: bool, shift: bool) {
        if let Some((layout, _)) = self.layout() {
            let (row, _) = self.caret_line(&layout);
            let lines: Vec<_> = layout.lines().collect();
            let line = &lines[row];
            let index = if end {
                let shown = self.shown();
                let text = &shown[line.range.clone()];
                let kept = text.trim_end_matches(['\n', '\r']);
                if kept.len() < text.len() {
                    // Before the break that ends the line.
                    line.range.start + kept.len()
                } else if row + 1 < lines.len() {
                    // Before the spaces a wrapped line hangs past its end.
                    line.range.start + kept.trim_end().len()
                } else {
                    line.range.end
                }
            } else {
                line.range.start
            };
            self.move_cursor(self.character_at(index), shift);
            return;
        }
        let lines = self.visual_lines();
        let (row, _) = visual_cursor(&lines, self.cursor);
        let next = if end {
            lines[row].end
        } else {
            lines[row].start
        };
        self.move_cursor(next, shift);
    }

    /// The caret position nearest `point`, in characters.
    fn character_at_point(&self, point: Point, multiline: bool) -> usize {
        if let Some((layout, origin)) = self.layout() {
            return self.character_at(layout.index_at(Point::new(point.x - origin.x, point.y - origin.y)));
        }
        if !multiline {
            let x = (point.x - self.content_x).max(0.0);
            let shown = self.value.chars().skip(self.view_start).collect::<String>();
            return self.view_start + nearest_char(&shown, x, |part| self.width(part));
        }
        let lines = self.visual_lines();
        let row = ((point.y - self.content_y).max(0.0) / self.line_height).floor() as usize;
        let row = (self.scroll_line + row).min(lines.len() - 1);
        let column = nearest_char(
            &lines[row].text,
            (point.x - self.content_x).max(0.0),
            |value| self.width(value),
        );
        lines[row].start + column.min(lines[row].end - lines[row].start)
    }

    /// Put the caret nearest `point`; with `extend`, keep the selection's
    /// other end where it was.
    pub(super) fn place_cursor_at(&mut self, point: Point, multiline: bool, extend: bool) {
        let next = self.character_at_point(point, multiline);
        self.move_cursor(next, extend);
    }

    /// Where an arrow takes the caret: a grapheme on, or with `by_word` to
    /// the start of the word before it or the end of the word after it. A
    /// password has no words to show, so a word step goes to its end.
    fn step_target(&self, forward: bool, by_word: bool) -> usize {
        match (by_word, self.password, forward) {
            (true, true, true) => self.value.chars().count(),
            (true, true, false) => 0,
            (true, false, _) => word_step(&self.value, self.cursor, forward),
            (false, ..) => grapheme_step(&self.value, self.cursor, forward),
        }
    }

    /// Left or Right: a selection collapses to the edge the arrow points
    /// at, unless Shift extends it.
    pub(super) fn step(&mut self, forward: bool, by_word: bool, shift: bool) {
        if let Some((start, end)) = self.selected_range().filter(|_| !shift) {
            self.move_cursor(if forward { end } else { start }, false);
            return;
        }
        self.move_cursor(self.step_target(forward, by_word), shift);
    }

    /// Select the word, run of spaces or mark at the caret; a password whole.
    pub(super) fn select_word(&mut self) {
        let (start, end) = if self.password {
            (0, self.value.chars().count())
        } else {
            word_around(&self.value, self.cursor)
        };
        self.select(start, end);
    }

    /// Select the caret's line, up to its break; a single line whole.
    pub(super) fn select_line(&mut self, multiline: bool) {
        let count = self.value.chars().count();
        let (start, end) = if multiline {
            let chars: Vec<char> = self.value.chars().collect();
            let start = chars[..self.cursor.min(count)].iter().rposition(|c| *c == '\n').map_or(0, |at| at + 1);
            let end = chars[self.cursor.min(count)..].iter().position(|c| matches!(c, '\n' | '\r')).map_or(count, |at| self.cursor + at);
            (start, end)
        } else {
            (0, count)
        };
        self.select(start, end);
    }

    fn select(&mut self, start: usize, end: usize) {
        self.set_selection(end, Some(start));
        self.desired_column = None;
        self.desired_x = None;
        self.keep_cursor_visible = true;
    }

    pub(super) fn scroll_lines(&mut self, delta_y: f32) {
        if delta_y == 0.0 {
            return;
        }
        let lines = self.layout().map_or_else(|| self.visual_lines().len(), |(layout, _)| layout.lines().count());
        let maximum = lines.saturating_sub(self.visible_lines);
        let steps = (delta_y.abs() / self.line_height).ceil().max(1.0) as usize;
        self.scroll_line = if delta_y > 0.0 {
            self.scroll_line.saturating_sub(steps)
        } else {
            self.scroll_line.saturating_add(steps).min(maximum)
        };
        self.keep_cursor_visible = false;
    }

    pub(super) fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selected_range() else {
            return false;
        };
        let start_byte = byte_index(&self.value, start);
        let end_byte = byte_index(&self.value, end);
        self.value.replace_range(start_byte..end_byte, "");
        self.paste_epoch = Arc::new(());
        self.cursor = start;
        self.anchor = None;
        self.keep_cursor_visible = true;
        true
    }

    pub(super) fn insert(&mut self, text: &str, multiline: bool) -> bool {
        let sanitized = if multiline && self.preserve_line_endings {
            text.to_owned()
        } else if multiline {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.replace(['\r', '\n'], "")
        };
        if sanitized.is_empty() {
            return false;
        }
        self.delete_selection();
        // Ctrl+A on an empty field (or Shift+Arrow at its boundary) leaves
        // a zero-length selection. Typing must collapse that anchor too,
        // otherwise the next character replaces the one just inserted.
        self.anchor = None;
        let byte = byte_index(&self.value, self.cursor);
        self.value.insert_str(byte, &sanitized);
        self.paste_epoch = Arc::new(());
        self.cursor += sanitized.chars().count();
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }

    /// Delete the selection, or the grapheme (with `by_word`, the word) the
    /// caret is before or after.
    pub(super) fn delete_step(&mut self, forward: bool, by_word: bool) -> bool {
        if self.delete_selection() {
            return true;
        }
        let to = self.step_target(forward, by_word);
        if to == self.cursor {
            return false;
        }
        let (start, end) = (self.cursor.min(to), self.cursor.max(to));
        self.value.replace_range(byte_index(&self.value, start)..byte_index(&self.value, end), "");
        self.paste_epoch = Arc::new(());
        self.cursor = start;
        self.anchor = None;
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }
}

/// The grapheme boundary before (`forward` false) or after the character
/// offset `at`, in characters: an emoji with its modifiers is one step.
fn grapheme_step(value: &str, at: usize, forward: bool) -> usize {
    let byte = byte_index(value, at);
    let next = if forward {
        value[byte..].grapheme_indices(true).nth(1).map_or(value.len(), |(index, _)| byte + index)
    } else {
        value[..byte].grapheme_indices(true).next_back().map_or(0, |(index, _)| index)
    };
    value[..next].chars().count()
}

/// Where Ctrl and an arrow take the caret from `at`: to the start of the
/// word before it, or to the end of the word after it, in characters.
fn word_step(value: &str, at: usize, forward: bool) -> usize {
    let byte = byte_index(value, at);
    let next = if forward {
        value.unicode_word_indices().map(|(index, word)| index + word.len()).find(|end| *end > byte).unwrap_or(value.len())
    } else {
        value.unicode_word_indices().map(|(index, _)| index).take_while(|start| *start < byte).last().unwrap_or(0)
    };
    value[..next].chars().count()
}

/// What a double click at the caret offset `at` selects, in characters: the
/// word, run of spaces or mark there, or the word just before it when the
/// caret stands at that word's end.
fn word_around(value: &str, at: usize) -> (usize, usize) {
    let byte = byte_index(value, at);
    let segments: Vec<(usize, &str)> = value.split_word_bound_indices().collect();
    let containing = |byte: usize| segments.iter().position(|(index, segment)| *index <= byte && byte < index + segment.len());
    let is_word = |segment: &str| segment.chars().next().is_some_and(char::is_alphanumeric);
    let Some(mut found) = containing(byte).or(segments.len().checked_sub(1)) else {
        return (0, 0);
    };
    if !is_word(segments[found].1) && found > 0 && segments[found].0 == byte && is_word(segments[found - 1].1) {
        found -= 1;
    }
    let (index, segment) = segments[found];
    (value[..index].chars().count(), value[..index + segment.len()].chars().count())
}

#[derive(Clone, Debug)]
pub(super) struct VisualLine {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) text: String,
}

fn nearest_char(value: &str, x: f32, mut measure: impl FnMut(&str) -> f32) -> usize {
    if x <= 0.0 {
        return 0;
    }
    let boundaries = value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .collect::<Vec<_>>();
    let mut lo = 0;
    let mut hi = boundaries.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if measure(&value[..boundaries[mid]]) < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let after = lo.min(boundaries.len() - 1);
    if after == 0 {
        return 0;
    }
    let before = after - 1;
    let left = measure(&value[..boundaries[before]]);
    let right = measure(&value[..boundaries[after]]);
    if x - left <= right - x {
        before
    } else {
        after
    }
}
