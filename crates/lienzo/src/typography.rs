//! Semantic text roles and width-aware line breaking for a host text renderer.
//!
//! The caller must measure with the same font and shaping engine it uses to
//! paint. A fixed average advance is not reliable for proportional text.

use unicode_linebreak::linebreaks;
use unicode_segmentation::UnicodeSegmentation;

/// The interface type scale in logical pixels, independent of display DPI.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextRole {
    Display,
    Heading,
    Body,
    Label,
    Caption,
    Small,
    Micro,
    Code,
}

impl TextRole {
    pub const fn size(self) -> f32 {
        match self {
            Self::Display => 18.0,
            Self::Heading => 16.0,
            Self::Body | Self::Code => 14.0,
            Self::Label => 13.0,
            Self::Caption | Self::Small => 12.0,
            Self::Micro => 10.0,
        }
    }

    pub const fn monospaced(self) -> bool {
        matches!(self, Self::Code)
    }
}

/// Shorten a single-line label to the width available in the painter.
///
/// `measure` must use the same font, size, and shaping engine as the renderer.
/// The result contains an ellipsis only when text was removed and that mark
/// itself fits. Grapheme boundaries keep combining marks and joined emoji
/// intact. A width too small for even the ellipsis produces an empty label.
pub fn truncate_measured(
    value: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> String {
    let limit = if max_width.is_nan() {
        0.0
    } else {
        max_width.max(0.0)
    };
    let fits = |width: f32| width.is_finite() && width <= limit;
    if value.is_empty() || limit == f32::INFINITY || fits(measure(value)) {
        return value.to_owned();
    }

    const ELLIPSIS: &str = "…";
    if !fits(measure(ELLIPSIS)) {
        return String::new();
    }

    let boundaries = std::iter::once(0)
        .chain(
            value
                .grapheme_indices(true)
                .map(|(start, part)| start + part.len()),
        )
        .collect::<Vec<_>>();
    let mut low = 0usize;
    // The full value was already rejected, so at least one grapheme must be
    // removed even if a painter reports a shorter width for an added mark.
    let mut high = boundaries.len() - 2;
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let mut candidate = String::with_capacity(boundaries[middle] + ELLIPSIS.len());
        candidate.push_str(&value[..boundaries[middle]]);
        candidate.push_str(ELLIPSIS);
        if fits(measure(&candidate)) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut result = String::with_capacity(boundaries[low] + ELLIPSIS.len());
    result.push_str(&value[..boundaries[low]]);
    result.push_str(ELLIPSIS);
    result
}

/// Wrap text using the painter's measured width. The returned slices borrow
/// `value`, preserve every non-newline byte, and never divide a grapheme.
/// Explicit newlines create separate paragraphs, including a trailing empty
/// line. A single grapheme wider than `max_width` stays intact and may overflow.
/// Unicode line-break opportunities keep closing punctuation with its word
/// and respect opening delimiters and CJK text. Oversized unbreakable tokens
/// still wrap at grapheme boundaries so they remain readable within the width.
///
/// This is intended for labels and paragraphs; a long editor needs shaped
/// line/glyph caches and incremental layout on top of this contract.
pub fn wrap_measured(
    value: &str,
    max_width: f32,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<&str> {
    let limit = if max_width.is_finite() {
        max_width.max(0.0)
    } else {
        f32::INFINITY
    };
    let mut lines = Vec::new();
    for paragraph in value.split('\n') {
        if paragraph.is_empty() {
            lines.push(paragraph);
            continue;
        }

        // Exponential probing followed by a binary search measures only near
        // the next line, rather than reshaping the full remaining paragraph
        // for every line of a long streaming response.
        let boundaries = std::iter::once(0)
            .chain(
                paragraph
                    .grapheme_indices(true)
                    .map(|(start, part)| start + part.len()),
            )
            .collect::<Vec<_>>();
        let break_ends = linebreaks(paragraph)
            .map(|(end, _)| end)
            // UAX #14 supplies line boundaries; UAX #29 still owns the
            // indivisible graphemes used by the painter and width probes.
            .filter(|end| boundaries.binary_search(end).is_ok())
            .collect::<Vec<usize>>();
        let mut first = 0;
        let last = boundaries.len() - 1;
        while first < last {
            let start = boundaries[first];
            let mut fitting = first;
            let mut span = 1usize;
            let mut probe = (first + span).min(last);
            loop {
                let width = measure(&paragraph[start..boundaries[probe]]);
                if !(width.is_finite() && width <= limit) {
                    break;
                }
                fitting = probe;
                if fitting == last {
                    break;
                }
                span = span.saturating_mul(2);
                probe = first.saturating_add(span).min(last);
            }
            if fitting == last {
                lines.push(&paragraph[start..]);
                break;
            }
            let mut lo = fitting + 1;
            let mut hi = probe;
            while lo < hi {
                let mid = lo + (hi - lo) / 2;
                let candidate_width = measure(&paragraph[start..boundaries[mid]]);
                if candidate_width.is_finite() && candidate_width <= limit {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            let fitting = lo.saturating_sub(1).max(first + 1);
            let fitted_end = boundaries[fitting];
            let opportunity = break_ends.partition_point(|end| *end <= fitted_end);
            let split = opportunity
                .checked_sub(1)
                .map(|index| break_ends[index])
                .filter(|end| *end > start)
                .unwrap_or(fitted_end);
            lines.push(&paragraph[start..split]);
            first = boundaries.binary_search(&split).unwrap_or(fitting);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_keep_one_logical_scale() {
        assert_eq!(TextRole::Display.size(), 18.0);
        assert_eq!(TextRole::Heading.size(), 16.0);
        assert_eq!(TextRole::Body.size(), 14.0);
        assert_eq!(TextRole::Label.size(), 13.0);
        assert_eq!(TextRole::Caption.size(), 12.0);
        assert_eq!(TextRole::Small.size(), 12.0);
        assert_eq!(TextRole::Micro.size(), 10.0);
        assert!(TextRole::Code.monospaced());
        assert!(!TextRole::Body.monospaced());
    }

    #[test]
    fn truncation_uses_painted_width_instead_of_character_count() {
        let measure = |part: &str| {
            part.chars()
                .map(|ch| match ch {
                    'W' => 8.0,
                    'i' => 2.0,
                    '…' => 3.0,
                    _ => 4.0,
                })
                .sum()
        };
        assert_eq!(truncate_measured("WWii", 13.0, measure), "W…");
        assert_eq!(truncate_measured("iiii", 13.0, measure), "iiii");
        assert_eq!(truncate_measured("WWii", 20.0, measure), "WWii");
    }

    #[test]
    fn truncation_respects_font_size_and_exact_fit() {
        let label = "Testing";
        let width_at = |size: f32, part: &str| part.graphemes(true).count() as f32 * size;
        assert_eq!(
            truncate_measured(label, 30.0, |part| width_at(5.0, part)),
            "Testi…"
        );
        assert_eq!(
            truncate_measured(label, 30.0, |part| width_at(10.0, part)),
            "Te…"
        );
        assert_eq!(
            truncate_measured(label, 70.0, |part| width_at(10.0, part)),
            label
        );
    }

    #[test]
    fn truncation_never_splits_combining_marks_or_joined_emoji() {
        let label = "e\u{301}👩‍💻🏳️‍🌈rest";
        assert_eq!(
            truncate_measured(label, 3.0, |part| part.graphemes(true).count() as f32),
            "e\u{301}👩‍💻…"
        );
        assert_eq!(
            truncate_measured(label, 2.0, |part| part.graphemes(true).count() as f32),
            "e\u{301}…"
        );
    }

    #[test]
    fn truncation_hides_mark_if_it_cannot_fit() {
        assert_eq!(
            truncate_measured("abc", 0.5, |part| part.chars().count() as f32),
            ""
        );
        assert_eq!(
            truncate_measured("abc", 1.0, |part| part.chars().count() as f32),
            "…"
        );
        assert_eq!(truncate_measured("", 0.0, |_| 0.0), "");
        assert_eq!(truncate_measured("abc", f32::INFINITY, |_| 99.0), "abc");
        assert_eq!(
            truncate_measured("abc", f32::NAN, |part| part.len() as f32),
            ""
        );
        assert_eq!(
            truncate_measured("ab", 2.0, |part| match part {
                "ab" => 3.0,
                "ab…" => 1.0,
                _ => part.chars().count() as f32,
            }),
            "a…"
        );
    }

    #[test]
    fn truncation_measures_logarithmically_many_candidates() {
        let label = "界".repeat(10_000);
        let mut calls = 0;
        let shortened = truncate_measured(&label, 11.0, |part| {
            calls += 1;
            part.graphemes(true).count() as f32
        });
        assert_eq!(shortened, format!("{}…", "界".repeat(10)));
        assert!(calls <= 20, "too many measurements: {calls}");
    }

    #[test]
    fn uses_real_width_and_prefers_word_boundaries() {
        let lines = wrap_measured("ii WW ii", 7.0, |text| {
            text.chars()
                .map(|ch| match ch {
                    'W' => 3.0,
                    'i' => 1.0,
                    _ => 0.5,
                })
                .sum()
        });
        assert_eq!(lines, ["ii ", "WW ", "ii"]);
    }

    #[test]
    fn sentence_punctuation_moves_with_a_word_that_fits_on_the_next_line() {
        for text in ["one task.", "one task!", "one task?", "one task,"] {
            let lines = wrap_measured(text, 8.0, |part| part.chars().count() as f32);
            assert_eq!(lines, ["one ", &text[4..]], "{text}");
            assert_eq!(lines.concat(), text);
        }

        // This is the Marketplace paragraph and width that left its final
        // period on a line of its own with word-boundary segmentation.
        let text = "Helps users discover and install agent skills when they ask questions like how do I do X, find a skill for X, or is there a skill that can help with a task.";
        let measure = |part: &str| part.chars().count() as f32 * (8.0 * (13.0 / 14.0));
        for width in [290.0, 393.0] {
            let lines = wrap_measured(text, width, measure);
            assert_eq!(lines.concat(), text);
            assert!(lines.last().unwrap().ends_with("task."));
            assert!(lines
                .iter()
                .all(|line| *line != "." && measure(line) <= width));
        }
    }

    #[test]
    fn closing_punctuation_uses_the_same_proportional_measurement_as_its_word() {
        let measure = |part: &str| {
            part.chars()
                .map(|ch| match ch {
                    'W' => 3.0,
                    '.' => 0.5,
                    ' ' => 0.5,
                    _ => 1.0,
                })
                .sum::<f32>()
        };
        let text = "WW task.";
        let lines = wrap_measured(text, measure("WW task"), measure);
        assert_eq!(lines, ["WW ", "task."]);
        assert_eq!(lines.concat(), text);
    }

    #[test]
    fn paired_delimiters_and_cjk_punctuation_remain_attached() {
        for text in ["one (task)", "one [task]", "one {task}"] {
            let lines = wrap_measured(text, 9.0, |part| part.chars().count() as f32);
            assert_eq!(lines, ["one ", &text[4..]], "{text}");
            assert_eq!(lines.concat(), text);
        }
        for text in [
            "天地玄黄。",
            "天地（玄黄）宇宙。",
            "天地「玄黄」宇宙。",
            "天地、玄黄。",
        ] {
            let lines = wrap_measured(text, 4.0, |part| part.graphemes(true).count() as f32);
            assert_eq!(lines.concat(), text);
            assert!(lines.iter().all(|line| line.graphemes(true).count() <= 4));
            assert!(
                lines.iter().all(|line| {
                    !line.starts_with(['。', '、', '）', '」']) && !line.ends_with(['（', '「'])
                }),
                "{text}: {lines:?}"
            );
        }
    }

    #[test]
    fn unbroken_unicode_tokens_preserve_all_bytes_and_grapheme_boundaries() {
        let text = format!("{} 尾巴。", "e\u{301}👩‍💻🏳️‍🌈".repeat(128));
        let lines = wrap_measured(&text, 5.0, |part| part.graphemes(true).count() as f32);
        assert_eq!(lines.concat(), text);
        let boundaries = std::iter::once(0)
            .chain(
                text.grapheme_indices(true)
                    .map(|(start, part)| start + part.len()),
            )
            .collect::<Vec<_>>();
        let mut offset = 0;
        for line in lines {
            assert!(!line.is_empty());
            assert!(line.graphemes(true).count() <= 5);
            assert!(boundaries.binary_search(&offset).is_ok());
            offset += line.len();
            assert!(boundaries.binary_search(&offset).is_ok());
        }
        assert_eq!(offset, text.len());
    }

    #[test]
    fn never_splits_combining_sequence_or_emoji() {
        let text = "e\u{301}👩‍💻x";
        let lines = wrap_measured(text, 1.0, |part| part.graphemes(true).count() as f32);
        assert_eq!(lines, ["e\u{301}", "👩‍💻", "x"]);
    }

    #[test]
    fn preserves_empty_paragraphs_and_every_visible_byte() {
        let text = "one\n\ntwo\n";
        let lines = wrap_measured(text, 99.0, |part| part.len() as f32);
        assert_eq!(lines, ["one", "", "two", ""]);
    }

    #[test]
    fn oversized_grapheme_is_not_discarded() {
        let text = "🎨x";
        let lines = wrap_measured(text, 0.0, |part| part.graphemes(true).count() as f32);
        assert_eq!(lines, ["🎨", "x"]);
    }

    #[test]
    fn long_unbroken_response_measures_bounded_local_slices() {
        let text = "x".repeat(10_000);
        let mut calls = 0usize;
        let mut largest_slice = 0usize;
        let lines = wrap_measured(&text, 10.0, |part| {
            calls += 1;
            largest_slice = largest_slice.max(part.len());
            part.len() as f32
        });
        assert_eq!(lines.len(), 1_000);
        assert!(lines.iter().all(|line| line.len() == 10));
        assert!(
            largest_slice <= 16,
            "reshaped a distant suffix: {largest_slice}"
        );
        assert!(calls < 10_000, "too many painter measurements: {calls}");
    }
}
