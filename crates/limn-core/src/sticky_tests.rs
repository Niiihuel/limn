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

/// Three sections whose passive headers are anchors `a0`, `a1` and `a2`.
fn anchored(sticky: bool) -> UiTree {
    let mut children = Vec::new();
    for section in 0..3 {
        let header = Element::text("Header").size(None, Some(20.0)).anchor(format!("a{section}"));
        children.push(if sticky { header.sticky_top() } else { header });
        children.extend((0..5).map(|_| Element::text("Row").size(None, Some(20.0))));
    }
    let mut tree = UiTree::new(Element::scroll("list", Element::column(children)));
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree
}

#[test]
fn a_passive_anchor_is_revealed_and_says_where_it_is() {
    let mut tree = anchored(false);
    // Laid out below the viewport, where nothing of it is painted.
    assert_eq!(tree.anchor_offset("a2"), Some(240.0));
    tree.reveal_with_margin("a2", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 160.0);
    assert_eq!(tree.anchor_offset("a2"), Some(80.0));
    assert_eq!(tree.anchor_offset("a0"), Some(-160.0));
    assert_eq!(tree.anchor_offset("missing"), None);
    // It is still no control: nothing to focus or click.
    assert!(tree.frame().regions.iter().all(|region| region.id == "list"));
}

fn nested_anchors(font_size: f32) -> Element {
    Element::scroll("list", Element::column(vec![
        Element::panel(Element::column(vec![
            Element::text("A paragraph before the title changes height with the viewport and font size.").font_size(font_size),
            Element::panel(Element::column(vec![
                Element::button("hidden-action", "Nested title").size(None, Some(24.0)).anchor("first"),
                Element::text("Another nested title").size(None, Some(24.0)).anchor("second"),
            ]).gap(12.0).padding(8.0)),
        ]).gap(16.0).padding(10.0)),
        Element::spacer().size(None, Some(2000.0)),
        Element::button("last", "Last row").size(None, Some(32.0)),
    ])).scrollbar_appearance(crate::ScrollbarAppearance::overlay())
}

#[test]
fn nested_anchors_have_current_geometry_on_the_first_deep_layout_without_painting_hidden_controls() {
    let mut tree = UiTree::new(nested_anchors(16.0));
    tree.set_reduced_motion(true);
    for (width, font_size) in [(240.0, 16.0), (120.0, 16.0), (120.0, 28.0)] {
        let mut reference = UiTree::new(nested_anchors(font_size));
        reference.set_reduced_motion(true);
        reference.layout(Size::new(width, 4000.0));
        let first = reference.anchor_offset("first").unwrap();
        let second = reference.anchor_offset("second").unwrap();

        tree.set_root(nested_anchors(font_size));
        tree.scroll_to_end("list");
        let frame = tree.layout(Size::new(width, 180.0));
        let offset = tree.scroll_offset("list");
        assert!(offset > second);
        assert!(frame.access.iter().any(|node| node.id == "last"));
        assert_eq!(tree.anchor_offset("first"), Some(first - offset));
        assert_eq!(tree.anchor_offset("second"), Some(second - offset));
        assert!(!frame.access.iter().any(|node| node.id == "hidden-action"));
        assert!(!frame.regions.iter().any(|region| region.id == "hidden-action"));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Nested title" || text == "Another nested title")));
    }
    tree.reveal_at_start("second", 8.0);
    tree.layout(Size::new(120.0, 180.0));
    assert_eq!(tree.anchor_offset("second"), Some(8.0));
    tree.set_root(Element::scroll("list", Element::text("Replacement without anchors")));
    tree.layout(Size::new(120.0, 180.0));
    assert_eq!(tree.anchor_offset("first"), None);
    assert_eq!(tree.anchor_offset("second"), None);
}

#[test]
fn an_anchor_in_a_fully_clipped_nested_scroll_still_uses_its_nearest_viewport() {
    let inner = Element::scroll("inner", Element::column(vec![
        Element::spacer().size(None, Some(40.0)),
        Element::text("Inner anchor").size(None, Some(20.0)).anchor("nested"),
        Element::spacer().size(None, Some(240.0)),
    ])).size(None, Some(100.0));
    let mut tree = UiTree::new(Element::scroll("outer", Element::column(vec![
        Element::spacer().size(None, Some(300.0)),
        Element::panel(inner),
        Element::spacer().size(None, Some(800.0)),
    ])));
    tree.set_reduced_motion(true);
    tree.scroll_to_end("outer");
    tree.scroll_to_end("inner");
    let frame = tree.layout(Size::new(200.0, 200.0));
    assert!(!frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "Inner anchor")));
    assert_eq!(tree.scroll_offset("inner"), 200.0);
    assert_eq!(tree.anchor_offset("nested"), Some(-160.0));
    let outer = tree.scroll_offset("outer");
    tree.reveal_at_start("nested", 0.0);
    let revealed = tree.layout(Size::new(200.0, 200.0));
    assert_eq!(tree.anchor_offset("nested"), Some(0.0));
    assert!(tree.scroll_offset("outer") < outer);
    assert!(revealed.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "Inner anchor")));
}

#[test]
fn revealing_a_pinned_anchor_scrolls_back_to_where_its_section_starts() {
    let mut tree = anchored(true);
    tree.handle_event(UiEvent::ScrollDirect { position: Point::new(5.0, 5.0), delta_y: -170.0 });
    tree.layout(SIZE);
    // Pinned at the top of the viewport, it answers for its place in the column.
    assert_eq!(tree.anchor_offset("a1"), Some(-50.0));
    tree.reveal_with_margin("a1", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 120.0);
    assert_eq!(tree.anchor_offset("a1"), Some(0.0));
}

#[test]
fn a_pinned_control_that_is_an_anchor_keeps_both_places() {
    let mut children = Vec::new();
    for section in 0..3 {
        children.push(
            Element::button(format!("h{section}"), "Header")
                .size(None, Some(20.0))
                .anchor(format!("a{section}"))
                .sticky_top(),
        );
        children.extend((0..5).map(|_| Element::text("Row").size(None, Some(20.0))));
    }
    let mut tree = UiTree::new(Element::scroll("list", Element::column(children)));
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree.handle_event(UiEvent::ScrollDirect { position: Point::new(5.0, 5.0), delta_y: -170.0 });
    tree.layout(SIZE);
    // The control is where it can be clicked; the anchor where its section starts.
    tree.reveal_with_margin("h1", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 170.0);
    tree.reveal_with_margin("a1", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 120.0);
}

#[test]
fn an_anchor_already_in_view_still_goes_to_the_start_when_asked_to() {
    let mut tree = anchored(false);
    tree.handle_event(UiEvent::ScrollDirect { position: Point::new(5.0, 5.0), delta_y: -60.0 });
    tree.layout(SIZE);
    assert_eq!(tree.anchor_offset("a1"), Some(60.0));
    // In view: revealing it moves nothing.
    tree.reveal_with_margin("a1", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 60.0);
    tree.reveal_at_start("a1", 10.0);
    tree.layout(SIZE);
    assert_eq!((tree.scroll_offset("list"), tree.anchor_offset("a1")), (110.0, Some(10.0)));
    // Back up to one above it, and no further than the content goes.
    tree.reveal_at_start("a0", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 0.0);
    tree.reveal_at_start("a2", 0.0);
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("list"), 240.0);
}

/// A page whose 20 px header fades its background in from 10 to 80 px of
/// scroll, over thirty 20 px rows.
fn page_under(header: Element) -> Element {
    let mut children = vec![header];
    children.extend((0..30).map(|row| Element::button(format!("r{row}"), "Row").size(None, Some(20.0))));
    Element::scroll("page", Element::column(children))
}

#[test]
fn a_background_that_fades_in_follows_the_scroll_past_the_pin_line() {
    let color = Color::rgb(80, 20, 40);
    let header = Element::text("Header")
        .size(None, Some(20.0))
        .background(color)
        .sticky_top_background_fade_in(10.0, 80.0);
    let fill = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color: fill, .. }
                if (fill.r, fill.g, fill.b) == (color.r, color.g, color.b) => Some((rect.y, fill.a)),
            _ => None,
        })
    };
    let said = |frame: &Frame| {
        frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Header"))
    };
    for (offset, alpha) in [(0.0, 0), (10.0, 0), (45.0, 128), (80.0, 255), (300.0, 255)] {
        let frame = scrolled(page_under(header.clone()), offset);
        assert_eq!(fill(&frame).map(|(_, a)| a).unwrap_or(0), alpha, "at {offset}");
        assert!(said(&frame), "the content is painted as it is at {offset}");
        if offset > 0.0 {
            assert_eq!(fill(&frame).map_or(0.0, |(y, _)| y), 0.0, "pinned at the top at {offset}");
        }
    }
    // Only the background: the same header without the option is opaque at rest.
    let opaque = scrolled(page_under(Element::text("Header").size(None, Some(20.0)).background(color).sticky_top()), 0.0);
    assert_eq!(fill(&opaque).map(|(_, a)| a), Some(255));
}

#[test]
fn a_fade_in_range_that_is_not_a_number_is_ignored() {
    for (from, to) in [(f32::NAN, 80.0), (10.0, f32::INFINITY), (-1.0, 80.0), (10.0, -5.0)] {
        let element = Element::spacer().sticky_top_background_fade_in(from, to);
        assert!(element.sticky_top.is_none() && element.kept.is_none(), "{from}, {to}");
    }
    let backwards = Element::spacer().sticky_top_background_fade_in(50.0, 10.0);
    assert_eq!(backwards.kept.and_then(|kept| kept.sticky_background), Some((50.0, 51.0)));
}
