//! One choice among several: a circle for each option, with a dot in the
//! chosen one and a label beside it that is part of the click target.

use crate::{Access, Color, Component, ComponentPalette, ControlSize, Element, FontFamily, Role};

/// Space between a circle and its label.
const GAP: f32 = 8.0;
const BORDER: f32 = 1.0;
/// Space between two options, one under the other and side by side.
const COLUMN_GAP: f32 = 8.0;
const ROW_GAP: f32 = 16.0;
/// Space between a label and the description under it.
const COPY_GAP: f32 = 2.0;

/// The circle's diameter and the label's text size, for a control size: the
/// same as a `Checkbox`, so the two line up in one form.
const fn metrics(size: ControlSize) -> (f32, f32) {
    match size {
        ControlSize::Dense => (14.0, 12.0),
        ControlSize::Compact => (16.0, 13.0),
        ControlSize::Regular => (16.0, 14.0),
        ControlSize::Spacious => (18.0, 16.0),
    }
}

/// The dot's diameter: about two fifths of the circle's, and even, so it
/// centers on whole pixels.
pub(crate) fn dot_side(side: f32) -> f32 {
    2.0 * (side / 5.0).round()
}

/// One option of a [`RadioGroup`].
pub struct RadioOption {
    value: String,
    label: String,
    description: String,
    enabled: bool,
}

impl RadioOption {
    /// An option that reports `UiAction::Clicked` with `value` as its ID when
    /// it is chosen, as a [`Tab`](crate::Tab) reports its `action_id`. The
    /// value must be unique in the window.
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self { value: value.into(), label: label.into(), description: String::new(), enabled: true }
    }

    /// A muted line under the label, clicked with it and given to the
    /// platform as the option's description. An empty one is no line.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }

    /// A disabled option keeps its place, in the muted color, and cannot be
    /// chosen; the arrow keys pass over it.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

/// The host owns the choice: the group marks the option whose value is
/// `selected` and reports `UiAction::Clicked` with the value of the option
/// that is clicked, from its circle, its label or its description. Options
/// stand one under the other unless [`horizontal`](Self::horizontal).
/// The arrow keys, Home and End move the focus among the enabled options and
/// report the one they reach, in either layout.
pub struct RadioGroup {
    id: String,
    options: Vec<RadioOption>,
    selected: Option<String>,
    horizontal: bool,
    size: ControlSize,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl RadioGroup {
    /// A vertical group of these options, `ControlSize::Regular`. `selected`
    /// is the chosen option's value; `None`, or a value no option has, marks
    /// none. `id` names the group and keys what it animates.
    pub fn new(
        id: impl Into<String>,
        options: Vec<RadioOption>,
        selected: Option<&str>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            options,
            selected: selected.map(str::to_owned),
            horizontal: false,
            size: ControlSize::Regular,
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// Lay the options side by side, 16 pixels apart, instead of one under
    /// the other.
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    /// The circle's diameter and the label's text: 14 and 12 pixels for
    /// `Dense`, 16 and 13 for `Compact`, 16 and 14 for `Regular`, 18 and 16
    /// for `Spacious`. An option's height is its text's, not the control height.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A disabled group is passive: every option shows in the muted color,
    /// the chosen one still marked, and none has a hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// What the group is called to the platform: the question its options
    /// answer. With it the group is a node of its own, known by the group's
    /// ID; without it only the options are told.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }
}

impl Component for RadioGroup {
    fn render(self) -> Element {
        let palette = self.palette;
        let (side, font_size) = metrics(self.size);
        let dot = dot_side(side);
        let hover_ms = crate::tokens().motion.hover_ms;
        // The arrow keys step among the options that can be chosen.
        let reachable = self
            .options
            .iter()
            .filter(|option| self.enabled && option.enabled)
            .map(|option| option.value.clone())
            .collect();
        let options = self
            .options
            .into_iter()
            .map(|option| {
                let enabled = self.enabled && option.enabled;
                let chosen = self.selected.as_deref() == Some(option.value.as_str());
                let ink = if enabled { palette.text } else { palette.muted };
                let (fill, edge) = if chosen { (ink, ink) } else { (Color::TRANSPARENT, palette.muted) };
                let key = format!("{}:{}", self.id, option.value);
                // The dot stays in the tree when the circle is empty, and both
                // are keyed, so the mark fades with the fill instead of
                // appearing at once.
                let mark = Element::panel(Element::spacer().size(Some(dot), Some(dot)))
                    .background(if chosen { palette.surface } else { Color::TRANSPARENT })
                    .hover_surface(format!("{key}:dot"), None::<String>)
                    .transition_ms(hover_ms)
                    .radius(dot / 2.0);
                // The empty circle lights while the pointer is anywhere on the option.
                let circle = Element::panel(mark)
                    .padding((side - dot) / 2.0)
                    .background(fill)
                    .hover_background(if chosen { fill } else { palette.hover })
                    .hover_surface(format!("{key}:circle"), enabled.then_some(option.value.as_str()))
                    .transition_ms(hover_ms)
                    .border(edge, BORDER)
                    .radius(side / 2.0)
                    .size(Some(side), Some(side));
                let text = |value: String, size: f32, ink: Color| {
                    Element::text(value).font_family(FontFamily::Sans).font_size(size).foreground(ink).shrink_to_fit()
                };
                let mut access = Access::new(Role::RadioButton)
                    .id(option.value.clone())
                    .name(option.label.clone())
                    .selected(chosen)
                    .disabled(!enabled);
                // The circle centers on the label, whatever line height the
                // host's font gives it; the description goes under the label.
                let mut lines =
                    vec![Element::row(vec![circle.centered_y(), text(option.label, font_size, ink)]).gap(GAP)];
                if !option.description.is_empty() {
                    access = access.description(option.description.clone());
                    lines.push(
                        Element::row(vec![
                            Element::spacer().size(Some(side), None),
                            text(option.description, font_size - 1.0, palette.muted),
                        ])
                        .gap(GAP),
                    );
                }
                // Each level gives way in a narrow width, down to the text, which wraps.
                let content = Element::column(lines).gap(COPY_GAP);
                if enabled {
                    Element::clickable(option.value, content).radius(crate::tokens().radius.control)
                } else {
                    Element::panel(content)
                }
                .shrink_to_fit()
                .accessible(access)
            })
            .collect();
        // A tab strip is what gives arrow keys to a set of actions. It lays
        // its children in a row, so a vertical group is a strip of one column.
        let group = if self.horizontal {
            Element::tab_strip(options, reachable).every_arrow().gap(ROW_GAP)
        } else {
            Element::tab_strip(vec![Element::column(options).gap(COLUMN_GAP).shrink_to_fit()], reachable).every_arrow()
        };
        match self.accessibility_label {
            Some(name) => group.accessible(Access::new(Role::Group).id(self.id).name(name)),
            None => group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const HINT: &str = "Three projects, one seat.";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(600.0, 400.0)
    }

    /// Free, with a description, then Team and Company; Team is the chosen one.
    fn group() -> RadioGroup {
        RadioGroup::new(
            "plan",
            vec![
                RadioOption::new("plan=free", "Free").description(HINT),
                RadioOption::new("plan=team", "Team"),
                RadioOption::new("plan=company", "Company"),
            ],
            Some("plan=team"),
            palette(),
        )
    }

    fn tree_of(group: RadioGroup) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![group.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree
    }

    fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key { key, shift: false, ctrl: false })
    }

    fn clicked(id: &str) -> Vec<UiAction> {
        vec![UiAction::Clicked { id: id.into() }]
    }

    /// Every filled shape of the frame: its rectangle, color and radius.
    fn fills(frame: &Frame) -> Vec<(Rect, Color, f32)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect()
    }

    /// Where each text of the frame starts.
    fn texts(frame: &Frame) -> Vec<(&str, Point, Color)> {
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
    fn the_chosen_circle_is_filled_with_the_text_color_and_holds_a_centered_dot_of_the_surface() {
        let frame = tree_of(group()).layout(window());
        let fills = fills(&frame);
        // A circle and its dot for each option, in order.
        assert_eq!(fills.len(), 6);
        for (index, pair) in fills.chunks(2).enumerate() {
            let ((circle, fill, radius), (dot, ink, dot_radius)) = (pair[0], pair[1]);
            assert_eq!((circle.width, circle.height, radius), (16.0, 16.0, 8.0));
            assert_eq!((dot.width, dot.height, dot_radius), (6.0, 6.0, 3.0));
            assert_eq!((dot.x - circle.x, dot.y - circle.y), (5.0, 5.0));
            let chosen = index == 1;
            let expected = if chosen { (palette().text, palette().surface) } else { (Color::TRANSPARENT, Color::TRANSPARENT) };
            assert_eq!((fill, ink), expected);
        }
        let edges: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StrokeRect { color, radius, .. } => Some((*color, *radius)),
                _ => None,
            })
            .collect();
        assert_eq!(edges, [(palette().muted, 8.0), (palette().text, 8.0), (palette().muted, 8.0)]);
    }

    #[test]
    fn a_value_no_option_has_and_no_value_choose_none() {
        for selected in [None, Some("plan=other")] {
            let options = vec![RadioOption::new("plan=free", "Free"), RadioOption::new("plan=team", "Team")];
            let frame = tree_of(RadioGroup::new("plan", options, selected, palette())).layout(window());
            assert!(fills(&frame).iter().all(|fill| fill.1 == Color::TRANSPARENT));
            assert!(frame.access.iter().all(|node| node.selected == Some(false)));
        }
    }

    #[test]
    fn the_circle_is_centered_on_its_label_and_the_description_starts_under_the_label() {
        let frame = tree_of(group()).layout(window());
        let texts = texts(&frame);
        assert_eq!(texts.iter().map(|text| text.0).collect::<Vec<_>>(), ["Free", HINT, "Team", "Company"]);
        assert_eq!((texts[0].2, texts[1].2), (palette().text, palette().muted));
        let circle = fills(&frame)[0].0;
        assert_eq!(texts[0].1.x, circle.x + circle.width + GAP);
        assert_eq!(texts[1].1.x, texts[0].1.x);
        assert!(texts[1].1.y > circle.y + circle.height);
        // Team has one line, so its region is that line: the circle is in its middle.
        let team = frame.regions.iter().find(|region| region.id == "plan=team").unwrap().rect;
        let circle = fills(&frame)[2].0;
        assert_eq!(circle.y + circle.height / 2.0, team.y + team.height / 2.0);
    }

    #[test]
    fn options_stand_in_a_column_and_horizontal_lays_them_in_a_row() {
        let places = |group: RadioGroup| {
            let frame = tree_of(group).layout(window());
            frame.regions.iter().map(|region| (region.rect.x, region.rect.y)).collect::<Vec<_>>()
        };
        let column = places(group());
        assert!(column.windows(2).all(|pair| pair[0].0 == pair[1].0 && pair[0].1 < pair[1].1));
        let row = places(group().horizontal());
        assert!(row.windows(2).all(|pair| pair[0].1 == pair[1].1 && pair[0].0 < pair[1].0));
    }

    #[test]
    fn a_click_on_the_circle_the_label_or_the_description_reports_the_options_value() {
        let mut tree = tree_of(group());
        let frame = tree.layout(window());
        assert!(frame.regions.iter().all(|region| region.kind == HitKind::Button));
        assert_eq!(
            frame.regions.iter().map(|region| region.id.as_str()).collect::<Vec<_>>(),
            ["plan=free", "plan=team", "plan=company"]
        );
        let circle = fills(&frame)[0].0;
        let texts = texts(&frame);
        for point in [
            Point::new(circle.x + 8.0, circle.y + 8.0),
            Point::new(texts[0].1.x + 4.0, texts[0].1.y + 4.0),
            Point::new(texts[1].1.x + 40.0, texts[1].1.y + 4.0),
        ] {
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked("plan=free"));
        }
    }

    #[test]
    fn the_empty_circle_lights_while_the_pointer_is_on_its_label() {
        let mut tree = tree_of(group());
        let frame = tree.layout(window());
        let label = texts(&frame)[0].1;
        tree.handle_event(UiEvent::PointerMove(Point::new(label.x + 4.0, label.y + 4.0)));
        let fills = fills(&tree.layout(window()));
        assert_eq!((fills[0].1, fills[2].1, fills[4].1), (palette().hover, palette().text, Color::TRANSPARENT));
    }

    #[test]
    fn a_new_choice_cross_fades_its_circle_and_its_dot_over_the_hover_duration() {
        let pair = |selected| {
            let options = vec![RadioOption::new("a", "A"), RadioOption::new("b", "B")];
            Element::column(vec![RadioGroup::new("letters", options, Some(selected), palette()).render()])
        };
        let mut tree = UiTree::new(pair("a"));
        let start = std::time::Instant::now();
        tree.layout_at(window(), start);
        tree.set_root(pair("b"));
        tree.layout_at(window(), start);
        let step = std::time::Duration::from_millis(u64::from(crate::tokens().motion.hover_ms));
        // Halfway, A's circle and dot are leaving and B's arriving: none is at rest.
        let halfway = fills(&tree.layout_at(window(), start + step / 2));
        assert_eq!(halfway.len(), 4);
        let at_rest = [palette().text, palette().surface, Color::TRANSPARENT];
        assert!(halfway.iter().all(|fill| !at_rest.contains(&fill.1)), "{halfway:?}");
        // Arrived, A paints nothing and B is the chosen circle.
        let arrived = fills(&tree.layout_at(window(), start + step * 2));
        assert_eq!(arrived.iter().map(|fill| fill.1).collect::<Vec<_>>(), [palette().text, palette().surface]);
    }

    #[test]
    fn tab_reaches_each_option_and_enter_and_space_choose_it() {
        let mut tree = tree_of(group());
        tree.layout(window());
        for id in ["plan=free", "plan=team", "plan=company"] {
            key(&mut tree, Key::Tab);
            assert_eq!(tree.focused_id(), Some(id));
            assert_eq!(key(&mut tree, Key::Enter), clicked(id));
            assert_eq!(key(&mut tree, Key::Space), clicked(id));
        }
    }

    #[test]
    fn left_right_home_and_end_move_the_choice_in_both_layouts_and_pass_over_a_disabled_option() {
        for horizontal in [false, true] {
            let options = vec![
                RadioOption::new("a", "A"),
                RadioOption::new("b", "B").disabled(),
                RadioOption::new("c", "C"),
                RadioOption::new("d", "D"),
            ];
            let group = RadioGroup::new("letters", options, Some("a"), palette());
            let mut tree = tree_of(if horizontal { group.horizontal() } else { group });
            tree.layout(window());
            assert!(tree.focus("a"));
            for (pressed, reached) in [
                (Key::ArrowRight, "c"),
                (Key::ArrowRight, "d"),
                // It wraps at the ends.
                (Key::ArrowRight, "a"),
                (Key::ArrowLeft, "d"),
                (Key::Home, "a"),
                (Key::End, "d"),
            ] {
                assert_eq!(key(&mut tree, pressed), clicked(reached), "{pressed:?}, horizontal {horizontal}");
                assert_eq!(tree.focused_id(), Some(reached));
            }
        }
    }

    #[test]
    fn up_and_down_move_the_choice_as_left_and_right_do() {
        for horizontal in [false, true] {
            let group = if horizontal { group().horizontal() } else { group() };
            let mut tree = tree_of(group);
            tree.layout(window());
            assert!(tree.focus("plan=team"));
            let down = key(&mut tree, Key::ArrowDown);
            let after_down = tree.focused_id().map(str::to_owned);
            assert!(matches!(down.as_slice(), [UiAction::Clicked { id }] if Some(id.as_str()) == after_down.as_deref() && id != "plan=team"), "{down:?}");
            // And Up comes back to where it was.
            assert_eq!(key(&mut tree, Key::ArrowUp), clicked("plan=team"));
            assert_eq!(tree.focused_id(), Some("plan=team"));
        }
    }

    #[test]
    fn a_disabled_group_has_no_region_takes_no_focus_reports_nothing_and_keeps_its_place() {
        let mut tree = tree_of(group().disabled());
        let frame = tree.layout(window());
        assert!(frame.regions.is_empty());
        assert!(!tree.focus("plan=team"));
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), None);
        let enabled = tree_of(group().enabled(true)).layout(window());
        for region in &enabled.regions {
            let point = Point::new(region.rect.x + 4.0, region.rect.y + 4.0);
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        }
        // The same shapes and lines in the same places, so a form does not move.
        let rects = |frame: &Frame| fills(frame).iter().map(|fill| fill.0).collect::<Vec<_>>();
        let lines = |frame: &Frame| texts(frame).iter().map(|text| text.1).collect::<Vec<_>>();
        assert_eq!((rects(&frame), lines(&frame)), (rects(&enabled), lines(&enabled)));
        // Muted, with the choice still marked.
        assert!(texts(&frame).iter().all(|text| text.2 == palette().muted));
        let fills = fills(&frame);
        assert_eq!((fills[2].1, fills[3].1), (palette().muted, palette().surface));
    }

    #[test]
    fn one_disabled_option_is_passive_and_the_others_still_report() {
        let options = vec![RadioOption::new("a", "A").enabled(false), RadioOption::new("b", "B")];
        let mut tree = tree_of(RadioGroup::new("letters", options, None, palette()));
        let frame = tree.layout(window());
        let [region] = frame.regions.as_slice() else {
            panic!("one region, got {:?}", frame.regions);
        };
        assert_eq!(region.id, "b");
        assert!(!tree.focus("a"));
        let a = fills(&frame)[0].0;
        let point = Point::new(a.x + 8.0, a.y + 8.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        key(&mut tree, Key::Tab);
        assert_eq!(key(&mut tree, Key::Space), clicked("b"));
    }

    #[test]
    fn each_option_is_a_radio_button_named_by_its_label_that_says_whether_it_is_chosen() {
        for enabled in [true, false] {
            let frame = tree_of(group().enabled(enabled)).layout(window());
            let said: Vec<_> = frame
                .access
                .iter()
                .map(|node| (node.id.as_str(), node.role, node.name.as_str(), node.selected, node.description.as_deref()))
                .collect();
            assert_eq!(
                said,
                [
                    ("plan=free", Role::RadioButton, "Free", Some(false), Some(HINT)),
                    ("plan=team", Role::RadioButton, "Team", Some(true), None),
                    ("plan=company", Role::RadioButton, "Company", Some(false), None),
                ]
            );
            assert!(frame.access.iter().all(|node| (node.disabled, node.actionable) == (!enabled, enabled)));
        }
    }

    #[test]
    fn a_named_group_is_a_node_of_its_own_around_its_options() {
        let frame = tree_of(group().accessibility_label("Plan")).layout(window());
        let said = frame.access.iter().find(|node| node.id == "plan").unwrap();
        assert_eq!((said.role, said.name.as_str(), said.actionable), (Role::Group, "Plan", false));
        assert_eq!(frame.access.len(), 4);
        assert!(frame.access.iter().all(|node| {
            node.rect.y >= said.rect.y && node.rect.y + node.rect.height <= said.rect.y + said.rect.height
        }));
    }

    #[test]
    fn a_size_sets_the_circle_and_the_label_text() {
        for (size, side, dot, font) in [
            (ControlSize::Dense, 14.0, 6.0, 12.0),
            (ControlSize::Compact, 16.0, 6.0, 13.0),
            (ControlSize::Regular, 16.0, 6.0, 14.0),
            (ControlSize::Spacious, 18.0, 8.0, 16.0),
        ] {
            let frame = tree_of(group().size(size)).layout(window());
            let fills = fills(&frame);
            assert_eq!((fills[0].0.width, fills[0].2, fills[1].0.width), (side, side / 2.0, dot));
            let sizes: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { font_size, .. } => Some(*font_size),
                    _ => None,
                })
                .collect();
            assert_eq!(sizes, [font, font - 1.0, font, font]);
        }
    }

    #[test]
    fn a_long_description_wraps_inside_a_narrow_width() {
        let frame = tree_of(group()).layout(Size::new(120.0, 400.0));
        assert!(texts(&frame).len() > 4);
        assert!(frame.regions.iter().all(|region| region.rect.x + region.rect.width <= 120.0));
    }
}
