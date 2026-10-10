//! A left-aligned inline notice: what a pane has to say, set where its
//! content would be.

use crate::{
    page::px, Color, Component, ComponentPalette, Element, Icon, RadiusRole, Text, TextRole,
};

/// Density of a [`Notice`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoticeSizing {
    /// Side of the leading icon.
    pub icon_size: f32,
    /// Type role of the title.
    pub title_role: TextRole,
    /// Type role of the description.
    pub description_role: TextRole,
    /// Space between the icon, the copy and the trailing element.
    pub gap: f32,
    /// Space between the title, the description and the action.
    pub copy_gap: f32,
    /// Space around the content.
    pub padding: f32,
    /// A fixed height, which cuts each line of copy instead of wrapping it;
    /// `None` follows the copy.
    pub height: Option<f32>,
    /// Height of the box a cut title is centered in; `None` follows the font.
    pub title_height: Option<f32>,
    /// The same for the description.
    pub description_height: Option<f32>,
}

impl NoticeSizing {
    /// A status in a narrow panel or a list.
    pub const COMPACT: Self = Self {
        icon_size: 16.0,
        title_role: TextRole::Caption,
        description_role: TextRole::Small,
        gap: 8.0,
        copy_gap: 2.0,
        padding: 8.0,
        height: None,
        title_height: None,
        description_height: None,
    };

    /// A notice that stands in for the content of a page.
    pub const REGULAR: Self = Self {
        icon_size: 20.0,
        title_role: TextRole::Body,
        description_role: TextRole::Caption,
        gap: 12.0,
        copy_gap: 6.0,
        padding: 16.0,
        height: None,
        title_height: None,
        description_height: None,
    };
}

/// A passive notice aligned to the leading edge: an optional icon, a title,
/// a reason, and controls the host owns. Use [`crate::Empty`] for a centered
/// blank slate and [`crate::Alert`] for a toned banner.
pub struct Notice {
    title: String,
    description: String,
    icon: Option<Icon>,
    ink: Option<Color>,
    action: Option<Element>,
    trailing: Option<Element>,
    framed: bool,
    width: Option<f32>,
    sizing: NoticeSizing,
    palette: ComponentPalette,
}

impl Notice {
    /// A title alone, in the [`NoticeSizing::REGULAR`] profile.
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            icon: None,
            ink: None,
            action: None,
            trailing: None,
            framed: false,
            width: None,
            sizing: NoticeSizing::REGULAR,
            palette,
        }
    }

    /// A muted icon before the copy, level with its first line.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A muted line under the title saying why, or what to do.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Paint the title in a color that carries meaning the host owns, such
    /// as an error.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// Place a caller-owned control under the copy, across its width. Put it
    /// in a row before a flexible spacer to keep it at its own width.
    pub fn action(mut self, action: Element) -> Self {
        self.action = Some(action);
        self
    }

    /// Place a caller-owned control after the copy, at the trailing edge.
    pub fn trailing(mut self, trailing: Element) -> Self {
        self.trailing = Some(trailing);
        self
    }

    /// Fill the notice with the palette's surface and round it with the card
    /// radius, for a notice that sits over other content.
    pub fn framed(mut self) -> Self {
        self.framed = true;
        self
    }

    /// A fixed width. Values that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// The notice's density. [`NoticeSizing::REGULAR`] unless set.
    pub fn sizing(mut self, sizing: NoticeSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for Notice {
    fn render(self) -> Element {
        let sizing = self.sizing;
        let palette = self.palette;
        let height = sizing.height.filter(|height| height.is_finite());
        // A notice that cannot grow keeps each line of copy to one line.
        let line = |value: String, role: TextRole, ink: Color, line_height: Option<f32>| {
            let text = Text::new(value, palette).role(role).ink(ink);
            if height.is_none() {
                return text.render();
            }
            let text = text.truncate().render();
            match line_height {
                Some(line_height) => text.size(None, Some(px(line_height))),
                None => text,
            }
        };
        let mut copy = vec![line(
            self.title,
            sizing.title_role,
            self.ink.unwrap_or(palette.text),
            sizing.title_height,
        )];
        if !self.description.is_empty() {
            copy.push(line(
                self.description,
                sizing.description_role,
                palette.muted,
                sizing.description_height,
            ));
        }
        copy.extend(self.action);
        let mut content = Vec::new();
        if let Some(icon) = self.icon {
            let side = px(sizing.icon_size);
            content.push(
                Element::icon(icon)
                    .foreground(palette.muted)
                    .size(Some(side), Some(side)),
            );
        }
        content.push(Element::column(copy).gap(px(sizing.copy_gap)).flex(1.0));
        content.extend(self.trailing);
        let notice = Element::row(content)
            .gap(px(sizing.gap))
            .padding(px(sizing.padding))
            .size(self.width, height);
        if self.framed {
            notice
                .background(palette.surface)
                .radius(RadiusRole::Card.px())
        } else {
            notice
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Button, DrawCommand, Frame, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const TITLE: &str = "Todavía no instalaste skills";
    const WHY: &str = "Elige una del catálogo y dónde instalarla en este proyecto.";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn tree_of(notice: Notice) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            notice.render(),
            Element::button("after", "Después").size(None, Some(30.0)),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap().rect
    }

    fn texts(frame: &Frame) -> Vec<(&str, Point, f32, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, font_size, color, .. } => {
                    Some((text.as_str(), *position, *font_size, *color))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn presets_wrap_their_copy_beside_the_icon_at_any_width() {
        for (sizing, title_size, description_size) in
            [(NoticeSizing::COMPACT, 12.0, 11.0), (NoticeSizing::REGULAR, 14.0, 12.0)]
        {
            let mut heights = Vec::new();
            for width in [140.0, 360.0, 900.0] {
                let notice = Notice::new(TITLE, palette()).icon(Icon::Grid).description(WHY).sizing(sizing);
                let frame = tree_of(notice).layout(Size::new(width, 600.0));
                let icon = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Icon { rect, tint, .. } if *tint == palette().muted => Some(*rect),
                        _ => None,
                    })
                    .expect("muted icon");
                // The glyph is centered in a box of the profile's side.
                let center = sizing.padding + sizing.icon_size / 2.0;
                assert_eq!(
                    (icon.x + icon.width / 2.0, icon.y + icon.height / 2.0),
                    (center, center)
                );
                let copy_x = sizing.padding + sizing.icon_size + sizing.gap;
                let lines = texts(&frame);
                let lines: Vec<_> = lines.iter().filter(|line| line.0 != "Después").collect();
                assert_eq!((lines[0].1.x, lines[0].1.y), (copy_x, sizing.padding));
                assert_eq!((lines[0].2, lines[0].3), (title_size, palette().text));
                let last = lines.last().unwrap();
                assert_eq!((last.2, last.3), (description_size, palette().muted));
                assert!(lines.iter().all(|line| line.1.x == copy_x));
                // The next sibling starts under the last line and the padding.
                let after = region(&frame, "after");
                assert!(after.y >= last.1.y + description_size + sizing.padding);
                heights.push(after.y);
            }
            assert!(heights[0] > heights[2], "the copy wraps in a narrow pane");
        }
    }

    #[test]
    fn a_fixed_height_cuts_each_line_and_centers_it_in_its_box() {
        let sizing = NoticeSizing {
            height: Some(51.0),
            title_height: Some(17.0),
            description_height: Some(16.0),
            ..NoticeSizing::COMPACT
        };
        for width in [120.0, 480.0] {
            let notice = Notice::new(TITLE, palette()).icon(Icon::Grid).description(WHY).width(width).sizing(sizing);
            let frame = tree_of(notice).layout(Size::new(600.0, 300.0));
            assert_eq!(region(&frame, "after").y, 51.0);
            let lines = texts(&frame);
            assert_eq!(lines.len(), 3, "one run for the title, one for the description");
            // Boxes of 17 and 16 under a padding of 8, two pixels apart.
            assert!(lines[0].1.y > 8.0 && lines[0].1.y + 12.0 < 25.0);
            assert!(lines[1].1.y > 27.0 && lines[1].1.y + 11.0 < 43.0);
            if width == 120.0 {
                assert!(lines[0].0.ends_with('…') && lines[1].0.ends_with('…'));
            }
        }
        // Without boxes the lines follow the font and still stay on one line.
        let plain = NoticeSizing { height: Some(51.0), ..NoticeSizing::COMPACT };
        let frame = tree_of(Notice::new(TITLE, palette()).description(WHY).width(120.0).sizing(plain))
            .layout(Size::new(600.0, 300.0));
        assert_eq!(texts(&frame)[0].1.y, 8.0);
        assert_eq!(texts(&frame).len(), 3);
    }

    #[test]
    fn framed_fills_the_palette_surface_with_the_card_radius() {
        let palette = ComponentPalette { surface: Color::rgb(30, 31, 32), ..palette() };
        for width in [160.0, 700.0] {
            let frame_of = |notice: Notice| tree_of(notice.description(WHY)).layout(Size::new(width, 600.0));
            let plain = frame_of(Notice::new(TITLE, palette));
            let framed = frame_of(Notice::new(TITLE, palette).framed());
            let fill = framed
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, radius }
                        if *color == palette.surface && *radius == RadiusRole::Card.px() =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("surface fill");
            assert_eq!(fill, Rect::new(0.0, 0.0, width, region(&framed, "after").y));
            // The frame is one fill: no border, and the copy does not move.
            assert_eq!(framed.commands.len(), plain.commands.len() + 1);
            assert_eq!(texts(&framed), texts(&plain));
        }
    }

    #[test]
    fn ink_colors_the_title_only() {
        let danger = Color::rgb(220, 80, 80);
        let frame = tree_of(Notice::new(TITLE, palette()).description(WHY).ink(danger))
            .layout(Size::new(900.0, 300.0));
        let lines = texts(&frame);
        assert_eq!(lines[0].3, danger);
        assert_eq!(lines[1].3, palette().muted);
    }

    #[test]
    fn action_sits_under_the_copy_and_trailing_at_the_far_edge() {
        let button = |id: &str| Button::new(id, "Reintentar", palette()).render();
        for width in [220.0, 640.0] {
            let notice = Notice::new(TITLE, palette())
                .description(WHY)
                .action(Element::row(vec![button("retry"), Element::spacer().flex(1.0)]))
                .trailing(button("restart"));
            let mut tree = tree_of(notice);
            let frame = tree.layout(Size::new(width, 600.0));
            let (retry, restart) = (region(&frame, "retry"), region(&frame, "restart"));
            let padding = NoticeSizing::REGULAR.padding;
            assert_eq!(retry.x, padding);
            assert!(texts(&frame).iter().filter(|line| line.0 != "Reintentar" && line.0 != "Después")
                .all(|line| line.1.y < retry.y));
            assert!((restart.x + restart.width - (width - padding)).abs() < 0.01);
            assert!(retry.x + retry.width <= restart.x);
            assert!(region(&frame, "after").y >= retry.y + retry.height + padding);
            for (id, rect) in [("retry", retry), ("restart", restart)] {
                let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                tree.handle_event(UiEvent::PointerDown(point));
                assert_eq!(
                    tree.handle_event(UiEvent::PointerUp(point)),
                    vec![UiAction::Clicked { id: id.into() }]
                );
            }
        }
    }

    #[test]
    fn bad_numbers_never_reach_the_layout() {
        let bad = NoticeSizing {
            icon_size: f32::NAN,
            title_role: TextRole::Body,
            description_role: TextRole::Caption,
            gap: f32::NEG_INFINITY,
            copy_gap: -3.0,
            padding: f32::NAN,
            height: Some(f32::NAN),
            title_height: Some(f32::INFINITY),
            description_height: Some(-1.0),
        };
        let notice = Notice::new(TITLE, palette())
            .icon(Icon::Grid)
            .description(WHY)
            .width(f32::NAN)
            .width(f32::INFINITY)
            .sizing(bad);
        let rendered = notice.render();
        assert_eq!(rendered.style.width, None);
        assert_eq!(rendered.style.height, None);
        let mut tree = UiTree::new(Element::column(vec![rendered, Element::spacer().flex(1.0)]));
        let frame = tree.layout(Size::new(200.0, 300.0));
        assert!(frame.commands.iter().all(|command| match command {
            DrawCommand::Text { position, .. } => position.x.is_finite() && position.y.is_finite(),
            DrawCommand::Icon { rect, .. } => rect.x.is_finite() && rect.width.is_finite(),
            _ => true,
        }));
        assert_eq!(Notice::new(TITLE, palette()).width(-20.0).render().style.width, Some(0.0));
    }
}
