//! A line or paragraph of text in one of the interface's type roles.

use crate::{Color, Component, ComponentPalette, Element, FontFamily, TextRole, Tooltip};

/// Passive text sized by a [`TextRole`] and colored from the palette. A host
/// whose design has a step between two roles names it with [`Text::size`].
pub struct Text {
    value: String,
    palette: ComponentPalette,
    role: TextRole,
    size: Option<f32>,
    ink: Color,
    mono: bool,
    truncate: bool,
    fade: Option<f32>,
    line_height: Option<f32>,
    tooltip: Option<String>,
}

impl Text {
    /// Caption-sized sans text in the palette's text color.
    pub fn new(value: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            value: value.into(),
            palette,
            role: TextRole::Caption,
            size: None,
            ink: palette.text,
            mono: false,
            truncate: false,
            fade: None,
            line_height: None,
            tooltip: None,
        }
    }

    /// A small monospaced label in the muted color, set above the group it names.
    pub fn eyebrow(value: impl Into<String>, palette: ComponentPalette) -> Self {
        Self::new(value, palette).role(TextRole::Micro).mono().muted()
    }

    /// The size, and for [`TextRole::Code`] the monospaced family.
    pub fn role(mut self, role: TextRole) -> Self {
        self.role = role;
        self
    }

    /// A size in pixels that is not on the scale, such as 10.5; the role
    /// still chooses the family. Values that are not finite and positive are
    /// ignored.
    pub fn font_size(mut self, px: f32) -> Self {
        if px.is_finite() && px > 0.0 {
            self.size = Some(px);
        }
        self
    }

    /// The former name of [`font_size`](Self::font_size).
    #[deprecated(since = "0.1.39", note = "renamed to `font_size`: `size` is a control's size elsewhere")]
    pub fn size(self, px: f32) -> Self {
        self.font_size(px)
    }

    /// Paint in the palette's muted color, for secondary copy.
    pub fn muted(mut self) -> Self {
        self.ink = self.palette.muted;
        self
    }

    /// Paint in a color that carries meaning the host owns, such as an
    /// error, a warning or a diff sign.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = color;
        self
    }

    /// Use the monospaced family whatever the role.
    pub fn mono(mut self) -> Self {
        self.mono = true;
        self
    }

    /// Keep to one line, ending in an ellipsis when the width runs out.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// Keep to one line, fading out over the last `width` pixels instead of
    /// showing an ellipsis. Takes precedence over [`Text::truncate`]. Values
    /// that are negative or not finite are ignored.
    pub fn fade(mut self, width: f32) -> Self {
        if width.is_finite() && width >= 0.0 {
            self.fade = Some(width);
        }
        self
    }

    /// Leading for wrapped paragraphs. Values that are not finite and
    /// positive are ignored.
    pub fn line_height(mut self, height: f32) -> Self {
        if height.is_finite() && height > 0.0 {
            self.line_height = Some(height);
        }
        self
    }

    /// Show the whole value in a tooltip, wrapped over as many lines as it
    /// needs, for text that [`Text::truncate`] may cut. `id` must be unique
    /// in the window. Ignored without `truncate`.
    pub fn tooltip(mut self, id: impl Into<String>) -> Self {
        self.tooltip = Some(id.into());
        self
    }
}

impl Component for Text {
    fn render(self) -> Element {
        let tooltip = self.tooltip.filter(|_| self.truncate).map(|id| (id, self.value.clone()));
        let element = match self.fade {
            Some(width) => Element::faded_text(self.value, width),
            None if self.truncate => Element::truncated_text(self.value),
            None => Element::text(self.value),
        };
        let family = if self.mono || self.role.monospaced() {
            FontFamily::Monospace
        } else {
            FontFamily::Sans
        };
        let mut element = element
            .font_family(family)
            .font_size(self.size.unwrap_or(self.role.size()))
            .foreground(self.ink);
        if let Some(height) = self.line_height {
            element = element.line_height(height);
        }
        match tooltip {
            // A tooltip title is one truncated line; only its description wraps.
            Some((id, value)) => Tooltip::new(element, id, "", self.palette)
                .description(value)
                .max_width(320.0)
                .render(),
            None => element,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, ElementKind, HitKind, Point, Size, UiEvent, UiTree};
    use web_time::Instant;

    const ROLES: [(TextRole, f32, FontFamily); 8] = [
        (TextRole::Display, 18.0, FontFamily::Sans),
        (TextRole::Heading, 16.0, FontFamily::Sans),
        (TextRole::Body, 14.0, FontFamily::Sans),
        (TextRole::Label, 13.0, FontFamily::Sans),
        (TextRole::Caption, 12.0, FontFamily::Sans),
        (TextRole::Small, 11.0, FontFamily::Sans),
        (TextRole::Micro, 10.0, FontFamily::Sans),
        (TextRole::Code, 14.0, FontFamily::Monospace),
    ];

    fn same(built: Element, by_hand: Element) {
        assert_eq!(format!("{built:?}"), format!("{by_hand:?}"));
    }

    #[test]
    fn every_role_renders_the_element_a_caller_would_write_by_hand() {
        let palette = ComponentPalette::neutral_dark();
        for (role, size, family) in ROLES {
            assert_eq!(role.size(), size);
            same(
                Text::new("Texto", palette).role(role).render(),
                Element::text("Texto").font_family(family).font_size(size).foreground(palette.text),
            );
            same(
                Text::new("Texto", palette).role(role).muted().truncate().render(),
                Element::truncated_text("Texto").font_family(family).font_size(size).foreground(palette.muted),
            );
        }
        same(
            Text::new("Texto", palette).render(),
            Element::text("Texto").font_family(FontFamily::Sans).font_size(12.0).foreground(palette.text),
        );
        let warning = Color::rgb(225, 169, 65);
        same(
            Text::new("Texto", palette).mono().ink(warning).fade(24.0).line_height(18.0).render(),
            Element::faded_text("Texto", 24.0)
                .font_family(FontFamily::Monospace)
                .font_size(12.0)
                .foreground(warning)
                .line_height(18.0),
        );
        // A size off the scale replaces the role's and nothing else.
        same(
            Text::new("Texto", palette).role(TextRole::Code).font_size(10.5).truncate().render(),
            Element::truncated_text("Texto")
                .font_family(FontFamily::Monospace)
                .font_size(10.5)
                .foreground(palette.text),
        );
        same(
            Text::eyebrow("GRUPO", palette).render(),
            Element::text("GRUPO").font_family(FontFamily::Monospace).font_size(10.0).foreground(palette.muted),
        );
    }

    #[test]
    fn bad_numbers_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        for bad in [f32::NAN, f32::INFINITY, -1.0] {
            same(
                Text::new("Texto", palette).fade(bad).line_height(bad).font_size(bad).render(),
                Text::new("Texto", palette).render(),
            );
        }
    }

    #[test]
    fn truncated_text_stays_inside_a_narrow_width_and_can_carry_its_full_value() {
        let palette = ComponentPalette::neutral_dark();
        let value = "Un título bastante más largo que su columna";
        // The tooltip needs truncation: text that wraps is already complete.
        assert!(matches!(
            Text::new(value, palette).tooltip("title").render().kind,
            ElementKind::Text(_)
        ));
        for width in [60.0, 120.0] {
            let text = Text::new(value, palette).truncate().tooltip("title").render();
            let mut tree = UiTree::new(Element::row(vec![text.flex(1.0)]));
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
            let frame = tree.layout(Size::new(width, 20.0));
            let painted = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .expect("the text is painted");
            assert!(painted.ends_with('…') && painted.chars().count() as f32 * 8.0 <= width);
            let region = frame.regions.iter().find(|region| region.id == "title").expect("hover region");
            assert_eq!(region.kind, HitKind::Hover);
        }
    }

    #[test]
    fn the_tooltip_of_truncated_text_shows_the_end_of_a_value_wider_than_itself() {
        let palette = ComponentPalette::neutral_dark();
        // 480 px at 8 px per character: wider than the 320 px tooltip.
        let value = "Un título bastante más largo que la columna que lo contiene fin";
        let text = Text::new(value, palette).truncate().tooltip("title").render();
        let filler = Element::column(vec![]).size(Some(280.0), None);
        let mut tree = UiTree::new(Element::row(vec![text.flex(1.0), filler]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
        let size = Size::new(400.0, 200.0);
        tree.layout(size);
        tree.handle_event(UiEvent::PointerMove(Point::new(40.0, 8.0)));
        let frame = tree.layout_at(size, Instant::now() + crate::tree::TOOLTIP_HOVER_DELAY * 2);
        let painted: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // The anchor is cut; the tooltip wraps and loses nothing.
        assert!(painted[0].ends_with('…'));
        assert!(painted.len() > 2 && painted.last().unwrap().ends_with("fin"));
        assert!(painted[1..].iter().all(|line| !line.contains('…')));
    }
    #[test]
    fn icon_only_tooltip_items_stay_on_one_line_without_their_labels() {
        let palette = ComponentPalette::neutral_dark();
        let items = |bare: bool| {
            ["Herramientas", "Visión", "Razonamiento", "Salida estructurada", "PDF"]
                .into_iter()
                .map(move |label| {
                    let item = crate::TooltipItem::new(label).icon(crate::Icon::Code);
                    if bare { item.icon_only() } else { item }
                })
        };
        let painted = |bare: bool| {
            let anchor = Element::clickable("anchor", Element::text("Modelo"))
                .size(Some(80.0), Some(20.0));
            let tooltip = crate::Tooltip::new(anchor, "anchor", "Modelo", palette)
                .max_width(320.0)
                .icon_row("Admite", items(bare))
                .render();
            let mut tree = UiTree::new(Element::column(vec![tooltip]));
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
            let size = Size::new(600.0, 400.0);
            tree.layout(size);
            tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
            let frame = tree.layout_at(size, Instant::now() + crate::tree::TOOLTIP_HOVER_DELAY * 2);
            let icons: Vec<f32> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(rect.y),
                    _ => None,
                })
                .collect();
            let labelled = frame.commands.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "Razonamiento"),
            );
            (icons, labelled)
        };
        let (labelled_rows, has_labels) = painted(false);
        assert!(has_labels && labelled_rows.iter().any(|y| *y != labelled_rows[0]));
        let (bare_rows, has_labels) = painted(true);
        assert_eq!(bare_rows.len(), 5);
        assert!(!has_labels && bare_rows.iter().all(|y| *y == bare_rows[0]));
    }

    #[test]
    fn a_press_on_a_hinted_passive_child_reaches_the_control_under_it() {
        let palette = ComponentPalette::neutral_dark();
        let glyph = crate::Tooltip::new(
            Element::icon(crate::Icon::Code).size(Some(16.0), Some(16.0)),
            "glyph",
            "Herramientas",
            palette,
        )
        .render();
        let row = Element::clickable("row", Element::row(vec![glyph])).size(Some(120.0), Some(24.0));
        let mut tree = UiTree::new(Element::column(vec![row]));
        let size = Size::new(200.0, 100.0);
        let frame = tree.layout(size);
        let point = Point::new(8.0, 8.0);
        // The hint's anchor is what the pointer hovers; the press is the row's.
        assert_eq!(frame.hit_test(point), Some("glyph"));
        let mut actions = tree.handle_event(UiEvent::PointerDown(point));
        actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
        assert_eq!(actions, vec![crate::UiAction::Clicked { id: "row".into() }]);
    }

}
