//! A tile of a grid or a shelf of albums and artists: square artwork over a
//! title and a second line, all under one hover surface.

use crate::{Component, ComponentPalette, Element, Icon, RadiusRole, Text, TextRole};

const DEFAULT_WIDTH: f32 = 160.0;
const MIN_WIDTH: f32 = 48.0;
/// Space around the copy, which also sets it off from the artwork.
const COPY_PADDING: f32 = 8.0;
const COPY_GAP: f32 = 2.0;
/// Side of the placeholder's glyph, as a share of the tile's width.
const NOTE_SHARE: f32 = 0.3;
/// The artwork and the hover fill round alike, so their top corners meet.
const RADIUS: RadiusRole = RadiusRole::Card;

/// The host owns the artwork and the action: the tile reports a click on its
/// ID from anywhere on it, hovering to `palette.hover` and pressing to
/// `palette.selected`. Its height follows the copy.
pub struct MediaTile {
    id: String,
    title: String,
    subtitle: String,
    artwork: Option<Element>,
    width: f32,
    round: bool,
    enabled: bool,
    palette: ComponentPalette,
}

impl MediaTile {
    /// A title under a placeholder cover, 160 wide.
    pub fn new(id: impl Into<String>, title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: String::new(),
            artwork: None,
            width: DEFAULT_WIDTH,
            round: false,
            enabled: true,
            palette,
        }
    }

    /// A caller-owned picture, held in a square of the tile's width with
    /// rounded corners. One of another shape fills the square when its
    /// `RasterImage` is marked `cover`. Without a picture the tile draws a
    /// muted note on the palette's `selected`, as for a cover that has not
    /// loaded.
    pub fn artwork(mut self, artwork: Element) -> Self {
        self.artwork = Some(artwork);
        self
    }

    /// A muted second line under the title. An empty one is no line at all.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// The tile's width, which is also the side of its artwork: 160 unless
    /// set and never under 48. Values that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(MIN_WIDTH);
        }
        self
    }

    /// Circular artwork over centered copy: an artist rather than an album.
    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self
    }

    /// A disabled tile is passive: a muted title and no hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for MediaTile {
    fn render(self) -> Element {
        let palette = self.palette;
        let side = self.width;
        let round = self.round;
        let artwork = self
            .artwork
            .unwrap_or_else(|| {
                Element::icon(Icon::MusicNote)
                    .large_icon(side * NOTE_SHARE)
                    .foreground(palette.muted)
                    .background(palette.selected)
            })
            .size(Some(side), Some(side))
            .radius(if round { side / 2.0 } else { RADIUS.px() });
        let line = |text: Text| {
            let line = text.truncate().render();
            if round { line.center_content() } else { line }
        };
        let ink = if self.enabled { palette.text } else { palette.muted };
        let mut copy = vec![line(Text::new(self.title, palette).role(TextRole::Label).ink(ink))];
        if !self.subtitle.is_empty() {
            copy.push(line(Text::new(self.subtitle, palette).muted()));
        }
        let content = Element::column(vec![
            artwork,
            Element::column(copy).gap(COPY_GAP).padding(COPY_PADDING),
        ]);
        if self.enabled {
            palette.interactive_surface(Element::clickable(self.id, content))
        } else {
            Element::panel(content)
        }
        .size(Some(side), None)
        .radius(RADIUS.px())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Color, DrawCommand, Frame, HitKind, Point, RasterImage, RasterImageFormat, Rect, Size, UiAction,
        UiEvent, UiTree,
    };

    const TITLE: &str = "Kind of Blue";
    const ARTIST: &str = "Miles Davis";
    const LONG: &str = "The Rise and Fall of Ziggy Stardust and the Spiders from Mars";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(600.0, 400.0)
    }

    fn tree_of(tile: MediaTile) -> UiTree {
        // The root fills the window; a row and a column keep the tile at its own size.
        let mut tree = UiTree::new(Element::row(vec![
            Element::column(vec![tile.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn texts(frame: &Frame) -> Vec<(&str, Point, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, color, .. } => Some((text.as_str(), *position, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn artwork_and_copy_are_one_button_region_that_hovers_and_clicks_as_a_whole() {
        let mut tree = tree_of(MediaTile::new("tile", TITLE, palette()).subtitle(ARTIST));
        let frame = tree.layout(window());
        let [region] = frame.regions.as_slice() else {
            panic!("one region, got {:?}", frame.regions);
        };
        assert_eq!((region.id.as_str(), region.kind), ("tile", HitKind::Button));
        let rect = region.rect;
        assert_eq!((rect.x, rect.y, rect.width), (0.0, 0.0, DEFAULT_WIDTH));
        let lines = texts(&frame);
        assert_eq!((lines[0].0, lines[1].0), (TITLE, ARTIST));
        // Both lines sit under the artwork and inside the region.
        assert!(lines[0].1.y >= DEFAULT_WIDTH && lines[1].1.y > lines[0].1.y);
        assert!(lines[1].1.y + COPY_PADDING < rect.height);
        assert_eq!((lines[0].2, lines[1].2), (palette().text, palette().muted));
        for point in [Point::new(80.0, 80.0), Point::new(80.0, lines[1].1.y + 2.0)] {
            tree.handle_event(UiEvent::PointerMove(point));
            // One fill, the whole tile's, whichever part the pointer is over.
            assert!(tree.layout(window()).commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { rect: fill, color, radius }
                    if *fill == rect && *color == palette().hover && *radius == RADIUS.px()
            )));
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "tile".into() }]
            );
        }
    }

    #[test]
    fn the_artwork_is_a_square_of_the_tile_width_plain_and_round() {
        let picture = || {
            let data = std::sync::Arc::from("AAAA");
            Element::raster_image(
                RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, data, "A cover").unwrap(),
            )
        };
        for (round, radius) in [(false, RADIUS.px()), (true, 100.0)] {
            let tile = MediaTile::new("tile", TITLE, palette()).artwork(picture()).width(200.0).round(round);
            let frame = tree_of(tile).layout(window());
            let painted = frame.commands.iter().find_map(|command| match command {
                DrawCommand::RasterImage { rect, radius, .. } => Some((*rect, *radius)),
                _ => None,
            });
            assert_eq!(painted, Some((Rect::new(0.0, 0.0, 200.0, 200.0), radius)));
            // An album's title starts at the inset; an artist's is centered.
            assert_eq!(texts(&frame)[0].1.x > COPY_PADDING, round);
            assert_eq!(frame.regions[0].rect.width, 200.0);
        }
    }

    #[test]
    fn without_artwork_a_muted_note_is_centered_on_the_selected_fill() {
        let frame = tree_of(MediaTile::new("tile", TITLE, palette())).layout(window());
        let fill = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, radius } if *color == palette().selected => {
                Some((*rect, *radius))
            }
            _ => None,
        });
        assert_eq!(fill, Some((Rect::new(0.0, 0.0, DEFAULT_WIDTH, DEFAULT_WIDTH), RADIUS.px())));
        let note = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { rect, icon, tint } => Some((*rect, *icon, *tint)),
            _ => None,
        });
        let (rect, icon, tint) = note.expect("a placeholder glyph");
        assert_eq!((icon, tint), (Icon::MusicNote, palette().muted));
        assert_eq!(rect.width, DEFAULT_WIDTH * NOTE_SHARE);
        assert_eq!((rect.x + rect.width / 2.0, rect.y + rect.height / 2.0), (80.0, 80.0));
    }

    #[test]
    fn a_disabled_tile_takes_no_pointer_or_focus_and_keeps_its_place() {
        let tile = |enabled| MediaTile::new("tile", TITLE, palette()).subtitle(ARTIST).enabled(enabled);
        let mut tree = tree_of(tile(false));
        let frame = tree.layout(window());
        assert!(frame.regions.is_empty());
        assert!(!tree.focus("tile"));
        let point = Point::new(80.0, 80.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        let lines = texts(&frame);
        assert_eq!((lines[0].2, lines[1].2), (palette().muted, palette().muted));
        // The same layout as the enabled tile, so a shelf does not move.
        let enabled = tree_of(tile(true)).layout(window());
        let places = |frame: &Frame| texts(frame).iter().map(|line| line.1).collect::<Vec<_>>();
        assert_eq!(places(&frame), places(&enabled));
    }

    #[test]
    fn a_long_title_is_cut_and_does_not_widen_the_tile() {
        for round in [false, true] {
            let tile = MediaTile::new("tile", LONG, palette()).subtitle(LONG).round(round);
            let frame = tree_of(tile).layout(window());
            assert_eq!(frame.regions[0].rect.width, DEFAULT_WIDTH);
            let lines = texts(&frame);
            assert_eq!(lines.len(), 2);
            // A cut line fills the copy's width, so even centered it starts by the inset.
            assert!(lines.iter().all(|line| {
                line.0.ends_with('…') && line.1.x >= COPY_PADDING && line.1.x < 2.0 * COPY_PADDING
            }));
        }
    }

    #[test]
    fn bad_widths_are_ignored_and_a_small_one_is_raised() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(MediaTile::new("tile", TITLE, palette()).width(bad).width, DEFAULT_WIDTH);
        }
        for small in [-8.0, 0.0, 12.0] {
            assert_eq!(MediaTile::new("tile", TITLE, palette()).width(small).width, MIN_WIDTH);
        }
    }
}
