//! Pointer tracking for a host-controlled downward dismiss gesture.

use std::collections::VecDeque;

use crate::Point;
use web_time::{Duration, Instant};

const CLAIM_DISTANCE: f32 = 8.0;
const VELOCITY_WINDOW: Duration = Duration::from_millis(100);
const FLICK_TRAVEL: f32 = 24.0;

/// Tracks a downward drag without owning pointer routing or animation.
///
/// The host decides eligibility at pointer down, cancels on pointer cancellation
/// or focus loss, and uses the offset to render and animate its content.
#[derive(Debug)]
pub struct DismissDrag {
    distance: f32,
    velocity: f32,
    origin: Option<Point>,
    dragging: bool,
    offset: f32,
    samples: VecDeque<(Instant, f32)>,
}

impl DismissDrag {
    /// Dismiss on release after `distance` logical pixels or `velocity` logical
    /// pixels per second. Negative or nonfinite thresholds disable capture.
    pub fn new(distance: f32, velocity: f32) -> Self {
        Self {
            distance,
            velocity,
            origin: None,
            dragging: false,
            offset: 0.0,
            samples: VecDeque::new(),
        }
    }

    /// Starts a new pointer sequence, replacing any previous gesture.
    /// `eligible` is fixed for this sequence; an ineligible sequence stays idle.
    pub fn begin(&mut self, position: Point, now: Instant, eligible: bool) {
        self.cancel();
        if eligible
            && position.x.is_finite()
            && position.y.is_finite()
            && self.distance.is_finite()
            && self.distance >= 0.0
            && self.velocity.is_finite()
            && self.velocity >= 0.0
        {
            self.origin = Some(position);
            self.samples.push_back((now, 0.0));
        }
    }

    /// Returns whether this gesture owns the pointer, including later updates.
    /// Capture needs at least 8 px downward with more vertical than horizontal
    /// movement. Horizontal or upward movement outside that slop cancels before
    /// capture. Invalid coordinates or a backwards timestamp cancel at any time.
    pub fn update(&mut self, position: Point, now: Instant) -> bool {
        let Some(origin) = self.origin else {
            return false;
        };
        let dx = position.x - origin.x;
        let dy = position.y - origin.y;
        if !position.x.is_finite()
            || !position.y.is_finite()
            || !dx.is_finite()
            || !dy.is_finite()
            || self.samples.back().is_some_and(|(time, _)| now < *time)
        {
            self.cancel();
            return false;
        }
        if !self.dragging {
            if dx.abs().max(dy.abs()) < CLAIM_DISTANCE {
                self.record(now, dy.max(0.0));
                return false;
            }
            if dy < CLAIM_DISTANCE || dx.abs() >= dy {
                self.cancel();
                return false;
            }
            self.dragging = true;
        }
        self.offset = dy.max(0.0);
        self.record(now, self.offset);
        true
    }

    /// Returns whether to dismiss, then clears the gesture and its offset.
    /// A flick needs at least 24 px of net downward travel in the last 100 ms;
    /// stale movement and reversals do not count. Read [`Self::offset`]
    /// before release if the host needs the starting point for an animation.
    pub fn release(&mut self, now: Instant) -> bool {
        let time_valid = self.samples.back().is_some_and(|(time, _)| now >= *time);
        let dismiss =
            self.dragging && time_valid && (self.offset >= self.distance || self.is_flick(now));
        self.cancel();
        dismiss
    }

    /// Clears the pointer sequence, offset, and velocity history.
    pub fn cancel(&mut self) {
        self.origin = None;
        self.dragging = false;
        self.offset = 0.0;
        self.samples.clear();
    }

    /// Downward displacement in logical pixels, or zero before capture.
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Downward speed over the last 100 ms in logical pixels per second, or
    /// zero before capture. Read it before [`Self::release`] to hand the
    /// gesture to a spring with [`crate::ScalarSpring::release`], so the
    /// content keeps the finger's speed instead of restarting from rest.
    pub fn velocity(&self, now: Instant) -> f32 {
        let window = |time: &Instant| now.saturating_duration_since(*time) <= VELOCITY_WINDOW;
        let (Some(&(since, start)), Some(&(_, end))) =
            (self.samples.iter().find(|(time, _)| window(time)), self.samples.back())
        else {
            return 0.0;
        };
        let seconds = now.saturating_duration_since(since).as_secs_f32();
        if !self.dragging || seconds <= 0.0 {
            return 0.0;
        }
        (end - start) / seconds
    }

    /// Whether the current pointer sequence has been captured.
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    fn record(&mut self, now: Instant, offset: f32) {
        if let Some(&(time, previous)) = self.samples.back() {
            if offset < previous {
                self.samples.clear();
            } else if now == time {
                self.samples.pop_back();
            }
        }
        self.discard_stale(now);
        self.samples.push_back((now, offset));
    }

    fn discard_stale(&mut self, now: Instant) {
        while self
            .samples
            .front()
            .is_some_and(|(time, _)| now.duration_since(*time) > VELOCITY_WINDOW)
        {
            self.samples.pop_front();
        }
    }

    fn is_flick(&mut self, now: Instant) -> bool {
        self.discard_stale(now);
        let Some(&(time, start)) = self.samples.front() else {
            return false;
        };
        let travel = self.samples.back().unwrap().1 - start;
        let seconds = now.duration_since(time).as_secs_f64();
        travel >= FLICK_TRAVEL
            && seconds > 0.0
            && f64::from(travel) / seconds >= f64::from(self.velocity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use web_time::Duration;

    #[test]
    fn velocity_is_the_recent_downward_speed_for_a_spring_handoff() {
        let now = Instant::now();
        let at = |ms| now + Duration::from_millis(ms);
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert_eq!(drag.velocity(now), 0.0);
        assert!(drag.update(Point::new(0.0, 10.0), at(10)));
        for step in 2..=10 {
            drag.update(Point::new(0.0, 10.0 * step as f32), at(10 * step));
        }
        // 10 px every 10 ms over the last 100 ms.
        assert!((drag.velocity(at(100)) - 1000.0).abs() < 1.0, "{}", drag.velocity(at(100)));
        // A finger held still slows it, and the release clears it.
        assert!(drag.velocity(at(150)) < 1000.0);
        assert_eq!(drag.velocity(at(400)), 0.0);
        drag.release(at(400));
        assert_eq!(drag.velocity(at(400)), 0.0);
    }

    #[test]
    fn captures_only_after_eight_pixels_downward() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::new(10.0, 20.0), now, true);
        assert!(!drag.update(Point::new(10.0, 27.9), now + Duration::from_millis(10)));
        assert_eq!(drag.offset(), 0.0);
        assert!(!drag.is_dragging());
        assert!(drag.update(Point::new(10.0, 28.0), now + Duration::from_millis(20)));
        assert_eq!(drag.offset(), 8.0);
        assert!(drag.is_dragging());
    }

    #[test]
    fn horizontal_and_upward_gestures_cannot_later_capture() {
        let now = Instant::now();
        for position in [
            Point::new(8.0, 7.0),
            Point::new(-8.0, 0.0),
            Point::new(9.0, 9.0),
            Point::new(0.0, -8.0),
        ] {
            let mut drag = DismissDrag::new(80.0, 800.0);
            drag.begin(Point::default(), now, true);
            assert!(!drag.update(position, now + Duration::from_millis(10)));
            assert!(!drag.update(Point::new(0.0, 100.0), now + Duration::from_millis(20)));
            assert!(!drag.release(now + Duration::from_millis(20)));
            assert_eq!(drag.offset(), 0.0);
        }
    }

    #[test]
    fn captured_gestures_keep_ownership_and_never_have_negative_offset() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 12.0), now + Duration::from_millis(10)));
        assert!(drag.update(Point::new(100.0, 9.0), now + Duration::from_millis(20)));
        assert_eq!(drag.offset(), 9.0);
        assert!(drag.update(Point::new(100.0, -10.0), now + Duration::from_millis(30)));
        assert_eq!(drag.offset(), 0.0);
        assert!(drag.is_dragging());
        assert!(!drag.release(now + Duration::from_millis(30)));
    }

    #[test]
    fn release_uses_the_distance_threshold_inclusively_and_resets() {
        let now = Instant::now();
        for (distance, expected) in [(79.0, false), (80.0, true)] {
            let mut drag = DismissDrag::new(80.0, 800.0);
            drag.begin(Point::default(), now, true);
            assert!(drag.update(Point::new(0.0, distance), now + Duration::from_secs(1)));
            assert_eq!(drag.release(now + Duration::from_secs(1)), expected);
            assert_eq!(drag.offset(), 0.0);
            assert!(!drag.is_dragging());
            assert!(!drag.release(now + Duration::from_secs(1)));
        }
    }

    #[test]
    fn a_recent_downward_flick_can_dismiss_below_the_distance_threshold() {
        let now = Instant::now();
        for (distance, expected) in [(39.0, false), (40.0, true)] {
            let mut drag = DismissDrag::new(80.0, 800.0);
            drag.begin(Point::default(), now, true);
            assert!(drag.update(Point::new(0.0, distance), now + Duration::from_millis(50)));
            assert_eq!(drag.release(now + Duration::from_millis(50)), expected);
        }
    }

    #[test]
    fn velocity_requires_meaningful_recent_travel() {
        let now = Instant::now();
        for (distance, expected) in [(23.0, false), (24.0, true)] {
            let mut drag = DismissDrag::new(80.0, 800.0);
            drag.begin(Point::default(), now, true);
            assert!(drag.update(Point::new(0.0, distance), now + Duration::from_millis(1)));
            assert_eq!(drag.release(now + Duration::from_millis(1)), expected);
        }
    }

    #[test]
    fn stale_samples_do_not_turn_a_slow_drag_into_a_flick() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_secs(1)));
        assert!(!drag.release(now + Duration::from_secs(1)));

        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(10)));
        assert!(!drag.release(now + Duration::from_millis(111)));
    }

    #[test]
    fn a_recent_flick_works_after_a_slow_start() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 20.0), now + Duration::from_secs(1)));
        assert!(drag.update(Point::new(0.0, 50.0), now + Duration::from_millis(1020)));
        assert!(drag.release(now + Duration::from_millis(1020)));
    }

    #[test]
    fn release_delay_reduces_velocity() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(50)));
        assert!(!drag.release(now + Duration::from_millis(100)));
    }

    #[test]
    fn recent_jitter_and_reversals_do_not_reuse_an_earlier_flick() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(10)));
        assert!(drag.update(Point::new(0.0, 40.1), now + Duration::from_millis(120)));
        assert!(!drag.release(now + Duration::from_millis(120)));

        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(10)));
        assert!(drag.update(Point::new(0.0, 35.0), now + Duration::from_millis(11)));
        assert!(!drag.release(now + Duration::from_millis(11)));
    }

    #[test]
    fn equal_timestamps_do_not_create_infinite_velocity() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now));
        assert!(!drag.release(now + Duration::from_millis(1)));
    }

    #[test]
    fn eligibility_is_decided_at_begin() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, false);
        assert!(!drag.update(Point::new(0.0, 100.0), now + Duration::from_millis(10)));
        assert!(!drag.release(now + Duration::from_millis(10)));
        assert_eq!(drag.offset(), 0.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 100.0), now + Duration::from_millis(10)));
        assert!(drag.release(now + Duration::from_millis(10)));
    }

    #[test]
    fn zero_velocity_does_not_dismiss_a_stationary_or_short_gesture() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 0.0);
        drag.begin(Point::default(), now, true);
        assert!(!drag.update(Point::default(), now + Duration::from_millis(10)));
        assert!(!drag.release(now + Duration::from_millis(10)));
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 8.0), now + Duration::from_millis(10)));
        assert!(!drag.release(now + Duration::from_millis(10)));
    }

    #[test]
    fn cancel_and_restart_clear_the_previous_gesture() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(10)));
        drag.cancel();
        assert_eq!(drag.offset(), 0.0);
        assert!(!drag.is_dragging());
        assert!(!drag.update(Point::new(0.0, 100.0), now + Duration::from_millis(20)));
        assert!(!drag.release(now + Duration::from_millis(20)));
        drag.begin(
            Point::new(0.0, 1000.0),
            now + Duration::from_millis(20),
            true,
        );
        assert!(drag.update(Point::new(0.0, 1030.0), now + Duration::from_secs(1)));
        assert!(!drag.release(now + Duration::from_secs(1)));
    }

    #[test]
    fn beginning_again_resets_an_active_drag() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::default(), now, true);
        assert!(drag.update(Point::new(0.0, 40.0), now + Duration::from_millis(10)));
        drag.begin(
            Point::new(0.0, 100.0),
            now + Duration::from_millis(20),
            true,
        );
        assert_eq!(drag.offset(), 0.0);
        assert!(!drag.is_dragging());
        assert!(!drag.update(Point::new(0.0, 107.0), now + Duration::from_millis(30)));
    }

    #[test]
    fn nonfinite_positions_and_thresholds_cannot_capture() {
        let now = Instant::now();
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            for position in [Point::new(invalid, 0.0), Point::new(0.0, invalid)] {
                let mut drag = DismissDrag::new(80.0, 800.0);
                drag.begin(position, now, true);
                assert!(!drag.update(Point::new(0.0, 100.0), now));
                assert_eq!(drag.offset(), 0.0);

                drag.begin(Point::default(), now, true);
                assert!(drag.update(Point::new(0.0, 40.0), now));
                assert!(!drag.update(position, now));
                assert_eq!(drag.offset(), 0.0);
                assert!(!drag.is_dragging());
                assert!(!drag.release(now));
            }
            for (distance, velocity) in [(invalid, 800.0), (80.0, invalid)] {
                let mut drag = DismissDrag::new(distance, velocity);
                drag.begin(Point::default(), now, true);
                assert!(!drag.update(Point::new(0.0, 100.0), now));
                assert_eq!(drag.offset(), 0.0);
                assert!(!drag.release(now));
            }
        }
    }

    #[test]
    fn overflowing_displacement_cancels_without_a_nonfinite_offset() {
        let now = Instant::now();
        let mut drag = DismissDrag::new(80.0, 800.0);
        drag.begin(Point::new(0.0, -f32::MAX), now, true);
        assert!(!drag.update(Point::new(0.0, f32::MAX), now));
        assert_eq!(drag.offset(), 0.0);
        assert!(!drag.is_dragging());
    }
}
