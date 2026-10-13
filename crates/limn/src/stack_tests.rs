use super::*;

const UNDER: Color = Color::rgb(200, 30, 30);
const OVER: Color = Color::rgb(30, 30, 200);

fn fills(frame: &Frame) -> Vec<(Rect, Color)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

fn swatch(color: Color, width: f32, height: f32) -> Element {
    Element::panel(Element::spacer().size(Some(width), Some(height))).background(color)
}

#[test]
fn a_stack_is_as_large_as_its_largest_child_and_paints_them_in_order_in_one_box() {
    let stack = Element::stack(vec![swatch(UNDER, 40.0, 30.0), swatch(OVER, 20.0, 50.0)]);
    let mut tree = UiTree::new(Element::row(vec![stack, Element::spacer().flex(1.0)]));
    let frame = tree.layout(Size::new(200.0, 50.0));
    let boxed = Rect::new(0.0, 0.0, 40.0, 50.0);
    assert_eq!(fills(&frame), [(boxed, UNDER), (boxed, OVER)]);
    assert!(fills(&UiTree::new(Element::stack(Vec::new())).layout(Size::new(10.0, 10.0))).is_empty());
}

#[test]
fn where_two_take_the_pointer_the_later_one_has_it() {
    let layer = |id: &str| Element::clickable(id, Element::spacer().size(Some(40.0), Some(40.0)));
    let mut tree = UiTree::new(Element::row(vec![Element::stack(vec![layer("under"), layer("over")])]));
    tree.layout(Size::new(100.0, 40.0));
    let point = Point::new(20.0, 20.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: "over".into() }]);
}

#[test]
fn under_the_pointer_a_concealed_layer_gives_way_to_a_revealed_one() {
    let stack = Element::stack(vec![
        swatch(UNDER, 40.0, 40.0).conceal_on_hover("resting"),
        swatch(OVER, 40.0, 40.0).reveal_on_hover("hovered"),
    ])
    .hover_region();
    let mut tree = UiTree::new(Element::row(vec![stack, Element::spacer().flex(1.0)]));
    tree.set_reduced_motion(true);
    let size = Size::new(200.0, 40.0);
    let shown = |frame: &Frame| fills(frame).into_iter().filter(|(_, color)| color.a > 0).map(|(_, color)| color).collect::<Vec<_>>();
    assert_eq!(shown(&tree.layout(size)), [UNDER]);
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    assert_eq!(shown(&tree.layout(size)), [OVER]);
    tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 20.0)));
    assert_eq!(shown(&tree.layout(size)), [UNDER]);
}
