# Button

A labelled action. Eleven variants and seven sizes, optional icons before and
after the label, a hint, and disabled, loading and active states.

## Import

```rust
use limn::{Button, ButtonSize, ButtonVariant, Component, Icon};
```

`Component` gives every component its `render`.

## Usage

A button needs an ID, a label and the palette. It reports
`UiAction::Clicked` with its ID.

```rust
fn view(palette: ComponentPalette) -> Element {
    Button::new("save", "Save changes", palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "save" {
            state.save();
        }
    }
}
```

## Variants

`Default` is the primary action. `Secondary`, `Outline`, `Ghost`, `Subtle`,
`Raised` and `Solid` are quieter; `Success`, `Warning` and `Destructive` say
what the action does; `Link` is a text action.

```rust
Button::new("delete", "Delete", palette)
    .variant(ButtonVariant::Destructive)
    .render()
```

`Subtle` is the navigation row and keeps its label at the left; the filled
variants center it.

## Sizes

```rust
Button::new("more", "More", palette)
    .button_size(ButtonSize::Small)
    .render()
```

`Default`, `Small` and `Large` are for labelled buttons; `Icon`, `IconSmall`
and `IconExtraSmall` are square; `Mini` is a 22 px action inside a dense row.
A host with a density of its own passes a `ButtonSizing` to `sizing` instead.

## Icons

```rust
Button::new("open", "Open", palette)
    .icon(Icon::Folder)
    .trailing_icon(Icon::Search)
    .render()
```

`IconButton` is the button whose icon is mandatory; without a label it is
square.

## Loading and disabled

```rust
Button::new("install", "Installing…", palette).loading(true).render()
Button::new("install", "Install", palette).enabled(can_install).render()
```

A loading button replaces its leading icon with a spinner and keeps its
height and label. Neither a loading nor a disabled button can be focused or
activated; a disabled one keeps its hint.

## Active

```rust
Button::new("view=list", "List", palette)
    .variant(ButtonVariant::Ghost)
    .active(view == "list")
    .render()
```

`active` marks the chosen one of several. `active_hover` chooses how it
hovers: `ActiveHover::Lift` lightens the selected fill.

## Hint

```rust
Button::new("search", "Search", palette)
    .tooltip("Search the project")
    .tooltip_shortcut("Ctrl+K")
    .render()
```

`tooltip_with` takes a whole `TooltipSpec`: description, placement, width.

## Ink, gap and radius

`ink`, `hover_ink` and `pressed_ink` draw the label and icons in a color of
the host's, such as its destructive ink. `gap` sets the space between icons
and label, `radius` takes another radius token, and `round` makes a pill or
a circle.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The button is activated by pointer, Enter or Space. |

`dispatch_as(action_id)` reports another ID while the button keeps its own
for focus, hover and its hint.
