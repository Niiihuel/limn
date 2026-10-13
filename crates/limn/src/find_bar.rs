//! Find and replace over a document: the companion of [`crate::CodeEditor`].
//! The host searches; this bar shows the query, the options and the count.

use crate::{
    ActiveHover, Button, ButtonSize, ButtonVariant, Color, Component, ComponentPalette,
    ControlSize, Element, FontFamily, Icon, IconButton, Point, PopoverPlacement, TextField,
    Tooltip, TooltipPlacement, TooltipSpec,
};

const MAX_WIDTH: f32 = 520.0;
// Narrower than this the options and the count take a row of their own.
const INLINE_OPTIONS_WIDTH: f32 = 480.0;
// Narrower than this the options row gives up its leading slot.
const LEADING_SLOT_WIDTH: f32 = 280.0;
const PADDING: f32 = 8.0;
const ROW: f32 = 28.0;
const ROW_GAP: f32 = 4.0;
const ERROR_ROW: f32 = 20.0;
// The slot of the replace toggle, repeated so both fields line up.
const SLOT: f32 = 24.0;

/// Which search options are on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    /// Upper and lower case differ.
    pub case: bool,
    /// Only whole words match.
    pub word: bool,
    /// The query is a regular expression.
    pub regex: bool,
    /// Only the selection is searched.
    pub in_selection: bool,
}

/// The action and input IDs of a [`FindBar`]. `dismiss` is the close button
/// and the dismiss ID of the popover [`FindBar::attach`] makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindIds {
    /// Input ID of the query field.
    pub query: String,
    /// Input ID of the replace field.
    pub replacement: String,
    /// Toggles [`FindOptions::case`].
    pub case: String,
    /// Toggles [`FindOptions::word`].
    pub word: String,
    /// Toggles [`FindOptions::regex`].
    pub regex: String,
    /// Toggles [`FindOptions::in_selection`].
    pub in_selection: String,
    /// Shows or hides the replace row.
    pub toggle_replace: String,
    /// Steps to the match before the current one.
    pub previous: String,
    /// Steps to the match after the current one.
    pub next: String,
    /// Closes the bar.
    pub dismiss: String,
    /// Replaces the current match.
    pub replace_one: String,
    /// Replaces every match.
    pub replace_all: String,
    /// Hover target of the count's tooltip.
    pub count: String,
    /// Hover target of the error's tooltip.
    pub error: String,
}

impl FindIds {
    /// Every ID derived from the bar's own, as `{id}:query` and so on.
    pub fn new(id: &str) -> Self {
        let of = |part: &str| format!("{id}:{part}");
        Self {
            query: of("query"),
            replacement: of("replacement"),
            case: of("case"),
            word: of("word"),
            regex: of("regex"),
            in_selection: of("in_selection"),
            toggle_replace: of("toggle_replace"),
            previous: of("previous"),
            next: of("next"),
            dismiss: of("dismiss"),
            replace_one: of("replace_one"),
            replace_all: of("replace_all"),
            count: of("count"),
            error: of("error"),
        }
    }
}

/// Every string a [`FindBar`] shows: the placeholders, the tooltips and the
/// shortcuts named in them. The default is English.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FindLabels {
    /// Placeholder of the query field.
    pub query: String,
    /// Placeholder of the replace field.
    pub replacement: String,
    /// Tooltip of the case option.
    pub case: String,
    /// Tooltip of the whole word option.
    pub word: String,
    /// Tooltip of the regular expression option.
    pub regex: String,
    /// Tooltip of the search in selection option.
    pub in_selection: String,
    /// Tooltip of the button that shows the replace row.
    pub toggle_replace: String,
    /// The shortcut that tooltip names.
    pub toggle_replace_shortcut: String,
    /// Tooltip of the previous match button.
    pub previous: String,
    /// The shortcut that tooltip names.
    pub previous_shortcut: String,
    /// Tooltip of the next match button.
    pub next: String,
    /// The shortcut that tooltip names.
    pub next_shortcut: String,
    /// Tooltip of the close button.
    pub dismiss: String,
    /// The shortcut that tooltip names.
    pub dismiss_shortcut: String,
    /// Tooltip of the replace button.
    pub replace_one: String,
    /// The shortcut that tooltip names.
    pub replace_one_shortcut: String,
    /// Tooltip of the replace all button.
    pub replace_all: String,
    /// What the count means, as its tooltip.
    pub count: String,
}

impl Default for FindLabels {
    fn default() -> Self {
        Self {
            query: "Find".into(),
            replacement: "Replace".into(),
            case: "Match case".into(),
            word: "Match whole word".into(),
            regex: "Use regular expression".into(),
            in_selection: "Find in selection".into(),
            toggle_replace: "Toggle replace".into(),
            toggle_replace_shortcut: "Ctrl+H".into(),
            previous: "Previous match".into(),
            previous_shortcut: "Shift+Enter".into(),
            next: "Next match".into(),
            next_shortcut: "Enter".into(),
            dismiss: "Close".into(),
            dismiss_shortcut: "Esc".into(),
            replace_one: "Replace".into(),
            replace_one_shortcut: "Enter".into(),
            replace_all: "Replace all".into(),
            count: "Current match / total".into(),
        }
    }
}

/// A controlled find bar. The host owns the query, the replacement, the
/// options and the results, and answers the `InputChanged` and `Clicked`
/// actions of [`FindIds`]. [`Component::render`] gives the bar alone;
/// [`FindBar::attach`] floats it under an anchor, over the document.
pub struct FindBar {
    query: String,
    palette: ComponentPalette,
    surface: ComponentPalette,
    replacement: Option<String>,
    replace_open: bool,
    options: FindOptions,
    count_label: String,
    has_matches: bool,
    has_current: bool,
    no_matches: bool,
    error: Option<String>,
    width: f32,
    labels: FindLabels,
    ids: FindIds,
    ink: Option<Color>,
    replace_icons: (Icon, Icon),
}

impl FindBar {
    /// `palette` colors the fields and the buttons and, unless
    /// [`Self::surface`] says otherwise, the bar under them.
    pub fn new(id: impl Into<String>, query: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            query: query.into(),
            palette,
            surface: palette,
            replacement: None,
            replace_open: false,
            options: FindOptions::default(),
            count_label: String::new(),
            has_matches: false,
            has_current: false,
            no_matches: false,
            error: None,
            width: MAX_WIDTH,
            labels: FindLabels::default(),
            ids: FindIds::new(&id.into()),
            ink: None,
            replace_icons: (Icon::ArrowRight, Icon::Refresh),
        }
    }

    /// The palette of the bar itself: its `surface`, `border` and `shadow`.
    /// For a bar that floats on a menu surface while its controls keep the
    /// page's palette.
    pub fn surface(mut self, palette: ComponentPalette) -> Self {
        self.surface = palette;
        self
    }

    /// The text of the replace field. `None` is an empty field.
    pub fn replacement(mut self, replacement: Option<&str>) -> Self {
        self.replacement = replacement.map(str::to_owned);
        self
    }

    /// Show the replace row.
    pub fn replace_open(mut self, open: bool) -> Self {
        self.replace_open = open;
        self
    }

    /// Which option toggles show as on.
    pub fn options(mut self, options: FindOptions) -> Self {
        self.options = options;
        self
    }

    /// What the count reads, such as `2 / 14`. The host formats it.
    pub fn count_label(mut self, label: impl Into<String>) -> Self {
        self.count_label = label.into();
        self
    }

    /// There is a match to step to: enables previous, next and replace all.
    pub fn has_matches(mut self, has_matches: bool) -> Self {
        self.has_matches = has_matches;
        self
    }

    /// The selection is one of the matches: enables replacing it.
    pub fn has_current(mut self, has_current: bool) -> Self {
        self.has_current = has_current;
        self
    }

    /// The search finished and found nothing: the count takes the ink.
    pub fn no_matches(mut self, no_matches: bool) -> Self {
        self.no_matches = no_matches;
        self
    }

    /// Why the query could not be searched, as a last row in the ink. It is
    /// shown on one line; its tooltip keeps the text as given.
    pub fn error(mut self, error: Option<&str>) -> Self {
        self.error = error.map(str::to_owned);
        self
    }

    /// The width available to the bar, which takes at most 520. Values that
    /// are not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.clamp(0.0, MAX_WIDTH);
        }
        self
    }

    /// The host's strings in place of the English defaults.
    pub fn labels(mut self, labels: FindLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Use the host's own IDs in place of the ones derived from the bar's.
    pub fn ids(mut self, ids: FindIds) -> Self {
        self.ids = ids;
        self
    }

    /// The host's failure color, for a count of zero and for the error.
    pub fn ink(mut self, ink: Color) -> Self {
        self.ink = Some(ink);
        self
    }

    /// The glyphs of replace and replace all.
    pub fn replace_icons(mut self, one: Icon, all: Icon) -> Self {
        self.replace_icons = (one, all);
        self
    }

    /// The height of a bar `width` wide in this state, so a host can keep
    /// what it covers in view. It is the height [`Component::render`] lays
    /// out.
    pub fn height(width: f32, replace_open: bool, error: bool) -> f32 {
        let rows = 1 + usize::from(replace_open) + usize::from(options_below(width, replace_open));
        2.0 * PADDING
            + rows as f32 * ROW
            + (rows - 1) as f32 * ROW_GAP
            + if error { ROW_GAP + ERROR_ROW } else { 0.0 }
    }

    /// Float the bar under the end of `anchor`. It takes neither the pointer
    /// nor the focus from the document, so the host closes it: on the close
    /// button and on its own shortcut.
    pub fn attach(self, anchor: Element) -> Element {
        let dismiss = self.ids.dismiss.clone();
        Element::popover(anchor, self.render(), dismiss, PopoverPlacement::BottomEnd)
            .popover_non_modal()
            // It appears where the user is already typing: no entrance.
            .popover_animation_ms(0)
    }
}

// A width that is not a number counts as narrow.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn options_below(width: f32, replace_open: bool) -> bool {
    !(width >= INLINE_OPTIONS_WIDTH) || replace_open
}

impl Component for FindBar {
    fn render(self) -> Element {
        let Self { palette, surface, labels, ids, width, .. } = self;
        let options_below = options_below(width, self.replace_open);
        let count = Tooltip::new(
            Element::truncated_text(self.count_label)
                .font_family(FontFamily::Sans)
                .font_size(11.0)
                .foreground(self.ink.filter(|_| self.no_matches).unwrap_or(palette.muted))
                .size(Some(72.0), Some(ROW)),
            ids.count,
            labels.count,
            palette,
        )
        .render();
        let field = |id, value: &str, placeholder| {
            TextField::new(id, value, placeholder, palette)
                .size(ControlSize::Compact)
                .persistent_placeholder(true)
                .render()
                .flex(1.0)
        };
        let option = |id, glyph, tip, active| {
            Button::new(id, glyph, palette)
                .variant(ButtonVariant::Ghost)
                .button_size(ButtonSize::IconExtraSmall)
                .width(ROW)
                .active(active)
                .active_hover(ActiveHover::Palette)
                .tooltip(tip)
                .render()
        };
        let tool = |id, icon, tip: TooltipSpec| {
            IconButton::new(id, icon, palette)
                .variant(ButtonVariant::Ghost)
                .size(ControlSize::Dense)
                .tooltip_with(tip.placement(TooltipPlacement::Above))
        };
        let options = Element::row(vec![
            option(ids.case, "Aa", labels.case, self.options.case),
            option(ids.word, "Ab", labels.word, self.options.word),
            option(ids.regex, ".*", labels.regex, self.options.regex),
            tool(ids.in_selection, Icon::Filter, TooltipSpec::new(labels.in_selection))
                .active(self.options.in_selection)
                .active_hover(ActiveHover::Palette)
                .render(),
        ])
        .gap(2.0)
        .size(None, Some(ROW));
        let mut query = vec![
            tool(
                ids.toggle_replace,
                if self.replace_open {
                    Icon::ChevronDown
                } else {
                    Icon::ChevronRight
                },
                TooltipSpec::new(labels.toggle_replace).shortcut(labels.toggle_replace_shortcut),
            )
            .active(self.replace_open)
            .active_hover(ActiveHover::Palette)
            .render(),
            field(ids.query, &self.query, labels.query),
        ];
        if !options_below {
            query.push(options.clone());
            query.push(count.clone());
        }
        query.extend([
            tool(
                ids.previous,
                Icon::ArrowUp,
                TooltipSpec::new(labels.previous).shortcut(labels.previous_shortcut),
            )
            .enabled(self.has_matches)
            .render(),
            tool(
                ids.next,
                Icon::ArrowDown,
                TooltipSpec::new(labels.next).shortcut(labels.next_shortcut),
            )
            .enabled(self.has_matches)
            .render(),
            tool(
                ids.dismiss,
                Icon::Close,
                TooltipSpec::new(labels.dismiss).shortcut(labels.dismiss_shortcut),
            )
            .render(),
        ]);
        let mut rows = vec![Element::row(query).gap(3.0).size(None, Some(ROW))];
        if self.replace_open {
            // Both fields share the leading slot and the three-button rail,
            // so the options never come between them.
            rows.push(
                Element::row(vec![
                    Element::spacer().size(Some(SLOT), None),
                    field(
                        ids.replacement,
                        self.replacement.as_deref().unwrap_or_default(),
                        labels.replacement,
                    ),
                    tool(
                        ids.replace_one,
                        self.replace_icons.0,
                        TooltipSpec::new(labels.replace_one).shortcut(labels.replace_one_shortcut),
                    )
                    .enabled(self.has_current)
                    .render(),
                    tool(ids.replace_all, self.replace_icons.1, TooltipSpec::new(labels.replace_all))
                        .enabled(self.has_matches)
                        .render(),
                    Element::spacer().size(Some(SLOT), None),
                ])
                .gap(3.0)
                .size(None, Some(ROW)),
            );
        }
        if options_below {
            rows.push(
                Element::row(vec![
                    Element::spacer()
                        .size(Some(if width >= LEADING_SLOT_WIDTH { SLOT } else { 0.0 }), None),
                    options,
                    Element::spacer().flex(1.0),
                    count,
                ])
                .gap(3.0)
                .size(None, Some(ROW)),
            );
        }
        if let Some(error) = &self.error {
            rows.push(
                Tooltip::new(
                    Element::truncated_text(error.split_whitespace().collect::<Vec<_>>().join(" "))
                        .font_family(FontFamily::Sans)
                        .font_size(12.0)
                        .foreground(self.ink.unwrap_or(palette.text))
                        .size(None, Some(ERROR_ROW)),
                    ids.error,
                    error,
                    palette,
                )
                .render(),
            );
        }
        Element::column(rows)
            .gap(ROW_GAP)
            .padding(PADDING)
            .size(Some(width), None)
            .background(surface.surface)
            .border(surface.border, 1.0)
            .radius(crate::tokens().radius.card)
            .shadow(surface.shadow, Point::new(0.0, 5.0), 14.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Rect, Size, UiAction, UiEvent, UiTree};

    const INK: Color = Color::rgb(220, 90, 90);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn bar(width: f32, replace: bool, error: bool) -> FindBar {
        FindBar::new("find", "render", palette())
            .replacement(Some("paint"))
            .replace_open(replace)
            .count_label("2 / 14")
            .has_matches(true)
            .has_current(true)
            .error(error.then_some("unclosed\n  group"))
            .ink(INK)
            .width(width)
    }

    // The bar at its own size in the corner of a larger window.
    fn frame_of(bar: FindBar) -> Frame {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![bar.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(800.0, 400.0))
    }

    fn surface(frame: &Frame) -> Rect {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == palette().surface => {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap()
    }

    fn region(frame: &Frame, id: &str) -> Option<Rect> {
        frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
    }

    #[test]
    fn height_is_the_rendered_height_in_every_state() {
        for width in [0.0, 120.0, 260.0, 479.0, 480.0, 520.0, 900.0] {
            for replace in [false, true] {
                for error in [false, true] {
                    let frame = frame_of(bar(width, replace, error));
                    let rect = surface(&frame);
                    assert_eq!(
                        rect.height,
                        FindBar::height(width, replace, error),
                        "{width} replace {replace} error {error}"
                    );
                    assert_eq!(rect.width, width.min(520.0));
                }
            }
        }
        assert_eq!(FindBar::height(520.0, false, false), 44.0);
        assert_eq!(FindBar::height(260.0, false, false), 76.0);
        assert_eq!(FindBar::height(520.0, true, true), 132.0);
        // A width that is not a number is laid out as the narrow bar.
        assert_eq!(FindBar::height(f32::NAN, false, false), 76.0);
    }

    #[test]
    fn options_share_the_query_row_only_when_wide_and_without_replace() {
        for (width, replace, below) in [
            (260.0, false, true),
            (480.0, false, false),
            (520.0, false, false),
            (520.0, true, true),
            (260.0, true, true),
        ] {
            let frame = frame_of(bar(width, replace, true));
            let bounds = surface(&frame);
            let query = region(&frame, "find:query").unwrap();
            let case = region(&frame, "find:case").unwrap();
            assert_eq!(case.y > query.y + query.height, below, "{width} {replace}");
            assert_eq!((case.width, case.height), (28.0, 24.0));
            assert_eq!(query.y, 8.0);
            // Every control stays inside the bar.
            for control in &frame.regions {
                assert!(
                    control.rect.x >= bounds.x
                        && control.rect.x + control.rect.width <= bounds.x + bounds.width + 0.1,
                    "{} escapes at {width}: {:?}",
                    control.id,
                    control.rect
                );
            }
            let replacement = region(&frame, "find:replacement");
            assert_eq!(replacement.is_some(), replace);
            if let Some(replacement) = replacement {
                // Under the query and exactly as wide: same slot, same rail.
                assert_eq!(replacement.y, query.y + 32.0);
                assert_eq!((replacement.x, replacement.width), (query.x, query.width));
                assert!(case.y > replacement.y + replacement.height);
            }
            let error = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, color, position, .. }
                        if text.starts_with("unclosed") =>
                    {
                        Some((text.clone(), *color, position.y))
                    }
                    _ => None,
                })
                .unwrap();
            // One line, in the ink, under every row.
            assert_eq!((error.0.as_str(), error.1), ("unclosed group", INK));
            assert!(error.2 > case.y);
        }
        // The options row gives up its leading slot on a very narrow bar.
        let case_x = |width| region(&frame_of(bar(width, false, false)), "find:case").unwrap().x;
        assert_eq!(case_x(280.0), 8.0 + 24.0 + 3.0);
        assert_eq!(case_x(279.0), 8.0 + 3.0);
    }

    #[test]
    fn every_control_reports_its_id_and_disabled_ones_are_passive() {
        let ids = FindIds {
            previous: "editor_search_prev".into(),
            ..FindIds::new("find")
        };
        let mut tree = UiTree::new(bar(520.0, true, false).ids(ids.clone()).render());
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(520.0, 200.0));
        for id in [
            &ids.case,
            &ids.word,
            &ids.regex,
            &ids.in_selection,
            &ids.toggle_replace,
            &ids.previous,
            &ids.next,
            &ids.dismiss,
            &ids.replace_one,
            &ids.replace_all,
        ] {
            let rect = frame
                .regions
                .iter()
                .find(|region| &region.id == id && region.kind == HitKind::Button)
                .unwrap_or_else(|| panic!("missing {id}"))
                .rect;
            let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: id.clone() }]
            );
        }
        assert!(tree.focus("find:query"));
        assert_eq!(
            tree.handle_event(UiEvent::Text("s".into())),
            vec![UiAction::InputChanged { id: "find:query".into(), value: "renders".into() }]
        );
        assert!(tree.focus("find:replacement"));

        // Nothing found: stepping and replacing cannot be reached.
        let idle = bar(520.0, true, false).has_matches(false).has_current(false).no_matches(true);
        let frame = frame_of(idle);
        for id in ["find:previous", "find:next", "find:replace_one", "find:replace_all"] {
            assert!(
                !frame.regions.iter().any(|region| region.id == id && region.kind == HitKind::Button),
                "{id} must be passive"
            );
        }
        assert!(region(&frame, "find:dismiss").is_some());
        let count = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Text { text, color, .. } if text == "2 / 14" => Some(*color),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(count(&frame), INK);
        assert_eq!(count(&frame_of(bar(520.0, false, false))), palette().muted);
    }

    #[test]
    fn the_bar_takes_its_surface_from_the_second_palette_and_floats_non_modal() {
        let menu = ComponentPalette {
            surface: Color::rgb(1, 2, 3),
            border: Color::rgb(4, 5, 6),
            ..palette()
        };
        let frame = frame_of(bar(520.0, false, false).surface(menu));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, rect, radius }
                if *color == menu.surface
                    && rect.width == 520.0
                    && *radius == crate::tokens().radius.card)));

        let anchor = Element::button("header", "Header").size(None, Some(30.0));
        let mut tree = UiTree::new(Element::column(vec![
            bar(300.0, false, false).attach(anchor),
            Element::button("document", "Document").flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(640.0, 400.0));
        let bounds = surface(&frame);
        // Under the anchor, against its end.
        let header = region(&frame, "header").unwrap();
        assert!(bounds.y >= header.y + header.height);
        // Against the anchor's end, less the margin a popover keeps from the window.
        let end = bounds.x + bounds.width;
        assert!(end <= header.x + header.width && end >= header.x + header.width - 12.0, "{bounds:?}");
        // The document under and beside the bar still takes the pointer.
        let point = Point::new(40.0, 200.0);
        assert_eq!(frame.hit_test(point), Some("document"));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "document".into() }]
        );
    }
}
