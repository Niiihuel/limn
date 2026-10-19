//! A scroll surface whose content, scrollbar layout and scrollbar colors are
//! configured independently. The same appearance is used by ordinary scroll
//! content and lazily built uniform lists.

use crate::motion::ScalarTransition;
use crate::{Color, Component, Element, ScrollMotion};
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
    /// The track, which shows only while the scrollbar is hovered or dragged. An
    /// end button under the pointer is filled at twice this, three times when
    /// held. 14 by default.
    pub hover_track_opacity: u8,
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
        }
    }
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

    pub(crate) fn effective_rail_width(self, viewport_width: f32) -> f32 {
        let width = if self.rail_width.is_finite() {
            self.rail_width
        } else {
            Self::default().rail_width
        };
        width.clamp(1.0, 24.0).min(viewport_width / 3.0)
    }

    pub(crate) fn content_width(self, viewport_width: f32, overflowing: bool) -> f32 {
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
