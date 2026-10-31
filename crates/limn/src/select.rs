//! A button that shows the current choice and opens the list of the others.

use crate::components::{control_radius, input_surface, lighter};
use crate::{
    Access, Button, ButtonVariant, Color, Component, ComponentPalette, ControlSize,
    DropdownMenu, DropdownOption, Element, FontFamily, Icon, PopoverPlacement,
    Role, SelectorSizing, ICON_SIZE,
};

/// How a [`Select`]'s trigger is drawn.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SelectVariant {
    /// An outlined button, its label in the middle unless it has an icon:
    /// a choice in a toolbar or a header.
    #[default]
    Button,
    /// A text field's fill, border, radius and height, the choice at its
    /// start where a field has its text and the chevron at its end: a choice
    /// in a form, among text fields.
    Field,
}

/// A controlled choice among a few options. The host owns the selection and
/// the open state: the trigger reports `id`, an option reports its own
/// action, and Escape or a click outside reports `dismiss_id`.
pub struct Select {
    id: String,
    current_label: String,
    options: Vec<DropdownOption>,
    open: bool,
    dismiss_id: String,
    palette: ComponentPalette,
    menu_palette: Option<ComponentPalette>,
    sizing: Option<SelectorSizing>,
    placement: PopoverPlacement,
    width: Option<f32>,
    menu_width: Option<f32>,
    icon: Option<Icon>,
    variant: SelectVariant,
    size: Option<ControlSize>,
    height: Option<f32>,
    enabled: bool,
    accessibility_label: Option<String>,
    header: Option<Box<Element>>,
    max_visible_items: Option<usize>,
    font_size: Option<f32>,
    ink: Option<Color>,
    outline: Option<(Color, Color)>,
    radius: Option<f32>,
}

impl Select {
    /// `current_label` names the selected option on the trigger.
    pub fn new(
        id: impl Into<String>,
        current_label: impl Into<String>,
        options: Vec<DropdownOption>,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            current_label: current_label.into(),
            options,
            open,
            dismiss_id: dismiss_id.into(),
            palette,
            menu_palette: None,
            sizing: None,
            placement: PopoverPlacement::BottomEnd,
            width: None,
            menu_width: None,
            icon: None,
            variant: SelectVariant::Button,
            size: None,
            height: None,
            enabled: true,
            accessibility_label: None,
            header: None,
            max_visible_items: None,
            font_size: None,
            ink: None,
            outline: None,
            radius: None,
        }
    }

    /// What the select is called to the platform. Its trigger shows the
    /// current choice, which is its value: without this, or a `FormField`
    /// around it, that choice is also its name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Paint the list with another palette than the trigger's.
    pub fn menu_palette(mut self, palette: ComponentPalette) -> Self {
        self.menu_palette = Some(palette);
        self
    }

    /// Content kept above the open menu's options, such as a search field.
    /// The header shares the menu's focus scope and does not scroll with its
    /// options. The host owns filtering and can set `popover_initial_focus`
    /// on the rendered element to focus a control inside the header.
    pub fn header(mut self, header: Element) -> Self {
        self.header = Some(Box::new(header));
        self
    }

    /// Share the row density of other selectors. The trigger takes the
    /// profile's `min_width` and the list its `preferred_width`, unless
    /// `width` or `menu_width` say otherwise.
    pub fn sizing(mut self, sizing: SelectorSizing) -> Self {
        self.sizing = Some(sizing);
        self
    }

    /// Where the list opens. The default is under the trigger's far edge.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Width of the trigger. A value that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Width of the list, which otherwise follows the profile or the
    /// trigger. A value that is not finite is ignored.
    pub fn menu_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.menu_width = Some(width.max(0.0));
        }
        self
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draw the trigger as a button, the default, or as a text field; see
    /// [`SelectVariant`].
    pub fn variant(mut self, variant: SelectVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Take the height of a control size. Unless it is set the button is
    /// `Compact`, 28 high, and the field `Regular`, as a `TextField` is.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = Some(size);
        self
    }

    /// A height of the host's own, as `TextField::height` takes, for a form
    /// whose fields are taller than a [`ControlSize`]. The trigger is never
    /// shorter than its control size; a value that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height);
        }
        self
    }

    /// How many options the open list shows before it scrolls, at least 1.
    /// Unless set, the list is as long as its options, within the window.
    pub fn max_visible_items(mut self, count: usize) -> Self {
        self.max_visible_items = Some(count.max(1));
        self
    }

    /// The size of the choice's text on the trigger, in logical pixels: 13
    /// in a field unless set, and the button's size otherwise. A size that
    /// is not finite or not above zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = Some(size);
        }
        self
    }

    /// The color of a field's border while its list is open, 2 pixels wide,
    /// such as the application's accent, in place of a focused field's
    /// lighter border. A field's chevron points up while its list is open
    /// either way.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// A field's outline, as [`TextField::outline`](crate::TextField::outline)
    /// takes it: `border` at rest and `focus` while the trigger has the focus
    /// or its list is open, in place of the palette's. `ink` still wins while
    /// the list is open. The button variant has no field and ignores it.
    pub fn outline(mut self, border: Color, focus: Color) -> Self {
        self.outline = Some((border, focus));
        self
    }

    /// A field's corner radius in logical pixels, in place of the control's;
    /// the button variant ignores it. A value that is not finite is ignored
    /// and a negative one is zero.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = Some(radius.max(0.0));
        }
        self
    }

    /// A disabled select shows its value, stays closed and cannot be focused.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Select {
    fn render(self) -> Element {
        let width = self
            .width
            .unwrap_or(self.sizing.unwrap_or(SelectorSizing::COMPACT).min_width);
        let open = self.open && self.enabled;
        let mut access =
            Access::new(Role::ComboBox).expanded(open).value_text(self.current_label.clone());
        if let Some(name) = self.accessibility_label.clone() {
            access = access.name(name);
        }
        let size = self.size.unwrap_or(match self.variant {
            SelectVariant::Button => ControlSize::Compact,
            SelectVariant::Field => ControlSize::Regular,
        });
        let least = match self.variant {
            SelectVariant::Button => size.height(),
            // As a text field: never under 28.
            SelectVariant::Field => size.height().max(28.0),
        };
        let height = self.height.map_or(least, |height| height.max(least));
        let width = match self.variant {
            SelectVariant::Button => width,
            // As a text field: never under 80.
            SelectVariant::Field => width.max(80.0),
        };
        let trigger = match self.variant {
            SelectVariant::Button => {
                let mut sizing = size.button_sizing();
                sizing.height = height;
                if let Some(font_size) = self.font_size {
                    sizing.font_size = font_size;
                }
                let mut trigger = Button::new(self.id, self.current_label, self.palette)
                    .variant(ButtonVariant::Outline)
                    .sizing(sizing)
                    .trailing_icon(Icon::ChevronDown)
                    .width(width)
                    .enabled(self.enabled);
                if let Some(icon) = self.icon {
                    trigger = trigger.icon(icon);
                }
                trigger.render()
            }
            SelectVariant::Field => self.field_trigger(size, width, height),
        };
        // The list is the application's one menu: compact rows and a check
        // on the choice, as in every other selector.
        let entries = self
            .options
            .into_iter()
            .map(|option| option.item().into())
            .collect();
        let menu_width = self
            .menu_width
            .unwrap_or_else(|| self.sizing.map_or(width, |sizing| sizing.preferred_width));
        let mut menu = DropdownMenu::new(
            // Over what the button says of itself, disabled or not.
            trigger.accessible(access),
            entries,
            open,
            self.dismiss_id,
            self.menu_palette.unwrap_or(self.palette),
        )
        .context_menu_style()
        .width(menu_width)
        .placement(self.placement);
        if let Some(count) = self.max_visible_items {
            menu = menu.max_visible_items(count);
        }
        if let Some(header) = self.header {
            menu = menu.header(*header);
        }
        menu.render().size(Some(width), Some(height))
    }
}

impl Select {
    // The trigger drawn as a text field: the choice where a field has its
    // text, after the icon if there is one, and the chevron at the end.
    // Open or focused from the keyboard, it has the border a focused field
    // has.
    fn field_trigger(&self, size: ControlSize, width: f32, height: f32) -> Element {
        let palette = self.palette;
        let label = self.current_label.clone();
        if !self.enabled {
            let mut children = Vec::new();
            if let Some(icon) = self.icon {
                children.push(
                    Element::icon(icon)
                        .foreground(palette.muted)
                        .size(Some(ICON_SIZE), Some(height)),
                );
            }
            children.push(
                Element::truncated_text(label.clone())
                    .foreground(palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(self.font_size.unwrap_or(13.0))
                    .size(None, Some(height))
                    .flex(1.0),
            );
            children.push(
                Element::icon(Icon::ChevronDown)
                    .foreground(palette.muted)
                    .size(Some(ICON_SIZE), Some(height)),
            );
            // Passive content has no name of its own: the choice is said.
            let said = Access::new(Role::ComboBox)
                .id(self.id.clone())
                .disabled(true)
                .name(label);
            return Element::panel(Element::row(vec![
                Element::spacer().size(Some(8.0), None),
                Element::row(children).gap(8.0).flex(1.0),
                Element::spacer().size(Some(8.0), None),
            ]))
            .background(palette.raised)
            .border(self.outline.map_or(palette.border, |(border, _)| border), 1.0)
            .radius(self.radius.unwrap_or_else(control_radius))
            .size(Some(width), Some(height))
            .accessible(said);
        }
        let trigger = match self.icon {
            Some(icon) => Element::icon_button(self.id.clone(), icon, label),
            None => Element::button(self.id.clone(), label),
        };
        let chevron = if self.open { Icon::ChevronUp } else { Icon::ChevronDown };
        let trigger = input_surface(trigger.trailing_icon(chevron), palette, size, Some(width));
        let trigger = match self.font_size {
            Some(font_size) => trigger.font_size(font_size),
            None => trigger,
        }
        .gap(8.0)
        // A focused field's border, not a button's ring, under the keyboard.
        .focus_ring(false)
        .size(Some(width), Some(height));
        let trigger = match self.outline {
            Some((border, focus)) => trigger.border(border, 1.0).focus_border(focus, 1.5),
            None => trigger,
        };
        let trigger = match self.radius {
            Some(radius) => trigger.radius(radius),
            None => trigger,
        };
        let focus = self.outline.map_or(lighter(palette.border, 70), |(_, focus)| focus);
        match (self.open, self.ink) {
            (true, Some(ink)) => trigger.border(ink, 2.0),
            (true, None) => trigger.border(focus, 1.5),
            (false, _) => trigger,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Key, Point, Size, UiAction, UiEvent, UiTree};

    fn select(open: bool) -> Select {
        Select::new(
            "toggle",
            "Bash",
            vec![
                DropdownOption::new("shell:bash", "Bash").selected(true),
                DropdownOption::new("shell:off", "Off").disabled(),
                DropdownOption::new("shell:zsh", "Zsh"),
            ],
            open,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
    }

    fn tree(select: Select) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![select.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    const VIEWPORT: Size = Size::new(600.0, 400.0);

    fn rect(frame: &crate::Frame, id: &str) -> Option<crate::Rect> {
        frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
    }

    #[test]
    fn closed_it_is_one_trigger_that_reports_its_id() {
        let mut tree = tree(select(false));
        let frame = tree.layout(VIEWPORT);
        let trigger = rect(&frame, "toggle").expect("trigger");
        assert_eq!(
            (trigger.width, trigger.height),
            (SelectorSizing::COMPACT.min_width, ControlSize::Compact.height())
        );
        assert!(rect(&frame, "shell:zsh").is_none());
        let center = Point::new(trigger.x + 10.0, trigger.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked { id: "toggle".into() }]
        );
    }

    #[test]
    fn open_it_lists_the_options_under_the_trigger_and_reports_the_one_clicked() {
        let mut tree = tree(select(true).sizing(SelectorSizing::COMPACT).width(180.0));
        let frame = tree.layout(VIEWPORT);
        let trigger = rect(&frame, "toggle").unwrap();
        assert_eq!(trigger.width, 180.0);
        let surface = rect(&frame, "popover_surface:dismiss").expect("list");
        assert_eq!(surface.width, SelectorSizing::COMPACT.preferred_width);
        assert!(surface.y >= trigger.y + trigger.height);
        assert!(rect(&frame, "shell:off").is_none(), "a disabled option is passive");
        let option = rect(&frame, "shell:zsh").unwrap();
        // The standard menu's compact row.
        assert_eq!(option.height, 26.0);
        let center = Point::new(option.x + 10.0, option.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked { id: "shell:zsh".into() }]
        );
    }

    #[test]
    fn an_option_with_a_hint_shows_it_under_the_pointer_and_a_blank_one_is_ignored() {
        let long = "Headset of the meeting room (USB, analog stereo)";
        let hinted = |hint: &str| {
            Select::new(
                "toggle",
                "Headset…",
                vec![
                    DropdownOption::new("mic:a", "Headset…").tooltip(hint).selected(true),
                    DropdownOption::new("mic:b", "Laptop"),
                ],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
        };
        assert_eq!(DropdownOption::new("a", "A").tooltip("  ").tooltip, None);
        let shown = |select: Select| {
            let mut tree = tree(select);
            let option = rect(&tree.layout(VIEWPORT), "mic:a").expect("option");
            tree.handle_event(UiEvent::PointerMove(Point::new(option.x + 10.0, option.y + 10.0)));
            let later = web_time::Instant::now() + std::time::Duration::from_secs(2);
            tree.layout_at(VIEWPORT, later)
                .commands
                .iter()
                .any(|command| matches!(command, crate::DrawCommand::Text { text, .. } if text == long))
        };
        assert!(shown(hinted(long)), "the whole name is in the hint");
        assert!(!shown(hinted("")), "no hint, nothing over the list");
        // The same hint through the plain dropdown list.
        let list = crate::DropdownList::new(
            Element::spacer().size(Some(80.0), Some(24.0)),
            vec![DropdownOption::new("mic:a", "Headset…").tooltip(long)],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        );
        let mut tree = UiTree::new(Element::column(vec![list.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let option = rect(&tree.layout(VIEWPORT), "mic:a").expect("option");
        tree.handle_event(UiEvent::PointerMove(Point::new(option.x + 10.0, option.y + 10.0)));
        let later = web_time::Instant::now() + std::time::Duration::from_secs(2);
        assert!(tree
            .layout_at(VIEWPORT, later)
            .commands
            .iter()
            .any(|command| matches!(command, crate::DrawCommand::Text { text, .. } if text == long)));
    }

    #[test]
    fn the_list_follows_the_trigger_width_unless_told_otherwise() {
        for (select, wanted) in [
            (select(true).width(300.0), 300.0),
            (select(true).width(300.0).menu_width(320.0), 320.0),
            (select(true).sizing(SelectorSizing::CATALOG).menu_width(250.0), 250.0),
            (select(true).width(f32::NAN).menu_width(f32::INFINITY), 200.0),
        ] {
            let frame = tree(select).layout(VIEWPORT);
            assert_eq!(rect(&frame, "popover_surface:dismiss").unwrap().width, wanted);
        }
    }

    #[test]
    fn the_keyboard_moves_through_enabled_options_chooses_and_dismisses() {
        let key = |key| UiEvent::Key { key, shift: false, ctrl: false, alt: false };
        let mut tree = tree(select(true));
        tree.layout(VIEWPORT);
        assert!(tree.focus("toggle"));
        tree.handle_event(key(Key::ArrowDown));
        assert_eq!(tree.focused_id(), Some("shell:bash"));
        tree.handle_event(key(Key::ArrowDown));
        assert_eq!(tree.focused_id(), Some("shell:zsh"));
        assert_eq!(
            tree.handle_event(key(Key::Enter)),
            vec![UiAction::Clicked { id: "shell:zsh".into() }]
        );
        assert_eq!(
            tree.handle_event(key(Key::Escape)),
            vec![UiAction::Clicked { id: "dismiss".into() }]
        );
    }

    #[test]
    fn the_platform_is_told_a_combo_box_its_choice_and_whether_it_is_open() {
        let node = |select: Select| {
            let frame = tree(select).layout(VIEWPORT);
            frame.access.iter().find(|node| node.id == "toggle").cloned().expect("a node")
        };
        for open in [false, true] {
            let said = node(select(open).accessibility_label("Shell"));
            assert_eq!((said.role, said.name.as_str()), (Role::ComboBox, "Shell"));
            assert_eq!((said.expanded, said.value_text.as_deref()), (Some(open), Some("Bash")));
            assert!(said.actionable && !said.disabled);
        }
        // The options are the menu's rows, and the chosen one says so.
        let frame = tree(select(true)).layout(VIEWPORT);
        let option = |id: &str| frame.access.iter().find(|node| node.id == id).map(|node| (node.role, node.selected));
        assert_eq!(option("shell:bash"), Some((Role::MenuItem, Some(true))));
        assert_eq!(option("shell:zsh"), Some((Role::MenuItem, Some(false))));
        // Without a name of its own the choice it shows is all there is.
        assert_eq!(node(select(false)).name, "Bash");
        let disabled = node(select(true).accessibility_label("Shell").enabled(false));
        assert_eq!((disabled.role, disabled.name.as_str()), (Role::ComboBox, "Shell"));
        assert_eq!((disabled.expanded, disabled.value_text.as_deref()), (Some(false), Some("Bash")));
        assert!(disabled.disabled && !disabled.actionable);
    }

    fn field(open: bool) -> Select {
        select(open).variant(SelectVariant::Field).width(240.0)
    }

    // Where the trigger's label and its icons are drawn.
    fn drawn(frame: &crate::Frame) -> (Option<Point>, Vec<(Icon, crate::Rect)>) {
        let label = frame.commands.iter().find_map(|command| match command {
            crate::DrawCommand::Text { text, position, .. } if text == "Bash" => Some(*position),
            _ => None,
        });
        let icons = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                crate::DrawCommand::Icon { icon, rect, .. } => Some((*icon, *rect)),
                _ => None,
            })
            .collect();
        (label, icons)
    }

    #[test]
    fn a_field_select_is_a_text_field_with_its_choice_at_the_start() {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(field(false)).layout(VIEWPORT);
        let trigger = rect(&frame, "toggle").unwrap();
        // A text field's height, unless a size says otherwise.
        assert_eq!(
            (trigger.width, trigger.height),
            (240.0, ControlSize::Regular.height())
        );
        let (label, icons) = drawn(&frame);
        // The choice where a field has its text; the chevron at the end.
        assert_eq!(label.expect("the choice").x, trigger.x + 8.0);
        assert_eq!(icons.len(), 1);
        let (icon, chevron) = icons[0];
        assert_eq!(
            (icon, chevron.x + chevron.width),
            (Icon::ChevronDown, trigger.x + trigger.width - 8.0)
        );
        // A text field's fill and border.
        assert!(frame.commands.iter().any(|command| matches!(command,
            crate::DrawCommand::FillRect { rect, color, .. } if *rect == trigger && *color == palette.raised)));
        assert!(frame.commands.iter().any(|command| matches!(command,
            crate::DrawCommand::StrokeRect { rect, color, width, .. }
                if *rect == trigger && *color == palette.border && *width == 1.0)));
        // With an icon, the choice follows it as a field's text follows its icon.
        let frame = tree(field(false).icon(Icon::Terminal)).layout(VIEWPORT);
        let (label, icons) = drawn(&frame);
        assert_eq!(
            icons[0],
            (
                Icon::Terminal,
                crate::Rect::new(trigger.x + 8.0, icons[0].1.y, ICON_SIZE, ICON_SIZE)
            )
        );
        assert_eq!(label.unwrap().x, trigger.x + 8.0 + ICON_SIZE + 8.0);
    }

    #[test]
    fn open_a_field_select_has_a_focused_fields_border_and_its_list_under_it() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(field(true).placement(PopoverPlacement::BottomStart));
        let frame = tree.layout(VIEWPORT);
        let trigger = rect(&frame, "toggle").unwrap();
        assert!(frame.commands.iter().any(|command| matches!(command,
            crate::DrawCommand::StrokeRect { rect, color, width, .. }
                if *rect == trigger && *color == lighter(palette.border, 70) && *width == 1.5)));
        let list = rect(&frame, "popover_surface:dismiss").expect("list");
        assert_eq!(list.width, 240.0);
        assert!(list.y >= trigger.y + trigger.height);
        let option = rect(&frame, "shell:zsh").unwrap();
        let center = Point::new(option.x + 10.0, option.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked {
                id: "shell:zsh".into()
            }]
        );
    }

    #[test]
    fn a_field_takes_its_text_size_rows_in_view_and_an_open_border_with_the_chevron_up() {
        const ACCENT: Color = Color::rgb(88, 101, 242);
        let many = |open: bool| {
            let options = (0..12).map(|index| DropdownOption::new(format!("o:{index}"), format!("Option {index}"))).collect();
            Select::new("toggle", "Bash", options, open, "dismiss", ComponentPalette::neutral_dark())
                .variant(SelectVariant::Field)
                .width(240.0)
                .font_size(16.0)
                .font_size(f32::NAN)
                .font_size(0.0)
                .ink(ACCENT)
        };
        for open in [false, true] {
            let frame = tree(many(open).max_visible_items(5).max_visible_items(0).max_visible_items(5)).layout(VIEWPORT);
            let trigger = rect(&frame, "toggle").unwrap();
            assert!(frame.commands.iter().any(|command| matches!(command,
                crate::DrawCommand::Text { text, font_size, .. } if text == "Bash" && *font_size == 16.0)));
            let (_, icons) = drawn(&frame);
            let chevron = icons.iter().find(|(_, rect)| rect.x > trigger.x + 200.0).unwrap().0;
            assert_eq!(chevron, if open { Icon::ChevronUp } else { Icon::ChevronDown });
            let accent = frame.commands.iter().any(|command| matches!(command,
                crate::DrawCommand::StrokeRect { rect, color, width, .. }
                    if *rect == trigger && *color == ACCENT && *width == 2.0));
            assert_eq!(accent, open);
            if open {
                // Five rows show; the rest are reached by scrolling.
                let shown = (0..12).filter(|index| rect(&frame, &format!("o:{index}")).is_some()).count();
                assert_eq!(shown, 5, "{:?}", frame.regions);
            }
        }
        // Without a limit every option shows, as before.
        let frame = tree(many(true)).layout(Size::new(600.0, 800.0));
        assert_eq!((0..12).filter(|index| rect(&frame, &format!("o:{index}")).is_some()).count(), 12);
        // A button takes the size too.
        let frame = tree(select(false).font_size(15.0)).layout(VIEWPORT);
        assert!(frame.commands.iter().any(|command| matches!(command,
            crate::DrawCommand::Text { text, font_size, .. } if text == "Bash" && *font_size == 15.0)));
    }

    #[test]
    fn a_field_select_focused_from_the_keyboard_has_a_fields_border_not_a_ring() {
        let palette = ComponentPalette::neutral_dark();
        for open in [false, true] {
            let mut tree = tree(field(open));
            tree.layout(VIEWPORT);
            assert!(tree.focus("toggle"));
            let frame = tree.layout(VIEWPORT);
            let trigger = rect(&frame, "toggle").unwrap();
            let strokes: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    crate::DrawCommand::StrokeRect {
                        rect, color, width, ..
                    } if *rect == trigger => Some((*color, *width)),
                    _ => None,
                })
                .collect();
            // As a focused TextField: its border alone, once.
            assert_eq!(
                strokes,
                [(lighter(palette.border, 70), 1.5)],
                "open: {open}"
            );
        }
    }

    #[test]
    fn a_field_select_takes_the_hosts_outline_and_radius_and_keeps_the_ink_while_open() {
        let palette = ComponentPalette::neutral_dark();
        let (border, focus) = (Color::rgb(60, 60, 60), Color::rgb(88, 101, 242));
        // The trigger's own outlines, as their color, width and radius.
        let strokes = |select: Select, focused: bool| {
            let mut tree = tree(select);
            tree.layout(VIEWPORT);
            if focused {
                assert!(tree.focus("toggle"));
            }
            let frame = tree.layout(VIEWPORT);
            // Disabled, the trigger has no region, and its outline is the only one drawn.
            let trigger = rect(&frame, "toggle");
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    crate::DrawCommand::StrokeRect { rect, color, width, radius }
                        if trigger.is_none_or(|trigger| *rect == trigger) =>
                    {
                        Some((*color, *width, *radius))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let set = |open: bool| field(open).outline(border, focus).radius(8.0);
        assert_eq!(strokes(set(false), false), [(border, 1.0, 8.0)]);
        assert_eq!(strokes(set(false), true), [(focus, 1.5, 8.0)], "focused: the focus border, once");
        assert_eq!(strokes(set(true), false), [(focus, 1.5, 8.0)], "open: the focus color");
        assert_eq!(strokes(set(true).ink(palette.text), false), [(palette.text, 2.0, 8.0)], "open: the ink wins");
        assert_eq!(strokes(set(false).disabled(), false), [(border, 1.0, 8.0)]);
        // A radius that is not finite is the control's.
        assert_eq!(strokes(field(false).radius(f32::NAN), false), [(palette.border, 1.0, control_radius())]);
    }

    #[test]
    fn a_field_select_takes_a_size_or_a_height_of_its_own_and_never_a_bad_one() {
        let height = |select: Select| {
            rect(&tree(select).layout(VIEWPORT), "toggle")
                .unwrap()
                .height
        };
        assert_eq!(height(field(false).size(ControlSize::Compact)), 28.0);
        assert_eq!(
            height(field(false).size(ControlSize::Dense)),
            28.0,
            "never under a field's least"
        );
        assert_eq!(height(field(false).size(ControlSize::Spacious)), 36.0);
        assert_eq!(height(field(false).height(41.0)), 41.0);
        assert_eq!(
            height(field(false).size(ControlSize::Spacious).height(30.0)),
            36.0
        );
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -1.0] {
            assert_eq!(height(field(false).height(bad)), 32.0, "{bad}");
            assert_eq!(height(select(false).height(bad)), 28.0, "{bad}");
        }
        // The button keeps its 28 unless told otherwise, and takes both too.
        assert_eq!(height(select(false)), 28.0);
        assert_eq!(height(select(false).size(ControlSize::Spacious)), 36.0);
        assert_eq!(height(select(false).height(40.0)), 40.0);
        // A taller field keeps its choice on the middle line.
        let frame = tree(field(false).height(48.0)).layout(VIEWPORT);
        let trigger = rect(&frame, "toggle").unwrap();
        let (_, icons) = drawn(&frame);
        assert_eq!(icons[0].1.y + icons[0].1.height / 2.0, trigger.y + 24.0);
    }

    #[test]
    fn a_disabled_field_select_is_passive_in_its_place_and_says_so() {
        let place = |select: Select| {
            let frame = tree(select).layout(VIEWPORT);
            let node = frame
                .access
                .iter()
                .find(|node| node.id == "toggle")
                .cloned()
                .expect("a node");
            (frame, node)
        };
        let (enabled, said) = place(field(true).accessibility_label("Shell"));
        let (disabled, quiet) = place(field(true).accessibility_label("Shell").disabled());
        assert!(disabled
            .regions
            .iter()
            .all(|region| region.id != "toggle" && region.id != "shell:zsh"));
        assert_eq!(quiet.rect, said.rect);
        assert_eq!(
            (quiet.role, quiet.name.as_str(), quiet.disabled),
            (Role::ComboBox, "Shell", true)
        );
        assert_eq!(
            (quiet.expanded, quiet.value_text.as_deref()),
            (Some(false), Some("Bash"))
        );
        // The choice and the chevron where the enabled one has them, muted.
        let (label, icons) = drawn(&enabled);
        let (quiet_label, quiet_icons) = drawn(&disabled);
        assert_eq!(quiet_label.unwrap().x, label.unwrap().x);
        // Disabled it stays closed, so its chevron points down.
        assert_eq!((quiet_icons[0].0, quiet_icons[0].1), (Icon::ChevronDown, icons[0].1));
        // Unnamed, the choice it shows is its name.
        let (_, unnamed) = place(field(false).disabled());
        assert_eq!(unnamed.name, "Bash");
        // Narrow, it is as wide as the narrowest text field, and its icons
        // stay inside it, enabled or not.
        for select in [
            field(false).icon(Icon::Terminal).width(40.0),
            field(false).icon(Icon::Terminal).width(40.0).disabled(),
        ] {
            let frame = tree(select).layout(VIEWPORT);
            let node = frame
                .access
                .iter()
                .find(|node| node.id == "toggle")
                .expect("a node");
            assert_eq!(node.rect.width, 80.0);
            let (_, icons) = drawn(&frame);
            assert_eq!(icons.len(), 2);
            assert!(icons.iter().all(
                |(_, icon)| icon.x >= node.rect.x && icon.x + icon.width <= node.rect.x + 80.0
            ));
        }
    }

    #[test]
    fn disabled_it_is_passive_and_stays_closed() {
        let mut tree = tree(select(true).enabled(false));
        let frame = tree.layout(VIEWPORT);
        assert!(frame.regions.iter().all(|region| region.id != "toggle" && region.id != "shell:zsh"));
        assert!(!tree.focus("toggle"));
    }

    #[test]
    fn a_header_only_appears_in_the_open_menu_and_leaves_the_trigger_unchanged() {
        let header = || {
            crate::SearchField::new(
                "query",
                "",
                "Filter shells",
                ComponentPalette::neutral_dark(),
            )
            .size(ControlSize::Compact)
            .width(184.0)
            .render()
        };
        let closed = tree(select(false)).layout(VIEWPORT);
        let with_closed_header = tree(select(false).header(header())).layout(VIEWPORT);
        assert_eq!(
            with_closed_header, closed,
            "an optional header must not change the closed markup"
        );
        let baseline = tree(select(true)).layout(VIEWPORT);
        let open = tree(select(true).header(header())).layout(VIEWPORT);
        assert_eq!(rect(&open, "toggle"), rect(&baseline, "toggle"));
        let query = rect(&open, "query").expect("the open menu contains its header");
        let surface = rect(&open, "popover_surface:dismiss").unwrap();
        let first = rect(&open, "shell:bash").unwrap();
        assert!(query.x >= surface.x && query.x + query.width <= surface.x + surface.width);
        assert!(query.y >= surface.y && query.y + query.height <= first.y);
        assert!(first.y > rect(&baseline, "shell:bash").unwrap().y);
        assert!(!baseline.regions.iter().any(|region| region.id == "query"));
    }

    #[test]
    fn a_header_search_keeps_filtering_owned_by_the_host_and_shares_menu_focus_and_escape() {
        let render = |query: &str, open| {
            let options = [("shell:bash", "Bash"), ("shell:zsh", "Zsh")]
                .into_iter()
                .filter(|(_, label)| label.to_ascii_lowercase().contains(query))
                .map(|(id, label)| DropdownOption::new(id, label))
                .collect();
            Select::new(
                "toggle",
                "Bash",
                options,
                open,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .header(
                crate::SearchField::new(
                    "query",
                    query,
                    "Filter shells",
                    ComponentPalette::neutral_dark(),
                )
                .size(ControlSize::Compact)
                .width(184.0)
                .render(),
            )
            .render()
            .popover_initial_focus("query")
            .popover_return_focus("toggle")
        };
        let mut tree = UiTree::new(render("", true));
        tree.set_reduced_motion(true);
        tree.layout(VIEWPORT);
        assert_eq!(tree.focused_id(), Some("query"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("z".into())),
            vec![UiAction::InputChanged {
                id: "query".into(),
                value: "z".into()
            }]
        );
        tree.set_root(render("z", true));
        let filtered = tree.layout(VIEWPORT);
        assert_eq!(tree.focused_id(), Some("query"));
        assert!(
            !filtered
                .regions
                .iter()
                .any(|region| region.id == "shell:bash")
        );
        assert!(
            filtered
                .regions
                .iter()
                .any(|region| region.id == "shell:zsh")
        );
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        assert_eq!(
            tree.handle_event(key(Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        tree.set_root(render("z", false));
        tree.layout(VIEWPORT);
        assert_eq!(tree.focused_id(), Some("toggle"));
        tree.set_root(render("z", true));
        tree.layout(VIEWPORT);
        assert!(tree.focus("shell:zsh"));
        assert_eq!(
            tree.handle_event(key(Key::Enter)),
            vec![UiAction::Clicked {
                id: "shell:zsh".into()
            }]
        );
    }

    #[test]
    fn an_option_shows_a_picture_or_a_surface_in_its_icon_slot() {
        let picture = crate::RasterImage::new(
            "a".repeat(64),
            crate::RasterImageFormat::Png,
            4,
            std::sync::Arc::from("AAAA"),
            "red",
        )
        .unwrap();
        let select = Select::new(
            "color",
            "Red",
            vec![
                DropdownOption::new("color:red", "Red").image(picture).selected(true),
                DropdownOption::new("color:party", "Party").surface("animation:party"),
                DropdownOption::new("color:plain", "Plain"),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        );
        let frame = tree(select).layout(VIEWPORT);
        let row = |id: &str| rect(&frame, id).expect("a row");
        let image = frame
            .commands
            .iter()
            .find_map(|command| match command {
                crate::DrawCommand::RasterImage { rect, .. } => Some(*rect),
                _ => None,
            })
            .expect("the picture");
        let (surface, label, fit) = frame
            .commands
            .iter()
            .find_map(|command| match command {
                crate::DrawCommand::Surface { rect, id, label, fit, .. } if id == "animation:party" => {
                    Some((*rect, label.clone(), *fit))
                }
                _ => None,
            })
            .expect("the surface");
        assert_eq!((label.as_str(), fit), ("Party", crate::SurfaceFit::Contain));
        for (slot, id) in [(image, "color:red"), (surface, "color:party")] {
            let row = row(id);
            assert!(slot.width <= ICON_SIZE && slot.height <= ICON_SIZE, "{id}: {slot:?}");
            assert!(slot.y >= row.y && slot.y + slot.height <= row.y + row.height, "{id} in its row");
            assert!(slot.x < row.x + 40.0, "{id} at the row's start");
        }
        // Every label starts after the slot, with a picture or without.
        let text_x = |said: &str| {
            frame.commands.iter().find_map(|command| match command {
                crate::DrawCommand::Text { text, position, .. } if text == said && position.y > row("color:red").y - 1.0 => {
                    Some(position.x)
                }
                _ => None,
            })
        };
        assert_eq!(text_x("Party"), text_x("Plain"));
        assert!(text_x("Plain").unwrap() > surface.x + surface.width);
    }
}
