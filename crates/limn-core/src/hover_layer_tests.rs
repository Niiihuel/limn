//! A floating surface under the pointer takes its hover from what lies
//! beneath it: only the layer on top, and what holds it there, is hovered.
use super::*;

const SIZE: Size = Size::new(300.0, 200.0);
const ROW_LIT: Color = Color::rgb(200, 30, 30);
const INNER_LIT: Color = Color::rgb(30, 200, 30);

/// A row of the base tree whose fill shows under the pointer, as a channel's.
fn lit_row(id: &str, color: Color) -> Element {
    Element::clickable(
        id,
        Element::stack(vec![
            Element::panel(Element::spacer()).background(color).reveal_on_hover(format!("{id}:lit")),
            Element::spacer(),
        ]),
    )
    .cursor(Cursor::ResizeRow)
    .hover_region()
}

fn shown(frame: &Frame, color: Color) -> bool {
    frame.commands.iter().any(|command| {
        matches!(command, DrawCommand::FillRect { color: fill, .. }
            if (fill.r, fill.g, fill.b) == (color.r, color.g, color.b) && fill.a > 0)
    })
}

fn middle(frame: &Frame, id: &str) -> Point {
    let rect = frame.regions.iter().rev().find(|region| region.id == id).unwrap().rect;
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

/// A trigger with `surface` floating under it, over a lit row.
fn page(surface: Option<Element>, modal: bool) -> Element {
    let trigger = Element::button("open", "Open").size(Some(60.0), Some(24.0));
    let trigger = match surface {
        Some(surface) => {
            let popover = Element::popover(trigger, surface, "dismiss", PopoverPlacement::BottomStart);
            if modal { popover } else { popover.popover_non_modal() }
        }
        None => trigger,
    };
    Element::column(vec![trigger, lit_row("row", ROW_LIT).flex(1.0)])
}

fn tree(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree
}

fn point_at(tree: &mut UiTree, point: Point) -> Frame {
    tree.handle_event(UiEvent::PointerMove(point));
    tree.layout(SIZE)
}

#[test]
fn a_row_under_a_popover_is_not_hovered_and_lights_again_beside_it() {
    let control = Element::button("inside", "Inside").size(Some(120.0), Some(40.0));
    let mut tree = tree(page(Some(control), false));
    let frame = tree.layout(SIZE);
    let inside = middle(&frame, "inside");
    let frame = point_at(&mut tree, inside);
    assert_eq!(tree.hovered_id(), Some("inside"), "the popover's control is hovered");
    assert!(!shown(&frame, ROW_LIT), "the row under the popover stays dark");
    assert_eq!(tree.hovered_region(), None);
    assert_eq!(tree.cursor(), None, "not the row's cursor");
    let frame = point_at(&mut tree, Point::new(250.0, 150.0));
    assert_eq!(tree.hovered_id(), Some("row"));
    assert!(shown(&frame, ROW_LIT), "beside the popover the row is hovered again");
    assert_eq!(tree.cursor(), Some(Cursor::ResizeRow));
}

#[test]
fn a_popovers_backdrop_takes_the_hover_from_the_rows_as_it_takes_their_presses() {
    let control = Element::button("inside", "Inside").size(Some(120.0), Some(40.0));
    let mut tree = tree(page(Some(control), true));
    tree.layout(SIZE);
    let frame = point_at(&mut tree, Point::new(250.0, 150.0));
    assert_eq!(tree.hovered_id(), Some("dismiss"));
    assert!(!shown(&frame, ROW_LIT));
}

#[test]
fn a_part_of_a_popover_past_its_box_covers_the_row_too() {
    // A bubble drawn past the surface's box, as a profile card's status over its banner.
    let bubble = Element::button("bubble", "Status").size(Some(120.0), Some(30.0)).translate(Point::new(0.0, 60.0));
    let surface = Element::column(vec![Element::spacer().size(Some(140.0), Some(20.0)), bubble]);
    let mut tree = tree(page(Some(surface), false));
    let frame = tree.layout(SIZE);
    let bubble = middle(&frame, "bubble");
    let own = frame.regions.iter().find(|region| region.id == "popover_surface:dismiss").unwrap().rect;
    assert!(!own.contains(bubble), "the bubble overhangs the surface's box");
    let frame = point_at(&mut tree, bubble);
    assert_eq!(tree.hovered_id(), Some("bubble"));
    assert!(!shown(&frame, ROW_LIT));
}

#[test]
fn a_popover_in_a_popover_covers_both_layers_beneath_it() {
    let deep = Element::button("deep", "Deep").size(Some(100.0), Some(30.0));
    let nested = Element::popover(
        Element::button("open_inner", "Inner").size(Some(60.0), Some(24.0)),
        deep,
        "inner_dismiss",
        PopoverPlacement::BottomStart,
    )
    .popover_non_modal();
    let surface = Element::column(vec![nested, lit_row("inner_row", INNER_LIT).size(Some(200.0), Some(80.0))]);
    let mut tree = tree(page(Some(surface), false));
    let frame = tree.layout(SIZE);
    let deep = middle(&frame, "deep");
    let frame = point_at(&mut tree, deep);
    assert_eq!(tree.hovered_id(), Some("deep"));
    assert!(!shown(&frame, INNER_LIT), "the outer popover's row is under the inner one");
    assert!(!shown(&frame, ROW_LIT));
    let inner_row = frame.regions.iter().find(|region| region.id == "inner_row").unwrap().rect;
    let frame = point_at(&mut tree, Point::new(inner_row.x + inner_row.width - 10.0, inner_row.y + inner_row.height - 10.0));
    assert_eq!(tree.hovered_id(), Some("inner_row"));
    assert!(shown(&frame, INNER_LIT), "the outer popover's row beside the inner one");
    assert!(!shown(&frame, ROW_LIT));
}

#[test]
fn a_surface_opening_or_closing_under_a_still_pointer_repaints_in_the_same_layout() {
    let control = || Element::button("inside", "Inside").size(Some(120.0), Some(40.0));
    let mut tree = tree(page(Some(control()), false));
    let point = middle(&tree.layout(SIZE), "inside");
    tree.set_root(page(None, false));
    assert!(shown(&point_at(&mut tree, point), ROW_LIT));
    tree.set_root(page(Some(control()), false));
    assert!(!shown(&tree.layout(SIZE), ROW_LIT), "opened over the pointer");
    tree.set_root(page(None, false));
    assert!(shown(&tree.layout(SIZE), ROW_LIT), "closed from under it");
}
