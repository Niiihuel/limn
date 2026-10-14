//! A value dragged along a track: a seek position, a volume, any share of a range.

use crate::{Access, Color, Component, ComponentPalette, CornerRadii, Element, FontFamily, Role};

/// Height of the pointer target. The track is thinner and centered in it.
const HEIGHT: f32 = 16.0;
const TRACK: f32 = 4.0;
const THUMB: f32 = 12.0;
/// A mark is a hairline as tall as the thumb.
const MARK: f32 = 1.0;
/// The text size of a stop's label, and the space between the labels and the track.
const LABEL_SIZE: f32 = 12.0;
const LABEL_GAP: f32 = 4.0;

/// The host owns the value: it passes `fraction` on every rebuild and applies
/// each `SliderChanged`. `committed` is false while the pointer is down and
/// true on release and for keyboard steps, so a seek bar can wait for the
/// release while a volume control follows the drag. Without a width the
/// slider fills the available horizontal space.
pub struct Slider {
    id: String,
    fraction: f32,
    palette: ComponentPalette,
    width: Option<f32>,
    steps: usize,
    enabled: bool,
    ink: Option<Color>,
    accessibility_label: Option<String>,
    marks: bool,
    labels: Vec<String>,
    default_stop: Option<usize>,
}

impl Slider {
    /// `fraction` runs from 0 to 1; values outside are clamped and NaN is 0.
    pub fn new(id: impl Into<String>, fraction: f32, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            fraction: if fraction.is_nan() { 0.0 } else { fraction.clamp(0.0, 1.0) },
            palette,
            width: None,
            steps: 0,
            enabled: true,
            ink: None,
            accessibility_label: None,
            marks: false,
            labels: Vec::new(),
            default_stop: None,
        }
    }

    /// What the slider is called to the platform. It shows no text of its
    /// own, so without this, or a `FormField` around it, it has no name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Fix the width, in logical pixels, never below the thumb's 12. A value that is not
    /// finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(THUMB));
        }
        self
    }

    /// Snap to this many evenly spaced stops, the ends included. Below 2 the
    /// slider is continuous, which is the default.
    pub fn steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    /// A disabled slider shows its value and takes no pointer or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Color of the filled part and the thumb, in place of `palette.text`.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// Draw a mark across the track at each stop, behind the thumb, in the
    /// palette's border color. Without `steps` of 2 or more there are no
    /// stops, so no marks.
    pub fn marks(mut self) -> Self {
        self.marks = true;
        self
    }

    /// Name the stops, from the first: a label in the muted color over each,
    /// which also draws the marks. The labels at the ends line up with the
    /// ends of the slider, the others center on their stop. Labels past the
    /// last stop are ignored and an empty one leaves its stop unnamed. The
    /// platform is told the label of the stop the value is on.
    pub fn mark_labels<I, S>(mut self, labels: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Show the label of this stop, counted from 0, in the ink instead of the
    /// muted color: the value a setting has until it is changed. A stop past
    /// the last, or one without a label, highlights nothing.
    pub fn default_stop(mut self, index: usize) -> Self {
        self.default_stop = Some(index);
        self
    }
}

impl Component for Slider {
    fn render(self) -> Element {
        let ink = match (self.enabled, self.ink) {
            (false, _) => self.palette.muted,
            (true, Some(color)) => color,
            (true, None) => self.palette.text,
        };
        let round = TRACK / 2.0;
        let segment = |share: f32, color: Color, radii: CornerRadii| {
            Element::panel(Element::spacer())
                .background(color)
                .corner_radii(radii)
                .size(None, Some(TRACK))
                .flex(share)
                .centered_y()
        };
        // The thumb sits between the two segments, so its center travels from
        // half a thumb inside one end to half a thumb inside the other: the
        // inset the tree is told about below.
        let mut pieces = Vec::with_capacity(3);
        if self.fraction > 0.0 {
            pieces.push(segment(
                self.fraction,
                ink,
                CornerRadii { top_left: round, bottom_left: round, top_right: 0.0, bottom_right: 0.0 },
            ));
        }
        if self.enabled {
            pieces.push(
                Element::spacer()
                    .size(Some(THUMB), Some(THUMB))
                    .background(ink)
                    .radius(THUMB / 2.0)
                    .centered_y(),
            );
        }
        if self.fraction < 1.0 {
            pieces.push(segment(
                1.0 - self.fraction,
                self.palette.selected,
                CornerRadii { top_left: 0.0, bottom_left: 0.0, top_right: round, bottom_right: round },
            ));
        }
        let mut track = Element::row(pieces).size(self.width, Some(HEIGHT));
        let stops = if self.steps >= 2 { self.steps } else { 0 };
        let mut labels = self.labels;
        labels.truncate(stops);
        if stops > 0 && (self.marks || !labels.is_empty()) {
            // Each mark's center is a stop: half a thumb in from the ends,
            // with even shares of the rest between them.
            let edge = || Element::spacer().size(Some((THUMB - MARK) / 2.0), None);
            let mut marks = vec![edge()];
            for stop in 0..stops {
                if stop > 0 {
                    marks.push(Element::spacer().flex(1.0));
                }
                marks.push(
                    Element::panel(Element::spacer())
                        .background(self.palette.border)
                        .size(Some(MARK), Some(THUMB))
                        .centered_y(),
                );
            }
            marks.push(edge());
            track = Element::stack(vec![Element::row(marks), track]).size(self.width, Some(HEIGHT));
        }
        // Drawn disabled it is passive, and known by the ID it would have had.
        let mut access = Access::new(Role::Slider)
            .id(self.id.clone())
            .value(self.fraction)
            .disabled(!self.enabled);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
        }
        let last = stops.saturating_sub(1);
        if let Some(label) = labels.get((self.fraction * last as f32).round() as usize).filter(|label| !label.is_empty()) {
            access = access.value_text(label.clone());
        }
        let slider = if self.enabled {
            Element::clickable(self.id, track).slider(self.fraction, self.steps, THUMB / 2.0)
        } else {
            Element::panel(track)
        }
        .size(self.width, Some(HEIGHT))
        .accessible(access);
        // Each label is a layer of its own, so a long one may reach past its
        // neighbours' share without moving them.
        let layers: Vec<Element> = labels
            .into_iter()
            .enumerate()
            .filter(|(_, label)| !label.is_empty())
            .map(|(stop, label)| {
                let text = Element::text(label)
                    .font_family(FontFamily::Sans)
                    .font_size(LABEL_SIZE)
                    .foreground(if self.default_stop == Some(stop) { ink } else { self.palette.muted });
                let share = |stops: usize| Element::spacer().flex(stops as f32);
                let edge = || Element::spacer().size(Some(THUMB / 2.0), None);
                match stop {
                    0 => Element::row(vec![text, share(1)]),
                    _ if stop == last => Element::row(vec![share(1), text]),
                    // Two stops wide, centered on its own.
                    _ => Element::row(vec![
                        edge(),
                        share(stop - 1),
                        text.center_content().flex(2.0),
                        share(last - stop - 1),
                        edge(),
                    ]),
                }
            })
            .collect();
        if layers.is_empty() {
            return slider;
        }
        Element::column(vec![Element::stack(layers), slider]).gap(LABEL_GAP).size(self.width, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Key, Point, Size, UiAction, UiEvent, UiTree};

    const WIDTH: f32 = 212.0;
    const TRAVEL: f32 = WIDTH - THUMB;

    fn tree(slider: Slider) -> UiTree {
        // The root fills the window; a row keeps the slider at its own width.
        let mut tree = UiTree::new(Element::row(vec![slider.render(), Element::spacer().flex(1.0)]));
        tree.layout(Size::new(400.0, HEIGHT));
        tree
    }

    fn thumb_center(fraction: f32) -> f32 {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(Slider::new("s", fraction, palette).width(WIDTH)).layout(Size::new(400.0, HEIGHT));
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillCornerRect { rect, .. } | DrawCommand::FillRect { rect, .. }
                    if (rect.width - THUMB).abs() < 0.01 && (rect.height - THUMB).abs() < 0.01 =>
                {
                    Some(rect.x + rect.width / 2.0)
                }
                _ => None,
            })
            .expect("a thumb is painted")
    }

    fn changed(actions: &[UiAction]) -> (f32, bool) {
        match actions {
            [UiAction::SliderChanged { id, fraction, committed }] if id == "s" => (*fraction, *committed),
            other => panic!("expected one SliderChanged, got {other:?}"),
        }
    }

    #[test]
    fn the_thumb_sits_at_the_fraction_and_bad_values_are_clamped() {
        for (fraction, expected) in [(0.0, 0.0), (0.25, 0.25), (1.0, 1.0), (7.0, 1.0), (-1.0, 0.0), (f32::NAN, 0.0)] {
            let center = thumb_center(fraction);
            assert!(
                (center - (THUMB / 2.0 + expected * TRAVEL)).abs() < 0.6,
                "fraction {fraction}: thumb at {center}"
            );
        }
    }

    #[test]
    fn a_drag_previews_and_the_release_commits() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.0, palette).width(WIDTH));
        let at = |share: f32| Point::new(THUMB / 2.0 + share * TRAVEL, HEIGHT / 2.0);

        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerDown(at(0.5))));
        assert!((fraction - 0.5).abs() < 0.01 && !committed);
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerMove(at(0.8))));
        assert!((fraction - 0.8).abs() < 0.01 && !committed);
        // Past the end of the track the value stops at 1.
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerUp(Point::new(900.0, 2.0))));
        assert!((fraction - 1.0).abs() < 0.001 && committed);
        assert!(tree.handle_event(UiEvent::PointerMove(at(0.2))).is_empty(), "nothing follows the release");
    }

    #[test]
    fn a_plain_slider_leaves_the_window_idle() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.3, palette).width(WIDTH));
        let point = Point::new(100.0, HEIGHT / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point));
        tree.handle_event(UiEvent::PointerLeave);
        tree.set_root(Element::row(vec![
            Slider::new("s", 0.3, palette).width(WIDTH).render(),
            Element::spacer().flex(1.0),
        ]));
        tree.layout(Size::new(400.0, HEIGHT));
        // A player bar is on screen for hours: a control that kept the frame
        // loop running after being touched would never let the app sleep.
        std::thread::sleep(std::time::Duration::from_millis(400));
        tree.layout(Size::new(400.0, HEIGHT));
        assert_eq!(tree.animation_repaint_after(), None);
    }

    #[test]
    fn arrows_home_and_end_move_it_from_the_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.5, palette).width(WIDTH));
        assert!(tree.focus("s"));
        let key = |key| UiEvent::Key { key, shift: false, ctrl: false };
        let (fraction, committed) = changed(&tree.handle_event(key(Key::ArrowRight)));
        assert!((fraction - 0.55).abs() < 0.001 && committed);
        assert!((changed(&tree.handle_event(key(Key::ArrowLeft))).0 - 0.45).abs() < 0.001);
        assert_eq!(changed(&tree.handle_event(key(Key::Home))).0, 0.0);
        assert_eq!(changed(&tree.handle_event(key(Key::End))).0, 1.0);
        assert!(tree.handle_event(key(Key::Enter)).is_empty(), "a slider has no click");
    }

    #[test]
    fn stops_snap_on_release() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.0, palette).width(WIDTH).steps(5));
        let point = Point::new(THUMB / 2.0 + 0.6 * TRAVEL, HEIGHT / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerUp(point)));
        assert!((fraction - 0.5).abs() < 0.001 && committed, "0.6 of five stops settles on 0.5, got {fraction}");
    }

    #[test]
    fn a_disabled_slider_takes_no_pointer_or_focus() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = tree(Slider::new("s", 0.4, palette).width(WIDTH).enabled(false));
        assert!(tree.layout(Size::new(400.0, HEIGHT)).regions.is_empty());
        assert!(!tree.focus("s"));
        assert!(tree.handle_event(UiEvent::PointerDown(Point::new(60.0, HEIGHT / 2.0))).is_empty());
    }

    #[test]
    fn the_platform_is_told_a_slider_its_value_and_its_name_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let slider = Slider::new("s", 0.4, palette).width(WIDTH).enabled(enabled);
            let frame = tree(slider.accessibility_label("Volume")).layout(Size::new(400.0, HEIGHT));
            let node = frame.access.iter().find(|node| node.id == "s").expect("a node");
            assert_eq!((node.role, node.name.as_str(), node.value), (Role::Slider, "Volume", Some(0.4)));
            assert_eq!((node.disabled, node.actionable), (!enabled, enabled));
            // It shows no number, so it says none in words.
            assert_eq!(node.value_text, None);
        }
        let bare = tree(Slider::new("s", 0.4, palette)).layout(Size::new(400.0, HEIGHT));
        assert_eq!(bare.access[0].name, "", "nothing names it until the host does");
    }

    const SIZES: [&str; 5] = ["12px", "14px", "16px", "20px", "24px"];
    /// Width of a character under the measurer `frame_of` sets.
    const CHAR: f32 = 7.0;

    /// Five named stops, the middle one the default, at the test width.
    fn named(fraction: f32) -> Slider {
        Slider::new("s", fraction, ComponentPalette::neutral_dark())
            .width(WIDTH)
            .steps(5)
            .mark_labels(SIZES)
            .default_stop(2)
    }

    /// A frame of the slider at the left of a window tall enough for its labels.
    fn frame_of(slider: Slider) -> crate::Frame {
        let mut tree = UiTree::new(Element::row(vec![slider.render(), Element::spacer().flex(1.0)]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * CHAR);
        tree.layout(Size::new(400.0, 100.0))
    }

    /// Where each stop is along a slider this wide.
    fn stop_center(width: f32, stop: usize, stops: usize) -> f32 {
        THUMB / 2.0 + (width - THUMB) * stop as f32 / (stops - 1) as f32
    }

    /// Every hairline mark of the frame, in paint order, with its place among all the commands.
    fn marks(frame: &crate::Frame) -> Vec<(usize, crate::Rect, Color)> {
        frame
            .commands
            .iter()
            .enumerate()
            .filter_map(|(index, command)| match command {
                DrawCommand::FillRect { rect, color, .. } if rect.width == MARK => Some((index, *rect, *color)),
                _ => None,
            })
            .collect()
    }

    /// Every text of the frame, with where it starts and its color.
    fn labels(frame: &crate::Frame) -> Vec<(&str, Point, Color)> {
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
    fn marks_stand_on_every_stop_behind_the_thumb_in_the_border_color() {
        let palette = ComponentPalette::neutral_dark();
        let frame = frame_of(Slider::new("s", 0.5, palette).width(WIDTH).steps(5).marks());
        let drawn = marks(&frame);
        assert_eq!(drawn.len(), 5);
        let thumb = frame
            .commands
            .iter()
            .position(|command| matches!(command, DrawCommand::FillRect { rect, .. } if rect.width == THUMB))
            .expect("a thumb");
        for (stop, (index, rect, color)) in drawn.into_iter().enumerate() {
            assert!((rect.x + MARK / 2.0 - stop_center(WIDTH, stop, 5)).abs() < 0.01, "stop {stop} at {rect:?}");
            assert_eq!((rect.height, rect.y + rect.height / 2.0, color), (THUMB, HEIGHT / 2.0, palette.border));
            assert!(index < thumb, "the thumb covers the mark it rests on");
        }
        // A slider that fills its row puts them on its own stops.
        let frame = frame_of(Slider::new("s", 0.5, palette).steps(3).marks());
        let width = frame.regions[0].rect.width;
        let centers: Vec<_> = marks(&frame).iter().map(|mark| mark.1.x + MARK / 2.0).collect();
        assert_eq!(centers, [0, 1, 2].map(|stop| stop_center(width, stop, 3)));
    }

    #[test]
    fn without_stops_there_are_no_marks_and_the_thumb_stays_where_it_was() {
        let palette = ComponentPalette::neutral_dark();
        for steps in [0, 1] {
            let frame = frame_of(Slider::new("s", 0.5, palette).width(WIDTH).steps(steps).marks().mark_labels(["a", "b"]));
            assert!(marks(&frame).is_empty() && labels(&frame).is_empty(), "steps {steps}");
        }
        let marked = Slider::new("s", 0.25, palette).width(WIDTH).steps(5).marks();
        let thumb = frame_of(marked)
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, .. } if rect.width == THUMB => Some(rect.x + THUMB / 2.0),
                _ => None,
            })
            .unwrap();
        assert!((thumb - thumb_center(0.25)).abs() < 0.01);
    }

    #[test]
    fn labels_stand_over_their_stops_the_ends_flush_with_the_slider_and_the_default_in_the_ink() {
        let palette = ComponentPalette::neutral_dark();
        let frame = frame_of(named(0.5));
        let drawn = labels(&frame);
        assert_eq!(drawn.iter().map(|label| label.0).collect::<Vec<_>>(), SIZES);
        assert_eq!(marks(&frame).len(), 5, "labels draw the marks");
        let width = |text: &str| text.chars().count() as f32 * CHAR;
        assert_eq!(drawn[0].1.x, 0.0);
        assert_eq!(drawn[4].1.x + width(drawn[4].0), WIDTH);
        for (stop, &(text, at, _)) in drawn.iter().enumerate().take(4).skip(1) {
            assert!((at.x + width(text) / 2.0 - stop_center(WIDTH, stop, 5)).abs() < 0.01, "{text} at {at:?}");
        }
        let colors: Vec<_> = drawn.iter().map(|label| label.2).collect();
        let muted = palette.muted;
        assert_eq!(colors, [muted, muted, palette.text, muted, muted]);
        // The track is under them, the gap apart, and is the only thing the pointer finds.
        let [region] = frame.regions.as_slice() else { panic!("one region, got {:?}", frame.regions) };
        assert!(drawn.iter().all(|label| label.1.y < region.rect.y));
        assert_eq!((region.rect.width, region.rect.height), (WIDTH, HEIGHT));
        // An ink of the host's colors the default; disabled, every label is muted.
        let ink = Color::rgb(224, 164, 60);
        assert_eq!(labels(&frame_of(named(0.5).ink(ink)))[2].2, ink);
        assert!(labels(&frame_of(named(0.5).disabled())).iter().all(|label| label.2 == muted));
    }

    #[test]
    fn labels_past_the_last_stop_are_ignored_and_an_empty_label_or_a_stop_past_the_last_names_nothing() {
        let palette = ComponentPalette::neutral_dark();
        let slider = Slider::new("s", 0.0, palette).width(WIDTH).steps(3).mark_labels(["low", "", "high", "more"]).default_stop(7);
        let frame = frame_of(slider);
        let drawn = labels(&frame);
        assert_eq!(drawn.iter().map(|label| label.0).collect::<Vec<_>>(), ["low", "high"]);
        assert!(drawn.iter().all(|label| label.2 == palette.muted));
        assert_eq!(marks(&frame).len(), 3);
    }

    #[test]
    fn a_labelled_slider_is_dragged_on_its_track_and_its_labels_take_no_pointer() {
        let mut tree = UiTree::new(Element::row(vec![named(0.0).render(), Element::spacer().flex(1.0)]));
        let frame = tree.layout(Size::new(400.0, 100.0));
        let track = frame.regions[0].rect;
        let label = labels(&frame)[3].1;
        assert!(tree.handle_event(UiEvent::PointerDown(Point::new(label.x + 2.0, label.y + 2.0))).is_empty());
        let stop = Point::new(track.x + stop_center(WIDTH, 3, 5), track.y + HEIGHT / 2.0);
        tree.handle_event(UiEvent::PointerDown(stop));
        let (fraction, committed) = changed(&tree.handle_event(UiEvent::PointerUp(stop)));
        assert!((fraction - 0.75).abs() < 0.001 && committed);
    }

    #[test]
    fn the_platform_is_told_the_label_of_the_stop_the_value_is_on() {
        let said = |slider: Slider| {
            let frame = frame_of(slider);
            frame.access.iter().find(|node| node.id == "s").expect("a node").value_text.clone()
        };
        assert_eq!(said(named(0.5)).as_deref(), Some("16px"));
        // A drag between stops is told the nearest.
        assert_eq!(said(named(0.8)).as_deref(), Some("20px"));
        let palette = ComponentPalette::neutral_dark();
        assert_eq!(said(Slider::new("s", 0.5, palette).steps(3).mark_labels(["low", "", "high"])), None);
    }

    #[test]
    fn in_a_narrow_slider_the_labels_stay_inside_it() {
        let palette = ComponentPalette::neutral_dark();
        let slider = Slider::new("s", 0.5, palette).width(60.0).steps(3).mark_labels(["Small", "Medium", "Large"]);
        let frame = frame_of(slider);
        let drawn = labels(&frame);
        assert_eq!(drawn.len(), 3);
        for (text, at, _) in drawn {
            assert!(at.x >= 0.0 && at.x + text.chars().count() as f32 * CHAR <= 60.0, "{text} at {at:?}");
        }
    }

    #[test]
    fn a_width_narrower_than_the_thumb_is_raised_to_it() {
        let palette = ComponentPalette::neutral_dark();
        let frame = tree(Slider::new("s", 0.5, palette).width(2.0)).layout(Size::new(400.0, HEIGHT));
        let region = frame.regions.iter().find(|region| region.id == "s").expect("a region");
        assert!((region.rect.width - THUMB).abs() < 0.01);
    }
}
