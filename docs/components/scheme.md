# Scheme

One color for each role of an interface, for a host that chooses every value
itself. No component takes a `Scheme`: it only produces the
`ComponentPalette`s components take.

## Import

```rust
use limn::{Color, ControlColors, Scheme, Surface};
```

## Usage

Build the scheme as a `const` and derive every palette from it, so two
palettes cannot disagree about a shared color.

```rust
const SCHEME: Scheme = Scheme {
    canvas: Color::rgb(24, 24, 24),
    panel: Color::rgb(24, 24, 24),
    raised: Color::rgb(32, 32, 32),
    hover: Color::rgb(40, 40, 40),
    selected: Color::rgb(48, 48, 48),
    border: Color::rgb(63, 63, 63),
    text: Color::rgb(225, 225, 225),
    muted: Color::rgb(153, 153, 153),
    muted_on_raised: Color::rgb(165, 165, 165),
    menu: Color::rgb(36, 36, 36),
    menu_hover: Color::rgb(52, 52, 52),
    menu_muted: Color::rgb(153, 153, 153),
    shadow: Color::rgba(0, 0, 0, 100),
    control: ControlColors {
        surface: Color::rgb(44, 44, 44),
        hover: Color::rgb(52, 52, 52),
        pressed: Color::rgb(60, 60, 60),
        border: Color::rgb(63, 63, 63),
        text: Color::rgb(225, 225, 225),
    },
};

fn view() -> Element {
    Button::new("save", "Save", SCHEME.palette()).render()
}
```

`theme.scheme()` gives the colors a `Theme` derives as a scheme, and its
`palette()` equals `theme.palette()`.

## Palettes

```rust
let page = scheme.palette();
let card = scheme.card();
let menu = scheme.menu();
let control = scheme.on_raised();
```

`palette` is for components on the canvas, `card` for content laid on a
raised surface, `menu` for a floating menu and `on_raised` for a filled
control on a raised surface.

## An action on a color of its own

```rust
let chosen_row = scheme.on(Surface::Raised(scheme.selected), scheme.text);
```

`on(surface, ink)` is the palette of an action painted on a color the caller
names by its role. `Surface::Flat` hovers with the scheme's `hover`,
`Surface::Raised` moves one `step` from its own color, and `Surface::Solid`
holds its color under the pointer.

`scheme.step(color)` moves a color toward the ink: lighter in a dark scheme,
darker in a light one.

## See also

[Theming](../theming.md) lists every role and what each palette is made of.
