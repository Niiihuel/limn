//! A stationary native touch requests the tree's existing context action.
use super::relative;
use eframe::egui::{self, Pos2};
use limn::{Frame, UiEvent};
use std::collections::VecDeque;

const HOLD_SECONDS: f64 = 0.430;
const MOVE_THRESHOLD: f32 = 8.0;

#[derive(Clone)]
struct Touch {
    device: egui::TouchDeviceId,
    id: egui::TouchId,
    position: Pos2,
    context: Option<String>,
    since: f64,
    cancelled: bool,
    consumed: bool,
}

#[derive(Clone, Default)]
struct Hold {
    touch: Option<Touch>,
    swallow_release: bool,
}

pub(super) fn translate(ctx: &egui::Context, origin: Pos2, translated: Vec<UiEvent>, frame: &Frame) -> Vec<UiEvent> {
    let key = egui::Id::unique(("limn_touch_context", ctx.viewport_id()));
    let mut hold = ctx.data_mut(|data| data.get_temp::<Hold>(key).unwrap_or_default());
    let (now, raw) = ctx.input(|input| (input.time, input.events.clone()));
    let mut suppress = VecDeque::new();
    let mut cancel_press = false;
    for event in raw {
        match event {
            egui::Event::Touch { device_id, id, phase, pos, .. } => {
                if phase == egui::TouchPhase::Start {
                    if let Some(touch) = &mut hold.touch {
                        // A second finger belongs to another gesture.
                        touch.cancelled = true;
                    } else {
                        hold.swallow_release = false;
                        let context = frame.context_menu_at(relative(origin, pos)).map(str::to_owned);
                        hold.touch = Some(Touch { device: device_id, id, position: pos, since: now,
                            cancelled: !pos.is_finite() || context.is_none(), context, consumed: false });
                    }
                } else if let Some(touch) = hold.touch.as_mut().filter(|touch| touch.device == device_id && touch.id == id) {
                    match phase {
                        egui::TouchPhase::Move if !pos.is_finite() || pos.distance(touch.position) >= MOVE_THRESHOLD => touch.cancelled = true,
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            hold.swallow_release = touch.consumed || phase == egui::TouchPhase::Cancel;
                            cancel_press |= phase == egui::TouchPhase::Cancel;
                            hold.touch = None;
                        }
                        _ => {}
                    }
                }
            }
            egui::Event::PointerButton { button: egui::PointerButton::Primary, pressed, .. } => {
                if pressed && hold.touch.is_none() { hold.swallow_release = false; }
                suppress.push_back(hold.swallow_release || hold.touch.as_ref().is_some_and(|touch| touch.consumed));
                if !pressed { hold.swallow_release = false; }
            }
            egui::Event::PointerMoved(_) => suppress.push_back(hold.touch.as_ref().is_some_and(|touch| touch.consumed)),
            egui::Event::PointerGone => {
                hold.touch = None;
                suppress.push_back(false);
            }
            egui::Event::MouseWheel { .. } | egui::Event::WindowFocused(false) => {
                if let Some(touch) = &mut hold.touch {
                    touch.cancelled = true;
                    cancel_press = true;
                }
            }
            _ => {}
        }
    }
    let mut events = Vec::with_capacity(translated.len() + 2);
    for event in translated {
        let pointer = matches!(event, UiEvent::PointerMove(_) | UiEvent::PointerLeave | UiEvent::PointerDown(_) | UiEvent::PointerDownModified { .. } | UiEvent::PointerDownEditor { .. } | UiEvent::PointerUp(_));
        if pointer && suppress.pop_front() == Some(true) {
            // Preserve the emulated pointer event's place for hosts that also pan touches.
            events.push(UiEvent::PointerLeave);
        } else {
            events.push(event);
        }
    }
    if cancel_press { events.push(UiEvent::PointerLeave); }
    if let Some(touch) = hold.touch.as_mut().filter(|touch| !touch.cancelled && !touch.consumed) {
        if frame.context_menu_at(relative(origin, touch.position)) != touch.context.as_deref() {
            // Its owner disappeared or a floating surface covered it during the hold.
            touch.cancelled = true;
        } else if touch.since + HOLD_SECONDS <= now {
            touch.consumed = true;
            events.push(UiEvent::PointerLeave);
            events.push(UiEvent::ContextMenu { position: relative(origin, touch.position) });
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(touch.since + HOLD_SECONDS - now));
        }
    }
    ctx.data_mut(|data| data.insert_temp(key, hold));
    events
}
