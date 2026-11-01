# StatusDot

A small round mark in a status color that says the state of what it sits
beside, such as green beside a session that is running. It can pulse while
that state is live.

## Import

```rust
use limn::{Component, StatusDot};
```

## Usage

The host passes the color, and whether the dot pulses, on every rebuild. The
dot is passive and reports nothing.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::row(vec![
        StatusDot::new(state.status_color()).pulse(state.running).render().centered_y(),
        Text::new(&state.title, palette).render(),
    ])
    .gap(8.0)
}
```

## Size

```rust
StatusDot::new(color).size(8.0).render()
```

The dot is 6 px across unless set, and never under 1 px.

## Motion

With `pulse(true)` the dot's opacity eases from 1 to 0.3 and back every
1.2 s. Every pulsing dot in a window keeps the same phase, so several pulse in
step. Under reduced motion it holds still at full opacity.

## Accessibility

With an `id` the dot is a picture the platform knows by that ID, named by its
`description`. Without an `id` the platform is told nothing: say the state in
the text beside it instead.

```rust
StatusDot::new(color)
    .pulse(true)
    .id("session-3:state")
    .description("Running")
    .render()
```
