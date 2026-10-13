# Notice

A left-aligned inline notice that stands where a pane's content would be: an
optional icon, a title, a reason, and controls the host owns. Use
[Empty](empty.md) for a centered blank slate and [Alert](alert.md) for a toned
banner.

## Import

```rust
use limn::{Component, Icon, Notice, NoticeSizing};
```

## Usage

A notice needs a title and the palette. It is passive: it reports nothing of
its own.

```rust
fn view(palette: ComponentPalette) -> Element {
    Notice::new("No results", palette)
        .icon(Icon::Search)
        .description("Try another search or change the filter.")
        .render()
}
```

The icon is muted and level with the first line of the copy. The description
is a muted line under the title, and both wrap in a narrow pane.

## Action and trailing

```rust
Notice::new("You have no skills installed", palette)
    .description("Choose one from the catalog.")
    .action(Element::row(vec![
        Button::new("explore", "Explore catalog", palette)
            .variant(ButtonVariant::Outline)
            .button_size(ButtonSize::Small)
            .render(),
        Element::spacer().flex(1.0),
    ]))
    .render()
```

`action` places a control of yours under the copy, across its width; put it in
a row before a flexible spacer to keep it at its own width. `trailing` places
one after the copy, at the far edge. Both keep their own IDs and report their
own actions.

## Stacked

```rust
Notice::new("Your agent, next to the code", palette)
    .icon(Icon::Chat)
    .description("Explore the project or ask for a change.")
    .stacked()
    .render()
```

`stacked` sets the icon over the copy instead of beside it, for a hint at the
top of an empty pane.

## Framed

```rust
let raised = ComponentPalette { surface: palette.raised, ..palette };
Notice::new("You have no skills installed", raised).framed().render()
```

`framed` fills the notice with the palette's `surface` and rounds it with the
card radius, with no border. The fill comes from the palette the notice is
given, so pass one whose surface is the fill you want.

## Ink

```rust
Notice::new("The process ended with an error", palette).ink(danger).render()
```

`ink` colors the title only, for a meaning the host owns such as an error.

## Density

```rust
Notice::new("No problems in this file", palette)
    .sizing(NoticeSizing::COMPACT)
    .render()
```

`NoticeSizing::REGULAR` is the default, for a notice that stands in for the
content of a page. `COMPACT` is a status in a narrow panel or a list: a
smaller icon and type, and less padding.

## Fixed height

```rust
const BAR: NoticeSizing = NoticeSizing {
    padding: 4.0,
    height: Some(40.0),
    title_height: Some(32.0),
    ..NoticeSizing::COMPACT
};

Notice::new(&state.status, palette).sizing(BAR).width(420.0).render()
```

A profile with a `height` cuts each line of copy with an ellipsis instead of
wrapping it. `title_height` and `description_height` are the boxes a cut line
is centered in; without them a line follows the font. `width` fixes the width.

## Accessibility

With an `id` the platform is told of a status known by that ID, named by the
title and described by the description. Without one only the controls the host
put in the notice are known.

```rust
Notice::new("No skills installed", palette)
    .id("no-skills")
    .description("Choose one from the catalog.")
    .render()
```
