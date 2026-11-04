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
a width as well.

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
padding and a pointer 16 wide and 10 deep. `open_scale` grows it from that
scale about the pointer's tip as it fades in, over `enter_ms` when set, on
`enter_easing`: `Easing::Spring` grows it a little past full size and back
as a spring does. See [Tokens](../tokens.md#tooltip).

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
as the description of the first control in its anchor. The hint of a `Button`
or an `IconButton` names an icon alone and describes a button with a label. A
passive anchor is not told to the platform at all.
