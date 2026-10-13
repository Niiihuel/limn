//! A button that copies something. Its icon turns into a check while the host
//! says the copy was just made.

use crate::{
    ButtonVariant, Component, ComponentPalette, ControlSize, Element, Icon, IconButton,
    MessageActionDone,
};

/// The time a [`Clipboard`] shows as copied, kept by the host beside its
/// state: mark it on the click and pass `shows(id, now)` to `copied`.
pub type ClipboardCopied = MessageActionDone;

/// A ghost icon button for copying a link, a key or a path. The host owns the
/// clipboard and the state: on `UiAction::Clicked { id }` it copies the value,
/// keeps a [`ClipboardCopied`] for the check and announces it with a `Toast`.
pub struct Clipboard {
    id: String,
    title: String,
    palette: ComponentPalette,
    icon: Icon,
    label: Option<String>,
    copied: bool,
    size: ControlSize,
    variant: ButtonVariant,
}

impl Clipboard {
    /// `title` says what is copied; it is the button's hint.
    pub fn new(id: impl Into<String>, title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            palette,
            icon: Icon::Copy,
            label: None,
            copied: false,
            size: ControlSize::Compact,
            variant: ButtonVariant::Ghost,
        }
    }

    /// Another icon at rest, such as a link's.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// Text after the icon, for a button that says what it copies.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show a check in place of the icon: the copy was just made.
    pub fn copied(mut self, copied: bool) -> Self {
        self.copied = copied;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Component for Clipboard {
    fn render(self) -> Element {
        let icon = if self.copied { Icon::Check } else { self.icon };
        let mut button = IconButton::new(self.id, icon, self.palette)
            .variant(self.variant)
            .size(self.size)
            .tooltip(self.title);
        if let Some(label) = self.label {
            button = button.label(label);
        }
        if self.variant == ButtonVariant::Ghost {
            // Muted until it is used or just was, like a message's actions.
            let ink = if self.copied { self.palette.text } else { self.palette.muted };
            button = button.ink(ink).hover_ink(self.palette.text);
        }
        button.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Point, Size, UiAction, UiEvent, UiTree};
    use std::time::{Duration, Instant};

    const SIZE: Size = Size::new(200.0, 60.0);

    fn frame(copied: bool) -> Frame {
        let button = Clipboard::new("copy-link", "Copiar enlace", ComponentPalette::neutral_dark())
            .copied(copied)
            .render();
        UiTree::new(Element::row(vec![button, Element::spacer().flex(1.0)])).layout(SIZE)
    }

    fn icons(frame: &Frame) -> Vec<Icon> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { icon, .. } => Some(*icon),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_icon_turns_into_a_check_in_the_same_place_while_copied() {
        let (rest, copied) = (frame(false), frame(true));
        assert_eq!(icons(&rest), [Icon::Copy]);
        assert_eq!(icons(&copied), [Icon::Check]);
        assert_eq!(rest.regions, copied.regions);
    }

    #[test]
    fn a_click_reports_its_id_and_the_host_times_the_check() {
        let button = Clipboard::new("copy-link", "Copiar enlace", ComponentPalette::neutral_dark())
            .label("Copiar enlace")
            .render();
        let mut tree = UiTree::new(button);
        tree.layout(SIZE);
        let point = Point::new(10.0, 10.0);
        tree.handle_event(UiEvent::PointerDown(point));
        let clicked = tree.handle_event(UiEvent::PointerUp(point));
        assert_eq!(clicked, [UiAction::Clicked { id: "copy-link".into() }]);

        let now = Instant::now();
        let copied = ClipboardCopied::new("copy-link", now);
        assert!(copied.shows("copy-link", now + Duration::from_millis(1400)));
        assert!(!copied.shows("copy-link", now + Duration::from_millis(1600)));
        assert!(!copied.shows("copy-path", now));
    }
}
