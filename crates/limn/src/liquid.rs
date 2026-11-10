//! aurora's liquid pop for a pane of glass, from its `GlassMorph.qml` and `Motion.qml`,
//! fitted there frame by frame to Apple's Liquid Glass. The glass leaves its trigger as a
//! small round drop, swells into the panel on springs of its own for each edge, gains its
//! veil and corners as it forms after a faint bloom, and the content, at its own place,
//! focuses in through it. Closing shrinks the panel evenly into the drop as the content
//! leaves, and the drop melts into the trigger.

use crate::{CornerRadii, DrawCommand, Rect};
use std::f32::consts::TAU;

/// A spring as `Motion.qml` gives one, SwiftUI's: perceptual duration in seconds and bounce;
/// ω = 2π / duration, ζ = 1 − bounce.
type Spring = (f32, f32);
const DROP: Spring = (0.16, 0.0);
const OPEN: Spring = (0.26, 0.30);
const CLOSE: Spring = (0.22, 0.0);
const MERGE: Spring = (0.20, 0.0);
/// How long the drop stays a drop, and when the closing drop starts melting into its trigger.
const DROP_HOLD: f32 = 0.06;
const CLOSE_HOLD: f32 = 0.17;
/// Seconds into the pop: the bloom peaks and clears, the shape forms, the content focuses.
const BLOOM_PEAK: f32 = 0.167;
const BLOOM_END: f32 = 0.367;
const SHAPE: (f32, f32) = (0.20, 0.333);
const FOCUS: (f32, f32) = (0.233, 0.40);

/// How long a glass popover's entrance runs: the pop's phases and its springs settled. Give
/// it to [`Element::popover_animation_ms`](crate::Element::popover_animation_ms) with
/// [`Element::popover_morph`](crate::Element::popover_morph).
pub const GLASS_POP_OPEN_MS: u16 = 600;
/// How long a glass popover's exit runs, until its drop has melted into the trigger.
pub const GLASS_POP_CLOSE_MS: u16 = 450;

/// The pop at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pop {
    pub rect: Rect,
    /// 0 a round drop, 1 the panel's own corners.
    pub shaped: f32,
    /// The milky light of the forming glass, 0 to 1.
    pub bloom: f32,
    /// How much of the veil is there.
    pub presence: f32,
    pub opacity: f32,
    /// 0 the content hidden, 1 shown.
    pub reveal: f32,
}

fn span(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// The drop the panel leaves its trigger as: a circle over the trigger's centre, 45% of the
/// panel's shorter side, 28 to 80 across.
fn drop_of(trigger: Rect, panel: Rect) -> Rect {
    let across = (0.45 * panel.width.min(panel.height)).clamp(28.0, 80.0);
    Rect::new(trigger.x + trigger.width / 2.0 - across / 2.0, trigger.y - across * 0.25, across, across)
}

/// Four springs, one for each of the rectangle's numbers, from `start` at rest toward each
/// stage's target from its second on, keeping their speed as the target changes; integrated
/// as aurora's `SpringValue`, in steps of at most 1/240 s.
fn follow(start: Rect, stages: &[(f32, Rect, Spring)], t: f32) -> Rect {
    let mut at = [start.x, start.y, start.width, start.height];
    let mut speed = [0.0; 4];
    let mut time = 0.0;
    while time < t {
        let step = (1.0 / 240.0_f32).min(t - time);
        let (_, goal, (duration, bounce)) = stages.iter().rev().find(|stage| stage.0 <= time).unwrap_or(&stages[0]);
        let omega = TAU / duration.max(0.01);
        let zeta = (1.0 - bounce).max(0.0);
        let goal = [goal.x, goal.y, goal.width, goal.height];
        for side in 0..4 {
            speed[side] += (-omega * omega * (at[side] - goal[side]) - 2.0 * zeta * omega * speed[side]) * step;
            at[side] += speed[side] * step;
        }
        time += step;
    }
    Rect::new(at[0], at[1], at[2].max(1.0), at[3].max(1.0))
}

/// The pop from `trigger` to `panel`, `t` seconds into opening or, `closing`, into closing.
pub(crate) fn pop(trigger: Rect, panel: Rect, t: f32, closing: bool) -> Pop {
    let drop = drop_of(trigger, panel);
    if !closing {
        let shaped = span(t, SHAPE.0, SHAPE.1);
        return Pop {
            rect: follow(trigger, &[(0.0, drop, DROP), (DROP_HOLD, panel, OPEN)], t),
            shaped,
            bloom: if t < BLOOM_PEAK { span(t, 0.0, BLOOM_PEAK) } else { 1.0 - span(t, BLOOM_PEAK, BLOOM_END) },
            presence: 0.25 + 0.75 * shaped,
            opacity: 1.0,
            reveal: span(t, FOCUS.0, FOCUS.1),
        };
    }
    let rect = follow(panel, &[(0.0, drop, CLOSE), (CLOSE_HOLD, trigger, MERGE)], t);
    let progress = ((rect.height - trigger.height) / (panel.height - trigger.height).max(1.0)).max(0.0);
    // Closing keeps the veil: the returning drop is the densest glass of the motion.
    Pop {
        rect,
        shaped: span(progress, 0.8, 0.98),
        bloom: (1.0 - progress).clamp(0.0, 1.0),
        presence: 1.0,
        opacity: span(progress, 0.0, 0.12),
        reveal: span(progress, 0.68, 0.98),
    }
}

/// The seconds an entrance or exit eased on limn's default curve has run, from how far it
/// has got (`eased`, 0 to 1) and its length.
pub(crate) fn seconds(eased: f32, duration_ms: u16) -> f32 {
    (1.0 - (1.0 - eased.clamp(0.0, 1.0)).cbrt()) * f32::from(duration_ms) / 1000.0
}

/// Whether `command` is the popover's own shell, painted over all of `panel`.
fn shell(command: &DrawCommand, panel: Rect) -> bool {
    matches!(command, DrawCommand::Shadow { rect, .. } | DrawCommand::Glass { rect, .. }
        | DrawCommand::FillRect { rect, .. } | DrawCommand::StrokeRect { rect, .. } if *rect == panel)
}

/// Gives a popover's paint from `start` the pop: its shell, painted at `panel`, takes the
/// pop's shape, corners, veil and light; the rest stays where it is laid out and is seen
/// only through the shape as it focuses in. `fade` dims the colours of a command.
pub(crate) fn apply(commands: &mut Vec<DrawCommand>, start: usize, panel: Rect, pop: Pop, fade: impl Fn(&mut DrawCommand, f32)) {
    let drop = pop.rect.width.min(pop.rect.height) / 2.0;
    let corner = |own: f32| drop + (own.min(pop.rect.width.min(pop.rect.height) / 2.0) - drop) * pop.shaped;
    let mut radius = drop;
    let mut content = commands.len();
    for (index, command) in commands.iter_mut().enumerate().skip(start) {
        if !shell(command, panel) {
            content = content.min(index);
            fade(command, pop.reveal);
            continue;
        }
        match command {
            DrawCommand::Glass { rect, radius: own, material, opacity } => {
                *rect = pop.rect;
                *own = corner(*own);
                radius = *own;
                material.presence *= pop.presence;
                material.bloom = pop.bloom;
                *opacity *= pop.opacity;
            }
            DrawCommand::Shadow { rect, radius: own, .. } | DrawCommand::FillRect { rect, radius: own, .. }
            | DrawCommand::StrokeRect { rect, radius: own, .. } => {
                *rect = pop.rect;
                *own = corner(*own);
                fade(command, pop.opacity);
            }
            _ => {}
        }
    }
    if content < commands.len() {
        let radii = CornerRadii { top_left: radius, top_right: radius, bottom_right: radius, bottom_left: radius };
        commands.insert(content, DrawCommand::RoundClip { rect: pop.rect, radii });
        commands.insert(content, DrawCommand::PushClip(pop.rect));
        commands.push(DrawCommand::PopClip);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUTTON: Rect = Rect { x: 300.0, y: 400.0, width: 44.0, height: 44.0 };
    const PANEL: Rect = Rect { x: 100.0, y: 60.0, width: 260.0, height: 700.0 };

    #[test]
    fn it_leaves_its_trigger_as_a_drop_overshoots_into_the_panel_and_focuses_its_content_last() {
        let start = pop(BUTTON, PANEL, 0.0, false);
        assert_eq!(start.rect, BUTTON);
        let dropped = pop(BUTTON, PANEL, DROP_HOLD, false);
        assert!(dropped.rect.width < 100.0 && dropped.shaped == 0.0, "a small round drop first: {dropped:?}");
        let bloom = pop(BUTTON, PANEL, BLOOM_PEAK, false).bloom;
        assert!((bloom - 1.0).abs() < 1e-3, "the light peaks at {BLOOM_PEAK} s");
        let widest = (1..60).map(|frame| pop(BUTTON, PANEL, frame as f32 / 100.0, false).rect.height).fold(0.0, f32::max);
        assert!(widest > PANEL.height, "a little past the panel before it settles: {widest}");
        let shown = pop(BUTTON, PANEL, 0.6, false);
        assert!((shown.rect.height - PANEL.height).abs() < 4.0 && shown.reveal == 1.0 && shown.shaped == 1.0 && shown.bloom == 0.0);
        assert!(pop(BUTTON, PANEL, 0.2, false).reveal == 0.0, "the content waits for the shape");
    }

    #[test]
    fn closing_it_shrinks_into_the_drop_then_melts_into_the_trigger() {
        let early = pop(BUTTON, PANEL, 0.1, true);
        assert!(early.reveal < 1.0 && early.opacity == 1.0, "the content leaves first: {early:?}");
        let gone = pop(BUTTON, PANEL, 0.45, true);
        assert!(gone.opacity < 0.05 && (gone.rect.width - BUTTON.width).abs() < 6.0, "back into the button: {gone:?}");
    }

    #[test]
    fn the_shell_takes_the_shape_and_the_content_is_seen_through_it() {
        let material = crate::GlassMaterial::new(crate::Color::rgba(38, 38, 40, 122));
        let mut commands = vec![
            DrawCommand::Glass { rect: PANEL, radius: 32.0, material, opacity: 1.0 },
            DrawCommand::FillRect { rect: Rect::new(110.0, 70.0, 240.0, 48.0), radius: 8.0, color: crate::Color::WHITE },
        ];
        let at = pop(BUTTON, PANEL, 0.3, false);
        apply(&mut commands, 0, PANEL, at, |command, share| if let DrawCommand::FillRect { color, .. } = command {
            color.a = (f32::from(color.a) * share) as u8;
        });
        assert!(matches!(commands[0], DrawCommand::Glass { rect, .. } if rect == at.rect));
        assert!(matches!(commands[1], DrawCommand::PushClip(rect) if rect == at.rect));
        assert!(matches!(commands[2], DrawCommand::RoundClip { .. }));
        assert!(matches!(commands[3], DrawCommand::FillRect { rect, .. } if rect.x == 110.0), "the content stays where it is laid out");
        assert!(matches!(commands.last(), Some(DrawCommand::PopClip)));
    }
}
