use super::{caption, row, section, Cx, Section};
use crate::app::State;
use limn::{Component, ComponentPalette, ControlSize, NumberInput, UiAction};

/// A live input: what its buttons need to step it, and what it shows before any change.
struct Field {
    id: &'static str,
    initial: &'static str,
    name: &'static str,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    decimals: u8,
}

const fn field(id: &'static str, initial: &'static str, name: &'static str) -> Field {
    Field { id, initial, name, min: None, max: None, step: 1.0, decimals: 0 }
}

const QUANTITY: Field = Field { min: Some(1.0), max: Some(99.0), ..field("number-quantity", "1", "Quantity") };
const OPACITY: Field = Field { min: Some(0.0), max: Some(1.0), step: 0.05, decimals: 2, ..field("number-opacity", "0.80", "Opacity") };
const OFFSET: Field = Field { step: 8.0, ..field("number-offset", "-16", "Offset") };
const VOLUME: Field = Field { min: Some(0.0), max: Some(10.0), ..field("number-volume", "10", "Volume") };
const GUESTS: Field = Field { min: Some(2.0), max: Some(12.0), ..field("number-guests", "", "Guests") };
const TYPED: Field = field("number-typed", "12a", "Count");
const DENSE: Field = field("number-dense", "4", "Dense");
const REGULAR: Field = field("number-regular", "4", "Regular");
const SPACIOUS: Field = field("number-spacious", "4", "Spacious");
const FIELDS: [&Field; 9] = [&QUANTITY, &OPACITY, &OFFSET, &VOLUME, &GUESTS, &TYPED, &DENSE, &REGULAR, &SPACIOUS];

impl Field {
    fn text<'a>(&'a self, state: &'a State) -> &'a str {
        state.choice(self.id, self.initial)
    }

    fn input(&self, state: &State, palette: ComponentPalette) -> NumberInput {
        let mut input = NumberInput::new(self.id, self.text(state), palette)
            .step(self.step)
            .decimals(self.decimals)
            .accessibility_label(self.name);
        if let Some(min) = self.min {
            input = input.min(min);
        }
        if let Some(max) = self.max {
            input = input.max(max);
        }
        input
    }
}

/// A button's click becomes the next text of its input. The fields report
/// their own edits, which the gallery keeps under their IDs.
pub fn update(state: &mut State, action: &UiAction) {
    let UiAction::Clicked { id } = action else { return };
    for field in FIELDS {
        let next = NumberInput::adjusted_value(id, field.id, field.text(state), field.min, field.max, field.step, field.decimals);
        if let Some(next) = next {
            state.set(field.id, &next);
        }
    }
}

/// Each live field and the text it stands for. A field keeps the text typed
/// in it, so after a button steps the value the tree is given these.
pub fn fields(state: &State) -> Vec<(&'static str, String)> {
    FIELDS.iter().map(|field| (field.id, field.text(state).to_owned())).collect()
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let state = cx.state;
    let input = |field: &Field| field.input(state, palette);
    let read = |field: &Field| match NumberInput::parse(field.text(state)) {
        Some(value) => format!("{:?} is {value}", field.text(state)),
        None => format!("{:?} is not a number yet", field.text(state)),
    };

    vec![
        section(cx, "Number input", vec![
            row(vec![input(&QUANTITY).render()]),
            caption(cx, &format!("From 1 to 99 by 1: {}. The field takes any text; the buttons step it.", read(&QUANTITY))),
        ]),
        section(cx, "Step and decimals", vec![
            row(vec![input(&OPACITY).render()]),
            caption(cx, &format!("From 0 to 1 by 0.05, written with two decimals: {}.", read(&OPACITY))),
            row(vec![input(&OFFSET).render()]),
            caption(cx, &format!("By 8 with no bounds, so it goes below zero: {}.", read(&OFFSET))),
        ]),
        section(cx, "At the ends of the range", vec![
            row(vec![input(&VOLUME).render()]),
            caption(cx, &format!("From 0 to 10: {}. At an end, the button that would leave the range is disabled.", read(&VOLUME))),
        ]),
        section(cx, "Text that is not a number", vec![
            row(vec![input(&GUESTS).placeholder("Guests").render()]),
            caption(cx, &format!("From 2 to 12: {}. From an empty field either button starts at the end nearest to zero.", read(&GUESTS))),
            row(vec![input(&TYPED).render()]),
            caption(cx, &format!("{}. The buttons leave other text as it is, and are disabled until it is a number.", read(&TYPED))),
        ]),
        section(cx, "Sizes and width", vec![
            row(vec![input(&DENSE).size(ControlSize::Dense).width(120.0).render(), input(&REGULAR).render()]),
            caption(cx, "Dense, 28 high like compact, at a width of 120; regular, 32 high, at the 160 it has unless set."),
            row(vec![input(&SPACIOUS).size(ControlSize::Spacious).width(260.0).render()]),
            caption(cx, "Spacious, 36 high, at 260. The width is the field and the buttons together."),
        ]),
        section(cx, "Disabled", vec![
            row(vec![NumberInput::new("number-off", "42", palette).accessibility_label("Locked count").disabled().render()]),
            caption(cx, "Disabled: the number and the buttons in muted ink, without pointer input or focus."),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_click_steps_its_own_input_and_the_fields_follow() {
        let mut state = State::default();
        let click = |id: &str| UiAction::Clicked { id: id.into() };
        update(&mut state, &click("number-quantity:increment"));
        update(&mut state, &click("number-opacity:decrement"));
        update(&mut state, &click("number-guests:decrement"));
        // Nowhere to go, and text no button steps.
        update(&mut state, &click("number-volume:increment"));
        update(&mut state, &click("number-typed:increment"));
        let shown = fields(&state);
        let text = |id: &str| shown.iter().find(|(field, _)| *field == id).unwrap().1.as_str();
        assert_eq!(text("number-quantity"), "2");
        assert_eq!(text("number-opacity"), "0.75");
        assert_eq!(text("number-guests"), "2");
        assert_eq!(text("number-volume"), "10");
        assert_eq!(text("number-typed"), "12a");
        assert_eq!(text("number-offset"), "-16");
    }
}
