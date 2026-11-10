//! Pointer-anchored actions with one target per row. The tree owns opening,
//! dismissal and keyboard navigation; hosts own the meaning of custom IDs.
//! Original native implementation; UX reference: Rust/UI Context Menu.

use crate::action_style::{
    self, GAP, HORIZONTAL_PADDING, SCROLLBAR_GUTTER, SEPARATOR_HEIGHT,
};
use crate::{Access, Color, Component, ComponentPalette, Element, FontFamily, Icon, Kbd, Role, ScrollArea, Size};
use std::sync::Arc;

/// One action in a [`ContextMenu`]: a label with an optional icon and shortcut.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuItem {
    /// What the host receives when it is chosen. The tree's own menus of text use `cut`,
    /// `copy`, `paste`, `select` and `select_all`.
    pub action_id: String,
    /// Its name, for the row and for the platform.
    pub label: String,
    /// Its keys, as `Ctrl+Shift+Z`: pressed while the menu is open, they choose it.
    pub shortcut: Option<String>,
    /// Its leading icon.
    pub icon: Option<Icon>,
    /// Whether it can be chosen. A disabled item stays in the menu, known to the platform as
    /// disabled.
    pub enabled: bool,
}

impl ContextMenuItem {
    /// An enabled item with this label and no icon or shortcut. `action_id` is what the
    /// host receives when it is chosen.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            shortcut: None,
            icon: None,
            enabled: true,
        }
    }
    /// Show a keyboard shortcut at the row's trailing edge. A blank one is not drawn.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
    /// A leading icon for the row.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// A disabled item is drawn in muted ink and is passive: it stays in the menu, known to
    /// the platform as disabled, and cannot be chosen.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

/// One row of a context menu: an action, or a line between groups of actions.
#[derive(Clone, Debug, PartialEq)]
pub enum ContextMenuEntry {
    /// An action the pointer or the keyboard can choose.
    Item(ContextMenuItem),
    /// A line between two groups of actions. It is not a target.
    Separator,
}

#[derive(Clone, Debug)]
pub(crate) struct ContextMenuDefinition {
    pub id: String,
    pub entries: Vec<ContextMenuEntry>,
    pub palette: ComponentPalette,
    pub open_action: Option<String>,
    pub view: Option<ContextMenuViewFn>,
}

/// Attach a contextual menu without adding layout, a primary-click target,
/// or changing the child's focus/selection behavior.
pub struct ContextMenu {
    child: Element,
    definition: ContextMenuDefinition,
}

impl ContextMenu {
    /// Attach `entries` to `child` as its context menu. `id` identifies the menu; the
    /// child's layout and its own actions are unchanged.
    pub fn new(
        id: impl Into<String>,
        child: Element,
        entries: Vec<ContextMenuEntry>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            child,
            definition: ContextMenuDefinition {
                id: id.into(),
                entries,
                palette,
                open_action: None,
                view: None,
            },
        }
    }

    /// Draw this menu with `view` rather than limn's look, or the tree's
    /// (`UiTree::set_context_menu_view`). The tree still opens, places, drives and dismisses
    /// it; [`ContextMenuView`] says what the drawing owes it.
    pub fn view(mut self, view: ContextMenuViewFn) -> Self {
        self.definition.view = Some(view);
        self
    }

    /// Request a host-controlled surface instead of opening the built-in menu.
    /// Pointer context requests and the keyboard shortcut emit `UiAction::Clicked`
    /// with this ID. The host owns the surface's content and open state.
    pub fn open_action(mut self, id: impl Into<String>) -> Self {
        self.definition.open_action = Some(id.into());
        self
    }
}

impl Component for ContextMenu {
    fn render(mut self) -> Element {
        self.child.context_menu = Some(Box::new(self.definition));
        self.child
    }
}

/// What a context menu is drawn from: its entries, and the room and measures the tree gives
/// it. A [`ContextMenuViewFn`] turns it into the menu's element. The tree keeps the rest: it
/// opens the menu, places it in the window, lights its active row, moves that row with the
/// pointer and the keyboard, scrolls it, runs its shortcuts and dismisses it.
///
/// The tree knows the menu's parts by their IDs, so a view owes it this:
///
/// - Entry `i` of `entries`, separators counted, is row `i`. An enabled entry is one
///   [`Element::clickable`] with the ID [`ContextMenuView::row_id`]`(i)`, opted into menu
///   focus with [`Element::action_menu_item`]. The tree reports a click on it as the entry's
///   action, lights it under the pointer or as the keyboard's row, and Enter or Space
///   chooses it. The row shows it is lit with its own [`Element::hover_background`], and
///   pressed with its [`Element::pressed_background`].
/// - A disabled entry and a separator take no pointer. The tree never chooses them, but it
///   would light one under the pointer that took it with a row's ID.
/// - Rows that do not fit in `max_height` scroll in one scroll area ([`Element::scroll`] or
///   [`ScrollArea`]) with the ID [`ContextMenuView::scroll_id`]. The tree starts it at its
///   top each time the menu opens, and lets its rail be dragged; under another ID, a press on
///   the rail closes the menu.
/// - A press on no row and not on that rail closes the menu, as one outside does, and what
///   was pressed is not told. Only a press where nothing in the menu has a hit region leaves
///   it open. A scroll area has one, so in limn's look a press on a separator or a disabled
///   row closes the menu.
/// - The size returned is the element's: the tree places the element in a rectangle of that
///   size, and the wheel in that rectangle scrolls the menu.
pub struct ContextMenuView<'a> {
    /// The menu's rows, in order.
    pub entries: &'a [ContextMenuEntry],
    /// The menu's colors: those given to [`ContextMenu::new`], or for a menu of text the
    /// tree's, from [`UiTree::set_context_menu_palette`](crate::UiTree::set_context_menu_palette).
    pub palette: ComponentPalette,
    /// The width of a pointer's menu, in logical pixels: the host's sizing, else 352, and at
    /// most the window's width less 16. limn's touch bar takes the width its actions need.
    pub width: f32,
    /// The tallest the menu may be, in logical pixels: the window's height less 16. Rows that
    /// would make it taller scroll.
    pub max_height: f32,
    /// The width of one line of text in the sans face at a font size, both in logical pixels:
    /// the host's measurer, from [`UiTree::set_text_measurer`](crate::UiTree::set_text_measurer),
    /// else limn's estimate.
    pub measure: &'a dyn Fn(&str, f32) -> f32,
    /// The height of one line of a label at `label_size`, in logical pixels.
    pub line_height: f32,
    /// The least height of a row, in logical pixels. This and the measures after it are
    /// limn's own, or the host's from
    /// [`UiTree::set_context_menu_sizing`](crate::UiTree::set_context_menu_sizing).
    pub row_height: f32,
    /// The font size of a row's label, in logical pixels.
    pub label_size: f32,
    /// The width of the slot kept for a row's icon, in logical pixels.
    pub icon_size: f32,
    /// The space between a row's edge and its content, in logical pixels.
    pub padding: f32,
    /// The space between a row's icon, its label and its shortcut, in logical pixels.
    pub gap: f32,
    /// The width the rows give up to the scrollbar while the menu scrolls, in logical pixels.
    pub scrollbar_gutter: f32,
    /// `Some` for a menu of text on a touch screen, with the glass of
    /// [`UiTree::set_touch_text_menu`](crate::UiTree::set_touch_text_menu): the tree centres
    /// it on the press, over what was held or under it. `None` for a pointer's menu, which the
    /// tree places at the press, and always for a menu given with [`ContextMenu`].
    pub touch: Option<crate::GlassMaterial>,
}

impl ContextMenuView<'_> {
    /// The ID row `index` must have, separators counted. It names the row in the frame's hit
    /// regions and accessibility nodes; the host's actions carry the entry's own ID instead.
    pub fn row_id(index: usize) -> String {
        row_id(index)
    }

    /// The ID the scroll area that holds the rows must have when they scroll.
    pub fn scroll_id() -> &'static str {
        SCROLL_ID
    }

    /// limn's own look, as the tree draws a menu with no view: rows of an icon, a label and a
    /// shortcut's keycaps on a raised surface, or iOS's edit bar when `touch` is set. A view
    /// that only wraps that look calls this.
    pub fn default_element(&self) -> (Element, Size) {
        if let Some(material) = self.touch {
            let (bar, width) = edit_bar(self.entries, self.palette, material, self.measure);
            return (bar, Size::new(width, EDIT_BAR_HEIGHT));
        }
        let sizes = Sizes {
            width: self.width,
            row_height: self.row_height,
            label_size: self.label_size,
            icon_size: self.icon_size,
            padding: self.padding,
            gap: self.gap,
            gutter: self.scrollbar_gutter,
        };
        let (surface, height) = surface(self.entries, self.palette, self.width, self.max_height, self.measure, self.line_height, sizes);
        (surface, Size::new(self.width, height))
    }
}

/// A context menu's drawing: from a [`ContextMenuView`], the menu's element and its size in
/// logical pixels. One menu takes it from [`ContextMenu::view`]; every other menu, a field's
/// included, from [`UiTree::set_context_menu_view`](crate::UiTree::set_context_menu_view).
/// A clone shares the function.
#[derive(Clone)]
pub struct ContextMenuViewFn(Arc<dyn Fn(&ContextMenuView<'_>) -> (Element, Size) + Send + Sync>);

impl ContextMenuViewFn {
    /// `draw`, which the tree calls at each layout while the menu is open.
    pub fn new(draw: impl Fn(&ContextMenuView<'_>) -> (Element, Size) + Send + Sync + 'static) -> Self {
        Self(Arc::new(draw))
    }

    pub(crate) fn draw(&self, view: &ContextMenuView<'_>) -> (Element, Size) {
        (self.0)(view)
    }
}

impl std::fmt::Debug for ContextMenuViewFn {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ContextMenuViewFn")
    }
}

pub(crate) use action_style::{INSET, LABEL_FONT_SIZE, ROW_HEIGHT, WIDTH};

/// The measures a context menu is drawn at: limn's own, or those of the host's menus,
/// from `UiTree::set_context_menu_sizing`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sizes {
    pub width: f32,
    pub row_height: f32,
    pub label_size: f32,
    pub icon_size: f32,
    pub padding: f32,
    pub gap: f32,
    pub gutter: f32,
}

impl Default for Sizes {
    fn default() -> Self {
        Self {
            width: WIDTH,
            row_height: ROW_HEIGHT,
            label_size: LABEL_FONT_SIZE,
            icon_size: 16.0,
            padding: HORIZONTAL_PADDING,
            gap: GAP,
            gutter: SCROLLBAR_GUTTER,
        }
    }
}

impl From<crate::SelectorSizing> for Sizes {
    fn from(sizing: crate::SelectorSizing) -> Self {
        Self {
            width: sizing.preferred_width,
            row_height: sizing.row_height,
            label_size: sizing.label_size,
            icon_size: sizing.icon_size,
            padding: sizing.padding,
            gap: sizing.gap,
            gutter: sizing.scrollbar_gutter,
        }
    }
}
pub(crate) const SCROLL_ID: &str = "__context_menu:scroll";

struct RowLayout {
    height: f32,
    label_width: f32,
    label_height: f32,
    shortcut: Option<(String, f32)>,
}

fn row_layout(
    entry: &ContextMenuEntry,
    row_width: f32,
    reserve_icon: bool,
    measure: &dyn Fn(&str, f32) -> f32,
    line_height: f32,
    sizes: Sizes,
) -> RowLayout {
    let ContextMenuEntry::Item(item) = entry else {
        return RowLayout {
            height: SEPARATOR_HEIGHT,
            label_width: 0.0,
            label_height: 0.0,
            shortcut: None,
        };
    };
    let available = (row_width
        - sizes.padding * 2.0
        - if reserve_icon { sizes.icon_size + sizes.gap } else { 0.0 })
    .max(0.0);
    let shortcut = item
        .shortcut
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| {
            let width =
                crate::shortcut::width(value, |key| measure(key, crate::shortcut::FONT_SIZE))
                    .ceil();
            // Keep both the full shortcut and a readable label column. Long
            // labels wrap using real font metrics; only very narrow viewports
            // omit the entire shortcut, never part of a chord.
            (width + sizes.gap + 40.0 <= available).then_some((value.to_owned(), width))
        });
    let label_width =
        (available - shortcut.as_ref().map_or(0.0, |(_, width)| width + sizes.gap)).max(0.0);
    let lines = crate::wrap_measured(&item.label, label_width, |part| {
        measure(part, sizes.label_size)
    });
    let label_height = lines.len().max(1) as f32 * line_height;
    RowLayout {
        height: (label_height + 6.0).max(sizes.row_height),
        label_width,
        label_height,
        shortcut,
    }
}

pub(crate) fn surface(
    entries: &[ContextMenuEntry],
    palette: ComponentPalette,
    width: f32,
    max_height: f32,
    measure: &dyn Fn(&str, f32) -> f32,
    line_height: f32,
    sizes: Sizes,
) -> (Element, f32) {
    let inner = (width - INSET * 2.0).max(0.0);
    let reserve_icon = entries
        .iter()
        .any(|entry| matches!(entry, ContextMenuEntry::Item(item) if item.icon.is_some()));
    let layouts_at = |row_width| {
        entries
            .iter()
            .map(|entry| row_layout(entry, row_width, reserve_icon, measure, line_height, sizes))
            .collect::<Vec<_>>()
    };
    let mut layouts = layouts_at(inner);
    let natural_height =
        |rows: &[RowLayout]| rows.iter().map(|row| row.height).sum::<f32>() + INSET * 2.0;
    let scrolling = natural_height(&layouts) > max_height;
    let row_width = (inner - if scrolling { sizes.gutter } else { 0.0 }).max(0.0);
    if scrolling {
        // Reserving the rail can add wrapped lines. Use that final width for
        // both text paint and the scroll extent, without guessing row counts.
        layouts = layouts_at(row_width);
    }
    let height = natural_height(&layouts).min(max_height);
    let rows = entries
        .iter()
        .zip(layouts)
        .enumerate()
        .map(|(index, (entry, layout))| match entry {
            ContextMenuEntry::Separator => Element::panel(
                Element::spacer()
                    .background(palette.border)
                    .size(None, Some(1.0)),
            )
            .padding((SEPARATOR_HEIGHT - 1.0) / 2.0)
            .size(Some(row_width), Some(layout.height)),
            ContextMenuEntry::Item(item) => {
                let ink = if item.enabled {
                    palette.text
                } else {
                    palette.muted
                };
                let centered = |child| {
                    Element::column(vec![
                        Element::spacer().flex(1.0),
                        child,
                        Element::spacer().flex(1.0),
                    ])
                    .size(None, Some(layout.height))
                };
                let mut children = Vec::new();
                if reserve_icon {
                    let icon = item
                        .icon
                        .map_or_else(Element::spacer, |icon| Element::icon(icon).foreground(ink));
                    let side = sizes.icon_size;
                    children.push(
                        centered(icon.size(Some(side), Some(side)))
                            .size(Some(side), Some(layout.height)),
                    );
                }
                children.push(
                    centered(
                        Element::text(&item.label)
                            .font_family(FontFamily::Sans)
                            .font_size(sizes.label_size)
                            .foreground(ink)
                            .size(Some(layout.label_width), Some(layout.label_height)),
                    )
                    .size(Some(layout.label_width), Some(layout.height)),
                );
                if let Some((keys, shortcut_width)) = layout.shortcut {
                    children.push(centered(
                        Kbd::new(
                            keys,
                            ComponentPalette {
                                text: ink,
                                ..palette
                            },
                        )
                        .max_width(shortcut_width)
                        .render()
                        .size(Some(shortcut_width), Some(crate::shortcut::HEIGHT)),
                    ));
                }
                let content = Element::row(vec![
                    Element::spacer().size(Some(sizes.padding), None),
                    Element::row(children).gap(sizes.gap).flex(1.0),
                    Element::spacer().size(Some(sizes.padding), None),
                ]);
                // The label alone names the row: its keys are not part of what it is called.
                let access = Access::new(Role::MenuItem).name(&item.label);
                let row = if item.enabled {
                    Element::clickable(row_id(index), content).action_menu_item().accessible(access)
                } else {
                    Element::panel(content).accessible(access.id(row_id(index)).disabled(true))
                };
                palette
                    .interactive_surface(row)
                    .radius(action_style::row_radius())
                    .transition_ms(0)
                    .size(Some(row_width), Some(layout.height))
            }
        })
        .collect();
    (
        action_style::surface(
            ScrollArea::new(SCROLL_ID, Element::column(rows))
                .tint(palette.muted)
                .render()
                .size(Some(inner), Some((height - INSET * 2.0).max(0.0))),
            palette,
            width,
            Some(height),
            INSET,
            None,
            None,
        ),
        height,
    )
}

/// iOS 26's edit menu: a capsule this tall.
pub(crate) const EDIT_BAR_HEIGHT: f32 = 38.0;
/// Room around each action's name in it, and the size of that name.
const EDIT_ITEM_PAD: f32 = 14.0;
const EDIT_LABEL: f32 = 15.0;
/// Its hairlines between actions: white at 18%, 18 tall.
const EDIT_HAIRLINE: Color = Color::rgba(255, 255, 255, 46);

/// iOS 26's edit menu: the actions that can be done, by name, in a row in a capsule of
/// `material`, split by hairlines; and how wide it is. Each keeps its row's ID, so choosing it
/// is choosing that row. Lit under the finger only.
// shortcut: no "›" page for more actions than fit across the screen; text menus hold four
// at most. Add it, as iOS pages its bar, when a host's actions can join them.
pub(crate) fn edit_bar(
    entries: &[ContextMenuEntry],
    palette: ComponentPalette,
    material: crate::GlassMaterial,
    measure: &dyn Fn(&str, f32) -> f32,
) -> (Element, f32) {
    let mut children = Vec::new();
    let mut width = 0.0;
    for (index, item) in entries.iter().enumerate().filter_map(|(index, entry)| match entry {
        ContextMenuEntry::Item(item) if item.enabled => Some((index, item)),
        _ => None,
    }) {
        if !children.is_empty() {
            children.push(Element::column(vec![
                Element::spacer().flex(1.0),
                Element::spacer().background(EDIT_HAIRLINE).size(Some(1.0), Some(18.0)),
                Element::spacer().flex(1.0),
            ]).size(Some(1.0), Some(EDIT_BAR_HEIGHT)));
            width += 1.0;
        }
        let across = measure(&item.label, EDIT_LABEL).ceil() + EDIT_ITEM_PAD * 2.0;
        let label = Element::text(&item.label).font_family(FontFamily::Sans).font_size(EDIT_LABEL).foreground(palette.text).centered();
        let row = Element::clickable(row_id(index), label)
            .action_menu_item()
            .press_highlight()
            .accessible(Access::new(Role::MenuItem).name(&item.label));
        children.push(palette.interactive_surface(row).transition_ms(0).size(Some(across), Some(EDIT_BAR_HEIGHT)));
        width += across;
    }
    let bar = Element::row(children)
        .glass(material)
        .radius(EDIT_BAR_HEIGHT / 2.0)
        .clip_children()
        .size(Some(width), Some(EDIT_BAR_HEIGHT));
    (bar, width)
}

pub(crate) fn row_id(index: usize) -> String {
    format!("__context_menu:item:{index}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Size, UiEvent, UiTree};

    #[test]
    fn an_open_menu_tells_the_platform_each_item_by_its_label_enabled_or_not() {
        let entries = vec![
            ContextMenuEntry::Item(ContextMenuItem::new("rename", "Rename").shortcut("Ctrl+R")),
            ContextMenuEntry::Separator,
            ContextMenuEntry::Item(ContextMenuItem::new("delete", "Delete").icon(Icon::Trash).disabled()),
        ];
        let owner = Element::button("owner", "Editor").size(Some(180.0), Some(32.0));
        let mut tree = UiTree::new(ContextMenu::new("owner", owner, entries, ComponentPalette::neutral_dark()).render());
        tree.layout(Size::new(520.0, 400.0));
        tree.handle_event(UiEvent::ContextMenu { position: Point::new(20.0, 16.0) });
        let frame = tree.layout(Size::new(520.0, 400.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));
        let (rename, delete) = (node(&row_id(0)), node(&row_id(2)));
        assert_eq!((rename.role, rename.name.as_str()), (Role::MenuItem, "Rename"));
        assert!(rename.actionable && !rename.disabled);
        assert_eq!((delete.role, delete.name.as_str()), (Role::MenuItem, "Delete"));
        assert!(delete.disabled && !delete.actionable);
        assert!(frame.access.iter().all(|node| node.id != row_id(1)), "a separator says nothing");
    }
}
