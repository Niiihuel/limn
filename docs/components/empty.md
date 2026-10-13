# Empty

The state of a page, pane or result list with nothing in it: an icon, what is
missing, why, and the next step. It is centered and has no frame of its own,
so it sits on whatever surface it is given.

## Import

```rust
use limn::{ButtonVariant, Component, Empty, Icon};
```

## Usage

An empty state needs a title, a description and the palette. An action is a
button that reports `UiAction::Clicked` with its ID.

```rust
fn view(palette: ComponentPalette) -> Element {
    Empty::new("No projects", "Create a project or import an existing one.", palette)
        .icon(Icon::Folder)
        .action("new-project", "Create project", ButtonVariant::Default)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "new-project" {
            state.create_project();
        }
    }
}
```

An empty description is left out, so `Empty::new("Nothing here", "", palette)`
is a title alone.

## Actions in a row

```rust
Empty::new("No projects", "Create a project or import an existing one.", palette)
    .action("new-project", "Create project", ButtonVariant::Default)
    .action("import", "Import", ButtonVariant::Outline)
    .render()
```

Each `action` adds a small [Button](button.md) to a centered row, in the order
given. `action_element` adds a control of your own after them, for an action
that is more than a button.

## Compact

```rust
Empty::new("No results", "Try another term.", palette)
    .icon(Icon::Search)
    .compact()
    .render()
```

`compact` is for a narrow pane or a filtered list: a smaller icon and type,
less padding, and a minimum height of 168 px in place of 208.

## Framed

```rust
Empty::new("No plugins yet", "Choose a plugin folder to install it.", palette)
    .compact()
    .framed()
    .render()
```

`framed` sets it in a card of its own: the palette's surface, a hairline
border and the card radius. Use it on a page of cards, where an empty group
should still read as one.

## Size

`width` fixes the width; without it the state takes the width it is given.
`height` sets a minimum height, never below the 208 or 168 px of its density.
The description wraps, and wrapped copy or actions make the state taller.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | One of its `action` buttons is activated, with the ID given to that action. |
