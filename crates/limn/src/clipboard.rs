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
    enabled: bool,
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
            enabled: true,
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

    /// Set the button's control size; `ControlSize::Compact` by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Set the button's variant. The default, `ButtonVariant::Ghost`, draws the icon and
    /// label muted until the button is hovered or `copied`.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A disabled one is passive: a muted icon and label with the hint kept,
    /// and no hover, hit region or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Clipboard {
    fn render(self) -> Element {
        let icon = if self.copied { Icon::Check } else { self.icon };
        let mut button = IconButton::new(self.id, icon, self.palette)
            .variant(self.variant)
            .size(self.size)
            .enabled(self.enabled)
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
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};
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

    #[test]
    fn the_platform_is_told_a_button_named_by_what_it_copies() {
        let palette = ComponentPalette::neutral_dark();
        let node = |button: Clipboard| {
            let frame = UiTree::new(Element::row(vec![button.render(), Element::spacer().flex(1.0)])).layout(SIZE);
            frame.access.iter().find(|node| node.id == "copy-link").cloned().expect("a node")
        };
        // The icon alone is named by its hint; a label names it and the hint describes it.
        for copied in [false, true] {
            let bare = node(Clipboard::new("copy-link", "Copy link", palette).copied(copied));
            assert_eq!((bare.role, bare.name.as_str(), bare.description), (crate::Role::Button, "Copy link", None));
            assert!(bare.actionable && !bare.disabled);
        }
        let labelled = node(Clipboard::new("copy-link", "Copy the link to this page", palette).label("Copy"));
        assert_eq!(labelled.name, "Copy");
        assert_eq!(labelled.description.as_deref(), Some("Copy the link to this page"));
        let disabled = node(Clipboard::new("copy-link", "Copy link", palette).disabled());
        assert_eq!((disabled.role, disabled.name.as_str()), (crate::Role::Button, "Copy link"));
        assert!(disabled.disabled && !disabled.actionable);
    }

    #[test]
    fn a_disabled_one_is_muted_takes_no_pointer_or_focus_and_keeps_its_place() {
        let palette = ComponentPalette::neutral_dark();
        let tree_of = |enabled| {
            let button = Clipboard::new("copy-link", "Copiar enlace", palette)
                .label("Copiar enlace")
                .enabled(enabled)
                .render();
            let mut tree = UiTree::new(Element::column(vec![
                Element::row(vec![button, Element::text("after")]),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            tree
        };
        // The icon and each line of text, with its place and its ink.
        let marks = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { rect, tint, .. } => Some((Point::new(rect.x, rect.y), *tint)),
                    DrawCommand::Text { position, color, .. } => Some((*position, *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let enabled = tree_of(true).layout(SIZE);
        let mut tree = tree_of(false);
        let disabled = tree.layout(SIZE);
        assert!(!disabled.regions.iter().any(|region| region.kind == HitKind::Button));
        let point = Point::new(10.0, 10.0);
        tree.handle_event(UiEvent::PointerMove(point));
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(!tree.focus("copy-link"));
        let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
        assert!(tree.handle_event(enter).is_empty());
        let (was, now) = (marks(&enabled), marks(&disabled));
        // The icon and the label are muted; the text beside it is not the button's.
        assert_eq!((now[0].1, now[1].1), (palette.muted, palette.muted));
        let places = |marks: &[(Point, crate::Color)]| marks.iter().map(|mark| mark.0).collect::<Vec<_>>();
        assert_eq!(places(&now), places(&was));
    }
}
