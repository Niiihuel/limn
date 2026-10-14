# SwatchPicker

One color among a few, all of them in view: a round swatch for each, with a
ring around the chosen one. Use it for an accent, a label or a role color
taken from a set; add a custom swatch when a color of the user's own is
allowed too, and open a [ColorPicker](color-picker.md) from it.

## Import

```rust
use limn::{Color, Component, Swatch, SwatchPicker};
```

## Usage

A swatch picker is controlled: pass the chosen swatch's value each frame, and
keep the value a swatch reports when it is clicked. Each swatch's value is the
ID of its `UiAction::Clicked`, as a [RadioOption](radio-group.md)'s is, so it
must be unique in the window: prefix it with the picker's name. A swatch shows
no text, so it takes a name for the platform.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    SwatchPicker::new(
        "accent",
        vec![
            Swatch::new("accent:teal", Color::rgb(26, 188, 156), "Teal"),
            Swatch::new("accent:blue", Color::rgb(52, 152, 219), "Blue"),
            Swatch::new("accent:red", Color::rgb(231, 76, 60), "Red"),
        ],
        Some(&state.accent),
        palette,
    )
    .accessibility_label("Accent color")
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id.starts_with("accent:") {
            state.accent = id.clone();
        }
    }
}
```

`selected` is `None`, or a value no swatch has, while nothing is chosen yet:
no swatch is ringed.

A swatch fills its circle with its color. The chosen one draws a ring in the
text's ink at its edge, with the color inside it, 8 pixels narrower, so the
picker does not move when the choice changes. A swatch that is not chosen has
a hairline in the border color, so one of the page's own color still shows.

Each swatch stands in a cell 4 pixels wider on every side, which is its hit
region. The keyboard focus ring is drawn at the cell's edge, 2 pixels outside
the chosen swatch's ring, so the two do not lie on each other while the arrows
move focus and choice together.

## Custom color

```rust
SwatchPicker::new("accent", swatches, Some(&state.accent), palette)
    .custom(state.custom_color, "Custom color")
```

A swatch before the others shows `color` under a pencil, in whichever of the
palette's text and surface inks stands out from it. It reports
`UiAction::Clicked` with `{id}:custom`: open a picker for the user's own
color there, and pass `{id}:custom` as `selected` while that color is the
choice, to ring it.

```rust
let picker = SwatchPicker::new("accent", swatches, Some(&state.accent), palette)
    .custom(Color::from(state.custom), "Custom color")
    .render();
Popover::new(
    picker,
    ColorPicker::new("accent-picker", state.custom, palette).render().padding(12.0),
    state.picking,
    "accent-picker:dismiss",
    palette,
)
.width(244.0)
.render()
```

## Rows

```rust
SwatchPicker::new("accent", swatches, Some(&state.accent), palette).columns(5)
```

The swatches stand in one row unless `columns` breaks them into rows of that
many, the custom swatch counted first. Swatches and rows are 8 pixels apart:
the cells touch. The picker is 4 pixels wider than its swatches on every
side, for the focus ring.

## Size

```rust
SwatchPicker::new("accent", swatches, Some(&state.accent), palette).size(ControlSize::Compact)
```

| Size | Diameter | Cell |
| --- | --- | --- |
| `Dense` | 20 | 28 |
| `Compact` | 24 | 32 |
| `Regular`, the default | 28 | 36 |
| `Spacious` | 32 | 40 |

## Disabled

```rust
SwatchPicker::new("accent", swatches, Some(&state.accent), palette).disabled()
SwatchPicker::new("accent", swatches, Some(&state.accent), palette).enabled(state.can_edit)
```

A disabled picker shows every color and its choice, ringed in the muted ink,
in the same place, and takes no pointer or focus.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Move the focus to the next or the previous swatch; every swatch, the custom one included, is a stop. |
| Enter, Space | Choose the focused swatch, or open the picker from the custom one. |
| Up, Left, Down, Right | Move the focus to the previous or the next swatch and choose it, going round at the ends, across rows. The custom swatch is passed by: it opens something rather than choosing. |
| Home, End | Move to the first or the last swatch and choose it. |

## Motion

The cell under the pointer lights in the palette's hover color, a halo
around the swatch, over the hover duration of the motion tokens. The ring and
the color change at once with the choice.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }`, where `id` is the swatch's value | A swatch is chosen by pointer, Enter or Space, or reached with an arrow, Home or End. Choosing the swatch that is already chosen reports it again. |
| `UiAction::Clicked { id }` with `{id}:custom` | The custom swatch is clicked, or Enter or Space is pressed on it. |

## Accessibility

Each swatch is told to the platform as a radio button named by its name, with
whether it is the chosen one and whether it is disabled. The custom swatch is
a button named by its name, selected while it is the choice.

Name the picker, and it becomes a group around its swatches, known by its ID:

```rust
SwatchPicker::new("accent", swatches, Some(&state.accent), palette).accessibility_label("Accent color")
```

## Limits

- Swatches are circles; there are no square ones.
- Rows do not wrap by themselves in a narrow width: set `columns` for the room there is.
- A single swatch cannot be disabled, only the whole picker.
- The arrows step through the swatches in order; Up and Down do not move by a row.
