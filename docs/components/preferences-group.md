# PreferencesGroup

A card of settings rows separated by hairlines, under a small muted label.
Its rows are `ActionRow`: a title, a subtitle, an icon and a control at the
end.

## Import

```rust
use limn::{ActionRow, ActionRowSizing, Component, PreferencesGroup};
```

## Usage

A group takes a title, its rows as elements and the palette. The group is
passive; a control in a row reports its own action.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    PreferencesGroup::new(
        "Plugins",
        vec![
            ActionRow::new("agent-skills", palette)
                .subtitle("25 skills")
                .trailing(Switch::new("agent-skills", state.agent_skills, palette).render())
                .render(),
            ActionRow::new("Installed skills", palette)
                .value("0 active")
                .render(),
        ],
        palette,
    )
    .description("Instructions installed for the agent.")
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "agent-skills" {
            state.agent_skills = !state.agent_skills;
        }
    }
}
```

`description` adds a second muted line under the title. With an empty title
and no description the group is just its card.

## Rows

`ActionRow::new(title, palette)` is one row. The title is cut to one line.

```rust
ActionRow::new("Account", palette)
    .icon(Icon::Users)
    .subtitle("Session, plan and billing.")
    .render()
```

`icon` puts an icon before the copy, and `icon_muted(true)` paints it in the
muted color, for a note rather than a setting. `subtitle` is supporting copy
under the title, in the muted color; it wraps.

## Trailing and value

```rust
ActionRow::new("Explore", palette)
    .trailing(Button::new("marketplace", "Open Marketplace", palette).render())
    .render()

ActionRow::new("Context", palette).value("82% left").render()
```

`trailing` puts an element the host owns at the end of the row, centered on
it: a [Switch](switch.md), a [Button](button.md), a chevron. `value` is a
read-only value there, as plain muted text so it is not mistaken for a
control; it replaces any trailing element.

## A row that is an action

```rust
ActionRow::new("Keyboard shortcuts", palette)
    .icon(Icon::Keyboard)
    .trailing(Element::icon(Icon::ChevronRight).foreground(palette.muted))
    .action("open-shortcuts")
    .render()
```

`action(id)` makes the whole row a button that reports `UiAction::Clicked`
with that ID.

## Long copy and narrow panes

```rust
ActionRow::new("Working folder of background agents", palette)
    .subtitle("Where an agent started in the background runs.")
    .wrap_title(true)
    .align_top(true)
    .trailing(Button::new("folder", "Choose folder", palette).render())
    .stacked(true)
    .render()
```

- `wrap_title(true)` wraps a long title over several lines instead of
  cutting it.
- `align_top(true)` starts the icon and the copy at the top of the row
  instead of centering them. The trailing element stays centered.
- `trailing_top(true)` keeps the trailing element on the row's first line
  instead of centering it, for a keycap beside copy that may wrap.
- `stacked(true)` places the trailing element under the copy, for a narrow
  pane. A trailing element with a fixed width of 64 or less, such as a
  switch, stays beside the copy.

## Sizing

```rust
const NOTE: ActionRowSizing = ActionRowSizing {
    icon_column: 16.0,
    min_height: 0.0,
    min_height_with_subtitle: 0.0,
    ..ActionRowSizing::DEFAULT
};

ActionRow::new("Only CLIs started from the app", palette).sizing(NOTE).render()
```

An `ActionRowSizing` holds a row's density in logical pixels. The row's
padding is not part of it: every row in a card shares one inset.

| Field | Sets | `DEFAULT` |
| --- | --- | --- |
| `gap` | Space between the icon, the copy and the trailing element | 10 |
| `copy_gap` | Space between the title and the subtitle | 4 |
| `icon_column` | Width of the column the icon sits in | 20 |
| `min_height` | Least height of a row with a title only | 36 |
| `min_height_with_subtitle` | Least height of a row with a subtitle | 54 |

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A row with `action(id)` is clicked. |
