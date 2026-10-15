//! A controlled bottom sheet built on the modal's shared overlay behavior.

use crate::{Component, ComponentPalette, Element, Modal, Point, UiAction};

/// A modal surface that meets the safe viewport's bottom edge. The header,
/// close control and optional footer stay visible while only the body scrolls.
/// The host owns `open` and handles the dismissal action, as for [`Modal`].
pub struct BottomSheet {
    modal: Modal,
    max_height: f32,
    safe_area_bottom: f32,
}

impl BottomSheet {
    /// A sheet over `base`, or `base` alone while closed. Its preferred width
    /// is 560 logical pixels, bounded by the viewport; its total height is at
    /// most 600. The backdrop dismisses by default. All close controls, Escape
    /// and the backdrop report `dismiss_id`. Use [`BottomSheetDrag`] to also
    /// interpret its `{dismiss_id}:drag` handle's controlled drag actions.
    pub fn new(
        base: Element,
        open: bool,
        title: impl Into<String>,
        description: impl Into<String>,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        let dismiss_id = dismiss_id.into();
        Self {
            modal: Modal::new(base, open, title, description, &dismiss_id, palette)
                .width(560.0)
                .initial_focus(format!("{dismiss_id}:close"))
                .backdrop_dismiss(true),
            max_height: 600.0,
            safe_area_bottom: 0.0,
        }
    }

    /// Preferred maximum width, at least 240; phone widths still take priority.
    /// A value that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        self.modal = self.modal.width(width);
        self
    }

    /// Cap the whole sheet, including its header, footer and safe-area space.
    /// The viewport may cap it further. Finite values are kept at least 240.
    pub fn max_height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.max_height = height.max(240.0);
        }
        self
    }

    /// Reserve this much extra space below the footer/body for a system home
    /// indicator when the host uses the full screen as its viewport. Leave it
    /// at zero if the host's viewport already excludes that safe area.
    /// Negative values become zero; a value that is not finite is ignored.
    pub fn safe_area_bottom(mut self, inset: f32) -> Self {
        if inset.is_finite() {
            self.safe_area_bottom = inset.max(0.0);
        }
        self
    }

    /// The only scrolling part of the sheet. Its scroll ID is
    /// `modal_body:{dismiss_id}`, shared with the modal composition.
    pub fn body(mut self, body: Element) -> Self {
        self.modal = self.modal.body(body);
        self
    }

    /// Fixed content below the body, such as a row of actions.
    pub fn footer(mut self, footer: Element) -> Self {
        self.modal = self.modal.footer(footer);
        self
    }

    /// The control inside the sheet that receives focus when it opens. The
    /// default is the visible close control, which has a 44-point target.
    pub fn initial_focus(mut self, id: impl Into<String>) -> Self {
        self.modal = self.modal.initial_focus(id);
        self
    }

    /// The stable control to focus when the sheet closes. Without this, the
    /// shared modal behavior restores whichever control was previously focused.
    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.modal = self.modal.return_focus(id);
        self
    }

    /// Whether clicking the scrim requests dismissal; true by default. The
    /// scrim blocks controls underneath in either case.
    pub fn backdrop_dismiss(mut self, enabled: bool) -> Self {
        self.modal = self.modal.backdrop_dismiss(enabled);
        self
    }

    /// Entrance duration in milliseconds. The shared Popover transition is
    /// interruptible and follows [`crate::UiTree::set_reduced_motion`].
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.modal = self.modal.animation_ms(duration);
        self
    }
}

impl Component for BottomSheet {
    fn render(self) -> Element {
        self.modal
            .bottom_sheet(self.max_height, self.safe_area_bottom)
            .render()
    }
}

/// Controlled dismissal for a sheet's drag handle. Keep one next to the
/// host's open state and pass it each [`UiAction`]. A `true` result asks the
/// host to close the sheet. Body scrolling never starts a handle drag.
///
/// The handle uses the existing in-place drag primitive: the sheet stays in
/// its layout while held, then the shared Popover transition closes it. No
/// detached drag preview or second overlay state machine is created.
#[derive(Debug)]
pub struct BottomSheetDrag {
    dismiss_id: String,
    handle_id: String,
    origin: Option<Point>,
}

impl BottomSheetDrag {
    /// Interpret the handle for the sheet with this same `dismiss_id`.
    pub fn new(dismiss_id: impl Into<String>) -> Self {
        let dismiss_id = dismiss_id.into();
        Self {
            handle_id: format!("{dismiss_id}:drag"),
            dismiss_id,
            origin: None,
        }
    }

    /// Recognize close clicks and a released downward handle drag of at least
    /// 64 logical pixels, with vertical travel at least the horizontal travel.
    /// Short/upward/sideways drags and cancellation keep the sheet open.
    pub fn handle_action(&mut self, action: &UiAction) -> bool {
        match action {
            UiAction::Clicked { id } if id == &self.dismiss_id => {
                self.reset();
                true
            }
            UiAction::DragStarted { id, position } if id == &self.handle_id => {
                self.origin = Some(*position);
                false
            }
            UiAction::Dropped { id, position, .. } if id == &self.handle_id => {
                self.origin.take().is_some_and(|origin| {
                    let down = position.y - origin.y;
                    down >= 64.0 && down >= (position.x - origin.x).abs()
                })
            }
            UiAction::DragCancelled { id } if id == &self.handle_id => {
                self.reset();
                false
            }
            _ => false,
        }
    }

    /// Forget a held drag when the host closes or replaces the sheet through
    /// another path. Close actions recognized by `handle_action` reset it too.
    pub fn reset(&mut self) {
        self.origin = None;
    }
}

#[cfg(test)]
#[path = "bottom_sheet_tests.rs"]
mod tests;
