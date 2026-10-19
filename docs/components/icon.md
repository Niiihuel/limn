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

## Smaller

```rust
Element::icon(Icon::Lock).small_icon(10.0)
```

`small_icon(px)` draws a standalone icon smaller than 16 px, down to
`SMALL_ICON_SIZE` (8), such as a padlock over a corner of another icon or a
speaker inside a small badge. The element measures the side it is drawn at.

## Categories

`Icon::Tree`, `FriedEgg`, `Gamepad`, `Car`, `Lightbulb` and `Flag`, with
`Emoji` and `Heart`, name the usual categories of an emoji picker: nature,
food, activities, travel, objects and flags, people and symbols.

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

## Platform-rendered icons

A host can render system artwork, such as SF Symbols on iOS, without
converting it to SVG. Supply a function with this signature:

```rust
fn render_platform_icon(icon: Icon, pixels: u16) -> Option<eframe::egui::ColorImage>
```

Install it on the UI thread with
`limn_egui::set_icon_rasterizer(Some(render_platform_icon))`. It receives the
final physical pixel side, including display scaling. Return a square image
of that exact size containing a white alpha mask; the painter applies the
element's color. Native premultiplied RGBA bytes can be converted with
`ColorImage::from_rgba_premultiplied`.

Return `None` for an unsupported symbol or for application artwork that
should remain unchanged. A missing or malformed image uses the configured
SVG pack, then the usual bundled artwork. The rasterizer can explicitly
handle `Icon::Custom`; file-type icons always keep their original colors.

The normal bounded texture cache reuses each icon at each physical size.
Replacing the renderer, or removing it with `set_icon_rasterizer(None)`,
clears only cached icon textures and requests a repaint. Raster images and
the application's own artwork data remain in their separate caches.

See [Icons](../components.md#icons) in the component reference.
