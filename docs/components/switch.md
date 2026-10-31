# Switch

An on/off control for a setting that takes effect at once.

## Import

```rust
use limn::{Component, ControlSize, Switch, SwitchVariant};
```

## Usage

A switch is controlled: pass whether it is on each frame, and flip your own
value when it reports a click.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Switch::new("notifications", state.notifications, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "notifications" {
            state.notifications = !state.notifications;
        }
    }
}
```

## Size

```rust
Switch::new("notifications", state.notifications, palette).size(ControlSize::Spacious)
```

| Size | Track | Thumb |
| --- | --- | --- |
| `Dense` | 28 × 16 | 10 |
| `Compact` | 32 × 18 | 12 |
| `Regular`, the default | 36 × 20 | 14 |
| `Spacious` | 44 × 24 | 18 |

The thumb is 3 pixels inside the track, or as `thumb_inset` says below.

```rust
Switch::new("notifications", state.notifications, palette).track_size(50.0, 26.0)
```

`track_size(width, height)` takes a track of the host's own size in place of
a control size's, such as a settings page's 50 × 26: the thumb is the height
less twice its inset, 6 by default. The height is at least 12, and the width
at least twice the height less twice the inset, so the thumb has room to
travel. Whichever of `size` and `track_size` comes last decides.

## Thumb inset and off track

```rust
Switch::new("notifications", state.notifications, palette)
    .track_size(40.0, 24.0)
    .thumb_inset(4.0)
    .off_track(Color::rgba(255, 255, 255, 40), palette.border)
```

`thumb_inset(px)` sets how far the thumb stands in from the track's edges:
3 pixels unless set. A 24-pixel track with a 16-pixel thumb takes 4. The
inset is at most half the track's height, so the thumb is never negative,
and a value that is not finite is ignored.

`off_track(fill, border)` paints the track while the switch is off in `fill`,
with a 1-pixel `border` around it. `fill` may be translucent, as Discord's
off track is, so what lies under it shows through. It replaces the variant's
off track in either variant; the thumb keeps its color, and the track while
the switch is on is unchanged.

## Variant

```rust
Switch::new("embeds", state.embeds, palette).variant(SwitchVariant::Accent)
```

| Variant | On | Off |
| --- | --- | --- |
| `Default` | A thumb of the palette's surface on a track of its text | A thumb of the palette's text on its selected fill |
| `Accent` | A light thumb on a track of the palette's `link` color | The same thumb on the palette's muted color |

`Accent` is how chat applications draw a switch. Its thumb is the lightest
color the palette has, at full strength: its text on a dark palette, its
surface on a light one. Its track, on or off, is darkened where needed until
the thumb stands at a contrast of 3 or more on it, the least a part that shows
state needs.

## Ink

```rust
Switch::new("embeds", state.embeds, palette).variant(SwitchVariant::Accent).ink(theme.accent)
```

`ink` is the track's color while the switch is on, in either variant. On
`Accent` a pale ink is darkened as the `link` color is. On `Default` it is
used as given, so choose one its thumb, the palette's surface, reads on.

## Disabled

```rust
Switch::new("sync", true, palette).disabled().render()
Switch::new("sync", state.sync, palette).enabled(state.signed_in).render()
```

A disabled switch shows its value, in the colors it has enabled, and takes no
pointer or focus.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The switch is flipped by pointer, Enter or Space. |

## Accessibility

The platform is told a switch and whether it is on, disabled or not. A switch
shows no text, so the host names it, or puts it in a
[Form field](form-field.md), whose label names it.

```rust
Switch::new("wifi", wifi_on, palette).accessibility_label("Wi-Fi")
```
