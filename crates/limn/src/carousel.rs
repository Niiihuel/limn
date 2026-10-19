//! A shelf of items that scrolls sideways inside a page that scrolls down,
//! under an optional heading: the rows of a music app's home page.

use crate::{
    tokens, Access, Component, ComponentPalette, Element, FontFamily, Role, ScrollbarVisibility, Text, TextRole,
};

const DEFAULT_GAP: f32 = 24.0;
const TITLE_SIZE: f32 = 24.0;
/// Side of the round picture beside the heading.
const LEADING_SIDE: f32 = 48.0;
/// Between the picture, the heading and the trailing element.
const HEADER_GAP: f32 = 12.0;
/// Between the eyebrow and the title.
const EYEBROW_GAP: f32 = 2.0;
/// Between the heading and the items.
const SHELF_GAP: f32 = 8.0;
/// Room under the items for the 3 px indicator, so it never lies on an
/// item's hover fill.
const INDICATOR_ROOM: f32 = 8.0;

/// A row of host-built items, such as [`crate::MediaTile`]s, that moves
/// sideways with the pointer: a drag anywhere on it pans it and, let go with
/// speed, it glides on and slows to rest. The trackpad's sideways scroll and
/// Shift with the wheel pan it too; the plain wheel scrolls the page. There
/// are no arrow buttons. The items report their own actions; a press that
/// becomes a drag clicks none of them.
///
/// The tree keeps where the row stands under the carousel's ID, so a host
/// can read it with [`crate::UiTree::horizontal_scroll_offset`] or put it
/// back to the start with [`crate::UiTree::set_horizontal_scroll_offset`].
pub struct Carousel {
    id: String,
    items: Vec<Element>,
    title: Option<String>,
    eyebrow: Option<String>,
    leading: Option<Element>,
    trailing: Option<Element>,
    gap: f32,
    inset: f32,
    palette: ComponentPalette,
}

impl Carousel {
    /// `items` in order, 24 apart, with no heading. `id` must be unique in
    /// the window.
    pub fn new(id: impl Into<String>, items: Vec<Element>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items,
            title: None,
            eyebrow: None,
            leading: None,
            trailing: None,
            gap: DEFAULT_GAP,
            inset: 0.0,
            palette,
        }
    }

    /// The heading over the row, 24 px bold on one line, cut with an
    /// ellipsis when the width runs out. The platform is told it is a heading.
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// A small muted line over the title that says what the row is to the
    /// reader, as "Made for Ana" over "Your mixes".
    pub fn eyebrow(mut self, text: impl Into<String>) -> Self {
        self.eyebrow = Some(text.into());
        self
    }

    /// A picture beside the heading, held round in 48 × 48, such as the
    /// artist a row is about.
    pub fn leading(mut self, element: Element) -> Self {
        self.leading = Some(element);
        self
    }

    /// An element at the far end of the heading, such as a "Show all" link.
    pub fn trailing(mut self, element: Element) -> Self {
        self.trailing = Some(element);
        self
    }

    /// Space between items in logical pixels: 24 unless set. A negative value
    /// is zero; one that is not finite is ignored.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
        self
    }

    /// Space before the first item and after the last, and on both sides of
    /// the heading, in logical pixels: zero unless set. With the page's
    /// gutter here and the carousel as wide as the page, the first item lines
    /// up with the page's content while the row still scrolls out to the
    /// page's edges. A negative value is zero; one that is not finite is
    /// ignored.
    pub fn inset(mut self, inset: f32) -> Self {
        if inset.is_finite() {
            self.inset = inset.max(0.0);
        }
        self
    }

    /// The heading row, or none when nothing was given for it.
    fn header(&mut self) -> Option<Element> {
        let palette = self.palette;
        let eyebrow = self.eyebrow.take();
        let mut lines = Vec::new();
        if let Some(eyebrow) = &eyebrow {
            lines.push(Text::new(eyebrow.clone(), palette).role(TextRole::Body).muted().truncate().render());
        }
        // The title's line, which also holds the trailing element at its far end.
        let mut line = Vec::new();
        if let Some(title) = self.title.take() {
            let said = Access::new(Role::Heading).id(format!("{}:title", self.id)).name(title.clone());
            line.push(
                Element::truncated_text(title)
                    .font_family(FontFamily::SansBold)
                    .font_size(TITLE_SIZE)
                    .foreground(palette.text)
                    .accessible(match eyebrow {
                        Some(eyebrow) => said.description(eyebrow),
                        None => said,
                    })
                    .flex(1.0),
            );
        }
        if let Some(trailing) = self.trailing.take() {
            if line.is_empty() {
                line.push(Element::spacer().flex(1.0));
            }
            line.push(trailing.centered_y());
        }
        if !line.is_empty() {
            lines.push(Element::row(line).gap(HEADER_GAP));
        }
        let mut row = Vec::new();
        if let Some(leading) = self.leading.take() {
            row.push(leading.size(Some(LEADING_SIDE), Some(LEADING_SIDE)).round().centered_y());
        }
        if lines.is_empty() && row.is_empty() {
            return None;
        }
        row.push(Element::column(lines).gap(EYEBROW_GAP).centered_y().flex(1.0));
        Some(Element::row(inset(vec![Element::row(row).gap(HEADER_GAP).flex(1.0)], self.inset)))
    }
}

/// `children` between two spacers of `width`, for a row without a gap.
fn inset(children: Vec<Element>, width: f32) -> Vec<Element> {
    if width <= 0.0 {
        return children;
    }
    let side = || Element::spacer().size(Some(width), None);
    let mut row = Vec::with_capacity(children.len() + 2);
    row.push(side());
    row.extend(children);
    row.push(side());
    row
}

impl Component for Carousel {
    fn render(mut self) -> Element {
        let palette = self.palette;
        let motion = tokens().motion;
        let header = self.header();
        let items = Element::row(std::mem::take(&mut self.items)).gap(self.gap);
        let content = Element::column(vec![
            Element::row(inset(vec![items], self.inset)),
            Element::spacer().size(None, Some(INDICATOR_ROOM)),
        ]);
        let strip = Element::horizontal_scroll_2d(self.id, content)
            .drag_to_pan()
            .foreground(palette.muted)
            // Over the inset and two gaps: 48 px at the defaults, about the
            // share of a wide shelf a desktop music client fades.
            .horizontal_edge_fade(palette.surface, self.inset + 2.0 * self.gap)
            .scrollbar_visibility(ScrollbarVisibility::AutoHide {
                hold_ms: motion.move_ms.saturating_mul(4),
                fade_ms: motion.exit_ms,
            });
        match header {
            Some(header) => Element::column(vec![header, strip]).gap(SHELF_GAP),
            None => strip,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Color, DrawCommand, Frame, Key, Link, MediaTile, Point, Rect, ScrollbarAppearance, Size, UiAction, UiEvent, UiTree,
    };

    fn palette() -> ComponentPalette {
        ComponentPalette { surface: Color::rgb(3, 7, 11), ..ComponentPalette::neutral_dark() }
    }

    fn tiles(count: usize) -> Vec<Element> {
        (0..count).map(|i| MediaTile::new(format!("album:{i}"), format!("Album {i}"), palette()).width(100.0).render()).collect()
    }

    /// The shelf at the top of a 300 × 200 page that scrolls down, its bar
    /// over the content.
    fn page(carousel: Carousel) -> UiTree {
        let mut tree = UiTree::new(
            Element::scroll("page", Element::column(vec![carousel.render(), Element::spacer().size(None, Some(600.0))]))
                .scrollbar_appearance(ScrollbarAppearance::overlay())
                .size(Some(300.0), Some(200.0)),
        );
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, size, _| text.chars().count() as f32 * size * 0.5);
        tree
    }

    const SIZE: Size = Size { width: 300.0, height: 200.0 };

    fn rect(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("{id}")).rect
    }

    fn text(frame: &Frame, value: &str) -> (Point, f32, FontFamily, Color) {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, font_size, font_family, color } if text == value => {
                    Some((*position, *font_size, *font_family, *color))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{value}"))
    }

    #[test]
    fn a_drag_pans_the_row_without_clicking_and_a_still_press_clicks_an_item() {
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()));
        let frame = tree.layout(SIZE);
        let first = rect(&frame, "album:0");
        assert_eq!((first.x, first.y), (0.0, 0.0));
        let on_tile = Point::new(150.0, 40.0);
        assert_eq!(frame.hit_test(on_tile), Some("album:1"));
        tree.handle_event(UiEvent::PointerDown(on_tile));
        tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 40.0)));
        assert!(tree.handle_event(UiEvent::PointerUp(Point::new(60.0, 40.0))).is_empty());
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 90.0);
        let frame = tree.layout(SIZE);
        assert_eq!(rect(&frame, "album:1").x, 34.0);
        // A press in the gap between two items moves the row; one on an item that does not travel clicks it.
        let on_tile = Point::new(200.0, 40.0);
        tree.handle_event(UiEvent::PointerDown(on_tile));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(on_tile)), vec![UiAction::Clicked { id: "album:2".into() }]);
    }

    #[test]
    fn the_trackpad_and_shift_wheel_pan_the_row_and_the_wheel_scrolls_the_page() {
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()).title("Albums"));
        tree.layout(SIZE);
        let over = Point::new(150.0, 80.0);
        let scroll = |delta_x, delta_y, shift| UiEvent::Scroll2D { position: over, delta_x, delta_y, shift, direct: true };
        tree.handle_event(scroll(-40.0, 0.0, false));
        assert_eq!((tree.horizontal_scroll_offset("shelf"), tree.scroll_offset("page")), (40.0, 0.0));
        tree.handle_event(scroll(0.0, -30.0, true));
        assert_eq!((tree.horizontal_scroll_offset("shelf"), tree.scroll_offset("page")), (70.0, 0.0));
        tree.handle_event(scroll(0.0, -50.0, false));
        tree.layout(SIZE);
        assert_eq!((tree.horizontal_scroll_offset("shelf"), tree.scroll_offset("page")), (70.0, 50.0));
    }

    #[test]
    fn the_heading_puts_the_eyebrow_over_the_title_beside_a_round_picture_with_the_trailing_element_at_the_end() {
        let picture = Element::spacer().background(Color::WHITE);
        let carousel = Carousel::new("mixes", tiles(8), palette())
            .eyebrow("Made for Ana")
            .title("Your mixes")
            .leading(picture)
            .trailing(Link::new("mixes:all", "Show all", palette()).render())
            .inset(16.0);
        let mut tree = page(carousel);
        let frame = tree.layout(SIZE);
        let (eyebrow, eyebrow_size, _, eyebrow_ink) = text(&frame, "Made for Ana");
        let (title, title_size, title_family, title_ink) = text(&frame, "Your mixes");
        assert_eq!((title_size, title_family, title_ink), (24.0, FontFamily::SansBold, palette().text));
        assert_eq!((eyebrow_size, eyebrow_ink), (TextRole::Body.size(), palette().muted));
        assert!(eyebrow.y < title.y && eyebrow.x == title.x);
        // The picture sits at the inset, round, and the heading after it.
        let picture = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } if *color == Color::WHITE => Some((*rect, *radius)),
                _ => None,
            })
            .unwrap();
        assert_eq!(picture.0, Rect::new(16.0, picture.0.y, 48.0, 48.0));
        assert_eq!(picture.1, 24.0);
        assert_eq!(title.x, 16.0 + 48.0 + 12.0);
        let all = rect(&frame, "mixes:all");
        assert_eq!(all.x + all.width, 300.0 - 16.0);
        // On the title's line, under the eyebrow's.
        assert!(all.y > eyebrow.y + TextRole::Body.size() && all.y < title.y + TITLE_SIZE, "{all:?} {title:?}");
        // The first item lines up with the heading's inset.
        let first = rect(&frame, "album:0");
        assert_eq!(first.x, 16.0);
        assert!(first.y > picture.0.y + 48.0);
        let heading = frame.access.iter().find(|node| node.id == "mixes:title").unwrap();
        assert_eq!((heading.role, heading.name.as_str()), (Role::Heading, "Your mixes"));
        assert_eq!(heading.description.as_deref(), Some("Made for Ana"));
    }

    #[test]
    fn a_row_without_heading_is_the_strip_alone_and_a_long_title_is_cut() {
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()));
        let frame = tree.layout(SIZE);
        assert!(!frame.access.iter().any(|node| node.id == "shelf:title"));
        assert_eq!(rect(&frame, "shelf").y, 0.0);
        let long = "A title far longer than the width of a phone held upright";
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()).title(long).trailing(Link::new("all", "Show all", palette()).render()));
        let frame = tree.layout(SIZE);
        let all = rect(&frame, "all");
        assert_eq!(all.x + all.width, 300.0);
        let cut = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, position, .. } if text.starts_with("A title") => Some((text.clone(), *position)),
            _ => None,
        });
        let (cut, position) = cut.unwrap();
        assert!(cut.ends_with('…') && position.x + cut.chars().count() as f32 * 12.0 <= all.x, "{cut}");
    }

    #[test]
    fn edges_fade_only_over_hidden_items_and_the_indicator_shows_only_while_it_moves() {
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()).gap(10.0).inset(4.0));
        let fade = |frame: &Frame| -> Vec<Rect> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if (color.r, color.g, color.b) == (3, 7, 11) && color.a < 255 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect()
        };
        let indicator = |frame: &Frame| frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, .. } if rect.height == 3.0));
        let frame = tree.layout(SIZE);
        let bands = fade(&frame);
        // Over the inset and two gaps, at the right only.
        assert!(!bands.is_empty() && bands.iter().all(|band| band.x >= 300.0 - 24.0 - 0.01), "{bands:?}");
        assert!(!indicator(&frame), "the indicator starts hidden");
        tree.handle_event(UiEvent::Scroll2D { position: Point::new(150.0, 60.0), delta_x: -50.0, delta_y: 0.0, shift: false, direct: true });
        let frame = tree.layout(SIZE);
        assert!(fade(&frame).iter().any(|band| band.x < 24.0));
        assert!(indicator(&frame), "moving shows the indicator");
    }

    #[test]
    fn keyboard_focus_brings_an_item_out_from_under_the_edge() {
        let mut tree = page(Carousel::new("shelf", tiles(8), palette()));
        tree.layout(SIZE);
        assert!(tree.focus("album:0"));
        for _ in 0..4 {
            tree.handle_event(UiEvent::Key { key: Key::Tab, ctrl: false, shift: false });
        }
        assert_eq!(tree.focused_id(), Some("album:4"));
        let frame = tree.layout(SIZE);
        let item = rect(&frame, "album:4");
        assert_eq!(item.width, 100.0);
        assert!(item.x >= 0.0 && item.x + item.width <= 300.0, "{item:?}");
    }

    #[test]
    fn bad_numbers_leave_the_gap_and_inset_as_they_were_or_at_zero() {
        let carousel = |build: fn(Carousel) -> Carousel| build(Carousel::new("shelf", Vec::new(), palette()));
        assert_eq!(carousel(|c| c.gap(f32::NAN)).gap, DEFAULT_GAP);
        assert_eq!(carousel(|c| c.gap(f32::INFINITY)).gap, DEFAULT_GAP);
        assert_eq!(carousel(|c| c.gap(-8.0)).gap, 0.0);
        assert_eq!(carousel(|c| c.inset(12.0).inset(f32::NAN)).inset, 12.0);
        assert_eq!(carousel(|c| c.inset(f32::NEG_INFINITY)).inset, 0.0);
        assert_eq!(carousel(|c| c.inset(-3.0)).inset, 0.0);
        // An empty row lays out, with nothing to pan.
        let mut tree = page(Carousel::new("shelf", Vec::new(), palette()).title("Nothing yet"));
        tree.layout(SIZE);
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 0.0);
    }
}
