use super::*;

const SIZE: Size = Size::new(100.0, 100.0);

/// Three sections of a 20 px header and five 20 px rows.
fn sections(sticky: bool, interactive: bool) -> Element {
    let mut children = Vec::new();
    for section in 0..3 {
        let header = if interactive {
            Element::button(format!("h{section}"), "Header").size(None, Some(20.0))
        } else {
            Element::text("Header").size(None, Some(20.0))
        };
        children.push(if sticky { header.sticky_top() } else { header });
        for row in 0..5 {
            children.push(Element::button(format!("r{section}_{row}"), "Row").size(None, Some(20.0)));
        }
    }
    Element::scroll("list", Element::column(children))
}

fn scrolled(root: Element, offset: f32) -> Frame {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(5.0, 5.0),
        delta_y: -offset,
    });
    tree.layout(SIZE)
}

fn top_of(frame: &Frame, id: &str) -> Option<f32> {
    frame.regions.iter().rev().find(|region| region.id == id).map(|region| region.rect.y)
}

#[test]
fn sticky_child_at_its_natural_position_changes_nothing() {
    assert_eq!(scrolled(sections(true, true), 0.0), scrolled(sections(false, true), 0.0));
}

#[test]
fn sticky_child_pins_to_the_viewport_top_and_takes_clicks_there() {
    let frame = scrolled(sections(true, true), 50.0);
    assert_eq!(top_of(&frame, "h0"), Some(0.0));
    // The row passing under it is still laid out, but the header wins.
    assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("h0"));
    assert_eq!(frame.hit_test(Point::new(5.0, 25.0)), Some("r0_2"));
    assert_eq!(frame.regions.iter().filter(|region| region.id == "h0").count(), 1);
    // It is painted after every row, so it covers them.
    let plain = scrolled(sections(false, true), 50.0);
    assert!(frame.commands.len() > plain.commands.len());
}

#[test]
fn next_sticky_child_pushes_the_pinned_one_out_then_takes_its_place() {
    let frame = scrolled(sections(true, true), 110.0);
    assert_eq!(top_of(&frame, "h1"), Some(10.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("h0"));
    assert_eq!(frame.hit_test(Point::new(5.0, 15.0)), Some("h1"));

    let frame = scrolled(sections(true, true), 130.0);
    assert_eq!(top_of(&frame, "h1"), Some(0.0));
    assert_eq!(top_of(&frame, "h0"), None);
}

#[test]
fn passive_sticky_child_blocks_the_rows_under_it() {
    let frame = scrolled(sections(true, false), 50.0);
    assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("list"));
    assert_eq!(frame.hit_test(Point::new(5.0, 25.0)), Some("r0_2"));
}

#[test]
fn sticky_children_leave_the_scroll_range_alone() {
    for offset in [0.0, 50.0, 130.0, 1000.0] {
        let sticky = scrolled(sections(true, true), offset);
        let plain = scrolled(sections(false, true), offset);
        assert_eq!(sticky.scroll_extents, plain.scroll_extents);
        assert_eq!(top_of(&sticky, "r2_4"), top_of(&plain, "r2_4"));
    }
}

#[test]
fn sticky_child_outside_a_scroll_area_is_an_ordinary_child() {
    let column = |sticky: bool| {
        let header = Element::button("h", "Header").size(None, Some(20.0));
        Element::column(vec![if sticky { header.sticky_top() } else { header }])
    };
    let mut sticky = UiTree::new(column(true));
    let mut plain = UiTree::new(column(false));
    assert_eq!(sticky.layout(SIZE), plain.layout(SIZE));
}

#[test]
fn capped_sticky_child_shrinks_with_the_scroll_down_to_its_cap() {
    let root = || {
        Element::scroll(
            "list",
            Element::column(vec![
                Element::button("tall", "Tall")
                    .size(None, Some(80.0))
                    .background(Color::rgb(20, 20, 20))
                    .sticky_top_max_height(30.0),
                Element::button("rest", "Rest").size(None, Some(400.0)),
            ]),
        )
    };
    let rect = |offset| {
        let frame = scrolled(root(), offset);
        let region = frame.regions.iter().rev().find(|region| region.id == "tall").unwrap();
        (region.rect.y, region.rect.height, frame)
    };
    let (y, height, frame) = rect(20.0);
    assert_eq!((y, height), (0.0, 60.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 70.0)), Some("rest"));
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillVerticalGradient { .. })));

    let (y, height, frame) = rect(200.0);
    assert_eq!((y, height), (0.0, 30.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 35.0)), Some("rest"));
    // Invalid caps are ignored.
    assert_eq!(Element::spacer().sticky_top_max_height(f32::NAN).sticky_top, None);
    assert_eq!(Element::spacer().sticky_top_inset(-4.0).sticky_top, None);
    assert_eq!(Element::spacer().sticky_top_max_height(-1.0).sticky_top, None);
    assert_eq!(Element::spacer().sticky_top_max_height(0.2).sticky_top, None);
}

#[test]
fn pinned_child_keeps_its_inset_and_paints_above_the_scroll_fade() {
    let mut children = vec![Element::button("h", "Header").size(None, Some(20.0)).sticky_top_inset(6.0)];
    children.extend((0..10).map(|row| Element::button(format!("r{row}"), "Row").size(None, Some(20.0))));
    let root = Element::scroll("list", Element::column(children)).top_fade(Color::rgb(20, 20, 20), 12.0);
    let frame = scrolled(root, 50.0);
    assert_eq!(top_of(&frame, "h"), Some(6.0));
    let fade = frame
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. }))
        .expect("the scrolled area fades its top");
    let header = frame
        .commands
        .iter()
        .rposition(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Header"))
        .expect("the pinned header is painted");
    assert!(header > fade);
    // Pushes and pops stay balanced around the deferred child.
    let depth = frame.commands.iter().fold(0_i32, |depth, command| match command {
        DrawCommand::PushClip(_) => depth + 1,
        DrawCommand::PopClip => depth - 1,
        _ => depth,
    });
    assert_eq!(depth, 0);
}

#[test]
fn pinned_child_fades_the_rows_under_it_in_the_scroll_areas_fade_color() {
    let color = Color::rgb(20, 20, 20);
    let root = |fade: bool, top_fade: bool| {
        let mut children = Vec::new();
        for section in 0..2 {
            let header = Element::text("Header").size(None, Some(20.0)).sticky_top();
            children.push(if fade { header.sticky_top_fade(16.0) } else { header });
            children.extend(
                (0..5).map(|row| Element::button(format!("r{section}_{row}"), "Row").size(None, Some(20.0))),
            );
        }
        let scroll = Element::scroll("list", Element::column(children));
        if top_fade { scroll.top_fade(color, 4.0) } else { scroll }
    };
    // The gradient under the pinned child: the last one painted.
    let fade = |frame: &Frame| {
        frame.commands.iter().rev().find_map(|command| match command {
            DrawCommand::FillVerticalGradient { rect, top, bottom } => Some((*rect, *top, *bottom)),
            _ => None,
        })
    };
    let frame = scrolled(root(true, true), 50.0);
    let (rect, top, bottom) = fade(&frame).unwrap();
    assert_eq!((rect.y, rect.height), (20.0, 16.0));
    assert_eq!((top, bottom), (color, Color { a: 0, ..color }));
    // Paint only: the row under the fade takes the click, and nothing moves.
    assert_eq!(frame.hit_test(Point::new(5.0, 25.0)), Some("r0_2"));
    let plain = scrolled(root(false, true), 50.0);
    assert_eq!(frame.regions, plain.regions);
    assert_eq!(frame.scroll_extents, plain.scroll_extents);
    // Behind the child the same color hides what scrolls around its corners.
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { rect, color: fill, .. }
            if *fill == color && (rect.y, rect.height) == (0.0, 20.0))));
    assert_eq!(frame.commands.len(), plain.commands.len() + 2);
    // Not at its natural position, and it arrives with the first pixels.
    assert_eq!(scrolled(root(true, true), 0.0), scrolled(root(false, true), 0.0));
    let (_, top, _) = fade(&scrolled(root(true, true), 4.0)).unwrap();
    assert_eq!(top.a, 64);
    // The next sticky child takes the room: 6 px are left, then none.
    let (rect, _, _) = fade(&scrolled(root(true, true), 94.0)).unwrap();
    assert_eq!((rect.y, rect.height), (20.0, 6.0));
    assert_eq!(scrolled(root(true, true), 110.0), scrolled(root(false, true), 110.0));
    // The color is the scroll area's: without a top fade nothing is drawn.
    assert_eq!(scrolled(root(true, false), 50.0), scrolled(root(false, false), 50.0));
    assert_eq!(Element::spacer().sticky_top_fade(f32::NAN).sticky_top, None);
    assert_eq!(Element::spacer().sticky_top_fade(0.0).sticky_top, None);
}

#[test]
fn popover_with_no_gap_starts_at_its_zero_height_anchor() {
    let root = |gap: Option<f32>| {
        let popover = Element::popover(
            Element::spacer().size(Some(100.0), Some(0.0)),
            Element::button("inside", "Inside").size(Some(100.0), Some(40.0)),
            "dismiss",
            PopoverPlacement::BottomStart,
        );
        Element::column(vec![
            Element::spacer().size(None, Some(30.0)),
            match gap {
                Some(gap) => popover.popover_gap(gap),
                None => popover,
            },
        ])
    };
    let top = |gap| {
        let mut tree = UiTree::new(root(gap));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 200.0));
        top_of(&frame, "inside").unwrap()
    };
    assert_eq!(top(None), 35.0);
    assert_eq!(top(Some(0.0)), 30.0);
    assert_eq!(top(Some(f32::NAN)), 35.0);
    assert_eq!(top(Some(-3.0)), 35.0);
}
