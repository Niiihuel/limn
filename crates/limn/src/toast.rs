//! A brief, caller-timed card announcing an event in a corner of the window.

use crate::{
    AlertColors, AlertTone, ButtonSize, ButtonVariant, Component, ComponentPalette, Element,
    FontFamily, Icon, IconButton, Point,
};

const EDGE_INSET: f32 = 12.0;

/// One announcement. The host decides when it appears and when it goes away;
/// the card itself has no timer and never takes keyboard focus.
pub struct Toast {
    title: String,
    description: String,
    tone: AlertTone,
    palette: ComponentPalette,
    action: Option<String>,
    dismiss: Option<String>,
    width: f32,
}

impl Toast {
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            tone: AlertTone::Neutral,
            palette,
            action: None,
            dismiss: None,
            width: 360.0,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Colours the title; the surface stays neutral.
    pub fn tone(mut self, tone: AlertTone) -> Self {
        self.tone = tone;
        self
    }

    /// Makes the card's copy a single click target dispatching `id`.
    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }

    /// Adds a close button dispatching `id`.
    pub fn dismissible(mut self, id: impl Into<String>) -> Self {
        self.dismiss = Some(id.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(120.0);
        }
        self
    }

    /// Floats `toasts` over `base`, stacked upward from the bottom right
    /// corner and `bottom_inset` above the window's lower edge. Everything
    /// outside the cards stays interactive.
    pub fn layer(base: Element, toasts: Vec<Element>, bottom_inset: f32) -> Element {
        if toasts.is_empty() {
            return base;
        }
        Element::overlay(
            base,
            Element::column(vec![
                Element::spacer().flex(1.0),
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    Element::column(toasts).gap(8.0),
                    Element::spacer().size(Some(EDGE_INSET), None),
                ]),
                Element::spacer().size(None, Some(bottom_inset.max(0.0))),
            ]),
        )
    }
}

impl Component for Toast {
    fn render(self) -> Element {
        // Identified by its title: the card fades in when it appears and out
        // in place when the host drops it.
        let presence = format!("toast:{}", self.title);
        let colors = AlertColors::for_tone(self.tone, self.palette);
        let line = |value: String, size: f32, color| {
            Element::truncated_text(value)
                .font_family(FontFamily::Sans)
                .font_size(size)
                .foreground(color)
        };
        let title_ink = if self.tone == AlertTone::Neutral {
            colors.title
        } else {
            colors.accent
        };
        let mut copy = vec![line(self.title, 13.0, title_ink)];
        if !self.description.trim().is_empty() {
            copy.push(line(self.description, 12.0, self.palette.muted));
        }
        let copy = Element::column(copy).gap(3.0).padding(12.0);
        let copy = match self.action {
            Some(id) => self
                .palette
                .interactive_surface(Element::clickable(id, copy)),
            None => copy,
        };
        let mut cells = vec![copy.flex(1.0)];
        if let Some(id) = self.dismiss {
            // The row stretches its cells, so the button is held in the middle
            // of the card: its hover square stays centered on the glyph and on
            // the copy beside it, whatever the card's height.
            cells.push(Element::column(vec![
                Element::spacer().flex(1.0),
                IconButton::new(id, Icon::Close, self.palette)
                    .button_size(ButtonSize::IconSmall)
                    .variant(ButtonVariant::Ghost)
                    .render(),
                Element::spacer().flex(1.0),
            ]));
            cells.push(Element::spacer().size(Some(8.0), None));
        }
        Element::row(cells)
            .gap(2.0)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(crate::components::surface_radius())
            .shadow(self.palette.shadow, Point::new(0.0, 3.0), 10.0, 0.0)
            .size(Some(self.width), None)
            .animate_presence(presence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Size, UiAction, UiEvent, UiTree};

    fn texts(frame: &Frame) -> Vec<&str> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }

    fn base() -> Element {
        Element::clickable("base", Element::spacer()).size(Some(800.0), Some(500.0))
    }

    #[test]
    fn cards_stack_in_the_bottom_right_corner_without_blocking_the_window() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(800.0, 500.0);
        let untouched = UiTree::new(Toast::layer(base(), Vec::new(), 36.0)).layout(size);
        assert_eq!(untouched.regions.len(), 1);

        let toasts = vec![
            Toast::new("Turno fallido", palette)
                .description("Migrar la base")
                .tone(AlertTone::Error)
                .action("open:1")
                .dismissible("close:1")
                .render(),
            Toast::new("Turno completado", palette)
                .action("open:2")
                .render(),
        ];
        let mut tree = UiTree::new(Toast::layer(base(), toasts, 36.0));
        let frame = tree.layout(size);
        assert_eq!(texts(&frame), ["Turno fallido", "Migrar la base", "Turno completado"]);
        let rect = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
        let (first, second) = (rect("open:1"), rect("open:2"));
        assert!(first.y + first.height <= second.y, "cards do not overlap");
        assert!(second.y + second.height <= 500.0 - 36.0);
        for card in [first, second] {
            assert!(card.x >= 800.0 - EDGE_INSET - 360.0 && card.x + card.width <= 800.0 - EDGE_INSET);
        }
        assert_eq!(frame.hit_test(Point::new(100.0, 100.0)), Some("base"));
        assert!(!frame.regions.iter().any(|region| region.id == "close:2"));

        // The error title takes the status colour; neutral copy keeps the text ink.
        let ink = |label: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == label => Some(*color),
                _ => None,
            })
        };
        assert_ne!(ink("Turno fallido"), ink("Turno completado"));

        for id in ["open:1", "close:1", "open:2"] {
            let target = rect(id);
            let point = Point::new(target.x + target.width / 2.0, target.y + target.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                [UiAction::Clicked { id: id.into() }]
            );
        }
    }

    #[test]
    fn a_card_without_actions_is_passive_and_respects_its_width() {
        let card = Toast::new("Guardado", ComponentPalette::neutral_dark())
            .description("   ")
            .width(40.0)
            .render();
        let mut tree = UiTree::new(Toast::layer(base(), vec![card], 0.0));
        // Checked at rest, not partway through its entrance.
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(800.0, 500.0));
        assert_eq!(texts(&frame), ["Guardado"]);
        assert_eq!(frame.regions.len(), 1, "only the base is interactive");
        let surface = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. }
                    if *radius == crate::components::surface_radius() =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(surface.width, 120.0);
        assert!((surface.y + surface.height - 500.0).abs() < 0.01);
    }

    #[test]
    fn a_card_fades_in_and_lingers_fading_once_the_host_drops_it() {
        use std::time::{Duration, Instant};
        let palette = ComponentPalette::neutral_dark();
        let shown = || {
            Toast::layer(base(), vec![Toast::new("Guardado", palette).render()], 0.0)
        };
        // Opacity of the card's surface, or none when it is not painted.
        let surface = |frame: &crate::Frame| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { color, radius, .. }
                    if *radius == crate::components::surface_radius() =>
                {
                    Some(color.a)
                }
                _ => None,
            })
        };
        let size = Size::new(800.0, 500.0);
        let start = Instant::now();
        let after = |ms| start + Duration::from_millis(ms);
        let mut tree = UiTree::new(shown());
        assert_eq!(surface(&tree.layout_at(size, start)), Some(0));
        let entering = surface(&tree.layout_at(size, after(40))).unwrap();
        assert!(entering > 0 && entering < 255, "{entering}");
        assert_eq!(surface(&tree.layout_at(size, after(200))), Some(255));

        tree.set_root(base());
        // The frame that drops the card still shows it whole, then it fades.
        assert_eq!(surface(&tree.layout_at(size, after(240))), Some(255));
        let leaving = surface(&tree.layout_at(size, after(290))).unwrap();
        assert!(leaving > 0 && leaving < 255, "{leaving}");
        assert_eq!(surface(&tree.layout_at(size, after(500))), None);

        // Reduced motion shows and removes the card at once.
        let mut still = UiTree::new(shown());
        still.set_reduced_motion(true);
        assert_eq!(surface(&still.layout_at(size, start)), Some(255));
        still.set_root(base());
        assert_eq!(surface(&still.layout_at(size, after(10))), None);
    }
}
