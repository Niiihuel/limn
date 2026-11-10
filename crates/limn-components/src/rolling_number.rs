//! A number whose digits roll to their new values.

use crate::{Color, Component, ComponentPalette, Element, FontFamily};

/// Text in which every digit is a column of 0 to 9 seen through a window one
/// line tall: when the value changes, each digit rolls up or down to its new
/// one while the other characters stay where they are. A digit is known by its
/// place from the right, so "99" becoming "100" rolls the two nines to zero and
/// brings in the one. Only the paint moves; the slot is one line tall.
pub struct RollingNumber {
    id: String,
    text: String,
    color: Color,
    size: f32,
    family: FontFamily,
}

impl RollingNumber {
    /// `text`, such as "5,318" or "+12", in the palette's text color at 13
    /// pixels in the sans family. `id` keeps each digit's roll across
    /// rebuilds.
    pub fn new(id: impl Into<String>, text: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            color: palette.text,
            size: 13.0,
            family: FontFamily::Sans,
        }
    }

    /// The color of every character.
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The text's size in logical pixels; one that is not finite or not above
    /// zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.size = size;
        }
        self
    }

    /// The family the text is set in; digits keep one width in either.
    pub fn font_family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    /// The height of the window each digit is seen through.
    fn line(&self) -> f32 {
        (self.size * 1.25).round()
    }
}

impl Component for RollingNumber {
    fn render(self) -> Element {
        let line = self.line();
        let glyph = |text: String| {
            Element::text(text)
                .foreground(self.color)
                .font_family(self.family)
                .font_size(self.size)
                .size(None, Some(line))
        };
        let count = self.text.chars().count();
        let cells = self
            .text
            .chars()
            .enumerate()
            .map(|(at, ch)| match ch.to_digit(10) {
                Some(digit) => Element::column(vec![Element::column(
                    (0..10).map(|n| glyph(n.to_string())).collect(),
                )
                // Its full height, which the one-line window around it clips.
                .size(None, Some(line * 10.0))
                .animate_slide(
                    format!("{}/{}", self.id, count - at),
                    -(digit as f32) * line,
                )])
                .size(None, Some(line))
                .clip_children(),
                None => glyph(ch.to_string()),
            })
            .collect();
        Element::row(cells)
            .size(None, Some(line))
            .accessibility_label(self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};
    use web_time::{Duration, Instant};

    const SIZE: Size = Size::new(300.0, 100.0);

    fn root(text: &str) -> Element {
        RollingNumber::new("added", text, ComponentPalette::neutral_dark()).render()
    }

    /// What each digit's window shows, left to right: its texts and where they sit.
    fn windows(frame: &crate::Frame) -> Vec<Vec<(String, f32)>> {
        let mut windows: Vec<Vec<(String, f32)>> = vec![];
        let mut open = false;
        for command in &frame.commands {
            match command {
                DrawCommand::PushClip(rect) => {
                    assert_eq!(rect.height, 16.0, "a window is one line tall");
                    windows.push(vec![]);
                    open = true;
                }
                DrawCommand::PopClip => open = false,
                DrawCommand::Text { position, text, .. } if open => {
                    windows.last_mut().unwrap().push((text.clone(), position.y));
                }
                _ => {}
            }
        }
        windows
    }

    fn at_rest(digit: &str) -> Vec<(String, f32)> {
        vec![(digit.to_owned(), 0.0)]
    }

    /// Half of a roll's three moves.
    fn half() -> Duration {
        Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3 / 2)
    }

    #[test]
    fn each_digit_shows_its_own_value_and_other_characters_stand_beside_them() {
        let mut tree = UiTree::new(root("+12"));
        let frame = tree.layout(SIZE);
        assert_eq!(windows(&frame), vec![at_rest("1"), at_rest("2")]);
        assert!(frame.commands.iter().any(
            |command| matches!(command, DrawCommand::Text { text, position, .. } if text == "+" && position.x == 0.0)
        ));
        assert!(!tree.is_animating());
    }

    #[test]
    fn a_changed_digit_rolls_to_its_new_value_and_the_rest_stay() {
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(root("12"));
        tree.layout_at(SIZE, start);
        tree.set_root(root("15"));
        tree.layout_at(SIZE, start);
        let frame = tree.layout_at(SIZE, start + half());
        let shown = windows(&frame);
        assert_eq!(shown[0], at_rest("1"));
        // The units are between 2 and 5, partway up the column.
        assert!(shown[1].iter().all(|(digit, _)| ("2"..="5").contains(&digit.as_str())), "{shown:?}");
        assert_ne!(shown[1], at_rest("2"));
        assert_ne!(shown[1], at_rest("5"));
        let frame = tree.layout_at(SIZE, start + half() * 6);
        assert_eq!(windows(&frame), vec![at_rest("1"), at_rest("5")]);
        assert!(!tree.is_animating());
    }

    #[test]
    fn digits_are_known_by_their_place_from_the_right() {
        let start = Instant::now() - Duration::from_secs(10);
        let mut tree = UiTree::new(root("9"));
        tree.layout_at(SIZE, start);
        tree.set_root(root("10"));
        tree.layout_at(SIZE, start);
        let shown = windows(&tree.layout_at(SIZE, start + half() / 4));
        // The new tens arrive at 1; the units roll back down from 9 towards 0.
        assert_eq!(shown[0], at_rest("1"));
        assert_ne!(shown[1], at_rest("9"));
        assert_ne!(shown[1], at_rest("0"));
        let shown = windows(&tree.layout_at(SIZE, start + half() * 6));
        assert_eq!(shown, vec![at_rest("1"), at_rest("0")]);
    }

    #[test]
    fn reduced_motion_shows_the_new_value_at_once() {
        let mut tree = UiTree::new(root("12"));
        tree.set_reduced_motion(true);
        tree.layout(SIZE);
        tree.set_root(root("17"));
        assert_eq!(windows(&tree.layout(SIZE)), vec![at_rest("1"), at_rest("7")]);
        assert!(!tree.is_animating());
    }
}
