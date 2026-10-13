# Slider

A value dragged along a track, from 0 to 1: a seek position or a volume.

## Import

```rust
use limn::{Component, Slider};
```

## Usage

A slider is controlled: pass its fraction each frame and store the one it
reports. Values outside 0 to 1 are clamped.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Slider::new("volume", state.volume, palette).width(280.0).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::SliderChanged { id, fraction, committed } = action {
        if id == "volume" {
            state.volume = *fraction;
            if *committed {
                state.save_volume();
            }
        }
    }
}
```

`committed` is false during a drag and true on release, so a host can follow
the thumb and write the value once.

## Steps

```rust
Slider::new("quality", state.quality, palette).steps(5).render()
```

With `steps(n)` the slider rests on one of `n` evenly spaced stops, the ends
included. Below two it is continuous, which is the default.

## Ink

```rust
Slider::new("seek", state.position, palette).ink(accent).render()
```

`ink` colors the filled part and the thumb, in place of the palette's text.

## Disabled

```rust
Slider::new("volume", state.volume, palette).enabled(false).render()
```

A disabled slider shows its value and takes no pointer or focus.

## Keyboard

| Key | Moves the value |
| --- | --- |
| Left, Right | By a twentieth, or by one stop with `steps` |
| Home, End | To either end |

## Actions

| Action | When |
| --- | --- |
| `UiAction::SliderChanged { id, fraction, committed }` | The value changes: during a drag with `committed` false, on release or from the keyboard with it true. |
