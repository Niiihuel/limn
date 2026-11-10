//! The focus ring takes its look from the tokens, or from its element.
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
