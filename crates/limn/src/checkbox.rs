//! A box that is checked or not, with a label beside it that is part of the
//! click target.

use crate::{Access, Color, Component, ComponentPalette, ControlSize, Element, FontFamily, Icon, Role, TextSpan};

/// Space between the box and its label.
const GAP: f32 = 8.0;
const BORDER: f32 = 1.0;

/// The side of the box, the label's text size and its line height, for a
/// control size. The line is never shorter than the box, so the box centers
/// on the first line.
const fn metrics(size: ControlSize) -> (f32, f32, f32) {
    match size {
        ControlSize::Dense => (14.0, 12.0, 16.0),
        ControlSize::Compact => (16.0, 13.0, 18.0),
        ControlSize::Regular => (16.0, 14.0, 20.0),
        ControlSize::Spacious => (18.0, 16.0, 24.0),
    }
}

/// The host owns the value: the checkbox shows `checked` and reports
/// `UiAction::Clicked` with its ID from the box or the label, and the host
/// flips its own value. Its height follows the label, which wraps in a narrow
/// width while the box stays on its first line.
pub struct Checkbox {
    id: String,
    checked: bool,
    indeterminate: bool,
    label: String,
    size: ControlSize,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl Checkbox {
    /// A box alone, `ControlSize::Regular`, that shows `checked` and reports
    /// `Clicked` with this ID.
    pub fn new(id: impl Into<String>, checked: bool, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            checked,
            indeterminate: false,
            label: String::new(),
            size: ControlSize::Regular,
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// Text beside the box, which is clicked with it and names it to the
    /// platform. It wraps when the width runs out. An empty one is no label.
    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = text.into();
        self
    }

    /// Draw a dash instead of the check or the empty box, whatever `checked`
    /// is: the parent of a list of which only some are chosen. The platform is
    /// told it is not on.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// The side of the box and the label's text: 14 and 12 pixels for `Dense`,
    /// 16 and 13 for `Compact`, 16 and 14 for `Regular`, 18 and 16 for
    /// `Spacious`. The height is the label's, not the control height.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A disabled checkbox is passive: it shows its value in the muted color
    /// and has no hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// What the checkbox is called to the platform, for a box without a
    /// label. Given with a label, it is said instead of it.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }
}

impl Component for Checkbox {
    fn render(self) -> Element {
        let palette = self.palette;
        let (side, font_size, line) = metrics(self.size);
        let filled = self.checked || self.indeterminate;
        let ink = if self.enabled { palette.text } else { palette.muted };
        // A filled box has no border: a stroke of the fill's own color shows as a ring inside its edge.
        let (fill, border) = if filled { (ink, 0.0) } else { (Color::TRANSPARENT, BORDER) };
        // The mark stays in the tree when the box is empty, so its ink fades
        // with the fill instead of appearing at once.
        let mark = Element::icon(if self.indeterminate { Icon::Minus } else { Icon::Check })
            .foreground(if filled { palette.surface } else { Color::TRANSPARENT })
            .animate_foreground(format!("{}:mark", self.id))
            .size(Some(side), Some(side));
        // The surface ID keeps the fill's transition across rebuilds, and
        // lights the empty box while the pointer is anywhere on the target.
        let square = Element::panel(mark)
            .background(fill)
            .hover_background(if filled { fill } else { palette.hover })
            .hover_surface(format!("{}:box", self.id), self.enabled.then_some(self.id.as_str()))
            .transition_ms(crate::tokens().motion.hover_ms)
            .border(palette.border, border)
            .radius(crate::tokens().radius.control)
            .size(Some(side), Some(side));
        // Centered on the first line, not on the whole of a wrapped label.
        // The same room under it, so a box alone is as high as a labeled one.
        let inset = || Element::spacer().size(Some(side), Some((line - side) / 2.0));
        let square = Element::column(vec![inset(), square, inset()]);
        // A passive element is not named by its text, so the label is said outright.
        let name = self.accessibility_label.or_else(|| (!self.label.is_empty()).then(|| self.label.clone()));
        let mut access = Access::new(Role::CheckBox)
            .id(self.id.clone())
            .toggled(self.checked && !self.indeterminate)
            .disabled(!self.enabled);
        if let Some(name) = name {
            access = access.name(name);
        }
        let content = if self.label.is_empty() {
            square
        } else {
            // Formatted text takes a line height, which plain text leaves to
            // the host's font: the box is centered on a line of known height.
            let label = Element::formatted_text(vec![TextSpan::new(self.label, ink, font_size, FontFamily::Sans)])
                .font_size(font_size)
                .line_height(line)
                .shrink_to_fit();
            Element::row(vec![square, label]).gap(GAP)
        };
        if self.enabled {
            Element::clickable(self.id, content).radius(crate::tokens().radius.control)
        } else {
            Element::panel(content)
        }
        .accessible(access)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};
    use std::time::{Duration, Instant};

    const LABEL: &str = "Send me the weekly summary";
    const LONG: &str = "Send me the weekly summary of everything that changed in every project I follow";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(600.0, 300.0)
    }

    /// The checkbox at the top left of a column `width` wide, settled.
    fn tree_in(checkbox: Checkbox, width: f32) -> UiTree {
        let mut tree = UiTree::new(root(checkbox, width));
        tree.set_reduced_motion(true);
        tree
    }

    fn root(checkbox: Checkbox, width: f32) -> Element {
        Element::row(vec![
            Element::column(vec![row_of(checkbox), Element::spacer().flex(1.0)]).size(Some(width), None),
            Element::spacer().flex(1.0),
        ])
    }

    /// A row keeps the checkbox at its own width while there is room.
    fn row_of(checkbox: Checkbox) -> Element {
        Element::row(vec![checkbox.render().shrink_to_fit(), Element::spacer().flex(1.0)])
    }

    fn tree_of(checkbox: Checkbox) -> UiTree {
        tree_in(checkbox, 400.0)
    }

    /// The box as painted: its rectangle, its fill and its border's color. An
    /// empty box has a fill that shows nothing, and a filled one no border.
    fn square(frame: &Frame) -> (Rect, Color, Option<Color>) {
        let radius = crate::tokens().radius.control;
        let fill = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, radius: rounded } if *rounded == radius => Some((*rect, *color)),
            _ => None,
        });
        let stroke = frame.commands.iter().find_map(|command| match command {
            DrawCommand::StrokeRect { rect, color, width, radius: rounded } => {
                assert_eq!((*width, *rounded), (BORDER, radius));
                Some((*rect, *color))
            }
            _ => None,
        });
        let rect = stroke.or(fill).expect("a fill or a border").0;
        (rect, fill.map_or(Color::TRANSPARENT, |fill| fill.1), stroke.map(|stroke| stroke.1))
    }

    /// The mark that shows: none when its ink is transparent.
    fn mark(frame: &Frame) -> Option<(Icon, Color, Rect)> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { icon, tint, rect } if tint.a > 0 => Some((*icon, *tint, *rect)),
            _ => None,
        })
    }

    fn lines(frame: &Frame) -> Vec<(&str, Point, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, color, .. } => Some((text.as_str(), *position, *color)),
                _ => None,
            })
            .collect()
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false }
    }

    #[test]
    fn checked_is_the_text_color_with_a_check_in_the_surface_color_and_unchecked_a_bordered_box() {
        let frame = tree_of(Checkbox::new("box", true, palette())).layout(window());
        let (rect, fill, edge) = square(&frame);
        assert_eq!(rect, Rect::new(0.0, 2.0, 16.0, 16.0));
        assert_eq!((fill, edge), (palette().text, None));
        assert_eq!(mark(&frame), Some((Icon::Check, palette().surface, rect)));

        let frame = tree_of(Checkbox::new("box", false, palette())).layout(window());
        let (empty, fill, edge) = square(&frame);
        assert_eq!((empty, fill.a, edge), (rect, 0, Some(palette().border)));
        assert_eq!(mark(&frame), None);
    }

    #[test]
    fn an_indeterminate_box_is_filled_with_a_dash_whatever_its_value() {
        for checked in [false, true] {
            let frame = tree_of(Checkbox::new("box", checked, palette()).indeterminate(true)).layout(window());
            let (rect, fill, _) = square(&frame);
            assert_eq!(fill, palette().text);
            assert_eq!(mark(&frame), Some((Icon::Minus, palette().surface, rect)));
        }
    }

    #[test]
    fn the_fill_and_the_mark_fade_in_when_the_box_is_checked() {
        let mut tree = UiTree::new(root(Checkbox::new("box", false, palette()), 400.0));
        let start = Instant::now();
        tree.layout_at(window(), start);
        tree.set_root(root(Checkbox::new("box", true, palette()), 400.0));
        tree.layout_at(window(), start);
        let midway = tree.layout_at(window(), start + Duration::from_millis(u64::from(crate::tokens().motion.hover_ms) / 2));
        let fill = square(&midway).1;
        assert!(fill != palette().text && fill.a > 0, "between empty and filled, got {fill:?}");
        let tint = mark(&midway).expect("a mark on its way in").1;
        assert!(tint != palette().surface, "the mark has not arrived, got {tint:?}");
        let settled = tree.layout_at(window(), start + Duration::from_secs(2));
        assert_eq!(square(&settled).1, palette().text);
        assert_eq!(mark(&settled).map(|mark| mark.1), Some(palette().surface));
    }

    #[test]
    fn the_box_and_its_label_are_one_target_that_reports_a_click_by_pointer_enter_and_space() {
        let mut tree = tree_of(Checkbox::new("news", false, palette()).label(LABEL));
        let frame = tree.layout(window());
        let [region] = frame.regions.as_slice() else {
            panic!("one region, got {:?}", frame.regions);
        };
        assert_eq!((region.id.as_str(), region.kind), ("news", HitKind::Button));
        let (rect, ..) = square(&frame);
        let label = lines(&frame)[0];
        assert_eq!((label.0, label.2), (LABEL, palette().text));
        assert_eq!(label.1.x, rect.width + GAP);
        // The region ends with the label, not with the column.
        assert!(region.rect.width > label.1.x && region.rect.width < 400.0);
        assert_eq!(region.rect.height, 20.0);
        let clicked = vec![UiAction::Clicked { id: "news".into() }];
        for point in [Point::new(8.0, 10.0), Point::new(region.rect.width - 4.0, 10.0)] {
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked);
        }
        assert!(tree.focus("news"));
        assert_eq!(tree.handle_event(key(Key::Enter)), clicked);
        assert_eq!(tree.handle_event(key(Key::Space)), clicked);
    }

    #[test]
    fn the_empty_box_lights_while_the_pointer_is_on_the_label() {
        let mut tree = tree_of(Checkbox::new("news", false, palette()).label(LABEL));
        let width = tree.layout(window()).regions[0].rect.width;
        tree.handle_event(UiEvent::PointerMove(Point::new(width - 4.0, 10.0)));
        assert_eq!(square(&tree.layout(window())).1, palette().hover);
    }

    #[test]
    fn a_disabled_checkbox_takes_no_pointer_or_focus_and_keeps_its_place() {
        for checked in [false, true] {
            let checkbox = |enabled| Checkbox::new("news", checked, palette()).label(LABEL).enabled(enabled);
            let mut tree = tree_of(checkbox(false));
            let frame = tree.layout(window());
            assert!(frame.regions.is_empty());
            assert!(!tree.focus("news"));
            let point = Point::new(8.0, 10.0);
            tree.handle_event(UiEvent::PointerMove(point));
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            assert!(tree.handle_event(key(Key::Space)).is_empty());
            // Muted, and not lit by the pointer over it.
            let frame = tree.layout(window());
            let (rect, fill, edge) = square(&frame);
            if checked {
                assert_eq!((fill, edge), (palette().muted, None));
            } else {
                assert_eq!((fill.a, edge), (0, Some(palette().border)));
            }
            assert_eq!(lines(&frame)[0].2, palette().muted);
            // The same rectangles as enabled, so a form does not move.
            let enabled = tree_of(checkbox(true)).layout(window());
            assert_eq!(rect, square(&enabled).0);
            assert_eq!(lines(&frame)[0].1, lines(&enabled)[0].1);
            let said = |frame: &Frame| frame.access.iter().find(|node| node.id == "news").unwrap().rect;
            assert_eq!(said(&frame), said(&enabled));
            assert_eq!(said(&enabled), enabled.regions[0].rect);
        }
    }

    #[test]
    fn a_long_label_wraps_inside_a_narrow_width_and_the_box_stays_on_its_first_line() {
        for width in [120.0, 200.0, 320.0] {
            let mut tree = tree_in(Checkbox::new("news", true, palette()).label(LONG), width);
            let frame = tree.layout(window());
            let (rect, ..) = square(&frame);
            let lines = lines(&frame);
            assert!(lines.len() > 1, "wrapped at {width}");
            // Every line starts beside the box, a line apart, and none is cut.
            for (index, line) in lines.iter().enumerate() {
                assert_eq!((line.1.x, line.1.y - lines[0].1.y), (rect.width + GAP, index as f32 * 20.0));
            }
            assert_eq!(lines.iter().map(|line| line.0).collect::<String>().trim_end(), LONG);
            // Centered on the first line, whatever the label's height.
            assert_eq!(rect, Rect::new(0.0, 2.0, 16.0, 16.0));
            let region = frame.regions[0].rect;
            assert!(region.width <= width, "{} wide in {width}", region.width);
            assert_eq!(region.height, lines.len() as f32 * 20.0);
            // A click on the last line is a click on the checkbox.
            let point = Point::new(rect.width + GAP + 4.0, region.height - 4.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: "news".into() }]);
        }
    }

    #[test]
    fn given_a_width_or_set_in_a_column_it_wraps_within_it() {
        let checkbox = || Checkbox::new("news", false, palette()).label(LONG).render();
        let sized = Element::row(vec![checkbox().size(Some(200.0), None), Element::spacer().flex(1.0)]);
        let in_column = Element::row(vec![
            Element::column(vec![checkbox()]).size(Some(200.0), None),
            Element::spacer().flex(1.0),
        ]);
        for root in [sized, in_column] {
            let mut tree = UiTree::new(root);
            tree.set_reduced_motion(true);
            let frame = tree.layout(window());
            assert!(lines(&frame).len() > 1);
            assert_eq!(frame.regions[0].rect.width, 200.0);
            assert!(lines(&frame).iter().all(|line| line.1.x == 16.0 + GAP));
        }
        // A column gives its width to the target, also beside a short label.
        let mut tree = UiTree::new(Element::row(vec![
            Element::column(vec![Checkbox::new("news", false, palette()).label("Sync").render()]).size(Some(200.0), None),
            Element::spacer().flex(1.0),
        ]));
        assert_eq!(tree.layout(window()).regions[0].rect, Rect::new(0.0, 0.0, 200.0, 20.0));
    }

    #[test]
    fn each_size_has_its_box_and_its_text_and_centers_the_box_on_the_line() {
        for (size, side, font, line) in [
            (ControlSize::Dense, 14.0, 12.0, 16.0),
            (ControlSize::Compact, 16.0, 13.0, 18.0),
            (ControlSize::Regular, 16.0, 14.0, 20.0),
            (ControlSize::Spacious, 18.0, 16.0, 24.0),
        ] {
            let frame = tree_of(Checkbox::new("news", true, palette()).label(LABEL).size(size)).layout(window());
            let (rect, ..) = square(&frame);
            assert_eq!(rect, Rect::new(0.0, (line - side) / 2.0, side, side));
            assert_eq!(frame.regions[0].rect.height, line);
            assert!(frame.commands.iter().any(
                |command| matches!(command, DrawCommand::Text { font_size, .. } if *font_size == font)
            ));
            // Without a label the target is the box's width and as high as a labeled one.
            let alone = tree_of(Checkbox::new("news", true, palette()).size(size)).layout(window());
            assert_eq!(square(&alone).0, rect);
            assert_eq!(alone.regions[0].rect, Rect::new(0.0, 0.0, side, line));
        }
    }

    #[test]
    fn it_is_a_check_box_named_by_its_label_that_says_whether_it_is_on_enabled_and_disabled() {
        for enabled in [true, false] {
            for (checked, indeterminate, on) in [(false, false, false), (true, false, true), (true, true, false), (false, true, false)] {
                let checkbox = Checkbox::new("news", checked, palette())
                    .label(LABEL)
                    .indeterminate(indeterminate)
                    .enabled(enabled);
                let frame = tree_of(checkbox).layout(window());
                let said = frame.access.iter().find(|node| node.id == "news").unwrap();
                assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::CheckBox, LABEL, Some(on)));
                assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            }
        }
    }

    #[test]
    fn a_box_without_a_label_is_named_by_its_accessibility_label_which_also_replaces_a_label() {
        for enabled in [true, false] {
            for label in ["", LABEL] {
                let checkbox = Checkbox::new("row-3", false, palette())
                    .label(label)
                    .accessibility_label("Select row 3")
                    .enabled(enabled);
                let frame = tree_of(checkbox).layout(window());
                let said = frame.access.iter().find(|node| node.id == "row-3").unwrap();
                assert_eq!((said.role, said.name.as_str()), (Role::CheckBox, "Select row 3"));
            }
        }
    }
}
