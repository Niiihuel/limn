# Clipboard

A button that copies a link, a key or a path. Its icon turns into a check
while the host says the copy was just made.

## Import

```rust
use limn::{Clipboard, ClipboardCopied, Component};
```

## Usage

The host owns the clipboard and the state. On the click it copies the value
and keeps a `ClipboardCopied`; each frame it passes whether that mark still
shows to `copied`.

```rust
fn view(state: &State, palette: ComponentPalette, now: Instant) -> Element {
    let copied = state
        .copied
        .as_ref()
        .is_some_and(|mark| mark.shows("copy-link", now));
    Clipboard::new("copy-link", "Copy link", palette)
        .copied(copied)
        .render()
}

fn update(state: &mut State, action: &UiAction, now: Instant) {
    if let UiAction::Clicked { id } = action {
        if id == "copy-link" {
            state.copy_link();
            state.copied = Some(ClipboardCopied::new("copy-link", now));
        }
    }
}
```

The second argument says what is copied; it is the button's hint. Without
anything else the button is a ghost icon button with the copy icon in muted
ink, which turns to the text color under the pointer and while copied.

## The check

`ClipboardCopied::new(id, now)` marks the copy. `shows(id, now)` is true for
1.5 s (`ClipboardCopied::SHOWN_FOR`), and `repaint_after(now)` gives the time
left until the mark ends, or `None` once it has, when the host drops it: ask
for one repaint then. The check takes the icon's place, so the button does
not move.

## Label

```rust
Clipboard::new("copy-link", "Copy link", palette)
    .label(if copied { "Link copied" } else { "Copy link" })
    .copied(copied)
    .render()
```

`label` adds text after the icon, for a button that says what it copies.

## Icon

```rust
Clipboard::new("copy-branch", "Copy branch name", palette)
    .icon(Icon::GitBranch)
    .render()
```

`icon` changes the icon at rest.

## Variant and size

```rust
Clipboard::new("copy-key", "Copy key", palette)
    .variant(ButtonVariant::Outline)
    .size(ControlSize::Dense)
    .render()
```

`variant` and `size` are those of a [Button](button.md). The defaults are
`ButtonVariant::Ghost` and `ControlSize::Compact`. The muted ink belongs to
the ghost variant; another variant keeps its own.

## Disabled

```rust
Clipboard::new("copy-link", "Copy link", palette).enabled(false).render()
```

A disabled one keeps its place and its hint, with a muted icon and label. It
takes no pointer or focus and reports nothing.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | The button is activated by pointer, Enter or Space. |
