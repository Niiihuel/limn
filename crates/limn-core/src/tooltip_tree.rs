use super::*;

const TOOLTIP_PADDING: f32 = 8.0;

/// The space between two blocks of a hint: a picture stands further off.
fn tooltip_block_gap(picture: bool) -> f32 {
    if picture { 12.0 } else { 6.0 }
}

struct TooltipIconLayout {
    icon: Option<Icon>,
    label: String,
    rect: Rect,
}

struct TooltipIconRowLayout {
    label_lines: Vec<String>,
    label_y: f32,
    items: Vec<TooltipIconLayout>,
    size: Size,
}

/// Lines of a hint's text block in one style.
struct TooltipRun<'a> {
    lines: Vec<&'a str>,
    style: Style,
    color: Color,
}

enum TooltipBlock<'a> {
    Separator,
    /// Runs one under the other with no space between them.
    Text(Vec<TooltipRun<'a>>),
    Picture {
        image: &'a RasterImage,
        size: Size,
    },
    IconRow(TooltipIconRowLayout),
    Faces {
        icon: Option<Icon>,
        faces: &'a [crate::TooltipFace],
        more: usize,
        width: f32,
    },
}

/// The style of a tooltip's «+N» after its faces.
fn tooltip_more_style() -> Style {
    Style {
        font_size: 12.0,
        font_family: FontFamily::SansMedium,
        ..Style::default()
    }
}

impl LayoutBuilder<'_> {
    /// The width of a «+N» pill: its text with 8 on each side, at least round.
    fn tooltip_more_width(&self, more: usize) -> f32 {
        (self.text_width(&format!("+{more}"), tooltip_more_style()) + 16.0).max(crate::TOOLTIP_FACE)
    }

    /// How a row of faces fits `available`: the faces shown, how many the
    /// «+N» says, and the row's width.
    fn measure_tooltip_faces(&self, icon: bool, faces: usize, more: usize, available: f32) -> (usize, usize, f32) {
        let lead = if icon { crate::TOOLTIP_FACE_ICON + 8.0 } else { 0.0 };
        let step = crate::TOOLTIP_FACE - crate::TOOLTIP_FACE_OVERLAP;
        let width = |shown: usize, more: usize| {
            lead + shown as f32 * step
                + crate::TOOLTIP_FACE_OVERLAP
                + if more > 0 { self.tooltip_more_width(more) - crate::TOOLTIP_FACE_OVERLAP } else { 0.0 }
        };
        let (mut shown, mut more) = (faces, more);
        while shown > 1 && width(shown, more) > available {
            shown -= 1;
            more += 1;
        }
        (shown, more, width(shown, more))
    }

    /// Paint a row of faces at `x`, `y`: each cut from the one before by a
    /// ring of the tooltip's fill, then «+N».
    fn paint_tooltip_faces(&mut self, x: f32, y: f32, icon: Option<Icon>, faces: &[crate::TooltipFace], more: usize, colors: TooltipStyle) {
        let side = crate::TOOLTIP_FACE;
        let mut x = x;
        if let Some(icon) = icon {
            let icon_side = crate::TOOLTIP_FACE_ICON;
            self.frame.commands.push(DrawCommand::Icon {
                rect: Rect::new(x, y + (side - icon_side) / 2.0, icon_side, icon_side),
                icon,
                tint: colors.text,
                transform: crate::IconTransform::IDENTITY,
            });
            x += icon_side + 8.0;
        }
        let ring = |commands: &mut Vec<DrawCommand>, rect: Rect| {
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(rect.x - 2.0, rect.y - 2.0, rect.width + 4.0, rect.height + 4.0),
                color: colors.background,
                radius: side / 2.0 + 2.0,
            });
        };
        let initial_style = Style {
            font_size: 11.0,
            font_family: FontFamily::SansMedium,
            ..Style::default()
        };
        for (index, face) in faces.iter().enumerate() {
            let rect = Rect::new(x, y, side, side);
            if index > 0 {
                ring(&mut self.frame.commands, rect);
            }
            match &face.image {
                Some(image) => self.frame.commands.push(DrawCommand::RasterImage {
                    rect,
                    image: image.clone(),
                    opacity: 1.0,
                    radius: side / 2.0,
                }),
                None => {
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect,
                        color: face.color,
                        radius: side / 2.0,
                    });
                    let initial: String = face.name.chars().next().into_iter().flat_map(char::to_uppercase).collect();
                    let width = self.text_width(&initial, initial_style);
                    let height = self.metrics_for(initial_style).line_height;
                    self.frame.commands.push(DrawCommand::Text {
                        position: Point::new(x + (side - width) / 2.0, y + (side - height) / 2.0),
                        text: initial,
                        color: Color::rgb(255, 255, 255),
                        font_size: initial_style.font_size,
                        font_family: initial_style.font_family,
                    });
                }
            }
            x += side - crate::TOOLTIP_FACE_OVERLAP;
        }
        if more > 0 {
            let label = format!("+{more}");
            let rect = Rect::new(x, y, self.tooltip_more_width(more), side);
            if !faces.is_empty() {
                ring(&mut self.frame.commands, rect);
            }
            self.frame.commands.push(DrawCommand::FillRect {
                rect,
                color: colors.keycap,
                radius: side / 2.0,
            });
            let style = tooltip_more_style();
            let width = self.text_width(&label, style);
            let height = self.metrics_for(style).line_height;
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(rect.x + (rect.width - width) / 2.0, y + (side - height) / 2.0),
                text: label,
                color: colors.text,
                font_size: style.font_size,
                font_family: style.font_family,
            });
        }
    }

    fn measure_tooltip_icon_row(
        &self,
        label: &str,
        items: &[TooltipItem],
        width: f32,
        label_width: f32,
        style: Style,
    ) -> TooltipIconRowLayout {
        let gap = 4.0;
        let item_height = ICON_SIZE + 8.0;
        let stacked = width < 160.0;
        let label_lines: Vec<_> = if label.trim().is_empty() {
            Vec::new()
        } else {
            self.wrapped_lines(label, if stacked { width } else { label_width }, style)
                .into_iter()
                .map(str::to_owned)
                .collect()
        };
        let label_height = label_lines.len() as f32 * self.metrics_for(style).line_height;
        let start_x = if stacked || label_lines.is_empty() {
            0.0
        } else {
            label_width + 10.0
        };
        let available = (width - start_x).max(0.0);
        let mut x = start_x;
        let mut y = if stacked && !label_lines.is_empty() {
            label_height + gap
        } else {
            0.0
        };
        let mut row_width = label_lines
            .iter()
            .map(|line| self.text_width(line, style))
            .fold(0.0_f32, f32::max);
        let mut measured = Vec::new();
        for item in items.iter().filter(|item| !item.label.trim().is_empty()) {
            // Very narrow viewports preserve the label when the icon cannot fit.
            let icon = item.icon.filter(|_| available >= ICON_SIZE + 8.0);
            let bare = item.icon_only && icon.is_some();
            let decoration = icon.map_or(0.0, |_| ICON_SIZE + if bare { 8.0 } else { 12.0 });
            let text = if bare {
                String::new()
            } else {
                truncate_measured(&item.label, (available - decoration).max(0.0), |text| {
                    self.text_width(text, style)
                })
            };
            let item_width = (self.text_width(&text, style) + decoration).min(available);
            if x > start_x && x + item_width > width {
                x = start_x;
                y += item_height + gap;
            }
            measured.push(TooltipIconLayout {
                icon,
                label: text,
                rect: Rect::new(x, y, item_width, item_height),
            });
            row_width = row_width.max(x + item_width);
            x += item_width + gap;
        }
        let label_y = if stacked {
            0.0
        } else {
            ((item_height - label_height) / 2.0).max(0.0)
        };
        TooltipIconRowLayout {
            label_lines,
            label_y,
            size: Size::new(
                row_width.min(width),
                (y + if measured.is_empty() {
                    0.0
                } else {
                    item_height
                })
                .max(label_y + label_height),
            ),
            items: measured,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn paint_tooltip(
        &mut self,
        anchor: Rect,
        title: &str,
        shortcut: Option<&str>,
        sections: &[TooltipSection],
        preferred_width: f32,
        preferred: TooltipPlacement,
        colors: TooltipStyle,
        options: TooltipOptions,
        viewport: Rect,
    ) -> Option<Point> {
        // A hint for a menu row must sit outside the entire menu, including
        // its padding and scrollbar. The vertical anchor remains the row.
        let row_anchor = anchor;
        let mut anchor = if matches!(preferred, TooltipPlacement::Left | TooltipPlacement::Right) {
            self.frame
                .regions
                .iter()
                .rev()
                .find(|region| {
                    region.id.starts_with("popover_surface:")
                        && region.rect.intersection(anchor).is_some()
                })
                .map_or(anchor, |region| {
                    Rect::new(region.rect.x, anchor.y, region.rect.width, anchor.height)
                })
        } else {
            anchor
        };
        let mut preferred = preferred;
        let mut max_width = (viewport.width - 16.0).min(preferred_width.max(32.0));
        if matches!(preferred, TooltipPlacement::Left | TooltipPlacement::Right) {
            let left = (anchor.x - viewport.x - 16.0).max(0.0);
            let right = (viewport.x + viewport.width - anchor.x - anchor.width - 16.0).max(0.0);
            let side_width = max_width.min(left.max(right));
            if side_width < 32.0 {
                // A narrow menu can occupy the complete viewport. Keep its
                // row's metadata accessible above/below it (or clamped within
                // the viewport) instead of silently omitting the hint. The
                // tooltip still owns no hit region, so the anchor retains its
                // pointer/keyboard behavior even when the hint must overlap.
                preferred = TooltipPlacement::Above;
                anchor = row_anchor;
            } else {
                max_width = side_width;
            }
        }
        if max_width < 32.0 || viewport.height < 32.0 {
            return None;
        }
        let shape = options.shape;
        let padding_x = shape.map_or(TOOLTIP_PADDING, |shape| shape.padding_x);
        let title_style = Style {
            font_size: options.title_font_size,
            font_family: options.title_font_family,
            ..Style::default()
        };
        // What a title taller than the default adds to a one-line hint.
        let title_growth = (self.metrics_for(title_style).line_height
            - self
                .metrics_for(Style {
                    font_size: TooltipOptions::default().title_font_size,
                    font_family: TooltipOptions::default().title_font_family,
                    ..Style::default()
                })
                .line_height)
            .max(0.0);
        let content_max = max_width - padding_x * 2.0;
        let shortcut = shortcut
            .map(shortcut::label)
            .filter(|value| !value.is_empty());
        let plain = crate::tokens().shortcut_style == crate::ShortcutStyle::Text;
        let keycap_width = shortcut
            .map(|value| {
                if plain {
                    self.menu_shortcut_width(value)
                } else {
                    self.shortcut_width(value)
                }
            })
            .filter(|width| content_max - width - 8.0 >= 24.0);
        let title_budget = content_max - keycap_width.map_or(0.0, |width| width + 8.0);
        let rich = !sections.is_empty() || title.contains('\n');
        let title_lines = if title.is_empty() {
            Vec::new()
        } else if rich {
            self.wrapped_lines(title, title_budget, title_style)
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        } else {
            vec![truncate_measured(title, title_budget, |part| {
                self.text_width(part, title_style)
            })]
        };
        let description_style = Style {
            font_size: 12.0,
            font_family: FontFamily::Sans,
            ..Style::default()
        };
        let footer_style = Style {
            font_size: 11.5,
            ..description_style
        };
        let icon_label_width = sections
            .iter()
            .filter_map(|section| match section {
                TooltipSection::IconRow { label, .. } => Some(self.text_width(label, footer_style)),
                _ => None,
            })
            .fold(0.0_f32, f32::max)
            .min(content_max * 0.4);
        let title_height = title_lines.len() as f32 * self.metrics_for(title_style).line_height;
        let header_width = title_lines
            .iter()
            .map(|line| self.text_width(line, title_style))
            .fold(0.0_f32, f32::max)
            + keycap_width.map_or(0.0, |width| width + 8.0);
        let mut content_width = header_width;
        let mut content_height = title_height.max(keycap_width.map_or(0.0, |_| shortcut::HEIGHT));
        let mut blocks = Vec::new();
        // Ignore trailing and repeated dividers, including when callers
        // construct the section data directly.
        let mut separator_pending = false;
        let mut after_picture = false;
        for section in sections {
            let block = match section {
                TooltipSection::Separator => {
                    separator_pending = true;
                    continue;
                }
                TooltipSection::Description(text) | TooltipSection::Footer(text) => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    let (style, color) = if matches!(section, TooltipSection::Footer(_)) {
                        (footer_style, colors.muted)
                    } else {
                        (description_style, colors.text)
                    };
                    TooltipBlock::Text(vec![TooltipRun {
                        lines: self.wrapped_lines(text, content_max, style),
                        style,
                        color,
                    }])
                }
                TooltipSection::Lines(spans) => {
                    let runs: Vec<_> = spans
                        .iter()
                        .filter(|span| !span.text.trim().is_empty())
                        .map(|span| {
                            let style = Style {
                                font_size: if span.font_size.is_finite() && span.font_size > 0.0 {
                                    span.font_size
                                } else {
                                    description_style.font_size
                                },
                                font_family: span.font_family,
                                ..Style::default()
                            };
                            TooltipRun {
                                lines: self.wrapped_lines(&span.text, content_max, style),
                                style,
                                color: span.color,
                            }
                        })
                        .collect();
                    if runs.is_empty() {
                        continue;
                    }
                    TooltipBlock::Text(runs)
                }
                TooltipSection::Picture { image, size } => {
                    if !(size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0) {
                        continue;
                    }
                    // One wider than the hint is scaled down whole.
                    let scale = (content_max / size.width).min(1.0);
                    TooltipBlock::Picture {
                        image,
                        size: Size::new(size.width * scale, size.height * scale),
                    }
                }
                TooltipSection::IconRow { label, items } => {
                    let row = self.measure_tooltip_icon_row(
                        label,
                        items,
                        content_max,
                        icon_label_width,
                        footer_style,
                    );
                    if row.size.height <= 0.0 {
                        continue;
                    }
                    TooltipBlock::IconRow(row)
                }
                TooltipSection::Faces { icon, faces, more } => {
                    if faces.is_empty() {
                        continue;
                    }
                    let (shown, more, width) =
                        self.measure_tooltip_faces(icon.is_some(), faces.len(), *more, content_max);
                    TooltipBlock::Faces {
                        icon: *icon,
                        faces: &faces[..shown],
                        more,
                        width,
                    }
                }
            };
            let picture = matches!(block, TooltipBlock::Picture { .. });
            if separator_pending {
                content_height += 15.0;
                blocks.push(TooltipBlock::Separator);
                separator_pending = false;
            } else if content_height > 0.0 {
                content_height += tooltip_block_gap(picture || after_picture);
            }
            after_picture = picture;
            let size = match &block {
                TooltipBlock::Text(runs) => runs.iter().fold(Size::new(0.0, 0.0), |size, run| {
                    Size::new(
                        run.lines
                            .iter()
                            .map(|line| self.text_width(line, run.style))
                            .fold(size.width, f32::max),
                        size.height + run.lines.len() as f32 * self.metrics_for(run.style).line_height,
                    )
                }),
                TooltipBlock::Picture { size, .. } => *size,
                TooltipBlock::IconRow(row) => row.size,
                TooltipBlock::Faces { width, .. } => Size::new(*width, crate::TOOLTIP_FACE),
                TooltipBlock::Separator => unreachable!(),
            };
            content_width = content_width.max(size.width);
            content_height += size.height;
            blocks.push(block);
        }
        let body_width = (padding_x * 2.0 + content_width)
            .max(TOOLTIP_HEIGHT)
            .min(max_width);
        let body_height = match shape {
            Some(shape) => content_height + shape.padding_y * 2.0,
            None if rich => (content_height + 12.0).max(TOOLTIP_HEIGHT),
            None => TOOLTIP_HEIGHT + title_growth,
        };
        let gap = match shape {
            Some(shape) if options.arrow => shape.caret.height + 2.0,
            _ => TOOLTIP_GAP,
        };
        let (body, placement) = place_tooltip(
            anchor,
            Size::new(body_width, body_height),
            viewport,
            preferred,
            gap,
        );
        if body.width < TOOLTIP_HEIGHT || body.height < TOOLTIP_HEIGHT {
            return None;
        }
        // A small pointer names the element the hint belongs to; a wide one
        // keeps clear of the rounded corners.
        let tip_margin = shape.map_or(TOOLTIP_PADDING, |shape| shape.radius + shape.caret.width / 2.0);
        let tip_x = (anchor.x + anchor.width / 2.0).clamp(
            body.x + tip_margin,
            (body.x + body.width - tip_margin).max(body.x + tip_margin),
        );
        let tip_y = (anchor.y + anchor.height / 2.0).clamp(
            body.y + tip_margin,
            (body.y + body.height - tip_margin).max(body.y + tip_margin),
        );
        // Where the hint grows from: the side that faces its anchor.
        let origin = match placement {
            TooltipPlacement::Above => Point::new(tip_x, body.y + body.height),
            TooltipPlacement::Below => Point::new(tip_x, body.y),
            TooltipPlacement::Left => Point::new(body.x + body.width, tip_y),
            TooltipPlacement::Right => Point::new(body.x, tip_y),
        };
        let vertical = |base: f32, away: f32| {
            (
                [
                    Point::new(tip_x - 5.0, base),
                    Point::new(tip_x + 5.0, base),
                    Point::new(tip_x, base + 5.0 * away),
                ],
                [
                    Point::new(tip_x - 3.5, base - away),
                    Point::new(tip_x + 3.5, base - away),
                    Point::new(tip_x, base + 3.0 * away),
                ],
            )
        };
        let horizontal = |base: f32, away: f32| {
            (
                [
                    Point::new(base, tip_y - 5.0),
                    Point::new(base, tip_y + 5.0),
                    Point::new(base + 5.0 * away, tip_y),
                ],
                [
                    Point::new(base - away, tip_y - 3.5),
                    Point::new(base - away, tip_y + 3.5),
                    Point::new(base + 3.0 * away, tip_y),
                ],
            )
        };
        let (outline, fill) = match placement {
            TooltipPlacement::Above => vertical(body.y + body.height - 1.0, 1.0),
            TooltipPlacement::Below => vertical(body.y + 1.0, -1.0),
            TooltipPlacement::Left => horizontal(body.x + body.width - 1.0, 1.0),
            TooltipPlacement::Right => horizontal(body.x + 1.0, -1.0),
        };
        let (radius, shadow) = match shape {
            Some(shape) => (
                shape.radius,
                BoxShadow::new(colors.shadow, shape.shadow_offset, shape.shadow_blur, 0.0),
            ),
            None => (
                crate::tokens().radius.control,
                BoxShadow::new(colors.shadow, Point::new(0.0, 2.0), 6.0, 0.0),
            ),
        };
        self.frame.commands.push(DrawCommand::Shadow {
            rect: body,
            shadow,
            radius,
        });
        self.frame.commands.push(DrawCommand::FillRect {
            rect: body,
            color: colors.background,
            radius,
        });
        self.frame.commands.push(DrawCommand::StrokeRect {
            rect: body,
            color: colors.border,
            width: 1.0,
            radius,
        });
        // Painted last so its fill covers the body border at the join; the
        // nested triangles leave a one-pixel outline.
        if let (true, Some(shape)) = (options.arrow, shape) {
            let (edge, outward) = match placement {
                TooltipPlacement::Above => (Point::new(tip_x, body.y + body.height), Point::new(0.0, 1.0)),
                TooltipPlacement::Below => (Point::new(tip_x, body.y), Point::new(0.0, -1.0)),
                TooltipPlacement::Left => (Point::new(body.x + body.width, tip_y), Point::new(1.0, 0.0)),
                TooltipPlacement::Right => (Point::new(body.x, tip_y), Point::new(-1.0, 0.0)),
            };
            self.frame
                .commands
                .extend(rounded_caret(edge, outward, shape.caret, colors.background, colors.border));
        } else if options.arrow {
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: outline,
                color: colors.border,
            });
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: fill,
                color: colors.background,
            });
        }
        let padding_y = shape.map_or(if rich { 6.0 } else { 1.0 }, |shape| shape.padding_y);
        let content = Rect::new(
            body.x + padding_x,
            body.y + padding_y,
            (body.width - padding_x * 2.0).max(0.0),
            (body.height - padding_y * 2.0).max(0.0),
        );
        self.frame.commands.push(DrawCommand::PushClip(content));
        let mut text_y = if rich {
            content.y
        } else {
            body.y + (body.height - title_height) / 2.0
        };
        // Where something `width` wide starts: the left edge, or centered.
        let start = |width: f32| {
            if options.centered {
                content.x + (content.width - width).max(0.0) / 2.0
            } else {
                content.x
            }
        };
        for line in title_lines {
            let x = start(self.text_width(&line, title_style));
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(x, text_y),
                text: line,
                color: colors.text,
                font_size: title_style.font_size,
                font_family: title_style.font_family,
            });
            text_y += self.metrics_for(title_style).line_height;
        }
        if rich {
            text_y = content.y + title_height.max(keycap_width.map_or(0.0, |_| shortcut::HEIGHT));
        }
        if let (Some(shortcut), Some(width)) = (shortcut, keycap_width) {
            let keycap = Rect::new(
                body.x + body.width - padding_x - width,
                if rich {
                    content.y
                } else {
                    body.y + (body.height - shortcut::HEIGHT) / 2.0
                },
                width,
                shortcut::HEIGHT,
            );
            if plain {
                self.paint_menu_shortcut(keycap, shortcut, colors.muted);
            } else {
                self.paint_shortcut(keycap, shortcut, colors.keycap, colors.text);
            }
        }
        let mut divided = false;
        let mut has_content = title_height > 0.0 || keycap_width.is_some();
        let mut after_picture = false;
        for block in blocks {
            if matches!(block, TooltipBlock::Separator) {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: Rect::new(content.x, text_y + 7.0, content.width, 1.0),
                    color: colors.border,
                    radius: 0.0,
                });
                text_y += 15.0;
                divided = true;
                continue;
            }
            let picture = matches!(block, TooltipBlock::Picture { .. });
            if !divided && has_content {
                text_y += tooltip_block_gap(picture || after_picture);
            }
            after_picture = picture;
            divided = false;
            has_content = true;
            match block {
                TooltipBlock::Text(runs) => {
                    for run in runs {
                        for line in run.lines {
                            let x = start(self.text_width(line, run.style));
                            self.frame.commands.push(DrawCommand::Text {
                                position: Point::new(x, text_y),
                                text: line.to_owned(),
                                color: run.color,
                                font_size: run.style.font_size,
                                font_family: run.style.font_family,
                            });
                            text_y += self.metrics_for(run.style).line_height;
                        }
                    }
                }
                TooltipBlock::Picture { image, size } => {
                    self.frame.commands.push(DrawCommand::RasterImage {
                        rect: Rect::new(start(size.width), text_y, size.width, size.height),
                        image: image.clone(),
                        opacity: 1.0,
                        radius: 0.0,
                    });
                    text_y += size.height;
                }
                TooltipBlock::IconRow(row) => {
                    let line_height = self.metrics_for(footer_style).line_height;
                    let mut label_y = text_y + row.label_y;
                    // The whole row moves to the middle; its own layout stays.
                    let left = start(row.size.width);
                    for label in row.label_lines {
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(left, label_y),
                            text: label,
                            color: colors.muted,
                            font_size: footer_style.font_size,
                            font_family: footer_style.font_family,
                        });
                        label_y += line_height;
                    }
                    for item in row.items {
                        let rect = Rect::new(
                            left + item.rect.x,
                            text_y + item.rect.y,
                            item.rect.width,
                            item.rect.height,
                        );
                        let text_x = if let Some(icon) = item.icon {
                            self.frame.commands.push(DrawCommand::StrokeRect {
                                rect,
                                color: colors.border,
                                width: 1.0,
                                radius: 3.0,
                            });
                            self.frame.commands.push(DrawCommand::Icon {
                                rect: Rect::new(rect.x + 4.0, rect.y + 4.0, ICON_SIZE, ICON_SIZE),
                                icon,
                                tint: colors.text,
                                transform: crate::IconTransform::IDENTITY,
                            });
                            rect.x + 4.0 + ICON_SIZE + 4.0
                        } else {
                            rect.x
                        };
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(
                                text_x,
                                rect.y + (rect.height - line_height) / 2.0,
                            ),
                            text: item.label,
                            color: if item.icon.is_some() {
                                colors.text
                            } else {
                                colors.muted
                            },
                            font_size: footer_style.font_size,
                            font_family: footer_style.font_family,
                        });
                    }
                    text_y += row.size.height;
                }
                TooltipBlock::Faces { icon, faces, more, width } => {
                    self.paint_tooltip_faces(start(width), text_y, icon, faces, more, colors);
                    text_y += crate::TOOLTIP_FACE;
                }
                TooltipBlock::Separator => unreachable!(),
            }
        }
        self.frame.commands.push(DrawCommand::PopClip);
        Some(origin)
    }
}
