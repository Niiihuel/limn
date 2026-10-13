# Switch

An on/off control for a setting that takes effect at once.

## Import

```rust
use limn::{Component, Switch};
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

## Disabled

```rust
Switch::new("sync", true, palette).disabled().render()
Switch::new("sync", state.sync, palette).enabled(state.signed_in).render()
```

A disabled switch shows its value and takes no pointer or focus.

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
