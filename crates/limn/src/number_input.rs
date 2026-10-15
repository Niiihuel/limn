//! A field for a number with a button that lowers it and one that raises it.

use crate::components::{Component, ComponentPalette, ControlSize, IconButton, TextField};
use crate::{Access, CornerRadii, Element, ElementKind, Icon, Role};

/// The whole control unless a width is set.
const DEFAULT_WIDTH: f32 = 160.0;
/// The least room the text keeps beside the two buttons.
const MIN_FIELD_WIDTH: f32 = 80.0;
/// The hairline around the buttons and between them, as wide as the field's border.
const RULE: f32 = 1.0;

/// Where a [`NumberInput`]'s two step buttons sit relative to its editable value.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NumberInputButtonPlacement {
    /// Both buttons follow the field: value, decrease, increase.
    #[default]
    End,
    /// The field and its text sit between decrease and increase.
    Split,
}

/// An editable field joined to a button that lowers its number and one that
/// raises it.
///
/// The host owns the text. The field emits `InputChanged` using `{id}`; the
/// buttons emit `Clicked` with `{id}:decrement` and `{id}:increment`. Use
/// [`NumberInput::adjusted_value`] to turn those clicks into the next text.
/// A button with nowhere to go, at `min` or at `max`, is drawn disabled.
pub struct NumberInput {
    id: String,
    value: String,
    placeholder: String,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    decimals: u8,
    width: f32,
    size: ControlSize,
    button_placement: NumberInputButtonPlacement,
    enabled: bool,
    accessibility_label: Option<String>,
    step_labels: (String, String),
    palette: ComponentPalette,
}

impl NumberInput {
    /// An input whose field shows `value`: the text of a number, or what has
    /// been typed of one so far.
    pub fn new(id: impl Into<String>, value: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: String::new(),
            min: None,
            max: None,
            step: 1.0,
            decimals: 0,
            width: DEFAULT_WIDTH,
            size: ControlSize::Regular,
            button_placement: NumberInputButtonPlacement::End,
            enabled: true,
            accessibility_label: None,
            step_labels: ("Decrease".into(), "Increase".into()),
            palette,
        }
    }

    /// The least value the buttons go to; none unless set. The button that
    /// lowers is disabled at it and below it. A value that is not finite is ignored.
    pub fn min(mut self, min: f64) -> Self {
        if min.is_finite() {
            self.min = Some(min);
        }
        self
    }

    /// The greatest value the buttons go to; none unless set. The button that
    /// raises is disabled at it and above it. A value that is not finite is ignored.
    pub fn max(mut self, max: f64) -> Self {
        if max.is_finite() {
            self.max = Some(max);
        }
        self
    }

    /// How far one click moves the value: 1 unless set. A step that is not
    /// finite or not above zero is ignored.
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    /// The digits after the point a button writes its result with: none
    /// unless set, so a step of 0.1 needs `decimals(1)`.
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// The width of the field and its buttons together, in logical pixels:
    /// 160 unless set, and never so narrow that the text has under 80. A
    /// width that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width;
        }
        self
    }

    /// Take the height of a control size, as a [`TextField`] does: never under 28.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Put the step buttons after the value, the default, or on either side
    /// of a centered editable value. IDs, actions and range behavior are unchanged.
    pub fn button_placement(mut self, placement: NumberInputButtonPlacement) -> Self {
        self.button_placement = placement;
        self
    }

    /// The text shown while the field is empty, which also names the field
    /// to the platform unless [`accessibility_label`](Self::accessibility_label) does.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// What the field is called to the platform: "Quantity". Nothing the
    /// input draws names it.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// The words the two buttons are named with for the platform, the one
    /// that lowers first. They are the English "Decrease" and "Increase"
    /// unless set: the library's labels have no words for them.
    pub fn step_labels(mut self, decrement: impl Into<String>, increment: impl Into<String>) -> Self {
        self.step_labels = (decrement.into(), increment.into());
        self
    }

    /// A disabled input shows its text in muted ink and is passive: the
    /// field and the buttons take no pointer or focus and report nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// The number `text` writes, ignoring spaces around it: an optional
    /// sign, digits and at most one point, as `-1.5`, `+3`, `.5` or `2.`.
    /// `None` for anything else, such as an empty text, `-`, `1e3` or `1,5`.
    pub fn parse(text: &str) -> Option<f64> {
        let text = text.trim();
        let digits = text.strip_prefix(['+', '-']).unwrap_or(text);
        let plain = digits.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
            && digits.bytes().any(|byte| byte.is_ascii_digit())
            && digits.bytes().filter(|byte| *byte == b'.').count() <= 1;
        // So many digits that they are no finite number are not one either.
        text.parse::<f64>().ok().filter(|value| plain && value.is_finite())
    }

    /// Whether `text` is a number, as [`parse`](Self::parse) reads it.
    pub fn is_valid_number(text: &str) -> bool {
        Self::parse(text).is_some()
    }

    /// The text a click on one of this input's buttons leads to, written
    /// with `decimals` digits after the point. `None` for an action that is
    /// not one of its buttons, so it can be tried on every click, and when
    /// the button has nowhere to go.
    ///
    /// The value moves by `step` (1 when it is not finite or not above zero)
    /// and stops at `min` and `max`; a bound that is `None` or not finite is
    /// no bound. A value outside the range comes back to its nearest end,
    /// and never moves away from it. An empty text starts from zero, or from
    /// the end of the range nearest to zero. Any other text that is not a
    /// number is left as it is.
    pub fn adjusted_value(
        action_id: &str,
        id: &str,
        current: &str,
        min: Option<f64>,
        max: Option<f64>,
        step: f64,
        decimals: u8,
    ) -> Option<String> {
        let up = match action_id.strip_prefix(id)?.strip_prefix(':')? {
            "increment" => true,
            "decrement" => false,
            _ => return None,
        };
        let step = if step.is_finite() && step > 0.0 { step } else { 1.0 };
        let min = min.filter(|min| min.is_finite()).unwrap_or(f64::NEG_INFINITY);
        let max = max.filter(|max| max.is_finite()).unwrap_or(f64::INFINITY);
        let within = |value: f64| value.max(min).min(max);
        let write = |value: f64| {
            let text = format!("{value:.*}", usize::from(decimals));
            // A value that rounds to nothing is written without its sign.
            match text.strip_prefix('-') {
                Some(rest) if rest.bytes().all(|byte| byte == b'0' || byte == b'.') => rest.to_owned(),
                _ => text,
            }
        };
        if current.trim().is_empty() {
            return Some(write(within(0.0)));
        }
        let value = Self::parse(current)?;
        let text = write(within(if up { value + step } else { value - step }));
        // What is written is what counts: a button never leaves the value
        // where it was, nor takes it the other way.
        let next = Self::parse(&text)?;
        (if up { next > value } else { next < value }).then_some(text)
    }

    /// One of the two buttons, square inside the hairline around them.
    fn button(&self, suffix: &str, label: &str, icon: Icon, radii: CornerRadii, side: f32) -> Element {
        let id = format!("{}:{suffix}", self.id);
        let next = Self::adjusted_value(&id, &self.id, &self.value, self.min, self.max, self.step, self.decimals);
        // Named by what it does and the value it goes to: "Increase 5".
        // With nowhere to go, by what it does.
        let name = match &next {
            Some(next) => format!("{label} {next}"),
            None => label.to_owned(),
        };
        let enabled = self.enabled && next.is_some();
        let said = Access::new(Role::Button).name(name);
        IconButton::new(&id, icon, self.palette)
            .variant(crate::ButtonVariant::Ghost)
            .enabled(enabled)
            .render()
            // Opaque, so the hairline behind the two shows only around them.
            .background(self.palette.raised)
            .corner_radii(radii)
            .size(Some(side), Some(side - 2.0 * RULE))
            // Drawn disabled it is passive, and still there for the platform.
            .accessible(if enabled { said } else { said.id(id).disabled(true) })
    }
}

impl Component for NumberInput {
    fn render(self) -> Element {
        let height = self.size.height().max(28.0);
        let radius = crate::tokens().radius.control;
        let buttons_width = 2.0 * (height + RULE);
        let field_width = (self.width - buttons_width).max(MIN_FIELD_WIDTH);
        let mut field = TextField::new(&self.id, &self.value, &self.placeholder, self.palette)
            .size(self.size)
            .width(field_width)
            .enabled(self.enabled);
        if let Some(name) = &self.accessibility_label {
            field = field.accessibility_label(name);
        }
        let square = CornerRadii::same(0.0);
        let rule = || Element::spacer().size(Some(RULE), Some(RULE));
        if self.button_placement == NumberInputButtonPlacement::Split {
            let mut field = field.render().corner_radii(square).center_content();
            // A disabled TextField keeps its value in a passive text child.
            if let ElementKind::Panel(content) = &mut field.kind {
                content.style.center_content = true;
            }
            let side = |suffix, label: &str, icon, left| {
                let radii = if left {
                    CornerRadii {
                        top_left: radius,
                        bottom_left: radius,
                        ..square
                    }
                } else {
                    CornerRadii {
                        top_right: radius,
                        bottom_right: radius,
                        ..square
                    }
                };
                let inner = if left {
                    CornerRadii {
                        top_left: radius - RULE,
                        bottom_left: radius - RULE,
                        ..square
                    }
                } else {
                    CornerRadii {
                        top_right: radius - RULE,
                        bottom_right: radius - RULE,
                        ..square
                    }
                };
                let button = self.button(suffix, label, icon, inner, height);
                let row = if left {
                    vec![rule(), button]
                } else {
                    vec![button, rule()]
                };
                Element::panel(Element::column(vec![rule(), Element::row(row), rule()]))
                    .background(self.palette.border)
                    .corner_radii(radii)
                    .size(Some(height + RULE), Some(height))
            };
            return Element::row(vec![
                side("decrement", &self.step_labels.0, Icon::Minus, true),
                field,
                side("increment", &self.step_labels.1, Icon::Add, false),
            ])
            .size(Some(field_width + buttons_width), Some(height));
        }
        // The field's own border closes the buttons on its side; a fill of
        // the border's color behind them is the line on the other three and
        // between the two.
        let buttons = Element::panel(Element::column(vec![
            rule(),
            Element::row(vec![
                self.button("decrement", &self.step_labels.0, Icon::Minus, square, height),
                rule(),
                self.button(
                    "increment",
                    &self.step_labels.1,
                    Icon::Add,
                    CornerRadii { top_right: radius - RULE, bottom_right: radius - RULE, ..square },
                    height,
                ),
                rule(),
            ]),
            rule(),
        ]))
        .background(self.palette.border)
        .corner_radii(CornerRadii { top_right: radius, bottom_right: radius, ..square })
        .size(Some(buttons_width), Some(height));
        Element::row(vec![
            field.render().corner_radii(CornerRadii { top_left: radius, bottom_left: radius, ..square }),
            buttons,
        ])
        .size(Some(field_width + buttons_width), Some(height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const WINDOW: Size = Size::new(400.0, 100.0);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("no region {id}")).rect
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn step(action: &str, current: &str) -> Option<String> {
        NumberInput::adjusted_value(action, "n", current, Some(0.0), Some(10.0), 1.0, 0)
    }

    #[test]
    fn reads_a_sign_digits_and_one_point_and_nothing_else() {
        for (text, value) in [("5", 5.0), ("-1.5", -1.5), ("+3", 3.0), (".5", 0.5), ("2.", 2.0), (" 12 ", 12.0), ("007", 7.0)] {
            assert_eq!(NumberInput::parse(text), Some(value), "{text}");
            assert!(NumberInput::is_valid_number(text), "{text}");
        }
        let endless = "9".repeat(400);
        for text in ["", " ", "-", "+", ".", "-.", "1e3", "1,5", "1.2.3", "--1", "1-", "12a", "inf", "NaN", "0x10", "1 2", &endless] {
            assert_eq!(NumberInput::parse(text), None, "{text}");
            assert!(!NumberInput::is_valid_number(text), "{text}");
        }
    }

    #[test]
    fn a_button_moves_the_value_by_a_step_and_stops_at_the_ends() {
        assert_eq!(step("n:increment", "4"), Some("5".into()));
        assert_eq!(step("n:decrement", "4"), Some("3".into()));
        assert_eq!(step("n:increment", "9.5"), Some("10".into()));
        // At an end, the button that would leave the range goes nowhere.
        assert_eq!(step("n:increment", "10"), None);
        assert_eq!(step("n:decrement", "0"), None);
        // Outside the range, the value comes back to the nearest end and never moves away.
        assert_eq!(step("n:decrement", "150"), Some("10".into()));
        assert_eq!(step("n:increment", "150"), None);
        assert_eq!(step("n:increment", "-7"), Some("0".into()));
        assert_eq!(step("n:decrement", "-7"), None);
        // Not one of this input's buttons.
        for action in ["other:increment", "n", "n:up", "nn:increment", "n:increment:x"] {
            assert_eq!(step(action, "4"), None, "{action}");
        }
        // Without bounds there is always somewhere to go.
        assert_eq!(NumberInput::adjusted_value("n:decrement", "n", "0", None, None, 1.0, 0), Some("-1".into()));
    }

    #[test]
    fn the_result_is_written_with_the_decimals_asked_for() {
        let adjust = |action: &str, current: &str, step: f64, decimals: u8| {
            NumberInput::adjusted_value(action, "n", current, None, None, step, decimals)
        };
        // Not 0.30000000000000004.
        assert_eq!(adjust("n:increment", "0.2", 0.1, 1), Some("0.3".into()));
        assert_eq!(adjust("n:increment", "1", 0.25, 2), Some("1.25".into()));
        assert_eq!(adjust("n:decrement", "1.5", 1.0, 0), Some("0".into()));
        // Nothing is not negative.
        assert_eq!(adjust("n:increment", "-0.1", 0.1, 1), Some("0.0".into()));
        assert_eq!(adjust("n:increment", "-1.4", 1.0, 0), Some("0".into()));
        // A step the decimals cannot write moves nothing, so the button goes nowhere.
        assert_eq!(adjust("n:increment", "1", 0.001, 0), None);
    }

    #[test]
    fn a_step_or_a_bound_that_is_not_a_number_is_a_step_of_one_and_no_bound() {
        for bad in [f64::NAN, f64::INFINITY, 0.0, -2.0] {
            assert_eq!(NumberInput::adjusted_value("n:increment", "n", "4", None, None, bad, 0), Some("5".into()), "{bad}");
        }
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                NumberInput::adjusted_value("n:increment", "n", "4", Some(bad), Some(bad), 1.0, 0),
                Some("5".into()),
                "{bad}"
            );
        }
    }

    #[test]
    fn an_empty_text_starts_from_zero_or_the_nearest_end_and_other_text_is_left_alone() {
        for action in ["n:increment", "n:decrement"] {
            assert_eq!(step(action, ""), Some("0".into()));
            assert_eq!(step(action, "  "), Some("0".into()));
            assert_eq!(NumberInput::adjusted_value(action, "n", "", Some(1.0), Some(99.0), 1.0, 0), Some("1".into()));
            assert_eq!(NumberInput::adjusted_value(action, "n", "", Some(-9.0), Some(-2.0), 1.0, 1), Some("-2.0".into()));
            for text in ["12a", "-", "1.2.3"] {
                assert_eq!(step(action, text), None, "{text}");
            }
        }
    }

    #[test]
    fn the_field_and_the_buttons_are_one_box_of_the_width_and_height_asked_for() {
        for (size, height) in [
            (ControlSize::Dense, 28.0),
            (ControlSize::Compact, 28.0),
            (ControlSize::Regular, 32.0),
            (ControlSize::Spacious, 36.0),
        ] {
            let input = NumberInput::new("n", "4", palette()).size(size).width(200.0).render();
            let frame = UiTree::new(input).layout(WINDOW);
            let (field, down, up) = (region(&frame, "n"), region(&frame, "n:decrement"), region(&frame, "n:increment"));
            assert_eq!((field.x, field.height), (0.0, height), "{size:?}");
            // Each button starts a hairline after what is before it, and one closes the box.
            assert_eq!(down.x, field.x + field.width);
            assert_eq!(up.x, down.x + down.width + RULE);
            assert_eq!(up.x + up.width + RULE, 200.0);
            for button in [down, up] {
                assert_eq!((button.y, button.width, button.height), (RULE, height, height - 2.0 * RULE));
            }
        }
    }

    #[test]
    fn split_buttons_surround_a_centered_value_without_changing_total_size() {
        for size in [
            ControlSize::Dense,
            ControlSize::Compact,
            ControlSize::Regular,
            ControlSize::Spacious,
        ] {
            for enabled in [true, false] {
                let input = NumberInput::new("n", "42", palette())
                    .size(size)
                    .width(200.0)
                    .enabled(enabled)
                    .button_placement(NumberInputButtonPlacement::Split)
                    .render();
                let mut tree = UiTree::new(input);
                tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 10.0);
                let frame = tree.layout(WINDOW);
                let rect = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().rect;
                let (field, down, up) = (rect("n"), rect("n:decrement"), rect("n:increment"));
                let height = size.height().max(28.0);
                assert_eq!(down.x, RULE);
                assert_eq!(down.x + down.width, field.x);
                assert_eq!(field.x + field.width, up.x);
                assert_eq!(up.x + up.width + RULE, 200.0);
                assert_eq!((field.y, field.height), (0.0, height));
                let text = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text { text, position, .. } if text == "42" => Some(*position),
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(text.x + 10.0, field.x + field.width / 2.0);
            }
        }
    }

    #[test]
    fn split_buttons_keep_left_to_right_focus_actions_and_disabled_limits() {
        for (value, expected) in [
            ("0", vec!["n", "n:increment"]),
            ("5", vec!["n:decrement", "n", "n:increment"]),
            ("10", vec!["n:decrement", "n"]),
        ] {
            let mut tree = UiTree::new(
                NumberInput::new("n", value, palette())
                    .min(0.0)
                    .max(10.0)
                    .button_placement(NumberInputButtonPlacement::Split)
                    .render(),
            );
            tree.set_reduced_motion(true);
            let frame = tree.layout(WINDOW);
            for (index, id) in expected.iter().enumerate() {
                tree.handle_event(UiEvent::Key {
                    key: Key::Tab,
                    shift: false,
                    ctrl: false,
                });
                assert_eq!(tree.focused_id(), Some(*id));
                if *id != "n" {
                    for key in [Key::Enter, Key::Space] {
                        assert_eq!(
                            tree.handle_event(UiEvent::Key {
                                key,
                                shift: false,
                                ctrl: false
                            }),
                            vec![UiAction::Clicked { id: (*id).into() }]
                        );
                    }
                    let point = center(region(&frame, id));
                    tree.handle_event(UiEvent::PointerDown(point));
                    assert_eq!(
                        tree.handle_event(UiEvent::PointerUp(point)),
                        vec![UiAction::Clicked { id: (*id).into() }]
                    );
                    assert!(tree.focus(id));
                }
                assert_eq!(tree.focused_id(), Some(expected[index]));
            }
            for (id, disabled) in [
                ("n:decrement", value == "0"),
                ("n:increment", value == "10"),
            ] {
                let node = frame.access.iter().find(|node| node.id == id).unwrap();
                assert_eq!(node.disabled, disabled);
                assert_eq!(tree.focus(id), !disabled);
                if disabled {
                    let point = center(node.rect);
                    tree.handle_event(UiEvent::PointerDown(point));
                    assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
                }
            }
        }
    }

    #[test]
    fn centered_value_uses_the_same_origin_for_caret_pointer_selection_and_edits() {
        let mut tree = UiTree::new(
            NumberInput::new("n", "42", palette())
                .width(200.0)
                .button_placement(NumberInputButtonPlacement::Split)
                .render(),
        );
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 10.0);
        tree.set_reduced_motion(true);
        tree.layout(WINDOW);
        assert!(tree.focus("n"));
        let frame = tree.layout(WINDOW);
        let field = region(&frame, "n");
        let origin = field.x + field.width / 2.0 - 10.0;
        let caret = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, .. } if rect.width == 1.5 => Some(*rect),
                _ => None,
            })
            .unwrap();
        assert_eq!(caret.x, origin + 20.0);
        tree.handle_event(UiEvent::PointerDown(Point::new(
            origin + 6.0,
            field.y + 12.0,
        )));
        assert_eq!(tree.input_cursor("n"), Some(1));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.input_selection("n"), Some((1, 2)));
        let selection = tree
            .layout(WINDOW)
            .commands
            .into_iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if color == crate::Color::rgba(75, 130, 210, 130) =>
                {
                    Some(rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!((selection.x, selection.width), (origin + 10.0, 10.0));
        assert_eq!(
            tree.handle_event(UiEvent::Text("7".into())),
            vec![UiAction::InputChanged {
                id: "n".into(),
                value: "47".into()
            }]
        );
        tree.set_input_value("n", "12345678901234567890");
        let long = tree.layout(WINDOW);
        let text_x = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { position, .. } => Some(position.x),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(
            text_x(&long),
            field.x + 8.0,
            "overflow retains ordinary horizontal editing"
        );
        tree.set_input_value("n", "4");
        let short = tree.layout(WINDOW);
        assert_eq!(text_x(&short), field.x + field.width / 2.0 - 5.0);
        tree.set_input_value("n", "");
        let empty = tree.layout(WINDOW);
        let caret_x = empty
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, .. } if rect.width == 1.5 => Some(rect.x),
                _ => None,
            })
            .unwrap();
        assert_eq!(caret_x, field.x + field.width / 2.0);
    }

    #[test]
    fn a_width_that_is_not_a_number_is_ignored_and_a_narrow_one_keeps_room_for_the_text() {
        let width = |input: NumberInput| {
            let frame = UiTree::new(input.render()).layout(WINDOW);
            let up = region(&frame, "n:increment");
            (region(&frame, "n").width, up.x + up.width + RULE)
        };
        let default = width(NumberInput::new("n", "4", palette()));
        assert_eq!(default.1, DEFAULT_WIDTH);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(width(NumberInput::new("n", "4", palette()).width(bad)), default, "{bad}");
        }
        for narrow in [-40.0, 0.0, 60.0] {
            assert_eq!(width(NumberInput::new("n", "4", palette()).width(narrow)).0, MIN_FIELD_WIDTH, "{narrow}");
        }
    }

    #[test]
    fn the_builders_ignore_a_bound_or_a_step_that_is_not_a_number() {
        let input = NumberInput::new("n", "4", palette())
            .min(0.0)
            .max(10.0)
            .step(2.0)
            .min(f64::NAN)
            .max(f64::INFINITY)
            .step(f64::NAN)
            .step(0.0)
            .step(-1.0)
            .step(f64::INFINITY);
        assert_eq!((input.min, input.max, input.step), (Some(0.0), Some(10.0), 2.0));
    }

    #[test]
    fn typing_reports_the_text_under_the_inputs_id() {
        let mut tree = UiTree::new(NumberInput::new("n", "", palette()).render());
        tree.layout(WINDOW);
        assert!(tree.focus("n"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("7".into())),
            vec![UiAction::InputChanged { id: "n".into(), value: "7".into() }]
        );
    }

    #[test]
    fn each_button_reports_its_click_by_pointer_enter_and_space() {
        let mut tree = UiTree::new(NumberInput::new("n", "4", palette()).render());
        let frame = tree.layout(WINDOW);
        for id in ["n:decrement", "n:increment"] {
            let clicked = vec![UiAction::Clicked { id: id.into() }];
            let point = center(region(&frame, id));
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked, "{id}");
            assert!(tree.focus(id));
            for key in [Key::Enter, Key::Space] {
                assert_eq!(tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false }), clicked, "{id} {key:?}");
            }
        }
    }

    #[test]
    fn a_button_with_nowhere_to_go_is_passive_and_the_other_still_works() {
        let places = |value: &str| {
            let input = NumberInput::new("n", value, palette()).min(0.0).max(10.0).render();
            let mut tree = UiTree::new(input);
            let frame = tree.layout(WINDOW);
            (tree, frame)
        };
        let (_, middle) = places("5");
        let up = region(&middle, "n:increment");
        let (mut tree, frame) = places("10");
        assert!(frame.regions.iter().all(|region| region.id != "n:increment"));
        assert!(!tree.focus("n:increment"));
        assert!(tree.handle_event(UiEvent::PointerDown(center(up))).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(center(up))).is_empty());
        // In the same place, and the field and the other button as they were.
        let node = frame.access.iter().find(|node| node.id == "n:increment").unwrap();
        assert_eq!((node.rect, node.disabled, node.actionable), (up, true, false));
        assert_eq!(region(&frame, "n:decrement"), region(&middle, "n:decrement"));
        assert_eq!(region(&frame, "n"), region(&middle, "n"));
        let (_, least) = places("0");
        assert!(least.regions.iter().all(|region| region.id != "n:decrement"));
        assert!(least.regions.iter().any(|region| region.id == "n:increment"));
        // Text that no button can step leaves both passive, and the field editable.
        let (_, typed) = places("12a");
        assert_eq!(typed.regions.iter().filter(|region| region.kind == HitKind::Button).count(), 0);
        assert!(typed.regions.iter().any(|region| region.id == "n" && region.kind == HitKind::TextInput));
    }

    #[test]
    fn a_disabled_input_takes_no_pointer_or_focus_and_keeps_its_place() {
        let input = |enabled| NumberInput::new("n", "4", palette()).enabled(enabled).render();
        let enabled = UiTree::new(input(true)).layout(WINDOW);
        let mut tree = UiTree::new(input(false));
        let frame = tree.layout(WINDOW);
        assert!(frame.regions.iter().all(|region| !matches!(region.kind, HitKind::Button | HitKind::TextInput)));
        for id in ["n", "n:decrement", "n:increment"] {
            assert!(!tree.focus(id), "{id}");
            let rect = region(&enabled, id);
            assert_eq!(frame.access.iter().find(|node| node.id == id).unwrap().rect, rect, "{id}");
            assert!(tree.handle_event(UiEvent::PointerDown(center(rect))).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(center(rect))).is_empty());
        }
        assert!(tree.handle_event(UiEvent::Key { key: Key::Enter, shift: false, ctrl: false }).is_empty());
        assert!(tree.handle_event(UiEvent::Text("7".into())).is_empty());
        // The number and the two glyphs, all in muted ink.
        let inks: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { color, .. } => Some(*color),
                DrawCommand::Icon { tint, .. } => Some(*tint),
                _ => None,
            })
            .collect();
        assert_eq!(inks, vec![palette().muted; 3]);
        // Both spellings disable it.
        assert!(!NumberInput::new("n", "4", palette()).disabled().enabled);
    }

    #[test]
    fn the_platform_is_told_a_named_field_and_two_buttons_named_by_where_they_go() {
        for enabled in [true, false] {
            let input = NumberInput::new("n", "4", palette())
                .min(0.0)
                .max(5.0)
                .accessibility_label("Quantity")
                .enabled(enabled)
                .render();
            let nodes = UiTree::new(input).layout(WINDOW).access;
            let node = |id: &str| nodes.iter().find(|node| node.id == id).unwrap();
            let field = node("n");
            assert_eq!((field.role, field.name.as_str(), field.value_text.as_deref()), (Role::TextInput, "Quantity", Some("4")));
            assert_eq!((field.disabled, field.actionable), (!enabled, enabled));
            for (id, name) in [("n:decrement", "Decrease 3"), ("n:increment", "Increase 5")] {
                let button = node(id);
                assert_eq!((button.role, button.name.as_str()), (Role::Button, name));
                assert_eq!((button.disabled, button.actionable), (!enabled, enabled), "{id}");
            }
        }
        // With nowhere to go a button is named by what it does, in the host's words if it gave any.
        let input = NumberInput::new("n", "5", palette()).max(5.0).placeholder("0").step_labels("Menos", "Más").render();
        let nodes = UiTree::new(input).layout(WINDOW).access;
        let node = |id: &str| nodes.iter().find(|node| node.id == id).unwrap();
        assert_eq!((node("n:increment").name.as_str(), node("n:increment").disabled), ("Más", true));
        assert_eq!(node("n:decrement").name, "Menos 4");
        // Without a name of its own the field is called by its placeholder.
        assert_eq!(node("n").name, "0");
    }
}
