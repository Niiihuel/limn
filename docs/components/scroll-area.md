# ScrollArea

A viewport that scrolls content taller than itself, with a vertical
scrollbar. It also builds long lists of rows of one height, only the rows in
view.

## Import

```rust
use limn::{Component, ScrollArea, ScrollMotion, ScrollbarAppearance};
```

## Usage

A scroll area takes an ID and its content. Give the rendered element a
height, or `flex` in a column, and the content scrolls inside it. The ID
keeps the scroll position from one frame to the next, so it must stay the
same.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    ScrollArea::new("files", Element::column(rows(state, palette)))
        .render()
        .size(None, Some(240.0))
}
```

It reports no actions. To scroll it from the host see
[Layout](../layout.md#scrolling-from-the-host), and to keep a child pinned at
its top see [Layout](../layout.md#sticky-children).

## A list of uniform rows

```rust
ScrollArea::uniform_list("log", state.lines.len(), 28.0, move |index| {
    ListRow::new(format!("line-{index}"), format!("Line {}", index + 1), palette).render()
})
.render()
.flex(1.0)
```

`uniform_list(id, row_count, row_height, build_row)` builds each row from
its index when it comes into view, so the row count can be large. The closure
is `Fn(usize) -> Element + Send + Sync + 'static`: it owns what it reads.

```rust
ScrollArea::uniform_list("log", count, 28.0, build_row).expansion(state.open_row, details)
```

`expansion(index, content)` shows `content` under one row, at the height it
takes. It scrolls with the rows and the rows after it move down. One row is
open at a time; pass the content on every rebuild while it is.

## Scrollbar

```rust
ScrollArea::new("files", content).appearance(ScrollbarAppearance::overlay()).render()
```

By default the scrollbar has a rail of its own beside the content while the
content overflows, so it stays clear of controls at the content's edge.
`ScrollbarAppearance::overlay()` draws it over the content, which keeps its
width. Content that fits shows no scrollbar and reserves no rail.

| Field | Default | What it is |
| --- | --- | --- |
| `placement` | `ScrollbarPlacement::Reserved` | Beside the content, or `Overlay` on it |
| `buttons` | `true` | An up and a down button at the ends of the rail |
| `rail_width` | 14 | From 4 to 24 through `.rail_width` |
| `thumb_width` | 5 | From 2 to 12 through `.thumb_width` |
| `min_thumb_length` | 24 | The shortest the thumb gets |
| `tint` | `None` | The scrollbar's color; `None` uses the element's foreground |
| `idle_thumb_opacity`, `hover_thumb_opacity`, `active_thumb_opacity` | 72, 118, 164 | The thumb at rest, under the pointer and dragged, out of 255 |
| `hover_track_opacity` | 14 | The rail under the pointer; at rest it is transparent |

`ScrollArea::buttons(enabled)` and `ScrollArea::tint(color)` change that one
field and keep the rest of the appearance.

## Motion

```rust
ScrollArea::new("files", content)
    .motion(ScrollMotion { duration_ms: 0, ..ScrollMotion::default() })
    .render()
```

A step of the wheel or of a scrollbar button eases to its place over
`ScrollMotion`, 140 ms by default. A duration of zero lands it at once.
Dragging the thumb is always direct.
