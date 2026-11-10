use super::*;

#[test]
fn passive_header_blocks_rows_only_inside_its_translated_and_clipped_bounds() {
    let header = Element::panel(Element::text("Title"))
        .size(None, Some(60.0))
        .translate(Point::new(20.0, 40.0))
        .block_pointer("header");
    let root = Element::column(vec![Element::stack(vec![
        Element::button("under", "Underlying row"),
        Element::column(vec![header, Element::spacer().flex(1.0)]),
    ])
    .size(Some(120.0), Some(80.0))
    .clip_children()]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(Size::new(240.0, 160.0));
    let point = Point::new(30.0, 60.0);
    assert_eq!(frame.hit_test(point), Some("header"));
    let blocker = frame
        .regions
        .iter()
        .find(|region| region.id == "header")
        .unwrap();
    assert_eq!(blocker.kind, HitKind::Surface);
    assert_eq!(blocker.rect, Rect::new(20.0, 40.0, 100.0, 40.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 60.0)), Some("under"));
    assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), None);
    tree.handle_event(UiEvent::PointerDown(point));
    assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
}

#[test]
fn controls_inside_a_pointer_blocker_keep_their_actions() {
    let header = Element::row(vec![
        Element::text("Title").flex(1.0),
        Element::button("close", "Close").size(Some(44.0), Some(44.0)),
    ])
    .padding(8.0)
    .block_pointer("header");
    let mut tree = UiTree::new(Element::stack(vec![
        Element::button("under", "Underlying row"),
        header,
    ]));
    let frame = tree.layout(Size::new(200.0, 60.0));
    let close = frame
        .regions
        .iter()
        .find(|region| region.id == "close")
        .unwrap()
        .rect;
    let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
    assert_eq!(frame.hit_test(Point::new(2.0, 2.0)), Some("header"));
    assert_eq!(frame.hit_test(point), Some("close"));
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked { id: "close".into() }]
    );
}

#[test]
fn pointer_blockers_add_no_accessibility_node_or_keyboard_focus_stop() {
    let root = Element::column(vec![
        Element::button("first", "First"),
        Element::panel(Element::text("Title"))
            .size(None, Some(40.0))
            .block_pointer("header"),
        Element::button("last", "Last"),
    ]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(Size::new(200.0, 160.0));
    assert_eq!(
        frame
            .access
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>(),
        ["first", "last"]
    );
    assert!(!tree.focus("header"));
    for expected in ["first", "last", "first"] {
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
                alt: false
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some(expected));
    }
}
