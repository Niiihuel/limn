use super::*;

const RED: Color = Color::rgb(255, 0, 0);
const BLUE: Color = Color::rgb(0, 0, 255);
const GREEN: Color = Color::rgb(0, 255, 0);

/// One element of this size alone at the top left of the window.
fn alone(element: Element, width: f32, height: f32) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![
        Element::row(vec![
            element.size(Some(width), Some(height)),
            Element::spacer().flex(1.0),
        ]),
        Element::spacer().flex(1.0),
    ]));
    tree.layout(Size::new(400.0, 300.0));
    tree
}

fn gradients(frame: &Frame) -> Vec<(Rect, Color, Color)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillHorizontalGradient { rect, left, right } => {
                Some((*rect, *left, *right))
            }
            DrawCommand::FillVerticalGradient { rect, top, bottom } => Some((*rect, *top, *bottom)),
            _ => None,
        })
        .collect()
}

fn strips(frame: &Frame) -> Vec<(Rect, Color)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_gradient_paints_one_span_between_each_two_stops_across_the_element() {
    let frame = alone(
        Element::spacer().horizontal_gradient([RED, GREEN, BLUE]),
        200.0,
        10.0,
    )
    .layout(Size::new(400.0, 300.0));
    assert_eq!(
        gradients(&frame),
        [
            (Rect::new(0.0, 0.0, 100.0, 10.0), RED, GREEN),
            (Rect::new(100.0, 0.0, 100.0, 10.0), GREEN, BLUE)
        ]
    );
    let frame = alone(
        Element::spacer().vertical_gradient(vec![RED, BLUE]),
        20.0,
        80.0,
    )
    .layout(Size::new(400.0, 300.0));
    assert_eq!(
        gradients(&frame),
        [(Rect::new(0.0, 0.0, 20.0, 80.0), RED, BLUE)]
    );
}

#[test]
fn one_stop_is_a_plain_fill_and_none_paints_nothing() {
    let frame = alone(Element::spacer().horizontal_gradient([RED]), 50.0, 10.0)
        .layout(Size::new(400.0, 300.0));
    assert_eq!(
        gradients(&frame),
        [(Rect::new(0.0, 0.0, 50.0, 10.0), RED, RED)]
    );
    let frame = alone(
        Element::spacer().horizontal_gradient(Vec::new()),
        50.0,
        10.0,
    )
    .layout(Size::new(400.0, 300.0));
    assert!(gradients(&frame).is_empty() && strips(&frame).is_empty());
}

#[test]
fn a_rounded_gradient_stops_short_of_its_ends_and_narrows_strips_to_the_curve() {
    let frame = alone(
        Element::spacer()
            .horizontal_gradient([RED, BLUE])
            .radius(5.0),
        100.0,
        10.0,
    )
    .layout(Size::new(400.0, 300.0));
    let [(straight, left, right)] = gradients(&frame)[..] else {
        panic!("one span: {:?}", gradients(&frame))
    };
    assert_eq!(straight, Rect::new(5.0, 0.0, 90.0, 10.0));
    // The span starts and ends on the colors of the gradient at those places.
    assert!(left.r > right.r && left.b < right.b && left != RED && right != BLUE);
    let strips = strips(&frame);
    assert_eq!(strips.len(), 10, "five one-pixel strips at each end");
    for (rect, _) in &strips {
        assert_eq!(rect.width, 1.0);
        assert!(rect.y >= 0.0 && rect.y + rect.height <= 10.0);
    }
    // The outermost strips are the shortest, and the colors run on from the ends.
    assert!(strips[0].0.height < strips[8].0.height);
    assert_eq!((strips[0].0.x, strips[1].0.x), (0.0, 99.0));
    assert!(strips[0].1.r > strips[1].1.r);
}

#[test]
fn an_area_slider_reports_both_shares_while_dragged_and_commits_on_release() {
    let mut tree = alone(
        Element::clickable("area", Element::spacer()).area_slider(0.0, 0.0, 10.0),
        120.0,
        70.0,
    );
    // The travel is 100 by 50, ten pixels in from every edge.
    let changed = |actions: Vec<UiAction>| match actions.as_slice() {
        [UiAction::AreaSliderChanged {
            id,
            x,
            y,
            committed,
        }] if id == "area" => (*x, *y, *committed),
        other => panic!("one AreaSliderChanged, got {other:?}"),
    };
    assert_eq!(
        changed(tree.handle_event(UiEvent::PointerDown(Point::new(35.0, 20.0)))),
        (0.25, 0.2, false)
    );
    assert_eq!(
        changed(tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 60.0)))),
        (0.5, 1.0, false)
    );
    // Outside the area both stop at their ends.
    assert_eq!(
        changed(tree.handle_event(UiEvent::PointerUp(Point::new(300.0, -40.0)))),
        (1.0, 0.0, true)
    );
    assert!(tree
        .handle_event(UiEvent::PointerMove(Point::new(40.0, 40.0)))
        .is_empty());
    // Leaving the window during a drag commits where it last was.
    tree.handle_event(UiEvent::PointerDown(Point::new(110.0, 35.0)));
    assert_eq!(
        changed(tree.handle_event(UiEvent::PointerLeave)),
        (1.0, 0.5, true)
    );
}

#[test]
fn an_area_sliders_arrows_move_it_across_and_up_and_down() {
    let mut tree = alone(
        Element::clickable("area", Element::spacer()).area_slider(0.5, 0.5, 0.0),
        100.0,
        100.0,
    );
    assert!(tree.focus("area"));
    let key = |tree: &mut UiTree, key| {
        tree.handle_event(UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        })
    };
    for (pressed, x, y) in [
        (Key::ArrowLeft, 0.45, 0.5),
        (Key::ArrowRight, 0.55, 0.5),
        (Key::ArrowUp, 0.5, 0.45),
        (Key::ArrowDown, 0.5, 0.55),
        (Key::Home, 0.0, 0.5),
        (Key::End, 1.0, 0.5),
    ] {
        match key(&mut tree, pressed).as_slice() {
            [UiAction::AreaSliderChanged {
                x: got_x,
                y: got_y,
                committed: true,
                ..
            }] => {
                assert!(
                    (got_x - x).abs() < 1e-6 && (got_y - y).abs() < 1e-6,
                    "{pressed:?}: {got_x}, {got_y}"
                );
            }
            other => panic!("{pressed:?}: {other:?}"),
        }
    }
    assert!(
        key(&mut tree, Key::Enter).is_empty(),
        "an area slider has no click"
    );
    // At an edge a step goes no further.
    let mut tree = alone(
        Element::clickable("area", Element::spacer()).area_slider(1.0, 0.0, 0.0),
        100.0,
        100.0,
    );
    tree.focus("area");
    assert!(matches!(
        key(&mut tree, Key::ArrowUp).as_slice(),
        [UiAction::AreaSliderChanged { x: 1.0, y: 0.0, .. }]
    ));
}

#[test]
fn a_track_slider_still_ignores_up_and_down() {
    let mut tree = alone(
        Element::clickable("track", Element::spacer()).slider(0.5, 0, 0.0),
        100.0,
        16.0,
    );
    tree.focus("track");
    assert!(tree
        .handle_event(UiEvent::Key {
            key: Key::ArrowUp,
            shift: false,
            ctrl: false
        })
        .is_empty());
}

#[test]
fn shares_that_are_not_numbers_or_out_of_range_are_brought_into_it() {
    for (x, y, expected) in [
        (f32::NAN, 2.0, (0.0, 1.0)),
        (-1.0, f32::INFINITY, (0.0, 0.0)),
    ] {
        let travel = Element::clickable("area", Element::spacer())
            .area_slider(x, y, f32::NAN)
            .slider
            .unwrap();
        assert_eq!(
            (travel.fraction, travel.y, travel.inset),
            (expected.0, Some(expected.1), 0.0)
        );
    }
}
