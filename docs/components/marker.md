# Marker

A one-line event in a conversation or a log: a branch created, a tool
running, the model thinking. The host supplies the label and the state; the
marker infers nothing.

## Import

```rust
use limn::{Component, Icon, Marker, MarkerVariant};
```

## Usage

A marker needs an ID, a label and the palette. It is drawn in the palette's
muted color and is passive unless it is given an action.

```rust
fn view(palette: ComponentPalette) -> Element {
    Marker::new("branch-created", "Branch created", palette)
        .icon(Icon::GitBranch)
        .render()
}
```

## Loading

```rust
Marker::new("tests", "Running tests", palette).loading(state.running).render()
```

A loading marker shows a [Spinner](spinner.md) in place of its icon and a
highlight sweeps across its label. `shimmer(true)` is the highlight alone,
with the icon kept.

## Thinking

```rust
Marker::new("thinking", "Thinking", palette).thinking(true).render()
```

`thinking` shows a still brain icon and a quieter highlight across the label.
It takes precedence over `loading` and over `icon`.

## Variants

```rust
Marker::new("today", "Today", palette)
    .variant(MarkerVariant::Separator)
    .render()
```

`Default` is the row alone. `Separator` centers the label between two rules
that take the width it leaves. `Border` draws one rule under the row.

## Truncate

```rust
Marker::new("tool", &state.step, palette).truncate(true).render()
```

A label wraps by default. `truncate(true)` keeps it on one line and cuts it
with an ellipsis.

## Ink

```rust
Marker::new("request", "Request failed", palette).ink(danger).render()
```

`ink` paints the label, the icon and the spinner in a status color of the
host's, in place of the palette's muted.

## As an action

```rust
fn view(palette: ComponentPalette) -> Element {
    Marker::new("result", "See result", palette).action("open-result").render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "open-result" {
            state.open_result();
        }
    }
}
```

`action` makes the whole marker a control at least 32 px high, with a hover
fill, that can be focused.

## Motion

The highlight of a loading, shimmering or thinking label and the spinner are
continuous, not transitions. With reduced motion both hold still and the
label is drawn in its plain color.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A marker with `action` is activated by pointer or Enter, with the ID given to `action`. |
