//! Shared menu primitives for popovers and context menus.
//!
//! The caller owns the menu's open state and the meaning of each action ID.
//! Each item owns one complete hit target, so moving between its icon, label,
//! and trailing hint never creates separate hover patches.

use crate::{
    Access, ButtonSize, ButtonVariant, Color, Component, ComponentPalette, Element, ElementKind,
    FontFamily, Icon, IconButton, Kbd, Role, SelectorSizing, TooltipSpec, ICON_SIZE,
};

/// Name the control inside `element` for the platform by the title of the
/// hint around it: an icon alone has nothing else to be called.
pub(crate) fn named_by_hint(element: Element) -> Element {
    match &element.kind {
        ElementKind::Tooltip { title, .. } if !title.is_empty() => {
            let title = title.clone();
            element.accessibility_label(title)
        }
        _ => element,
    }
}

/// The editor's context menu defines the visual standard for action menus.
/// Selector menus can keep their own density and richer option content.
pub(crate) mod action_style {
    use crate::{ComponentPalette, Element, Point};

    pub(crate) const ROW_HEIGHT: f32 = 26.0;
    pub(crate) const INSET: f32 = 4.0;
    pub(crate) const WIDTH: f32 = 352.0;
    pub(crate) const MAX_HEIGHT: f32 = 480.0;
    pub(crate) const LABEL_FONT_SIZE: f32 = 13.0;
    pub(crate) const SHORTCUT_FONT_SIZE: f32 = 12.0;
    pub(crate) fn row_radius() -> f32 {
        crate::components::control_radius()
    }
    pub(crate) const HORIZONTAL_PADDING: f32 = 8.0;
    pub(crate) const GAP: f32 = 8.0;
    pub(crate) const SEPARATOR_HEIGHT: f32 = 9.0;
    pub(crate) const SCROLLBAR_GUTTER: f32 = 14.0;
    pub(crate) const HEADER_HEIGHT: f32 = 24.0;

    pub(crate) fn surface(
        content: Element,
        palette: ComponentPalette,
        width: f32,
        height: Option<f32>,
    ) -> Element {
        Element::panel(content)
            .padding(INSET)
            .background(palette.surface)
            .border(palette.border, 1.0)
            .radius(crate::components::surface_radius())
            .shadow(palette.shadow, Point::new(0.0, 6.0), 18.0, 0.0)
            .size(Some(width), height)
            .clip_children()
    }
}

/// The width of a switch's track, which the check column takes in a menu
/// with switches, in place of an icon's.
pub(crate) const SWITCH_WIDTH: f32 = 28.0;

/// What a menu item that is a check box, a radio item or a switch draws at
/// the end of its row, and tells the platform it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuCheck {
    /// A check box, ticked while `true`.
    Box(bool),
    /// One radio item of a set, the chosen one while `true`.
    Radio(bool),
    /// A switch, on while `true`: a small track with its thumb at the end it
    /// is on at. It is a static mark; the row is what is chosen.
    Switch(bool),
}

impl MenuCheck {
    /// The box or the circle, `side` square: filled with `ink` and marked in
    /// the surface color while checked, an outline in the muted color while
    /// not, as a `Checkbox` and a `RadioGroup` draw theirs. A switch is
    /// `SWITCH_WIDTH` wide instead, colored as `SwitchVariant::Accent`, its
    /// on color `accent` or the palette's link color; `side` and `ink` are not
    /// used by it.
    pub(crate) fn mark(
        self,
        side: f32,
        ink: Color,
        accent: Option<Color>,
        palette: ComponentPalette,
    ) -> Element {
        let (checked, round) = match self {
            Self::Box(checked) => (checked, false),
            Self::Radio(checked) => (checked, true),
            Self::Switch(on) => {
                let (thumb, track) = crate::components::accent_switch_colors(palette, on, accent);
                let thumb = Element::spacer()
                    .background(thumb)
                    .radius(5.0)
                    .size(Some(10.0), Some(10.0));
                let gap = Element::spacer().flex(1.0);
                let thumbs = if on { vec![gap, thumb] } else { vec![thumb, gap] };
                return Element::panel(Element::row(thumbs))
                    .padding(3.0)
                    .background(track)
                    .radius(8.0)
                    .size(Some(SWITCH_WIDTH), Some(16.0));
            }
        };
        let (fill, mark) = if checked {
            (ink, palette.surface)
        } else {
            (Color::TRANSPARENT, Color::TRANSPARENT)
        };
        let shape = if round {
            let dot = crate::radio::dot_side(side);
            Element::panel(Element::spacer().background(mark).radius(dot / 2.0))
                .padding((side - dot) / 2.0)
                .radius(side / 2.0)
        } else {
            Element::panel(Element::icon(Icon::Check).foreground(mark))
                .radius(crate::tokens().radius.control)
        };
        // A filled mark has no border: a stroke of the fill's own color shows as a ring inside its edge.
        shape
            .background(fill)
            .border(palette.muted, if checked { 0.0 } else { 1.0 })
            .size(Some(side), Some(side))
    }

    /// `access` as a check box or a radio item of a menu, on or off. There is
    /// no menu-item switch role, so a switch is a check box that is on or off.
    pub(crate) fn access(self, access: Access) -> Access {
        let (role, checked) = match self {
            Self::Box(checked) | Self::Switch(checked) => (Role::MenuItemCheckBox, checked),
            Self::Radio(checked) => (Role::MenuItemRadio, checked),
        };
        Access {
            role: Some(role),
            ..access
        }
        .toggled(checked)
    }
}

/// The two-line row shared by [`MenuItem`] and `DropdownMenuItem`: a label
/// over a quiet detail, between a leading icon and a check or a chevron.
/// Its metrics are its menu's, so it lines up with the menu's other rows.
pub(crate) struct DetailRow {
    pub(crate) action_id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) icon: Option<Icon>,
    /// Keep the icon's slot without an icon, as the menu's other rows do.
    pub(crate) icon_slot: bool,
    pub(crate) mono: bool,
    pub(crate) status: Option<String>,
    pub(crate) status_width: Option<f32>,
    /// `None` leaves out the chevron column; `Some(false)` keeps it empty.
    /// A row with a chevron leads somewhere: it is not a choice.
    pub(crate) chevron: Option<bool>,
    /// `None` leaves out the check column; `Some(false)` keeps it empty.
    pub(crate) selected: Option<bool>,
    /// Whether the platform hears `selected`: the row is one of a set of
    /// choices, not an action in a menu that has some.
    pub(crate) choice: bool,
    /// A check box or a radio item in the check column, in place of the tick.
    pub(crate) check: Option<MenuCheck>,
    /// The check column's width: a switch's, where the menu has switches.
    pub(crate) check_width: f32,
    pub(crate) enabled: bool,
    /// Its height, icon slot, text sizes and gap, and `padding` at its sides.
    pub(crate) sizing: SelectorSizing,
}

impl DetailRow {
    /// The padded row without a width: the caller fixes it or lets it stretch.
    pub(crate) fn render(self, palette: ComponentPalette) -> Element {
        let sizing = self.sizing;
        // Two lines of text need about 1.2 times their sizes; the padding
        // above and below gives way to them, never past zero.
        let text_height = (sizing.label_size + sizing.detail_size) * 1.2;
        let inset_y = ((sizing.detail_row_height - text_height) / 2.0)
            .min(sizing.padding)
            .max(0.0);
        let body_height = (sizing.detail_row_height - inset_y * 2.0).max(0.0);
        let line_height = body_height / 2.0;
        let leads = self.chevron == Some(true);
        let ink = if self.enabled {
            palette.text
        } else {
            palette.muted
        };
        // The label names the row; its second line is said after it.
        let mut access = Access::new(Role::MenuItem).name(&self.label);
        if !self.description.is_empty() {
            access = access.description(&self.description);
        }
        if let Some(check) = self.check.filter(|_| !leads) {
            access = check.access(access);
        } else if let Some(selected) = self.selected.filter(|_| self.choice && !leads) {
            access = access.selected(selected);
        }
        let detail = Element::truncated_text(self.description)
            .font_family(if self.mono {
                FontFamily::Monospace
            } else {
                FontFamily::Sans
            })
            .font_size(sizing.detail_size)
            .foreground(palette.muted)
            .size(None, Some(line_height));
        let detail = match self.status.filter(|status| !status.is_empty()) {
            Some(status) => Element::row(vec![
                detail.flex(1.0),
                Element::truncated_text(status)
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.detail_size)
                    .foreground(palette.muted)
                    .size(self.status_width, Some(line_height)),
            ])
            .gap(sizing.gap)
            .size(None, Some(line_height)),
            None => detail,
        };
        let slot = |element: Element| element.size(Some(sizing.icon_size), Some(body_height));
        let mut children = Vec::with_capacity(4);
        if let Some(icon) = self.icon {
            children.push(slot(
                Element::icon(icon)
                    // A row that only leads somewhere keeps its icon as quiet
                    // as its chevron; the label says where it goes.
                    .foreground(if leads { palette.muted } else { ink }),
            ));
        } else if self.icon_slot {
            children.push(slot(Element::spacer()));
        }
        children.push(
            Element::column(vec![
                Element::truncated_text(self.label)
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.label_size)
                    .foreground(ink)
                    .size(None, Some(line_height)),
                detail,
            ])
            .flex(1.0),
        );
        if self.selected.is_some() || self.check.is_some() {
            let accent = (!self.enabled).then_some(palette.muted);
            children.push(
                match (self.check, self.selected) {
                    _ if leads => Element::spacer(),
                    (Some(check), _) => check
                        .mark(sizing.icon_size.min(ICON_SIZE), ink, accent, palette)
                        .centered(),
                    (None, Some(true)) => Element::icon(Icon::Check).foreground(palette.text),
                    (None, _) => Element::spacer(),
                }
                .size(Some(self.check_width), Some(body_height)),
            );
        }
        if let Some(chevron) = self.chevron {
            children.push(slot(if chevron {
                Element::icon(Icon::ChevronRight).foreground(palette.muted)
            } else {
                Element::spacer()
            }));
        }
        let visual = Element::row(children).gap(sizing.gap).clip_children();
        // The sides keep the whole padding, as the one-line rows do.
        let side = (sizing.padding - inset_y).max(0.0);
        let visual = if side > 0.0 {
            Element::row(vec![
                Element::spacer().size(Some(side), Some(body_height)),
                visual.flex(1.0),
                Element::spacer().size(Some(side), Some(body_height)),
            ])
        } else {
            visual
        };
        let row = if self.enabled {
            Element::clickable(self.action_id, visual).action_menu_item().accessible(access)
        } else {
            Element::panel(visual).accessible(access.id(self.action_id).disabled(true))
        };
        palette
            .interactive_surface(row)
            .transition_ms(0)
            .padding(inset_y)
            .size(None, Some(sizing.detail_row_height))
    }
}

/// A full-width menu action with an optional icon, annotation or shortcut.
/// With a [`description`](Self::description) it is a two-line row instead.
pub struct MenuItem {
    action_id: String,
    label: String,
    icon: Option<Icon>,
    trailing: Option<String>,
    shortcut: Option<String>,
    width: f32,
    height: f32,
    palette: ComponentPalette,
    destructive: Option<Color>,
    description: Option<String>,
    detail_mono: bool,
    status: Option<String>,
    status_width: Option<f32>,
    chevron: bool,
    selected: Option<bool>,
    enabled: bool,
    tooltip: Option<TooltipSpec>,
    trailing_action: Option<TrailingAction>,
}

struct TrailingAction {
    id: String,
    icon: Icon,
    tooltip: TooltipSpec,
    dispatch_as: Option<String>,
}

impl MenuItem {
    /// An enabled row 220 wide that shows `label`. `action_id` is what the host receives
    /// when it is chosen.
    pub fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            trailing: None,
            shortcut: None,
            width: 220.0,
            height: action_style::ROW_HEIGHT,
            palette,
            destructive: None,
            description: None,
            detail_mono: false,
            status: None,
            status_width: None,
            chevron: false,
            selected: None,
            enabled: true,
            tooltip: None,
            trailing_action: None,
        }
    }

    /// A leading icon for the row.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Plain text at the trailing edge. It replaces any `shortcut`.
    pub fn trailing(mut self, trailing: impl Into<String>) -> Self {
        self.trailing = Some(trailing.into());
        self.shortcut = None;
        self
    }

    /// A keyboard shortcut at the trailing edge. It replaces any `trailing` text.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self.trailing = None;
        self
    }

    /// Set the row's width in logical pixels, never below 40 (220 by default). A value that
    /// is not finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(40.0);
        }
        self
    }

    /// Set the row's height in logical pixels, never below 24. A row with a `description`
    /// ignores it. A value that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(24.0);
        }
        self
    }

    /// Draw the one-line row in `ink` while it is enabled, for an action that destroys
    /// something.
    pub fn destructive(mut self, ink: Color) -> Self {
        self.destructive = Some(ink);
        self
    }

    /// A quiet second line under the label. The row becomes two lines tall
    /// (`SelectorSizing::detail_row_height`) and ignores `height`, `trailing`
    /// and `shortcut`.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Set the description in the monospaced family, for an identifier.
    pub fn detail_mono(mut self, mono: bool) -> Self {
        self.detail_mono = mono;
        self
    }

    /// Short muted text at the far end of the description line.
    pub fn status(mut self, text: impl Into<String>) -> Self {
        self.status = Some(text.into());
        self
    }

    /// Fix the status slot's width instead of following its text. Values
    /// that are not finite are ignored.
    pub fn status_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.status_width = Some(width.max(0.0));
        }
        self
    }

    /// End a two-line row with a chevron: it opens another view.
    pub fn chevron(mut self) -> Self {
        self.chevron = true;
        self
    }

    /// Keep a column for a check at the end of a two-line row, and fill it
    /// when `selected`.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// A disabled item is passive: no hover, no click and no keyboard stop.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// A hint for the row itself.
    pub fn tooltip_with(mut self, tooltip: TooltipSpec) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// A second, square action at the end of the row, such as "close". Tab
    /// reaches it; the arrow keys keep moving between the rows. It stays
    /// live when the row itself is disabled.
    pub fn trailing_action(
        mut self,
        id: impl Into<String>,
        icon: Icon,
        tooltip: TooltipSpec,
    ) -> Self {
        self.trailing_action = Some(TrailingAction {
            id: id.into(),
            icon,
            tooltip,
            dispatch_as: None,
        });
        self
    }

    /// Report this action when the trailing action is activated, while it
    /// keeps its own ID as a focus target.
    pub fn trailing_dispatch_as(mut self, action: impl Into<String>) -> Self {
        if let Some(trailing) = &mut self.trailing_action {
            trailing.dispatch_as = Some(action.into());
        }
        self
    }
}

impl Component for MenuItem {
    fn render(mut self) -> Element {
        let palette = self.palette;
        let width = self.width;
        let tooltip = self.tooltip.take();
        let trailing_action = self.trailing_action.take();
        let id = self.action_id.clone();
        let (main, height) = match self.description.take() {
            Some(description) => (
                DetailRow {
                    action_id: self.action_id,
                    label: self.label,
                    description,
                    icon: self.icon,
                    mono: self.detail_mono,
                    status: self.status,
                    status_width: self.status_width,
                    icon_slot: false,
                    // A chevron takes the check's place: the row leads somewhere.
                    chevron: self.chevron.then_some(true),
                    selected: self.selected.filter(|_| !self.chevron),
                    choice: true,
                    check: None,
                    check_width: SelectorSizing::COMPACT.icon_size,
                    enabled: self.enabled,
                    sizing: SelectorSizing::COMPACT,
                }
                .render(palette),
                SelectorSizing::COMPACT.detail_row_height,
            ),
            None => {
                let height = self.height;
                (self.action_row(), height)
            }
        };
        let row_id = id.clone();
        let hint = |main: Element| match tooltip {
            Some(tooltip) => tooltip.wrap(main, id, palette),
            None => main,
        };
        let Some(action) = trailing_action else {
            return hint(main.size(Some(width), Some(height)));
        };
        // The row answers the pointer over either of its targets with one
        // fill behind both; the action answers with a fill of its own, a
        // shade over the row's, so it reads as a button inside the row.
        let surface = [row_id, action.id.clone()];
        let stepped = ComponentPalette {
            hover: palette.selected,
            ..palette
        };
        let size = ButtonSize::IconExtraSmall.sizing().height;
        let mut button = IconButton::new(&action.id, action.icon, stepped)
            .variant(ButtonVariant::Ghost)
            .button_size(ButtonSize::IconExtraSmall)
            .render();
        if let Some(target) = action.dispatch_as {
            button = button.dispatch_as(target);
        }
        let button = named_by_hint(action.tooltip.wrap(
            stepped.interactive_surface(button.menu_secondary_action()),
            action.id,
            palette,
        ));
        let main = main
            .hover_background(Color::TRANSPARENT)
            .pressed_background(Color::TRANSPARENT);
        Element::row(vec![
            hint(main.size(None, Some(height)).flex(1.0)).flex(1.0),
            button.centered_y().size(Some(size), Some(height)),
        ])
        .gap(2.0)
        .size(Some(width), Some(height))
        .radius(action_style::row_radius())
        .hover_background(palette.hover)
        .pressed_background(palette.selected)
        .hover_surface(format!("{}:surface", surface[0]), surface)
        .clip_children()
    }
}

impl MenuItem {
    // The one-line action row, without its outer width.
    fn action_row(self) -> Element {
        let ink = if self.enabled {
            self.destructive.unwrap_or(self.palette.text)
        } else {
            self.palette.muted
        };
        let content_height = self.height;
        // The label alone names the row: its keys are not part of what it is called.
        let mut access = Access::new(Role::MenuItem).name(&self.label);
        if let Some(trailing) = self.trailing.as_deref().filter(|text| !text.is_empty()) {
            access = access.description(trailing);
        }
        let mut children = Vec::with_capacity(3);
        if let Some(icon) = self.icon {
            children.push(
                Element::icon(icon)
                    .foreground(ink)
                    .size(Some(ICON_SIZE), Some(content_height)),
            );
        }
        children.push(
            Element::truncated_text(self.label)
                .foreground(ink)
                .font_family(FontFamily::Sans)
                .font_size(action_style::LABEL_FONT_SIZE)
                .size(None, Some(content_height))
                .flex(1.0),
        );
        if let Some(shortcut) = self.shortcut {
            let icon_budget = if self.icon.is_some() {
                ICON_SIZE + action_style::GAP
            } else {
                0.0
            };
            let hint_budget = (self.width
                - action_style::HORIZONTAL_PADDING * 2.0
                - icon_budget
                - action_style::GAP
                - 40.0)
                .max(0.0);
            children.push(
                Kbd::new(shortcut, self.palette)
                    .max_width(hint_budget)
                    .render()
                    .size(None, Some(content_height)),
            );
        } else if let Some(trailing) = self.trailing {
            children.push(
                Element::truncated_text(trailing)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(11.0)
                    .size(None, Some(content_height)),
            );
        }
        let content = Element::row(vec![
            Element::spacer().size(Some(action_style::HORIZONTAL_PADDING), None),
            Element::row(children).gap(action_style::GAP).flex(1.0),
            Element::spacer().size(Some(action_style::HORIZONTAL_PADDING), None),
        ]);
        if !self.enabled {
            return Element::panel(content)
                .clip_children()
                .size(None, Some(self.height))
                .accessible(access.id(self.action_id).disabled(true));
        }
        self.palette
            .interactive_surface(Element::clickable(self.action_id, content).accessible(access))
            .foreground(ink)
            .action_menu_item()
            .transition_ms(0)
            .radius(action_style::row_radius())
            .clip_children()
            .size(None, Some(self.height))
    }
}

/// A quiet separator that retains the menu's inner horizontal inset.
pub struct MenuSeparator {
    width: f32,
    color: Color,
}

impl MenuSeparator {
    /// A rule of this width and color between two groups of rows. A width
    /// that is not finite or is negative is zero.
    pub fn new(width: f32, color: Color) -> Self {
        Self { width: crate::page::px(width), color }
    }
}

impl Component for MenuSeparator {
    fn render(self) -> Element {
        Element::panel(Element::spacer().background(self.color))
            .padding((action_style::SEPARATOR_HEIGHT - 1.0) / 2.0)
            .size(Some(self.width), Some(action_style::SEPARATOR_HEIGHT))
    }
}

#[cfg(test)]
mod tests {
    use crate::{DrawCommand, Point, Size, UiEvent, UiTree};

    use super::*;

    #[test]
    fn a_menu_item_tells_the_platform_its_label_its_state_and_its_second_action() {
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(Element::column(vec![
            MenuItem::new("rename", "Rename", palette).shortcut("Ctrl+R").render(),
            MenuItem::new("shell", "Terminal 1", palette)
                .trailing("Active")
                .trailing_action("close", Icon::Close, TooltipSpec::new("Close"))
                .render(),
            MenuItem::new("plain", "Plain", palette).disabled().render(),
            MenuItem::new("project", "limn", palette).description("/home/user/limn").selected(true).render(),
            MenuItem::new("gone", "Missing", palette).description("/home/user/gone").selected(false).disabled().render(),
            MenuItem::new("more", "More", palette).description("Other projects").chevron().render(),
            MenuSeparator::new(220.0, palette.border).render(),
        ]))
        .layout(Size::new(260.0, 320.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));
        let rename = node("rename");
        assert_eq!((rename.role, rename.name.as_str(), rename.selected), (Role::MenuItem, "Rename", None));
        assert!(rename.actionable && !rename.disabled);
        let shell = node("shell");
        assert_eq!((shell.name.as_str(), shell.description.as_deref()), ("Terminal 1", Some("Active")));
        let close = node("close");
        assert_eq!((close.role, close.name.as_str()), (Role::Button, "Close"));
        let plain = node("plain");
        assert_eq!((plain.role, plain.name.as_str()), (Role::MenuItem, "Plain"));
        assert!(plain.disabled && !plain.actionable);
        let project = node("project");
        assert_eq!((project.role, project.name.as_str()), (Role::MenuItem, "limn"));
        assert_eq!((project.description.as_deref(), project.selected), (Some("/home/user/limn"), Some(true)));
        let gone = node("gone");
        assert_eq!((gone.role, gone.name.as_str(), gone.selected), (Role::MenuItem, "Missing", Some(false)));
        assert!(gone.disabled && !gone.actionable);
        assert_eq!(node("more").selected, None, "a row that leads somewhere is not a choice");
        assert_eq!(frame.access.len(), 7, "a separator says nothing");
    }

    #[test]
    fn a_row_with_an_action_has_one_hover_behind_both_and_the_action_one_of_its_own() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![
            MenuItem::new("select", "Terminal 1", palette)
                .icon(Icon::Terminal)
                .trailing_action("close", Icon::Close, TooltipSpec::new("Close"))
                .width(240.0)
                .render(),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(240.0, 120.0));
        let rect = |id: &str| frame.regions.iter().find(|row| row.id == id).unwrap().rect;
        let (select, close) = (rect("select"), rect("close"));
        let fills = |tree: &mut UiTree, color: Color| -> Vec<crate::Rect> {
            tree.layout(Size::new(240.0, 120.0))
                .commands
                .iter()
                .filter_map(|command| match command {
                    crate::DrawCommand::FillRect { rect, color: fill, .. } if *fill == color => Some(*rect),
                    _ => None,
                })
                .collect()
        };
        let center = |rect: crate::Rect| Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        let whole = crate::Rect::new(0.0, 0.0, 240.0, action_style::ROW_HEIGHT);
        // At rest neither paints.
        assert!(fills(&mut tree, palette.hover).is_empty());
        // Over the row: one fill behind the row and its action, nothing on the action.
        tree.handle_event(UiEvent::PointerMove(center(select)));
        assert_eq!(fills(&mut tree, palette.hover), vec![whole]);
        assert!(fills(&mut tree, palette.selected).is_empty());
        // Over the action: the row keeps its fill, and the action adds its own.
        tree.handle_event(UiEvent::PointerMove(center(close)));
        assert_eq!(fills(&mut tree, palette.hover), vec![whole]);
        assert_eq!(fills(&mut tree, palette.selected), vec![close]);
    }

    #[test]
    fn a_trailing_action_is_a_square_of_24_in_a_row_of_26_with_its_own_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![
            MenuItem::new("select", "Terminal 1", palette)
                .icon(Icon::Terminal)
                .trailing("Active")
                .tooltip_with(TooltipSpec::new("Terminal 1"))
                .trailing_action("close", Icon::Trash, TooltipSpec::new("Close"))
                .trailing_dispatch_as("close-session")
                .width(240.0)
                .render(),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(240.0, 120.0));
        let rect = |id: &str| frame.regions.iter().find(|row| row.id == id).unwrap().rect;
        let (select, close) = (rect("select"), rect("close"));
        assert_eq!(select.height, action_style::ROW_HEIGHT);
        assert_eq!((close.width, close.height), (24.0, 24.0));
        assert_eq!(close.y, (action_style::ROW_HEIGHT - 24.0) / 2.0);
        assert_eq!(close.x + close.width, 240.0);
        assert_eq!(select.x + select.width + 2.0, close.x);
        let point = Point::new(close.x + 12.0, close.y + 12.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![crate::UiAction::Clicked {
                id: "close-session".into()
            }]
        );
    }

    #[test]
    fn a_described_item_is_two_lines_tall_and_a_disabled_one_is_passive() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = SelectorSizing::COMPACT;
        let frame = UiTree::new(Element::column(vec![
            MenuItem::new("project", "limn", palette)
                .icon(Icon::Folder)
                .description("/home/user/limn")
                .selected(true)
                .width(260.0)
                .render(),
            MenuItem::new("gone", "Missing", palette)
                .description("/home/user/gone")
                .enabled(false)
                .width(260.0)
                .render(),
            MenuItem::new("plain", "Plain", palette).enabled(false).render(),
            Element::spacer().flex(1.0),
        ]))
        .layout(Size::new(260.0, 200.0));
        let project = frame.regions.iter().find(|row| row.id == "project").unwrap().rect;
        assert_eq!(
            (project.width, project.height),
            (260.0, sizing.detail_row_height)
        );
        assert_eq!(frame.regions.len(), 1, "disabled items have no target");
        for (label, ink) in [("Missing", palette.muted), ("Plain", palette.muted)] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, .. } if text == label && *color == ink)));
        }
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::Check, tint, .. } if *tint == palette.text)));
        let item = MenuItem::new("a", "A", palette).status_width(f32::INFINITY);
        assert_eq!(item.status_width, None);
        assert_eq!(item.status_width(-2.0).status_width, Some(0.0));
    }

    #[test]
    fn whole_menu_item_is_one_hover_and_click_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            MenuItem::new("rename", "Renombrar", palette)
                .icon(Icon::Edit)
                .shortcut("Ctrl+R")
                .width(236.0)
                .render(),
        );
        tree.set_reduced_motion(true);
        let initial = tree.layout(Size::new(236.0, 36.0));
        for key in [
            if cfg!(target_os = "macos") {
                "⌃"
            } else {
                "Ctrl"
            },
            "R",
        ] {
            assert!(initial.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_family: FontFamily::Sans, .. }
                    if text == key
            )));
        }
        for x in [8.0, 42.0, 205.0] {
            tree.handle_event(UiEvent::PointerMove(Point::new(x, 18.0)));
            let frame = tree.layout(Size::new(236.0, 36.0));
            assert_eq!(frame.hit_test(Point::new(x, 18.0)), Some("rename"));
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::FillRect { color, rect, .. }
                    if *color == palette.hover && rect.width == 236.0
            )));
        }
    }

    #[test]
    fn narrow_menu_retains_its_action_label_without_partial_shortcuts() {
        let frame = UiTree::new(
            MenuItem::new("rename", "Renombrar", ComponentPalette::neutral_dark())
                .icon(Icon::Edit)
                .shortcut("Ctrl+Shift+R")
                .width(120.0)
                .render(),
        )
        .layout(Size::new(120.0, 36.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.starts_with("Renom"))));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if ["Ctrl", "Shift", "R"].contains(&text.as_str()))));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "rename");
    }

    #[test]
    fn compact_menu_shortcuts_are_visible_and_centered_inside_the_action() {
        let palette = ComponentPalette::neutral_dark();
        for height in [24.0, 28.0, 32.0, 36.0] {
            let frame = UiTree::new(
                MenuItem::new("rename", "Renombrar", palette)
                    .shortcut("Ctrl+R")
                    .height(height)
                    .render(),
            )
            .layout(Size::new(220.0, height));
            let caps: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                    } if *color == crate::shortcut::cap_fill(palette.text)
                        && *radius == crate::shortcut::radius() =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(caps.len(), 2, "both shortcut keys are visible");
            for cap in caps {
                assert_eq!(cap.height, crate::shortcut::HEIGHT);
                assert!((cap.y - (height - cap.height) / 2.0).abs() < 0.01);
            }
            assert_eq!(frame.regions.len(), 1);
            assert_eq!(frame.regions[0].rect.height, height);
        }
    }

    #[test]
    fn hovering_one_menu_item_does_not_highlight_its_sibling() {
        let palette = ComponentPalette::neutral_dark();
        let items = Element::column(vec![
            MenuItem::new("rename", "Renombrar", palette)
                .icon(Icon::Edit)
                .width(236.0)
                .render(),
            MenuItem::new("archive", "Archivar", palette)
                .icon(Icon::Archive)
                .width(236.0)
                .render(),
        ]);
        let mut tree = UiTree::new(items);
        tree.set_reduced_motion(true);
        tree.layout(Size::new(236.0, 52.0));
        for (y, id) in [(13.0, "rename"), (39.0, "archive")] {
            tree.handle_event(UiEvent::PointerMove(Point::new(42.0, y)));
            let frame = tree.layout(Size::new(236.0, 52.0));
            assert_eq!(frame.hit_test(Point::new(42.0, y)), Some(id));
            assert_eq!(
                frame
                    .commands
                    .iter()
                    .filter(|command| matches!(
                        command,
                        DrawCommand::FillRect { color, rect, .. }
                            if *color == palette.hover && rect.width == 236.0
                    ))
                    .count(),
                1,
            );
        }
    }
}
