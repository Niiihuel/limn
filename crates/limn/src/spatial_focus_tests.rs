//! The arrow keys move the focus as a television's remote does.
use super::*;

const VIEWPORT: Size = Size::new(600.0, 400.0);

fn button(id: &str) -> Element {
    Element::button(id, id).size(Some(100.0), Some(40.0))
}

/// Three rows of three, 20 apart.
fn grid() -> Element {
    let row = |row: usize| Element::row((0..3).map(|column| button(&format!("{row}{column}"))).collect()).gap(20.0);
    Element::column((0..3).map(row).collect()).gap(20.0)
}

fn press(tree: &mut UiTree, key: Key) {
    tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false });
    tree.layout(VIEWPORT);
}

#[test]
fn the_arrows_move_the_focus_to_the_neighbour_their_way_and_stop_at_the_edge() {
    let mut tree = UiTree::new(grid());
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("00"), "with nothing focused, the first control takes it");
    for (key, expected) in [(Key::ArrowRight, "01"), (Key::ArrowDown, "11"), (Key::ArrowRight, "12"), (Key::ArrowDown, "22"), (Key::ArrowLeft, "21"), (Key::ArrowUp, "11")] {
        press(&mut tree, key);
        assert_eq!(tree.focused_id(), Some(expected), "{key:?}");
    }
    press(&mut tree, Key::ArrowUp);
    press(&mut tree, Key::ArrowUp);
    assert_eq!(tree.focused_id(), Some("01"), "nothing above the top row: it stays");
}

#[test]
fn off_by_default_the_arrows_leave_the_focus_where_it_is() {
    let mut tree = UiTree::new(grid());
    tree.layout(VIEWPORT);
    assert!(tree.focus("11"));
    press(&mut tree, Key::ArrowRight);
    assert_eq!(tree.focused_id(), Some("11"));
}

#[test]
fn the_neighbour_most_in_line_wins_and_one_out_of_sight_is_scrolled_to() {
    // A wide control over two narrow ones: down goes to the one under its middle.
    let page = Element::column(vec![
        Element::button("wide", "wide").size(Some(300.0), Some(40.0)),
        Element::row(vec![button("left"), Element::spacer().size(Some(40.0), None), button("under")]).gap(20.0),
    ])
    .gap(20.0);
    let mut tree = UiTree::new(page);
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("wide"));
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("under"), "the one under its middle");

    let rows = (0..30).map(|row| button(&format!("row:{row}"))).collect();
    let list = Element::scroll("list", Element::column(rows).gap(10.0)).size(None, Some(200.0));
    let mut tree = UiTree::new(Element::column(vec![list, Element::spacer().flex(1.0)]));
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("row:3"));
    tree.layout(VIEWPORT);
    for _ in 0..3 {
        press(&mut tree, Key::ArrowDown);
    }
    assert_eq!(tree.focused_id(), Some("row:6"), "past the bottom of the list's view");
    assert!(tree.scroll_offset("list") > 0.0, "and brought into it");
}

#[test]
fn a_fields_caret_keeps_left_and_right_but_up_and_down_leave_it() {
    let page = Element::column(vec![Element::text_input("query", "hola", "").size(Some(300.0), Some(40.0)), button("below")]).gap(20.0);
    let mut tree = UiTree::new(page);
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("query"));
    press(&mut tree, Key::ArrowLeft);
    assert_eq!(tree.focused_id(), Some("query"), "the caret moves, not the focus");
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("below"));
}

#[test]
fn a_rows_end_stops_left_and_right_but_up_and_down_reach_a_short_row() {
    // A bar of three over one card at the far left.
    let page = Element::column(vec![
        Element::row(vec![Element::spacer().flex(1.0), button("home"), button("library"), button("settings")]).gap(20.0),
        Element::row(vec![button("card"), Element::spacer().flex(1.0)]),
    ])
    .gap(60.0);
    let mut tree = UiTree::new(page);
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("home"));
    press(&mut tree, Key::ArrowLeft);
    assert_eq!(tree.focused_id(), Some("home"), "nothing in its row to the left: it stays rather than drop to the card");
    assert!(tree.focus("settings"));
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("card"), "down reaches the row below, short as it is");
}

#[test]
fn down_reaches_a_shelf_below_the_view_whose_cards_scroll_sideways() {
    // A television's page: shelves one under the other, each a strip that scrolls sideways.
    let shelf = |shelf: usize| {
        let cards = (0..8).map(|card| Element::button(format!("{shelf}:{card}"), "card").size(Some(100.0), Some(120.0))).collect();
        Element::horizontal_scroll(format!("shelf:{shelf}"), Element::row(cards).gap(16.0))
    };
    let page = Element::scroll("page", Element::column((0..4).map(shelf).collect()).gap(20.0)).size(None, Some(300.0));
    let mut tree = UiTree::new(Element::column(vec![page, Element::spacer().flex(1.0)]));
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("1:0"));
    tree.layout(VIEWPORT);
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("2:0"), "into the shelf under the view");
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("3:0"));
    assert!(tree.scroll_offset("page") > 0.0, "and the page scrolled to it");
    press(&mut tree, Key::ArrowRight);
    press(&mut tree, Key::ArrowUp);
    press(&mut tree, Key::ArrowUp);
    press(&mut tree, Key::ArrowUp);
    assert_eq!(tree.focused_id(), Some("0:1"), "up the same column, to the shelf above the view");
}

#[test]
fn a_slider_keeps_the_arrows_until_its_end_where_they_move_the_focus_on() {
    // A picture's crop over its size: the area takes every arrow, the size Left and Right.
    let area = Element::clickable("area", Element::spacer()).area_slider(0.0, 0.5, 0.0).size(Some(200.0), Some(200.0));
    let page = Element::column(vec![area, button("done")]).gap(20.0);
    let mut tree = UiTree::new(page);
    tree.set_spatial_focus(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("area"));
    let moved = tree.handle_event(UiEvent::Key { key: Key::ArrowDown, shift: false, ctrl: false, alt: false });
    assert!(matches!(moved.as_slice(), [UiAction::AreaSliderChanged { .. }]), "it moves the area's mark: {moved:?}");
    assert_eq!(tree.focused_id(), Some("area"));
    press(&mut tree, Key::ArrowLeft);
    assert_eq!(tree.focused_id(), Some("area"), "at its left end, nothing to the left either: it stays");
    let page = Element::column(vec![Element::clickable("area", Element::spacer()).area_slider(0.5, 1.0, 0.0).size(Some(200.0), Some(200.0)), button("done")]).gap(20.0);
    tree.set_root(page);
    tree.layout(VIEWPORT);
    press(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("done"), "at its foot, Down goes on to what is under it");
}
