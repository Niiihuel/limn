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

## Color

```rust
Avatar::new("Ada Lovelace", palette).color(person.color).render()
```

`color` is the fill behind the initial, such as the color an application
gives each person, in place of the palette's `selected`. The initial keeps
the palette's `text`. A photo covers it.

## Size

```rust
Avatar::new("Ada Lovelace", palette).size(48.0).render()
```

`size` is the diameter, from 16 to 256; the default is 32. The initial scales
with it. A size that is not finite is ignored.

## Accessibility

An avatar is decoration: it tells the platform nothing, because it usually
stands beside the person's name, which would be said twice. Where it stands
alone, give it a node of its own:

```rust
Avatar::new("Ada Lovelace", palette)
    .render()
    .accessible(Access::new(Role::Image).id("author-avatar").name("Ada Lovelace"))
```
