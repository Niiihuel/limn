use super::*;
use crate::{Component, ComponentPalette, IconButton, IconKeyframe, IconMotion, Tooltip, TooltipShape, TooltipSpec};
use std::time::Duration;

const WINDOW: Size = Size::new(300.0, 200.0);
/// The fill of every hint here.
const FILL: Color = Color::rgb(10, 10, 12);
/// The ink of what a hint of the component's own holds.
const INK: Color = Color::rgb(200, 30, 30);

fn hint() -> Tooltip {
    Tooltip::new(
        Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
        "mark",
        "Server",
        ComponentPalette::neutral_dark(),
    )
    .background(FILL)
}

/// A block of ink `width` by `height`: content a component built.
fn block(width: f32, height: f32) -> Element {
    Element::spacer().size(Some(width), Some(height)).background(INK)
}

/// `hint` with its anchor `left` and `top` into the window.
fn at(hint: Element, left: f32, top: f32) -> Element {
    Element::column(vec![
        Element::spacer().size(None, Some(top)),
        Element::row(vec![Element::spacer().size(Some(left), None), hint, Element::spacer().flex(1.0)]),
        Element::spacer().flex(1.0),
    ])
}

/// The window with `hint` at `left`, `top`, its anchor focused so that it shows.
fn shown(hint: Element, left: f32, top: f32) -> Frame {
    let mut tree = UiTree::new(at(hint, left, top));
    // Every character 8 wide, so a title's width is known.
    tree.set_text_measurer(|text: &str, _: f32, _: FontFamily| text.chars().count() as f32 * 8.0);
    tree.set_reduced_motion(true);
    tree.layout(WINDOW);
    assert!(tree.focus("mark"));
    tree.layout(WINDOW)
}

/// What the hint paints of itself: its shadow, body, outline and pointer.
fn surface(frame: &Frame) -> Vec<DrawCommand> {
    let start = frame
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::Shadow { .. }))
        .expect("a hint");
    frame.commands[start..]
        .iter()
        .take_while(|command| !matches!(command, DrawCommand::PushClip(_)))
        .cloned()
        .collect()
}

fn body_of(frame: &Frame) -> Rect {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == FILL => Some(*rect),
            _ => None,
        })
        .expect("the body")
}

/// Where the content's ink is painted, and how opaque.
fn ink(frame: &Frame) -> Vec<(Rect, u8)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if Color { a: INK.a, ..*color } == INK => Some((*rect, color.a)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_hint_of_the_components_own_is_sized_by_its_content_and_holds_it_inside_its_padding() {
    let frame = shown(hint().shortcut("Ctrl+B").description("More").content(block(60.0, 30.0)).render(), 134.0, 120.0);
    let body = body_of(&frame);
    // 8 at the sides and 6 above and below, as a built-in hint of more than one line.
    assert_eq!((body.width, body.height), (76.0, 42.0));
    assert_eq!(ink(&frame), [(Rect::new(body.x + 8.0, body.y + 6.0, 60.0, 30.0), 255)]);
    // The built-in layout steps aside: no title, shortcut or description.
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));

    // Text of its own wraps within the hint's width, less its padding.
    let words = "one two three four five";
    let frame = shown(hint().max_width(100.0).content(Element::text(words)).render(), 134.0, 120.0);
    let body = body_of(&frame);
    let lines: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, position, .. } => Some((text.clone(), *position)),
            _ => None,
        })
        .collect();
    assert!(lines.len() > 1, "{lines:?}");
    assert!(body.width <= 100.0, "{body:?}");
    for (text, position) in &lines {
        assert_eq!(position.x, body.x + 8.0, "{text}");
        assert!(position.x + text.chars().count() as f32 * 8.0 <= body.x + body.width - 8.0, "{text}");
    }
}

#[test]
fn content_of_the_components_own_takes_no_pointer_focus_or_reader() {
    let view = |content: Option<Element>| {
        let hint = match content {
            Some(content) => hint().content(content),
            None => hint(),
        };
        Element::row(vec![hint.render(), Element::button("after", "After")])
    };
    let inside = Element::column(vec![Element::button("inside", "Press"), Element::text("Read me")]);
    let built_in = shown(view(None), 134.0, 120.0);
    let own = shown(view(Some(inside.clone())), 134.0, 120.0);
    // It is painted, and that is all.
    for said in ["Press", "Read me"] {
        assert!(own.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == said)), "{said}");
    }
    assert_eq!(own.regions, built_in.regions);
    assert_eq!(own.access, built_in.access);
    assert!(!own.access.iter().any(|node| node.id == "inside"));
    assert!(own.duplicate_ids().is_empty());
    // The title is still what the platform is told: it names an icon alone.
    let mark = own.access.iter().find(|node| node.id == "mark").expect("the anchor");
    assert_eq!(mark.name, "Server");
    // Tab goes from the anchor to the next control, past what the hint holds.
    let mut tree = UiTree::new(at(view(Some(inside)), 134.0, 120.0));
    tree.layout(WINDOW);
    assert!(tree.focus("mark"));
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
    tree.layout(WINDOW);
    assert_eq!(tree.focused_id(), Some("after"));
}

#[test]
fn what_content_of_the_components_own_floats_takes_no_pointer_focus_or_reader() {
    let view = |content: Option<Element>| {
        let hint = match content {
            Some(content) => hint().content(content),
            None => hint(),
        };
        Element::row(vec![hint.render(), Element::button("after", "After")])
    };
    let built_in = shown(view(None), 134.0, 120.0);
    // Each floats a button to the layout's top layer, past the hint's body.
    let floats = [
        ("an open popover", Element::popover(block(40.0, 20.0), Element::button("inside", "Press"), "inside-dismiss", PopoverPlacement::Bottom)),
        ("an overlay", Element::overlay(block(40.0, 20.0), Element::button("inside", "Press"))),
    ];
    for (float, content) in floats {
        let own = shown(view(Some(content.clone())), 134.0, 120.0);
        assert!(!own.regions.iter().any(|region| region.id == "inside"), "{float}");
        assert!(!own.access.iter().any(|node| node.id == "inside"), "{float}");
        assert_eq!(own.regions, built_in.regions, "{float}");
        assert_eq!(own.access, built_in.access, "{float}");
        // Tab goes from the anchor to the next control, past what it floats.
        let mut tree = UiTree::new(at(view(Some(content)), 134.0, 120.0));
        tree.layout(WINDOW);
        assert!(tree.focus("mark"));
        tree.layout(WINDOW);
        tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
        tree.layout(WINDOW);
        assert_eq!(tree.focused_id(), Some("after"), "{float}");
    }
}

#[test]
fn a_hint_of_the_components_own_is_placed_flipped_and_clamped_as_a_built_in_one_of_its_size() {
    // "Server" is 48 wide on one line: a body 64 by 24, as content 48 by 12
    // with 8 at its sides and 6 over and under it.
    let mut bodies = Vec::new();
    for (left, top, placement) in [
        (134.0, 84.0, TooltipPlacement::Above),
        (0.0, 0.0, TooltipPlacement::Above),
        (268.0, 84.0, TooltipPlacement::Right),
        (134.0, 168.0, TooltipPlacement::Below),
    ] {
        let built_in = shown(hint().placement(placement).render(), left, top);
        let own = shown(hint().placement(placement).content(block(48.0, 12.0)).render(), left, top);
        assert_eq!(surface(&own), surface(&built_in), "at {left}, {top}, {placement:?}");
        let body = body_of(&own);
        assert_eq!(ink(&own), [(Rect::new(body.x + 8.0, body.y + 6.0, 48.0, 12.0), 255)]);
        bodies.push(body);
    }
    let [middle, corner, right, bottom] = bodies[..] else { unreachable!() };
    assert_eq!(middle.y + middle.height, 84.0 - TOOLTIP_GAP, "above, as asked");
    // No room above: below, kept 8 from the window's left edge.
    assert_eq!((corner.x, corner.y), (8.0, 32.0 + TOOLTIP_GAP));
    // No room on the right: on the left.
    assert_eq!(right.x + right.width, 268.0 - TOOLTIP_GAP);
    // No room below: above.
    assert_eq!(bottom.y + bottom.height, 168.0 - TOOLTIP_GAP);
}

#[test]
fn a_hint_of_the_components_own_takes_the_shapes_radius_padding_shadow_pointer_and_entrance() {
    let shape = TooltipShape {
        radius: 8.0,
        padding_x: 12.0,
        padding_y: 8.0,
        shadow_offset: Point::new(0.0, 12.0),
        shadow_blur: 24.0,
        caret: Size::new(16.0, 10.0),
    };
    let shaped = || hint().placement(TooltipPlacement::Below).shape(shape);
    let built_in = shown(shaped().render(), 134.0, 40.0);
    // A block as tall as the built-in hint's line makes a body of the same size.
    let line = body_of(&built_in).height - 16.0;
    let own = shown(shaped().content(block(48.0, line)).render(), 134.0, 40.0);
    assert_eq!(surface(&own), surface(&built_in));
    let body = body_of(&own);
    assert_eq!(ink(&own), [(Rect::new(body.x + 12.0, body.y + 8.0, 48.0, line), 255)]);
    assert!(own.commands.iter().any(|command| matches!(command,
        DrawCommand::Shadow { shadow, radius, .. }
            if *radius == 8.0 && shadow.offset == Point::new(0.0, 12.0) && shadow.blur == 24.0
    )));
    // The pointer's depth and 2 px off the anchor.
    assert_eq!(body.y, 40.0 + 32.0 + 12.0);
    // It comes in with the body, faded at first.
    let mut tree = UiTree::new(at(shaped().content(block(48.0, line)).render(), 134.0, 40.0));
    let start = Instant::now();
    tree.layout_at(WINDOW, start);
    assert!(tree.focus("mark"));
    let early = tree.layout_at(WINDOW, start + Duration::from_millis(20));
    assert!(ink(&early).iter().all(|(_, alpha)| *alpha < 255), "{:?}", ink(&early));
    let late = tree.layout_at(WINDOW, start + Duration::from_secs(1));
    assert!(ink(&late).iter().all(|(_, alpha)| *alpha == 255));
}

#[test]
fn a_hint_without_content_of_its_own_is_the_built_in_one() {
    let built_in = shown(hint().shortcut("Ctrl+B").description("More").render(), 134.0, 120.0);
    let mut element = hint().shortcut("Ctrl+B").description("More").content(block(60.0, 30.0)).render();
    let ElementKind::Tooltip { content, .. } = &mut element.kind else {
        unreachable!("a tooltip renders a tooltip");
    };
    assert!(content.take().is_some());
    // The title, shortcut and description were kept, and drawn as before.
    assert_eq!(shown(element, 134.0, 120.0).commands, built_in.commands);
}

#[test]
fn a_buttons_hint_carries_content_of_its_own() {
    let button = IconButton::new("mark", Icon::PanelLeft, ComponentPalette::neutral_dark())
        .tooltip_with(TooltipSpec::new("Server").content(block(60.0, 30.0)))
        .render();
    let frame = shown(button, 134.0, 120.0);
    assert_eq!(ink(&frame).len(), 1);
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "Server")));
}

#[test]
fn content_of_the_components_own_keeps_its_state_while_shown_and_loses_it_when_gone() {
    // A download arrow that bobs: 2 px up at 0 ms, home at 1 s, up at 2 s.
    let bob = IconMotion::new()
        .keyframe(IconKeyframe::at(0).offset(Point::new(0.0, -2.0)))
        .keyframe(IconKeyframe::at(1000))
        .keyframe(IconKeyframe::at(2000).offset(Point::new(0.0, -2.0)));
    let arrow = Element::icon(Icon::ArrowDown).size(Some(16.0), Some(16.0)).icon_loop("hint:bob", bob);
    let mut tree = UiTree::new(at(hint().content(arrow).render(), 134.0, 120.0));
    let start = Instant::now();
    let ms = |ms: u64| start + Duration::from_millis(ms);
    let y = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon { icon: Icon::ArrowDown, rect, .. } => Some(rect.y),
                _ => None,
            })
            .expect("the arrow")
    };
    tree.layout_at(WINDOW, ms(0));
    assert!(tree.focus("mark"));
    let up = y(&tree.layout_at(WINDOW, ms(0)));
    let halfway = y(&tree.layout_at(WINDOW, ms(500)));
    let home = y(&tree.layout_at(WINDOW, ms(1000)));
    assert_eq!((up - home, halfway - home), (-2.0, -1.0), "it plays, kept by its ID from frame to frame");
    assert!(tree.is_animating(), "and asks for its frames");
    tree.clear_focus();
    tree.layout_at(WINDOW, ms(1000));
    tree.layout_at(WINDOW, ms(2000));
    assert!(!tree.is_animating(), "nothing asks for frames once the hint is gone");
    // Shown again, it starts over: its state went with the hint.
    assert!(tree.focus("mark"));
    assert_eq!(y(&tree.layout_at(WINDOW, ms(3000))) - home, -2.0);
}
