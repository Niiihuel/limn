//! The bar across the top of a music app's window: the host's controls at
//! both ends, and a round Home button and a wide search in the middle.

use crate::components::{clear_button, disabled_field, hover_ms, lighter, CLEAR_SIDE};
use crate::{
    Access, ButtonSizing, ButtonVariant, Component, ComponentPalette, ControlSize, Element,
    FontFamily, Icon, IconButton, Role, Separator, TooltipPlacement, ICON_SIZE,
};

/// Height of the bar, in logical pixels.
const HEIGHT: f32 = 64.0;
/// Between the bar's edges and what it holds, and between its controls.
const SPACE: f32 = 8.0;
/// Side of the Home button and height of the search.
const CONTROL: f32 = 48.0;
/// Side of the browse button, 4 inside the end of the search.
const BROWSE: f32 = 40.0;
/// The search takes half the bar's width, kept between these.
const SEARCH_MIN: f32 = 350.0;
const SEARCH_MAX: f32 = 546.0;
/// A search this wide or narrower is a round button.
const SEARCH_COLLAPSE: f32 = 400.0;
const SEARCH_FONT: f32 = 16.0;
/// Room around the magnifier and the text.
const SEARCH_PADDING: f32 = 12.0;
/// The rule before the browse button, and the space on each side of it.
const DIVIDER: f32 = 24.0;
const DIVIDER_MARGIN: f32 = 8.0;
/// Between the clear button and the rule.
const CLEAR_GAP: f32 = 4.0;

/// The bar across the top of a window, laid out as a music app's: the host's
/// controls at the start, such as back and forward; a round Home button and
/// a search in the middle of the window; the host's controls at the end,
/// such as notifications, settings and the account. It is 64 px tall.
///
/// The host owns the query and the page shown. The search reports
/// `InputChanged` and `Submitted` with `{id}:search`, and its clear button
/// `Clicked` with `{id}:search:clear`, as a [`SearchField`](crate::SearchField)
/// with that ID would; Home reports `Clicked` with `{id}:home` and the
/// browse button `{id}:browse`. Tab goes from the start controls to Home,
/// the search, its buttons and the end controls.
pub struct TopBar {
    id: String,
    query: String,
    placeholder: String,
    palette: ComponentPalette,
    width: Option<f32>,
    caption: (f32, f32),
    leading: Vec<Element>,
    trailing: Vec<Element>,
    home_active: bool,
    browse_active: bool,
    home_label: String,
    browse_label: String,
    enabled: bool,
}

impl TopBar {
    /// A bar whose search shows `query`, and `placeholder` while it is
    /// empty. The host stores what `InputChanged` reports for `{id}:search`.
    pub fn new(
        id: impl Into<String>,
        query: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            query: query.into(),
            placeholder: placeholder.into(),
            palette,
            width: None,
            caption: (0.0, 0.0),
            leading: Vec::new(),
            trailing: Vec::new(),
            home_active: false,
            browse_active: false,
            home_label: "Home".into(),
            browse_label: "Browse".into(),
            enabled: true,
        }
    }

    /// The width the bar is laid out at, the window's, in logical pixels.
    /// The search takes half of it, kept between 350 and 546; when that is
    /// 400 or less the search is a round button with a magnifier, which
    /// reports `Clicked` with `{id}:search`. Without a width the search is
    /// 546 wide. A width that is not finite is ignored and a negative one is
    /// zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Keep `left` and `right` logical pixels free at the start and the end
    /// of the bar for the window's own controls, such as the buttons macOS
    /// draws at the start or Windows at the end. The search stays centered
    /// in the whole bar. A value that is not finite leaves that side as it
    /// was and a negative one is zero.
    pub fn caption(mut self, left: f32, right: f32) -> Self {
        let inset = |value: f32, current: f32| if value.is_finite() { value.max(0.0) } else { current };
        self.caption = (inset(left, self.caption.0), inset(right, self.caption.1));
        self
    }

    /// The host's controls at the start, such as back and forward buttons:
    /// 8 apart, each centered on the bar's height, first in the Tab order.
    pub fn leading(mut self, controls: Vec<Element>) -> Self {
        self.leading = controls;
        self
    }

    /// The host's controls at the end, such as a bell, settings and the
    /// account's avatar: 8 apart, each centered on the bar's height, last in
    /// the Tab order.
    pub fn trailing(mut self, controls: Vec<Element>) -> Self {
        self.trailing = controls;
        self
    }

    /// Draw Home as the page shown: its icon solid and in the palette's
    /// text color, where it is otherwise outlined and muted.
    pub fn home_active(mut self, active: bool) -> Self {
        self.home_active = active;
        self
    }

    /// Draw the browse button as the page shown, as [`Self::home_active`].
    pub fn browse_active(mut self, active: bool) -> Self {
        self.browse_active = active;
        self
    }

    /// The hint of the Home button, which also names it to the platform;
    /// "Home" unless set.
    pub fn home_label(mut self, label: impl Into<String>) -> Self {
        self.home_label = label.into();
        self
    }

    /// The hint and name of the browse button; "Browse" unless set.
    pub fn browse_label(mut self, label: impl Into<String>) -> Self {
        self.browse_label = label.into();
        self
    }

    /// Disable Home and the search: they keep their place and their query
    /// and cannot be used. The host's controls are left as given.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled)
    /// for a state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // Home, and the search when it has no room: a circle on the raised fill.
    fn round(&self, id: String, icon: Icon, active: bool, label: &str) -> Element {
        IconButton::new(id, icon, self.palette)
            .variant(ButtonVariant::Secondary)
            .round()
            .sizing(ButtonSizing { height: CONTROL, ..ControlSize::Spacious.button_sizing() })
            .ink(if active { self.palette.text } else { self.palette.muted })
            .hover_ink(self.palette.text)
            .tooltip(label)
            .tooltip_placement(TooltipPlacement::Below)
            .enabled(self.enabled)
            .render()
    }

    fn search(&self, search: String, width: f32) -> Element {
        let palette = self.palette;
        let clear = format!("{search}:clear");
        let browse = format!("{}:browse", self.id);
        let has_query = !self.query.is_empty();
        let mut end = vec![Element::spacer().flex(1.0)];
        if has_query {
            end.push(clear_button(clear.clone(), palette, self.enabled).centered_y());
            end.push(Element::spacer().size(Some(CLEAR_GAP), None));
        }
        end.push(Separator::vertical(palette).length(DIVIDER).margin(DIVIDER_MARGIN).centered_in(CONTROL).ink(palette.muted).render());
        end.push(
            IconButton::new(browse.clone(), if self.browse_active { Icon::GridFill } else { Icon::Grid }, palette)
                .variant(ButtonVariant::Ghost)
                .round()
                .sizing(ButtonSizing { height: BROWSE, ..ControlSize::Spacious.button_sizing() })
                .ink(if self.browse_active { palette.text } else { palette.muted })
                .hover_ink(palette.text)
                .tooltip(self.browse_label.as_str())
                .tooltip_placement(TooltipPlacement::Below)
                .enabled(self.enabled)
                .render()
                .centered_y(),
        );
        end.push(Element::spacer().size(Some((CONTROL - BROWSE) / 2.0), None));
        // What the text leaves free at the end, under the controls above.
        let end_width = if has_query { CLEAR_SIDE + CLEAR_GAP } else { 0.0 }
            + 1.0
            + DIVIDER_MARGIN * 2.0
            + BROWSE
            + (CONTROL - BROWSE) / 2.0;
        let name = crate::labels().search;
        let field = if self.enabled {
            Element::text_input(search.clone(), self.query.clone(), self.placeholder.clone())
                .leading_icon(Icon::Search)
                .leading_icon_tint(palette.muted)
                .foreground(palette.text)
                .font_family(FontFamily::Sans)
                .font_size(SEARCH_FONT)
                .padding(SEARCH_PADDING)
                .radius(CONTROL / 2.0)
                .focus_border(palette.text, 2.0)
                .input_trailing_space(end_width)
                .accessible(Access::new(Role::SearchInput).name(name))
        } else {
            let shown = if has_query { self.query.clone() } else { self.placeholder.clone() };
            Element::row(vec![
                Element::icon(Icon::Search)
                    .foreground(palette.muted)
                    .size(Some(ICON_SIZE), Some(ICON_SIZE))
                    .centered_y(),
                Element::truncated_text(shown)
                    .foreground(palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(SEARCH_FONT)
                    .centered_y()
                    .flex(1.0),
                Element::spacer().size(Some(end_width), None),
            ])
            .gap(SPACE)
            .padding(SEARCH_PADDING)
            .accessible(disabled_field(Role::SearchInput, search.clone(), Some(name.into()), &self.placeholder, Some(self.query.clone())))
        };
        let pill = Element::stack(vec![field, Element::row(end)])
            .background(palette.raised)
            // A shape, not a token: half the height.
            .radius(CONTROL / 2.0)
            .size(Some(width), Some(CONTROL))
            // Narrower when the bar's ends leave it less room.
            .shrink_to_fit();
        if !self.enabled {
            return pill;
        }
        // One fill for the field and its buttons, lighter under the pointer.
        pill.hover_background(lighter(palette.raised, 10))
            .hover_surface(format!("{search}:fill"), [search, clear, browse])
            .transition_ms(hover_ms())
    }
}

impl Component for TopBar {
    fn render(mut self) -> Element {
        let side = |controls: Vec<Element>| {
            Element::row(controls.into_iter().map(Element::centered_y).collect()).gap(SPACE)
        };
        let leading = Element::row(vec![
            Element::spacer().size(Some(self.caption.0), None),
            side(std::mem::take(&mut self.leading)),
        ]);
        let trailing = Element::row(vec![
            side(std::mem::take(&mut self.trailing)),
            Element::spacer().size(Some(self.caption.1), None),
        ]);
        let home_icon = if self.home_active { Icon::HomeFill } else { Icon::Home };
        let home = self.round(format!("{}:home", self.id), home_icon, self.home_active, &self.home_label);
        let search = format!("{}:search", self.id);
        let width = self.width.map_or(SEARCH_MAX, |width| (width / 2.0).clamp(SEARCH_MIN, SEARCH_MAX));
        let search = if width <= SEARCH_COLLAPSE {
            self.round(search, Icon::Search, false, crate::labels().search)
        } else {
            self.search(search, width)
        };
        Element::row(vec![leading, Element::row(vec![home, search]).gap(SPACE), trailing])
            .gap(SPACE)
            .center_middle()
            .padding(SPACE)
            .size(None, Some(HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn button(id: &str, icon: Icon) -> Element {
        IconButton::new(id, icon, palette()).accessibility_label(id).render()
    }

    fn bar(width: f32) -> TopBar {
        TopBar::new("bar", "", "What do you want to play?", palette())
            .width(width)
            .leading(vec![button("back", Icon::ArrowLeft), button("forward", Icon::ArrowRight)])
            .trailing(vec![button("bell", Icon::Bell), button("settings", Icon::Settings)])
    }

    fn tree(bar: TopBar, width: f32) -> (UiTree, Frame) {
        let mut tree = UiTree::new(bar.render());
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _size, _family| text.chars().count() as f32 * 8.0);
        let frame = tree.layout(Size::new(width, HEIGHT));
        (tree, frame)
    }

    fn rect(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap_or_else(|| panic!("no region {id}")).rect
    }

    fn has(frame: &Frame, id: &str) -> bool {
        frame.regions.iter().any(|region| region.id == id)
    }

    fn center(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn click(tree: &mut UiTree, frame: &Frame, id: &str) -> Vec<UiAction> {
        let point = center(rect(frame, id));
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point))
    }

    fn key(key: Key) -> UiEvent {
        UiEvent::Key { key, shift: false, ctrl: false }
    }

    fn icons(frame: &Frame) -> Vec<Icon> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { icon, .. } => Some(*icon),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_search_takes_half_the_window_between_350_and_546_centered_with_home() {
        for (window, search) in [(900.0, 450.0), (1400.0, 546.0), (1092.0, 546.0), (820.0, 410.0)] {
            let (_, frame) = tree(bar(window), window);
            let (home, field) = (rect(&frame, "bar:home"), rect(&frame, "bar:search"));
            assert_eq!(field.width, search, "at {window}");
            assert_eq!((home.width, home.height, field.height), (CONTROL, CONTROL, CONTROL));
            assert_eq!(field.x, home.x + CONTROL + SPACE);
            assert_eq!(home.x + (CONTROL + SPACE + search) / 2.0, window / 2.0, "centered at {window}");
            assert_eq!(field.y, (HEIGHT - CONTROL) / 2.0);
        }
    }

    #[test]
    fn caption_insets_keep_the_window_controls_ends_free_and_the_search_centered() {
        let window = 1200.0;
        let (_, frame) = tree(bar(window).caption(72.0, 138.0), window);
        assert_eq!(rect(&frame, "back").x, SPACE + 72.0);
        let settings = rect(&frame, "settings");
        assert_eq!(settings.x + settings.width, window - SPACE - 138.0);
        let home = rect(&frame, "bar:home");
        assert_eq!(home.x + (CONTROL + SPACE + 546.0) / 2.0, window / 2.0);
        assert!(rect(&frame, "forward").x < home.x && rect(&frame, "bar:search").x + 546.0 < rect(&frame, "bell").x);
    }

    #[test]
    fn a_search_that_would_meet_a_side_follows_the_leading_controls() {
        let window = 1000.0;
        let wide = Element::button("premium", "Explore Premium").size(Some(380.0), Some(32.0));
        let (_, frame) = tree(bar(window).trailing(vec![wide]), window);
        let forward = rect(&frame, "forward");
        assert_eq!(rect(&frame, "bar:home").x, forward.x + forward.width + SPACE);
        // A wide caption at the start does the same.
        let (_, frame) = tree(bar(window).caption(300.0, 0.0), window);
        let forward = rect(&frame, "forward");
        assert_eq!(rect(&frame, "bar:home").x, forward.x + forward.width + SPACE);
    }

    #[test]
    fn a_search_with_no_room_between_the_ends_narrows_to_the_room_left() {
        let window = 1000.0;
        let wide = |id: &str| Element::button(id, "Explore Premium").size(Some(260.0), Some(32.0));
        let (_, frame) = tree(bar(window).leading(vec![wide("install")]).trailing(vec![wide("premium")]), window);
        let (install, field, premium) = (rect(&frame, "install"), rect(&frame, "bar:search"), rect(&frame, "premium"));
        assert_eq!(rect(&frame, "bar:home").x, install.x + install.width + SPACE);
        assert_eq!(field.x + field.width, premium.x - SPACE);
        assert!(field.width < 500.0);
        let browse = rect(&frame, "bar:browse");
        assert_eq!(browse.x + browse.width, field.x + field.width - (CONTROL - BROWSE) / 2.0);
    }

    #[test]
    fn a_search_400_wide_or_less_is_a_round_button() {
        let (mut tree, frame) = tree(bar(800.0), 800.0);
        let search = rect(&frame, "bar:search");
        assert_eq!((search.width, search.height), (CONTROL, CONTROL));
        assert!(!has(&frame, "bar:browse"));
        assert_eq!(click(&mut tree, &frame, "bar:search"), vec![UiAction::Clicked { id: "bar:search".into() }]);
        let (_, frame) = self::tree(bar(802.0), 802.0);
        assert_eq!(rect(&frame, "bar:search").width, 401.0);
        assert!(has(&frame, "bar:browse"));
    }

    #[test]
    fn the_search_ends_in_a_rule_and_browse_with_a_clear_button_before_them_while_there_is_a_query() {
        let rules = |frame: &Frame| -> Vec<Rect> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == palette().muted && rect.width == 1.0 => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let (_, empty) = tree(bar(1200.0), 1200.0);
        let (field, browse) = (rect(&empty, "bar:search"), rect(&empty, "bar:browse"));
        assert_eq!(browse.x + browse.width, field.x + field.width - (CONTROL - BROWSE) / 2.0);
        assert_eq!(browse.height, BROWSE);
        let [rule] = rules(&empty)[..] else { panic!("one rule: {:?}", rules(&empty)) };
        assert_eq!((rule.height, rule.x + 1.0 + DIVIDER_MARGIN), (DIVIDER, browse.x));
        assert_eq!(rule.y + DIVIDER / 2.0, field.y + CONTROL / 2.0);
        assert!(!has(&empty, "bar:search:clear"));

        let (_, typed) = tree(TopBar { query: "abba".into(), ..bar(1200.0) }, 1200.0);
        let clear = rect(&typed, "bar:search:clear");
        let [rule] = rules(&typed)[..] else { panic!("one rule") };
        assert_eq!(clear.x + clear.width + CLEAR_GAP + DIVIDER_MARGIN, rule.x);
        assert_eq!(rect(&typed, "bar:browse"), browse, "browse keeps its place");
    }

    #[test]
    fn home_and_browse_are_solid_while_they_are_the_page_shown() {
        let (_, frame) = tree(bar(1200.0), 1200.0);
        let drawn = icons(&frame);
        assert!(drawn.contains(&Icon::Home) && drawn.contains(&Icon::Grid) && drawn.contains(&Icon::Search));
        assert!(!drawn.contains(&Icon::HomeFill) && !drawn.contains(&Icon::GridFill));
        let (_, frame) = tree(bar(1200.0).home_active(true).browse_active(true), 1200.0);
        let drawn = icons(&frame);
        assert!(drawn.contains(&Icon::HomeFill) && drawn.contains(&Icon::GridFill));
        assert!(!drawn.contains(&Icon::Home) && !drawn.contains(&Icon::Grid));
        assert_eq!(Icon::Home.filled(), Icon::HomeFill);
    }

    #[test]
    fn the_search_is_lighter_under_the_pointer_and_outlined_while_focused() {
        let fill = |frame: &Frame, field: Rect| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *rect == field => Some(*color),
                _ => None,
            })
        };
        let (mut tree, frame) = tree(bar(1200.0), 1200.0);
        let field = rect(&frame, "bar:search");
        assert_eq!(fill(&frame, field), Some(palette().raised));
        // Over the browse button the whole search stays lit.
        tree.handle_event(UiEvent::PointerMove(center(rect(&frame, "bar:browse"))));
        let lit = tree.layout(Size::new(1200.0, HEIGHT));
        assert_eq!(fill(&lit, field), Some(lighter(palette().raised, 10)));
        assert!(tree.focus("bar:search"));
        let focused = tree.layout(Size::new(1200.0, HEIGHT));
        assert!(focused.commands.iter().any(|command| matches!(command,
            DrawCommand::StrokeRect { rect, color, width, .. } if *rect == field && *color == palette().text && *width == 2.0)));
    }

    #[test]
    fn every_action_is_reported_by_pointer_and_keyboard() {
        let (mut tree, frame) = tree(TopBar { query: "abba".into(), ..bar(1200.0) }, 1200.0);
        for id in ["bar:home", "bar:browse", "bar:search:clear"] {
            assert_eq!(click(&mut tree, &frame, id), vec![UiAction::Clicked { id: id.into() }], "click {id}");
        }
        // Tab: the start controls, Home, the search and its buttons, the end controls.
        assert!(tree.focus("back"));
        let mut order = Vec::new();
        for _ in 0..7 {
            tree.handle_event(key(Key::Tab));
            order.extend(tree.focused_id().map(str::to_owned));
        }
        assert_eq!(order, ["forward", "bar:home", "bar:search", "bar:search:clear", "bar:browse", "bell", "settings"]);
        for (id, pressed) in [("bar:home", Key::Enter), ("bar:browse", Key::Space), ("bar:search:clear", Key::Enter)] {
            assert!(tree.focus(id));
            assert_eq!(tree.handle_event(key(pressed)), vec![UiAction::Clicked { id: id.into() }], "{pressed:?} on {id}");
        }
        assert!(tree.focus("bar:search"));
        let typed = tree.handle_event(UiEvent::Text("!".into()));
        assert!(typed.contains(&UiAction::InputChanged { id: "bar:search".into(), value: "abba!".into() }), "{typed:?}");
        assert_eq!(
            tree.handle_event(key(Key::Enter)),
            vec![UiAction::Submitted { id: "bar:search".into(), value: "abba!".into() }]
        );
    }

    #[test]
    fn disabled_home_and_search_are_passive_and_keep_their_place() {
        let (_, enabled) = tree(TopBar { query: "abba".into(), ..bar(1200.0) }, 1200.0);
        let (mut tree, disabled) = tree(TopBar { query: "abba".into(), ..bar(1200.0) }.disabled(), 1200.0);
        // A disabled button keeps only the region of its hint.
        let control = |frame: &Frame, id: &str| {
            frame.regions.iter().any(|region| region.id == id && matches!(region.kind, HitKind::Button | HitKind::TextInput))
        };
        for id in ["bar:home", "bar:search", "bar:search:clear", "bar:browse"] {
            assert!(control(&enabled, id), "{id} has no region enabled");
            assert!(!control(&disabled, id), "{id} has a region");
            assert!(!tree.focus(id), "{id} takes the focus");
        }
        // The host's controls are its own.
        assert!(has(&disabled, "back") && has(&disabled, "settings"));
        let fills = |frame: &Frame| -> Vec<Rect> {
            frame.commands.iter().filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == palette().raised => Some(*rect),
                _ => None,
            }).collect()
        };
        assert_eq!(fills(&disabled), fills(&enabled));
        assert!(icons(&disabled).contains(&Icon::Search));
        let (_, both) = self::tree(bar(1200.0).enabled(false), 1200.0);
        assert!(!control(&both, "bar:home"));
    }

    #[test]
    fn numbers_that_are_not_sizes_never_reach_the_layout() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -40.0] {
            let (_, frame) = tree(bar(1200.0).width(bad).caption(bad, bad), 1200.0);
            for region in &frame.regions {
                let r = region.rect;
                assert!([r.x, r.y, r.width, r.height].iter().all(|value| value.is_finite()), "{bad}: {region:?}");
            }
            assert_eq!(rect(&frame, "back").x, SPACE, "{bad}: no inset");
        }
        // Not finite is ignored; negative is zero, which leaves no room for the field.
        let (_, frame) = tree(bar(1200.0).width(f32::NAN), 1200.0);
        assert_eq!(rect(&frame, "bar:search").width, 546.0);
        let (_, frame) = tree(bar(1200.0).width(-1.0), 1200.0);
        assert_eq!(rect(&frame, "bar:search").width, CONTROL);
        let (_, frame) = tree(bar(1200.0).caption(10.0, 20.0).caption(f32::NAN, -3.0), 1200.0);
        assert_eq!(rect(&frame, "back").x, SPACE + 10.0);
        let settings = rect(&frame, "settings");
        assert_eq!(settings.x + settings.width, 1200.0 - SPACE);
    }
}
