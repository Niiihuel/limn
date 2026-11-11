//! A search input: a magnifier, the query, and a button that clears it.

use crate::text_field::{disabled_field, input_radius, input_surface};
use crate::{Access, Component, ComponentPalette, ControlSize, ControlState, Element, FontFamily, Icon, Kbd, Role, ICON_SIZE};

/// An editable search control. Its magnifier, text, caret, selection and
/// focus border all live in the same hit box, so pointer and keyboard focus
/// cannot disagree about the field's bounds. The host owns the query and
/// handles `UiAction::InputChanged` for `id`.
pub struct SearchField {
    id: String,
    value: String,
    placeholder: String,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
    height: Option<f32>,
    state: ControlState,
    shortcut: Option<String>,
    clearable: bool,
    persistent_placeholder: bool,
    accessibility_label: Option<String>,
}

/// The application's shared search input; the existing `SearchField` name
/// remains available to current callers.
pub type Search = SearchField;

impl SearchField {
    /// A search field with its current query and the placeholder shown while
    /// it is empty. The host stores what `InputChanged` reports under this ID.
    pub fn new(
        id: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            palette,
            size: ControlSize::Spacious,
            width: None,
            height: None,
            state: ControlState::Enabled,
            shortcut: None,
            clearable: false,
            persistent_placeholder: false,
            accessibility_label: None,
        }
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Take the height and text size of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// A height of the host's own, for a field taller than a [`ControlSize`]'s,
    /// such as a settings page's 40. The field is never shorter than its control
    /// size; a value that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height);
        }
        self
    }

    /// Keep the placeholder visible while the field is focused and empty, as
    /// a settings page's search does, instead of hiding it at the first key.
    pub fn persistent_placeholder(mut self, visible: bool) -> Self {
        self.persistent_placeholder = visible;
        self
    }

    /// Show the keys of a shortcut in the field, as a hint. The field does
    /// not bind them: the host does.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Show a button at the end of the field while the query is not empty,
    /// which reports `Clicked` with `"{id}:clear"`. The field does not empty
    /// itself: the host clears its query and gives the field its new text
    /// with `UiTree::set_input_value`, and may give it the focus back.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for SearchField {
    fn render(self) -> Element {
        let clear =
            (self.clearable && !self.value.is_empty()).then(|| format!("{}:clear", self.id));
        let least = self.size.height().max(28.0);
        let height = self.height.map_or(least, |height| height.max(least));
        let radius = input_radius(self.size);
        let surface = self.palette.raised;
        if self.state == ControlState::Disabled {
            let access = disabled_field(
                Role::SearchInput,
                self.id,
                self.accessibility_label,
                &self.placeholder,
                Some(self.value.clone()),
            );
            // What the enabled field shows away from the focus: the hint only
            // while there is no query, so it never meets the clear button.
            let shortcut = self.shortcut.filter(|_| self.value.is_empty());
            let (value, ink) = if self.value.is_empty() {
                (self.placeholder, self.palette.muted)
            } else {
                (self.value, self.palette.muted)
            };
            let mut children = vec![
                Element::icon(Icon::Search)
                    .foreground(self.palette.muted)
                    .size(Some(ICON_SIZE), Some(height)),
                Element::truncated_text(value)
                    .foreground(ink)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0)
                    .size(None, Some(height))
                    .flex(1.0),
            ];
            if let Some(shortcut) = shortcut {
                let mut hint = Kbd::new(shortcut, self.palette);
                if let Some(width) = self.width {
                    hint = hint.max_width((width - 16.0 - ICON_SIZE - 16.0 - 24.0).max(0.0));
                }
                let hint = hint.render().size(None, Some(height));
                children.push(if self.width.is_some() {
                    hint
                } else {
                    hint.flex(1.0)
                });
            }
            let mut row = vec![
                Element::spacer().size(Some(8.0), None),
                Element::row(children).gap(8.0).flex(1.0),
            ];
            match clear {
                // Where the enabled one is, passive and still known by its ID.
                Some(id) => row.extend([
                    clear_button(id, self.palette, false).centered_y(),
                    Element::spacer().size(Some(CLEAR_INSET), None),
                ]),
                None => row.push(Element::spacer().size(Some(8.0), None)),
            }
            return Element::panel(Element::row(row))
                .background(surface)
                .border(self.palette.border, 1.0)
                .radius(radius)
                .size(self.width, Some(height))
                .accessible(access);
        }
        let mut access = Access::new(Role::SearchInput);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
        }
        let mut input = input_surface(
            Element::text_input(self.id, self.value, self.placeholder)
                .leading_icon(Icon::Search)
                .leading_icon_tint(self.palette.muted)
                .persistent_placeholder(self.persistent_placeholder),
            self.palette,
            self.size,
            self.width,
        )
        .size(self.width, Some(height));
        if let Some(shortcut) = self.shortcut {
            input = input
                .input_shortcut(shortcut)
                .input_shortcut_fill(crate::shortcut::cap_fill(self.palette.text));
        }
        let input = input.accessible(access);
        let Some(clear) = clear else { return input };
        // The button lies over the field's end, which the text leaves free as
        // it leaves the magnifier's room at the start; the field stays lit
        // while the pointer is on the button.
        Element::stack(vec![
            input
                .input_trailing_space(ICON_SIZE + 8.0)
                .hover_with(clear.clone()),
            Element::row(vec![
                Element::spacer().flex(1.0),
                clear_button(clear, self.palette, true).centered_y(),
                Element::spacer().size(Some(CLEAR_INSET), None),
            ]),
        ])
        // As wide as the field alone, so typing does not narrow it.
        .min_size(80.0, height)
        .size(self.width, Some(height))
    }
}

/// Side of a search field's clear button: its glyph and 4 around it.
pub(crate) const CLEAR_SIDE: f32 = ICON_SIZE + 8.0;
/// From the clear button to the field's end, so that its glyph is as far
/// from that end as the magnifier is from the start.
const CLEAR_INSET: f32 = 8.0 - (CLEAR_SIDE - ICON_SIZE) / 2.0;

// A circled cross that empties a search field; drawn disabled it is passive.
pub(crate) fn clear_button(id: String, palette: ComponentPalette, enabled: bool) -> Element {
    // An icon alone: it is named for what it does.
    let said = Access::new(Role::Button).name(crate::labels().clear_search);
    if enabled {
        Element::icon_button(id, Icon::CloseCircle, "")
            .foreground(palette.muted)
            .hover_foreground(palette.text)
            .pressed_foreground(palette.text)
            .hover_background(palette.hover)
            .pressed_background(palette.selected)
            .accessible(said)
    } else {
        Element::panel(
            Element::icon(Icon::CloseCircle)
                .foreground(palette.muted)
                .size(Some(ICON_SIZE), Some(ICON_SIZE))
                .centered(),
        )
        .accessible(said.id(id).disabled(true))
    }
    // Half the side is a shape, not a token.
    .radius(CLEAR_SIDE / 2.0)
    .size(Some(CLEAR_SIDE), Some(CLEAR_SIDE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lighter, AccessNode, Button, ButtonVariant, DrawCommand, Point, Size, UiAction, UiEvent, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    // A field at the top left of a window, at its own size.
    fn field_tree(field: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![field, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    #[test]
    fn a_search_field_is_a_search_input_with_its_name_and_query_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let field = || SearchField::new("search", "tabs", "Search", palette).shortcut("Ctrl+K").enabled(enabled);
            let said = access_node(field().render(), "search");
            assert_eq!((said.role, said.name.as_str()), (Role::SearchInput, "Search"));
            assert_eq!(said.value_text.as_deref(), Some("tabs"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(field().accessibility_label("Search files").render(), "search").name, "Search files");
        }
    }


    #[test]
    fn search_field_taller_than_its_line_centers_the_text_and_disabled_labels_fit() {
        let palette = ComponentPalette::neutral_dark();
        let frame = UiTree::new(Element::column(vec![
            SearchField::new("search", "", "Buscar", palette).shortcut("Ctrl+K").width(300.0).render(),
            Element::row(vec![
                Button::new("off", "Destructive", palette)
                    .variant(ButtonVariant::Destructive)
                    .disabled()
                    .render(),
                Element::spacer().flex(1.0),
            ]),
        ]))
        .layout(Size::new(400.0, 200.0));
        let field = frame.regions.iter().find(|region| region.id == "search").expect("field").rect;
        let text = |wanted: &str| {
            frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == wanted => Some(*position),
                _ => None,
            })
        };
        let placeholder = text("Buscar").expect("placeholder");
        let line = 14.0 * 13.0 / 14.0;
        assert!(field.height > 32.0);
        assert!(
            (placeholder.y - field.y - (field.height - line) / 2.0).abs() < 4.0
                && placeholder.y - field.y > 8.5,
            "placeholder at {} in a field of {}",
            placeholder.y - field.y,
            field.height
        );
        assert!(text("Destructive").is_some(), "disabled label was truncated");
    }


    #[test]
    fn search_field_height_and_persistent_placeholder_are_opt_in() {
        let palette = ComponentPalette::neutral_dark();
        let search = || SearchField::new("search", "", "Buscar", palette).width(300.0);
        let height = |field: SearchField| {
            // A column keeps the field at its own height; a root would be the whole window.
            let frame = UiTree::new(Element::column(vec![field.render()])).layout(Size::new(400.0, 200.0));
            frame.regions.iter().find(|region| region.id == "search").expect("field").rect.height
        };
        assert_eq!(height(search()), 36.0, "Spacious unless the host says otherwise");
        assert_eq!(height(search().height(40.0)), 40.0);
        assert_eq!(height(search().height(30.0)), 36.0, "never shorter than its control size");
        assert_eq!(height(search().height(f32::NAN)), 36.0, "a height that is not finite is ignored");
        // Focused and empty, the placeholder goes unless it is kept.
        let placeholder_while_focused = |field: SearchField| {
            let mut tree = UiTree::new(field.render());
            tree.layout(Size::new(400.0, 200.0));
            assert!(tree.focus("search"));
            tree.layout(Size::new(400.0, 200.0)).commands.iter().any(
                |command| matches!(command, DrawCommand::Text { text, .. } if text == "Buscar"),
            )
        };
        assert!(!placeholder_while_focused(search()));
        assert!(placeholder_while_focused(search().persistent_placeholder(true)));
    }


    #[test]
    fn search_hint_stays_inside_the_editors_single_hit_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(280.0)
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 36.0));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "query");
        for key in ["Ctrl", "K"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, font_family: FontFamily::Sans, .. } if text == key)));
        }
        assert!(tree.focus("query"));
    }


    #[test]
    fn search_shortcut_yields_space_to_focus_and_typed_text() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(280.0)
                .render(),
            Element::button("outside", "Otro"),
        ]));
        let has_hint = |frame: &crate::Frame| {
            ["Ctrl", "K"].iter().all(|key| {
                frame
                    .commands
                    .iter()
                    .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == key))
            })
        };
        assert!(has_hint(&tree.layout(Size::new(280.0, 72.0))));
        assert!(tree.focus("query"));
        assert!(!has_hint(&tree.layout(Size::new(280.0, 72.0))));
        tree.handle_event(UiEvent::Text("Rust".to_owned()));
        assert!(tree.focus("outside"));
        assert!(!has_hint(&tree.layout(Size::new(280.0, 72.0))));
        tree.set_input_value("query", "");
        assert!(has_hint(&tree.layout(Size::new(280.0, 72.0))));
    }


    #[test]
    fn a_clearable_search_field_has_its_button_only_while_there_is_a_query() {
        let palette = ComponentPalette::neutral_dark();
        let window = Size::new(400.0, 120.0);
        let field = |query: &str| {
            SearchField::new("query", query, "Search", palette)
                .clearable(true)
                .width(280.0)
        };
        let ids = |tree: &mut UiTree| -> Vec<String> {
            tree.layout(window)
                .regions
                .iter()
                .map(|region| region.id.clone())
                .collect()
        };
        // Empty, or with a query but not clearable, it is one input as before.
        assert_eq!(ids(&mut field_tree(field("").render())), ["query"]);
        let plain = SearchField::new("query", "rust", "Search", palette).width(280.0);
        assert_eq!(ids(&mut field_tree(plain.render())), ["query"]);

        let mut tree = field_tree(field("rust").render());
        let frame = tree.layout(window);
        let rect = |id: &str| {
            frame
                .regions
                .iter()
                .find(|region| region.id == id)
                .map(|region| region.rect)
        };
        let (input, clear) = (
            rect("query").unwrap(),
            rect("query:clear").expect("the clear button"),
        );
        // A circle at the end whose glyph is as far from it as the magnifier
        // is from the start, on the field's middle line.
        assert_eq!((input.width, input.height), (280.0, 36.0));
        assert_eq!((clear.width, clear.height), (24.0, 24.0));
        assert_eq!(input.x + input.width - (clear.x + clear.width), 4.0);
        assert_eq!(clear.y + clear.height / 2.0, input.y + input.height / 2.0);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::CloseCircle, rect, tint, .. }
                if *tint == palette.muted && rect.x + rect.width == input.x + input.width - 8.0)));
        // The text, its caret and its selection are drawn before the button.
        let text = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::PushClip(clip) if clip.x == input.x + 8.0 + ICON_SIZE + 8.0 => {
                    Some(*clip)
                }
                _ => None,
            })
            .expect("the text's clip");
        assert!(
            text.x + text.width < clear.x,
            "{text:?} runs under {clear:?}"
        );
        assert_eq!(
            frame.hit_test(Point::new(clear.x + 12.0, clear.y + 12.0)),
            Some("query:clear")
        );

        // The pointer on the button keeps the field lit and lights the button.
        let middle = Point::new(clear.x + 12.0, clear.y + 12.0);
        tree.handle_event(UiEvent::PointerMove(middle));
        let lit = tree.layout(window);
        let fill = |rect: crate::Rect| {
            lit.commands.iter().find_map(|command| match command {
                DrawCommand::FillRect {
                    rect: drawn, color, ..
                } if *drawn == rect => Some(*color),
                _ => None,
            })
        };
        assert_eq!(fill(input), Some(lighter(palette.raised, 5)));
        assert_eq!(fill(clear), Some(palette.hover));

        // A click reports it; the host empties its query and the field.
        tree.handle_event(UiEvent::PointerDown(middle));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(middle)),
            vec![UiAction::Clicked {
                id: "query:clear".into()
            }]
        );
        tree.set_input_value("query", "");
        tree.set_root(Element::column(vec![
            Element::row(vec![field("").render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        assert_eq!(ids(&mut tree), ["query"]);
        assert_eq!(tree.input_value("query"), Some(""));
    }


    #[test]
    fn typing_into_a_clearable_search_field_keeps_its_focus_and_caret_as_the_button_comes() {
        let palette = ComponentPalette::neutral_dark();
        let window = Size::new(400.0, 120.0);
        let view = |query: &str| {
            Element::column(vec![
                Element::row(vec![
                    SearchField::new("query", query, "Search", palette)
                        .clearable(true)
                        .width(280.0)
                        .render(),
                    Element::spacer().flex(1.0),
                ]),
                Element::spacer().flex(1.0),
            ])
        };
        let mut tree = UiTree::new(view(""));
        tree.layout(window);
        assert!(tree.focus("query"));
        let reported = tree.handle_event(UiEvent::Text("ru".into()));
        assert_eq!(
            reported,
            vec![UiAction::InputChanged {
                id: "query".into(),
                value: "ru".into()
            }]
        );
        // The host stores the query; the field is rebuilt with its button.
        tree.set_root(view("ru"));
        let frame = tree.layout(window);
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "query:clear"));
        assert_eq!(tree.focused_id(), Some("query"));
        assert_eq!(
            (tree.input_value("query"), tree.input_cursor("query")),
            (Some("ru"), Some(2))
        );
        tree.handle_event(UiEvent::Text("st".into()));
        assert_eq!(tree.input_value("query"), Some("rust"));
    }


    #[test]
    fn a_disabled_clearable_search_field_keeps_a_passive_clear_button_in_place() {
        let palette = ComponentPalette::neutral_dark();
        let field = || {
            SearchField::new("query", "rust", "Search", palette)
                .clearable(true)
                .width(280.0)
        };
        let node = |element: Element| {
            let frame = field_tree(element).layout(Size::new(400.0, 120.0));
            assert!(frame
                .regions
                .iter()
                .all(|region| region.id != "query:clear"));
            frame
                .access
                .iter()
                .find(|node| node.id == "query:clear")
                .cloned()
                .expect("its node")
        };
        let said = access_node(field().render(), "query:clear");
        assert_eq!(
            (said.role, said.name.as_str(), said.disabled),
            (Role::Button, "Clear search", false)
        );
        let disabled = node(field().disabled().render());
        assert_eq!(
            (disabled.role, disabled.name.as_str(), disabled.disabled),
            (Role::Button, "Clear search", true)
        );
        assert_eq!(disabled.rect, said.rect);
        // Without a query there is nothing to clear, enabled or not.
        let empty = field_tree(
            SearchField::new("query", "", "Search", palette)
                .clearable(true)
                .disabled()
                .render(),
        )
        .layout(Size::new(400.0, 120.0));
        assert!(empty.access.iter().all(|node| node.id != "query:clear"));
    }


    #[test]
    fn a_clearable_search_field_keeps_its_button_inside_at_every_width_and_size() {
        let palette = ComponentPalette::neutral_dark();
        for size in [
            ControlSize::Dense,
            ControlSize::Compact,
            ControlSize::Regular,
            ControlSize::Spacious,
        ] {
            for width in [60.0, 120.0, 360.0] {
                let field =
                    SearchField::new("query", "a query longer than the field", "Search", palette)
                        .clearable(true)
                        .size(size)
                        .width(width);
                let frame = field_tree(field.render()).layout(Size::new(400.0, 120.0));
                let rect = |id: &str| {
                    frame
                        .regions
                        .iter()
                        .find(|region| region.id == id)
                        .unwrap()
                        .rect
                };
                let (input, clear) = (rect("query"), rect("query:clear"));
                assert!(
                    clear.x >= input.x
                        && clear.x + clear.width <= input.x + input.width
                        && clear.y >= input.y
                        && clear.y + clear.height <= input.y + input.height,
                    "{size:?} at {width}: {clear:?} outside {input:?}"
                );
                // As wide as it is empty: never under a field's 80.
                let empty = field_tree(
                    SearchField::new("query", "", "Search", palette)
                        .clearable(true)
                        .size(size)
                        .width(width)
                        .render(),
                )
                .layout(Size::new(400.0, 120.0));
                assert_eq!(
                    input.width, empty.regions[0].rect.width,
                    "{size:?} at {width}"
                );
                assert_eq!(input.width, f32::max(width, 80.0));
            }
        }
        // The narrowest field still has room for what is typed in it.
        let frame = field_tree(
            SearchField::new("query", "a", "Search", palette)
                .clearable(true)
                .width(60.0)
                .render(),
        )
        .layout(Size::new(400.0, 120.0));
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "a")));
    }


    #[test]
    fn a_disabled_search_field_shows_its_hint_only_without_a_query() {
        let palette = ComponentPalette::neutral_dark();
        let drawn = |query: &str, clearable: bool| {
            let frame = field_tree(
                SearchField::new("query", query, "Search", palette)
                    .clearable(clearable)
                    .shortcut("Ctrl+K")
                    .width(140.0)
                    .disabled()
                    .render(),
            )
            .layout(Size::new(400.0, 120.0));
            let texts: Vec<String> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            (
                texts.iter().any(|text| text == "Ctrl"),
                texts.iter().any(|text| text == "rust"),
            )
        };
        // As the enabled field away from the focus: the hint while it is
        // empty, the query and its clear button once it is not.
        assert_eq!(drawn("", true), (true, false));
        assert_eq!(drawn("rust", true), (false, true));
        assert_eq!(drawn("rust", false), (false, true));
    }


    #[test]
    fn disabled_search_retains_complete_caps_within_compact_control() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .size(ControlSize::Regular)
                .width(280.0)
                .disabled()
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 32.0));
        for key in ["Ctrl", "K"] {
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == key)));
        }
        for command in &frame.commands {
            if let DrawCommand::FillRect { rect, color, .. } = command {
                if *color == crate::shortcut::cap_fill(palette.text) {
                    assert!(rect.y >= 0.0 && rect.y + rect.height <= 32.0);
                }
            }
        }
        assert!(frame.regions.is_empty());
        assert!(!tree.focus("query"));
        let narrow = UiTree::new(
            Search::new("query", "", "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(80.0)
                .disabled()
                .render(),
        )
        .layout(Size::new(80.0, 36.0));
        assert!(!narrow.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K")));
    }


    #[test]
    fn disabled_search_without_explicit_width_bounds_shortcuts_to_parent() {
        let palette = ComponentPalette::neutral_dark();
        for (width, hint_visible) in [(80.0, false), (280.0, true)] {
            let frame = UiTree::new(
                Search::new("query", "", "Buscar", palette)
                    .shortcut("Ctrl+K")
                    .disabled()
                    .render(),
            )
            .layout(Size::new(width, 36.0));
            for key in ["Ctrl", "K"] {
                assert_eq!(
                    frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { text, .. } if text == key)),
                    hint_visible
                );
            }
            assert!(frame.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text.starts_with('B'))));
            for command in &frame.commands {
                if let DrawCommand::FillRect { rect, color, .. } = command {
                    if *color == crate::shortcut::cap_fill(palette.text) {
                        assert!(rect.x >= 8.0 && rect.x + rect.width <= width - 8.0);
                    }
                }
            }
            assert!(frame.regions.is_empty());
        }
    }


    #[test]
    fn search_field_keeps_icon_editor_and_focus_ring_in_one_hit_box() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(
            SearchField::new("query", "", "Buscar", palette)
                .width(280.0)
                .render(),
        );
        let frame = tree.layout(Size::new(280.0, 40.0));
        let region = frame
            .regions
            .iter()
            .find(|region| region.id == "query")
            .unwrap();
        assert_eq!(region.rect.width, 280.0);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Icon { icon: Icon::Search, rect, .. }
                if rect.x >= region.rect.x && rect.x + rect.width <= region.rect.x + region.rect.width
        )));
        let hit_box = region.rect;
        assert!(tree.focus("query"));
        let focused = tree.layout(Size::new(280.0, 40.0));
        assert_eq!(
            focused
                .regions
                .iter()
                .find(|region| region.id == "query")
                .unwrap()
                .rect,
            hit_box
        );
        assert!(focused.commands.iter().any(|command| matches!(
            command,
            DrawCommand::StrokeRect { rect, .. } if *rect == hit_box
        )));
    }
}
