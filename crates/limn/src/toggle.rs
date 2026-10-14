//! A button that stays pressed, alone or joined with others in a row.

use crate::{
    Access, Button, ButtonContent, ButtonSizing, ButtonVariant, Component, ComponentPalette, ControlSize,
    CornerRadii, Element, Icon, Role, Separator, Tooltip,
};

// The outline of a group, which its toggles sit inside.
const GROUP_BORDER: f32 = 1.0;

/// How a [`Toggle`] is drawn while it is not pressed. Pressed, both take the
/// palette's selected fill, as an active [`Button`] of the same variant does.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ToggleVariant {
    /// No fill and no outline at rest, for a toolbar.
    #[default]
    Ghost,
    /// The palette's surface inside its border, for a toggle that stands alone.
    Outline,
}

/// One toggle of a [`ToggleGroup`]: what it shows, whether it is pressed and
/// the ID it reports. A [`Toggle`] takes the same options for itself.
#[derive(Clone, Debug)]
pub struct ToggleItem {
    id: String,
    label: String,
    icon: Option<Icon>,
    pressed: bool,
    enabled: bool,
    width: Option<f32>,
    tooltip: Option<String>,
    accessibility_label: Option<String>,
}

impl ToggleItem {
    /// A toggle with this label that reports `UiAction::Clicked` with `id`.
    /// `pressed` is the host's value; the item never flips it.
    pub fn new(id: impl Into<String>, label: impl Into<String>, pressed: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            pressed,
            enabled: true,
            width: None,
            tooltip: None,
            accessibility_label: None,
        }
    }

    /// A toggle of an icon alone, as wide as it is high. Nothing visible names
    /// it: give it an `accessibility_label`, or a `tooltip`, whose title it is
    /// then called by.
    pub fn icon_only(id: impl Into<String>, icon: Icon, pressed: bool) -> Self {
        Self::new(id, "", pressed).icon(icon)
    }

    /// Put an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Fix the width in logical pixels, in place of the content's: a square
    /// for a label of one letter. A negative width becomes zero; one that is
    /// not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Give the toggle a hint with this title. It names a toggle that has no
    /// other name and describes one that has. A disabled toggle keeps it.
    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.tooltip = Some(title.into());
        self
    }

    /// What the toggle is called to the platform, in place of its label.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Draw the toggle muted, as passive content: it keeps its size, its
    /// pressed fill and its hint, and cannot be focused or activated.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable it from a state the host computes; `enabled(false)`
    /// is [`Self::disabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // The toggle as an active or resting button, told to the platform as a
    // button that is pressed or not. `radii` squares the corners a group joins.
    fn render(
        self,
        variant: ButtonVariant,
        sizing: ButtonSizing,
        radii: Option<CornerRadii>,
        palette: ComponentPalette,
    ) -> Element {
        let content = match self.icon {
            Some(icon) if self.label.is_empty() => ButtonContent::Icon(icon),
            Some(icon) => ButtonContent::IconAndLabel(icon, self.label),
            None => ButtonContent::Label(self.label),
        };
        let mut button = Button::with_content(self.id.clone(), content, palette)
            .variant(variant)
            .sizing(sizing)
            .active(self.pressed)
            .enabled(self.enabled);
        if let Some(width) = self.width {
            button = button.width(width);
        }
        if let Some(name) = self.accessibility_label {
            button = button.accessibility_label(name);
        }
        let mut element = button.render().accessible(Access::default().toggled(self.pressed));
        if let Some(radii) = radii {
            element = element.corner_radii(radii);
        }
        // The hint goes on last, so that the corners above reach the button
        // and not the hint around it.
        match self.tooltip.filter(|title| !title.trim().is_empty()) {
            Some(title) => Tooltip::new(element, self.id, title, palette).render(),
            None => element,
        }
    }
}

/// A button that stays pressed while something is on: a filter, a pinned
/// panel, a text style. The host owns `pressed` and flips it when the toggle
/// reports `UiAction::Clicked` with its ID.
pub struct Toggle {
    item: ToggleItem,
    variant: ToggleVariant,
    size: ControlSize,
    palette: ComponentPalette,
}

impl Toggle {
    /// A toggle with this label that reports `id`. It starts as a ghost at
    /// `ControlSize::Regular`.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        pressed: bool,
        palette: ComponentPalette,
    ) -> Self {
        Self::from_item(ToggleItem::new(id, label, pressed), palette)
    }

    /// A toggle of an icon alone, as wide as it is high. Nothing visible names
    /// it: give it an `accessibility_label`, or a `tooltip`, whose title it is
    /// then called by.
    pub fn icon_only(id: impl Into<String>, icon: Icon, pressed: bool, palette: ComponentPalette) -> Self {
        Self::from_item(ToggleItem::icon_only(id, icon, pressed), palette)
    }

    fn from_item(item: ToggleItem, palette: ComponentPalette) -> Self {
        Self { item, variant: ToggleVariant::default(), size: ControlSize::Regular, palette }
    }

    /// Put an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.item = self.item.icon(icon);
        self
    }

    /// Choose how it is drawn at rest; see [`ToggleVariant`].
    pub fn variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Size the toggle from a shared control height.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Fix the width in logical pixels, in place of the content's. A negative
    /// width becomes zero; one that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        self.item = self.item.width(width);
        self
    }

    /// Give the toggle a hint with this title. It names a toggle that has no
    /// other name and describes one that has. A disabled toggle keeps it.
    pub fn tooltip(mut self, title: impl Into<String>) -> Self {
        self.item = self.item.tooltip(title);
        self
    }

    /// What the toggle is called to the platform, in place of its label.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.item = self.item.accessibility_label(name);
        self
    }

    /// Draw the toggle muted, as passive content: it keeps its size, its
    /// pressed fill and its hint, and cannot be focused or activated.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable it from a state the host computes; `enabled(false)`
    /// is [`Self::disabled`].
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.item = self.item.enabled(enabled);
        self
    }
}

impl Component for Toggle {
    fn render(self) -> Element {
        let variant = match self.variant {
            ToggleVariant::Ghost => ButtonVariant::Ghost,
            ToggleVariant::Outline => ButtonVariant::Outline,
        };
        self.item.render(variant, self.size.button_sizing(), None, self.palette)
    }
}

/// Toggles joined in one outlined row, of which any number may be pressed at
/// once: bold, italic and underline. Each item reports its own ID; the group
/// reports nothing and keeps nothing.
pub struct ToggleGroup {
    id: String,
    items: Vec<ToggleItem>,
    size: ControlSize,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl ToggleGroup {
    /// A group of these toggles, in order, at `ControlSize::Regular`. `id` is
    /// what the platform knows the group by once it has an
    /// `accessibility_label`.
    pub fn new(id: impl Into<String>, items: Vec<ToggleItem>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items,
            size: ControlSize::Regular,
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// The height of the group, outline included, from a shared control height.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// What the group is called to the platform, such as "Text style". Without
    /// one the platform is told of the toggles alone.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Disable every toggle of the group, whatever each one says.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Enable or disable the whole group from a state the host computes;
    /// `enabled(true)` leaves each toggle as it says it is.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for ToggleGroup {
    fn render(self) -> Element {
        let height = self.size.height();
        let sizing = ButtonSizing { height: height - GROUP_BORDER * 2.0, ..self.size.button_sizing() };
        let radius = crate::tokens().radius.control;
        // The toggles sit inside the outline, so their outer corners turn one pixel tighter.
        let inner = (radius - GROUP_BORDER).max(0.0);
        let last = self.items.len().saturating_sub(1);
        let mut children = Vec::with_capacity(self.items.len() * 2);
        for (index, item) in self.items.into_iter().enumerate() {
            if index > 0 {
                children.push(Separator::vertical(self.palette).render());
            }
            let (start, end) = (if index == 0 { inner } else { 0.0 }, if index == last { inner } else { 0.0 });
            let radii = CornerRadii { top_left: start, bottom_left: start, top_right: end, bottom_right: end };
            let enabled = self.enabled && item.enabled;
            children.push(item.enabled(enabled).render(ButtonVariant::Ghost, sizing, Some(radii), self.palette));
        }
        let group = Element::panel(Element::row(children))
            .padding(GROUP_BORDER)
            // The ground an outlined button stands on, which a resting toggle lets through.
            .background(self.palette.surface)
            .border(self.palette.border, GROUP_BORDER)
            .radius(radius)
            .size(None, Some(height));
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

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(600.0, 200.0)
    }

    fn tree_of(element: Element) -> UiTree {
        // The root fills the window; a row and a column keep the control at its own size.
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

    // The corners of the palette's selected fill over `rect`, if it is painted: a
    // toggle alone has four alike, one in a group those the group gives it.
    fn pressed_fill(frame: &Frame, rect: Rect) -> Option<CornerRadii> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect: fill, color, radius } if *fill == rect && *color == palette().selected => {
                Some(CornerRadii::same(*radius))
            }
            DrawCommand::FillCornerRect { rect: fill, color, radii } if *fill == rect && *color == palette().selected => {
                Some(*radii)
            }
            _ => None,
        })
    }

    // The regions that take a click; a hint's hover region is not one.
    fn buttons(frame: &Frame) -> Vec<&str> {
        let clickable = frame.regions.iter().filter(|region| region.kind == HitKind::Button);
        clickable.map(|region| region.id.as_str()).collect()
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false }
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn styles() -> ToggleGroup {
        ToggleGroup::new(
            "style",
            vec![
                ToggleItem::new("bold", "B", true).width(30.0).accessibility_label("Bold"),
                ToggleItem::new("italic", "I", false).width(30.0).accessibility_label("Italic"),
                ToggleItem::icon_only("pin", Icon::Pin, true).tooltip("Pin"),
            ],
            palette(),
        )
    }

    #[test]
    fn a_pressed_toggle_takes_the_selected_fill_and_a_resting_one_none_in_both_variants() {
        for variant in [ToggleVariant::Ghost, ToggleVariant::Outline] {
            for pressed in [false, true] {
                let toggle = Toggle::new("wrap", "Wrap", pressed, palette()).variant(variant);
                let frame = tree_of(toggle.render()).layout(window());
                let rect = frame.regions[0].rect;
                let fill = pressed.then(|| CornerRadii::same(crate::tokens().radius.control));
                assert_eq!(pressed_fill(&frame, rect), fill, "{variant:?}");
                // Only the outline has a border, pressed or not.
                let outlined = frame.commands.iter().any(|command| matches!(
                    command,
                    DrawCommand::StrokeRect { color, .. } if *color == palette().border
                ));
                assert_eq!(outlined, variant == ToggleVariant::Outline);
            }
        }
    }

    #[test]
    fn every_size_gives_its_height_and_an_icon_alone_is_a_square() {
        for size in [ControlSize::Dense, ControlSize::Compact, ControlSize::Regular, ControlSize::Spacious] {
            let icon = Toggle::icon_only("pin", Icon::Pin, false, palette()).size(size).tooltip("Pin");
            let rect = tree_of(icon.render()).layout(window()).regions[0].rect;
            assert_eq!((rect.width, rect.height), (size.height(), size.height()));
            let labelled = Toggle::new("wrap", "Wrap", false, palette()).icon(Icon::Filter).size(size);
            let rect = tree_of(labelled.render()).layout(window()).regions[0].rect;
            assert_eq!(rect.height, size.height());
            assert!(rect.width > size.height());
        }
    }

    #[test]
    fn width_fixes_the_toggle_and_ignores_what_is_not_a_number() {
        let width = |toggle: Toggle| tree_of(toggle.render()).layout(window()).regions[0].rect.width;
        let toggle = || Toggle::new("bold", "B", false, palette());
        let own = width(toggle());
        assert_eq!(width(toggle().width(40.0)), 40.0);
        assert_eq!(width(toggle().width(-5.0)), 0.0);
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(width(toggle().width(bad)), own);
            assert_eq!(width(toggle().width(40.0).width(bad)), 40.0);
        }
    }

    #[test]
    fn a_click_enter_and_space_each_report_the_toggle_once_whether_pressed_or_not() {
        for pressed in [false, true] {
            let mut tree = tree_of(Toggle::new("wrap", "Wrap", pressed, palette()).render());
            let point = center(tree.layout(window()).regions[0].rect);
            let clicked = vec![UiAction::Clicked { id: "wrap".into() }];
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked);
            assert!(tree.focus("wrap"));
            assert_eq!(tree.handle_event(key(Key::Enter)), clicked);
            assert_eq!(tree.handle_event(key(Key::Space)), clicked);
        }
    }

    #[test]
    fn a_disabled_toggle_keeps_its_place_and_its_fill_and_reports_nothing() {
        for variant in [ToggleVariant::Ghost, ToggleVariant::Outline] {
            let toggle = |enabled| {
                Toggle::new("wrap", "Wrap", true, palette()).icon(Icon::Filter).variant(variant).enabled(enabled)
            };
            let enabled = tree_of(toggle(true).render()).layout(window());
            let mut tree = tree_of(toggle(false).render());
            let frame = tree.layout(window());
            assert!(frame.regions.is_empty());
            assert!(!tree.focus("wrap"));
            let rect = node(&frame, "wrap").rect;
            assert_eq!(rect, enabled.regions[0].rect);
            assert!(pressed_fill(&frame, rect).is_some());
            let point = center(rect);
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            assert!(tree.handle_event(key(Key::Enter)).is_empty());
            assert!(tree.handle_event(key(Key::Space)).is_empty());
        }
        let disabled = format!("{:?}", Toggle::new("a", "A", false, palette()).disabled().render());
        assert_eq!(disabled, format!("{:?}", Toggle::new("a", "A", false, palette()).enabled(false).render()));
    }

    #[test]
    fn the_platform_is_told_a_button_its_name_and_whether_it_is_pressed() {
        for enabled in [true, false] {
            for pressed in [false, true] {
                let frame = tree_of(Element::row(vec![
                    Toggle::new("wrap", "Wrap", pressed, palette()).enabled(enabled).render(),
                    Toggle::new("bold", "B", pressed, palette())
                        .accessibility_label("Bold")
                        .tooltip("Bold text")
                        .enabled(enabled)
                        .render(),
                    Toggle::icon_only("pin", Icon::Pin, pressed, palette()).tooltip("Pin").enabled(enabled).render(),
                ]))
                .layout(window());
                for (id, name, description) in
                    [("wrap", "Wrap", None), ("bold", "Bold", Some("Bold text")), ("pin", "Pin", None)]
                {
                    let said = node(&frame, id);
                    assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::Button, name, Some(pressed)));
                    assert_eq!(said.description.as_deref(), description, "{id}");
                    assert_eq!((said.disabled, said.actionable), (!enabled, enabled), "{id}");
                }
            }
        }
    }

    #[test]
    fn a_group_joins_its_toggles_inside_one_outline_a_hairline_apart() {
        for size in [ControlSize::Dense, ControlSize::Regular, ControlSize::Spacious] {
            let frame = tree_of(styles().size(size).accessibility_label("Text style").render()).layout(window());
            let rects: Vec<Rect> = ["bold", "italic", "pin"].iter().map(|id| node(&frame, id).rect).collect();
            let inner = size.height() - 2.0;
            assert_eq!((rects[0].x, rects[0].y), (1.0, 1.0));
            for pair in rects.windows(2) {
                assert_eq!(pair[1].x, pair[0].x + pair[0].width + 1.0);
            }
            assert!(rects.iter().all(|rect| rect.height == inner));
            // A fixed width holds and an icon alone is a square.
            assert_eq!((rects[0].width, rects[1].width, rects[2].width), (30.0, 30.0, inner));
            let group = node(&frame, "style");
            assert_eq!((group.role, group.name.as_str(), group.actionable), (Role::Group, "Text style", false));
            let end = rects[2].x + rects[2].width + 1.0;
            assert_eq!(group.rect, Rect::new(0.0, 0.0, end, size.height()));
            // Any number may be pressed: the first and the last are, the middle is not.
            // Only the corners at the ends of the row are round, a pixel inside the outline's.
            let round = crate::tokens().radius.control - 1.0;
            let start = CornerRadii { top_left: round, bottom_left: round, top_right: 0.0, bottom_right: 0.0 };
            let end = CornerRadii { top_left: 0.0, bottom_left: 0.0, top_right: round, bottom_right: round };
            assert_eq!(pressed_fill(&frame, rects[0]), Some(start));
            assert_eq!(pressed_fill(&frame, rects[1]), None);
            assert_eq!(pressed_fill(&frame, rects[2]), Some(end));
        }
        // Unnamed, a group is its toggles alone.
        let frame = tree_of(styles().render()).layout(window());
        assert!(frame.access.iter().all(|node| node.id != "style"));
    }

    #[test]
    fn each_toggle_of_a_group_reports_its_own_id_by_pointer_enter_and_space() {
        let mut tree = tree_of(styles().render());
        let frame = tree.layout(window());
        assert_eq!(buttons(&frame), ["bold", "italic", "pin"]);
        for id in ["bold", "italic", "pin"] {
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
    fn a_group_disables_all_its_toggles_or_one_and_none_of_them_moves() {
        let enabled = tree_of(styles().render()).layout(window());
        let place = |frame: &Frame| ["bold", "italic", "pin"].map(|id| node(frame, id).rect);

        let mut tree = tree_of(styles().disabled().render());
        let frame = tree.layout(window());
        assert!(buttons(&frame).is_empty());
        assert_eq!(place(&frame), place(&enabled));
        for id in ["bold", "italic", "pin"] {
            let said = node(&frame, id);
            assert_eq!((said.role, said.disabled, said.actionable), (Role::Button, true, false));
            assert!(!tree.focus(id));
            let point = center(said.rect);
            tree.handle_event(UiEvent::PointerDown(point));
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        }
        assert_eq!((node(&frame, "bold").toggled, node(&frame, "pin").name.as_str()), (Some(true), "Pin"));

        let one = ToggleGroup::new(
            "style",
            vec![ToggleItem::new("bold", "B", true).width(30.0), ToggleItem::new("italic", "I", false).width(30.0).disabled()],
            palette(),
        );
        let mut tree = tree_of(one.enabled(true).render());
        let frame = tree.layout(window());
        assert_eq!(buttons(&frame), ["bold"]);
        assert!(node(&frame, "italic").disabled && !node(&frame, "bold").disabled);
        assert_eq!(node(&frame, "italic").rect.x, node(&enabled, "italic").rect.x);
    }

    #[test]
    fn an_empty_group_and_a_group_of_one_still_lay_out() {
        let frame = tree_of(ToggleGroup::new("none", Vec::new(), palette()).render()).layout(window());
        assert!(frame.regions.is_empty() && frame.access.is_empty());
        let one = ToggleGroup::new("one", vec![ToggleItem::icon_only("pin", Icon::Pin, true).tooltip("Pin")], palette());
        let frame = tree_of(one.render()).layout(window());
        assert_eq!(node(&frame, "pin").rect, Rect::new(1.0, 1.0, 30.0, 30.0));
        let all = CornerRadii::same(crate::tokens().radius.control - 1.0);
        assert_eq!(pressed_fill(&frame, node(&frame, "pin").rect), Some(all));
    }
}
