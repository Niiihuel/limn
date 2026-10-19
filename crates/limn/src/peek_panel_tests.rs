use std::time::{Duration, Instant};

use super::*;
use crate::{Button, DrawCommand, Frame, HitKind, Key, Rect, Size, UiAction, UiEvent, UiTree};

const WINDOW: Size = Size::new(400.0, 300.0);
/// Where the strip at the right edge rests.
const STRIP: Rect = Rect::new(360.0, 0.0, 40.0, 300.0);

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

/// A panel's content: a fill as wide as the open panel with a button on it.
fn content() -> Element {
    Element::column(vec![Button::new("inside", "Clear the queue", palette()).render()]).background(Color::rgb(200, 30, 30))
}

fn strip() -> PeekPanel {
    PeekPanel::new("peek", content(), palette()).accessibility_label("Show the queue")
}

/// The strip against the window's edge on its side, beside the main area.
fn tree(peek: PeekPanel, reduced: bool) -> UiTree {
    let left = matches!(peek.side, Side::Left);
    let main = Element::spacer().flex(1.0);
    let row = if left { vec![peek.render(), main] } else { vec![main, peek.render()] };
    let mut tree = UiTree::new(Element::row(row));
    tree.set_reduced_motion(reduced);
    tree
}

fn strip_rect(frame: &Frame) -> Rect {
    let regions: Vec<_> = frame.regions.iter().filter(|region| region.kind == HitKind::Button).collect();
    let [region] = regions.as_slice() else { panic!("the strip is the only button, got {regions:?}") };
    assert_eq!(region.id, "peek");
    region.rect
}

fn fills(frame: &Frame, rect: Rect) -> Vec<Color> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect: fill, color, .. } if *fill == rect => Some(*color),
            _ => None,
        })
        .collect()
}

/// The coverage of the veils over the content, as one opacity.
fn veil(frame: &Frame, rect: Rect) -> f32 {
    let clear = fills(frame, rect)
        .iter()
        .filter(|color| Color { a: 255, ..**color } == palette().surface && color.a < 255)
        .map(|color| 1.0 - f32::from(color.a) / 255.0)
        .product::<f32>();
    1.0 - clear
}

/// The chevrons that show.
fn chevron(frame: &Frame) -> Vec<(Rect, Icon, Color)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            // A chevron faded out is painted, transparent.
            DrawCommand::Icon { rect, icon, tint } if tint.a > 0 => Some((*rect, *icon, *tint)),
            _ => None,
        })
        .collect()
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, shift: false, ctrl: false }
}

fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

#[test]
fn the_strip_is_40_wide_at_the_edge_rounded_where_it_faces_the_window_over_a_veiled_glimpse() {
    let frame = tree(strip(), true).layout(WINDOW);
    assert_eq!(strip_rect(&frame), STRIP);
    let radius = crate::tokens().radius.panel;
    let corners = CornerRadii { top_left: radius, bottom_left: radius, top_right: 0.0, bottom_right: 0.0 };
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillCornerRect { rect, color, radii } if *rect == STRIP && *color == palette().surface && *radii == corners
    )));
    // The content is laid out at the open panel's width from the strip's edge, and clipped to the strip.
    let content = frame
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == Color::rgb(200, 30, 30)))
        .expect("the content is painted");
    let DrawCommand::FillRect { rect, .. } = &frame.commands[content] else { unreachable!() };
    assert_eq!((rect.x, rect.width), (STRIP.x, 320.0));
    let clip = frame.commands[..content].iter().rposition(|command| matches!(command, DrawCommand::PushClip(_))).unwrap();
    assert_eq!(frame.commands[clip], DrawCommand::PushClip(STRIP));
    assert!(matches!(frame.commands[clip + 1], DrawCommand::RoundClip { rect, radii } if rect == STRIP && radii == corners));
    // Almost opaque: the content barely shows.
    assert!((veil(&frame, STRIP) - 0.9).abs() < 0.01, "{}", veil(&frame, STRIP));
    // The content's controls are only painted: no pointer target, focus or node.
    assert!(frame.access.iter().all(|node| node.id != "inside"));
    let node = frame.access.iter().find(|node| node.id == "peek").unwrap();
    assert_eq!((node.role, node.name.as_str()), (Role::Button, "Show the queue"));
    let [(rect, Icon::ChevronLeft, tint)] = chevron(&frame)[..] else { panic!("one chevron, {:?}", chevron(&frame)) };
    assert_eq!((middle(rect), tint), (middle(STRIP), palette().muted));
}

#[test]
fn under_the_pointer_the_strip_reaches_out_past_12_px_and_settles_there_then_back() {
    let mut tree = tree(strip(), false);
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    tree.layout_at(WINDOW, start);
    tree.handle_event(UiEvent::PointerMove(middle(STRIP)));
    let widths = |tree: &mut UiTree, from: u64| -> Vec<f32> {
        (0..=90).map(|step| strip_rect(&tree.layout_at(WINDOW, at(from + step * 8))).width).collect()
    };
    let out = widths(&mut tree, 0);
    // It starts from rest and keeps its far edge at the window's.
    assert_eq!(out[0], 40.0);
    let peak = out.iter().copied().fold(0.0, f32::max);
    assert!(peak > 52.5 && peak < 55.0, "it overshoots a little: {peak}");
    assert_eq!(out[90], 52.0, "{out:?}");
    let rect = strip_rect(&tree.layout_at(WINDOW, at(720)));
    assert_eq!(rect.x + rect.width, WINDOW.width);
    // The pointer over the part it reached keeps it out.
    tree.handle_event(UiEvent::PointerMove(Point::new(STRIP.x - 6.0, 150.0)));
    assert_eq!(strip_rect(&tree.layout_at(WINDOW, at(900))).width, 52.0);
    tree.handle_event(UiEvent::PointerLeave);
    let back = widths(&mut tree, 1000);
    assert!(back.iter().copied().fold(f32::MAX, f32::min) < 40.0, "it springs back past its rest: {back:?}");
    assert_eq!(back[90], 40.0);
}

#[test]
fn under_the_pointer_the_veil_lifts_and_the_chevron_comes_to_full_ink() {
    let mut tree = tree(strip(), true);
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::PointerMove(middle(STRIP)));
    let frame = tree.layout(WINDOW);
    let out = Rect::new(348.0, 0.0, 52.0, 300.0);
    assert_eq!(strip_rect(&frame), out);
    assert!((veil(&frame, out) - 0.8).abs() < 0.01, "{}", veil(&frame, out));
    let [(rect, Icon::ChevronLeft, tint)] = chevron(&frame)[..] else { panic!("one chevron, {:?}", chevron(&frame)) };
    // It travelled with the strip's edge.
    assert_eq!((middle(rect), tint), (middle(out), palette().text));
}

#[test]
fn reduced_motion_reaches_out_and_back_at_once() {
    let mut tree = tree(strip(), true);
    let start = Instant::now();
    tree.layout_at(WINDOW, start);
    tree.handle_event(UiEvent::PointerMove(middle(STRIP)));
    assert_eq!(strip_rect(&tree.layout_at(WINDOW, start)).width, 52.0);
    tree.handle_event(UiEvent::PointerLeave);
    assert_eq!(strip_rect(&tree.layout_at(WINDOW, start)), STRIP);
}

#[test]
fn a_click_enter_or_space_reports_the_strip_and_focus_reaches_out_as_the_pointer_does() {
    let mut tree = tree(strip(), true);
    tree.layout(WINDOW);
    tree.handle_event(UiEvent::PointerDown(middle(STRIP)));
    assert_eq!(tree.handle_event(UiEvent::PointerUp(middle(STRIP))), vec![UiAction::Clicked { id: "peek".into() }]);
    tree.handle_event(UiEvent::PointerLeave);
    let mut tree = self::tree(strip(), true);
    tree.layout(WINDOW);
    // Tab stops on the strip alone: the button in its content is out of reach.
    tree.handle_event(key(Key::Tab));
    assert_eq!(tree.focused_id(), Some("peek"));
    let frame = tree.layout(WINDOW);
    assert_eq!(strip_rect(&frame).width, 52.0);
    assert_eq!(chevron(&frame)[0].2, palette().text);
    tree.handle_event(key(Key::Tab));
    assert_eq!(tree.focused_id(), Some("peek"));
    for pressed in [Key::Enter, Key::Space] {
        assert_eq!(tree.handle_event(key(pressed)), vec![UiAction::Clicked { id: "peek".into() }]);
    }
}

#[test]
fn on_the_left_it_reaches_right_with_its_right_corners_rounded() {
    let mut tree = tree(strip().side(Side::Left), true);
    let frame = tree.layout(WINDOW);
    assert_eq!(strip_rect(&frame), Rect::new(0.0, 0.0, 40.0, 300.0));
    let radius = crate::tokens().radius.panel;
    let corners = CornerRadii { top_right: radius, bottom_right: radius, top_left: 0.0, bottom_left: 0.0 };
    assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::FillCornerRect { radii, .. } if *radii == corners)));
    assert_eq!(chevron(&frame)[0].1, Icon::ChevronRight);
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 150.0)));
    assert_eq!(strip_rect(&tree.layout(WINDOW)), Rect::new(0.0, 0.0, 52.0, 300.0));
}

#[test]
fn widths_that_are_not_numbers_are_ignored_and_negative_ones_are_zero() {
    let laid = |peek: PeekPanel| {
        let frame = tree(peek, true).layout(WINDOW);
        let width = frame.regions.iter().find(|region| region.id == "peek").map_or(0.0, |region| region.rect.width);
        let content = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == Color::rgb(200, 30, 30) => Some(rect.width),
            _ => None,
        });
        (width, content)
    };
    assert_eq!(laid(strip().width(f32::NAN).content_width(f32::INFINITY)), (40.0, Some(320.0)));
    assert_eq!(laid(strip().width(56.0).content_width(280.0)), (56.0, Some(280.0)));
    assert_eq!(laid(strip().width(-4.0).content_width(-1.0)).0, 0.0);
}

#[test]
fn reaching_out_it_takes_the_room_and_the_main_area_beside_it_gives_way() {
    let main = Element::clickable("main", Element::spacer()).flex(1.0);
    let mut tree = UiTree::new(Element::row(vec![main, strip().render()]));
    let start = Instant::now();
    let rect = |frame: &Frame, id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
    let rest = tree.layout_at(WINDOW, start);
    assert_eq!(rect(&rest, "main").width, STRIP.x, "at rest the main area runs to the strip");
    tree.handle_event(UiEvent::PointerMove(middle(STRIP)));
    // The frame the pointer arrives in sets the reach off; it has settled by 900 ms.
    tree.layout_at(WINDOW, start);
    let settled = tree.layout_at(WINDOW, start + Duration::from_millis(900));
    let (main, reached) = (rect(&settled, "main"), rect(&settled, "peek"));
    assert_eq!((reached.x, reached.width), (STRIP.x - 12.0, 52.0), "the strip reaches 12 px into the row, against the edge");
    assert_eq!(main.x + main.width, reached.x, "the main area gives way instead of being covered");
}
