# Attachment

A card for a file or an image attached to a message or a draft: an icon or a
preview, a title and a description, and optionally a button that removes it.

## Import

```rust
use limn::{Attachment, AttachmentOrientation, Component};
```

## Usage

An attachment takes an ID, a title and the palette. The host owns the file:
the card shows what it is given and reports the actions the host names.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Attachment::new("notes", "notes.md", palette)
        .description("Markdown · 2 KB")
        .remove_action("notes-remove")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if id == "notes-remove" {
            state.detach("notes");
        }
    }
}
```

The title and the description take one line each and are cut with an
ellipsis. Hovering them shows the title in full over the description.

## Icon and preview

```rust
Attachment::new("take", "take-04.wav", palette).icon(Icon::Audio).render()
Attachment::new("photo", "dusk.png", palette).preview(picture).render()
```

`icon` replaces the file glyph. `preview` takes a `RasterImage` and shows it
in the icon's place.

## Remove and open

`remove_action(id)` adds a close button that reports that ID.
`open_action(id)` makes the whole card a button that reports that ID, with
the title and the description as its hint.

```rust
Attachment::new("report", "report.pdf", palette)
    .description("PDF · 1.2 MB")
    .help("Shared by Ada on Monday")
    .open_action("report-open")
    .render()
```

`help` is the text of the hint, in place of the description.

## Orientation

```rust
Attachment::new("photo", "dusk.png", palette)
    .preview(picture)
    .orientation(AttachmentOrientation::Vertical)
    .render()
```

`AttachmentOrientation::Horizontal`, the default, puts the icon or preview
beside the caption in a card `Attachment::FILE_HEIGHT` (64) high.
`Vertical` puts it above the caption in a card `Attachment::IMAGE_HEIGHT`
(140) high, with the remove button over its top corner.

## Preview only

```rust
Attachment::new("photo", "dusk.png", palette)
    .preview(picture)
    .preview_only()
    .width(Attachment::PREVIEW_SIZE)
    .render()
```

`preview_only` draws a square of the card's width with the preview alone;
the title and the description stay in its hint. It sets the vertical
orientation, and a later `orientation` brings the caption back.

## Width

`width` fixes the card's width, never under 80; the default is 220. A width
that is not finite is ignored.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with the remove ID | The close button is clicked. |
| `UiAction::Clicked { id }` with the open ID | The card is clicked. |

## Accessibility

With an open action the card is a button named by its title and described by
`help`, or by the description without one. The remove button is named for
what it removes, as "Remove notes.md".
