//! The header row of a panel and the padded column of a sidebar.

use crate::{
    page::px, ButtonVariant, Component, ComponentPalette, ControlSize, Element, Icon, IconButton,
    Text, TextRole, TooltipPlacement, TooltipSpec,
};

/// Density of a [`PanelHeader`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelHeaderSizing {
    /// Height of the header.
    pub height: f32,
    /// Space around the row, inside the header.
    pub padding: f32,
    /// Space between the title and each tool.
    pub gap: f32,
    /// Type role of the title.
    pub title_role: TextRole,
    /// Center the tools in the header's height. `false` hangs them from the
    /// top inset, for a header whose tools were set there before.
    pub center_tools: bool,
}


/// The former name of [`PanelHeaderSizing`].
#[deprecated(since = "0.1.39", note = "renamed to `PanelHeaderSizing`")]
pub type HeaderSizing = PanelHeaderSizing;

impl PanelHeaderSizing {
    /// The heading of a sidebar: 26 high, tools 2 apart.
    pub const SIDEBAR: Self = Self {
        height: 26.0,
        padding: 0.0,
        gap: 2.0,
        title_role: TextRole::Caption,
        center_tools: true,
    };

    /// The bar over a pane, as tall as a tab strip: 32 high, inset 4.
    pub const PANE: Self = Self {
        height: 32.0,
        padding: 4.0,
        gap: 4.0,
        ..Self::SIDEBAR
    };

    /// The heading of a card: 28 high, inset 2, a body-sized title.
    pub const CARD: Self = Self {
        height: 28.0,
        padding: 2.0,
        gap: 4.0,
        title_role: TextRole::Body,
        ..Self::SIDEBAR
    };

    /// The heading of a popover: 24 high, tools 6 apart.
    pub const POPOVER: Self = Self {
        height: 24.0,
        gap: 6.0,
        ..Self::SIDEBAR
    };
}

/// One row over a panel: a back button, a title that takes the width left,
/// the panel's tools and a dismiss button. The header decides the height,
/// the inset, the gap and where the tools stand; it paints no fill.
pub struct PanelHeader {
    id: Option<String>,
    title: String,
    leading: Option<Element>,
    back: Option<(String, String)>,
    trailing: Vec<Element>,
    dismiss: Option<(String, String)>,
    width: Option<f32>,
    sizing: PanelHeaderSizing,
    palette: ComponentPalette,
}

impl PanelHeader {
    /// A title cut to one line, in the [`PanelHeaderSizing::SIDEBAR`] profile.
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: None,
            title: title.into(),
            leading: None,
            back: None,
            trailing: Vec::new(),
            dismiss: None,
            width: None,
            sizing: PanelHeaderSizing::SIDEBAR,
            palette,
        }
    }

    /// Make the title known to the platform by this ID, as a heading.
    /// Without one only the header's buttons are known.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// A header that holds this element where a title would be, such as a
    /// strip of tabs, a breadcrumb or a field: [`new`](Self::new) with
    /// [`leading`](Self::leading), without a title to leave unused.
    pub fn with_leading(element: Element, palette: ComponentPalette) -> Self {
        Self::new("", palette).leading(element)
    }

    /// Put this element in place of the title, which is then not shown. It
    /// takes the width the tools leave.
    pub fn leading(mut self, element: Element) -> Self {
        self.leading = Some(element);
        self
    }

    /// A back arrow before the title, reporting a click on `id`. An empty
    /// tooltip shows none.
    pub fn back(mut self, id: impl Into<String>, tooltip: impl Into<String>) -> Self {
        self.back = Some((id.into(), tooltip.into()));
        self
    }

    /// The panel's tools, after the title. A rule between two groups is a
    /// [`crate::Separator`] with a length in the vector.
    pub fn trailing(mut self, tools: Vec<Element>) -> Self {
        self.trailing = tools;
        self
    }

    /// A close button after the tools, reporting a click on `id`. An empty
    /// tooltip shows none.
    pub fn dismiss(mut self, id: impl Into<String>, tooltip: impl Into<String>) -> Self {
        self.dismiss = Some((id.into(), tooltip.into()));
        self
    }

    /// Fix the header's width. Values that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Height, inset, gap and title size.
    pub fn sizing(mut self, sizing: PanelHeaderSizing) -> Self {
        self.sizing = sizing;
        self
    }

    fn tool(&self, (id, tooltip): (String, String), icon: Icon) -> Element {
        // A header sits at the top of its panel, so its hints open downward.
        let hint = TooltipSpec::new(tooltip.as_str())
            .placement(TooltipPlacement::Below)
            .suppressed(tooltip.is_empty());
        IconButton::new(id, icon, self.palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .tooltip_with(hint)
            .render()
    }
}

impl Component for PanelHeader {
    fn render(mut self) -> Element {
        let sizing = self.sizing;
        let place = |tool: Element| if sizing.center_tools { tool.centered_y() } else { tool };
        let mut children = Vec::with_capacity(self.trailing.len() + 3);
        if let Some(back) = self.back.take() {
            children.push(place(self.tool(back, Icon::ArrowLeft)));
        }
        children.push(
            match self.leading.take() {
                Some(element) => element,
                None => {
                    let said = self.id.take().map(|id| {
                        crate::Access::new(crate::Role::Heading).id(id).name(self.title.clone())
                    });
                    let title = Text::new(std::mem::take(&mut self.title), self.palette)
                        .role(sizing.title_role)
                        .truncate()
                        .render();
                    match said {
                        Some(said) => title.accessible(said),
                        None => title,
                    }
                }
            }
            .flex(1.0),
        );
        children.extend(std::mem::take(&mut self.trailing).into_iter().map(place));
        if let Some(dismiss) = self.dismiss.take() {
            children.push(place(self.tool(dismiss, Icon::Close)));
        }
        Element::row(children)
            .gap(px(sizing.gap))
            .padding(px(sizing.padding))
            .size(self.width, Some(px(sizing.height)))
    }
}

// Space around the content of a sidebar and between its blocks.
const SIDEBAR_INSET: f32 = 6.0;

/// The body of a sidebar: its blocks in a column, inset from the edges, on
/// the palette's surface.
pub struct SidebarPanel {
    children: Vec<Element>,
    width: Option<f32>,
    height: Option<f32>,
    palette: ComponentPalette,
}

impl SidebarPanel {
    /// The blocks from top to bottom, filling whatever holds the panel.
    pub fn new(children: Vec<Element>, palette: ComponentPalette) -> Self {
        Self { children, width: None, height: None, palette }
    }

    /// Fix the panel's size. A value that is not finite leaves that axis to
    /// the container.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width.is_finite().then(|| width.max(0.0));
        self.height = height.is_finite().then(|| height.max(0.0));
        self
    }
}

impl Component for SidebarPanel {
    fn render(self) -> Element {
        Element::column(self.children)
            .gap(SIDEBAR_INSET)
            .padding(SIDEBAR_INSET)
            .background(self.palette.surface)
            .size(self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn tool(id: &str, palette: ComponentPalette) -> Element {
        IconButton::new(id, Icon::Refresh, palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .render()
    }

    #[test]
    fn the_title_with_an_id_is_a_heading_and_back_and_dismiss_are_named_by_their_hints() {
        let header = || PanelHeader::new("Archivos", ComponentPalette::neutral_dark()).back("back", "Volver").dismiss("close", "");
        let frame = tree(header().id("files").width(240.0).render()).layout(Size::new(240.0, 200.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        assert_eq!((node("files").role, node("files").name.as_str()), (crate::Role::Heading, "Archivos"));
        assert!(!node("files").actionable);
        assert_eq!((node("back").role, node("back").name.as_str()), (crate::Role::Button, "Volver"));
        // A hint the host left empty names nothing.
        assert_eq!(node("close").name, "");
        let frame = tree(header().render()).layout(Size::new(240.0, 200.0));
        assert_eq!(frame.access.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(), ["back", "close"]);
    }

    // The header at the top of a taller window, at its own height.
    fn tree(header: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![header, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(frame: &crate::Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .unwrap_or_else(|| panic!("{id} is a button"))
            .rect
    }

    fn title(frame: &crate::Frame) -> (Point, String, f32) {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { position, text, font_size, .. } => {
                    Some((*position, text.clone(), *font_size))
                }
                _ => None,
            })
            .expect("the title is painted")
    }

    #[test]
    fn each_profile_sets_the_height_the_inset_and_centers_the_tools() {
        let palette = ComponentPalette::neutral_dark();
        for sizing in [PanelHeaderSizing::SIDEBAR, PanelHeaderSizing::PANE, PanelHeaderSizing::CARD, PanelHeaderSizing::POPOVER] {
            for width in [180.0, 420.0] {
                let header = PanelHeader::new("Changes", palette)
                    .trailing(vec![tool("one", palette), tool("two", palette)])
                    .sizing(sizing)
                    .width(width)
                    .render();
                assert_eq!((header.style.width, header.style.height), (Some(width), Some(sizing.height)));
                let frame = tree(header).layout(Size::new(width, 200.0));
                let (one, two) = (region(&frame, "one"), region(&frame, "two"));
                assert_eq!((one.width, one.height), (24.0, 24.0));
                assert_eq!(one.y, (sizing.height - 24.0) / 2.0, "{sizing:?}");
                assert_eq!(two.x + two.width, width - sizing.padding);
                assert_eq!(two.x - (one.x + one.width), sizing.gap);
                let (position, _, size) = title(&frame);
                assert_eq!((position.x, size), (sizing.padding, sizing.title_role.size()));
            }
        }
    }

    #[test]
    fn tools_hang_from_the_top_inset_when_not_centered() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = PanelHeaderSizing { center_tools: false, ..PanelHeaderSizing::SIDEBAR };
        let header = PanelHeader::new("Changes", palette)
            .trailing(vec![tool("one", palette)])
            .sizing(sizing)
            .render();
        let frame = tree(header).layout(Size::new(240.0, 200.0));
        assert_eq!(region(&frame, "one").y, 0.0);
    }

    #[test]
    fn a_long_title_is_cut_before_the_tools() {
        let palette = ComponentPalette::neutral_dark();
        let header = PanelHeader::new("A title far longer than a narrow sidebar can show", palette)
            .trailing(vec![tool("one", palette)])
            .width(120.0)
            .render();
        let mut tree = tree(header);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(Size::new(120.0, 200.0));
        let (_, text, _) = title(&frame);
        assert!(text.ends_with('…'), "{text}");
        assert!(text.chars().count() as f32 * 8.0 <= region(&frame, "one").x);
    }

    #[test]
    fn back_and_dismiss_stand_at_the_ends_and_report_their_ids() {
        let palette = ComponentPalette::neutral_dark();
        let header = || {
            PanelHeader::new("Commit", palette)
                .back("back", "Back to changes")
                .trailing(vec![tool("one", palette)])
                .dismiss("close", "")
                .sizing(PanelHeaderSizing::CARD)
                .render()
        };
        let mut tree = tree(header());
        let frame = tree.layout(Size::new(300.0, 200.0));
        let (back, one, close) = (region(&frame, "back"), region(&frame, "one"), region(&frame, "close"));
        assert_eq!((back.x, back.y), (2.0, 2.0));
        assert_eq!(close.x + close.width, 298.0);
        assert!(one.x + one.width <= close.x);
        assert!(title(&frame).0.x >= back.x + back.width);
        for (id, rect) in [("back", back), ("close", close)] {
            let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(center));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: id.into() }]);
        }
    }

    #[test]
    fn a_leading_element_replaces_the_title_and_takes_the_width_left() {
        let palette = ComponentPalette::neutral_dark();
        let field = Element::spacer().background(palette.raised).size(None, Some(24.0));
        let header = PanelHeader::new("unused", palette)
            .leading(field)
            .trailing(vec![tool("one", palette)])
            .sizing(PanelHeaderSizing::PANE)
            .render();
        let frame = tree(header).layout(Size::new(300.0, 200.0));
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { .. })));
        let fill = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == palette.raised => Some(*rect),
                _ => None,
            })
            .expect("the field is painted");
        assert_eq!((fill.x, fill.width), (4.0, 300.0 - 4.0 - 4.0 - 24.0 - 4.0));
    }

    #[test]
    fn bad_numbers_do_not_reach_the_layout() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = PanelHeaderSizing { height: f32::NAN, padding: -4.0, gap: f32::INFINITY, ..PanelHeaderSizing::SIDEBAR };
        let header = PanelHeader::new("Changes", palette).sizing(sizing).width(f32::NAN).render();
        assert_eq!((header.style.width, header.style.height), (None, Some(0.0)));
        assert_eq!((header.style.padding, header.style.gap), (0.0, 0.0));

        let panel = SidebarPanel::new(Vec::new(), palette).size(f32::NAN, -10.0).render();
        assert_eq!((panel.style.width, panel.style.height), (None, Some(0.0)));
    }

    #[test]
    fn a_sidebar_panel_insets_its_blocks_on_the_surface() {
        let palette = ComponentPalette::neutral_dark();
        let mark = crate::Color::rgb(1, 2, 3);
        let block = |height| Element::spacer().background(mark).size(None, Some(height));
        for width in [180.0, 320.0] {
            let panel = SidebarPanel::new(vec![block(26.0), block(40.0)], palette)
                .size(width, 300.0)
                .render();
            let frame = UiTree::new(Element::row(vec![panel, Element::spacer().flex(1.0)]))
                .layout(Size::new(500.0, 300.0));
            let fills: Vec<(Rect, crate::Color)> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } => Some((*rect, *color)),
                    _ => None,
                })
                .collect();
            assert_eq!(fills[0], (Rect::new(0.0, 0.0, width, 300.0), palette.surface));
            assert_eq!(fills[1].0, Rect::new(6.0, 6.0, width - 12.0, 26.0));
            assert_eq!(fills[2].0.y, 6.0 + 26.0 + 6.0);
            assert!(frame.regions.is_empty(), "a panel is passive");
        }
    }
}
