use super::*;
use crate::{Component, ScrollArea, ScrollbarAppearance, ScrollbarTrack};

const SIZE: Size = Size::new(200.0, 120.0);

/// Rows in a scroll area with a rail 20 wide, so a button's share of its
/// square is larger than the default arrow.
fn rows(appearance: ScrollbarAppearance) -> UiTree {
    let rows = Element::column((0..40).map(|index| Element::text(format!("row {index}")).size(None, Some(24.0))).collect());
    let mut tree = UiTree::new(ScrollArea::new("rows", rows).appearance(appearance.rail_width(20.0)).render());
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree
}

/// The down button under the pointer: its square, its hover fill and the
/// fill's corner, and the box its arrow takes.
fn down_button(appearance: ScrollbarAppearance) -> (Rect, Rect, f32, Size) {
    let mut tree = rows(appearance);
    tree.handle_event(UiEvent::PointerMove(Point::new(190.0, 110.0)));
    let frame = tree.layout(SIZE);
    let button = frame.scrollbars["rows"].down_button.expect("a down button");
    let (fill, radius) = frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, radius, .. } if button.contains(Point::new(rect.x, rect.y)) => Some((*rect, *radius)),
        _ => None,
    }).expect("the hover fill");
    // The down arrow is the last one painted.
    let points = frame.commands.iter().rev().find_map(|command| match command {
        DrawCommand::FillTriangle { points, .. } => Some(*points),
        _ => None,
    }).expect("an arrow");
    let span = |along: fn(&Point) -> f32| {
        let (low, high) = points.iter().map(along).fold((f32::MAX, f32::MIN), |(low, high), value| (low.min(value), high.max(value)));
        high - low
    };
    (button, fill, radius, Size::new(span(|point| point.x), span(|point| point.y)))
}

#[test]
fn an_end_buttons_hover_fill_and_arrow_take_their_inset_corner_and_box_from_the_appearance() {
    let (button, fill, radius, arrow) = down_button(ScrollbarAppearance::default());
    assert_eq!(fill, button.inset(1.0));
    assert_eq!(radius, crate::tokens().radius.control);
    assert_eq!(arrow, Size::new(7.0, 4.0), "the default box, smaller than the button's share");
    let own = ScrollbarAppearance::default().button_inset(4.0).button_radius(0.0).arrow(Size::new(9.0, 5.0));
    let (button, fill, radius, arrow) = down_button(own);
    assert_eq!(fill, button.inset(4.0));
    assert_eq!(radius, 0.0);
    assert_eq!(arrow, Size::new(9.0, 5.0));
}

#[test]
fn the_track_shows_while_active_always_or_never_as_the_appearance_says() {
    let track = |appearance: ScrollbarAppearance, hovered: bool| {
        let mut tree = rows(appearance);
        if hovered {
            // Over the track, below the thumb.
            tree.handle_event(UiEvent::PointerMove(Point::new(190.0, 80.0)));
        }
        let frame = tree.layout(SIZE);
        let track = frame.scrollbars["rows"].track;
        frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { rect, .. } if *rect == track))
    };
    let default = ScrollbarAppearance::default();
    assert_eq!((track(default, false), track(default, true)), (false, true), "while active, by default");
    let always = default.track(ScrollbarTrack::Always);
    assert_eq!((track(always, false), track(always, true)), (true, true));
    let never = default.track(ScrollbarTrack::Never);
    assert_eq!((track(never, false), track(never, true)), (false, false));
}
