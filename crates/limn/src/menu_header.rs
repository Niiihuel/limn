//! The fixed head of a menu: what it lists, a way back, and a search.

use crate::{
    ButtonVariant, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon,
    IconButton, SearchField, SelectorSizing,
};

/// Pass the rendered header to `DropdownMenu::header`, which keeps it above
/// the options while they scroll. The host owns the query and handles the
/// back action; the search field paints the palette's `raised`.
pub struct MenuHeader {
    palette: ComponentPalette,
    title: Option<String>,
    back: Option<(String, String)>,
    trailing: Option<Element>,
    search: Option<(String, String, String)>,
    width: Option<f32>,
}

impl MenuHeader {
    /// An empty header; give it a title or a back action.
    pub fn new(palette: ComponentPalette) -> Self {
        Self {
            palette,
            title: None,
            back: None,
            trailing: None,
            search: None,
            width: None,
        }
    }

    /// Name what the menu lists.
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// Lead with a back button reporting `id` in place of the title, for a
    /// view the user entered from another one. `label` names this view.
    pub fn back(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.back = Some((id.into(), label.into()));
        self
    }

    /// One square tool at the far end, the height of the header: refresh.
    pub fn trailing(mut self, element: Element) -> Self {
        self.trailing = Some(element);
        self
    }

    /// A compact search field under the title row, as wide as the header.
    pub fn search(
        mut self,
        id: impl Into<String>,
        query: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        self.search = Some((id.into(), query.into(), placeholder.into()));
        self
    }

    /// A fixed width, for a menu whose inner width the host computes. Values
    /// that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }
}

impl Component for MenuHeader {
    fn render(self) -> Element {
        let sizing = SelectorSizing::COMPACT;
        let palette = self.palette;
        let tool = ControlSize::Compact.height();
        let lead = match (self.back, self.title) {
            (Some((id, label)), _) => {
                let back = IconButton::new(id, Icon::ArrowLeft, palette)
                    .label(label)
                    .variant(ButtonVariant::Subtle)
                    .size(ControlSize::Compact);
                // The button fills what the trailing tool leaves, so the
                // whole strip is the way back.
                match self.width {
                    Some(width) => {
                        let taken = if self.trailing.is_some() {
                            tool + sizing.gap
                        } else {
                            0.0
                        };
                        back.width((width - taken).max(0.0)).render()
                    }
                    None => back.render().flex(1.0),
                }
            }
            (None, title) => {
                let title = Element::text(title.unwrap_or_default())
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.label_size)
                    .foreground(palette.text)
                    .padding(sizing.padding);
                if self.trailing.is_none() {
                    title.size(self.width, Some(sizing.header_height))
                } else {
                    title.flex(1.0)
                }
            }
        };
        let head = match self.trailing {
            Some(trailing) => Element::row(vec![lead, trailing.size(Some(tool), Some(tool))])
                .gap(sizing.gap)
                .size(self.width, Some(sizing.header_height)),
            None => lead.size(self.width, Some(sizing.header_height)),
        };
        let Some((id, query, placeholder)) = self.search else {
            return head;
        };
        let mut search =
            SearchField::new(id, query, placeholder, palette).size(ControlSize::Compact);
        if let Some(width) = self.width {
            search = search.width(width);
        }
        // Two pixels, the gap a menu keeps between its rows.
        Element::column(vec![head, search.render()]).gap(2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DrawCommand, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, Frame, Point, Rect, Size,
        UiAction, UiEvent, UiTree,
    };

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn rect(frame: &Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
            .rect
    }

    #[test]
    fn a_title_is_one_header_row_and_the_search_sits_under_it_at_full_width() {
        for width in [200.0, 308.0] {
            let mut tree = UiTree::new(Element::column(vec![
                MenuHeader::new(palette())
                    .title("Projects")
                    .search("find", "", "Search")
                    .width(width)
                    .render(),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(400.0, 200.0));
            let search = rect(&frame, "find");
            let sizing = SelectorSizing::COMPACT;
            assert_eq!(search.y, sizing.header_height + 2.0);
            assert_eq!(search.width, width);
            assert_eq!(search.height, ControlSize::Compact.height());
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_size, .. }
                    if text == "Projects" && *font_size == sizing.label_size)));
        }
    }

    #[test]
    fn back_fills_the_row_beside_a_square_trailing_tool_and_reports_its_id() {
        let mut tree = UiTree::new(Element::column(vec![
            MenuHeader::new(palette())
                .back("back", "Model")
                .trailing(IconButton::new("refresh", Icon::Refresh, palette()).render())
                .width(300.0)
                .render(),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 200.0));
        let (back, refresh) = (rect(&frame, "back"), rect(&frame, "refresh"));
        let sizing = SelectorSizing::COMPACT;
        assert_eq!((refresh.width, refresh.height), (28.0, 28.0));
        assert_eq!(back.width, 300.0 - 28.0 - sizing.gap);
        assert_eq!(refresh.x + refresh.width, 300.0);
        let point = Point::new(back.x + back.width - 4.0, back.y + 4.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "back".into() }]
        );
    }

    #[test]
    fn the_search_stays_put_while_the_options_scroll() {
        let menu = |offset: f32| {
            let mut tree = UiTree::new(
                DropdownMenu::new(
                    Element::button("trigger", "Open").size(Some(80.0), Some(28.0)),
                    (0..30)
                        .map(|index| {
                            DropdownMenuEntry::from(DropdownMenuItem::new(
                                format!("item:{index}"),
                                format!("Item {index}"),
                            ))
                        })
                        .collect(),
                    true,
                    "dismiss",
                    palette(),
                )
                .header(
                    MenuHeader::new(palette())
                        .title("Items")
                        .search("find", "", "Search")
                        .render(),
                )
                .render(),
            );
            tree.set_reduced_motion(true);
            let size = Size::new(400.0, 600.0);
            let frame = tree.layout(size);
            let over = rect(&frame, "item:1");
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(over.x + 20.0, over.y + 4.0),
                delta_y: offset,
            });
            let frame = tree.layout(size);
            let row = frame.regions.iter().find(|region| region.id == "item:1");
            (rect(&frame, "find"), row.map(|region| region.rect.y))
        };
        let (search, row) = menu(0.0);
        let (scrolled_search, scrolled_row) = menu(-120.0);
        assert_eq!(search, scrolled_search);
        assert_ne!(row, scrolled_row, "the options did scroll");
    }

    #[test]
    fn a_width_that_is_not_finite_is_ignored() {
        let header = MenuHeader::new(palette())
            .title("Items")
            .width(f32::NAN)
            .width(f32::INFINITY)
            .render();
        assert_eq!(header.style.width, None);
        assert_eq!(
            MenuHeader::new(palette()).title("Items").width(-4.0).render().style.width,
            Some(0.0)
        );
    }
}
