//! How the tree places text: plain, truncated, faded, shimmering and
//! formatted text, and keyboard shortcuts.

use super::*;

impl LayoutBuilder<'_> {
    pub(super) fn place_faded_text(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let ElementKind::FadedText {
            text,
            fade_width,
            shimmer,
        } = &element.kind else { unreachable!() };
        let y = content.y + (content.height - metrics.line_height) / 2.0;
        let width = self.text_width(text, style);
        let x = if style.center_content && width <= content.width {
            content.x + (content.width - width) / 2.0
        } else {
            content.x
        };
        // An intrinsic label can be clipped by a bounded flex slot.
        // Fade at that visible right edge, without moving the shaped
        // line or treating clipped whitespace as part of its width.
        let visible_width = clip.map_or(content.width, |bounds| {
            content
                .width
                .min(bounds.x + bounds.width - content.x)
                .max(0.0)
        });
        let fade = if width > visible_width {
            fade_width.min(visible_width)
        } else {
            0.0
        };
        let solid = (visible_width - fade).max(0.0);
        // Where the highlight is, as `paint_shimmer_line` sweeps it over
        // what shows of the line.
        let sweep = shimmer
            .as_ref()
            .filter(|_| !self.reduced_motion && !text.is_empty())
            .map(|(id, highlight)| {
                let started = states_mut!(self).touch(id, self.now, || ShimmerStart(self.now)).0;
                let elapsed = self.now.saturating_duration_since(started).as_secs_f32();
                let reach = width.min(visible_width);
                (*highlight, (elapsed / 1.8).fract() * (reach + 100.0) - 50.0)
            });
        // Reuse the same shaped line and native clip primitives as
        // shimmer text. Alpha bands keep arbitrary hover/selection
        // backgrounds correct without painting an opaque mask; a
        // shimmer splits the solid part in bands of its own.
        let solid_bands = if sweep.is_some() { 24 } else { 1 };
        let fade_bands = if fade > 0.0 { 16 } else { 0 };
        for band in 0..solid_bands + fade_bands {
            let (left, right, opacity) = if band < solid_bands {
                let share = |at: usize| solid * at as f32 / solid_bands as f32;
                (content.x + share(band), content.x + share(band + 1), 1.0)
            } else {
                let start = (band - solid_bands) as f32 / fade_bands as f32;
                let end = (band - solid_bands + 1) as f32 / fade_bands as f32;
                (
                    content.x + solid + fade * start,
                    content.x + solid + fade * end,
                    1.0 - (start + end) / 2.0,
                )
            };
            let area = Rect::new(left, y, right - left, metrics.line_height);
            let visible = clip.map_or(Some(area), |outer| area.intersection(outer));
            if let Some(area) = visible.filter(|area| area.width > 0.0 && area.height > 0.0)
            {
                let mut ink = style.foreground;
                if let Some((highlight, center)) = sweep {
                    let distance = ((left + right) / 2.0 - x - center).abs();
                    let mix = (1.0 - distance / 38.0).max(0.0).powi(2);
                    let blend = |base: u8, bright: u8| {
                        (base as f32 + (bright as f32 - base as f32) * mix).round() as u8
                    };
                    ink = Color::rgba(
                        blend(ink.r, highlight.r),
                        blend(ink.g, highlight.g),
                        blend(ink.b, highlight.b),
                        ink.a,
                    );
                }
                self.frame.commands.push(DrawCommand::PushClip(area));
                self.frame.commands.push(DrawCommand::Text {
                    position: Point::new(x, y),
                    text: text.clone(),
                    color: Color::rgba(
                        ink.r,
                        ink.g,
                        ink.b,
                        (f32::from(ink.a) * opacity).round() as u8,
                    ),
                    font_size: style.font_size,
                    font_family: style.font_family,
                });
                self.frame.commands.push(DrawCommand::PopClip);
            }
        }
    }

    pub(super) fn place_truncated_text(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let (ElementKind::TruncatedText(value)
        | ElementKind::ShimmerText {
            text: value,
            truncated: true,
            ..
        }) = &element.kind else { unreachable!() };
        let shown =
            truncate_measured(value, content.width, |part| self.text_width(part, style));
        let x = if style.center_content {
            content.x + ((content.width - self.text_width(&shown, style)) / 2.0).max(0.0)
        } else {
            content.x
        };
        let y = content.y + (content.height - metrics.line_height) / 2.0;
        let shimmer = match &element.kind {
            ElementKind::ShimmerText { id, highlight, .. }
                if !self.reduced_motion && !shown.is_empty() =>
            {
                Some((id, highlight))
            }
            _ => None,
        };
        let ink = element
            .kept
            .as_ref()
            .and_then(|kept| kept.ink_gradient.clone());
        if !shown.is_empty() {
            self.paint_text_effects(&text_effects(element.kept.as_deref(), style.foreground), || {
                vec![DrawCommand::Text {
                    position: Point::new(x, y),
                    text: shown.clone(),
                    color: style.foreground,
                    font_size: style.font_size,
                    font_family: style.font_family,
                }]
            });
        }
        if let Some((id, highlight)) = shimmer {
            let started = states_mut!(self).touch(id, self.now, || ShimmerStart(self.now)).0;
            let width = self.text_width(&shown, style);
            self.paint_shimmer_line(
                &shown,
                Rect::new(x, y, width, metrics.line_height),
                style,
                (
                    *highlight,
                    self.now.saturating_duration_since(started).as_secs_f32(),
                    width,
                ),
                clip,
            );
        } else if let Some(stops) = ink.filter(|_| !shown.is_empty()) {
            let width = self.text_width(&shown, style);
            self.paint_gradient_line(
                &shown,
                Rect::new(x, y, width, metrics.line_height),
                style,
                &stops,
                clip,
            );
        } else {
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(x, y),
                text: shown,
                color: style.foreground,
                font_size: style.font_size,
                font_family: style.font_family,
            });
        }
    }

    pub(super) fn place_keyboard_shortcut(&mut self, element: &Element, content: Rect, style: Style) {
        let ElementKind::KeyboardShortcut {
            keys,
            fill,
            max_width,
            menu,
        } = &element.kind else { unreachable!() };
        let width = if *menu {
            self.menu_shortcut_width(keys)
        } else {
            self.shortcut_width(keys)
        };
        if max_width.is_none_or(|limit| width <= limit) {
            if *menu {
                self.paint_menu_shortcut(content, keys, style.foreground);
            } else {
                self.paint_shortcut(content, keys, *fill, style.foreground);
            }
        }
    }

    pub(super) fn place_text(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style, metrics: FontMetrics) {
        let (ElementKind::Text(value)
        | ElementKind::ShimmerText {
            text: value,
            truncated: false,
            ..
        }) = &element.kind else { unreachable!() };
        let shimmer = if let ElementKind::ShimmerText { id, highlight, .. } = &element.kind
        {
            if !self.reduced_motion && !value.is_empty() && content.width > 0.0 {
                let started = states_mut!(self).touch(id, self.now, || ShimmerStart(self.now)).0;
                Some((
                    *highlight,
                    self.now.saturating_duration_since(started).as_secs_f32(),
                ))
            } else {
                None
            }
        } else {
            None
        };
        let block_index = self.frame.text_blocks.len();
        self.frame.text_blocks.push(SelectableTextBlock {
            source: value.clone(),
            scope: self.selection_scope,
            runs: Vec::new(),
            region_layer: self.frame.regions.len(),
            layout: None,
        });
        let mut source_byte = 0;
        let mut source_char = 0;
        let effects = text_effects(element.kept.as_deref(), style.foreground);
        let lines = self.wrapped_lines(value, content.width, style);
        let sweep_width = if shimmer.is_some() {
            lines
                .iter()
                .map(|line| self.text_width(line, style))
                .fold(0.0, f32::max)
        } else {
            0.0
        };
        for (index, line) in lines.into_iter().enumerate() {
            if index > 0 && value[source_byte..].starts_with('\n') {
                source_byte += 1;
                source_char += 1;
            }
            debug_assert!(value[source_byte..].starts_with(line));
            let count = line.chars().count();
            let y = content.y + index as f32 * metrics.line_height;
            let line_width = self.text_width(line, style);
            let x = if style.center_content {
                content.x + ((content.width - line_width) / 2.0).max(0.0)
            } else {
                content.x
            };
            if !line.is_empty() {
                self.paint_text_effects(&effects, || {
                    vec![DrawCommand::Text {
                        position: Point::new(x, y),
                        text: line.to_owned(),
                        color: style.foreground,
                        font_size: style.font_size,
                        font_family: style.font_family,
                    }]
                });
            }
            self.paint_static_run(
                block_index,
                value,
                SelectableTextRun {
                    start: source_char,
                    end: source_char + count,
                    text: line.to_owned(),
                    rect: Rect::new(x, y, line_width, metrics.line_height),
                    visible: None,
                    font_size: style.font_size,
                    font_family: style.font_family,
                    atomic: false,
                },
                Point::new(x, y),
                style.foreground,
                clip,
            );
            if let Some((highlight, elapsed)) = shimmer {
                // Disjoint clips paint each shaped glyph once. Keep the
                // original selectable run and the native text measurer;
                // the sweep never changes wrapping or clipboard text.
                self.frame.commands.pop();
                self.paint_shimmer_line(
                    line,
                    Rect::new(x, y, line_width.max(0.0), metrics.line_height),
                    style,
                    (highlight, elapsed, sweep_width),
                    clip,
                );
            } else if let Some(stops) = element
                .kept
                .as_ref()
                .and_then(|kept| kept.ink_gradient.clone())
                .filter(|_| !line.is_empty())
            {
                // As the sweep: the same run, painted in bands.
                self.frame.commands.pop();
                self.paint_gradient_line(
                    line,
                    Rect::new(x, y, line_width.max(0.0), metrics.line_height),
                    style,
                    &stops,
                    clip,
                );
            }
            source_byte += line.len();
            source_char += count;
        }
    }

    pub(super) fn place_formatted_text(&mut self, element: &Element, content: Rect, clip: Option<Rect>, style: Style) {
        let ElementKind::FormattedText(spans) = &element.kind else { unreachable!() };
        let source = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<String>();
        let block_index = self.frame.text_blocks.len();
        self.frame.text_blocks.push(SelectableTextBlock {
            source: source.clone(),
            scope: self.selection_scope,
            runs: Vec::new(),
            region_layer: self.frame.regions.len(),
            layout: None,
        });
        let mut source_byte = 0;
        let mut source_char = 0;
        let mut y = content.y;
        // The hint of each run, by its target, and its parts' bounds.
        let mut hints: Vec<(String, Rect, TextHint)> = Vec::new();
        for (line_index, line) in self
            .formatted_lines(spans, content.width, style)
            .iter()
            .enumerate()
        {
            if line_index > 0 && source[source_byte..].starts_with('\n') {
                source_byte += 1;
                source_char += 1;
            }
            let natural_height = line
                .fragments
                .iter()
                .map(|fragment| fragment.line_height)
                .fold(0.0_f32, f32::max);
            let leading = (line.height - natural_height).max(0.0);
            let mut x = content.x;
            for fragment in &line.fragments {
                let fragment_rect = Rect::new(x, y, fragment.width, line.height);
                if let Some(id) = &fragment.action {
                    self.record_inline_focus_target(id, fragment_rect);
                    self.add_region(id, fragment_rect, HitKind::Button, clip);
                }
                if let Some(hint) = &fragment.tooltip {
                    // A run without an action is a passive target
                    // named after its text and its hint, so another
                    // text with the same hint is another target.
                    let id = fragment.action.clone().unwrap_or_else(|| {
                        let id = format!("{TEXT_HINT_PREFIX}{block_index}:{}", hint.title);
                        self.add_region(&id, fragment_rect, HitKind::Hover, clip);
                        id
                    });
                    hints.push((id, fragment_rect, hint.clone()));
                }
                if fragment.ellipsis {
                    self.frame.commands.push(DrawCommand::Text {
                        position: Point::new(
                            x,
                            y + line.height - fragment.line_height - leading / 2.0,
                        ),
                        text: fragment.text.clone(),
                        color: fragment.color,
                        font_size: fragment.font_size,
                        font_family: fragment.font_family,
                    });
                    x += fragment.width;
                    continue;
                }
                if let Some(image) = &fragment.image {
                    let side = fragment.width;
                    let picture = Rect::new(x, y + (line.height - side) / 2.0, side, side);
                    if let Some(visible) = clip
                        .and_then(|clip| clip.intersection(picture))
                        .or_else(|| clip.is_none().then_some(picture))
                    {
                        self.frame.commands.push(DrawCommand::PushClip(visible));
                        self.frame.commands.push(match image {
                            InlineImage::Raster(image) => DrawCommand::RasterImage {
                                rect: picture,
                                image: image.clone(),
                                opacity: 1.0,
                                radius: 0.0,
                            },
                            InlineImage::Surface(id) => DrawCommand::Surface {
                                rect: picture,
                                id: id.clone(),
                                label: fragment.text.clone(),
                                opacity: 1.0,
                                fit: crate::SurfaceFit::Contain,
                                radius: 0.0,
                                mirrored: false,
                            },
                        });
                        self.frame.commands.push(DrawCommand::PopClip);
                    }
                    // The text, unpainted, is what a selection takes.
                    let count = fragment.text.chars().count();
                    self.paint_static_run(
                        block_index,
                        &source,
                        SelectableTextRun {
                            start: source_char,
                            end: source_char + count,
                            text: fragment.text.clone(),
                            rect: fragment_rect,
                            visible: None,
                            font_size: fragment.font_size,
                            font_family: fragment.font_family,
                            atomic: true,
                        },
                        Point::new(x, y),
                        fragment.color,
                        clip,
                    );
                    if let Some(id) = &fragment.action {
                        if self.keyboard_focus && self.focused == Some(id.as_str()) {
                            self.draw_focus_ring(fragment_rect, style, element);
                        }
                    }
                    source_byte += fragment.text.len();
                    source_char += count;
                    x += fragment.width;
                    continue;
                }
                if let Some(icon) = fragment.icon {
                    if let Some(visible) = clip
                        .and_then(|clip| clip.intersection(fragment_rect))
                        .or_else(|| clip.is_none().then_some(fragment_rect))
                    {
                        self.frame.commands.push(DrawCommand::PushClip(visible));
                        self.frame.commands.push(DrawCommand::Icon {
                            rect: Rect::new(
                                x,
                                y + (line.height - ICON_SIZE) / 2.0,
                                ICON_SIZE,
                                ICON_SIZE,
                            ),
                            icon,
                            tint: fragment.color,
                            transform: crate::IconTransform::IDENTITY,
                        });
                        self.frame.commands.push(DrawCommand::PopClip);
                    }
                    x += fragment.width;
                    continue;
                }
                if let Some(background) = fragment.background {
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            x,
                            y + leading / 2.0,
                            fragment.width,
                            natural_height,
                        ),
                        color: background,
                        radius: 3.0,
                    });
                    x += 3.0;
                }
                debug_assert!(source[source_byte..].starts_with(&fragment.text));
                let count = fragment.text.chars().count();
                self.paint_static_run(
                    block_index,
                    &source,
                    SelectableTextRun {
                        start: source_char,
                        end: source_char + count,
                        text: fragment.text.clone(),
                        rect: Rect::new(
                            x,
                            y,
                            fragment.width
                                - if fragment.background.is_some() {
                                    6.0
                                } else {
                                    0.0
                                },
                            line.height,
                        ),
                        visible: None,
                        font_size: fragment.font_size,
                        font_family: fragment.font_family,
                        atomic: false,
                    },
                    Point::new(x, y + line.height - fragment.line_height - leading / 2.0),
                    fragment.color,
                    clip,
                );
                if fragment.underline || fragment.strikethrough {
                    let top = y + line.height - fragment.line_height - leading / 2.0;
                    let thickness = (fragment.font_size / 14.0).max(1.0);
                    let width = fragment.width
                        - if fragment.background.is_some() {
                            6.0
                        } else {
                            0.0
                        };
                    for (drawn, at) in [
                        (fragment.underline, TEXT_UNDERLINE_AT),
                        (fragment.strikethrough, TEXT_STRIKETHROUGH_AT),
                    ] {
                        if drawn {
                            self.frame.commands.push(DrawCommand::FillRect {
                                rect: Rect::new(
                                    x,
                                    top + fragment.line_height * at - thickness / 2.0,
                                    width,
                                    thickness,
                                ),
                                color: fragment.color,
                                radius: 0.0,
                            });
                        }
                    }
                }
                if let Some(id) = &fragment.action {
                    if self.keyboard_focus && self.focused == Some(id.as_str()) {
                        self.draw_focus_ring(fragment_rect, style, element);
                    }
                    if self.hovered == Some(id.as_str()) {
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(
                                fragment_rect.x,
                                fragment_rect.y + fragment_rect.height - 1.0,
                                fragment_rect.width,
                                1.0,
                            ),
                            color: fragment.color,
                            radius: 0.0,
                        });
                    }
                }
                source_byte += fragment.text.len();
                source_char += count;
                x += fragment.width
                    - if fragment.background.is_some() {
                        3.0
                    } else {
                        0.0
                    };
            }
            y += line.height;
        }
        self.show_text_hints(&hints);
    }
}
