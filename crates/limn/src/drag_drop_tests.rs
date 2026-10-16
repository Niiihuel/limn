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

#[test]
fn files_cannot_reach_a_target_behind_a_modal_but_can_reach_its_body() {
    use crate::{Component, ComponentPalette, Modal};
    let palette = ComponentPalette::neutral_dark();
    let background = Element::button("behind", "Background")
        .size(Some(480.0), Some(480.0))
        .drop_target();
    let modal = Modal::new(background, true, "Import", "", "close", palette)
        .width(280.0)
        .body(Element::button("inside", "Import files").drop_target())
        .render();
    let mut tree = UiTree::new(modal);
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(480.0, 480.0));
    let inside = frame.regions.iter().find(|region| region.id == "inside").unwrap().rect;
    let points = [
        (Point::new(5.0, 5.0), None),
        (Point::new(inside.x + 2.0, inside.y + 2.0), Some("inside".to_owned())),
    ];
    let paths = vec!["/tmp/limn-a.txt".into(), "/tmp/limn-b.txt".into()];
    for (position, target) in points {
        assert_eq!(
            tree.handle_event(UiEvent::FilesHovered { position }),
            vec![UiAction::FilesHovered { position, over: target.clone() }],
            "file hover must stay in the active modal"
        );
        assert_eq!(
            tree.handle_event(UiEvent::FilesDropped { position, paths: paths.clone() }),
            vec![UiAction::FilesDropped { position, target, paths: paths.clone() }],
            "the complete file list must stay in the active modal"
        );
    }
}

#[test]
fn files_cannot_reach_a_target_through_an_open_context_menu() {
    use crate::{Component, ComponentPalette, ContextMenu, ContextMenuEntry, ContextMenuItem};
    let root = ContextMenu::new(
        "file-menu",
        Element::button("behind", "Background").size(Some(480.0), Some(480.0)).drop_target(),
        vec![ContextMenuEntry::Item(ContextMenuItem::new("inspect", "Inspect"))],
        ComponentPalette::neutral_dark(),
    ).render();
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(Size::new(480.0, 480.0));
    tree.handle_event(UiEvent::ContextMenu { position: Point::new(20.0, 20.0) });
    assert!(tree.context_menu_open());
    let frame = tree.layout(Size::new(480.0, 480.0));
    let menu = frame.regions.iter().find(|region| region.id == "__context_menu:surface").unwrap().rect;
    let position = Point::new(menu.x + 4.0, menu.y + 4.0);
    assert_eq!(tree.handle_event(UiEvent::FilesHovered { position }), vec![UiAction::FilesHovered { position, over: None }]);
    assert_eq!(
        tree.handle_event(UiEvent::FilesDropped { position, paths: vec!["/tmp/limn-a.txt".into()] }),
        vec![UiAction::FilesDropped { position, target: None, paths: vec!["/tmp/limn-a.txt".into()] }]
    );
}

#[test]
fn ghost_survives_its_source_scrolling_out_of_a_virtual_list() {
    let list = Element::uniform_list("files", 100, 30.0, |index| {
        Element::button(format!("row{index}"), format!("row{index}")).draggable()
    });
    let mut tree = UiTree::new(list);
    tree.set_reduced_motion(true);
    let size = Size::new(100.0, 90.0);
    tree.layout(size);
    let ghosts = |frame: &crate::Frame| {
        frame
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { text, .. } if text == "row0"))
            .count()
    };
    tree.handle_event(UiEvent::PointerDown(at(10.0)));
    tree.handle_event(UiEvent::PointerMove(at(40.0)));
    assert_eq!(ghosts(&tree.layout(size)), 2);
    tree.handle_event(UiEvent::Scroll { position: at(40.0), delta_y: -600.0 });
    let scrolled = tree.layout(size);
    assert_eq!(ghosts(&scrolled), 1, "the row is recycled; only its ghost remains");
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(at(40.0))),
        vec![UiAction::Dropped { id: "row0".into(), position: at(40.0), target: None }]
    );
}
