//! A track as a card to hand around: its record sleeve beside its title and
//! artist, over a tint taken from its artwork.
//!
//! The arrangement follows Spell UI's Spotify card (MIT, Copyright (c) 2025
//! Spell UI, github.com/xxtomm/spell-ui).

use crate::{Color, Component, ComponentPalette, Element, FontFamily, RadiusRole, RasterImage, Text, TextRole, RECORD_REACH};

const DEFAULT_WIDTH: f32 = 325.0;
const MIN_WIDTH: f32 = 220.0;
const PADDING: f32 = 12.0;
const SLEEVE: f32 = 75.0;
/// How far the title's fade runs when the title does not fit.
const TITLE_FADE: f32 = 24.0;
/// How dark the foot of the card gets, so the copy reads on any tint.
const FOOT_ALPHA: u8 = 204;

/// The host owns the picture, the tint and the action: the sleeve reports a
/// click on the card's ID, and the host answers by playing or pausing and
/// passing `playing`. While it plays the record is out of its sleeve and
/// turning; under the pointer it comes out as an invitation. The card is
/// controlled and keeps no state of its own.
pub struct MediaCard {
    id: String,
    title: String,
    artist: String,
    artwork: Option<RasterImage>,
    tint: Option<Color>,
    mark: Option<Element>,
    playing: bool,
    width: f32,
    enabled: bool,
    palette: ComponentPalette,
}

impl MediaCard {
    /// A title beside an empty sleeve, 325 wide.
    pub fn new(id: impl Into<String>, title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            artist: String::new(),
            artwork: None,
            tint: None,
            mark: None,
            playing: false,
            width: DEFAULT_WIDTH,
            enabled: true,
            palette,
        }
    }

    /// A muted line under the title. An empty one is no line at all.
    pub fn artist(mut self, artist: impl Into<String>) -> Self {
        self.artist = artist.into();
        self
    }

    /// The picture on the sleeve, cropped to its square.
    pub fn artwork(mut self, artwork: RasterImage) -> Self {
        self.artwork = Some(artwork);
        self
    }

    /// The card's fill, a color the host takes from the artwork. Without it
    /// the card is the palette's `raised`.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);
        self
    }

    /// An element for the top corner at the end: a mark, or a link out.
    pub fn mark(mut self, mark: Element) -> Self {
        self.mark = Some(mark);
        self
    }

    /// Whether the track plays: the record is out and turning.
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }

    /// The card's width: 325 unless set and never under 220. Values that are
    /// not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(MIN_WIDTH);
        }
        self
    }

    /// A disabled card is passive: its sleeve takes no pointer and its
    /// record stays in.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Component for MediaCard {
    fn render(self) -> Element {
        let palette = self.palette;
        let record = Element::record(format!("{}:record", self.id), self.artwork, self.playing && self.enabled, self.playing && self.enabled)
            .background(palette.selected)
            .radius(RadiusRole::Control.px())
            .size(Some(SLEEVE), Some(SLEEVE));
        let sleeve = if self.enabled {
            // The sleeve is the control; the record peeks out under the pointer.
            Element::clickable(&self.id, record.record_peek(&self.id))
        } else {
            Element::panel(record)
        }
        .size(Some(SLEEVE), Some(SLEEVE));

        // The copy hugs the end: the mark above, the title over the artist below.
        let ending = |element: Element| Element::row(vec![Element::spacer().flex(1.0), element]);
        let ink = if self.enabled { palette.text } else { palette.muted };
        let title = Element::faded_text(self.title, TITLE_FADE)
            .font_family(FontFamily::Sans)
            .font_size(TextRole::Label.size())
            .foreground(ink)
            .min_size(0.0, 0.0);
        let mut copy = vec![ending(self.mark.unwrap_or_else(Element::spacer)), Element::spacer().flex(1.0), ending(title)];
        if !self.artist.is_empty() {
            copy.push(ending(Text::new(self.artist, palette).role(TextRole::Label).muted().truncate().render()));
        }
        // Room for the record to come out before the copy begins.
        let reach = SLEEVE * RECORD_REACH + PADDING;
        let content = Element::row(vec![sleeve, Element::spacer().size(Some(reach), None), Element::column(copy).flex(1.0)]);
        Element::panel(content)
            .padding(PADDING)
            .background(self.tint.unwrap_or(palette.raised))
            .bottom_background_fade(Color { a: FOOT_ALPHA, ..palette.surface }, SLEEVE + 2.0 * PADDING)
            .radius(RadiusRole::Panel.px())
            .size(Some(self.width), Some(SLEEVE + 2.0 * PADDING))
            .clip_children()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Point, Size, UiAction, UiEvent, UiTree};
    use std::time::{Duration, Instant};

    const SIZE: Size = Size::new(400.0, 200.0);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn card() -> MediaCard {
        MediaCard::new("card", "Past Won't Leave My Bed", palette()).artist("Joji")
    }

    /// How far out and how far turned the record of the frame is.
    fn record(tree: &mut UiTree, card: MediaCard, now: Instant) -> (f32, f32) {
        tree.set_root(card.render());
        let frame = tree.layout_at(SIZE, now);
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Record { out, angle, .. } => Some((*out, *angle)),
                _ => None,
            })
            .expect("the card draws a record")
    }

    #[test]
    fn the_sleeve_is_the_control_and_a_click_on_it_reports_the_card() {
        let mut tree = UiTree::new(card().render());
        let frame = tree.layout(SIZE);
        let region = frame.regions.iter().find(|region| region.id == "card").expect("the sleeve takes the pointer");
        assert_eq!(region.kind, HitKind::Button);
        assert_eq!((region.rect.x, region.rect.y, region.rect.width, region.rect.height), (PADDING, PADDING, SLEEVE, SLEEVE));
        let at = Point::new(PADDING + 10.0, PADDING + 10.0);
        tree.handle_event(UiEvent::PointerDown(at));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(at)), vec![UiAction::Clicked { id: "card".into() }]);
    }

    #[test]
    fn a_playing_card_draws_its_record_out_and_turns_it_and_a_stop_keeps_the_angle() {
        let start = Instant::now();
        let mut tree = UiTree::new(card().render());
        assert_eq!(record(&mut tree, card(), start), (0.0, 0.0), "at rest the record is in and still");
        record(&mut tree, card().playing(true), start);
        let (out, angle) = record(&mut tree, card().playing(true), start + Duration::from_millis(750));
        assert_eq!(out, 1.0, "out once the slide is over");
        assert!((angle - std::f32::consts::FRAC_PI_2).abs() < 0.02, "a quarter turn in a quarter of three seconds: {angle}");
        let (_, stopped) = record(&mut tree, card(), start + Duration::from_millis(750));
        let (back, later) = record(&mut tree, card(), start + Duration::from_secs(5));
        assert_eq!((back, later), (0.0, stopped), "in again, and no further round");
    }

    #[test]
    fn the_record_peeks_out_under_the_pointer() {
        let start = Instant::now();
        let mut tree = UiTree::new(card().render());
        tree.layout_at(SIZE, start);
        tree.handle_event(UiEvent::PointerMove(Point::new(PADDING + 10.0, PADDING + 10.0)));
        record(&mut tree, card(), start);
        let (out, angle) = record(&mut tree, card(), start + Duration::from_secs(1));
        assert_eq!((out, angle), (1.0, 0.0), "out, and still: nothing plays");
        tree.handle_event(UiEvent::PointerMove(Point::new(390.0, 190.0)));
        record(&mut tree, card(), start + Duration::from_secs(1));
        assert_eq!(record(&mut tree, card(), start + Duration::from_secs(2)).0, 0.0);
    }

    #[test]
    fn a_disabled_card_takes_no_pointer_and_keeps_its_record_in() {
        let start = Instant::now();
        let mut tree = UiTree::new(card().enabled(false).playing(true).render());
        let frame = tree.layout_at(SIZE, start);
        assert!(frame.regions.iter().all(|region| region.id != "card"));
        assert_eq!(record(&mut tree, card().enabled(false).playing(true), start + Duration::from_secs(1)), (0.0, 0.0));
    }

    #[test]
    fn reduced_motion_shows_the_record_out_and_holds_it_still() {
        let start = Instant::now();
        let mut tree = UiTree::new(card().playing(true).render());
        tree.set_reduced_motion(true);
        record(&mut tree, card().playing(true), start);
        assert_eq!(record(&mut tree, card().playing(true), start + Duration::from_secs(1)), (1.0, 0.0));
    }

    #[test]
    fn the_card_keeps_its_width_whatever_the_title_and_never_goes_under_its_least() {
        let width = |card: MediaCard| {
            let mut tree = UiTree::new(Element::row(vec![card.render(), Element::spacer().flex(1.0)]));
            let frame = tree.layout(Size::new(900.0, 200.0));
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, .. } => Some(rect.width),
                _ => None,
            })
        };
        let long = "The Rise and Fall of Ziggy Stardust and the Spiders from Mars, the whole of it";
        assert_eq!(width(MediaCard::new("card", long, palette())), Some(DEFAULT_WIDTH));
        assert_eq!(width(card().width(10.0)), Some(MIN_WIDTH));
        assert_eq!(width(card().width(f32::NAN)), Some(DEFAULT_WIDTH));
    }
}
