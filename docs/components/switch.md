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

The thumb is always 3 pixels inside the track.

## Variant

```rust
Switch::new("embeds", state.embeds, palette).variant(SwitchVariant::Accent)
```

| Variant | On | Off |
| --- | --- | --- |
| `Default` | A thumb of the palette's surface on a track of its text | A thumb of the palette's text on its selected fill |
| `Accent` | A light thumb on a track of the palette's `link` color | The same thumb on the palette's muted color |

`Accent` is how chat applications draw a switch. Its thumb is the lightest
color the palette has: its text on a dark palette, its surface on a light one.

## Ink

```rust
Switch::new("embeds", state.embeds, palette).variant(SwitchVariant::Accent).ink(theme.accent)
```

`ink` is the track's color while the switch is on, in either variant. Choose
one the thumb reads on: a light thumb disappears on a pale ink.

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
