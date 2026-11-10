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

#[test]
fn a_soft_keyboards_return_alone_submits_a_line_and_breaks_a_text_area() {
    let page = Element::column(vec![
        Element::text_input("query", "consulta".to_owned(), "Search").size(None, Some(32.0)),
        Element::text_area("notes", "uno".to_owned(), "Notes").size(None, Some(64.0)),
    ]);
    let mut tree = UiTree::new(page);
    tree.layout(Size::new(240.0, 120.0));
    assert!(tree.focus("query"));
    assert_eq!(
        tree.handle_event(UiEvent::Text("\n".into())),
        vec![UiAction::Submitted { id: "query".into(), value: "consulta".into() }],
        "iOS's Return comes as a line break with no Enter key",
    );
    assert_eq!(tree.input_value("query"), Some("consulta"), "and leaves the line as it was");
    assert!(tree.focus("notes"));
    tree.handle_event(UiEvent::Text("\n".into()));
    assert_eq!(tree.input_value("notes").map(|value| value.contains('\n')), Some(true), "a text area takes the break");
}
