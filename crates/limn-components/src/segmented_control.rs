//! Tabs in one rounded track, as a choice among a few options.

use crate::tabs::{TAB_GAP, TAB_TRACK_PADDING};
use crate::{Color, Component, ComponentPalette, Element, Tab, Tabs};

/// Tabs in one rounded track. Each item remains a normal tab action, so the
/// existing keyboard navigation and focus model also applies to segmented
/// selectors. The caller controls the selected item and every visual color.
pub struct SegmentedControl {
    items: Vec<Tab>,
    palette: ComponentPalette,
    height: f32,
    selection_border: Option<Color>,
    radius: Option<f32>,
}

impl SegmentedControl {
    /// A control of these segments, in order. Each is a [`Tab`].
    pub fn new(items: Vec<Tab>, palette: ComponentPalette) -> Self {
        Self {
            items,
            palette,
            height: 28.0,
            selection_border: None,
            radius: None,
        }
    }

    /// Set the selection's corner radius in logical pixels; half its height makes a pill.
    /// Non-finite values are ignored and negative radii become zero.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() { self.radius = Some(radius.max(0.0)); }
        self
    }

    /// Outline the chosen segment one pixel wide in this color: a white chip
    /// on a pale track, as a desktop settings page draws a theme picker.
    pub fn selection_border(mut self, color: Color) -> Self {
        self.selection_border = Some(color);
        self
    }

    /// Set the height, in logical pixels, never below 28. A height that is
    /// not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(28.0);
        }
        self
    }
}

impl Component for SegmentedControl {
    fn render(self) -> Element {
        let inner_width = self.items.iter().map(|item| item.width).sum::<f32>()
            + TAB_GAP * self.items.len().saturating_sub(1) as f32;
        let mut tabs = Tabs::new(self.items, self.palette).height(self.height - TAB_TRACK_PADDING * 2.0);
        tabs.selection_border = self.selection_border;
        tabs.radius = self.radius;
        tabs.render()
            .size(
                Some(inner_width + TAB_TRACK_PADDING * 2.0),
                Some(self.height),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{surface_radius, DrawCommand, Icon, Role, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn tabs_and_segments_are_tabs_and_the_chosen_one_says_so() {
        let palette = ComponentPalette::neutral_dark();
        let items = || {
            vec![
                Tab::new("files", "Files", 80.0).active(true),
                Tab::new("search", "Search", 80.0).icon(Icon::Search),
                Tab::new("git", "Git", 80.0).disabled(),
            ]
        };
        for strip in [Tabs::new(items(), palette).render(), SegmentedControl::new(items(), palette).render()] {
            let row = Element::row(vec![strip, Element::spacer().flex(1.0)]);
            let nodes = UiTree::new(row).layout(Size::new(480.0, 60.0)).access;
            let said: Vec<_> = nodes
                .iter()
                .map(|node| (node.id.as_str(), node.role, node.name.as_str(), node.selected, node.disabled))
                .collect();
            assert_eq!(
                said,
                [
                    ("files", Role::Tab, "Files", Some(true), false),
                    ("search", Role::Tab, "Search", Some(false), false),
                    ("git", Role::Tab, "Git", Some(false), true),
                ]
            );
            assert!(nodes[0].actionable && !nodes[2].actionable);
        }
    }


    #[test]
    fn segmented_control_keeps_the_tab_keyboard_contract_and_fixed_track() {
        let palette = ComponentPalette::neutral_dark();
        let segment = SegmentedControl::new(
            vec![
                Tab::new("group", "Grupo", 70.0).icon(Icon::Hash),
                Tab::new("project", "Proyecto", 78.0)
                    .icon(Icon::Folder)
                    .active(true),
            ],
            palette,
        )
        .render();
        assert_eq!(segment.style.radius, surface_radius());
        assert_eq!(segment.style.width, Some(154.0));
        let mut tree = UiTree::new(segment);
        let frame = tree.layout(Size::new(154.0, 28.0));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, radius }
                if *color == palette.surface && rect.width == 154.0 && rect.height == 28.0 && *radius == surface_radius()
        )));
        assert!(tree.focus("group"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::ArrowRight,
                shift: false,
                ctrl: false,
                alt: false
            }),
            vec![UiAction::Clicked {
                id: "project".into()
            }]
        );
    }


    #[test]
    fn icon_segments_can_be_narrow_and_the_chosen_one_outlined() {
        let palette = ComponentPalette::neutral_dark();
        let edge = Color::rgb(231, 231, 231);
        let icons = || {
            vec![
                Tab::new("system", "", 26.0).icon(Icon::Hash).active(true),
                Tab::new("light", "", 26.0).icon(Icon::Folder),
                Tab::new("dark", "", 10.0).icon(Icon::Code),
            ]
        };
        // Three 26 px segments, the last kept at the 24 px floor, on a 2 px track.
        let segment = SegmentedControl::new(icons(), palette).selection_border(edge).render();
        assert_eq!(segment.style.width, Some(26.0 + 26.0 + 24.0 + 2.0 * TAB_GAP + 2.0 * TAB_TRACK_PADDING));
        let mut tree = UiTree::new(segment);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 28.0));
        let chip = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == palette.selected => Some(*rect),
            _ => None,
        });
        let outline = frame.commands.iter().find_map(|command| match command {
            DrawCommand::StrokeRect { rect, color, width, .. } if *color == edge => Some((*rect, *width)),
            _ => None,
        });
        assert_eq!(chip.map(|rect| rect.width), Some(26.0));
        assert_eq!(outline, chip.map(|rect| (rect, 1.0)));
        // Without it, no outline.
        let mut plain = UiTree::new(SegmentedControl::new(icons(), palette).render());
        plain.set_reduced_motion(true);
        let plain = plain.layout(Size::new(200.0, 28.0));
        assert!(!plain.commands.iter().any(|command| matches!(command, DrawCommand::StrokeRect { color, .. } if *color == edge)));
    }
}
