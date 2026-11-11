# Tooltip

A hint for a control: its name, its shortcut and, when needed, a sentence on
what it does. It appears after the pointer rests on the control for 350 ms, or
at once with keyboard focus.

## Import

```rust
use limn::{Component, Tooltip, TooltipPlacement};
```

## Usage

A tooltip wraps its anchor where the anchor is built. It adds no layout size,
takes no keyboard input and reports no action, so there is nothing to handle.

```rust
fn view(palette: ComponentPalette) -> Element {
    Tooltip::new(
        Button::new("plan", "Plan", palette).render(),
        "plan",
        "Plan mode",
        palette,
    )
    .render()
}
```

The arguments are the anchor, the anchor's ID, the title and the palette. An
anchor that is not a control gets a hover region, with no click action and no
keyboard stop.

## On a button

```rust
Button::new("save", "Save", palette)
    .tooltip("Save changes")
    .tooltip_shortcut("Ctrl+S")
    .render()
```

[Button](button.md) and `IconButton` take their hint directly, so they need
no wrapper. `tooltip_with(TooltipSpec)` takes a description, a placement and
a width as well, and `content` for a body of your own.

## Shortcut, description and footer

```rust
Tooltip::new(anchor, "plan", "Plan mode", palette)
    .shortcut("Shift+Tab")
    .description("The agent proposes the steps and waits for approval.")
    .separator()
    .footer("Available in any conversation")
    .render()
```

`shortcut` shows keycaps beside the title. `description` adds a paragraph
that wraps, `separator` a hairline between two sections and `footer`
secondary information. `icon_row(label, items)` adds a row of labelled icons
built from `TooltipItem`s. Empty text is ignored.

## Faces

```rust
Tooltip::new(anchor, "server", "Rust workshop", palette)
    .placement(TooltipPlacement::Right)
    .faces(
        Some(Icon::Volume),
        people.iter().map(|person| match &person.picture {
            Some(picture) => TooltipFace::new(&person.name, person.color).image(picture.clone()),
            None => TooltipFace::new(&person.name, person.color),
        }),
        not_shown,
    )
    .render()
```

`faces(icon, faces, more)` adds a row for who is somewhere, such as in a
call: the icon (`TOOLTIP_FACE_ICON`, 20 px), then round pictures
`TOOLTIP_FACE` (24 px) wide that overlap by `TOOLTIP_FACE_OVERLAP` (6 px),
each cut from the one before by a ring of the tooltip's fill, and «+N» in a
pill for `more`. A face without a picture shows its initial on its color;
pass pictures already fitted (`RasterImage::cover`). Faces that do not fit
the hint's width join «+N». A row without faces is not added. Like the rest
of the hint it is passive: nothing in it can be clicked.

## Picture and lines

```rust
Tooltip::new(badge, "badge:0", "", palette)
    .picture(artwork, Size::new(96.0, 64.0))
    .lines([
        TextSpan::new("Account age", palette.muted, 14.0, FontFamily::SansMedium),
        TextSpan::new("BLOOM", palette.text, 18.0, FontFamily::SansBold),
        TextSpan::new("5 years, 10 months", palette.muted, 14.0, FontFamily::SansMedium),
    ])
    .centered()
    .shape(TooltipShape { padding_x: 16.0, padding_y: 16.0, ..shape })
    .render()
```

`picture(image, size)` adds a picture in a box `size` large, such as a
badge's artwork: whole inside it at its own proportions, or filling it when
`RasterImage::cover`; a box wider than the hint is scaled down whole. It stands 12 px clear of the blocks
around it, where text stands 6. `lines(spans)` adds lines one under the
other with no space between them, each `TextSpan` wrapped on its own in its
size, face and color (the rest of a span is not drawn), such as a heading
between two quieter lines; blank spans are left out. `centered()` centers
every line and block across the hint, and `shape(TooltipShape)` draws this
hint to its own measures instead of the tokens', such as a roomier padding.
With an empty title the hint starts with its first block.

## Delay, title, pointer and fill

```rust
Tooltip::new(anchor, "server", "Rust workshop", palette)
    .placement(TooltipPlacement::Right)
    .delay_ms(0)
    .title_font_size(16.0)
    .title_font_family(FontFamily::SansMedium)
    .background(Color::rgb(10, 10, 12))
    .render()
```

| Option | Default | What it changes |
| --- | --- | --- |
| `delay_ms` | 350 | How long the pointer rests before the hint shows; 0 shows it at once, for a strip of marks read by moving along it. Keyboard focus shows it at once either way. |
| `title_font_size` | 12 | The title's size. A one-line hint grows by what a larger title adds. |
| `title_font_family` | `Sans` | The title's face. |
| `arrow` | `true` | The small pointer that joins the hint to its anchor. |
| `background` | the palette's | The fill of the hint and its pointer. |

They are kept as `TooltipOptions` on the rendered `ElementKind::Tooltip`. Every
tooltip starts from the tokens' `tooltip_options`, so a host changes the
defaults of all of them, the engine's own hints included, in one place; see
[Tokens](../tokens.md#tooltip).

## Shape and entrance

The tokens' `tooltip_options.shape`, a `TooltipShape`, draws every hint to
the host's measures: its radius, its padding, its shadow and a pointer with a
rounded tip in the body's fill, such as a body rounded 8 with 8 by 12 of
padding and a pointer 16 wide and 10 deep. The body's 1 px outline goes on
round the pointer's sides and tip as one line, with none across its base, as
Discord draws its hints; a transparent `border` draws neither. `open_scale` grows it from that
scale about the pointer's tip as it fades in, over `enter_ms` when set, on
`enter_easing`: `Easing::Spring` grows it a little past full size and back
as a spring does. See [Tokens](../tokens.md#tooltip).

## Content of your own

```rust
Tooltip::new(anchor, "deploy", "Deploy to production", palette)
    .content(
        Element::column(vec![
            Element::text("Deploy to production"),
            Element::row(vec![Element::icon(Icon::Check), Element::text("All checks passed")]).gap(6.0),
        ])
        .gap(4.0),
    )
    .render()
```

`content(element)` lays the body out as an element you build, in place of
the title, shortcut and sections, for a layout the sections do not offer.
The body is sized to it, at most `max_width` wide, with the shape's padding
round it: 8 at the sides and 6 above and below without a `TooltipShape`.
The rest is the hint's own: when it shows, the side it opens on and how it
flips and stays in the window, its fill, border, radius and shadow, its
pointer and its entrance. `title_font_size`, `title_font_family` and
`centered` are the built-in layout's and do not reach it.

What it holds is painted and nothing more: nothing in it can be clicked,
focused or read by a screen reader. So the title is still required, and is
what the platform is told; see [Accessibility](#accessibility). What it would
float over the window, such as an open popover or an overlay that does not
clip its children, is left out, since it would be placed outside the hint. An element
in it that moves or keeps state by its ID does so while the hint shows, as
anywhere else. `TooltipSpec::content` gives a `Button` or an `IconButton`
such a hint.

## Placement

```rust
Tooltip::new(anchor, "more", "More", palette)
    .placement(TooltipPlacement::Right)
    .render()
```

`Above` unless set; `Below`, `Left` and `Right` are the others. The hint flips
to the opposite side when the preferred one would leave the window.

## Title face

```rust
Tooltip::new(anchor, "server:7", "Rust workshop", palette)
    .title_font_family(FontFamily::SansSemibold)
    .render()
```

`title_font_family` sets the face of the title, which is measured in it: a
heavier face wraps or shortens a long title sooner. The default is
`FontFamily::Sans`; descriptions, rows and footers keep their own.

## Width

`max_width` is 360 unless set and never under 32; the window bounds it too.

## A file path

```rust
Tooltip::for_path(anchor, "tab-3", "src/pages/menus.rs", palette).render()
```

`Tooltip::for_path` names a file: its name as the title and the directory
that holds it beneath, no wider than 320.

## Motion

The hint fades in over `Motion::enter_ms`, or the options' `enter_ms`,
growing from `open_scale` when it is below 1 on the options' `enter_easing`
(on `Easing::Spring` past full size and back, its fade over early), and
fades out; moving between neighbouring controls does not blink. Nothing
moves under reduced motion. See [Animation](../animation.md).

## Accessibility

A hint is not seen by who cannot hover, so its title is told to the platform
as the description of the first control in its anchor. A hint without a title
is described by its `lines`, one per line. The hint of a `Button`
or an `IconButton` names an icon alone and describes a button with a label. A
passive anchor is not told to the platform at all.
