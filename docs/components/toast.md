# Toast

A brief card in the bottom right corner of the window that announces
something that happened: a save, a copy, a failure.

## Import

```rust
use limn::{AlertTone, Component, Toast};
```

## Usage

Toasts are laid over the window's root element with `Toast::layer`, built
last, around everything else. A toast has no timer: the host decides when it
appears and leaves it out of the list when it should go.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let root = window_content(state, palette);
    let toasts = state
        .saved
        .then(|| Toast::new("Changes saved", palette).description("notes.md").render())
        .into_iter()
        .collect();
    Toast::layer(root, toasts, 16.0)
}
```

`Toast::layer(base, toasts, bottom_inset)` stacks the cards upward from the
bottom right corner, `bottom_inset` above the window's lower edge. Everything
outside the cards stays interactive, and with no toasts it returns the base
alone. A card never takes keyboard focus.

## Description and tone

```rust
Toast::new("Offline", palette)
    .description("Retrying in a few seconds.")
    .tone(AlertTone::Error)
    .render()
```

The description is a muted second line. The tone colors the title: `Neutral`
unless set, or `Info`, `Success`, `Warning`, `Error`. The surface stays
neutral. Long copy wraps and the card grows; it is never cut.

## Dismiss and action

```rust
Toast::new("Turn finished", palette)
    .action("turn:open")
    .dismissible("turn:dismiss")
    .render()
```

`dismissible(id)` adds a close button and `action(id)` makes the card's copy
one click target. Both only report: the host removes the toast.

## Width

`width` is 360 unless set and never under 120.

## Motion

A card fades in and settles when it appears, and fades out where it was once
the host drops it; see [Animation](../animation.md). With reduced motion it
appears and goes at once. A card is identified by its title, so two cards
shown together need different titles to animate apart.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the `action` ID | The card's copy is clicked. |
| `UiAction::Clicked { id }` with the `dismissible` ID | The close button is activated. |
