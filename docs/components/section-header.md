# SectionHeader

The heading of a section: a title, a count right after it, and a detail and a
trailing element at the far end. With an action the whole row is a button
that can fold the section.

## Import

```rust
use limn::{Component, SectionHeader, SectionSizing};
```

## Usage

A heading needs a title and the palette. Without an action it is passive.

```rust
fn view(palette: ComponentPalette) -> Element {
    SectionHeader::new("Installed", palette).count(128).render()
}
```

## Count and detail

```rust
SectionHeader::new("Sources", palette).detail("12").render()
```

`count` shows how many items the section holds, in a chip right after the
title. `detail` is muted text at the far end of the row.

## Trailing

```rust
SectionHeader::new("Tools", palette)
    .trailing(
        IconButton::new("add-tool", Icon::Add, palette)
            .variant(ButtonVariant::Ghost)
            .button_size(ButtonSize::Mini)
            .render(),
    )
    .render()
```

`trailing` puts an element at the far end of the row, after the detail.

## A section that folds

`disclosure` draws a chevron before the title: down for `Some(true)`, right
for `Some(false)`, none for `None`. `action` makes the whole row a button
that reports `UiAction::Clicked` with the given ID. The header keeps no
state: the host holds whether the section is open.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    SectionHeader::new("Changes", palette)
        .disclosure(Some(state.changes_open))
        .detail(if state.changes_open { "3 files" } else { "hidden" })
        .action("fold-changes")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "fold-changes" {
            state.changes_open = !state.changes_open;
        }
    }
}
```

## Width and truncate

```rust
SectionHeader::new("A section title too long for its row", palette)
    .truncate(true)
    .width(280.0)
    .render()
```

`width` fixes the row's width, for a row that must not follow its parent's;
a non-finite value is ignored. `truncate(true)` cuts the title to one line
instead of wrapping it.

## Sizing

```rust
SectionHeader::new("Sources", palette).sizing(SectionSizing::LABEL).render()
```

| Profile | For | Height | Title |
| --- | --- | --- | --- |
| `SectionSizing::TITLE` | A section of a page; the default | 28 | 15 px |
| `SectionSizing::LIST` | The heading of a list | 28 | 14 px |
| `SectionSizing::LABEL` | A quiet label over a group in a side panel | 25 | 11 px, muted |

`LABEL` also insets the row by 4 and starts the title and the detail at the
top inset instead of centering them. A host with a density of its own builds
a `SectionSizing` from its fields: `height`, `padding`, `gap`, `title_size`,
`title_muted`, `detail_size` and `align_top`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` | A row with `action(id)` is clicked. |
