use super::*;
use limn::{Component, ComponentPalette, ContextMenu, Element, Size, UiAction, UiTree};

fn touch(phase: egui::TouchPhase, x: f32) -> egui::Event {
    egui::Event::Touch { device_id: egui::TouchDeviceId(1), id: egui::TouchId(1), phase, pos: egui::pos2(x, 20.0), force: None }
}
fn button(pressed: bool) -> egui::Event {
    egui::Event::PointerButton { pos: egui::pos2(20.0, 20.0), button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }
}
fn frame(ctx: &egui::Context, time: f64, raw: Vec<egui::Event>) -> Vec<UiEvent> {
    let scene = UiTree::new(ContextMenu::new("touch-target", Element::button("row", "Row"), Vec::new(), ComponentPalette::neutral_dark()).open_action("more").render()).layout(Size::new(320.0, 200.0));
    frame_in(ctx, time, raw, &scene)
}
fn frame_in(ctx: &egui::Context, time: f64, raw: Vec<egui::Event>, scene: &Frame) -> Vec<UiEvent> {
    let mut translated = Vec::new();
    let mut output = ctx.run_ui(egui::RawInput {
        time: Some(time), events: raw,
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(320.0, 200.0))),
        ..Default::default()
    }, |_| translated = events_with_touch(ctx, egui::Pos2::ZERO, scene));
    output.textures_delta.clear();
    translated
}
fn start() -> Vec<egui::Event> {
    vec![touch(egui::TouchPhase::Start, 20.0), egui::Event::PointerMoved(egui::pos2(20.0, 20.0)), button(true)]
}
fn context(events: &[UiEvent]) -> bool {
    events.iter().any(|event| matches!(event, UiEvent::ContextMenu { .. }))
}

#[test]
fn stationary_touch_opens_the_actual_controlled_context_once_and_release_never_plays() {
    let ctx = egui::Context::default();
    let mut tree = UiTree::new(ContextMenu::new("context:song", Element::button("play:song", "Song"), Vec::new(), ComponentPalette::neutral_dark()).open_action("more:song").render());
    let scene = tree.layout(Size::new(320.0, 200.0));
    let frame = |ctx: &egui::Context, time, raw| frame_in(ctx, time, raw, &scene);
    for event in frame(&ctx, 0.0, start()) { assert!(tree.handle_event(event).is_empty()); }
    assert!(!context(&frame(&ctx, 0.429, Vec::new())));
    let held = frame(&ctx, 0.430, Vec::new());
    assert!(context(&held));
    let actions: Vec<_> = held.into_iter().flat_map(|event| tree.handle_event(event)).collect();
    assert_eq!(actions, vec![UiAction::Clicked { id: "more:song".into() }]);
    assert!(!tree.context_menu_open());
    assert!(!context(&frame(&ctx, 0.8, Vec::new())));
    let release = frame(&ctx, 0.9, vec![touch(egui::TouchPhase::End, 20.0), button(false), egui::Event::PointerGone]);
    assert!(!release.iter().any(|event| matches!(event, UiEvent::PointerUp(_))));
    assert!(release.into_iter().flat_map(|event| tree.handle_event(event)).collect::<Vec<_>>().is_empty());
}

#[test]
fn movement_end_cancel_and_scrolling_cancel_a_pending_hold() {
    for cancel in [
        vec![touch(egui::TouchPhase::Move, 28.0), egui::Event::PointerMoved(egui::pos2(28.0, 20.0))],
        vec![touch(egui::TouchPhase::End, 20.0), button(false)],
        vec![touch(egui::TouchPhase::Cancel, 20.0)],
        vec![egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, 40.0), phase: egui::TouchPhase::Move, modifiers: Default::default() }],
        vec![egui::Event::WindowFocused(false)],
    ] {
        let ctx = egui::Context::default();
        frame(&ctx, 0.0, start());
        let events = frame(&ctx, 0.2, cancel);
        assert!(!context(&events));
        frame(&ctx, 0.3, vec![touch(egui::TouchPhase::Move, 20.0)]);
        assert!(!context(&frame(&ctx, 0.9, Vec::new())));
    }
}

#[test]
fn a_mouse_hold_stays_a_primary_click_and_secondary_click_stays_contextual() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0, vec![button(true)]);
    assert!(!context(&frame(&ctx, 0.8, Vec::new())));
    assert!(frame(&ctx, 0.9, vec![button(false)]).iter().any(|event| matches!(event, UiEvent::PointerUp(_))));
    let secondary = egui::Event::PointerButton { pos: egui::pos2(20.0, 20.0), button: egui::PointerButton::Secondary, pressed: true, modifiers: Default::default() };
    assert!(context(&frame(&ctx, 1.0, vec![secondary])));
}

#[test]
fn a_short_touch_taps_but_a_cancelled_touch_cannot_activate_on_a_later_release() {
    for phase in [egui::TouchPhase::End, egui::TouchPhase::Cancel] {
        let ctx = egui::Context::default();
        let mut tree = UiTree::new(Element::button("play", "Play"));
        tree.layout(Size::new(320.0, 200.0));
        for event in frame(&ctx, 0.0, start()) { tree.handle_event(event); }
        let mut actions = Vec::new();
        for (time, raw) in [(0.2, vec![touch(phase, 20.0)]), (0.3, vec![button(false)])] {
            actions.extend(frame(&ctx, time, raw).into_iter().flat_map(|event| tree.handle_event(event)));
        }
        let expected = if phase == egui::TouchPhase::End { vec![UiAction::Clicked { id: "play".into() }] } else { Vec::new() };
        assert_eq!(actions, expected);
    }
}

#[test]
fn a_touch_without_a_context_target_keeps_its_control_release() {
    let scene = UiTree::new(Element::button("seek", "Seek")).layout(Size::new(320.0, 200.0));
    let ctx = egui::Context::default();
    frame_in(&ctx, 0.0, start(), &scene);
    assert!(!context(&frame_in(&ctx, 0.8, Vec::new(), &scene)));
    assert!(frame_in(&ctx, 0.9, vec![touch(egui::TouchPhase::End, 20.0), button(false)], &scene).iter().any(|event| matches!(event, UiEvent::PointerUp(_))));
}

#[test]
fn replacing_the_context_owner_during_a_hold_cancels_it() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0, start());
    let replacement = UiTree::new(ContextMenu::new("replacement", Element::button("row", "Row"), Vec::new(), ComponentPalette::neutral_dark()).open_action("other").render()).layout(Size::new(320.0, 200.0));
    assert!(!context(&frame_in(&ctx, 0.43, Vec::new(), &replacement)));
}

#[test]
fn a_cancel_without_a_native_release_does_not_swallow_the_next_mouse_click() {
    let ctx = egui::Context::default();
    frame(&ctx, 0.0, start());
    frame(&ctx, 0.2, vec![touch(egui::TouchPhase::Cancel, 20.0)]);
    assert!(frame(&ctx, 0.9, vec![button(true)]).iter().any(|event| matches!(event, UiEvent::PointerDown(_))));
    assert!(frame(&ctx, 1.0, vec![button(false)]).iter().any(|event| matches!(event, UiEvent::PointerUp(_))));
}
