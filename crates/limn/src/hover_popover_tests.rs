//! A popover shown on hover is held by its trigger, by what lies inside the
//! trigger, and by its own surface.
use super::*;
use crate::{Component, ComponentPalette, Tooltip};

const VIEWPORT: Size = Size::new(600.0, 300.0);

fn tree() -> UiTree {
    // A row whose trigger holds a hint's passive anchor, as a person's row
    // with a muted mark, and a card beside it.
    let mark = Tooltip::new(
        Element::panel(Element::spacer()).size(Some(16.0), Some(16.0)),
        "row:muted",
        "Muted",
        ComponentPalette::neutral_dark(),
    )
    .render();
    let row = Element::clickable(
        "row",
        Element::row(vec![Element::text("Ada"), Element::spacer().flex(1.0), mark])
            .size(None, Some(32.0)),
    );
    let card = Element::button("watch", "Watch").size(Some(120.0), Some(32.0));
    let hovered = Element::popover(row, card, "card_dismiss", PopoverPlacement::RightStart)
        .popover_on_hover();
    let root = Element::column(vec![
        Element::spacer().size(None, Some(40.0)),
        Element::row(vec![
            hovered.size(Some(200.0), None),
            Element::spacer().flex(1.0),
        ]),
        Element::spacer().flex(1.0),
    ]);
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    tree
}

fn shown(tree: &mut UiTree) -> bool {
    tree.layout(VIEWPORT).regions.iter().any(|region| region.id == "watch")
}

fn rect(tree: &mut UiTree, id: &str) -> Rect {
    tree.layout(VIEWPORT)
        .regions
        .iter()
        .rev()
        .find(|region| region.id == id)
        .map(|region| region.rect)
        .unwrap()
}

#[test]
fn a_hint_inside_the_trigger_holds_a_popover_shown_on_hover() {
    let mut tree = tree();
    assert!(!shown(&mut tree));
    let row = rect(&mut tree, "row");
    tree.handle_event(UiEvent::PointerMove(Point::new(row.x + 10.0, row.y + 10.0)));
    assert!(shown(&mut tree));
    let mark = rect(&mut tree, "row:muted");
    tree.handle_event(UiEvent::PointerMove(Point::new(mark.x + 4.0, mark.y + 4.0)));
    assert_eq!(tree.hovered_id(), Some("row:muted"));
    assert!(shown(&mut tree), "the mark is part of the row");
    let watch = rect(&mut tree, "watch");
    tree.handle_event(UiEvent::PointerMove(Point::new(watch.x + 4.0, watch.y + 4.0)));
    assert!(shown(&mut tree));
    tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 280.0)));
    assert!(!shown(&mut tree));
}
