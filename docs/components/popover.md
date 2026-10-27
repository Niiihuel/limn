# Popover

A floating surface anchored to any element, for content of the host's own: a
short form, a share panel, a list. For a list of actions use a
[DropdownMenu](dropdown-menu.md).

## Import

```rust
use limn::{Component, Popover, PopoverPlacement};
```

## Usage

A popover wraps its anchor where the anchor is built; it is not laid over the
window's root. It is controlled: pass whether it is open, and close it when it
reports its dismiss ID. While closed, `render` returns the anchor alone.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Popover::new(
        Button::new("share", "Share", palette).render(),
        share_form(state, palette),
        state.share_open,
        "share:dismiss",
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "share" => state.share_open = true,
            "share:dismiss" => state.share_open = false,
            _ => {}
        }
    }
}
```

The anchor is any element; the popover does not open itself, so the anchor's
own action is what the host answers to open it. The content sits on a raised
surface with a border, a shadow and 6 px of padding.

## Placement

```rust
Popover::new(anchor, content, open, "dismiss", palette)
    .placement(PopoverPlacement::TopEnd)
    .render()
```

`BottomStart` unless set. The others are `BottomEnd`, `TopStart`, `TopEnd`,
`RightStart`, `LeftStart`, `RightCorner`, `Center` and `Bottom`. `RightCorner` is
a column beside an anchor's top-right corner: outside it when the right has room,
inside the corner when it has not. Anchored surfaces flip
vertically when the preferred side would leave the window. `Center` and
`Bottom` ignore the anchor and gap; `Bottom` meets the safe viewport's bottom
edge, centers horizontally and can use its full width.

## Width

```rust
Popover::new(anchor, content, open, "dismiss", palette).width(260.0).render()
```

240 unless set, and never under 96.

## A panel as content

`PopoverPanel` stacks a header, a body that scrolls and a footer that stays;
pass its rendered element as the content. See [Catalog menus](picker.md).

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves among the controls inside the popover. |
| Escape | Reports the dismiss ID and returns focus to the anchor. |

Inside a modal, an open child popover keeps its own keyboard scope. Escape
closes the child first and returns to its trigger; the next Escape can close
the modal. Tab and the menu arrow keys stay in the active child, including
nested submenus. Menus in the covered background cannot take that scope.

## Motion

The surface fades in and settles 4 px into place, and fades out; see
[Animation](../animation.md). `animation_ms` and `close_animation_ms` set the
two durations, `open_scale` the scale it starts from (0.5 to 1.25), and
`view_key` animates the content when a popover that stays open changes view.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the dismiss ID | Escape, a click outside the surface, or a wheel or trackpad scroll outside it, which still scrolls what is under the pointer. A modal is not closed by a scroll. |

## Accessibility

The first control of the anchor says whether what it opens is open. The
content says of itself what its own controls say.
