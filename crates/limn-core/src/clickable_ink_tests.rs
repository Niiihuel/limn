//! A clickable can paint what it holds in its hover and pressed inks, and
//! draw its ring under what it holds, as a button does with its label.
use crate::{Color, DrawCommand, Element, Frame, Icon, Key, Point, Size, UiEvent, UiTree};
use web_time::{Duration, Instant};

const INK: Color = Color::rgb(150, 150, 150);
const HOVER: Color = Color::rgb(250, 120, 0);
const PRESSED: Color = Color::rgb(0, 200, 120);
const OTHER: Color = Color::rgb(30, 90, 250);
const SIZE: Size = Size::new(300.0, 60.0);
const INSIDE: Point = Point::new(20.0, 15.0);

fn row(lends: bool) -> Element {
    let clickable = Element::clickable(
        "open",
        Element::row(vec![
            Element::icon(Icon::Folder).foreground(INK),
            Element::text("Open").foreground(INK),
            Element::text("3").foreground(OTHER),
        ])
        .gap(4.0),
    )
    .foreground(INK)
    .hover_foreground(HOVER)
    .pressed_foreground(PRESSED)
    .size(Some(160.0), Some(30.0));
    let clickable = if lends { clickable.content_takes_ink() } else { clickable };
    Element::column(vec![clickable, Element::spacer().flex(1.0)])
}

/// The inks of the icon and of each text, in the order they are painted.
fn inks(frame: &Frame) -> Vec<Color> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Icon { tint, .. } => Some(*tint),
            DrawCommand::Text { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

#[test]
fn what_a_clickable_holds_takes_its_hover_and_pressed_ink_only_when_it_says_so() {
    for lends in [true, false] {
        let mut tree = UiTree::new(row(lends));
        let start = Instant::now();
        let rest = tree.layout_at(SIZE, start);
        assert_eq!(inks(&rest), [INK, INK, OTHER]);
        tree.handle_event_at(UiEvent::PointerMove(INSIDE), start);
        // At once, as a button's label: no fade.
        let hovered = tree.layout_at(SIZE, start);
        tree.handle_event_at(UiEvent::PointerDown(INSIDE), start);
        let pressed = tree.layout_at(SIZE, start);
        tree.handle_event_at(UiEvent::PointerUp(INSIDE), start);
        tree.handle_event_at(UiEvent::PointerLeave, start);
        let left = tree.layout_at(SIZE, start);
        if lends {
            assert_eq!(inks(&hovered), [HOVER, HOVER, OTHER]);
            assert_eq!(inks(&pressed), [PRESSED, PRESSED, OTHER]);
        } else {
            assert_eq!(inks(&hovered), [INK, INK, OTHER]);
            assert_eq!(inks(&pressed), [INK, INK, OTHER]);
        }
        assert_eq!(inks(&left), [INK, INK, OTHER]);
        // Only the ink moves: what is painted where, and the targets, stay.
        assert_eq!(hovered.regions, rest.regions);
        assert_eq!(hovered.commands.len(), rest.commands.len());
    }
}

#[test]
fn what_a_clickable_holds_fades_to_its_ink_when_it_animates_its_foreground() {
    let clickable = Element::clickable("open", Element::text("Open").foreground(INK).animate_foreground("open:ink"))
        .foreground(INK)
        .hover_foreground(HOVER)
        .content_takes_ink()
        .size(Some(160.0), Some(30.0));
    let mut tree = UiTree::new(Element::column(vec![clickable, Element::spacer().flex(1.0)]));
    let start = Instant::now();
    tree.layout_at(SIZE, start);
    tree.handle_event_at(UiEvent::PointerMove(INSIDE), start);
    let at = |ms: u64| start + Duration::from_millis(ms);
    assert_eq!(inks(&tree.layout_at(SIZE, at(0))), [INK], "it sets off from its ink");
    let [between] = inks(&tree.layout_at(SIZE, at(u64::from(crate::tokens().motion.move_ms) / 2)))[..] else {
        panic!("one text");
    };
    assert!(between != INK && between != HOVER, "part-way: {between:?}");
    assert_eq!(inks(&tree.layout_at(SIZE, at(5_000))), [HOVER]);
}

const CARD: Color = Color::rgb(60, 60, 60);

/// Where the ring is among the commands, and where the content's fill is.
fn ring_and_fill(under: bool) -> (usize, usize) {
    let clickable = Element::clickable("card", Element::panel(Element::text("Card")).background(CARD))
        .size(Some(120.0), Some(40.0))
        .radius(6.0);
    let clickable = if under { clickable.focus_ring_under() } else { clickable };
    let mut tree = UiTree::new(Element::column(vec![clickable, Element::spacer().flex(1.0)]));
    tree.layout(SIZE);
    tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
    let frame = tree.layout(SIZE);
    let at = |found: fn(&DrawCommand) -> bool| frame.commands.iter().position(found).expect("painted");
    (
        at(|command| matches!(command, DrawCommand::StrokeRect { .. } | DrawCommand::StrokeCornerRect { .. })),
        at(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == CARD)),
    )
}

#[test]
fn a_clickable_draws_its_ring_under_what_it_holds_only_when_it_says_so() {
    let (ring, fill) = ring_and_fill(false);
    assert!(ring > fill, "over the content: ring {ring}, fill {fill}");
    let (ring, fill) = ring_and_fill(true);
    assert!(ring < fill, "under the content: ring {ring}, fill {fill}");
}
