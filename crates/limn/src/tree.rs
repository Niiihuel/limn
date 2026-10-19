#[path = "horizontal_scroll_tree.rs"]
mod horizontal_scroll_tree;
use horizontal_scroll_tree::{HorizontalPan, HorizontalScrollState};
#[path = "corner_mask_tree.rs"]
mod corner_mask_tree;
#[path = "plot_tree.rs"]
mod plot_tree;
#[cfg(test)]
#[path = "popover_initial_focus_tests.rs"]
mod popover_initial_focus_tests;
#[cfg(test)]
#[path = "coach_mark_tests.rs"]
mod coach_mark_tests;

#[cfg(test)]
#[path = "drag_drop_tests.rs"]
mod drag_drop_tests;
#[cfg(test)]
#[path = "sticky_tests.rs"]
mod sticky_tests;
#[cfg(test)]
#[path = "top_fade_tests.rs"]
mod top_fade_tests;
#[cfg(test)]
#[path = "text_marks_tests.rs"]
mod text_marks_tests;
#[cfg(test)]
#[path = "scrollbar_visibility_tests.rs"]
mod scrollbar_visibility_tests;
#[cfg(test)]
#[path = "horizontal_scrollbar_visibility_tests.rs"]
mod horizontal_scrollbar_visibility_tests;
#[cfg(test)]
#[path = "translation_tests.rs"]
mod translation_tests;
#[cfg(test)]
#[path = "pointer_blocking_tests.rs"]
mod pointer_blocking_tests;
#[cfg(test)]
#[path = "soft_focus_tests.rs"]
mod soft_focus_tests;
#[cfg(test)]
#[path = "stack_tests.rs"]
mod stack_tests;
#[cfg(test)]
#[path = "wrap_tests.rs"]
mod wrap_tests;
#[cfg(test)]
#[path = "area_slider_tests.rs"]
mod area_slider_tests;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use web_time::{Duration, Instant};

use unicode_segmentation::UnicodeSegmentation;

use crate::element::{CoachAlign, CoachAnchor, CoachSide, HoverReveal, MeasuredRows, UniformList};
use crate::motion::{
    skeleton_color, spinner_angle, ColorTransition, ScalarSpring, ScalarTransition,
};
use crate::shortcut;
use crate::scroll_area::ScrollbarState;
use crate::text_width_cache::TextWidthCache;
use crate::text_layout_cache::TextLayoutCache;
#[path = "formatted_text_cache.rs"]
mod formatted_text_cache;
use formatted_text_cache::FormattedTextCache;
#[path = "measurement_cache.rs"]
mod measurement_cache;
use measurement_cache::{fills_available_width, MeasurementCache};
#[path = "inline_menu_navigation.rs"]
mod inline_menu_navigation;
use inline_menu_navigation::InlineMenuFocusScope;
#[path = "context_menu_tree.rs"]
mod context_menu_tree;
use context_menu_tree::{ContextMenuRegion, ContextMenuState};
#[path = "ime_tree.rs"]
mod ime_tree;
#[path = "captured_paste.rs"]
mod captured_paste;
pub use captured_paste::PasteTarget;
use crate::typography::{truncate_measured, wrap_measured};
use crate::{
    BoxShadow, Color, CornerRadii, Cursor, Element, ElementKind, FontFamily, Icon, InlineImage, Point, PopoverPlacement,
    RasterImage, Rect, ScrollMotion, Size, SpinnerVariant, Style, TextHint, TextSpan, TooltipItem,
    TooltipOptions, TooltipPlacement, TooltipSection, TooltipStyle, DEFAULT_FONT_SIZE, ICON_SIZE,
};

const TOOLTIP_HEIGHT: f32 = 24.0;
const TOOLTIP_PADDING: f32 = 8.0;
/// How far a [`Element::rise_on_hover`] element travels up as it appears.
const REVEAL_RISE: f32 = 8.0;
pub(crate) const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(350);
/// The start of the hover target of a text run's hint that has no action;
/// the index of its text in the frame and the hint's title follow.
const TEXT_HINT_PREFIX: &str = "__text_hint:";
/// Where a text run's underline and strikethrough sit, as fractions of its
/// natural line height from the top of its glyphs.
const TEXT_UNDERLINE_AT: f32 = 0.86;
const TEXT_STRIKETHROUGH_AT: f32 = 0.55;
const SLIDER_THUMB_INSET: f32 = 17.0;
/// One arrow press on a slider without stops.
const CONTINUOUS_SLIDER_KEY_STEP: f32 = 0.05;
const SLIDER_SETTLE_DURATION: Duration = Duration::from_millis(220);
const SLIDER_ULTRA_FADE_DURATION: Duration = Duration::from_millis(300);
const SLIDER_BURST_DURATION: Duration = Duration::from_millis(280);
const SCROLL_BUTTON_STEP: f32 = 48.0;
const SCROLL_REPEAT_DELAY: Duration = Duration::from_millis(350);
const SCROLL_REPEAT_INTERVAL: Duration = Duration::from_millis(60);

/// Text dimensions at [`DEFAULT_FONT_SIZE`], scaled in proportion for other
/// sizes. The tree measures with them when the host has installed no text
/// measurer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMetrics {
    /// Advance of one monospace character in logical pixels.
    pub char_width: f32,
    /// Height of one row of text in logical pixels.
    pub line_height: f32,
}

impl Default for FontMetrics {
    fn default() -> Self {
        Self {
            char_width: 8.0,
            line_height: 18.0,
        }
    }
}

impl FontMetrics {
    fn at_size(self, size: f32) -> Self {
        let scale = size / DEFAULT_FONT_SIZE;
        Self {
            char_width: self.char_width * scale,
            line_height: self.line_height * scale,
        }
    }
}

type TextMeasurer = dyn Fn(&str, f32, FontFamily) -> f32 + Send + Sync;
type TextHeightMeasurer = dyn Fn(f32, FontFamily) -> f32 + Send + Sync;

fn measured_width(
    measurer: Option<&TextMeasurer>,
    text: &str,
    size: f32,
    family: FontFamily,
    fallback_char_width: f32,
) -> f32 {
    let fallback = text.chars().count() as f32 * fallback_char_width;
    measurer
        .map(|measure| measure(text, size, family))
        .filter(|width| width.is_finite() && *width >= 0.0)
        .unwrap_or(fallback)
}

#[derive(Clone, Debug)]
struct FormattedFragment {
    text: String,
    color: Color,
    font_size: f32,
    font_family: FontFamily,
    background: Option<Color>,
    action: Option<String>,
    icon: Option<Icon>,
    underline: bool,
    strikethrough: bool,
    tooltip: Option<TextHint>,
    /// The «…» that ends a text cut to its `max_lines`: painted, but not
    /// part of the source.
    ellipsis: bool,
    /// A picture painted in place of `text`, `width` on a side.
    image: Option<InlineImage>,
    width: f32,
    line_height: f32,
}

#[derive(Debug)]
struct FormattedLine {
    fragments: Vec<FormattedFragment>,
    width: f32,
    height: f32,
}

impl FormattedLine {
    fn new(height: f32) -> Self {
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
fn layout_formatted(
    spans: &[TextSpan],
    max_width: f32,
    default_line_height: f32,
    mut measure: impl FnMut(&str, &TextSpan) -> f32,
    mut line_height: impl FnMut(&TextSpan) -> f32,
) -> Vec<FormattedLine> {
    let limit = if max_width.is_finite() {
        max_width.max(0.0)
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
fn clamp_formatted(
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

/// One paint instruction of a [`Frame`], in logical pixels. A backend
/// carries out `Frame::commands` in order.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    /// A box shadow, pushed before the fill of the box that casts it.
    Shadow {
        /// The box that casts the shadow, not the shadow's own extent.
        rect: Rect,
        /// Color, offset, blur and spread of the shadow.
        shadow: BoxShadow,
        /// Corner radius of the casting box.
        radius: f32,
    },
    /// A filled rectangle with the same radius on every corner.
    FillRect {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
        /// Corner radius; 0 is a square corner.
        radius: f32,
    },
    /// A filled rectangle whose corners do not all have the same radius.
    FillCornerRect {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
        /// Radius of each corner.
        radii: CornerRadii,
    },
    /// One implicitly closed, simple Y-monotone selection contour. Renderers
    /// tessellate its interior once and antialias only its exterior boundary.
    SelectionContour {
        /// The contour's vertices, relative to the frame's top-left corner; the
        /// last joins the first.
        points: Arc<[Point]>,
        /// Fill color of the interior.
        color: Color,
    },
    /// Paint the exterior of four rounded corners as a continuous surface.
    /// Renderers must not antialias internal tessellation edges independently.
    CornerMask {
        /// The rectangle whose corners are masked.
        rect: Rect,
        /// Color of the area between the rectangle's square corners and its
        /// rounded ones.
        color: Color,
        /// Radius of each rounded corner; the mask covers what lies outside it.
        radii: CornerRadii,
    },
    /// A rectangle filled with a linear gradient from its top edge to its
    /// bottom edge. The tree paints fades with it, one end transparent.
    FillVerticalGradient {
        /// Bounds of the gradient.
        rect: Rect,
        /// Color along the top edge.
        top: Color,
        /// Color along the bottom edge.
        bottom: Color,
    },
    /// A rectangle filled with a gradient from `left` to `right`, with square
    /// corners: the length of a bar that runs sideways.
    FillHorizontalGradient {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Color at the left edge.
        left: Color,
        /// Color at the right edge.
        right: Color,
    },
    /// A rectangle outline with the same radius on every corner. The egui
    /// backend strokes inside `rect`.
    StrokeRect {
        /// Outer edge of the outline.
        rect: Rect,
        /// Stroke color; its alpha is the outline's opacity.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// Corner radius of the outer edge.
        radius: f32,
    },
    /// A rectangle outline whose corners do not all have the same radius. The
    /// egui backend strokes inside `rect`.
    StrokeCornerRect {
        /// Outer edge of the outline.
        rect: Rect,
        /// Stroke color; its alpha is the outline's opacity.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// Radius of each corner of the outer edge.
        radii: CornerRadii,
    },
    /// A filled triangle: the caret of a tooltip or coach mark, or the arrow of
    /// a scrollbar button.
    FillTriangle {
        /// The three vertices, relative to the frame's top-left corner.
        points: [Point; 3],
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
    },
    /// Triangles with a color at each corner, blended across each: the body of
    /// an area under a line, a slice of a pie, the shape of a radar's series.
    /// Its edges are not smoothed; what draws it strokes them.
    FillMesh {
        /// The corners, relative to the frame's top-left corner.
        points: Arc<[Point]>,
        /// The color at each corner, one for each point.
        colors: Arc<[Color]>,
        /// Three indices into `points` for each triangle.
        indices: Arc<[u32]>,
    },
    /// An open stroked path through `points`, centered on them. A dashed
    /// path alternates short marks and gaps along its length.
    Polyline {
        /// The vertices in order, relative to the frame's top-left corner.
        points: Arc<[Point]>,
        /// Stroke color.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// True for a dashed stroke; the egui backend draws 3 pixel marks and 3
        /// pixel gaps.
        dashed: bool,
    },
    /// Glyphs limn has shaped and placed; see [`crate::GlyphRun`]. A backend
    /// draws each glyph where the run says, and lays nothing out again.
    Glyphs(crate::GlyphRun),
    /// A run of text the tree has already measured and wrapped. A backend
    /// paints it from `position` without wrapping it again.
    Text {
        /// Top-left corner of the run.
        position: Point,
        /// The run's characters.
        text: String,
        /// Color of the glyphs.
        color: Color,
        /// Font size in logical pixels.
        font_size: f32,
        /// The face to paint in: monospace, or a weight or slant of the sans
        /// face.
        font_family: FontFamily,
    },
    /// One of the library's icons, drawn in one color.
    Icon {
        /// Where the icon is drawn; the egui backend aligns it to the pixel
        /// grid.
        rect: Rect,
        /// Which of the library's icons to draw.
        icon: Icon,
        /// Color the icon's shape is drawn in.
        tint: Color,
    },
    /// A picture. A cover image fills `rect` and is cropped; any other is
    /// fitted inside it.
    RasterImage {
        /// The area given to the picture.
        rect: Rect,
        /// The picture, with the text alternative a backend gives to assistive
        /// technology.
        image: RasterImage,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
        /// Corner radius the picture is rounded to.
        radius: f32,
    },
    /// Pixels the host presents to the backend under `id`, outside the frame.
    /// They start at the rectangle's top-left corner at their own density and
    /// are clipped to the rectangle, never stretched to fill it.
    Surface {
        /// The area the surface shows in; its pixels are clipped to it.
        rect: Rect,
        /// The ID the host presents the surface's pixels under.
        id: String,
        /// The name a backend gives to assistive technology for the surface.
        label: String,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
    },
    /// An activity indicator that turns, sized to the shorter side of `rect`.
    Spinner {
        /// Bounds of the indicator.
        rect: Rect,
        /// Stroke color.
        color: Color,
        /// Rotation in radians, from 0 to a full turn every 0.9 seconds. Always
        /// 0 under reduced motion.
        angle: f32,
        /// Which shape turns: an open circle or eight spokes.
        variant: SpinnerVariant,
    },
    /// A record behind its sleeve. `rect` is the sleeve; the record's
    /// diameter is `RECORD_SHARE` of its side and at `out` 1 it shows
    /// `RECORD_REACH` of the side beyond the sleeve's end. `fill` paints a
    /// sleeve with no picture.
    Record {
        /// The sleeve. The egui backend draws it as a square of the shorter
        /// side, from the top-left corner.
        rect: Rect,
        /// The sleeve's picture, cropped to cover it. `None` paints `fill`.
        image: Option<RasterImage>,
        /// Sleeve color when there is no picture: the element's background, or
        /// transparent without one.
        fill: Color,
        /// How far the record is drawn out, from 0 (centered behind the sleeve)
        /// to 1.
        out: f32,
        /// The record's turn in radians, from 0 to a full turn.
        angle: f32,
        /// Corner radius of the sleeve.
        radius: f32,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
    },
    /// A circle of `track` with an arc of `fill` over it, starting at twelve
    /// o'clock and running clockwise. It is centered in `rect` and sized to the
    /// shorter side.
    ProgressRing {
        /// Bounds of the ring.
        rect: Rect,
        /// Share of the circle the arc covers, from 0 to 1. `None` draws the
        /// track alone.
        fraction: Option<f32>,
        /// Color of the whole circle underneath.
        track: Color,
        /// Color of the arc.
        fill: Color,
    },
    /// The animated effort slider: a pill-shaped rail filled up to a round
    /// knob, with a dot for each step and particles drifting in the fill.
    EffortSlider {
        /// Bounds of the control; the rail is centered vertically in them.
        rect: Rect,
        /// Where the knob is shown, 0 at the left end and 1 at the right. It
        /// follows the pointer during a drag and eases to the value after it.
        fraction: f32,
        /// Number of levels. The egui backend draws their dots when there are 2
        /// to 12.
        steps: usize,
        /// Clock of the idle animation in seconds, wrapping at 1000. It does
        /// not advance under reduced motion.
        phase: f32,
        /// True while the pointer holds the knob.
        dragging: bool,
        /// True when the tree was told to reduce motion: a backend leaves out
        /// the knob's pulse and the burst.
        reduced_motion: bool,
        /// How much of the top level's gradient covers the fill, from 0 to 1.
        /// It eases to 1 while the slider is on its last step and that level is
        /// available.
        ultra_mix: f32,
        /// Seconds since the slider settled on its top level, for the 0.28
        /// seconds the burst lasts. `None` at any other time.
        burst_age: Option<f32>,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
    },
    /// Clip every command up to the matching `PopClip` to this rectangle,
    /// intersected with the clips already in force.
    PushClip(Rect),
    /// Round the clip of the `PushClip` just before it: until that clip's
    /// `PopClip`, fills and images stay inside `rect` with these corners, so
    /// content flush with or past the edge of a rounded panel keeps the
    /// panel's corners. Text is not affected. A nested rounded clip replaces
    /// it; a nested plain one keeps it. A backend that cannot round a clip
    /// may ignore this and keep the rectangle.
    RoundClip {
        /// The rounded shape, inside the rectangle of the clip it rounds.
        rect: Rect,
        /// Its corners.
        radii: CornerRadii,
    },
    /// End the innermost `PushClip`.
    PopClip,
}

impl DrawCommand {
    /// A translated copy at `opacity` of its own: a drag preview, or a
    /// surface on its way to where it is laid out.
    fn moved(&self, dx: f32, dy: f32, opacity: f32) -> Self {
        let fade = |color: &mut Color| color.a = (f32::from(color.a) * opacity).round() as u8;
        let shift = |rect: &mut Rect| {
            rect.x += dx;
            rect.y += dy;
        };
        let opacity_factor = opacity;
        let mut ghost = self.clone();
        match &mut ghost {
            Self::Shadow { rect, shadow, .. } => {
                shift(rect);
                fade(&mut shadow.color);
            }
            Self::FillRect { rect, color, .. }
            | Self::FillCornerRect { rect, color, .. }
            | Self::CornerMask { rect, color, .. }
            | Self::StrokeRect { rect, color, .. }
            | Self::StrokeCornerRect { rect, color, .. }
            | Self::Spinner { rect, color, .. }
            | Self::Icon {
                rect, tint: color, ..
            } => {
                shift(rect);
                fade(color);
            }
            Self::FillVerticalGradient { rect, top, bottom } => {
                shift(rect);
                fade(top);
                fade(bottom);
            }
            Self::FillHorizontalGradient { rect, left, right } => {
                shift(rect);
                fade(left);
                fade(right);
            }
            Self::ProgressRing {
                rect, track, fill, ..
            } => {
                shift(rect);
                fade(track);
                fade(fill);
            }
            Self::RasterImage { rect, opacity, .. }
            | Self::Record { rect, opacity, .. }
            | Self::Surface { rect, opacity, .. }
            | Self::EffortSlider { rect, opacity, .. } => {
                shift(rect);
                *opacity *= opacity_factor;
            }
            Self::SelectionContour { points, color } | Self::Polyline { points, color, .. } => {
                *points = points
                    .iter()
                    .map(|point| Point::new(point.x + dx, point.y + dy))
                    .collect();
                fade(color);
            }
            Self::FillMesh { points, colors, .. } => {
                *points = points.iter().map(|point| Point::new(point.x + dx, point.y + dy)).collect();
                *colors = colors
                    .iter()
                    .map(|color| {
                        let mut color = *color;
                        fade(&mut color);
                        color
                    })
                    .collect();
            }
            Self::FillTriangle { points, color } => {
                for point in points {
                    *point = Point::new(point.x + dx, point.y + dy);
                }
                fade(color);
            }
            Self::Text {
                position, color, ..
            } => {
                *position = Point::new(position.x + dx, position.y + dy);
                fade(color);
            }
            Self::Glyphs(run) => {
                run.origin = Point::new(run.origin.x + dx, run.origin.y + dy);
                fade(&mut run.color);
            }
            Self::PushClip(rect) | Self::RoundClip { rect, .. } => shift(rect),
            Self::PopClip => {}
        }
        ghost
    }
}

/// `from` with `amount` of the way to `to`, channel by channel.
fn mix(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |from: u8, to: u8| (f32::from(from) + (f32::from(to) - f32::from(from)) * amount).round() as u8;
    Color::rgba(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
        channel(from.a, to.a),
    )
}

/// A record turns once in this long.
const RECORD_TURN: Duration = Duration::from_secs(3);
/// The record's diameter, as a share of the sleeve's side.
pub const RECORD_SHARE: f32 = 0.8;
/// How much of the sleeve's side a record drawn out shows beyond it.
pub const RECORD_REACH: f32 = 0.22;

/// How far out of its sleeve a record is, and how far it has turned.
#[derive(Clone, Debug)]
struct RecordMotion {
    out: ScalarTransition,
    /// The angle reached when it last stopped, in radians.
    turned: f32,
    spinning_since: Option<Instant>,
}

impl RecordMotion {
    fn angle(&self, now: Instant) -> f32 {
        let since = self.spinning_since.map_or(0.0, |since| now.saturating_duration_since(since).as_secs_f32());
        (self.turned + std::f32::consts::TAU * since / RECORD_TURN.as_secs_f32()) % std::f32::consts::TAU
    }

    /// Starts or stops the turn, keeping the angle across a stop.
    fn spin(&mut self, spinning: bool, now: Instant) {
        if spinning == self.spinning_since.is_some() {
            return;
        }
        self.turned = self.angle(now);
        self.spinning_since = spinning.then_some(now);
    }

    fn is_moving(&self, now: Instant) -> bool {
        self.spinning_since.is_some() || self.out.is_active(now)
    }
}

fn paint_spinner(
    commands: &mut Vec<DrawCommand>,
    rect: Rect,
    color: Color,
    variant: SpinnerVariant,
    started: Instant,
    now: Instant,
    reduced_motion: bool,
) {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    commands.push(DrawCommand::Spinner {
        rect,
        color,
        angle: spinner_angle(started, now, reduced_motion),
        variant,
    });
}

/// A vertical fade that keeps to its element's rounded corners. The straight
/// part is one gradient; under the corners, at the top or at the bottom, it
/// is rows a pixel high, each as wide as the corner leaves it.
fn paint_fade(commands: &mut Vec<DrawCommand>, rect: Rect, top: Color, bottom: Color, radius: f32, corners_at_top: bool) {
    let rounded = radius.min(rect.height).min(rect.width / 2.0).max(0.0).floor();
    let mix = |share: f32| {
        let channel = |from: u8, to: u8| (f32::from(from) + (f32::from(to) - f32::from(from)) * share).round() as u8;
        Color { r: channel(top.r, bottom.r), g: channel(top.g, bottom.g), b: channel(top.b, bottom.b), a: channel(top.a, bottom.a) }
    };
    let straight = if corners_at_top {
        Rect::new(rect.x, rect.y + rounded, rect.width, rect.height - rounded)
    } else {
        Rect::new(rect.x, rect.y, rect.width, rect.height - rounded)
    };
    if straight.height > 0.0 {
        commands.push(DrawCommand::FillVerticalGradient {
            rect: straight,
            top: mix((straight.y - rect.y) / rect.height),
            bottom: mix((straight.y + straight.height - rect.y) / rect.height),
        });
    }
    for row in 0..rounded as usize {
        // How far this row is from the edge the corners are on, at its middle.
        let depth = row as f32 + 0.5;
        let inset = radius - (radius * radius - (radius - depth).powi(2)).max(0.0).sqrt();
        let y = if corners_at_top { rect.y + row as f32 } else { rect.y + rect.height - row as f32 - 1.0 };
        commands.push(DrawCommand::FillRect {
            rect: Rect::new(rect.x + inset, y, (rect.width - 2.0 * inset).max(0.0), 1.0),
            color: mix((y + 0.5 - rect.y) / rect.height),
            radius: 0.0,
        });
    }
}

/// Evenly spaced `stops` across `rect`, left to right or top to bottom, its
/// four corners rounded by `radius`: a gradient between each two stops, cut
/// short of the rounded ends, and one-pixel strips across each end, narrowed
/// to the curve as `paint_fade` narrows its rows.
fn paint_gradient(
    commands: &mut Vec<DrawCommand>,
    rect: Rect,
    stops: &[Color],
    vertical: bool,
    radius: f32,
) {
    let (start, length, across) = if vertical {
        (rect.y, rect.height, rect.width)
    } else {
        (rect.x, rect.width, rect.height)
    };
    if stops.is_empty() || !(length > 0.0 && across > 0.0) {
        return;
    }
    let spans = stops.len().saturating_sub(1).max(1);
    // The color a share of the way along: between the two stops around it.
    let at = |share: f32| {
        let place = share.clamp(0.0, 1.0) * spans as f32;
        let index = (place.floor() as usize).min(spans - 1);
        let (from, to) = (stops[index], stops[(index + 1).min(stops.len() - 1)]);
        let mix = |from: u8, to: u8| {
            (f32::from(from) + (f32::from(to) - f32::from(from)) * (place - index as f32)).round()
                as u8
        };
        Color {
            r: mix(from.r, to.r),
            g: mix(from.g, to.g),
            b: mix(from.b, to.b),
            a: mix(from.a, to.a),
        }
    };
    // The part of `rect` from `from` to `to` along the gradient, `inset` in from both sides.
    let band = |from: f32, to: f32, inset: f32| {
        if vertical {
            Rect::new(
                rect.x + inset,
                start + from,
                across - 2.0 * inset,
                to - from,
            )
        } else {
            Rect::new(
                start + from,
                rect.y + inset,
                to - from,
                across - 2.0 * inset,
            )
        }
    };
    let rounded = if radius.is_finite() {
        radius.min(length / 2.0).min(across / 2.0).max(0.0).floor()
    } else {
        0.0
    };
    for span in 0..spans {
        let from = (length * span as f32 / spans as f32).max(rounded);
        let to = (length * (span + 1) as f32 / spans as f32).min(length - rounded);
        if to > from {
            let (first, last) = (at(from / length), at(to / length));
            commands.push(if vertical {
                DrawCommand::FillVerticalGradient {
                    rect: band(from, to, 0.0),
                    top: first,
                    bottom: last,
                }
            } else {
                DrawCommand::FillHorizontalGradient {
                    rect: band(from, to, 0.0),
                    left: first,
                    right: last,
                }
            });
        }
    }
    for step in 0..rounded as usize {
        let depth = step as f32 + 0.5;
        let inset = rounded
            - (rounded * rounded - (rounded - depth).powi(2))
                .max(0.0)
                .sqrt();
        for from in [step as f32, length - step as f32 - 1.0] {
            commands.push(DrawCommand::FillRect {
                rect: band(from, from + 1.0, inset),
                color: at((from + 0.5) / length),
                radius: 0.0,
            });
        }
    }
}

fn paint_corner_rect(
    commands: &mut Vec<DrawCommand>,
    rect: Rect,
    color: Color,
    width: Option<f32>,
    radius: f32,
    corners: Option<CornerRadii>,
) {
    let mut emit = |radius| {
        commands.push(if let Some(width) = width {
            DrawCommand::StrokeRect {
                rect,
                color,
                width,
                radius,
            }
        } else {
            DrawCommand::FillRect {
                rect,
                color,
                radius,
            }
        });
    };
    let Some(corners) = corners else {
        emit(radius);
        return;
    };
    let radii = [
        corners.top_left,
        corners.top_right,
        corners.bottom_right,
        corners.bottom_left,
    ];
    if radii.iter().all(|radius| *radius == radii[0]) {
        emit(radii[0]);
        return;
    }
    commands.push(if let Some(width) = width {
        DrawCommand::StrokeCornerRect {
            rect,
            color,
            width,
            radii: corners,
        }
    } else {
        DrawCommand::FillCornerRect {
            rect,
            color,
            radii: corners,
        }
    });
}

/// Complete the dashed straight edges with rounded corner arcs. The stroke is
/// centered inside the panel bounds so a scroll clip cannot cut its outer half.
fn paint_dashed_corners(
    commands: &mut Vec<DrawCommand>,
    rect: Rect,
    radius: f32,
    width: f32,
    color: Color,
) {
    let radius = radius.min(rect.width / 2.0).min(rect.height / 2.0);
    let dot = width.max(1.25);
    let arc_radius = radius - dot / 2.0;
    if arc_radius <= 0.0 {
        return;
    }
    let arc_length = std::f32::consts::FRAC_PI_2 * arc_radius;
    let steps = (arc_length / 1.1).ceil().max(1.0) as usize;
    let corners = [
        (rect.x + rect.width - radius, rect.y + radius, -1.0_f32),
        (
            rect.x + rect.width - radius,
            rect.y + rect.height - radius,
            0.0,
        ),
        (rect.x + radius, rect.y + rect.height - radius, 1.0),
        (rect.x + radius, rect.y + radius, 2.0),
    ];
    for (center_x, center_y, quarter_turn) in corners {
        for step in 0..=steps {
            let progress = step as f32 / steps as f32;
            let arc_distance = progress * arc_length;
            if arc_distance.rem_euclid(9.0) >= 5.0 {
                continue;
            }
            let angle = (quarter_turn + progress) * std::f32::consts::FRAC_PI_2;
            let x = center_x + arc_radius * angle.cos();
            let y = center_y + arc_radius * angle.sin();
            commands.push(DrawCommand::FillRect {
                rect: Rect::new(x - dot / 2.0, y - dot / 2.0, dot, dot),
                color,
                radius: dot / 2.0,
            });
        }
    }
}

/// Move and fade a floating surface's paint only. Its layout, pointer regions,
/// scroll geometry, and focus targets stay at their final coordinates, so a
/// short entrance cannot make a menu's click target drift under the pointer.
fn animate_popover_command(
    command: &mut DrawCommand,
    opacity: f32,
    slide_y: f32,
    scale: f32,
    origin: Point,
) {
    fn fade(color: &mut Color, opacity: f32) {
        color.a = (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
    }
    fn scale_point(point: &mut Point, scale: f32, origin: Point, slide_y: f32) {
        point.x = origin.x + (point.x - origin.x) * scale;
        point.y = origin.y + (point.y - origin.y) * scale + slide_y;
    }
    fn scale_rect(rect: &mut Rect, scale: f32, origin: Point, slide_y: f32) {
        let mut top_left = Point::new(rect.x, rect.y);
        scale_point(&mut top_left, scale, origin, slide_y);
        rect.x = top_left.x;
        rect.y = top_left.y;
        rect.width *= scale;
        rect.height *= scale;
    }
    match command {
        DrawCommand::Shadow {
            rect,
            shadow,
            radius,
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *radius *= scale;
            shadow.offset.x *= scale;
            shadow.offset.y *= scale;
            shadow.blur *= scale;
            shadow.spread *= scale;
            fade(&mut shadow.color, opacity);
        }
        DrawCommand::FillRect {
            rect,
            color,
            radius,
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *radius *= scale;
            fade(color, opacity);
        }
        DrawCommand::SelectionContour { points, color } => {
            for point in Arc::make_mut(points) {
                scale_point(point, scale, origin, slide_y);
            }
            fade(color, opacity);
        }
        DrawCommand::Polyline { points, color, width, .. } => {
            for point in Arc::make_mut(points) {
                scale_point(point, scale, origin, slide_y);
            }
            *width *= scale;
            fade(color, opacity);
        }
        DrawCommand::FillMesh { points, colors, .. } => {
            for point in Arc::make_mut(points) {
                scale_point(point, scale, origin, slide_y);
            }
            for color in Arc::make_mut(colors) {
                fade(color, opacity);
            }
        }
        DrawCommand::FillCornerRect { rect, color, radii }
        | DrawCommand::CornerMask { rect, color, radii } => {
            scale_rect(rect, scale, origin, slide_y);
            radii.top_left *= scale;
            radii.top_right *= scale;
            radii.bottom_right *= scale;
            radii.bottom_left *= scale;
            fade(color, opacity);
        }
        DrawCommand::StrokeRect {
            rect,
            color,
            width,
            radius,
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *radius *= scale;
            *width *= scale;
            fade(color, opacity);
        }
        DrawCommand::StrokeCornerRect {
            rect,
            color,
            width,
            radii,
        } => {
            scale_rect(rect, scale, origin, slide_y);
            radii.top_left *= scale;
            radii.top_right *= scale;
            radii.bottom_right *= scale;
            radii.bottom_left *= scale;
            *width *= scale;
            fade(color, opacity);
        }
        DrawCommand::FillVerticalGradient { rect, top, bottom } => {
            scale_rect(rect, scale, origin, slide_y);
            fade(top, opacity);
            fade(bottom, opacity);
        }
        DrawCommand::FillHorizontalGradient { rect, left: start, right: end } => {
            scale_rect(rect, scale, origin, slide_y);
            fade(start, opacity);
            fade(end, opacity);
        }
        DrawCommand::FillTriangle { points, color } => {
            for point in points {
                scale_point(point, scale, origin, slide_y);
            }
            fade(color, opacity);
        }
        DrawCommand::Text {
            position,
            color,
            font_size,
            ..
        } => {
            scale_point(position, scale, origin, slide_y);
            *font_size *= scale;
            fade(color, opacity);
        }
        DrawCommand::Glyphs(run) => {
            scale_point(&mut run.origin, scale, origin, slide_y);
            run.size *= scale;
            // Placed from the run's origin, the glyphs scale about it.
            if scale != 1.0 {
                run.glyphs = run.glyphs.iter().map(|glyph| crate::Glyph { x: glyph.x * scale, y: glyph.y * scale, ..*glyph }).collect();
            }
            fade(&mut run.color, opacity);
        }
        DrawCommand::Icon { rect, tint, .. } => {
            scale_rect(rect, scale, origin, slide_y);
            fade(tint, opacity);
        }
        DrawCommand::RasterImage {
            rect,
            opacity: image_opacity,
            radius,
            ..
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *image_opacity *= opacity.clamp(0.0, 1.0);
            *radius *= scale;
        }
        DrawCommand::Spinner { rect, color, .. } => {
            scale_rect(rect, scale, origin, slide_y);
            fade(color, opacity);
        }
        DrawCommand::Record {
            rect,
            opacity: record_opacity,
            radius,
            ..
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *record_opacity *= opacity.clamp(0.0, 1.0);
            *radius *= scale;
        }
        DrawCommand::ProgressRing {
            rect, track, fill, ..
        } => {
            scale_rect(rect, scale, origin, slide_y);
            fade(track, opacity);
            fade(fill, opacity);
        }
        DrawCommand::Surface {
            rect,
            opacity: own_opacity,
            ..
        }
        | DrawCommand::EffortSlider {
            rect,
            opacity: own_opacity,
            ..
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *own_opacity *= opacity.clamp(0.0, 1.0);
        }
        DrawCommand::PushClip(rect) => scale_rect(rect, scale, origin, slide_y),
        DrawCommand::RoundClip { rect, radii } => {
            scale_rect(rect, scale, origin, slide_y);
            radii.top_left *= scale;
            radii.top_right *= scale;
            radii.bottom_right *= scale;
            radii.bottom_left *= scale;
        }
        DrawCommand::PopClip => {}
    }
}

fn popover_shell_command(command: &DrawCommand, rect: Rect) -> bool {
    match command {
        DrawCommand::Shadow { rect: painted, .. }
        | DrawCommand::FillRect { rect: painted, .. }
        | DrawCommand::StrokeRect { rect: painted, .. }
        | DrawCommand::FillCornerRect { rect: painted, .. }
        | DrawCommand::CornerMask { rect: painted, .. }
        | DrawCommand::StrokeCornerRect { rect: painted, .. } => *painted == rect,
        _ => false,
    }
}

fn morph_popover_shell(command: &mut DrawCommand, rect: Rect) {
    match command {
        DrawCommand::Shadow { rect: painted, .. }
        | DrawCommand::FillRect { rect: painted, .. }
        | DrawCommand::StrokeRect { rect: painted, .. }
        | DrawCommand::FillCornerRect { rect: painted, .. }
        | DrawCommand::CornerMask { rect: painted, .. }
        | DrawCommand::StrokeCornerRect { rect: painted, .. } => *painted = rect,
        _ => {}
    }
}

/// What a [`HitRegion`] belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HitKind {
    /// A code editor's viewport: it takes focus, text, the editor's pointer
    /// gestures and scrolling on both axes.
    CodeEditor,
    /// A read-only diff's viewport: it scrolls on both axes and hands vertical
    /// travel it cannot use to the scrolling areas around it.
    CodeDiff,
    /// Anything pressable: a press and a release on it report
    /// `UiAction::Clicked`. A slider uses it too.
    Button,
    /// Read-only pointer target, such as a chart observation's tooltip.
    Hover,
    /// A single-line text field: a press focuses it and places the caret.
    TextInput,
    /// A multi-line text field: as `TextInput`, and the wheel scrolls its
    /// lines.
    TextArea,
    /// A non-activating floating surface that blocks backdrop dismissal.
    Surface,
    /// A scrolling area's viewport: the wheel over it moves its content. A
    /// press neither presses nor focuses it.
    Scroll,
    /// A scrollbar's rail: its thumb drags, its buttons step and a press on the
    /// rail pages.
    Scrollbar,
}

/// A rectangle of the frame that answers the pointer, with the ID of its
/// element. Of two regions under a point, the later one is on top.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    /// ID of the element the region belongs to.
    pub id: String,
    /// The part of the element that answers the pointer: its rectangle cut to
    /// the clip in force, relative to the frame's top-left corner.
    pub rect: Rect,
    /// What the tree does with a press, the wheel or focus here.
    pub kind: HitKind,
}

/// The result of one layout: what to paint, where the pointer lands and
/// what the platform is told.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// Paint instructions in the order a backend carries them out: a later one
    /// covers an earlier one.
    pub commands: Vec<DrawCommand>,
    /// Pointer targets from bottom to top: of two under a point, the later one
    /// answers.
    pub regions: Vec<HitRegion>,
    /// What the platform should know of the frame: one node for each control
    /// and each thing a component described, in the order they were placed.
    pub access: Vec<crate::AccessNode>,
    action_menu_items: HashSet<String>,
    menu_secondary_actions: HashSet<String>,
    draggables: HashSet<String>,
    cursors: HashMap<String, Cursor>,
    drop_targets: HashSet<String>,
    submenu_targets: HashMap<String, String>,
    submenu_triggers: HashSet<String>,
    sliders: HashMap<String, SliderGeometry>,
    scroll_extents: HashMap<String, f32>,
    horizontal_viewports: HashMap<String, Rect>,
    /// Each button inside a `drag_to_pan` viewport, with the viewport a drag
    /// that starts on it pans.
    pan_items: HashMap<String, String>,
    scrollbars: HashMap<String, ScrollbarGeometry>,
    scroll_motion: HashMap<String, ScrollMotion>,
    scroll_focus_targets: HashMap<String, ScrollFocusTarget>,
    scroll_focus_candidates: HashSet<String>,
    text_blocks: Vec<SelectableTextBlock>,
    popover_focus: Vec<PopoverFocusScope>,
    tab_focus: Vec<TabFocusScope>,
    inline_menu_focus: Vec<InlineMenuFocusScope>,
    virtual_lists: HashMap<String, VirtualListFocus>,
    context_menu_regions: Vec<ContextMenuRegion>,
    coach: Option<CoachFrame>,
}

/// The coach mark rendered this frame, laid out or not.
#[derive(Clone, Debug, PartialEq)]
struct CoachFrame {
    key: String,
    target: String,
    focus: Option<String>,
    missing: bool,
}

/// Where a coach mark's ring and bubble are painted while they travel to a
/// new step's target: the ring's rectangle and the bubble's origin.
#[derive(Debug)]
struct CoachMotion {
    key: String,
    values: [ScalarSpring; 6],
}

impl Frame {
    /// The target of a coach mark that was rendered but could not be placed:
    /// no control with that ID is laid out in view, so nothing was painted.
    /// The host moves on to another step.
    pub fn missing_coach_target(&self) -> Option<&str> {
        self.coach
            .as_ref()
            .filter(|coach| coach.missing)
            .map(|coach| coach.target.as_str())
    }
}

/// Read-only diagnostic copy of laid-out text, including wrapped runs and
/// their visible clip rectangles. Selection bookkeeping remains private.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayoutBlock {
    /// The block's whole text, before wrapping.
    pub source: String,
    /// The pieces `source` was laid out in.
    pub runs: Vec<TextLayoutRun>,
}

/// One run of a [`TextLayoutBlock`]: its text, its rectangle and its
/// visible clip rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct TextLayoutRun {
    /// The characters of this run.
    pub text: String,
    /// Where the run was laid out, before clipping.
    pub rect: Rect,
    /// The part of `rect` that is shown after clipping. A run with `None` does
    /// not answer the pointer.
    pub visible: Option<Rect>,
}

/// What a measured list keeps between layouts.
#[derive(Debug, Default)]
struct MeasuredListState {
    /// The height each row was last measured at, by its key.
    heights: HashMap<u64, f32>,
    /// The key of the row at the top of the viewport at the last layout, and
    /// where its top was in the content. When rows above it change or are
    /// added, the offset moves by as much and the row stays where it showed.
    anchor: Option<(u64, f32)>,
    /// The offset of the end of the content at the last layout.
    end: f32,
}

impl MeasuredListState {
    /// Keep the height row `index`, named `key`, was measured at, in the
    /// state and in this layout's `heights`. True when it differs from what
    /// the layout had counted for it.
    fn learn(&mut self, heights: &mut [f32], key: u64, index: usize, height: f32) -> bool {
        self.heights.insert(key, height);
        let changed = (height - heights[index]).abs() > 0.01;
        heights[index] = height;
        changed
    }
}

/// The most layouts of a measured list's rows one pass runs before it
/// settles: each one measures the rows that came into view with the heights
/// the one before learned.
const MEASURED_LIST_PASSES: usize = 8;

#[derive(Clone, Debug, PartialEq)]
struct VirtualListFocus {
    first_visible: usize,
    end_visible: usize,
    row_count: usize,
    rows: Vec<(usize, Vec<(String, HitKind)>)>,
}

impl VirtualListFocus {
    fn focusables(&self) -> impl Iterator<Item = &(String, HitKind)> {
        self.rows.iter().flat_map(|(_, focusables)| focusables)
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PopoverFocusScope {
    dismiss_id: String,
    trigger_id: Option<String>,
    focusables: Vec<(String, HitKind)>,
    initial_focus: Option<String>,
    return_focus: Option<String>,
    submenu: bool,
    modal: bool,
    /// A coach mark's actions for the Left and Right arrow keys.
    arrows: Option<(Option<String>, Option<String>)>,
    /// Placed at a point, not against a trigger: the arrow keys reach it
    /// from wherever the focus is.
    pointed: bool,
}

fn modal_popover_scopes<'a>(
    scopes: &'a [PopoverFocusScope],
    closed: &HashSet<String>,
) -> Vec<&'a PopoverFocusScope> {
    let modal = scopes.iter().rposition(|scope| scope.modal);
    let mut allowed = HashSet::new();
    let mut exposed = Vec::new();
    for scope in &scopes[modal.unwrap_or(0)..] {
        if scope.submenu && scope.trigger_id.as_ref().is_some_and(|id| closed.contains(id)) {
            continue;
        }
        // Hover children are absent from their parent's static focus order.
        // Admit their descendants transitively, but never a covered base menu.
        if modal.is_none()
            || scope.modal
            || scope.trigger_id.as_deref().is_some_and(|id| allowed.contains(id))
            || scope.focusables.iter().any(|(id, _)| allowed.contains(id.as_str()))
        {
            allowed.extend(scope.focusables.iter().map(|(id, _)| id.as_str()));
            exposed.push(scope);
        }
    }
    exposed
}

/// A `sticky_top` child held back to be placed above its scroll area: the
/// element, where it is drawn, the clip, and the fade under it with its
/// strength.
type PinnedChild = (Element, Rect, Option<Rect>, Option<(Rect, Rect, f32)>);

/// Height of the fade at the cut of a capped `sticky_top` child.
const STICKY_FADE: f32 = 16.0;

/// Room [`UiTree::reveal`] leaves between a control and its viewport's edge.
const REVEAL_MARGIN: f32 = 12.0;

#[derive(Clone, Debug, PartialEq)]
struct ScrollFocusTarget {
    scroll_id: String,
    rect: Rect,
    viewport: Rect,
    layout_offset: f32,
    /// Scroll areas around the innermost one, outermost first. Each must
    /// scroll too, or a partly visible inner list keeps its control clipped.
    ancestors: Vec<(String, Rect, f32)>,
}

struct ScrollFocusContext {
    id: String,
    viewport: Rect,
    offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct TabFocusScope {
    action_ids: Vec<String>,
    /// Up and Down step it, and it stops at its ends.
    vertical: bool,
    /// Every arrow steps it, and it goes round at its ends: a group of choices.
    every_arrow: bool,
}

#[derive(Debug)]
struct TabIndicatorState {
    selected_id: String,
    // Springs, so an indicator redirected mid-flight keeps its speed instead
    // of stopping and starting again.
    x: ScalarSpring,
    width: ScalarSpring,
}

/// Final-position paint for a floating surface. It is deliberately separate
/// from regions and focus scopes so closing can animate visually without
/// leaving any invisible interactive target behind.
#[derive(Clone)]
struct PopoverSnapshot {
    commands: Vec<DrawCommand>,
    animation_ms: u16,
    close_animation_ms: Option<u16>,
    open_scale: f32,
    view_key: Option<String>,
    rect: Rect,
    slide_sign: f32,
    /// The dimming behind a dialog; it fades with the surface.
    scrim: Option<(Rect, Color)>,
}

/// The paint an `animate_replace` element had last frame, and the one it
/// replaced while that is still leaving.
struct Replacement {
    key: String,
    rect: Rect,
    commands: Vec<DrawCommand>,
    leaving: Option<(Rect, Vec<DrawCommand>)>,
    progress: ScalarTransition,
}

/// An `animate_height` slot: the share of its content's height it occupies,
/// and the last paint of that content for when it has left.
/// A value the tree moves for a `RollingNumber` or a part of a `UnitGrid`.
struct AnimatedScalar {
    value: ScalarTransition,
    seen: Instant,
}

/// A value the tree follows for a slot of `LevelBars`.
struct AnimatedSpring {
    value: ScalarSpring,
    seen: Instant,
}

/// The most slots a `LevelBars` draws, whatever width it is given.
const MAX_LEVEL_SLOTS: u16 = 512;

/// A slot fully open, in the units its spring moves in. A spring is at rest
/// once both what is left of the way and its speed are under a hundredth of
/// a unit: in hundredths of the height, that is far under a pixel.
const SLOT_OPEN: f32 = 100.0;

/// A slot opens and closes on a spring with no overshoot: toggled midway it
/// keeps its speed and turns back instead of starting again from rest.
fn slot_spring(value: f32, now: Instant) -> ScalarSpring {
    ScalarSpring::new(value, now).motion(slot_motion())
}

/// The spring an [`Element::extend_on_hover`] edge travels on: it overshoots
/// by a tenth, as something let go does, and is within a thousandth of its
/// way after three `move_ms`.
fn bounce_motion() -> crate::SpringMotion {
    const DAMPING: f32 = 0.6;
    let seconds = (f32::from(crate::tokens().motion.move_ms) * 3.0 / 1000.0).max(0.001);
    let omega = 1000_f32.ln() / (DAMPING * seconds);
    crate::SpringMotion { damping_ratio: DAMPING, stiffness: omega * omega }
}

fn slot_motion() -> crate::SpringMotion {
    crate::SpringMotion::settling_in(slot_settle())
}

/// The time in which a slot covers 99% of its way: one and a half moves.
pub(crate) fn slot_settle() -> Duration {
    Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3 / 2)
}

struct FlowHeight {
    fraction: ScalarSpring,
    natural: f32,
    rect: Rect,
    commands: Vec<DrawCommand>,
    seen: Instant,
}

struct PopoverExit {
    paint: PopoverSnapshot,
    opacity: ScalarTransition,
}

struct PopoverViewTransition {
    from: PopoverSnapshot,
    to_key: String,
    opacity: ScalarTransition,
    height: ScalarTransition,
}

#[derive(Clone, Debug, PartialEq)]
struct SelectableTextRun {
    start: usize,
    end: usize,
    text: String,
    rect: Rect,
    visible: Option<Rect>,
    font_size: f32,
    font_family: FontFamily,
    /// An inline picture: its text is selected whole or not at all.
    atomic: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct SelectableTextBlock {
    source: String,
    runs: Vec<SelectableTextRun>,
    region_layer: usize,
    /// Text laid out in a `TextLayout`, which a pointer is read back from.
    layout: Option<PlacedText>,
}

/// A text layout and where it was drawn.
#[derive(Clone, Debug)]
struct PlacedText {
    layout: crate::TextLayout,
    origin: Point,
}

impl PartialEq for PlacedText {
    fn eq(&self, other: &Self) -> bool {
        self.origin == other.origin
            && self.layout.text() == other.layout.text()
            && self.layout.style() == other.layout.style()
            && self.layout.size() == other.layout.size()
    }
}

impl SelectableTextBlock {
    fn contains(&self, point: Point) -> bool {
        self.runs
            .iter()
            .any(|run| run.visible.is_some_and(|rect| rect.contains(point)))
    }

    fn offset_at(
        &self,
        point: Point,
        measure: Option<&TextMeasurer>,
        metrics: FontMetrics,
        sans_metrics: FontMetrics,
    ) -> usize {
        if let Some(placed) = &self.layout {
            let at = Point::new(point.x - placed.origin.x, point.y - placed.origin.y);
            let byte = placed.layout.index_at(at);
            let offset = self.source.get(..byte).map_or(0, |before| before.chars().count());
            return nearest_source_grapheme_boundary(&self.source, offset);
        }
        let Some(row) = self.runs.iter().min_by(|left, right| {
            vertical_distance(left.rect, point.y).total_cmp(&vertical_distance(right.rect, point.y))
        }) else {
            return 0;
        };
        let run = self
            .runs
            .iter()
            .filter(|run| run.rect.y == row.rect.y)
            .min_by(|left, right| {
                horizontal_distance(left.rect, point.x)
                    .total_cmp(&horizontal_distance(right.rect, point.x))
            })
            .unwrap_or(row);
        let char_width = match run.font_family {
            FontFamily::Monospace => metrics,
            FontFamily::Sans
            | FontFamily::SansItalic
            | FontFamily::SansMedium
            | FontFamily::SansBold
            | FontFamily::SansBoldItalic => sans_metrics,
        }
        .at_size(run.font_size)
        .char_width;
        let x = (point.x - run.rect.x).max(0.0);
        if run.atomic {
            return if x * 2.0 < run.rect.width { run.start } else { run.end };
        }
        let offset = run.start
            + nearest_grapheme(&run.text, x, |text| {
                measured_width(measure, text, run.font_size, run.font_family, char_width)
            });
        nearest_source_grapheme_boundary(&self.source, offset)
    }
}

fn horizontal_distance(rect: Rect, x: f32) -> f32 {
    if x < rect.x {
        rect.x - x
    } else {
        (x - (rect.x + rect.width)).max(0.0)
    }
}

fn vertical_distance(rect: Rect, y: f32) -> f32 {
    if y < rect.y {
        rect.y - y
    } else {
        (y - (rect.y + rect.height)).max(0.0)
    }
}

fn nearest_source_grapheme_boundary(source: &str, char_offset: usize) -> usize {
    let mut previous = 0;
    for grapheme in source.graphemes(true) {
        let next = previous + grapheme.chars().count();
        if char_offset <= next {
            return if char_offset - previous <= next - char_offset {
                previous
            } else {
                next
            };
        }
        previous = next;
    }
    previous
}

#[derive(Clone, Debug)]
struct StaticSelection {
    block: usize,
    source: String,
    anchor: usize,
    cursor: usize,
    dragging: bool,
}

impl StaticSelection {
    fn range(&self) -> Option<(usize, usize)> {
        (self.anchor != self.cursor)
            .then_some((self.anchor.min(self.cursor), self.anchor.max(self.cursor)))
    }
}

/// How long a text area's scrollbar stays after its last scroll.
const TEXT_AREA_BAR_HOLD: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy, Debug, PartialEq)]
struct ScrollbarGeometry {
    rail: Rect,
    track: Rect,
    thumb: Rect,
    up_button: Option<Rect>,
    down_button: Option<Rect>,
    max_offset: f32,
    placement: crate::ScrollbarPlacement,
}

impl ScrollbarGeometry {
    fn new(
        viewport: Rect,
        max_offset: f32,
        offset: f32,
        appearance: crate::ScrollbarAppearance,
    ) -> Option<Self> {
        if viewport.width <= 0.0 || viewport.height <= 0.0 || max_offset <= 0.0 {
            return None;
        }
        let gutter = appearance.effective_rail_width(viewport.width);
        let rail = Rect::new(
            viewport.x + viewport.width - gutter,
            viewport.y,
            gutter,
            viewport.height,
        );
        // Very short viewports keep a useful thumb instead of squeezing it
        // between two buttons. The buttons never reduce content height.
        let button_height = if appearance.buttons && viewport.height >= gutter * 4.0 {
            gutter
        } else {
            0.0
        };
        let up_button =
            (button_height > 0.0).then_some(Rect::new(rail.x, rail.y, gutter, button_height));
        let down_button = (button_height > 0.0).then_some(Rect::new(
            rail.x,
            rail.y + rail.height - button_height,
            gutter,
            button_height,
        ));
        let track = Rect::new(
            rail.x,
            rail.y + button_height,
            gutter,
            (rail.height - button_height * 2.0).max(0.0),
        );
        let fraction = viewport.height / (viewport.height + max_offset);
        let minimum = (if appearance.min_thumb_length.is_finite() {
            appearance.min_thumb_length
        } else {
            crate::ScrollbarAppearance::default().min_thumb_length
        })
        .max(1.0)
        .min((track.height / 2.0).max(1.0))
        .min(track.height);
        let thumb_height = (track.height * fraction).clamp(minimum, track.height);
        let travel = track.height - thumb_height;
        let thumb_width = (if appearance.thumb_width.is_finite() {
            appearance.thumb_width
        } else {
            crate::ScrollbarAppearance::default().thumb_width
        })
        .max(1.0)
        .min((gutter - 2.0).max(1.0))
        .min(gutter);
        let thumb = Rect::new(
            track.x + (gutter - thumb_width) / 2.0,
            track.y + offset.clamp(0.0, max_offset) / max_offset * travel,
            thumb_width,
            thumb_height,
        );
        Some(Self {
            rail,
            track,
            thumb,
            up_button,
            down_button,
            max_offset,
            placement: appearance.placement,
        })
    }

    fn button_at(self, point: Point) -> Option<i8> {
        if self.up_button.is_some_and(|rect| rect.contains(point)) {
            Some(-1)
        } else if self.down_button.is_some_and(|rect| rect.contains(point)) {
            Some(1)
        } else {
            None
        }
    }

    fn offset_at(self, pointer_y: f32, grab_y: f32) -> f32 {
        let travel = self.track.height - self.thumb.height;
        if travel <= 0.0 {
            return 0.0;
        }
        ((pointer_y - self.track.y - grab_y) / travel).clamp(0.0, 1.0) * self.max_offset
    }
}

struct ScrollDrag {
    id: String,
    grab_y: f32,
}

struct ScrollButtonHold {
    id: String,
    direction: i8,
    repeat_at: Instant,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SliderGeometry {
    rect: Rect,
    steps: usize,
    fraction: f32,
    ultra_available: bool,
    /// Distance from each end of `rect` to the ends of the thumb's travel.
    inset: f32,
    /// The effort slider settles on a spring; a plain slider follows the host.
    animated: bool,
    /// The share of the height from the top, for a slider over an area.
    y: Option<f32>,
    /// A slider standing up: its fraction runs from the bottom to the top.
    vertical: bool,
}

/// What a slider reports: its fraction along a track, or both shares over an area.
fn slider_action(id: String, fraction: f32, y: Option<f32>, committed: bool) -> UiAction {
    match y {
        Some(y) => UiAction::AreaSliderChanged {
            id,
            x: fraction,
            y,
            committed,
        },
        None => UiAction::SliderChanged {
            id,
            fraction,
            committed,
        },
    }
}

impl SliderGeometry {
    /// The pointer's share of the height from the top, for an area slider.
    fn y_at(self, y: f32) -> Option<f32> {
        let travel = self.rect.height - self.inset * 2.0;
        self.y.map(|_| {
            if travel <= 0.0 {
                0.5
            } else {
                ((y - self.rect.y - self.inset) / travel).clamp(0.0, 1.0)
            }
        })
    }

    fn fraction_at(self, point: Point) -> f32 {
        let (along, start, length) = if self.vertical {
            // From the bottom: dragging up raises the value.
            (self.rect.y + self.rect.height - point.y, 0.0, self.rect.height)
        } else {
            (point.x, self.rect.x, self.rect.width)
        };
        let travel = length - self.inset * 2.0;
        if travel <= 0.0 {
            return 0.5;
        }
        let raw = ((along - start - self.inset) / travel).clamp(0.0, 1.0);
        if self.steps < 2 {
            return raw;
        }
        // The reference's magnetic stops attract the thumb continuously during
        // drag, while a release still chooses the nearest advertised level.
        let intervals = self.steps.saturating_sub(1).max(1) as f32;
        let scaled = raw * intervals;
        let nearest = scaled.round();
        let distance = ((scaled - nearest).abs() * 2.0).min(1.0);
        let influence = 0.9 * (1.0 - distance).powf(1.45);
        (scaled + (nearest - scaled) * influence) / intervals
    }

    fn snap(self, fraction: f32) -> f32 {
        if self.steps < 2 {
            return fraction.clamp(0.0, 1.0);
        }
        let intervals = self.steps.saturating_sub(1).max(1) as f32;
        (fraction.clamp(0.0, 1.0) * intervals).round() / intervals
    }
}

struct SliderDrag {
    id: String,
    geometry: SliderGeometry,
    fraction: f32,
    y: Option<f32>,
}

struct SliderMotion {
    phase: f32,
    last_sample: Instant,
    displayed_fraction: f32,
    target_fraction: Option<f32>,
    snap: Option<SliderSnap>,
    burst_started: Option<Instant>,
    ultra_from: f32,
    ultra_to: f32,
    ultra_started: Instant,
}

#[derive(Clone, Copy)]
struct SliderSnap {
    from: f32,
    to: f32,
    started: Instant,
    celebrate_ultra: bool,
}

fn slider_spring(progress: f32) -> f32 {
    // The selector reference uses this gently overdamped spring: one roughly
    // 5% overshoot and no second oscillation. Preserve its sampled curve so
    // rendering is deterministic across frame rates.
    const KEYS: &[(f32, f32)] = &[
        (0.0, 0.0),
        (0.034, 0.062),
        (0.075, 0.24),
        (0.125, 0.51),
        (0.179, 0.76),
        (0.224, 0.905),
        (0.272, 0.997),
        (0.324, 1.038),
        (0.374, 1.05),
        (0.427, 1.044),
        (0.492, 1.028),
        (0.569, 1.012),
        (0.657, 1.003),
        (0.76, 0.999),
        (1.0, 1.0),
    ];
    let progress = progress.clamp(0.0, 1.0);
    for pair in KEYS.windows(2) {
        let (start_t, start_value) = pair[0];
        let (end_t, end_value) = pair[1];
        if progress <= end_t {
            let fraction = (progress - start_t) / (end_t - start_t);
            return start_value + (end_value - start_value) * fraction;
        }
    }
    1.0
}

impl SliderMotion {
    fn new(now: Instant) -> Self {
        Self {
            phase: 0.0,
            last_sample: now,
            displayed_fraction: 0.0,
            target_fraction: None,
            snap: None,
            burst_started: None,
            ultra_from: 0.0,
            ultra_to: 0.0,
            ultra_started: now,
        }
    }

    fn animate_to(&mut self, to: f32, now: Instant, celebrate_ultra: bool) {
        let from = self.position_at(now);
        self.target_fraction = Some(to);
        self.burst_started = None;
        if (from - to).abs() < 0.0001 {
            self.snap = None;
            self.displayed_fraction = to;
            if celebrate_ultra {
                self.burst_started = Some(now);
            }
        } else {
            self.snap = Some(SliderSnap {
                from,
                to,
                started: now,
                celebrate_ultra,
            });
        }
    }

    fn position_at(&self, now: Instant) -> f32 {
        let Some(snap) = self.snap else {
            return self.displayed_fraction;
        };
        let progress = now.saturating_duration_since(snap.started).as_secs_f32()
            / SLIDER_SETTLE_DURATION.as_secs_f32();
        snap.from + (snap.to - snap.from) * slider_spring(progress)
    }

    fn sample(
        &mut self,
        now: Instant,
        target: f32,
        drag: Option<f32>,
        steps: usize,
        ultra_available: bool,
        reduced_motion: bool,
    ) -> (f32, f32, f32, Option<f32>) {
        if reduced_motion {
            self.last_sample = now;
            self.snap = None;
            self.burst_started = None;
            self.displayed_fraction = drag.unwrap_or(target);
            self.target_fraction = Some(target);
            self.ultra_to = if ultra_available && target >= 1.0 {
                1.0
            } else {
                0.0
            };
            self.ultra_from = self.ultra_to;
            return (self.phase, self.displayed_fraction, self.ultra_to, None);
        }
        let dragging = drag.is_some();
        if let Some(drag) = drag {
            self.snap = None;
            self.displayed_fraction = drag;
            self.target_fraction = Some(target);
        } else if let Some(previous) = self.target_fraction {
            if (target - previous).abs() > 0.0001 {
                self.animate_to(
                    target,
                    now,
                    ultra_available && previous < 1.0 && target >= 1.0,
                );
            }
        } else {
            self.target_fraction = Some(target);
            self.displayed_fraction = target;
            self.ultra_to = if ultra_available && target >= 1.0 {
                1.0
            } else {
                0.0
            };
            self.ultra_from = self.ultra_to;
        }

        if let Some(snap) = self.snap {
            if now.saturating_duration_since(snap.started) >= SLIDER_SETTLE_DURATION {
                self.displayed_fraction = snap.to;
                self.snap = None;
                if snap.celebrate_ultra && ultra_available {
                    self.burst_started = Some(snap.started + SLIDER_SETTLE_DURATION);
                }
            } else {
                self.displayed_fraction = self.position_at(now);
            }
        }

        let ultra_active = ultra_available
            && if dragging {
                (self.displayed_fraction * steps.saturating_sub(1).max(1) as f32).round()
                    >= steps.saturating_sub(1).max(1) as f32
            } else {
                target >= 1.0
            };
        let next_ultra = if ultra_active { 1.0 } else { 0.0 };
        if next_ultra != self.ultra_to {
            let elapsed = now
                .saturating_duration_since(self.ultra_started)
                .as_secs_f32()
                / SLIDER_ULTRA_FADE_DURATION.as_secs_f32();
            let eased = 1.0 - (1.0 - elapsed.clamp(0.0, 1.0)).powi(3);
            self.ultra_from += (self.ultra_to - self.ultra_from) * eased;
            self.ultra_to = next_ultra;
            self.ultra_started = now;
        }
        let ultra_elapsed = now
            .saturating_duration_since(self.ultra_started)
            .as_secs_f32()
            / SLIDER_ULTRA_FADE_DURATION.as_secs_f32();
        let eased = 1.0 - (1.0 - ultra_elapsed.clamp(0.0, 1.0)).powi(3);
        let ultra_mix = self.ultra_from + (self.ultra_to - self.ultra_from) * eased;

        let burst_age = self.burst_started.and_then(|start| {
            let age = now.saturating_duration_since(start).as_secs_f32();
            (age < SLIDER_BURST_DURATION.as_secs_f32()).then_some(age)
        });
        if burst_age.is_none() {
            self.burst_started = None;
        }

        let settling = self.snap.is_some();
        let elapsed = now.saturating_duration_since(self.last_sample);
        // The resting field advances at roughly 30 fps; pointer motion keeps
        // every input frame. Repaints are scheduled by the host separately.
        if !dragging && !settling && elapsed < Duration::from_millis(30) {
            return (self.phase, self.displayed_fraction, ultra_mix, burst_age);
        }
        let seconds = elapsed.as_secs_f32().min(0.032);
        self.phase = (self.phase + seconds).rem_euclid(1000.0);
        self.last_sample = now;
        (self.phase, self.displayed_fraction, ultra_mix, burst_age)
    }

    fn is_fast(&self, now: Instant) -> bool {
        self.snap.is_some_and(|snap| {
            now.saturating_duration_since(snap.started) < SLIDER_SETTLE_DURATION
        }) || self
            .burst_started
            .is_some_and(|start| now.saturating_duration_since(start) < SLIDER_BURST_DURATION)
            || (self.ultra_from != self.ultra_to
                && now.saturating_duration_since(self.ultra_started) < SLIDER_ULTRA_FADE_DURATION)
    }
}

impl Frame {
    /// A copy of the selectable text blocks laid out in this frame, for
    /// diagnostics and tests.
    pub fn text_layouts(&self) -> Vec<TextLayoutBlock> {
        self.text_blocks
            .iter()
            .map(|block| TextLayoutBlock {
                source: block.source.clone(),
                runs: block
                    .runs
                    .iter()
                    .map(|run| TextLayoutRun {
                        text: run.text.clone(),
                        rect: run.rect,
                        visible: run.visible,
                    })
                    .collect(),
            })
            .collect()
    }

    /// Returns the topmost interactive element at a point.
    pub fn hit_test(&self, point: Point) -> Option<&str> {
        self.hit_region(point).map(|region| region.id.as_str())
    }

    /// The topmost region at a point, with its rectangle and kind. For a press
    /// use `press_region`, which looks through passive hover targets.
    pub fn hit_region(&self, point: Point) -> Option<&HitRegion> {
        self.region_at(point, false)
    }

    /// The region a press at `point` belongs to. A passive hover target, such
    /// as the anchor of a hint on an icon, shows its hint but lets the press
    /// reach the control it sits on.
    pub fn press_region(&self, point: Point) -> Option<&HitRegion> {
        self.region_at(point, true)
    }

    fn region_at(&self, point: Point, pressing: bool) -> Option<&HitRegion> {
        for (index, region) in self.regions.iter().enumerate().rev() {
            if !region.rect.contains(point) {
                continue;
            }
            if pressing
                && region.kind == HitKind::Hover
                && self.regions[..index].iter().any(|underneath| {
                    underneath.rect.contains(point) && underneath.kind != HitKind::Hover
                })
            {
                continue;
            }
            if region.kind == HitKind::Scrollbar
                && self.scrollbars.get(&region.id).is_some_and(|bar| {
                    bar.placement == crate::ScrollbarPlacement::Overlay
                        && !bar.thumb.contains(point)
                        && bar.button_at(point).is_none()
                })
                && self.regions[..index].iter().any(|underneath| {
                    underneath.rect.contains(point)
                        && matches!(
                            underneath.kind,
                            HitKind::Button | HitKind::TextInput | HitKind::TextArea
                        )
                })
            {
                // A transparent overlay track should not swallow controls at
                // the content edge. The visible thumb still wins for dragging.
                continue;
            }
            return Some(region);
        }
        None
    }
}

/// The keys a host can forward in [`UiEvent::Key`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    /// Page Up: without `ctrl`, scrolls up a page behind the focus or under the
    /// pointer.
    PageUp,
    /// Page Down: without `ctrl`, scrolls down a page behind the focus or under
    /// the pointer.
    PageDown,
    /// Escape: dismisses the open popover or cancels a drag in progress.
    Escape,
    /// Tab: outside an editable code editor, moves focus to the next control,
    /// or to the previous one with `shift`.
    Tab,
    /// Left arrow: moves the caret, or steps a focused slider, tab strip, menu
    /// or coach mark.
    ArrowLeft,
    /// Right arrow: moves the caret, or steps a focused slider, tab strip, menu
    /// or coach mark.
    ArrowRight,
    /// Up arrow: moves the caret up a line in a text area, or steps a tab strip
    /// or menu.
    ArrowUp,
    /// Down arrow: moves the caret down a line in a text area, or steps a tab
    /// strip or menu.
    ArrowDown,
    /// Home: start of the line, or of the whole text with `ctrl` or in a
    /// single-line field. Sliders, tab strips and menus read it too.
    Home,
    /// End: end of the line, or of the whole text with `ctrl` or in a
    /// single-line field. Sliders, tab strips and menus read it too.
    End,
    /// Backspace: deletes backward in the focused text field.
    Backspace,
    /// Delete: deletes forward in the focused text field.
    Delete,
    /// Enter: activates the focused button, submits a text input, or adds a
    /// line to a text area (which Ctrl+Enter submits, and Enter alone when it
    /// has `enter_submits`).
    Enter,
    /// Space: activates the focused button. Text fields take the character from
    /// `UiEvent::Text`, not from this key.
    Space,
    /// The A key: with `ctrl`, selects all of the focused text field.
    A,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    B,
    /// The C key: with `ctrl`, reports `UiAction::CopyRequested` for the
    /// selection.
    C,
    /// The D key: with `ctrl` in a code editor, selects the next occurrence, or
    /// every one with `shift`.
    D,
    /// Read only as the shortcut of an item in an open context menu.
    E,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    F,
    /// Read only as the shortcut of an item in an open context menu.
    G,
    /// Read only as the shortcut of an item in an open context menu.
    H,
    /// Read only as the shortcut of an item in an open context menu.
    I,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    J,
    /// Read only as the shortcut of an item in an open context menu.
    K,
    /// Read only as the shortcut of an item in an open context menu.
    L,
    /// Read only as the shortcut of an item in an open context menu.
    M,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    N,
    /// Read only as the shortcut of an item in an open context menu.
    O,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    P,
    /// Read only as the shortcut of an item in an open context menu.
    Q,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    R,
    /// The S key: with `ctrl` in a code editor, reports
    /// `UiAction::EditorSaveRequested`.
    S,
    /// Read only as the shortcut of an item in an open context menu.
    T,
    /// With `ctrl`, a Vim motion in a code editor in Vim mode.
    U,
    /// The V key. Pasted text arrives as `UiEvent::Paste`, not as this key.
    V,
    /// Read only as the shortcut of an item in an open context menu.
    W,
    /// The X key: with `ctrl`, reports `UiAction::CopyRequested` for the
    /// selection and deletes it.
    X,
    /// The Y key: with `ctrl` in an editable code editor, redoes.
    Y,
    /// The Z key: with `ctrl` in an editable code editor, undoes, or redoes
    /// with `shift`.
    Z,
    /// Read only as the shortcut of an item in an open context menu.
    F2,
    /// Read only as the shortcut of an item in an open context menu.
    F5,
    /// Read only as the shortcut of an item in an open context menu.
    F9,
    /// Read only as the shortcut of an item in an open context menu.
    F10,
    /// Read only as the shortcut of an item in an open context menu.
    F11,
    /// Read only as the shortcut of an item in an open context menu.
    F12,
}

/// Input a host forwards to `UiTree::handle_event`. Positions are in
/// logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    /// Open text or host-defined actions without changing the selection.
    ContextMenu {
        /// Where the secondary button was pressed; the menu opens there.
        position: Point,
    },
    /// Hosts map Shift+F10 or their platform's context-menu key here.
    ContextMenuKeyboard,
    /// Clipboard replies remain bound to the editor which requested them.
    PasteTo {
        /// The control that reported `UiAction::PasteRequested`. The text is
        /// dropped unless that control is still focused and laid out.
        id: String,
        /// The clipboard text to insert there.
        text: String,
    },
    /// Deliver an asynchronous reply only to its captured editing state.
    /// A rejected target never falls back to whichever control is focused now.
    PasteCaptured {
        /// The destination captured before the host authorized its request.
        target: PasteTarget,
        /// Clipboard text, subject to the control's ordinary paste policy.
        text: String,
    },
    /// The text an input method is composing, replacing what it sent before.
    /// Only a code editor shows it.
    ImePreedit(String),
    /// An input method committed this text. A code editor ends its composition
    /// with it; other text fields take it as `Text`.
    ImeCommit(String),
    /// The composition ended without a commit: a code editor drops what was
    /// being composed.
    ImeCancelled,
    /// An input method asks the focused code editor to delete text around the
    /// caret.
    ImeDeleteSurrounding {
        /// Characters to delete before the caret.
        before_chars: usize,
        /// Characters to delete after the caret.
        after_chars: usize,
    },
    /// Additive editor selection gesture; ordinary controls treat it as a click.
    PointerDownModified {
        /// Where the primary button was pressed.
        position: Point,
        /// True to extend a code editor's selection to `position`.
        shift: bool,
    },
    /// Native editor pointer gestures; other controls receive an ordinary click.
    PointerDownEditor {
        /// Where the primary button was pressed.
        position: Point,
        /// Extends the selection to `position`; with `alt`, selects a
        /// rectangle.
        shift: bool,
        /// Adds a caret at `position`; with `shift`, selects a rectangle.
        alt: bool,
        /// Whether Ctrl or Command was held. The tree does not read it.
        ctrl: bool,
        /// 1 for a single click, 2 for a double click, 3 for a triple click.
        click_count: u8,
    },
    /// Pixel wheel deltas; hosts retain the legacy wheel event outside editors.
    Scroll2D {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Horizontal travel in logical pixels; a positive value moves toward
        /// the left edge of the content.
        delta_x: f32,
        /// Vertical travel in logical pixels; a positive value moves toward the
        /// top of the content.
        delta_y: f32,
        /// True for pixel-precise input, handled as `ScrollDirect`; false is
        /// eased as `Scroll` is.
        direct: bool,
        /// True while Shift is held: with a `delta_x` of 0, a code editor or
        /// diff scrolls sideways by `delta_y`.
        shift: bool,
    },
    /// Clipboard text for the focused control. A text field inserts it as it
    /// does `Text`.
    Paste(String),
    /// Files dragged from another application are over the window.
    FilesHovered {
        /// The pointer's position, as far as the host knows it.
        position: Point,
    },
    /// Files dragged from another application were dropped on the window.
    FilesDropped {
        /// Where they were dropped.
        position: Point,
        /// Paths of the dropped files.
        paths: Vec<std::path::PathBuf>,
    },
    /// The pointer moved to this position: updates what is hovered and carries
    /// on a drag, a text selection or a scrollbar drag.
    PointerMove(Point),
    /// The pointer left the window: clears the hover and the press and ends any
    /// drag.
    PointerLeave,
    /// The primary button was pressed at this position, with no modifier held.
    PointerDown(Point),
    /// The primary button was released at this position. A release on the
    /// control that was pressed reports `UiAction::Clicked`.
    PointerUp(Point),
    /// A wheel step, converted to logical pixels by the host (`limn-egui`
    /// moves a notch 120 px, as a browser does). The scrolling area under the
    /// pointer eases to its new offset.
    Scroll {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Travel in logical pixels; a positive value moves toward the top of
        /// the content.
        delta_y: f32,
    },
    /// Pixel-precise input, such as a touchpad, already supplies a continuous
    /// stream. Keep it attached to the gesture instead of adding wheel easing.
    ScrollDirect {
        /// Where the pointer is; what scrolls is under it.
        position: Point,
        /// Travel in logical pixels; a positive value moves toward the top of
        /// the content.
        delta_y: f32,
    },
    /// Typed characters, inserted into the focused text field or code editor.
    Text(String),
    /// A key was pressed.
    Key {
        /// The key that was pressed.
        key: Key,
        /// True while Shift is held.
        shift: bool,
        /// True while Ctrl is held. The egui backend sets it for Command too.
        ctrl: bool,
    },
}

/// What `UiTree::handle_event` reports for the host to act on.
#[derive(Clone, Debug, PartialEq)]
pub enum UiAction {
    /// An edit changed the document of a code editor.
    EditorChanged {
        /// ID of the code editor.
        id: String,
        /// The buffer's document ID.
        document_id: u64,
        /// The buffer's revision after the edit.
        revision: u64,
    },
    /// Ctrl+S was pressed in a code editor. Saving is the host's work.
    EditorSaveRequested {
        /// ID of the code editor.
        id: String,
        /// The buffer's document ID.
        document_id: u64,
        /// The buffer's revision when the key was pressed.
        revision: u64,
    },
    /// A code editor could not carry out a command, such as an edit, or a copy
    /// over the clipboard limit.
    EditorFailed {
        /// ID of the code editor.
        id: String,
        /// The reason, as text.
        error: String,
    },
    /// A control was activated: pressed and released with the pointer, or Enter
    /// or Space while it had focus. Escape reports it for an open popover's
    /// dismiss ID, and a host's context menu for the chosen item's action ID.
    Clicked {
        /// The control's ID, or the action ID it was given to report instead.
        id: String,
    },
    /// Continuous preview on pointer down/move; snapped selection on release.
    SliderChanged {
        /// ID of the slider.
        id: String,
        /// Position along the track, from 0 to 1. A committed one is snapped to
        /// a step.
        fraction: f32,
        /// False while the pointer drags. True on release, when the pointer
        /// leaves the window, and for a step from the keyboard.
        committed: bool,
    },
    /// An area slider moved: previewed while the pointer is down, committed
    /// on release and from the keyboard. See [`Element::area_slider`].
    AreaSliderChanged {
        /// ID of the slider.
        id: String,
        /// Share of the width from the left, from 0 to 1.
        x: f32,
        /// Share of the height from the top, from 0 to 1.
        y: f32,
        /// False while the pointer drags. True on release, when the pointer
        /// leaves the window, and for a step from the keyboard.
        committed: bool,
    },
    /// The text of a text input or text area changed.
    InputChanged {
        /// ID of the field.
        id: String,
        /// The field's whole text after the change.
        value: String,
    },
    /// Enter was pressed in a text input, or Ctrl+Enter in a text area (Enter
    /// alone in one with `enter_submits`).
    Submitted {
        /// ID of the field.
        id: String,
        /// The field's whole text.
        value: String,
    },
    /// The host should put `text` on the clipboard, for example after Ctrl+C or
    /// Ctrl+X on a selection.
    CopyRequested {
        /// The selected text. Ctrl+C and Ctrl+X never report a password field's
        /// selection.
        text: String,
    },
    /// The paste item of a text context menu was chosen. The host reads the
    /// clipboard and answers with `UiEvent::PasteTo`.
    PasteRequested {
        /// The text field or code editor to paste into; `PasteTo` carries it
        /// back.
        id: String,
    },
    /// The pointer left the click tolerance of a pressed `draggable` element.
    /// No `Clicked` follows this press.
    DragStarted {
        /// ID of the dragged element.
        id: String,
        /// Where the press began, not where the pointer is now.
        position: Point,
    },
    /// `over` is the topmost `drop_target` under the pointer, never the source.
    DragMoved {
        /// ID of the dragged element.
        id: String,
        /// Where the pointer is.
        position: Point,
        /// The drop target under the pointer; `None` over none.
        over: Option<String>,
    },
    /// The pointer was released during a drag.
    Dropped {
        /// ID of the dragged element.
        id: String,
        /// Where the pointer was released.
        position: Point,
        /// The topmost `drop_target` there other than the source; `None` when
        /// it was dropped on none.
        target: Option<String>,
    },
    /// Escape or the pointer leaving the window ended the drag.
    DragCancelled {
        /// ID of the element that was being dragged.
        id: String,
    },
    /// `over` and `target` are the topmost `drop_target` under the pointer.
    FilesHovered {
        /// The position the host sent.
        position: Point,
        /// The drop target at `position`; `None` over none.
        over: Option<String>,
    },
    /// Files from another application were dropped: the answer to
    /// `UiEvent::FilesDropped`.
    FilesDropped {
        /// The position the host sent.
        position: Point,
        /// The drop target at `position`; `None` when they were dropped on
        /// none.
        target: Option<String>,
        /// The paths the host sent, unchanged.
        paths: Vec<std::path::PathBuf>,
    },
}

/// Pointer travel that turns a press into a drag instead of a click.
const DRAG_THRESHOLD: f32 = 4.0;

struct PointerDrag {
    id: String,
    origin: Point,
    active: bool,
    /// The source as painted when the drag began, before any translation.
    ghost: Vec<DrawCommand>,
}

/// A field's value as limn lays it out, in a tree told to
/// (`UiTree::set_text_layout`), and where it was last painted.
#[derive(Clone)]
struct FieldLayout {
    style: crate::TextStyle,
    /// The width a text area breaks its lines at; a single line breaks none.
    width: Option<f32>,
    /// The value as shown, a password masked: never the placeholder.
    layout: crate::TextLayout,
    /// Its top-left corner in the tree, scrolled.
    origin: Point,
}

struct InputState {
    paste_epoch: Arc<()>,
    paste_kind: Option<HitKind>,
    value: String,
    cursor: usize,
    anchor: Option<usize>,
    view_start: usize,
    content_x: f32,
    content_y: f32,
    char_width: f32,
    content_width: f32,
    font_size: f32,
    font_family: FontFamily,
    measurer: Option<Arc<TextMeasurer>>,
    line_height: f32,
    visible_lines: usize,
    scroll_line: usize,
    /// The scroll position last painted and when it last moved, for the
    /// scrollbar that shows while a text area scrolls.
    painted_scroll_line: usize,
    scrolled_at: Option<Instant>,
    keep_cursor_visible: bool,
    desired_column: Option<usize>,
    /// Laid out by limn: what it painted, and where Up and Down aim.
    laid_out: Option<FieldLayout>,
    desired_x: Option<f32>,
    /// How far a single line laid out by limn is scrolled to show the caret.
    scroll_x: f32,
    preserve_line_endings: bool,
    /// Enter submits and Shift+Enter breaks the line.
    enter_submits: bool,
    password: bool,
}

impl InputState {
    fn new(value: String) -> Self {
        let cursor = value.chars().count();
        Self {
            paste_epoch: Arc::new(()),
            paste_kind: None,
            value,
            cursor,
            anchor: None,
            view_start: 0,
            content_x: 0.0,
            content_y: 0.0,
            char_width: FontMetrics::default().char_width,
            content_width: 8.0,
            font_size: DEFAULT_FONT_SIZE,
            font_family: FontFamily::Monospace,
            measurer: None,
            line_height: FontMetrics::default().line_height,
            visible_lines: 1,
            scroll_line: 0,
            painted_scroll_line: 0,
            scrolled_at: None,
            keep_cursor_visible: true,
            desired_column: None,
            laid_out: None,
            desired_x: None,
            scroll_x: 0.0,
            preserve_line_endings: false,
            enter_submits: false,
            password: false,
        }
    }

    fn selected_range(&self) -> Option<(usize, usize)> {
        self.anchor
            .filter(|anchor| *anchor != self.cursor)
            .map(|anchor| (anchor.min(self.cursor), anchor.max(self.cursor)))
    }

    fn set_selection(&mut self, cursor: usize, anchor: Option<usize>) {
        if self.cursor != cursor || self.anchor != anchor {
            self.paste_epoch = Arc::new(());
            self.cursor = cursor;
            self.anchor = anchor;
        }
    }

    fn observe_paste_kind(&mut self, kind: HitKind) {
        if self.paste_kind != Some(kind) {
            self.paste_epoch = Arc::new(());
            self.paste_kind = Some(kind);
        }
    }

    fn width(&self, value: &str) -> f32 {
        let masked;
        let value = if self.password {
            masked = "•".repeat(value.chars().count());
            masked.as_str()
        } else {
            value
        };
        measured_width(
            self.measurer.as_deref(),
            value,
            self.font_size,
            self.font_family,
            self.char_width,
        )
    }

    /// The value as it is shown: a password as one dot a character.
    fn shown(&self) -> Cow<'_, str> {
        if self.password {
            Cow::Owned("•".repeat(self.value.chars().count()))
        } else {
            Cow::Borrowed(&self.value)
        }
    }

    /// The value laid out as last painted, laid out again when an edit has
    /// changed it since, and where it was painted. `None` while the tree
    /// measures text through the host.
    fn layout(&self) -> Option<(crate::TextLayout, Point)> {
        let field = self.laid_out.as_ref()?;
        let shown = self.shown();
        if field.layout.text() == shown {
            return Some((field.layout.clone(), field.origin));
        }
        let layout = crate::TextLayout::new(shown.into_owned(), field.style);
        Some((field.width.map_or_else(|| layout.clone(), |width| layout.wrapped(width)), field.origin))
    }

    /// The line of `layout` the caret is on, and the caret.
    fn caret_line(&self, layout: &crate::TextLayout) -> (usize, Rect) {
        let caret = layout.caret(byte_index(&self.shown(), self.cursor));
        let row = layout.lines().take_while(|line| line.rect.y <= caret.y + caret.height / 2.0).count();
        (row.saturating_sub(1), caret)
    }

    fn visual_lines(&self) -> Vec<VisualLine> {
        visual_lines_measured_endings(
            &self.value,
            self.content_width,
            self.preserve_line_endings,
            |value| self.width(value),
        )
    }

    fn move_cursor(&mut self, next: usize, shift: bool) {
        let anchor = shift.then_some(self.anchor.unwrap_or(self.cursor));
        self.set_selection(next.min(self.value.chars().count()), anchor);
        self.desired_column = None;
        self.desired_x = None;
        self.keep_cursor_visible = true;
    }

    /// The character before the byte at `index` of the value as shown.
    fn character_at(&self, index: usize) -> usize {
        let shown = self.shown();
        shown[..index.min(shown.len())].chars().count()
    }

    fn move_vertical(&mut self, direction: i32, shift: bool) {
        if let Some((layout, _)) = self.layout() {
            let (row, caret) = self.caret_line(&layout);
            let goal = self.desired_x.unwrap_or(caret.x);
            let lines: Vec<_> = layout.lines().collect();
            let line = &lines[(row as i64 + i64::from(direction)).clamp(0, lines.len() as i64 - 1) as usize];
            let index = layout.index_at(Point::new(goal, line.rect.y + line.rect.height / 2.0));
            self.move_cursor(self.character_at(index), shift);
            self.desired_x = Some(goal);
            return;
        }
        let lines = self.visual_lines();
        let (row, column) = visual_cursor(&lines, self.cursor);
        let desired = self.desired_column.unwrap_or(column);
        let next_row =
            (row as i64 + direction as i64).clamp(0, lines.len().saturating_sub(1) as i64) as usize;
        let next = lines[next_row].start + desired.min(lines[next_row].end - lines[next_row].start);
        self.move_cursor(next, shift);
        self.desired_column = Some(desired);
    }

    fn move_line_edge(&mut self, end: bool, shift: bool) {
        if let Some((layout, _)) = self.layout() {
            let (row, _) = self.caret_line(&layout);
            let lines: Vec<_> = layout.lines().collect();
            let line = &lines[row];
            let index = if end {
                let shown = self.shown();
                let text = &shown[line.range.clone()];
                let kept = text.trim_end_matches(['\n', '\r']);
                if kept.len() < text.len() {
                    // Before the break that ends the line.
                    line.range.start + kept.len()
                } else if row + 1 < lines.len() {
                    // Before the spaces a wrapped line hangs past its end.
                    line.range.start + kept.trim_end().len()
                } else {
                    line.range.end
                }
            } else {
                line.range.start
            };
            self.move_cursor(self.character_at(index), shift);
            return;
        }
        let lines = self.visual_lines();
        let (row, _) = visual_cursor(&lines, self.cursor);
        let next = if end {
            lines[row].end
        } else {
            lines[row].start
        };
        self.move_cursor(next, shift);
    }

    fn place_cursor_at(&mut self, point: Point) {
        if let Some((layout, origin)) = self.layout() {
            let index = layout.index_at(Point::new(point.x - origin.x, point.y - origin.y));
            self.move_cursor(self.character_at(index), false);
            return;
        }
        let lines = self.visual_lines();
        let row = ((point.y - self.content_y).max(0.0) / self.line_height).floor() as usize;
        let row = (self.scroll_line + row).min(lines.len() - 1);
        let column = nearest_char(
            &lines[row].text,
            (point.x - self.content_x).max(0.0),
            |value| self.width(value),
        );
        let next = lines[row].start + column.min(lines[row].end - lines[row].start);
        self.move_cursor(next, false);
    }

    fn scroll_lines(&mut self, delta_y: f32) {
        if delta_y == 0.0 {
            return;
        }
        let lines = self.layout().map_or_else(|| self.visual_lines().len(), |(layout, _)| layout.lines().count());
        let maximum = lines.saturating_sub(self.visible_lines);
        let steps = (delta_y.abs() / self.line_height).ceil().max(1.0) as usize;
        self.scroll_line = if delta_y > 0.0 {
            self.scroll_line.saturating_sub(steps)
        } else {
            self.scroll_line.saturating_add(steps).min(maximum)
        };
        self.keep_cursor_visible = false;
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selected_range() else {
            return false;
        };
        let start_byte = byte_index(&self.value, start);
        let end_byte = byte_index(&self.value, end);
        self.value.replace_range(start_byte..end_byte, "");
        self.paste_epoch = Arc::new(());
        self.cursor = start;
        self.anchor = None;
        self.keep_cursor_visible = true;
        true
    }

    fn insert(&mut self, text: &str, multiline: bool) -> bool {
        let sanitized = if multiline && self.preserve_line_endings {
            text.to_owned()
        } else if multiline {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.replace(['\r', '\n'], "")
        };
        if sanitized.is_empty() {
            return false;
        }
        self.delete_selection();
        // Ctrl+A on an empty field (or Shift+Arrow at its boundary) leaves
        // a zero-length selection. Typing must collapse that anchor too,
        // otherwise the next character replaces the one just inserted.
        self.anchor = None;
        let byte = byte_index(&self.value, self.cursor);
        self.value.insert_str(byte, &sanitized);
        self.paste_epoch = Arc::new(());
        self.cursor += sanitized.chars().count();
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }

    fn backspace(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        let end = byte_index(&self.value, self.cursor);
        let start = byte_index(&self.value, self.cursor - 1);
        self.value.replace_range(start..end, "");
        self.paste_epoch = Arc::new(());
        self.cursor -= 1;
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }

    fn delete_forward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor >= self.value.chars().count() {
            return false;
        }
        let start = byte_index(&self.value, self.cursor);
        let end = byte_index(&self.value, self.cursor + 1);
        self.value.replace_range(start..end, "");
        self.paste_epoch = Arc::new(());
        self.keep_cursor_visible = true;
        self.desired_column = None;
        self.desired_x = None;
        true
    }
}

/// The fill under selected text in a field.
const INPUT_SELECTION: Color = Color::rgba(75, 130, 210, 130);

/// A text area's value as spans in its highlights' inks and slants. Where
/// highlights overlap, the later one wins, as it painted over the earlier.
fn highlighted_spans(value: &str, highlights: &[crate::TextHighlight], style: Style) -> Vec<TextSpan> {
    let count = value.chars().count();
    let mut cuts: Vec<usize> = highlights
        .iter()
        .flat_map(|highlight| [highlight.range.start.min(count), highlight.range.end.min(count)])
        .chain([0, count])
        .collect();
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .map(|cut| {
            let mark = highlights.iter().rev().find(|highlight| highlight.range.start <= cut[0] && cut[1] <= highlight.range.end);
            let family = if mark.is_some_and(|highlight| highlight.italic) { FontFamily::SansItalic } else { style.font_family };
            TextSpan::new(char_range(value, cut[0], cut[1]), mark.map_or(style.foreground, |highlight| highlight.color), style.font_size, family)
        })
        .collect()
}

fn byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map_or(value.len(), |(index, _)| index)
}

fn char_range(value: &str, start: usize, end: usize) -> &str {
    &value[byte_index(value, start)..byte_index(value, end)]
}

fn fitting_chars(value: &str, max_width: f32, mut measure: impl FnMut(&str) -> f32) -> usize {
    let boundaries = value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .collect::<Vec<_>>();
    let mut lo = 0;
    let mut hi = boundaries.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if measure(&value[..boundaries[mid]]) <= max_width {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.saturating_sub(1)
}

#[derive(Clone, Debug)]
struct VisualLine {
    start: usize,
    end: usize,
    text: String,
}

/// The text editor shares the same measured line breaks as painted labels.
/// Indices remain Unicode scalar positions because input events use char IDs.
fn visual_lines_measured(
    value: &str,
    width: f32,
    measure: impl FnMut(&str) -> f32,
) -> Vec<VisualLine> {
    visual_lines_measured_endings(value, width, false, measure)
}

fn visual_lines_measured_endings(
    value: &str,
    width: f32,
    preserve_line_endings: bool,
    mut measure: impl FnMut(&str) -> f32,
) -> Vec<VisualLine> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut byte_start = 0usize;
    loop {
        let remainder = &value[byte_start..];
        let separator = remainder
            .bytes()
            .position(|byte| byte == b'\n' || (preserve_line_endings && byte == b'\r'));
        let paragraph = separator.map_or(remainder, |end| &remainder[..end]);
        for text in wrap_measured(paragraph, width, &mut measure) {
            let count = text.chars().count();
            lines.push(VisualLine {
                start,
                end: start + count,
                text: text.to_owned(),
            });
            start += count;
        }
        let Some(separator) = separator else {
            break;
        };
        let separator_length =
            if preserve_line_endings && remainder[separator..].starts_with("\r\n") {
                2
            } else {
                1
            };
        start += separator_length;
        byte_start += separator + separator_length;
    }
    lines
}

fn nearest_char(value: &str, x: f32, mut measure: impl FnMut(&str) -> f32) -> usize {
    if x <= 0.0 {
        return 0;
    }
    let boundaries = value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .collect::<Vec<_>>();
    let mut lo = 0;
    let mut hi = boundaries.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if measure(&value[..boundaries[mid]]) < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let after = lo.min(boundaries.len() - 1);
    if after == 0 {
        return 0;
    }
    let before = after - 1;
    let left = measure(&value[..boundaries[before]]);
    let right = measure(&value[..boundaries[after]]);
    if x - left <= right - x {
        before
    } else {
        after
    }
}

fn nearest_grapheme(value: &str, x: f32, mut measure: impl FnMut(&str) -> f32) -> usize {
    if x <= 0.0 {
        return 0;
    }
    let boundaries = std::iter::once(0)
        .chain(
            value
                .grapheme_indices(true)
                .map(|(index, part)| index + part.len()),
        )
        .collect::<Vec<_>>();
    let mut lo = 0;
    let mut hi = boundaries.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if measure(&value[..boundaries[mid]]) < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    let after = lo.min(boundaries.len() - 1);
    if after == 0 {
        return 0;
    }
    let before = after - 1;
    let left = measure(&value[..boundaries[before]]);
    let right = measure(&value[..boundaries[after]]);
    let boundary = if x - left <= right - x {
        boundaries[before]
    } else {
        boundaries[after]
    };
    value[..boundary].chars().count()
}

fn visual_cursor(lines: &[VisualLine], cursor: usize) -> (usize, usize) {
    for (row, line) in lines.iter().enumerate() {
        let wraps_here = lines.get(row + 1).is_some_and(|next| next.start == cursor);
        if cursor < line.end || (cursor == line.end && !wraps_here) {
            return (row, cursor.saturating_sub(line.start));
        }
        if lines.get(row + 1).is_some_and(|next| cursor < next.start) {
            // CRLF occupies two original scalar positions but one visual
            // break. A cursor between its bytes stays at the preceding edge.
            return (row, line.end - line.start);
        }
    }
    let row = lines.len() - 1;
    (
        row,
        cursor
            .saturating_sub(lines[row].start)
            .min(lines[row].end - lines[row].start),
    )
}

/// The retained state and last laid-out frame of an application surface.
pub struct UiTree {
    paste_epoch: Arc<()>,
    paste_layout_pending: bool,
    root: Element,
    metrics: FontMetrics,
    sans_metrics: FontMetrics,
    text_measurer: Option<Arc<TextMeasurer>>,
    text_height_measurer: Option<Arc<TextHeightMeasurer>>,
    width_cache: RefCell<TextWidthCache>,
    formatted_cache: RefCell<FormattedTextCache>,
    layout_cache: RefCell<TextLayoutCache>,
    /// Whether `Element::text` is laid out in a `TextLayout`; see
    /// [`UiTree::set_text_layout`].
    text_layout: bool,
    measurement_cache: RefCell<MeasurementCache>,
    horizontal_scrolls: HashMap<String, HorizontalScrollState>,
    frame: Frame,
    inputs: HashMap<String, InputState>,
    code_editors: HashMap<String, crate::code_editor::EditorView>,
    code_diffs: HashMap<String, crate::code_editor::DiffView>,
    code_view_last_seen: HashMap<String, u64>,
    code_view_epoch: u64,
    scroll_offsets: HashMap<String, f32>,
    scroll_transitions: HashMap<String, ScalarTransition>,
    scrollbar_states: HashMap<String, ScrollbarState>,
    scroll_to_end: HashSet<String>,
    scroll_to_index: HashMap<String, usize>,
    /// What each measured list keeps between layouts, by its ID.
    measured_lists: HashMap<String, MeasuredListState>,
    scroll_drag: Option<ScrollDrag>,
    /// The press in a `drag_to_pan` viewport, until the pointer is let go.
    horizontal_pan: Option<HorizontalPan>,
    scroll_button_hold: Option<ScrollButtonHold>,
    slider_drag: Option<SliderDrag>,
    slider_motion: HashMap<String, SliderMotion>,
    hovered_scrollbar: Option<String>,
    focus_order: Vec<(String, HitKind)>,
    action_aliases: HashMap<String, String>,
    focused: Option<String>,
    caret_blink: crate::caret_blink::CaretBlink,
    modal_return_focus: Option<String>,
    pending_scroll_focus: Option<String>,
    /// The control to bring into view, its margin, and whether it goes to
    /// the start of the viewport instead of just into it.
    pending_reveal: Option<(String, f32, bool)>,
    pending_center: Option<String>,
    hovered: Option<String>,
    pointer: Option<Point>,
    tooltip_hover_since: Option<(String, Instant)>,
    keyboard_focus: bool,
    keyboard_tooltip_focus: bool,
    closed_submenu_triggers: HashSet<String>,
    /// The anchors of the last layout's hints, with how long each waits.
    tooltip_triggers: HashMap<String, Duration>,
    pressed: Option<String>,
    pressed_dispatch: Option<String>,
    pointer_drag: Option<PointerDrag>,
    transitions: HashMap<String, ColorTransition>,
    popover_transitions: HashMap<String, ScalarTransition>,
    popover_snapshots: HashMap<String, PopoverSnapshot>,
    popover_exits: HashMap<String, PopoverExit>,
    replacements: HashMap<String, Replacement>,
    // Measuring reads and starts these, and measuring borrows the tree shared.
    flow_heights: RefCell<HashMap<String, FlowHeight>>,
    scalars: RefCell<HashMap<String, AnimatedScalar>>,
    springs: RefCell<HashMap<String, AnimatedSpring>>,
    /// A pushing reach jumped while placed: the layout it was measured for is stale.
    pushes_moved: std::cell::Cell<bool>,
    /// The ink of each `animate_foreground` element and when it was seen.
    inks: RefCell<HashMap<String, (ColorTransition, Instant)>>,
    /// The slots each `keep_leaving` column last laid out, in order.
    leaving: RefCell<HashMap<String, (Vec<String>, Instant)>>,
    popover_view_transitions: HashMap<String, PopoverViewTransition>,
    coach_motion: Option<CoachMotion>,
    /// The coach mark step whose target has been revealed and focused.
    coach_key: Option<String>,
    tab_indicators: HashMap<String, TabIndicatorState>,
    skeleton_started: HashMap<String, Instant>,
    shimmer_started: HashMap<String, Instant>,
    spinner_started: HashMap<String, Instant>,
    record_motion: HashMap<String, RecordMotion>,
    reduced_motion: bool,
    static_selection: Option<StaticSelection>,
    context_menu: Option<ContextMenuState>,
    context_menu_palette: crate::ComponentPalette,
    context_menu_swallow_release: bool,
    ime_owner: Option<ime_tree::ImeOwner>,
    ime_window_focused: bool,
}

impl UiTree {
    /// A tree showing `root`, with the default font metrics and no host text
    /// measurer.
    pub fn new(root: Element) -> Self {
        Self {
            paste_epoch: Arc::new(()),
            paste_layout_pending: true,
            root,
            metrics: FontMetrics::default(),
            sans_metrics: FontMetrics::default(),
            text_measurer: None,
            text_height_measurer: None,
            width_cache: RefCell::new(TextWidthCache::default()),
            formatted_cache: RefCell::new(FormattedTextCache::default()),
            layout_cache: RefCell::new(TextLayoutCache::default()),
            text_layout: false,
            measurement_cache: RefCell::new(MeasurementCache::default()),
            horizontal_scrolls: HashMap::new(),
            frame: Frame::default(),
            inputs: HashMap::new(),
            code_editors: HashMap::new(),
            code_diffs: HashMap::new(),
            code_view_last_seen: HashMap::new(),
            code_view_epoch: 0,
            scroll_offsets: HashMap::new(),
            scroll_transitions: HashMap::new(),
            scrollbar_states: HashMap::new(),
            measured_lists: HashMap::new(),
            scroll_to_end: HashSet::new(),
            scroll_to_index: HashMap::new(),
            scroll_drag: None,
            horizontal_pan: None,
            scroll_button_hold: None,
            slider_drag: None,
            slider_motion: HashMap::new(),
            hovered_scrollbar: None,
            focus_order: Vec::new(),
            action_aliases: HashMap::new(),
            focused: None,
            caret_blink: crate::caret_blink::CaretBlink::default(),
            modal_return_focus: None,
            pending_scroll_focus: None,
            pending_reveal: None,
            pending_center: None,
            hovered: None,
            pointer: None,
            tooltip_hover_since: None,
            keyboard_focus: false,
            keyboard_tooltip_focus: false,
            closed_submenu_triggers: HashSet::new(),
            tooltip_triggers: HashMap::new(),
            pressed: None,
            pressed_dispatch: None,
            pointer_drag: None,
            transitions: HashMap::new(),
            popover_transitions: HashMap::new(),
            popover_snapshots: HashMap::new(),
            popover_exits: HashMap::new(),
            replacements: HashMap::new(),
            flow_heights: RefCell::new(HashMap::new()),
            scalars: RefCell::new(HashMap::new()),
            springs: RefCell::new(HashMap::new()),
            pushes_moved: std::cell::Cell::new(false),
            inks: RefCell::new(HashMap::new()),
            leaving: RefCell::new(HashMap::new()),
            popover_view_transitions: HashMap::new(),
            coach_motion: None,
            coach_key: None,
            tab_indicators: HashMap::new(),
            skeleton_started: HashMap::new(),
            shimmer_started: HashMap::new(),
            spinner_started: HashMap::new(),
            record_motion: HashMap::new(),
            reduced_motion: false,
            static_selection: None,
            context_menu: None,
            context_menu_palette: crate::ComponentPalette::neutral_dark(),
            context_menu_swallow_release: false,
            ime_owner: None,
            ime_window_focused: true,
        }
    }

    /// The builder form of [`Self::set_metrics`].
    pub fn with_metrics(mut self, metrics: FontMetrics) -> Self {
        self.set_metrics(metrics);
        self
    }

    /// Updates the dimensions used for text layout and cursor positioning.
    /// Match these values to the font selected by the host renderer.
    pub fn set_metrics(&mut self, metrics: FontMetrics) {
        let metrics = FontMetrics {
            char_width: metrics.char_width.max(1.0),
            line_height: metrics.line_height.max(1.0),
        };
        if self.metrics != metrics {
            self.width_cache.get_mut().clear();
            self.formatted_cache.get_mut().clear();
            self.metrics = metrics;
        }
    }

    /// Calibrate the proportional UI font at [`DEFAULT_FONT_SIZE`]. Text width
    /// is estimated with one representative character because glyphs vary.
    pub fn set_sans_metrics(&mut self, metrics: FontMetrics) {
        let metrics = FontMetrics {
            char_width: metrics.char_width.max(1.0),
            line_height: metrics.line_height.max(1.0),
        };
        if self.sans_metrics != metrics {
            self.width_cache.get_mut().clear();
            self.formatted_cache.get_mut().clear();
            self.sans_metrics = metrics;
        }
    }

    /// Install the host painter's text width function. It must use the same
    /// font family and size as `DrawCommand::Text`. The callback is retained
    /// between frames; a missing callback uses deterministic font metrics.
    /// Reinstall after font or scale changes to invalidate cached widths.
    pub fn set_text_measurer(
        &mut self,
        measure: impl Fn(&str, f32, FontFamily) -> f32 + Send + Sync + 'static,
    ) {
        self.text_measurer = Some(Arc::new(measure));
        self.width_cache.get_mut().clear();
        self.formatted_cache.get_mut().clear();
    }

    /// Lay `Element::text` out in limn's own [`crate::TextLayout`] and draw it
    /// as glyph runs, instead of measuring it through the text measurer and
    /// leaving the backend to lay it out again. Its caret, its selection and
    /// its glyphs then come from one layout. `limn_egui::attach_fonts` turns
    /// it on. Off by default while limn's components move to it one by one;
    /// once they all have, it is always on and this goes away.
    pub fn set_text_layout(&mut self, enabled: bool) {
        self.text_layout = enabled;
    }

    /// Remove the host's text width function, so widths come from the font
    /// metrics again. Cached widths are dropped.
    pub fn clear_text_measurer(&mut self) {
        self.text_measurer = None;
        self.width_cache.get_mut().clear();
        self.formatted_cache.get_mut().clear();
    }

    /// Supply the painter's unwrapped font row height at the exact logical
    /// size. Code views use it to center every syntax run and gutter number in
    /// their fixed-height rows. This is a font metric, not each string's ink
    /// bounds: all runs must retain one baseline, including empty lines.
    /// Invalid values fall back to the calibrated `FontMetrics`.
    pub fn set_text_height_measurer(
        &mut self,
        measure: impl Fn(f32, FontFamily) -> f32 + Send + Sync + 'static,
    ) {
        self.text_height_measurer = Some(Arc::new(measure));
    }

    /// Remove the function installed with [`Self::set_text_height_measurer`].
    pub fn clear_text_height_measurer(&mut self) {
        self.text_height_measurer = None;
    }

    /// Replaces the visual tree without discarding named input, focus or scroll
    /// state. Reuse IDs for widgets that represent the same logical control.
    /// Temporary anonymous code viewports belong to the replaced instance.
    pub fn set_root(&mut self, root: Element) {
        self.paste_layout_pending = true;
        self.root = root;
        self.action_aliases.clear();
        // Anonymous code viewports belong to the replaced instance; explicit IDs retain state.
        self.horizontal_scrolls.retain(|id, _| !crate::code_block::is_anonymous_code_scroll_id(id));
    }

    /// The text a text input or text area holds now. `None` until a field
    /// with this ID has been laid out or set with [`Self::set_input_value`].
    pub fn input_value(&self, id: &str) -> Option<&str> {
        self.inputs.get(id).map(|state| state.value.as_str())
    }

    /// Caret position measured in Unicode characters, matching editing events.
    pub fn input_cursor(&self, id: &str) -> Option<usize> {
        self.inputs.get(id).map(|state| state.cursor)
    }

    /// Selected Unicode character range; an empty range represents the caret.
    pub fn input_selection(&self, id: &str) -> Option<(usize, usize)> {
        self.inputs.get(id).map(|state| {
            state
                .selected_range()
                .unwrap_or((state.cursor, state.cursor))
        })
    }

    /// Move the caret without changing focus or replacing text. Autocomplete
    /// uses this after inserting a token in the middle of a multiline draft.
    pub fn set_input_cursor(&mut self, id: &str, cursor: usize) {
        if let Some(state) = self.inputs.get_mut(id) {
            let cursor = cursor.min(state.value.chars().count());
            if self.focused.as_deref() == Some(id)
                && (state.cursor != cursor || state.selected_range().is_some())
            {
                self.caret_blink.reset();
            }
            state.set_selection(cursor, None);
            state.keep_cursor_visible = true;
            state.desired_column = None;
            state.desired_x = None;
        }
    }

    /// Text selected by dragging over one static text or formatted-text block.
    /// The copied slice preserves source newlines and Unicode boundaries.
    pub fn selected_text(&self) -> Option<&str> {
        let selection = self.static_selection.as_ref()?;
        let (start, end) = selection.range()?;
        Some(char_range(&selection.source, start, end))
    }

    /// Replace the text of a field from the host. The caret goes to the end
    /// and any selection is dropped. A field not laid out yet takes this
    /// value in place of its initial one.
    pub fn set_input_value(&mut self, id: &str, value: impl Into<String>) {
        let value = value.into();
        let cursor = value.chars().count();
        let state = self
            .inputs
            .entry(id.to_owned())
            .or_insert_with(|| InputState::new(String::new()));
        if self.focused.as_deref() == Some(id)
            && (state.value != value || state.cursor != cursor || state.selected_range().is_some())
        {
            self.caret_blink.reset();
        }
        if state.value != value {
            state.paste_epoch = Arc::new(());
            state.value = value;
        }
        state.set_selection(cursor, None);
        state.view_start = 0;
        state.scroll_x = 0.0;
        state.scroll_line = 0;
        state.keep_cursor_visible = true;
        state.desired_column = None;
        state.desired_x = None;
    }

    /// Where an input method belongs for the focused code editor: the visible
    /// part of the editor's body and of its caret. `None` when the window is
    /// not focused, a popover is open or no code editor has the focus.
    pub fn editor_ime_rects(&self) -> Option<(Rect, Rect)> {
        if !self.ime_window_focused || self.has_open_popover() {
            return None;
        }
        self.code_editors.get(self.focused.as_ref()?)?.ime_rects()
    }

    /// The ID of the control that has the focus, from the pointer or the
    /// keyboard.
    pub fn focused_id(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Dismiss the focused control and cancel its native composition without
    /// changing window focus. Returns whether a control had focus.
    pub fn clear_focus(&mut self) -> bool {
        let had_focus = self.focused.is_some();
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, None);
        self.pending_scroll_focus = None;
        self.keyboard_focus = false;
        self.keyboard_tooltip_focus = false;
        self.validate_ime_owner(Instant::now());
        had_focus
    }

    /// Whether the current focus came from keyboard navigation rather than a
    /// pointer press. Hosts can retain focus while navigating a tab strip.
    pub fn has_keyboard_focus(&self) -> bool {
        self.focused.is_some() && self.keyboard_focus
    }

    /// Whether a floating surface currently owns keyboard navigation.
    pub fn has_open_popover(&self) -> bool {
        self.context_menu.is_some() || self.active_popover_scope().is_some()
    }

    /// Whether a floating surface other than the named container owns input.
    /// A drawer may contain an ordinary form while nested menus must still
    /// suspend that form's shortcuts. Native context menus always own input.
    pub fn has_open_popover_except(&self, container_dismiss_id: &str) -> bool {
        self.context_menu.is_some()
            || self
                .frame
                .popover_focus
                .iter()
                .any(|scope| scope.dismiss_id != container_dismiss_id)
    }

    /// The ID of the control under the pointer.
    pub fn hovered_id(&self) -> Option<&str> {
        self.hovered.as_deref()
    }

    /// The shape the pointer should have: that of the element being dragged,
    /// wherever the pointer went with it, or else of the element under it.
    /// `None` leaves it to the platform.
    pub fn cursor(&self) -> Option<Cursor> {
        let dragged = self.pointer_drag.as_ref().filter(|drag| drag.active).map(|drag| drag.id.as_str());
        self.frame.cursors.get(dragged.or(self.hovered.as_deref())?).copied()
    }

    /// Focus an interactive element after layout. Returns false for an unknown ID.
    /// Text inputs place the caret at the end so command launchers can accept
    /// typing immediately after opening.
    pub fn focus(&mut self, id: &str) -> bool {
        self.focus_with_modality(id, true)
    }

    /// For a host that intercepts pointer presses (for example draggable tabs).
    /// Focus the target without presenting keyboard focus or an instant tooltip.
    pub fn focus_from_pointer(&mut self, id: &str) -> bool {
        self.focus_with_modality(id, false)
    }

    fn focus_with_modality(&mut self, id: &str, keyboard: bool) -> bool {
        if !self.focus_order.iter().any(|(candidate, kind)| {
            candidate == id
                && matches!(
                    kind,
                    HitKind::Button | HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor | HitKind::CodeDiff
                )
        }) {
            return false;
        }
        self.request_scroll_focus(id);
        self.caret_blink.reset();
        let next_focus = Some(id.to_owned());
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        self.validate_ime_owner(Instant::now());
        self.keyboard_focus = keyboard;
        self.keyboard_tooltip_focus = keyboard;
        if let Some(input) = self.inputs.get_mut(id) {
            input.set_selection(input.value.chars().count(), None);
            input.keep_cursor_visible = true;
        }
        true
    }

    /// Disable decorative color transitions when the user requests reduced motion.
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        if self.reduced_motion != reduced {
            self.caret_blink.reset();
        }
        if reduced {
            for (id, motion) in self.scroll_transitions.drain() {
                self.scroll_offsets.insert(id, motion.target());
            }
            self.transitions.clear();
            self.popover_transitions.clear();
            self.popover_snapshots.clear();
            self.popover_exits.clear();
            self.replacements.clear();
            self.flow_heights.get_mut().clear();
            self.scalars.get_mut().clear();
            self.springs.get_mut().clear();
            self.inks.get_mut().clear();
            self.leaving.get_mut().clear();
            self.popover_view_transitions.clear();
            self.coach_motion = None;
            self.tab_indicators.clear();
            self.skeleton_started.clear();
            self.shimmer_started.clear();
            self.spinner_started.clear();
            self.record_motion.clear();
            self.slider_motion.clear();
        }
        self.reduced_motion = reduced;
    }

    /// How soon the next frame is needed, or `None` at rest.
    ///
    /// A transition returns zero: paint on the very next display refresh, so
    /// movement runs at the display's own rate and in step with it. A fixed
    /// interval would cap a 120 Hz display at half its rate and drift against
    /// any display, which reads as stutter. Ambient indicators such as a
    /// spinner keep a 16 ms cadence: they never finish, so painting them at
    /// display rate would cost power for no visible gain. A host that slows
    /// its frames down when idle should do so only for those: throttling a
    /// zero turns every transition into a stutter.
    pub fn animation_repaint_after(&self) -> Option<Duration> {
        self.animation_repaint_after_at(Instant::now())
    }

    /// Hosts forward native window focus (and which tree owns keyboard input).
    /// A change requires one repaint; an inactive caret has no timer deadline.
    pub fn set_window_focused(&mut self, focused: bool) -> bool {
        if self.ime_window_focused != focused {
            self.invalidate_paste_targets();
        }
        self.ime_window_focused = focused;
        self.validate_ime_owner(Instant::now());
        self.caret_blink.set_window_focused(focused)
    }

    /// How long until the blinking caret has to be painted again; zero is
    /// now. `None` when no caret blinks or a context menu is open.
    pub fn caret_repaint_after(&self) -> Option<Duration> {
        self.caret_repaint_after_at(Instant::now())
    }

    fn caret_repaint_after_at(&self, now: Instant) -> Option<Duration> {
        if self.context_menu.is_some() {
            None
        } else {
            self.caret_blink.repaint_after(now)
        }
    }

    fn animation_repaint_after_at(&self, now: Instant) -> Option<Duration> {
        let caret = self.caret_repaint_after_at(now);
        let animation = self.visual_animation_repaint_after_at(now);
        match (caret, animation) {
            (Some(caret), Some(animation)) => Some(caret.min(animation)),
            (caret, animation) => caret.or(animation),
        }
    }

    fn visual_animation_repaint_after_at(&self, now: Instant) -> Option<Duration> {
        let animation = self.other_animation_repaint_after_at(now);
        let scrollbar = self.scrollbar_states.values().filter_map(|state| state.repaint_after(now)).min();
        // A text area's scrollbar hides a moment after its last scroll.
        let bar = self
            .inputs
            .values()
            .filter_map(|input| input.scrolled_at)
            .map(|at| (at + TEXT_AREA_BAR_HOLD).saturating_duration_since(now))
            .filter(|left| !left.is_zero())
            .min();
        let bar = match (bar, scrollbar) {
            (Some(bar), Some(scrollbar)) => Some(bar.min(scrollbar)),
            (bar, scrollbar) => bar.or(scrollbar),
        };
        match (animation, bar) {
            (Some(animation), Some(bar)) => Some(animation.min(bar)),
            (animation, bar) => animation.or(bar),
        }
    }

    fn other_animation_repaint_after_at(&self, now: Instant) -> Option<Duration> {
        if self.frame.regions.iter().any(|region| match region.kind {
            HitKind::CodeEditor => self
                .code_editors
                .get(&region.id)
                .is_some_and(|view| view.syntax_pending()),
            HitKind::CodeDiff => self
                .code_diffs
                .get(&region.id)
                .is_some_and(|view| view.syntax_pending()),
            _ => false,
        }) {
            // Poll bounded background work even when motion is disabled.
            return Some(Duration::from_millis(16));
        }
        if self
            .scroll_transitions
            .values()
            .any(|state| state.is_active(now))
        {
            // Paint each refresh during direct manipulation. Interpolation
            // uses elapsed time, so it is independent of the frame rate.
            return Some(Duration::ZERO);
        }
        let ambient = (!self.reduced_motion
            && (!self.skeleton_started.is_empty()
                || !self.shimmer_started.is_empty()
                || !self.spinner_started.is_empty()
                || self.record_motion.values().any(|record| record.is_moving(now))))
            || self.scroll_button_hold.is_some();
        let transition = (!self.reduced_motion
            && (self.slider_drag.is_some()
                || self.slider_motion.values().any(|state| state.is_fast(now))))
            || self.transitions.values().any(|state| state.is_active(now))
            || self
                .popover_transitions
                .values()
                .any(|state| state.is_active(now))
            || self
                .popover_exits
                .values()
                .any(|state| state.opacity.is_active(now))
            || self
                .replacements
                .values()
                .any(|state| state.progress.is_active(now))
            || self
                .flow_heights
                .borrow()
                .values()
                .any(|state| state.fraction.is_active(now))
            || self
                .scalars
                .borrow()
                .values()
                .any(|state| state.value.is_active(now))
            || self
                .springs
                .borrow()
                .values()
                .any(|state| state.value.is_active(now))
            || self.inks.borrow().values().any(|(ink, _)| ink.is_active(now))
            || self
                .popover_view_transitions
                .values()
                .any(|state| state.opacity.is_active(now) || state.height.is_active(now))
            || self
                .coach_motion
                .as_ref()
                .is_some_and(|state| state.values.iter().any(|value| value.is_active(now)))
            || self
                .tab_indicators
                .values()
                .any(|state| state.x.is_active(now) || state.width.is_active(now))
            || self.horizontal_scrolls.values().any(|state| state.is_gliding(now));
        let hover_wait = self.tooltip_hover_since.as_ref().and_then(|(id, since)| {
            let elapsed = now.saturating_duration_since(*since);
            let delay = *self.tooltip_triggers.get(id)?;
            (self.pressed.as_deref() != Some(id) && elapsed < delay).then(|| delay - elapsed)
        });
        if transition {
            Some(Duration::ZERO)
        } else if ambient {
            Some(Duration::from_millis(16))
        } else if !self.reduced_motion && !self.slider_motion.is_empty() {
            Some(hover_wait.map_or(Duration::from_millis(33), |wait| {
                wait.min(Duration::from_millis(33))
            }))
        } else {
            // Waiting to open a tooltip has no intermediate visual states.
            // Wake once at the deadline instead of rendering at display rate.
            hover_wait
        }
    }

    /// Hosts should schedule a repaint while this is true.
    pub fn is_animating(&self) -> bool {
        self.animation_repaint_after().is_some()
    }

    /// How far a scroll area is scrolled, in logical pixels. Zero for an ID
    /// that has no offset yet.
    pub fn scroll_offset(&self, id: &str) -> f32 {
        self.scroll_offsets.get(id).copied().unwrap_or(0.0)
    }

    /// Checks the last layout's scroll extent. Call before changing content to
    /// decide whether incoming content should keep the viewport pinned to bottom.
    pub fn is_scroll_at_end(&self, id: &str, tolerance: f32) -> bool {
        self.frame
            .scroll_extents
            .get(id)
            .is_some_and(|maximum| maximum - self.scroll_target(id) <= tolerance.max(0.0))
    }

    /// Position a vertical scroll area from the host, for example from a
    /// spring, as [`Self::set_horizontal_scroll_offset`] does sideways. It
    /// takes over from any wheel easing and pending end or index request; the
    /// next layout clamps it to the content. A value that is not finite is
    /// ignored.
    pub fn set_scroll_offset(&mut self, id: &str, offset: f32) {
        if !offset.is_finite() {
            return;
        }
        self.scroll_transitions.remove(id);
        self.cancel_scroll_hold(id);
        self.scroll_to_end.remove(id);
        self.scroll_to_index.remove(id);
        self.scroll_offsets.insert(id.to_owned(), offset);
    }

    /// Scrolls to the bottom on the next layout, after new content is measured.
    pub fn scroll_to_end(&mut self, id: &str) {
        self.scroll_transitions.remove(id);
        self.cancel_scroll_hold(id);
        self.scroll_to_index.remove(id);
        self.scroll_to_end.insert(id.to_owned());
    }

    /// Reset any scroll area to its beginning when its content changes scope
    /// or page. This also cancels pending wheel easing and end/index requests.
    pub fn scroll_to_start(&mut self, id: &str) {
        self.scroll_transitions.remove(id);
        self.cancel_scroll_hold(id);
        self.scroll_to_end.remove(id);
        self.scroll_to_index.remove(id);
        self.scroll_offsets.insert(id.to_owned(), 0.0);
    }

    /// Keeps one fixed-height list row in view on the next layout. The scroll
    /// position is unchanged if the row is already fully visible.
    pub fn scroll_to_index(&mut self, id: &str, index: usize) {
        self.scroll_transitions.remove(id);
        self.cancel_scroll_hold(id);
        self.scroll_to_end.remove(id);
        self.scroll_to_index.insert(id.to_owned(), index);
    }

    /// Brings the control `id` into view on the next layout: every scroll
    /// area around it scrolls just enough to show it whole, 12 px clear of
    /// the edge. It needs no fixed row height, moves nothing when the
    /// control is already in view, and does nothing when no control has
    /// that ID. Focus does not move. Rows of a virtual list are not in the
    /// tree until they are in view: use [`UiTree::scroll_to_index`] there.
    pub fn reveal(&mut self, id: &str) {
        self.reveal_with_margin(id, REVEAL_MARGIN);
    }

    /// As [`UiTree::reveal`], keeping `margin` clear of the edges instead:
    /// more when a fade or a fixed header covers the viewport's edge, or to
    /// show what follows a heading. A margin the viewport has no room for
    /// shrinks until the control fits; one that is not finite is ignored.
    pub fn reveal_with_margin(&mut self, id: &str, margin: f32) {
        let margin = if margin.is_finite() { margin.max(0.0) } else { REVEAL_MARGIN };
        self.pending_reveal = Some((id.to_owned(), margin, false));
    }

    /// Scrolls so the control or anchor `id` starts `margin` under the top
    /// of its scroll area, wherever it was: also when it is already in view,
    /// which [`UiTree::reveal`] leaves alone. It is how a table of contents
    /// jumps to a section. The scroll areas around that one only show it,
    /// and the end of the content stops it short.
    pub fn reveal_at_start(&mut self, id: &str, margin: f32) {
        let margin = if margin.is_finite() { margin.max(0.0) } else { REVEAL_MARGIN };
        self.pending_reveal = Some((id.to_owned(), margin, true));
    }

    /// Where the `Element::anchor` named `id` was at the last layout: how far
    /// its top is below the top of the scroll area around it, negative once
    /// it has scrolled past. Anchors in clipped static branches retain their
    /// current geometry without being painted. `None` when the anchor is absent
    /// (including a virtual row that has not been built).
    pub fn anchor_offset(&self, id: &str) -> Option<f32> {
        let target = self.frame.scroll_focus_targets.get(id)?;
        Some(target.rect.y - target.viewport.y)
    }

    /// The rows of the virtual list `id` that were built at the last layout,
    /// as a range of indices: those in view, from the one at the top of the
    /// viewport to the one at its bottom. `None` when no uniform or measured
    /// list has that ID. A host reads it to load more rows before the first
    /// one shows, or to know which messages were seen.
    pub fn visible_rows(&self, id: &str) -> Option<std::ops::Range<usize>> {
        let list = self.frame.virtual_lists.get(id)?;
        Some(list.first_visible..list.end_visible)
    }

    /// Brings the control `id` to the middle of the scroll area that holds
    /// it, on the next layout, travelling there by that area's scroll motion
    /// or at once under reduced motion. For content that follows a clock,
    /// such as the line being sung. Near either end of the content it goes
    /// as far as the area scrolls. It does nothing when no control has that
    /// ID, and focus does not move.
    pub fn center(&mut self, id: &str) {
        self.pending_center = Some(id.to_owned());
    }

    /// Lay the tree out in a viewport of this size at the current time, and
    /// return the frame to paint; see [`Self::layout_at`]. A negative width
    /// or height is read as zero.
    pub fn layout(&mut self, viewport: Size) -> Frame {
        self.layout_at(viewport, Instant::now())
    }

    /// Layout using the host's monotonic frame time. This also lets component
    /// consumers exercise animations and delayed tooltips with a fixed clock.
    pub fn layout_at(&mut self, viewport: Size, now: Instant) -> Frame {
        self.layout_at_with_hover_refresh(viewport, now, true)
    }

    fn layout_at_with_hover_refresh(
        &mut self,
        viewport: Size,
        now: Instant,
        refresh_hover_paint: bool,
    ) -> Frame {
        self.begin_paste_layout();
        self.layout_cache.get_mut().begin_pass();
        let viewport = Size::new(viewport.width.max(0.0), viewport.height.max(0.0));
        for state in self.scrollbar_states.values_mut() { state.seen = false; }
        self.caret_blink.begin_frame(self.context_menu.is_none());
        self.repeat_scroll_button(now);
        if self.pending_scroll_focus.as_deref() != self.focused.as_deref() {
            self.pending_scroll_focus = None;
        }
        let previous_popovers = self.frame.popover_focus.clone();
        let mut focus_order = Vec::new();
        let mut seen = HashSet::new();
        collect_focus_order(&self.root, &mut focus_order, &mut seen);
        self.action_aliases.clear();
        let mut menu_secondary_actions = HashSet::new();
        collect_action_metadata(
            &self.root,
            &mut self.action_aliases,
            &mut menu_secondary_actions,
        );
        // One control is located per layout: the focused one first.
        let probe = self
            .pending_scroll_focus
            .as_deref()
            .or(self.pending_reveal.as_ref().map(|(id, ..)| id.as_str()));
        let mut focus_path = HashSet::new();
        find_focus_path(&self.root, probe, &mut focus_path);
        let mut observed_buttons = HashSet::new();
        let mut observed_popovers = HashSet::new();
        let mut observed_replacements = HashSet::new();
        let mut current_popover_snapshots = HashMap::new();
        let mut observed_tabs = HashSet::new();
        let mut observed_tooltips = HashMap::new();
        let mut observed_skeletons = HashSet::new();
        let mut observed_shimmers = HashSet::new();
        let mut observed_spinners = HashSet::new();
        let mut observed_records = HashSet::new();
        let mut observed_sliders = HashSet::new();
        let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
        let context_menu_paint = self.context_menu_paint(viewport_rect);
        self.measurement_cache.get_mut().clear();
        let ghost_range;
        let (mut frame, overlay_start) = {
            let mut builder = LayoutBuilder {
                viewport: viewport_rect,
                frame: Frame {
                    menu_secondary_actions,
                    ..Frame::default()
                },
                action_aliases: &mut self.action_aliases,
                overlays: VecDeque::new(),
                scroll_focus_context: Vec::new(),
                pinned: Vec::new(),
                translated_overflow: HashMap::new(),
                focus_path,
                focus_probe: probe,
                drag_source: self
                    .pointer_drag
                    .as_ref()
                    .filter(|drag| drag.active)
                    .map(|drag| drag.id.as_str()),
                drag_ghost: None,
                metrics: self.metrics,
                sans_metrics: self.sans_metrics,
                text_measurer: self.text_measurer.clone(),
                text_height_measurer: self.text_height_measurer.clone(),
                width_cache: &self.width_cache,
                formatted_cache: &self.formatted_cache,
                layout_cache: &self.layout_cache,
                text_layout: self.text_layout,
                measurement_cache: &self.measurement_cache,
                flow_heights: &self.flow_heights,
                scalars: &self.scalars,
                springs: &self.springs,
                pushes_moved: &self.pushes_moved,
                inks: &self.inks,
                leaving: &self.leaving,
                measurement_scope: 0,
                next_measurement_scope: 0,
                horizontal_scrolls: &mut self.horizontal_scrolls,
                inputs: &mut self.inputs,
                code_editors: &mut self.code_editors,
                code_diffs: &mut self.code_diffs,
                scroll_offsets: &mut self.scroll_offsets,
                scroll_transitions: &mut self.scroll_transitions,
                scrollbar_states: &mut self.scrollbar_states,
                measured_lists: &mut self.measured_lists,
                scroll_to_end: &mut self.scroll_to_end,
                scroll_to_index: &mut self.scroll_to_index,
                hovered_scrollbar: self.hovered_scrollbar.as_deref(),
                dragging_scrollbar: self.scroll_drag.as_ref().map(|drag| drag.id.as_str()),
                dragging_horizontal: self
                    .horizontal_pan
                    .as_ref()
                    .filter(|pan| pan.claimed)
                    .map(|pan| pan.id.as_str()),
                scroll_button_hold: self.scroll_button_hold.as_ref(),
                // An open menu is modal: the page under it does not answer the pointer.
                pointer: self.pointer.filter(|_| context_menu_paint.is_none()),
                region_hovered: false,
                access_above: None,
                slider_drag: self.slider_drag.as_ref(),
                slider_motion: &mut self.slider_motion,
                focused: self.focused.as_deref(),
                caret_blink: &mut self.caret_blink,
                hovered: if self.context_menu.is_some() {
                    None
                } else {
                    self.hovered.as_deref()
                },
                tooltip_hover_since: self.tooltip_hover_since.as_ref(),
                keyboard_focus: self.keyboard_focus,
                closed_submenu_triggers: &self.closed_submenu_triggers,
                keyboard_tooltip_focus: self.keyboard_tooltip_focus,
                pressed: self.pressed.as_deref(),
                selection: self.static_selection.as_ref(),
                transitions: &mut self.transitions,
                popover_transitions: &mut self.popover_transitions,
                popover_exits: &self.popover_exits,
                replacements: &mut self.replacements,
                observed_replacements: &mut observed_replacements,
                previous_popover_snapshots: &self.popover_snapshots,
                popover_view_transitions: &mut self.popover_view_transitions,
                coach_motion: &mut self.coach_motion,
                tab_indicators: &mut self.tab_indicators,
                skeleton_started: &mut self.skeleton_started,
                shimmer_started: &mut self.shimmer_started,
                spinner_started: &mut self.spinner_started,
                record_motion: &mut self.record_motion,
                observed_records: &mut observed_records,
                observed_buttons: &mut observed_buttons,
                hover_surface_fill: None,
                surface_gaps_lit: self.context_menu.is_none() && self.hovered.is_none(),
                observed_popovers: &mut observed_popovers,
                popover_snapshots: &mut current_popover_snapshots,
                observed_tabs: &mut observed_tabs,
                observed_tooltips: &mut observed_tooltips,
                observed_skeletons: &mut observed_skeletons,
                observed_shimmers: &mut observed_shimmers,
                observed_spinners: &mut observed_spinners,
                observed_sliders: &mut observed_sliders,
                reduced_motion: self.reduced_motion,
                locating: probe.is_some(),
                now,
            };
            builder.place(&self.root, viewport_rect, None);
            let overlay_start = builder.frame.commands.len();
            // Deferred painting gives popovers one stable top layer even when
            // their anchor lives inside a clipped scroll container. Nested
            // overlays are appended and painted above their parent overlay.
            while let Some(overlay) = builder.overlays.pop_front() {
                let previous_scope = builder.enter_measurement_scope();
                match overlay {
                    DeferredOverlay::Free { floating, area } => {
                        let mut floating_path = HashSet::new();
                        find_focus_path(&floating, builder.focus_probe, &mut floating_path);
                        builder.focus_path.extend(floating_path);
                        builder.place(&floating, area, Some(viewport_rect));
                    }
                    DeferredOverlay::Anchored {
                        mut floating,
                        mut anchor,
                        dismiss_id,
                        placement,
                        gap,
                        trigger_id,
                        focusables,
                        animation_ms,
                        close_animation_ms,
                        open_scale,
                        view_key,
                        backdrop,
                        focus_scope,
                        backdrop_dismiss,
                        scrim,
                        initial_focus,
                        return_focus,
                        coach,
                        arrows,
                        pointed,
                    } => {
                        let coach = coach.as_deref();
                        if let Some(coach) = coach {
                            // Every control of the base tree is laid out by
                            // now, wherever it was built.
                            let target = builder
                                .frame
                                .regions
                                .iter()
                                .filter(|region| coach.targets.contains(&region.id))
                                .filter_map(|region| region.rect.intersection(viewport_rect))
                                .reduce(|a, b| {
                                    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
                                    Rect::new(
                                        x,
                                        y,
                                        (a.x + a.width).max(b.x + b.width) - x,
                                        (a.y + a.height).max(b.y + b.height) - y,
                                    )
                                });
                            builder.frame.coach = Some(CoachFrame {
                                key: coach.key.clone(),
                                target: coach.targets.first().cloned().unwrap_or_default(),
                                focus: initial_focus.clone(),
                                missing: target.is_none(),
                            });
                            let Some(target) = target else {
                                // Nothing to point at: no bubble adrift in a
                                // corner, and the window stays usable.
                                builder.measurement_scope = previous_scope;
                                continue;
                            };
                            anchor = target;
                        }
                        // A coach mark dims around its target, further down.
                        let scrim_index = scrim.filter(|_| coach.is_none()).map(|color| {
                            builder.frame.commands.push(DrawCommand::FillRect {
                                rect: viewport_rect,
                                color,
                                radius: 0.0,
                            });
                            builder.frame.commands.len() - 1
                        });
                        if backdrop {
                            builder.frame.regions.push(HitRegion {
                                id: dismiss_id.clone(),
                                rect: viewport_rect,
                                kind: if backdrop_dismiss {
                                    HitKind::Button
                                } else {
                                    HitKind::Surface
                                },
                            });
                        }
                        if focus_scope {
                            builder.frame.popover_focus.push(PopoverFocusScope {
                                dismiss_id: dismiss_id.clone(),
                                trigger_id,
                                focusables,
                                initial_focus,
                                return_focus,
                                submenu: !backdrop,
                                modal: scrim.is_some(),
                                arrows: coach
                                    .map(|coach| (coach.back.clone(), coach.next.clone()))
                                    .or_else(|| arrows.as_deref().cloned()),
                                pointed,
                            });
                        }
                        // Measure wrapped content at the width the viewport
                        // can actually allocate. Shrinking only the painted
                        // rectangle would retain the wider layout's height.
                        let available_width = if placement == PopoverPlacement::Bottom {
                            viewport.width.max(0.0)
                        } else {
                            (viewport.width - 16.0).max(0.0)
                        };
                        if let Some(width) = floating.style.width.as_mut() {
                            *width = width.min(available_width);
                        }
                        let measured = builder.measure(
                            &floating,
                            Size::new(available_width, f32::INFINITY),
                        );
                        let (rect, coach_side) = match coach {
                            Some(coach) => {
                                place_coach(anchor, measured, viewport_rect, coach.side, coach.align)
                            }
                            None => (
                                place_popover(anchor, measured, viewport_rect, placement, gap),
                                None,
                            ),
                        };
                        let rect = LayoutBuilder::translated_rect(&floating, rect);
                        builder.observed_popovers.insert(dismiss_id.clone());
                        let surface = if floating.kept.as_ref().is_some_and(|kept| kept.translation.is_some()) {
                            rect.intersection(viewport_rect)
                        } else { Some(rect) };
                        if let Some(visible) = surface {
                            builder.frame.regions.push(HitRegion {
                                id: format!("popover_surface:{dismiss_id}"),
                                rect: visible,
                                kind: HitKind::Surface,
                            });
                        }
                        let progress = if builder.reduced_motion || animation_ms == 0 {
                            1.0
                        } else {
                            let transition = builder
                                .popover_transitions
                                .entry(dismiss_id.clone())
                                .or_insert_with(|| {
                                    ScalarTransition::new(
                                        builder
                                            .popover_exits
                                            .get(&dismiss_id)
                                            .map(|exit| exit.opacity.value(now))
                                            .unwrap_or(0.0),
                                        now,
                                    )
                                });
                            transition.retarget(
                                1.0,
                                now,
                                Duration::from_millis(u64::from(animation_ms)),
                            );
                            transition.value(now)
                        };
                        if let Some(DrawCommand::FillRect { color, .. }) =
                            scrim_index.and_then(|index| builder.frame.commands.get_mut(index))
                        {
                            color.a = (f32::from(color.a) * progress.clamp(0.0, 1.0)).round() as u8;
                        }
                        let command_start = builder.frame.commands.len();
                        // Deferred surfaces own a clone of the original tree.
                        // Rebuild the pointer-based focus path for that clone
                        // so an offscreen field can be found inside its scroll
                        // area and revealed when focused.
                        let mut floating_path = HashSet::new();
                        find_focus_path(&floating, builder.focus_probe, &mut floating_path);
                        builder.focus_path.extend(floating_path);
                        let mut travel = (0.0, 0.0);
                        if let Some(coach) = coach {
                            let radius = coach
                                .targets
                                .iter()
                                .find_map(|id| find_radius(&self.root, id))
                                .unwrap_or(crate::tokens().radius.control);
                            let ring = Rect::new(
                                anchor.x - COACH_RING_GAP,
                                anchor.y - COACH_RING_GAP,
                                anchor.width + COACH_RING_GAP * 2.0,
                                anchor.height + COACH_RING_GAP * 2.0,
                            );
                            let (ring, offset) = builder.coach_travel(&coach.key, ring, rect);
                            travel = offset;
                            if let Some(color) = coach.dim {
                                let right = ring.x + ring.width;
                                let bottom = ring.y + ring.height;
                                // Four rectangles around the target: no mask
                                // and no second pass over what is behind.
                                for rect in [
                                    Rect::new(0.0, 0.0, viewport.width, ring.y),
                                    Rect::new(0.0, bottom, viewport.width, viewport.height - bottom),
                                    Rect::new(0.0, ring.y, ring.x, ring.height),
                                    Rect::new(right, ring.y, viewport.width - right, ring.height),
                                ] {
                                    if rect.width > 0.0 && rect.height > 0.0 {
                                        builder.frame.commands.push(DrawCommand::FillRect {
                                            rect,
                                            color,
                                            radius: 0.0,
                                        });
                                    }
                                }
                            }
                            builder.frame.commands.push(DrawCommand::StrokeRect {
                                rect: ring,
                                color: coach.fill,
                                width: COACH_RING_WIDTH,
                                radius: (radius + COACH_RING_GAP)
                                    .min(ring.width / 2.0)
                                    .min(ring.height / 2.0),
                            });
                        }
                        let bubble_start = builder.frame.commands.len();
                        builder.place_translated(&floating, rect, Some(viewport_rect));
                        if let (Some(coach), Some(side)) = (coach, coach_side) {
                            builder.frame.commands.push(DrawCommand::FillTriangle {
                                points: coach_caret(anchor, rect, side),
                                color: coach.fill,
                            });
                        }
                        let slide_sign = if placement == PopoverPlacement::Bottom
                            || rect.y >= anchor.y + anchor.height {
                            1.0
                        } else {
                            -1.0
                        };
                        let raw_commands = builder.frame.commands[command_start..].to_vec();
                        if builder.reduced_motion || view_key.is_none() {
                            builder.popover_view_transitions.remove(&dismiss_id);
                        } else if let (Some(key), Some(previous)) = (
                            view_key.as_deref(),
                            builder.previous_popover_snapshots.get(&dismiss_id),
                        ) {
                            if previous.view_key.as_deref().is_some_and(|old| old != key) {
                                let mut opacity = ScalarTransition::new(0.0, now);
                                opacity.retarget(1.0, now, Duration::from_millis(180));
                                let mut height = ScalarTransition::new(previous.rect.height, now);
                                height.retarget(rect.height, now, Duration::from_millis(260));
                                builder.popover_view_transitions.insert(
                                    dismiss_id.clone(),
                                    PopoverViewTransition {
                                        from: previous.clone(),
                                        to_key: key.to_owned(),
                                        opacity,
                                        height,
                                    },
                                );
                            }
                        }
                        builder.popover_snapshots.insert(
                            dismiss_id.clone(),
                            PopoverSnapshot {
                                commands: raw_commands,
                                animation_ms,
                                close_animation_ms,
                                open_scale,
                                view_key: view_key.clone(),
                                rect,
                                slide_sign,
                                scrim: scrim
                                    .filter(|_| coach.is_none())
                                    .map(|color| (viewport_rect, color)),
                            },
                        );
                        if let Some(transition) = builder
                            .popover_view_transitions
                            .get(&dismiss_id)
                            .filter(|transition| {
                                view_key.as_deref() == Some(transition.to_key.as_str())
                            })
                        {
                            let fraction = transition.opacity.value(now).clamp(0.0, 1.0);
                            let mut shell_rect = rect;
                            shell_rect.height = transition.height.value(now).max(0.0);
                            if slide_sign < 0.0 {
                                shell_rect.y += rect.height - shell_rect.height;
                            }
                            let fresh = builder.frame.commands.split_off(command_start);
                            // Keep one opaque shell while the old and new
                            // content crossfade. Pointer regions are already
                            // laid out solely for the incoming view.
                            for mut command in fresh
                                .iter()
                                .filter(|command| popover_shell_command(command, rect))
                                .cloned()
                            {
                                morph_popover_shell(&mut command, shell_rect);
                                builder.frame.commands.push(command);
                            }
                            builder
                                .frame
                                .commands
                                .push(DrawCommand::PushClip(shell_rect));
                            if fraction < 1.0 {
                                for command in transition.from.commands.iter().filter(|command| {
                                    !popover_shell_command(command, transition.from.rect)
                                }) {
                                    let mut command = command.clone();
                                    animate_popover_command(
                                        &mut command,
                                        1.0 - fraction,
                                        -fraction * slide_sign * 10.0,
                                        1.0,
                                        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height),
                                    );
                                    builder.frame.commands.push(command);
                                }
                            }
                            if fraction > 0.0 {
                                for command in fresh
                                    .into_iter()
                                    .filter(|command| !popover_shell_command(command, rect))
                                {
                                    let mut command = command;
                                    animate_popover_command(
                                        &mut command,
                                        fraction,
                                        (1.0 - fraction) * slide_sign * 10.0,
                                        1.0,
                                        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height),
                                    );
                                    builder.frame.commands.push(command);
                                }
                            }
                            builder.frame.commands.push(DrawCommand::PopClip);
                        }
                        if progress < 1.0 {
                            let slide = (1.0 - progress) * slide_sign * 4.0;
                            let scale = open_scale + (1.0 - open_scale) * progress;
                            let origin =
                                Point::new(rect.x + rect.width / 2.0, rect.y + rect.height);
                            for (index, command) in
                                builder.frame.commands[command_start..].iter_mut().enumerate()
                            {
                                // A coach mark's ring stays on its target.
                                let slide = if command_start + index < bubble_start { 0.0 } else { slide };
                                animate_popover_command(command, progress, slide, scale, origin);
                            }
                        }
                        if travel != (0.0, 0.0) {
                            for command in &mut builder.frame.commands[bubble_start..] {
                                *command = command.moved(travel.0, travel.1, 1.0);
                            }
                        }
                    }
                    DeferredOverlay::Tooltip {
                        anchor,
                        title,
                        shortcut,
                        sections,
                        max_width,
                        placement,
                        colors,
                        options,
                    } => {
                        let start = builder.frame.commands.len();
                        builder.paint_tooltip(
                            anchor,
                            &title,
                            shortcut.as_deref(),
                            &sections,
                            max_width,
                            placement,
                            colors,
                            options,
                            viewport_rect,
                        );
                        // One hint shows at a time, so they share one fade:
                        // moving between neighbours does not blink.
                        builder.animate_surface("__tooltip", start, anchor, 0.0, true);
                    }
                }
                builder.measurement_scope = previous_scope;
            }
            if let Some((menu, rect, focused, pressed)) = context_menu_paint.as_ref().filter(|_| {
                self.context_menu
                    .as_ref()
                    .is_some_and(|menu| menu.valid_in(&builder.frame))
            }) {
                let old_focus = builder.focused;
                let old_pressed = builder.pressed;
                let old_hovered = builder.hovered;
                let old_keyboard = builder.keyboard_focus;
                builder.focused = focused.as_deref();
                builder.pressed = pressed.as_deref();
                builder.hovered = self
                    .hovered
                    .as_deref()
                    .filter(|id| id.starts_with("__context_menu:item:"));
                builder.keyboard_focus = focused.is_some();
                builder.frame.regions.push(HitRegion {
                    id: "__context_menu:surface".into(),
                    rect: *rect,
                    kind: HitKind::Surface,
                });
                let start = builder.frame.commands.len();
                builder.place(menu, *rect, Some(viewport_rect));
                builder.animate_surface("__context_menu", start, *rect, 1.0, true);
                builder.focused = old_focus;
                builder.pressed = old_pressed;
                builder.hovered = old_hovered;
                builder.keyboard_focus = old_keyboard;
            }
            ghost_range = builder.drag_ghost.take();
            (builder.frame, overlay_start)
        };
        // The dragged element follows the pointer above every layer. It
        // is captured once, so it survives its source scrolling away or
        // being recycled by a virtual list.
        if let (Some(drag), Some(pointer)) = (
            self.pointer_drag.as_mut().filter(|drag| drag.active),
            self.pointer,
        ) {
            if let Some(range) = ghost_range.filter(|_| drag.ghost.is_empty()) {
                drag.ghost = frame.commands[range].to_vec();
            }
            let (dx, dy) = (pointer.x - drag.origin.x, pointer.y - drag.origin.y);
            frame
                .commands
                .extend(drag.ghost.iter().map(|command| command.moved(dx, dy, 0.8)));
        }
        // Stable focus IDs may represent different virtual results after an
        // asynchronous refresh. Keep focus, but do not transfer a held press
        // to the replacement action. Unchanged aliases keep their capture.
        if self.pressed.as_deref().is_some_and(|id| {
            self.pressed_dispatch.as_deref() != Some(self.dispatch_id(id).as_str())
        }) {
            self.pressed = None;
        }
        if self.pressed.is_none() {
            self.pressed_dispatch = None;
        }
        if self.reduced_motion {
            self.popover_exits.clear();
        } else {
            for (id, paint) in std::mem::take(&mut self.popover_snapshots) {
                if observed_popovers.contains(&id)
                    || paint.close_animation_ms == Some(0)
                    || (paint.close_animation_ms.is_none() && paint.animation_ms == 0)
                {
                    continue;
                }
                let starting_opacity = self
                    .popover_transitions
                    .get(&id)
                    .map(|transition| transition.value(now))
                    .unwrap_or(1.0);
                let mut opacity = ScalarTransition::new(starting_opacity, now);
                opacity.retarget(
                    0.0,
                    now,
                    Duration::from_millis(u64::from(
                        paint
                            .close_animation_ms
                            .unwrap_or(paint.animation_ms.min(crate::tokens().motion.exit_ms)),
                    )),
                );
                self.popover_exits
                    .insert(id, PopoverExit { paint, opacity });
            }
            self.popover_exits
                .retain(|id, exit| !observed_popovers.contains(id) && exit.opacity.is_active(now));
            let mut exit_commands = Vec::new();
            for exit in self.popover_exits.values() {
                let opacity = exit.opacity.value(now);
                let slide = (1.0 - opacity) * exit.paint.slide_sign * 4.0;
                let scale = exit.paint.open_scale + (1.0 - exit.paint.open_scale) * opacity;
                let rect = exit.paint.rect;
                let origin = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height);
                if let Some((rect, mut color)) = exit.paint.scrim {
                    color.a = (f32::from(color.a) * opacity.clamp(0.0, 1.0)).round() as u8;
                    exit_commands.push(DrawCommand::FillRect {
                        rect,
                        color,
                        radius: 0.0,
                    });
                }
                for command in &exit.paint.commands {
                    let mut command = command.clone();
                    animate_popover_command(&mut command, opacity, slide, scale, origin);
                    exit_commands.push(command);
                }
            }
            // Above the ordinary scene, below every currently open popover.
            frame
                .commands
                .splice(overlay_start..overlay_start, exit_commands);
        }
        self.popover_snapshots = current_popover_snapshots;
        // Hover-open submenus are excluded from the static tree walk so their
        // dormant options cannot receive focus. Once a submenu is painted,
        // its controls must participate in the next frame's focus validation.
        for scope in &frame.popover_focus {
            for (id, kind) in &scope.focusables {
                if seen.insert(id.clone()) {
                    focus_order.push((id.clone(), *kind));
                }
            }
        }
        self.focus_order = focus_order
            .into_iter()
            .flat_map(|(id, kind)| {
                if kind == HitKind::Scroll && id.starts_with(VIRTUAL_FOCUS_PREFIX) {
                    frame
                        .virtual_lists
                        .get(&id[VIRTUAL_FOCUS_PREFIX.len()..])
                        .map(|scope| scope.focusables().cloned().collect::<Vec<_>>())
                        .unwrap_or_default()
                } else {
                    vec![(id, kind)]
                }
            })
            .collect();
        if let Some(id) = self.focused.as_ref().filter(|id| {
            !self
                .focus_order
                .iter()
                .any(|(candidate, _)| candidate == *id)
        }) {
            let next_focus = previous_popovers
                .iter()
                .rev()
                .find(|scope| {
                    scope
                        .focusables
                        .iter()
                        .any(|(candidate, _)| candidate == id)
                })
                .and_then(|scope| scope.trigger_id.as_ref())
                .filter(|trigger| {
                    self.focus_order
                        .iter()
                        .any(|(candidate, _)| candidate == *trigger)
                })
                .cloned();
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        }
        self.transitions
            .retain(|id, _| observed_buttons.contains(id));
        self.popover_transitions
            .retain(|id, _| observed_popovers.contains(id));
        self.replacements
            .retain(|id, _| observed_replacements.contains(id));
        // A slot is remembered while it is rendered, empty or not: content
        // that arrives in one seen empty is what `expand` opens.
        self.scalars.get_mut().retain(|_, state| state.seen == now);
        self.springs.get_mut().retain(|_, state| state.seen == now);
        self.inks.get_mut().retain(|_, (_, seen)| *seen == now);
        self.leaving.get_mut().retain(|_, (_, seen)| *seen == now);
        self.flow_heights.get_mut().retain(|_, state| {
            if !state.fraction.is_active(now) {
                // At rest by this frame's clock, whatever clock is read later.
                state.fraction = slot_spring(state.fraction.target(), now);
                if state.fraction.target() <= 0.0 {
                    state.commands.clear();
                }
            }
            state.seen == now
        });
        if frame.coach.as_ref().is_none_or(|coach| coach.missing) {
            self.coach_motion = None;
        }
        self.popover_view_transitions.retain(|id, state| {
            observed_popovers.contains(id)
                && (state.opacity.is_active(now) || state.height.is_active(now))
        });
        self.tab_indicators
            .retain(|id, _| observed_tabs.contains(id));
        self.skeleton_started
            .retain(|id, _| observed_skeletons.contains(id));
        self.shimmer_started
            .retain(|id, _| observed_shimmers.contains(id));
        self.spinner_started
            .retain(|id, _| observed_spinners.contains(id));
        self.record_motion
            .retain(|id, _| observed_records.contains(id));
        self.slider_motion
            .retain(|id, _| observed_sliders.contains(id));
        self.tooltip_triggers = observed_tooltips;
        self.scroll_transitions.retain(|id, motion| {
            if !frame.scroll_extents.contains_key(id) {
                self.scroll_offsets.insert(id.clone(), motion.target());
                false
            } else {
                motion.is_active(now)
            }
        });
        self.scrollbar_states.retain(|_, state| state.seen);
        self.frame = frame;
        self.finish_paste_layout();
        if previous_popovers != self.frame.popover_focus {
            self.invalidate_paste_targets();
        }
        self.closed_submenu_triggers
            .retain(|id| self.frame.submenu_triggers.contains(id));
        if self
            .context_menu
            .as_ref()
            .is_some_and(|menu| !menu.valid_in(&self.frame))
        {
            self.invalidate_paste_targets();
            self.context_menu = None;
        }
        if self
            .slider_drag
            .as_ref()
            .is_some_and(|drag| !self.frame.sliders.contains_key(&drag.id))
        {
            self.slider_drag = None;
        }
        let modal = self
            .frame
            .popover_focus
            .iter()
            .rev()
            .find(|scope| scope.modal);
        let had_modal = previous_popovers.iter().any(|scope| scope.modal);
        let modal_scopes = modal.map(|_| modal_popover_scopes(
            &self.frame.popover_focus, &self.closed_submenu_triggers,
        ));
        let modal_contains = |id: &str| modal_scopes.as_ref().is_none_or(|scopes| {
            scopes.iter().any(|scope| scope.focusables.iter().any(|(candidate, _)| candidate == id))
        });
        if let Some(scope) = modal {
            if !had_modal {
                self.modal_return_focus = scope
                    .return_focus
                    .as_ref()
                    .filter(|target| self.focus_order.iter().any(|(id, _)| id == *target))
                    .cloned()
                    .or_else(|| self.focused.clone());
            }
            if !self.focused.as_deref().is_some_and(modal_contains) {
                let next_focus = scope
                    .initial_focus
                    .as_ref()
                    .filter(|preferred| scope.focusables.iter().any(|(id, _)| id == *preferred))
                    .cloned()
                    .or_else(|| scope.focusables.first().map(|(id, _)| id.clone()));
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            }
        } else if had_modal {
            let next_focus = self.modal_return_focus.take().filter(|id| {
                self.focus_order
                    .iter()
                    .any(|(candidate, _)| candidate == id)
            });
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        }
        // A menu can be replaced by a floating form without a modal scrim.
        // Restore the departing scope first, then honor the new scope's
        // explicit target once. Rebuilding an already open form must retain
        // the user's caret or their focus on another control.
        let initial_focus = self
            .frame
            .popover_focus
            .iter()
            .rev()
            .filter(|scope| {
                !scope.modal
                    && scope.initial_focus.is_some()
                    && !previous_popovers
                        .iter()
                        .any(|previous| previous.dismiss_id == scope.dismiss_id)
            })
            .find_map(|scope| {
                scope.initial_focus.as_ref().filter(|preferred| {
                    scope.focusables.iter().any(|(id, kind)| {
                        id == *preferred
                            && matches!(
                                kind,
                                HitKind::Button
                                    | HitKind::TextInput
                                    | HitKind::TextArea
                                    | HitKind::CodeEditor
                            )
                    }) && modal_contains(preferred)
                })
            })
            .cloned();
        if let Some(id) = initial_focus.filter(|id| self.focused.as_ref() != Some(id)) {
            let next_focus = Some(id.clone());
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            self.pending_scroll_focus = Some(id);
            self.caret_blink.reset();
            self.validate_ime_owner(now);
            // The frame now records this scope, so the second layout
            // paints its focused field without applying initial focus again.
            return self.layout_at_with_hover_refresh(viewport, now, refresh_hover_paint);
        }
        // A coach mark on a new step: scroll its target into view and put
        // focus on the step's primary action, once.
        let coach = self.frame.coach.as_ref();
        if coach.map(|coach| &coach.key) != self.coach_key.as_ref() {
            self.coach_key = coach.map(|coach| coach.key.clone());
            if let Some(coach) = coach {
                let focus = coach.focus.clone().filter(|id| {
                    !coach.missing && self.focus_order.iter().any(|(candidate, _)| candidate == id)
                });
                self.pending_reveal = Some((coach.target.clone(), REVEAL_MARGIN, false));
                if focus.is_some() {
                    let next_focus = focus;
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                }
                return self.layout_at_with_hover_refresh(viewport, now, refresh_hover_paint);
            }
        }
        // A pending focus was this layout's probe; a reveal waits its turn.
        let located = match self.pending_scroll_focus.take() {
            Some(id) => Some((id, None, false)),
            None => self.pending_reveal.take().map(|(id, margin, start)| (id, Some(margin), start)),
        };
        if let Some((id, margin, start)) = located {
            let offsets = |tree: &Self| {
                tree.frame.scroll_focus_targets.get(&id).map(|target| {
                    target
                        .ancestors
                        .iter()
                        .map(|(scroll_id, ..)| scroll_id)
                        .chain([&target.scroll_id])
                        .map(|scroll_id| tree.scroll_offsets.get(scroll_id).copied().unwrap_or_default())
                        .collect::<Vec<_>>()
                })
            };
            let previous = offsets(self);
            self.reveal_scroll_focus(&id, margin.unwrap_or(0.0), start);
            let moved = offsets(self) != previous;
            if let Some(margin) = margin.filter(|_| moved) {
                // A scroll area that was itself out of view only gives the
                // control its place once the one around it has moved.
                self.pending_reveal.get_or_insert((id, margin, start));
            }
            if moved || self.pending_reveal.is_some() {
                return self.layout_at_with_hover_refresh(viewport, now, refresh_hover_paint);
            }
        }
        if let Some(target) = self.pending_center.take().and_then(|id| self.frame.scroll_focus_targets.get(&id)) {
            // Where the control was laid out, in the content's own measure.
            let place = target.rect.y + target.layout_offset;
            let middle = target.viewport.y + ((target.viewport.height - target.rect.height) / 2.0).max(0.0);
            let scroll_id = target.scroll_id.clone();
            self.set_scroll_target(&scroll_id, place - middle, now, true);
        }
        if self.pressed.as_ref().is_some_and(|id| {
            !self
                .frame
                .regions
                .iter()
                .any(|region| region.kind == HitKind::Button && region.id == *id)
        }) {
            self.pressed = None;
        }
        self.hovered_scrollbar = self.pointer.and_then(|point| {
            self.frame
                .hit_region(point)
                .filter(|region| region.kind == HitKind::Scrollbar)
                .map(|region| region.id.clone())
        });
        if self.pushes_moved.replace(false) {
            return self.layout_at_with_hover_refresh(viewport, now, refresh_hover_paint);
        }
        // Reconcile pointer state with the new layout: a scroll, navigation,
        // or popover can move the target without any physical pointer motion.
        if let Some(point) = self.pointer {
            let target = self.frame.hit_test(point).map(str::to_owned);
            if target != self.hovered {
                self.set_hovered(target, now);
                if refresh_hover_paint {
                    return self.layout_at_with_hover_refresh(viewport, now, false);
                }
            }
        }
        if self.static_selection.as_ref().is_some_and(|selection| {
            self.frame
                .text_blocks
                .get(selection.block)
                .is_none_or(|block| block.source != selection.source)
        }) {
            self.static_selection = None;
        }
        if self
            .scroll_drag
            .as_ref()
            .is_some_and(|drag| !self.frame.scrollbars.contains_key(&drag.id))
        {
            self.scroll_drag = None;
        }
        if self.scroll_button_hold.as_ref().is_some_and(|hold| {
            !self.frame.scrollbars.contains_key(&hold.id)
                || !self.can_scroll_step(&hold.id, hold.direction)
        }) {
            self.scroll_button_hold = None;
        }
        self.caret_blink.finish_frame(self.focused.as_deref());
        self.trim_code_view_cache();
        self.validate_ime_owner(now);
        self.paste_layout_pending = false;
        self.frame.clone()
    }

    fn set_hovered(&mut self, target: Option<String>, now: Instant) {
        if target != self.hovered {
            self.tooltip_hover_since = target.as_ref().map(|id| (id.clone(), now));
            self.hovered = target;
        }
    }

    /// The frame of the last layout.
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    /// The Vim mode a code editor is in. `None` when no editor has this ID or
    /// the editor does not use Vim keys.
    pub fn code_editor_vim_mode(&self, id: &str) -> Option<crate::VimMode> {
        self.code_editors.get(id).and_then(|view| view.vim_mode())
    }
    /// The Vim mode of the code editor that has the focus, as
    /// [`Self::code_editor_vim_mode`].
    pub fn focused_code_editor_vim_mode(&self) -> Option<crate::VimMode> {
        self.focused
            .as_deref()
            .and_then(|id| self.code_editor_vim_mode(id))
    }
    /// Hosts consult this before resolving workbench keybindings. An active
    /// popup owns keyboard input ahead of the underlying modal editor.
    pub fn focused_code_editor_vim_consumes(&self, key: Key, ctrl: bool) -> bool {
        self.active_popover_scope().is_none()
            && self
                .focused
                .as_deref()
                .and_then(|id| self.code_editors.get(id))
                .is_some_and(|view| view.vim_consumes(key, ctrl))
    }

    /// Release retained viewport and lexical worker state when a host closes a tab.
    pub fn forget_code_view(&mut self, id: &str) {
        self.code_editors.remove(id);
        self.code_diffs.remove(id);
        self.code_view_last_seen.remove(id);
        if self.focused.as_deref() == Some(id) {
            let next_focus = None;
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        }
    }
    /// Retain a bounded viewport cache. Buffers and unsaved edits belong to the
    /// host controller; evicting an invisible view never discards source text.
    fn trim_code_view_cache(&mut self) {
        self.code_view_epoch = self.code_view_epoch.saturating_add(1);
        let visible: HashSet<_> = self
            .frame
            .regions
            .iter()
            .filter(|region| matches!(region.kind, HitKind::CodeEditor | HitKind::CodeDiff))
            .map(|region| region.id.clone())
            .collect();
        for id in &visible {
            self.code_view_last_seen
                .insert(id.clone(), self.code_view_epoch);
        }
        for (editor, cap) in [(true, 16usize), (false, 64usize)] {
            let keys: Vec<_> = if editor {
                self.code_editors.keys().cloned().collect()
            } else {
                self.code_diffs.keys().cloned().collect()
            };
            let excess = keys.len().saturating_sub(cap);
            if excess == 0 {
                continue;
            }
            let mut dormant: Vec<_> = keys
                .into_iter()
                .filter(|id| !visible.contains(id))
                .map(|id| (self.code_view_last_seen.get(&id).copied().unwrap_or(0), id))
                .collect();
            dormant.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
            for (_, id) in dormant.into_iter().take(excess) {
                self.forget_code_view(&id);
            }
        }
    }

    /// Instrumentation for lifecycle/end-to-end checks; not a memory estimate.
    pub fn retained_code_view_counts(&self) -> (usize, usize) {
        (self.code_editors.len(), self.code_diffs.len())
    }
    /// Ask a code editor to bring its caret into view. False when no editor
    /// has this ID.
    pub fn reveal_code_caret(&mut self, id: &str) -> bool {
        if let Some(view) = self.code_editors.get_mut(id) {
            view.viewport.reveal_caret = true;
            true
        } else {
            false
        }
    }

    /// Reveal the start of an exact review hunk, including a deletion at EOF.
    /// This never changes the editable selection or focus.
    pub fn reveal_code_review_hunk(&mut self, id: &str, index: usize) -> bool {
        self.code_editors
            .get_mut(id)
            .is_some_and(|view| view.reveal_review_hunk(index))
    }

    /// Exact snapshot and active hunk from the editor's most recent layout.
    /// Scrolling and caret movement update the same position used by its pill.
    pub fn code_review_current_hunk(&self, id: &str) -> Option<crate::CodeReviewPosition> {
        self.code_editors
            .get(id)
            .and_then(|view| view.current_review_hunk())
    }

    fn scroll_target(&self, id: &str) -> f32 {
        self.scroll_transitions
            .get(id)
            .map(ScalarTransition::target)
            .unwrap_or_else(|| self.scroll_offset(id))
            .clamp(0.0, self.scroll_max(id))
    }

    fn set_scroll_target(&mut self, id: &str, target: f32, now: Instant, smooth: bool) {
        let max = self.scroll_max(id);
        let target = target.clamp(0.0, max);
        let policy = self
            .frame
            .scroll_motion
            .get(id)
            .copied()
            .unwrap_or_default();
        if !smooth || self.reduced_motion || policy.duration_ms == 0 {
            self.scroll_transitions.remove(id);
            self.scroll_offsets.insert(id.to_owned(), target);
            return;
        }
        let current = self
            .scroll_transitions
            .get(id)
            .map(|motion| motion.value(now))
            .unwrap_or_else(|| self.scroll_offset(id))
            .clamp(0.0, max);
        self.scroll_offsets.insert(id.to_owned(), current);
        let motion = self
            .scroll_transitions
            .entry(id.to_owned())
            .or_insert_with(|| ScalarTransition::new(current, now));
        *motion = motion.easing(policy.easing);
        motion.retarget(
            target,
            now,
            Duration::from_millis(u64::from(policy.duration_ms)),
        );
    }

    fn can_scroll_step(&self, id: &str, direction: i8) -> bool {
        let target = self.scroll_target(id);
        if direction < 0 {
            target > 0.001
        } else {
            target < self.scroll_max(id) - 0.001
        }
    }

    fn cancel_scroll_hold(&mut self, id: &str) {
        if self
            .scroll_button_hold
            .as_ref()
            .is_some_and(|hold| hold.id == id)
        {
            self.scroll_button_hold = None;
        }
    }

    fn repeat_scroll_button(&mut self, now: Instant) {
        let Some(mut hold) = self.scroll_button_hold.take() else {
            return;
        };
        let Some(geometry) = self.frame.scrollbars.get(&hold.id).copied() else {
            return;
        };
        if !self.can_scroll_step(&hold.id, hold.direction) {
            return;
        }
        let over_button = self
            .pointer
            .is_some_and(|point| geometry.button_at(point) == Some(hold.direction));
        if !over_button {
            hold.repeat_at = now + SCROLL_REPEAT_INTERVAL;
        } else if now >= hold.repeat_at {
            // Catch up a few missed refreshes without replaying a long pause
            // as an unbounded jump through the content.
            let repeats = (1 + now.saturating_duration_since(hold.repeat_at).as_millis()
                / SCROLL_REPEAT_INTERVAL.as_millis())
            .min(8) as f32;
            let target = self.scroll_target(&hold.id)
                + f32::from(hold.direction) * SCROLL_BUTTON_STEP * repeats;
            self.set_scroll_target(&hold.id, target, now, true);
            hold.repeat_at = now + SCROLL_REPEAT_INTERVAL;
        }
        if self.can_scroll_step(&hold.id, hold.direction) {
            self.scroll_button_hold = Some(hold);
        }
    }

    /// Take one input event at the current time and return what it caused,
    /// in order; see [`Self::handle_event_at`].
    pub fn handle_event(&mut self, event: UiEvent) -> Vec<UiAction> {
        self.handle_event_at(event, Instant::now())
    }

    /// Scroll what scrolls behind the focused control, or under the pointer,
    /// or the largest scrolling area there is, by most of its height: what
    /// Page Up and Page Down do. False when nothing there scrolls.
    fn scroll_by_page(&mut self, down: bool, now: Instant) -> bool {
        let middle = |rect: Rect| Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let scrolls = |region: &&HitRegion| region.kind == HitKind::Scroll;
        let at = self
            .focused
            .as_deref()
            .and_then(|id| self.frame.regions.iter().find(|region| region.id == id && region.kind != HitKind::Scroll))
            .map(|region| middle(region.rect))
            .or(self.pointer)
            .filter(|at| self.frame.regions.iter().filter(scrolls).any(|region| region.rect.contains(*at)))
            .or_else(|| {
                self.frame
                    .regions
                    .iter()
                    .filter(scrolls)
                    .max_by(|a, b| (a.rect.width * a.rect.height).total_cmp(&(b.rect.width * b.rect.height)))
                    .map(|region| middle(region.rect))
            });
        let Some(at) = at else { return false };
        // The innermost area there: the last one placed.
        let Some(page) = self.frame.regions.iter().rev().filter(scrolls).find(|region| region.rect.contains(at)).map(|region| region.rect.height)
        else {
            return false;
        };
        // The wheel's path, without the pointer having moved.
        let pointer = self.pointer;
        let step = page * 0.9;
        // As the wheel has it: a negative delta moves down the content.
        self.scroll_regular_regions(at, if down { -step } else { step }, true, now);
        self.pointer = pointer;
        true
    }

    // Input must not reach behind a floating surface at the pointer, or
    // anywhere behind a modal, even outside the modal's card.
    fn floating_surface_floor(&self, position: Point) -> usize {
        let modal_surface = self
            .frame
            .popover_focus
            .iter()
            .rev()
            .find(|scope| scope.modal)
            .map(|scope| format!("popover_surface:{}", scope.dismiss_id));
        self
            .frame
            .regions
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, region)| {
                (region.kind == HitKind::Surface
                    && (((region.id.starts_with("popover_surface:")
                        || region.id == "__context_menu:surface")
                        && region.rect.contains(position))
                        || modal_surface.as_deref() == Some(region.id.as_str())))
                .then_some(index)
            })
            .unwrap_or(0)
    }

    // Native diff viewports own their horizontal scroll. Their unconsumed
    // vertical delta uses this same hierarchy as an ordinary nested scroll,
    // including smooth targets and floating/modal containment.
    fn scroll_regular_regions(
        &mut self,
        position: Point,
        delta_y: f32,
        smooth: bool,
        now: Instant,
    ) {
        self.pointer = Some(position);
        if !delta_y.is_finite() || delta_y == 0.0 {
            return;
        }
        let mut remaining = delta_y;
        self.scroll_button_hold = None;
        let floor = self.floating_surface_floor(position);
        let regions: Vec<_> = self
            .frame
            .regions
            .iter()
            .skip(floor)
            .rev()
            .filter(|region| {
                matches!(region.kind, HitKind::Scroll | HitKind::TextArea)
                    && region.rect.contains(position)
            })
            .cloned()
            .collect();
        for region in regions {
            if region.kind == HitKind::TextArea {
                if let Some(input) = self.inputs.get_mut(&region.id) {
                    let before = input.scroll_line;
                    input.scroll_lines(remaining);
                    if input.scroll_line != before {
                        break;
                    }
                }
            } else {
                let before = if smooth {
                    self.scroll_target(&region.id)
                } else {
                    self.scroll_transitions
                        .get(&region.id)
                        .map(|motion| motion.value(now))
                        .unwrap_or_else(|| self.scroll_offset(&region.id))
                };
                let next = (before - remaining).clamp(0.0, self.scroll_max(&region.id));
                if next != before || !smooth {
                    remaining += next - before;
                    self.set_scroll_target(&region.id, next, now, smooth);
                    if remaining.abs() < 0.001 {
                        break;
                    }
                }
            }
        }
    }

    fn handle_code_event(&mut self, event: &UiEvent, now: Instant) -> Option<Vec<UiAction>> {
        let measurer = self.text_measurer.clone();
        let char_width = self.metrics.char_width;
        let measure = |size: f32, text: &str| {
            measured_width(
                measurer.as_deref(),
                text,
                size,
                FontFamily::Monospace,
                char_width * size / DEFAULT_FONT_SIZE,
            )
        };
        match event {
            UiEvent::ImePreedit(text) | UiEvent::ImeCommit(text) => {
                let id = self.focused.as_ref()?;
                let editor = self.code_editors.get_mut(id)?;
                Some(if matches!(event, UiEvent::ImePreedit(_)) {
                    editor.composition(Some(text), None, now)
                } else {
                    editor.composition(None, Some(text), now)
                })
            }
            UiEvent::ImeDeleteSurrounding {
                before_chars,
                after_chars,
            } => Some(
                self.code_editors
                    .get_mut(self.focused.as_ref()?)?
                    .delete_surrounding(*before_chars, *after_chars, now),
            ),
            UiEvent::ImeCancelled => Some(
                self.code_editors
                    .get_mut(self.focused.as_ref()?)?
                    .composition(None, None, now),
            ),
            UiEvent::PointerDown(point)
            | UiEvent::PointerDownModified {
                position: point, ..
            }
            | UiEvent::PointerDownEditor {
                position: point, ..
            } => {
                let hit = self.frame.hit_region(*point)?;
                if hit.kind == HitKind::CodeDiff {
                    if self
                        .code_diffs
                        .get_mut(&hit.id)?
                        .viewport
                        .begin_scrollbar(*point)
                    {
                        return Some(Vec::new());
                    }
                    return None;
                }
                if hit.kind != HitKind::CodeEditor {
                    return None;
                }
                let id = hit.id.clone();
                let next_focus = Some(id.clone());
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                self.pointer = Some(*point);
                self.keyboard_focus = false;
                self.static_selection = None;
                let (extend, add, rectangle, clicks) = match event {
                    UiEvent::PointerDownEditor {
                        shift,
                        alt,
                        click_count,
                        ..
                    } => (*shift, *alt && !*shift, *alt && *shift, *click_count),
                    UiEvent::PointerDownModified { shift, .. } => (*shift, false, false, 1),
                    _ => (false, false, false, 1),
                };
                let state = self.code_editors.get_mut(&id)?;
                let size = state.editor.font_size;
                Some(state.pointer_modified(*point, extend, add, rectangle, clicks, true,
                    &|text| measure(size, text)))
            }
            UiEvent::PointerMove(point) | UiEvent::PointerUp(point) => {
                if let Some(viewport) = self
                    .code_diffs
                    .values_mut()
                    .find(|view| view.viewport.scrollbar_drag.is_some())
                {
                    viewport
                        .viewport
                        .drag_scrollbar(*point, matches!(event, UiEvent::PointerMove(_)));
                    return Some(Vec::new());
                }
                let state = self.code_editors.values_mut().find(|view| {
                    view.viewport.dragging || view.viewport.scrollbar_drag.is_some()
                })?;
                self.pointer = Some(*point);
                let dragging = matches!(event, UiEvent::PointerMove(_));
                let size = state.editor.font_size;
                Some(state.point(*point, true, dragging, &|text| measure(size, text)))
            }
            UiEvent::Scroll { position, delta_y }
            | UiEvent::ScrollDirect { position, delta_y }
            | UiEvent::Scroll2D {
                position, delta_y, ..
            } => {
                let hit = self.frame.hit_region(*position)?;
                let (x, y) = if let UiEvent::Scroll2D {
                    delta_x,
                    delta_y,
                    shift,
                    ..
                } = event
                {
                    if *shift && *delta_x == 0.0 {
                        (*delta_y, 0.0)
                    } else {
                        (*delta_x, *delta_y)
                    }
                } else {
                    (0.0, *delta_y)
                };
                match hit.kind {
                    HitKind::CodeEditor => {
                        self.code_editors.get_mut(&hit.id)?.viewport.scroll(x, y)
                    }
                    HitKind::Button => {
                        // Only controls emitted by this exact editor viewport
                        // forward the wheel. Never search through arbitrary
                        // menu, popover or other overlay hit regions.
                        let view = self
                            .code_editors
                            .values_mut()
                            .find(|view| view.owns_structure_control(&hit.id, *position))?;
                        view.viewport.scroll(x, y);
                    }
                    HitKind::CodeDiff => {
                        let viewport = &mut self.code_diffs.get_mut(&hit.id)?.viewport;
                        let before = viewport.scroll_y;
                        viewport.scroll(x, y);
                        // A read-only diff embedded in the transcript consumes
                        // only the vertical distance it can move. Horizontal
                        // intent remains in the diff; vertical overflow uses
                        // the enclosing scroll regions' normal motion contract.
                        let remaining = y + viewport.scroll_y - before;
                        let smooth = matches!(
                            event,
                            UiEvent::Scroll { .. } | UiEvent::Scroll2D { direct: false, .. }
                        );
                        self.scroll_regular_regions(*position, remaining, smooth, now);
                    }
                    _ => return None,
                }
                Some(Vec::new())
            }
            UiEvent::Text(_) | UiEvent::Paste(_) | UiEvent::Key { .. } => {
                let id = self.focused.as_deref()?;
                if !self
                    .frame
                    .regions
                    .iter()
                    .any(|region| region.id == id && region.kind == HitKind::CodeEditor)
                {
                    return None;
                }
                if matches!(
                    event,
                    UiEvent::Key {
                        key: Key::Escape,
                        ..
                    }
                ) && (self.active_popover_scope().is_some()
                    || !self.code_editors.get(id).is_some_and(|view| {
                        let shared = view.editor.controller.shared_buffer();
                        let active = view.editor.vim_mode
                            || shared.lock().is_ok_and(|buffer| {
                                buffer.has_snippet() || buffer.selections().count() > 1
                            });
                        active
                    }))
                {
                    return None;
                }
                let state = self.code_editors.get_mut(id)?;
                if state.editor.read_only && matches!(event, UiEvent::Key { key: Key::Tab, .. }) {
                    return None;
                }
                let size = state.editor.font_size;
                Some(state.input(event, now, &|text| measure(size, text)))
            }
            _ => None,
        }
    }

    /// Dispatch with the same monotonic clock used by `layout_at`.
    pub fn handle_event_at(&mut self, event: UiEvent, now: Instant) -> Vec<UiAction> {
        let event = match event {
            UiEvent::PasteCaptured { target, text } => {
                return self.paste_captured(target, text, now)
            }
            event => event,
        };
        self.validate_ime_owner(now);
        if self.reject_ime_event(&event, now) {
            return Vec::new();
        }
        let event = match event {
            UiEvent::FilesHovered { position } => {
                return vec![UiAction::FilesHovered {
                    position,
                    over: self.drop_target_at(position, ""),
                }]
            }
            UiEvent::FilesDropped { position, paths } => {
                return vec![UiAction::FilesDropped {
                    position,
                    target: self.drop_target_at(position, ""),
                    paths,
                }]
            }
            other => other,
        };
        if let Some(actions) = self.handle_drag_event(&event) {
            return actions;
        }
        let pressed_at = match &event {
            UiEvent::PointerDown(position)
            | UiEvent::PointerDownModified { position, .. }
            | UiEvent::PointerDownEditor { position, .. } => Some(*position),
            _ => None,
        };
        let cancelled = matches!(event, UiEvent::PointerLeave)
            .then(|| self.pointer_drag.take())
            .flatten()
            .filter(|drag| drag.active);
        let dismissed = self.dismiss_by_scroll(&event);
        let mut actions = self.handle_event_inner(event, now);
        if let Some(dismissed) = dismissed {
            actions.insert(0, dismissed);
        }
        if let Some(drag) = cancelled {
            actions.push(UiAction::DragCancelled { id: drag.id });
        }
        if let Some(origin) = pressed_at {
            self.pointer_drag = self
                .pressed
                .as_ref()
                .filter(|id| self.frame.draggables.contains(*id))
                .map(|id| PointerDrag {
                    id: id.clone(),
                    origin,
                    active: false,
                    ghost: Vec::new(),
                });
        }
        self.validate_ime_owner(now);
        actions
    }

    /// A wheel or a trackpad outside an open menu or popover closes it, as a
    /// press on its backdrop does, and still scrolls what is under it. A
    /// modal stays open.
    fn dismiss_by_scroll(&mut self, event: &UiEvent) -> Option<UiAction> {
        let position = scroll_at(event)?;
        let backdrop = self.frame.press_region(position).filter(|region| region.kind == HitKind::Button)?;
        let scope = self.frame.popover_focus.iter().find(|scope| scope.dismiss_id == backdrop.id && !scope.modal)?;
        // A control of the surface may report the same ID as its backdrop.
        let surface = format!("popover_surface:{}", scope.dismiss_id);
        if self.frame.regions.iter().any(|region| region.id == surface && region.rect.contains(position)) {
            return None;
        }
        let id = self.dispatch_id(&backdrop.id);
        let next_focus = scope.trigger_id.clone();
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        self.keyboard_focus = false;
        Some(UiAction::Clicked { id })
    }

    /// Owns the pointer while a drag is active so the source never clicks and
    /// controls under the pointer do not react as if hovered.
    fn handle_drag_event(&mut self, event: &UiEvent) -> Option<Vec<UiAction>> {
        let drag = self.pointer_drag.as_mut()?;
        match *event {
            UiEvent::PointerMove(position) => {
                let mut actions = Vec::new();
                if !drag.active {
                    let (dx, dy) = (position.x - drag.origin.x, position.y - drag.origin.y);
                    if dx.hypot(dy) < DRAG_THRESHOLD {
                        return None;
                    }
                    drag.active = true;
                    actions.push(UiAction::DragStarted {
                        id: drag.id.clone(),
                        position: drag.origin,
                    });
                }
                let id = drag.id.clone();
                self.pressed = None;
                self.pressed_dispatch = None;
                self.pointer = Some(position);
                actions.push(UiAction::DragMoved {
                    over: self.drop_target_at(position, &id),
                    id,
                    position,
                });
                Some(actions)
            }
            UiEvent::PointerUp(position) => {
                let drag = self.pointer_drag.take()?;
                drag.active.then(|| {
                    vec![UiAction::Dropped {
                        target: self.drop_target_at(position, &drag.id),
                        id: drag.id,
                        position,
                    }]
                })
            }
            UiEvent::Key {
                key: Key::Escape, ..
            } if drag.active => {
                let drag = self.pointer_drag.take()?;
                Some(vec![UiAction::DragCancelled { id: drag.id }])
            }
            _ => None,
        }
    }

    fn drop_target_at(&self, point: Point, source: &str) -> Option<String> {
        self.frame
            .regions
            .iter()
            .skip(self.floating_surface_floor(point))
            .rev()
            .find(|region| {
                region.rect.contains(point)
                    && region.id != source
                    && self.frame.drop_targets.contains(&region.id)
            })
            .map(|region| region.id.clone())
    }

    fn handle_event_inner(&mut self, event: UiEvent, now: Instant) -> Vec<UiAction> {
        if let Some(actions) = self.handle_context_menu_event(&event, now) {
            return actions;
        }
        if matches!(
            &event,
            UiEvent::Text(_)
                | UiEvent::Paste(_)
                | UiEvent::Key { .. }
                | UiEvent::PointerDown(_)
                | UiEvent::PointerDownModified { .. }
                | UiEvent::PointerDownEditor { .. }
        ) {
            self.caret_blink.reset();
        }
        if let Some(actions) = self.handle_code_event(&event, now) {
            return actions;
        }
        if self.handle_horizontal_scroll(&event) {
            return Vec::new();
        }
        let event = match event {
            UiEvent::ImeCommit(text) => UiEvent::Text(text),
            UiEvent::ImePreedit(_)
            | UiEvent::ImeCancelled
            | UiEvent::ImeDeleteSurrounding { .. } => return Vec::new(),
            UiEvent::PointerDownModified { position, .. }
            | UiEvent::PointerDownEditor { position, .. } => UiEvent::PointerDown(position),
            UiEvent::Scroll2D {
                position,
                delta_y,
                direct,
                ..
            } => {
                if direct {
                    UiEvent::ScrollDirect { position, delta_y }
                } else {
                    UiEvent::Scroll { position, delta_y }
                }
            }
            UiEvent::Paste(text) => UiEvent::Text(text),
            other => other,
        };
        let mut actions = Vec::new();
        match event {
            UiEvent::ContextMenu { .. }
            | UiEvent::ContextMenuKeyboard
            | UiEvent::PasteCaptured { .. }
            | UiEvent::PasteTo { .. } => {
                unreachable!("context menu events handled before ordinary controls")
            }
            UiEvent::ImePreedit(_)
            | UiEvent::ImeCommit(_)
            | UiEvent::ImeCancelled
            | UiEvent::ImeDeleteSurrounding { .. }
            | UiEvent::PointerDownModified { .. }
            | UiEvent::PointerDownEditor { .. }
            | UiEvent::Scroll2D { .. }
            | UiEvent::Paste(_) => {
                unreachable!("additive editor events normalized above")
            }
            UiEvent::FilesHovered { .. } | UiEvent::FilesDropped { .. } => {
                unreachable!("external file events resolved before ordinary controls")
            }
            UiEvent::PointerMove(point) => {
                let moved = self.pointer != Some(point);
                self.pointer = Some(point);
                let target = self.frame.hit_test(point).map(str::to_owned);
                // Menus track a single active row. Moving the pointer takes
                // over from keyboard navigation without stealing focus from
                // inputs inside rich popovers or reacting to a stationary mouse.
                if moved {
                    // Focus on a submenu's row or in a submenu holds that one
                    // open; the pointer onto another submenu's row takes over.
                    let holds_submenu = self.focused.as_ref().is_some_and(|focused| {
                        self.frame.submenu_triggers.contains(focused)
                            || self.frame.popover_focus.iter().any(|scope| {
                                scope.submenu
                                    && scope.focusables.iter().any(|(id, _)| id == focused)
                            })
                    });
                    if let Some(id) = target.as_ref().filter(|id| {
                        self.frame.action_menu_items.contains(*id)
                            || holds_submenu && self.frame.submenu_triggers.contains(*id)
                    }) {
                        let next_focus = Some(id.clone());
                        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                        self.closed_submenu_triggers.remove(id);
                        self.keyboard_focus = false;
                    } else if self
                        .focused
                        .as_ref()
                        .is_some_and(|id| self.frame.action_menu_items.contains(id))
                    {
                        let next_focus = self
                            .active_popover_scope()
                            .and_then(|scope| scope.trigger_id.clone());
                        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                        self.keyboard_focus = false;
                    }
                }
                if target.as_deref() != self.focused.as_deref() {
                    self.keyboard_tooltip_focus = false;
                }
                self.set_hovered(target, now);
                self.hovered_scrollbar = self
                    .frame
                    .hit_region(point)
                    .filter(|region| region.kind == HitKind::Scrollbar)
                    .map(|region| region.id.clone());
                self.drag_horizontal_pan(point, now);
                if let Some(drag) = self.scroll_drag.as_ref() {
                    if let Some(geometry) = self.frame.scrollbars.get(&drag.id) {
                        self.scroll_transitions.remove(&drag.id);
                        self.scroll_offsets
                            .insert(drag.id.clone(), geometry.offset_at(point.y, drag.grab_y));
                    }
                }
                if let Some(drag) = self.slider_drag.as_mut() {
                    let fraction = drag.geometry.fraction_at(point);
                    let y = drag.geometry.y_at(point.y);
                    if fraction != drag.fraction || y != drag.y {
                        drag.fraction = fraction;
                        drag.y = y;
                        actions.push(slider_action(drag.id.clone(), fraction, y, false));
                    }
                }
                if let Some(selection) = self.static_selection.as_mut().filter(|s| s.dragging) {
                    if let Some(block) = self.frame.text_blocks.get(selection.block) {
                        if block.source == selection.source {
                            selection.cursor = block.offset_at(
                                point,
                                self.text_measurer.as_deref(),
                                self.metrics,
                                self.sans_metrics,
                            );
                        }
                    }
                }
            }
            UiEvent::PointerLeave => {
                for editor in self.code_editors.values_mut() {
                    editor.viewport.cancel_pointer();
                }
                for diff in self.code_diffs.values_mut() {
                    diff.viewport.cancel_pointer();
                }
                if let Some(drag) = self.slider_drag.take() {
                    let snapped = drag.geometry.snap(drag.fraction);
                    if !self.reduced_motion && drag.geometry.animated {
                        let motion = self
                            .slider_motion
                            .entry(drag.id.clone())
                            .or_insert_with(|| SliderMotion::new(now));
                        motion.displayed_fraction = drag.fraction;
                        motion.snap = None;
                        motion.animate_to(
                            snapped,
                            now,
                            drag.geometry.ultra_available
                                && snapped >= 1.0
                                && drag.geometry.fraction < 1.0,
                        );
                    }
                    actions.push(slider_action(drag.id.clone(), snapped, drag.y, true));
                }
                self.pointer = None;
                self.set_hovered(None, now);
                self.hovered_scrollbar = None;
                self.pressed = None;
                self.scroll_drag = None;
                self.horizontal_pan = None;
                self.scroll_button_hold = None;
                self.keyboard_tooltip_focus = false;
                if let Some(selection) = self.static_selection.as_mut() {
                    selection.dragging = false;
                }
            }
            UiEvent::PointerDown(point) => {
                self.scroll_button_hold = None;
                self.scroll_drag = None;
                self.horizontal_pan = None;
                self.pointer = Some(point);
                self.set_hovered(self.frame.hit_test(point).map(str::to_owned), now);
                self.keyboard_focus = false;
                self.keyboard_tooltip_focus = false;
                self.tooltip_hover_since = None;
                let hit = self.frame.press_region(point).cloned();
                let dismiss_scope = hit.as_ref().and_then(|region| {
                    self.frame
                        .popover_focus
                        .iter()
                        .rev()
                        .find(|scope| scope.dismiss_id == region.id)
                });
                let dismiss_trigger = dismiss_scope.and_then(|scope| scope.trigger_id.clone());
                self.static_selection = None;
                if hit
                    .as_ref()
                    .is_some_and(|region| region.kind == HitKind::Surface)
                    && dismiss_scope.is_some_and(|scope| scope.modal)
                {
                    self.keyboard_focus = true;
                    self.pressed = None;
                    return actions;
                }
                if let Some((id, geometry)) = hit.as_ref().and_then(|region| {
                    self.frame
                        .sliders
                        .get(&region.id)
                        .copied()
                        .map(|geometry| (region.id.clone(), geometry))
                }) {
                    let fraction = geometry.fraction_at(point);
                    let y = geometry.y_at(point.y);
                    self.slider_drag = Some(SliderDrag {
                        id: id.clone(),
                        geometry,
                        fraction,
                        y,
                    });
                    // An entry in `slider_motion` keeps the window repainting,
                    // so only the animated effort slider gets one.
                    if geometry.animated {
                        let motion = self
                            .slider_motion
                            .entry(id.clone())
                            .or_insert_with(|| SliderMotion::new(now));
                        motion.last_sample = now;
                        motion.snap = None;
                        motion.burst_started = None;
                        motion.displayed_fraction = fraction;
                    }
                    let next_focus = Some(id.clone());
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                    self.pressed = None;
                    actions.push(slider_action(id, fraction, y, false));
                    return actions;
                }
                if let Some(pan) = Self::horizontal_pan_at(&self.frame, &mut self.horizontal_scrolls, point, now) {
                    let claimed = pan.claimed;
                    self.horizontal_pan = Some(pan);
                    if claimed {
                        return actions;
                    }
                }
                if let Some(region) = hit
                    .as_ref()
                    .filter(|region| region.kind == HitKind::Scrollbar)
                {
                    if let Some(geometry) = self.frame.scrollbars.get(&region.id).copied() {
                        if let Some(direction) = geometry.button_at(point) {
                            if self.can_scroll_step(&region.id, direction) {
                                let target = self.scroll_target(&region.id)
                                    + f32::from(direction) * SCROLL_BUTTON_STEP;
                                self.set_scroll_target(&region.id, target, now, true);
                                self.scroll_button_hold = Some(ScrollButtonHold {
                                    id: region.id.clone(),
                                    direction,
                                    repeat_at: now + SCROLL_REPEAT_DELAY,
                                });
                            }
                        } else if geometry.thumb.contains(point) {
                            // Grab the painted thumb, cancelling any pending
                            // wheel destination. Its position remains attached
                            // to the pointer for the complete drag.
                            self.scroll_transitions.remove(&region.id);
                            self.scroll_drag = Some(ScrollDrag {
                                id: region.id.clone(),
                                grab_y: point.y - geometry.thumb.y,
                            });
                        } else {
                            let direction = if point.y < geometry.thumb.y {
                                -1.0
                            } else {
                                1.0
                            };
                            let target = self.scroll_target(&region.id)
                                + direction * geometry.rail.height * 0.9;
                            self.set_scroll_target(&region.id, target, now, true);
                        }
                        self.hovered_scrollbar = Some(region.id.clone());
                    }
                    self.pressed = None;
                    return actions;
                }
                if !hit.as_ref().is_some_and(|region| {
                    matches!(
                        region.kind,
                        HitKind::Button | HitKind::TextInput | HitKind::TextArea
                    )
                }) {
                    if let Some((index, block)) = self
                        .frame
                        .text_blocks
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, block)| block.contains(point))
                    {
                        let offset = block.offset_at(
                            point,
                            self.text_measurer.as_deref(),
                            self.metrics,
                            self.sans_metrics,
                        );
                        self.static_selection = Some(StaticSelection {
                            block: index,
                            source: block.source.clone(),
                            anchor: offset,
                            cursor: offset,
                            dragging: true,
                        });
                    }
                }
                self.pressed = hit
                    .as_ref()
                    .filter(|region| region.kind == HitKind::Button)
                    .map(|region| region.id.clone());
                self.pressed_dispatch = self.pressed.as_deref().map(|id| self.dispatch_id(id));
                let next_focus =
                    hit.as_ref()
                        .filter(|region| {
                            (matches!(
                                region.kind,
                                HitKind::Button | HitKind::TextInput | HitKind::TextArea
                            ) || (region.kind == HitKind::CodeDiff
                                && self.focus_order.iter().any(|(id, kind)| {
                                    id == &region.id && *kind == HitKind::CodeDiff
                                })))
                                && dismiss_scope.is_none()
                        })
                        .map(|region| region.id.clone())
                        .or(dismiss_trigger);
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                if let Some(region) = hit
                    .filter(|region| matches!(region.kind, HitKind::TextInput | HitKind::TextArea))
                {
                    if let Some(input) = self.inputs.get_mut(&region.id) {
                        if region.kind == HitKind::TextArea || input.laid_out.is_some() {
                            input.place_cursor_at(point);
                        } else {
                            let x = (point.x - input.content_x).max(0.0);
                            let shown = input
                                .value
                                .chars()
                                .skip(input.view_start)
                                .collect::<String>();
                            let character = nearest_char(&shown, x, |part| input.width(part));
                            input.move_cursor(input.view_start + character, false);
                        }
                    }
                }
            }
            UiEvent::PointerUp(point) => {
                self.pointer = Some(point);
                self.set_hovered(self.frame.hit_test(point).map(str::to_owned), now);
                self.tooltip_hover_since = self.hovered.as_ref().map(|id| (id.clone(), now));
                if let Some(drag) = self.slider_drag.take() {
                    let released_fraction = drag.geometry.fraction_at(point);
                    let snapped = drag.geometry.snap(released_fraction);
                    if !self.reduced_motion && drag.geometry.animated {
                        let motion = self
                            .slider_motion
                            .entry(drag.id.clone())
                            .or_insert_with(|| SliderMotion::new(now));
                        motion.displayed_fraction = released_fraction;
                        motion.snap = None;
                        motion.animate_to(
                            snapped,
                            now,
                            drag.geometry.ultra_available
                                && snapped >= 1.0
                                && drag.geometry.fraction < 1.0,
                        );
                    }
                    let y = drag.geometry.y_at(point.y);
                    actions.push(slider_action(drag.id, snapped, y, true));
                    return actions;
                }
                if let Some(selection) = self.static_selection.as_mut().filter(|s| s.dragging) {
                    if let Some(block) = self.frame.text_blocks.get(selection.block) {
                        if block.source == selection.source {
                            selection.cursor = block.offset_at(
                                point,
                                self.text_measurer.as_deref(),
                                self.metrics,
                                self.sans_metrics,
                            );
                        }
                    }
                    selection.dragging = false;
                }
                if self.scroll_drag.take().is_some() {
                    return actions;
                }
                if let Some(pan) = self.horizontal_pan.take().filter(|pan| pan.claimed) {
                    self.release_horizontal_pan(pan, now);
                    return actions;
                }
                if self.scroll_button_hold.take().is_some() {
                    return actions;
                }
                let pressed_dispatch = self.pressed_dispatch.take();
                if let Some(pressed) = self.pressed.take() {
                    if self.frame.press_region(point).map(|region| region.id.as_str())
                        == Some(pressed.as_str())
                        && pressed_dispatch.as_deref() == Some(self.dispatch_id(&pressed).as_str())
                    {
                        actions.push(UiAction::Clicked {
                            id: self.dispatch_id(&pressed),
                        });
                    }
                }
            }
            UiEvent::Scroll { position, delta_y } | UiEvent::ScrollDirect { position, delta_y } => {
                let smooth = matches!(event, UiEvent::Scroll { .. });
                self.scroll_regular_regions(position, delta_y, smooth, now);
            }
            UiEvent::Text(text) => {
                if let Some(id) = self.focused.as_ref() {
                    let multiline = self
                        .focus_order
                        .iter()
                        .any(|(candidate, kind)| candidate == id && *kind == HitKind::TextArea);
                    if let Some(input) = self.inputs.get_mut(id) {
                        if input.insert(&text, multiline) {
                            actions.push(UiAction::InputChanged {
                                id: id.clone(),
                                value: input.value.clone(),
                            });
                        }
                    }
                }
            }
            UiEvent::Key { key, shift, ctrl } => {
                if matches!(key, Key::PageUp | Key::PageDown) && !ctrl && self.scroll_by_page(key == Key::PageDown, now) {
                    // Scrolled: a page of what is behind the focus, or under the pointer.
                } else if key == Key::Escape {
                    if let Some(scope) = self.active_popover_scope() {
                        let dismiss_id = scope.dismiss_id.clone();
                        let trigger_id = scope.trigger_id.clone();
                        if scope.submenu {
                            if let Some(id) = &trigger_id {
                                self.closed_submenu_triggers.insert(id.clone());
                            }
                        } else {
                            actions.push(UiAction::Clicked { id: dismiss_id });
                        }
                        let next_focus = trigger_id;
                        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                        self.keyboard_focus = true;
                        self.keyboard_tooltip_focus = true;
                    }
                } else if key == Key::C && ctrl {
                    let input_text = self.focused.as_ref().and_then(|id| {
                        let input = self.inputs.get(id)?;
                        if input.password {
                            return None;
                        }
                        let (start, end) = input.selected_range()?;
                        Some(char_range(&input.value, start, end).to_owned())
                    });
                    if let Some(text) =
                        input_text.or_else(|| self.selected_text().map(str::to_owned))
                    {
                        actions.push(UiAction::CopyRequested { text });
                    }
                } else if key == Key::X && ctrl {
                    if let Some(id) = self.focused.as_ref() {
                        if let Some(input) = self.inputs.get_mut(id) {
                            if let Some((start, end)) = input.selected_range() {
                                let text = char_range(&input.value, start, end).to_owned();
                                input.delete_selection();
                                if !input.password {
                                    actions.push(UiAction::CopyRequested { text });
                                }
                                actions.push(UiAction::InputChanged {
                                    id: id.clone(),
                                    value: input.value.clone(),
                                });
                            }
                        }
                    }
                } else if key == Key::Tab {
                    self.step_focus(shift);
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if !shift
                    && !ctrl
                    && matches!(key, Key::ArrowLeft | Key::ArrowRight)
                    && self.step_coach(key, &mut actions)
                {
                    self.keyboard_focus = true;
                } else if !shift
                    && !ctrl
                    && matches!(
                        key,
                        Key::ArrowLeft
                            | Key::ArrowRight
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::Home
                            | Key::End
                    )
                    && self.step_slider(key, &mut actions)
                {
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if !shift
                    && !ctrl
                    && matches!(
                        key,
                        Key::ArrowLeft
                            | Key::ArrowRight
                            | Key::ArrowUp
                            | Key::ArrowDown
                            | Key::Home
                            | Key::End
                    )
                    && self.step_tab(key, &mut actions)
                {
                    // A tab strip activates the newly focused tab.
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if !shift
                    && !ctrl
                    && matches!(
                        key,
                        Key::ArrowDown
                            | Key::ArrowUp
                            | Key::ArrowRight
                            | Key::ArrowLeft
                            | Key::Home
                            | Key::End
                    )
                    && self.step_popover_option(key)
                {
                    // Menu navigation consumes arrow/home/end keys only while
                    // its trigger or one of its buttons owns focus.
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if !shift
                    && !ctrl
                    && matches!(key, Key::ArrowDown | Key::ArrowUp | Key::Home | Key::End)
                    && self.step_inline_menu_option(key)
                {
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if let Some(id) = self.focused.as_ref() {
                    let kind = self
                        .focus_order
                        .iter()
                        .find(|(candidate, _)| candidate == id)
                        .map(|(_, kind)| *kind);
                    if kind == Some(HitKind::Button) && !self.frame.sliders.contains_key(id) {
                        if matches!(key, Key::Enter | Key::Space) {
                            self.keyboard_focus = true;
                            self.keyboard_tooltip_focus = true;
                            actions.push(UiAction::Clicked {
                                id: self.dispatch_id(id),
                            });
                        }
                    } else if matches!(kind, Some(HitKind::TextInput | HitKind::TextArea)) {
                        let multiline = kind == Some(HitKind::TextArea);
                        let Some(input) = self.inputs.get_mut(id) else {
                            return actions;
                        };
                        let changed = match key {
                            Key::Escape | Key::Tab | Key::Space => false,
                            Key::ArrowLeft => {
                                input.move_cursor(input.cursor.saturating_sub(1), shift);
                                false
                            }
                            Key::ArrowRight => {
                                input.move_cursor(input.cursor.saturating_add(1), shift);
                                false
                            }
                            Key::ArrowUp => {
                                if multiline {
                                    input.move_vertical(-1, shift);
                                }
                                false
                            }
                            Key::ArrowDown => {
                                if multiline {
                                    input.move_vertical(1, shift);
                                }
                                false
                            }
                            Key::Home => {
                                if multiline && !ctrl {
                                    input.move_line_edge(false, shift);
                                } else {
                                    input.move_cursor(0, shift);
                                }
                                false
                            }
                            Key::End => {
                                if multiline && !ctrl {
                                    input.move_line_edge(true, shift);
                                } else {
                                    input.move_cursor(input.value.chars().count(), shift);
                                }
                                false
                            }
                            Key::Backspace => input.backspace(),
                            Key::Delete => input.delete_forward(),
                            Key::A if ctrl => {
                                input.set_selection(input.value.chars().count(), Some(0));
                                input.keep_cursor_visible = true;
                                false
                            }
                            Key::Enter => {
                                // A chat composer breaks the line on Shift+Enter;
                                // any other text area on Enter alone.
                                let breaks = !ctrl && (!input.enter_submits || shift);
                                if multiline && breaks {
                                    input.insert("\n", true)
                                } else {
                                    actions.push(UiAction::Submitted {
                                        id: id.clone(),
                                        value: input.value.clone(),
                                    });
                                    false
                                }
                            }
                            Key::A => false,
                            _ => false,
                        };
                        if changed {
                            actions.push(UiAction::InputChanged {
                                id: id.clone(),
                                value: input.value.clone(),
                            });
                        }
                    }
                }
            }
        }
        actions
    }

    fn scroll_max(&self, id: &str) -> f32 {
        // The layout builder records scroll extents separately in the frame.
        self.frame.scroll_extents.get(id).copied().unwrap_or(0.0)
    }

    fn dispatch_id(&self, focus_id: &str) -> String {
        self.action_aliases
            .get(focus_id)
            .cloned()
            .unwrap_or_else(|| focus_id.to_owned())
    }

    fn step_slider(&self, key: Key, actions: &mut Vec<UiAction>) -> bool {
        let Some(id) = self.focused.as_ref() else {
            return false;
        };
        let Some(geometry) = self.frame.sliders.get(id) else {
            return false;
        };
        // An area slider: Left and Right move across, Up and Down up and down.
        if let Some(y) = geometry.y {
            let x = geometry.fraction;
            let step = CONTINUOUS_SLIDER_KEY_STEP;
            let (x, y) = match key {
                Key::ArrowLeft => ((x - step).max(0.0), y),
                Key::ArrowRight => ((x + step).min(1.0), y),
                Key::ArrowUp => (x, (y - step).max(0.0)),
                Key::ArrowDown => (x, (y + step).min(1.0)),
                Key::Home => (0.0, y),
                Key::End => (1.0, y),
                _ => return false,
            };
            actions.push(UiAction::AreaSliderChanged {
                id: id.clone(),
                x,
                y,
                committed: true,
            });
            return true;
        }
        // A vertical slider: Up raises it and Down lowers it.
        let key = match (geometry.vertical, key) {
            (true, Key::ArrowUp) => Key::ArrowRight,
            (true, Key::ArrowDown) => Key::ArrowLeft,
            (_, key) => key,
        };
        if geometry.steps < 2 {
            let current = geometry.fraction.clamp(0.0, 1.0);
            let fraction = match key {
                Key::ArrowLeft => (current - CONTINUOUS_SLIDER_KEY_STEP).max(0.0),
                Key::ArrowRight => (current + CONTINUOUS_SLIDER_KEY_STEP).min(1.0),
                Key::Home => 0.0,
                Key::End => 1.0,
                _ => return false,
            };
            actions.push(UiAction::SliderChanged {
                id: id.clone(),
                fraction,
                committed: true,
            });
            return true;
        }
        let intervals = geometry.steps.saturating_sub(1).max(1);
        let current = (geometry.fraction.clamp(0.0, 1.0) * intervals as f32).round() as usize;
        let next = match key {
            Key::ArrowLeft => current.saturating_sub(1),
            Key::ArrowRight => (current + 1).min(intervals),
            Key::Home => 0,
            Key::End => intervals,
            _ => return false,
        };
        actions.push(UiAction::SliderChanged {
            id: id.clone(),
            fraction: next as f32 / intervals as f32,
            committed: true,
        });
        true
    }

    fn request_scroll_focus(&mut self, id: &str) {
        if self.frame.scroll_focus_targets.contains_key(id) {
            self.reveal_scroll_focus(id, 0.0, false);
            self.pending_scroll_focus = None;
        } else {
            self.pending_scroll_focus = Some(id.to_owned());
        }
    }

    fn step_focus(&mut self, reverse: bool) {
        if let Some(scope) = self.active_popover_scope().cloned() {
            let reachable = self.reachable_popover_focusables(&scope);
            if !reachable.is_empty() {
                let current = self
                    .focused
                    .as_ref()
                    .and_then(|id| reachable.iter().position(|(candidate, _)| candidate == id));
                let next = match (current, reverse) {
                    (Some(index), false) => (index + 1) % reachable.len(),
                    (Some(0), true) => reachable.len() - 1,
                    (Some(index), true) => index - 1,
                    (None, false) => 0,
                    (None, true) => reachable.len() - 1,
                };
                let id = reachable[next].0.clone();
                self.request_scroll_focus(&id);
                let next_focus = Some(id);
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                return;
            }
            if let Some(trigger) = scope.trigger_id.as_ref() {
                let next_focus = Some(trigger.clone());
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                return;
            }
        }
        if self.step_virtual_list_focus(reverse) {
            return;
        }
        if self.focus_order.is_empty() {
            let next_focus = None;
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            return;
        }
        let current = self.focused.as_ref().and_then(|id| {
            self.focus_order
                .iter()
                .position(|(candidate, _)| candidate == id)
        });
        let next = match (current, reverse) {
            (Some(index), false) => (index + 1) % self.focus_order.len(),
            (Some(0), true) => self.focus_order.len() - 1,
            (Some(index), true) => index - 1,
            (None, false) => 0,
            (None, true) => self.focus_order.len() - 1,
        };
        let id = self.focus_order[next].0.clone();
        self.request_scroll_focus(&id);
        let next_focus = Some(id);
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
    }

    fn step_virtual_list_focus(&mut self, reverse: bool) -> bool {
        let Some(focused) = self.focused.as_ref() else {
            return false;
        };
        let Some((list_id, scope)) = self.frame.virtual_lists.iter().find(|(_, scope)| {
            let edge = if reverse {
                scope.focusables().next()
            } else {
                scope.focusables().last()
            };
            edge.is_some_and(|(id, _)| id == focused)
        }) else {
            return false;
        };
        let list_id = list_id.clone();
        let scope = scope.clone();
        let Some(list) = find_uniform_list(&self.root, &list_id).cloned() else {
            return false;
        };
        let candidates: Box<dyn Iterator<Item = usize>> = if reverse {
            Box::new((0..scope.first_visible).rev())
        } else {
            Box::new(scope.end_visible..scope.row_count)
        };
        for index in candidates {
            let row = (list.build_row)(index);
            let mut focusables = Vec::new();
            let mut seen = HashSet::new();
            collect_focus_order(&row, &mut focusables, &mut seen);
            if let Some((_, opened)) = list.expansion.as_ref().filter(|(open, _)| *open == index) {
                collect_focus_order(opened, &mut focusables, &mut seen);
            }
            let target = if reverse {
                focusables.last()
            } else {
                focusables.first()
            };
            if let Some((id, _)) = target {
                self.scroll_to_index(&list_id, index);
                let next_focus = Some(id.clone());
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                return true;
            }
        }
        false
    }

    fn reachable_popover_focusables(&self, scope: &PopoverFocusScope) -> Vec<(String, HitKind)> {
        // A modal form may contain controls below its scroll viewport. Keep
        // them in Tab order; request_scroll_focus will reveal the chosen one.
        // Menus also retain controls inside clipped scroll rows. Their exact
        // geometry is probed only when chosen; other clipped controls stay out.
        if scope.modal {
            return scope.focusables.clone();
        }
        scope
            .focusables
            .iter()
            .filter(|(id, kind)| {
                self.frame.scroll_focus_targets.contains_key(id)
                    || self.frame.scroll_focus_candidates.contains(id)
                    || self
                        .frame
                        .regions
                        .iter()
                        .any(|region| region.id == *id && region.kind == *kind)
            })
            .cloned()
            .collect()
    }

    /// Scroll every area around the control so it shows whole, `margin`
    /// clear of the viewport's edges where the control leaves that room.
    fn reveal_scroll_focus(&mut self, id: &str, margin: f32, start: bool) {
        let Some(target) = self.frame.scroll_focus_targets.get(id) else {
            return;
        };
        let rect = target.rect;
        let mut areas = target.ancestors.clone();
        areas.push((target.scroll_id.clone(), target.viewport, target.layout_offset));
        let innermost = areas.len() - 1;
        for (index, (scroll_id, viewport, layout_offset)) in areas.into_iter().enumerate() {
            let max_offset = self.scroll_max(&scroll_id);
            self.scroll_transitions.remove(&scroll_id);
            if self
                .scroll_button_hold
                .as_ref()
                .is_some_and(|hold| hold.id == scroll_id)
            {
                self.scroll_button_hold = None;
            }
            let offset = self.scroll_offsets.entry(scroll_id).or_default();
            let top = rect.y - (*offset - layout_offset);
            let bottom = top + rect.height;
            let margin = margin.min(((viewport.height - rect.height) / 2.0).max(0.0));
            let viewport_top = viewport.y + margin;
            let viewport_bottom = viewport.y + viewport.height - margin;
            // Half a pixel is in view: a second pass must find nothing to do.
            if start && index == innermost {
                if (top - viewport_top).abs() > 0.5 {
                    *offset = (*offset + top - viewport_top).clamp(0.0, max_offset);
                }
            } else if top < viewport_top - 0.5 {
                *offset = (*offset - (viewport_top - top)).clamp(0.0, max_offset);
            } else if bottom > viewport_bottom + 0.5 {
                // A control taller than its viewport keeps its start in view.
                let down = (bottom - viewport_bottom).min(top - viewport_top);
                *offset = (*offset + down).clamp(0.0, max_offset);
            }
        }
    }

    fn active_popover_scope(&self) -> Option<&PopoverFocusScope> {
        let scopes = modal_popover_scopes(&self.frame.popover_focus, &self.closed_submenu_triggers);
        let focused = self.focused.as_deref();
        scopes
            .iter()
            .rev()
            .copied()
            .find(|scope| {
                focused.is_some_and(|id| {
                    scope.trigger_id.as_deref() == Some(id)
                        || scope.focusables.iter().any(|(candidate, _)| candidate == id)
                })
            })
            .or_else(|| scopes.last().copied())
    }

    fn step_tab(&mut self, key: Key, actions: &mut Vec<UiAction>) -> bool {
        let Some(focused) = self.focused.as_deref() else {
            return false;
        };
        let Some(scope) = self
            .frame
            .tab_focus
            .iter()
            .find(|scope| scope.action_ids.iter().any(|id| id == focused))
        else {
            return false;
        };
        // Hit regions are clipped to the viewport. Keyboard navigation must
        // include offscreen tabs so the horizontal scroller can reveal them
        // on the next layout, rather than wrapping among only visible tabs.
        let tabs: Vec<_> = scope
            .action_ids
            .iter()
            .filter(|id| {
                self.focus_order
                    .iter()
                    .any(|(candidate, kind)| candidate == *id && *kind == HitKind::Button)
            })
            .collect();
        let Some(index) = tabs.iter().position(|id| id.as_str() == focused) else {
            return false;
        };
        let next = match (key, scope.vertical) {
            (Key::ArrowUp, false) if scope.every_arrow => (index + tabs.len() - 1) % tabs.len(),
            (Key::ArrowDown, false) if scope.every_arrow => (index + 1) % tabs.len(),
            (Key::ArrowLeft, false) => (index + tabs.len() - 1) % tabs.len(),
            (Key::ArrowRight, false) => (index + 1) % tabs.len(),
            (Key::ArrowUp, true) => index.saturating_sub(1),
            (Key::ArrowDown, true) => (index + 1).min(tabs.len() - 1),
            (Key::Home, _) => 0,
            (Key::End, _) => tabs.len() - 1,
            _ => return false,
        };
        if next != index {
            let id = tabs[next].to_string();
            let next_focus = Some(id.clone());
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            actions.push(UiAction::Clicked {
                id: self.dispatch_id(&id),
            });
        }
        true
    }

    /// Left and Right walk a coach mark's steps, whichever of its buttons
    /// has focus. A step with no action in that direction keeps the key.
    fn step_coach(&mut self, key: Key, actions: &mut Vec<UiAction>) -> bool {
        let Some((back, next)) = self.active_popover_scope().and_then(|scope| scope.arrows.clone())
        else {
            return false;
        };
        let id = if key == Key::ArrowLeft { back } else { next };
        actions.extend(id.map(|id| UiAction::Clicked { id }));
        true
    }

    fn step_popover_option(&mut self, key: Key) -> bool {
        if key == Key::ArrowRight {
            if let Some((trigger, target)) = self.focused.as_ref().and_then(|id| {
                self.frame
                    .submenu_targets
                    .get(id)
                    .map(|target| (id.clone(), target.clone()))
            }) {
                self.closed_submenu_triggers.remove(&trigger);
                self.request_scroll_focus(&target);
                let next_focus = Some(target);
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                return true;
            }
        }
        let Some(scope) = self.active_popover_scope().cloned() else {
            return false;
        };
        let buttons: Vec<_> = self
            .reachable_popover_focusables(&scope)
            .into_iter()
            .filter(|(_, kind)| *kind == HitKind::Button)
            .collect();
        if buttons.is_empty() {
            return false;
        }
        let current = self
            .focused
            .as_ref()
            .and_then(|id| buttons.iter().position(|(candidate, _)| candidate == id));
        // A menu at a point has no trigger: the focus left outside it stands in.
        let trigger_focused = self.focused.is_none()
            || scope.pointed && current.is_none()
            || self
                .focused
                .as_ref()
                .is_some_and(|id| scope.trigger_id.as_ref() == Some(id));
        if current.is_none() && !trigger_focused {
            return false;
        }
        // Keep the complete button order so arrows from an auxiliary action
        // continue to the adjacent primary action instead of losing the row.
        let primary = |index: &usize| {
            !self
                .frame
                .menu_secondary_actions
                .contains(&buttons[*index].0)
        };
        let next = match key {
            Key::ArrowDown => (1..=buttons.len())
                .map(|step| (current.unwrap_or(buttons.len() - 1) + step) % buttons.len())
                .find(primary),
            Key::ArrowUp => (1..=buttons.len())
                .map(|step| (current.unwrap_or(0) + buttons.len() - step) % buttons.len())
                .find(primary),
            Key::ArrowRight if scope.submenu && trigger_focused => Some(0),
            Key::ArrowLeft if scope.submenu && current.is_some() => {
                if let Some(id) = &scope.trigger_id {
                    self.closed_submenu_triggers.insert(id.clone());
                }
                let next_focus = scope.trigger_id;
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                return true;
            }
            Key::Home => (0..buttons.len()).find(primary),
            Key::End => (0..buttons.len()).rev().find(primary),
            _ => return false,
        };
        let Some(next) = next else {
            return false;
        };
        let id = buttons[next].0.clone();
        self.request_scroll_focus(&id);
        let next_focus = Some(id);
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
        true
    }
}

const VIRTUAL_FOCUS_PREFIX: &str = "\0uniform_list:";

fn find_uniform_list<'a>(element: &'a Element, id: &str) -> Option<&'a UniformList> {
    match &element.kind {
        ElementKind::UniformList(list) if list.id == id => Some(list),
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => children
            .iter()
            .find_map(|child| find_uniform_list(child, id)),
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => find_uniform_list(child, id),
        ElementKind::Overlay { base, floating } => {
            find_uniform_list(base, id).or_else(|| find_uniform_list(floating, id))
        }
        ElementKind::Popover {
            anchor, floating, ..
        } => find_uniform_list(anchor, id).or_else(|| find_uniform_list(floating, id)),
        ElementKind::Tooltip { anchor, .. } => find_uniform_list(anchor, id),
        _ => None,
    }
}

fn collect_focus_order(
    element: &Element,
    order: &mut Vec<(String, HitKind)>,
    seen: &mut HashSet<String>,
) {
    if element.kept.as_ref().is_some_and(|kept| kept.inert) {
        return;
    }
    match &element.kind {
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => {
            for child in children {
                collect_focus_order(child, order, seen);
            }
        }
        ElementKind::Panel(child)
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => {
            collect_focus_order(child, order, seen);
        }
        ElementKind::UniformList(list) => {
            // Layout replaces this marker with focusable controls from the
            // visible rows. No row factory is called during this tree walk.
            order.push((
                format!("{VIRTUAL_FOCUS_PREFIX}{}", list.id),
                HitKind::Scroll,
            ));
        }
        ElementKind::Clickable { id, child } => {
            if seen.insert(id.clone()) {
                order.push((id.clone(), HitKind::Button));
            }
            collect_focus_order(child, order, seen);
        }
        ElementKind::Overlay { base, floating } => {
            collect_focus_order(base, order, seen);
            collect_focus_order(floating, order, seen);
        }
        ElementKind::Popover {
            anchor,
            floating,
            hover_open,
            ..
        } => {
            collect_focus_order(anchor, order, seen);
            if !*hover_open {
                collect_focus_order(floating, order, seen);
            }
        }
        ElementKind::Tooltip { anchor, .. } => collect_focus_order(anchor, order, seen),
        ElementKind::MarkRail(rail) => {
            for mark in &rail.marks {
                if seen.insert(mark.id.clone()) {
                    order.push((mark.id.clone(), HitKind::Button));
                }
            }
        }
        ElementKind::Button { id, .. }
        | ElementKind::IconButton { id, .. }
        | ElementKind::EffortSlider { id, .. }
            if seen.insert(id.clone()) =>
        {
            order.push((id.clone(), HitKind::Button));
        }
        ElementKind::TextInput { id, .. } if seen.insert(id.clone()) => {
            order.push((id.clone(), HitKind::TextInput));
        }
        ElementKind::TextArea { id, .. } if seen.insert(id.clone()) => {
            order.push((id.clone(), HitKind::TextArea));
        }
        ElementKind::CodeEditor(editor) if seen.insert(editor.id.clone()) => {
            order.push((editor.id.clone(), HitKind::CodeEditor));
        }
        ElementKind::CodeDiff(diff) if diff.focusable && seen.insert(diff.id.clone()) => {
            order.push((diff.id.clone(), HitKind::CodeDiff));
        }
        ElementKind::FormattedText(spans) => {
            for id in spans
                .iter()
                .filter(|span| !span.text.is_empty())
                .filter_map(|span| span.action.as_ref())
            {
                if seen.insert(id.clone()) {
                    order.push((id.clone(), HitKind::Button));
                }
            }
        }
        ElementKind::Button { .. }
        | ElementKind::IconButton { .. }
        | ElementKind::TextInput { .. }
        | ElementKind::TextArea { .. }
        | ElementKind::CodeEditor(_)
        | ElementKind::CodeDiff(_)
        | ElementKind::Flowchart(_)
        | ElementKind::SequenceDiagram(_)
        | ElementKind::EffortSlider { .. }
        | ElementKind::Skeleton { .. }
        | ElementKind::Spinner { .. }
        | ElementKind::Record { .. }
        | ElementKind::ProgressRing { .. }
        | ElementKind::RollingNumber { .. }
        | ElementKind::UnitGrid(_)
        | ElementKind::LevelBars(_)
        | ElementKind::BarPlot(_)
        | ElementKind::LinePlot(_)
        | ElementKind::PiePlot(_)
        | ElementKind::RadarPlot(_)
        | ElementKind::HeatPlot(_)
        | ElementKind::Text(_)
        | ElementKind::ShimmerText { .. }
        | ElementKind::KeyboardShortcut { .. }
        | ElementKind::ShortcutHint { .. }
        | ElementKind::TruncatedText(_)
        | ElementKind::FadedText { .. }
        | ElementKind::RasterImage(_)
        | ElementKind::Surface { .. }
        | ElementKind::Icon(_)
        | ElementKind::Drawing(_)
        | ElementKind::Spacer => {}
    }
}

fn collect_action_metadata(
    element: &Element,
    aliases: &mut HashMap<String, String>,
    secondary_actions: &mut HashSet<String>,
) {
    if let Some(id) = focusable_id(element) {
        if let Some(action) = &element.action_alias {
            aliases.insert(id.to_owned(), action.clone());
        }
        if element.menu_secondary_action {
            secondary_actions.insert(id.to_owned());
        }
    }
    match &element.kind {
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => {
            for child in children {
                collect_action_metadata(child, aliases, secondary_actions);
            }
        }
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => {
            collect_action_metadata(child, aliases, secondary_actions)
        }
        ElementKind::Overlay { base, floating }
        | ElementKind::Popover {
            anchor: base,
            floating,
            ..
        } => {
            collect_action_metadata(base, aliases, secondary_actions);
            collect_action_metadata(floating, aliases, secondary_actions);
        }
        ElementKind::Tooltip { anchor, .. } => {
            collect_action_metadata(anchor, aliases, secondary_actions)
        }
        ElementKind::UniformList(_)
        | ElementKind::Text(_)
        | ElementKind::ShimmerText { .. }
        | ElementKind::KeyboardShortcut { .. }
        | ElementKind::ShortcutHint { .. }
        | ElementKind::TruncatedText(_)
        | ElementKind::FadedText { .. }
        | ElementKind::FormattedText(_)
        | ElementKind::RasterImage(_)
        | ElementKind::Surface { .. }
        | ElementKind::Icon(_)
        | ElementKind::Button { .. }
        | ElementKind::IconButton { .. }
        | ElementKind::TextInput { .. }
        | ElementKind::TextArea { .. }
        | ElementKind::CodeEditor(_)
        | ElementKind::CodeDiff(_)
        | ElementKind::Flowchart(_)
        | ElementKind::SequenceDiagram(_)
        | ElementKind::EffortSlider { .. }
        | ElementKind::Skeleton { .. }
        | ElementKind::Spinner { .. }
        | ElementKind::Record { .. }
        | ElementKind::ProgressRing { .. }
        | ElementKind::RollingNumber { .. }
        | ElementKind::UnitGrid(_)
        | ElementKind::LevelBars(_)
        | ElementKind::BarPlot(_)
        | ElementKind::LinePlot(_)
        | ElementKind::PiePlot(_)
        | ElementKind::RadarPlot(_)
        | ElementKind::HeatPlot(_)
        | ElementKind::MarkRail(_)
        | ElementKind::Drawing(_)
        | ElementKind::Spacer => {}
    }
}

fn focusable_id(element: &Element) -> Option<&str> {
    match &element.kind {
        ElementKind::CodeEditor(editor) => Some(&editor.id),
        ElementKind::CodeDiff(diff) if diff.focusable => Some(&diff.id),
        ElementKind::Button { id, .. }
        | ElementKind::IconButton { id, .. }
        | ElementKind::Clickable { id, .. }
        | ElementKind::TextInput { id, .. }
        | ElementKind::TextArea { id, .. }
        | ElementKind::EffortSlider { id, .. } => Some(id),
        _ => None,
    }
}

/// What an element's kind tells of it: a button is a button named by its
/// label, a field is a field named by its placeholder.
fn inferred_access(element: &Element) -> crate::Access {
    use crate::{Access, Role};
    match &element.kind {
        ElementKind::Button { label, .. } | ElementKind::IconButton { label, .. } => {
            Access { name: (!label.is_empty()).then(|| label.clone()), ..Access::new(Role::Button) }
        }
        ElementKind::Clickable { child, .. } => {
            let role = if element.slider.is_some() { Role::Slider } else { Role::Button };
            let name = spoken_text(child);
            Access { name: (!name.is_empty()).then_some(name), ..Access::new(role) }
        }
        ElementKind::TextInput { initial_value, placeholder, password, .. } => Access {
            name: (!placeholder.is_empty()).then(|| placeholder.clone()),
            // What is typed into a password field is nobody's to read out.
            value_text: (!*password).then(|| initial_value.clone()),
            ..Access::new(if *password { Role::PasswordInput } else { Role::TextInput })
        },
        ElementKind::TextArea { initial_value, placeholder, .. } => Access {
            name: (!placeholder.is_empty()).then(|| placeholder.clone()),
            value_text: Some(initial_value.clone()),
            ..Access::new(Role::MultilineTextInput)
        },
        ElementKind::EffortSlider { fraction, .. } => Access::new(Role::Slider).value(*fraction),
        ElementKind::CodeEditor(_) => Access::new(Role::MultilineTextInput),
        _ => Access::default(),
    }
}

/// The text an element shows, as one line: what names a control that was
/// given no name of its own.
fn spoken_text(element: &Element) -> String {
    fn gather(element: &Element, out: &mut String) {
        // A name is a phrase, not a page.
        if out.len() >= 160 {
            return;
        }
        let mut say = |text: &str| {
            let text = text.trim();
            if !text.is_empty() {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(text);
            }
        };
        match &element.kind {
            ElementKind::Text(text)
            | ElementKind::TruncatedText(text)
            | ElementKind::FadedText { text, .. }
            | ElementKind::ShimmerText { text, .. } => say(text),
            ElementKind::Button { label, .. } | ElementKind::IconButton { label, .. } => say(label),
            ElementKind::FormattedText(spans) => {
                say(&spans.iter().map(|span| span.text.as_str()).collect::<String>())
            }
            ElementKind::Column(children) | ElementKind::Row(children) | ElementKind::Stack(children) => {
                children.iter().for_each(|child| gather(child, out))
            }
            ElementKind::TabStrip { children, .. } => children.iter().for_each(|child| gather(child, out)),
            ElementKind::Panel(child)
            | ElementKind::Clickable { child, .. }
            | ElementKind::ShortcutHint { child, .. }
            | ElementKind::Scroll { child, .. }
            | ElementKind::HorizontalScroll { child, .. } => gather(child, out),
            ElementKind::Overlay { base, .. } => gather(base, out),
            ElementKind::Popover { anchor, .. } | ElementKind::Tooltip { anchor, .. } => gather(anchor, out),
            _ => {}
        }
    }
    let mut out = String::new();
    gather(element, &mut out);
    out
}

/// Whether a hover-revealed element must stay painted without the pointer:
/// a popover inside it is open, or the keyboard focus is on one of its
/// controls.
fn holds_reveal(element: &Element, keyboard_focus: bool, focused: Option<&str>) -> bool {
    if keyboard_focus && focused.is_some() && focusable_id(element) == focused {
        return true;
    }
    let holds = |child: &Element| holds_reveal(child, keyboard_focus, focused);
    match &element.kind {
        ElementKind::Popover { .. } => true,
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => children.iter().any(holds),
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. }
        | ElementKind::Tooltip { anchor: child, .. } => holds(child),
        ElementKind::Overlay { base, floating } => holds(base) || holds(floating),
        _ => false,
    }
}

/// Where a wheel or a trackpad scroll that moves something is.
fn scroll_at(event: &UiEvent) -> Option<Point> {
    let moves = |delta: f32| delta.is_finite() && delta != 0.0;
    match *event {
        UiEvent::Scroll { position, delta_y } | UiEvent::ScrollDirect { position, delta_y } => {
            moves(delta_y).then_some(position)
        }
        UiEvent::Scroll2D { position, delta_x, delta_y, .. } => (moves(delta_x) || moves(delta_y)).then_some(position),
        _ => None,
    }
}

/// Whether the pointer or the focus is on something `element` holds, the
/// submenus nested in it included: a submenu that opens on hover stays open
/// while it is, however deep the submenu the pointer went into.
fn holds_submenu(element: &Element, hovered: Option<&str>, focused: Option<&str>) -> bool {
    let on = |id: &str| hovered == Some(id) || focused == Some(id);
    if focusable_id(element).is_some_and(on) {
        return true;
    }
    let holds = |child: &Element| holds_submenu(child, hovered, focused);
    match &element.kind {
        ElementKind::Popover {
            anchor,
            floating,
            dismiss_id,
            ..
        } => {
            hovered
                .is_some_and(|id| id.strip_prefix("popover_surface:") == Some(dismiss_id.as_str()))
                || holds(anchor)
                || holds(floating)
        }
        ElementKind::Scroll { id, child, .. } => on(id) || holds(child),
        ElementKind::Tooltip {
            anchor, anchor_id, ..
        } => on(anchor_id) || holds(anchor),
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => children.iter().any(holds),
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => holds(child),
        ElementKind::Overlay { base, floating } => holds(base) || holds(floating),
        _ => false,
    }
}

/// Record paths to explicit anchors and the requested control. Clipped branches
/// outside these paths still skip their descendants entirely.
fn find_focus_path(element: &Element, id: Option<&str>, path: &mut HashSet<usize>) -> bool {
    let descendants = match &element.kind {
        ElementKind::FormattedText(spans) => spans
            .iter()
            .any(|span| !span.text.is_empty() && span.action.as_deref().is_some_and(|action| Some(action) == id)),
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => {
            let mut found = false;
            for child in children { found |= find_focus_path(child, id, path); }
            found
        }
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => find_focus_path(child, id, path),
        ElementKind::Overlay { base, floating } => {
            find_focus_path(base, id, path) | find_focus_path(floating, id, path)
        }
        ElementKind::Popover {
            anchor, floating, ..
        } => find_focus_path(anchor, id, path) | find_focus_path(floating, id, path),
        ElementKind::Tooltip { anchor, .. } => find_focus_path(anchor, id, path),
        _ => false,
    };
    let found = descendants || element.anchor.is_some()
        || focusable_id(element).is_some_and(|target| Some(target) == id);
    if found {
        path.insert(element as *const Element as usize);
    }
    found
}

struct LayoutBuilder<'a> {
    /// The window: what floats beside the pointer stays inside it.
    viewport: Rect,
    frame: Frame,
    action_aliases: &'a mut HashMap<String, String>,
    overlays: VecDeque<DeferredOverlay>,
    scroll_focus_context: Vec<ScrollFocusContext>,
    /// Sticky children the scroll area being placed draws last.
    pinned: Vec<PinnedChild>,
    /// Per-layout, scope-aware queries for descendants that can leave their flow slot.
    translated_overflow: HashMap<(u64, usize), bool>,
    focus_path: HashSet<usize>,
    focus_probe: Option<&'a str>,
    /// Source of the active pointer drag and the commands it painted.
    drag_source: Option<&'a str>,
    drag_ghost: Option<std::ops::Range<usize>>,
    metrics: FontMetrics,
    sans_metrics: FontMetrics,
    text_measurer: Option<Arc<TextMeasurer>>,
    text_height_measurer: Option<Arc<TextHeightMeasurer>>,
    width_cache: &'a RefCell<TextWidthCache>,
    formatted_cache: &'a RefCell<FormattedTextCache>,
    layout_cache: &'a RefCell<TextLayoutCache>,
    text_layout: bool,
    measurement_cache: &'a RefCell<MeasurementCache>,
    flow_heights: &'a RefCell<HashMap<String, FlowHeight>>,
    scalars: &'a RefCell<HashMap<String, AnimatedScalar>>,
    springs: &'a RefCell<HashMap<String, AnimatedSpring>>,
    pushes_moved: &'a std::cell::Cell<bool>,
    inks: &'a RefCell<HashMap<String, (ColorTransition, Instant)>>,
    leaving: &'a RefCell<HashMap<String, (Vec<String>, Instant)>>,
    measurement_scope: u64,
    next_measurement_scope: u64,
    horizontal_scrolls: &'a mut HashMap<String, HorizontalScrollState>,
    inputs: &'a mut HashMap<String, InputState>,
    code_editors: &'a mut HashMap<String, crate::code_editor::EditorView>,
    code_diffs: &'a mut HashMap<String, crate::code_editor::DiffView>,
    scroll_offsets: &'a mut HashMap<String, f32>,
    scroll_transitions: &'a mut HashMap<String, ScalarTransition>,
    scrollbar_states: &'a mut HashMap<String, ScrollbarState>,
    measured_lists: &'a mut HashMap<String, MeasuredListState>,
    scroll_to_end: &'a mut HashSet<String>,
    scroll_to_index: &'a mut HashMap<String, usize>,
    hovered_scrollbar: Option<&'a str>,
    dragging_scrollbar: Option<&'a str>,
    dragging_horizontal: Option<&'a str>,
    scroll_button_hold: Option<&'a ScrollButtonHold>,
    pointer: Option<Point>,
    /// The pointer is over the `hover_region` being placed.
    region_hovered: bool,
    /// What a wrapper said of the control inside it, until that control takes it.
    access_above: Option<crate::Access>,
    slider_drag: Option<&'a SliderDrag>,
    slider_motion: &'a mut HashMap<String, SliderMotion>,
    focused: Option<&'a str>,
    caret_blink: &'a mut crate::caret_blink::CaretBlink,
    hovered: Option<&'a str>,
    tooltip_hover_since: Option<&'a (String, Instant)>,
    keyboard_focus: bool,
    keyboard_tooltip_focus: bool,
    pressed: Option<&'a str>,
    closed_submenu_triggers: &'a HashSet<String>,
    selection: Option<&'a StaticSelection>,
    transitions: &'a mut HashMap<String, ColorTransition>,
    popover_transitions: &'a mut HashMap<String, ScalarTransition>,
    popover_exits: &'a HashMap<String, PopoverExit>,
    replacements: &'a mut HashMap<String, Replacement>,
    observed_replacements: &'a mut HashSet<String>,
    previous_popover_snapshots: &'a HashMap<String, PopoverSnapshot>,
    popover_view_transitions: &'a mut HashMap<String, PopoverViewTransition>,
    coach_motion: &'a mut Option<CoachMotion>,
    tab_indicators: &'a mut HashMap<String, TabIndicatorState>,
    skeleton_started: &'a mut HashMap<String, Instant>,
    shimmer_started: &'a mut HashMap<String, Instant>,
    spinner_started: &'a mut HashMap<String, Instant>,
    record_motion: &'a mut HashMap<String, RecordMotion>,
    observed_records: &'a mut HashSet<String>,
    observed_buttons: &'a mut HashSet<String>,
    /// The fill of the shared hover surface the pointer is over, if any.
    hover_surface_fill: Option<Color>,
    /// The pointer is over no control: a shared surface under it stays lit,
    /// so crossing the gap between two of its actions does not blink.
    surface_gaps_lit: bool,
    observed_popovers: &'a mut HashSet<String>,
    popover_snapshots: &'a mut HashMap<String, PopoverSnapshot>,
    observed_tabs: &'a mut HashSet<String>,
    observed_tooltips: &'a mut HashMap<String, Duration>,
    observed_skeletons: &'a mut HashSet<String>,
    observed_shimmers: &'a mut HashSet<String>,
    observed_spinners: &'a mut HashSet<String>,
    observed_sliders: &'a mut HashSet<String>,
    reduced_motion: bool,
    /// A control is being scrolled into view, which needs the layout an
    /// opening `animate_height` slot ends at.
    locating: bool,
    now: Instant,
}

enum DeferredOverlay {
    Free {
        floating: Element,
        area: Rect,
    },
    Anchored {
        floating: Element,
        anchor: Rect,
        dismiss_id: String,
        placement: PopoverPlacement,
        gap: f32,
        trigger_id: Option<String>,
        focusables: Vec<(String, HitKind)>,
        animation_ms: u16,
        close_animation_ms: Option<u16>,
        open_scale: f32,
        view_key: Option<String>,
        backdrop: bool,
        focus_scope: bool,
        backdrop_dismiss: bool,
        scrim: Option<Color>,
        initial_focus: Option<String>,
        return_focus: Option<String>,
        coach: Option<Box<CoachAnchor>>,
        /// What the Left and Right arrow keys report inside the surface.
        arrows: Option<Box<(Option<String>, Option<String>)>>,
        pointed: bool,
    },
    Tooltip {
        anchor: Rect,
        title: String,
        shortcut: Option<String>,
        sections: Vec<TooltipSection>,
        max_width: f32,
        placement: TooltipPlacement,
        colors: TooltipStyle,
        options: TooltipOptions,
    },
}

fn place_popover(
    anchor: Rect,
    desired: Size,
    viewport: Rect,
    placement: PopoverPlacement,
    gap: f32,
) -> Rect {
    if placement == PopoverPlacement::Bottom {
        let width = desired.width.max(0.0).min(viewport.width.max(0.0));
        let height = desired.height.max(0.0).min((viewport.height - 8.0).max(0.0));
        return Rect::new(
            viewport.x + (viewport.width - width) / 2.0,
            viewport.y + viewport.height - height,
            width,
            height,
        );
    }
    let inset = 8.0;
    let width = desired
        .width
        .max(0.0)
        .min((viewport.width - inset * 2.0).max(0.0));
    let mut height = desired
        .height
        .max(0.0)
        .min((viewport.height - inset * 2.0).max(0.0));
    let start = matches!(
        placement,
        PopoverPlacement::BottomStart | PopoverPlacement::TopStart | PopoverPlacement::RightStart
    );
    let prefer_below = matches!(
        placement,
        PopoverPlacement::BottomStart | PopoverPlacement::BottomEnd | PopoverPlacement::RightStart
    );
    let x = match placement {
        PopoverPlacement::Center => viewport.x + (viewport.width - width) / 2.0,
        PopoverPlacement::RightStart => {
            if anchor.x + anchor.width + gap + width <= viewport.x + viewport.width - inset {
                anchor.x + anchor.width + gap
            } else {
                anchor.x - gap - width
            }
        }
        PopoverPlacement::LeftStart => {
            if anchor.x - gap - width >= viewport.x + inset {
                anchor.x - gap - width
            } else {
                anchor.x + anchor.width + gap
            }
        }
        _ if start => anchor.x,
        _ => anchor.x + anchor.width - width,
    };
    let x = x.clamp(
        viewport.x + inset,
        (viewport.x + viewport.width - inset - width).max(viewport.x + inset),
    );
    let below = anchor.y + anchor.height + gap;
    let available_below = (viewport.y + viewport.height - inset - below).max(0.0);
    let available_above = (anchor.y - gap - viewport.y - inset).max(0.0);
    let y = if placement == PopoverPlacement::Center {
        viewport.y + (viewport.height - height) / 2.0
    } else if matches!(
        placement,
        PopoverPlacement::RightStart | PopoverPlacement::LeftStart
    ) {
        anchor.y
    } else {
        if available_below == 0.0 && available_above == 0.0 {
            // Container-wide anchors can legitimately occupy the viewport.
            // There is no external side in that case; retain the bounded
            // overlay instead of collapsing its usable height to zero.
            return Rect::new(x, viewport.y + inset, width, height);
        }
        // Keep vertical popovers outside their anchor even in a short window.
        // Pick a fitting side, or the roomier side when neither can fit, and
        // give its scroll viewport only that space. Moving a full-height
        // surface upward instead would cover the menu bar's hit targets.
        let use_below = if prefer_below {
            height <= available_below
                || (height > available_above && available_below >= available_above)
        } else {
            height > available_above
                && (height <= available_below || available_below > available_above)
        };
        height = height.min(if use_below {
            available_below
        } else {
            available_above
        });
        if use_below {
            below
        } else {
            anchor.y - gap - height
        }
    };
    let y = y.clamp(
        viewport.y + inset,
        (viewport.y + viewport.height - inset - height).max(viewport.y + inset),
    );
    Rect::new(x, y, width, height)
}

/// Space between a coach mark's target and the ring around it.
const COACH_RING_GAP: f32 = 3.0;
const COACH_RING_WIDTH: f32 = 2.0;
/// How far the caret reaches from the bubble towards the ring.
const COACH_CARET: f32 = 7.0;

/// Where a coach mark sits against `target`, and on which side it ended up:
/// the preferred side, else the opposite one, else the other axis. With room
/// nowhere it stays on the roomiest side and overlaps the target, and has no
/// side to point from.
fn place_coach(
    target: Rect,
    desired: Size,
    viewport: Rect,
    side: CoachSide,
    align: CoachAlign,
) -> (Rect, Option<CoachSide>) {
    let inset = 8.0;
    let gap = COACH_RING_GAP + COACH_RING_WIDTH + COACH_CARET;
    let width = desired.width.max(0.0).min((viewport.width - inset * 2.0).max(0.0));
    let height = desired.height.max(0.0).min((viewport.height - inset * 2.0).max(0.0));
    let room = |side| match side {
        CoachSide::Above => target.y - viewport.y - inset - gap - height,
        CoachSide::Below => viewport.y + viewport.height - inset - gap - height - target.y - target.height,
        CoachSide::Left => target.x - viewport.x - inset - gap - width,
        CoachSide::Right => viewport.x + viewport.width - inset - gap - width - target.x - target.width,
    };
    let (opposite, across) = match side {
        CoachSide::Above => (CoachSide::Below, [CoachSide::Right, CoachSide::Left]),
        CoachSide::Below => (CoachSide::Above, [CoachSide::Right, CoachSide::Left]),
        CoachSide::Left => (CoachSide::Right, [CoachSide::Below, CoachSide::Above]),
        CoachSide::Right => (CoachSide::Left, [CoachSide::Below, CoachSide::Above]),
    };
    let order = [side, opposite, across[0], across[1]];
    let fitting = order.into_iter().find(|side| room(*side) >= 0.0);
    let side = fitting.unwrap_or_else(|| {
        order
            .into_iter()
            .max_by(|a, b| room(*a).total_cmp(&room(*b)))
            .unwrap_or(side)
    });
    let along = |start: f32, length: f32, size: f32| match align {
        CoachAlign::Start => start,
        CoachAlign::Center => start + (length - size) / 2.0,
        CoachAlign::End => start + length - size,
    };
    let (x, y) = match side {
        CoachSide::Above => (along(target.x, target.width, width), target.y - gap - height),
        CoachSide::Below => (along(target.x, target.width, width), target.y + target.height + gap),
        CoachSide::Left => (target.x - gap - width, along(target.y, target.height, height)),
        CoachSide::Right => (target.x + target.width + gap, along(target.y, target.height, height)),
    };
    let x = x.clamp(
        viewport.x + inset,
        (viewport.x + viewport.width - inset - width).max(viewport.x + inset),
    );
    let y = y.clamp(
        viewport.y + inset,
        (viewport.y + viewport.height - inset - height).max(viewport.y + inset),
    );
    (Rect::new(x, y, width, height), fitting.map(|_| side))
}

/// The caret from the bubble's edge towards the middle of its target, kept
/// clear of the bubble's corners.
fn coach_caret(target: Rect, bubble: Rect, side: CoachSide) -> [Point; 3] {
    let half = COACH_CARET;
    let margin = crate::tokens().radius.surface + half;
    let clamp = |value: f32, start: f32, length: f32| {
        value.clamp(start + margin, (start + length - margin).max(start + margin))
    };
    let x = clamp(target.x + target.width / 2.0, bubble.x, bubble.width);
    let y = clamp(target.y + target.height / 2.0, bubble.y, bubble.height);
    // The base sits one pixel inside the bubble so the join never shows.
    let vertical = |edge: f32, sign: f32| {
        [
            Point::new(x - half, edge - sign),
            Point::new(x + half, edge - sign),
            Point::new(x, edge + sign * COACH_CARET),
        ]
    };
    let horizontal = |edge: f32, sign: f32| {
        [
            Point::new(edge - sign, y - half),
            Point::new(edge - sign, y + half),
            Point::new(edge + sign * COACH_CARET, y),
        ]
    };
    match side {
        CoachSide::Above => vertical(bubble.y + bubble.height, 1.0),
        CoachSide::Below => vertical(bubble.y, -1.0),
        CoachSide::Left => horizontal(bubble.x + bubble.width, 1.0),
        CoachSide::Right => horizontal(bubble.x, -1.0),
    }
}

/// The corner radius of the control or scroll area `id`, for a ring that
/// follows it. Rows of a virtual list are not in the static tree.
fn find_radius(element: &Element, id: &str) -> Option<f32> {
    let named = match &element.kind {
        ElementKind::Scroll { id: scroll, .. } => Some(scroll.as_str()),
        _ => focusable_id(element),
    };
    if named == Some(id) {
        return Some(element.style.radius);
    }
    match &element.kind {
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => {
            children.iter().find_map(|child| find_radius(child, id))
        }
        ElementKind::Panel(child)
        | ElementKind::Clickable { child, .. }
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => find_radius(child, id),
        ElementKind::Overlay { base, floating } => {
            find_radius(base, id).or_else(|| find_radius(floating, id))
        }
        ElementKind::Popover { anchor, .. } | ElementKind::Tooltip { anchor, .. } => {
            find_radius(anchor, id)
        }
        _ => None,
    }
}

fn place_tooltip(
    anchor: Rect,
    desired: Size,
    viewport: Rect,
    preferred: TooltipPlacement,
) -> (Rect, TooltipPlacement) {
    let inset = 8.0;
    let gap = 7.0;
    let width = desired.width.min((viewport.width - inset * 2.0).max(0.0));
    let height = desired.height.min((viewport.height - inset * 2.0).max(0.0));
    let left_room = (anchor.x - viewport.x - inset - gap).max(0.0);
    let right_room = (viewport.x + viewport.width - inset - anchor.x - anchor.width - gap).max(0.0);
    let above_room = (anchor.y - viewport.y - inset - gap).max(0.0);
    let below_room =
        (viewport.y + viewport.height - inset - anchor.y - anchor.height - gap).max(0.0);
    // Prefer the host's direction and its opposite. Before clamping a hint
    // over its trigger, try the other axis when it has enough space instead.
    let lateral = matches!(preferred, TooltipPlacement::Left | TooltipPlacement::Right);
    if lateral
        && width > left_room
        && width > right_room
        && (height <= above_room || height <= below_room)
    {
        return place_tooltip(
            anchor,
            desired,
            viewport,
            if above_room >= below_room {
                TooltipPlacement::Above
            } else {
                TooltipPlacement::Below
            },
        );
    }
    if !lateral
        && height > above_room
        && height > below_room
        && (width <= left_room || width <= right_room)
    {
        return place_tooltip(
            anchor,
            desired,
            viewport,
            if left_room >= right_room {
                TooltipPlacement::Left
            } else {
                TooltipPlacement::Right
            },
        );
    }
    if matches!(preferred, TooltipPlacement::Left | TooltipPlacement::Right) {
        let left = anchor.x - gap - width;
        let right = anchor.x + anchor.width + gap;
        let fits_left = left >= viewport.x + inset;
        let fits_right = right + width <= viewport.x + viewport.width - inset;
        let placement = match preferred {
            TooltipPlacement::Left if fits_left => TooltipPlacement::Left,
            TooltipPlacement::Right if fits_right => TooltipPlacement::Right,
            _ if fits_left => TooltipPlacement::Left,
            _ if fits_right => TooltipPlacement::Right,
            _ if anchor.x - viewport.x > viewport.x + viewport.width - anchor.x - anchor.width => {
                TooltipPlacement::Left
            }
            _ => TooltipPlacement::Right,
        };
        let x = if placement == TooltipPlacement::Left {
            left
        } else {
            right
        };
        let x = x.clamp(
            viewport.x + inset,
            (viewport.x + viewport.width - inset - width).max(viewport.x + inset),
        );
        let y = (anchor.y + anchor.height / 2.0 - height / 2.0).clamp(
            viewport.y + inset,
            (viewport.y + viewport.height - inset - height).max(viewport.y + inset),
        );
        return (Rect::new(x, y, width, height), placement);
    }
    let x = (anchor.x + anchor.width / 2.0 - width / 2.0).clamp(
        viewport.x + inset,
        (viewport.x + viewport.width - inset - width).max(viewport.x + inset),
    );
    let above = anchor.y - gap - height;
    let below = anchor.y + anchor.height + gap;
    let fits_above = above >= viewport.y + inset;
    let fits_below = below + height <= viewport.y + viewport.height - inset;
    let placement = match preferred {
        TooltipPlacement::Above if fits_above => TooltipPlacement::Above,
        TooltipPlacement::Below if fits_below => TooltipPlacement::Below,
        _ if fits_above => TooltipPlacement::Above,
        _ if fits_below => TooltipPlacement::Below,
        _ if anchor.y - viewport.y > viewport.y + viewport.height - anchor.y - anchor.height => {
            TooltipPlacement::Above
        }
        _ => TooltipPlacement::Below,
    };
    let y = match placement {
        TooltipPlacement::Above => above,
        TooltipPlacement::Below => below,
        TooltipPlacement::Left | TooltipPlacement::Right => {
            unreachable!("lateral tooltip placed above")
        }
    }
    .clamp(
        viewport.y + inset,
        (viewport.y + viewport.height - inset - height).max(viewport.y + inset),
    );
    (Rect::new(x, y, width, height), placement)
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

enum TooltipBlock<'a> {
    Separator,
    Text {
        lines: Vec<&'a str>,
        style: Style,
        color: Color,
    },
    IconRow(TooltipIconRowLayout),
}

impl LayoutBuilder<'_> {
    /// Give what was painted since command `start` the shared entrance and,
    /// if it `leaves`, keep its paint to fade out once it stops being painted.
    /// Used by everything that is not a popover: tooltips, context menus and
    /// elements with [`Element::animate_enter`] or `animate_presence`.
    fn animate_surface(&mut self, id: &str, start: usize, rect: Rect, slide_sign: f32, leaves: bool) {
        if self.reduced_motion || self.frame.commands.len() == start {
            return;
        }
        let now = self.now;
        self.observed_popovers.insert(id.to_owned());
        let exits = self.popover_exits;
        let transition = self
            .popover_transitions
            .entry(id.to_owned())
            .or_insert_with(|| {
                // Reopened while leaving: continue from what is still visible.
                ScalarTransition::new(
                    exits.get(id).map_or(0.0, |exit| exit.opacity.value(now)),
                    now,
                )
            });
        transition.retarget(
            1.0,
            now,
            Duration::from_millis(u64::from(crate::tokens().motion.enter_ms)),
        );
        let progress = transition.value(now);
        if leaves {
            self.popover_snapshots.insert(
            id.to_owned(),
            PopoverSnapshot {
                commands: self.frame.commands[start..].to_vec(),
                animation_ms: crate::tokens().motion.enter_ms,
                close_animation_ms: None,
                open_scale: 1.0,
                view_key: None,
                rect,
                slide_sign,
                scrim: None,
            },
            );
        }
        if progress < 1.0 {
            let origin = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height);
            let slide = (1.0 - progress) * slide_sign * 4.0;
            for command in &mut self.frame.commands[start..] {
                animate_popover_command(command, progress, slide, 1.0, origin);
            }
        }
    }

    /// Paint only: fade what was painted since command `start` in or out
    /// with the hover of the region around it. Layout, hit regions and the
    /// focus order are the same either way.
    fn reveal_on_hover(&mut self, element: &Element, id: &str, start: usize, reveal: HoverReveal) {
        let concealed = reveal == HoverReveal::Concealed;
        let shown = if concealed {
            !self.region_hovered
        } else {
            self.region_hovered || holds_reveal(element, self.keyboard_focus, self.focused)
        };
        let target = if shown { 1.0 } else { 0.0 };
        let now = self.now;
        self.observed_popovers.insert(id.to_owned());
        // First seen at rest: a page does not open with its rows fading.
        let transition = self
            .popover_transitions
            .entry(id.to_owned())
            .or_insert_with(|| ScalarTransition::new(target, now));
        if self.reduced_motion {
            transition.snap_to(target, now);
        } else if transition.target() != target {
            let motion = crate::tokens().motion;
            let ms = if shown { motion.enter_ms } else { motion.exit_ms };
            transition.retarget(target, now, Duration::from_millis(u64::from(ms)));
        }
        let opacity = transition.value(now);
        let rise = if reveal == HoverReveal::Rising { REVEAL_RISE * (1.0 - opacity) } else { 0.0 };
        if opacity < 1.0 {
            for command in &mut self.frame.commands[start..] {
                animate_popover_command(command, opacity, rise, 1.0, Point::new(0.0, 0.0));
            }
        }
    }

    /// Paint only: the new content travels up from one own-height below and
    /// the paint it replaced leaves upwards by the same distance, fading.
    fn animate_replacement(&mut self, id: &str, key: &str, start: usize, rect: Rect) {
        if self.reduced_motion {
            return;
        }
        let now = self.now;
        self.observed_replacements.insert(id.to_owned());
        let commands = self.frame.commands[start..].to_vec();
        let Some(state) = self.replacements.get_mut(id) else {
            // The first content is simply there; only a change travels.
            self.replacements.insert(
                id.to_owned(),
                Replacement {
                    key: key.to_owned(),
                    rect,
                    commands,
                    leaving: None,
                    progress: ScalarTransition::new(1.0, now),
                },
            );
            return;
        };
        if state.key != key {
            state.leaving = Some((state.rect, std::mem::take(&mut state.commands)));
            state.key = key.to_owned();
            state.progress = ScalarTransition::new(0.0, now);
            state.progress.retarget(
                1.0,
                now,
                Duration::from_millis(u64::from(crate::tokens().motion.move_ms)),
            );
        }
        state.rect = rect;
        state.commands = commands;
        let progress = state.progress.value(now);
        if progress >= 1.0 {
            state.leaving = None;
            return;
        }
        let origin = Point::new(rect.x, rect.y);
        for command in &mut self.frame.commands[start..] {
            animate_popover_command(command, progress, (1.0 - progress) * rect.height, 1.0, origin);
        }
        if let Some((from, leaving)) = &state.leaving {
            // The slot may have scrolled since the leaving paint was recorded.
            let slide = rect.y - from.y - progress * rect.height;
            for command in leaving {
                let mut command = command.clone();
                animate_popover_command(&mut command, 1.0 - progress, slide, 1.0, origin);
                self.frame.commands.push(command);
            }
        }
    }

    fn code_text_height(&self, size: f32) -> f32 {
        self.text_height_measurer
            .as_ref()
            .map(|measure| measure(size, FontFamily::Monospace))
            .filter(|height| height.is_finite() && *height > 0.0)
            .unwrap_or_else(|| {
                self.metrics
                    .at_size(size)
                    .line_height
            })
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
    fn paint_tooltip(
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
    ) {
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
            return;
        }
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
        let content_max = max_width - TOOLTIP_PADDING * 2.0;
        let shortcut = shortcut
            .map(shortcut::label)
            .filter(|value| !value.is_empty());
        let keycap_width = shortcut
            .map(|value| self.shortcut_width(value))
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
                    TooltipBlock::Text {
                        lines: self.wrapped_lines(text, content_max, style),
                        style,
                        color,
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
            };
            if separator_pending {
                content_height += 15.0;
                blocks.push(TooltipBlock::Separator);
                separator_pending = false;
            } else if content_height > 0.0 {
                content_height += 6.0;
            }
            let size = match &block {
                TooltipBlock::Text { lines, style, .. } => Size::new(
                    lines
                        .iter()
                        .map(|line| self.text_width(line, *style))
                        .fold(0.0_f32, f32::max),
                    lines.len() as f32 * self.metrics_for(*style).line_height,
                ),
                TooltipBlock::IconRow(row) => row.size,
                TooltipBlock::Separator => unreachable!(),
            };
            content_width = content_width.max(size.width);
            content_height += size.height;
            blocks.push(block);
        }
        let body_width = (TOOLTIP_PADDING * 2.0 + content_width)
            .max(TOOLTIP_HEIGHT)
            .min(max_width);
        let body_height = if rich {
            (content_height + 12.0).max(TOOLTIP_HEIGHT)
        } else {
            TOOLTIP_HEIGHT + title_growth
        };
        let (body, placement) = place_tooltip(
            anchor,
            Size::new(body_width, body_height),
            viewport,
            preferred,
        );
        if body.width < TOOLTIP_HEIGHT || body.height < TOOLTIP_HEIGHT {
            return;
        }
        // A small pointer names the element the hint belongs to.
        let tip_x = (anchor.x + anchor.width / 2.0).clamp(
            body.x + TOOLTIP_PADDING,
            (body.x + body.width - TOOLTIP_PADDING).max(body.x + TOOLTIP_PADDING),
        );
        let tip_y = (anchor.y + anchor.height / 2.0).clamp(
            body.y + TOOLTIP_PADDING,
            (body.y + body.height - TOOLTIP_PADDING).max(body.y + TOOLTIP_PADDING),
        );
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
        self.frame.commands.push(DrawCommand::Shadow {
            rect: body,
            shadow: BoxShadow::new(colors.shadow, Point::new(0.0, 2.0), 6.0, 0.0),
            radius: crate::tokens().radius.control,
        });
        self.frame.commands.push(DrawCommand::FillRect {
            rect: body,
            color: colors.background,
            radius: crate::tokens().radius.control,
        });
        self.frame.commands.push(DrawCommand::StrokeRect {
            rect: body,
            color: colors.border,
            width: 1.0,
            radius: crate::tokens().radius.control,
        });
        // Painted last so its fill covers the body border at the join; the
        // nested triangles leave a one-pixel outline.
        if options.arrow {
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: outline,
                color: colors.border,
            });
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: fill,
                color: colors.background,
            });
        }
        let content = Rect::new(
            body.x + TOOLTIP_PADDING,
            body.y + if rich { 6.0 } else { 1.0 },
            (body.width - TOOLTIP_PADDING * 2.0).max(0.0),
            (body.height - if rich { 12.0 } else { 2.0 }).max(0.0),
        );
        self.frame.commands.push(DrawCommand::PushClip(content));
        let mut text_y = if rich {
            content.y
        } else {
            body.y + (body.height - title_height) / 2.0
        };
        for line in title_lines {
            self.frame.commands.push(DrawCommand::Text {
                position: Point::new(content.x, text_y),
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
                body.x + body.width - TOOLTIP_PADDING - width,
                if rich {
                    content.y
                } else {
                    body.y + (body.height - shortcut::HEIGHT) / 2.0
                },
                width,
                shortcut::HEIGHT,
            );
            self.paint_shortcut(keycap, shortcut, colors.keycap, colors.text);
        }
        let mut divided = false;
        let mut has_content = title_height > 0.0 || keycap_width.is_some();
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
            if !divided && has_content {
                text_y += 6.0;
            }
            divided = false;
            has_content = true;
            match block {
                TooltipBlock::Text {
                    style,
                    lines,
                    color,
                } => {
                    for line in lines {
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(content.x, text_y),
                            text: line.to_owned(),
                            color,
                            font_size: style.font_size,
                            font_family: style.font_family,
                        });
                        text_y += self.metrics_for(style).line_height;
                    }
                }
                TooltipBlock::IconRow(row) => {
                    let line_height = self.metrics_for(footer_style).line_height;
                    let mut label_y = text_y + row.label_y;
                    for label in row.label_lines {
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(content.x, label_y),
                            text: label,
                            color: colors.muted,
                            font_size: footer_style.font_size,
                            font_family: footer_style.font_family,
                        });
                        label_y += line_height;
                    }
                    for item in row.items {
                        let rect = Rect::new(
                            content.x + item.rect.x,
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
                TooltipBlock::Separator => unreachable!(),
            }
        }
        self.frame.commands.push(DrawCommand::PopClip);
    }

    fn shortcut_width(&self, label: &str) -> f32 {
        shortcut::width(label, |key| {
            self.text_width(
                key,
                Style {
                    font_size: shortcut::FONT_SIZE,
                    font_family: FontFamily::Sans,
                    ..Style::default()
                },
            )
        })
    }

    fn menu_shortcut_width(&self, label: &str) -> f32 {
        self.text_width(
            &shortcut::menu_label(label),
            Style {
                font_size: shortcut::MENU_FONT_SIZE,
                font_family: FontFamily::Sans,
                ..Style::default()
            },
        )
    }

    fn paint_menu_shortcut(&mut self, rect: Rect, label: &str, ink: Color) {
        let text = shortcut::menu_label(label);
        let style = Style {
            font_size: shortcut::MENU_FONT_SIZE,
            font_family: FontFamily::Sans,
            ..Style::default()
        };
        let width = self.text_width(&text, style);
        if text.is_empty() || width > rect.width || rect.height < shortcut::HEIGHT {
            return;
        }
        self.frame.commands.push(DrawCommand::Text {
            position: Point::new(
                rect.x + rect.width - width,
                rect.y + (rect.height - self.metrics_for(style).line_height) / 2.0,
            ),
            text,
            color: ink,
            font_size: shortcut::MENU_FONT_SIZE,
            font_family: FontFamily::Sans,
        });
    }

    fn paint_shortcut(&mut self, rect: Rect, label: &str, fill: Color, ink: Color) {
        let text_style = Style {
            font_size: shortcut::FONT_SIZE,
            font_family: FontFamily::Sans,
            ..Style::default()
        };
        let group_width = self.shortcut_width(label);
        if group_width <= 0.0 || group_width > rect.width || rect.height < shortcut::HEIGHT {
            return;
        }
        let mut x = rect.x + rect.width - group_width;
        let y = rect.y + (rect.height - shortcut::HEIGHT) / 2.0;
        let line_height = self.metrics_for(text_style).line_height;
        for part in shortcut::parts(label) {
            let key = match part {
                shortcut::Part::Key(key) => key,
                shortcut::Part::KeyGap => {
                    x += shortcut::KEY_SPACING;
                    if let Some(separator) = shortcut::KEY_SEPARATOR {
                        self.frame.commands.push(DrawCommand::Text {
                            position: Point::new(x, y + (shortcut::HEIGHT - line_height) / 2.0),
                            text: separator.to_owned(),
                            color: shortcut::separator_ink(ink),
                            font_size: shortcut::FONT_SIZE,
                            font_family: FontFamily::Sans,
                        });
                        x += self.text_width(separator, text_style);
                    }
                    x += shortcut::KEY_SPACING;
                    continue;
                }
                shortcut::Part::ChordGap => {
                    x += shortcut::CHORD_GAP;
                    continue;
                }
            };
            let icon = shortcut::key_icon(key);
            let text_width = if icon.is_some() {
                shortcut::ICON_SIZE
            } else {
                self.text_width(key, text_style)
            };
            let cap = Rect::new(x, y, shortcut::cap_width(text_width), shortcut::HEIGHT);
            let radius = shortcut::radius();
            self.frame.commands.push(DrawCommand::FillRect {
                rect: cap,
                color: fill,
                radius,
            });
            self.frame.commands.push(DrawCommand::StrokeRect {
                rect: cap,
                color: shortcut::cap_border(ink),
                width: 1.0,
                radius,
            });
            self.frame.commands.push(DrawCommand::FillRect {
                rect: Rect::new(cap.x + radius, cap.y + cap.height - 1.0, cap.width - radius * 2.0, 1.0),
                color: shortcut::cap_edge(ink),
                radius: 0.0,
            });
            if let Some(icon) = icon {
                self.frame.commands.push(DrawCommand::Icon {
                    rect: Rect::new(
                        cap.x + (cap.width - shortcut::ICON_SIZE) / 2.0,
                        cap.y + (cap.height - shortcut::ICON_SIZE) / 2.0,
                        shortcut::ICON_SIZE,
                        shortcut::ICON_SIZE,
                    ),
                    icon,
                    tint: ink,
                });
            } else {
                self.frame.commands.push(DrawCommand::Text {
                    position: Point::new(
                        cap.x + (cap.width - text_width) / 2.0,
                        cap.y + (cap.height - line_height) / 2.0,
                    ),
                    text: key.to_owned(),
                    color: ink,
                    font_size: shortcut::FONT_SIZE,
                    font_family: FontFamily::Sans,
                });
            }
            x += cap.width;
        }
    }

    /// `value` in `style`, as laid out for this tree, broken at `width` when
    /// it is finite.
    fn laid_out_text(&self, value: &str, style: Style, width: f32, centered: bool) -> crate::TextLayout {
        self.layout_cache.borrow_mut().get(value, self.text_style(style), width, centered)
    }

    /// How text in `style` is laid out: in the line height and placement the
    /// measured path gives, so the switch moves nothing.
    fn text_style(&self, style: Style) -> crate::TextStyle {
        crate::TextStyle::new(style.font_size, style.font_family)
            .line_height(self.metrics_for(style).line_height)
            .leading(crate::Leading::Below)
    }

    /// Paint a single-line field laid out by limn: its text or placeholder,
    /// scrolled no further than keeps the caret in view, its selection and
    /// its caret.
    fn paint_laid_out_input(
        &mut self,
        id: &str,
        content: Rect,
        style: Style,
        clip: Option<Rect>,
        placeholder: &str,
        persistent_placeholder: bool,
    ) {
        let text_style = self.text_style(style);
        let focused = self.focused == Some(id);
        let Some(state) = self.inputs.get(id) else {
            return;
        };
        let is_placeholder = state.value.is_empty() && (!focused || persistent_placeholder);
        let shown = state.shown().into_owned();
        let layout = self.layout_cache.borrow_mut().get(&shown, text_style, f32::INFINITY, false);
        let painted = if is_placeholder {
            self.layout_cache.borrow_mut().get(placeholder, text_style, f32::INFINITY, false)
        } else {
            layout.clone()
        };
        let state = self.inputs.get_mut(id).expect("looked up above");
        let caret = layout.caret(byte_index(&shown, state.cursor));
        let width = painted.size().width;
        let room = (content.width - 2.0).max(0.0);
        let (text_x, scroll) = if width <= room {
            let x = if style.center_content { content.x + ((content.width - width) / 2.0).max(0.0) } else { content.x };
            (x, 0.0)
        } else {
            let scroll = state.scroll_x.min(caret.x).max(caret.x - room).min(width - room).max(0.0);
            (content.x, scroll)
        };
        let origin = Point::new(text_x - scroll, content.y);
        state.scroll_x = scroll;
        state.content_x = text_x;
        state.laid_out = Some(FieldLayout { style: text_style, width: None, layout: layout.clone(), origin });
        let selection = state
            .selected_range()
            .filter(|_| !is_placeholder)
            .map(|(start, end)| layout.selection(byte_index(&shown, start)..byte_index(&shown, end)));
        let location = crate::caret_blink::CaretLocation { document: 0, revision: state.value.len() as u64, offset: state.cursor };
        let Some(input_clip) = clip.and_then(|outer| outer.intersection(content)).or_else(|| clip.is_none().then_some(content)) else {
            return;
        };
        let at = |area: Rect| Rect::new(origin.x + area.x, origin.y + area.y, area.width, area.height);
        self.frame.commands.push(DrawCommand::PushClip(input_clip));
        for bounds in selection.iter().flatten() {
            self.frame.commands.push(DrawCommand::FillRect { rect: at(*bounds), color: INPUT_SELECTION, radius: 0.0 });
        }
        let ink = if is_placeholder { Color { a: 110, ..style.foreground } } else { style.foreground };
        self.frame.commands.extend(painted.draw(origin, ink));
        let caret = Rect { width: 1.5, ..at(caret) };
        if focused
            && selection.is_none()
            && caret.intersection(input_clip).is_some()
            && self.caret_blink.observe(id, location, self.now, self.reduced_motion)
        {
            self.frame.commands.push(DrawCommand::FillRect { rect: caret, color: style.foreground, radius: 0.0 });
        }
        self.frame.commands.push(DrawCommand::PopClip);
    }

    /// Paint a text area laid out by limn: its lines from the one scrolled
    /// to, in its highlights' inks and fills, its selection, its caret and,
    /// while it scrolls or is under the pointer, its scrollbar.
    #[allow(clippy::too_many_arguments)]
    fn paint_laid_out_area(
        &mut self,
        id: &str,
        rect: Rect,
        content: Rect,
        style: Style,
        clip: Option<Rect>,
        placeholder: &str,
        initial_value: &str,
        highlights: &[crate::TextHighlight],
    ) {
        let text_style = self.text_style(style);
        let focused = self.focused == Some(id);
        let Some(state) = self.inputs.get(id) else {
            return;
        };
        let is_placeholder = state.value.is_empty() && !focused;
        // Highlights follow the host's text; a stale edit is laid out plain
        // until the host catches up.
        let marked = !highlights.is_empty() && state.value == initial_value;
        let layout = if marked {
            let spans = highlighted_spans(&state.value, highlights, style);
            self.layout_cache.borrow_mut().formatted(&spans, text_style, content.width, None)
        } else {
            self.layout_cache.borrow_mut().get(&state.value, text_style, content.width, false)
        };
        let painted = if is_placeholder {
            self.layout_cache.borrow_mut().get(placeholder, text_style, content.width, false)
        } else {
            layout.clone()
        };
        let state = self.inputs.get_mut(id).expect("looked up above");
        let lines: Vec<crate::TextLine> = layout.lines().collect();
        let (cursor_row, caret) = state.caret_line(&layout);
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
        let top = if is_placeholder { 0.0 } else { lines.get(state.scroll_line).map_or(0.0, |line| line.rect.y) };
        let origin = Point::new(content.x, content.y - top);
        state.laid_out = Some(FieldLayout { style: text_style, width: Some(content.width), layout: layout.clone(), origin });
        let selection = state.selected_range().filter(|_| !is_placeholder).map(|(start, end)| {
            layout.selection(byte_index(&state.value, start)..byte_index(&state.value, end))
        });
        let fills: Vec<(Rect, Color)> = highlights
            .iter()
            .filter(|_| marked)
            .filter_map(|highlight| {
                let color = highlight.background?;
                let range = byte_index(&state.value, highlight.range.start)..byte_index(&state.value, highlight.range.end);
                Some(layout.selection(range).into_iter().map(move |bounds| (bounds, color)))
            })
            .flatten()
            .collect();
        let caret_shown = focused
            && selection.is_none()
            && (state.scroll_line..state.scroll_line + state.visible_lines).contains(&cursor_row);
        let location = crate::caret_blink::CaretLocation { document: 0, revision: state.value.len() as u64, offset: state.cursor };
        if state.scroll_line != state.painted_scroll_line {
            state.painted_scroll_line = state.scroll_line;
            state.scrolled_at = Some(self.now);
        }
        let scrolled = state.scrolled_at.is_some_and(|at| self.now.saturating_duration_since(at) < TEXT_AREA_BAR_HOLD);
        let (scroll_line, visible_lines) = (state.scroll_line, state.visible_lines);
        let at = |area: Rect| Rect::new(origin.x + area.x, origin.y + area.y, area.width, area.height);
        if let Some(input_clip) = clip.and_then(|outer| outer.intersection(content)).or_else(|| clip.is_none().then_some(content)) {
            self.frame.commands.push(DrawCommand::PushClip(input_clip));
            // Highlight fills go under the selection.
            for (bounds, color) in fills {
                self.frame.commands.push(DrawCommand::FillRect { rect: at(bounds), color, radius: 3.0 });
            }
            for bounds in selection.iter().flatten() {
                self.frame.commands.push(DrawCommand::FillRect { rect: at(*bounds), color: INPUT_SELECTION, radius: 0.0 });
            }
            let ink = if is_placeholder { Color { a: 110, ..style.foreground } } else { style.foreground };
            // Only the lines in view: a long draft is not drawn whole each frame.
            let (above, below) = (input_clip.y - text_style.line_height.unwrap_or(0.0), input_clip.y + input_clip.height);
            self.frame.commands.extend(painted.draw(origin, ink).into_iter().filter(|command| match command {
                DrawCommand::Glyphs(run) => run.glyphs.first().is_none_or(|glyph| (above..=below + run.size).contains(&(run.origin.y + glyph.y))),
                _ => true,
            }));
            let caret = Rect { width: 1.5, ..at(caret) };
            if caret_shown
                && caret.intersection(input_clip).is_some()
                && self.caret_blink.observe(id, location, self.now, self.reduced_motion)
            {
                self.frame.commands.push(DrawCommand::FillRect { rect: caret, color: style.foreground, radius: 0.0 });
            }
            self.frame.commands.push(DrawCommand::PopClip);
        }
        if max_scroll > 0 && !is_placeholder && (scrolled || self.pointer.is_some_and(|point| rect.contains(point))) {
            // shortcut: the thumb shows the position but is not a pointer
            // target; give it the scroll area's drag when a text area grows
            // past a few dozen lines.
            let track = rect.inset(3.0);
            let height = (track.height * visible_lines as f32 / lines.len() as f32).max(16.0).min(track.height);
            let thumb = Rect::new(
                track.x + track.width - 4.0,
                track.y + (track.height - height) * scroll_line as f32 / max_scroll as f32,
                4.0,
                height,
            );
            if clip.is_none_or(|clip| clip.intersection(thumb).is_some()) {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: thumb,
                    color: crate::ScrollbarAppearance::default().thumb_color(style.foreground, false, false),
                    radius: 2.0,
                });
            }
        }
    }

    /// Paint text laid out in a `TextLayout`: its selection, its glyphs, and
    /// the selectable block a pointer reads it back from.
    fn paint_laid_out_text(&mut self, value: &str, content: Rect, style: Style, clip: Option<Rect>) {
        let layout = self.laid_out_text(value, style, content.width, style.center_content);
        let origin = Point::new(content.x, content.y);
        let block_index = self.frame.text_blocks.len();
        let runs = self.paint_layout(&layout, value, origin, style, clip, block_index);
        self.frame.text_blocks.push(SelectableTextBlock {
            source: value.to_owned(),
            runs,
            region_layer: self.frame.regions.len(),
            layout: Some(PlacedText { layout, origin }),
        });
    }

    /// One hint for a run, over its part under the pointer, or its first
    /// part for the keyboard. Another text with a run of the same target,
    /// such as one action twice, shows none.
    fn show_text_hints(&mut self, hints: &[(String, Rect, TextHint)]) {
        let mut shown = HashSet::new();
        for (id, _, hint) in hints {
            // A run's hint waits as long as a `Tooltip` waits by default.
            self.observed_tooltips.insert(id.clone(), TOOLTIP_HOVER_DELAY);
            let Some(keyboard) = self.hint_due(id, TOOLTIP_HOVER_DELAY) else {
                continue;
            };
            if !shown.insert(id.as_str()) {
                continue;
            }
            let mut parts = hints
                .iter()
                .filter(|(other, ..)| other == id)
                .map(|(_, rect, _)| *rect);
            let pointer = self.pointer;
            let anchor = if keyboard {
                parts.next()
            } else {
                parts.find(|rect| pointer.is_some_and(|pointer| rect.contains(pointer)))
            };
            let Some(anchor) = anchor else {
                continue;
            };
            self.overlays.push_back(DeferredOverlay::Tooltip {
                anchor,
                title: hint.title.clone(),
                shortcut: None,
                sections: Vec::new(),
                max_width: 360.0,
                placement: TooltipPlacement::Above,
                colors: hint.colors,
                options: TooltipOptions::default(),
            });
        }
    }

    /// Formatted text as laid out for this tree, broken at `width` when it
    /// is finite and cut to the style's limit of lines.
    fn laid_out_formatted(&self, spans: &[TextSpan], style: Style, width: f32) -> crate::TextLayout {
        // The paragraph's least line height, as the measured path takes it.
        let least = self.metrics_for(style).line_height.max(style.line_height.unwrap_or(0.0));
        let paragraph = crate::TextStyle::new(style.font_size, style.font_family).line_height(least);
        self.layout_cache.borrow_mut().formatted(spans, paragraph, width, style.max_lines)
    }

    /// Paint formatted text laid out in a `TextLayout`: its spans' targets
    /// and hints, its selection, what it draws, its actions' focus and hover
    /// marks, and the selectable block a pointer reads it back from.
    fn paint_laid_out_formatted(&mut self, spans: &[TextSpan], content: Rect, style: Style, clip: Option<Rect>) {
        let layout = self.laid_out_formatted(spans, style, content.width);
        let origin = Point::new(content.x, content.y);
        let at = |area: Rect| Rect::new(origin.x + area.x, origin.y + area.y, area.width, area.height);
        let source: String = spans.iter().map(|span| span.text.as_str()).collect();
        let block_index = self.frame.text_blocks.len();
        let region_layer = self.frame.regions.len();
        // Each part of a span with an action or a hint, on every line it runs onto.
        let mut hints: Vec<(String, Rect, TextHint)> = Vec::new();
        for (index, span) in spans.iter().enumerate() {
            if span.text.is_empty() || (span.action.is_none() && span.tooltip.is_none()) {
                continue;
            }
            for bounds in layout.span_bounds(index).into_iter().map(at) {
                if let Some(id) = &span.action {
                    self.record_inline_focus_target(id, bounds);
                    self.add_region(id, bounds, HitKind::Button, clip);
                }
                if let Some(hint) = &span.tooltip {
                    // A run without an action is a passive target named after
                    // its text and its hint, so another text with the same
                    // hint is another target.
                    let id = span.action.clone().unwrap_or_else(|| {
                        let id = format!("{TEXT_HINT_PREFIX}{block_index}:{}", hint.title);
                        self.add_region(&id, bounds, HitKind::Hover, clip);
                        id
                    });
                    hints.push((id, bounds, hint.clone()));
                }
            }
        }
        let runs = self.paint_layout(&layout, &source, origin, style, clip, block_index);
        for (index, span) in spans.iter().enumerate() {
            let Some(id) = span.action.as_deref() else {
                continue;
            };
            let focused = self.keyboard_focus && self.focused == Some(id);
            let hovered = self.hovered == Some(id);
            if !focused && !hovered {
                continue;
            }
            for bounds in layout.span_bounds(index).into_iter().map(at) {
                if focused {
                    self.draw_focus_ring(bounds, style);
                }
                if hovered {
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: Rect::new(bounds.x, bounds.y + bounds.height - 1.0, bounds.width, 1.0),
                        color: span.color,
                        radius: 0.0,
                    });
                }
            }
        }
        self.frame.text_blocks.push(SelectableTextBlock {
            source,
            runs,
            region_layer,
            layout: Some(PlacedText { layout, origin }),
        });
        self.show_text_hints(&hints);
    }

    /// What every text laid out in a `TextLayout` paints: its selection under
    /// it, then what the layout draws. Gives its lines as selectable runs of
    /// `source`.
    fn paint_layout(
        &mut self,
        layout: &crate::TextLayout,
        source: &str,
        origin: Point,
        style: Style,
        clip: Option<Rect>,
        block_index: usize,
    ) -> Vec<SelectableTextRun> {
        let mut runs = Vec::new();
        let mut start = 0;
        for line in layout.lines() {
            let held = &source[line.range.clone()];
            // A line's own text; the break that ends it is the source's.
            let shown = held.strip_suffix('\n').map_or(held, |line| line.strip_suffix('\r').unwrap_or(line));
            let rect = Rect::new(origin.x + line.rect.x, origin.y + line.rect.y, line.rect.width, line.rect.height);
            runs.push(SelectableTextRun {
                start,
                end: start + shown.chars().count(),
                text: shown.to_owned(),
                rect,
                visible: clip.and_then(|area| rect.intersection(area)).or_else(|| clip.is_none().then_some(rect)),
                font_size: style.font_size,
                font_family: style.font_family,
                atomic: false,
            });
            start += held.chars().count();
        }
        if let Some((from, to)) = self
            .selection
            .filter(|selection| selection.block == block_index && selection.source == source)
            .and_then(|selection| selection.range())
        {
            let byte = |offset: usize| source.char_indices().nth(offset).map_or(source.len(), |(byte, _)| byte);
            for rect in layout.selection(byte(from)..byte(to)) {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: Rect::new(origin.x + rect.x, origin.y + rect.y, rect.width, rect.height),
                    color: Color::rgba(180, 180, 180, 84),
                    radius: 0.0,
                });
            }
        }
        self.frame.commands.extend(layout.draw(origin, style.foreground));
        runs
    }

    fn paint_static_run(
        &mut self,
        block_index: usize,
        source: &str,
        mut run: SelectableTextRun,
        position: Point,
        color: Color,
        clip: Option<Rect>,
    ) {
        run.visible = clip
            .and_then(|area| run.rect.intersection(area))
            .or_else(|| clip.is_none().then_some(run.rect));
        if let Some(selection) = self
            .selection
            .filter(|selection| selection.block == block_index && selection.source == source)
        {
            if let Some((start, end)) = selection.range() {
                let start = start.max(run.start).min(run.end);
                let end = end.max(run.start).min(run.end);
                if end > start {
                    let style = Style {
                        font_size: run.font_size,
                        font_family: run.font_family,
                        ..Style::default()
                    };
                    let (left, right) = if run.atomic {
                        (0.0, run.rect.width)
                    } else {
                        (
                            self.text_width(char_range(&run.text, 0, start - run.start), style),
                            self.text_width(char_range(&run.text, 0, end - run.start), style),
                        )
                    };
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            run.rect.x + left,
                            run.rect.y,
                            (right - left).max(0.0),
                            run.rect.height,
                        ),
                        color: Color::rgba(180, 180, 180, 84),
                        radius: 0.0,
                    });
                }
            }
        }
        if !run.atomic {
            self.frame.commands.push(DrawCommand::Text {
                position,
                text: run.text.clone(),
                color,
                font_size: run.font_size,
                font_family: run.font_family,
            });
        }
        self.frame.text_blocks[block_index].runs.push(run);
    }

    fn metrics_for(&self, style: Style) -> FontMetrics {
        match style.font_family {
            FontFamily::Monospace => self.metrics,
            FontFamily::Sans
            | FontFamily::SansItalic
            | FontFamily::SansMedium
            | FontFamily::SansBold
            | FontFamily::SansBoldItalic => self.sans_metrics,
        }
        .at_size(style.font_size)
    }

    /// Underline the text painted since command `first` and text block
    /// `first_block`, kept inside `bounds`. A faded label paints its line
    /// several times; each is underlined once.
    fn underline_text(&mut self, first: usize, first_block: usize, bounds: Rect, style: Style) {
        let mut lines: Vec<(Point, f32, f32, Color)> = Vec::new();
        for command in &self.frame.commands[first..] {
            let DrawCommand::Text {
                position,
                text,
                color,
                font_size,
                font_family,
            } = command
            else {
                continue;
            };
            let run = Style {
                font_size: *font_size,
                font_family: *font_family,
                ..style
            };
            match lines.iter_mut().find(|line| line.0 == *position) {
                Some(line) if color.a > line.3.a => line.3 = *color,
                Some(_) => {}
                None => lines.push((
                    *position,
                    self.text_width(text, run),
                    self.metrics_for(run).line_height,
                    *color,
                )),
            }
        }
        // Text laid out in a `TextLayout`: under each of its lines, in its ink.
        let ink = self.frame.commands[first..]
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Glyphs(run) => Some(run.color),
                _ => None,
            })
            .max_by_key(|color| color.a);
        if let Some(ink) = ink {
            for block in self.frame.text_blocks[first_block..].iter().filter(|block| block.layout.is_some()) {
                for run in &block.runs {
                    lines.push((Point::new(run.rect.x, run.rect.y), run.rect.width, run.rect.height, ink));
                }
            }
        }
        for (position, width, line_height, color) in lines {
            let left = position.x.max(bounds.x);
            let right = (position.x + width).min(bounds.x + bounds.width);
            if right > left {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: Rect::new(left, position.y + line_height - 2.0, right - left, 1.0),
                    color,
                    radius: 0.0,
                });
            }
        }
    }

    fn text_width(&self, value: &str, style: Style) -> f32 {
        if let Some(width) =
            self.width_cache
                .borrow()
                .get(value, style.font_size, style.font_family)
        {
            return width;
        }
        let width = measured_width(
            self.text_measurer.as_deref(),
            value,
            style.font_size,
            style.font_family,
            self.metrics_for(style).char_width,
        );
        self.width_cache
            .borrow_mut()
            .insert(value, style.font_size, style.font_family, width);
        width
    }

    /// One line of interface text at `size`, for elements that place their
    /// own labels.
    fn sans_width(&self, value: &str, size: f32) -> f32 {
        self.text_width(
            value,
            Style { font_size: size, font_family: FontFamily::Sans, ..Style::default() },
        )
    }

    fn wrapped_lines<'t>(&self, value: &'t str, width: f32, style: Style) -> Vec<&'t str> {
        wrap_measured(value, width, |part| self.text_width(part, style))
    }

    /// Paint the same sweep over a shaped paragraph line or a bounded label.
    /// The tuple contains highlight, elapsed seconds and widest line width.
    fn paint_shimmer_line(
        &mut self,
        text: &str,
        rect: Rect,
        style: Style,
        sweep: (Color, f32, f32),
        clip: Option<Rect>,
    ) {
        let (highlight, elapsed, sweep_width) = sweep;
        let center = (elapsed / 1.8).fract() * (sweep_width + 100.0) - 50.0;
        for band in 0..24 {
            let left = rect.x + rect.width * band as f32 / 24.0;
            let right = rect.x + rect.width * (band + 1) as f32 / 24.0;
            let area = Rect::new(left, rect.y, right - left, rect.height);
            let visible = if let Some(outer) = clip {
                area.intersection(outer)
            } else {
                Some(area)
            };
            if let Some(area) = visible.filter(|area| area.width > 0.0 && area.height > 0.0) {
                let distance = ((left + right) / 2.0 - rect.x - center).abs();
                let mix = (1.0 - distance / 38.0).max(0.0).powi(2);
                let blend = |base: u8, bright: u8| {
                    (base as f32 + (bright as f32 - base as f32) * mix).round() as u8
                };
                let color = Color::rgba(
                    blend(style.foreground.r, highlight.r),
                    blend(style.foreground.g, highlight.g),
                    blend(style.foreground.b, highlight.b),
                    style.foreground.a,
                );
                self.frame.commands.push(DrawCommand::PushClip(area));
                self.frame.commands.push(DrawCommand::Text {
                    position: Point::new(rect.x, rect.y),
                    text: text.to_owned(),
                    color,
                    font_size: style.font_size,
                    font_family: style.font_family,
                });
                self.frame.commands.push(DrawCommand::PopClip);
            }
        }
    }

    fn formatted_lines(
        &self,
        spans: &[TextSpan],
        width: f32,
        style: Style,
    ) -> Arc<[FormattedLine]> {
        let line_height = self
            .metrics_for(style)
            .line_height
            .max(style.line_height.unwrap_or(0.0));
        if let Some(lines) =
            self.formatted_cache
                .borrow()
                .get(spans, width, line_height, style.max_lines)
        {
            return lines;
        }
        let measure = |text: &str, span: &TextSpan| {
            self.text_width(
                text,
                Style {
                    font_size: span.font_size,
                    font_family: span.font_family,
                    ..style
                },
            )
        };
        let natural_height = |span: &TextSpan| {
            self.metrics_for(Style {
                font_size: span.font_size,
                font_family: span.font_family,
                ..style
            })
            .line_height
        };
        let mut lines = layout_formatted(spans, width, line_height, measure, natural_height);
        if let Some(max) = style.max_lines {
            lines = clamp_formatted(lines, max, width, measure, natural_height);
        }
        let lines: Arc<[FormattedLine]> = lines.into();
        self.formatted_cache.borrow_mut().insert(
            spans,
            width,
            line_height,
            style.max_lines,
            lines.clone(),
        );
        lines
    }

    /// Round the clip just pushed for `content` as the element's own corners
    /// are, less its padding, so they stay concentric.
    fn round_clip(&mut self, content: Rect, style: &Style) {
        let radii = style.corner_radii.unwrap_or(CornerRadii::same(style.radius));
        let inner = |radius: f32| (radius - style.padding).max(0.0);
        let radii = CornerRadii {
            top_left: inner(radii.top_left),
            top_right: inner(radii.top_right),
            bottom_right: inner(radii.bottom_right),
            bottom_left: inner(radii.bottom_left),
        };
        if [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left].iter().any(|radius| *radius > 0.0) {
            self.frame.commands.push(DrawCommand::RoundClip { rect: content, radii });
        }
    }

    fn draw_focus_ring(&mut self, rect: Rect, style: Style) {
        if !style.focus_ring {
            return;
        }
        let edge = 2.0_f32.min(rect.width / 2.0).min(rect.height / 2.0);
        let ink = Color::rgb(170, 170, 170);
        if edge > 0.0 {
            paint_corner_rect(
                &mut self.frame.commands,
                rect,
                ink,
                Some(edge),
                style.radius,
                style.corner_radii,
            );
        }
    }

    fn draw_icon(&mut self, icon: Icon, rect: Rect, style: Style) {
        self.frame.commands.push(DrawCommand::Icon {
            rect,
            icon,
            tint: style.foreground,
        });
    }

    /// Build row `index` of a measured list, once in a layout, and measure it
    /// at `width` in a measurement scope of its own, kept for placing it.
    fn measure_list_row(
        &mut self,
        list: &UniformList,
        built: &mut HashMap<usize, (Box<Element>, u64, f32)>,
        index: usize,
        width: f32,
    ) -> f32 {
        if let Some((_, _, height)) = built.get(&index) {
            return *height;
        }
        let row = Box::new((list.build_row)(index));
        let previous = self.enter_measurement_scope();
        let measured = self.measure(&row, Size::new(width, f32::INFINITY)).height;
        let scope = self.measurement_scope;
        self.measurement_scope = previous;
        let height = if measured.is_finite() {
            measured.max(0.0)
        } else {
            list.row_height
        };
        built.insert(index, (row, scope, height));
        height
    }

    /// A list whose rows take their own heights: see
    /// [`Element::measured_list`]. Rows are measured as they are built, and
    /// what the tree learned of them moves the offset so the row at the top
    /// of the viewport holds still.
    fn place_measured_list(
        &mut self,
        list: &UniformList,
        rect: Rect,
        content: Rect,
        clip: Option<Rect>,
        foreground: Color,
        visibility: crate::ScrollbarVisibility,
    ) {
        let Some(MeasuredRows { key, follow_end }) = list.measured.as_ref() else {
            return;
        };
        let id = list.id.as_str();
        self.add_region(id, rect, HitKind::Scroll, clip);
        let count = list.row_count;
        let keys: Vec<u64> = (0..count).map(|index| key(index)).collect();
        let mut state = self.measured_lists.remove(id).unwrap_or_default();
        let mut heights: Vec<f32> = keys
            .iter()
            .map(|key| state.heights.get(key).copied().unwrap_or(list.row_height))
            .collect();
        let tops = |heights: &[f32]| {
            let mut tops = Vec::with_capacity(heights.len() + 1);
            let mut top = 0.0_f32;
            tops.push(top);
            for height in heights {
                top = (top + height).min(f32::MAX / 4.0);
                tops.push(top);
            }
            tops
        };
        let mut top = tops(&heights);
        // Where the offset is, or is going: a scroll on its way to the end is
        // at the end.
        let target = self
            .scroll_transitions
            .get(id)
            .map(ScalarTransition::target)
            .or_else(|| self.scroll_offsets.get(id).copied())
            .unwrap_or(0.0);
        let to_end =
            self.scroll_to_end.remove(id) || (*follow_end && target >= state.end - 0.5);
        let to_index = self
            .scroll_to_index
            .remove(id)
            .filter(|_| count > 0)
            .map(|index| index.min(count - 1));
        // The row the view holds still: the one at the top of the viewport at
        // the last layout, and where its top was in the content then.
        let mut anchor = state
            .anchor
            .filter(|_| !to_end && to_index.is_none())
            .and_then(|(anchor, was)| {
                keys.iter().position(|key| *key == anchor).map(|index| (index, was))
            });
        let viewport = content.height;
        // Each row built in this layout: the row, the measurement scope it
        // was measured in, and its height.
        let mut built: HashMap<usize, (Box<Element>, u64, f32)> = HashMap::new();
        let mut row_width = f32::NAN;
        let (mut offset, mut max_offset, mut first, mut end) = (0.0, 0.0, 0, 0);
        for _ in 0..MEASURED_LIST_PASSES {
            let width = list
                .appearance
                .content_width(content.width, top[count] > viewport);
            if width != row_width {
                // Heights measured at another width stand as estimates.
                built.clear();
                row_width = width;
            }
            let mut changed = false;
            // The end, or a row to bring into view, is placed by the heights
            // of the rows around it: measure those first, from the bottom up.
            if to_end {
                let mut filled = 0.0;
                for index in (0..count).rev() {
                    if filled >= viewport {
                        break;
                    }
                    let height = self.measure_list_row(list, &mut built, index, row_width);
                    changed |= state.learn(&mut heights, keys[index], index, height);
                    filled += height;
                }
            }
            if let Some(index) = to_index {
                let height = self.measure_list_row(list, &mut built, index, row_width);
                changed |= state.learn(&mut heights, keys[index], index, height);
            }
            if changed {
                top = tops(&heights);
                changed = false;
            }
            // Rows above the one held still were added, measured or changed
            // height: move the offset, and a scroll on its way, by as much,
            // so what shows stays where it was.
            if let Some((index, was)) = anchor.as_mut() {
                let shift = top[*index] - *was;
                if shift.abs() > 0.01 {
                    if let Some(offset) = self.scroll_offsets.get_mut(id) {
                        *offset += shift;
                    }
                    if let Some(motion) = self.scroll_transitions.get_mut(id) {
                        motion.shift(shift);
                    }
                }
                *was = top[*index];
            }
            max_offset = (top[count] - viewport).max(0.0);
            let mut snap = to_end.then_some(max_offset);
            if let Some(index) = to_index {
                let mut target =
                    snap.unwrap_or_else(|| self.scroll_offsets.get(id).copied().unwrap_or(0.0));
                if top[index] < target {
                    target = top[index];
                } else if top[index + 1] > target + viewport {
                    target = top[index + 1] - viewport;
                }
                snap = Some(target);
            }
            offset = self.scroll_position(id, max_offset, list.motion, snap);
            first = top[1..].partition_point(|bottom| *bottom <= offset).min(count);
            // A row held still that the scroll took out of view gives way to
            // the one now at the top.
            let out_of_view =
                |index: usize| top[index + 1] <= offset || top[index] >= offset + viewport;
            if anchor.is_some_and(|(index, _)| out_of_view(index)) {
                anchor = (first < count).then(|| (first, top[first]));
            }
            // From the row at the top of the view down, at the heights they
            // really take: what fills the viewport.
            let (mut index, mut y) = (first, top[first]);
            while index < count && y < offset + viewport {
                let height = self.measure_list_row(list, &mut built, index, row_width);
                changed |= state.learn(&mut heights, keys[index], index, height);
                y += height;
                index += 1;
            }
            end = index;
            if !changed {
                break;
            }
            top = tops(&heights);
        }
        self.frame
            .scroll_extents
            .insert(list.id.clone(), max_offset);
        let scrollbar = ScrollbarGeometry::new(content, max_offset, offset, list.appearance);
        let visible = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content));
        let mut rows = Vec::new();
        if let Some(visible) = visible {
            self.frame.commands.push(DrawCommand::PushClip(visible));
            // Rows are content of a scroll area: `reveal` and anchors find
            // what they hold, as they do in a column of a `ScrollArea`.
            self.scroll_focus_context.push(ScrollFocusContext {
                id: list.id.clone(),
                viewport: visible,
                offset,
            });
            for (index, row_top) in top.iter().enumerate().take(end).skip(first) {
                let Some((row, scope, height)) = built.get(&index) else {
                    continue;
                };
                let previous = self.measurement_scope;
                self.measurement_scope = *scope;
                let mut focusables = Vec::new();
                collect_focus_order(row, &mut focusables, &mut HashSet::new());
                self.place(
                    row,
                    Rect::new(
                        content.x,
                        content.y + row_top - offset,
                        row_width,
                        *height,
                    ),
                    Some(visible),
                );
                rows.push((index, focusables));
                self.measurement_scope = previous;
            }
            self.scroll_focus_context.pop();
            self.frame.commands.push(DrawCommand::PopClip);
        }
        self.frame.virtual_lists.insert(
            list.id.clone(),
            VirtualListFocus {
                first_visible: first,
                end_visible: end,
                row_count: count,
                rows,
            },
        );
        if let Some(scrollbar) = scrollbar {
            self.paint_scrollbar(id, scrollbar, content, clip, foreground, list.appearance, visibility);
        }
        state.anchor = keys.get(first).map(|key| (*key, top[first]));
        state.end = max_offset;
        // Rows that left the list leave their heights behind; forget them
        // once they outnumber the rows still there.
        if state.heights.len() > keys.len() * 2 + 256 {
            let present: HashSet<u64> = keys.iter().copied().collect();
            state.heights.retain(|key, _| present.contains(key));
        }
        self.measured_lists.insert(list.id.clone(), state);
    }

    /// How visible the bar of scroll surface `id` is, from 0 to 1. `area` is
    /// the surface's visible box: under the pointer it wakes a bar that
    /// hides unless hovered.
    fn scrollbar_opacity(
        &mut self,
        id: &str,
        offset: f32,
        interacting: bool,
        visibility: crate::ScrollbarVisibility,
        area: Rect,
    ) -> f32 {
        let (hold_ms, fade_ms, hovered) = match visibility {
            crate::ScrollbarVisibility::Always => {
                self.scrollbar_states.remove(id);
                return 1.0;
            }
            crate::ScrollbarVisibility::AutoHide { hold_ms, fade_ms } => (hold_ms, fade_ms, false),
            crate::ScrollbarVisibility::AutoHideUnlessHovered { hold_ms, fade_ms } => {
                (hold_ms, fade_ms, self.pointer.is_some_and(|point| area.contains(point)))
            }
        };
        self.scrollbar_states
            .entry(id.to_owned())
            .or_insert_with(|| ScrollbarState::new(offset, self.now))
            .observe(offset, interacting || hovered, hold_ms, fade_ms, self.reduced_motion, self.now)
    }

    fn paint_scrollbar(
        &mut self,
        id: &str,
        geometry: ScrollbarGeometry,
        content: Rect,
        clip: Option<Rect>,
        ink: Color,
        appearance: crate::ScrollbarAppearance,
        visibility: crate::ScrollbarVisibility,
    ) {
        let visible = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content));
        let Some(visible) = visible else {
            return;
        };
        let active = self.dragging_scrollbar == Some(id);
        let hovered = self.hovered_scrollbar == Some(id);
        let offset = self.scroll_offsets.get(id).copied().unwrap_or(0.0);
        let interacting = active || hovered || self.scroll_button_hold.is_some_and(|hold| hold.id == id);
        let opacity = self.scrollbar_opacity(id, offset, interacting, visibility, visible);
        if opacity <= 0.0 { return; }
        let ink = appearance.tint.unwrap_or(ink);
        let ink = Color { a: (f32::from(ink.a) * opacity).round() as u8, ..ink };
        let appearance = crate::ScrollbarAppearance { tint: None, ..appearance };
        let alpha = |opacity: u8| {
            Color::rgba(
                ink.r,
                ink.g,
                ink.b,
                ((u16::from(ink.a) * u16::from(opacity)) / 255) as u8,
            )
        };
        self.frame.commands.push(DrawCommand::PushClip(visible));
        // The overlay rail never paints a permanent gray stripe behind
        // content. Its track appears only while interacting with it.
        if active || hovered {
            self.frame.commands.push(DrawCommand::FillRect {
                rect: geometry.track,
                color: appearance.track_color(ink),
                radius: appearance.corner(geometry.track.width),
            });
        }
        self.frame.commands.push(DrawCommand::FillRect {
            rect: geometry.thumb,
            color: appearance.thumb_color(ink, hovered, active),
            radius: appearance.corner(geometry.thumb.width),
        });
        let target = self
            .scroll_transitions
            .get(id)
            .map(ScalarTransition::target)
            .unwrap_or_else(|| self.scroll_offsets.get(id).copied().unwrap_or(0.0));
        for (direction, button) in [(-1, geometry.up_button), (1, geometry.down_button)] {
            let Some(button) = button else {
                continue;
            };
            let enabled = if direction < 0 {
                target > 0.001
            } else {
                target < geometry.max_offset - 0.001
            };
            let over = enabled && self.pointer.is_some_and(|point| button.contains(point));
            let pressed = over
                && self
                    .scroll_button_hold
                    .is_some_and(|hold| hold.id == id && hold.direction == direction);
            if over {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: button.inset(1.0),
                    color: alpha(appearance.hover_track_opacity.saturating_mul(if pressed {
                        3
                    } else {
                        2
                    })),
                    radius: crate::tokens().radius.control,
                });
            }
            let x = button.x + button.width / 2.0;
            let y = button.y + button.height / 2.0;
            let half_width = (button.width * 0.25).min(3.5);
            let half_height = (button.height * 0.15).min(2.0);
            let direction = f32::from(direction);
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: [
                    Point::new(x - half_width, y - direction * half_height),
                    Point::new(x + half_width, y - direction * half_height),
                    Point::new(x, y + direction * half_height),
                ],
                color: alpha(if !enabled {
                    appearance.idle_thumb_opacity / 3
                } else if pressed {
                    appearance.active_thumb_opacity
                } else if over {
                    appearance.hover_thumb_opacity
                } else {
                    appearance.idle_thumb_opacity
                }),
            });
        }
        self.frame.commands.push(DrawCommand::PopClip);
        self.add_region(id, geometry.rail, HitKind::Scrollbar, Some(visible));
        self.frame.scrollbars.insert(id.to_owned(), geometry);
    }

    fn scroll_position(
        &mut self,
        id: &str,
        max: f32,
        policy: ScrollMotion,
        snap: Option<f32>,
    ) -> f32 {
        self.frame.scroll_motion.insert(id.to_owned(), policy);
        let offset = self.scroll_offsets.entry(id.to_owned()).or_default();
        if let Some(value) = snap {
            *offset = value.clamp(0.0, max);
            self.scroll_transitions.remove(id);
        } else if let Some(motion) = self.scroll_transitions.get_mut(id) {
            let current = motion.value(self.now).clamp(0.0, max);
            let target = motion.target().clamp(0.0, max);
            if self.reduced_motion || policy.duration_ms == 0 {
                *offset = target;
                self.scroll_transitions.remove(id);
            } else {
                // Resizing/filtering may reduce the extent during a gesture.
                // Keep both ends of the remaining movement inside new bounds.
                if target != motion.target() || current != motion.value(self.now) {
                    motion.snap_to(current, self.now);
                    motion.retarget(
                        target,
                        self.now,
                        Duration::from_millis(u64::from(policy.duration_ms)),
                    );
                }
                *offset = current;
            }
        } else {
            *offset = offset.clamp(0.0, max);
        }
        *offset
    }

    fn measure(&self, element: &Element, available: Size) -> Size {
        let mut size = self.measure_natural(element, available);
        if let (Some(expand), Some((id, _))) = (element.animate_height, &element.presence) {
            size.height = self.flow_height(id, expand, size.height);
        }
        // A pushing reach is room taken: as far as its spring has got.
        if let Some(kept) = element.kept.as_ref().filter(|kept| kept.extend_pushes) {
            if let Some((id, by)) = &kept.extend {
                let length = by.x.abs().max(by.y.abs());
                if length > 0.0 {
                    let out = self.springs.borrow().get(&format!("extend/{id}")).map_or(0.0, |state| state.value.value(self.now));
                    size.width = (size.width + by.x.abs() * out / length).max(0.0);
                    size.height = (size.height + by.y.abs() * out / length).max(0.0);
                }
            }
        }
        size
    }

    /// The height an `animate_height` slot occupies now, given its content's.
    /// Starts or reverses its transition when the content arrives or leaves.
    fn flow_height(&self, id: &str, expand: bool, natural: f32) -> f32 {
        if self.reduced_motion {
            return natural;
        }
        let now = self.now;
        let present = natural > 0.0;
        let mut states = self.flow_heights.borrow_mut();
        // Content already there when the slot is first seen is simply there.
        let state = states.entry(id.to_owned()).or_insert_with(|| FlowHeight {
            fraction: slot_spring(if present { SLOT_OPEN } else { 0.0 }, now),
            natural,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            commands: Vec::new(),
            seen: now,
        });
        state.seen = now;
        let target = if present {
            state.natural = natural;
            SLOT_OPEN
        } else {
            0.0
        };
        if state.fraction.target() != target {
            if present && (!expand || self.locating) && state.fraction.value(now) <= 0.0 {
                state.fraction = slot_spring(SLOT_OPEN, now);
            } else {
                state.fraction.launch(target, slot_motion(), now);
            }
        }
        state.natural * (state.fraction.value(now) / SLOT_OPEN).clamp(0.0, 1.0)
    }

    /// Place an `animate_height` element in a slot that may be shorter than
    /// its content: clipped to the slot and faded, or, once the content has
    /// left, its last paint alone, with no regions.
    fn place_flow_height(&mut self, element: &Element, id: &str, rect: Rect, clip: Option<Rect>) {
        if self.reduced_motion {
            return self.place_inner(element, rect, clip);
        }
        if self.flow_heights.borrow().get(id).is_none_or(|state| state.seen != self.now) {
            // A parent that sizes its child without measuring it.
            self.measure(element, Size::new(rect.width, rect.height));
        }
        let now = self.now;
        let Some((fraction, natural, present)) = self.flow_heights.borrow().get(id).map(|state| {
            (
                (state.fraction.value(now) / SLOT_OPEN).clamp(0.0, 1.0),
                state.natural,
                state.fraction.target() > 0.0,
            )
        }) else {
            return self.place_inner(element, rect, clip);
        };
        let start = self.frame.commands.len();
        if present && fraction >= 1.0 {
            self.place_inner(element, rect, clip);
            if let Some(state) = self.flow_heights.borrow_mut().get_mut(id) {
                state.rect = rect;
                state.commands = self.frame.commands[start..].to_vec();
            }
            return;
        }
        let slot = clip
            .map_or(Some(rect), |clip| rect.intersection(clip))
            .filter(|slot| slot.height > 0.0);
        let origin = Point::new(rect.x, rect.y);
        if present {
            let full = Rect::new(rect.x, rect.y, rect.width, natural);
            self.frame.commands.push(DrawCommand::PushClip(slot.unwrap_or(Rect::new(rect.x, rect.y, 0.0, 0.0))));
            let inner = self.frame.commands.len();
            self.place_inner(element, full, slot.or(Some(Rect::new(rect.x, rect.y, 0.0, 0.0))));
            if let Some(state) = self.flow_heights.borrow_mut().get_mut(id) {
                state.rect = full;
                state.commands = self.frame.commands[inner..].to_vec();
            }
            for command in &mut self.frame.commands[inner..] {
                animate_popover_command(command, fraction, 0.0, 1.0, origin);
            }
            self.frame.commands.push(DrawCommand::PopClip);
        } else if let (Some(slot), Some(state)) = (slot, self.flow_heights.borrow().get(id)) {
            self.frame.commands.push(DrawCommand::PushClip(slot));
            // The slot may have scrolled since the paint was recorded.
            let slide = rect.y - state.rect.y;
            for command in &state.commands {
                let mut command = command.clone();
                animate_popover_command(&mut command, fraction, slide, 1.0, origin);
                self.frame.commands.push(command);
            }
            self.frame.commands.push(DrawCommand::PopClip);
        }
    }

    fn measure_natural(&self, element: &Element, available: Size) -> Size {
        let style = element.style;
        // Explicit dimensions discard natural dimensions. Avoid recursively
        // measuring an entire toolbar/panel whose final size is already known.
        if !self.measurement_cache.borrow().disabled() {
            let width = style.width.or_else(|| {
                (available.width.is_finite() && fills_available_width(&element.kind))
                    .then_some(available.width)
            });
            if let (Some(width), Some(height)) = (width, style.height) {
                return Size::new(
                    width
                        .min(style.max_width.unwrap_or(f32::INFINITY))
                        .max(style.min_width),
                    height
                        .min(style.max_height.unwrap_or(f32::INFINITY))
                        .max(style.min_height),
                );
            }
        }
        if let Some(measured) =
            self.measurement_cache
                .borrow()
                .get(self.measurement_scope, element, available)
        {
            return measured;
        }
        let measured = self.measure_uncached(element, available);
        self.measurement_cache.borrow_mut().insert(
            self.measurement_scope,
            element,
            available,
            measured,
        );
        measured
    }

    /// Where to paint a coach mark's ring this frame, and how far its bubble
    /// is from `bubble`, where it is laid out. Both spring to a new step's
    /// place; anything else that moves the target, such as a resize, is
    /// followed at once.
    fn coach_travel(&mut self, key: &str, ring: Rect, bubble: Rect) -> (Rect, (f32, f32)) {
        let now = self.now;
        let targets = [ring.x, ring.y, ring.width, ring.height, bubble.x, bubble.y];
        let state = self.coach_motion.get_or_insert_with(|| CoachMotion {
            key: key.to_owned(),
            values: targets.map(|value| ScalarSpring::new(value, now)),
        });
        let travels = state.key != key && !self.reduced_motion;
        if travels {
            let motion = crate::SpringMotion::settling_in(Duration::from_millis(u64::from(
                crate::tokens().motion.move_ms,
            )));
            for (value, target) in state.values.iter_mut().zip(targets) {
                value.launch(target, motion, now);
            }
        } else if state.key != key
            || state.values.iter().zip(targets).any(|(value, target)| value.target() != target)
        {
            for (value, target) in state.values.iter_mut().zip(targets) {
                value.snap_to(target, now);
            }
        }
        state.key = key.to_owned();
        let [x, y, width, height, bubble_x, bubble_y] = state.values.each_ref().map(|value| value.value(now));
        (
            Rect::new(x, y, width.max(0.0), height.max(0.0)),
            (bubble_x - bubble.x, bubble_y - bubble.y),
        )
    }

    fn enter_measurement_scope(&mut self) -> u64 {
        let previous = self.measurement_scope;
        self.next_measurement_scope += 1;
        self.measurement_scope = self.next_measurement_scope;
        previous
    }

    // Passive shortcut hints yield their whole chord before taking space
    // promised to a flexible action label. Use the same budget for intrinsic
    // measurement and final placement, including after a viewport resize.
    fn fit_row_shortcuts(
        &self,
        children: &[Element],
        sizes: &mut [Size],
        available: Size,
        gap: f32,
    ) {
        if !available.width.is_finite() {
            return;
        }
        let minimum = children
            .iter()
            .filter(|child| child.style.flex > 0.0)
            .map(|child| child.style.min_width)
            .sum::<f32>();
        let mut fixed = gap * children.len().saturating_sub(1) as f32
            + children
                .iter()
                .zip(sizes.iter())
                .filter(|(child, _)| child.style.flex <= 0.0)
                .map(|(_, size)| size.width)
                .sum::<f32>();
        for (child, size) in children.iter().zip(sizes.iter_mut()).rev() {
            if fixed + minimum <= available.width {
                break;
            }
            if child.style.flex <= 0.0
                && child.style.width.is_none()
                && matches!(child.kind,
                    ElementKind::KeyboardShortcut { .. } | ElementKind::ShortcutHint { .. })
            {
                let budget = (available.width - minimum - (fixed - size.width)).max(0.0);
                let constrained = self.measure(child, Size::new(budget, available.height));
                fixed += constrained.width - size.width;
                *size = constrained;
            }
        }
    }

    /// Where the value kept under `key` stands on this frame, heading for
    /// `target`. A key seen for the first time rests on its target.
    fn scalar(&self, key: &str, target: f32) -> f32 {
        let mut scalars = self.scalars.borrow_mut();
        let state = scalars.entry(key.to_owned()).or_insert_with(|| AnimatedScalar {
            value: ScalarTransition::new(target, self.now),
            seen: self.now,
        });
        state.seen = self.now;
        if state.value.target() != target {
            // A count is read, not followed: it takes three moves.
            let duration = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3);
            state
                .value
                .retarget_with_motion(target, self.now, duration, self.reduced_motion);
        }
        state.value.value(self.now)
    }

    /// Puts the text painted since `start` out of focus: each run becomes
    /// copies of itself over a disc of the radius kept under `id`, one in
    /// the middle, four half way out and eight on the rim, which together
    /// weigh what the sharp run did.
    fn soften(&mut self, id: &str, radius: f32, start: usize) {
        const RINGS: [(usize, f32, f32); 3] = [(1, 0.0, 0.0), (4, 0.5, 0.125), (8, 1.0, 0.0)];
        const TAPS: usize = 13;
        let radius = self.spring(&format!("soft/{id}"), radius);
        if radius < 0.05 {
            return;
        }
        let sharp: Vec<DrawCommand> = self.frame.commands.drain(start..).collect();
        for command in sharp {
            let color = match &command {
                DrawCommand::Text { color, .. } => color,
                DrawCommand::Glyphs(run) => &run.color,
                _ => {
                    self.frame.commands.push(command);
                    continue;
                }
            };
            // Where every copy overlaps, the ink is nearly what it was.
            let ink = f32::from(color.a) / 255.0;
            let share = if ink > 0.0 { (1.0 - (1.0 - 0.95 * ink).powf(1.0 / TAPS as f32)) / ink } else { 0.0 };
            for (count, reach, turn) in RINGS {
                for tap in 0..count {
                    let (sin, cos) = (std::f32::consts::TAU * (tap as f32 / count as f32 + turn)).sin_cos();
                    self.frame.commands.push(command.moved(radius * reach * cos, radius * reach * sin, share));
                }
            }
        }
    }

    /// As `scalar`, on a spring that covers its way in one move: for a value
    /// that is followed, and redirected before it arrives.
    fn spring(&self, key: &str, target: f32) -> f32 {
        let motion = crate::tokens().motion.move_ms;
        self.spring_paced(key, target, motion, motion)
    }

    /// As `spring`, covering its way in `rise_ms` towards a greater value
    /// and in `fall_ms` towards a lesser one.
    fn spring_paced(&self, key: &str, target: f32, rise_ms: u16, fall_ms: u16) -> f32 {
        let paced = |settle_ms: u16| crate::SpringMotion::settling_in(Duration::from_millis(u64::from(settle_ms)));
        let mut springs = self.springs.borrow_mut();
        let state = springs.entry(key.to_owned()).or_insert_with(|| AnimatedSpring {
            value: ScalarSpring::new(target, self.now),
            seen: self.now,
        });
        state.seen = self.now;
        if self.reduced_motion {
            state.value = ScalarSpring::new(target, self.now);
        } else if target != state.value.target() {
            let pace = paced(if target > state.value.target() { rise_ms } else { fall_ms });
            // It turns from where it is, at the speed it has, and takes its new pace from there.
            state.value.launch(target, pace, self.now);
        }
        state.value.value(self.now)
    }

    /// As `spring`, on `motion`: from rest it starts at standstill, and
    /// redirected it turns from where it is at the speed it has.
    fn spring_on(&self, key: &str, target: f32, motion: crate::SpringMotion) -> f32 {
        let mut springs = self.springs.borrow_mut();
        let state = springs.entry(key.to_owned()).or_insert_with(|| AnimatedSpring {
            value: ScalarSpring::new(target, self.now).motion(motion),
            seen: self.now,
        });
        state.seen = self.now;
        if self.reduced_motion {
            state.value = ScalarSpring::new(target, self.now).motion(motion);
        } else {
            state.value.retarget(target, self.now);
        }
        state.value.value(self.now)
    }

    /// The slot a child of a `keep_leaving` column answers to, if it has one.
    fn slot_id(child: &Element) -> Option<&str> {
        child
            .animate_height
            .and(child.presence.as_ref())
            .map(|(id, _)| id.as_str())
    }

    /// A `keep_leaving` column's children with, where each one was, an empty
    /// slot for every child that left and has not closed yet.
    fn with_leaving<'e>(&self, element: &'e Element, children: &'e [Element]) -> Cow<'e, [Element]> {
        let leaving = element.kept.as_ref().and_then(|kept| kept.leaving.as_deref());
        let Some(list) = leaving.filter(|_| !self.reduced_motion) else {
            return Cow::Borrowed(children);
        };
        let leaving = self.leaving.borrow();
        let Some((previous, _)) = leaving.get(list) else {
            return Cow::Borrowed(children);
        };
        let present = |id: &str| children.iter().any(|child| Self::slot_id(child) == Some(id));
        let closing = |id: &str| {
            self.flow_heights
                .borrow()
                .get(id)
                .is_some_and(|state| state.fraction.value(self.now) > 0.0)
        };
        if !previous.iter().any(|id| !present(id) && closing(id)) {
            return Cow::Borrowed(children);
        }
        let mut kept = children.to_vec();
        // Each one goes back after the slot it used to follow.
        let mut after: Option<&str> = None;
        for id in previous {
            if !present(id) && closing(id) {
                let index = after
                    .and_then(|after| kept.iter().position(|child| Self::slot_id(child) == Some(after)))
                    .map_or(0, |index| index + 1);
                kept.insert(index, Element::column(Vec::new()).animate_height(id.clone(), false));
            }
            if present(id) || closing(id) {
                after = Some(id);
            }
        }
        Cow::Owned(kept)
    }

    /// The text a rolling number reads as on this frame.
    fn rolling_text(&self, element: &Element) -> Option<Element> {
        let ElementKind::RollingNumber { id, value, format } = &element.kind else {
            return None;
        };
        let mut text = element.clone();
        text.kind = ElementKind::Text(format(self.scalar(id, *value)));
        Some(text)
    }

    fn measure_uncached(&self, element: &Element, available: Size) -> Size {
        if let Some(text) = self.rolling_text(element) {
            return self.measure_uncached(&text, available);
        }
        let style = element.style;
        let metrics = self.metrics_for(style);
        let padding = style.padding * 2.0;
        let inner = Size::new(
            (style.width.unwrap_or(available.width) - padding).max(0.0),
            (available.height - padding).max(0.0),
        );
        // A paragraph's width cap also constrains its wrapping. Containers
        // still measure intrinsically before their own final width is capped.
        let wrap_width = inner.width.min(style.max_width
            .map_or(f32::INFINITY, |max| (max - padding).max(0.0)));
        let natural = match &element.kind {
            ElementKind::Text(value) if self.text_layout => {
                let size = self.laid_out_text(value, style, wrap_width, false).size();
                Size::new(size.width + padding, size.height + padding)
            }
            ElementKind::Text(value)
            | ElementKind::ShimmerText {
                text: value,
                truncated: false,
                ..
            } => {
                let lines = self.wrapped_lines(value, wrap_width, style);
                let longest = lines
                    .iter()
                    .map(|line| self.text_width(line, style))
                    .fold(0.0, f32::max);
                Size::new(
                    longest + padding,
                    lines.len() as f32 * metrics.line_height + padding,
                )
            }
            ElementKind::TruncatedText(value)
            | ElementKind::FadedText { text: value, .. }
            | ElementKind::ShimmerText {
                text: value,
                truncated: true,
                ..
            } => Size::new(
                self.text_width(value, style) + padding,
                metrics.line_height + padding,
            ),
            ElementKind::KeyboardShortcut {
                keys,
                max_width,
                menu,
                ..
            } => {
                let width = if *menu {
                    self.menu_shortcut_width(keys)
                } else {
                    self.shortcut_width(keys)
                };
                let budget = max_width.unwrap_or(f32::INFINITY).min(inner.width);
                Size::new(
                    if width <= budget {
                        width + padding
                    } else {
                        0.0
                    },
                    shortcut::HEIGHT + padding,
                )
            }
            ElementKind::ShortcutHint { child, max_width } => {
                let natural = self.measure(child, Size::new(f32::INFINITY, inner.height));
                let budget = max_width.unwrap_or(f32::INFINITY).min(inner.width);
                Size::new(
                    if natural.width <= budget { natural.width + padding } else { 0.0 },
                    natural.height + padding,
                )
            }
            ElementKind::FormattedText(spans) if self.text_layout => {
                let size = self.laid_out_formatted(spans, style, wrap_width).size();
                Size::new(size.width + padding, size.height + padding)
            }
            ElementKind::FormattedText(spans) => {
                let lines = self.formatted_lines(spans, wrap_width, style);
                Size::new(
                    lines.iter().map(|line| line.width).fold(0.0, f32::max) + padding,
                    lines.iter().map(|line| line.height).sum::<f32>() + padding,
                )
            }
            ElementKind::RasterImage(_) | ElementKind::Surface { .. } => {
                Size::new(available.width.min(280.0), 180.0 + padding)
            }
            ElementKind::Icon(_) => {
                let side = style.icon_side.unwrap_or(ICON_SIZE);
                Size::new(side + padding, side + padding)
            }
            ElementKind::Button {
                label,
                trailing_icon,
                ..
            } => Size::new(
                self.text_width(label, style)
                    + if trailing_icon.is_some() {
                        style.gap + ICON_SIZE
                    } else {
                        0.0
                    }
                    + padding,
                metrics.line_height.max(if trailing_icon.is_some() {
                    ICON_SIZE
                } else {
                    0.0
                }) + padding,
            ),
            ElementKind::IconButton {
                label,
                trailing_icon,
                ..
            } => Size::new(
                ICON_SIZE
                    + if label.is_empty() {
                        0.0
                    } else {
                        style.gap + self.text_width(label, style)
                    }
                    + if trailing_icon.is_some() {
                        style.gap + ICON_SIZE
                    } else {
                        0.0
                    }
                    + padding,
                ICON_SIZE.max(metrics.line_height) + padding,
            ),
            ElementKind::TextInput { .. } => Size::new(
                if available.width.is_finite() {
                    available.width
                } else {
                    240.0
                },
                metrics.line_height + padding,
            ),
            ElementKind::CodeEditor(_) | ElementKind::CodeDiff(_) => {
                Size::new(available.width.max(0.0), available.height.max(24.0))
            }
            ElementKind::TextArea {
                id,
                initial_value,
                auto_grow,
                preserve_line_endings,
                ..
            } => {
                let height = if let Some(maximum) = auto_grow {
                    let value = self
                        .inputs
                        .get(id)
                        .map_or(initial_value.as_str(), |input| input.value.as_str());
                    let lines = if self.text_layout {
                        self.laid_out_text(value, style, inner.width, false).lines().count()
                    } else {
                        visual_lines_measured_endings(
                            value,
                            inner.width,
                            *preserve_line_endings,
                            |part| self.text_width(part, style),
                        )
                        .len()
                    };
                    let max_lines =
                        ((maximum - padding) / metrics.line_height).floor().max(1.0) as usize;
                    (lines.min(max_lines) as f32 * metrics.line_height + padding)
                        .clamp(style.min_height, *maximum)
                } else {
                    metrics.line_height * 4.0 + padding
                };
                Size::new(
                    if available.width.is_finite() {
                        available.width
                    } else {
                        240.0
                    },
                    height,
                )
            }
            ElementKind::EffortSlider { .. } => Size::new(
                if available.width.is_finite() {
                    available.width
                } else {
                    220.0
                },
                42.0 + padding,
            ),
            ElementKind::ProgressRing { .. } => {
                Size::new(crate::ICON_SIZE + padding, crate::ICON_SIZE + padding)
            }
            ElementKind::UnitGrid(grid) => {
                let size = grid.size();
                Size::new(size.width + padding, size.height + padding)
            }
            // The caller sizes it; left alone it is one dot.
            ElementKind::LevelBars(bars) => Size::new(bars.bar + padding, bars.bar + padding),
            // The caller sizes it.
            ElementKind::Drawing(_) => Size::new(padding, padding),
            // The caller sizes it; left alone it takes the width it is offered.
            ElementKind::HeatPlot(plot) => {
                let rows = plot.columns.iter().map(Vec::len).max().unwrap_or(0) as f32;
                let across = |count: f32| (count * (plot.side + plot.gap) - plot.gap).max(0.0);
                Size::new(across(plot.columns.len() as f32) + padding, across(rows) + padding)
            }
            ElementKind::BarPlot(_) | ElementKind::LinePlot(_) | ElementKind::PiePlot(_) | ElementKind::RadarPlot(_) => Size::new(
                if available.width.is_finite() { available.width } else { 480.0 },
                240.0 + padding,
            ),
            ElementKind::MarkRail(rail) => {
                // With less room than its marks ask for, they sit closer.
                let height = rail.marks.len() as f32 * rail.pitch;
                let room = (available.height - padding).max(0.0);
                Size::new(rail.peak + padding, height.min(room) + padding)
            }
            ElementKind::Flowchart(flowchart) => {
                let size = flowchart.layout(&|text, size| self.sans_width(text, size), &|size| {
                    self.sans_metrics.at_size(size).line_height
                })
                .size;
                Size::new(size.width + padding, size.height + padding)
            }
            ElementKind::SequenceDiagram(diagram) => {
                let size = diagram.layout(&|text, size| self.sans_width(text, size), &|size| {
                    self.sans_metrics.at_size(size).line_height
                })
                .size;
                Size::new(size.width + padding, size.height + padding)
            }
            // A rolling number was measured as its text above.
            ElementKind::Skeleton { .. }
            | ElementKind::Spinner { .. }
            | ElementKind::Record { .. }
            | ElementKind::RollingNumber { .. }
            | ElementKind::Spacer => Size::new(0.0, 0.0),
            ElementKind::Panel(child) | ElementKind::Clickable { child, .. } => {
                let child = self.measure(child, inner);
                Size::new(child.width + padding, child.height + padding)
            }
            // As large as the largest of what it holds.
            ElementKind::Stack(children) => {
                let (width, height) = children.iter().map(|child| self.measure(child, inner)).fold((0.0f32, 0.0f32), |(width, height), size| {
                    (width.max(size.width), height.max(size.height))
                });
                Size::new(width + padding, height + padding)
            }
            ElementKind::Overlay { base, .. } => {
                let base = self.measure(base, inner);
                Size::new(base.width + padding, base.height + padding)
            }
            ElementKind::Popover { anchor, .. } => {
                let anchor = self.measure(anchor, inner);
                Size::new(anchor.width + padding, anchor.height + padding)
            }
            ElementKind::Tooltip { anchor, .. } => {
                let anchor = self.measure(anchor, inner);
                Size::new(anchor.width + padding, anchor.height + padding)
            }
            ElementKind::HorizontalScroll { child, .. } => {
                let natural = self.measure(child, Size::new(f32::INFINITY, inner.height));
                Size::new(
                    if available.width.is_finite() {
                        available.width
                    } else {
                        natural.width + padding
                    },
                    natural.height + padding,
                )
            }
            ElementKind::Scroll { child, .. } => {
                let height = if style.max_height.is_some() {
                    (self
                        .measure(child, Size::new(inner.width, f32::INFINITY))
                        .height
                        + padding)
                        .min(available.height)
                } else if available.height.is_finite() {
                    available.height
                } else {
                    self.measure(child, Size::new(inner.width, f32::INFINITY))
                        .height
                        + padding
                };
                Size::new(available.width, height)
            }
            ElementKind::UniformList(list) => {
                let height = if available.height.is_finite() {
                    available.height
                } else if let Some(measured) = &list.measured {
                    let state = self.measured_lists.get(&list.id);
                    let rows: f32 = (0..list.row_count)
                        .map(|index| {
                            state
                                .and_then(|state| state.heights.get(&(measured.key)(index)))
                                .copied()
                                .unwrap_or(list.row_height)
                        })
                        .sum();
                    rows.min(f32::MAX / 4.0) + padding
                } else {
                    let rows = (list.row_count as f32 * list.row_height).min(f32::MAX / 4.0);
                    let open = match &list.expansion {
                        Some((index, content)) if *index < list.row_count => {
                            self.measure(content, Size::new(inner.width, f32::INFINITY)).height
                        }
                        _ => 0.0,
                    };
                    rows + open + padding
                };
                Size::new(available.width, height)
            }
            ElementKind::Column(children) => {
                let children = self.with_leaving(element, children);
                let sizes: Vec<_> = children
                    .iter()
                    .map(|child| self.measure(child, Size::new(inner.width, f32::INFINITY)))
                    .collect();
                let width = sizes.iter().map(|size| size.width).fold(0.0_f32, f32::max);
                let height = sizes.iter().map(|size| size.height).sum::<f32>()
                    + style.gap * children.len().saturating_sub(1) as f32;
                Size::new(width + padding, height + padding)
            }
            ElementKind::Row(children) if element.wrapping_line_gap().is_some() => {
                let line_gap = element.wrapping_line_gap().unwrap_or_default();
                // Wrap at the width it is placed at, which `max_width` caps.
                let max = style
                    .max_width
                    .map_or(f32::INFINITY, |max| (max - padding).max(0.0));
                let lines = Rect::new(0.0, 0.0, inner.width.min(max), 0.0);
                let (width, height) = self
                    .wrap_rects(children, lines, style.gap, line_gap)
                    .iter()
                    .fold((0.0_f32, 0.0_f32), |(width, height), rect| {
                        (
                            width.max(rect.x + rect.width),
                            height.max(rect.y + rect.height),
                        )
                    });
                Size::new(width + padding, height + padding)
            }
            ElementKind::Row(children) | ElementKind::TabStrip { children, .. } => {
                let mut sizes: Vec<_> = children
                    .iter()
                    .map(|child| self.measure(child, Size::new(f32::INFINITY, inner.height)))
                    .collect();
                self.fit_row_shortcuts(children, &mut sizes, inner, style.gap);
                self.fit_shrinking(children, &mut sizes, inner, style.gap);
                // Row layout reserves intrinsic widths for fixed siblings and
                // distributes the remainder between flex children. Measure
                // those children at the same assigned widths so wrapped text
                // contributes its actual height before the next row is placed.
                if inner.width.is_finite() {
                    let fixed = style.gap * children.len().saturating_sub(1) as f32
                        + children
                            .iter()
                            .zip(&sizes)
                            .filter(|(child, _)| child.style.flex <= 0.0)
                            .map(|(_, size)| size.width)
                            .sum::<f32>();
                    let weights = children
                        .iter()
                        .map(|child| child.style.flex)
                        .filter(|weight| *weight > 0.0)
                        .sum::<f32>();
                    if weights > 0.0 {
                        let flexible = (inner.width - fixed).max(0.0);
                        for (child, size) in children.iter().zip(&mut sizes) {
                            if child.style.flex > 0.0 {
                                let width = flexible * child.style.flex / weights;
                                *size = self.measure(child, Size::new(width, inner.height));
                            }
                        }
                    }
                }
                let width = sizes.iter().map(|size| size.width).sum::<f32>()
                    + style.gap * children.len().saturating_sub(1) as f32;
                let height = sizes.iter().map(|size| size.height).fold(0.0_f32, f32::max);
                Size::new(width + padding, height + padding)
            }
        };
        let fill_width = fills_available_width(&element.kind);
        let width = style.width.unwrap_or_else(|| {
            if fill_width && available.width.is_finite() {
                available.width
            } else {
                natural.width
            }
        });
        Size::new(
            width
                .min(style.max_width.unwrap_or(f32::INFINITY))
                .max(style.min_width),
            style
                .height
                .unwrap_or(natural.height)
                .min(style.max_height.unwrap_or(f32::INFINITY))
                .max(style.min_height),
        )
    }

    fn translated_rect(element: &Element, rect: Rect) -> Rect {
        match element.kept.as_ref().and_then(|kept| kept.translation) {
            Some(offset) => Rect::new(rect.x + offset.x, rect.y + offset.y, rect.width, rect.height),
            None => rect,
        }
    }

    fn has_translated_overflow(&mut self, element: &Element) -> bool {
        // These viewports clip their descendants even without clip_children.
        if element.style.clip_children || matches!(element.kind,
            ElementKind::Scroll { .. } | ElementKind::HorizontalScroll { .. } | ElementKind::UniformList(_)) {
            return false;
        }
        let key = (self.measurement_scope, element as *const Element as usize);
        if let Some(found) = self.translated_overflow.get(&key) { return *found; }
        let mut translated = |child: &Element| {
            child.kept.as_ref().is_some_and(|kept| kept.translation.is_some())
                || self.has_translated_overflow(child)
        };
        let found = match &element.kind {
            ElementKind::Column(children) | ElementKind::Row(children)
            | ElementKind::TabStrip { children, .. } | ElementKind::Stack(children) => children.iter().any(&mut translated),
            ElementKind::Panel(child) | ElementKind::Clickable { child, .. }
            | ElementKind::Tooltip { anchor: child, .. } => translated(child),
            ElementKind::Overlay { base, floating }
            | ElementKind::Popover { anchor: base, floating, .. } => translated(base) || translated(floating),
            _ => false,
        };
        self.translated_overflow.insert(key, found);
        found
    }

    fn place(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        self.place_translated(element, Self::translated_rect(element, rect), clip);
    }

    fn place_translated(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        let rect = self.extended(element, rect, clip);
        let dragged = element.draggable
            && !element.drag_in_place
            && self.drag_source.is_some()
            && focusable_id(element) == self.drag_source;
        let start = self.frame.commands.len();
        let inert = element.kept.as_ref().is_some_and(|kept| kept.inert).then_some((
            self.frame.regions.len(),
            self.frame.access.len(),
            self.frame.tab_focus.len(),
            self.frame.inline_menu_focus.len(),
            self.frame.context_menu_regions.len(),
            self.frame.text_blocks.len(),
        ));
        let reveal = element.hover_reveal;
        let around = self.region_hovered;
        if reveal == HoverReveal::Region {
            let visible = clip.map_or(Some(rect), |clip| rect.intersection(clip));
            let focused = self.keyboard_focus && focusable_id(element).is_some_and(|id| self.focused == Some(id));
            self.region_hovered = focused
                || self
                    .pointer
                    .zip(visible)
                    .is_some_and(|(point, visible)| visible.contains(point));
        }
        // A wrapper that is not itself a control speaks for the control inside
        // it: a field in its frame, a switch in its row.
        let hands_down = element.access.as_deref().filter(|said| said.id.is_none() && focusable_id(element).is_none());
        // The outermost control inside a wrapper is the one it spoke for: it
        // takes what was said before anything inside it can.
        let known = focusable_id(element).is_some() || element.access.as_deref().is_some_and(|said| said.id.is_some());
        let for_this = if known { self.access_above.take() } else { None };
        let above = self.access_above.take();
        self.access_above = match (hands_down, &above) {
            (Some(said), Some(outer)) => Some(outer.clone().over(said.clone())),
            (Some(said), None) => Some(said.clone()),
            (None, _) => above.clone(),
        };
        match (&element.presence, element.animate_height) {
            (Some((id, _)), Some(_)) => self.place_flow_height(element, id, rect, clip),
            _ => self.place_inner(element, rect, clip),
        }
        // Whoever was inside took it, or nobody was: either way it goes no further.
        let taken = self.access_above.is_none();
        self.access_above = if taken || known { None } else { above };
        self.region_hovered = around;
        if let Some((id, radius)) = element.kept.as_ref().and_then(|kept| kept.soft.as_ref()) {
            self.soften(id, *radius, start);
        }
        if let Some((id, factor, anchor)) = element.kept.as_ref().and_then(|kept| kept.scale.as_ref()) {
            let scale = self.spring(&format!("scale/{id}"), *factor);
            if scale != 1.0 {
                let origin = Point::new(rect.x + rect.width * anchor.x, rect.y + rect.height * anchor.y);
                for command in &mut self.frame.commands[start..] {
                    animate_popover_command(command, 1.0, 0.0, scale, origin);
                }
            }
        }
        match &element.presence {
            Some((id, _)) if matches!(reveal, HoverReveal::Revealed | HoverReveal::Concealed | HoverReveal::Rising) => {
                self.reveal_on_hover(element, id, start, reveal)
            }
            Some((id, leaves)) if element.animate_height.is_none() => {
                self.animate_surface(id, start, rect, if *leaves { 1.0 } else { -1.0 }, *leaves);
            }
            _ => {}
        }
        if let Some(replacement) = &element.replacement {
            self.animate_replacement(&replacement.0, &replacement.1, start, rect);
        }
        if dragged {
            self.drag_ghost = Some(start..self.frame.commands.len());
        }
        self.note_access(element, rect, clip, for_this);
        if let Some((regions, access, tabs, menus, context, text)) = inert {
            // Only its paint stays: nothing in it takes the pointer, the focus or a reader.
            self.frame.regions.truncate(regions);
            self.frame.access.truncate(access);
            self.frame.tab_focus.truncate(tabs);
            self.frame.inline_menu_focus.truncate(menus);
            self.frame.context_menu_regions.truncate(context);
            self.frame.text_blocks.truncate(text);
        }
    }

    /// The box an [`Element::extend_on_hover`] element is placed in: `rect`
    /// reaching out as far as its spring has got. Whether the pointer is
    /// over it is asked of the box it is heading for, so the pointer over
    /// what it shows keeps it out.
    fn extended(&self, element: &Element, rect: Rect, clip: Option<Rect>) -> Rect {
        let Some((id, by)) = element.kept.as_ref().and_then(|kept| kept.extend.as_ref()) else {
            return rect;
        };
        let length = by.x.abs().max(by.y.abs());
        if length == 0.0 {
            return rect;
        }
        let reach = |out: f32| {
            let (x, y) = (by.x * out / length, by.y * out / length);
            // An edge that springs back past where it rests draws the box in.
            let (left, width) = if by.x < 0.0 { (rect.x + x, rect.width - x) } else { (rect.x, rect.width + x) };
            let (top, height) = if by.y < 0.0 { (rect.y + y, rect.height - y) } else { (rect.y, rect.height + y) };
            Rect::new(left, top, width.max(0.0), height.max(0.0))
        };
        let key = format!("extend/{id}");
        let pushes = element.kept.as_ref().is_some_and(|kept| kept.extend_pushes);
        // A pushing reach was laid out already: the box is the room it has taken.
        let heading = if pushes { 0.0 } else { self.springs.borrow().get(&key).map_or(0.0, |state| state.value.target()) };
        let visible = clip.map_or(Some(reach(heading)), |clip| reach(heading).intersection(clip));
        let held = self.pointer.zip(visible).is_some_and(|(point, visible)| visible.contains(point))
            || holds_reveal(element, self.keyboard_focus, self.focused);
        let measured = self.springs.borrow().get(&key).map_or(0.0, |state| state.value.value(self.now));
        let out = self.spring_on(&key, if held { length } else { 0.0 }, bounce_motion());
        if pushes {
            // Measured before the pointer was known: a jump, as under reduced motion, is laid out again.
            if (out - measured).abs() > 0.01 {
                self.pushes_moved.set(true);
            }
            rect
        } else {
            reach(out)
        }
    }

    /// Record what the platform should know of `element`: what it says of
    /// itself over what its kind tells.
    fn note_access(&mut self, element: &Element, rect: Rect, clip: Option<Rect>, above: Option<crate::Access>) {
        let own = focusable_id(element);
        if own.is_none() && element.access.is_none() {
            return;
        }
        let mut said = element.access.as_deref().cloned().unwrap_or_default();
        let Some(id) = said.id.clone().or_else(|| own.map(str::to_owned)) else {
            // Something described without a name to know it by: what it said
            // was for the control inside it.
            return;
        };
        // What a wrapper said is for the control inside it, whether that is a
        // control or one drawn disabled and known by the ID it was given.
        if let Some(above) = above {
            said = above.over(said);
        }
        let Some(rect) = clip.map_or(Some(rect), |clip| rect.intersection(clip)) else {
            return;
        };
        // One control drawn in pieces, such as a link wrapped over two lines, is one node.
        if self.frame.access.iter().any(|node| node.id == id) {
            return;
        }
        let access = said.over(inferred_access(element));
        let actionable = own.is_some() && !access.disabled;
        self.frame.access.push(crate::AccessNode {
            focused: actionable && self.focused == Some(id.as_str()),
            id,
            rect,
            role: access.role.unwrap_or(crate::Role::Group),
            // A hint names what nothing else names, and describes what has a name.
            description: access.description.or_else(|| access.name.as_ref().and(access.hint.clone())).filter(|said| Some(said) != access.name.as_ref()),
            name: access.name.or(access.hint).unwrap_or_default(),
            disabled: access.disabled,
            toggled: access.toggled,
            mixed: access.mixed,
            selected: access.selected,
            expanded: access.expanded,
            value: access.value,
            value_text: access.value_text,
            actionable,
        });
    }

    fn place_inner(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        let scrollbar_visibility = element.kept.as_ref().map_or(crate::ScrollbarVisibility::Always, |kept| kept.scrollbar_visibility);
        if let Some(text) = self.rolling_text(element) {
            return self.place_inner(&text, rect, clip);
        }
        let foreground = element.kept.as_ref().and_then(|kept| kept.foreground.as_deref());
        if let Some(id) = foreground.filter(|_| !self.reduced_motion) {
            let given = element.style.foreground;
            let ink = {
                let mut inks = self.inks.borrow_mut();
                let (ink, seen) = inks
                    .entry(id.to_owned())
                    .or_insert_with(|| (ColorTransition::new(given, self.now), self.now));
                *seen = self.now;
                let duration = Duration::from_millis(u64::from(crate::tokens().motion.move_ms));
                ink.retarget(given, self.now, duration);
                ink.color(self.now)
            };
            if ink != given {
                let mut fading = element.clone();
                // A column keeps its children under the same box.
                fading.kept.as_mut().unwrap().foreground = None;
                fading.style.foreground = ink;
                let previous_scope = self.enter_measurement_scope();
                self.place_inner(&fading, rect, clip);
                self.measurement_scope = previous_scope;
                return;
            }
        }
        // Retain the un-clipped bounds of directly laid-out controls in a
        // scroll area. A menu can then focus its next option and reveal it,
        // even when that option has no visible hit region yet.
        self.record_scroll_focus_target(element, rect);
        if clip.is_some_and(|clip| rect.intersection(clip).is_none())
            && !self.has_translated_overflow(element) {
            if !self.scroll_focus_context.is_empty() {
                // Composite menu rows can be skipped before their controls
                // receive geometry. Keep only their IDs reachable; selecting
                // one uses the existing targeted probe to reveal its bounds.
                let mut focusables = Vec::new();
                collect_focus_order(element, &mut focusables, &mut HashSet::new());
                self.frame
                    .scroll_focus_candidates
                    .extend(focusables.into_iter().map(|(id, _)| id));
            }
            if self.focus_path.contains(&(element as *const Element as usize)) {
                self.probe_scroll_focus_translated(element, rect, clip);
            }
            return;
        }
        if let Some(id) = element.kept.as_ref().and_then(|kept| kept.pointer_blocker.as_deref()) {
            self.add_region(id, rect, HitKind::Surface, clip);
        }
        // Virtual rows and deferred overlays do not exist in the static root
        // walk. Register their actual, visible controls as they are placed.
        // The owner clears this map before each layout, so recycled/removed
        // rows cannot retain an alias from an earlier viewport or query.
        if let (Some(action), Some(id)) = (&element.action_alias, focusable_id(element)) {
            self.action_aliases.insert(id.to_owned(), action.clone());
        }
        if element.menu_secondary_action {
            if let Some(id) = focusable_id(element) {
                self.frame.menu_secondary_actions.insert(id.to_owned());
            }
        }
        if element.draggable {
            if let Some(id) = focusable_id(element) {
                self.frame.draggables.insert(id.to_owned());
            }
        }
        if let Some(cursor) = element.cursor {
            if let Some(id) = focusable_id(element) {
                self.frame.cursors.insert(id.to_owned(), cursor);
            }
        }
        if element.drop_target {
            let id = focusable_id(element).or(match &element.kind {
                ElementKind::Scroll { id, .. } | ElementKind::HorizontalScroll { id, .. } => {
                    Some(id.as_str())
                }
                ElementKind::UniformList(list) => Some(list.id.as_str()),
                _ => None,
            });
            if let Some(id) = id {
                self.frame.drop_targets.insert(id.to_owned());
            }
        }
        if element.menu_navigation {
            let mut focusables = Vec::new();
            collect_focus_order(element, &mut focusables, &mut HashSet::new());
            self.frame
                .inline_menu_focus
                .push(InlineMenuFocusScope { focusables });
        }
        if let Some(definition) = &element.context_menu {
            if let Some(visible) = clip.map_or(Some(rect), |clip| rect.intersection(clip)) {
                self.frame.context_menu_regions.push(ContextMenuRegion {
                    definition: definition.clone(),
                    rect: visible,
                    region_layer: self.frame.regions.len(),
                });
            }
        }
        let mut style = element.style;
        // Fill, border, focus ring and shadow share one radius that fits the
        // rectangle: what `f32::INFINITY` asks for, and an exact pill otherwise.
        style.radius = style.radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
        if let ElementKind::Skeleton {
            id,
            base,
            highlight,
        } = &element.kind
        {
            if !self.reduced_motion {
                self.observed_skeletons.insert(id.clone());
            }
            let started = self.skeleton_started.entry(id.clone()).or_insert(self.now);
            style.background = Some(skeleton_color(
                *base,
                *highlight,
                *started,
                self.now,
                self.reduced_motion,
            ));
        }
        let mut transition_id = None;
        if let ElementKind::Button { id, .. }
        | ElementKind::IconButton { id, .. }
        | ElementKind::Clickable { id, .. }
        | ElementKind::TextInput { id, .. }
        | ElementKind::TextArea { id, .. } = &element.kind
        {
            if style.action_menu_item {
                self.frame.action_menu_items.insert(id.clone());
            }
            let hovered = if style.action_menu_item && self.keyboard_focus {
                self.focused == Some(id.as_str())
            } else {
                self.hovered == Some(id.as_str())
                    || element.hover_peer.as_deref() == self.hovered && self.hovered.is_some()
            };
            // A handle dragged in place is held for as long as its drag lasts,
            // wherever the pointer has gone.
            let held = element.drag_in_place && self.drag_source == Some(id.as_str());
            if self.pressed == Some(id.as_str()) && hovered || held {
                style.background = style
                    .pressed_background
                    .or(style.hover_background)
                    .or(style.background);
                style.foreground = style
                    .pressed_foreground
                    .or(style.hover_foreground)
                    .unwrap_or(style.foreground);
            } else if hovered {
                style.background = style.hover_background.or(style.background);
                style.foreground = style.hover_foreground.unwrap_or(style.foreground);
                // On a shared surface that is already lit with the same fill,
                // the control's own hover would vanish: lift it one step.
                if let (Some(fill), Some(surface)) = (style.background, self.hover_surface_fill) {
                    if fill == surface && style.hover_background.is_some() {
                        let ink = style.foreground;
                        let lift = |a: u8, b: u8| ((u32::from(a) * 88 + u32::from(b) * 12 + 50) / 100) as u8;
                        style.background = Some(Color::rgba(
                            lift(fill.r, ink.r),
                            lift(fill.g, ink.g),
                            lift(fill.b, ink.b),
                            fill.a,
                        ));
                    }
                }
            }
            // An editor shows it on any focus; another control without a
            // ring shows it as its keyboard focus mark.
            if self.focused == Some(id.as_str())
                && (matches!(
                    &element.kind,
                    ElementKind::TextInput { .. } | ElementKind::TextArea { .. }
                ) || self.keyboard_focus && !style.focus_ring)
            {
                style.border = style.focus_border.or(style.border);
            }
            transition_id = Some(id);
        } else if let Some((id, actions)) = element.hover_surface.as_deref() {
            // Decoration only: the child actions still own all hit testing,
            // focus and disabled behavior. One transition avoids a seam when
            // moving between the primary action and a secondary icon.
            let followed = |target: Option<&str>| {
                target.is_some_and(|target| actions.iter().any(|action| action == target))
            };
            let in_gap = self.surface_gaps_lit
                && !actions.is_empty()
                && self.pointer.is_some_and(|point| {
                    rect.contains(point) && clip.is_none_or(|clip| clip.contains(point))
                });
            if followed(self.hovered) || in_gap {
                self.hover_surface_fill = style.hover_background;
                style.background = if followed(self.pressed) {
                    style
                        .pressed_background
                        .or(style.hover_background)
                        .or(style.background)
                } else {
                    style.hover_background.or(style.background)
                };
            }
            transition_id = Some(id);
        }
        if let Some(id) = transition_id.filter(|_| !self.reduced_motion && style.transition_ms > 0)
        {
            self.observed_buttons.insert(id.clone());
            let target = style.background.unwrap_or(Color::TRANSPARENT);
            let state = self
                .transitions
                .entry(id.clone())
                .or_insert_with(|| ColorTransition::new(target, self.now));
            state.retarget(
                target,
                self.now,
                Duration::from_millis(u64::from(style.transition_ms)),
            );
            let color = state.color(self.now);
            style.background = (color.a > 0).then_some(color);
        }
        let metrics = self.metrics_for(style);
        if let Some(shadow) = style.shadow {
            self.frame.commands.push(DrawCommand::Shadow {
                rect,
                shadow,
                radius: style.radius,
            });
        }
        if let Some(color) = style.background {
            paint_corner_rect(
                &mut self.frame.commands,
                rect,
                color,
                None,
                style.radius,
                style.corner_radii,
            );
        }
        if let Some((stops, vertical)) = element
            .kept
            .as_ref()
            .and_then(|kept| kept.gradient.as_ref())
        {
            paint_gradient(
                &mut self.frame.commands,
                rect,
                stops,
                *vertical,
                style.radius,
            );
        }
        if let Some((color, height)) = style.bottom_background_fade {
            if rect.width > 0.0 && rect.height > 0.0 {
                let height = height.min(rect.height);
                let fade = Rect::new(rect.x, rect.y + rect.height - height, rect.width, height);
                paint_fade(&mut self.frame.commands, fade, Color { a: 0, ..color }, color, style.radius, false);
            }
        }
        if let Some((color, height)) = style.top_background_fade {
            if rect.width > 0.0 && rect.height > 0.0 {
                let fade = Rect::new(rect.x, rect.y, rect.width, height.min(rect.height));
                paint_fade(&mut self.frame.commands, fade, color, Color { a: 0, ..color }, style.radius, true);
            }
        }
        if let Some(border) = style
            .border
            .filter(|_| style.corner_mask.is_none() || style.dashed_border)
        {
            if style.dashed_border {
                let dash: f32 = 5.0;
                let step = 9.0;
                let inset = style.radius.min(rect.width / 2.0).min(rect.height / 2.0);
                let mut x = rect.x + inset;
                while x < rect.x + rect.width - inset {
                    let length = dash.min(rect.x + rect.width - inset - x);
                    for y in [rect.y, rect.y + rect.height - border.width] {
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(x, y, length, border.width),
                            color: border.color,
                            radius: 0.0,
                        });
                    }
                    x += step;
                }
                let mut y = rect.y + inset;
                while y < rect.y + rect.height - inset {
                    let length = dash.min(rect.y + rect.height - inset - y);
                    for x in [rect.x, rect.x + rect.width - border.width] {
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(x, y, border.width, length),
                            color: border.color,
                            radius: 0.0,
                        });
                    }
                    y += step;
                }
                paint_dashed_corners(
                    &mut self.frame.commands,
                    rect,
                    inset,
                    border.width,
                    border.color,
                );
            } else {
                paint_corner_rect(
                    &mut self.frame.commands,
                    rect,
                    border.color,
                    Some(border.width),
                    style.radius,
                    style.corner_radii,
                );
            }
        }
        let content = rect.inset(style.padding);
        // A button's padding is horizontal: its label and icon are centered in
        // the full height, so deep padding on a short button cannot erase them.
        let content = if matches!(
            element.kind,
            ElementKind::Button { .. } | ElementKind::IconButton { .. }
        ) {
            Rect::new(content.x, rect.y, content.width, rect.height)
        } else {
            content
        };
        let parent_clip = clip;
        let clip = if style.clip_children {
            let child_clip = clip
                .and_then(|outer| outer.intersection(content))
                .unwrap_or_else(|| {
                    if clip.is_some() {
                        Rect::new(content.x, content.y, 0.0, 0.0)
                    } else {
                        content
                    }
                });
            self.frame.commands.push(DrawCommand::PushClip(child_clip));
            self.round_clip(content, &style);
            Some(child_clip)
        } else {
            clip
        };
        let mut pinned = Vec::new();
        match &element.kind {
            ElementKind::Column(children) => {
                let kept = self.with_leaving(element, children);
                let previous_scope = matches!(&kept, Cow::Owned(_)).then(|| self.enter_measurement_scope());
                let children = kept.as_ref();
                if let Some(list) = element.kept.as_ref().and_then(|kept| kept.leaving.as_deref()) {
                    let slots = children.iter().filter_map(Self::slot_id).map(str::to_owned).collect();
                    self.leaving.borrow_mut().insert(list.to_owned(), (slots, self.now));
                }
                let viewport = self
                    .scroll_focus_context
                    .last()
                    .filter(|_| children.iter().any(|child| child.sticky_top.is_some()))
                    .map(|scroll| scroll.viewport);
                match viewport {
                    Some(viewport) => {
                        self.place_sticky_column(children, content, style.gap, clip, viewport)
                    }
                    None => {
                        self.place_flex(children, content, style.gap, false, clip);
                    }
                }
                if let Some(scope) = previous_scope { self.measurement_scope = scope; }
            }
            ElementKind::Row(children) => {
                for (child, rect) in children
                    .iter()
                    .zip(self.child_rects(element, children, content))
                {
                    self.place(child, rect, clip);
                }
            }
            ElementKind::TabStrip {
                children,
                action_ids,
                selection,
                every_arrow,
            } => {
                let command_start = self.frame.commands.len();
                let rects = self.place_flex(children, content, style.gap, true, clip);
                if let Some(selection) = selection {
                    let index = children.iter().position(|child| match &child.kind {
                        ElementKind::Button { id, .. } | ElementKind::IconButton { id, .. } => {
                            id == &selection.selected_id
                        }
                        _ => child.indicator_slot.as_ref() == Some(&selection.selected_id),
                    });
                    if let Some(target) = index.and_then(|index| rects.get(index)).copied() {
                        self.observed_tabs.insert(selection.group_id.clone());
                        let state = self
                            .tab_indicators
                            .entry(selection.group_id.clone())
                            .or_insert_with(|| TabIndicatorState {
                                selected_id: selection.selected_id.clone(),
                                x: ScalarSpring::new(target.x, self.now),
                                width: ScalarSpring::new(target.width, self.now),
                            });
                        if self.reduced_motion || selection.animation_ms == 0 {
                            state.x.snap_to(target.x, self.now);
                            state.width.snap_to(target.width, self.now);
                            state.selected_id.clone_from(&selection.selected_id);
                        } else if state.selected_id != selection.selected_id {
                            let duration = Duration::from_millis(u64::from(selection.animation_ms));
                            let motion = crate::SpringMotion::settling_in(duration);
                            // Setting a pace before rebasing would redraw the
                            // flight so far on the new curve: a jump.
                            state.x.launch(target.x, motion, self.now);
                            state.width.launch(target.width, motion, self.now);
                            state.selected_id.clone_from(&selection.selected_id);
                        } else if state.x.target() != target.x
                            || state.width.target() != target.width
                        {
                            // Resizing a dock or window moves the entire strip;
                            // only changing selection should leave a sliding trail.
                            state.x.snap_to(target.x, self.now);
                            state.width.snap_to(target.width, self.now);
                        }
                        let indicator = Rect::new(
                            state.x.value(self.now),
                            target.y,
                            state.width.value(self.now),
                            target.height,
                        );
                        self.frame.commands.insert(
                            command_start,
                            DrawCommand::FillRect {
                                rect: indicator,
                                color: selection.color,
                                radius: selection.radius.min(indicator.width / 2.0).min(indicator.height / 2.0).max(0.0),
                            },
                        );
                    }
                }
                self.frame.tab_focus.push(TabFocusScope {
                    action_ids: action_ids.clone(),
                    vertical: false,
                    every_arrow: *every_arrow,
                });
            }
            ElementKind::Panel(child) => self.place(child, content, clip),
            ElementKind::Stack(children) => {
                for child in children {
                    self.place(child, content, clip);
                }
            }
            ElementKind::Clickable { id, child } => {
                self.add_region(id, rect, HitKind::Button, clip);
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
                let first = self.frame.commands.len();
                let first_block = self.frame.text_blocks.len();
                self.place(child, content, clip);
                if style.underline_on_hover
                    && (self.hovered == Some(id.as_str())
                        || self.keyboard_focus && self.focused == Some(id.as_str()))
                {
                    self.underline_text(first, first_block, content, style);
                }
                if self.keyboard_focus && self.focused == Some(id.as_str()) {
                    self.draw_focus_ring(rect, style);
                }
            }
            ElementKind::Overlay { base, floating } => {
                self.place(base, content, clip);
                if style.clip_children {
                    // Local layers (for example a thumbnail's remove button)
                    // keep their ancestor scroll clip and ordinary paint order.
                    self.place(floating, content, clip);
                } else {
                    self.overlays.push_back(DeferredOverlay::Free {
                        floating: floating.as_ref().clone(),
                        area: content,
                    });
                }
            }
            ElementKind::Popover {
                anchor,
                floating,
                dismiss_id,
                placement,
                gap,
                animation_ms,
                close_animation_ms,
                open_scale,
                view_key,
                hover_open,
                backdrop,
                focus_scope,
                backdrop_dismiss,
                scrim,
                initial_focus,
                return_focus,
                coach,
                arrows,
                at,
            } => {
                // A root wrapper receives the entire viewport, unlike a
                // child allocated by a flex parent. Honor its explicit anchor
                // constraints here too. Keep the entire anchor rectangle:
                // a composite header may contain focusable breadcrumbs, but
                // the first breadcrumb is not its positioning anchor.
                let anchor_rect = Rect::new(
                    content.x,
                    content.y,
                    element
                        .style
                        .width
                        .unwrap_or(content.width)
                        .min(content.width)
                        .max(0.0),
                    element
                        .style
                        .height
                        .unwrap_or(content.height)
                        .min(content.height)
                        .max(0.0),
                );
                self.place(anchor, anchor_rect, clip);
                let mut seen = HashSet::new();
                let mut anchor_focus = Vec::new();
                collect_focus_order(anchor, &mut anchor_focus, &mut seen);
                let trigger_id = anchor_focus.first().map(|(id, _)| id.clone());
                let mut focusables = Vec::new();
                collect_focus_order(floating, &mut focusables, &mut seen);
                if *hover_open {
                    if let Some(id) = &trigger_id {
                        self.frame.submenu_triggers.insert(id.clone());
                        if let Some((target, _)) =
                            focusables.iter().find(|(_, kind)| *kind == HitKind::Button)
                        {
                            self.frame
                                .submenu_targets
                                .insert(id.clone(), target.clone());
                        }
                        // Returning to a parent with Left/Escape must not
                        // immediately reopen the child merely because its
                        // trigger regained focus. Right or pointer movement
                        // into that trigger explicitly opens it again.
                        if self.closed_submenu_triggers.contains(id) {
                            return;
                        }
                    }
                    let hovered = self.hovered;
                    let focused = self.focused;
                    let surface_id = format!("popover_surface:{dismiss_id}");
                    let active = trigger_id
                        .as_deref()
                        .is_some_and(|id| hovered == Some(id) || focused == Some(id))
                        || hovered == Some(surface_id.as_str())
                        || focusables.iter().any(|(id, _)| {
                            hovered == Some(id.as_str()) || focused == Some(id.as_str())
                        })
                        || holds_submenu(floating, hovered, focused);
                    if !active {
                        return;
                    }
                }
                self.overlays.push_back(DeferredOverlay::Anchored {
                    floating: floating.as_ref().clone(),
                    anchor: at.map_or(anchor_rect, |point| Rect::new(point.x, point.y, 0.0, 0.0)),
                    dismiss_id: dismiss_id.clone(),
                    placement: *placement,
                    gap: *gap,
                    trigger_id,
                    focusables,
                    animation_ms: *animation_ms,
                    close_animation_ms: *close_animation_ms,
                    open_scale: *open_scale,
                    view_key: view_key.clone(),
                    backdrop: *backdrop,
                    focus_scope: *focus_scope,
                    backdrop_dismiss: *backdrop_dismiss,
                    scrim: *scrim,
                    initial_focus: initial_focus.clone(),
                    return_focus: return_focus.clone(),
                    coach: coach.clone(),
                    arrows: arrows.clone(),
                    pointed: at.is_some(),
                });
            }
            ElementKind::Tooltip {
                anchor,
                anchor_id,
                title,
                shortcut,
                sections,
                max_width,
                placement,
                colors,
                options,
            } => {
                let region_start = self.frame.regions.len();
                self.place(anchor, content, clip);
                if !self.frame.regions[region_start..]
                    .iter()
                    .any(|region| region.id == *anchor_id)
                {
                    let region_end = self.frame.regions.len();
                    self.add_region(anchor_id, content, HitKind::Hover, clip);
                    if self.frame.regions.len() > region_end {
                        // Descendant controls retain precedence over this hint.
                        self.frame.regions[region_start..].rotate_right(1);
                    }
                }
                let delay = Duration::from_millis(u64::from(options.delay_ms));
                self.observed_tooltips.insert(anchor_id.clone(), delay);
                if let Some(keyboard_focused) = self.hint_due(anchor_id, delay) {
                    // The hint points at the control it describes. Inside a
                    // larger anchor, such as a link in a paragraph, that is
                    // the control's own bounds, and the part of them under
                    // the pointer when it runs over several lines.
                    let target = {
                        let mut parts = self.frame.regions[region_start..]
                            .iter()
                            .filter(|region| region.id == *anchor_id)
                            .map(|region| region.rect);
                        let under = self
                            .pointer
                            .filter(|_| !keyboard_focused)
                            .and_then(|pointer| parts.clone().find(|rect| rect.contains(pointer)));
                        under.or_else(|| parts.next()).unwrap_or(content)
                    };
                    self.overlays.push_back(DeferredOverlay::Tooltip {
                        anchor: target,
                        title: title.clone(),
                        shortcut: shortcut.clone(),
                        sections: sections.clone(),
                        max_width: *max_width,
                        placement: *placement,
                        colors: *colors,
                        options: *options,
                    });
                }
            }
            ElementKind::HorizontalScroll {
                id,
                child,
                reveal,
                vertical_wheel,
                drag_pan,
            } => {
                self.paint_horizontal_scroll(
                    id,
                    child,
                    reveal.as_deref(),
                    content,
                    clip,
                    style.horizontal_edge_fade,
                    *vertical_wheel,
                    drag_pan.then_some(style.foreground),
                    scrollbar_visibility,
                );
            }
            ElementKind::Scroll {
                id,
                child,
                appearance,
                motion,
            } => {
                self.add_region(id, rect, HitKind::Scroll, clip);
                let full_width_height = self
                    .measure(child, Size::new(content.width, f32::INFINITY))
                    .height;
                let child_width =
                    appearance.content_width(content.width, full_width_height > content.height);
                let child_height = if child_width < content.width {
                    self.measure(child, Size::new(child_width, f32::INFINITY))
                        .height
                } else {
                    full_width_height
                };
                let max_offset = (child_height - content.height).max(0.0);
                self.frame.scroll_extents.insert(id.clone(), max_offset);
                let snap = self.scroll_to_end.remove(id).then_some(max_offset);
                let current_offset = self.scroll_position(id, max_offset, *motion, snap);
                let scrollbar =
                    ScrollbarGeometry::new(content, max_offset, current_offset, *appearance);
                let child_rect = Rect::new(
                    content.x,
                    content.y - current_offset,
                    child_width,
                    child_height.max(content.height),
                );
                let new_clip = clip
                    .and_then(|outer| outer.intersection(content))
                    .or_else(|| clip.is_none().then_some(content));
                if let Some(new_clip) = new_clip {
                    self.frame.commands.push(DrawCommand::PushClip(new_clip));
                    self.round_clip(content, &style);
                    self.scroll_focus_context.push(ScrollFocusContext {
                        id: id.clone(),
                        viewport: new_clip,
                        offset: current_offset,
                    });
                    let outer = std::mem::take(&mut self.pinned);
                    self.place(child, child_rect, Some(new_clip));
                    pinned = std::mem::replace(&mut self.pinned, outer);
                    self.scroll_focus_context.pop();
                    self.frame.commands.push(DrawCommand::PopClip);
                }
                if let Some(scrollbar) = scrollbar {
                    self.paint_scrollbar(
                        id,
                        scrollbar,
                        content,
                        clip,
                        style.foreground,
                        *appearance,
                        scrollbar_visibility,
                    );
                }
            }
            ElementKind::UniformList(list) if list.measured.is_some() => {
                self.place_measured_list(list, rect, content, clip, style.foreground, scrollbar_visibility);
            }
            ElementKind::UniformList(list) => {
                let id = list.id.as_str();
                self.add_region(id, rect, HitKind::Scroll, clip);
                let rows_height = (list.row_count as f32 * list.row_height).min(f32::MAX / 4.0);
                // What a row opened under itself, at the height it takes in
                // the width the rows have: the narrower one while a bar shows.
                let open = list
                    .expansion
                    .as_ref()
                    .filter(|(index, _)| *index < list.row_count);
                let mut extra = 0.0;
                if let Some((_, opened)) = open {
                    let narrow = list.appearance.content_width(content.width, true);
                    extra = self.measure(opened, Size::new(narrow, f32::INFINITY)).height;
                    if rows_height + extra <= content.height && narrow != content.width {
                        extra = self.measure(opened, Size::new(content.width, f32::INFINITY)).height;
                    }
                }
                // The rows from `split` on stand under what was opened.
                let split = open.map_or(usize::MAX, |(index, _)| index + 1);
                let parted = open.map_or(f32::INFINITY, |(index, _)| (index + 1) as f32 * list.row_height);
                let row_top =
                    |index: usize| index as f32 * list.row_height + if index >= split { extra } else { 0.0 };
                let content_height = rows_height + extra;
                let max_offset = (content_height - content.height).max(0.0);
                self.frame
                    .scroll_extents
                    .insert(list.id.clone(), max_offset);
                let mut snap = self.scroll_to_end.remove(id).then_some(max_offset);
                if let Some(index) = self.scroll_to_index.remove(id) {
                    let mut offset =
                        snap.unwrap_or_else(|| self.scroll_offsets.get(id).copied().unwrap_or(0.0));
                    if list.row_count > 0 {
                        let index = index.min(list.row_count - 1);
                        let top = row_top(index);
                        let bottom = top + list.row_height;
                        if top < offset {
                            offset = top;
                        } else if bottom > offset + content.height {
                            offset = bottom - content.height;
                        }
                    }
                    snap = Some(offset);
                }
                let current_offset = self.scroll_position(id, max_offset, list.motion, snap);
                let scrollbar =
                    ScrollbarGeometry::new(content, max_offset, current_offset, list.appearance);
                let row_width = list
                    .appearance
                    .content_width(content.width, scrollbar.is_some());
                // A height of the content as a height of the rows alone: what
                // was opened stands between two of them and holds no row.
                let among_rows = |y: f32| if y <= parted { y } else { (y - extra).max(parted) };
                let first_visible = (among_rows(current_offset) / list.row_height).floor() as usize;
                let end_visible = (among_rows(current_offset + content.height) / list.row_height)
                    .ceil() as usize;
                let mut first_visible = first_visible.min(list.row_count);
                let end_visible = end_visible.min(list.row_count);
                // What a row opened is built with its row and only with it, so
                // the keyboard walks from one into the other. Its row is built
                // while any of it shows, even once the row itself scrolled off.
                let opened_shows = extra > 0.0
                    && parted + extra > current_offset
                    && parted < current_offset + content.height;
                if opened_shows && first_visible >= split {
                    first_visible = split - 1;
                }
                let visible = clip
                    .and_then(|outer| outer.intersection(content))
                    .or_else(|| clip.is_none().then_some(content));
                let mut rows = Vec::new();
                if let Some(visible) = visible {
                    self.frame.commands.push(DrawCommand::PushClip(visible));
                    for index in first_visible..end_visible {
                        let row = (list.build_row)(index);
                        let previous_scope = self.enter_measurement_scope();
                        let mut focusables = Vec::new();
                        let mut seen = HashSet::new();
                        collect_focus_order(&row, &mut focusables, &mut seen);
                        self.place(
                            &row,
                            Rect::new(
                                content.x,
                                content.y + row_top(index) - current_offset,
                                row_width,
                                list.row_height,
                            ),
                            Some(visible),
                        );
                        if let Some((_, opened)) = open.filter(|(open, _)| *open == index) {
                            collect_focus_order(opened, &mut focusables, &mut seen);
                            self.place(
                                opened,
                                Rect::new(
                                    content.x,
                                    content.y + parted - current_offset,
                                    row_width,
                                    extra,
                                ),
                                Some(visible),
                            );
                        }
                        rows.push((index, focusables));
                        self.measurement_scope = previous_scope;
                    }
                    self.frame.commands.push(DrawCommand::PopClip);
                }
                self.frame.virtual_lists.insert(
                    list.id.clone(),
                    VirtualListFocus {
                        first_visible,
                        end_visible,
                        row_count: list.row_count,
                        rows,
                    },
                );
                if let Some(scrollbar) = scrollbar {
                    self.paint_scrollbar(
                        id,
                        scrollbar,
                        content,
                        clip,
                        style.foreground,
                        list.appearance,
                        scrollbar_visibility,
                    );
                }
            }
            ElementKind::FadedText { text, fade_width } => {
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
                // Reuse the same shaped line and native clip primitives as
                // shimmer text. Alpha bands keep arbitrary hover/selection
                // backgrounds correct without painting an opaque mask.
                let bands = if fade > 0.0 { 16 } else { 0 };
                for band in 0..=bands {
                    let (left, right, opacity) = if band == 0 {
                        (content.x, content.x + solid, 1.0)
                    } else {
                        let start = (band - 1) as f32 / bands as f32;
                        let end = band as f32 / bands as f32;
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
                        let ink = style.foreground;
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
            ElementKind::TruncatedText(value)
            | ElementKind::ShimmerText {
                text: value,
                truncated: true,
                ..
            } => {
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
                if let Some((id, highlight)) = shimmer {
                    self.observed_shimmers.insert(id.clone());
                    let started = *self.shimmer_started.entry(id.clone()).or_insert(self.now);
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
            ElementKind::ShortcutHint { child, max_width } => {
                let natural = self.measure(child, Size::new(f32::INFINITY, content.height));
                if natural.width <= max_width.unwrap_or(f32::INFINITY).min(content.width) {
                    self.place(child, content, clip);
                }
            }
            ElementKind::KeyboardShortcut {
                keys,
                fill,
                max_width,
                menu,
            } => {
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
            ElementKind::Text(value) if self.text_layout => {
                self.paint_laid_out_text(value, content, style, clip);
            }
            ElementKind::Text(value)
            | ElementKind::ShimmerText {
                text: value,
                truncated: false,
                ..
            } => {
                let shimmer = if let ElementKind::ShimmerText { id, highlight, .. } = &element.kind
                {
                    if !self.reduced_motion && !value.is_empty() && content.width > 0.0 {
                        self.observed_shimmers.insert(id.clone());
                        let started = *self.shimmer_started.entry(id.clone()).or_insert(self.now);
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
                    runs: Vec::new(),
                    region_layer: self.frame.regions.len(),
                    layout: None,
                });
                let mut source_byte = 0;
                let mut source_char = 0;
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
                    }
                    source_byte += line.len();
                    source_char += count;
                }
            }
            ElementKind::FormattedText(spans) if self.text_layout => {
                self.paint_laid_out_formatted(spans, content, style, clip);
            }
            ElementKind::FormattedText(spans) => {
                let source = spans
                    .iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>();
                let block_index = self.frame.text_blocks.len();
                self.frame.text_blocks.push(SelectableTextBlock {
                    source: source.clone(),
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
                                    self.draw_focus_ring(fragment_rect, style);
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
                                self.draw_focus_ring(fragment_rect, style);
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
            ElementKind::RasterImage(image) => {
                self.frame.commands.push(DrawCommand::RasterImage {
                    rect: content,
                    image: image.clone(),
                    opacity: 1.0,
                    radius: style.radius,
                });
            }
            ElementKind::Surface { id, label } => {
                self.frame.commands.push(DrawCommand::Surface {
                    rect: content,
                    id: id.clone(),
                    label: label.clone(),
                    opacity: 1.0,
                });
            }
            ElementKind::Icon(icon) => {
                let side = style
                    .icon_side
                    .unwrap_or(ICON_SIZE)
                    .min(content.width)
                    .min(content.height)
                    .max(0.0);
                let icon_rect = Rect::new(
                    content.x + (content.width - side) / 2.0,
                    content.y + (content.height - side) / 2.0,
                    side,
                    side,
                );
                self.draw_icon(*icon, icon_rect, style);
            }
            ElementKind::Button {
                id,
                label,
                trailing_icon,
            } => {
                self.add_region(id, rect, HitKind::Button, clip);
                if self.keyboard_focus && self.focused == Some(id.as_str()) {
                    self.draw_focus_ring(rect, style);
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
            ElementKind::IconButton {
                id,
                icon,
                label,
                trailing_icon,
            } => {
                self.add_region(id, rect, HitKind::Button, clip);
                if self.keyboard_focus && self.focused == Some(id.as_str()) {
                    self.draw_focus_ring(rect, style);
                }
                let side = ICON_SIZE.min(content.width).min(content.height).max(0.0);
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
                self.draw_icon(*icon, icon_rect, style);
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
            ElementKind::TextInput {
                id,
                initial_value,
                placeholder,
                persistent_placeholder,
                leading_icon,
                leading_icon_tint,
                shortcut,
                shortcut_fill,
                password,
            } => {
                self.add_region(id, rect, HitKind::TextInput, clip);
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
                // The line sits in the middle of a field taller than its text.
                let slack = (content.height - metrics.line_height).max(0.0);
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
                        placeholder.clone()
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
            ElementKind::CodeEditor(editor) => {
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
                            crate::code_editor::EditorView::new(editor.clone(), self.now)
                        });
                    state.update(editor.clone());
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
                            options: TooltipOptions::default(),
                        });
                    }
                    for control in controls {
                        self.place(&control.element, control.rect, Some(control.clip));
                    }
                }
            }
            ElementKind::CodeDiff(diff) => {
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
                }
            }
            ElementKind::TextArea {
                id,
                initial_value,
                placeholder,
                preserve_line_endings,
                enter_submits,
                highlights,
                ..
            } => {
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
                    self.paint_laid_out_area(id, rect, content, style, clip, placeholder, initial_value, highlights);
                } else {
                    state.laid_out = None;
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
                    let is_placeholder = state.value.is_empty() && !is_focused;
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
            ElementKind::Flowchart(flowchart) => {
                let mut commands = Vec::new();
                flowchart.paint(
                    content,
                    &|text, size| self.sans_width(text, size),
                    &|size| self.sans_metrics.at_size(size).line_height,
                    &mut commands,
                );
                self.frame.commands.append(&mut commands);
            }
            ElementKind::SequenceDiagram(diagram) => {
                let mut commands = Vec::new();
                diagram.paint(
                    content,
                    &|text, size| self.sans_width(text, size),
                    &|size| self.sans_metrics.at_size(size).line_height,
                    &mut commands,
                );
                self.frame.commands.append(&mut commands);
            }
            ElementKind::UnitGrid(grid) => {
                // Each part's cells follow those of the parts before it.
                let focused = grid.focus.filter(|focus| *focus < grid.parts.len());
                let mut end = 0.0;
                let spans: Vec<(f32, f32, Color)> = grid
                    .parts
                    .iter()
                    .enumerate()
                    .map(|(index, part)| {
                        let key = format!("{}/{}", grid.id, part.key);
                        let cells = self.scalar(&key, part.cells.max(0.0)).max(0.0);
                        let receded = focused.is_some_and(|focus| focus != index);
                        let recede = self.scalar(&format!("{key}/recede"), f32::from(u8::from(receded)));
                        let start = end;
                        end += cells;
                        (start, end, mix(part.color, grid.empty, recede * 0.78))
                    })
                    .collect();
                let pitch = grid.cell + grid.gap;
                for row in 0..grid.rows {
                    for column in 0..grid.columns {
                        let cell = f32::from(row) * f32::from(grid.columns) + f32::from(column);
                        // A cell a part covers in part shows that share of its colour.
                        let color = spans.iter().fold(grid.empty, |color, (start, end, ink)| {
                            let cover = (end.min(cell + 1.0) - start.max(cell)).clamp(0.0, 1.0);
                            if cover > 0.0 { mix(color, *ink, cover) } else { color }
                        });
                        self.frame.commands.push(DrawCommand::FillRect {
                            rect: Rect::new(
                                content.x + f32::from(column) * pitch,
                                content.y + f32::from(row) * pitch,
                                grid.cell,
                                grid.cell,
                            ),
                            color,
                            radius: grid.radius,
                        });
                    }
                }
            }
            ElementKind::BarPlot(plot) => self.paint_bar_plot(plot, content, clip),
            ElementKind::LinePlot(plot) => self.paint_line_plot(plot, content, clip),
            ElementKind::PiePlot(plot) => self.paint_pie_plot(plot, content, clip),
            ElementKind::RadarPlot(plot) => self.paint_radar_plot(plot, content, clip),
            ElementKind::HeatPlot(plot) => self.paint_heat_plot(plot, content, clip),
            ElementKind::Drawing(drawing) => {
                self.frame.commands.push(DrawCommand::PushClip(content));
                // The drawing's own clips stay balanced, so none outlives it.
                let mut open = 0;
                for command in drawing.draw(Size::new(content.width, content.height)) {
                    match command {
                        DrawCommand::PushClip(_) => open += 1,
                        DrawCommand::PopClip if open == 0 => continue,
                        DrawCommand::PopClip => open -= 1,
                        _ => {}
                    }
                    self.frame.commands.push(command.moved(content.x, content.y, 1.0));
                }
                self.frame.commands.extend(std::iter::repeat_n(DrawCommand::PopClip, open + 1));
            }
            ElementKind::LevelBars(bars) => {
                let pitch = bars.bar + bars.gap;
                let fit = if bars.bar > 0.0 && pitch > 0.0 { ((content.width + bars.gap) / pitch).floor() } else { 0.0 };
                let slots = fit.clamp(0.0, f32::from(MAX_LEVEL_SLOTS)) as usize;
                // The newest level takes the last slot; the levels that do
                // not fit are the oldest, and slots without one stay dots.
                let skipped = bars.levels.len().saturating_sub(slots);
                let empty = slots.saturating_sub(bars.levels.len());
                let left = content.x + content.width - (slots as f32 * pitch - bars.gap).max(0.0);
                for slot in 0..slots {
                    let level = slot
                        .checked_sub(empty)
                        .and_then(|index| bars.levels.get(skipped + index))
                        .filter(|level| level.is_finite())
                        .map_or(0.0, |level| level.clamp(0.0, 1.0));
                    // A peak shows almost at once and falls back slowly, as a
                    // meter's needle does: a bar that chased every dip would flicker.
                    let motion = crate::tokens().motion;
                    let shown = self
                        .spring_paced(&format!("{}/{slot}", bars.id), level, motion.hover_ms, motion.move_ms * 2)
                        .clamp(0.0, 1.0);
                    let height = bars.bar + (content.height - bars.bar).max(0.0) * shown;
                    self.frame.commands.push(DrawCommand::FillRect {
                        rect: Rect::new(
                            left + slot as f32 * pitch,
                            content.y + (content.height - height) / 2.0,
                            bars.bar,
                            height,
                        ),
                        // A dot takes the bars' ink as soon as it stands up.
                        color: mix(bars.rest, bars.color, shown * 4.0),
                        // Half the width is a shape, not a token.
                        radius: bars.bar / 2.0,
                    });
                }
            }
            ElementKind::MarkRail(rail) => {
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
                        options: TooltipOptions::default(),
                    });
                }
                self.frame.tab_focus.push(TabFocusScope {
                    action_ids: rail.marks.iter().map(|mark| mark.id.clone()).collect(),
                    vertical: true,
                    every_arrow: false,
                });
            }
            ElementKind::ProgressRing {
                fraction,
                track,
                fill,
            } => {
                self.frame.commands.push(DrawCommand::ProgressRing {
                    rect: content,
                    fraction: *fraction,
                    track: *track,
                    fill: *fill,
                });
            }
            ElementKind::Record { id, image, out, spinning, peek } => {
                self.observed_records.insert(id.clone());
                let now = self.now;
                let peeked = peek.as_deref().is_some_and(|control| self.hovered == Some(control));
                let target = if *out || peeked { 1.0 } else { 0.0 };
                let motion = self.record_motion.entry(id.clone()).or_insert_with(|| RecordMotion {
                    out: ScalarTransition::new(target, now),
                    turned: 0.0,
                    spinning_since: None,
                });
                if motion.out.target() != target {
                    let travel = Duration::from_millis(u64::from(crate::tokens().motion.move_ms));
                    motion.out.retarget_with_motion(target, now, travel, self.reduced_motion);
                }
                motion.spin(*spinning && !self.reduced_motion, now);
                if rect.width > 0.0 && rect.height > 0.0 {
                    self.frame.commands.push(DrawCommand::Record {
                        rect,
                        image: image.clone(),
                        fill: style.background.unwrap_or(Color::TRANSPARENT),
                        out: motion.out.value(now),
                        angle: motion.angle(now),
                        radius: style.radius,
                        opacity: 1.0,
                    });
                }
            }
            ElementKind::Spinner { id, color, variant } => {
                if !self.reduced_motion {
                    self.observed_spinners.insert(id.clone());
                }
                let started = self.spinner_started.entry(id.clone()).or_insert(self.now);
                paint_spinner(
                    &mut self.frame.commands,
                    rect,
                    *color,
                    *variant,
                    *started,
                    self.now,
                    self.reduced_motion,
                );
            }
            ElementKind::EffortSlider {
                id,
                fraction,
                steps,
                ultra_available,
            } => {
                self.observed_sliders.insert(id.clone());
                let drag = self.slider_drag.filter(|drag| drag.id == *id);
                let motion = self
                    .slider_motion
                    .entry(id.clone())
                    .or_insert_with(|| SliderMotion::new(self.now));
                let (phase, visual_fraction, ultra_mix, burst_age) = motion.sample(
                    self.now,
                    *fraction,
                    drag.map(|drag| drag.fraction),
                    *steps,
                    *ultra_available,
                    self.reduced_motion,
                );
                self.frame.commands.push(DrawCommand::EffortSlider {
                    rect: content,
                    fraction: visual_fraction,
                    steps: *steps,
                    phase,
                    dragging: drag.is_some(),
                    reduced_motion: self.reduced_motion,
                    ultra_mix,
                    burst_age,
                    opacity: 1.0,
                });
                self.add_region(id, rect, HitKind::Button, clip);
                self.frame.sliders.insert(
                    id.clone(),
                    SliderGeometry {
                        rect: content,
                        steps: *steps,
                        fraction: *fraction,
                        ultra_available: *ultra_available,
                        inset: SLIDER_THUMB_INSET,
                        animated: true,
                        y: None,
                        vertical: false,
                    },
                );
                if self.keyboard_focus && self.focused == Some(id.as_str()) {
                    self.draw_focus_ring(
                        rect,
                        Style {
                            radius: style.radius.max(8.0),
                            ..style
                        },
                    );
                }
            }
            // A rolling number was placed as its text above.
            ElementKind::Skeleton { .. }
            | ElementKind::RollingNumber { .. }
            | ElementKind::Spacer => {}
        }
        let fade_area = |mut area: Rect| {
            let top = style.fade_insets.0.clamp(0.0, area.height);
            let bottom = style.fade_insets.1.clamp(0.0, area.height);
            area.y += top;
            area.height = (area.height - top - bottom).max(0.0);
            area
        };
        if let Some((color, height)) = style.top_fade {
            let scroll_area = |id: &str| {
                (self.scroll_offsets.get(id).copied().unwrap_or(0.0) > 0.0).then(|| {
                    let width = self
                        .frame
                        .scrollbars
                        .get(id)
                        .map_or(content.width, |bar| (bar.rail.x - content.x).max(0.0));
                    Rect::new(content.x, content.y, width, content.height)
                })
            };
            // A scroll fade marks content above the viewport, not a permanent
            // veil over its first row. Keep scrollbars and code gutters clear.
            let area = match &element.kind {
                ElementKind::Scroll { id, .. } => scroll_area(id),
                ElementKind::UniformList(list) => scroll_area(&list.id),
                ElementKind::CodeEditor(editor) => self
                    .code_editors
                    .get(&editor.id)
                    .filter(|state| state.viewport.scroll_y > 0.0)
                    .map(|state| state.content_visible()),
                ElementKind::CodeDiff(diff) => self
                    .code_diffs
                    .get(&diff.id)
                    .filter(|state| state.viewport.scroll_y > 0.0)
                    .map(|state| state.viewport.body),
                _ => Some(content),
            };
            if let Some(area) =
                area.map(fade_area).filter(|area| area.width > 0.0 && area.height > 0.0 && height > 0.0)
            {
                self.frame.commands.push(DrawCommand::FillVerticalGradient {
                    rect: Rect::new(area.x, area.y, area.width, height.min(area.height)),
                    top: color,
                    bottom: Color { a: 0, ..color },
                });
            }
        }
        if let Some((color, height)) = style.bottom_fade {
            let scroll_area = |id: &str| {
                let offset = self.scroll_offsets.get(id).copied().unwrap_or(0.0);
                let maximum = self.frame.scroll_extents.get(id).copied().unwrap_or(0.0);
                (maximum - offset > 0.5).then(|| {
                    let width = self
                        .frame
                        .scrollbars
                        .get(id)
                        .map_or(content.width, |bar| (bar.rail.x - content.x).max(0.0));
                    Rect::new(content.x, content.y, width, content.height)
                })
            };
            let area = match &element.kind {
                ElementKind::Scroll { id, .. } => scroll_area(id),
                ElementKind::UniformList(list) => scroll_area(&list.id),
                _ => Some(content),
            };
            if let Some(area) = area.map(fade_area).filter(|area| area.width > 0.0 && area.height > 0.0) {
                let height = height.min(area.height);
                let fade = Rect::new(area.x, area.y + area.height - height, area.width, height);
                if let Some(visible) = clip.map_or(Some(fade), |clip| fade.intersection(clip)) {
                    // Paint after content, inside its viewport. This is not a
                    // hit surface and never spills into a sibling composer.
                    self.frame.commands.push(DrawCommand::PushClip(visible));
                    self.frame.commands.push(DrawCommand::FillVerticalGradient {
                        rect: fade,
                        top: Color { a: 0, ..color },
                        bottom: color,
                    });
                    self.frame.commands.push(DrawCommand::PopClip);
                }
            }
        }
        if let ElementKind::Scroll { id, .. } = &element.kind {
            self.place_pinned(id, pinned, style.top_fade.map(|(color, _)| color));
        }
        if style.clip_children {
            self.frame.commands.push(DrawCommand::PopClip);
        }
        if style.corner_mask.is_some() {
            corner_mask_tree::finish_rounded_surface(
                &mut self.frame.commands,
                rect,
                parent_clip,
                style,
            );
        }
    }

    fn record_scroll_focus_target(&mut self, element: &Element, rect: Rect) {
        // Tooltips are transparent wrappers. Retain their anchor's un-clipped
        // bounds even when place() skips the wrapper before visiting its child.
        // Menu keyboard navigation can then reach and reveal offscreen options.
        if let ElementKind::Tooltip { anchor, .. } = &element.kind {
            self.record_scroll_focus_target(anchor, Self::translated_rect(anchor, rect.inset(element.style.padding)));
        }
        // A control that is also an anchor answers to both names.
        let ids = [focusable_id(element), element.anchor.as_deref()];
        if let Some(scroll) = self.scroll_focus_context.last() {
            let (scroll_id, viewport, layout_offset) = (scroll.id.clone(), scroll.viewport, scroll.offset);
            for id in ids.into_iter().flatten() {
                let ancestors = self.scroll_focus_ancestors();
                self.frame.scroll_focus_targets.insert(
                    id.to_owned(),
                    ScrollFocusTarget {
                        scroll_id: scroll_id.clone(),
                        rect,
                        viewport,
                        layout_offset,
                        ancestors,
                    },
                );
            }
        }
    }

    fn scroll_focus_ancestors(&self) -> Vec<(String, Rect, f32)> {
        let outer = self.scroll_focus_context.len().saturating_sub(1);
        self.scroll_focus_context[..outer]
            .iter()
            .map(|scroll| (scroll.id.clone(), scroll.viewport, scroll.offset))
            .collect()
    }

    fn record_inline_focus_target(&mut self, id: &str, rect: Rect) {
        let ancestors = self.scroll_focus_ancestors();
        if let Some(scroll) = self.scroll_focus_context.last() {
            // A multi-line link is one keyboard stop. Reveal its first painted
            // fragment, without creating a giant rectangle over nearby prose.
            self.frame
                .scroll_focus_targets
                .entry(id.to_owned())
                .or_insert_with(|| ScrollFocusTarget {
                    scroll_id: scroll.id.clone(),
                    rect,
                    viewport: scroll.viewport,
                    layout_offset: scroll.offset,
                    ancestors,
                });
        }
    }

    /// Resolve anchors and the requested control inside a clipped branch.
    /// This does not paint, add hit regions, or visit unrelated descendants.
    fn probe_scroll_focus(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        self.probe_scroll_focus_translated(element, Self::translated_rect(element, rect), clip);
    }

    fn probe_scroll_focus_translated(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        self.record_scroll_focus_target(element, rect);
        let style = element.style;
        let content = rect.inset(style.padding);
        let clip = if style.clip_children {
            Some(
                clip.and_then(|outer| outer.intersection(content))
                    .unwrap_or_else(|| {
                        if clip.is_some() {
                            Rect::new(content.x, content.y, 0.0, 0.0)
                        } else {
                            content
                        }
                    }),
            )
        } else {
            clip
        };
        match &element.kind {
            ElementKind::FormattedText(spans) if self.text_layout => {
                let probe = spans.iter().enumerate().find_map(|(index, span)| {
                    span.action.as_deref().filter(|id| Some(*id) == self.focus_probe).map(|id| (index, id))
                });
                if let Some((index, id)) = probe {
                    let layout = self.laid_out_formatted(spans, style, content.width);
                    if let Some(bounds) = layout.span_bounds(index).first() {
                        let bounds = Rect::new(content.x + bounds.x, content.y + bounds.y, bounds.width, bounds.height);
                        self.record_inline_focus_target(id, bounds);
                    }
                }
            }
            ElementKind::FormattedText(spans) => {
                let mut y = content.y;
                for line in self.formatted_lines(spans, content.width, style).iter() {
                    let mut x = content.x;
                    for fragment in &line.fragments {
                        if let Some(id) = fragment
                            .action
                            .as_deref()
                            .filter(|id| Some(*id) == self.focus_probe)
                        {
                            self.record_inline_focus_target(
                                id,
                                Rect::new(x, y, fragment.width, line.height),
                            );
                            return;
                        }
                        x += fragment.width;
                    }
                    y += line.height;
                }
            }
            ElementKind::Column(children)
            | ElementKind::Row(children)
            | ElementKind::TabStrip { children, .. } => {
                for (child, child_rect) in children
                    .iter()
                    .zip(self.child_rects(element, children, content))
                {
                    if self
                        .focus_path
                        .contains(&(child as *const Element as usize))
                    {
                        self.probe_scroll_focus(child, child_rect, clip);
                    }
                }
            }
            ElementKind::Panel(child)
            | ElementKind::Clickable { child, .. }
            | ElementKind::Tooltip { anchor: child, .. } => {
                self.probe_scroll_focus(child, content, clip);
            }
            ElementKind::Stack(children) => {
                for child in children {
                    if self.focus_path.contains(&(child as *const Element as usize)) {
                        self.probe_scroll_focus(child, content, clip);
                    }
                }
            }
            ElementKind::Scroll {
                id,
                child,
                appearance,
                motion,
            } => {
                let full_width_height = self
                    .measure(child, Size::new(content.width, f32::INFINITY))
                    .height;
                let child_width =
                    appearance.content_width(content.width, full_width_height > content.height);
                let child_height = if child_width < content.width {
                    self.measure(child, Size::new(child_width, f32::INFINITY))
                        .height
                } else {
                    full_width_height
                };
                let max_offset = (child_height - content.height).max(0.0);
                self.frame.scroll_extents.insert(id.clone(), max_offset);
                let snap = self.scroll_to_end.remove(id).then_some(max_offset);
                let current_offset = self.scroll_position(id, max_offset, *motion, snap);
                let child_rect = Rect::new(
                    content.x,
                    content.y - current_offset,
                    child_width,
                    child_height.max(content.height),
                );
                let new_clip = clip
                    .and_then(|outer| outer.intersection(content))
                    .or_else(|| clip.is_none().then_some(content));
                self.scroll_focus_context.push(ScrollFocusContext {
                    id: id.clone(),
                    viewport: new_clip.unwrap_or(content),
                    offset: current_offset,
                });
                self.probe_scroll_focus(child, child_rect, new_clip.or(clip));
                self.scroll_focus_context.pop();
            }
            ElementKind::Overlay { base, .. }
                if self
                    .focus_path
                    .contains(&(base.as_ref() as *const Element as usize)) =>
            {
                self.probe_scroll_focus(base, content, clip);
            }
            ElementKind::Popover { anchor, .. }
                if self
                    .focus_path
                    .contains(&(anchor.as_ref() as *const Element as usize)) =>
            {
                self.probe_scroll_focus(anchor, content, clip);
            }
            _ => {}
        }
    }

    fn place_flex(
        &mut self,
        children: &[Element],
        content: Rect,
        gap: f32,
        row: bool,
        clip: Option<Rect>,
    ) -> Vec<Rect> {
        let rects = self.flex_rects(children, content, gap, row);
        for (child, rect) in children.iter().zip(&rects) {
            self.place(child, *rect, clip);
        }
        rects
    }

    /// A column with `sticky_top` children inside a scroll area. The last
    /// one scrolled past the top is held back for [`Self::place_pinned`]
    /// instead of being placed where the column has it.
    fn place_sticky_column(
        &mut self,
        children: &[Element],
        content: Rect,
        gap: f32,
        clip: Option<Rect>,
        viewport: Rect,
    ) {
        let rects = self.flex_rects(children, content, gap, false);
        let top = |child: &Element| child.sticky_top.map(|sticky| viewport.y + f32::from(sticky.inset));
        let pinned = children
            .iter()
            .zip(&rects)
            .rposition(|(child, rect)| top(child).is_some_and(|top| rect.y < top));
        // A background that fades in follows how far its child has scrolled
        // past the line it pins at.
        let faded = |child: &Element, rect: &Rect| {
            let (from, to) = child.kept.as_deref()?.sticky_background?;
            let color = child.style.background?;
            let share = ((top(child)? - rect.y - from) / (to - from)).clamp(0.0, 1.0);
            (share < 1.0).then(|| Color { a: (f32::from(color.a) * share).round() as u8, ..color })
        };
        for (index, (child, rect)) in children.iter().zip(&rects).enumerate() {
            if Some(index) == pinned {
                continue;
            }
            match faded(child, rect) {
                Some(color) => {
                    let mut child = child.clone();
                    child.style.background = Some(color);
                    self.place(&child, *rect, clip);
                }
                None => self.place(child, *rect, clip),
            }
        }
        let Some(index) = pinned else { return };
        let (child, mut rect) = (&children[index], rects[index]);
        let (Some(sticky), Some(top)) = (child.sticky_top, top(child)) else { return };
        let background = faded(child, &rect).or(child.style.background);
        // The next sticky child pushes this one out; without one it leaves
        // with the end of its column.
        let limit = children[index + 1..]
            .iter()
            .zip(&rects[index + 1..])
            .find(|(child, _)| child.sticky_top.is_some())
            .map_or(content.y + content.height, |(_, next)| next.y - gap);
        // A capped child keeps its bottom edge on the scroll until only the
        // cap is left, so it shrinks instead of jumping.
        let natural = rect.height;
        let cap = match sticky.max_height.get() {
            u16::MAX => natural,
            cap => f32::from(cap).min(natural),
        };
        rect.height = (rect.y + natural - top).clamp(cap, natural);
        rect.y = top.min(limit - rect.height).max(rect.y);
        let mut element = child.clone();
        element.style.background = background;
        // An anchor stays where the column has it, so revealing it scrolls
        // back to the start of what it pins over.
        if let Some(anchor) = element.anchor.take() {
            self.record_scroll_focus_target(&Element::spacer().anchor(anchor), rects[index]);
        }
        if rect.height < natural {
            element.style.clip_children = true;
            element.style.bottom_fade = element
                .style
                .bottom_fade
                .or(element.style.background.map(|color| (color, STICKY_FADE)));
        }
        // The fade under it grows in as the child pins and shortens as the
        // next sticky child arrives, so it never veils that one.
        let fade = f32::from(sticky.fade);
        let below = (limit - rect.y - rect.height).min(fade);
        let fade = (below > 0.0).then(|| {
            let strength = ((top - rects[index].y) / fade).min(1.0);
            // The same color hides what would show over the child and
            // through its rounded corners, so nothing scrolls around it.
            let cover = Rect::new(rect.x, viewport.y, rect.width, rect.y + rect.height - viewport.y);
            let under = Rect::new(rect.x, rect.y + rect.height, rect.width, below);
            (cover, under, strength)
        });
        self.pinned.push((element, rect, clip, fade));
    }

    /// Place what a scroll area's columns pinned, after its content and its
    /// fades, so it paints and hit-tests above both.
    fn place_pinned(&mut self, scroll_id: &str, pinned: Vec<PinnedChild>, fade_color: Option<Color>) {
        for (element, rect, clip, fade) in pinned {
            // Its passive parts must not let a click reach the rows under it.
            self.add_region(scroll_id, rect, HitKind::Scroll, clip);
            // The copy lives for this call only: keep its address out of the
            // frame's measurements.
            let previous_scope = self.enter_measurement_scope();
            if let Some(clip) = clip {
                self.frame.commands.push(DrawCommand::PushClip(clip));
            }
            if let (Some((cover, under, strength)), Some(color)) = (fade, fade_color) {
                let top = Color { a: (f32::from(color.a) * strength).round() as u8, ..color };
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: cover,
                    color: top,
                    radius: 0.0,
                });
                self.frame.commands.push(DrawCommand::FillVerticalGradient {
                    rect: under,
                    top,
                    bottom: Color { a: 0, ..color },
                });
            }
            self.place(&element, rect, clip);
            if clip.is_some() {
                self.frame.commands.push(DrawCommand::PopClip);
            }
            self.measurement_scope = previous_scope;
        }
    }

    /// Re-measure a row's `shrink` children at the width the row can spare,
    /// so their text wraps or truncates instead of pushing siblings out.
    /// They give up width in order, each down to its minimum width first.
    fn fit_shrinking(&self, children: &[Element], sizes: &mut [Size], row: Size, gap: f32) {
        if !row.width.is_finite() || !children.iter().any(|child| child.style.shrink) {
            return;
        }
        let reserved = gap * children.len().saturating_sub(1) as f32
            + children
                .iter()
                .zip(sizes.iter())
                .filter(|(child, _)| !child.style.shrink)
                .map(|(child, size)| {
                    if child.style.flex > 0.0 {
                        child.style.min_width
                    } else {
                        size.width
                    }
                })
                .sum::<f32>();
        let spare = (row.width - reserved).max(0.0);
        let natural: f32 = children
            .iter()
            .zip(sizes.iter())
            .filter(|(child, _)| child.style.shrink)
            .map(|(_, size)| size.width)
            .sum();
        let mut deficit = natural - spare;
        let mut widths: Vec<f32> = sizes.iter().map(|size| size.width).collect();
        for respect_minimum in [true, false] {
            for (child, width) in children.iter().zip(&mut widths) {
                if deficit <= 0.0 || !child.style.shrink {
                    continue;
                }
                let floor = if respect_minimum { child.style.min_width } else { 0.0 };
                let given = (*width - floor).clamp(0.0, deficit);
                *width -= given;
                deficit -= given;
            }
        }
        for ((child, size), width) in children.iter().zip(sizes.iter_mut()).zip(widths) {
            if child.style.shrink && width < size.width {
                *size = self.measure(child, Size::new(width, row.height));
                size.width = size.width.min(width);
            }
        }
    }

    /// Fit opted-in, non-flex column children without changing their widths.
    /// A scroll area inside a row then receives its actual shorter viewport.
    fn fit_column_heights(children: &[Element], sizes: &mut [Size], height: f32, gap: f32) -> bool {
        let shrinks = |child: &Element| child.style.shrink_height && child.style.flex <= 0.0;
        if !height.is_finite() || !children.iter().any(shrinks) {
            return false;
        }
        // Viewport-like children without an explicit height measure as
        // unbounded. Start them at the finite column budget so shrinking
        // never subtracts infinity from itself or discards all their space.
        for (child, size) in children.iter().zip(sizes.iter_mut()) {
            if shrinks(child) && size.height == f32::INFINITY {
                size.height = height.max(0.0);
            }
        }
        let reserved = f64::from(gap) * children.len().saturating_sub(1) as f64
            + children
                .iter()
                .zip(sizes.iter())
                .filter(|(child, _)| !shrinks(child))
                .map(|(child, size)| {
                    f64::from(if child.style.flex > 0.0 {
                        child.style.min_height
                    } else {
                        size.height
                    })
                })
                .sum::<f64>();
        let natural: f64 = children
            .iter()
            .zip(sizes.iter())
            .filter(|(child, _)| shrinks(child))
            .map(|(_, size)| f64::from(size.height))
            .sum();
        let mut deficit = natural - (f64::from(height) - reserved).max(0.0);
        for respect_minimum in [true, false] {
            for (child, size) in children.iter().zip(sizes.iter_mut()) {
                if deficit <= 0.0 || !shrinks(child) {
                    continue;
                }
                let floor = if respect_minimum {
                    f64::from(child.style.min_height)
                } else {
                    0.0
                };
                let given = (f64::from(size.height) - floor).clamp(0.0, deficit);
                size.height = (f64::from(size.height) - given).max(0.0) as f32;
                deficit -= given;
            }
        }
        true
    }

    /// Keep weighted shares until a minimum binds, then divide the rest among
    /// the remaining children. Only a column opting in to height shrinking uses
    /// this allocator; existing rows and columns retain their distribution.
    fn fit_column_flex(children: &[Element], sizes: &mut [Size], available: f32) {
        let mut weights: Vec<f64> = children
            .iter()
            .map(|child| f64::from(child.style.flex.max(0.0)))
            .collect();
        let minimum: f64 = children
            .iter()
            .filter(|child| child.style.flex > 0.0)
            .map(|child| f64::from(child.style.min_height))
            .sum();
        if minimum > f64::from(available) {
            // Fixed siblings, gaps and minima cannot all fit. Preserve those
            // constraints; an opted-in child cannot repair an impossible base.
            for (child, size) in children.iter().zip(sizes) {
                if child.style.flex > 0.0 {
                    size.height = child.style.min_height;
                }
            }
            return;
        }
        let mut remaining = f64::from(available);
        loop {
            let total: f64 = weights.iter().sum();
            if total <= 0.0 {
                break;
            }
            let round_remaining = remaining;
            let mut constrained = false;
            for ((child, size), weight) in children.iter().zip(sizes.iter_mut()).zip(&mut weights) {
                if *weight > 0.0
                    && round_remaining * *weight / total < f64::from(child.style.min_height)
                {
                    size.height = child.style.min_height;
                    remaining -= f64::from(size.height);
                    *weight = 0.0;
                    constrained = true;
                }
            }
            if !constrained {
                for (size, weight) in sizes.iter_mut().zip(&weights) {
                    if *weight > 0.0 {
                        size.height = (remaining * weight / total) as f32;
                    }
                }
                break;
            }
        }
    }

    /// Where a row, a column or a tab strip puts its children in `content`.
    fn child_rects(&mut self, element: &Element, children: &[Element], content: Rect) -> Vec<Rect> {
        match element.wrapping_line_gap() {
            Some(line_gap) => {
                // Wrap at the width it was measured at, which `max_width` caps.
                let max = element
                    .style
                    .max_width
                    .map_or(f32::INFINITY, |max| (max - element.style.padding * 2.0).max(0.0));
                let lines = Rect::new(content.x, content.y, content.width.min(max), content.height);
                self.wrap_rects(children, lines, element.style.gap, line_gap)
            }
            None if element.centers_middle(children.len()) => {
                self.center_middle_rects(children, content, element.style.gap)
            }
            None => {
                let row = !matches!(element.kind, ElementKind::Column(_));
                self.flex_rects(children, content, element.style.gap, row)
            }
        }
    }

    /// The three children of a [`Element::center_middle`] row: the ends at
    /// their natural widths against the row's edges, the middle centered in
    /// the row unless it would meet one of them, then `gap` after the first
    /// and no wider than the room it has there.
    fn center_middle_rects(&self, children: &[Element], content: Rect, gap: f32) -> Vec<Rect> {
        let width = |child: &Element| self.measure(child, Size::new(f32::INFINITY, content.height)).width;
        let (start, mut middle, end) = (width(&children[0]), width(&children[1]), width(&children[2]));
        let after_start = content.x + start + gap;
        let before_end = content.x + content.width - end - gap;
        let centered = content.x + (content.width - middle) / 2.0;
        let fits = centered >= after_start && centered + middle <= before_end;
        if !fits {
            middle = middle.min((before_end - after_start).max(0.0));
        }
        let x = [content.x, if fits { centered } else { after_start }, content.x + content.width - end];
        children
            .iter()
            .zip(x.into_iter().zip([start, middle, end]))
            .map(|(child, (x, width))| {
                let height = child.style.height.unwrap_or(content.height).min(content.height);
                Rect::new(x, content.y, width, height.max(0.0))
            })
            .collect()
    }

    /// Where a wrapping row puts its children in `content`: at their natural
    /// widths, as many on a line as fit, each line as tall as its tallest
    /// child and `line_gap` under the one before. A child wider than the row
    /// has a line to itself and is measured at the row's width. Measuring
    /// the row uses this too, so its height is the one it is placed at.
    fn wrap_rects(
        &self,
        children: &[Element],
        content: Rect,
        gap: f32,
        line_gap: f32,
    ) -> Vec<Rect> {
        let mut rects: Vec<Rect> = Vec::with_capacity(children.len());
        let (mut line, mut x, mut y, mut height) = (0, 0.0_f32, 0.0_f32, 0.0_f32);
        // A row is as tall as its line; so is each child, unless sized.
        let stretch = |rects: &mut [Rect], children: &[Element], height: f32| {
            for (rect, child) in rects.iter_mut().zip(children) {
                rect.height = child.style.height.unwrap_or(height).min(height);
            }
        };
        for (index, child) in children.iter().enumerate() {
            let mut size = self.measure(child, Size::new(f32::INFINITY, f32::INFINITY));
            if size.width > content.width {
                size = self.measure(child, Size::new(content.width, f32::INFINITY));
                size.width = size.width.min(content.width);
            }
            // The tolerance keeps a row placed at the width it measured on
            // one line, whatever its padding did to the last bit.
            if index > line && x + size.width > content.width + 0.01 {
                stretch(&mut rects[line..], &children[line..], height);
                (line, x, y, height) = (index, 0.0, y + height + line_gap, 0.0);
            }
            rects.push(Rect::new(
                content.x + x,
                content.y + y,
                size.width,
                size.height,
            ));
            x += size.width + gap;
            height = height.max(size.height);
        }
        stretch(&mut rects[line..], &children[line..], height);
        rects
    }

    fn flex_rects(
        &mut self,
        children: &[Element],
        content: Rect,
        gap: f32,
        row: bool,
    ) -> Vec<Rect> {
        let count = children.len();
        if count == 0 {
            return Vec::new();
        }
        let available_main = if row { content.width } else { content.height };
        let available_cross = if row { content.height } else { content.width };
        let mut fixed = gap * count.saturating_sub(1) as f32;
        let mut weights = 0.0;
        let mut sizes = Vec::with_capacity(count);
        for child in children {
            let measured = if row {
                self.measure(child, Size::new(f32::INFINITY, available_cross))
            } else {
                self.measure(child, Size::new(available_cross, f32::INFINITY))
            };
            if child.style.flex > 0.0 {
                weights += child.style.flex;
            } else {
                fixed += if row { measured.width } else { measured.height };
            }
            sizes.push(measured);
        }
        let fitted_height =
            !row && Self::fit_column_heights(children, &mut sizes, available_main, gap);
        if row {
            self.fit_row_shortcuts(
                children,
                &mut sizes,
                Size::new(available_main, available_cross),
                gap,
            );
            self.fit_shrinking(
                children,
                &mut sizes,
                Size::new(available_main, available_cross),
                gap,
            );
            fixed = gap * count.saturating_sub(1) as f32
                + children
                    .iter()
                    .zip(&sizes)
                    .filter(|(child, _)| child.style.flex <= 0.0)
                    .map(|(_, size)| size.width)
                    .sum::<f32>();
        } else if fitted_height {
            fixed = gap * count.saturating_sub(1) as f32
                + children
                    .iter()
                    .zip(&sizes)
                    .filter(|(child, _)| child.style.flex <= 0.0)
                    .map(|(_, size)| size.height)
                    .sum::<f32>();
        }
        let flexible = (available_main - fixed).max(0.0);
        if fitted_height {
            Self::fit_column_flex(children, &mut sizes, flexible);
        }
        let mut offset = 0.0;
        let mut rects = Vec::with_capacity(count);
        for (child, measured) in children.iter().zip(sizes) {
            let main = if child.style.flex > 0.0 && weights > 0.0 {
                if fitted_height {
                    measured.height
                } else {
                    flexible * child.style.flex / weights
                }
            } else if row {
                measured.width
            } else {
                measured.height
            };
            let cross = if row {
                child
                    .style
                    .height
                    .unwrap_or(available_cross)
                    .min(available_cross)
            } else {
                child
                    .style
                    .width
                    .unwrap_or(available_cross)
                    .min(available_cross)
            };
            let rect = if row {
                Rect::new(content.x + offset, content.y, main.max(0.0), cross.max(0.0))
            } else {
                Rect::new(content.x, content.y + offset, cross.max(0.0), main.max(0.0))
            };
            rects.push(rect);
            offset += main + gap;
        }
        rects
    }

    /// Whether the hint of `anchor_id` shows now: `Some(true)` while it has
    /// the keyboard focus, `Some(false)` once the pointer has rested on it
    /// for `delay` without pressing it.
    fn hint_due(&self, anchor_id: &str, delay: Duration) -> Option<bool> {
        if self.keyboard_tooltip_focus {
            return (self.focused == Some(anchor_id)).then_some(true);
        }
        (self.hovered == Some(anchor_id)
            && self.pressed != Some(anchor_id)
            && self.tooltip_hover_since.is_some_and(|(id, since)| {
                id == anchor_id
                    && self.now.saturating_duration_since(*since) >= delay
            }))
        .then_some(false)
    }

    fn add_region(&mut self, id: &str, rect: Rect, kind: HitKind, clip: Option<Rect>) {
        let visible = match clip {
            Some(clip) => rect.intersection(clip),
            None => Some(rect),
        };
        if let Some(rect) = visible {
            self.frame.regions.push(HitRegion {
                id: id.to_owned(),
                rect,
                kind,
            });
        }
    }
}

#[cfg(test)]
fn wrap_lines(value: &str, width: f32, char_width: f32) -> Vec<String> {
    wrap_measured(value, width, |part| {
        part.chars().count() as f32 * char_width
    })
    .into_iter()
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu_with_secondary_actions(mark_secondary: bool, visible_rows: usize) -> Element {
        use crate::{
            Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
            MenuItem, Tooltip,
        };

        let palette = ComponentPalette::neutral_dark();
        let mut entries = vec![DropdownMenuItem::new("normal", "New terminal").into()];
        for index in 1..=2 {
            let close_id = format!("close:{index}");
            let mut close = Element::icon_button(&close_id, Icon::Close, "")
                .size(Some(24.0), Some(24.0))
                .dispatch_as(format!("terminal_close:{index}"));
            if mark_secondary {
                close = close.menu_secondary_action();
            }
            entries.push(DropdownMenuEntry::custom(
                Element::row(vec![
                    MenuItem::new(
                        format!("main:{index}"),
                        format!("Terminal {index}"),
                        palette,
                    )
                    .width(180.0)
                    .height(28.0)
                    .render(),
                    Tooltip::new(close, close_id, "Close terminal", palette).render(),
                ])
                .gap(4.0)
                .size(None, Some(28.0)),
            ));
        }
        DropdownMenu::new(
            Element::button("trigger", "Terminals").size(Some(100.0), Some(28.0)),
            entries,
            true,
            "dismiss",
            palette,
        )
        .width(240.0)
        .max_visible_items(visible_rows)
        .render()
    }

    #[test]
    fn popover_arrows_skip_secondary_actions_but_tab_and_activation_keep_them() {
        let viewport = Size::new(320.0, 240.0);
        let mut tree = UiTree::new(menu_with_secondary_actions(true, 8));
        tree.set_reduced_motion(true);
        tree.layout(viewport);
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        for (from, direction, expected) in [
            ("main:1", Key::ArrowDown, "main:2"),
            ("main:2", Key::ArrowUp, "main:1"),
            ("close:1", Key::ArrowDown, "main:2"),
            ("close:2", Key::ArrowUp, "main:2"),
            ("close:2", Key::ArrowDown, "normal"),
            ("normal", Key::ArrowUp, "main:2"),
            ("close:2", Key::Home, "normal"),
            ("normal", Key::End, "main:2"),
            ("trigger", Key::ArrowUp, "main:2"),
            ("trigger", Key::ArrowDown, "normal"),
        ] {
            assert!(tree.focus(from));
            assert!(tree.handle_event(key(direction)).is_empty());
            assert_eq!(tree.focused_id(), Some(expected), "{from}: {direction:?}");
        }
        assert!(tree.focus("main:1"));
        assert!(tree.handle_event(key(Key::Tab)).is_empty());
        assert_eq!(tree.focused_id(), Some("close:1"));
        for activate in [Key::Enter, Key::Space] {
            assert_eq!(
                tree.handle_event(key(activate)),
                vec![UiAction::Clicked {
                    id: "terminal_close:1".into(),
                }]
            );
        }
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: true,
                ctrl: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("main:1"));

        let frame = tree.layout(viewport);
        let close = frame
            .regions
            .iter()
            .find(|region| region.id == "close:2")
            .unwrap()
            .rect;
        let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
        assert_eq!(frame.hit_test(point), Some("close:2"));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "terminal_close:2".into(),
            }]
        );
        assert_eq!(
            tree.handle_event(key(Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss".into(),
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn popover_secondary_actions_preserve_offscreen_primary_focus_and_reveal() {
        let viewport = Size::new(320.0, 240.0);
        let mut tree = UiTree::new(menu_with_secondary_actions(true, 1));
        tree.set_reduced_motion(true);
        let before = tree.layout(viewport);
        assert!(!before.regions.iter().any(|region| region.id == "main:2"));
        assert!(!before.scroll_focus_targets.contains_key("main:2"));
        assert!(before.scroll_focus_candidates.contains("main:2"));
        assert!(tree.focus("trigger"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::End,
                shift: false,
                ctrl: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("main:2"));
        let frame = tree.layout(viewport);
        let options = frame
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        let main = frame
            .regions
            .iter()
            .find(|region| region.id == "main:2")
            .unwrap()
            .rect;
        assert_eq!(main.height, 28.0);
        assert!(main.y >= options.y && main.y + main.height <= options.y + options.height);
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("close:2"));
    }

    #[test]
    fn unmarked_popover_buttons_keep_the_existing_arrow_order() {
        let mut tree = UiTree::new(menu_with_secondary_actions(false, 8));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(320.0, 240.0));
        assert!(tree.focus("main:1"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowDown,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("close:1"));
        tree.handle_event(UiEvent::Key {
            key: Key::End,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("close:2"));
    }

    #[test]
    fn max_width_paragraph_measures_its_wrapped_height_before_the_next_sibling() {
        let description = "A paragraph whose natural width is longer than its maximum must keep every wrapped line above the following control.";
        for width in [160.0, 480.0] {
            let paragraph = Element::text(description)
                .max_width(120.0)
                .shrink_to_fit()
                .centered_x();
            let mut tree = UiTree::new(
                Element::column(vec![
                    paragraph,
                    Element::button("next", "Continue").size(None, Some(32.0)),
                ])
                .gap(12.0),
            );
            let frame = tree.layout(Size::new(width, 600.0));
            let blocks = frame.text_layouts();
            let paragraph = blocks
                .iter()
                .find(|block| block.source == description)
                .unwrap();
            assert!(
                paragraph.runs.len() > 1,
                "the capped paragraph must wrap at {width}"
            );
            let bottom = paragraph
                .runs
                .iter()
                .map(|run| run.rect.y + run.rect.height)
                .fold(0.0_f32, f32::max);
            let next = frame
                .regions
                .iter()
                .find(|region| region.id == "next")
                .unwrap();
            assert!(next.rect.y >= bottom + 12.0 - 0.1,
                "wrapped paragraph consumed its following gap at {width}: bottom={bottom}, next={:?}", next.rect);
        }
    }

    #[test]
    fn intrinsic_width_cap_keeps_breadcrumb_stats_adjacent_and_actions_in_bounds() {
        use crate::{Breadcrumb, BreadcrumbItem, Component, ComponentPalette};

        let measured_width = |text: &str| {
            text.chars()
                .map(|character| match character {
                    'W' => 12.0,
                    'i' => 3.0,
                    _ => 6.0,
                })
                .sum::<f32>()
        };
        for cache_disabled in [false, true] {
            for label in [
                "README.md",
                "Wider_resource_with_a_long_filename_for_this_header.md",
            ] {
                let budget = 140.0;
                let path = Breadcrumb::new(
                    "path",
                    vec![BreadcrumbItem::new(label).action("open_path")],
                    ComponentPalette::neutral_dark(),
                )
                .width(budget)
                .height(24.0)
                .render()
                .size(None, Some(24.0))
                .max_width(budget);
                let mut tree = UiTree::new(
                    Element::row(vec![
                        path,
                        Element::text("Local +62 −5")
                            .font_family(FontFamily::Sans)
                            .size(Some(80.0), Some(24.0)),
                        Element::spacer().flex(1.0),
                        Element::icon_button("refresh", Icon::Refresh, "")
                            .size(Some(24.0), Some(24.0)),
                    ])
                    .gap(8.0),
                );
                tree.measurement_cache.get_mut().disabled = cache_disabled;
                tree.set_reduced_motion(true);
                tree.set_text_measurer(move |text, _, _| measured_width(text));
                let size = Size::new(320.0, 24.0);
                let frame = tree.layout(size);
                let path_rect = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "open_path" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                let expected_width = (measured_width(label) + 4.0).min(budget);
                assert_eq!(path_rect, Rect::new(0.0, 0.0, expected_width, 24.0));
                let stats_x = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text { text, position, .. } if text == "Local +62 −5" => {
                            Some(position.x)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(stats_x, expected_width + 8.0);
                let refresh_rect = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "refresh" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                assert_eq!(refresh_rect, Rect::new(296.0, 0.0, 24.0, 24.0));
                assert!(stats_x + 80.0 < refresh_rect.x);
                assert_ne!(
                    frame.hit_test(Point::new(expected_width + 2.0, 12.0)),
                    Some("open_path"),
                    "the resource action must not extend into its trailing gap"
                );
                for (id, rect) in [("open_path", path_rect), ("refresh", refresh_rect)] {
                    let point = Point::new(rect.x + rect.width / 2.0, 12.0);
                    tree.handle_event(UiEvent::PointerDown(point));
                    assert_eq!(
                        tree.handle_event(UiEvent::PointerUp(point)),
                        vec![UiAction::Clicked { id: id.into() }],
                        "clipped filename and toolbar keep independent actions"
                    );
                }
            }
        }
    }

    #[test]
    fn intrinsic_width_cap_preserves_explicit_minimums_and_anchor_wrappers() {
        use crate::{Component, ComponentPalette, Tooltip};

        for cache_disabled in [false, true] {
            for minimum in [0.0_f32, 96.0] {
                for wrapper in 0..3 {
                    let anchor = Element::button("bounded", "Bounded")
                        .size(Some(160.0), Some(24.0))
                        .min_size(minimum, 0.0)
                        .max_width(64.0);
                    let wrapped = match wrapper {
                        0 => anchor,
                        1 => Tooltip::new(
                            anchor,
                            "bounded",
                            "Bounded action",
                            ComponentPalette::neutral_dark(),
                        )
                        .render(),
                        _ => Element::popover(
                            anchor,
                            Element::spacer(),
                            "dismiss",
                            PopoverPlacement::BottomStart,
                        ),
                    };
                    let mut tree = UiTree::new(Element::row(vec![
                        wrapped,
                        Element::button("after", "After").size(Some(48.0), Some(24.0)),
                    ]));
                    tree.measurement_cache.get_mut().disabled = cache_disabled;
                    tree.set_reduced_motion(true);
                    let frame = tree.layout(Size::new(300.0, 80.0));
                    let bounded = frame
                        .regions
                        .iter()
                        .find(|region| region.id == "bounded" && region.kind == HitKind::Button)
                        .unwrap()
                        .rect;
                    let after = frame
                        .regions
                        .iter()
                        .find(|region| region.id == "after" && region.kind == HitKind::Button)
                        .unwrap()
                        .rect;
                    assert_eq!(bounded.width, minimum.max(64.0));
                    assert_eq!(after.x, bounded.x + bounded.width);
                }
            }
        }
    }

    #[test]
    fn intrinsic_height_cap_keeps_short_content_compact_and_long_content_scrollable() {
        for cache_disabled in [false, true] {
            for content_height in [20.0, 200.0] {
                let mut tree = UiTree::new(Element::column(vec![
                    Element::scroll("capped", Element::spacer().size(None, Some(content_height)))
                        .max_height(48.0),
                    Element::button("after", "After").size(Some(100.0), Some(24.0)),
                ]));
                tree.measurement_cache.get_mut().disabled = cache_disabled;
                let frame = tree.layout(Size::new(220.0, 300.0));
                let after = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "after")
                    .unwrap();
                assert_eq!(after.rect.y, content_height.min(48.0));
                assert_eq!(
                    frame.scroll_extents.get("capped").copied().unwrap_or(0.0),
                    (content_height - 48.0).max(0.0)
                );
            }
            let mut tree = UiTree::new(Element::column(vec![
                Element::button("fixed", "Fixed")
                    .size(Some(100.0), Some(80.0))
                    .max_height(32.0),
                Element::button("after", "After").size(Some(100.0), Some(24.0)),
            ]));
            tree.measurement_cache.get_mut().disabled = cache_disabled;
            let frame = tree.layout(Size::new(220.0, 300.0));
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == "fixed")
                    .unwrap()
                    .rect
                    .height,
                32.0
            );
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == "after")
                    .unwrap()
                    .rect
                    .y,
                32.0
            );
        }
    }

    #[test]
    fn narrow_rows_omit_whole_shortcuts_before_consuming_the_labels_minimum_width() {
        for width in [352.0, 120.0] {
            let mut tree = UiTree::new(Element::column(vec![
                Element::clickable(
                    "action",
                    Element::row(vec![
                        Element::text("Todas las coincidencias")
                            .min_size(40.0, 22.0)
                            .flex(1.0),
                        Element::keyboard_shortcut("Ctrl+Shift+L", Color::rgb(0, 0, 0), None),
                    ])
                    .gap(8.0),
                )
                .min_size(0.0, 26.0),
                Element::button("after", "After").size(None, Some(24.0)),
            ]));
            let frame = tree.layout(Size::new(width, 300.0));
            let action = frame
                .regions
                .iter()
                .find(|region| region.id == "action")
                .unwrap();
            let after = frame
                .regions
                .iter()
                .find(|region| region.id == "after")
                .unwrap();
            assert_eq!(after.rect.y, action.rect.y + action.rect.height);
            let has_ctrl = frame.commands.iter().any(|command| {
                matches!(command,
                DrawCommand::Text { text, .. } if text == "Ctrl")
            });
            assert_eq!(
                has_ctrl,
                width > 300.0,
                "the complete chord must yield in narrow rows"
            );
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text.contains("Todas"))));
        }
    }

    #[test]
    fn intrinsic_faded_text_uses_its_ancestor_clip_as_the_visible_edge() {
        let text = "a_long_filename_with_an_intrinsic_width.rs";
        let mut tree = UiTree::new(Element::column(vec![Element::row(vec![
            Element::faded_text(text, 20.0).size(Some(400.0), Some(24.0)),
        ])
        .size(Some(100.0), Some(24.0))
        .clip_children()]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(Size::new(300.0, 100.0));
        let painted: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text: value,
                    color,
                    position,
                    ..
                } if value == text => Some((*position, color.a)),
                _ => None,
            })
            .collect();
        assert_eq!(painted.len(), 17, "solid text plus sixteen fade bands");
        assert!(painted.windows(2).all(|pair| pair[0].0 == pair[1].0));
        assert_eq!(painted[0].1, 255);
        assert!(painted.last().unwrap().1 < 16);
        assert!(!tree.is_animating());
    }

    #[test]
    fn independent_corners_preserve_hover_press_focus_and_action_geometry() {
        let hover = Color::rgb(66, 66, 66);
        let pressed = Color::rgb(76, 76, 76);
        for width in [101.0, 360.0, 900.0] {
            let mut tree = UiTree::new(
                Element::clickable("rounded-row", Element::text("Provider"))
                    .size(Some(width), Some(51.0))
                    .hover_background(hover)
                    .pressed_background(pressed)
                    .corner_radii(CornerRadii::top(12.0)),
            );
            tree.set_reduced_motion(true);
            let initial = tree.layout(Size::new(width, 100.0));
            let rect = initial
                .regions
                .iter()
                .find(|region| region.id == "rounded-row")
                .unwrap()
                .rect;
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerMove(point));
            let hovered = tree.layout(Size::new(width, 100.0));
            let radii = |frame: &Frame, color| {
                frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::FillCornerRect {
                            rect: painted,
                            color: ink,
                            radii,
                        } if *painted == rect && *ink == color => Some(*radii),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(radii(&hovered, hover), [CornerRadii::top(12.0)]);
            assert_eq!(hovered.hit_test(point), Some("rounded-row"));
            tree.handle_event(UiEvent::PointerDown(point));
            let down = tree.layout(Size::new(width, 100.0));
            assert_eq!(radii(&down, pressed), [CornerRadii::top(12.0)]);
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                [UiAction::Clicked {
                    id: "rounded-row".into()
                }]
            );
            assert!(tree.focus("rounded-row"));
            let focused = tree.layout(Size::new(width, 100.0));
            assert_eq!(
                focused
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::StrokeCornerRect {
                            rect: painted,
                            width: stroke,
                            radii,
                            ..
                        } if *painted == rect && *stroke == 2.0 => Some(*radii),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
                [CornerRadii::top(12.0)]
            );
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                [UiAction::Clicked {
                    id: "rounded-row".into()
                }]
            );
            let mut clips = 0_i32;
            for command in focused.commands {
                match command {
                    DrawCommand::PushClip(_) => clips += 1,
                    DrawCommand::PopClip => clips -= 1,
                    _ => {}
                }
                assert!(clips >= 0);
            }
            assert_eq!(clips, 0);
        }
    }
    use crate::{Component, ComponentPalette, DropdownMenu, DropdownMenuItem, Tab, Tabs, Tooltip};

    #[test]
    fn row_measure_wraps_flex_column_at_assigned_width() {
        let title_ink = Color::rgb(191, 210, 221);
        let description_ink = Color::rgb(112, 140, 165);
        let note_surface = Color::rgb(32, 39, 43);
        for width in [180.0, 360.0, 900.0] {
            let note = Element::row(vec![
                Element::icon(Icon::Settings).size(Some(16.0), Some(16.0)),
                Element::column(vec![
                    Element::text("Authentication belongs to the selected provider connection")
                        .font_family(FontFamily::Sans)
                        .foreground(title_ink),
                    Element::text("Connect the provider with its own credential, then refresh the catalog to inspect the models declared by that account. A listed model does not prove that generation has been accepted.")
                        .font_family(FontFamily::Sans)
                        .foreground(description_ink),
                ])
                .gap(5.0)
                .flex(1.0),
            ])
            .gap(10.0)
            .padding(16.0)
            .background(note_surface);
            let mut tree = UiTree::new(
                Element::column(vec![
                    note,
                    Element::button("following-row", "Following row").size(None, Some(28.0)),
                ])
                .gap(8.0),
            );
            let frame = tree.layout(Size::new(width, 800.0));
            let note_rect = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == note_surface => {
                        Some(*rect)
                    }
                    _ => None,
                })
                .unwrap();
            let last_copy_bottom = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        position, color, ..
                    } if *color == description_ink => {
                        Some(position.y + FontMetrics::default().line_height)
                    }
                    _ => None,
                })
                .fold(0.0_f32, f32::max);
            let following = frame
                .regions
                .iter()
                .find(|region| region.id == "following-row")
                .unwrap();
            assert!(
                last_copy_bottom <= note_rect.y + note_rect.height - 16.0 + 0.1,
                "wrapped copy escapes its measured note at {width}: bottom={last_copy_bottom}, note={note_rect:?}"
            );
            assert!(
                following.rect.y >= last_copy_bottom + 16.0 + 8.0 - 0.1,
                "next row overlaps wrapped note at {width}"
            );
        }
    }

    #[test]
    fn row_measure_keeps_inline_switch_and_next_setting_below_wrapped_copy() {
        use crate::Switch;

        let description_ink = Color::rgb(121, 135, 147);
        let row_surface = Color::rgb(28, 32, 35);
        for width in [180.0, 360.0, 900.0] {
            // This is the shared Settings row shape: flexible label and
            // description column, fixed-width inline switch, padding and gap.
            let setting = Element::row(vec![
                Element::column(vec![
                    Element::text("Require approval before running a workspace command")
                        .font_family(FontFamily::Sans)
                        .font_size(14.0),
                    Element::text("Every command requires a persisted permission decision before it is sent to the native execution service. This preference applies to the current agent profile.")
                        .font_family(FontFamily::Sans)
                        .font_size(12.0)
                        .foreground(description_ink),
                ])
                .gap(6.0)
                .flex(1.0),
                Switch::new("approval-switch", false, ComponentPalette::neutral_dark())
                    .render(),
            ])
            .gap(24.0)
            .min_size(0.0, 54.0)
            .padding(16.0)
            .background(row_surface);
            let mut tree = UiTree::new(
                Element::column(vec![
                    setting,
                    Element::button("next-setting", "Next setting").size(None, Some(28.0)),
                ])
                .gap(1.0),
            );
            let frame = tree.layout(Size::new(width, 800.0));
            let row_rect = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == row_surface => {
                        Some(*rect)
                    }
                    _ => None,
                })
                .unwrap();
            let switch = frame
                .regions
                .iter()
                .find(|region| region.id == "approval-switch")
                .unwrap();
            let following = frame
                .regions
                .iter()
                .find(|region| region.id == "next-setting")
                .unwrap();
            let last_copy_bottom = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        position,
                        color,
                        font_size,
                        ..
                    } if *color == description_ink => {
                        Some(position.y + FontMetrics::default().at_size(*font_size).line_height)
                    }
                    _ => None,
                })
                .fold(0.0_f32, f32::max);
            assert_eq!(switch.rect.width, 36.0);
            assert!(switch.rect.x + switch.rect.width <= row_rect.x + row_rect.width - 16.0 + 0.1);
            assert!(
                following.rect.y >= last_copy_bottom + 16.0 + 1.0 - 0.1,
                "next setting overlaps wrapped copy at {width}"
            );
            let switch_point = Point::new(switch.rect.x + 18.0, switch.rect.y + 10.0);
            tree.handle_event(UiEvent::PointerDown(switch_point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(switch_point)),
                vec![UiAction::Clicked {
                    id: "approval-switch".into()
                }]
            );
        }
    }

    #[test]
    fn effort_slider_previews_pointer_capture_and_snaps_to_dynamic_stops() {
        let mut tree =
            UiTree::new(Element::effort_slider("effort", 0.0, 5).size(Some(220.0), Some(42.0)));
        let frame = tree.layout(Size::new(220.0, 42.0));
        assert!(frame.regions.iter().any(|region| region.id == "effort"));
        let down = tree.handle_event(UiEvent::PointerDown(Point::new(12.0, 21.0)));
        assert_eq!(
            down,
            vec![UiAction::SliderChanged {
                id: "effort".into(),
                fraction: 0.0,
                committed: false,
            }]
        );
        let preview = tree.handle_event(UiEvent::PointerMove(Point::new(170.0, 21.0)));
        assert!(matches!(
            preview.as_slice(),
            [UiAction::SliderChanged {
                id,
                fraction,
                committed: false,
            }] if id == "effort" && *fraction > 0.75 && *fraction < 0.85
        ));
        let release = tree.handle_event(UiEvent::PointerUp(Point::new(170.0, 21.0)));
        assert_eq!(
            release,
            vec![UiAction::SliderChanged {
                id: "effort".into(),
                fraction: 0.75,
                committed: true,
            }]
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(170.0, 21.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(500.0, 21.0))),
            vec![UiAction::SliderChanged {
                id: "effort".into(),
                fraction: 1.0,
                committed: true,
            }]
        );
        assert!(tree.is_animating());
    }

    #[test]
    fn effort_slider_magnetism_uses_the_advertised_stop_count() {
        let rect = Rect::new(0.0, 0.0, 220.0, 38.0);
        let five = SliderGeometry {
            rect,
            steps: 5,
            fraction: 0.0,
            ultra_available: true,
            inset: SLIDER_THUMB_INSET,
            animated: true,
            y: None,
            vertical: false,
        };
        let three = SliderGeometry { steps: 3, ..five };
        let x = SLIDER_THUMB_INSET + 0.3 * (rect.width - 2.0 * SLIDER_THUMB_INSET);
        let x = Point::new(x, 0.0);
        assert!(five.fraction_at(x) < 0.3, "five stops attract toward 0.25");
        assert!(three.fraction_at(x) > 0.3, "three stops attract toward 0.5");
        assert_eq!(five.snap(five.fraction_at(x)), 0.25);
        assert_eq!(three.snap(three.fraction_at(x)), 0.5);
    }

    #[test]
    fn effort_slider_spring_settles_before_ultra_celebration() {
        let start = Instant::now();
        let mut motion = SliderMotion::new(start);
        motion.sample(start, 0.5, None, 5, true, false);
        motion.animate_to(1.0, start, true);
        let (_, near_end, _, burst) =
            motion.sample(start + Duration::from_millis(82), 1.0, None, 5, true, false);
        assert!(near_end > 1.0, "the knob briefly overshoots its final stop");
        assert!(burst.is_none());
        let (_, settled, _, burst) =
            motion.sample(start + SLIDER_SETTLE_DURATION, 1.0, None, 5, true, false);
        assert_eq!(settled, 1.0);
        assert_eq!(burst, Some(0.0));
        assert!(motion.is_fast(start + SLIDER_SETTLE_DURATION));
        let (_, _, _, burst) = motion.sample(
            start + SLIDER_SETTLE_DURATION + SLIDER_BURST_DURATION,
            1.0,
            None,
            5,
            true,
            false,
        );
        assert!(burst.is_none());

        let mut reduced = SliderMotion::new(start);
        reduced.sample(start, 0.5, None, 5, true, true);
        reduced.animate_to(1.0, start, true);
        let (_, position, _, burst) =
            reduced.sample(start + Duration::from_millis(82), 1.0, None, 5, true, true);
        assert_eq!(position, 1.0);
        assert!(burst.is_none());

        let mut limited_model = SliderMotion::new(start);
        limited_model.sample(start, 0.5, None, 3, false, false);
        limited_model.animate_to(1.0, start, false);
        let (_, _, ultra_mix, burst) =
            limited_model.sample(start + SLIDER_SETTLE_DURATION, 1.0, None, 3, false, false);
        assert_eq!(
            ultra_mix, 0.0,
            "a model ending at High has no Ultra gradient"
        );
        assert!(burst.is_none(), "a model ending at High has no Ultra burst");
    }

    #[test]
    fn effort_slider_reduced_motion_freezes_effects_but_keeps_input() {
        let mut tree =
            UiTree::new(Element::effort_slider("effort", 0.5, 3).size(Some(220.0), Some(42.0)));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(220.0, 42.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::EffortSlider {
                reduced_motion: true,
                phase,
                ..
            } if *phase == 0.0
        )));
        tree.handle_event(UiEvent::PointerDown(Point::new(110.0, 21.0)));
        assert!(!tree.is_animating());
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(165.0, 21.0))),
            vec![UiAction::SliderChanged {
                id: "effort".into(),
                fraction: 1.0,
                committed: true,
            }]
        );
    }

    #[test]
    fn effort_slider_keyboard_steps_follow_model_level_count() {
        let mut tree =
            UiTree::new(Element::effort_slider("effort", 0.5, 5).size(Some(220.0), Some(42.0)));
        tree.layout(Size::new(220.0, 42.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(110.0, 21.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(110.0, 21.0)));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::ArrowRight,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::SliderChanged {
                id: "effort".into(),
                fraction: 0.75,
                committed: true,
            }]
        );
    }

    #[test]
    fn effort_slider_animates_only_while_visible_and_freezes_for_reduced_motion() {
        let slider = || Element::effort_slider("effort", 0.5, 4).size(Some(220.0), Some(42.0));
        let mut tree = UiTree::new(slider());
        let viewport = Size::new(220.0, 42.0);
        let start = Instant::now();
        let phase = |frame: &Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::EffortSlider { phase, .. } => Some(*phase),
                _ => None,
            })
        };
        let initial = phase(&tree.layout_at(viewport, start)).unwrap();
        assert_eq!(
            tree.animation_repaint_after(),
            Some(Duration::from_millis(33))
        );
        assert_eq!(
            phase(&tree.layout_at(viewport, start + Duration::from_millis(16))),
            Some(initial)
        );
        assert!(
            phase(&tree.layout_at(viewport, start + Duration::from_millis(34))).unwrap() > initial
        );

        tree.set_root(Element::spacer());
        tree.layout_at(viewport, start + Duration::from_millis(50));
        assert_eq!(tree.animation_repaint_after(), None);

        tree.set_reduced_motion(true);
        tree.set_root(slider());
        let frozen = phase(&tree.layout_at(viewport, start + Duration::from_millis(84))).unwrap();
        assert_eq!(
            phase(&tree.layout_at(viewport, start + Duration::from_millis(134))),
            Some(frozen)
        );
        assert_eq!(tree.animation_repaint_after(), None);
    }

    #[test]
    fn clip_children_limits_descendant_hitboxes_without_clipping_parent_surface() {
        let parent_color = Color::rgb(29, 31, 34);
        let clipped = Element::panel(Element::row(vec![
            Element::button("wide_child", "Action").size(Some(150.0), Some(28.0))
        ]))
        .background(parent_color)
        .border(Color::rgb(70, 70, 70), 1.0)
        .padding(4.0)
        .clip_children()
        .size(Some(80.0), Some(40.0));
        let frame = UiTree::new(clipped).layout(Size::new(80.0, 40.0));
        let child = frame
            .regions
            .iter()
            .find(|region| region.id == "wide_child")
            .unwrap();
        assert_eq!(child.rect, Rect::new(4.0, 4.0, 72.0, 28.0));
        assert_eq!(frame.hit_test(Point::new(75.0, 16.0)), Some("wide_child"));
        assert_eq!(frame.hit_test(Point::new(90.0, 16.0)), None);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == Rect::new(0.0, 0.0, 80.0, 40.0) && *color == parent_color
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::PushClip(rect) if *rect == Rect::new(4.0, 4.0, 72.0, 32.0)
        )));
    }

    #[test]
    fn tab_indicator_moves_without_moving_click_regions_and_reduced_motion_snaps() {
        let palette = ComponentPalette::neutral_dark();
        let make_tabs = |active: &str| {
            Tabs::new(
                vec![
                    Tab::new("group", "Grupo", 80.0).active(active == "group"),
                    Tab::new("project", "Proyecto", 96.0).active(active == "project"),
                ],
                palette,
            )
            .id("session-grouping")
            .render()
        };
        let start = Instant::now();
        let mut tree = UiTree::new(make_tabs("group"));
        let viewport = Size::new(240.0, 32.0);
        let initial = tree.layout_at(viewport, start);
        let indicator = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == palette.selected => {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("selected tab must have a painted indicator")
        };
        let start_rect = indicator(&initial);
        let project_region = initial
            .regions
            .iter()
            .find(|region| region.id == "project")
            .unwrap()
            .rect;
        tree.set_root(make_tabs("project"));
        let switched = tree.layout_at(viewport, start + Duration::from_millis(10));
        assert_eq!(indicator(&switched).x, start_rect.x);
        let halfway = tree.layout_at(viewport, start + Duration::from_millis(95));
        assert!(indicator(&halfway).x > start_rect.x);
        assert!(indicator(&halfway).x < project_region.x);
        assert_eq!(
            halfway.hit_test(Point::new(project_region.x + 5.0, 16.0)),
            Some("project")
        );
        // A spring covers 99% of the way in the duration and then settles.
        let arriving = tree.layout_at(viewport, start + Duration::from_millis(180));
        assert!((indicator(&arriving).x - project_region.x).abs() < 1.5);
        let complete = tree.layout_at(viewport, start + Duration::from_millis(600));
        assert_eq!(indicator(&complete).x, project_region.x);
        tree.set_reduced_motion(true);
        tree.set_root(make_tabs("group"));
        assert_eq!(
            indicator(&tree.layout_at(viewport, start + Duration::from_millis(190))).x,
            start_rect.x
        );
    }

    #[test]
    fn popover_entrance_fades_paint_but_keeps_stable_targets() {
        let popup_color = Color::rgb(71, 75, 81);
        let make_open = || {
            Element::popover(
                Element::button("trigger", "Abrir").size(Some(92.0), Some(30.0)),
                Element::panel(Element::button("option", "Opción").size(Some(112.0), Some(34.0)))
                    .background(popup_color)
                    .size(Some(124.0), Some(42.0)),
                "dismiss",
                PopoverPlacement::BottomStart,
            )
        };
        let start = Instant::now();
        let viewport = Size::new(300.0, 200.0);
        let mut tree = UiTree::new(make_open());
        let first = tree.layout_at(viewport, start);
        let target = first
            .regions
            .iter()
            .find(|region| region.id == "option")
            .unwrap()
            .rect;
        let popup_fill = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if (color.r, color.g, color.b)
                            == (popup_color.r, popup_color.g, popup_color.b) =>
                    {
                        Some((*rect, *color))
                    }
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(popup_fill(&first).1.a, 0);
        let halfway = tree.layout_at(viewport, start + Duration::from_millis(70));
        let (animated_rect, animated_color) = popup_fill(&halfway);
        assert!(animated_color.a > 0 && animated_color.a < 255);
        assert_eq!(
            halfway.hit_test(Point::new(target.x + 8.0, target.y + 8.0)),
            Some("option")
        );
        let complete = tree.layout_at(viewport, start + Duration::from_millis(140));
        assert_eq!(popup_fill(&complete).1.a, 255);
        assert!((animated_rect.y - popup_fill(&complete).0.y).abs() > 0.1);
        assert!(tree.focus("option"));
        tree.set_root(Element::button("trigger", "Abrir"));
        let closed = tree.layout_at(viewport, start + Duration::from_millis(150));
        assert!(tree.popover_transitions.is_empty());
        assert_eq!(tree.focused_id(), Some("trigger"));
        assert_eq!(popup_fill(&closed).1.a, 255);
        assert!(!closed.regions.iter().any(|region| {
            region.id == "option"
                || region.id == "dismiss"
                || region.id == "popover_surface:dismiss"
        }));
        assert_ne!(
            closed.hit_test(Point::new(target.x + 8.0, target.y + 8.0)),
            Some("option")
        );
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            })
            .is_empty());
        let exiting = tree.layout_at(viewport, start + Duration::from_millis(205));
        assert!(popup_fill(&exiting).1.a > 0 && popup_fill(&exiting).1.a < 255);
        let gone = tree.layout_at(viewport, start + Duration::from_millis(260));
        assert!(!gone.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. }
                if (color.r, color.g, color.b)
                    == (popup_color.r, popup_color.g, popup_color.b)
        )));
        tree.set_reduced_motion(true);
        tree.set_root(make_open());
        let reduced = tree.layout_at(viewport, start + Duration::from_millis(270));
        assert_eq!(popup_fill(&reduced).1.a, 255);
        tree.set_root(Element::button("trigger", "Abrir"));
        let reduced_close = tree.layout_at(viewport, start + Duration::from_millis(280));
        assert!(!reduced_close.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. }
                if (color.r, color.g, color.b)
                    == (popup_color.r, popup_color.g, popup_color.b)
        )));
    }

    #[test]
    fn opt_in_popover_scale_and_close_duration_never_move_hit_targets() {
        let ink = Color::rgb(86, 89, 92);
        let open = || {
            Element::popover(
                Element::button("trigger", "Open").size(Some(90.0), Some(32.0)),
                Element::panel(Element::button("choice", "Choose"))
                    .background(ink)
                    .size(Some(160.0), Some(64.0)),
                "dismiss",
                PopoverPlacement::BottomStart,
            )
            .popover_animation_ms(200)
            .popover_close_animation_ms(150)
            .popover_open_scale(0.96)
        };
        let surface = |frame: &Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if (color.r, color.g, color.b) == (ink.r, ink.g, ink.b) =>
                {
                    Some((*rect, color.a))
                }
                _ => None,
            })
        };
        let start = Instant::now();
        let viewport = Size::new(400.0, 260.0);
        let mut tree = UiTree::new(open());
        let first = tree.layout_at(viewport, start);
        let choice = first
            .regions
            .iter()
            .find(|region| region.id == "choice")
            .unwrap();
        let choice_point = Point::new(choice.rect.x + 4.0, choice.rect.y + 4.0);
        assert_eq!(first.hit_test(choice_point), Some("choice"));
        let middle = tree.layout_at(viewport, start + Duration::from_millis(100));
        let (scaled_rect, alpha) = surface(&middle).unwrap();
        assert!(alpha > 0 && alpha < 255);
        assert!(scaled_rect.width < 160.0);
        assert_eq!(middle.hit_test(choice_point), Some("choice"));
        tree.layout_at(viewport, start + Duration::from_millis(200));
        tree.set_root(Element::button("trigger", "Open"));
        let closed = tree.layout_at(viewport, start + Duration::from_millis(210));
        assert!(surface(&closed).is_some());
        assert_ne!(closed.hit_test(choice_point), Some("choice"));
        assert!(!closed.regions.iter().any(|region| region.id == "choice"));
        let fading = tree.layout_at(viewport, start + Duration::from_millis(330));
        assert!(surface(&fading).is_some_and(|(_, alpha)| alpha > 0));
        let gone = tree.layout_at(viewport, start + Duration::from_millis(361));
        assert!(surface(&gone).is_none());
    }

    #[test]
    fn keyed_popover_crossfades_and_morphs_without_old_hit_regions() {
        let surface_color = Color::rgb(71, 74, 79);
        let view = |key: &str, id: &str, label: &str, height: f32| {
            Element::popover(
                Element::button("trigger", "Open").size(Some(90.0), Some(32.0)),
                Element::panel(Element::button(id, label))
                    .background(surface_color)
                    .size(Some(180.0), Some(height)),
                "dismiss",
                PopoverPlacement::BottomStart,
            )
            .popover_animation_ms(0)
            .popover_view_key(key)
        };
        let shell = |frame: &Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == surface_color => Some(*rect),
                _ => None,
            })
        };
        let start = Instant::now();
        let viewport = Size::new(420.0, 320.0);
        let mut tree = UiTree::new(view("overview", "old", "Old", 70.0));
        let old = tree.layout_at(viewport, start);
        assert_eq!(shell(&old).unwrap().height, 70.0);
        tree.set_root(view("details", "new", "New", 130.0));
        let switching = tree.layout_at(viewport, start + Duration::from_millis(10));
        assert_eq!(shell(&switching).unwrap().height, 70.0);
        assert!(!switching.regions.iter().any(|region| region.id == "old"));
        let new_region = switching
            .regions
            .iter()
            .find(|region| region.id == "new")
            .unwrap();
        assert_eq!(
            switching.hit_test(Point::new(new_region.rect.x + 4.0, new_region.rect.y + 4.0)),
            Some("new")
        );
        let middle = tree.layout_at(viewport, start + Duration::from_millis(140));
        let height = shell(&middle).unwrap().height;
        assert!(height > 70.0 && height < 130.0);
        assert!(middle.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "Old" && color.a > 0 && color.a < 255
        )));
        assert!(middle.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "New" && color.a > 0 && color.a < 255
        )));
        let finished = tree.layout_at(viewport, start + Duration::from_millis(271));
        assert_eq!(shell(&finished).unwrap().height, 130.0);
        assert!(!finished.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Old"
        )));
        tree.set_reduced_motion(true);
        tree.set_root(view("overview", "old", "Old", 70.0));
        let reduced = tree.layout_at(viewport, start + Duration::from_millis(280));
        assert_eq!(shell(&reduced).unwrap().height, 70.0);
        assert!(!reduced.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "New"
        )));
    }

    #[test]
    fn menu_entrance_has_one_surface_and_only_the_hovered_row_gets_a_fill() {
        let mut palette = ComponentPalette::neutral_dark();
        palette.surface = Color::rgb(30, 30, 30);
        palette.raised = palette.surface;
        palette.hover = Color::rgb(48, 48, 48);
        let menu = DropdownMenu::new(
            Element::button("trigger", "Abrir").size(Some(90.0), Some(30.0)),
            vec![
                DropdownMenuItem::new("first", "Primera").into(),
                DropdownMenuItem::new("second", "Segunda").into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(220.0)
        .render();
        let start = Instant::now();
        let viewport = Size::new(320.0, 220.0);
        let mut tree = UiTree::new(menu);
        tree.layout_at(viewport, start);
        let entering = tree.layout_at(viewport, start + Duration::from_millis(70));
        let surface = entering.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if rect.width == 220.0 && *color == Color::rgba(30, 30, 30, color.a) =>
            {
                Some(*color)
            }
            _ => None,
        });
        assert!(surface.is_some_and(|color| color.a > 0 && color.a < 255));
        assert!(!entering.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, .. } if rect.width == 208.0 && rect.height == 36.0
        )));

        let first = entering
            .regions
            .iter()
            .find(|region| region.id == "first")
            .unwrap();
        let point = Point::new(first.rect.x + 8.0, first.rect.y + 8.0);
        tree.handle_event(UiEvent::PointerMove(point));
        tree.set_reduced_motion(true);
        let hovered = tree.layout_at(viewport, start + Duration::from_millis(80));
        assert_eq!(hovered.hit_test(point), Some("first"));
        assert_eq!(
            hovered
                .commands
                .iter()
                .filter(|command| matches!(command,
                    DrawCommand::FillRect { rect, color, .. }
                        if rect.width == 208.0 && rect.height == 36.0 && *color == palette.hover
                ))
                .count(),
            1,
        );
    }

    #[test]
    fn closing_during_popover_entrance_preserves_current_opacity() {
        let start = Instant::now();
        let viewport = Size::new(300.0, 200.0);
        let popup_color = Color::rgb(111, 112, 113);
        let open = || {
            Element::popover(
                Element::button("trigger", "Open"),
                Element::panel(Element::text("Item"))
                    .background(popup_color)
                    .size(Some(100.0), Some(40.0)),
                "dismiss",
                PopoverPlacement::BottomStart,
            )
        };
        let mut tree = UiTree::new(open());
        tree.layout_at(viewport, start);
        let entering = tree.layout_at(viewport, start + Duration::from_millis(40));
        let opacity = |frame: &Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { color, .. }
                    if (color.r, color.g, color.b)
                        == (popup_color.r, popup_color.g, popup_color.b) =>
                {
                    Some(color.a)
                }
                _ => None,
            })
        };
        let visible = opacity(&entering).unwrap();
        assert!(visible > 0 && visible < 255);
        tree.set_root(Element::button("trigger", "Open"));
        let closing = tree.layout_at(viewport, start + Duration::from_millis(40));
        assert_eq!(opacity(&closing), Some(visible));
        let fading = tree.layout_at(viewport, start + Duration::from_millis(80));
        let fading_alpha = opacity(&fading).unwrap();
        assert!(fading_alpha < visible);
        tree.set_root(open());
        let reopened = tree.layout_at(viewport, start + Duration::from_millis(80));
        assert_eq!(opacity(&reopened), Some(fading_alpha));
        assert!(tree.popover_exits.is_empty());
    }

    #[test]
    fn keyboard_shortcut_caps_use_proportional_measurement_and_remain_passive() {
        let fill = Color::rgba(235, 235, 235, 25);
        let ink = Color::rgb(235, 235, 235);
        let mut tree = UiTree::new(Element::row(vec![
            Element::keyboard_shortcut("WW+ii", fill, None)
                .foreground(ink)
                .size(None, Some(shortcut::HEIGHT)),
            Element::button("next", "Next").size(Some(50.0), Some(30.0)),
        ]));
        tree.set_text_measurer(|text, size, family| {
            if size == shortcut::FONT_SIZE {
                assert_eq!(family, FontFamily::Sans);
                text.chars()
                    .map(|character| if character == 'W' { 12.0 } else { 2.0 })
                    .sum()
            } else {
                text.chars().count() as f32 * 7.0
            }
        });
        let frame = tree.layout(Size::new(160.0, 30.0));
        let caps: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect {
                    rect,
                    color,
                    radius,
                } if *color == fill => {
                    assert_eq!(*radius, shortcut::radius());
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0].width, shortcut::cap_width(24.0));
        assert_eq!(caps[1].width, shortcut::cap_width(4.0));
        assert_eq!(
            caps[1].x - caps[0].x - caps[0].width,
            shortcut::key_gap(|separator| separator.len() as f32 * 2.0)
        );
        let labels: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    position,
                    text,
                    color,
                    font_family,
                    ..
                } if *color == ink => {
                    assert_eq!(*font_family, FontFamily::Sans);
                    Some((*position, text.as_str()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            labels.iter().map(|(_, label)| *label).collect::<Vec<_>>(),
            ["WW", "ii"]
        );
        let line_height = tree.sans_metrics.at_size(shortcut::FONT_SIZE).line_height;
        for ((position, _), (cap, text_width)) in labels.iter().zip(caps.iter().zip([24.0, 4.0])) {
            assert!((position.x + text_width / 2.0 - cap.x - cap.width / 2.0).abs() < 0.01);
            assert!((position.y + line_height / 2.0 - cap.y - cap.height / 2.0).abs() < 0.01);
        }
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. }
                if text == "+" && *color == shortcut::separator_ink(ink))));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::StrokeRect { rect, color, .. }
                if *rect == caps[0] && *color == shortcut::cap_border(ink))));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "next");
        assert_eq!(frame.regions[0].rect.x, caps[1].x + caps[1].width);
        assert_eq!(frame.hit_test(Point::new(5.0, 15.0)), None);
        assert!(
            frame.text_blocks.is_empty(),
            "decorative caps are not selectable text"
        );
    }

    #[test]
    fn keyboard_shortcut_omits_whole_chord_when_bounded_and_keeps_literal_plus() {
        let fill = Color::rgba(235, 235, 235, 25);
        let shortcut = |value: &str, max_width| {
            Element::keyboard_shortcut(value, fill, max_width).size(None, Some(shortcut::HEIGHT))
        };
        let mut tree = UiTree::new(Element::row(vec![
            shortcut("Ctrl++", Some(1.0)),
            Element::button("next", "Next").size(Some(50.0), Some(30.0)),
        ]));
        let bounded = tree.layout(Size::new(160.0, 30.0));
        assert_eq!(bounded.regions[0].rect.x, 0.0);
        assert!(!bounded.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == fill
        )));
        tree.set_root(Element::column(vec![shortcut("Ctrl++", None)]));
        let narrow = tree.layout(Size::new(20.0, 30.0));
        assert!(!narrow.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == fill
        )));
        tree.set_root(Element::column(vec![shortcut("Ctrl++", None)]));
        let wide = tree.layout(Size::new(160.0, 30.0));
        let labels: Vec<_> = wide
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, ["Ctrl", "+", "+"], "the joining plus, then the literal key");
        tree.set_root(shortcut("   ", None));
        let empty = tree.layout(Size::new(160.0, 30.0));
        assert!(empty.commands.is_empty());
        assert!(empty.regions.is_empty());
    }

    #[test]
    fn input_shortcut_preserves_text_space_and_uses_text_ink() {
        let ink = Color::rgb(235, 235, 235);
        let muted = Color::rgb(120, 120, 120);
        let search = |width| {
            Element::text_input("search", "", "Find")
                .leading_icon(Icon::Search)
                .leading_icon_tint(muted)
                .input_shortcut("Ctrl+K")
                .foreground(ink)
                .size(Some(width), Some(34.0))
        };
        let mut tree = UiTree::new(search(200.0));
        let wide = tree.layout(Size::new(200.0, 34.0));
        for key in ["Ctrl", "K"] {
            assert!(wide.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, font_family, .. }
                    if text == key && *color == ink && *font_family == FontFamily::Sans
            )));
        }
        assert!(wide.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::Search, tint, .. } if *tint == muted
        )));
        assert!(wide
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == shortcut::cap_fill(ink) =>
                    Some(rect),
                _ => None,
            })
            .all(|rect| rect.height == shortcut::HEIGHT
                && rect.y >= 0.0
                && rect.y + rect.height <= 34.0));
        let wide_text = tree.inputs["search"].content_width;
        assert!(wide_text >= 24.0);
        tree.set_root(search(80.0));
        let narrow = tree.layout(Size::new(80.0, 34.0));
        assert!(!narrow.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K"
        )));
        assert_eq!(tree.inputs["search"].content_width, 40.0);
        assert_eq!(narrow.regions.len(), 1);
        assert_eq!(narrow.regions[0].id, "search");
        assert!(tree.focus("search"));
        let focused = tree.layout(Size::new(80.0, 34.0));
        assert!(!focused.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K"
        )));
    }

    #[test]
    fn a_control_without_a_ring_shows_its_focus_border_under_the_keyboard_only() {
        let mark = Color::rgb(1, 2, 3);
        let marked = |frame: &Frame| {
            frame.commands.iter().any(|command| {
                matches!(command, DrawCommand::StrokeRect { color, width, .. }
                    if *color == mark && *width == 1.5)
            })
        };
        let button = |ring: bool| {
            Element::button("go", "Go")
                .border(Color::rgb(9, 9, 9), 1.0)
                .focus_border(mark, 1.5)
                .focus_ring(ring)
                .size(Some(80.0), Some(28.0))
        };
        let window = Size::new(80.0, 28.0);
        let mut tree = UiTree::new(button(false));
        assert!(!marked(&tree.layout(window)));
        assert!(tree.focus_from_pointer("go"));
        assert!(!marked(&tree.layout(window)));
        assert!(tree.focus("go"));
        assert!(marked(&tree.layout(window)));
        // With its ring, a control keeps the ring and its own border.
        tree.set_root(button(true));
        assert!(!marked(&tree.layout(window)));
    }

    #[test]
    fn input_trailing_space_keeps_the_end_free_of_text_caret_and_hint() {
        let field = |trailing: f32| {
            Element::text_input("search", "", "Find")
                .leading_icon(Icon::Search)
                .input_shortcut("K")
                .input_trailing_space(trailing)
                .size(Some(200.0), Some(34.0))
        };
        let hint_right = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == "K" => Some(position.x),
                    _ => None,
                })
                .next()
                .expect("the hint")
        };
        let mut tree = UiTree::new(field(0.0));
        let plain = tree.layout(Size::new(200.0, 34.0));
        let (plain_x, plain_width) = (
            tree.inputs["search"].content_x,
            tree.inputs["search"].content_width,
        );
        tree.set_root(field(24.0));
        let kept = tree.layout(Size::new(200.0, 34.0));
        // The start is where it was; the text, and the hint at its end, give up the room.
        assert_eq!(tree.inputs["search"].content_x, plain_x);
        assert_eq!(tree.inputs["search"].content_width, plain_width - 24.0);
        assert_eq!(hint_right(&kept), hint_right(&plain) - 24.0);
        assert_eq!(
            kept.regions[0].rect, plain.regions[0].rect,
            "the input's own box is unchanged"
        );
        // More than the field has leaves no room, never a negative one.
        tree.set_root(field(500.0));
        tree.layout(Size::new(200.0, 34.0));
        assert_eq!(tree.inputs["search"].content_width, 0.0);
        // Not a number, or another kind of element: nothing is kept.
        for element in [
            field(f32::NAN),
            Element::spacer().input_trailing_space(24.0),
        ] {
            assert!(element.kept.is_none());
        }
    }

    #[test]
    fn persistent_input_placeholder_survives_focus_edits_and_refocus() {
        use crate::{Component, ComponentPalette, TextField};

        let palette = ComponentPalette::neutral_dark();
        let root = || {
            Element::column(vec![
                TextField::new("replace", "", "Reemplazar", palette)
                    .persistent_placeholder(true)
                    .render(),
                Element::button("other", "Other").size(None, Some(32.0)),
            ])
        };
        let viewport = Size::new(240.0, 80.0);
        let placeholder = |frame: &Frame| {
            frame
                .commands
                .iter()
                .enumerate()
                .find_map(|(index, command)| match command {
                    DrawCommand::Text {
                        text,
                        position,
                        color,
                        ..
                    } if text == "Reemplazar" => Some((index, *position, *color)),
                    _ => None,
                })
        };
        let mut tree = UiTree::new(root());
        tree.set_reduced_motion(true);
        assert!(placeholder(&tree.layout(viewport)).is_some());
        assert!(tree.focus("replace"));
        let focused = tree.layout(viewport);
        let (label_index, label_position, label_color) =
            placeholder(&focused).expect("focused empty field keeps its purpose visible");
        assert_eq!(label_color.a, 110);
        assert!(
            focused.commands[label_index + 1..].iter().any(|command| {
                matches!(command,
                    DrawCommand::FillRect { rect, color, .. }
                        if rect.x == label_position.x && rect.y == label_position.y
                            && rect.width == 1.5 && color.a > label_color.a
                )
            }),
            "caret must remain visible in front of the placeholder"
        );

        tree.handle_event(UiEvent::Text("ñ".into()));
        let typed = tree.layout(viewport);
        assert!(placeholder(&typed).is_none());
        assert_eq!(tree.input_value("replace"), Some("ñ"));
        assert!(typed.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. }
                if text == "ñ" && *color == palette.text
        )));
        tree.handle_event(UiEvent::Key {
            key: Key::Backspace,
            shift: false,
            ctrl: false,
        });
        assert!(placeholder(&tree.layout(viewport)).is_some());
        assert!(tree.focus("other"));
        assert!(placeholder(&tree.layout(viewport)).is_some());
        assert!(tree.focus("replace"));
        tree.set_root(root());
        assert!(placeholder(&tree.layout(viewport)).is_some());
        assert_eq!(tree.focused_id(), Some("replace"));
        assert_eq!(tree.input_value("replace"), Some(""));
    }

    #[test]
    fn input_placeholder_still_hides_on_focus_by_default() {
        let mut tree =
            UiTree::new(Element::text_input("default", "", "Placeholder").size(None, Some(32.0)));
        let viewport = Size::new(240.0, 32.0);
        assert!(tree
            .layout(viewport)
            .commands
            .iter()
            .any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "Placeholder"
            )));
        assert!(tree.focus("default"));
        assert!(!tree
            .layout(viewport)
            .commands
            .iter()
            .any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "Placeholder"
            )));
    }

    #[test]
    fn tooltip_preserves_anchor_bounds_and_has_no_hit_region() {
        let palette = ComponentPalette::neutral_dark();
        let anchor =
            Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0));
        let baseline = UiTree::new(anchor.clone()).layout(Size::new(300.0, 100.0));
        let wrapped = Tooltip::new(anchor, "toggle", "Toggle sidebar", palette)
            .shortcut("Ctrl+B")
            .render();
        let mut tree = UiTree::new(wrapped);
        let frame = tree.layout(Size::new(300.0, 100.0));
        assert_eq!(frame.regions, baseline.regions);
        assert!(!frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
        assert!(tree.focus("toggle"));
        let focused = tree.layout(Size::new(300.0, 100.0));
        assert_eq!(focused.regions, baseline.regions);
        assert_eq!(
            focused
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::FillTriangle { .. }))
                .count(),
            2,
        );
        assert!(focused.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, .. } if text == "Toggle sidebar"
        )));
        let tooltip_ink = focused
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == "Toggle sidebar" => Some(*color),
                _ => None,
            })
            .unwrap();
        for key in ["Ctrl", "B"] {
            assert!(focused.commands.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, color, font_family, .. }
                    if text == key && *color == tooltip_ink && *font_family == FontFamily::Sans
            )));
        }
    }

    #[test]
    fn tooltip_omits_a_shortcut_that_would_take_the_title_space() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Tooltip::new(
                Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
                "toggle",
                "A",
                palette,
            )
            .shortcut("Ctrl+Shift+B")
            .max_width(64.0)
            .render(),
        );
        tree.layout(Size::new(200.0, 100.0));
        assert!(tree.focus("toggle"));
        let frame = tree.layout(Size::new(200.0, 100.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "A"
        )));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if ["Ctrl", "Shift", "B"].contains(&text.as_str())
        )));
        assert_eq!(frame.regions.len(), 1);
    }

    #[test]
    fn tooltip_hover_waits_and_keyboard_focus_is_immediate() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Tooltip::new(
                Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
                "toggle",
                "Toggle sidebar",
                palette,
            )
            .render(),
        );
        tree.layout(Size::new(200.0, 100.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(16.0, 16.0)));
        let started = tree.tooltip_hover_since.as_ref().unwrap().1;
        let early = tree.layout_at(Size::new(200.0, 100.0), started + TOOLTIP_HOVER_DELAY / 2);
        assert!(!early
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
        let late = tree.layout_at(Size::new(200.0, 100.0), started + TOOLTIP_HOVER_DELAY);
        assert!(late
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
        tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 80.0)));
        let left = tree.layout(Size::new(200.0, 100.0));
        assert!(!left
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        let focused = tree.layout(Size::new(200.0, 100.0));
        assert!(focused
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    }

    #[test]
    fn a_hint_takes_its_own_delay_title_pointer_and_fill() {
        let palette = ComponentPalette::neutral_dark();
        let fill = Color::rgb(10, 10, 12);
        let hint = |tooltip: Tooltip| {
            let mut tree = UiTree::new(tooltip.render());
            tree.set_reduced_motion(true);
            tree.layout(Size::new(240.0, 120.0));
            tree.handle_event(UiEvent::PointerMove(Point::new(16.0, 16.0)));
            let started = tree.tooltip_hover_since.as_ref().unwrap().1;
            (tree, started)
        };
        let anchor = || Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0));
        let body = |frame: &Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if rect.x > 32.0 => Some((*rect, *color)),
                _ => None,
            })
        };
        // The default waits, is 24 tall, has a pointer and the palette's fill.
        let (mut plain, started) = hint(Tooltip::new(anchor(), "mark", "Server", palette).placement(TooltipPlacement::Right));
        assert!(body(&plain.layout_at(Size::new(240.0, 120.0), started)).is_none());
        assert_eq!(plain.animation_repaint_after_at(started), Some(TOOLTIP_HOVER_DELAY));
        let shown = plain.layout_at(Size::new(240.0, 120.0), started + TOOLTIP_HOVER_DELAY);
        let (rect, color) = body(&shown).unwrap();
        assert_eq!((rect.height, color), (TOOLTIP_HEIGHT, palette.tooltip_style().background));
        assert_eq!(shown.commands.iter().filter(|command| matches!(command, DrawCommand::FillTriangle { .. })).count(), 2);
        // No wait, a larger medium title, no pointer and a fill of its own.
        let (mut quick, started) = hint(
            Tooltip::new(anchor(), "mark", "Server", palette)
                .placement(TooltipPlacement::Right)
                .delay_ms(0)
                .title_font_size(16.0)
                .title_font_size(f32::NAN)
                .title_font_size(-1.0)
                .title_font_family(FontFamily::SansMedium)
                .arrow(false)
                .background(fill),
        );
        assert_eq!(quick.animation_repaint_after_at(started), None);
        let shown = quick.layout_at(Size::new(240.0, 120.0), started);
        let (rect, color) = body(&shown).unwrap();
        let grown = FontMetrics::default().at_size(16.0).line_height - FontMetrics::default().at_size(12.0).line_height;
        assert!((rect.height - (TOOLTIP_HEIGHT + grown)).abs() < 0.01, "{rect:?}");
        assert_eq!(color, fill);
        assert!(!shown.commands.iter().any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
        assert!(shown.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, font_family, .. }
                if text == "Server" && *font_size == 16.0 && *font_family == FontFamily::SansMedium
        )));
        // A delay of its own is what the tree waits for.
        let (mut slow, started) = hint(Tooltip::new(anchor(), "mark", "Server", palette).placement(TooltipPlacement::Right).delay_ms(800));
        assert_eq!(slow.animation_repaint_after_at(started), Some(Duration::from_millis(800)));
        assert!(body(&slow.layout_at(Size::new(240.0, 120.0), started + TOOLTIP_HOVER_DELAY)).is_none());
    }

    #[test]
    fn a_small_icon_is_drawn_and_measured_at_its_side_between_8_and_16() {
        for (asked, side) in [(12.0, 12.0), (4.0, crate::SMALL_ICON_SIZE), (30.0, ICON_SIZE), (f32::NAN, ICON_SIZE)] {
            let icon = Element::icon(Icon::Volume).small_icon(asked);
            // Natural size in a row, centered in a larger box in a column.
            let frame = UiTree::new(Element::column(vec![
                Element::row(vec![icon.clone(), Element::spacer().flex(1.0)]),
                Element::panel(icon).size(Some(20.0), Some(20.0)),
                Element::spacer().flex(1.0),
            ]))
            .layout(Size::new(100.0, 100.0));
            let rects: Vec<Rect> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { rect, icon: Icon::Volume, .. } => Some(*rect),
                    _ => None,
                })
                .collect();
            assert_eq!(rects.len(), 2, "{asked}");
            assert_eq!(rects[0], Rect::new(0.0, 0.0, side, side), "{asked}");
            assert_eq!(rects[1], Rect::new((20.0 - side) / 2.0, side + (20.0 - side) / 2.0, side, side), "{asked}");
        }
    }

    #[test]
    fn a_hint_for_a_control_inside_its_anchor_points_at_that_control() {
        let palette = ComponentPalette::neutral_dark();
        // A link at the start of a wide line, hinted on the whole line.
        let line = Element::row(vec![
            Element::clickable("link", Element::text("link")).size(Some(40.0), Some(20.0)),
            Element::spacer().flex(1.0),
        ])
        .size(None, Some(20.0));
        let size = Size::new(400.0, 200.0);
        let mut tree = UiTree::new(Element::column(vec![
            Element::spacer().size(None, Some(100.0)),
            Tooltip::new(line, "link", "Where it goes", palette).render(),
        ]));
        tree.layout(size);
        tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 110.0)));
        let started = tree.tooltip_hover_since.as_ref().unwrap().1;
        let frame = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY);
        let tip = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillTriangle { points, .. } => Some(*points),
                _ => None,
            })
            .expect("the hint is open");
        // Its arrow stands over the link, not over the middle of the line.
        let center = tip.iter().map(|point| point.x).sum::<f32>() / 3.0;
        assert!(center > 0.0 && center < 40.0, "{center}");
    }

    #[test]
    fn pointer_hover_and_keyboard_focus_do_not_show_two_tooltips() {
        let palette = ComponentPalette::neutral_dark();
        let tooltip = |id, title| {
            Tooltip::new(
                Element::icon_button(id, Icon::Add, "").size(Some(32.0), Some(32.0)),
                id,
                title,
                palette,
            )
            .render()
        };
        let mut tree = UiTree::new(Element::row(vec![
            tooltip("first", "First tooltip"),
            tooltip("second", "Second tooltip"),
        ]));
        tree.layout(Size::new(160.0, 80.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(48.0, 16.0)));
        tree.tooltip_hover_since = Some(("second".into(), Instant::now() - TOOLTIP_HOVER_DELAY));
        assert!(tree.focus("first"));
        let frame = tree.layout(Size::new(160.0, 80.0));
        let titles: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. }
                    if matches!(text.as_str(), "First tooltip" | "Second tooltip") =>
                {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec!["First tooltip"]);

        tree.handle_event(UiEvent::PointerMove(Point::new(49.0, 16.0)));
        let frame = tree.layout(Size::new(160.0, 80.0));
        let titles: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. }
                    if matches!(text.as_str(), "First tooltip" | "Second tooltip") =>
                {
                    Some(text.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(titles, vec!["Second tooltip"]);
    }

    #[test]
    fn tooltip_flips_from_top_and_clamps_to_viewport() {
        let viewport = Rect::new(0.0, 0.0, 180.0, 100.0);
        let anchor = Rect::new(154.0, 4.0, 24.0, 24.0);
        let (body, actual) = place_tooltip(
            anchor,
            Size::new(140.0, 32.0),
            viewport,
            TooltipPlacement::Above,
        );
        assert_eq!(actual, TooltipPlacement::Below);
        assert!(body.x >= 8.0);
        assert!(body.x + body.width <= viewport.width - 8.0);
        assert!(body.y >= anchor.y + anchor.height);
        assert!(body.y + body.height <= viewport.height - 8.0);
    }

    #[test]
    fn tooltips_prefer_the_requested_side_and_avoid_the_anchor_at_viewport_edges() {
        for (viewport, anchor, desired, preferred, expected) in [
            (
                Rect::new(0.0, 0.0, 400.0, 300.0),
                Rect::new(6.0, 100.0, 24.0, 24.0),
                Size::new(120.0, 36.0),
                TooltipPlacement::Left,
                TooltipPlacement::Right,
            ),
            (
                Rect::new(0.0, 0.0, 400.0, 300.0),
                Rect::new(370.0, 100.0, 24.0, 24.0),
                Size::new(120.0, 36.0),
                TooltipPlacement::Right,
                TooltipPlacement::Left,
            ),
            (
                Rect::new(0.0, 0.0, 500.0, 90.0),
                Rect::new(200.0, 34.0, 24.0, 24.0),
                Size::new(150.0, 50.0),
                TooltipPlacement::Above,
                TooltipPlacement::Right,
            ),
            (
                Rect::new(0.0, 0.0, 300.0, 400.0),
                Rect::new(100.0, 150.0, 100.0, 24.0),
                Size::new(140.0, 44.0),
                TooltipPlacement::Left,
                TooltipPlacement::Below,
            ),
            (
                Rect::new(0.0, 0.0, 400.0, 300.0),
                Rect::new(200.0, 260.0, 24.0, 24.0),
                Size::new(140.0, 36.0),
                TooltipPlacement::Below,
                TooltipPlacement::Above,
            ),
        ] {
            let (body, actual) = place_tooltip(anchor, desired, viewport, preferred);
            assert_eq!(actual, expected);
            assert!(
                body.intersection(anchor).is_none(),
                "hint covers trigger: {body:?} / {anchor:?}"
            );
            assert!(body.x >= viewport.x + 8.0 && body.y >= viewport.y + 8.0);
            assert!(body.x + body.width <= viewport.x + viewport.width - 8.0);
            assert!(body.y + body.height <= viewport.y + viewport.height - 8.0);
        }
    }

    #[test]
    fn lateral_menu_tooltip_falls_back_on_narrow_viewports_without_losing_hover_or_actions() {
        let palette = ComponentPalette::neutral_dark();
        for width in [360.0, 900.0] {
            let hint = Tooltip::new(
                Element::button("exact-model", "Exact model").size(Some(328.0), Some(42.0)),
                "exact-model",
                "Exact Messages model",
                palette,
            )
            .placement(TooltipPlacement::Left)
            .description("Entrada máxima: 100000\nSalida máxima: 8192\nContexto: no informado")
            .footer("Proveedor propio · ID exacto")
            .render();
            let popup = Element::column(vec![hint])
                .padding(8.0)
                .size(Some(344.0), Some(74.0));
            let root = Element::column(vec![
                Element::spacer().size(None, Some(140.0)),
                Element::popover(
                    Element::button("models", "Modelos").size(Some(100.0), Some(30.0)),
                    popup,
                    "dismiss-models",
                    PopoverPlacement::BottomStart,
                ),
            ]);
            let mut tree = UiTree::new(root);
            tree.set_reduced_motion(true);
            let start = Instant::now();
            let viewport = Size::new(width, 640.0);
            let initial = tree.layout_at(viewport, start);
            let target = initial
                .regions
                .iter()
                .find(|region| region.id == "exact-model")
                .unwrap()
                .rect;
            let point = Point::new(
                target.x + target.width / 2.0,
                target.y + target.height / 2.0,
            );
            tree.handle_event_at(UiEvent::PointerMove(point), start);
            for elapsed in [
                TOOLTIP_HOVER_DELAY,
                TOOLTIP_HOVER_DELAY + Duration::from_millis(100),
            ] {
                let frame = tree.layout_at(viewport, start + elapsed);
                assert_eq!(frame.hit_test(point), Some("exact-model"));
                assert!(
                    frame.commands.iter().any(|command| matches!(command,
                        DrawCommand::Text { text, .. } if text == "Entrada máxima: 100000"
                    )),
                    "missing exact metadata at {width}px"
                );
                let body = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Shadow { rect, .. } => Some(*rect),
                        _ => None,
                    })
                    .expect("tooltip is painted");
                assert!(body.x >= 8.0 && body.y >= 8.0);
                assert!(body.x + body.width <= width - 8.0);
                assert!(body.y + body.height <= viewport.height - 8.0);
                assert_eq!(
                    frame
                        .regions
                        .iter()
                        .filter(|region| region.id == "exact-model")
                        .count(),
                    1
                );
            }
            assert!(tree.focus("exact-model"));
            let focused = tree.layout_at(viewport, start + Duration::from_secs(1));
            assert!(focused.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "Proveedor propio · ID exacto"
            )));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked {
                    id: "exact-model".into()
                }]
            );
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "exact-model".into()
                }]
            );
        }
    }

    #[test]
    fn popover_container_exception_still_blocks_nested_and_context_menus() {
        let panel = |nested| {
            let field = Element::text_input("field", "editable", "").size(Some(180.0), Some(32.0));
            let contents = if nested {
                Element::column(vec![
                    field,
                    Element::popover(
                        Element::button("nested_trigger", "Menu"),
                        Element::button("nested_action", "Action").size(Some(120.0), Some(32.0)),
                        "nested_dismiss",
                        PopoverPlacement::BottomStart,
                    ),
                ])
            } else {
                field
            };
            Element::popover(
                Element::button("panel_trigger", "Panel").size(Some(80.0), Some(32.0)),
                contents.size(Some(220.0), Some(160.0)),
                "panel_dismiss",
                PopoverPlacement::BottomStart,
            )
            .popover_initial_focus("field")
        };
        let size = Size::new(600.0, 440.0);
        let mut tree = UiTree::new(panel(false));
        tree.set_reduced_motion(true);
        tree.layout(size);
        assert!(tree.has_open_popover());
        assert!(!tree.has_open_popover_except("panel_dismiss"));
        assert!(tree.has_open_popover_except("other_container"));
        assert!(tree.focus("field"));
        tree.handle_event(UiEvent::ContextMenuKeyboard);
        assert!(tree.context_menu_open());
        assert!(tree.has_open_popover_except("panel_dismiss"));
        tree.dismiss_context_menu();
        tree.set_root(panel(true));
        tree.layout(size);
        assert!(tree.has_open_popover_except("panel_dismiss"));
    }

    #[test]
    fn root_popover_positions_below_the_explicit_trigger_not_its_viewport_allocation() {
        let mut tree = UiTree::new(Element::popover(
            Element::button("trigger", "Abrir").size(Some(100.0), Some(30.0)),
            Element::button("option", "Opción").size(Some(160.0), Some(28.0)),
            "dismiss",
            PopoverPlacement::BottomStart,
        ));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 300.0));
        let trigger = frame
            .regions
            .iter()
            .find(|region| region.id == "trigger")
            .unwrap()
            .rect;
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap()
            .rect;
        assert_eq!(trigger.width, 100.0);
        assert_eq!(trigger.height, 30.0);
        assert!(surface.y >= trigger.y + trigger.height);
        assert_eq!(surface.height, 28.0);
        assert!(frame.regions.iter().any(|region| region.id == "option"));
    }

    #[test]
    fn end_aligned_popover_uses_the_whole_composite_anchor() {
        for controls in [1, 2] {
            let mut header = vec![
                Element::button("breadcrumb", "src").size(Some(40.0), Some(30.0)),
                Element::spacer().flex(1.0),
            ];
            if controls == 2 {
                header.push(Element::button("more", "…").size(Some(30.0), Some(30.0)));
            }
            let mut tree = UiTree::new(Element::row(vec![
                Element::spacer().size(Some(311.0), None),
                Element::popover(
                    Element::row(header).size(Some(240.0), Some(30.0)),
                    Element::text_input("search", "", "Buscar").size(Some(232.0), Some(76.0)),
                    "search_close",
                    PopoverPlacement::BottomEnd,
                )
                .popover_non_modal(),
            ]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(560.0, 300.0));
            let surface = frame
                .regions
                .iter()
                .find(|region| region.id == "popover_surface:search_close")
                .unwrap()
                .rect;
            assert_eq!(surface.x, 319.0, "{controls} controls: {surface:?}");
            assert_eq!(surface.x + surface.width, 551.0);
            assert!(surface.y >= 30.0);
        }
    }

    #[test]
    fn viewport_wide_popover_anchor_keeps_a_usable_bounded_surface() {
        let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
        let rect = place_popover(
            viewport,
            Size::new(200.0, 180.0),
            viewport,
            PopoverPlacement::BottomStart,
            4.0,
        );
        assert_eq!(rect.height, 180.0);
        assert!(rect.y >= 8.0 && rect.y + rect.height <= 292.0);
    }

    #[test]
    fn bottom_popover_ignores_anchor_and_gap_and_meets_the_safe_viewport_edge() {
        let viewport = Rect::new(12.0, 24.0, 390.0, 700.0);
        for anchor in [viewport, Rect::new(80.0, 400.0, 40.0, 30.0)] {
            for gap in [0.0, 90.0] {
                assert_eq!(
                    place_popover(anchor, Size::new(560.0, 300.0), viewport, PopoverPlacement::Bottom, gap),
                    Rect::new(12.0, 424.0, 390.0, 300.0),
                );
                assert_eq!(
                    place_popover(anchor, Size::new(240.0, 900.0), viewport, PopoverPlacement::Bottom, gap),
                    Rect::new(87.0, 32.0, 240.0, 692.0),
                );
            }
        }
    }

    #[test]
    fn bottom_popover_measures_wrapped_content_at_full_phone_width() {
        let mut tree = UiTree::new(Element::popover(
            Element::spacer(),
            Element::button("sheet_body", "Actions").size(Some(560.0), Some(120.0)),
            "sheet_close",
            PopoverPlacement::Bottom,
        ));
        tree.set_reduced_motion(true);
        for width in [320.0, 390.0, 430.0] {
            let frame = tree.layout(Size::new(width, 700.0));
            let surface = frame.regions.iter().find(|r| r.id == "popover_surface:sheet_close").unwrap().rect;
            let body = frame.regions.iter().find(|r| r.id == "sheet_body").unwrap().rect;
            assert_eq!(surface, Rect::new(0.0, 580.0, width, 120.0));
            assert_eq!(body, surface);
        }
    }

    #[test]
    fn anchored_popover_flips_and_keeps_its_own_surface_interactive() {
        let trigger = Element::button("trigger", "Abrir").size(Some(100.0), Some(26.0));
        let popup =
            Element::panel(Element::button("option", "Opción").size(Some(100.0), Some(28.0)))
                .padding(8.0)
                .size(Some(130.0), Some(48.0))
                .background(Color::rgb(40, 40, 40));
        let root = Element::column(vec![
            Element::spacer().size(None, Some(145.0)),
            Element::popover(trigger, popup, "dismiss", PopoverPlacement::BottomStart),
        ]);
        let mut tree = UiTree::new(root);
        let frame = tree.layout(Size::new(240.0, 190.0));
        let anchor = frame
            .regions
            .iter()
            .find(|region| region.id == "trigger")
            .unwrap();
        let option = frame
            .regions
            .iter()
            .find(|region| region.id == "option")
            .unwrap();
        assert!(
            option.rect.y < anchor.rect.y,
            "must flip above viewport edge"
        );
        assert_eq!(frame.hit_test(Point::new(228.0, 176.0)), Some("dismiss"));
        assert_eq!(
            frame.hit_test(Point::new(anchor.rect.x + 10.0, anchor.rect.y + 10.0)),
            Some("dismiss"),
            "an open popover's backdrop closes before the anchor can retrigger"
        );
        let popup_surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap();
        assert_eq!(
            frame.hit_test(Point::new(
                popup_surface.rect.x + 3.0,
                popup_surface.rect.y + 3.0
            )),
            Some("popover_surface:dismiss")
        );
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn clickable_session_row_keeps_nested_menu_action_and_escape_focus() {
        let row = |open| {
            let menu = Element::icon_button("more", Icon::More, "").size(Some(24.0), Some(28.0));
            let menu = if open {
                Element::popover(
                    menu,
                    Element::column(vec![
                        Element::button("pin", "Fijar").size(Some(90.0), Some(28.0)),
                        Element::button("archive", "Archivar").size(Some(90.0), Some(28.0)),
                    ])
                    .size(Some(90.0), Some(56.0)),
                    "dismiss",
                    PopoverPlacement::BottomEnd,
                )
            } else {
                menu
            };
            Element::clickable(
                "session",
                Element::row(vec![
                    Element::truncated_text("Tarea").size(Some(130.0), Some(28.0)),
                    menu,
                ])
                .size(Some(160.0), Some(28.0)),
            )
            .size(Some(160.0), Some(28.0))
        };
        let mut tree = UiTree::new(row(false));
        let frame = tree.layout(Size::new(240.0, 180.0));
        assert_eq!(frame.hit_test(Point::new(5.0, 14.0)), Some("session"));
        assert_eq!(frame.hit_test(Point::new(142.0, 14.0)), Some("more"));
        for (point, expected) in [
            (Point::new(5.0, 14.0), "session"),
            (Point::new(142.0, 14.0), "more"),
        ] {
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: expected.into()
                }]
            );
        }
        assert!(tree.focus("more"));
        tree.set_root(row(true));
        tree.layout(Size::new(240.0, 180.0));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        tree.handle_event(key(Key::ArrowDown));
        assert_eq!(tree.focused_id(), Some("pin"));
        tree.set_root(row(false));
        tree.layout(Size::new(240.0, 180.0));
        assert_eq!(tree.focused_id(), Some("more"));
        tree.set_root(row(true));
        tree.layout(Size::new(240.0, 180.0));
        tree.handle_event(key(Key::End));
        assert_eq!(tree.focused_id(), Some("archive"));
        assert_eq!(
            tree.handle_event(key(Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("more"));
        tree.set_root(row(false));
        tree.layout(Size::new(240.0, 180.0));
        assert_eq!(tree.focused_id(), Some("more"));
    }

    #[test]
    fn anchored_popover_escapes_scroll_clip() {
        let trigger = Element::button("trigger", "Abrir").size(Some(80.0), Some(26.0));
        let popup = Element::button("option", "Elegir").size(Some(95.0), Some(30.0));
        let scrolled = Element::scroll(
            "list",
            Element::column(vec![
                Element::spacer().size(None, Some(77.0)),
                Element::popover(trigger, popup, "dismiss", PopoverPlacement::BottomStart),
            ]),
        )
        .size(Some(160.0), Some(105.0));
        let mut tree = UiTree::new(Element::column(vec![scrolled]));
        let frame = tree.layout(Size::new(220.0, 200.0));
        let option = frame
            .regions
            .iter()
            .find(|region| region.id == "option")
            .unwrap();
        assert!(option.rect.y > 105.0, "floating option escapes scroll clip");
        assert_eq!(
            frame.hit_test(Point::new(option.rect.x + 10.0, option.rect.y + 10.0)),
            Some("option")
        );
    }

    #[test]
    fn button_labels_truncate_at_painted_width_without_splitting_graphemes() {
        let mut tree = UiTree::new(Element::column(vec![
            Element::button("plain", "WWii").size(Some(30.0), Some(28.0)),
            Element::icon_button("with_icon", Icon::Folder, "👩‍💻WWii").size(Some(50.0), Some(28.0)),
        ]));
        tree.set_text_measurer(|text, _, _| {
            text.graphemes(true)
                .map(|part| match part {
                    "W" => 10.0,
                    "i" => 2.0,
                    "…" => 4.0,
                    "👩‍💻" => 14.0,
                    _ => 5.0,
                })
                .sum()
        });
        let frame = tree.layout(Size::new(70.0, 70.0));
        let labels: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(labels, vec!["W…", "👩‍💻…"]);
    }

    #[test]
    fn formatted_spans_use_painter_width_across_font_families() {
        let spans = vec![
            TextSpan::new("ii ", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new("W", Color::rgb(250, 100, 100), 14.0, FontFamily::Monospace),
            TextSpan::new("i", Color::WHITE, 14.0, FontFamily::Sans),
        ];
        let mut tree = UiTree::new(Element::formatted_text(spans.clone()).size(Some(9.0), None));
        tree.set_text_measurer(|part, _, family| {
            part.chars()
                .map(|character| match (character, family) {
                    ('W', FontFamily::Monospace) => 5.0,
                    ('i', _) | (' ', _) => 1.0,
                    _ => 2.0,
                })
                .sum()
        });
        let frame = tree.layout(Size::new(9.0, 100.0));
        let runs: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    position,
                    text,
                    font_family,
                    color,
                    ..
                } => Some((position.x, position.y, text.as_str(), *font_family, *color)),
                _ => None,
            })
            .collect();
        assert_eq!(runs.iter().map(|run| run.2).collect::<String>(), "ii Wi");
        assert!(runs.iter().all(|run| run.1 == 0.0));
        assert!(runs.iter().any(|run| run.0 == 3.0
            && run.2 == "W"
            && run.3 == FontFamily::Monospace
            && run.4 == Color::rgb(250, 100, 100)));
        assert!(runs.iter().any(|run| run.0 == 8.0 && run.2 == "i"));

        tree.set_root(Element::formatted_text(spans).size(Some(6.0), None));
        let frame = tree.layout(Size::new(6.0, 100.0));
        let w_y = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == "W" => Some(position.y),
                _ => None,
            })
            .unwrap();
        assert!(
            w_y > 0.0,
            "narrow layout should wrap the measured wide glyph"
        );
    }

    fn inline_picture() -> RasterImage {
        RasterImage::new(
            "b".repeat(64),
            crate::RasterImageFormat::Png,
            4,
            Arc::from("AAAA"),
            "wave",
        )
        .unwrap()
    }

    #[test]
    fn inline_pictures_are_unsplit_squares_painted_in_place_of_their_text() {
        let spans = vec![
            TextSpan::new("hi ", Color::WHITE, 16.0, FontFamily::Sans),
            TextSpan::new(":wave:", Color::WHITE, 16.0, FontFamily::Sans).image(inline_picture()),
            TextSpan::new(" ok", Color::WHITE, 16.0, FontFamily::Sans),
        ];
        let mut tree = UiTree::new(Element::formatted_text(spans.clone()));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
        let frame = tree.layout(Size::new(200.0, 100.0));
        let pictures: Vec<Rect> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RasterImage { rect, image, .. } if image.sha256() == "b".repeat(64) => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(pictures.len(), 1);
        // 1.375 times the 16 px text, after the 12 px of "hi ".
        assert_eq!((pictures[0].x, pictures[0].width, pictures[0].height), (12.0, 22.0, 22.0));
        let painted: String = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(painted, "hi  ok", "the alternative text is not painted");
        let block = &frame.text_layouts()[0];
        assert_eq!(block.source, "hi :wave: ok");
        let after = block.runs.iter().find(|run| run.text == " ok").unwrap();
        assert_eq!(after.rect.x, 34.0, "the text goes on after the square");

        // Too narrow for the square beside "hi ": it moves down whole.
        tree.set_root(Element::formatted_text(spans).size(Some(30.0), None));
        let frame = tree.layout(Size::new(30.0, 100.0));
        let picture = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::RasterImage { rect, .. } => Some(*rect),
                _ => None,
            })
            .unwrap();
        assert_eq!((picture.x, picture.width), (0.0, 22.0));
        assert!(picture.y > 0.0);
    }

    #[test]
    fn an_inline_picture_is_selected_and_copied_whole_as_its_text() {
        let mut tree = UiTree::new(Element::formatted_text(vec![
            TextSpan::new("ab", Color::WHITE, 16.0, FontFamily::Sans),
            TextSpan::new(":wave:", Color::WHITE, 16.0, FontFamily::Sans).image(inline_picture()),
            TextSpan::new("cd", Color::WHITE, 16.0, FontFamily::Sans),
        ]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
        tree.layout(Size::new(200.0, 100.0));
        // From the start of the text to a third into the 22 px square, at 8.
        tree.handle_event(UiEvent::PointerDown(Point::new(0.0, 4.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(15.0, 4.0)));
        assert_eq!(tree.selected_text(), Some("ab"));
        tree.handle_event(UiEvent::PointerMove(Point::new(21.0, 4.0)));
        assert_eq!(tree.selected_text(), Some("ab:wave:"));
        tree.handle_event(UiEvent::PointerUp(Point::new(21.0, 4.0)));
        let frame = tree.layout(Size::new(200.0, 100.0));
        let highlight = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if *color == Color::rgba(180, 180, 180, 84) && rect.x == 8.0 =>
            {
                Some(*rect)
            }
            _ => None,
        });
        assert_eq!(highlight.map(|rect| rect.width), Some(22.0), "the whole square is selected");
    }

    #[test]
    fn an_inline_surface_paints_the_hosts_pixels_in_the_square() {
        let mut tree = UiTree::new(Element::formatted_text(vec![TextSpan::new(
            ":dance:",
            Color::WHITE,
            14.0,
            FontFamily::Sans,
        )
        .surface("animation:dance")
        .action("emoji:dance")]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
        let frame = tree.layout(Size::new(200.0, 100.0));
        let surface = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Surface { rect, id, label, .. } => {
                Some((*rect, id.clone(), label.clone()))
            }
            _ => None,
        });
        let (rect, id, label) = surface.unwrap();
        assert_eq!((rect.width, rect.height), (19.25, 19.25));
        assert_eq!((id.as_str(), label.as_str()), ("animation:dance", ":dance:"));
        assert_eq!(
            frame.hit_test(Point::new(rect.x + 5.0, rect.y + 5.0)),
            Some("emoji:dance")
        );
        assert_eq!(frame.text_layouts()[0].source, ":dance:");
    }

    #[test]
    fn inline_actions_follow_measured_wrapped_fragments_and_preserve_plain_selection() {
        let link = "WW ii WW";
        let mut tree = UiTree::new(Element::formatted_text(vec![
            TextSpan::new("ii ", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new(link, Color::WHITE, 14.0, FontFamily::Sans).action("link"),
            TextSpan::new(" fin", Color::WHITE, 14.0, FontFamily::Sans),
        ]));
        tree.set_text_measurer(|text, _, _| {
            text.chars()
                .map(|ch| if ch == 'W' { 9.0 } else { 2.0 })
                .sum()
        });
        let viewport = Size::new(30.0, 200.0);
        let frame = tree.layout(viewport);
        let regions: Vec<_> = frame
            .regions
            .iter()
            .filter(|r| r.id == "link")
            .cloned()
            .collect();
        assert_eq!(regions.len(), 2, "one target per painted link fragment");
        let runs = &frame.text_layouts()[0].runs;
        let linked_runs: Vec<_> = runs
            .iter()
            .filter(|run| regions.iter().any(|r| r.rect == run.rect))
            .collect();
        assert_eq!(
            linked_runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            link
        );
        assert_eq!(
            linked_runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<Vec<_>>(),
            ["WW ii", " WW"],
            "word and whitespace tokens fill the exact measured 30 px row"
        );
        assert_eq!(frame.hit_test(Point::new(1.0, 2.0)), None);
        for region in &regions {
            let point = Point::new(region.rect.x + 1.0, region.rect.y + 2.0);
            assert_eq!(frame.hit_test(point), Some("link"));
        }
        let point = Point::new(regions[1].rect.x + 1.0, regions[1].rect.y + 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "link".into() }]
        );
        assert_eq!(tree.selected_text(), None);

        // Selection starting in ordinary prose still crosses actionable spans.
        tree.handle_event(UiEvent::PointerDown(Point::new(0.0, 2.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(
            regions[0].rect.x + regions[0].rect.width,
            2.0,
        )));
        assert_eq!(tree.selected_text(), Some("ii WW ii"));
    }

    #[test]
    fn inline_actions_keep_distinct_targets_and_invalidate_cached_destinations() {
        let root = |target: &str| {
            Element::formatted_text(vec![
                TextSpan::new("one", Color::WHITE, 14.0, FontFamily::Sans).action(target),
                TextSpan::new("two", Color::WHITE, 14.0, FontFamily::Sans).action("second"),
            ])
        };
        let mut tree = UiTree::new(root("old"));
        tree.set_text_measurer(|text, _, _| text.len() as f32 * 8.0);
        tree.layout(Size::new(100.0, 40.0));
        tree.set_root(root("new"));
        let frame = tree.layout(Size::new(100.0, 40.0));
        assert_eq!(frame.hit_test(Point::new(4.0, 4.0)), Some("new"));
        assert_eq!(frame.hit_test(Point::new(28.0, 4.0)), Some("second"));
        assert!(!frame.regions.iter().any(|region| region.id == "old"));
        assert_eq!(frame.text_layouts()[0].source, "onetwo");
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("new"));
        for key in [Key::Enter, Key::Space] {
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked { id: "new".into() }]
            );
        }
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("second"));
    }

    #[test]
    fn inline_link_keyboard_focus_reveals_offscreen_paragraph_and_clips_hit_regions() {
        let paragraph = Element::formatted_text(vec![TextSpan::new(
            "wrapped link text",
            Color::WHITE,
            14.0,
            FontFamily::Sans,
        )
        .action("link")]);
        let mut tree = UiTree::new(
            Element::scroll(
                "document",
                Element::column(vec![
                    Element::button("before", "Before").size(None, Some(24.0)),
                    Element::spacer().size(None, Some(180.0)),
                    paragraph,
                    Element::button("after", "After").size(None, Some(24.0)),
                ]),
            )
            .size(None, Some(42.0)),
        );
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.len() as f32 * 8.0);
        let viewport = Size::new(70.0, 42.0);
        assert!(!tree.layout(viewport).regions.iter().any(|r| r.id == "link"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("before"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("link"));
        let frame = tree.layout(viewport);
        let regions: Vec<_> = frame.regions.iter().filter(|r| r.id == "link").collect();
        assert!(
            !regions.is_empty(),
            "Tab reveals the measured first link fragment"
        );
        assert!(regions
            .iter()
            .all(|r| r.rect.y >= 0.0 && r.rect.y + r.rect.height <= 42.0));
        assert!(frame
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DrawCommand::StrokeRect { width, .. } if *width == 2.0)));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(
            tree.focused_id(),
            Some("after"),
            "a wrapped link has only one Tab stop"
        );
    }

    #[test]
    fn formatted_leading_keeps_cache_selection_and_inline_code_geometry_consistent() {
        let fill = Color::rgb(45, 46, 47);
        let paragraph = |height| {
            Element::formatted_text(vec![TextSpan::new(
                "one\ntwo",
                Color::WHITE,
                14.0,
                FontFamily::Sans,
            )
            .background(fill)])
            .font_size(14.0)
            .font_family(FontFamily::Sans)
            .line_height(height)
            .size(Some(160.0), None)
        };
        let mut tree = UiTree::new(paragraph(24.0));
        for height in [24.0, 32.0, 24.0] {
            tree.set_root(paragraph(height));
            let frame = tree.layout(Size::new(160.0, 200.0));
            let block = &frame.text_layouts()[0];
            assert_eq!(block.source, "one\ntwo");
            assert_eq!(block.runs.len(), 2);
            assert_eq!(block.runs[0].rect.height, height);
            assert_eq!(block.runs[1].rect.y - block.runs[0].rect.y, height);
            let painted: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { position, .. } => Some(position.y),
                    _ => None,
                })
                .collect();
            assert_eq!(painted[1] - painted[0], height);
            let backgrounds: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == fill => Some(rect),
                    _ => None,
                })
                .collect();
            assert_eq!(backgrounds.len(), 2);
            assert!(
                backgrounds[0].height < height,
                "code chips must not fill the leading"
            );
            assert!(backgrounds[0].y > block.runs[0].rect.y);
        }
        assert_eq!(paragraph(f32::NAN).style.line_height, None);
        assert_eq!(paragraph(-1.0).style.line_height, None);
    }

    #[test]
    fn formatted_wrap_keeps_graphemes_and_explicit_line_breaks() {
        let source = "a👩‍💻e\u{301}b\n🦀";
        let mut tree = UiTree::new(
            Element::formatted_text(vec![TextSpan::new(
                source,
                Color::WHITE,
                14.0,
                FontFamily::Sans,
            )])
            .size(Some(5.0), None),
        );
        tree.set_text_measurer(|part, _, _| {
            part.graphemes(true)
                .map(|part| if part == "👩‍💻" { 5.0 } else { 1.0 })
                .sum()
        });
        let frame = tree.layout(Size::new(5.0, 100.0));
        let runs: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, .. } => Some((position.y, text.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(
            runs.iter().map(|(_, text)| *text).collect::<String>(),
            source.replace('\n', "")
        );
        assert!(runs.iter().any(|(_, text)| *text == "👩‍💻"));
        assert!(runs.iter().any(|(_, text)| text.contains("e\u{301}")));
        assert!(runs.last().unwrap().0 > runs.first().unwrap().0);
    }

    #[test]
    fn static_selection_crosses_styled_spans_without_splitting_emoji() {
        let spans = vec![
            TextSpan::new("hi ", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new("👩‍💻", Color::rgb(180, 180, 180), 14.0, FontFamily::Sans),
            TextSpan::new(" fin", Color::WHITE, 14.0, FontFamily::Sans),
        ];
        let mut tree = UiTree::new(Element::formatted_text(spans).size(Some(200.0), None));
        tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
        tree.layout(Size::new(200.0, 40.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(43.0, 5.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(43.0, 5.0)));
        assert_eq!(tree.selected_text(), Some("hi 👩‍💻"));
        let frame = tree.layout(Size::new(200.0, 40.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *color == Color::rgba(180, 180, 180, 84)
                    && (rect.x - 30.0).abs() < 0.01
                    && (rect.width - 10.0).abs() < 0.01
        )));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::C,
                shift: false,
                ctrl: true,
            }),
            vec![UiAction::CopyRequested {
                text: "hi 👩‍💻".into(),
            }]
        );
    }

    #[test]
    fn pointer_leave_finishes_static_selection_drag() {
        let mut tree = UiTree::new(Element::text("abcd").size(Some(100.0), None));
        tree.layout(Size::new(100.0, 30.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(18.0, 5.0)));
        let before_leave = tree.selected_text().map(str::to_owned);
        assert!(before_leave.is_some());
        tree.handle_event(UiEvent::PointerLeave);
        assert!(!tree.static_selection.as_ref().unwrap().dragging);
        tree.handle_event(UiEvent::PointerMove(Point::new(80.0, 5.0)));
        assert_eq!(tree.selected_text(), before_leave.as_deref());
    }

    #[test]
    fn pointer_leave_releases_editor_minimap_selection_and_diff_scrollbar_capture() {
        let controller = crate::EditorController::new(
            limn_editor::TextBuffer::new(&"source code\n".repeat(400)).unwrap(),
        );
        for point in [
            Point::new(650.0, 140.0),
            Point::new(698.0, 140.0),
            Point::new(120.0, 24.0),
        ] {
            let mut tree = UiTree::new(
                crate::CodeEditor::new("editor", controller.clone())
                    .minimap(true)
                    .height(300.0)
                    .render(),
            );
            tree.layout(Size::new(700.0, 300.0));
            tree.handle_event(UiEvent::PointerDown(point));
            tree.handle_event(UiEvent::PointerMove(Point::new(point.x, 240.0)));
            let before = tree.code_editors["editor"].viewport.scroll_y;
            let selection = controller.snapshot().unwrap().selection();
            tree.handle_event(UiEvent::PointerLeave);
            assert!(!tree.code_editors["editor"].viewport.dragging);
            assert!(tree.code_editors["editor"]
                .viewport
                .scrollbar_drag
                .is_none());
            tree.handle_event(UiEvent::PointerMove(Point::new(point.x, 5.0)));
            assert_eq!(tree.code_editors["editor"].viewport.scroll_y, before);
            assert_eq!(controller.snapshot().unwrap().selection(), selection);
        }
        let lines = (0..400)
            .map(|row| crate::CodeDiffLine {
                kind: crate::CodeDiffLineKind::Context,
                old_line: Some(row + 1),
                new_line: Some(row + 1),
                text: "source code".into(),
            })
            .collect::<Vec<_>>();
        let mut tree = UiTree::new(crate::CodeDiff::new("diff", lines).height(300.0).render());
        tree.layout(Size::new(700.0, 300.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(698.0, 150.0)));
        assert!(tree.code_diffs["diff"].viewport.scrollbar_drag.is_some());
        let before = tree.code_diffs["diff"].viewport.scroll_y;
        tree.handle_event(UiEvent::PointerLeave);
        tree.handle_event(UiEvent::PointerMove(Point::new(698.0, 5.0)));
        assert!(tree.code_diffs["diff"].viewport.scrollbar_drag.is_none());
        assert_eq!(tree.code_diffs["diff"].viewport.scroll_y, before);
    }

    #[test]
    fn static_selection_preserves_source_newline_and_soft_wraps() {
        let mut tree = UiTree::new(Element::formatted_text(vec![
            TextSpan::new("α\n", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new("β", Color::WHITE, 14.0, FontFamily::Sans),
        ]));
        tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
        tree.layout(Size::new(100.0, 50.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(10.0, 23.0)));
        assert_eq!(tree.selected_text(), Some("α\nβ"));

        tree.set_root(Element::text("abcd").size(Some(20.0), None));
        tree.layout(Size::new(20.0, 50.0));
        assert_eq!(tree.selected_text(), None);
        tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(20.0, 23.0)));
        assert_eq!(tree.selected_text(), Some("abcd"));
    }

    #[test]
    fn selection_does_not_cut_a_grapheme_across_span_boundaries() {
        let mut tree = UiTree::new(Element::formatted_text(vec![
            TextSpan::new("e", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new("\u{301}", Color::rgb(180, 180, 180), 14.0, FontFamily::Sans),
            TextSpan::new("x", Color::WHITE, 14.0, FontFamily::Sans),
        ]));
        tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
        tree.layout(Size::new(100.0, 30.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(20.0, 5.0)));
        assert_eq!(tree.selected_text(), Some("e\u{301}"));
    }

    #[test]
    fn wrapping_prefers_word_boundaries_and_preserves_newlines() {
        let lines = wrap_lines("El agente trabaja en este proyecto.\nSiguiente", 22.0, 1.0);
        assert_eq!(
            lines,
            ["El agente trabaja en ", "este proyecto.", "Siguiente"]
        );
    }

    #[test]
    fn painter_measurement_controls_proportional_wrapping_and_font_size() {
        let root = Element::text("iiii WWWW")
            .font_family(FontFamily::Sans)
            .font_size(28.0);
        let mut tree = UiTree::new(root);
        tree.set_text_measurer(|part, size, _| {
            part.chars()
                .map(|ch| if ch == 'W' { 7.0 } else { 2.0 })
                .sum::<f32>()
                * size
                / 14.0
        });
        let frame = tree.layout(Size::new(40.0, 100.0));
        let lines = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(lines, ["iiii ", "WW", "WW"]);
    }

    #[test]
    fn input_caret_and_selection_follow_painter_advances() {
        let mut tree = UiTree::new(
            Element::text_input("text", "iW", "")
                .font_family(FontFamily::Sans)
                .padding(0.0)
                .size(Some(120.0), Some(30.0)),
        );
        tree.set_text_measurer(|part, _, _| {
            part.chars()
                .map(|ch| if ch == 'W' { 20.0 } else { 2.0 })
                .sum()
        });
        tree.layout(Size::new(120.0, 30.0));
        assert!(tree.focus("text"));
        let focused = tree.layout(Size::new(120.0, 30.0));
        assert!(focused.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, .. } if (rect.x - 22.0).abs() < 0.01 && rect.width == 1.5
        )));
        tree.handle_event(UiEvent::PointerDown(Point::new(4.0, 4.0)));
        let clicked = tree.layout(Size::new(120.0, 30.0));
        assert!(clicked.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, .. } if (rect.x - 2.0).abs() < 0.01 && rect.width == 1.5
        )));
        tree.handle_event(UiEvent::Key {
            key: Key::A,
            shift: false,
            ctrl: true,
        });
        let selected = tree.layout(Size::new(120.0, 30.0));
        assert!(selected.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *color == Color::rgba(75, 130, 210, 130)
                    && (rect.width - 22.0).abs() < 0.01
        )));
    }

    #[test]
    fn text_area_uses_measured_wraps_for_paint_and_vertical_navigation() {
        let mut tree = UiTree::new(
            Element::text_area("area", "iWWW", "")
                .font_family(FontFamily::Sans)
                .padding(0.0)
                .size(Some(20.0), Some(60.0)),
        );
        tree.set_text_measurer(|part, _, _| {
            part.chars()
                .map(|ch| if ch == 'W' { 10.0 } else { 2.0 })
                .sum()
        });
        let frame = tree.layout(Size::new(20.0, 60.0));
        let lines = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(lines, ["iW", "WW"]);
        assert!(tree.focus("area"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowLeft,
            shift: false,
            ctrl: false,
        });
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowUp,
            shift: false,
            ctrl: false,
        });
        let moved = tree.layout(Size::new(20.0, 60.0));
        assert!(moved.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, .. }
                if (rect.x - 2.0).abs() < 0.01 && rect.y == 0.0 && rect.width == 1.5
        )));
    }

    #[test]
    fn flex_layout_distributes_remaining_height() {
        let root = Element::column(vec![
            Element::text("header").size(None, Some(20.0)),
            Element::panel(Element::text("body")).flex(1.0),
            Element::text("footer").size(None, Some(10.0)),
        ]);
        let frame = UiTree::new(root).layout(Size::new(300.0, 100.0));
        let body = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { position, text, .. } if text == "body" => Some(position.y),
            _ => None,
        });
        let footer = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { position, text, .. } if text == "footer" => Some(position.y),
            _ => None,
        });
        assert_eq!(body, Some(20.0));
        assert_eq!(footer, Some(90.0));
    }

    #[test]
    fn typography_and_border_affect_scene_without_changing_hit_bounds() {
        let accent = Color::rgb(34, 145, 210);
        let root = Element::column(vec![
            Element::text("Title")
                .font_family(FontFamily::Sans)
                .font_size(28.0),
            Element::button("open", "Open")
                .background(Color::rgb(20, 20, 20))
                .border(accent, 1.0)
                .radius(8.0)
                .size(None, Some(32.0)),
        ]);
        let mut tree = UiTree::new(root);
        tree.set_sans_metrics(FontMetrics {
            char_width: 9.0,
            line_height: 20.0,
        });
        let frame = tree.layout(Size::new(200.0, 100.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text {
                text,
                position,
                font_size: 28.0,
                font_family: FontFamily::Sans,
                ..
            } if text == "Title" && position.y == 0.0
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect {
                color,
                width: 1.0,
                radius: 8.0,
                ..
            } if *color == accent
        )));
        // The title's calibrated line height is 40 at 28px.
        assert_eq!(frame.hit_test(Point::new(5.0, 41.0)), Some("open"));
        assert_eq!(frame.hit_test(Point::new(5.0, 39.0)), None);
    }

    #[test]
    fn shadow_precedes_surface_and_does_not_expand_hit_bounds() {
        let ink = Color::rgba(0, 0, 0, 90);
        let surface = Color::rgb(35, 35, 35);
        let frame = UiTree::new(Element::column(vec![Element::button("card", "Card")
            .background(surface)
            .shadow(ink, Point::new(0.0, 5.0), 16.0, 2.0)
            .radius(12.0)
            .size(Some(120.0), Some(42.0))]))
        .layout(Size::new(200.0, 80.0));
        assert!(matches!(
            frame.commands.first(),
            Some(DrawCommand::Shadow {
                rect: Rect { width: 120.0, height: 42.0, .. },
                shadow: BoxShadow { color, blur: 16.0, spread: 2.0, .. },
                radius: 12.0,
            }) if *color == ink
        ));
        assert!(matches!(
            frame.commands.get(1),
            Some(DrawCommand::FillRect { color, .. }) if *color == surface
        ));
        assert_eq!(frame.hit_test(Point::new(5.0, 40.0)), Some("card"));
        assert_eq!(frame.hit_test(Point::new(5.0, 45.0)), None);
    }

    #[test]
    fn invalid_shadow_measurements_are_sanitized() {
        let element = Element::panel(Element::spacer()).shadow(
            Color::rgba(0, 0, 0, 80),
            Point::new(f32::NAN, f32::INFINITY),
            -5.0,
            f32::INFINITY,
        );
        assert_eq!(
            element.style.shadow,
            Some(BoxShadow::new(
                Color::rgba(0, 0, 0, 80),
                Point::new(0.0, 0.0),
                0.0,
                0.0,
            ))
        );
    }

    #[test]
    fn overlay_paints_after_base_and_owns_topmost_hit() {
        let frame = UiTree::new(Element::overlay(
            Element::button("base", "base").size(None, Some(40.0)),
            Element::button("floating", "floating").size(None, Some(40.0)),
        ))
        .layout(Size::new(120.0, 40.0));
        let labels = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, ["base", "floating"]);
        assert_eq!(frame.hit_test(Point::new(8.0, 8.0)), Some("floating"));
    }

    #[test]
    fn overlay_escapes_scroll_clip_but_stays_inside_viewport() {
        let scroll_contents = Element::column(vec![
            Element::spacer().size(None, Some(35.0)),
            Element::overlay(
                Element::button("anchor", "anchor").size(None, Some(50.0)),
                Element::button("popup", "popup")
                    .background(Color::rgb(40, 40, 40))
                    .size(None, Some(50.0)),
            ),
        ]);
        let root = Element::column(vec![
            Element::scroll("list", scroll_contents).size(None, Some(40.0)),
            Element::spacer().flex(1.0),
        ]);
        let frame = UiTree::new(root).layout(Size::new(120.0, 100.0));
        let pop_clip = frame
            .commands
            .iter()
            .position(|command| matches!(command, DrawCommand::PopClip))
            .unwrap();
        let popup = frame
            .commands
            .iter()
            .position(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "popup"),
            )
            .unwrap();
        assert!(popup > pop_clip);
        assert_eq!(frame.hit_test(Point::new(8.0, 60.0)), Some("popup"));
        assert_eq!(frame.hit_test(Point::new(8.0, 98.0)), None);
    }

    #[test]
    fn icon_button_uses_one_theme_tint_and_keeps_button_interaction() {
        let tint = Color::rgb(219, 234, 254);
        let mut tree = UiTree::new(
            Element::icon_button("new", Icon::Add, "Nuevo")
                .icon_size(20.0)
                .foreground(tint)
                .size(Some(100.0), Some(36.0)),
        );
        let frame = tree.layout(Size::new(100.0, 36.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                rect: Rect {
                    width: 16.0,
                    height: 16.0,
                    ..
                },
                ..
            }
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                icon: Icon::Add,
                tint: actual,
                ..
            } if *actual == tint
        )));
        assert_eq!(frame.hit_test(Point::new(12.0, 12.0)), Some("new"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked { id: "new".into() }]
        );
    }

    #[test]
    fn trailing_icon_shares_one_click_target_and_leaves_room_for_label() {
        let mut tree = UiTree::new(
            Element::icon_button("backend", Icon::Chat, "Codex agent")
                .trailing_icon(Icon::ArrowDown)
                .size(Some(120.0), Some(32.0)),
        );
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(Size::new(120.0, 32.0));
        let icons: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { icon, rect, .. } => Some((*icon, *rect)),
                _ => None,
            })
            .collect();
        assert_eq!(icons.len(), 2);
        assert_eq!(icons[0].0, Icon::Chat);
        assert_eq!(icons[1].0, Icon::ArrowDown);
        assert!(icons[0].1.x + icons[0].1.width < icons[1].1.x);
        assert_eq!(frame.hit_test(Point::new(110.0, 16.0)), Some("backend"));
        let label = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        });
        assert!(label.unwrap().ends_with('…'));
    }

    #[test]
    fn icon_only_button_fits_canonical_icon_in_compact_target() {
        let mut tree = UiTree::new(
            Element::icon_button("close", Icon::Close, "").size(Some(24.0), Some(24.0)),
        );
        let frame = tree.layout(Size::new(24.0, 24.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon {
                rect: Rect {
                    width: 16.0,
                    height: 16.0,
                    ..
                },
                ..
            }
        )));
    }

    #[test]
    fn icon_only_glyph_and_hover_stay_centered_in_square_hit_target() {
        let hover = Color::rgb(41, 44, 47);
        let pressed = Color::rgb(57, 60, 63);
        for size in [24.0, 26.0, 28.0, 32.0, 36.0] {
            let control = Element::icon_button("toolbar", Icon::Settings, "")
                .padding(4.0)
                .background(Color::TRANSPARENT)
                .hover_background(hover)
                .pressed_background(pressed)
                .transition_ms(0)
                .size(Some(size), Some(size));
            let mut tree = UiTree::new(control);
            let viewport = Size::new(size, size);
            let base = tree.layout(viewport);
            let target = base
                .regions
                .iter()
                .find(|region| region.id == "toolbar")
                .expect("icon-only hit region")
                .rect;
            assert_eq!(target, Rect::new(0.0, 0.0, size, size));
            let glyph = base
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon {
                        icon: Icon::Settings,
                        rect,
                        ..
                    } => Some(*rect),
                    _ => None,
                })
                .expect("icon glyph");
            assert_eq!(
                glyph,
                Rect::new((size - 16.0) / 2.0, (size - 16.0) / 2.0, 16.0, 16.0)
            );
            tree.handle_event(UiEvent::PointerMove(Point::new(size / 2.0, size / 2.0)));
            let hovered = tree.layout(viewport);
            assert_eq!(hovered.regions, base.regions);
            assert!(hovered.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { rect, color, .. } if *rect == target && *color == hover
            )));
            tree.handle_event(UiEvent::PointerDown(Point::new(size / 2.0, size / 2.0)));
            let pressed_frame = tree.layout(viewport);
            assert_eq!(pressed_frame.regions, base.regions);
            assert!(pressed_frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { rect, color, .. } if *rect == target && *color == pressed
            )));
        }
    }

    #[test]
    fn text_area_edits_lines_and_submits_only_on_ctrl_enter() {
        let area = || Element::text_area("prompt", "stale", "Ask").size(None, Some(88.0));
        let mut tree = UiTree::new(area());
        tree.layout(Size::new(240.0, 88.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
        tree.set_input_value("prompt", "");
        tree.handle_event(UiEvent::Text("uno".into()));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::InputChanged {
                id: "prompt".into(),
                value: "uno\n".into(),
            }]
        );
        tree.handle_event(UiEvent::Text("dos".into()));
        tree.set_root(area());
        tree.layout(Size::new(240.0, 88.0));
        assert_eq!(tree.input_value("prompt"), Some("uno\ndos"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: true,
            }),
            vec![UiAction::Submitted {
                id: "prompt".into(),
                value: "uno\ndos".into(),
            }]
        );
        assert_eq!(tree.input_value("prompt"), Some("uno\ndos"));
    }

    #[test]
    fn button_uses_hover_and_pressed_fill_without_changing_hit_region() {
        let base = Color::rgb(20, 20, 20);
        let hover = Color::rgb(40, 40, 40);
        let pressed = Color::rgb(60, 60, 60);
        let button = Element::icon_button("action", Icon::Add, "Action")
            .background(base)
            .hover_background(hover)
            .pressed_background(pressed)
            .transition_ms(0)
            .size(Some(120.0), Some(32.0));
        let mut tree = UiTree::new(button);
        tree.layout(Size::new(120.0, 32.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
        let frame = tree.layout(Size::new(120.0, 32.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == hover
        )));
        tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
        let frame = tree.layout(Size::new(120.0, 32.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == pressed
        )));
        assert_eq!(frame.hit_test(Point::new(10.0, 10.0)), Some("action"));
    }

    #[test]
    fn pointer_focus_does_not_draw_keyboard_ring() {
        let mut tree =
            UiTree::new(Element::button("action", "Action").size(Some(100.0), Some(28.0)));
        tree.layout(Size::new(100.0, 28.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
        assert_eq!(tree.focused_id(), Some("action"));
        assert!(!tree
            .layout(Size::new(100.0, 28.0))
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));

        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert!(tree
            .layout(Size::new(100.0, 28.0))
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
        // Pointer motion changes hover, but it is not a focus change.
        tree.handle_event(UiEvent::PointerMove(Point::new(101.0, 5.0)));
        assert!(tree
            .layout(Size::new(100.0, 28.0))
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
    }

    #[test]
    fn stationary_pointer_retargets_hover_after_tree_rebuild() {
        let left_hover = Color::rgb(90, 10, 10);
        let right_hover = Color::rgb(10, 90, 10);
        let row = |first: bool| {
            let left = Element::button("left", "Left")
                .hover_background(left_hover)
                .transition_ms(0)
                .size(Some(50.0), Some(28.0));
            let right = Element::button("right", "Right")
                .hover_background(right_hover)
                .transition_ms(0)
                .size(Some(50.0), Some(28.0));
            Element::row(if first {
                vec![left, right]
            } else {
                vec![right, left]
            })
        };
        let mut tree = UiTree::new(row(true));
        tree.layout(Size::new(100.0, 28.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
        assert_eq!(tree.hovered.as_deref(), Some("left"));

        tree.set_root(row(false));
        let frame = tree.layout(Size::new(100.0, 28.0));
        assert_eq!(tree.hovered.as_deref(), Some("right"));
        assert!(
            frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, rect, .. }
                    if *color == right_hover && rect.x == 0.0
            )),
            "commands={:?}; regions={:?}; hovered={:?}",
            frame.commands,
            frame.regions,
            tree.hovered
        );
        assert!(!frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == left_hover
        )));
    }

    #[test]
    fn pointer_leave_clears_hover_and_nested_icon_has_priority() {
        let row_hover = Color::rgb(50, 50, 50);
        let icon_hover = Color::rgb(90, 90, 90);
        let row = Element::clickable(
            "row",
            Element::row(vec![
                Element::spacer().size(Some(72.0), Some(28.0)),
                Element::icon_button("more", Icon::More, "")
                    .hover_background(icon_hover)
                    .transition_ms(0)
                    .size(Some(28.0), Some(28.0)),
            ]),
        )
        .hover_background(row_hover)
        .transition_ms(0)
        .size(Some(100.0), Some(28.0));
        let mut tree = UiTree::new(row);
        tree.layout(Size::new(100.0, 28.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(86.0, 10.0)));
        let frame = tree.layout(Size::new(100.0, 28.0));
        assert_eq!(tree.hovered.as_deref(), Some("more"));
        assert!(
            frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, .. } if *color == icon_hover
            )),
            "commands={:?}; regions={:?}; hovered={:?}",
            frame.commands,
            frame.regions,
            tree.hovered
        );
        assert!(!frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == row_hover
        )));

        tree.handle_event(UiEvent::PointerLeave);
        let frame = tree.layout(Size::new(100.0, 28.0));
        assert_eq!(tree.hovered, None);
        assert!(!frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == icon_hover
        )));
    }

    #[test]
    fn nested_action_can_share_parent_hover_surface_without_sharing_click_target() {
        let row_hover = Color::rgb(50, 50, 50);
        let row = Element::clickable(
            "row",
            Element::row(vec![
                Element::spacer().size(Some(72.0), Some(28.0)),
                Element::icon_button("more", Icon::More, "")
                    .background(Color::TRANSPARENT)
                    .hover_background(Color::TRANSPARENT)
                    .transition_ms(0)
                    .size(Some(28.0), Some(28.0)),
            ]),
        )
        .hover_with("more")
        .hover_background(row_hover)
        .transition_ms(0)
        .size(Some(100.0), Some(28.0));
        let mut tree = UiTree::new(row);
        tree.layout(Size::new(100.0, 28.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(86.0, 10.0)));
        let frame = tree.layout(Size::new(100.0, 28.0));
        assert_eq!(frame.hit_test(Point::new(86.0, 10.0)), Some("more"));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, rect, .. }
                if *color == row_hover && rect.width == 100.0
        )));
        tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 10.0)));
        let frame = tree.layout(Size::new(100.0, 28.0));
        assert_eq!(frame.hit_test(Point::new(20.0, 10.0)), Some("row"));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, rect, .. }
                if *color == row_hover && rect.width == 100.0
        )));
    }

    #[test]
    fn a_shared_surface_stays_lit_in_the_gap_between_its_actions() {
        let hover = Color::rgb(70, 70, 70);
        let row = Element::row(vec![
            Element::clickable("open", Element::spacer()).size(Some(60.0), Some(28.0)),
            Element::spacer().size(Some(10.0), None),
            Element::clickable("more", Element::spacer()).size(Some(24.0), Some(24.0)),
        ])
        .size(Some(100.0), Some(28.0))
        .hover_surface("row-surface", ["open", "more"])
        .hover_background(hover);
        let mut tree = UiTree::new(Element::column(vec![row, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let viewport = Size::new(100.0, 80.0);
        tree.layout(viewport);
        let lit = |tree: &mut UiTree, point: Point| {
            tree.handle_event(UiEvent::PointerMove(point));
            tree.layout(viewport).commands.iter().any(
                |command| matches!(command, DrawCommand::FillRect { color, .. } if *color == hover),
            )
        };
        // In the 10 px gap no control is under the pointer; the band holds.
        assert_eq!(tree.layout(viewport).hit_test(Point::new(65.0, 14.0)), None);
        assert!(lit(&mut tree, Point::new(65.0, 14.0)));
        // Off the row it goes dark.
        assert!(!lit(&mut tree, Point::new(65.0, 60.0)));
    }

    #[test]
    fn a_control_hovered_on_a_lit_shared_surface_stands_out_from_it() {
        let hover = Color::rgb(70, 70, 70);
        let build = || {
            Element::row(vec![
                Element::clickable("open", Element::spacer())
                    .hover_background(Color::TRANSPARENT)
                    .size(Some(72.0), Some(28.0)),
                Element::clickable("more", Element::spacer())
                    .hover_background(hover)
                    .foreground(Color::rgb(230, 230, 230))
                    .size(Some(24.0), Some(24.0)),
            ])
            .size(Some(100.0), Some(28.0))
            .hover_surface("row-surface", ["open", "more"])
            .hover_background(hover)
        };
        let fills = |point: Point| {
            let mut tree = UiTree::new(build());
            tree.set_reduced_motion(true);
            let viewport = Size::new(100.0, 28.0);
            tree.layout(viewport);
            tree.handle_event(UiEvent::PointerMove(point));
            let frame = tree.layout(viewport);
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if color.a > 0 => Some((rect.width, *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Over the main part: the surface alone is lit.
        assert_eq!(fills(Point::new(20.0, 14.0)), vec![(100.0, hover)]);
        // Over the control: the surface stays lit and the control is lighter.
        let over = fills(Point::new(84.0, 12.0));
        assert_eq!(over[0], (100.0, hover));
        let (width, lifted) = over[1];
        assert_eq!(width, 24.0);
        assert!(lifted.r > hover.r && lifted.g > hover.g && lifted.b > hover.b, "{lifted:?}");
    }

    #[test]
    fn passive_hover_surface_keeps_one_transition_across_independent_actions() {
        let base = Color::rgb(20, 20, 20);
        let hover = Color::rgb(70, 70, 70);
        let pressed = Color::rgb(100, 100, 100);
        let row = Element::row(vec![
            Element::clickable("open", Element::spacer()).size(Some(72.0), Some(28.0)),
            Element::clickable("stage", Element::spacer()).size(Some(28.0), Some(28.0)),
        ])
        .size(Some(100.0), Some(28.0))
        .hover_surface("row-surface", ["open", "stage"])
        .background(base)
        .hover_background(hover)
        .pressed_background(pressed)
        .transition_ms(120);
        let mut tree = UiTree::new(row);
        let viewport = Size::new(100.0, 28.0);
        let now = Instant::now();
        let initial = tree.layout_at(viewport, now);
        let surface_color = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *rect == Rect::new(0.0, 0.0, 100.0, 28.0) && *radius == 0.0 => {
                        Some(*color)
                    }
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(surface_color(&initial), base);
        assert!(!tree.focus("row-surface"));
        tree.handle_event_at(UiEvent::PointerMove(Point::new(10.0, 14.0)), now);
        tree.layout_at(viewport, now);
        let halfway = now + Duration::from_millis(60);
        let middle = tree.layout_at(viewport, halfway);
        let color = surface_color(&middle);
        assert!(color.r > base.r && color.r < hover.r);
        let stage = Point::new(86.0, 14.0);
        tree.handle_event_at(UiEvent::PointerMove(stage), halfway);
        let switched = tree.layout_at(viewport, halfway);
        assert_eq!(surface_color(&switched), color);
        assert_eq!(switched.regions, initial.regions);
        let end = now + Duration::from_millis(120);
        assert_eq!(surface_color(&tree.layout_at(viewport, end)), hover);
        tree.set_reduced_motion(true);
        tree.handle_event_at(UiEvent::PointerDown(stage), end);
        assert_eq!(surface_color(&tree.layout_at(viewport, end)), pressed);
        assert_eq!(
            tree.handle_event_at(UiEvent::PointerUp(stage), end),
            vec![UiAction::Clicked { id: "stage".into() }]
        );
        tree.handle_event_at(UiEvent::PointerLeave, end);
        assert_eq!(surface_color(&tree.layout_at(viewport, end)), base);
    }

    #[test]
    fn hover_fill_animates_without_moving_the_button() {
        let start = Instant::now();
        let base = Color::rgb(20, 20, 20);
        let hover = Color::rgb(80, 80, 80);
        let button = Element::button("action", "Action")
            .background(base)
            .hover_background(hover)
            .size(Some(120.0), Some(32.0));
        let mut tree = UiTree::new(button);
        tree.layout_at(Size::new(120.0, 32.0), start);
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));

        let first = tree.layout_at(Size::new(120.0, 32.0), start);
        assert!(first.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == base
        )));
        let middle = tree.layout_at(Size::new(120.0, 32.0), start + Duration::from_millis(60));
        assert!(middle.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if color.r > base.r && color.r < hover.r
        )));
        let end = tree.layout_at(Size::new(120.0, 32.0), start + Duration::from_millis(120));
        assert!(end.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == hover
        )));
        assert_eq!(first.regions, end.regions);

        tree.set_reduced_motion(true);
        tree.handle_event(UiEvent::PointerMove(Point::new(121.0, 10.0)));
        let reduced = tree.layout_at(Size::new(120.0, 32.0), start);
        assert!(reduced.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == base
        )));
    }

    #[test]
    fn focus_by_id_accepts_text_and_preserves_focus_on_unknown_id() {
        let mut tree = UiTree::new(Element::text_input("query", "foo", "Search"));
        tree.layout(Size::new(240.0, 38.0));
        assert!(tree.focus("query"));
        assert!(!tree.focus("missing"));
        assert_eq!(tree.focused_id(), Some("query"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("bar".into())),
            vec![UiAction::InputChanged {
                id: "query".into(),
                value: "foobar".into(),
            }]
        );
    }

    #[test]
    fn selecting_all_in_an_empty_input_preserves_the_first_typed_character() {
        for multiline in [false, true] {
            let field = |value: &str| {
                if multiline {
                    Element::text_area("query", value, "Search")
                } else {
                    crate::SearchField::new(
                        "query",
                        value,
                        "Search",
                        crate::ComponentPalette::neutral_dark(),
                    )
                    .render()
                }
            };
            let viewport = Size::new(320.0, 88.0);
            let mut tree = UiTree::new(field(""));
            let frame = tree.layout(viewport);
            let rect = frame
                .regions
                .iter()
                .find(|region| region.id == "query")
                .unwrap()
                .rect;
            let point = Point::new(rect.x + 16.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            tree.handle_event(UiEvent::PointerUp(point));
            assert_eq!(tree.focused_id(), Some("query"));
            tree.handle_event(UiEvent::Key {
                key: Key::A,
                shift: false,
                ctrl: true,
            });
            let mut expected = String::new();
            for character in "fixture ñ🦀".chars() {
                expected.push(character);
                assert_eq!(
                    tree.handle_event(UiEvent::Text(character.to_string())),
                    vec![UiAction::InputChanged {
                        id: "query".into(),
                        value: expected.clone(),
                    }],
                    "every native Text event must append, including the first two"
                );
                tree.set_root(field(&expected));
                tree.layout(viewport);
                assert_eq!(tree.input_value("query"), Some(expected.as_str()));
            }
            // Replacing a nonempty selection remains a single replacement;
            // subsequent native character events then append normally.
            tree.handle_event(UiEvent::Key {
                key: Key::A,
                shift: false,
                ctrl: true,
            });
            tree.handle_event(UiEvent::Text("n".into()));
            tree.handle_event(UiEvent::Text("a".into()));
            assert_eq!(tree.input_value("query"), Some("na"));
        }
    }

    #[test]
    fn typing_after_a_zero_length_shift_selection_does_not_select_inserted_text() {
        let mut tree = UiTree::new(Element::text_input("query", "", "Search"));
        tree.layout(Size::new(240.0, 38.0));
        assert!(tree.focus("query"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowLeft,
            shift: true,
            ctrl: false,
        });
        tree.handle_event(UiEvent::Paste("ñ".into()));
        tree.handle_event(UiEvent::Text("🦀".into()));
        assert_eq!(tree.input_value("query"), Some("ñ🦀"));
        assert!(
            tree.handle_event(UiEvent::Key {
                key: Key::C,
                shift: false,
                ctrl: true,
            })
            .is_empty(),
            "typing must leave no active selection"
        );
    }

    #[test]
    fn text_area_selection_replaces_line_breaks_and_moves_vertically() {
        let mut tree = UiTree::new(Element::text_area("prompt", "one\ntwo", ""));
        tree.layout(Size::new(240.0, 88.0));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        tree.handle_event(UiEvent::Key {
            key: Key::A,
            shift: false,
            ctrl: true,
        });
        let selected = tree.layout(Size::new(240.0, 88.0));
        assert_eq!(
            selected
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == Color::rgba(75, 130, 210, 130)))
                .count(),
            2
        );
        tree.handle_event(UiEvent::Text("x\ny".into()));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowUp,
            shift: true,
            ctrl: false,
        });
        tree.handle_event(UiEvent::Text("!".into()));
        assert_eq!(tree.input_value("prompt"), Some("x!"));
    }

    #[test]
    fn text_area_wheel_scrolls_wrapped_lines_inside_its_clip() {
        let mut tree =
            UiTree::new(Element::text_area("prompt", "abcdefghi", "").size(Some(40.0), Some(52.0)));
        let bottom = tree.layout(Size::new(40.0, 52.0));
        assert!(bottom
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "ghi")));
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: 18.0,
        });
        let top = tree.layout(Size::new(40.0, 52.0));
        assert!(top
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "abc")));
        assert!(!top
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "ghi")));
    }

    #[test]
    fn hit_testing_respects_scroll_clipping() {
        let child = Element::column(vec![
            Element::button("one", "one").size(None, Some(30.0)),
            Element::button("two", "two").size(None, Some(30.0)),
        ]);
        let mut tree = UiTree::new(Element::scroll("list", child));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(100.0, 30.0));
        assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("one"));
        assert_eq!(frame.hit_test(Point::new(5.0, 31.0)), None);
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(5.0, 5.0),
            delta_y: -30.0,
        });
        let frame = tree.layout(Size::new(100.0, 30.0));
        assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("two"));
    }

    #[test]
    fn fixed_width_column_in_row_keeps_wrapped_action_scrollable() {
        let copy = "A long description with enough words to wrap into several lines inside a narrow column";
        let content = Element::row(vec![
            Element::spacer().flex(1.0),
            Element::column(vec![
                Element::text(copy),
                Element::button("last_action", "Open").size(None, Some(24.0)),
            ])
            .size(Some(100.0), None),
            Element::spacer().flex(1.0),
        ]);
        let mut tree = UiTree::new(Element::scroll("list", content));
        let initial = tree.layout(Size::new(240.0, 56.0));
        assert!(!initial
            .regions
            .iter()
            .any(|region| region.id == "last_action"));
        assert!(initial.scroll_extents["list"] > 0.0);

        tree.scroll_to_end("list");
        let bottom = tree.layout(Size::new(240.0, 56.0));
        assert!(bottom
            .regions
            .iter()
            .any(|region| region.id == "last_action"));
    }

    fn scroll_rows(count: usize) -> Element {
        Element::scroll(
            "list",
            Element::column(
                (0..count)
                    .map(|index| {
                        Element::button(format!("row:{index}"), "Row").size(None, Some(20.0))
                    })
                    .collect(),
            ),
        )
    }

    fn rect_center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn assert_offset(tree: &UiTree, id: &str, expected: f32) {
        assert!(
            (tree.scroll_offset(id) - expected).abs() < 0.01,
            "{id}: expected {expected}, got {}",
            tree.scroll_offset(id)
        );
    }

    #[test]
    fn scroll_buttons_share_one_rail_target_and_stop_at_content_bounds() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        for root in [
            scroll_rows(10),
            Element::uniform_list("list", 10, 20.0, |index| {
                Element::button(format!("row:{index}"), "Row")
            }),
        ] {
            let mut tree = UiTree::new(root);
            tree.set_reduced_motion(true);
            let frame = tree.layout_at(viewport, start);
            let bar = frame.scrollbars["list"];
            let up = rect_center(bar.up_button.expect("default up button"));
            let down = rect_center(bar.down_button.expect("default down button"));
            assert_eq!(bar.rail.width, 14.0);
            for row in frame
                .regions
                .iter()
                .filter(|hit| hit.kind == HitKind::Button)
            {
                assert!(
                    row.rect.x + row.rect.width <= bar.rail.x,
                    "default row hover and hit geometry must clear the rail"
                );
            }
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .filter(|hit| hit.kind == HitKind::Scrollbar)
                    .count(),
                1
            );
            for point in [up, down, rect_center(bar.thumb)] {
                assert_eq!(
                    frame.hit_region(point).map(|hit| (&*hit.id, hit.kind)),
                    Some(("list", HitKind::Scrollbar))
                );
            }
            tree.handle_event_at(UiEvent::PointerDown(up), start);
            tree.handle_event_at(UiEvent::PointerUp(up), start);
            assert_offset(&tree, "list", 0.0);
            tree.handle_event_at(UiEvent::PointerDown(down), start);
            tree.handle_event_at(UiEvent::PointerUp(down), start);
            assert_offset(&tree, "list", 48.0);
            tree.layout_at(viewport, start);
            tree.handle_event_at(UiEvent::PointerDown(up), start);
            tree.handle_event_at(UiEvent::PointerUp(up), start);
            assert_offset(&tree, "list", 0.0);
            tree.scroll_to_end("list");
            let frame = tree.layout_at(viewport, start);
            assert_offset(&tree, "list", 100.0);
            let down = rect_center(frame.scrollbars["list"].down_button.unwrap());
            tree.handle_event_at(UiEvent::PointerDown(down), start);
            tree.layout_at(viewport, start + Duration::from_secs(1));
            assert_offset(&tree, "list", 100.0);
            tree.handle_event_at(UiEvent::PointerUp(down), start + Duration::from_secs(1));
            assert!(
                !tree.is_animating(),
                "disabled arrows do not keep requesting repaint"
            );
        }
    }

    #[test]
    fn scrollbar_buttons_and_thumb_stay_bounded_in_tiny_viewports() {
        for uniform in [false, true] {
            for height in [0.25, 0.5, 1.0, 12.0, 24.0, 40.0] {
                let root = if uniform {
                    Element::uniform_list("list", 10, 20.0, |_| Element::text("Row"))
                } else {
                    scroll_rows(10)
                };
                let mut tree = UiTree::new(root);
                let frame = tree.layout(Size::new(12.0, height));
                let bar = frame.scrollbars.get("list").expect("overflow has a rail");
                for rect in [
                    Some(bar.rail),
                    Some(bar.track),
                    Some(bar.thumb),
                    bar.up_button,
                    bar.down_button,
                ]
                .into_iter()
                .flatten()
                {
                    assert!(rect.width > 0.0 && rect.height > 0.0);
                    assert!(rect.x >= 0.0 && rect.y >= 0.0);
                    assert!(rect.x + rect.width <= 12.0 + 0.001);
                    assert!(rect.y + rect.height <= height + 0.001);
                }
                if let Some(up) = bar.up_button {
                    assert!(up.y + up.height <= bar.track.y + 0.001);
                }
                if let Some(down) = bar.down_button {
                    assert!(bar.track.y + bar.track.height <= down.y + 0.001);
                }
            }
        }
        let mut tree = UiTree::new(scroll_rows(1));
        assert!(!tree
            .layout(Size::new(100.0, 100.0))
            .scrollbars
            .contains_key("list"));
    }

    #[test]
    fn wheel_motion_accumulates_and_reverses_without_display_jumps_across_rebuilds() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(scroll_rows(25));
        tree.layout_at(viewport, start);
        let wheel = |delta_y| UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y,
        };
        tree.handle_event_at(wheel(-100.0), start);
        assert_offset(&tree, "list", 0.0);
        assert!(tree.is_animating());
        tree.layout_at(viewport, start + Duration::from_millis(70));
        assert_offset(&tree, "list", 87.5);
        tree.handle_event_at(wheel(-40.0), start + Duration::from_millis(70));
        assert_offset(&tree, "list", 87.5);
        tree.set_root(scroll_rows(25));
        tree.layout_at(viewport, start + Duration::from_millis(70));
        assert_offset(&tree, "list", 87.5);
        tree.layout_at(viewport, start + Duration::from_millis(140));
        assert_offset(&tree, "list", 133.4375);
        tree.handle_event_at(wheel(100.0), start + Duration::from_millis(140));
        assert_offset(&tree, "list", 133.4375);
        tree.layout_at(viewport, start + Duration::from_millis(210));
        assert_offset(&tree, "list", 51.679_688);
        tree.layout_at(viewport, start + Duration::from_millis(280));
        assert_offset(&tree, "list", 40.0);
        assert!(!tree.is_animating());
    }

    #[test]
    fn direct_scroll_cancels_wheel_motion_and_uses_displayed_position() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(scroll_rows(25));
        tree.layout_at(viewport, start);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: -100.0,
            },
            start,
        );
        tree.layout_at(viewport, start + Duration::from_millis(70));
        tree.handle_event_at(
            UiEvent::ScrollDirect {
                position: Point::new(10.0, 10.0),
                delta_y: 20.0,
            },
            start + Duration::from_millis(70),
        );
        assert_offset(&tree, "list", 67.5);
        tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 67.5);
        assert!(!tree.is_animating());
    }

    #[test]
    fn nested_smooth_wheel_consumes_pending_targets_before_bubbling() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 40.0);
        let inner = Element::scroll(
            "inner",
            Element::column(
                (0..10)
                    .map(|index| {
                        Element::button(format!("inner:{index}"), "Row").size(None, Some(20.0))
                    })
                    .collect(),
            ),
        )
        .size(None, Some(80.0));
        let mut tree = UiTree::new(Element::scroll(
            "outer",
            Element::column(vec![inner, Element::spacer().size(None, Some(200.0))]),
        ));
        tree.layout_at(viewport, start);
        for delta_y in [-50.0, -100.0] {
            tree.handle_event_at(
                UiEvent::Scroll {
                    position: Point::new(10.0, 10.0),
                    delta_y,
                },
                start,
            );
        }
        assert_offset(&tree, "inner", 0.0);
        assert_offset(&tree, "outer", 0.0);
        assert!(tree.is_scroll_at_end("inner", 0.0));
        assert!(!tree.is_scroll_at_end("outer", 0.0));
        tree.layout_at(viewport, start + Duration::from_millis(140));
        assert_offset(&tree, "inner", 120.0);
        assert_offset(&tree, "outer", 30.0);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: 150.0,
            },
            start + Duration::from_millis(140),
        );
        tree.layout_at(viewport, start + Duration::from_millis(280));
        assert_offset(&tree, "inner", 0.0);
        assert_offset(&tree, "outer", 0.0);
    }

    #[test]
    fn track_page_click_animates_without_turning_pointer_motion_into_a_drag() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(
            scroll_rows(25)
                .scrollbar_appearance(crate::ScrollbarAppearance::reserved().buttons(false)),
        );
        let frame = tree.layout_at(viewport, start);
        let bar = frame.scrollbars["list"];
        assert!(bar.up_button.is_none() && bar.down_button.is_none());
        assert_eq!(bar.track, bar.rail);
        tree.handle_event_at(UiEvent::PointerDown(Point::new(94.0, 80.0)), start);
        assert_offset(&tree, "list", 0.0);
        tree.handle_event_at(UiEvent::PointerMove(Point::new(94.0, 10.0)), start);
        tree.handle_event_at(UiEvent::PointerUp(Point::new(94.0, 10.0)), start);
        tree.layout_at(viewport, start + Duration::from_millis(140));
        assert_offset(&tree, "list", 90.0);
    }

    #[test]
    fn thumb_drag_cancels_pending_motion_and_tracks_pointer_directly() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(
            scroll_rows(25).scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
        );
        tree.layout_at(viewport, start);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: -100.0,
            },
            start,
        );
        let middle = start + Duration::from_millis(35);
        let frame = tree.layout_at(viewport, middle);
        let bar = frame.scrollbars["list"];
        let grab = rect_center(bar.thumb);
        let displayed = tree.scroll_offset("list");
        tree.handle_event_at(UiEvent::PointerDown(grab), middle);
        assert_offset(&tree, "list", displayed);
        let end = Point::new(
            grab.x,
            bar.track.y + bar.track.height - bar.thumb.height / 2.0,
        );
        tree.handle_event_at(UiEvent::PointerMove(end), middle);
        assert_offset(&tree, "list", 400.0);
        tree.handle_event_at(UiEvent::PointerUp(end), middle);
        tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 400.0);
        assert!(!tree.is_animating());
    }

    #[test]
    fn programmatic_end_index_and_focus_override_pending_wheel_motion() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let wheel = || UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        };
        let mut end = UiTree::new(scroll_rows(25));
        end.layout_at(viewport, start);
        end.handle_event_at(wheel(), start);
        end.scroll_to_end("list");
        end.layout_at(viewport, start + Duration::from_millis(35));
        assert_offset(&end, "list", 400.0);
        end.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&end, "list", 400.0);

        let mut index = UiTree::new(Element::uniform_list("list", 25, 20.0, |index| {
            Element::button(format!("row:{index}"), "Row")
        }));
        index.layout_at(viewport, start);
        index.handle_event_at(wheel(), start);
        index.scroll_to_index("list", 20);
        index.layout_at(viewport, start + Duration::from_millis(35));
        assert_offset(&index, "list", 320.0);
        index.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&index, "list", 320.0);

        let mut focus = UiTree::new(scroll_rows(25));
        focus.layout_at(viewport, start);
        focus.handle_event_at(wheel(), start);
        focus.layout_at(viewport, start + Duration::from_millis(35));
        assert!(focus.focus("row:0"));
        let revealed = focus.layout_at(viewport, start + Duration::from_millis(35));
        assert_offset(&focus, "list", 0.0);
        assert!(revealed.regions.iter().any(|hit| hit.id == "row:0"));
        focus.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&focus, "list", 0.0);
    }

    #[test]
    fn shrinking_content_and_reduced_motion_stop_pending_scroll_animation() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(scroll_rows(25));
        tree.layout_at(viewport, start);
        let wheel = || UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -200.0,
        };
        tree.handle_event_at(wheel(), start);
        tree.layout_at(viewport, start + Duration::from_millis(70));
        assert_offset(&tree, "list", 175.0);
        tree.set_root(scroll_rows(7));
        let frame = tree.layout_at(viewport, start + Duration::from_millis(70));
        assert_eq!(frame.scroll_extents["list"], 40.0);
        assert_offset(&tree, "list", 40.0);
        tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 40.0);
        tree.set_root(scroll_rows(2));
        let frame = tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 0.0);
        assert!(!frame.scrollbars.contains_key("list"));
        assert!(!tree.is_animating());

        tree.set_root(scroll_rows(25));
        tree.layout_at(viewport, start + Duration::from_secs(2));
        tree.handle_event_at(wheel(), start + Duration::from_secs(2));
        tree.layout_at(
            viewport,
            start + Duration::from_secs(2) + Duration::from_millis(70),
        );
        tree.set_reduced_motion(true);
        tree.layout_at(
            viewport,
            start + Duration::from_secs(2) + Duration::from_millis(70),
        );
        assert_offset(&tree, "list", 200.0);
        assert!(!tree.is_animating());
        tree.handle_event_at(wheel(), start + Duration::from_secs(3));
        assert_offset(&tree, "list", 400.0);
    }

    #[test]
    fn scrollbar_arrow_hold_repeats_after_delay_and_stops_on_release_or_leave() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(scroll_rows(25));
        let frame = tree.layout_at(viewport, start);
        let down = rect_center(frame.scrollbars["list"].down_button.unwrap());
        tree.handle_event_at(UiEvent::PointerDown(down), start);
        tree.layout_at(viewport, start + Duration::from_millis(140));
        assert_offset(&tree, "list", 48.0);
        tree.layout_at(viewport, start + Duration::from_millis(349));
        assert_offset(&tree, "list", 48.0);
        assert!(
            tree.is_animating(),
            "a held arrow needs another repaint after its repeat delay"
        );
        tree.layout_at(viewport, start + Duration::from_millis(350));
        tree.layout_at(viewport, start + Duration::from_millis(410));
        tree.handle_event_at(UiEvent::PointerUp(down), start + Duration::from_millis(411));
        tree.layout_at(viewport, start + Duration::from_millis(600));
        assert_offset(&tree, "list", 144.0);
        assert!(!tree.is_animating());
        tree.layout_at(viewport, start + Duration::from_secs(2));
        assert_offset(&tree, "list", 144.0);

        for cancel in [UiEvent::PointerUp(down), UiEvent::PointerLeave] {
            let mut tree = UiTree::new(scroll_rows(25));
            tree.layout_at(viewport, start);
            tree.handle_event_at(UiEvent::PointerDown(down), start);
            tree.handle_event_at(cancel, start + Duration::from_millis(100));
            tree.layout_at(viewport, start + Duration::from_secs(1));
            assert_offset(&tree, "list", 48.0);
            assert!(!tree.is_animating());
        }
    }

    #[test]
    fn pending_wheel_target_preserves_stream_follow_end_intention() {
        let start = Instant::now();
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(scroll_rows(10));
        tree.layout_at(viewport, start);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: -100.0,
            },
            start,
        );
        assert_offset(&tree, "list", 0.0);
        assert!(
            tree.is_scroll_at_end("list", 0.0),
            "follow-end uses intended target during interpolation"
        );
        tree.set_root(scroll_rows(15));
        tree.scroll_to_end("list");
        tree.layout_at(viewport, start + Duration::from_millis(30));
        assert_offset(&tree, "list", 200.0);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: 50.0,
            },
            start + Duration::from_millis(30),
        );
        assert!(
            !tree.is_scroll_at_end("list", 0.0),
            "scrolling away immediately releases follow-end"
        );
    }

    #[test]
    fn animated_uniform_list_remains_lazy_at_each_intermediate_frame() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let start = Instant::now();
        let viewport = Size::new(160.0, 100.0);
        let built = Arc::new(AtomicUsize::new(0));
        let counter = built.clone();
        let mut tree = UiTree::new(Element::uniform_list("list", 1_000, 20.0, move |index| {
            counter.fetch_add(1, Ordering::SeqCst);
            Element::button(format!("row:{index}"), "Row")
        }));
        tree.layout_at(viewport, start);
        assert_eq!(built.load(Ordering::SeqCst), 5);
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y: -100.0,
            },
            start,
        );
        for milliseconds in [35, 70, 105, 140] {
            let before = built.load(Ordering::SeqCst);
            let frame = tree.layout_at(viewport, start + Duration::from_millis(milliseconds));
            let after = built.load(Ordering::SeqCst);
            assert!(
                (5..=12).contains(&(after - before)),
                "only intersecting rows plus one hover repaint are built at {milliseconds}ms"
            );
            assert!(
                frame
                    .regions
                    .iter()
                    .filter(|hit| hit.kind == HitKind::Button)
                    .count()
                    <= 6
            );
        }
        assert_offset(&tree, "list", 100.0);
    }

    #[test]
    fn scrollbar_tracks_overflow_and_preserves_child_hit_regions() {
        let mut tree = UiTree::new(
            scroll_rows(10).scrollbar_appearance(crate::ScrollbarAppearance::overlay()),
        );
        let frame = tree.layout(Size::new(100.0, 100.0));
        let bar = frame.scrollbars["list"];
        assert_eq!(bar.rail, Rect::new(86.0, 0.0, 14.0, 100.0));
        assert_eq!(bar.track, Rect::new(86.0, 14.0, 14.0, 72.0));
        assert_eq!(bar.up_button, Some(Rect::new(86.0, 0.0, 14.0, 14.0)));
        assert_eq!(bar.down_button, Some(Rect::new(86.0, 86.0, 14.0, 14.0)));
        assert_eq!(bar.thumb.height, 36.0);
        assert_eq!(bar.max_offset, 100.0);
        assert_eq!(
            frame
                .regions
                .iter()
                .find(|region| region.id == "row:0")
                .unwrap()
                .rect
                .width,
            100.0,
            "an overlay must not shrink the content when it overflows"
        );
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, .. } if *rect == bar.thumb
        )));
        assert!(
            frame.commands.iter().all(|command| !matches!(command,
                DrawCommand::FillRect { rect, .. } if *rect == bar.track
            )),
            "the idle track must be transparent"
        );
        assert_eq!(frame.hit_test(Point::new(82.0, 10.0)), Some("row:0"));
        assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("list"));
        assert_eq!(
            frame.hit_region(Point::new(94.0, 10.0)).map(|hit| hit.kind),
            Some(HitKind::Scrollbar)
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(82.0, 10.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(82.0, 10.0))),
            vec![UiAction::Clicked { id: "row:0".into() }]
        );

        tree.set_root(scroll_rows(2).scrollbar_appearance(crate::ScrollbarAppearance::overlay()));
        let frame = tree.layout(Size::new(100.0, 100.0));
        assert!(!frame.scrollbars.contains_key("list"));
        assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("row:0"));
        assert_eq!(
            frame
                .regions
                .iter()
                .find(|region| region.id == "row:0")
                .unwrap()
                .rect
                .width,
            100.0,
            "content width must stay stable when overflow disappears"
        );
    }

    #[test]
    fn overlay_rail_leaves_a_trailing_action_clickable() {
        let rows = Element::column(
            (0..10)
                .map(|index| {
                    Element::row(vec![
                        Element::spacer().flex(1.0),
                        Element::button(format!("action:{index}"), "···")
                            .padding(0.0)
                            .size(Some(20.0), Some(20.0)),
                    ])
                    .size(None, Some(20.0))
                })
                .collect(),
        );
        let mut tree = UiTree::new(
            Element::scroll("list", rows).scrollbar_appearance(
                crate::ScrollbarAppearance::overlay()
                    .buttons(false)
                    .rail_width(8.0),
            ),
        );
        let frame = tree.layout(Size::new(100.0, 100.0));
        assert_eq!(frame.hit_test(Point::new(90.0, 10.0)), Some("action:0"));
        assert_eq!(
            frame.hit_test(Point::new(96.0, 70.0)),
            Some("action:3"),
            "the transparent track must not take clicks from a trailing action"
        );
        assert_eq!(
            frame.hit_test(Point::new(96.0, 10.0)),
            Some("list"),
            "the visible thumb remains draggable"
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(90.0, 10.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(90.0, 10.0))),
            vec![UiAction::Clicked {
                id: "action:0".into()
            }]
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(96.0, 70.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(96.0, 70.0))),
            vec![UiAction::Clicked {
                id: "action:3".into()
            }]
        );
        tree.handle_event(UiEvent::PointerDown(Point::new(96.0, 25.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(96.0, 50.0)));
        assert_eq!(tree.scroll_offset("list"), 50.0);
    }

    #[test]
    fn focusing_nested_offscreen_action_reveals_it_without_offscreen_hits() {
        let rows = Element::column(
            (0..10)
                .map(|index| {
                    Element::panel(
                        Element::row(vec![
                            Element::spacer().flex(1.0),
                            Element::button(format!("action:{index}"), "Open")
                                .size(Some(40.0), Some(20.0)),
                        ])
                        .size(None, Some(20.0)),
                    )
                    .size(None, Some(20.0))
                })
                .collect(),
        );
        let mut tree = UiTree::new(Element::scroll("list", rows));
        let initial = tree.layout(Size::new(100.0, 60.0));
        assert!(!initial.regions.iter().any(|hit| hit.id == "action:6"));
        assert!(!initial.scroll_focus_targets.contains_key("action:6"));

        assert!(tree.focus("action:6"));
        let revealed = tree.layout(Size::new(100.0, 60.0));
        assert_eq!(tree.scroll_offset("list"), 80.0);
        assert!(revealed.regions.iter().any(|hit| hit.id == "action:6"));
        assert!(!revealed.regions.iter().any(|hit| hit.id == "action:0"));
        assert_eq!(revealed.hit_test(Point::new(80.0, 50.0)), Some("action:6"));
    }

    /// Rows of uneven heights, so no row's position follows from its index.
    fn uneven_rows() -> Element {
        Element::column(
            (0..10)
                .map(|index| {
                    Element::button(format!("row:{index}"), "Row")
                        .size(None, Some(20.0 + 10.0 * (index % 3) as f32))
                })
                .collect(),
        )
    }

    #[test]
    fn reveal_scrolls_a_control_of_any_height_into_view_with_a_margin() {
        let viewport = Size::new(100.0, 100.0);
        let mut tree = UiTree::new(Element::scroll("list", uneven_rows()));
        tree.set_reduced_motion(true);
        let initial = tree.layout(viewport);
        assert!(!initial.regions.iter().any(|hit| hit.id == "row:7"));

        // Rows 0..=6 are 200 tall and row 7 is 30: its end, with the margin
        // under it, lands on the viewport's end.
        tree.reveal("row:7");
        let revealed = tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), 230.0 + REVEAL_MARGIN - 100.0);
        let row = revealed.regions.iter().find(|hit| hit.id == "row:7").unwrap().rect;
        assert_eq!((row.y + row.height, row.height), (100.0 - REVEAL_MARGIN, 30.0));
        assert_eq!(tree.focused_id(), None, "revealing does not move focus");

        // A control above the viewport comes down to the margin under its top.
        tree.reveal("row:2");
        let above = tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), 50.0 - REVEAL_MARGIN);
        let row = above.regions.iter().find(|hit| hit.id == "row:2").unwrap().rect;
        assert_eq!(row.y, REVEAL_MARGIN);

        // In view already, or not in the tree: nothing moves, nothing lingers.
        let offset = tree.scroll_offset("list");
        for id in ["row:3", "missing"] {
            tree.reveal(id);
            tree.layout(viewport);
            assert_eq!(tree.scroll_offset("list"), offset, "{id}");
        }
        tree.set_root(Element::scroll("list", uneven_rows()));
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), offset);

        // A margin of the host's own; one too large for the viewport shrinks
        // to center the control, and a bad one falls back to the default.
        tree.reveal_with_margin("row:7", 30.0);
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), 230.0 + 30.0 - 100.0);
        tree.reveal_with_margin("row:2", 500.0);
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), 50.0 - (100.0 - 40.0) / 2.0);
        tree.reveal_with_margin("row:7", f32::NAN);
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), 230.0 + REVEAL_MARGIN - 100.0);
    }

    #[test]
    fn reveal_scrolls_every_area_around_the_control_and_yields_to_focus() {
        let viewport = Size::new(100.0, 100.0);
        let root = || {
            Element::scroll(
                "page",
                Element::column(vec![
                    Element::button("top", "Top").size(None, Some(20.0)),
                    Element::spacer().size(None, Some(300.0)),
                    Element::scroll("inner", uneven_rows()).size(None, Some(60.0)),
                ]),
            )
        };
        let mut tree = UiTree::new(root());
        tree.set_reduced_motion(true);
        tree.layout(viewport);
        // A focus request in the same frame is located first; the reveal
        // still resolves in that layout call.
        assert!(tree.focus("top"));
        tree.reveal("row:9");
        let frame = tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("top"));
        assert!(tree.scroll_offset("page") > 0.0);
        assert!(tree.scroll_offset("inner") > 0.0);
        let row = frame.regions.iter().find(|hit| hit.id == "row:9").unwrap().rect;
        assert!(row.y >= 0.0 && row.y + row.height <= viewport.height, "{row:?}");
        assert_eq!(row.height, 20.0);
    }

    #[test]
    fn reserved_rail_remains_available_for_dense_right_edge_content() {
        let rows = Element::column(
            (0..10)
                .map(|index| Element::button(format!("row:{index}"), "Row").size(None, Some(20.0)))
                .collect(),
        );
        let mut tree = UiTree::new(
            Element::scroll("list", rows)
                .scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
        );
        let frame = tree.layout(Size::new(100.0, 100.0));
        assert_eq!(frame.scrollbars["list"].rail.width, 14.0);
        assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("list"));
        assert_eq!(frame.hit_test(Point::new(82.0, 10.0)), Some("row:0"));
    }

    #[test]
    fn reserved_scrollbar_drag_and_track_click_move_retained_scroll_offset() {
        let mut tree = UiTree::new(
            scroll_rows(10).scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
        );
        tree.set_reduced_motion(true);
        tree.layout(Size::new(100.0, 100.0));
        assert!(tree
            .handle_event(UiEvent::PointerDown(Point::new(94.0, 32.0)))
            .is_empty());
        tree.handle_event(UiEvent::PointerMove(Point::new(94.0, 50.0)));
        assert_eq!(tree.scroll_offset("list"), 50.0);
        assert!(tree
            .handle_event(UiEvent::PointerUp(Point::new(94.0, 50.0)))
            .is_empty());
        let frame = tree.layout(Size::new(100.0, 100.0));
        assert_eq!(frame.scrollbars["list"].thumb.y, 32.0);

        tree.handle_event(UiEvent::PointerDown(Point::new(94.0, 80.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(94.0, 80.0)));
        assert_eq!(tree.scroll_offset("list"), 100.0);
        tree.set_root(scroll_rows(15).scrollbar_appearance(crate::ScrollbarAppearance::reserved()));
        let frame = tree.layout(Size::new(100.0, 100.0));
        assert_eq!(tree.scroll_offset("list"), 100.0);
        assert_eq!(frame.scrollbars["list"].max_offset, 200.0);
    }

    #[test]
    fn scrollbar_is_bounded_in_narrow_and_clipped_nested_viewports() {
        let mut tree = UiTree::new(scroll_rows(4));
        let frame = tree.layout(Size::new(12.0, 40.0));
        let bar = frame.scrollbars["list"];
        assert_eq!(bar.rail, Rect::new(8.0, 0.0, 4.0, 40.0));
        assert!(bar.track.y >= bar.rail.y);
        assert!(bar.track.y + bar.track.height <= bar.rail.y + bar.rail.height);
        assert!(bar.thumb.width > 0.0 && bar.thumb.height > 0.0);
        assert_eq!(frame.hit_test(Point::new(2.0, 10.0)), Some("row:0"));
        assert_eq!(frame.hit_test(Point::new(10.0, 10.0)), Some("list"));

        let inner = Element::scroll(
            "inner",
            Element::column(
                (0..10)
                    .map(|index| {
                        Element::button(format!("inner:{index}"), "I").size(None, Some(20.0))
                    })
                    .collect(),
            ),
        )
        .size(None, Some(80.0));
        let mut nested = UiTree::new(Element::scroll("outer", inner));
        let frame = nested.layout(Size::new(100.0, 40.0));
        assert!(frame.scrollbars.contains_key("inner"));
        assert_eq!(frame.hit_test(Point::new(82.0, 41.0)), None);
        assert_eq!(
            frame.hit_region(Point::new(96.0, 10.0)).map(|hit| hit.kind),
            Some(HitKind::Scrollbar)
        );
    }

    #[test]
    fn exhausted_floating_menu_does_not_scroll_covered_background() {
        let viewport = Size::new(320.0, 240.0);
        let menu = Element::scroll(
            "menu",
            Element::column(
                (0..12)
                    .map(|index| {
                        Element::button(format!("menu:{index}"), "Item").size(None, Some(20.0))
                    })
                    .collect(),
            ),
        )
        .size(Some(160.0), Some(80.0));
        let mut tree = UiTree::new(Element::popover(
            Element::scroll("background", Element::spacer().size(None, Some(600.0))),
            menu,
            "dismiss_menu",
            PopoverPlacement::BottomStart,
        ));
        tree.set_reduced_motion(true);
        tree.scroll_to_end("menu");
        let frame = tree.layout(viewport);
        let menu = frame
            .regions
            .iter()
            .find(|region| region.kind == HitKind::Scroll && region.id == "menu")
            .unwrap();
        let point = Point::new(menu.rect.x + 20.0, menu.rect.y + 20.0);
        assert!(
            frame.regions.iter().any(|region| {
                region.kind == HitKind::Scroll
                    && region.id == "background"
                    && region.rect.contains(point)
            }),
            "the background scroll is covered by this menu"
        );
        let menu_end = tree.scroll_offset("menu");
        assert!(menu_end > 0.0);
        for event in [
            UiEvent::Scroll {
                position: point,
                delta_y: -40.0,
            },
            UiEvent::ScrollDirect {
                position: point,
                delta_y: -40.0,
            },
        ] {
            tree.handle_event(event);
            tree.layout(viewport);
            assert_eq!(tree.scroll_offset("menu"), menu_end);
            assert_eq!(tree.scroll_offset("background"), 0.0);
        }
        tree.handle_event(UiEvent::Scroll {
            position: point,
            delta_y: 40.0,
        });
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("menu"), menu_end - 40.0);
        assert_eq!(tree.scroll_offset("background"), 0.0);
    }

    #[test]
    fn modal_blocks_outside_wheel_but_its_body_still_scrolls() {
        let viewport = Size::new(500.0, 340.0);
        let mut tree = UiTree::new(
            crate::Modal::new(
                Element::scroll("background", Element::spacer().size(None, Some(800.0))),
                true,
                "Scrollable dialog",
                "",
                "dismiss_modal",
                ComponentPalette::neutral_dark(),
            )
            .width(240.0)
            .height(180.0)
            .body(Element::spacer().size(None, Some(600.0)))
            // A press on the backdrop closes it; a wheel there never does.
            .backdrop_dismiss(true)
            .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(viewport);
        let outside = Point::new(10.0, 10.0);
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss_modal")
            .unwrap();
        assert!(!surface.rect.contains(outside));
        let body = frame
            .regions
            .iter()
            .find(|region| {
                region.kind == HitKind::Scroll && region.id == "modal_body:dismiss_modal"
            })
            .unwrap();
        let inside = Point::new(body.rect.x + 20.0, body.rect.y + 20.0);
        for event in [
            UiEvent::Scroll {
                position: outside,
                delta_y: -40.0,
            },
            UiEvent::ScrollDirect {
                position: outside,
                delta_y: -40.0,
            },
        ] {
            assert!(tree.handle_event(event).is_empty(), "a wheel never closes a modal");
            tree.layout(viewport);
            assert_eq!(tree.scroll_offset("background"), 0.0);
            assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 0.0);
        }
        tree.handle_event(UiEvent::PointerDown(outside));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(outside)), vec![UiAction::Clicked { id: "dismiss_modal".into() }]);
        tree.handle_event(UiEvent::Scroll {
            position: inside,
            delta_y: -40.0,
        });
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 40.0);
        assert_eq!(tree.scroll_offset("background"), 0.0);
        tree.handle_event(UiEvent::ScrollDirect {
            position: inside,
            delta_y: -20.0,
        });
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 60.0);
        assert_eq!(tree.scroll_offset("background"), 0.0);
    }

    #[test]
    fn nested_wheel_bubbles_when_inner_scroll_reaches_its_end() {
        let inner = Element::scroll(
            "inner",
            Element::column(
                (0..10)
                    .map(|index| {
                        Element::button(format!("inner:{index}"), "Row").size(None, Some(20.0))
                    })
                    .collect(),
            ),
        )
        .size(None, Some(80.0));
        let outer = Element::scroll(
            "outer",
            Element::column(vec![inner, Element::spacer().size(None, Some(200.0))]),
        );
        let mut tree = UiTree::new(outer);
        tree.set_reduced_motion(true);
        tree.layout(Size::new(100.0, 40.0));
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -50.0,
        });
        assert_eq!(tree.scroll_offset("inner"), 50.0);
        assert_eq!(tree.scroll_offset("outer"), 0.0);
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        });
        assert_eq!(tree.scroll_offset("inner"), 120.0);
        assert_eq!(tree.scroll_offset("outer"), 30.0);
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -30.0,
        });
        assert_eq!(tree.scroll_offset("outer"), 60.0);
    }

    fn nested_diff_scroll_tree(rows: usize, height: f32, long_lines: bool) -> UiTree {
        use crate::Component;
        let lines: Vec<_> = (0..rows)
            .map(|index| crate::CodeDiffLine {
                kind: crate::CodeDiffLineKind::Context,
                old_line: Some(index + 1),
                new_line: Some(index + 1),
                text: if long_lines {
                    "long line ".repeat(100)
                } else {
                    format!("line {index}")
                },
            })
            .collect();
        let diff = crate::CodeDiff::new("review", lines)
            .height(height)
            .render();
        UiTree::new(Element::scroll(
            "outer",
            Element::column(vec![diff, Element::spacer().size(None, Some(400.0))]),
        ))
    }

    #[test]
    fn nested_short_diff_bubbles_direct_and_smooth_wheel_to_parent() {
        let now = Instant::now();
        for direct in [false, true] {
            let mut tree = nested_diff_scroll_tree(2, 80.0, false);
            tree.layout_at(Size::new(360.0, 100.0), now);
            let position = Point::new(120.0, 30.0);
            assert_eq!(
                tree.frame.hit_region(position).unwrap().kind,
                HitKind::CodeDiff
            );
            assert_eq!(tree.code_diffs["review"].viewport.max_y, 0.0);
            tree.handle_event_at(
                if direct {
                    UiEvent::ScrollDirect {
                        position,
                        delta_y: -50.0,
                    }
                } else {
                    UiEvent::Scroll {
                        position,
                        delta_y: -50.0,
                    }
                },
                now,
            );
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
            assert_eq!(tree.scroll_target("outer"), 50.0);
            tree.layout_at(Size::new(360.0, 100.0), now + Duration::from_secs(1));
            assert_eq!(tree.scroll_offset("outer"), 50.0);
        }
    }

    #[test]
    fn nested_long_diff_bubbles_only_vertical_residual_at_each_boundary() {
        let now = Instant::now();
        for direct in [false, true] {
            let mut tree = nested_diff_scroll_tree(10, 140.0, false);
            tree.layout_at(Size::new(360.0, 100.0), now);
            let position = Point::new(120.0, 30.0);
            let max_y = tree.code_diffs["review"].viewport.max_y;
            assert!(max_y > 30.0 && max_y < 110.0);
            let scroll = |tree: &mut UiTree, delta_y| {
                tree.handle_event_at(
                    if direct {
                        UiEvent::ScrollDirect { position, delta_y }
                    } else {
                        UiEvent::Scroll { position, delta_y }
                    },
                    now,
                )
            };
            scroll(&mut tree, -30.0);
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 30.0);
            assert_eq!(tree.scroll_target("outer"), 0.0);
            scroll(&mut tree, -80.0);
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, max_y);
            let first_residual = 110.0 - max_y;
            assert_eq!(tree.scroll_target("outer"), first_residual);
            scroll(&mut tree, -30.0);
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, max_y);
            assert_eq!(tree.scroll_target("outer"), first_residual + 30.0);
            // Reversing direction first moves the diff to its start; only the
            // remaining delta moves the enclosing transcript back upward.
            scroll(&mut tree, 100.0);
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
            assert_eq!(tree.scroll_target("outer"), 40.0);
        }
    }

    #[test]
    fn nested_diff_shift_wheel_keeps_horizontal_intent_and_bubbles_diagonal_y() {
        let now = Instant::now();
        for direct in [false, true] {
            let mut tree = nested_diff_scroll_tree(2, 80.0, true);
            tree.layout_at(Size::new(360.0, 100.0), now);
            let position = Point::new(120.0, 30.0);
            assert!(tree.code_diffs["review"].viewport.max_x > 80.0);
            tree.handle_event_at(
                UiEvent::Scroll2D {
                    position,
                    delta_x: 0.0,
                    delta_y: -60.0,
                    direct,
                    shift: true,
                },
                now,
            );
            assert_eq!(tree.code_diffs["review"].viewport.scroll_x, 60.0);
            assert_eq!(tree.scroll_target("outer"), 0.0);
            tree.handle_event_at(
                UiEvent::Scroll2D {
                    position,
                    delta_x: -20.0,
                    delta_y: -40.0,
                    direct,
                    shift: false,
                },
                now,
            );
            assert_eq!(tree.code_diffs["review"].viewport.scroll_x, 80.0);
            assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
            assert_eq!(tree.scroll_target("outer"), 40.0);
        }
    }

    #[test]
    fn shifted_two_axis_wheel_outside_code_preserves_legacy_vertical_contract() {
        let now = Instant::now();
        let position = Point::new(10.0, 10.0);
        for direct in [false, true] {
            let mut legacy = UiTree::new(scroll_rows(10));
            let mut two_axis = UiTree::new(scroll_rows(10));
            for tree in [&mut legacy, &mut two_axis] {
                tree.layout_at(Size::new(100.0, 40.0), now);
            }
            legacy.handle_event_at(
                if direct {
                    UiEvent::ScrollDirect {
                        position,
                        delta_y: -75.0,
                    }
                } else {
                    UiEvent::Scroll {
                        position,
                        delta_y: -75.0,
                    }
                },
                now,
            );
            two_axis.handle_event_at(
                UiEvent::Scroll2D {
                    position,
                    delta_x: 0.0,
                    delta_y: -75.0,
                    direct,
                    shift: true,
                },
                now,
            );
            assert_eq!(legacy.scroll_target("list"), 75.0);
            assert_eq!(two_axis.scroll_target("list"), legacy.scroll_target("list"));
            let size = Size::new(100.0, 40.0);
            let settled = now + Duration::from_secs(1);
            legacy.layout_at(size, settled);
            two_axis.layout_at(size, settled);
            assert_eq!(two_axis.scroll_offset("list"), legacy.scroll_offset("list"));
        }
    }

    #[test]
    fn invalid_wheel_delta_does_not_change_scroll_position() {
        let mut tree = UiTree::new(scroll_rows(10));
        tree.layout(Size::new(100.0, 40.0));
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: f32::NAN,
        });
        assert_eq!(tree.scroll_offset("list"), 0.0);
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: f32::INFINITY,
        });
        assert_eq!(tree.scroll_offset("list"), 0.0);
    }

    #[test]
    fn input_state_survives_tree_rebuild_and_edits_unicode() {
        let mut tree = UiTree::new(Element::text_input("prompt", "", "Ask"));
        tree.layout(Size::new(200.0, 40.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
        assert_eq!(
            tree.handle_event(UiEvent::Text("a🦀b".into())),
            vec![UiAction::InputChanged {
                id: "prompt".into(),
                value: "a🦀b".into()
            }]
        );
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowLeft,
            shift: true,
            ctrl: false,
        });
        tree.handle_event(UiEvent::Key {
            key: Key::Backspace,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.input_value("prompt"), Some("a🦀"));
        tree.set_root(Element::text_input("prompt", "stale", "Ask"));
        tree.layout(Size::new(200.0, 40.0));
        assert_eq!(tree.focused_id(), Some("prompt"));
        assert_eq!(tree.input_value("prompt"), Some("a🦀"));
    }

    #[test]
    fn click_requires_release_inside_same_button() {
        let mut tree = UiTree::new(Element::button("send", "Send"));
        tree.layout(Size::new(100.0, 40.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
        assert!(tree
            .handle_event(UiEvent::PointerUp(Point::new(120.0, 5.0)))
            .is_empty());
        tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(5.0, 5.0))),
            vec![UiAction::Clicked { id: "send".into() }]
        );
    }

    #[test]
    fn scroll_to_end_uses_new_content_height() {
        let rows = |count: usize| {
            Element::scroll(
                "transcript",
                Element::column(
                    (0..count)
                        .map(|index| Element::text(index.to_string()).size(None, Some(20.0)))
                        .collect(),
                ),
            )
        };
        let mut tree = UiTree::new(rows(2));
        tree.layout(Size::new(100.0, 20.0));
        tree.set_root(rows(4));
        tree.scroll_to_end("transcript");
        let frame = tree.layout(Size::new(100.0, 20.0));
        assert_eq!(tree.scroll_offset("transcript"), 60.0);
        assert!(frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "3")));
        assert!(!frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "0")));
    }

    #[test]
    fn uniform_list_builds_only_visible_rows_and_reaches_last_of_one_thousand() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let built = Arc::new(AtomicUsize::new(0));
        let list = |built: Arc<AtomicUsize>| {
            Element::uniform_list("sessions", 1_000, 20.0, move |index| {
                built.fetch_add(1, Ordering::SeqCst);
                Element::button(format!("session:{index}"), format!("Session {index}"))
            })
        };
        let mut tree = UiTree::new(list(built.clone()));
        let frame = tree.layout(Size::new(160.0, 100.0));
        assert_eq!(built.load(Ordering::SeqCst), 5);
        assert_eq!(frame.scroll_extents["sessions"], 19_900.0);
        assert_eq!(
            frame
                .regions
                .iter()
                .filter(|hit| hit.id.starts_with("session:"))
                .count(),
            5
        );
        assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:4"));
        assert_eq!(frame.hit_test(Point::new(30.0, 101.0)), None);
        assert_eq!(frame.hit_test(Point::new(140.0, 90.0)), Some("session:4"));
        assert_eq!(frame.hit_test(Point::new(155.0, 90.0)), Some("sessions"));
        assert_eq!(frame.hit_test(Point::new(155.0, 10.0)), Some("sessions"));
        assert_eq!(
            frame
                .regions
                .iter()
                .find(|region| region.id == "session:4")
                .unwrap()
                .rect
                .width,
            146.0,
            "default virtualized rows reserve a separate scrollbar rail"
        );
        let rail = frame.scrollbars["sessions"].rail;
        for row in frame
            .regions
            .iter()
            .filter(|hit| hit.id.starts_with("session:"))
        {
            assert!(
                row.rect.x + row.rect.width <= rail.x,
                "row hover and hit geometry must not touch the scrollbar rail"
            );
        }

        tree.scroll_to_index("sessions", 999);
        let frame = tree.layout(Size::new(160.0, 100.0));
        assert_eq!(tree.scroll_offset("sessions"), 19_900.0);
        assert_eq!(built.load(Ordering::SeqCst), 10);
        assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:999"));
        assert!(!frame.regions.iter().any(|hit| hit.id == "session:0"));

        tree.set_root(list(built.clone()));
        tree.layout(Size::new(160.0, 100.0));
        assert_eq!(tree.scroll_offset("sessions"), 19_900.0);
        assert_eq!(built.load(Ordering::SeqCst), 15);
    }

    #[test]
    fn a_row_of_a_uniform_list_opens_content_under_itself_and_the_list_stays_lazy() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const SIZE: Size = Size::new(160.0, 100.0);
        let built = Arc::new(AtomicUsize::new(0));
        let list = |built: Arc<AtomicUsize>, open: Option<(usize, f32)>| {
            let list = Element::uniform_list("sessions", 1_000, 20.0, move |index| {
                built.fetch_add(1, Ordering::SeqCst);
                Element::button(format!("session:{index}"), format!("Session {index}"))
            });
            match open {
                Some((index, height)) => list.list_expansion(
                    index,
                    Element::button("detail", "Detail").size(None, Some(height)),
                ),
                None => list,
            }
        };
        let rect = |frame: &Frame, id: &str| {
            frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
        };
        let mut tree = UiTree::new(list(built.clone(), Some((1, 50.0))));
        let frame = tree.layout(SIZE);
        // Under its row, as wide as a row and as tall as it asked to be; the
        // rows after it start where it ends.
        let (row, detail, next) = (
            rect(&frame, "session:1").unwrap(),
            rect(&frame, "detail").unwrap(),
            rect(&frame, "session:2").unwrap(),
        );
        assert_eq!((detail.y, detail.height), (row.y + row.height, 50.0));
        assert_eq!((detail.x, detail.width), (row.x, row.width));
        assert_eq!(next.y, detail.y + detail.height);
        // The list is that much longer, and builds only the rows that show.
        assert_eq!(frame.scroll_extents["sessions"], 19_900.0 + 50.0);
        assert_eq!(built.load(Ordering::SeqCst), 3);
        assert_eq!(frame.hit_test(Point::new(30.0, 60.0)), Some("detail"));
        assert_eq!(frame.hit_test(Point::new(30.0, 95.0)), Some("session:2"));

        // Scrolled into it, its row is still built with it and nothing above.
        tree.handle_event(UiEvent::ScrollDirect { position: Point::new(30.0, 50.0), delta_y: -60.0 });
        let frame = tree.layout(SIZE);
        assert_eq!(tree.scroll_offset("sessions"), 60.0);
        // Its part in view: the last thirty of its fifty.
        let detail = rect(&frame, "detail").unwrap();
        assert_eq!((detail.y, detail.height), (0.0, 30.0));
        assert!(rect(&frame, "session:0").is_none());
        assert_eq!(rect(&frame, "session:2").unwrap().y, 30.0);
        assert_eq!(frame.hit_test(Point::new(30.0, 10.0)), Some("detail"));

        // Far from it, neither it nor its row is built, and a row is revealed
        // where it stands under what was opened.
        tree.scroll_to_index("sessions", 500);
        let frame = tree.layout(SIZE);
        assert_eq!(tree.scroll_offset("sessions"), 501.0 * 20.0 + 50.0 - 100.0);
        assert!(rect(&frame, "detail").is_none() && rect(&frame, "session:1").is_none());
        assert_eq!(rect(&frame, "session:500").unwrap().y, 80.0);
        tree.scroll_to_index("sessions", 999);
        let frame = tree.layout(SIZE);
        assert_eq!(tree.scroll_offset("sessions"), 19_950.0);
        assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:999"));

        // Closed again, the list is the rows alone.
        tree.set_root(list(built.clone(), None));
        let frame = tree.layout(SIZE);
        assert_eq!(frame.scroll_extents["sessions"], 19_900.0);
        assert!(rect(&frame, "detail").is_none());
        // An index past the rows opens nothing.
        tree.set_root(list(built, Some((1_000, 50.0))));
        assert_eq!(tree.layout(SIZE).scroll_extents["sessions"], 19_900.0);
    }

    #[test]
    fn the_keyboard_walks_from_a_row_into_what_it_opened_and_on_to_the_next() {
        let list = Element::uniform_list("sessions", 10, 20.0, |index| {
            Element::button(format!("row:{index}"), format!("Task {index}"))
        })
        .list_expansion(
            3,
            Element::column(vec![
                Element::button("detail:first", "First").size(None, Some(20.0)),
                Element::button("detail:last", "Last").size(None, Some(20.0)),
            ]),
        );
        let tab = |tree: &mut UiTree, shift: bool| {
            tree.handle_event(UiEvent::Key { key: Key::Tab, shift, ctrl: false });
            tree.layout(Size::new(120.0, 60.0));
        };
        let mut tree = UiTree::new(list);
        tree.layout(Size::new(120.0, 60.0));
        // Row 3 is under the viewport: reaching it brings what it opened too.
        assert!(tree.focus("row:2"));
        for expected in ["row:3", "detail:first", "detail:last", "row:4"] {
            tab(&mut tree, false);
            assert_eq!(tree.focused_id(), Some(expected));
        }
        // And back, from the row after it into its last control.
        for expected in ["detail:last", "detail:first", "row:3", "row:2"] {
            tab(&mut tree, true);
            assert_eq!(tree.focused_id(), Some(expected));
        }
    }

    #[test]
    fn what_a_row_opens_can_open_on_the_spring_and_the_rows_follow_it() {
        let list = |open: bool| {
            Element::uniform_list("sessions", 100, 20.0, |index| {
                Element::button(format!("row:{index}"), format!("Task {index}"))
            })
            .list_expansion(
                0,
                crate::Collapse::new(
                    "detail",
                    open.then(|| Element::button("detail:body", "Body").size(None, Some(60.0))),
                )
                .expand()
                .render(),
            )
        };
        let size = Size::new(160.0, 200.0);
        let next = |frame: &Frame| {
            frame.regions.iter().find(|region| region.id == "row:1").unwrap().rect.y
        };
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(list(false));
        assert_eq!(next(&tree.layout_at(size, start)), 20.0);
        tree.set_root(list(true));
        tree.layout_at(size, start);
        let settle = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3 / 2);
        let midway = next(&tree.layout_at(size, start + settle / 2));
        assert!(20.0 < midway && midway < 80.0, "{midway}");
        let open = tree.layout_at(size, start + settle * 3);
        assert_eq!(next(&open), 80.0);
        assert_eq!(open.scroll_extents["sessions"], 100.0 * 20.0 + 60.0 - 200.0);
        assert!(!tree.is_animating());
    }

    #[test]
    fn uniform_list_tab_focus_continues_across_viewport_boundary() {
        let list = Element::uniform_list("sessions", 10, 20.0, |index| {
            Element::clickable(
                format!("row:{index}"),
                Element::row(vec![
                    Element::truncated_text(format!("Task {index}")).size(Some(58.0), Some(20.0)),
                    Element::button(format!("menu:{index}"), "⋯").size(Some(20.0), Some(20.0)),
                ]),
            )
        });
        let mut tree = UiTree::new(list);
        tree.layout(Size::new(120.0, 60.0));
        assert!(tree.focus("menu:2"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("row:3"));
        let frame = tree.layout(Size::new(120.0, 60.0));
        assert_eq!(tree.scroll_offset("sessions"), 20.0);
        assert!(frame.regions.iter().any(|hit| hit.id == "row:3"));

        assert!(tree.focus("row:1"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("menu:0"));
        let frame = tree.layout(Size::new(120.0, 60.0));
        assert_eq!(tree.scroll_offset("sessions"), 0.0);
        assert!(frame.regions.iter().any(|hit| hit.id == "menu:0"));
    }

    #[test]
    fn uniform_list_visible_rows_dispatch_scoped_aliases_and_drop_recycled_aliases() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let built = Arc::new(AtomicUsize::new(0));
        let rows = |scope: Option<&'static str>, built: Arc<AtomicUsize>| {
            Element::uniform_list("command_results", 1_000, 20.0, move |index| {
                built.fetch_add(1, Ordering::SeqCst);
                let row = Element::clickable(
                    format!("command_result:{index}"),
                    Element::text(format!("Result {index}")),
                )
                .size(None, Some(20.0));
                Element::column(vec![if let Some(scope) = scope {
                    row.dispatch_as(format!("command_pick:{index}:{scope}"))
                } else {
                    row
                }])
            })
        };
        let click = |tree: &mut UiTree, point| {
            tree.handle_event(UiEvent::PointerDown(point));
            tree.handle_event(UiEvent::PointerUp(point))
        };
        let size = Size::new(200.0, 60.0);
        let mut tree = UiTree::new(rows(Some("workspace-a/query-a"), built.clone()));
        tree.layout(size);
        assert_eq!(
            built.load(Ordering::SeqCst),
            3,
            "alias collection remains lazy"
        );
        assert_eq!(tree.action_aliases.len(), 3);
        assert_eq!(
            click(&mut tree, Point::new(20.0, 10.0)),
            vec![UiAction::Clicked {
                id: "command_pick:0:workspace-a/query-a".into(),
            }]
        );
        tree.scroll_to_index("command_results", 999);
        tree.layout(size);
        assert!(!tree.action_aliases.contains_key("command_result:0"));
        assert_eq!(tree.action_aliases.len(), 3);
        assert_eq!(
            click(&mut tree, Point::new(20.0, 50.0)),
            vec![UiAction::Clicked {
                id: "command_pick:999:workspace-a/query-a".into(),
            }]
        );
        tree.set_root(rows(Some("workspace-b/query-b"), built.clone()));
        tree.scroll_to_start("command_results");
        tree.layout(size);
        assert!(!tree.action_aliases.contains_key("command_result:999"));
        assert!(tree
            .action_aliases
            .values()
            .all(|action| action.ends_with("workspace-b/query-b")));
        assert!(tree.focus("command_result:0"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "command_pick:0:workspace-b/query-b".into(),
            }]
        );
        tree.set_root(rows(None, built));
        tree.layout(size);
        assert!(
            tree.action_aliases.is_empty(),
            "removing dispatch_as removes the previous alias too"
        );
        assert_eq!(
            click(&mut tree, Point::new(20.0, 10.0)),
            vec![UiAction::Clicked {
                id: "command_result:0".into(),
            }]
        );
    }

    #[test]
    fn virtual_alias_refresh_preserves_focus_and_cancels_only_retargeted_pointer_presses() {
        let rows = |scope: &'static str| {
            Element::uniform_list("results", 10, 20.0, move |index| {
                Element::clickable(format!("result:{index}"), Element::text("Result"))
                    .size(None, Some(20.0))
                    .dispatch_as(format!("pick:{scope}:{index}"))
            })
        };
        let size = Size::new(200.0, 60.0);
        let point = Point::new(20.0, 10.0);
        let mut tree = UiTree::new(rows("old"));
        tree.layout(size);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.set_root(rows("new"));
        tree.layout(size);
        assert_eq!(tree.focused_id(), Some("result:0"));
        assert!(
            tree.handle_event(UiEvent::PointerUp(point)).is_empty(),
            "held press must not activate a replacement result"
        );
        tree.handle_event(UiEvent::PointerDown(point));
        tree.set_root(rows("new"));
        tree.layout(size);
        assert_eq!(tree.focused_id(), Some("result:0"));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "pick:new:0".into(),
            }],
            "unchanged rebuilds keep their original pointer capture"
        );
        tree.handle_event(UiEvent::PointerDown(point));
        tree.scroll_to_index("results", 9);
        tree.layout(size);
        assert!(
            tree.handle_event(UiEvent::PointerUp(point)).is_empty(),
            "scrolling away cannot click another row at the same position"
        );
    }

    #[test]
    fn empty_uniform_list_has_no_rows_or_scrollbar() {
        let mut tree = UiTree::new(Element::uniform_list("empty", 0, 20.0, |_| {
            panic!("empty list must never build a row")
        }));
        tree.scroll_to_index("empty", 42);
        let frame = tree.layout(Size::new(100.0, 60.0));
        assert!(!frame.scrollbars.contains_key("empty"));
        assert_eq!(frame.scroll_extents["empty"], 0.0);
        assert!(tree.focus_order.is_empty());
    }

    #[test]
    fn selected_text_can_be_replaced() {
        let mut tree = UiTree::new(Element::text_input("input", "hello", ""));
        tree.layout(Size::new(200.0, 40.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(48.0, 10.0)));
        tree.handle_event(UiEvent::Key {
            key: Key::A,
            shift: false,
            ctrl: true,
        });
        tree.handle_event(UiEvent::Text("rust".into()));
        assert_eq!(tree.input_value("input"), Some("rust"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Submitted {
                id: "input".into(),
                value: "rust".into()
            }]
        );
    }

    #[test]
    fn tab_navigation_wraps_and_survives_tree_rebuild() {
        let controls = || {
            Element::row(vec![
                Element::button("first", "First"),
                Element::text_input("prompt", "", "Prompt"),
                Element::button("last", "Last"),
            ])
        };
        let mut tree = UiTree::new(controls());
        tree.layout(Size::new(400.0, 40.0));
        let tab = |shift| UiEvent::Key {
            key: Key::Tab,
            shift,
            ctrl: false,
        };
        tree.handle_event(tab(false));
        assert_eq!(tree.focused_id(), Some("first"));
        tree.handle_event(tab(false));
        assert_eq!(tree.focused_id(), Some("prompt"));

        tree.set_root(controls());
        tree.layout(Size::new(400.0, 40.0));
        assert_eq!(tree.focused_id(), Some("prompt"));
        tree.handle_event(tab(false));
        assert_eq!(tree.focused_id(), Some("last"));
        tree.handle_event(tab(false));
        assert_eq!(tree.focused_id(), Some("first"));
        tree.handle_event(tab(true));
        assert_eq!(tree.focused_id(), Some("last"));
    }

    #[test]
    fn enter_and_space_activate_focused_button() {
        let mut tree = UiTree::new(Element::button("send", "Send"));
        tree.layout(Size::new(100.0, 40.0));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        for key in [Key::Enter, Key::Space] {
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key,
                    shift: false,
                    ctrl: false,
                }),
                vec![UiAction::Clicked { id: "send".into() }]
            );
        }
        let frame = tree.layout(Size::new(100.0, 40.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { color, width: 2.0, .. }
                if *color == Color::rgb(170, 170, 170)
        )));
    }

    #[test]
    fn removing_focused_control_clears_focus() {
        let mut tree = UiTree::new(Element::button("old", "Old"));
        tree.layout(Size::new(100.0, 40.0));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        tree.set_root(Element::button("new", "New"));
        tree.layout(Size::new(100.0, 40.0));
        assert_eq!(tree.focused_id(), None);
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("new"));
    }

    #[test]
    fn decorative_top_fade_keeps_legacy_panel_behavior() {
        let content = || {
            Element::panel(Element::scroll(
                "scroll",
                Element::button("row", "A scrollable row").size(None, Some(36.0)),
            ))
            .size(None, Some(60.0))
        };
        let plain = UiTree::new(content()).layout(Size::new(220.0, 60.0));
        let faded = UiTree::new(content().top_fade(Color::rgb(20, 20, 20), 24.0))
            .layout(Size::new(220.0, 60.0));
        assert_eq!(plain.regions, faded.regions);
        let fade_index = faded
            .commands
            .iter()
            .position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. }))
            .unwrap();
        assert!(faded.commands[..fade_index].iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, .. } if text == "A scrollable row"
        )));
        assert!(matches!(
            faded.commands[fade_index],
            DrawCommand::FillVerticalGradient {
                rect: Rect {
                    y: 0.0,
                    height: 24.0,
                    ..
                },
                top: Color { a: 255, .. },
                bottom: Color { a: 0, .. },
            }
        ));
    }

    #[test]
    fn a_top_background_fade_is_painted_behind_the_children_from_the_top_edge_down() {
        let tint = Color::rgb(90, 40, 120);
        let content = || Element::column(vec![Element::text("Título")]).size(None, Some(200.0));
        let plain = UiTree::new(content()).layout(Size::new(220.0, 200.0));
        let washed = UiTree::new(content().top_background_fade(tint, 500.0)).layout(Size::new(220.0, 200.0));
        assert_eq!(plain.regions, washed.regions);
        let wash = washed.commands.iter().position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. })).unwrap();
        let text = washed.commands.iter().position(|command| matches!(command, DrawCommand::Text { .. })).unwrap();
        assert!(wash < text, "behind what the element holds");
        assert!(matches!(
            washed.commands[wash],
            // No taller than the element it is behind.
            DrawCommand::FillVerticalGradient { rect: Rect { y: 0.0, height: 200.0, .. }, top, bottom: Color { a: 0, .. } } if top == tint
        ));
    }

    #[test]
    fn spinner_phase_moves_and_reduced_motion_is_static() {
        let start = Instant::now();
        let area = Rect::new(0.0, 0.0, 32.0, 32.0);
        let ink = Color::rgb(220, 220, 220);
        let colors = |at, reduced| {
            let mut commands = Vec::new();
            paint_spinner(
                &mut commands,
                area,
                ink,
                SpinnerVariant::Circle,
                start,
                at,
                reduced,
            );
            commands
                .into_iter()
                .filter_map(|command| match command {
                    DrawCommand::Spinner { angle, .. } => Some(angle),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_ne!(
            colors(start, false),
            colors(start + Duration::from_millis(225), false)
        );
        assert_eq!(
            colors(start, true),
            colors(start + Duration::from_secs(1), true)
        );
    }

    #[test]
    fn rounded_dashed_border_paints_all_four_corners_inside_bounds() {
        let area = Rect::new(10.0, 20.0, 320.0, 240.0);
        let mut commands = Vec::new();
        paint_dashed_corners(&mut commands, area, 14.0, 1.0, Color::WHITE);
        let mut corners = [false; 4];
        for command in commands {
            let DrawCommand::FillRect { rect, .. } = command else {
                panic!("corner must use a rounded fill");
            };
            assert!(rect.x >= area.x && rect.y >= area.y);
            assert!(rect.x + rect.width <= area.x + area.width);
            assert!(rect.y + rect.height <= area.y + area.height);
            let left = rect.x < area.x + 14.0;
            let top = rect.y < area.y + 14.0;
            corners[match (left, top) {
                (true, true) => 0,
                (false, true) => 1,
                (false, false) => 2,
                (true, false) => 3,
            }] = true;
        }
        assert_eq!(corners, [true; 4]);
    }
}
