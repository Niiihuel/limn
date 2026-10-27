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

```rust
SuggestionItem::new("mention=@ada", "Ada Lovelace")
    .leading(avatar)
    .ink(role_color)
    .detail("ada")
```

`leading` puts an element of the host's in the icon's place, such as a
person's avatar 24 across, centered at its own size. `ink` colors the label,
such as in a role's color; a disabled item stays muted. `detail` adds a
quiet text at the end of the label's line, such as a handle, and the label is
cut before it. An item without a description shows its line in the middle
of the row. The platform hears the detail after the description.

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

## Matched text

```rust
SuggestionItem::new("complete=/plan", "/plan").matched(vec![1..3])
```

`matched` takes byte ranges of the label to draw in the bold sans, such as
what the query matched. A range out of bounds or not on a character boundary
is ignored.

## Compact

```rust
SuggestionList::new(anchor, "options", items, selected, open, "dismiss", palette)
    .compact(true)
    .render()
```

A compact list is for a command or mention palette: one-line rows, 30 high,
with the icon, the label and the `detail` at its end. Its items' descriptions
are not drawn in the rows. The selected item's description shows in a
tooltip-styled box beside the list, top-aligned with its row and 8 px from it,
at most 320 wide and ten lines high. The box is not interactive and moves
with `selected_index`. It is not drawn for an item without a description.

## Width and placement

`width(available)` is the room the host can offer; the list is as wide as
that allows, between 160 and `max_width` (352 unless set). `max_width(px)`
raises or lowers that cap. `placement` takes a `PopoverPlacement`; the list
opens above the start of its anchor (`TopStart`) unless set. Rows are 42 high
(30 when compact) and at most seven show; the rest scroll, and only the rows
in view are built.

## Disabled

```rust
SuggestionItem::new("complete=/deploy", "/deploy").description("No remote configured").disabled()
```

A disabled suggestion keeps its row with icon and label in muted ink and
reports no click. It does not take the highlight, so the host's
`selected_index` should step over it.

## Motion

The list appears and leaves with the motion tokens' `enter_ms` and `exit_ms`.
`animation_ms(open, close)` sets both, for a host that wants it quicker.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with an item's ID | The item is clicked. |
| `UiAction::Clicked { id }` with the dismiss ID | The pointer is pressed outside the list. |

## Accessibility

A suggestion is an item of a list named by its label, with its description
said after it. It says whether it is the selected one, the row under the
host's highlight, and a disabled suggestion is there and cannot be used.
