use super::*;
use std::time::Duration;

fn list(order: &[&str]) -> Element {
    Element::column(
        order
            .iter()
            .map(|name| Element::button(*name, *name).size(None, Some(30.0)).animate_move(*name))
            .collect(),
    )
}

fn top_of(frame: &Frame, id: &str) -> f32 {
    frame.regions.iter().find(|region| region.id == id).unwrap().rect.y
}

#[test]
fn a_moved_element_glides_from_where_it_was_to_its_new_slot() {
    let size = Size::new(200.0, 120.0);
    let start = Instant::now();
    let mut tree = UiTree::new(list(&["a", "b", "c"]));
    let rest = tree.layout_at(size, start);
    let (a, c) = (top_of(&rest, "a"), top_of(&rest, "c"));

    // "c" goes first: on the frame of the change everything is still where it was shown.
    tree.set_root(list(&["c", "a", "b"]));
    let changed = tree.layout_at(size, start);
    assert_eq!(top_of(&changed, "c"), c);
    assert_eq!(top_of(&changed, "a"), a);
    let midway = top_of(&tree.layout_at(size, start + Duration::from_millis(60)), "c");
    assert!(midway < c && midway > a, "on its way up: {midway}");
    let settled = tree.layout_at(size, start + Duration::from_secs(1));
    assert_eq!(top_of(&settled, "c"), a);

    // A new one rests where it lands, and reduced motion moves at once.
    tree.set_root(list(&["d", "c", "a", "b"]));
    assert_eq!(top_of(&tree.layout_at(size, start + Duration::from_secs(1)), "d"), a);
    tree.set_reduced_motion(true);
    tree.set_root(list(&["a", "b", "c", "d"]));
    let step = top_of(&settled, "b") - top_of(&settled, "a");
    let now = start + Duration::from_secs(2);
    assert_eq!(top_of(&tree.layout_at(size, now), "d"), a + 3.0 * step);
}

#[test]
fn the_parents_own_move_carries_a_moving_element_at_once() {
    let size = Size::new(200.0, 200.0);
    let start = Instant::now();
    let page = |gap: f32| Element::column(vec![Element::spacer().size(None, Some(gap)), list(&["a", "b"])]);
    let mut tree = UiTree::new(page(0.0));
    let before = top_of(&tree.layout_at(size, start), "a");
    tree.set_root(page(40.0));
    assert_eq!(top_of(&tree.layout_at(size, start), "a"), before + 40.0);
}
