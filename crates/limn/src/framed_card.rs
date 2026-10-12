//! A card that stands on a frame a shade apart from it.

use crate::components::control_radius;
use crate::page::px;
use crate::{Collapse, Color, Component, ComponentPalette, Element};

/// The parts stack inside the card; the foot sits on the frame below it, so
/// a status and an action read as belonging to the whole card rather than to
/// its last row. It has no surface of its own: the host gives it one with
/// rounded corners and no padding, such as a popover or a panel.
pub struct FramedCard {
    palette: ComponentPalette,
    width: Option<f32>,
    body: Vec<Element>,
    foot: Option<Element>,
}

impl FramedCard {
    /// The frame around the card, and how far the card's content stands from
    /// the component's edge.
    pub const FRAME: f32 = 4.0;
    pub const INSET: f32 = 16.0;
    /// How far a row's fill reaches past its content on each side.
    pub const REACH: f32 = 10.0;
    pub const ROW_HEIGHT: f32 = 30.0;
    const RADIUS: f32 = 12.0;
    const FOOT_HEIGHT: f32 = 40.0;

    pub fn new(palette: ComponentPalette) -> Self {
        Self {
            palette,
            width: None,
            body: vec![Element::spacer().size(None, Some(8.0))],
            foot: None,
        }
    }

    /// Without a width the card fills the space it is given.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(Self::INSET * 4.0));
        }
        self
    }

    /// Content that keeps to the card's inset: a header, a figure, a paragraph.
    pub fn section(mut self, content: Element) -> Self {
        self.body.push(Self::inset(content, Self::INSET));
        self
    }

    /// A row from [`FramedCard::row`], or a block of them: its fill reaches
    /// past the inset on both sides while its content keeps to it.
    pub fn rows(mut self, rows: Element) -> Self {
        self.body.push(Self::inset(rows, Self::INSET - Self::REACH));
        self
    }

    /// Room between two parts.
    pub fn space(mut self, height: f32) -> Self {
        self.body.push(Element::spacer().size(None, Some(px(height))));
        self
    }

    /// What sits on the frame under the card, inset like the card's content.
    pub fn foot(mut self, foot: Element) -> Self {
        self.foot = Some(foot);
        self
    }

    /// One row for [`FramedCard::rows`]: `content` between the pads its
    /// hover fill reaches by. `actions` are the IDs of the controls inside
    /// it, which share that one fill; `id` names the fill.
    pub fn row(
        id: impl Into<String>,
        content: Element,
        actions: impl IntoIterator<Item = impl Into<String>>,
        palette: ComponentPalette,
    ) -> Element {
        let pad = || Element::spacer().size(Some(Self::REACH), None);
        Element::row(vec![pad(), content.flex(1.0), pad()])
            .size(None, Some(Self::ROW_HEIGHT))
            .radius(control_radius())
            .hover_background(palette.hover)
            .hover_surface(id, actions)
    }

    /// A row and what it opens to list, for [`FramedCard::rows`]. They read
    /// as one block while it is open. Render it on every rebuild and pass
    /// `listed` while the row is open, so it opens and closes in place.
    pub fn block(
        id: impl Into<String>,
        row: Element,
        listed: Option<Element>,
        palette: ComponentPalette,
    ) -> Element {
        let open = listed.is_some();
        let block = Element::column(vec![row, Collapse::new(id, listed).expand().render()])
            .radius(control_radius());
        if open {
            block.background(palette.hover)
        } else {
            block
        }
    }

    fn inset(element: Element, side: f32) -> Element {
        let side = || Element::spacer().size(Some(side - Self::FRAME), None);
        Element::row(vec![side(), element.flex(1.0), side()])
    }
}

impl Component for FramedCard {
    fn render(self) -> Element {
        let palette = self.palette;
        let frame = || Element::spacer().size(Some(Self::FRAME), Some(Self::FRAME));
        let mut body = self.body;
        body.push(Element::spacer().size(None, Some(8.0)));
        let mut parts = vec![
            frame(),
            Element::row(vec![
                frame(),
                Element::column(body)
                    .background(palette.surface)
                    .border(palette.border, 1.0)
                    .radius(Self::RADIUS)
                    .flex(1.0),
                frame(),
            ]),
        ];
        match self.foot {
            Some(foot) => {
                let side = || Element::spacer().size(Some(Self::INSET), None);
                parts.push(
                    Element::row(vec![side(), foot.flex(1.0).centered_y(), side()])
                        .size(None, Some(Self::FOOT_HEIGHT)),
                );
            }
            None => parts.push(frame()),
        }
        // The frame is the host's surface under a veil of the hover shade.
        let hover = palette.hover;
        Element::column(parts)
            .background(Color::rgba(hover.r, hover.g, hover.b, hover.a / 2))
            .size(self.width.map(px), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, FontFamily, Point, Size, UiEvent, UiTree};

    const SURFACE: Color = Color::rgba(255, 255, 255, 255);
    const HOVER: Color = Color::rgba(220, 220, 220, 255);

    fn palette() -> ComponentPalette {
        ComponentPalette {
            surface: SURFACE,
            raised: SURFACE,
            hover: HOVER,
            selected: HOVER,
            border: Color::rgba(200, 200, 200, 255),
            text: Color::rgba(20, 20, 20, 255),
            muted: Color::rgba(120, 120, 120, 255),
            shadow: Color::rgba(0, 0, 0, 40),
        }
    }

    fn label(text: &str) -> Element {
        Element::text(text).font_family(FontFamily::Sans).font_size(13.0)
    }

    #[test]
    fn a_rows_fill_reaches_past_the_content_and_the_foot_stands_under_the_card() {
        let card = FramedCard::new(palette())
            .width(320.0)
            .section(label("Header"))
            .rows(FramedCard::row(
                "row:fill",
                Element::clickable("row", Element::row(vec![label("Row")])),
                ["row"],
                palette(),
            ))
            .foot(label("Foot"))
            .render();
        // Beside a spacer, as in a host: a root element is given the whole window.
        let mut tree = UiTree::new(Element::row(vec![card, Element::spacer().flex(1.0)]));
        let size = Size::new(400.0, 300.0);
        let frame = tree.layout(size);
        let x = |wanted: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == wanted => Some(*position),
                _ => None,
            })
        };
        // A section, a row's content and the foot all start at the inset.
        let (header, row, foot) = (x("Header").unwrap(), x("Row").unwrap(), x("Foot").unwrap());
        assert_eq!((header.x, row.x, foot.x), (FramedCard::INSET, FramedCard::INSET, FramedCard::INSET));
        // The card ends above the foot: its fill does not reach the foot's line.
        let card = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == SURFACE => Some(*rect),
                _ => None,
            })
            .expect("the card paints its surface");
        assert_eq!((card.x, card.width), (FramedCard::FRAME, 320.0 - FramedCard::FRAME * 2.0));
        assert!(foot.y > card.y + card.height, "foot {foot:?} card {card:?}");

        // Under the pointer the row's fill reaches past its content on both sides.
        let region = frame.regions.iter().find(|region| region.id == "row").unwrap().rect;
        tree.handle_event(UiEvent::PointerMove(Point::new(region.x + 4.0, region.y + 4.0)));
        tree.set_reduced_motion(true);
        let frame = tree.layout(size);
        let fill = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == HOVER => Some(*rect),
                _ => None,
            })
            .expect("the hovered row paints its fill");
        assert_eq!(fill.x, FramedCard::INSET - FramedCard::REACH);
        assert_eq!(fill.x + fill.width, 320.0 - (FramedCard::INSET - FramedCard::REACH));
        assert_eq!(fill.height, FramedCard::ROW_HEIGHT);
    }
}
