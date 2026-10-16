//! A passive mark over the corner of another element: a count, or a dot for something new.

use crate::{Access, Color, Component, ComponentPalette, Element, FontFamily};

/// Height of a mark that shows a count, its outline included.
const COUNT_HEIGHT: f32 = 16.0;
/// Diameter of a dot, its outline included.
const DOT_SIZE: f32 = 10.0;
/// Space between the ends of the pill and a count of more than one character.
const COUNT_INSET: f32 = 4.0;
/// The outline that separates the mark from what is under it.
const OUTLINE: f32 = 1.0;
/// The size of a chip's label.
const FONT_SIZE: f32 = 11.0;
const DEFAULT_MAX: u32 = 99;

/// The host owns the element that is marked and the count. The badge reports
/// nothing and takes no pointer or focus: the element under it keeps its hit
/// region, its focus and its actions, and its layout size.
pub struct Badge {
    child: Element,
    palette: ComponentPalette,
    count: u32,
    max: u32,
    show_zero: bool,
    dot: bool,
    color: Option<Color>,
    description: Option<String>,
}

impl Badge {
    /// Mark `child`. Nothing is drawn over it until `count` or `dot` says what.
    pub fn new(child: Element, palette: ComponentPalette) -> Self {
        Self {
            child,
            palette,
            count: 0,
            max: DEFAULT_MAX,
            show_zero: false,
            dot: false,
            color: None,
            description: None,
        }
    }

    /// Show this number. Zero shows no mark unless `show_zero` is set.
    pub fn count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// The largest count written in full, 99 unless set and at least 1. A
    /// larger count shows this number followed by `+`.
    pub fn max(mut self, max: u32) -> Self {
        self.max = max.max(1);
        self
    }

    /// Whether a count of zero shows a mark that reads `0`.
    pub fn show_zero(mut self, show: bool) -> Self {
        self.show_zero = show;
        self
    }

    /// Show a dot with no number, whatever the count is.
    pub fn dot(mut self) -> Self {
        self.dot = true;
        self
    }

    /// The fill of the mark, in place of the palette's text color: an alert
    /// color for something urgent. The count stays in the surface color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// What the mark means, in the host's words, such as "3 new". It becomes
    /// the description of the control inside the badge while the mark shows.
    /// Without it the platform is told nothing of the mark.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// What the mark reads: `None` when there is no mark, an empty string for a dot.
    fn label(&self) -> Option<String> {
        if self.dot {
            Some(String::new())
        } else if self.count > self.max {
            Some(format!("{}+", self.max))
        } else if self.count > 0 || self.show_zero {
            Some(self.count.to_string())
        } else {
            None
        }
    }
}

impl Component for Badge {
    fn render(self) -> Element {
        let Some(label) = self.label() else {
            return self.child;
        };
        let palette = self.palette;
        let height = if self.dot { DOT_SIZE } else { COUNT_HEIGHT };
        // One character stands in a circle, as in a chip; more widen the pill.
        let round = label.chars().count() <= 1;
        let content = if self.dot {
            Element::spacer()
        } else {
            let count = Element::text(label)
                .font_family(FontFamily::Sans)
                .font_size(FONT_SIZE)
                .foreground(palette.surface)
                .center_content()
                .centered_y();
            if round {
                count
            } else {
                // The count keeps its measured width, as a chip's label does.
                Element::row(vec![
                    Element::spacer().min_size(COUNT_INSET, 0.0).flex(1.0),
                    count,
                    Element::spacer().min_size(COUNT_INSET, 0.0).flex(1.0),
                ])
            }
        };
        let mark = Element::panel(content)
            .background(self.color.unwrap_or(palette.text))
            .border(palette.surface, OUTLINE)
            .radius(height / 2.0)
            .min_size(height, height)
            .size(round.then_some(height), Some(height));
        // The mark is held in the top right corner of the child's box. Its
        // holder measures as nothing, so the stack is as large as the child alone.
        let corner = Element::column(vec![
            Element::row(vec![Element::spacer().flex(1.0), mark]),
            Element::spacer().flex(1.0),
        ])
        .size(Some(0.0), Some(0.0));
        // The stack stands in its parent as the child would alone: the same
        // fixed size and flex, so a taller row does not stretch what is marked.
        let layout = self.child.style;
        let mut marked = Element::stack(vec![self.child, corner])
            .min_size(layout.min_width, layout.min_height)
            .flex(layout.flex);
        marked.style.width = layout.width;
        marked.style.height = layout.height;
        marked.style.max_width = layout.max_width;
        marked.style.max_height = layout.max_height;
        match self.description {
            // Said on the wrapper, which is no control: the tree hands it to the control inside.
            Some(text) => marked.accessible(Access::default().description(text)),
            None => marked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ButtonVariant, DrawCommand, Frame, Icon, IconButton, Key, Point, Rect, Role, Size, UiAction, UiEvent, UiTree};

    const FILL: Color = Color::rgb(200, 40, 40);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(400.0, 200.0)
    }

    fn bell(id: &str) -> Element {
        IconButton::new(id, Icon::Bell, palette()).variant(ButtonVariant::Ghost).accessibility_label("Notifications").render()
    }

    /// A row of three buttons whose middle one is given to `mark`.
    fn tree_of(mark: impl Fn(Element) -> Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![bell("before"), mark(bell("bell")), bell("after"), Element::spacer().flex(1.0)]).gap(8.0),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap().rect
    }

    /// The mark's fill: its rectangle and its radius.
    fn mark(frame: &Frame, color: Color) -> Option<(Rect, f32)> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color: fill, radius } if *fill == color => Some((*rect, *radius)),
            _ => None,
        })
    }

    fn texts(frame: &Frame) -> Vec<String> {
        frame.text_layouts().iter().map(|block| block.source.clone()).collect()
    }

    #[test]
    fn a_count_is_a_pill_in_the_top_right_corner_of_the_child() {
        for (count, label) in [(3, "3"), (42, "42"), (99, "99"), (100, "99+")] {
            let frame = tree_of(|child| Badge::new(child, palette()).count(count).color(FILL).render()).layout(window());
            let child = region(&frame, "bell");
            let (rect, radius) = mark(&frame, FILL).expect("a mark");
            assert_eq!((rect.height, radius), (COUNT_HEIGHT, COUNT_HEIGHT / 2.0), "{label}");
            assert_eq!((rect.x + rect.width, rect.y), (child.x + child.width, child.y), "{label}");
            // One digit is a circle; more are wider than high and stay inside the child.
            assert_eq!(rect.width == COUNT_HEIGHT, count < 10, "{label}");
            assert!(rect.width >= COUNT_HEIGHT && rect.x >= child.x, "{label}");
            let blocks = frame.text_layouts();
            let text = &blocks.iter().find(|block| block.source == label).expect(label).runs[0];
            assert!((text.rect.x + text.rect.width / 2.0 - rect.x - rect.width / 2.0).abs() < 0.1, "{label}");
            assert!((text.rect.y + text.rect.height / 2.0 - rect.y - rect.height / 2.0).abs() < 0.1, "{label}");
        }
    }

    #[test]
    fn the_mark_is_the_text_color_with_surface_ink_and_a_surface_outline_unless_colored() {
        for color in [None, Some(FILL)] {
            let fill = color.unwrap_or(palette().text);
            let frame = tree_of(|child| {
                let badge = Badge::new(child, palette()).count(12);
                match color {
                    Some(color) => badge.color(color),
                    None => badge,
                }
                .render()
            })
            .layout(window());
            let (rect, _) = mark(&frame, fill).expect("a mark");
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, color, font_size, .. }
                    if text == "12" && *color == palette().surface && *font_size == FONT_SIZE
            )));
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::StrokeRect { rect: stroke, color, radius, .. }
                    if *color == palette().surface && *radius == COUNT_HEIGHT / 2.0 && stroke.height == rect.height
            )));
        }
    }

    #[test]
    fn a_dot_is_a_small_circle_with_no_number_whatever_the_count() {
        for count in [0, 7] {
            let frame = tree_of(|child| Badge::new(child, palette()).count(count).dot().color(FILL).render()).layout(window());
            let child = region(&frame, "bell");
            let (rect, radius) = mark(&frame, FILL).expect("a dot");
            assert_eq!((rect.width, rect.height, radius), (DOT_SIZE, DOT_SIZE, DOT_SIZE / 2.0));
            assert_eq!((rect.x + rect.width, rect.y), (child.x + child.width, child.y));
            assert!(texts(&frame).is_empty());
        }
    }

    #[test]
    fn zero_shows_nothing_unless_asked_and_a_count_over_the_maximum_shows_it_with_a_plus() {
        let label = |badge: fn(Badge) -> Badge| {
            let frame = tree_of(|child| badge(Badge::new(child, palette()).color(FILL)).render()).layout(window());
            assert_eq!(mark(&frame, FILL).is_some(), !texts(&frame).is_empty());
            texts(&frame).first().cloned()
        };
        assert_eq!(label(|badge| badge), None);
        assert_eq!(label(|badge| badge.count(0)), None);
        assert_eq!(label(|badge| badge.count(0).show_zero(true)).as_deref(), Some("0"));
        assert_eq!(label(|badge| badge.count(99)).as_deref(), Some("99"));
        assert_eq!(label(|badge| badge.count(100)).as_deref(), Some("99+"));
        assert_eq!(label(|badge| badge.count(1000).max(999)).as_deref(), Some("999+"));
        assert_eq!(label(|badge| badge.count(9).max(9)).as_deref(), Some("9"));
        // A maximum of zero would read "0+" for every count: it is raised to one.
        assert_eq!(label(|badge| badge.count(5).max(0)).as_deref(), Some("1+"));
        assert_eq!(label(|badge| badge.count(u32::MAX).max(u32::MAX)).as_deref(), Some("4294967295"));
    }

    #[test]
    fn a_row_of_buttons_does_not_move_when_one_gains_a_badge() {
        let plain = tree_of(|child| child).layout(window());
        let marks: [fn(Badge) -> Badge; 4] =
            [|badge| badge.count(3), |badge| badge.count(1000), |badge| badge.count(123_456).max(999_999), Badge::dot];
        for with in marks {
            let frame = tree_of(|child| with(Badge::new(child, palette())).description("new").render()).layout(window());
            assert_eq!(frame.regions, plain.regions);
        }
        // In a taller row a child of a fixed size keeps it, and one that flexes keeps its share.
        let beside = |marked: bool| {
            let mark = |child: Element| if marked { Badge::new(child, palette()).count(5).render() } else { child };
            let fixed = Element::clickable("fixed", Element::spacer()).size(Some(32.0), Some(32.0));
            let wide = Element::clickable("wide", Element::spacer()).flex(1.0);
            let tall = Element::clickable("tall", Element::spacer()).size(Some(20.0), Some(80.0));
            UiTree::new(Element::row(vec![mark(fixed), mark(wide), tall])).layout(window()).regions
        };
        assert_eq!(beside(true), beside(false));
        // A child smaller than the mark keeps its size too.
        let small = |marked: bool| {
            let dot = Element::spacer().size(Some(6.0), Some(6.0));
            let child = if marked { Badge::new(dot, palette()).count(100).render() } else { dot };
            let row = Element::row(vec![child, bell("after"), Element::spacer().flex(1.0)]);
            UiTree::new(Element::column(vec![row, Element::spacer().flex(1.0)])).layout(window()).regions
        };
        assert_eq!(small(true), small(false));
    }

    #[test]
    fn the_child_keeps_its_region_focus_and_action_under_the_mark() {
        for dot in [false, true] {
            let mut tree = tree_of(|child| {
                let badge = Badge::new(child, palette()).count(100).color(FILL);
                if dot { badge.dot() } else { badge }.render()
            });
            let frame = tree.layout(window());
            // The badge adds no region and no tab stop of its own.
            let ids: Vec<_> = frame.regions.iter().map(|region| region.id.as_str()).collect();
            assert_eq!(ids, ["before", "bell", "after"]);
            let child = region(&frame, "bell");
            let (over, _) = mark(&frame, FILL).unwrap();
            let on_mark = Point::new(over.x + over.width / 2.0, over.y + over.height / 2.0);
            let off_mark = Point::new(child.x + 4.0, child.y + child.height - 4.0);
            for point in [on_mark, off_mark] {
                tree.handle_event(UiEvent::PointerMove(point));
                assert_eq!(tree.hovered_id(), Some("bell"));
                tree.handle_event(UiEvent::PointerDown(point));
                assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: "bell".into() }]);
            }
            assert!(tree.focus("bell"));
            for key in [Key::Enter, Key::Space] {
                assert_eq!(
                    tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false, alt: false }),
                    vec![UiAction::Clicked { id: "bell".into() }]
                );
            }
            tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false, alt: false });
            assert_eq!(tree.focused_id(), Some("after"));
        }
    }

    #[test]
    fn the_description_reaches_the_child_control_and_the_badge_has_no_node() {
        let nodes = |mark: fn(Badge) -> Badge| {
            let frame = tree_of(|child| mark(Badge::new(child, palette())).render()).layout(window());
            let ids: Vec<_> = frame.access.iter().map(|node| node.id.clone()).collect();
            assert_eq!(ids, ["before", "bell", "after"]);
            let said = frame.access.iter().find(|node| node.id == "bell").unwrap().clone();
            assert_eq!((said.role, said.name.as_str(), said.actionable), (Role::Button, "Notifications", true));
            // What is said of the marked control goes no further than it.
            assert!(frame.access.iter().all(|node| node.id == "bell" || node.description.is_none()));
            said.description
        };
        assert_eq!(nodes(|badge| badge.count(3).description("3 new")).as_deref(), Some("3 new"));
        assert_eq!(nodes(|badge| badge.dot().description("New activity")).as_deref(), Some("New activity"));
        // No wording is invented, and nothing is said of a mark that does not show.
        assert_eq!(nodes(|badge| badge.count(3)), None);
        assert_eq!(nodes(|badge| badge.count(0).description("0 new")), None);
    }

    #[test]
    fn a_disabled_child_stays_disabled_and_described_under_its_badge() {
        let disabled = || Element::panel(Element::icon(Icon::Bell)).accessible(Access::new(Role::Button).id("off").name("Inbox").disabled(true));
        let mut tree = UiTree::new(Element::row(vec![Badge::new(disabled(), palette()).count(2).description("2 new").render()]));
        let frame = tree.layout(window());
        assert!(frame.regions.is_empty());
        let [said] = frame.access.as_slice() else {
            panic!("one node, got {:?}", frame.access);
        };
        assert_eq!((said.id.as_str(), said.disabled, said.actionable), ("off", true, false));
        assert_eq!(said.description.as_deref(), Some("2 new"));
        let point = Point::new(4.0, 4.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
    }
}
