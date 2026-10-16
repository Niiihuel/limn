//! A slot in the flow that closes when its content leaves.

use crate::{Component, Element};

/// The host renders this every frame and passes the content while there is
/// any. When the content stops being passed, the slot keeps its place and
/// closes on a spring, in one and a half `move_ms`, while the last paint
/// fades, so what follows glides up instead of jumping. The closing paint takes no pointer input.
pub struct Collapse {
    id: String,
    content: Option<Element>,
    expand: bool,
}

impl Collapse {
    /// A slot named `id`. Pass `Some(content)` while there is content and `None` once it
    /// leaves, with the same `id` on every rebuild.
    pub fn new(id: impl Into<String>, content: Option<Element>) -> Self {
        Self {
            id: id.into(),
            content,
            expand: false,
        }
    }

    /// Also open from zero height, on the same spring, when content arrives in a
    /// slot that was rendered empty. Without this, arriving content takes its
    /// height at once. Content there when the slot first appears never opens.
    pub fn expand(mut self) -> Self {
        self.expand = true;
        self
    }
}

impl Component for Collapse {
    fn render(self) -> Element {
        Element::column(self.content.into_iter().collect()).animate_height(self.id, self.expand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    const SIZE: Size = Size::new(300.0, 200.0);

    fn slot(open: bool) -> Collapse {
        Collapse::new(
            "slot",
            open.then(|| Element::clickable("inside", Element::text("content")).size(None, Some(40.0))),
        )
    }

    fn root(slot: Collapse) -> Element {
        Element::column(vec![
            Element::text("above").size(None, Some(20.0)),
            slot.render(),
            Element::text("below").size(None, Some(20.0)),
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

    fn exit() -> Duration {
        // The slot moves on a spring; this is the time it takes to all but arrive.
        crate::tree::slot_settle()
    }

    #[test]
    fn the_slot_closes_on_its_spring_and_what_follows_moves_with_it() {
        // In the past, so the wall clock `is_animating` reads is after the end.
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(root(slot(true)));
        let open = tree.layout_at(SIZE, start);
        let (content_y, content_alpha) = painted(&open, "content").unwrap();
        let below_open = painted(&open, "below").unwrap().0;
        assert!(open.regions.iter().any(|region| region.id == "inside"));

        tree.set_root(root(slot(false)));
        // The frame in which the content leaves has not moved yet.
        let frame = tree.layout_at(SIZE, start);
        assert_eq!(painted(&frame, "below").unwrap().0, below_open);

        let frame = tree.layout_at(SIZE, start + exit() / 2);
        let below = painted(&frame, "below").unwrap().0;
        assert!(below < below_open && below > below_open - 40.0, "{below}");
        // The last paint stays where it was, fading, clipped to what is left.
        let (y, alpha) = painted(&frame, "content").unwrap();
        assert_eq!(y, content_y);
        assert!(alpha < content_alpha && alpha > 0);
        let left = below - (below_open - 40.0);
        assert!(frame.commands.iter().any(
            |command| matches!(command, DrawCommand::PushClip(rect) if (rect.height - left).abs() < 0.01)
        ));
        // It is paint only: nothing is there to click.
        assert!(frame.regions.iter().all(|region| region.id != "inside"));
        let inside = Point::new(10.0, 22.0);
        tree.handle_event(UiEvent::PointerDown(inside));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(inside)), Vec::<UiAction>::new());

        let frame = tree.layout_at(SIZE, start + exit() * 3);
        assert_eq!(painted(&frame, "below").unwrap().0, below_open - 40.0);
        assert_eq!(painted(&frame, "content"), None);
        assert!(!tree.is_animating());

        // A close that has only just begun keeps the window painting.
        tree.set_root(root(slot(true)));
        tree.layout_at(SIZE, Instant::now() + Duration::from_secs(60));
        tree.set_root(root(slot(false)));
        tree.layout_at(SIZE, Instant::now() + Duration::from_secs(60));
        assert!(tree.is_animating());
    }

    #[test]
    fn content_arrives_at_once_unless_the_slot_expands() {
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(root(slot(false)));
        let closed = painted(&tree.layout_at(SIZE, start), "below").unwrap().0;
        tree.set_root(root(slot(true)));
        let frame = tree.layout_at(SIZE, start);
        assert_eq!(painted(&frame, "below").unwrap().0, closed + 40.0);
        assert!(!tree.is_animating());

        let mut tree = UiTree::new(root(slot(false).expand()));
        tree.layout_at(SIZE, start);
        tree.set_root(root(slot(true).expand()));
        assert_eq!(painted(&tree.layout_at(SIZE, start), "below").unwrap().0, closed);
        let enter = crate::tree::slot_settle();
        let frame = tree.layout_at(SIZE, start + enter / 2);
        let below = painted(&frame, "below").unwrap().0;
        assert!(below > closed && below < closed + 40.0, "{below}");
        // The control inside is reachable only where the slot shows it.
        let region = frame.regions.iter().find(|region| region.id == "inside").unwrap().rect;
        assert!((region.height - (below - closed)).abs() < 0.01, "{region:?}");
        let frame = tree.layout_at(SIZE, start + enter * 3);
        assert_eq!(painted(&frame, "below").unwrap().0, closed + 40.0);
        assert!(!tree.is_animating());

        // Content already there when the slot is first seen does not open.
        let mut tree = UiTree::new(root(slot(true).expand()));
        assert_eq!(painted(&tree.layout_at(SIZE, start), "below").unwrap().0, closed + 40.0);
        assert!(!tree.is_animating());
    }

    #[test]
    fn a_control_revealed_inside_an_opening_slot_is_aimed_at_its_final_place() {
        let page = |open: bool| {
            let mut rows: Vec<_> =
                (0..20).map(|row| Element::text(format!("row {row}")).size(None, Some(20.0))).collect();
            rows.insert(10, slot(open).expand().render());
            Element::scroll("page", Element::column(rows))
        };
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(page(false));
        tree.layout_at(SIZE, start);
        tree.set_root(page(true));
        tree.reveal("inside");
        // The slot is open in the frame that locates the control.
        let frame = tree.layout_at(SIZE, start);
        let region = frame.regions.iter().find(|region| region.id == "inside").unwrap().rect;
        assert_eq!(region.height, 40.0);
        assert!(region.y >= 0.0 && region.y + region.height <= SIZE.height, "{region:?}");
        assert!(!tree.is_animating());
    }

    #[test]
    fn content_that_returns_while_closing_reopens_from_where_it_is() {
        let start = Instant::now();
        let mut tree = UiTree::new(root(slot(true)));
        let open = painted(&tree.layout_at(SIZE, start), "below").unwrap().0;
        tree.set_root(root(slot(false)));
        tree.layout_at(SIZE, start);
        let half = painted(&tree.layout_at(SIZE, start + exit() / 2), "below").unwrap().0;
        tree.set_root(root(slot(true)));
        let reopened = painted(&tree.layout_at(SIZE, start + exit() / 2), "below").unwrap().0;
        assert!((reopened - half).abs() < 1e-3, "{reopened} after {half}");
        let later = start + exit() / 2 + Duration::from_secs(1);
        assert_eq!(painted(&tree.layout_at(SIZE, later), "below").unwrap().0, open);
    }

    #[test]
    fn reduced_motion_removes_the_slot_at_once() {
        let mut tree = UiTree::new(root(slot(true).expand()));
        tree.set_reduced_motion(true);
        let open = painted(&tree.layout(SIZE), "below").unwrap().0;
        tree.set_root(root(slot(false).expand()));
        let frame = tree.layout(SIZE);
        assert_eq!(painted(&frame, "below").unwrap().0, open - 40.0);
        assert_eq!(painted(&frame, "content"), None);
        assert!(!tree.is_animating());
        tree.set_root(root(slot(true).expand()));
        assert_eq!(painted(&tree.layout(SIZE), "below").unwrap().0, open);
        assert!(!tree.is_animating());
    }

    #[test]
    fn closing_above_a_scrolled_viewport_keeps_the_scroll_offset() {
        let page = |open: bool| {
            let mut rows = vec![slot(open).render()];
            rows.extend((0..20).map(|row| Element::text(format!("row {row}")).size(None, Some(20.0))));
            Element::scroll("page", Element::column(rows))
        };
        let start = Instant::now();
        let mut tree = UiTree::new(page(true));
        tree.layout_at(SIZE, start);
        let position = Point::new(10.0, 100.0);
        tree.handle_event(UiEvent::ScrollDirect { position, delta_y: 100.0 });
        tree.handle_event(UiEvent::ScrollDirect { position, delta_y: -200.0 });
        tree.handle_event(UiEvent::ScrollDirect { position, delta_y: 100.0 });
        let offset = tree.scroll_offset("page");
        assert!(offset > 40.0, "{offset}");
        let row = |frame: &crate::Frame| painted(frame, "row 10").unwrap().0;
        let before = row(&tree.layout_at(SIZE, start));
        tree.set_root(page(false));
        tree.layout_at(SIZE, start);
        let half = row(&tree.layout_at(SIZE, start + exit() / 2));
        assert!(half < before && half > before - 40.0, "{half}");
        assert_eq!(row(&tree.layout_at(SIZE, start + exit() * 3)), before - 40.0);
        assert_eq!(tree.scroll_offset("page"), offset);
    }
}
