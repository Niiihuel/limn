//! The grip on an item the pointer drags to move it among its siblings.

use crate::{Access, Component, ComponentPalette, Cursor, Element, Icon, Role};

const DEFAULT_SIDE: f32 = 24.0;
const MIN_SIDE: f32 = 16.0;

/// Two columns of dots ([`Icon::DragHandle`]) on a card or a row, the grip
/// the pointer takes it by to move it among its siblings. It is muted at rest
/// and fills with `palette.hover` under the pointer. The pointer is the
/// platform's own open hand over it ([`Cursor::Grab`]) and its closed hand
/// while the host says a drag from it is under way ([`Cursor::Grabbing`]),
/// when the handle also fills with `palette.selected`.
///
/// It is controlled, as [`crate::ResizeHandle`] is: it reports `DragStarted`,
/// `DragMoved` and `Dropped` (or `DragCancelled`) on its ID, and nothing
/// follows the pointer: the host moves the item. Make each sibling, or a
/// control of each such as its header, a [`Element::drop_target`]: `DragMoved`
/// names the one under the pointer in `over`, so the host can put the item in
/// its place as the pointer goes, and with [`Element::animate_move`] on each
/// item the others glide aside. A press released without moving, or Enter on
/// the focused handle, is a `Clicked` the host may ignore.
///
/// [`on_hover`](Self::on_hover) shows it only while the pointer is over the
/// [`Element::hover_region`] around it (its item), while it has the focus, or
/// while it is dragged; it keeps its place, so nothing moves when it appears.
pub struct DragHandle {
    id: String,
    side: f32,
    dragging: bool,
    on_hover: bool,
    enabled: bool,
    label: Option<String>,
    palette: ComponentPalette,
}

impl DragHandle {
    /// A handle reporting its drag on `id`.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            side: DEFAULT_SIDE,
            dragging: false,
            on_hover: false,
            enabled: true,
            label: None,
            palette,
        }
    }

    /// Its side, which is also its pointer target: 24 unless set and never
    /// under 16. Values that are not finite are ignored.
    pub fn size(mut self, side: f32) -> Self {
        if side.is_finite() {
            self.side = side.max(MIN_SIDE);
        }
        self
    }

    /// The host says a drag from this handle is under way: the pointer is a
    /// closed hand and the handle stays filled and in sight until the drop.
    pub fn dragging(mut self, dragging: bool) -> Self {
        self.dragging = dragging;
        self
    }

    /// Show it only while the pointer is over the hover region around it,
    /// while it has the focus, or while it is dragged.
    pub fn on_hover(mut self, on_hover: bool) -> Self {
        self.on_hover = on_hover;
        self
    }

    /// What the platform calls the handle, such as "Move Context". It shows
    /// no text, so without this it has no name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.label = Some(name.into());
        self
    }

    /// A disabled handle keeps its place and cannot be dragged or focused. It
    /// draws nothing and leaves the pointer its shape.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for DragHandle {
    fn render(self) -> Element {
        let side = Some(self.side);
        let said = Access { name: self.label, ..Access::new(Role::Button) };
        if !self.enabled {
            return Element::spacer().size(side, side).accessible(said.id(self.id).disabled(true));
        }
        let reveal = format!("{}:reveal", self.id);
        let button = Element::icon_button(self.id, Icon::DragHandle, "").accessible(said);
        let mut handle = self
            .palette
            .interactive_surface(button)
            .foreground(if self.dragging { self.palette.text } else { self.palette.muted })
            .draggable_in_place()
            .cursor(if self.dragging { Cursor::Grabbing } else { Cursor::Grab })
            .size(side, side);
        if self.dragging {
            handle = handle.background(self.palette.selected);
        } else if self.on_hover {
            handle = handle.reveal_on_hover(reveal);
        }
        handle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point, Size, UiAction, UiEvent, UiTree};

    const WINDOW: Size = Size::new(200.0, 200.0);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    /// Two cards in a column, 100 tall, each a header (its drop target) with
    /// a handle at its end, in a window 200 wide.
    fn cards(handle: impl Fn(&str) -> DragHandle) -> UiTree {
        let card = |id: &str| {
            Element::column(vec![
                Element::clickable(
                    format!("{id}:head"),
                    Element::row(vec![Element::spacer().flex(1.0), handle(id).render()]),
                )
                .drop_target(),
                Element::spacer().size(None, Some(76.0)),
            ])
            .size(None, Some(100.0))
            .hover_region()
        };
        let mut tree = UiTree::new(Element::column(vec![card("a"), card("b")]));
        tree.set_reduced_motion(true);
        tree.layout(WINDOW);
        tree
    }

    fn handle(id: &str) -> DragHandle {
        DragHandle::new(format!("{id}:drag"), palette()).accessibility_label(format!("Move {id}"))
    }

    #[test]
    fn a_handle_is_a_button_named_by_the_host_enabled_or_not() {
        for enabled in [true, false] {
            let mut tree = cards(|id| handle(id).enabled(enabled));
            let frame = tree.layout(WINDOW);
            let node = frame.access.iter().find(|node| node.id == "a:drag").cloned().unwrap();
            assert_eq!((node.role, node.name.as_str()), (Role::Button, "Move a"));
            assert_eq!((node.disabled, node.actionable), (!enabled, enabled));
            assert_eq!((node.rect.width, node.rect.height), (24.0, 24.0));
        }
    }

    #[test]
    fn the_pointer_is_an_open_hand_over_it_and_a_closed_one_while_it_drags() {
        let mut tree = cards(handle);
        tree.handle_event(UiEvent::PointerMove(Point::new(188.0, 12.0)));
        assert_eq!(tree.cursor(), Some(Cursor::Grab));
        tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 60.0)));
        assert_eq!(tree.cursor(), None, "only over the handle");
        let mut tree = cards(|id| handle(id).dragging(id == "a"));
        tree.handle_event(UiEvent::PointerMove(Point::new(188.0, 12.0)));
        assert_eq!(tree.cursor(), Some(Cursor::Grabbing));
    }

    #[test]
    fn a_drag_names_the_drop_target_under_the_pointer_and_nothing_follows_it() {
        let mut tree = cards(handle);
        let mut actions = tree.handle_event(UiEvent::PointerDown(Point::new(188.0, 12.0)));
        actions.extend(tree.handle_event(UiEvent::PointerMove(Point::new(188.0, 40.0))));
        actions.extend(tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 112.0))));
        assert!(actions.iter().any(|a| matches!(a, UiAction::DragStarted { id, .. } if id == "a:drag")));
        assert!(
            actions.iter().any(|a| matches!(a, UiAction::DragMoved { id, over: Some(over), .. }
                if id == "a:drag" && over == "b:head")),
            "{actions:?}"
        );
        let handles = |tree: &mut UiTree| {
            let frame = tree.layout(WINDOW);
            frame.commands.iter().filter(|c| matches!(c, DrawCommand::Icon { icon: Icon::DragHandle, .. })).count()
        };
        assert_eq!(handles(&mut tree), 2, "no copy follows the pointer");
        let dropped = tree.handle_event(UiEvent::PointerUp(Point::new(150.0, 112.0)));
        assert!(dropped.iter().any(|a| matches!(a, UiAction::Dropped { id, target: Some(target), .. }
            if id == "a:drag" && target == "b:head")));
    }

    #[test]
    fn on_hover_it_shows_over_its_item_only_and_always_while_dragged() {
        let painted = |tree: &mut UiTree| {
            let frame = tree.layout(WINDOW);
            // A hidden one is still drawn, see-through.
            let shown = |c: &&DrawCommand| matches!(c, DrawCommand::Icon { icon: Icon::DragHandle, tint, .. } if tint.a > 0);
            frame.commands.iter().filter(shown).count()
        };
        let mut tree = cards(|id| handle(id).on_hover(true));
        tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 150.0)));
        assert_eq!(painted(&mut tree), 1, "the second card's, under the pointer");
        let mut tree = cards(|id| handle(id).on_hover(true).dragging(id == "a"));
        tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 150.0)));
        assert_eq!(painted(&mut tree), 2, "the dragged one stays");
    }
}
