//! A list of options under a trigger: a `DropdownMenu` of plain choices.

use crate::*;

/// One option in a dropdown list; its ID is the host action to execute.
pub struct DropdownOption {
    /// What the host receives when the option is chosen.
    pub action_id: String,
    /// The option's text, and its name for the platform.
    pub label: String,
    /// A leading icon. `None` by default.
    pub icon: Option<Icon>,
    /// Whether it is the current choice.
    pub selected: bool,
    /// Whether it can be chosen. A disabled option stays in the list.
    pub state: ControlState,
    /// A hint shown while the pointer rests on the option or it has the
    /// keyboard, such as the whole of a name its label shortens. `None` by
    /// default.
    pub tooltip: Option<String>,
    /// A picture in the icon's place, such as a custom emoji. `None` by
    /// default.
    pub picture: Option<InlineImage>,
    /// Whether a long label is cut with an ellipsis on one line, as in
    /// [`DropdownMenuItem::truncate`], instead of wrapping. `false` by
    /// default.
    pub truncate: bool,
}

impl DropdownOption {
    /// An enabled option, not selected, that reports `action_id`.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            state: ControlState::Enabled,
            tooltip: None,
            picture: None,
            truncate: false,
        }
    }

    /// Cut a long label with an ellipsis on one line instead of wrapping it,
    /// as a list of device names does; pair it with a `tooltip` that says
    /// the rest.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// A hint for the option, for what its label cannot say: the whole of a
    /// name the label shortens, say. A blank hint is ignored.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.tooltip = (!text.trim().is_empty()).then_some(text);
        self
    }

    /// Give the option an icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Show a picture in the icon's place; see [`DropdownMenuItem::image`].
    pub fn image(mut self, image: RasterImage) -> Self {
        self.picture = Some(InlineImage::Raster(image));
        self
    }

    /// Show the host-fed surface `id` in the icon's place, for an
    /// animation; see [`DropdownMenuItem::surface`].
    pub fn surface(mut self, id: impl Into<String>) -> Self {
        self.picture = Some(InlineImage::Surface(id.into()));
        self
    }

    /// The menu item that shows the option.
    pub(crate) fn item(self) -> DropdownMenuItem {
        let mut item = DropdownMenuItem::new(self.action_id, self.label)
            .selected(self.selected)
            .enabled(self.state != ControlState::Disabled);
        if let Some(icon) = self.icon {
            item = item.icon(icon);
        }
        if let Some(hint) = self.tooltip {
            item = item.tooltip_with(TooltipSpec::new(hint));
        }
        if self.truncate {
            item = item.truncate();
        }
        match self.picture {
            Some(InlineImage::Raster(image)) => item.image(image),
            Some(InlineImage::Surface(id)) => item.surface(id),
            None => item,
        }
    }

    /// Mark the option as the current choice.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disable the option.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled { ControlState::Enabled } else { ControlState::Disabled };
        self
    }
}

/// A dropdown with caller-owned selection/open state and stable action IDs.
/// The anchor is a slot, so callers can supply an icon button, field, or any
/// other visual trigger without changing the list behavior. Arrow keys,
/// Home/End and Tab move through enabled options and reveal offscreen rows;
/// Enter activates the focused option. The host still owns the open state and
/// handles the resulting ID.
pub struct DropdownList {
    /// The element the list opens from: a button, a field or any other trigger.
    pub anchor: Element,
    /// The options, in the order they are listed.
    pub options: Vec<DropdownOption>,
    /// Whether the list shows. The host owns it.
    pub open: bool,
    /// Reported when the list is dismissed, for the host to close it.
    pub dismiss_id: String,
    /// Width of the list in logical pixels, 240 by default and 96 at least when
    /// it is drawn. `sizing` replaces it with the profile's preferred width.
    pub width: f32,
    /// The side of the anchor it prefers. `BottomStart` by default.
    pub placement: PopoverPlacement,
    /// The colors of the list's surface and of its rows.
    pub palette: ComponentPalette,
    /// Durations of the opening and of the item feedback.
    pub motion: MenuMotion,
    /// A selector profile for the row density and the scroll budget. `None`, the
    /// default, keeps the menu's own.
    pub sizing: Option<crate::SelectorSizing>,
}

impl DropdownList {
    /// A list of `options` on `anchor`, shown while `open` is true and
    /// dismissed with `dismiss_id`. It opens under the start of the anchor,
    /// 240 pixels wide.
    pub fn new(
        anchor: Element,
        options: Vec<DropdownOption>,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            options,
            open,
            dismiss_id: dismiss_id.into(),
            width: 240.0,
            placement: PopoverPlacement::BottomStart,
            palette,
            motion: MenuMotion::default(),
            sizing: None,
        }
    }

    /// Width of the list in logical pixels, at least 96. A width that is not
    /// finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// The side of the anchor it prefers. The default is `BottomStart`.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Share the width, row density and scroll budget of other selectors.
    /// Call before `width` to override the preferred width for the host.
    pub fn sizing(mut self, sizing: crate::SelectorSizing) -> Self {
        self.width = sizing.preferred_width;
        self.sizing = Some(sizing);
        self
    }

    /// Durations of the opening and of the item feedback; see `MenuMotion`.
    pub fn motion(mut self, motion: MenuMotion) -> Self {
        self.motion = motion;
        self
    }
}

impl Component for DropdownList {
    fn render(self) -> Element {
        let entries = self
            .options
            .into_iter()
            .map(|option| DropdownMenuEntry::Item(option.item()))
            .collect();
        let mut menu = DropdownMenu::new(
            self.anchor,
            entries,
            self.open,
            self.dismiss_id,
            self.palette,
        );
        if let Some(sizing) = self.sizing {
            menu = menu.sizing(sizing);
        }
        menu.width(self.width)
            .placement(self.placement)
            .motion(self.motion)
            .render()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn a_dropdown_list_says_which_option_is_the_chosen_one() {
        let palette = ComponentPalette::neutral_dark();
        let list = DropdownList::new(
            Button::new("open", "Shell", palette).render(),
            vec![
                DropdownOption::new("bash", "Bash").selected(true),
                DropdownOption::new("zsh", "Zsh"),
                DropdownOption::new("off", "Off").disabled(),
            ],
            true,
            "dismiss",
            palette,
        );
        let mut tree = UiTree::new(Element::column(vec![list.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let nodes = tree.layout(Size::new(480.0, 320.0)).access;
        let said = |id: &str| {
            let node = nodes.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id} among {nodes:?}"));
            (node.role, node.name.as_str(), node.selected, node.disabled)
        };
        assert_eq!(said("bash"), (Role::MenuItem, "Bash", Some(true), false));
        assert_eq!(said("zsh"), (Role::MenuItem, "Zsh", Some(false), false));
        assert_eq!(said("off"), (Role::MenuItem, "Off", Some(false), true));
    }

    #[test]
    fn dropdown_uses_shared_popover_and_keeps_item_ids() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("backend:codex", "Codex").selected(true),
                DropdownOption::new("backend:claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        let frame = tree.layout(Size::new(400.0, 300.0));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:codex"));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:claude"));
        assert_eq!(frame.hit_test(Point::new(390.0, 290.0)), Some("dismiss"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Escape,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn dropdown_list_sizing_preserves_compact_rows_and_keyboard_scroll() {
        let sizing = crate::SelectorSizing {
            max_visible_items: 2,
            ..crate::SelectorSizing::COMPACT
        };
        let dropdown = DropdownList::new(
            Element::button("trigger", "Choice").size(Some(160.0), Some(36.0)),
            (0..5)
                .map(|index| {
                    DropdownOption::new(format!("choice:{index}"), format!("Choice {index}"))
                        .icon(Icon::Folder)
                        .selected(index == 0)
                })
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .sizing(sizing)
        .width(220.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(360.0, 280.0));
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap();
        assert_eq!(surface.rect.width, 220.0);
        let first = frame
            .regions
            .iter()
            .find(|region| region.id == "choice:0")
            .unwrap();
        assert_eq!(first.rect.height, sizing.row_height);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, .. }
                if text == "Choice 0" && *font_size == sizing.label_size
        )));
        assert!(tree.focus("choice:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::End,
            shift: false,
            ctrl: false,
            alt: false,
        });
        let final_frame = tree.layout(Size::new(360.0, 280.0));
        assert_eq!(tree.focused_id(), Some("choice:4"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(final_frame
            .regions
            .iter()
            .any(|region| region.id == "choice:4"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Enter,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked {
                id: "choice:4".into()
            }]
        );
    }

    #[test]
    fn selected_dropdown_option_keeps_selected_hover_color() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agent"),
            vec![
                DropdownOption::new("codex", "Codex").selected(true),
                DropdownOption::new("claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let ElementKind::Popover { floating, .. } = dropdown.kind else {
            panic!("open dropdown must have a popover");
        };
        let ElementKind::Panel(content) = floating.kind else {
            panic!("popover must contain a panel");
        };
        let ElementKind::Column(options) = content.kind else {
            panic!("short dropdown must contain an option column");
        };
        assert_eq!(
            options[0].style.hover_background,
            Some(lighter(palette.selected, 8))
        );
        assert_eq!(options[1].style.hover_background, Some(palette.hover));
    }

    #[test]
    fn dropdown_rows_keep_one_hit_target_and_stable_icon_columns() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("codex", "Codex")
                    .icon(Icon::Chat)
                    .selected(true),
                DropdownOption::new("native", "Nativo"),
                DropdownOption::new("offline", "No instalado").disabled(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 300.0));
        let rows: Vec<_> = frame
            .regions
            .iter()
            .filter(|region| region.id == "codex" || region.id == "native")
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rect.width, rows[1].rect.width);
        assert_eq!(rows[0].rect.height, 36.0);
        assert_eq!(rows[1].rect.height, 36.0);
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
        assert_eq!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    DrawCommand::Icon {
                        icon: Icon::Check,
                        ..
                    }
                ))
                .count(),
            1
        );
        tree.handle_event(UiEvent::PointerMove(Point::new(
            rows[1].rect.x + 4.0,
            rows[1].rect.y + 4.0,
        )));
        let hovered = tree.layout(Size::new(400.0, 300.0));
        assert!(hovered.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == rows[1].rect && *color == palette.hover
        )));
    }

    #[test]
    fn dropdown_keyboard_stays_in_menu_skips_disabled_and_returns_to_trigger() {
        let palette = ComponentPalette::neutral_dark();
        let make_dropdown = || {
            DropdownList::new(
                Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
                vec![
                    DropdownOption::new("codex", "Codex"),
                    DropdownOption::new("offline", "No instalado").disabled(),
                    DropdownOption::new("claude", "Claude"),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(170.0)
            .render()
        };
        let key = |key, shift| UiEvent::Key {
            key,
            shift,
            ctrl: false,
            alt: false,
        };
        let mut tree = UiTree::new(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("trigger"));

        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::End, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Home, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::Tab, true));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Enter, false)),
            vec![UiAction::Clicked { id: "codex".into() }]
        );

        tree.set_root(Element::button("trigger", "Agente"));
        tree.layout(Size::new(400.0, 300.0));
        assert_eq!(tree.focused_id(), Some("trigger"));

        tree.set_root(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Escape, false)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn long_dropdown_reveals_options_reached_by_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let options = (0..12)
            .map(|index| {
                let option =
                    DropdownOption::new(format!("option:{index}"), format!("Opción {index}"));
                if index == 5 {
                    option.disabled()
                } else {
                    option
                }
            })
            .collect();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            options,
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        let viewport = Size::new(400.0, 400.0);
        tree.layout(viewport);
        assert!(tree.focus("trigger"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        for _ in 0..9 {
            tree.handle_event(key(crate::Key::ArrowDown));
        }
        assert_eq!(tree.focused_id(), Some("option:9"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        let scrolled = tree.layout(viewport);
        assert!(scrolled
            .regions
            .iter()
            .any(|region| region.id == "option:9"));
        tree.handle_event(key(crate::Key::End));
        assert_eq!(tree.focused_id(), Some("option:11"));
        let end = tree.layout(viewport);
        assert!(end.regions.iter().any(|region| region.id == "option:11"));
        tree.handle_event(key(crate::Key::Home));
        assert_eq!(tree.focused_id(), Some("option:0"));
        assert_eq!(tree.scroll_offset("dismiss:options"), 0.0);
        let home = tree.layout(viewport);
        assert!(home.regions.iter().any(|region| region.id == "option:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::Tab,
            shift: true,
            ctrl: false,
            alt: false,
        });
        assert_eq!(tree.focused_id(), Some("option:11"));
        let wrapped = tree.layout(viewport);
        assert!(wrapped
            .regions
            .iter()
            .any(|region| region.id == "option:11"));
    }

    #[test]
    fn clicking_outside_dropdown_returns_focus_before_dismiss_action() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![DropdownOption::new("codex", "Codex")],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("codex"));
        let outside = Point::new(390.0, 290.0);
        tree.handle_event(UiEvent::PointerDown(outside));
        assert_eq!(tree.focused_id(), Some("trigger"));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(outside)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn disabled_options_are_visible_without_click_regions() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![Tab::new("soon", "Próximamente", 140.0).disabled()],
            palette,
        )
        .render();
        let frame = UiTree::new(tabs).layout(Size::new(180.0, 40.0));
        assert!(!frame.regions.iter().any(|region| region.id == "soon"));
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente"),
            vec![DropdownOption::new("offline", "No instalado").disabled()],
            true,
            "dismiss",
            palette,
        )
        .render();
        let frame = UiTree::new(dropdown).layout(Size::new(300.0, 200.0));
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
    }
}
