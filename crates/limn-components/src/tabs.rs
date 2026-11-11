//! A row of tabs with a selection that slides to the active one.

use crate::{control_radius, lighter, surface_radius, Access, Color, Component, ComponentPalette, ControlSize, ControlState, Element, FontFamily, Icon, Role};

/// One selectable tab. `action_id` is a stable ID handled by the host's
/// `UiAction::Clicked`; it can represent any domain value (file, view, task).
pub struct Tab {
    /// Reported in `UiAction::Clicked` when the tab is chosen. A disabled tab is
    /// known to the platform by it.
    pub action_id: String,
    /// The tab's text.
    pub label: String,
    /// An icon before the label. `None` by default.
    pub icon: Option<Icon>,
    /// Whether it is the chosen tab: it takes the accent of [`Tabs`], or the
    /// text color without one, and the sliding selection rests under it,
    /// unless it is disabled.
    pub active: bool,
    /// Width in logical pixels. `new` keeps it at 24 or more, the least
    /// target a pointer is given, and `Tabs::fill_width` replaces it.
    pub width: f32,
    /// `Disabled` keeps the tab in its place, in the muted color, and it cannot
    /// be chosen.
    pub state: ControlState,
}

impl Tab {
    /// A tab of this width that reports `Clicked` with this ID.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>, width: f32) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            active: false,
            // Never under 24, the least pointer target (WCAG 2.5.8), so an
            // icon alone can sit in a narrow segment; 40 when it is not a number.
            width: if width.is_finite() { width.max(24.0) } else { 40.0 },
            state: ControlState::Enabled,
        }
    }

    /// Show an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Mark this tab as the chosen one.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Disable the tab: it keeps its place and cannot be chosen.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled { ControlState::Enabled } else { ControlState::Disabled };
        self
    }
}

pub(crate) const TAB_GAP: f32 = 2.0;
pub(crate) const TAB_TRACK_PADDING: f32 = 2.0;
const TAB_ITEM_PADDING: f32 = 4.0;
const TAB_ICON_GAP: f32 = 4.0;
const TAB_FONT_SIZE: f32 = 11.5;
// The underline fills the track's bottom padding, so both variants keep one size.
const TAB_UNDERLINE: f32 = TAB_TRACK_PADDING;
const TAB_HAIRLINE: f32 = 1.0;

/// How [`Tabs`] mark the active tab. Both take the same size, report the same
/// actions and answer the same keys.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabsVariant {
    /// A filled selection on a rounded track: the palette's `selected` on its
    /// `surface`.
    #[default]
    Segmented,
    /// Text tabs with no track: a 2 px underline in the accent under the
    /// active tab, over a 1 px hairline in the palette's `border` as wide as
    /// the strip is laid out.
    Underlined,
}

/// A row of [`Tab`]s with a selection that slides to the active one.
pub struct Tabs {
    /// The tabs, in the order they are drawn.
    pub items: Vec<Tab>,
    /// `surface` fills the track and `selected` the sliding selection; `text` is
    /// the ink of the active tab unless [`accent`](Self::accent) is set, and
    /// `muted` of the others. Underlined, `border` draws the hairline and
    /// neither fill is used.
    pub palette: ComponentPalette,
    /// Height of a tab in logical pixels, 24 by default. The track is 4 taller.
    pub height: f32,
    /// Space between two tabs in logical pixels, 2 by default.
    pub gap: f32,
    /// Keys the selection's animation. `None`, the default, derives it from the
    /// first tab's `action_id`.
    pub id: Option<String>,
    /// How long the selection takes to slide to another tab, in milliseconds.
    /// The default is the `move_ms` token.
    pub animation_ms: u16,
    /// How the active tab is marked. [`TabsVariant::Segmented`] by default.
    pub variant: TabsVariant,
    /// Ink of the active tab, and color of the underline. `None`, the default,
    /// takes the palette's `text`.
    pub accent: Option<Color>,
    /// Corners of the selection, in logical pixels; the segmented track
    /// rounds 2 more so the two stay concentric. `None`, the default, takes
    /// the `control` radius token for the selection and `surface` for the track.
    pub radius: Option<f32>,
    /// Size of the labels in logical pixels, 11.5 by default.
    pub font_size: f32,
    /// A one pixel outline round the segmented selection, as a raised chip
    /// on its track. `None`, the default, draws none.
    pub selection_border: Option<Color>,
}

impl Tabs {
    /// A row of these tabs, in order.
    pub fn new(items: Vec<Tab>, palette: ComponentPalette) -> Self {
        Self {
            items,
            palette,
            height: ControlSize::Dense.height(),
            gap: TAB_GAP,
            id: None,
            animation_ms: crate::tokens().motion.move_ms,
            variant: TabsVariant::Segmented,
            accent: None,
            radius: None,
            font_size: TAB_FONT_SIZE,
            selection_border: None,
        }
    }

    /// Outline the segmented selection one pixel wide in this color, such as
    /// a white chip that needs an edge to stand on a pale track.
    pub fn selection_border(mut self, color: Color) -> Self {
        self.selection_border = Some(color);
        self
    }

    /// Round the selection by `radius` logical pixels and the segmented track
    /// by 2 more, concentric with it, in place of the radius tokens. Half the
    /// height or more makes both a capsule. A negative radius becomes zero;
    /// one that is not finite is ignored.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = Some(radius.max(0.0));
        }
        self
    }

    /// Set the label size in logical pixels, in place of 11.5. A size that is
    /// not finite or not above zero is ignored.
    pub fn font_size(mut self, size: f32) -> Self {
        if size.is_finite() && size > 0.0 {
            self.font_size = size;
        }
        self
    }

    /// Choose how the active tab is marked; see [`TabsVariant`].
    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Ink the active tab, and its underline, in this color in place of the
    /// palette's `text`, such as a theme's `accent_text()`.
    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = Some(accent);
        self
    }

    /// A stable ID for the strip's selection animation. Useful when tabs are
    /// dynamically inserted, removed, or reordered.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// How long the selection takes to slide to another tab, in milliseconds,
    /// in place of the `move_ms` token.
    pub fn animation_ms(mut self, duration: u16) -> Self {
        self.animation_ms = duration;
        self
    }

    /// Set the height, in logical pixels, never below 24. A height that is
    /// not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(24.0);
        }
        self
    }

    /// Take the height of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.height = size.height();
        self
    }

    /// Set the space between two tabs, in logical pixels, never below 0. A gap
    /// that is not finite is ignored. Set it before
    /// [`fill_width`](Self::fill_width), which reads it.
    pub fn gap(mut self, gap: f32) -> Self {
        if gap.is_finite() {
            self.gap = gap.max(0.0);
        }
        self
    }

    /// Give every tab the same width so the track is `total` wide, whatever
    /// width each tab was built with. A total that is not finite is ignored.
    pub fn fill_width(mut self, total: f32) -> Self {
        if total.is_finite() && !self.items.is_empty() {
            let count = self.items.len() as f32;
            let width = (total - TAB_TRACK_PADDING * 2.0 - self.gap * (count - 1.0)) / count;
            for item in &mut self.items {
                // The same floor as `Tab::new`.
                item.width = width.max(24.0);
            }
        }
        self
    }
}

impl Component for Tabs {
    fn render(self) -> Element {
        let selected_id = self
            .items
            .iter()
            .find(|item| item.active && item.state == ControlState::Enabled)
            .map(|item| item.action_id.clone());
        let enabled_ids = self
            .items
            .iter()
            .filter(|item| item.state == ControlState::Enabled)
            .map(|item| item.action_id.clone())
            .collect();
        let group_id = self.id.unwrap_or_else(|| {
            self.items
                .first()
                .map(|item| format!("tabs:{}", item.action_id))
                .unwrap_or_else(|| "tabs:empty".to_owned())
        });
        let accent = self.accent.unwrap_or(self.palette.text);
        let selection_radius = self.radius.unwrap_or_else(control_radius);
        let track_radius = self.radius.map_or_else(surface_radius, |radius| radius + TAB_TRACK_PADDING);
        let font_size = self.font_size;
        // Underlined, a strip of slots under the tabs gives the sliding
        // underline its thin target while the tabs keep their full height.
        let slots = (self.variant == TabsVariant::Underlined).then(|| {
            self.items
                .iter()
                .map(|item| {
                    Element::spacer()
                        .size(Some(item.width), Some(TAB_UNDERLINE))
                        .indicator_slot(item.action_id.clone())
                })
                .collect::<Vec<_>>()
        });
        let items = self
            .items
            .into_iter()
            .map(|item| {
                let ink = if item.active {
                    accent
                } else {
                    self.palette.muted
                };
                // The shared indicator paints below the controls and moves
                // between them. Individual surfaces stay transparent so they
                // cannot hide its intermediate position.
                let fill = Color::TRANSPARENT;
                // A tab is a text treatment above the shared selection
                // indicator. Hover changes ink only; another fill would
                // obscure the sliding indicator and make the track look
                // uneven. The active tab retains its selected surface.
                let hover_ink = if item.active {
                    lighter(accent, 12)
                } else {
                    self.palette.text
                };
                let access = Access::new(Role::Tab).selected(item.active);
                if item.state == ControlState::Disabled {
                    let access = access.id(item.action_id).name(item.label.clone()).disabled(true);
                    let label = Element::text(item.label)
                        .foreground(self.palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(font_size);
                    let visual = if let Some(icon) = item.icon {
                        Element::row(vec![
                            Element::icon(icon).foreground(self.palette.muted),
                            label,
                        ])
                        .gap(TAB_ICON_GAP)
                    } else {
                        label
                    };
                    return Element::row(vec![
                        Element::spacer().flex(1.0),
                        // At the height of an enabled tab's label.
                        visual.centered_y(),
                        Element::spacer().flex(1.0),
                    ])
                    .padding(TAB_ITEM_PADDING)
                    .background(Color::TRANSPARENT)
                    .radius(selection_radius)
                    .size(Some(item.width), Some(self.height))
                    .accessible(access);
                }
                let button = if let Some(icon) = item.icon {
                    Element::icon_button(item.action_id, icon, item.label)
                } else {
                    Element::button(item.action_id, item.label)
                };
                button
                    .background(fill)
                    .hover_foreground(hover_ink)
                    .pressed_foreground(hover_ink)
                    .foreground(ink)
                    .font_family(FontFamily::Sans)
                    .font_size(font_size)
                    .padding(TAB_ITEM_PADDING)
                    .gap(TAB_ICON_GAP)
                    .radius(selection_radius)
                    .center_content()
                    .size(Some(item.width), Some(self.height))
                    .accessible(access)
            })
            .collect();
        let strip = Element::tab_strip(items, enabled_ids)
            .gap(self.gap)
            .padding(TAB_TRACK_PADDING)
            .size(None, Some(self.height + TAB_TRACK_PADDING * 2.0));
        if let Some(slots) = slots {
            let mut underline = Element::tab_strip(slots, Vec::new())
                .gap(self.gap)
                .size(None, Some(TAB_UNDERLINE));
            if let Some(selected_id) = selected_id {
                underline =
                    underline.tab_selection(group_id, selected_id, accent, 0.0, self.animation_ms);
            }
            let strip_height = self.height + TAB_TRACK_PADDING * 2.0;
            // Anchored to the bottom of the strip, not of the box: a box taller
            // than the strip stretches the stack, never the tabs.
            let foot = |line: Element, height: f32| {
                Element::column(vec![
                    Element::spacer().size(None, Some(strip_height - height)),
                    line,
                ])
            };
            return Element::stack(vec![
                foot(
                    Element::spacer()
                        .size(None, Some(TAB_HAIRLINE))
                        .background(self.palette.border),
                    TAB_HAIRLINE,
                ),
                foot(
                    Element::row(vec![
                        Element::spacer().size(Some(TAB_TRACK_PADDING), Some(TAB_UNDERLINE)),
                        underline,
                    ]),
                    TAB_UNDERLINE,
                ),
                strip,
            ])
            .size(None, Some(strip_height));
        }
        let strip = strip
            .background(self.palette.surface)
            .radius(track_radius);
        if let Some(selected_id) = selected_id {
            let strip = strip.tab_selection(
                group_id,
                selected_id,
                self.palette.selected,
                selection_radius,
                self.animation_ms,
            );
            match self.selection_border {
                Some(color) => strip.tab_selection_border(color),
                None => strip,
            }
        } else {
            strip
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, ElementKind, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn all_tabs_share_the_segmented_track() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(vec![Tab::new("files", "Archivos", 88.0)], palette).render();
        assert_eq!(tabs.style.background, Some(palette.surface));
        assert_eq!(tabs.style.radius, surface_radius());
        assert_eq!(tabs.style.padding, 2.0);
        assert_eq!(tabs.style.height, Some(28.0));
        assert_eq!(tabs.style.gap, 2.0);
        let ElementKind::TabStrip { children, .. } = tabs.kind else {
            panic!("Tabs must render one strip");
        };
        assert_eq!(children[0].style.height, Some(24.0));
        assert_eq!(children[0].style.padding, 4.0);
        assert_eq!(children[0].style.font_size, 11.5);
    }


    #[test]
    fn every_tab_centers_its_label_or_icon_label_group() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("all", "Todo", 120.0).active(true),
                Tab::new("actions", "Acciones", 120.0).icon(Icon::Code),
                Tab::new("disabled", "Archivos", 120.0).disabled(),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(tabs);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 10.0);
        let frame = tree.layout(Size::new(368.0, 28.0));
        let text_x = |name: &str| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, position, .. } if text == name => Some(position.x),
                    _ => None,
                })
                .unwrap()
        };
        let icon_x = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon {
                    icon: Icon::Code,
                    rect,
                    ..
                } => Some(rect.x),
                _ => None,
            })
            .unwrap();
        let center = |left: f32, right: f32| (left + right) / 2.0;
        assert!((center(text_x("Todo"), text_x("Todo") + 40.0) - 62.0).abs() < 0.1);
        assert!((center(icon_x, text_x("Acciones") + 80.0) - 184.0).abs() < 0.1);
        assert!((center(text_x("Archivos"), text_x("Archivos") + 80.0) - 306.0).abs() < 0.1);
    }


    #[test]
    fn explicit_tab_height_preserves_files_and_project_switchers() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("files", "Archivos", 88.0).active(true),
                Tab::new("projects", "Proyectos", 92.0),
            ],
            palette,
        )
        .height(32.0)
        .render();
        assert_eq!(tabs.style.height, Some(36.0));
        let ElementKind::TabStrip {
            children,
            selection,
            ..
        } = tabs.kind
        else {
            panic!("Tabs must render one strip");
        };
        assert_eq!(children[0].style.height, Some(32.0));
        assert_eq!(children[1].style.height, Some(32.0));
        assert_eq!(selection.unwrap().selected_id, "files");
    }


    #[test]
    fn tabs_preserve_domain_action_ids() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("files", "Archivos", 100.0).icon(Icon::Folder),
                Tab::new("sessions", "Sesiones", 100.0).active(true),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(tabs);
        let frame = tree.layout(Size::new(240.0, 40.0));
        assert!(frame.regions.iter().any(|region| region.id == "files"));
        assert!(frame.regions.iter().any(|region| region.id == "sessions"));
    }


    #[test]
    fn active_tab_hover_preserves_selected_treatment() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("group", "Grupo", 80.0),
                Tab::new("project", "Proyecto", 80.0).active(true),
            ],
            palette,
        )
        .render();
        let ElementKind::TabStrip { children, .. } = tabs.kind else {
            panic!("Tabs must render a tab strip");
        };
        assert_eq!(children[0].style.hover_background, None);
        assert_eq!(children[1].style.hover_background, None);
        assert_eq!(children[0].style.hover_foreground, Some(palette.text));
        assert_eq!(
            children[1].style.hover_foreground,
            Some(lighter(palette.text, 12))
        );
    }


    #[test]
    fn tab_arrows_activate_visible_enabled_tabs_and_wrap_within_their_strip() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![
                Tab::new("sessions", "Sesiones", 100.0),
                Tab::new("unavailable", "Pronto", 100.0).disabled(),
                Tab::new("files", "Archivos", 100.0),
            ],
            palette,
        )
        .render();
        let mut tree = UiTree::new(Element::column(vec![
            tabs,
            Element::button("outside", "Otro"),
        ]));
        tree.layout(Size::new(340.0, 100.0));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        assert!(tree.focus("sessions"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: crate::Key::ArrowRight,
                shift: true,
                ctrl: false,
                alt: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("sessions"));
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert_eq!(tree.focused_id(), Some("files"));
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked {
                id: "sessions".into()
            }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowLeft)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::Home)),
            vec![UiAction::Clicked {
                id: "sessions".into()
            }]
        );
        assert!(tree.handle_event(key(crate::Key::Home)).is_empty());
        assert_eq!(
            tree.handle_event(key(crate::Key::End)),
            vec![UiAction::Clicked { id: "files".into() }]
        );
        assert!(!tree.focus("unavailable"));
        assert!(tree.focus("outside"));
        assert!(tree.handle_event(key(crate::Key::ArrowLeft)).is_empty());
        assert_eq!(tree.focused_id(), Some("outside"));
    }


    #[test]
    fn fill_width_shares_the_total_between_the_tabs_and_ignores_a_bad_total() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |total: f32| {
            Tabs::new(
                vec![Tab::new("a", "A", 50.0), Tab::new("b", "B", 90.0), Tab::new("c", "C", 70.0)],
                palette,
            )
            .fill_width(total)
        };
        for total in [240.0, 301.0, 520.0] {
            let mut tree = UiTree::new(Element::column(vec![
                Element::row(vec![tabs(total).render(), Element::spacer().flex(1.0)]),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(800.0, 100.0));
            let rect = |id: &str| frame.regions.iter().find(|region| region.id == id).unwrap().rect;
            assert_eq!(rect("a").width, rect("b").width);
            assert_eq!(rect("b").width, rect("c").width);
            assert!((rect("a").x - TAB_TRACK_PADDING).abs() < 0.01);
            let end = rect("c").x + rect("c").width + TAB_TRACK_PADDING;
            assert!((end - total).abs() < 0.01, "{total}: track ends at {end}");
        }
        for bad in [f32::NAN, f32::INFINITY] {
            let widths: Vec<_> = tabs(bad).items.iter().map(|item| item.width).collect();
            assert_eq!(widths, [50.0, 90.0, 70.0]);
        }
        assert!(tabs(-10.0).items.iter().all(|item| item.width == 24.0));
        assert!(Tabs::new(Vec::new(), palette).fill_width(200.0).items.is_empty());
    }


    fn underlined(active: &str, accent: Color) -> Element {
        Tabs::new(
            ["general", "roles", "emoji"]
                .iter()
                .map(|id| Tab::new(*id, id.to_uppercase(), 80.0).active(*id == active))
                .chain([Tab::new("audit", "Audit", 80.0).disabled()])
                .collect(),
            ComponentPalette::neutral_dark(),
        )
        .variant(TabsVariant::Underlined)
        .accent(accent)
        .gap(16.0)
        .id("settings")
        .render()
    }


    fn underlined_frame(tree: &mut UiTree) -> crate::Frame {
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
        // Taller than the strip's 28: the lines must stay under the tabs.
        tree.layout(Size::new(600.0, 40.0))
    }


    fn fills(frame: &crate::Frame, of: Color) -> Vec<crate::Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == of => Some(*rect),
                _ => None,
            })
            .collect()
    }


    #[test]
    fn underlined_tabs_draw_the_accent_under_the_active_tab_over_a_hairline() {
        let palette = ComponentPalette::neutral_dark();
        let accent = Color::rgb(25, 170, 216);
        let mut tree = UiTree::new(underlined("roles", accent));
        let frame = underlined_frame(&mut tree);
        let roles = frame
            .regions
            .iter()
            .find(|region| region.id == "roles")
            .unwrap()
            .rect;
        assert_eq!(
            fills(&frame, accent),
            [crate::Rect::new(roles.x, 26.0, roles.width, 2.0)]
        );
        // The hairline runs the width it is given, under the underline.
        assert_eq!(
            fills(&frame, palette.border),
            [crate::Rect::new(0.0, 27.0, 600.0, 1.0)]
        );
        assert!(
            fills(&frame, palette.surface).is_empty() && fills(&frame, palette.selected).is_empty()
        );
        let ink = |name: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, color, .. } if text == name => Some(*color),
                _ => None,
            })
        };
        assert_eq!(ink("ROLES"), Some(accent));
        assert_eq!(ink("GENERAL"), Some(palette.muted));
        assert_eq!(ink("Audit"), Some(palette.muted));
        // The disabled label sits at the height of the enabled ones.
        let top = |name: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == name => Some(position.y),
                _ => None,
            })
        };
        assert!(top("Audit").is_some());
        assert_eq!(top("Audit"), top("GENERAL"));
        // Without an accent the active tab takes the palette's text, as segmented.
        let plain = Tabs::new(vec![Tab::new("a", "A", 60.0).active(true)], palette)
            .variant(TabsVariant::Underlined)
            .render();
        let mut tree = UiTree::new(plain);
        assert_eq!(fills(&underlined_frame(&mut tree), palette.text).len(), 1);
    }


    #[test]
    fn underlined_tabs_keep_the_segmented_geometry_and_hover_ink() {
        let palette = ComponentPalette::neutral_dark();
        let items = || {
            vec![
                Tab::new("a", "A", 70.0).active(true),
                Tab::new("b", "B", 90.0),
            ]
        };
        let regions = |variant| {
            let tabs = Tabs::new(items(), palette)
                .variant(variant)
                .fill_width(300.0)
                .render();
            let mut tree = UiTree::new(Element::row(vec![tabs, Element::spacer().flex(1.0)]));
            let frame = underlined_frame(&mut tree);
            frame
                .regions
                .iter()
                .map(|region| (region.id.clone(), region.rect))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            regions(TabsVariant::Underlined),
            regions(TabsVariant::Segmented)
        );
        let ElementKind::Stack(layers) = Tabs::new(items(), palette)
            .variant(TabsVariant::Underlined)
            .accent(palette.border)
            .render()
            .kind
        else {
            panic!("Underlined tabs must stack the strip over its lines");
        };
        let ElementKind::TabStrip { children, .. } = &layers[2].kind else {
            panic!("The tabs must be the top layer");
        };
        assert_eq!(
            children[0].style.hover_foreground,
            Some(lighter(palette.border, 12))
        );
        assert_eq!(children[1].style.hover_foreground, Some(palette.text));
        assert_eq!(children[0].style.hover_background, None);
    }


    #[test]
    fn underlined_tabs_report_clicks_and_keys_and_the_underline_follows() {
        let accent = Color::rgb(25, 170, 216);
        let mut tree = UiTree::new(underlined("general", accent));
        let frame = underlined_frame(&mut tree);
        let emoji = frame
            .regions
            .iter()
            .find(|region| region.id == "emoji")
            .unwrap()
            .rect;
        let center = Point::new(emoji.x + emoji.width / 2.0, emoji.y + emoji.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(center));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(center)),
            vec![UiAction::Clicked { id: "emoji".into() }]
        );
        assert!(tree.focus("general"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked { id: "roles".into() }]
        );
        // The disabled tab is stepped over: from the last enabled one, Right wraps.
        assert_eq!(
            tree.handle_event(key(crate::Key::End)),
            vec![UiAction::Clicked { id: "emoji".into() }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::ArrowRight)),
            vec![UiAction::Clicked {
                id: "general".into()
            }]
        );
        assert!(!underlined_frame(&mut tree)
            .regions
            .iter()
            .any(|region| region.id == "audit"));
        tree.set_root(underlined("emoji", accent));
        let frame = underlined_frame(&mut tree);
        assert_eq!(
            fills(&frame, accent),
            [crate::Rect::new(emoji.x, 26.0, emoji.width, 2.0)]
        );
        // A disabled tab marked active draws no underline.
        tree.set_root(underlined("audit", accent));
        assert!(fills(&underlined_frame(&mut tree), accent).is_empty());
    }


    #[test]
    fn tabs_gap_ignores_bad_input_and_underlined_tabs_fill_a_narrow_width() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = || Tabs::new(vec![Tab::new("a", "A", 60.0)], palette);
        assert_eq!(tabs().gap(f32::NAN).gap, TAB_GAP);
        assert_eq!(tabs().gap(f32::INFINITY).gap, TAB_GAP);
        assert_eq!(tabs().gap(-4.0).gap, 0.0);
        let accent = Color::rgb(25, 170, 216);
        let narrow = Tabs::new(
            vec![
                Tab::new("a", "Alpha", 80.0).active(true),
                Tab::new("b", "Beta", 80.0),
            ],
            palette,
        )
        .variant(TabsVariant::Underlined)
        .accent(accent)
        .gap(16.0)
        .fill_width(120.0)
        .render();
        let mut tree = UiTree::new(narrow);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(120.0, 28.0));
        assert_eq!(
            fills(&frame, palette.border),
            [crate::Rect::new(0.0, 27.0, 120.0, 1.0)]
        );
        // Nothing escapes the width: neither a tab nor the underline.
        let inside = |rect: &crate::Rect| rect.x >= 0.0 && rect.x + rect.width <= 120.0;
        assert_eq!(frame.regions.len(), 2);
        assert!(frame.regions.iter().all(|region| inside(&region.rect)));
        let underline = fills(&frame, accent);
        assert_eq!(underline.len(), 1);
        assert!(inside(&underline[0]));
    }


    #[test]
    fn capsule_tabs_are_concentric_stadiums_with_labels_of_their_own_size() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |radius: Option<f32>| {
            let tab = |index: usize| Tab::new(format!("t{index}"), "Tab", 90.0).active(index == 0);
            let tabs = Tabs::new(vec![tab(0), tab(1)], palette).height(40.0).font_size(15.0);
            match radius { Some(radius) => tabs.radius(radius), None => tabs }.render()
        };
        let painted = |radius| {
            let mut tree = UiTree::new(Element::column(vec![tabs(radius)]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(300.0, 60.0));
            let fill = |color| frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color: fill, radius } if *fill == color => Some((rect.height, *radius)),
                _ => None,
            }).unwrap();
            let size = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { font_size, .. } => Some(*font_size),
                _ => None,
            });
            (fill(palette.surface), fill(palette.selected), size)
        };
        // A radius of half the height or more: both ends are semicircles.
        assert_eq!(painted(Some(100.0)), ((44.0, 22.0), (40.0, 20.0), Some(15.0)));
        // A smaller one keeps the track concentric, 2 rounder.
        assert_eq!(painted(Some(10.0)), ((44.0, 12.0), (40.0, 10.0), Some(15.0)));
        assert_eq!(painted(None), ((44.0, surface_radius()), (40.0, control_radius()), Some(15.0)));
        let plain = Tabs::new(Vec::new(), palette);
        assert_eq!((plain.radius, plain.font_size), (None, TAB_FONT_SIZE));
        let ignored = Tabs::new(Vec::new(), palette).radius(f32::NAN).font_size(0.0).font_size(f32::INFINITY);
        assert_eq!((ignored.radius, ignored.font_size), (None, TAB_FONT_SIZE));
        assert_eq!(Tabs::new(Vec::new(), palette).radius(-3.0).radius, Some(0.0));
    }


    #[test]
    fn a_tab_selection_sets_off_on_the_first_frame_after_the_change() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = |chosen: usize| {
            let tab = |index: usize| Tab::new(format!("t{index}"), "Tab", 80.0).active(index == chosen);
            Tabs::new(vec![tab(0), tab(1), tab(2)], palette).id("tabs").render()
        };
        let size = Size::new(300.0, 40.0);
        let start = web_time::Instant::now() - web_time::Duration::from_secs(10);
        let frame = web_time::Duration::from_micros(16_667);
        let mut tree = UiTree::new(tabs(0));
        let from = fills(&tree.layout_at(size, start), palette.selected)[0].x;
        tree.set_root(tabs(2));
        assert_eq!(fills(&tree.layout_at(size, start), palette.selected)[0].x, from);
        let to = {
            let mut rest = UiTree::new(tabs(2));
            fills(&rest.layout(size), palette.selected)[0].x
        };
        // An ease-out over the move token covers a quarter of the way in one
        // frame; a spring from standstill covered under a seventh.
        let first = fills(&tree.layout_at(size, start + frame), palette.selected)[0].x;
        assert!((first - from) / (to - from) > 0.25, "{first} between {from} and {to}");
        let settled = start + web_time::Duration::from_secs(2);
        assert_eq!(fills(&tree.layout_at(size, settled), palette.selected)[0].x, to);
    }
}
