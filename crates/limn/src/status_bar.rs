//! The bar at the foot of a window: cells of status that give way by priority.

use crate::{
    page::px, Color, Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, Element,
    FontFamily, Icon, PopoverPlacement, Separator, TooltipSpec,
};

// Height of the glyphs and of the text line inside a cell.
const LINE: f32 = 16.0;
const FONT_SIZE: f32 = 11.0;
// Width over which a label that does not fit goes transparent.
const FADE: f32 = 6.0;
const PADDING: f32 = 2.0;
const MENU_WIDTH: f32 = 256.0;

/// Density of a [`StatusBarItem`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StatusBarItemSizing {
    /// Width of the column of each glyph.
    pub icon_width: f32,
    /// Space between the glyphs, the label and the counts.
    pub gap: f32,
}

impl StatusBarItemSizing {
    /// A glyph 4 from its label.
    pub const LABEL: Self = Self { icon_width: 12.0, gap: 4.0 };
    /// Glyphs 3 from their counts, so each pair reads as one figure.
    pub const COUNTS: Self = Self { gap: 3.0, ..Self::LABEL };
}

// Height of the hairline of a divider, centered in the bar.
const DIVIDER: f32 = 11.0;

enum Part {
    /// With the color of its own, if it has one.
    Icon(Icon, Option<Color>),
    Label(String),
    Count(usize),
}

struct Menu {
    entries: Vec<DropdownMenuEntry>,
    open: bool,
    dismiss_id: String,
    palette: ComponentPalette,
}

/// One cell of a [`StatusBar`]: glyphs, a label and counts in the order they
/// are added, across the bar's whole height. The host owns the width, since
/// it knows the text; the cell reports a click on its ID and hovers with
/// square corners, as its neighbours do.
pub struct StatusBarItem {
    id: String,
    width: f32,
    parts: Vec<Part>,
    priority: i32,
    passive: bool,
    centered: bool,
    strong: bool,
    ink: Option<Color>,
    tooltip: Option<TooltipSpec>,
    action: Option<String>,
    menu: Option<Menu>,
    hosted: Option<Element>,
    sizing: StatusBarItemSizing,
    palette: ComponentPalette,
}

impl StatusBarItem {
    /// An empty cell of this width. A width that is not finite is none.
    pub fn new(id: impl Into<String>, width: f32, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            width: px(width),
            parts: Vec::new(),
            priority: 0,
            passive: false,
            centered: false,
            strong: false,
            ink: None,
            tooltip: None,
            action: None,
            menu: None,
            hosted: None,
            sizing: StatusBarItemSizing::LABEL,
            palette,
        }
    }

    /// A hairline between two groups of cells, shorter than the bar: a
    /// passive cell one pixel wide.
    pub fn divider(id: impl Into<String>, palette: ComponentPalette) -> Self {
        let rule = Separator::vertical(palette).length(DIVIDER).centered_in(StatusBar::HEIGHT);
        Self::new(id, 1.0, palette).hosted(rule.render())
    }

    /// A glyph. Each call adds one more.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.parts.push(Part::Icon(icon, None));
        self
    }

    /// A glyph in a color of its own whatever the cell's ink, such as the
    /// one of a severity. Each call adds one more.
    pub fn tinted_icon(mut self, icon: Icon, color: Color) -> Self {
        self.parts.push(Part::Icon(icon, Some(color)));
        self
    }

    /// Muted text that takes the width left and fades where it does not
    /// fit. An empty label is no label at all.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        let label = label.into();
        if !label.is_empty() {
            self.parts.push(Part::Label(label));
        }
        self
    }

    /// A muted number at its own width, after the glyph it counts. Each call
    /// adds one more.
    pub fn count(mut self, count: usize) -> Self {
        self.parts.push(Part::Count(count));
        self
    }

    /// Which cells stay when the bar is too narrow: the higher stays longer.
    /// It matters only among the trailing items of a [`StatusBar`].
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Information without an action: no hit region and no hover, and the
    /// hint stays.
    pub fn passive(mut self) -> Self {
        self.passive = true;
        self
    }

    /// Center the content in the cell instead of starting it at the leading
    /// edge, for a glyph alone or with its count.
    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    /// Set the label in the text ink instead of the muted one, for the one
    /// value the bar is read for, such as the branch. Counts stay muted.
    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    /// Paint the glyphs in a color that carries meaning the host owns, such
    /// as unread. Text stays muted.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// The hint shown over the cell, interactive or passive.
    pub fn tooltip_with(mut self, spec: TooltipSpec) -> Self {
        self.tooltip = Some(spec);
        self
    }

    /// Report `action` instead of the cell's own ID, which stays its focus
    /// target.
    pub fn dispatch_as(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// A menu of actions that opens upward from the cell's trailing edge.
    /// The host owns `open` and receives `dismiss_id`.
    pub fn menu(
        mut self,
        entries: Vec<DropdownMenuEntry>,
        open: bool,
        dismiss_id: impl Into<String>,
        menu_palette: ComponentPalette,
    ) -> Self {
        self.menu = Some(Menu { entries, open, dismiss_id: dismiss_id.into(), palette: menu_palette });
        self
    }

    /// Stand an element the host built in this cell's place, such as the
    /// cell rendered and wrapped in a popover. The bar keeps the width and
    /// the priority.
    pub fn hosted(mut self, element: Element) -> Self {
        self.hosted = Some(element);
        self
    }

    /// The cell's density. [`StatusBarItemSizing::LABEL`] unless set.
    pub fn sizing(mut self, sizing: StatusBarItemSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for StatusBarItem {
    fn render(self) -> Element {
        if let Some(element) = self.hosted {
            return element;
        }
        let palette = self.palette;
        let passive = self.passive;
        let text = |value: String, ink: Color| {
            let text = Element::faded_text(value, FADE)
                .font_family(FontFamily::Sans)
                .font_size(FONT_SIZE)
                .foreground(ink);
            // Passive text takes the cell's own height instead of the line
            // box, as the cells this replaced had it: half a pixel lower.
            if passive {
                text
            } else {
                text.size(None, Some(LINE))
            }
        };
        let icon_width = px(self.sizing.icon_width);
        let ink = self.ink.unwrap_or(palette.muted);
        let label_ink = if self.strong { palette.text } else { palette.muted };
        let children = self
            .parts
            .into_iter()
            .map(|part| match part {
                Part::Icon(icon, tint) => {
                    Element::icon(icon).foreground(tint.unwrap_or(ink)).size(Some(icon_width), Some(LINE))
                }
                Part::Label(label) => text(label, label_ink).flex(1.0),
                Part::Count(count) => text(count.to_string(), palette.muted),
            })
            .collect();
        let mut content = Element::row(children).gap(px(self.sizing.gap));
        if self.centered {
            content = content.centered_x();
        }
        let cell = if passive {
            Element::panel(content)
        } else {
            let target = Element::clickable(&self.id, content);
            palette.bar_item_surface(match self.action {
                Some(action) => target.dispatch_as(action),
                None => target,
            })
        }
        .padding(PADDING)
        .size(Some(self.width), Some(StatusBar::HEIGHT));
        let cell = match self.tooltip {
            Some(spec) => spec.wrap(cell, self.id, palette),
            None => cell,
        };
        match self.menu {
            Some(menu) => DropdownMenu::new(cell, menu.entries, menu.open, menu.dismiss_id, menu.palette)
                .context_menu_style()
                .placement(PopoverPlacement::TopEnd)
                .width(MENU_WIDTH)
                .render(),
            None => cell,
        }
    }
}

/// A row of [`StatusBarItem`]s: the leading ones always stand, and the
/// trailing ones are offered the room that is left, the highest priority
/// first. One that does not fit is left out, and a narrower one of lower
/// priority may still stay. The bar adds up the widths it is given and
/// measures no text.
pub struct StatusBar {
    leading: Vec<StatusBarItem>,
    trailing: Vec<StatusBarItem>,
    overflow: Option<StatusBarItem>,
    width: Option<f32>,
    trailing_budget: Option<f32>,
    top_hairline: bool,
    palette: ComponentPalette,
}

impl StatusBar {
    /// Height of the bar and of every cell, without the hairline.
    pub const HEIGHT: f32 = 21.0;
    /// Space between two cells.
    pub const GAP: f32 = 4.0;

    /// The trailing items are given in the order they are shown.
    pub fn new(leading: Vec<StatusBarItem>, trailing: Vec<StatusBarItem>, palette: ComponentPalette) -> Self {
        Self {
            leading,
            trailing,
            overflow: None,
            width: None,
            trailing_budget: None,
            top_hairline: false,
            palette,
        }
    }

    /// The bar's width, which the trailing items are fitted to. Without it
    /// every item is shown. Values that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// A cell shown at the end only while some trailing item does not fit,
    /// such as a More button whose menu the host fills. Its width is set
    /// aside before the others are fitted.
    pub fn overflow(mut self, item: StatusBarItem) -> Self {
        self.overflow = Some(item);
        self
    }

    /// The width the trailing items may share, for a host that reserves
    /// space by its own rule. By default it is what the leading items and
    /// their gaps leave of the width. Values that are not finite are ignored.
    pub fn trailing_budget(mut self, budget: f32) -> Self {
        if budget.is_finite() {
            self.trailing_budget = Some(budget.max(0.0));
        }
        self
    }

    /// For a bar that closes a panel of its own: a hairline above it and the
    /// palette's surface behind it, one pixel taller in all.
    pub fn top_hairline(mut self, hairline: bool) -> Self {
        self.top_hairline = hairline;
        self
    }

    // Which trailing items stay, and whether the overflow cell is shown.
    fn fit(&self) -> (Vec<bool>, bool) {
        let all = vec![true; self.trailing.len()];
        let budget = match (self.trailing_budget, self.width) {
            (Some(budget), _) => budget,
            // One more gap than there are leading items, for the spacer.
            (None, Some(width)) => (width
                - self.leading.iter().map(|item| item.width + Self::GAP).sum::<f32>()
                - Self::GAP)
                .max(0.0),
            (None, None) => return (all, false),
        };
        let wanted: f32 = self.trailing.iter().map(|item| item.width + Self::GAP).sum();
        if wanted <= budget {
            return (all, false);
        }
        let reserved = self.overflow.as_ref().map(|item| item.width + Self::GAP);
        let mut remaining = (budget - reserved.unwrap_or(0.0)).max(0.0);
        let mut order: Vec<usize> = (0..self.trailing.len()).collect();
        // Stable: among equals the first given is the first kept.
        order.sort_by_key(|index| std::cmp::Reverse(self.trailing[*index].priority));
        let mut visible = vec![false; self.trailing.len()];
        for index in order {
            let needed = self.trailing[index].width + Self::GAP;
            if needed <= remaining {
                visible[index] = true;
                remaining -= needed;
            }
        }
        let more = self.overflow.as_ref().is_some_and(|item| budget >= item.width);
        (visible, more)
    }
}

impl Component for StatusBar {
    fn render(self) -> Element {
        let (visible, more) = self.fit();
        let mut cells: Vec<Element> = self.leading.into_iter().map(Component::render).collect();
        cells.push(Element::spacer().flex(1.0));
        cells.extend(
            self.trailing
                .into_iter()
                .zip(visible)
                .filter(|(_, visible)| *visible)
                .map(|(item, _)| item.render()),
        );
        cells.extend(self.overflow.filter(|_| more).map(Component::render));
        let row = Element::row(cells)
            .gap(Self::GAP)
            .size(self.width, Some(Self::HEIGHT))
            .clip_children();
        if !self.top_hairline {
            return row;
        }
        let rule = Separator::horizontal(self.palette);
        let rule = match self.width {
            Some(width) => rule.length(width),
            None => rule,
        };
        Element::column(vec![rule.render(), row])
            .gap(0.0)
            .background(self.palette.surface)
            .size(self.width, Some(Self::HEIGHT + 1.0))
            .clip_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, DropdownMenuItem, Frame, HitKind, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn tree_of(bar: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![Element::spacer().flex(1.0), bar]));
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        tree
    }

    fn button(frame: &Frame, id: &str) -> Option<Rect> {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .map(|region| region.rect)
    }

    // Three leading cells of 60 and five trailing of 50, priorities 5 to 1
    // against the order they are shown in.
    fn bar(width: f32) -> StatusBar {
        let cell = |id: String| StatusBarItem::new(id, 50.0, palette()).label("x");
        StatusBar::new(
            (0..3).map(|index| StatusBarItem::new(format!("lead{index}"), 60.0, palette()).label("x")).collect(),
            (1..=5).map(|priority| cell(format!("p{priority}")).priority(priority)).collect(),
            palette(),
        )
        .width(width)
        .overflow(StatusBarItem::new("more", 20.0, palette()).icon(Icon::More))
    }

    #[test]
    fn trailing_items_leave_lowest_priority_first_and_nothing_overlaps() {
        let mut last = usize::MAX;
        for width in (200..=1200).step_by(20) {
            let width = width as f32;
            let size = Size::new(width, 100.0);
            let frame = tree_of(bar(width).render()).layout(size);
            let shown: Vec<bool> = (1..=5).map(|p| button(&frame, &format!("p{p}")).is_some()).collect();
            let count = shown.iter().filter(|shown| **shown).count();
            // The kept ones are always the highest priorities.
            assert!(shown.iter().skip_while(|shown| !**shown).all(|shown| *shown), "{width}: {shown:?}");
            assert!(last == usize::MAX || count >= last, "{width}: a wider bar shows no fewer");
            last = count;
            // Below 216 not even the overflow cell has room.
            assert_eq!(button(&frame, "more").is_some(), count < 5 && width >= 216.0, "{width}");
            let mut cells: Vec<Rect> = frame
                .regions
                .iter()
                .filter(|region| region.kind == HitKind::Button)
                .map(|region| region.rect)
                .collect();
            cells.sort_by(|a, b| a.x.total_cmp(&b.x));
            let mut end = 0.0;
            for cell in cells {
                if width >= 240.0 {
                    assert!(cell.x >= end && cell.x + cell.width <= width, "{width}: {cell:?} after {end}");
                }
                assert_eq!((cell.y, cell.height), (100.0 - StatusBar::HEIGHT, StatusBar::HEIGHT));
                end = cell.x + cell.width;
            }
        }
        assert_eq!(last, 5, "everything fits at 1200");
        // 196 for the leading cells and the spacer: 54 each, 24 for More.
        let frame = tree_of(bar(196.0 + 24.0 + 54.0 * 2.0).render()).layout(Size::new(328.0, 100.0));
        assert!(button(&frame, "p5").is_some() && button(&frame, "p4").is_some());
        assert!(button(&frame, "p3").is_none());
        assert_eq!(button(&frame, "more").unwrap().x, 328.0 - 20.0);
        // Shown in the order given, not by priority.
        assert!(button(&frame, "p4").unwrap().x < button(&frame, "p5").unwrap().x);
    }

    #[test]
    fn a_budget_from_the_host_replaces_the_bars_own_and_bad_numbers_are_ignored() {
        let frame = tree_of(bar(1200.0).trailing_budget(54.0 + 24.0).render()).layout(Size::new(1200.0, 100.0));
        assert!(button(&frame, "p5").is_some() && button(&frame, "p4").is_none());
        assert!(button(&frame, "more").is_some());
        // Too narrow even for the overflow cell.
        let frame = tree_of(bar(1200.0).trailing_budget(10.0).render()).layout(Size::new(1200.0, 100.0));
        assert!(button(&frame, "p5").is_none() && button(&frame, "more").is_none());
        let frame = tree_of(bar(1200.0).trailing_budget(f32::NAN).width(f32::INFINITY).render())
            .layout(Size::new(1200.0, 100.0));
        assert!(button(&frame, "p1").is_some() && button(&frame, "more").is_none());
        let item = StatusBarItem::new("bad", f32::NAN, palette())
            .label("x")
            .sizing(StatusBarItemSizing { icon_width: f32::NAN, gap: -2.0 })
            .render();
        assert_eq!(item.style.width, Some(0.0));
        // Without a width nothing is fitted.
        let unfitted = StatusBar::new(Vec::new(), vec![StatusBarItem::new("only", 9000.0, palette())], palette());
        assert_eq!(unfitted.fit(), (vec![true], false));
    }

    #[test]
    fn a_divider_is_a_passive_hairline_between_two_cells() {
        let cell = |id: &str| StatusBarItem::new(id, 60.0, palette()).label("x");
        let bar = StatusBar::new(
            vec![cell("a"), StatusBarItem::divider("rule", palette()), cell("b")],
            Vec::new(),
            palette(),
        )
        .width(300.0);
        let frame = tree_of(bar.render()).layout(Size::new(300.0, 100.0));
        let top = 100.0 - StatusBar::HEIGHT;
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == Rect::new(64.0, top + 5.0, 1.0, DIVIDER) && *color == palette().border)));
        assert!(frame.regions.iter().all(|region| region.id != "rule"));
        // It takes its pixel and a gap, like any cell.
        assert_eq!(button(&frame, "b").map(|rect| rect.x), Some(69.0));
    }

    #[test]
    fn a_cell_fills_the_bar_hovers_square_and_reports_its_action() {
        let item = StatusBarItem::new("branch", 80.0, palette())
            .icon(Icon::GitBranch)
            .label("main")
            .dispatch_as("branch_open");
        let mut tree = tree_of(StatusBar::new(vec![item], Vec::new(), palette()).width(300.0).render());
        let size = Size::new(300.0, 100.0);
        let frame = tree.layout(size);
        let rect = button(&frame, "branch").unwrap();
        assert_eq!(rect, Rect::new(0.0, 100.0 - StatusBar::HEIGHT, 80.0, StatusBar::HEIGHT));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::GitBranch, tint, .. } if *tint == palette().muted)));
        let center = Point::new(40.0, rect.y + 10.0);
        tree.handle_event(UiEvent::PointerMove(center));
        let hovered = tree.layout(size);
        assert!(hovered.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect: fill, color, radius }
                if *fill == rect && *color == palette().hover && *radius == 0.0)));
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: "branch_open".into() }]);
    }

    #[test]
    fn counts_follow_their_glyphs_and_ink_colors_only_the_glyphs() {
        let ink = Color::rgb(250, 250, 250);
        let item = StatusBarItem::new("problems", 80.0, palette())
            .icon(Icon::ErrorCircle)
            .count(3)
            .icon(Icon::WarningTriangle)
            .count(12)
            .ink(ink)
            .sizing(StatusBarItemSizing::COUNTS);
        let frame = tree_of(item.render()).layout(Size::new(80.0, 100.0));
        let texts: Vec<(f32, &str, Color)> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, color, .. } => Some((position.x, text.as_str(), *color)),
                _ => None,
            })
            .collect();
        // Padding 2, glyph 12, gap 3; "3" is 6 wide.
        assert_eq!(texts, vec![(17.0, "3", palette().muted), (41.0, "12", palette().muted)]);
        assert!(frame.commands.iter().all(|command| !matches!(command,
            DrawCommand::Icon { tint, .. } if *tint != ink)));

        // A glyph with a color of its own keeps it; the label can stand out.
        let item = StatusBarItem::new("state", 120.0, palette())
            .tinted_icon(Icon::CheckCircle, Color::rgb(1, 2, 3))
            .count(0)
            .icon(Icon::GitBranch)
            .label("main")
            .strong()
            .ink(ink);
        let frame = tree_of(item.render()).layout(Size::new(120.0, 100.0));
        let tints: Vec<Color> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { tint, .. } => Some(*tint),
                _ => None,
            })
            .collect();
        assert_eq!(tints, vec![Color::rgb(1, 2, 3), ink]);
        let inks: Vec<(&str, Color)> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text.as_str(), *color)),
                _ => None,
            })
            .collect();
        assert_eq!(inks, vec![("0", palette().muted), ("main", palette().text)]);

        let alone = |centered: bool| {
            let item = StatusBarItem::new("bell", 28.0, palette())
                .icon(Icon::More)
                .sizing(StatusBarItemSizing { icon_width: 15.0, ..StatusBarItemSizing::COUNTS });
            let item = if centered { item.centered() } else { item };
            let frame = tree_of(item.render()).layout(Size::new(28.0, 100.0));
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!((alone(false).x, alone(false).width), (2.0, 15.0));
        assert_eq!(alone(true).x, 2.0 + (24.0 - 15.0) / 2.0);
    }

    #[test]
    fn a_passive_cell_has_no_target_and_keeps_its_hint() {
        let item = StatusBarItem::new("encoding", 60.0, palette())
            .label("UTF-8")
            .passive()
            .tooltip_with(TooltipSpec::new("File encoding"));
        let mut tree = tree_of(Element::row(vec![item.render(), Element::spacer().flex(1.0)]));
        let size = Size::new(300.0, 100.0);
        let frame = tree.layout(size);
        assert!(button(&frame, "encoding").is_none());
        tree.handle_event(UiEvent::PointerMove(Point::new(30.0, 90.0)));
        assert!(!tree.layout(size).commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == palette().hover)));
        let start = web_time::Instant::now();
        tree.layout_at(size, start);
        let hinted = tree.layout_at(size, start + std::time::Duration::from_secs(2));
        assert!(hinted.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "File encoding")));
    }

    #[test]
    fn a_menu_opens_upward_inside_the_window_and_a_hosted_cell_stands_as_given() {
        let size = Size::new(300.0, 400.0);
        let item = |open: bool| {
            StatusBarItem::new("ending", 40.0, palette()).label("LF").menu(
                vec![DropdownMenuItem::new("lf", "LF").into(), DropdownMenuItem::new("crlf", "CRLF").into()],
                open,
                "ending_dismiss",
                palette(),
            )
        };
        let closed = tree_of(StatusBar::new(Vec::new(), vec![item(false)], palette()).width(300.0).render())
            .layout(size);
        assert!(button(&closed, "crlf").is_none());
        let mut tree = tree_of(StatusBar::new(Vec::new(), vec![item(true)], palette()).width(300.0).render());
        let frame = tree.layout(size);
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:ending_dismiss")
            .unwrap()
            .rect;
        assert_eq!(surface.width, MENU_WIDTH);
        assert!(surface.x >= 0.0 && surface.x + surface.width <= 300.0);
        assert!(surface.y + surface.height <= 400.0 - StatusBar::HEIGHT);
        let target = button(&frame, "crlf").unwrap();
        let center = Point::new(target.x + 10.0, target.y + target.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert!(tree.handle_event(UiEvent::PointerUp(center)).contains(&UiAction::Clicked { id: "crlf".into() }));

        let hosted = StatusBarItem::new("hosted", 30.0, palette())
            .hosted(Element::button("mine", "x").size(Some(30.0), Some(StatusBar::HEIGHT)));
        let frame = tree_of(StatusBar::new(vec![hosted], Vec::new(), palette()).width(300.0).render()).layout(size);
        assert_eq!(button(&frame, "mine").unwrap().width, 30.0);
        assert!(button(&frame, "hosted").is_none());
    }

    #[test]
    fn the_hairline_adds_one_pixel_above_the_cells_and_a_surface() {
        let item = StatusBarItem::new("cell", 40.0, palette()).label("x");
        let bar = StatusBar::new(vec![item], Vec::new(), palette()).width(200.0).top_hairline(true);
        let frame = tree_of(bar.render()).layout(Size::new(200.0, 100.0));
        let top = 100.0 - StatusBar::HEIGHT - 1.0;
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == Rect::new(0.0, top, 200.0, 1.0) && *color == palette().border)));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == Rect::new(0.0, top, 200.0, StatusBar::HEIGHT + 1.0) && *color == palette().surface)));
        assert_eq!(button(&frame, "cell").unwrap().y, top + 1.0);
    }
}
