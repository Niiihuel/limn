//! The shell of a page: a centered scrolling column, its heading, the
//! headings of its sections and a grid of cards.

use crate::{
    Chip, Component, ComponentPalette, Element, FontFamily, Icon, ScrollArea, ScrollbarAppearance,
};

// A profile is plain data from the host; a bad number stops here.
pub(crate) fn px(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn sans(value: String, size: f32, truncate: bool) -> Element {
    if truncate {
        Element::truncated_text(value)
    } else {
        Element::text(value)
    }
    .font_family(FontFamily::Sans)
    .font_size(px(size))
}

/// Density of a [`PageHeader`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageHeaderSizing {
    /// Font size of the title.
    pub title_size: f32,
    /// Font size of the description.
    pub description_size: f32,
    /// Space between the title and the description.
    pub copy_gap: f32,
    /// Space between the leading element, the copy and the trailing element.
    pub row_gap: f32,
    /// Space between the copy and a trailing element placed under it.
    pub stacked_gap: f32,
    /// Fixed height of the header while it is a row; zero follows the copy.
    pub height: f32,
}

impl PageHeaderSizing {
    /// The heading of a settings page.
    pub const DEFAULT: Self = Self {
        title_size: 18.0,
        description_size: 11.0,
        copy_gap: 4.0,
        row_gap: 12.0,
        stacked_gap: 8.0,
        height: 0.0,
    };
}

/// The heading of a page: a title, an optional description, and elements
/// the host owns before and after them. It is passive.
pub struct PageHeader {
    title: String,
    description: Option<String>,
    leading: Option<Element>,
    trailing: Option<Element>,
    stacked: bool,
    trailing_top: bool,
    truncate: bool,
    sizing: PageHeaderSizing,
    palette: ComponentPalette,
}

impl PageHeader {
    /// A heading with a title in the palette's text color.
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            description: None,
            leading: None,
            trailing: None,
            stacked: false,
            trailing_top: false,
            truncate: false,
            sizing: PageHeaderSizing::DEFAULT,
            palette,
        }
    }

    /// Supporting copy under the title, in the muted color.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// An element before the copy, centered on the header: a back button.
    pub fn leading(mut self, element: Element) -> Self {
        self.leading = Some(element);
        self
    }

    /// An element after the copy, centered on the header at its own width.
    pub fn trailing(mut self, element: Element) -> Self {
        self.trailing = Some(element);
        self
    }

    /// Place the trailing element under the copy, for a narrow page.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Keep the trailing element on the title's line instead of centering
    /// it on the header.
    pub fn trailing_top(mut self, top: bool) -> Self {
        self.trailing_top = top;
        self
    }

    /// Cut the title and the description to one line instead of wrapping.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// The header's density. [`PageHeaderSizing::DEFAULT`] unless set.
    pub fn sizing(mut self, sizing: PageHeaderSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for PageHeader {
    fn render(self) -> Element {
        let sizing = self.sizing;
        let mut copy = vec![
            sans(self.title, sizing.title_size, self.truncate).foreground(self.palette.text)
        ];
        if let Some(description) = self.description {
            copy.push(
                sans(description, sizing.description_size, self.truncate)
                    .foreground(self.palette.muted),
            );
        }
        let copy = Element::column(copy).gap(px(sizing.copy_gap));
        let mut trailing = self.trailing;
        let below = trailing.take_if(|_| self.stacked);
        let header = if self.leading.is_none() && trailing.is_none() {
            copy
        } else {
            let mut row = Vec::new();
            if let Some(leading) = self.leading {
                row.push(leading.centered_y());
            }
            row.push(copy.flex(1.0));
            if let Some(trailing) = trailing {
                row.push(if self.trailing_top {
                    trailing
                } else {
                    trailing.centered_y()
                });
            }
            let row = Element::row(row).gap(px(sizing.row_gap));
            match px(sizing.height) {
                height if height > 0.0 => row.size(None, Some(height)),
                _ => row,
            }
        };
        match below {
            Some(below) => Element::column(vec![header, below]).gap(px(sizing.stacked_gap)),
            None => header,
        }
    }
}

/// A scrolling page whose content is a centered column of bounded width.
/// The host builds the content at [`PageColumn::content_width`], then hands
/// it over with [`PageColumn::children`]. The page fills its parent.
pub struct PageColumn {
    scroll_id: String,
    available: f32,
    children: Vec<Element>,
    max_width: f32,
    min_width: f32,
    gutter: (f32, f32, f32),
    gap: f32,
    reserve_rail: bool,
    tint_scrollbar: bool,
    palette: ComponentPalette,
}

impl PageColumn {
    /// A page `available` pixels wide whose scroll position is kept under
    /// `scroll_id`.
    pub fn new(scroll_id: impl Into<String>, available: f32, palette: ComponentPalette) -> Self {
        Self {
            scroll_id: scroll_id.into(),
            available: px(available),
            children: Vec::new(),
            max_width: 960.0,
            min_width: 0.0,
            gutter: (14.0, 28.0, 640.0),
            gap: 10.0,
            reserve_rail: false,
            tint_scrollbar: true,
            palette,
        }
    }

    /// The sections of the page, top to bottom.
    pub fn children(mut self, children: Vec<Element>) -> Self {
        self.children = children;
        self
    }

    /// The widest the column grows (960 by default).
    pub fn max_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.max_width = width.max(0.0);
        }
        self
    }

    /// The narrowest the column shrinks, even when the page is narrower
    /// (0 by default).
    pub fn min_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.min_width = width.max(0.0);
        }
        self
    }

    /// The padding around the column: `narrow` on a page under `breakpoint`
    /// wide, `wide` from there on (14 and 28 at 640 by default).
    pub fn gutter(mut self, narrow: f32, wide: f32, breakpoint: f32) -> Self {
        if narrow.is_finite() && wide.is_finite() && breakpoint.is_finite() {
            self.gutter = (narrow.max(0.0), wide.max(0.0), breakpoint);
        }
        self
    }

    /// The space between sections (10 by default).
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
        self
    }

    /// Keep a scrollbar rail's width out of the column even before the page
    /// overflows, so the content keeps its width when the scrollbar arrives.
    pub fn reserve_rail(mut self, reserve: bool) -> Self {
        self.reserve_rail = reserve;
        self
    }

    /// Paint the scrollbar in the palette's muted color (the default) or
    /// leave it the scroll area's own.
    pub fn tint_scrollbar(mut self, tint: bool) -> Self {
        self.tint_scrollbar = tint;
        self
    }

    /// The padding in force at this page's width.
    pub fn gutter_width(&self) -> f32 {
        let (narrow, wide, breakpoint) = self.gutter;
        if self.available < breakpoint {
            narrow
        } else {
            wide
        }
    }

    /// The width of the column, for content the host sizes itself.
    pub fn content_width(&self) -> f32 {
        let rail = if self.reserve_rail {
            ScrollbarAppearance::default().rail_width
        } else {
            0.0
        };
        (self.available - rail - 2.0 * self.gutter_width())
            .min(self.max_width)
            .max(self.min_width)
    }
}

impl Component for PageColumn {
    fn render(self) -> Element {
        let (width, gutter) = (self.content_width(), self.gutter_width());
        let column = Element::column(self.children).gap(self.gap).size(Some(width), None);
        let area = ScrollArea::new(self.scroll_id, column.centered_x().padding(gutter));
        if self.tint_scrollbar {
            area.tint(self.palette.muted)
        } else {
            area
        }
        .render()
        .flex(1.0)
    }
}

const CARD_GAP: f32 = 8.0;

/// Cards of equal width in rows, or in a single column on a narrow page.
/// The host builds each card at [`CardGrid::item_width`], then hands them
/// over with [`CardGrid::items`].
pub struct CardGrid {
    width: f32,
    breakpoint: f32,
    columns: usize,
    items: Vec<Element>,
}

impl CardGrid {
    /// A grid `width` pixels wide with two columns from `breakpoint` on and
    /// one below it.
    pub fn new(width: f32, breakpoint: f32) -> Self {
        Self {
            width: px(width),
            breakpoint: px(breakpoint),
            columns: 2,
            items: Vec::new(),
        }
    }

    /// The number of columns on a wide page (2 by default, at least 1).
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// The cards, in reading order.
    pub fn items(mut self, items: Vec<Element>) -> Self {
        self.items = items;
        self
    }

    fn columns_in_force(&self) -> usize {
        if self.width < self.breakpoint {
            1
        } else {
            self.columns
        }
    }

    /// The width of one card at this grid's width.
    pub fn item_width(&self) -> f32 {
        let columns = self.columns_in_force() as f32;
        ((self.width - CARD_GAP * (columns - 1.0)) / columns).max(0.0)
    }
}

impl Component for CardGrid {
    fn render(self) -> Element {
        let columns = self.columns_in_force();
        if columns == 1 {
            return Element::column(self.items).gap(CARD_GAP);
        }
        let item_width = self.item_width();
        let mut rows = Vec::new();
        let mut items = self.items.into_iter().peekable();
        while items.peek().is_some() {
            let mut row: Vec<_> = items.by_ref().take(columns).collect();
            // A short last row keeps its cards in their columns.
            row.resize_with(columns, || Element::spacer().size(Some(item_width), None));
            rows.push(Element::row(row).gap(CARD_GAP));
        }
        Element::column(rows).gap(CARD_GAP)
    }
}

/// Density of a [`SectionHeader`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionSizing {
    /// Height of the row, padding included.
    pub height: f32,
    /// Inset on every side.
    pub padding: f32,
    /// Space between the row's parts.
    pub gap: f32,
    /// Font size of the title.
    pub title_size: f32,
    /// Paint the title in the muted color, for a label rather than a heading.
    pub title_muted: bool,
    /// Font size of the detail.
    pub detail_size: f32,
    /// Start the title and the detail at the top inset instead of centering
    /// them on the row.
    pub align_top: bool,
}

impl SectionSizing {
    /// A section of a page: a heading, usually with a count.
    pub const TITLE: Self = Self {
        height: 28.0,
        padding: 0.0,
        gap: 8.0,
        title_size: 15.0,
        title_muted: false,
        detail_size: 11.0,
        align_top: false,
    };

    /// A quiet label over a group in a side panel.
    pub const LABEL: Self = Self {
        height: 25.0,
        padding: 4.0,
        gap: 6.0,
        title_size: 11.0,
        title_muted: true,
        detail_size: 10.5,
        align_top: true,
    };

    /// The heading of a list, with its counter as a detail.
    pub const LIST: Self = Self {
        title_size: 14.0,
        ..Self::TITLE
    };
}

/// The heading of a section: a title with an optional disclosure chevron
/// before it, a count right after it, and a detail and a trailing element at
/// the far end. With an action the whole row is a button.
pub struct SectionHeader {
    title: String,
    count: Option<usize>,
    detail: Option<String>,
    trailing: Option<Element>,
    disclosure: Option<bool>,
    action: Option<String>,
    width: Option<f32>,
    truncate: bool,
    sizing: SectionSizing,
    palette: ComponentPalette,
}

impl SectionHeader {
    /// A heading in the [`SectionSizing::TITLE`] profile.
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            count: None,
            detail: None,
            trailing: None,
            disclosure: None,
            action: None,
            width: None,
            truncate: false,
            sizing: SectionSizing::TITLE,
            palette,
        }
    }

    /// How many items the section holds, in a chip right after the title.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Muted text at the far end of the row.
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        self.detail = Some(text.into());
        self
    }

    /// An element at the far end of the row, after the detail.
    pub fn trailing(mut self, element: Element) -> Self {
        self.trailing = Some(element);
        self
    }

    /// A chevron before the title: down when `Some(true)`, right when
    /// `Some(false)`, none for a section that does not fold.
    pub fn disclosure(mut self, open: Option<bool>) -> Self {
        self.disclosure = open;
        self
    }

    /// Make the whole row a button reporting `id`.
    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }

    /// A fixed width, for a row that must not follow its parent's.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Cut the title to one line instead of wrapping it, for a heading in
    /// a list of rows of one height.
    pub fn truncate(mut self, truncate: bool) -> Self {
        self.truncate = truncate;
        self
    }

    /// The row's density. [`SectionSizing::TITLE`] unless set.
    pub fn sizing(mut self, sizing: SectionSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for SectionHeader {
    fn render(self) -> Element {
        let sizing = self.sizing;
        let mut row = Vec::new();
        if let Some(open) = self.disclosure {
            row.push(
                Element::icon(if open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                })
                .foreground(self.palette.muted)
                .centered_y(),
            );
        }
        let place = |text: Element| {
            if sizing.align_top {
                text
            } else {
                text.centered_y()
            }
        };
        let title = sans(self.title, sizing.title_size, self.truncate).foreground(if sizing.title_muted {
            self.palette.muted
        } else {
            self.palette.text
        });
        if let Some(count) = self.count {
            // The count belongs to the title, so the title keeps its own
            // width and the free space comes after the chip.
            row.push(place(title));
            row.push(Chip::new(count.to_string(), self.palette).render().centered_y());
            row.push(Element::spacer().flex(1.0));
        } else {
            row.push(place(title.flex(1.0)));
        }
        if let Some(detail) = self.detail {
            row.push(place(
                sans(detail, sizing.detail_size, false).foreground(self.palette.muted),
            ));
        }
        if let Some(trailing) = self.trailing {
            row.push(trailing.centered_y());
        }
        let row = Element::row(row).gap(px(sizing.gap)).padding(px(sizing.padding));
        let row = match self.action {
            Some(id) => self.palette.interactive_surface(Element::clickable(id, row)),
            None => row,
        };
        row.size(self.width, Some(px(sizing.height)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn frame_of(element: Element, width: f32) -> Frame {
        let mut tree = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(width, 600.0))
    }

    fn text_rect(frame: &Frame, label: &str) -> Rect {
        frame
            .text_layouts()
            .iter()
            .find(|block| block.source == label)
            .unwrap_or_else(|| panic!("no text {label:?}"))
            .runs[0]
            .rect
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap().rect
    }

    fn button(id: &str) -> Element {
        crate::Button::new(id, "Nuevo", palette()).render()
    }

    #[test]
    fn page_header_centers_its_trailing_element_and_stacks_it_when_asked() {
        let header = |stacked| {
            PageHeader::new("Proveedores", palette())
                .description("Cuentas conectadas")
                .trailing(button("new"))
                .stacked(stacked)
                .render()
        };
        let frame = frame_of(header(false), 400.0);
        let title = text_rect(&frame, "Proveedores");
        let description = text_rect(&frame, "Cuentas conectadas");
        let action = region(&frame, "new");
        assert_eq!(description.y - (title.y + title.height), 4.0);
        assert!((action.x + action.width - 400.0).abs() < 0.01);
        let copy_center = (title.y + description.y + description.height) / 2.0;
        assert!((action.y + action.height / 2.0 - copy_center).abs() < 0.51);

        let frame = frame_of(header(true), 400.0);
        let description = text_rect(&frame, "Cuentas conectadas");
        assert_eq!(region(&frame, "new").y, description.y + description.height + 8.0);
    }

    #[test]
    fn page_header_takes_its_sizes_from_the_profile_and_survives_bad_numbers() {
        let sizing = PageHeaderSizing {
            title_size: 20.0,
            copy_gap: 2.0,
            row_gap: 6.0,
            height: 40.0,
            ..PageHeaderSizing::DEFAULT
        };
        let frame = frame_of(
            PageHeader::new("Marketplace", palette())
                .description("Skills")
                .leading(button("back"))
                .trailing(button("new"))
                .truncate(true)
                .sizing(sizing)
                .render(),
            480.0,
        );
        let back = region(&frame, "back");
        assert_eq!(back.x, 0.0);
        assert!((back.y + back.height / 2.0 - 20.0).abs() < 0.01);
        // A cut title is still painted as one run of text.
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, position, .. }
                if text == "Marketplace" && *font_size == 20.0 && position.x == back.width + 6.0)));

        let bad = PageHeaderSizing {
            title_size: f32::NAN,
            description_size: f32::INFINITY,
            copy_gap: -4.0,
            row_gap: f32::NAN,
            stacked_gap: f32::NEG_INFINITY,
            height: f32::NAN,
        };
        for width in [120.0, 400.0] {
            let frame = frame_of(
                PageHeader::new("Título", palette())
                    .description("Detalle")
                    .trailing(button("new"))
                    .sizing(bad)
                    .render(),
                width,
            );
            assert!(frame.regions.iter().all(|region| region.rect.x.is_finite()
                && region.rect.y.is_finite()
                && region.rect.x + region.rect.width <= width + 0.01));
        }
    }

    #[test]
    fn page_column_width_follows_gutter_rail_and_bounds() {
        let page = |available| PageColumn::new("page", available, palette());
        // Defaults: 14 under 640, 28 from there, at most 960.
        assert_eq!(page(639.0).content_width(), 639.0 - 28.0);
        assert_eq!(page(640.0).content_width(), 640.0 - 56.0);
        assert_eq!(page(2000.0).content_width(), 960.0);
        assert_eq!(page(640.0).reserve_rail(true).content_width(), 640.0 - 56.0 - 14.0);
        let settings = |available| page(available).gutter(12.0, 20.0, 600.0).reserve_rail(true);
        assert_eq!(settings(599.0).content_width(), 599.0 - 14.0 - 24.0);
        assert_eq!(settings(600.0).content_width(), 600.0 - 14.0 - 40.0);
        assert_eq!(page(10.0).content_width(), 0.0);
        assert_eq!(page(10.0).min_width(240.0).content_width(), 240.0);
        assert_eq!(page(2000.0).max_width(780.0).content_width(), 780.0);
        // Bad numbers leave the defaults in force.
        let bad = page(f32::NAN)
            .max_width(f32::NAN)
            .min_width(f32::INFINITY)
            .gutter(f32::NAN, 1.0, 2.0)
            .gap(f32::NAN);
        assert_eq!(bad.content_width(), 0.0);
        assert_eq!(page(2000.0).max_width(f32::NAN).content_width(), 960.0);
    }

    #[test]
    fn page_header_trailing_top_keeps_the_action_on_the_title_line() {
        let header = PageHeader::new("Proveedores", palette())
            .description("Cuentas conectadas")
            .trailing(button("new").size(None, Some(16.0)))
            .trailing_top(true)
            .render();
        let frame = frame_of(header, 400.0);
        assert_eq!(region(&frame, "new").y, 0.0);
        assert!((region(&frame, "new").x + region(&frame, "new").width - 400.0).abs() < 0.01);
    }

    #[test]
    fn page_column_scrollbar_is_tinted_unless_told_otherwise() {
        let tint = |page: PageColumn| match page.render().kind {
            crate::ElementKind::Scroll { appearance, .. } => appearance.tint,
            _ => panic!("a page column is a scroll area"),
        };
        assert_eq!(tint(PageColumn::new("page", 400.0, palette())), Some(palette().muted));
        assert_eq!(tint(PageColumn::new("page", 400.0, palette()).tint_scrollbar(false)), None);
    }

    #[test]
    fn section_header_truncate_keeps_a_long_title_on_one_line() {
        let title = "Un título de sección bastante más largo que su fila";
        let header = |truncate| {
            frame_of(
                SectionHeader::new(title, palette())
                    .detail("12")
                    .truncate(truncate)
                    .sizing(SectionSizing::LABEL)
                    .render(),
                120.0,
            )
        };
        let cut = |frame: &Frame| {
            frame.commands.iter().any(|command| {
                matches!(command, DrawCommand::Text { text, .. } if text.ends_with('…'))
            })
        };
        assert!(!cut(&header(false)));
        assert!(cut(&header(true)));
    }

    #[test]
    fn page_column_centers_its_content_inside_the_gutter() {
        for (width, gutter) in [(360.0, 14.0), (1200.0, 28.0)] {
            let page = PageColumn::new("page", width, palette()).reserve_rail(true);
            let content_width = page.content_width();
            let mut tree = UiTree::new(Element::column(vec![page
                .children(vec![button("first"), button("second").size(Some(content_width), None)])
                .render()]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(width, 600.0));
            let second = region(&frame, "second");
            assert!((second.x - (width - content_width) / 2.0).abs() < 0.01);
            assert!(second.x >= gutter);
            assert_eq!(region(&frame, "first").y, gutter);
            assert!(second.y > region(&frame, "first").y + 10.0 - 0.01);
            assert_eq!(region(&frame, "page").width, width);
        }
    }

    #[test]
    fn card_grid_has_one_column_below_its_breakpoint_and_pads_a_short_row() {
        let cards = |grid: &CardGrid, count: usize| {
            (0..count)
                .map(|index| {
                    Element::clickable(format!("card:{index}"), Element::spacer())
                        .size(Some(grid.item_width()), Some(40.0))
                })
                .collect::<Vec<_>>()
        };
        let narrow = CardGrid::new(739.0, 740.0);
        assert_eq!(narrow.item_width(), 739.0);
        let frame = frame_of(narrow.items(cards(&CardGrid::new(739.0, 740.0), 3)).render(), 739.0);
        assert_eq!(region(&frame, "card:1").y, 48.0);
        assert_eq!(region(&frame, "card:1").x, 0.0);

        let wide = CardGrid::new(740.0, 740.0);
        assert_eq!(wide.item_width(), 366.0);
        let frame = frame_of(wide.items(cards(&CardGrid::new(740.0, 740.0), 3)).render(), 740.0);
        assert_eq!(region(&frame, "card:1"), Rect::new(374.0, 0.0, 366.0, 40.0));
        // The odd card keeps its column instead of stretching.
        assert_eq!(region(&frame, "card:2"), Rect::new(0.0, 48.0, 366.0, 40.0));

        assert_eq!(CardGrid::new(740.0, 740.0).columns(1).item_width(), 740.0);
        assert_eq!(CardGrid::new(740.0, 740.0).columns(0).item_width(), 740.0);
        assert_eq!(CardGrid::new(916.0, 740.0).columns(3).item_width(), 300.0);
        assert_eq!(CardGrid::new(f32::NAN, f32::NAN).item_width(), 0.0);
    }

    #[test]
    fn section_header_count_is_a_passive_circle_after_the_title() {
        let frame = frame_of(SectionHeader::new("Skills", palette()).count(1).render(), 640.0);
        let title = text_rect(&frame, "Skills");
        let circle = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. } if *radius == 11.0 => Some(*rect),
                _ => None,
            })
            .expect("count chip");
        assert_eq!((circle.width, circle.height, circle.y), (22.0, 22.0, 3.0));
        assert!((circle.x - (title.x + title.width + 8.0)).abs() < 0.01);
        assert!(frame.regions.is_empty());
    }

    #[test]
    fn section_header_puts_detail_and_trailing_at_the_far_end() {
        for width in [180.0, 360.0] {
            let frame = frame_of(
                SectionHeader::new("Fuentes", palette())
                    .detail("12")
                    .trailing(button("more"))
                    .sizing(SectionSizing::LABEL)
                    .render(),
                width,
            );
            let more = region(&frame, "more");
            let detail = text_rect(&frame, "12");
            assert!((more.x + more.width - (width - 4.0)).abs() < 0.01);
            assert!((detail.x + detail.width - (more.x - 6.0)).abs() < 0.01);
            // The label profile hangs its text from the top inset.
            assert_eq!(text_rect(&frame, "Fuentes").y, 4.0);
            assert_eq!(detail.y, 4.0);
            assert_eq!(text_rect(&frame, "Fuentes").x, 4.0);
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, font_size, .. }
                    if text == "Fuentes" && *color == palette().muted && *font_size == 11.0)));
        }
    }

    #[test]
    fn section_header_disclosure_shows_its_state_and_the_row_reports_its_action() {
        let header = |open| {
            SectionHeader::new("Cambios", palette())
                .disclosure(Some(open))
                .action("fold")
                .sizing(SectionSizing::LIST)
                .render()
        };
        for (open, icon) in [(true, Icon::ChevronDown), (false, Icon::ChevronRight)] {
            let frame = frame_of(header(open), 300.0);
            let chevron = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, icon: drawn, .. } if *drawn == icon => Some(*rect),
                    _ => None,
                })
                .expect("chevron");
            assert!(chevron.x < text_rect(&frame, "Cambios").x);
        }
        let mut tree = UiTree::new(Element::column(vec![header(true), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(300.0, 600.0));
        assert_eq!(region(&frame, "fold"), Rect::new(0.0, 0.0, 300.0, 28.0));
        let point = Point::new(150.0, 14.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "fold".into() }]
        );
        let narrow = SectionHeader::new("Cambios", palette()).action("fold").width(120.0);
        assert_eq!(region(&frame_of(narrow.render(), 300.0), "fold").width, 120.0);
        let bad = SectionHeader::new("Cambios", palette()).action("fold").width(f32::NAN);
        assert_eq!(region(&frame_of(bad.render(), 300.0), "fold").width, 300.0);
        // Without an action or a chevron it is passive.
        let frame = frame_of(SectionHeader::new("Cambios", palette()).render(), 300.0);
        assert!(frame.regions.is_empty());
    }

    #[test]
    fn section_header_survives_a_bad_profile() {
        let bad = SectionSizing {
            height: f32::NAN,
            padding: -3.0,
            gap: f32::INFINITY,
            title_size: f32::NAN,
            title_muted: false,
            detail_size: -1.0,
            align_top: false,
        };
        let frame = frame_of(
            SectionHeader::new("Cambios", palette()).count(12).detail("x").sizing(bad).render(),
            200.0,
        );
        for label in ["Cambios", "12", "x"] {
            let rect = text_rect(&frame, label);
            assert!(rect.x.is_finite() && rect.y.is_finite());
        }
    }
}
