//! A set of actions that sits in a row when there is room and in a column
//! when there is not.

use crate::{Component, Element};

// One distance between actions in both directions.
const GAP: f32 = 6.0;

/// The host decides when the group stacks and passes rendered buttons; the
/// group owns their order, spacing and alignment.
pub struct ButtonGroup {
    buttons: Vec<Element>,
    leading: Option<Element>,
    stacked: bool,
    align_end: bool,
    reverse_when_stacked: bool,
    sizing: ButtonGroupSizing,
}

/// Density of a [`ButtonGroup`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonGroupSizing {
    /// Space between actions in a row.
    pub gap: f32,
    /// Space between actions in a column.
    pub stacked_gap: f32,
}

impl ButtonGroupSizing {
    /// Actions 6 px apart in both directions.
    pub const DEFAULT: Self = Self {
        gap: GAP,
        stacked_gap: GAP,
    };
}

impl ButtonGroup {
    /// `buttons` in reading order, the primary action last.
    pub fn new(buttons: Vec<Element>) -> Self {
        Self {
            buttons,
            leading: None,
            stacked: false,
            align_end: false,
            reverse_when_stacked: false,
            sizing: ButtonGroupSizing::DEFAULT,
        }
    }

    /// Lay the actions in a column, each as wide as the group.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Push the actions to the far edge of the row.
    pub fn align_end(mut self) -> Self {
        self.align_end = true;
        self
    }

    /// An element that stays at the near edge, before the actions: at the
    /// start of the row, and first in the column whatever the actions' order.
    pub fn leading(mut self, leading: Element) -> Self {
        self.leading = Some(leading);
        self
    }

    /// Reverse the actions in the column so the primary one comes first.
    pub fn reverse_when_stacked(mut self, reverse: bool) -> Self {
        self.reverse_when_stacked = reverse;
        self
    }

    /// The group's density. [`ButtonGroupSizing::DEFAULT`] unless set.
    pub fn sizing(mut self, sizing: ButtonGroupSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for ButtonGroup {
    fn render(self) -> Element {
        // A profile is plain data from the host; a bad number stops here.
        let px = |value: f32| if value.is_finite() { value.max(0.0) } else { 0.0 };
        let mut children: Vec<Element> = self.leading.into_iter().collect();
        if self.stacked {
            // Only the actions swap; `leading` stays before them.
            if self.reverse_when_stacked {
                children.extend(self.buttons.into_iter().rev());
            } else {
                children.extend(self.buttons);
            }
            return Element::column(children).gap(px(self.sizing.stacked_gap));
        }
        if self.align_end {
            children.push(Element::spacer().flex(1.0));
        }
        children.extend(self.buttons);
        Element::row(children).gap(px(self.sizing.gap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Button, ComponentPalette, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn buttons() -> Vec<Element> {
        let palette = ComponentPalette::neutral_dark();
        ["cancel", "save"]
            .into_iter()
            .map(|id| Button::new(id, id, palette).width(80.0).render())
            .collect()
    }

    fn fluid() -> Vec<Element> {
        let palette = ComponentPalette::neutral_dark();
        ["cancel", "save"]
            .into_iter()
            .map(|id| Button::new(id, id, palette).render())
            .collect()
    }

    fn rects(group: ButtonGroup, width: f32) -> Vec<(String, Rect)> {
        let mut tree = UiTree::new(Element::column(vec![
            group.render(),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(width, 200.0));
        frame
            .regions
            .iter()
            .map(|region| (region.id.clone(), region.rect))
            .collect()
    }

    #[test]
    fn sizing_sets_one_gap_for_the_row_and_another_for_the_column() {
        let sizing = ButtonGroupSizing { gap: 10.0, stacked_gap: 8.0 };
        let row = rects(ButtonGroup::new(buttons()).sizing(sizing), 360.0);
        assert_eq!(rect(&row, "save").x - rect(&row, "cancel").x, 80.0 + 10.0);
        let column = rects(ButtonGroup::new(buttons()).sizing(sizing).stacked(true), 360.0);
        let cancel = rect(&column, "cancel");
        assert_eq!(rect(&column, "save").y - cancel.y, cancel.height + 8.0);

        let plain = ButtonGroup::new(buttons()).render();
        let default = ButtonGroup::new(buttons()).sizing(ButtonGroupSizing::DEFAULT).render();
        assert_eq!(format!("{plain:?}"), format!("{default:?}"));
        let bad = ButtonGroupSizing { gap: f32::NAN, stacked_gap: -3.0 };
        assert_eq!(ButtonGroup::new(buttons()).sizing(bad).render().style.gap, 0.0);
        assert_eq!(ButtonGroup::new(buttons()).sizing(bad).stacked(true).render().style.gap, 0.0);
    }

    fn rect(rects: &[(String, Rect)], id: &str) -> Rect {
        rects.iter().find(|(name, _)| name == id).expect(id).1
    }

    #[test]
    fn a_row_keeps_reading_order_six_pixels_apart() {
        let rects = rects(ButtonGroup::new(buttons()), 400.0);
        let (cancel, save) = (rect(&rects, "cancel"), rect(&rects, "save"));
        assert_eq!(cancel.x, 0.0);
        assert_eq!(save.x, cancel.x + cancel.width + 6.0);
        assert_eq!(save.y, cancel.y);
    }

    #[test]
    fn align_end_sends_the_actions_to_the_far_edge_and_keeps_leading_near() {
        let palette = ComponentPalette::neutral_dark();
        let group = ButtonGroup::new(buttons())
            .align_end()
            .leading(Button::new("help", "help", palette).width(60.0).render());
        let rects = rects(group, 400.0);
        assert_eq!(rect(&rects, "help").x, 0.0);
        let save = rect(&rects, "save");
        assert_eq!(save.x + save.width, 400.0);
        assert_eq!(rect(&rects, "cancel").x, 400.0 - 80.0 - 6.0 - 80.0);
    }

    #[test]
    fn stacked_fills_the_width_and_reverse_puts_the_primary_first() {
        for width in [180.0, 360.0] {
            let plain = rects(ButtonGroup::new(fluid()).stacked(true).align_end(), width);
            let (cancel, save) = (rect(&plain, "cancel"), rect(&plain, "save"));
            assert_eq!((cancel.x, cancel.width), (0.0, width));
            assert_eq!(save.y, cancel.y + cancel.height + 6.0);

            let reversed = rects(
                ButtonGroup::new(fluid()).stacked(true).reverse_when_stacked(true),
                width,
            );
            let (cancel, save) = (rect(&reversed, "cancel"), rect(&reversed, "save"));
            assert_eq!(cancel.y, save.y + save.height + 6.0);
        }
        // In a row the option does nothing.
        let row = rects(ButtonGroup::new(buttons()).reverse_when_stacked(true), 400.0);
        assert!(rect(&row, "cancel").x < rect(&row, "save").x);
    }

    #[test]
    fn leading_stays_first_in_the_column_with_and_without_reverse() {
        let palette = ComponentPalette::neutral_dark();
        for reverse in [false, true] {
            let group = ButtonGroup::new(fluid())
                .stacked(true)
                .reverse_when_stacked(reverse)
                .leading(Button::new("help", "help", palette).render());
            let rects = rects(group, 180.0);
            let (help, cancel, save) =
                (rect(&rects, "help"), rect(&rects, "cancel"), rect(&rects, "save"));
            assert_eq!((help.x, help.y, help.width), (0.0, 0.0, 180.0));
            let (first, second) = if reverse { (save, cancel) } else { (cancel, save) };
            assert_eq!(first.y, help.y + help.height + 6.0);
            assert_eq!(second.y, first.y + first.height + 6.0);
        }
    }

    #[test]
    fn every_action_still_reports_its_own_id() {
        let mut tree = UiTree::new(ButtonGroup::new(buttons()).stacked(true).render());
        let frame = tree.layout(Size::new(200.0, 200.0));
        for id in ["cancel", "save"] {
            let rect = frame.regions.iter().find(|region| region.id == id).unwrap().rect;
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: id.into() }]
            );
        }
    }
}
