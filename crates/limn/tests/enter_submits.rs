//! A text area that sends on Enter, as a chat composer does: Enter submits,
//! Shift+Enter breaks the line, Ctrl+Enter submits either way. Without the
//! option Enter breaks the line, as it always has.

use limn::{
    Component, ComponentPalette, Composer, Element, Key, Point, Size, TextArea, UiAction, UiEvent, UiTree,
};

const VIEW: Size = Size::new(480.0, 240.0);

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

/// A tree holding `field`, laid out, with the field focused and "hi" typed.
fn typed(field: Element, id: &str) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![field, Element::spacer().flex(1.0)]));
    tree.set_reduced_motion(true);
    let frame = tree.layout(VIEW);
    let rect = frame.regions.iter().find(|region| region.id == id).expect("the field is laid out").rect;
    let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    tree.handle_event(UiEvent::PointerDown(center));
    tree.handle_event(UiEvent::PointerUp(center));
    tree.layout(VIEW);
    tree.handle_event(UiEvent::Text("hi".into()));
    tree.layout(VIEW);
    tree
}

fn key(tree: &mut UiTree, shift: bool, ctrl: bool) -> Vec<UiAction> {
    let actions = tree.handle_event(UiEvent::Key { key: Key::Enter, shift, ctrl, alt: false });
    tree.layout(VIEW);
    actions
}

fn submitted(id: &str, value: &str) -> UiAction {
    UiAction::Submitted { id: id.into(), value: value.into() }
}

fn changed(id: &str, value: &str) -> UiAction {
    UiAction::InputChanged { id: id.into(), value: value.into() }
}

#[test]
fn a_text_area_that_submits_on_enter_breaks_the_line_on_shift_enter() {
    let field = TextArea::new("message", "", "Message", palette()).enter_submits(true).render();
    let mut tree = typed(field, "message");
    assert_eq!(key(&mut tree, false, false), vec![submitted("message", "hi")]);
    assert_eq!(key(&mut tree, true, false), vec![changed("message", "hi\n")]);
    assert_eq!(key(&mut tree, false, true), vec![submitted("message", "hi\n")]);
}

#[test]
fn a_text_area_breaks_the_line_on_enter_by_default() {
    let field = TextArea::new("notes", "", "Notes", palette()).render();
    let mut tree = typed(field, "notes");
    assert_eq!(key(&mut tree, false, false), vec![changed("notes", "hi\n")]);
    assert_eq!(key(&mut tree, true, false), vec![changed("notes", "hi\n\n")]);
    assert_eq!(key(&mut tree, false, true), vec![submitted("notes", "hi\n\n")]);
}

#[test]
fn a_composer_that_submits_on_enter_hands_it_to_its_editor() {
    let composer = Composer::new("composer", "", "Message #general", 400.0, palette())
        .enter_submits(true)
        .render();
    let mut tree = typed(composer, "composer");
    assert_eq!(key(&mut tree, false, false), vec![submitted("composer", "hi")]);
    assert_eq!(key(&mut tree, true, false), vec![changed("composer", "hi\n")]);
}

#[test]
fn an_element_text_area_can_submit_on_enter() {
    let field = Element::text_area("raw", "", "Raw").text_area_enter_submits(true).size(None, Some(80.0));
    let mut tree = typed(field, "raw");
    assert_eq!(key(&mut tree, false, false), vec![submitted("raw", "hi")]);
    // On anything but a text area the option does nothing.
    let text = Element::text("not a field").text_area_enter_submits(true);
    assert!(matches!(text.kind, limn::ElementKind::Text(_)));
}
