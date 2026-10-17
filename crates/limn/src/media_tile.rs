//! A tile of a grid or a shelf of albums and artists: square artwork over a
//! title and a second line, all under one hover surface.

use crate::{Access, Component, ComponentPalette, ControlSize, Element, Icon, IconButton, RadiusRole, Role, Text, TextRole};

const DEFAULT_WIDTH: f32 = 160.0;
const MIN_WIDTH: f32 = 48.0;
/// Space around the copy, which also sets it off from the artwork.
const COPY_PADDING: f32 = 8.0;
const COPY_GAP: f32 = 2.0;
/// Side of the placeholder's glyph, as a share of the tile's width.
const NOTE_SHARE: f32 = 0.3;
/// The artwork and the hover fill round alike, so their top corners meet.
const RADIUS: RadiusRole = RadiusRole::Card;
/// How far the play button stands in from the artwork's bottom corner.
const PLAY_INSET: f32 = 8.0;

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
    padding: f32,
    play: Option<(String, String)>,
    playing: bool,
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
            padding: 0.0,
            play: None,
            playing: false,
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

    /// Stand the artwork and its copy this far inside the tile, so the hover
    /// fill frames them as a card. None by default, and never more than a
    /// quarter of the width. Values that are not finite are ignored.
    pub fn padding(mut self, padding: f32) -> Self {
        if padding.is_finite() {
            self.padding = padding.max(0.0);
        }
        self
    }

    /// A round play button over the artwork's bottom corner, named `label`,
    /// that reports `id` and rises into view under the pointer. The tile's
    /// own click is untouched.
    pub fn play(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.play = Some((id.into(), label.into()));
        self
    }

    /// What the tile stands for plays: its play button is a pause and stays.
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }
}

impl Component for MediaTile {
    fn render(self) -> Element {
        let palette = self.palette;
        let inset = self.padding.min(self.width / 4.0);
        let side = self.width - 2.0 * inset;
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
        // Named by the title, described by the line under it.
        let mut said = Access::new(Role::Button).name(self.title.clone());
        if !self.subtitle.is_empty() {
            said = said.description(self.subtitle.clone());
        }
        let ink = if self.enabled { palette.text } else { palette.muted };
        let mut copy = vec![line(Text::new(self.title, palette).role(TextRole::Label).ink(ink))];
        if !self.subtitle.is_empty() {
            copy.push(line(Text::new(self.subtitle, palette).muted()));
        }
        let play = self.play.filter(|_| self.enabled);
        let artwork = match &play {
            Some((id, label)) => {
                let icon = if self.playing { Icon::PauseFill } else { Icon::PlayFill };
                let button = IconButton::new(id, icon, palette).size(ControlSize::Spacious).round().accessibility_label(label).render();
                let corner = Element::column(vec![Element::spacer().flex(1.0), Element::row(vec![Element::spacer().flex(1.0), button])]).padding(PLAY_INSET);
                let corner = if self.playing { corner } else { corner.rise_on_hover(format!("{id}:rise")) };
                Element::stack(vec![artwork, corner])
            }
            None => artwork,
        };
        // Inset, the copy lines up with the artwork and the tile's padding frames both.
        let copy = Element::column(copy).gap(COPY_GAP);
        let content = if inset > 0.0 {
            Element::column(vec![artwork, Element::spacer().size(None, Some(COPY_PADDING)), copy]).padding(inset)
        } else {
            Element::column(vec![artwork, copy.padding(COPY_PADDING)])
        };
        if self.enabled {
            let tile = Element::clickable(self.id, content).accessible(said);
            match play {
                Some((id, _)) => palette.interactive_surface(tile.hover_with(id)).hover_region(),
                None => palette.interactive_surface(tile),
            }
        } else {
            Element::panel(content).accessible(said.id(self.id).disabled(true))
        }
        .size(Some(self.width), None)
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

    #[test]
    fn a_tile_is_a_button_named_by_its_title_and_described_by_its_second_line() {
        for enabled in [true, false] {
            let tile = MediaTile::new("album", TITLE, palette()).subtitle(ARTIST).enabled(enabled);
            let frame = tree_of(tile).layout(window());
            let said = frame.access.iter().find(|node| node.id == "album").unwrap();
            assert_eq!((said.role, said.name.as_str(), said.description.as_deref()), (Role::Button, TITLE, Some(ARTIST)));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
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
    fn a_play_button_rises_over_the_inset_artwork_under_the_pointer_and_reports_its_own_id() {
        let tile = |playing| MediaTile::new("tile", TITLE, palette()).subtitle(ARTIST).padding(12.0).play("tile:play", "Play").playing(playing);
        let mut tree = tree_of(tile(false));
        let frame = tree.layout(window());
        let button = frame.regions.iter().find(|region| region.id == "tile:play").expect("the play button takes the pointer").rect;
        let whole = frame.regions.iter().find(|region| region.id == "tile").unwrap().rect;
        assert_eq!(whole.width, DEFAULT_WIDTH, "the padding is inside the tile");
        let art = DEFAULT_WIDTH - 24.0;
        assert_eq!((button.x + button.width, button.y + button.height), (12.0 + art - PLAY_INSET, 12.0 + art - PLAY_INSET));
        let ink = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { icon: Icon::PlayFill | Icon::PauseFill, tint, .. } => Some(tint.a),
            _ => None,
        }).unwrap_or(0);
        assert_eq!(ink(&frame), 0, "hidden at rest");
        let center = Point::new(button.x + button.width / 2.0, button.y + button.height / 2.0);
        tree.handle_event(UiEvent::PointerMove(center));
        assert!(ink(&tree.layout(window())) > 0, "shown under the pointer");
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: "tile:play".into() }]);
        let playing = tree_of(tile(true)).layout(window());
        assert!(ink(&playing) > 0, "a tile that plays keeps its pause in view");
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
