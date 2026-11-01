//! Pairs of a label and its value, for the details of one thing: an order, a server, a person.

use crate::{Component, ComponentPalette, ControlSize, Element, FontFamily, Separator, TextSpan};

/// Width of the label column unless set.
const DEFAULT_LABEL_WIDTH: f32 = 120.0;
const MAX_COLUMNS: usize = 4;
/// Room above and under what a row holds.
const PAD_Y: f32 = 4.0;
/// Room on each side of a hairline in a bordered list.
const PAD_X: f32 = 12.0;
/// Space between a label and the value beside it in a plain list.
const LABEL_GAP: f32 = 12.0;
/// Space between two columns of a plain list.
const COLUMN_GAP: f32 = 24.0;
/// Space between a label and the value under it.
const STACK_GAP: f32 = 2.0;
const HAIRLINE: f32 = 1.0;

/// The font size and the line height of a size.
const fn metrics(size: ControlSize) -> (f32, f32) {
    match size {
        ControlSize::Dense => (12.0, 16.0),
        ControlSize::Compact | ControlSize::Regular => (13.0, 18.0),
        ControlSize::Spacious => (14.0, 20.0),
    }
}

/// What stands for the value of a pair.
enum Value {
    Text(String),
    Element(Element),
}

/// One pair of a [`DescriptionList`]: a label and the value it names.
pub struct DescriptionItem {
    label: String,
    value: Value,
    span: usize,
}

impl DescriptionItem {
    /// A pair whose value is text, in the palette's text color. A long value
    /// wraps. A number given as text starts at the left edge like any other.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self { label: label.into(), value: Value::Text(value.into()), span: 1 }
    }

    /// A pair whose value is an element the host built: a chip, a link, a
    /// copy button. It keeps its own size, hit region, focus and actions,
    /// and starts at the left edge of the value's place. One that flexes is
    /// given the whole width of that place.
    pub fn with(label: impl Into<String>, value: Element) -> Self {
        Self { label: label.into(), value: Value::Element(value), span: 1 }
    }

    /// How many columns the pair takes, 1 unless set and at least 1. It is
    /// held to the number of columns the list has.
    pub fn span(mut self, columns: usize) -> Self {
        self.span = columns.max(1);
        self
    }
}

/// The host owns the pairs and passes them each frame. The list is passive:
/// it reports nothing, takes no pointer and adds no tab stop, and an element
/// given as a value works as it does outside the list.
pub struct DescriptionList {
    items: Vec<DescriptionItem>,
    palette: ComponentPalette,
    label_width: f32,
    stacked: bool,
    columns: usize,
    bordered: bool,
    dividers: bool,
    size: ControlSize,
    width: Option<f32>,
}

impl DescriptionList {
    /// The pairs in reading order, one to a row, each label beside its value,
    /// with a hairline between rows.
    pub fn new(items: Vec<DescriptionItem>, palette: ComponentPalette) -> Self {
        Self {
            items,
            palette,
            label_width: DEFAULT_LABEL_WIDTH,
            stacked: false,
            columns: 1,
            bordered: false,
            dividers: true,
            size: ControlSize::Regular,
            width: None,
        }
    }

    /// The width of the label column in logical pixels, 120 unless set. A
    /// label that does not fit is cut with an ellipsis. A negative width
    /// becomes zero and one that is not finite is ignored. A stacked list
    /// ignores it.
    pub fn label_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.label_width = width.max(0.0);
        }
        self
    }

    /// Put each label over its value instead of beside it, for long values
    /// or a narrow pane.
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }

    /// Lay the pairs in this many columns of equal width, from 1 to 4. The
    /// last pair of a row takes the columns the row has left.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, MAX_COLUMNS);
        self
    }

    /// Draw the list as an outlined surface, with hairlines between rows,
    /// between columns and between each label and its value.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Whether a plain list has a hairline between rows, which it has unless
    /// this is `false`. A bordered list always has them.
    pub fn dividers(mut self, dividers: bool) -> Self {
        self.dividers = dividers;
        self
    }

    /// The least height of a row, which is the size's height, and the size of
    /// the text: 12 for dense, 13 for compact and regular, 14 for spacious.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Fix the width of the list in logical pixels. Without one it fills the
    /// width its parent gives it. A negative width becomes zero and one that
    /// is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// The items in rows, each with the columns it takes. A pair that does
    /// not fit in what its row has left starts the next, and the last pair
    /// of a row takes what is left of it.
    fn rows(items: Vec<DescriptionItem>, columns: usize) -> Vec<Vec<(DescriptionItem, usize)>> {
        let mut rows: Vec<Vec<(DescriptionItem, usize)>> = Vec::new();
        let mut used = columns;
        for item in items {
            let span = item.span.min(columns);
            if used + span > columns {
                Self::fill(rows.last_mut(), columns - used);
                rows.push(Vec::new());
                used = 0;
            }
            used += span;
            rows.last_mut().expect("a row was started").push((item, span));
        }
        Self::fill(rows.last_mut(), columns - used);
        rows
    }

    fn fill(row: Option<&mut Vec<(DescriptionItem, usize)>>, left: usize) {
        if let Some((_, span)) = row.and_then(|row| row.last_mut()) {
            *span += left;
        }
    }
}

/// `content` with room above and under it, and `left` and `right` of it.
fn padded(content: Element, left: f32, right: f32) -> Element {
    Element::row(vec![
        Element::spacer().size(Some(left), None),
        Element::column(vec![
            Element::spacer().size(None, Some(PAD_Y)),
            content,
            Element::spacer().size(None, Some(PAD_Y)),
        ])
        .flex(1.0),
        Element::spacer().size(Some(right), None),
    ])
}

impl Component for DescriptionList {
    fn render(self) -> Element {
        let palette = self.palette;
        let (font_size, line) = metrics(self.size);
        // What a row holds is at least this high, so a row is the size's height.
        let inner = self.size.height() - PAD_Y * 2.0;
        // Text of one line is centered in a row by this much above and under it.
        let lead = ((inner - line) / 2.0).max(0.0);
        let (bordered, stacked, label_width) = (self.bordered, self.stacked, self.label_width);
        let label = |text: String| {
            // Plain text leaves its line height to the host's font, so the
            // label is centered in a line of the value's height.
            Element::truncated_text(text)
                .font_family(FontFamily::Sans)
                .font_size(font_size)
                .foreground(palette.muted)
                .centered_y()
                .size(None, Some(line))
        };
        // `lead` is the room that centers one line in a row; a stacked pair has none.
        let value = |value: Value, lead: f32| match value {
            Value::Text(text) => Element::column(vec![
                Element::spacer().size(None, Some(lead)),
                Element::formatted_text(vec![TextSpan::new(text, palette.text, font_size, FontFamily::Sans)])
                    .font_size(font_size)
                    .line_height(line),
                Element::spacer().size(None, Some(lead)),
            ]),
            Value::Element(element) => {
                // An element keeps its measured width, as in a row of its own;
                // a column would stretch a chip across the value's place.
                let element = if element.style.flex > 0.0 {
                    element
                } else {
                    Element::row(vec![element, Element::spacer().flex(1.0)])
                };
                // Centered on the first line when it is lower than a row, at the top when higher.
                Element::column(vec![Element::spacer().flex(1.0), element, Element::spacer().flex(1.0)])
                    .min_size(0.0, if lead > 0.0 { inner } else { 0.0 })
            }
        };
        let cell = |item: DescriptionItem, span: usize, first: bool| {
            // A plain list has no rule between columns: the room is inside the cell,
            // so the edges of the cells stand on the columns whatever their spans.
            let (left, right) = match (bordered, first) {
                (true, _) => (PAD_X, PAD_X),
                (false, true) => (0.0, 0.0),
                (false, false) => (COLUMN_GAP, 0.0),
            };
            let mut parts = Vec::new();
            if bordered && !first {
                parts.push(Separator::vertical(palette).render());
            }
            if stacked && bordered {
                parts.push(
                    Element::column(vec![
                        padded(label(item.label), left, right),
                        Separator::horizontal(palette).render(),
                        padded(value(item.value, 0.0), left, right),
                    ])
                    .flex(1.0),
                );
            } else if stacked {
                let pair = Element::column(vec![label(item.label), value(item.value, 0.0)]).gap(STACK_GAP);
                parts.push(padded(pair, left, right).flex(1.0));
            } else {
                let name = Element::column(vec![Element::spacer().size(None, Some(lead)), label(item.label)]);
                let (between, after) = if bordered { (right, PAD_X) } else { (0.0, LABEL_GAP) };
                parts.push(padded(name, left, between).size(Some(left + label_width + between), None));
                if bordered {
                    parts.push(Separator::vertical(palette).render());
                }
                parts.push(padded(value(item.value, lead), after, right).flex(1.0));
            }
            Element::row(parts).flex(span as f32)
        };
        let mut lines = Vec::new();
        for row in Self::rows(self.items, self.columns) {
            if !lines.is_empty() && (bordered || self.dividers) {
                lines.push(Separator::horizontal(palette).render());
            }
            let cells = row.into_iter().enumerate().map(|(index, (item, span))| cell(item, span, index == 0));
            lines.push(Element::row(cells.collect()));
        }
        let list = Element::column(lines);
        if bordered {
            Element::panel(list)
                .background(palette.surface)
                .border(palette.border, HAIRLINE)
                .radius(crate::tokens().radius.card)
                .clip_children()
        } else {
            list
        }
        .size(self.width, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Access, Chip, DrawCommand, Frame, Key, Link, Point, Rect, Role, Size, UiAction, UiEvent, UiTree};

    const WIDTH: f32 = 600.0;
    const LONG: &str = "A very long piece of text that goes on well past the width any narrow pane could give to it";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn pairs() -> Vec<DescriptionItem> {
        vec![
            DescriptionItem::new("Name", "atlas-01"),
            DescriptionItem::new("Region", "eu-west"),
            DescriptionItem::new("Memory", "16 GB"),
        ]
    }

    fn tree_of(list: DescriptionList, width: f32) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![list.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(width, 600.0));
        tree
    }

    fn frame_of(list: DescriptionList, width: f32) -> Frame {
        tree_of(list, width).frame().clone()
    }

    /// Where each painted line that starts with `text` is, with its ink and size.
    fn painted(frame: &Frame, start: &str) -> Vec<(Point, crate::Color, f32)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, color, font_size, .. } if text.starts_with(start) => {
                    Some((*position, *color, *font_size))
                }
                _ => None,
            })
            .collect()
    }

    fn at(frame: &Frame, text: &str) -> Point {
        painted(frame, text).first().unwrap_or_else(|| panic!("{text} is painted")).0
    }

    /// The hairlines: rules across, then rules down.
    fn rules(frame: &Frame) -> (Vec<Rect>, Vec<Rect>) {
        let fills = frame.commands.iter().filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == palette().border => Some(*rect),
            _ => None,
        });
        fills.partition(|rect| rect.height == HAIRLINE)
    }

    #[test]
    fn a_label_stands_beside_its_value_in_a_column_of_the_label_width() {
        for label_width in [DEFAULT_LABEL_WIDTH, 200.0] {
            let frame = frame_of(DescriptionList::new(pairs(), palette()).label_width(label_width), WIDTH);
            for (label, value) in [("Name", "atlas-01"), ("Region", "eu-west"), ("Memory", "16 GB")] {
                let (name, said) = (at(&frame, label), at(&frame, value));
                assert_eq!((name.x, said.x), (0.0, label_width + LABEL_GAP), "{label}");
                assert!((name.y - said.y).abs() < 3.0, "{label} is on the line of {value}");
            }
            assert!(at(&frame, "Name").y < at(&frame, "Region").y && at(&frame, "Region").y < at(&frame, "Memory").y);
        }
    }

    #[test]
    fn labels_are_muted_and_values_are_in_the_text_color() {
        let frame = frame_of(DescriptionList::new(pairs(), palette()), WIDTH);
        assert_eq!(painted(&frame, "Region")[0].1, palette().muted);
        assert_eq!(painted(&frame, "eu-west")[0].1, palette().text);
    }

    #[test]
    fn stacked_puts_the_label_over_its_value_at_the_same_edge() {
        for bordered in [false, true] {
            let frame = frame_of(DescriptionList::new(pairs(), palette()).stacked().bordered(bordered), WIDTH);
            let (name, said) = (at(&frame, "Region"), at(&frame, "eu-west"));
            assert_eq!(name.x, said.x, "bordered {bordered}");
            assert!(said.y > name.y + 10.0, "bordered {bordered}");
            assert!(at(&frame, "Memory").y > said.y);
        }
    }

    #[test]
    fn columns_are_of_equal_width_and_a_span_takes_several() {
        let items = || {
            vec![
                DescriptionItem::new("One", "1"),
                DescriptionItem::new("Two", "2"),
                DescriptionItem::new("Three", "3"),
                DescriptionItem::new("Wide", "4").span(2),
                DescriptionItem::new("Five", "5"),
                DescriptionItem::new("Whole", "6").span(9),
            ]
        };
        for bordered in [false, true] {
            let frame = frame_of(DescriptionList::new(items(), palette()).columns(3).bordered(bordered), WIDTH);
            // What stands before a label in its cell: the room between columns, or the rule and its room.
            let (first, later) = if bordered { (PAD_X, HAIRLINE + PAD_X) } else { (0.0, COLUMN_GAP) };
            let column = WIDTH / 3.0;
            for (label, index, row) in [("One", 0, 0), ("Two", 1, 0), ("Three", 2, 0), ("Wide", 0, 1), ("Five", 2, 1), ("Whole", 0, 2)]
            {
                let point = at(&frame, label);
                let edge = column * index as f32 + if index == 0 { first } else { later };
                assert!((point.x - edge).abs() < 0.01, "{label}: {} and not {edge}, bordered {bordered}", point.x);
                assert_eq!(point.y > at(&frame, "One").y, row > 0, "{label}");
                assert_eq!(point.y > at(&frame, "Wide").y, row > 1, "{label}");
            }
        }
    }

    #[test]
    fn a_pair_that_does_not_fit_starts_a_row_and_the_last_of_a_row_takes_what_is_left() {
        let spans = |spans: &[usize], columns: usize| -> Vec<Vec<usize>> {
            let items = spans.iter().map(|span| DescriptionItem::new("a", "b").span(*span)).collect();
            let rows = DescriptionList::rows(items, DescriptionList::new(Vec::new(), palette()).columns(columns).columns);
            rows.iter().map(|row| row.iter().map(|(_, span)| *span).collect()).collect()
        };
        assert_eq!(spans(&[1, 1, 1], 1), [[1], [1], [1]]);
        assert_eq!(spans(&[1, 1, 1], 2), [vec![1, 1], vec![2]]);
        assert_eq!(spans(&[1, 2, 1], 2), [vec![2], vec![2], vec![2]]);
        assert_eq!(spans(&[2, 2, 3, 1], 4), [vec![2, 2], vec![3, 1]]);
        assert_eq!(spans(&[1, 3, 1], 3), [vec![3], vec![3], vec![3]]);
        // A span of none is one, and the columns are held between 1 and 4.
        assert_eq!(spans(&[0, 0], 0), [[1], [1]]);
        assert_eq!(spans(&[1, 1, 1, 1, 1], 9), [vec![1, 1, 1, 1], vec![4]]);
        assert_eq!(spans(&[usize::MAX], 3), [[3]]);
        assert!(spans(&[], 3).is_empty());
    }

    #[test]
    fn a_plain_list_has_a_rule_between_rows_unless_dividers_are_off() {
        let frame = frame_of(DescriptionList::new(pairs(), palette()), WIDTH);
        let (across, down) = rules(&frame);
        assert_eq!((across.len(), down.len()), (2, 0));
        assert!(across.iter().all(|rule| (rule.x, rule.width) == (0.0, WIDTH)));
        assert!(across[0].y > at(&frame, "Name").y && across[0].y < at(&frame, "Region").y);
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
        let bare = frame_of(DescriptionList::new(pairs(), palette()).dividers(false), WIDTH);
        assert_eq!(rules(&bare), (Vec::new(), Vec::new()));
        // The rules take a pixel each and move nothing else.
        assert_eq!(at(&bare, "Memory").y + 2.0 * HAIRLINE, at(&frame, "Memory").y);
    }

    #[test]
    fn a_bordered_list_is_an_outlined_surface_ruled_between_rows_columns_and_label_and_value() {
        let list = || DescriptionList::new(pairs(), palette()).bordered(true).dividers(false);
        let frame = frame_of(list().columns(2), WIDTH);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { rect, color, width, radius }
                if *color == palette().border && *width == HAIRLINE && *radius == crate::tokens().radius.card && rect.width == WIDTH
        )));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. } if *color == palette().surface && rect.width == WIDTH
        )));
        // Two rows: one rule across. Down: after each of the three labels, and between the two columns.
        let (across, down) = rules(&frame);
        assert_eq!((across.len(), down.len()), (1, 4));
        let after_label = PAD_X + DEFAULT_LABEL_WIDTH + PAD_X;
        let mut edges: Vec<f32> = down.iter().map(|rule| rule.x).collect();
        edges.sort_by(f32::total_cmp);
        assert_eq!(edges, [after_label, after_label, WIDTH / 2.0, WIDTH / 2.0 + HAIRLINE + after_label]);
        assert_eq!(at(&frame, "Name").x, PAD_X);
        assert_eq!(at(&frame, "atlas-01").x, after_label + HAIRLINE + PAD_X);
        // Stacked, the rule between a label and its value runs across.
        let (across, down) = rules(&frame_of(list().stacked(), WIDTH));
        assert_eq!((across.len(), down.len()), (5, 0));
    }

    #[test]
    fn the_size_sets_the_height_of_a_row_and_the_size_of_the_text() {
        for (size, font) in [
            (ControlSize::Dense, 12.0),
            (ControlSize::Compact, 13.0),
            (ControlSize::Regular, 13.0),
            (ControlSize::Spacious, 14.0),
        ] {
            let frame = frame_of(DescriptionList::new(pairs(), palette()).size(size).dividers(false), WIDTH);
            assert_eq!(at(&frame, "Region").y - at(&frame, "Name").y, size.height(), "{size:?}");
            assert_eq!((painted(&frame, "Name")[0].2, painted(&frame, "atlas-01")[0].2), (font, font), "{size:?}");
            // A chip no higher than what a row holds leaves the row its height.
            let chip = || DescriptionItem::with("State", Chip::new("Running", palette()).height(16.0).render());
            let items = vec![chip(), DescriptionItem::new("After", "x")];
            let frame = frame_of(DescriptionList::new(items, palette()).size(size).dividers(false), WIDTH);
            assert_eq!(at(&frame, "After").y - at(&frame, "State").y, size.height(), "{size:?}");
        }
    }

    #[test]
    fn a_long_value_wraps_and_a_long_label_is_cut_with_an_ellipsis_at_a_narrow_width() {
        for (stacked, bordered) in [(false, false), (false, true), (true, false), (true, true)] {
            let list = |items| {
                let list = DescriptionList::new(items, palette()).bordered(bordered).width(260.0);
                if stacked { list.stacked() } else { list }
            };
            let case = format!("stacked {stacked}, bordered {bordered}");
            let frame = frame_of(list(vec![DescriptionItem::new("Note", LONG), DescriptionItem::new("After", "x")]), 260.0);
            let lines: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { position, text, color, .. } if *color == palette().text && text != "x" => {
                        Some((*position, text.clone()))
                    }
                    _ => None,
                })
                .collect();
            assert!(lines.len() > 1, "{case}: {lines:?}");
            let whole: String = lines.iter().map(|(_, text)| text.trim()).collect::<Vec<_>>().join(" ");
            assert_eq!(whole, LONG, "{case}");
            assert!(lines.iter().all(|(point, _)| point.x == lines[0].0.x), "{case}");
            // The next row starts under the last line.
            assert!(at(&frame, "After").y > lines.last().unwrap().0.y, "{case}");

            let frame = frame_of(list(vec![DescriptionItem::new(LONG, "short")]), 260.0);
            let labels: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, color, .. } if *color == palette().muted => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let [cut] = labels.as_slice() else {
                panic!("{case}: one line, got {labels:?}");
            };
            assert!(cut.ends_with('…') && cut.chars().count() < LONG.chars().count(), "{case}: {cut}");
            assert_eq!(painted(&frame, "short").len(), 1, "{case}");
        }
    }

    #[test]
    fn a_number_given_as_text_starts_where_any_other_value_does() {
        let items = vec![DescriptionItem::new("Total", "1,204.50"), DescriptionItem::new("Paid", "yes")];
        let frame = frame_of(DescriptionList::new(items, palette()), WIDTH);
        assert_eq!(at(&frame, "1,204.50").x, at(&frame, "yes").x);
    }

    #[test]
    fn the_width_fixes_the_list_and_bad_numbers_change_nothing() {
        let frame = frame_of(DescriptionList::new(pairs(), palette()).width(300.0), WIDTH);
        assert!(rules(&frame).0.iter().all(|rule| rule.width == 300.0));
        let plain = frame_of(DescriptionList::new(pairs(), palette()), WIDTH).commands;
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let list = DescriptionList::new(pairs(), palette()).width(bad).label_width(bad);
            assert_eq!(frame_of(list, WIDTH).commands, plain, "{bad}");
        }
        // A negative number is zero: no list at all, or values that start after the gap alone.
        let frame = frame_of(DescriptionList::new(pairs(), palette()).label_width(-40.0), WIDTH);
        assert_eq!(at(&frame, "atlas-01").x, LABEL_GAP);
        let frame = frame_of(DescriptionList::new(pairs(), palette()).width(-40.0), WIDTH);
        assert!(rules(&frame).0.iter().all(|rule| rule.width == 0.0));
    }

    #[test]
    fn the_list_takes_no_pointer_and_no_focus_and_tells_the_platform_nothing() {
        for bordered in [false, true] {
            let mut tree = tree_of(DescriptionList::new(pairs(), palette()).columns(2).bordered(bordered), WIDTH);
            assert!(tree.frame().regions.is_empty());
            assert!(tree.frame().access.is_empty());
            let point = Point::new(20.0, 12.0);
            tree.handle_event(UiEvent::PointerMove(point));
            assert_eq!(tree.hovered_id(), None);
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
            assert_eq!(tree.focused_id(), None);
        }
    }

    #[test]
    fn a_link_given_as_a_value_still_reports_by_pointer_and_keyboard_and_keeps_its_node() {
        for (stacked, bordered) in [(false, false), (false, true), (true, false), (true, true)] {
            let items = vec![
                DescriptionItem::new("Name", "atlas-01"),
                DescriptionItem::with("Repository", Link::new("open-repo", "limn", palette()).render()),
                DescriptionItem::with("State", Chip::new("Running", palette()).render()),
                DescriptionItem::with(
                    "Owner",
                    Element::text("Ada").accessible(Access::new(Role::Label).id("owner").name("Ada")),
                ),
            ];
            let list = DescriptionList::new(items, palette()).bordered(bordered);
            let mut tree = tree_of(if stacked { list.stacked() } else { list }, WIDTH);
            let frame = tree.frame().clone();
            // The link is the only target, and it is as wide as its label, not as its cell.
            let [region] = frame.regions.as_slice() else {
                panic!("one region, got {:?}", frame.regions);
            };
            assert_eq!(region.id, "open-repo");
            assert!(region.rect.width < 60.0, "{}", region.rect.width);
            // What a value says of itself reaches the platform; the list adds nothing.
            let said: Vec<_> = frame.access.iter().map(|node| (node.id.as_str(), node.role, node.name.as_str())).collect();
            assert_eq!(said, [("open-repo", Role::Link, "limn"), ("owner", Role::Label, "Ada")]);
            let point = Point::new(region.rect.x + region.rect.width / 2.0, region.rect.y + region.rect.height / 2.0);
            tree.handle_event(UiEvent::PointerMove(point));
            assert_eq!(tree.hovered_id(), Some("open-repo"));
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: "open-repo".into() }]);
            // Tab reaches it, and it alone.
            tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
            assert_eq!(tree.focused_id(), Some("open-repo"));
            for key in [Key::Enter, Key::Space] {
                assert_eq!(
                    tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false }),
                    vec![UiAction::Clicked { id: "open-repo".into() }]
                );
            }
        }
    }

    #[test]
    fn a_list_of_no_pairs_paints_nothing() {
        for bordered in [false, true] {
            let frame = frame_of(DescriptionList::new(Vec::new(), palette()).bordered(bordered).dividers(false), WIDTH);
            assert!(painted(&frame, "").is_empty());
            assert_eq!(rules(&frame), (Vec::new(), Vec::new()));
        }
    }
}
