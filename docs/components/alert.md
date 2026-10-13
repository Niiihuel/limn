# Alert

A persistent inline notice in one of five tones, with an optional action and
a dismiss button. It has no timer and takes no focus: it stays until the host
stops rendering it.

## Import

```rust
use limn::{Alert, AlertTone, Component};
```

## Usage

An alert needs a title, a description and the palette. It fills the width it
is given.

```rust
fn view(palette: ComponentPalette) -> Element {
    Alert::new("Saved", "Your changes were saved.", palette)
        .tone(AlertTone::Success)
        .render()
}
```

An empty description is left out. An empty title is replaced by the tone's
own label.

## Tones

`Neutral` is the default; `Info`, `Success`, `Warning` and `Error` are the
others. The tone chooses the icon and its color. The surface and the border
stay neutral, and only `Error` also tints the title and the description.

```rust
Alert::new("Payment declined", "Check the payment method and try again.", palette)
    .tone(AlertTone::Error)
    .render()
```

`colors` takes a whole `AlertColors` in place of the tone's defaults.
`AlertColors::for_tone(tone, palette)` gives those defaults, whose `accent` is
a status color a host can use elsewhere.

## Action

```rust
Alert::new("Dark mode is available", "Turn it on in your profile settings.", palette)
    .action(Button::new("enable-dark", "Enable", palette).variant(ButtonVariant::Outline).render())
    .render()
```

`action` takes a control or a group of controls of yours, which keep their own
IDs. It sits beside the copy, at the top. `compact(true)` puts it below the
message, for a narrow layout.

## Dismiss

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    if state.renewal_dismissed {
        return Element::spacer();
    }
    Alert::new("Your subscription ends in 3 days", "Renew it to keep access.", palette)
        .tone(AlertTone::Warning)
        .dismiss("dismiss-renewal")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "dismiss-renewal" {
            state.renewal_dismissed = true;
        }
    }
}
```

`dismiss` adds a close button at the end. Dismissal is controlled: the alert
reports the click and stays until the host hides it.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The close button is activated, with the ID given to `dismiss`. |

A control passed to `action` reports its own actions.

## Accessibility

With an `id` the platform is told of an alert known by that ID, named by its
title and described by its description. The close button is a button named by
`labels().dismiss_notice`. Without an `id` only the alert's controls are known.

```rust
Alert::new("Offline", "Changes are kept on this device.", palette)
    .id("offline")
    .render()
```
