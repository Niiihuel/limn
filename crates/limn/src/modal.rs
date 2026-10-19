//! A reusable centered modal for forms and other multi-step content.

use crate::components::{Component, ComponentPalette, Tooltip};
use crate::{
    Access, Color, CornerRadii, Element, FontFamily, Icon, Point, PopoverPlacement, Role, ScrollArea,
    ScrollbarAppearance, TextSpan,
};

/// A shared compact close icon with a mouse/keyboard target.
/// Hosts keep ownership of dismissal; this component only dispatches its action.
pub struct DismissButton {
    id: String,
    action_id: Option<String>,
    enabled: bool,
    target_size: f32,
    palette: ComponentPalette,
}

impl DismissButton {
    /// A close button 24 logical pixels a side that reports `id` when clicked. Its name and
    /// its hint are `Labels::close`.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            action_id: None,
            enabled: true,
            target_size: 24.0,
            palette,
        }
    }

    /// Report `action` instead of the button's own ID, which stays the ID of the control
    /// and of its tooltip. A disabled button reports nothing.
    pub fn dispatch_as(mut self, action: impl Into<String>) -> Self {
        self.action_id = Some(action.into());
        self
    }

    /// Enlarge the square pointer/keyboard target while keeping its icon
    /// centered at 16 pixels. The default and minimum are 24; use 44 for touch.
    /// A value that is not finite is ignored.
    pub fn target_size(mut self, size: f32) -> Self {
        if size.is_finite() { self.target_size = size.max(24.0); }
        self
    }

    /// A disabled one is passive: the same icon and hint in the same place,
    /// with no hover, hit region or focus, and nothing dispatched.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for DismissButton {
    fn render(self) -> Element {
        let icon = Element::column(vec![
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
        ]);
        // An icon alone: its hint is its name.
        let said = Access::new(Role::Button).name(crate::labels().close);
        let mut button = if self.enabled {
            self.palette
                .interactive_surface(Element::clickable(self.id.clone(), icon))
                .accessible(said)
        } else {
            Element::panel(icon).accessible(said.id(self.id.clone()).disabled(true))
        }
        .size(Some(self.target_size), Some(self.target_size));
        if let Some(action) = self.action_id.filter(|_| self.enabled) {
            button = button.dispatch_as(action);
        }
        Tooltip::new(button, self.id, crate::labels().close, self.palette)
            .shortcut("Esc")
            .render()
    }
}

// The card's inset and the rail its body reserves for the scrollbar.
const PADDING: f32 = 14.0;
const RAIL_WIDTH: f32 = 14.0;
// The least space kept between the card and the window, both sides together.
const VIEWPORT_MARGIN: f32 = 24.0;

struct SheetLayout {
    max_height: f32,
    safe_area_bottom: f32,
}

/// A controlled modal with a fixed header and footer. Only the body scrolls,
/// up to `height` or to 480 without one, keeping its actions visible in a
/// short viewport.
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
    role: Role,
    spoken_description: String,
    sheet: Option<SheetLayout>,
}

impl Modal {
    /// A modal over `base`, drawn only while `open`; closed, it renders `base` alone. The
    /// close button reports `close_id`, which also prefixes the modal's own IDs,
    /// `{close_id}:close` and `{close_id}:dialog`. An empty `description` is left out. The
    /// card is 520 wide by default.
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
            role: Role::Dialog,
            spoken_description: String::new(),
            sheet: None,
        }
    }

    /// An alert dialog shows its description in its body: say it is one,
    /// described by that.
    pub(crate) fn alert(mut self, description: &str) -> Self {
        self.role = Role::AlertDialog;
        self.spoken_description = description.to_owned();
        self
    }

    // BottomSheet shares this composition, changing only its presentation.
    pub(crate) fn bottom_sheet(mut self, max_height: f32, safe_area_bottom: f32) -> Self {
        self.sheet = Some(SheetLayout { max_height, safe_area_bottom });
        self
    }

    /// Set the card's width in logical pixels, never below 240 (520 by default). A value
    /// that is not finite is ignored.
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

    /// The width of a body that fits its card without scrolling: no rail is
    /// reserved, so it reaches the padding on both sides.
    pub fn fitted_body_width(modal_width: f32) -> f32 {
        (modal_width - 2.0 * PADDING).max(0.0)
    }

    /// Set a bounded card height. The body receives the remaining space and
    /// becomes scrollable; the viewport can clamp the card further.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height.max(160.0));
        }
        self
    }

    /// The content between the header and the footer. It is the only part that scrolls.
    pub fn body(mut self, body: Element) -> Self {
        self.body = Some(body);
        self
    }

    /// Content under the body, such as the row of actions. It does not scroll with the
    /// body.
    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }

    /// The control inside the modal that takes focus when it opens.
    pub fn initial_focus(mut self, id: impl Into<String>) -> Self {
        self.initial_focus = Some(id.into());
        self
    }

    /// The control that takes focus when the modal closes.
    pub fn return_focus(mut self, id: impl Into<String>) -> Self {
        self.return_focus = Some(id.into());
        self
    }

    /// Whether a click on the backdrop dismisses the modal; off by default. The backdrop
    /// blocks the content underneath either way.
    pub fn backdrop_dismiss(mut self, enabled: bool) -> Self {
        self.backdrop_dismiss = enabled;
        self
    }

    /// Set how long the modal takes to appear, in milliseconds. The default is the
    /// `enter_ms` motion token.
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
            .target_size(if self.sheet.is_some() { 44.0 } else { 24.0 })
            .dispatch_as(self.close_id.clone())
            .render();

        // Known by an ID of its own: `close_id` may be a button's inside it.
        let mut said = Access::new(self.role).id(format!("{}:dialog", self.close_id)).name(self.title.clone());
        let description = if self.description.is_empty() { self.spoken_description } else { self.description.clone() };
        if !description.is_empty() {
            said = said.description(description);
        }

        let title = Element::formatted_text(vec![TextSpan::new(
            self.title, self.palette.text, 16.0, FontFamily::Sans,
        )]);
        let header = if self.sheet.is_some() {
            // Share one touch-height header: a separate fixed handle row
            // would crowd the footer when the keyboard shortens the viewport.
            // These are sibling controls, so the close button wins hit tests
            // without nesting interactive accessibility nodes.
            let handle = Element::column(vec![
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    Element::spacer().size(Some(36.0), Some(4.0))
                        .background(self.palette.muted).radius(2.0),
                    Element::spacer().flex(1.0),
                ]).size(None, Some(4.0)),
                Element::spacer().flex(1.0),
            ]);
            let handle = self.palette.interactive_surface(
                Element::clickable(format!("{}:drag", self.close_id), handle)
                    .dispatch_as(self.close_id.clone())
                    .draggable_in_place()
                    .accessible(Access::new(Role::Button).name(crate::labels().close))
            ).size(None, Some(44.0));
            let title = Element::column(vec![
                Element::spacer().size(None, Some(12.0)), title,
            ]).flex(1.0);
            Element::stack(vec![handle, Element::row(vec![title, close]).gap(8.0)])
                .size(None, Some(44.0))
        } else {
            Element::row(vec![title.flex(1.0), close]).gap(8.0).min_size(0.0, 24.0)
        };
        let mut sections = vec![header];
        let mut body = self.body;

        if !self.description.is_empty() {
            let description = Element::text(self.description)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(12.0);
            if self.sheet.is_some() {
                let mut content = vec![description];
                if let Some(body) = body { content.push(body); }
                body = Some(Element::column(content).gap(10.0));
            } else { sections.push(description); }
        }

        if let Some(body) = body {
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
                    .max_height(self.sheet.as_ref().map_or(self.height.unwrap_or(480.0), |s| s.max_height))
                    .flex(1.0),
            );
        }

        if let Some(footer) = self.footer {
            sections.push(footer);
        }

        let mut content = Element::column(sections).gap(10.0);
        if let Some(sheet) = &self.sheet {
            if sheet.safe_area_bottom > 0.0 {
                content = Element::column(vec![
                    content.flex(1.0),
                    Element::spacer().size(None, Some(sheet.safe_area_bottom)),
                ]);
            }
        }
        let mut card = Element::panel(content)
            .padding(PADDING)
            .size(Some(self.width), self.height)
            .background(self.palette.surface)
            .border(self.palette.border, 1.0)
            .radius(crate::tokens().radius.panel)
            .shadow(self.palette.shadow, Point::new(0.0, 16.0), 32.0, 0.0)
            .clip_children()
            .accessible(said);

        let placement = if let Some(sheet) = self.sheet {
            card = card.max_height(sheet.max_height)
                .corner_radii(CornerRadii::top(crate::tokens().radius.panel));
            PopoverPlacement::Bottom
        } else { PopoverPlacement::Center };

        let mut popover =
            Element::popover(self.base, card, self.close_id, placement)
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
    fn a_modal_is_a_dialog_named_by_its_title_and_its_close_button_is_named_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Modal::new(Element::spacer(), true, "Nuevo proyecto", "Elige una carpeta.", "new", palette)
                .footer(DismissButton::new("locked", palette).disabled().render())
                .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(800.0, 600.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let dialog = node("new:dialog");
        assert_eq!((dialog.role, dialog.name.as_str()), (Role::Dialog, "Nuevo proyecto"));
        assert_eq!(dialog.description.as_deref(), Some("Elige una carpeta."));
        assert!(!dialog.actionable && dialog.rect.width == 520.0);
        let close = node("new:close");
        assert_eq!((close.role, close.name.as_str()), (Role::Button, crate::labels().close));
        assert!(close.actionable && !close.disabled);
        let locked = node("locked");
        assert_eq!((locked.role, locked.name.as_str()), (Role::Button, crate::labels().close));
        assert!(locked.disabled && !locked.actionable);
    }

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
                ctrl: false,
                alt: false
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn a_disabled_dismiss_icon_keeps_its_place_and_takes_no_pointer_or_focus() {
        let palette = ComponentPalette::neutral_dark();
        let tree_of = |enabled| {
            let mut tree = UiTree::new(
                DismissButton::new("close", palette)
                    .dispatch_as("dismiss")
                    .enabled(enabled)
                    .render(),
            );
            tree.set_reduced_motion(true);
            tree
        };
        let icon = |frame: &crate::Frame| {
            frame.commands.iter().find_map(|command| match command {
                crate::DrawCommand::Icon { rect, icon: Icon::Close, tint } => Some((*rect, *tint)),
                _ => None,
            })
        };
        let viewport = Size::new(24.0, 24.0);
        let enabled = tree_of(true).layout(viewport);
        let mut tree = tree_of(false);
        let disabled = tree.layout(viewport);
        assert!(!disabled.regions.iter().any(|region| region.kind == HitKind::Button));
        let point = Point::new(12.0, 12.0);
        tree.handle_event(UiEvent::PointerMove(point));
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        for id in ["close", "dismiss"] {
            assert!(!tree.focus(id));
        }
        let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false, alt: false };
        assert!(tree.handle_event(enter).is_empty());
        assert_eq!(icon(&disabled), icon(&enabled));
        assert_eq!(icon(&disabled).unwrap().1, palette.muted);
        // No hover fill: under the pointer it paints what it did at rest.
        assert_eq!(tree.layout(viewport).commands, disabled.commands);
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
                if rail > 0.0 {
                    assert_eq!(field.width, Modal::fitted_body_width(width), "{width}");
                }
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
            alt: false,
        });
        assert_ne!(tree.focused_id(), Some("behind"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
                alt: false,
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
            alt: false,
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
