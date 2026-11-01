# DropdownList

A floating list for choosing one option, with the current one marked. For a
list of actions use a [DropdownMenu](dropdown-menu.md).

## Import

```rust
use limn::{Component, DropdownList, DropdownOption};
```

## Usage

The list wraps its anchor where the anchor is built. It is controlled: the
host owns which option is selected and whether the list is open. An option's
ID is the action it reports; the list does not close itself, so the host
closes it when it handles that action or the dismiss ID.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    DropdownList::new(
        Button::new("language", state.language.label(), palette)
            .trailing_icon(Icon::ChevronDown)
            .render(),
        vec![
            DropdownOption::new("language=rust", "Rust").selected(state.language == Language::Rust),
            DropdownOption::new("language=python", "Python")
                .selected(state.language == Language::Python),
        ],
        state.language_open,
        "language:dismiss",
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "language" => state.language_open = true,
            "language:dismiss" => state.language_open = false,
            "language=rust" => state.choose(Language::Rust),
            "language=python" => state.choose(Language::Python),
            _ => {}
        }
    }
}
```

Here `choose` stores the language and sets `language_open` to false. While
closed, `render` returns the anchor alone.

## Options

```rust
DropdownOption::new("language=rust", "Rust").icon(Icon::Code).selected(true)
DropdownOption::new("language=cobol", "COBOL").disabled()
```

`icon` puts a glyph before the label, `selected` marks the current option and
`disabled` leaves the option out of the pointer's and the keyboard's reach.
`tooltip(text)` gives the option a hint of its own, for what its label cannot
say.

## Width and placement

```rust
DropdownList::new(anchor, options, open, "dismiss", palette)
    .width(200.0)
    .placement(PopoverPlacement::BottomEnd)
    .render()
```

240 wide unless set, never under 96, and under the start of the anchor
(`BottomStart`). `sizing(SelectorSizing::COMPACT)` takes the width, row
density and scroll budget other selectors share; call it before `width` to
override the width.

## Keyboard

| Key | Effect |
| --- | --- |
| Up, Down, Home, End, Tab | Move through the enabled options and bring a row out of view into view. |
| Enter | Activates the focused option. |
| Escape | Reports the dismiss ID. |

## Motion

The list fades in and settles into place, and fades out; see
[Animation](../animation.md). `motion(MenuMotion { open_ms, item_ms })` sets
how long the surface and a row's feedback take.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an option's ID | The option is activated. |
| `UiAction::Clicked { id }` with the dismiss ID | Escape, or a click outside the list. |

## Accessibility

Each option is told to the platform as a menu item named by its label, and
says whether it is the selected one. A disabled option is still there and
says it cannot be used.
