# ListRow

One entry of a list: an icon, a title, an optional second line and what sits
at its trailing edge, all under one hover surface.

## Import

```rust
use limn::{Component, DiffStat, Icon, ListRow, RadiusRole, RowSizing, TooltipSpec};
```

## Usage

A row needs an ID, a title and the palette. It reports `UiAction::Clicked`
with its ID; the host owns the selection and the action.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::column(
        state
            .files
            .iter()
            .map(|file| ListRow::new(format!("file:{}", file.name), &file.name, palette).icon(Icon::File).render())
            .collect(),
    )
    .gap(2.0)
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(name) = id.strip_prefix("file:") {
            state.open(name);
        }
    }
}
```

A row has no fill at rest. It hovers to the palette's `hover` and presses to
its `selected`, so a row on a card takes a palette whose `hover` is one step
above that card.

## Icon and leading element

```rust
ListRow::new("branch", "main", palette).icon(Icon::GitBranch).render()
ListRow::new("run", "Run the project", palette).leading(thumbnail).render()
```

`icon` is a muted glyph before the copy, centered in the row's height;
`icon_ink` paints it in another color. `leading` takes an element of the
host's, such as a thumbnail, held in a square at the top of the row. It
replaces the icon.

## Subtitle

```rust
ListRow::new("source", "list_row.rs", palette)
    .subtitle("crates/limn/src/list_row.rs")
    .subtitle_mono(true)
    .render()
```

`subtitle` adds a muted second line; an empty one is no line at all.
`subtitle_mono` sets it in the monospaced family, for a command or a path.
`subtitle_ink` paints it in a color that carries meaning, such as a warning.

## Meta

```rust
ListRow::new("source", "list_row.rs", palette).meta("2 min ago").render()
ListRow::new("changes", "12 files", palette)
    .meta_element(DiffStat::new(128, 17, palette).render())
    .render()
```

`meta` is muted text at the end of the title's line, such as a status or an
age. `meta_element` puts an element of the host's in the same place; each call
adds one more, after the meta text.

## Trailing

```rust
ListRow::new("branch", "main", palette).trailing_icon(Icon::ChevronDown).render()
ListRow::new("changes", "12 files", palette).trailing(stat).render()
```

`trailing` is an element after the copy, across the row's height.
`trailing_icon` is a muted glyph at the trailing edge, such as the chevron of
a row that opens a menu.

## Controls inside a row

```rust
ListRow::new("task", "Split the view", palette)
    .trailing(more_button)
    .hover_with("task-more")
    .render()

ListRow::new("change", "list_row.rs", palette)
    .trailing_action("change-stage", stage_control)
    .render()
```

`hover_with(id)` keeps the row hovered while the pointer is over the control
`id`, for a trailing element that takes the pointer from the row.

`trailing_action(id, control)` adds a control with a hit target of its own
after the row's; each call adds one more. The row and its actions then share
one hover surface, painted behind all of them, so draw the control with no
hover fill of its own. The row's hint covers the row's own target alone.

## Title

```rust
ListRow::new("task", state.title(), palette).title_fade(true).render()
ListRow::new("mail", "Unread", palette).title_ink(accent).render()
```

A title that does not fit is cut with an ellipsis. `title_fade(true)` fades
its end instead, for a title that changes while it is on screen. `title_ink`
paints it in a color the host owns; it holds while the row is disabled.

## Fill, outline and radius

```rust
ListRow::new("account", "Account", palette).filled().outlined().render()
ListRow::new("task", "Sidebar rows", palette).radius(RadiusRole::Surface).render()
```

`filled` fills the row with the palette's `surface` at rest, for a row that
stands on its own. `outlined` draws a hairline in the palette's border color,
for a row that is a card of its own. `radius` takes the role of the corners of
the fill and the hover; `RadiusRole::Control` unless set.

## Width

```rust
ListRow::new("source", "list_row.rs", palette).width(320.0).render()
ListRow::new("budget", "A long title", palette).copy_width(160.0).render()
```

Without `width` the row takes the width it is given. `width` fixes it and
clips the content to it. `copy_width` holds the title and subtitle at a fixed
width instead of the width the row has left.

## Sizing

```rust
ListRow::new("frame", "render", palette).sizing(RowSizing::DENSE).render()
```

`RowSizing::COMPACT`, the default, is 28 high, or 40 with a subtitle.
`RowSizing::DENSE` is 24 and 36, for a long list of short entries. `RowSizing`
is a struct of the heights, padding, gaps, column widths and font sizes, so a
host can write a profile of its own; a `None` height follows the content.

## Disabled

```rust
ListRow::new("locked", "Locked while saving", palette).enabled(false).render()
```

A disabled row is passive: a muted title and no hit region.

## Hint

```rust
ListRow::new("branch", "main", palette)
    .tooltip_with(TooltipSpec::new("Switch branch"))
    .render()
```

The hint is shown over the row, enabled or not.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the row's ID | An enabled row is activated. |
| `UiAction::Clicked { id }` with a trailing action's ID | The control given to `trailing_action` is activated, if the host built it as a click target with that ID. |

## Accessibility

A row is an item of a list. One that can be clicked is named by everything it
shows; a disabled one is named by its title, with its subtitle as a
description, and says it cannot be used. The row keeps no selection of its
own, so the host says which one is chosen:

```rust
ListRow::new("inbox", "Inbox", palette).render().accessible(Access::default().selected(true))
```
