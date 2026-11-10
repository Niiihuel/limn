use crate::{Cursor, Element, Point, Size, UiEvent, UiTree};

fn video(hidden: bool) -> Element {
    let picture = Element::clickable("video", Element::spacer().size(Some(160.0), Some(90.0)));
    let picture = if hidden { picture.cursor(Cursor::Hidden) } else { picture };
    Element::column(vec![picture, Element::spacer().size(Some(160.0), Some(40.0))])
}

#[test]
fn a_hidden_pointer_is_reported_only_over_its_element_and_only_while_asked_for() {
    let mut tree = UiTree::new(video(true));
    tree.layout(Size::new(160.0, 130.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(80.0, 45.0)));
    assert_eq!(tree.cursor(), Some(Cursor::Hidden));
    tree.handle_event(UiEvent::PointerMove(Point::new(80.0, 110.0)));
    assert_eq!(tree.cursor(), None, "off the video the platform's pointer is back");

    tree.handle_event(UiEvent::PointerMove(Point::new(80.0, 45.0)));
    tree.set_root(video(false));
    tree.layout(Size::new(160.0, 130.0));
    assert_eq!(tree.cursor(), None, "the host shows it again by no longer asking");
}
