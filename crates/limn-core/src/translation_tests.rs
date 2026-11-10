use super::*;

#[test]
fn translation_moves_paint_hits_and_access_without_moving_flow_siblings() {
    let root = |offset| {
        Element::column(vec![
            Element::button("moving", "Moving")
                .size(Some(80.0), Some(32.0))
                .background(Color::rgb(1, 2, 3))
                .translate(offset),
            Element::button("sibling", "Sibling").size(Some(80.0), Some(32.0)),
        ])
    };
    let size = Size::new(240.0, 160.0);
    let plain = UiTree::new(root(Point::new(0.0, 0.0))).layout(size);
    let moved = UiTree::new(root(Point::new(100.0, 20.0))).layout(size);
    let region = |frame: &Frame, id: &str| {
        frame
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap()
            .rect
    };
    assert_eq!(region(&moved, "moving"), Rect::new(100.0, 20.0, 80.0, 32.0));
    assert_eq!(region(&moved, "sibling"), region(&plain, "sibling"));
    let access = moved
        .access
        .iter()
        .find(|node| node.id == "moving")
        .unwrap();
    assert_eq!(access.rect, region(&moved, "moving"));
    assert_eq!(moved.hit_test(Point::new(110.0, 24.0)), Some("moving"));
    assert_ne!(moved.hit_test(Point::new(10.0, 4.0)), Some("moving"));
    assert!(moved.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { rect, color, .. } if *rect == access.rect && *color == Color::rgb(1, 2, 3))));
}

#[test]
fn translation_keeps_the_ancestor_clip_fixed() {
    let child = Element::button("moving", "Moving")
        .background(Color::rgb(1, 2, 3))
        .size(Some(100.0), Some(60.0))
        .translate(Point::new(20.0, 40.0));
    let root = Element::column(vec![Element::column(vec![child])
        .size(Some(120.0), Some(80.0))
        .clip_children()]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(Size::new(240.0, 160.0));
    let visible = Rect::new(20.0, 40.0, 100.0, 40.0);
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "moving")
            .unwrap()
            .rect,
        visible
    );
    assert_eq!(
        frame
            .access
            .iter()
            .find(|node| node.id == "moving")
            .unwrap()
            .rect,
        visible
    );
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::PushClip(rect) if *rect == Rect::new(0.0, 0.0, 120.0, 80.0))));
    assert_eq!(frame.hit_test(Point::new(30.0, 70.0)), Some("moving"));
    assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), None);
    tree.handle_event(UiEvent::PointerDown(Point::new(30.0, 70.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(30.0, 70.0))),
        vec![UiAction::Clicked {
            id: "moving".into()
        }]
    );
}

#[test]
fn translation_moves_scroll_geometry_and_scroll_input_together() {
    let root = |offset| {
        Element::column(vec![Element::scroll(
            "rows",
            Element::column(
                (0..20)
                    .map(|index| {
                        Element::button(format!("row:{index}"), "Row").size(None, Some(32.0))
                    })
                    .collect(),
            ),
        )
        .size(Some(120.0), Some(100.0))
        .translate(offset)])
        .clip_children()
    };
    let size = Size::new(240.0, 160.0);
    let plain = UiTree::new(root(Point::new(0.0, 0.0))).layout(size);
    let mut tree = UiTree::new(root(Point::new(80.0, 30.0)));
    let moved = tree.layout(size);
    let before = plain.scrollbars["rows"];
    let after = moved.scrollbars["rows"];
    for (before, after) in [
        (before.rail, after.rail),
        (before.track, after.track),
        (before.thumb, after.thumb),
    ] {
        assert_eq!(
            after,
            Rect::new(
                before.x + 80.0,
                before.y + 30.0,
                before.width,
                before.height
            )
        );
    }
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(10.0, 10.0),
        delta_y: -32.0,
    });
    assert_eq!(tree.scroll_offset("rows"), 0.0);
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(90.0, 50.0),
        delta_y: -32.0,
    });
    assert_eq!(tree.scroll_offset("rows"), 32.0);
    let frame = tree.layout(size);
    assert_eq!(frame.hit_test(Point::new(90.0, 40.0)), Some("row:1"));
}

#[test]
fn nested_translations_compose_and_reduced_motion_preserves_direct_manipulation() {
    let child = Element::button("moving", "Moving")
        .size(Some(40.0), Some(32.0))
        .translate(Point::new(5.0, 7.0));
    let mut tree = UiTree::new(Element::column(vec![child]).translate(Point::new(10.0, 20.0)));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(200.0, 160.0));
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "moving")
            .unwrap()
            .rect,
        Rect::new(15.0, 27.0, 40.0, 32.0)
    );
    assert_eq!(tree.animation_repaint_after(), None);
}

#[test]
fn invalid_translations_leave_geometry_unchanged() {
    let root = || Element::button("moving", "Moving").size(Some(40.0), Some(32.0));
    let size = Size::new(200.0, 160.0);
    let plain = UiTree::new(root()).layout(size);
    for point in [Point::new(f32::NAN, 0.0), Point::new(0.0, f32::INFINITY)] {
        assert_eq!(UiTree::new(root().translate(point)).layout(size), plain);
    }
}

#[test]
fn keyboard_focus_reveals_a_translated_offscreen_descendant_in_a_moved_scroll() {
    let rows = Element::scroll(
        "rows",
        Element::column(vec![
            Element::spacer().size(None, Some(200.0)),
            Element::column(vec![Element::button("target", "Target")
                .size(None, Some(32.0))
                .translate(Point::new(0.0, 40.0))])
            .size(None, Some(80.0)),
            Element::spacer().size(None, Some(300.0)),
        ]),
    )
    .size(Some(200.0), Some(100.0))
    .translate(Point::new(20.0, 30.0));
    let mut tree = UiTree::new(Element::column(vec![rows]));
    tree.set_reduced_motion(true);
    let size = Size::new(240.0, 160.0);
    tree.layout(size);
    assert!(tree.focus("target"));
    let frame = tree.layout(size);
    assert_eq!(tree.scroll_offset("rows"), 172.0);
    let region = frame
        .regions
        .iter()
        .find(|region| region.id == "target")
        .unwrap();
    assert_eq!(region.rect.y, 98.0);
    assert_eq!(region.rect.height, 32.0);
}

#[test]
fn translated_popover_surface_blocks_the_backdrop_where_it_is_painted() {
    let color = Color::rgb(1, 2, 3);
    let floating = Element::panel(Element::spacer())
        .background(color)
        .size(Some(100.0), Some(60.0))
        .translate(Point::new(0.0, 40.0));
    let root = Element::popover(
        Element::spacer(),
        floating,
        "dismiss",
        PopoverPlacement::Center,
    )
    .popover_scrim(Color::rgba(0, 0, 0, 180))
    .popover_backdrop_dismiss(true);
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(240.0, 200.0));
    let painted = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect {
                rect, color: fill, ..
            } if *fill == color => Some(*rect),
            _ => None,
        })
        .unwrap();
    let surface = frame
        .regions
        .iter()
        .find(|region| region.id == "popover_surface:dismiss")
        .unwrap();
    assert_eq!(surface.rect, painted);
    let point = Point::new(painted.x + 10.0, painted.y + painted.height - 5.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
}

#[test]
fn translation_can_bring_a_child_from_an_unclipped_offscreen_flow_slot_into_view() {
    for clipped in [false, true] {
        let child = Element::button("target", "Target")
            .size(None, Some(32.0))
            .translate(Point::new(0.0, -180.0));
        let row = Element::column(vec![child]).size(None, Some(80.0));
        let row = if clipped { row.clip_children() } else { row };
        let root = Element::scroll(
            "rows",
            Element::column(vec![
                Element::spacer().size(None, Some(200.0)),
                row,
                Element::spacer().size(None, Some(300.0)),
            ]),
        );
        let frame = UiTree::new(root).layout(Size::new(200.0, 100.0));
        let target = frame.regions.iter().find(|region| region.id == "target");
        if clipped {
            assert!(
                target.is_none(),
                "a translation cannot escape an explicit ancestor clip"
            );
        } else {
            assert_eq!(target.unwrap().rect, Rect::new(0.0, 20.0, 186.0, 32.0));
            assert_eq!(frame.hit_test(Point::new(20.0, 25.0)), Some("target"));
        }
    }
}

#[test]
fn animated_foreground_clones_do_not_share_translation_culling_results() {
    let root = |color| {
        Element::scroll(
            "rows",
            Element::column(vec![
                Element::spacer().size(None, Some(200.0)),
                Element::column(vec![
                    Element::button("plain", "Plain").size(None, Some(32.0))
                ])
                .size(None, Some(80.0))
                .foreground(color)
                .animate_foreground("first"),
                Element::column(vec![Element::button("target", "Target")
                    .size(None, Some(32.0))
                    .translate(Point::new(0.0, -260.0))])
                .size(None, Some(80.0))
                .foreground(color)
                .animate_foreground("second"),
                Element::spacer().size(None, Some(300.0)),
            ]),
        )
    };
    let now = Instant::now();
    let size = Size::new(200.0, 100.0);
    let mut tree = UiTree::new(root(Color::rgb(255, 0, 0)));
    tree.layout_at(size, now);
    tree.set_root(root(Color::rgb(0, 0, 255)));
    let frame = tree.layout_at(size, now + Duration::from_millis(16));
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "target")
            .unwrap()
            .rect
            .y,
        20.0
    );
}
