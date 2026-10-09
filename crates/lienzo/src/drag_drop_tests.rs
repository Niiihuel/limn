use crate::{DrawCommand, Element, Key, Point, Size, UiAction, UiEvent, UiTree};

fn tree() -> UiTree {
    let rows = Element::column(vec![
        Element::button("file", "file").size(None, Some(30.0)).draggable(),
        Element::button("folder", "folder").size(None, Some(30.0)).drop_target(),
        Element::button("plain", "plain").size(None, Some(30.0)),
    ]);
    let mut tree = UiTree::new(Element::scroll("column", rows).drop_target());
    tree.set_reduced_motion(true);
    tree.layout(Size::new(100.0, 200.0));
    tree
}

fn at(y: f32) -> Point {
    Point::new(10.0, y)
}

#[test]
fn press_within_tolerance_still_clicks() {
    let mut tree = tree();
    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    assert!(tree.handle_event(UiEvent::PointerMove(at(12.0))).is_empty());
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(at(12.0))),
        vec![UiAction::Clicked { id: "file".into() }]
    );
}

#[test]
fn drag_reports_the_innermost_target_then_drops_without_clicking() {
    let mut tree = tree();
    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerMove(at(45.0))),
        vec![
            UiAction::DragStarted { id: "file".into(), position: at(10.0) },
            UiAction::DragMoved { id: "file".into(), position: at(45.0), over: Some("folder".into()) },
        ]
    );
    // A row that accepts nothing falls through to the column behind it.
    assert_eq!(
        tree.handle_event(UiEvent::PointerMove(at(75.0))),
        vec![UiAction::DragMoved { id: "file".into(), position: at(75.0), over: Some("column".into()) }]
    );
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(at(45.0))),
        vec![UiAction::Dropped { id: "file".into(), position: at(45.0), target: Some("folder".into()) }]
    );
    assert!(tree.handle_event(UiEvent::PointerUp(at(45.0))).is_empty());
}

#[test]
fn escape_and_pointer_leave_cancel_and_undraggable_presses_never_drag() {
    let mut tree = tree();
    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    tree.handle_event(UiEvent::PointerMove(at(45.0)));
    assert_eq!(
        tree.handle_event(UiEvent::Key { key: Key::Escape, shift: false, ctrl: false }),
        vec![UiAction::DragCancelled { id: "file".into() }]
    );
    assert!(tree.handle_event(UiEvent::PointerUp(at(45.0))).is_empty());

    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    tree.handle_event(UiEvent::PointerMove(at(45.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerLeave),
        vec![UiAction::DragCancelled { id: "file".into() }]
    );

    tree.handle_event(UiEvent::PointerDown(at(75.0)));
    assert!(tree.handle_event(UiEvent::PointerMove(at(10.0))).is_empty());
}

#[test]
fn active_drag_paints_a_ghost_of_the_source_at_the_pointer_offset() {
    let mut tree = tree();
    let label = |frame: &crate::Frame| -> Vec<Point> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "file" => Some(*position),
                _ => None,
            })
            .collect()
    };
    let resting = label(&tree.layout(Size::new(100.0, 200.0)));
    assert_eq!(resting.len(), 1);
    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(30.0, 70.0)));
    let dragging = tree.layout(Size::new(100.0, 200.0));
    let labels = label(&dragging);
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[1], Point::new(resting[0].x + 20.0, resting[0].y + 60.0));
    // The ghost is painted last and adds no pointer target.
    assert!(matches!(dragging.commands.last(), Some(DrawCommand::Text { text, .. }) if text == "file"));
    tree.handle_event(UiEvent::PointerUp(Point::new(30.0, 70.0)));
    assert_eq!(label(&tree.layout(Size::new(100.0, 200.0))).len(), 1);
}

#[test]
fn files_from_another_application_resolve_the_drop_target_under_the_pointer() {
    let mut tree = tree();
    assert_eq!(
        tree.handle_event(UiEvent::FilesHovered { position: at(10.0) }),
        vec![UiAction::FilesHovered { position: at(10.0), over: Some("column".into()) }]
    );
    let paths = vec![std::path::PathBuf::from("/tmp/a.txt")];
    assert_eq!(
        tree.handle_event(UiEvent::FilesDropped { position: at(45.0), paths: paths.clone() }),
        vec![UiAction::FilesDropped { position: at(45.0), target: Some("folder".into()), paths }]
    );
}
