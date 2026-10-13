# SuggestionList

Completions floating over a text input while the user keeps typing: commands
after a slash, mentions, file names. Focus stays in the input.

## Import

```rust
use limn::{Component, SuggestionItem, SuggestionList};
```

## Usage

The list wraps the input it completes, where the input is built. It is
controlled, and it does not handle keys: the host moves `selected_index` on
the arrow keys, accepts on Enter and closes on Escape. The list shows that
state and reports clicks.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let items = state
        .matches
        .iter()
        .map(|command| {
            SuggestionItem::new(format!("complete={}", command.name), &command.name)
                .icon(command.icon)
                .description(&command.summary)
        })
        .collect();
    SuggestionList::new(
        TextField::new("prompt", &state.prompt, "Type / for commands", palette).render(),
        "prompt:options",
        items,
        state.selected,
        state.completing,
        "prompt:dismiss",
        palette,
    )
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "prompt:dismiss" {
            state.completing = false;
        } else if let Some(name) = id.strip_prefix("complete=") {
            state.accept(name);
        }
    }
}
```

The arguments are the anchor, an ID for the scrolling options, the items, the
index of the selected one, whether the list is open, the dismiss ID and the
palette. While closed, `render` returns the anchor alone.

## Items

```rust
SuggestionItem::new("complete=/plan", "/plan")
    .icon(Icon::Plan)
    .description("Describe the steps before editing")
    .section("Commands")
```

An item is a label over a description, after an optional icon. `section`
starts a section at that item, with a heading on a row of its own above it:
set it on the first item of each group.

## Title, hints and note

```rust
SuggestionList::new(anchor, "options", items, selected, open, "dismiss", palette)
    .title("Commands and templates")
    .hints(vec![Kbd::new("Enter", palette).caption("Accept").render()])
    .note("Results limited")
    .render()
```

`title` is a muted line above the options, `hints` are elements under them
that share a row and stack when the list is narrow, and `note` is a muted
last line.

## Empty

```rust
SuggestionList::new(anchor, "options", Vec::new(), 0, open, "dismiss", palette)
    .empty("No command matches.")
    .render()
```

`empty` is what the list says in place of the options when there are none.
The title, hints and note stay.

## Width and placement

`width(available)` is the room the host can offer; the list is as wide as
that allows, between 160 and 352. `placement` takes a `PopoverPlacement`; the
list opens above the start of its anchor (`TopStart`) unless set. Rows are 42
high and at most seven show; the rest scroll, and only the rows in view are
built.

## Motion

The list appears and leaves with the motion tokens' `enter_ms` and `exit_ms`.
`animation_ms(open, close)` sets both, for a host that wants it quicker.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's ID | The item is clicked. |
| `UiAction::Clicked { id }` with the dismiss ID | The pointer is pressed outside the list. |
