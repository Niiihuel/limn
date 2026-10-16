//! Titled items one under another, each of which opens to show its content.

use crate::{
    Access, Collapse, Component, ComponentPalette, ControlSize, CornerRadii, Element, FontFamily, Icon, Role,
    Separator,
};

/// Space between a header's edge and its content, and beside an item's content.
const INSET: f32 = 12.0;
/// Space between the parts of a header.
const GAP: f32 = 8.0;
const ICON_SIDE: f32 = 16.0;
// The outline of a bordered accordion, which its items sit inside.
const BORDER: f32 = 1.0;
/// What a header is higher than a control of its size by.
const HEADER_ROOM: f32 = 8.0;
/// Space under an open item's content, before the next hairline.
const BODY_END: f32 = 12.0;

/// One item of an [`Accordion`]: its title, what it shows while open and the
/// ID its header reports.
pub struct AccordionItem {
    id: String,
    title: String,
    content: Element,
    open: bool,
    icon: Option<Icon>,
    description: Option<String>,
    enabled: bool,
}

impl AccordionItem {
    /// A closed, enabled item. Its header reports `UiAction::Clicked` with
    /// `id`; `content` is in the window only while the item is open.
    pub fn new(id: impl Into<String>, title: impl Into<String>, content: Element) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            content,
            open: false,
            icon: None,
            description: None,
            enabled: true,
        }
    }

    /// Show the content under the header. The host sets this from its own
    /// state; the item never opens or closes itself.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Put an icon before the title, in the palette's muted color.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Muted text beside the title, on its line, which the platform is told as
    /// the header's description. It is cut with an ellipsis after the title
    /// is. An empty one is no description.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// A disabled item cannot be opened or closed: its header is passive, with
    /// a muted title and no hit region or focus. It stays as `open` says, and
    /// what is inside an open one can still be used.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    // The header and the slot its content opens in. `radii` rounds the header's
    // hover fill where it meets the corners of a bordered accordion.
    fn render(self, height: f32, font_size: f32, radii: CornerRadii, palette: ComponentPalette) -> [Element; 2] {
        let description = self.description.filter(|text| !text.is_empty());
        let mut said = Access::new(Role::Button).name(self.title.clone()).expanded(self.open);
        if let Some(description) = &description {
            said = said.description(description.clone());
        }
        let mut parts = Vec::with_capacity(5);
        if let Some(icon) = self.icon {
            let icon = Element::icon(icon).foreground(palette.muted).size(Some(ICON_SIDE), Some(ICON_SIDE));
            parts.push(icon.centered_y());
        }
        parts.push(
            Element::truncated_text(self.title)
                .font_family(FontFamily::Sans)
                .font_size(font_size)
                .foreground(if self.enabled { palette.text } else { palette.muted })
                .size(None, Some(height))
                .shrink_to_fit(),
        );
        if let Some(description) = description {
            parts.push(
                Element::truncated_text(description)
                    .font_family(FontFamily::Sans)
                    .font_size(font_size - 1.0)
                    .foreground(palette.muted)
                    .size(None, Some(height))
                    .shrink_to_fit(),
            );
        }
        parts.push(Element::spacer().flex(1.0));
        let chevron = Element::icon(if self.open { Icon::ChevronDown } else { Icon::ChevronRight })
            .foreground(palette.muted)
            .size(Some(ICON_SIDE), Some(ICON_SIDE));
        parts.push(chevron.centered_y());
        let header = Element::row(vec![
            Element::spacer().size(Some(INSET), Some(1.0)),
            Element::row(parts).gap(GAP).clip_children().flex(1.0),
            Element::spacer().size(Some(INSET), Some(1.0)),
        ]);
        let header = if self.enabled {
            palette.bar_item_surface(Element::clickable(&self.id, header).accessible(said))
        } else {
            Element::panel(header).accessible(said.id(self.id.clone()).disabled(true))
        }
        .corner_radii(radii)
        .size(None, Some(height));
        let content = self.content;
        let body = self.open.then(|| {
            Element::column(vec![
                Element::row(vec![
                    Element::spacer().size(Some(INSET), Some(1.0)),
                    Element::column(vec![content]).flex(1.0),
                    Element::spacer().size(Some(INSET), Some(1.0)),
                ]),
                Element::spacer().size(None, Some(BODY_END)),
            ])
        });
        // The library's own slot: it opens from nothing and closes in place,
        // and a closed one holds nothing to click or to focus.
        [header, Collapse::new(format!("{}:body", self.id), body).expand().render()]
    }
}

/// Several titled items, each of which opens under its header. The host owns
/// which are open: it passes `open` for each item and changes its own state
/// when a header reports `UiAction::Clicked` with the item's ID, so whether
/// one or several may be open at once is the host's rule. The accordion
/// reports nothing else and keeps nothing.
pub struct Accordion {
    id: String,
    items: Vec<AccordionItem>,
    bordered: bool,
    size: ControlSize,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl Accordion {
    /// These items in order, a hairline apart, at `ControlSize::Regular` and
    /// with no outline. `id` is what the platform knows the group by once it
    /// has an `accessibility_label`.
    pub fn new(id: impl Into<String>, items: Vec<AccordionItem>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items,
            bordered: false,
            size: ControlSize::Regular,
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// Draw the items as one surface: the palette's `surface` inside its
    /// `border`, with the corners of a card. Without it they are a plain list
    /// with only the hairlines between them.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// The height of every header, 8 pixels more than a control of this size
    /// (32, 36, 40 and 44), and its title in the text size of a button of
    /// this size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// What the accordion is called to the platform, such as "Settings".
    /// Without one the platform is told of the headers alone.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Disable every item, whatever each one says.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable the whole accordion from a state the host computes;
    /// `enabled(true)` leaves each item as it says it is.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for Accordion {
    fn render(self) -> Element {
        let palette = self.palette;
        let height = self.size.height() + HEADER_ROOM;
        let font_size = self.size.button_sizing().font_size;
        let radius = crate::tokens().radius.card;
        // The headers sit inside the outline, so their outer corners turn one pixel tighter.
        let inner = if self.bordered { (radius - BORDER).max(0.0) } else { 0.0 };
        let last = self.items.len().saturating_sub(1);
        let mut rows = Vec::with_capacity(self.items.len() * 3);
        for (index, item) in self.items.into_iter().enumerate() {
            if index > 0 {
                rows.push(Separator::horizontal(palette).render());
            }
            let top = if index == 0 { inner } else { 0.0 };
            // An open last item ends in its content, not in its header.
            let bottom = if index == last && !item.open { inner } else { 0.0 };
            let radii = CornerRadii { top_left: top, top_right: top, bottom_left: bottom, bottom_right: bottom };
            let enabled = self.enabled && item.enabled;
            rows.extend(item.enabled(enabled).render(height, font_size, radii, palette));
        }
        let empty = rows.is_empty();
        let list = Element::column(rows);
        // An outline around nothing would be a stray line.
        let group = if self.bordered && !empty {
            Element::panel(list)
                .padding(BORDER)
                .background(palette.surface)
                .border(palette.border, BORDER)
                .radius(radius)
        } else {
            list
        };
        match self.accessibility_label {
            Some(name) => group.accessible(Access::new(Role::Group).id(self.id).name(name)),
            None => group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    const WINDOW: Size = Size::new(400.0, 400.0);
    /// The height of the content of every item in these tests.
    const CONTENT: f32 = 40.0;

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    /// Three items, each holding one button named after it, open as `open` says.
    fn items(open: [bool; 3]) -> Vec<AccordionItem> {
        ["a", "b", "c"]
            .into_iter()
            .zip(open)
            .map(|(id, open)| {
                let inside = Element::button(format!("{id}:inside"), "Inside").size(None, Some(CONTENT));
                AccordionItem::new(id, format!("Item {id}"), inside).open(open)
            })
            .collect()
    }

    fn accordion(open: [bool; 3]) -> Accordion {
        Accordion::new("group", items(open), palette())
    }

    fn root(accordion: Accordion) -> Element {
        Element::column(vec![accordion.render(), Element::spacer().flex(1.0)])
    }

    fn tree_of(accordion: Accordion) -> UiTree {
        let mut tree = UiTree::new(root(accordion));
        tree.set_reduced_motion(true);
        tree
    }

    fn node<'a>(frame: &'a Frame, id: &str) -> &'a AccessNode {
        frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"))
    }

    fn headers(frame: &Frame) -> [Rect; 3] {
        ["a", "b", "c"].map(|id| node(frame, id).rect)
    }

    // The regions that take a click, in order.
    fn buttons(frame: &Frame) -> Vec<&str> {
        let clickable = frame.regions.iter().filter(|region| region.kind == HitKind::Button);
        clickable.map(|region| region.id.as_str()).collect()
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false, alt: false }
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn outlined(frame: &Frame) -> bool {
        frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { color, .. } if *color == palette().border
        ))
    }

    #[test]
    fn headers_stack_a_hairline_apart_and_an_open_item_makes_room_for_its_content() {
        let frame = tree_of(accordion([false; 3])).layout(WINDOW);
        let [a, b, c] = headers(&frame);
        assert_eq!(a, Rect::new(0.0, 0.0, 400.0, 40.0));
        assert_eq!((b.y, c.y), (41.0, 82.0));
        assert_eq!(buttons(&frame), ["a", "b", "c"]);
        assert!(!outlined(&frame), "a plain list has hairlines only");

        let frame = tree_of(accordion([true, false, true])).layout(WINDOW);
        let [a, b, c] = headers(&frame);
        assert_eq!((a.y, b.y, c.y), (0.0, 41.0 + CONTENT + BODY_END, 82.0 + CONTENT + BODY_END));
        // The content sits under its header, inset as the title is.
        let inside = frame.regions.iter().find(|region| region.id == "a:inside").unwrap().rect;
        assert_eq!(inside, Rect::new(INSET, 40.0, 400.0 - INSET * 2.0, CONTENT));
        assert_eq!(buttons(&frame), ["a", "a:inside", "b", "c", "c:inside"]);
    }

    #[test]
    fn bordered_sets_the_items_inside_one_outline_and_every_size_gives_its_header_height() {
        for size in [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious] {
            let height = size.height() + HEADER_ROOM;
            let frame = tree_of(accordion([false; 3]).size(size)).layout(WINDOW);
            assert!(headers(&frame).iter().all(|rect| rect.height == height));

            let frame = tree_of(accordion([false; 3]).size(size).bordered(true)).layout(WINDOW);
            let [a, b, c] = headers(&frame);
            assert_eq!(a, Rect::new(1.0, 1.0, 398.0, height));
            assert_eq!((b.y, c.y), (a.y + height + 1.0, a.y + (height + 1.0) * 2.0));
            let outline = frame.commands.iter().find_map(|command| match command {
                DrawCommand::StrokeRect { rect, color, radius, .. } if *color == palette().border => {
                    Some((*rect, *radius))
                }
                _ => None,
            });
            let whole = Rect::new(0.0, 0.0, 400.0, c.y + height + 1.0);
            assert_eq!(outline, Some((whole, crate::tokens().radius.card)));
        }
    }

    #[test]
    fn the_chevron_points_down_on_an_open_item_and_an_icon_stands_before_the_title() {
        let mut items = items([true, false, false]);
        items[1] = AccordionItem::new("b", "Item b", Element::spacer()).icon(Icon::Bell);
        let frame = tree_of(Accordion::new("group", items, palette())).layout(WINDOW);
        let icons: Vec<(Icon, Rect)> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { icon, rect, tint } if *tint == palette().muted => Some((*icon, *rect)),
                _ => None,
            })
            .collect();
        let kinds: Vec<Icon> = icons.iter().map(|(icon, _)| *icon).collect();
        assert_eq!(kinds, [Icon::ChevronDown, Icon::Bell, Icon::ChevronRight, Icon::ChevronRight]);
        let [a, b, _] = headers(&frame);
        // The chevron is centered in the header, its inset from the trailing edge.
        assert_eq!(icons[0].1, Rect::new(400.0 - INSET - ICON_SIDE, a.y + 12.0, ICON_SIDE, ICON_SIDE));
        assert_eq!(icons[1].1, Rect::new(INSET, b.y + 12.0, ICON_SIDE, ICON_SIDE));
        let title = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, position, .. } if text == "Item b" => Some(position.x),
            _ => None,
        });
        assert_eq!(title, Some(INSET + ICON_SIDE + GAP));
    }

    #[test]
    fn a_header_reports_its_item_by_pointer_enter_and_space_open_or_closed() {
        let mut tree = tree_of(accordion([true, false, false]));
        let frame = tree.layout(WINDOW);
        for id in ["a", "b", "c"] {
            let clicked = vec![UiAction::Clicked { id: id.into() }];
            let point = center(node(&frame, id).rect);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked);
            assert!(tree.focus(id));
            assert_eq!(tree.handle_event(key(Key::Enter)), clicked);
            assert_eq!(tree.handle_event(key(Key::Space)), clicked);
        }
    }

    #[test]
    fn tab_passes_over_the_content_of_a_closed_item_and_stops_in_an_open_one() {
        let mut tree = tree_of(accordion([false, true, false]));
        tree.layout(WINDOW);
        assert!(!tree.focus("a:inside"));
        assert!(tree.focus("a"));
        let mut stops = Vec::new();
        for _ in 0..3 {
            tree.handle_event(key(Key::Tab));
            tree.layout(WINDOW);
            stops.push(tree.focused_id().unwrap_or_default().to_owned());
        }
        assert_eq!(stops, ["b", "b:inside", "c"]);
    }

    #[test]
    fn an_item_opens_and_closes_on_the_collapse_and_closing_content_is_out_of_reach_at_once() {
        let start = Instant::now() - Duration::from_secs(60);
        let settle = crate::tree::slot_settle();
        let mut tree = UiTree::new(root(accordion([false; 3])));
        let closed = headers(&tree.layout_at(WINDOW, start))[1].y;
        let open = closed + CONTENT + BODY_END;

        tree.set_root(root(accordion([true, false, false])));
        assert_eq!(headers(&tree.layout_at(WINDOW, start))[1].y, closed);
        let half = headers(&tree.layout_at(WINDOW, start + settle / 2))[1].y;
        assert!(half > closed && half < open, "{half}");
        assert_eq!(headers(&tree.layout_at(WINDOW, start + settle * 3))[1].y, open);
        assert!(tree.focus("a:inside"));

        tree.set_root(root(accordion([false; 3])));
        let later = start + settle * 3;
        let frame = tree.layout_at(WINDOW, later);
        assert_eq!(headers(&frame)[1].y, open);
        assert!(!buttons(&frame).contains(&"a:inside"));
        assert!(!tree.focus("a:inside"));
        let half = headers(&tree.layout_at(WINDOW, later + settle / 2))[1].y;
        assert!(half > closed && half < open, "{half}");
        assert_eq!(headers(&tree.layout_at(WINDOW, later + settle * 3))[1].y, closed);
    }

    #[test]
    fn a_disabled_item_keeps_its_place_and_reports_nothing_and_its_open_content_stays_in_use() {
        let make = |enabled: bool| {
            let mut items = items([true, true, false]);
            let first = items.remove(0).icon(Icon::Bell).description("Two rules").enabled(enabled);
            items.insert(0, first);
            Accordion::new("group", items, palette())
        };
        let enabled = tree_of(make(true)).layout(WINDOW);
        let mut tree = tree_of(make(false));
        let frame = tree.layout(WINDOW);
        assert_eq!(buttons(&frame), ["a:inside", "b", "b:inside", "c"]);
        assert_eq!(headers(&frame), headers(&enabled));
        let said = node(&frame, "a");
        assert_eq!((said.role, said.disabled, said.actionable, said.expanded), (Role::Button, true, false, Some(true)));
        assert!(!tree.focus("a"));
        let point = center(said.rect);
        tree.handle_event(UiEvent::PointerMove(point));
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(tree.handle_event(key(Key::Enter)).is_empty());
        assert!(tree.handle_event(key(Key::Space)).is_empty());
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, .. } if text == "Item a" && *color == palette().muted
        )));
        assert!(tree.focus("a:inside"));
        let disabled = format!("{:?}", make(true).disabled().render());
        assert_eq!(disabled, format!("{:?}", make(true).enabled(false).render()));
    }

    #[test]
    fn a_disabled_accordion_disables_every_item_and_none_of_them_moves() {
        for bordered in [false, true] {
            let enabled = tree_of(accordion([false, true, false]).bordered(bordered)).layout(WINDOW);
            let mut tree = tree_of(accordion([false, true, false]).bordered(bordered).disabled());
            let frame = tree.layout(WINDOW);
            assert_eq!(buttons(&frame), ["b:inside"]);
            assert_eq!(headers(&frame), headers(&enabled));
            for id in ["a", "b", "c"] {
                let said = node(&frame, id);
                assert_eq!((said.disabled, said.actionable), (true, false));
                assert!(!tree.focus(id));
                let point = center(said.rect);
                tree.handle_event(UiEvent::PointerDown(point));
                assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            }
        }
    }

    #[test]
    fn the_platform_is_told_a_button_its_title_its_description_and_whether_it_is_open() {
        let mut items = items([true, false, false]);
        items[1] = AccordionItem::new("b", "Item b", Element::spacer()).description("Two rules");
        items[2] = AccordionItem::new("c", "Item c", Element::spacer()).description("");
        let frame = tree_of(Accordion::new("group", items, palette()).accessibility_label("Settings")).layout(WINDOW);
        for (id, name, description, open) in
            [("a", "Item a", None, true), ("b", "Item b", Some("Two rules"), false), ("c", "Item c", None, false)]
        {
            let said = node(&frame, id);
            assert_eq!((said.role, said.name.as_str(), said.expanded), (Role::Button, name, Some(open)));
            assert_eq!(said.description.as_deref(), description, "{id}");
            assert_eq!((said.disabled, said.actionable), (false, true));
        }
        let group = node(&frame, "group");
        assert_eq!((group.role, group.name.as_str(), group.actionable), (Role::Group, "Settings", false));
        // Unnamed, an accordion is its headers alone.
        let frame = tree_of(accordion([false; 3])).layout(WINDOW);
        assert!(frame.access.iter().all(|node| node.id != "group"));
    }

    #[test]
    fn a_long_title_is_cut_before_the_chevron_and_an_empty_accordion_draws_nothing() {
        let title = "A title that is much too long for the width this accordion has been given";
        let item = AccordionItem::new("a", title, Element::spacer()).description("And a description");
        let mut tree = tree_of(Accordion::new("group", vec![item], palette()).bordered(true));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
        let frame = tree.layout(Size::new(200.0, 200.0));
        let chevron = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { icon: Icon::ChevronRight, rect, .. } => Some(*rect),
            _ => None,
        });
        assert_eq!(chevron.map(|rect| rect.x), Some(200.0 - BORDER - INSET - ICON_SIDE));
        let texts: Vec<(&str, f32)> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, .. } => Some((text.as_str(), position.x)),
                _ => None,
            })
            .collect();
        assert!(texts[0].0.ends_with('…'), "{texts:?}");
        for (text, x) in &texts {
            assert!(x + text.chars().count() as f32 * 7.0 <= chevron.unwrap().x, "{texts:?}");
        }

        for bordered in [false, true] {
            let frame = tree_of(Accordion::new("none", Vec::new(), palette()).bordered(bordered)).layout(WINDOW);
            assert!(frame.regions.is_empty() && frame.access.is_empty() && frame.commands.is_empty());
        }
    }
}
