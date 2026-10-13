# Avatar

A round account photo, or the initial of a name where there is no photo.

## Import

```rust
use limn::{Avatar, Component};
```

## Usage

An avatar takes a name and the palette. Without a photo it draws the first
character of the name, in upper case, on the palette's `selected`; a name
that is empty or only spaces draws `?`. It reports nothing.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Avatar::new(&state.account.name, palette).render()
}
```

## Photo

```rust
Avatar::new("Ada Lovelace", palette).image(photo).render()
```

`image` takes a `RasterImage` and shows it in the circle in place of the
initial. The host provides a square photo; one of another shape fills the
circle when it is marked `.cover()`.

## Size

```rust
Avatar::new("Ada Lovelace", palette).size(48.0).render()
```

`size` is the diameter, from 16 to 256; the default is 32. The initial scales
with it. A size that is not finite is ignored.
