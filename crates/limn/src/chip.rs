//! Inline counts, labels and states on a fixed-height pill surface.

use crate::{Access, AlertColors, AlertTone, Color, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon, Role};

/// Space between the ends of the pill and its content.
const INSET: f32 = 8.0;
/// Space between a leading icon or dot and the label.
const LEADING_GAP: f32 = 4.0;
/// Diameter of the leading dot.
const DOT_SIZE: f32 = 6.0;
/// Space between the remove button and the edge of the pill, on every side.
const REMOVE_INSET: f32 = 2.0;
/// How much of the accent a toned chip's fill and edge take over the surface, out of 255.
const TINT_FILL: u16 = 40;
const TINT_EDGE: u16 = 96;

/// The height and the label's text size for a control size. `Regular` is the
/// chip as it stands without a size.
const fn metrics(size: ControlSize) -> (f32, f32) {
    match size {
        ControlSize::Dense => (18.0, 10.0),
        ControlSize::Compact => (20.0, 11.0),
        ControlSize::Regular => (22.0, 11.0),
        ControlSize::Spacious => (26.0, 12.0),
    }
}

/// What stands before the label.
enum Leading {
    Icon(Icon),
    Dot,
}

/// A compact informational label. Place it in an inline row so its width follows
/// the measured text; a one-digit ASCII count alone uses a circular surface instead.
/// The chip registers no action and receives no focus unless `remove_action`
/// gives it a remove button, which is its only control.
pub struct Chip {
    label: String,
    palette: ComponentPalette,
    height: f32,
    font_size: f32,
    outline: bool,
    accent: Option<Color>,
    leading: Option<Leading>,
    remove_action: Option<String>,
    enabled: bool,
}

impl Chip {
    /// A filled chip 22 logical pixels tall that shows `label`.
    pub fn new(label: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            label: label.into(),
            palette,
            height: 22.0,
            font_size: 11.0,
            outline: false,
            accent: None,
            leading: None,
            remove_action: None,
            enabled: true,
        }
    }

    /// Draw the border only, with no fill, for a label that should sit
    /// quietly on its surface. With a tone or a color, the border and the
    /// label take it.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the fixed height, in logical pixels (22 by default, at least 16).
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(16.0);
        }
        self
    }

    /// Set the height and the label's text size together: 18 and 10 pixels
    /// for `Dense`, 20 and 11 for `Compact`, 22 and 11 for `Regular`, which is
    /// the chip without a size, 26 and 12 for `Spacious`. A later `height`
    /// changes the height alone.
    pub fn size(mut self, size: ControlSize) -> Self {
        (self.height, self.font_size) = metrics(size);
        self
    }

    /// Say a state with the color an [`Alert`](crate::Alert) of this tone
    /// gives its icon, `AlertColors::for_tone(tone, palette).accent`: a tint
    /// of it as the fill, under a label in it. `AlertTone::Neutral` is the
    /// chip without a tone. It replaces an earlier `color`.
    pub fn tone(mut self, tone: AlertTone) -> Self {
        self.accent = (tone != AlertTone::Neutral).then(|| AlertColors::for_tone(tone, self.palette).accent);
        self
    }

    /// A color of the host's own, drawn as a tone is: a tint of it as the
    /// fill, under a label in it. The label is moved toward white or black
    /// until it reads on the fill at a contrast of 4.5, and the border of an
    /// outline chip until it shows on the palette's surface at 3. It replaces
    /// an earlier `tone`.
    pub fn color(mut self, color: Color) -> Self {
        self.accent = Some(readable_ink(color, self.palette.surface, 3.0));
        self
    }

    /// Draw this icon before the label, in the label's color. It replaces a `dot`.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.leading = Some(Leading::Icon(icon));
        self
    }

    /// Draw a circle 6 pixels across before the label, in the label's color:
    /// the mark of a state such as "online" or "failed". It replaces an `icon`.
    pub fn dot(mut self) -> Self {
        self.leading = Some(Leading::Dot);
        self
    }

    /// Add a round close button after the label that reports
    /// `UiAction::Clicked` with this ID. Its name to the platform is
    /// `Labels::remove` followed by the label. The chip does not remove itself.
    pub fn remove_action(mut self, id: impl Into<String>) -> Self {
        self.remove_action = Some(id.into());
        self
    }

    /// A disabled chip is drawn in the muted color, and its remove button is
    /// passive: it keeps its place, has no hit region and reports nothing.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

impl Component for Chip {
    fn render(self) -> Element {
        let palette = self.palette;
        let (height, font_size) = (self.height, self.font_size);
        // The fill, the edge and the ink. A tint is mixed over the palette's
        // surface, the ground an alert of the same tone is read on.
        let (fill, edge, ink) = match (self.accent, self.outline) {
            (None, outline) => ((!outline).then_some(palette.selected), palette.border, palette.text),
            (Some(accent), true) => (None, accent, readable_ink(accent, palette.surface, 4.5)),
            (Some(accent), false) => {
                let fill = blend(palette.surface, accent, TINT_FILL);
                (Some(fill), blend(palette.surface, accent, TINT_EDGE), readable_ink(accent, fill, 4.5))
            }
        };
        let ink = if self.enabled { ink } else { palette.muted };
        let decorated = self.leading.is_some() || self.remove_action.is_some();
        let circular_count = !decorated && self.label.len() == 1 && self.label.as_bytes()[0].is_ascii_digit();
        let remove = self.remove_action.map(|id| {
            let side = (height - 2.0 * REMOVE_INSET).max(0.0);
            let glyph_side = (side * 0.6).round();
            let glyph = Element::column(vec![
                Element::spacer().flex(1.0),
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    Element::icon(Icon::CloseBold).foreground(ink).size(Some(glyph_side), Some(glyph_side)),
                    Element::spacer().flex(1.0),
                ])
                .size(None, Some(glyph_side)),
                Element::spacer().flex(1.0),
            ]);
            // An icon alone: it is named for what it removes.
            let said = Access::new(Role::Button).name(format!("{} {}", crate::labels().remove, self.label).trim_end());
            if self.enabled {
                // A veil of the ink shows on any fill, toned or not.
                Element::clickable(id, glyph)
                    .hover_background(Color { a: 36, ..ink })
                    .pressed_background(Color { a: 60, ..ink })
                    .accessible(said)
            } else {
                Element::panel(glyph).accessible(said.id(id).disabled(true))
            }
            .radius(side / 2.0)
            .size(Some(side), Some(side))
            .centered_y()
        });
        let centered = Element::text(self.label)
            .font_family(FontFamily::Sans)
            .font_size(font_size)
            .foreground(ink)
            .center_content()
            .centered_y();
        let content = if circular_count {
            centered
        } else {
            // Keep the label at its measured width. Reconstructing that width
            // by subtracting padding from a flex allocation can lose an f32
            // unit and wrap a label which fitted during intrinsic measurement.
            let mut row = vec![if remove.is_some() {
                // Two flexible slots share what both hold, which would take
                // from this inset: with a button the label keeps to the start.
                Element::spacer().size(Some(INSET), None)
            } else {
                Element::spacer().min_size(INSET, 0.0).flex(1.0)
            }];
            if let Some(leading) = self.leading {
                row.push(match leading {
                    Leading::Icon(icon) => {
                        let side = font_size + 2.0;
                        Element::icon(icon).foreground(ink).size(Some(side), Some(side))
                    }
                    Leading::Dot => Element::panel(Element::spacer())
                        .background(ink)
                        .radius(DOT_SIZE / 2.0)
                        .size(Some(DOT_SIZE), Some(DOT_SIZE)),
                }
                .centered_y());
                row.push(Element::spacer().size(Some(LEADING_GAP), None));
            }
            row.push(centered);
            match remove {
                // The button keeps to the end of the pill, concentric with its cap.
                Some(remove) => row.extend([
                    Element::spacer().min_size(REMOVE_INSET, 0.0).flex(1.0),
                    remove,
                    Element::spacer().size(Some(REMOVE_INSET), None),
                ]),
                None => row.push(Element::spacer().min_size(INSET, 0.0).flex(1.0)),
            }
            Element::row(row)
        };
        let mut surface = Element::panel(content);
        if let Some(fill) = fill {
            surface = surface.background(fill);
        }
        surface
            .border(edge, 1.0)
            .radius(height / 2.0)
            .min_size(height, height)
            .size(circular_count.then_some(height), Some(height))
    }
}

fn blend(from: Color, to: Color, amount: u16) -> Color {
    let channel = |a: u8, b: u8| ((u16::from(a) * (255 - amount) + u16::from(b) * amount + 127) / 255) as u8;
    Color::rgb(channel(from.r, to.r), channel(from.g, to.g), channel(from.b, to.b))
}

pub(crate) fn contrast(a: Color, b: Color) -> f32 {
    let luminance = |color: Color| {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// `candidate` as it looks on `surface`, moved toward white or black until it
/// stands at `minimum` contrast on it. The rule `AlertColors::for_tone` keeps
/// its inks to, so a chip and an alert of one tone correct a color alike.
pub(crate) fn readable_ink(candidate: Color, surface: Color, minimum: f32) -> Color {
    let candidate = blend(surface, Color::rgb(candidate.r, candidate.g, candidate.b), u16::from(candidate.a));
    if contrast(candidate, surface) >= minimum {
        return candidate;
    }
    let black = Color::rgb(0, 0, 0);
    let end = if contrast(Color::WHITE, surface) >= contrast(black, surface) { Color::WHITE } else { black };
    (8..=248)
        .step_by(8)
        .map(|amount| blend(candidate, end, amount))
        .find(|ink| contrast(*ink, surface) >= minimum)
        .unwrap_or(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, FontMetrics, Frame, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn installed_labels_remain_complete_on_one_line_inside_a_centered_row() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["Instalada · Usuario", "Instalada · Proyecto"] {
            for row_height in [28.0, 44.0] {
                let control = Chip::new(label, palette).render();
                let centered = Element::column(vec![
                    Element::spacer().flex(1.0),
                    control,
                    Element::spacer().flex(1.0),
                ]);
                let mut tree = UiTree::new(
                    Element::row(vec![centered, Element::spacer().flex(1.0)])
                        .size(None, Some(row_height)),
                );
                let frame = tree.layout(Size::new(920.0, row_height));
                let surface = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, color, .. } if *color == palette.selected => {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                let blocks = frame.text_layouts();
                let block = blocks.iter().find(|block| block.source == label).unwrap();
                assert_eq!(block.runs.len(), 1, "{label} at row height {row_height}");
                let text = &block.runs[0];
                assert_eq!(text.text, label);
                assert_eq!(surface.height, 22.0);
                assert!((surface.width - text.rect.width - 16.0).abs() < 0.01);
                assert!((text.rect.x - surface.x - 8.0).abs() < 0.01);
                assert!(text.rect.y >= surface.y);
                assert!(text.rect.y + text.rect.height <= surface.y + surface.height);
                assert!((surface.y + surface.height / 2.0 - row_height / 2.0).abs() < 0.01);
                assert!(
                    (text.rect.y + text.rect.height / 2.0 - surface.y - surface.height / 2.0).abs()
                        < 0.01
                );
            }
        }
    }

    #[test]
    fn counts_and_unicode_labels_use_measured_width_and_centered_copy() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["1", "12", "999", "Reparación · 模型 🦀", "W", "終"] {
            for row_height in [28.0, 44.0] {
                let mut tree = UiTree::new(
                    Element::row(vec![Chip::new(label, palette).render()])
                        .size(None, Some(row_height)),
                );
                // Exercise the host's proportional measurement rather than
                // calculating a label width from a guessed character count.
                tree.set_text_measurer(|text, size, _| {
                    text.chars()
                        .map(|character| match character {
                            '1' => 4.0,
                            'W' => 10.0,
                            '終' | '模' | '型' | '🦀' => 11.0,
                            _ => 6.0,
                        })
                        .sum::<f32>()
                        * size
                        / 11.0
                });
                tree.set_sans_metrics(FontMetrics {
                    char_width: 7.0,
                    line_height: 18.0,
                });
                let frame = tree.layout(Size::new(600.0, row_height));
                let (surface, radius) = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect {
                            rect,
                            color,
                            radius,
                        } if *color == palette.selected => Some((*rect, *radius)),
                        _ => None,
                    })
                    .unwrap();
                let blocks = frame.text_layouts();
                let block = blocks.iter().find(|block| block.source == label).unwrap();
                assert_eq!(block.runs.len(), 1);
                let text = &block.runs[0];
                assert_eq!(text.text, label);
                assert_eq!(surface.height, 22.0);
                assert_eq!(radius, surface.height / 2.0);
                if label == "1" {
                    assert_eq!(surface.width, surface.height);
                } else {
                    assert!((surface.width - (text.rect.width + 16.0).max(22.0)).abs() < 0.1);
                }
                assert!(
                    (text.rect.x + text.rect.width / 2.0 - surface.x - surface.width / 2.0).abs()
                        < 0.1
                );
                assert!(
                    (text.rect.y + text.rect.height / 2.0 - surface.y - surface.height / 2.0).abs()
                        < 0.1
                );
                assert!(text.visible.is_some());
                assert!(frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { font_family: FontFamily::Sans, font_size, .. } if *font_size == 11.0
                )));
            }
        }
    }

    #[test]
    fn outline_keeps_the_border_and_geometry_without_a_fill() {
        let palette = ComponentPalette::neutral_dark();
        let frame_of = |chip: Chip| {
            UiTree::new(Element::row(vec![chip.height(20.0).render()])).layout(Size::new(300.0, 32.0))
        };
        let filled = frame_of(Chip::new("Plan Pro", palette));
        let outline = frame_of(Chip::new("Plan Pro", palette).outline());
        assert!(!outline.commands.iter().any(|command| matches!(command, DrawCommand::FillRect { .. })));
        let without_fill: Vec<_> = filled
            .commands
            .iter()
            .filter(|command| !matches!(command, DrawCommand::FillRect { .. }))
            .cloned()
            .collect();
        assert_eq!(outline.commands, without_fill);
        assert!(outline.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { rect, color, radius, .. }
                if *color == palette.border && *radius == 10.0 && rect.height == 20.0
        )));
        assert!(outline.regions.is_empty());
    }

    #[test]
    fn chip_remains_passive_under_pointer_and_keyboard_events() {
        let mut tree = UiTree::new(Element::row(vec![Chip::new(
            "1",
            ComponentPalette::neutral_dark(),
        )
        .render()]));
        let frame = tree.layout(Size::new(120.0, 44.0));
        assert!(frame.regions.is_empty());
        assert_eq!(tree.focused_id(), None);
        assert!(!tree.focus("chip"));
        let point = Point::new(11.0, 11.0);
        for event in [
            UiEvent::PointerMove(point),
            UiEvent::PointerDown(point),
            UiEvent::PointerUp(point),
            UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
                alt: false,
            },
            UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
                alt: false,
            },
        ] {
            assert!(tree.handle_event(event).is_empty());
        }
        assert_eq!(tree.focused_id(), None);
        assert!(!tree.is_animating());
        assert!(tree.layout(Size::new(120.0, 44.0)).regions.is_empty());
    }

    #[test]
    fn custom_height_keeps_a_real_pill_and_does_not_stretch_with_the_row() {
        let palette = ComponentPalette::neutral_dark();
        for label in ["1", "12", "Nombre Unicode 終わり"] {
            let mut tree = UiTree::new(Element::row(vec![Chip::new(label, palette)
                .height(28.0)
                .render()]));
            let frame = tree.layout(Size::new(600.0, 44.0));
            let (rect, radius) = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *color == palette.selected => Some((*rect, *radius)),
                    _ => None,
                })
                .unwrap();
            assert_eq!(rect.height, 28.0);
            assert_eq!(radius, 14.0);
            assert!(rect.width >= rect.height);
            if label == "1" {
                assert_eq!(rect.width, 28.0);
            }
            assert_eq!(frame.text_layouts()[0].source, label);
        }
    }

    const TONES: [AlertTone; 4] = [AlertTone::Info, AlertTone::Success, AlertTone::Warning, AlertTone::Error];

    fn light_palette() -> ComponentPalette {
        ComponentPalette {
            surface: Color::rgb(240, 240, 240),
            raised: Color::rgb(250, 250, 250),
            hover: Color::rgb(230, 230, 230),
            selected: Color::rgb(220, 220, 220),
            border: Color::rgb(205, 205, 205),
            text: Color::rgb(32, 32, 32),
            muted: Color::rgb(100, 100, 100),
            shadow: Color::rgba(0, 0, 0, 40),
        }
    }

    fn frame_of(chip: Chip) -> Frame {
        let mut tree = UiTree::new(Element::row(vec![chip.render()]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(600.0, 44.0))
    }

    /// The pill as painted: its fill, if it has one, then its edge with its rectangle, then the label's ink.
    fn paint(frame: &Frame, label: &str) -> (Option<Color>, Color, Rect, Color) {
        let fill = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { color, .. } => Some(*color),
            _ => None,
        });
        let (rect, edge) = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::StrokeRect { rect, color, .. } => Some((*rect, *color)),
                _ => None,
            })
            .unwrap();
        let ink = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == label => Some(*color),
                _ => None,
            })
            .unwrap();
        (fill, edge, rect, ink)
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false, alt: false }
    }

    #[test]
    fn a_plain_chip_paints_the_selected_fill_the_border_and_the_text_ink_and_nothing_else() {
        let palette = ComponentPalette::neutral_dark();
        let plain = frame_of(Chip::new("Plan Pro", palette));
        assert_eq!(plain.commands.len(), 3, "{:?}", plain.commands);
        let (fill, edge, rect, ink) = paint(&plain, "Plan Pro");
        assert_eq!((fill, edge, ink), (Some(palette.selected), palette.border, palette.text));
        assert_eq!(rect.height, 22.0);
        assert!(plain.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { font_family: FontFamily::Sans, font_size, .. } if *font_size == 11.0
        )));
        assert!(plain.regions.is_empty() && plain.access.is_empty());
        // The defaults of every new builder are the chip as it was.
        let spelled = Chip::new("Plan Pro", palette).tone(AlertTone::Neutral).size(ControlSize::Regular).enabled(true);
        assert_eq!(frame_of(spelled).commands, plain.commands);
        let cleared = Chip::new("Plan Pro", palette).color(Color::rgb(200, 40, 40)).tone(AlertTone::Neutral);
        assert_eq!(frame_of(cleared).commands, plain.commands);
    }

    #[test]
    fn a_toned_chip_is_a_tint_of_the_alert_accent_under_readable_ink_in_both_themes() {
        for palette in [ComponentPalette::neutral_dark(), light_palette()] {
            for tone in TONES {
                let accent = AlertColors::for_tone(tone, palette).accent;
                let (fill, edge, _, ink) = paint(&frame_of(Chip::new("State", palette).tone(tone)), "State");
                let fill = fill.unwrap();
                assert_eq!(fill, blend(palette.surface, accent, TINT_FILL), "{tone:?}");
                assert_eq!(edge, blend(palette.surface, accent, TINT_EDGE), "{tone:?}");
                assert_ne!(fill, palette.selected, "{tone:?}");
                assert!(contrast(ink, fill) >= 4.5, "{tone:?}: {}", contrast(ink, fill));
                // The tint stays near the surface, so the chip does not shout.
                assert!(contrast(fill, palette.surface) < 1.6, "{tone:?}");
                let (fill, edge, _, ink) = paint(&frame_of(Chip::new("State", palette).tone(tone).outline()), "State");
                assert_eq!((fill, edge), (None, accent), "{tone:?}");
                assert!(contrast(ink, palette.surface) >= 4.5, "{tone:?}");
                assert!(contrast(edge, palette.surface) >= 3.0, "{tone:?}");
            }
        }
    }

    #[test]
    fn a_host_color_is_drawn_as_a_tone_and_corrected_until_it_reads() {
        for palette in [ComponentPalette::neutral_dark(), light_palette()] {
            // A purple, then colors that cannot be read on one theme or the other, then a veil.
            for color in [Color::rgb(138, 92, 246), Color::rgb(10, 10, 10), Color::WHITE, Color::rgba(255, 255, 0, 40)] {
                let (fill, _, _, ink) = paint(&frame_of(Chip::new("Tag", palette).color(color)), "Tag");
                assert!(contrast(ink, fill.unwrap()) >= 4.5, "{color:?}");
                let (fill, edge, _, ink) = paint(&frame_of(Chip::new("Tag", palette).color(color).outline()), "Tag");
                assert_eq!(fill, None);
                assert!(contrast(edge, palette.surface) >= 3.0, "{color:?}");
                assert!(contrast(ink, palette.surface) >= 4.5, "{color:?}");
            }
        }
        // A color that already reads is the edge of the outline chip as given, and the later of tone and color wins.
        let palette = ComponentPalette::neutral_dark();
        let purple = Color::rgb(160, 120, 250);
        assert_eq!(paint(&frame_of(Chip::new("Tag", palette).tone(AlertTone::Error).color(purple).outline()), "Tag").1, purple);
        let error = AlertColors::for_tone(AlertTone::Error, palette).accent;
        assert_eq!(paint(&frame_of(Chip::new("Tag", palette).color(purple).tone(AlertTone::Error).outline()), "Tag").1, error);
    }

    #[test]
    fn a_size_sets_the_height_and_the_text_together_and_a_later_height_only_the_height() {
        let palette = ComponentPalette::neutral_dark();
        let measured = |chip: Chip| {
            let frame = frame_of(chip);
            let (_, _, rect, _) = paint(&frame, "Size");
            let font_size = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { font_size, .. } => Some(*font_size),
                _ => None,
            });
            let radius = frame.commands.iter().find_map(|command| match command {
                DrawCommand::StrokeRect { radius, .. } => Some(*radius),
                _ => None,
            });
            (rect.height, font_size.unwrap(), radius.unwrap())
        };
        for (size, height, font_size) in [
            (ControlSize::Dense, 18.0, 10.0),
            (ControlSize::Compact, 20.0, 11.0),
            (ControlSize::Regular, 22.0, 11.0),
            (ControlSize::Spacious, 26.0, 12.0),
        ] {
            assert_eq!(measured(Chip::new("Size", palette).size(size)), (height, font_size, height / 2.0));
        }
        assert_eq!(measured(Chip::new("Size", palette).size(ControlSize::Spacious).height(30.0)), (30.0, 12.0, 15.0));
        assert_eq!(measured(Chip::new("Size", palette).height(30.0).size(ControlSize::Dense)), (18.0, 10.0, 9.0));
        // A height that is no number leaves the size's own; one too low is raised to 16.
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(measured(Chip::new("Size", palette).size(ControlSize::Dense).height(bad)).0, 18.0);
        }
        assert_eq!(measured(Chip::new("Size", palette).height(-4.0)).0, 16.0);
        // One digit is a circle at every size.
        let frame = frame_of(Chip::new("3", palette).size(ControlSize::Spacious));
        let (_, _, rect, _) = paint(&frame, "3");
        assert_eq!((rect.width, rect.height), (26.0, 26.0));
    }

    #[test]
    fn an_icon_or_a_dot_stands_before_the_label_in_its_ink_and_centered_on_the_pill() {
        for palette in [ComponentPalette::neutral_dark(), light_palette()] {
            let plain = frame_of(Chip::new("Online", palette).tone(AlertTone::Success));
            let (_, _, bare, ink) = paint(&plain, "Online");
            let text_of = |frame: &Frame| frame.text_layouts().iter().find(|block| block.source == "Online").unwrap().runs[0].rect;
            let check = |frame: &Frame, mark: Rect, side: f32| {
                let (_, _, pill, _) = paint(frame, "Online");
                let text = text_of(frame);
                assert_eq!((mark.width, mark.height), (side, side));
                assert!((mark.x - pill.x - INSET).abs() < 0.01);
                assert!((text.x - mark.x - side - LEADING_GAP).abs() < 0.01);
                assert!((mark.y + side / 2.0 - pill.y - pill.height / 2.0).abs() < 0.01);
                // The pill grows by the mark and its gap, and keeps its inset after the label.
                assert!((pill.width - bare.width - side - LEADING_GAP).abs() < 0.01);
                assert!((pill.x + pill.width - text.x - text.width - INSET).abs() < 0.01);
                assert!(frame.regions.is_empty() && frame.access.is_empty());
            };
            let dotted = frame_of(Chip::new("Online", palette).tone(AlertTone::Success).dot());
            let dot = dotted
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, radius } if *color == ink && *radius == DOT_SIZE / 2.0 => Some(*rect),
                    _ => None,
                })
                .expect("a dot in the ink");
            check(&dotted, dot, DOT_SIZE);
            let with_icon = frame_of(Chip::new("Online", palette).tone(AlertTone::Success).dot().icon(Icon::Check));
            let glyph = with_icon
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, icon: Icon::Check, tint } if *tint == ink => Some(*rect),
                    _ => None,
                })
                .expect("an icon in the ink");
            check(&with_icon, glyph, 13.0);
            // The later of the two is the one drawn.
            assert_eq!(with_icon.commands.iter().filter(|command| matches!(command, DrawCommand::FillRect { .. })).count(), 1);
        }
        // A digit with a mark before it is a pill, not a circle.
        let palette = ComponentPalette::neutral_dark();
        let (_, _, pill, _) = paint(&frame_of(Chip::new("3", palette).dot()), "3");
        assert!(pill.width > pill.height);
    }

    #[test]
    fn the_remove_button_reports_its_id_by_pointer_enter_and_space_and_is_reached_by_tab() {
        let palette = ComponentPalette::neutral_dark();
        let chip = || Chip::new("rust", palette).remove_action("remove-rust");
        let mut tree = UiTree::new(Element::row(vec![chip().render(), Element::button("after", "After")]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(600.0, 44.0));
        let (_, _, pill, _) = paint(&frame, "rust");
        let ids: Vec<_> = frame.regions.iter().map(|region| region.id.as_str()).collect();
        assert_eq!(ids, ["remove-rust", "after"]);
        // A circle at the end of the pill, concentric with its cap, after the label.
        let button = frame.regions[0].rect;
        assert_eq!((button.width, button.height), (18.0, 18.0));
        assert!((pill.x + pill.width - button.x - button.width - REMOVE_INSET).abs() < 0.01);
        assert!((button.y - pill.y - REMOVE_INSET).abs() < 0.01);
        let text = frame.text_layouts().iter().find(|block| block.source == "rust").unwrap().runs[0].rect;
        assert!((text.x - pill.x - INSET).abs() < 0.01);
        assert!(button.x >= text.x + text.width + REMOVE_INSET - 0.01);
        let glyph = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon { rect, icon: Icon::CloseBold, tint } if *tint == palette.text => Some(*rect),
                _ => None,
            })
            .expect("a close glyph in the ink");
        assert!((glyph.x + glyph.width / 2.0 - button.x - 9.0).abs() < 0.01);
        assert!((glyph.y + glyph.height / 2.0 - button.y - 9.0).abs() < 0.01);
        let said = frame.access.iter().find(|node| node.id == "remove-rust").unwrap();
        assert_eq!((said.role, said.name.clone()), (Role::Button, format!("{} rust", crate::labels().remove)));
        assert!(said.actionable && !said.disabled);
        let clicked = vec![UiAction::Clicked { id: "remove-rust".into() }];
        let point = Point::new(button.x + 9.0, button.y + 9.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked);
        // The label is no part of the target.
        let on_label = Point::new(text.x + 2.0, text.y + text.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(on_label));
        assert!(tree.handle_event(UiEvent::PointerUp(on_label)).is_empty());
        let mut tree = UiTree::new(Element::row(vec![chip().render(), Element::button("after", "After")]));
        tree.layout(Size::new(600.0, 44.0));
        tree.handle_event(key(Key::Tab));
        assert_eq!(tree.focused_id(), Some("remove-rust"));
        assert_eq!(tree.handle_event(key(Key::Enter)), clicked);
        assert_eq!(tree.handle_event(key(Key::Space)), clicked);
        tree.handle_event(key(Key::Tab));
        assert_eq!(tree.focused_id(), Some("after"));
        // A chip without a label is named by the word alone.
        let bare = frame_of(Chip::new("", palette).remove_action("remove"));
        assert_eq!(bare.access[0].name, crate::labels().remove);
    }

    #[test]
    fn a_disabled_chip_is_muted_and_its_remove_button_is_passive_in_the_same_place() {
        let palette = ComponentPalette::neutral_dark();
        for tone in [AlertTone::Neutral, AlertTone::Warning] {
            let chip = || Chip::new("rust", palette).tone(tone).dot().remove_action("remove-rust");
            let enabled = frame_of(chip());
            let mut tree = UiTree::new(Element::row(vec![chip().disabled().render()]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(600.0, 44.0));
            assert!(frame.regions.is_empty());
            let (fill, edge, pill, ink) = paint(&frame, "rust");
            let (enabled_fill, enabled_edge, enabled_pill, _) = paint(&enabled, "rust");
            assert_eq!((fill, edge, pill), (enabled_fill, enabled_edge, enabled_pill));
            assert_eq!(ink, palette.muted);
            let glyph = |frame: &Frame| {
                frame.commands.iter().find_map(|command| match command {
                    DrawCommand::Icon { rect, icon: Icon::CloseBold, tint } => Some((*rect, *tint)),
                    _ => None,
                })
            };
            assert_eq!(glyph(&frame).unwrap(), (glyph(&enabled).unwrap().0, palette.muted));
            let [said] = frame.access.as_slice() else {
                panic!("one node, got {:?}", frame.access);
            };
            assert_eq!((said.id.as_str(), said.role, said.disabled, said.actionable), ("remove-rust", Role::Button, true, false));
            assert_eq!(said.name, format!("{} rust", crate::labels().remove));
            assert_eq!(said.rect, enabled.regions[0].rect);
            assert!(!tree.focus("remove-rust"));
            let point = Point::new(said.rect.x + 9.0, said.rect.y + 9.0);
            for event in [UiEvent::PointerMove(point), UiEvent::PointerDown(point), UiEvent::PointerUp(point), key(Key::Tab), key(Key::Enter), key(Key::Space)] {
                assert!(tree.handle_event(event).is_empty());
            }
            assert_eq!(tree.focused_id(), None);
            // `enabled(false)` is the same chip, and a disabled chip without a button is only muted.
            assert_eq!(frame_of(chip().enabled(false)).commands, frame.commands);
        }
        let passive = frame_of(Chip::new("rust", palette).disabled());
        assert_eq!(paint(&passive, "rust").3, palette.muted);
        assert!(passive.regions.is_empty() && passive.access.is_empty());
    }
}
