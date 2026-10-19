# Badge

A small mark over the top right corner of another element, such as an icon
button or an avatar, that says there is something new or how many. It is
passive: the element under it works exactly as it does without the mark.

## Import

```rust
use limn::{Badge, BadgePlacement, Component};
```

## Usage

A badge wraps the element it marks. The host owns the count and passes it
each frame; the badge reports nothing, so the action handled is the child's.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let bell = IconButton::new("notifications", Icon::Bell, palette)
        .accessibility_label("Notifications")
        .render();
    Badge::new(bell, palette)
        .count(state.unread)
        .description(format!("{} unread", state.unread))
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "notifications" {
            state.unread = 0;
        }
    }
}
```

The mark is a pill 16 px high in the palette's text color, with the count in
the surface color and an outline in the surface color that separates it from
what is under it. A single digit stands in a circle.

## Count

```rust
Badge::new(bell, palette).count(3).render()                    // 3
Badge::new(bell, palette).count(120).render()                  // 99+
Badge::new(bell, palette).count(1200).max(999).render()        // 999+
Badge::new(bell, palette).count(0).render()                    // no mark
Badge::new(bell, palette).count(0).show_zero(true).render()    // 0
```

`max` is the largest count written in full: 99 unless set, and at least 1. A
larger count shows the maximum followed by `+`. A count of zero shows no mark
unless `show_zero(true)`. With no mark, `render` returns the child as it was
given.

## Dot

```rust
Badge::new(bell, palette).dot().render()
```

`dot` shows a circle 10 px across with no number, whatever the count is: there
is something new, without saying how many.

## Color

```rust
Badge::new(bell, palette)
    .count(3)
    .color(AlertColors::for_tone(AlertTone::Error, palette).accent)
    .render()
```

`color` sets the fill of the mark. The count and the outline stay in the
surface color, so pass a color the surface reads on, or set them too.

## Corner and size

```rust
Badge::new(server_icon, palette)
    .count(mentions)
    .placement(BadgePlacement::BottomRight)
    .height(23.0)
    .outline(rail_color, 3.0)
    .color(red)
    .ink(Color::rgb(255, 255, 255))
    .font_size(12.0)
    .font_family(FontFamily::SansBold)
    .render()
```

| Option | Default | What it changes |
| --- | --- | --- |
| `placement` | `TopRight` | The corner of the child's box the mark sits in: `TopRight`, `BottomRight`, `BottomLeft` or `TopLeft`. |
| `height` | 16 | The count's height, its outline included, at least 8. A dot keeps the proportion, 10 to 16. |
| `outline(color, width)` | surface, 1 | The ring that parts the mark from what is under it, drawn inside the mark; 0 draws none. A count keeps 3 px clear of it. |
| `ink` | surface | The count's color. |
| `font_size`, `font_family` | 11, `Sans` | The count's text. |

## Layout

The badge takes the place of its child and no more: the same size, the same
fixed size and the same `flex`, so a row of buttons does not move when one
gains or loses its mark. The mark lies inside a corner of the child's box,
the top right unless `placement` says otherwise. A count wider than the child, such as `999+` on a 32 px button,
runs past the child's left edge, over what is beside it.

## Badge or Chip

Use a badge for a mark that belongs to another element and lies over it: the
unread count on a bell, the dot on an avatar. Use a [Chip](chip.md) for a
count or a label that stands in the flow by itself, such as the number after a
section's title or a tag in a row. A chip takes its own space in the layout; a
badge takes none.

## Disabled

A badge has no disabled state, because it is not a control. A disabled child
stays disabled and keeps its mark.

## Keyboard

None of its own. The badge adds no tab stop; Tab, Enter and Space reach the
child as they do without it.

## Actions

None. A click on the child reports the child's action, also where the mark
lies over it.

## Accessibility

The badge has no node of its own. What it means reaches the platform as the
description of the control inside it, in words the host gives:

```rust
Badge::new(bell, palette).count(3).description("3 unread").render()
```

Without `description` the platform is told nothing of the mark, and nothing
is said while no mark shows. The description replaces one the child gave
itself. It goes to the outermost control inside the badge; a child that is
not a control, such as an avatar, needs a node to carry it:

```rust
let photo = Avatar::new("Ada Lovelace", palette)
    .render()
    .accessible(Access::new(Role::Image).id("author").name("Ada Lovelace"));
Badge::new(photo, palette).dot().description("Online").render()
```

## Limits

- The mark does not hang outside the child's box.
- It appears and leaves at once, without a transition.
- It shows a count or a dot, not an icon.
