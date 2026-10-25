# Progress

A thin bar for work whose completed share is known. For work of unknown
duration use a [Spinner](spinner.md).

## Import

```rust
use limn::{Component, Progress};
```

## Usage

The host owns the value and passes it on every rebuild, as a fraction from 0
to 1. Values outside are clamped and NaN is empty.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Progress::new(state.uploaded, palette).width(336.0).render()
}
```

The fill is `palette.text` and the rest of the track `palette.selected`.
Without a `width` the bar fills the available horizontal space. It is passive
and reports nothing.

## Height

```rust
Progress::new(1.0, palette).height(8.0).render()
```

The bar is 4 px high unless set, and never thinner than 1 px. Its ends are
rounded to half its height; the fill ends flat against the remaining track
until it is complete.

## Flat

```rust
let status = ComponentPalette { text: warning, ..palette };
Progress::new(state.quota, status).flat().render()
```

`flat` gives the bar square ends, as two pieces side by side: the meter of a
dense list, where a rounded cap would read as a control. The colors come from
the palette, so name a status color as its `text`.

## Ring

```rust
Progress::new(state.context_used, palette).ring(16.0).render()
```

`ring` draws a ring `diameter` pixels across in place of the bar: the gauge
of how much of a budget is used, such as a model's context. The arc covers
the fraction clockwise from twelve o'clock in `palette.text`, over the rest of
the circle in `palette.selected`. `width`, `height` and `flat` do not apply to
it. A diameter below 1 is 1.

## Motion

A value that changes is not animated: the fill jumps to the new fraction.

## Accessibility

With an `id` the bar is a progress indicator the platform knows by that ID,
with its fraction as the value. It shows no text, so the host names what it
renders. Without an `id` the platform is told nothing.

```rust
Progress::new(state.uploaded, palette)
    .id("upload")
    .render()
    .accessibility_label("Uploading")
```
