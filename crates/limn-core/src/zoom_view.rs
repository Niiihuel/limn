//! A picture zoomed and panned inside a box: the geometry, kept apart from
//! the tree so it is tested without a window, and the state the tree keeps
//! for each [`Element::zoom_view`](crate::Element::zoom_view) between frames.

use web_time::{Duration, Instant};

use crate::{Easing, Point, Rect, Size, ZoomControl};

/// The largest scale a zoom view reaches: 20 pixels on screen for each
/// pixel of the picture (2000 %, as GNOME's Loupe and Zed), or its fit when
/// that is larger.
pub const ZOOM_VIEW_MAX_SCALE: f32 = 20.0;

/// What one press of a zoom control or key multiplies the scale by (Loupe's).
pub(crate) const STEP: f32 = 1.5;
/// A mouse wheel notch zooms by 1.3 (Loupe's); limn's backends send a notch
/// as 30 logical pixels.
const WHEEL_NOTCH: f32 = 1.3;
const NOTCH: f32 = 30.0;
/// A touchpad's scroll with Ctrl zooms by 1.005 for each logical pixel.
const WHEEL_PIXEL: f32 = 1.005;
/// What a double click at the fit zooms to: 2.5 times it, as Loupe's double
/// tap and Discord's click do.
const TOGGLE: f32 = 2.5;
/// How long a zoom by a control, a key or a double click takes (Loupe's and
/// Telegram's 200 ms).
const ZOOM_MS: u64 = 200;
/// How long a wheel notch's zoom takes: short, so a spin of the wheel
/// follows it.
const WHEEL_MS: u64 = 120;
/// The time constant of a fling's deceleration: speed falls as e^(−t/τ),
/// GTK's friction of 4. It comes to rest after 3τ, along a curve that
/// leaves the pointer at the speed it had.
const GLIDE_TAU: f32 = 0.25;
/// Slower than this, in logical pixels a second, a released drag stops.
const GLIDE_MIN_SPEED: f32 = 120.0;
/// Pointer samples older than this do not count toward a fling's speed.
pub(crate) const GLIDE_WINDOW: Duration = Duration::from_millis(80);
/// How long the scale stays on screen after it changes, and its fade.
const BADGE_MS: u64 = 1_200;
pub(crate) const BADGE_FADE_MS: u64 = 250;

/// Where a picture sits in its box: logical pixels on screen per pixel of the
/// picture, and its top-left corner from the box's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct View {
    pub scale: f32,
    pub x: f32,
    pub y: f32,
}

impl View {
    /// The picture's rectangle inside a box at `room`'s position.
    pub fn rect(self, content: Size, room: Rect) -> Rect {
        Rect::new(room.x + self.x, room.y + self.y, content.width * self.scale, content.height * self.scale)
    }

    fn lerp(self, other: Self, t: f32) -> Self {
        // Scale and corner move together, so the point a zoom is anchored on
        // stays where it is all the way.
        Self {
            scale: self.scale + (other.scale - self.scale) * t,
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// A box and the picture in it, in logical pixels and pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub room: Size,
    pub content: Size,
    /// The scale at which the whole picture fits, never past `upscale`.
    pub fit: f32,
    pub upscale: f32,
}

impl Bounds {
    /// A picture of `content` pixels in a box of `room`, fitted whole and
    /// enlarged to fill it by at most `upscale` times its own size. A side
    /// that is not finite or not above zero counts as one pixel.
    pub fn new(room: Size, content: Size, upscale: f32) -> Self {
        let side = |value: f32| if value.is_finite() && value > 0.0 { value } else { 1.0 };
        let room = Size::new(room.width.max(0.0), room.height.max(0.0));
        let content = Size::new(side(content.width), side(content.height));
        let fit = (room.width / content.width).min(room.height / content.height);
        let upscale = if upscale.is_finite() { upscale.max(1.0) } else { 1.0 };
        let fit = if fit.is_finite() && fit > 0.0 { fit.min(upscale) } else { 1.0 };
        Self { room, content, fit, upscale }
    }

    /// The smallest scale: the fit, or the picture's own size when that is
    /// smaller, so a small picture shown enlarged can come back to it.
    pub fn min_scale(&self) -> f32 {
        self.fit.min(1.0)
    }

    pub fn max_scale(&self) -> f32 {
        ZOOM_VIEW_MAX_SCALE.max(self.fit)
    }

    /// The whole picture, centered.
    pub fn fitted(&self) -> View {
        self.centered(self.fit)
    }

    fn centered(&self, scale: f32) -> View {
        View {
            scale,
            x: (self.room.width - self.content.width * scale) / 2.0,
            y: (self.room.height - self.content.height * scale) / 2.0,
        }
    }

    /// `view` brought into bounds: its scale between the least and the most,
    /// a side smaller than the box centered on it, and a larger one covering
    /// it, so the picture is never lost off the box.
    pub fn clamp(&self, view: View) -> View {
        let scale = if view.scale.is_finite() {
            view.scale.clamp(self.min_scale(), self.max_scale())
        } else {
            self.fit
        };
        let axis = |offset: f32, room: f32, content: f32| {
            let shown = content * scale;
            if shown <= room {
                (room - shown) / 2.0
            } else if offset.is_finite() {
                offset.clamp(room - shown, 0.0)
            } else {
                (room - shown) / 2.0
            }
        };
        View {
            scale,
            x: axis(view.x, self.room.width, self.content.width),
            y: axis(view.y, self.room.height, self.content.height),
        }
    }

    /// `view` scaled by `factor` about `anchor`, a point of the box: the
    /// pixel under it stays under it, unless that would leave the picture
    /// short of an edge. A scale within a quarter of a step of the fit
    /// rests on it, as Loupe's does.
    pub fn zoom_at(&self, view: View, anchor: Point, factor: f32) -> View {
        if !factor.is_finite() || factor <= 0.0 {
            return view;
        }
        let scale = (view.scale * factor).clamp(self.min_scale(), self.max_scale());
        if (scale / self.fit).ln().abs() < STEP.ln() / 4.0 && (scale - view.scale).abs() > f32::EPSILON {
            return self.fitted();
        }
        self.scaled_at(view, anchor, scale)
    }

    /// `view` at `scale` about `anchor`, in bounds.
    pub fn scaled_at(&self, view: View, anchor: Point, scale: f32) -> View {
        let ratio = scale / view.scale;
        self.clamp(View {
            scale,
            x: anchor.x - (anchor.x - view.x) * ratio,
            y: anchor.y - (anchor.y - view.y) * ratio,
        })
    }

    /// `view` moved by `dx`, `dy`, in bounds.
    pub fn pan(&self, view: View, dx: f32, dy: f32) -> View {
        self.clamp(View { x: view.x + dx, y: view.y + dy, ..view })
    }

    /// What a double click does at `anchor`: fitted, two and a half times
    /// the fit there; otherwise back to the fit.
    pub fn toggled(&self, view: View, anchor: Point) -> View {
        if (view.scale - self.fit).abs() > 0.001 {
            return self.fitted();
        }
        self.scaled_at(view, anchor, (self.fit * TOGGLE).min(self.max_scale()))
    }

    /// The middle of the box, where a control or a key zooms about.
    pub fn center(&self) -> Point {
        Point::new(self.room.width / 2.0, self.room.height / 2.0)
    }
}

/// What the tree keeps of one zoom view between frames.
#[derive(Clone, Debug)]
pub(crate) struct ZoomState {
    /// Where the picture rests, or is moving to.
    pub view: View,
    from: View,
    started: Instant,
    duration: Duration,
    easing: Easing,
    /// Its fit follows the box until the person zooms or pans.
    pub fitted: bool,
    pub bounds: Bounds,
    /// When the scale was last changed, which shows it for a moment.
    pub badge_from: Option<Instant>,
    /// Laid out in the last layout; one that is not is forgotten.
    pub seen: bool,
    /// Quarter turns clockwise the picture is shown at.
    pub turns: u8,
    /// Its picture can turn: a raster picture, not a surface.
    pub turnable: bool,
}

impl ZoomState {
    pub fn new(bounds: Bounds, now: Instant) -> Self {
        let view = bounds.fitted();
        Self {
            view,
            from: view,
            started: now,
            duration: Duration::ZERO,
            easing: Easing::OutCubic,
            fitted: true,
            bounds,
            badge_from: None,
            seen: true,
            turns: 0,
            turnable: false,
        }
    }

    /// Turn the picture a quarter clockwise, or back, and fit it again, as
    /// its proportions change. Returns whether it turned.
    pub fn rotate(&mut self, clockwise: bool, now: Instant) -> bool {
        if !self.turnable {
            return false;
        }
        self.turns = (self.turns + if clockwise { 1 } else { 3 }) % 4;
        let old = self.bounds;
        self.bounds = Bounds::new(old.room, Size::new(old.content.height, old.content.width), old.upscale);
        self.fitted = true;
        self.jump(self.bounds.fitted(), now);
        self.badge_from = Some(now);
        true
    }

    /// Where the picture is shown at `now`, on its way to `view`.
    pub fn shown(&self, now: Instant) -> View {
        if self.duration.is_zero() {
            return self.view;
        }
        let progress = now.saturating_duration_since(self.started).as_secs_f32() / self.duration.as_secs_f32();
        if progress >= 1.0 {
            self.view
        } else {
            self.from.lerp(self.view, self.easing.sample(progress))
        }
    }

    pub fn is_moving(&self, now: Instant) -> bool {
        !self.duration.is_zero() && now.saturating_duration_since(self.started) < self.duration
    }

    /// How soon it needs another frame: at once while it moves or its badge
    /// fades, when the badge starts to fade while it shows, or never.
    pub fn repaint_after(&self, now: Instant) -> Option<Duration> {
        if self.is_moving(now) {
            return Some(Duration::ZERO);
        }
        let from = self.badge_from?;
        let age = now.saturating_duration_since(from);
        let hold = Duration::from_millis(BADGE_MS);
        if age < hold {
            Some(hold - age)
        } else if age < hold + Duration::from_millis(BADGE_FADE_MS) {
            Some(Duration::ZERO)
        } else {
            None
        }
    }

    /// Stop where it is shown, as a press does to a glide.
    pub fn hold(&mut self, now: Instant) {
        if self.is_moving(now) {
            let shown = self.bounds.clamp(self.shown(now));
            self.jump(shown, now);
        }
    }

    /// Opacity of the scale's badge at `now`, 0 once it has faded.
    pub fn badge(&self, now: Instant) -> f32 {
        let Some(from) = self.badge_from else {
            return 0.0;
        };
        let age = now.saturating_duration_since(from).as_millis() as u64;
        if age < BADGE_MS {
            1.0
        } else if age < BADGE_MS + BADGE_FADE_MS {
            1.0 - (age - BADGE_MS) as f32 / BADGE_FADE_MS as f32
        } else {
            0.0
        }
    }

    /// The box or the picture changed size: a fitted picture fits the new
    /// box; another keeps the pixel at the middle of the box there.
    pub fn resize(&mut self, bounds: Bounds, now: Instant) {
        if bounds == self.bounds {
            return;
        }
        let old = self.bounds;
        self.bounds = bounds;
        if self.fitted || old.content != bounds.content {
            self.fitted = true;
            self.jump(bounds.fitted(), now);
            return;
        }
        let view = self.view;
        let moved = View {
            x: view.x + (bounds.room.width - old.room.width) / 2.0,
            y: view.y + (bounds.room.height - old.room.height) / 2.0,
            ..view
        };
        self.jump(bounds.clamp(moved), now);
    }

    /// Rest on `view` at once.
    pub fn jump(&mut self, view: View, now: Instant) {
        self.view = view;
        self.from = view;
        self.started = now;
        self.duration = Duration::ZERO;
    }

    /// Move to `view` over `ms`, from where the picture is shown now; at once
    /// under reduced motion or with `ms` zero.
    pub fn move_to(&mut self, view: View, now: Instant, ms: u64, easing: Easing, reduced_motion: bool) {
        if reduced_motion || ms == 0 {
            self.jump(view, now);
            return;
        }
        self.from = self.shown(now);
        self.view = view;
        self.started = now;
        self.duration = Duration::from_millis(ms);
        self.easing = easing;
    }

    /// Change the scale: it shows for a moment and the view stops following
    /// the fit unless it is back at it.
    fn rescale(&mut self, view: View, now: Instant, ms: u64, reduced_motion: bool) -> bool {
        if view == self.view {
            return false;
        }
        let scaled = (view.scale - self.view.scale).abs() > f32::EPSILON;
        self.fitted = (view.scale - self.bounds.fit).abs() < 0.001 && view == self.bounds.fitted();
        self.move_to(view, now, ms, Easing::OutCubic, reduced_motion);
        if scaled {
            self.badge_from = Some(now);
        }
        scaled
    }

    /// Apply `command`. Returns whether the scale changed.
    pub fn command(&mut self, command: ZoomControl, now: Instant, reduced_motion: bool) -> bool {
        let bounds = self.bounds;
        let center = bounds.center();
        let view = match command {
            ZoomControl::In => bounds.zoom_at(self.view, center, STEP),
            ZoomControl::Out => bounds.zoom_at(self.view, center, 1.0 / STEP),
            ZoomControl::Fit => bounds.fitted(),
            ZoomControl::Actual => bounds.scaled_at(self.view, center, 1.0f32.clamp(bounds.min_scale(), bounds.max_scale())),
            ZoomControl::Toggle => bounds.toggled(self.view, center),
            ZoomControl::RotateClockwise | ZoomControl::RotateCounterclockwise => {
                return self.rotate(command == ZoomControl::RotateClockwise, now);
            }
        };
        self.rescale(view, now, ZOOM_MS, reduced_motion)
    }

    /// Zoom by `factor` about `anchor`, a point of the box. A pinch passes
    /// `smooth` false: it follows the fingers at once.
    pub fn zoom_at(&mut self, anchor: Point, factor: f32, now: Instant, smooth: bool, reduced_motion: bool) -> bool {
        let view = self.bounds.zoom_at(self.view, anchor, factor);
        self.rescale(view, now, if smooth { WHEEL_MS } else { 0 }, reduced_motion)
    }

    /// The zoom of a mouse wheel's `delta` logical pixels of travel,
    /// positive toward the top of the content: up zooms in.
    pub fn wheel_factor(delta: f32) -> f32 {
        (delta / NOTCH * WHEEL_NOTCH.ln()).exp()
    }

    /// The zoom of a touchpad's `delta` logical pixels of scroll with Ctrl.
    pub fn pixel_factor(delta: f32) -> f32 {
        (delta * WHEEL_PIXEL.ln()).exp()
    }

    /// Rest at `scale` about the middle of the box, as `2` and `3` ask.
    pub fn scale_to(&mut self, scale: f32, now: Instant, reduced_motion: bool) -> bool {
        let bounds = self.bounds;
        let view = bounds.scaled_at(self.view, bounds.center(), scale.clamp(bounds.min_scale(), bounds.max_scale()));
        self.rescale(view, now, ZOOM_MS, reduced_motion)
    }

    /// A double click at `anchor`.
    pub fn toggle_at(&mut self, anchor: Point, now: Instant, reduced_motion: bool) -> bool {
        let view = self.bounds.toggled(self.view, anchor);
        self.rescale(view, now, ZOOM_MS, reduced_motion)
    }

    /// Move the picture by `dx`, `dy` at once, as a drag or a touchpad does.
    /// Returns whether it moved.
    pub fn pan(&mut self, dx: f32, dy: f32, now: Instant) -> bool {
        let view = self.bounds.pan(self.shown(now), dx, dy);
        if view == self.view && !self.is_moving(now) {
            return false;
        }
        self.fitted = false;
        self.jump(view, now);
        true
    }

    /// A drag let go at `velocity` logical pixels a second: the picture
    /// glides on and slows to a stop, never past an edge.
    pub fn fling(&mut self, velocity: Point, now: Instant, reduced_motion: bool) {
        let speed = velocity.x.hypot(velocity.y);
        if reduced_motion || !speed.is_finite() || speed < GLIDE_MIN_SPEED {
            return;
        }
        let view = self.bounds.pan(self.view, velocity.x * GLIDE_TAU, velocity.y * GLIDE_TAU);
        if view != self.view {
            self.move_to(view, now, (GLIDE_TAU * 3_000.0) as u64, Easing::OutCubic, false);
        }
    }

    /// Start, as `from`, on its way to its fit: the picture grows out of
    /// the thumbnail it was opened from.
    pub fn open_from(&mut self, from: View, now: Instant, reduced_motion: bool) {
        if !reduced_motion {
            self.from = from;
            self.started = now;
            self.duration = Duration::from_millis(u64::from(crate::tokens().motion.enter_ms) + 80);
            self.easing = Easing::OutCubic;
        }
    }
}

/// Where the picture of `content` pixels would rest at the size and place of
/// `rect`, a rectangle of the box, as large as fits inside it and centered.
pub(crate) fn view_in(rect: Rect, room: Rect, content: Size) -> View {
    let scale = (rect.width / content.width.max(1.0)).min(rect.height / content.height.max(1.0)).max(0.001);
    View {
        scale,
        x: rect.x - room.x + (rect.width - content.width * scale) / 2.0,
        y: rect.y - room.y + (rect.height - content.height * scale) / 2.0,
    }
}

/// A scale as a whole percentage, as the badge and the zoom control say it.
pub fn zoom_percent(scale: f32) -> u32 {
    if scale.is_finite() && scale > 0.0 {
        (scale * 100.0).round() as u32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: Size = Size::new(800.0, 600.0);

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn a_large_picture_fits_whole_and_centered() {
        let bounds = Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0);
        assert!(close(bounds.fit, 1.0 / 3.0));
        let view = bounds.fitted();
        // 800 by 533⅓: as wide as the box, centered down it.
        assert!(close(view.x, 0.0) && close(view.y, (600.0 - 1600.0 / 3.0) / 2.0));
        assert_eq!(bounds.min_scale(), bounds.fit);
    }

    #[test]
    fn a_small_picture_is_enlarged_only_as_far_as_allowed() {
        let small = Size::new(300.0, 340.0);
        assert_eq!(Bounds::new(ROOM, small, 1.0).fit, 1.0);
        let enlarged = Bounds::new(ROOM, small, 2.0);
        assert!(close(enlarged.fit, (600.0f32 / 340.0).min(2.0)));
        // It can come back down to its own size, and no further.
        assert_eq!(enlarged.min_scale(), 1.0);
        let view = enlarged.zoom_at(enlarged.fitted(), enlarged.center(), 0.1);
        assert_eq!(view.scale, 1.0);
        assert!(close(view.x, 250.0) && close(view.y, 130.0));
    }

    #[test]
    fn a_zoom_keeps_the_pixel_under_the_pointer_where_it_is() {
        let bounds = Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0);
        let view = bounds.fitted();
        let anchor = Point::new(300.0, 250.0);
        let pixel = |view: View| ((anchor.x - view.x) / view.scale, (anchor.y - view.y) / view.scale);
        let zoomed = bounds.zoom_at(view, anchor, 2.0);
        assert!(close(zoomed.scale, view.scale * 2.0));
        let (before, after) = (pixel(view), pixel(zoomed));
        assert!(close(before.0, after.0) && close(before.1, after.1), "{before:?} {after:?}");
    }

    #[test]
    fn the_picture_never_leaves_an_edge_of_the_box_bare() {
        let bounds = Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0);
        let zoomed = bounds.zoom_at(bounds.fitted(), bounds.center(), 3.0);
        // Dragged far right and down: its left and top edges stop at the box's.
        let view = bounds.pan(zoomed, 10_000.0, 10_000.0);
        assert_eq!((view.x, view.y), (0.0, 0.0));
        // And far left and up: its right and bottom edges stop at the box's.
        let view = bounds.pan(zoomed, -10_000.0, -10_000.0);
        assert!(close(view.x + 2400.0 * view.scale, 800.0) && close(view.y + 1600.0 * view.scale, 600.0));
        // A side that fits stays centered whatever the drag.
        let fitted = bounds.pan(bounds.fitted(), 0.0, 500.0);
        assert_eq!(fitted, bounds.fitted());
    }

    #[test]
    fn the_scale_stays_between_the_fit_and_the_most() {
        let bounds = Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0);
        let most = bounds.zoom_at(bounds.fitted(), bounds.center(), 1_000.0);
        assert_eq!(most.scale, ZOOM_VIEW_MAX_SCALE);
        let least = bounds.zoom_at(most, bounds.center(), 0.000_1);
        assert_eq!(least, bounds.fitted());
        assert_eq!(bounds.zoom_at(least, bounds.center(), f32::NAN), least);
        // Close to the fit, it rests on it.
        let near = bounds.zoom_at(bounds.fitted(), Point::new(10.0, 10.0), 1.05);
        assert_eq!(near, bounds.fitted());
        // A picture larger on screen than the most still fits.
        let tiny = Bounds::new(ROOM, Size::new(10.0, 10.0), 100.0);
        assert_eq!(tiny.max_scale(), 60.0);
    }

    #[test]
    fn a_double_click_zooms_in_two_and_a_half_times_and_back_to_the_fit() {
        let bounds = Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0);
        let anchor = Point::new(100.0, 300.0);
        let zoomed = bounds.toggled(bounds.fitted(), anchor);
        assert!(close(zoomed.scale, bounds.fit * 2.5));
        assert_eq!(bounds.toggled(zoomed, anchor), bounds.fitted());
        // A small picture at its own size goes to 250 %.
        let small = Bounds::new(ROOM, Size::new(300.0, 340.0), 1.0);
        assert_eq!(small.toggled(small.fitted(), anchor).scale, 2.5);
    }

    #[test]
    fn a_notch_of_the_wheel_zooms_by_a_third_and_the_keys_step_by_a_half() {
        assert!(close(ZoomState::wheel_factor(30.0), 1.3));
        assert!(close(ZoomState::wheel_factor(-30.0), 1.0 / 1.3));
        assert!(close(ZoomState::pixel_factor(10.0), 1.005f32.powi(10)));
        let now = Instant::now();
        let mut state = ZoomState::new(Bounds::new(ROOM, Size::new(300.0, 340.0), 1.0), now);
        assert!(state.scale_to(3.0, now, true));
        assert_eq!(state.view.scale, 3.0);
        assert!(state.command(ZoomControl::Out, now, true));
        assert!(close(state.view.scale, 2.0));
    }

    #[test]
    fn a_fitted_view_follows_the_box_and_a_zoomed_one_keeps_its_middle() {
        let now = Instant::now();
        let content = Size::new(2400.0, 1600.0);
        let mut state = ZoomState::new(Bounds::new(ROOM, content, 1.0), now);
        let wider = Bounds::new(Size::new(1200.0, 600.0), content, 1.0);
        state.resize(wider, now);
        assert_eq!(state.view, wider.fitted());
        assert!(state.command(ZoomControl::Actual, now, true));
        assert!(!state.fitted);
        let middle = |state: &ZoomState| {
            let center = state.bounds.center();
            ((center.x - state.view.x) / state.view.scale, (center.y - state.view.y) / state.view.scale)
        };
        let before = middle(&state);
        state.resize(Bounds::new(Size::new(1000.0, 500.0), content, 1.0), now);
        let after = middle(&state);
        assert!(close(before.0, after.0) && close(before.1, after.1));
    }

    #[test]
    fn commands_step_and_animate_unless_motion_is_reduced() {
        let now = Instant::now();
        let mut state = ZoomState::new(Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0), now);
        let fit = state.view.scale;
        assert!(state.command(ZoomControl::In, now, false));
        assert!(close(state.view.scale, fit * STEP));
        // Moving: shown between, there after.
        assert!(state.is_moving(now + Duration::from_millis(50)));
        let halfway = state.shown(now + Duration::from_millis(50)).scale;
        assert!(halfway > fit && halfway < state.view.scale);
        assert_eq!(state.shown(now + Duration::from_secs(1)), state.view);
        assert_eq!(state.badge(now), 1.0);
        // Reduced motion: at once.
        assert!(state.command(ZoomControl::Fit, now, true));
        assert!(!state.is_moving(now) && state.fitted);
        // Nothing to do: nothing changes.
        assert!(!state.command(ZoomControl::Fit, now, true));
        assert!(!state.command(ZoomControl::Out, now, true));
    }

    #[test]
    fn a_fling_glides_on_within_the_edges_and_a_slow_release_stops() {
        let now = Instant::now();
        let mut state = ZoomState::new(Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0), now);
        state.command(ZoomControl::Actual, now, true);
        let rest = state.view;
        state.fling(Point::new(50.0, 0.0), now, false);
        assert_eq!(state.view, rest);
        state.fling(Point::new(-1_000.0, 0.0), now, false);
        assert!(close(state.view.x, rest.x - 250.0) && state.is_moving(now));
        state.jump(rest, now);
        state.fling(Point::new(-1_000.0, 0.0), now, true);
        assert_eq!(state.view, rest, "no glide under reduced motion");
        state.fling(Point::new(1.0e6, 0.0), now, false);
        assert_eq!(state.view.x, 0.0);
    }

    #[test]
    fn a_turn_swaps_the_sides_and_fits_again_but_only_a_turnable_picture() {
        let now = Instant::now();
        let mut state = ZoomState::new(Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0), now);
        assert!(!state.rotate(true, now), "a surface does not turn");
        state.turnable = true;
        state.command(ZoomControl::In, now, true);
        assert!(state.command(ZoomControl::RotateClockwise, now, true));
        assert_eq!((state.turns, state.bounds.content), (1, Size::new(1600.0, 2400.0)));
        assert!(state.fitted && close(state.view.scale, 600.0 / 2400.0));
        state.command(ZoomControl::RotateCounterclockwise, now, true);
        state.command(ZoomControl::RotateCounterclockwise, now, true);
        assert_eq!(state.turns, 3);
    }

    #[test]
    fn opening_from_a_thumbnail_starts_there() {
        let now = Instant::now();
        let room = Rect::new(10.0, 20.0, 800.0, 600.0);
        let content = Size::new(2400.0, 1600.0);
        let thumb = Rect::new(100.0, 100.0, 120.0, 80.0);
        let from = view_in(thumb, room, content);
        assert!(close(from.rect(content, room).x, thumb.x) && close(from.rect(content, room).width, 120.0));
        let mut state = ZoomState::new(Bounds::new(room_size(room), content, 1.0), now);
        state.open_from(from, now, false);
        assert_eq!(state.shown(now), from);
        assert_eq!(state.shown(now + Duration::from_secs(1)), state.view);
        let mut still = ZoomState::new(Bounds::new(room_size(room), content, 1.0), now);
        still.open_from(from, now, true);
        assert_eq!(still.shown(now), still.view);
    }

    fn room_size(room: Rect) -> Size {
        Size::new(room.width, room.height)
    }

    #[test]
    fn the_badge_says_a_percentage_and_fades() {
        assert_eq!((zoom_percent(1.0), zoom_percent(0.3333), zoom_percent(f32::NAN)), (100, 33, 0));
        let now = Instant::now();
        let mut state = ZoomState::new(Bounds::new(ROOM, Size::new(2400.0, 1600.0), 1.0), now);
        assert_eq!(state.badge(now), 0.0);
        state.command(ZoomControl::In, now, true);
        assert_eq!(state.badge(now + Duration::from_millis(BADGE_MS - 1)), 1.0);
        assert!(state.badge(now + Duration::from_millis(BADGE_MS + BADGE_FADE_MS / 2)) < 1.0);
        assert_eq!(state.badge(now + Duration::from_millis(BADGE_MS + BADGE_FADE_MS)), 0.0);
    }
}
