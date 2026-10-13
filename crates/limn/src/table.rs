//! A table: a header of column labels over rows whose cells line up under it.

use crate::{
    page::px, ButtonVariant, Color, Component, ComponentPalette, ContextMenu, ContextMenuEntry,
    ContextMenuItem, ControlSize, DropdownMenu, DropdownMenuItem, Element, FontFamily, Icon,
    IconButton, PopoverPlacement, RadiusRole, ResizeHandle, Separator, Text, TextRole, Tooltip,
    TooltipPlacement, TooltipSpec,
};

// Width of the strip that takes the pointer on a boundary between two columns.
const GRIP: f32 = 6.0;
// Width of the menu that lists the optional columns.
const MENU_WIDTH: f32 = 200.0;

/// Density of a [`Table`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableSizing {
    /// Height of the header, with the hairline under it.
    pub header_height: f32,
    /// Height of a row.
    pub row_height: f32,
    /// Space between the header's hairline and the first row: a row lit
    /// under the pointer does not touch the line.
    pub header_gap: f32,
    /// Space between two rows.
    pub row_gap: f32,
    /// Space between the table's edge and its first and last columns,
    /// inside a row's hover surface.
    pub inset: f32,
    /// Space between two columns, where a resize handle stands.
    pub gap: f32,
    /// Type role of the header's labels.
    pub label_role: TextRole,
    /// Space between a label and its sort caret.
    pub caret_gap: f32,
}

impl TableSizing {
    /// A table of data in a panel: rows 28 high under a header of 28.
    pub const COMPACT: Self = Self {
        header_height: 28.0,
        row_height: 28.0,
        header_gap: 4.0,
        row_gap: 2.0,
        inset: 8.0,
        gap: 12.0,
        label_role: TextRole::Small,
        caret_gap: 2.0,
    };

    /// Rows that hold artwork and two lines of copy, such as a list of
    /// tracks: 56 high under a header of 36.
    pub const ROOMY: Self = Self {
        header_height: 36.0,
        row_height: 56.0,
        header_gap: 8.0,
        inset: 16.0,
        gap: 16.0,
        label_role: TextRole::Caption,
        caret_gap: 4.0,
        ..Self::COMPACT
    };
}

/// The column a [`Table`] is sorted by. The host owns it and sorts its rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableSort {
    /// Key of the sorted column.
    pub key: String,
    /// Largest first.
    pub descending: bool,
}

impl TableSort {
    /// What a click on the header of `key` leaves: ascending, then
    /// descending, then no sort at all. Another column starts over.
    pub fn toggled(current: Option<Self>, key: &str) -> Option<Self> {
        match current {
            Some(sort) if sort.key == key => (!sort.descending).then_some(Self { descending: true, ..sort }),
            _ => Some(Self { key: key.to_owned(), descending: false }),
        }
    }
}

/// One column of a [`Table`]: what its header shows and how wide it is.
pub struct TableColumn {
    key: String,
    label: String,
    icon: Option<Icon>,
    // The width of a fixed column, the least width of a flexible one.
    width: f32,
    flexible: bool,
    min_width: Option<f32>,
    sortable: bool,
    align_end: bool,
    shown: Option<bool>,
    hide_below: f32,
}

impl TableColumn {
    /// A column `width` wide. `key` names it in the table's actions; `label`
    /// is its header, and its name in the columns menu.
    pub fn fixed(key: impl Into<String>, label: impl Into<String>, width: f32) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            icon: None,
            width: px(width),
            flexible: false,
            min_width: None,
            sortable: false,
            align_end: false,
            shown: None,
            hide_below: 0.0,
        }
    }

    /// A column that shares the width the fixed ones leave, never under
    /// `min_width`.
    pub fn flexible(key: impl Into<String>, label: impl Into<String>, min_width: f32) -> Self {
        Self { flexible: true, ..Self::fixed(key, label, min_width) }
    }

    /// Show a glyph in the header in place of the label, which becomes its
    /// hint: a clock over a column of durations.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Make the header a button that reports `{table}:sort:{key}`.
    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    /// Let the boundary beside a fixed column be dragged. The drag is
    /// reported on `{table}:resize:{key}`; the host stores the width and
    /// passes it back, and the column is never drawn under `min_width`.
    /// A flexible column ignores it, as are values that are not finite.
    pub fn resizable(mut self, min_width: f32) -> Self {
        if min_width.is_finite() && !self.flexible {
            let min_width = min_width.max(0.0);
            self.min_width = Some(min_width);
            self.width = self.width.max(min_width);
        }
        self
    }

    /// Set the label and the cells against the end of the column, for
    /// figures.
    pub fn align_end(mut self) -> Self {
        self.align_end = true;
        self
    }

    /// List the column in the columns menu, checked while `shown`; toggling
    /// it reports `{table}:column:{key}`. A column that is not shown is
    /// left out of the table.
    pub fn optional(mut self, shown: bool) -> Self {
        self.shown = Some(shown);
        self
    }

    /// Leave the column out while the table is narrower than `width`.
    /// Values that are not finite are ignored.
    pub fn hide_below(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.hide_below = width.max(0.0);
        }
        self
    }

    // The least this column is drawn at while the others still fit.
    fn floor(&self) -> f32 {
        self.min_width.unwrap_or(self.width)
    }
}

/// One row of a [`Table`]: a click target across its cells.
pub struct TableRow {
    id: String,
    cells: Vec<Element>,
    selected: bool,
    controls: Vec<String>,
}

impl TableRow {
    /// A row reporting a click on `id`, with one cell per column of the
    /// table in the order the columns were given, those not shown included.
    /// A cell hugs its content unless the host made it flexible.
    pub fn new(id: impl Into<String>, cells: Vec<Element>) -> Self {
        Self { id: id.into(), cells, selected: false, controls: Vec::new() }
    }

    /// Fill the row with the palette's `selected`: the chosen row, or the
    /// one that plays.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// The controls inside its cells, by ID. They keep their own targets,
    /// and the row stays lit while the pointer is over one of them.
    pub fn controls(mut self, ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.controls = ids.into_iter().map(Into::into).collect();
        self
    }
}

/// A header of column labels over rows of cells that line up under it. The
/// host owns everything: the rows, already filtered and in order, the sort,
/// the width of each resizable column, which optional columns are shown and
/// whether the columns menu is open. The table reports:
///
/// - `Clicked` on `{id}:sort:{key}` for the header of a sortable column;
/// - `DragStarted`, `DragMoved` and `Dropped` or `DragCancelled` on
///   `{id}:resize:{key}`, as a [`ResizeHandle`] does. A column before the
///   first flexible one grows as its handle goes right, one after it as
///   its handle goes left, and the flexible column takes up the change;
/// - `Clicked` on `{id}:column:{key}` for an optional column toggled in
///   the columns menu or in the header's context menu;
/// - `Clicked` on `{id}:columns` for the columns button and on
///   `{id}:columns:dismiss` when its menu is dismissed;
/// - `Clicked` on a row's own ID.
pub struct Table {
    id: String,
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    width: f32,
    sort: Option<TableSort>,
    sticky: bool,
    menu: Option<(bool, String)>,
    menu_palette: ComponentPalette,
    radius: RadiusRole,
    sizing: TableSizing,
    palette: ComponentPalette,
}

impl Table {
    /// A header alone, `width` wide, in the [`TableSizing::COMPACT`]
    /// profile. The width decides which columns fit and how wide the
    /// flexible ones are, so the host passes what the table is given.
    pub fn new(
        id: impl Into<String>,
        columns: Vec<TableColumn>,
        width: f32,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            columns,
            rows: Vec::new(),
            width: px(width),
            sort: None,
            sticky: false,
            menu: None,
            menu_palette: palette,
            radius: RadiusRole::Control,
            sizing: TableSizing::COMPACT,
            palette,
        }
    }

    /// The rows to show, in the order to show them.
    pub fn rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }

    /// The column the rows are sorted by: its label takes the text color
    /// and a caret that says which way.
    pub fn sort(mut self, sort: Option<TableSort>) -> Self {
        self.sort = sort;
        self
    }

    /// Keep the header at the top of the scroll area around the table
    /// while its rows scroll under it, filled with the palette's `surface`.
    pub fn sticky_header(mut self) -> Self {
        self.sticky = true;
        self
    }

    /// A button at the end of the header that opens the list of optional
    /// columns, with `title` as its hint. Rows leave its slot empty. Without
    /// it the same list is still the header's context menu.
    pub fn columns_menu(mut self, open: bool, title: impl Into<String>) -> Self {
        self.menu = Some((open, title.into()));
        self
    }

    /// The palette of the columns menu and of the header's context menu;
    /// the table's own unless set.
    pub fn menu_palette(mut self, palette: ComponentPalette) -> Self {
        self.menu_palette = palette;
        self
    }

    /// The corners of a row's fill, by role. [`RadiusRole::Control`] unless
    /// set.
    pub fn radius(mut self, role: RadiusRole) -> Self {
        self.radius = role;
        self
    }

    /// The table's density. [`TableSizing::COMPACT`] unless set.
    pub fn sizing(mut self, sizing: TableSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// The header and the rows apart, for a host that wraps each row in
    /// something of its own, such as a context menu. Stack them in one
    /// column, the first row `header_gap` under the header and the rows
    /// `row_gap` apart: a sticky header pins inside that column.
    pub fn into_parts(self) -> (Element, Vec<Element>) {
        let Self { id, columns, rows, width, sort, sticky, menu, menu_palette, radius, sizing, palette } = self;
        let (inset, gap) = (px(sizing.inset), px(sizing.gap));
        let shown: Vec<usize> = (0..columns.len())
            .filter(|&index| columns[index].shown.unwrap_or(true) && width >= columns[index].hide_below)
            .collect();
        let visible: Vec<&TableColumn> = shown.iter().map(|&index| &columns[index]).collect();
        let optional: Vec<&TableColumn> = columns.iter().filter(|column| column.shown.is_some()).collect();
        let menu = menu.filter(|_| !optional.is_empty());
        let tail = if menu.is_some() { gap + ControlSize::Dense.height() } else { 0.0 };
        let room = (width - 2.0 * inset - tail - gap * visible.len().saturating_sub(1) as f32).max(0.0);
        let widths = column_widths(&visible, room);

        let heading = |column: &TableColumn, width: f32| {
            let sort_id = format!("{id}:sort:{}", column.key);
            let sorted = sort.as_ref().filter(|sort| column.sortable && sort.key == column.key);
            let ink = if sorted.is_some() { palette.text } else { palette.muted };
            let (label, anchor) = if column.sortable {
                let button = match column.icon {
                    Some(icon) => Element::icon_button(&sort_id, icon, ""),
                    None => Element::button(&sort_id, &column.label),
                };
                let button = match sorted {
                    Some(sort) => button.trailing_icon(if sort.descending { Icon::ArrowDown } else { Icon::ArrowUp }),
                    None => button,
                };
                let button = button
                    .foreground(ink)
                    .hover_foreground(palette.text)
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.label_role.size())
                    .padding(0.0)
                    .gap(px(sizing.caret_gap))
                    .max_width(width);
                (button, sort_id)
            } else {
                let label = match column.icon {
                    Some(icon) => Element::icon(icon).foreground(ink),
                    None => Text::new(&column.label, palette).role(sizing.label_role).ink(ink).truncate().render(),
                };
                (label, format!("{id}:label:{}", column.key))
            };
            // A glyph alone does not name its column.
            let label = match column.icon {
                Some(_) if !column.label.is_empty() => Tooltip::new(label, anchor, &column.label, palette)
                    .placement(TooltipPlacement::Below)
                    .render(),
                _ => label,
            };
            cell(label, width, column.align_end)
        };
        let boundary = |owner: &TableColumn| match owner.min_width {
            Some(_) => {
                let handle = ResizeHandle::vertical(format!("{id}:resize:{}", owner.key), palette)
                    .thickness(gap.min(GRIP))
                    .render();
                let across = |part: Element| Element::row(vec![Element::spacer().flex(1.0), part, Element::spacer().flex(1.0)]);
                // A handle draws nothing at rest: a short rule says where the
                // boundary can be taken, and the handle stands over it.
                let bar = px(sizing.header_height) - 1.0;
                let rule = Separator::vertical(palette).length(bar / 2.0).centered_in(bar).render();
                Element::stack(vec![across(rule), across(handle)]).size(Some(gap), None)
            }
            None => Element::spacer().size(Some(gap), None),
        };
        let flexible = visible.iter().position(|column| column.flexible);
        let mut cells = Vec::with_capacity(visible.len() * 2);
        for (slot, (column, &width)) in visible.iter().zip(&widths).enumerate() {
            if slot > 0 {
                // A boundary resizes the column on its side away from the
                // flexible one, which takes up the change.
                let before = flexible.is_none_or(|flexible| slot <= flexible);
                cells.push(boundary(if before { visible[slot - 1] } else { column }));
            }
            cells.push(heading(column, width));
        }

        let toggle = |column: &TableColumn| format!("{id}:column:{}", column.key);
        let mut bar = vec![Element::spacer().size(Some(inset), None), Element::row(cells).clip_children().flex(1.0)];
        if let Some((open, title)) = menu {
            let entries = optional
                .iter()
                .map(|column| {
                    DropdownMenuItem::new(toggle(column), &column.label).selected(column.shown == Some(true)).into()
                })
                .collect();
            let trigger = IconButton::new(format!("{id}:columns"), Icon::Sliders, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .tooltip_with(TooltipSpec::new(title).placement(TooltipPlacement::Below).suppressed(open))
                .active(open)
                .render();
            bar.push(Element::spacer().size(Some(gap), None));
            bar.push(
                DropdownMenu::new(trigger, entries, open, format!("{id}:columns:dismiss"), menu_palette)
                    .context_menu_style()
                    .width(MENU_WIDTH)
                    .placement(PopoverPlacement::BottomEnd)
                    .render()
                    .centered_y(),
            );
        }
        bar.push(Element::spacer().size(Some(inset), None));
        let mut header = Element::column(vec![Element::row(bar).flex(1.0), Separator::horizontal(palette).render()])
            .size(Some(width), Some(px(sizing.header_height)));
        if sticky {
            // Opaque, or the rows show through it while it is pinned.
            header = header.background(palette.surface).sticky_top();
        }
        if !optional.is_empty() {
            let entries = optional
                .iter()
                .map(|column| {
                    let item = ContextMenuItem::new(toggle(column), &column.label);
                    ContextMenuEntry::Item(if column.shown == Some(true) { item.icon(Icon::Check) } else { item })
                })
                .collect();
            header = ContextMenu::new(format!("{id}:header"), header, entries, menu_palette).render();
        }

        let rows = rows
            .into_iter()
            .map(|row| {
                let mut cells: Vec<Option<Element>> = row.cells.into_iter().map(Some).collect();
                let cells = shown
                    .iter()
                    .zip(&widths)
                    .map(|(&index, &width)| {
                        let content = cells.get_mut(index).and_then(Option::take).unwrap_or_else(Element::spacer);
                        cell(content.centered_y(), width, columns[index].align_end)
                    })
                    .collect();
                let content = Element::row(vec![
                    Element::spacer().size(Some(inset), None),
                    Element::row(cells).gap(gap).clip_children().flex(1.0),
                    Element::spacer().size(Some(inset + tail), None),
                ]);
                // The target paints nothing: the surface around it is lit
                // for the row and for the controls in its cells alike.
                let target = Element::clickable(&row.id, content)
                    .background(Color::TRANSPARENT)
                    .transition_ms(0)
                    .radius(radius.px())
                    .flex(1.0);
                let surface = Element::row(vec![target])
                    .hover_background(if row.selected { palette.selected } else { palette.hover })
                    .pressed_background(palette.selected)
                    .hover_surface(format!("{}:surface", row.id), std::iter::once(row.id).chain(row.controls))
                    .transition_ms(crate::tokens().motion.hover_ms)
                    .radius(radius.px())
                    .size(Some(width), Some(px(sizing.row_height)));
                if row.selected {
                    surface.background(palette.selected)
                } else {
                    surface
                }
            })
            .collect();
        (header, rows)
    }
}

impl Component for Table {
    fn render(self) -> Element {
        let (width, under, gap) = (self.width, px(self.sizing.header_gap), px(self.sizing.row_gap));
        let (header, rows) = self.into_parts();
        Element::column(vec![header, Element::column(rows).gap(gap)]).gap(under).size(Some(width), None)
    }
}

/// A cell held to its column: at its start or its end, giving up width
/// when it is wider than the column.
fn cell(content: Element, width: f32, end: bool) -> Element {
    // A cell the host made flexible fills its column instead.
    let children = if content.style.flex > 0.0 {
        vec![content]
    } else if end {
        vec![Element::spacer().flex(1.0), content.shrink_to_fit()]
    } else {
        vec![content.shrink_to_fit(), Element::spacer().flex(1.0)]
    };
    Element::row(children).size(Some(width), None)
}

/// How wide each column is in `room`. Fixed columns take what they ask and
/// flexible ones share the rest over their minimums. Short of room, the
/// resizable columns give width up in order, each down to its minimum, and
/// past that every column shrinks alike, so the total never exceeds `room`.
fn column_widths(columns: &[&TableColumn], room: f32) -> Vec<f32> {
    let mut widths: Vec<f32> = columns.iter().map(|column| column.width).collect();
    let mut excess = widths.iter().sum::<f32>() - room;
    if excess <= 0.0 {
        let flexible = columns.iter().filter(|column| column.flexible).count() as f32;
        for (column, width) in columns.iter().zip(&mut widths) {
            if column.flexible {
                *width -= excess / flexible;
            }
        }
        return widths;
    }
    for (column, width) in columns.iter().zip(&mut widths) {
        let given = (*width - column.floor()).clamp(0.0, excess);
        *width -= given;
        excess -= given;
    }
    if excess > 0.0 {
        let total: f32 = widths.iter().sum();
        for width in &mut widths {
            *width *= room / total;
        }
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cursor, DrawCommand, Frame, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const WIDTH: f32 = 800.0;
    const SIZE: Size = Size::new(WIDTH, 300.0);
    const LONG: &str = "Un título demasiado largo para caber en la columna que le toca";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn text(value: &str) -> Element {
        Text::new(value, palette()).truncate().render()
    }

    // A number, a flexible title, an album to resize, the day it was added
    // and a duration under a clock.
    fn columns() -> Vec<TableColumn> {
        vec![
            TableColumn::fixed("index", "#", 24.0),
            TableColumn::flexible("title", "Título", 120.0).sortable(),
            TableColumn::fixed("album", "Álbum", 160.0).sortable().resizable(80.0).optional(true).hide_below(480.0),
            TableColumn::fixed("added", "Agregado", 100.0).optional(true).hide_below(640.0),
            TableColumn::fixed("duration", "Duración", 48.0).icon(Icon::Clock).sortable().align_end(),
        ]
    }

    fn rows(count: usize) -> Vec<TableRow> {
        (0..count)
            .map(|index| {
                let cells = [&(index + 1).to_string(), &format!("Canción {index}"), "Disco", "ayer", "3:45"];
                TableRow::new(format!("row:{index}"), cells.into_iter().map(text).collect())
            })
            .collect()
    }

    fn table(width: f32) -> Table {
        Table::new("t", columns(), width, palette()).rows(rows(3))
    }

    // The table at the top of a taller window, eight pixels to a character.
    fn tree_of(table: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![table, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        tree
    }

    fn has(frame: &Frame, id: &str) -> bool {
        frame.regions.iter().any(|region| region.id == id)
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("no region {id}")).rect
    }

    fn texts(frame: &Frame) -> Vec<(&str, Point, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, color, .. } => Some((text.as_str(), *position, *color)),
                _ => None,
            })
            .collect()
    }

    fn label(frame: &Frame, value: &str) -> (Point, Color) {
        let found = texts(frame).into_iter().find(|line| line.0 == value);
        found.map(|line| (line.1, line.2)).unwrap_or_else(|| panic!("{value} is not painted"))
    }

    fn icons(frame: &Frame, wanted: &[Icon]) -> Vec<(Icon, Rect, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { rect, icon, tint } if wanted.contains(icon) => Some((*icon, *rect, *tint)),
                _ => None,
            })
            .collect()
    }

    fn fills(frame: &Frame, color: Color) -> Vec<Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
                _ => None,
            })
            .collect()
    }

    fn click(tree: &mut UiTree, point: Point) -> Vec<UiAction> {
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point))
    }

    fn middle(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn clicked(id: &str) -> Vec<UiAction> {
        vec![UiAction::Clicked { id: id.into() }]
    }

    #[test]
    fn cells_line_up_under_their_headers() {
        let frame = tree_of(table(WIDTH).render()).layout(SIZE);
        // Inset 8, then 24, the title, 160, 100 and 48, twelve apart.
        for (header, cell, x) in
            [("#", "1", 8.0), ("Título", "Canción 0", 44.0), ("Álbum", "Disco", 460.0), ("Agregado", "ayer", 632.0)]
        {
            assert_eq!((label(&frame, header).0.x, label(&frame, cell).0.x), (x, x), "{header}");
        }
        // A column of figures ends where its glyph does.
        let clock = icons(&frame, &[Icon::Clock])[0].1;
        assert_eq!((clock.x + clock.width, label(&frame, "3:45").0.x + 32.0), (792.0, 792.0));
        // The glyph names its column in a hint.
        let mut tree = tree_of(table(WIDTH).render());
        let start = std::time::Instant::now();
        tree.layout_at(SIZE, start);
        tree.handle_event_at(UiEvent::PointerMove(middle(clock)), start);
        tree.layout_at(SIZE, start);
        let hinted = tree.layout_at(SIZE, start + std::time::Duration::from_secs(2));
        assert!(texts(&hinted).iter().any(|line| line.0 == "Duración"));
        assert!(texts(&frame).iter().all(|line| line.0 != "Duración"));
        // Every row repeats the first.
        let starts: Vec<f32> = texts(&frame).iter().filter(|line| line.0 == "Disco").map(|line| line.1.x).collect();
        assert_eq!(starts, [460.0; 3]);
    }

    #[test]
    fn each_profile_sets_the_heights_of_the_header_and_the_rows() {
        for sizing in [TableSizing::COMPACT, TableSizing::ROOMY] {
            let frame = tree_of(table(WIDTH).sizing(sizing).render()).layout(SIZE);
            let first = region(&frame, "row:0");
            assert_eq!(first, Rect::new(0.0, sizing.header_height + sizing.header_gap, WIDTH, sizing.row_height));
            assert_eq!(region(&frame, "row:1").y, first.y + sizing.row_height + sizing.row_gap);
            // The hairline closes the header across the whole table.
            let rule = Rect::new(0.0, sizing.header_height - 1.0, WIDTH, 1.0);
            assert!(fills(&frame, palette().border).contains(&rule));
            let title = label(&frame, "Canción 0").0;
            assert_eq!(title.x, sizing.inset + 24.0 + sizing.gap);
            assert!(title.y > first.y && title.y + 12.0 < first.y + first.height);
            // Passive or a button, a label sits on the same line of the header.
            let (passive, button) = (label(&frame, "#"), label(&frame, "Título"));
            assert_eq!((passive.1, passive.0.y), (palette().muted, button.0.y));
            assert!(passive.0.y > 0.0 && passive.0.y + sizing.label_role.size() < sizing.header_height);
        }
    }

    #[test]
    fn a_sortable_header_reports_its_key_and_the_sorted_one_wears_a_caret() {
        const CARETS: [Icon; 2] = [Icon::ArrowUp, Icon::ArrowDown];
        let mut tree = tree_of(table(WIDTH).render());
        let frame = tree.layout(SIZE);
        assert_eq!(label(&frame, "Título").1, palette().muted);
        assert!(icons(&frame, &CARETS).is_empty());
        assert_eq!(click(&mut tree, middle(region(&frame, "t:sort:title"))), clicked("t:sort:title"));
        assert_eq!(click(&mut tree, middle(region(&frame, "t:sort:duration"))), clicked("t:sort:duration"));
        // A column that does not sort has no target.
        assert!(!has(&frame, "t:sort:index") && !has(&frame, "t:sort:added"));

        let gap = TableSizing::COMPACT.caret_gap;
        for (descending, caret) in [(false, Icon::ArrowUp), (true, Icon::ArrowDown)] {
            let sort = |key: &str| Some(TableSort { key: key.into(), descending });
            let frame = tree_of(table(WIDTH).sort(sort("title")).render()).layout(SIZE);
            let title = label(&frame, "Título");
            assert_eq!((title.1, label(&frame, "Álbum").1), (palette().text, palette().muted));
            // Right after the label, in its ink.
            let carets = icons(&frame, &CARETS);
            assert_eq!(carets.len(), 1);
            assert_eq!((carets[0].0, carets[0].1.x, carets[0].2), (caret, title.0.x + 48.0 + gap, palette().text));
            // A glyph sorts as a label does, and still ends its column.
            let frame = tree_of(table(WIDTH).sort(sort("duration")).render()).layout(SIZE);
            let (clock, carets) = (icons(&frame, &[Icon::Clock])[0], icons(&frame, &CARETS));
            assert_eq!((clock.2, carets[0].0), (palette().text, caret));
            assert_eq!((carets[0].1.x, carets[0].1.x + carets[0].1.width), (clock.1.x + clock.1.width + gap, 792.0));
            assert_eq!(label(&frame, "Título").1, palette().muted);
        }
    }

    #[test]
    fn tab_walks_the_headers_then_the_rows_and_enter_activates_them() {
        let mut tree = tree_of(table(WIDTH).columns_menu(false, "Columnas").render());
        tree.layout(SIZE);
        let key = |key| UiEvent::Key { key, shift: false, ctrl: false };
        let mut reported = Vec::new();
        for _ in 0..7 {
            tree.handle_event(key(Key::Tab));
            tree.layout(SIZE);
            reported.extend(tree.handle_event(key(Key::Enter)).into_iter().map(|action| match action {
                UiAction::Clicked { id } => id,
                other => panic!("{other:?}"),
            }));
        }
        let headers = ["t:sort:title", "t:resize:album", "t:sort:album", "t:sort:duration", "t:columns"];
        assert_eq!(reported, [&headers[..], &["row:0", "row:1"]].concat());
    }

    #[test]
    fn toggling_a_sort_goes_up_then_down_then_off() {
        let up = TableSort::toggled(None, "title");
        assert_eq!(up, Some(TableSort { key: "title".into(), descending: false }));
        let down = TableSort::toggled(up, "title");
        assert_eq!(down, Some(TableSort { key: "title".into(), descending: true }));
        // Another column starts over; the same one once more clears it.
        let other = TableSort::toggled(down.clone(), "album");
        assert_eq!(other, Some(TableSort { key: "album".into(), descending: false }));
        assert_eq!(TableSort::toggled(down, "title"), None);
    }

    #[test]
    fn dragging_a_boundary_is_reported_on_the_column_it_resizes() {
        let mut tree = tree_of(table(WIDTH).render());
        let frame = tree.layout(SIZE);
        // In the gap between the title and the album, as tall as the header.
        assert_eq!(region(&frame, "t:resize:album"), Rect::new(451.0, 0.0, GRIP, 27.0));
        // The day and the duration do not resize: their boundaries are bare.
        assert_eq!(frame.regions.iter().filter(|region| region.id.contains(":resize:")).count(), 1);
        // At rest the handle paints nothing: a rule half the header tall marks it, and only it.
        let rules: Vec<Rect> = fills(&frame, palette().border).into_iter().filter(|rule| rule.width == 1.0).collect();
        assert_eq!(rules, [Rect::new(453.5, 6.75, 1.0, 13.5)]);
        let at = |x: f32| Point::new(x, 14.0);
        let id = || "t:resize:album".to_owned();
        tree.handle_event(UiEvent::PointerMove(at(454.0)));
        assert_eq!(tree.cursor(), Some(Cursor::ResizeColumn));
        tree.handle_event(UiEvent::PointerDown(at(454.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerMove(at(420.0))),
            vec![
                UiAction::DragStarted { id: id(), position: at(454.0) },
                UiAction::DragMoved { id: id(), position: at(420.0), over: None },
            ]
        );
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(at(420.0))),
            vec![UiAction::Dropped { id: id(), position: at(420.0), target: None }]
        );
    }

    #[test]
    fn a_resizable_column_is_as_wide_as_the_host_says_within_its_limits() {
        let album = |width: f32| {
            let mut columns = columns();
            columns[2] = TableColumn::fixed("album", "Álbum", width).resizable(80.0);
            let frame = tree_of(Table::new("t", columns, WIDTH, palette()).rows(rows(1)).render()).layout(SIZE);
            (label(&frame, "Álbum").0.x, label(&frame, "Disco").0.x, region(&frame, "t:resize:album").x)
        };
        assert_eq!(album(160.0), (460.0, 460.0, 451.0));
        // After the flexible title it grows towards it, and its handle with it.
        assert_eq!(album(194.0), (426.0, 426.0, 417.0));
        // Never under its own minimum, nor into the title's.
        assert_eq!((album(10.0).0, album(f32::NAN).0), (540.0, 540.0));
        assert_eq!(album(2000.0).0, 8.0 + 24.0 + 12.0 + 120.0 + 12.0);

        // Before the flexible column the handle stands on the far edge.
        let columns = vec![
            TableColumn::fixed("name", "Nombre", 100.0).resizable(60.0),
            TableColumn::flexible("path", "Ruta", 80.0).resizable(40.0),
            TableColumn::fixed("size", "Tamaño", 60.0).resizable(40.0),
        ];
        let frame = tree_of(Table::new("t", columns, WIDTH, palette()).render()).layout(SIZE);
        assert_eq!((region(&frame, "t:resize:name").x, region(&frame, "t:resize:size").x), (111.0, 723.0));
        assert!(!has(&frame, "t:resize:path"), "a flexible column takes what is left");
    }

    #[test]
    fn the_columns_menu_lists_the_optional_columns_and_reports_a_toggle() {
        let menu = |open: bool| {
            let mut columns = columns();
            columns[3] = TableColumn::fixed("added", "Agregado", 100.0).optional(false);
            Table::new("t", columns, WIDTH, palette()).rows(rows(1)).columns_menu(open, "Columnas").render()
        };
        let mut tree = tree_of(menu(false));
        let frame = tree.layout(SIZE);
        // A slot of its own after the last column, which the row keeps clear.
        let trigger = region(&frame, "t:columns");
        assert_eq!((trigger.x, trigger.width, trigger.height), (WIDTH - 8.0 - 24.0, 24.0, 24.0));
        let clock = icons(&frame, &[Icon::Clock])[0].1;
        assert_eq!((clock.x + clock.width, label(&frame, "3:45").0.x + 32.0), (trigger.x - 12.0, trigger.x - 12.0));
        // The column the host hides is gone, and the title has its width.
        assert!(texts(&frame).iter().all(|line| line.0 != "Agregado" && line.0 != "ayer"));
        assert_eq!(label(&frame, "Álbum").0.x, clock.x + clock.width - 48.0 - 12.0 - 160.0);
        assert!(!has(&frame, "t:column:album"));
        assert_eq!(click(&mut tree, middle(trigger)), clicked("t:columns"));

        let mut tree = tree_of(menu(true));
        let frame = tree.layout(SIZE);
        assert!(!has(&frame, "t:column:title"), "only the optional columns are listed");
        // One check: the album is shown and the day is not.
        let (album, added) = (region(&frame, "t:column:album"), region(&frame, "t:column:added"));
        let checks = icons(&frame, &[Icon::Check]);
        assert_eq!(checks.len(), 1);
        assert!(checks[0].1.y >= album.y && checks[0].1.y < added.y);
        assert_eq!(click(&mut tree, middle(added)), clicked("t:column:added"));
        assert_eq!(click(&mut tree, Point::new(20.0, 280.0)), clicked("t:columns:dismiss"));

        // No optional column, no button.
        let bare = Table::new("t", vec![TableColumn::flexible("title", "Título", 80.0)], WIDTH, palette());
        assert!(!has(&tree_of(bare.columns_menu(true, "Columnas").render()).layout(SIZE), "t:columns"));
    }

    #[test]
    fn the_header_context_menu_toggles_the_same_columns() {
        let mut tree = tree_of(table(WIDTH).render());
        tree.layout(SIZE);
        assert!(tree.handle_event(UiEvent::ContextMenu { position: Point::new(300.0, 14.0) }).is_empty());
        assert!(tree.context_menu_open());
        let frame = tree.layout(SIZE);
        // Album and day, in the order of the columns, both shown.
        assert_eq!(icons(&frame, &[Icon::Check]).len(), 2);
        let added = region(&frame, &crate::context_menu::row_id(1));
        assert_eq!(click(&mut tree, middle(added)), clicked("t:column:added"));
        // A row is not the header.
        let mut tree = tree_of(table(WIDTH).render());
        tree.layout(SIZE);
        tree.handle_event(UiEvent::ContextMenu { position: Point::new(300.0, 44.0) });
        assert!(!tree.context_menu_open());
    }

    #[test]
    fn a_narrow_table_drops_the_columns_that_ask_to_and_never_overflows() {
        for width in [200.0, 400.0, 479.0, 480.0, 600.0, 640.0, 800.0] {
            let sort = TableSort { key: "title".into(), descending: false };
            let table = table(width).columns_menu(false, "Columnas").sort(Some(sort));
            let frame = tree_of(table.render()).layout(Size::new(width, 300.0));
            let painted = |value: &str| texts(&frame).iter().any(|line| line.0 == value);
            assert_eq!((painted("Álbum"), painted("Disco")), (width >= 480.0, width >= 480.0), "{width}");
            assert_eq!((painted("Agregado"), painted("ayer")), (width >= 640.0, width >= 640.0), "{width}");
            assert_eq!(has(&frame, "t:resize:album"), width >= 480.0);
            for (text, position, _) in texts(&frame) {
                let end = position.x + text.chars().count() as f32 * 8.0;
                assert!(position.x >= 0.0 && end <= width, "{text} at {width}: {end}");
            }
            for command in &frame.commands {
                if let DrawCommand::Icon { rect, .. } = command {
                    assert!(rect.x >= 0.0 && rect.x + rect.width <= width, "{width}: {rect:?}");
                }
            }
            assert!(frame.regions.iter().all(|region| region.rect.x + region.rect.width <= width));
        }
    }

    #[test]
    fn a_row_is_one_target_and_a_control_in_a_cell_keeps_its_own() {
        let (hover, selected) = (Color::rgb(50, 51, 52), Color::rgb(70, 71, 72));
        let palette = ComponentPalette { hover, selected, ..palette() };
        let build = |declared: bool, selected: bool| {
            let more = IconButton::new("more:0", Icon::More, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .render();
            let row = TableRow::new("row:0", vec![text("Canción"), more]).selected(selected);
            let row = if declared { row.controls(["more:0"]) } else { row };
            let columns = vec![TableColumn::flexible("title", "Título", 80.0), TableColumn::fixed("more", "", 24.0)];
            tree_of(Table::new("t", columns, 300.0, palette).rows(vec![row]).render())
        };
        let size = Size::new(300.0, 200.0);
        let whole = Rect::new(0.0, 32.0, 300.0, 28.0);
        let mut tree = build(true, false);
        let frame = tree.layout(size);
        assert!(fills(&frame, palette.hover).is_empty() && fills(&frame, palette.selected).is_empty());
        // The control keeps the size the host gave it, centered in the row.
        let more = region(&frame, "more:0");
        assert_eq!(more, Rect::new(300.0 - 8.0 - 24.0, whole.y + 2.0, 24.0, 24.0));
        for (point, id) in [(Point::new(60.0, 44.0), "row:0"), (middle(more), "more:0")] {
            assert_eq!(frame.hit_test(point), Some(id));
            tree.handle_event(UiEvent::PointerMove(point));
            // One fill across the row, whichever of the two the pointer is over.
            assert!(fills(&tree.layout(size), palette.hover).contains(&whole), "{id}");
            assert_eq!(click(&mut tree, point), clicked(id));
        }
        // A control the row was not told about takes the hover with it.
        let mut tree = build(false, false);
        tree.layout(size);
        tree.handle_event(UiEvent::PointerMove(middle(more)));
        assert!(!fills(&tree.layout(size), palette.hover).contains(&whole));
        // The selected row is filled at rest, and under the pointer too.
        let mut tree = build(true, true);
        assert!(fills(&tree.layout(size), palette.selected).contains(&whole));
        tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 44.0)));
        let hovered = tree.layout(size);
        assert!(fills(&hovered, palette.selected).contains(&whole) && fills(&hovered, palette.hover).is_empty());
    }

    #[test]
    fn text_that_does_not_fit_is_cut_inside_its_column() {
        let mut columns = columns();
        columns[3] = TableColumn::fixed("added", "Fecha en que se agregó a la lista", 100.0).sortable();
        columns.push(TableColumn::fixed("plays", "Reproducciones totales", 60.0));
        let cells = vec![text("1"), text(LONG), text(LONG), text(LONG), text("3:45"), text(LONG)];
        let table = Table::new("t", columns, WIDTH, palette()).rows(vec![TableRow::new("row:0", cells)]);
        let frame = tree_of(table.render()).layout(SIZE);
        // 724 of room less 24, 160, 100, 48 and 60 leave the title 332.
        let columns = [(44.0, 332.0), (388.0, 160.0), (560.0, 100.0), (732.0, 60.0)];
        let cut: Vec<_> = texts(&frame).into_iter().filter(|line| line.0.ends_with('…')).collect();
        // A label that sorts, a passive one, and the four cells under them.
        assert_eq!(cut.len(), 6);
        for (text, position, _) in cut {
            let column = columns.into_iter().find(|column| column.0 == position.x);
            let (start, width) = column.unwrap_or_else(|| panic!("{text} starts no column"));
            let painted = text.chars().count() as f32 * 8.0;
            assert!(painted <= width && painted > width - 16.0, "{text} in {width} from {start}");
        }
        // The cells are clipped to the row as well.
        let row = region(&frame, "row:0");
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::PushClip(clip) if clip.y == row.y && clip.x == 8.0 && clip.x + clip.width == WIDTH - 8.0
        )));
    }

    #[test]
    fn a_flexible_cell_fills_its_column_and_a_missing_one_is_empty() {
        let mark = Color::rgb(1, 2, 3);
        let bar = Element::spacer().background(mark).size(None, Some(4.0)).flex(1.0);
        let rows = vec![TableRow::new("row:0", vec![text("1"), bar]), TableRow::new("row:1", Vec::new())];
        let frame = tree_of(Table::new("t", columns(), WIDTH, palette()).rows(rows).render()).layout(SIZE);
        // As wide as the title's column and centered in the row.
        assert_eq!(fills(&frame, mark), vec![Rect::new(44.0, 32.0 + 12.0, 404.0, 4.0)]);
        assert_eq!(region(&frame, "row:1").y, 62.0);
    }

    #[test]
    fn a_sticky_header_stays_at_the_top_while_the_rows_scroll_under_it() {
        let scrolled = |sticky: bool| {
            // Narrower than the window: the scroll area keeps a rail for its bar.
            let table = Table::new("t", columns(), 700.0, palette()).rows(rows(20));
            let table = if sticky { table.sticky_header() } else { table };
            // As on a page: something above the table, in the same scroll.
            let page = Element::column(vec![Element::spacer().size(None, Some(40.0)), table.render()]);
            let mut tree = UiTree::new(Element::scroll("page", page));
            tree.set_reduced_motion(true);
            let size = Size::new(WIDTH, 120.0);
            tree.layout(size);
            tree.handle_event(UiEvent::ScrollDirect { position: Point::new(5.0, 5.0), delta_y: -100.0 });
            tree.layout(size)
        };
        let frame = scrolled(true);
        let title = region(&frame, "t:sort:title");
        assert_eq!(title.y, 0.0);
        assert_eq!(frame.hit_test(middle(title)), Some("t:sort:title"));
        // Opaque, so the rows passing under it do not show through.
        assert!(fills(&frame, palette().surface).contains(&Rect::new(0.0, 0.0, 700.0, 28.0)));
        assert!(!has(&scrolled(false), "t:sort:title"), "a header that is not sticky scrolls away");
    }

    #[test]
    fn the_parts_stacked_by_the_host_are_the_table() {
        let build = || table(WIDTH).sticky_header().columns_menu(false, "Columnas");
        let (header, rows) = build().into_parts();
        assert_eq!(rows.len(), 3);
        assert!(header.sticky_top.is_some());
        let stacked = Element::column(vec![header, Element::column(rows).gap(TableSizing::COMPACT.row_gap)])
            .gap(TableSizing::COMPACT.header_gap)
            .size(Some(WIDTH), None);
        assert_eq!(tree_of(stacked).layout(SIZE), tree_of(build().render()).layout(SIZE));
    }

    #[test]
    fn bad_numbers_are_ignored() {
        let column = TableColumn::fixed("a", "A", f32::NAN).resizable(f32::INFINITY).hide_below(f32::NAN);
        assert_eq!((column.width, column.min_width, column.hide_below), (0.0, None, 0.0));
        let column = TableColumn::flexible("a", "A", -5.0).resizable(40.0).hide_below(-1.0);
        assert_eq!((column.width, column.min_width, column.hide_below), (0.0, None, 0.0));
        let sizing = TableSizing {
            header_height: f32::NAN,
            row_height: -4.0,
            row_gap: f32::INFINITY,
            inset: f32::NAN,
            gap: f32::NEG_INFINITY,
            caret_gap: f32::NAN,
            ..TableSizing::COMPACT
        };
        for width in [f32::NAN, f32::INFINITY, -10.0, 0.0, 300.0] {
            for sizing in [sizing, TableSizing::COMPACT] {
                let table = Table::new("t", columns(), width, palette())
                    .rows(rows(2))
                    .sizing(sizing)
                    .columns_menu(true, "Columnas")
                    .sort(Some(TableSort { key: "title".into(), descending: true }));
                let frame = tree_of(table.render()).layout(Size::new(300.0, 200.0));
                let finite =
                    |rect: Rect| [rect.x, rect.y, rect.width, rect.height].iter().all(|value| value.is_finite());
                assert!(frame.regions.iter().all(|region| finite(region.rect)));
                for command in &frame.commands {
                    match command {
                        DrawCommand::Text { position, .. } => {
                            assert!(position.x.is_finite() && position.y.is_finite());
                        }
                        DrawCommand::Icon { rect, .. } | DrawCommand::FillRect { rect, .. } => assert!(finite(*rect)),
                        _ => {}
                    }
                }
            }
        }
    }
}
