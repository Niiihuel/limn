//! A row or a column of panes with a resize handle between each pair.

use crate::{Component, ComponentPalette, Element, ResizeHandle};

/// One pane of a [`SplitView`]: its content and its share of the room.
pub struct Pane {
    element: Element,
    fraction: Option<f32>,
}

impl Pane {
    /// A pane that takes what the panes with a fraction leave, shared evenly
    /// with any other such pane. Its content is clipped to the pane.
    pub fn new(element: Element) -> Self {
        Self { element, fraction: None }
    }

    /// The pane's share of the room the handles leave, from 0 to 1. A value
    /// that is not finite is ignored; one outside the range is clamped.
    pub fn fraction(mut self, fraction: f32) -> Self {
        if fraction.is_finite() {
            self.fraction = Some(fraction.clamp(0.0, 1.0));
        }
        self
    }

    /// Take what the other panes leave, as a pane without a fraction does.
    pub fn flex(mut self) -> Self {
        self.fraction = None;
        self
    }
}

/// Panes side by side, or one over the other, with a [`ResizeHandle`]
/// between each pair. It is controlled: the host owns the fractions, and
/// dragging the handle after pane `i` reports `DragStarted`, `DragMoved` and
/// `Dropped` (or `DragCancelled`) on `{id}:handle:{i}`, as a `ResizeHandle`
/// does. The host keeps the fractions it had when the drag began and passes
/// them, with the pointer's travel, to [`SplitView::resized`], which keeps
/// every pane at or above its minimum. The layout knows no minimum: a pane
/// shrinks with its split until the host fits the fractions again.
pub struct SplitView {
    id: String,
    vertical: bool,
    panes: Vec<Pane>,
    thickness: Option<f32>,
    palette: ComponentPalette,
}

impl SplitView {
    /// Panes side by side, with vertical handles between them.
    pub fn row(id: impl Into<String>, panes: Vec<Pane>, palette: ComponentPalette) -> Self {
        Self { id: id.into(), vertical: true, panes, thickness: None, palette }
    }

    /// Panes one over the other, with horizontal handles between them.
    pub fn column(id: impl Into<String>, panes: Vec<Pane>, palette: ComponentPalette) -> Self {
        Self { id: id.into(), vertical: false, panes, thickness: None, palette }
    }

    /// The room each handle takes between two panes, as
    /// [`ResizeHandle::thickness`]: 8 unless set and never under 2.
    pub fn handle_thickness(mut self, thickness: f32) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// The fractions after the handle after pane `handle` has moved
    /// `delta_px` toward the end, in a split whose panes share `total_px`
    /// (its length less its handles). Only the two panes beside the handle
    /// change, and neither is moved below its minimum in `mins` (pixels, one
    /// for each pane; a missing one is 0). Any pane still under its minimum
    /// then takes the shortfall from the panes above theirs, so a `delta_px`
    /// of 0 only fits the fractions to the mins, as after the split's length
    /// changes. Panes whose minimums do not fit share the room in proportion
    /// to them. The result sums to 1.
    pub fn resized(fractions: &[f32], handle: usize, delta_px: f32, total_px: f32, mins: &[f32]) -> Vec<f32> {
        let clean = |value: f32| if value.is_finite() { value.max(0.0) } else { 0.0 };
        let sum: f32 = fractions.iter().copied().map(clean).sum();
        if sum <= 0.0 || !total_px.is_finite() || total_px <= 0.0 {
            return fractions.to_vec();
        }
        let mins: Vec<f32> = (0..fractions.len()).map(|i| clean(mins.get(i).copied().unwrap_or(0.0))).collect();
        let mut px: Vec<f32> = fractions.iter().map(|&f| clean(f) / sum * total_px).collect();
        if handle + 1 < px.len() && delta_px.is_finite() {
            let delta = delta_px.clamp(-(px[handle] - mins[handle]).max(0.0), (px[handle + 1] - mins[handle + 1]).max(0.0));
            px[handle] += delta;
            px[handle + 1] -= delta;
        }
        let needed: f32 = mins.iter().sum();
        if needed >= total_px {
            return mins.iter().map(|min| if needed > 0.0 { min / needed } else { 1.0 / mins.len() as f32 }).collect();
        }
        let shortfall: f32 = px.iter().zip(&mins).map(|(size, min)| (min - size).max(0.0)).sum();
        // Positive whenever there is a shortfall: the minimums fit, so the spare room exceeds it.
        let spare: f32 = px.iter().zip(&mins).map(|(size, min)| (size - min).max(0.0)).sum();
        for (size, min) in px.iter_mut().zip(&mins).filter(|_| shortfall > 0.0) {
            // The spare room shrinks in proportion, so no pane goes under its minimum.
            *size = if *size < *min { *min } else { *size - (*size - min) * shortfall / spare };
        }
        px.iter().map(|size| size / total_px).collect()
    }
}

impl Component for SplitView {
    fn render(self) -> Element {
        let fixed: f32 = self.panes.iter().filter_map(|pane| pane.fraction).sum();
        let flexible = self.panes.iter().filter(|pane| pane.fraction.is_none()).count() as f32;
        let rest = (1.0 - fixed).max(0.0) / flexible.max(1.0);
        let count = self.panes.len();
        let mut children = Vec::with_capacity(count * 2);
        for (index, pane) in self.panes.into_iter().enumerate() {
            // A flex of 0 would mean the content's natural size, not an empty pane.
            let share = pane.fraction.unwrap_or(rest).max(f32::MIN_POSITIVE);
            children.push(Element::column(vec![pane.element.flex(1.0)]).flex(share).clip_children());
            if index + 1 < count {
                let id = format!("{}:handle:{index}", self.id);
                let handle = if self.vertical {
                    ResizeHandle::vertical(id, self.palette)
                } else {
                    ResizeHandle::horizontal(id, self.palette)
                };
                let handle = match self.thickness {
                    Some(thickness) => handle.thickness(thickness),
                    None => handle,
                };
                children.push(handle.render());
            }
        }
        if self.vertical { Element::row(children) } else { Element::column(children) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementKind, Point, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn pane(id: &str) -> Element {
        Element::clickable(id, Element::spacer())
    }

    fn rects(split: SplitView, size: Size, ids: &[&str]) -> Vec<(f32, f32, f32, f32)> {
        let frame = UiTree::new(split.render()).layout(size);
        ids.iter()
            .map(|id| {
                let rect = frame.regions.iter().find(|region| region.id == *id).unwrap().rect;
                (rect.x, rect.y, rect.width, rect.height)
            })
            .collect()
    }

    #[test]
    fn a_row_shares_what_its_handles_leave_by_fraction_and_the_flex_pane_takes_the_rest() {
        let split = SplitView::row(
            "split",
            vec![Pane::new(pane("a")).fraction(0.25), Pane::new(pane("b")), Pane::new(pane("c")).fraction(0.25)],
            palette(),
        );
        assert_eq!(
            rects(split, Size::new(416.0, 60.0), &["a", "split:handle:0", "b", "split:handle:1", "c"]),
            vec![
                (0.0, 0.0, 100.0, 60.0),
                (100.0, 0.0, 8.0, 60.0),
                (108.0, 0.0, 200.0, 60.0),
                (308.0, 0.0, 8.0, 60.0),
                (316.0, 0.0, 100.0, 60.0),
            ]
        );
    }

    #[test]
    fn a_column_stacks_its_panes_with_horizontal_handles_of_the_given_thickness() {
        let split = SplitView::column("split", vec![Pane::new(pane("a")).fraction(0.5), Pane::new(pane("b")).flex()], palette())
            .handle_thickness(4.0);
        assert_eq!(
            rects(split, Size::new(120.0, 204.0), &["a", "split:handle:0", "b"]),
            vec![(0.0, 0.0, 120.0, 100.0), (0.0, 100.0, 120.0, 4.0), (0.0, 104.0, 120.0, 100.0)]
        );
    }

    #[test]
    fn a_flex_pane_left_no_room_is_empty_and_a_bad_thickness_is_ignored() {
        let split = SplitView::row("split", vec![Pane::new(pane("a")).fraction(1.0), Pane::new(pane("b"))], palette())
            .handle_thickness(f32::NAN);
        let frame = UiTree::new(split.render()).layout(Size::new(208.0, 60.0));
        let width = |id: &str| frame.regions.iter().find(|region| region.id == id).map_or(0.0, |region| region.rect.width);
        assert_eq!((width("a"), width("split:handle:0")), (200.0, 8.0));
        assert!(width("b") < 0.01, "the flex pane is not given its natural size");
        assert_eq!(Pane::new(Element::spacer()).fraction(f32::NAN).fraction, None);
        assert_eq!(Pane::new(Element::spacer()).fraction(2.0).fraction, Some(1.0));
    }

    #[test]
    fn each_pane_clips_its_content() {
        let split = SplitView::row("split", vec![Pane::new(pane("a")), Pane::new(pane("b"))], palette()).render();
        let ElementKind::Row(children) = &split.kind else { panic!("a row split is a row") };
        assert_eq!(children.len(), 3);
        assert!(children[0].style.clip_children && children[2].style.clip_children);
    }

    #[test]
    fn dragging_a_handle_reports_on_its_derived_id() {
        let split = SplitView::row("split", vec![Pane::new(pane("a")), Pane::new(pane("b")), Pane::new(pane("c"))], palette());
        let mut tree = UiTree::new(split.render());
        tree.set_reduced_motion(true);
        tree.layout(Size::new(316.0, 60.0));
        let at = |x: f32| Point::new(x, 30.0);
        tree.handle_event(UiEvent::PointerDown(at(212.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerMove(at(240.0))),
            vec![
                UiAction::DragStarted { id: "split:handle:1".into(), position: at(212.0) },
                UiAction::DragMoved { id: "split:handle:1".into(), position: at(240.0), over: None },
            ]
        );
    }

    fn close(got: Vec<f32>, want: &[f32]) {
        assert_eq!(got.len(), want.len());
        assert!(got.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-5), "{got:?} is not {want:?}");
    }

    #[test]
    fn resized_moves_only_the_panes_beside_the_handle_and_stops_at_their_minimums() {
        close(SplitView::resized(&[0.25, 0.5, 0.25], 0, 100.0, 400.0, &[]), &[0.5, 0.25, 0.25]);
        close(SplitView::resized(&[0.25, 0.5, 0.25], 1, -100.0, 400.0, &[]), &[0.25, 0.25, 0.5]);
        // The middle pane keeps 160 of its 200.
        close(SplitView::resized(&[0.25, 0.5, 0.25], 0, 100.0, 400.0, &[0.0, 160.0, 0.0]), &[0.35, 0.4, 0.25]);
        close(SplitView::resized(&[0.25, 0.5, 0.25], 0, -300.0, 400.0, &[80.0]), &[0.2, 0.55, 0.25]);
    }

    #[test]
    fn resized_by_nothing_fits_the_fractions_to_the_minimums() {
        // 400 shrank to 200: the first pane is raised to 80, the others give it room in proportion.
        close(SplitView::resized(&[0.2, 0.4, 0.4], 0, 0.0, 200.0, &[80.0, 0.0, 0.0]), &[0.4, 0.3, 0.3]);
        // Minimums that do not fit share the room in proportion to them.
        close(SplitView::resized(&[0.5, 0.5], 0, 0.0, 100.0, &[100.0, 300.0]), &[0.25, 0.75]);
        close(SplitView::resized(&[0.5, 0.5], 0, 0.0, 400.0, &[]), &[0.5, 0.5]);
    }

    #[test]
    fn resized_ignores_what_is_not_a_size() {
        close(SplitView::resized(&[1.0, 3.0], 0, f32::NAN, 400.0, &[]), &[0.25, 0.75]);
        close(SplitView::resized(&[0.5, 0.5], 5, 100.0, 400.0, &[]), &[0.5, 0.5]);
        assert_eq!(SplitView::resized(&[0.5, 0.5], 0, 100.0, f32::INFINITY, &[]), vec![0.5, 0.5]);
        assert_eq!(SplitView::resized(&[], 0, 100.0, 400.0, &[]), Vec::<f32>::new());
    }
}
