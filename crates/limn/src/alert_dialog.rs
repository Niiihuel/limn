//! A centered confirmation surface for consequential actions.

use crate::components::{Component, ComponentPalette};
use crate::{
    Button, ButtonSizing, ButtonVariant, Color, ControlSize, Element, FontFamily, Modal, TextSpan,
};

/// Controlled modal dialog. The host performs the action only after receiving
/// `confirm_id`; close, Cancel and Escape emit `cancel_id`. The scrim blocks
/// interaction with the background and does not dismiss the dialog.
pub struct AlertDialog {
    base: Element,
    open: bool,
    title: String,
    description: String,
    confirm_id: String,
    cancel_id: String,
    confirm_label: String,
    confirm_enabled: bool,
    cancel_label: String,
    palette: ComponentPalette,
    width: f32,
    // The confirm button's fill, outline (drawn when set) and ink.
    confirm_colors: Option<(Color, Option<Color>, Color)>,
    cancel_variant: ButtonVariant,
    description_size: f32,
    closable: bool,
    scrim: Option<Color>,
    title_gap: Option<f32>,
    button_sizing: ButtonSizing,
    animation_ms: u16,
    body: Option<Element>,
    alternative: Option<(String, String)>,
}

impl AlertDialog {
    /// An alert dialog over `base`, drawn only while `open`; closed, it renders `base`
    /// alone. The confirm button reports `confirm_id`; Cancel and the close button report
    /// `cancel_id`. The button labels come from the current `Labels`, the card is 440 wide
    /// and Cancel takes the initial focus.
    pub fn new(
        base: Element,
        open: bool,
        title: impl Into<String>,
        description: impl Into<String>,
        confirm_id: impl Into<String>,
        cancel_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            open,
            title: title.into(),
            description: description.into(),
            confirm_id: confirm_id.into(),
            cancel_id: cancel_id.into(),
            confirm_label: crate::labels().confirm.into(),
            confirm_enabled: true,
            cancel_label: crate::labels().cancel.into(),
            palette,
            width: 440.0,
            confirm_colors: None,
            cancel_variant: ButtonVariant::Outline,
            description_size: 12.0,
            closable: true,
            scrim: None,
            title_gap: None,
            button_sizing: ControlSize::Compact.button_sizing(),
            animation_ms: crate::tokens().motion.enter_ms,
            body: None,
            alternative: None,
        }
    }

    /// Replace the confirm button's text, which is `Labels::confirm` by default.
    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Replace the cancel button's text, which is `Labels::cancel` by default.
    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Keep the action visible while an asynchronous operation prepares it.
    pub fn confirm_enabled(mut self, enabled: bool) -> Self {
        self.confirm_enabled = enabled;
        self
    }

    /// Fill the confirm button with `color` and draw its label in white, for an action that
    /// destroys something. Without it the button is filled with the palette's text color.
    pub fn destructive(mut self, color: Color) -> Self {
        self.confirm_colors = Some((color, None, Color::WHITE));
        self
    }

    /// A quieter destructive confirm: a pale `fill` with a one-pixel `border`
    /// and a label in `ink`, such as #FBE7E7, #DC3C3D and #A42528.
    pub fn soft_destructive(mut self, fill: Color, border: Color, ink: Color) -> Self {
        self.confirm_colors = Some((fill, Some(border), ink));
        self
    }

    /// Paint Cancel as this variant, `Outline` by default; `Secondary` is
    /// filled with the palette's `raised`.
    pub fn cancel_variant(mut self, variant: ButtonVariant) -> Self {
        self.cancel_variant = variant;
        self
    }

    /// Set the description's size in logical pixels, 12 by default. A value
    /// that is not finite is ignored.
    pub fn description_size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.description_size = size.max(0.0);
        }
        self
    }

    /// Whether the title row has a close button; on by default. Without it,
    /// Cancel and Escape still report `cancel_id`.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// The color laid over the window behind the card; see [`Modal::scrim`].
    pub fn scrim(mut self, color: Color) -> Self {
        self.scrim = Some(color);
        self
    }

    /// The space under the title row; see [`Modal::title_gap`].
    pub fn title_gap(mut self, gap: f32) -> Self {
        self.title_gap = Some(gap);
        self
    }

    /// The geometry of the action buttons, `ControlSize::Compact`'s by
    /// default: 28 high with a 12 pixel label.
    pub fn button_sizing(mut self, sizing: ButtonSizing) -> Self {
        self.button_sizing = sizing;
        self
    }

    /// Set the card's width in logical pixels, never below 240 (440 by default). Below 360
    /// the actions stack in a column. A value that is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(240.0);
        }
        self
    }

    /// Set how long the dialog takes to appear, in milliseconds. The default is the
    /// `enter_ms` motion token.
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    /// A second way to go ahead, between Cancel and the confirm action: the
    /// same step without its consequential part, such as "Don't revert". The
    /// host receives `id` as it receives `confirm_id`.
    pub fn alternative(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.alternative = Some((id.into(), label.into()));
        self
    }

    /// Optional form or other content between the description and actions.
    pub fn body(mut self, body: Element) -> Self {
        self.body = Some(body);
        self
    }
}

impl Component for AlertDialog {
    fn render(self) -> Element {
        if !self.open {
            return self.base;
        }
        let (confirm_fill, confirm_border, confirm_ink) = self
            .confirm_colors
            .unwrap_or((self.palette.text, None, self.palette.surface));
        let cancel = Button::new(self.cancel_id.clone(), self.cancel_label, self.palette)
            .variant(self.cancel_variant)
            .sizing(self.button_sizing)
            .render();
        let confirm = Button::new(
            self.confirm_id,
            self.confirm_label,
            ComponentPalette {
                text: confirm_ink,
                surface: confirm_fill,
                hover: confirm_fill,
                selected: confirm_fill,
                border: confirm_border.unwrap_or(self.palette.border),
                ..self.palette
            },
        )
        .variant(if confirm_border.is_some() {
            ButtonVariant::Outline
        } else {
            ButtonVariant::Solid
        })
        .sizing(self.button_sizing)
        .enabled(self.confirm_enabled)
        .render()
        .center_content();
        let mut actions = vec![cancel];
        if let Some((id, label)) = self.alternative {
            actions.push(
                Button::new(id, label, self.palette)
                    .variant(ButtonVariant::Outline)
                    .sizing(self.button_sizing)
                    .render(),
            );
        }
        actions.push(confirm);
        let footer = if self.width < 360.0 {
            Element::column(actions).gap(6.0)
        } else {
            actions.insert(0, Element::spacer().flex(1.0));
            Element::row(actions)
                .gap(6.0)
                .size(None, Some(self.button_sizing.height))
        };
        let mut content = Vec::new();
        if !self.description.is_empty() {
            content.push(Element::formatted_text(vec![TextSpan::new(
                self.description.clone(),
                self.palette.muted,
                self.description_size,
                FontFamily::Sans,
            )]));
        }
        if let Some(body) = self.body {
            content.push(body);
        }
        // Share the same title, close control, scroll policy and spacing as
        // forms. Confirmation descriptions can contain long names and paths;
        // their scrolling must never push Cancel or the primary action away.
        let mut modal = Modal::new(
            self.base,
            true,
            self.title,
            "",
            self.cancel_id.clone(),
            self.palette,
        )
        .alert(&self.description)
        .width(self.width)
        .footer(footer)
        .initial_focus(self.cancel_id)
        .backdrop_dismiss(false)
        .closable(self.closable)
        .animation_ms(self.animation_ms);
        if let Some(color) = self.scrim {
            modal = modal.scrim(color);
        }
        if let Some(gap) = self.title_gap {
            modal = modal.title_gap(gap);
        }
        if !content.is_empty() {
            modal = modal.body(Element::column(content).gap(10.0));
        }
        modal.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Key, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn an_alert_dialog_says_it_is_one_named_by_its_title_and_described_by_its_description() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            AlertDialog::new(Element::spacer(), true, "¿Borrar?", "No se puede deshacer.", "confirm", "cancel", palette)
                .confirm_label("Borrar")
                .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(600.0, 400.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let dialog = node("cancel:dialog");
        assert_eq!((dialog.role, dialog.name.as_str()), (crate::Role::AlertDialog, "¿Borrar?"));
        assert_eq!(dialog.description.as_deref(), Some("No se puede deshacer."));
        assert!(!dialog.actionable);
        assert_eq!(node("confirm").name, "Borrar");
        assert_eq!(node("cancel").name, crate::labels().cancel);
        assert_eq!(node("cancel:close").name, crate::labels().close);
    }

    #[test]
    fn soft_destructive_confirm_filled_cancel_and_larger_description_paint_as_asked() {
        let palette = ComponentPalette::neutral_dark();
        let (fill, border, ink) =
            (Color::rgb(0xFB, 0xE7, 0xE7), Color::rgb(0xDC, 0x3C, 0x3D), Color::rgb(0xA4, 0x25, 0x28));
        let mut tree = UiTree::new(
            AlertDialog::new(Element::spacer(), true, "Close terminal?", "A command is still running.", "close", "cancel", palette)
                .confirm_label("Close terminal")
                .soft_destructive(fill, border, ink)
                .cancel_variant(ButtonVariant::Secondary)
                .description_size(14.0)
                .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(800.0, 600.0));
        let rect = |id: &str| {
            frame.regions.iter().find(|region| region.id == id && region.kind == HitKind::Button).unwrap().rect
        };
        let (confirm, cancel) = (rect("close"), rect("cancel"));
        let has = |wanted: &dyn Fn(&DrawCommand) -> bool| frame.commands.iter().any(wanted);
        assert!(has(&|c| matches!(c, DrawCommand::FillRect { rect, color, .. } if *rect == confirm && *color == fill)));
        assert!(has(&|c| matches!(c, DrawCommand::StrokeRect { rect, color, width, .. }
            if *rect == confirm && *color == border && *width == 1.0)));
        assert!(has(&|c| matches!(c, DrawCommand::Text { text, color, .. } if text == "Close terminal" && *color == ink)));
        assert!(has(&|c| matches!(c, DrawCommand::FillRect { rect, color, .. } if (rect.x, rect.y) == (cancel.x, cancel.y) && *color == palette.raised)));
        assert!(has(&|c| matches!(c, DrawCommand::Text { text, font_size, .. }
            if text.contains("still running") && *font_size == 14.0)));
    }

    #[test]
    fn scrim_title_gap_and_button_sizing_reach_the_painted_dialog() {
        let scrim = Color::rgba(0, 0, 0, 105);
        let sizing = ButtonSizing { height: 30.0, font_size: 14.0, horizontal_padding: 12.0, ..ControlSize::Compact.button_sizing() };
        let dialog = || AlertDialog::new(Element::spacer(), true, "Close terminal?", "Still running.", "ok", "cancel", ComponentPalette::neutral_dark());
        let lay_out = |dialog: AlertDialog| {
            let mut tree = UiTree::new(dialog.render());
            tree.set_reduced_motion(true);
            tree.layout(Size::new(1408.0, 844.0))
        };
        // From the top of the title to the top of the body that holds the description.
        let drop = |frame: &crate::Frame| {
            let title = frame.commands.iter().find_map(|c| match c {
                DrawCommand::Text { text, position, .. } if text == "Close terminal?" => Some(position.y),
                _ => None,
            }).unwrap();
            frame.regions.iter().find(|r| r.id == "modal_body:cancel").unwrap().rect.y - title
        };
        // Unless set, the body sits 34 under the title: a 24 high row and 10.
        let plain = lay_out(dialog());
        assert_eq!(drop(&plain), 34.0);
        assert!(plain.commands.iter().any(|c| matches!(c, DrawCommand::FillRect { color, .. } if *color == Color::rgba(0, 0, 0, 180))));

        let frame = lay_out(dialog().scrim(scrim).title_gap(1.0).button_sizing(sizing));
        assert_eq!(drop(&frame), 25.0);
        assert!(frame.commands.iter().any(|c| matches!(c, DrawCommand::FillRect { color, .. } if *color == scrim)));
        assert!(!frame.commands.iter().any(|c| matches!(c, DrawCommand::FillRect { color, .. } if *color == Color::rgba(0, 0, 0, 180))));
        for id in ["cancel", "ok"] {
            let rect = frame.regions.iter().find(|r| r.id == id && r.kind == HitKind::Button).unwrap().rect;
            assert_eq!(rect.height, 30.0);
        }
        assert!(frame.commands.iter().any(|c| matches!(c, DrawCommand::Text { text, font_size, .. } if text == "Cancel" && *font_size == 14.0)));
    }

    #[test]
    fn without_its_close_button_escape_and_cancel_still_cancel() {
        let mut tree = UiTree::new(
            AlertDialog::new(Element::spacer(), true, "¿Cerrar?", "", "confirm", "cancel", ComponentPalette::neutral_dark())
                .closable(false)
                .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(600.0, 400.0));
        assert!(!frame.regions.iter().any(|region| region.id == "cancel:close"));
        assert!(!frame.access.iter().any(|node| node.id == "cancel:close"));
        let escape = UiEvent::Key { key: Key::Escape, shift: false, ctrl: false };
        assert_eq!(tree.handle_event(escape), vec![UiAction::Clicked { id: "cancel".into() }]);
        assert!(tree.focus("cancel"));
        let enter = UiEvent::Key { key: Key::Enter, shift: false, ctrl: false };
        assert_eq!(tree.handle_event(enter), vec![UiAction::Clicked { id: "cancel".into() }]);
    }

    #[test]
    fn alternative_action_sits_between_cancel_and_confirm_and_reports_its_id() {
        let palette = ComponentPalette::neutral_dark();
        for (width, stacked) in [(440.0, false), (300.0, true)] {
            let mut tree = UiTree::new(
                AlertDialog::new(Element::spacer(), true, "¿Enviar?", "", "confirm", "cancel", palette)
                    .alternative("keep", "No revertir")
                    .width(width)
                    .render(),
            );
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(600.0, 400.0));
            let rect = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
            let (cancel, keep, confirm) = (rect("cancel"), rect("keep"), rect("confirm"));
            if stacked {
                assert!(cancel.y < keep.y && keep.y < confirm.y);
            } else {
                assert!(cancel.x < keep.x && keep.x < confirm.x);
                assert_eq!(keep.y, confirm.y);
            }
            let point = Point::new(keep.x + keep.width / 2.0, keep.y + keep.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "keep".into() }]
            );
        }
    }

    #[test]
    fn modal_blocks_backdrop_and_traps_keyboard_until_dismissed() {
        let palette = ComponentPalette::neutral_dark();
        let base = || Element::button("underlying", "Fondo").size(Some(400.0), Some(300.0));
        let mut tree = UiTree::new(base());
        // The dialog is checked settled, not partway through its entrance.
        tree.set_reduced_motion(true);
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("underlying"));
        tree.set_root(
            AlertDialog::new(
                base(),
                true,
                "¿Eliminar la tarea?",
                "Se perderá el historial de esta tarea.",
                "confirm",
                "cancel",
                palette,
            )
            .render(),
        );
        let frame = tree.layout(Size::new(400.0, 300.0));
        assert!(frame.regions.iter().any(|region| region.id == "confirm"));
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == Color::rgba(0, 0, 0, 180)
        )));
        assert_eq!(tree.focused_id(), Some("cancel"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("cancel:close"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("confirm"));
        tree.handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("cancel"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "cancel".into()
            }]
        );
        tree.set_root(base());
        tree.layout(Size::new(400.0, 300.0));
        assert_eq!(tree.focused_id(), Some("underlying"));
    }

    #[test]
    fn close_button_has_own_focus_but_dispatches_cancel_action() {
        let mut tree = UiTree::new(
            AlertDialog::new(
                Element::button("underlying", "Fondo"),
                true,
                "¿Salir?",
                "Se cancelarán los cambios pendientes.",
                "confirm",
                "cancel",
                ComponentPalette::neutral_dark(),
            )
            .render(),
        );
        let frame = tree.layout(Size::new(700.0, 500.0));
        let close = frame
            .regions
            .iter()
            .find(|region| region.id == "cancel:close")
            .expect("separate close target")
            .rect;
        let center = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked {
                id: "cancel".into()
            }]
        );
        assert!(tree.focus("cancel:close"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "cancel".into()
            }]
        );
    }

    #[test]
    fn narrow_dialog_keeps_both_actions_inside_its_card() {
        let frame = UiTree::new(
            AlertDialog::new(
                Element::spacer(),
                true,
                "¿Eliminar?",
                "Esta acción no se puede deshacer.",
                "confirm",
                "cancel",
                ComponentPalette::neutral_dark(),
            )
            .width(256.0)
            .render(),
        )
        .layout(Size::new(280.0, 320.0));
        let card = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:cancel")
            .unwrap()
            .rect;
        let confirm = frame
            .regions
            .iter()
            .find(|region| region.id == "confirm")
            .unwrap();
        assert!(confirm.rect.x >= card.x);
        assert!(confirm.rect.x + confirm.rect.width <= card.x + card.width);
    }

    #[test]
    fn dialog_actions_stay_within_a_narrow_viewport() {
        let frame = UiTree::new(
            AlertDialog::new(
                Element::spacer(),
                true,
                "¿Eliminar?",
                "Esta acción no se puede deshacer.",
                "confirm",
                "cancel",
                ComponentPalette::neutral_dark(),
            )
            .render(),
        )
        .layout(Size::new(320.0, 420.0));
        let card = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:cancel")
            .unwrap()
            .rect;
        for id in ["cancel:close", "cancel", "confirm"] {
            let action = frame
                .regions
                .iter()
                .find(|region| region.id == id && region.kind == HitKind::Button)
                .unwrap()
                .rect;
            assert!(
                action.x >= card.x && action.x + action.width <= card.x + card.width,
                "{id}: action={action:?}, card={card:?}"
            );
            assert!(
                action.y >= card.y && action.y + action.height <= card.y + card.height,
                "{id}: action={action:?}, card={card:?}"
            );
        }
    }

    #[test]
    fn backdrop_blocks_pointer_without_dismissing_alert() {
        let mut tree = UiTree::new(
            AlertDialog::new(
                Element::button("underlying", "Fondo"),
                true,
                "Renombrar",
                "Nuevo nombre",
                "save",
                "cancel",
                ComponentPalette::neutral_dark(),
            )
            .body(Element::text_input("name", "Anterior", "Nombre"))
            .render(),
        );
        tree.layout(Size::new(800.0, 600.0));
        assert!(tree.focus("name"));
        let outside = Point::new(4.0, 4.0);
        assert!(tree.handle_event(UiEvent::PointerDown(outside)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(outside)).is_empty());
        assert_eq!(tree.focused_id(), Some("name"));
    }

    #[test]
    fn long_confirmation_copy_scrolls_without_hiding_title_close_or_compact_actions() {
        let title = "¿Descartar los cambios de este archivo antes de volver a cargarlo?";
        for width in [280.0, 520.0] {
            let mut tree = UiTree::new(
                AlertDialog::new(
                    Element::spacer(),
                    true,
                    title,
                    "Este archivo contiene cambios locales que todavía no se guardaron. "
                        .repeat(40),
                    "discard",
                    "cancel",
                    ComponentPalette::neutral_dark(),
                )
                .width(width - 24.0)
                .confirm_label("Descartar y recargar")
                .render(),
            );
            tree.set_reduced_motion(true);
            let size = Size::new(width, 340.0);
            let first = tree.layout(size);
            assert_eq!(tree.focused_id(), Some("cancel"));
            let rect = |frame: &crate::Frame, id: &str| {
                frame
                    .regions
                    .iter()
                    .find(|region| {
                        region.id == id && (id != "cancel" || region.kind == HitKind::Button)
                    })
                    .unwrap()
                    .rect
            };
            let card = rect(&first, "popover_surface:cancel");
            let close = rect(&first, "cancel:close");
            let cancel = rect(&first, "cancel");
            let discard = rect(&first, "discard");
            assert_eq!((close.width, close.height), (24.0, 24.0));
            for control in [cancel, discard] {
                assert_eq!(control.height, ControlSize::Compact.height());
                assert!(
                    control.x >= card.x && control.x + control.width <= card.x + card.width + 0.01
                );
                assert!(control.y + control.height <= card.y + card.height + 0.01);
            }
            let title_painted = first
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        text, font_size, ..
                    } if *font_size == 16.0 => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(" ");
            assert_eq!(
                title_painted.split_whitespace().collect::<Vec<_>>(),
                title.split_whitespace().collect::<Vec<_>>()
            );
            assert!(!first
                .commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Text {text, ..} if text == "Esc")));
            let body = rect(&first, "modal_body:cancel");
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(body.x + 10.0, body.y + body.height / 2.0),
                delta_y: -10000.0,
            });
            let last = tree.layout(size);
            assert!(tree.is_scroll_at_end("modal_body:cancel", 0.01));
            assert_eq!(rect(&last, "cancel:close"), close);
            assert_eq!(rect(&last, "cancel"), cancel);
            assert_eq!(rect(&last, "discard"), discard);
        }
    }
}
