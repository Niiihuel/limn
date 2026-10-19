//! Native compositions keep the document/input that received their preedit.
//! A late Commit must not become ordinary typing in whichever field gained focus.
use super::*;

#[derive(Clone, PartialEq, Eq)]
enum Target {
    Editor { id: String, document: u64 },
    Input { id: String },
}

/// Where an input method's window belongs, from [`UiTree::ime_area`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImeArea {
    /// The visible part of the focused text.
    pub area: Rect,
    /// Its caret: where what is being composed is shown.
    pub caret: Rect,
    /// A password: the platform should compose and predict nothing in it.
    pub secret: bool,
}

pub(super) struct ImeOwner {
    target: Target,
    cancelled: bool,
}

impl UiTree {
    /// Where an input method belongs for the text that has the focus: a
    /// code editor's, or a text field's or text area's in a tree that lays
    /// its text out (`UiTree::set_text_layout`). A host tells the platform
    /// once a frame, after the layout, as `limn_egui::report_ime` does.
    /// `None` when the window is not focused, a context menu is open, or
    /// nothing that takes text has the focus; for a code editor, also while
    /// a popover is open over it.
    pub fn ime_area(&self) -> Option<ImeArea> {
        match self.current_ime_target()? {
            Target::Editor { .. } => {
                let (area, caret) = self.editor_ime_rects()?;
                Some(ImeArea { area, caret, secret: false })
            }
            Target::Input { id } => {
                let input = self.inputs.get(&id)?;
                let (area, caret) = input.ime?;
                Some(ImeArea { area, caret, secret: input.password })
            }
        }
    }

    pub(super) fn ime_composing(&self) -> bool {
        self.ime_owner
            .as_ref()
            .is_some_and(|owner| !owner.cancelled)
    }

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
        let mut changed = false;
        if let Some(owner) = &mut self.ime_owner {
            if !owner.cancelled {
                changed = true;
                owner.cancelled = true;
                if let Target::Editor { id, .. } = &owner.target {
                    if let Some(editor) = self.code_editors.get_mut(id) {
                        editor.composition(None, None, now);
                    }
                }
            }
        }
        if changed {
            self.invalidate_paste_targets();
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
                if !self.ime_composing() {
                    self.invalidate_paste_targets();
                }
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
