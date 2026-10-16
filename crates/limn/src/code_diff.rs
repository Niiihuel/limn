//! Original, virtualized review rows. The host supplies journal-backed content.
use crate::{CodeLanguage, Component, ComponentPalette, Element};
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
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiEvent, UiTree};

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
                alt: false,
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
                    alt: false,
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
