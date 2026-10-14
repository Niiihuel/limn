# Pagination

Numbered pages between a previous and a next arrow. Use it under a list or a
table that is shown one page at a time, where a reader may want to jump to a
page as well as step to the next.

## Import

```rust
use limn::{Component, ControlSize, Pagination};
```

## Usage

A pagination is controlled: pass the current page and the number of pages
each frame, and store the page a button reports. Pages count from 1.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Pagination::new("results", state.page, state.pages, palette).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(page) = Pagination::page_of(id, "results") {
            state.page = page;
        }
    }
}
```

Every button reports `UiAction::Clicked` with `{id}={page}`: `results=4` for
the number 4, and for the previous arrow on page 5. `Pagination::page_of(action_id, id)`
reads the page back. It is `None` for an action that is not from this row, so
it can be tried on every click. The current page is a button too and reports
its own page.

A `page` below 1 is read as the first page, and one past the end as the last.

## Which pages show

```rust
Pagination::new("results", 6, 40, palette).siblings(2).render()
```

The first and the last page always show. Between them stand the current page
and `siblings` pages on each side of it, 1 unless set, and an ellipsis where
pages are left out: `1 … 4 5 6 … 12`. An ellipsis is three muted dots, not a button.

The row has `2 * siblings + 5` cells between its arrows whatever the current
page. Near an end the cells an ellipsis would not need hold more numbers
instead: page 1 of 12 is `1 2 3 4 5 … 12`. So a button stays under the
pointer while the reader clicks through. With no more pages than cells, every
page shows and there is no ellipsis.

Each number's cell is as wide as the row is high, and wider for all of them
when the last page has more digits than fit, so the row is the same width on
every page. The width is worked out from the number of digits, not measured.

## Compact

```rust
Pagination::new("results", state.page, state.pages, palette).compact().render()
```

`compact()` shows the two arrows around the position as text, "3 / 12", in
place of the numbers. The text sits in a slot wide enough for the last page,
so the arrows do not move.

## Sizes

```rust
Pagination::new("results", state.page, state.pages, palette).size(ControlSize::Dense).render()
```

| Size | Height of the arrows and the numbers |
| --- | --- |
| `ControlSize::Dense` | 24 |
| `ControlSize::Compact` | 28 |
| `ControlSize::Regular`, the default | 32 |
| `ControlSize::Spacious` | 36 |

## The ends and an empty list

On the first page the previous arrow is disabled, and on the last the next
one: it keeps its place and its hint, and takes no pointer or focus.

With one page both arrows are disabled around the number 1. With no pages the
row is the two disabled arrows alone, around "0 / 0" if it is compact, so the
place under an empty list does not collapse. Leave the row out if you would
rather show nothing.

## Labels

```rust
Pagination::new("resultados", state.page, state.pages, palette)
    .labels("Página anterior", "Página siguiente", "Página")
    .render()
```

`labels(previous, next, page)` gives the words of the row: the hint and name
of each arrow, and the word a number is named with for a screen reader, said
before the number. They are "Previous page", "Next page" and "Page" unless
set.

## Disabled

```rust
Pagination::new("results", state.page, state.pages, palette).disabled().render()
Pagination::new("results", state.page, state.pages, palette).enabled(!state.loading).render()
```

A disabled row keeps every cell in its place, in the muted color, with the
current page still marked by a fill. It takes no pointer or focus and reports
nothing.

## Pagination or Pager

| Use | When |
| --- | --- |
| `Pagination` | The sequence is numbered pages and the reader may jump to one. It owns the arithmetic: which pages show, which ends can be taken, what each button reports. |
| `Pagination` with `compact()` | The same pages in a narrow place: previous, "3 / 12", next. |
| [Pager](pager.md) | The position is not a page number, or the label is yours to write: "change 2 of 5", a file name, search matches. The host writes the label, gives each arrow its own action ID and says which ends can be taken. It is 24 high and can point up and down. |

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Move through the arrows and the numbers that can be taken, in order. |
| Enter, Space | Report the page of the focused button. |

A number keeps the focus when it becomes the current page, so Enter can be
followed by Tab to the next one.

## Motion

The hover and press fills of each button fade over `tokens().motion.hover_ms`.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with `{id}={page}` | A number is activated, with its own page; the previous arrow, with the page before the current one; the next arrow, with the page after it. |

## Accessibility

Each number is a button named "Page 4", and the current one is told as the
selected one. The arrows are buttons named by their hints, "Previous page" and
"Next page". A button that cannot be taken, at an end or in a disabled row, is
still there and says it is disabled.

```rust
Pagination::new("results", state.page, state.pages, palette).accessibility_label("Results pages")
```

With `accessibility_label` the row is a group of that name, known by the
row's ID, around its buttons. In compact form the position is plain text
between the arrows.
