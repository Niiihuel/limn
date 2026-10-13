//! A floating desktop bar with workspaces, a clock and status actions.

use crate::{
    Color, Component, ComponentPalette, Element, FontFamily, GlassMaterial, Icon, Point,
    TooltipPlacement, TooltipSpec,
};

const CONTROL: f32 = 40.0;
const PADDING: f32 = 12.0;
const GROUP_GAP: f32 = 16.0;
const DOT: f32 = 6.0;
const WORKSPACE: f32 = 24.0;

/// A workspace whose selection and occupied state the host supplies.
/// `action_id` is reported through [`crate::UiAction::Clicked`].
pub struct NavigationWorkspace {
    action_id: String,
    label: String,
    selected: bool,
    occupied: bool,
    disabled: bool,
}

impl NavigationWorkspace {
    /// A workspace dot with a descriptive tooltip.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            selected: false,
            occupied: false,
            disabled: false,
        }
    }

    /// Show the shared capsule on this workspace. The first selected one wins.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Give a workspace containing windows stronger ink.
    pub fn occupied(mut self, occupied: bool) -> Self {
        self.occupied = occupied;
        self
    }

    /// Leave its dot and tooltip visible without a pointer or keyboard action.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Enable or disable it; the opposite of [`disabled`](Self::disabled).
    pub fn enabled(self, enabled: bool) -> Self {
        self.disabled(!enabled)
    }
}

/// A status icon that reports an action, optionally followed by a short value.
/// Supply values and active states on every rebuild; this does no system I/O.
pub struct NavigationAction {
    action_id: String,
    icon: Icon,
    label: String,
    value: Option<String>,
    active: bool,
    disabled: bool,
}

impl NavigationAction {
    /// An icon button with this tooltip and reported action ID.
    pub fn new(action_id: impl Into<String>, icon: Icon, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            icon,
            label: label.into(),
            value: None,
            active: false,
            disabled: false,
        }
    }

    /// Short status text such as `100%`. Compact bars retain it in the tooltip.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into()).filter(|value| !value.is_empty());
        self
    }

    /// Give an open panel or enabled mode the accent color and selected fill.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Render a passive status icon with a tooltip.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Enable or disable it; the opposite of [`disabled`](Self::disabled).
    pub fn enabled(self, enabled: bool) -> Self {
        self.disabled(!enabled)
    }

    fn width(&self, compact: bool) -> f32 {
        CONTROL
            + if compact {
                0.0
            } else {
                self.value.as_ref().map_or(0.0, |value| {
                    (value.chars().count() as f32 * 8.0).min(96.0) + 4.0
                })
            }
    }

    fn render(
        self,
        bar_id: &str,
        compact: bool,
        accent: Color,
        palette: ComponentPalette,
    ) -> Element {
        let width = self.width(compact);
        let id = format!("{bar_id}:action:{}", self.action_id);
        let ink = if self.disabled {
            palette.muted
        } else if self.active {
            accent
        } else {
            palette.text
        };
        let mut content = vec![Element::icon(self.icon)
            .foreground(ink)
            .size(Some(20.0), Some(20.0))];
        if !compact {
            if let Some(value) = &self.value {
                content.push(
                    Element::truncated_text(value)
                        .font_family(FontFamily::Sans)
                        .font_size(15.0)
                        .foreground(ink)
                        .flex(1.0),
                );
            }
        }
        let content = Element::row(content).gap(4.0).centered_x();
        let control = if self.disabled {
            Element::panel(content)
        } else {
            Element::clickable(&id, content)
                .dispatch_as(self.action_id)
                .hover_background(palette.hover)
                .pressed_background(palette.selected)
                .transition_ms(crate::tokens().motion.hover_ms)
        }
        .padding(10.0)
        .radius(CONTROL / 2.0)
        .size(Some(width), Some(CONTROL));
        let control = if self.active {
            control.background(palette.selected)
        } else {
            control
        };
        let hint = match self.value {
            Some(value) => TooltipSpec::new(self.label).description(value),
            None => TooltipSpec::new(self.label),
        };
        hint.placement(TooltipPlacement::Below)
            .wrap(control, id, palette)
    }
}

/// A pill-shaped desktop bar composed of workspace dots, a two-line clock
/// and status actions. The host owns all values and handles reported actions.
/// Pass the available width on resize; narrow bars omit secondary text and
/// scroll horizontally, including keyboard focus, so every control is kept.
pub struct NavigationBar {
    id: String,
    palette: ComponentPalette,
    workspaces: Vec<NavigationWorkspace>,
    clock: Option<(String, String)>,
    clock_action: Option<String>,
    actions: Vec<NavigationAction>,
    width: Option<f32>,
    glass: Option<GlassMaterial>,
    accent: Color,
}

impl NavigationBar {
    /// Outer height in logical pixels.
    pub const HEIGHT: f32 = CONTROL + PADDING * 2.0;

    /// An empty bar. All retained control and animation IDs derive from `id`.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            palette,
            workspaces: Vec::new(),
            clock: None,
            clock_action: None,
            actions: Vec::new(),
            width: None,
            glass: None,
            accent: palette.text,
        }
    }

    /// Workspace dots in display order; action IDs must be unique in the group.
    pub fn workspaces(mut self, workspaces: Vec<NavigationWorkspace>) -> Self {
        self.workspaces = workspaces;
        self
    }

    /// Text from the host's clock and locale; no timer or date formatting is run.
    pub fn clock(mut self, time: impl Into<String>, date: impl Into<String>) -> Self {
        self.clock = Some((time.into(), date.into()));
        self
    }

    /// Make the clock a calendar trigger reporting this action ID.
    pub fn clock_action(mut self, action_id: impl Into<String>) -> Self {
        self.clock_action = Some(action_id.into());
        self
    }

    /// Status controls in display order; action IDs must be unique in the group.
    pub fn actions(mut self, actions: Vec<NavigationAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Available outer width. Non-finite input is ignored; negatives become zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Replace the opaque palette surface with a backdrop material.
    pub fn glass(mut self, material: GlassMaterial) -> Self {
        self.glass = Some(material.normalized());
        self
    }

    /// Ink for the selected workspace and active status actions.
    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self
    }

    fn workspace_strip(&self) -> Element {
        let selected = self
            .workspaces
            .iter()
            .position(|workspace| workspace.selected);
        let mut markers = Vec::new();
        let mut controls = Vec::new();
        let mut enabled = Vec::new();
        let mut selected_id = None;
        for (index, workspace) in self.workspaces.iter().enumerate() {
            let id = format!("{}:workspace:{}", self.id, workspace.action_id);
            let active = Some(index) == selected;
            let width = if active { WORKSPACE + 16.0 } else { WORKSPACE };
            let marker_width = if active { WORKSPACE } else { DOT };
            let ink = if workspace.occupied && !workspace.disabled {
                self.palette.muted
            } else {
                self.palette.border
            };
            // A separate paint strip gives the spring a thin capsule target,
            // while transparent controls keep a comfortable full-height target.
            markers.push(Element::spacer().size(Some((width - marker_width) / 2.0), Some(DOT)));
            markers.push(
                Element::spacer()
                    .size(Some(marker_width), Some(DOT))
                    .radius(DOT / 2.0)
                    .background(if active { Color::TRANSPARENT } else { ink })
                    .indicator_slot(&id),
            );
            markers.push(Element::spacer().size(Some((width - marker_width) / 2.0), Some(DOT)));
            let control = if workspace.disabled {
                Element::panel(Element::spacer())
            } else {
                enabled.push(id.clone());
                Element::clickable(&id, Element::spacer())
                    .dispatch_as(&workspace.action_id)
                    .hover_background(self.palette.hover)
                    .pressed_background(self.palette.selected)
                    .transition_ms(crate::tokens().motion.hover_ms)
            }
            .size(Some(width), Some(CONTROL))
            .radius(CONTROL / 2.0);
            controls.push(
                TooltipSpec::new(&workspace.label)
                    .placement(TooltipPlacement::Below)
                    .wrap(control, id.clone(), self.palette),
            );
            if active {
                selected_id = Some(id);
            }
        }
        let mut markers = Element::tab_strip(markers, Vec::new()).size(None, Some(DOT));
        if let Some(selected_id) = selected_id {
            markers = markers.tab_selection(
                format!("{}:selection", self.id),
                selected_id,
                self.accent,
                DOT / 2.0,
                crate::tokens().motion.move_ms,
            );
        }
        let width =
            self.workspaces.len() as f32 * WORKSPACE + if selected.is_some() { 16.0 } else { 0.0 };
        Element::tab_strip(
            vec![
                Element::stack(vec![Element::row(controls), markers.centered_y()])
                    .size(Some(width), Some(CONTROL)),
            ],
            enabled,
        )
        .size(Some(width), Some(CONTROL))
    }
}

impl Component for NavigationBar {
    fn render(self) -> Element {
        let workspace_width = self.workspaces.len() as f32 * WORKSPACE
            + if self.workspaces.iter().any(|workspace| workspace.selected) {
                16.0
            } else {
                0.0
            };
        let groups = usize::from(!self.workspaces.is_empty())
            + usize::from(self.clock.is_some())
            + usize::from(!self.actions.is_empty());
        let separators = groups.saturating_sub(1) as f32 * (GROUP_GAP * 2.0 + 1.0);
        let action_width: f32 = self.actions.iter().map(|action| action.width(false)).sum();
        let natural_width = PADDING * 2.0
            + workspace_width
            + if self.clock.is_some() { 148.0 } else { 0.0 }
            + action_width
            + separators;
        let compact = self.width.is_some_and(|width| width < natural_width);
        let mut groups = Vec::new();
        if !self.workspaces.is_empty() {
            groups.push(self.workspace_strip());
        }
        if let Some((time, date)) = self.clock {
            let text = |value: String, size: f32, ink: Color| {
                Element::truncated_text(value)
                    .font_family(FontFamily::Sans)
                    .font_size(size)
                    .foreground(ink)
                    .centered_x()
            };
            let mut lines = vec![
                text(time, if compact { 18.0 } else { 22.0 }, self.palette.text)
                    .size(None, Some(if compact { CONTROL } else { 25.0 })),
            ];
            if !compact {
                lines.push(text(date.clone(), 10.0, self.palette.muted).size(None, Some(13.0)));
            }
            let content = Element::column(lines).gap(2.0);
            let id = format!("{}:clock", self.id);
            let clock = match self.clock_action {
                Some(action) => Element::clickable(&id, content)
                    .dispatch_as(action)
                    .hover_background(self.palette.hover)
                    .pressed_background(self.palette.selected)
                    .transition_ms(crate::tokens().motion.hover_ms),
                None => Element::panel(content),
            }
            .radius(crate::tokens().radius.control)
            .size(Some(if compact { 100.0 } else { 148.0 }), Some(CONTROL));
            groups.push(
                TooltipSpec::new(date)
                    .placement(TooltipPlacement::Below)
                    .wrap(clock, id, self.palette),
            );
        }
        if !self.actions.is_empty() {
            groups.push(Element::row(
                self.actions
                    .into_iter()
                    .map(|action| action.render(&self.id, compact, self.accent, self.palette))
                    .collect(),
            ));
        }
        let mut children = Vec::new();
        for group in groups {
            if !children.is_empty() {
                children.push(
                    Element::spacer()
                        .size(Some(1.0), Some(26.0))
                        .background(self.palette.border),
                );
            }
            children.push(group);
        }
        let content = Element::row(children).gap(GROUP_GAP).centered_x();
        let content = Element::horizontal_scroll(format!("{}:scroll", self.id), content)
            .drag_to_pan()
            .foreground(self.palette.muted)
            .size(None, Some(CONTROL));
        let bar = Element::panel(content)
            .padding(PADDING)
            .radius(Self::HEIGHT / 2.0)
            .shadow(self.palette.shadow, Point::new(0.0, 4.0), 16.0, 0.0)
            .size(
                Some(self.width.unwrap_or(natural_width)),
                Some(Self::HEIGHT),
            );
        match self.glass {
            // Apple's glass shadow: `0 10px 40px`, black at 20% under dark
            // glass and 10% under light glass.
            Some(material) => {
                let tint = material.tint;
                let dark = u16::from(tint.r) + u16::from(tint.g) + u16::from(tint.b) < 383;
                bar.glass(material).shadow(
                    Color::rgba(0, 0, 0, if dark { 51 } else { 26 }),
                    Point::new(0.0, 10.0),
                    40.0,
                    0.0,
                )
            }
            // Glass draws its own inner stroke; only the opaque bar is outlined.
            None => bar
                .background(self.palette.surface)
                .border(self.palette.border, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Rect, Size, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    fn bar(selected: usize, width: f32) -> NavigationBar {
        NavigationBar::new("desktop", ComponentPalette::neutral_dark())
            .workspaces(
                (0..5)
                    .map(|n| {
                        NavigationWorkspace::new(format!("workspace-{n}"), format!("Workspace {n}"))
                            .selected(n == selected)
                            .occupied(n < 2)
                            .disabled(n == 3)
                    })
                    .collect(),
            )
            .clock("11:06:46", "TUE 13 OCT")
            .clock_action("calendar")
            .actions(vec![
                NavigationAction::new("network", Icon::Globe, "Network").value("Connected"),
                NavigationAction::new("settings", Icon::Settings, "Settings"),
                NavigationAction::new("disabled", Icon::More, "Unavailable").disabled(true),
            ])
            .width(width)
    }

    fn root(bar: NavigationBar) -> Element {
        Element::column(vec![bar.render(), Element::spacer().flex(1.0)])
    }

    fn button(frame: &Frame, suffix: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|r| r.id == format!("desktop:{suffix}") && r.kind == HitKind::Button)
            .unwrap()
            .rect
    }

    #[test]
    fn pill_geometry_and_every_enabled_action_report_the_host_id() {
        let mut tree = UiTree::new(root(bar(4, 800.0)));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(800.0, 200.0));
        assert!(frame.commands.iter().any(|c| matches!(c, DrawCommand::FillRect { rect, radius, .. }
            if rect.width == 800.0 && rect.height == NavigationBar::HEIGHT && *radius == NavigationBar::HEIGHT / 2.0)));
        assert!(!frame.regions.iter().any(|r| r.kind == HitKind::Button
            && (r.id.ends_with("workspace-3") || r.id.ends_with("disabled"))));
        for (suffix, action) in [
            ("workspace:workspace-1", "workspace-1"),
            ("clock", "calendar"),
            ("action:network", "network"),
            ("action:settings", "settings"),
        ] {
            let rect = button(&frame, suffix);
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: action.into() }]
            );
        }
    }

    #[test]
    fn keyboard_workspace_navigation_skips_disabled_and_actions_accept_enter() {
        let mut tree = UiTree::new(root(bar(2, 800.0)));
        tree.layout(Size::new(800.0, 200.0));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        };
        assert!(tree.focus("desktop:workspace:workspace-2"));
        assert_eq!(
            tree.handle_event(key(Key::ArrowRight)),
            vec![UiAction::Clicked {
                id: "workspace-4".into()
            }]
        );
        assert!(tree.focus("desktop:action:settings"));
        assert_eq!(
            tree.handle_event(key(Key::Enter)),
            vec![UiAction::Clicked {
                id: "settings".into()
            }]
        );
    }

    #[test]
    fn narrow_bars_compact_and_clip_controls_and_bad_width_is_ignored() {
        assert_eq!(
            bar(0, 500.0)
                .width(f32::NAN)
                .width(f32::INFINITY)
                .render()
                .style
                .width,
            Some(500.0)
        );
        assert_eq!(bar(0, 500.0).width(-2.0).render().style.width, Some(0.0));
        for width in [0.0, 160.0, 320.0, 600.0, 1000.0] {
            let mut tree = UiTree::new(root(bar(0, width)));
            let frame = tree.layout(Size::new(width, 200.0));
            for region in frame.regions.iter().filter(|r| r.kind == HitKind::Button) {
                assert!(
                    region.rect.x >= 0.0 && region.rect.x + region.rect.width <= width + 0.01,
                    "{width}: {:?}",
                    region.rect
                );
            }
            if width <= 320.0 {
                assert!(!frame.commands.iter().any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "TUE 13 OCT" || text == "Connected")));
            }
        }
    }

    #[test]
    fn keyboard_focus_reveals_status_actions_beyond_a_narrow_viewport() {
        let mut tree = UiTree::new(root(bar(0, 160.0)));
        tree.set_reduced_motion(true);
        let size = Size::new(160.0, 200.0);
        let initial = tree.layout(size);
        assert!(!initial
            .regions
            .iter()
            .any(|r| r.id == "desktop:action:settings"));
        assert!(tree.focus("desktop:action:settings"));
        let frame = tree.layout(size);
        let rect = button(&frame, "action:settings");
        assert_eq!(rect.width, CONTROL);
        assert!(rect.x >= PADDING && rect.x + rect.width <= size.width - PADDING);
        assert!(tree.horizontal_scroll_offset("desktop:scroll") > 0.0);
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "settings".into()
            }]
        );
    }

    #[test]
    fn workspace_marker_springs_and_reduced_motion_snaps() {
        let size = Size::new(800.0, 200.0);
        let accent = Color::rgb(210, 150, 90);
        let render = |selected| root(bar(selected, 800.0).accent(accent));
        let marker = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|c| match c {
                    DrawCommand::FillRect { rect, color, .. }
                        if *color == accent && rect.height == 6.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let mut tree = UiTree::new(render(0));
        let now = Instant::now();
        let initial = marker(&tree.layout_at(size, now));
        tree.set_root(render(4));
        let first = marker(&tree.layout_at(size, now));
        let midway = marker(&tree.layout_at(size, now + Duration::from_millis(60)));
        let settled = marker(&tree.layout_at(size, now + Duration::from_secs(2)));
        assert_eq!(first.x, initial.x);
        assert!(midway.x > initial.x && midway.x < settled.x);
        tree.set_reduced_motion(true);
        tree.set_root(render(0));
        assert_eq!(
            marker(&tree.layout_at(size, now + Duration::from_secs(3))).x,
            initial.x
        );
    }

    #[test]
    fn glass_replaces_opaque_surface_and_tooltips_stay_available() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(root(
            bar(0, 800.0).glass(GlassMaterial::new(palette.surface)),
        ));
        tree.set_reduced_motion(true);
        let size = Size::new(800.0, 200.0);
        let now = Instant::now();
        let frame = tree.layout_at(size, now);
        assert!(frame.commands.iter().any(
            |c| matches!(c, DrawCommand::Glass { rect, .. } if rect.height == NavigationBar::HEIGHT)
        ));
        assert!(!frame.commands.iter().any(|c| matches!(c, DrawCommand::FillRect { rect, color, .. } if rect.height == NavigationBar::HEIGHT && *color == palette.surface)));
        let rect = button(&frame, "action:settings");
        tree.handle_event(UiEvent::PointerMove(Point::new(
            rect.x + 10.0,
            rect.y + 10.0,
        )));
        tree.layout_at(size, now);
        let hinted = tree.layout_at(size, now + Duration::from_secs(2));
        assert!(hinted
            .commands
            .iter()
            .any(|c| matches!(c, DrawCommand::Text { text, .. } if text == "Settings")));
    }
}
