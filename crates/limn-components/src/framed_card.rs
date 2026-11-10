//! A card that stands on a frame a shade apart from it.

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
    /// Whether the last part is a row, which the next row keeps apart from.
    after_row: bool,
    foot: Option<Element>,
    flat: bool,
}

impl FramedCard {
    /// The frame around the card, and how far the card's content stands from
    /// the component's edge.
    pub const FRAME: f32 = 4.0;
    /// How far the card's content stands from the component's edge, in logical pixels.
    pub const INSET: f32 = 16.0;
    /// How far a row's fill reaches past its content on each side. It leaves
    /// the fill as far from the card's edge as the card's last row is from
    /// its foot, so a fill in a corner follows the corner.
    pub const REACH: f32 = 6.0;
    /// The height of a row, in logical pixels.
    pub const ROW_HEIGHT: f32 = 30.0;
    /// Between two rows, so their fills stay apart.
    pub const ROW_GAP: f32 = 2.0;
    /// Between a row's fill and the card's edge, on every side.
    const MARGIN: f32 = Self::INSET - Self::REACH - Self::FRAME;
    const FOOT_HEIGHT: f32 = 40.0;

    /// An empty card with no foot that fills the width it is given.
    pub fn new(palette: ComponentPalette) -> Self {
        Self {
            palette,
            width: None,
            body: vec![Element::spacer().size(None, Some(8.0))],
            after_row: false,
            foot: None,
            flat: false,
        }
    }

    /// For a host whose surface already is the card, such as a popover: no
    /// frame and no card are drawn, and the parts keep the same insets from
    /// the host's edge. Call it before adding parts.
    pub fn flat(mut self) -> Self {
        self.flat = true;
        self
    }

    /// The body and the foot apart, for a host that scrolls one and pins the
    /// other. Only for a [`FramedCard::flat`] card, which draws nothing
    /// around them.
    pub fn into_parts(self) -> (Element, Option<Element>) {
        let side = || Element::spacer().size(Some(Self::INSET), None);
        let foot = self.foot.map(|foot| {
            Element::row(vec![side(), foot.flex(1.0).centered_y(), side()])
                .size(None, Some(Self::FOOT_HEIGHT))
        });
        // The last row keeps clear of whatever the host puts under the body.
        let mut body = self.body;
        body.push(Element::spacer().size(None, Some(8.0)));
        (Element::column(body).size(self.width.map(px), None), foot)
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
        self.body.push(self.inset(content, Self::INSET));
        self.after_row = false;
        self
    }

    /// A row from [`FramedCard::row`], or a block of them: its fill reaches
    /// past the inset on both sides while its content keeps to it.
    pub fn rows(mut self, rows: Element) -> Self {
        // Two fills never touch: the fill of a row under the pointer and of
        // an open block beside it stay two shapes.
        if self.after_row {
            self.body.push(Element::spacer().size(None, Some(Self::ROW_GAP)));
        }
        self.body.push(self.inset(rows, Self::INSET - Self::REACH));
        self.after_row = true;
        self
    }

    /// Room between two parts.
    pub fn space(mut self, height: f32) -> Self {
        self.body.push(Element::spacer().size(None, Some(px(height))));
        self.after_row = false;
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
            .radius(Self::fill_radius())
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
            .radius(Self::fill_radius());
        if open {
            block.background(palette.hover)
        } else {
            block
        }
    }

    fn card_radius() -> f32 {
        crate::tokens().radius.panel
    }

    /// A fill's corner follows the card's from its margin inside it.
    fn fill_radius() -> f32 {
        (Self::card_radius() - Self::MARGIN).max(0.0)
    }

    fn inset(&self, element: Element, side: f32) -> Element {
        let frame = if self.flat { 0.0 } else { Self::FRAME };
        let side = || Element::spacer().size(Some(side - frame), None);
        Element::row(vec![side(), element.flex(1.0), side()])
    }
}

impl Component for FramedCard {
    fn render(self) -> Element {
        if self.flat {
            let (body, foot) = self.into_parts();
            return Element::column(std::iter::once(body).chain(foot).collect());
        }
        let palette = self.palette;
        let frame = || Element::spacer().size(Some(Self::FRAME), Some(Self::FRAME));
        let mut body = self.body;
        body.push(Element::spacer().size(None, Some(Self::MARGIN)));
        let mut parts = vec![
            frame(),
            Element::row(vec![
                frame(),
                Element::column(body)
                    .background(palette.surface)
                    .border(palette.border, 1.0)
                    .radius(Self::card_radius())
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
        // The frame has corners of its own, a frame's width outside the
        // card's, so it is a shape whatever the host does around it.
        Element::column(parts)
            .background(Color::rgba(hover.r, hover.g, hover.b, hover.a / 2))
            .radius(Self::card_radius() + Self::FRAME)
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
        // The frame is a shape of its own: its corners stand a frame's width outside the card's.
        let frame_radius = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. } if rect.x == 0.0 && rect.width == 320.0 => Some(*radius),
                _ => None,
            })
            .expect("the frame paints its tint");
        assert_eq!(frame_radius, crate::tokens().radius.panel + FramedCard::FRAME);
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

    #[test]
    fn a_flat_card_draws_no_card_of_its_own_and_gives_its_foot_apart() {
        let card = || {
            FramedCard::new(palette())
                .flat()
                .width(320.0)
                .section(label("Header"))
                .rows(FramedCard::row(
                    "row:fill",
                    Element::clickable("row", Element::row(vec![label("Row")])),
                    ["row"],
                    palette(),
                ))
                .foot(label("Foot"))
        };
        let (body, foot) = card().into_parts();
        let foot = foot.expect("the foot comes apart from the body");
        let size = Size::new(400.0, 300.0);
        // The host places each part itself: here the body, then the foot.
        let mut tree = UiTree::new(Element::row(vec![
            Element::column(vec![body, foot]).size(Some(320.0), None),
            Element::spacer().flex(1.0),
        ]));
        let frame = tree.layout(size);
        let x = |wanted: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == wanted => Some(position.x),
                _ => None,
            })
        };
        // The same insets as on a frame, now from the host's own edge.
        assert_eq!((x("Header"), x("Row"), x("Foot")), (Some(16.0), Some(16.0), Some(16.0)));
        // Nothing is painted around the content: no card and no frame.
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { .. })));
        // Rendered whole, it is the body with the foot under it.
        let mut whole = UiTree::new(Element::row(vec![card().render(), Element::spacer().flex(1.0)]));
        let frame = whole.layout(size);
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { .. })));
    }
}
