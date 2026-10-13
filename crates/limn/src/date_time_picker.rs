//! Composition of the shared calendar and 24-hour time controls.

use crate::components::{Component, ComponentPalette};
use crate::date_picker::DatePicker;
use crate::time_picker::TimePicker;
use crate::{Element, FontFamily};

/// A local date and time picker with separate controlled values and action IDs.
/// Keeping the date and time independent allows a form to preserve a partially
/// typed value while the other half changes. The caller combines them when it
/// validates or persists a local timestamp.
pub struct DateTimePicker {
    date_id: String,
    time_id: String,
    date_value: String,
    time_value: String,
    display_month: Option<(i32, u32)>,
    enabled: bool,
    palette: ComponentPalette,
}

impl DateTimePicker {
    /// A date picker above a time picker, each under its label from `Labels`. `date_value`
    /// is `YYYY-MM-DD` and `time_value` is `HH:MM`; each half reports under its own ID, as
    /// `DatePicker` and `TimePicker` do.
    pub fn new(
        date_id: impl Into<String>,
        time_id: impl Into<String>,
        date_value: impl Into<String>,
        time_value: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            date_id: date_id.into(),
            time_id: time_id.into(),
            date_value: date_value.into(),
            time_value: time_value.into(),
            display_month: None,
            enabled: true,
            palette,
        }
    }

    /// Show this month in the calendar instead of the selected date's. A year outside 1 to
    /// 9999 or a month outside 1 to 12 is ignored.
    pub fn display_month(mut self, year: i32, month: u32) -> Self {
        self.display_month = Some((year, month));
        self
    }

    /// A disabled picker disables its date and its time alike: both show
    /// their values in muted ink and nothing in them takes the pointer or
    /// focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for DateTimePicker {
    fn render(self) -> Element {
        let mut date =
            DatePicker::new(self.date_id, self.date_value, self.palette).enabled(self.enabled);
        if let Some((year, month)) = self.display_month {
            date = date.display_month(year, month);
        }
        let label = |text: &str| {
            Element::text(text)
                .font_family(FontFamily::Sans)
                .font_size(12.0)
                .foreground(self.palette.muted)
        };
        Element::column(vec![
            label(crate::labels().date),
            date.render(),
            label(crate::labels().local_time),
            TimePicker::new(self.time_id, self.time_value, self.palette)
                .enabled(self.enabled)
                // The label drawn over it.
                .accessibility_label(crate::labels().local_time)
                .render(),
        ])
        .gap(6.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiEvent, UiTree};

    #[test]
    fn exposes_date_and_time_as_independent_controls() {
        let picker = DateTimePicker::new(
            "task_date",
            "task_time",
            "2025-05-14",
            "09:30",
            ComponentPalette::neutral_dark(),
        )
        .display_month(2025, 5)
        .render();
        let mut tree = UiTree::new(picker);
        let frame = tree.layout(Size::new(340.0, 480.0));
        for id in [
            "task_date",
            "task_date:day:2025-05-14",
            "task_time",
            "task_time:minute_up",
        ] {
            assert!(frame.regions.iter().any(|region| region.id == id), "{id}");
        }
    }

    #[test]
    fn calendar_and_time_controls_fit_a_narrow_modal_body() {
        let picker = DateTimePicker::new(
            "task_date",
            "task_time",
            "2025-05-14",
            "09:30",
            ComponentPalette::neutral_dark(),
        )
        .display_month(2025, 5)
        .render();
        let mut tree = UiTree::new(picker);
        let frame = tree.layout(Size::new(248.0, 480.0));
        for region in frame.regions.iter().filter(|region| {
            region.id.starts_with("task_date") || region.id.starts_with("task_time")
        }) {
            assert!(
                region.rect.x + region.rect.width <= 248.5,
                "{} overflows: {:?}",
                region.id,
                region.rect
            );
        }
    }

    #[test]
    fn a_disabled_picker_disables_its_date_and_its_time_and_keeps_its_place() {
        let size = Size::new(340.0, 480.0);
        let picker = |enabled| {
            DateTimePicker::new(
                "task_date",
                "task_time",
                "2025-05-14",
                "09:30",
                ComponentPalette::neutral_dark(),
            )
            .display_month(2025, 5)
            .enabled(enabled)
            .render()
        };
        // The day numbers, the hour and the minute: everything under both fields.
        let places = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text.len() <= 2 => {
                        Some((text.clone(), *position))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let enabled = UiTree::new(picker(true)).layout(size);
        let mut tree = UiTree::new(picker(false));
        let frame = tree.layout(size);
        assert!(frame
            .regions
            .iter()
            .all(|region| !matches!(region.kind, HitKind::Button | HitKind::TextInput)));
        for id in ["task_date", "task_date:day:2025-05-14", "task_time", "task_time:minute_up"] {
            assert!(!tree.focus(id), "{id}");
            let rect = enabled.regions.iter().find(|region| region.id == id).unwrap().rect;
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty(), "{id}");
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty(), "{id}");
        }
        assert!(tree.handle_event(UiEvent::Key { key: Key::Enter, shift: false, ctrl: false }).is_empty());
        assert_eq!(places(&frame), places(&enabled));
    }
}
