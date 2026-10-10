# Animation

What the library animates, how long each animation lasts, how to animate your
own content, and how to switch movement off.

## Principles

- **Animation is automatic.** Opening a menu, hovering a button or flipping a
  switch animates without any code in the application. You describe the new
  state; the tree moves from the old one.
- **Animation belongs to an ID.** A transition is remembered under the
  control's ID, so it survives the element being rebuilt every frame and is
  not restarted by an unrelated change.
- **Animation never changes behavior.** Layout, hit testing and focus use the
  final state at once. A closing menu stops receiving clicks the moment it
  closes, while its picture is still fading.
- **Four durations cover everything.** A component takes one of them instead
  of inventing a number, so the same kind of change always takes the same
  time. See [Tokens](tokens.md#motion).

## What animates

| What | Entering | Leaving | Duration |
| --- | --- | --- | --- |
| `Popover`, `DropdownMenu`, `DropdownList`, `Modal`, `AlertDialog` | Fades in and settles 4 px into place. | Fades out. | `enter_ms`, `exit_ms` |
| The dimmed backdrop of `Modal` and `AlertDialog` | Fades in with the dialog. | Fades out with it. | `enter_ms`, `exit_ms` |
| Tooltip | Fades in. | Fades out. Moving between neighbouring controls does not blink. | `enter_ms`, `exit_ms` |
| Context menu | Fades in and settles. | Fades out. | `enter_ms`, `exit_ms` |
| `Toast` | Fades in and settles. | Fades out where it was. | `enter_ms`, `exit_ms` |
| Body of a `Collapsible` | Fades in and settles. | Removed at once. | `enter_ms` |
| Hover and press on any control | The fill cross-fades. | The fill cross-fades. | `hover_ms` |
| `Tabs`, `SegmentedControl` | The selection springs to the chosen tab. | | `move_ms` |
| `Switch` | The thumb springs across. | | `move_ms` |
| `Stepper` | The badge cross-fades between active and inactive. | | `move_ms` |

Continuous indicators run on fixed periods and are not transitions: `Spinner`,
`Skeleton`, `TextShimmer` and the text caret.

### Not animated

- The height of a `Collapsible`. The content below it moves at once.
- A value changing in `Progress`. The fill jumps to the new fraction.

## Switching motion off

```rust
tree.set_reduced_motion(true);
```

One switch for the whole tree. Every surface appears and disappears at once,
indicators jump to their place, and spinners, pulses and shimmer hold still.
Nothing about layout or behavior changes.

Use this for a user's accessibility preference. The library does not read the
operating system's reduced-motion setting; the host passes it in.

To change the pace instead of removing movement, scale the durations:

```rust
use limn::{set_tokens, Motion, Tokens};

set_tokens(Tokens { motion: Motion::DEFAULT.scaled(0.5), ..Tokens::DEFAULT });
```

## Keeping the window painting

The tree only produces a frame when asked. While something is animating, the
host must keep asking:

```rust
if let Some(after) = tree.animation_repaint_after() {
    ctx.request_repaint_after(after);
}
```

`animation_repaint_after` returns how soon the next frame is needed:

| Returns | When | What the host does |
| --- | --- | --- |
| `None` | Everything is at rest. | Nothing. An idle window costs nothing. |
| `Some(Duration::ZERO)` | A transition is running, or the user is scrolling or dragging. | Paints on the very next display refresh. |
| `Some(16 ms)` | Only ambient indicators are running: a spinner, a skeleton, a shimmer. | Paints at that cadence. |
| A longer wait | A tooltip is about to open, or the caret is about to blink. | Wakes once, then. |

Zero matters. A transition paced by a fixed timer runs at half the rate of a
120 Hz display and drifts against any display, which reads as stutter even
when every frame is cheap. Painting each refresh keeps movement at the
display's own rate and in step with it. Interpolation uses elapsed time, so a
transition lasts the same on every display.

Ambient indicators never finish, so they keep a slower cadence rather than
holding the display at full rate for as long as they are visible.

`tree.is_animating()` is the same question as a boolean.

## Animating your own content

### An element that appears

```rust
let details = Element::column(rows).animate_enter("details-body");
```

The element fades in and settles the first time it is laid out. Remove it from
the tree and it is gone at once; add it again and it enters again. Use this
for content in the page's flow.

### An element that appears and disappears

```rust
let banner = Element::panel(message).animate_presence("offline-banner");
```

As above, and when it is removed its last picture stays behind and fades out
where it was. Use this only for **floating** content, such as a toast or a
badge in an overlay. For content in the flow the neighbours move into its
place at once, and a fading ghost on top of them looks wrong.

Both take an ID. Give two elements the same ID only if they are the same thing
in two states.

### A control's fill

```rust
Element::clickable("row", content)
    .hover_background(hover)
    .transition_ms(limn::tokens().motion.hover_ms)
```

The background cross-fades between its normal, hovered and pressed colors.
Components do this already; you need it only for controls built from raw
elements.

### A value the application owns

For a width, an offset or an opacity that your own state drives, keep a
`ScalarTransition` in your state:

```rust
use limn::{Easing, ScalarTransition};
use std::time::{Duration, Instant};

// When the target changes:
self.panel_width.retarget(target, Instant::now(), Duration::from_millis(170));

// Every frame:
let width = self.panel_width.value(Instant::now());
if self.panel_width.is_active(Instant::now()) {
    ctx.request_repaint();
}
```

| Method | Effect |
| --- | --- |
| `ScalarTransition::new(value, now)` | Starts at rest at `value`. |
| `.easing(Easing::InOutCubic)` | Chooses the curve; the default is `OutCubic`. |
| `.retarget(value, now, duration)` | Heads for a new value from wherever it is now, without a jump. |
| `.retarget_with_motion(value, now, duration, reduced_motion)` | As above, or a snap when `reduced_motion` is true. |
| `.snap_to(value, now)` | Jumps immediately. |
| `.value(now)`, `.target()`, `.is_active(now)` | Reads it. |

`ScalarSpring` is the same idea with velocity: an interrupted movement or a
released drag carries on instead of restarting from rest.

## How transitions behave

- **Retargeting is continuous.** Hovering a button and leaving before the
  fade finishes reverses from the color it had reached.
- **Reopening continues.** A menu reopened while it is still fading out picks
  up from its visible opacity instead of starting again from zero.
- **Colors mix as painted.** A fade between an opaque fill and a translucent
  veil mixes what each contributes on screen, so it never flashes brighter
  than both ends.
- **Curves are fixed.** Fills and surfaces ease out: quick to start, soft to
  finish.
- **Things that travel are springs.** A tab indicator or a switch thumb moves
  on a spring with no overshoot. Redirect it mid-flight, by choosing a third
  tab before it reaches the second, and it keeps its speed and bends towards
  the new target instead of stopping and starting again. `move_ms` is the time
  in which it covers 99% of the distance; it settles a little after.

## Adding animation to a component

1. Pick the duration by what kind of change it is, from
   `limn::tokens().motion`. Do not write a number.
2. Use the mechanism that already exists: a popover for anything anchored and
   dismissible, `animate_enter` or `animate_presence` for content,
   `transition_ms` for a fill, a tab strip's selection for an indicator that
   travels between positions.
3. Check it under `set_reduced_motion(true)`: it must reach its final state on
   the first frame.
4. Test it with a fixed clock. See [Testing](testing.md#animation).

## See also

- [Tokens](tokens.md#motion) for the duration values.
- [Testing](testing.md) for testing animated components.
