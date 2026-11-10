//! Keyboard focus in menus expanded in normal layout. These tests exercise
//! the engine's inline menu navigation through `DropdownMenu`.
use crate::*;
use crate::{Component, ComponentPalette, DropdownMenu, DropdownMenuItem, SearchField};

fn fixture() -> Element {
    let menu = DropdownMenu::new(
        Element::button("choose", "Elegir rama"),
        (0..14)
            .map(|index| {
                DropdownMenuItem::new(format!("choice:{index}"), format!("Rama {index}")).into()
            })
            .collect(),
        true,
        "dismiss",
        ComponentPalette::neutral_dark(),
    )
    .header(SearchField::new("search", "", "Buscar", ComponentPalette::neutral_dark()).render())
    .width(260.0)
    .max_visible_items(3)
    .inline(true)
    .render();
    Element::column(vec![menu, Element::button("outside", "Otro control")]).gap(10.0)
}

fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
    tree.handle_event(UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
        alt: false,
    })
}

#[test]
fn inline_arrows_enter_options_reveal_last_and_enter_activates_exactly_once() {
    let mut tree = UiTree::new(fixture());
    tree.set_reduced_motion(true);
    let viewport = Size::new(340.0, 500.0);
    tree.layout(viewport);
    assert_eq!(
        tree.focused_id(),
        None,
        "opening inline content must not steal focus"
    );
    assert!(tree.focus("choose"));
    assert!(key(&mut tree, Key::ArrowDown).is_empty());
    assert_eq!(tree.focused_id(), Some("choice:0"));
    tree.layout(viewport);
    assert!(key(&mut tree, Key::End).is_empty());
    let frame = tree.layout(viewport);
    assert_eq!(tree.focused_id(), Some("choice:13"));
    assert!(frame.regions.iter().any(|region| region.id == "choice:13"));
    assert_eq!(
        key(&mut tree, Key::Enter),
        [UiAction::Clicked {
            id: "choice:13".into()
        }]
    );
    assert!(key(&mut tree, Key::ArrowDown).is_empty());
    tree.layout(viewport);
    assert_eq!(tree.focused_id(), Some("choice:0"));
    assert!(key(&mut tree, Key::ArrowUp).is_empty());
    tree.layout(viewport);
    assert_eq!(tree.focused_id(), Some("choice:13"));
    key(&mut tree, Key::Tab);
    assert_eq!(
        tree.focused_id(),
        Some("outside"),
        "Tab must leave the menu"
    );
    assert!(key(&mut tree, Key::ArrowDown).is_empty());
    assert_eq!(
        tree.focused_id(),
        Some("outside"),
        "arrows outside must not enter the menu"
    );
}

#[test]
fn inline_search_keeps_caret_keys_and_routes_only_vertical_arrows() {
    let mut tree = UiTree::new(fixture());
    tree.set_reduced_motion(true);
    tree.layout(Size::new(340.0, 500.0));
    tree.set_input_value("search", "texto");
    assert!(tree.focus("search"));
    key(&mut tree, Key::Home);
    assert_eq!(tree.focused_id(), Some("search"));
    assert_eq!(tree.input_cursor("search"), Some(0));
    key(&mut tree, Key::End);
    assert_eq!(tree.input_cursor("search"), Some(5));
    key(&mut tree, Key::ArrowUp);
    tree.layout(Size::new(340.0, 500.0));
    assert_eq!(tree.focused_id(), Some("choice:13"));
}

#[test]
fn floating_menu_owns_navigation_while_inline_menu_stays_passive() {
    let popup = Element::popover(
        Element::button("floating_trigger", "Acciones"),
        Element::column(vec![
            Element::button("floating_first", "Primero"),
            Element::button("floating_last", "Último"),
        ]),
        "floating_dismiss",
        PopoverPlacement::BottomStart,
    );
    let mut tree = UiTree::new(Element::column(vec![fixture(), popup]));
    tree.set_reduced_motion(true);
    tree.layout(Size::new(420.0, 700.0));
    assert!(tree.focus("floating_first"));
    key(&mut tree, Key::ArrowDown);
    assert_eq!(tree.focused_id(), Some("floating_last"));
    assert!(!tree.step_inline_menu_option(Key::ArrowUp));
    assert_eq!(tree.focused_id(), Some("floating_last"));
}
