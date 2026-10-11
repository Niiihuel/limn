//! Folder tabs for open resources: a scrolling strip whose active tab opens
//! onto the body beneath it, and the bar that holds the strip and its tools.

use crate::{
    tokens, Color, Component, ComponentPalette, CornerRadii, Element, FontFamily, Icon,
    RadiusRole, Separator, TextRole, Tooltip, TooltipPlacement, ICON_SIZE,
};

fn dimension(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// One tab of a [`TabStrip`]: an icon, a title that fades when it does not
/// fit, and a trailing slot for the host's own close button.
///
/// The host owns the width; limn never estimates one from the title. Clicking
/// the tab reports `Clicked` with `id`. `extent_id` names the passive area of
/// the whole tab, close slot included, which is what the strip scrolls into
/// view and where the tooltip also opens.
pub struct TabItem {
    id: String,
    extent_id: String,
    title: String,
    tooltip: Option<String>,
    icon: Icon,
    active: bool,
    width: f32,
    close: Option<Element>,
    title_ink: Option<Color>,
    tooltip_placement: TooltipPlacement,
}

impl TabItem {
    /// A width that is negative or not finite is no width at all.
    pub fn new(
        id: impl Into<String>,
        extent_id: impl Into<String>,
        icon: Icon,
        title: impl Into<String>,
        width: f32,
    ) -> Self {
        Self {
            id: id.into(),
            extent_id: extent_id.into(),
            title: title.into(),
            tooltip: None,
            icon,
            active: false,
            width: dimension(width),
            close: None,
            title_ink: None,
            tooltip_placement: TooltipPlacement::Above,
        }
    }

    /// Text of the tooltip, such as a full path. The default is the title.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    /// Where the tooltip opens. The default is above the tab.
    pub fn tooltip_placement(mut self, placement: TooltipPlacement) -> Self {
        self.tooltip_placement = placement;
        self
    }

    /// Mark the tab as the one whose content is shown.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The control in the trailing slot, built by the host so each strip
    /// keeps its own close button and ink. It is centered in the
    /// [`TabStrip::CLOSE_SIZE`] slot and should be 20 px square, so its hover
    /// stays inside the tab's rounded corner; without one the slot stays empty.
    pub fn close(mut self, close: Element) -> Self {
        self.close = Some(close);
        self
    }

    /// Ink of the title, active or not, for a state the host marks by color,
    /// such as a file's status in version control. The icon keeps the
    /// palette's ink. The default is `text` when active and `muted` otherwise.
    pub fn title_ink(mut self, ink: Color) -> Self {
        self.title_ink = Some(ink);
        self
    }

    /// The tab alone, for a group that shows a single resource. Colors follow
    /// the roles documented on [`TabStrip`].
    pub fn render(self, palette: ComponentPalette) -> Element {
        let ink = if self.active { palette.text } else { palette.muted };
        let close = self
            .close
            .map_or_else(Element::spacer, Element::centered)
            .size(Some(TabStrip::CLOSE_SIZE), Some(TabStrip::CLOSE_SIZE));
        let content = Element::row(vec![
            Element::icon(self.icon)
                .foreground(ink)
                .size(Some(ICON_SIZE), Some(TabStrip::CLOSE_SIZE)),
            Element::faded_text(&self.title, 14.0)
                .font_family(FontFamily::Sans)
                .font_size(TextRole::Label.size())
                .foreground(self.title_ink.unwrap_or(ink))
                .flex(1.0)
                .size(None, Some(TabStrip::CLOSE_SIZE)),
            close,
        ])
        .gap(6.0)
        .size(None, Some(TabStrip::TOOL_SIZE))
        .flex(1.0);
        let content = Element::row(vec![
            Element::spacer().size(Some(8.0), None),
            content,
            Element::spacer().size(Some(2.0), None),
        ])
        .size(None, Some(TabStrip::TOOL_SIZE));
        let tooltip = self.tooltip.unwrap_or(self.title);
        let select = TabStrip::surface(&self.id, content, self.active, self.width, palette);
        let select = Tooltip::new(select, self.id, &tooltip, palette)
            .placement(self.tooltip_placement)
            .render();
        // Passive full-width extent includes the independently clickable close.
        Tooltip::new(select, self.extent_id, tooltip, palette)
            .placement(self.tooltip_placement)
            .render()
            .size(Some(self.width), Some(TabStrip::TAB_HEIGHT))
    }
}

/// A strip of folder tabs that scrolls sideways when they do not fit.
///
/// The host owns which tab is active and how wide each one is. The strip
/// draws a short rule between neighbouring inactive tabs, fades the edge that
/// hides more tabs, and keeps the active tab in view.
///
/// Palette roles:
///
/// | Field | Paints |
/// | --- | --- |
/// | `surface` | The active tab, which must match the body under the strip. |
/// | `raised` | The bar behind the tabs. |
/// | `hover` | An inactive tab under the pointer. |
/// | `border` | The baseline and the rules between tabs. |
/// | `text`, `muted` | Active and inactive ink. |
pub struct TabStrip {
    scroll_id: String,
    tabs: Vec<StripTab>,
    palette: ComponentPalette,
}

struct StripTab {
    id: String,
    extent_id: String,
    width: f32,
    active: bool,
    element: Element,
}

impl TabStrip {
    /// Height of the bar that holds a strip and its tools.
    pub const HEADER_HEIGHT: f32 = 32.0;
    /// Height of a tab: its content and the connection to the body.
    pub const TAB_HEIGHT: f32 = 28.0;
    /// Height of a tab's content, and the side of a tool beside the strip.
    pub const TOOL_SIZE: f32 = 24.0;
    /// Side of the slot a tab reserves for its close button.
    pub const CLOSE_SIZE: f32 = 24.0;
    /// Default space between the bar's edges and its content.
    pub const HEADER_SIDE_INSET: f32 = 6.0;

    /// `scroll_id` identifies the scroll position between frames.
    pub fn new(
        scroll_id: impl Into<String>,
        items: Vec<TabItem>,
        palette: ComponentPalette,
    ) -> Self {
        let tabs = items
            .into_iter()
            .map(|item| StripTab {
                id: item.id.clone(),
                extent_id: item.extent_id.clone(),
                width: item.width,
                active: item.active,
                element: item.render(palette),
            })
            .collect();
        Self { scroll_id: scroll_id.into(), tabs, palette }
    }

    /// Wrap each rendered tab, by position, in something the host owns, such
    /// as a context menu. The wrapper must keep the tab's size.
    pub fn each_tab(mut self, mut wrap: impl FnMut(usize, Element) -> Element) -> Self {
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let element = std::mem::replace(&mut tab.element, Element::spacer());
            tab.element = wrap(index, element);
        }
        self
    }

    /// The clickable surface of one tab around caller content
    /// [`TabStrip::TOOL_SIZE`] tall, for tabs that are only a label or an
    /// icon. An inactive tab keeps the baseline beneath it; the active one
    /// opens onto the body.
    pub fn surface(
        id: &str,
        content: Element,
        active: bool,
        width: f32,
        palette: ComponentPalette,
    ) -> Element {
        let connection = if active {
            Element::spacer()
        } else {
            // Hovering an inactive tab must not erase the boundary with the body.
            Element::column(vec![
                Element::spacer().flex(1.0),
                Separator::horizontal(palette).render(),
            ])
        }
        .size(None, Some(Self::TAB_HEIGHT - Self::TOOL_SIZE));
        Element::clickable(
            id,
            Element::column(vec![content.size(None, Some(Self::TOOL_SIZE)), connection]),
        )
        .background(if active { palette.surface } else { Color::TRANSPARENT })
        .hover_background(if active { palette.surface } else { palette.hover })
        .pressed_background(palette.surface)
        .transition_ms(tokens().motion.hover_ms)
        .corner_radii(CornerRadii::top(RadiusRole::Card.px()))
        .size(Some(dimension(width)), Some(Self::TAB_HEIGHT))
    }

    /// Set a row of tabs on the bottom of a [`TabStrip::HEADER_HEIGHT`] bar.
    pub fn align(tabs: Element) -> Element {
        let width = tabs.style.width;
        Element::column(vec![
            Element::spacer().size(None, Some(Self::HEADER_HEIGHT - Self::TAB_HEIGHT)),
            tabs,
        ])
        .size(width, Some(Self::HEADER_HEIGHT))
    }

    /// The bar that holds tabs in `leading` and fixed tools in `trailing`.
    pub fn header(
        leading: Element,
        trailing: Element,
        width: f32,
        palette: ComponentPalette,
    ) -> TabStripHeader {
        TabStripHeader {
            leading,
            trailing,
            width: dimension(width),
            side_inset: Self::HEADER_SIDE_INSET,
            gap: 8.0,
            palette,
        }
    }
}

impl Component for TabStrip {
    fn render(self) -> Element {
        let content_width = self.tabs.iter().map(|tab| tab.width).sum::<f32>();
        let count = self.tabs.len();
        let active = self.tabs.iter().position(|tab| tab.active);
        let reveal = active.map(|index| self.tabs[index].extent_id.clone());
        let action_ids = self.tabs.iter().map(|tab| tab.id.clone()).collect();
        let palette = self.palette;
        let tabs = self
            .tabs
            .into_iter()
            .enumerate()
            .map(|(index, tab)| {
                // The active tab's own edges separate it from its neighbours.
                if index + 1 == count || active == Some(index) || active == Some(index + 1) {
                    return tab.element;
                }
                let separator = Element::column(vec![
                    Element::spacer().size(None, Some(5.0)),
                    Element::row(vec![
                        Element::spacer().flex(1.0),
                        Separator::vertical(palette).length(14.0).render(),
                    ])
                    .size(None, Some(14.0)),
                ])
                .size(Some(tab.width), Some(Self::TAB_HEIGHT));
                Element::overlay(tab.element, separator)
                    .size(Some(tab.width), Some(Self::TAB_HEIGHT))
                    .clip_children()
            })
            .collect();
        let row = Element::tab_strip(tabs, action_ids)
            .size(Some(content_width), Some(Self::TAB_HEIGHT));
        let strip = Element::horizontal_scroll(self.scroll_id, row)
            .horizontal_edge_fade(palette.raised, 16.0)
            .size(None, Some(Self::TAB_HEIGHT));
        let strip = match reveal {
            Some(reveal) => strip.reveal_horizontal_item(reveal),
            None => strip,
        };
        Self::align(strip).flex(1.0)
    }
}

/// The bar built by [`TabStrip::header`]. Its baseline lies behind the tabs,
/// so only the active tab opens onto the body, and the tools stay centered
/// and outside the scrolling part.
pub struct TabStripHeader {
    leading: Element,
    trailing: Element,
    width: f32,
    side_inset: f32,
    gap: f32,
    palette: ComponentPalette,
}

impl TabStripHeader {
    /// Space between each edge of the bar and its content. The default is
    /// [`TabStrip::HEADER_SIDE_INSET`]. Values that are not finite are ignored.
    pub fn side_inset(mut self, inset: f32) -> Self {
        if inset.is_finite() {
            self.side_inset = inset.max(0.0);
        }
        self
    }

    /// Space between the tabs and the tools. The default is 8. Values that
    /// are not finite are ignored.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
        self
    }
}

impl Component for TabStripHeader {
    fn render(self) -> Element {
        let height = TabStrip::HEADER_HEIGHT;
        let trailing_width = self.trailing.style.width;
        let trailing = self.trailing.centered_y().size(trailing_width, Some(height));
        let content = Element::row(vec![
            Element::spacer().size(Some(self.side_inset), None),
            Element::row(vec![self.leading, trailing]).gap(self.gap).flex(1.0),
            Element::spacer().size(Some(self.side_inset), None),
        ])
        .size(Some(self.width), Some(height));
        Element::overlay(
            Element::column(vec![
                Element::spacer().flex(1.0),
                Separator::horizontal(self.palette).render(),
            ])
            .background(self.palette.raised)
            .size(Some(self.width), Some(height)),
            content,
        )
        .size(Some(self.width), Some(height))
        .clip_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const NAMES: [&str; 3] = ["main.rs", "a_much_longer_file_name.rs", "lib.rs"];

    /// A palette whose six roles are all different, so a test can tell them apart.
    fn palette() -> ComponentPalette {
        ComponentPalette {
            surface: Color::rgb(10, 10, 10),
            raised: Color::rgb(20, 20, 20),
            hover: Color::rgb(30, 30, 30),
            ..ComponentPalette::neutral_dark()
        }
    }

    fn items(count: usize, active: Option<usize>, width: f32) -> Vec<TabItem> {
        (0..count)
            .map(|index| {
                TabItem::new(
                    format!("tab:{index}"),
                    format!("extent:{index}"),
                    Icon::File,
                    NAMES[index % NAMES.len()],
                    width,
                )
                .active(active == Some(index))
            })
            .collect()
    }

    fn strip_frame(count: usize, active: Option<usize>, width: f32) -> (UiTree, Frame) {
        let strip = TabStrip::new("tabs", items(count, active, 120.0), palette()).render();
        let mut tree = UiTree::new(Element::column(vec![
            TabStrip::header(strip, Element::spacer().size(Some(0.0), None), width, palette())
                .gap(0.0)
                .render(),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(width, 80.0));
        (tree, frame)
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap_or_else(|| panic!("no region {id}"))
            .rect
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

    /// Tab surfaces: fills with rounded top corners and square bottom ones.
    fn tab_fills(frame: &Frame, color: Color) -> Vec<Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillCornerRect { rect, color: fill, radii }
                    if *fill == color && *radii == CornerRadii::top(RadiusRole::Card.px()) =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn title_ink_colors_the_title_alone_whether_the_tab_is_active_or_not() {
        let palette = palette();
        let ink = Color::rgb(200, 150, 50);
        let inks = |frame: &Frame| -> Vec<(String, Color)> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
                    _ => None,
                })
                .collect()
        };
        let icons = |frame: &Frame| -> Vec<Color> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { tint, .. } => Some(*tint),
                    _ => None,
                })
                .collect()
        };
        for active in [None, Some(0)] {
            let frame_of = |tinted: bool| {
                let mut items = items(2, active, 120.0);
                if tinted {
                    let first = items.remove(0);
                    items.insert(0, first.title_ink(ink));
                }
                let mut tree = UiTree::new(TabStrip::new("tabs", items, palette).render());
                tree.set_reduced_motion(true);
                tree.layout(Size::new(420.0, 80.0))
            };
            let (plain, tinted) = (frame_of(false), frame_of(true));
            let rest = if active == Some(0) { palette.text } else { palette.muted };
            assert_eq!(inks(&plain)[0], (NAMES[0].to_owned(), rest));
            assert_eq!(inks(&tinted)[0], (NAMES[0].to_owned(), ink));
            // The other tab, every icon and every region stay as they were.
            assert_eq!(inks(&plain)[1..], inks(&tinted)[1..]);
            assert_eq!(icons(&plain), icons(&tinted));
            assert_eq!(plain.regions, tinted.regions);
        }
    }

    #[test]
    fn tabs_sit_on_the_bottom_of_the_bar_at_the_widths_they_were_given() {
        for width in [420.0, 900.0] {
            let (_, frame) = strip_frame(3, Some(1), width);
            for index in 0..3 {
                let tab = region(&frame, &format!("tab:{index}"));
                assert_eq!(
                    (tab.x, tab.y, tab.width, tab.height),
                    (
                        TabStrip::HEADER_SIDE_INSET + 120.0 * index as f32,
                        TabStrip::HEADER_HEIGHT - TabStrip::TAB_HEIGHT,
                        120.0,
                        TabStrip::TAB_HEIGHT
                    ),
                    "width={width}"
                );
            }
        }
    }

    #[test]
    fn only_the_active_tab_is_filled_and_it_opens_onto_the_body() {
        let palette = palette();
        let (_, frame) = strip_frame(3, Some(1), 420.0);
        let active = region(&frame, "tab:1");
        assert_eq!(tab_fills(&frame, palette.surface), vec![active]);
        // The bar's baseline spans the whole width behind the tabs...
        let bar = Rect::new(0.0, TabStrip::HEADER_HEIGHT - 1.0, 420.0, 1.0);
        let rules = fills(&frame, palette.border);
        assert!(rules.contains(&bar), "{rules:?}");
        // ...and each inactive tab repeats it, so a hover fill cannot erase it.
        let baselines: Vec<_> = rules
            .iter()
            .filter(|rule| rule.height == 1.0 && rule.width == 120.0)
            .map(|rule| (rule.x, rule.y))
            .collect();
        let y = TabStrip::HEADER_HEIGHT - 1.0;
        assert_eq!(baselines, vec![(region(&frame, "tab:0").x, y), (region(&frame, "tab:2").x, y)]);
    }

    #[test]
    fn separators_stand_between_inactive_neighbours_only() {
        let palette = palette();
        let separators = |count, active| {
            let (_, frame) = strip_frame(count, active, 900.0);
            fills(&frame, palette.border)
                .into_iter()
                .filter(|rule| rule.width == 1.0)
                .map(|rule| (rule.x, rule.y, rule.height))
                .collect::<Vec<_>>()
        };
        let inset = TabStrip::HEADER_SIDE_INSET;
        let top = TabStrip::HEADER_HEIGHT - TabStrip::TAB_HEIGHT + 5.0;
        // Right edge of every tab but the last.
        assert_eq!(
            separators(3, None),
            vec![(inset + 119.0, top, 14.0), (inset + 239.0, top, 14.0)]
        );
        // None on either side of the active tab.
        assert_eq!(separators(3, Some(1)), vec![]);
        assert_eq!(separators(3, Some(0)), vec![(inset + 239.0, top, 14.0)]);
        assert_eq!(separators(1, None), vec![]);
    }

    #[test]
    fn an_inactive_tab_hovers_and_a_click_reports_its_id() {
        let palette = palette();
        let (mut tree, frame) = strip_frame(3, Some(1), 420.0);
        let tab = region(&frame, "tab:0");
        // Left of the close slot, which is passive here.
        let point = Point::new(tab.x + 20.0, tab.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(point));
        let hovered = tree.layout(Size::new(420.0, 80.0));
        assert_eq!(tab_fills(&hovered, palette.hover), vec![tab]);
        assert_eq!(tab_fills(&hovered, palette.surface), vec![region(&frame, "tab:1")]);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "tab:0".into() }]
        );
    }

    #[test]
    fn the_active_tab_is_scrolled_into_view() {
        // Twelve 120 px tabs in 420 px: the last starts far outside.
        let (_, frame) = strip_frame(12, Some(11), 420.0);
        let active = region(&frame, "tab:11");
        assert!(active.x >= 0.0 && active.x + active.width <= 420.0, "{active:?}");
        let (_, frame) = strip_frame(12, Some(0), 420.0);
        assert_eq!(region(&frame, "tab:0").x, TabStrip::HEADER_SIDE_INSET);
    }

    #[test]
    fn each_tab_wraps_by_position_and_a_close_control_keeps_its_own_action() {
        let mut items = items(2, Some(0), 120.0);
        let close = Element::clickable("close:1", Element::spacer()).size(Some(20.0), Some(20.0));
        let last = items.pop().unwrap().close(close);
        items.push(last);
        let mut seen = Vec::new();
        let strip = TabStrip::new("tabs", items, palette()).each_tab(|index, tab| {
            seen.push(index);
            tab
        });
        assert_eq!(seen, vec![0, 1]);
        let mut tree = UiTree::new(Element::row(vec![strip.render()]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(420.0, TabStrip::HEADER_HEIGHT));
        let (tab, close) = (region(&frame, "tab:1"), region(&frame, "close:1"));
        // The slot ends 2 px before the tab's right edge and the control is
        // centered in it, clear of the tab's top edge and rounded corner.
        assert_eq!(
            (close.x + close.width, close.y, close.width, close.height),
            (tab.x + tab.width - 4.0, tab.y + 2.0, 20.0, 20.0)
        );
        let point = Point::new(close.x + 10.0, close.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "close:1".into() }]
        );
    }

    #[test]
    fn the_header_centers_its_tools_and_honours_inset_and_gap() {
        let palette = palette();
        let tool = || {
            Element::clickable("tool", Element::spacer())
                .size(Some(TabStrip::TOOL_SIZE), Some(TabStrip::TOOL_SIZE))
        };
        let lay = |header: TabStripHeader, width: f32| {
            let mut tree = UiTree::new(Element::column(vec![
                header.render(),
                Element::spacer().flex(1.0),
            ]));
            tree.layout(Size::new(width, 80.0))
        };
        let leading = || Element::clickable("lead", Element::spacer()).flex(1.0);
        for width in [240.0, 900.0] {
            let frame = lay(TabStrip::header(leading(), tool(), width, palette), width);
            let (lead, tool_rect) = (region(&frame, "lead"), region(&frame, "tool"));
            assert_eq!(fills(&frame, palette.raised), vec![Rect::new(0.0, 0.0, width, 32.0)]);
            assert_eq!((tool_rect.x, tool_rect.y), (width - 6.0 - 24.0, 4.0));
            assert_eq!((lead.x, lead.x + lead.width), (6.0, tool_rect.x - 8.0));

            let frame = lay(
                TabStrip::header(leading(), tool(), width, palette).side_inset(2.0).gap(2.0),
                width,
            );
            let (lead, tool_rect) = (region(&frame, "lead"), region(&frame, "tool"));
            assert_eq!(tool_rect.x, width - 2.0 - 24.0);
            assert_eq!((lead.x, lead.x + lead.width), (2.0, tool_rect.x - 2.0));

            // Bad input leaves the defaults.
            let frame = lay(
                TabStrip::header(leading(), tool(), width, palette)
                    .side_inset(f32::NAN)
                    .gap(f32::INFINITY),
                width,
            );
            assert_eq!(region(&frame, "tool").x, width - 6.0 - 24.0);
        }
    }

    #[test]
    fn widths_that_are_not_usable_collapse_to_nothing() {
        for width in [f32::NAN, f32::INFINITY, -40.0] {
            let item = TabItem::new("tab", "extent", Icon::File, "main.rs", width);
            assert_eq!(item.width, 0.0);
            let surface = TabStrip::surface("tab", Element::spacer(), false, width, palette());
            assert_eq!(surface.style.width, Some(0.0));
            let header =
                TabStrip::header(Element::spacer(), Element::spacer(), width, palette()).render();
            assert_eq!(header.style.width, Some(0.0));
        }
    }
}
