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
| `theme.scheme()` | The same colors as a [`Scheme`](#a-scheme-of-explicit-roles). |

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

### A scheme of explicit roles

A `Theme` derives every color from three. A host with a finished design has
the opposite need: it knows each value and wants them named once. A `Scheme`
is that list of roles, built as a `const`:

| Field | Role |
| --- | --- |
| `canvas`, `panel` | The page, and the docks and side panels level with it. |
| `raised` | Cards, sidebars and inputs. |
| `hover`, `selected` | Pointer feedback on a flat surface; a pressed control or chosen item. |
| `border`, `shadow` | Hairlines; the shadow under floating surfaces. |
| `text`, `muted`, `muted_on_raised` | Primary ink, and secondary ink on a flat and on a raised surface. |
| `menu`, `menu_hover`, `menu_muted` | Floating menus and popovers. |
| `control` | A `ControlColors`: the fill, hover, pressed, border and text of a filled control on a raised surface. |

No component takes a `Scheme`. It only produces the palettes they take:

| Method | Palette for |
| --- | --- |
| `scheme.palette()` | Components on the canvas. |
| `scheme.card()` | Content laid on a raised surface. |
| `scheme.menu()` | A floating menu. Its chosen row holds the hover fill. |
| `scheme.on_raised()` | A filled control on a raised surface. |
| `scheme.on(surface, ink)` | An action painted on a color of its own. |

`on` takes the role of that color, because a scheme never guesses a role from
a value:

| `Surface` | Hover | Pressed |
| --- | --- | --- |
| `Flat(color)` | `scheme.hover` | `scheme.selected` |
| `Raised(color)` | `scheme.step(color)` | `scheme.selected` |
| `Solid(color)` | `color` | `color` |

```rust
let chosen_row = scheme.on(Surface::Raised(scheme.selected), scheme.text);
```

`scheme.step(color)` moves each channel 8 toward the ink: lighter in a dark
scheme, darker in a light one. Alpha is kept.

`theme.scheme()` gives a theme's colors by role, and
`theme.scheme().palette()` equals `theme.palette()`. A scheme holds no status,
link, diff or syntax colors: those stay with the host.

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

The installer also keeps egui's fallback faces and appends Noto Sans Math
3.000 for mathematical alphabets, including names such as `𝖗𝖚𝖓𝖊` and
`𝓡𝓤𝓝𝓔`. The fallback is shared by regular, italic, medium, bold and
monospace text. Existing glyphs still come from their original faces;
missing mathematical glyphs use Noto's own shapes and widths, including
inside monospace text. `attach_fonts` measures the same fallback that paints.

The bundled font adds about 1 MiB and uses SIL OFL-1.1; its unchanged license,
version, pinned source and hashes are in
[`crates/limn-egui/assets/fonts/NOTICE.md`](../crates/limn-egui/assets/fonts/NOTICE.md).
This is additional mathematical coverage, not general Unicode shaping,
CJK coverage or color emoji. No fonts are read from the host's system or
downloaded at runtime.

Noto Sans Symbols 2 v2.008 follows Math in every family for missing
decorative symbols, including `✦`, `✧`, `❖` and `➜`. The unmodified,
unhinted OpenType face adds 404,576 bytes under OFL-1.1, with its release
source, hashes and full license alongside the Math notices. The Text
gallery uses synthetic labels to demonstrate the fallback. Neither this
font nor those tests imply that every user-supplied name is covered.
The proportional families also reuse egui's already embedded monospace
fallbacks at the end of their chain, supplying box drawing such as
`├─│╭╰║` without another font asset. Missing glyphs keep their fallback
face's advance width.

Noto Emoji 3.002 is appended last in all five families for missing
monochrome emoji, including `🙂`, `🤣`, `🦀` and `🫡`. It adds 1,982,596
bytes under OFL-1.1. The original variable font, full license and pinned
source hashes are bundled alongside the other font notices. Previously
covered text retains its face and metrics; new emoji use the fallback's
own advance width, including in monospace text. There is no runtime font
download or new runtime dependency.

### A font of the person's choice

`limn_egui::font_definitions_with(UiFaces { regular, medium, bold })` puts a
host's faces, such as a family the person picked from the system's fonts,
first in the interface's families: `regular` for regular and italic text,
`medium` and `bold` for their weights, which fall back to `regular` when
left out. Monospace text keeps its face, and every bundled face and fallback
stays behind the chosen ones, so what the family lacks still draws. limn
reads no file itself: the host loads the bytes, off its interface's
thread, and installs the result with `Context::set_fonts`, then calls
`attach_fonts` again so the tree measures with it.

```rust
let faces = limn_egui::UiFaces {
    regular: egui::FontData::from_owned(regular_bytes),
    medium: None,
    bold: Some(egui::FontData::from_owned(bold_bytes)),
};
ctx.set_fonts(limn_egui::font_definitions_with(faces));
limn_egui::attach_fonts(&mut tree, &ctx);
```

The 623-entry synthetic fixture in `limn-egui/src/emoji_samples.txt` covers
all visible scalars from RUNE's Unicode picker at the time of this change.
Tests inspect actual installed charmaps in all five families and exercise
egui's lookup, layout metrics and composition of `❤️`, `☀️`, `🧑‍💻` and
`🏴‍☠️`. The fixture contains only Unicode strings, without application
data or assets. It is a coverage snapshot, not a complete emoji standard.

Scalar coverage is distinct from sequence shaping. egui 0.36 chooses the
face for a grapheme from its first character. When that character already
exists in an earlier font, the appended font cannot supply a missing
combined glyph. For example, the regional flag `🇦🇷` still paints two
glyphs, and some keycaps and joined flags remain incomplete. The fallback
does not provide color emoji, complete combined-sequence support or CJK
coverage. The Text gallery shows both supported samples and those limits.

## Language

The words the library says by itself, such as the hint of a close button, the
default actions of an alert dialog or the names in the text context menu, are
in one value. They are English unless the host sets them, once:

```rust
limn::set_labels(limn::Labels::SPANISH);

// Or a few of them, in any language:
limn::set_labels(limn::Labels {
    close: "Fermer",
    cancel: "Annuler",
    ..limn::Labels::ENGLISH
});
```

`Labels::ENGLISH` and `Labels::SPANISH` are complete. A component that takes
a label of its own, such as `AlertDialog::confirm_label`, uses these only
when it is given none. `FindBar`, `Markdown` and `DiagramViewer` take their
own labels, in `FindLabels`, `MarkdownLabels` and `close_title`.

## What cannot be themed yet

- The easing curve of transitions.
- Font sizes, which each component sets for itself, and font weight.
- Spacing inside components.
- The icon size, 16 px.

## See also

- [Tokens](tokens.md) for the reference tables.
- [Animation](animation.md) for what the durations apply to.
