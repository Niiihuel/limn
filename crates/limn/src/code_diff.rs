//! Original, virtualized review rows. The host supplies journal-backed content.
use crate::{CodeLanguage, Component, ComponentPalette, Element};
use std::sync::Arc;

/// Whether a row is unchanged, added or removed. It sets the row's marker and colours.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeDiffLineKind {
    Context,
    Added,
    Removed,
}

/// Text is retained exactly as supplied; the view never applies a patch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeDiffLine {
    pub kind: CodeDiffLineKind,
    /// The numbers shown in the old and the new line-number column, as given. `None` leaves
    /// that column empty on the row.
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
}

/// A read-only, scrollable list of diff rows with line numbers and change colours.
#[derive(Clone, Debug)]
pub struct CodeDiff {
    pub id: String,
    pub lines: Arc<[CodeDiffLine]>,
    pub palette: ComponentPalette,
    pub height: f32,
    pub tab_width: usize,
    pub language: CodeLanguage,
    pub single_gutter: bool,
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
            .font_size(13.0)
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
