//! A floating tab capsule whose player joins the collapsed row.

use crate::{
    Access, Component, ComponentPalette, Element, GlassMaterial, Icon, Role, ScalarSpring,
    SpringMotion,
};
use web_time::Instant;

/// One tab in a [`FloatingGlassBar`]. The action remains the same in both layouts.
#[derive(Clone, Debug)]
pub struct FloatingGlassTab {
    id: String,
    label: String,
    icon: Icon,
    selected: bool,
}

impl FloatingGlassTab {
    /// A tab reporting `id` with its visible label and icon.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            selected: false,
        }
    }

    /// Whether this is the selected tab, retained as the compact icon.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

/// The host's interruptible expanded-to-inline spring. Keep it across view
/// rebuilds and repaint while [`Self::is_active`] returns true.
#[derive(Clone, Copy, Debug)]
pub struct FloatingBarMotion {
    spring: ScalarSpring,
    scroll: f32,
    threshold: f32,
}

impl FloatingBarMotion {
    /// Start expanded, or already collapsed, without an entrance animation.
    pub fn new(collapsed: bool, now: Instant) -> Self {
        let mut motion = SpringMotion::settling_in(web_time::Duration::from_millis(u64::from(
            crate::tokens().motion.move_ms,
        )));
        motion.damping_ratio = 0.82;
        Self {
            spring: ScalarSpring::new(if collapsed { 1.0 } else { 0.0 }, now).motion(motion),
            scroll: 0.0,
            threshold: 16.0,
        }
    }

    /// Follow the host's scroll direction. Reduced motion lands immediately;
    /// reversing direction keeps the spring's current position and velocity.
    pub fn set_collapsed(&mut self, collapsed: bool, now: Instant, reduced_motion: bool) {
        self.spring
            .retarget_with_motion(if collapsed { 1.0 } else { 0.0 }, now, reduced_motion);
    }

    /// Set how many logical pixels of consistent scrolling change the layout.
    /// Finite values are kept at least one; the default is 16 points.
    pub fn scroll_threshold(mut self, threshold: f32) -> Self {
        if threshold.is_finite() {
            self.threshold = threshold.max(1.0);
        }
        self
    }

    /// Observe a change in content offset, positive when scrolling down.
    /// Direction reversal resets accumulated travel; small jitter does not
    /// repeatedly fold the bar. This observes scrolling without consuming it.
    pub fn scroll_by(&mut self, delta: f32, now: Instant, reduced_motion: bool) {
        if !delta.is_finite() || delta == 0.0 {
            return;
        }
        if self.scroll.signum() != delta.signum() {
            self.scroll = 0.0;
        }
        self.scroll += delta;
        if self.scroll.abs() >= self.threshold {
            self.set_collapsed(self.scroll > 0.0, now, reduced_motion);
            self.scroll = 0.0;
        }
    }

    /// The collapsed share for [`FloatingGlassBar::collapse`], bounded to 0..=1.
    pub fn value(&self, now: Instant) -> f32 {
        self.spring.value(now).clamp(0.0, 1.0)
    }

    /// Whether another frame is needed to settle the layout.
    pub fn is_active(&self, now: Instant) -> bool {
        self.spring.is_active(now)
    }
}

/// A tab capsule beside a separate search circle. An optional player above
/// the expanded tabs moves into the same row as they collapse. The host owns
/// scroll intent, selection and playback actions; this component owns geometry.
/// Supply the same captured page to the glass renderer for every part.
pub struct FloatingGlassBar {
    id: String,
    tabs: Vec<FloatingGlassTab>,
    search: Element,
    player: Option<Element>,
    material: GlassMaterial,
    palette: ComponentPalette,
    width: f32,
    collapse: f32,
}

impl FloatingGlassBar {
    /// A 440-point bar. `search` is the host's labeled control; IDs created
    /// for decoration and motion are derived from `id`.
    pub fn new(
        id: impl Into<String>,
        tabs: Vec<FloatingGlassTab>,
        search: Element,
        material: GlassMaterial,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            tabs,
            search,
            player: None,
            material,
            palette,
            width: 440.0,
            collapse: 0.0,
        }
    }

    /// The same player content in both layouts. Its controls keep their IDs
    /// and state while its capsule moves from above the tabs into their row.
    pub fn player(mut self, player: Element) -> Self {
        self.player = Some(player);
        self
    }

    /// Available width in logical pixels, at least 200. Non-finite input is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(200.0);
        }
        self
    }

    /// Collapsed share from 0 (two rows) to 1 (active icon, player, search).
    /// Non-finite input is ignored. Use [`FloatingBarMotion`] for scroll changes.
    pub fn collapse(mut self, progress: f32) -> Self {
        if progress.is_finite() {
            self.collapse = progress.clamp(0.0, 1.0);
        }
        self
    }
}

impl Component for FloatingGlassBar {
    fn render(self) -> Element {
        let p = self.collapse;
        let height = 56.0;
        let gap = 8.0;
        let padding = 4.0;
        let has_player = self.player.is_some();
        let expanded_tab_width = self.width - height - gap;
        let tab_width = expanded_tab_width + (height - expanded_tab_width) * p;
        let count = self.tabs.len().max(1) as f32;
        let rest_cell = (expanded_tab_width - 2.0 * padding) / count;
        let selected_id = self
            .tabs
            .iter()
            .find(|tab| tab.selected)
            .map(|tab| tab.id.clone());
        let action_ids = self
            .tabs
            .iter()
            .filter(|tab| p < 1.0 || tab.selected)
            .map(|tab| tab.id.clone())
            .collect();
        let mut children = Vec::new();
        for tab in self.tabs {
            let target = if tab.selected {
                height - 2.0 * padding
            } else {
                0.0
            };
            let width = rest_cell + (target - rest_cell) * p;
            if width <= 0.01 {
                continue;
            }
            let label_height = 12.0 * (1.0 - p);
            let ink = if tab.selected {
                self.palette.text
            } else {
                self.palette.muted
            };
            let copy = Element::column(vec![
                Element::icon(tab.icon)
                    .large_icon(24.0)
                    .foreground(ink)
                    .centered_x(),
                Element::panel(
                    Element::text(&tab.label)
                        .font_size(10.0)
                        .foreground(ink)
                        .centered_x(),
                )
                .size(None, Some(label_height))
                .clip_children(),
            ])
            .gap(2.0 * (1.0 - p))
            .centered();
            let cell = if p >= 1.0 || tab.selected || p < 0.9 {
                Element::clickable(&tab.id, copy).accessible(
                    Access::new(Role::Tab)
                        .name(&tab.label)
                        .selected(tab.selected),
                )
            } else {
                Element::panel(copy)
            };
            children.push(
                cell.size(Some(width), Some(height - 2.0 * padding))
                    .round()
                    .clip_children()
                    .indicator_slot(&tab.id),
            );
        }
        let mut tabs = Element::tab_strip(children, action_ids);
        if let Some(selected) = selected_id {
            tabs = tabs
                .tab_selection(
                    format!("{}:selection", self.id),
                    selected,
                    self.palette.selected,
                    height / 2.0,
                    crate::tokens().motion.move_ms,
                )
                .glass_selection(
                    GlassMaterial::new(crate::Color {
                        a: 0,
                        ..self.palette.selected
                    })
                    .blur(0.0)
                    .magnify(1.1)
                    .bezel(6.0),
                );
        }
        let y = if has_player {
            (height + gap) * (1.0 - p)
        } else {
            0.0
        };
        let total_height = height + y;
        // Unclipped: the lens stands past the capsule while it travels.
        let tabs = Element::panel(tabs.padding(padding))
            .glass(self.material)
            .round()
            .block_pointer(format!("{}:tabs", self.id))
            .size(Some(tab_width), Some(height));
        let place = |surface: Element, x: f32, y: f32| {
            Element::column(vec![
                Element::spacer().size(None, Some(y)),
                Element::row(vec![Element::spacer().size(Some(x), None), surface]),
            ])
        };
        let tabs = place(tabs, 0.0, y);
        let search = Element::panel(self.search.centered())
            .glass(self.material)
            .round()
            .clip_children()
            .size(Some(height), Some(height));
        let search = place(search, self.width - height, y);
        let mut surfaces = vec![tabs, search];
        if let Some(player) = self.player {
            let compact_width = self.width - 2.0 * (height + gap);
            let width = self.width + (compact_width - self.width) * p;
            let player = Element::panel(player)
                .glass(self.material)
                .round()
                .clip_children()
                .block_pointer(format!("{}:player", self.id))
                .size(Some(width), Some(height));
            surfaces.insert(0, place(player, (height + gap) * p, 0.0));
        }
        Element::stack(surfaces).size(Some(self.width), Some(total_height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Size, UiTree};

    #[test]
    fn interrupted_collapse_is_continuous_and_reduced_motion_lands() {
        let now = Instant::now();
        let mut motion = FloatingBarMotion::new(false, now);
        motion.set_collapsed(true, now, false);
        let midway = now + web_time::Duration::from_millis(60);
        let before = motion.value(midway);
        assert!(before > 0.0 && before < 1.0);
        motion.set_collapsed(false, midway, false);
        assert_eq!(motion.value(midway), before);
        motion.set_collapsed(true, midway, true);
        assert_eq!(motion.value(midway), 1.0);
        assert!(!motion.is_active(midway));
    }

    #[test]
    fn collapse_keeps_player_and_search_actions_and_shrinks_the_tabs() {
        let palette = ComponentPalette::neutral_dark();
        let view = |p| {
            FloatingGlassBar::new(
                "music",
                vec![
                    FloatingGlassTab::new("home", "Home", Icon::Home),
                    FloatingGlassTab::new("library", "Library", Icon::Library).selected(true),
                ],
                Element::button("search", "Search"),
                GlassMaterial::new(palette.surface),
                palette,
            )
            .player(Element::button("play", "Play"))
            .collapse(p)
            .render()
        };
        let mut expanded = UiTree::new(view(0.0));
        let mut collapsed = UiTree::new(view(1.0));
        let size = Size::new(440.0, 200.0);
        let a = expanded.layout(size);
        let b = collapsed.layout(size);
        for id in ["play", "search", "library"] {
            assert!(a.regions.iter().any(|region| region.id == id));
            assert!(b.regions.iter().any(|region| region.id == id));
        }
        assert!(!b.regions.iter().any(|region| region.id == "home"));
        let glass = |frame: &crate::Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Glass { rect, .. } => Some(*rect),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(glass(&a).len(), 3);
        assert_eq!(glass(&b).len(), 3);
        assert!(glass(&a).iter().any(|rect| rect.y >= 60.0));
        assert!(glass(&b).iter().all(|rect| rect.y < 1.0));
        assert!(glass(&a).iter().any(|rect| (rect.width - 56.0).abs() < 0.1));
        let mut widths = glass(&b)
            .iter()
            .map(|rect| rect.width.round() as u32)
            .collect::<Vec<_>>();
        widths.sort();
        assert_eq!(widths, vec![56, 56, 312]);
        assert!(glass(&b).iter().all(|rect| rect.x + rect.width <= 440.1));
        // The chosen tab carries the lens, which needs its cell found.
        assert!(a.commands.iter().any(|command| matches!(command, DrawCommand::GlassSelection { .. })));
    }

    #[test]
    fn scroll_direction_uses_a_threshold_and_reversal_drops_old_travel() {
        let now = Instant::now();
        let mut motion = FloatingBarMotion::new(false, now).scroll_threshold(12.0);
        motion.scroll_by(8.0, now, true);
        assert_eq!(motion.value(now), 0.0);
        motion.scroll_by(-4.0, now, true);
        motion.scroll_by(8.0, now, true);
        assert_eq!(motion.value(now), 0.0);
        motion.scroll_by(4.0, now, true);
        assert_eq!(motion.value(now), 1.0);
        motion.scroll_by(-12.0, now, true);
        assert_eq!(motion.value(now), 0.0);
        motion.scroll_by(f32::NAN, now, true);
        assert_eq!(motion.value(now), 0.0);
    }

    #[test]
    fn invalid_geometry_preserves_valid_defaults() {
        let palette = ComponentPalette::neutral_dark();
        let bar = FloatingGlassBar::new(
            "b",
            vec![],
            Element::spacer(),
            GlassMaterial::new(palette.surface),
            palette,
        )
        .width(f32::NAN)
        .collapse(f32::INFINITY);
        assert_eq!(bar.width, 440.0);
        assert_eq!(bar.collapse, 0.0);
    }
}
