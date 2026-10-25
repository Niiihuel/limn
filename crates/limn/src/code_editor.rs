//! Original native editor view over a host-owned, bounded text buffer.
//! No file operations or whole-document change payloads occur in this module.
use crate::{
    CodeDiff, CodeDiffLineKind, CodeLanguage, Color, Component, ComponentPalette, DrawCommand,
    Element, FontFamily, Key, Point, Rect, UiAction, UiEvent,
};
use limn_editor::{
    BufferSnapshot, EditKind, EditorCommand, IndentStyle, Position, SearchError, SearchOptions,
    SearchResults, Selection, TextBuffer, VisualMap,
};
use std::sync::{Arc, Mutex};
use web_time::Instant;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

const CLIPBOARD_CHAR_LIMIT: usize = 1_048_576;
#[path = "code_editor_vim.rs"]
mod vim;
pub use vim::VimMode;
#[cfg(test)]
#[path = "code_editor_vim_tests.rs"]
mod vim_tests;
pub(crate) const FONT_SIZE: f32 = 13.0;
const LINE_HEIGHT: f32 = 20.0;

#[cfg(test)]
#[path = "code_editor_commands_tests.rs"]
mod command_tests;
#[cfg(test)]
#[path = "code_editor_structure_tests.rs"]
mod structure_tests;

#[path = "code_editor_selection.rs"]
mod selection;
#[path = "code_editor_structure.rs"]
mod structure;
pub use structure::{
    CodeBreakpoint, CodeBreakpointState, CodeExecutionLine, CodeFoldCandidate, CodeStickyScope,
};
#[path = "code_editor_guides.rs"]
mod guides;
#[path = "code_editor_minimap.rs"]
mod minimap;
#[path = "code_editor_review.rs"]
mod review;
pub use review::{CodeReviewHunk, CodeReviewHunkActions, CodeReviewPosition};

/// The application keeps this controller and decides when/how to save it.
/// Only edits and cheap snapshot acquisition hold its lock.
#[derive(Clone, Debug)]
pub struct EditorController(Arc<Mutex<TextBuffer>>);

impl EditorController {
    /// Execute a user-requested command. Snapshot identity/revision lets the host
    /// validate notifications without copying whole-document text.
    pub fn execute(&self, command: EditorCommand) -> Result<BufferSnapshot, String> {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let timestamp = START
            .get_or_init(Instant::now)
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let mut buffer = self
            .0
            .lock()
            .map_err(|_| "Editor buffer lock is unavailable".to_owned())?;
        buffer
            .execute_command(command, timestamp)
            .map_err(|error| error.to_string())?;
        Ok(buffer.snapshot())
    }
    /// Search the current text; see `BufferSnapshot::search`. A buffer lock that cannot be taken
    /// is reported as `SearchError::InvalidPattern`.
    pub fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResults, SearchError> {
        let snapshot = self.snapshot().map_err(SearchError::InvalidPattern)?;
        snapshot.search(query, options)
    }
    /// Put a buffer behind a controller. Clones of the controller share the buffer.
    pub fn new(buffer: TextBuffer) -> Self {
        Self(Arc::new(Mutex::new(buffer)))
    }
    /// A controller over a buffer the application already shares.
    pub fn from_shared(buffer: Arc<Mutex<TextBuffer>>) -> Self {
        Self(buffer)
    }
    /// The buffer behind this controller, shared and not copied.
    pub fn shared_buffer(&self) -> Arc<Mutex<TextBuffer>> {
        self.0.clone()
    }
    /// The current text and selections. `Err` only when the buffer lock is poisoned.
    pub fn snapshot(&self) -> Result<BufferSnapshot, String> {
        self.0
            .lock()
            .map(|buffer| buffer.snapshot())
            .map_err(|_| "Editor buffer lock is unavailable".to_owned())
    }
    fn state(&self) -> Result<(BufferSnapshot, Selection), String> {
        self.0
            .lock()
            .map(|buffer| (buffer.snapshot(), buffer.selection()))
            .map_err(|_| "Editor buffer lock is unavailable".to_owned())
    }
}

/// A colour for a range of text, in the snapshot's Unicode scalar offsets.
#[derive(Clone, Debug)]
pub struct CodeHighlight {
    /// The characters to colour, end-exclusive.
    pub range: std::ops::Range<usize>,
    /// The colour of the text in `range`, in place of the palette's text colour.
    pub color: Color,
}

/// A confirmed diagnostic expressed in the snapshot's Unicode scalar offsets.
#[derive(Clone, Debug)]
pub struct CodeDiagnostic {
    /// The characters to underline. An empty range is underlined one space wide at its
    /// offset; a range that ends past the document is not drawn.
    pub range: std::ops::Range<usize>,
    /// The colour of the underline.
    pub color: Color,
    /// Shown in a tooltip while the pointer is over the range. Empty shows none.
    pub message: String,
}

/// The shape of `CodeEditor`'s caret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CaretStyle {
    /// A thin bar before the character. The default.
    #[default]
    Line,
    /// The whole cell of the character, translucent.
    Block,
    /// A bar under the character.
    Underline,
}

/// A code editor over a buffer the host owns. An edit reports `UiAction::EditorChanged` and
/// a failed one `UiAction::EditorFailed`.
#[derive(Clone, Debug)]
pub struct CodeEditor {
    /// Identifier of the view: its focus, scroll position and editing state are kept under
    /// it, and its actions carry it.
    pub id: String,
    /// The host's buffer. The editor reads a snapshot of it each frame and edits it in place.
    pub controller: EditorController,
    /// Colours of the surface, the text, the gutter numbers and the marks. Set with `palette`.
    pub palette: ComponentPalette,
    /// Height in logical pixels, status bar included. 320 by default; `height` keeps it at
    /// 24 or more.
    pub height: f32,
    /// Code and gutter size in logical pixels, 13 by default.
    pub font_size: f32,
    /// Paint added and removed row backgrounds. Change markers remain visible.
    pub diff_backgrounds: bool,
    /// Refuse every edit. False by default.
    pub read_only: bool,
    /// Native modal editing, independent of the workbench shortcut profile.
    pub vim_mode: bool,
    /// Show line numbers in the gutter. True by default.
    pub line_numbers: bool,
    /// Paint live cursor/selection and document metadata from this frame's snapshot.
    pub status_bar: bool,
    /// Columns between tab stops, within 1..=32. 4 by default. `tab_width` and `indent_style`
    /// keep it equal to the width of `indent_style`.
    pub tab_width: usize,
    /// What the Tab key inserts and what one indent level is. Four spaces by default.
    pub indent_style: IndentStyle,
    /// Close a typed bracket or quote and delete an empty pair with one backspace. True by
    /// default.
    pub auto_pairs: bool,
    /// Wrap long lines at the width of the editor instead of scrolling sideways. False by
    /// default.
    pub soft_wrap: bool,
    /// A compact, navigable overview. Hidden automatically in narrow editors.
    pub minimap: bool,
    /// Draw a vertical line at each indent level. True by default.
    pub indent_guides: bool,
    /// Tint the caret's row while the editor has focus. True by default.
    pub highlight_current_line: bool,
    /// Mark spaces with `·` and tabs with `→`. False by default.
    pub show_whitespace: bool,
    /// Columns that get a vertical guide, at most 16 within 1..=1024. Set with `rulers`.
    pub rulers: Arc<[usize]>,
    /// How the caret is drawn outside Vim's Normal mode, which always uses a block.
    pub caret_style: CaretStyle,
    /// Let the caret blink. True by default; false keeps it shown.
    pub caret_blink: bool,
    /// Keep revealed carets below a floating surface without changing layout.
    pub reveal_top_inset: f32,
    /// The folded blocks, as ranges of zero-based rows: the first row of each stays visible
    /// and the rows after it, up to the exclusive end, are hidden. Set with `folds`.
    pub folds: Arc<[std::ops::Range<usize>]>,
    /// The document `fold_candidates` and `sticky_scopes` were computed for. Once set, the
    /// gutter keeps a lane for fold controls, even with no candidates. Set with `structure`.
    pub structure_document_id: Option<u64>,
    /// The revision they were computed for. They are ignored for any other snapshot.
    pub structure_revision: Option<u64>,
    /// Blocks that get a fold control in the gutter, at most 16,384. Of several that start
    /// on one row, the longest is kept.
    pub fold_candidates: Arc<[CodeFoldCandidate]>,
    /// Scopes whose first row may be shown as a sticky header, at most 16,384.
    pub sticky_scopes: Arc<[CodeStickyScope]>,
    /// The document `breakpoint_rows` belong to. Set with `breakpoints`.
    pub breakpoint_document_id: Option<u64>,
    /// The revision `breakpoint_rows` belong to. The gutter's breakpoint controls are only
    /// built for that snapshot.
    pub breakpoint_revision: Option<u64>,
    /// Start of the action a click on the breakpoint lane reports, followed by `:` and the
    /// zero-based row. Once set, the gutter keeps that lane; an empty prefix or one over 512
    /// bytes gives a lane without controls.
    pub breakpoint_action_prefix: Option<String>,
    /// Zero-based rows with a breakpoint the debugger has not answered for, drawn as
    /// pending. Rows past the document are ignored.
    pub breakpoint_rows: Arc<[usize]>,
    /// The document `debugger_breakpoints` and `execution_line` describe. Set with
    /// `debugger`.
    pub debugger_document_id: Option<u64>,
    /// The revision they describe. They are ignored for any other snapshot.
    pub debugger_revision: Option<u64>,
    /// Breakpoints as the debugger reports them. One on a row takes the place of that row's
    /// pending mark.
    pub debugger_breakpoints: Arc<[CodeBreakpoint]>,
    /// The row the debugger is stopped at, drawn with an amber band. `None` draws none.
    pub execution_line: Option<CodeExecutionLine>,
    /// Character ranges painted as search matches. Set with `search_matches`.
    pub search_matches: Arc<[std::ops::Range<usize>]>,
    /// The buffer revision `search_matches` were found in; they are painted only at that
    /// revision. No document identifier is compared.
    pub search_revision: Option<u64>,
    /// Entries added to the context menu after a separator, below the text entries. Empty
    /// by default.
    pub context_actions: Vec<crate::ContextMenuEntry>,
    /// The revision `highlights` were computed for. On a later revision of the same document
    /// they stay, moved with the text, on the rows the edits did not change.
    pub highlight_revision: Option<u64>,
    /// The document `highlights` were computed for. For any other, the built-in colours of
    /// `language` are used.
    pub highlight_document_id: Option<u64>,
    /// The host's syntax colours, in place of the built-in ones. Give them sorted by start
    /// offset: unsorted, all of them are scanned for every row. Set with `highlights`.
    pub highlights: Arc<[CodeHighlight]>,
    /// The revision `diagnostics` were computed for. They are drawn only for that snapshot.
    pub diagnostic_revision: Option<u64>,
    /// The document `diagnostics` were computed for.
    pub diagnostic_document_id: Option<u64>,
    /// Underlines with a tooltip. Only the first 4096 are looked at. Set with `diagnostics`.
    pub diagnostics: Arc<[CodeDiagnostic]>,
    /// The language of the built-in syntax colours and the name in the status bar. Plain
    /// text by default.
    pub language: CodeLanguage,
    /// The document `review_hunks` describe. Set with `review`.
    pub review_document_id: Option<u64>,
    /// The revision `review_hunks` describe. They are ignored for any other snapshot.
    pub review_revision: Option<u64>,
    /// The hunks under review: added rows are tinted and removed lines shown between rows.
    pub review_hunks: Arc<[CodeReviewHunk]>,
    /// The buttons of each hunk, at the same index. No hunk gets controls unless there is
    /// exactly one entry per hunk.
    pub review_actions: Arc<[CodeReviewHunkActions]>,
}

impl CodeEditor {
    /// An editor for `controller` with the neutral dark palette: 320 pixels high, editable, line
    /// numbers on, tab width 4 with spaces, auto pairs on; status bar, soft wrap, minimap and
    /// modal editing off.
    pub fn new(id: impl Into<String>, controller: EditorController) -> Self {
        Self {
            id: id.into(),
            controller,
            palette: ComponentPalette::neutral_dark(),
            height: 320.0,
            font_size: FONT_SIZE,
            diff_backgrounds: true,
            read_only: false,
            vim_mode: false,
            line_numbers: true,
            status_bar: false,
            tab_width: 4,
            indent_style: IndentStyle::default(),
            auto_pairs: true,
            soft_wrap: false,
            minimap: false,
            indent_guides: true,
            highlight_current_line: true,
            show_whitespace: false,
            rulers: Arc::from([]),
            caret_style: CaretStyle::Line,
            caret_blink: true,
            reveal_top_inset: 0.0,
            folds: Arc::from([]),
            structure_document_id: None,
            structure_revision: None,
            fold_candidates: Arc::from([]),
            sticky_scopes: Arc::from([]),
            breakpoint_document_id: None,
            breakpoint_revision: None,
            breakpoint_action_prefix: None,
            breakpoint_rows: Arc::from([]),
            debugger_document_id: None,
            debugger_revision: None,
            debugger_breakpoints: Arc::from([]),
            execution_line: None,
            search_matches: Arc::from([]),
            search_revision: None,
            context_actions: Vec::new(),
            highlight_revision: None,
            highlight_document_id: None,
            highlights: Arc::from([]),
            diagnostic_revision: None,
            diagnostic_document_id: None,
            diagnostics: Arc::from([]),
            language: CodeLanguage::Plain,
            review_document_id: None,
            review_revision: None,
            review_hunks: Arc::from([]),
            review_actions: Arc::from([]),
        }
    }
    /// Use this palette for the surface, the text and the editor's marks.
    pub fn palette(mut self, palette: ComponentPalette) -> Self {
        self.palette = palette;
        self
    }
    /// Set code and gutter size, including row layout and pointer positioning.
    /// Nonpositive or nonfinite values are ignored; valid sizes stay within 1..=96.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size.clamp(1.0, 96.0);
        }
        self
    }
    /// Show tinted added and removed row backgrounds. True by default.
    /// Disabling them preserves change markers and review controls.
    pub fn diff_backgrounds(mut self, enabled: bool) -> Self {
        self.diff_backgrounds = enabled;
        self
    }
    pub(crate) fn line_height(&self) -> f32 {
        LINE_HEIGHT * self.font_size / FONT_SIZE
    }
    /// Review rows belong to this exact current snapshot. Removed text is a
    /// non-editable view zone, never part of the buffer or its selection.
    /// Hunks must be ordered and non-overlapping. Invalid or oversized overlays
    /// are omitted atomically (4096 hunks, 65536 removed display rows, 4 MiB).
    pub fn review(
        mut self,
        document_id: u64,
        revision: u64,
        hunks: impl Into<Arc<[CodeReviewHunk]>>,
    ) -> Self {
        self.review_document_id = Some(document_id);
        self.review_revision = Some(revision);
        self.review_hunks = hunks.into();
        self
    }
    /// Host-owned, version-scoped actions, indexed exactly like review hunks.
    /// The view emits these IDs; it never accepts or rewrites a source file.
    pub fn review_actions(mut self, actions: impl Into<Arc<[CodeReviewHunkActions]>>) -> Self {
        self.review_actions = actions.into();
        self
    }
    /// Set the height, in logical pixels, never below 24. A height that is
    /// not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(24.0);
        }
        self
    }
    /// Refuse edits: typing, the input method and the context menu's cut and paste do nothing.
    /// The status bar, when shown, says the document is read-only.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
    /// Turn modal editing on or off. Changing it puts the editor back in Normal mode.
    pub fn vim_mode(mut self, enabled: bool) -> Self {
        self.vim_mode = enabled;
        self
    }
    /// Show line numbers in the gutter. Without them the gutter keeps 8 pixels of padding.
    pub fn line_numbers(mut self, enabled: bool) -> Self {
        self.line_numbers = enabled;
        self
    }
    /// Add a bar 28 pixels high at the bottom, inside the editor's height, with the caret's line
    /// and column, the size of the selection, the language, the encoding and the line ending.
    pub fn status_bar(mut self, enabled: bool) -> Self {
        self.status_bar = enabled;
        self
    }
    /// Set the tab stop in columns, clamped to 1..=32. The indent style keeps its kind and takes
    /// this width.
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab_width = width.clamp(1, 32);
        self.indent_style = match self.indent_style {
            IndentStyle::Spaces(_) => IndentStyle::Spaces(self.tab_width),
            IndentStyle::Tabs(_) => IndentStyle::Tabs(self.tab_width),
        };
        self
    }
    /// Indent with spaces or with tabs. The style's width, clamped to 1..=32, also becomes the
    /// tab stop.
    pub fn indent_style(mut self, style: IndentStyle) -> Self {
        self.tab_width = style.width().clamp(1, 32);
        self.indent_style = match style {
            IndentStyle::Spaces(_) => IndentStyle::Spaces(self.tab_width),
            IndentStyle::Tabs(_) => IndentStyle::Tabs(self.tab_width),
        };
        self
    }
    /// Insert the closing bracket or quote when the opening one is typed, and delete an empty
    /// pair with one backspace. On by default.
    pub fn auto_pairs(mut self, enabled: bool) -> Self {
        self.auto_pairs = enabled;
        self
    }
    /// Wrap long lines at the width of the editor; there is then no horizontal scrolling.
    pub fn soft_wrap(mut self, enabled: bool) -> Self {
        self.soft_wrap = enabled;
        self
    }
    /// Show a 72-pixel overview of the document that scrolls on press and drag. It is left out
    /// when fewer than 380 pixels remain beside the gutter.
    pub fn minimap(mut self, enabled: bool) -> Self {
        self.minimap = enabled;
        self
    }
    /// Draw a faint vertical line at each indent level. On by default.
    pub fn indent_guides(mut self, enabled: bool) -> Self {
        self.indent_guides = enabled;
        self
    }
    /// Tint the caret's row while the editor has focus. On by default.
    pub fn highlight_current_line(mut self, enabled: bool) -> Self {
        self.highlight_current_line = enabled;
        self
    }
    /// Mark every space with a centred `·` and every tab with `→`, in the muted colour.
    pub fn show_whitespace(mut self, enabled: bool) -> Self {
        self.show_whitespace = enabled;
        self
    }
    /// Draw a vertical guide after each of these columns, counted in spaces. Columns outside
    /// 1..=1024 and any past the first 16 are ignored.
    pub fn rulers(mut self, columns: impl IntoIterator<Item = usize>) -> Self {
        self.rulers = columns
            .into_iter()
            .filter(|column| (1..=1024).contains(column))
            .take(16)
            .collect();
        self
    }
    /// Draw the caret as a line, a block or an underline. Vim's Normal mode keeps its block.
    pub fn caret_style(mut self, style: CaretStyle) -> Self {
        self.caret_style = style;
        self
    }
    /// Let the caret blink, as it does by default. Off, it stays shown while focused.
    pub fn caret_blink(mut self, enabled: bool) -> Self {
        self.caret_blink = enabled;
        self
    }
    /// Reserves visibility only when revealing the caret, so opening a floating
    /// find panel does not immediately move the document. The first lines may
    /// temporarily scroll below the surface; clearing the inset removes that gap.
    pub fn reveal_top_inset(mut self, inset: f32) -> Self {
        self.reveal_top_inset = if inset.is_finite() {
            inset.max(0.0)
        } else {
            0.0
        };
        self
    }
    /// Fold these ranges of zero-based rows: the first row of each stays visible and the rows
    /// after it, up to the exclusive end, are hidden.
    pub fn folds(mut self, rows: impl Into<Arc<[std::ops::Range<usize>]>>) -> Self {
        self.folds = rows.into();
        self
    }
    /// Analysis-derived controls are painted only for this exact snapshot.
    /// Rows are zero-based, end-exclusive. Empty data still reserves the fold lane.
    pub fn structure(
        mut self,
        document_id: u64,
        revision: u64,
        folds: impl Into<Arc<[CodeFoldCandidate]>>,
        scopes: impl Into<Arc<[CodeStickyScope]>>,
    ) -> Self {
        self.structure_document_id = Some(document_id);
        self.structure_revision = Some(revision);
        self.fold_candidates = folds.into();
        self.sticky_scopes = scopes.into();
        self
    }
    /// Local, pending breakpoint marks. No debugger or execution claim is implied.
    /// A clicked row emits `{action_prefix}:{zero_based_row}` for host validation.
    pub fn breakpoints(
        mut self,
        document_id: u64,
        revision: u64,
        action_prefix: impl Into<String>,
        rows: impl Into<Arc<[usize]>>,
    ) -> Self {
        self.breakpoint_document_id = Some(document_id);
        self.breakpoint_revision = Some(revision);
        self.breakpoint_action_prefix = Some(action_prefix.into());
        self.breakpoint_rows = rows.into();
        self
    }
    /// Adapter states and execution decoration apply only to this exact source
    /// snapshot. This changes neither cursor selection nor document contents.
    pub fn debugger(
        mut self,
        document_id: u64,
        revision: u64,
        breakpoints: impl Into<Arc<[CodeBreakpoint]>>,
        execution: Option<CodeExecutionLine>,
    ) -> Self {
        self.debugger_document_id = Some(document_id);
        self.debugger_revision = Some(revision);
        self.debugger_breakpoints = breakpoints.into();
        self.execution_line = execution;
        self
    }
    /// Mark these character ranges as search matches. They are painted only while the buffer is
    /// at `revision`.
    pub fn search_matches(
        mut self,
        revision: u64,
        ranges: impl Into<Arc<[std::ops::Range<usize>]>>,
    ) -> Self {
        self.search_revision = Some(revision);
        self.search_matches = ranges.into();
        self
    }
    /// Entries for the editor's context menu, added after a separator below the text entries.
    pub fn context_actions(mut self, actions: Vec<crate::ContextMenuEntry>) -> Self {
        self.context_actions = actions;
        self
    }
    /// The language for the built-in syntax colours and for the status bar. The default is
    /// plain text.
    pub fn language(mut self, language: CodeLanguage) -> Self {
        self.language = language;
        self
    }
    /// Syntax colours computed for the snapshot with this `document_id` and `revision`. They
    /// are never painted onto another document. After an edit, spans already shown stay on the
    /// rows the edit did not change, moved with the text; the changed rows wait for new spans.
    pub fn highlights(
        mut self,
        document_id: u64,
        revision: u64,
        highlights: impl Into<Arc<[CodeHighlight]>>,
    ) -> Self {
        self.highlight_revision = Some(revision);
        self.highlight_document_id = Some(document_id);
        self.highlights = highlights.into();
        self
    }
    /// Diagnostics are valid only for the exact document snapshot analyzed.
    pub fn diagnostics(
        mut self,
        document_id: u64,
        revision: u64,
        diagnostics: impl Into<Arc<[CodeDiagnostic]>>,
    ) -> Self {
        self.diagnostic_document_id = Some(document_id);
        self.diagnostic_revision = Some(revision);
        self.diagnostics = diagnostics.into();
        self
    }
}

impl Component for CodeEditor {
    fn render(self) -> Element {
        Element::code_editor(self.clone())
            .size(None, Some(self.height))
            .font_size(self.font_size)
            .background(self.palette.surface)
            .foreground(self.palette.text)
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct CodeViewport {
    pub rect: Rect,
    pub body: Rect,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub dragging: bool,
    pub desired_x: Option<f32>,
    pub reveal_caret: bool,
    pub scrollbar_drag: Option<(bool, f32)>,
    minimap: Option<minimap::Layout>,
    minimap_drag: bool,
    scrollbar_pointer: Option<Point>,
}

impl CodeViewport {
    pub(crate) fn set_hovered_point(&mut self, point: Option<Point>) {
        self.scrollbar_pointer = point;
    }
    pub(crate) fn cancel_pointer(&mut self) {
        self.dragging = false;
        self.scrollbar_drag = None;
        self.minimap_drag = false;
        self.scrollbar_pointer = None;
    }
    /// Start a drag of a scrollbar or of the minimap when `point` is on one, and report whether
    /// it did. A press beside the thumb centres the thumb on the pointer.
    pub fn begin_scrollbar(&mut self, point: Point) -> bool {
        if let Some(layout) = self.minimap.filter(|layout| layout.rect.contains(point)) {
            let thumb = layout.thumb(self);
            let grab = if thumb.contains(point) {
                point.y - thumb.y
            } else {
                thumb.height / 2.0
            };
            self.minimap_drag = true;
            self.dragging = false;
            self.scrollbar_drag = Some((true, grab));
            self.drag_scrollbar(point, true);
            return true;
        }
        let vertical = self.max_y > self.min_y
            && point.x >= self.rect.x + self.rect.width - 8.0
            && point.y >= self.body.y
            && point.y <= self.body.y + self.body.height;
        let horizontal = self.max_x > 0.0
            && point.y >= self.rect.y + self.rect.height - 8.0
            && point.x >= self.body.x
            && point.x <= self.body.x + self.body.width;
        if !vertical && !horizontal {
            return false;
        }
        let (start, length, maximum, offset, position) = if vertical {
            (
                self.body.y,
                self.body.height,
                self.max_y - self.min_y,
                self.scroll_y - self.min_y,
                point.y,
            )
        } else {
            (
                self.body.x,
                self.body.width,
                self.max_x,
                self.scroll_x,
                point.x,
            )
        };
        let thumb = (length * length / (length + maximum)).max(16.0).min(length);
        let top = start + (length - thumb) * offset / maximum;
        let grab = if position >= top && position <= top + thumb {
            position - top
        } else {
            thumb / 2.0
        };
        self.scrollbar_drag = Some((vertical, grab));
        self.minimap_drag = false;
        self.drag_scrollbar(point, true);
        true
    }
    /// Scroll so that the grabbed thumb follows `point`. Pass `dragging: false` on release: the
    /// position is applied once more and the drag ends.
    pub fn drag_scrollbar(&mut self, point: Point, dragging: bool) {
        if self.minimap_drag {
            if let (Some(layout), Some((_, grab))) = (self.minimap, self.scrollbar_drag) {
                let thumb = layout.thumb(self);
                let travel = layout.content_height - thumb.height;
                let fraction = if travel > 0.0 {
                    ((point.y - layout.rect.y - grab) / travel).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                self.scroll_y = self.min_y + fraction * (self.max_y - self.min_y);
                self.reveal_caret = false;
            }
            if !dragging {
                self.minimap_drag = false;
                self.scrollbar_drag = None;
            }
            return;
        }
        if let Some((vertical, grab)) = self.scrollbar_drag {
            let (start, length, maximum, position) = if vertical {
                (
                    self.body.y,
                    self.body.height,
                    self.max_y - self.min_y,
                    point.y,
                )
            } else {
                (self.body.x, self.body.width, self.max_x, point.x)
            };
            let thumb = (length * length / (length + maximum)).max(16.0).min(length);
            let offset = if length > thumb {
                (position - start - grab) / (length - thumb) * maximum
            } else {
                0.0
            };
            if vertical {
                self.scroll_y = self.min_y + offset.clamp(0.0, maximum);
            } else {
                self.scroll_x = offset.clamp(0.0, maximum);
            }
        }
        if !dragging {
            self.scrollbar_drag = None;
        }
    }
    /// Scroll by a wheel delta in logical pixels; a positive value moves towards the start. The
    /// result is clamped to the scrollable range and a non-finite delta is ignored.
    pub fn scroll(&mut self, x: f32, y: f32) {
        if x.is_finite() {
            self.scroll_x = (self.scroll_x - x).clamp(0.0, self.max_x);
        }
        if y.is_finite() {
            self.scroll_y = (self.scroll_y - y).clamp(self.min_y, self.max_y);
        }
    }
}

type CachedVisualMap = (u64, usize, usize, Arc<[std::ops::Range<usize>]>, VisualMap);

pub(crate) struct EditorView {
    paste_epoch: Arc<()>,
    paste_seen: bool,
    paste_present: bool,
    pub editor: CodeEditor,
    pub viewport: CodeViewport,
    document_id: Option<u64>,
    horizontal_extent_key: Option<(u64, u64, usize, u32, u32, u64)>,
    started: Instant,
    syntax: crate::code_syntax::SyntaxCache,
    visual_map: Option<CachedVisualMap>,
    visual_snapshot: Option<BufferSnapshot>,
    rectangle_anchor: Option<Position>,
    modified_drag: Option<ModifiedDrag>,
    composition: Option<CompositionState>,
    ime_rects: Option<(Rect, Rect)>,
    guide_cache: guides::IndentGuideCache,
    structure_cache: structure::StructureCache,
    structure_controls: Vec<structure::StructureControl>,
    breakpoint_pointer: Option<Point>,
    content_visible: Rect,
    minimap_cache: minimap::Cache,
    highlight_index: HighlightIndex,
    /// The diagnostic under the pointer and when the pointer reached it.
    diagnostic_hover: Option<(std::ops::Range<usize>, Instant)>,
    diagnostic_tooltip: Option<(Rect, String)>,
    minimap_hovered: bool,
    visual_map_generation: u64,
    vim: vim::VimState,
    review_layout: review::Layout,
    review_reveal: Option<(u64, u64, usize)>,
    review_interaction: review::Interaction,
}
/// Finds the spans of a document-wide highlight list that can colour one row.
/// Spans keep their order, so the first span containing an offset is the same
/// one a scan of the whole list would find.
#[derive(Default)]
struct HighlightIndex {
    key: Option<(u64, u64, usize, usize)>,
    /// Largest span end up to and including each position. Empty when the
    /// list is not ordered by start, in which case every row gets all of it.
    max_end: Vec<usize>,
    /// The text the indexed spans describe.
    source: Option<BufferSnapshot>,
    /// A later revision the spans are carried onto, with its changed rows.
    carried: Option<(u64, (usize, usize, usize))>,
}
impl HighlightIndex {
    /// Whether the host's spans cover `snapshot`, exactly or carried over from
    /// the revision they were computed for.
    fn follow(&mut self, editor: &CodeEditor, snapshot: &BufferSnapshot) -> bool {
        let document = snapshot.document_id();
        let Some(revision) = editor
            .highlight_revision
            .filter(|_| editor.highlight_document_id == Some(document))
        else {
            self.carried = None;
            return false;
        };
        let spans = &editor.highlights;
        if revision == snapshot.revision() {
            self.sync((document, revision), spans);
            self.source = Some(snapshot.clone());
            self.carried = None;
            return true;
        }
        let indexed = self.key == Some((document, revision, spans.as_ptr() as usize, spans.len()));
        let Some(source) = self.source.as_ref().filter(|_| indexed) else {
            self.carried = None;
            return false;
        };
        if self.carried.is_none_or(|(carried, _)| carried != snapshot.revision()) {
            self.carried = limn_editor::changed_rows(source, snapshot)
                .ok()
                .map(|rows| (snapshot.revision(), rows));
        }
        self.carried.is_some()
    }
    /// Spans for one row of `snapshot`; `range` is the part of it being drawn.
    fn lookup<'a>(
        &'a self,
        editor: &'a CodeEditor,
        syntax: &'a crate::code_syntax::SyntaxCache,
        snapshot: &BufferSnapshot,
        row: usize,
        range: std::ops::Range<usize>,
    ) -> std::borrow::Cow<'a, [CodeHighlight]> {
        use std::borrow::Cow;
        let spans = editor.highlights.as_ref();
        if editor.highlight_document_id == Some(snapshot.document_id())
            && editor.highlight_revision == Some(snapshot.revision())
        {
            return Cow::Borrowed(self.row(spans, range));
        }
        let (Some(source), Some((_, (prefix, old_end, new_end)))) = (
            &self.source,
            self.carried
                .filter(|(revision, _)| *revision == snapshot.revision()),
        ) else {
            return Cow::Borrowed(syntax.line(snapshot, editor.language, row));
        };
        if row < prefix {
            return Cow::Borrowed(self.row(spans, range));
        }
        let old_row = if row >= new_end {
            row - new_end + old_end
        } else if row < old_end {
            row
        } else {
            return Cow::Borrowed(&[]);
        };
        let (Ok(old), Ok(new)) = (source.line_range(old_row), snapshot.line_range(row)) else {
            return Cow::Borrowed(&[]);
        };
        let moved = |offset: usize| (offset.clamp(old.start, old.end) - old.start + new.start).min(new.end);
        Cow::Owned(
            self.row(spans, old.clone())
                .iter()
                .map(|span| CodeHighlight {
                    range: moved(span.range.start)..moved(span.range.end),
                    color: span.color,
                })
                .filter(|span| span.range.start < span.range.end)
                .collect(),
        )
    }
    fn sync(&mut self, document: (u64, u64), spans: &Arc<[CodeHighlight]>) {
        let key = (document.0, document.1, spans.as_ptr() as usize, spans.len());
        if self.key == Some(key) {
            return;
        }
        self.key = Some(key);
        self.max_end.clear();
        if spans
            .windows(2)
            .any(|pair| pair[0].range.start > pair[1].range.start)
        {
            return;
        }
        let mut furthest = 0;
        self.max_end.extend(spans.iter().map(|span| {
            furthest = furthest.max(span.range.end);
            furthest
        }));
    }
    fn row<'a>(
        &self,
        spans: &'a [CodeHighlight],
        row: std::ops::Range<usize>,
    ) -> &'a [CodeHighlight] {
        if self.max_end.len() != spans.len() {
            return spans;
        }
        let last = spans.partition_point(|span| span.range.start < row.end);
        let first = self.max_end.partition_point(|end| *end <= row.start);
        &spans[first.min(last)..last]
    }
}
struct ModifiedDrag {
    point: Point,
    selection: Selection,
    secondary: Vec<Selection>,
    clicks: u8,
}
struct CompositionState {
    text: String,
    document_id: u64,
    revision: u64,
    selections: Vec<Selection>,
    cancelled: bool,
}

impl EditorView {
    /// Fresh view state for `editor`: nothing scrolled, no caches and Normal mode.
    pub fn new(editor: CodeEditor, now: Instant) -> Self {
        let document_id = editor
            .controller
            .snapshot()
            .ok()
            .map(|snapshot| snapshot.document_id());
        Self {
            paste_epoch: Arc::new(()),
            paste_seen: false,
            paste_present: false,
            editor,
            viewport: CodeViewport::default(),
            document_id,
            horizontal_extent_key: None,
            started: now,
            syntax: crate::code_syntax::SyntaxCache::new(),
            visual_map: None,
            visual_snapshot: None,
            rectangle_anchor: None,
            modified_drag: None,
            composition: None,
            ime_rects: None,
            guide_cache: Default::default(),
            structure_cache: Default::default(),
            structure_controls: Vec::new(),
            breakpoint_pointer: None,
            content_visible: Rect::new(0.0, 0.0, 0.0, 0.0),
            minimap_cache: Default::default(),
            highlight_index: Default::default(),
            diagnostic_hover: None,
            diagnostic_tooltip: None,
            minimap_hovered: false,
            visual_map_generation: 0,
            vim: Default::default(),
            review_layout: Default::default(),
            review_reveal: None,
            review_interaction: review::Interaction::new(now),
        }
    }
    /// Take this frame's description of the editor. A different document resets the viewport,
    /// the caches and the composition; that, or a change of `vim_mode`, resets modal editing.
    pub fn update(&mut self, editor: CodeEditor) {
        self.paste_seen = true;
        let next_id = editor
            .controller
            .snapshot()
            .ok()
            .map(|snapshot| snapshot.document_id());
        if next_id != self.document_id
            || !Arc::ptr_eq(&editor.controller.0, &self.editor.controller.0)
            || editor.read_only != self.editor.read_only
            || editor.vim_mode != self.editor.vim_mode
        {
            self.paste_epoch = Arc::new(());
        }
        if next_id != self.document_id || editor.vim_mode != self.editor.vim_mode {
            self.vim = Default::default();
        }
        if next_id != self.document_id {
            self.viewport = CodeViewport::default();
            self.document_id = next_id;
            self.horizontal_extent_key = None;
            self.visual_map = None;
            self.visual_snapshot = None;
            self.review_layout = Default::default();
            self.review_reveal = None;
            self.review_interaction = review::Interaction::new(Instant::now());
            self.minimap_cache = Default::default();
            self.modified_drag = None;
            self.cancel_composition();
            self.ime_rects = None;
        }
        if editor.font_size != self.editor.font_size {
            self.horizontal_extent_key = None;
            self.visual_map = None;
            self.visual_snapshot = None;
            self.viewport.max_x = 0.0;
            self.viewport.reveal_caret = true;
        }
        self.editor = editor;
    }
    pub(crate) fn content_visible(&self) -> Rect {
        self.content_visible
    }
    pub(crate) fn structure_controls(&self) -> &[structure::StructureControl] {
        &self.structure_controls
    }
    /// Preview only this editor's currently exposed gutter. The pointer is
    /// re-tested against this frame's row rectangles after scrolling/folding;
    /// a menu or other overlay's hover target suppresses the preview.
    pub(crate) fn set_hovered_control(&mut self, id: Option<&str>, pointer: Option<Point>) {
        self.viewport.set_hovered_point(
            (id == Some(self.editor.id.as_str()))
                .then_some(pointer)
                .flatten(),
        );
        self.minimap_hovered = id == Some(self.editor.id.as_str())
            && pointer.is_some_and(|point| {
                self.viewport
                    .minimap
                    .is_some_and(|layout| layout.rect.contains(point))
            });
        self.breakpoint_pointer = id.and_then(|id| {
            self.editor
                .breakpoint_action_prefix
                .as_deref()
                .and_then(|prefix| id.strip_prefix(prefix))
                .filter(|suffix| suffix.starts_with(':'))
                .and(pointer)
        });
    }
    pub(crate) fn owns_structure_control(&self, id: &str, position: Point) -> bool {
        self.structure_controls.iter().any(|control| {
            control.action_id == id
                && control.rect.contains(position)
                && control.clip.contains(position)
        })
    }
    fn fail(&self, error: impl ToString) -> UiAction {
        UiAction::EditorFailed {
            id: self.editor.id.clone(),
            error: error.to_string(),
        }
    }
    /// True while a repaint is due without any input: syntax colours are still being computed,
    /// or the pointer rests on a review hunk or a diagnostic whose popup has not appeared yet.
    pub fn syntax_pending(&self) -> bool {
        self.syntax.pending()
            || self.review_interaction.hover_pending()
            || self.diagnostic_hover.is_some() && self.diagnostic_tooltip.is_none()
    }
    /// The message to show once the pointer has rested on a diagnostic.
    pub fn diagnostic_tooltip(&self) -> Option<(Rect, String)> {
        self.diagnostic_tooltip.clone()
    }
    fn navigation_target(
        &mut self,
        snapshot: &BufferSnapshot,
        selection: Selection,
        key: Key,
        shift: bool,
        ctrl: bool,
    ) -> Option<usize> {
        let start = selection.anchor.min(selection.head);
        let end = selection.anchor.max(selection.head);
        let position = snapshot.position_of(selection.head).ok()?;
        match key {
            Key::ArrowLeft if !shift && start != end => Some(start),
            Key::ArrowRight if !shift && start != end => Some(end),
            Key::ArrowLeft if ctrl => snapshot.previous_word_start(selection.head).ok(),
            Key::ArrowRight if ctrl => snapshot.next_word_end(selection.head).ok(),
            Key::ArrowLeft => snapshot.prev_grapheme(selection.head).ok(),
            Key::ArrowRight => snapshot.next_grapheme(selection.head).ok(),
            Key::Home if ctrl => Some(0),
            Key::End if ctrl => Some(snapshot.char_count()),
            Key::Home => snapshot.smart_line_start(selection.head).ok(),
            Key::End => snapshot
                .line_range(position.row)
                .ok()
                .map(|range| range.end),
            Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown => {
                if let Some((_, _, _, _, map)) = &self.visual_map {
                    let current = map.row_for_offset(selection.head);
                    let visual = &map.rows[current];
                    let column = snapshot
                        .display_column(selection.head, self.editor.tab_width)
                        .ok()?
                        .saturating_sub(visual.start_column);
                    let desired = *self.viewport.desired_x.get_or_insert(column as f32) as usize;
                    let amount = if matches!(key, Key::PageUp | Key::PageDown) {
                        (self.viewport.body.height / self.editor.line_height()).floor().max(1.0) as usize
                    } else {
                        1
                    };
                    let index = if matches!(key, Key::PageUp | Key::PageDown) {
                        self.review_page_target(current, amount, key == Key::PageUp, map.rows.len())
                    } else if matches!(key, Key::ArrowUp) {
                        current.saturating_sub(amount)
                    } else {
                        current
                            .saturating_add(amount)
                            .min(map.rows.len().saturating_sub(1))
                    };
                    let row = &map.rows[index];
                    let target = snapshot
                        .offset_for_display_column(
                            row.physical_row,
                            row.start_column + desired,
                            self.editor.tab_width,
                        )
                        .ok()?
                        .clamp(row.range.start, row.range.end);
                    return Some(target);
                }
                let column = snapshot
                    .display_column(selection.head, self.editor.tab_width)
                    .ok()?;
                let x = *self.viewport.desired_x.get_or_insert(column as f32);
                let amount = if matches!(key, Key::PageUp | Key::PageDown) {
                    (self.viewport.body.height / self.editor.line_height()).floor().max(1.0) as usize
                } else {
                    1
                };
                let row = if matches!(key, Key::PageUp | Key::PageDown) {
                    self.review_page_target(
                        position.row,
                        amount,
                        key == Key::PageUp,
                        snapshot.line_count(),
                    )
                } else if matches!(key, Key::ArrowUp) {
                    position.row.saturating_sub(amount)
                } else {
                    position
                        .row
                        .saturating_add(amount)
                        .min(snapshot.line_count().saturating_sub(1))
                };
                snapshot
                    .offset_for_display_column(row, x as usize, self.editor.tab_width)
                    .ok()
            }
            _ => None,
        }
    }
    pub(crate) fn begin_paste_layout(&mut self) {
        self.paste_seen = false;
    }

    pub(crate) fn finish_paste_layout(&mut self, visible: bool) {
        let present = visible && self.paste_seen;
        if self.paste_present != present {
            self.paste_epoch = Arc::new(());
            self.paste_present = present;
        }
    }

    fn accepts_captured_paste(&self) -> bool {
        self.paste_present
            && !self.editor.read_only
            && (!self.editor.vim_mode || self.vim.mode == VimMode::Insert)
            && self
                .composition
                .as_ref()
                .is_none_or(|state| state.cancelled)
    }

    pub(crate) fn capture_paste_state(&self) -> Option<(Arc<()>, limn_editor::EditStateToken)> {
        if !self.accepts_captured_paste() {
            return None;
        }
        let buffer = self.editor.controller.0.try_lock().ok()?;
        Some((self.paste_epoch.clone(), buffer.edit_state_token()))
    }

    pub(crate) fn paste_captured(
        &mut self,
        epoch: &Arc<()>,
        state: &limn_editor::EditStateToken,
        ticket: &std::sync::atomic::AtomicBool,
        text: &str,
        now: Instant,
    ) -> Option<Vec<UiAction>> {
        use std::sync::atomic::Ordering;
        if !self.accepts_captured_paste() || !Arc::ptr_eq(epoch, &self.paste_epoch) {
            return None;
        }
        let controller = self.editor.controller.clone();
        let Ok(mut buffer) = controller.0.try_lock() else {
            return None;
        };
        // Authority is checked and claimed under the same guard as the edit.
        if *state != buffer.edit_state_token()
            || ticket
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return None;
        }
        if text.is_empty() {
            return Some(Vec::new());
        }
        self.cancel_composition();
        let snapshot = buffer.snapshot();
        let timestamp = now
            .saturating_duration_since(self.started)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let result = buffer
            .insert(text, EditKind::Paste, timestamp)
            .map_err(|error| error.to_string());
        Some(self.finish_input(&buffer, &snapshot, result, Vec::new(), false))
    }

    /// Apply a key, text, paste or other input event and return the actions it caused. `measure`
    /// gives the width of a string in the editor's font.
    pub fn input(
        &mut self,
        event: &UiEvent,
        now: Instant,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<UiAction> {
        if let Some(actions) = self.vim_input(event, now) {
            return actions;
        }
        self.cancel_composition();
        let controller = self.editor.controller.clone();
        let Ok((snapshot, selection)) = controller.state() else {
            return vec![self.fail("Editor buffer lock is unavailable")];
        };
        let before = snapshot.revision();
        if self.viewport.body.width > 0.0 {
            self.ensure_visual_map(&snapshot, self.viewport.body.width, measure);
        }
        self.ensure_review_layout(&snapshot);
        let targets = if let UiEvent::Key { key, shift, ctrl } = event {
            let mut targets = Vec::new();
            let mut primary_x = self.viewport.desired_x;
            for (index, selected) in snapshot.selections().enumerate() {
                if index > 0 {
                    self.viewport.desired_x = None;
                }
                if let Some(target) =
                    self.navigation_target(&snapshot, selected, *key, *shift, *ctrl)
                {
                    targets.push(Selection {
                        anchor: if *shift { selected.anchor } else { target },
                        head: target,
                    });
                }
                if index == 0 {
                    primary_x = self.viewport.desired_x;
                }
            }
            self.viewport.desired_x = primary_x;
            targets
        } else {
            Vec::new()
        };
        let mut copy_ranges: Vec<_> = snapshot
            .selections()
            .filter(|s| !s.is_empty())
            .map(Selection::range)
            .collect();
        copy_ranges.sort_by_key(|range| range.start);
        let copy_chars = copy_ranges.iter().map(|range| range.len()).sum::<usize>();
        let copy_text = if matches!(
            event,
            UiEvent::Key {
                key: Key::C | Key::X,
                ctrl: true,
                ..
            }
        ) && !copy_ranges.is_empty()
            && copy_chars <= CLIPBOARD_CHAR_LIMIT
        {
            Some(
                copy_ranges
                    .iter()
                    .map(|range| snapshot.text_range(range.clone()))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|parts| parts.join("\n")),
            )
        } else {
            None
        };
        let Ok(mut buffer) = controller.0.lock() else {
            return vec![self.fail("Editor buffer lock is unavailable")];
        };
        if buffer.document_id() != snapshot.document_id()
            || buffer.revision() != before
            || buffer.selection() != selection
        {
            return vec![self.fail("Editor changed while processing the input")];
        }
        let timestamp = now
            .saturating_duration_since(self.started)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let mut actions = Vec::new();
        let mut result: Result<bool, String> = Ok(false);
        let edit = !self.editor.read_only;
        match event {
            UiEvent::Key {
                key: Key::D,
                ctrl: true,
                shift,
            } => {
                result = buffer
                    .execute_command(
                        EditorCommand::SelectNextOccurrence { all: *shift },
                        timestamp,
                    )
                    .map_err(|e| e.to_string());
            }
            UiEvent::Key {
                key: Key::Escape, ..
            } => {
                buffer.cancel_snippet();
                result = buffer
                    .set_selection(selection)
                    .map(|_| false)
                    .map_err(|e| e.to_string());
            }
            UiEvent::Text(text) if edit => {
                result = buffer
                    .type_text(text, self.editor.auto_pairs, timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Paste(text) if edit => {
                result = buffer
                    .insert(text, EditKind::Paste, timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::S,
                ctrl: true,
                ..
            } => {
                actions.push(UiAction::EditorSaveRequested {
                    id: self.editor.id.clone(),
                    document_id: buffer.document_id(),
                    revision: before,
                });
            }
            UiEvent::Key {
                key: Key::C | Key::X,
                ctrl: true,
                ..
            } => {
                if copy_chars > CLIPBOARD_CHAR_LIMIT {
                    actions.push(self.fail("Selected text exceeds the clipboard limit"));
                } else if !copy_ranges.is_empty() {
                    match copy_text {
                        Some(Ok(text)) => {
                            actions.push(UiAction::CopyRequested { text });
                            if edit && matches!(event, UiEvent::Key { key: Key::X, .. }) {
                                result = buffer
                                    .insert("", EditKind::Replace, timestamp)
                                    .map_err(|error| error.to_string());
                            }
                        }
                        Some(Err(error)) => actions.push(self.fail(error)),
                        None => actions.push(self.fail("Selected text is unavailable")),
                    }
                }
            }
            UiEvent::Key {
                key: Key::Z,
                ctrl: true,
                shift,
                ..
            } if edit => {
                result = if *shift { buffer.redo() } else { buffer.undo() }
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Y,
                ctrl: true,
                ..
            } if edit => {
                result = buffer.redo().map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::A,
                ctrl: true,
                ..
            } => {
                result = buffer
                    .set_selection(Selection {
                        anchor: 0,
                        head: snapshot.char_count(),
                    })
                    .map(|_| false)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Backspace,
                ctrl: true,
                ..
            } if edit => {
                result = buffer
                    .delete_word_backward(timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Delete,
                ctrl: true,
                ..
            } if edit => {
                result = buffer
                    .delete_word_forward(timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Backspace,
                ..
            } if edit => {
                result = (if self.editor.auto_pairs {
                    buffer.delete_pair_backward(timestamp)
                } else {
                    buffer.delete_backward(timestamp)
                })
                .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Delete, ..
            } if edit => {
                result = buffer
                    .delete_forward(timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Enter, ..
            } if edit => {
                result = buffer
                    .insert_newline(timestamp)
                    .map_err(|error| error.to_string());
            }
            UiEvent::Key {
                key: Key::Tab,
                shift,
                ..
            } if edit => {
                result = (if buffer.has_snippet() {
                    buffer.next_snippet(*shift)
                } else {
                    buffer.indent_with_style(*shift, self.editor.indent_style, timestamp)
                })
                .map_err(|error| error.to_string());
            }
            UiEvent::Key { key, .. } if !targets.is_empty() => {
                result = buffer
                    .set_selections(targets)
                    .map(|_| false)
                    .map_err(|error| error.to_string());
                if !matches!(
                    key,
                    Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown
                ) {
                    self.viewport.desired_x = None;
                }
            }
            _ => {}
        }
        let history = matches!(
            event,
            UiEvent::Key {
                key: Key::Z | Key::Y,
                ctrl: true,
                ..
            }
        );
        self.finish_input(&buffer, &snapshot, result, actions, history)
    }

    fn finish_input(
        &mut self,
        buffer: &TextBuffer,
        snapshot: &BufferSnapshot,
        result: Result<bool, String>,
        mut actions: Vec<UiAction>,
        history: bool,
    ) -> Vec<UiAction> {
        match result {
            Err(error) => actions.push(self.fail(error)),
            Ok(_) if snapshot.revision() != buffer.revision() => {
                self.viewport.desired_x = None;
                let changed_row = if history {
                    0
                } else {
                    let selection = snapshot.selection();
                    let before_row = snapshot
                        .position_of(selection.anchor.min(selection.head))
                        .map_or(0, |position| position.row);
                    // Deleting a newline (including Ctrl+Backspace) changes
                    // the preceding row, before the old selection's row.
                    let after_row = buffer
                        .snapshot()
                        .position_of(buffer.selection().range().start)
                        .map_or(0, |position| position.row);
                    before_row.min(after_row)
                };
                self.syntax.invalidate_from(changed_row);
                actions.push(UiAction::EditorChanged {
                    id: self.editor.id.clone(),
                    document_id: buffer.document_id(),
                    revision: buffer.revision(),
                });
            }
            Ok(_) => {}
        }
        self.vim.observe_local_edit(buffer);
        self.viewport.reveal_caret = true;
        actions
    }
    /// Handle the pointer at `point`: place the caret, select by dragging or drive a scrollbar.
    /// `extend` keeps the anchor of the current selection; `dragging` is true while the button
    /// is held and false on release.
    pub fn point(
        &mut self,
        point: Point,
        extend: bool,
        dragging: bool,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<UiAction> {
        self.cancel_composition();
        if !extend {
            self.modified_drag = None;
            self.rectangle_anchor = None;
        }
        if self.editor.status_bar && point.y >= self.viewport.rect.y + self.viewport.rect.height {
            if !dragging {
                self.viewport.dragging = false;
            }
            return Vec::new();
        }
        if self.viewport.scrollbar_drag.is_some() {
            self.viewport.drag_scrollbar(point, dragging);
            return Vec::new();
        }
        if dragging && !extend && self.viewport.begin_scrollbar(point) {
            return Vec::new();
        }
        let Ok((snapshot, selection)) = self.editor.controller.state() else {
            return vec![self.fail("Editor buffer lock is unavailable")];
        };
        self.ensure_visual_map(&snapshot, self.viewport.body.width, measure);
        self.ensure_review_layout(&snapshot);
        if !extend && self.review_zone_at(point) {
            self.viewport.dragging = false;
            return Vec::new();
        }
        if self.rectangle_anchor.is_some() && extend {
            return self.pointer_modified(point, true, false, true, 1, dragging, measure);
        }
        let offset = self.offset_at_point(&snapshot, point, measure);
        let controller = self.editor.controller.clone();
        let Ok(mut buffer) = controller.0.lock() else {
            return vec![self.fail("Editor buffer lock is unavailable")];
        };
        if buffer.document_id() != snapshot.document_id()
            || buffer.revision() != snapshot.revision()
        {
            return Vec::new();
        }
        let next = Selection {
            anchor: if extend { selection.anchor } else { offset },
            head: offset,
        };
        let selected = if let Some(drag) = self.modified_drag.as_ref().filter(|_| extend) {
            if (point.x - drag.point.x).abs() < 2.0 && (point.y - drag.point.y).abs() < 2.0 {
                std::iter::once(drag.selection)
                    .chain(drag.secondary.iter().copied())
                    .collect()
            } else {
                let current = unit_selection(&snapshot, offset, drag.clicks);
                let selection = if offset < drag.selection.range().start {
                    Selection {
                        anchor: drag.selection.range().end,
                        head: current.range().start,
                    }
                } else {
                    Selection {
                        anchor: drag.selection.range().start,
                        head: current.range().end,
                    }
                };
                std::iter::once(selection)
                    .chain(drag.secondary.iter().copied())
                    .collect()
            }
        } else {
            vec![next]
        };
        if let Err(error) = buffer.set_selections(selected) {
            return vec![self.fail(error)];
        }
        if self.editor.vim_mode {
            self.vim.pointer_selection(&buffer.snapshot());
            self.vim.observe_local_edit(&buffer);
        }
        self.viewport.dragging = dragging;
        self.viewport.reveal_caret = true;
        self.viewport.desired_x = None;
        if !dragging {
            self.modified_drag = None;
        }
        Vec::new()
    }
    fn cancel_composition(&mut self) {
        if let Some(state) = self.composition.as_mut() {
            state.cancelled = true;
            state.text.clear();
        }
    }
    /// IME text remains provisional. Only Commit mutates the authoritative
    /// buffer, once, and only while its captured document/selection is current.
    pub(crate) fn composition(
        &mut self,
        preedit: Option<&str>,
        commit: Option<&str>,
        now: Instant,
    ) -> Vec<UiAction> {
        if self.editor.vim_mode && self.vim.mode != VimMode::Insert {
            self.cancel_composition();
            return Vec::new();
        }
        if let Some(text) = commit {
            let state = self.composition.take();
            let Ok(snapshot) = self.editor.controller.snapshot() else {
                return Vec::new();
            };
            if self.editor.read_only
                || state.as_ref().is_some_and(|state| {
                    state.cancelled
                        || state.document_id != snapshot.document_id()
                        || state.revision != snapshot.revision()
                        || state.selections != snapshot.selections().collect::<Vec<_>>()
                })
            {
                return Vec::new();
            }
            return self.input(&UiEvent::Paste(text.into()), now, &|text| {
                text.chars().count() as f32 * 8.0
            });
        }
        let Some(text) = preedit else {
            self.cancel_composition();
            return Vec::new();
        };
        if self.editor.read_only {
            return Vec::new();
        }
        if text.len() > 65_536 {
            return vec![self.fail("IME composition exceeds its preview limit")];
        }
        if text.is_empty() {
            if let Some(state) = self.composition.as_mut() {
                // Dismissal commonly precedes Commit. Keep its original scope,
                // including cancellation after focus/selection changed.
                state.text.clear();
                return Vec::new();
            }
        }
        let Ok(snapshot) = self.editor.controller.snapshot() else {
            return Vec::new();
        };
        self.composition = Some(CompositionState {
            text: text.into(),
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
            selections: snapshot.selections().collect(),
            cancelled: false,
        });
        self.viewport.reveal_caret = true;
        Vec::new()
    }
    /// The IME operates on its one primary cursor, not a multicursor batch.
    /// Counts are Unicode scalars; endpoints expand to whole graphemes so a
    /// platform request cannot leave half a combining sequence or CRLF.
    pub(crate) fn delete_surrounding(
        &mut self,
        before_chars: usize,
        after_chars: usize,
        now: Instant,
    ) -> Vec<UiAction> {
        if self.editor.read_only
            || (self.editor.vim_mode && self.vim.mode != VimMode::Insert)
            || (before_chars == 0 && after_chars == 0)
        {
            return Vec::new();
        }
        let controller = self.editor.controller.clone();
        let shared = controller.shared_buffer();
        let Ok(mut buffer) = shared.lock() else {
            return vec![self.fail("Editor buffer lock is unavailable")];
        };
        let snapshot = buffer.snapshot();
        let cursor = snapshot.selection().head;
        let Ok(start) = snapshot.snap_grapheme_backward(cursor.saturating_sub(before_chars)) else {
            return Vec::new();
        };
        let Ok(end) = snapshot.snap_grapheme_forward(
            cursor
                .saturating_add(after_chars)
                .min(snapshot.char_count()),
        ) else {
            return Vec::new();
        };
        let timestamp = now
            .saturating_duration_since(self.started)
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        match buffer.edit(
            start..end,
            "",
            Selection::caret(start),
            EditKind::Replace,
            timestamp,
        ) {
            Ok(true) => {
                self.vim.observe_local_edit(&buffer);
                self.syntax.invalidate_from(
                    snapshot
                        .position_of(start)
                        .map_or(0, |position| position.row),
                );
                self.viewport.reveal_caret = true;
                self.viewport.desired_x = None;
                if let Some(composition) =
                    self.composition.as_mut().filter(|state| !state.cancelled)
                {
                    composition.document_id = buffer.document_id();
                    composition.revision = buffer.revision();
                    composition.selections = buffer.snapshot().selections().collect();
                }
                vec![UiAction::EditorChanged {
                    id: self.editor.id.clone(),
                    document_id: buffer.document_id(),
                    revision: buffer.revision(),
                }]
            }
            Ok(false) => Vec::new(),
            Err(error) => vec![self.fail(error)],
        }
    }
    /// Last painted editable body and primary caret, in the same coordinates as
    /// DrawCommand. The host offsets them once when positioning the IME window.
    pub(crate) fn ime_rects(&self) -> Option<(Rect, Rect)> {
        self.ime_rects
    }
    fn ensure_visual_map(
        &mut self,
        snapshot: &BufferSnapshot,
        width: f32,
        measure: &dyn Fn(&str) -> f32,
    ) {
        if !self.editor.soft_wrap && self.editor.folds.is_empty() {
            if self.visual_map.is_some() {
                self.visual_map_generation = self.visual_map_generation.wrapping_add(1);
            }
            self.visual_map = None;
            self.visual_snapshot = None;
            return;
        }
        let columns = if self.editor.soft_wrap {
            ((width / measure(" ").max(1.0)).floor() as usize).clamp(1, 8192)
        } else {
            0
        };
        let rebuild = self
            .visual_map
            .as_ref()
            .is_none_or(|(revision, cols, tabs, folds, map)| {
                *revision != snapshot.revision()
                    || *cols != columns
                    || *tabs != self.editor.tab_width
                    || **folds != *self.editor.folds
                    || snapshot.selections().any(|s| {
                        snapshot.position_of(s.head).is_ok_and(|p| {
                            map.rows[map.row_for_offset(s.head)].physical_row != p.row
                        })
                    })
            });
        if rebuild {
            // Keep a caret visible: a fold containing it is temporarily expanded.
            let folds: Vec<_> = self
                .editor
                .folds
                .iter()
                .filter(|fold| {
                    !snapshot.selections().any(|s| {
                        snapshot
                            .position_of(s.head)
                            .is_ok_and(|p| p.row > fold.start && p.row < fold.end)
                    })
                })
                .cloned()
                .collect();
            let cached = self
                .visual_map
                .as_ref()
                .zip(self.visual_snapshot.as_ref())
                .filter(|((_, old_columns, tabs, _, _), _)| {
                    *old_columns == columns && *tabs == self.editor.tab_width
                });
            let next_map = if let Some(((_, _, _, old_folds, map), previous)) = cached {
                let old_folds: Vec<_> = old_folds
                    .iter()
                    .filter(|fold| {
                        !previous.selections().any(|s| {
                            previous
                                .position_of(s.head)
                                .is_ok_and(|p| p.row > fold.start && p.row < fold.end)
                        })
                    })
                    .cloned()
                    .collect();
                map.update(
                    previous,
                    snapshot,
                    (columns > 0).then_some(columns),
                    self.editor.tab_width,
                    &old_folds,
                    &folds,
                )
            } else {
                VisualMap::new(
                    snapshot,
                    (columns > 0).then_some(columns),
                    self.editor.tab_width,
                    &folds,
                )
            };
            if let Ok(map) = next_map {
                self.visual_map_generation = self.visual_map_generation.wrapping_add(1);
                self.visual_map = Some((
                    snapshot.revision(),
                    columns,
                    self.editor.tab_width,
                    self.editor.folds.clone(),
                    map,
                ));
                self.visual_snapshot = Some(snapshot.clone());
            }
        }
    }
    fn visual_line(
        &self,
        snapshot: &BufferSnapshot,
        index: usize,
        measure: &dyn Fn(&str) -> f32,
    ) -> (DisplayLine, std::ops::Range<usize>, bool, usize, bool, bool) {
        if let Some((_, _, _, _, map)) = &self.visual_map {
            let row = &map.rows[index.min(map.rows.len() - 1)];
            let origin = row.start_column as f32 * measure(" ").max(1.0);
            let (mut display, mut range, truncated) = visible_display_line(
                snapshot,
                row.physical_row,
                self.viewport.scroll_x + origin,
                self.viewport.body.width,
                self.editor.tab_width,
                measure,
            );
            if self.editor.soft_wrap {
                let end = row.range.end.saturating_sub(range.start);
                display.clusters.retain(|cluster| cluster.start < end);
                for cluster in &mut display.clusters {
                    cluster.x -= origin;
                }
                display.width = display.clusters.last().map_or(0.0, |c| c.x + c.width);
                range.end = range.end.min(row.range.end);
            }
            return (
                display,
                range,
                truncated && !self.editor.soft_wrap,
                row.physical_row,
                row.continuation,
                row.folded,
            );
        }
        let (display, range, truncated) = visible_display_line(
            snapshot,
            index,
            self.viewport.scroll_x,
            self.viewport.body.width,
            self.editor.tab_width,
            measure,
        );
        (display, range, truncated, index, false, false)
    }
    fn offset_at_point(
        &self,
        snapshot: &BufferSnapshot,
        point: Point,
        measure: &dyn Fn(&str) -> f32,
    ) -> usize {
        let count = self
            .visual_map
            .as_ref()
            .map_or(snapshot.line_count(), |(_, _, _, _, map)| map.rows.len());
        let display_row = ((point.y - self.viewport.body.y + self.viewport.scroll_y).max(0.0)
            / self.editor.line_height())
            .floor() as usize;
        let visual = self.review_layout.visual_row(display_row);
        let index = visual.min(count.saturating_sub(1));
        if self.review_layout.zone_row(display_row).is_some() {
            // Dragging through an old line lands at a real boundary, never a
            // column fabricated from removed text. A click is handled above.
            return if visual == count {
                snapshot.char_count()
            } else {
                self.visual_line(snapshot, index, measure).1.start
            };
        }
        let (display, range, _, _, _, _) = self.visual_line(snapshot, index, measure);
        (range.start + display.offset_at(point.x - self.viewport.body.x + self.viewport.scroll_x))
            .min(range.end)
    }
    /// Reposition a context-menu target only when outside every current selection.
    pub(crate) fn context_point(
        &mut self,
        point: Point,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<UiAction> {
        if self
            .viewport
            .minimap
            .is_some_and(|layout| layout.rect.contains(point))
        {
            return Vec::new();
        }
        let Ok(snapshot) = self.editor.controller.snapshot() else {
            return Vec::new();
        };
        self.ensure_visual_map(&snapshot, self.viewport.body.width, measure);
        self.ensure_review_layout(&snapshot);
        if self.review_zone_at(point) {
            return Vec::new();
        }
        let offset = self.offset_at_point(&snapshot, point, measure);
        if snapshot
            .selections()
            .any(|s| !s.is_empty() && s.range().contains(&offset))
        {
            return Vec::new();
        }
        self.point(point, false, false, measure)
    }
    // Native pointer boundary keeps independent modifiers and gesture values
    // explicit, alongside the renderer's matching text measurement callback.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pointer_modified(
        &mut self,
        point: Point,
        extend: bool,
        add: bool,
        rectangle: bool,
        click_count: u8,
        dragging: bool,
        measure: &dyn Fn(&str) -> f32,
    ) -> Vec<UiAction> {
        self.cancel_composition();
        if dragging
            && !self.viewport.dragging
            && self
                .viewport
                .minimap
                .is_some_and(|layout| layout.rect.contains(point))
        {
            self.viewport.begin_scrollbar(point);
            return Vec::new();
        }
        if !add && !rectangle && click_count < 2 {
            return self.point(point, extend, dragging, measure);
        }
        let Ok(snapshot) = self.editor.controller.snapshot() else {
            return Vec::new();
        };
        self.ensure_visual_map(&snapshot, self.viewport.body.width, measure);
        self.ensure_review_layout(&snapshot);
        if !extend && self.review_zone_at(point) {
            self.viewport.dragging = false;
            self.modified_drag = None;
            return Vec::new();
        }
        let offset = self.offset_at_point(&snapshot, point, measure);
        let Ok(mut buffer) = self.editor.controller.0.lock() else {
            return Vec::new();
        };
        if rectangle {
            let position = snapshot.position_of(offset).unwrap_or_default();
            let column = snapshot
                .display_column(offset, self.editor.tab_width)
                .unwrap_or(0);
            let head = Position {
                row: position.row,
                column,
            };
            let anchor = *self.rectangle_anchor.get_or_insert_with(|| {
                let p = snapshot
                    .position_of(snapshot.selection().anchor)
                    .unwrap_or_default();
                Position {
                    row: p.row,
                    column: snapshot
                        .display_column(snapshot.selection().anchor, self.editor.tab_width)
                        .unwrap_or(0),
                }
            });
            if let Err(error) = buffer.select_rectangle(anchor, head, self.editor.tab_width) {
                return vec![self.fail(error)];
            }
        } else {
            self.rectangle_anchor = None;
            let selected = if click_count >= 2 {
                unit_selection(&snapshot, offset, click_count)
            } else {
                Selection {
                    anchor: if extend {
                        snapshot.selection().anchor
                    } else {
                        offset
                    },
                    head: offset,
                }
            };
            let selections = if add {
                std::iter::once(selected)
                    .chain(snapshot.selections())
                    .collect()
            } else {
                vec![selected]
            };
            self.modified_drag = if dragging && (add || click_count >= 2) {
                Some(ModifiedDrag {
                    point,
                    selection: selected,
                    secondary: if add {
                        snapshot.selections().collect()
                    } else {
                        Vec::new()
                    },
                    clicks: click_count,
                })
            } else {
                None
            };
            if let Err(error) = buffer.set_selections(selections) {
                return vec![self.fail(error)];
            }
        }
        if self.editor.vim_mode {
            self.vim.pointer_selection(&buffer.snapshot());
            self.vim.observe_local_edit(&buffer);
        }
        self.viewport.dragging = dragging;
        self.viewport.reveal_caret = true;
        self.viewport.desired_x = None;
        if !dragging {
            self.rectangle_anchor = None;
            self.modified_drag = None;
        }
        Vec::new()
    }
    /// Draw the editor in `rect`, clipped to `clip`, and bring the viewport's scroll limits up
    /// to date. `show_caret` is asked, with the document, the revision and the caret's offset,
    /// whether the blinking caret is visible now.
    pub fn paint(
        &mut self,
        rect: Rect,
        clip: Rect,
        focused: bool,
        show_caret: &mut dyn FnMut(u64, u64, usize) -> bool,
        measure: &dyn Fn(&str) -> f32,
        text_height: f32,
    ) -> Vec<DrawCommand> {
        self.ime_rects = None;
        self.diagnostic_tooltip = None;
        self.structure_controls.clear();
        self.content_visible = Rect::new(rect.x, rect.y, 0.0, 0.0);
        let Ok((snapshot, selection)) = self.editor.controller.state() else {
            return Vec::new();
        };
        self.structure_cache.update(&self.editor, &snapshot);
        let external_syntax = self.editor.highlight_document_id == Some(snapshot.document_id())
            && self.editor.highlight_revision == Some(snapshot.revision());
        if self.highlight_index.follow(&self.editor, &snapshot) {
            self.syntax.release();
        } else {
            self.syntax.update(&snapshot, self.editor.language);
        }
        if self.composition.as_ref().is_some_and(|state| {
            state.cancelled
                || state.document_id != snapshot.document_id()
                || state.revision != snapshot.revision()
                || state.selections != snapshot.selections().collect::<Vec<_>>()
        }) {
            self.cancel_composition();
        }
        let mut digits = snapshot.line_count().max(1).to_string().len();
        if self.editor.review_document_id == Some(snapshot.document_id())
            && self.editor.review_revision == Some(snapshot.revision())
        {
            for hunk in self
                .editor
                .review_hunks
                .iter()
                .take(4096)
                .filter(|hunk| !hunk.removed_lines.is_empty())
            {
                digits = digits.max(
                    hunk.old_start_row
                        .saturating_add(hunk.removed_lines.len())
                        .to_string()
                        .len()
                        + 1,
                );
            }
        }
        let status_height = if self.editor.status_bar {
            (28.0 * self.editor.font_size / FONT_SIZE).min(rect.height)
        } else {
            0.0
        };
        let code_rect = Rect::new(
            rect.x,
            rect.y,
            rect.width,
            (rect.height - status_height).max(0.0),
        );
        let fold_lane = if self.editor.structure_document_id.is_some() {
            structure::GLYPH_LANE
        } else {
            0.0
        };
        let breakpoint_lane = if self.editor.breakpoint_action_prefix.is_some() {
            structure::BREAKPOINT_LANE
        } else {
            0.0
        };
        let gutter = fold_lane
            + breakpoint_lane
            + if self.editor.line_numbers {
                measure(&"9".repeat(digits)) + 12.0 + structure::NUMBER_TRAILING_PADDING
            } else {
                8.0
            };
        let minimap_width = minimap::reserved_width(self.editor.minimap, rect.width - gutter);
        let body = Rect::new(
            rect.x + gutter,
            rect.y + 4.0,
            (rect.width - gutter - 8.0 - minimap_width).max(0.0),
            (code_rect.height - 12.0).max(0.0),
        );
        self.viewport.rect = code_rect;
        self.viewport.body = body;
        self.ensure_visual_map(&snapshot, body.width, measure);
        self.ensure_review_layout(&snapshot);
        if self.editor.soft_wrap {
            self.viewport.scroll_x = 0.0;
        }
        let visual_count = self
            .visual_map
            .as_ref()
            .map_or(snapshot.line_count(), |(_, _, _, _, map)| map.rows.len());
        self.viewport.minimap = (minimap_width > 0.0).then(|| {
            minimap::Layout::new(
                Rect::new(body.x + body.width, body.y, minimap_width, body.height),
                self.review_layout.rows,
            )
        });
        if self.viewport.minimap.is_none() && self.viewport.minimap_drag {
            self.viewport.minimap_drag = false;
            self.viewport.scrollbar_drag = None;
        }
        let extent_key = (
            snapshot.document_id(),
            snapshot.revision(),
            self.editor.tab_width,
            measure(" ").to_bits(),
            body.width.to_bits(),
            self.review_layout.generation,
        );
        if self.horizontal_extent_key != Some(extent_key) {
            // Extents are discovered from visible lines. They may grow while
            // scrolling one revision, but must shrink after deletion or resize.
            self.horizontal_extent_key = Some(extent_key);
            self.viewport.max_x = 0.0;
        }
        self.viewport.max_y = (self.review_layout.rows as f32 * self.editor.line_height() - body.height).max(0.0);
        let reveal_top_inset = self
            .editor
            .reveal_top_inset
            .max(0.0)
            .min((body.height - self.editor.line_height()).max(0.0));
        self.viewport.min_y = -reveal_top_inset;
        self.viewport.scroll_y = self
            .viewport
            .scroll_y
            .clamp(self.viewport.min_y, self.viewport.max_y);
        let cursor_head = if self.editor.vim_mode {
            self.vim.cursor.unwrap_or(selection.head)
        } else {
            selection.head
        };
        let cursor_position = snapshot
            .position_of(cursor_head)
            .unwrap_or(Position { row: 0, column: 0 });
        let cursor_visual = self
            .visual_map
            .as_ref()
            .map_or(cursor_position.row, |(_, _, _, _, map)| {
                map.row_for_offset(cursor_head)
            });
        let mut review_top = None;
        if let Some((document, revision, hunk)) = self.review_reveal.take() {
            if document == snapshot.document_id() && revision == snapshot.revision() {
                if let Some(row) = self.review_layout.hunk_row(
                    hunk,
                    &self.editor,
                    self.visual_map.as_ref().map(|(_, _, _, _, map)| map),
                ) {
                    self.viewport.scroll_y = (row as f32 * self.editor.line_height() - reveal_top_inset)
                        .clamp(self.viewport.min_y, self.viewport.max_y);
                    self.viewport.reveal_caret = false;
                    review_top = Some(row as f32 * self.editor.line_height());
                }
            }
        }
        let mut sticky_rows = self.sticky_rows(&snapshot, body, reveal_top_inset);
        if let Some(top) = review_top {
            // Deleted zones need the same fixed-header clearance as a caret.
            // Re-evaluate ancestors after each upward adjustment; never let a
            // newly visible sticky row obscure the beginning of the hunk.
            for _ in 0..4 {
                let occupied = reveal_top_inset + sticky_rows.len() as f32 * self.editor.line_height();
                if top >= self.viewport.scroll_y + occupied {
                    break;
                }
                self.viewport.scroll_y = (top - occupied).max(self.viewport.min_y);
                sticky_rows = self.sticky_rows(&snapshot, body, reveal_top_inset);
            }
        }
        if self.viewport.reveal_caret {
            let top = self.review_layout.display_row(cursor_visual) as f32 * self.editor.line_height();
            // Re-evaluate the fixed headers after revealing: changing the top
            // row may reveal a new ancestor, which must not cover the caret.
            for _ in 0..4 {
                let occupied = reveal_top_inset + sticky_rows.len() as f32 * self.editor.line_height();
                let previous = self.viewport.scroll_y;
                if top < self.viewport.scroll_y + occupied {
                    self.viewport.scroll_y = (top - occupied).max(self.viewport.min_y);
                } else if top + self.editor.line_height() > self.viewport.scroll_y + body.height {
                    self.viewport.scroll_y = (top + self.editor.line_height() - body.height).max(0.0);
                }
                sticky_rows = self.sticky_rows(&snapshot, body, reveal_top_inset);
                if self.viewport.scroll_y == previous {
                    break;
                }
            }
            let x = snapshot
                .display_column(selection.head, self.editor.tab_width)
                .unwrap_or(0) as f32
                * measure(" ").max(1.0);
            if self.editor.soft_wrap {
                self.viewport.scroll_x = 0.0;
            } else if x < self.viewport.scroll_x {
                self.viewport.scroll_x = x;
            } else if x + 2.0 > self.viewport.scroll_x + body.width {
                self.viewport.scroll_x = (x + 2.0 - body.width).max(0.0);
            }
            self.viewport.reveal_caret = false;
        }
        let occupied = reveal_top_inset + sticky_rows.len() as f32 * self.editor.line_height();
        let visible_body = if sticky_rows.is_empty() {
            body
        } else {
            Rect::new(
                body.x,
                body.y + occupied,
                body.width,
                (body.height - occupied).max(0.0),
            )
        };
        self.content_visible = visible_body;
        self.update_review_active(&snapshot);
        // The find inset is a reveal constraint, not a permanent full-width
        // obstruction. Its floating surface owns its own clip/hit area. Paint
        // source above and below the fixed headers without moving the document.
        let first_display = (self.viewport.scroll_y.max(0.0) / self.editor.line_height()).floor() as usize;
        let end_display = ((self.viewport.scroll_y + body.height) / self.editor.line_height()).ceil() as usize;
        let first = self
            .review_layout
            .visual_row(first_display)
            .min(visual_count);
        let end = self.review_layout.visual_row(end_display).min(visual_count);
        let mut commands = vec![DrawCommand::PushClip(clip)];
        let mut backgrounds = Vec::new();
        let selections: Vec<_> = snapshot
            .selections()
            .filter(|selection| !selection.is_empty())
            .collect();
        let mut selection_rows: Vec<Vec<Rect>> = vec![Vec::new(); selections.len()];
        let has_selection = !selections.is_empty();
        // Include one neighboring visual row on each viewport edge, so clipped
        // continuations do not acquire artificial rounded caps while scrolling.
        // Shape each line once and reuse its exact tab/Unicode glyph geometry.
        let visible_lines: Vec<_> = (first.saturating_sub(usize::from(has_selection))
            ..end
                .saturating_add(usize::from(has_selection))
                .min(visual_count))
            .map(|visual| (visual, self.visual_line(&snapshot, visual, measure)))
            .collect();
        for (visual, (display, range, _, _, _, _)) in &visible_lines {
            for (selected, rows) in selections.iter().zip(&mut selection_rows) {
                let selected = selected.range();
                let left = selected.start.max(range.start).min(range.end) - range.start;
                let right = selected.end.min(range.end).max(range.start) - range.start;
                if right > left || (selected.start <= range.end && selected.end > range.end) {
                    let x1 = display.x_at(left);
                    let x2 = display.x_at(right).max(
                        x1 + if selected.end > range.end {
                            measure(" ")
                        } else {
                            0.0
                        },
                    );
                    rows.push(Rect::new(
                        body.x + x1 - self.viewport.scroll_x,
                        body.y + self.review_layout.display_row(*visual) as f32 * self.editor.line_height()
                            - self.viewport.scroll_y,
                        (x2 - x1).max(1.5),
                        self.editor.line_height(),
                    ));
                }
            }
        }
        let selections: Vec<_> = snapshot.selections().collect();
        let mut blink = None;
        let mut longest = self.paint_removed_review_rows(
            rect,
            visible_body,
            clip,
            first_display,
            end_display,
            fold_lane,
            measure,
            text_height,
            &mut commands,
        );
        let ime_offset = if self
            .composition
            .as_ref()
            .is_some_and(|state| !state.cancelled && !state.text.is_empty())
        {
            selection.range().start
        } else {
            selection.head
        };
        let ime_visual = self.visual_map.as_ref().map_or_else(
            || {
                snapshot
                    .position_of(ime_offset)
                    .map_or(0, |position| position.row)
            },
            |(_, _, _, _, map)| map.row_for_offset(ime_offset),
        );
        let diagnostics = if self.editor.diagnostic_document_id == Some(snapshot.document_id())
            && self.editor.diagnostic_revision == Some(snapshot.revision())
        {
            self.editor.diagnostics.as_ref()
        } else {
            &[]
        };
        let pointer = self.review_interaction.pointer();
        let mut hovered_diagnostic = None;
        if let Some(ruler_clip) = visible_body.intersection(clip).filter(|_| !self.editor.rulers.is_empty()) {
            let cell = measure(" ").max(1.0);
            commands.push(DrawCommand::PushClip(ruler_clip));
            for column in self.editor.rulers.iter() {
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(
                        (body.x + *column as f32 * cell - self.viewport.scroll_x).round(),
                        visible_body.y,
                        1.0,
                        visible_body.height,
                    ),
                    color: Color::rgba(
                        self.editor.palette.muted.r,
                        self.editor.palette.muted.g,
                        self.editor.palette.muted.b,
                        48,
                    ),
                    radius: 0.0,
                });
            }
            commands.push(DrawCommand::PopClip);
        }
        for (visual, (display, range, truncated, row, continuation, folded)) in visible_lines {
            if visual < first || visual >= end {
                continue;
            }
            let background_start = commands.len();
            longest = longest.max(display.width + if truncated { body.width } else { 0.0 });
            let y = body.y + self.review_layout.display_row(visual) as f32 * self.editor.line_height()
                - self.viewport.scroll_y;
            let text_y = centered_text_y(y, self.editor.line_height(), text_height);
            let visible_body = if !sticky_rows.is_empty() && y < body.y + reveal_top_inset {
                Rect::new(body.x, body.y, body.width, reveal_top_inset)
            } else {
                visible_body
            };
            let visible_rows = Rect::new(rect.x, visible_body.y, rect.width, visible_body.height);
            let Some(row_clip) = visible_rows.intersection(clip) else {
                continue;
            };
            if Rect::new(rect.x, y, rect.width, self.editor.line_height())
                .intersection(row_clip)
                .is_none()
            {
                continue;
            }
            commands.push(DrawCommand::PushClip(row_clip));
            if self.review_layout.added(row) {
                if self.editor.diff_backgrounds {
                    commands.push(DrawCommand::FillRect {
                        rect: Rect::new(rect.x, y, rect.width, self.editor.line_height()),
                        color: review::ADDED_FILL,
                        radius: 0.0,
                    });
                }
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, 2.0, self.editor.line_height()),
                    color: review::ADDED_MARKER,
                    radius: 0.0,
                });
            }
            if !continuation {
                if let Some(candidate) = self.structure_cache.fold_at(row) {
                    self.structure_controls.push(structure::fold_control(
                        candidate,
                        folded,
                        Rect::new(body.x - fold_lane - 4.0, y, fold_lane, self.editor.line_height()),
                        row_clip,
                        self.editor.palette,
                    ));
                }
                if self.editor.breakpoint_document_id == Some(snapshot.document_id())
                    && self.editor.breakpoint_revision == Some(snapshot.revision())
                {
                    if let Some(prefix) = self
                        .editor
                        .breakpoint_action_prefix
                        .as_deref()
                        .filter(|prefix| !prefix.is_empty() && prefix.len() <= 512)
                    {
                        let marker_rect = Rect::new(rect.x, y, breakpoint_lane, self.editor.line_height());
                        let hovered = self.breakpoint_pointer.is_some_and(|point| {
                            marker_rect.contains(point) && row_clip.contains(point)
                        });
                        self.structure_controls.push(structure::breakpoint_control(
                            prefix,
                            row,
                            self.structure_cache.has_breakpoint(row),
                            self.structure_cache.debugger_marker(row),
                            hovered,
                            marker_rect,
                            row_clip,
                            self.editor.palette,
                        ));
                    }
                }
            }
            if focused
                && !self.editor.read_only
                && visual == ime_visual
                && (!self.editor.vim_mode || self.vim.mode == VimMode::Insert)
            {
                let composing = self
                    .composition
                    .as_ref()
                    .filter(|state| !state.cancelled && !state.text.is_empty());
                let x = body.x + display.x_at(ime_offset.saturating_sub(range.start))
                    - self.viewport.scroll_x
                    + composing.map_or(0.0, |state| measure(&state.text));
                let caret = Rect::new(x, y, 1.5, self.editor.line_height());
                if let Some(visible_body) = visible_body.intersection(clip) {
                    if let Some(visible_caret) = caret.intersection(visible_body) {
                        self.ime_rects = Some((visible_body, visible_caret));
                    }
                }
            }
            let execution = if self.editor.debugger_document_id == Some(snapshot.document_id())
                && self.editor.debugger_revision == Some(snapshot.revision())
            {
                self.editor
                    .execution_line
                    .filter(|execution| execution.row == row)
            } else {
                None
            };
            if let Some(execution) = execution {
                let color = Color::rgba(232, 183, 67, if execution.primary { 38 } else { 20 });
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, rect.width, self.editor.line_height()),
                    color,
                    radius: 0.0,
                });
                if !continuation {
                    commands.push(DrawCommand::FillTriangle {
                        points: [
                            Point::new(rect.x + breakpoint_lane + 1.0, y + 4.0),
                            Point::new(rect.x + breakpoint_lane + 6.0, y + self.editor.line_height() * 0.5),
                            Point::new(rect.x + breakpoint_lane + 1.0, y + self.editor.line_height() - 4.0),
                        ],
                        color: Color::rgb(232, 183, 67),
                    });
                }
            }
            if self.editor.highlight_current_line && focused && row == cursor_position.row {
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, rect.width, self.editor.line_height()),
                    color: Color::rgba(
                        self.editor.palette.text.r,
                        self.editor.palette.text.g,
                        self.editor.palette.text.b,
                        8,
                    ),
                    radius: 0.0,
                });
            }
            if self.editor.line_numbers && !continuation {
                let number = (row + 1).to_string();
                commands.push(text_command(
                    self.editor.font_size,
                    Point::new(
                        body.x - fold_lane - structure::NUMBER_TRAILING_PADDING - measure(&number),
                        text_y,
                    ),
                    number,
                    if focused && row == cursor_position.row {
                        self.editor.palette.text
                    } else {
                        self.editor.palette.muted
                    },
                ));
            }
            let row_diagnostics = diagnostics.iter().take(4096).filter(|diagnostic| {
                diagnostic.range.start <= diagnostic.range.end
                    && diagnostic.range.end <= snapshot.char_count()
                    && (diagnostic.range.start < range.end && diagnostic.range.end > range.start
                        || diagnostic.range.is_empty()
                            && diagnostic.range.start >= range.start
                            && diagnostic.range.start <= range.end)
            });
            if self.editor.line_numbers && execution.is_none() {
                if let Some(diagnostic) = row_diagnostics.clone().next() {
                    commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            rect.x + breakpoint_lane + 3.0,
                            y + (self.editor.line_height() - 5.0) * 0.5,
                            5.0,
                            5.0,
                        ),
                        color: diagnostic.color,
                        radius: 2.5,
                    });
                    let target = Rect::new(rect.x + breakpoint_lane, y, 11.0, self.editor.line_height());
                    if hovered_diagnostic.is_none()
                        && !diagnostic.message.is_empty()
                        && pointer.is_some_and(|(point, _)| target.contains(point))
                    {
                        hovered_diagnostic =
                            Some((diagnostic.range.clone(), target, &diagnostic.message));
                    }
                }
            }
            commands.push(DrawCommand::PushClip(
                visible_body.intersection(clip).unwrap_or(Rect::new(
                    body.x,
                    visible_body.y,
                    0.0,
                    0.0,
                )),
            ));
            if self.editor.indent_guides {
                self.guide_cache.paint_row(
                    &snapshot,
                    row,
                    continuation,
                    body,
                    y,
                    self.editor.line_height(),
                    self.viewport.scroll_x,
                    self.editor.tab_width,
                    measure(" ").max(1.0),
                    cursor_position.row,
                    focused,
                    self.editor.language,
                    self.editor.palette.muted,
                    &mut commands,
                );
            }
            if self.editor.search_revision == Some(snapshot.revision()) {
                for found in self
                    .editor
                    .search_matches
                    .iter()
                    .filter(|found| found.start < range.end && found.end > range.start)
                {
                    let left = found.start.max(range.start) - range.start;
                    let right = found.end.min(range.end) - range.start;
                    commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            body.x + display.x_at(left) - self.viewport.scroll_x,
                            y,
                            (display.x_at(right) - display.x_at(left)).max(2.0),
                            self.editor.line_height(),
                        ),
                        color: Color::rgba(218, 173, 54, 70),
                        radius: 2.0,
                    });
                }
            }
            // Backgrounds for every row precede the continuous selection.
            // Reopen the same clips for glyphs/carets; this avoids drawing a
            // following row's background over a rounded concave connection.
            if has_selection {
                backgrounds.extend(commands.drain(background_start..));
                backgrounds.push(DrawCommand::PopClip);
                backgrounds.push(DrawCommand::PopClip);
                commands.push(DrawCommand::PushClip(row_clip));
                commands.push(DrawCommand::PushClip(
                    visible_body.intersection(clip).unwrap_or(Rect::new(
                        body.x,
                        visible_body.y,
                        0.0,
                        0.0,
                    )),
                ));
            }
            let highlights = self.highlight_index.lookup(
                &self.editor,
                &self.syntax,
                &snapshot,
                row,
                range.clone(),
            );
            display.paint(self.editor.font_size,
                &mut commands,
                Point::new(body.x - self.viewport.scroll_x, text_y),
                (self.viewport.scroll_x, body.width),
                self.editor.palette.text,
                range.start,
                &highlights,
            );
            if self.editor.show_whitespace {
                let muted = self.editor.palette.muted;
                display.paint_whitespace(
                    self.editor.font_size,
                    &mut commands,
                    Point::new(body.x - self.viewport.scroll_x, text_y),
                    (self.viewport.scroll_x, body.width),
                    Color::rgba(muted.r, muted.g, muted.b, 110),
                    measure,
                );
            }
            for diagnostic in row_diagnostics {
                let left = diagnostic.range.start.max(range.start).min(range.end) - range.start;
                let right = diagnostic.range.end.min(range.end).max(range.start) - range.start;
                let x = body.x + display.x_at(left) - self.viewport.scroll_x;
                let width = (display.x_at(right) - display.x_at(left)).max(measure(" "));
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(x, y + self.editor.line_height() - 2.0, width, 1.5),
                    color: diagnostic.color,
                    radius: 0.0,
                });
                let target = Rect::new(x, y, width, self.editor.line_height());
                if hovered_diagnostic.is_none()
                    && !diagnostic.message.is_empty()
                    && pointer.is_some_and(|(point, _)| {
                        target.contains(point) && visible_body.contains(point)
                    })
                {
                    hovered_diagnostic = Some((diagnostic.range.clone(), target, &diagnostic.message));
                }
            }
            for selected in &selections {
                let vim_block = self.editor.vim_mode && self.vim.mode != VimMode::Insert;
                let style = if vim_block { CaretStyle::Block } else { self.editor.caret_style };
                let head = if vim_block {
                    self.vim.cursor.unwrap_or(selected.head)
                } else {
                    selected.head
                };
                let selected_visual = self.visual_map.as_ref().map_or_else(
                    || snapshot.position_of(head).map_or(0, |p| p.row),
                    |(_, _, _, _, map)| map.row_for_offset(head),
                );
                if focused
                    && self
                        .composition
                        .as_ref()
                        .is_none_or(|state| state.cancelled || state.text.is_empty())
                    && (selected.is_empty() || vim_block)
                    && visual == selected_visual
                    && head >= range.start
                    && head <= range.end
                {
                    let cell = || {
                        let next = snapshot.next_grapheme(head).unwrap_or(head).min(range.end);
                        (display.x_at(next.saturating_sub(range.start))
                            - display.x_at(head.saturating_sub(range.start)))
                        .max(measure(" "))
                    };
                    let (width, top, height) = match style {
                        CaretStyle::Line => (1.5, 0.0, self.editor.line_height()),
                        CaretStyle::Block => (cell(), 0.0, self.editor.line_height()),
                        CaretStyle::Underline => (cell(), self.editor.line_height() - 2.0, 2.0),
                    };
                    let caret_rect = Rect::new(
                        body.x + display.x_at(head.saturating_sub(range.start))
                            - self.viewport.scroll_x,
                        y + top,
                        width,
                        height,
                    );
                    if visible_body
                        .intersection(clip)
                        .and_then(|clip| caret_rect.intersection(clip))
                        .is_some()
                        && (!self.editor.caret_blink
                            || *blink.get_or_insert_with(|| {
                                show_caret(snapshot.document_id(), snapshot.revision(), selection.head)
                            }))
                    {
                        commands.push(DrawCommand::FillRect {
                            rect: caret_rect,
                            color: if style == CaretStyle::Block {
                                Color::rgba(
                                    self.editor.palette.text.r,
                                    self.editor.palette.text.g,
                                    self.editor.palette.text.b,
                                    96,
                                )
                            } else {
                                self.editor.palette.text
                            },
                            radius: 0.0,
                        });
                    }
                }
            }
            if folded {
                commands.push(text_command(
                    self.editor.font_size,
                    Point::new(
                        body.x + display.width - self.viewport.scroll_x + 8.0,
                        text_y,
                    ),
                    "…".into(),
                    self.editor.palette.muted,
                ));
            }
            if focused {
                if let Some(preedit) = &self.composition {
                    let start = selection.range().start;
                    let preedit_visual = self.visual_map.as_ref().map_or_else(
                        || snapshot.position_of(start).map_or(0, |p| p.row),
                        |(_, _, _, _, map)| map.row_for_offset(start),
                    );
                    if visual == preedit_visual && !preedit.cancelled && !preedit.text.is_empty() {
                        let x = body.x + display.x_at(start.saturating_sub(range.start))
                            - self.viewport.scroll_x;
                        let width = measure(&preedit.text);
                        commands.push(DrawCommand::FillRect {
                            rect: Rect::new(x, y, width, self.editor.line_height()),
                            color: self.editor.palette.surface,
                            radius: 0.0,
                        });
                        commands.push(text_command(
                    self.editor.font_size,
                            Point::new(x, text_y),
                            preedit.text.clone(),
                            self.editor.palette.text,
                        ));
                        commands.push(DrawCommand::FillRect {
                            rect: Rect::new(x, y + self.editor.line_height() - 2.0, width, 1.0),
                            color: self.editor.palette.text,
                            radius: 0.0,
                        });
                    }
                }
            }
            commands.push(DrawCommand::PopClip);
            commands.push(DrawCommand::PopClip);
        }
        let hovered_diagnostic = hovered_diagnostic
            .zip(pointer)
            .map(|((range, target, message), (_, now))| {
                let since = self
                    .diagnostic_hover
                    .as_ref()
                    .filter(|(hovered, _)| *hovered == range)
                    .map_or(now, |(_, since)| *since);
                let rested = now.saturating_duration_since(since) >= crate::tree::TOOLTIP_HOVER_DELAY;
                ((range, since), rested.then(|| (target, message.clone())))
            });
        let (hover, tooltip) = hovered_diagnostic.unzip();
        self.diagnostic_hover = hover;
        self.diagnostic_tooltip = tooltip.flatten();
        self.paint_sticky_rows(
            &sticky_rows,
            &snapshot,
            rect,
            body,
            clip,
            reveal_top_inset,
            fold_lane,
            measure,
            text_height,
            &mut commands,
        );
        self.build_review_controls(&snapshot, clip);
        self.viewport.max_x = if self.editor.soft_wrap {
            0.0
        } else {
            self.viewport.max_x.max((longest - body.width).max(0.0))
        };
        if self.viewport.scroll_x > self.viewport.max_x {
            self.viewport.scroll_x = self.viewport.max_x;
            return self.paint(rect, clip, focused, show_caret, measure, text_height);
        }
        self.paint_minimap(&snapshot, external_syntax, &mut commands);
        paint_scrollbars(&mut commands, &self.viewport, self.editor.palette.muted);
        if focused {
            self.paint_vim_mode(code_rect, &mut commands);
        }
        if self.editor.status_bar {
            paint_editor_status(
                &mut commands,
                self.editor.font_size,
                &snapshot,
                self.editor.language,
                self.editor.read_only,
                Rect::new(rect.x, rect.y + code_rect.height, rect.width, status_height),
                self.editor.palette,
                measure,
                text_height,
            );
        }
        commands.push(DrawCommand::PopClip);
        if !has_selection {
            return commands;
        }
        let mut painted =
            Vec::with_capacity(backgrounds.len() + commands.len() + selections.len() + 2);
        painted.push(DrawCommand::PushClip(clip));
        painted.extend(backgrounds);
        painted.push(DrawCommand::PushClip(
            body.intersection(clip)
                .unwrap_or(Rect::new(body.x, body.y, 0.0, 0.0)),
        ));
        for rows in selection_rows {
            for points in selection::contours(&rows) {
                painted.push(DrawCommand::SelectionContour {
                    points,
                    color: selection::COLOR,
                });
            }
        }
        painted.push(DrawCommand::PopClip);
        painted.extend(commands.into_iter().skip(1));
        painted
    }
}

fn unit_selection(snapshot: &BufferSnapshot, offset: usize, clicks: u8) -> Selection {
    if clicks >= 3 {
        let row = snapshot.position_of(offset).unwrap_or_default().row;
        let range = snapshot.line_range(row).unwrap_or(offset..offset);
        return Selection {
            anchor: range.start,
            head: if row + 1 < snapshot.line_count() {
                snapshot.line_range(row + 1).unwrap().start
            } else {
                range.end
            },
        };
    }
    if clicks == 2 {
        let start = if offset == 0 {
            0
        } else {
            snapshot
                .previous_word_start(snapshot.next_grapheme(offset).unwrap_or(offset))
                .unwrap_or(offset)
        };
        let end = snapshot.next_word_end(start).unwrap_or(offset);
        return Selection {
            anchor: start,
            head: end,
        };
    }
    Selection::caret(offset)
}

// Drawing boundary receives the same snapshot, geometry, palette and native
// font metrics used by the editor body, without hidden layout state.
#[allow(clippy::too_many_arguments)]
fn paint_editor_status(
    commands: &mut Vec<DrawCommand>,
    font_size: f32,
    snapshot: &BufferSnapshot,
    language: CodeLanguage,
    read_only: bool,
    rect: Rect,
    palette: ComponentPalette,
    measure: &dyn Fn(&str) -> f32,
    text_height: f32,
) {
    if rect.width <= 16.0 || rect.height < LINE_HEIGHT * font_size / FONT_SIZE {
        return;
    }
    let position = snapshot
        .position_of(snapshot.selection().head)
        .unwrap_or_default();
    let labels = crate::labels();
    let cursor = format!("{} {}, {} {}", labels.line_short, position.row + 1, labels.column_short, position.column + 1);
    let selected = snapshot.selection().range().len();
    let cursor_selection = if selected > 0 {
        format!("{cursor} · {selected} {}", labels.selected_short)
    } else {
        cursor.clone()
    };
    let language = match language {
        CodeLanguage::Plain => crate::labels().plain_text,
        CodeLanguage::Rust => "Rust",
        CodeLanguage::Json => "JSON",
        CodeLanguage::JavaScript => "JavaScript",
        CodeLanguage::TypeScript => "TypeScript",
        CodeLanguage::Tsx => "TSX",
        CodeLanguage::Python => "Python",
        CodeLanguage::Toml => "TOML",
        CodeLanguage::Css => "CSS",
        CodeLanguage::Html => "HTML",
        CodeLanguage::Markdown => "Markdown",
    };
    let ending = match snapshot.preferred_line_ending() {
        limn_editor::LineEnding::Lf => "LF",
        limn_editor::LineEnding::CrLf => "CRLF",
        limn_editor::LineEnding::Cr => "CR",
    };
    let labels = crate::labels();
    let state = if read_only {
        labels.read_only
    } else if snapshot.is_dirty() {
        labels.unsaved
    } else {
        labels.saved
    };
    let compact_state = if read_only {
        format!("{language} · {}", labels.read_only)
    } else {
        format!("{language}{}", if snapshot.is_dirty() { " •" } else { "" })
    };
    let candidates = [
        (
            state.to_owned(),
            format!("{cursor_selection} · {language} · UTF-8 · {ending}"),
        ),
        (compact_state, cursor_selection.clone()),
        (String::new(), cursor_selection),
        (String::new(), cursor),
        (
            String::new(),
            format!("{}:{}", position.row + 1, position.column + 1),
        ),
    ];
    let Some((left, right)) = candidates.into_iter().find(|(left, right)| {
        measure(left) + measure(right) + if left.is_empty() { 16.0 } else { 28.0 } <= rect.width
    }) else {
        return;
    };
    commands.push(DrawCommand::PushClip(rect));
    commands.push(DrawCommand::FillRect {
        rect,
        color: palette.surface,
        radius: 0.0,
    });
    commands.push(DrawCommand::FillRect {
        rect: Rect::new(rect.x, rect.y, rect.width, 1.0),
        color: palette.border,
        radius: 0.0,
    });
    let y = centered_text_y(rect.y, rect.height, text_height);
    if !left.is_empty() {
        commands.push(text_command(
                    font_size,
            Point::new(rect.x + 8.0, y),
            left,
            palette.muted,
        ));
    }
    let x = rect.x + rect.width - 8.0 - measure(&right);
    commands.push(text_command(font_size, Point::new(x, y), right, palette.muted));
    commands.push(DrawCommand::PopClip);
}

struct Cluster {
    start: usize,
    end: usize,
    x: f32,
    width: f32,
    text: String,
    tab: bool,
}
struct DisplayLine {
    clusters: Vec<Cluster>,
    width: f32,
    truncated: bool,
}
impl DisplayLine {
    fn window(
        text: &str,
        tabs: usize,
        first: f32,
        width: f32,
        measure: &dyn Fn(&str) -> f32,
    ) -> Self {
        Self::window_at(text, tabs, first, width, 0, measure)
    }
    fn window_at(
        text: &str,
        tabs: usize,
        first: f32,
        width: f32,
        first_column: usize,
        measure: &dyn Fn(&str) -> f32,
    ) -> Self {
        let cell = measure(" ").max(1.0);
        let mut clusters = Vec::new();
        let mut column = first_column;
        let mut offset = 0usize;
        let mut bytes = 0usize;
        let mut truncated = false;
        for grapheme in text.graphemes(true) {
            let count = grapheme.chars().count();
            let columns = if grapheme == "\t" {
                tabs - column % tabs
            } else {
                UnicodeWidthStr::width(grapheme)
            };
            let x = column as f32 * cell;
            let end = (column + columns) as f32 * cell;
            if x > first + width {
                truncated = true;
                break;
            }
            if end >= first {
                let shown = if grapheme == "\t" {
                    " ".repeat(columns)
                } else {
                    grapheme
                        .chars()
                        .take(65_536usize.saturating_sub(bytes) / 4)
                        .collect::<String>()
                };
                bytes = bytes.saturating_add(shown.len());
                let advance = if grapheme == "\t" {
                    columns as f32 * cell
                } else {
                    measure(&shown).max(0.0)
                };
                clusters.push(Cluster {
                    start: offset,
                    end: offset + count,
                    x,
                    width: advance,
                    text: shown,
                    tab: grapheme == "\t",
                });
                if bytes >= 65_532 || clusters.len() >= 8192 {
                    truncated = true;
                    break;
                }
            }
            column = column.saturating_add(columns);
            offset = offset.saturating_add(count);
        }
        Self {
            width: clusters
                .last()
                .map_or(column as f32 * cell, |cluster| cluster.x + cluster.width),
            clusters,
            truncated,
        }
    }
    fn new(text: &str, tabs: usize, measure: &dyn Fn(&str) -> f32) -> Self {
        Self::new_at(text, tabs, 0, measure)
    }
    fn new_at(text: &str, tabs: usize, first_column: usize, measure: &dyn Fn(&str) -> f32) -> Self {
        let cell = measure(" ").max(1.0);
        let mut clusters = Vec::new();
        let mut x = first_column as f32 * cell;
        let mut offset = 0;
        let mut column = first_column;
        for grapheme in text.graphemes(true) {
            let count = grapheme.chars().count();
            let (shown, width) = if grapheme == "\t" {
                let spaces = tabs - column % tabs;
                (" ".repeat(spaces), cell * spaces as f32)
            } else {
                (grapheme.to_owned(), measure(grapheme).max(0.0))
            };
            column += (width / cell).round().max(1.0) as usize;
            clusters.push(Cluster {
                start: offset,
                end: offset + count,
                x,
                width,
                tab: grapheme == "\t",
                text: shown,
            });
            offset += count;
            x += width;
        }
        Self {
            clusters,
            width: x,
            truncated: false,
        }
    }
    fn x_at(&self, offset: usize) -> f32 {
        self.clusters
            .iter()
            .find(|cluster| offset < cluster.end)
            .map_or(self.width, |cluster| cluster.x)
    }
    fn offset_at(&self, x: f32) -> usize {
        for cluster in &self.clusters {
            if x < cluster.x + cluster.width / 2.0 {
                return cluster.start;
            }
            if x < cluster.x + cluster.width {
                return cluster.end;
            }
        }
        self.clusters.last().map_or(0, |cluster| cluster.end)
    }
    /// `·` centred on each visible space and `→` at the start of each visible tab.
    fn paint_whitespace(
        &self,
        font_size: f32,
        commands: &mut Vec<DrawCommand>,
        origin: Point,
        window: (f32, f32),
        color: Color,
        measure: &dyn Fn(&str) -> f32,
    ) {
        let (offset, width) = window;
        let dot = measure("·");
        for cluster in self.clusters.iter().filter(|cluster| {
            (cluster.tab || cluster.text == " ")
                && cluster.x + cluster.width >= offset
                && cluster.x <= offset + width
        }) {
            let (mark, x) = if cluster.tab {
                ("→", cluster.x)
            } else {
                ("·", cluster.x + (cluster.width - dot) * 0.5)
            };
            commands.push(text_command(font_size, Point::new(origin.x + x, origin.y), mark.into(), color));
        }
    }
    fn paint(
        &self,
        font_size: f32,
        commands: &mut Vec<DrawCommand>,
        origin: Point,
        window: (f32, f32),
        color: Color,
        line_start: usize,
        highlights: &[CodeHighlight],
    ) {
        let mut run = String::new();
        let mut run_x = 0.0;
        let mut run_color = color;
        let (offset, width) = window;
        for cluster in self
            .clusters
            .iter()
            .filter(|cluster| cluster.x + cluster.width >= offset && cluster.x <= offset + width)
        {
            let tint = highlights
                .iter()
                .find(|span| {
                    span.range.start <= line_start + cluster.start
                        && span.range.end > line_start + cluster.start
                })
                .map_or(color, |span| span.color);
            if !run.is_empty() && tint != run_color {
                commands.push(text_command(
                    font_size,
                    Point::new(origin.x + run_x, origin.y),
                    std::mem::take(&mut run),
                    run_color,
                ));
            }
            if run.is_empty() {
                run_x = cluster.x;
                run_color = tint;
            }
            run.push_str(&cluster.text);
        }
        if !run.is_empty() {
            commands.push(text_command(
                    font_size,
                Point::new(origin.x + run_x, origin.y),
                run,
                run_color,
            ));
        }
    }
}

fn visible_display_line(
    snapshot: &BufferSnapshot,
    row: usize,
    scroll_x: f32,
    width: f32,
    tabs: usize,
    measure: &dyn Fn(&str) -> f32,
) -> (DisplayLine, std::ops::Range<usize>, bool) {
    let cell = measure(" ").max(1.0);
    let first = (scroll_x / cell).floor().max(0.0) as usize;
    let count = ((width / cell).ceil().max(1.0) as usize)
        .saturating_add(4)
        .min(8192);
    if let Ok(window) = snapshot.display_line_window(row, first, count, tabs) {
        let source = if window.char_range.len() > 65_536 {
            window.text.clone()
        } else {
            snapshot
                .text_range(window.char_range.clone())
                .unwrap_or_default()
        };
        (
            DisplayLine::new_at(&source, tabs, window.start_column, measure),
            window.char_range,
            window.truncated,
        )
    } else {
        (DisplayLine::new("", tabs, measure), 0..0, false)
    }
}
// DrawCommand::Text is top-aligned. Keep this inset separate from the row
// origin used by selection, caret, hit testing and scroll calculations.
fn centered_text_y(row_y: f32, row_height: f32, text_height: f32) -> f32 {
    row_y + (row_height - text_height).max(0.0) / 2.0
}

fn text_command(font_size: f32, position: Point, text: String, color: Color) -> DrawCommand {
    DrawCommand::Text {
        position,
        text,
        color,
        font_size,
        font_family: FontFamily::Monospace,
    }
}
fn paint_scrollbars(commands: &mut Vec<DrawCommand>, viewport: &CodeViewport, color: Color) {
    // Share visual tokens with ScrollArea while keeping the editor's existing
    // eight-pixel pointer rail and proportional scroll/drag calculations.
    let appearance = crate::ScrollbarAppearance::overlay().buttons(false);
    let thickness = appearance.thumb_width.min(8.0);
    let mut paint = |track: Rect, thumb: Rect, vertical: bool| {
        let hovered = viewport
            .scrollbar_pointer
            .is_some_and(|point| track.contains(point));
        let active = !viewport.minimap_drag
            && viewport
                .scrollbar_drag
                .is_some_and(|(axis, _)| axis == vertical);
        if hovered || active {
            commands.push(DrawCommand::FillRect {
                rect: track,
                color: appearance.track_color(color),
                radius: 4.0,
            });
        }
        commands.push(DrawCommand::FillRect {
            rect: thumb,
            color: appearance.thumb_color(color, hovered, active),
            radius: thickness / 2.0,
        });
    };
    let vertical_range = viewport.max_y - viewport.min_y;
    if vertical_range > 0.0 {
        let h = (viewport.body.height * viewport.body.height
            / (viewport.body.height + vertical_range))
            .max(16.0)
            .min(viewport.body.height);
        let y = viewport.body.y
            + (viewport.body.height - h) * (viewport.scroll_y - viewport.min_y) / vertical_range;
        let track = Rect::new(
            viewport.rect.x + viewport.rect.width - 8.0,
            viewport.body.y,
            8.0,
            viewport.body.height,
        );
        paint(
            track,
            Rect::new(track.x + (8.0 - thickness) / 2.0, y, thickness, h),
            true,
        );
    }
    if viewport.max_x > 0.0 {
        let w = (viewport.body.width * viewport.body.width
            / (viewport.body.width + viewport.max_x))
            .max(16.0)
            .min(viewport.body.width);
        let x = viewport.body.x + (viewport.body.width - w) * viewport.scroll_x / viewport.max_x;
        let track = Rect::new(
            viewport.body.x,
            viewport.rect.y + viewport.rect.height - 8.0,
            viewport.body.width,
            8.0,
        );
        paint(
            track,
            Rect::new(x, track.y + (8.0 - thickness) / 2.0, w, thickness),
            false,
        );
    }
}

pub(crate) struct DiffView {
    pub viewport: CodeViewport,
    syntax: crate::code_syntax::SyntaxCache,
    identity: Option<Arc<[crate::CodeDiffLine]>>,
    digits: usize,
    single_gutter: bool,
    /// The folded rows in view: their index, their rect and its clip, for the tree to lay
    /// their controls over.
    pub expand: Vec<(usize, Rect, Rect)>,
}
impl Default for DiffView {
    fn default() -> Self {
        Self {
            viewport: CodeViewport::default(),
            syntax: crate::code_syntax::SyntaxCache::new(),
            identity: None,
            digits: 1,
            single_gutter: false,
            expand: Vec::new(),
        }
    }
}
impl DiffView {
    /// True while syntax colours for the diff are still being computed.
    pub fn syntax_pending(&self) -> bool {
        self.syntax.pending()
    }
    /// True when `hit` names an expand control this diff, `id`, laid at `position`.
    pub(crate) fn owns_expand_control(&self, id: &str, hit: &str, position: Point) -> bool {
        self.expand.iter().any(|(row, rect, clip)| {
            rect.contains(position) && clip.contains(position) && hit == format!("{id}:expand:{row}")
        })
    }
}

pub(crate) fn paint_diff(
    diff: &CodeDiff,
    view: &mut DiffView,
    rect: Rect,
    clip: Rect,
    measure: &dyn Fn(&str) -> f32,
    text_height: f32,
) -> Vec<DrawCommand> {
    let line_height = LINE_HEIGHT * diff.font_size / FONT_SIZE;
    view.syntax.update_diff(diff.lines.clone(), diff.language);
    let viewport = &mut view.viewport;
    if view
        .identity
        .as_ref()
        .is_none_or(|previous| !Arc::ptr_eq(previous, &diff.lines))
        || view.single_gutter != diff.single_gutter
    {
        view.digits = diff
            .lines
            .iter()
            .filter_map(|line| {
                if diff.single_gutter {
                    line.new_line.or(line.old_line)
                } else {
                    line.old_line.max(line.new_line)
                }
            })
            .max()
            .unwrap_or(1)
            .to_string()
            .len();
        view.identity = Some(diff.lines.clone());
        view.single_gutter = diff.single_gutter;
    }
    let digits = view.digits;
    let number_width = measure(&"9".repeat(digits)) + 10.0;
    let gutter = if diff.single_gutter {
        number_width + 8.0
    } else {
        number_width * 2.0 + 24.0
    };
    let previous_width = viewport.body.width;
    viewport.rect = rect;
    viewport.body = Rect::new(
        rect.x + gutter,
        rect.y + 4.0,
        (rect.width - gutter - 8.0).max(0.0),
        (rect.height - 12.0).max(0.0),
    );
    if viewport.body.width != previous_width {
        // The previous extent can be a bounded estimate for a long line.
        // Re-measure the visible window when its gutter or width changes.
        viewport.max_x = 0.0;
    }
    let previous_max_y = viewport.max_y;
    let was_at_bottom = previous_max_y - viewport.scroll_y <= 2.0;
    viewport.max_y = (diff.lines.len() as f32 * line_height - viewport.body.height).max(0.0);
    viewport.scroll_y = if diff.follow_tail && was_at_bottom && viewport.max_y > previous_max_y {
        viewport.max_y
    } else {
        viewport.scroll_y.min(viewport.max_y)
    };
    let first = (viewport.scroll_y / line_height).floor() as usize;
    let end = ((viewport.scroll_y + viewport.body.height) / line_height).ceil() as usize;
    let visible_end = end.min(diff.lines.len());
    let mut displays = Vec::with_capacity(visible_end.saturating_sub(first));
    for row in first..visible_end {
        let line = &diff.lines[row];
        let text = line
            .text
            .strip_suffix("\r\n")
            .or_else(|| line.text.strip_suffix('\n'))
            .or_else(|| line.text.strip_suffix('\r'))
            .unwrap_or(&line.text);
        let display = DisplayLine::window(
            text,
            diff.tab_width,
            viewport.scroll_x,
            viewport.body.width,
            measure,
        );
        viewport.max_x = viewport.max_x.max(
            (display.width
                + if display.truncated {
                    viewport.body.width
                } else {
                    0.0
                }
                - viewport.body.width)
                .max(0.0),
        );
        displays.push((row, display));
    }
    let previous_scroll = viewport.scroll_x;
    viewport.scroll_x = viewport.scroll_x.min(viewport.max_x);
    if viewport.scroll_x != previous_scroll {
        for (row, display) in &mut displays {
            let line = &diff.lines[*row];
            let text = line
                .text
                .strip_suffix("\r\n")
                .or_else(|| line.text.strip_suffix('\n'))
                .or_else(|| line.text.strip_suffix('\r'))
                .unwrap_or(&line.text);
            *display = DisplayLine::window(
                text,
                diff.tab_width,
                viewport.scroll_x,
                viewport.body.width,
                measure,
            );
        }
    }
    let mut commands = vec![DrawCommand::PushClip(clip)];
    view.expand.clear();
    let colors = diff.colors;
    for (row, display) in displays {
        let line = &diff.lines[row];
        let y = viewport.body.y + row as f32 * line_height - viewport.scroll_y;
        let text_y = centered_text_y(y, line_height, text_height);
        // Gutter density changes the line numbers, not the meaning of a row.
        // A created file still contains additions with a single number column.
        let (marker, marker_color, fill, word) = match line.kind {
            CodeDiffLineKind::Context => (" ", diff.palette.text, Color::TRANSPARENT, Color::TRANSPARENT),
            CodeDiffLineKind::Added => ("+", colors.add_bar, colors.add_row, colors.add_word),
            CodeDiffLineKind::Removed => ("−", colors.del_bar, colors.del_row, colors.del_word),
            CodeDiffLineKind::Folded => {
                // The band only; the tree lays the row's label and control over it.
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, rect.width, line_height),
                    color: diff.palette.hover,
                    radius: 0.0,
                });
                let rows = Rect::new(rect.x, viewport.body.y, rect.width, viewport.body.height);
                if let Some(row_clip) = rows.intersection(clip) {
                    // Short of the scrollbar, which keeps its own presses.
                    let right = viewport.body.x + viewport.body.width;
                    view.expand.push((row, Rect::new(rect.x, y, right - rect.x, line_height), row_clip));
                }
                continue;
            }
        };
        let changed = line.kind != CodeDiffLineKind::Context;
        if changed {
            if diff.diff_backgrounds {
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, rect.width, line_height),
                    color: fill,
                    radius: 0.0,
                });
            }
            if diff.margin_bar {
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(rect.x, y, 2.0, line_height),
                    color: marker_color,
                    radius: 0.0,
                });
            }
        }
        let numbers = if diff.single_gutter {
            [(rect.x + 8.0, line.new_line.or(line.old_line)), (0.0, None)]
        } else {
            [
                (rect.x + 8.0, line.old_line),
                (rect.x + 8.0 + number_width, line.new_line),
            ]
        };
        for (x, value) in numbers {
            if let Some(number) = value {
                commands.push(text_command(
                    diff.font_size,
                    Point::new(x, text_y),
                    number.to_string(),
                    diff.palette.muted,
                ));
            }
        }
        if !diff.single_gutter {
            commands.push(text_command(
                    diff.font_size,
                Point::new(rect.x + gutter - 16.0, text_y),
                marker.to_owned(),
                marker_color,
            ));
        }
        commands.push(DrawCommand::PushClip(
            viewport.body.intersection(clip).unwrap_or(Rect::new(
                viewport.body.x,
                viewport.body.y,
                0.0,
                0.0,
            )),
        ));
        for range in diff.word_ranges.get(row).filter(|_| changed).into_iter().flatten() {
            let (start, end) = (display.x_at(range.start), display.x_at(range.end));
            if end > start {
                commands.push(DrawCommand::FillRect {
                    rect: Rect::new(viewport.body.x - viewport.scroll_x + start, y, end - start, line_height),
                    color: word,
                    radius: 0.0,
                });
            }
        }
        display.paint(diff.font_size,
            &mut commands,
            Point::new(viewport.body.x - viewport.scroll_x, text_y),
            (viewport.scroll_x, viewport.body.width),
            diff.palette.text,
            0,
            view.syntax.diff_line(&diff.lines, diff.language, row),
        );
        commands.push(DrawCommand::PopClip);
    }
    paint_scrollbars(&mut commands, viewport, diff.palette.muted);
    commands.push(DrawCommand::PopClip);
    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(range: std::ops::Range<usize>, shade: u8) -> CodeHighlight {
        CodeHighlight {
            range,
            color: Color::rgb(shade, shade, shade),
        }
    }

    fn first_containing(spans: &[CodeHighlight], offset: usize) -> Option<Color> {
        spans
            .iter()
            .find(|span| span.range.start <= offset && span.range.end > offset)
            .map(|span| span.color)
    }

    #[test]
    fn highlight_index_gives_every_offset_the_colour_a_full_scan_would() {
        // Nested, overlapping, adjacent and empty spans, ordered by start.
        let spans: Arc<[CodeHighlight]> = vec![
            span(0..400, 1),
            span(2..5, 2),
            span(2..40, 3),
            span(10..12, 4),
            span(10..10, 5),
            span(38..60, 6),
            span(60..61, 7),
            span(90..95, 8),
            span(300..301, 9),
            span(500..520, 10),
            span(505..506, 11),
            span(700..701, 12),
        ]
        .into();
        let mut index = HighlightIndex::default();
        index.sync((1, 1), &spans);
        let mut narrowed = 0;
        for start in (0..720).step_by(7) {
            for len in [1, 3, 16, 80] {
                let row = start..start + len;
                let slice = index.row(&spans, row.clone());
                narrowed += usize::from(slice.len() < spans.len());
                for offset in row {
                    assert_eq!(
                        first_containing(slice, offset),
                        first_containing(&spans, offset),
                        "offset {offset} in row {start}+{len}"
                    );
                }
            }
        }
        assert!(narrowed > 0);
        // Past the long span, a row sees only what can reach it.
        assert_eq!(index.row(&spans, 700..702).len(), 1);
        assert!(index.row(&spans, 600..650).is_empty());
        assert!(index.row(&spans, 9..9).len() <= spans.len());
    }

    #[test]
    fn highlight_index_falls_back_to_the_whole_list_when_it_is_not_ordered() {
        let spans: Arc<[CodeHighlight]> = vec![span(50..60, 1), span(0..5, 2), span(52..53, 3)].into();
        let mut index = HighlightIndex::default();
        index.sync((1, 1), &spans);
        assert_eq!(index.row(&spans, 0..3).len(), spans.len());
        assert_eq!(first_containing(index.row(&spans, 0..3), 1), first_containing(&spans, 1));
    }

    #[test]
    fn highlight_index_is_rebuilt_for_another_revision_or_list() {
        let first: Arc<[CodeHighlight]> = vec![span(0..5, 1), span(10..15, 2)].into();
        let second: Arc<[CodeHighlight]> = vec![span(0..5, 1), span(100..150, 2)].into();
        let mut index = HighlightIndex::default();
        index.sync((1, 1), &first);
        assert_eq!(index.row(&first, 10..12).len(), 1);
        index.sync((1, 2), &second);
        assert!(index.row(&second, 10..12).is_empty());
        assert_eq!(index.row(&second, 120..121).len(), 1);
        index.sync((1, 2), &Arc::from([]));
        assert!(index.row(&[], 0..10).is_empty());
    }

    #[test]
    fn highlights_are_carried_onto_an_edited_revision_until_new_ones_arrive() {
        let text = "let a = 1;\nlet b = 2;\nlet c = 3;\n";
        let mut buffer = TextBuffer::new(text).unwrap();
        let before = buffer.snapshot();
        let document = before.document_id();
        let spans: Arc<[CodeHighlight]> = (0..3)
            .flat_map(|row| [span(row * 11..row * 11 + 3, 1), span(row * 11 + 8..row * 11 + 9, 2)])
            .collect::<Vec<_>>()
            .into();
        let editor = |revision| {
            CodeEditor::new("editor", EditorController::new(TextBuffer::new("").unwrap()))
                .highlights(document, revision, spans.clone())
        };
        let syntax = crate::code_syntax::SyntaxCache::new();
        let mut index = HighlightIndex::default();
        let analysed = editor(before.revision());

        // Nothing to carry before the analysed revision has been seen.
        buffer
            .edit(11..11, "// note\n", Selection::caret(19), EditKind::Typing, 0)
            .unwrap();
        let inserted = buffer.snapshot();
        assert!(!index.follow(&analysed, &inserted));
        assert!(index.follow(&analysed, &before));
        assert!(index.follow(&analysed, &inserted));

        let row = |index: &HighlightIndex, snapshot: &BufferSnapshot, row: usize| {
            let range = snapshot.line_range(row).unwrap();
            index
                .lookup(&analysed, &syntax, snapshot, row, range)
                .iter()
                .map(|span| (span.range.clone(), span.color))
                .collect::<Vec<_>>()
        };
        let (keyword, number) = (Color::rgb(1, 1, 1), Color::rgb(2, 2, 2));
        assert_eq!(row(&index, &inserted, 0), [(0..3, keyword), (8..9, number)]);
        assert!(row(&index, &inserted, 1).is_empty());
        assert_eq!(row(&index, &inserted, 2), [(19..22, keyword), (27..28, number)]);
        assert_eq!(row(&index, &inserted, 3), [(30..33, keyword), (38..39, number)]);

        // A row retyped in place keeps its colours, clipped to its new length.
        buffer
            .edit(4..10, "", Selection::caret(4), EditKind::Typing, 0)
            .unwrap();
        let shortened = buffer.snapshot();
        assert_eq!(shortened.line_text(0).unwrap().trim_end_matches('\n'), "let ");
        assert!(index.follow(&analysed, &shortened));
        assert_eq!(row(&index, &shortened, 0), [(0..3, keyword)]);
        assert!(row(&index, &shortened, 1).is_empty());
        assert_eq!(row(&index, &shortened, 2), [(13..16, keyword), (21..22, number)]);
        assert_eq!(row(&index, &shortened, 3), [(24..27, keyword), (32..33, number)]);

        // Another list, or another document, is never carried.
        let other: Arc<[CodeHighlight]> = spans.to_vec().into();
        let replaced = CodeEditor::new("editor", EditorController::new(TextBuffer::new("").unwrap()))
            .highlights(document, before.revision(), other);
        assert!(!index.follow(&replaced, &shortened));
        assert!(index.follow(&editor(shortened.revision()), &shortened));
        let elsewhere = TextBuffer::new(text).unwrap().snapshot();
        assert!(!index.follow(&analysed, &elsewhere));
    }
    use crate::{CodeDiffLine, Frame, Size, UiTree};

    fn fixture(text: &str, width: f32) -> (EditorController, UiTree) {
        let controller = EditorController::new(TextBuffer::new(text).unwrap());
        let mut tree = UiTree::new(
            CodeEditor::new("editor:view:1", controller.clone())
                .height(140.0)
                .render(),
        );
        tree.layout(Size::new(width, 140.0));
        assert!(tree.focus("editor:view:1"));
        (controller, tree)
    }
    fn key(tree: &mut UiTree, key: Key, shift: bool, ctrl: bool) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key { key, shift, ctrl })
    }
    fn text(controller: &EditorController) -> String {
        let snapshot = controller.snapshot().unwrap();
        snapshot.text_range(0..snapshot.char_count()).unwrap()
    }

    #[test]
    fn native_editor_unicode_paste_copy_cut_and_undo_keep_exact_source_and_selection() {
        let (controller, mut tree) = fixture("", 360.0);
        let source = "ñ\r\ne\u{301} 🧑‍💻\t終";
        let actions = tree.handle_event(UiEvent::Paste(source.into()));
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(actions[0],UiAction::EditorChanged {document_id,revision:1,..} if document_id==controller.snapshot().unwrap().document_id())
        );
        assert_eq!(text(&controller), source);
        key(&mut tree, Key::A, false, true);
        assert_eq!(
            key(&mut tree, Key::C, false, true),
            vec![UiAction::CopyRequested {
                text: source.into()
            }]
        );
        let before = controller.shared_buffer().lock().unwrap().selection();
        let cut = key(&mut tree, Key::X, false, true);
        assert_eq!(cut.len(), 2);
        assert_eq!(text(&controller), "");
        key(&mut tree, Key::Z, false, true);
        assert_eq!(text(&controller), source);
        assert_eq!(
            controller.shared_buffer().lock().unwrap().selection(),
            before
        );
        key(&mut tree, Key::End, false, true);
        key(&mut tree, Key::Backspace, false, false);
        assert_eq!(text(&controller), "ñ\r\ne\u{301} 🧑‍💻\t");
        key(&mut tree, Key::Backspace, false, false);
        key(&mut tree, Key::Backspace, false, false);
        assert_eq!(text(&controller), "ñ\r\ne\u{301} ");
        let save = key(&mut tree, Key::S, false, true);
        assert!(
            matches!(save.as_slice(),[UiAction::EditorSaveRequested {revision,..}] if *revision==controller.snapshot().unwrap().revision())
        );
        assert_eq!(tree.input_value("editor:view:1"), None);
    }

    #[test]
    fn native_editor_click_shift_drag_and_vertical_navigation_use_grapheme_boundaries() {
        let controller =
            EditorController::new(TextBuffer::new("a\te\u{301}終\r\nnext line\r\nend").unwrap());
        let mut view = EditorView::new(CodeEditor::new("view", controller.clone()), Instant::now());
        let measure = |text: &str| text.graphemes(true).count() as f32 * 8.0;
        view.paint(
            Rect::new(0.0, 0.0, 360.0, 140.0),
            Rect::new(0.0, 0.0, 360.0, 140.0),
            true,
            &mut |_, _, _| true,
            &measure,
            16.0,
        );
        let first = Point::new(view.viewport.body.x + 32.0, view.viewport.body.y + 5.0);
        view.point(first, false, true, &measure);
        view.point(first, true, false, &measure);
        assert_eq!(
            controller.shared_buffer().lock().unwrap().selection().head,
            2
        );
        let second = Point::new(view.viewport.body.x + 16.0, view.viewport.body.y + 25.0);
        view.point(second, true, true, &measure);
        view.point(second, true, false, &measure);
        let selection = controller.shared_buffer().lock().unwrap().selection();
        assert_eq!(selection.anchor, 2);
        assert_eq!(selection.head, 9);
        view.input(
            &UiEvent::Key {
                key: Key::ArrowUp,
                shift: false,
                ctrl: false,
            },
            Instant::now(),
            &measure,
        );
        let head = controller.shared_buffer().lock().unwrap().selection().head;
        assert!(head == 1 || head == 2);
        assert!(controller.snapshot().unwrap().prev_grapheme(head).unwrap() <= head);
    }

    #[test]
    fn native_editor_virtualizes_rows_and_large_horizontal_windows_at_both_widths() {
        let source = (0..50_000)
            .map(|row| format!("row {row} content\n"))
            .collect::<String>();
        for width in [360.0, 900.0] {
            let (_, mut tree) = fixture(&source, width);
            let frame = tree.layout(Size::new(width, 140.0));
            assert!(
                frame
                    .commands
                    .iter()
                    .filter(|command| matches!(command, DrawCommand::Text { .. }))
                    .count()
                    <= 16
            );
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(100.0, 40.0),
                delta_y: -20_000.0,
            });
            let frame = tree.layout(Size::new(width, 140.0));
            assert!(frame.commands.iter().any(
                |command| matches!(command,DrawCommand::Text {text,..} if text.contains("row 1000"))
            ));
        }
        let giant = "x".repeat(1_000_000);
        let (_, mut tree) = fixture(&giant, 360.0);
        let frame = tree.layout(Size::new(360.0, 140.0));
        let payload: usize = frame
            .commands
            .iter()
            .filter_map(|command| {
                if let DrawCommand::Text { text, .. } = command {
                    Some(text.len())
                } else {
                    None
                }
            })
            .sum();
        assert!(payload < 1024);
        tree.handle_event(UiEvent::Scroll2D {
            position: Point::new(100.0, 30.0),
            delta_x: -240.0,
            delta_y: 0.0,
            direct: true,
            shift: false,
        });
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(
            frame
                .commands
                .iter()
                .filter_map(|command| if let DrawCommand::Text { text, .. } = command {
                    Some(text.len())
                } else {
                    None
                })
                .sum::<usize>()
                < 1024
        );
    }

    #[test]
    fn native_editor_read_only_and_reduced_motion_do_not_mutate_or_animate() {
        let (controller, mut tree) = fixture("let café = 1;", 900.0);
        tree.set_root(
            CodeEditor::new("editor:view:1", controller.clone())
                .read_only(true)
                .height(140.0)
                .render(),
        );
        tree.set_reduced_motion(true);
        let before = tree.layout(Size::new(900.0, 140.0));
        assert!(tree.handle_event(UiEvent::Paste("oops".into())).is_empty());
        assert!(key(&mut tree, Key::Backspace, false, false).is_empty());
        assert!(key(&mut tree, Key::Z, false, true).is_empty());
        assert_eq!(controller.snapshot().unwrap().revision(), 0);
        assert_eq!(text(&controller), "let café = 1;");
        assert_eq!(
            before.commands,
            tree.layout(Size::new(900.0, 140.0)).commands
        );
        assert!(!tree.is_animating());
    }

    #[test]
    fn view_options_draw_whitespace_rulers_and_caret_shapes_and_can_hide_line_and_guides() {
        let (controller, mut tree) = fixture("    a b\tc", 900.0);
        let commands = |tree: &mut UiTree, editor: CodeEditor| {
            tree.set_root(editor.height(140.0).render());
            tree.layout(Size::new(900.0, 140.0)).commands
        };
        let texts = |commands: &[DrawCommand]| -> Vec<String> {
            commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        let rects = |commands: &[DrawCommand]| -> Vec<Rect> {
            commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, .. } => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let base = || CodeEditor::new("editor:view:1", controller.clone()).caret_blink(false);
        let plain = commands(&mut tree, base());
        assert!(!texts(&plain).iter().any(|text| text == "·" || text == "→"));

        let marked = commands(&mut tree, base().show_whitespace(true));
        let marks = texts(&marked);
        assert_eq!(marks.iter().filter(|text| *text == "·").count(), 5);
        assert_eq!(marks.iter().filter(|text| *text == "→").count(), 1);

        let ruled = commands(&mut tree, base().rulers([0, 80, 2000]));
        let tall = |commands: &[DrawCommand]| {
            rects(commands).into_iter().filter(|rect| rect.width == 1.0 && rect.height > 100.0).count()
        };
        assert_eq!(tall(&ruled), tall(&plain) + 1);

        let line = rects(&plain);
        let underline = rects(&commands(&mut tree, base().caret_style(CaretStyle::Underline)));
        assert!(line.iter().any(|rect| rect.width == 1.5 && rect.height > 2.0));
        assert!(underline.iter().any(|rect| rect.height == 2.0 && rect.width > 1.5));
        assert!(!underline.iter().any(|rect| rect.width == 1.5));

        let full_width = |commands: &[DrawCommand]| rects(commands).into_iter().filter(|rect| rect.width >= 899.0).count();
        let bare = commands(&mut tree, base().highlight_current_line(false).indent_guides(false));
        assert_eq!(full_width(&bare) + 1, full_width(&plain));
        assert!(rects(&bare).len() < rects(&plain).len() - 1, "the indent guide is gone too");
    }

    #[test]
    fn native_editor_indent_transactions_preserve_crlf_and_undo_selection() {
        let (controller, mut tree) = fixture("one\r\ntwo\r\nthree", 360.0);
        key(&mut tree, Key::A, false, true);
        let before = controller.shared_buffer().lock().unwrap().selection();
        key(&mut tree, Key::Tab, false, false);
        assert_eq!(text(&controller), "    one\r\n    two\r\n    three");
        key(&mut tree, Key::Z, false, true);
        assert_eq!(text(&controller), "one\r\ntwo\r\nthree");
        assert_eq!(
            controller.shared_buffer().lock().unwrap().selection(),
            before
        );
        key(&mut tree, Key::Tab, false, false);
        key(&mut tree, Key::Tab, true, false);
        assert_eq!(text(&controller), "one\r\ntwo\r\nthree");
    }

    #[test]
    fn native_editor_scrollbar_drag_changes_viewport_without_changing_selection() {
        let (_, mut tree) = fixture(&"long long long long long line\n".repeat(1000), 360.0);
        tree.handle_event(UiEvent::PointerDown(Point::new(357.0, 100.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(357.0, 120.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(357.0, 120.0)));
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(
            !frame
                .commands
                .iter()
                .any(|command| matches!(command,DrawCommand::Text {text,..} if text=="1"))
        );
    }

    #[test]
    fn reveal_inset_moves_first_lines_only_on_navigation_and_preserves_pointer_scrolling() {
        let source = "first\nsecond\nthird";
        let (controller, mut tree) = fixture(source, 360.0);
        let first_position = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == "first" => Some(*position),
                    _ => None,
                })
                .expect("first line is visible")
        };
        let baseline = first_position(&tree.layout(Size::new(360.0, 140.0)));
        tree.set_root(
            CodeEditor::new("editor:view:1", controller.clone())
                .height(140.0)
                .reveal_top_inset(60.0)
                .render(),
        );
        assert_eq!(
            first_position(&tree.layout(Size::new(360.0, 140.0))),
            baseline,
            "opening an overlay must not move the document"
        );
        assert!(tree.reveal_code_caret("editor:view:1"));
        let revealed = tree.layout(Size::new(360.0, 140.0));
        assert_eq!(first_position(&revealed).y, baseline.y + 60.0);

        let second = Point::new(baseline.x + 1.0, baseline.y + 60.0 + LINE_HEIGHT + 4.0);
        tree.handle_event(UiEvent::PointerDown(second));
        tree.handle_event(UiEvent::PointerUp(second));
        assert_eq!(controller.snapshot().unwrap().selection().head, 6);
        tree.layout(Size::new(360.0, 140.0));
        let appearance = crate::ScrollbarAppearance::default();
        let thumb_x = 360.0 - 8.0 + (8.0 - appearance.thumb_width) / 2.0;
        for (delta_y, expected_y) in [(-1000.0, baseline.y), (1000.0, baseline.y + 60.0)] {
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(100.0, 40.0),
                delta_y,
            });
            let painted = tree.layout(Size::new(360.0, 140.0));
            assert_eq!(first_position(&painted).y, expected_y);
            let thumb = painted.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. }
                    if rect.x == thumb_x
                        && rect.width == appearance.thumb_width
                        && *radius == appearance.thumb_width / 2.0 =>
                {
                    Some(*rect)
                }
                _ => None,
            });
            let thumb = thumb.expect("temporary top space has a usable scrollbar");
            assert!(thumb.y >= 4.0 && thumb.y + thumb.height <= 132.0);
        }
        let selection = controller.snapshot().unwrap().selection();
        tree.handle_event(UiEvent::PointerDown(Point::new(357.0, 4.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(357.0, 132.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(357.0, 132.0)));
        assert_eq!(
            first_position(&tree.layout(Size::new(360.0, 140.0))).y,
            baseline.y
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(357.0, 132.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(357.0, 4.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(357.0, 4.0)));
        assert_eq!(
            first_position(&tree.layout(Size::new(360.0, 140.0))).y,
            baseline.y + 60.0
        );
        assert_eq!(controller.snapshot().unwrap().selection(), selection);
        tree.set_root(
            CodeEditor::new("editor:view:1", controller.clone())
                .height(140.0)
                .render(),
        );
        assert_eq!(
            first_position(&tree.layout(Size::new(360.0, 140.0))),
            baseline
        );
        assert_eq!(text(&controller), source);
    }

    #[test]
    fn host_rerender_preserves_viewport_and_document_replacement_resets_it() {
        let (controller, mut tree) = fixture(&"unchanged row\n".repeat(1000), 360.0);
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(100.0, 30.0),
            delta_y: -2000.0,
        });
        let before = tree.layout(Size::new(360.0, 140.0));
        tree.set_root(
            CodeEditor::new("editor:view:1", controller)
                .height(140.0)
                .render(),
        );
        assert_eq!(
            before.commands,
            tree.layout(Size::new(360.0, 140.0)).commands
        );
        tree.set_root(
            CodeEditor::new(
                "editor:view:1",
                EditorController::new(TextBuffer::new("replacement").unwrap()),
            )
            .height(140.0)
            .render(),
        );
        assert!(
            tree.layout(Size::new(360.0, 140.0)).commands.iter().any(
                |command| matches!(command,DrawCommand::Text {text,..} if text=="replacement")
            )
        );
    }

    #[test]
    fn a_diagnostic_shows_its_message_once_the_pointer_rests_on_it() {
        let controller = EditorController::new(TextBuffer::new("let total = missing;\nok").unwrap());
        let snapshot = controller.snapshot().unwrap();
        let mark = Color::rgb(250, 60, 70);
        let message = "cannot find value `missing`";
        let mut tree = UiTree::new(
            CodeEditor::new("editor:diagnostic", controller.clone())
                .height(140.0)
                .diagnostics(
                    snapshot.document_id(),
                    snapshot.revision(),
                    vec![CodeDiagnostic {
                        range: 12..19,
                        color: mark,
                        message: message.into(),
                    }],
                )
                .render(),
        );
        let size = Size::new(480.0, 140.0);
        let start = Instant::now();
        let shown = |frame: &Frame| {
            frame
                .commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == message))
        };
        let frame = tree.layout_at(size, start);
        let underline = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == mark && rect.height == 1.5 => {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        assert!(!shown(&frame));

        let over = Point::new(underline.x + underline.width * 0.5, underline.y - 6.0);
        tree.handle_event(UiEvent::PointerMove(over));
        assert!(!shown(&tree.layout_at(size, start)));
        assert!(tree.animation_repaint_after().is_some());
        let rested = start + crate::tree::TOOLTIP_HOVER_DELAY;
        assert!(shown(&tree.layout_at(size, rested)));

        tree.handle_event(UiEvent::PointerMove(Point::new(over.x, over.y + LINE_HEIGHT)));
        assert!(!shown(&tree.layout_at(size, rested)));
        let dot = tree
            .layout_at(size, rested)
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == mark && rect.height == 5.0 => {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        tree.handle_event(UiEvent::PointerMove(Point::new(dot.x + 2.0, dot.y + 2.0)));
        assert!(!shown(&tree.layout_at(size, rested)));
        assert!(shown(&tree.layout_at(size, rested + crate::tree::TOOLTIP_HOVER_DELAY)));
        let later = rested + crate::tree::TOOLTIP_HOVER_DELAY;
        tree.handle_event(UiEvent::PointerMove(Point::new(over.x, over.y + LINE_HEIGHT)));
        assert!(!shown(&tree.layout_at(size, later)));
        tree.handle_event(UiEvent::PointerMove(over));
        assert!(!shown(&tree.layout_at(size, later)));
        assert!(shown(&tree.layout_at(size, later + crate::tree::TOOLTIP_HOVER_DELAY)));

        controller
            .execute(EditorCommand::InsertText("x".into()))
            .unwrap();
        assert!(!shown(&tree.layout_at(size, later + crate::tree::TOOLTIP_HOVER_DELAY)));
    }

    #[test]
    fn external_highlights_require_both_document_identity_and_revision() {
        let controller = EditorController::new(TextBuffer::new("literal").unwrap());
        let snapshot = controller.snapshot().unwrap();
        let custom = Color::rgb(1, 222, 111);
        let highlighted = |document_id, revision| {
            CodeEditor::new("editor", controller.clone())
                .highlights(
                    document_id,
                    revision,
                    vec![CodeHighlight {
                        range: 0..snapshot.char_count(),
                        color: custom,
                    }],
                )
                .render()
        };
        let has_custom = |frame: Frame| {
            frame
                .commands
                .iter()
                .any(|command| matches!(command,DrawCommand::Text {color,..} if *color==custom))
        };
        let mut tree = UiTree::new(highlighted(snapshot.document_id(), snapshot.revision()));
        assert!(has_custom(tree.layout(Size::new(360.0, 140.0))));
        tree.set_root(highlighted(
            TextBuffer::new("other document").unwrap().document_id(),
            snapshot.revision(),
        ));
        assert!(!has_custom(tree.layout(Size::new(360.0, 140.0))));
        tree.set_root(highlighted(snapshot.document_id(), snapshot.revision() + 1));
        assert!(!has_custom(tree.layout(Size::new(360.0, 140.0))));
    }

    #[test]
    fn forgetting_an_unmounted_view_releases_its_shared_document_controller() {
        let shared = Arc::new(Mutex::new(TextBuffer::new("retained document").unwrap()));
        let weak = Arc::downgrade(&shared);
        let mut tree = UiTree::new(
            CodeEditor::new(
                "editor:retired",
                EditorController::from_shared(shared.clone()),
            )
            .render(),
        );
        tree.layout(Size::new(360.0, 140.0));
        tree.set_root(Element::spacer());
        tree.layout(Size::new(360.0, 140.0));
        drop(shared);
        assert!(weak.upgrade().is_some(), "unmounted view retains its state");
        tree.forget_code_view("editor:retired");
        assert!(
            weak.upgrade().is_none(),
            "retirement releases the controller"
        );
    }

    #[test]
    fn code_diff_keeps_original_rows_and_virtualizes_journal_line_numbers() {
        let lines: Arc<[CodeDiffLine]> = (0..10_000)
            .map(|row| CodeDiffLine {
                kind: if row % 3 == 0 {
                    CodeDiffLineKind::Added
                } else if row % 3 == 1 {
                    CodeDiffLineKind::Removed
                } else {
                    CodeDiffLineKind::Context
                },
                old_line: Some(row + 1),
                new_line: Some(row + 2),
                text: format!("literal row {row}\r\n"),
            })
            .collect::<Vec<_>>()
            .into();
        let original = lines.clone();
        let mut tree = UiTree::new(CodeDiff::new("review", lines).height(140.0).render());
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count()
                <= 32
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(100.0, 30.0),
            delta_y: -2000.0,
        });
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(frame.commands.iter().any(|command|matches!(command,DrawCommand::Text {text,..} if text.contains("literal row 100"))));
        assert_eq!(original[0].text, "literal row 0\r\n");
        assert!(
            !tree
                .handle_event(UiEvent::Text("bad".into()))
                .iter()
                .any(|action| matches!(action, UiAction::EditorChanged { .. }))
        );
    }

    #[test]
    fn reading_gutter_uses_one_number_and_retains_change_colors_without_marker_column() {
        let lines: Arc<[CodeDiffLine]> = vec![
            CodeDiffLine {
                kind: CodeDiffLineKind::Context,
                old_line: Some(10),
                new_line: Some(20),
                text: "alpha".into(),
            },
            CodeDiffLine {
                kind: CodeDiffLineKind::Added,
                old_line: None,
                new_line: Some(21),
                text: "beta".into(),
            },
            CodeDiffLine {
                kind: CodeDiffLineKind::Removed,
                old_line: Some(12),
                new_line: None,
                text: "gamma".into(),
            },
        ]
        .into();
        let bounds = Rect::new(0.0, 0.0, 360.0, 140.0);
        let measure = |text: &str| text.chars().count() as f32 * 8.0;
        let mut view = DiffView::default();
        let diff = CodeDiff::new("content", lines.clone());
        let default = paint_diff(&diff, &mut view, bounds, bounds, &measure, 16.0);
        let original_x = view.viewport.body.x;
        let labels = |commands: &[DrawCommand]| {
            commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let full = labels(&default);
        for expected in ["10", "20", "21", "12", "+", "−"] {
            assert!(full.iter().any(|text| text == expected), "{expected}");
        }
        let reading = diff.clone().single_gutter(true);
        let commands = paint_diff(&reading, &mut view, bounds, bounds, &measure, 16.0);
        let text = labels(&commands);
        assert!(
            !text
                .iter()
                .any(|text| matches!(text.as_str(), "10" | "+" | "−" | " "))
        );
        for expected in ["20", "21", "12"] {
            assert_eq!(text.iter().filter(|text| *text == expected).count(), 1);
        }
        assert!(view.viewport.body.x < original_x);
        let fills = |commands: &[DrawCommand]| {
            commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } => Some((*rect, *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(fills(&commands), fills(&default));
        assert_eq!(
            fills(&commands).len(),
            4,
            "two changed rows, each with a gutter stripe"
        );
        assert!(Arc::ptr_eq(view.identity.as_ref().unwrap(), &lines));
        paint_diff(&diff, &mut view, bounds, bounds, &measure, 16.0);
        assert_eq!(view.viewport.body.x, original_x);
        assert_eq!(lines[0].old_line, Some(10));
    }

    #[test]
    fn diff_changes_keep_editor_foreground_and_use_subtle_background_with_either_gutter() {
        let lines: Arc<[CodeDiffLine]> = vec![
            CodeDiffLine {
                kind: CodeDiffLineKind::Added,
                old_line: None,
                new_line: Some(1),
                text: "new value".into(),
            },
            CodeDiffLine {
                kind: CodeDiffLineKind::Removed,
                old_line: Some(1),
                new_line: None,
                text: "old value".into(),
            },
        ]
        .into();
        let diff = CodeDiff::new("review", lines);
        let bounds = Rect::new(0.0, 0.0, 360.0, 140.0);
        for single_gutter in [false, true] {
            let diff = diff.clone().single_gutter(single_gutter);
            let commands = paint_diff(
                &diff,
                &mut DiffView::default(),
                bounds,
                bounds,
                &|s| s.len() as f32 * 8.0,
                16.0,
            );
            for expected in ["new value", "old value"] {
                assert!(commands.iter().any(|command|matches!(command,DrawCommand::Text {text,color,..} if text==expected && *color==diff.palette.text)));
            }
            assert_eq!(commands.iter().filter(|c|matches!(c,DrawCommand::FillRect {rect,color,..} if rect.width==360.0 && color.a > 0 && color.a<=24)).count(),2);
            assert_eq!(commands.iter().filter(|c|matches!(c,DrawCommand::FillRect {rect,color,..} if rect.width==2.0 && color.a==255)).count(),2);
            assert!(commands.iter().any(|c| matches!(c, DrawCommand::FillRect {rect,color,..} if rect.width == 360.0 && color.g > color.r)), "added row must have a green background");
            assert!(commands.iter().any(|c| matches!(c, DrawCommand::FillRect {rect,color,..} if rect.width == 360.0 && color.r > color.g)), "removed row must have a red background");
        }
    }

    #[test]
    fn captured_content_without_a_before_snapshot_stays_neutral_with_either_gutter() {
        let lines: Arc<[CodeDiffLine]> = vec![CodeDiffLine {
            kind: CodeDiffLineKind::Context,
            old_line: None,
            new_line: Some(1),
            text: "captured content\r\n".into(),
        }]
        .into();
        let bounds = Rect::new(0.0, 0.0, 360.0, 140.0);
        for single_gutter in [false, true] {
            let diff = CodeDiff::new("captured", lines.clone()).single_gutter(single_gutter);
            let commands = paint_diff(
                &diff,
                &mut DiffView::default(),
                bounds,
                bounds,
                &|s| s.len() as f32 * 8.0,
                16.0,
            );
            assert!(
                !commands
                    .iter()
                    .any(|c| matches!(c, DrawCommand::FillRect { .. }))
            );
            assert!(
                commands.iter().any(
                    |c| matches!(c, DrawCommand::Text {text,..} if text == "captured content")
                )
            );
        }
        assert_eq!(lines[0].text, "captured content\r\n");
    }

    #[test]
    fn reading_gutter_recalculates_horizontal_extent_and_clamps_old_scroll() {
        let lines: Arc<[CodeDiffLine]> = vec![CodeDiffLine {
            kind: CodeDiffLineKind::Context,
            old_line: Some(1),
            new_line: Some(1),
            text: "abcdefghijklmnopqrst".into(),
        }]
        .into();
        let bounds = Rect::new(0.0, 0.0, 200.0, 80.0);
        let measure = |text: &str| text.chars().count() as f32 * 8.0;
        let mut view = DiffView::default();
        let diff = CodeDiff::new("content", lines);
        paint_diff(&diff, &mut view, bounds, bounds, &measure, 16.0);
        assert!(view.viewport.max_x > 0.0);
        view.viewport.scroll_x = view.viewport.max_x;
        let reading = diff.single_gutter(true);
        let commands = paint_diff(&reading, &mut view, bounds, bounds, &measure, 16.0);
        assert_eq!(
            view.viewport.max_x, 0.0,
            "the wider body fits the entire line"
        );
        assert_eq!(view.viewport.scroll_x, 0.0);
        assert!(commands.iter().any(|command| matches!(command,
            DrawCommand::PushClip(rect) if *rect == view.viewport.body.intersection(bounds).unwrap()
        )));
        let content: String = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, .. } if position.x >= view.viewport.body.x => {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(content, "abcdefghijklmnopqrst");
    }

    #[test]
    fn reading_gutter_keeps_large_content_virtualized() {
        let lines: Arc<[CodeDiffLine]> = (0..10_000)
            .map(|row| CodeDiffLine {
                kind: CodeDiffLineKind::Context,
                old_line: Some(row + 1),
                new_line: Some(row + 1),
                text: format!("captured row {row}"),
            })
            .collect::<Vec<_>>()
            .into();
        let mut tree = UiTree::new(
            CodeDiff::new("reading", lines)
                .single_gutter(true)
                .height(140.0)
                .render(),
        );
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::Text { .. }))
                .count()
                < 24
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(100.0, 30.0),
            delta_y: -2000.0,
        });
        let frame = tree.layout(Size::new(360.0, 140.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.contains("captured row 100")
        )));
    }

    #[test]
    fn streaming_preview_follows_appends_pauses_for_manual_scroll_and_resumes_at_bottom() {
        let preview = |count, follow| {
            let lines: Arc<[CodeDiffLine]> = (0..count)
                .map(|row| CodeDiffLine {
                    kind: CodeDiffLineKind::Context,
                    old_line: None,
                    new_line: Some(row + 1),
                    text: format!("stream row {row}"),
                })
                .collect::<Vec<_>>()
                .into();
            CodeDiff::new("stream", lines)
                .single_gutter(true)
                .follow_tail(follow)
                .height(120.0)
                .render()
        };
        let visible = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } if text.starts_with("stream row ") => {
                        Some(text.clone())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let size = Size::new(360.0, 120.0);
        let mut tree = UiTree::new(preview(2, true));
        let frame = tree.layout(size);
        assert!(visible(&frame).contains(&"stream row 0".into()));
        for count in [12, 24] {
            tree.set_root(preview(count, true));
            let frame = tree.layout(size);
            assert!(visible(&frame).contains(&format!("stream row {}", count - 1)));
        }
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(100.0, 30.0),
            delta_y: 100.0,
        });
        let manual = visible(&tree.layout(size));
        assert!(!manual.contains(&"stream row 23".into()));
        tree.set_root(preview(28, true));
        assert_eq!(
            visible(&tree.layout(size)),
            manual,
            "new rows must preserve a manual reader"
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(100.0, 30.0),
            delta_y: -1_000_000.0,
        });
        assert!(visible(&tree.layout(size)).contains(&"stream row 27".into()));
        tree.set_root(preview(36, true));
        let bottom = visible(&tree.layout(size));
        assert!(bottom.contains(&"stream row 35".into()));
        tree.set_root(preview(50, false));
        assert_eq!(
            visible(&tree.layout(size)),
            bottom,
            "static confirmed diffs must not follow appends"
        );
    }

    #[test]
    fn preview_follow_tail_has_two_pixel_tolerance_and_is_opt_in() {
        let lines = |count| -> Arc<[CodeDiffLine]> {
            (0..count)
                .map(|row| CodeDiffLine {
                    kind: CodeDiffLineKind::Context,
                    old_line: None,
                    new_line: Some(row + 1),
                    text: format!("line {row}"),
                })
                .collect::<Vec<_>>()
                .into()
        };
        let bounds = Rect::new(0.0, 0.0, 360.0, 120.0);
        let measure = |text: &str| text.len() as f32 * 8.0;
        for (distance, follows) in [(2.0, true), (2.1, false)] {
            let mut view = DiffView::default();
            let initial = CodeDiff::new("stream", lines(12)).follow_tail(true);
            paint_diff(&initial, &mut view, bounds, bounds, &measure, 16.0);
            view.viewport.scroll_y = view.viewport.max_y - distance;
            let position = view.viewport.scroll_y;
            let next = CodeDiff::new("stream", lines(16)).follow_tail(true);
            paint_diff(&next, &mut view, bounds, bounds, &measure, 16.0);
            assert_eq!(
                view.viewport.scroll_y,
                if follows {
                    view.viewport.max_y
                } else {
                    position
                }
            );
        }
        let mut view = DiffView::default();
        let review = CodeDiff::new("review", lines(40));
        assert!(!review.follow_tail);
        paint_diff(&review, &mut view, bounds, bounds, &measure, 16.0);
        assert_eq!(view.viewport.scroll_y, 0.0);
    }
}
