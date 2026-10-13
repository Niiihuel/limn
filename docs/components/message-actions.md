# MessageActions

The quiet row of icon actions under a chat message: copy, retry, edit. Ghost
icon buttons 24 px high and 2 apart in muted ink, with muted captions and an
overflow menu for what is used less.

## Import

```rust
use limn::{Component, Icon, MessageAction, MessageActions, MessageActionsVisibility};
```

## Usage

A row needs an ID that is unique in the window and the palette. Items stand
in the order they are added. Each action reports `UiAction::Clicked` with
its own ID; its title is its hint, the only name an icon has.

```rust
fn view(palette: ComponentPalette) -> Element {
    MessageActions::new("message-7", palette)
        .action(MessageAction::new("message-7-copy", Icon::Copy, "Copy"))
        .action(MessageAction::new("message-7-retry", Icon::Refresh, "Retry"))
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "message-7-copy" {
            state.copy_message(7);
        }
    }
}
```

## Captions

```rust
MessageActions::new("message-7", palette)
    .action(MessageAction::new("message-7-copy", Icon::Copy, "Copy"))
    .caption("5 min ago · 14 s")
    .render()
```

`caption` adds passive muted text: a relative time, a duration, a model
name. Empty text is ignored. A caption is cut with an ellipsis when the row
is too narrow; the buttons keep their size.

## Visibility

```rust
Element::column(vec![
    message,
    MessageActions::new("message-3", palette)
        .action(MessageAction::new("message-3-copy", Icon::Copy, "Copy"))
        .visibility(MessageActionsVisibility::Hover)
        .render(),
])
.hover_region()
```

| Visibility | Painted |
| --- | --- |
| `Always` | Always: the last message of a conversation. The default. |
| `Hover` | While the pointer is over the message, while one of the row's controls has keyboard focus, or while its menu is open. |
| `Hidden` | Never. Room only, with no controls: a message that is still being written. |

The row takes the same room in all three, so a message never changes height.
For `Hover`, the message is the nearest `Element::hover_region()` around the
row.

## Done

```rust
MessageAction::new("message-7-copy", Icon::Copy, "Copy")
    .done(mark.shows("message-7-copy", now))
```

`done(true)` shows a check in place of the icon: the action was just taken.
The host decides for how long. `MessageActionDone::new(id, now)` keeps that
time: `shows(id, now)` is true for 1.5 s (`MessageActionDone::SHOWN_FOR`),
and `repaint_after(now)` gives the time left, or `None` once the mark has
ended.

## Toggles

```rust
MessageAction::new("message-7-pin", Icon::Pin, "Pin to context").active(state.pinned)
```

`active(true)` fills the button: a toggle that is on.

## Disabled

```rust
MessageAction::new("message-2-edit", Icon::Edit, "Edit")
    .disabled("Wait for the agent to finish.")
```

A disabled action takes no pointer or focus, and its hint gives the reason
under its title.

## Hints

```rust
MessageAction::new("message-2-edit", Icon::Edit, "Edit").shortcut("Enter")
```

`shortcut` shows the keys beside the title. `tooltip_with` replaces the hint
with a whole `TooltipSpec`, for another placement or palette.

## Overflow

```rust
MessageActions::new("message-7", palette)
    .action(MessageAction::new("message-7-copy", Icon::Copy, "Copy"))
    .overflow(
        vec![
            DropdownMenuItem::new("message-7-plain", "Copy as plain text").into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("message-7-activity", "Show activity").into(),
        ],
        state.more_open,
        "More actions",
    )
    .render()
```

`overflow(entries, open, title)` adds a "···" button that shows `entries` as
a menu while `open`. The host owns `open`: the trigger reports
`{id}:more`, and a click outside the menu reports `{id}:more:dismiss`, where
`{id}` is the row's ID. Without entries nothing is added.

## Aligned to the end

```rust
MessageActions::new("message-2", palette)
    .caption("14:32")
    .action(MessageAction::new("message-2-copy", Icon::Copy, "Copy"))
    .align_end()
    .render()
```

`align_end` pushes the row to the far edge, for a user's message. The
default is the near edge, where an assistant's text starts.

For a branch position such as "2 / 3", put a `Pager` beside the row.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves through the enabled actions and the overflow trigger in order. It reaches a `Hover` row that is not painted, and the row stays shown while one of its controls has the focus. |

## Motion

A `Hover` row fades in over `enter_ms` and out over `exit_ms`; with reduced
motion it switches at once. Only the paint changes: nothing moves. See
[Tokens](tokens.md).

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an action's ID | That action is activated. With `dispatch_as(action_id)` it reports `action_id` instead, while its own ID stays the focus stop. |
| `UiAction::Clicked { id }` with `{id}:more` | The overflow trigger is activated. |
| `UiAction::Clicked { id }` with `{id}:more:dismiss` | A click lands outside the open menu. |
| `UiAction::Clicked { id }` with a menu item's ID | That item of the overflow menu is chosen. |
