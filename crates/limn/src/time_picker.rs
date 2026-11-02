//! Compact controlled selector for a local 24-hour time of day.

use crate::components::{Component, ComponentPalette, ControlSize, IconButton, TextField};
use crate::{Access, Element, FontFamily, Icon, Role};

/// An editable `HH:MM` field plus keyboard-reachable hour and minute controls.
///
/// The field emits `InputChanged` using `{id}`. Adjustment buttons emit
/// `{id}:hour_up`, `{id}:hour_down`, `{id}:minute_up`, or `{id}:minute_down`.
/// Use [`TimePicker::adjusted_value`] to turn those actions into a validated
/// new value. Invalid partially typed text remains editable and is never
/// silently replaced by an adjustment.
pub struct TimePicker {
    id: String,
    value: String,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl TimePicker {
    /// A picker whose field shows `value`, a 24-hour `HH:MM` time.
    pub fn new(id: impl Into<String>, value: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// What the field is called to the platform, in place of the library's
    /// own word for it: "Departure" says more than "Time".
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// A disabled picker shows its time in muted ink and is passive: the field
    /// and the hour and minute buttons take no pointer or focus and report
    /// nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Whether `value` is exactly `HH:MM` with an hour below 24 and a minute below 60.
    pub fn is_valid_time(value: &str) -> bool {
        parse_time(value).is_some()
    }

    /// Applies a single button action to a valid 24-hour time. Hour adjustment
    /// wraps across midnight; minute adjustment carries into the hour.
    pub fn adjusted_value(action_id: &str, id: &str, current: &str) -> Option<String> {
        let (hour, minute) = parse_time(current)?;
        let action = action_id.strip_prefix(id)?.strip_prefix(':')?;
        let total = i32::from(hour) * 60 + i32::from(minute);
        let delta = match action {
            "hour_up" => 60,
            "hour_down" => -60,
            "minute_up" => 1,
            "minute_down" => -1,
            _ => return None,
        };
        let adjusted = (total + delta).rem_euclid(24 * 60);
        Some(format!("{:02}:{:02}", adjusted / 60, adjusted % 60))
    }

    fn adjustment(&self, label: &str, suffix: &str, icon: Icon) -> Element {
        let id = format!("{}:{suffix}", self.id);
        // An arrow alone is named by its unit and the value it goes to:
        // "Hour 10". With no valid time to step from, by its unit.
        let name = match Self::adjusted_value(&id, &self.id, &self.value) {
            Some(time) if suffix.starts_with("hour") => format!("{label} {}", &time[..2]),
            Some(time) => format!("{label} {}", &time[3..]),
            None => label.to_owned(),
        };
        let said = Access::new(Role::Button).name(name);
        IconButton::new(&id, icon, self.palette)
            .size(ControlSize::Compact)
            .variant(crate::ButtonVariant::Ghost)
            .enabled(self.enabled)
            .render()
            // Drawn disabled it is passive, and still there for the platform.
            .accessible(if self.enabled { said } else { said.id(id).disabled(true) })
    }

    /// One unit as a column: the arrow that raises it, its two digits, the
    /// arrow that lowers it, and its name under them.
    fn unit(&self, label: &str, display: String, down: &str, up: &str) -> Element {
        let ink = if self.enabled { self.palette.text } else { self.palette.muted };
        let centered = |element: Element| Element::row(vec![Element::spacer().flex(1.0), element, Element::spacer().flex(1.0)]);
        Element::column(vec![
            centered(self.adjustment(label, up, Icon::ArrowUp)),
            centered(
                Element::text(display)
                    .font_family(FontFamily::Sans)
                    .font_size(DIGITS)
                    .foreground(ink),
            )
            .size(None, Some(DIGITS + 8.0)),
            centered(self.adjustment(label, down, Icon::ArrowDown)),
            centered(
                Element::text(label)
                    .font_family(FontFamily::Sans)
                    .font_size(10.0)
                    .foreground(self.palette.muted),
            ),
        ])
        .gap(2.0)
        .size(Some(UNIT_WIDTH), None)
    }
}

/// The size of the hour's and the minute's digits.
const DIGITS: f32 = 24.0;
/// A unit's column: room for two digits at that size, so a change of value moves nothing.
const UNIT_WIDTH: f32 = 56.0;
/// The field and the card under it: two units, the colon between them and the card's padding.
const WIDTH: f32 = 2.0 * UNIT_WIDTH + 34.0;

impl Component for TimePicker {
    fn render(self) -> Element {
        let parsed = parse_time(&self.value);
        let hour = parsed
            .map(|(hour, _)| format!("{hour:02}"))
            .unwrap_or_else(|| "--".into());
        let minute = parsed
            .map(|(_, minute)| format!("{minute:02}"))
            .unwrap_or_else(|| "--".into());
        let input = TextField::new(&self.id, &self.value, crate::labels().time_placeholder, self.palette)
            .size(ControlSize::Compact)
            .width(WIDTH)
            .enabled(self.enabled)
            .accessibility_label(self.accessibility_label.as_deref().unwrap_or(crate::labels().time))
            .render();
        // Between the two, on the line of the digits: under one arrow and over the other.
        let colon = Element::column(vec![
            Element::spacer().size(None, Some(ControlSize::Compact.height() + 2.0)),
            Element::text(":")
                .font_family(FontFamily::Sans)
                .font_size(DIGITS)
                .foreground(self.palette.muted)
                .size(None, Some(DIGITS + 8.0)),
        ]);
        let controls = Element::row(vec![
            self.unit(crate::labels().hour, hour, "hour_down", "hour_up"),
            colon,
            self.unit(crate::labels().minute, minute, "minute_down", "minute_up"),
        ])
        .gap(4.0);
        // The same card as a date picker's calendar, so the two stand together.
        Element::column(vec![
            input,
            // As wide as its field, wherever it is put.
            Element::panel(controls)
                .padding(8.0)
                .background(self.palette.surface)
                .border(self.palette.border, 1.0)
                .radius(crate::tokens().radius.card)
                .size(Some(WIDTH), None),
        ])
        .gap(7.0)
    }
}

fn parse_time(value: &str) -> Option<(u8, u8)> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 2 || byte.is_ascii_digit())
    {
        return None;
    }
    let hour = value[..2].parse::<u8>().ok()?;
    let minute = value[3..5].parse::<u8>().ok()?;
    (hour < 24 && minute < 60).then_some((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn each_arrow_is_named_by_its_unit_and_the_value_it_goes_to() {
        let labels = crate::labels();
        let nodes = |value: &str, enabled: bool| {
            let picker = TimePicker::new("time", value, ComponentPalette::neutral_dark()).enabled(enabled);
            UiTree::new(picker.render()).layout(Size::new(240.0, 120.0)).access
        };
        for enabled in [true, false] {
            let nodes = nodes("09:59", enabled);
            let node = |id: &str| nodes.iter().find(|node| node.id == id).unwrap();
            assert_eq!(node("time:hour_up").name, format!("{} 10", labels.hour));
            assert_eq!(node("time:hour_down").name, format!("{} 08", labels.hour));
            assert_eq!(node("time:minute_up").name, format!("{} 00", labels.minute));
            assert_eq!(node("time:minute_down").name, format!("{} 58", labels.minute));
            let up = node("time:hour_up");
            assert_eq!((up.role, up.disabled, up.actionable), (Role::Button, !enabled, enabled));
        }
        // A time that cannot be stepped names the arrow by its unit alone.
        let nodes = nodes("9:5", true);
        assert_eq!(nodes.iter().find(|node| node.id == "time:minute_up").unwrap().name, labels.minute);
    }

    #[test]
    fn validates_exact_24_hour_times() {
        assert!(TimePicker::is_valid_time("00:00"));
        assert!(TimePicker::is_valid_time("23:59"));
        for value in ["24:00", "23:60", "9:30", "09:3", "09.30", "-1:30", "09:30x"] {
            assert!(!TimePicker::is_valid_time(value), "{value}");
        }
    }

    #[test]
    fn minute_adjustment_carries_and_hour_adjustment_wraps() {
        assert_eq!(
            TimePicker::adjusted_value("time:minute_up", "time", "23:59"),
            Some("00:00".into())
        );
        assert_eq!(
            TimePicker::adjusted_value("time:minute_down", "time", "00:00"),
            Some("23:59".into())
        );
        assert_eq!(
            TimePicker::adjusted_value("time:hour_up", "time", "23:15"),
            Some("00:15".into())
        );
        assert_eq!(
            TimePicker::adjusted_value("time:hour_down", "time", "00:15"),
            Some("23:15".into())
        );
        assert_eq!(
            TimePicker::adjusted_value("other:minute_up", "time", "23:59"),
            None
        );
        assert_eq!(
            TimePicker::adjusted_value("time:minute_up", "time", "invalid"),
            None
        );
    }

    #[test]
    fn adjustment_buttons_work_with_keyboard() {
        let picker = TimePicker::new("time", "09:00", ComponentPalette::neutral_dark()).render();
        let mut tree = UiTree::new(picker);
        let frame = tree.layout(Size::new(300.0, 100.0));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "time:minute_up"));
        assert!(tree.focus("time:minute_up"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
                alt: false
            }),
            vec![UiAction::Clicked {
                id: "time:minute_up".into()
            }]
        );
    }

    #[test]
    fn a_disabled_picker_takes_no_pointer_or_focus_and_keeps_its_place() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(300.0, 100.0);
        let picker = |enabled| TimePicker::new("time", "09:05", palette).enabled(enabled).render();
        // The hour and the minute, and the arrows either side of each.
        let places = |frame: &crate::Frame| {
            let digits: Vec<Point> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == "09" || text == "05" => Some(*position),
                    _ => None,
                })
                .collect();
            let arrows: Vec<Rect> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .collect();
            (digits, arrows)
        };
        let enabled = UiTree::new(picker(true)).layout(size);
        let up = enabled.regions.iter().find(|region| region.id == "time:minute_up").unwrap().rect;
        let mut tree = UiTree::new(picker(false));
        let frame = tree.layout(size);
        assert!(frame
            .regions
            .iter()
            .all(|region| !matches!(region.kind, HitKind::Button | HitKind::TextInput)));
        for id in ["time", "time:hour_up", "time:hour_down", "time:minute_up", "time:minute_down"] {
            assert!(!tree.focus(id), "{id}");
        }
        let point = Point::new(up.x + up.width / 2.0, up.y + up.height / 2.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(tree.handle_event(UiEvent::Key { key: Key::Enter, shift: false, ctrl: false, alt: false }).is_empty());
        assert!(frame.commands.iter().all(|command| match command {
            DrawCommand::Text { color, .. } => *color == palette.muted,
            DrawCommand::Icon { tint, .. } => *tint == palette.muted,
            _ => true,
        }));
        let (digits, arrows) = places(&frame);
        assert_eq!((digits.len(), arrows.len()), (2, 4));
        assert_eq!((digits, arrows), places(&enabled));
    }
}
