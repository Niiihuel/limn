//! Shared action menu built on the retained popover and color transitions.

use crate::components::{
    Component, ComponentPalette, ControlState, Kbd, Popover, Tooltip, lighter,
};
use crate::menu::{DetailRow, action_style};
use crate::{
    Access, Color, Element, FontFamily, ICON_SIZE, Icon, MenuMotion, PopoverPlacement, Role,
    ScrollArea, ScrollbarAppearance, SelectorSizing, Separator, Skeleton, TooltipSpec,
};

// The height of the viewport that holds an error too long for one row.
const ERROR_HEIGHT: f32 = 104.0;

// The scroll viewport remains the full menu content width. Only its children
// give up this much room, so the scrollbar never paints over a row or label.
const SCROLLBAR_GUTTER: f32 = 16.0;

/// One action in a menu. The caller handles `action_id` from `UiAction`.
pub struct DropdownMenuItem {
    pub action_id: String,
    pub label: String,
    pub icon: Option<Icon>,
    pub shortcut: Option<String>,
    pub annotation: Option<String>,
    pub selected: bool,
    pub state: ControlState,
    pub submenu: Option<Vec<DropdownMenuEntry>>,
    pub destructive: Option<Color>,
    description: Option<String>,
    detail_mono: bool,
    status: Option<String>,
    status_width: Option<f32>,
    chevron: bool,
    tooltip: Option<TooltipSpec>,
    truncate: bool,
}

impl DropdownMenuItem {
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            shortcut: None,
            annotation: None,
            selected: false,
            state: ControlState::Enabled,
            submenu: None,
            destructive: None,
            description: None,
            detail_mono: false,
            status: None,
            status_width: None,
            chevron: false,
            tooltip: None,
            truncate: false,
        }
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self.annotation = None;
        self
    }

    /// Plain trailing status text, separate from keyboard shortcuts.
    pub fn annotation(mut self, annotation: impl Into<String>) -> Self {
        self.annotation = Some(annotation.into());
        self.shortcut = None;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable the item; the counterpart of [`disabled`](Self::disabled)
    /// for a state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }

    /// A quiet second line under the label. The item becomes a two-line row
    /// (`SelectorSizing::detail_row_height`) with a column for the check;
    /// it ignores `shortcut`, `annotation`, `submenu` and `destructive`.
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

    /// End a two-line item with a chevron instead of the check column: it
    /// opens another view of the same menu.
    pub fn chevron(mut self) -> Self {
        self.chevron = true;
        self
    }

    /// A hint for the row, for what its label cannot say.
    pub fn tooltip_with(mut self, tooltip: TooltipSpec) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    /// Cut a long label with an ellipsis in a contextual menu, where it
    /// would otherwise wrap; pair it with a hint that says the rest.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// Opens a second menu beside this row on pointer hover or keyboard focus.
    pub fn submenu(mut self, entries: Vec<DropdownMenuEntry>) -> Self {
        self.submenu = Some(entries);
        self
    }

    pub fn destructive(mut self, ink: Color) -> Self {
        self.destructive = Some(ink);
        self
    }
}

/// Static entries do not enter keyboard focus order.
pub enum DropdownMenuEntry {
    Item(DropdownMenuItem),
    Heading(String),
    Separator,
    /// An application-specific control inside the shared menu surface.
    Custom(Box<Element>),
    /// A group heading that folds and unfolds its items.
    Section(DropdownMenuSection),
    /// A muted sentence in place of items: nothing found, or work in course.
    Message(String),
    /// Placeholder rows while the items load. Skeleton IDs derive from `id`.
    Loading { id: String, rows: usize },
    /// A failure too long for a row, in its own short scroll area named `id`.
    Error { id: String, text: String },
}

/// The heading of a group the user can fold: an icon, a label, how many
/// items it holds and a chevron. The host owns `collapsed` and leaves the
/// group's items out of the entries while it is set.
pub struct DropdownMenuSection {
    id: String,
    label: String,
    icon: Option<Icon>,
    count: Option<usize>,
    collapsed: bool,
    width: Option<f32>,
}

impl DropdownMenuSection {
    /// A section reporting `id` when its heading is activated.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            count: None,
            collapsed: false,
            width: None,
        }
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// How many items the section holds, before the chevron.
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Point the chevron right instead of down.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// A fixed width, for a menu whose rows keep clear of a reserved rail.
    /// Values that are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    fn render(self, palette: ComponentPalette) -> Element {
        let sizing = SelectorSizing::COMPACT;
        let mut children = Vec::with_capacity(4);
        if let Some(icon) = self.icon {
            children.push(
                Element::icon(icon)
                    .foreground(palette.muted)
                    .size(Some(sizing.icon_size), Some(sizing.icon_size)),
            );
        }
        children.push(
            Element::truncated_text(self.label)
                .font_family(FontFamily::Sans)
                .font_size(sizing.label_size)
                .foreground(palette.muted)
                .flex(1.0),
        );
        if let Some(count) = self.count {
            children.push(
                Element::text(count.to_string())
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.detail_size)
                    .foreground(palette.muted),
            );
        }
        children.push(
            Element::icon(if self.collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            })
            .foreground(palette.muted)
            .size(Some(sizing.icon_size), Some(sizing.icon_size)),
        );
        palette
            .interactive_surface(
                Element::clickable(self.id, Element::row(children).gap(sizing.gap))
                    .action_menu_item()
                    .accessible(Access::default().expanded(!self.collapsed)),
            )
            .padding(sizing.padding)
            .size(self.width, Some(sizing.row_height))
    }
}

impl From<DropdownMenuSection> for DropdownMenuEntry {
    fn from(section: DropdownMenuSection) -> Self {
        Self::Section(section)
    }
}

// What a two-line item, a section and the state rows occupy, whatever the
// density of the menu's one-line items; `None` for an ordinary row.
fn fixed_entry_height(entry: &DropdownMenuEntry) -> Option<f32> {
    let sizing = SelectorSizing::COMPACT;
    match entry {
        DropdownMenuEntry::Item(item) if item.description.is_some() => {
            Some(sizing.detail_row_height)
        }
        DropdownMenuEntry::Section(_) => Some(sizing.row_height),
        DropdownMenuEntry::Loading { rows, .. } => Some(*rows as f32 * sizing.row_height),
        DropdownMenuEntry::Error { .. } => Some(ERROR_HEIGHT),
        DropdownMenuEntry::Custom(element) => element.style.height,
        _ => None,
    }
}

impl DropdownMenuEntry {
    /// Add caller-owned content without expanding every ordinary menu entry
    /// to the size of a complete element tree.
    pub fn custom(element: Element) -> Self {
        Self::Custom(Box::new(element))
    }
}

impl From<DropdownMenuItem> for DropdownMenuEntry {
    fn from(item: DropdownMenuItem) -> Self {
        Self::Item(item)
    }
}

/// A heading names the next group; every subsequent group receives one
/// divider even when the caller supplied only headings. Explicit dividers
/// are retained, with no empty leading/trailing or duplicate separator rows.
fn group_boundaries(entries: Vec<DropdownMenuEntry>) -> Vec<DropdownMenuEntry> {
    let mut grouped = Vec::with_capacity(entries.len());
    let mut has_content = false;
    for entry in entries {
        match &entry {
            DropdownMenuEntry::Separator => {
                if !has_content {
                    continue;
                }
                has_content = false;
            }
            DropdownMenuEntry::Heading(_) => {
                if has_content {
                    grouped.push(DropdownMenuEntry::Separator);
                }
                has_content = false;
            }
            _ => {
                has_content = true;
            }
        }
        grouped.push(entry);
    }
    if matches!(grouped.last(), Some(DropdownMenuEntry::Separator)) {
        grouped.pop();
    }
    grouped
}

/// Caller-owned open state with one retained, animated popover. Arrow keys,
/// Home/End, Tab, Escape and viewport-aware placement come from `UiTree`.
pub struct DropdownMenu {
    anchor: Element,
    header: Option<Element>,
    footer: Option<Element>,
    entries: Vec<DropdownMenuEntry>,
    open: bool,
    dismiss_id: String,
    width: f32,
    placement: PopoverPlacement,
    palette: ComponentPalette,
    motion: MenuMotion,
    close_animation_ms: Option<u16>,
    open_scale: f32,
    view_key: Option<String>,
    max_visible_items: usize,
    sizing: Option<SelectorSizing>,
    inline: bool,
    context_menu_style: bool,
    hints_in_tooltips: bool,
    // A submenu opens under the pointer: only the tree knows when.
    submenu: bool,
}

impl DropdownMenu {
    pub fn new(
        anchor: Element,
        entries: Vec<DropdownMenuEntry>,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            header: None,
            footer: None,
            entries,
            open,
            dismiss_id: dismiss_id.into(),
            width: 240.0,
            placement: PopoverPlacement::BottomStart,
            palette,
            motion: MenuMotion::default(),
            close_animation_ms: None,
            open_scale: 1.0,
            view_key: None,
            max_visible_items: 8,
            sizing: None,
            inline: false,
            context_menu_style: false,
            hints_in_tooltips: false,
            submenu: false,
        }
    }

    /// Use the editor context menu's shared action-menu surface and density.
    /// Catalog selectors keep their own profile; this style uses shared
    /// keycaps and aligned icon/check columns. Checked actions retain
    /// their checkmark without looking like a second hovered row.
    pub fn context_menu_style(mut self) -> Self {
        self.context_menu_style = true;
        self.width = action_style::WIDTH;
        self.max_visible_items = usize::MAX;
        self.sizing = None;
        self
    }

    /// Keep action labels readable in a deliberately narrow menu. Keyboard
    /// shortcuts and status annotations remain available in the row tooltip;
    /// action IDs, enabled state and keyboard navigation are unchanged.
    pub fn hints_in_tooltips(mut self, enabled: bool) -> Self {
        self.hints_in_tooltips = enabled;
        self
    }

    /// Render the expanded options in normal layout, below the trigger.
    /// Useful inside narrow side panels where a floating menu would obscure
    /// adjacent content. The host owns open state and keyboard dismissal.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// Use the shared selector width, row density and scroll budget. Call
    /// before `width` or `max_visible_items` to override either for the host.
    pub fn sizing(mut self, sizing: SelectorSizing) -> Self {
        self.width = sizing.preferred_width;
        self.max_visible_items = sizing.max_visible_items;
        self.sizing = Some(sizing);
        self
    }

    /// Content kept above the options viewport, such as search and navigation.
    /// It shares the menu's focus scope but never scrolls with the options.
    pub fn header(mut self, header: Element) -> Self {
        self.header = Some(header);
        self
    }

    /// Content kept under the options, below a hairline: an action that
    /// must stay in reach however far the list scrolls.
    pub fn footer(mut self, footer: Element) -> Self {
        self.footer = Some(footer);
        self
    }

    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn motion(mut self, motion: MenuMotion) -> Self {
        self.motion = motion;
        self
    }

    pub fn close_animation_ms(mut self, duration: u16) -> Self {
        self.close_animation_ms = Some(duration);
        self
    }

    pub fn open_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.open_scale = scale.clamp(0.5, 1.25);
        }
        self
    }

    pub fn view_key(mut self, key: impl Into<String>) -> Self {
        self.view_key = Some(key.into());
        self
    }

    pub fn max_visible_items(mut self, count: usize) -> Self {
        self.max_visible_items = count.max(1);
        self
    }
}

impl Component for DropdownMenu {
    fn render(mut self) -> Element {
        if !self.submenu {
            // The trigger is the host's; whether its menu is open is known here.
            self.anchor = self.anchor.accessible(Access::default().expanded(self.open));
        }
        if !self.open {
            return self.anchor;
        }

        self.entries = group_boundaries(self.entries);

        // Headings, separators and custom content occupy vertical space too.
        // Count them for the scroll threshold so a long mixed menu stays
        // bounded and every option remains reachable.
        let row_count = self.entries.len();
        let contextual = self.context_menu_style;
        let row_gap = if contextual { 0.0 } else { 2.0 };
        let row_height = if contextual {
            action_style::ROW_HEIGHT
        } else {
            self.sizing.map_or(36.0, |sizing| sizing.row_height)
        };
        let icon_size = self.sizing.map_or(ICON_SIZE, |sizing| sizing.icon_size);
        // The row content centers the shared 18px keycaps. Horizontal padding
        // is independent from the vertical inset of a compact row.
        let content_height = 22.0;
        let horizontal_padding = if contextual {
            action_style::HORIZONTAL_PADDING
        } else {
            self.sizing.map_or(7.0, |sizing| sizing.padding)
        };
        let vertical_padding = ((row_height - content_height) / 2.0).max(0.0);
        let extra_horizontal = (horizontal_padding - vertical_padding).max(0.0);
        let scrollbar_gutter = if contextual {
            action_style::SCROLLBAR_GUTTER
        } else {
            self.sizing
                .map_or(SCROLLBAR_GUTTER, |sizing| sizing.scrollbar_gutter)
        };
        let action_height = |entry: &DropdownMenuEntry| match entry {
            DropdownMenuEntry::Heading(_) => action_style::HEADER_HEIGHT,
            DropdownMenuEntry::Separator => action_style::SEPARATOR_HEIGHT,
            entry => fixed_entry_height(entry).unwrap_or(row_height),
        };
        let visible_height = if contextual {
            // This is a ceiling, not the list's actual height. Rows are
            // measured with the native font and may wrap next to keycaps.
            // A short list keeps its natural height; a long one scrolls.
            (self.max_visible_items as f32 * row_height)
                .min(action_style::MAX_HEIGHT - action_style::INSET * 2.0)
        } else if let Some(sizing) = self.sizing {
            // Mixed lists can contain short headings, dividers and taller
            // detail rows. Budget the visible prefix itself; multiplying the
            // tallest entry by every row inflates an otherwise compact menu.
            let heights =
                self.entries
                    .iter()
                    .take(self.max_visible_items)
                    .map(|entry| match entry {
                        DropdownMenuEntry::Heading(_) => sizing.header_height,
                        DropdownMenuEntry::Separator => 1.0,
                        entry => fixed_entry_height(entry).unwrap_or(row_height),
                    });
            heights.sum::<f32>() + self.max_visible_items.saturating_sub(1) as f32 * 2.0
        } else {
            self.max_visible_items as f32 * (row_height + 2.0) - 2.0
        };
        let reserve_icon = self
            .entries
            .iter()
            .any(|entry| matches!(entry, DropdownMenuEntry::Item(item) if item.icon.is_some()));
        let reserve_check = self
            .entries
            .iter()
            .any(|entry| matches!(entry, DropdownMenuEntry::Item(item) if item.selected));
        let reserve_submenu = contextual
            && self.entries.iter().any(
                |entry| matches!(entry, DropdownMenuEntry::Item(item) if item.submenu.is_some()),
            );
        let scrolls = if contextual {
            self.entries.iter().map(action_height).sum::<f32>() > visible_height
        } else {
            row_count > self.max_visible_items
        };
        let inset = if contextual { action_style::INSET } else { 6.0 };
        let viewport_width = (self.width - inset * 2.0).max(0.0);
        let row_width = viewport_width - if scrolls { scrollbar_gutter } else { 0.0 };
        let palette = self.palette;
        let rows = self
            .entries
            .into_iter()
            .map(|entry| match entry {
                DropdownMenuEntry::Heading(label) if contextual => Element::row(vec![
                    Element::spacer().size(Some(action_style::HORIZONTAL_PADDING), None),
                    Element::truncated_text(label)
                        .foreground(palette.muted)
                        .font_family(FontFamily::Sans)
                        .font_size(action_style::SHORTCUT_FONT_SIZE)
                        .flex(1.0)
                        .size(None, Some(action_style::HEADER_HEIGHT)),
                ])
                .size(None, Some(action_style::HEADER_HEIGHT)),
                DropdownMenuEntry::Heading(label) => Element::truncated_text(label)
                    .foreground(palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(12.0)
                    .padding(8.0)
                    .size(
                        Some(row_width),
                        Some(self.sizing.map_or(28.0, |sizing| sizing.header_height)),
                    ),
                DropdownMenuEntry::Separator if contextual => Element::panel(
                    Element::spacer()
                        .background(palette.border)
                        .size(None, Some(1.0)),
                )
                .padding(4.0)
                .size(None, Some(action_style::SEPARATOR_HEIGHT)),
                DropdownMenuEntry::Separator => Element::spacer()
                    .background(palette.border)
                    .size(Some(row_width), Some(1.0)),
                // The menu knows whether it scrolls; a custom row spans the
                // same width as its neighbours so its hover reaches the edge.
                DropdownMenuEntry::Custom(element) => {
                    let height = element.style.height;
                    element.size(Some(row_width), height)
                }
                DropdownMenuEntry::Section(section) => {
                    let section = section.render(palette);
                    if contextual || section.style.width.is_some() {
                        section
                    } else {
                        section.size(Some(row_width), Some(SelectorSizing::COMPACT.row_height))
                    }
                }
                DropdownMenuEntry::Message(text) => {
                    state_text(text, palette).size((!contextual).then_some(row_width), None)
                }
                DropdownMenuEntry::Loading { id, rows } => {
                    let sizing = SelectorSizing::COMPACT;
                    Element::column(
                        (0..rows)
                            .map(|index| {
                                Element::panel(
                                    Skeleton::block(
                                        format!("{id}:{index}"),
                                        palette,
                                        (row_width - sizing.padding * 2.0).max(0.0),
                                        sizing.icon_size,
                                    )
                                    .render(),
                                )
                                .padding(sizing.padding)
                                .size(Some(row_width), Some(sizing.row_height))
                            })
                            .collect(),
                    )
                }
                DropdownMenuEntry::Error { id, text } => ScrollArea::new(
                    id,
                    state_text(text, palette)
                        .size(Some((row_width - scrollbar_gutter).max(0.0)), None),
                )
                .render()
                .size(Some(row_width), Some(ERROR_HEIGHT)),
                DropdownMenuEntry::Item(item) if item.description.is_some() => {
                    let enabled = item.state == ControlState::Enabled;
                    let id = item.action_id.clone();
                    let row = DetailRow {
                        action_id: item.action_id,
                        label: item.label,
                        description: item.description.unwrap_or_default(),
                        icon: item.icon,
                        mono: item.detail_mono,
                        status: item.status,
                        status_width: item.status_width,
                        chevron: item.chevron,
                        selected: Some(item.selected),
                        enabled,
                    }
                    .render(palette);
                    let row = if contextual {
                        row
                    } else {
                        row.size(Some(row_width), Some(SelectorSizing::COMPACT.detail_row_height))
                    };
                    match item.tooltip {
                        Some(tooltip) => tooltip.wrap(row, id, palette),
                        None => row,
                    }
                }
                DropdownMenuEntry::Item(mut item) => {
                    let tooltip_hint = if self.hints_in_tooltips
                        && (item.shortcut.is_some() || item.annotation.is_some())
                    {
                        let title = match item.annotation.take() {
                            Some(annotation) => format!("{} · {annotation}", item.label),
                            None => item.label.clone(),
                        };
                        Some((title, item.shortcut.take()))
                    } else {
                        None
                    };
                    let enabled = item.state == ControlState::Enabled;
                    let has_submenu = enabled && item.submenu.is_some();
                    let reserve_submenu = reserve_submenu || has_submenu;
                    // Flex children with a natural text width can paint past a
                    // fixed row. Budget the trailing text against the row's
                    // real inner width, including reserved icons and gaps.
                    let control_count = usize::from(reserve_icon)
                        + usize::from(reserve_check)
                        + usize::from(reserve_submenu);
                    let inner_width = (row_width - 2.0 * horizontal_padding).max(0.0);
                    let control_width = control_count as f32 * icon_size;
                    let preferred_gap = if contextual {
                        action_style::GAP
                    } else {
                        self.sizing.map_or(9.0, |sizing| sizing.gap)
                    };
                    let gap = if control_count == 0 {
                        preferred_gap
                    } else {
                        preferred_gap
                            .min(((inner_width - control_width) / control_count as f32).max(0.0))
                    };
                    let main_width =
                        (inner_width - control_width - control_count as f32 * gap).max(0.0);
                    let shortcut_limit = (main_width - 40.0 - gap).max(0.0);
                    let annotation_width = item
                        .annotation
                        .as_ref()
                        .map(|value| {
                            (value.chars().count() as f32 * 7.0 + 12.0)
                                .min(140.0)
                                .min((main_width - 40.0 - gap).max(0.0))
                        })
                        .filter(|width| *width >= 28.0);
                    // The label alone names the row: its keys are not part of what it is called.
                    let mut access = Access::new(Role::MenuItem).name(&item.label);
                    if let Some(annotation) = &item.annotation {
                        access = access.description(annotation);
                    }
                    // A menu with a check column is a set of choices.
                    if reserve_check {
                        access = access.selected(item.selected);
                    }
                    let submenu_id = item.action_id.clone();
                    let submenu = if enabled { item.submenu } else { None };
                    let ink = item.destructive.unwrap_or(palette.text);
                    let mut children = Vec::with_capacity(5);
                    if reserve_icon {
                        children.push(
                            if let Some(icon) = item.icon {
                                Element::icon(icon).foreground(if enabled {
                                    ink
                                } else {
                                    palette.muted
                                })
                            } else {
                                Element::spacer()
                            }
                            .size(
                                Some(icon_size),
                                if contextual {
                                    None
                                } else {
                                    Some(content_height)
                                },
                            )
                            .min_size(icon_size, content_height),
                        );
                    }
                    let label = (if contextual && !item.truncate {
                        Element::text(item.label)
                    } else {
                        Element::truncated_text(item.label)
                    })
                    .foreground(if enabled { ink } else { palette.muted })
                    .font_family(FontFamily::Sans)
                    .font_size(if contextual {
                        action_style::LABEL_FONT_SIZE
                    } else {
                        self.sizing.map_or(13.0, |sizing| sizing.label_size)
                    });
                    children.push(if contextual {
                        // Center the measured text block, including a single
                        // short line, while allowing long labels to wrap.
                        Element::column(vec![
                            Element::spacer().flex(1.0),
                            label,
                            Element::spacer().flex(1.0),
                        ])
                        .flex(1.0)
                        .min_size(40.0, content_height)
                    } else {
                        label.flex(1.0).size(None, Some(content_height))
                    });
                    if let Some(shortcut) = item.shortcut {
                        // Keep complete keycaps and let an action label wrap
                        // at its measured remaining width. Only a very narrow
                        // viewport omits the complete chord.
                        children.push(
                            Kbd::new(
                                shortcut,
                                ComponentPalette {
                                    text: if enabled { ink } else { palette.muted },
                                    ..palette
                                },
                            )
                            .max_width(shortcut_limit)
                            .render(),
                        );
                    } else if let (Some(annotation), Some(annotation_width)) =
                        (item.annotation, annotation_width)
                    {
                        children.push(
                            Element::truncated_text(annotation)
                                .foreground(palette.muted)
                                .font_family(FontFamily::Sans)
                                .font_size(if contextual {
                                    crate::shortcut::FONT_SIZE
                                } else {
                                    self.sizing.map_or(12.0, |sizing| sizing.detail_size)
                                })
                                .size(Some(annotation_width), Some(content_height)),
                        );
                    }
                    if reserve_check {
                        children.push(
                            if item.selected {
                                Element::icon(Icon::Check).foreground(if enabled {
                                    palette.text
                                } else {
                                    palette.muted
                                })
                            } else {
                                Element::spacer()
                            }
                            .size(
                                Some(icon_size),
                                if contextual {
                                    None
                                } else {
                                    Some(content_height)
                                },
                            )
                            .min_size(icon_size, content_height),
                        );
                    }
                    if reserve_submenu {
                        children.push(
                            if has_submenu {
                                Element::icon(Icon::ChevronRight).foreground(palette.muted)
                            } else {
                                Element::spacer()
                            }
                            .size(
                                Some(icon_size),
                                if contextual {
                                    None
                                } else {
                                    Some(content_height)
                                },
                            )
                            .min_size(icon_size, content_height),
                        );
                    }
                    let visual = Element::row(children).gap(gap).clip_children();
                    let visual = if extra_horizontal > 0.0 {
                        Element::row(vec![
                            Element::spacer().size(Some(extra_horizontal), Some(content_height)),
                            visual.flex(1.0),
                            Element::spacer().size(Some(extra_horizontal), Some(content_height)),
                        ])
                        .size(
                            None,
                            if contextual {
                                None
                            } else {
                                Some(content_height)
                            },
                        )
                    } else {
                        visual
                    };
                    let row = if enabled {
                        Element::clickable(item.action_id, visual).accessible(access)
                    } else if item.action_id.is_empty() {
                        // Nothing to know it by: a disabled row with no action.
                        Element::panel(visual)
                    } else {
                        Element::panel(visual).accessible(access.id(item.action_id).disabled(true))
                    };
                    // The popover paints one continuous surface. Resting rows
                    // are transparent so their fills cannot stack with that
                    // surface while the whole menu fades in or out.
                    let row = if item.selected && !contextual {
                        row.background(palette.selected)
                    } else {
                        row
                    };
                    let row = palette
                        .interactive_surface(row)
                        .hover_background(if item.selected && !contextual {
                            lighter(palette.selected, 8)
                        } else {
                            palette.hover
                        })
                        .pressed_background(if item.selected && !contextual {
                            lighter(palette.selected, 14)
                        } else {
                            palette.selected
                        })
                        .transition_ms(self.motion.item_ms)
                        .padding(vertical_padding)
                        .size(Some(row_width), Some(row_height));
                    let row = if contextual {
                        let row = row
                            .size(None, None)
                            .min_size(0.0, row_height)
                            .radius(action_style::row_radius())
                            .transition_ms(0);
                        if enabled { row.action_menu_item() } else { row }
                    } else {
                        row
                    };
                    let row = if let Some((title, shortcut)) = tooltip_hint {
                        let mut tooltip = Tooltip::new(row, &submenu_id, title, palette);
                        if let Some(shortcut) = shortcut {
                            tooltip = tooltip.shortcut(shortcut);
                        }
                        tooltip.render()
                    } else if let Some(tooltip) = item.tooltip {
                        tooltip.wrap(row, submenu_id.clone(), palette)
                    } else {
                        row
                    };
                    if let Some(entries) = submenu {
                        let mut submenu = DropdownMenu::new(
                            row,
                            entries,
                            true,
                            format!("{}:{submenu_id}", self.dismiss_id),
                            palette,
                        );
                        if contextual {
                            submenu = submenu.context_menu_style();
                        } else if let Some(sizing) = self.sizing {
                            submenu = submenu.sizing(sizing);
                        }
                        submenu.submenu = true;
                        submenu
                            .width(self.width)
                            .hints_in_tooltips(self.hints_in_tooltips)
                            .placement(PopoverPlacement::RightStart)
                            .motion(self.motion)
                            .max_visible_items(self.max_visible_items)
                            .render()
                            .popover_on_hover()
                    } else {
                        row
                    }
                }
            })
            .collect();
        let list = Element::column(rows).gap(row_gap);
        // Even a menu below the nominal cap may be taller than the space
        // above/below its anchor. Keep a scroll viewport ready for that final
        // allocation; the reserved rail appears only if it actually overflows.
        let scroll_container = contextual || scrolls;
        let content = if scroll_container {
            let content = ScrollArea::new(format!("{}:options", self.dismiss_id), list)
                .appearance(ScrollbarAppearance::reserved().rail_width(scrollbar_gutter))
                .tint(palette.muted)
                .render();
            if contextual {
                content.size(None, None).max_height(visible_height)
            } else {
                content.size(Some(viewport_width), Some(visible_height))
            }
        } else {
            list
        };
        let content = if let Some(header) = self.header {
            // Retain the preferred viewport height when measuring the menu,
            // but let the options shrink below a fixed header when placement
            // has to clamp the surface to a shorter window.
            let content = if scroll_container {
                content.flex(1.0)
            } else {
                content
            };
            Element::column(vec![header, content]).gap(2.0)
        } else {
            content
        };
        let content = if let Some(footer) = self.footer {
            // The options give way first when placement shortens the surface,
            // so the footer is never the part that is cut off.
            Element::column(vec![
                content.flex(1.0),
                Separator::horizontal(palette).render(),
                footer,
            ])
            .gap(SelectorSizing::COMPACT.gap)
        } else {
            content
        };
        if contextual {
            let surface = action_style::surface(content, palette, self.width, None);
            if self.inline {
                return Element::column(vec![self.anchor, surface])
                    .gap(4.0)
                    .size(Some(self.width), None)
                    .menu_navigation();
            }
            let mut popover =
                Element::popover(self.anchor, surface, self.dismiss_id, self.placement)
                    .popover_animation_ms(self.motion.open_ms)
                    .popover_open_scale(self.open_scale);
            if let Some(duration) = self.close_animation_ms {
                popover = popover.popover_close_animation_ms(duration);
            }
            if let Some(key) = self.view_key {
                popover = popover.popover_view_key(key);
            }
            return popover;
        }
        if self.inline {
            return Element::column(vec![
                self.anchor,
                Element::panel(content)
                    .padding(6.0)
                    .size(Some(self.width), None)
                    .background(palette.surface)
                    .border(palette.border, 1.0)
                    .radius(crate::components::surface_radius())
                    .clip_children(),
            ])
            .gap(4.0)
            .size(Some(self.width), None)
            .menu_navigation();
        }
        let mut popover = Popover::new(
            self.anchor,
            content,
            true,
            self.dismiss_id,
            ComponentPalette {
                raised: palette.surface,
                ..palette
            },
        )
        .width(self.width)
        .placement(self.placement)
        .animation_ms(self.motion.open_ms)
        .open_scale(self.open_scale);
        if let Some(duration) = self.close_animation_ms {
            popover = popover.close_animation_ms(duration);
        }
        if let Some(key) = self.view_key {
            popover = popover.view_key(key);
        }
        popover.render()
    }
}

// The muted sentence of a message or an error.
fn state_text(text: String, palette: ComponentPalette) -> Element {
    let sizing = SelectorSizing::COMPACT;
    Element::text(text)
        .font_family(FontFamily::Sans)
        .font_size(sizing.label_size)
        .foreground(palette.muted)
        .padding(sizing.padding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ControlSize, DrawCommand, HitKind, Key, Point, SearchField, Size, Tooltip,
        TooltipPlacement, UiAction, UiEvent, UiTree,
    };

    fn region(frame: &crate::Frame, id: &str) -> crate::Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind != HitKind::Scrollbar)
            .unwrap_or_else(|| panic!("missing {id}"))
            .rect
    }

    fn open_menu(menu: DropdownMenu) -> UiTree {
        let mut tree = UiTree::new(menu.render());
        tree.set_reduced_motion(true);
        tree
    }

    fn trigger() -> Element {
        Element::button("trigger", "Open").size(Some(80.0), Some(28.0))
    }

    #[test]
    fn the_trigger_says_whether_the_menu_is_open_and_each_item_what_it_is() {
        let palette = ComponentPalette::neutral_dark();
        let menu = |open: bool, contextual: bool| {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("wrap", "Word wrap").shortcut("Alt+Z").selected(true).into(),
                    DropdownMenuItem::new("map", "Minimap").annotation("Off").into(),
                    DropdownMenuItem::new("zen", "Zen mode").disabled().into(),
                    DropdownMenuItem::new("more", "More")
                        .submenu(vec![DropdownMenuItem::new("deep", "Deep").into()])
                        .into(),
                    DropdownMenuItem::new("model", "Opus").description("The largest").selected(true).into(),
                    DropdownMenuItem::new("gone", "Retired").description("No longer served").disabled().into(),
                    DropdownMenuSection::new("group", "Group").collapsed(true).into(),
                ],
                open,
                "dismiss",
                palette,
            )
            .max_visible_items(12);
            if contextual { menu.context_menu_style() } else { menu }
        };
        let closed = open_menu(menu(false, false)).layout(Size::new(520.0, 640.0));
        assert_eq!(closed.access.len(), 1);
        assert_eq!((closed.access[0].role, closed.access[0].name.as_str()), (Role::Button, "Open"));
        assert_eq!(closed.access[0].expanded, Some(false));
        for contextual in [false, true] {
            let frame = open_menu(menu(true, contextual)).layout(Size::new(520.0, 640.0));
            let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));
            assert_eq!(node("trigger").expanded, Some(true));
            let wrap = node("wrap");
            assert_eq!((wrap.role, wrap.name.as_str(), wrap.selected), (Role::MenuItem, "Word wrap", Some(true)));
            assert!(wrap.actionable && !wrap.disabled);
            let map = node("map");
            assert_eq!((map.name.as_str(), map.description.as_deref(), map.selected), ("Minimap", Some("Off"), Some(false)));
            let zen = node("zen");
            assert_eq!((zen.role, zen.name.as_str(), zen.selected), (Role::MenuItem, "Zen mode", Some(false)));
            assert!(zen.disabled && !zen.actionable);
            // Its submenu opens under the pointer: the item does not claim to know.
            // Outside the contextual style the shared popover speaks for its anchor.
            let more = node("more");
            assert_eq!((more.role, more.name.as_str()), (Role::MenuItem, "More"));
            assert!(!contextual || more.expanded.is_none());
            let model = node("model");
            assert_eq!((model.role, model.name.as_str()), (Role::MenuItem, "Opus"));
            assert_eq!((model.description.as_deref(), model.selected), (Some("The largest"), Some(true)));
            let gone = node("gone");
            assert_eq!((gone.role, gone.name.as_str(), gone.selected), (Role::MenuItem, "Retired", Some(false)));
            assert!(gone.disabled && !gone.actionable);
            assert_eq!((node("group").role, node("group").expanded), (Role::Button, Some(false)));
        }
    }

    #[test]
    fn the_footer_stays_under_a_hairline_while_the_options_scroll() {
        let palette = ComponentPalette::neutral_dark();
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                (0..40)
                    .map(|index| {
                        DropdownMenuItem::new(format!("item:{index}"), format!("Item {index}"))
                            .into()
                    })
                    .collect(),
                true,
                "dismiss",
                palette,
            )
            .footer(Element::button("connect", "Connect").size(None, Some(28.0)));
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let size = Size::new(640.0, 420.0);
            let frame = tree.layout(size);
            let footer = region(&frame, "connect");
            let rule = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if *color == palette.border && rect.height == 1.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("a hairline above the footer");
            assert_eq!(footer.y - (rule.y + 1.0), SelectorSizing::COMPACT.gap);
            assert!(footer.y + footer.height <= size.height, "the footer is in the window");
            let first = region(&frame, "item:1");
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(first.x + 20.0, first.y + 4.0),
                delta_y: -200.0,
            });
            let scrolled = tree.layout(size);
            assert_eq!(region(&scrolled, "connect"), footer);
            assert!(
                scrolled.regions.iter().all(|row| row.id != "item:1")
                    || region(&scrolled, "item:1").y != first.y,
                "the options did scroll"
            );
        }
    }

    #[test]
    fn a_two_line_item_has_the_selector_metrics_in_both_menu_styles() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = SelectorSizing::COMPACT;
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("model", "Model")
                        .icon(Icon::Server)
                        .description("large-v3")
                        .detail_mono(true)
                        .status("Verified")
                        .selected(true)
                        .into(),
                    DropdownMenuItem::new("effort", "Effort")
                        .icon(Icon::Brain)
                        .description("Automatic")
                        .chevron()
                        .into(),
                    DropdownMenuItem::new("off", "Unavailable")
                        .description("No credential")
                        .enabled(false)
                        .into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(300.0);
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let frame = tree.layout(Size::new(640.0, 480.0));
            let (model, effort) = (region(&frame, "model"), region(&frame, "effort"));
            assert_eq!(model.height, sizing.detail_row_height);
            assert_eq!(effort.height, sizing.detail_row_height);
            let text = |label: &str| {
                frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text {
                            text,
                            position,
                            font_size,
                            font_family,
                            color,
                        } if text == label => Some((*position, *font_size, *font_family, *color)),
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing {label}"))
            };
            let (label, label_size, _, label_ink) = text("Model");
            let (detail, detail_size, detail_family, detail_ink) = text("large-v3");
            assert_eq!((label_size, label_ink), (sizing.label_size, palette.text));
            assert_eq!(
                (detail_size, detail_family, detail_ink),
                (sizing.detail_size, FontFamily::Monospace, palette.muted)
            );
            assert_eq!(label.x, model.x + sizing.padding + sizing.icon_size + sizing.gap);
            assert_eq!(label.x, detail.x);
            assert!(detail.y > label.y);
            assert!(text("Verified").0.x > detail.x);
            let icons: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Icon { icon, rect, tint } => Some((*icon, *rect, *tint)),
                    _ => None,
                })
                .collect();
            let trailing = |icon: Icon, row: crate::Rect| {
                let (_, rect, color) = icons.iter().find(|(kind, ..)| *kind == icon).unwrap();
                assert_eq!(
                    rect.x + rect.width,
                    row.x + row.width - sizing.padding,
                    "{icon:?} ends the row"
                );
                *color
            };
            assert_eq!(trailing(Icon::Check, model), palette.text);
            assert_eq!(trailing(Icon::ChevronRight, effort), palette.muted);
            assert!(
                frame.regions.iter().all(|row| row.id != "off"),
                "a disabled item is passive"
            );
            assert_eq!(text("Unavailable").3, palette.muted);
        }
    }

    #[test]
    fn a_section_reports_its_toggle_and_turns_its_chevron_when_collapsed() {
        let palette = ComponentPalette::neutral_dark();
        for collapsed in [false, true] {
            let mut entries = vec![
                DropdownMenuSection::new("toggle", "Provider")
                    .icon(Icon::Server)
                    .count(2)
                    .collapsed(collapsed)
                    .into(),
            ];
            if !collapsed {
                entries.push(DropdownMenuItem::new("first", "First").into());
            }
            let mut tree = open_menu(
                DropdownMenu::new(trigger(), entries, true, "dismiss", palette)
                    .context_menu_style(),
            );
            let frame = tree.layout(Size::new(640.0, 480.0));
            let section = region(&frame, "toggle");
            assert_eq!(section.height, SelectorSizing::COMPACT.row_height);
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, .. } if text == "2" && *color == palette.muted)));
            let chevron = if collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            };
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Icon { icon, .. } if *icon == chevron)));
            let point = Point::new(section.x + 20.0, section.y + 10.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: "toggle".into() }]
            );
        }
        let fixed = DropdownMenuSection::new("toggle", "Provider").width(f32::NAN);
        assert_eq!(fixed.render(palette).style.width, None);
        let fixed = DropdownMenuSection::new("toggle", "Provider").width(-1.0);
        assert_eq!(fixed.render(palette).style.width, Some(0.0));
    }

    #[test]
    fn state_entries_paint_a_message_skeleton_rows_and_a_bounded_error() {
        let palette = ComponentPalette::neutral_dark();
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuEntry::Message("Nothing here yet.".into()),
                    DropdownMenuEntry::Loading {
                        id: "loading".into(),
                        rows: 3,
                    },
                    DropdownMenuEntry::Error {
                        id: "failure".into(),
                        text: "The repository could not be read. ".repeat(30),
                    },
                    DropdownMenuItem::new("retry", "Retry").into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(280.0);
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let frame = tree.layout(Size::new(640.0, 700.0));
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, color, font_size, .. }
                    if text == "Nothing here yet."
                        && *color == palette.muted
                        && *font_size == SelectorSizing::COMPACT.label_size)));
            let retry = region(&frame, "retry");
            let error = frame
                .regions
                .iter()
                .find(|row| row.id == "failure" && row.kind == HitKind::Scroll)
                .expect("the error scrolls in its own viewport")
                .rect;
            assert!(error.height <= ERROR_HEIGHT + 0.01, "{error:?}");
            assert!(retry.y >= error.y, "the action after a long error stays in reach");
            assert!(retry.y + retry.height <= 700.0);
        }
    }

    #[test]
    fn an_item_tooltip_wraps_the_row_and_a_bad_status_width_is_ignored() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = open_menu(
            DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("copy", "Copy address")
                        .tooltip_with(TooltipSpec::new("Copy").description("Copies the address."))
                        .into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .context_menu_style(),
        );
        let size = Size::new(640.0, 480.0);
        let frame = tree.layout(size);
        let copy = region(&frame, "copy");
        tree.handle_event(UiEvent::PointerMove(Point::new(copy.x + 8.0, copy.y + 8.0)));
        let hinted = tree.layout_at(
            size,
            web_time::Instant::now() + std::time::Duration::from_secs(2),
        );
        assert!(hinted.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.contains("Copies the address"))));
        let item = DropdownMenuItem::new("a", "A").status_width(f32::NAN);
        assert_eq!(item.status_width, None);
        assert_eq!(item.status_width(-3.0).status_width, Some(0.0));
    }

    #[test]
    fn a_truncated_contextual_item_keeps_one_line_where_the_label_would_wrap() {
        let palette = ComponentPalette::neutral_dark();
        let label = "Copy the connection settings of this page";
        let rows = |truncate: bool| {
            let item = DropdownMenuItem::new("copy", label).icon(Icon::Code);
            let mut tree = open_menu(
                DropdownMenu::new(
                    trigger(),
                    vec![if truncate { item.truncate() } else { item }.into()],
                    true,
                    "dismiss",
                    palette,
                )
                .context_menu_style()
                .width(140.0),
            );
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
            let frame = tree.layout(Size::new(640.0, 480.0));
            let texts: Vec<String> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } if text != "Open" => Some(text.clone()),
                    _ => None,
                })
                .collect();
            (region(&frame, "copy").height, texts)
        };
        let (wrapped_height, _) = rows(false);
        let (height, texts) = rows(true);
        assert!(height < wrapped_height, "{height} < {wrapped_height}");
        assert_eq!(texts.len(), 1);
        assert!(texts[0].ends_with('…'), "{texts:?}");
    }

    #[test]
    fn narrow_menu_can_move_hints_to_tooltips_without_changing_actions_or_disabled_state() {
        let mut tree = UiTree::new(
            DropdownMenu::new(
                Element::button("trigger", "Acciones").size(Some(24.0), Some(24.0)),
                vec![
                    DropdownMenuItem::new("replace", "Reemplazar")
                        .shortcut("Ctrl+H")
                        .into(),
                    DropdownMenuItem::new("save", "Guardar")
                        .annotation("Sin cambios")
                        .disabled()
                        .into(),
                ],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .context_menu_style()
            .width(224.0)
            .hints_in_tooltips(true)
            .render(),
        );
        tree.set_reduced_motion(true);
        let size = Size::new(240.0, 360.0);
        let frame = tree.layout(size);
        for label in ["Reemplazar", "Guardar"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == label
            )));
        }
        assert!(
            !tree.focus("save"),
            "a status tooltip must not enable its action"
        );
        assert!(tree.focus("replace"));
        let hinted = tree.layout_at(
            size,
            web_time::Instant::now() + std::time::Duration::from_secs(1),
        );
        for key in [
            if cfg!(target_os = "macos") {
                "⌃"
            } else {
                "Ctrl"
            },
            "H",
        ] {
            assert!(hinted.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == key
            )));
        }
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "replace".into()
            }]
        );
        let save = frame
            .regions
            .iter()
            .find(|region| region.id == "save")
            .unwrap();
        tree.handle_event(UiEvent::PointerMove(Point::new(
            save.rect.x + 4.0,
            save.rect.y + save.rect.height / 2.0,
        )));
        let hinted = tree.layout_at(
            size,
            web_time::Instant::now() + std::time::Duration::from_secs(2),
        );
        assert!(hinted.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.contains("Sin cambios")
        )));
    }

    #[test]
    fn grouped_menus_paint_one_divider_between_groups_without_empty_edges() {
        for contextual in [false, true] {
            let mut menu = DropdownMenu::new(
                Element::button("trigger", "Acciones").size(Some(100.0), Some(28.0)),
                vec![
                    DropdownMenuEntry::Separator,
                    DropdownMenuEntry::Heading("Archivo".into()),
                    DropdownMenuItem::new("save", "Guardar")
                        .shortcut("Ctrl+S")
                        .into(),
                    DropdownMenuEntry::Heading("Selección".into()),
                    DropdownMenuItem::new("next", "Siguiente")
                        .shortcut("Ctrl+D")
                        .into(),
                    DropdownMenuEntry::Separator,
                    DropdownMenuEntry::Separator,
                    DropdownMenuEntry::Heading("Lenguaje".into()),
                    DropdownMenuItem::new("complete", "Completar")
                        .shortcut("Ctrl+Space")
                        .into(),
                    DropdownMenuEntry::Separator,
                ],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            );
            if contextual {
                menu = menu.context_menu_style();
            }
            let mut tree = UiTree::new(menu.render());
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(640.0, 600.0));
            let row = |id: &str| {
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == id)
                    .unwrap()
                    .rect
            };
            let dividers: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if *color == ComponentPalette::neutral_dark().border
                            && rect.height == 1.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(dividers.len(), 2, "each group has exactly one divider");
            for (divider, (previous, next)) in dividers
                .iter()
                .zip([("save", "next"), ("next", "complete")])
            {
                let previous = row(previous);
                let next = row(next);
                assert!(divider.y >= previous.y + previous.height);
                assert!(divider.y + divider.height <= next.y);
            }
            assert!(
                frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { text, .. } if text == "Space"
                )),
                "last group remains complete and reachable"
            );
        }
    }

    #[test]
    fn checked_actions_use_checkmarks_and_only_the_active_row_is_highlighted() {
        let palette = ComponentPalette::neutral_dark();
        let menu = DropdownMenu::new(
            Element::button("trigger", "Ver"),
            vec![
                DropdownMenuItem::new("files", "Archivos")
                    .shortcut("Ctrl+Shift+E")
                    .selected(true)
                    .into(),
                DropdownMenuItem::new("search", "Buscar")
                    .shortcut("Ctrl+Shift+F")
                    .into(),
                DropdownMenuItem::new("sidebar", "Mostrar barra lateral")
                    .selected(true)
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .context_menu_style()
        .width(304.0)
        .render();
        let mut tree = UiTree::new(menu);
        tree.set_reduced_motion(true);
        let viewport = Size::new(500.0, 350.0);
        let frame = tree.layout(viewport);
        let rows: Vec<_> = ["files", "search", "sidebar"]
            .iter()
            .map(|id| {
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == *id)
                    .unwrap()
                    .rect
            })
            .collect();
        assert!(
            rows.iter()
                .all(|row| row.height == action_style::ROW_HEIGHT)
        );
        assert_eq!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    DrawCommand::Icon {
                        icon: Icon::Check,
                        ..
                    }
                ))
                .count(),
            2
        );
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if rows.contains(rect) && (*color == palette.hover || *color == palette.selected)
        )), "checked values must not paint resting selection fills");
        for row in &rows {
            let point = Point::new(row.x + 20.0, row.y + row.height / 2.0);
            tree.handle_event(UiEvent::PointerMove(point));
            let frame = tree.layout(viewport);
            let fills: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect {
                        rect,
                        color,
                        radius,
                        ..
                    } if rows.contains(rect) && *color == palette.hover => Some((*rect, *radius)),
                    _ => None,
                })
                .collect();
            assert_eq!(fills, vec![(*row, action_style::row_radius())]);
        }
    }

    #[test]
    fn action_menu_keycaps_share_a_trailing_column_with_checked_rows() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Ver"),
            vec![
                DropdownMenuItem::new("files", "Archivos")
                    .shortcut("Ctrl+Shift+E")
                    .selected(true)
                    .into(),
                DropdownMenuItem::new("search", "Buscar")
                    .shortcut("Ctrl+Shift+F")
                    .into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .context_menu_style()
        .width(304.0)
        .render();
        let frame = UiTree::new(menu).layout(Size::new(500.0, 300.0));
        let positions: Vec<_> = ["E", "F"]
            .iter()
            .map(|key| {
                frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text { text, position, .. } if text == *key => Some(*position),
                        _ => None,
                    })
                    .expect("menu keycap")
            })
            .collect();
        assert_eq!(positions[0].x, positions[1].x);
    }

    #[test]
    fn action_labels_wrap_next_to_complete_keycaps_and_scroll_uses_their_real_height() {
        let label = "Guardar todos los cambios del archivo seleccionado";
        for width in [180.0, 520.0] {
            for count in [1, 12] {
                let menu = DropdownMenu::new(
                    Element::button("trigger", "Acciones").size(Some(100.0), Some(28.0)),
                    (0..count)
                        .map(|index| {
                            DropdownMenuItem::new(format!("wrapped:{index}"), label)
                                .icon(Icon::File)
                                .selected(index == 0)
                                .shortcut("Ctrl+Shift+Z")
                                .into()
                        })
                        .collect(),
                    true,
                    "dismiss",
                    ComponentPalette::neutral_dark(),
                )
                .context_menu_style()
                .render();
                let mut tree = UiTree::new(menu);
                tree.set_reduced_motion(true);
                tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 9.0);
                let viewport = Size::new(width, 900.0);
                let frame = tree.layout(viewport);
                let surface = frame
                    .regions
                    .iter()
                    .find(|row| row.id == "popover_surface:dismiss")
                    .unwrap()
                    .rect;
                assert_eq!(surface.width, action_style::WIDTH.min(width - 16.0));
                assert!(surface.height <= action_style::MAX_HEIGHT);
                let first = frame
                    .regions
                    .iter()
                    .find(|row| row.id == "wrapped:0")
                    .unwrap()
                    .rect;
                assert!(first.height > action_style::ROW_HEIGHT);
                let lines: Vec<_> = frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::Text {
                            text,
                            position,
                            font_size,
                            ..
                        } if *font_size == action_style::LABEL_FONT_SIZE
                            && position.y >= first.y
                            && position.y < first.y + first.height =>
                        {
                            Some((text, position))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    lines
                        .iter()
                        .map(|(text, _)| text.as_str())
                        .collect::<String>(),
                    label
                );
                let caps: Vec<_> = frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::FillRect { rect, radius, .. }
                            if *radius == crate::shortcut::radius()
                                && rect.height == crate::shortcut::HEIGHT
                                && rect.y >= first.y
                                && rect.y + rect.height <= first.y + first.height =>
                        {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(caps.len(), if width > 200.0 { 3 } else { 0 });
                let label_right = caps.first().map_or(first.x + first.width - 24.0, |cap| {
                    cap.x - action_style::GAP
                });
                for (text, position) in lines {
                    assert!(position.x >= first.x + action_style::HORIZONTAL_PADDING);
                    assert!(
                        position.x + text.chars().count() as f32 * 9.0 <= label_right + 0.1,
                        "label {text:?} overlaps trailing controls at {width}px"
                    );
                }
                for cap in caps {
                    assert!(cap.x >= first.x && cap.x + cap.width <= first.x + first.width);
                }
                if count == 1 {
                    assert_eq!(surface.height, first.height + action_style::INSET * 2.0);
                    assert!(
                        !frame.regions.iter().any(
                            |row| row.id == "dismiss:options" && row.kind == HitKind::Scrollbar
                        )
                    );
                } else {
                    assert!(
                        frame.regions.iter().any(
                            |row| row.id == "dismiss:options" && row.kind == HitKind::Scrollbar
                        )
                    );
                    tree.focus("trigger");
                    tree.handle_event(UiEvent::Key {
                        key: Key::End,
                        ctrl: false,
                        shift: false,
                    });
                    let frame = tree.layout(viewport);
                    let last = frame
                        .regions
                        .iter()
                        .find(|row| row.id == "wrapped:11")
                        .unwrap()
                        .rect;
                    assert!(
                        last.y >= surface.y && last.y + last.height <= surface.y + surface.height
                    );
                    assert!(
                        tree.scroll_offset("dismiss:options")
                            > count as f32 * action_style::ROW_HEIGHT - surface.height
                    );
                    assert_eq!(
                        tree.handle_event(UiEvent::Key {
                            key: Key::Enter,
                            ctrl: false,
                            shift: false
                        }),
                        vec![UiAction::Clicked {
                            id: "wrapped:11".into()
                        }]
                    );
                }
            }
        }
    }

    #[test]
    fn action_menus_scroll_in_short_viewports_without_covering_the_trigger() {
        for top in [0.0, 190.0] {
            let menu = DropdownMenu::new(
                Element::button("trigger", "Ver").size(Some(60.0), Some(24.0)),
                (0..14)
                    .map(|index| {
                        DropdownMenuItem::new(format!("action:{index}"), format!("Acción {index}"))
                            .into()
                    })
                    .collect(),
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .context_menu_style()
            .width(304.0)
            .render();
            let mut tree = UiTree::new(Element::column(vec![
                Element::spacer().size(None, Some(top)),
                menu,
            ]));
            tree.set_reduced_motion(true);
            let viewport = Size::new(500.0, 240.0);
            let frame = tree.layout(viewport);
            let trigger = frame
                .regions
                .iter()
                .find(|row| row.id == "trigger")
                .unwrap()
                .rect;
            let surface = frame
                .regions
                .iter()
                .find(|row| row.id == "popover_surface:dismiss")
                .unwrap()
                .rect;
            assert!(
                surface.y >= trigger.y + trigger.height || surface.y + surface.height <= trigger.y,
                "menu {surface:?} must not cover trigger {trigger:?}"
            );
            assert!(surface.y >= 8.0 && surface.y + surface.height <= viewport.height - 8.0);
            assert!(
                frame
                    .regions
                    .iter()
                    .any(|row| row.id == "dismiss:options" && row.kind == HitKind::Scrollbar),
                "short-window overflow must have a real scrollbar even below the nominal height cap"
            );
            tree.focus("trigger");
            tree.handle_event(UiEvent::Key {
                key: Key::End,
                ctrl: false,
                shift: false,
            });
            let frame = tree.layout(viewport);
            let last = frame
                .regions
                .iter()
                .find(|row| row.id == "action:13")
                .unwrap()
                .rect;
            assert!(last.y >= surface.y && last.y + last.height <= surface.y + surface.height);
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    ctrl: false,
                    shift: false
                }),
                vec![UiAction::Clicked {
                    id: "action:13".into()
                }]
            );
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(surface.x + 30.0, surface.y + 30.0),
                delta_y: 100_000.0,
            });
            let frame = tree.layout(viewport);
            assert!(frame.regions.iter().any(|row| row.id == "action:0"));
            tree.handle_event(UiEvent::ScrollDirect {
                position: Point::new(surface.x + 30.0, surface.y + 30.0),
                delta_y: -100_000.0,
            });
            let frame = tree.layout(viewport);
            assert!(frame.regions.iter().any(|row| row.id == "action:13"));
        }
    }

    fn submenu_fixture(child_enabled: bool) -> Element {
        let child = DropdownMenuItem::new("child", "Abrir archivo");
        DropdownMenu::new(
            Element::button("trigger", "Menú"),
            vec![
                DropdownMenuItem::new("parent", "Archivo")
                    .submenu(vec![if child_enabled {
                        child.into()
                    } else {
                        child.disabled().into()
                    }])
                    .into(),
                DropdownMenuItem::new("sibling", "Editar").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .context_menu_style()
        .render()
    }

    #[test]
    fn closing_a_submenu_then_escape_before_layout_closes_its_parent() {
        let mut tree = UiTree::new(submenu_fixture(true));
        tree.set_reduced_motion(true);
        let viewport = Size::new(640.0, 360.0);
        tree.layout(viewport);
        tree.focus("parent");
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            ctrl: false,
            shift: false,
        });
        tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("child"));
        assert!(
            tree.handle_event(UiEvent::Key {
                key: Key::ArrowLeft,
                ctrl: false,
                shift: false
            })
            .is_empty()
        );
        // Native input can deliver several key events before the next paint.
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                ctrl: false,
                shift: false
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn closed_submenu_with_only_disabled_actions_stays_closed_across_frames() {
        let mut tree = UiTree::new(submenu_fixture(false));
        tree.set_reduced_motion(true);
        let viewport = Size::new(640.0, 360.0);
        tree.layout(viewport);
        tree.focus("parent");
        let frame = tree.layout(viewport);
        assert!(
            frame
                .regions
                .iter()
                .any(|row| row.id == "popover_surface:dismiss:parent")
        );
        assert!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                ctrl: false,
                shift: false
            })
            .is_empty()
        );
        for _ in 0..3 {
            let frame = tree.layout(viewport);
            assert!(
                !frame
                    .regions
                    .iter()
                    .any(|row| row.id == "popover_surface:dismiss:parent")
            );
            assert_eq!(tree.focused_id(), Some("parent"));
        }
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                ctrl: false,
                shift: false
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn inline_options_take_layout_space_and_scroll_without_covering_neighbors() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Elegir rama").size(Some(240.0), Some(32.0)),
            (0..20)
                .map(|index| {
                    DropdownMenuItem::new(format!("branch:{index}"), format!("rama-{index}")).into()
                })
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .width(240.0)
        .max_visible_items(3)
        .inline(true)
        .render();
        let mut tree = UiTree::new(Element::column(vec![
            menu,
            Element::button("following", "Continuar").size(Some(240.0), Some(32.0)),
        ]));
        tree.set_reduced_motion(true);
        let viewport = Size::new(240.0, 300.0);
        let frame = tree.layout(viewport);
        assert!(
            !frame
                .regions
                .iter()
                .any(|region| region.id.starts_with("popover_"))
        );
        let trigger = frame
            .regions
            .iter()
            .find(|region| region.id == "trigger")
            .unwrap()
            .rect;
        let options = frame
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options")
            .unwrap()
            .rect;
        let following = frame
            .regions
            .iter()
            .find(|region| region.id == "following")
            .unwrap()
            .rect;
        assert!(options.y >= trigger.y + trigger.height);
        assert!(following.y >= options.y + options.height);
        tree.handle_event(UiEvent::Scroll {
            position: Point::new(options.x + 12.0, options.y + 12.0),
            delta_y: -10000.0,
        });
        let scrolled = tree.layout(viewport);
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(
            scrolled
                .regions
                .iter()
                .any(|region| region.id == "branch:19")
        );
        assert_eq!(
            following,
            scrolled
                .regions
                .iter()
                .find(|region| region.id == "following")
                .unwrap()
                .rect
        );
    }

    #[test]
    fn compact_profile_sets_action_intrinsics_and_is_inherited_by_submenus() {
        let sizing = SelectorSizing::COMPACT;
        let mut tree = UiTree::new(
            DropdownMenu::new(
                Element::button("trigger", "Menú"),
                vec![
                    DropdownMenuItem::new("settings", "Configuración")
                        .icon(Icon::Settings)
                        .submenu(vec![DropdownMenuItem::new("account", "Cuenta").into()])
                        .into(),
                ],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .sizing(sizing)
            .render(),
        );
        tree.set_reduced_motion(true);
        let viewport = Size::new(800.0, 360.0);
        let first = tree.layout(viewport);
        let settings = first
            .regions
            .iter()
            .find(|row| row.id == "settings")
            .unwrap();
        assert_eq!(settings.rect.height, sizing.row_height);
        assert_eq!(settings.rect.width, sizing.preferred_width - 12.0);
        assert!(first.commands.iter().any(|command| matches!(command,
            DrawCommand::Text {text,font_size,..}
                if text == "Configuración" && *font_size == sizing.label_size
        )));
        assert!(first.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon {icon:Icon::Settings,rect,..}
                if rect.width == sizing.icon_size && rect.height == sizing.icon_size
        )));
        assert!(tree.focus("settings"));
        let before_key = tree.layout(viewport);
        assert!(
            before_key.regions.iter().any(|row| row.id == "account"),
            "nested rows before key: {:?}",
            before_key.regions
        );
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            shift: false,
            ctrl: false,
        });
        assert_eq!(
            tree.focused_id(),
            Some("account"),
            "submenu right-arrow dispatch"
        );
        let nested = tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("account"));
        let account = nested
            .regions
            .iter()
            .find(|row| row.id == "account")
            .unwrap();
        assert_eq!(account.rect.height, sizing.row_height);
        assert_eq!(account.rect.width, sizing.preferred_width - 12.0);
        assert!(nested.commands.iter().any(|command| matches!(command,
            DrawCommand::Text {text,font_size,..}
                if text == "Cuenta" && *font_size == sizing.label_size
        )));
    }

    #[test]
    fn a_custom_row_in_a_short_menu_is_as_wide_as_its_neighbours() {
        let sizing = SelectorSizing::CATALOG;
        let narrow = sizing.preferred_width - 12.0 - sizing.scrollbar_gutter;
        let entries = vec![
            DropdownMenuEntry::custom(
                Element::clickable("custom", Element::text("Proyecto"))
                    .size(Some(narrow), Some(sizing.detail_row_height)),
            ),
            DropdownMenuItem::new("plain", "Abrir otra carpeta").into(),
        ];
        let mut tree = UiTree::new(
            DropdownMenu::new(
                Element::button("trigger", "Proyectos"),
                entries,
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .sizing(sizing)
            .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(600.0, 440.0));
        let rect = |id: &str| frame.regions.iter().find(|row| row.id == id).unwrap().rect;
        assert_eq!(rect("custom").width, rect("plain").width);
        assert_eq!(rect("custom").height, sizing.detail_row_height);
    }

    #[test]
    fn catalog_mixed_list_budgets_the_visible_prefix_and_reserves_its_rail() {
        let sizing = SelectorSizing::CATALOG;
        let row_width = sizing.preferred_width - 12.0 - sizing.scrollbar_gutter;
        let detail = Element::clickable(
            "detail",
            Element::column(vec![
                Element::text("Modelo")
                    .font_size(sizing.label_size)
                    .size(None, Some(16.0)),
                Element::text("modelo-exacto")
                    .font_size(sizing.detail_size)
                    .size(None, Some(16.0)),
            ]),
        )
        .padding(6.0)
        .size(Some(row_width), Some(sizing.detail_row_height));
        let entries = vec![
            DropdownMenuEntry::Heading("Proveedor".into()),
            DropdownMenuEntry::custom(
                Tooltip::new(detail, "detail", "Modelo", ComponentPalette::neutral_dark())
                    .placement(TooltipPlacement::Left)
                    .max_width(sizing.tooltip_width)
                    .render()
                    .size(Some(row_width), Some(sizing.detail_row_height)),
            ),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("basic", "Acción")
                .icon(Icon::Folder)
                .into(),
            DropdownMenuEntry::custom(
                Element::text("Diagnóstico de conexión").size(Some(row_width), Some(104.0)),
            ),
            DropdownMenuItem::new("tail:0", "Primera opción posterior").into(),
            DropdownMenuItem::new("tail:1", "Última opción").into(),
        ];
        let mut tree = UiTree::new(
            DropdownMenu::new(
                Element::button("trigger", "Modelos"),
                entries,
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .sizing(sizing)
            .max_visible_items(4)
            .header(
                SearchField::new(
                    "search",
                    "",
                    "Buscar modelos",
                    ComponentPalette::neutral_dark(),
                )
                .size(ControlSize::Regular)
                .width(sizing.preferred_width - 12.0)
                .render(),
            )
            .render(),
        );
        tree.set_reduced_motion(true);
        let viewport = Size::new(600.0, 440.0);
        let first = tree.layout(viewport);
        let options = first
            .regions
            .iter()
            .find(|row| row.id == "dismiss:options" && row.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        let rail = first
            .regions
            .iter()
            .find(|row| row.id == "dismiss:options" && row.kind == HitKind::Scrollbar)
            .unwrap()
            .rect;
        assert_eq!(
            options.height,
            sizing.header_height + sizing.detail_row_height + 1.0 + sizing.row_height + 6.0
        );
        assert_eq!(rail.width, sizing.scrollbar_gutter);
        assert_eq!(
            first
                .regions
                .iter()
                .find(|row| row.id == "detail")
                .unwrap()
                .rect
                .height,
            sizing.detail_row_height
        );
        let basic = first
            .regions
            .iter()
            .find(|row| row.id == "basic")
            .unwrap()
            .rect;
        assert_eq!(basic.height, sizing.row_height);
        assert_eq!(basic.width, row_width);
        assert!(basic.x + basic.width <= rail.x);
        assert!(tree.focus("basic"));
        tree.handle_event(UiEvent::Key {
            key: Key::End,
            shift: false,
            ctrl: false,
        });
        let end = tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("tail:1"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(end.regions.iter().any(|row| row.id == "tail:1"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "tail:1".into()
            }]
        );
    }

    #[test]
    fn compact_catalog_keeps_header_and_last_keyboard_option_inside_short_viewport() {
        let sizing = SelectorSizing::CATALOG;
        for viewport in [Size::new(220.0, 148.0), Size::new(320.0, 180.0)] {
            let width = sizing.fit_width(viewport.width - 16.0);
            let mut tree = UiTree::new(
                DropdownMenu::new(
                    Element::button("trigger", "Modelos"),
                    (0..12)
                        .map(|index| {
                            DropdownMenuItem::new(format!("model:{index}"), "Modelo")
                                .icon(Icon::Code)
                                .into()
                        })
                        .collect(),
                    true,
                    "dismiss",
                    ComponentPalette::neutral_dark(),
                )
                .sizing(sizing)
                .width(width)
                .header(
                    SearchField::new(
                        "search",
                        "",
                        "Buscar modelos",
                        ComponentPalette::neutral_dark(),
                    )
                    .size(ControlSize::Regular)
                    .width(width - 12.0)
                    .render(),
                )
                .render(),
            );
            tree.set_reduced_motion(true);
            let first = tree.layout(viewport);
            let shell = first
                .regions
                .iter()
                .find(|row| row.id == "popover_surface:dismiss")
                .unwrap()
                .rect;
            let search = first
                .regions
                .iter()
                .find(|row| row.id == "search")
                .unwrap()
                .rect;
            let options = first
                .regions
                .iter()
                .find(|row| row.id == "dismiss:options" && row.kind == HitKind::Scroll)
                .unwrap()
                .rect;
            assert!(shell.x >= 0.0 && shell.y >= 0.0);
            assert!(
                shell.x + shell.width <= viewport.width
                    && shell.y + shell.height <= viewport.height
            );
            assert_eq!(search.height, ControlSize::Regular.height());
            assert!(search.y + search.height <= options.y);
            assert!(tree.focus("model:0"));
            tree.handle_event(UiEvent::Key {
                key: Key::End,
                shift: false,
                ctrl: false,
            });
            let end = tree.layout(viewport);
            assert_eq!(tree.focused_id(), Some("model:11"));
            assert_eq!(
                end.regions
                    .iter()
                    .find(|row| row.id == "search")
                    .unwrap()
                    .rect,
                search
            );
            let last = end
                .regions
                .iter()
                .find(|row| row.id == "model:11")
                .unwrap()
                .rect;
            assert!(last.y >= options.y && last.y + last.height <= options.y + options.height);
            assert!(last.x >= shell.x && last.x + last.width <= shell.x + shell.width);
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked {
                    id: "model:11".into()
                }]
            );
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Escape,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked {
                    id: "dismiss".into()
                }]
            );
        }
    }

    #[test]
    fn menu_skips_static_and_disabled_rows_and_reuses_popover_motion() {
        let palette = ComponentPalette::neutral_dark();
        let entries = vec![
            DropdownMenuEntry::Heading("Proyecto".into()),
            DropdownMenuItem::new("open", "Abrir")
                .icon(Icon::Folder)
                .shortcut("Ctrl+O")
                .into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("blocked", "No disponible")
                .disabled()
                .into(),
            DropdownMenuItem::new("settings", "Configurar").into(),
        ];
        let menu = DropdownMenu::new(
            Element::button("trigger", "Proyecto"),
            entries,
            true,
            "dismiss",
            palette,
        )
        .motion(MenuMotion {
            open_ms: 180,
            item_ms: 90,
        })
        .render();
        let mut tree = UiTree::new(menu);
        let frame = tree.layout(Size::new(360.0, 280.0));
        assert!(frame.regions.iter().any(|region| region.id == "open"));
        assert!(frame.regions.iter().any(|region| region.id == "settings"));
        assert!(!frame.regions.iter().any(|region| region.id == "blocked"));
        assert!(tree.focus("trigger"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowDown,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("open"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowDown,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("settings"));
        let option = frame
            .regions
            .iter()
            .find(|region| region.id == "settings")
            .unwrap();
        assert_eq!(
            frame.hit_test(Point::new(option.rect.x + 5.0, option.rect.y + 5.0)),
            Some("settings")
        );
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn menu_shortcut_uses_shared_keycaps_and_annotation_stays_plain() {
        let palette = ComponentPalette::neutral_dark();
        let menu = DropdownMenu::new(
            Element::button("trigger", "Menú"),
            vec![
                DropdownMenuItem::new("new", "Nueva tarea")
                    .shortcut("Ctrl+N")
                    .into(),
                DropdownMenuItem::new("current", "Sesión")
                    .annotation("Abierta")
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let frame = UiTree::new(menu).layout(Size::new(360.0, 220.0));
        for key in ["Ctrl", "N"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_family: FontFamily::Sans, .. }
                    if text == key
            )));
        }
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl+N"
        )));
        assert!(
            frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "+"
            )),
            "a plus joins the keys of one combination"
        );
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, radius, rect }
                if (color.r, color.g, color.b) == {
                    let fill = crate::shortcut::cap_fill(palette.text);
                    (fill.r, fill.g, fill.b)
                }
                    && *radius == crate::shortcut::radius()
                    && rect.height == crate::shortcut::HEIGHT
        )));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_family: FontFamily::Sans, .. }
                if text == "Abierta"
        )));
    }

    #[test]
    fn menu_motion_snaps_when_reduced_motion_is_enabled() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Abrir"),
            vec![DropdownMenuItem::new("option", "Opción").into()],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .motion(MenuMotion {
            open_ms: 2_000,
            item_ms: 2_000,
        })
        .render();
        let mut tree = UiTree::new(menu);
        tree.layout(Size::new(320.0, 220.0));
        assert!(tree.is_animating());
        tree.set_reduced_motion(true);
        tree.layout(Size::new(320.0, 220.0));
        assert!(!tree.is_animating());
    }

    #[test]
    fn submenu_opens_on_hover_and_stays_interactive() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Menú"),
            vec![
                DropdownMenuEntry::Heading("Menú principal".into()),
                DropdownMenuItem::new("settings", "Configuración")
                    .submenu(vec![
                        DropdownMenuItem::new("account", "Cuenta").into(),
                        DropdownMenuItem::new("privacy", "Privacidad").into(),
                    ])
                    .into(),
                DropdownMenuEntry::Separator,
                DropdownMenuItem::new("home", "Inicio").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .render();
        let mut tree = UiTree::new(menu);
        let frame = tree.layout(Size::new(640.0, 360.0));
        assert!(!frame.regions.iter().any(|region| region.id == "account"));
        let trigger = frame
            .regions
            .iter()
            .find(|region| region.id == "settings")
            .unwrap();
        let point = Point::new(trigger.rect.x + 12.0, trigger.rect.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(point));
        let frame = tree.layout(Size::new(640.0, 360.0));
        let child = frame
            .regions
            .iter()
            .find(|region| region.id == "account")
            .unwrap();
        assert!(child.rect.x >= trigger.rect.x + trigger.rect.width - 1.0);
        let child_point = Point::new(child.rect.x + 12.0, child.rect.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(child_point));
        let frame = tree.layout(Size::new(640.0, 360.0));
        assert_eq!(frame.hit_test(child_point), Some("account"));
        tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 300.0)));
        let frame = tree.layout(Size::new(640.0, 360.0));
        assert!(!frame.regions.iter().any(|region| region.id == "account"));
        assert!(tree.focus("settings"));
        tree.layout(Size::new(640.0, 360.0));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("account"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowLeft,
            shift: false,
            ctrl: false,
        });
        assert_eq!(tree.focused_id(), Some("settings"));
    }

    #[test]
    fn disabled_parent_never_opens_its_submenu() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Menú"),
            vec![
                DropdownMenuItem::new("settings", "Settings")
                    .submenu(vec![DropdownMenuItem::new("account", "Account").into()])
                    .disabled()
                    .into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .render();
        let mut tree = UiTree::new(menu);
        let frame = tree.layout(Size::new(420.0, 280.0));
        assert!(!frame.regions.iter().any(|region| region.id == "settings"));
        let label = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "Settings" => Some(*position),
                _ => None,
            })
            .expect("disabled parent is visible");
        tree.handle_event(UiEvent::PointerMove(Point::new(
            label.x + 4.0,
            label.y + 4.0,
        )));
        let frame = tree.layout(Size::new(420.0, 280.0));
        assert!(!frame.regions.iter().any(|region| region.id == "account"));
    }

    #[test]
    fn headings_and_separators_count_toward_scroll_limit() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Menú"),
            vec![
                DropdownMenuEntry::Heading("General".into()),
                DropdownMenuItem::new("first", "Primero").into(),
                DropdownMenuEntry::Separator,
                DropdownMenuItem::new("last", "Último").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .max_visible_items(2)
        .render();
        let frame = UiTree::new(menu).layout(Size::new(400.0, 320.0));
        assert!(
            frame
                .regions
                .iter()
                .any(|region| region.id == "dismiss:options")
        );
    }

    #[test]
    fn scrolling_menu_keeps_rows_clear_of_reserved_scrollbar() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Modelos"),
            vec![
                DropdownMenuItem::new("first", "Primero").into(),
                DropdownMenuItem::new("second", "Segundo").into(),
                DropdownMenuItem::new("third", "Tercero").into(),
                DropdownMenuItem::new("fourth", "Cuarto").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .width(280.0)
        .max_visible_items(2)
        .render();
        let frame = UiTree::new(menu).layout(Size::new(480.0, 320.0));
        let scroll = frame
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
            .expect("full-width scroll viewport");
        let rail = frame
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scrollbar)
            .expect("reserved scrollbar");
        let first = frame
            .regions
            .iter()
            .find(|region| region.id == "first")
            .expect("first row");
        assert_eq!(scroll.rect.width, 280.0 - 12.0);
        assert_eq!(rail.rect.width, SCROLLBAR_GUTTER);
        assert_eq!(first.rect.width, scroll.rect.width - SCROLLBAR_GUTTER);
        assert!(first.rect.x + first.rect.width <= rail.rect.x);
    }

    #[test]
    fn menu_header_keeps_search_fixed_and_interactive_after_options_scroll() {
        let header = Element::column(vec![
            Element::button("back", "Modelos").size(None, Some(36.0)),
            Element::text_input("search", "", "Buscar modelos").size(Some(268.0), Some(36.0)),
        ])
        .gap(2.0);
        let menu = DropdownMenu::new(
            Element::button("trigger", "Modelos"),
            (0..8)
                .map(|index| DropdownMenuItem::new(format!("model:{index}"), "Modelo").into())
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .header(header)
        .width(280.0)
        .max_visible_items(2)
        .render();
        let mut tree = UiTree::new(menu);
        tree.set_reduced_motion(true);
        let initial = tree.layout(Size::new(480.0, 320.0));
        let search_rect = initial
            .regions
            .iter()
            .find(|region| region.id == "search")
            .unwrap()
            .rect;
        let back_rect = initial
            .regions
            .iter()
            .find(|region| region.id == "back")
            .unwrap()
            .rect;
        let options = initial
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        assert_eq!(search_rect.width, options.width);
        assert!(search_rect.y + search_rect.height < options.y);

        tree.handle_event(UiEvent::Scroll {
            position: Point::new(options.x + 16.0, options.y + 16.0),
            delta_y: -76.0,
        });
        let offset = tree.scroll_offset("dismiss:options");
        assert!(offset > 0.0);
        let scrolled = tree.layout(Size::new(480.0, 320.0));
        for (id, expected) in [("search", search_rect), ("back", back_rect)] {
            let region = scrolled
                .regions
                .iter()
                .find(|region| region.id == id)
                .unwrap();
            assert_eq!(region.rect, expected);
        }
        assert!(!scrolled.regions.iter().any(|region| region.id == "model:0"));
        let search_point = Point::new(search_rect.x + 16.0, search_rect.y + 16.0);
        assert_eq!(scrolled.hit_test(search_point), Some("search"));
        tree.handle_event(UiEvent::PointerDown(search_point));
        tree.handle_event(UiEvent::PointerUp(search_point));
        assert_eq!(tree.focused_id(), Some("search"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("test".into())),
            vec![UiAction::InputChanged {
                id: "search".into(),
                value: "test".into()
            }]
        );
        tree.layout(Size::new(480.0, 320.0));
        assert_eq!(tree.scroll_offset("dismiss:options"), offset);
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Escape,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn header_options_shrink_to_short_surface_and_last_option_stays_reachable() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Modelos"),
            (0..12)
                .map(|index| DropdownMenuItem::new(format!("model:{index}"), "Modelo").into())
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .header(
            Element::column(vec![
                Element::button("back", "Modelos").size(None, Some(32.0)),
                Element::text_input("search", "", "Buscar modelos").size(Some(268.0), Some(36.0)),
            ])
            .gap(2.0),
        )
        .width(280.0)
        .max_visible_items(8)
        .render();
        let mut tree = UiTree::new(menu);
        tree.set_reduced_motion(true);
        let viewport = Size::new(480.0, 180.0);
        let initial = tree.layout(viewport);
        let shell = initial
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap()
            .rect;
        let search_rect = initial
            .regions
            .iter()
            .find(|region| region.id == "search")
            .unwrap()
            .rect;
        let options = initial
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        assert!(search_rect.y >= shell.y + 6.0);
        assert!(search_rect.y + search_rect.height < options.y);
        assert!(options.height < 8.0 * 38.0 - 2.0);
        assert!(options.y + options.height <= shell.y + shell.height - 6.0);

        tree.handle_event(UiEvent::Scroll {
            position: Point::new(options.x + 16.0, options.y + 16.0),
            delta_y: -10_000.0,
        });
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        let scrolled = tree.layout(viewport);
        assert_eq!(
            scrolled
                .regions
                .iter()
                .find(|region| region.id == "search")
                .unwrap()
                .rect,
            search_rect
        );
        for region in scrolled
            .regions
            .iter()
            .filter(|region| region.id.starts_with("model:") || region.id == "dismiss:options")
        {
            assert!(region.rect.x >= shell.x + 6.0);
            assert!(region.rect.y >= options.y);
            assert!(region.rect.x + region.rect.width <= shell.x + shell.width - 6.0);
            assert!(region.rect.y + region.rect.height <= shell.y + shell.height - 6.0);
        }
        let last = scrolled
            .regions
            .iter()
            .find(|region| region.id == "model:11")
            .unwrap();
        let point = Point::new(
            last.rect.x + last.rect.width / 2.0,
            last.rect.y + last.rect.height / 2.0,
        );
        assert_eq!(scrolled.hit_test(point), Some("model:11"));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "model:11".into()
            }]
        );
    }

    #[test]
    fn annotation_only_uses_space_in_its_own_row() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Modelos"),
            vec![
                DropdownMenuItem::new("plain", "Modelo de prueba largo").into(),
                DropdownMenuItem::new("annotated", "Otro")
                    .annotation("En tu cuenta")
                    .into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .width(225.0)
        .render();
        let frame = UiTree::new(menu).layout(Size::new(400.0, 260.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Modelo de prueba largo"
        )));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "En tu cuenta"
        )));
    }

    #[test]
    fn narrow_rows_keep_labels_annotations_and_shortcuts_inside_hover_bounds() {
        let menu = DropdownMenu::new(
            Element::button("trigger", "Abrir"),
            vec![
                DropdownMenuItem::new("annotated", "Descripción del modelo muy extensa")
                    .icon(Icon::Folder)
                    .annotation("Estado de cuenta demasiado extenso")
                    .selected(true)
                    .into(),
                DropdownMenuItem::new("shortcut", "Otra descripción larga")
                    .shortcut("Ctrl+Shift+Alt+N")
                    .into(),
                DropdownMenuItem::new("extra", "Tercera opción").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .width(184.0)
        .max_visible_items(2)
        .render();
        let mut tree = UiTree::new(menu);
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 9.0);
        let frame = tree.layout(Size::new(400.0, 320.0));

        for id in ["annotated", "shortcut"] {
            let row = frame
                .regions
                .iter()
                .find(|region| region.id == id)
                .expect("visible menu row");
            let texts: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, .. }
                        if position.y >= row.rect.y
                            && position.y < row.rect.y + row.rect.height =>
                    {
                        Some((text, position))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(texts.len(), if id == "annotated" { 2 } else { 1 });
            assert!(texts.iter().any(|(text, _)| text.ends_with('…')));
            for (text, position) in texts {
                let right = position.x + text.chars().count() as f32 * 9.0;
                assert!(position.x >= row.rect.x + 7.0, "{id}: {text}");
                assert!(right <= row.rect.x + row.rect.width - 7.0, "{id}: {text}");
            }
        }
        assert!(
            !frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. }
                    if ["Ctrl", "Shift", "Alt", "N"].contains(&text.as_str())
            )),
            "a shortcut that cannot fit is omitted as a complete group"
        );
    }

    #[test]
    fn shortcuts_use_measured_cap_widths_and_keep_the_full_combination() {
        for (width, expected_keys) in [(184.0, 0), (300.0, 3)] {
            let menu = DropdownMenu::new(
                Element::button("trigger", "Abrir"),
                vec![
                    DropdownMenuItem::new("shortcut", "Acción con descripción extensa")
                        .shortcut("Ctrl+Shift+M")
                        .into(),
                ],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .width(width)
            .render();
            let mut tree = UiTree::new(menu);
            tree.set_reduced_motion(true);
            tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 9.0);
            let frame = tree.layout(Size::new(400.0, 180.0));
            let row = frame
                .regions
                .iter()
                .find(|region| region.id == "shortcut")
                .unwrap();
            let keys: Vec<_> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text {
                        text,
                        position,
                        font_family: FontFamily::Sans,
                        ..
                    } if ["Ctrl", "Shift", "M"].contains(&text.as_str()) => Some((text, position)),
                    _ => None,
                })
                .collect();
            assert_eq!(keys.len(), expected_keys, "at {width}px");
            for (key, position) in keys {
                assert!(position.x >= row.rect.x + 7.0);
                assert!(
                    position.x + key.chars().count() as f32 * 9.0
                        <= row.rect.x + row.rect.width - 7.0
                );
            }
            assert!(!frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text.starts_with("Ctrl+")
            )));
        }
    }
}
