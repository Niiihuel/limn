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

#[test]
fn under_the_pointer_a_dimmed_layer_fades_only_to_its_opacity() {
    // How opaque the top layer is: away, under the pointer, then away again.
    let alphas = |opacity: f32| {
        let stack = Element::stack(vec![swatch(UNDER, 40.0, 40.0), swatch(OVER, 40.0, 40.0).dim_on_hover("dimmed", opacity)])
            .hover_region();
        let mut tree = UiTree::new(Element::row(vec![stack, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let size = Size::new(200.0, 40.0);
        let alpha = |tree: &mut UiTree| {
            fills(&tree.layout(size)).into_iter().find(|(_, color)| (color.r, color.b) == (OVER.r, OVER.b)).map_or(0, |(_, color)| color.a)
        };
        let away = alpha(&mut tree);
        tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
        let under = alpha(&mut tree);
        tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 20.0)));
        (away, under, alpha(&mut tree))
    };
    assert_eq!(alphas(0.35), (255, 89, 255), "35 % under the pointer, whole away from it");
    assert_eq!(alphas(2.0), (255, 255, 255), "clamped to 1");
    assert_eq!(alphas(-1.0).1, 0, "clamped to 0");
    assert_eq!(alphas(f32::NAN), (255, 255, 255), "not finite: left as it was");
}

#[test]
fn the_innermost_named_region_under_the_pointer_is_reported() {
    let row = |id: &str| Element::panel(Element::spacer()).size(None, Some(40.0)).hover_region_id(id);
    let list = Element::column(vec![row("row:1"), row("row:2"), Element::spacer().size(None, Some(40.0))]).hover_region_id("list");
    let mut tree = UiTree::new(list);
    let size = Size::new(200.0, 120.0);
    tree.layout(size);
    assert_eq!(tree.hovered_region(), None, "no pointer yet");
    for (y, expected) in [(20.0, "row:1"), (60.0, "row:2"), (100.0, "list")] {
        tree.handle_event(UiEvent::PointerMove(Point::new(20.0, y)));
        tree.layout(size);
        assert_eq!(tree.hovered_region(), Some(expected), "at {y}");
    }
    tree.handle_event(UiEvent::PointerLeave);
    tree.layout(size);
    assert_eq!(tree.hovered_region(), None, "the pointer left");
}

#[test]
fn a_popover_over_a_named_region_takes_the_pointer_from_it() {
    let base = |open: bool| {
        let row = Element::panel(Element::spacer()).size(Some(200.0), Some(200.0)).hover_region_id("row");
        let trigger = Element::button("open", "Open").size(Some(60.0), Some(24.0));
        let trigger = if open {
            let inside = Element::panel(Element::spacer()).size(Some(120.0), Some(60.0)).hover_region_id("inside");
            Element::popover(trigger, inside, "dismiss", PopoverPlacement::BottomStart)
        } else {
            trigger
        };
        Element::column(vec![trigger, row])
    };
    let size = Size::new(300.0, 300.0);
    let mut tree = UiTree::new(base(false));
    tree.handle_event(UiEvent::PointerMove(Point::new(40.0, 50.0)));
    tree.layout(size);
    assert_eq!(tree.hovered_region(), Some("row"));
    tree.set_root(base(true));
    tree.layout(size);
    assert_eq!(tree.hovered_region(), Some("inside"), "the popover's own region");
    tree.handle_event(UiEvent::PointerMove(Point::new(250.0, 250.0)));
    tree.layout(size);
    assert_eq!(tree.hovered_region(), None, "under its backdrop");
}
