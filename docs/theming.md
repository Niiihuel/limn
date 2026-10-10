# Theming

How to change the way the library looks. There are two independent controls:

| Control | Covers | Scope |
| --- | --- | --- |
| `Theme` and `ComponentPalette` | Color. | Passed to each component, so two palettes can share a window. |
| `Tokens` | Corner radii and animation durations. | One setting for the whole process. |

## Color

A `Theme` is three colors. Every other color is derived from them, so no token
can drift out of step with the rest.

```rust
use limn::{Color, Theme};

let theme = Theme {
    background: Color::rgb(255, 255, 255),
    foreground: Color::rgb(24, 26, 31),
    accent: Color::rgb(9, 105, 218),
};
let palette = theme.palette();
```

`Theme::DARK` is the built-in dark theme.

Pass the palette to components and use the theme's own colors for the page:

```rust
Element::panel(Button::new("save", "Save", palette).render())
    .background(theme.background)
```

### What a theme derives

| Method | Result |
| --- | --- |
| `theme.surface(level)` | An opaque surface, from 1 (barely above the background) to 7. |
| `theme.outline()` | Hairlines between and around surfaces. |
| `theme.text()`, `text_sub()`, `text_hint()`, `text_disabled()` | Text at falling strength. |
| `theme.hover()`, `pressed()` | Translucent veils for pointer feedback. |
| `theme.accent_surface()`, `accent_text()` | The accent diluted into a fill, and as readable ink. |
| `theme.palette()` | The `ComponentPalette` components take. |

### The palette

`ComponentPalette` is the subset components need:

| Field | Used for |
| --- | --- |
| `surface`, `raised` | Component backgrounds, flat and lifted. |
| `hover`, `selected` | Pointer feedback and chosen items. |
| `border` | Outlines and dividers. |
| `text`, `muted` | Primary and secondary ink. |
| `shadow` | Shadows under floating surfaces. |

`palette.link()` is the ink for links: a blue chosen to read on that palette,
dark or light.

Hover and selection are veils, not opaque fills, so the same feedback reads on
any surface. To give one component different colors, pass it a modified copy:

```rust
let danger = ComponentPalette { text: red, ..palette };
```

### Light and dark together

Because color is a value passed to each component, nothing stops two palettes
from sharing a window: a dark sidebar beside a light document, for instance.

## Shape and pace

Corner radii and durations are set once:

```rust
use limn::{set_tokens, Motion, Radius, Tokens};

set_tokens(Tokens {
    radius: Radius::SQUARE,
    motion: Motion::DEFAULT.scaled(0.5),
});
```

They are global because the library paints tooltips, menus, scrollbars and
focus rings itself, without being handed a palette. [Tokens](tokens.md) lists
every value.

## Fonts

`limn_egui::install_fonts(&ctx)` installs Ubuntu Regular as the UI font.
Call it once when the window is created. Without it egui's default
proportional face, Ubuntu Light, is used, whose thin strokes look washed out
at small sizes.

## What cannot be themed yet

- The easing curve of transitions.
- Font sizes, which each component sets for itself, and font weight.
- Spacing inside components.
- The icon size, 16 px.

## See also

- [Tokens](tokens.md) for the reference tables.
- [Animation](animation.md) for what the durations apply to.
