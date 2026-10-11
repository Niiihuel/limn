//! The microphone control of a composer: it starts dictation, and stops it
//! while a session is live.

use crate::{
    ButtonVariant, Color, Component, ComponentPalette, ControlSize, Element, Icon, IconButton,
    Skeleton, Spinner,
};

const MARK: f32 = 10.0;

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
}

impl DictationButton {
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
        }
    }

    /// The colors of a live session: `surface` fills the circle and `text`
    /// draws the mark. Defaults to the palette's selected fill.
    pub fn live_palette(mut self, palette: ComponentPalette) -> Self {
        self.live = palette;
        self
    }

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

    /// The ink of the resting microphone under the pointer.
    pub fn hover_ink(mut self, color: Color) -> Self {
        self.hover_ink = Some(color);
        self
    }
}

impl Component for DictationButton {
    fn render(self) -> Element {
        if self.state == DictationState::Idle || !self.enabled {
            let mut button = IconButton::new(self.id, Icon::Microphone, self.palette)
                .variant(ButtonVariant::Ghost)
                .size(self.size)
                .round()
                .enabled(self.enabled);
            if let Some(ink) = self.hover_ink {
                button = button.hover_ink(ink);
            }
            return button.render();
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Point, Rect, Size, UiAction, UiEvent, UiTree};

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
