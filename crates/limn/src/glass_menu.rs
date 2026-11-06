//! Context actions on a pane of glass that grows out of its trigger.

use crate::{
    Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, Element, ElementKind,
    GlassMaterial, MenuMotion, Point, PopoverPlacement, Rect,
};

/// A controlled action menu with an optical surface. Placement, focus,
/// keyboard navigation and dismissal use [`DropdownMenu`]. The source bounds
/// are in window coordinates, obtained from the host's laid-out trigger.
/// Use the row bounds for a menu opened by a held row.
pub struct GlassMenu {
    menu: DropdownMenu,
    material: GlassMaterial,
    origin: Rect,
}

/// BitChord's menu opens on a stiff, critically damped spring, about 200 ms,
/// and closes in 170 ms.
const OPEN_MS: u16 = 200;
const CLOSE_MS: u16 = 170;

impl GlassMenu {
    /// A menu anchored to the end of `anchor`. All generated IDs derive from
    /// `id`; dismissal reports `{id}:dismiss`. The host handles each entry's
    /// supplied action ID and rebuilds `open`.
    pub fn new(
        id: impl Into<String>,
        anchor: Element,
        origin: Rect,
        entries: Vec<DropdownMenuEntry>,
        open: bool,
        material: GlassMaterial,
        palette: ComponentPalette,
    ) -> Self {
        let id = id.into();
        Self {
            menu: DropdownMenu::new(anchor, entries, open, format!("{id}:dismiss"), palette)
                .placement(PopoverPlacement::BottomEnd)
                .motion(MenuMotion {
                    open_ms: OPEN_MS,
                    item_ms: crate::tokens().motion.hover_ms,
                })
                .close_animation_ms(CLOSE_MS),
            material,
            origin,
        }
    }

    /// Surface width in logical pixels. The underlying menu sanitizes values
    /// and keeps all actions scrollable when the viewport is smaller.
    pub fn width(mut self, width: f32) -> Self {
        self.menu = self.menu.width(width);
        self
    }

    /// Open beside a held row or a secondary-click position in window
    /// coordinates, retaining `origin` as the return destination on close.
    pub fn at(mut self, point: Point) -> Self {
        self.menu = self.menu.at(point);
        self
    }

    /// Surface placement relative to the anchor, below its end by default.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.menu = self.menu.placement(placement);
        self
    }
}

impl Component for GlassMenu {
    fn render(self) -> Element {
        let mut menu = self.menu.render();
        if let ElementKind::Popover { floating, .. } = &mut menu.kind {
            let mut surface = (**floating).clone();
            surface.style.background = None;
            surface.style.border = None;
            **floating = surface.glass(self.material);
        }
        // As BitChord's: all of it, rows too, grows out of its corner nearest the trigger.
        let centre = Point::new(self.origin.x + self.origin.width / 2.0, self.origin.y + self.origin.height / 2.0);
        menu.popover_grow_toward(centre)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, DropdownMenuItem, Size, UiTree};

    fn view(open: bool) -> Element {
        let palette = ComponentPalette::neutral_dark();
        GlassMenu::new(
            "song",
            Element::button("more", "More"),
            Rect::new(200.0, 20.0, 44.0, 44.0),
            vec![DropdownMenuItem::new("queue", "Add to queue").into()],
            open,
            GlassMaterial::new(palette.surface),
            palette,
        )
        .render()
    }

    #[test]
    fn closed_menu_keeps_only_its_trigger() {
        let mut tree = UiTree::new(view(false));
        let frame = tree.layout(Size::new(400.0, 500.0));
        assert!(frame.regions.iter().any(|region| region.id == "more"));
        assert!(!frame.regions.iter().any(|region| region.id == "queue"));
    }

    #[test]
    fn held_row_requests_the_same_host_controlled_menu() {
        let palette = ComponentPalette::neutral_dark();
        let row = crate::ContextMenu::new(
            "song:context",
            Element::button("song", "Song"),
            vec![],
            palette,
        )
        .open_action("song:open")
        .render();
        let mut tree = UiTree::new(row);
        let frame = tree.layout(Size::new(400.0, 500.0));
        let bounds = frame.regions.iter().find(|r| r.id == "song").unwrap().rect;
        let actions = tree.handle_event(crate::UiEvent::ContextMenu {
            position: Point::new(
                bounds.x + bounds.width / 2.0,
                bounds.y + bounds.height / 2.0,
            ),
        });
        assert_eq!(
            actions,
            vec![crate::UiAction::Clicked {
                id: "song:open".into()
            }]
        );
    }

    #[test]
    fn menu_action_and_escape_keep_the_dropdown_contract() {
        let mut tree = UiTree::new(view(true));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 500.0));
        let rect = frame
            .regions
            .iter()
            .find(|region| region.id == "queue")
            .unwrap()
            .rect;
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(crate::UiEvent::PointerMove(point));
        tree.handle_event(crate::UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(crate::UiEvent::PointerUp(point)),
            vec![crate::UiAction::Clicked { id: "queue".into() }]
        );
        assert_eq!(
            tree.handle_event(crate::UiEvent::Key {
                key: crate::Key::Escape,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![crate::UiAction::Clicked {
                id: "song:dismiss".into()
            }]
        );
    }

    #[test]
    fn open_menu_uses_optical_surface_and_retains_action_ids() {
        let mut tree = UiTree::new(view(true));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 500.0));
        assert!(frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Glass { .. })));
        assert!(frame.regions.iter().any(|region| region.id == "queue"));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "song:dismiss"));
    }
}
