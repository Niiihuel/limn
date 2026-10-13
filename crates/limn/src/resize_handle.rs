//! The strip between two panes that the pointer drags to move their boundary.

use crate::{Component, ComponentPalette, Cursor, Element};

const DEFAULT_THICKNESS: f32 = 8.0;
const MIN_THICKNESS: f32 = 2.0;

/// The handle draws nothing at rest, so a gap between two panes can be the
/// handle itself; under the pointer it fills with `palette.hover`, and while
/// held with `palette.selected`. The pointer takes the resize shape over it
/// and keeps it through the drag. It is controlled: it reports `DragStarted`,
/// `DragMoved` and `Dropped` (or `DragCancelled`) on its ID with the
/// pointer's position, and the host, which owns the panes' sizes, adds how
/// far the pointer has travelled to the size the pane had when the drag
/// began. A press released without moving is a `Clicked` the host may ignore.
pub struct ResizeHandle {
    id: String,
    vertical: bool,
    thickness: f32,
    palette: ComponentPalette,
}

impl ResizeHandle {
    /// A handle that stands between two columns and moves sideways. It fills
    /// the height of the row that holds it.
    pub fn vertical(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self::new(id, true, palette)
    }

    /// A handle that lies between two rows and moves up and down. It fills
    /// the width of the column that holds it.
    pub fn horizontal(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self::new(id, false, palette)
    }

    fn new(id: impl Into<String>, vertical: bool, palette: ComponentPalette) -> Self {
        Self { id: id.into(), vertical, thickness: DEFAULT_THICKNESS, palette }
    }

    /// How much room the handle takes across the boundary, which is also its
    /// pointer target: 8 unless set and never under 2. Values that are not
    /// finite are ignored.
    pub fn thickness(mut self, thickness: f32) -> Self {
        if thickness.is_finite() {
            self.thickness = thickness.max(MIN_THICKNESS);
        }
        self
    }
}

impl Component for ResizeHandle {
    fn render(self) -> Element {
        let across = Some(self.thickness);
        let (width, height) = if self.vertical { (across, None) } else { (None, across) };
        self.palette
            .interactive_surface(Element::clickable(self.id, Element::spacer()))
            .draggable()
            .cursor(if self.vertical { Cursor::ResizeColumn } else { Cursor::ResizeRow })
            .size(width, height)
            // Rounded ends, whatever the thickness.
            .radius(self.thickness / 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HitKind, Point, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    /// Two panes 100 wide with a handle between them, in a window 60 tall.
    fn columns(handle: ResizeHandle) -> UiTree {
        let pane = || Element::spacer().size(Some(100.0), None);
        let mut tree = UiTree::new(Element::row(vec![pane(), handle.render(), pane()]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(208.0, 60.0));
        tree
    }

    #[test]
    fn a_vertical_handle_is_a_target_as_wide_as_its_thickness_and_as_tall_as_its_row() {
        let mut tree = columns(ResizeHandle::vertical("split", palette()));
        let frame = tree.layout(Size::new(208.0, 60.0));
        let region = frame.regions.iter().find(|region| region.id == "split").expect("the handle takes the pointer");
        assert_eq!(region.kind, HitKind::Button);
        assert_eq!((region.rect.x, region.rect.width, region.rect.height), (100.0, 8.0, 60.0));
    }

    #[test]
    fn a_horizontal_handle_fills_the_width_of_its_column() {
        let pane = || Element::spacer().size(None, Some(40.0));
        let mut tree = UiTree::new(Element::column(vec![pane(), ResizeHandle::horizontal("split", palette()).render(), pane()]));
        let frame = tree.layout(Size::new(120.0, 88.0));
        let region = frame.regions.iter().find(|region| region.id == "split").unwrap();
        assert_eq!((region.rect.y, region.rect.width, region.rect.height), (40.0, 120.0, 8.0));
    }

    #[test]
    fn dragging_it_reports_where_the_pointer_started_went_and_let_go() {
        let mut tree = columns(ResizeHandle::vertical("split", palette()));
        let at = |x: f32| Point::new(x, 30.0);
        tree.handle_event(UiEvent::PointerDown(at(104.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerMove(at(140.0))),
            vec![
                UiAction::DragStarted { id: "split".into(), position: at(104.0) },
                UiAction::DragMoved { id: "split".into(), position: at(140.0), over: None },
            ]
        );
        // The pointer may leave the strip: the drag follows it.
        assert_eq!(
            tree.handle_event(UiEvent::PointerMove(at(20.0))),
            vec![UiAction::DragMoved { id: "split".into(), position: at(20.0), over: None }]
        );
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(at(20.0))),
            vec![UiAction::Dropped { id: "split".into(), position: at(20.0), target: None }]
        );
    }

    #[test]
    fn the_pointer_takes_the_resize_shape_over_the_handle_and_through_the_drag() {
        let mut tree = columns(ResizeHandle::vertical("split", palette()));
        let at = |x: f32| Point::new(x, 30.0);
        tree.handle_event(UiEvent::PointerMove(at(50.0)));
        assert_eq!(tree.cursor(), None, "a pane is not a boundary");
        tree.handle_event(UiEvent::PointerMove(at(104.0)));
        assert_eq!(tree.cursor(), Some(Cursor::ResizeColumn));
        tree.handle_event(UiEvent::PointerDown(at(104.0)));
        tree.handle_event(UiEvent::PointerMove(at(20.0)));
        assert_eq!(tree.cursor(), Some(Cursor::ResizeColumn), "the drag keeps it away from the strip");
        tree.handle_event(UiEvent::PointerUp(at(20.0)));
        tree.handle_event(UiEvent::PointerMove(at(21.0)));
        assert_eq!(tree.cursor(), None);

        let pane = || Element::spacer().size(None, Some(40.0));
        let mut rows = UiTree::new(Element::column(vec![pane(), ResizeHandle::horizontal("split", palette()).render(), pane()]));
        rows.layout(Size::new(120.0, 88.0));
        rows.handle_event(UiEvent::PointerMove(Point::new(60.0, 44.0)));
        assert_eq!(rows.cursor(), Some(Cursor::ResizeRow));
    }

    #[test]
    fn a_thickness_that_is_not_a_size_is_ignored_and_a_thin_one_is_raised() {
        let width = |handle: ResizeHandle| {
            let mut tree = columns(handle);
            let frame = tree.layout(Size::new(208.0, 60.0));
            frame.regions.iter().find(|region| region.id == "split").unwrap().rect.width
        };
        assert_eq!(width(ResizeHandle::vertical("split", palette()).thickness(f32::NAN)), 8.0);
        assert_eq!(width(ResizeHandle::vertical("split", palette()).thickness(0.0)), 2.0);
        assert_eq!(width(ResizeHandle::vertical("split", palette()).thickness(12.0)), 12.0);
    }
}
