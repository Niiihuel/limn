# Icon

A semantic icon name, such as `Icon::Trash` or `Icon::Search`. Components
take an `Icon` and the backend chooses the artwork, which is Bootstrap Icons
unless the host gives another set.

## Import

```rust
use limn::{Element, Icon};
```

`Icon` is an enum, not a component: it has no `render`.

## Usage

Pass an icon to the component that shows it, or paint one on its own with
`Element::icon`, which draws it at 16 px in the element's foreground.

```rust
fn view(palette: ComponentPalette) -> Element {
    Element::row(vec![
        Element::icon(Icon::Search).foreground(palette.muted),
        Button::new("open", "Open", palette).icon(Icon::Folder).render(),
    ])
}
```

## The chosen item

```rust
let icon = if chosen { Icon::Folder.filled() } else { Icon::Folder };
Element::icon(icon).foreground(if chosen { palette.text } else { palette.muted })
```

`filled()` returns the solid form of an icon, for the active or chosen item
of a list. An icon without one, such as `Icon::Search`, is returned as it
is.

## Larger

```rust
Element::icon(Icon::Folder).large_icon(48.0)
```

`large_icon(px)` draws a standalone icon larger than 16 px, up to 128, for
thumbnails and empty states. Icons inside controls keep 16 px.

## File types

```rust
Element::icon(file_icon("src/main.rs"))
```

`file_icon(path)` returns the artwork for a file's type. These icons keep
their own colors.

## Artwork of your own

```rust
const LOGO: &str = include_str!("../assets/logo.svg");
Element::icon(Icon::Custom(LOGO)).foreground(palette.text)
```

`Icon::Custom` takes the source of an SVG drawn with `currentColor`. It is
fitted to the icon's square without distortion and tinted like any other
icon.

## Another icon set

```rust
fn lucide(icon: Icon) -> Option<&'static str> {
    match icon {
        Icon::Search => Some(include_str!("../assets/lucide/search.svg")),
        Icon::Trash => Some(include_str!("../assets/lucide/trash-2.svg")),
        _ => None,
    }
}

limn_egui::set_icon_pack(lucide);
```

A pack is a function from an `Icon` to the SVG that draws it, or `None` to
leave that one to Bootstrap Icons, so it can replace a few icons or all of
them. The artwork must draw with `currentColor`. Call `set_icon_pack` once
on the UI thread when the window is created, before the first frame: an icon
already drawn stays as it was. File type icons and `Icon::Custom` are not
asked of the pack.

See [Icons](../components.md#icons) in the component reference.
