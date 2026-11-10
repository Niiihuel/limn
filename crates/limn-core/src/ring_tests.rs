//! The focus ring takes its look from the tokens, or from its element; a
//! coach mark built on the engine alone takes its ring and caret from its
//! anchor.
use crate::{
    with_tokens, Color, CornerRadii, DrawCommand, Element, FontFamily, Frame, Key, Rect, Ring, Size, TextSpan,
    Tokens, UiEvent, UiTree,
};

const BLUE: Color = Color::rgb(0, 120, 255);
const RED: Color = Color::rgb(220, 30, 30);

/// `root` in a padded column, laid out after one Tab, so its first control
/// has the keyboard focus.
fn focused(root: Element) -> Frame {
    let size = Size::new(400.0, 300.0);
    let mut tree = UiTree::new(Element::column(vec![root]).padding(10.0));
    tree.layout(size);
    tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
    tree.layout(size)
}

/// Every outline the frame strokes: its rectangle, color, width, and its
/// radius or its four radii.
fn rings(frame: &Frame) -> Vec<(Rect, Color, f32, [f32; 4])> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::StrokeRect { rect, color, width, radius } => Some((*rect, *color, *width, [*radius; 4])),
            DrawCommand::StrokeCornerRect { rect, color, width, radii } => Some((
                *rect,
                *color,
                *width,
                [radii.top_left, radii.top_right, radii.bottom_right, radii.bottom_left],
            )),
            _ => None,
        })
        .collect()
}

fn button() -> Element {
    Element::button("b", "Send").size(Some(80.0), Some(30.0)).radius(6.0)
}

fn tokens(ring: Ring) -> Tokens {
    Tokens { focus_ring: ring, ..Tokens::DEFAULT }
}

const BOUNDS: Rect = Rect::new(10.0, 10.0, 80.0, 30.0);

#[test]
fn the_tokens_ring_sets_the_focus_rings_color_and_width() {
    let gray = Color::rgb(170, 170, 170);
    assert_eq!(Ring::default(), Tokens::DEFAULT.focus_ring);
    assert_eq!(rings(&focused(button())), vec![(BOUNDS, gray, 2.0, [6.0; 4])]);
    let blue = Ring { color: BLUE, width: 3.0, ..Ring::default() };
    let frame = with_tokens(tokens(blue), || focused(button()));
    assert_eq!(rings(&frame), vec![(BOUNDS, BLUE, 3.0, [6.0; 4])]);
    // Still capped at half the control, as the library's ring is.
    let frame = with_tokens(tokens(Ring { width: 40.0, ..blue }), || focused(button()));
    assert_eq!(rings(&frame), vec![(BOUNDS, BLUE, 15.0, [6.0; 4])]);
}

#[test]
fn an_elements_own_ring_wins_over_the_tokens() {
    let own = Ring { color: RED, width: 1.0, ..Ring::default() };
    let blue = tokens(Ring { color: BLUE, ..Ring::default() });
    let frame = with_tokens(blue, || focused(button().focus_ring_style(own)));
    assert_eq!(rings(&frame), vec![(BOUNDS, RED, 1.0, [6.0; 4])]);
    // A link in formatted text takes its element's ring too.
    let link = Element::formatted_text(vec![
        TextSpan::new("Read the ", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("guide", Color::WHITE, 14.0, FontFamily::Sans).action("link"),
    ])
    .focus_ring_style(own);
    let frame = with_tokens(blue, || focused(link));
    let found = rings(&frame);
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].1, found[0].2), (RED, 1.0));
    // Turned off, it is not drawn, whoever styled it.
    let frame = with_tokens(blue, || focused(button().focus_ring_style(own).focus_ring(false)));
    assert!(rings(&frame).is_empty());
}

#[test]
fn an_offset_grows_the_ring_and_the_corners_it_follows() {
    let out = Ring { offset: 3.0, ..Ring::default() };
    let frame = focused(button().focus_ring_style(out));
    let grown = Rect::new(7.0, 7.0, 86.0, 36.0);
    assert_eq!(rings(&frame), vec![(grown, Color::rgb(170, 170, 170), 2.0, [9.0; 4])]);
    let corners = CornerRadii { top_left: 1.0, top_right: 2.0, bottom_right: 3.0, bottom_left: 4.0 };
    let frame = focused(button().corner_radii(corners).focus_ring_style(out));
    assert_eq!(rings(&frame)[0].3, [4.0, 5.0, 6.0, 7.0]);
    // A radius of its own is the ring's, on every corner.
    let frame = focused(button().corner_radii(corners).focus_ring_style(Ring { radius: Some(1.0), ..out }));
    assert_eq!(rings(&frame), vec![(grown, Color::rgb(170, 170, 170), 2.0, [1.0; 4])]);
}

#[test]
fn numbers_that_cannot_be_painted_are_made_safe() {
    for width in [f32::NAN, f32::INFINITY, -2.0] {
        let frame = focused(button().focus_ring_style(Ring { width, ..Ring::default() }));
        assert!(rings(&frame).is_empty(), "{width}");
    }
    for offset in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let frame = with_tokens(tokens(Ring { offset, ..Ring::default() }), || focused(button()));
        assert_eq!(rings(&frame), vec![(BOUNDS, Color::rgb(170, 170, 170), 2.0, [6.0; 4])], "{offset}");
    }
    for radius in [f32::NAN, f32::INFINITY, -4.0] {
        let frame = focused(button().focus_ring_style(Ring { radius: Some(radius), ..Ring::default() }));
        assert_eq!(rings(&frame)[0].3, [0.0; 4], "{radius}");
    }
}

/// A coach mark built on the engine alone: a bubble below the button
/// `target`, painted in blue.
fn coach(ring: Option<Ring>, caret: Option<Size>) -> Frame {
    use crate::{CoachAlign, CoachAnchor, CoachSide, PopoverPlacement};
    let base = Element::column(vec![Element::button("target", "Target").size(Some(80.0), Some(30.0)).radius(5.0)])
        .padding(20.0);
    let bubble = Element::panel(Element::text("Here")).size(Some(200.0), Some(60.0)).background(BLUE);
    let root = Element::popover(base, bubble, "tip", PopoverPlacement::BottomStart).popover_coach(CoachAnchor {
        targets: vec!["target".into()],
        side: CoachSide::Below,
        align: CoachAlign::Start,
        key: "step".into(),
        fill: BLUE,
        dim: None,
        back: None,
        next: None,
        ring,
        caret,
    });
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(Size::new(400.0, 300.0))
}

fn region(frame: &Frame, id: &str) -> Rect {
    frame.regions.iter().find(|region| region.id == id).map(|region| region.rect).unwrap()
}

fn carets(frame: &Frame) -> Vec<[crate::Point; 3]> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillTriangle { points, color } if *color == BLUE => Some(*points),
            _ => None,
        })
        .collect()
}

#[test]
fn a_coach_mark_draws_a_ring_and_a_caret_of_its_own_and_stands_off_by_them() {
    let ring = Ring { color: RED, width: 4.0, offset: 6.0, radius: None };
    let frame = coach(Some(ring), Some(Size::new(20.0, 10.0)));
    let (target, bubble) = (region(&frame, "target"), region(&frame, "popover_surface:tip"));
    let grown = Rect::new(target.x - 6.0, target.y - 6.0, target.width + 12.0, target.height + 12.0);
    // The target's own radius, grown with the ring.
    assert_eq!(rings(&frame), vec![(grown, RED, 4.0, [11.0; 4])]);
    assert_eq!(bubble.y, target.y + target.height + 6.0 + 4.0 + 10.0);
    let [[a, b, tip]] = carets(&frame)[..] else { panic!("one caret") };
    assert_eq!((b.x - a.x, tip.y), (20.0, bubble.y - 10.0));
    assert_eq!(tip.x, target.x + target.width / 2.0);
    let frame = coach(Some(Ring { radius: Some(2.0), ..ring }), None);
    assert_eq!(rings(&frame)[0].3, [2.0; 4]);
    // The library's caret, 14 by 7, and the bubble 6 + 4 + 7 off.
    let [[a, b, tip]] = carets(&frame)[..] else { panic!("one caret") };
    let bubble = region(&frame, "popover_surface:tip");
    assert_eq!((b.x - a.x, tip.y), (14.0, bubble.y - 7.0));
    assert_eq!(bubble.y, target.y + target.height + 17.0);
}

#[test]
fn a_coach_mark_without_a_caret_draws_none_and_bad_numbers_are_made_safe() {
    let frame = coach(None, Some(Size::new(0.0, 7.0)));
    let (target, bubble) = (region(&frame, "target"), region(&frame, "popover_surface:tip"));
    assert!(carets(&frame).is_empty());
    // The default ring, 2 px of the fill 3 px out, and the bubble only that far off.
    assert_eq!(rings(&frame).iter().map(|ring| (ring.1, ring.2)).collect::<Vec<_>>(), vec![(BLUE, 2.0)]);
    assert_eq!(bubble.y, target.y + target.height + 3.0 + 2.0);
    let bad = Ring { color: RED, width: f32::NAN, offset: f32::INFINITY, radius: Some(f32::NAN) };
    let frame = coach(Some(bad), Some(Size::new(f32::NAN, 7.0)));
    let (target, bubble) = (region(&frame, "target"), region(&frame, "popover_surface:tip"));
    assert!(rings(&frame).iter().all(|ring| ring.1 != RED));
    assert!(carets(&frame).is_empty());
    assert_eq!(bubble.y, target.y + target.height);
}
