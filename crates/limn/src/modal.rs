//! A reusable centered modal for forms and other multi-step content.

use crate::components::{Component, ComponentPalette, Tooltip};
use crate::{
    Color, Element, FontFamily, Icon, Point, PopoverPlacement, ScrollArea, ScrollbarAppearance,
    TextSpan,
};

/// A shared compact close icon with a mouse/keyboard target.
/// Hosts keep ownership of dismissal; this component only dispatches its action.
pub struct DismissButton {
    id: String,
    action_id: Option<String>,
    palette: ComponentPalette,
}

impl DismissButton {
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            action_id: None,
            palette,
        }
    }

    pub fn dispatch_as(mut self, action: impl Into<String>) -> Self {
        self.action_id = Some(action.into());
        self
    }
}

impl Component for DismissButton {
    fn render(self) -> Element {
        let mut button = self
            .palette
            .interactive_surface(Element::clickable(
                self.id.clone(),
                Element::column(vec![
                    Element::spacer().flex(1.0),
                    Element::row(vec![
                        Element::spacer().flex(1.0),
                        Element::icon(Icon::Close)
                            .foreground(self.palette.muted)
                            .size(Some(16.0), Some(16.0)),
                        Element::spacer().flex(1.0),
                    ])
                    .size(None, Some(16.0)),
                    Element::spacer().flex(1.0),
                ]),
            ))
            .size(Some(24.0), Some(24.0));
        if let Some(action) = self.action_id {
            button = button.dispatch_as(action);
        }
        Tooltip::new(button, self.id, "Cerrar", self.palette)
            .shortcut("Esc")
            .render()
    }
}

// The card's inset and the rail its body reserves for the scrollbar.
const PADDING: f32 = 14.0;
const RAIL_WIDTH: f32 = 14.0;
// The least space kept between the card and the window, both sides together.
const VIEWPORT_MARGIN: f32 = 24.0;

/// A controlled modal with a fixed header and footer. When `height` is set,
/// only the body scrolls, keeping its actions visible in a short viewport.
pub struct Modal {
    base: Element,
    open: bool,
    title: String,
    description: String,
    close_id: String,
    palette: ComponentPalette,
    width: f32,
    height: Option<f32>,
    body: Option<Element>,
    footer: Option<Element>,
    initial_focus: Option<String>,
    return_focus: Option<String>,
    backdrop_dismiss: bool,
    animation_ms: u16,
}

impl Modal {
    pub fn new(
        base: Element,
        open: bool,
        title: impl Into<String>,
        description: impl Into<String>,
        close_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            open,
            title: title.into(),
            description: description.into(),
            close_id: close_id.into(),
            palette,
            width: 520.0,
            height: None,
            body: None,
            footer: None,
            initial_focus: None,
            return_focus: None,
            backdrop_dismiss: false,
            animation_ms: crate::tokens().motion.enter_ms,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(240.0);
        }
        self
    }

    /// The width for a card that prefers `max` in a window `viewport_width`
    /// wide: it gives way so the window's margin stays clear.
    pub fn fit_width(viewport_width: f32, max: f32) -> f32 {
        (viewport_width - VIEWPORT_MARGIN).min(max)
    }

    /// The width a body can count on in a card `modal_width` wide: what is
    /// left inside the padding and the rail reserved while the body scrolls.
    /// A body that does not scroll is laid out one rail wider.
    pub fn body_width(modal_width: f32) -> f32 {
        (modal_width - 2.0 * PADDING - RAIL_WIDTH).max(0.0)
    }

    /// Set a bounded card height. The body receives the remaining space and
    /// becomes scrollable; the viewport can clamp the card further.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height.max(160.0));
        }
        self
    }

    pub fn body(mut self, body: Element) -> Self {
        self.body = Some(body);
        self
    }

    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn initial_focus(mut self, id: impl Into<String>) -> Self {
        self.initial_focus = Some(id.into());
        self
    }

    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.return_focus = Some(id.into());
        self
    }

    pub fn backdrop_dismiss(mut self, enabled: bool) -> Self {
        self.backdrop_dismiss = enabled;
        self
    }

    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }
}

impl Component for Modal {
    fn render(self) -> Element {
        if !self.open {
            return self.base;
        }

        let close = DismissButton::new(format!("{}:close", self.close_id), self.palette)
            .dispatch_as(self.close_id.clone())
            .render();

        let mut sections = vec![Element::row(vec![
            Element::formatted_text(vec![TextSpan::new(
                self.title,
                self.palette.text,
                16.0,
                FontFamily::Sans,
            )])
            .flex(1.0),
            close,
        ])
        .gap(8.0)
        .min_size(0.0, 24.0)];

        if !self.description.is_empty() {
            sections.push(
                Element::text(self.description)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(12.0),
            );
        }

        if let Some(body) = self.body {
            sections.push(
                ScrollArea::new(format!("modal_body:{}", self.close_id), body)
                    // A dedicated rail keeps the thumb outside bordered form
                    // sections and never covers controls at the right edge.
                    .appearance(
                        ScrollbarAppearance::reserved()
                            .rail_width(RAIL_WIDTH)
                            .thumb_width(4.0)
                            .tint(self.palette.muted),
                    )
                    .render()
                    .max_height(self.height.unwrap_or(480.0))
                    .flex(1.0),
            );
        }

        if let Some(footer) = self.footer {
            sections.push(footer);
        }

        let card = Element::panel(Element::column(sections).gap(10.0))
            .padding(PADDING)
            .size(Some(self.width), self.height)
            .background(self.palette.surface)
            .border(self.palette.border, 1.0)
            .radius(crate::tokens().radius.panel)
            .shadow(self.palette.shadow, Point::new(0.0, 16.0), 32.0, 0.0)
            .clip_children();

        let mut popover =
            Element::popover(self.base, card, self.close_id, PopoverPlacement::Center)
                .popover_scrim(Color::rgba(0, 0, 0, 180))
                .popover_backdrop_dismiss(self.backdrop_dismiss)
                .popover_animation_ms(self.animation_ms);
        if let Some(id) = self.initial_focus {
            popover = popover.popover_initial_focus(id);
        }
        if let Some(id) = self.return_focus {
            popover = popover.popover_return_focus(id);
        }
        popover
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HitKind, Key, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn dismiss_icon_is_centered_at_rest_hover_and_focus_and_dispatches_once() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            DismissButton::new("close", palette)
                .dispatch_as("dismiss")
                .render(),
        );
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
        let viewport = Size::new(24.0, 24.0);
        for hover in [false, true] {
            if hover {
                tree.handle_event(UiEvent::PointerMove(Point::new(12.0, 12.0)));
            }
            let frame = tree.layout(viewport);
            let icon = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    crate::DrawCommand::Icon {
                        rect,
                        icon: Icon::Close,
                        ..
                    } => Some(rect),
                    _ => None,
                })
                .expect("shared close icon");
            assert!((icon.x + icon.width / 2.0 - 12.0).abs() < 0.1);
            assert!((icon.y + icon.height / 2.0 - 12.0).abs() < 0.1);
            assert!(!frame.commands.iter().any(|command| matches!(command,
                crate::DrawCommand::Text { text, font_family: FontFamily::Sans, .. } if text == "Esc"
            )));
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .filter(|region| region.kind == HitKind::Button)
                    .count(),
                1
            );
        }
        tree.handle_event(UiEvent::PointerDown(Point::new(12.0, 12.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(12.0, 12.0))),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert!(tree.focus("close"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn modal_backdrop_respects_its_policy_and_never_clicks_the_underlying_control() {
        for dismiss in [false, true] {
            let mut tree = UiTree::new(
                Modal::new(
                    Element::button("behind", "Background"),
                    true,
                    "History",
                    "",
                    "dismiss",
                    ComponentPalette::neutral_dark(),
                )
                .width(320.0)
                .height(180.0)
                .body(Element::text("No output yet"))
                .backdrop_dismiss(dismiss)
                .animation_ms(0)
                .render(),
            );
            tree.set_reduced_motion(true);
            tree.layout(Size::new(640.0, 480.0));
            let point = Point::new(4.0, 4.0);
            let mut actions = tree.handle_event(UiEvent::PointerDown(point));
            actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
            assert_eq!(
                actions,
                if dismiss {
                    vec![UiAction::Clicked {
                        id: "dismiss".into(),
                    }]
                } else {
                    Vec::new()
                }
            );
        }
    }

    #[test]
    fn body_width_is_the_rendered_inner_width_and_fit_width_keeps_the_margin() {
        for width in [240.0, 361.0, 520.0, 860.0] {
            for (content, rail) in [(600.0, 0.0), (20.0, RAIL_WIDTH)] {
                let mut tree = UiTree::new(
                    Modal::new(
                        Element::spacer(),
                        true,
                        "Title",
                        "",
                        "dismiss",
                        ComponentPalette::neutral_dark(),
                    )
                    .width(width)
                    .height(300.0)
                    .body(Element::column(vec![
                        Element::text_input("fill", "", ""),
                        Element::spacer().size(None, Some(content)),
                    ]))
                    .render(),
                );
                tree.set_reduced_motion(true);
                let frame = tree.layout(Size::new(1200.0, 600.0));
                let field = frame.regions.iter().find(|region| region.id == "fill").unwrap().rect;
                assert_eq!(field.width, Modal::body_width(width) + rail, "{width} {content}");
            }
        }
        assert_eq!(Modal::body_width(10.0), 0.0);
        assert_eq!(Modal::fit_width(1200.0, 560.0), 560.0);
        assert_eq!(Modal::fit_width(360.0, 560.0), 336.0);
    }

    fn sample_modal() -> Element {
        Modal::new(
            Element::button("behind", "Background"),
            true,
            "Add source",
            "Choose a folder.",
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .width(420.0)
        .height(300.0)
        .body(Element::column(vec![
            Element::text_input("path", "", "Folder"),
            Element::spacer().size(None, Some(600.0)),
            Element::text_input("later", "", "Later field"),
        ]))
        .footer(Element::button("save", "Save"))
        .initial_focus("path")
        .render()
    }

    #[test]
    fn modal_centers_body_scrolls_and_footer_stays_visible() {
        let mut tree = UiTree::new(sample_modal());
        let frame = tree.layout(Size::new(640.0, 480.0));
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .expect("modal card");
        assert!((surface.rect.x + surface.rect.width / 2.0 - 320.0).abs() < 1.0);
        assert!((surface.rect.y + surface.rect.height / 2.0 - 240.0).abs() < 1.0);
        assert_eq!(tree.focused_id(), Some("path"));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "modal_body:dismiss" && region.kind == HitKind::Scroll));
        let scrollbar = frame
            .regions
            .iter()
            .find(|region| region.id == "modal_body:dismiss" && region.kind == HitKind::Scrollbar)
            .expect("modal scrollbar");
        let field = frame
            .regions
            .iter()
            .find(|region| region.id == "path")
            .unwrap();
        assert!(
            field.rect.x + field.rect.width <= scrollbar.rect.x,
            "scrollbar stays outside the form's available width"
        );
        let footer = frame
            .regions
            .iter()
            .find(|region| region.id == "save")
            .unwrap();
        assert!(footer.rect.y >= surface.rect.y);
        assert!(footer.rect.y + footer.rect.height <= surface.rect.y + surface.rect.height);
    }

    #[test]
    fn modal_traps_focus_blocks_backdrop_and_escape_dismisses() {
        let mut tree = UiTree::new(Element::button("behind", "Background"));
        tree.layout(Size::new(640.0, 480.0));
        assert!(tree.focus("behind"));
        tree.set_root(sample_modal());
        let frame = tree.layout(Size::new(640.0, 480.0));
        assert_eq!(tree.focused_id(), Some("path"));
        assert!(frame
            .regions
            .iter()
            .any(|region| { region.id == "dismiss" && region.kind == HitKind::Surface }));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_ne!(tree.focused_id(), Some("behind"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        tree.set_root(Element::button("behind", "Background"));
        tree.layout(Size::new(640.0, 480.0));
        assert_eq!(tree.focused_id(), Some("behind"));
    }

    #[test]
    fn focusing_an_offscreen_modal_field_reveals_it_in_the_outer_scroll_area() {
        let mut tree = UiTree::new(sample_modal());
        tree.layout(Size::new(640.0, 480.0));
        assert!(tree.focus("later"));
        let frame = tree.layout(Size::new(640.0, 480.0));
        assert!(tree.scroll_offset("modal_body:dismiss") > 0.0);
        assert!(frame.regions.iter().any(|region| region.id == "later"));
    }

    #[test]
    fn tab_reaches_an_offscreen_modal_field_and_reveals_it() {
        let mut tree = UiTree::new(sample_modal());
        tree.layout(Size::new(640.0, 480.0));
        assert!(tree.focus("path"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("later"));
        let frame = tree.layout(Size::new(640.0, 480.0));
        assert!(tree.scroll_offset("modal_body:dismiss") > 0.0);
        assert!(frame.regions.iter().any(|region| region.id == "later"));
    }

    #[test]
    fn narrow_viewport_keeps_close_field_and_footer_inside_card() {
        let mut tree = UiTree::new(sample_modal());
        let frame = tree.layout(Size::new(320.0, 240.0));
        let card = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap()
            .rect;
        assert!(card.x >= 8.0 && card.y >= 8.0);
        assert!(card.x + card.width <= 312.0);
        assert!(card.y + card.height <= 232.0);
        for id in ["dismiss:close", "path", "save"] {
            let control = frame.regions.iter().find(|region| region.id == id).unwrap();
            assert!(control.rect.x >= card.x, "{id}");
            assert!(control.rect.y >= card.y, "{id}");
            assert!(
                control.rect.x + control.rect.width <= card.x + card.width,
                "{id}"
            );
            assert!(
                control.rect.y + control.rect.height <= card.y + card.height,
                "{id}"
            );
        }
    }

    #[test]
    fn return_focus_uses_stable_control_when_opening_menu_item_disappears() {
        let mut tree = UiTree::new(Element::button("temporary_menu_item", "Folder"));
        tree.layout(Size::new(640.0, 480.0));
        assert!(tree.focus("temporary_menu_item"));
        let base = || Element::button("add_menu", "Add source");
        tree.set_root(
            Modal::new(
                base(),
                true,
                "Add source",
                "",
                "cancel",
                ComponentPalette::neutral_dark(),
            )
            .body(Element::text_input("path", "", "Folder"))
            .initial_focus("path")
            .return_focus("add_menu")
            .render(),
        );
        tree.layout(Size::new(640.0, 480.0));
        assert_eq!(tree.focused_id(), Some("path"));
        tree.set_root(base());
        tree.layout(Size::new(640.0, 480.0));
        assert_eq!(tree.focused_id(), Some("add_menu"));
    }
}
