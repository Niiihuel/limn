//! A diagram over the whole window, panned in both directions.

use crate::{Color, Component, ComponentPalette, Diagram, Element, FontFamily, Icon, PopoverPlacement, Size};

/// `chart`, a flowchart or a sequence diagram, at its natural size in a
/// panel that covers the window, as [`crate::ImageViewer`] shows a picture:
/// the wheel moves it down, a drag or a sideways gesture moves it across.
/// The host opens it by rendering it and closes it on `close_id`, which
/// Escape and the backdrop also send.
pub struct DiagramViewer {
    base: Element,
    title: String,
    close_title: Option<String>,
    close_id: String,
    palette: ComponentPalette,
    viewport: Size,
    chart: Diagram,
}

impl DiagramViewer {
    /// A diagram shown over `base`, under this title, with a control that
    /// reports `close_id`. The host shows it while its own flag is set.
    pub fn new(
        base: Element,
        title: impl Into<String>,
        close_id: impl Into<String>,
        palette: ComponentPalette,
        chart: impl Into<Diagram>,
    ) -> Self {
        Self {
            base,
            title: title.into(),
            close_title: None,
            close_id: close_id.into(),
            palette,
            viewport: Size::new(900.0, 620.0),
            chart: chart.into(),
        }
    }

    /// The hint of the close control. Without one it is the library's own
    /// word for closing and the key that does it.
    pub fn close_title(mut self, title: impl Into<String>) -> Self {
        self.close_title = Some(title.into());
        self
    }

    /// The size of the window the viewer stands in: its panel takes that less
    /// a margin. 900 by 620 unless set; a size that is not finite is ignored.
    pub fn viewport(mut self, size: Size) -> Self {
        if size.width.is_finite() && size.height.is_finite() {
            self.viewport = Size::new(size.width.max(0.0), size.height.max(0.0));
        }
        self
    }
}

impl Component for DiagramViewer {
    fn render(self) -> Element {
        let close_title = self.close_title.clone().unwrap_or_else(|| format!("{} · Esc", crate::labels().close));
        let mut controls = self.palette;
        controls.surface = self.palette.hover;
        let header = Element::row(vec![
            Element::truncated_text(&self.title)
                .font_family(FontFamily::Sans)
                .font_size(14.0)
                .foreground(self.palette.text)
                .flex(1.0)
                .size(None, Some(36.0)),
            crate::image_viewer::control(
                &self.close_id,
                Icon::Close,
                &close_title,
                &close_title,
                true,
                controls,
            ),
        ])
        .gap(12.0)
        .size(None, Some(36.0));
        let chart = Element::scroll(
            format!("{}:down", self.close_id),
            self.chart.scrollable(format!("{}:across", self.close_id)),
        )
        .flex(1.0);
        let viewer = Element::panel(Element::column(vec![header, chart]).gap(12.0))
            .padding(16.0)
            // The fill of the card the chart was drawn on in the text.
            .background(self.palette.raised)
            .radius(crate::tokens().radius.panel)
            .clip_children()
            .size(
                Some((self.viewport.width - 16.0).max(0.0)),
                Some((self.viewport.height - 16.0).max(0.0)),
            )
            // Known by an ID of its own: `close_id` is its close control's.
            .accessible(
                crate::Access::new(crate::Role::Dialog)
                    .id(format!("{}:dialog", self.close_id))
                    .name(&self.title),
            );
        Element::popover(self.base, viewer, &self.close_id, PopoverPlacement::Center)
            .popover_scrim(Color::rgba(0, 0, 0, 230))
            .popover_animation_ms(crate::tokens().motion.enter_ms)
            .popover_backdrop_dismiss(true)
            .popover_initial_focus(&self.close_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Flowchart, Point, UiAction, UiEvent, UiTree};

    #[test]
    fn the_viewer_is_a_dialog_named_by_its_title_and_its_close_control_by_its_hint() {
        let palette = ComponentPalette::neutral_dark();
        let chart = Flowchart::parse("flowchart LR\n A[Uno] --> B[Dos]", palette).unwrap();
        let viewer = DiagramViewer::new(Element::spacer(), "Diagrama", "viewer_close", palette, chart)
            .close_title("Cerrar")
            .render();
        let mut tree = UiTree::new(viewer);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(900.0, 620.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let dialog = node("viewer_close:dialog");
        assert_eq!((dialog.role, dialog.name.as_str()), (crate::Role::Dialog, "Diagrama"));
        let close = node("viewer_close");
        assert_eq!((close.role, close.name.as_str()), (crate::Role::Button, "Cerrar"));
        assert!(close.actionable && !dialog.actionable);
    }

    #[test]
    fn a_wide_chart_is_dragged_across_and_the_close_control_closes() {
        let palette = ComponentPalette::neutral_dark();
        let source = "flowchart LR\n A[Uno uno uno] --> B[Dos dos dos] --> C[Tres tres tres] --> D[Cuatro cuatro] --> E[Cinco cinco cinco]";
        let viewer = DiagramViewer::new(
            Element::spacer(),
            "Diagrama",
            "viewer_close",
            palette,
            Flowchart::parse(source, palette).unwrap(),
        )
        .viewport(Size::new(360.0, 300.0))
        .render();
        let size = Size::new(360.0, 300.0);
        let mut tree = UiTree::new(viewer);
        tree.set_reduced_motion(true);
        let first = |tree: &mut UiTree| {
            tree.layout(size).commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text.starts_with("Uno") => Some(position.x),
                _ => None,
            })
        };
        let before = first(&mut tree).unwrap();
        tree.handle_event(UiEvent::PointerDown(Point::new(200.0, 150.0)));
        tree.handle_event(UiEvent::PointerMove(Point::new(120.0, 150.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(120.0, 150.0)));
        assert_eq!(first(&mut tree).unwrap(), before - 80.0, "the chart follows the pointer");
        let frame = tree.layout(size);
        let close = frame
            .regions
            .iter()
            .rev()
            .find(|region| region.id == "viewer_close" && region.kind == crate::HitKind::Button)
            .unwrap()
            .rect;
        let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), [UiAction::Clicked { id: "viewer_close".into() }]);
    }
}
