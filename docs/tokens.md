# Tokens

Reference for the values that define the library's shape and pace. For color,
see [Theming](theming.md).

## Setting tokens

```rust
use limn::{set_tokens, tokens, Motion, Radius, Tokens};

set_tokens(Tokens {
    radius: Radius::DEFAULT.scaled(1.5),
    motion: Motion::DEFAULT,
});

let current = tokens(); // read them back
```

| Function | Effect |
| --- | --- |
| `set_tokens(tokens)` | Replaces the tokens for the whole process. |
| `tokens()` | Returns the tokens in force. |

Elements read the tokens when they are built. An application that rebuilds its
tree every frame, which is the normal case, changes on the next frame.

Tokens are process-wide. They cannot differ between two parts of a window or
between two windows.

## Radius

Corner radii in logical pixels, named for what they round.

| Field | Default | Rounds |
| --- | --- | --- |
| `control` | 4 | Buttons, inputs, menu rows, keycaps, tooltips. |
| `surface` | 6 | Surfaces that float: popovers, menus, toasts, the segmented track. |
| `card` | 8 | Groups set into the page: alerts, preference groups, activity cards, the calendar. |
| `panel` | 12 | The largest containers: dialogs, empty states, image previews, the composer. |

| Constructor | Result |
| --- | --- |
| `Radius::DEFAULT` | The table above. |
| `Radius::SQUARE` | Every radius zero. |
| `radius.scaled(factor)` | Every radius multiplied by `factor`. A factor that is negative or not finite leaves it unchanged. |

A radius that is negative or not finite is replaced by its default when the
tokens are set.

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
| Spinners, skeleton pulses, text shimmer, the caret | Fixed periods. Reduced motion stops them. |
| Scrolling | `ScrollMotion`, per scroll area. |
| The crossfade when a popover changes content | Fixed. |
| The delay before a tooltip appears | Fixed at 350 ms. |
| Easing curves | Fixed. |

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
