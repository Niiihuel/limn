//! A paste reply names one observed editing state, never the current focus alone.
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
enum EditingState {
    Field {
        epoch: Arc<()>,
        kind: HitKind,
    },
    Editor {
        epoch: Arc<()>,
        state: limn_editor::EditStateToken,
    },
}

/// An opaque, single-use destination captured from one tree's focused editor.
///
/// Clones share a delivery ticket. Accepted replies consume it, including empty
/// text and failed edits. A busy buffer, pending layout or foreign tree does not
/// consume it. The target holds no document text, buffer or controller.
///
/// The host must separately correlate its clipboard request and application
/// context, and invalidate targets when it rebinds a window or logical editor.
#[derive(Clone)]
pub struct PasteTarget {
    tree: Arc<()>,
    id: String,
    editing: EditingState,
    ticket: Arc<AtomicBool>,
}

impl std::fmt::Debug for PasteTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasteTarget").finish_non_exhaustive()
    }
}

impl PartialEq for PasteTarget {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.ticket, &other.ticket)
    }
}

impl Eq for PasteTarget {}

impl UiTree {
    /// Capture the currently writable, focused control after layout.
    ///
    /// Returns `None` during composition, while another keyboard scope owns
    /// input, or when a buffer is busy. Focus/selection changes and subsequent
    /// restoration do not restore old targets. Ordinary equivalent relayouts
    /// preserve them. No clipboard read or request is performed here.
    pub fn capture_paste_target(&mut self) -> Option<PasteTarget> {
        let (id, kind) = self.paste_control()?;
        let editing = match kind {
            HitKind::TextInput | HitKind::TextArea => EditingState::Field {
                epoch: self.inputs.get(&id)?.paste_epoch.clone(),
                kind,
            },
            HitKind::CodeEditor => {
                let (epoch, state) = self.code_editors.get(&id)?.capture_paste_state()?;
                EditingState::Editor { epoch, state }
            }
            _ => return None,
        };
        Some(PasteTarget {
            tree: self.paste_epoch.clone(),
            id,
            editing,
            ticket: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Invalidate replies when the host changes an editor association or the
    /// window/application context without an observable tree transition.
    ///
    /// This includes moving a buffer out and restoring that same instance
    /// through `shared_buffer` between tree observations. Native clipboard
    /// endpoint lifetime and account/channel generations remain the host's job.
    pub fn invalidate_paste_targets(&mut self) {
        self.paste_epoch = Arc::new(());
    }

    fn paste_control(&self) -> Option<(String, HitKind)> {
        if self.paste_layout_pending
            || !self.ime_window_focused
            || self.context_menu.is_some()
            || self.ime_composing()
        {
            return None;
        }
        let id = self.focused.as_ref()?;
        if self.active_popover_scope().is_some_and(|scope| {
            !scope
                .focusables
                .iter()
                .any(|(candidate, _)| candidate == id)
        }) {
            return None;
        }
        let region = self
            .frame
            .regions
            .iter()
            .rev()
            .find(|region| region.id == *id)?;
        matches!(
            region.kind,
            HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor
        )
        .then(|| (id.clone(), region.kind))
    }

    pub(super) fn paste_captured(
        &mut self,
        target: PasteTarget,
        text: String,
        now: Instant,
    ) -> Vec<UiAction> {
        if !Arc::ptr_eq(&self.paste_epoch, &target.tree) {
            return Vec::new();
        }
        let Some((id, kind)) = self.paste_control().filter(|(id, _)| *id == target.id) else {
            return Vec::new();
        };
        match target.editing {
            EditingState::Field {
                epoch,
                kind: expected,
            } if kind == expected => {
                let Some(input) = self.inputs.get_mut(&id) else {
                    return Vec::new();
                };
                if !Arc::ptr_eq(&epoch, &input.paste_epoch)
                    || target
                        .ticket
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_err()
                {
                    return Vec::new();
                }
                self.caret_blink.reset();
                if input.insert(&text, kind == HitKind::TextArea) {
                    vec![UiAction::InputChanged {
                        id,
                        value: input.value.clone(),
                    }]
                } else {
                    Vec::new()
                }
            }
            EditingState::Editor { epoch, state } if kind == HitKind::CodeEditor => {
                let Some(view) = self.code_editors.get_mut(&id) else {
                    return Vec::new();
                };
                if let Some(actions) =
                    view.paste_captured(&epoch, &state, &target.ticket, &text, now)
                {
                    self.caret_blink.reset();
                    actions
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        }
    }

    // Borrow only these fields so layout can retain references to its current frame.
    pub(super) fn replace_focused(
        focused: &mut Option<String>,
        epoch: &mut Arc<()>,
        next: Option<String>,
    ) {
        if *focused != next {
            *focused = next;
            *epoch = Arc::new(());
        }
    }

    pub(super) fn begin_paste_layout(&mut self) {
        self.paste_layout_pending = true;
        for view in self.code_editors.values_mut() {
            view.begin_paste_layout();
        }
    }

    pub(super) fn finish_paste_layout(&mut self) {
        let controls: HashMap<_, _> = self
            .frame
            .regions
            .iter()
            .filter(|region| {
                matches!(
                    region.kind,
                    HitKind::TextInput | HitKind::TextArea | HitKind::CodeEditor
                )
            })
            .map(|region| (region.id.as_str(), region.kind))
            .collect();
        for (id, input) in &mut self.inputs {
            let kind = controls
                .get(id.as_str())
                .copied()
                .filter(|kind| matches!(kind, HitKind::TextInput | HitKind::TextArea));
            if input.paste_kind != kind {
                input.paste_epoch = Arc::new(());
                input.paste_kind = kind;
            }
        }
        for (id, view) in &mut self.code_editors {
            view.finish_paste_layout(controls.get(id.as_str()) == Some(&HitKind::CodeEditor));
        }
    }
}
