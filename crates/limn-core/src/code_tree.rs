//! How the tree places the code editor and the code diff.

use super::*;

impl LayoutBuilder<'_> {
    pub(super) fn place_code_editor(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>) {
        let ElementKind::CodeEditor(editor) = &element.kind else { unreachable!() };
        self.add_region(&editor.id, rect, HitKind::CodeEditor, clip);
        let text_height = self.code_text_height(editor.font_size);
        let measurer = self.text_measurer.clone();
        let advance = self.metrics.at_size(editor.font_size).char_width;
        let measure = |text: &str| {
            measured_width(
                measurer.as_deref(),
                text,
                editor.font_size,
                FontFamily::Monospace,
                advance,
            )
        };
        if let Some(visible) = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content))
        {
            let state = self
                .code_editors
                .entry(editor.id.clone())
                .or_insert_with(|| {
                    crate::code_editor::EditorView::new((**editor).clone(), self.now)
                });
            state.update((**editor).clone());
            state.set_hovered_control(self.hovered, self.pointer);
            state.set_review_pointer(self.hovered, self.pointer, self.now);
            self.frame.commands.extend(state.paint(
                content,
                visible,
                self.focused == Some(editor.id.as_str()),
                &mut |document, revision, offset| {
                    self.caret_blink.observe(
                        &editor.id,
                        crate::caret_blink::CaretLocation {
                            document,
                            revision,
                            offset,
                        },
                        self.now,
                        self.reduced_motion,
                    )
                },
                &measure,
                text_height,
            ));
            let diagnostic = state.diagnostic_tooltip();
            let controls = state.structure_controls().to_vec();
            if let Some((anchor, message)) = diagnostic {
                self.overlays.push_back(DeferredOverlay::Tooltip {
                    anchor,
                    title: String::new(),
                    shortcut: None,
                    sections: vec![TooltipSection::Description(message)],
                    max_width: 420.0,
                    placement: TooltipPlacement::Above,
                    colors: editor.palette.tooltip_style(),
                    options: crate::tokens().tooltip_options,
                    content: None,
                });
            }
            for control in controls {
                self.place(&control.element, control.rect, Some(control.clip));
            }
        }
    }

    pub(super) fn place_code_diff(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>) {
        let ElementKind::CodeDiff(diff) = &element.kind else { unreachable!() };
        self.add_region(&diff.id, rect, HitKind::CodeDiff, clip);
        let text_height = self.code_text_height(diff.font_size);
        let measurer = self.text_measurer.clone();
        let advance = self.metrics.at_size(diff.font_size).char_width;
        let measure = |text: &str| {
            measured_width(
                measurer.as_deref(),
                text,
                diff.font_size,
                FontFamily::Monospace,
                advance,
            )
        };
        if let Some(visible) = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content))
        {
            let state = self.code_diffs.entry(diff.id.clone()).or_default();
            state.viewport.set_hovered_point(
                (self.hovered == Some(diff.id.as_str()))
                    .then_some(self.pointer)
                    .flatten(),
            );
            self.frame.commands.extend(crate::code_editor::paint_diff(
                diff,
                state,
                content,
                visible,
                &measure,
                text_height,
            ));
            let indent = state.viewport.body.x - content.x;
            let expand = std::mem::take(&mut state.expand);
            for &(row, at, row_clip) in &expand {
                let control = crate::code_diff::expand_control(diff, row, indent, (at.width, at.height));
                self.place(&control, at, Some(row_clip));
            }
            if let Some(state) = self.code_diffs.get_mut(&diff.id) {
                state.expand = expand;
            }
        }
    }
}
