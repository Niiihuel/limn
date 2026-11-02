//! Numbered pages between a previous and a next arrow: the way through a
//! long list that is shown one page at a time.

use crate::{
    Access, Button, ButtonSizing, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily,
    Icon, IconButton, Role, Text, TextRole, TooltipSpec,
};

/// Space between two cells of the row.
const GAP: f32 = 4.0;
/// Space between a number and the sides of its cell.
const PADDING: f32 = 4.0;

/// What stands between the arrows: each page shown, and `None` where pages
/// are left out. Past `2 * siblings + 5` pages the count is always that, so
/// a cell keeps its place whatever the current page.
fn cells(page: usize, pages: usize, siblings: usize) -> Vec<Option<usize>> {
    let window = siblings.saturating_mul(2).saturating_add(1);
    if pages <= window.saturating_add(4) {
        return (1..=pages).map(Some).collect();
    }
    // The window slides with the page and stops two short of each end, where
    // the cell beside it holds the one page left instead of an ellipsis.
    let start = page.saturating_sub(siblings).clamp(3, pages - 1 - window);
    let end = start + window - 1;
    let mut cells = vec![Some(1), (start == 3).then_some(2)];
    cells.extend((start..=end).map(Some));
    cells.extend([(end == pages - 2).then_some(pages - 1), Some(pages)]);
    cells
}

/// The host owns the position: it passes the current page and the number of
/// pages, and the row reports `UiAction::Clicked` with `{id}={page}` for the
/// page a number or an arrow leads to. [`Pagination::page_of`] reads that
/// page back. The first and the last page always show, with the current one
/// and its neighbours between them and an ellipsis where pages are left out.
/// For a position without numbers to jump to, such as "change 2 of 5", use a
/// [`Pager`](crate::Pager).
pub struct Pagination {
    id: String,
    page: usize,
    pages: usize,
    siblings: usize,
    compact: bool,
    size: ControlSize,
    enabled: bool,
    labels: (String, String, String),
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl Pagination {
    /// A row over `pages` pages with `page` the current one, counting from
    /// 1: a page below 1 is read as the first and one past the end as the
    /// last. With no pages the row is the two arrows alone, both disabled,
    /// around "0 / 0" if it is compact.
    /// `id` must be unique in the window; the buttons take their IDs from it.
    pub fn new(id: impl Into<String>, page: usize, pages: usize, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            page: page.clamp(1, pages.max(1)),
            pages,
            siblings: 1,
            compact: false,
            size: ControlSize::Regular,
            enabled: true,
            labels: ("Previous page".into(), "Next page".into(), "Page".into()),
            accessibility_label: None,
            palette,
        }
    }

    /// The page an action of the row with this `id` asks for: `None` for any
    /// other action, so it can be tried on every click. The page is one the
    /// row showed, from 1 to its number of pages.
    pub fn page_of(action_id: &str, id: &str) -> Option<usize> {
        let page = action_id.strip_prefix(id)?.strip_prefix('=')?.parse().ok()?;
        (page > 0).then_some(page)
    }

    /// How many pages show on each side of the current one; 1 unless set.
    /// The row has `2 * siblings + 5` cells between its arrows, and every
    /// page when there are no more than that.
    pub fn siblings(mut self, siblings: usize) -> Self {
        self.siblings = siblings;
        self
    }

    /// Show the two arrows around the position as text, "3 / 12", in place
    /// of the numbers.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// The height of the arrows and the numbers: 24, 28, 32 or 36 pixels.
    /// A number's cell is that wide, or wider where the last page has more
    /// than two digits.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A disabled row is passive: every button keeps its place in the muted
    /// color, the current page still marked, and none has a hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// The words of the row, for a host that is not in English: the hint and
    /// name of each arrow, and the word a number is named with for the
    /// platform, said before it. They are "Previous page", "Next page" and
    /// "Page" unless set: the library's labels have no words for them.
    pub fn labels(mut self, previous: impl Into<String>, next: impl Into<String>, page: impl Into<String>) -> Self {
        self.labels = (previous.into(), next.into(), page.into());
        self
    }

    /// What the row is called to the platform, such as "Results pages". With
    /// it the row is a group of its own, known by the row's ID; without it
    /// only the buttons are told.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }
}

impl Component for Pagination {
    fn render(self) -> Element {
        let palette = self.palette;
        let (id, page, pages) = (self.id, self.page, self.pages);
        let (previous_label, next_label, page_label) = self.labels;
        let height = self.size.height();
        let sizing = ButtonSizing { horizontal_padding: PADDING, ..self.size.button_sizing() };
        // The row must not change width with the page, so every number has the
        // room of the widest one. shortcut: a digit is taken as 0.62 of the
        // font size, which holds for the usual sans faces; measure the text if
        // a host's font is wider.
        let digits = pages.to_string().len() as f32;
        let digit = (sizing.font_size * 0.62).ceil();
        let cell = height.max(digits * digit + PADDING * 2.0);

        let arrow = |end: &str, icon: Icon, label: String, target: usize, enabled: bool| {
            IconButton::new(format!("{id}:{end}"), icon, palette)
                .variant(ButtonVariant::Ghost)
                .size(self.size)
                .tooltip_with(TooltipSpec::new(label))
                .dispatch_as(format!("{id}={target}"))
                .enabled(enabled)
                .render()
        };
        let mut children = vec![arrow("previous", Icon::ArrowLeft, previous_label, page - 1, self.enabled && page > 1)];
        if self.compact {
            // Two numbers of the widest kind, and " / " between them.
            let width = (digits * 2.0 + 2.0) * digit + PADDING * 2.0;
            let label = Text::new(format!("{} / {pages}", page.min(pages)), palette).role(TextRole::Small).muted().render();
            children.push(label.center_content().centered_y().size(Some(width), Some(height)));
        } else {
            children.extend(cells(page, pages, self.siblings).into_iter().map(|shown| {
                let Some(number) = shown else {
                    // Three dots on the middle line, where a typed ellipsis sits on the baseline.
                    let side = sizing.icon_size;
                    let dots = Element::icon(Icon::More).foreground(palette.muted).size(Some(side), Some(side));
                    return Element::row(vec![Element::spacer().flex(1.0), dots.centered_y(), Element::spacer().flex(1.0)])
                        .size(Some(cell), Some(height));
                };
                let current = number == page;
                let name = format!("{page_label} {number}");
                if !self.enabled {
                    // A disabled `Button` of a fixed width starts its label at
                    // the left, so the passive cell is drawn here, centered
                    // as the live one is.
                    let fill = if current { palette.raised } else { Color::TRANSPARENT };
                    let label = Element::text(number.to_string())
                        .font_family(FontFamily::Sans)
                        .font_size(sizing.font_size)
                        .foreground(palette.muted)
                        .center_content()
                        .centered_y()
                        .size(Some(cell), None);
                    let access = Access::new(Role::Button).id(format!("{id}:{number}")).name(name);
                    return Element::panel(label)
                        .background(fill)
                        .radius(crate::tokens().radius.control)
                        .size(Some(cell), Some(height))
                        .accessible(access.selected(current).disabled(true));
                }
                // The current page stays a button: activating a number by the
                // keyboard must not take the focus from under the user.
                Button::new(format!("{id}:{number}"), number.to_string(), palette)
                    .variant(if current { ButtonVariant::Default } else { ButtonVariant::Ghost })
                    .sizing(sizing)
                    .width(cell)
                    .dispatch_as(format!("{id}={number}"))
                    .accessibility_label(name)
                    .render()
                    .accessible(Access::default().selected(current))
            }));
        }
        children.push(arrow("next", Icon::ArrowRight, next_label, page + 1, self.enabled && page < pages));
        let row = Element::row(children).gap(GAP);
        match self.accessibility_label {
            Some(name) => row.accessible(Access::new(Role::Group).id(id).name(name)),
            None => row,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(900.0, 200.0)
    }

    fn pagination(page: usize, pages: usize) -> Pagination {
        Pagination::new("results", page, pages, palette())
    }

    fn tree_of(pagination: Pagination) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![pagination.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        tree
    }

    fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false })
    }

    fn clicked(id: &str) -> Vec<UiAction> {
        vec![UiAction::Clicked { id: id.into() }]
    }

    fn button(frame: &Frame, id: &str) -> Option<Rect> {
        frame.regions.iter().find(|region| region.id == id && region.kind == HitKind::Button).map(|region| region.rect)
    }

    /// The buttons of the frame, without the regions of the arrows' hints.
    fn buttons(frame: &Frame) -> Vec<(&str, Rect)> {
        frame
            .regions
            .iter()
            .filter(|region| region.kind == HitKind::Button)
            .map(|region| (region.id.as_str(), region.rect))
            .collect()
    }

    fn click(tree: &mut UiTree, rect: Rect) -> Vec<UiAction> {
        let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        tree.handle_event(UiEvent::PointerUp(center))
    }

    /// Every text of the frame, with where it starts and its color.
    fn texts(frame: &Frame) -> Vec<(String, Point, crate::Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, color, .. } => Some((text.clone(), *position, *color)),
                _ => None,
            })
            .collect()
    }

    /// The row as it reads: numbers, and "…" where pages are left out.
    fn shown(page: usize, pages: usize, siblings: usize) -> String {
        cells(page, pages, siblings)
            .iter()
            .map(|cell| cell.map_or("…".to_owned(), |number| number.to_string()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn the_first_the_last_and_the_neighbours_of_the_current_page_show_around_ellipses() {
        assert_eq!(shown(1, 12, 1), "1 2 3 4 5 … 12");
        assert_eq!(shown(4, 12, 1), "1 2 3 4 5 … 12");
        assert_eq!(shown(5, 12, 1), "1 … 4 5 6 … 12");
        assert_eq!(shown(8, 12, 1), "1 … 7 8 9 … 12");
        assert_eq!(shown(9, 12, 1), "1 … 8 9 10 11 12");
        assert_eq!(shown(12, 12, 1), "1 … 8 9 10 11 12");
        assert_eq!(shown(6, 12, 0), "1 … 6 … 12");
        assert_eq!(shown(6, 12, 2), "1 … 4 5 6 7 8 … 12");
        // Few enough pages all show, and an ellipsis never stands for one page.
        assert_eq!(shown(4, 7, 1), "1 2 3 4 5 6 7");
        assert_eq!(shown(1, 1, 1), "1");
        assert_eq!(shown(1, 0, 1), "");
        assert_eq!(shown(3, 5, usize::MAX), "1 2 3 4 5");
        assert_eq!(shown(usize::MAX, usize::MAX, 1).split(' ').count(), 7);
    }

    #[test]
    fn the_row_has_the_same_cells_in_the_same_places_whatever_the_current_page() {
        for siblings in 0..4 {
            for pages in [1, 2, 7, 8, 12, 40] {
                let count = pages.min(2 * siblings + 5);
                for page in 1..=pages {
                    let cells = cells(page, pages, siblings);
                    assert_eq!(cells.len(), count, "page {page} of {pages}, siblings {siblings}");
                    assert_eq!((cells[0], cells[count - 1]), (Some(1), Some(pages)));
                    assert!(cells.contains(&Some(page)));
                }
            }
        }
        // In the frame: the next arrow never moves, and every button is in one of the row's slots.
        let next = |page| button(&tree_of(pagination(page, 120)).layout(window()), "results:next");
        let cell = 35.0;
        assert_eq!(next(1), Some(Rect::new(32.0 + GAP + 7.0 * (cell + GAP), 0.0, 32.0, 32.0)));
        for page in 1..=120 {
            let frame = tree_of(pagination(page, 120)).layout(window());
            assert_eq!(button(&frame, "results:next").or(next(1)), next(1), "page {page}");
            for (id, rect) in buttons(&frame).into_iter().filter(|(id, _)| !id.ends_with("previous")) {
                let slot = (rect.x - 32.0 - GAP) / (cell + GAP);
                assert_eq!(slot.fract(), 0.0, "{id} on page {page}");
            }
        }
    }

    #[test]
    fn the_current_page_is_filled_with_the_text_color_and_written_in_the_surface_color() {
        let frame = tree_of(pagination(5, 12)).layout(window());
        let current = button(&frame, "results:5").unwrap();
        let fills: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } if color.a > 0 => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        assert_eq!(fills, [(current, palette().text, crate::tokens().radius.control)]);
        let texts = texts(&frame);
        assert_eq!(texts.iter().map(|text| text.0.as_str()).collect::<Vec<_>>(), ["1", "4", "5", "6", "12"]);
        for (text, position, color) in &texts {
            assert_eq!(*color, if text == "5" { palette().surface } else { palette().text }, "{text}");
            // Each number is centered in its cell.
            let rect = button(&frame, &format!("results:{text}")).unwrap();
            let width = text.chars().count() as f32 * 6.0;
            assert_eq!(position.x, rect.x + (rect.width - width) / 2.0, "{text}");
        }
        // Each ellipsis is three muted dots in the middle of its cell: the second and the sixth.
        let dots: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { rect, icon: Icon::More, tint } => Some((*rect, *tint)),
                _ => None,
            })
            .collect();
        let slot = |index: f32| Rect::new(32.0 + GAP + index * (32.0 + GAP) + 8.0, 8.0, 16.0, 16.0);
        assert_eq!(dots, [(slot(1.0), palette().muted), (slot(5.0), palette().muted)]);
    }

    #[test]
    fn a_click_on_a_number_or_an_arrow_reports_the_page_it_leads_to() {
        let mut tree = tree_of(pagination(5, 12));
        let frame = tree.layout(window());
        for (id, page) in [
            ("results:previous", 4),
            ("results:1", 1),
            ("results:4", 4),
            ("results:5", 5),
            ("results:6", 6),
            ("results:12", 12),
            ("results:next", 6),
        ] {
            let actions = click(&mut tree, button(&frame, id).unwrap_or_else(|| panic!("no button {id}")));
            assert_eq!(actions, clicked(&format!("results={page}")));
            let UiAction::Clicked { id } = &actions[0] else { unreachable!() };
            assert_eq!(Pagination::page_of(id, "results"), Some(page));
        }
        assert_eq!(buttons(&frame).len(), 7, "the ellipses are not buttons");
    }

    #[test]
    fn page_of_reads_only_the_actions_of_its_own_row() {
        assert_eq!(Pagination::page_of("results=3", "results"), Some(3));
        for other in ["results", "results=", "results=0", "results=-1", "results=x", "results:3", "results-b=3", "b=3"] {
            assert_eq!(Pagination::page_of(other, "results"), None, "{other}");
        }
    }

    #[test]
    fn tab_reaches_each_button_and_enter_and_space_report_its_page() {
        let mut tree = tree_of(pagination(5, 12));
        tree.layout(window());
        for (id, page) in [("previous", 4), ("1", 1), ("4", 4), ("5", 5), ("6", 6), ("12", 12), ("next", 6)] {
            key(&mut tree, Key::Tab);
            assert_eq!(tree.focused_id(), Some(format!("results:{id}").as_str()));
            assert_eq!(key(&mut tree, Key::Enter), clicked(&format!("results={page}")));
            assert_eq!(key(&mut tree, Key::Space), clicked(&format!("results={page}")));
        }
        // The number that was activated keeps the focus once it is the current page.
        assert!(tree.focus("results:6"));
        tree.set_root(Element::column(vec![pagination(6, 12).render()]));
        tree.layout(window());
        assert_eq!(tree.focused_id(), Some("results:6"));
        assert_eq!(key(&mut tree, Key::Enter), clicked("results=6"));
    }

    #[test]
    fn previous_and_next_are_passive_at_the_ends_and_keep_their_place_and_their_name() {
        let middle = tree_of(pagination(5, 12)).layout(window());
        for (page, end, other) in [(1, "results:previous", "results:next"), (12, "results:next", "results:previous")] {
            let mut tree = tree_of(pagination(page, 12));
            let frame = tree.layout(window());
            assert!(button(&frame, end).is_none());
            assert!(button(&frame, other).is_some());
            assert!(!tree.focus(end));
            assert!(click(&mut tree, button(&middle, end).unwrap()).is_empty());
            let node = frame.access.iter().find(|node| node.id == end).unwrap();
            assert_eq!((node.role, node.disabled, node.actionable), (Role::Button, true, false));
            assert_eq!(node.rect, button(&middle, end).unwrap());
            assert_eq!(node.name, if page == 1 { "Previous page" } else { "Next page" });
        }
    }

    #[test]
    fn a_page_out_of_range_is_the_nearest_end_and_no_pages_is_two_disabled_arrows() {
        let current = |page, pages| {
            let frame = tree_of(pagination(page, pages)).layout(window());
            frame.access.iter().find(|node| node.selected == Some(true)).map(|node| node.name.clone())
        };
        assert_eq!(current(0, 12).as_deref(), Some("Page 1"));
        assert_eq!(current(99, 12).as_deref(), Some("Page 12"));
        assert_eq!(current(1, 1).as_deref(), Some("Page 1"));

        for page in [0, 1, 7] {
            let mut tree = tree_of(pagination(page, 0));
            let frame = tree.layout(window());
            assert!(buttons(&frame).is_empty());
            assert!(texts(&frame).is_empty());
            let said: Vec<_> = frame.access.iter().map(|node| (node.id.as_str(), node.disabled)).collect();
            assert_eq!(said, [("results:previous", true), ("results:next", true)]);
            key(&mut tree, Key::Tab);
            assert_eq!(tree.focused_id(), None);
        }
        // One page: its number between two arrows that lead nowhere.
        let frame = tree_of(pagination(1, 1)).layout(window());
        assert_eq!(buttons(&frame).iter().map(|button| button.0).collect::<Vec<_>>(), ["results:1"]);
    }

    #[test]
    fn a_disabled_row_has_no_region_takes_no_focus_reports_nothing_and_keeps_its_place() {
        let mut tree = tree_of(pagination(5, 12).disabled());
        let frame = tree.layout(window());
        assert!(buttons(&frame).is_empty());
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), None);
        let enabled = tree_of(pagination(5, 12).enabled(true)).layout(window());
        assert_eq!(buttons(&enabled).len(), 7);
        for (id, rect) in buttons(&enabled) {
            assert!(!tree.focus(id));
            assert!(click(&mut tree, rect).is_empty());
            let node = frame.access.iter().find(|node| node.id == id).unwrap();
            assert_eq!((node.rect, node.disabled, node.actionable), (rect, true, false));
        }
        let places = |frame: &Frame| texts(frame).into_iter().map(|text| (text.0, text.1)).collect::<Vec<_>>();
        assert_eq!(places(&frame), places(&enabled));
        // Muted, with the current page still marked by a fill.
        assert!(texts(&frame).iter().all(|text| text.2 == palette().muted));
        let current = button(&enabled, "results:5").unwrap();
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. } if *rect == current && *color == palette().raised)));
    }

    #[test]
    fn each_number_is_a_button_named_page_n_and_the_current_one_is_the_selected() {
        for enabled in [true, false] {
            let frame = tree_of(pagination(5, 12).enabled(enabled)).layout(window());
            let said: Vec<_> = frame.access.iter().map(|node| (node.id.as_str(), node.name.as_str(), node.selected)).collect();
            assert_eq!(
                said,
                [
                    ("results:previous", "Previous page", None),
                    ("results:1", "Page 1", Some(false)),
                    ("results:4", "Page 4", Some(false)),
                    ("results:5", "Page 5", Some(true)),
                    ("results:6", "Page 6", Some(false)),
                    ("results:12", "Page 12", Some(false)),
                    ("results:next", "Next page", None),
                ]
            );
            assert!(frame.access.iter().all(|node| node.role == Role::Button));
            assert!(frame.access.iter().all(|node| (node.disabled, node.actionable) == (!enabled, enabled)));
        }
    }

    #[test]
    fn the_host_gives_the_words_and_a_named_row_is_a_group_around_its_buttons() {
        let row = pagination(2, 3).labels("Página anterior", "Página siguiente", "Página").accessibility_label("Resultados");
        let frame = tree_of(row).layout(window());
        let said: Vec<_> = frame.access.iter().map(|node| (node.id.as_str(), node.role, node.name.as_str())).collect();
        assert_eq!(
            said,
            [
                ("results:previous", Role::Button, "Página anterior"),
                ("results:1", Role::Button, "Página 1"),
                ("results:2", Role::Button, "Página 2"),
                ("results:3", Role::Button, "Página 3"),
                ("results:next", Role::Button, "Página siguiente"),
                ("results", Role::Group, "Resultados"),
            ]
        );
    }

    #[test]
    fn compact_is_the_two_arrows_around_the_position_and_does_not_move_with_the_page() {
        let mut tree = tree_of(pagination(3, 12).compact());
        let frame = tree.layout(window());
        assert_eq!(texts(&frame).iter().map(|text| text.0.as_str()).collect::<Vec<_>>(), ["3 / 12"]);
        assert_eq!(buttons(&frame).iter().map(|button| button.0).collect::<Vec<_>>(), ["results:previous", "results:next"]);
        let (previous, next) = (button(&frame, "results:previous").unwrap(), button(&frame, "results:next").unwrap());
        assert_eq!(click(&mut tree, previous), clicked("results=2"));
        assert_eq!(click(&mut tree, next), clicked("results=4"));
        // The label is centered between the arrows.
        let (_, position, color) = texts(&frame).remove(0);
        assert_eq!(color, palette().muted);
        assert_eq!(position.x - (previous.x + previous.width), next.x - (position.x + 6.0 * 6.0));
        for page in [1, 9, 11] {
            let frame = tree_of(pagination(page, 12).compact()).layout(window());
            assert_eq!(button(&frame, "results:next").unwrap_or(next), next);
        }
        let last = tree_of(pagination(12, 12).compact()).layout(window());
        assert!(button(&last, "results:next").is_none());
        let none = tree_of(pagination(1, 0).compact()).layout(window());
        assert!(buttons(&none).is_empty());
        assert_eq!(texts(&none)[0].0, "0 / 0");
    }

    #[test]
    fn a_size_sets_the_height_of_every_cell_and_a_long_last_page_widens_the_numbers() {
        for (size, height) in [
            (ControlSize::Dense, 24.0),
            (ControlSize::Compact, 28.0),
            (ControlSize::Regular, 32.0),
            (ControlSize::Spacious, 36.0),
        ] {
            let frame = tree_of(pagination(5, 12).size(size)).layout(window());
            assert_eq!(buttons(&frame).len(), 7);
            assert!(buttons(&frame).iter().all(|(_, rect)| (rect.width, rect.height) == (height, height)));
        }
        let frame = tree_of(pagination(5000, 20000)).layout(window());
        let (arrow, number) = (button(&frame, "results:next").unwrap(), button(&frame, "results:5000").unwrap());
        assert_eq!((arrow.width, number.width, number.height), (32.0, 53.0, 32.0));
        assert_eq!(number.width, button(&frame, "results:1").unwrap().width);
    }
}
