# ContextWindow

What fills a model's context window: the figure in use, a field of cells
shared out among the shares, one row for each share with its count, and a
foot with a status line and an action.

## Import

```rust
use limn::{
    Component, ContextItem, ContextShare, ContextShareAction, ContextTones, ContextWindow,
};
```

## Usage

The host owns every figure and passes them on each rebuild. A card needs an
ID, a title, its shares, the capacity of the window in tokens, the palette
and the colors that carry meaning.

```rust
fn view(state: &State, palette: ComponentPalette, tones: ContextTones) -> Element {
    let shares = vec![
        ContextShare::new("system", "System", 4_200, palette.text),
        ContextShare::new("tools", "Tools", 12_800, palette.muted),
        ContextShare::new("chat", "Conversation", state.chat_tokens, state.accent),
    ];
    ContextWindow::new("context", "Context window", shares, 200_000, palette, tones)
        .caption("of 200K tokens")
        .status(&state.context_status)
        .render()
}
```

Each `ContextShare` has an ID, a label, a count of tokens and a color. The
card draws its own card on a frame a shade apart, with the foot on the frame,
so give it a surface with rounded corners and no padding. It is 320 px wide
unless `width` sets it, and never narrower than 200 px.

A capacity of zero is a window of unknown size: the shares fill the cells
between them and no percentage is shown.

## Tones

```rust
let tone = |tone| AlertColors::for_tone(tone, palette).accent;
let tones = ContextTones {
    warning: tone(AlertTone::Warning),
    danger: tone(AlertTone::Error),
    success: tone(AlertTone::Success),
};
```

The percentage is drawn in `warning` from 80% of the window and in `danger`
from 95%. `success` colors a share's finished action and a status set with
`status_done`.

## Shares that open

```rust
fn view(state: &State, palette: ComponentPalette, tones: ContextTones) -> Element {
    let files = state.files.iter().map(|file| ContextItem {
        id: file.id.clone(),
        label: file.name.clone(),
        tokens: file.tokens,
    });
    let shares = vec![
        ContextShare::new("files", "Files", state.file_tokens(), palette.muted).items(files.collect()),
    ];
    ContextWindow::new("context", "Context window", shares, 200_000, palette, tones)
        .expanded(state.files_open.then_some("files"))
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "context:share:files" {
            state.files_open = !state.files_open;
        } else if let Some(file) = id.strip_prefix("context:remove:files:") {
            state.remove_file(file);
        }
    }
}
```

A share with `items` shows a chevron and opens to list them, each with its
count and a button that removes it. The host decides which shares are open:
`expanded` takes their IDs, and a click on a share's row reports
`"{id}:share:{share}"` for the host to answer.

## A share's action

```rust
ContextShare::new("chat", "Conversation", state.chat_tokens, accent).action(match state.compaction {
    Compaction::Idle => ContextShareAction::Offer("Compact".into()),
    Compaction::Running => ContextShareAction::Busy("Compacting".into()),
    Compaction::Done => ContextShareAction::Done("Compacted".into()),
})
```

`Offer` is an action in the share's row that reports `"{id}:action:{share}"`.
`Busy` shows its label beside a spinner while the host carries it out, and
`Done` shows it with a check in the `success` color. Neither can be clicked.

## Focus

```rust
let focused = tree
    .hovered_id()
    .and_then(|hovered| ContextWindow::share_under("context", hovered));

ContextWindow::new("context", "Context window", shares, 200_000, palette, tones)
    .caption("of 200K tokens")
    .focused(focused)
    .render()
```

`focused` makes one share stand out: the figure becomes its count, the
caption its label, and the other shares recede. `share_under` gives the share
whose row or action a control ID belongs to, so the host passes the ID under
the pointer.

## Unmeasured shares

```rust
ContextShare::new("memory", "Memory", 0, palette.muted).unmeasured()
```

A share the host could not measure is listed with a dash where its count
would be, also in the figure while it is in focus, and takes no cells.

## Header and foot

```rust
ContextWindow::new("context", "Context window", shares, 200_000, palette, tones)
    .badge("model-name")
    .header_icon("context-copy", Icon::Copy, "Copy diagnostics")
    .status_lead("Files")
    .status("Files · 23% of the window")
    .footer_action("context-new", "New chat")
    .render()
```

`badge` is a short name beside the title. `header_icon` adds a small action
at the end of the header, with its hint; call it again for another.
`status` is the line at the foot and `status_done` the same in the `success`
color. `status_lead` names the words the status opens with, which read
stronger. `footer_action` is an action opposite the status.

## Format

`format` takes a `fn(f32) -> String` that writes a count of tokens, also
while it counts. The default is `ContextWindow::tokens_text`: "950", "4.2K",
"175.6K", "1.25M".

## Inside a popover

`flat` is for a host whose surface already is the card, such as a popover:
the content stands on that surface, with no frame and no card around it.
`parts` then returns the body and the foot apart, for a host that scrolls one
and pins the other under it.

## Motion

When a figure changes, the counts roll to the new value over three times
`move_ms`, and the cells of a share that shrinks empty from its end while the
shares after it follow. The caption and the status change as a
[Ticker](ticker.md) line does, over `move_ms`, and the percentage fades to
the color of its new state. An item that is removed closes where it was, as a
[Collapse](collapse.md) does. With reduced motion the new state is shown at
once.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id: "{id}:share:{share}" }` | A share's row is clicked. The host opens or closes it. |
| `UiAction::Clicked { id: "{id}:action:{share}" }` | A share's `Offer` action is clicked. |
| `UiAction::Clicked { id: "{id}:remove:{share}:{item}" }` | An item's remove button is clicked. |
| `UiAction::Clicked { id }` | A `header_icon` or the `footer_action` is clicked, with the ID given to it. |
