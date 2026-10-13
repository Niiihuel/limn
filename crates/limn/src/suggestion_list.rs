//! Completions floating over a text input while the user keeps typing.

use std::sync::Arc;

use crate::{
    Component, ComponentPalette, Element, FontFamily, Icon, Popover, PopoverPlacement,
    ScrollArea, ScrollbarAppearance, SelectorSizing, Separator, TextSpan,
};

// Two lines and their padding; taller than a catalog row because the list
// floats alone over an editor.
const ROW_HEIGHT: f32 = 42.0;
const MAX_VISIBLE: usize = 7;
const MAX_WIDTH: f32 = 352.0;
const MIN_WIDTH: f32 = 160.0;
// Below this the hints stack instead of sharing a row.
const STACKED_HINTS_BELOW: f32 = 300.0;

/// One completion: a label over a description, after an optional icon.
#[derive(Clone)]
pub struct SuggestionItem {
    id: String,
    label: String,
    description: String,
    icon: Option<Icon>,
    section: Option<String>,
}

impl SuggestionItem {
    /// A completion reporting `id` when clicked.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: String::new(),
            icon: None,
            section: None,
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

    /// Start a section named `label` at this item. The heading takes a row
    /// of its own above it.
    pub fn section(mut self, label: impl Into<String>) -> Self {
        self.section = Some(label.into());
        self
    }
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
    placement: PopoverPlacement,
    animation_ms: (u16, u16),
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
            available: MAX_WIDTH,
            placement: PopoverPlacement::TopStart,
            animation_ms: (motion.enter_ms, motion.exit_ms),
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
    /// between 160 and 352. Values that are not finite are ignored.
    pub fn width(mut self, available: f32) -> Self {
        if available.is_finite() {
            self.available = available;
        }
        self
    }

    /// Where the list opens from its anchor (above its start unless set).
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
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
        let sizing = SelectorSizing::CATALOG;
        let palette = self.palette;
        // Not `clamp`: a width that is not a number must still give a width.
        #[allow(clippy::manual_clamp)]
        let width = self.available.min(MAX_WIDTH).max(MIN_WIDTH);
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
                .min_size(0.0, ROW_HEIGHT)
                .size(Some(inner_width), Some(ROW_HEIGHT))
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
                ScrollArea::uniform_list(self.id, row_count, ROW_HEIGHT, move |slot| {
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
                        .size(Some(row_width), Some(ROW_HEIGHT)),
                        Slot::Item(index) => {
                            let item = &items[index];
                            let mut row = Vec::with_capacity(2);
                            if let Some(icon) = item.icon {
                                row.push(
                                    Element::icon(icon)
                                        .foreground(palette.text)
                                        .size(Some(sizing.icon_size), Some(30.0)),
                                );
                            }
                            row.push(
                                Element::column(vec![
                                    Element::truncated_text(&item.label)
                                        .font_family(FontFamily::Sans)
                                        .font_size(sizing.label_size)
                                        .foreground(palette.text)
                                        .size(None, Some(16.0)),
                                    Element::truncated_text(&item.description)
                                        .font_family(FontFamily::Sans)
                                        .font_size(sizing.detail_size)
                                        .foreground(palette.muted)
                                        .size(None, Some(14.0)),
                                ])
                                .flex(1.0),
                            );
                            palette
                                .interactive_surface(Element::clickable(
                                    &item.id,
                                    Element::row(row).gap(sizing.gap).padding(sizing.padding),
                                ))
                                // The host's selection, not the pointer, owns
                                // the highlight: it follows the arrow keys.
                                .background(if index == selected {
                                    palette.hover
                                } else {
                                    palette.surface
                                })
                                .transition_ms(0)
                                .size(Some(row_width), Some(ROW_HEIGHT))
                        }
                    }
                })
                .appearance(appearance)
                .render()
                .min_size(0.0, ROW_HEIGHT)
                .size(
                    Some(inner_width),
                    Some(row_count.min(MAX_VISIBLE) as f32 * ROW_HEIGHT),
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
}
