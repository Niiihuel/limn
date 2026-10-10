//! Keyboard focus for menus expanded in normal layout. These scopes never
//! capture Tab or Escape and never change focus when merely appearing.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InlineMenuFocusScope {
    pub(super) focusables: Vec<(String, HitKind)>,
}

impl UiTree {
    pub(super) fn step_inline_menu_option(&mut self, key: Key) -> bool {
        // Floating menus retain priority, including submenus opened from one
        // of these inline options. Do not route through a modal to the card.
        if !self.frame.popover_focus.is_empty() {
            return false;
        }
        let Some(focused) = self.focused.as_deref() else {
            return false;
        };
        let Some(scope) = self
            .frame
            .inline_menu_focus
            .iter()
            .rev()
            .find(|scope| scope.focusables.iter().any(|(id, _)| id == focused))
        else {
            return false;
        };
        let kind = scope
            .focusables
            .iter()
            .find(|(id, _)| id == focused)
            .unwrap()
            .1;
        if !matches!(kind, HitKind::Button | HitKind::TextInput)
            || kind == HitKind::TextInput && matches!(key, Key::Home | Key::End)
        {
            return false;
        }
        let buttons: Vec<_> = scope
            .focusables
            .iter()
            .skip(1)
            .filter(|(_, kind)| *kind == HitKind::Button)
            .map(|(id, _)| id.as_str())
            .collect();
        if buttons.is_empty() {
            return false;
        }
        let current = buttons.iter().position(|id| *id == focused);
        let index = match key {
            Key::ArrowDown => current.map_or(0, |index| (index + 1) % buttons.len()),
            Key::ArrowUp => current.map_or(buttons.len() - 1, |index| {
                (index + buttons.len() - 1) % buttons.len()
            }),
            Key::Home => 0,
            Key::End => buttons.len() - 1,
            _ => return false,
        };
        let target = buttons[index].to_owned();
        self.request_scroll_focus(&target);
        self.focused = Some(target);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
