//! Small, backend-independent transitions for interaction colors and geometry.
//!
//! A transition belongs to a stable widget ID, not to a transient element.

use web_time::{Duration, Instant};

use crate::Color;

/// Durations for a menu's floating surface and item feedback. UiTree keeps
/// both transitions by stable ID and snaps them under reduced motion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuMotion {
    /// How long the menu's surface takes to open, in milliseconds. The default is
    /// the `enter_ms` token.
    pub open_ms: u16,
    /// How long an item's hover and press feedback takes, in milliseconds. The
    /// default is the `hover_ms` token.
    pub item_ms: u16,
}

impl Default for MenuMotion {
    fn default() -> Self {
        Self {
            open_ms: crate::tokens().motion.enter_ms,
            item_ms: crate::tokens().motion.hover_ms,
        }
    }
}

/// Wheel and scrollbar-button motion in logical pixels. Retained scroll
/// surfaces share this policy; thumb dragging and precise pointer scrolling
/// remain direct so the surface stays attached to the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMotion {
    /// Zero disables interpolation. The host also snaps under reduced motion.
    pub duration_ms: u16,
    /// The curve the offset follows over `duration_ms`. `Easing::OutCubic` by default.
    pub easing: Easing,
}

impl Default for ScrollMotion {
    fn default() -> Self {
        Self {
            duration_ms: 140,
            easing: Easing::OutCubic,
        }
    }
}

/// Timing curve shared by layout and component transitions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Easing {
    /// A responsive start and a soft finish for short interface movements.
    #[default]
    OutCubic,
    /// A gentle start and finish for larger panel movements.
    InOutCubic,
    /// A CSS `cubic-bezier(x1, y1, x2, y2)` curve from (0, 0) to (1, 1). The x
    /// values are clamped to 0..=1, as CSS requires, and y may overshoot.
    CubicBezier(f32, f32, f32, f32),
    /// A softer start and finish than `InOutCubic`, within two hundredths of
    /// CSS `ease-in-out` all the way.
    InOutSine,
    /// The same speed all the way.
    Linear,
    /// An under-damped spring set off from rest, as a hint grows in: it
    /// reaches the end at a quarter of the time, passes it by 14 % of the
    /// way at 0.37, comes back 2 % short at 0.74 and rests on it at 1. Its
    /// share goes above 1 on the way, so a value it drives overshoots.
    Spring,
}

impl Easing {
    /// The eased share of the way, from 0 to 1, for a linear `progress`;
    /// `Spring` passes 1 on its way there. `progress` is clamped to 0..=1.
    pub fn sample(self, progress: f32) -> f32 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::OutCubic => 1.0 - (1.0 - progress).powi(3),
            Self::InOutCubic if progress < 0.5 => 4.0 * progress.powi(3),
            Self::InOutCubic => 1.0 - (-2.0 * progress + 2.0).powi(3) / 2.0,
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier(x1.clamp(0.0, 1.0), y1, x2.clamp(0.0, 1.0), y2, progress),
            Self::InOutSine => (1.0 - (std::f32::consts::PI * progress).cos()) / 2.0,
            Self::Linear => progress,
            Self::Spring if progress == 1.0 => 1.0,
            Self::Spring => {
                // Damping ratio 0.535 turned 10 radians by the end, within
                // 0.02 % of rest there: a hint's measured spring over 200 ms.
                const ZETA: f32 = 0.535;
                const TURN: f32 = 10.0;
                let damped = TURN * (1.0 - ZETA * ZETA).sqrt();
                let (sin, cos) = (damped * progress).sin_cos();
                1.0 - (-ZETA * TURN * progress).exp() * (cos + ZETA * TURN / damped * sin)
            }
        }
    }
}

/// The y of a cubic Bézier at the parameter `s` whose x is `t`. Newton's method
/// finds `s` in a few steps; where it stalls, bisection on the monotone x does.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 || t >= 1.0 {
        return t.clamp(0.0, 1.0);
    }
    let coordinate = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let slope = |a: f32, b: f32, s: f32| {
        let u = 1.0 - s;
        3.0 * u * u * a + 6.0 * u * s * (b - a) + 3.0 * s * s * (1.0 - b)
    };
    let mut s = t;
    for _ in 0..8 {
        let error = coordinate(x1, x2, s) - t;
        let derivative = slope(x1, x2, s);
        if error.abs() < 1e-6 || derivative.abs() < 1e-6 {
            break;
        }
        s = (s - error / derivative).clamp(0.0, 1.0);
    }
    if (coordinate(x1, x2, s) - t).abs() > 1e-5 {
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        s = t;
        for _ in 0..40 {
            if coordinate(x1, x2, s) < t {
                low = s;
            } else {
                high = s;
            }
            s = (low + high) / 2.0;
        }
    }
    coordinate(y1, y2, s)
}

/// How an icon is turned and stretched about the middle of the rectangle it
/// is drawn in, as [`DrawCommand::Icon`](crate::DrawCommand::Icon) carries it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconTransform {
    /// Clockwise turn in radians.
    pub angle: f32,
    /// Stretch across: 1 is the icon's own width.
    pub scale_x: f32,
    /// Stretch down: 1 is the icon's own height.
    pub scale_y: f32,
}

impl IconTransform {
    /// No turn and no stretch: the icon as it is drawn at rest.
    pub const IDENTITY: Self = Self { angle: 0.0, scale_x: 1.0, scale_y: 1.0 };
}

impl Default for IconTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Where an icon is in an [`IconMotion`]: a clockwise turn in degrees, a
/// stretch across and down, and an offset in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct IconPose {
    pub(crate) turn: f32,
    pub(crate) scale_x: f32,
    pub(crate) scale_y: f32,
    pub(crate) offset: crate::Point,
}

impl IconPose {
    pub(crate) const REST: Self = Self { turn: 0.0, scale_x: 1.0, scale_y: 1.0, offset: crate::Point { x: 0.0, y: 0.0 } };

    fn toward(self, to: Self, share: f32) -> Self {
        let mix = |from: f32, to: f32| from + (to - from) * share;
        Self {
            turn: mix(self.turn, to.turn),
            scale_x: mix(self.scale_x, to.scale_x),
            scale_y: mix(self.scale_y, to.scale_y),
            offset: crate::Point::new(mix(self.offset.x, to.offset.x), mix(self.offset.y, to.offset.y)),
        }
    }
}

/// One pose of an [`IconMotion`], and when it is reached.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconKeyframe {
    at_ms: u16,
    pose: IconPose,
    easing: Easing,
}

impl IconKeyframe {
    /// The icon at rest, reached `ms` milliseconds after the pointer came
    /// over the element, from the keyframe before it on
    /// [`Easing::InOutSine`]. Set what differs from rest with the methods
    /// below.
    pub fn at(ms: u16) -> Self {
        Self { at_ms: ms, pose: IconPose::REST, easing: Easing::InOutSine }
    }

    /// Turned `degrees` clockwise about the icon's middle; a negative turn
    /// is anticlockwise. A value that is not finite is ignored.
    pub fn turn(mut self, degrees: f32) -> Self {
        if degrees.is_finite() {
            self.pose.turn = degrees;
        }
        self
    }

    /// Stretched `x` times across and `y` times down about the icon's
    /// middle; 1 is its own size. Each is clamped to `0.0..=4.0`; a pair
    /// with a value that is not finite is ignored.
    pub fn scale(mut self, x: f32, y: f32) -> Self {
        if x.is_finite() && y.is_finite() {
            self.pose.scale_x = x.clamp(0.0, 4.0);
            self.pose.scale_y = y.clamp(0.0, 4.0);
        }
        self
    }

    /// Moved by `offset` logical pixels. An offset that is not finite is
    /// ignored.
    pub fn offset(mut self, offset: crate::Point) -> Self {
        if offset.x.is_finite() && offset.y.is_finite() {
            self.pose.offset = offset;
        }
        self
    }

    /// The curve the way from the keyframe before follows.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }
}

/// The keyframes the icons inside an element follow while the pointer is
/// over it: see [`Element::hover_motion`](crate::Element::hover_motion).
/// It starts from rest at 0 ms, passes through each keyframe in the order of
/// their times and holds the last one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct IconMotion {
    keyframes: Vec<IconKeyframe>,
    return_ms: Option<u16>,
}

impl IconMotion {
    /// No keyframes yet: the icons stay at rest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add `keyframe`. Keyframes are kept in the order of their times; one at
    /// the same time as another comes after it, so the way between them
    /// is a jump.
    pub fn keyframe(mut self, keyframe: IconKeyframe) -> Self {
        let at = self.keyframes.partition_point(|key| key.at_ms <= keyframe.at_ms);
        self.keyframes.insert(at, keyframe);
        self
    }

    /// How long the icons take to ease back to rest once the pointer has
    /// left, in milliseconds. `move_ms` by default; 0 puts them back at once.
    pub fn return_ms(mut self, ms: u16) -> Self {
        self.return_ms = Some(ms);
        self
    }

    /// How long playing lasts before the last keyframe is held, in ms.
    pub(crate) fn play_ms(&self) -> u16 {
        self.keyframes.last().map_or(0, |key| key.at_ms)
    }

    /// How long the return to rest lasts, in ms.
    pub(crate) fn rest_ms(&self) -> u16 {
        self.return_ms.unwrap_or(crate::tokens().motion.move_ms)
    }

    /// The pose `elapsed_ms` into playing from `from`, or into returning
    /// from it when `hovered` is false.
    pub(crate) fn pose(&self, from: IconPose, hovered: bool, elapsed_ms: f32) -> IconPose {
        if !hovered {
            let length = f32::from(self.rest_ms());
            let share = if length > 0.0 { Easing::OutCubic.sample(elapsed_ms / length) } else { 1.0 };
            return from.toward(IconPose::REST, share);
        }
        let (mut at, mut pose) = (0.0, from);
        for key in &self.keyframes {
            let next = f32::from(key.at_ms);
            if elapsed_ms < next {
                return pose.toward(key.pose, key.easing.sample((elapsed_ms - at) / (next - at)));
            }
            (at, pose) = (next, key.pose);
        }
        pose
    }
}

/// Retargetable animation of one logical-pixel value or opacity. A host owns
/// this state across rebuilt visual trees and schedules repaint while active.
#[derive(Clone, Copy, Debug)]
pub struct ScalarTransition {
    from: f32,
    target: f32,
    started: Instant,
    duration: Duration,
    easing: Easing,
}

impl ScalarTransition {
    /// A transition at rest on `value`. A value that is not finite is read as
    /// zero.
    pub fn new(value: f32, now: Instant) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Self {
            from: value,
            target: value,
            started: now,
            duration: Duration::ZERO,
            easing: Easing::default(),
        }
    }

    /// Choose the timing curve. The default is `Easing::OutCubic`.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// The value it is moving to, or rests on.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Move to `value` over `duration`, starting from the value shown at `now`.
    /// A value that is not finite, or that is already the target, is ignored.
    pub fn retarget(&mut self, value: f32, now: Instant, duration: Duration) {
        if !value.is_finite() || value == self.target {
            return;
        }
        self.from = self.value(now);
        self.target = value;
        self.started = now;
        self.duration = duration;
    }

    /// Honors a host's reduced-motion preference without changing the
    /// transition's ownership or target semantics.
    pub fn retarget_with_motion(
        &mut self,
        value: f32,
        now: Instant,
        duration: Duration,
        reduced_motion: bool,
    ) {
        if reduced_motion {
            self.snap_to(value, now);
        } else {
            self.retarget(value, now, duration);
        }
    }

    /// Jump to `value` and rest there. A value that is not finite is ignored.
    pub fn snap_to(&mut self, value: f32, now: Instant) {
        if !value.is_finite() {
            return;
        }
        self.from = value;
        self.target = value;
        self.started = now;
        self.duration = Duration::ZERO;
    }

    /// Move the whole transition by `by`: the value it started from and the
    /// one it goes to, so it keeps its pace and its curve. A content that
    /// grew above a scrolling viewport moves its offset this way without
    /// interrupting the scroll. A value that is not finite is ignored.
    pub fn shift(&mut self, by: f32) {
        if by.is_finite() {
            self.from += by;
            self.target += by;
        }
    }

    /// The value to show at `now`. It is the target once the duration is over.
    pub fn value(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return self.target;
        }
        let progress = (now.saturating_duration_since(self.started).as_secs_f32()
            / self.duration.as_secs_f32())
        .clamp(0.0, 1.0);
        self.from + (self.target - self.from) * self.easing.sample(progress)
    }

    /// Whether it is still moving at `now`. A host repaints while it is.
    pub fn is_active(&self, now: Instant) -> bool {
        !self.duration.is_zero()
            && self.from != self.target
            && now.saturating_duration_since(self.started) < self.duration
    }
}

/// Physical parameters of a unit-mass spring. A damping ratio of 1.0 settles
/// without overshoot; lower values bounce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringMotion {
    /// 1.0 settles without overshoot and lower values bounce. Negative values are
    /// read as zero. 1.0 by default.
    pub damping_ratio: f32,
    /// The spring constant: the natural angular frequency, in radians per second,
    /// is its square root. 800 by default.
    pub stiffness: f32,
}

impl Default for SpringMotion {
    fn default() -> Self {
        Self {
            damping_ratio: 1.0,
            stiffness: 800.0,
        }
    }
}

impl SpringMotion {
    /// A spring with no overshoot that has covered 99% of any distance after
    /// `duration`. It lets a duration token set the pace of a movement that
    /// still behaves like a spring: it keeps its speed when it is redirected.
    pub fn settling_in(duration: Duration) -> Self {
        // A critically damped spring is within 1% of rest when ωt = 6.64.
        let seconds = duration.as_secs_f32().max(0.001);
        let omega = 6.64 / seconds;
        Self {
            damping_ratio: 1.0,
            stiffness: omega * omega,
        }
    }
}

/// Retargetable spring for one logical-pixel value. Unlike `ScalarTransition`
/// it carries velocity across a retarget, so an interrupted movement or a
/// released drag continues instead of restarting from rest.
#[derive(Clone, Copy, Debug)]
pub struct ScalarSpring {
    from: f32,
    target: f32,
    velocity: f32,
    started: Instant,
    motion: SpringMotion,
}

impl ScalarSpring {
    /// Distance and speed below which the spring counts as settled.
    const REST: f32 = 0.01;

    /// A spring at rest on `value`, with the default [`SpringMotion`]. A value
    /// that is not finite is read as zero.
    pub fn new(value: f32, now: Instant) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Self {
            from: value,
            target: value,
            velocity: 0.0,
            started: now,
            motion: SpringMotion::default(),
        }
    }

    /// Choose the stiffness and damping.
    pub fn motion(mut self, motion: SpringMotion) -> Self {
        self.motion = motion;
        self
    }

    /// The value it is moving to, or rests on.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Move to `value`, keeping the position and the speed it has at `now`. A
    /// value that is not finite, or that is already the target, is ignored.
    pub fn retarget(&mut self, value: f32, now: Instant) {
        if !value.is_finite() || value == self.target {
            return;
        }
        let (position, velocity) = self.state(now);
        self.from = position;
        self.velocity = velocity;
        self.target = value;
        self.started = now;
    }

    /// Move to `value` on `motion`, for a discrete change such as a tap. From
    /// rest a spring starts at standstill and creeps for its first frames; a
    /// launch sets off at the speed an ease-out cubic over the same settling
    /// time starts with, so the first frame already shows the change, and a
    /// critically damped spring still never overshoots. In flight it keeps
    /// its position and any faster speed, and momentum away from `value`
    /// turns back as with [`Self::retarget`]. A value that is not finite, or
    /// that is already the target, is ignored.
    pub fn launch(&mut self, value: f32, motion: SpringMotion, now: Instant) {
        if !value.is_finite() || value == self.target {
            return;
        }
        let (position, velocity) = self.state(now);
        // A critically damped spring settles at ωt = 6.64; an ease-out cubic
        // over that time starts at 3 / 6.64 of ω times the distance.
        let launch = (value - position) * motion.stiffness.max(f32::EPSILON).sqrt() * (3.0 / 6.64);
        self.velocity = if velocity * launch >= 0.0 && velocity.abs() < launch.abs() { launch } else { velocity };
        self.from = position;
        self.target = value;
        self.started = now;
        self.motion = motion;
    }

    /// Release a drag: continue from `value` at the pointer's `velocity`
    /// (logical pixels per second) towards `target`.
    pub fn release(&mut self, value: f32, velocity: f32, target: f32, now: Instant) {
        if !(value.is_finite() && velocity.is_finite() && target.is_finite()) {
            return;
        }
        self.from = value;
        self.velocity = velocity;
        self.target = target;
        self.started = now;
    }

    /// Retarget, or snap when the host asks for reduced motion.
    pub fn retarget_with_motion(&mut self, value: f32, now: Instant, reduced_motion: bool) {
        if reduced_motion {
            self.snap_to(value, now);
        } else {
            self.retarget(value, now);
        }
    }

    /// Jump to `value` and rest there, with no speed left. A value that is not
    /// finite is ignored.
    pub fn snap_to(&mut self, value: f32, now: Instant) {
        if !value.is_finite() {
            return;
        }
        self.from = value;
        self.target = value;
        self.velocity = 0.0;
        self.started = now;
    }

    /// The position at `now`. It is exactly the target once the spring settles.
    pub fn value(&self, now: Instant) -> f32 {
        if self.is_active(now) {
            self.state(now).0
        } else {
            self.target
        }
    }

    /// The speed at `now`, in units per second.
    pub fn velocity(&self, now: Instant) -> f32 {
        self.state(now).1
    }

    /// Whether it is still more than 0.01 away from its target, or moving
    /// faster than 0.01 per second. A host repaints while it is.
    pub fn is_active(&self, now: Instant) -> bool {
        let (position, velocity) = self.state(now);
        (position - self.target).abs() > Self::REST || velocity.abs() > Self::REST
    }

    /// Closed-form position and velocity, so a frame of any length lands on
    /// the same curve and no per-frame integration state is needed.
    fn state(&self, now: Instant) -> (f32, f32) {
        let t = f64::from(now.saturating_duration_since(self.started).as_secs_f32());
        let x0 = f64::from(self.from - self.target);
        let v0 = f64::from(self.velocity);
        let zeta = f64::from(self.motion.damping_ratio.max(0.0));
        let omega = f64::from(self.motion.stiffness.max(f32::EPSILON)).sqrt();
        let (x, v) = if (zeta - 1.0).abs() < 1e-4 {
            let b = v0 + omega * x0;
            let decay = (-omega * t).exp();
            (decay * (x0 + b * t), decay * (b - omega * (x0 + b * t)))
        } else if zeta < 1.0 {
            let damped = omega * (1.0 - zeta * zeta).sqrt();
            let b = (v0 + zeta * omega * x0) / damped;
            let decay = (-zeta * omega * t).exp();
            let (sin, cos) = (damped * t).sin_cos();
            let x = decay * (x0 * cos + b * sin);
            (x, decay * damped * (b * cos - x0 * sin) - zeta * omega * x)
        } else {
            let root = omega * (zeta * zeta - 1.0).sqrt();
            let (r1, r2) = (-zeta * omega - root, -zeta * omega + root);
            let c2 = (v0 - r1 * x0) / (r2 - r1);
            let c1 = x0 - c2;
            let (e1, e2) = ((r1 * t).exp(), (r2 * t).exp());
            (c1 * e1 + c2 * e2, c1 * r1 * e1 + c2 * r2 * e2)
        };
        (self.target + x as f32, v as f32)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ColorTransition {
    from: Color,
    target: Color,
    started: Instant,
    duration: Duration,
}

impl ColorTransition {
    pub(crate) fn new(color: Color, now: Instant) -> Self {
        Self {
            from: color,
            target: color,
            started: now,
            duration: Duration::ZERO,
        }
    }

    pub(crate) fn retarget(&mut self, color: Color, now: Instant, duration: Duration) {
        if color == self.target || (color.a == 0 && self.target.a == 0) {
            return;
        }
        let mut from = self.color(now);
        let mut target = color;
        // Preserve the visible hue while fading to or from an absent fill.
        if from.a == 0 {
            from.r = target.r;
            from.g = target.g;
            from.b = target.b;
        }
        if target.a == 0 {
            target.r = from.r;
            target.g = from.g;
            target.b = from.b;
        }
        self.from = from;
        self.target = target;
        self.started = now;
        self.duration = duration;
    }

    pub(crate) fn color(&self, now: Instant) -> Color {
        if self.duration.is_zero() {
            return self.target;
        }
        let progress = (now.saturating_duration_since(self.started).as_secs_f32()
            / self.duration.as_secs_f32())
        .clamp(0.0, 1.0);
        // Fast onset, soft finish; the same curve applies when retargeting mid-flight.
        let eased = 1.0 - (1.0 - progress).powi(3);
        // Mix what each color contributes once painted. Mixing raw channels
        // between an opaque fill and a translucent veil passes through a
        // bright, half-opaque color: a flash on every hover.
        let weighted = |color: Color| {
            let alpha = f32::from(color.a) / 255.0;
            [
                f32::from(color.r) * alpha,
                f32::from(color.g) * alpha,
                f32::from(color.b) * alpha,
                f32::from(color.a),
            ]
        };
        let (from, target) = (weighted(self.from), weighted(self.target));
        let mixed: [f32; 4] = std::array::from_fn(|index| from[index] + (target[index] - from[index]) * eased);
        let alpha = mixed[3] / 255.0;
        if alpha <= 0.0 {
            return Color::TRANSPARENT;
        }
        let plain = |value: f32| (value / alpha).round().clamp(0.0, 255.0) as u8;
        Color::rgba(plain(mixed[0]), plain(mixed[1]), plain(mixed[2]), mixed[3].round() as u8)
    }

    pub(crate) fn is_active(&self, now: Instant) -> bool {
        !self.duration.is_zero()
            && self.from != self.target
            && now.saturating_duration_since(self.started) < self.duration
    }
}

fn channel(from: u8, to: u8, progress: f32) -> u8 {
    (f32::from(from) + (f32::from(to) - f32::from(from)) * progress).round() as u8
}

/// One cycle of a skeleton's pulse, from its base to its highlight and back.
pub(crate) const SKELETON_PERIOD: Duration = Duration::from_millis(1_600);

/// A slow, paint-only pulse for passive loading surfaces, one cycle every
/// `period`. Its phase belongs to a stable element ID in UiTree, so
/// rebuilding an Element does not restart it.
pub(crate) fn skeleton_color(
    base: Color,
    highlight: Color,
    started: Instant,
    now: Instant,
    reduced_motion: bool,
    period: Duration,
) -> Color {
    if reduced_motion || period.is_zero() {
        return base;
    }
    let phase = (now.saturating_duration_since(started).as_secs_f32() % period.as_secs_f32())
        / period.as_secs_f32();
    let triangle = 1.0 - (2.0 * phase - 1.0).abs();
    let amount = Easing::InOutCubic.sample(triangle);
    Color::rgba(
        channel(base.r, highlight.r, amount),
        channel(base.g, highlight.g, amount),
        channel(base.b, highlight.b, amount),
        channel(base.a, highlight.a, amount),
    )
}

/// Continuous paint-only rotation for a passive progress icon. The phase is
/// measured from a stable element ID's start time and stops under reduced motion.
pub(crate) fn spinner_angle(started: Instant, now: Instant, reduced_motion: bool, period: f32) -> f32 {
    if reduced_motion {
        return 0.0;
    }
    (now.saturating_duration_since(started).as_secs_f32() / period * std::f32::consts::TAU)
        .rem_euclid(std::f32::consts::TAU)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The y of a cubic Bézier at x = `t`, found by plain bisection on x with no
    /// shortcut, to check `sample` against.
    fn reference(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
        let at = |a: f32, b: f32, s: f32| 3.0 * (1.0 - s).powi(2) * s * a + 3.0 * (1.0 - s) * s * s * b + s.powi(3);
        let (mut low, mut high) = (0.0_f64, 1.0_f64);
        for _ in 0..200 {
            let mid = (low + high) / 2.0;
            if f64::from(at(x1, x2, mid as f32)) < f64::from(t) {
                low = mid;
            } else {
                high = mid;
            }
        }
        at(y1, y2, ((low + high) / 2.0) as f32)
    }

    #[test]
    fn a_cubic_bezier_on_the_diagonal_is_linear() {
        for step in 0..=20 {
            let t = step as f32 / 20.0;
            assert!((Easing::CubicBezier(0.0, 0.0, 1.0, 1.0).sample(t) - t).abs() < 1e-4, "{t}");
        }
    }

    #[test]
    fn the_sheet_curve_starts_and_ends_at_rest_and_never_falls_back() {
        let curve = Easing::CubicBezier(0.32, 0.72, 0.0, 1.0);
        assert_eq!(curve.sample(0.0), 0.0);
        assert!((curve.sample(1.0) - 1.0).abs() < 1e-6);
        let mut last = 0.0;
        for step in 0..=1000 {
            let value = curve.sample(step as f32 / 1000.0);
            assert!(value >= last - 1e-6, "falls back at {step}");
            last = value;
        }
        // The curve is fast at first: well past half way by the middle of its time.
        assert!(curve.sample(0.5) > 0.8);
    }

    #[test]
    fn the_sheet_curve_matches_a_dense_bisection_at_the_middle() {
        let (x1, y1, x2, y2) = (0.32, 0.72, 0.0, 1.0);
        let value = Easing::CubicBezier(x1, y1, x2, y2).sample(0.5);
        assert!((value - reference(x1, y1, x2, y2, 0.5)).abs() < 0.01, "{value}");
    }

    #[test]
    fn scalar_retarget_keeps_current_position_without_jump() {
        let start = Instant::now();
        let mut scalar = ScalarTransition::new(0.0, start);
        scalar.retarget(100.0, start, Duration::from_millis(200));
        let halfway = start + Duration::from_millis(100);
        let before = scalar.value(halfway);
        assert!(before > 50.0 && before < 100.0);
        scalar.retarget(20.0, halfway, Duration::from_millis(100));
        assert_eq!(scalar.value(halfway), before);
        assert_eq!(scalar.value(halfway + Duration::from_millis(100)), 20.0);
        assert!(!scalar.is_active(halfway + Duration::from_millis(100)));
    }

    #[test]
    fn scalar_reduced_motion_and_invalid_target_are_safe() {
        let start = Instant::now();
        let mut scalar = ScalarTransition::new(30.0, start).easing(Easing::InOutCubic);
        scalar.retarget_with_motion(300.0, start, Duration::from_millis(160), true);
        assert_eq!(scalar.value(start), 300.0);
        assert!(!scalar.is_active(start));
        scalar.retarget(f32::NAN, start, Duration::from_millis(100));
        assert_eq!(scalar.target(), 300.0);
        assert_eq!(Easing::InOutCubic.sample(0.5), 0.5);
    }

    #[test]
    fn a_spring_curve_overshoots_dips_back_and_rests_on_the_end() {
        let curve: Vec<f32> = (0..=100).map(|step| Easing::Spring.sample(step as f32 / 100.0)).collect();
        assert_eq!((curve[0], curve[100], Easing::Spring.sample(2.0)), (0.0, 1.0, 1.0));
        // It starts at standstill, as a spring from rest.
        assert!(curve[1] < 0.01, "{}", curve[1]);
        let (peak, at) = curve.iter().zip(0..).fold((0.0_f32, 0), |best, (&v, i)| if v > best.0 { (v, i) } else { best });
        assert!((1.12..1.16).contains(&peak) && (35..=39).contains(&at), "{peak} at {at}");
        let dip = curve[at..].iter().copied().fold(f32::MAX, f32::min);
        assert!((0.97..0.99).contains(&dip), "{dip}");
        assert!(curve[90..].iter().all(|v| (v - 1.0).abs() < 0.01), "{:?}", &curve[90..]);
    }

    #[test]
    fn transition_reaches_target_and_can_retarget_midway() {
        let start = Instant::now();
        let mut color = ColorTransition::new(Color::rgb(20, 20, 20), start);
        color.retarget(Color::rgb(80, 80, 80), start, Duration::from_millis(120));
        let halfway = start + Duration::from_millis(60);
        assert!(color.is_active(halfway));
        assert!(color.color(halfway).r > 20);
        assert!(color.color(halfway).r < 80);
        let midway_color = color.color(halfway);
        color.retarget(Color::rgb(30, 30, 30), halfway, Duration::from_millis(120));
        assert_eq!(color.color(halfway), midway_color);
        assert_eq!(
            color.color(halfway + Duration::from_millis(120)),
            Color::rgb(30, 30, 30)
        );
        assert!(!color.is_active(halfway + Duration::from_millis(120)));
    }

    #[test]
    fn absent_fill_fades_without_dark_fringe() {
        let start = Instant::now();
        let mut color = ColorTransition::new(Color::TRANSPARENT, start);
        color.retarget(Color::rgb(100, 120, 140), start, Duration::from_millis(100));
        let intermediate = color.color(start + Duration::from_millis(50));
        assert_eq!(
            (intermediate.r, intermediate.g, intermediate.b),
            (100, 120, 140)
        );
        assert!(intermediate.a > 0 && intermediate.a < 255);

        let opaque_at = start + Duration::from_millis(100);
        color.retarget(Color::TRANSPARENT, opaque_at, Duration::from_millis(100));
        let transparent_at = opaque_at + Duration::from_millis(100);
        assert_eq!(color.color(transparent_at).a, 0);
        color.retarget(
            Color::TRANSPARENT,
            transparent_at,
            Duration::from_millis(100),
        );
        assert!(!color.is_active(transparent_at));
    }

    #[test]
    fn skeleton_pulse_cycles_and_reduced_motion_is_static() {
        let start = Instant::now();
        let base = Color::rgb(30, 30, 30);
        let highlight = Color::rgb(50, 50, 50);
        assert_eq!(skeleton_color(base, highlight, start, start, false, SKELETON_PERIOD), base);
        assert_eq!(
            skeleton_color(
                base,
                highlight,
                start,
                start + Duration::from_millis(800),
                false,
                SKELETON_PERIOD,
            ),
            highlight
        );
        assert_eq!(
            skeleton_color(
                base,
                highlight,
                start,
                start + Duration::from_millis(1_600),
                false,
                SKELETON_PERIOD,
            ),
            base
        );
        assert_eq!(
            skeleton_color(
                base,
                highlight,
                start,
                start + Duration::from_millis(800),
                true,
                SKELETON_PERIOD,
            ),
            base
        );
    }

    #[test]
    fn spinner_rotates_and_reduced_motion_holds_one_angle() {
        let start = Instant::now();
        let quarter = start + Duration::from_millis(225);
        assert_eq!(spinner_angle(start, start, false, 0.9), 0.0);
        assert!((spinner_angle(start, quarter, false, 0.9) - std::f32::consts::FRAC_PI_2).abs() < 0.01);
        assert!(spinner_angle(start, start + Duration::from_millis(900), false, 0.9) < 0.001);
        assert_eq!(spinner_angle(start, quarter, true, 0.9), 0.0);
    }

    #[test]
    fn spring_settles_on_its_target_for_every_damping_regime() {
        let start = Instant::now();
        for damping_ratio in [0.5, 1.0, 2.0] {
            let mut spring = ScalarSpring::new(0.0, start).motion(SpringMotion {
                damping_ratio,
                stiffness: 800.0,
            });
            spring.retarget(300.0, start);
            assert!(spring.is_active(start + Duration::from_millis(16)));
            let settled = start + Duration::from_secs(3);
            assert!(!spring.is_active(settled), "ratio {damping_ratio}");
            assert_eq!(spring.value(settled), 300.0);
        }
    }

    #[test]
    fn critically_damped_spring_never_overshoots() {
        let start = Instant::now();
        let mut spring = ScalarSpring::new(0.0, start);
        spring.retarget(100.0, start);
        let mut previous = 0.0;
        for ms in 1..1000 {
            let value = spring.value(start + Duration::from_millis(ms));
            assert!(value >= previous && value <= 100.0, "{ms} ms: {value}");
            previous = value;
        }
    }

    #[test]
    fn spring_keeps_position_and_velocity_across_a_retarget() {
        let start = Instant::now();
        let mut spring = ScalarSpring::new(0.0, start);
        spring.retarget(400.0, start);
        let mid = start + Duration::from_millis(60);
        let (position, velocity) = (spring.value(mid), spring.velocity(mid));
        assert!(velocity > 100.0);
        spring.retarget(-200.0, mid);
        assert!((spring.value(mid) - position).abs() < 0.01);
        assert!((spring.velocity(mid) - velocity).abs() < 0.5);
        // Momentum carries it further right before it turns back.
        assert!(spring.value(mid + Duration::from_millis(16)) > position);
    }

    #[test]
    fn released_drag_continues_at_pointer_velocity_and_reduced_motion_snaps() {
        let start = Instant::now();
        let mut spring = ScalarSpring::new(0.0, start);
        spring.release(50.0, 900.0, 0.0, start);
        assert!(spring.value(start + Duration::from_millis(16)) > 50.0);
        spring.retarget_with_motion(120.0, start, true);
        assert_eq!(spring.value(start), 120.0);
        assert!(!spring.is_active(start));
    }
}

#[cfg(test)]
mod spring_tests {
    use super::*;

    #[test]
    fn a_launch_moves_on_its_first_frame_like_an_ease_out_and_never_overshoots() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_micros(ms * 1000);
        let motion = SpringMotion::settling_in(Duration::from_millis(170));
        let frame = Duration::from_micros(16_667);
        let mut retargeted = ScalarSpring::new(0.0, start).motion(motion);
        retargeted.retarget(100.0, start);
        let mut launched = ScalarSpring::new(0.0, start);
        launched.launch(100.0, motion, start);
        let eased = 100.0 * Easing::OutCubic.sample(frame.as_secs_f32() / 0.17);
        // From standstill a retarget covers a seventh of the way in a frame.
        assert!(retargeted.value(start + frame) < 15.0);
        let first = launched.value(start + frame);
        assert!(first > eased * 0.9, "{first} against an ease-out's {eased}");
        let mut previous = 0.0;
        for ms in 1..=600 {
            let value = launched.value(at(ms));
            assert!(value >= previous && value <= 100.0, "{value} at {ms} ms");
            previous = value;
        }
        assert!((launched.value(at(170)) - 100.0).abs() < 1.0);
        assert!(!launched.is_active(at(600)));

        // In flight it keeps a faster speed, and turns back from momentum.
        let mid = at(40);
        let (position, speed) = (launched.value(mid), launched.velocity(mid));
        launched.launch(110.0, motion, mid);
        assert!((launched.value(mid) - position).abs() < 1e-3);
        assert!((launched.velocity(mid) - speed).abs() < 1e-3);
        launched.launch(0.0, motion, mid);
        assert!((launched.velocity(mid) - speed).abs() < 1e-3);
        assert!(launched.value(mid + frame) > position, "momentum carries it on before it turns");
        // Ignored like a retarget.
        launched.launch(f32::NAN, motion, mid);
        assert_eq!(launched.target(), 0.0);
    }

    #[test]
    fn a_spring_paced_by_a_duration_arrives_on_time_without_overshoot_and_keeps_its_speed() {
        let start = Instant::now();
        let at = |ms| start + Duration::from_millis(ms);
        let motion = SpringMotion::settling_in(Duration::from_millis(170));
        let mut spring = ScalarSpring::new(0.0, start).motion(motion);
        spring.retarget(100.0, start);

        // 99% of the way after the duration, and never past the target.
        assert!((spring.value(at(170)) - 99.0).abs() < 0.5, "{}", spring.value(at(170)));
        for ms in (0..=600).step_by(10) {
            let value = spring.value(at(ms));
            assert!((0.0..=100.0).contains(&value), "{value} at {ms} ms");
        }
        assert!(!spring.is_active(at(600)));

        // Redirected mid-flight it continues at the speed it had: no stop.
        let before = spring.velocity(at(60));
        assert!(before > 100.0, "moving fast at 60 ms: {before}");
        spring.retarget(300.0, at(60));
        assert!((spring.velocity(at(60)) - before).abs() < 1.0);
        assert!((spring.value(at(60)) - ScalarSpring::new(0.0, start).motion(motion).value(at(60))).abs() < 100.0);

        // A shorter duration is a stiffer spring.
        assert!(
            SpringMotion::settling_in(Duration::from_millis(85)).stiffness
                > motion.stiffness * 3.9
        );
        assert!(SpringMotion::settling_in(Duration::ZERO).stiffness.is_finite());
    }
}

#[cfg(test)]
mod color_transition_tests {
    use super::*;

    #[test]
    fn fading_an_opaque_fill_into_a_veil_never_flashes_brighter_than_both() {
        let started = Instant::now();
        let fill = Color::rgb(40, 41, 44);
        let veil = Color::rgba(255, 255, 255, 26);
        let mut transition = ColorTransition::new(fill, started);
        transition.retarget(veil, started, Duration::from_millis(120));
        // Brightness once painted over the window background.
        let painted = |color: Color| {
            let alpha = f32::from(color.a) / 255.0;
            f32::from(color.r) * alpha + 30.0 * (1.0 - alpha)
        };
        let (low, high) = (painted(fill).min(painted(veil)), painted(fill).max(painted(veil)));
        for step in 0..=12 {
            let seen = painted(transition.color(started + Duration::from_millis(step * 10)));
            assert!((low - 1.0..=high + 1.0).contains(&seen), "{seen} at {step}0 ms");
        }
        assert_eq!(transition.color(started + Duration::from_millis(120)), veil);
        // Opaque colors still mix channel by channel.
        let mut opaque = ColorTransition::new(Color::rgb(0, 0, 0), started);
        opaque.retarget(Color::rgb(200, 100, 50), started, Duration::from_millis(100));
        assert_eq!(opaque.color(started + Duration::from_millis(100)), Color::rgb(200, 100, 50));
    }
}

#[cfg(test)]
mod icon_motion_tests {
    use super::*;

    fn gear() -> IconMotion {
        IconMotion::new().keyframe(IconKeyframe::at(1070).turn(180.0)).keyframe(IconKeyframe::at(120))
    }

    #[test]
    fn keyframes_play_in_the_order_of_their_times_eased_and_hold_the_last() {
        let gear = gear();
        let turn = |ms: f32| gear.pose(IconPose::REST, true, ms).turn;
        assert_eq!((turn(0.0), turn(100.0), turn(120.0)), (0.0, 0.0, 0.0), "held still for 120 ms");
        assert!((turn(595.0) - 90.0).abs() < 0.01, "half way at the middle of an in-out curve");
        assert!(turn(240.0) < 180.0 * (120.0 / 950.0), "a gentle start: {}", turn(240.0));
        assert_eq!((turn(1070.0), turn(5000.0)), (180.0, 180.0), "the last keyframe is held");
        assert_eq!(gear.play_ms(), 1070);

        let mic = IconMotion::new()
            .keyframe(IconKeyframe::at(160).turn(-5.0))
            .keyframe(IconKeyframe::at(400).turn(5.0))
            .keyframe(IconKeyframe::at(650));
        let turns: Vec<f32> = [160.0, 280.0, 400.0, 650.0].map(|ms| mic.pose(IconPose::REST, true, ms).turn).into();
        assert_eq!(turns, [-5.0, 0.0, 5.0, 0.0]);

        let linear = IconMotion::new().keyframe(IconKeyframe::at(100).scale(1.06, 0.88).offset(crate::Point::new(0.0, 2.0)).easing(Easing::Linear));
        let pose = linear.pose(IconPose::REST, true, 50.0);
        assert!((pose.scale_x - 1.03).abs() < 1e-5 && (pose.scale_y - 0.94).abs() < 1e-5 && (pose.offset.y - 1.0).abs() < 1e-5, "{pose:?}");
    }

    #[test]
    fn it_plays_from_where_it_is_and_returns_from_there_to_rest() {
        let gear = gear().return_ms(200);
        let turned = gear.pose(IconPose::REST, true, 2000.0);
        assert_eq!(gear.pose(turned, false, 0.0), turned, "no jump on leaving");
        let halfway = gear.pose(turned, false, 100.0).turn;
        assert!(halfway > 0.0 && halfway < 180.0, "{halfway}");
        assert_eq!(gear.pose(turned, false, 200.0), IconPose::REST);
        assert_eq!(gear.pose(turned, true, 0.0), turned, "no jump on coming back");
        assert_eq!(gear.clone().return_ms(0).pose(turned, false, 0.0), IconPose::REST, "a return of 0 ms is at once");
        assert_eq!(IconMotion::new().pose(turned, true, 10.0), turned, "no keyframes: it stays");
    }

    #[test]
    fn bad_numbers_are_ignored_or_clamped() {
        let key = IconKeyframe::at(10).turn(f32::NAN).scale(f32::INFINITY, 1.0).offset(crate::Point::new(f32::NAN, 0.0));
        assert_eq!(key, IconKeyframe::at(10));
        assert_eq!(IconKeyframe::at(10).scale(-1.0, 9.0).pose, IconPose { scale_x: 0.0, scale_y: 4.0, ..IconPose::REST });
    }
}
