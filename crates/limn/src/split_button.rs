//! A primary action joined to a menu of the others, such as "Always allow"
//! beside a chevron that offers "Allow once".

use crate::{
    Access, Button, ButtonVariant, Component, ComponentPalette, ControlSize, CornerRadii, DropdownMenu,
    DropdownMenuEntry, Element, FontFamily, Icon, IconButton, Kbd, PopoverPlacement, Role,
};

// The parent surface shows through this seam, so the halves read as two targets.
const SEAM: f32 = 1.0;

/// Two buttons drawn as one: the action at the start, which may show its
/// keyboard shortcut in keycaps, and a chevron at the end that opens a
/// [`DropdownMenu`]. Each half is its own target, reached by Tab on its own.
/// The host owns whether the menu is open: the action reports
/// `UiAction::Clicked` with its `id`, the chevron with `menu_id`, and the
/// menu its items and its `dismiss_id`, as a [`DropdownMenu`] does.
pub struct SplitButton {
    id: String,
    label: String,
    menu_id: String,
    palette: ComponentPalette,
    variant: ButtonVariant,
    keys: Vec<String>,
    menu: Option<(Vec<DropdownMenuEntry>, bool, String)>,
}

impl SplitButton {
    /// An action with this label that reports `id`, joined to a chevron that
    /// reports `menu_id`. It starts as the default variant, 32 px high, with
    /// no keycaps and no menu.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        menu_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            menu_id: menu_id.into(),
            palette,
            variant: ButtonVariant::default(),
            keys: Vec::new(),
            menu: None,
        }
    }

    /// The fill, ink and hover of both halves, as [`Button::variant`].
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The action's keyboard shortcut, one key to a cap, drawn after the label
    /// in the button's ink, such as `&["Ctrl", "Enter"]`. The platform is told
    /// it as the action's description. Empty keys are left out.
    pub fn keys(mut self, keys: &[&str]) -> Self {
        self.keys = keys.iter().map(|key| key.trim()).filter(|key| !key.is_empty()).map(str::to_owned).collect();
        self
    }

    /// The menu the chevron opens, drawn while `open`, below the button and
    /// aligned to its end. `dismiss_id` is reported when it is dismissed, as
    /// [`DropdownMenu::new`] says.
    pub fn menu(mut self, entries: Vec<DropdownMenuEntry>, open: bool, dismiss_id: impl Into<String>) -> Self {
        self.menu = Some((entries, open, dismiss_id.into()));
        self
    }
}

impl Component for SplitButton {
    fn render(self) -> Element {
        let palette = self.palette;
        let (base, hover, pressed, ink, outline) = Button::new("", "", palette).variant(self.variant).colors();
        let sizing = ControlSize::Regular.button_sizing();
        let radius = crate::tokens().radius.control;
        let shortcut = self.keys.join("+");

        let mut content = vec![Element::truncated_text(self.label.clone())
            .foreground(ink)
            .font_family(FontFamily::Sans)
            .font_size(sizing.font_size)
            .centered_y()
            .shrink_to_fit()];
        if !shortcut.is_empty() {
            // The keycaps take the button's ink, so they read on a light fill as on a dark one.
            content.push(Kbd::new(shortcut.clone(), ComponentPalette { text: ink, ..palette }).render().centered_y());
        }
        let pad = || Element::spacer().size(Some(sizing.horizontal_padding), None);
        let mut said = Access::new(Role::Button).name(self.label);
        if !shortcut.is_empty() {
            said = said.description(shortcut);
        }
        let mut action = Element::clickable(
            self.id,
            Element::row(vec![pad(), Element::row(content).gap(sizing.inner_spacing).shrink_to_fit(), pad()]),
        )
        .shrink_to_fit()
        .background(base)
        .hover_background(hover)
        .pressed_background(pressed)
        .size(None, Some(sizing.height))
        .corner_radii(CornerRadii { top_left: radius, bottom_left: radius, ..CornerRadii::default() })
        .accessible(said);

        let mut chevron = IconButton::new(self.menu_id, Icon::ChevronDown, palette)
            .variant(self.variant)
            .accessibility_label(crate::labels().more_options)
            .render()
            .corner_radii(CornerRadii { top_right: radius, bottom_right: radius, ..CornerRadii::default() });
        if outline {
            action = action.border(palette.border, 1.0);
            chevron = chevron.border(palette.border, 1.0);
        }
        // The chevron is the menu's anchor, so it is the one said to be expanded.
        if let Some((entries, open, dismiss_id)) = self.menu {
            chevron = DropdownMenu::new(chevron, entries, open, dismiss_id, palette)
                .placement(PopoverPlacement::BottomEnd)
                .render();
        }
        Element::row(vec![action, chevron]).gap(SEAM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, DropdownMenuItem, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const WINDOW: Size = Size::new(480.0, 320.0);

    fn allow(open: bool) -> SplitButton {
        SplitButton::new("allow", "Always allow", "allow:menu", ComponentPalette::neutral_dark()).menu(
            vec![DropdownMenuItem::new("once", "Allow once").shortcut("Ctrl+Enter").into()],
            open,
            "allow:closed",
        )
    }

    fn tree(split: SplitButton) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![split.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree.layout(WINDOW);
        tree
    }

    fn rect(tree: &UiTree, id: &str) -> Rect {
        tree.frame().regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("{id}")).rect
    }

    fn click(tree: &mut UiTree, rect: Rect) -> Vec<UiAction> {
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point))
    }

    fn clicked(id: &str) -> Vec<UiAction> {
        vec![UiAction::Clicked { id: id.into() }]
    }

    #[test]
    fn the_halves_sit_side_by_side_and_each_reports_its_own_id() {
        let mut tree = tree(allow(false));
        let (action, chevron) = (rect(&tree, "allow"), rect(&tree, "allow:menu"));
        assert_eq!((action.x, action.height), (0.0, 32.0));
        assert_eq!((chevron.x, chevron.y, chevron.height), (action.x + action.width + SEAM, action.y, 32.0));
        assert_eq!(chevron.width, 32.0);
        assert_eq!(click(&mut tree, action), clicked("allow"));
        assert_eq!(click(&mut tree, chevron), clicked("allow:menu"));
    }

    #[test]
    fn tab_reaches_each_half_and_enter_reports_it() {
        let mut tree = tree(allow(false));
        let tab = UiEvent::Key { key: Key::Tab, shift: false, ctrl: false };
        let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
        for id in ["allow", "allow:menu"] {
            tree.handle_event(tab.clone());
            tree.layout(WINDOW);
            assert_eq!(tree.focused_id(), Some(id));
            assert_eq!(tree.handle_event(enter.clone()), clicked(id));
        }
    }

    #[test]
    fn keycaps_sit_inside_the_action_and_tell_the_platform_the_shortcut() {
        let plain = rect(&tree(allow(false)), "allow");
        let with_keys = tree(allow(false).keys(&["Ctrl", "Shift", "⏎", " "]));
        let action = rect(&with_keys, "allow");
        assert!(action.width > plain.width + 30.0, "{} vs {}", action.width, plain.width);
        let node = with_keys.frame().access.iter().find(|node| node.id == "allow").unwrap();
        assert_eq!(node.name, "Always allow");
        assert_eq!(node.description.as_deref(), Some("Ctrl+Shift+⏎"));
        let label = with_keys.frame().access.iter().find(|node| node.id == "allow:menu").unwrap();
        assert_eq!(label.name, crate::labels().more_options);
    }

    #[test]
    fn the_chevron_opens_the_menu_and_says_so() {
        let closed = tree(allow(false));
        let expanded = |tree: &UiTree, id: &str| tree.frame().access.iter().find(|node| node.id == id).unwrap().expanded;
        assert_eq!(expanded(&closed, "allow:menu"), Some(false));
        assert_eq!(expanded(&closed, "allow"), None);
        assert!(closed.frame().regions.iter().all(|region| region.id != "once"));

        // At the end of the window, as a dialog's actions are, so the menu has room to align.
        let mut open = UiTree::new(Element::column(vec![
            Element::row(vec![Element::spacer().flex(1.0), allow(true).render()]),
            Element::spacer().flex(1.0),
        ]));
        open.set_reduced_motion(true);
        open.layout(WINDOW);
        assert_eq!(expanded(&open, "allow:menu"), Some(true));
        let (chevron, item) = (rect(&open, "allow:menu"), rect(&open, "once"));
        assert!(item.y >= chevron.y + chevron.height);
        // Aligned to the end of the button, not its start.
        assert!(item.x + item.width <= chevron.x + chevron.width && item.x < chevron.x, "{item:?} {chevron:?}");
        assert_eq!(click(&mut open, item), clicked("once"));
    }

    #[test]
    fn both_halves_take_the_variants_fill() {
        let palette = ComponentPalette::neutral_dark();
        let (fill, ..) = Button::new("", "", palette).variant(ButtonVariant::Destructive).colors();
        let tree = tree(allow(false).variant(ButtonVariant::Destructive));
        let filled = |rect: Rect| {
            tree.frame().commands.iter().any(|command| match command {
                DrawCommand::FillCornerRect { rect: painted, color, .. } => *color == fill && *painted == rect,
                _ => false,
            })
        };
        assert!(filled(rect(&tree, "allow")));
        assert!(filled(rect(&tree, "allow:menu")));
    }

    #[test]
    fn a_long_label_in_a_narrow_window_stays_in_its_row() {
        let split = SplitButton::new("allow", "Always allow this tool in every project", "allow:menu", ComponentPalette::neutral_dark());
        let mut tree = UiTree::new(Element::column(vec![split.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(160.0, 100.0));
        let chevron = frame.regions.iter().find(|region| region.id == "allow:menu").unwrap().rect;
        assert!(chevron.x + chevron.width <= 160.0 + 0.5 && chevron.width == 32.0, "{chevron:?}");
    }
}
