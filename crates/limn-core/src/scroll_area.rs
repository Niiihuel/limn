//! A scroll surface whose content, scrollbar layout and scrollbar colors are
//! configured independently. The same appearance is used by ordinary scroll
//! content and lazily built uniform lists.

use crate::motion::ScalarTransition;
use crate::{Color, Component, Element, ScrollMotion, Size};
use web_time::{Duration, Instant};

/// When an ordinary, virtual or horizontal scroll surface shows its indicator.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollbarVisibility {
    /// Keep the configured idle appearance visible. This is the default.
    #[default]
    Always,
    /// Show after scrolling or scrollbar interaction, then hold and fade out.
    /// Initially hidden; hiding never changes the reserved content width.
    /// Hidden vertical bars have no pointer target; a horizontal viewport keeps
    /// its scroll and drag targets. Reduced motion keeps the hold but skips
    /// the fade. Durations are in milliseconds; zero is immediate.
    AutoHide {
        /// How long the bar stays fully visible after the last activity.
        hold_ms: u16,
        /// How long it fades after the hold, skipped under reduced motion.
        fade_ms: u16,
    },
    /// As [`Self::AutoHide`], and also shown while the pointer is over the
    /// scroll surface, as a chat's history shows its bar under the pointer:
    /// the hold and the fade start when the pointer leaves it.
    AutoHideUnlessHovered {
        /// How long the bar stays fully visible after the last activity.
        hold_ms: u16,
        /// How long it fades after the hold, skipped under reduced motion.
        fade_ms: u16,
    },
}

pub(crate) struct ScrollbarState {
    offset: f32,
    activity: Option<Instant>,
    interacting: bool,
    hold_ms: u16,
    opacity: ScalarTransition,
    pub(crate) seen: bool,
}

impl ScrollbarState {
    pub(crate) fn new(offset: f32, now: Instant) -> Self {
        Self {
            offset,
            activity: None,
            interacting: false,
            hold_ms: 0,
            opacity: ScalarTransition::new(0.0, now),
            seen: false,
        }
    }

    pub(crate) fn observe(
        &mut self,
        offset: f32,
        interacting: bool,
        hold_ms: u16,
        fade_ms: u16,
        reduced_motion: bool,
        now: Instant,
    ) -> f32 {
        self.seen = true;
        self.hold_ms = hold_ms;
        if self.offset != offset || interacting || self.interacting {
            self.activity = Some(now);
            self.opacity.snap_to(1.0, now);
        }
        self.offset = offset;
        self.interacting = interacting;
        if let Some(deadline) = self
            .activity
            .map(|at| at + Duration::from_millis(u64::from(hold_ms)))
        {
            if !interacting && now >= deadline {
                if reduced_motion || fade_ms == 0 {
                    self.opacity.snap_to(0.0, now);
                } else {
                    self.opacity
                        .retarget(0.0, deadline, Duration::from_millis(u64::from(fade_ms)));
                }
            }
        }
        self.opacity.value(now)
    }

    pub(crate) fn repaint_after(&self, now: Instant) -> Option<Duration> {
        if self.interacting {
            return None;
        }
        let deadline = self.activity? + Duration::from_millis(u64::from(self.hold_ms));
        if now < deadline {
            Some(deadline - now)
        } else {
            self.opacity.is_active(now).then_some(Duration::ZERO)
        }
    }
}

/// Where the scrollbar sits relative to scroll content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollbarPlacement {
    /// Paint over the content without reducing its layout width.
    Overlay,
    /// Reserve a separate rail for content with controls at its far edge.
    #[default]
    Reserved,
}

/// When a vertical scrollbar paints its track, the stripe its thumb runs
/// along, at [`ScrollbarAppearance::hover_track_opacity`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollbarTrack {
    /// While the scrollbar is under the pointer or its thumb is dragged, so
    /// an overlay rail leaves no stripe over the content at rest. The default.
    #[default]
    WhenActive,
    /// Always, at rest too.
    Always,
    /// Never: the thumb alone.
    Never,
}

/// Visual and pointer policy for a vertical scrollbar, in logical pixels.
///
/// The default reserves a separate rail while content overflows, keeping
/// scrollbar arrows and pointer targets clear of row controls and hovers.
/// The idle track is transparent. An explicit overlay can keep the content
/// width unchanged when the surface leaves room for the scrollbar itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarAppearance {
    /// Whether the rail takes its width from the content or paints over it.
    /// `Reserved` by default.
    pub placement: ScrollbarPlacement,
    /// Show compact up/down controls at the ends of an overflowing rail.
    pub buttons: bool,
    /// Width of the rail, 14 by default. When laid out it is kept between 1 and
    /// 24 and under a third of the viewport's width.
    pub rail_width: f32,
    /// Width of the thumb, 5 by default, centered in the rail. When laid out it
    /// is at least 1 and at most the rail's width less 2.
    pub thumb_width: f32,
    /// The shortest the thumb gets, 24 by default, however long the content. It
    /// is never more than half the track.
    pub min_thumb_length: f32,
    /// `None` uses the scroll element's foreground color, so a theme can tint
    /// the scrollbar through its ordinary typography token.
    pub tint: Option<Color>,
    /// The four opacities run from 0 to 255 and multiply the ink's alpha.
    pub idle_thumb_opacity: u8,
    /// The thumb while the scrollbar is hovered, and the arrow of an end
    /// button under the pointer. 118 by default.
    pub hover_thumb_opacity: u8,
    /// The thumb while it is dragged, and the arrow of an end button held down.
    /// 164 by default.
    pub active_thumb_opacity: u8,
    /// The track, which shows when [`Self::track`] says: by default only while
    /// the scrollbar is hovered or dragged. An end button under the pointer is
    /// filled at twice this, three times when held. 14 by default.
    pub hover_track_opacity: u8,
    /// The corner of the thumb and of its track, in logical pixels. `None`, the
    /// default, rounds their ends fully; `Some(0.0)` draws them square, as
    /// Spotify's desktop client on Windows and Linux. Clamped to half the
    /// thumb's width; a value that is not finite counts as `None`.
    pub thumb_radius: Option<f32>,
    /// How far inside its square an end button's hover fill is drawn, in
    /// logical pixels: 1 by default. A value that is not finite counts as 1, a
    /// negative one as 0.
    pub button_inset: f32,
    /// The corner of an end button's hover fill, in logical pixels. `None`, the
    /// default, takes `tokens().radius.control`; a value that is not finite
    /// counts as `None`, a negative one as 0.
    pub button_radius: Option<f32>,
    /// The largest box an end button's arrow is drawn in, in logical pixels: 7
    /// wide and 4 tall by default. A smaller button draws it smaller still, at
    /// most half the button's width and 0.3 of its height. A side that is not
    /// finite counts as the default's, a negative one as 0.
    pub arrow: Size,
    /// When the track shows behind the thumb: [`ScrollbarTrack::WhenActive`]
    /// by default.
    pub track: ScrollbarTrack,
}

impl Default for ScrollbarAppearance {
    fn default() -> Self {
        Self {
            placement: ScrollbarPlacement::Reserved,
            buttons: true,
            rail_width: 14.0,
            thumb_width: 5.0,
            min_thumb_length: 24.0,
            tint: None,
            idle_thumb_opacity: 72,
            hover_thumb_opacity: 118,
            active_thumb_opacity: 164,
            hover_track_opacity: 14,
            thumb_radius: None,
            button_inset: 1.0,
            button_radius: None,
            arrow: Size::new(7.0, 4.0),
            track: ScrollbarTrack::WhenActive,
        }
    }
}

/// A length from a caller, made safe: `default` when it is not finite, and
/// never below zero.
fn length_or(value: f32, default: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { default }
}

impl ScrollbarAppearance {
    /// The default appearance with the scrollbar painted over the content, whose width does
    /// not change.
    pub fn overlay() -> Self {
        Self {
            placement: ScrollbarPlacement::Overlay,
            ..Self::default()
        }
    }

    /// The default appearance: a separate rail, reserved while the content overflows.
    pub fn reserved() -> Self {
        Self::default()
    }

    /// Draw the scrollbar in this color instead of the scroll element's foreground. The
    /// opacities still apply.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Show or hide the up and down controls at the ends of the rail; shown by default.
    pub fn buttons(mut self, enabled: bool) -> Self {
        self.buttons = enabled;
        self
    }

    /// Set the rail's width in logical pixels, clamped to 4 to 24 (14 by default). A value
    /// that is not finite is ignored.
    pub fn rail_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.rail_width = width.clamp(4.0, 24.0);
        }
        self
    }

    /// Set the thumb's width in logical pixels, clamped to 2 to 12 (5 by default). A value
    /// that is not finite is ignored.
    pub fn thumb_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.thumb_width = width.clamp(2.0, 12.0);
        }
        self
    }

    /// Shared scrollbar ink for native editors and ordinary scroll surfaces.
    pub(crate) fn thumb_color(self, foreground: Color, hovered: bool, active: bool) -> Color {
        self.color_with_opacity(
            foreground,
            if active {
                self.active_thumb_opacity
            } else if hovered {
                self.hover_thumb_opacity
            } else {
                self.idle_thumb_opacity
            },
        )
    }

    pub(crate) fn track_color(self, foreground: Color) -> Color {
        self.color_with_opacity(foreground, self.hover_track_opacity)
    }

    fn color_with_opacity(self, foreground: Color, opacity: u8) -> Color {
        let ink = self.tint.unwrap_or(foreground);
        Color::rgba(
            ink.r,
            ink.g,
            ink.b,
            ((u16::from(ink.a) * u16::from(opacity)) / 255) as u8,
        )
    }

    /// Set the corner of the thumb and its track: 0 draws them square. A value
    /// that is not finite keeps their ends fully round; a negative one is 0.
    pub fn thumb_radius(mut self, radius: f32) -> Self {
        self.thumb_radius = radius.is_finite().then_some(radius.max(0.0));
        self
    }

    /// The corner of a bar `width` wide: what was asked, at most half of it.
    pub(crate) fn corner(self, width: f32) -> f32 {
        let round = width / 2.0;
        self.thumb_radius.filter(|radius| radius.is_finite()).map_or(round, |radius| radius.clamp(0.0, round))
    }

    /// Draw an end button's hover fill `inset` logical pixels inside its
    /// square, 1 by default. A value that is not finite is 1; a negative one
    /// is 0.
    pub fn button_inset(mut self, inset: f32) -> Self {
        self.button_inset = length_or(inset, Self::default().button_inset);
        self
    }

    /// Round an end button's hover fill by `radius` logical pixels in place of
    /// `tokens().radius.control`. A value that is not finite takes the token
    /// again; a negative one is 0.
    pub fn button_radius(mut self, radius: f32) -> Self {
        self.button_radius = radius.is_finite().then_some(radius.max(0.0));
        self
    }

    /// Draw an end button's arrow at most `size` large, in logical pixels: 7
    /// wide and 4 tall by default. A side that is not finite keeps the
    /// default's; a negative one is 0.
    pub fn arrow(mut self, size: Size) -> Self {
        self.arrow = size;
        self.arrow = self.arrow_box();
        self
    }

    /// Choose when the track shows behind the thumb.
    pub fn track(mut self, track: ScrollbarTrack) -> Self {
        self.track = track;
        self
    }

    /// The inset of an end button's hover fill, as painted.
    pub(crate) fn button_fill_inset(self) -> f32 {
        length_or(self.button_inset, Self::default().button_inset)
    }

    /// The corner of an end button's hover fill, as painted.
    pub(crate) fn button_fill_radius(self) -> f32 {
        self.button_radius.filter(|radius| radius.is_finite()).map_or_else(|| crate::tokens().radius.control, |radius| radius.max(0.0))
    }

    /// The box an end button's arrow is drawn in at most, as painted.
    pub(crate) fn arrow_box(self) -> Size {
        let default = Self::default().arrow;
        Size::new(length_or(self.arrow.width, default.width), length_or(self.arrow.height, default.height))
    }

    pub(crate) fn effective_rail_width(self, viewport_width: f32) -> f32 {
        let width = if self.rail_width.is_finite() {
            self.rail_width
        } else {
            Self::default().rail_width
        };
        width.clamp(1.0, 24.0).min(viewport_width / 3.0)
    }

    #[doc(hidden)]
    pub fn content_width(self, viewport_width: f32, overflowing: bool) -> f32 {
        if overflowing && self.placement == ScrollbarPlacement::Reserved {
            viewport_width - self.effective_rail_width(viewport_width)
        } else {
            viewport_width
        }
    }
}

/// Reusable wrapper for a normal or virtualized scroll surface. It preserves
/// the stable element ID used for retained scroll position.
pub struct ScrollArea {
    element: Element,
}

impl ScrollArea {
    /// A scroll surface around `child`. `id` is the ID of the scroll element, under which
    /// its position is retained.
    pub fn new(id: impl Into<String>, child: Element) -> Self {
        Self {
            element: Element::scroll(id, child),
        }
    }

    /// A scroll surface over `row_count` rows `row_height` tall, each built from its index
    /// by `build_row` instead of all at once.
    pub fn uniform_list(
        id: impl Into<String>,
        row_count: usize,
        row_height: f32,
        build_row: impl Fn(usize) -> Element + Send + Sync + 'static,
    ) -> Self {
        Self {
            element: Element::uniform_list(id, row_count, row_height, build_row),
        }
    }

    /// A scroll surface over `row_count` rows of their own heights, each built from its
    /// index by `build_row` as it comes into view and measured then. `row_key` names a row
    /// so its height and the place the view holds follow it when rows are added before it;
    /// `estimated_row_height` stands for a row not measured yet. See
    /// [`Element::measured_list`].
    pub fn measured_list(
        id: impl Into<String>,
        row_count: usize,
        estimated_row_height: f32,
        row_key: impl Fn(usize) -> u64 + Send + Sync + 'static,
        build_row: impl Fn(usize) -> Element + Send + Sync + 'static,
    ) -> Self {
        Self {
            element: Element::measured_list(
                id,
                row_count,
                estimated_row_height,
                row_key,
                build_row,
            ),
        }
    }

    /// Keep a measured list at its end while it is scrolled there, as a conversation
    /// follows new messages: see [`Element::list_follow_end`].
    pub fn follow_end(mut self, follow: bool) -> Self {
        self.element = self.element.list_follow_end(follow);
        self
    }

    /// What row `index` of a uniform list opens under itself: see
    /// [`Element::list_expansion`].
    pub fn expansion(mut self, index: usize, content: Element) -> Self {
        self.element = self.element.list_expansion(index, content);
        self
    }

    /// Replace the whole scrollbar appearance, including a `tint` or `buttons` set before
    /// it.
    pub fn appearance(mut self, appearance: ScrollbarAppearance) -> Self {
        self.element = self.element.scrollbar_appearance(appearance);
        self
    }

    /// Set how wheel and scrollbar-button scrolling is interpolated.
    pub fn motion(mut self, motion: ScrollMotion) -> Self {
        self.element = self.element.scroll_motion(motion);
        self
    }

    /// Choose whether the scrollbar stays visible or follows scroll activity.
    /// Applies equally to ordinary, uniform and measured scroll content.
    pub fn visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.element = self.element.scrollbar_visibility(visibility);
        self
    }

    /// Show or hide the scrollbar's up and down controls, keeping the rest of the
    /// appearance.
    pub fn buttons(mut self, enabled: bool) -> Self {
        match &mut self.element.kind {
            crate::ElementKind::Scroll { appearance, .. } => appearance.buttons = enabled,
            crate::ElementKind::UniformList(list) => list.appearance.buttons = enabled,
            _ => unreachable!("ScrollArea always contains a scroll element"),
        }
        self
    }

    /// Draw the scrollbar in `color`, keeping the rest of the appearance.
    pub fn tint(mut self, color: Color) -> Self {
        match &mut self.element.kind {
            crate::ElementKind::Scroll { appearance, .. } => appearance.tint = Some(color),
            crate::ElementKind::UniformList(list) => list.appearance.tint = Some(color),
            _ => unreachable!("ScrollArea always contains a scroll element"),
        }
        self
    }
}

impl Component for ScrollArea {
    fn render(self) -> Element {
        self.element
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Easing, ElementKind};

    fn policies(element: &Element) -> (ScrollbarAppearance, ScrollMotion) {
        match &element.kind {
            ElementKind::Scroll {
                appearance, motion, ..
            } => (*appearance, *motion),
            ElementKind::UniformList(list) => (list.appearance, list.motion),
            _ => panic!("expected a scroll surface"),
        }
    }

    #[test]
    fn appearance_applies_to_both_scroll_content_types() {
        let appearance = ScrollbarAppearance::reserved().tint(Color::rgb(1, 2, 3));
        let ordinary = ScrollArea::new("ordinary", Element::text("content"))
            .appearance(appearance)
            .render();
        let virtualized = ScrollArea::uniform_list("virtual", 2, 24.0, |_| Element::text("row"))
            .appearance(appearance)
            .render();
        assert!(
            matches!(ordinary.kind, ElementKind::Scroll { appearance: a, .. } if a == appearance)
        );
        assert!(
            matches!(virtualized.kind, ElementKind::UniformList(list) if list.appearance == appearance)
        );
    }

    #[test]
    fn a_thumb_is_round_unless_its_corner_is_set_and_square_at_zero() {
        let thumb = |appearance: ScrollbarAppearance| {
            let tall = Element::column((0..40).map(|_| Element::text("row")).collect());
            let mut tree = crate::UiTree::new(ScrollArea::new("page", tall).appearance(appearance.buttons(false).rail_width(12.0).thumb_width(10.0)).render());
            tree.set_reduced_motion(true);
            let frame = tree.layout(crate::Size::new(200.0, 100.0));
            frame.commands.iter().rev().find_map(|command| match command {
                crate::DrawCommand::FillRect { rect, radius, .. } if rect.width == 10.0 => Some(*radius),
                _ => None,
            }).expect("a thumb")
        };
        assert_eq!(thumb(ScrollbarAppearance::default()), 5.0, "round ends by default");
        assert_eq!(thumb(ScrollbarAppearance::default().thumb_radius(0.0)), 0.0, "square at zero");
        assert_eq!(thumb(ScrollbarAppearance::default().thumb_radius(2.0)), 2.0);
        assert_eq!(thumb(ScrollbarAppearance::default().thumb_radius(40.0)), 5.0, "never past half its width");
        assert_eq!(ScrollbarAppearance::default().thumb_radius(f32::NAN).thumb_radius, None);
        assert_eq!(ScrollbarAppearance::default().thumb_radius(-3.0).thumb_radius, Some(0.0));
    }

    #[test]
    fn an_end_buttons_numbers_from_a_caller_are_made_safe() {
        let default = ScrollbarAppearance::default();
        assert_eq!((default.button_inset, default.button_radius, default.arrow, default.track), (1.0, None, Size::new(7.0, 4.0), ScrollbarTrack::WhenActive));
        assert_eq!(default.button_inset(3.0).button_inset, 3.0);
        assert_eq!(default.button_inset(f32::NAN).button_inset, 1.0, "not finite: the default");
        assert_eq!(default.button_inset(-2.0).button_inset, 0.0);
        assert_eq!(default.button_radius(2.0).button_radius, Some(2.0));
        assert_eq!(default.button_radius(f32::INFINITY).button_radius, None, "not finite: the token");
        assert_eq!(default.button_radius(-1.0).button_radius, Some(0.0));
        assert_eq!(default.arrow(Size::new(9.0, 5.0)).arrow, Size::new(9.0, 5.0));
        assert_eq!(default.arrow(Size::new(f32::NAN, -3.0)).arrow, Size::new(7.0, 0.0));
        // Set on the fields, they are made safe where they are painted.
        let raw = ScrollbarAppearance { button_inset: f32::INFINITY, button_radius: Some(f32::NAN), arrow: Size::new(-1.0, f32::NEG_INFINITY), ..default };
        assert_eq!((raw.button_fill_inset(), raw.button_fill_radius(), raw.arrow_box()), (1.0, crate::tokens().radius.control, Size::new(0.0, 4.0)));
        let raw = ScrollbarAppearance { button_inset: -4.0, button_radius: Some(-4.0), ..default };
        assert_eq!((raw.button_fill_inset(), raw.button_fill_radius()), (0.0, 0.0));
    }

    #[test]
    fn overlay_keeps_content_width_when_overflow_changes() {
        let overlay = ScrollbarAppearance::overlay();
        let reserved = ScrollbarAppearance::reserved();
        assert_eq!(overlay.content_width(100.0, true), 100.0);
        assert_eq!(reserved.content_width(100.0, true), 86.0);
        assert_eq!(reserved.content_width(100.0, false), 100.0);
    }

    #[test]
    fn tint_does_not_replace_a_previous_layout_policy() {
        let element = ScrollArea::new("list", Element::text("content"))
            .appearance(ScrollbarAppearance::reserved())
            .tint(Color::rgb(1, 2, 3))
            .render();
        assert!(
            matches!(element.kind, ElementKind::Scroll { appearance, .. }
            if appearance.placement == ScrollbarPlacement::Reserved
                && appearance.tint == Some(Color::rgb(1, 2, 3)))
        );
    }

    #[test]
    fn motion_and_button_overrides_preserve_each_other_on_both_content_types() {
        let tint = Color::rgb(1, 2, 3);
        let appearance = ScrollbarAppearance::reserved().tint(tint);
        let motion = ScrollMotion {
            duration_ms: 210,
            easing: Easing::InOutCubic,
        };
        let ordinary = ScrollArea::new("ordinary", Element::text("content"))
            .appearance(appearance)
            .motion(motion)
            .buttons(false)
            .render();
        let virtualized = ScrollArea::uniform_list("virtual", 2, 24.0, |_| Element::text("row"))
            .appearance(appearance)
            .buttons(false)
            .motion(motion)
            .render();
        let expected = (appearance.buttons(false), motion);
        assert_eq!(policies(&ordinary), expected);
        assert_eq!(policies(&virtualized), expected);
    }

    #[test]
    fn low_level_elements_can_disable_motion_without_replacing_appearance() {
        let appearance = ScrollbarAppearance::overlay()
            .buttons(false)
            .rail_width(18.0);
        let motion = ScrollMotion {
            duration_ms: 0,
            ..ScrollMotion::default()
        };
        for surface in [
            Element::scroll("ordinary", Element::text("content")),
            Element::uniform_list("virtual", 3, 24.0, |_| Element::text("row")),
        ] {
            let element = surface
                .scroll_motion(motion)
                .scrollbar_appearance(appearance);
            assert_eq!(policies(&element), (appearance, motion));
        }
    }
}
