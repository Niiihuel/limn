use crate::components::{ComponentPalette, Tooltip};
use crate::{Access, Color, Component, Element, Icon, Role};

// Windows' caption glyphs: hairlines in a 10 pixel square.
const MINIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0 5.5h10" fill="none" stroke="currentColor"/></svg>"#;
const MAXIMIZE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor"/></svg>"#;
const RESTORE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M2.5 2.5V0.5h7v7h-2M0.5 2.5h7v7h-7z" fill="none" stroke="currentColor"/></svg>"#;
const CLOSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" fill="none" stroke="currentColor"/></svg>"#;
// The close button's fill under the pointer and pressed, as Windows draws it.
const CLOSE_HOVER: Color = Color::rgb(196, 43, 28);
const CLOSE_PRESSED: Color = Color::rgb(200, 64, 49);

/// The buttons at the end of a window's own title bar, for an app that draws
/// its frame: minimize, maximize (restore while maximized) and close, in that
/// order, as Windows places them. Each is [`Self::BUTTON_WIDTH`] wide and as
/// tall as the bar; close turns red under the pointer.
///
/// The host owns the window. It says whether the window is maximized and
/// answers `Clicked` with [`Self::minimize_id`], [`Self::maximize_id`] (in
/// both states) or [`Self::close_id`], with its backend's window commands.
pub struct WindowControls {
    id: String,
    maximized: bool,
    height: f32,
    palette: ComponentPalette,
}

impl WindowControls {
    /// The width of each button.
    pub const BUTTON_WIDTH: f32 = 46.0;
    /// The width of all three: the room to keep at the end of the title bar.
    pub const WIDTH: f32 = 3.0 * Self::BUTTON_WIDTH;

    /// The three buttons, 32 logical pixels tall, under IDs made from `id`.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self { id: id.into(), maximized: false, height: 32.0, palette }
    }

    /// Whether the window is maximized: the middle button then restores it.
    pub fn maximized(mut self, maximized: bool) -> Self {
        self.maximized = maximized;
        self
    }

    /// Make the buttons as tall as the title bar. A value that is not finite
    /// or not above zero is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() && height > 0.0 {
            self.height = height;
        }
        self
    }

    /// What the minimize button reports: `id` followed by `:minimize`.
    pub fn minimize_id(id: &str) -> String {
        format!("{id}:minimize")
    }

    /// What the middle button reports, maximized or not: `id` followed by `:maximize`.
    pub fn maximize_id(id: &str) -> String {
        format!("{id}:maximize")
    }

    /// What the close button reports: `id` followed by `:close`.
    pub fn close_id(id: &str) -> String {
        format!("{id}:close")
    }
}

impl Component for WindowControls {
    fn render(self) -> Element {
        let labels = crate::labels();
        let (middle, middle_label) =
            if self.maximized { (RESTORE, labels.restore) } else { (MAXIMIZE, labels.maximize) };
        let button = |id: String, glyph: &'static str, name: &'static str, close: bool| {
            let glyph = Element::icon(Icon::Custom(glyph)).foreground(self.palette.text).size(Some(10.0), Some(10.0));
            let centered = Element::column(vec![
                Element::spacer().flex(1.0),
                Element::row(vec![Element::spacer().flex(1.0), glyph, Element::spacer().flex(1.0)]),
                Element::spacer().flex(1.0),
            ]);
            let mut element = self
                .palette
                .bar_item_surface(Element::clickable(id.clone(), centered))
                .accessible(Access::new(Role::Button).name(name))
                .size(Some(Self::BUTTON_WIDTH), Some(self.height));
            if close {
                element = element
                    .hover_background(CLOSE_HOVER)
                    .pressed_background(CLOSE_PRESSED)
                    .hover_foreground(Color::WHITE)
                    .pressed_foreground(Color::WHITE);
            }
            Tooltip::new(element, id, name, self.palette).render()
        };
        Element::row(vec![
            button(Self::minimize_id(&self.id), MINIMIZE, labels.minimize, false),
            button(Self::maximize_id(&self.id), middle, middle_label, false),
            button(Self::close_id(&self.id), CLOSE, labels.close, true),
        ])
        .size(Some(Self::WIDTH), Some(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HitKind, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn tree(maximized: bool) -> UiTree {
        let bar = Element::row(vec![
            Element::spacer().flex(1.0),
            WindowControls::new("window", ComponentPalette::neutral_dark()).maximized(maximized).height(40.0).render(),
        ]);
        let mut tree = UiTree::new(Element::column(vec![bar, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree
    }

    #[test]
    fn three_buttons_fill_the_bar_end_and_report_their_ids() {
        let mut tree = tree(false);
        let frame = tree.layout(Size::new(400.0, 300.0));
        let rect = |id: &str| {
            frame.regions.iter().find(|region| region.id == id && region.kind == HitKind::Button).map(|region| region.rect)
        };
        assert_eq!(rect("window:minimize"), Some(Rect::new(262.0, 0.0, 46.0, 40.0)));
        assert_eq!(rect("window:maximize"), Some(Rect::new(308.0, 0.0, 46.0, 40.0)));
        assert_eq!(rect("window:close"), Some(Rect::new(354.0, 0.0, 46.0, 40.0)));
        let center = Point::new(377.0, 20.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: "window:close".into() }]);
    }

    #[test]
    fn the_middle_button_is_named_for_what_it_will_do() {
        for (maximized, name) in [(false, "Maximize"), (true, "Restore")] {
            let frame = tree(maximized).layout(Size::new(400.0, 300.0));
            let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));
            assert_eq!((node("window:minimize").role, node("window:minimize").name.as_str()), (Role::Button, "Minimize"));
            assert_eq!(node("window:maximize").name, name);
            assert_eq!(node("window:close").name, "Close");
        }
    }
}
