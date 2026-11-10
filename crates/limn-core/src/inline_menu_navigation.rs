//! Keyboard focus for menus expanded in normal layout. These scopes never
//! capture Tab or Escape and never change focus when merely appearing.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct InlineMenuFocusScope {
    pub(super) focusables: Vec<(String, HitKind)>,
}

impl UiTree {
    /// Move the focus among the options of the inline menu that holds it,
    /// for `key`; says whether it moved.
    #[doc(hidden)]
    pub fn step_inline_menu_option(&mut self, key: Key) -> bool {
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
