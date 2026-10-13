//! One retained caret clock, shared by text fields, text areas and editors.
//! Only a caret actually observed inside the visible clip schedules a wakeup.

use web_time::{Duration, Instant};

const HALF_PERIOD: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CaretLocation {
    pub document: u64,
    pub revision: u64,
    pub offset: usize,
}

struct ActiveCaret {
    id: String,
    location: CaretLocation,
    epoch: Instant,
    next_deadline: Option<Instant>,
}

pub(crate) struct CaretBlink {
    active: Option<ActiveCaret>,
    window_focused: bool,
    allowed: bool,
    observed: bool,
    reset_pending: bool,
}

impl Default for CaretBlink {
    fn default() -> Self {
        Self {
            active: None,
            window_focused: true,
            allowed: true,
            observed: false,
            reset_pending: false,
        }
    }
}

impl CaretBlink {
    /// Record whether the window has focus. On a change the active caret is dropped and the
    /// blink restarts when a caret is next observed. Returns whether the value changed.
    pub fn set_window_focused(&mut self, focused: bool) -> bool {
        if self.window_focused == focused {
            return false;
        }
        self.window_focused = focused;
        self.active = None;
        self.reset_pending = true;
        true
    }

    /// Restart the blink at the next `observe`, which then begins a new period with the
    /// caret shown.
    pub fn reset(&mut self) {
        self.reset_pending = true;
    }

    /// Start a frame with no caret observed yet. With `allowed` false no caret is shown
    /// this frame and the active one is dropped.
    pub fn begin_frame(&mut self, allowed: bool) {
        self.observed = false;
        self.allowed = allowed;
        if !allowed {
            self.active = None;
        }
    }

    /// Called only for the focused, unselected caret intersecting the clip.
    pub fn observe(
        &mut self,
        id: &str,
        location: CaretLocation,
        now: Instant,
        reduced_motion: bool,
    ) -> bool {
        if !self.window_focused || !self.allowed {
            return false;
        }
        self.observed = true;
        if self.reset_pending
            || self
                .active
                .as_ref()
                .is_none_or(|active| active.id != id || active.location != location)
        {
            self.active = Some(ActiveCaret {
                id: id.into(),
                location,
                epoch: now,
                next_deadline: None,
            });
            self.reset_pending = false;
        }
        let active = self.active.as_mut().expect("observed caret");
        if reduced_motion {
            active.next_deadline = None;
            return true;
        }
        let elapsed = now.saturating_duration_since(active.epoch).as_nanos();
        let half = HALF_PERIOD.as_nanos();
        let wait = Duration::from_nanos((half - elapsed % half) as u64);
        active.next_deadline = Some(now + wait);
        (elapsed / half).is_multiple_of(2)
    }

    /// End a frame: forget the active caret if none was observed, or if it does not belong
    /// to the `focused` control.
    pub fn finish_frame(&mut self, focused: Option<&str>) {
        if !self.observed
            || self
                .active
                .as_ref()
                .is_some_and(|active| Some(active.id.as_str()) != focused)
        {
            self.active = None;
        }
    }

    /// How long until the caret next changes, for scheduling a repaint: zero while a
    /// restart is pending, `None` with no active caret or under reduced motion.
    pub fn repaint_after(&self, now: Instant) -> Option<Duration> {
        let active = self.active.as_ref()?;
        if self.reset_pending {
            return Some(Duration::ZERO);
        }
        active
            .next_deadline
            .map(|deadline| deadline.saturating_duration_since(now))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CodeEditor, Component, DrawCommand, EditorController, Element, Frame, Key, Size, UiEvent,
        UiTree,
    };

    fn caret(frame: &Frame) -> bool {
        frame.commands.iter().any(|command| {
            matches!(command,
                DrawCommand::FillRect { rect, .. } if rect.width == 1.5 && rect.height >= 16.0
            )
        })
    }

    fn key(tree: &mut UiTree, key: Key, ctrl: bool, now: Instant) {
        tree.handle_event_at(
            UiEvent::Key {
                key,
                ctrl,
                shift: false,
            },
            now,
        );
    }

    #[test]
    fn idle_caret_schedules_only_the_next_edge_and_a_late_frame_catches_up() {
        let now = Instant::now();
        let mut blink = CaretBlink::default();
        let location = CaretLocation {
            document: 0,
            revision: 3,
            offset: 3,
        };
        blink.begin_frame(true);
        assert!(blink.observe("field", location, now, false));
        blink.finish_frame(Some("field"));
        assert_eq!(blink.repaint_after(now), Some(Duration::from_millis(500)));
        assert_eq!(
            blink.repaint_after(now + Duration::from_millis(250)),
            Some(Duration::from_millis(250))
        );
        assert_eq!(
            blink.repaint_after(now + Duration::from_millis(600)),
            Some(Duration::ZERO)
        );
        blink.begin_frame(true);
        assert!(!blink.observe("field", location, now + Duration::from_millis(600), false));
        blink.finish_frame(Some("field"));
        assert_eq!(
            blink.repaint_after(now + Duration::from_millis(600)),
            Some(Duration::from_millis(400))
        );
        blink.reset();
        assert_eq!(
            blink.repaint_after(now + Duration::from_millis(610)),
            Some(Duration::ZERO)
        );
        blink.begin_frame(true);
        assert!(blink.observe("field", location, now + Duration::from_millis(610), false));
        blink.finish_frame(Some("field"));
        assert_eq!(
            blink.repaint_after(now + Duration::from_millis(610)),
            Some(Duration::from_millis(500))
        );
        blink.begin_frame(false);
        assert!(!blink.observe("field", location, now + Duration::from_millis(700), false));
        blink.finish_frame(Some("field"));
        assert_eq!(blink.repaint_after(now + Duration::from_millis(700)), None);
    }

    #[test]
    fn fields_textareas_and_editors_share_blink_reset_selection_and_window_focus() {
        let size = Size::new(240.0, 100.0);
        for kind in 0..3 {
            let controller =
                EditorController::new(limn_editor::TextBuffer::new("hello world").unwrap());
            let make = || match kind {
                0 => Element::text_input("field", "hello world", "")
                    .padding(6.0)
                    .size(Some(240.0), Some(36.0)),
                1 => Element::text_area("field", "hello world", "")
                    .padding(9.0)
                    .size(Some(240.0), Some(100.0)),
                _ => CodeEditor::new("field", controller.clone())
                    .height(100.0)
                    .render(),
            };
            let start = Instant::now();
            let mut tree = UiTree::new(make());
            tree.layout_at(size, start);
            assert!(tree.focus("field"));
            assert!(caret(&tree.layout_at(size, start)), "kind {kind}");
            tree.set_root(make());
            assert!(
                !caret(&tree.layout_at(size, start + Duration::from_millis(500))),
                "kind {kind}"
            );
            assert!(
                caret(&tree.layout_at(size, start + Duration::from_millis(1000))),
                "kind {kind}"
            );
            let moved = start + Duration::from_millis(1600);
            assert!(!caret(&tree.layout_at(size, moved)), "kind {kind}");
            key(&mut tree, Key::ArrowRight, false, moved);
            assert!(
                caret(&tree.layout_at(size, moved)),
                "navigation resets kind {kind}"
            );
            assert!(!caret(
                &tree.layout_at(size, moved + Duration::from_millis(500))
            ));
            let typed = moved + Duration::from_millis(550);
            tree.handle_event_at(UiEvent::Text("ñ".into()), typed);
            assert!(
                caret(&tree.layout_at(size, typed)),
                "typing resets kind {kind}"
            );
            key(&mut tree, Key::A, true, typed);
            assert!(
                !caret(&tree.layout_at(size, typed)),
                "selection hides kind {kind}"
            );
            assert_eq!(tree.caret_repaint_after(), None);
            key(&mut tree, Key::ArrowRight, false, typed);
            assert!(caret(&tree.layout_at(size, typed)));
            assert!(tree.set_window_focused(false));
            assert!(!caret(&tree.layout_at(size, typed)));
            assert_eq!(tree.caret_repaint_after(), None);
            assert!(!tree.set_window_focused(false));
            assert!(tree.set_window_focused(true));
            assert!(caret(&tree.layout_at(size, typed)));
            tree.set_reduced_motion(true);
            assert!(caret(
                &tree.layout_at(size, typed + Duration::from_secs(10))
            ));
            assert_eq!(tree.caret_repaint_after(), None);
        }
    }

    #[test]
    fn streaming_tree_rebuilds_and_unchanged_input_sync_do_not_restart_blink() {
        let size = Size::new(240.0, 100.0);
        let make = |chunk: usize| {
            Element::column(vec![
                Element::text(format!("Respuesta {chunk}")).size(None, Some(20.0)),
                Element::text_area("prompt", "draft", "")
                    .padding(9.0)
                    .size(None, Some(36.0)),
            ])
        };
        let start = Instant::now();
        let mut tree = UiTree::new(make(0));
        tree.layout_at(size, start);
        assert!(tree.focus("prompt"));
        assert!(caret(&tree.layout_at(size, start)));
        for chunk in 1..=50 {
            assert!(!tree.set_window_focused(true));
            tree.set_input_value("prompt", "draft");
            tree.set_input_cursor("prompt", 5);
            tree.set_root(make(chunk));
            let frame = tree.layout_at(size, start + Duration::from_millis(chunk as u64 * 20));
            assert_eq!(caret(&frame), chunk < 25 || chunk == 50, "chunk {chunk}");
        }
    }

    #[test]
    fn an_offscreen_focused_input_has_no_caret_or_wakeup() {
        let size = Size::new(200.0, 80.0);
        let make = || {
            Element::text_input("field", "hello", "")
                .padding(6.0)
                .size(Some(200.0), Some(36.0))
        };
        let now = Instant::now();
        let mut tree = UiTree::new(make());
        tree.layout_at(size, now);
        assert!(tree.focus("field"));
        assert!(caret(&tree.layout_at(size, now)));
        tree.set_root(
            Element::column(vec![Element::spacer().size(None, Some(200.0)), make()])
                .clip_children(),
        );
        assert!(!caret(&tree.layout_at(size, now)));
        assert_eq!(tree.focused_id(), Some("field"));
        assert_eq!(tree.caret_repaint_after(), None);
        tree.set_root(make());
        assert!(caret(&tree.layout_at(size, now + Duration::from_secs(10))));
    }
}
