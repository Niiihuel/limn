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

## Delay

```rust
Tooltip::new(anchor, "liked", "Liked Songs", palette)
    .placement(TooltipPlacement::Right)
    .delay_ms(0)
    .render()
```

`delay_ms` is how long the pointer rests on the anchor before the hint
shows: 350 unless set, and 0 shows it as soon as the pointer is over it, for
a strip of items read by moving along it. Keyboard focus shows it at once
either way. It is kept as `TooltipOptions` on the rendered
`ElementKind::Tooltip`.

## Placement

```rust
Tooltip::new(anchor, "more", "More", palette)
    .placement(TooltipPlacement::Right)
    .render()
```

`Above` unless set; `Below`, `Left` and `Right` are the others. The hint flips
to the opposite side when the preferred one would leave the window.

## Width

`max_width` is 360 unless set and never under 32; the window bounds it too.

## A file path

```rust
Tooltip::for_path(anchor, "tab-3", "src/pages/menus.rs", palette).render()
```

`Tooltip::for_path` names a file: its name as the title and the directory
that holds it beneath, no wider than 320.

## Motion

The hint fades in and out, and moving between neighbouring controls does not
blink; see [Animation](../animation.md).

## Accessibility

A hint is not seen by who cannot hover, so its title is told to the platform
as the description of the first control in its anchor. The hint of a `Button`
or an `IconButton` names an icon alone and describes a button with a label. A
passive anchor is not told to the platform at all.
