//! Two arrows around a position: the step-through control of a sequence.

use crate::{
    page::px, Access, ButtonVariant, Component, ComponentPalette, ControlSize, Element,
    ElementKind, Icon, IconButton, Role, Text, TextRole, TooltipSpec,
};

/// Density of a [`Pager`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagerSizing {
    /// Space between each arrow and the label.
    pub gap: f32,
}

impl PagerSizing {
    /// Arrows 4 from the label.
    pub const DEFAULT: Self = Self { gap: 4.0 };
}

// One end of the pager: its action, whether it can be taken, and its hint.
struct End {
    id: String,
    enabled: bool,
    icon: Icon,
    tooltip: Option<TooltipSpec>,
}

/// A previous arrow, a muted label such as "3 / 12" and a next arrow. The
/// host owns the position: it writes the label and says which ends can be
/// taken. Each arrow reports a click on its ID; an end that cannot be taken
/// is passive and keeps its hint.
pub struct Pager {
    label: String,
    label_width: Option<f32>,
    label_tooltip: Option<(String, TooltipSpec)>,
    previous: Option<End>,
    next: Option<End>,
    sizing: PagerSizing,
    palette: ComponentPalette,
}

impl Pager {
    /// The label alone; add the arrows with [`Pager::previous`] and
    /// [`Pager::next`].
    pub fn new(label: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            label: label.into(),
            label_width: None,
            label_tooltip: None,
            previous: None,
            next: None,
            sizing: PagerSizing::DEFAULT,
            palette,
        }
    }

    /// The arrow before the label, pointing left.
    pub fn previous(mut self, id: impl Into<String>, enabled: bool) -> Self {
        self.previous = Some(End { id: id.into(), enabled, icon: Icon::ArrowLeft, tooltip: None });
        self
    }

    /// The arrow after the label, pointing right.
    pub fn next(mut self, id: impl Into<String>, enabled: bool) -> Self {
        self.next = Some(End { id: id.into(), enabled, icon: Icon::ArrowRight, tooltip: None });
        self
    }

    /// Other glyphs for the arrows already added, such as up and down for a
    /// sequence that runs down the page.
    pub fn icons(mut self, previous: Icon, next: Icon) -> Self {
        if let Some(end) = &mut self.previous {
            end.icon = previous;
        }
        if let Some(end) = &mut self.next {
            end.icon = next;
        }
        self
    }

    /// Hints for the arrows already added. A disabled arrow still shows its
    /// hint.
    pub fn tooltips(mut self, previous: TooltipSpec, next: TooltipSpec) -> Self {
        if let Some(end) = &mut self.previous {
            end.tooltip = Some(previous);
        }
        if let Some(end) = &mut self.next {
            end.tooltip = Some(next);
        }
        self
    }

    /// Hold the label in a slot of this width, so the arrows do not move as
    /// the position changes. Values that are not finite are ignored.
    pub fn label_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.label_width = Some(width.max(0.0));
        }
        self
    }

    /// A hint on the label, for a position spelled out in words. `id` must be
    /// unique in the window.
    pub fn label_tooltip(mut self, id: impl Into<String>, tooltip: TooltipSpec) -> Self {
        self.label_tooltip = Some((id.into(), tooltip));
        self
    }

    /// The space between the arrows and the label.
    pub fn sizing(mut self, sizing: PagerSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for Pager {
    fn render(self) -> Element {
        let palette = self.palette;
        let arrow = |end: End| {
            let button = IconButton::new(&end.id, end.icon, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .enabled(end.enabled);
            let button = match end.tooltip {
                Some(tooltip) => button.tooltip_with(tooltip),
                None => button,
            }
            .render();
            // An arrow alone says nothing: its hint's title names it.
            let mut access = Access::new(Role::Button);
            if let ElementKind::Tooltip { title, .. } = &button.kind {
                access = access.name(title);
            }
            if end.enabled {
                button.accessible(access)
            } else {
                button.accessible(access.id(end.id).disabled(true))
            }
        };
        let label = Text::new(self.label, palette)
            .role(TextRole::Small)
            .muted()
            .truncate()
            .render()
            .center_content();
        let label = match self.label_tooltip {
            Some((id, tooltip)) => tooltip.wrap(label, id, palette),
            None => label,
        };
        let mut children = Vec::with_capacity(3);
        children.extend(self.previous.map(arrow));
        children.push(label.centered_y().size(self.label_width, Some(ControlSize::Dense.height())));
        children.extend(self.next.map(arrow));
        Element::row(children).gap(px(self.sizing.gap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, HitKind, Point, Rect, Size, TooltipPlacement, UiAction, UiEvent, UiTree};

    fn tree(pager: Pager) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![pager.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        tree
    }

    fn button(frame: &crate::Frame, id: &str) -> Option<Rect> {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .map(|region| region.rect)
    }

    fn texts(frame: &crate::Frame) -> Vec<(Point, String)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, text, .. } => Some((*position, text.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn an_arrow_is_named_by_its_hint_and_a_disabled_end_is_still_there() {
        let palette = ComponentPalette::neutral_dark();
        let pager = Pager::new("1 / 12", palette)
            .previous("prev", false)
            .next("next", true)
            .tooltips(TooltipSpec::new("Previous page"), TooltipSpec::new("Next page"));
        let frame = tree(pager).layout(Size::new(400.0, 100.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));

        let (previous, next) = (node("prev"), node("next"));
        assert_eq!((previous.role, previous.name.as_str()), (Role::Button, "Previous page"));
        assert!(previous.disabled && !previous.actionable);
        assert_eq!((next.role, next.name.as_str()), (Role::Button, "Next page"));
        assert!(next.actionable && !next.disabled);
        // Without hints there is nothing to call them.
        let bare = tree(Pager::new("1", palette).previous("prev", false).next("next", true)).layout(Size::new(400.0, 100.0));
        assert_eq!(bare.access.iter().map(|node| (node.id.as_str(), node.name.as_str())).collect::<Vec<_>>(),
            [("prev", ""), ("next", "")]);
    }

    #[test]
    fn the_arrows_stand_around_a_centered_label_of_fixed_width() {
        let palette = ComponentPalette::neutral_dark();
        for (sizing, label) in [(PagerSizing::DEFAULT, "3 / 12"), (PagerSizing { gap: 0.0 }, "1/2")] {
            let pager = Pager::new(label, palette)
                .previous("prev", true)
                .next("next", true)
                .label_width(52.0)
                .sizing(sizing);
            let mut tree = tree(pager);
            let frame = tree.layout(Size::new(400.0, 100.0));
            let (previous, next) = (button(&frame, "prev").unwrap(), button(&frame, "next").unwrap());
            assert_eq!(previous, Rect::new(0.0, 0.0, 24.0, 24.0));
            assert_eq!(next, Rect::new(24.0 + 52.0 + sizing.gap * 2.0, 0.0, 24.0, 24.0));
            let (position, text) = texts(&frame).remove(0);
            assert_eq!(text, label);
            let width = label.chars().count() as f32 * 6.0;
            assert_eq!(position.x, 24.0 + sizing.gap + (52.0 - width) / 2.0);
            for (id, rect) in [("prev", previous), ("next", next)] {
                let center = Point::new(rect.x + 12.0, rect.y + 12.0);
                tree.handle_event(UiEvent::PointerDown(center));
                assert_eq!(tree.handle_event(UiEvent::PointerUp(center)), vec![UiAction::Clicked { id: id.into() }]);
            }
        }
    }

    #[test]
    fn a_disabled_end_is_passive_and_keeps_its_hint() {
        let palette = ComponentPalette::neutral_dark();
        let pager = Pager::new("1 / 12", palette)
            .previous("prev", false)
            .next("next", true)
            .icons(Icon::ArrowUp, Icon::ArrowDown)
            .tooltips(
                TooltipSpec::new("Previous page").placement(TooltipPlacement::Below),
                TooltipSpec::new("Next page").placement(TooltipPlacement::Below),
            )
            .label_width(40.0);
        let mut tree = tree(pager);
        let frame = tree.layout(Size::new(400.0, 100.0));
        assert!(button(&frame, "prev").is_none(), "a first page has no way back");
        assert!(button(&frame, "next").is_some());
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::ArrowUp, .. })));
        assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::ArrowDown, .. })));

        tree.handle_event(UiEvent::PointerMove(Point::new(12.0, 12.0)));
        let start = web_time::Instant::now();
        tree.layout_at(Size::new(400.0, 100.0), start);
        let hinted = tree.layout_at(Size::new(400.0, 100.0), start + std::time::Duration::from_secs(2));
        assert!(texts(&hinted).iter().any(|(_, text)| text == "Previous page"));
    }

    #[test]
    fn the_label_takes_a_hint_of_its_own() {
        let palette = ComponentPalette::neutral_dark();
        let pager = Pager::new("2/5", palette)
            .previous("prev", true)
            .next("next", true)
            .label_width(40.0)
            .label_tooltip("where", TooltipSpec::new("Change 2 of 5"));
        let mut tree = tree(pager);
        tree.layout(Size::new(400.0, 100.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(24.0 + 4.0 + 20.0, 12.0)));
        let start = web_time::Instant::now();
        tree.layout_at(Size::new(400.0, 100.0), start);
        let hinted = tree.layout_at(Size::new(400.0, 100.0), start + std::time::Duration::from_secs(2));
        assert!(texts(&hinted).iter().any(|(_, text)| text == "Change 2 of 5"));
    }

    #[test]
    fn a_long_label_is_cut_to_its_slot_and_bad_numbers_are_ignored() {
        let palette = ComponentPalette::neutral_dark();
        let pager = Pager::new("1000 / 20000", palette)
            .previous("prev", true)
            .next("next", true)
            .label_width(40.0)
            .label_width(f32::NAN)
            .sizing(PagerSizing { gap: f32::NAN });
        let rendered = Pager::new("1", palette).sizing(PagerSizing { gap: -3.0 }).render();
        assert_eq!(rendered.style.gap, 0.0);
        let mut tree = tree(pager);
        let frame = tree.layout(Size::new(400.0, 100.0));
        assert_eq!(button(&frame, "next").unwrap().x, 24.0 + 40.0);
        let (_, text) = texts(&frame).remove(0);
        assert!(text.ends_with('…') && text.chars().count() as f32 * 6.0 <= 40.0, "{text}");
    }
}
