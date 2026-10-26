use super::*;

#[test]
fn other_buttons_and_pasted_pictures_reach_the_host_as_actions() {
    let mut tree = UiTree::new(Element::text_input("note", String::new(), "Note").size(None, Some(32.0)));
    tree.layout(Size::new(200.0, 80.0));
    let position = Point::new(12.0, 14.0);
    for (button, pressed) in [(OtherButton::Middle, true), (OtherButton::Middle, false), (OtherButton::Back, false)] {
        assert_eq!(
            tree.handle_event(UiEvent::OtherButton { button, pressed, position }),
            vec![UiAction::OtherButton { button, pressed, position }],
        );
    }
    assert_eq!(
        tree.handle_event(UiEvent::PasteImage { width: 1, height: 1, rgba: Arc::from([1, 2, 3, 4]) }),
        vec![UiAction::PasteImage { target: None, width: 1, height: 1, rgba: Arc::from([1, 2, 3, 4]) }],
    );
}
