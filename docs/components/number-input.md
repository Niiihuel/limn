# NumberInput

A field for a number, joined to a button that lowers it and one that raises
it. Use it for a quantity, a size or any value that is typed as often as it
is nudged.

## Import

```rust
use limn::{Component, NumberInput};
```

## Usage

A number input is controlled: pass the text of the field each frame, and
store what the field and the buttons report. The field reports every edit
under the input's own ID. A button reports a click, and
`NumberInput::adjusted_value` turns that click into the next text.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    NumberInput::new("quantity", &state.quantity, palette)
        .min(1.0)
        .max(99.0)
        .accessibility_label("Quantity")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    match action {
        UiAction::InputChanged { id, value } if id == "quantity" => state.quantity = value.clone(),
        UiAction::Clicked { id } => {
            let next = NumberInput::adjusted_value(id, "quantity", &state.quantity, Some(1.0), Some(99.0), 1.0, 0);
            if let Some(next) = next {
                state.quantity = next;
            }
        }
        _ => {}
    }
}
```

`adjusted_value(action_id, id, current, min, max, step, decimals)` is `None`
for an action that is not one of this input's buttons, so it can be tried on
every click. Give it the same range, step and decimals as the input.

A field keeps the text typed in it, whatever value it is built with again.
So after a button changes the value, give the field the new text:

```rust
tree.set_input_value("quantity", &state.quantity);
```

The state is text, not a number, so that what is half typed stays as typed.
`NumberInput::parse(&state.quantity)` is the number when there is one.

## Range and step

```rust
NumberInput::new("opacity", &state.opacity, palette).min(0.0).max(1.0).step(0.05).decimals(2).render()
```

| Builder | Sets | Unless set |
| --- | --- | --- |
| `min(f64)` | The least value the buttons go to. | No bound |
| `max(f64)` | The greatest value the buttons go to. | No bound |
| `step(f64)` | How far one click moves the value. | 1 |
| `decimals(u8)` | The digits after the point a button writes its result with. | 0 |

A bound that is not finite, and a step that is not finite or not above zero,
are ignored. `adjusted_value` treats them the same way: no bound, and a step
of 1.

A button moves the value by one step and stops at the end of the range: a
step up from `9.5` with a `max` of 10 is `10`. The result is rounded to
`decimals`, so a step of `0.1` up from `0.2` is `0.3` with `decimals(1)`.
With the default of no decimals a step below 1 moves nothing: set `decimals`
whenever the step has a fraction.

The field takes any text, so a value outside the range can be typed. A
button brings it back to the nearest end and never moves it further out:
with a `max` of 10, the button that lowers takes `150` to `10`, and the one
that raises does nothing.

## At the ends

A button with nowhere to go is drawn disabled and is passive: the one that
lowers at `min` or below it, the one that raises at `max` or above it. For
such a click `adjusted_value` is `None`.

## Text that is not a number

```rust
NumberInput::parse("-1.5")         // Some(-1.5)
NumberInput::parse("12a")          // None
NumberInput::is_valid_number(".5") // true
```

A number is an optional sign, digits and at most one point, with spaces
around it ignored: `-1.5`, `+3`, `.5`, `2.`. Anything else is not: an empty
text, `-`, `1e3`, `1,5`.

The text is the host's, a number or not. From an empty field either button
writes the starting value: zero, or the end of the range nearest to zero
when zero is outside it. Any other text that is not a number is left as it
is: `adjusted_value` is `None` and both buttons are disabled until the text
is a number again.

## Placeholder

```rust
NumberInput::new("guests", &state.guests, palette).placeholder("Guests").render()
```

The placeholder shows while the field is empty.

## Size and width

```rust
NumberInput::new("quantity", &state.quantity, palette).size(ControlSize::Spacious).width(240.0).render()
```

`size` takes the height of a control size, as a text field does: 28 for
`Dense` and `Compact`, 32 for `Regular`, the default, and 36 for `Spacious`.
`width` is the field and the buttons together, 160 unless set. The text
always keeps 80, so a narrower width is widened to fit. A width that is not
finite is ignored.

## Disabled

```rust
NumberInput::new("quantity", &state.quantity, palette).disabled().render()
NumberInput::new("quantity", &state.quantity, palette).enabled(state.can_edit).render()
```

A disabled input shows its text and its buttons in muted ink, in the same
place. The field and the buttons take no pointer or focus and report
nothing.

## Keyboard

The field and each button that has somewhere to go are tab stops. Enter or
Space on a button reports its click. The arrow keys in the field move the
caret, as in any field: they do not step the value.

## Motion

A button's hover and press fills change over `tokens().motion.hover_ms`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::InputChanged { id, value }` | The text of the field changes; `id` is the input's. |
| `UiAction::Clicked { id }` with `{id}:decrement` or `{id}:increment` | A button is pressed. |

## Accessibility

The field is a text input whose value is its text. Nothing the input draws
names it, so give it a name; without one it is called by its placeholder.

```rust
NumberInput::new("quantity", &state.quantity, palette).accessibility_label("Quantity").render()
```

Each button is named by what it does and the value it goes to, such as
"Increase 5", or by what it does alone when it has nowhere to go. A button
drawn disabled is still there, as disabled. The two words are the English
"Decrease" and "Increase": `limn::labels()` has no words for them, so an
application in another language passes its own.

```rust
NumberInput::new("quantity", &state.quantity, palette).step_labels("Menos", "Más").render()
```

## Limits

- The field does not refuse a character: letters can be typed, and the host
  decides what to do with text that `parse` does not read.
- A typed value is not brought into the range by itself. A host that wants
  that does it where it reads the value.
- Numbers are written with a point and without separators of thousands.
