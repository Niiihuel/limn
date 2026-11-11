//! A floating surface anchored to any element.

use crate::{surface_radius, Access, Component, ComponentPalette, Element, Point, PopoverPlacement};

/// A floating slot anchored to any element. The child may itself be a
/// dropdown, command list, form, or another application-specific component.
/// While open, Tab stays among visible controls in the floating surface;
/// Escape or backdrop dismissal returns focus to the first focusable anchor.
pub struct Popover {
    /// The element it opens from, told to the platform as expanded or not. It
    /// is all that is drawn while the popover is closed.
    pub anchor: Element,
    /// What the floating surface holds, inside 6 pixels of padding.
    pub content: Element,
    /// Whether the surface shows. The host owns it.
    pub open: bool,
    /// Reported in `UiAction::Clicked` when the popover is dismissed, for the
    /// host to close it.
    pub dismiss_id: String,
    /// The side of the anchor it prefers. `BottomStart` by default.
    pub placement: PopoverPlacement,
    /// Width of the surface in logical pixels, 240 by default. The `width`
    /// builder keeps it at 96 or more.
    pub width: f32,
    /// `raised` fills the surface, `border` outlines it and `shadow` falls under it.
    pub palette: ComponentPalette,
    /// Duration of the entrance in milliseconds. The default is the `enter_ms`
    /// token.
    pub animation_ms: u16,
    /// Duration of the exit in milliseconds. `None`, the default, keeps the
    /// engine's own.
    pub close_animation_ms: Option<u16>,
    /// The scale the entrance starts at. 1.0, the default, does not scale; the
    /// `open_scale` builder keeps it within 0.5..=1.25.
    pub open_scale: f32,
    /// Names the view the content shows: a change of key while the popover stays
    /// open is animated. `None` by default.
    pub view_key: Option<String>,
}

impl Popover {
    /// A popover on `anchor` that shows `content` while `open` is true.
    /// Escape reports `UiAction::Clicked` with `dismiss_id`, for the host to
    /// close it. It opens under the start of the anchor, 240 pixels wide.
    pub fn new(
        anchor: Element,
        content: Element,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            content,
            open,
            dismiss_id: dismiss_id.into(),
            placement: PopoverPlacement::BottomStart,
            width: 240.0,
            palette,
            animation_ms: crate::tokens().motion.enter_ms,
            close_animation_ms: None,
            open_scale: 1.0,
            view_key: None,
        }
    }

    /// The side of the anchor it prefers. The default is `BottomStart`.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Width of the floating surface in logical pixels, at least 96. A width
    /// that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// Duration of the entrance, in milliseconds. The default is the enter
    /// duration of the motion tokens.
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    /// Duration of the exit, in milliseconds, in place of the default short
    /// fade.
    pub fn close_animation_ms(mut self, duration: u16) -> Self {
        self.close_animation_ms = Some(duration);
        self
    }

    /// The scale the entrance starts at, clamped to 0.5..=1.25. The default,
    /// 1.0, does not scale; a value that is not finite is ignored.
    pub fn open_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.open_scale = scale.clamp(0.5, 1.25);
        }
        self
    }

    /// Name the view the content shows, so that a change of key while the
    /// popover stays open is animated.
    pub fn view_key(mut self, key: impl Into<String>) -> Self {
        self.view_key = Some(key.into());
        self
    }
}

impl Component for Popover {
    fn render(self) -> Element {
        // What the anchor opens is open or it is not; said of the anchor
        // alone, so it cannot reach a control of the content.
        let anchor = self.anchor.accessible(Access::default().expanded(self.open));
        if !self.open {
            return anchor;
        }
        let floating = Element::panel(self.content)
            .padding(6.0)
            .size(Some(self.width), None)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(surface_radius())
            .shadow(self.palette.shadow, Point::new(0.0, 9.0), 22.0, 0.0);
        let mut popover = Element::popover(anchor, floating, self.dismiss_id, self.placement)
            .popover_animation_ms(self.animation_ms)
            .popover_open_scale(self.open_scale);
        if let Some(duration) = self.close_animation_ms {
            popover = popover.popover_close_animation_ms(duration);
        }
        if let Some(key) = self.view_key {
            popover = popover.popover_view_key(key);
        }
        popover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Button, Role, Size, UiTree};

    #[test]
    fn a_popover_tells_its_anchor_whether_it_is_open() {
        let palette = ComponentPalette::neutral_dark();
        for open in [false, true] {
            let popover = Popover::new(
                Element::panel(Button::new("more", "More", palette).render()),
                Button::new("inside", "Inside", palette).render(),
                open,
                "dismiss",
                palette,
            );
            let nodes = UiTree::new(Element::column(vec![popover.render(), Element::spacer().flex(1.0)]))
                .layout(Size::new(480.0, 320.0))
                .access;
            let anchor = nodes.iter().find(|node| node.id == "more").expect("the anchor");
            assert_eq!((anchor.role, anchor.name.as_str(), anchor.expanded), (Role::Button, "More", Some(open)));
            // What is said of the anchor is not said of what it opens.
            let inside = nodes.iter().find(|node| node.id == "inside");
            assert_eq!(inside.map(|node| node.expanded), open.then_some(None));
        }
    }
}
