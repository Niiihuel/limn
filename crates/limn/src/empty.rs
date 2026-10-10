//! Shared empty state for pages, panes and filtered results.

use crate::{
    Button, ButtonSize, ButtonVariant, Component, ComponentPalette, Element, FontFamily, Icon,
};

pub struct Empty {
    title: String,
    description: String,
    icon: Option<Icon>,
    actions: Vec<(String, String, ButtonVariant)>,
    action_element: Option<Element>,
    palette: ComponentPalette,
    width: Option<f32>,
    requested_height: Option<f32>,
    compact: bool,
}

impl Empty {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            icon: None,
            actions: Vec::new(),
            action_element: None,
            palette,
            width: None,
            requested_height: None,
            compact: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn action(
        mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        variant: ButtonVariant,
    ) -> Self {
        self.actions.push((id.into(), label.into(), variant));
        self
    }

    /// Place a caller-owned control in the centered action row.
    /// This is useful when an empty state's action is more than a button.
    pub fn action_element(mut self, element: Element) -> Self {
        self.action_element = Some(element);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width.max(0.0));
        self
    }

    /// Set a minimum height; wrapped copy and actions can make the state taller.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.requested_height = Some(height.max(0.0));
        }
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }
}

impl Component for Empty {
    fn render(self) -> Element {
        // A blank slate is only its content: an icon, what is missing, why,
        // and the one thing to do next. It has no frame of its own, so it
        // sits on whatever surface the caller gives it.
        let minimum_height = if self.compact { 168.0 } else { 208.0 };
        let height = self
            .requested_height
            .unwrap_or(minimum_height)
            .max(minimum_height);
        let center = Element::centered_x;
        let mut content = Vec::new();
        if let Some(icon) = self.icon {
            let side = if self.compact { 22.0 } else { 26.0 };
            content.push(center(
                Element::icon(icon)
                    .foreground(self.palette.muted)
                    .size(Some(side), Some(side)),
            ));
            content.push(Element::spacer().size(None, Some(2.0)));
        }
        content.push(
            Element::text(self.title)
                .foreground(self.palette.text)
                .font_family(FontFamily::Sans)
                .font_size(if self.compact { 14.0 } else { 16.0 })
                .center_content(),
        );
        if !self.description.is_empty() {
            content.push(center(
                Element::text(self.description)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(if self.compact { 12.5 } else { 13.0 })
                    .center_content()
                    .max_width(COPY_WIDTH)
                    // In a pane narrower than the copy, wrap instead of overflowing.
                    .shrink_to_fit(),
            ));
        }
        if !self.actions.is_empty() || self.action_element.is_some() {
            let mut actions: Vec<Element> = self
                .actions
                .into_iter()
                .map(|(id, label, variant)| {
                    Button::new(id, label, self.palette)
                        .variant(variant)
                        .button_size(ButtonSize::Small)
                        .render()
                })
                .collect();
            if let Some(element) = self.action_element {
                actions.push(element);
            }
            content.push(Element::spacer().size(None, Some(4.0)));
            content.push(center(Element::row(actions).gap(8.0)));
        }
        let body = Element::column(content).gap(if self.compact { 6.0 } else { 8.0 });
        Element::panel(body.centered_y())
            .padding(if self.compact { 16.0 } else { 24.0 })
            .min_size(0.0, height)
            .size(self.width, None)
    }
}

/// The widest a line of explanation runs before it wraps.
const COPY_WIDTH: f32 = 440.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn wrapped_empty_grows_without_overlapping_its_action_or_next_sibling() {
        let palette = ComponentPalette::neutral_dark();
        for compact in [false, true] {
            for width in [360.0, 620.0, 1300.0] {
                let mut empty = Empty::new(
                    "Persistent memory across conversations is not available for this connection yet",
                    "The memory service and its editor are still unavailable. Reusable instructions are managed in Skills, while the active conversation retains its own turns and checkpoints. Open the existing instructions catalog to review the documents that can already be used by the current agent profile.",
                    palette,
                )
                .icon(Icon::Grid)
                .action("open-skills", "View skills", ButtonVariant::Default)
                .width(width)
                .height(150.0);
                if compact {
                    empty = empty.compact();
                }
                let mut tree = UiTree::new(
                    Element::column(vec![
                        empty.render(),
                        Element::button("following-group", "Following settings group")
                            .size(None, Some(36.0)),
                    ])
                    .gap(12.0),
                );
                let frame = tree.layout(Size::new(width, 1200.0));
                let action = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "open-skills")
                    .expect("real empty action");
                let following = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "following-group")
                    .expect("following group");
                // The blank slate paints no frame; it ends where the next group starts.
                let surface = crate::Rect::new(0.0, 0.0, width, following.rect.y - 12.0);
                assert!(
                    !frame.commands.iter().any(|command| matches!(
                        command,
                        DrawCommand::StrokeRect { rect, .. } if rect.width > width * 0.8
                    )),
                    "a blank slate has no outline"
                );
                let minimum = if compact { 168.0 } else { 208.0 };
                let padding = if compact { 16.0 } else { 24.0 };
                assert!(surface.height >= minimum);
                if width == 360.0 {
                    assert!(
                        surface.height > minimum,
                        "wrapped content should grow at {width}"
                    );
                }
                assert!(
                    action.rect.y + action.rect.height <= surface.y + surface.height - padding + 0.1,
                    "empty action escapes surface at {width}, compact={compact}: action={:?}, surface={surface:?}", action.rect
                );
                assert!(
                    following.rect.y >= action.rect.y + action.rect.height + padding + 12.0 - 0.1,
                    "empty action overlaps following group at {width}, compact={compact}"
                );
                let point = Point::new(
                    action.rect.x + action.rect.width / 2.0,
                    action.rect.y + action.rect.height / 2.0,
                );
                tree.handle_event(UiEvent::PointerDown(point));
                assert_eq!(
                    tree.handle_event(UiEvent::PointerUp(point)),
                    vec![UiAction::Clicked {
                        id: "open-skills".into()
                    }]
                );
            }
        }
    }

    #[test]
    fn empty_keeps_its_actions_without_a_frame() {
        let palette = ComponentPalette::neutral_dark();
        let empty = Empty::new("Sin proyectos", "Crea tu primer proyecto", palette)
            .icon(Icon::Folder)
            .action("create", "Crear proyecto", ButtonVariant::Default)
            .action("import", "Importar", ButtonVariant::Outline)
            .width(420.0)
            .render();
        assert!(!empty.style.dashed_border);
        let frame = UiTree::new(empty).layout(Size::new(420.0, 256.0));
        assert!(frame.regions.iter().any(|region| region.id == "create"));
        assert!(frame.regions.iter().any(|region| region.id == "import"));
    }

    #[test]
    fn compact_empty_keeps_its_action_inside_a_flexible_pane() {
        let palette = ComponentPalette::neutral_dark();
        let empty = Empty::new(
            "Sin archivos",
            "Abre un proyecto para ver sus archivos.",
            palette,
        )
        .compact()
        .icon(Icon::Folder)
        .action("open", "Abrir proyecto", ButtonVariant::Default)
        .render();
        assert_eq!(empty.style.width, None);
        assert_eq!(empty.style.height, None);
        assert_eq!(empty.style.min_height, 168.0);
        let frame = UiTree::new(empty).layout(Size::new(280.0, 228.0));
        let action = frame
            .regions
            .iter()
            .find(|region| region.id == "open")
            .unwrap();
        assert!(action.rect.y >= 0.0);
        assert!(action.rect.y + action.rect.height <= 228.0);
    }

    #[test]
    fn empty_centers_content_and_honors_minimum_height() {
        let palette = ComponentPalette::neutral_dark();
        for requested_height in [150.0_f32, 420.0] {
            let height = requested_height.max(168.0);
            let empty = Empty::new("No hay resultados", "Prueba otra búsqueda", palette)
                .icon(Icon::Search)
                .compact()
                .height(requested_height)
                .render();
            assert_eq!(empty.style.height, None);
            assert_eq!(empty.style.min_height, height);
            let frame = UiTree::new(empty).layout(Size::new(420.0, height));
            let icon = frame.commands.iter().find_map(|command| match command {
                crate::DrawCommand::Icon {
                    icon: Icon::Search,
                    rect,
                    ..
                } => Some(rect),
                _ => None,
            });
            let icon = icon.expect("empty icon");
            assert!((icon.x + icon.width / 2.0 - 210.0).abs() < 1.0);
            let title = frame.commands.iter().find_map(|command| match command {
                crate::DrawCommand::Text { position, text, .. } if text == "No hay resultados" => {
                    Some(position)
                }
                _ => None,
            });
            let title = title.expect("empty title");
            assert!(title.x > 80.0 && title.x < 210.0);
            assert!(title.y > height / 2.0 - 34.0 && title.y < height / 2.0 + 50.0);
        }
    }

    #[test]
    fn empty_keeps_caller_owned_action_in_centered_row() {
        let palette = ComponentPalette::neutral_dark();
        let empty = Empty::new("Sin conexión", "Vuelve a intentarlo", palette)
            .action_element(
                Button::new("retry", "Reintentar", palette)
                    .variant(ButtonVariant::Secondary)
                    .render(),
            )
            .width(420.0)
            .render();
        let frame = UiTree::new(empty).layout(Size::new(420.0, 256.0));
        let retry = frame
            .regions
            .iter()
            .find(|region| region.id == "retry")
            .expect("caller-owned empty action");
        assert!((retry.rect.x + retry.rect.width / 2.0 - 210.0).abs() < 1.0);
    }
}
