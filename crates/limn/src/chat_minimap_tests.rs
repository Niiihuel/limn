use std::time::{Duration, Instant};

use super::*;
use crate::{DrawCommand, Frame, Key, Point, Size, UiAction, UiEvent, UiTree};

const SIZE: Size = Size::new(400.0, 300.0);

/// A rail of `count` turns at the top left, the second one in view.
fn tree(count: usize) -> UiTree {
    let marks = (0..count)
        .map(|index| {
            ChatMinimapMark::new(format!("turn-{index}"), format!("Pregunta {index}"))
                .preview(format!("Respuesta {index}"))
        })
        .collect();
    let rail = ChatMinimap::new("rail", marks, ComponentPalette::neutral_dark())
        .active("turn-1")
        .render();
    UiTree::new(Element::row(vec![rail, Element::spacer().flex(1.0)]))
}

/// The length of each mark, from the top.
fn lengths(frame: &Frame) -> Vec<f32> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, .. } if rect.x == 0.0 && rect.height == 2.0 => {
                Some(rect.width)
            }
            _ => None,
        })
        .collect()
}

fn shows(frame: &Frame, text: &str) -> bool {
    format!("{:?}", frame.commands).contains(text)
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, shift: false, ctrl: false }
}

#[test]
fn marks_rest_short_and_the_one_in_view_stands_out() {
    let frame = tree(4).layout(SIZE);
    assert_eq!(lengths(&frame), [8.0, 13.4, 8.0, 8.0]);
    assert!(!shows(&frame, "Pregunta"));
}

#[test]
fn marks_step_down_from_the_one_under_the_pointer_which_shows_its_turn_at_once() {
    let mut tree = tree(12);
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree.handle_event(UiEvent::PointerMove(Point::new(4.0, 55.0)));
    let frame = tree.layout(SIZE);
    let lengths = lengths(&frame);
    assert_eq!(lengths[5], 26.0);
    assert!(lengths[5..=10].windows(2).all(|pair| pair[0] > pair[1]), "{lengths:?}");
    assert_eq!(lengths[10..], [8.0, 8.0]);
    // Above it the stairs are the same, down to the mark in view's own length.
    assert_eq!(lengths[2..5], lengths[6..9].iter().rev().copied().collect::<Vec<_>>());
    assert_eq!(lengths[1], 13.4);
    assert!(shows(&frame, "Pregunta 5") && shows(&frame, "Respuesta 5"));

    tree.handle_event(UiEvent::PointerLeave);
    let frame = tree.layout(SIZE);
    assert_eq!(self::lengths(&frame)[5], 8.0);
    assert!(!shows(&frame, "Pregunta"));
}

#[test]
fn a_mark_grows_on_a_spring() {
    let mut tree = tree(12);
    tree.layout(SIZE);
    tree.handle_event(UiEvent::PointerMove(Point::new(4.0, 55.0)));
    let start = Instant::now();
    assert_eq!(lengths(&tree.layout_at(SIZE, start))[5], 8.0);
    assert!(tree.is_animating());
    let midway = lengths(&tree.layout_at(SIZE, start + Duration::from_millis(40)));
    assert!(midway[5] > 8.0 && midway[5] < 26.0, "{midway:?}");
    assert!(midway[6] > 8.0 && midway[6] < midway[5], "{midway:?}");
    let settled = lengths(&tree.layout_at(SIZE, start + Duration::from_secs(1)));
    assert_eq!(settled[5], 26.0);
}

#[test]
fn a_click_reports_the_mark() {
    let mut tree = tree(12);
    tree.layout(SIZE);
    let point = Point::new(4.0, 35.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        [UiAction::Clicked { id: "turn-3".into() }]
    );
}

#[test]
fn up_and_down_walk_the_rail_and_stop_at_its_ends() {
    let mut tree = tree(12);
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    assert!(tree.focus("turn-10"));
    assert_eq!(
        tree.handle_event(key(Key::ArrowDown)),
        [UiAction::Clicked { id: "turn-11".into() }]
    );
    let frame = tree.layout(SIZE);
    assert_eq!(lengths(&frame)[11], 26.0);
    assert!(shows(&frame, "Pregunta 11"));
    assert_eq!(tree.handle_event(key(Key::ArrowDown)), []);
    assert_eq!(tree.focused_id(), Some("turn-11"));
    assert_eq!(
        tree.handle_event(key(Key::Home)),
        [UiAction::Clicked { id: "turn-0".into() }]
    );
    assert_eq!(tree.handle_event(key(Key::ArrowUp)), []);
}

#[test]
fn marks_sit_closer_when_they_do_not_all_fit() {
    let mut tree = tree(100);
    let frame = tree.layout(SIZE);
    let last = frame.regions.iter().find(|region| region.id == "turn-99").unwrap();
    assert_eq!((last.rect.height, last.rect.y + last.rect.height), (3.0, 300.0));
    assert_eq!(lengths(&frame).len(), 100);
}

#[test]
fn a_turn_reads_as_one_line_cut_with_an_ellipsis() {
    assert_eq!(clipped("  uno\n\ndos  tres ", 72), "uno dos tres");
    assert_eq!(clipped("abcdefgh", 5), "abcd…");
}
