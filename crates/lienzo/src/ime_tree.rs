//! Native compositions keep the document/input that received their preedit.
//! A late Commit must not become ordinary typing in whichever field gained focus.
use super::*;

#[derive(Clone, PartialEq, Eq)]
enum Target {
    Editor { id: String, document: u64 },
    Input { id: String },
}

pub(super) struct ImeOwner {
    target: Target,
    cancelled: bool,
}

impl UiTree {
    fn current_ime_target(&self) -> Option<Target> {
        if !self.ime_window_focused || self.context_menu.is_some() {
            return None;
        }
        let id = self.focused.as_ref()?;
        let region = self
            .frame
            .regions
            .iter()
            .rev()
            .find(|region| region.id == *id)?;
        match region.kind {
            HitKind::CodeEditor => {
                let editor = self.code_editors.get(id)?;
                if editor.editor.read_only {
                    return None;
                }
                Some(Target::Editor {
                    id: id.clone(),
                    document: editor.editor.controller.snapshot().ok()?.document_id(),
                })
            }
            HitKind::TextInput | HitKind::TextArea => Some(Target::Input { id: id.clone() }),
            _ => None,
        }
    }

    fn cancel_ime_owner(&mut self, now: Instant) {
        if let Some(owner) = &mut self.ime_owner {
            if !owner.cancelled {
                owner.cancelled = true;
                if let Target::Editor { id, .. } = &owner.target {
                    if let Some(editor) = self.code_editors.get_mut(id) {
                        editor.composition(None, None, now);
                    }
                }
            }
        }
    }

    pub(super) fn validate_ime_owner(&mut self, now: Instant) {
        if self.ime_owner.as_ref().is_some_and(|owner| {
            !owner.cancelled && self.current_ime_target().as_ref() != Some(&owner.target)
        }) {
            self.cancel_ime_owner(now);
        }
    }

    /// True means this native event belongs to an invalidated composition and
    /// must be swallowed before generic TextInput fallback can accept it.
    pub(super) fn reject_ime_event(&mut self, event: &UiEvent, now: Instant) -> bool {
        match event {
            UiEvent::ImePreedit(text) => {
                // Empty preedit is dismissal, often immediately before Commit.
                // Preserve the old owner/tombstone rather than retargeting it.
                if text.is_empty() && self.ime_owner.is_some() {
                    return self.ime_owner.as_ref().is_some_and(|owner| owner.cancelled);
                }
                let Some(target) = self.current_ime_target() else {
                    return true;
                };
                self.ime_owner = Some(ImeOwner {
                    target,
                    cancelled: false,
                });
                false
            }
            UiEvent::ImeCommit(_) => {
                let current = self.current_ime_target();
                self.ime_owner.take().map_or(current.is_none(), |owner| {
                    owner.cancelled || current.as_ref() != Some(&owner.target)
                })
            }
            UiEvent::ImeDeleteSurrounding { .. } => {
                self.current_ime_target().is_none()
                    || self.ime_owner.as_ref().is_some_and(|owner| owner.cancelled)
            }
            UiEvent::ImeCancelled => {
                self.cancel_ime_owner(now);
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "ime_tree_tests.rs"]
mod tests;
