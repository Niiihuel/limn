# Tokens

Reference for the values that define the library's shape and pace, and the
one look a host may give every tooltip. For color, see [Theming](theming.md).

## Setting tokens

```rust
use limn::{set_tokens, tokens, Motion, Radius, Tokens};

set_tokens(Tokens {
    radius: Radius::DEFAULT.scaled(1.5),
    motion: Motion::DEFAULT,
    ..Tokens::DEFAULT
});

let current = tokens(); // read them back
```

| Function | Effect |
| --- | --- |
| `set_tokens(tokens)` | Replaces the tokens for the whole process. |
| `with_tokens(tokens, f)` | Runs `f` with these tokens in force on the current thread only, then puts back the ones before. |
| `tokens()` | Returns the tokens in force: the innermost `with_tokens` on this thread, else the process's. |

Elements read the tokens when they are built. An application that rebuilds its
tree every frame, which is the normal case, changes on the next frame.

Tokens are process-wide unless a thread runs under `with_tokens`. Since both
building elements and laying them out read them, a window with tokens of its
own builds and lays out its tree inside the call:

```rust
let frame = limn::with_tokens(compact, || {
    tree.set_root(view(&state));
    tree.layout(size)
});
```

They cannot differ between two parts of one tree.

## Radius

Corner radii in logical pixels, named for what they round.

| Field | Default | Rounds |
| --- | --- | --- |
| `control` | 4 | Buttons, inputs, menu rows, keycaps, tooltips. |
| `surface` | 6 | Surfaces that float: popovers, menus, toasts, the segmented track. |
| `card` | 8 | Groups set into the page: alerts, preference groups, activity cards, the calendar. |
| `tile` | 16 | Cards that stand on their own in a grid: a directory's entries, a feature's card. |
| `panel` | 12 | The largest containers: dialogs, image previews, the composer. |

| Constructor | Result |
| --- | --- |
| `Radius::DEFAULT` | The table above. |
| `Radius::SQUARE` | Every radius zero. |
| `Radius::ROUND` | Controls as pills: 16, 16, 20, 24 and 28. |
| `radius.scaled(factor)` | Every radius multiplied by `factor`. A factor that is negative or not finite leaves it unchanged. |

A radius that is negative or not finite is replaced by its default when the
tokens are set.

A component that lets its caller choose a radius takes a `RadiusRole`
(`Control`, `Surface`, `Card`, `Tile` or `Panel`), never a number. `role.px()` reads
the radius in force for that role. `RadiusRole::Square` is always zero: a
cell that sits flush inside a row or a bar that paints the surface.

### Not governed by radius tokens

| What | Why |
| --- | --- |
| Pills and circles: chips, avatars, the switch, the progress bar, step badges | These are shapes, not rounded corners. Their radius is half their height. |
| Marks inside content: text selections, chart bars, code editor marks | They keep their own 2 to 3 px so they stay legible at any setting. |

Image attachments use the `panel` radius scaled to keep their nested corners
concentric.

## Motion

Durations in milliseconds, named for the kind of change.

| Field | Default | Applies to |
| --- | --- | --- |
| `hover_ms` | 120 | A control answering the pointer: hover and press fills. |
| `enter_ms` | 140 | A surface arriving: popover, menu, dialog, tooltip, toast, an opening collapsible. |
| `exit_ms` | 110 | A surface leaving. |
| `move_ms` | 170 | Something travelling or changing state in place: tab indicator, switch thumb, step badge. For things that travel it is the time a spring takes to cover 99% of the distance. |

| Constructor | Result |
| --- | --- |
| `Motion::DEFAULT` | The table above. |
| `motion.scaled(factor)` | Every duration multiplied by `factor`: `2.0` is half speed, `0.5` is double. A factor that is negative or not finite leaves it unchanged. |

A field set to zero makes that kind of change immediate.

A surface never takes longer to leave than to arrive: its exit lasts `exit_ms`
or its own entrance duration, whichever is shorter.

### Not governed by motion tokens

| What | Controlled by |
| --- | --- |
| Whether anything moves at all | `UiTree::set_reduced_motion(true)`. See [Animation](animation.md#switching-motion-off). |
| Spinners, scribbles, skeleton pulses, text shimmer, the caret | Fixed periods. Reduced motion stops them. |
| Scrolling | `ScrollMotion`, per scroll area. |
| The crossfade when a popover changes content | Fixed. |
| The delay before a tooltip appears | `tooltip_options.delay_ms`, 350 ms; see [Tooltip](#tooltip). |
| Easing curves | Fixed. |

## Tooltip

Each palette derives its tooltips' colors. `tooltip: Some(style)` paints
every tooltip with one `TooltipStyle` instead, whichever palette its control
was built with, such as a music app's:

```rust
use limn::{set_tokens, Color, Tokens, TooltipStyle};

set_tokens(Tokens {
    tooltip: Some(TooltipStyle {
        background: Color::rgb(40, 40, 40),
        border: Color::TRANSPARENT, // no outline
        keycap: Color::rgb(62, 62, 62),
        text: Color::WHITE,
        muted: Color::rgb(179, 179, 179),
        shadow: Color::rgba(0, 0, 0, 128),
    }),
    ..Tokens::DEFAULT
});
```

`None`, the default, leaves each palette its own. A tooltip given its own
`background` keeps it.

`tooltip_options` are the `TooltipOptions` every tooltip starts from: its
delay, its title's size and face, its pointer, its shape and its entrance. A
`Tooltip`'s builders change them for that one hint; the hints the engine draws
itself, such as a menu row's, take them as they are. `TooltipOptions::DEFAULT`
is the library's hint.

```rust
use limn::{set_tokens, FontFamily, Point, Size, Tokens, TooltipOptions, TooltipShape};

set_tokens(Tokens {
    tooltip_options: TooltipOptions {
        delay_ms: 70,
        title_font_size: 14.0,
        title_font_family: FontFamily::SansMedium,
        shape: Some(TooltipShape {
            radius: 8.0,
            padding_x: 12.0,
            padding_y: 8.0,
            shadow_offset: Point::new(0.0, 12.0),
            shadow_blur: 24.0,
            caret: Size::new(16.0, 10.0),
        }),
        open_scale: 0.95,
        enter_ms: Some(70),
        ..TooltipOptions::DEFAULT
    },
    ..Tokens::DEFAULT
});
```

| Field | Default | What it changes |
| --- | --- | --- |
| `shape` | `None` | The body's radius, the padding at its sides and above and below its text (a one-line hint is its title's line and twice that), its shadow's offset and blur, and the pointer's width and depth, a triangle with a rounded tip in the body's fill, the body's outline going on round its sides and tip, joined to it and not across its base. The hint stands the pointer's depth and 2 px off its anchor. `None` is the control radius, 8 at the sides, one line in 24, a shadow 2 below blurred 6 and an outlined 10 by 5 pointer. |
| `open_scale` | 1 | The scale the hint grows from about its pointer's tip while it fades in, within 0.5 to 1. |
| `enter_ms` | `None` | How long it fades in, in place of `Motion::enter_ms`. |
| `enter_easing` | `Easing::OutCubic` | The curve it fades in and grows on. `Easing::Spring` grows it past full size and back, settling at the end of `enter_ms`, while its fade is over in about a quarter of it: with `open_scale` 0.95 and `enter_ms` 200, it is opaque by 55 ms, 0.7 % past full size at 75 and settled at 200. |

`set_tokens` makes the options' numbers safe: a title size that is not a
positive number is 12, the scale is clamped and the shape's measures are
never negative.

## Shortcuts

`shortcut_style` says how keyboard shortcuts are drawn: `ShortcutStyle::Keycaps`,
the default, puts each key in an outlined cap; `ShortcutStyle::Text` draws the
whole combination as one line of muted text, such as "Ctrl+Shift+I", in every
menu and `Kbd`, and in a tooltip in its style's `muted`. A text field's
shortcut hint keeps its keycaps.

```rust
use limn::{set_tokens, ShortcutStyle, Tokens};

set_tokens(Tokens { shortcut_style: ShortcutStyle::Text, ..Tokens::DEFAULT });
```

## Focus ring

`focus_ring` is the `Ring` drawn around the control that has the keyboard
focus: a button, a clickable, a link in text, the effort slider.

```rust
use limn::{set_tokens, Color, Ring, Tokens};

set_tokens(Tokens {
    focus_ring: Ring { color: Color::rgb(0, 120, 255), width: 2.0, offset: 2.0, radius: None },
    ..Tokens::DEFAULT
});
```

| Field | Default | What it changes |
| --- | --- | --- |
| `color` | #AAAAAA | The stroke's color. |
| `width` | 2 | Its thickness in logical pixels, never more than half the ringed rectangle's side. |
| `offset` | 0 | How far outside the control the ring's outer edge is; 0 strokes inside it. |
| `radius` | `None` | The ring's corner radius. `None` follows the control's radius or corner radii, grown by the offset so the two stay concentric. |

`Element::focus_ring_style(ring)` gives one element a ring of its own in
place of the token's, and `Element::focus_ring(false)` still turns it off. A
width that is negative or not finite draws no ring, and an offset that is not
finite or a radius that is negative or not finite is 0. `Ring::default()` is
the library's ring.

## Per-component overrides

Most animated components accept an explicit duration that takes precedence
over the token for that one instance:

```rust
Popover::new(anchor, content, open, "dismiss", palette).animation_ms(0) // no entrance
Tabs::new(items, palette).animation_ms(300)                             // slower indicator
Element::button("id", "Label").transition_ms(0)                         // instant hover
```

Prefer the tokens. An override is for a component that must differ from the
rest of the application.

## See also

- [Theming](theming.md) for color.
- [Animation](animation.md) for the behavior these durations drive.
