//! The microphone control of a composer: it starts dictation, and stops it
//! while a session is live. Beside it, the level of what the microphone hears.

use crate::{
    Access, ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, Icon, IconButton,
    LevelBars, Skeleton, Spinner,
};

const MARK: f32 = 10.0;
/// A bar of the waveform and the space between two.
const BAR: f32 = 2.0;
const BAR_GAP: f32 = 2.0;
/// Two seconds of levels sampled every 80 ms.
const WAVEFORM_WIDTH: f32 = 96.0;

/// Where the host's dictation session stands.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DictationState {
    /// No session: the button offers to start one.
    #[default]
    Idle,
    /// The microphone is opening. A click cancels.
    Starting,
    /// The microphone is open. A click stops.
    Listening,
    /// The microphone has closed and the last speech is still being
    /// transcribed. A click cancels.
    Transcribing,
}

/// A round microphone button that becomes a stop control while a session is
/// live. The host owns the session and the transcript; every state reports
/// `UiAction::Clicked` with the one ID, so the host toggles on it.
pub struct DictationButton {
    id: String,
    state: DictationState,
    palette: ComponentPalette,
    live: ComponentPalette,
    size: ControlSize,
    enabled: bool,
    hover_ink: Option<Color>,
    accessibility_label: Option<String>,
}

impl DictationButton {
    /// An enabled button of regular size in the given state, with no name for the platform.
    /// A click reports `id` in every state.
    pub fn new(id: impl Into<String>, state: DictationState, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            state,
            palette,
            live: ComponentPalette {
                surface: palette.selected,
                ..palette
            },
            size: ControlSize::Regular,
            enabled: true,
            hover_ink: None,
            accessibility_label: None,
        }
    }

    /// What the button is called to the platform. It is a mark alone in
    /// every state, so without this it has no name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// The colors of a live session: `surface` fills the circle and `text`
    /// draws the mark. Defaults to the palette's selected fill.
    pub fn live_palette(mut self, palette: ComponentPalette) -> Self {
        self.live = palette;
        self
    }

    /// Set the button's control size; `ControlSize::Regular` by default.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Without a dictation backend the button is a passive microphone, in
    /// any state.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// The ink of the resting microphone under the pointer.
    pub fn hover_ink(mut self, color: Color) -> Self {
        self.hover_ink = Some(color);
        self
    }
}

impl Component for DictationButton {
    fn render(self) -> Element {
        // On while a session is live, as it is drawn: disabled it is the
        // resting microphone in any state.
        let mut access = Access::default().toggled(self.enabled && self.state != DictationState::Idle);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
        }
        if self.state == DictationState::Idle || !self.enabled {
            let mut button = IconButton::new(self.id, Icon::Microphone, self.palette)
                .variant(ButtonVariant::Ghost)
                .size(self.size)
                .round()
                .enabled(self.enabled);
            if let Some(ink) = self.hover_ink {
                button = button.hover_ink(ink);
            }
            return button.render().accessible(access);
        }
        let ink = self.live.text;
        let (mark, mark_side) = if self.state != DictationState::Listening {
            let side = self.size.button_sizing().icon_size;
            let spinner = Spinner::new(format!("{}:busy", self.id), ink).size(side);
            (spinner.render(), side)
        } else {
            // The stop square is a `Skeleton`, so it breathes on the library's
            // one pulse and holds at full ink under reduced motion.
            let pulse = ComponentPalette {
                raised: ink,
                hover: Color::rgba(ink.r, ink.g, ink.b, ink.a / 3),
                ..self.live
            };
            let square = Skeleton::new(format!("{}:pulse", self.id), pulse)
                .width(MARK)
                .height(MARK)
                .radius(crate::tokens().radius.control.min(MARK / 4.0))
                .render();
            (square, MARK)
        };
        let side = self.size.height();
        let centered = Element::column(vec![
            Element::spacer().flex(1.0),
            Element::row(vec![
                Element::spacer().flex(1.0),
                mark,
                Element::spacer().flex(1.0),
            ])
            .size(None, Some(mark_side)),
            Element::spacer().flex(1.0),
        ]);
        // Half the height is a shape, not a token.
        Element::clickable(self.id, centered)
            .background(self.live.surface)
            .radius(side / 2.0)
            .size(Some(side), Some(side))
            .accessible(access)
    }
}

/// The input level of a live session: a bar for each level the host
/// sampled, the newest at the trailing edge and as many before it as fit.
/// Silence, and a session that has not started, is a row of dots. The host
/// owns the microphone and samples it; this only draws what it is given.
pub struct DictationWaveform {
    id: String,
    levels: Vec<f32>,
    palette: ComponentPalette,
    width: f32,
    size: ControlSize,
}

impl DictationWaveform {
    /// `levels` run from 0 to 1, oldest first. One that is not finite is
    /// drawn as silence; one outside the range is clamped.
    pub fn new(id: impl Into<String>, levels: &[f32], palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            levels: levels.to_vec(),
            palette,
            width: WAVEFORM_WIDTH,
            size: ControlSize::Regular,
        }
    }

    /// The width the bars are drawn in, 96 by default. A width that is not
    /// finite or not positive is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width;
        }
        self
    }

    /// The height of a control of this size, so it lines up with the
    /// `DictationButton` beside it.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }
}

impl Component for DictationWaveform {
    fn render(self) -> Element {
        Element::level_bars(LevelBars {
            id: self.id,
            levels: self.levels,
            bar: BAR,
            gap: BAR_GAP,
            color: self.palette.text,
            rest: self.palette.muted,
        })
        .size(Some(self.width), Some(self.size.height()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point, Rect, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    const WAVE: Size = Size::new(120.0, 28.0);

    fn waveform(levels: &[f32]) -> Element {
        Element::row(vec![
            DictationWaveform::new("wave", levels, ComponentPalette::neutral_dark())
                .size(ControlSize::Compact)
                .render(),
            Element::spacer().flex(1.0),
        ])
    }

    fn bars(frame: &crate::Frame) -> Vec<(Rect, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if rect.width == BAR => Some((*rect, *color)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn silence_is_a_row_of_dots_and_the_newest_level_stands_at_the_trailing_edge() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(waveform(&[]));
        tree.set_reduced_motion(true);
        let dots = bars(&tree.layout(WAVE));
        assert_eq!(dots.len(), 24, "96 wide holds 24 bars of 2 set 2 apart");
        assert!(dots.iter().all(|(rect, color)| rect.height == BAR && rect.y == 13.0 && *color == palette.muted));
        assert_eq!((dots[0].0.x, dots[23].0.x + BAR), (2.0, 96.0), "they end at the trailing edge");

        tree.set_root(waveform(&[0.5, 1.0]));
        let spoken = bars(&tree.layout(WAVE));
        assert!(spoken[..22].iter().all(|(rect, _)| rect.height == BAR));
        assert_eq!((spoken[22].0.y, spoken[22].0.height), (6.5, 15.0), "half way between a dot and the height");
        assert_eq!((spoken[23].0.y, spoken[23].0.height, spoken[23].1), (0.0, 28.0, palette.text));
    }

    #[test]
    fn a_bar_springs_to_its_level_and_follows_it_to_the_next_slot() {
        let mut tree = UiTree::new(waveform(&[]));
        let start = Instant::now();
        tree.layout_at(WAVE, start);
        tree.set_root(waveform(&[1.0]));
        tree.layout_at(WAVE, start);
        let rising = bars(&tree.layout_at(WAVE, start + Duration::from_millis(40)))[23].0.height;
        assert!(rising > BAR && rising < 28.0, "on its way: {rising}");

        // The level moves on a slot before the bar has arrived: both are in motion.
        tree.set_root(waveform(&[1.0, 0.0]));
        let turned = bars(&tree.layout_at(WAVE, start + Duration::from_millis(80)))[23].0.height;
        assert!(turned > rising && turned < 28.0, "it kept rising until the level left: {turned}");
        let moved = bars(&tree.layout_at(WAVE, start + Duration::from_millis(120)));
        assert!(moved[22].0.height > BAR && moved[22].0.height < 28.0);
        assert!(moved[23].0.height > BAR && moved[23].0.height < turned, "it falls from where it had reached");

        let settled = bars(&tree.layout_at(WAVE, start + Duration::from_secs(2)));
        assert_eq!((settled[22].0.height, settled[23].0.height), (28.0, BAR));
    }

    #[test]
    fn levels_that_do_not_fit_are_the_oldest_and_bad_numbers_never_reach_the_paint() {
        let mut tree = UiTree::new(waveform(&[f32::NAN, 7.0, -3.0]));
        tree.set_reduced_motion(true);
        let drawn = bars(&tree.layout(WAVE));
        assert_eq!(drawn[21..].iter().map(|(rect, _)| rect.height).collect::<Vec<_>>(), [BAR, 28.0, BAR]);

        let mut long = vec![1.0; 3];
        long.extend([0.0; 24]);
        tree.set_root(waveform(&long));
        assert!(bars(&tree.layout(WAVE)).iter().all(|(rect, _)| rect.height == BAR), "the three loud ones scrolled out");

        for width in [f32::NAN, f32::INFINITY, 0.0, -8.0] {
            let element = DictationWaveform::new("wave", &[], ComponentPalette::neutral_dark()).width(width).render();
            let mut tree = UiTree::new(Element::row(vec![element, Element::spacer().flex(1.0)]));
            assert_eq!(bars(&tree.layout(Size::new(200.0, 40.0))).len(), 24, "{width}");
        }
        // Narrower than a bar there is nothing to draw, and nothing breaks.
        let element = DictationWaveform::new("wave", &[1.0], ComponentPalette::neutral_dark()).width(1.0).render();
        let mut tree = UiTree::new(Element::row(vec![element, Element::spacer().flex(1.0)]));
        assert!(bars(&tree.layout(Size::new(200.0, 40.0))).is_empty());
    }

    fn tree(state: DictationState, enabled: bool) -> UiTree {
        UiTree::new(Element::row(vec![
            DictationButton::new("mic", state, ComponentPalette::neutral_dark())
                .size(ControlSize::Compact)
                .enabled(enabled)
                .render(),
            Element::spacer().flex(1.0),
        ]))
    }

    fn hit(tree: &mut UiTree) -> Option<Rect> {
        let frame = tree.layout(Size::new(120.0, 28.0));
        frame
            .regions
            .iter()
            .find(|region| region.id == "mic")
            .map(|region| region.rect)
    }

    #[test]
    fn the_platform_is_told_a_button_that_is_on_while_a_session_is_live() {
        let palette = ComponentPalette::neutral_dark();
        let node = |state, enabled| {
            let button = DictationButton::new("mic", state, palette).enabled(enabled).accessibility_label("Dictate");
            let frame = UiTree::new(Element::row(vec![button.render(), Element::spacer().flex(1.0)]))
                .layout(Size::new(120.0, 40.0));
            frame.access.iter().find(|node| node.id == "mic").cloned().expect("a node")
        };
        for (state, live) in [
            (DictationState::Idle, false),
            (DictationState::Starting, true),
            (DictationState::Listening, true),
            (DictationState::Transcribing, true),
        ] {
            let said = node(state, true);
            assert_eq!((said.role, said.name.as_str(), said.toggled), (crate::Role::Button, "Dictate", Some(live)));
            assert!(said.actionable && !said.disabled);
            // Disabled it is drawn at rest whatever the state.
            let off = node(state, false);
            assert_eq!((off.role, off.name.as_str(), off.toggled), (crate::Role::Button, "Dictate", Some(false)));
            assert!(off.disabled && !off.actionable);
        }
        let bare = UiTree::new(DictationButton::new("mic", DictationState::Listening, palette).render())
            .layout(Size::new(120.0, 40.0));
        assert_eq!(bare.access[0].name, "", "nothing names it until the host does");
    }

    #[test]
    fn every_state_is_one_circle_that_reports_the_same_click() {
        for state in [
            DictationState::Idle,
            DictationState::Starting,
            DictationState::Listening,
            DictationState::Transcribing,
        ] {
            let mut tree = tree(state, true);
            let rect = hit(&mut tree).unwrap();
            assert_eq!((rect.x, rect.y, rect.width, rect.height), (0.0, 0.0, 28.0, 28.0));
            let point = Point::new(14.0, 14.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "mic".into() }],
                "{state:?}"
            );
        }
    }

    #[test]
    fn a_live_session_animates_and_holds_still_under_reduced_motion() {
        assert!(!tree(DictationState::Idle, true).is_animating());
        for state in [
            DictationState::Starting,
            DictationState::Listening,
            DictationState::Transcribing,
        ] {
            let mut tree = tree(state, true);
            let frame = tree.layout(Size::new(120.0, 28.0));
            assert!(tree.is_animating(), "{state:?}");
            assert_eq!(
                frame
                    .commands
                    .iter()
                    .any(|command| matches!(command, DrawCommand::Spinner { .. })),
                state != DictationState::Listening
            );
            tree.set_reduced_motion(true);
            tree.layout(Size::new(120.0, 28.0));
            assert!(!tree.is_animating(), "{state:?}");
        }
    }

    #[test]
    fn disabled_is_a_passive_microphone_in_any_state() {
        for state in [DictationState::Idle, DictationState::Listening] {
            let mut tree = tree(state, false);
            assert_eq!(hit(&mut tree), None);
            assert!(!tree.is_animating());
        }
    }
}
