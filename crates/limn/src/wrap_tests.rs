use super::*;

/// A target `width` by `height` that reports `id` when clicked.
fn tile(id: &str, width: f32, height: f32) -> Element {
    Element::clickable(id, Element::spacer().size(Some(width), Some(height)))
}

fn tags(count: usize) -> Element {
    Element::row(
        (0..count)
            .map(|index| tile(&format!("tag:{index}"), 40.0, 20.0))
            .collect(),
    )
    .gap(10.0)
    .wrap()
}

fn rect(frame: &Frame, id: &str) -> Rect {
    frame
        .regions
        .iter()
        .find(|region| region.id == id)
        .unwrap()
        .rect
}

#[test]
fn children_move_to_the_next_line_and_the_row_grows_with_its_lines() {
    let mut tree = UiTree::new(Element::column(vec![
        tags(5).line_gap(6.0),
        tile("after", 100.0, 10.0),
    ]));
    let frame = tree.layout(Size::new(100.0, 200.0));
    let placed: Vec<_> = (0..5)
        .map(|index| rect(&frame, &format!("tag:{index}")))
        .collect();
    assert_eq!(
        placed,
        [
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Rect::new(50.0, 0.0, 40.0, 20.0),
            Rect::new(0.0, 26.0, 40.0, 20.0),
            Rect::new(50.0, 26.0, 40.0, 20.0),
            Rect::new(0.0, 52.0, 40.0, 20.0),
        ]
    );
    assert_eq!(
        rect(&frame, "after").y,
        72.0,
        "the next sibling starts under the last line"
    );
    let point = Point::new(70.0, 36.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked { id: "tag:3".into() }]
    );
}

#[test]
fn a_line_is_as_tall_as_its_tallest_child_and_its_lines_are_gap_apart_by_default() {
    let row = Element::row(vec![
        tile("short", 30.0, 20.0),
        tile("tall", 30.0, 40.0),
        tile("sized", 30.0, 10.0).size(None, Some(10.0)),
        tile("next", 70.0, 20.0),
    ])
    .gap(8.0)
    .wrap();
    let frame = UiTree::new(Element::column(vec![row])).layout(Size::new(120.0, 200.0));
    assert_eq!(rect(&frame, "short"), Rect::new(0.0, 0.0, 30.0, 40.0));
    assert_eq!(rect(&frame, "tall"), Rect::new(38.0, 0.0, 30.0, 40.0));
    assert_eq!(rect(&frame, "sized"), Rect::new(76.0, 0.0, 30.0, 10.0));
    assert_eq!(rect(&frame, "next"), Rect::new(0.0, 48.0, 70.0, 20.0));
}

#[test]
fn a_child_wider_than_the_row_has_a_line_of_its_own_at_the_row_width() {
    let row = Element::row(vec![
        tile("first", 40.0, 20.0),
        Element::clickable("long", Element::text("aaaa bbbb cccc dddd")),
        tile("last", 40.0, 20.0),
    ])
    .gap(4.0)
    .wrap();
    let mut tree = UiTree::new(Element::column(vec![row]));
    tree.set_text_measurer(|text, _size, _family| text.chars().count() as f32 * 8.0);
    let frame = tree.layout(Size::new(100.0, 300.0));
    let long = rect(&frame, "long");
    assert_eq!((long.x, long.y, long.width), (0.0, 24.0, 100.0));
    assert!(long.height > 20.0, "its text wraps at the row's width");
    assert_eq!(
        rect(&frame, "last"),
        Rect::new(0.0, long.y + long.height + 4.0, 40.0, 20.0)
    );
}

#[test]
fn tab_and_assistive_technology_take_the_children_in_order_across_lines() {
    let ids = ["one", "two", "three", "four", "five"];
    let buttons = ids
        .iter()
        .map(|id| Element::button(*id, *id).size(Some(40.0), Some(20.0)))
        .collect();
    let mut tree = UiTree::new(Element::column(vec![Element::row(buttons).gap(4.0).wrap()]));
    let frame = tree.layout(Size::new(90.0, 200.0));
    assert!(
        rect(&frame, "five").y > rect(&frame, "two").y,
        "the buttons take three lines"
    );
    let announced: Vec<_> = frame
        .access
        .iter()
        .map(|node| node.id.as_str())
        .filter(|id| ids.contains(id))
        .collect();
    assert_eq!(announced, ids);
    let mut walked = Vec::new();
    for _ in ids {
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        walked.extend(tree.focused_id().map(str::to_owned));
    }
    assert_eq!(walked, ids);
}

#[test]
fn nothing_escapes_a_narrow_row_and_a_wide_one_keeps_a_single_line() {
    let mut heights = Vec::new();
    for width in [30.0, 90.0, 140.0, 400.0] {
        let mut tree = UiTree::new(Element::column(vec![tags(5), tile("after", 10.0, 10.0)]));
        let frame = tree.layout(Size::new(width, 400.0));
        for index in 0..5 {
            let tag = rect(&frame, &format!("tag:{index}"));
            assert!(tag.x + tag.width <= width, "tag {index} at {width}");
        }
        heights.push(rect(&frame, "after").y);
    }
    assert_eq!(heights, [140.0, 80.0, 50.0, 20.0]);
}

#[test]
fn a_wrapping_row_in_a_row_is_measured_at_the_width_it_is_given() {
    for tags in [tags(4).flex(1.0), tags(4).shrink_to_fit()] {
        let header = Element::row(vec![tile("side", 20.0, 20.0), tags]).gap(10.0);
        let frame = UiTree::new(Element::column(vec![header, tile("after", 10.0, 10.0)]))
            .layout(Size::new(120.0, 200.0));
        assert_eq!(rect(&frame, "tag:2"), Rect::new(30.0, 30.0, 40.0, 20.0));
        assert_eq!(rect(&frame, "after").y, 50.0);
    }
}

#[test]
fn reveal_finds_a_child_on_a_later_line() {
    let content = Element::column(vec![
        tile("top", 100.0, 150.0),
        tags(5).line_gap(6.0),
        tile("bottom", 100.0, 150.0),
    ]);
    let mut tree = UiTree::new(Element::scroll("list", content));
    tree.set_reduced_motion(true);
    let viewport = Size::new(100.0, 100.0);
    tree.layout(viewport);
    tree.reveal("tag:4");
    let frame = tree.layout(viewport);
    let tag = rect(&frame, "tag:4");
    assert_eq!(tag.y + tag.height, 100.0 - REVEAL_MARGIN);
}

#[test]
fn only_a_row_wraps_and_its_line_gap_takes_lengths_only() {
    let row = || Element::row(Vec::new()).gap(8.0);
    assert_eq!(row().wrapping_line_gap(), None);
    assert_eq!(row().wrap().wrapping_line_gap(), Some(8.0));
    assert_eq!(row().line_gap(3.0).wrap().wrapping_line_gap(), Some(3.0));
    assert_eq!(row().wrap().line_gap(-3.0).wrapping_line_gap(), Some(0.0));
    for bad in [f32::NAN, f32::INFINITY] {
        assert_eq!(row().wrap().line_gap(bad).wrapping_line_gap(), Some(8.0));
    }
    assert_eq!(Element::column(Vec::new()).wrap().wrapping_line_gap(), None);
    let empty = UiTree::new(Element::column(vec![
        row().wrap(),
        tile("after", 10.0, 10.0),
    ]))
    .layout(Size::new(100.0, 100.0));
    assert_eq!(rect(&empty, "after").y, 0.0, "an empty row takes no height");
}
