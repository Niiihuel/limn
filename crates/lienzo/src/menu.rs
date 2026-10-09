//! Shared menu primitives for popovers and context menus.
//!
//! The caller owns the menu's open state and the meaning of each action ID.
//! Each item owns one complete hit target, so moving between its icon, label,
//! and trailing hint never creates separate hover patches.

use crate::{Color, Component, ComponentPalette, Element, FontFamily, Icon, Kbd, ICON_SIZE};

/// The editor's context menu defines the visual standard for action menus.
/// Selector menus can keep their own density and richer option content.
pub(crate) mod action_style {
    use crate::{ComponentPalette, Element, Point};

    pub(crate) const ROW_HEIGHT: f32 = 26.0;
    pub(crate) const INSET: f32 = 4.0;
    pub(crate) const WIDTH: f32 = 352.0;
    pub(crate) const MAX_HEIGHT: f32 = 384.0;
    pub(crate) const LABEL_FONT_SIZE: f32 = 13.0;
    pub(crate) const SHORTCUT_FONT_SIZE: f32 = 12.0;
    pub(crate) const ROW_RADIUS: f32 = crate::components::CONTROL_RADIUS;
    pub(crate) const HORIZONTAL_PADDING: f32 = 8.0;
    pub(crate) const GAP: f32 = 8.0;
    pub(crate) const SEPARATOR_HEIGHT: f32 = 9.0;
    pub(crate) const SCROLLBAR_GUTTER: f32 = 14.0;
    pub(crate) const HEADER_HEIGHT: f32 = 24.0;

    pub(crate) fn surface(
        content: Element,
        palette: ComponentPalette,
        width: f32,
        height: Option<f32>,
    ) -> Element {
        Element::panel(content)
            .padding(INSET)
            .background(palette.surface)
            .border(palette.border, 1.0)
            .radius(crate::components::FLOATING_SURFACE_RADIUS)
            .shadow(palette.shadow, Point::new(0.0, 6.0), 18.0, 0.0)
            .size(Some(width), height)
            .clip_children()
    }
}

/// A full-width menu action with an optional icon, annotation or shortcut.
pub struct MenuItem {
    action_id: String,
    label: String,
    icon: Option<Icon>,
    trailing: Option<String>,
    shortcut: Option<String>,
    width: f32,
    height: f32,
    palette: ComponentPalette,
    destructive: Option<Color>,
}

impl MenuItem {
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            trailing: None,
            shortcut: None,
            width: 220.0,
            height: action_style::ROW_HEIGHT,
            palette,
            destructive: None,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn trailing(mut self, trailing: impl Into<String>) -> Self {
        self.trailing = Some(trailing.into());
        self.shortcut = None;
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self.trailing = None;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(40.0);
        }
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(24.0);
        }
        self
    }

    pub fn destructive(mut self, ink: Color) -> Self {
        self.destructive = Some(ink);
        self
    }
}

impl Component for MenuItem {
    fn render(self) -> Element {
        let ink = self.destructive.unwrap_or(self.palette.text);
        let content_height = self.height;
        let mut children = Vec::with_capacity(3);
        if let Some(icon) = self.icon {
            children.push(
                Element::icon(icon)
                    .foreground(ink)
                    .size(Some(ICON_SIZE), Some(content_height)),
            );
        }
        children.push(
            Element::truncated_text(self.label)
                .foreground(ink)
                .font_family(FontFamily::Sans)
                .font_size(action_style::LABEL_FONT_SIZE)
                .size(None, Some(content_height))
                .flex(1.0),
        );
        if let Some(shortcut) = self.shortcut {
            let icon_budget = if self.icon.is_some() {
                ICON_SIZE + action_style::GAP
            } else {
                0.0
            };
            let hint_budget = (self.width
                - action_style::HORIZONTAL_PADDING * 2.0
                - icon_budget
                - action_style::GAP
                - 40.0)
                .max(0.0);
            children.push(
                Kbd::new(shortcut, self.palette)
                    .max_width(hint_budget)
                    .render()
                    .size(None, Some(content_height)),
            );
        } else if let Some(trailing) = self.trailing {
            children.push(
                Element::truncated_text(trailing)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(11.0)
                    .size(None, Some(content_height)),
            );
        }
        self.palette
            .interactive_surface(Element::clickable(
                self.action_id,
                Element::row(vec![
                    Element::spacer().size(Some(action_style::HORIZONTAL_PADDING), None),
                    Element::row(children).gap(action_style::GAP).flex(1.0),
                    Element::spacer().size(Some(action_style::HORIZONTAL_PADDING), None),
                ]),
            ))
            .foreground(ink)
            .action_menu_item()
            .transition_ms(0)
            .radius(action_style::ROW_RADIUS)
            .clip_children()
            .size(Some(self.width), Some(self.height))
    }
}

/// A quiet separator that retains the menu's inner horizontal inset.
pub struct MenuSeparator {
    width: f32,
    color: Color,
}

impl MenuSeparator {
    pub fn new(width: f32, color: Color) -> Self {
        Self { width, color }
    }
}

impl Component for MenuSeparator {
    fn render(self) -> Element {
        Element::panel(Element::spacer().background(self.color))
            .padding((action_style::SEPARATOR_HEIGHT - 1.0) / 2.0)
            .size(Some(self.width), Some(action_style::SEPARATOR_HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use crate::{DrawCommand, Point, Size, UiEvent, UiTree};

    use super::*;

    #[test]
    fn whole_menu_item_is_one_hover_and_click_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            MenuItem::new("rename", "Renombrar", palette)
                .icon(Icon::Edit)
                .shortcut("Ctrl+R")
                .width(236.0)
                .render(),
        );
        tree.set_reduced_motion(true);
        let initial = tree.layout(Size::new(236.0, 36.0));
        for key in [
            if cfg!(target_os = "macos") {
                "⌃"
            } else {
                "Ctrl"
            },
            "R",
        ] {
            assert!(initial.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_family: FontFamily::Sans, .. }
                    if text == key
            )));
        }
        for x in [8.0, 42.0, 205.0] {
            tree.handle_event(UiEvent::PointerMove(Point::new(x, 18.0)));
            let frame = tree.layout(Size::new(236.0, 36.0));
            assert_eq!(frame.hit_test(Point::new(x, 18.0)), Some("rename"));
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, rect, .. }
                    if *color == palette.hover && rect.width == 236.0
            )));
        }
    }

    #[test]
    fn narrow_menu_retains_its_action_label_without_partial_shortcuts() {
        let frame = UiTree::new(
            MenuItem::new("rename", "Renombrar", ComponentPalette::neutral_dark())
                .icon(Icon::Edit)
                .shortcut("Ctrl+Shift+R")
                .width(120.0)
                .render(),
        )
        .layout(Size::new(120.0, 36.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.starts_with("Renom"))));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if ["Ctrl", "Shift", "R"].contains(&text.as_str()))));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "rename");
    }

    #[test]
    fn compact_menu_shortcuts_are_visible_and_centered_inside_the_action() {
        let palette = ComponentPalette::neutral_dark();
        for height in [24.0, 28.0, 32.0, 36.0] {
            let frame = UiTree::new(
                MenuItem::new("rename", "Renombrar", palette)
                    .shortcut("Ctrl+R")
                    .height(height)
                    .render(),
            )
            .layout(Size::new(220.0, height));
            let caps: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *color == crate::shortcut::cap_fill(palette.text)
                        && *radius == crate::shortcut::RADIUS =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(caps.len(), 2, "both shortcut keys are visible");
            for cap in caps {
                assert_eq!(cap.height, crate::shortcut::HEIGHT);
                assert!((cap.y - (height - cap.height) / 2.0).abs() < 0.01);
            }
            assert_eq!(frame.regions.len(), 1);
            assert_eq!(frame.regions[0].rect.height, height);
        }
    }

    #[test]
    fn hovering_one_menu_item_does_not_highlight_its_sibling() {
        let palette = ComponentPalette::neutral_dark();
        let items = Element::column(vec![
            MenuItem::new("rename", "Renombrar", palette)
                .icon(Icon::Edit)
                .width(236.0)
                .render(),
            MenuItem::new("archive", "Archivar", palette)
                .icon(Icon::Archive)
                .width(236.0)
                .render(),
        ]);
        let mut tree = UiTree::new(items);
        tree.set_reduced_motion(true);
        tree.layout(Size::new(236.0, 52.0));
        for (y, id) in [(13.0, "rename"), (39.0, "archive")] {
            tree.handle_event(UiEvent::PointerMove(Point::new(42.0, y)));
            let frame = tree.layout(Size::new(236.0, 52.0));
            assert_eq!(frame.hit_test(Point::new(42.0, y)), Some(id));
            assert_eq!(
                frame
                    .commands
                    .iter()
                    .filter(|command| matches!(
                        command,
                        DrawCommand::FillRect { color, rect, .. }
                            if *color == palette.hover && rect.width == 236.0
                    ))
                    .count(),
                1,
            );
        }
    }
}
