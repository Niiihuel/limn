//! Persistent, caller-controlled inline feedback with optional actions.

use crate::{
    Access, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, FontFamily,
    Icon, IconButton, Role,
};

/// The icon, status label and accent communicate the message's meaning together.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlertTone {
    #[default]
    /// A notice with no status. The icon is the information circle, in the palette's text color.
    Neutral,
    /// Something worth knowing. The icon is the information circle, as `Neutral`,
    /// in the tone's own color.
    Info,
    /// Something that went well. The icon is a check in a circle.
    Success,
    /// Something to look at before going on. The icon is a warning triangle.
    Warning,
    /// Something that failed. The icon is an error circle, and the title and the
    /// description take its hue as well.
    Error,
}

impl AlertTone {
    fn icon(self) -> Icon {
        match self {
            Self::Neutral => Icon::InfoCircle,
            Self::Info => Icon::InfoCircle,
            Self::Success => Icon::CheckCircle,
            Self::Warning => Icon::WarningTriangle,
            Self::Error => Icon::ErrorCircle,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Neutral => crate::labels().notice,
            Self::Info => crate::labels().information,
            Self::Success => crate::labels().success,
            Self::Warning => crate::labels().warning,
            Self::Error => crate::labels().error,
        }
    }
}

/// Independently configurable alert colors. The surface and border stay
/// neutral. Status color belongs to the icon, and an error also tints its
/// copy; the other tones keep ordinary ink so a page of notices stays calm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlertColors {
    /// The alert's fill. `for_tone` takes the palette's `surface`.
    pub surface: Color,
    /// The one-pixel line around it. `for_tone` takes the palette's `border`.
    pub border: Color,
    /// The icon's color. `for_tone` keeps it at a contrast of 3 or more on the surface.
    pub accent: Color,
    /// The title's color, and the ink of the close button. `for_tone` keeps it at
    /// a contrast of 4.5 or more on the surface.
    pub title: Color,
    /// The description's color. `for_tone` keeps it at a contrast of 4.5 or more
    /// on the surface.
    pub description: Color,
}

impl AlertColors {
    /// Choose dark/light status tokens and keep text and icons readable on the
    /// palette's surface. Custom colors can be supplied with `.colors`.
    pub fn for_tone(tone: AlertTone, palette: ComponentPalette) -> Self {
        let surface = palette.surface;
        let dark = luminance(surface) < 0.179;
        let accent = match (tone, dark) {
            (AlertTone::Neutral, _) => palette.text,
            (AlertTone::Info, true) => Color::rgb(120, 169, 255),
            (AlertTone::Success, true) => Color::rgb(111, 220, 140),
            (AlertTone::Warning, true) => Color::rgb(241, 194, 27),
            (AlertTone::Error, true) => Color::rgb(255, 131, 137),
            (AlertTone::Info, false) => Color::rgb(0, 67, 206),
            (AlertTone::Success, false) => Color::rgb(14, 96, 39),
            (AlertTone::Warning, false) => Color::rgb(104, 78, 0),
            (AlertTone::Error, false) => Color::rgb(184, 25, 34),
        };
        let accent = readable_ink(accent, surface, 3.0);
        let (title, description) = if tone == AlertTone::Error {
            // The description sits a step back from the title, in the same hue.
            (accent, blend(accent, surface, 40))
        } else {
            (palette.text, palette.muted)
        };
        Self {
            surface,
            border: palette.border,
            accent,
            title: readable_ink(title, surface, 4.5),
            description: readable_ink(description, surface, 4.5),
        }
    }
}

/// Inline feedback without a timer, modal backdrop or automatic focus change.
/// Action IDs stay caller-owned. A dismiss click emits its ID; the caller hides
/// the alert when appropriate.
pub struct Alert {
    id: Option<String>,
    title: String,
    description: String,
    palette: ComponentPalette,
    tone: AlertTone,
    colors: Option<AlertColors>,
    action: Option<Element>,
    dismiss_id: Option<String>,
    compact: bool,
}

impl Alert {
    /// A neutral alert with no ID, action or close button. A blank title is replaced by the
    /// tone's name, such as "Warning"; a blank description is left out.
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: None,
            title: title.into(),
            description: description.into(),
            palette,
            tone: AlertTone::Neutral,
            colors: None,
            action: None,
            dismiss_id: None,
            compact: false,
        }
    }

    /// Choose the icon and, unless `colors` is set, the colors that `AlertColors::for_tone`
    /// derives. The default is `AlertTone::Neutral`.
    pub fn tone(mut self, tone: AlertTone) -> Self {
        self.tone = tone;
        self
    }

    /// Make the alert known to the platform by this ID, as an alert named
    /// by its title and described by its description. Without one only its
    /// controls are known.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Use these colors instead of the ones derived from the tone and the palette.
    pub fn colors(mut self, colors: AlertColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Supply a real control or control group, preserving every action ID.
    pub fn action(mut self, action: Element) -> Self {
        self.action = Some(action);
        self
    }

    /// Add a close button at the trailing edge that reports `action_id` when clicked. The
    /// alert does not hide itself.
    pub fn dismiss(mut self, action_id: impl Into<String>) -> Self {
        self.dismiss_id = Some(action_id.into());
        self
    }

    /// Put the action below the message on narrow layouts.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl Component for Alert {
    fn render(self) -> Element {
        let colors = self
            .colors
            .unwrap_or_else(|| AlertColors::for_tone(self.tone, self.palette));
        let title = if self.title.trim().is_empty() {
            self.tone.label().to_owned()
        } else {
            self.title
        };
        let said = self.id.map(|id| {
            let said = Access::new(Role::Alert).id(id).name(title.clone());
            if self.description.trim().is_empty() {
                said
            } else {
                said.description(self.description.clone())
            }
        });
        let mut copy = vec![Element::text(title)
            .foreground(colors.title)
            .font_family(FontFamily::Sans)
            .font_size(14.0)];
        if !self.description.trim().is_empty() {
            copy.push(
                Element::text(self.description)
                    .foreground(colors.description)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0),
            );
        }
        let copy = Element::column(copy).gap(2.0);
        let copy = match (self.action, self.compact) {
            (Some(action), true) => Element::column(vec![copy, action]).gap(12.0),
            (Some(mut action), false) => {
                let width = action.style.width;
                let flex = action.style.flex;
                action.style.flex = 0.0;
                // The action holds the top corner beside the title.
                let action = Element::column(vec![action, Element::spacer().flex(1.0)])
                .size(width, None)
                .flex(flex);
                Element::row(vec![copy.flex(1.0), action]).gap(12.0)
            }
            (None, _) => copy,
        };
        let mut body = vec![
            Element::icon(self.tone.icon())
                .foreground(colors.accent)
                .size(Some(16.0), Some(16.0)),
            copy.flex(1.0),
        ];
        if let Some(id) = self.dismiss_id {
            let palette = ComponentPalette {
                surface: colors.surface,
                raised: colors.surface,
                hover: blend(colors.surface, colors.title, 20),
                selected: blend(colors.surface, colors.title, 36),
                border: colors.border,
                text: colors.title,
                muted: colors.description,
                ..self.palette
            };
            body.push(
                IconButton::new(id, Icon::CloseBold, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .tooltip(crate::labels().dismiss_notice)
                    // The tooltip is the direct row child. Its own square
                    // bounds keep a long message from stretching the button.
                    .render()
                    .accessibility_label(crate::labels().dismiss_notice)
                    .size(Some(28.0), Some(28.0)),
            );
        }
        // Uniform tree padding is expanded horizontally with two small slots,
        // giving the message 16 px horizontal and 12 px vertical breathing room.
        let padded = Element::row(vec![
            Element::spacer().size(Some(4.0), None),
            Element::row(body).gap(12.0).flex(1.0),
            Element::spacer().size(Some(4.0), None),
        ])
        .padding(12.0);
        let alert = Element::panel(padded)
            .background(colors.surface)
            .border(colors.border, 1.0)
            .radius(crate::tokens().radius.card)
            .min_size(0.0, 48.0)
            .clip_children();
        match said {
            Some(said) => alert.accessible(said),
            None => alert,
        }
    }
}

fn blend(from: Color, to: Color, amount: u16) -> Color {
    let channel =
        |a: u8, b: u8| ((u16::from(a) * (255 - amount) + u16::from(b) * amount + 127) / 255) as u8;
    Color::rgb(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
    )
}

fn luminance(color: Color) -> f32 {
    let channel = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

fn contrast(a: Color, b: Color) -> f32 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn readable_ink(candidate: Color, surface: Color, minimum: f32) -> Color {
    // A translucent ink is judged as it will look on this surface. Dropping
    // its alpha would turn a muted veil into full-strength text.
    let candidate = blend(
        surface,
        Color::rgb(candidate.r, candidate.g, candidate.b),
        u16::from(candidate.a),
    );
    if contrast(candidate, surface) >= minimum {
        return candidate;
    }
    let black = Color::rgb(0, 0, 0);
    let end = if contrast(Color::WHITE, surface) >= contrast(black, surface) {
        Color::WHITE
    } else {
        black
    };
    for amount in (8..=248).step_by(8) {
        let ink = blend(candidate, end, amount);
        if contrast(ink, surface) >= minimum {
            return ink;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Button, DrawCommand, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const TONES: [AlertTone; 5] = [
        AlertTone::Neutral,
        AlertTone::Info,
        AlertTone::Success,
        AlertTone::Warning,
        AlertTone::Error,
    ];

    fn light_palette() -> ComponentPalette {
        ComponentPalette {
            surface: Color::rgb(240, 240, 240),
            raised: Color::rgb(250, 250, 250),
            hover: Color::rgb(230, 230, 230),
            selected: Color::rgb(220, 220, 220),
            border: Color::rgb(205, 205, 205),
            text: Color::rgb(32, 32, 32),
            muted: Color::rgb(100, 100, 100),
            shadow: Color::rgba(0, 0, 0, 40),
        }
    }

    #[test]
    fn an_alert_with_an_id_is_an_alert_named_by_its_title_and_its_dismiss_button_has_a_name() {
        let palette = ComponentPalette::neutral_dark();
        let alert = Alert::new("Sin conexión", "Reintenta en un momento.", palette)
            .id("offline")
            .action(Button::new("retry", "Reintentar", palette).render())
            .dismiss("dismiss");
        let frame = UiTree::new(alert.render()).layout(Size::new(480.0, 200.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let said = node("offline");
        assert_eq!((said.role, said.name.as_str()), (Role::Alert, "Sin conexión"));
        assert_eq!(said.description.as_deref(), Some("Reintenta en un momento."));
        assert!(!said.actionable);
        assert_eq!((node("retry").role, node("retry").name.as_str()), (Role::Button, "Reintentar"));
        assert_eq!((node("dismiss").role, node("dismiss").name.as_str()), (Role::Button, crate::labels().dismiss_notice));
        // An alert without a title is named by its tone, as it is drawn; without an ID only its controls are there.
        let bare = UiTree::new(Alert::new("", "", palette).tone(AlertTone::Error).id("failed").render())
            .layout(Size::new(480.0, 200.0));
        assert_eq!((bare.access[0].name.as_str(), bare.access[0].description.as_deref()), ("Error", None));
        let unknown = UiTree::new(Alert::new("Listo", "", palette).dismiss("dismiss").render())
            .layout(Size::new(480.0, 200.0));
        assert_eq!(unknown.access.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(), ["dismiss"]);
    }

    fn contains(outer: Rect, inner: Rect) -> bool {
        inner.x >= outer.x
            && inner.y >= outer.y
            && inner.x + inner.width <= outer.x + outer.width + 0.1
            && inner.y + inner.height <= outer.y + outer.height + 0.1
    }

    #[test]
    fn default_colors_keep_copy_and_status_readable_in_both_themes() {
        for palette in [ComponentPalette::neutral_dark(), light_palette()] {
            for tone in TONES {
                let colors = AlertColors::for_tone(tone, palette);
                assert_eq!(colors.surface, palette.surface);
                assert_eq!(colors.border, palette.border, "the frame stays neutral");
                assert!(
                    contrast(colors.title, colors.surface) >= 4.5,
                    "title: {tone:?}"
                );
                assert!(
                    contrast(colors.description, colors.surface) >= 4.5,
                    "description: {tone:?}"
                );
                assert!(
                    contrast(colors.accent, colors.surface) >= 3.0,
                    "accent: {tone:?}"
                );
            }
        }
        // Theme detection alone does not guarantee contrast on intermediate
        // surfaces; unsuitable caller inks receive a measured correction.
        let palette = ComponentPalette {
            raised: Color::rgb(128, 128, 128),
            text: Color::rgb(130, 130, 130),
            muted: Color::rgb(140, 140, 140),
            ..light_palette()
        };
        for tone in TONES {
            let colors = AlertColors::for_tone(tone, palette);
            assert!(contrast(colors.title, colors.surface) >= 4.5);
            assert!(contrast(colors.description, colors.surface) >= 4.5);
            assert!(contrast(colors.accent, colors.surface) >= 3.0);
        }
    }

    #[test]
    fn passive_alerts_have_no_targets_and_communicate_tone_with_labels_and_icons() {
        let mut icons = std::collections::HashSet::new();
        let mut labels = std::collections::HashSet::new();
        for tone in TONES {
            let mut tree = UiTree::new(
                Alert::new("", "", ComponentPalette::neutral_dark())
                    .tone(tone)
                    .render(),
            );
            let frame = tree.layout(Size::new(300.0, 60.0));
            assert!(frame.regions.is_empty());
            assert_eq!(tree.focused_id(), None);
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == tone.label()
            )));
            let icon = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { icon, .. } => Some(*icon),
                    _ => None,
                })
                .unwrap();
            icons.insert(icon);
            labels.insert(tone.label());
        }
        // Neutral uses the information glyph with neutral ink and its own
        // label. Other semantic states retain distinct shapes as well.
        assert_eq!(icons.len(), 4);
        assert_eq!(labels.len(), TONES.len());
    }

    #[test]
    fn wrapped_copy_actions_and_dismiss_stay_inside_narrow_and_wide_alerts() {
        let palette = ComponentPalette::neutral_dark();
        let title = "No se pudo actualizar la configuración del proveedor seleccionado";
        let description = "La conexión no respondió. Revisa la configuración y vuelve a intentarlo cuando el servicio esté disponible; se conserva la selección guardada para continuar trabajando.";
        for compact in [false, true] {
            for width in [300.0, 640.0, 1000.0] {
                let alert = Alert::new(title, description, palette)
                    .tone(AlertTone::Error)
                    .compact(compact)
                    .action(
                        Button::new("retry", "Reintentar", palette)
                            .size(ControlSize::Regular)
                            .render(),
                    )
                    .dismiss("dismiss")
                    .render();
                let mut tree = UiTree::new(
                    Element::column(vec![
                        alert,
                        Element::button("following", "Siguiente").size(None, Some(28.0)),
                    ])
                    .gap(8.0),
                );
                let frame = tree.layout(Size::new(width, 1400.0));
                let surface = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, color, .. } if *color == palette.surface => {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(surface.height >= 48.0);
                let copy = frame.text_layouts();
                let description_block = copy
                    .iter()
                    .find(|block| block.source == description)
                    .unwrap();
                let description_bottom = description_block
                    .runs
                    .iter()
                    .map(|run| run.rect.y + run.rect.height)
                    .fold(0.0_f32, f32::max);
                assert!(copy.iter().any(|block| block.source == title));
                for block in copy
                    .iter()
                    .filter(|block| block.source == title || block.source == description)
                {
                    for run in &block.runs {
                        assert!(contains(surface, run.rect));
                    }
                }
                for id in ["retry", "dismiss"] {
                    let hit = frame.regions.iter().find(|region| region.id == id).unwrap();
                    assert!(
                        contains(surface, hit.rect),
                        "{id} escapes {width}, compact={compact}"
                    );
                }
                let retry = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "retry")
                    .unwrap();
                if compact {
                    assert!(retry.rect.y >= description_bottom + 12.0 - 0.1);
                }
                let close = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "dismiss")
                    .unwrap();
                assert_eq!(close.rect.width, 28.0);
                assert_eq!(close.rect.height, 28.0);
                let following = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "following")
                    .unwrap();
                assert!(following.rect.y >= surface.y + surface.height + 8.0 - 0.1);
            }
        }
    }

    #[test]
    fn explicit_controls_emit_original_ids_with_mouse_and_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Alert::new("Sin conexión", "Prueba de nuevo", palette)
                .action(Button::new("retry-original", "Reintentar", palette).render())
                .dismiss("dismiss-original")
                .compact(true)
                .render(),
        );
        let frame = tree.layout(Size::new(300.0, 180.0));
        assert_eq!(tree.focused_id(), None);
        let controls: Vec<_> = ["retry-original", "dismiss-original"]
            .into_iter()
            .map(|id| {
                (
                    id,
                    frame
                        .regions
                        .iter()
                        .find(|region| region.id == id)
                        .unwrap()
                        .rect,
                )
            })
            .collect();
        for (id, rect) in controls {
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: id.into() }]
            );
            assert!(tree.focus(id));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false,
                    alt: false
                }),
                vec![UiAction::Clicked { id: id.into() }]
            );
        }
        // Dismissal is controlled: the component remains until its caller hides it.
        assert!(tree
            .layout(Size::new(300.0, 180.0))
            .regions
            .iter()
            .any(|region| region.id == "dismiss-original"));
    }

    #[test]
    fn explicit_colors_override_tone_defaults() {
        let colors = AlertColors {
            surface: Color::rgb(18, 29, 40),
            border: Color::rgb(45, 56, 67),
            accent: Color::rgb(78, 189, 210),
            title: Color::WHITE,
            description: Color::rgb(198, 209, 220),
        };
        let mut tree = UiTree::new(
            Alert::new("Título", "Descripción", ComponentPalette::neutral_dark())
                .colors(colors)
                .tone(AlertTone::Warning)
                .render(),
        );
        let frame = tree.layout(Size::new(300.0, 100.0));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == colors.surface)));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::WarningTriangle, tint, .. } if *tint == colors.accent)));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Text { text, color, .. } if text == "Descripción" && *color == colors.description)));
    }

    #[test]
    fn alerts_wrap_full_unicode_copy_and_keep_actions_before_the_next_group() {
        let palette = ComponentPalette::neutral_dark();
        let title = "Recuperación del turno: revisión de herramientas y permisos pendientes";
        let description = "La herramienta pudo haber producido cambios. Verifica su efecto antes de continuar.\nEl historial conserva las instrucciones y el resultado observado, incluso con texto Unicode: recuperación, 終わり, 🦀. Un diagnóstico largo conserva el documento completo https://example.test/proyecto/instrucciones/archivo-con-nombre-largo-y-sin-espacios.";
        for (compact, width) in [(true, 280.0), (true, 360.0), (false, 640.0), (false, 900.0)] {
            let mut tree = UiTree::new(
                Element::column(vec![
                    Alert::new(title, description, palette)
                        .tone(AlertTone::Warning)
                        .compact(compact)
                        .action(
                            Button::new("review", "Revisar herramientas", palette)
                                .width(180.0)
                                .render(),
                        )
                        .render(),
                    Element::button("following", "Siguiente grupo")
                        .font_family(FontFamily::Sans)
                        .size(None, Some(32.0)),
                ])
                .gap(12.0),
            );
            let frame = tree.layout(Size::new(width, 1600.0));
            let blocks = frame.text_layouts();
            for source in [title, description] {
                let block = blocks.iter().find(|block| block.source == source).unwrap();
                assert!(!block.runs.is_empty());
                assert!(block.runs.iter().all(|run| run.visible.is_some()));
                for run in &block.runs {
                    assert!(
                        run.rect.x >= 0.0 && run.rect.x + run.rect.width <= width + 0.1,
                        "copy escaped width {width}: {run:?}"
                    );
                }
            }
            assert!(frame.commands.iter().all(|command| !matches!(command,
                DrawCommand::Text { font_family, .. } if *font_family != FontFamily::Sans
            )));
            let review = frame
                .regions
                .iter()
                .find(|region| region.id == "review")
                .unwrap();
            let following = frame
                .regions
                .iter()
                .find(|region| region.id == "following")
                .unwrap();
            let description_bottom = blocks
                .iter()
                .find(|block| block.source == description)
                .unwrap()
                .runs
                .iter()
                .map(|run| run.rect.y + run.rect.height)
                .fold(0.0, f32::max);
            if compact {
                assert!(review.rect.y >= description_bottom + 12.0 - 0.1);
            }
            assert_eq!(review.rect.width, 180.0);
            assert!(review.rect.x >= 0.0 && review.rect.x + review.rect.width <= width);
            assert!(following.rect.y >= review.rect.y + review.rect.height + 12.0);
            let point = Point::new(
                review.rect.x + review.rect.width / 2.0,
                review.rect.y + review.rect.height / 2.0,
            );
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "review".into()
                }]
            );
            assert!(tree.focus("review"));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false,
                    alt: false
                }),
                vec![UiAction::Clicked {
                    id: "review".into()
                }]
            );
        }
    }

    #[test]
    fn alerts_are_passive_and_preserve_a_disabled_caller_action() {
        let palette = ComponentPalette::neutral_dark();
        for disabled in [false, true] {
            let mut alert = Alert::new(
                "Permiso pendiente",
                "La herramienta aún no se ejecutó.",
                palette,
            );
            if disabled {
                alert = alert.action(
                    Button::new("approve", "Aprobar", palette)
                        .disabled()
                        .render(),
                );
            }
            let mut tree = UiTree::new(alert.render());
            let frame = tree.layout(Size::new(360.0, 200.0));
            assert!(frame.regions.is_empty());
            assert!(!tree.focus("approve"));
            let point = Point::new(50.0, 60.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            assert!(tree
                .handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false,
                    alt: false
                })
                .is_empty());
        }
    }

    #[test]
    fn alert_tones_use_semantic_icons_without_replacing_the_supplied_copy() {
        for tone in TONES {
            let mut tree = UiTree::new(
                Alert::new(
                    "Título exacto",
                    "Mensaje exacto",
                    ComponentPalette::neutral_dark(),
                )
                .tone(tone)
                .render(),
            );
            let frame = tree.layout(Size::new(360.0, 200.0));
            let expected_icon = match tone {
                AlertTone::Neutral | AlertTone::Info => Icon::InfoCircle,
                AlertTone::Success => Icon::CheckCircle,
                AlertTone::Warning => Icon::WarningTriangle,
                AlertTone::Error => Icon::ErrorCircle,
            };
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Icon { icon, .. } if *icon == expected_icon
            )));
            assert_eq!(
                frame
                    .text_layouts()
                    .iter()
                    .map(|block| block.source.as_str())
                    .collect::<Vec<_>>(),
                vec!["Título exacto", "Mensaje exacto"]
            );
            assert!(frame.regions.is_empty());
        }
    }

    #[test]
    fn only_an_error_tints_its_copy_and_no_tone_paints_a_stripe() {
        let palette = ComponentPalette::neutral_dark();
        for tone in TONES {
            let colors = AlertColors::for_tone(tone, palette);
            if tone == AlertTone::Error {
                assert_eq!(colors.title, colors.accent);
                assert_ne!(colors.description, palette.muted);
                assert_ne!(colors.description, colors.title, "the description sits a step back");
            } else {
                assert_eq!((colors.title, colors.description), (palette.text, palette.muted));
            }
            let alert = Alert::new("Título", "Descripción", palette).tone(tone).render();
            let frame = UiTree::new(alert).layout(Size::new(400.0, 80.0));
            // One fill, the surface: nothing else is painted behind the copy.
            let fills = frame
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::FillRect { .. }))
                .count();
            assert_eq!(fills, 1, "{tone:?}");
        }
    }
}
