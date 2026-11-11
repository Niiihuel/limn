//! How the tree places the controls it draws itself: clickables, buttons,
//! text inputs, text areas and mark rails.

use super::*;

#[cfg(test)]
#[path = "clickable_ink_tests.rs"]
mod clickable_ink_tests;

/// Paint in `now` what `element` holds in `rest`. What floats away from it,
/// a popover's surface or a hint, and the rows of a list keep their ink.
fn take_ink(element: &mut Element, rest: Color, now: Color) {
    if element.style.foreground == rest {
        element.style.foreground = now;
    }
    match &mut element.kind {
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::Stack(children)
        | ElementKind::TabStrip { children, .. } => children.iter_mut().for_each(|child| take_ink(child, rest, now)),
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. }
        | ElementKind::ShortcutHint { child, .. }
        | ElementKind::Popover { anchor: child, .. }
        | ElementKind::Tooltip { anchor: child, .. } => take_ink(child, rest, now),
        ElementKind::Overlay { base, floating } => {
            take_ink(base, rest, now);
            take_ink(floating, rest, now);
        }
        _ => {}
    }
}

impl LayoutBuilder<'_> {
    pub(super) fn place_clickable(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style) {
        let ElementKind::Clickable { id, child } = &element.kind else { unreachable!() };
        self.add_region(id, rect, clickable_kind(element), clip);
        if is_pointer_only(element) {
            self.frame.pointer_only.insert(id.clone());
        }
        if let Some(style) = element.kept.as_ref().and_then(|kept| kept.sketch) {
            self.frame.sketches.insert(id.clone(), (content, style));
        }
        if let Some(travel) = element.slider.as_deref() {
            self.frame.sliders.insert(
                id.clone(),
                SliderGeometry {
                    rect: content,
                    steps: travel.steps,
                    fraction: travel.fraction,
                    ultra_available: false,
                    inset: travel.inset,
                    animated: false,
                    y: travel.y,
                    vertical: travel.vertical,
                },
            );
        }
        let kept = element.kept.as_deref();
        let ringed = self.keyboard_focus && self.focused == Some(id.as_str());
        let under = kept.is_some_and(|kept| kept.ring_under);
        if ringed && under {
            self.draw_focus_ring(rect, style, element);
        }
        let first = self.frame.commands.len();
        let first_block = self.frame.text_blocks.len();
        let rest = element.style.foreground;
        if kept.is_some_and(|kept| kept.content_ink) && style.foreground != rest {
            // Placed again in the ink it has now, as `animate_foreground` is.
            let mut inked = (**child).clone();
            take_ink(&mut inked, rest, style.foreground);
            let previous_scope = self.enter_measurement_scope();
            self.place(&inked, content, clip);
            self.measurement_scope = previous_scope;
        } else {
            self.place(child, content, clip);
        }
        if let Some(draft) = self.sketch_draft.filter(|draft| draft.id == *id) {
            self.frame.commands.push(DrawCommand::PushClip(content));
            self.frame.commands.extend(crate::sketch::mark_commands(&draft.mark(), content));
            self.frame.commands.push(DrawCommand::PopClip);
        }
        if style.underline_on_hover
            && (self.hovered == Some(id.as_str())
                || self.keyboard_focus && self.focused == Some(id.as_str()))
        {
            self.underline_text(first, first_block, content, style);
        }
        if ringed && !under {
            self.draw_focus_ring(rect, style, element);
        }
    }

    pub(super) fn place_button(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let ElementKind::Button {
            id,
            label,
            trailing_icon,
        } = &element.kind else { unreachable!() };
        self.add_region(id, rect, HitKind::Button, clip);
        if self.keyboard_focus && self.focused == Some(id.as_str()) {
            self.draw_focus_ring(rect, style, element);
        }
        let trailing_side = if trailing_icon.is_some() {
            ICON_SIZE.min(content.width).min(content.height).max(0.0)
        } else {
            0.0
        };
        let label_width = (content.width
            - if trailing_icon.is_some() {
                trailing_side + style.gap
            } else {
                0.0
            })
        .max(0.0);
        let shown =
            truncate_measured(label, label_width, |part| self.text_width(part, style));
        let shown_width = self.text_width(&shown, style);
        let text_x = if style.center_content {
            content.x
                + ((content.width
                    - shown_width
                    - if trailing_icon.is_some() {
                        trailing_side + style.gap
                    } else {
                        0.0
                    })
                    / 2.0)
                    .max(0.0)
        } else {
            content.x
        };
        let text_y = content.y + (content.height - metrics.line_height) / 2.0;
        self.frame.commands.push(DrawCommand::Text {
            position: Point::new(text_x, text_y),
            text: shown,
            color: style.foreground,
            font_size: style.font_size,
            font_family: style.font_family,
        });
        if style.underline_on_hover
            && (self.hovered == Some(id.as_str())
                || self.keyboard_focus && self.focused == Some(id.as_str()))
        {
            self.frame.commands.push(DrawCommand::FillRect {
                rect: Rect::new(
                    text_x,
                    text_y + metrics.line_height - 2.0,
                    shown_width,
                    1.0,
                ),
                color: style.foreground,
                radius: 0.0,
            });
        }
        if let Some(icon) = trailing_icon {
            if trailing_side > 0.0 {
                self.draw_icon(
                    *icon,
                    Rect::new(
                        if style.center_content {
                            text_x + shown_width + style.gap
                        } else {
                            content.x + content.width - trailing_side
                        },
                        content.y + (content.height - trailing_side) / 2.0,
                        trailing_side,
                        trailing_side,
                    ),
                    style,
                );
            }
        }
    }

    pub(super) fn place_icon_button(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let ElementKind::IconButton {
            id,
            icon,
            label,
            trailing_icon,
        } = &element.kind else { unreachable!() };
        self.add_region(id, rect, HitKind::Button, clip);
        if self.keyboard_focus && self.focused == Some(id.as_str()) {
            self.draw_focus_ring(rect, style, element);
        }
        let side = style
            .icon_side
            .unwrap_or(ICON_SIZE)
            .min(content.width)
            .min(content.height)
            .max(0.0);
        let trailing_side = if trailing_icon.is_some() {
            ICON_SIZE
                .min((content.width - side - style.gap).max(0.0))
                .min(content.height)
        } else {
            0.0
        };
        let shown = if label.is_empty() {
            String::new()
        } else {
            truncate_measured(
                label,
                (content.width
                    - side
                    - style.gap
                    - if trailing_icon.is_some() {
                        trailing_side + style.gap
                    } else {
                        0.0
                    })
                .max(0.0),
                |part| self.text_width(part, style),
            )
        };
        let shown_width = self.text_width(&shown, style);
        let group_width =
            side + if label.is_empty() {
                0.0
            } else {
                style.gap + shown_width
            } + if trailing_icon.is_some() {
                style.gap + trailing_side
            } else {
                0.0
            };
        let start_x = if style.center_content || label.is_empty() && trailing_icon.is_none()
        {
            content.x + ((content.width - group_width) / 2.0).max(0.0)
        } else {
            content.x
        };
        let icon_rect = Rect::new(
            start_x,
            content.y + (content.height - side) / 2.0,
            side,
            side,
        );
        match element.kept.as_ref().and_then(|kept| kept.leading_image.as_ref()) {
            Some(InlineImage::Raster(image)) => self.frame.commands.push(DrawCommand::RasterImage {
                rect: icon_rect,
                image: image.clone(),
                opacity: 1.0,
                radius: 0.0,
            }),
            Some(InlineImage::Surface(surface)) => self.frame.commands.push(DrawCommand::Surface {
                rect: icon_rect,
                id: surface.clone(),
                label: label.clone(),
                opacity: 1.0,
                fit: crate::SurfaceFit::Contain,
                radius: 0.0,
                mirrored: false,
            }),
            None => self.draw_icon(*icon, icon_rect, style),
        }
        if !label.is_empty() {
            let text_x = start_x + side + style.gap;
            let text_y = content.y + (content.height - metrics.line_height) / 2.0;
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(text_x, text_y),
                text: shown,
                color: style.foreground,
                font_size: style.font_size,
                font_family: style.font_family,
            });
            if style.underline_on_hover
                && (self.hovered == Some(id.as_str())
                    || self.keyboard_focus && self.focused == Some(id.as_str()))
            {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: Rect::new(
                        text_x,
                        text_y + metrics.line_height - 2.0,
                        shown_width,
                        1.0,
                    ),
                    color: style.foreground,
                    radius: 0.0,
                });
            }
        }
        if let Some(trailing_icon) = trailing_icon {
            if trailing_side > 0.0 {
                self.draw_icon(
                    *trailing_icon,
                    Rect::new(
                        if style.center_content {
                            start_x + group_width - trailing_side
                        } else {
                            content.x + content.width - trailing_side
                        },
                        content.y + (content.height - trailing_side) / 2.0,
                        trailing_side,
                        trailing_side,
                    ),
                    style,
                );
            }
        }
    }

    pub(super) fn place_text_input(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let ElementKind::TextInput {
            id,
            initial_value,
            placeholder,
            persistent_placeholder,
            leading_icon,
            leading_icon_tint,
            shortcut,
            shortcut_fill,
            password,
        } = &element.kind else { unreachable!() };
        self.add_region(id, rect, HitKind::TextInput, clip);
        let floating = element.kept.as_ref().and_then(|kept| kept.floating_label.as_ref());
        let placeholder: &str = if floating.is_some() && self.focused != Some(id.as_str()) { "" } else { placeholder };
        if let Some((label, backdrop)) = floating {
            let focused = self.focused == Some(id.as_str());
            let filled = self.inputs.get(id).map_or(!initial_value.is_empty(), |input| !input.value.is_empty());
            let risen = self.spring(&format!("float/{id}"), if focused || filled { 1.0 } else { 0.0 });
            let inset = if leading_icon.is_some() { ICON_SIZE + 8.0 } else { 0.0 };
            self.paint_floating_label(rect, content, inset, label, *backdrop, style, risen, focused);
        }
        // Room a component keeps at the end for a control over it.
        let trailing = element
            .kept
            .as_ref()
            .map_or(0.0, |kept| kept.input_trailing_space);
        let content = Rect::new(
            content.x,
            content.y,
            (content.width - trailing).max(0.0),
            content.height,
        );
        let is_focused = self.focused == Some(id.as_str());
        let show_shortcut = !is_focused
            && initial_value.is_empty()
            && self
                .inputs
                .get(id)
                .map(|input| input.value.is_empty())
                .unwrap_or(true);
        let visible_shortcut = shortcut
            .as_deref()
            .map(shortcut::label)
            .filter(|value| show_shortcut && !value.is_empty());
        let icon_width = if leading_icon.is_some() {
            ICON_SIZE + 8.0
        } else {
            0.0
        };
        let text_reserve = self.text_width(placeholder, style).min(24.0);
        let hint_width = visible_shortcut
            .map(|value| self.shortcut_width(value))
            .filter(|width| {
                *width > 0.0 && *width + 8.0 + icon_width + text_reserve <= content.width
            })
            .unwrap_or(0.0);
        if let Some(shortcut) = visible_shortcut.filter(|_| hint_width > 0.0) {
            let hint = Rect::new(
                content.x + content.width - hint_width,
                content.y + (content.height - shortcut::HEIGHT) / 2.0,
                hint_width,
                shortcut::HEIGHT,
            );
            self.paint_shortcut(
                hint,
                shortcut,
                shortcut_fill.unwrap_or_else(|| shortcut::cap_fill(style.foreground)),
                style.foreground,
            );
        }
        let content = Rect::new(
            content.x,
            content.y,
            (content.width - hint_width - if hint_width > 0.0 { 8.0 } else { 0.0 })
                .max(0.0),
            content.height,
        );
        let content = if let Some(icon) = leading_icon {
            let side = ICON_SIZE.min(content.width).min(content.height).max(0.0);
            self.draw_icon(
                *icon,
                Rect::new(
                    content.x,
                    content.y + (content.height - side) / 2.0,
                    side,
                    side,
                ),
                Style {
                    foreground: if self.focused == Some(id.as_str()) {
                        style.foreground
                    } else {
                        leading_icon_tint.unwrap_or(style.foreground)
                    },
                    ..style
                },
            );
            let inset = (ICON_SIZE + 8.0).min(content.width);
            Rect::new(
                content.x + inset,
                content.y,
                content.width - inset,
                content.height,
            )
        } else {
            content
        };
        // The line sits in the middle of the field: of a taller one, and of a compact
        // one whose padding leaves less than a line, which it then takes from the
        // padding rather than be cut at the top and bottom.
        let slack = content.height - metrics.line_height;
        let content = Rect::new(
            content.x,
            content.y + slack / 2.0,
            content.width,
            content.height - slack,
        );
        let state = self
            .inputs
            .entry(id.clone())
            .or_insert_with(|| InputState::new(initial_value.clone()));
        state.observe_paste_kind(HitKind::TextInput);
        if state.password != *password {
            state.paste_epoch = Arc::new(());
        }
        state.password = *password;
        state.char_width = metrics.char_width;
        state.content_width = content.width;
        state.font_size = style.font_size;
        state.font_family = style.font_family;
        state.measurer = self.text_measurer.clone();
        if self.text_layout {
            self.paint_laid_out_input(id, content, style, clip, placeholder, *persistent_placeholder);
        } else {
            state.laid_out = None;
            state.ime = None;
            let center_text = style.center_content && state.width(&state.value) <= content.width - 2.0;
            if center_text {
                state.view_start = 0;
            }
            if state.cursor < state.view_start {
                state.view_start = state.cursor;
            }
            if state.width(char_range(&state.value, state.view_start, state.cursor))
                > content.width - 2.0
            {
                let mut lo = state.view_start;
                let mut hi = state.cursor;
                while lo < hi {
                    let mid = lo + (hi - lo) / 2;
                    if state.width(char_range(&state.value, mid, state.cursor))
                        <= content.width - 2.0
                    {
                        hi = mid;
                    } else {
                        lo = mid + 1;
                    }
                }
                state.view_start = lo;
            }
            let remaining =
                char_range(&state.value, state.view_start, state.value.chars().count());
            let visible_chars =
                fitting_chars(remaining, content.width, |part| state.width(part));
            let visible_end = state.view_start + visible_chars;
            let is_placeholder =
                state.value.is_empty() && (!is_focused || *persistent_placeholder);
            let shown = if is_placeholder {
                placeholder.to_owned()
            } else {
                if *password {
                    "•".repeat(visible_end.saturating_sub(state.view_start))
                } else {
                    char_range(&state.value, state.view_start, visible_end).to_owned()
                }
            };
            let text_x = if center_text {
                let text_width = measured_width(
                    state.measurer.as_deref(), &shown, style.font_size, style.font_family, metrics.char_width,
                );
                content.x + ((content.width - text_width) / 2.0).max(0.0)
            } else {
                content.x
            };
            state.content_x = text_x;
            let input_clip = clip
                .and_then(|outer| outer.intersection(content))
                .or_else(|| clip.is_none().then_some(content));
            if let Some(input_clip) = input_clip {
                self.frame.commands.push(DrawCommand::PushClip(input_clip));
                if let Some((selection_start, selection_end)) = state.selected_range() {
                    let start = selection_start.clamp(state.view_start, visible_end);
                    let end = selection_end.clamp(state.view_start, visible_end);
                    if end > start {
                        let left =
                            state.width(char_range(&state.value, state.view_start, start));
                        let right =
                            state.width(char_range(&state.value, state.view_start, end));
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(
                                text_x + left,
                                content.y,
                                right - left,
                                metrics.line_height,
                            ),
                            color: Color::rgba(75, 130, 210, 130),
                            radius: 0.0,
                        });
                    }
                }
                self.frame.commands.push(DrawCommand::Text {
                    position: Point::new(text_x, content.y),
                    text: shown,
                    color: if is_placeholder {
                        Color::rgba(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            110,
                        )
                    } else {
                        style.foreground
                    },
                    font_size: style.font_size,
                    font_family: style.font_family,
                });
                if is_focused && state.selected_range().is_none() {
                    let cursor_x =
                        state.width(char_range(&state.value, state.view_start, state.cursor));
                    let caret_rect =
                        Rect::new(text_x + cursor_x, content.y, 1.5, metrics.line_height);
                    if caret_rect.intersection(input_clip).is_some()
                        && self.caret_blink.observe(
                            id,
                            crate::caret_blink::CaretLocation {
                                document: 0,
                                revision: state.value.len() as u64,
                                offset: state.cursor,
                            },
                            self.now,
                            self.reduced_motion,
                        )
                    {
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: caret_rect,
                            color: style.foreground,
                            radius: 0.0,
                        });
                    }
                }
                self.frame.commands.push(DrawCommand::PopClip);
            }
        }
    }

    pub(super) fn place_text_area(&mut self, element: &Element, rect: Rect, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let ElementKind::TextArea {
            id,
            initial_value,
            placeholder,
            persistent_placeholder,
            preserve_line_endings,
            enter_submits,
            highlights,
            ..
        } = &element.kind else { unreachable!() };
        self.add_region(id, rect, HitKind::TextArea, clip);
        if !*preserve_line_endings
            && self
                .inputs
                .get(id)
                .map_or(initial_value.as_str(), |input| input.value.as_str())
                .contains('\r')
        {
            // Normalizing an existing CR draft changes the input
            // observed by auto-grow measurements later in this pass.
            self.measurement_cache.borrow_mut().clear();
        }
        let state = self.inputs.entry(id.clone()).or_insert_with(|| {
            InputState::new(if *preserve_line_endings {
                initial_value.clone()
            } else {
                initial_value.replace("\r\n", "\n").replace('\r', "\n")
            })
        });
        state.observe_paste_kind(HitKind::TextArea);
        if state.preserve_line_endings != *preserve_line_endings
            || state.enter_submits != *enter_submits
        {
            state.paste_epoch = Arc::new(());
        }
        state.preserve_line_endings = *preserve_line_endings;
        state.enter_submits = *enter_submits;
        if !state.preserve_line_endings && state.value.contains('\r') {
            state.value = state.value.replace("\r\n", "\n").replace('\r', "\n");
            state.paste_epoch = Arc::new(());
            state.set_selection(state.value.chars().count(), None);
        }
        state.content_x = content.x;
        state.content_y = content.y;
        state.char_width = metrics.char_width;
        state.line_height = metrics.line_height;
        state.content_width = content.width;
        state.font_size = style.font_size;
        state.font_family = style.font_family;
        state.measurer = self.text_measurer.clone();
        state.visible_lines = (content.height / metrics.line_height + 0.0001)
            .floor()
            .max(1.0) as usize;
        if self.text_layout {
            self.paint_laid_out_area(
                id,
                rect,
                content,
                style,
                clip,
                placeholder,
                *persistent_placeholder,
                initial_value,
                highlights,
            );
        } else {
            state.laid_out = None;
            state.ime = None;
            let lines = state.visual_lines();
            let (cursor_row, cursor_column) = visual_cursor(&lines, state.cursor);
            let max_scroll = lines.len().saturating_sub(state.visible_lines);
            state.scroll_line = state.scroll_line.min(max_scroll);
            if state.keep_cursor_visible {
                if cursor_row < state.scroll_line {
                    state.scroll_line = cursor_row;
                } else if cursor_row >= state.scroll_line + state.visible_lines {
                    state.scroll_line = cursor_row + 1 - state.visible_lines;
                }
                state.keep_cursor_visible = false;
            }

            let is_focused = self.focused == Some(id.as_str());
            let is_placeholder = state.value.is_empty() && (!is_focused || *persistent_placeholder);
            let placeholder_lines;
            let shown_lines = if is_placeholder {
                placeholder_lines =
                    visual_lines_measured(placeholder, content.width, |part| state.width(part));
                &placeholder_lines
            } else {
                &lines
            };
            let shown_start = if is_placeholder { 0 } else { state.scroll_line };
            let input_clip = clip
                .and_then(|outer| outer.intersection(content))
                .or_else(|| clip.is_none().then_some(content));
            if let Some(input_clip) = input_clip {
                self.frame.commands.push(DrawCommand::PushClip(input_clip));
                for (visible_row, line) in shown_lines
                    .iter()
                    .skip(shown_start)
                    .take(state.visible_lines)
                    .enumerate()
                {
                    let y = content.y + visible_row as f32 * metrics.line_height;
                    // Highlight fills go under the selection, painted next.
                    let fills_at = self.frame.commands.len();
                    if !is_placeholder {
                        if let Some((selection_start, selection_end)) = state.selected_range() {
                            let first = selection_start.max(line.start).min(line.end);
                            let last = selection_end.min(line.end).max(first);
                            if last > first {
                                let left =
                                    state.width(char_range(&line.text, 0, first - line.start));
                                let right =
                                    state.width(char_range(&line.text, 0, last - line.start));
                                self.frame.commands.push(DrawCommand::FillRect {
                                    rect: Rect::new(
                                        content.x + left,
                                        y,
                                        right - left,
                                        metrics.line_height,
                                    ),
                                    color: Color::rgba(75, 130, 210, 130),
                                    radius: 0.0,
                                });
                            }
                        }
                    }
                    let ink = if is_placeholder {
                        Color::rgba(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                            110,
                        )
                    } else {
                        style.foreground
                    };
                    // Highlights follow the host's text; a stale edit
                    // paints plain until the host catches up.
                    let spans: Vec<(f32, f32, &crate::TextHighlight)> =
                        if !is_placeholder && state.value == *initial_value {
                            highlights
                                .iter()
                                .filter_map(|highlight| {
                                    let first =
                                        highlight.range.start.max(line.start).min(line.end);
                                    let last = highlight.range.end.min(line.end).max(first);
                                    (first < last).then(|| {
                                        (
                                            state.width(char_range(
                                                &line.text,
                                                0,
                                                first - line.start,
                                            )),
                                            state.width(char_range(
                                                &line.text,
                                                0,
                                                last - line.start,
                                            )),
                                            highlight,
                                        )
                                    })
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };
                    let fills: Vec<DrawCommand> = spans
                        .iter()
                        .filter_map(|(left, right, highlight)| {
                            Some(DrawCommand::FillRect {
                                rect: Rect::new(
                                    content.x + left,
                                    y,
                                    right - left,
                                    metrics.line_height,
                                ),
                                color: highlight.background?,
                                radius: 3.0,
                            })
                        })
                        .collect();
                    self.frame.commands.splice(fills_at..fills_at, fills);
                    let plain = |commands: &mut Vec<DrawCommand>| {
                        commands.push(DrawCommand::Text {
                            position: Point::new(content.x, y),
                            text: line.text.clone(),
                            color: ink,
                            font_size: style.font_size,
                            font_family: style.font_family,
                        });
                    };
                    // Upright ink must not show under a slanted range, so
                    // the line is painted around those ranges.
                    let mut slanted: Vec<(f32, f32)> = spans
                        .iter()
                        .filter(|(_, _, highlight)| highlight.italic)
                        .map(|(left, right, _)| (*left, *right))
                        .collect();
                    if slanted.is_empty() {
                        plain(&mut self.frame.commands);
                    } else {
                        slanted.sort_by(|a, b| a.0.total_cmp(&b.0));
                        slanted.push((content.width.max(0.0), f32::INFINITY));
                        let mut from = 0.0_f32;
                        for (left, right) in slanted {
                            if left > from {
                                self.frame.commands.push(DrawCommand::PushClip(Rect::new(
                                    content.x + from,
                                    y,
                                    left - from,
                                    metrics.line_height,
                                )));
                                plain(&mut self.frame.commands);
                                self.frame.commands.push(DrawCommand::PopClip);
                            }
                            from = from.max(right);
                        }
                    }
                    // Repaint the exact line through a colored clip. This
                    // keeps shaping, wrapping and caret advances identical
                    // to an unstyled editable buffer, including ligatures.
                    for (left, right, highlight) in spans {
                        // A slanted last glyph leans past its advance.
                        let lean = if highlight.italic { 3.0 } else { 0.0 };
                        self.frame.commands.push(DrawCommand::PushClip(Rect::new(
                            content.x + left,
                            y,
                            right - left + lean,
                            metrics.line_height,
                        )));
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(content.x, y),
                            text: line.text.clone(),
                            color: highlight.color,
                            font_size: style.font_size,
                            font_family: if highlight.italic {
                                FontFamily::SansItalic
                            } else {
                                style.font_family
                            },
                        });
                        self.frame.commands.push(DrawCommand::PopClip);
                    }
                }
                if is_focused
                    && state.selected_range().is_none()
                    && cursor_row >= state.scroll_line
                {
                    let visible_row = cursor_row - state.scroll_line;
                    if visible_row < state.visible_lines {
                        let caret_rect = Rect::new(
                            content.x
                                + state.width(char_range(
                                    &lines[cursor_row].text,
                                    0,
                                    cursor_column,
                                )),
                            content.y + visible_row as f32 * metrics.line_height,
                            1.5,
                            metrics.line_height,
                        );
                        if caret_rect.intersection(input_clip).is_some()
                            && self.caret_blink.observe(
                                id,
                                crate::caret_blink::CaretLocation {
                                    document: 0,
                                    revision: state.value.len() as u64,
                                    offset: state.cursor,
                                },
                                self.now,
                                self.reduced_motion,
                            )
                        {
                            self.frame.commands.push(DrawCommand::FillRect {
                                rect: caret_rect,
                                color: style.foreground,
                                radius: 0.0,
                            });
                        }
                    }
                }
                self.frame.commands.push(DrawCommand::PopClip);
            }
            if state.scroll_line != state.painted_scroll_line {
                state.painted_scroll_line = state.scroll_line;
                state.scrolled_at = Some(self.now);
            }
            let scrolled = state
                .scrolled_at
                .is_some_and(|at| self.now.saturating_duration_since(at) < TEXT_AREA_BAR_HOLD);
            if max_scroll > 0
                && !is_placeholder
                && (scrolled || self.pointer.is_some_and(|point| rect.contains(point)))
            {
                // shortcut: the thumb shows the position but is not a
                // pointer target; give it the scroll area's drag when a
                // text area grows past a few dozen lines.
                let track = rect.inset(3.0);
                let height = (track.height * state.visible_lines as f32 / lines.len() as f32)
                    .max(16.0)
                    .min(track.height);
                let thumb = Rect::new(
                    track.x + track.width - 4.0,
                    track.y
                        + (track.height - height) * state.scroll_line as f32
                            / max_scroll as f32,
                    4.0,
                    height,
                );
                if clip.is_none_or(|clip| clip.intersection(thumb).is_some()) {
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: thumb,
                        color: crate::ScrollbarAppearance::default().thumb_color(
                            style.foreground,
                            false,
                            false,
                        ),
                        radius: 2.0,
                    });
                }
            }
        }
    }

    pub(super) fn place_mark_rail(&mut self, element: &Element, content: Rect, clip: Option<Rect>) {
        let ElementKind::MarkRail(rail) = &element.kind else { unreachable!() };
        let pitch = (content.height / rail.marks.len().max(1) as f32).min(rail.pitch);
        let at = |id: Option<&str>| {
            id.and_then(|id| rail.marks.iter().position(|mark| mark.id == id))
        };
        // As for a tooltip, the keyboard leads until the pointer moves again.
        let lead = at(self.focused.filter(|_| self.keyboard_tooltip_focus)).or(at(self.hovered));
        let thickness = rail.thickness.min(pitch);
        for (index, mark) in rail.marks.iter().enumerate() {
            let row = Rect::new(
                content.x,
                content.y + index as f32 * pitch,
                content.width,
                pitch,
            );
            self.add_region(&mark.id, row, HitKind::Button, clip);
            let rest = rail.length(index, None);
            let length =
                self.spring(&format!("{}/{}", rail.id, mark.id), rail.length(index, lead));
            // The ink follows the length, so only the raised mark is lit.
            let lit = ((length - rest) / (rail.peak - rest).max(1.0)).clamp(0.0, 1.0);
            let base = if rail.active == Some(index) { rail.active_ink } else { rail.ink };
            self.frame.commands.push(DrawCommand::FillRect {
                rect: Rect::new(row.x, row.y + (pitch - thickness) / 2.0, length, thickness),
                color: mix(base, rail.lead_ink, lit.powi(3)),
                radius: thickness / 2.0,
            });
        }
        if let Some(index) = lead {
            let mark = &rail.marks[index];
            // Muted under the title, which is what the turn is known by.
            let preview = (!mark.preview.is_empty())
                .then(|| TooltipSection::Footer(mark.preview.clone()));
            // A rail is read by moving along it: no wait before the hint.
            self.overlays.push_back(DeferredOverlay::Tooltip {
                anchor: Rect::new(
                    content.x,
                    content.y + index as f32 * pitch,
                    rail.peak,
                    pitch,
                ),
                title: mark.title.clone(),
                shortcut: None,
                sections: preview.into_iter().collect(),
                max_width: 320.0,
                placement: TooltipPlacement::Right,
                colors: rail.tooltip,
                options: crate::tokens().tooltip_options,
                content: None,
            });
        }
        self.frame.tab_focus.push(TabFocusScope {
            action_ids: rail.marks.iter().map(|mark| mark.id.clone()).collect(),
            vertical: true,
            every_arrow: false,
        });
    }
}
