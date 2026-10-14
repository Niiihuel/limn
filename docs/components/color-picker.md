# ColorPicker

A color chosen by hand: a field whose thumb sets the saturation, across it,
and the value, up it; a bar whose thumb sets the hue; and a field for the
color's hex digits. Use it where any color is allowed, often in a popover
opened from a [SwatchPicker](swatch-picker.md)'s custom swatch.

## Import

```rust
use limn::{ColorPicker, Component, Hsv};
```

## Usage

A color picker is controlled: pass the color each frame as an `Hsv`, and turn
each action it reports into the next color with `ColorPicker::color_after`.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    ColorPicker::new("role-color", state.color, palette).render()
}

fn update(state: &mut State, tree: &mut UiTree, action: &UiAction) {
    if let Some(next) = ColorPicker::color_after("role-color", state.color, action) {
        state.color = next;
        // A thumb moved: the hex field shows the color it leads to.
        if !matches!(action, UiAction::InputChanged { .. }) {
            tree.set_input_value("role-color:hex", ColorPicker::hex(next.into()));
        }
    }
}
```

`color_after(id, current, action)` is `None` for an action that is not one of
this picker's parts, so it can be tried on every action, and for hex text
that is not a color yet.

A field keeps the text typed in it, whatever value it is built with again. So
after a thumb moves, give the hex field the new text, as above; while the
user types, leave it alone.

Keep the color as an `Hsv`, not a `Color`: a gray has no hue of its own and
black no saturation, and the thumbs would jump if they were worked out from
the color each time. `Color::from(hsv)` and `Hsv::from(color)` convert.

## Hue, saturation and value

```rust
Hsv::new(210.0, 0.75, 0.85)
```

| Field | Range | Meaning |
| --- | --- | --- |
| `hue` | 0 to 360 | Degrees round the color wheel: 0 red, 120 green, 240 blue, 360 red again. |
| `saturation` | 0 to 1 | From a gray to the full hue: across the field. |
| `value` | 0 to 1 | From black to the brightest: up the field. |

`Hsv::new` clamps each coordinate to its range and takes one that is not a
number as 0; the picker does the same with an `Hsv` built by hand. A `Color`
made from an `Hsv` is opaque, and an `Hsv` made from a `Color` leaves its
alpha out.

## Hex

```rust
ColorPicker::hex(Color::rgb(26, 188, 156))   // "#1abc9c"
ColorPicker::parse_hex("1ABC9C")             // Some(Color::rgb(26, 188, 156))
ColorPicker::parse_hex("#fa0")               // Some(Color::rgb(255, 170, 0))
ColorPicker::parse_hex("#12")                // None
```

The field shows the color as `#rrggbb`, in lowercase. It reads six digits or
three, with or without a `#` and with spaces around them. A typed gray keeps
the hue the picker had, and a typed black its hue and saturation, so the
thumbs stay where they were; typing the color it already shows changes
nothing.

## Width

```rust
ColorPicker::new("role-color", state.color, palette).width(300.0)
```

The whole picker is 220 pixels wide unless set, and never under 120. The
field is three quarters as high as it is wide. The field, the bar and the hex
field are inset by half a thumb, 8 pixels, so a thumb at an edge overhangs
it and its center stands on the color it picks.

## Disabled

```rust
ColorPicker::new("role-color", state.color, palette).disabled()
ColorPicker::new("role-color", state.color, palette).enabled(state.can_edit)
```

A disabled picker shows its color in the same place, the thumbs ringed in the
muted ink, and nothing in it takes the pointer or focus.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Move the focus between the field, the bar and the hex field. |
| Left, Right | In the field, lower or raise the saturation by a twentieth; on the bar, move the hue by a twentieth of the wheel, 18 degrees. |
| Up, Down | In the field, raise or lower the value by a twentieth. |
| Home, End | In the field, take the saturation to 0 or 1; on the bar, take the hue to 0 or 360. |

In the hex field the keys edit the text, as in any field.

## Motion

None: the thumbs follow the pointer and the color.

## Actions

| Action | When |
| --- | --- |
| `UiAction::AreaSliderChanged { id, x, y, committed }` with `{id}:area` | The field is pressed or dragged, or stepped from the keyboard: `x` is the saturation and `y` one less the value. `committed` is false during a drag. |
| `UiAction::SliderChanged { id, fraction, committed }` with `{id}:hue` | The bar is pressed or dragged, or stepped from the keyboard: `fraction` is the hue over 360. |
| `UiAction::InputChanged { id, value }` with `{id}:hex` | The text of the hex field changes. |

`color_after` reads all three; there is no need to match them by hand.

## Accessibility

The field and the bar are sliders: the field named "Saturation and
brightness", its value told as "50%, 80%", and the bar named "Hue", its value
told in degrees. The hex field is a text input named "Hex" whose value is its
text. These names come from the library's labels, `saturation_brightness`,
`hue` and `hex`, so `limn::set_labels(Labels::SPANISH)` reaches them; a
picker that needs other words is given its own:

```rust
ColorPicker::new("role-color", state.color, palette).part_labels("Couleur", "Teinte", "Hex")
```

## Limits

- No alpha: the picker chooses opaque colors, and the hex field reads no
  fourth pair of digits.
- The colors are worked out in sRGB as most pickers do, not in a perceptual space.
- The arrows move by a twentieth; the hex field is the way to an exact color.
- The host gives the hex field its text after a thumb moves, with
  `UiTree::set_input_value`.
- The picker keeps its width in a narrower container and goes past its edge;
  set `width` for the room there is, down to 120.
