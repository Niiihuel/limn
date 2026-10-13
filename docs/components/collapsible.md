# Collapsible

A header that shows or hides content beneath it. Use it for detail that is
not always needed: a list of changed files, the steps of a tool call.

## Import

```rust
use limn::{Collapsible, Component, Icon};
```

## Usage

A collapsible is controlled: pass whether it is expanded each frame and flip
your own value when the header reports a click. It takes the toggle's action
ID, a title, the content and the palette.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Collapsible::new("changed-files", "Changed files", file_list(state), palette)
        .expanded(state.files_open)
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "changed-files" {
            state.files_open = !state.files_open;
        }
    }
}
```

The content is present only while the collapsible is expanded. While open,
the header and the content read as one filled block. Keep the ID stable
across content updates so the focus stays on the header.

## Icon and detail

```rust
Collapsible::new("changed-files", "Changed files", content, palette)
    .icon(Icon::Folder)
    .detail("3 files")
    .expanded(state.files_open)
    .render()
```

`icon` leads the header. `detail` is short metadata after the title. Both
the title and the detail truncate on a narrow surface.

## Loading and thinking

```rust
Collapsible::new("search", "Searching", content, palette).loading(true).render()
Collapsible::new("reasoning", "Thinking", content, palette).thinking(true).render()
```

`loading(true)` replaces the leading icon with a spinner; the leading slot
keeps its width. `thinking(true)` shows a brain icon and sweeps a highlight
across the title, and takes precedence over `loading`. Both are visual
states only: the collapsible never infers completion.

## Ink

```rust
Collapsible::new("exploration", "Exploration", content, palette)
    .detail("2 failed")
    .ink(danger)
    .render()
```

`ink` is a status color from the host, such as its failure ink. It paints
the icon, the detail, the chevron and the footer in place of the palette's
muted. The title keeps the palette's text color.

## Footer

```rust
Collapsible::new("log", "Output", long_content, palette)
    .collapse_footer(true)
    .expanded(state.log_open)
    .render()
```

`collapse_footer(true)` repeats the disclosure at the end of long content,
as a button that reports the header's ID. The free function
`collapse_footer(id, palette)` builds the same footer for a disclosure of
the host's own.

## Disabled

```rust
Collapsible::new("log", "Output", content, palette).expanded(true).enabled(false).render()
```

A disabled collapsible stays as `expanded` says: its header, with a muted
title, and its footer take no pointer or focus and report nothing. The
content is the host's and works as before.

## Motion

The content opens and closes in place instead of appearing at once. Under
reduced motion the title and the icon of `thinking` stay static.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The header, or the footer's button, is activated by pointer or keyboard. |

## Accessibility

The header is a button named by the title and described by the `detail`, and
says whether it is expanded. The footer's button is named by
`labels().collapse`. Disabled, both are still there for the platform, as
disabled.
