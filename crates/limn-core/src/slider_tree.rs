use super::*;

const SLIDER_ULTRA_FADE_DURATION: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct SliderGeometry {
    pub(super) rect: Rect,
    pub(super) steps: usize,
    pub(super) fraction: f32,
    pub(super) ultra_available: bool,
    /// Distance from each end of `rect` to the ends of the thumb's travel.
    pub(super) inset: f32,
    /// The effort slider settles on a spring; a plain slider follows the host.
    pub(super) animated: bool,
    /// The share of the height from the top, for a slider over an area.
    pub(super) y: Option<f32>,
    /// A slider standing up: its fraction runs from the bottom to the top.
    pub(super) vertical: bool,
}

impl SliderGeometry {
    /// The pointer's share of the height from the top, for an area slider.
    pub(super) fn y_at(self, y: f32) -> Option<f32> {
        let travel = self.rect.height - self.inset * 2.0;
        self.y.map(|_| {
            if travel <= 0.0 {
                0.5
            } else {
                ((y - self.rect.y - self.inset) / travel).clamp(0.0, 1.0)
            }
        })
    }

    pub(super) fn fraction_at(self, point: Point) -> f32 {
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

    pub(super) fn snap(self, fraction: f32) -> f32 {
        if self.steps < 2 {
            return fraction.clamp(0.0, 1.0);
        }
        let intervals = self.steps.saturating_sub(1).max(1) as f32;
        (fraction.clamp(0.0, 1.0) * intervals).round() / intervals
    }
}

pub(super) struct SliderDrag {
    pub(super) id: String,
    pub(super) geometry: SliderGeometry,
    pub(super) fraction: f32,
    pub(super) y: Option<f32>,
}

pub(super) struct SliderMotion {
    phase: f32,
    pub(super) last_sample: Instant,
    pub(super) displayed_fraction: f32,
    target_fraction: Option<f32>,
    pub(super) snap: Option<SliderSnap>,
    pub(super) burst_started: Option<Instant>,
    ultra_from: f32,
    ultra_to: f32,
    ultra_started: Instant,
}

#[derive(Clone, Copy)]
pub(super) struct SliderSnap {
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
    pub(super) fn new(now: Instant) -> Self {
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

    pub(super) fn animate_to(&mut self, to: f32, now: Instant, celebrate_ultra: bool) {
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

    pub(super) fn sample(
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

    pub(super) fn is_fast(&self, now: Instant) -> bool {
        self.snap.is_some_and(|snap| {
            now.saturating_duration_since(snap.started) < SLIDER_SETTLE_DURATION
        }) || self
            .burst_started
            .is_some_and(|start| now.saturating_duration_since(start) < SLIDER_BURST_DURATION)
            || (self.ultra_from != self.ultra_to
                && now.saturating_duration_since(self.ultra_started) < SLIDER_ULTRA_FADE_DURATION)
    }
}
