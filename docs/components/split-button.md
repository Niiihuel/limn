# SplitButton

A primary action joined to a menu of the others: the action at the start and
a chevron at the end, drawn as one button. Use it when one choice is the
usual one and the rest are near it, such as "Always allow" with "Allow once"
in its menu.

## Import

```rust
use limn::{Component, DropdownMenuItem, SplitButton};
```

## Usage

The host owns whether the menu is open. The action reports its ID, the
chevron reports the menu's ID, and the menu reports its items and its
dismiss ID.

```rust
fn view(palette: ComponentPalette, state: &State) -> Element {
    SplitButton::new("allow", "Always allow", "allow:menu", palette)
        .menu(
            vec![DropdownMenuItem::new("allow-once", "Allow once").shortcut("Ctrl+Enter").into()],
            state.allow_menu_open,
            "allow:dismiss",
        )
        .render()
}

match action {
    UiAction::Clicked { id } if id == "allow" => state.allow_always(),
    UiAction::Clicked { id } if id == "allow:menu" => state.allow_menu_open = !state.allow_menu_open,
    UiAction::Clicked { id } if id == "allow-once" => {
        state.allow_menu_open = false;
        state.allow_once();
    }
    UiAction::Clicked { id } if id == "allow:dismiss" => state.allow_menu_open = false,
    _ => {}
}
```

The menu opens below the button, aligned to its end, and is a
[DropdownMenu](dropdown-menu.md) in every other way.

## Keycaps

```rust
SplitButton::new("allow", "Always allow", "allow:menu", palette)
    .keys(&["Ctrl", "Shift", "Enter"])
    .render()
```

`keys` draws the action's shortcut after its label, one key to a cap, in the
button's ink. The platform is told the shortcut as the action's description.
The keys only show the shortcut: the host still handles it.

## Variants

```rust
SplitButton::new("deploy", "Deploy", "deploy:menu", palette)
    .variant(ButtonVariant::Outline)
    .render()
```

Both halves take a [Button](button.md)'s variant: its fill, its ink, its
hover and, for `Outline`, its border. They are 32 px high, the chevron half
square, with 1 px between them where the surface shows through.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves to the action, then to the chevron. |
| Enter, Space | Activates the half with the focus. |

The open menu takes the arrow keys, Home, End and Escape, as a
[DropdownMenu](dropdown-menu.md#keyboard) does.

## Actions

| Action | When |
| --- | --- |
| `Clicked { id }` | The action is activated. |
| `Clicked { id: menu_id }` | The chevron is activated. |
| `Clicked { id: dismiss_id }` | The open menu is dismissed. |
| `Clicked { id }` of an item | An item of the menu is chosen. |

## Limits

There is no disabled state and no size other than 32 px. A long label is
cut short with an ellipsis before the chevron is pushed out.
