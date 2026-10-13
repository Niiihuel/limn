//! The quiet row of icon actions under a chat message: copy, retry, edit.

use web_time::{Duration, Instant};

use crate::{
    ButtonVariant, Component, ComponentPalette, ControlSize, DropdownMenu, DropdownMenuEntry,
    Element, Icon, IconButton, PopoverPlacement, Text, TextRole, TooltipPlacement, TooltipSpec,
};

// Controls sit 2 apart; a caption keeps 4 clear of its neighbours, which is
// also how far a 16 px glyph sits inside its 24 px button.
const GAP: f32 = 2.0;
const CAPTION_INSET: f32 = 4.0;

/// When a [`MessageActions`] row is painted. It takes the same room in all
/// three, so a message never changes height.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MessageActionsVisibility {
    /// Always painted: the last message of a conversation.
    #[default]
    Always,
    /// Painted while the pointer is over the message, while one of the
    /// row's controls has keyboard focus, or while its menu is open. The
    /// message is the nearest [`Element::hover_region`] around the row.
    Hover,
    /// Room only, with no controls: a message that is still being written.
    Hidden,
}

/// One icon action of a [`MessageActions`] row.
pub struct MessageAction {
    id: String,
    icon: Icon,
    tooltip: TooltipSpec,
    dispatch_as: Option<String>,
    enabled: bool,
    done: bool,
    active: bool,
}

impl MessageAction {
    /// An action that reports a click on `id`. `title` is its hint, shown
    /// below the button; it is the only name an icon has.
    pub fn new(id: impl Into<String>, icon: Icon, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon,
            tooltip: TooltipSpec::new(title).placement(TooltipPlacement::Below),
            dispatch_as: None,
            enabled: true,
            done: false,
            active: false,
        }
    }

    /// Show the keys that trigger the action beside its title.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.tooltip = self.tooltip.shortcut(shortcut);
        self
    }

    /// Replace the hint, for another placement or palette.
    pub fn tooltip_with(mut self, tooltip: TooltipSpec) -> Self {
        self.tooltip = tooltip;
        self
    }

    /// The action cannot be taken, and its hint says why. It is passive:
    /// neither the pointer nor the keyboard reaches it.
    pub fn disabled(mut self, reason: impl Into<String>) -> Self {
        self.enabled = false;
        self.tooltip = self.tooltip.description(reason);
        self
    }

    /// Show a check in place of the icon: the action was just taken. The
    /// host decides for how long; [`MessageActionDone`] keeps that time.
    pub fn done(mut self, done: bool) -> Self {
        self.done = done;
        self
    }

    /// A toggle that is on, such as a rating already given.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Report a click as `action_id` while keeping `id` as the focus stop,
    /// for an action another control of the window also offers.
    pub fn dispatch_as(mut self, action_id: impl Into<String>) -> Self {
        self.dispatch_as = Some(action_id.into());
        self
    }

    fn render(self, palette: ComponentPalette) -> Element {
        let icon = if self.done { Icon::Check } else { self.icon };
        let mut button = IconButton::new(self.id, icon, palette)
            .variant(ButtonVariant::Ghost)
            .size(ControlSize::Dense)
            .ink(if self.done || self.active { palette.text } else { palette.muted })
            .hover_ink(palette.text)
            .active(self.active)
            .enabled(self.enabled)
            .tooltip_with(self.tooltip);
        if let Some(action) = self.dispatch_as {
            button = button.dispatch_as(action);
        }
        button.render()
    }
}

/// The time an action shows as done, kept by the host beside its state.
/// Mark it when the action is taken, pass [`shows`](Self::shows) to
/// [`MessageAction::done`], and ask for one repaint when
/// [`repaint_after`](Self::repaint_after) says the mark ends.
#[derive(Clone, Debug, PartialEq)]
pub struct MessageActionDone {
    id: String,
    since: Instant,
}

impl MessageActionDone {
    /// How long the check stays: long enough to be read, not a transition.
    pub const SHOWN_FOR: Duration = Duration::from_millis(1500);

    /// The action `id` was taken at `now`.
    pub fn new(id: impl Into<String>, now: Instant) -> Self {
        Self { id: id.into(), since: now }
    }

    /// Whether `id` still shows as done at `now`.
    pub fn shows(&self, id: &str, now: Instant) -> bool {
        self.id == id && self.repaint_after(now).is_some()
    }

    /// How long until the mark ends; `None` once it has, when the host
    /// drops it. Nothing needs painting in between.
    pub fn repaint_after(&self, now: Instant) -> Option<Duration> {
        Self::SHOWN_FOR
            .checked_sub(now.saturating_duration_since(self.since))
            .filter(|left| !left.is_zero())
    }
}

enum Item {
    Action(MessageAction),
    Caption(String),
    Overflow { entries: Vec<DropdownMenuEntry>, open: bool, title: String },
}

/// The actions of one chat message: icon-only ghost buttons 24 px high and
/// 2 apart, with muted captions such as a time or a model name, and an
/// overflow menu for what is used less. Items stand in the order they are
/// added. The host owns every state and gets a click per ID; the overflow
/// trigger reports `{id}:more` and its menu dismisses with
/// `{id}:more:dismiss`.
///
/// For a branch position such as "2 / 3", put a [`crate::Pager`] beside it.
pub struct MessageActions {
    id: String,
    items: Vec<Item>,
    align_end: bool,
    visibility: MessageActionsVisibility,
    palette: ComponentPalette,
}

impl MessageActions {
    /// An empty row. `id` must be unique in the window.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            align_end: false,
            visibility: MessageActionsVisibility::Always,
            palette,
        }
    }

    /// Add an icon action.
    pub fn action(mut self, action: MessageAction) -> Self {
        self.items.push(Item::Action(action));
        self
    }

    /// Add passive muted text: a relative time, a duration, a model name.
    /// Empty text is ignored.
    pub fn caption(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        if !text.trim().is_empty() {
            self.items.push(Item::Caption(text));
        }
        self
    }

    /// Add a "···" button that opens `entries` as a menu while `open`.
    /// `title` is the button's hint. Without entries nothing is added.
    pub fn overflow(
        mut self,
        entries: Vec<DropdownMenuEntry>,
        open: bool,
        title: impl Into<String>,
    ) -> Self {
        if !entries.is_empty() {
            self.items.push(Item::Overflow { entries, open, title: title.into() });
        }
        self
    }

    /// Push the row to the far edge: a user's message. The default is the
    /// near edge, where an assistant's text starts.
    pub fn align_end(mut self) -> Self {
        self.align_end = true;
        self
    }

    /// When the row is painted. See [`MessageActionsVisibility`].
    pub fn visibility(mut self, visibility: MessageActionsVisibility) -> Self {
        self.visibility = visibility;
        self
    }
}

impl Component for MessageActions {
    fn render(self) -> Element {
        let height = ControlSize::Dense.height();
        if self.visibility == MessageActionsVisibility::Hidden || self.items.is_empty() {
            return Element::spacer().size(None, Some(height));
        }
        let palette = self.palette;
        let (id, align_end) = (self.id, self.align_end);
        let mut children = Vec::with_capacity(self.items.len() + 1);
        if align_end {
            children.push(Element::spacer().flex(1.0));
        }
        children.extend(self.items.into_iter().map(|item| match item {
            Item::Action(action) => action.render(palette),
            Item::Caption(text) => Element::row(vec![
                Element::spacer().size(Some(CAPTION_INSET), None),
                Text::new(text, palette)
                    .role(TextRole::Small)
                    .muted()
                    .truncate()
                    .render()
                    .centered_y()
                    .shrink_to_fit(),
                Element::spacer().size(Some(CAPTION_INSET), None),
            ])
            .size(None, Some(height))
            .shrink_to_fit(),
            Item::Overflow { entries, open, title } => {
                let trigger = MessageAction::new(format!("{id}:more"), Icon::More, title)
                    .active(open)
                    .tooltip_with_suppressed(open)
                    .render(palette);
                DropdownMenu::new(trigger, entries, open, format!("{id}:more:dismiss"), palette)
                    .context_menu_style()
                    .placement(if align_end {
                        PopoverPlacement::BottomEnd
                    } else {
                        PopoverPlacement::BottomStart
                    })
                    .render()
            }
        }));
        let row = Element::row(children).gap(GAP).size(None, Some(height));
        match self.visibility {
            MessageActionsVisibility::Hover => row.reveal_on_hover(id),
            _ => row,
        }
    }
}

impl MessageAction {
    // The hint of a menu's trigger would cover the menu it opened.
    fn tooltip_with_suppressed(mut self, suppressed: bool) -> Self {
        self.tooltip = self.tooltip.suppressed(suppressed);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DrawCommand, DropdownMenuItem, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent,
        UiTree,
    };

    const SIZE: Size = Size::new(400.0, 200.0);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn copy() -> MessageAction {
        MessageAction::new("copy", Icon::Copy, "Copy")
    }

    // A message 60 high over its actions, both inside one hover region,
    // with room below that belongs to neither.
    fn tree(actions: MessageActions) -> UiTree {
        let message = Element::column(vec![Element::spacer().size(None, Some(60.0)), actions.render()])
            .hover_region();
        let mut tree = UiTree::new(Element::column(vec![message, Element::spacer().flex(1.0)]));
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        tree
    }

    fn button(frame: &Frame, id: &str) -> Option<Rect> {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .map(|region| region.rect)
    }

    // The alpha a glyph is painted with, if it is painted at all.
    fn icon_alpha(frame: &Frame, wanted: Icon) -> Option<u8> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon { icon, tint, .. } if *icon == wanted => Some(tint.a),
            _ => None,
        })
    }

    fn texts(frame: &Frame) -> Vec<(Point, String)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, color, .. } if color.a > 0 => {
                    Some((*position, text.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn click(tree: &mut UiTree, rect: Rect) -> Vec<UiAction> {
        let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        tree.handle_event(UiEvent::PointerUp(center))
    }

    #[test]
    fn actions_are_dense_squares_two_apart_at_either_edge() {
        let row = |end: bool| {
            let row = MessageActions::new("m", palette())
                .action(copy())
                .action(MessageAction::new("retry", Icon::Refresh, "Retry"))
                .caption("2 min")
                .caption("  ");
            if end { row.align_end() } else { row }
        };
        let mut start = tree(row(false));
        start.set_reduced_motion(true);
        let frame = start.layout(SIZE);
        assert_eq!(button(&frame, "copy"), Some(Rect::new(0.0, 60.0, 24.0, 24.0)));
        assert_eq!(button(&frame, "retry"), Some(Rect::new(26.0, 60.0, 24.0, 24.0)));
        let (position, text) = texts(&frame).remove(0);
        assert_eq!(text, "2 min");
        assert_eq!(position.x, 52.0 + CAPTION_INSET);
        assert_eq!(click(&mut start, Rect::new(26.0, 60.0, 24.0, 24.0)), vec![UiAction::Clicked { id: "retry".into() }]);

        let mut end = tree(row(true));
        end.set_reduced_motion(true);
        let frame = end.layout(SIZE);
        let caption = 5.0 * 6.0 + CAPTION_INSET * 2.0;
        let retry = button(&frame, "retry").unwrap();
        assert_eq!(retry.x + retry.width + GAP + caption, SIZE.width);
        assert_eq!(button(&frame, "copy").unwrap().x, retry.x - 26.0);
    }

    #[test]
    fn hover_reveals_the_row_without_moving_anything() {
        let actions = || {
            MessageActions::new("m", palette())
                .action(copy())
                .caption("now")
                .visibility(MessageActionsVisibility::Hover)
        };
        let mut tree = tree(actions());
        let start = Instant::now();
        let rest = tree.layout_at(SIZE, start);
        // Hidden, yet in its place and in the focus order.
        assert_eq!(icon_alpha(&rest, Icon::Copy), Some(0));
        assert!(texts(&rest).is_empty());
        assert_eq!(button(&rest, "copy"), Some(Rect::new(0.0, 60.0, 24.0, 24.0)));

        // Anywhere over the message counts, not only over the row.
        tree.handle_event_at(UiEvent::PointerMove(Point::new(300.0, 10.0)), start);
        tree.layout_at(SIZE, start);
        let motion = crate::tokens().motion;
        let half = start + Duration::from_millis(u64::from(motion.enter_ms) / 2);
        let fading = icon_alpha(&tree.layout_at(SIZE, half), Icon::Copy).unwrap();
        let shown_at = start + Duration::from_millis(u64::from(motion.enter_ms));
        let shown = tree.layout_at(SIZE, shown_at);
        let full = icon_alpha(&shown, Icon::Copy).unwrap();
        assert!(fading > 0 && fading < full);
        assert_eq!(texts(&shown).len(), 1);
        assert_eq!(shown.regions, rest.regions, "revealing is paint only");

        // Below the message the row leaves again, and then rests.
        tree.handle_event_at(UiEvent::PointerMove(Point::new(300.0, 150.0)), shown_at);
        tree.layout_at(SIZE, shown_at);
        let gone = shown_at + Duration::from_millis(u64::from(motion.exit_ms));
        assert_eq!(icon_alpha(&tree.layout_at(SIZE, gone), Icon::Copy), Some(0));
        tree.handle_event_at(UiEvent::PointerMove(Point::new(300.0, 10.0)), gone);
        tree.handle_event_at(UiEvent::PointerLeave, gone);
        assert_eq!(icon_alpha(&tree.layout_at(SIZE, gone + Duration::from_secs(1)), Icon::Copy), Some(0));
    }

    #[test]
    fn a_resting_row_asks_for_no_frames() {
        let mut tree = tree(
            MessageActions::new("m", palette()).action(copy()).visibility(MessageActionsVisibility::Hover),
        );
        tree.layout(SIZE);
        assert!(!tree.is_animating(), "a hidden row at rest costs nothing");
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
        tree.layout(SIZE);
        assert!(tree.is_animating());
        std::thread::sleep(Duration::from_millis(u64::from(crate::tokens().motion.enter_ms) + 30));
        tree.layout(SIZE);
        // The pointer rests on the message, not on a control with a hint.
        assert!(!tree.is_animating(), "the fade ends");
    }

    #[test]
    fn reduced_motion_switches_at_once() {
        let mut tree = tree(
            MessageActions::new("m", palette()).action(copy()).visibility(MessageActionsVisibility::Hover),
        );
        tree.set_reduced_motion(true);
        let now = Instant::now();
        assert_eq!(icon_alpha(&tree.layout_at(SIZE, now), Icon::Copy), Some(0));
        tree.handle_event_at(UiEvent::PointerMove(Point::new(10.0, 10.0)), now);
        assert_eq!(icon_alpha(&tree.layout_at(SIZE, now), Icon::Copy), Some(palette().muted.a));
        tree.handle_event_at(UiEvent::PointerLeave, now);
        assert_eq!(icon_alpha(&tree.layout_at(SIZE, now), Icon::Copy), Some(0));
    }

    #[test]
    fn tab_reaches_a_hidden_row_in_order_and_keeps_it_shown() {
        let mut tree = tree(
            MessageActions::new("m", palette())
                .action(copy())
                .action(MessageAction::new("retry", Icon::Refresh, "Retry").disabled("Still running"))
                .action(MessageAction::new("edit", Icon::Edit, "Edit"))
                .overflow(vec![DropdownMenuItem::new("plain", "Copy as plain text").into()], false, "More")
                .visibility(MessageActionsVisibility::Hover),
        );
        tree.set_reduced_motion(true);
        let hidden = tree.layout(SIZE);
        assert_eq!(icon_alpha(&hidden, Icon::Copy), Some(0));
        assert!(button(&hidden, "retry").is_none(), "a disabled action is passive");
        let mut order = Vec::new();
        for _ in 0..3 {
            tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false });
            order.push(tree.focused_id().unwrap().to_owned());
            assert!(icon_alpha(&tree.layout(SIZE), Icon::Copy).unwrap() > 0);
        }
        assert_eq!(order, ["copy", "edit", "m:more"]);
        // A click elsewhere takes the keyboard focus away, and the row with it.
        tree.handle_event(UiEvent::PointerDown(Point::new(300.0, 150.0)));
        tree.handle_event(UiEvent::PointerUp(Point::new(300.0, 150.0)));
        assert_eq!(icon_alpha(&tree.layout(SIZE), Icon::Copy), Some(0));
    }

    #[test]
    fn a_disabled_action_explains_itself() {
        let mut tree = tree(
            MessageActions::new("m", palette())
                .action(MessageAction::new("edit", Icon::Edit, "Edit").disabled("An agent is running")),
        );
        tree.set_reduced_motion(true);
        tree.layout(SIZE);
        tree.handle_event(UiEvent::PointerMove(Point::new(12.0, 72.0)));
        let start = Instant::now();
        tree.layout_at(SIZE, start);
        let hinted = tree.layout_at(SIZE, start + Duration::from_secs(2));
        let texts: Vec<String> = texts(&hinted).into_iter().map(|(_, text)| text).collect();
        assert!(texts.contains(&"Edit".to_owned()) && texts.contains(&"An agent is running".to_owned()), "{texts:?}");
    }

    #[test]
    fn done_shows_a_check_for_its_time_and_then_asks_for_nothing() {
        let now = Instant::now();
        let done = MessageActionDone::new("copy", now);
        assert!(done.shows("copy", now) && !done.shows("edit", now));
        assert_eq!(done.repaint_after(now), Some(MessageActionDone::SHOWN_FOR));
        let later = now + Duration::from_millis(1000);
        assert_eq!(done.repaint_after(later), Some(Duration::from_millis(500)));
        let over = now + MessageActionDone::SHOWN_FOR;
        assert!(!done.shows("copy", over));
        assert_eq!(done.repaint_after(over), None);

        let mut tree = tree(MessageActions::new("m", palette()).action(copy().done(true)));
        tree.set_reduced_motion(true);
        let frame = tree.layout(SIZE);
        assert_eq!(icon_alpha(&frame, Icon::Copy), None);
        assert_eq!(icon_alpha(&frame, Icon::Check), Some(palette().text.a));
        assert_eq!(button(&frame, "copy"), Some(Rect::new(0.0, 60.0, 24.0, 24.0)), "still the same control");
    }

    #[test]
    fn a_toggle_that_is_on_is_filled() {
        let fill = |active: bool| {
            let mut tree = tree(
                MessageActions::new("m", palette())
                    .action(MessageAction::new("pin", Icon::Pin, "Pin").active(active).dispatch_as("toggle-pin")),
            );
            tree.set_reduced_motion(true);
            let frame = tree.layout(SIZE);
            let filled = frame.commands.iter().any(|command| {
                matches!(command, DrawCommand::FillRect { rect, color, .. }
                    if *rect == Rect::new(0.0, 60.0, 24.0, 24.0) && *color == palette().selected)
            });
            (filled, click(&mut tree, Rect::new(0.0, 60.0, 24.0, 24.0)))
        };
        assert!(!fill(false).0);
        let (filled, actions) = fill(true);
        assert!(filled);
        assert_eq!(actions, vec![UiAction::Clicked { id: "toggle-pin".into() }]);
    }

    #[test]
    fn the_overflow_menu_opens_under_its_trigger_and_holds_the_row() {
        let actions = |open: bool| {
            MessageActions::new("m", palette())
                .action(copy())
                .overflow(
                    vec![
                        DropdownMenuItem::new("plain", "Copy as plain text").into(),
                        DropdownMenuItem::new("activity", "Show activity").into(),
                    ],
                    open,
                    "More",
                )
                .overflow(Vec::new(), true, "Nothing")
                .visibility(MessageActionsVisibility::Hover)
        };
        let mut closed = tree(actions(false));
        closed.set_reduced_motion(true);
        let frame = closed.layout(SIZE);
        let trigger = button(&frame, "m:more").unwrap();
        assert_eq!(trigger, Rect::new(26.0, 60.0, 24.0, 24.0));
        assert!(button(&frame, "plain").is_none());
        assert_eq!(click(&mut closed, trigger), vec![UiAction::Clicked { id: "m:more".into() }]);

        // Open with the pointer nowhere near: the row stays painted.
        let mut open = tree(actions(true));
        open.set_reduced_motion(true);
        let frame = open.layout(SIZE);
        assert!(icon_alpha(&frame, Icon::Copy).unwrap() > 0);
        let item = button(&frame, "plain").unwrap();
        assert!(item.y >= trigger.y + trigger.height);
        assert_eq!(click(&mut open, item), vec![UiAction::Clicked { id: "plain".into() }]);
        let outside = Point::new(390.0, 190.0);
        open.handle_event(UiEvent::PointerDown(outside));
        assert_eq!(
            open.handle_event(UiEvent::PointerUp(outside)),
            vec![UiAction::Clicked { id: "m:more:dismiss".into() }]
        );
    }

    #[test]
    fn hidden_and_empty_rows_keep_their_room_and_nothing_else() {
        for actions in [
            MessageActions::new("m", palette()).action(copy()).visibility(MessageActionsVisibility::Hidden),
            MessageActions::new("m", palette()),
        ] {
            let below = Element::button("below", "Below").size(None, Some(20.0));
            let mut tree = UiTree::new(Element::column(vec![actions.render(), below, Element::spacer().flex(1.0)]));
            let frame = tree.layout(SIZE);
            assert_eq!(button(&frame, "below").unwrap().y, 24.0);
            assert!(button(&frame, "copy").is_none());
            assert_eq!(icon_alpha(&frame, Icon::Copy), None);
        }
    }

    #[test]
    fn a_narrow_row_cuts_its_caption_instead_of_escaping() {
        let mut tree = tree(
            MessageActions::new("m", palette()).action(copy()).caption("claude-opus-with-a-very-long-name"),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(90.0, 200.0));
        let (position, text) = texts(&frame).remove(0);
        assert!(text.ends_with('…'), "{text}");
        assert!(position.x + text.chars().count() as f32 * 6.0 <= 90.0);
    }
}
