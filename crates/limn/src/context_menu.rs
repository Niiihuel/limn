//! Pointer-anchored actions with one target per row. The tree owns opening,
//! dismissal and keyboard navigation; hosts own the meaning of custom IDs.
//! Original native implementation; UX reference: Rust/UI Context Menu.

use crate::menu::action_style::{
    self, GAP, HORIZONTAL_PADDING, SCROLLBAR_GUTTER, SEPARATOR_HEIGHT,
};
use crate::{Component, ComponentPalette, Element, FontFamily, Icon, Kbd, ScrollArea};

#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuItem {
    pub(crate) action_id: String,
    pub(crate) label: String,
    pub(crate) shortcut: Option<String>,
    pub(crate) icon: Option<Icon>,
    pub(crate) enabled: bool,
}

impl ContextMenuItem {
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            shortcut: None,
            icon: None,
            enabled: true,
        }
    }
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContextMenuEntry {
    Item(ContextMenuItem),
    Separator,
}

#[derive(Clone, Debug)]
pub(crate) struct ContextMenuDefinition {
    pub id: String,
    pub entries: Vec<ContextMenuEntry>,
    pub palette: ComponentPalette,
}

/// Attach a contextual menu without adding layout, a primary-click target,
/// or changing the child's focus/selection behavior.
pub struct ContextMenu {
    child: Element,
    definition: ContextMenuDefinition,
}

impl ContextMenu {
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
            },
        }
    }
}

impl Component for ContextMenu {
    fn render(mut self) -> Element {
        self.child.context_menu = Some(self.definition);
        self.child
    }
}

pub(crate) use action_style::{INSET, LABEL_FONT_SIZE, MAX_HEIGHT, ROW_HEIGHT, WIDTH};
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
) -> RowLayout {
    let ContextMenuEntry::Item(item) = entry else {
        return RowLayout {
            height: SEPARATOR_HEIGHT,
            label_width: 0.0,
            label_height: 0.0,
            shortcut: None,
        };
    };
    let available =
        (row_width - HORIZONTAL_PADDING * 2.0 - if reserve_icon { 16.0 + GAP } else { 0.0 })
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
            (width + GAP + 40.0 <= available).then_some((value.to_owned(), width))
        });
    let label_width =
        (available - shortcut.as_ref().map_or(0.0, |(_, width)| width + GAP)).max(0.0);
    let lines = crate::wrap_measured(&item.label, label_width, |part| {
        measure(part, LABEL_FONT_SIZE)
    });
    let label_height = lines.len().max(1) as f32 * line_height;
    RowLayout {
        height: (label_height + 6.0).max(ROW_HEIGHT),
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
) -> (Element, f32) {
    let inner = (width - INSET * 2.0).max(0.0);
    let reserve_icon = entries
        .iter()
        .any(|entry| matches!(entry, ContextMenuEntry::Item(item) if item.icon.is_some()));
    let layouts_at = |row_width| {
        entries
            .iter()
            .map(|entry| row_layout(entry, row_width, reserve_icon, measure, line_height))
            .collect::<Vec<_>>()
    };
    let mut layouts = layouts_at(inner);
    let natural_height =
        |rows: &[RowLayout]| rows.iter().map(|row| row.height).sum::<f32>() + INSET * 2.0;
    let scrolling = natural_height(&layouts) > max_height;
    let row_width = (inner - if scrolling { SCROLLBAR_GUTTER } else { 0.0 }).max(0.0);
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
                    children.push(
                        centered(icon.size(Some(16.0), Some(16.0)))
                            .size(Some(16.0), Some(layout.height)),
                    );
                }
                children.push(
                    centered(
                        Element::text(&item.label)
                            .font_family(FontFamily::Sans)
                            .font_size(LABEL_FONT_SIZE)
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
                    Element::spacer().size(Some(HORIZONTAL_PADDING), None),
                    Element::row(children).gap(GAP).flex(1.0),
                    Element::spacer().size(Some(HORIZONTAL_PADDING), None),
                ]);
                let row = if item.enabled {
                    Element::clickable(row_id(index), content).action_menu_item()
                } else {
                    Element::panel(content)
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
        ),
        height,
    )
}

pub(crate) fn row_id(index: usize) -> String {
    format!("__context_menu:item:{index}")
}
