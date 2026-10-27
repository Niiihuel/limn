//! Completions floating over a text input while the user keeps typing.

use std::ops::Range;
use std::sync::Arc;

use crate::{
    Access, Color, Component, ComponentPalette, Element, FontFamily, Icon, Popover, PopoverPlacement,
    Role, ScrollArea, ScrollbarAppearance, SelectorSizing, Separator, TextSpan,
};

// Two lines and their padding; taller than a catalog row because the list
// floats alone over an editor.
const ROW_HEIGHT: f32 = 42.0;
// One line and its padding, for a compact list.
const COMPACT_ROW_HEIGHT: f32 = 30.0;
const MAX_VISIBLE: usize = 7;
const MAX_WIDTH: f32 = 352.0;
const MIN_WIDTH: f32 = 160.0;
// The description beside a compact list's selected row.
const DESCRIPTION_WIDTH: f32 = 320.0;
const DESCRIPTION_LINES: usize = 10;
const DESCRIPTION_GAP: f32 = 8.0;
// Below this the hints stack instead of sharing a row.
const STACKED_HINTS_BELOW: f32 = 300.0;

/// One completion: a label over a description, after an optional icon.
#[derive(Clone)]
pub struct SuggestionItem {
    id: String,
    label: String,
    description: String,
    icon: Option<Icon>,
    leading: Option<Element>,
    ink: Option<Color>,
    detail: String,
    section: Option<String>,
    enabled: bool,
    matched: Vec<Range<usize>>,
}

impl SuggestionItem {
    /// A completion reporting `id` when clicked.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: String::new(),
            icon: None,
            leading: None,
            ink: None,
            detail: String::new(),
            section: None,
            enabled: true,
            matched: Vec::new(),
        }
    }

    /// An icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The quiet second line.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }

    /// An element of the host's in the icon's place, such as a person's
    /// avatar 24 across, centered at its own size. It takes the place of
    /// `icon`.
    pub fn leading(mut self, element: Element) -> Self {
        self.leading = Some(element);
        self
    }

    /// The label's color, in place of the palette's text, such as a
    /// person's role color. A disabled item stays muted.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// A quiet text at the end of the label's line, such as a handle or what
    /// the completion inserts. The label is cut short before it.
    pub fn detail(mut self, text: impl Into<String>) -> Self {
        self.detail = text.into();
        self
    }

    /// Start a section named `label` at this item. The heading takes a row
    /// of its own above it.
    pub fn section(mut self, label: impl Into<String>) -> Self {
        self.section = Some(label.into());
        self
    }

    /// A disabled suggestion keeps its row with icon and label in `muted`.
    /// It is passive: a click on it reports nothing, and it does not take
    /// the highlight, so the host's `selected_index` should step over it.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Byte ranges of the label drawn in bold, such as what the query matched.
    /// A range out of bounds or not on a character boundary is ignored.
    pub fn matched(mut self, ranges: Vec<std::ops::Range<usize>>) -> Self {
        self.matched = ranges;
        self
    }
}

// The label as runs, the matched ranges in bold; `None` when no range is usable.
fn label_runs(
    label: &str,
    ranges: &[Range<usize>],
    color: Color,
    size: f32,
) -> Option<Vec<TextSpan>> {
    let mut ranges: Vec<_> = ranges
        .iter()
        .filter(|range| {
            range.start < range.end
                && range.end <= label.len()
                && label.is_char_boundary(range.start)
                && label.is_char_boundary(range.end)
        })
        .cloned()
        .collect();
    if ranges.is_empty() {
        return None;
    }
    ranges.sort_by_key(|range| range.start);
    let mut runs = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut cursor = 0;
    for range in ranges {
        // A range overlapping the one before it is skipped.
        if range.start < cursor {
            continue;
        }
        if range.start > cursor {
            runs.push(TextSpan::new(&label[cursor..range.start], color, size, FontFamily::Sans));
        }
        runs.push(TextSpan::new(&label[range.clone()], color, size, FontFamily::SansBold));
        cursor = range.end;
    }
    if cursor < label.len() {
        runs.push(TextSpan::new(&label[cursor..], color, size, FontFamily::Sans));
    }
    Some(runs)
}

// The selected row's description, in a tooltip's colors beside the row. It is a
// popover only so the row places it: it takes no pointer or focus.
fn beside(row: Element, text: &str, palette: ComponentPalette, dismiss_id: String) -> Element {
    let colors = palette.tooltip_style();
    Popover::new(
        row,
        Element::formatted_text(vec![TextSpan::new(text, colors.text, 12.0, FontFamily::Sans)])
            .max_lines(DESCRIPTION_LINES)
            .padding(8.0)
            .size(Some(DESCRIPTION_WIDTH - 12.0), None),
        true,
        dismiss_id,
        ComponentPalette {
            raised: colors.background,
            border: colors.border,
            shadow: colors.shadow,
            ..palette
        },
    )
    .width(DESCRIPTION_WIDTH)
    .placement(PopoverPlacement::RightStart)
    .render()
    .popover_gap(DESCRIPTION_GAP)
    .popover_non_modal()
}

#[derive(Clone, Copy)]
enum Slot {
    Section(usize),
    Item(usize),
}

/// A non-modal list anchored to an input. Focus stays in the input: the host
/// moves `selected_index` on the arrow keys, accepts on Enter and closes on
/// Escape, and the list only shows the state and reports clicks. Rows are
/// built as they scroll into view, so a long result costs nothing.
pub struct SuggestionList {
    anchor: Element,
    id: String,
    items: Vec<SuggestionItem>,
    selected_index: usize,
    open: bool,
    dismiss_id: String,
    palette: ComponentPalette,
    title: Option<String>,
    empty: String,
    hints: Vec<Element>,
    note: Option<String>,
    available: f32,
    max_width: f32,
    compact: bool,
    placement: PopoverPlacement,
    animation_ms: (u16, u16),
    sizing: SelectorSizing,
}

impl SuggestionList {
    /// `id` names the scrolling options; `dismiss_id` is reported when the
    /// pointer is pressed outside the list.
    pub fn new(
        anchor: Element,
        id: impl Into<String>,
        items: Vec<SuggestionItem>,
        selected_index: usize,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        let motion = crate::tokens().motion;
        Self {
            anchor,
            id: id.into(),
            items,
            selected_index,
            open,
            dismiss_id: dismiss_id.into(),
            palette,
            title: None,
            empty: String::new(),
            hints: Vec::new(),
            note: None,
            available: f32::INFINITY,
            max_width: MAX_WIDTH,
            compact: false,
            placement: PopoverPlacement::TopStart,
            animation_ms: (motion.enter_ms, motion.exit_ms),
            sizing: SelectorSizing::CATALOG,
        }
    }

    /// A muted line above the options saying what is being completed.
    pub fn title(mut self, text: impl Into<String>) -> Self {
        self.title = Some(text.into());
        self
    }

    /// What to say in place of the options when there are none: nothing
    /// matches, the search is running, or it failed.
    pub fn empty(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }

    /// Key hints under the options, such as `Kbd::caption` elements. They
    /// share a row, and stack when the list is narrow.
    pub fn hints(mut self, hints: Vec<Element>) -> Self {
        self.hints = hints;
        self
    }

    /// A muted last line, such as a remark that the results were cut short.
    pub fn note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    /// The room the host can offer. The list is as wide as that allows,
    /// between 160 and `max_width` (352 unless set). Values that are not
    /// finite are ignored.
    pub fn width(mut self, available: f32) -> Self {
        if available.is_finite() {
            self.available = available;
        }
        self
    }

    /// The widest the list may be, in place of 352. `width` still bounds it,
    /// and 160 remains the least. Values that are not finite are ignored.
    pub fn max_width(mut self, max_width: f32) -> Self {
        if max_width.is_finite() {
            self.max_width = max_width;
        }
        self
    }

    /// One-line rows, 30 high, with the icon, the label and the detail at its
    /// end; the description is not drawn. The selected item's description
    /// shows in a tooltip-styled box beside the list, top-aligned with its
    /// row, up to ten lines at 320 wide.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Where the list opens from its anchor (above its start unless set).
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The text sizes and icon slot of the rows, in place of the catalog's,
    /// so the list reads at the size of the editor it completes.
    pub fn sizing(mut self, sizing: SelectorSizing) -> Self {
        self.sizing = sizing;
        self
    }

    /// How long the list takes to appear and to leave, in milliseconds,
    /// for a host that wants it quicker than the motion tokens.
    pub fn animation_ms(mut self, open: u16, close: u16) -> Self {
        self.animation_ms = (open, close);
        self
    }
}

impl Component for SuggestionList {
    fn render(self) -> Element {
        if !self.open {
            return self.anchor;
        }
        let sizing = self.sizing;
        let palette = self.palette;
        let compact = self.compact;
        let row_height = if compact { COMPACT_ROW_HEIGHT } else { ROW_HEIGHT };
        // Not `clamp`: a width that is not a number must still give a width.
        #[allow(clippy::manual_clamp)]
        let width = self.available.min(self.max_width).max(MIN_WIDTH);
        let inner_width = width - sizing.padding * 2.0;
        let mut contents = Vec::with_capacity(5);
        if let Some(title) = self.title {
            contents.push(
                Element::text(title)
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.label_size)
                    .foreground(palette.muted)
                    .padding(sizing.padding)
                    .size(Some(inner_width), Some(sizing.header_height)),
            );
        }
        let appearance = ScrollbarAppearance::reserved()
            .rail_width(sizing.scrollbar_gutter)
            .tint(palette.muted);
        // Reserve the rail before rows are built so scrollbar targets cannot
        // overlap their full-width click surface, even as overflow changes.
        let row_width = (inner_width - sizing.scrollbar_gutter - 2.0).max(0.0);
        let description_id = format!("{}:description", self.id);
        if self.items.is_empty() {
            contents.push(
                ScrollArea::new(
                    self.id,
                    Element::formatted_text(vec![TextSpan::new(
                        self.empty,
                        palette.muted,
                        sizing.label_size,
                        FontFamily::Sans,
                    )])
                    .padding(8.0)
                    .size(Some(row_width), None),
                )
                .appearance(appearance)
                .render()
                .min_size(0.0, row_height)
                .size(Some(inner_width), Some(row_height))
                .flex(1.0),
            );
        } else {
            let mut slots = Vec::with_capacity(self.items.len() + 2);
            for (index, item) in self.items.iter().enumerate() {
                if item.section.is_some() {
                    slots.push(Slot::Section(index));
                }
                slots.push(Slot::Item(index));
            }
            let row_count = slots.len();
            let items = Arc::new(self.items);
            let selected = self.selected_index;
            contents.push(
                ScrollArea::uniform_list(self.id, row_count, row_height, move |slot| {
                    match slots[slot] {
                        Slot::Section(index) => Element::column(vec![
                            Separator::horizontal(palette).render(),
                            Element::text(items[index].section.clone().unwrap_or_default())
                                .font_family(FontFamily::Sans)
                                .font_size(sizing.detail_size)
                                .foreground(palette.muted)
                                .padding(sizing.padding)
                                .flex(1.0),
                        ])
                        .size(Some(row_width), Some(row_height)),
                        Slot::Item(index) => {
                            let item = &items[index];
                            let ink = if item.enabled { palette.text } else { palette.muted };
                            let label_ink = if item.enabled { item.ink.unwrap_or(ink) } else { ink };
                            let mut row = Vec::with_capacity(2);
                            if let Some(leading) = &item.leading {
                                row.push(leading.clone().centered_y());
                            } else if let Some(icon) = item.icon {
                                row.push(
                                    Element::icon(icon)
                                        .foreground(ink)
                                        .size(Some(sizing.icon_size), Some(30.0)),
                                );
                            }
                            let runs = label_runs(&item.label, &item.matched, label_ink, sizing.label_size);
                            let label = match runs {
                                Some(runs) => Element::formatted_text(runs).max_lines(1),
                                None => Element::truncated_text(&item.label)
                                    .font_family(FontFamily::Sans)
                                    .font_size(sizing.label_size)
                                    .foreground(label_ink),
                            }
                            .size(None, Some(16.0));
                            let label = if item.detail.is_empty() {
                                label
                            } else {
                                // The detail keeps its width up to half the row, cut past it; the
                                // label is cut before it, so a long detail never hides the label.
                                Element::row(vec![
                                    label.flex(1.0),
                                    Element::truncated_text(&item.detail)
                                        .font_family(FontFamily::Sans)
                                        .font_size(sizing.detail_size)
                                        .foreground(palette.muted)
                                        .max_width(row_width / 2.0)
                                        .centered_y(),
                                ])
                                .gap(sizing.gap)
                                .size(None, Some(16.0))
                            };
                            row.push(if compact || item.description.is_empty() {
                                // One line stands in the middle of the row.
                                label.centered_y().flex(1.0)
                            } else {
                                Element::column(vec![
                                    label,
                                    Element::truncated_text(&item.description)
                                        .font_family(FontFamily::Sans)
                                        .font_size(sizing.detail_size)
                                        .foreground(palette.muted)
                                        .size(None, Some(14.0)),
                                ])
                                .flex(1.0)
                            });
                            let row = Element::row(row).gap(sizing.gap).padding(sizing.padding);
                            let mut access = Access::new(Role::ListItem).name(&item.label);
                            let said: Vec<&str> = [item.description.as_str(), item.detail.as_str()]
                                .into_iter()
                                .filter(|text| !text.is_empty())
                                .collect();
                            if !said.is_empty() {
                                access = access.description(said.join(", "));
                            }
                            let element = if !item.enabled {
                                Element::panel(row)
                                    .size(Some(row_width), Some(row_height))
                                    .accessible(access.id(&item.id).disabled(true))
                            } else {
                                palette
                                    .interactive_surface(
                                        Element::clickable(&item.id, row)
                                            .accessible(access.selected(index == selected)),
                                    )
                                    // The host's selection, not the pointer, owns
                                    // the highlight: it follows the arrow keys.
                                    .background(if index == selected {
                                        palette.hover
                                    } else {
                                        palette.surface
                                    })
                                    .transition_ms(0)
                                    .size(Some(row_width), Some(row_height))
                            };
                            if compact && index == selected && !item.description.is_empty() {
                                beside(element, &item.description, palette, description_id.clone())
                            } else {
                                element
                            }
                        }
                    }
                })
                .appearance(appearance)
                .render()
                .min_size(0.0, row_height)
                .size(
                    Some(inner_width),
                    Some(row_count.min(MAX_VISIBLE) as f32 * row_height),
                )
                .flex(1.0),
            );
        }
        if !self.hints.is_empty() {
            contents.push(Separator::horizontal(palette).render());
            contents.push(
                if width < STACKED_HINTS_BELOW {
                    Element::column(self.hints).gap(4.0)
                } else {
                    Element::row(self.hints).gap(10.0)
                }
                .padding(4.0)
                .size(Some(inner_width), None),
            );
        }
        if let Some(note) = self.note {
            contents.push(
                Element::text(note)
                    .font_family(FontFamily::Sans)
                    .font_size(sizing.detail_size)
                    .foreground(palette.muted)
                    .padding(4.0),
            );
        }
        Popover::new(
            self.anchor,
            Element::column(contents).gap(2.0),
            true,
            self.dismiss_id,
            // One surface: the rows rest on the fill the popover paints.
            ComponentPalette {
                raised: palette.surface,
                ..palette
            },
        )
        .width(width)
        .placement(self.placement)
        .animation_ms(self.animation_ms.0)
        .close_animation_ms(self.animation_ms.1)
        .render()
        .popover_non_modal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Kbd, Point, Rect, Size, UiAction, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn commands(count: usize) -> Vec<SuggestionItem> {
        (0..count)
            .map(|index| {
                let item = SuggestionItem::new(format!("pick:{index}"), format!("/command-{index}"))
                    .icon(Icon::Terminal)
                    .description(format!("Runs command {index}"));
                if index == 0 {
                    item.section("Commands")
                } else {
                    item
                }
            })
            .collect()
    }

    fn list(items: Vec<SuggestionItem>, selected: usize, available: f32) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::spacer().flex(1.0),
            SuggestionList::new(
                Element::text_input("editor", "/", "").size(Some(400.0), Some(32.0)),
                "options",
                items,
                selected,
                true,
                "dismiss",
                palette(),
            )
            .title("Commands and skills")
            .empty("Nothing matches.")
            .hints(vec![Kbd::new("Esc", palette()).caption("Close").render()])
            .note("Results limited")
            .width(available)
            .render(),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn row(frame: &Frame, id: &str) -> Option<Rect> {
        frame
            .regions
            .iter()
            .find(|region| region.id == id && region.kind != HitKind::Scrollbar)
            .map(|region| region.rect)
    }

    fn has_text(frame: &Frame, label: &str) -> bool {
        frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == label))
    }

    #[test]
    fn a_suggestion_tells_the_platform_its_label_and_which_one_is_highlighted() {
        let mut items = commands(3);
        items[2] = items[2].clone().disabled();
        let frame = list(items, 1, 600.0).layout(Size::new(600.0, 400.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));

        let (first, chosen, off) = (node("pick:0"), node("pick:1"), node("pick:2"));
        assert_eq!((first.role, first.name.as_str(), first.selected), (Role::ListItem, "/command-0", Some(false)));
        assert_eq!(first.description.as_deref(), Some("Runs command 0"));
        assert_eq!((chosen.name.as_str(), chosen.selected), ("/command-1", Some(true)));
        assert!(chosen.actionable && !chosen.disabled);
        assert_eq!((off.role, off.name.as_str(), off.selected), (Role::ListItem, "/command-2", None));
        assert!(off.disabled && !off.actionable);
    }

    #[test]
    fn a_suggestion_takes_a_leading_element_its_own_ink_and_a_detail_at_its_end() {
        const ROLE: Color = Color::rgb(46, 204, 113);
        let face = Element::panel(Element::spacer()).background(Color::rgb(1, 2, 3)).radius(12.0).size(Some(24.0), Some(24.0));
        let items = vec![
            SuggestionItem::new("pick:ada", "Ada Lovelace")
                .icon(Icon::At)
                .leading(face)
                .ink(ROLE)
                .detail("ada"),
            SuggestionItem::new("pick:off", "Bruno").ink(ROLE).detail("bruno").disabled(),
        ];
        let frame = list(items, 0, 600.0).layout(Size::new(600.0, 400.0));
        let ada = row(&frame, "pick:ada").unwrap();
        // The host's element in the icon's place, centered on the row.
        let picture = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == Color::rgb(1, 2, 3) => Some(*rect),
            _ => None,
        });
        let picture = picture.expect("the leading element");
        assert_eq!((picture.width, picture.height), (24.0, 24.0));
        assert!((picture.y + 12.0 - (ada.y + ada.height / 2.0)).abs() < 0.5, "{picture:?} in {ada:?}");
        assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Icon { icon: Icon::At, .. })));
        // The label in its ink, on the middle line without a description; the detail at the end, muted.
        let text = |label: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, color, position, .. } if text == label => Some((*color, *position)),
                _ => None,
            })
        };
        let (ink, at) = text("Ada Lovelace").unwrap();
        assert_eq!(ink, ROLE);
        assert!(at.y > ada.y + 8.0, "one line stands in the middle: {at:?} in {ada:?}");
        let (muted, end) = text("ada").unwrap();
        assert_eq!(muted, palette().muted);
        assert!(end.x > at.x + 100.0 && end.x < ada.x + ada.width);
        // A disabled one stays muted, whatever its ink.
        assert_eq!(text("Bruno").unwrap().0, palette().muted);
        // The platform hears the detail.
        let said = frame.access.iter().find(|node| node.id == "pick:ada").unwrap();
        assert_eq!(said.description.as_deref(), Some("ada"));
    }

    #[test]
    fn only_the_rows_in_view_are_built_and_seven_slots_bound_the_list() {
        let mut tree = list(commands(200), 0, 600.0);
        let frame = tree.layout(Size::new(600.0, 600.0));
        let first = row(&frame, "pick:0").expect("the first item is in view");
        assert_eq!(first.height, ROW_HEIGHT);
        // 352 wide, minus the popover inset and the reserved rail.
        assert_eq!(first.width, 352.0 - 12.0 - 14.0);
        assert!(has_text(&frame, "Commands"), "the section heading has a row");
        assert!(row(&frame, "pick:5").is_some());
        assert!(row(&frame, "pick:150").is_none(), "rows far below are not built");
        let built = frame
            .regions
            .iter()
            .filter(|region| region.id.starts_with("pick:"))
            .count();
        assert!(built <= MAX_VISIBLE + 2, "{built} rows built for seven slots");
    }

    #[test]
    fn the_selected_row_carries_the_hover_fill_and_a_click_reports_the_item() {
        let mut tree = list(commands(4), 2, 600.0);
        let size = Size::new(600.0, 600.0);
        let frame = tree.layout(size);
        let selected = row(&frame, "pick:2").unwrap();
        let fills: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == palette().hover => {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        assert_eq!(fills, vec![selected]);
        let point = Point::new(selected.x + 30.0, selected.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "pick:2".into() }]
        );
        assert!(!tree.focus("pick:2") || tree.focus("editor"), "the input stays reachable");
    }

    #[test]
    fn a_disabled_suggestion_is_muted_keeps_its_row_and_cannot_be_chosen() {
        let size = Size::new(600.0, 600.0);
        let items = |enabled: bool| {
            let mut items = commands(4);
            let item = items.remove(2).enabled(enabled);
            items.insert(2, item);
            items
        };
        let drawn = |frame: &Frame| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, position, color, .. } => Some((text.clone(), *position, *color)),
                    DrawCommand::Icon { rect, tint, .. } => {
                        Some((String::new(), Point::new(rect.x, rect.y), *tint))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Selected by the host all the same: it is not highlighted.
        let mut tree = list(items(false), 2, 600.0);
        let frame = tree.layout(size);
        assert!(frame.regions.iter().all(|region| region.id != "pick:2"), "{:?}", frame.regions);
        assert!(!tree.focus("pick:2"));
        assert!(!frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == palette().hover)));
        let enabled = list(items(true), 2, 600.0).layout(size);
        let place = row(&enabled, "pick:2").unwrap();
        let point = Point::new(place.x + 30.0, place.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        // Icon and label are muted; everything stands where it did.
        let (shown, was) = (drawn(&frame), drawn(&enabled));
        assert_eq!(shown.len(), was.len());
        for (shown, was) in shown.iter().zip(&was) {
            assert_eq!((&shown.0, shown.1), (&was.0, was.1));
            let inside = (place.y..place.y + place.height).contains(&shown.1.y);
            assert_eq!(shown.2, if inside { palette().muted } else { was.2 }, "{shown:?}");
        }
        assert_eq!(row(&frame, "pick:3"), row(&enabled, "pick:3"));
    }

    #[test]
    fn an_empty_list_says_why_and_keeps_title_hints_and_note() {
        let mut tree = list(Vec::new(), 0, 600.0);
        let frame = tree.layout(Size::new(600.0, 600.0));
        for label in ["Commands and skills", "Nothing matches.", "Close", "Results limited"] {
            assert!(has_text(&frame, label), "missing {label}");
        }
    }

    #[test]
    fn the_width_follows_the_room_between_its_bounds_and_ignores_bad_input() {
        for (available, expected) in [
            (240.0, 240.0),
            (900.0, MAX_WIDTH),
            (f32::NAN, MAX_WIDTH),
            (f32::INFINITY, MAX_WIDTH),
        ] {
            let mut tree = list(commands(3), 0, available);
            let frame = tree.layout(Size::new(600.0, 600.0));
            let item = row(&frame, "pick:1").unwrap();
            // Three rows do not scroll, so no rail takes room from them.
            assert_eq!(item.width, expected - 12.0, "available {available}");
        }
        let width = |available: f32| {
            let mut tree = list(commands(3), 0, available);
            row(&tree.layout(Size::new(600.0, 600.0)), "pick:1").unwrap().width
        };
        assert_eq!(width(40.0), width(MIN_WIDTH), "no narrower than the minimum");
    }

    #[test]
    fn a_closed_list_is_only_its_anchor() {
        let mut tree = UiTree::new(
            SuggestionList::new(
                Element::text_input("editor", "", ""),
                "options",
                commands(3),
                0,
                false,
                "dismiss",
                palette(),
            )
            .render(),
        );
        let frame = tree.layout(Size::new(400.0, 200.0));
        assert!(row(&frame, "pick:0").is_none());
        assert!(row(&frame, "editor").is_some());
    }

    fn shown(
        items: Vec<SuggestionItem>,
        selected: usize,
        configure: impl FnOnce(SuggestionList) -> SuggestionList,
    ) -> UiTree {
        let list = SuggestionList::new(
            Element::text_input("editor", "/", "").size(Some(400.0), Some(32.0)),
            "options",
            items,
            selected,
            true,
            "dismiss",
            palette(),
        );
        let mut tree = UiTree::new(Element::column(vec![
            Element::spacer().flex(1.0),
            configure(list).render(),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn drawn(frame: &Frame, label: &str) -> Option<(Point, FontFamily)> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text {
                text,
                position,
                font_family,
                ..
            } if text == label => Some((*position, *font_family)),
            _ => None,
        })
    }

    // Fills in the tooltip's background: the description box, and nothing else.
    fn boxes(frame: &Frame) -> Vec<Rect> {
        let background = palette().tooltip_style().background;
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == background => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn compact_rows_are_one_line_with_the_detail_at_the_end_and_no_description() {
        let items = vec![
            SuggestionItem::new("pick:0", "/plan")
                .icon(Icon::Plan)
                .detail("plan")
                .description("Describe the steps"),
            SuggestionItem::new("pick:1", "/review").description("Review the diff"),
        ];
        let frame = shown(items, 1, |list| list.compact(true).width(600.0))
            .layout(Size::new(900.0, 600.0));
        let first = row(&frame, "pick:0").unwrap();
        assert_eq!(first.height, COMPACT_ROW_HEIGHT);
        assert!(!has_text(&frame, "Describe the steps"), "no description line in a compact row");
        let (label, _) = drawn(&frame, "/plan").unwrap();
        let (detail, _) = drawn(&frame, "plan").unwrap();
        assert!(
            (label.y - detail.y).abs() < 2.0 && detail.x > first.x + first.width / 2.0,
            "{label:?} {detail:?} in {first:?}"
        );
        assert!(label.y >= first.y && label.y < first.y + first.height);
        // The unselected row keeps its height in a list of one-line rows.
        let second = row(&frame, "pick:1").unwrap();
        assert_eq!(second.height, COMPACT_ROW_HEIGHT);
    }

    #[test]
    fn a_long_detail_is_cut_and_never_hides_the_label() {
        let items = vec![SuggestionItem::new("pick:0", "effort").detail("[low|medium|high|xhigh|max|ultra] [--fix] [--all] [--verbose] [--dry-run]")];
        let frame = shown(items, 0, |list| list.compact(true).width(320.0)).layout(Size::new(900.0, 600.0));
        let row = row(&frame, "pick:0").unwrap();
        let (label, _) = drawn(&frame, "effort").expect("the label is drawn");
        assert!(label.x >= row.x && label.x < row.x + row.width / 2.0, "{label:?} in {row:?}");
    }

    #[test]
    fn rows_take_the_hosts_text_sizes() {
        let size = |configure: fn(SuggestionList) -> SuggestionList| {
            let frame = shown(commands(2), 0, |list| configure(list.compact(true)))
                .layout(Size::new(900.0, 600.0));
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { text, font_size, .. } if text.starts_with("/command") => Some(*font_size),
                _ => None,
            })
        };
        let larger = SelectorSizing { label_size: 15.0, ..SelectorSizing::CATALOG };
        assert_eq!(size(|list| list), Some(SelectorSizing::CATALOG.label_size));
        assert_eq!(size(|list| list.sizing(SelectorSizing { label_size: 15.0, ..SelectorSizing::CATALOG })), Some(larger.label_size));
    }

    #[test]
    fn max_width_raises_the_cap_and_width_still_bounds_it() {
        let width = |configure: fn(SuggestionList) -> SuggestionList| {
            let mut tree = shown(commands(3), 0, |list| configure(list.width(800.0)));
            row(&tree.layout(Size::new(900.0, 600.0)), "pick:1").unwrap().width
        };
        assert_eq!(width(|list| list), 352.0 - 12.0);
        assert_eq!(width(|list| list.max_width(512.0)), 512.0 - 12.0);
        assert_eq!(width(|list| list.max_width(f32::NAN)), 352.0 - 12.0, "not finite is ignored");
        let mut tree = shown(commands(3), 0, |list| list.max_width(512.0).width(400.0));
        assert_eq!(row(&tree.layout(Size::new(900.0, 600.0)), "pick:1").unwrap().width, 400.0 - 12.0);
    }

    #[test]
    fn the_selected_description_sits_beside_its_row_and_moves_with_the_selection() {
        let beside = |selected: usize| {
            let mut tree = shown(commands(3), selected, |list| list.compact(true).width(600.0));
            let frame = tree.layout(Size::new(900.0, 600.0));
            let picked = row(&frame, &format!("pick:{selected}")).unwrap();
            let surfaces = boxes(&frame);
            assert_eq!(surfaces.len(), 1, "{surfaces:?}");
            (picked, surfaces[0], frame)
        };
        let (picked, surface, frame) = beside(1);
        assert_eq!(surface.width, DESCRIPTION_WIDTH);
        assert!(
            (surface.x - (picked.x + picked.width + DESCRIPTION_GAP)).abs() < 0.5,
            "{surface:?} beside {picked:?}"
        );
        assert!((surface.y - picked.y).abs() < 0.5, "{surface:?} top of {picked:?}");
        assert!(drawn(&frame, "Runs command 1").is_some());
        assert!(!has_text(&frame, "Runs command 0"), "other rows keep no description");
        let (picked, surface, _) = beside(2);
        assert!((surface.y - picked.y).abs() < 0.5, "{surface:?} follows the selection to {picked:?}");
    }

    #[test]
    fn no_description_box_unless_the_list_is_compact_and_the_selection_has_one() {
        // Two lines: the description is in its row, and there is no box.
        let frame = shown(commands(3), 1, |list| list.width(600.0)).layout(Size::new(900.0, 600.0));
        assert!(boxes(&frame).is_empty());
        assert!(drawn(&frame, "Runs command 1").is_some());
        let items = || {
            vec![
                SuggestionItem::new("pick:0", "/a"),
                SuggestionItem::new("pick:1", "/b").description("Runs b"),
            ]
        };
        let frame = shown(items(), 0, |list| list.compact(true)).layout(Size::new(900.0, 600.0));
        assert!(boxes(&frame).is_empty(), "the selected item has no description");
        let frame = shown(items(), 1, |list| list.compact(true)).layout(Size::new(900.0, 600.0));
        assert_eq!(boxes(&frame).len(), 1);
    }

    #[test]
    fn matched_ranges_are_drawn_in_bold_and_a_bad_range_is_ignored() {
        let items = vec![
            SuggestionItem::new("pick:0", "/command-0").matched(vec![1..4]),
            SuggestionItem::new("pick:1", "éa").matched(vec![1..2, 0..99]),
        ];
        let mut tree = shown(items, 0, |list| list.width(600.0));
        let frame = tree.layout(Size::new(900.0, 600.0));
        // The "/" of the input is drawn too, so only the runs of the label are looked up.
        assert_eq!(drawn(&frame, "com").unwrap().1, FontFamily::SansBold);
        assert_eq!(drawn(&frame, "mand-0").unwrap().1, FontFamily::Sans);
        // The bad ranges leave the label whole and regular.
        assert_eq!(drawn(&frame, "éa").unwrap().1, FontFamily::Sans);
        let bold = frame
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::Text { font_family: FontFamily::SansBold, .. }))
            .count();
        assert_eq!(bold, 1, "only the usable range is bold");
    }
}
