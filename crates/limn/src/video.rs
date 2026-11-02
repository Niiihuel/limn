//! A short muted clip that plays in place, such as a feature shown in a
//! "what's new" dialog, with a play and pause button over its corner.

use crate::{
    Access, ButtonVariant, Color, Component, ComponentPalette, Element, Icon, IconButton, RasterImage, Role, SurfaceFit,
};

// The button sits over moving pixels of any color, not over the palette:
// white on a translucent black disc reads on all of them, as a video
// player's controls do.
const VEIL: Color = Color::rgba(0, 0, 0, 140);
const VEIL_HOVER: Color = Color::rgba(0, 0, 0, 190);
const INK: Color = Color::rgb(255, 255, 255);
const BUTTON: f32 = 36.0;
const INSET: f32 = 12.0;

/// A clip the host decodes and presents on the surface `id`, as
/// [`Element::surface`] describes, filling its box and cropped around the
/// center ([`SurfaceFit::Cover`]), laid over its still poster and under a
/// round play and pause button at its bottom-right corner. The host owns
/// whether it plays: it passes `playing` on every rebuild and flips it when
/// [`Video::toggle_id`] is reported. The component decodes nothing; the
/// `limn-video` crate does that for a host on `limn-egui`.
pub struct Video {
    id: String,
    label: String,
    playing: bool,
    palette: ComponentPalette,
    poster: Option<RasterImage>,
    radius: Option<f32>,
    width: Option<f32>,
    play_label: String,
    pause_label: String,
    controls: bool,
}

impl Video {
    /// A clip shown on the host-fed surface `id`, named `label` to the
    /// platform, playing or paused as `playing` says.
    pub fn new(id: impl Into<String>, label: impl Into<String>, playing: bool, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            playing,
            palette,
            poster: None,
            radius: None,
            width: None,
            play_label: "Play video".into(),
            pause_label: "Pause video".into(),
            controls: true,
        }
    }

    /// The action the play and pause button reports, `{id}:toggle`, for a
    /// video on the surface `id`.
    pub fn toggle_id(id: &str) -> String {
        format!("{id}:toggle")
    }

    /// A still picture shown under the clip until its first frame is
    /// presented: while it loads, and for as long as it never plays, as under
    /// reduced motion. Give it the clip's size, usually its first frame.
    pub fn poster(mut self, poster: RasterImage) -> Self {
        self.poster = Some(poster);
        self
    }

    /// Round the corners by this radius in logical pixels, such as the
    /// container's inner radius; `tokens().radius.card` unless set. A value
    /// that is negative or not finite is ignored.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() && radius >= 0.0 {
            self.radius = Some(radius);
        }
        self
    }

    /// Lay the video out this wide, in logical pixels, and 9/16 of that
    /// high. Without it the video fills the box its parent gives it. A
    /// value that is not finite or not above zero is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = Some(width);
        }
        self
    }

    /// Whether the play and pause button is there: `true` unless set. Without
    /// it the video is a picture that may move, for a host that cannot play
    /// the clip, as without a codec, and shows only its poster.
    pub fn controls(mut self, controls: bool) -> Self {
        self.controls = controls;
        self
    }

    /// What the button is called while the clip is paused and while it
    /// plays: "Play video" and "Pause video" unless set.
    pub fn control_labels(mut self, play: impl Into<String>, pause: impl Into<String>) -> Self {
        self.play_label = play.into();
        self.pause_label = pause.into();
        self
    }
}

impl Component for Video {
    fn render(self) -> Element {
        let radius = self.radius.unwrap_or(crate::tokens().radius.card);
        let (icon, name) = if self.playing {
            (Icon::PauseFill, self.pause_label)
        } else {
            (Icon::PlayFill, self.play_label)
        };
        let button = IconButton::new(Self::toggle_id(&self.id), icon, self.palette)
            .variant(ButtonVariant::Ghost)
            .ink(INK)
            .hover_ink(INK)
            .round()
            .render()
            .background(VEIL)
            .hover_background(VEIL_HOVER)
            .size(Some(BUTTON), Some(BUTTON))
            .accessible(Access::new(Role::Button).name(name));
        // While it plays the button keeps out of the way until the pointer or
        // the focus is on the video; paused, it says there is something to play.
        let button = if self.playing {
            button.reveal_on_hover(format!("{}:controls", self.id))
        } else {
            button
        };
        let corner = Element::column(vec![
            Element::spacer().flex(1.0),
            Element::row(vec![Element::spacer().flex(1.0), button]),
        ])
        .padding(INSET);

        let mut layers = Vec::with_capacity(3);
        if let Some(poster) = self.poster {
            // The surface already names the video to the platform.
            layers.push(Element::raster_image(poster.cover()).radius(radius).inert());
        }
        layers.push(Element::surface(self.id, self.label).surface_fit(SurfaceFit::Cover).radius(radius));
        if self.controls {
            layers.push(corner);
        }
        let video = Element::stack(layers).hover_region().radius(radius);
        match self.width {
            Some(width) => video.size(Some(width), Some(width * 9.0 / 16.0)),
            None => video.flex(1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const POSTER_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

    fn poster() -> RasterImage {
        RasterImage::new("a".repeat(64), crate::RasterImageFormat::Png, 70, POSTER_PNG.into(), "First frame").unwrap()
    }

    fn tree(video: Video) -> UiTree {
        let mut tree = UiTree::new(Element::row(vec![video.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(tree: &mut UiTree, id: &str) -> Option<Rect> {
        tree.layout(Size::new(640.0, 400.0)).regions.iter().find(|region| region.id == id).map(|region| region.rect)
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false, alt: false }
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    #[test]
    fn the_clip_is_a_sixteen_by_nine_surface_over_its_poster_with_a_button_at_its_corner() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Video::new("clip", "New sidebar", true, palette).poster(poster()).width(480.0).radius(10.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 100.0)));
        let frame = tree.layout(Size::new(640.0, 400.0));
        let poster_at = frame.commands.iter().position(|command| matches!(command, DrawCommand::RasterImage { .. }));
        let surface = frame.commands.iter().enumerate().find_map(|(at, command)| match command {
            DrawCommand::Surface { rect, id, label, fit, radius, .. } => Some((at, *rect, id.clone(), label.clone(), *fit, *radius)),
            _ => None,
        });
        let (surface_at, rect, id, label, fit, radius) = surface.expect("a surface");
        assert!(poster_at.expect("a poster") < surface_at, "the poster is under the clip");
        assert_eq!((rect.width, rect.height), (480.0, 270.0));
        assert_eq!((id.as_str(), label.as_str(), fit, radius), ("clip", "New sidebar", SurfaceFit::Cover, 10.0));
        let button = frame.regions.iter().find(|region| region.id == "clip:toggle").expect("the button").rect;
        assert_eq!((button.width, button.height), (BUTTON, BUTTON));
        assert_eq!(button.x + button.width, rect.x + rect.width - INSET);
        assert_eq!(button.y + button.height, rect.y + rect.height - INSET);
        let named: Vec<_> = frame.access.iter().map(|node| node.name.as_str()).collect();
        assert!(named.contains(&"Pause video"), "{named:?}");
        assert!(!named.contains(&"First frame"), "the poster is not a second picture: {named:?}");
    }

    #[test]
    fn the_button_says_what_it_does_next() {
        let palette = ComponentPalette::neutral_dark();
        for (playing, icon, name) in [(true, Icon::PauseFill, "Pausar"), (false, Icon::PlayFill, "Reproducir")] {
            let frame = tree(Video::new("clip", "Clip", playing, palette).control_labels("Reproducir", "Pausar"))
                .layout(Size::new(640.0, 400.0));
            assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: drawn, .. } if *drawn == icon)));
            assert!(frame.access.iter().any(|node| node.name == name));
        }
    }

    #[test]
    fn the_button_toggles_by_pointer_enter_and_space() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Video::new("clip", "Clip", true, palette).width(320.0));
        let toggled = vec![UiAction::Clicked { id: "clip:toggle".into() }];
        let button = region(&mut tree, "clip:toggle").expect("the button");
        tree.handle_event(UiEvent::PointerMove(center(button)));
        tree.handle_event(UiEvent::PointerDown(center(button)));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(center(button))), toggled);
        tree.handle_event(UiEvent::PointerMove(Point::new(600.0, 390.0)));
        tree.handle_event(key(Key::Tab));
        assert_eq!(tree.focused_id(), Some("clip:toggle"));
        assert_eq!(tree.handle_event(key(Key::Enter)), toggled);
        assert_eq!(tree.handle_event(key(Key::Space)), toggled);
    }

    #[test]
    fn while_it_plays_the_button_shows_only_under_the_pointer_or_the_focus() {
        let palette = ComponentPalette::neutral_dark();
        let painted = |tree: &mut UiTree| {
            tree.layout(Size::new(640.0, 400.0))
                .commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Icon { tint, .. } if tint.a > 0))
        };
        let mut playing = tree(Video::new("clip", "Clip", true, palette).width(320.0));
        playing.handle_event(UiEvent::PointerMove(Point::new(600.0, 390.0)));
        assert!(!painted(&mut playing), "away from the video, the button is hidden");
        playing.handle_event(UiEvent::PointerMove(Point::new(100.0, 90.0)));
        assert!(painted(&mut playing), "over the video it shows");
        playing.handle_event(UiEvent::PointerMove(Point::new(600.0, 390.0)));
        playing.handle_event(key(Key::Tab));
        assert!(painted(&mut playing), "with the focus it shows");

        let mut paused = tree(Video::new("clip", "Clip", false, palette).width(320.0));
        paused.handle_event(UiEvent::PointerMove(Point::new(600.0, 390.0)));
        assert!(painted(&mut paused), "paused, the play button stays");
    }

    #[test]
    fn the_frame_says_whether_the_clip_is_in_view() {
        let palette = ComponentPalette::neutral_dark();
        let video = || Video::new("clip", "Clip", true, palette).width(160.0).render();
        let size = Size::new(640.0, 400.0);
        assert!(UiTree::new(Element::column(vec![video()])).layout(size).surface_shown("clip"));
        assert!(!UiTree::new(Element::column(vec![video()])).layout(size).surface_shown("other"));
        // Scrolled out of a box that clips what it holds.
        let below = Element::column(vec![Element::spacer().size(None, Some(300.0)), video()])
            .size(None, Some(200.0))
            .clip_children();
        assert!(!UiTree::new(Element::column(vec![below])).layout(size).surface_shown("clip"));
        // Gone from the tree, as a closed dialog's.
        assert!(!UiTree::new(Element::spacer()).layout(size).surface_shown("clip"));
    }

    #[test]
    fn without_controls_it_is_a_picture_with_no_button() {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(Video::new("clip", "Clip", false, palette).poster(poster()).controls(false)).layout(Size::new(640.0, 400.0));
        assert!(frame.regions.iter().all(|region| region.id != "clip:toggle"));
        assert!(frame.surface_shown("clip"));
    }

    #[test]
    fn bad_numbers_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        for bad in [f32::NAN, f32::INFINITY, -4.0, 0.0] {
            let video = Video::new("clip", "Clip", true, palette).width(bad).radius(if bad == 0.0 { -1.0 } else { bad });
            assert_eq!((video.width, video.radius), (None, None));
        }
    }
}
