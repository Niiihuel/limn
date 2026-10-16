//! A bar of commands in groups, which decides their height, their spacing and the rules between them.

use unicode_width::UnicodeWidthStr;

use crate::{
    page::px, Access, Button, ButtonContent, ButtonVariant, Component, ComponentPalette, ControlSize, DropdownMenu,
    DropdownMenuEntry, DropdownMenuItem, Element, Icon, PopoverPlacement, Role, Separator, Toggle, TooltipSpec,
};

// Between two controls, and between a control and a rule's margin.
const GAP: f32 = 2.0;
// Empty space on each side of a rule, across it.
const RULE_MARGIN: f32 = 4.0;
// How far a rule stops short of each end of the controls beside it.
const RULE_INSET: f32 = 6.0;
// Around the controls of a bar that draws its own ground or outline, the outline included.
const INSET: f32 = 4.0;
// The width a letter is taken for, as a share of the font size, when the bar decides what fits.
const LETTER: f32 = 0.6;
const MENU_WIDTH: f32 = 220.0;

/// One control of a [`Toolbar`]: what it shows and the ID it reports. The bar
/// renders it, so it comes out at the bar's size, without a fill at rest, and
/// disabled when the bar is.
#[derive(Debug)]
pub struct ToolbarItem {
    id: String,
    label: String,
    icon: Option<Icon>,
    // `Some` for a toggle, with the host's value.
    pressed: Option<bool>,
    enabled: bool,
    width: Option<f32>,
    tooltip: Option<String>,
    accessibility_label: Option<String>,
    priority: i32,
    // An element the host built, which the bar only places.
    custom: Option<Element>,
}

impl ToolbarItem {
    fn described(id: String, label: String, icon: Option<Icon>, pressed: Option<bool>) -> Self {
        Self {
            id,
            label,
            icon,
            pressed,
            enabled: true,
            width: None,
            tooltip: None,
            accessibility_label: None,
            priority: 0,
            custom: None,
        }
    }

    /// A button with this label that reports `UiAction::Clicked` with `id`.
    pub fn button(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::described(id.into(), label.into(), None, None)
    }

    /// A button of an icon alone, as wide as the bar's controls are high.
    /// `title` is its hint and what the platform calls it.
    pub fn icon_button(id: impl Into<String>, icon: Icon, title: impl Into<String>) -> Self {
        Self::described(id.into(), String::new(), Some(icon), None).tooltip(title)
    }

    /// A [`Toggle`] with this label. `pressed` is the host's value, which it
    /// flips when the toggle reports `UiAction::Clicked` with `id`.
    pub fn toggle(id: impl Into<String>, label: impl Into<String>, pressed: bool) -> Self {
        Self::described(id.into(), label.into(), None, Some(pressed))
    }

    /// A [`Toggle`] of an icon alone. `title` is its hint and what the
    /// platform calls it.
    pub fn icon_toggle(id: impl Into<String>, icon: Icon, title: impl Into<String>, pressed: bool) -> Self {
        Self::described(id.into(), String::new(), Some(icon), Some(pressed)).tooltip(title)
    }

    /// An element the host built, such as a field or a select, centered in
    /// the bar and `width` logical pixels wide for the bar's count of what
    /// fits. The bar cannot change it: it keeps its own size, it is not
    /// disabled with the bar and it never moves to the overflow menu. The
    /// other builders do nothing to it. A width that is not finite is none.
    pub fn element(element: Element, width: f32) -> Self {
        let mut item = Self::described(String::new(), String::new(), None, None);
        item.width = Some(px(width));
        item.custom = Some(element);
        item
    }

    /// Put an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Give the control a hint with this title. It names a control that has
    /// no other name and describes one that has. A disabled control keeps it.
    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.tooltip = Some(title.into());
        self
    }

    /// What the control is called to the platform, in place of its label,
    /// and in the overflow menu when it has no label.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Fix the width in logical pixels, in place of the content's. The bar
    /// then counts it exactly, where it estimates the width of a label. A
    /// negative width becomes zero; one that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && self.custom.is_none() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Which controls stay when the bar is too narrow: the higher stays
    /// longer, and among equals the one given first. It matters only to a
    /// [`Toolbar`] with a width and an overflow menu.
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Draw the control muted, as passive content: it keeps its size and its
    /// hint, and cannot be focused or activated.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable it from a state the host computes; `enabled(false)`
    /// is [`Self::disabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // What a row of the overflow menu calls the control.
    fn name(&self) -> &str {
        if !self.label.is_empty() {
            return &self.label;
        }
        self.accessibility_label.as_deref().or(self.tooltip.as_deref()).unwrap_or("")
    }

    // The width the bar counts for the control. Exact for an icon alone and
    // for a fixed width; a label is estimated, since text is measured only
    // when the tree is laid out.
    fn extent(&self, size: ControlSize) -> f32 {
        if let Some(width) = self.width {
            return width;
        }
        let sizing = size.button_sizing();
        if self.label.is_empty() {
            return sizing.height;
        }
        let icon = if self.icon.is_some() { sizing.icon_size + sizing.inner_spacing } else { 0.0 };
        sizing.horizontal_padding * 2.0 + icon + self.label.width() as f32 * sizing.font_size * LETTER
    }

    fn render(self, size: ControlSize, enabled: bool, palette: ComponentPalette) -> Element {
        if let Some(element) = self.custom {
            return element;
        }
        let enabled = enabled && self.enabled;
        let Some(pressed) = self.pressed else {
            let content = match self.icon {
                Some(icon) if self.label.is_empty() => ButtonContent::Icon(icon),
                Some(icon) => ButtonContent::IconAndLabel(icon, self.label),
                None => ButtonContent::Label(self.label),
            };
            let mut button =
                Button::with_content(self.id, content, palette).variant(ButtonVariant::Ghost).size(size).enabled(enabled);
            if let Some(width) = self.width {
                button = button.width(width);
            }
            if let Some(title) = self.tooltip {
                button = button.tooltip(title);
            }
            if let Some(name) = self.accessibility_label {
                button = button.accessibility_label(name);
            }
            return button.render();
        };
        let mut toggle = match self.icon {
            Some(icon) if self.label.is_empty() => Toggle::icon_only(self.id, icon, pressed, palette),
            Some(icon) => Toggle::new(self.id, self.label, pressed, palette).icon(icon),
            None => Toggle::new(self.id, self.label, pressed, palette),
        }
        .size(size)
        .enabled(enabled);
        if let Some(width) = self.width {
            toggle = toggle.width(width);
        }
        if let Some(title) = self.tooltip {
            toggle = toggle.tooltip(title);
        }
        if let Some(name) = self.accessibility_label {
            toggle = toggle.accessibility_label(name);
        }
        toggle.render()
    }
}

// What the bar lays out, in order. A group is a run of `items`.
enum Entry {
    Group(std::ops::Range<usize>),
    Spacer,
}

// One child of the bar once it is known which controls stay.
enum Part {
    Item(usize),
    Rule,
    Spacer,
}

/// A bar of commands in groups: it decides the height, the space between
/// controls, the rule between two groups and the size of every control it is
/// given as a [`ToolbarItem`]. Each control reports its own ID; the bar
/// reports only the toggle and the dismissal of its overflow menu, if it has
/// one, and keeps nothing.
pub struct Toolbar {
    id: String,
    items: Vec<ToolbarItem>,
    entries: Vec<Entry>,
    size: ControlSize,
    surface: bool,
    bordered: bool,
    vertical: bool,
    width: Option<f32>,
    enabled: bool,
    accessibility_label: Option<String>,
    // Whether the menu is open, and the title of its trigger.
    overflow: Option<(bool, String)>,
    palette: ComponentPalette,
}

impl Toolbar {
    /// An empty horizontal bar at `ControlSize::Regular`, with no ground and
    /// no outline of its own. `id` is what the platform knows the bar by once
    /// it has an `accessibility_label`, and the prefix of its overflow menu's
    /// IDs.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            entries: Vec::new(),
            size: ControlSize::Regular,
            surface: false,
            bordered: false,
            vertical: false,
            width: None,
            enabled: true,
            accessibility_label: None,
            overflow: None,
            palette,
        }
    }

    /// Add a group of controls after what the bar already holds. A rule is
    /// drawn between two groups that follow each other; an empty group adds
    /// nothing.
    pub fn group(mut self, items: Vec<ToolbarItem>) -> Self {
        let start = self.items.len();
        self.items.extend(items);
        self.entries.push(Entry::Group(start..self.items.len()));
        self
    }

    /// Push the groups added after this to the far end of the bar. No rule
    /// is drawn across it. It needs room to take: a bar with a `width`, or
    /// one its container stretches.
    pub fn spacer(mut self) -> Self {
        self.entries.push(Entry::Spacer);
        self
    }

    /// The height of the bar's controls, from a shared control height. The
    /// bar is that high, and 8 more when it draws a surface or a border.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Paint the palette's surface behind the bar, 4 around its controls, for
    /// a bar that floats or stands alone. Without it the bar shows what it
    /// sits on.
    pub fn surface(mut self, surface: bool) -> Self {
        self.surface = surface;
        self
    }

    /// Outline the bar in the palette's border, 4 around its controls.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Lay the bar down a side: controls in a column, horizontal rules
    /// between groups. A vertical bar has no overflow menu.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Fix the bar's width in logical pixels; it then cuts what falls
    /// outside, or moves it to the menu given with [`Self::overflow`]. A
    /// negative width becomes zero; one that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Move the controls a horizontal bar with a `width` has no room for
    /// into a menu, behind a button at its far end that is shown only then.
    /// The lowest [`ToolbarItem::priority`] leaves first. The host owns
    /// `open`: the button reports `{id}:overflow_toggle`, a click outside the
    /// open menu or Escape `{id}:overflow_dismiss`, and each row the ID of
    /// its control, after which the host closes the menu. `title` names the
    /// button, such as "More".
    pub fn overflow(mut self, open: bool, title: impl Into<String>) -> Self {
        self.overflow = Some((open, title.into()));
        self
    }

    /// What the bar is called to the platform, such as "Formatting". With
    /// one the bar is told as a group that holds its controls; without, the
    /// platform is told of the controls alone.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Disable every control the bar renders, whatever each one says. An
    /// item made with [`ToolbarItem::element`] stays as the host built it.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable the whole bar from a state the host computes;
    /// `enabled(true)` leaves each control as it says it is.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn inset(&self) -> f32 {
        if self.surface || self.bordered {
            INSET
        } else {
            0.0
        }
    }

    // The bar's children for these shown controls: a rule only between two
    // groups that both still show something and have no spacer between them.
    fn parts(&self, shown: &[bool]) -> Vec<Part> {
        let mut parts = Vec::with_capacity(self.items.len() + self.entries.len());
        let mut after_group = false;
        for entry in &self.entries {
            match entry {
                Entry::Spacer => {
                    parts.push(Part::Spacer);
                    after_group = false;
                }
                Entry::Group(range) => {
                    let mut first = true;
                    for index in range.clone().filter(|index| shown[*index]) {
                        if first && after_group {
                            parts.push(Part::Rule);
                        }
                        first = false;
                        parts.push(Part::Item(index));
                    }
                    after_group |= !first;
                }
            }
        }
        parts
    }

    // The width a horizontal bar needs for these controls.
    fn extent(&self, shown: &[bool]) -> f32 {
        let parts = self.parts(shown);
        let content: f32 = parts
            .iter()
            .map(|part| match part {
                Part::Item(index) => self.items[*index].extent(self.size),
                Part::Rule => 1.0 + RULE_MARGIN * 2.0,
                Part::Spacer => 0.0,
            })
            .sum();
        content + GAP * parts.len().saturating_sub(1) as f32 + self.inset() * 2.0
    }

    // Which controls stay in the bar. All of them, unless the bar has a
    // width they do not fit and a menu to move them to.
    fn fit(&self) -> Vec<bool> {
        let mut shown = vec![true; self.items.len()];
        let (Some(width), Some(_), false) = (self.width, &self.overflow, self.vertical) else {
            return shown;
        };
        if self.extent(&shown) <= width {
            return shown;
        }
        // The trigger stands at the far end, behind a spacer of its own if the bar has none.
        let pushed = self.entries.iter().any(|entry| matches!(entry, Entry::Spacer));
        let trigger = self.size.height() + GAP + if pushed { 0.0 } else { GAP };
        // An element of the host's cannot be a row of a menu, so it stays.
        let mut leaving: Vec<usize> = (0..self.items.len()).filter(|index| self.items[*index].custom.is_none()).collect();
        // Stable: among equals the one given last leaves first.
        leaving.sort_by_key(|index| std::cmp::Reverse(self.items[*index].priority));
        while let Some(index) = leaving.pop() {
            shown[index] = false;
            if self.extent(&shown) + trigger <= width {
                break;
            }
        }
        shown
    }

    // The rows of the overflow menu: the controls left out, in the order
    // given, with a line between those of two groups.
    fn menu(&self, shown: &[bool]) -> Vec<DropdownMenuEntry> {
        let mut rows = Vec::new();
        for entry in &self.entries {
            let Entry::Group(range) = entry else { continue };
            let before = rows.len();
            for item in range.clone().filter(|index| !shown[*index]).map(|index| &self.items[index]) {
                let mut row = DropdownMenuItem::new(item.id.as_str(), item.name())
                    .selected(item.pressed == Some(true))
                    .enabled(self.enabled && item.enabled);
                if let Some(icon) = item.icon {
                    row = row.icon(icon);
                }
                rows.push(DropdownMenuEntry::Item(row));
            }
            // The menu drops the one that ends it.
            if rows.len() > before {
                rows.push(DropdownMenuEntry::Separator);
            }
        }
        rows
    }
}

impl Component for Toolbar {
    fn render(self) -> Element {
        let shown = self.fit();
        let parts = self.parts(&shown);
        let rows = self.menu(&shown);
        let (height, inset, palette) = (self.size.height(), self.inset(), self.palette);
        let mut items: Vec<Option<ToolbarItem>> = self.items.into_iter().map(Some).collect();
        let mut children: Vec<Element> = parts
            .into_iter()
            .filter_map(|part| match part {
                Part::Spacer => Some(Element::spacer().flex(1.0)),
                Part::Rule if self.vertical => Some(Separator::horizontal(palette).margin(RULE_MARGIN).render()),
                Part::Rule => Some(
                    Separator::vertical(palette)
                        .length(height - RULE_INSET * 2.0)
                        .centered_in(height)
                        .margin(RULE_MARGIN)
                        .render(),
                ),
                Part::Item(index) => {
                    let item = items[index].take()?;
                    let custom = item.custom.is_some();
                    let element = item.render(self.size, self.enabled, palette);
                    // The host's element keeps its own height; the bar centers it.
                    Some(if custom && !self.vertical { element.centered_y() } else { element })
                }
            })
            .collect();
        if let Some((open, title)) = self.overflow.filter(|_| !rows.is_empty()) {
            if !self.entries.iter().any(|entry| matches!(entry, Entry::Spacer)) {
                children.push(Element::spacer().flex(1.0));
            }
            let trigger = Button::with_content(format!("{}:overflow_toggle", self.id), ButtonContent::Icon(Icon::More), palette)
                .variant(ButtonVariant::Ghost)
                .size(self.size)
                .active(open)
                .enabled(self.enabled)
                .accessibility_label(title.as_str())
                .tooltip_with(TooltipSpec::new(title.as_str()).suppressed(open))
                .render()
                .accessible(Access::default().expanded(open));
            children.push(
                DropdownMenu::new(trigger, rows, open, format!("{}:overflow_dismiss", self.id), palette)
                    .placement(PopoverPlacement::BottomEnd)
                    .width(MENU_WIDTH)
                    .render(),
            );
        }
        let mut bar = if self.vertical {
            Element::column(children).size(self.width, None)
        } else {
            Element::row(children).size(self.width, Some(height + inset * 2.0))
        }
        .gap(GAP)
        .padding(inset);
        if self.width.is_some() {
            bar = bar.clip_children();
        }
        if self.surface {
            bar = bar.background(palette.surface);
        }
        if self.bordered {
            bar = bar.border(palette.border, 1.0);
        }
        if self.surface || self.bordered {
            bar = bar.radius(crate::tokens().radius.card);
        }
        match self.accessibility_label {
            Some(name) => bar.accessible(Access::new(Role::Group).id(self.id).name(name).disabled(!self.enabled)),
            None => bar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(600.0, 300.0)
    }

    fn tree_of(element: Element) -> UiTree {
        // The root fills the window; a row and a column keep the bar at its own size.
        let mut tree = UiTree::new(Element::row(vec![
            Element::column(vec![element, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn node<'a>(frame: &'a Frame, id: &str) -> &'a AccessNode {
        frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"))
    }

    // The regions that take a click, in the order they are laid out; a hint's hover region is not one.
    fn buttons(frame: &Frame) -> Vec<&str> {
        let clickable = frame.regions.iter().filter(|region| region.kind == HitKind::Button);
        clickable.map(|region| region.id.as_str()).collect()
    }

    fn button(frame: &Frame, id: &str) -> Option<Rect> {
        let mut clickable = frame.regions.iter().filter(|region| region.kind == HitKind::Button);
        clickable.find(|region| region.id == id).map(|region| region.rect)
    }

    fn painted(frame: &Frame, rect: Rect, ink: crate::Color) -> bool {
        frame.commands.iter().any(|command| match command {
            DrawCommand::FillRect { rect: fill, color, .. } => *fill == rect && *color == ink,
            DrawCommand::StrokeRect { rect: stroke, color, .. } => *stroke == rect && *color == ink,
            _ => false,
        })
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false, alt: false }
    }

    fn click(tree: &mut UiTree, rect: Rect) -> Vec<UiAction> {
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point))
    }

    const IDS: [&str; 5] = ["new", "open", "bold", "wrap", "settings"];

    // Two icon buttons, a rule, two toggles of a fixed width, a spacer and an icon button at the far end.
    fn bar() -> Toolbar {
        Toolbar::new("bar", palette())
            .group(vec![
                ToolbarItem::icon_button("new", Icon::Add, "New"),
                ToolbarItem::icon_button("open", Icon::Folder, "Open"),
            ])
            .group(vec![
                ToolbarItem::toggle("bold", "B", true).width(30.0).accessibility_label("Bold"),
                ToolbarItem::button("wrap", "Wrap").width(50.0),
            ])
            .spacer()
            .group(vec![ToolbarItem::icon_toggle("settings", Icon::Settings, "Settings", false)])
    }

    // Six icon buttons in two groups of three: 213 wide in all at the regular size.
    fn six() -> Toolbar {
        let item = |index: usize| ToolbarItem::icon_button(format!("i{index}"), Icon::Add, format!("Item {index}"));
        Toolbar::new("bar", palette()).group((0..3).map(item).collect()).group((3..6).map(item).collect())
    }

    #[test]
    fn groups_stand_in_order_two_apart_with_a_rule_between_and_a_spacer_sends_the_rest_to_the_far_end() {
        let frame = tree_of(bar().width(400.0).accessibility_label("Document").render()).layout(window());
        assert_eq!(buttons(&frame), IDS);
        let rect = |id: &str| button(&frame, id).unwrap();
        assert_eq!(rect("new"), Rect::new(0.0, 0.0, 32.0, 32.0));
        assert_eq!(rect("open").x, 34.0);
        // A gap, the rule's margin, the rule, its margin and a gap.
        assert!(painted(&frame, Rect::new(72.0, RULE_INSET, 1.0, 32.0 - RULE_INSET * 2.0), palette().border));
        assert_eq!((rect("bold").x, rect("bold").width), (79.0, 30.0));
        assert_eq!((rect("wrap").x, rect("wrap").width), (111.0, 50.0));
        assert_eq!(rect("settings"), Rect::new(400.0 - 32.0, 0.0, 32.0, 32.0));
        // One rule only: none is drawn across the spacer.
        let rules = frame.commands.iter().filter(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. } if rect.width == 1.0 && *color == palette().border));
        assert_eq!(rules.count(), 1);
        assert_eq!(node(&frame, "bar").rect, Rect::new(0.0, 0.0, 400.0, 32.0));
    }

    #[test]
    fn every_size_sets_the_height_of_the_bar_and_of_each_control_whatever_the_order_of_the_builders() {
        for size in [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious] {
            let height = size.height();
            let frame = tree_of(bar().accessibility_label("Document").size(size).render()).layout(window());
            for id in IDS {
                assert_eq!(button(&frame, id).unwrap().height, height, "{id}");
            }
            assert_eq!(button(&frame, "open").unwrap(), Rect::new(height + GAP, 0.0, height, height));
            assert_eq!(node(&frame, "bar").rect.height, height);
        }
    }

    #[test]
    fn a_surface_and_a_border_are_painted_four_around_the_controls_and_neither_by_default() {
        let plain = tree_of(bar().width(400.0).render()).layout(window());
        let ground = Rect::new(0.0, 0.0, 400.0, 40.0);
        assert!(!plain.commands.iter().any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
        assert!(!painted(&plain, Rect::new(0.0, 0.0, 400.0, 32.0), palette().surface));
        for (surface, bordered) in [(true, false), (false, true), (true, true)] {
            let frame = tree_of(bar().width(400.0).surface(surface).bordered(bordered).render()).layout(window());
            assert_eq!(button(&frame, "new").unwrap(), Rect::new(INSET, INSET, 32.0, 32.0));
            assert_eq!(button(&frame, "settings").unwrap().x, 400.0 - INSET - 32.0);
            let fill = frame.commands.iter().any(|command| matches!(command,
                DrawCommand::FillRect { rect, color, radius }
                    if *rect == ground && *color == palette().surface && *radius == crate::tokens().radius.card));
            let stroke = frame.commands.iter().any(|command| matches!(command,
                DrawCommand::StrokeRect { rect, color, .. } if *rect == ground && *color == palette().border));
            assert_eq!((fill, stroke), (surface, bordered));
        }
    }

    #[test]
    fn a_vertical_bar_stacks_its_controls_with_a_horizontal_rule_between_groups() {
        let frame = tree_of(six().vertical().bordered(true).render()).layout(window());
        let rects: Vec<Rect> = (0..6).map(|index| button(&frame, &format!("i{index}")).unwrap()).collect();
        assert_eq!(rects[0], Rect::new(INSET, INSET, 32.0, 32.0));
        assert_eq!(rects[2].y, INSET + 2.0 * 34.0);
        // A gap, the margin, the rule, the margin and a gap below the third.
        let rule = rects[2].y + 32.0 + GAP + RULE_MARGIN;
        assert!(painted(&frame, Rect::new(INSET, rule, 32.0, 1.0), palette().border));
        assert_eq!(rects[3], Rect::new(INSET, rule + 1.0 + RULE_MARGIN + GAP, 32.0, 32.0));
        assert!(rects.iter().all(|rect| rect.x == INSET));
        // Given a height, a spacer pushes what follows to the foot.
        let tall = six().spacer().group(vec![ToolbarItem::icon_button("last", Icon::Settings, "Last")]).vertical();
        let frame = tree_of(tall.render().size(None, Some(280.0))).layout(window());
        assert_eq!(button(&frame, "last").unwrap(), Rect::new(0.0, 280.0 - 32.0, 32.0, 32.0));
        // It has no overflow menu, whatever its width.
        let frame = tree_of(six().vertical().width(20.0).overflow(true, "More").render()).layout(window());
        assert!(button(&frame, "bar:overflow_toggle").is_none());
    }

    #[test]
    fn a_click_enter_and_space_each_report_the_control_they_are_on_and_tab_reaches_each_in_order() {
        let mut tree = tree_of(bar().width(400.0).render());
        let frame = tree.layout(window());
        for id in IDS {
            let clicked = vec![UiAction::Clicked { id: id.into() }];
            assert_eq!(click(&mut tree, button(&frame, id).unwrap()), clicked);
            assert!(tree.focus(id));
            assert_eq!(tree.handle_event(key(Key::Enter)), clicked);
            assert_eq!(tree.handle_event(key(Key::Space)), clicked);
        }
        let mut tree = tree_of(bar().width(400.0).render());
        tree.layout(window());
        for id in IDS {
            assert!(tree.handle_event(key(Key::Tab)).is_empty());
            assert_eq!(tree.focused_id(), Some(id));
            // The arrows are not the bar's: they move nothing and report nothing.
            assert!(tree.handle_event(key(Key::ArrowRight)).is_empty());
            assert_eq!(tree.focused_id(), Some(id));
        }
    }

    #[test]
    fn a_disabled_bar_keeps_every_control_in_place_and_reports_nothing_and_a_hosts_element_stays_its_own() {
        let with_field = |enabled: bool| {
            bar()
                .group(vec![ToolbarItem::element(Element::button("mine", "x").size(Some(40.0), Some(20.0)), 40.0)])
                .width(400.0)
                .accessibility_label("Document")
                .enabled(enabled)
        };
        let enabled = tree_of(with_field(true).render()).layout(window());
        // The host's element is centered in the bar at its own height.
        assert_eq!(button(&enabled, "mine").unwrap(), Rect::new(360.0, 6.0, 40.0, 20.0));
        let mut tree = tree_of(with_field(false).render());
        let frame = tree.layout(window());
        assert_eq!(buttons(&frame), ["mine"]);
        assert!(node(&frame, "bar").disabled && !node(&enabled, "bar").disabled);
        for id in IDS {
            let said = node(&frame, id);
            assert_eq!((said.role, said.disabled, said.actionable), (Role::Button, true, false), "{id}");
            assert_eq!(said.rect, button(&enabled, id).unwrap(), "{id}");
            assert!(!tree.focus(id));
            assert!(click(&mut tree, said.rect).is_empty());
        }
        assert!(tree.handle_event(key(Key::Enter)).is_empty() && tree.handle_event(key(Key::Space)).is_empty());
        // One control disabled alone leaves the others.
        let one = Toolbar::new("bar", palette())
            .group(vec![ToolbarItem::button("a", "A"), ToolbarItem::button("b", "B").disabled()])
            .enabled(true);
        let frame = tree_of(one.render()).layout(window());
        assert_eq!(buttons(&frame), ["a"]);
        assert!(node(&frame, "b").disabled);
        let disabled = format!("{:?}", bar().disabled().render());
        assert_eq!(disabled, format!("{:?}", bar().enabled(false).render()));
    }

    #[test]
    fn the_platform_is_told_a_named_group_and_each_control_by_its_name_and_whether_it_is_pressed() {
        let frame = tree_of(bar().accessibility_label("Document").render()).layout(window());
        let said = node(&frame, "bar");
        assert_eq!((said.role, said.name.as_str(), said.actionable, said.disabled), (Role::Group, "Document", false, false));
        for (id, name, toggled) in [
            ("new", "New", None),
            ("open", "Open", None),
            ("bold", "Bold", Some(true)),
            ("wrap", "Wrap", None),
            ("settings", "Settings", Some(false)),
        ] {
            let said = node(&frame, id);
            assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::Button, name, toggled));
            assert!(said.actionable && !said.disabled);
        }
        // Unnamed, a bar is its controls alone.
        let frame = tree_of(bar().render()).layout(window());
        assert!(frame.access.iter().all(|node| node.id != "bar"));
    }

    #[test]
    fn what_does_not_fit_moves_behind_the_menu_lowest_priority_first_and_reports_its_own_id_from_there() {
        let shown = |frame: &Frame| (0..6).filter(|index| button(frame, &format!("i{index}")).is_some()).collect::<Vec<_>>();
        let fitted = |width: f32, open: bool| six().width(width).overflow(open, "More");
        // 213 holds all six, and the trigger is not shown.
        let frame = tree_of(fitted(213.0, true).render()).layout(window());
        assert_eq!(shown(&frame), [0, 1, 2, 3, 4, 5]);
        assert!(button(&frame, "bar:overflow_toggle").is_none());
        // One pixel less: the trigger takes a square at the far end, so the last two leave.
        let frame = tree_of(fitted(212.0, false).render()).layout(window());
        assert_eq!(shown(&frame), [0, 1, 2, 3]);
        assert_eq!(button(&frame, "bar:overflow_toggle").unwrap(), Rect::new(212.0 - 32.0, 0.0, 32.0, 32.0));
        let trigger = node(&frame, "bar:overflow_toggle");
        assert_eq!((trigger.name.as_str(), trigger.expanded), ("More", Some(false)));
        // A whole group gone takes its rule with it.
        let frame = tree_of(fitted(140.0, false).render()).layout(window());
        assert_eq!(shown(&frame), [0, 1, 2]);
        assert_eq!(button(&frame, "bar:overflow_toggle").unwrap().x, 140.0 - 32.0);
        // Behind a spacer of the bar's own it stands after the last control.
        let frame = tree_of(fitted(140.0, false).spacer().render()).layout(window());
        assert_eq!(button(&frame, "bar:overflow_toggle").unwrap().x, 140.0 - 32.0);
        // Nothing reaches past the bar at any width, and a wider bar shows no fewer.
        let mut last = 0;
        for width in (0..=240).step_by(7) {
            let frame = tree_of(fitted(width as f32, false).render()).layout(window());
            let count = shown(&frame).len();
            assert!(count >= last, "{width}");
            last = count;
            for region in frame.regions.iter().filter(|region| region.kind == HitKind::Button) {
                assert!(region.rect.x + region.rect.width <= width as f32, "{width}: {}", region.id);
            }
        }
        assert_eq!(last, 6);

        // Priority decides who leaves, not the order.
        let by_priority = Toolbar::new("bar", palette())
            .group(vec![
                ToolbarItem::icon_button("low", Icon::Add, "Low").priority(-1),
                ToolbarItem::icon_toggle("mid", Icon::Pin, "Mid", true),
                ToolbarItem::icon_button("high", Icon::Settings, "High").priority(1).disabled(),
            ])
            .width(70.0)
            .overflow(true, "More");
        let mut tree = tree_of(by_priority.render());
        let frame = tree.layout(window());
        assert_eq!(button(&frame, "high"), None, "a disabled control is no button");
        assert_eq!(node(&frame, "high").rect, Rect::new(0.0, 0.0, 32.0, 32.0));
        let trigger = button(&frame, "bar:overflow_toggle").unwrap();
        assert_eq!(node(&frame, "bar:overflow_toggle").expanded, Some(true));
        // Open, the menu holds the two that left, under the bar.
        let (low, mid) = (button(&frame, "low").unwrap(), button(&frame, "mid").unwrap());
        assert!(low.y >= 32.0 && mid.y > low.y);
        assert_eq!(click(&mut tree, mid), vec![UiAction::Clicked { id: "mid".into() }]);
        // An open menu holds the pointer: a click outside it, or on the trigger, dismisses.
        let dismissed = vec![UiAction::Clicked { id: "bar:overflow_dismiss".into() }];
        assert_eq!(click(&mut tree, Rect::new(500.0, 250.0, 2.0, 2.0)), dismissed);
        assert_eq!(click(&mut tree, trigger), dismissed);
        assert_eq!(tree.handle_event(key(Key::Escape)), dismissed);
        let mut closed = tree_of(fitted(140.0, false).render());
        let trigger = button(&closed.layout(window()), "bar:overflow_toggle").unwrap();
        let toggled = vec![UiAction::Clicked { id: "bar:overflow_toggle".into() }];
        assert_eq!(click(&mut closed, trigger), toggled);
        assert!(closed.focus("bar:overflow_toggle"));
        assert_eq!(closed.handle_event(key(Key::Enter)), toggled);
        assert_eq!(closed.handle_event(key(Key::Space)), toggled);
    }

    #[test]
    fn without_a_menu_a_narrow_bar_keeps_every_control_and_cuts_what_falls_outside() {
        let mut tree = tree_of(six().width(120.0).render());
        let frame = tree.layout(window());
        assert!(button(&frame, "bar:overflow_toggle").is_none());
        for region in frame.regions.iter().filter(|region| region.kind == HitKind::Button) {
            assert!(region.rect.x + region.rect.width <= 120.0, "{}", region.id);
        }
        // Past the edge there is nothing to click.
        assert!(click(&mut tree, Rect::new(150.0, 10.0, 2.0, 2.0)).is_empty());
        // A host's element is counted at the width it gives and never leaves.
        let field = ToolbarItem::element(Element::button("mine", "x").size(Some(60.0), Some(24.0)), 60.0).priority(-9);
        let frame = tree_of(six().group(vec![field]).width(150.0).overflow(false, "More").render()).layout(window());
        assert!(button(&frame, "mine").is_some() && button(&frame, "i2").is_none());
        assert!(button(&frame, "i0").is_some() && button(&frame, "bar:overflow_toggle").is_some());
    }

    #[test]
    fn a_label_is_counted_by_its_letters_and_a_fixed_width_exactly() {
        let regular = ControlSize::Regular;
        assert_eq!(ToolbarItem::icon_button("a", Icon::Add, "Add").extent(regular), 32.0);
        assert_eq!(ToolbarItem::button("a", "Save").width(50.0).extent(regular), 50.0);
        // Padding 8 each side, four letters of 13 at 0.6, and 22 more for an icon.
        let label = ToolbarItem::button("a", "Save").extent(regular);
        assert_eq!(label, 16.0 + 4.0 * 13.0 * LETTER);
        assert_eq!(ToolbarItem::toggle("a", "Save", true).icon(Icon::Add).extent(regular), label + 22.0);
    }

    #[test]
    fn widths_that_are_not_numbers_are_ignored_and_negative_ones_are_none() {
        let width = |bar: Toolbar| bar.accessibility_label("Bar").render().style.width;
        assert_eq!(width(six().width(120.0)), Some(120.0));
        assert_eq!(width(six().width(-5.0)), Some(0.0));
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(width(six().width(bad)), None);
            assert_eq!(width(six().width(120.0).width(bad)), Some(120.0));
            let item = || ToolbarItem::button("a", "Save");
            assert_eq!(item().width(bad).extent(ControlSize::Regular), item().extent(ControlSize::Regular));
            assert_eq!(item().width(40.0).width(bad).extent(ControlSize::Regular), 40.0);
            assert_eq!(ToolbarItem::element(Element::spacer(), bad).extent(ControlSize::Regular), 0.0);
        }
        assert_eq!(ToolbarItem::button("a", "Save").width(-5.0).extent(ControlSize::Regular), 0.0);
        // A fixed width reaches the control.
        let frame = tree_of(Toolbar::new("bar", palette()).group(vec![ToolbarItem::button("a", "Save").width(90.0)]).render())
            .layout(window());
        assert_eq!(button(&frame, "a").unwrap().width, 90.0);
    }

    #[test]
    fn an_empty_bar_and_empty_groups_lay_out_and_draw_no_rule() {
        let frame = tree_of(Toolbar::new("none", palette()).render()).layout(window());
        assert!(frame.regions.is_empty() && frame.access.is_empty());
        let sparse = Toolbar::new("bar", palette())
            .group(Vec::new())
            .group(vec![ToolbarItem::icon_button("only", Icon::Add, "Only")])
            .group(Vec::new())
            .spacer();
        let frame = tree_of(sparse.render()).layout(window());
        assert_eq!(button(&frame, "only").unwrap(), Rect::new(0.0, 0.0, 32.0, 32.0));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == palette().border)));
    }
}
