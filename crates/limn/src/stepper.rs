//! A controlled navigation strip for short, ordered forms.

use crate::{Access, Component, ComponentPalette, Element, FontFamily, Icon, Role};

/// Diameter of a step's badge.
const BADGE: f32 = 24.0;
const GLYPH: f32 = 14.0;
/// Thickness of the rule between two badges, and its distance from each.
const RULE: f32 = 2.0;
const RULE_GAP: f32 = 4.0;
/// Shortest a rule gets across, in a row of steps, and down, in a column,
/// where it is what sets the steps apart.
const RULE_MIN: f32 = 16.0;
const RULE_MIN_DOWN: f32 = 28.0;
const COPY_GAP: f32 = 8.0;
const LABEL_SIZE: f32 = 12.0;
const LABEL_LINE: f32 = 16.0;

/// One step of a [`Stepper`]: its label, and what it reports when chosen.
pub struct Step {
    id: String,
    label: String,
    description: Option<String>,
    icon: Option<Icon>,
    enabled: bool,
}

impl Step {
    /// A step with this label. Its ID is reported as `Clicked` when it is chosen.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            icon: None,
            enabled: true,
        }
    }

    /// A line of muted text under the label.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A glyph in the badge instead of the step's number.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A disabled step cannot be chosen: it keeps its place, its badge and
    /// its rule, with a muted label, and takes no pointer or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

/// The steps of a short form: a round badge per step with its number or
/// glyph, a rule from each badge to the next, and the step's label under it.
/// The steps up to the active one have a filled badge, and the rules between
/// them are filled too; the ones still to come are quiet. Each step reports
/// a click on its ID, unless it is disabled; the host owns the active index.
pub struct Stepper {
    steps: Vec<Step>,
    active_index: usize,
    palette: ComponentPalette,
    width: Option<f32>,
    vertical: bool,
}

impl Stepper {
    /// The steps of a short form, with the one being filled in. Steps before
    /// it are done; steps after it are still to come.
    pub fn new(steps: Vec<Step>, active_index: usize, palette: ComponentPalette) -> Self {
        Self {
            steps,
            active_index,
            palette,
            width: None,
            vertical: false,
        }
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Stack the steps, each with its label and description beside its badge
    /// and the rule running down to the next.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// The stepper as an element. An empty list of steps renders nothing.
    pub fn render(self) -> Element {
        if self.steps.is_empty() {
            return Element::spacer();
        }
        let palette = self.palette;
        let vertical = self.vertical;
        let count = self.steps.len();
        let active_index = self.active_index.min(count - 1);
        // Rows stretch their children; this holds one at its own height.
        let middle = |child: Element| {
            Element::column(vec![
                Element::spacer().flex(1.0),
                child,
                Element::spacer().flex(1.0),
            ])
        };
        let mut items = Vec::with_capacity(count);
        for (index, step) in self.steps.into_iter().enumerate() {
            let last = index + 1 == count;
            let reached = index <= active_index;
            let enabled = step.enabled;
            let quiet = if enabled { palette.text } else { palette.muted };
            // A filled badge keeps its ink when disabled: muted would not read on it.
            let (fill, ink) = if reached {
                (palette.text, palette.surface)
            } else {
                (palette.selected, quiet)
            };
            let glyph = match step.icon {
                Some(icon) => Element::icon(icon)
                    .foreground(ink)
                    .size(Some(GLYPH), Some(GLYPH))
                    .centered_x(),
                None => Element::text((index + 1).to_string())
                    .foreground(ink)
                    .font_family(FontFamily::Sans)
                    .font_size(LABEL_SIZE)
                    .center_content(),
            };
            // The decoration IDs let the fills cross-fade when the step
            // changes; a step still to come lifts under the pointer.
            let badge = Element::panel(middle(glyph))
                .background(fill)
                .hover_background(if reached || !enabled { fill } else { palette.hover })
                .hover_surface(format!("{}:badge", step.id), enabled.then_some(step.id.as_str()))
                .transition_ms(crate::tokens().motion.move_ms)
                .radius(BADGE / 2.0)
                .size(Some(BADGE), Some(BADGE));
            // Filled once the step it leaves is done.
            let rule = (!last).then(|| {
                Element::panel(Element::spacer())
                    .background(if index < active_index { palette.text } else { palette.selected })
                    .hover_surface(format!("{}:rule", step.id), [] as [&str; 0])
                    .transition_ms(crate::tokens().motion.move_ms)
                    .radius(RULE / 2.0)
            });
            // The label names the step, not the number in its badge.
            let mut access = Access::new(Role::Button).name(&step.label).selected(index == active_index);
            if let Some(description) = &step.description {
                access = access.description(description);
            }
            let mut copy = vec![Element::text(step.label)
                .foreground(quiet)
                .font_family(FontFamily::Sans)
                .font_size(LABEL_SIZE)];
            if let Some(description) = step.description {
                copy.push(
                    Element::text(description)
                        .foreground(palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(10.0),
                );
            }
            let copy = Element::column(copy).gap(2.0);
            let content = if vertical {
                let mut rail = vec![badge];
                if let Some(rule) = rule {
                    rail.push(
                        rule.size(Some(RULE), None)
                            .centered_x()
                            .min_size(0.0, RULE_MIN_DOWN)
                            .flex(1.0),
                    );
                }
                // The label sits level with the badge's middle.
                let copy = Element::column(vec![
                    Element::spacer().size(None, Some((BADGE - LABEL_LINE) / 2.0)),
                    copy,
                ]);
                Element::row(vec![
                    Element::column(rail).gap(RULE_GAP).size(Some(BADGE), None),
                    copy.flex(1.0),
                ])
                .gap(COPY_GAP)
            } else {
                let head = match rule {
                    Some(rule) => Element::row(vec![
                        badge,
                        middle(rule.size(None, Some(RULE))).min_size(RULE_MIN, 0.0).flex(1.0),
                    ])
                    .gap(RULE_GAP)
                    .size(None, Some(BADGE)),
                    None => badge,
                };
                Element::column(vec![head, copy]).gap(COPY_GAP)
            };
            let item = if enabled {
                Element::clickable(step.id, content).accessible(access)
            } else {
                Element::panel(content).accessible(access.id(step.id).disabled(true))
            }
            .radius(crate::RadiusRole::Control.px());
            // Every step but the last takes an equal share, rule included.
            items.push(if vertical || last { item } else { item.flex(1.0) });
        }
        if vertical {
            Element::column(items).gap(RULE_GAP).size(self.width, None)
        } else {
            Element::row(items).gap(RULE_GAP).size(self.width, None)
        }
    }
}

impl Component for Stepper {
    fn render(self) -> Element {
        Stepper::render(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn steps() -> Vec<Step> {
        vec![
            Step::new("one", "Contenido").description("Define la tarea").icon(Icon::File),
            Step::new("two", "Horario").description("Elige cuándo"),
            Step::new("three", "Agente").description("Modelo y acceso"),
        ]
    }

    fn frame(stepper: Stepper, size: Size) -> Frame {
        let mut tree = UiTree::new(Element::column(vec![stepper.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(size)
    }

    /// The fills of the badges and of the rules, in the order they are painted.
    fn fills(frame: &Frame, badge: bool) -> Vec<(Rect, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if badge == (rect.width == BADGE && rect.height == BADGE)
                        && (badge || rect.width == RULE || rect.height == RULE) =>
                {
                    Some((*rect, *color))
                }
                _ => None,
            })
            .collect()
    }

    fn step(frame: &Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind == HitKind::Button)
            .unwrap()
            .rect
    }

    #[test]
    fn a_step_tells_the_platform_its_label_and_whether_it_is_the_active_one() {
        let mut steps = steps();
        let last = steps.pop().unwrap().disabled();
        steps.push(last);
        let frame = UiTree::new(Stepper::new(steps, 1, palette()).width(360.0).render()).layout(Size::new(360.0, 120.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));

        let (one, two, three) = (node("one"), node("two"), node("three"));
        assert_eq!((one.role, one.name.as_str(), one.selected), (Role::Button, "Contenido", Some(false)));
        assert_eq!(one.description.as_deref(), Some("Define la tarea"));
        assert_eq!((two.name.as_str(), two.selected), ("Horario", Some(true)));
        assert!(two.actionable && !two.disabled);
        assert_eq!((three.role, three.name.as_str(), three.selected), (Role::Button, "Agente", Some(false)));
        assert!(three.disabled && !three.actionable);
    }

    #[test]
    fn steps_up_to_the_active_one_are_filled_and_so_are_the_rules_between_them() {
        let size = Size::new(420.0, 120.0);
        let frame = frame(Stepper::new(steps(), 1, palette()).width(420.0), size);
        let badges: Vec<Color> = fills(&frame, true).into_iter().map(|(_, color)| color).collect();
        assert_eq!(badges, [palette().text, palette().text, palette().selected]);
        let rules = fills(&frame, false);
        assert_eq!(
            rules.iter().map(|(_, color)| *color).collect::<Vec<_>>(),
            [palette().text, palette().selected]
        );
        // A rule runs from its badge to the next step, level with the badges' middle.
        let (one, two) = (step(&frame, "one"), step(&frame, "two"));
        assert_eq!(rules[0].0.x, one.x + BADGE + RULE_GAP);
        assert_eq!(rules[0].0.x + rules[0].0.width + RULE_GAP, two.x);
        assert_eq!(rules[0].0.y, one.y + (BADGE - RULE) / 2.0);
        // The steps with a rule share the width; the last keeps its own.
        assert_eq!(one.width, two.width);
        assert!(step(&frame, "three").width < one.width);
        // The first badge shows its glyph, the others their number.
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::File, tint, .. } if *tint == palette().surface)));
        let texts: Vec<&str> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"2") && texts.contains(&"3") && !texts.contains(&"1"), "{texts:?}");
        // Every label is under its badge, from the same edge.
        let label_x = |label: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == label => Some(*position),
                _ => None,
            })
        };
        let label = label_x("Horario").unwrap();
        assert_eq!(label.x, two.x);
        assert!(label.y >= two.y + BADGE + COPY_GAP);
    }

    #[test]
    fn a_column_of_steps_runs_its_rules_down_and_sets_the_copy_beside_the_badges() {
        let size = Size::new(240.0, 300.0);
        let frame = frame(Stepper::new(steps(), 2, palette()).vertical().width(240.0), size);
        let badges = fills(&frame, true);
        assert!(badges.iter().all(|(rect, color)| rect.x == 0.0 && *color == palette().text));
        let rules = fills(&frame, false);
        assert_eq!(rules.len(), 2);
        for (index, (rule, color)) in rules.iter().enumerate() {
            assert_eq!(*color, palette().text);
            assert_eq!((rule.x, rule.width), ((BADGE - RULE) / 2.0, RULE));
            assert_eq!(rule.y, badges[index].0.y + BADGE + RULE_GAP);
            assert_eq!(rule.y + rule.height + RULE_GAP, badges[index + 1].0.y);
            assert!(rule.height >= RULE_MIN_DOWN);
        }
        let label = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "Horario" => Some(*position),
                _ => None,
            })
            .unwrap();
        assert_eq!(label.x, BADGE + COPY_GAP);
    }

    #[test]
    fn a_step_reports_its_click_from_its_badge_its_rule_or_its_label() {
        let size = Size::new(420.0, 120.0);
        let mut tree = UiTree::new(Stepper::new(steps(), 0, palette()).width(420.0).render());
        tree.set_reduced_motion(true);
        let frame = tree.layout(size);
        let two = step(&frame, "two");
        for point in [
            Point::new(two.x + 12.0, two.y + 12.0),
            Point::new(two.x + two.width - 20.0, two.y + 12.0),
            Point::new(two.x + 12.0, two.y + two.height - 4.0),
        ] {
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                [UiAction::Clicked { id: "two".into() }]
            );
        }
    }

    #[test]
    fn a_disabled_step_takes_no_pointer_or_focus_and_keeps_its_place() {
        let size = Size::new(420.0, 120.0);
        let stepper = |enabled| {
            let mut steps = steps();
            let two = steps.remove(1).enabled(enabled);
            steps.insert(1, two);
            Stepper::new(steps, 0, palette()).width(420.0)
        };
        let texts = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, color, .. } => Some((text.clone(), *position, *color)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let enabled = frame(stepper(true), size);
        let two = step(&enabled, "two");
        let mut tree = UiTree::new(Element::column(vec![stepper(false).render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let passive = tree.layout(size);
        assert!(!passive.regions.iter().any(|region| region.id == "two"));
        assert!(!tree.focus("two"));
        // Its badge, its rule and its label: none of them answers.
        for point in [
            Point::new(two.x + 12.0, two.y + 12.0),
            Point::new(two.x + two.width - 20.0, two.y + 12.0),
            Point::new(two.x + 12.0, two.y + two.height - 4.0),
        ] {
            tree.handle_event(UiEvent::PointerMove(point));
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        }
        assert!(tree.handle_event(UiEvent::Key { key: Key::Enter, shift: false, ctrl: false, alt: false }).is_empty());
        // The pointer is still over it: its badge does not lift.
        let hovered = tree.layout(size);
        assert_eq!(fills(&hovered, true), fills(&enabled, true));
        assert_eq!(fills(&hovered, false), fills(&enabled, false));
        // The others still answer, and nothing has moved.
        assert_eq!(step(&passive, "one"), step(&enabled, "one"));
        assert_eq!(step(&passive, "three"), step(&enabled, "three"));
        let (muted, plain) = (texts(&passive), texts(&enabled));
        let places = |texts: &[(String, Point, Color)]| texts.iter().map(|text| text.1).collect::<Vec<_>>();
        assert_eq!(places(&muted), places(&plain));
        for (text, _, color) in &muted {
            let own = text == "2" || text == "Horario" || text == "Elige cuándo";
            let was = plain.iter().find(|other| other.0 == *text).unwrap().2;
            assert_eq!(*color, if own { palette().muted } else { was }, "{text}");
        }
    }
}
