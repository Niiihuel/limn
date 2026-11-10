//! A teaching bubble that points at a control which already exists, and the
//! sequence of them that makes a guided tour.

use crate::element::CoachAnchor;
use crate::modal::DismissButton;
use crate::theme::mix;
use crate::{
    Button, ButtonVariant, CoachAlign, CoachSide, Color, Component, ComponentPalette, ControlSize,
    Element, Point, PopoverPlacement, Text, TextRole,
};

/// One step of a guided tour: a bubble with a caret towards its target and a
/// ring around it. The target is named by the ID of a control built anywhere
/// in `base`, so nothing changes where that control is built.
///
/// The host owns the step. The component reports `id` for the close button
/// and Escape, `{id}:next` for the primary action and the Right arrow,
/// `{id}:back` for the secondary action and the Left arrow, and `{id}:skip`;
/// `on_next`, `on_back` and `on_skip` name other actions.
/// While it shows, the rest of the window takes no input and a click outside
/// the bubble does nothing: a stray click neither loses the tour nor acts on
/// what is behind it.
///
/// When no control with the target's ID is laid out in view the component
/// paints nothing and [`Frame::missing_coach_target`](crate::Frame::missing_coach_target)
/// names the target, so the host can move to another step.
pub struct CoachMark {
    base: Element,
    id: String,
    targets: Vec<String>,
    palette: ComponentPalette,
    open: bool,
    title: String,
    body: String,
    illustration: Option<Element>,
    step: Option<(usize, usize)>,
    counter: Option<String>,
    side: CoachSide,
    align: CoachAlign,
    width: f32,
    accent: Option<(Color, Color)>,
    dim: Option<Color>,
    next_label: String,
    done_label: String,
    back_label: String,
    skip_label: Option<String>,
    key: Option<String>,
    next_action: Option<String>,
    back_action: Option<String>,
    skip_action: Option<String>,
}

impl CoachMark {
    /// A bubble below `target`, painted in the palette's solid role: the
    /// palette's ink as the fill and its surface as the ink, so it stands
    /// apart from menus and tooltips in a light and in a dark theme.
    pub fn new(
        base: Element,
        id: impl Into<String>,
        target: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            id: id.into(),
            targets: vec![target.into()],
            palette,
            open: true,
            title: String::new(),
            body: String::new(),
            illustration: None,
            step: None,
            counter: None,
            side: CoachSide::Below,
            align: CoachAlign::Start,
            width: 320.0,
            accent: None,
            dim: None,
            next_label: crate::labels().next.into(),
            done_label: crate::labels().done.into(),
            back_label: crate::labels().back.into(),
            skip_label: None,
            key: None,
            next_action: None,
            back_action: None,
            skip_action: None,
        }
    }

    /// Report `action` for the primary action instead of `{id}:next`.
    pub fn on_next(mut self, action: impl Into<String>) -> Self {
        self.next_action = Some(action.into());
        self
    }

    /// Report `action` for the secondary action instead of `{id}:back`.
    pub fn on_back(mut self, action: impl Into<String>) -> Self {
        self.back_action = Some(action.into());
        self
    }

    /// Report `action` for the skip action instead of `{id}:skip`.
    pub fn on_skip(mut self, action: impl Into<String>) -> Self {
        self.skip_action = Some(action.into());
        self
    }

    /// Render `base` alone while `false`.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Point at the area that `target` and this control cover together, such
    /// as a group of buttons.
    pub fn also(mut self, target: impl Into<String>) -> Self {
        self.targets.push(target.into());
        self
    }

    /// The bubble's heading, which is also the name of its dialog to the platform. Empty by
    /// default.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// One or two sentences. The text wraps; it is never cut.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    /// A picture above the title, as wide as the bubble's content.
    pub fn illustration(mut self, illustration: Element) -> Self {
        self.illustration = Some(illustration);
        self
    }

    /// The step's place in its tour, counted from zero. It shows "2 de 5",
    /// hides the secondary action on the first step and gives the primary
    /// action its last-step label on the last one.
    pub fn step(mut self, index: usize, count: usize) -> Self {
        self.step = Some((index.min(count.saturating_sub(1)), count.max(1)));
        self
    }

    /// The counter's text, in place of "2 de 5".
    pub fn counter(mut self, text: impl Into<String>) -> Self {
        self.counter = Some(text.into());
        self
    }

    /// The side of the target the bubble prefers and where it sits along
    /// that edge. It takes another side when this one has no room.
    pub fn placement(mut self, side: CoachSide, align: CoachAlign) -> Self {
        self.side = side;
        self.align = align;
        self
    }

    /// The bubble's width, 320 unless set and never under 220. A value that
    /// is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(220.0);
        }
        self
    }

    /// Paint the bubble, its caret and the ring in a statement color of the
    /// host, such as its brand color, with `ink` for the text on it.
    pub fn accent(mut self, fill: Color, ink: Color) -> Self {
        self.accent = Some((fill, ink));
        self
    }

    /// Dim the window around the target.
    pub fn dim(mut self, color: Color) -> Self {
        self.dim = Some(color);
        self
    }

    /// The primary action's label, "Siguiente" unless set.
    pub fn next_label(mut self, label: impl Into<String>) -> Self {
        self.next_label = label.into();
        self
    }

    /// The primary action's label on the last step, "Entendido" unless set.
    pub fn done_label(mut self, label: impl Into<String>) -> Self {
        self.done_label = label.into();
        self
    }

    /// The secondary action's label, "Atrás" unless set.
    pub fn back_label(mut self, label: impl Into<String>) -> Self {
        self.back_label = label.into();
        self
    }

    /// Offer to leave the whole tour, on every step but the last.
    pub fn skip(mut self, label: impl Into<String>) -> Self {
        self.skip_label = Some(label.into());
        self
    }

    /// Names the step, for a host that shows two steps on one target. The
    /// bubble travels and takes focus again when the key changes; by default
    /// the key is the target and the step's index.
    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }
}

impl Component for CoachMark {
    fn render(self) -> Element {
        if !self.open || self.targets.is_empty() {
            return self.base;
        }
        // A bubble must hide what it covers: an ink with transparency is
        // taken as it looks on the page.
        let opaque = |color: Color, behind: Color| {
            mix(behind, color, u32::from(color.a) * 100 / 255)
        };
        let (fill, ink) = match self.accent {
            Some((fill, ink)) => (opaque(fill, self.palette.surface), ink),
            None => (opaque(self.palette.text, self.palette.surface), self.palette.surface),
        };
        let ink = opaque(ink, fill);
        let bubble = ComponentPalette {
            surface: fill,
            raised: fill,
            hover: mix(fill, ink, 14),
            selected: mix(fill, ink, 22),
            border: fill,
            text: ink,
            muted: mix(fill, ink, 72),
            shadow: self.palette.shadow,
        };
        let (index, count) = self.step.unwrap_or((0, 1));
        let last = index + 1 == count;
        let next_id = format!("{}:next", self.id);
        let back_id = (index > 0).then(|| format!("{}:back", self.id));
        // The controls keep their own IDs, so focus stays on the primary
        // action from step to step whatever each step reports.
        let next_action = self.next_action.unwrap_or_else(|| next_id.clone());
        let back_action = back_id.as_ref().map(|id| self.back_action.unwrap_or_else(|| id.clone()));

        // Known by an ID of its own, as the close control is.
        let mut said = crate::Access::new(crate::Role::Dialog)
            .id(format!("{}:dialog", self.id))
            .name(self.title.clone());
        if !self.body.is_empty() {
            said = said.description(self.body.clone());
        }
        let mut rows = Vec::with_capacity(4);
        if let Some(illustration) = self.illustration {
            rows.push(illustration);
        }
        rows.push(
            Element::row(vec![
                Text::new(self.title, bubble).role(TextRole::Body).render().flex(1.0),
                DismissButton::new(format!("{}:close", self.id), bubble)
                    .dispatch_as(self.id.clone())
                    .render(),
            ])
            .gap(8.0),
        );
        if !self.body.is_empty() {
            rows.push(
                Text::new(self.body, bubble)
                    .ink(mix(fill, ink, 88))
                    .line_height(17.0)
                    .render(),
            );
        }
        let mut footer = Vec::with_capacity(5);
        let counter = self
            .counter
            .or_else(|| self.step.map(|_| format!("{} {} {count}", index + 1, crate::labels().of)));
        if let Some(counter) = counter {
            footer.push(Text::new(counter, bubble).muted().render().centered_y());
        }
        if let Some(label) = self.skip_label.filter(|_| !last) {
            footer.push(
                Button::new(format!("{}:skip", self.id), label, bubble)
                    .dispatch_as(self.skip_action.unwrap_or_else(|| format!("{}:skip", self.id)))
                    .variant(ButtonVariant::Link)
                    .size(ControlSize::Compact)
                    .ink(bubble.muted)
                    .render(),
            );
        }
        footer.push(Element::spacer().flex(1.0));
        if let (Some(id), Some(action)) = (back_id, &back_action) {
            footer.push(
                Button::new(id, self.back_label, bubble)
                    .dispatch_as(action.clone())
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Compact)
                    .render(),
            );
        }
        footer.push(
            Button::new(
                next_id.clone(),
                if last { self.done_label } else { self.next_label },
                // The bubble's own solid role: its ink as the fill.
                ComponentPalette {
                    surface: ink,
                    hover: ink,
                    selected: ink,
                    text: fill,
                    ..bubble
                },
            )
            .dispatch_as(next_action.clone())
            .variant(ButtonVariant::Solid)
            .size(ControlSize::Compact)
            .render()
            .center_content(),
        );
        rows.push(
            Element::row(footer)
                .gap(6.0)
                .size(None, Some(ControlSize::Compact.height())),
        );

        let card = Element::panel(Element::column(rows).gap(8.0))
            .padding(12.0)
            .size(Some(self.width), None)
            .background(fill)
            .radius(crate::tokens().radius.surface)
            .shadow(self.palette.shadow, Point::new(0.0, 9.0), 22.0, 0.0)
            .accessible(said);
        let key = self
            .key
            .unwrap_or_else(|| format!("{}:{index}", self.targets.join("+")));
        Element::popover(self.base, card, self.id, PopoverPlacement::BottomStart)
            // Transparent: the scope is modal, so focus stays in the bubble
            // and returns where it was, without painting over the window.
            .popover_scrim(Color::rgba(0, 0, 0, 0))
            .popover_backdrop_dismiss(false)
            .popover_initial_focus(next_id.clone())
            .popover_coach(CoachAnchor {
                targets: self.targets,
                side: self.side,
                align: self.align,
                key,
                fill,
                dim: self.dim,
                back: back_action,
                next: Some(next_action),
            })
    }
}

/// One step of a [`Tour`]: what it points at and what it says.
pub struct TourStep {
    targets: Vec<String>,
    title: String,
    body: String,
    side: CoachSide,
    align: CoachAlign,
    illustration: Option<Element>,
}

impl TourStep {
    /// A step below the control `target`.
    pub fn new(
        target: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            targets: vec![target.into()],
            title: title.into(),
            body: body.into(),
            side: CoachSide::Below,
            align: CoachAlign::Start,
            illustration: None,
        }
    }

    /// Point at the area that the step's controls cover together.
    pub fn also(mut self, target: impl Into<String>) -> Self {
        self.targets.push(target.into());
        self
    }

    /// The side of the target this step's bubble prefers and where it sits along that edge.
    /// A step sits below, at the start, by default.
    pub fn placement(mut self, side: CoachSide, align: CoachAlign) -> Self {
        self.side = side;
        self.align = align;
        self
    }

    /// A picture above this step's title, as wide as the bubble's content.
    pub fn illustration(mut self, illustration: Element) -> Self {
        self.illustration = Some(illustration);
        self
    }
}

/// A sequence of coach marks. The host owns `current` and moves it when it
/// receives `{id}:next` and `{id}:back`; `id` and `{id}:skip` end the tour,
/// and so does `{id}:next` on the last step. With `current` at `None` or
/// past the end it renders `base` alone.
pub struct Tour {
    base: Element,
    id: String,
    steps: Vec<TourStep>,
    current: Option<usize>,
    palette: ComponentPalette,
}

impl Tour {
    /// A tour over `base` that shows step `current` of `steps`. `id` prefixes the actions
    /// it reports, which the description of [`Tour`] lists.
    pub fn new(
        base: Element,
        id: impl Into<String>,
        steps: Vec<TourStep>,
        current: Option<usize>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            id: id.into(),
            steps,
            current,
            palette,
        }
    }

    /// The current step as a [`CoachMark`], to set what the whole tour
    /// shares: its labels, its accent, a dimmed window, a skip action.
    pub fn coach_mark(mut self) -> CoachMark {
        let count = self.steps.len();
        let Some(index) = self.current.filter(|index| *index < count) else {
            return CoachMark::new(self.base, self.id, "", self.palette).open(false);
        };
        let step = self.steps.swap_remove(index);
        let mut mark = CoachMark::new(self.base, self.id, "", self.palette)
            .title(step.title)
            .body(step.body)
            .placement(step.side, step.align)
            .step(index, count);
        mark.targets = step.targets;
        mark.illustration = step.illustration;
        mark
    }
}

impl Component for Tour {
    fn render(self) -> Element {
        self.coach_mark().render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Role, Size, UiTree};

    #[test]
    fn a_coach_mark_is_a_dialog_named_by_its_title_with_a_named_close_control() {
        let palette = ComponentPalette::neutral_dark();
        let base = Element::column(vec![Element::button("tasks", "Tareas").size(Some(80.0), Some(32.0))]);
        let mut tree = UiTree::new(
            CoachMark::new(base, "tour", "tasks", palette)
                .title("Tareas")
                .body("Cada tarea es una conversación.")
                .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(800.0, 600.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let dialog = node("tour:dialog");
        assert_eq!((dialog.role, dialog.name.as_str()), (Role::Dialog, "Tareas"));
        assert_eq!(dialog.description.as_deref(), Some("Cada tarea es una conversación."));
        assert!(!dialog.actionable);
        assert_eq!((node("tour:close").role, node("tour:close").name.as_str()), (Role::Button, crate::labels().close));
        assert_eq!(node("tour:next").name, crate::labels().done);
    }
}
