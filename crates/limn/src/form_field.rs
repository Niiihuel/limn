//! A control under its label, with the hint and the error that belong to it.

use crate::{page::px, Color, Component, ComponentPalette, Element, Text, TextRole};

/// Density of a [`FormField`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormFieldSizing {
    /// Type role of the label.
    pub label_role: TextRole,
    /// Paint the label in the palette's muted color.
    pub label_muted: bool,
    /// Least height of the label's line; `0` follows the font.
    pub label_height: f32,
    /// Type role of the hints and the error.
    pub hint_role: TextRole,
    /// Space between the label, the hints, the control and the error.
    pub gap: f32,
}

impl FormFieldSizing {
    /// A field of a settings form.
    pub const DEFAULT: Self = Self {
        label_role: TextRole::Caption,
        label_muted: false,
        label_height: 0.0,
        hint_role: TextRole::Small,
        gap: 7.0,
    };
}

/// A labelled control. The host owns the control and its value; the field
/// owns the order and spacing of the copy around it. The control fills the
/// field's width.
pub struct FormField {
    label: String,
    control: Element,
    hints: Vec<String>,
    hint_above: bool,
    error: Option<(String, Color)>,
    sizing: FormFieldSizing,
    palette: ComponentPalette,
}

impl FormField {
    /// A label over `control`, in the [`FormFieldSizing::DEFAULT`] profile.
    pub fn new(label: impl Into<String>, control: Element, palette: ComponentPalette) -> Self {
        Self {
            label: label.into(),
            control,
            hints: Vec::new(),
            hint_above: false,
            error: None,
            sizing: FormFieldSizing::DEFAULT,
            palette,
        }
    }

    /// A muted line explaining the field. Each call adds a line.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hints.push(hint.into());
        self
    }

    /// Put the hints between the label and the control instead of under it.
    pub fn hint_above(mut self, above: bool) -> Self {
        self.hint_above = above;
        self
    }

    /// A line under everything else, in a color whose meaning the host owns.
    pub fn error(mut self, error: impl Into<String>, ink: Color) -> Self {
        self.error = Some((error.into(), ink));
        self
    }

    /// Replace the density profile.
    pub fn sizing(mut self, sizing: FormFieldSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for FormField {
    fn render(self) -> Element {
        let sizing = self.sizing;
        let mut label = Text::new(self.label, self.palette).role(sizing.label_role);
        if sizing.label_muted {
            label = label.muted();
        }
        let mut label = label.render();
        if px(sizing.label_height) > 0.0 {
            label = label.min_size(0.0, sizing.label_height);
        }
        let hints = self
            .hints
            .into_iter()
            .map(|hint| Text::new(hint, self.palette).role(sizing.hint_role).muted().render());
        let mut children = vec![label];
        if self.hint_above {
            children.extend(hints);
            children.push(self.control);
        } else {
            children.push(self.control);
            children.extend(hints);
        }
        if let Some((error, ink)) = self.error {
            children.push(Text::new(error, self.palette).role(sizing.hint_role).ink(ink).render());
        }
        Element::column(children).gap(px(sizing.gap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, TextField, UiTree};

    const DANGER: Color = Color::rgb(200, 60, 60);

    fn frame(field: FormField, width: f32) -> crate::Frame {
        let mut tree = UiTree::new(Element::column(vec![field.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(width, 400.0))
    }

    fn text_y(frame: &crate::Frame, wanted: &str) -> f32 {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == wanted => Some(position.y),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{wanted}"))
    }

    fn field(palette: ComponentPalette) -> FormField {
        FormField::new("Name", TextField::new("name", "", "", palette).render(), palette)
    }

    #[test]
    fn label_hint_control_and_error_stack_in_order_and_the_control_fills_the_width() {
        let palette = ComponentPalette::neutral_dark();
        for width in [180.0, 360.0, 900.0] {
            let below = frame(field(palette).hint("Hint").error("Wrong", DANGER), width);
            let control = below.regions.iter().find(|region| region.id == "name").unwrap().rect;
            assert_eq!((control.x, control.width), (0.0, width));
            assert!(text_y(&below, "Name") < control.y);
            assert!(control.y < text_y(&below, "Hint"));
            assert!(text_y(&below, "Hint") < text_y(&below, "Wrong"));

            let above = frame(field(palette).hint("Hint").hint("More").hint_above(true), width);
            let control = above.regions.iter().find(|region| region.id == "name").unwrap().rect;
            assert!(text_y(&above, "Name") < text_y(&above, "Hint"));
            assert!(text_y(&above, "Hint") < text_y(&above, "More"));
            assert!(text_y(&above, "More") < control.y);
        }
    }

    #[test]
    fn copy_takes_its_size_and_color_from_the_profile_the_palette_and_the_ink() {
        let palette = ComponentPalette::neutral_dark();
        let paint = |frame: &crate::Frame, wanted: &str| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, color, font_size, .. } if text == wanted => {
                        Some((*color, *font_size))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let default = frame(field(palette).hint("Hint").error("Wrong", DANGER), 300.0);
        assert_eq!(paint(&default, "Name"), (palette.text, 12.0));
        assert_eq!(paint(&default, "Hint"), (palette.muted, 11.0));
        assert_eq!(paint(&default, "Wrong"), (DANGER, 11.0));

        let sizing = FormFieldSizing {
            label_role: TextRole::Label,
            label_muted: true,
            label_height: 30.0,
            hint_role: TextRole::Caption,
            gap: 8.0,
        };
        let custom = frame(field(palette).hint("Hint").sizing(sizing), 300.0);
        assert_eq!(paint(&custom, "Name"), (palette.muted, 13.0));
        assert_eq!(paint(&custom, "Hint"), (palette.muted, 12.0));
        let control = custom.regions.iter().find(|region| region.id == "name").unwrap().rect;
        assert_eq!(control.y, 38.0, "label line of 30 and a gap of 8");
    }

    #[test]
    fn a_bad_profile_number_does_not_reach_the_layout() {
        let palette = ComponentPalette::neutral_dark();
        for bad in [f32::NAN, f32::INFINITY, -4.0] {
            let sizing = FormFieldSizing { gap: bad, label_height: bad, ..FormFieldSizing::DEFAULT };
            let bad_frame = frame(field(palette).sizing(sizing), 300.0);
            let zero = FormFieldSizing { gap: 0.0, ..FormFieldSizing::DEFAULT };
            assert_eq!(bad_frame.commands, frame(field(palette).sizing(zero), 300.0).commands);
        }
    }
}
