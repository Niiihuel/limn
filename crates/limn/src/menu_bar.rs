//! The application menus across the top of a window, or behind one button.

use crate::{
    components::control_radius, page::px, ActiveHover, Button, ButtonSize, ButtonVariant,
    Component, ComponentPalette, ControlSize, ControlState, DropdownMenu, DropdownMenuEntry,
    DropdownMenuItem, Element, FontFamily, Icon, IconButton, TooltipSpec,
};

const FONT_SIZE: f32 = 12.0;
const PADDING: f32 = 4.0;

/// One menu of a [`MenuBar`]: its trigger and what it opens. The host owns
/// the trigger's width, since it knows the label's language.
pub struct MenuBarMenu {
    id: String,
    label: String,
    width: f32,
    dismiss_id: String,
    compact_id: Option<String>,
    entries: Vec<DropdownMenuEntry>,
    enabled: bool,
}

impl MenuBarMenu {
    /// `id` is the trigger's, reported when it is clicked; `dismiss_id` is
    /// reported when the open menu is left. A width that is not finite is
    /// none.
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        width: f32,
        dismiss_id: impl Into<String>,
        entries: Vec<DropdownMenuEntry>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            width: px(width),
            dismiss_id: dismiss_id.into(),
            compact_id: None,
            entries,
            enabled: true,
        }
    }

    /// The ID of this menu's row in the compact bar, `{id}:compact` unless
    /// set.
    pub fn compact_id(mut self, id: impl Into<String>) -> Self {
        self.compact_id = Some(id.into());
        self
    }

    /// A disabled menu keeps its trigger's place with a muted label and
    /// cannot be opened: the trigger is passive, and an `open_id` that names
    /// it opens nothing. In the compact bar it is a disabled row.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

struct Compact {
    id: String,
    dismiss_id: String,
    tooltip: TooltipSpec,
}

/// A row of menu triggers, one of which may be open. The host owns which:
/// each trigger reports a click on its ID, and the open menu takes keyboard
/// focus on its first enabled item. In a narrow window the same menus stand
/// as the submenus of one button.
pub struct MenuBar {
    menus: Vec<MenuBarMenu>,
    open_id: Option<String>,
    compact: Option<Compact>,
    menu_palette: ComponentPalette,
    palette: ComponentPalette,
}

impl MenuBar {
    /// Height of the bar and of its triggers.
    pub const HEIGHT: f32 = 24.0;
    /// Space between two triggers.
    pub const GAP: f32 = 2.0;
    /// Width of the compact bar, which its one button fills.
    pub const COMPACT_WIDTH: f32 = 32.0;

    /// `open_id` is the ID of the menu that is open, if any.
    pub fn new(menus: Vec<MenuBarMenu>, open_id: Option<&str>, palette: ComponentPalette) -> Self {
        Self {
            menus,
            open_id: open_id.map(str::to_owned),
            compact: None,
            menu_palette: palette,
            palette,
        }
    }

    /// Color the open menus from this palette instead of the bar's own.
    pub fn menu_palette(mut self, palette: ComponentPalette) -> Self {
        self.menu_palette = palette;
        self
    }

    /// Fold the bar into one button `id` that opens every menu as a submenu.
    /// It is open while any `open_id` is set, and reports `dismiss_id` when
    /// left.
    pub fn compact(mut self, id: impl Into<String>, dismiss_id: impl Into<String>, tooltip: TooltipSpec) -> Self {
        self.compact = Some(Compact { id: id.into(), dismiss_id: dismiss_id.into(), tooltip });
        self
    }

    /// The width the bar takes: its triggers and the gaps between them, or
    /// the one button when compact.
    pub fn width(&self) -> f32 {
        if self.compact.is_some() {
            return Self::COMPACT_WIDTH;
        }
        let gaps = self.menus.len().saturating_sub(1) as f32 * Self::GAP;
        self.menus.iter().map(|menu| menu.width).sum::<f32>() + gaps
    }
}

impl Component for MenuBar {
    fn render(self) -> Element {
        let width = self.width();
        let (palette, menu_palette) = (self.palette, self.menu_palette);
        if let Some(compact) = self.compact {
            let open = self.open_id.is_some();
            let items = if open {
                self.menus
                    .into_iter()
                    .map(|menu| {
                        let id = menu.compact_id.unwrap_or_else(|| format!("{}:compact", menu.id));
                        DropdownMenuItem::new(id, menu.label)
                            .submenu(menu.entries)
                            .enabled(menu.enabled)
                            .into()
                    })
                    .collect()
            } else {
                Vec::new()
            };
            let button = IconButton::new(compact.id, Icon::Menu, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .active(open)
                .active_hover(ActiveHover::Palette)
                .tooltip_with(compact.tooltip)
                .render();
            return DropdownMenu::new(
                Element::panel(button).size(Some(width), Some(Self::HEIGHT)),
                items,
                open,
                compact.dismiss_id,
                menu_palette,
            )
            .context_menu_style()
            .render();
        }
        let open_id = self.open_id;
        let triggers = self
            .menus
            .into_iter()
            .map(|menu| {
                if !menu.enabled {
                    // The label where the button draws its own, and nothing to press.
                    return Element::text(menu.label)
                        .font_family(FontFamily::Sans)
                        .font_size(FONT_SIZE)
                        .foreground(palette.muted)
                        .center_content()
                        .centered_y()
                        .padding(PADDING)
                        .background(palette.surface)
                        .radius(control_radius())
                        .size(Some(menu.width), Some(Self::HEIGHT));
                }
                let open = open_id.as_deref() == Some(menu.id.as_str());
                let anchor = palette.interactive_surface(
                    Button::new(menu.id, menu.label, palette)
                        .variant(ButtonVariant::Ghost)
                        .button_size(ButtonSize::Small)
                        .render()
                        .font_size(FONT_SIZE)
                        .padding(PADDING)
                        .min_size(0.0, 0.0)
                        .background(if open { palette.selected } else { palette.surface })
                        .size(Some(menu.width), Some(Self::HEIGHT)),
                );
                let entries = if open { menu.entries } else { Vec::new() };
                let initial_focus = entries.iter().find_map(|entry| match entry {
                    DropdownMenuEntry::Item(item) if item.state == ControlState::Enabled => {
                        Some(item.action_id.clone())
                    }
                    _ => None,
                });
                let dropdown = DropdownMenu::new(anchor, entries, open, menu.dismiss_id, menu_palette)
                    .context_menu_style()
                    .render();
                match initial_focus {
                    Some(id) => dropdown.popover_initial_focus(id),
                    None => dropdown,
                }
            })
            .collect();
        Element::row(triggers).gap(Self::GAP).size(Some(width), Some(Self::HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const SIZE: Size = Size { width: 400.0, height: 300.0 };

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn bar(open: Option<&str>) -> MenuBar {
        let menu = |id: &str, label: &str, width: f32, items: Vec<DropdownMenuEntry>| {
            MenuBarMenu::new(id, label, width, format!("{id}:dismiss"), items)
        };
        MenuBar::new(
            vec![
                menu(
                    "file",
                    "File",
                    40.0,
                    vec![
                        DropdownMenuItem::new("revert", "Revert").disabled().into(),
                        DropdownMenuItem::new("open", "Open…").into(),
                        DropdownMenuItem::new("save", "Save").into(),
                    ],
                ),
                menu("edit", "Edit", 44.0, vec![DropdownMenuItem::new("undo", "Undo").into()]),
                menu("view", "View", 48.0, Vec::new()),
            ],
            open,
            palette(),
        )
    }

    fn tree_of(bar: MenuBar) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![bar.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn rect(frame: &crate::Frame, id: &str) -> Option<Rect> {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .map(|region| region.rect)
    }

    fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key { key, ctrl: false, shift: false })
    }

    #[test]
    fn triggers_take_their_widths_and_report_their_ids() {
        assert_eq!(bar(None).width(), 40.0 + 44.0 + 48.0 + 2.0 * MenuBar::GAP);
        let mut tree = tree_of(bar(None));
        let frame = tree.layout(SIZE);
        assert_eq!(rect(&frame, "file"), Some(Rect::new(0.0, 0.0, 40.0, MenuBar::HEIGHT)));
        assert_eq!(rect(&frame, "edit"), Some(Rect::new(42.0, 0.0, 44.0, MenuBar::HEIGHT)));
        assert_eq!(rect(&frame, "view"), Some(Rect::new(88.0, 0.0, 48.0, MenuBar::HEIGHT)));
        assert!(!tree.has_open_popover());
        assert!(rect(&frame, "open").is_none());
        let center = Point::new(60.0, 12.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: "edit".into() }]);
    }

    #[test]
    fn the_open_menu_fills_its_trigger_focuses_its_first_enabled_item_and_closes_on_escape() {
        let mut tree = tree_of(bar(Some("file")));
        let frame = tree.layout(SIZE);
        assert!(tree.has_open_popover());
        let fill = |id: &str| {
            let trigger = rect(&frame, id).unwrap();
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *rect == trigger => Some(*color),
                _ => None,
            })
        };
        assert_eq!(fill("file"), Some(palette().selected));
        assert_ne!(fill("edit"), Some(palette().selected));
        // Only the open menu's items exist; the first enabled one has focus.
        assert!(rect(&frame, "save").is_some() && rect(&frame, "undo").is_none());
        assert_eq!(key(&mut tree, Key::Enter), vec![UiAction::Clicked { id: "open".into() }]);
        key(&mut tree, Key::ArrowDown);
        assert_eq!(key(&mut tree, Key::Enter), vec![UiAction::Clicked { id: "save".into() }]);
        assert!(key(&mut tree, Key::Escape).contains(&UiAction::Clicked { id: "file:dismiss".into() }));
        // A menu with nothing to focus still opens.
        let mut tree = tree_of(bar(Some("view")));
        tree.layout(SIZE);
        assert!(key(&mut tree, Key::Escape).contains(&UiAction::Clicked { id: "view:dismiss".into() }));
    }

    #[test]
    fn a_disabled_menu_keeps_its_place_and_cannot_be_opened_by_pointer_or_keyboard() {
        let with_edit = |enabled: bool, open: Option<&str>| {
            let mut bar = bar(open);
            let edit = bar.menus.remove(1).enabled(enabled);
            bar.menus.insert(1, edit);
            bar
        };
        let labels = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, color, .. } => Some((text.clone(), *position, *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(with_edit(false, None).width(), bar(None).width());
        let mut tree = tree_of(with_edit(false, None));
        let frame = tree.layout(SIZE);
        assert!(frame.regions.iter().all(|region| region.id != "edit"), "{:?}", frame.regions);
        assert!(!tree.focus("edit"));
        let center = Point::new(60.0, 12.0);
        assert!(tree.handle_event(UiEvent::PointerDown(center)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(center)).is_empty());
        assert!(tree.focus("file"));
        assert_eq!(key(&mut tree, Key::Enter), vec![UiAction::Clicked { id: "file".into() }]);
        // The label is muted and stands where the enabled one does.
        let enabled = tree_of(with_edit(true, None)).layout(SIZE);
        let (shown, was) = (labels(&frame), labels(&enabled));
        assert_eq!(shown[1], ("Edit".to_owned(), was[1].1, palette().muted));
        assert_eq!((&shown[0], &shown[2]), (&was[0], &was[2]));
        assert_eq!((rect(&frame, "file"), rect(&frame, "view")), (rect(&enabled, "file"), rect(&enabled, "view")));
        // A host that names it open opens nothing.
        let mut tree = tree_of(with_edit(false, Some("edit")));
        let frame = tree.layout(SIZE);
        assert!(!tree.has_open_popover() && rect(&frame, "undo").is_none());
        // In the compact bar it is a row that opens no submenu.
        let compact = with_edit(false, Some("file")).compact("all", "dismiss", TooltipSpec::new("Main menu"));
        let mut tree = tree_of(compact);
        let frame = tree.layout(SIZE);
        assert!(rect(&frame, "file:compact").is_some() && rect(&frame, "edit:compact").is_none());
        assert!(!tree.focus("edit:compact"));
    }

    #[test]
    fn the_compact_bar_is_one_button_with_the_menus_as_submenus() {
        let compact = |open: Option<&str>| bar(open).compact("all", "dismiss", TooltipSpec::new("Main menu"));
        assert_eq!(compact(None).width(), MenuBar::COMPACT_WIDTH);
        let mut tree = tree_of(compact(None));
        let frame = tree.layout(SIZE);
        assert!(rect(&frame, "file").is_none() && !tree.has_open_popover());
        assert_eq!(rect(&frame, "all"), Some(Rect::new(0.0, 0.0, MenuBar::COMPACT_WIDTH, MenuBar::HEIGHT)));
        let center = Point::new(16.0, 12.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: "all".into() }]);

        let mut tree = tree_of(compact(Some("edit")));
        let frame = tree.layout(SIZE);
        assert!(tree.has_open_popover());
        for id in ["file:compact", "edit:compact", "view:compact"] {
            assert!(rect(&frame, id).is_some(), "{id}");
        }
        assert!(tree.focus("file:compact"));
        tree.layout(SIZE);
        key(&mut tree, Key::ArrowRight);
        // The submenu skips its disabled first item, as the open menu does.
        assert!(key(&mut tree, Key::Enter).contains(&UiAction::Clicked { id: "open".into() }));
        let mut tree = tree_of(compact(Some("edit")));
        tree.layout(SIZE);
        assert!(key(&mut tree, Key::Escape).contains(&UiAction::Clicked { id: "dismiss".into() }));

        let named = MenuBar::new(
            vec![MenuBarMenu::new("file", "File", f32::NAN, "d", Vec::new()).compact_id("category:file")],
            Some("file"),
            palette(),
        );
        assert_eq!(named.width(), 0.0, "a width that is not finite is none");
        let mut tree = tree_of(named.compact("all", "dismiss", TooltipSpec::new("Main menu")));
        assert!(rect(&tree.layout(SIZE), "category:file").is_some());
    }
}
