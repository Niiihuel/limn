//! A button that shows the current choice and opens the list of the others.

use crate::{
    Button, ButtonVariant, Component, ComponentPalette, ControlSize, DropdownList, DropdownOption,
    Element, Icon, PopoverPlacement, SelectorSizing,
};

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
    enabled: bool,
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
            enabled: true,
        }
    }

    /// Paint the list with another palette than the trigger's.
    pub fn menu_palette(mut self, palette: ComponentPalette) -> Self {
        self.menu_palette = Some(palette);
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

    /// A disabled select shows its value, stays closed and cannot be focused.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for Select {
    fn render(self) -> Element {
        let width = self
            .width
            .unwrap_or(self.sizing.unwrap_or(SelectorSizing::COMPACT).min_width);
        let mut trigger = Button::new(self.id, self.current_label, self.palette)
            .variant(ButtonVariant::Outline)
            .size(ControlSize::Compact)
            .trailing_icon(Icon::ChevronDown)
            .width(width)
            .enabled(self.enabled);
        if let Some(icon) = self.icon {
            trigger = trigger.icon(icon);
        }
        let mut list = DropdownList::new(
            trigger.render(),
            self.options,
            self.open && self.enabled,
            self.dismiss_id,
            self.menu_palette.unwrap_or(self.palette),
        );
        let mut menu_width = width;
        if let Some(sizing) = self.sizing {
            list = list.sizing(sizing);
            menu_width = sizing.preferred_width;
        }
        list.width(self.menu_width.unwrap_or(menu_width))
            .placement(self.placement)
            .render()
            .size(Some(width), Some(ControlSize::Compact.height()))
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
        assert_eq!(option.height, SelectorSizing::COMPACT.row_height);
        let center = Point::new(option.x + 10.0, option.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked { id: "shell:zsh".into() }]
        );
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
        let key = |key| UiEvent::Key { key, shift: false, ctrl: false };
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
    fn disabled_it_is_passive_and_stays_closed() {
        let mut tree = tree(select(true).enabled(false));
        let frame = tree.layout(VIEWPORT);
        assert!(frame.regions.iter().all(|region| region.id != "toggle" && region.id != "shell:zsh"));
        assert!(!tree.focus("toggle"));
    }
}
