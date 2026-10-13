//! A fixed-height slot that shows the latest of a series of lines.

use crate::{Component, Element};

/// The host passes the current line and a key that changes when the line
/// does. The slot keeps its height, so content below it never moves; a new
/// line rises in from below while the one it replaces moves up and fades.
pub struct Ticker {
    id: String,
    key: String,
    line: Element,
    height: f32,
}

impl Ticker {
    /// `key` names the content: the line travels only when it changes.
    pub fn new(id: impl Into<String>, key: impl Into<String>, line: Element) -> Self {
        Self {
            id: id.into(),
            key: key.into(),
            line,
            height: 20.0,
        }
    }

    /// The slot's height. The line rests at its top; a slot taller than the
    /// line shows the next one arriving before it takes its place.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(0.0);
        }
        self
    }
}

impl Component for Ticker {
    fn render(self) -> Element {
        Element::column(vec![self.line.animate_replace(self.id, self.key)])
            .size(None, Some(self.height))
            .clip_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};
    use web_time::{Duration, Instant};

    const SIZE: Size = Size::new(300.0, 200.0);

    fn root(step: &str) -> Element {
        Element::column(vec![
            Ticker::new("ticker", step, Element::truncated_text(step).size(None, Some(20.0)))
                .height(40.0)
                .render(),
            Element::text("below"),
        ])
    }

    /// Where a text is painted and how opaque, if it is.
    fn painted(frame: &crate::Frame, wanted: &str) -> Option<(f32, u8)> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { position, text, color, .. } if text == wanted => {
                Some((position.y, color.a))
            }
            _ => None,
        })
    }

    #[test]
    fn the_slot_keeps_its_height_and_clips_its_line() {
        let mut tree = UiTree::new(root("one"));
        let frame = tree.layout(SIZE);
        // The first line is simply there.
        assert!(!tree.is_animating());
        assert!(frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::PushClip(rect) if rect.height == 40.0)));
        let below = painted(&frame, "below").unwrap().0;
        tree.set_root(root("two"));
        assert_eq!(painted(&tree.layout(SIZE), "below").unwrap().0, below);
    }

    #[test]
    fn a_new_line_rises_while_the_old_one_leaves_upwards() {
        // In the past, so the wall clock `is_animating` reads is after the end.
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(root("one"));
        let rest = painted(&tree.layout_at(SIZE, start), "one").unwrap();
        tree.set_root(root("two"));
        tree.layout_at(SIZE, start);
        let half = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) / 2);
        let frame = tree.layout_at(SIZE, start + half);
        let (old_y, old_alpha) = painted(&frame, "one").expect("the old line is still leaving");
        let (new_y, new_alpha) = painted(&frame, "two").unwrap();
        assert!(old_y < rest.0 && old_y > rest.0 - 20.0, "{old_y}");
        assert!(new_y > rest.0 && new_y < rest.0 + 20.0, "{new_y}");
        assert!(old_alpha < rest.1 && new_alpha < rest.1 && new_alpha > 0);

        let frame = tree.layout_at(SIZE, start + half * 4);
        assert_eq!(painted(&frame, "two"), Some(rest));
        assert_eq!(painted(&frame, "one"), None);
        assert!(!tree.is_animating());
        // The same key again does not travel.
        tree.set_root(root("two"));
        assert_eq!(painted(&tree.layout_at(SIZE, start + half * 5), "two"), Some(rest));
        assert!(!tree.is_animating());
        // A change that has only just begun keeps the window painting.
        tree.set_root(root("three"));
        tree.layout_at(SIZE, Instant::now() + Duration::from_secs(60));
        assert!(tree.is_animating());
    }

    #[test]
    fn reduced_motion_swaps_at_once() {
        let mut tree = UiTree::new(root("one"));
        tree.set_reduced_motion(true);
        let rest = painted(&tree.layout(SIZE), "one").unwrap();
        tree.set_root(root("two"));
        let frame = tree.layout(SIZE);
        assert_eq!(painted(&frame, "two"), Some(rest));
        assert_eq!(painted(&frame, "one"), None);
        assert!(!tree.is_animating());
    }

    #[test]
    fn a_height_that_is_not_finite_is_ignored() {
        let slot = |height: f32| {
            let ticker = Ticker::new("ticker", "one", Element::text("one")).height(height).render();
            UiTree::new(Element::column(vec![ticker, Element::text("below")]))
                .layout(SIZE)
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::PushClip(rect) => Some(rect.height),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(slot(f32::NAN), 20.0);
        assert_eq!(slot(f32::INFINITY), 20.0);
        assert_eq!(slot(-5.0), 0.0);
    }
}
