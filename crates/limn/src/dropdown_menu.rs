//! Shared action menu built on the retained popover and color transitions.

use crate::components::{
    Component, ComponentPalette, ControlState, Kbd, Popover, Tooltip, lighter,
};
use crate::menu::{DetailRow, MenuCheck, SWITCH_WIDTH, action_style};
use crate::{
    Access, Color, Element, FontFamily, ICON_SIZE, Icon, MenuMotion, Point, PopoverPlacement, Role,
    ScrollArea, ScrollbarAppearance, ScrollbarVisibility, SelectorSizing, Separator, Skeleton, TooltipSpec,
};

// The height of the viewport that holds an error too long for one row.
const ERROR_HEIGHT: f32 = 104.0;

// The scroll viewport remains the full menu content width. Only its children
// give up this much room, so the scrollbar never paints over a row or label.
const SCROLLBAR_GUTTER: f32 = 16.0;

/// One action in a menu. The caller handles `action_id` from `UiAction`.
pub struct DropdownMenuItem {
    /// What the host receives when the item is chosen. A disabled item is known
    /// to the platform by it.
    pub action_id: String,
    /// The row's text, and its name for the platform.
    pub label: String,
    /// A leading icon. `None` leaves the slot empty when another item has one.
    pub icon: Option<Icon>,
    /// A leading picture in its own colors, such as an application's icon, drawn in the
    /// icon's slot in place of `icon` and never tinted. Set with `picture`.
    pub picture: Option<crate::RasterImage>,
    /// A keyboard shortcut drawn as keycaps at the trailing edge. The `shortcut`
    /// builder clears `annotation`; with both set, only the shortcut is drawn.
    pub shortcut: Option<String>,
    /// Plain trailing text, and the item's description for the platform. The
    /// `annotation` builder clears `shortcut`.
    pub annotation: Option<String>,
    /// Whether it is the current choice. One selected item gives the whole menu
    /// a check column.
    pub selected: bool,
    /// Whether it can be chosen. A disabled item is drawn in the muted color, is
    /// passive and does not open its submenu.
    pub state: ControlState,
    /// The entries of a second menu that opens beside the row. `None` for an
    /// ordinary action.
    pub submenu: Option<Vec<DropdownMenuEntry>>,
    /// The width of the submenu, in place of this menu's. Set with `submenu_width`.
    submenu_width: Option<f32>,
    /// The ink of the label and the icon, in place of the palette's text. `None`
    /// for an ordinary action.
    pub destructive: Option<Color>,
    description: Option<String>,
    detail_mono: bool,
    status: Option<String>,
    status_width: Option<f32>,
    chevron: bool,
    tooltip: Option<TooltipSpec>,
    truncate: bool,
    check: Option<MenuCheck>,
}

impl DropdownMenuItem {
    /// An enabled, unselected item that shows `label`. `action_id` is what the host
    /// receives when it is chosen.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            picture: None,
            shortcut: None,
            annotation: None,
            selected: false,
            state: ControlState::Enabled,
            submenu: None,
            submenu_width: None,
            destructive: None,
            description: None,
            detail_mono: false,
            status: None,
            status_width: None,
            chevron: false,
            tooltip: None,
            truncate: false,
            check: None,
        }
    }

    /// A leading icon, drawn in the item's ink, or muted while the item is disabled.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A leading picture in its own colors, such as the icon of the application the item
    /// opens. It takes the icon's slot, square, and is drawn as it is: not tinted, not muted
    /// while the item is disabled. It is drawn in place of any `icon`.
    pub fn picture(mut self, picture: crate::RasterImage) -> Self {
        self.picture = Some(picture);
        self
    }

    /// A keyboard shortcut at the trailing edge. It replaces any `annotation`.
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

    /// Mark the item as the current choice. It is reported as selected to the platform and
    /// shown as such in the menu.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disable the item. Its submenu, if it has one, does not open.
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

    /// A quiet second line under the label, such as the value a setting has
    /// or, on a disabled item, why it cannot be chosen; the platform hears it
    /// after the label. The item becomes a two-line row with a column for the
    /// check, as tall as the menu's `sizing` says
    /// (`SelectorSizing::detail_row_height`, 40 without one), with the sides,
    /// icon slot, label size, gap and columns of the menu's one-line rows, so
    /// it lines up with them. With a `submenu` the row ends with a chevron and
    /// opens it. It ignores `shortcut`, `annotation` and `destructive`.
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

    /// Gives the submenu its own width, for rows longer than this menu's: a
    /// narrow menu whose submenu holds a longer action. Values that are not
    /// finite are ignored; without it the submenu is as wide as this menu.
    pub fn submenu_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.submenu_width = Some(width.max(0.0));
        }
        self
    }

    /// Draw the label and the icon in `ink`, for an action that destroys something. An item
    /// with a `description` ignores it.
    pub fn destructive(mut self, ink: Color) -> Self {
        self.destructive = Some(ink);
        self
    }

    /// Make the item a check box, ticked while `checked`, for a setting it
    /// turns on and off. A box at the end of the row shows the value, the
    /// platform is told it is a check box in a menu and whether it is on, and
    /// choosing it reports `action_id`: the host flips its own value. The box
    /// takes the place of the `selected` tick; `radio` replaces it.
    pub fn checkbox(mut self, checked: bool) -> Self {
        self.check = Some(MenuCheck::Box(checked));
        self
    }

    /// Make the item one radio item of a set, the chosen one while
    /// `checked`. A circle at the end of the row, with a dot while chosen,
    /// shows it, the platform is told it is a radio item in a menu and
    /// whether it is the chosen one, and choosing it reports `action_id`. The
    /// circle takes the place of the `selected` tick; `checkbox` replaces it.
    pub fn radio(mut self, checked: bool) -> Self {
        self.check = Some(MenuCheck::Radio(checked));
        self
    }

    /// Make the item a switch, on while `on`, for a setting it turns on and
    /// off. A small track at the end of the row shows it, with its thumb at
    /// the end it is on at; the platform is told it is a check box in a menu
    /// and whether it is on, and choosing it reports `action_id`: the host
    /// flips its own value. The track takes the place of the `selected` tick;
    /// `checkbox` and `radio` replace it, and it replaces them.
    pub fn switch(mut self, on: bool) -> Self {
        self.check = Some(MenuCheck::Switch(on));
        self
    }
}

/// Static entries do not enter keyboard focus order.
pub enum DropdownMenuEntry {
    /// An action that can be chosen.
    Item(DropdownMenuItem),
    /// A muted title over the entries that follow. One that comes after other
    /// entries gets a divider before it.
    Heading(String),
    /// A one-pixel line between two groups. One with no entry before it, or
    /// that ends the menu, is dropped.
    Separator,
    /// An application-specific control inside the shared menu surface.
    Custom(Box<Element>),
    /// A group heading that folds and unfolds its items.
    Section(DropdownMenuSection),
    /// A muted sentence in place of items: nothing found, or work in course.
    Message(String),
    /// Placeholder rows while the items load. Skeleton IDs derive from `id`.
    Loading {
        /// The skeleton of each row is `"{id}:{index}"`, counted from zero.
        id: String,
        /// How many placeholder rows, each as high as a one-line row.
        rows: usize,
    },
    /// A failure too long for a row, in its own short scroll area named `id`.
    Error {
        /// The ID of the scroll area the text sits in.
        id: String,
        /// The failure, in the muted style of a message. It wraps and scrolls.
        text: String,
    },
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

// What a two-line item at the menu's `detail` profile, a section and the
// state rows occupy, whatever the density of the menu's one-line items;
// `None` for an ordinary row.
fn fixed_entry_height(entry: &DropdownMenuEntry, detail: SelectorSizing) -> Option<f32> {
    let sizing = SelectorSizing::COMPACT;
    match entry {
        DropdownMenuEntry::Item(item) if item.description.is_some() => {
            Some(detail.detail_row_height)
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

/// The space between a menu's edge and a submenu beside it.
const SUBMENU_CLEARANCE: f32 = 3.0;

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
    scrollbar: Option<ScrollbarAppearance>,
    scrollbar_visibility: ScrollbarVisibility,
    sizing: Option<SelectorSizing>,
    inline: bool,
    context_menu_style: bool,
    hints_in_tooltips: bool,
    check_ink: Option<Color>,
    selected_fill: bool,
    // A submenu opens under the pointer: only the tree knows when.
    submenu: bool,
    at: Option<Point>,
}

impl DropdownMenu {
    /// A menu of `entries` on `anchor`, drawn only while `open`; closed, it renders the
    /// anchor alone. `dismiss_id` is reported when the menu is dismissed and prefixes the
    /// IDs of its parts, such as `{dismiss_id}:options` for its scroll area. The menu is
    /// 240 wide, opens below the start of the anchor and shows 8 rows before it scrolls.
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
            scrollbar: None,
            scrollbar_visibility: ScrollbarVisibility::Always,
            sizing: None,
            inline: false,
            context_menu_style: false,
            hints_in_tooltips: false,
            check_ink: None,
            selected_fill: true,
            submenu: false,
            at: None,
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

    /// Draw the check of a selected row, and the track of a switch that is on,
    /// in this color, such as an accent blue, rather than the row's ink or the
    /// link color. A disabled row's stays muted.
    pub fn check_ink(mut self, color: Color) -> Self {
        self.check_ink = Some(color);
        self
    }

    /// Whether a selected row is filled with the palette's `selected`; on by
    /// default. Off, the check alone marks it, and it hovers as the others
    /// do. A context menu style never fills it.
    pub fn selected_fill(mut self, fill: bool) -> Self {
        self.selected_fill = fill;
        self
    }

    /// Render the expanded options in normal layout, below the trigger.
    /// Useful inside narrow side panels where a floating menu would obscure
    /// adjacent content. The host owns open state and keyboard dismissal.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Set the menu's width in logical pixels, never below 96 (240 by default). A value
    /// that is not finite is ignored.
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

    /// Where the menu floats against its anchor; `PopoverPlacement::BottomStart` by
    /// default.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The durations of the menu's opening and of its rows' pointer feedback. Submenus take
    /// the same ones.
    pub fn motion(mut self, motion: MenuMotion) -> Self {
        self.motion = motion;
        self
    }

    /// Set how long the menu takes to close, in milliseconds, apart from its opening.
    /// Without it the popover's own short exit is used.
    pub fn close_animation_ms(mut self, duration: u16) -> Self {
        self.close_animation_ms = Some(duration);
        self
    }

    /// The visual scale the menu starts its entrance at, clamped to 0.5 to 1.25. The
    /// default, 1, does not scale. A value that is not finite is ignored.
    pub fn open_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() {
            self.open_scale = scale.clamp(0.5, 1.25);
        }
        self
    }

    /// Name the view the menu shows. When the key changes while the menu stays open, the
    /// content animates from one view to the other.
    pub fn view_key(mut self, key: impl Into<String>) -> Self {
        self.view_key = Some(key.into());
        self
    }

    /// How many rows show before the options scroll: at least 1, and 8 by default.
    pub fn max_visible_items(mut self, count: usize) -> Self {
        self.max_visible_items = count.max(1);
        self
    }

    /// The bar the options scroll with, such as the one the host's pages
    /// scroll with; a reserved rail in the palette's muted ink unless set.
    /// Without a `tint` of its own the bar is muted, and an overlay bar takes
    /// no width from the rows. Its submenus scroll with it too.
    pub fn scrollbar(mut self, appearance: ScrollbarAppearance) -> Self {
        self.scrollbar = Some(appearance);
        self
    }

    /// When the options' bar shows, as [`ScrollArea::visibility`]; always
    /// unless set. Its submenus follow it too.
    pub fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    /// Open the menu at a point of the window, as a context menu opens where
    /// the secondary click was: its corner is on the point, and it goes above
    /// or is moved in where the window has no room below or after it. The
    /// anchor keeps its place in the layout; pass the element the menu
    /// belongs to. The arrow keys reach the menu wherever the focus is, and a
    /// secondary click outside it reports the dismiss ID. A point that is not
    /// finite is ignored.
    pub fn at(mut self, position: Point) -> Self {
        if position.x.is_finite() && position.y.is_finite() {
            self.at = Some(position);
        }
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
        let filled = self.selected_fill && !contextual;
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
        let preferred_gap = if contextual {
            action_style::GAP
        } else {
            self.sizing.map_or(9.0, |sizing| sizing.gap)
        };
        let label_size = if contextual {
            action_style::LABEL_FONT_SIZE
        } else {
            self.sizing.map_or(13.0, |sizing| sizing.label_size)
        };
        // A two-line item is as tall as the menu's profile, or the catalogs',
        // says, and takes its sides, icon slot, label and gap from the menu's
        // one-line rows, so the two line up.
        let detail_sizing = SelectorSizing {
            icon_size,
            label_size,
            detail_size: if contextual {
                action_style::SHORTCUT_FONT_SIZE
            } else {
                self.sizing.map_or(12.0, |sizing| sizing.detail_size)
            },
            padding: vertical_padding.max(horizontal_padding),
            gap: preferred_gap,
            ..self.sizing.unwrap_or(SelectorSizing::COMPACT)
        };
        let action_height = |entry: &DropdownMenuEntry| match entry {
            DropdownMenuEntry::Heading(_) => action_style::HEADER_HEIGHT,
            DropdownMenuEntry::Separator => action_style::SEPARATOR_HEIGHT,
            entry => fixed_entry_height(entry, detail_sizing).unwrap_or(row_height),
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
                        entry => fixed_entry_height(entry, sizing).unwrap_or(row_height),
                    });
            heights.sum::<f32>() + self.max_visible_items.saturating_sub(1) as f32 * 2.0
        } else {
            self.max_visible_items as f32 * (row_height + 2.0) - 2.0
        };
        let reserve_icon = self
            .entries
            .iter()
            .any(|entry| matches!(entry, DropdownMenuEntry::Item(item) if item.icon.is_some() || item.picture.is_some()));
        let choices = self
            .entries
            .iter()
            .any(|entry| matches!(entry, DropdownMenuEntry::Item(item) if item.selected));
        let reserve_check = choices
            || self.entries.iter().any(
                |entry| matches!(entry, DropdownMenuEntry::Item(item) if item.check.is_some()),
            );
        // A menu with switches gives every row's check column a switch's width.
        let check_width = if self.entries.iter().any(|entry| {
            matches!(entry, DropdownMenuEntry::Item(item) if matches!(item.check, Some(MenuCheck::Switch(_))))
        }) {
            SWITCH_WIDTH
        } else {
            icon_size
        };
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
        // A host's bar keeps from the rows what its rail takes, an overlay nothing.
        let gutter = self.scrollbar.map_or(scrollbar_gutter, |bar| viewport_width - bar.content_width(viewport_width, true));
        let row_width = viewport_width - if scrolls { gutter } else { 0.0 };
        let palette = self.palette;
        // A row that opens a submenu floats it beside itself on hover.
        let with_submenu = |row: Element, id: &str, entries: Option<Vec<DropdownMenuEntry>>, width: Option<f32>| {
            let Some(entries) = entries else {
                return row;
            };
            let mut submenu = DropdownMenu::new(
                row,
                entries,
                true,
                format!("{}:{id}", self.dismiss_id),
                palette,
            );
            if contextual {
                submenu = submenu.context_menu_style();
            } else if let Some(sizing) = self.sizing {
                submenu = submenu.sizing(sizing);
            }
            submenu.submenu = true;
            submenu.scrollbar = self.scrollbar;
            submenu.check_ink = self.check_ink;
            submenu
                .scrollbar_visibility(self.scrollbar_visibility)
                .width(width.unwrap_or(self.width))
                .hints_in_tooltips(self.hints_in_tooltips)
                .selected_fill(self.selected_fill)
                .placement(PopoverPlacement::RightStart)
                .motion(self.motion)
                .max_visible_items(self.max_visible_items)
                .render()
                .popover_on_hover()
                // Measured from the row, which sits `inset` inside the menu's edge:
                // the submenu stands clear of the menu instead of overlapping it.
                .popover_gap(inset + SUBMENU_CLEARANCE)
        };
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
                    let submenu = if enabled { item.submenu } else { None };
                    let submenu_width = item.submenu_width;
                    let leads = item.chevron || submenu.is_some();
                    let row = DetailRow {
                        action_id: item.action_id,
                        label: item.label,
                        description: item.description.unwrap_or_default(),
                        icon: item.icon,
                        picture: item.picture,
                        mono: item.detail_mono,
                        status: item.status,
                        status_width: item.status_width,
                        icon_slot: reserve_icon,
                        // The menu's chevron column, or one of its own; a row
                        // that leads somewhere keeps the check column only
                        // where the menu has checks.
                        chevron: (reserve_submenu || leads).then_some(leads),
                        selected: (reserve_check || !leads).then_some(item.selected),
                        choice: choices,
                        check: item.check,
                        check_width,
                        enabled,
                        sizing: detail_sizing,
                    }
                    .render(palette);
                    let row = if contextual {
                        row
                    } else {
                        row.size(Some(row_width), Some(detail_sizing.detail_row_height))
                    };
                    let row = match item.tooltip {
                        Some(tooltip) => tooltip.wrap(row, id.clone(), palette),
                        None => row,
                    };
                    with_submenu(row, &id, submenu, submenu_width)
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
                    let control_width = control_count as f32 * icon_size
                        + if reserve_check { check_width - icon_size } else { 0.0 };
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
                    // A menu with checks is a set of choices; a check box or a
                    // radio item says what it is instead.
                    if let Some(check) = item.check {
                        access = check.access(access);
                    } else if choices {
                        access = access.selected(item.selected);
                    }
                    let submenu_id = item.action_id.clone();
                    let submenu = if enabled { item.submenu } else { None };
                    let submenu_width = item.submenu_width;
                    let ink = item.destructive.unwrap_or(palette.text);
                    let mut children = Vec::with_capacity(5);
                    if reserve_icon {
                        children.push(
                            if let Some(picture) = item.picture {
                                crate::menu::picture_slot(picture, icon_size)
                            } else if let Some(icon) = item.icon {
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
                    .font_size(label_size);
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
                        let ink = if enabled { palette.text } else { palette.muted };
                        // A switch is on in the menu's check ink; disabled, in the muted color, as the text is.
                        let accent = if enabled { self.check_ink } else { Some(palette.muted) };
                        children.push(
                            if let Some(check) = item.check {
                                check
                                    .mark(icon_size.min(ICON_SIZE), ink, accent, palette)
                                    .centered()
                            } else if item.selected {
                                Element::icon(Icon::Check)
                                    .foreground(if enabled { self.check_ink.unwrap_or(ink) } else { ink })
                            } else {
                                Element::spacer()
                            }
                            .size(
                                Some(check_width),
                                if contextual {
                                    None
                                } else {
                                    Some(content_height)
                                },
                            )
                            .min_size(check_width, content_height),
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
                    let row = if item.selected && filled {
                        row.background(palette.selected)
                    } else {
                        row
                    };
                    let row = palette
                        .interactive_surface(row)
                        .hover_background(if item.selected && filled {
                            lighter(palette.selected, 8)
                        } else {
                            palette.hover
                        })
                        .pressed_background(if item.selected && filled {
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
                    with_submenu(row, &submenu_id, submenu, submenu_width)
                }
            })
            .collect();
        let list = Element::column(rows).gap(row_gap);
        // Even a menu below the nominal cap may be taller than the space
        // above/below its anchor. Keep a scroll viewport ready for that final
        // allocation; the reserved rail appears only if it actually overflows.
        let scroll_container = contextual || scrolls;
        let content = if scroll_container {
            let bar = self.scrollbar.unwrap_or_else(|| ScrollbarAppearance::reserved().rail_width(scrollbar_gutter));
            let content = ScrollArea::new(format!("{}:options", self.dismiss_id), list)
                .appearance(ScrollbarAppearance { tint: bar.tint.or(Some(palette.muted)), ..bar })
                .visibility(self.scrollbar_visibility)
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
            return at_point(popover, self.at);
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
        at_point(popover.render(), self.at)
    }
}

// A menu at a point has its corner on it, with no gap.
fn at_point(popover: Element, at: Option<Point>) -> Element {
    match at {
        Some(point) => popover.popover_at(point).popover_gap(0.0),
        None => popover,
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
    fn a_selector_can_mark_its_choice_with_an_accent_check_and_no_fill() {
        let palette = ComponentPalette::neutral_dark();
        let blue = Color::rgb(0x25, 0x7D, 0xE0);
        let laid = |menu: &dyn Fn(DropdownMenu) -> DropdownMenu| {
            let entries = vec![
                DropdownMenuItem::new("opus", "Opus").selected(true).into(),
                DropdownMenuItem::new("sonnet", "Sonnet").into(),
            ];
            let menu = menu(DropdownMenu::new(Element::button("model", "Model"), entries, true, "models", palette)
                .sizing(SelectorSizing::COMPACT));
            let mut tree = UiTree::new(menu.render());
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(500.0, 400.0));
            let row = frame.regions.iter().find(|region| region.id == "opus").unwrap().rect;
            let filled = frame.commands.iter().any(|command| matches!(command,
                DrawCommand::FillRect { rect, color, .. } if *rect == row && *color == palette.selected));
            let check = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Icon { icon: Icon::Check, tint, .. } => Some(*tint),
                _ => None,
            });
            (filled, check)
        };
        assert_eq!(laid(&|menu| menu), (true, Some(palette.text)));
        assert_eq!(laid(&|menu| menu.check_ink(blue).selected_fill(false)), (false, Some(blue)));
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

    // The menu of every two-line test: one-line and two-line rows side by
    // side, with icons, check boxes, submenus and a disabled one.
    fn two_line_entries() -> Vec<DropdownMenuEntry> {
        let submenu = || vec![DropdownMenuItem::new("deep", "Deep").into()];
        vec![
            DropdownMenuItem::new("mute", "Mute")
                .icon(Icon::Bell)
                .checkbox(true)
                .into(),
            DropdownMenuItem::new("model", "Model")
                .icon(Icon::Server)
                .description("large-v3")
                .detail_mono(true)
                .status("Verified")
                .checkbox(true)
                .into(),
            DropdownMenuItem::new("more", "More")
                .icon(Icon::Edit)
                .submenu(submenu())
                .into(),
            DropdownMenuItem::new("effort", "Effort")
                .icon(Icon::Brain)
                .description("Automatic")
                .submenu(submenu())
                .into(),
            DropdownMenuItem::new("off", "Unavailable")
                .description("No credential")
                .disabled()
                .into(),
        ]
    }

    #[test]
    fn a_two_line_item_lines_up_with_the_one_line_rows_in_every_style() {
        let palette = ComponentPalette::neutral_dark();
        let server = SelectorSizing {
            row_height: 38.0,
            icon_size: 20.0,
            label_size: 13.0,
            detail_size: 12.0,
            padding: 8.0,
            gap: 10.0,
            ..SelectorSizing::COMPACT
        };
        let tall = SelectorSizing {
            row_height: 40.0,
            padding: 4.0,
            ..SelectorSizing::COMPACT
        };
        // Style, profile, and the sides and text sizes its one-line rows have.
        let styles = [
            (false, None, 7.0, 13.0, 12.0),
            (true, None, action_style::HORIZONTAL_PADDING, 13.0, 12.0),
            (false, Some(SelectorSizing::COMPACT), 6.0, 12.0, 11.0),
            (false, Some(tall), 9.0, 12.0, 11.0),
            (false, Some(server), 8.0, 13.0, 12.0),
        ];
        for (contextual, sizing, side, label_size, detail_size) in styles {
            let style = format!("contextual: {contextual}, sizing: {sizing:?}");
            let mut menu =
                DropdownMenu::new(trigger(), two_line_entries(), true, "dismiss", palette)
                    .width(300.0);
            if contextual {
                menu = menu.context_menu_style();
            }
            if let Some(sizing) = sizing {
                menu = menu.sizing(sizing).width(300.0);
            }
            let mut tree = open_menu(menu);
            let frame = tree.layout(Size::new(640.0, 480.0));
            let height = sizing.unwrap_or(SelectorSizing::COMPACT).detail_row_height;
            let (model, effort) = (region(&frame, "model"), region(&frame, "effort"));
            assert_eq!((model.height, effort.height), (height, height), "{style}");
            let font = |label: &str| {
                frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text {
                            text, font_family, ..
                        } if text == label => Some(*font_family),
                        _ => None,
                    })
                    .unwrap()
            };
            // Every label starts in one column, at the one-line rows' size.
            let (mute, mute_size, _) = text_of(&frame, "Mute");
            let (label, model_size, label_ink) = text_of(&frame, "Model");
            let (detail, model_detail_size, detail_ink) = text_of(&frame, "large-v3");
            assert_eq!((model_size, mute_size), (label_size, label_size), "{style}");
            assert_eq!(model_detail_size, detail_size, "{style}");
            assert_eq!(font("large-v3"), FontFamily::Monospace);
            assert_eq!((label_ink, detail_ink), (palette.text, palette.muted));
            for other in [
                "More",
                "Effort",
                "Automatic",
                "Unavailable",
                "No credential",
            ] {
                assert_eq!(text_of(&frame, other).0.x, mute.x, "{other} ({style})");
            }
            assert_eq!(detail.x, label.x, "{style}");
            assert!(text_of(&frame, "Verified").0.x > detail.x);
            // Two lines that do not run into each other, inside the row.
            assert!(detail.y - label.y >= label_size, "{style}");
            assert!(label.y >= model.y, "{style}");
            assert!(detail.y + detail_size <= model.y + model.height, "{style}");
            // The check boxes share a column, and so do the chevrons, which
            // end the rows at the one-line rows' side.
            let icons = |kind: Icon| -> Vec<(crate::Rect, Color)> {
                frame
                    .commands
                    .iter()
                    .filter_map(|command| match command {
                        DrawCommand::Icon { icon, rect, tint } if *icon == kind => {
                            Some((*rect, *tint))
                        }
                        _ => None,
                    })
                    .collect()
            };
            let checks = icons(Icon::Check);
            assert_eq!(checks.len(), 2, "{style}");
            assert_eq!(checks[0].0.x, checks[1].0.x, "{style}");
            assert!(checks.iter().all(|(_, tint)| *tint == palette.surface));
            let chevrons = icons(Icon::ChevronRight);
            assert_eq!(chevrons.len(), 2, "{style}");
            assert_eq!(chevrons[0].0.x, chevrons[1].0.x, "{style}");
            let icon_size = sizing.map_or(ICON_SIZE, |sizing| sizing.icon_size);
            for (rect, tint) in chevrons {
                // Centered in a column as wide as the icon slot.
                let end = effort.x + effort.width - side;
                assert_eq!(rect.x + rect.width / 2.0, end - icon_size / 2.0, "{style}");
                assert_eq!(tint, palette.muted);
            }
            // Disabled, it is passive and muted, and says no choice.
            assert!(!has(&frame, "off"), "a disabled item is passive");
            assert_eq!(text_of(&frame, "Unavailable").2, palette.muted);
            let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
            assert_eq!((node("off").selected, node("off").toggled), (None, None));
            assert_eq!(
                (node("model").role, node("model").toggled),
                (Role::MenuItemCheckBox, Some(true))
            );
        }
    }

    #[test]
    fn a_catalog_two_line_item_keeps_its_compact_metrics() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = SelectorSizing::COMPACT;
        let menu = DropdownMenu::new(
            trigger(),
            vec![
                DropdownMenuItem::new("model", "Model")
                    .icon(Icon::Server)
                    .description("large-v3")
                    .selected(true)
                    .into(),
                DropdownMenuItem::new("effort", "Effort")
                    .icon(Icon::Brain)
                    .description("Automatic")
                    .chevron()
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .sizing(sizing);
        let frame = open_menu(menu).layout(Size::new(640.0, 480.0));
        let model = region(&frame, "model");
        let (label, ..) = text_of(&frame, "Model");
        let (detail, ..) = text_of(&frame, "large-v3");
        // The lines share the 28 a one-line row has, 6 from the top.
        assert_eq!(
            label.x,
            model.x + sizing.padding + sizing.icon_size + sizing.gap
        );
        assert!((detail.y - label.y - sizing.row_height / 2.0).abs() < 1.0);
        let ends = |kind: Icon| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Icon { icon, rect, tint } if *icon == kind => {
                    Some((rect.x + rect.width, *tint))
                }
                _ => None,
            })
        };
        let end = model.x + model.width - sizing.padding;
        assert_eq!(ends(Icon::Check), Some((end, palette.text)));
        assert_eq!(ends(Icon::ChevronRight), Some((end, palette.muted)));
        let node = frame.access.iter().find(|node| node.id == "model").unwrap();
        assert_eq!(node.selected, Some(true), "a menu with a choice says it");
    }

    #[test]
    fn check_boxes_and_two_line_items_stay_inside_a_narrow_menu() {
        let palette = ComponentPalette::neutral_dark();
        for contextual in [false, true] {
            for width in [90.0, 140.0] {
                let menu =
                    DropdownMenu::new(trigger(), two_line_entries(), true, "dismiss", palette);
                // The style sets its own width: the narrow one comes after it.
                let mut tree = open_menu(if contextual {
                    menu.context_menu_style().width(width)
                } else {
                    menu.width(width)
                });
                let frame = tree.layout(Size::new(640.0, 480.0));
                let surface = region(&frame, "popover_surface:dismiss");
                for id in ["mute", "model", "more", "effort"] {
                    let row = region(&frame, id);
                    assert!(
                        row.x >= surface.x && row.x + row.width <= surface.x + surface.width,
                        "{id} at {width} (contextual: {contextual})"
                    );
                }
                for command in &frame.commands {
                    if let DrawCommand::Icon { rect, icon, .. } = command {
                        assert!(
                            rect.x >= surface.x - 0.5
                                && rect.x + rect.width <= surface.x + surface.width + 0.5,
                            "{icon:?} at {width} (contextual: {contextual})"
                        );
                    }
                }
            }
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
    fn a_picture_takes_the_icon_slot_square_in_plain_and_detail_rows() {
        let palette = ComponentPalette::neutral_dark();
        let picture = |alt: &str| {
            crate::RasterImage::new(
                "a".repeat(64),
                crate::RasterImageFormat::Png,
                4,
                std::sync::Arc::from("AAAA"),
                alt,
            )
            .unwrap()
        };
        let menu = DropdownMenu::new(
            Element::button("trigger", "Open in"),
            vec![
                DropdownMenuItem::new("editor", "Editor")
                    .picture(picture("Editor"))
                    .into(),
                DropdownMenuItem::new("files", "File manager")
                    .icon(Icon::Folder)
                    .into(),
                DropdownMenuItem::new("other", "Other")
                    .description("With a line under it")
                    .picture(picture("Other"))
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let frame = UiTree::new(menu).layout(Size::new(360.0, 260.0));
        let pictures: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::RasterImage { rect, image, .. } => Some((*rect, image.alt())),
                _ => None,
            })
            .collect();
        let side = SelectorSizing::COMPACT.icon_size;
        assert_eq!(pictures.len(), 2, "one picture a row that has one");
        for (rect, _) in &pictures {
            assert_eq!((rect.width, rect.height), (side, side));
        }
        // In line with the icon of the row between them.
        let folder = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon {
                    icon: Icon::Folder,
                    rect,
                    ..
                } => Some(*rect),
                _ => None,
            })
            .unwrap();
        let center = |r: crate::Rect| r.x + r.width / 2.0;
        assert!((center(pictures[0].0) - center(folder)).abs() < 1.0);
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
    fn a_submenu_stands_clear_of_its_menu_and_ends_at_its_row_near_the_foot() {
        let more: Vec<DropdownMenuEntry> = (0..8)
            .map(|n| DropdownMenuItem::new(format!("model-{n}"), format!("Model {n}")).into())
            .collect();
        let menu = DropdownMenu::new(
            Element::button("trigger", "Model"),
            vec![
                DropdownMenuItem::new("one", "One").into(),
                DropdownMenuItem::new("more", "More models").submenu(more).into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .placement(PopoverPlacement::TopStart)
        .render();
        let size = Size::new(640.0, 400.0);
        let mut tree = UiTree::new(Element::column(vec![Element::spacer().flex(1.0), menu]));
        let frame = tree.layout(size);
        let row = frame.regions.iter().find(|region| region.id == "more").unwrap().rect;
        tree.handle_event(UiEvent::PointerMove(Point::new(row.x + 12.0, row.y + 12.0)));
        let frame = tree.layout(size);
        let first = frame.regions.iter().find(|region| region.id == "model-0").unwrap().rect;
        let last = frame.regions.iter().find(|region| region.id == "model-7").unwrap().rect;
        // The row sits 6 inside its menu, the submenu 3 beyond the menu, its rows 6 inside it.
        assert_eq!(first.x, row.x + row.width + 6.0 + SUBMENU_CLEARANCE + 6.0);
        // No room below the row: the submenu ends level with it instead.
        assert!(last.y + last.height <= row.y + row.height, "{last:?} {row:?}");
        assert!(last.y + last.height >= row.y + row.height - 12.0, "{last:?} {row:?}");
        // Crossing the menu's padding and the gap on the way keeps it open.
        let gap = Point::new(row.x + row.width + 6.0 + SUBMENU_CLEARANCE / 2.0, row.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(gap));
        let frame = tree.layout(size);
        let last = frame.regions.iter().find(|region| region.id == "model-7").unwrap().rect;
        let inside = Point::new(last.x + 12.0, last.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(inside));
        assert_eq!(tree.layout(size).hit_test(inside), Some("model-7"));
    }

    #[test]
    fn a_submenu_takes_its_own_width_when_given_one() {
        let rows = |width: Option<f32>| {
            let item = DropdownMenuItem::new("view", "View").submenu(vec![
                DropdownMenuItem::new("default", "Make this the default").into(),
            ]);
            let item = match width {
                Some(width) => item.submenu_width(width),
                None => item,
            };
            let menu = DropdownMenu::new(
                Element::button("trigger", "Menu"),
                vec![item.into()],
                true,
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .width(160.0)
            .render();
            let mut tree = UiTree::new(menu);
            let frame = tree.layout(Size::new(800.0, 360.0));
            let row = frame.regions.iter().find(|region| region.id == "view").unwrap().rect;
            tree.handle_event(UiEvent::PointerMove(Point::new(row.x + 12.0, row.y + 12.0)));
            let frame = tree.layout(Size::new(800.0, 360.0));
            let child = frame.regions.iter().find(|region| region.id == "default").unwrap().rect;
            (row.width, child.width)
        };
        let (parent, same) = rows(None);
        assert_eq!(parent, same, "without a width the submenu is as wide as its menu");
        let (_, wider) = rows(Some(240.0));
        assert!(wider > same + 70.0, "{wider} vs {same}");
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
    fn a_scroll_outside_the_menu_closes_it_and_still_moves_the_page_and_one_over_its_rows_scrolls_them() {
        let viewport = Size::new(480.0, 320.0);
        let palette = ComponentPalette::neutral_dark();
        let rows = (0..6).map(|index| DropdownMenuItem::new(format!("row:{index}"), format!("Fila {index}")).into()).collect();
        let menu = DropdownMenu::new(trigger(), rows, true, "dismiss", palette).max_visible_items(2);
        let tiles = (0..8).map(|index| Element::button(format!("album:{index}"), "Álbum").size(Some(100.0), Some(60.0))).collect();
        // Low enough on the page that the trigger stays in view while it scrolls.
        let page = Element::column(vec![
            Element::spacer().size(None, Some(200.0)),
            menu.render(),
            Element::spacer().size(None, Some(60.0)),
            crate::Carousel::new("shelf", tiles, palette).render(),
            Element::spacer().size(None, Some(1000.0)),
        ]);
        let mut tree = UiTree::new(Element::scroll("page", page));
        tree.set_reduced_motion(true);
        let frame = tree.layout(viewport);
        // Over its own rows the wheel scrolls them and the menu stays open.
        let options = region(&frame, "dismiss:options");
        let rows = Point::new(options.x + 20.0, options.y + 10.0);
        assert!(tree.handle_event(UiEvent::Scroll { position: rows, delta_y: -30.0 }).is_empty());
        tree.layout(viewport);
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert_eq!(tree.scroll_offset("page"), 0.0);
        // Outside it every kind of scroll reports the dismissal, and the page under the pointer moves.
        let dismissed = vec![UiAction::Clicked { id: "dismiss".into() }];
        let outside = Point::new(400.0, 250.0);
        for event in [
            UiEvent::Scroll { position: outside, delta_y: -40.0 },
            UiEvent::ScrollDirect { position: outside, delta_y: -40.0 },
            UiEvent::Scroll2D { position: outside, delta_x: 0.0, delta_y: -40.0, direct: false, shift: false },
        ] {
            let before = tree.scroll_offset("page");
            assert_eq!(tree.handle_event(event.clone()), dismissed, "{event:?}");
            tree.layout(viewport);
            assert_eq!(tree.scroll_offset("page"), before + 40.0, "{event:?}");
        }
        // A trackpad's sideways pan over a shelf of the page, too.
        let shelf = tree.frame().regions.iter().find(|region| region.id == "shelf" && region.kind == HitKind::Scroll).unwrap().rect;
        let pan = UiEvent::Scroll2D {
            position: Point::new(shelf.x + 300.0, shelf.y + shelf.height / 2.0),
            delta_x: -40.0,
            delta_y: 0.0,
            direct: true,
            shift: false,
        };
        assert_eq!(tree.handle_event(pan), dismissed);
        assert_eq!(tree.horizontal_scroll_offset("shelf"), 40.0);
    }

    #[test]
    fn a_host_s_bar_scrolls_the_options_and_an_overlay_keeps_the_rows_whole() {
        let layout = |host: fn(DropdownMenu) -> DropdownMenu| {
            let rows = ["first", "second", "third", "fourth"].map(|id| DropdownMenuItem::new(id, id).into());
            let menu = DropdownMenu::new(Element::button("trigger", "Modelos"), rows.into(), true, "dismiss", ComponentPalette::neutral_dark());
            let mut tree = open_menu(host(menu).width(280.0).max_visible_items(2));
            let frame = tree.layout(Size::new(480.0, 320.0));
            (tree, frame)
        };
        let width = |frame: &crate::Frame, id: &str| region(frame, id).width;
        // A reserved rail of the host's width keeps that much from the rows.
        let (_, frame) = layout(|menu| menu.scrollbar(ScrollbarAppearance::reserved().rail_width(10.0)));
        assert_eq!(width(&frame, "first"), width(&frame, "dismiss:options") - 10.0);
        // An overlay, square, white and hidden until the pointer is over it.
        let (mut tree, frame) = layout(|menu| {
            let bar = ScrollbarAppearance::overlay().thumb_width(12.0).thumb_radius(0.0).tint(Color::WHITE);
            menu.scrollbar(bar).scrollbar_visibility(ScrollbarVisibility::AutoHideUnlessHovered { hold_ms: 800, fade_ms: 200 })
        });
        let options = region(&frame, "dismiss:options");
        assert_eq!(width(&frame, "first"), options.width);
        let thumb = |frame: &crate::Frame| {
            frame.commands.iter().any(|command| matches!(command,
                DrawCommand::FillRect { rect, color, radius } if rect.width == 12.0 && *radius == 0.0
                    && (color.r, color.g, color.b) == (255, 255, 255) && color.a > 0))
        };
        assert!(!thumb(&frame), "hidden until hovered");
        tree.handle_event(UiEvent::PointerMove(Point::new(options.x + 8.0, options.y + 8.0)));
        assert!(thumb(&tree.layout(Size::new(480.0, 320.0))), "a square white thumb under the pointer");
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

    fn center(rect: crate::Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
        })
    }

    fn has(frame: &crate::Frame, id: &str) -> bool {
        frame.regions.iter().any(|region| region.id == id)
    }

    // Where `label` is drawn, at which size and in which ink.
    fn text_of(frame: &crate::Frame, label: &str) -> (Point, f32, Color) {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text {
                    text,
                    position,
                    font_size,
                    color,
                    ..
                } if text == label => Some((*position, *font_size, *color)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {label}"))
    }

    #[test]
    fn check_box_and_radio_items_draw_their_mark_report_their_action_and_say_what_they_are() {
        let palette = ComponentPalette::neutral_dark();
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("mute", "Mute channel")
                        .checkbox(true)
                        .into(),
                    DropdownMenuItem::new("hide", "Hide muted")
                        .checkbox(false)
                        .into(),
                    DropdownMenuEntry::Separator,
                    DropdownMenuItem::new("all", "All messages")
                        .radio(true)
                        .into(),
                    DropdownMenuItem::new("mentions", "Only mentions")
                        .radio(false)
                        .into(),
                    DropdownMenuItem::new("nothing", "Nothing")
                        .radio(false)
                        .disabled()
                        .into(),
                    DropdownMenuItem::new("plain", "Mark as read").into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(260.0);
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let size = Size::new(640.0, 480.0);
            let frame = tree.layout(size);
            let node = |id: &str| {
                frame
                    .access
                    .iter()
                    .find(|node| node.id == id)
                    .unwrap_or_else(|| panic!("no node {id}"))
            };
            for (id, role, on) in [
                ("mute", Role::MenuItemCheckBox, true),
                ("hide", Role::MenuItemCheckBox, false),
                ("all", Role::MenuItemRadio, true),
                ("mentions", Role::MenuItemRadio, false),
                ("nothing", Role::MenuItemRadio, false),
            ] {
                let item = node(id);
                assert_eq!(
                    (item.role, item.toggled, item.selected),
                    (role, Some(on), None),
                    "{id}"
                );
                assert_eq!(item.disabled, id == "nothing", "{id}");
            }
            let plain = node("plain");
            assert_eq!(
                (plain.role, plain.toggled, plain.selected),
                (Role::MenuItem, None, None)
            );
            // Each mark is 16 square at the end of its row, the box filled
            // with a check while ticked and the circle with a dot while chosen.
            let inset = if contextual {
                action_style::HORIZONTAL_PADDING
            } else {
                7.0
            };
            let mark = |id: &str| -> (crate::Rect, Option<Color>, Option<Color>) {
                let row = region(&frame, id);
                let inside = |rect: &crate::Rect| {
                    rect.width == 16.0
                        && rect.y >= row.y
                        && rect.y + rect.height <= row.y + row.height
                };
                let fill = frame.commands.iter().find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if inside(rect) && color.a > 0 && *color != palette.surface =>
                    {
                        Some((*rect, *color))
                    }
                    _ => None,
                });
                let stroke = frame.commands.iter().find_map(|command| match command {
                    DrawCommand::StrokeRect { rect, color, .. } if inside(rect) => {
                        Some((*rect, *color))
                    }
                    _ => None,
                });
                let rect = fill
                    .or(stroke)
                    .map(|(rect, _)| rect)
                    .unwrap_or_else(|| panic!("no mark in {id}"));
                assert_eq!(
                    rect.x + rect.width,
                    row.x + row.width - inset,
                    "{id} ends its row"
                );
                assert!(
                    (rect.y - (row.y + (row.height - 16.0) / 2.0)).abs() < 0.5,
                    "{id} is centered"
                );
                (
                    rect,
                    fill.map(|(_, color)| color),
                    stroke.map(|(_, color)| color),
                )
            };
            let (ticked, fill, _) = mark("mute");
            assert_eq!(fill, Some(palette.text));
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Icon { icon: Icon::Check, tint, rect } if *tint == palette.surface && rect.x >= ticked.x)));
            assert_eq!(mark("hide").1, None, "an empty box is an outline");
            assert_eq!(mark("hide").2, Some(palette.muted));
            let (chosen, fill, _) = mark("all");
            assert_eq!(fill, Some(palette.text));
            let dot = crate::radio::dot_side(16.0);
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::FillRect { rect, color, radius } if *color == palette.surface && rect.width == dot
                    && *radius == dot / 2.0 && (rect.x - (chosen.x + (16.0 - dot) / 2.0)).abs() < 0.5)));
            assert_eq!(mark("mentions").1, None);
            assert!(!has(&frame, "nothing"), "a disabled radio item is passive");
            // Choosing one reports its action: the host flips the value.
            for id in ["hide", "all"] {
                let point = center(region(&tree.layout(size), id));
                tree.handle_event(UiEvent::PointerDown(point));
                assert_eq!(
                    tree.handle_event(UiEvent::PointerUp(point)),
                    vec![UiAction::Clicked { id: id.into() }]
                );
            }
        }
    }

    /// The switch's track inside `row`: a fill 28 wide and 16 high.
    fn switch_track(frame: &crate::Frame, row: crate::Rect) -> Option<crate::Rect> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, .. }
                if rect.width == 28.0
                    && rect.height == 16.0
                    && rect.y >= row.y
                    && rect.y + rect.height <= row.y + row.height =>
            {
                Some(*rect)
            }
            _ => None,
        })
    }

    #[test]
    fn a_switch_that_is_on_takes_the_menus_check_ink() {
        let palette = ComponentPalette::neutral_dark();
        let blue = Color::rgb(37, 125, 224);
        let menu = DropdownMenu::new(
            trigger(),
            vec![
                DropdownMenuItem::new("on", "On").switch(true).into(),
                DropdownMenuItem::new("off", "Off").switch(false).into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(260.0)
        .check_ink(blue);
        let mut tree = open_menu(menu);
        let frame = tree.layout(Size::new(640.0, 480.0));
        let fill = |id: &str| {
            let track = switch_track(&frame, region(&frame, id)).expect("a track");
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *rect == track => Some(*color),
                _ => None,
            })
        };
        assert_eq!(fill("on"), Some(crate::chip::readable_ink(blue, palette.text, 3.0)));
        assert_ne!(fill("off"), fill("on"));
    }

    #[test]
    fn switch_items_draw_a_track_at_the_end_report_their_action_and_say_what_they_are() {
        let palette = ComponentPalette::neutral_dark();
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("sync", "Sync every hour")
                        .switch(true)
                        .into(),
                    DropdownMenuItem::new("mirror", "Mirror changes")
                        .switch(false)
                        .into(),
                    DropdownMenuItem::new("plain", "Mark as read").into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(260.0);
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let size = Size::new(640.0, 480.0);
            let frame = tree.layout(size);
            let node = |id: &str| {
                frame
                    .access
                    .iter()
                    .find(|node| node.id == id)
                    .unwrap_or_else(|| panic!("no node {id}"))
            };
            for (id, on) in [("sync", true), ("mirror", false)] {
                let item = node(id);
                assert_eq!((item.role, item.toggled), (Role::MenuItemCheckBox, Some(on)), "{id}");
            }
            assert_eq!(node("plain").role, Role::MenuItem);
            let inset = if contextual {
                action_style::HORIZONTAL_PADDING
            } else {
                7.0
            };
            for id in ["sync", "mirror"] {
                let row = region(&frame, id);
                let track = switch_track(&frame, row).unwrap_or_else(|| panic!("no track in {id}"));
                assert_eq!(
                    track.x + track.width,
                    row.x + row.width - inset,
                    "{id} ends its row"
                );
                let plain = region(&frame, "plain");
                assert_eq!(row.width, plain.width, "{id} is as wide as the menu's rows");
                // The thumb is round, at the end the switch is on at.
                let thumb = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::FillRect { rect, radius, .. }
                            if rect.width == 10.0
                                && rect.height == 10.0
                                && *radius == 5.0
                                && rect.x >= track.x
                                && rect.x + rect.width <= track.x + track.width
                                && rect.y >= track.y
                                && rect.y + rect.height <= track.y + track.height =>
                        {
                            Some(*rect)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("no thumb in {id}"));
                let on_end = thumb.x > track.x + track.width / 2.0;
                assert_eq!(on_end, id == "sync", "{id}");
                // Its label stays left of the track.
                let label = if id == "sync" { "Sync every hour" } else { "Mirror changes" };
                let (label_at, _, _) = text_of(&frame, label);
                assert!(label_at.x < track.x, "{id}");
            }
            // Choosing a switch reports its action: the host flips the value.
            for id in ["sync", "mirror"] {
                let point = center(region(&tree.layout(size), id));
                tree.handle_event(UiEvent::PointerDown(point));
                assert_eq!(
                    tree.handle_event(UiEvent::PointerUp(point)),
                    vec![UiAction::Clicked { id: id.into() }]
                );
            }
        }
    }

    #[test]
    fn a_two_line_switch_item_keeps_its_track_inside_its_row() {
        let palette = ComponentPalette::neutral_dark();
        let menu = DropdownMenu::new(
            trigger(),
            vec![
                DropdownMenuItem::new("sync", "Sync")
                    .description("Every hour")
                    .switch(true)
                    .into(),
                DropdownMenuItem::new("mark", "Mark as read")
                    .description("Hides the badge")
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(260.0);
        let mut tree = open_menu(menu);
        let frame = tree.layout(Size::new(640.0, 480.0));
        let row = region(&frame, "sync");
        let track = switch_track(&frame, row).expect("no track in a two-line switch");
        assert!(track.y >= row.y && track.y + track.height <= row.y + row.height);
        assert!(track.x + track.width <= row.x + row.width);
    }

    #[test]
    fn a_described_item_takes_the_menus_own_sizing_and_says_why_it_is_disabled() {
        let palette = ComponentPalette::neutral_dark();
        let sizing = SelectorSizing {
            row_height: 38.0,
            detail_row_height: 54.0,
            icon_size: 20.0,
            label_size: 13.0,
            detail_size: 12.0,
            padding: 8.0,
            gap: 10.0,
            ..SelectorSizing::COMPACT
        };
        let reason = "Needs Manage Server";
        let menu = DropdownMenu::new(
            trigger(),
            vec![
                DropdownMenuItem::new("invite", "Invite people")
                    .icon(Icon::Edit)
                    .into(),
                DropdownMenuItem::new("edit", "Server settings")
                    .icon(Icon::Settings)
                    .description(reason)
                    .disabled()
                    .into(),
                DropdownMenuItem::new("level", "Notifications")
                    .icon(Icon::Bell)
                    .description("Only mentions")
                    .into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .sizing(sizing)
        .width(300.0);
        let mut tree = open_menu(menu);
        let frame = tree.layout(Size::new(640.0, 480.0));
        let level = region(&frame, "level");
        assert_eq!(level.height, sizing.detail_row_height);
        // The two-line rows line up with the one-line one, at the profile's sizes.
        let (invite, ..) = text_of(&frame, "Invite people");
        let (notifications, label_size, _) = text_of(&frame, "Notifications");
        let (detail, detail_size, detail_ink) = text_of(&frame, "Only mentions");
        assert_eq!(
            notifications.x,
            level.x + sizing.padding + sizing.icon_size + sizing.gap
        );
        assert_eq!((invite.x, detail.x), (notifications.x, notifications.x));
        assert_eq!(
            (label_size, detail_size, detail_ink),
            (13.0, 12.0, palette.muted)
        );
        // Disabled, the row is passive and muted, and the reason is said.
        let (settings, _, settings_ink) = text_of(&frame, "Server settings");
        let (why, why_size, why_ink) = text_of(&frame, reason);
        assert_eq!((settings.x, why.x), (notifications.x, notifications.x));
        assert_eq!(
            (settings_ink, why_size, why_ink),
            (palette.muted, 12.0, palette.muted)
        );
        assert!(!has(&frame, "edit"));
        let node = frame
            .access
            .iter()
            .find(|node| node.id == "edit")
            .expect("the disabled item is known");
        assert_eq!(
            (node.role, node.name.as_str(), node.description.as_deref()),
            (Role::MenuItem, "Server settings", Some(reason))
        );
        assert!(node.disabled && !node.actionable);
        // The scroll budget counts the two-line rows at their own height.
        assert_eq!(
            fixed_entry_height(
                &DropdownMenuItem::new("a", "A").description("b").into(),
                sizing
            ),
            Some(54.0)
        );
    }

    #[test]
    fn a_described_item_with_a_submenu_ends_with_a_chevron_and_opens_it() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(900.0, 480.0);
        for contextual in [false, true] {
            let menu = DropdownMenu::new(
                trigger(),
                vec![
                    DropdownMenuItem::new("notify", "Notifications")
                        .description("Only mentions")
                        .submenu(vec![
                            DropdownMenuItem::new("notify:all", "All messages")
                                .radio(false)
                                .into(),
                            DropdownMenuItem::new("notify:mentions", "Only @mentions")
                                .radio(true)
                                .into(),
                        ])
                        .into(),
                    DropdownMenuItem::new("locked", "Mute")
                        .description("Not allowed here")
                        .submenu(vec![DropdownMenuItem::new("mute:1h", "For an hour").into()])
                        .disabled()
                        .into(),
                    DropdownMenuItem::new("other", "Copy link").into(),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(260.0);
            let mut tree = open_menu(if contextual {
                menu.context_menu_style()
            } else {
                menu
            });
            let frame = tree.layout(size);
            let row = region(&frame, "notify");
            assert_eq!(row.height, SelectorSizing::COMPACT.detail_row_height);
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Icon { icon: Icon::ChevronRight, rect, tint }
                    if *tint == palette.muted && rect.y >= row.y && rect.y + rect.height <= row.y + row.height)));
            let node = frame
                .access
                .iter()
                .find(|node| node.id == "notify")
                .unwrap();
            assert_eq!(
                (node.description.as_deref(), node.selected),
                (Some("Only mentions"), None)
            );
            assert!(!has(&frame, "notify:all"));
            // On hover it opens beside the row, and its items can be chosen.
            tree.handle_event(UiEvent::PointerMove(center(row)));
            let frame = tree.layout(size);
            let all = region(&frame, "notify:all");
            assert!(all.x >= row.x + row.width - 1.0);
            let point = center(region(&frame, "notify:mentions"));
            tree.handle_event(UiEvent::PointerMove(point));
            tree.layout(size);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "notify:mentions".into()
                }]
            );
            // A disabled one does not open, whatever the pointer does.
            let (mute, ..) = text_of(&tree.layout(size), "Mute");
            tree.handle_event(UiEvent::PointerMove(Point::new(mute.x + 4.0, mute.y + 4.0)));
            assert!(!has(&tree.layout(size), "mute:1h"));
            // Right opens it from the keyboard and Left goes back to the row.
            let other = center(region(&tree.layout(size), "other"));
            tree.handle_event(UiEvent::PointerMove(other));
            tree.layout(size);
            assert!(tree.focus("notify"));
            tree.layout(size);
            key(&mut tree, Key::ArrowRight);
            assert_eq!(tree.focused_id(), Some("notify:all"));
            assert!(has(&tree.layout(size), "notify:mentions"));
            key(&mut tree, Key::ArrowLeft);
            assert_eq!(tree.focused_id(), Some("notify"));
            assert!(!has(&tree.layout(size), "notify:all"));
        }
    }

    fn nested_menu(contextual: bool) -> Element {
        let menu = DropdownMenu::new(
            trigger(),
            vec![
                DropdownMenuItem::new("status", "Status")
                    .submenu(vec![
                        DropdownMenuItem::new("online", "Online").into(),
                        DropdownMenuItem::new("idle", "Idle")
                            .submenu(vec![
                                DropdownMenuItem::new("idle:15m", "For 15 minutes").into(),
                                DropdownMenuItem::new("idle:locked", "Until I change it")
                                    .tooltip_with(TooltipSpec::new("Ask an admin"))
                                    .disabled()
                                    .into(),
                                DropdownMenuItem::new("idle:1h", "For an hour").into(),
                            ])
                            .into(),
                    ])
                    .into(),
                DropdownMenuItem::new("more", "More")
                    .submenu(vec![
                        DropdownMenuItem::new("more:archive", "Archive").into(),
                    ])
                    .into(),
                DropdownMenuItem::new("copy", "Copy ID").into(),
            ],
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        );
        // Narrow enough for three levels side by side.
        if contextual {
            menu.context_menu_style()
        } else {
            menu
        }
        .width(220.0)
        .render()
    }

    #[test]
    fn a_submenu_stays_open_while_the_pointer_is_in_one_nested_in_it() {
        let size = Size::new(900.0, 480.0);
        for contextual in [false, true] {
            let mut tree = UiTree::new(nested_menu(contextual));
            tree.set_reduced_motion(true);
            let frame = tree.layout(size);
            tree.handle_event(UiEvent::PointerMove(center(region(&frame, "status"))));
            let frame = tree.layout(size);
            tree.handle_event(UiEvent::PointerMove(center(region(&frame, "idle"))));
            let frame = tree.layout(size);
            tree.handle_event(UiEvent::PointerMove(center(region(&frame, "idle:15m"))));
            // Over the nested submenu's item, its disabled item with a hint,
            // and its surface: every level stays open, frame after frame.
            let (locked, ..) = text_of(&tree.layout(size), "Until I change it");
            let surface = region(&tree.layout(size), "popover_surface:dismiss:status:idle");
            for point in [
                Point::new(locked.x + 4.0, locked.y + 4.0),
                Point::new(surface.x + 2.0, surface.y + 1.0),
            ] {
                tree.handle_event(UiEvent::PointerMove(point));
                for _ in 0..2 {
                    let frame = tree.layout(size);
                    for id in ["status", "online", "idle", "idle:15m", "idle:1h"] {
                        assert!(has(&frame, id), "{id} is shown (contextual: {contextual})");
                    }
                }
            }
            let point = center(region(&tree.layout(size), "idle:1h"));
            tree.handle_event(UiEvent::PointerMove(point));
            tree.layout(size);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "idle:1h".into()
                }]
            );
        }
        // Another row of the first menu closes both submenus.
        for contextual in [false, true] {
            let mut tree = UiTree::new(nested_menu(contextual));
            tree.set_reduced_motion(true);
            for id in ["status", "idle", "idle:15m", "copy"] {
                let point = center(region(&tree.layout(size), id));
                tree.handle_event(UiEvent::PointerMove(point));
            }
            let frame = tree.layout(size);
            assert!(
                !has(&frame, "online") && !has(&frame, "idle:15m"),
                "contextual: {contextual}"
            );
        }
    }

    #[test]
    fn the_pointer_on_another_submenu_row_closes_the_ones_the_keyboard_opened() {
        let size = Size::new(900.0, 480.0);
        for contextual in [false, true] {
            let mut tree = UiTree::new(nested_menu(contextual));
            tree.set_reduced_motion(true);
            tree.layout(size);
            assert!(tree.focus("status"));
            tree.layout(size);
            for pressed in [Key::ArrowRight, Key::ArrowDown, Key::ArrowRight] {
                key(&mut tree, pressed);
                tree.layout(size);
            }
            assert_eq!(tree.focused_id(), Some("idle:15m"));
            let more = center(region(&tree.layout(size), "more"));
            tree.handle_event(UiEvent::PointerMove(more));
            let frame = tree.layout(size);
            assert!(has(&frame, "more:archive"), "contextual: {contextual}");
            assert!(
                !has(&frame, "online") && !has(&frame, "idle:15m"),
                "one submenu at a time (contextual: {contextual})"
            );
        }
    }

    #[test]
    fn the_arrow_keys_walk_into_a_nested_submenu_and_back() {
        let size = Size::new(900.0, 480.0);
        for contextual in [false, true] {
            let mut tree = UiTree::new(nested_menu(contextual));
            tree.set_reduced_motion(true);
            tree.layout(size);
            assert!(tree.focus("status"));
            tree.layout(size);
            let step = |tree: &mut UiTree, pressed: Key, focused: &str| {
                key(tree, pressed);
                assert_eq!(
                    tree.focused_id(),
                    Some(focused),
                    "{pressed:?} (contextual: {contextual})"
                );
                tree.layout(size)
            };
            step(&mut tree, Key::ArrowRight, "online");
            step(&mut tree, Key::ArrowDown, "idle");
            let frame = step(&mut tree, Key::ArrowRight, "idle:15m");
            assert!(has(&frame, "online"), "the parent submenu stays open");
            // The disabled item is skipped.
            step(&mut tree, Key::ArrowDown, "idle:1h");
            assert_eq!(
                key(&mut tree, Key::Enter),
                vec![UiAction::Clicked {
                    id: "idle:1h".into()
                }]
            );
            let frame = step(&mut tree, Key::ArrowLeft, "idle");
            assert!(!has(&frame, "idle:15m") && has(&frame, "online"));
            let frame = step(&mut tree, Key::ArrowLeft, "status");
            assert!(!has(&frame, "online"));
        }
    }

    #[test]
    fn a_menu_at_a_point_has_its_corner_there_and_takes_the_arrow_keys() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(800.0, 600.0);
        let view = |at: Point, contextual: bool| {
            let menu = DropdownMenu::new(
                Element::text("general").size(None, Some(400.0)),
                vec![
                    DropdownMenuItem::new("read", "Mark as read").into(),
                    DropdownMenuItem::new("mute", "Mute channel")
                        .description("Until I turn it back on")
                        .submenu(vec![DropdownMenuItem::new("mute:1h", "For an hour").into()])
                        .into(),
                    DropdownMenuItem::new("hide", "Hide muted channels")
                        .checkbox(true)
                        .into(),
                    DropdownMenuItem::new("edit", "Edit channel")
                        .description("You need Manage Channels")
                        .disabled()
                        .into(),
                ],
                true,
                "menu:dismiss",
                palette,
            )
            .width(240.0)
            .at(at);
            let menu = if contextual {
                menu.context_menu_style()
            } else {
                menu
            };
            let mut tree = UiTree::new(Element::column(vec![
                Element::text_input("search", "", "Search"),
                menu.render(),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            tree
        };
        for contextual in [false, true] {
            let mut tree = view(Point::new(120.0, 90.0), contextual);
            let surface = region(&tree.layout(size), "popover_surface:menu:dismiss");
            assert_eq!((surface.x, surface.y), (120.0, 90.0));
            // With no room after and under the point, it goes above and is moved in.
            let corner = region(
                &view(Point::new(790.0, 590.0), contextual).layout(size),
                "popover_surface:menu:dismiss",
            );
            assert_eq!(corner.y + corner.height, 590.0);
            assert_eq!(corner.x + corner.width, 800.0 - 8.0);
            // The arrow keys reach it while the focus is on a field outside it.
            assert!(tree.focus("search"));
            tree.layout(size);
            key(&mut tree, Key::ArrowDown);
            assert_eq!(tree.focused_id(), Some("read"));
            tree.layout(size);
            key(&mut tree, Key::ArrowDown);
            tree.layout(size);
            key(&mut tree, Key::ArrowRight);
            assert_eq!(tree.focused_id(), Some("mute:1h"));
            assert!(has(&tree.layout(size), "mute:1h"));
            key(&mut tree, Key::ArrowLeft);
            assert_eq!(tree.focused_id(), Some("mute"));
            tree.layout(size);
            assert_eq!(
                key(&mut tree, Key::Escape),
                vec![UiAction::Clicked {
                    id: "menu:dismiss".into()
                }]
            );
            // Its submenu opens on hover too.
            let mut tree = view(Point::new(120.0, 90.0), contextual);
            let mute = region(&tree.layout(size), "mute");
            tree.handle_event(UiEvent::PointerMove(center(mute)));
            assert!(has(&tree.layout(size), "mute:1h"));
        }
        // A point that is not finite is ignored: the menu opens against its anchor.
        let menu = |at: Point| {
            DropdownMenu::new(
                trigger(),
                vec![DropdownMenuItem::new("a", "A").into()],
                true,
                "d",
                palette,
            )
            .at(at)
        };
        assert_eq!(menu(Point::new(f32::NAN, 4.0)).at, None);
        assert_eq!(menu(Point::new(4.0, f32::INFINITY)).at, None);
        let mut tree = open_menu(menu(Point::new(f32::NEG_INFINITY, 4.0)));
        let frame = tree.layout(size);
        assert!(region(&frame, "popover_surface:d").y >= region(&frame, "trigger").y + 28.0);
    }

    #[test]
    fn a_secondary_click_outside_closes_the_menu_and_one_inside_does_not() {
        let palette = ComponentPalette::neutral_dark();
        let size = Size::new(800.0, 600.0);
        let menu_click = |id: &str| vec![UiAction::Clicked { id: id.into() }];
        for at in [None, Some(Point::new(120.0, 90.0))] {
            let mut menu = DropdownMenu::new(
                trigger(),
                vec![DropdownMenuItem::new("read", "Mark as read").into()],
                true,
                "menu:dismiss",
                palette,
            )
            .context_menu_style();
            if let Some(at) = at {
                menu = menu.at(at);
            }
            let mut tree = UiTree::new(Element::column(vec![
                Element::text_input("search", "", "Search"),
                menu.render(),
                Element::spacer().flex(1.0),
            ]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(size);
            let (read, surface) = (
                region(&frame, "read"),
                region(&frame, "popover_surface:menu:dismiss"),
            );
            let search = center(region(&frame, "search"));
            for (point, expected) in [
                (center(read), vec![]),
                (Point::new(surface.x + 1.0, surface.y + 1.0), vec![]),
                (Point::new(700.0, 500.0), menu_click("menu:dismiss")),
                (search, menu_click("menu:dismiss")),
            ] {
                assert_eq!(
                    tree.handle_event(UiEvent::ContextMenu { position: point }),
                    expected,
                    "{point:?} (at: {at:?})"
                );
                tree.layout(size);
            }
        }
        // A control of the popover that reports its dismiss ID is not outside it.
        let mut tree = UiTree::new(Element::popover(
            trigger(),
            Element::button("close", "Close"),
            "close",
            PopoverPlacement::BottomStart,
        ));
        tree.set_reduced_motion(true);
        // The backdrop reports the same ID: the button is the last region.
        let frame = tree.layout(size);
        let button = frame
            .regions
            .iter()
            .rev()
            .find(|region| region.id == "close");
        let close = center(button.unwrap().rect);
        assert_eq!(
            tree.handle_event(UiEvent::ContextMenu { position: close }),
            vec![]
        );
        assert_eq!(
            tree.handle_event(UiEvent::ContextMenu {
                position: Point::new(700.0, 500.0)
            }),
            menu_click("close")
        );
    }
}
