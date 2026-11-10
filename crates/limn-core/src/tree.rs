/// The layout's state store, to change. Measuring changes no state: what a
/// measure finds is kept when the element is placed, so the same tree measures
/// the same way however often it is asked. Debug builds check it.
macro_rules! states_mut {
    ($builder:expr) => {{
        debug_assert!($builder.measuring.get() == 0, "a measure changed the state store");
        $builder.states.borrow_mut()
    }};
}
#[path = "horizontal_scroll_tree.rs"]
mod horizontal_scroll_tree;
use horizontal_scroll_tree::{HorizontalPan, HorizontalScrollState};
#[path = "corner_mask_tree.rs"]
mod corner_mask_tree;
#[path = "plot_tree.rs"]
mod plot_tree;
#[path = "zoom_view_tree.rs"]
mod zoom_view_tree;
#[cfg(test)]
#[path = "popover_initial_focus_tests.rs"]
mod popover_initial_focus_tests;
#[cfg(test)]
#[path = "region_focus_tests.rs"]
mod region_focus_tests;
#[cfg(test)]
#[path = "spatial_focus_tests.rs"]
mod spatial_focus_tests;

#[cfg(test)]
#[path = "drag_drop_tests.rs"]
mod drag_drop_tests;
#[cfg(test)]
#[path = "host_input_tests.rs"]
mod host_input_tests;
#[cfg(test)]
#[path = "hover_popover_tests.rs"]
mod hover_popover_tests;
#[cfg(test)]
#[path = "sticky_tests.rs"]
mod sticky_tests;
#[cfg(test)]
#[path = "top_fade_tests.rs"]
mod top_fade_tests;
#[cfg(test)]
#[path = "text_gradient_tests.rs"]
mod text_gradient_tests;
#[cfg(test)]
#[path = "text_effect_tests.rs"]
mod text_effect_tests;
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
#[path = "keyboard_tests.rs"]
mod keyboard_tests;
#[cfg(test)]
#[path = "soft_focus_tests.rs"]
mod soft_focus_tests;
#[cfg(test)]
#[path = "animate_move_tests.rs"]
mod animate_move_tests;
#[cfg(test)]
#[path = "hover_motion_tests.rs"]
mod hover_motion_tests;
#[cfg(test)]
#[path = "stack_tests.rs"]
mod stack_tests;
#[cfg(test)]
#[path = "hover_layer_tests.rs"]
mod hover_layer_tests;
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

use crate::element::{CoachAlign, CoachAnchor, CoachSide, HoverReveal, Kept, MeasuredRows, UniformList};
use crate::motion::{
    skeleton_color, spinner_angle, SKELETON_PERIOD, ColorTransition, ScalarSpring, ScalarTransition,
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
#[path = "state_store.rs"]
mod state_store;
use state_store::StateStore;
#[path = "inline_menu_navigation.rs"]
mod inline_menu_navigation;
use inline_menu_navigation::InlineMenuFocusScope;
#[path = "context_menu_tree.rs"]
mod context_menu_tree;
use context_menu_tree::{ContextMenuRegion, ContextMenuState};
#[path = "ime_tree.rs"]
mod ime_tree;
pub use ime_tree::ImeArea;
#[path = "captured_paste.rs"]
mod captured_paste;
pub use captured_paste::PasteTarget;
#[path = "draw_command.rs"]
mod draw_command;
pub use draw_command::DrawCommand;
#[path = "events.rs"]
mod events;
pub use events::{Key, Modifiers, OtherButton, UiAction, UiEvent};
#[path = "frame.rs"]
mod frame;
pub use frame::{Frame, HitKind, HitRegion, TextLayoutBlock, TextLayoutRun};
#[path = "input_state.rs"]
mod input_state;
use input_state::{FieldLayout, InputState, VisualLine};
#[path = "formatted_text.rs"]
mod formatted_text;
use formatted_text::{clamp_formatted, layout_formatted, FormattedFragment, FormattedLine};
#[path = "tooltip_tree.rs"]
mod tooltip_tree;
use crate::typography::{truncate_measured, wrap_measured};
use crate::{
    GlassMaterial, BoxShadow, Color, CornerRadii, Cursor, Easing, Element, ElementKind, FontFamily, Icon, InlineImage, Point, PopoverPlacement,
    RasterImage, Rect, ScrollMotion, Size, SpinnerVariant, Style, TextEffect, TextHint, TextSpan, TooltipItem,
    TooltipOptions, TooltipPlacement, TooltipSection, TooltipStyle, DEFAULT_FONT_SIZE, ICON_SIZE,
};

const TOOLTIP_HEIGHT: f32 = 24.0;
/// How far a [`Element::rise_on_hover`] element travels up as it appears.
const REVEAL_RISE: f32 = 8.0;
/// Between a library-shaped hint and its anchor: its 5 px pointer and 2 px.
const TOOLTIP_GAP: f32 = 7.0;
#[doc(hidden)]
pub const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(350);
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

/// What `kept` has a text inked in `ink` paint under its glyphs, the shadow
/// first, their alpha scaled by the ink's.
fn text_effects(kept: Option<&Kept>, ink: Color) -> Vec<TextEffect> {
    let Some(kept) = kept else {
        return Vec::new();
    };
    let faded = |color: Color| Color::rgba(color.r, color.g, color.b, (f32::from(color.a) * f32::from(ink.a) / 255.0).round() as u8);
    [kept.text_shadow, kept.text_stroke]
        .into_iter()
        .flatten()
        .map(|effect| match effect {
            TextEffect::Stroke { width, color } => TextEffect::Stroke { width, color: faded(color) },
            TextEffect::Shadow { offset, blur, color } => TextEffect::Shadow { offset, blur, color: faded(color) },
        })
        .collect()
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
        angle: spinner_angle(started, now, reduced_motion, variant.period()),
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
/// A spring as aurora's `Motion.qml` gives one, SwiftUI's: perceptual duration in seconds and
/// bounce; ω = 2π / duration, ζ = 1 − bounce.
fn aurora_spring(duration: f32, bounce: f32) -> crate::SpringMotion {
    let omega = std::f32::consts::TAU / duration;
    crate::SpringMotion { stiffness: omega * omega, damping_ratio: 1.0 - bounce }
}

/// A bottom sheet's spring: back to its place when let go short, and to a new height when
/// what it holds changes; a little bounce, as iOS's sheets.
fn sheet_motion() -> crate::SpringMotion {
    aurora_spring(0.45, 0.12)
}
/// How far down a sheet's handle takes it before it leaves when let go: this share of its
/// height, between these.
const SHEET_AWAY: f32 = 0.3;
const SHEET_AWAY_LEAST: f32 = 60.0;
const SHEET_AWAY_MOST: f32 = 180.0;
/// How fast a flick down, in points a second, sends a sheet away however short.
const SHEET_FLING: f32 = 700.0;
/// How deep an iOS sheet's rim bends what is behind it.
const SHEET_BEZEL: f32 = 12.0;
/// How far below its large height an iOS sheet starts to push back what it was opened over.
const RECEDE_FROM: f32 = 40.0;
/// How far past the foot of the screen a sheet let go goes before it is gone.
const SHEET_EXIT_PAST: f32 = 24.0;
/// How far a sheet stretches up at most, held by its handle past its place.
const SHEET_STRETCH: f32 = 48.0;

/// Pushes back the scene painted before command `end`, `depth` of the way to iOS's receded
/// card: smaller about the screen's middle, its corners rounded as the display's are.
fn recede_scene(commands: &mut Vec<DrawCommand>, end: usize, depth: f32, corner: f32, viewport: Size) {
    let end = end.min(commands.len());
    let scale = 1.0 - (1.0 - crate::sheet_geometry::RECEDE_SCALE) * depth;
    let origin = Point::new(viewport.width / 2.0, viewport.height / 2.0);
    for command in &mut commands[..end] {
        animate_popover_command(command, 1.0, 0.0, scale, origin);
    }
    let card = Rect::new(origin.x - viewport.width * scale / 2.0, origin.y - viewport.height * scale / 2.0, viewport.width * scale, viewport.height * scale);
    let radius = (corner - crate::sheet_geometry::FLOAT_INSET).max(10.0) * depth;
    commands.insert(end, DrawCommand::PopClip);
    commands.insert(0, DrawCommand::RoundClip { rect: card, radii: CornerRadii { top_left: radius, top_right: radius, bottom_right: radius, bottom_left: radius } });
    commands.insert(0, DrawCommand::PushClip(card));
}

/// The finger's way down as the sheet follows it: all of it down, and up a rubber band's,
/// less the further it pulls, never past `SHEET_STRETCH`.
fn rubber_band(down: f32) -> f32 {
    if down >= 0.0 { return down; }
    let pulled = -down;
    -SHEET_STRETCH * (1.0 - 1.0 / (pulled * 0.55 / SHEET_STRETCH + 1.0))
}

/// How far past its tab strip a lifted selection lens stands, top and bottom together.
const LENS_LIFT: f32 = 9.0;
/// How closely a held tab bar's lens follows the finger: tight, a hair behind, no bounce.
const SCRUB_SPRING: crate::SpringMotion = crate::SpringMotion { stiffness: 900.0, damping_ratio: 0.85 };
/// The speed, in points a second, at which a dragged lens stretches along its way by a
/// tenth, as iOS's drop of glass does; it stretches by a fifth at most.
const SCRUB_STRETCH: f32 = 2000.0;

/// The curve a popover's pictures travel on from their origin, and back on close.
const MORPH_CURVE: Easing = Easing::CubicBezier(0.32, 0.72, 0.0, 1.0);

/// Moves a picture `t` of the way from `origin` to where it is laid out, its
/// corner radius from `radius` to its own. Other commands are left alone.
fn morph_picture(command: &mut DrawCommand, origin: Rect, radius: f32, t: f32) {
    if let DrawCommand::RasterImage { rect, radius: corner, .. }
        | DrawCommand::Glass { rect, radius: corner, .. }
        | DrawCommand::GlassSelection { rect, radius: corner, .. } = command {
        let lerp = |from: f32, to: f32| from + (to - from) * t;
        *rect = Rect::new(
            lerp(origin.x, rect.x),
            lerp(origin.y, rect.y),
            lerp(origin.width, rect.width),
            lerp(origin.height, rect.height),
        );
        *corner = lerp(radius, *corner);
    }
}

/// How a surface the engine paints itself comes in: the scale it grows from about
/// `origin` (the middle of its anchor's bottom unless set) and how long it takes, the
/// tokens' `enter_ms` unless set.
#[derive(Clone, Copy)]
struct SurfaceEntrance {
    scale: f32,
    ms: Option<u16>,
    origin: Option<Point>,
    easing: Easing,
}

impl Default for SurfaceEntrance {
    fn default() -> Self {
        Self {
            scale: 1.0,
            ms: None,
            origin: None,
            easing: Easing::default(),
        }
    }
}

/// Dims a command's colours to `share` of their alpha, in its place.
fn liquid_fade(command: &mut DrawCommand, share: f32) {
    let place = command.bounds().map_or(Point::new(0.0, 0.0), |rect| Point::new(rect.x, rect.y));
    animate_popover_command(command, share, 0.0, 1.0, place);
}

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
        DrawCommand::TextEffect { effect, runs } => {
            match effect {
                crate::TextEffect::Stroke { width, color } => {
                    *width *= scale;
                    fade(color, opacity);
                }
                crate::TextEffect::Shadow { offset, blur, color } => {
                    offset.x *= scale;
                    offset.y *= scale;
                    *blur *= scale;
                    fade(color, opacity);
                }
            }
            for run in runs {
                animate_popover_command(run, opacity, slide_y, scale, origin);
            }
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
        DrawCommand::GlassSelection { rect, radius, opacity: own_opacity, backdrop_rect, backdrop, .. } => {
            scale_rect(rect, scale, origin, slide_y);
            scale_rect(backdrop_rect, scale, origin, slide_y);
            *radius *= scale;
            *own_opacity *= opacity.clamp(0.0, 1.0);
            for command in backdrop { animate_popover_command(command, 1.0, slide_y, scale, origin); }
        }
        DrawCommand::Glass { rect, radius, opacity: own_opacity, .. } => {
            scale_rect(rect, scale, origin, slide_y);
            *radius *= scale;
            *own_opacity *= opacity.clamp(0.0, 1.0);
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
            radius,
            ..
        } => {
            scale_rect(rect, scale, origin, slide_y);
            *own_opacity *= opacity.clamp(0.0, 1.0);
            *radius *= scale;
        }
        DrawCommand::EffortSlider {
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
        | DrawCommand::Glass { rect: painted, .. }
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
        | DrawCommand::Glass { rect: painted, .. }
        | DrawCommand::FillRect { rect: painted, .. }
        | DrawCommand::StrokeRect { rect: painted, .. }
        | DrawCommand::FillCornerRect { rect: painted, .. }
        | DrawCommand::CornerMask { rect: painted, .. }
        | DrawCommand::StrokeCornerRect { rect: painted, .. } => *painted = rect,
        _ => {}
    }
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

/// Out of sight, state is kept only for the IDs laid out most recently: a tab
/// switched away from keeps its scroll and caret, while IDs that come and go,
/// one per message, do not pile up.
const DORMANT_KEPT: usize = 64;

/// The layout each ID of a retained map was last laid out in.
#[derive(Default)]
struct LastLaidOut {
    layout: u64,
    inputs: HashMap<String, u64>,
    scrolls: HashMap<String, u64>,
    measured_lists: HashMap<String, u64>,
    horizontal_scrolls: HashMap<String, u64>,
}

/// Stamp the IDs of `map` that `layout` used, then forget the dormant ones laid
/// out longest ago, beyond the `DORMANT_KEPT` most recent.
fn keep_recent<'a, V>(
    map: &mut HashMap<String, V>,
    stamps: &mut HashMap<String, u64>,
    used: impl Iterator<Item = &'a str>,
    layout: u64,
) {
    for id in used {
        if let Some(stamp) = stamps.get_mut(id) {
            *stamp = layout;
        } else if map.contains_key(id) {
            stamps.insert(id.to_owned(), layout);
        }
    }
    stamps.retain(|id, _| map.contains_key(id));
    let mut dormant: Vec<(u64, &String)> = map
        .keys()
        .map(|id| (stamps.get(id).copied().unwrap_or(0), id))
        .filter(|(stamp, _)| *stamp != layout)
        .collect();
    if dormant.len() <= DORMANT_KEPT {
        return;
    }
    dormant.sort_unstable();
    let gone: Vec<String> = dormant[..dormant.len() - DORMANT_KEPT].iter().map(|(_, id)| (*id).clone()).collect();
    for id in gone {
        map.remove(&id);
        stamps.remove(&id);
    }
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct GlassSwitchGeometry {
    bounds: Rect,
    checked: bool,
}

struct GlassSwitchDrag {
    origin: Point,
    geometry: GlassSwitchGeometry,
    moved: bool,
}

#[derive(Debug)]
struct TabIndicatorState {
    selected_id: String,
    // Springs, so an indicator redirected mid-flight keeps its speed instead
    // of stopping and starting again.
    x: ScalarSpring,
    width: ScalarSpring,
    lift_x: ScalarSpring,
    lift_y: ScalarSpring,
    material_presence: ScalarSpring,
    travel_from: f32,
    press_start: Option<(f32, f32)>,
    dragging: bool,
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
    morph: Option<(Rect, f32)>,
    view_key: Option<String>,
    rect: Rect,
    slide_sign: f32,
    /// The dimming behind a dialog; it fades with the surface.
    scrim: Option<(Rect, Color)>,
    /// How far down a bottom sheet stood from its place, held or springing: let go there,
    /// it leaves down from it.
    sheet_offset: f32,
    /// An iOS sheet's way down past the foot of the screen, which it goes when it closes
    /// unless it melts back into the button it grew out of.
    sheet_exit: Option<f32>,
    /// How far an iOS sheet had pushed back what it was opened over, 0 to 1.
    recede: f32,
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
}

/// A value the tree follows for a slot of `LevelBars`.
struct AnimatedSpring {
    value: ScalarSpring,
}

/// An `animate_move` element: its slot from its parent's origin, and how far
/// from it it is painted on its way there.
struct AnimatedMove {
    slot: Point,
    x: ScalarTransition,
    y: ScalarTransition,
}

/// The ink of an `animate_foreground` element on its way to its color.
struct Ink(ColorTransition);

/// A control's fill on its way to its hover or press color.
struct Fill(ColorTransition);

/// When a skeleton started to pulse.
struct SkeletonStart(Instant);

/// When a shimmering text's highlight started to travel.
struct ShimmerStart(Instant);

/// When a spinner started to turn, or a scribble to be written.
struct SpinnerStart(Instant);

/// The slots a `keep_leaving` column last laid out, in order.
struct Leaving(Vec<String>);

/// Where an element's hover motion stands: since `since` it has played from
/// `from` while hovered, or returned from it to rest while not. Kept while
/// its element is laid out.
struct IconMotionState {
    hovered: bool,
    since: Instant,
    from: crate::motion::IconPose,
    /// When it comes to rest or to its last keyframe.
    settles: Instant,
    /// An `icon_loop`'s, which plays since `since` and never settles.
    looping: bool,
}

impl IconMotionState {
    fn pose(&self, motion: &crate::IconMotion, now: Instant) -> crate::motion::IconPose {
        let elapsed = now.saturating_duration_since(self.since).as_secs_f32() * 1000.0;
        motion.pose(self.from, self.hovered, elapsed)
    }
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
/// Where a slot's spring and content height stand at `now`, from where they
/// stood before, once it shows content `natural` high: the transition starts
/// or reverses when the content arrives or leaves. Content already there when
/// the slot is first seen is simply there.
fn next_flow(before: Option<(ScalarSpring, f32)>, expand: bool, natural: f32, now: Instant, locating: bool) -> (ScalarSpring, f32) {
    let present = natural > 0.0;
    let (mut fraction, mut kept) = before.unwrap_or((slot_spring(if present { SLOT_OPEN } else { 0.0 }, now), natural));
    let target = if present {
        kept = natural;
        SLOT_OPEN
    } else {
        0.0
    };
    if fraction.target() != target {
        if present && (!expand || locating) && fraction.value(now) <= 0.0 {
            fraction = slot_spring(SLOT_OPEN, now);
        } else {
            fraction.launch(target, slot_motion(), now);
        }
    }
    (fraction, kept)
}

#[doc(hidden)]
pub fn slot_settle() -> Duration {
    Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3 / 2)
}

struct FlowHeight {
    fraction: ScalarSpring,
    natural: f32,
    rect: Rect,
    commands: Vec<DrawCommand>,
}

struct PopoverExit {
    paint: PopoverSnapshot,
    opacity: ScalarTransition,
    /// Opened after every popover still open: it leaves above them, where it stood, not
    /// under a sheet it was opened over.
    on_top: bool,
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
    /// The innermost [`Element::selection_scope`] it is in, by the order the
    /// scopes were laid out.
    scope: Option<usize>,
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
            FontFamily::Monospace | FontFamily::MonospaceBold => metrics,
            FontFamily::Sans
            | FontFamily::SansItalic
            | FontFamily::SansMedium
            | FontFamily::SansSemibold
            | FontFamily::SansBold
            | FontFamily::SansBoldItalic
            | FontFamily::Display
            | FontFamily::Custom(_) => sans_metrics,
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

/// One end of a selection: a text block of the frame, its text when the end
/// was placed, and a character offset in it.
#[derive(Clone, Debug, PartialEq)]
struct TextPoint {
    block: usize,
    source: String,
    offset: usize,
}

impl TextPoint {
    fn at(blocks: &[SelectableTextBlock], block: usize, offset: usize) -> Self {
        Self { block, source: blocks[block].source.clone(), offset }
    }
}

/// Text selected by a drag over static text: within one block, or, inside an
/// [`Element::selection_scope`], from one block to another and every block of
/// the scope between them.
#[derive(Clone, Debug)]
struct StaticSelection {
    /// The selection scope both ends are in; `None` keeps them in one block.
    scope: Option<usize>,
    anchor: TextPoint,
    cursor: TextPoint,
    dragging: bool,
}

impl StaticSelection {
    /// A selection collapsed at `offset` of `block`.
    fn at(blocks: &[SelectableTextBlock], block: usize, offset: usize, dragging: bool) -> Self {
        let point = TextPoint::at(blocks, block, offset);
        Self { scope: blocks[block].scope, anchor: point.clone(), cursor: point, dragging }
    }

    /// The two ends in the order the blocks were laid out.
    fn ordered(&self) -> (&TextPoint, &TextPoint) {
        let (a, c) = (&self.anchor, &self.cursor);
        if (a.block, a.offset) <= (c.block, c.offset) { (a, c) } else { (c, a) }
    }

    /// Whether both ends are still where they were placed in `blocks`.
    fn holds_in(&self, blocks: &[SelectableTextBlock]) -> bool {
        [&self.anchor, &self.cursor].into_iter().all(|end| {
            blocks.get(end.block).is_some_and(|block| block.source == end.source && block.scope == self.scope)
        })
    }

    /// The characters selected in block `index`, which holds `source` and is
    /// in `scope`. `None` when it has none selected.
    fn range_in(&self, index: usize, scope: Option<usize>, source: &str) -> Option<(usize, usize)> {
        let (first, last) = self.ordered();
        if first.block == last.block {
            return (index == first.block && source == first.source && first.offset != last.offset)
                .then_some((first.offset, last.offset));
        }
        if self.scope.is_none() || scope != self.scope || !(first.block..=last.block).contains(&index) {
            return None;
        }
        let start = if index == first.block { first.offset } else { 0 };
        let end = if index == last.block { last.offset } else { source.chars().count() };
        let end_text = (index == first.block && source != first.source) || (index == last.block && source != last.source);
        (start < end && !end_text).then_some((start, end))
    }

    /// Whether block `index` holds part of the selection or its caret.
    fn touches(&self, index: usize, block: &SelectableTextBlock) -> bool {
        let caret = |end: &TextPoint| end.block == index && end.source == block.source;
        caret(&self.anchor) || caret(&self.cursor) || self.range_in(index, block.scope, &block.source).is_some()
    }
}

/// What goes between two texts copied one after the other: a space when they
/// share a line (a list's marker and its item), a line break when the next is
/// just below, and a blank line when the gap between them is wider than a
/// quarter of a line (between paragraphs).
fn text_break(before: &SelectableTextBlock, after: &SelectableTextBlock) -> &'static str {
    let (Some(above), Some(below)) = (before.runs.last(), after.runs.first()) else {
        return "\n";
    };
    let (above, below) = (above.rect, below.rect);
    let overlap = (above.y + above.height).min(below.y + below.height) - above.y.max(below.y);
    let line = above.height.min(below.height);
    if overlap > line / 2.0 {
        " "
    } else if below.y - (above.y + above.height) > line / 4.0 {
        "\n\n"
    } else {
        "\n"
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

/// How far a scroll element's content runs past its viewport. Layout sums
/// leave millionths of a pixel, so content up to half a pixel taller fits:
/// it reserves no rail and shows no bar.
fn overflow(content: f32, viewport: f32) -> f32 {
    let over = content - viewport;
    if over > 0.5 {
        over
    } else {
        0.0
    }
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

/// Pointer travel that turns a press into a drag instead of a click.
const DRAG_THRESHOLD: f32 = 4.0;

struct PointerDrag {
    id: String,
    origin: Point,
    active: bool,
    /// The source as painted when the drag began, before any translation.
    ghost: Vec<DrawCommand>,
    /// Where the pointer was last seen down the screen and when, and how fast it went
    /// there, in points a second: what a sheet's handle is let go with.
    last: Option<(f32, Instant)>,
    speed: f32,
}

/// What an iOS sheet was laid out as: the heights it rests at, ascending, the height it
/// stood at, the screen's height, and whether a drag may send it away.
#[derive(Clone, Debug)]
struct SheetFrame {
    heights: Vec<f32>,
    resting: usize,
    height: f32,
    viewport: f32,
    dismissible: bool,
}

/// How an iOS sheet is painted this frame: its shape; how much of its smallest height shows,
/// which the dimming follows; how far it goes to leave the screen; and how far it pushes
/// back what it was opened over.
#[derive(Clone, Copy, Debug)]
struct SheetPaint {
    rect: Rect,
    /// Its surface: its content's rectangle, reaching on down behind a keyboard.
    shell: Rect,
    entered: f32,
    leave: f32,
    recede: f32,
}

/// An iOS sheet held: by its grabber, from where the finger went down, or by its content
/// as that pans. `raw` is where the finger has taken it, before its rubber band; `speed`,
/// how fast it grows, in points a second.
#[derive(Clone, Debug)]
struct SheetDrag {
    dismiss: String,
    grabber: Option<(String, f32, f32)>,
    raw: f32,
    height: f32,
    speed: f32,
    last: Instant,
}

/// The line under what an input method composes in a field, a pixel high
/// at the bottom of each of its lines.
fn composition_underline(
    layout: &crate::TextLayout,
    range: std::ops::Range<usize>,
    origin: Point,
    ink: Color,
) -> impl Iterator<Item = DrawCommand> {
    layout.selection(range).into_iter().map(move |bounds| DrawCommand::FillRect {
        rect: Rect::new(origin.x + bounds.x, origin.y + bounds.y + bounds.height - 1.0, bounds.width, 1.0),
        color: ink,
        radius: 0.0,
    })
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

/// The bands a gradient ink is painted in across `rect`: two dozen slices of it, each inside
/// `clip`, in its colour blended from `stops` spread evenly from the left to the right, its
/// alpha scaled by `opacity`.
fn gradient_bands(rect: Rect, stops: &[Color], opacity: f32, clip: Option<Rect>) -> Vec<(Rect, Color)> {
    const BANDS: usize = 24;
    let last = (stops.len() - 1) as f32;
    (0..BANDS)
        .filter_map(|band| {
            let left = rect.x + rect.width * band as f32 / BANDS as f32;
            let right = rect.x + rect.width * (band + 1) as f32 / BANDS as f32;
            let area = Rect::new(left, rect.y, right - left, rect.height);
            let area = clip.map_or(Some(area), |outer| area.intersection(outer)).filter(|area| area.width > 0.0 && area.height > 0.0)?;
            let at = (band as f32 + 0.5) / BANDS as f32 * last;
            let from = stops[at.floor() as usize];
            let to = stops[(at.ceil() as usize).min(stops.len() - 1)];
            let mix = at.fract();
            let blend = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * mix).round() as u8;
            let color = Color::rgba(blend(from.r, to.r), blend(from.g, to.g), blend(from.b, to.b), (f32::from(blend(from.a, to.a)) * opacity).round() as u8);
            Some((area, color))
        })
        .collect()
}

/// Of `candidates`, the one a press of `key` moves the focus to from `from`,
/// as a remote's arrows do: one ahead of it in that direction, those in its
/// lane first (overlapping it across the direction), then the nearest by how
/// far ahead it is and twice how far it stands to the side, the most in line
/// among equals. Out of its lane Up and Down still reach the nearest, as from
/// a bar down into a short shelf, but Left and Right only what stands less
/// than its own height off, so the end of a row does not drop into the next.
fn nearest_in_direction<'a>(from: Rect, key: Key, candidates: impl Iterator<Item = (&'a str, Rect)>) -> Option<&'a str> {
    // How far apart two spans are along one axis; 0 where they overlap.
    let apart = |a: (f32, f32), b: (f32, f32)| (b.0 - a.1).max(a.0 - b.1).max(0.0);
    let (across_x, across_y) = ((from.x, from.x + from.width), (from.y, from.y + from.height));
    let centre = |rect: Rect| (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let (fx, fy) = centre(from);
    let sideways = matches!(key, Key::ArrowLeft | Key::ArrowRight);
    candidates
        .filter_map(|(id, to)| {
            let (tx, ty) = centre(to);
            let (ahead, aside, offset) = match key {
                Key::ArrowDown => (to.y - (from.y + from.height), apart(across_x, (to.x, to.x + to.width)), (tx - fx).abs()),
                Key::ArrowUp => (from.y - (to.y + to.height), apart(across_x, (to.x, to.x + to.width)), (tx - fx).abs()),
                Key::ArrowRight => (to.x - (from.x + from.width), apart(across_y, (to.y, to.y + to.height)), (ty - fy).abs()),
                Key::ArrowLeft => (from.x - (to.x + to.width), apart(across_y, (to.y, to.y + to.height)), (ty - fy).abs()),
                _ => return None,
            };
            // Past its edge, give or take a hairline of overlap.
            let ahead_of_it = ahead >= -1.0;
            let reachable = aside == 0.0 || !sideways || aside < from.height;
            (ahead_of_it && reachable).then_some((id, aside > 0.0, ahead.max(0.0) + 2.0 * aside + 0.01 * offset))
        })
        .min_by(|a, b| a.1.cmp(&b.1).then(a.2.total_cmp(&b.2)))
        .map(|(id, ..)| id)
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

/// Whether `id` is one a code block made for itself, having none of its own:
/// its horizontal scroll belongs to the tree it was built in, not to the next.
fn is_anonymous_code_scroll_id(id: &str) -> bool {
    id.strip_prefix("markdown:").unwrap_or(id).starts_with("code_block:auto:")
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
    /// The faces' generation the measured widths were taken with: a face
    /// given to `limn::set_fonts`, as a host installs one, measures afresh.
    measured_fonts: u64,
    /// Whether `Element::text` is laid out in a `TextLayout`; see
    /// [`UiTree::set_text_layout`].
    text_layout: bool,
    measurement_cache: RefCell<MeasurementCache>,
    horizontal_scrolls: HashMap<String, HorizontalScrollState>,
    /// When the IDs of each map that outlives its controls were last laid out.
    last_laid_out: LastLaidOut,
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
    glass_switch_drag: Option<GlassSwitchDrag>,
    sketch_draft: Option<crate::sketch::SketchDraft>,
    hovered_scrollbar: Option<String>,
    focus_order: Vec<(String, HitKind)>,
    /// Each landmark region and its controls in focus order, outermost first.
    focus_regions: Vec<(String, Vec<String>)>,
    action_aliases: HashMap<String, String>,
    focused: Option<String>,
    caret_blink: crate::caret_blink::CaretBlink,
    modal_return_focus: Option<String>,
    /// Whether the focus a modal hands back came from the keyboard when the modal opened: the
    /// key that closes it (Esc) does not make a pointer's focus the keyboard's.
    modal_return_keyboard: bool,
    pending_scroll_focus: Option<String>,
    /// A control asked to be focused before a layout held it, and whether by
    /// the keyboard: the next layout focuses it if it holds it.
    pending_focus: Option<(String, bool)>,
    /// The control to bring into view, its margin, and whether it goes to
    /// the start of the viewport instead of just into it.
    pending_reveal: Option<(String, f32, bool)>,
    pending_center: Option<String>,
    hovered: Option<String>,
    /// The named `hover_region` under the pointer as of the last layout.
    hovered_region: Option<String>,
    pointer: Option<Point>,
    tooltip_hover_since: Option<(String, Instant)>,
    keyboard_focus: bool,
    keyboard_tooltip_focus: bool,
    /// The arrow keys move the focus to the nearest control their way; see
    /// [`UiTree::set_spatial_focus`].
    spatial_focus: bool,
    closed_submenu_triggers: HashSet<String>,
    /// The anchors of the last layout's hints, with how long each waits.
    tooltip_triggers: HashMap<String, Duration>,
    pressed: Option<String>,
    pressed_dispatch: Option<String>,
    pointer_drag: Option<PointerDrag>,
    /// iOS sheets: the height each rests at, by its dismissal, while it is open.
    sheet_detents: HashMap<String, usize>,
    /// What each open iOS sheet was last laid out as, for a drag to start from and let go to.
    sheet_frames: HashMap<String, SheetFrame>,
    /// An iOS sheet held by its grabber or by its content.
    sheet_drag: Option<SheetDrag>,
    popover_transitions: HashMap<String, ScalarTransition>,
    popover_snapshots: HashMap<String, PopoverSnapshot>,
    popover_exits: HashMap<String, PopoverExit>,
    /// The order the open popovers opened in, so one that closes keeps its depth.
    popover_order: HashMap<String, u64>,
    /// Whatever appears in the next layout stands as its entrance would leave it.
    settle_entrances: bool,
    popover_opened: u64,
    /// What elements keep while layouts touch them: slots' heights, values
    /// and springs, moves, inks and fills, leaving slots, replaced content,
    /// tab indicators, records, sliders, and when skeletons, shimmers and
    /// spinners started. Measuring reads and starts some of these, and
    /// measuring borrows the tree shared.
    states: RefCell<StateStore>,
    /// A pushing reach jumped while placed: the layout it was measured for is stale.
    pushes_moved: std::cell::Cell<bool>,
    popover_view_transitions: HashMap<String, PopoverViewTransition>,
    coach_motion: Option<CoachMotion>,
    /// The coach mark step whose target has been revealed and focused.
    coach_key: Option<String>,
    /// Scribbles written once whose pen has gone: their start is kept, and they ask for no
    /// frames.
    settled_spinners: HashSet<String>,
    reduced_motion: bool,
    static_selection: Option<StaticSelection>,
    context_menu: Option<ContextMenuState>,
    /// Where a double tap selected a word on a touch screen: its edit menu opens as the finger
    /// lifts, as iOS's does.
    edit_menu_on_release: Option<Point>,
    context_menu_palette: crate::ComponentPalette,
    context_menu_sizes: crate::context_menu::Sizes,
    /// Text menus drawn as iOS 26's edit menu, in this glass; see `set_touch_text_menu`.
    touch_text_menu: Option<crate::GlassMaterial>,
    context_menu_swallow_release: bool,
    ime_owner: Option<ime_tree::ImeOwner>,
    /// The field a held press extends the selection of, and whether it is
    /// a text area.
    field_drag: Option<(String, bool)>,
    ime_window_focused: bool,
    keys_held: Vec<Key>,
    modifiers: Modifiers,
    /// Where each zoom view's picture is, by the view's ID.
    zoom_views: HashMap<String, crate::zoom_view::ZoomState>,
    zoom_press: Option<zoom_view_tree::ZoomPress>,
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
            measured_fonts: crate::fonts::generation(),
            text_layout: false,
            measurement_cache: RefCell::new(MeasurementCache::default()),
            horizontal_scrolls: HashMap::new(),
            last_laid_out: LastLaidOut::default(),
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
            glass_switch_drag: None,
            sketch_draft: None,
            hovered_scrollbar: None,
            focus_order: Vec::new(),
            focus_regions: Vec::new(),
            action_aliases: HashMap::new(),
            focused: None,
            caret_blink: crate::caret_blink::CaretBlink::default(),
            modal_return_focus: None,
            modal_return_keyboard: false,
            pending_scroll_focus: None,
            pending_focus: None,
            pending_reveal: None,
            pending_center: None,
            hovered: None,
            hovered_region: None,
            pointer: None,
            tooltip_hover_since: None,
            keyboard_focus: false,
            keyboard_tooltip_focus: false,
            spatial_focus: false,
            closed_submenu_triggers: HashSet::new(),
            tooltip_triggers: HashMap::new(),
            pressed: None,
            pressed_dispatch: None,
            pointer_drag: None,
            sheet_detents: HashMap::new(),
            sheet_frames: HashMap::new(),
            sheet_drag: None,
            popover_transitions: HashMap::new(),
            popover_snapshots: HashMap::new(),
            popover_exits: HashMap::new(),
            popover_order: HashMap::new(),
            settle_entrances: false,
            popover_opened: 0,
            states: RefCell::new(StateStore::default()),
            pushes_moved: std::cell::Cell::new(false),
            popover_view_transitions: HashMap::new(),
            coach_motion: None,
            coach_key: None,
            settled_spinners: HashSet::new(),
            reduced_motion: false,
            static_selection: None,
            context_menu: None,
            context_menu_palette: crate::ComponentPalette::neutral_dark(),
            touch_text_menu: None,
            edit_menu_on_release: None,
            context_menu_sizes: crate::context_menu::Sizes::default(),
            context_menu_swallow_release: false,
            ime_owner: None,
            field_drag: None,
            ime_window_focused: true,
            keys_held: Vec::new(),
            modifiers: Modifiers::default(),
            zoom_views: HashMap::new(),
            zoom_press: None,
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
        self.horizontal_scrolls.retain(|id, _| !is_anonymous_code_scroll_id(id));
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
        self.set_input_selection(id, cursor, cursor);
    }

    /// Select the Unicode characters `start..end` of a text input or text
    /// area without changing focus or replacing text: the anchor at `start`
    /// and the caret at `end`, each kept within the text. Equal ends leave
    /// only the caret there. A composer uses this to keep words selected
    /// after wrapping them in markers. A field not laid out yet is left alone.
    pub fn set_input_selection(&mut self, id: &str, start: usize, end: usize) {
        if let Some(state) = self.inputs.get_mut(id) {
            let count = state.value.chars().count();
            let cursor = end.min(count);
            let anchor = Some(start.min(count)).filter(|anchor| *anchor != cursor);
            let current = state.anchor.filter(|anchor| *anchor != state.cursor);
            if self.focused.as_deref() == Some(id) && (state.cursor, current) != (cursor, anchor) {
                self.caret_blink.reset();
            }
            state.set_selection(cursor, anchor);
            state.keep_cursor_visible = true;
            state.desired_column = None;
            state.desired_x = None;
        }
    }

    /// Text selected by dragging over static text: one text or formatted-text
    /// block, or the blocks of an [`Element::selection_scope`] it runs over,
    /// joined as that method says. Each block's slice preserves its source
    /// newlines and Unicode boundaries.
    pub fn selected_text(&self) -> Option<String> {
        let selection = self.static_selection.as_ref()?;
        let (first, last) = selection.ordered();
        let mut text = String::new();
        let mut previous: Option<&SelectableTextBlock> = None;
        for (index, block) in self.frame.text_blocks.iter().enumerate().take(last.block + 1).skip(first.block) {
            let Some((start, end)) = selection.range_in(index, block.scope, &block.source) else {
                continue;
            };
            if let Some(previous) = previous {
                text.push_str(text_break(previous, block));
            }
            text.push_str(char_range(&block.source, start, end));
            previous = Some(block);
        }
        (!text.is_empty()).then_some(text)
    }

    /// Where `point` falls for `selection`'s moving end: in its block, or,
    /// within a scope, in the scope's block under it or else nearest to it,
    /// at the start of a block below the point and the end of one above it.
    fn selection_point(&self, selection: &StaticSelection, point: Point) -> Option<(usize, usize)> {
        let offset = |block: &SelectableTextBlock| {
            block.offset_at(point, self.text_measurer.as_deref(), self.metrics, self.sans_metrics)
        };
        let Some(scope) = selection.scope else {
            let end = &selection.cursor;
            let block = self.frame.text_blocks.get(end.block).filter(|block| block.source == end.source)?;
            return Some((end.block, offset(block)));
        };
        let (index, block) = self
            .frame
            .text_blocks
            .iter()
            .enumerate()
            .filter(|(_, block)| block.scope == Some(scope) && !block.runs.is_empty())
            .min_by(|(_, left), (_, right)| {
                let distance = |block: &SelectableTextBlock| {
                    let rows = block.runs.iter().map(|run| (vertical_distance(run.rect, point.y), horizontal_distance(run.rect, point.x)));
                    rows.fold((f32::INFINITY, f32::INFINITY), |best, row| if row < best { row } else { best })
                };
                distance(left).partial_cmp(&distance(right)).unwrap_or(std::cmp::Ordering::Equal)
            })?;
        let top = block.runs.first().map_or(0.0, |run| run.rect.y);
        let bottom = block.runs.last().map_or(0.0, |run| run.rect.y + run.rect.height);
        Some((
            index,
            if point.y < top {
                0
            } else if point.y > bottom {
                block.source.chars().count()
            } else {
                offset(block)
            },
        ))
    }

    /// Move the dragged selection's end to `point`; `still` keeps the drag on.
    fn drag_selection(&mut self, point: Point, still: bool) {
        let Some(selection) = self.static_selection.as_ref().filter(|selection| selection.dragging) else {
            return;
        };
        let end = self
            .selection_point(selection, point)
            .map(|(block, offset)| TextPoint::at(&self.frame.text_blocks, block, offset));
        if let Some(selection) = self.static_selection.as_mut() {
            if let Some(end) = end {
                selection.cursor = end;
            }
            selection.dragging = still;
        }
    }

    /// Select every text of the scope the selection is in, or all of its
    /// one block. Whether there was a selection to grow.
    fn select_all_text(&mut self) -> bool {
        let Some(selection) = self.static_selection.as_mut() else {
            return false;
        };
        let blocks = &self.frame.text_blocks;
        let mut within = blocks.iter().enumerate().filter(|(_, block)| block.scope == selection.scope).map(|(index, _)| index);
        let (first, last) = match selection.scope {
            Some(_) => {
                let Some(first) = within.next() else { return false };
                (first, within.next_back().unwrap_or(first))
            }
            None => (selection.anchor.block, selection.anchor.block),
        };
        if last >= blocks.len() {
            return false;
        }
        selection.anchor = TextPoint::at(blocks, first, 0);
        selection.cursor = TextPoint::at(blocks, last, blocks[last].source.chars().count());
        selection.dragging = false;
        true
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

    /// Show what an input method composes in the focused field, or end it.
    fn compose_in_field(&mut self, text: Option<String>) {
        if let Some(input) = self.focused.as_ref().and_then(|id| self.inputs.get_mut(id)) {
            input.preedit = text.filter(|text| !text.is_empty());
            input.keep_cursor_visible = true;
        }
    }

    /// Delete what an input method asks around the focused field's caret.
    fn delete_around_caret(&mut self, before: usize, after: usize) -> Vec<UiAction> {
        let Some(id) = self.focused.clone() else {
            return Vec::new();
        };
        let Some(input) = self.inputs.get_mut(&id) else {
            return Vec::new();
        };
        if !input.delete_around(before, after) {
            return Vec::new();
        }
        vec![UiAction::InputChanged { value: input.value.clone(), id }]
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
        self.pending_focus = None;
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

    /// The ID of the innermost [`Element::hover_region_id`] under the
    /// pointer as of the last layout, such as the row of a message while the
    /// pointer is over its text, which is no control. A popover, a dialog's
    /// backdrop or a context menu over it takes the pointer from it.
    pub fn hovered_region(&self) -> Option<&str> {
        self.hovered_region.as_deref()
    }

    /// The shape the pointer should have: that of the element being dragged,
    /// wherever the pointer went with it, or else of the element under it.
    /// `None` leaves it to the platform.
    pub fn cursor(&self) -> Option<Cursor> {
        let dragged = self.pointer_drag.as_ref().filter(|drag| drag.active).map(|drag| drag.id.as_str());
        self.frame.cursors.get(dragged.or(self.hovered.as_deref())?).copied()
    }

    /// The controls inside the landmark region `region` (an element marked
    /// `Access::new(Role::Region).id(region)`), in focus order, as of the
    /// last layout. Empty for an ID that is not a region.
    pub fn focusables_within(&self, region: &str) -> Vec<&str> {
        self.focus_regions
            .iter()
            .find(|(id, _)| id == region)
            .map(|(_, members)| members.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Move the focus to the first control of the next landmark region, or
    /// of the previous one with `reverse`, wrapping around, as F6 and
    /// Shift+F6 do. Regions follow the order they are in the tree, a nested
    /// one after the region around it; one with nothing to focus is passed
    /// over. Returns the ID of the region reached, for a host that would
    /// rather land on another of its controls; `None` when a popover holds
    /// the focus or no region has a control.
    pub fn focus_next_region(&mut self, reverse: bool) -> Option<String> {
        if self.has_open_popover() {
            return None;
        }
        let reachable: Vec<usize> =
            (0..self.focus_regions.len()).filter(|index| !self.focus_regions[*index].1.is_empty()).collect();
        let focused = self.focused.as_deref();
        // The innermost region the focus is in: nested regions come later.
        let current = focused.and_then(|id| {
            reachable.iter().rposition(|index| self.focus_regions[*index].1.iter().any(|member| member == id))
        });
        let count = reachable.len();
        let next = match (current, reverse) {
            _ if count == 0 => return None,
            (Some(at), false) => (at + 1) % count,
            (Some(at), true) => (at + count - 1) % count,
            (None, false) => 0,
            (None, true) => count - 1,
        };
        let (region, members) = &self.focus_regions[reachable[next]];
        let (region, first) = (region.clone(), members[0].clone());
        self.focus(&first).then_some(region)
    }

    /// Let the arrow keys move the focus to the nearest control in their
    /// direction, as a television's remote moves it, when the focused control
    /// does not take them itself: a slider, a tab strip, a menu and a text
    /// field's caret still do, but Up and Down leave a single-line field. With
    /// nothing focused, an arrow focuses the first control. Off by default,
    /// where the arrows only reach the focused control.
    pub fn set_spatial_focus(&mut self, enabled: bool) {
        self.spatial_focus = enabled;
    }

    /// Focus an interactive element. True when the last layout held it and it
    /// has the focus now. Otherwise false, and the next layout focuses it if it
    /// holds it, so a host may focus a control its new view adds before laying
    /// that view out; a later `focus` or `clear_focus` replaces the request.
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
        if !takes_focus(&self.focus_order, id) {
            self.pending_focus = Some((id.to_owned(), keyboard));
            return false;
        }
        self.pending_focus = None;
        self.give_focus(id, keyboard);
        true
    }

    fn give_focus(&mut self, id: &str, keyboard: bool) {
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
    }

    /// Have whatever appears in the next layout stand as its entrance would leave it, without
    /// fading or settling in: for a host showing again what it had just drawn, such as a page
    /// uncovered by a sheet the host moved itself, whose parts would otherwise enter anew.
    pub fn settle_entrances(&mut self) {
        self.settle_entrances = true;
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
            self.popover_transitions.clear();
            self.popover_snapshots.clear();
            self.popover_exits.clear();
            self.states.get_mut().clear();
            self.popover_order.clear();
            self.popover_view_transitions.clear();
            self.coach_motion = None;
            let now = Instant::now();
            for state in self.zoom_views.values_mut() {
                state.jump(state.view, now);
            }
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
    /// Losing the focus also lets go of every key and modifier held, whose
    /// release the window may never see.
    pub fn set_window_focused(&mut self, focused: bool) -> bool {
        if self.ime_window_focused != focused {
            self.invalidate_paste_targets();
        }
        if !focused {
            self.keys_held.clear();
            self.modifiers = Modifiers::default();
        }
        self.ime_window_focused = focused;
        self.validate_ime_owner(Instant::now());
        self.caret_blink.set_window_focused(focused)
    }

    /// Whether `key` is held: from its [`UiEvent::Key`] until its
    /// [`UiEvent::KeyReleased`], or the window losing the focus. Asked before
    /// a press is handled, false tells a first press from a key repeating.
    pub fn key_held(&self, key: Key) -> bool {
        self.keys_held.contains(&key)
    }

    /// The modifiers held, as the latest [`UiEvent::Key`] or
    /// [`UiEvent::ModifiersChanged`] said; none once the window loses the
    /// focus.
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Note what `event` says of the keys and modifiers held, without
    /// handling it: for an event the host answers before the tree, which
    /// then never sees it. [`Self::handle_event`] does this itself, and
    /// noting an event twice changes nothing.
    pub fn observe_keys(&mut self, event: &UiEvent) {
        match *event {
            UiEvent::Key {
                key,
                shift,
                ctrl,
                alt,
            } => {
                if !self.keys_held.contains(&key) {
                    self.keys_held.push(key);
                }
                self.modifiers = Modifiers { shift, ctrl, alt };
            }
            UiEvent::KeyReleased { key } => self.keys_held.retain(|held| *held != key),
            UiEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers,
            _ => {}
        }
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

    /// How soon the next frame is needed at `now` for what moves, the
    /// blinking caret left out; `None` at rest.
    #[doc(hidden)]
    pub fn visual_animation_repaint_after_at(&self, now: Instant) -> Option<Duration> {
        let animation = match (self.other_animation_repaint_after_at(now), self.zoom_repaint_after(now)) {
            (Some(other), Some(zoom)) => Some(other.min(zoom)),
            (other, zoom) => other.or(zoom),
        };
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

    /// Whether a state kept for an element is still on its way.
    fn states_animate(&self, now: Instant) -> bool {
        let states = self.states.borrow();
        states.values::<FlowHeight>().any(|state| state.fraction.is_active(now))
            || states.values::<AnimatedScalar>().any(|state| state.value.is_active(now))
            || states.values::<AnimatedSpring>().any(|state| state.value.is_active(now))
            || states.values::<AnimatedMove>().any(|state| state.x.is_active(now) || state.y.is_active(now))
            || states.values::<Ink>().any(|ink| ink.0.is_active(now))
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
        let states = self.states.borrow();
        let ambient = (!self.reduced_motion
            && (states.values::<SkeletonStart>().next().is_some()
                || states.values::<ShimmerStart>().next().is_some()
                || states.ids::<SpinnerStart>().any(|id| !self.settled_spinners.contains(id))
                || states.values::<IconMotionState>().any(|state| state.looping)
                || states.values::<RecordMotion>().any(|record| record.is_moving(now))))
            || self.scroll_button_hold.is_some();
        let transition = (!self.reduced_motion
            && (self.slider_drag.is_some()
                || states.values::<SliderMotion>().any(|state| state.is_fast(now))))
            || states.values::<Fill>().any(|fill| fill.0.is_active(now))
            || self
                .popover_transitions
                .values()
                .any(|state| state.is_active(now))
            || self
                .popover_exits
                .values()
                .any(|state| state.opacity.is_active(now))
            || states.values::<Replacement>().any(|state| state.progress.is_active(now))
            || states.values::<IconMotionState>().any(|state| !state.looping && now < state.settles)
            || self.states_animate(now)
            || self
                .popover_view_transitions
                .values()
                .any(|state| state.opacity.is_active(now) || state.height.is_active(now))
            || self
                .coach_motion
                .as_ref()
                .is_some_and(|state| state.values.iter().any(|value| value.is_active(now)))
            || states.values::<TabIndicatorState>().any(|state| state.x.is_active(now) || state.width.is_active(now) || state.lift_x.is_active(now) || state.lift_y.is_active(now) || state.material_presence.is_active(now))
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
        } else if !self.reduced_motion && states.values::<SliderMotion>().next().is_some() {
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
        self.width_cache.get_mut().begin_pass();
        let fonts = crate::fonts::generation();
        if fonts != self.measured_fonts {
            self.measured_fonts = fonts;
            self.width_cache.get_mut().clear();
            self.formatted_cache.get_mut().clear();
        }
        let viewport = Size::new(viewport.width.max(0.0), viewport.height.max(0.0));
        for state in self.scrollbar_states.values_mut() { state.seen = false; }
        self.caret_blink.begin_frame(self.context_menu.is_none());
        self.repeat_scroll_button(now);
        if self.pending_scroll_focus.as_deref() != self.focused.as_deref() {
            self.pending_scroll_focus = None;
        }
        let previous_popovers = self.frame.popover_focus.clone();
        let mut focus_order = Vec::new();
        let mut seen = crate::FastSet::default();
        let mut duplicate_ids = Vec::new();
        collect_focus_claims(&self.root, &mut focus_order, &mut seen, &mut duplicate_ids);
        // A focus asked for before this tree was laid out lands now, so this
        // layout already paints it and brings it into view.
        if let Some((id, keyboard)) = self.pending_focus.take_if(|(id, _)| takes_focus(&focus_order, id)) {
            self.give_focus(&id, keyboard);
        }
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
        let mut observed_popovers = HashSet::new();
        let mut current_popover_snapshots = HashMap::new();
        let mut observed_tooltips = HashMap::new();
        let mut settled_spinners = HashSet::new();
        let viewport_rect = Rect::new(0.0, 0.0, viewport.width, viewport.height);
        let context_menu_paint = self.context_menu_paint(viewport_rect);
        // Paint sees the pointer only on the layer that held it in the last
        // frame and those above: a floating surface hides it from what is under.
        let pointer_layer = self.pointer.map_or(0, |point| self.frame.layer_at(point));
        // An open menu is modal: the page under it does not answer the pointer.
        let page_pointer = self.pointer.filter(|_| context_menu_paint.is_none());
        self.measurement_cache.get_mut().clear();
        let ghost_range;
        let mut receding: Option<(f32, f32)>;
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
                translated_overflow: crate::FastMap::default(),
                focus_path,
                focus_probe: probe,
                spatial_focus: self.spatial_focus,
                drag_source: self
                    .pointer_drag
                    .as_ref()
                    .filter(|drag| drag.active)
                    .map(|drag| drag.id.as_str()),
                sheet_hold: self.sheet_drag.as_ref().map(|drag| (drag.dismiss.as_str(), drag.height)),
                sheet_detents: &self.sheet_detents,
                sheet_frames: &mut self.sheet_frames,
                recede: None,
                sheet_held: self
                    .pointer_drag
                    .as_ref()
                    .filter(|drag| drag.active)
                    .zip(self.pointer)
                    .and_then(|(drag, pointer)| {
                        let dismiss = drag.id.strip_suffix(":drag")?;
                        Some((dismiss, rubber_band(pointer.y - drag.origin.y), drag.speed))
                    }),
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
                states: &self.states,
                measuring: std::cell::Cell::new(0),
                flow_naturals: RefCell::new(crate::FastMap::default()),
                flow_origin: Point::default(),
                pushes_moved: &self.pushes_moved,
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
                pointer: page_pointer.filter(|_| pointer_layer == 0),
                pointer_layer,
                region_hovered: false,
                hovered_regions: Vec::new(),
                access_above: None,
                slider_drag: self.slider_drag.as_ref(),
                sketch_draft: self.sketch_draft.as_ref(),
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
                selection_scope: None,
                next_selection_scope: 0,
                popover_transitions: &mut self.popover_transitions,
                popover_exits: &self.popover_exits,
                settle_entrances: std::mem::take(&mut self.settle_entrances),
                previous_popover_snapshots: &self.popover_snapshots,
                popover_view_transitions: &mut self.popover_view_transitions,
                coach_motion: &mut self.coach_motion,
                hover_surface_fill: None,
                surface_gaps_lit: self.context_menu.is_none() && self.hovered.is_none(),
                observed_popovers: &mut observed_popovers,
                popover_snapshots: &mut current_popover_snapshots,
                observed_tooltips: &mut observed_tooltips,
                settled_spinners: &mut settled_spinners,
                zoom_views: &mut self.zoom_views,
                reduced_motion: self.reduced_motion,
                locating: probe.is_some(),
                now,
            };
            for state in builder.zoom_views.values_mut() {
                state.seen = false;
            }
            builder.place(&self.root, viewport_rect, None);
            let overlay_start = builder.frame.commands.len();
            let base_regions = builder.frame.regions.len();
            // Deferred painting gives popovers one stable top layer even when
            // their anchor lives inside a clipped scroll container. Nested
            // overlays are appended and painted above their parent overlay.
            while let Some(overlay) = builder.overlays.pop_front() {
                builder.enter_layer(page_pointer);
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
                        morph,
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
                        anchor_outset,
                        clear_of,
                        sheet,
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
                            builder.frame.backdrops.push(builder.frame.regions.len());
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
                        // An iOS sheet: its height from the finger holding it or its spring,
                        // and its shape from that height.
                        let sheet = sheet.filter(|_| placement == PopoverPlacement::Bottom && coach.is_none());
                        let sheet_shape = sheet.as_deref().map(|spec| {
                            builder.sheet_layout(&mut floating, spec, &dismiss_id, viewport, morph.is_some())
                        });
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
                            None => {
                                let [left, top, right, bottom] = anchor_outset;
                                // A parent surface, when one is open, sets the sides instead.
                                let (x, width) = match clear_of
                                    .as_deref()
                                    .and_then(|id| surface_rect(&builder.frame, id))
                                {
                                    Some(surface) => (surface.x, surface.width),
                                    None => (anchor.x - left, anchor.width + left + right),
                                };
                                let grown = Rect::new(
                                    x,
                                    anchor.y - top,
                                    width,
                                    anchor.height + top + bottom,
                                );
                                (
                                    place_popover(grown, measured, viewport_rect, placement, gap),
                                    None,
                                )
                            }
                        };
                        let rect = sheet_shape.map_or_else(|| LayoutBuilder::translated_rect(&floating, rect), |shape| shape.rect);
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
                        // A sheet rises on its spring, not with a popover's fade, unless it
                        // grows out of the button that opened it.
                        let progress = if builder.reduced_motion || animation_ms == 0 || (sheet_shape.is_some() && morph.is_none()) {
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
                                morph,
                                view_key: view_key.clone(),
                                rect,
                                slide_sign,
                                scrim: scrim
                                    .filter(|_| coach.is_none())
                                    .map(|color| (viewport_rect, color)),
                                sheet_offset: 0.0,
                                sheet_exit: None,
                                recede: 0.0,
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
                        let glass_morph = morph.is_some() && builder.frame.commands[command_start..].iter().any(|command| matches!(command, DrawCommand::Glass { .. }));
                        if progress < 1.0 && !glass_morph {
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
                        if let (Some((origin, radius)), true) = (morph, progress < 1.0) {
                            if glass_morph {
                                let pop = crate::liquid::pop(origin, rect, crate::liquid::seconds(progress, animation_ms), false);
                                crate::liquid::apply(&mut builder.frame.commands, command_start, rect, pop, liquid_fade);
                            } else {
                                for command in &mut builder.frame.commands[command_start..] {
                                    morph_picture(command, origin, radius, MORPH_CURVE.sample(progress));
                                }
                            }
                        }
                        if let Some(paint) = sheet_shape {
                            // Over a keyboard its glass goes on down behind it.
                            if paint.shell != rect {
                                for command in &mut builder.frame.commands[bubble_start..] {
                                    if popover_shell_command(command, rect) { morph_popover_shell(command, paint.shell); }
                                }
                            }
                            // Taken down, or rising, it dims the page as much as it shows.
                            if let Some(DrawCommand::FillRect { color, .. }) = scrim_index.and_then(|index| builder.frame.commands.get_mut(index)) {
                                color.a = (f32::from(color.a) * paint.entered).round() as u8;
                            }
                            if let Some(snapshot) = builder.popover_snapshots.get_mut(&dismiss_id) {
                                snapshot.sheet_exit = Some(paint.leave);
                                snapshot.recede = paint.recede;
                            }
                        } else if placement == PopoverPlacement::Bottom && coach.is_none() {
                            let offset = builder.sheet_spring(&dismiss_id, rect, progress);
                            if offset.abs() > 0.1 {
                                // Its shell keeps the foot of the screen when it rises past its
                                // place; what it holds goes with its top, cut by the shell.
                                let shell = Rect::new(rect.x, rect.y + offset, rect.width, rect.height + (-offset).max(0.0));
                                let painted = builder.frame.commands.split_off(bubble_start);
                                for command in painted.iter().filter(|command| popover_shell_command(command, rect)) {
                                    let mut command = command.clone();
                                    morph_popover_shell(&mut command, shell);
                                    builder.frame.commands.push(command);
                                }
                                builder.frame.commands.push(DrawCommand::PushClip(shell));
                                builder.frame.commands.extend(painted.into_iter()
                                    .filter(|command| !popover_shell_command(command, rect))
                                    .map(|command| command.moved(0.0, offset, 1.0)));
                                builder.frame.commands.push(DrawCommand::PopClip);
                                // Taken down, it dims the page less.
                                if let Some(DrawCommand::FillRect { color, .. }) = scrim_index.and_then(|index| builder.frame.commands.get_mut(index)) {
                                    let left = 1.0 - (offset / rect.height.max(1.0)).clamp(0.0, 1.0);
                                    color.a = (f32::from(color.a) * left).round() as u8;
                                }
                            }
                            if let Some(snapshot) = builder.popover_snapshots.get_mut(&dismiss_id) {
                                snapshot.sheet_offset = offset;
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
                        let origin = builder.paint_tooltip(
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
                        builder.animate_surface(
                            "__tooltip",
                            start,
                            anchor,
                            0.0,
                            true,
                            SurfaceEntrance {
                                scale: options.open_scale,
                                ms: options.enter_ms,
                                origin,
                                easing: options.enter_easing,
                            },
                        );
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
                builder.enter_layer(self.pointer);
                builder.frame.regions.push(HitRegion {
                    id: "__context_menu:surface".into(),
                    rect: *rect,
                    kind: HitKind::Surface,
                });
                let start = builder.frame.commands.len();
                builder.place(menu, *rect, Some(viewport_rect));
                builder.animate_surface("__context_menu", start, *rect, 1.0, true, SurfaceEntrance::default());
                builder.focused = old_focus;
                builder.pressed = old_pressed;
                builder.hovered = old_hovered;
                builder.keyboard_focus = old_keyboard;
            }
            ghost_range = builder.drag_ghost.take();
            builder.pointer = self.pointer;
            receding = builder.recede.take();
            self.hovered_region = builder.hovered_region(base_regions);
            // A slot measured and never placed keeps what its measure found, as one placed does.
            let unplaced: Vec<String> = builder.flow_naturals.borrow().keys().cloned().collect();
            for id in unplaced {
                builder.keep_flow(&id);
            }
            (builder.frame, overlay_start)
        };
        // A zoom view not laid out is gone: a new one under its ID opens fitted.
        self.zoom_views.retain(|_, state| state.seen);
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
                let opened = self.popover_order.get(&id).copied().unwrap_or(0);
                let on_top = observed_popovers.iter().filter_map(|open| self.popover_order.get(open)).all(|other| *other < opened);
                self.popover_exits
                    .insert(id, PopoverExit { paint, opacity, on_top });
            }
            self.popover_exits
                .retain(|id, exit| !observed_popovers.contains(id) && exit.opacity.is_active(now));
            let mut exit_commands = Vec::new();
            let mut top_exits = Vec::new();
            for exit in self.popover_exits.values() {
                let exit_commands = if exit.on_top { &mut top_exits } else { &mut exit_commands };
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
                // An iOS sheet goes on down past the foot of the screen, unless it melts back
                // into the button it grew out of; what it pushed back comes forward again.
                if let (Some(travel), None) = (exit.paint.sheet_exit, exit.paint.morph) {
                    let gone = 1.0 - (1.0 - (1.0 - opacity).clamp(0.0, 1.0)).powi(3);
                    exit_commands.extend(exit.paint.commands.iter().map(|command| command.moved(0.0, travel * gone, 1.0)));
                    if exit.paint.recede > 0.0 {
                        let left = exit.paint.recede * (1.0 - gone);
                        receding = Some((receding.map_or(0.0, |(deepest, _)| deepest).max(left), receding.map_or(0.0, |(_, corner)| corner)));
                    }
                    continue;
                }
                // Let go down its way, a sheet goes on down from where the finger left it.
                if exit.paint.sheet_offset > 1.0 {
                    let gone = 1.0 - (1.0 - (1.0 - opacity).clamp(0.0, 1.0)).powi(3);
                    let from = exit.paint.sheet_offset;
                    let offset = from + (rect.height + SHEET_EXIT_PAST - from) * gone;
                    exit_commands.extend(exit.paint.commands.iter().map(|command| command.moved(0.0, offset, 1.0)));
                    continue;
                }
                let glass_morph = exit.paint.morph.is_some() && exit.paint.commands.iter().any(|command| matches!(command, DrawCommand::Glass { .. }));
                if let (Some((from, _)), true) = (exit.paint.morph, glass_morph) {
                    let closing = exit.paint.close_animation_ms.unwrap_or(exit.paint.animation_ms.min(crate::tokens().motion.exit_ms));
                    let mut commands = exit.paint.commands.clone();
                    let pop = crate::liquid::pop(from, rect, crate::liquid::seconds(1.0 - opacity, closing), true);
                    crate::liquid::apply(&mut commands, 0, rect, pop, liquid_fade);
                    exit_commands.extend(commands);
                    continue;
                }
                for command in &exit.paint.commands {
                    let mut command = command.clone();
                    if let Some((from, radius)) = exit.paint.morph {
                        morph_picture(&mut command, from, radius, MORPH_CURVE.sample(opacity));
                    }
                    animate_popover_command(&mut command, opacity, slide, scale, origin);
                    exit_commands.push(command);
                }
            }
            // Above the ordinary scene, below every currently open popover; one opened over all
            // of them leaves over them.
            frame
                .commands
                .splice(overlay_start..overlay_start, exit_commands);
            frame.commands.extend(top_exits);
        }
        // What an iOS sheet at its large height was opened over recedes behind it, smaller and
        // rounded as a card.
        if let Some((depth, corner)) = receding.filter(|(depth, _)| *depth > 0.001) {
            recede_scene(&mut frame.commands, overlay_start, depth, corner, viewport);
        }
        for id in current_popover_snapshots.keys() {
            if !self.popover_order.contains_key(id) {
                self.popover_opened += 1;
                self.popover_order.insert(id.clone(), self.popover_opened);
            }
        }
        self.popover_order.retain(|id, _| current_popover_snapshots.contains_key(id) || self.popover_exits.contains_key(id));
        // An iOS sheet closed forgets its height: opened again, it rises to its first.
        self.sheet_frames.retain(|id, _| current_popover_snapshots.contains_key(id));
        self.sheet_detents.retain(|id, _| current_popover_snapshots.contains_key(id));
        if self.sheet_drag.as_ref().is_some_and(|drag| !self.sheet_frames.contains_key(&drag.dismiss)) {
            self.sheet_drag = None;
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
        let expand = |order: Vec<(String, HitKind)>| -> Vec<(String, HitKind)> {
            order
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
                .collect()
        };
        self.focus_order = expand(focus_order);
        // The rows of a virtual list are known only once placed: a control in
        // one takes the focus now, its ring on the next layout. A request this
        // layout did not hold is dropped.
        if let Some((id, keyboard)) = self.pending_focus.take() {
            if takes_focus(&self.focus_order, &id) {
                self.give_focus(&id, keyboard);
            }
        }
        let mut regions = Vec::new();
        collect_focus_regions(&self.root, &mut regions);
        self.focus_regions = regions
            .into_iter()
            .map(|(id, order)| {
                let members = expand(order)
                    .into_iter()
                    .map(|(member, _)| member)
                    .filter(|member| self.focus_order.iter().any(|(id, _)| id == member))
                    .collect();
                (id, members)
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
        self.popover_transitions
            .retain(|id, _| observed_popovers.contains(id));
        // A slot is remembered while it is rendered, empty or not: content
        // that arrives in one seen empty is what `expand` opens.
        let states = self.states.get_mut();
        states.sweep(now);
        for state in states.values_mut::<FlowHeight>() {
            if !state.fraction.is_active(now) {
                // At rest by this frame's clock, whatever clock is read later.
                state.fraction = slot_spring(state.fraction.target(), now);
                if state.fraction.target() <= 0.0 {
                    state.commands.clear();
                }
            }
        }
        if frame.coach.as_ref().is_none_or(|coach| coach.missing) {
            self.coach_motion = None;
        }
        self.popover_view_transitions.retain(|id, state| {
            observed_popovers.contains(id)
                && (state.opacity.is_active(now) || state.height.is_active(now))
        });
        self.settled_spinners = settled_spinners;
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
        frame.duplicate_ids = duplicate_ids;
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
                self.modal_return_keyboard = self.keyboard_focus;
            }
            // A press on blank space inside the open modal blurs; only focus
            // that is outside it (or a modal that just opened) is moved in.
            let blurred_inside = self.focused.is_none()
                && previous_popovers
                    .iter()
                    .any(|previous| previous.modal && previous.dismiss_id == scope.dismiss_id);
            if !blurred_inside && !self.focused.as_deref().is_some_and(modal_contains) {
                let next_focus = scope
                    .initial_focus
                    .as_ref()
                    .filter(|preferred| scope.focusables.iter().any(|(id, _)| id == *preferred))
                    .cloned()
                    .or_else(|| scope.focusables.first().map(|(id, _)| id.clone()));
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                // A modal hands its first control the focus; its tooltip waits for the keyboard
                // to move there, as a pointer that opened it never asked for one.
                self.keyboard_tooltip_focus = false;
            }
        } else if had_modal {
            let next_focus = self.modal_return_focus.take().filter(|id| {
                self.focus_order
                    .iter()
                    .any(|(candidate, _)| candidate == id)
            });
            Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
            self.keyboard_focus = self.modal_return_keyboard;
            self.keyboard_tooltip_focus = self.modal_return_keyboard;
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
            // As a modal's, a surface's opening focus waits for the keyboard to show a tooltip.
            self.keyboard_tooltip_focus = false;
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
            // A surface that came or went under the pointer repaints what it covers.
            if target != self.hovered || self.frame.layer_at(point) != pointer_layer {
                self.set_hovered(target, now);
                if refresh_hover_paint {
                    return self.layout_at_with_hover_refresh(viewport, now, false);
                }
            }
        }
        if self.static_selection.as_ref().is_some_and(|selection| !selection.holds_in(&self.frame.text_blocks)) {
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
        self.keep_recent_state();
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

    /// Whether the springs that carry a coach mark's ring and bubble to a new
    /// step are all at rest at `now`; `None` when no coach mark is kept.
    #[doc(hidden)]
    pub fn coach_motion_at_rest(&self, now: Instant) -> Option<bool> {
        self.coach_motion.as_ref().map(|motion| motion.values.iter().all(|value| !value.is_active(now)))
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

    /// Bound what outlives its controls: the caret of each field, the offset of
    /// each scroll area and list, the rows a measured list measured.
    fn keep_recent_state(&mut self) {
        let seen = &mut self.last_laid_out;
        seen.layout += 1;
        let layout = seen.layout;
        let fields = self
            .focus_order
            .iter()
            .filter(|(_, kind)| matches!(kind, HitKind::TextInput | HitKind::TextArea))
            .map(|(id, _)| id.as_str());
        keep_recent(&mut self.inputs, &mut seen.inputs, fields, layout);
        // Every scroll area and list asks for its offset once per layout.
        let scrolls = || self.frame.scroll_motion.keys().map(String::as_str);
        keep_recent(&mut self.scroll_offsets, &mut seen.scrolls, scrolls(), layout);
        keep_recent(&mut self.measured_lists, &mut seen.measured_lists, scrolls(), layout);
        let strips = self.frame.horizontal_viewports.keys().map(String::as_str);
        keep_recent(&mut self.horizontal_scrolls, &mut seen.horizontal_scrolls, strips, layout);
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
                        if let Some((_, view)) = self
                            .code_diffs
                            .iter_mut()
                            .find(|(id, view)| view.owns_expand_control(id, &hit.id, *position))
                        {
                            view.viewport.scroll(x, y);
                            return Some(Vec::new());
                        }
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
        let mut actions = self.handle_event_at_inner(event, now);
        // A tap on an iOS sheet's grabber steps it through its heights.
        actions.retain(|action| !matches!(action, UiAction::Clicked { id } if self.step_sheet(id)));
        actions
    }

    fn handle_event_at_inner(&mut self, event: UiEvent, now: Instant) -> Vec<UiAction> {
        self.observe_keys(&event);
        let event = match event {
            // Only kept, for `key_held` and `modifiers`.
            UiEvent::KeyReleased { .. } | UiEvent::ModifiersChanged(_) => return Vec::new(),
            // A direct pan over an iOS sheet's content may take the sheet instead.
            UiEvent::Scroll2D { position, delta_x, delta_y, direct: true, .. }
                if delta_x.abs() <= delta_y.abs() && self.pan_sheet(position, delta_y, now) => return Vec::new(),
            UiEvent::ScrollEnded => {
                let by_content = self.sheet_drag.as_ref().is_some_and(|drag| drag.grabber.is_none());
                return if by_content { self.let_go_sheet(now).into_iter().collect() } else { Vec::new() };
            }
            UiEvent::PasteCaptured { target, text } => {
                return self.paste_captured(target, text, now)
            }
            // The host's history buttons: nothing in the tree reads them.
            UiEvent::PointerBack(_) | UiEvent::PointerForward(_) => return Vec::new(),
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
            UiEvent::OtherButton { button, pressed, position } => {
                return vec![UiAction::OtherButton { button, pressed, position }]
            }
            UiEvent::PasteImage { width, height, rgba } => {
                return vec![UiAction::PasteImage {
                    target: self.focused.clone(),
                    width,
                    height,
                    rgba,
                }]
            }
            other => other,
        };
        if let Some(actions) = self.handle_drag_event(&event, now) {
            return actions;
        }
        if let (Some(drag), UiEvent::PointerMove(point) | UiEvent::PointerUp(point)) = (&mut self.glass_switch_drag, &event) {
            drag.moved |= (point.x - drag.origin.x).abs() > 3.0;
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
        self.apply_zoom_controls(&mut actions, now);
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
                    last: None,
                    speed: 0.0,
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
    fn handle_drag_event(&mut self, event: &UiEvent, now: Instant) -> Option<Vec<UiAction>> {
        let drag = self.pointer_drag.as_mut()?;
        match *event {
            UiEvent::PointerMove(position) => {
                // The speed of the last stretch, smoothed: what a flick is told by.
                if let Some((y, at)) = drag.last {
                    let seconds = now.saturating_duration_since(at).as_secs_f32();
                    if seconds > 0.004 {
                        drag.speed = drag.speed * 0.3 + (position.y - y) / seconds * 0.7;
                    }
                }
                drag.last = Some((position.y, now));
                let mut actions = Vec::new();
                let origin = drag.origin.y;
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
                // An iOS sheet's grabber: the sheet follows the finger.
                if let Some(dismiss) = id.strip_suffix(":drag").filter(|dismiss| self.sheet_frames.contains_key(*dismiss)) {
                    let from = match &self.sheet_drag {
                        Some(SheetDrag { grabber: Some((_, _, from)), .. }) => *from,
                        _ => self.sheet_frames[dismiss].height,
                    };
                    let dismiss = dismiss.to_owned();
                    self.move_sheet(&dismiss, Some((id.clone(), origin, from)), from - (position.y - origin), now);
                }
                actions.push(UiAction::DragMoved {
                    over: self.drop_target_at(position, &id),
                    id,
                    position,
                });
                Some(actions)
            }
            UiEvent::PointerUp(position) => {
                let drag = self.pointer_drag.take()?;
                if !drag.active {
                    return None;
                }
                // An iOS sheet's grabber let go: the sheet rests at the height its speed carries it
                // nearest to, or leaves, which reports its dismissal.
                if self.sheet_drag.as_ref().is_some_and(|sheet| sheet.grabber.as_ref().is_some_and(|(id, _, _)| *id == drag.id)) {
                    return Some(match self.let_go_sheet(now) {
                        Some(dismissal) => vec![dismissal],
                        None => vec![UiAction::DragCancelled { id: drag.id }],
                    });
                }
                // A sheet's handle let go: far enough down, or flung down, the sheet leaves,
                // its dismissal named as where it was dropped; short of that, it springs back.
                if let Some((dismiss, sheet)) = self.sheet_of(&drag.id) {
                    let down = position.y - drag.origin.y;
                    // A finger that stopped before letting go flicks nothing.
                    let still = drag.last.is_none_or(|(_, at)| now.saturating_duration_since(at) > Duration::from_millis(80));
                    let speed = if still { 0.0 } else { drag.speed };
                    let leaves = down >= (sheet.height * SHEET_AWAY).clamp(SHEET_AWAY_LEAST, SHEET_AWAY_MOST)
                        || (speed >= SHEET_FLING && down > DRAG_THRESHOLD);
                    return Some(vec![if leaves {
                        UiAction::Dropped { id: drag.id, position, target: Some(dismiss) }
                    } else {
                        UiAction::DragCancelled { id: drag.id }
                    }]);
                }
                Some(vec![UiAction::Dropped {
                    target: self.drop_target_at(position, &drag.id),
                    id: drag.id,
                    position,
                }])
            }
            UiEvent::Key {
                key: Key::Escape, ..
            } if drag.active => {
                let drag = self.pointer_drag.take()?;
                // A sheet held comes back to where it rested.
                if let Some(sheet) = self.sheet_drag.take() {
                    self.set_sheet_spring(&sheet.dismiss, ScalarSpring::new(sheet.height, now), now);
                }
                Some(vec![UiAction::DragCancelled { id: drag.id }])
            }
            _ => None,
        }
    }

    /// Takes the iOS sheet `dismiss` where a finger has, `raw` points tall before its rubber
    /// band past its largest height; `grabber`: held by it, with the finger's first `y` and
    /// the sheet's height then.
    fn move_sheet(&mut self, dismiss: &str, grabber: Option<(String, f32, f32)>, raw: f32, now: Instant) {
        let Some(frame) = self.sheet_frames.get(dismiss) else { return };
        let largest = frame.heights.last().copied().unwrap_or(frame.height);
        let height = crate::sheet_geometry::followed(raw, largest, frame.viewport);
        match self.sheet_drag.as_mut().filter(|drag| drag.dismiss == dismiss) {
            Some(drag) => {
                let seconds = now.saturating_duration_since(drag.last).as_secs_f32();
                if seconds > 0.004 {
                    drag.speed = drag.speed * 0.3 + (height - drag.height) / seconds * 0.7;
                }
                drag.raw = raw;
                drag.height = height;
                drag.last = now;
            }
            None => {
                // As UIKit's: taking the sheet first puts the keyboard away.
                let typing = self.focused.as_deref().is_some_and(|id| self.frame.regions.iter()
                    .any(|region| region.id == id && matches!(region.kind, HitKind::TextInput | HitKind::TextArea)));
                if typing {
                    Self::replace_focused(&mut self.focused, &mut self.paste_epoch, None);
                }
                self.sheet_drag = Some(SheetDrag { dismiss: dismiss.to_owned(), grabber, raw, height, speed: 0.0, last: now });
            }
        }
    }

    /// Puts an iOS sheet's spring where a release or a cancel leaves it. The layout that
    /// holds the sheet keeps it from there.
    fn set_sheet_spring(&mut self, dismiss: &str, value: ScalarSpring, now: Instant) {
        self.states.get_mut().touch(&format!("__sheet:{dismiss}"), now, || AnimatedSpring { value }).value = value;
    }

    /// Lets go of the iOS sheet held: its spring sets off from where the finger left it, at its
    /// speed, to the height that speed carries it nearest to; or it leaves, sliding on down
    /// rather than into the button it grew out of, which reports its dismissal.
    fn let_go_sheet(&mut self, now: Instant) -> Option<UiAction> {
        let drag = self.sheet_drag.take()?;
        let frame = self.sheet_frames.get(&drag.dismiss)?;
        // A finger that stopped before letting go flicks nothing.
        let speed = if now.saturating_duration_since(drag.last) > Duration::from_millis(80) { 0.0 } else { drag.speed };
        let rest = crate::sheet_geometry::release(&frame.heights, drag.height, speed, frame.dismissible);
        self.set_sheet_spring(&drag.dismiss, ScalarSpring::moving(drag.height, speed, now), now);
        match rest {
            Some(index) => {
                self.sheet_detents.insert(drag.dismiss, index);
                None
            }
            None => {
                if let Some(snapshot) = self.popover_snapshots.get_mut(&drag.dismiss) {
                    snapshot.morph = None;
                }
                Some(UiAction::Clicked { id: drag.dismiss })
            }
        }
    }

    /// A direct pan over an iOS sheet takes the sheet where its content cannot go that way:
    /// down while the content is at its top, up while the sheet is short of its largest
    /// height. Back at its largest going up, the content scrolls again. Whether it took it.
    fn pan_sheet(&mut self, position: Point, delta: f32, now: Instant) -> bool {
        if let Some(drag) = self.sheet_drag.as_ref().filter(|drag| drag.grabber.is_none()) {
            let dismiss = drag.dismiss.clone();
            let raw = drag.raw - delta;
            let Some(frame) = self.sheet_frames.get(&dismiss) else { return false };
            let largest = frame.heights.last().copied().unwrap_or(frame.height);
            if raw >= largest && delta < 0.0 {
                let last = frame.heights.len().saturating_sub(1);
                self.set_sheet_spring(&dismiss, ScalarSpring::new(largest, now), now);
                self.sheet_detents.insert(dismiss, last);
                self.sheet_drag = None;
                return false;
            }
            self.move_sheet(&dismiss, None, raw, now);
            return true;
        }
        if delta == 0.0 {
            return false;
        }
        // The surface under the finger, topmost first: only an iOS sheet's is taken.
        let Some((index, dismiss)) = self.frame.regions.iter().enumerate().rev()
            .filter(|(_, region)| region.rect.contains(position))
            .find_map(|(index, region)| region.id.strip_prefix("popover_surface:").map(|id| (index, id.to_owned())))
        else { return false };
        let Some(frame) = self.sheet_frames.get(&dismiss) else { return false };
        let largest = frame.heights.last().copied().unwrap_or(frame.height);
        // What scrolls under the finger, inside the sheet, and whether it is at its top.
        let at_top = self.frame.regions[index..].iter().rev()
            .find(|region| region.kind == HitKind::Scroll && region.rect.contains(position))
            .is_none_or(|region| self.scroll_offset(&region.id) <= 0.5);
        let takes = (delta > 0.0 && at_top) || (delta < 0.0 && frame.height < largest - 0.5);
        if takes {
            let raw = frame.height - delta;
            self.move_sheet(&dismiss, None, raw, now);
        }
        takes
    }

    /// A tap on an iOS sheet's grabber, `id`: the sheet steps to its next height, after its
    /// largest back to its smallest. Whether `id` was one.
    fn step_sheet(&mut self, id: &str) -> bool {
        let Some((dismiss, frame)) = id.strip_suffix(":drag").and_then(|dismiss| self.sheet_frames.get_key_value(dismiss)) else { return false };
        if frame.heights.len() > 1 {
            let next = (frame.resting + 1) % frame.heights.len();
            self.sheet_detents.insert(dismiss.clone(), next);
        }
        true
    }

    /// The bottom sheet whose handle `id` is, by its dismissal, and where it stands.
    fn sheet_of(&self, id: &str) -> Option<(String, Rect)> {
        let dismiss = id.strip_suffix(":drag")?;
        let surface = format!("popover_surface:{dismiss}");
        let rect = self.frame.regions.iter().find(|region| region.id == surface)?.rect;
        Some((dismiss.to_owned(), rect))
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
        if let Some(actions) = self.handle_zoom_event(&event, now) {
            return actions;
        }
        if self.handle_horizontal_scroll(&event) {
            return Vec::new();
        }
        // A field reads Shift and the click count of a press that ordinary
        // controls take as a plain click.
        let (extend, clicks) = match &event {
            UiEvent::PointerDownModified { shift, .. } => (*shift, 1),
            UiEvent::PointerDownEditor { shift, click_count, .. } => (*shift, *click_count),
            _ => (false, 1),
        };
        let event = match event {
            UiEvent::ImeCommit(text) => {
                self.compose_in_field(None);
                UiEvent::Text(text)
            }
            UiEvent::ImePreedit(text) => {
                self.compose_in_field(Some(text));
                return Vec::new();
            }
            UiEvent::ImeCancelled => {
                self.compose_in_field(None);
                return Vec::new();
            }
            UiEvent::ImeDeleteSurrounding { before_chars, after_chars } => {
                return self.delete_around_caret(before_chars, after_chars);
            }
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
            | UiEvent::ScrollEnded
            | UiEvent::Paste(_) => {
                unreachable!("additive editor events normalized above")
            }
            UiEvent::FilesHovered { .. }
            | UiEvent::FilesDropped { .. }
            | UiEvent::PasteImage { .. }
            | UiEvent::OtherButton { .. }
            | UiEvent::PointerBack(_)
            | UiEvent::PointerForward(_)
            | UiEvent::KeyReleased { .. }
            | UiEvent::ModifiersChanged(_) => {
                unreachable!("external file, history and held-key events resolved before ordinary controls")
            }
            // A pinch over no zoom view.
            UiEvent::Zoom { .. } => {}
            UiEvent::PointerMove(point) => {
                if let Some(draft) = self.sketch_draft.as_mut() {
                    draft.extend(point);
                }
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
                self.drag_selection(point, true);
                if let Some((id, multiline)) = &self.field_drag {
                    if let Some(input) = self.inputs.get_mut(id) {
                        input.place_cursor_at(point, *multiline, true);
                    }
                }
            }
            UiEvent::PointerLeave => {
                self.glass_switch_drag = None;
                if let Some(draft) = self.sketch_draft.take() {
                    actions.push(UiAction::Sketched { id: draft.id.clone(), mark: draft.mark() });
                }
                self.field_drag = None;
                for editor in self.code_editors.values_mut() {
                    editor.viewport.cancel_pointer();
                }
                for diff in self.code_diffs.values_mut() {
                    diff.viewport.cancel_pointer();
                }
                if let Some(drag) = self.slider_drag.take() {
                    let snapped = drag.geometry.snap(drag.fraction);
                    if !self.reduced_motion && drag.geometry.animated {
                        let states = self.states.get_mut();
                            let motion = states.touch(&drag.id, now, || SliderMotion::new(now));
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
                let focused_before = self.focused.clone();
                self.field_drag = None;
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
                let extended = self.static_selection.take().filter(|_| extend);
                if hit
                    .as_ref()
                    .is_some_and(|region| region.kind == HitKind::Surface)
                    && dismiss_scope.is_some_and(|scope| scope.modal)
                {
                    self.keyboard_focus = true;
                    self.pressed = None;
                    return actions;
                }
                if let Some((id, (rect, style))) = hit.as_ref().and_then(|region| {
                    self.frame.sketches.get(&region.id).map(|sketch| (region.id.clone(), *sketch))
                }) {
                    self.sketch_draft = Some(crate::sketch::SketchDraft::new(id, rect, style, point));
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
                        let states = self.states.get_mut();
                            let motion = states.touch(&id, now, || SliderMotion::new(now));
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
                        // Shift and a press in the same block, or scope, move
                        // the selection's end there and drag it on.
                        let mut selection = extended
                            .filter(|selection| match selection.scope {
                                Some(scope) => block.scope == Some(scope),
                                None => selection.anchor.block == index && selection.anchor.source == block.source,
                            })
                            .unwrap_or_else(|| StaticSelection::at(&self.frame.text_blocks, index, offset, true));
                        selection.cursor = TextPoint::at(&self.frame.text_blocks, index, offset);
                        selection.dragging = true;
                        self.static_selection = Some(selection);
                    }
                }
                self.pressed = hit
                    .as_ref()
                    .filter(|region| region.kind == HitKind::Button)
                    .map(|region| region.id.clone());
                self.pressed_dispatch = self.pressed.as_deref().map(|id| self.dispatch_id(id));
                self.glass_switch_drag = self.pressed.as_ref().and_then(|id| self.frame.glass_switches.get(id))
                    .map(|geometry| GlassSwitchDrag { origin: point, geometry: *geometry, moved: false });
                let next_focus =
                    hit.as_ref()
                        .filter(|region| {
                            (matches!(
                                region.kind,
                                HitKind::Button | HitKind::Focus | HitKind::TextInput | HitKind::TextArea
                            ) || (region.kind == HitKind::CodeDiff
                                && self.focus_order.iter().any(|(id, kind)| {
                                    id == &region.id && *kind == HitKind::CodeDiff
                                })))
                                && dismiss_scope.is_none()
                                && !self.frame.pointer_only.contains(&region.id)
                        })
                        .map(|region| region.id.clone())
                        .or(dismiss_trigger);
                Self::replace_focused(&mut self.focused, &mut self.paste_epoch, next_focus);
                if let Some(region) = hit
                    .filter(|region| matches!(region.kind, HitKind::TextInput | HitKind::TextArea))
                {
                    let multiline = region.kind == HitKind::TextArea;
                    if let Some(input) = self.inputs.get_mut(&region.id) {
                        // Shift extends the selection of a field that already had the focus.
                        let extend = extend && focused_before.as_deref() == Some(region.id.as_str());
                        input.place_cursor_at(point, multiline, extend);
                        match clicks {
                            2 => {
                                input.select_word();
                                if self.touch_text_menu.is_some() { self.edit_menu_on_release = Some(point); }
                            }
                            3.. => input.select_line(multiline),
                            _ => self.field_drag = Some((region.id.clone(), multiline)),
                        }
                    }
                }
            }
            UiEvent::PointerUp(point) => {
                if let Some(at) = self.edit_menu_on_release.take() {
                    self.pressed = None;
                    self.field_drag = None;
                    actions.extend(self.open_text_context_menu(at, false, now));
                    return actions;
                }
                if let Some(mut draft) = self.sketch_draft.take() {
                    draft.extend(point);
                    actions.push(UiAction::Sketched { id: draft.id.clone(), mark: draft.mark() });
                }
                self.field_drag = None;
                self.pointer = Some(point);
                self.set_hovered(self.frame.hit_test(point).map(str::to_owned), now);
                self.tooltip_hover_since = self.hovered.as_ref().map(|id| (id.clone(), now));
                if let Some(drag) = self.slider_drag.take() {
                    let released_fraction = drag.geometry.fraction_at(point);
                    let snapped = drag.geometry.snap(released_fraction);
                    if !self.reduced_motion && drag.geometry.animated {
                        let states = self.states.get_mut();
                            let motion = states.touch(&drag.id, now, || SliderMotion::new(now));
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
                self.drag_selection(point, false);
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
                let switch_drag = self.glass_switch_drag.take();
                let pressed_dispatch = self.pressed_dispatch.take();
                if let Some(pressed) = self.pressed.take() {
                    let unchanged_drag = switch_drag.is_some_and(|drag| {
                        let geometry = self.frame.glass_switches.get(&pressed).copied().unwrap_or(drag.geometry);
                        drag.moved && geometry.checked == (point.x >= geometry.bounds.x + geometry.bounds.width / 2.0)
                    });
                    if unchanged_drag { return actions; }
                    // A finger slid from one touch menu row onto another chooses that one.
                    let slid = self.frame.press_region(point).map(|region| region.id.clone())
                        .filter(|id| *id != pressed && self.frame.press_highlight_rows.contains(id)
                            && self.frame.press_highlight_rows.contains(&pressed));
                    // A tab bar's lens dragged along it lands on the tab nearest the finger.
                    let landed = self.frame.glass_tabs.iter().find(|tabs| tabs.iter().any(|(id, _)| *id == pressed))
                        .filter(|_| self.frame.press_region(point).map(|region| region.id.as_str()) != Some(pressed.as_str()))
                        .and_then(|tabs| tabs.iter().min_by(|a, b| {
                            let off = |rect: &Rect| (rect.x + rect.width / 2.0 - point.x).abs();
                            off(&a.1).total_cmp(&off(&b.1))
                        }))
                        .map(|(id, _)| id.clone());
                    if let Some(id) = slid.or(landed) {
                        actions.push(UiAction::Clicked { id: self.dispatch_id(&id) });
                    } else if self.frame.press_region(point).map(|region| region.id.as_str())
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
                        // A soft keyboard's Return can come as a line break alone, with no
                        // Enter key, as iOS's does: a single-line field submits on it.
                        if !multiline && matches!(text.as_str(), "\n" | "\r" | "\r\n") {
                            actions.push(UiAction::Submitted {
                                id: id.clone(),
                                value: input.value.clone(),
                            });
                        } else if input.insert(&text, multiline) {
                            actions.push(UiAction::InputChanged {
                                id: id.clone(),
                                value: input.value.clone(),
                            });
                        }
                    }
                }
            }
            UiEvent::Key { key, shift, ctrl, alt } => {
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
                    if let Some(text) = input_text.or_else(|| self.selected_text()) {
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
                } else if key == Key::F6 && !ctrl && !alt {
                    self.focus_next_region(shift);
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
                } else if (!shift
                    && !ctrl
                    && matches!(key, Key::ArrowDown | Key::ArrowUp | Key::Home | Key::End)
                    && self.step_inline_menu_option(key))
                    // Past what the focused control takes, a remote's arrows move the focus.
                    || (self.spatial_focus
                        && !shift
                        && !ctrl
                        && !alt
                        && matches!(key, Key::ArrowUp | Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight)
                        && !self.arrow_is_the_focused_controls(key)
                        && self.step_spatial(key))
                {
                    self.keyboard_focus = true;
                    self.keyboard_tooltip_focus = true;
                } else if key == Key::A
                    && ctrl
                    && !shift
                    && !alt
                    && !self.focused.as_ref().is_some_and(|id| self.inputs.contains_key(id))
                    && self.select_all_text()
                {
                    // The texts of the scope a press landed in, as a page's Ctrl+A.
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
                                input.step(false, ctrl, shift);
                                false
                            }
                            Key::ArrowRight => {
                                input.step(true, ctrl, shift);
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
                            Key::Backspace => input.delete_step(false, ctrl),
                            Key::Delete => input.delete_step(true, ctrl),
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
        // Under a remote, an arrow that would not move a slider at its end moves the focus on
        // instead: otherwise the focus could never leave it that way.
        let stays = |changed: bool| self.spatial_focus && !changed && matches!(key, Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown);
        // An area slider: Left and Right move across, Up and Down up and down.
        if let Some(y) = geometry.y {
            let (from_x, from_y) = (geometry.fraction, y);
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
            if stays(x != from_x || y != from_y) { return false; }
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
            if stays(fraction != current) { return false; }
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
        if stays(next != current) { return false; }
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

    /// Whether the focused control moves its own caret with `key`: Left and
    /// Right in a text field, and every arrow in a text area or code editor.
    fn arrow_is_the_focused_controls(&self, key: Key) -> bool {
        let Some(id) = self.focused.as_ref() else { return false };
        let kind = self.focus_order.iter().find(|(candidate, _)| candidate == id).map(|(_, kind)| *kind);
        match kind {
            Some(HitKind::TextInput) => matches!(key, Key::ArrowLeft | Key::ArrowRight),
            Some(HitKind::TextArea | HitKind::CodeEditor | HitKind::CodeDiff) => true,
            _ => false,
        }
    }

    /// Focus the control nearest the focused one in `key`'s direction and
    /// bring it into view, within the open popover when one holds the focus.
    /// With nothing focused, the first control takes it. Says whether the
    /// focus moved.
    fn step_spatial(&mut self, key: Key) -> bool {
        let candidates: Vec<String> = match self.active_popover_scope().cloned() {
            Some(scope) => self.reachable_popover_focusables(&scope).into_iter().map(|(id, _)| id).collect(),
            None => self
                .focus_order
                .iter()
                .filter(|(_, kind)| matches!(kind, HitKind::Button | HitKind::Focus | HitKind::TextInput | HitKind::TextArea))
                .map(|(id, _)| id.clone())
                .collect(),
        };
        // Where each is laid out, scrolled out of sight or not.
        let rect = |id: &str| {
            self.frame
                .scroll_focus_targets
                .get(id)
                .map(|target| target.rect)
                .or_else(|| self.frame.regions.iter().find(|region| region.id == id).map(|region| region.rect))
                .filter(|rect| rect.width > 0.0 && rect.height > 0.0)
        };
        let next = match self.focused.as_deref().and_then(rect) {
            Some(from) => {
                let others = candidates
                    .iter()
                    .filter(|id| Some(id.as_str()) != self.focused.as_deref())
                    .filter_map(|id| rect(id).map(|to| (id.as_str(), to)));
                nearest_in_direction(from, key, others).map(str::to_owned)
            }
            None => candidates.iter().find(|id| rect(id).is_some()).cloned(),
        };
        let Some(id) = next else { return false };
        self.request_scroll_focus(&id);
        Self::replace_focused(&mut self.focused, &mut self.paste_epoch, Some(id));
        true
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
            let mut seen = crate::FastSet::default();
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

/// Each landmark region in `element`, outermost first, with what it holds in
/// focus order (virtual list markers included, for layout to expand).
fn collect_focus_regions(element: &Element, out: &mut Vec<(String, Vec<(String, HitKind)>)>) {
    let region = element
        .access
        .as_deref()
        .filter(|said| said.role == Some(crate::Role::Region))
        .and_then(|said| said.id.clone());
    if let Some(id) = region {
        let mut order = Vec::new();
        collect_focus_order(element, &mut order, &mut crate::FastSet::default());
        out.push((id, order));
    }
    match &element.kind {
        ElementKind::Column(children)
        | ElementKind::Row(children)
        | ElementKind::TabStrip { children, .. }
        | ElementKind::Stack(children) => {
            for child in children {
                collect_focus_regions(child, out);
            }
        }
        ElementKind::Panel(child)
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. }
        | ElementKind::Clickable { child, .. }
        | ElementKind::Tooltip { anchor: child, .. } => collect_focus_regions(child, out),
        ElementKind::Overlay { base, floating } => {
            collect_focus_regions(base, out);
            collect_focus_regions(floating, out);
        }
        ElementKind::Popover { anchor, floating, hover_open, .. } => {
            collect_focus_regions(anchor, out);
            if !*hover_open {
                collect_focus_regions(floating, out);
            }
        }
        _ => {}
    }
}

fn collect_focus_order(
    element: &Element,
    order: &mut Vec<(String, HitKind)>,
    seen: &mut crate::FastSet<String>,
) {
    collect_focus_claims(element, order, seen, &mut Vec::new());
}

/// Gives `id` its place in the focus order, or notes it in `duplicates` when
/// an earlier element already claimed it.
fn claim_focus(
    id: &str,
    kind: HitKind,
    order: &mut Vec<(String, HitKind)>,
    seen: &mut crate::FastSet<String>,
    duplicates: &mut Vec<String>,
) {
    if seen.insert(id.to_owned()) {
        order.push((id.to_owned(), kind));
    } else if !duplicates.iter().any(|duplicate| duplicate == id) {
        duplicates.push(id.to_owned());
    }
}

/// As [`collect_focus_order`], also noting each ID that two elements claim.
fn collect_focus_claims(
    element: &Element,
    order: &mut Vec<(String, HitKind)>,
    seen: &mut crate::FastSet<String>,
    duplicates: &mut Vec<String>,
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
                collect_focus_claims(child, order, seen, duplicates);
            }
        }
        ElementKind::Panel(child)
        | ElementKind::Scroll { child, .. }
        | ElementKind::HorizontalScroll { child, .. } => {
            collect_focus_claims(child, order, seen, duplicates);
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
            if !is_pointer_only(element) {
                claim_focus(id, clickable_kind(element), order, seen, duplicates);
            }
            collect_focus_claims(child, order, seen, duplicates);
        }
        ElementKind::Overlay { base, floating } => {
            collect_focus_claims(base, order, seen, duplicates);
            collect_focus_claims(floating, order, seen, duplicates);
        }
        ElementKind::Popover {
            anchor,
            floating,
            hover_open,
            ..
        } => {
            collect_focus_claims(anchor, order, seen, duplicates);
            if !*hover_open {
                collect_focus_claims(floating, order, seen, duplicates);
            }
        }
        ElementKind::Tooltip { anchor, .. } => collect_focus_claims(anchor, order, seen, duplicates),
        ElementKind::MarkRail(rail) => {
            for mark in &rail.marks {
                claim_focus(&mark.id, HitKind::Button, order, seen, duplicates);
            }
        }
        ElementKind::Button { id, .. }
        | ElementKind::IconButton { id, .. }
        | ElementKind::EffortSlider { id, .. } => claim_focus(id, HitKind::Button, order, seen, duplicates),
        ElementKind::TextInput { id, .. } => claim_focus(id, HitKind::TextInput, order, seen, duplicates),
        ElementKind::TextArea { id, .. } => claim_focus(id, HitKind::TextArea, order, seen, duplicates),
        ElementKind::CodeEditor(editor) => claim_focus(&editor.id, HitKind::CodeEditor, order, seen, duplicates),
        ElementKind::CodeDiff(diff) if diff.focusable => {
            claim_focus(&diff.id, HitKind::CodeDiff, order, seen, duplicates);
        }
        ElementKind::FormattedText(spans) => {
            // A link's action is the same wherever its target is linked
            // again, and a span has no alias to tell them apart: not a duplicate.
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
        ElementKind::CodeDiff(_)
        | ElementKind::Flowchart(_)
        | ElementKind::SequenceDiagram(_)
        | ElementKind::Skeleton { .. }
        | ElementKind::Spinner { .. }
        | ElementKind::Scribble { .. }
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
        | ElementKind::ZoomView(_)
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
        | ElementKind::ZoomView(_)
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
        | ElementKind::Scribble { .. }
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

/// A clickable's kind of region: a button, or a region that only takes the
/// focus.
/// Whether `order` holds `id` as a control that takes the focus.
fn takes_focus(order: &[(String, HitKind)], id: &str) -> bool {
    order.iter().any(|(candidate, kind)| {
        candidate == id
            && matches!(
                kind,
                HitKind::Button | HitKind::Focus | HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor | HitKind::CodeDiff
            )
    })
}

fn is_pointer_only(element: &Element) -> bool {
    element.kept.as_ref().is_some_and(|kept| kept.pointer_only)
}

fn clickable_kind(element: &Element) -> HitKind {
    if element.kept.as_ref().is_some_and(|kept| kept.focus_area) {
        HitKind::Focus
    } else {
        HitKind::Button
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
        ElementKind::Clickable { .. } if element.kept.as_ref().is_some_and(|kept| kept.focus_area) => Access::new(Role::Group),
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
    let found = descendants || element.kept.as_ref().is_some_and(|kept| kept.anchor.is_some())
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
    translated_overflow: crate::FastMap<(u64, usize), bool>,
    focus_path: HashSet<usize>,
    focus_probe: Option<&'a str>,
    /// Whether the arrows move the focus, which needs the bounds of the
    /// controls held out of sight.
    spatial_focus: bool,
    /// Source of the active pointer drag and the commands it painted.
    drag_source: Option<&'a str>,
    /// A sheet held by its handle: its dismissal, how far the finger has taken it down
    /// (less than that up, as a rubber band), and how fast.
    sheet_held: Option<(&'a str, f32, f32)>,
    /// The iOS sheet a finger holds, by its dismissal, at the height it has taken it to.
    sheet_hold: Option<(&'a str, f32)>,
    sheet_detents: &'a HashMap<String, usize>,
    sheet_frames: &'a mut HashMap<String, SheetFrame>,
    /// The deepest an iOS sheet at its large height has pushed back what it was opened over,
    /// 0 to 1, and the screen's corner radius it rounds that with.
    recede: Option<(f32, f32)>,
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
    states: &'a RefCell<StateStore>,
    /// How deep the measures under way are, in debug builds, which check
    /// that a measure changes no state.
    measuring: std::cell::Cell<u32>,
    /// The content height and `expand` each `animate_height` slot was last
    /// measured with in this layout, for placing it to keep.
    flow_naturals: RefCell<crate::FastMap<String, (f32, bool)>>,
    /// Where the column or row being placed starts its children, which an
    /// `animate_move` slot is measured from.
    flow_origin: Point,
    pushes_moved: &'a std::cell::Cell<bool>,
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
    /// The pointer as the layer being placed sees it: `None` while a layer
    /// above it holds the pointer.
    pointer: Option<Point>,
    /// The layer that held the pointer in the last frame.
    pointer_layer: usize,
    /// The pointer is over the `hover_region` being placed.
    region_hovered: bool,
    /// The named hover regions the pointer is over, outermost and earliest
    /// first, each with the count of hit regions once its own were laid out.
    hovered_regions: Vec<(String, usize)>,
    /// What a wrapper said of the control inside it, until that control takes it.
    access_above: Option<crate::Access>,
    slider_drag: Option<&'a SliderDrag>,
    sketch_draft: Option<&'a crate::sketch::SketchDraft>,
    focused: Option<&'a str>,
    caret_blink: &'a mut crate::caret_blink::CaretBlink,
    hovered: Option<&'a str>,
    tooltip_hover_since: Option<&'a (String, Instant)>,
    keyboard_focus: bool,
    keyboard_tooltip_focus: bool,
    pressed: Option<&'a str>,
    closed_submenu_triggers: &'a HashSet<String>,
    selection: Option<&'a StaticSelection>,
    /// The innermost selection scope being laid out, and the next one's number.
    selection_scope: Option<usize>,
    next_selection_scope: usize,
    popover_transitions: &'a mut HashMap<String, ScalarTransition>,
    popover_exits: &'a HashMap<String, PopoverExit>,
    /// Entrances first seen in this layout start where they end.
    settle_entrances: bool,
    previous_popover_snapshots: &'a HashMap<String, PopoverSnapshot>,
    popover_view_transitions: &'a mut HashMap<String, PopoverViewTransition>,
    coach_motion: &'a mut Option<CoachMotion>,
    /// The fill of the shared hover surface the pointer is over, if any.
    hover_surface_fill: Option<Color>,
    /// The pointer is over no control: a shared surface under it stays lit,
    /// so crossing the gap between two of its actions does not blink.
    surface_gaps_lit: bool,
    observed_popovers: &'a mut HashSet<String>,
    popover_snapshots: &'a mut HashMap<String, PopoverSnapshot>,
    observed_tooltips: &'a mut HashMap<String, Duration>,
    settled_spinners: &'a mut HashSet<String>,
    zoom_views: &'a mut HashMap<String, crate::zoom_view::ZoomState>,
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
        morph: Option<(Rect, f32)>,
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
        anchor_outset: [f32; 4],
        clear_of: Option<String>,
        sheet: Option<Box<crate::SheetSpec>>,
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

/// Whether the pointer lies in the space a popover's trigger leaves beside its rows: the
/// outset and the gap, or, when the popover stands beside its parent's `surface`, the gap
/// on either side of that surface. A hover popover placed clear of its row holds while the
/// pointer crosses that space to reach it.
fn beside_trigger(
    trigger: Rect,
    outset: [f32; 4],
    gap: f32,
    surface: Option<Rect>,
    pointer: Option<Point>,
) -> bool {
    let [left, _, right, _] = outset;
    let (x, width) = match surface {
        Some(surface) => (surface.x, surface.width),
        None => (trigger.x - left, trigger.width + left + right),
    };
    let band = Rect::new(x - gap, trigger.y, width + gap * 2.0, trigger.height);
    pointer.is_some_and(|point| band.contains(point) && !trigger.contains(point))
}

/// The rectangle of the surface of the popover `dismiss_id`, when it was placed in this frame.
fn surface_rect(frame: &Frame, dismiss_id: &str) -> Option<Rect> {
    let id = format!("popover_surface:{dismiss_id}");
    frame
        .regions
        .iter()
        .rev()
        .find(|region| region.id == id)
        .map(|region| region.rect)
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
    let right_has_room = anchor.x + anchor.width + gap + width <= viewport.x + viewport.width - inset;
    let x = match placement {
        PopoverPlacement::Center => viewport.x + (viewport.width - width) / 2.0,
        PopoverPlacement::RightStart => {
            if right_has_room {
                anchor.x + anchor.width + gap
            } else {
                anchor.x - gap - width
            }
        }
        PopoverPlacement::RightCorner => {
            if right_has_room {
                anchor.x + anchor.width + gap
            } else {
                anchor.x + anchor.width - gap - width
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
    } else if placement == PopoverPlacement::RightCorner {
        if right_has_room { anchor.y } else { anchor.y + gap }
    } else if matches!(
        placement,
        PopoverPlacement::RightStart | PopoverPlacement::LeftStart
    ) {
        // With no room below it ends level with its anchor, as a submenu near
        // the foot of the window does, instead of being pushed up by the edge.
        if anchor.y + height > viewport.y + viewport.height - inset {
            anchor.y + anchor.height - height
        } else {
            anchor.y
        }
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
#[doc(hidden)]
pub const COACH_RING_GAP: f32 = 3.0;
/// The width of the stroke of a coach mark's ring.
#[doc(hidden)]
pub const COACH_RING_WIDTH: f32 = 2.0;
/// How far the caret reaches from the bubble towards the ring.
#[doc(hidden)]
pub const COACH_CARET: f32 = 7.0;

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

/// A tooltip's pointer `size` wide along the body and deep away from it, from the body's
/// `edge` toward `outward` (a unit vector), its tip rounded: two triangles up to where the
/// rounding starts and a circle tangent to both sides. Its sides meet 1 px inside the body
/// and go on 1 px further, so its fill covers the body's outline where they meet wherever
/// the edge falls on the pixels. A visible `ring`, the body's 1 px
/// inset outline, goes on around its sides and tip as one stroke joined to the body's on
/// both sides of the base, and not across it.
fn rounded_caret(edge: Point, outward: Point, size: Size, color: Color, ring: Color) -> Vec<DrawCommand> {
    let half = size.width / 2.0;
    let depth = size.height + 1.0;
    if half <= 0.0 || size.height <= 0.0 {
        return Vec::new();
    }
    let along = Point::new(-outward.y, outward.x);
    let at = |base: Point, sideways: f32, out: f32| {
        Point::new(
            base.x + along.x * sideways + outward.x * out,
            base.y + along.y * sideways + outward.y * out,
        )
    };
    let base = at(edge, 0.0, -1.0);
    // The tip's half angle, and a rounding a fifth of the pointer's smaller side, at most 3.
    let slant = (half * half + depth * depth).sqrt();
    let (sin, cos) = (half / slant, depth / slant);
    let round = (size.width.min(size.height) * 0.2).min(3.0);
    // Where the circle meets the sides, measured from the tip along them and outward.
    let along_side = round * cos / sin;
    let tangent_depth = depth - along_side * cos;
    let tangent_half = along_side * sin;
    let center_depth = depth - round / sin;
    let left = at(base, -half * (1.0 + 1.0 / depth), -1.0);
    let right = at(base, half * (1.0 + 1.0 / depth), -1.0);
    let tip_right = at(base, tangent_half, tangent_depth);
    let tip_left = at(base, -tangent_half, tangent_depth);
    let center = at(base, 0.0, center_depth);
    let mut commands = vec![
        DrawCommand::FillTriangle {
            points: [left, right, tip_right],
            color,
        },
        DrawCommand::FillTriangle {
            points: [left, tip_right, tip_left],
            color,
        },
        DrawCommand::FillRect {
            rect: Rect::new(center.x - round, center.y - round, round * 2.0, round * 2.0),
            color,
            radius: round,
        },
    ];
    if ring.a > 0 {
        // The outline's middle line, half a pixel in from the pointer's sides and round its
        // tip, from the body's outline (0.5 out from `base`) on one side to it on the other.
        // In the color it shows over the fill, so where it overlaps the body's it adds nothing.
        let side = |sign: f32| {
            let outer = half * (1.0 - 0.5 / depth);
            [
                at(base, sign * (outer + 1.0), 0.5),
                at(base, sign * (outer - 0.5 * slant / depth), 0.5),
            ]
        };
        let inner = round - 0.5;
        let turn = sin.atan2(cos);
        let tip: Vec<Point> = if inner > 0.0 {
            (0..=8)
                .map(|step| {
                    let angle = turn + (std::f32::consts::PI - 2.0 * turn) * step as f32 / 8.0;
                    at(base, inner * angle.cos(), center_depth + inner * angle.sin())
                })
                .collect()
        } else {
            vec![center]
        };
        let [far, near] = side(-1.0);
        commands.push(DrawCommand::Polyline {
            points: side(1.0).into_iter().chain(tip).chain([near, far]).collect(),
            color: mix(color, Color { a: color.a, ..ring }, f32::from(ring.a) / 255.0),
            width: 1.0,
            dashed: false,
        });
    }
    commands
}

fn place_tooltip(
    anchor: Rect,
    desired: Size,
    viewport: Rect,
    preferred: TooltipPlacement,
    gap: f32,
) -> (Rect, TooltipPlacement) {
    let inset = 8.0;
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
            gap,
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
            gap,
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

impl LayoutBuilder<'_> {
    /// Give what was painted since command `start` the shared entrance and,
    /// if it `leaves`, keep its paint to fade out once it stops being painted.
    /// Used by everything that is not a popover: tooltips, context menus and
    /// elements with [`Element::animate_enter`] or `animate_presence`.
    /// Returns how far it has come in, from 0 to 1.
    fn animate_surface(
        &mut self,
        id: &str,
        start: usize,
        rect: Rect,
        slide_sign: f32,
        leaves: bool,
        entrance: SurfaceEntrance,
    ) -> f32 {
        if self.reduced_motion || self.frame.commands.len() == start {
            return 1.0;
        }
        let enter_ms = entrance.ms.unwrap_or(crate::tokens().motion.enter_ms);
        let now = self.now;
        self.observed_popovers.insert(id.to_owned());
        let exits = self.popover_exits;
        let settled = self.settle_entrances;
        let transition = self
            .popover_transitions
            .entry(id.to_owned())
            .or_insert_with(|| {
                // Reopened while leaving: continue from what is still visible.
                let from = if settled { 1.0 } else { exits.get(id).map_or(0.0, |exit| exit.opacity.value(now)) };
                ScalarTransition::new(from, now).easing(entrance.easing)
            });
        transition.retarget(1.0, now, Duration::from_millis(u64::from(enter_ms)));
        let progress = transition.value(now);
        if leaves {
            self.popover_snapshots.insert(
            id.to_owned(),
            PopoverSnapshot {
                commands: self.frame.commands[start..].to_vec(),
                animation_ms: enter_ms,
                close_animation_ms: None,
                open_scale: 1.0,
                morph: None,
                view_key: None,
                rect,
                slide_sign,
                scrim: None,
                sheet_offset: 0.0,
                sheet_exit: None,
                recede: 0.0,
            },
            );
        }
        // A spring passes 1 on its way: the scale overshoots, the fade clamps.
        if progress != 1.0 {
            let origin = entrance
                .origin
                .unwrap_or(Point::new(rect.x + rect.width / 2.0, rect.y + rect.height));
            let slide = (1.0 - progress) * slide_sign * 4.0;
            let scale = entrance.scale + (1.0 - entrance.scale) * progress;
            for command in &mut self.frame.commands[start..] {
                animate_popover_command(command, progress, slide, scale, origin);
            }
        }
        progress
    }

    /// Start the next floating layer, which sees `pointer` unless a layer
    /// above it held the pointer in the last frame.
    fn enter_layer(&mut self, pointer: Option<Point>) {
        self.frame.layers.push(self.frame.regions.len());
        self.pointer = pointer.filter(|_| self.frame.layers.len() >= self.pointer_layer);
    }

    /// The innermost named hover region under the pointer that nothing laid
    /// over it takes the pointer from: a popover, a dialog's backdrop or a
    /// context menu, placed after the base tree's `base_regions` hit regions.
    fn hovered_region(&mut self, base_regions: usize) -> Option<String> {
        let point = self.pointer?;
        let regions = &self.frame.regions;
        let index = self.hovered_regions.iter().rposition(|(_, end)| {
            let above = (*end).max(base_regions).min(regions.len());
            !regions[above..].iter().any(|region| region.rect.contains(point))
        })?;
        Some(std::mem::take(&mut self.hovered_regions[index].0))
    }

    /// Paint only: fade what was painted since command `start` in or out
    /// with the hover of the region around it. Layout, hit regions and the
    /// focus order are the same either way. Returns how far it is shown,
    /// from 0 hidden or dimmed to 1.
    fn reveal_on_hover(&mut self, element: &Element, id: &str, start: usize, reveal: HoverReveal) -> f32 {
        let concealed = reveal == HoverReveal::Concealed;
        let shown = if concealed {
            !self.region_hovered
        } else {
            self.region_hovered || holds_reveal(element, self.keyboard_focus, self.focused)
        };
        let hidden = element.kept.as_ref().and_then(|kept| kept.dim).unwrap_or(0.0);
        let target = if shown { 1.0 } else { hidden };
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
        if hidden < 1.0 { ((opacity - hidden) / (1.0 - hidden)).clamp(0.0, 1.0) } else { 1.0 }
    }

    /// Paint only: an element `shown` of the way in, from 0 to 1, has the
    /// fills painted since command `start` grown from its `grow_from` height
    /// and everything moved by what is left of its `enter_from` offset.
    fn grow_and_travel(&mut self, element: &Element, start: usize, shown: f32) {
        let Some(kept) = element.kept.as_ref().filter(|_| shown < 1.0 && !self.reduced_motion) else {
            return;
        };
        let away = 1.0 - shown;
        for command in &mut self.frame.commands[start..] {
            if let Some(from) = kept.grow_from {
                let grow = |rect: &mut Rect| {
                    let height = rect.height + (from - rect.height) * away;
                    rect.y += (rect.height - height) / 2.0;
                    rect.height = height;
                    height / 2.0
                };
                match command {
                    DrawCommand::FillRect { rect, radius, .. } => *radius = radius.min(grow(rect)),
                    DrawCommand::FillCornerRect { rect, radii, .. } => {
                        let half = grow(rect);
                        for radius in [&mut radii.top_left, &mut radii.top_right, &mut radii.bottom_right, &mut radii.bottom_left] {
                            *radius = radius.min(half);
                        }
                    }
                    _ => {}
                }
            }
            if let Some(offset) = kept.enter_from {
                *command = command.moved(offset.x * away, offset.y * away, 1.0);
            }
        }
    }

    /// Paint only: turn, stretch and move the icons painted since command
    /// `start` as `motion` has them, playing while `hovered` and returning
    /// to rest after.
    fn move_icons(&mut self, id: &str, motion: &crate::IconMotion, hovered: bool, start: usize) {
        if self.reduced_motion {
            return;
        }
        let now = self.now;
        let rest = crate::motion::IconPose::REST;
        let pose = {
            let mut states = states_mut!(self);
            let state = states.touch(id, now, || IconMotionState { hovered: false, since: now, from: rest, settles: now, looping: false });
            if state.hovered != hovered {
                state.from = state.pose(motion, now);
                state.hovered = hovered;
                state.since = now;
                let length = if hovered { motion.play_ms() } else { motion.rest_ms() };
                state.settles = now + Duration::from_millis(u64::from(length));
            }
            state.pose(motion, now)
        };
        self.pose_icons(start, pose);
    }

    /// Paint only: the icons painted since command `start` where `motion`
    /// has them, played over and over since the element was first painted.
    fn loop_icons(&mut self, id: &str, motion: &crate::IconMotion, start: usize) {
        let round = Duration::from_millis(u64::from(motion.play_ms()));
        if self.reduced_motion || round.is_zero() {
            return;
        }
        let now = self.now;
        let rest = crate::motion::IconPose::REST;
        let since = states_mut!(self)
            .touch(id, now, || IconMotionState { hovered: true, since: now, from: rest, settles: now, looping: true })
            .since;
        let into = now.saturating_duration_since(since).as_nanos() % round.as_nanos();
        let pose = motion.pose(rest, true, into as f32 / 1e6);
        self.pose_icons(start, pose);
    }

    /// Paint only: turn, stretch and move the icons painted since command
    /// `start` by `pose`.
    fn pose_icons(&mut self, start: usize, pose: crate::motion::IconPose) {
        if pose == crate::motion::IconPose::REST {
            return;
        }
        for command in &mut self.frame.commands[start..] {
            if let DrawCommand::Icon { rect, transform, .. } = command {
                rect.x += pose.offset.x;
                rect.y += pose.offset.y;
                transform.angle += pose.turn.to_radians();
                transform.scale_x *= pose.scale_x;
                transform.scale_y *= pose.scale_y;
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
        let commands = self.frame.commands[start..].to_vec();
        let mut states = states_mut!(self);
        let Some(state) = states.keep::<Replacement>(id, now) else {
            // The first content is simply there; only a change travels.
            states.touch(id, now, || Replacement {
                key: key.to_owned(),
                rect,
                commands,
                leaving: None,
                progress: ScalarTransition::new(1.0, now),
            });
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
                    transform: crate::IconTransform::IDENTITY,
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

    /// Paint a text input's floating label `risen` of the way from resting
    /// inside it, `inset` after the start of its content, to sitting on its
    /// top edge at four fifths of the size, the edge parted behind it.
    #[allow(clippy::too_many_arguments)]
    fn paint_floating_label(&mut self, rect: Rect, content: Rect, inset: f32, label: &str, backdrop: Color, style: Style, risen: f32, focused: bool) {
        let lerp = |from: f32, to: f32| from + (to - from) * risen;
        let font_size = lerp(style.font_size, (style.font_size * 0.8).max(10.0));
        let text_style = Style { font_size, ..style };
        let line = self.metrics_for(text_style).line_height;
        let width = self.text_width(label, text_style);
        // Risen, it keeps clear of a round field's curve.
        let x = lerp(content.x + inset, content.x.max(rect.x + style.radius * 0.8));
        let y = lerp(content.y + (content.height - line) / 2.0, rect.y - line / 2.0);
        if risen > 0.01 {
            let gap = Rect::new(x - 4.0, rect.y - 2.0, width + 8.0, 4.0);
            self.frame.commands.push(DrawCommand::FillRect { rect: gap, color: Color { a: (f32::from(backdrop.a) * risen.min(1.0)) as u8, ..backdrop }, radius: 0.0 });
        }
        // Resting it reads as the placeholder does; risen, white while the field is in use.
        let ink = if focused { style.foreground } else { Color { a: 150, ..style.foreground } };
        self.frame.commands.push(DrawCommand::Text { position: Point::new(x, y), text: label.to_owned(), color: ink, font_size, font_family: style.font_family });
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
        let composed = state.composed().filter(|_| focused);
        let is_placeholder = state.value.is_empty() && composed.is_none() && (!focused || persistent_placeholder);
        let shown = composed.as_ref().map_or_else(|| state.shown().into_owned(), |(text, _)| text.clone());
        let layout = self.layout_cache.borrow_mut().get(&shown, text_style, f32::INFINITY, false);
        let painted = if is_placeholder {
            self.layout_cache.borrow_mut().get(placeholder, text_style, f32::INFINITY, false)
        } else {
            layout.clone()
        };
        let state = self.inputs.get_mut(id).expect("looked up above");
        let caret = layout.caret(composed.as_ref().map_or_else(|| byte_index(&shown, state.cursor), |(_, range)| range.end));
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
            .filter(|_| !is_placeholder && composed.is_none())
            .map(|(start, end)| layout.selection(byte_index(&shown, start)..byte_index(&shown, end)));
        let location = crate::caret_blink::CaretLocation { document: 0, revision: state.value.len() as u64, offset: state.cursor };
        let at = |area: Rect| Rect::new(origin.x + area.x, origin.y + area.y, area.width, area.height);
        let caret = Rect { width: 1.5, ..at(caret) };
        let input_clip = clip.and_then(|outer| outer.intersection(content)).or_else(|| clip.is_none().then_some(content));
        state.ime = input_clip.filter(|_| focused).map(|area| (area, caret));
        let Some(input_clip) = input_clip else {
            return;
        };
        self.frame.commands.push(DrawCommand::PushClip(input_clip));
        for bounds in selection.iter().flatten() {
            self.frame.commands.push(DrawCommand::FillRect { rect: at(*bounds), color: INPUT_SELECTION, radius: 0.0 });
        }
        let ink = if is_placeholder { Color { a: 110, ..style.foreground } } else { style.foreground };
        self.frame.commands.extend(painted.draw(origin, ink));
        if let Some((_, range)) = composed {
            self.frame.commands.extend(composition_underline(&layout, range, origin, ink));
        }
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
        persistent_placeholder: bool,
        initial_value: &str,
        highlights: &[crate::TextHighlight],
    ) {
        let text_style = self.text_style(style);
        let focused = self.focused == Some(id);
        let Some(state) = self.inputs.get(id) else {
            return;
        };
        let composed = state.composed().filter(|_| focused);
        let is_placeholder = state.value.is_empty() && composed.is_none() && (!focused || persistent_placeholder);
        // Highlights follow the host's text; a stale edit, or one being
        // composed, is laid out plain until the host catches up.
        let marked = !highlights.is_empty() && state.value == initial_value && composed.is_none();
        let layout = if marked {
            let spans = highlighted_spans(&state.value, highlights, style);
            self.layout_cache.borrow_mut().formatted(&spans, text_style, content.width, None)
        } else {
            let shown = composed.as_ref().map_or(state.value.as_str(), |(text, _)| text.as_str());
            self.layout_cache.borrow_mut().get(shown, text_style, content.width, false)
        };
        let painted = if is_placeholder {
            self.layout_cache.borrow_mut().get(placeholder, text_style, content.width, false)
        } else {
            layout.clone()
        };
        let state = self.inputs.get_mut(id).expect("looked up above");
        let lines: Vec<crate::TextLine> = layout.lines().collect();
        let (cursor_row, caret) = match &composed {
            // The caret stands at the end of what is composed.
            Some((_, range)) => {
                let caret = layout.caret(range.end);
                (lines.iter().take_while(|line| line.rect.y <= caret.y + caret.height / 2.0).count().saturating_sub(1), caret)
            }
            None => state.caret_line(&layout),
        };
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
        let selection = state.selected_range().filter(|_| !is_placeholder && composed.is_none()).map(|(start, end)| {
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
        let input_clip = clip.and_then(|outer| outer.intersection(content)).or_else(|| clip.is_none().then_some(content));
        state.ime = input_clip.filter(|_| focused).map(|area| (area, Rect { width: 1.5, ..at(caret) }));
        if let Some(input_clip) = input_clip {
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
            if let Some((_, range)) = composed {
                self.frame.commands.extend(composition_underline(&layout, range, origin, ink));
            }
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
    fn paint_laid_out_text(
        &mut self,
        value: &str,
        content: Rect,
        style: Style,
        clip: Option<Rect>,
        ink: Option<&[Color]>,
        effects: &[TextEffect],
    ) {
        let layout = self.laid_out_text(value, style, content.width, style.center_content);
        let origin = Point::new(content.x, content.y);
        let block_index = self.frame.text_blocks.len();
        self.paint_text_effects(effects, || {
            let drawn = layout.draw(origin, style.foreground);
            drawn.into_iter().filter(|command| matches!(command, DrawCommand::Glyphs(_))).collect()
        });
        let runs = self.paint_layout(&layout, value, origin, style, clip, block_index, ink);
        self.frame.text_blocks.push(SelectableTextBlock {
            source: value.to_owned(),
            scope: self.selection_scope,
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
                options: crate::tokens().tooltip_options,
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
        let runs = self.paint_layout(&layout, &source, origin, style, clip, block_index, None);
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
            scope: self.selection_scope,
            runs,
            region_layer,
            layout: Some(PlacedText { layout, origin }),
        });
        self.show_text_hints(&hints);
    }

    /// What every text laid out in a `TextLayout` paints: its selection under
    /// it, then what the layout draws. Gives its lines as selectable runs of
    /// `source`.
    /// Paint `layout` from `origin`, in the foreground or, given `ink` of two stops or more,
    /// in them across each of its lines as `foreground_gradient` says.
    #[allow(clippy::too_many_arguments)]
    fn paint_layout(
        &mut self,
        layout: &crate::TextLayout,
        source: &str,
        origin: Point,
        style: Style,
        clip: Option<Rect>,
        block_index: usize,
        ink: Option<&[Color]>,
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
        if let Some((from, to)) =
            self.selection.and_then(|selection| selection.range_in(block_index, self.selection_scope, source))
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
        match ink.filter(|stops| stops.len() > 1) {
            // As the measured path paints a gradient: the same glyphs in each band of each line.
            Some(stops) => {
                let opacity = f32::from(style.foreground.a) / 255.0;
                for run in &runs {
                    for (area, color) in gradient_bands(run.rect, stops, opacity, clip) {
                        self.frame.commands.push(DrawCommand::PushClip(area));
                        self.frame.commands.extend(layout.draw(origin, color));
                        self.frame.commands.push(DrawCommand::PopClip);
                    }
                }
            }
            None => self.frame.commands.extend(layout.draw(origin, style.foreground)),
        }
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
        if let Some(selection) = self.selection {
            if let Some((start, end)) = selection.range_in(block_index, self.selection_scope, source) {
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
            FontFamily::Monospace | FontFamily::MonospaceBold => self.metrics,
            FontFamily::Sans
            | FontFamily::SansItalic
            | FontFamily::SansMedium
            | FontFamily::SansSemibold
            | FontFamily::SansBold
            | FontFamily::SansBoldItalic
            | FontFamily::Display
            | FontFamily::Custom(_) => self.sans_metrics,
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

    /// `text` in `rect`, inked with `stops` from its left edge to its right:
    /// bands clipped side by side, each in the color at its middle, as the
    /// shimmer paints its sweep.
    fn paint_gradient_line(
        &mut self,
        text: &str,
        rect: Rect,
        style: Style,
        stops: &[Color],
        clip: Option<Rect>,
    ) {
        let opacity = f32::from(style.foreground.a) / 255.0;
        for (area, color) in gradient_bands(rect, stops, opacity, clip) {
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

    /// `effects` of the text `runs` gives, each its own command, before the
    /// text is drawn. Without effects the runs are never made.
    fn paint_text_effects(&mut self, effects: &[TextEffect], runs: impl FnOnce() -> Vec<DrawCommand>) {
        if effects.is_empty() {
            return;
        }
        let runs = runs();
        for effect in effects {
            self.frame.commands.push(DrawCommand::TextEffect { effect: *effect, runs: runs.clone() });
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
            transform: crate::IconTransform::IDENTITY,
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
                collect_focus_order(row, &mut focusables, &mut crate::FastSet::default());
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
        #[cfg(debug_assertions)]
        self.measuring.set(self.measuring.get() + 1);
        let size = self.measure_now(element, available);
        #[cfg(debug_assertions)]
        self.measuring.set(self.measuring.get() - 1);
        size
    }

    fn measure_now(&self, element: &Element, available: Size) -> Size {
        let mut size = self.measure_natural(element, available);
        if let (Some(expand), Some((id, _))) = (element.animate_height, &element.presence) {
            size.height = self.flow_height(id, expand, size.height);
        }
        // A pushing reach is room taken: as far as its spring has got.
        if let Some(kept) = element.kept.as_ref().filter(|kept| kept.extend_pushes) {
            if let Some((id, by)) = &kept.extend {
                let length = by.x.abs().max(by.y.abs());
                if length > 0.0 {
                    let out = self.states.borrow().get::<AnimatedSpring>(&format!("extend/{id}")).map_or(0.0, |state| state.value.value(self.now));
                    size.width = (size.width + by.x.abs() * out / length).max(0.0);
                    size.height = (size.height + by.y.abs() * out / length).max(0.0);
                }
            }
        }
        size
    }

    /// The height an `animate_height` slot occupies now, given its content's.
    /// Measuring changes nothing: it works out where the slot's spring stands
    /// from where the last layout left it, and notes the content's height for
    /// the placing, which keeps the spring started or reversed.
    fn flow_height(&self, id: &str, expand: bool, natural: f32) -> f32 {
        if self.reduced_motion {
            return natural;
        }
        self.flow_naturals.borrow_mut().insert(id.to_owned(), (natural, expand));
        let before = self.states.borrow().get::<FlowHeight>(id).map(|state| (state.fraction, state.natural));
        let (fraction, natural) = next_flow(before, expand, natural, self.now, self.locating);
        natural * (fraction.value(self.now) / SLOT_OPEN).clamp(0.0, 1.0)
    }

    /// Keep what measuring found for the `animate_height` slot `id`: its
    /// spring started or reversed, and its content's height.
    fn keep_flow(&self, id: &str) {
        let Some((natural, expand)) = self.flow_naturals.borrow_mut().remove(id) else {
            return;
        };
        let before = self.states.borrow().get::<FlowHeight>(id).map(|state| (state.fraction, state.natural));
        let (fraction, natural) = next_flow(before, expand, natural, self.now, self.locating);
        let mut states = states_mut!(self);
        let state = states.touch(id, self.now, || FlowHeight {
            fraction,
            natural,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            commands: Vec::new(),
        });
        state.fraction = fraction;
        state.natural = natural;
    }

    /// Place an `animate_height` element in a slot that may be shorter than
    /// its content: clipped to the slot and faded, or, once the content has
    /// left, its last paint alone, with no regions.
    fn place_flow_height(&mut self, element: &Element, id: &str, rect: Rect, clip: Option<Rect>) {
        if self.reduced_motion {
            return self.place_inner(element, rect, clip);
        }
        if !self.flow_naturals.borrow().contains_key(id) {
            // A parent that sizes its child without measuring it.
            self.measure(element, Size::new(rect.width, rect.height));
        }
        self.keep_flow(id);
        let now = self.now;
        let Some((fraction, natural, present)) = self.states.borrow().get::<FlowHeight>(id).map(|state| {
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
            if let Some(state) = states_mut!(self).get_mut::<FlowHeight>(id) {
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
            if let Some(state) = states_mut!(self).get_mut::<FlowHeight>(id) {
                state.rect = full;
                state.commands = self.frame.commands[inner..].to_vec();
            }
            for command in &mut self.frame.commands[inner..] {
                animate_popover_command(command, fraction, 0.0, 1.0, origin);
            }
            self.frame.commands.push(DrawCommand::PopClip);
        } else if let (Some(slot), Some(state)) = (slot, self.states.borrow().get::<FlowHeight>(id)) {
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
        let mut states = states_mut!(self);
        let state = states.touch(key, self.now, || AnimatedScalar {
            value: ScalarTransition::new(target, self.now),
        });
        state.value = self.next_scalar(state.value, target);
        state.value.value(self.now)
    }

    /// As [`Self::scalar`], for a measure: where the value stands, changing nothing.
    fn scalar_peek(&self, key: &str, target: f32) -> f32 {
        let before = self.states.borrow().get::<AnimatedScalar>(key).map(|state| state.value);
        self.next_scalar(before.unwrap_or(ScalarTransition::new(target, self.now)), target).value(self.now)
    }

    /// A scalar's transition once it heads for `target`.
    fn next_scalar(&self, mut value: ScalarTransition, target: f32) -> ScalarTransition {
        if value.target() != target {
            // A count is read, not followed: it takes three moves.
            let duration = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3);
            value.retarget_with_motion(target, self.now, duration, self.reduced_motion);
        }
        value
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
    /// aurora's interactive glass under a press, from its `Motion.qml`: `press` (0.30 s, bounce
    /// 0.35), `pressScale` 1.075, `glowIn` 120 ms and `glowOut` 300 ms.
    fn press_glass(&mut self, id: &str, rect: Rect, start: usize) {
        let inside = self.pointer.filter(|point| rect.contains(*point));
        let pressed = self.pressed.is_some() && inside.is_some();
        let omega = std::f32::consts::TAU / 0.30;
        let press = crate::SpringMotion { stiffness: omega * omega, damping_ratio: 0.65 };
        let lift = self.spring_moving(&format!("glass-press/{id}"), if pressed { 1.0 } else { 0.0 }, press);
        let glow = self.spring_paced(&format!("glass-glow/{id}"), if pressed { 1.0 } else { 0.0 }, 120, 300).clamp(0.0, 1.0);
        if lift.abs() < 1e-3 && glow < 1e-3 {
            return;
        }
        let at = inside.map_or((0.5, 0.5), |point| ((point.x - rect.x) / rect.width.max(1.0), (point.y - rect.y) / rect.height.max(1.0)));
        let scale = 1.0 + 0.075 * lift;
        let centre = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let mut lit = false;
        for command in &mut self.frame.commands[start..] {
            if let (DrawCommand::Glass { material, .. }, false) = (&mut *command, lit) {
                (material.glow, material.glow_at, lit) = (glow, at, true);
            }
            animate_popover_command(command, 1.0, 0.0, scale, centre);
        }
    }

    /// Lays an iOS sheet out at the height a finger has taken it to or its spring stands at:
    /// `floating` takes the size, corners and material that height gives it. `morphs`: it
    /// grows out of the button that opened it, so it does not rise from the foot.
    fn sheet_layout(&mut self, floating: &mut Element, spec: &crate::SheetSpec, dismiss_id: &str, viewport: Size, morphs: bool) -> SheetPaint {
        use crate::sheet_geometry as geometry;
        let floats = spec.screen_corner > 0.0;
        let width = (viewport.width - if floats { 2.0 * geometry::FLOAT_INSET } else { 0.0 }).max(0.0);
        floating.style.width = Some(width);
        floating.style.height = None;
        floating.style.max_height = None;
        let natural = self.measure(floating, Size::new(width, f32::INFINITY)).height;
        let heights = geometry::heights(spec, natural, viewport.height);
        let smallest = heights.first().copied().unwrap_or(natural);
        let resting = self.sheet_detents.get(dismiss_id).copied().unwrap_or(spec.opens_at).min(heights.len().saturating_sub(1));
        let target = heights.get(resting).copied().unwrap_or(natural);
        let key = format!("__sheet:{dismiss_id}");
        let height = match self.sheet_hold.filter(|(id, _)| *id == dismiss_id) {
            Some((_, held)) => {
                // The spring waits, with the speed the finger gives it, for the release.
                states_mut!(self).keep::<AnimatedSpring>(&key, self.now);
                held
            }
            None => {
                // First seen, it rises from the foot of the screen; grown out of a button, it
                // is there at once as the button's glass becomes it.
                if !morphs && !self.reduced_motion {
                    let now = self.now;
                    states_mut!(self).touch(&key, now, || AnimatedSpring { value: ScalarSpring::new(0.0, now) });
                }
                self.spring_moving(&key, target, sheet_motion())
            }
        };
        let shape = geometry::shape(spec, height, smallest, viewport.width, viewport.height);
        floating.style.width = Some(shape.rect.width);
        floating.style.height = Some(shape.rect.height);
        floating.style.corner_radii = Some(CornerRadii { top_left: shape.top_radius, top_right: shape.top_radius, bottom_right: shape.bottom_radius, bottom_left: shape.bottom_radius });
        // Glass and its shadow round all four corners alike: the screen's own corners hide the
        // foot's once the sheet anchors to it.
        floating.style.radius = shape.top_radius;
        // Glass at partial heights, growing opaque as it anchors to the screen's edges.
        if let Some(glass) = floating.kept.as_mut().and_then(|kept| kept.glass.as_mut()) {
            glass.tint.a = (f32::from(glass.tint.a) + (255.0 - f32::from(glass.tint.a)) * shape.anchored).round() as u8;
            // Its corners are round arcs, concentric with the display's, not a pane's squircle,
            // and it bends what is behind it only at its rim, not a corner's depth into it.
            glass.power = 2.0;
            glass.bezel = SHEET_BEZEL;
        }
        let large = geometry::large(spec, viewport.height);
        self.sheet_frames.insert(dismiss_id.to_owned(), SheetFrame { heights, resting, height, viewport: viewport.height, dismissible: spec.dismissible });
        // Only its large height pushes back what it was opened over.
        let recede = if spec.recede && large > 0.0 { ((height - (large - RECEDE_FROM)) / RECEDE_FROM).clamp(0.0, 1.0) } else { 0.0 };
        if recede > 0.0 {
            self.recede = Some((self.recede.map_or(0.0, |(deepest, _)| deepest).max(recede), spec.screen_corner));
        }
        SheetPaint {
            rect: shape.rect,
            shell: shape.shell,
            entered: if smallest > 0.0 { (height / smallest).clamp(0.0, 1.0) } else { 1.0 },
            leave: viewport.height - shape.shell.y + SHEET_EXIT_PAST,
            recede,
        }
    }

    /// How far a bottom sheet stands below its place `rect`: where the finger holding its
    /// handle has taken it, or where its spring is on its way back or to a new height. It
    /// stands in its place while it enters.
    fn sheet_spring(&self, dismiss_id: &str, rect: Rect, entered: f32) -> f32 {
        let key = format!("__sheet:{dismiss_id}");
        let held = self.sheet_held.filter(|(id, _, _)| *id == dismiss_id);
        let rest = |top: f32, speed: f32| {
            let value = ScalarSpring::moving(top, speed, self.now);
            states_mut!(self).touch(&key, self.now, || AnimatedSpring { value }).value = value;
        };
        if let Some((_, down, speed)) = held {
            rest(rect.y + down, speed);
            return down;
        }
        if entered < 1.0 {
            rest(rect.y, 0.0);
            return 0.0;
        }
        self.spring_moving(&key, rect.y, sheet_motion()) - rect.y
    }

    /// As `spring`, on `motion`.
    fn spring_moving(&self, key: &str, target: f32, motion: crate::SpringMotion) -> f32 {
        let mut states = states_mut!(self);
        let state = states.touch(key, self.now, || AnimatedSpring {
            value: ScalarSpring::new(target, self.now),
        });
        if self.reduced_motion {
            state.value = ScalarSpring::new(target, self.now);
        } else if target != state.value.target() {
            state.value.launch(target, motion, self.now);
        }
        state.value.value(self.now)
    }

    fn spring(&self, key: &str, target: f32) -> f32 {
        let motion = crate::tokens().motion.move_ms;
        self.spring_paced(key, target, motion, motion)
    }

    /// As `spring`, covering its way in `rise_ms` towards a greater value
    /// and in `fall_ms` towards a lesser one.
    fn spring_paced(&self, key: &str, target: f32, rise_ms: u16, fall_ms: u16) -> f32 {
        let paced = |settle_ms: u16| crate::SpringMotion::settling_in(Duration::from_millis(u64::from(settle_ms)));
        let mut states = states_mut!(self);
        let state = states.touch(key, self.now, || AnimatedSpring {
            value: ScalarSpring::new(target, self.now),
        });
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
        let mut states = states_mut!(self);
        let state = states.touch(key, self.now, || AnimatedSpring {
            value: ScalarSpring::new(target, self.now).motion(motion),
        });
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
        let states = self.states.borrow();
        let Some(Leaving(previous)) = states.get::<Leaving>(list) else {
            return Cow::Borrowed(children);
        };
        let present = |id: &str| children.iter().any(|child| Self::slot_id(child) == Some(id));
        let closing = |id: &str| {
            states
                .get::<FlowHeight>(id)
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
    /// A rolling number as the text it shows now; a measure only reads its value.
    fn rolling_text(&self, element: &Element, measuring: bool) -> Option<Element> {
        let ElementKind::RollingNumber { id, value, format } = &element.kind else {
            return None;
        };
        let shown = if measuring { self.scalar_peek(id, *value) } else { self.scalar(id, *value) };
        let mut text = element.clone();
        text.kind = ElementKind::Text(format(shown));
        Some(text)
    }

    fn measure_uncached(&self, element: &Element, available: Size) -> Size {
        if let Some(text) = self.rolling_text(element, true) {
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
            ElementKind::RasterImage(_) | ElementKind::Surface { .. } | ElementKind::ZoomView(_) => {
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
                style.icon_side.unwrap_or(ICON_SIZE)
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
                style
                    .icon_side
                    .unwrap_or(ICON_SIZE)
                    .max(metrics.line_height)
                    + padding,
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
            | ElementKind::Scribble { .. }
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
        if element.kept.as_ref().is_some_and(|kept| kept.selection_scope) {
            let scope = self.next_selection_scope;
            self.next_selection_scope += 1;
            let outer = self.selection_scope.replace(scope);
            self.place_unscoped(element, rect, clip);
            self.selection_scope = outer;
        } else {
            self.place_unscoped(element, rect, clip);
        }
    }

    fn place_unscoped(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        let mut placed = Self::translated_rect(element, rect);
        if let Some((id, y)) = element.kept.as_ref().and_then(|kept| kept.slide.as_ref()) {
            // Three moves: long enough to see the rows go by, as a counter rolls.
            let pace = crate::tokens().motion.move_ms.saturating_mul(3);
            placed.y += self.spring_paced(&format!("slide/{id}"), *y, pace, pace);
        }
        if let Some(id) = element.kept.as_ref().and_then(|kept| kept.moving.as_deref()) {
            let offset = self.moved(id, rect);
            placed = Rect::new(placed.x + offset.x, placed.y + offset.y, placed.width, placed.height);
        }
        self.place_translated(element, placed, clip);
    }

    /// How far from `rect` an `animate_move` element kept under `id` is
    /// painted on this frame: from where it was shown when its slot changed,
    /// easing to nothing over `move_ms`.
    fn moved(&mut self, id: &str, rect: Rect) -> Point {
        let now = self.now;
        let slot = Point::new(rect.x - self.flow_origin.x, rect.y - self.flow_origin.y);
        let mut states = states_mut!(self);
        let state = states.touch(id, now, || AnimatedMove {
            slot,
            x: ScalarTransition::new(0.0, now),
            y: ScalarTransition::new(0.0, now),
        });
        if state.slot != slot {
            let duration = Duration::from_millis(u64::from(crate::tokens().motion.move_ms));
            // It sets off from where it is shown, as far from its new slot as that is.
            for (axis, from) in [(&mut state.x, state.slot.x - slot.x), (&mut state.y, state.slot.y - slot.y)] {
                axis.snap_to(axis.value(now) + from, now);
                axis.retarget_with_motion(0.0, now, duration, self.reduced_motion);
            }
            state.slot = slot;
        }
        Point::new(state.x.value(now), state.y.value(now))
    }

    fn place_translated(&mut self, element: &Element, rect: Rect, clip: Option<Rect>) {
        let rect = self.extended(element, rect, clip);
        if let Some((id, offset)) = element.kept.as_ref().and_then(|kept| kept.caret.as_ref()) {
            let location = crate::caret_blink::CaretLocation { document: 0, revision: 0, offset: *offset };
            let shown = self.focused == Some(id.as_str())
                && clip.is_none_or(|clip| rect.intersection(clip).is_some())
                && self.caret_blink.observe(id, location, self.now, self.reduced_motion);
            if !shown {
                return;
            }
        }
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
        let named = element
            .kept
            .as_ref()
            .and_then(|kept| kept.hover_name.clone())
            .filter(|_| reveal == HoverReveal::Region && self.region_hovered)
            .map(|name| {
                self.hovered_regions.push((name, usize::MAX));
                self.hovered_regions.len() - 1
            });
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
        if let Some(index) = named {
            self.hovered_regions[index].1 = self.frame.regions.len();
        }
        // Whoever was inside took it, or nobody was: either way it goes no further.
        let taken = self.access_above.is_none();
        self.access_above = if taken || known { None } else { above };
        self.region_hovered = around;
        if let Some((id, radius)) = element.kept.as_ref().and_then(|kept| kept.soft.as_ref()) {
            self.soften(id, *radius, start);
        }
        if let Some((id, motion)) = element.kept.as_ref().and_then(|kept| kept.icon_motion.as_ref()) {
            let visible = clip.map_or(Some(rect), |clip| rect.intersection(clip));
            let hovered = self.pointer.zip(visible).is_some_and(|(point, visible)| visible.contains(point));
            self.move_icons(id, motion, hovered, start);
        }
        if let Some((id, motion)) = element.kept.as_ref().and_then(|kept| kept.icon_loop.as_ref()) {
            self.loop_icons(id, motion, start);
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
        if let Some(id) = element.kept.as_ref().and_then(|kept| kept.glass_press.as_ref()) {
            self.press_glass(id, rect, start);
        }
        let shown = match &element.presence {
            Some((id, _)) if matches!(reveal, HoverReveal::Revealed | HoverReveal::Concealed | HoverReveal::Rising) => {
                self.reveal_on_hover(element, id, start, reveal)
            }
            Some((id, leaves)) if element.animate_height.is_none() => {
                // Growing or travelling takes the place of the settle.
                let reshaped = element.kept.as_ref().is_some_and(|kept| kept.grow_from.is_some() || kept.enter_from.is_some());
                let sign = if reshaped { 0.0 } else if *leaves { 1.0 } else { -1.0 };
                self.animate_surface(id, start, rect, sign, *leaves, SurfaceEntrance::default())
            }
            _ => 1.0,
        };
        self.grow_and_travel(element, start, shown);
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
        let heading = if pushes { 0.0 } else { self.states.borrow().get::<AnimatedSpring>(&key).map_or(0.0, |state| state.value.target()) };
        let visible = clip.map_or(Some(reach(heading)), |clip| reach(heading).intersection(clip));
        let held = self.pointer.zip(visible).is_some_and(|(point, visible)| visible.contains(point))
            || holds_reveal(element, self.keyboard_focus, self.focused);
        let measured = self.states.borrow().get::<AnimatedSpring>(&key).map_or(0.0, |state| state.value.value(self.now));
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
        let own = focusable_id(element).filter(|_| !is_pointer_only(element));
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
        if let Some(text) = self.rolling_text(element, false) {
            return self.place_inner(&text, rect, clip);
        }
        let foreground = element.kept.as_ref().and_then(|kept| kept.foreground.as_deref());
        if let Some(id) = foreground.filter(|_| !self.reduced_motion) {
            let given = element.style.foreground;
            let ink = {
                let mut states = states_mut!(self);
                let Ink(ink) = states.touch(id, self.now, || Ink(ColorTransition::new(given, self.now)));
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
            let mut held = false;
            if !self.scroll_focus_context.is_empty() {
                // Composite menu rows can be skipped before their controls
                // receive geometry. Keep only their IDs reachable; selecting
                // one uses the existing targeted probe to reveal its bounds.
                let mut focusables = Vec::new();
                collect_focus_order(element, &mut focusables, &mut crate::FastSet::default());
                held = focusable_id(element).is_none() && !focusables.is_empty();
                self.frame
                    .scroll_focus_candidates
                    .extend(focusables.into_iter().map(|(id, _)| id));
            }
            // A remote's arrows go to the nearest control their way, so the
            // ones held out of sight need their bounds too, not only the one
            // being revealed.
            if (held && self.spatial_focus) || self.focus_path.contains(&(element as *const Element as usize)) {
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
            collect_focus_order(element, &mut focusables, &mut crate::FastSet::default());
            self.frame
                .inline_menu_focus
                .push(InlineMenuFocusScope { focusables });
        }
        if let Some(definition) = &element.context_menu {
            if let Some(visible) = clip.map_or(Some(rect), |clip| rect.intersection(clip)) {
                self.frame.context_menu_regions.push(ContextMenuRegion {
                    definition: (**definition).clone(),
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
            // Under reduced motion a skeleton starts on every frame: nothing pulses.
            let started = if self.reduced_motion {
                self.now
            } else {
                states_mut!(self).touch(id, self.now, || SkeletonStart(self.now)).0
            };
            let period = element.kept.as_ref().and_then(|kept| kept.pulse_ms);
            style.background = Some(skeleton_color(
                *base,
                *highlight,
                started,
                self.now,
                self.reduced_motion,
                period.map_or(SKELETON_PERIOD, |ms| Duration::from_millis(u64::from(ms))),
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
            if style.press_highlight {
                self.frame.press_highlight_rows.insert(id.clone());
            }
            let hovered = if style.action_menu_item && self.keyboard_focus {
                self.focused == Some(id.as_str())
            } else {
                (self.hovered == Some(id.as_str())
                    || element.hover_peer.as_deref() == self.hovered && self.hovered.is_some())
                    && (!style.press_highlight || self.pressed.is_some())
            };
            // Under a press slid across a touch menu, the row under the finger is the lit one.
            let slid_onto = style.press_highlight && hovered && self.pressed.is_some();
            // A handle dragged in place is held for as long as its drag lasts,
            // wherever the pointer has gone.
            let held = element.drag_in_place && self.drag_source == Some(id.as_str());
            if self.pressed == Some(id.as_str()) && hovered || held || slid_onto {
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
                style.border = style.hover_border.or(style.border);
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
            let target = style.background.unwrap_or(Color::TRANSPARENT);
            let mut states = states_mut!(self);
            let Fill(state) = states.touch(id, self.now, || Fill(ColorTransition::new(target, self.now)));
            state.retarget(
                target,
                self.now,
                Duration::from_millis(u64::from(style.transition_ms)),
            );
            let color = state.color(self.now);
            style.background = (color.a > 0).then_some(color);
        }
        let metrics = self.metrics_for(style);
        // A zoom view's picture casts its shadow, not its box.
        if let Some(shadow) = style.shadow.filter(|_| !matches!(element.kind, ElementKind::ZoomView(_))) {
            self.frame.commands.push(DrawCommand::Shadow {
                rect,
                shadow,
                radius: style.radius,
            });
        }
        if let Some(material) = element.kept.as_ref().and_then(|kept| kept.glass) {
            self.frame.commands.push(DrawCommand::Glass {
                rect,
                radius: style.radius,
                material: material.normalized(),
                opacity: 1.0,
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
                let outer = std::mem::replace(&mut self.flow_origin, Point::new(content.x, content.y));
                let kept = self.with_leaving(element, children);
                let previous_scope = matches!(&kept, Cow::Owned(_)).then(|| self.enter_measurement_scope());
                let children = kept.as_ref();
                if let Some(list) = element.kept.as_ref().and_then(|kept| kept.leaving.as_deref()) {
                    let slots = children.iter().filter_map(Self::slot_id).map(str::to_owned).collect();
                    *states_mut!(self).touch(list, self.now, || Leaving(Vec::new())) = Leaving(slots);
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
                self.flow_origin = outer;
            }
            ElementKind::Row(children) => {
                let outer = std::mem::replace(&mut self.flow_origin, Point::new(content.x, content.y));
                for (child, rect) in children
                    .iter()
                    .zip(self.child_rects(element, children, content))
                {
                    self.place(child, rect, clip);
                }
                self.flow_origin = outer;
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
                    if let Some(mut target) = index.and_then(|index| rects.get(index)).copied() {
                        if let Some(width) = selection.width {
                            if index == Some(children.len().saturating_sub(1)) { target.x += target.width - width; }
                            target.width = width;
                        }
                        if let Some(height) = selection.height {
                            target.y += (target.height - height) / 2.0;
                            target.height = height;
                        }
                        if selection.glass.is_some() && matches!(selection.selected_id.as_str(), "on" | "off") {
                            if let Some(id) = &selection.press_id {
                                self.frame.glass_switches.insert(id.clone(), GlassSwitchGeometry { bounds: rect, checked: selection.selected_id == "on" });
                            }
                        }
                        let mut states = states_mut!(self);
                        let state = states.touch(&selection.group_id, self.now, || TabIndicatorState {
                                selected_id: selection.selected_id.clone(),
                                x: ScalarSpring::new(target.x, self.now),
                                width: ScalarSpring::new(target.width, self.now),
                                lift_x: ScalarSpring::new(0.0, self.now).motion(if selection.press_id.is_some() { crate::SpringMotion { stiffness: 900.0, damping_ratio: 0.7 } } else { crate::SpringMotion { stiffness: 250.0, damping_ratio: 0.60 } }),
                                lift_y: ScalarSpring::new(0.0, self.now).motion(if selection.press_id.is_some() { crate::SpringMotion { stiffness: 900.0, damping_ratio: 0.7 } } else { crate::SpringMotion { stiffness: 250.0, damping_ratio: 0.70 } }),
                                material_presence: ScalarSpring::new(0.0, self.now).motion(crate::SpringMotion { stiffness: 1000.0, damping_ratio: 1.0 }),
                                travel_from: target.x,
                                press_start: None,
                                dragging: false,
                            });
                        // A lens of a strip of tabs, not a switch's thumb nor a segmented knob,
                        // under a finger held on one of its tabs.
                        let scrubbing = selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none()
                            && action_ids.iter().any(|id| self.pressed == Some(id.as_str()));
                        if selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none() {
                            self.frame.glass_tabs.push(children.iter().zip(rects.iter()).filter_map(|(child, rect)| match &child.kind {
                                ElementKind::Button { id, .. } | ElementKind::IconButton { id, .. } | ElementKind::Clickable { id, .. } => Some((id.clone(), *rect)),
                                _ => None,
                            }).collect());
                        }
                        if self.reduced_motion || selection.animation_ms == 0 {
                            state.x.snap_to(target.x, self.now);
                            state.width.snap_to(target.width, self.now);
                            state.selected_id.clone_from(&selection.selected_id);
                        } else if state.selected_id != selection.selected_id {
                            let duration = Duration::from_millis(u64::from(selection.animation_ms));
                            let motion = if selection.glass.is_some() && selection.press_id.is_some() {
                                crate::SpringMotion { stiffness: 700.0, damping_ratio: 0.78 }
                            } else if selection.glass.is_some() && selection.lift.is_some() {
                                aurora_spring(0.30, 0.15)
                            } else if selection.glass.is_some() {
                                crate::SpringMotion { stiffness: 280.0, damping_ratio: 0.938 }
                            } else { crate::SpringMotion::settling_in(duration) };
                            // Setting a pace before rebasing would redraw the
                            // flight so far on the new curve: a jump.
                            state.travel_from = state.x.value(self.now);
                            state.x.launch(target.x, motion, self.now);
                            state.width.launch(target.width, motion, self.now);
                            state.selected_id.clone_from(&selection.selected_id);
                        } else if !scrubbing && (state.x.target() != target.x
                            || state.width.target() != target.width)
                        {
                            // Resizing a dock or window moves the entire strip;
                            // only changing selection should leave a sliding trail.
                            if state.dragging {
                                // Let go off its tab, the lens travels back to it lifted, as it left.
                                state.travel_from = state.x.value(self.now);
                                state.x.launch(target.x, crate::SpringMotion { stiffness: 700.0, damping_ratio: 0.78 }, self.now);
                            } else { state.x.snap_to(target.x, self.now); }
                            state.width.snap_to(target.width, self.now);
                        }
                        let parent_pressed = selection.press_id.as_deref().is_some_and(|id| self.pressed == Some(id));
                        if let (true, Some(pointer)) = (scrubbing, self.pointer) {
                            // iOS 26's tab bar: held, its lens goes under the finger and follows
                            // it along the bar, centred on it, until the finger lets go.
                            let left = rects.first().map_or(target.x, |rect| rect.x);
                            let right = rects.last().map_or(target.x, |rect| rect.x + rect.width - target.width);
                            state.dragging = true;
                            let goal = (pointer.x - target.width / 2.0).clamp(left, right.max(left));
                            if self.reduced_motion { state.x.snap_to(goal, self.now) } else { state.x.launch(goal, SCRUB_SPRING, self.now) }
                        } else if selection.glass.is_some() && parent_pressed {
                            if let Some(pointer) = self.pointer {
                                let (press_x, start_x) = *state.press_start.get_or_insert((pointer.x, state.x.value(self.now)));
                                if (pointer.x - press_x).abs() > 3.0 {
                                    state.dragging = true;
                                    let left = rects.first().map_or(target.x, |rect| rect.x);
                                    let right = rects.last().map_or(target.x, |rect| rect.x + rect.width - target.width);
                                    state.x.snap_to((start_x + pointer.x - press_x).clamp(left, right), self.now);
                                }
                            }
                        } else {
                            state.press_start = None;
                            state.dragging = false;
                        }
                        let x = state.x.value(self.now);
                        let remaining = (target.x - x).abs();
                        let distance = (target.x - state.travel_from).abs();
                        let pressed = selection.press_id.as_deref().is_some_and(|id| self.pressed == Some(id))
                            || action_ids.iter().any(|id| self.pressed == Some(id.as_str()));
                        // aurora's knob is held up while more than 45% of a segment is left
                        // to go; BitChord's lens until 92% of the way.
                        let lifting = selection.glass.is_some() && (pressed || match selection.lift {
                            Some(_) => remaining > 0.45 * target.width,
                            None => distance > 0.1 && remaining > distance * 0.08,
                        });
                        if self.reduced_motion || selection.glass.is_none() {
                            state.lift_x.snap_to(0.0, self.now);
                            state.lift_y.snap_to(0.0, self.now);
                            state.material_presence.snap_to(0.0, self.now);
                        } else if selection.lift.is_some() {
                            // Lifted on the press spring, landed on the bouncy one.
                            let goal = if lifting { 1.0 } else { 0.0 };
                            if state.lift_x.target() != goal {
                                let motion = if lifting { aurora_spring(0.30, 0.35) } else { aurora_spring(0.42, 0.55) };
                                state.lift_x.launch(goal, motion, self.now);
                                state.lift_y.launch(goal, motion, self.now);
                            }
                            state.material_presence.retarget(goal, self.now);
                        } else {
                            state.lift_x.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                            state.lift_y.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                            state.material_presence.retarget(if lifting { 1.0 } else { 0.0 }, self.now);
                        }
                        let lift_x = state.lift_x.value(self.now);
                        let lift_y = state.lift_y.value(self.now);
                        let presence = state.material_presence.value(self.now).clamp(0.0, 1.0);
                        // A switch's thumb stretches as it is held. A tab's lens stands as tall as
                        // the whole strip and LENS_LIFT more, in its cell's proportions, as
                        // BitChord's tab bar's does while it travels.
                        let (grow_x, grow_y) = if selection.press_id.is_some() {
                            (if pressed { 0.18 } else { 0.12 }, if pressed { 0.18 } else { -0.06 })
                        } else if let Some(scale) = selection.lift {
                            (scale - 1.0, scale - 1.0)
                        } else {
                            let grow = ((rect.height + LENS_LIFT) / target.height.max(1.0) - 1.0).max(0.0);
                            (grow, grow)
                        };
                        // A tab bar's lens dragged fast stretches along its way, thinner, as a drop.
                        let stretch = if selection.glass.is_some() && selection.press_id.is_none() && selection.lift.is_none() {
                            (state.x.velocity(self.now).abs() / SCRUB_STRETCH * 0.1).min(0.2)
                        } else { 0.0 };
                        let width = state.width.value(self.now) * (1.0 + lift_x * grow_x) * (1.0 + stretch);
                        let height = target.height * (1.0 + lift_y * grow_y) * (1.0 - stretch / 2.0);
                        let indicator = Rect::new(x - (width - state.width.value(self.now)) / 2.0,
                            target.y - (height - target.height) / 2.0, width, height);
                        // Its corners grow with it: a capsule lifted is a capsule still, as Apple's lens.
                        let radius = (selection.radius * indicator.height / target.height.max(1.0))
                            .min(indicator.width / 2.0).min(indicator.height / 2.0).max(0.0);
                        let pad = selection.glass.map_or(0.0, |material| material.refraction.max(material.blur).max(target.height * 0.2));
                        let local_rect = Rect::new(rect.x - pad, rect.y - pad, rect.width + pad * 2.0, rect.height + pad * 2.0);
                        let command = match selection.glass {
                            Some(material) => DrawCommand::GlassSelection {
                                rect: indicator, radius, material, opacity: presence,
                                backdrop_rect: local_rect,
                                backdrop: self.frame.commands.iter().filter(|command| match command {
                                    DrawCommand::PushClip(_) | DrawCommand::RoundClip { .. } | DrawCommand::PopClip => true,
                                    _ => command.bounds().is_some_and(|bounds| bounds.intersection(local_rect).is_some()),
                                }).filter_map(|command| match command {
                                    DrawCommand::GlassSelection { .. } => None,
                                    DrawCommand::Glass { rect, radius, material, opacity } => Some(DrawCommand::FillRect {
                                        rect: *rect, radius: *radius, color: Color { a: (material.tint.a as f32 * opacity) as u8, ..material.tint },
                                    }),
                                    _ => Some(command.clone()),
                                }).collect(),
                            },
                            None => DrawCommand::FillRect { rect: indicator, color: selection.color, radius },
                        };
                        if selection.glass.is_some() {
                            let color = Color { a: (selection.color.a as f32 * (1.0 - presence)).round() as u8, ..selection.color };
                            self.frame.commands.insert(command_start, DrawCommand::FillRect { rect: indicator, radius, color });
                            self.frame.commands.push(command);
                        }
                        else { self.frame.commands.insert(command_start, command); }
                        if let Some(mut color) = selection.border {
                            if selection.glass.is_some() { color.a = (color.a as f32 * (1.0 - presence)).round() as u8; }
                            self.frame.commands.insert(
                                command_start + 1,
                                DrawCommand::StrokeRect { rect: indicator, color, radius, width: 1.0 },
                            );
                        }
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
                let first = self.frame.commands.len();
                let first_block = self.frame.text_blocks.len();
                self.place(child, content, clip);
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
                morph,
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
                anchor_outset,
                clear_of,
                sheet,
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
                let mut seen = crate::FastSet::default();
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
                    // A trigger the pointer pressed keeps the focus, and gets it back from a
                    // modal it opened: that focus opens nothing, only the keyboard's does; the
                    // pointer opens the card by resting on the trigger.
                    let trigger_focused = focused.filter(|_| self.keyboard_focus);
                    let surface_id = format!("popover_surface:{dismiss_id}");
                    let active = trigger_id
                        .as_deref()
                        .is_some_and(|id| hovered == Some(id) || trigger_focused == Some(id))
                        || hovered == Some(surface_id.as_str())
                        || focusables.iter().any(|(id, _)| {
                            hovered == Some(id.as_str()) || focused == Some(id.as_str())
                        })
                        // What lies inside the trigger holds it too, such as a
                        // hint's anchor in a row that opens a card on hover.
                        || holds_submenu(anchor, hovered, trigger_focused)
                        || holds_submenu(floating, hovered, focused)
                        // The pointer crosses the space the trigger's outset and the gap leave
                        // beside its rows, or beside the parent's surface, on the way in.
                        || beside_trigger(
                            anchor_rect,
                            *anchor_outset,
                            *gap,
                            clear_of.as_deref().and_then(|id| surface_rect(&self.frame, id)),
                            self.pointer,
                        );
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
                    morph: *morph,
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
                    anchor_outset: *anchor_outset,
                    clear_of: clear_of.clone(),
                    sheet: sheet.clone(),
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
                let child_width = appearance
                    .content_width(content.width, overflow(full_width_height, content.height) > 0.0);
                let child_height = if child_width < content.width {
                    self.measure(child, Size::new(child_width, f32::INFINITY))
                        .height
                } else {
                    full_width_height
                };
                let max_offset = overflow(child_height, content.height);
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
                        let mut seen = crate::FastSet::default();
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
            ElementKind::FadedText {
                text,
                fade_width,
                shimmer,
            } => {
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
                let ink = element.kept.as_ref().and_then(|kept| kept.ink_gradient.clone());
                let effects = text_effects(element.kept.as_deref(), style.foreground);
                self.paint_laid_out_text(value, content, style, clip, ink.as_deref(), &effects);
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
            ElementKind::Surface {
                id,
                label,
                fit,
                mirrored,
            } => {
                self.frame.commands.push(DrawCommand::Surface {
                    rect: content,
                    id: id.clone(),
                    label: label.clone(),
                    opacity: 1.0,
                    fit: *fit,
                    radius: style.radius,
                    mirrored: *mirrored,
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
            ElementKind::TextArea {
                id,
                initial_value,
                placeholder,
                persistent_placeholder,
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
                        options: crate::tokens().tooltip_options,
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
                let now = self.now;
                let peeked = peek.as_deref().is_some_and(|control| self.hovered == Some(control));
                let target = if *out || peeked { 1.0 } else { 0.0 };
                let mut states = states_mut!(self);
                let motion = states.touch(id, now, || RecordMotion {
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
                // Under reduced motion a spinner starts on every frame: it does not turn.
                let started = if self.reduced_motion {
                    self.now
                } else {
                    states_mut!(self).touch(id, self.now, || SpinnerStart(self.now)).0
                };
                paint_spinner(
                    &mut self.frame.commands,
                    rect,
                    *color,
                    *variant,
                    started,
                    self.now,
                    self.reduced_motion,
                );
            }
            ElementKind::Scribble { id, color, stroke, drawing } => {
                let started = states_mut!(self).touch(id, self.now, || SpinnerStart(self.now)).0;
                let (commands, moving) = crate::scribble::paint(
                    rect,
                    *color,
                    *stroke,
                    drawing.as_deref(),
                    started,
                    self.now,
                    self.reduced_motion,
                );
                // A drawing written once stops asking for frames when its pen has gone; its
                // start is still kept, so it is not written again.
                if !moving {
                    self.settled_spinners.insert(id.clone());
                }
                self.frame.commands.extend(commands);
            }
            ElementKind::EffortSlider {
                id,
                fraction,
                steps,
                ultra_available,
            } => {
                let drag = self.slider_drag.filter(|drag| drag.id == *id);
                let mut states = states_mut!(self);
                let motion = states.touch(id, self.now, || SliderMotion::new(self.now));
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
            ElementKind::ZoomView(view) => self.place_zoom_view(view, content, clip, style.shadow),
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
        let ids = [focusable_id(element), element.kept.as_ref().and_then(|kept| kept.anchor.as_deref())];
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
                    if self.probes(child) {
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
                    if self.probes(child) {
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
                let child_width = appearance
                    .content_width(content.width, overflow(full_width_height, content.height) > 0.0);
                let child_height = if child_width < content.width {
                    self.measure(child, Size::new(child_width, f32::INFINITY))
                        .height
                } else {
                    full_width_height
                };
                let max_offset = overflow(child_height, content.height);
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
            // A strip that scrolls sideways holds its cards where it last
            // showed them.
            ElementKind::HorizontalScroll { id, child, .. } => {
                let width = self
                    .measure(child, Size::new(f32::INFINITY, content.height))
                    .width
                    .max(content.width);
                let offset = self
                    .horizontal_scrolls
                    .get(id.as_str())
                    .map_or(0.0_f32, HorizontalScrollState::offset)
                    .clamp(0.0, width - content.width);
                self.probe_scroll_focus(child, Rect::new(content.x - offset, content.y, width, content.height), clip);
            }
            ElementKind::Overlay { base, .. } if self.probes(base) => {
                self.probe_scroll_focus(base, content, clip);
            }
            ElementKind::Popover { anchor, .. } if self.probes(anchor) => {
                self.probe_scroll_focus(anchor, content, clip);
            }
            _ => {}
        }
    }

    /// Whether a probe of a branch held out of sight goes into `child`: the
    /// one on the way to the control being revealed, or every one while a
    /// remote's arrows move the focus.
    fn probes(&self, child: &Element) -> bool {
        self.spatial_focus || self.focus_path.contains(&(child as *const Element as usize))
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
        if let Some(anchor) = element.kept.as_mut().and_then(|kept| kept.anchor.take()) {
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
        // A backdrop sharing the anchor's id is not the anchor.
        let on_backdrop = self.pointer.is_some_and(|point| {
            (self.frame.regions.iter().enumerate().rev())
                .find(|(_, region)| region.rect.contains(point))
                .is_some_and(|(index, _)| self.frame.backdrops.contains(&index))
        });
        (self.hovered == Some(anchor_id)
            && !on_backdrop
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
#[path = "tree_tests.rs"]
mod tests;
