use super::*;

#[derive(Clone, Debug)]
pub(super) struct FormattedFragment {
    pub(super) text: String,
    pub(super) color: Color,
    pub(super) font_size: f32,
    pub(super) font_family: FontFamily,
    pub(super) background: Option<Color>,
    pub(super) action: Option<String>,
    pub(super) icon: Option<Icon>,
    pub(super) underline: bool,
    pub(super) strikethrough: bool,
    pub(super) tooltip: Option<TextHint>,
    /// The «…» that ends a text cut to its `max_lines`: painted, but not
    /// part of the source.
    pub(super) ellipsis: bool,
    /// A picture painted in place of `text`, `width` on a side.
    pub(super) image: Option<InlineImage>,
    pub(super) width: f32,
    pub(super) line_height: f32,
}

#[derive(Debug)]
pub(super) struct FormattedLine {
    pub(super) fragments: Vec<FormattedFragment>,
    pub(super) width: f32,
    pub(super) height: f32,
}

impl FormattedLine {
    pub(super) fn new(height: f32) -> Self {
        Self {
            fragments: Vec::new(),
            width: 0.0,
            height,
        }
    }
}

fn same_span_style(fragment: &FormattedFragment, span: &TextSpan) -> bool {
    fragment.icon.is_none()
        && fragment.image.is_none()
        && fragment.color == span.color
        && fragment.font_size == span.font_size
        && fragment.font_family == span.font_family
        && fragment.background == span.background
        && fragment.action == span.action
        && fragment.underline == span.underline
        && fragment.strikethrough == span.strikethrough
        && fragment.tooltip == span.tooltip
}

fn formatted_advance(
    line: &FormattedLine,
    span: &TextSpan,
    text: &str,
    measure: &mut impl FnMut(&str, &TextSpan) -> f32,
) -> f32 {
    let inset = if span.background.is_some() { 6.0 } else { 0.0 };
    if let Some(last) = line
        .fragments
        .last()
        .filter(|last| last.text.len() <= 256 && same_span_style(last, span))
    {
        let mut combined = last.text.clone();
        combined.push_str(text);
        measure(&combined, span) + inset - last.width
    } else {
        measure(text, span) + inset
    }
}

fn push_formatted_fragment(
    line: &mut FormattedLine,
    span: &TextSpan,
    text: &str,
    measure: &mut impl FnMut(&str, &TextSpan) -> f32,
    line_height: &mut impl FnMut(&TextSpan) -> f32,
) {
    let inset = if span.background.is_some() { 6.0 } else { 0.0 };
    let height = line_height(span);
    if let Some(last) = line
        .fragments
        .last_mut()
        .filter(|last| last.text.len() <= 256 && same_span_style(last, span))
    {
        let previous_width = last.width;
        last.text.push_str(text);
        last.width = measure(&last.text, span) + inset;
        line.width += last.width - previous_width;
        line.height = line.height.max(height);
        return;
    }
    let width = measure(text, span) + inset;
    line.fragments.push(FormattedFragment {
        text: text.to_owned(),
        color: span.color,
        font_size: span.font_size,
        font_family: span.font_family,
        background: span.background,
        action: span.action.clone(),
        icon: None,
        underline: span.underline,
        strikethrough: span.strikethrough,
        tooltip: span.tooltip.clone(),
        ellipsis: false,
        image: None,
        width,
        line_height: height,
    });
    line.width += width;
    line.height = line.height.max(height);
}

/// Flow styled runs in one paragraph. Tokens share a line until the painter's
/// measured width reaches the available width; oversized words are split only
/// at Unicode grapheme boundaries. A small background inset is included in
/// measurement and paint so inline code does not overlap adjacent text.
pub(super) fn layout_formatted(
    spans: &[TextSpan],
    max_width: f32,
    default_line_height: f32,
    mut measure: impl FnMut(&str, &TextSpan) -> f32,
    mut line_height: impl FnMut(&TextSpan) -> f32,
) -> Vec<FormattedLine> {
    // A hundredth past the width, as `TextLayout` allows: a box handed back the
    // width it measured by sums can come out a rounding error short of it.
    let limit = if max_width.is_finite() {
        max_width.max(0.0) + 0.01
    } else {
        f32::INFINITY
    };
    let mut lines = Vec::new();
    let mut current = FormattedLine::new(default_line_height);
    for span in spans {
        // A picture is one square that moves to the next line whole.
        if let Some(image) = span.image.as_ref().filter(|_| !span.text.is_empty()) {
            let side = span.font_size * crate::INLINE_IMAGE_SCALE;
            if current.width + side > limit && !current.fragments.is_empty() {
                lines.push(std::mem::replace(
                    &mut current,
                    FormattedLine::new(default_line_height),
                ));
            }
            let height = line_height(span).max(side);
            current.fragments.push(FormattedFragment {
                text: span.text.clone(),
                color: span.color,
                font_size: span.font_size,
                font_family: span.font_family,
                background: None,
                action: span.action.clone(),
                icon: None,
                // Lines are not drawn under a picture, as under an icon.
                underline: false,
                strikethrough: false,
                tooltip: span.tooltip.clone(),
                ellipsis: false,
                image: Some(image.clone()),
                width: side,
                line_height: height,
            });
            current.width += side;
            current.height = current.height.max(height);
            continue;
        }
        if let Some(icon) = span.icon.filter(|_| !span.text.is_empty()) {
            let width = ICON_SIZE + 4.0;
            let first = span.text.split_word_bounds().next().unwrap_or_default();
            if current.width + width + measure(first, span) > limit && !current.fragments.is_empty()
            {
                lines.push(std::mem::replace(
                    &mut current,
                    FormattedLine::new(default_line_height),
                ));
            }
            let height = line_height(span).max(ICON_SIZE);
            current.fragments.push(FormattedFragment {
                text: String::new(),
                color: span.color,
                font_size: span.font_size,
                font_family: span.font_family,
                background: None,
                action: span.action.clone(),
                icon: Some(icon),
                underline: false,
                strikethrough: false,
                tooltip: span.tooltip.clone(),
                ellipsis: false,
                image: None,
                width,
                line_height: height,
            });
            current.width += width;
            current.height = current.height.max(height);
        }
        for (paragraph_index, paragraph) in span.text.split('\n').enumerate() {
            if paragraph_index > 0 {
                lines.push(std::mem::replace(
                    &mut current,
                    FormattedLine::new(default_line_height),
                ));
            }
            let tokens: Vec<&str> = if span.background.is_some() {
                vec![paragraph]
            } else {
                paragraph
                    .split_word_bound_indices()
                    .map(|(_, token)| token)
                    .collect()
            };
            for token in tokens.into_iter().filter(|token| !token.is_empty()) {
                let inset = if span.background.is_some() { 6.0 } else { 0.0 };
                let token_advance = formatted_advance(&current, span, token, &mut measure);
                let only_icon = current.fragments.len() == 1 && current.fragments[0].icon.is_some();
                if current.width + token_advance > limit
                    && !current.fragments.is_empty()
                    && !only_icon
                {
                    lines.push(std::mem::replace(
                        &mut current,
                        FormattedLine::new(default_line_height),
                    ));
                }
                if current.width + formatted_advance(&current, span, token, &mut measure) <= limit
                    || token.graphemes(true).count() == 1
                {
                    push_formatted_fragment(
                        &mut current,
                        span,
                        token,
                        &mut measure,
                        &mut line_height,
                    );
                    continue;
                }
                let mut piece = String::new();
                for grapheme in token.graphemes(true) {
                    let mut candidate = piece.clone();
                    candidate.push_str(grapheme);
                    if !piece.is_empty()
                        && current.width + measure(&candidate, span) + inset > limit
                    {
                        push_formatted_fragment(
                            &mut current,
                            span,
                            &piece,
                            &mut measure,
                            &mut line_height,
                        );
                        lines.push(std::mem::replace(
                            &mut current,
                            FormattedLine::new(default_line_height),
                        ));
                        piece.clear();
                    }
                    piece.push_str(grapheme);
                }
                if !piece.is_empty() {
                    push_formatted_fragment(
                        &mut current,
                        span,
                        &piece,
                        &mut measure,
                        &mut line_height,
                    );
                }
            }
        }
    }
    lines.push(current);
    lines
}

/// Keep the first `max` of `lines`. When more follow, the last one kept
/// ends in «…» in the style of its last fragment, giving up graphemes, then
/// whole icons and pictures, from its end until it fits `max_width`, and
/// the spaces left before the «…».
pub(super) fn clamp_formatted(
    mut lines: Vec<FormattedLine>,
    max: usize,
    max_width: f32,
    mut measure: impl FnMut(&str, &TextSpan) -> f32,
    mut line_height: impl FnMut(&TextSpan) -> f32,
) -> Vec<FormattedLine> {
    if lines.len() <= max {
        return lines;
    }
    lines.truncate(max.max(1));
    let Some(style) = lines
        .iter()
        .rev()
        .find_map(|line| line.fragments.last())
        .map(|last| TextSpan::new("…", last.color, last.font_size, last.font_family))
    else {
        return lines;
    };
    let limit = if max_width.is_finite() {
        max_width.max(0.0)
    } else {
        f32::INFINITY
    };
    let ellipsis_width = measure("…", &style);
    let line = lines.last_mut().expect("at least one line is kept");
    let as_span = |fragment: &FormattedFragment| {
        TextSpan::new("", fragment.color, fragment.font_size, fragment.font_family)
    };
    loop {
        let over = line.width + ellipsis_width > limit;
        let Some(last) = line.fragments.last_mut() else {
            break;
        };
        let blank = last.text.ends_with(char::is_whitespace);
        if !over && !blank {
            break;
        }
        // An icon or a picture goes whole.
        if last.icon.is_some() || last.image.is_some() {
            line.width -= last.width;
            line.fragments.pop();
            continue;
        }
        let cut = last
            .text
            .grapheme_indices(true)
            .next_back()
            .map_or(0, |(at, _)| at);
        last.text.truncate(cut);
        if last.text.is_empty() {
            line.width -= last.width;
            line.fragments.pop();
            continue;
        }
        let inset = if last.background.is_some() { 6.0 } else { 0.0 };
        let width = measure(&last.text, &as_span(last)) + inset;
        line.width += width - last.width;
        last.width = width;
    }
    let height = line_height(&style);
    line.fragments.push(FormattedFragment {
        text: style.text,
        color: style.color,
        font_size: style.font_size,
        font_family: style.font_family,
        background: None,
        action: None,
        icon: None,
        underline: false,
        strikethrough: false,
        tooltip: None,
        ellipsis: true,
        image: None,
        width: ellipsis_width,
        line_height: height,
    });
    line.width += ellipsis_width;
    line.height = line.height.max(height);
    lines
}
