# Table

Rows of cells that line up under a header of column labels, for a list the
user sorts, filters and reshapes. The host owns the rows and every choice; the
table lays them out and reports what was asked.

## Import

```rust
use limn::{Component, Icon, RadiusRole, Table, TableColumn, TableRow, TableSizing, TableSort};
```

## Usage

A table takes an ID, its columns, the width it is given and the palette. Each
row takes an ID and one cell per column.

```rust
fn view(state: &State, width: f32, palette: ComponentPalette) -> Element {
    let text = |value: &str| Text::new(value, palette).truncate().render();
    let columns = vec![
        TableColumn::flexible("title", "Title", 160.0).sortable(),
        TableColumn::fixed("length", "Length", 56.0).sortable().align_end(),
    ];
    let rows = state
        .tracks
        .iter()
        .map(|track| {
            TableRow::new(format!("track:{}", track.id), vec![text(&track.title), text(&track.length)])
                .selected(track.id == state.playing)
        })
        .collect();
    Table::new("tracks", columns, width, palette).rows(rows).sort(state.sort.clone()).render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        if let Some(key) = id.strip_prefix("tracks:sort:") {
            state.sort = TableSort::toggled(state.sort.take(), key);
            state.sort_tracks();
        } else if let Some(track) = id.strip_prefix("track:") {
            state.play(track);
        }
    }
}
```

The width decides which columns fit and how wide the flexible ones are, so
pass the width the table really gets. The host filters and sorts the rows; the
table shows them in the order given. `crates/limn-gallery/src/pages/table.rs`
is a whole host. [Components](../components.md) has the full reference.

## Columns

```rust
TableColumn::fixed("index", "#", 24.0).align_end()
TableColumn::flexible("title", "Title", 160.0)
TableColumn::fixed("length", "Length", 56.0).icon(Icon::Clock)
```

`fixed(key, label, width)` is a column of that width. `flexible(key, label,
min_width)` shares the width the fixed ones leave, never under its minimum.
`key` names the column in the table's actions. `align_end` sets the label and
the cells against the end of the column, for figures. `icon` shows a glyph in
the header in place of the label, which becomes its hint.

## Sorting

```rust
TableColumn::fixed("album", "Album", 180.0).sortable()
Table::new("tracks", columns, width, palette).sort(state.sort.clone())
```

`sortable` makes the header a button that reports `{id}:sort:{key}`. `sort`
takes the `TableSort` the host keeps: the sorted column's label takes the text
color and an arrow, up for ascending and down for descending.
`TableSort::toggled(current, key)` gives what a click leaves: ascending, then
descending, then no sort; another column starts over.

## Resizing

```rust
TableColumn::fixed("album", "Album", state.album_width).resizable(80.0)
```

`resizable(min_width)` lets the boundary beside a fixed column be dragged. The
drag is reported on `{id}:resize:{key}`; the host stores the width and passes
it back to `fixed`. The column is never drawn under `min_width`. A flexible
column ignores it.

A column before the first flexible one grows as its handle goes right, one
after it as its handle goes left, and the flexible column takes up the change.

## Optional columns

```rust
TableColumn::fixed("album", "Album", 180.0).optional(state.album_shown).hide_below(460.0)
Table::new("tracks", columns, width, palette).columns_menu(state.columns_open, "Columns")
```

`optional(shown)` lists the column in the columns menu, checked while shown; a
column that is not shown is left out of the table. `hide_below(width)` leaves
a column out while the table is narrower than that.

`columns_menu(open, title)` adds a button at the end of the header that opens
the list of optional columns, with `title` as its hint. Without it the same
list is still the header's context menu. `menu_palette` colors both menus.

## Rows

```rust
TableRow::new("track:7", cells).selected(true).controls(["track:7:more"])
```

A row takes one cell per column in the order the columns were given, those not
shown included. `selected` fills the row with the palette's `selected`.
`controls` names the controls inside its cells: they keep their own targets,
and the row stays lit while the pointer is over one of them.

A cell hugs its content at the start or the end of its column. A cell the host
made `.flex(1.0)` fills its column.

## Sticky header

```rust
let table = Table::new("tracks", columns, width, palette).sticky_header().render();
ScrollArea::new("tracks-scroll", table).render()
```

`sticky_header` keeps the header at the top of the scroll area around the
table while its rows scroll under it, filled with the palette's `surface`.

## Sizing and radius

```rust
Table::new("tracks", columns, width, palette)
    .sizing(TableSizing::ROOMY)
    .radius(RadiusRole::Surface)
```

`TableSizing::COMPACT`, the default, is rows 28 high under a header of 28.
`ROOMY` is 56 under 36, for rows that hold artwork and two lines of copy.
`radius` takes the role of the corners of a row's fill; `RadiusRole::Control`
unless set.

## Wrapping each row

```rust
let (header, rows) = Table::new("tracks", columns, width, palette).rows(rows).into_parts();
```

`into_parts`, in place of `render`, gives the header and one element per row,
for a host that wraps each row in something of its own, such as a context
menu. Stack them in one column, the first row `header_gap` under the header
and the rows `row_gap` apart.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves through the sortable headers, the handles and the columns button, then the rows. |
| Enter | Activates the header, the button or the row in focus. |

## Motion

A row's fill answers the pointer over `motion.hover_ms`.

## Actions

| Action | When |
| --- | --- |
| `Clicked` on `{id}:sort:{key}` | The header of a sortable column is activated. |
| `DragStarted`, `DragMoved`, then `Dropped` or `DragCancelled`, on `{id}:resize:{key}` | The boundary of a resizable column is dragged. |
| `Clicked` on `{id}:column:{key}` | An optional column is toggled, in the columns menu or in the header's context menu. |
| `Clicked` on `{id}:columns`, and on `{id}:columns:dismiss` | The columns button is activated; its menu is dismissed. |
| `Clicked` on a row's ID | The row is activated. |

## Accessibility

The header of a sortable column is a button named by the column's label, also
when it shows only an icon, and says whether it is the selected one, the
column the rows are sorted by. The columns button is named by its title and
says whether its menu is expanded. A row is an item of a list named by the
text of its cells, and says whether it is selected. The platform is not told
the direction of the sort, nor that the rows and columns form a table.
