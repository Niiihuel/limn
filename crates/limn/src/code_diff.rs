//! Original, virtualized review rows. The host supplies journal-backed content.
use crate::{Access, Color, CodeLanguage, Component, ComponentPalette, Element, FontFamily, Icon, Role};
use std::ops::Range;
use std::sync::Arc;

/// Whether a row is unchanged, added or removed. It sets the row's marker and colours.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeDiffLineKind {
    /// A line present on both sides: a blank marker, no fill, and it advances the syntax state
    /// of both the old and the new text.
    Context,
    /// A line only in the new text: a `+` marker and a green tint across the row.
    Added,
    /// A line only in the old text: a `−` marker and a red tint across the row.
    Removed,
    /// A run of unchanged lines the host left out. The row shows its `text` as a label, such
    /// as "Show 12 more lines", and no line numbers; activating it reports
    /// `Clicked { id: "{id}:expand:{row}" }`, `row` being its index in `lines`, for the host to
    /// put the lines in its place. Syntax colouring restarts after it.
    Folded,
}

/// The colours of changed rows. The default is the diff's own green and red.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiffColors {
    /// The tint across an added row.
    pub add_row: Color,
    /// The stronger tint behind the changed words of an added row.
    pub add_word: Color,
    /// The tint across a removed row.
    pub del_row: Color,
    /// The stronger tint behind the changed words of a removed row.
    pub del_word: Color,
    /// The `+` marker and the margin bar of an added row, and a header's count of added lines.
    pub add_bar: Color,
    /// The `−` marker and the margin bar of a removed row, and a header's count of removed
    /// lines.
    pub del_bar: Color,
}

impl Default for DiffColors {
    fn default() -> Self {
        Self {
            add_row: Color::rgba(72, 142, 92, 20),
            add_word: Color::rgba(72, 142, 92, 72),
            del_row: Color::rgba(170, 76, 85, 20),
            del_word: Color::rgba(170, 76, 85, 72),
            add_bar: Color::rgb(105, 185, 135),
            del_bar: Color::rgb(210, 115, 122),
        }
    }
}

/// Text is retained exactly as supplied; the view never applies a patch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeDiffLine {
    /// Which side of the change the row belongs to.
    pub kind: CodeDiffLineKind,
    /// The numbers shown in the old and the new line-number column, as given. `None` leaves
    /// that column empty on the row.
    pub old_line: Option<usize>,
    /// The number in the new column, and the one a single gutter shows when it is set. A row
    /// with neither number is not syntax-coloured and restarts the colouring after it.
    pub new_line: Option<usize>,
    /// The line's text. One trailing CR, LF or CRLF is not drawn.
    pub text: String,
}

/// A read-only, scrollable list of diff rows with line numbers and change colours.
#[derive(Clone, Debug)]
pub struct CodeDiff {
    /// Identifier of the view: its scroll position is kept under it, and its focus when
    /// `focusable` is set.
    pub id: String,
    /// The rows, top to bottom. Only the rows in view are measured and painted.
    pub lines: Arc<[CodeDiffLine]>,
    /// Colours of the surface, the text, and the line numbers and scrollbars (`muted`). Set
    /// with `palette`.
    pub palette: ComponentPalette,
    /// Height in logical pixels, 320 by default. `height` keeps it at 24 or more.
    pub height: f32,
    /// Code and gutter size in logical pixels, 13 by default.
    pub font_size: f32,
    /// Paint full row tints; disabling these preserves change markers.
    pub diff_backgrounds: bool,
    /// Columns between tab stops, 4 by default. `tab_width` clamps it to 1..=16.
    pub tab_width: usize,
    /// Language the rows are syntax-coloured as. Plain text by default.
    pub language: CodeLanguage,
    /// Show one line-number column (the new number, else the old one) and no +/- marker
    /// column. False by default.
    pub single_gutter: bool,
    /// Scroll to the new bottom when rows are appended while the view is at the bottom.
    /// False by default.
    pub follow_tail: bool,
    /// Standalone read-only documents may own focus; inline activity previews do not.
    pub focusable: bool,
    /// The changed words of each row, in Unicode scalar offsets into its text, end-exclusive:
    /// entry `i` belongs to `lines[i]`. Painted on added and removed rows only. Empty by
    /// default.
    pub word_ranges: Arc<[Vec<Range<usize>>]>,
    /// Paint a bar at the left edge of added and removed rows. True by default.
    pub margin_bar: bool,
    /// The colours of changed rows, words, markers and bars.
    pub colors: DiffColors,
}

impl CodeDiff {
    /// A diff over `lines` with the neutral dark palette: 320 pixels high, tab width 4, plain
    /// text, two line-number columns, not focusable and not following its tail.
    pub fn new(id: impl Into<String>, lines: impl Into<Arc<[CodeDiffLine]>>) -> Self {
        Self {
            id: id.into(),
            lines: lines.into(),
            palette: ComponentPalette::neutral_dark(),
            height: 320.0,
            font_size: 13.0,
            diff_backgrounds: true,
            tab_width: 4,
            language: CodeLanguage::Plain,
            single_gutter: false,
            follow_tail: false,
            focusable: false,
            word_ranges: Arc::from([]),
            margin_bar: true,
            colors: DiffColors::default(),
        }
    }
    /// Use this palette for the surface, the text and the gutter.
    pub fn palette(mut self, palette: ComponentPalette) -> Self {
        self.palette = palette;
        self
    }
    /// Set code and gutter size, including row layout. Nonpositive or nonfinite
    /// values are ignored; valid sizes stay within 1..=96 logical pixels.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size.clamp(1.0, 96.0);
        }
        self
    }
    /// Show tinted added and removed row backgrounds. True by default.
    /// Disabling them preserves the +/- markers and edge bands.
    pub fn diff_backgrounds(mut self, enabled: bool) -> Self {
        self.diff_backgrounds = enabled;
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
    /// Set the tab stop in columns, clamped to 1..=16.
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab_width = width.clamp(1, 16);
        self
    }
    /// The language used to colour the text of the rows. The default is plain text.
    pub fn language(mut self, language: CodeLanguage) -> Self {
        self.language = language;
        self
    }
    /// Use one line-number column and omit the separate +/- marker column.
    /// Added/removed rows retain their change colors; captured or proposed
    /// content remains neutral when supplied as `CodeDiffLineKind::Context`.
    pub fn single_gutter(mut self, enabled: bool) -> Self {
        self.single_gutter = enabled;
        self
    }
    /// Opt into pointer and Tab focus for a standalone read-only document.
    /// This does not make diff rows editable or add them to inline chat navigation.
    pub fn focusable(mut self, enabled: bool) -> Self {
        self.focusable = enabled;
        self
    }
    /// Follow appended rows only while the reader remains at the bottom.
    /// Static reviews leave this disabled; scrolling upward pauses following.
    pub fn follow_tail(mut self, enabled: bool) -> Self {
        self.follow_tail = enabled;
        self
    }
    /// Tint the changed words of added and removed rows with `add_word` or `del_word`, over
    /// the row's tint. Entry `i` holds the ranges of `lines[i]`, in Unicode scalar offsets;
    /// rows past the end have none. Keep it in the state with the rows, as an `Arc`.
    pub fn word_ranges(mut self, ranges: impl Into<Arc<[Vec<Range<usize>>]>>) -> Self {
        self.word_ranges = ranges.into();
        self
    }
    /// Show the bar at the left edge of added and removed rows. True by default.
    pub fn margin_bar(mut self, enabled: bool) -> Self {
        self.margin_bar = enabled;
        self
    }
    /// Use these colours for changed rows, words, markers and bars.
    pub fn colors(mut self, colors: DiffColors) -> Self {
        self.colors = colors;
        self
    }
}

/// The control laid over a `Folded` row: its label, reporting `{id}:expand:{row}`.
pub(crate) fn expand_control(diff: &CodeDiff, row: usize, indent: f32, size: (f32, f32)) -> Element {
    let label = diff.lines[row].text.trim_end_matches(['\r', '\n']).to_owned();
    diff.palette
        .bar_item_surface(Element::clickable(
            format!("{}:expand:{row}", diff.id),
            Element::row(vec![
                Element::spacer().size(Some(indent), Some(1.0)),
                Element::truncated_text(label.clone())
                    .font_size(diff.font_size)
                    .foreground(diff.palette.muted)
                    .shrink_to_fit(),
            ])
            .clip_children(),
        ))
        .accessible(Access::new(Role::Button).name(label))
        .size(Some(size.0), Some(size.1))
}

/// The bar above a file's diff: a chevron, the file's path with its name in the text colour
/// and its folder muted, and the count of added and removed lines. The host owns whether the
/// diff is collapsed; activating the bar reports `Clicked { id }`.
pub struct DiffFileHeader {
    id: String,
    path: String,
    palette: ComponentPalette,
    stat: Option<(usize, usize)>,
    collapsed: bool,
    colors: DiffColors,
}

impl DiffFileHeader {
    /// An expanded header for `path`, without counts, reporting `id` when activated.
    pub fn new(id: impl Into<String>, path: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            palette,
            stat: None,
            collapsed: false,
            colors: DiffColors::default(),
        }
    }
    /// Show `+added` in `add_bar` and `−removed` in `del_bar` at the end of the bar.
    pub fn stat(mut self, added: usize, removed: usize) -> Self {
        self.stat = Some((added, removed));
        self
    }
    /// Point the chevron right and tell the platform the diff is collapsed. False by default.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
    /// Take the colours of the counts from these, as the diff below does.
    pub fn colors(mut self, colors: DiffColors) -> Self {
        self.colors = colors;
        self
    }
}

impl Component for DiffFileHeader {
    fn render(self) -> Element {
        let split = self.path.rfind(['/', '\\']).map_or(0, |at| at + 1);
        let (folder, name) = self.path.split_at(split);
        let text = |value: &str, ink: Color| {
            Element::truncated_text(value)
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .foreground(ink)
        };
        let mut row = vec![
            Element::icon(if self.collapsed { Icon::ChevronRight } else { Icon::ChevronDown })
                .foreground(self.palette.muted)
                .size(Some(14.0), Some(14.0)),
            // The folder gives way first, so the name stays readable.
            text(folder, self.palette.muted).shrink_to_fit(),
            text(name, self.palette.text),
            Element::spacer().flex(1.0),
        ];
        if let Some((added, removed)) = self.stat {
            row.push(text(&format!("+{added}"), self.colors.add_bar));
            row.push(text(&format!("−{removed}"), self.colors.del_bar));
        }
        let said = Access::new(Role::Button).name(self.path.clone()).expanded(!self.collapsed);
        self.palette
            .bar_item_surface(Element::clickable(
                &self.id,
                Element::row(row).gap(6.0).clip_children(),
            ))
            .accessible(said)
            .padding(8.0)
            .size(None, Some(32.0))
            .background(self.palette.raised)
    }
}

impl Component for CodeDiff {
    fn render(self) -> Element {
        Element::code_diff(self.clone())
            .size(None, Some(self.height))
            .font_size(self.font_size)
            .background(self.palette.surface)
            .foreground(self.palette.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn line(kind: CodeDiffLineKind, number: usize, text: &str) -> CodeDiffLine {
        CodeDiffLine {
            kind,
            old_line: (kind != CodeDiffLineKind::Added).then_some(number),
            new_line: (kind != CodeDiffLineKind::Removed).then_some(number),
            text: text.into(),
        }
    }

    /// A tree measuring 8 px a character, so a column is 8 px wide.
    fn laid(root: Element, size: Size) -> (UiTree, crate::Frame) {
        let mut tree = UiTree::new(root);
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(size);
        (tree, frame)
    }

    fn fills(frame: &crate::Frame, color: Color) -> Vec<Rect> {
        frame
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn texts(frame: &crate::Frame) -> Vec<(String, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn changed_words_take_the_stronger_tint_over_their_columns_on_changed_rows_only() {
        let colors = DiffColors::default();
        let lines: Arc<[CodeDiffLine]> = vec![
            line(CodeDiffLineKind::Context, 1, "keep this"),
            line(CodeDiffLineKind::Removed, 2, "let a = 1;"),
            line(CodeDiffLineKind::Added, 2, "let b = 2;"),
        ]
        .into();
        let words: Arc<[Vec<Range<usize>>]> = vec![vec![0..4], vec![4..5, 8..9], vec![4..5, Range { start: 9, end: 4 }]].into();
        let (_, frame) = laid(
            CodeDiff::new("diff", lines).word_ranges(words).height(120.0).render(),
            Size::new(400.0, 120.0),
        );
        let removed = fills(&frame, colors.del_word);
        let added = fills(&frame, colors.add_word);
        // Two removed words, one column each; the reversed added range paints nothing and
        // the context row's range is ignored.
        assert_eq!(removed.len(), 2);
        assert_eq!(added.len(), 1);
        assert!(removed.iter().chain(&added).all(|rect| rect.width == 8.0));
        assert_eq!(removed[1].x - removed[0].x, 32.0);
        assert_eq!(added[0].x, removed[0].x);
        assert!(added[0].y > removed[0].y);
        assert_eq!(fills(&frame, colors.del_row).len(), 1);
        assert_eq!(fills(&frame, colors.add_row).len(), 1);
    }

    #[test]
    fn colors_are_configurable_and_the_margin_bar_can_be_turned_off() {
        let colors = DiffColors {
            add_row: Color::rgba(1, 2, 3, 30),
            add_bar: Color::rgb(4, 5, 6),
            del_row: Color::rgba(7, 8, 9, 30),
            del_bar: Color::rgb(10, 11, 12),
            ..DiffColors::default()
        };
        let lines: Arc<[CodeDiffLine]> = vec![
            line(CodeDiffLineKind::Removed, 1, "old"),
            line(CodeDiffLineKind::Added, 1, "new"),
        ]
        .into();
        for bar in [true, false] {
            let (_, frame) = laid(
                CodeDiff::new("diff", lines.clone()).colors(colors).margin_bar(bar).render(),
                Size::new(400.0, 120.0),
            );
            for (row, bar_color) in [(colors.add_row, colors.add_bar), (colors.del_row, colors.del_bar)] {
                assert_eq!(fills(&frame, row).len(), 1);
                let bars = fills(&frame, bar_color);
                assert_eq!(bars.len(), usize::from(bar));
                assert!(bars.iter().all(|rect| rect.width == 2.0));
                // The marker keeps the bar's colour either way.
                assert!(texts(&frame).iter().any(|(_, color)| *color == bar_color));
            }
        }
    }

    #[test]
    fn a_folded_row_shows_its_label_and_reports_its_expand_id() {
        let lines: Arc<[CodeDiffLine]> = vec![
            line(CodeDiffLineKind::Context, 1, "fn a() {}"),
            CodeDiffLine {
                kind: CodeDiffLineKind::Folded,
                old_line: Some(99),
                new_line: Some(99),
                text: "Show 12 more lines".into(),
            },
            line(CodeDiffLineKind::Context, 14, "fn b() {}"),
        ]
        .into();
        let root = Element::column(vec![
            CodeDiff::new("diff", lines).height(60.0).render(),
            Element::spacer().flex(1.0),
        ]);
        let (mut tree, frame) = laid(root, Size::new(400.0, 200.0));
        let shown = texts(&frame);
        assert!(shown.iter().any(|(text, _)| text == "Show 12 more lines"));
        assert!(!shown.iter().any(|(text, _)| text == "99"), "a folded row has no numbers");
        let control = frame
            .regions
            .iter()
            .find(|region| region.id == "diff:expand:1")
            .expect("the folded row is a control");
        assert_eq!(control.kind, HitKind::Button);
        let node = frame.access.iter().find(|node| node.id == "diff:expand:1").unwrap();
        assert_eq!((node.role, node.name.as_str()), (Role::Button, "Show 12 more lines"));
        let point = Point::new(control.rect.x + 40.0, control.rect.y + control.rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "diff:expand:1".into() }]
        );
        // The wheel over the control still scrolls the diff under it.
        tree.handle_event(UiEvent::ScrollDirect { position: point, delta_y: -40.0 });
        let after = tree.layout(Size::new(400.0, 200.0));
        let moved = after.regions.iter().find(|region| region.id == "diff:expand:1");
        assert!(moved.is_none_or(|region| region.rect.y < control.rect.y));
    }

    #[test]
    fn the_file_header_reports_its_id_and_shows_the_name_and_counts() {
        let palette = ComponentPalette::neutral_dark();
        let colors = DiffColors::default();
        for collapsed in [false, true] {
            let root = Element::column(vec![
                DiffFileHeader::new("file", "src/app/main.rs", palette)
                    .stat(1, 1)
                    .collapsed(collapsed)
                    .render(),
                Element::spacer().flex(1.0),
            ]);
            let (mut tree, frame) = laid(root, Size::new(400.0, 200.0));
            let shown = texts(&frame);
            for expected in [
                ("src/app/".to_owned(), palette.muted),
                ("main.rs".to_owned(), palette.text),
                ("+1".to_owned(), colors.add_bar),
                ("−1".to_owned(), colors.del_bar),
            ] {
                assert!(shown.contains(&expected), "{expected:?} in {shown:?}");
            }
            let chevron = frame.commands.iter().find_map(|cmd| match cmd {
                DrawCommand::Icon { icon, .. } => Some(*icon),
                _ => None,
            });
            assert_eq!(chevron, Some(if collapsed { Icon::ChevronRight } else { Icon::ChevronDown }));
            let node = frame.access.iter().find(|node| node.id == "file").unwrap();
            assert_eq!((node.role, node.name.as_str()), (Role::Button, "src/app/main.rs"));
            assert_eq!(node.expanded, Some(!collapsed));
            let header = frame.regions.iter().find(|region| region.id == "file").unwrap().rect;
            let point = Point::new(header.x + 10.0, header.y + header.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "file".into() }]
            );
        }
        // Without a folder or counts there is only the name.
        let (_, frame) = laid(
            Element::column(vec![DiffFileHeader::new("file", "README", palette).render()]),
            Size::new(400.0, 200.0),
        );
        let shown = texts(&frame);
        assert!(shown.iter().any(|(text, _)| text == "README"));
        assert!(!shown.iter().any(|(text, _)| text.starts_with('+')));
    }

    #[test]
    fn standalone_diff_focus_is_opt_in_and_preserves_read_only_rows_and_scrolling() {
        let lines: Arc<[CodeDiffLine]> = (0..60)
            .map(|row| CodeDiffLine {
                kind: CodeDiffLineKind::Removed,
                old_line: Some(row + 1),
                new_line: None,
                text: format!("recorded row {row}"),
            })
            .collect::<Vec<_>>()
            .into();
        for focusable in [false, true] {
            let root = |enabled| {
                Element::column(vec![
                    Element::button("before", "Before"),
                    CodeDiff::new("diff", lines.clone())
                        .focusable(enabled)
                        .height(120.0)
                        .render(),
                    Element::button("after", "After"),
                ])
            };
            let mut tree = UiTree::new(root(focusable));
            let frame = tree.layout(Size::new(400.0, 240.0));
            let content = frame
                .regions
                .iter()
                .find(|hit| hit.id == "diff" && hit.kind == HitKind::CodeDiff)
                .unwrap()
                .rect;
            assert!(tree.focus("before"));
            tree.handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
            });
            assert_eq!(
                tree.focused_id(),
                Some(if focusable { "diff" } else { "after" })
            );
            assert_eq!(tree.focus("diff"), focusable);
            let point = Point::new(content.x + 80.0, content.y + 18.0);
            tree.handle_event(UiEvent::PointerDown(point));
            tree.handle_event(UiEvent::PointerUp(point));
            assert_eq!(tree.focused_id(), focusable.then_some("diff"));
            for event in [
                UiEvent::Text("overwrite".into()),
                UiEvent::Paste("replace".into()),
                UiEvent::Key {
                    key: Key::Backspace,
                    shift: false,
                    ctrl: false,
                },
            ] {
                assert!(
                    tree.handle_event(event).is_empty(),
                    "read-only diff never mutates content"
                );
            }
            let visible_text = |frame: crate::Frame| {
                frame
                    .commands
                    .into_iter()
                    .filter_map(|cmd| {
                        if let DrawCommand::Text { text, .. } = cmd {
                            Some(text)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            };
            let before = visible_text(tree.layout(Size::new(400.0, 240.0)));
            tree.handle_event(UiEvent::ScrollDirect {
                position: point,
                delta_y: -160.0,
            });
            let after = visible_text(tree.layout(Size::new(400.0, 240.0)));
            assert_ne!(
                before, after,
                "existing diff wheel scroll remains functional"
            );
            assert_eq!(tree.focused_id(), focusable.then_some("diff"));
            assert_eq!(lines[0].text, "recorded row 0");
            tree.set_root(root(false));
            tree.layout(Size::new(400.0, 240.0));
            assert_ne!(
                tree.focused_id(),
                Some("diff"),
                "removing opt-in removes focus ownership"
            );
        }
    }
}
