//! What a frame tells the platform: every control is there, with a role, a
//! name and its state.

use limn::{
    Access, AccessNode, Button, Component, ComponentPalette, Element, Icon, IconButton, Role, Size, TextField,
    UiEvent, UiTree,
};

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

fn nodes(element: Element) -> Vec<AccessNode> {
    UiTree::new(element).layout(Size::new(480.0, 320.0)).access
}

fn node(element: Element, id: &str) -> AccessNode {
    let nodes = nodes(element);
    nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
}

#[test]
fn a_button_is_a_button_named_by_its_label() {
    let save = node(Button::new("save", "Save changes", palette()).render(), "save");
    assert_eq!((save.role, save.name.as_str()), (Role::Button, "Save changes"));
    assert!(save.actionable && !save.disabled);
    assert!(save.rect.width > 0.0 && save.rect.height > 0.0);
}

#[test]
fn an_icon_alone_has_no_name_until_it_is_given_one() {
    let bare = node(IconButton::new("close", Icon::Close, palette()).render(), "close");
    assert_eq!((bare.role, bare.name.as_str()), (Role::Button, ""));
    let named = node(IconButton::new("close", Icon::Close, palette()).render().accessibility_label("Close"), "close");
    assert_eq!(named.name, "Close");
}

#[test]
fn a_row_that_can_be_clicked_is_named_by_the_text_in_it() {
    let row = Element::clickable(
        "open",
        Element::row(vec![Element::icon(Icon::Folder), Element::text("Documents"), Element::text("12 items")]),
    );
    assert_eq!(node(row, "open").name, "Documents 12 items");
}

#[test]
fn a_field_is_named_by_its_placeholder_and_a_password_keeps_its_value() {
    let name = node(TextField::new("name", "limn", "Project name", palette()).render(), "name");
    assert_eq!((name.role, name.name.as_str()), (Role::TextInput, "Project name"));
    assert_eq!(name.value_text.as_deref(), Some("limn"));
    let secret = node(TextField::new("key", "hunter2", "Password", palette()).password(true).render(), "key");
    assert_eq!(secret.role, Role::PasswordInput);
    assert_eq!(secret.value_text, None);
}

#[test]
fn what_a_component_says_goes_over_what_the_tree_works_out() {
    let switch = Element::clickable("wifi", Element::text("on"))
        .accessible(Access::new(Role::Switch).name("Wi-Fi").toggled(true));
    let said = node(switch, "wifi");
    assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::Switch, "Wi-Fi", Some(true)));
    // Said twice, the later adds to the earlier.
    let tab = Element::clickable("tab", Element::text("Files"))
        .accessible(Access::new(Role::Tab))
        .accessible(Access::default().selected(true));
    let said = node(tab, "tab");
    assert_eq!((said.role, said.name.as_str(), said.selected), (Role::Tab, "Files", Some(true)));
}

#[test]
fn a_passive_thing_is_there_when_it_has_a_name_to_be_known_by() {
    let bar = Element::spacer()
        .size(Some(120.0), Some(4.0))
        .accessible(Access::new(Role::ProgressIndicator).id("upload").name("Uploading").value(0.4));
    let said = node(bar, "upload");
    assert_eq!((said.role, said.value), (Role::ProgressIndicator, Some(0.4)));
    assert!(!said.actionable);
    // Without an ID there is nothing to call it.
    assert!(nodes(Element::text("plain").accessible(Access::new(Role::Label))).is_empty());
}

#[test]
fn a_control_drawn_disabled_is_there_and_says_so() {
    let disabled = Element::text("Save")
        .accessible(Access::new(Role::Button).id("save").name("Save").disabled(true));
    let said = node(disabled, "save");
    assert!(said.disabled && !said.actionable);
}

#[test]
fn a_value_that_is_not_a_number_is_none() {
    let bar = Element::spacer().accessible(Access::new(Role::Slider).id("s").value(f32::NAN));
    assert_eq!(node(bar, "s").value, None);
    let over = Element::spacer().accessible(Access::new(Role::Slider).id("s").value(7.0));
    assert_eq!(node(over, "s").value, Some(1.0));
}

#[test]
fn the_focused_control_says_it_has_the_focus() {
    let mut tree = UiTree::new(Element::column(vec![
        Button::new("one", "One", palette()).render(),
        Button::new("two", "Two", palette()).render(),
    ]));
    tree.layout(Size::new(320.0, 200.0));
    tree.handle_event(UiEvent::Key { key: limn::Key::Tab, shift: false, ctrl: false });
    let frame = tree.layout(Size::new(320.0, 200.0));
    let focused: Vec<_> = frame.access.iter().filter(|node| node.focused).map(|node| node.id.as_str()).collect();
    assert_eq!(focused, ["one"]);
}
