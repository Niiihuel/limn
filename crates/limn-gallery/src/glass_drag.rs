//! Gallery-only positioning. Layout and pointer input share logical coordinates.

use crate::app::State;
use limn::{DrawCommand, Frame, NavigationBar, Point, Rect, UiAction};

pub const HANDLE: &str = "glass-drag-handle";
pub const PREVIEW_HEIGHT: f32 = 340.0;
pub const HANDLE_HEIGHT: f32 = 28.0;
pub const HANDLE_GAP: f32 = 8.0;
const INSET: f32 = 16.0;
const FOOTER: f32 = 36.0;
pub const DEFAULT_X: f32 = 0.5;
pub const DEFAULT_Y: f32 = 0.25;

pub fn fraction(state: &State, key: &str, default: f32) -> f32 {
    state
        .text(key)
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .unwrap_or(default)
        .clamp(0.0, 1.0)
}

pub struct Geometry {
    pub bar: Rect,
    origin: Point,
    travel: Point,
}

/// Both the page and drag handler derive geometry from the same preview box.
/// No framebuffer pixels or paint-only translations enter the pointer path.
pub fn geometry(state: &State, preview: Rect) -> Geometry {
    let inset = INSET.min(preview.width / 2.0);
    let width = (preview.width - 64.0)
        .max(0.0)
        .min(if state.flag("glass-compact") {
            360.0
        } else {
            860.0
        });
    let origin = Point::new(preview.x + inset, preview.y + INSET);
    let travel = Point::new(
        (preview.width - inset * 2.0 - width).max(0.0),
        (preview.height - INSET - FOOTER - NavigationBar::HEIGHT - HANDLE_GAP - HANDLE_HEIGHT)
            .max(0.0),
    );
    Geometry {
        bar: Rect::new(
            origin.x + travel.x * fraction(state, "glass-x", DEFAULT_X),
            origin.y + travel.y * fraction(state, "glass-y", DEFAULT_Y),
            width,
            NavigationBar::HEIGHT,
        ),
        origin,
        travel,
    }
}

fn backdrop(frame: &Frame) -> Option<Rect> {
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::Surface { rect, id, .. } if id == crate::pages::glass::BACKDROP => Some(*rect),
        _ => None,
    })
}

fn move_to(state: &mut State, position: Point, frame: &Frame) {
    if !state.flag("glass-dragging") || !position.x.is_finite() || !position.y.is_finite() {
        return;
    }
    let Some(preview) = backdrop(frame) else {
        return;
    };
    let geometry = geometry(state, preview);
    let grab = |key: &str| {
        state
            .text(key)
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    };
    let x = position.x - grab("glass-grab-x") - geometry.origin.x;
    let y = position.y - grab("glass-grab-y") - geometry.origin.y;
    if geometry.travel.x > 0.0 {
        state.set(
            "glass-x",
            &(x / geometry.travel.x).clamp(0.0, 1.0).to_string(),
        );
    }
    if geometry.travel.y > 0.0 {
        state.set(
            "glass-y",
            &(y / geometry.travel.y).clamp(0.0, 1.0).to_string(),
        );
    }
}

/// Consume only this demo's drag handle. Other controls follow the normal
/// gallery update path, including navigation buttons and position sliders.
pub fn handle(state: &mut State, action: &UiAction, frame: &Frame) -> bool {
    match action {
        UiAction::DragStarted { id, position } if id == HANDLE => {
            if let Some(preview) =
                backdrop(frame).filter(|_| position.x.is_finite() && position.y.is_finite())
            {
                let bar = geometry(state, preview).bar;
                state.set("glass-grab-x", &(position.x - bar.x).to_string());
                state.set("glass-grab-y", &(position.y - bar.y).to_string());
                state.set(
                    "glass-start-x",
                    &fraction(state, "glass-x", DEFAULT_X).to_string(),
                );
                state.set(
                    "glass-start-y",
                    &fraction(state, "glass-y", DEFAULT_Y).to_string(),
                );
                state.set("glass-dragging", "on");
            }
        }
        UiAction::DragMoved { id, position, .. } if id == HANDLE => {
            move_to(state, *position, frame)
        }
        UiAction::Dropped { id, position, .. } if id == HANDLE => {
            move_to(state, *position, frame);
            state.set("glass-dragging", "off");
        }
        UiAction::DragCancelled { id } if id == HANDLE => {
            if state.flag("glass-dragging") {
                state.set(
                    "glass-x",
                    &fraction(state, "glass-start-x", DEFAULT_X).to_string(),
                );
                state.set(
                    "glass-y",
                    &fraction(state, "glass-start-y", DEFAULT_Y).to_string(),
                );
            }
            state.set("glass-dragging", "off");
        }
        UiAction::Clicked { id } if id == HANDLE => {}
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app,
        pages::{self, Cx},
    };
    use limn::{ComponentPalette, Element, Key, Size, UiEvent, UiTree};

    fn view(state: &State, width: f32) -> Element {
        let cx = Cx {
            state,
            palette: ComponentPalette::neutral_dark(),
            width: width - 40.0,
        };
        Element::panel(Element::column(vec![
            pages::find("glass").render(&cx),
            Element::spacer().flex(1.0),
        ]))
        .padding(20.0)
    }

    fn tree(state: &State, size: Size) -> UiTree {
        let mut tree = UiTree::new(view(state, size.width));
        tree.set_reduced_motion(true);
        tree.layout(size);
        tree
    }

    fn region(tree: &UiTree, id: &str) -> Rect {
        tree.frame()
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap()
            .rect
    }

    fn event(tree: &mut UiTree, state: &mut State, size: Size, event: UiEvent) -> Vec<UiAction> {
        let actions = tree.handle_event(event);
        for action in &actions {
            if !handle(state, action, tree.frame()) {
                app::update(state, action);
            }
        }
        tree.set_root(view(state, size.width));
        tree.layout(size);
        actions
    }

    #[test]
    fn dragging_relayouts_bar_and_its_buttons_without_activating_them() {
        let size = Size::new(1200.0, 1800.0);
        let mut state = State::default();
        let mut tree = tree(&state, size);
        let before = region(&tree, "glass-nav:workspace:glass-workspace=1");
        let handle = region(&tree, HANDLE);
        let down = Point::new(handle.x + 23.0, handle.y + 12.0);
        event(&mut tree, &mut state, size, UiEvent::PointerDown(down));
        let to = Point::new(down.x + 50.0, down.y + 70.0);
        let actions = event(&mut tree, &mut state, size, UiEvent::PointerMove(to));
        assert!(matches!(
            actions.as_slice(),
            [UiAction::DragStarted { .. }, UiAction::DragMoved { .. }]
        ));
        let moved = region(&tree, "glass-nav:workspace:glass-workspace=1");
        assert!((moved.x - before.x - 50.0).abs() < 0.01);
        assert!((moved.y - before.y - 70.0).abs() < 0.01);
        event(&mut tree, &mut state, size, UiEvent::PointerUp(to));
        assert_eq!(state.text("glass-workspace"), "");
        let point = Point::new(moved.x + 12.0, moved.y + 20.0);
        event(&mut tree, &mut state, size, UiEvent::PointerDown(point));
        event(&mut tree, &mut state, size, UiEvent::PointerUp(point));
        assert_eq!(state.text("glass-workspace"), "1");
    }

    #[test]
    fn bounds_survive_resize_and_invalid_coordinates_and_escape_restores_position() {
        let size = Size::new(1200.0, 1800.0);
        let mut state = State::default();
        let mut tree = tree(&state, size);
        let rect = region(&tree, HANDLE);
        let down = Point::new(rect.x + 12.0, rect.y + 12.0);
        event(&mut tree, &mut state, size, UiEvent::PointerDown(down));
        event(
            &mut tree,
            &mut state,
            size,
            UiEvent::PointerMove(Point::new(10000.0, 10000.0)),
        );
        assert_eq!(state.text("glass-x"), "1");
        assert_eq!(state.text("glass-y"), "1");
        event(
            &mut tree,
            &mut state,
            size,
            UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            },
        );
        assert_eq!(fraction(&state, "glass-x", DEFAULT_X), DEFAULT_X);
        assert_eq!(fraction(&state, "glass-y", DEFAULT_Y), DEFAULT_Y);
        state.set("glass-x", "NaN");
        state.set("glass-y", "Infinity");
        for width in [320.0, 600.0, 1200.0] {
            for compact in ["on", "off"] {
                state.set("glass-compact", compact);
                let preview = Rect::new(37.0, 89.0, width, PREVIEW_HEIGHT);
                let geometry = geometry(&state, preview);
                assert!(geometry.bar.x >= preview.x && geometry.bar.y >= preview.y);
                assert!(geometry.bar.x + geometry.bar.width <= preview.x + width);
                assert!(
                    geometry.bar.y + NavigationBar::HEIGHT + HANDLE_GAP + HANDLE_HEIGHT
                        <= preview.y + preview.height
                );
            }
        }
    }

    #[test]
    fn position_sliders_and_reset_are_keyboard_operable() {
        let size = Size::new(1200.0, 1800.0);
        let mut state = State::default();
        let mut tree = tree(&state, size);
        for id in ["glass-x", "glass-y"] {
            assert!(tree.focus(id));
            event(
                &mut tree,
                &mut state,
                size,
                UiEvent::Key {
                    key: Key::End,
                    shift: false,
                    ctrl: false,
                },
            );
            assert_eq!(fraction(&state, id, 0.0), 1.0);
        }
        assert!(tree.focus("glass-x=0.5&glass-y=0.25"));
        event(
            &mut tree,
            &mut state,
            size,
            UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
            },
        );
        assert_eq!(fraction(&state, "glass-x", 0.0), DEFAULT_X);
        assert_eq!(fraction(&state, "glass-y", 0.0), DEFAULT_Y);
    }

    #[test]
    fn relayout_after_resize_keeps_bar_and_handle_inside_the_preview() {
        let mut state = State::default();
        let mut tree = tree(&state, Size::new(1200.0, 1800.0));
        for width in [360.0, 900.0, 1600.0] {
            for compact in ["on", "off"] {
                state.set("glass-compact", compact);
                for (x, y) in [("0", "0"), ("1", "1"), ("0.5", "0.25")] {
                    state.set("glass-x", x);
                    state.set("glass-y", y);
                    let size = Size::new(width, 1800.0);
                    tree.set_root(view(&state, width));
                    let frame = tree.layout(size);
                    let preview = backdrop(&frame).unwrap();
                    let expected = geometry(&state, preview).bar;
                    let bar = frame
                        .commands
                        .iter()
                        .find_map(|command| match command {
                            DrawCommand::Glass { rect, .. } => Some(*rect),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(bar, expected);
                    let handle = region(&tree, HANDLE);
                    assert!(
                        handle.x >= preview.x
                            && handle.x + handle.width <= preview.x + preview.width
                    );
                    assert!(
                        handle.y >= preview.y
                            && handle.y + handle.height <= preview.y + preview.height
                    );
                    for target in frame
                        .regions
                        .iter()
                        .filter(|r| r.id.starts_with("glass-nav:"))
                    {
                        assert!(
                            target.rect.x >= preview.x
                                && target.rect.x + target.rect.width <= preview.x + preview.width
                        );
                        assert!(
                            target.rect.y >= preview.y
                                && target.rect.y + target.rect.height <= preview.y + preview.height
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn animation_defaults_on_and_keyboard_toggle_keeps_focus() {
        let size = Size::new(1200.0, 1800.0);
        let mut state = State::default();
        let mut tree = tree(&state, size);
        assert!(tree.focus("glass-animate"));
        for expected in ["off", "on"] {
            event(
                &mut tree,
                &mut state,
                size,
                UiEvent::Key {
                    key: Key::Space,
                    shift: false,
                    ctrl: false,
                },
            );
            assert_eq!(state.choice("glass-animate", "on"), expected);
            assert_eq!(tree.focused_id(), Some("glass-animate"));
        }
    }
}
