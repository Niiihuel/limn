# ScrollArea

A viewport that scrolls content taller than itself, with a vertical
scrollbar. It also builds long lists, of rows of one height or of rows that
each take their own, only the rows in view.

## Import

```rust
use limn::{Component, ScrollArea, ScrollMotion, ScrollbarAppearance, ScrollbarVisibility};
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

## A list of rows of their own heights

```rust
ScrollArea::measured_list(
    "messages",
    state.messages.len(),
    56.0,
    move |index| ids[index],
    move |index| message_row(&messages[index], palette),
)
.follow_end(true)
.render()
.flex(1.0)
```

`measured_list(id, row_count, estimated_row_height, row_key, build_row)` is a
list of rows that each take the height they need, such as the messages of a
conversation. Like a uniform list it builds only the rows in view. Each row
is measured at the width of the list when it is built; until then
`estimated_row_height` stands for it, so the scrollbar is exact for the rows
seen and close for the others.

`row_key` names the row at an index, with a `u64` that stays the same for
that row: an ID the host already has, or a hash of one. The tree keeps each
height under its key, so a row measured once is not measured from nothing
again, and the row at the top of the viewport stays where it is when the
rows above it are measured, grow, shrink or are added. A page of older
messages loaded above the ones being read does not move them, nor does a
row above them that a reaction made taller.

`follow_end(true)` keeps the list at its end while it is scrolled there: rows
added at the end, or a last row that grows, come into view, as a
conversation follows new messages. Scrolled away from the end, the list holds
its place. A short list stays at its top until it overflows.

`UiTree::visible_rows(id)` gives the range of rows built at the last layout,
for a host that loads more before the first one shows or marks what was seen.
`scroll_to_index`, `scroll_to_end` and Tab work as they do in a uniform list,
and `reveal` and `anchor_offset` reach controls and anchors inside the rows
in view.

A measured list has no `expansion`: a row that opens grows itself.

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

To show a scrollbar only after scrolling, opt in to activity visibility:

```rust
ScrollArea::new("files", content)
    .appearance(ScrollbarAppearance::overlay().buttons(false))
    .visibility(ScrollbarVisibility::AutoHide { hold_ms: 600, fade_ms: 200 })
    .render()
```

The bar starts hidden. Changes in scroll position show it immediately;
hovering or dragging a visible bar keeps it shown. After activity stops it
stays for `hold_ms`, then fades over `fade_ms`. A hidden bar has no pointer
target. Hiding preserves content width, including any reserved rail, so rows
never move sideways. `UiTree::animation_repaint_after()` includes its hold
deadline and fade; reduced motion keeps the hold and hides without fading.
`ScrollbarVisibility::Always` is the unchanged default. The policy also
works on uniform and measured lists, and on low-level scroll elements through
`Element::scrollbar_visibility`.

The same policy applies to the thin indicator enabled by `drag_to_pan()` on
`Element::horizontal_scroll` and `horizontal_scroll_2d`:

```rust
Element::horizontal_scroll_2d("shelf", cards)
    .drag_to_pan()
    .scrollbar_visibility(ScrollbarVisibility::AutoHide { hold_ms: 600, fade_ms: 200 })
```

Horizontal position changes show it, and an active pointer pan keeps it visible
until release. Only the indicator's paint hides: the viewport still scrolls,
drags and dispatches child controls while it is hidden. The indicator overlays
the same content bounds throughout. Without `drag_to_pan()` no indicator is
added. Its default is still `Always`, with the existing geometry and opacity.

## Motion

```rust
ScrollArea::new("files", content)
    .motion(ScrollMotion { duration_ms: 0, ..ScrollMotion::default() })
    .render()
```

A step of the wheel or of a scrollbar button eases to its place over
`ScrollMotion`, 140 ms by default. A duration of zero lands it at once.
Dragging the thumb is always direct.
