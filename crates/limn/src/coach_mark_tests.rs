//! A coach mark is anchored to controls it does not wrap, by their IDs.
use super::*;
use crate::{CoachMark, Component, ComponentPalette, Tour, TourStep};

const VIEWPORT: Size = Size::new(800.0, 600.0);
const RADIUS: f32 = 5.0;

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

fn button(id: &str) -> Element {
    Element::button(id, id).size(Some(80.0), Some(32.0)).radius(RADIUS)
}

/// A button in each corner around a list that scrolls.
fn page() -> Element {
    let edge = |left, right| {
        Element::row(vec![button(left), Element::spacer().flex(1.0), button(right)])
            .size(None, Some(32.0))
    };
    Element::column(vec![
        edge("top-left", "top-right"),
        Element::scroll(
            "list",
            Element::column((0..40).map(|index| button(&format!("row:{index}"))).collect()),
        )
        .flex(1.0),
        edge("bottom-left", "bottom-right"),
    ])
}

fn mark(target: &str) -> CoachMark {
    CoachMark::new(page(), "tour", target, palette())
        .title("Tareas")
        .body("Cada tarea es una conversación con un agente.")
}

fn settled(root: Element) -> (UiTree, Frame) {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    let frame = tree.layout(VIEWPORT);
    (tree, frame)
}

fn region(frame: &Frame, id: &str) -> Rect {
    frame
        .regions
        .iter()
        .find(|region| region.id == id)
        .unwrap_or_else(|| panic!("no region {id}"))
        .rect
}

fn bubble(frame: &Frame) -> Rect {
    region(frame, "popover_surface:tour")
}

fn ring(frame: &Frame) -> (Rect, f32, Color) {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::StrokeRect { rect, radius, color, width } if *width == COACH_RING_WIDTH => {
                Some((*rect, *radius, *color))
            }
            _ => None,
        })
        .expect("a ring")
}

fn caret(frame: &Frame) -> Option<[Point; 3]> {
    let fill = ring(frame).2;
    frame.commands.iter().rev().find_map(|command| match command {
        DrawCommand::FillTriangle { points, color } if *color == fill => Some(*points),
        _ => None,
    })
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, shift: false, ctrl: false }
}

fn click(tree: &mut UiTree, rect: Rect) -> Vec<UiAction> {
    let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    tree.handle_event(UiEvent::PointerDown(point));
    tree.handle_event(UiEvent::PointerUp(point))
}

fn inside(inner: Rect, outer: Size) -> bool {
    inner.x >= 0.0
        && inner.y >= 0.0
        && inner.x + inner.width <= outer.width
        && inner.y + inner.height <= outer.height
}

#[test]
fn bubble_takes_the_preferred_side_and_another_one_at_a_viewport_edge() {
    use CoachAlign::*;
    use CoachSide::*;
    for (target, side, align, expected) in [
        ("top-left", Below, Start, Below),
        ("bottom-left", Below, Start, Above),
        ("top-left", Above, Center, Below),
        ("top-right", Right, Start, Left),
        ("top-left", Left, End, Right),
        ("bottom-right", Right, End, Left),
    ] {
        let (_, frame) = settled(mark(target).placement(side, align).render());
        let (target, bubble) = (region(&frame, target), bubble(&frame));
        let gap = COACH_RING_GAP + COACH_RING_WIDTH + COACH_CARET;
        let found = if bubble.y == target.y + target.height + gap {
            Below
        } else if bubble.y + bubble.height + gap == target.y {
            Above
        } else if bubble.x == target.x + target.width + gap {
            Right
        } else {
            assert_eq!(bubble.x + bubble.width + gap, target.x);
            Left
        };
        assert_eq!(found, expected, "{target:?} {side:?}");
        assert!(inside(bubble, VIEWPORT), "{bubble:?}");
        assert!(bubble.intersection(target).is_none());
    }
}

#[test]
fn alignment_runs_along_the_target_edge_and_is_shifted_into_the_viewport() {
    let width = |align| {
        let (_, frame) = settled(
            mark("row:3").placement(CoachSide::Right, align).render(),
        );
        (region(&frame, "row:3"), bubble(&frame))
    };
    let (target, start) = width(CoachAlign::Start);
    assert_eq!(start.y, target.y);
    let (target, center) = width(CoachAlign::Center);
    assert_eq!(center.y + center.height / 2.0, target.y + target.height / 2.0);
    let (target, end) = width(CoachAlign::End);
    assert_eq!(end.y + end.height, target.y + target.height);
    // Ending at the right edge of a button in the left corner would leave
    // the window: the bubble is moved in and still points at the button.
    let (_, frame) = settled(mark("top-left").placement(CoachSide::Below, CoachAlign::End).render());
    assert_eq!(bubble(&frame).x, 8.0);
    let target = region(&frame, "top-left");
    assert_eq!(caret(&frame).unwrap()[2].x, target.x + target.width / 2.0);
}

#[test]
fn caret_points_at_the_middle_of_the_target_from_the_bubble_edge() {
    let (_, frame) = settled(mark("top-right").placement(CoachSide::Below, CoachAlign::End).render());
    let (target, bubble) = (region(&frame, "top-right"), bubble(&frame));
    let [a, b, tip] = caret(&frame).unwrap();
    assert_eq!(tip.x, target.x + target.width / 2.0);
    assert_eq!(tip.y, bubble.y - COACH_CARET);
    // Its base is inside the bubble, so no seam shows at the join.
    assert!(a.y > bubble.y && b.y > bubble.y && a.x < tip.x && b.x > tip.x);

    let (_, frame) = settled(mark("row:2").placement(CoachSide::Right, CoachAlign::Center).render());
    let (target, bubble) = (region(&frame, "row:2"), self::bubble(&frame));
    let tip = caret(&frame).unwrap()[2];
    assert_eq!(tip.y, target.y + target.height / 2.0);
    assert_eq!(tip.x, bubble.x - COACH_CARET);
}

#[test]
fn a_window_with_room_on_no_side_keeps_the_bubble_whole_and_drops_the_caret() {
    let mut tree = UiTree::new(mark("top-left").width(240.0).render());
    tree.set_reduced_motion(true);
    let small = Size::new(250.0, 120.0);
    let frame = tree.layout(small);
    assert!(inside(bubble(&frame), small));
    assert!(caret(&frame).is_none());
    assert!(frame.regions.iter().any(|region| region.id == "tour:close"));
}

#[test]
fn ring_follows_the_target_rect_and_radius_in_the_bubble_fill() {
    let fill = Color::rgb(20, 90, 200);
    let (_, frame) = settled(mark("top-right").accent(fill, Color::WHITE).render());
    let target = region(&frame, "top-right");
    let (rect, radius, color) = ring(&frame);
    assert_eq!(
        rect,
        Rect::new(
            target.x - COACH_RING_GAP,
            target.y - COACH_RING_GAP,
            target.width + COACH_RING_GAP * 2.0,
            target.height + COACH_RING_GAP * 2.0
        )
    );
    assert_eq!(radius, RADIUS + COACH_RING_GAP);
    assert_eq!(color, fill);
    let bubble = bubble(&frame);
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. } if *rect == bubble && *color == fill
    )));
    // Without an accent the bubble takes the palette's solid role.
    let (_, frame) = settled(mark("top-right").render());
    assert_eq!(ring(&frame).2, palette().text);
    // An ink with transparency still makes a bubble that hides the page.
    let veiled = ComponentPalette { text: Color::rgba(255, 255, 255, 128), ..palette() };
    let (_, frame) = settled(CoachMark::new(page(), "tour", "top-right", veiled).render());
    assert_eq!(ring(&frame).2.a, 255);
    assert_ne!(ring(&frame).2, Color::WHITE);
}

#[test]
fn several_targets_are_ringed_as_one_area() {
    let (_, frame) = settled(mark("top-left").also("top-right").also("absent").render());
    let (left, right) = (region(&frame, "top-left"), region(&frame, "top-right"));
    let rect = ring(&frame).0;
    assert_eq!(rect.x, left.x - COACH_RING_GAP);
    assert_eq!(rect.x + rect.width, right.x + right.width + COACH_RING_GAP);
}

#[test]
fn dimming_covers_the_window_except_the_target() {
    let dim = Color::rgba(0, 0, 0, 90);
    let (_, frame) = settled(mark("row:4").dim(dim).render());
    let hole = ring(&frame).0;
    let dimmed: Vec<Rect> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == dim => Some(*rect),
            _ => None,
        })
        .collect();
    assert!(dimmed.iter().all(|rect| rect.intersection(hole).is_none()));
    let area: f32 = dimmed.iter().map(|rect| rect.width * rect.height).sum();
    let hole = hole.intersection(Rect::new(0.0, 0.0, VIEWPORT.width, VIEWPORT.height)).unwrap();
    assert_eq!(area, VIEWPORT.width * VIEWPORT.height - hole.width * hole.height);
}

#[test]
fn a_missing_target_paints_nothing_is_reported_and_leaves_the_window_usable() {
    let (_, plain) = settled(page());
    let (mut tree, frame) = settled(mark("nowhere").render());
    assert_eq!(frame.missing_coach_target(), Some("nowhere"));
    assert_eq!(frame.commands, plain.commands);
    assert_eq!(frame.regions, plain.regions);
    assert_eq!(
        click(&mut tree, region(&frame, "top-left")),
        vec![UiAction::Clicked { id: "top-left".into() }]
    );
    let (_, frame) = settled(mark("top-left").render());
    assert_eq!(frame.missing_coach_target(), None);
    assert_eq!(plain.missing_coach_target(), None);
}

#[test]
fn a_target_scrolled_out_of_view_is_revealed_before_the_bubble_is_placed() {
    let (_, plain) = settled(page());
    assert!(!plain.regions.iter().any(|region| region.id == "row:30"));
    let (_, frame) = settled(mark("row:30").placement(CoachSide::Right, CoachAlign::Center).render());
    assert_eq!(frame.missing_coach_target(), None);
    let (target, list) = (region(&frame, "row:30"), region(&frame, "list"));
    assert_eq!(target.height, 32.0);
    assert!(target.y >= list.y && target.y + target.height <= list.y + list.height);
    assert_eq!(ring(&frame).0.y, target.y - COACH_RING_GAP);
}

#[test]
fn the_window_behind_takes_no_input_and_only_the_bubble_ends_the_step() {
    let (mut tree, frame) = settled(mark("top-left").step(1, 3).skip("Omitir recorrido").render());
    assert!(tree.has_open_popover());
    // A stray click on a control behind, or on nothing, changes nothing.
    assert!(click(&mut tree, region(&frame, "bottom-right")).is_empty());
    assert!(click(&mut tree, Rect::new(400.0, 300.0, 2.0, 2.0)).is_empty());
    assert!(tree.layout(VIEWPORT).regions.iter().any(|region| region.id == "tour:next"));
    for (control, action) in [
        ("tour:close", "tour"),
        ("tour:next", "tour:next"),
        ("tour:back", "tour:back"),
        ("tour:skip", "tour:skip"),
    ] {
        let frame = tree.layout(VIEWPORT);
        let rect = frame
            .regions
            .iter()
            .rev()
            .find(|region| region.id == control && region.kind == HitKind::Button)
            .unwrap()
            .rect;
        assert_eq!(click(&mut tree, rect), vec![UiAction::Clicked { id: action.into() }]);
    }
}

#[test]
fn escape_closes_arrows_walk_the_steps_and_enter_takes_the_primary_action() {
    let clicked = |id: &str| vec![UiAction::Clicked { id: id.into() }];
    let (mut tree, _) = settled(mark("top-left").step(0, 3).render());
    assert_eq!(tree.handle_event(key(Key::Enter)), clicked("tour:next"));
    assert_eq!(tree.handle_event(key(Key::ArrowRight)), clicked("tour:next"));
    // The first step has nowhere to go back to.
    assert!(tree.handle_event(key(Key::ArrowLeft)).is_empty());
    assert_eq!(tree.handle_event(key(Key::Escape)), clicked("tour"));

    let (mut tree, _) = settled(mark("top-left").step(2, 3).render());
    assert_eq!(tree.handle_event(key(Key::ArrowLeft)), clicked("tour:back"));
    assert_eq!(tree.handle_event(key(Key::ArrowRight)), clicked("tour:next"));
    // Tab stays among the bubble's own controls.
    for _ in 0..6 {
        tree.handle_event(key(Key::Tab));
        tree.layout(VIEWPORT);
        assert!(tree.focused_id().is_some_and(|id| id.starts_with("tour:")));
    }
}

#[test]
fn focus_goes_to_the_primary_action_on_each_step_and_returns_when_the_tour_ends() {
    let mut tree = UiTree::new(page());
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("bottom-left"));
    tree.set_root(mark("top-left").step(1, 3).render());
    let frame = tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("tour:next"));
    // Going back with the pointer leaves focus on a button the first step
    // does not have.
    click(&mut tree, region(&frame, "tour:back"));
    assert_eq!(tree.focused_id(), Some("tour:back"));
    tree.set_root(mark("top-right").step(0, 3).render());
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("tour:next"));
    // The same step laid out again leaves the user's focus alone.
    tree.handle_event(key(Key::Tab));
    let moved = tree.focused_id().map(str::to_owned);
    assert_ne!(moved.as_deref(), Some("tour:next"));
    tree.set_root(mark("top-right").step(0, 3).render());
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), moved.as_deref());
    tree.set_root(page());
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("bottom-left"));
}

#[test]
fn a_new_step_travels_from_the_last_target_while_input_is_already_at_the_new_one() {
    let start = Instant::now();
    let mut tree = UiTree::new(mark("top-left").step(0, 2).render());
    tree.layout_at(VIEWPORT, start);
    let open = start + Duration::from_secs(1);
    let first = tree.layout_at(VIEWPORT, open);
    let (from_ring, from_bubble) = (ring(&first).0, bubble(&first));
    assert!(tree.visual_animation_repaint_after_at(open).is_none());

    tree.set_root(mark("bottom-right").step(1, 2).render());
    let leaving = tree.layout_at(VIEWPORT, open);
    let to_bubble = bubble(&leaving);
    assert_ne!(to_bubble, from_bubble);
    // Paint starts where the last step was; the buttons are at the new one.
    assert_eq!(ring(&leaving).0, from_ring);
    assert!(region(&leaving, "tour:next").intersection(to_bubble).is_some());
    assert_eq!(tree.visual_animation_repaint_after_at(open), Some(Duration::ZERO));

    let midway = tree.layout_at(VIEWPORT, open + Duration::from_millis(60));
    let (between, target) = (ring(&midway).0, region(&midway, "bottom-right"));
    assert!(between.x > from_ring.x && between.x < target.x - COACH_RING_GAP);
    assert!(between.y > from_ring.y && between.y < target.y - COACH_RING_GAP);
    let painted = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Shadow { rect, .. } if rect.width == to_bubble.width => Some(*rect),
                _ => None,
            })
            .unwrap()
    };
    let travelling = painted(&midway);
    assert!(travelling.x > from_bubble.x && travelling.x < to_bubble.x);
    assert_eq!(bubble(&midway), to_bubble);

    let rest = open + Duration::from_secs(2);
    let arrived = tree.layout_at(VIEWPORT, rest);
    assert_eq!(ring(&arrived).0.x, target.x - COACH_RING_GAP);
    assert_eq!(painted(&arrived), to_bubble);
    // Nothing left to animate: an idle tour asks for no frames.
    assert!(tree.visual_animation_repaint_after_at(rest).is_none());
}

#[test]
fn reduced_motion_puts_a_new_step_in_place_at_once() {
    let (mut tree, _) = settled(mark("top-left").step(0, 2).render());
    tree.set_root(mark("bottom-right").step(1, 2).render());
    let now = Instant::now();
    let frame = tree.layout_at(VIEWPORT, now);
    let target = region(&frame, "bottom-right");
    assert_eq!(ring(&frame).0.x, target.x - COACH_RING_GAP);
    assert!(tree.visual_animation_repaint_after_at(now).is_none());
    assert!(tree.coach_motion.as_ref().is_some_and(|motion| {
        motion.values.iter().all(|value| !value.is_active(now))
    }));
}

#[test]
fn a_resize_moves_the_ring_with_its_target_without_a_trail() {
    let start = Instant::now();
    let mut tree = UiTree::new(mark("top-right").render());
    tree.layout_at(VIEWPORT, start);
    let later = start + Duration::from_secs(1);
    let frame = tree.layout_at(Size::new(600.0, 600.0), later);
    assert_eq!(ring(&frame).0.x, region(&frame, "top-right").x - COACH_RING_GAP);
    assert!(tree.visual_animation_repaint_after_at(later).is_none());
}

#[test]
fn tour_counts_its_steps_and_names_the_last_action() {
    let steps = || {
        vec![
            TourStep::new("top-left", "Uno", "Primero."),
            TourStep::new("row:1", "Dos", "Segundo.").placement(CoachSide::Right, CoachAlign::Start),
            TourStep::new("bottom-right", "Tres", "Tercero.").also("bottom-left"),
        ]
    };
    let texts = |current| {
        let (_, frame) = settled(Tour::new(page(), "tour", steps(), current, palette()).render());
        let texts: Vec<String> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect();
        (frame, texts)
    };
    let has = |texts: &[String], value: &str| texts.iter().any(|text| text == value);
    let (frame, first) = texts(Some(0));
    assert!(has(&first, "1 de 3") && has(&first, "Siguiente") && has(&first, "Uno"));
    assert!(!has(&first, "Atrás"));
    assert!(!frame.regions.iter().any(|region| region.id == "tour:back"));
    let (_, second) = texts(Some(1));
    assert!(has(&second, "2 de 3") && has(&second, "Atrás") && has(&second, "Siguiente"));
    let (_, last) = texts(Some(2));
    assert!(has(&last, "3 de 3") && has(&last, "Entendido") && !has(&last, "Siguiente"));
    // Over, or not started: the page alone.
    let (_, plain) = settled(page());
    for current in [None, Some(3)] {
        assert_eq!(texts(current).0.commands, plain.commands);
    }
}

#[test]
fn labels_are_the_hosts_and_a_long_body_wraps_inside_the_bubble() {
    let body = "Una explicación bastante más larga de lo habitual, para comprobar que el \
                texto baja de línea dentro de la burbuja en lugar de cortarse con puntos.";
    let (_, frame) = settled(
        mark("top-left")
            .body(body)
            .step(0, 2)
            .next_label("Next")
            .counter("Step 1")
            .skip("Skip")
            .width(f32::NAN)
            .render(),
    );
    let bubble = bubble(&frame);
    assert_eq!(bubble.width, 320.0);
    let mut lines = 0;
    for command in &frame.commands {
        if let DrawCommand::Text { text, position, .. } = command {
            if bubble.contains(*position) {
                assert!(!text.contains('…'), "{text}");
                lines += usize::from(body.contains(text.trim()) && !text.trim().is_empty());
            }
        }
    }
    assert!(lines > 1, "the body wraps");
    let texts: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.to_string()),
            _ => None,
        })
        .collect();
    for label in ["Next", "Step 1", "Skip"] {
        assert!(texts.iter().any(|text| text == label), "{label}");
    }
    assert_eq!(CoachMark::new(page(), "t", "x", palette()).width(10.0).render().style.width, None);
}

#[test]
fn a_host_names_the_actions_while_the_controls_keep_their_ids() {
    let clicked = |id: &str| vec![UiAction::Clicked { id: id.into() }];
    let (mut tree, frame) = settled(
        mark("top-left")
            .step(1, 3)
            .skip("Omitir")
            .on_next("step=2")
            .on_back("step=0")
            .on_skip("step=off")
            .render(),
    );
    assert_eq!(tree.focused_id(), Some("tour:next"));
    assert_eq!(tree.handle_event(key(Key::ArrowRight)), clicked("step=2"));
    assert_eq!(tree.handle_event(key(Key::ArrowLeft)), clicked("step=0"));
    assert_eq!(tree.handle_event(key(Key::Enter)), clicked("step=2"));
    assert_eq!(click(&mut tree, region(&frame, "tour:skip")), clicked("step=off"));
}
