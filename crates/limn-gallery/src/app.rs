//! Gallery state and shell. Everything here is rebuilt per frame from
//! [`State`], so a hot-patched view picks up where the old one left off.

use crate::pages::{self, Cx};
use limn::{
    Button, ButtonSize, ButtonVariant, Color, Component, Element, FontFamily, Motion, Radius,
    ScrollArea, Size, Theme, Tokens, UiAction,
};
use std::collections::HashMap;

const SIDEBAR_WIDTH: f32 = 220.0;

const LIGHT: Theme = Theme {
    background: Color::rgb(255, 255, 255),
    foreground: Color::rgb(24, 26, 31),
    accent: Color::rgb(9, 105, 218),
};

/// Demo state keyed by control ID. It is a map rather than one field per
/// demo so that adding a demo never changes this struct's layout, which a
/// hot patch could not survive.
#[derive(Default)]
pub struct State {
    values: HashMap<String, String>,
}

impl State {
    pub fn starting_at(page: &str) -> Self {
        let mut state = Self::default();
        state.set("page", page);
        state
    }

    /// One `id<TAB>value` line per entry, with tabs, newlines and backslashes
    /// escaped, so the state can be written to a file and read back.
    pub fn serialize(&self) -> String {
        let escape = |text: &str| {
            text.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n")
        };
        let mut lines: Vec<String> = self
            .values
            .iter()
            .map(|(id, value)| format!("{}\t{}\n", escape(id), escape(value)))
            .collect();
        lines.sort();
        lines.concat()
    }

    /// The inverse of [`State::serialize`]. Lines it cannot read are skipped.
    // Only the hot-reloading desktop build restores a session.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub fn deserialize(text: &str) -> Self {
        fn unescape(text: &str) -> String {
            let mut out = String::with_capacity(text.len());
            let mut characters = text.chars();
            while let Some(character) = characters.next() {
                if character != '\\' {
                    out.push(character);
                    continue;
                }
                match characters.next() {
                    Some('t') => out.push('\t'),
                    Some('n') => out.push('\n'),
                    Some(other) => out.push(other),
                    None => {}
                }
            }
            out
        }
        let mut state = Self::default();
        for line in text.lines() {
            if let Some((id, value)) = line.split_once('\t') {
                state.values.insert(unescape(id), unescape(value));
            }
        }
        state
    }

    pub fn text(&self, id: &str) -> &str {
        self.values.get(id).map_or("", String::as_str)
    }

    pub fn flag(&self, id: &str) -> bool {
        self.text(id) == "on"
    }

    /// The chosen value of a group, or `default` before any choice.
    pub fn choice<'a>(&'a self, group: &str, default: &'a str) -> &'a str {
        self.values.get(group).map_or(default, String::as_str)
    }

    pub fn set(&mut self, id: &str, value: &str) {
        self.values.insert(id.to_owned(), value.to_owned());
    }

    /// Shape and pace chosen on the tokens page.
    pub fn tokens(&self) -> Tokens {
        let radius = match self.choice("tokens-radius", "default") {
            "square" => Radius::SQUARE,
            "soft" => Radius::DEFAULT.scaled(2.0),
            "round" => Radius::ROUND,
            _ => Radius::DEFAULT,
        };
        let motion = match self.choice("tokens-motion", "default") {
            "fast" => Motion::DEFAULT.scaled(0.5),
            "slow" => Motion::DEFAULT.scaled(4.0),
            _ => Motion::DEFAULT,
        };
        Tokens { radius, motion }
    }

    pub fn theme(&self) -> Theme {
        if self.flag("light") {
            LIGHT
        } else {
            Theme::DARK
        }
    }
}

/// Apply one action. A click on `group=value` chooses within a group, and
/// `a=1&b=2` makes several choices at once; any other click flips the flag
/// named by its ID. Pages need no update code, except a demo whose state is
/// neither, which answers in [`pages::update`].
pub fn update(state: &mut State, action: &UiAction) {
    pages::update(state, action);
    match action {
        UiAction::Clicked { id } => {
            for part in id.split('&') {
                match part.split_once('=') {
                    Some((group, value)) => state.set(group, value),
                    None => state.set(part, if state.flag(part) { "off" } else { "on" }),
                }
            }
            state.set("last-action", &format!("Clicked {id}"));
        }
        UiAction::InputChanged { id, value } => state.set(id, value),
        UiAction::SliderChanged { id, fraction, .. } => state.set(id, &fraction.to_string()),
        UiAction::Submitted { id, .. } => state.set("last-action", &format!("Submitted {id}")),
        _ => {}
    }
}

pub fn view(state: &State, window: Size) -> Element {
    // Before anything is built: elements read the tokens as they are made.
    limn::set_tokens(state.tokens());
    let theme = state.theme();
    let palette = theme.palette();
    let current = pages::find(state.choice("page", pages::PAGES[0].id));
    let cx = Cx {
        state,
        palette,
        width: if state.flag("embed") {
            window.width - 48.0
        } else {
            window.width - SIDEBAR_WIDTH - 64.0
        }
        .max(240.0),
    };

    let mut sidebar = vec![Element::text("limn")
        .font_family(FontFamily::Sans)
        .font_size(16.0)
        .foreground(palette.text)
        .padding(8.0)];
    let mut group = "";
    for page in pages::PAGES {
        if page.group != group {
            group = page.group;
            sidebar.push(pages::caption(&cx, group).padding(8.0));
        }
        sidebar.push(
            Button::new(format!("page={}", page.id), page.title, palette)
                // Subtle is the navigation row: it keeps its label at the left.
                .variant(ButtonVariant::Subtle)
                .button_size(ButtonSize::Small)
                .active(page.id == current.id)
                .width(SIDEBAR_WIDTH - 24.0)
                .render(),
        );
    }
    // A page for each component is more than a window's height of them.
    let mut sidebar = vec![ScrollArea::new("sidebar-scroll", Element::column(sidebar).gap(2.0)).render().flex(1.0)];
    sidebar.push(
        Button::new("light", if state.flag("light") { "Dark theme" } else { "Light theme" }, palette)
            .variant(ButtonVariant::Outline)
            .button_size(ButtonSize::Small)
            .width(SIDEBAR_WIDTH - 24.0)
            .render(),
    );
    sidebar.push(pages::caption(&cx, state.text("last-action")).padding(8.0));

    let page = Element::column(vec![
        Element::text(current.title)
            .font_family(FontFamily::Sans)
            .font_size(24.0)
            .foreground(palette.text),
        current.render(&cx),
    ])
    .gap(24.0)
    .padding(32.0);

    // Embedded in a site's page, the gallery is only the component: the site
    // has the navigation and the title.
    // A site that embeds a page gives it its own ground, as `embed-bg=rrggbb`.
    let ground = pages::ground(&cx);
    let content = if state.flag("embed") && !state.text("embed-diagram").is_empty() {
        // A site's own diagram, drawn here from its Mermaid source.
        let diagram = match limn::Diagram::parse(state.text("embed-diagram"), palette) {
            Ok(diagram) => diagram.backdrop(ground).scrollable("embed-diagram"),
            Err(error) => pages::caption(&cx, &error.to_string()),
        };
        vec![ScrollArea::new("page-scroll", diagram.centered_x().padding(24.0)).render().flex(1.0)]
    } else if state.flag("embed") {
        let page = current.render(&cx).padding(24.0);
        let page = if state.text("embed-variant").is_empty() {
            page
        } else {
            // One example alone stands in the middle of its card, and scrolls when it is taller.
            Element::column(vec![Element::spacer().flex(1.0), page, Element::spacer().flex(1.0)])
                .min_size(0.0, window.height)
        };
        vec![ScrollArea::new("page-scroll", page).render().flex(1.0)]
    } else {
        vec![
            Element::panel(Element::column(sidebar).gap(2.0))
                .padding(12.0)
                .background(palette.surface)
                .size(Some(SIDEBAR_WIDTH), None),
            ScrollArea::new("page-scroll", page).render().flex(1.0),
        ]
    };
    let root = Element::panel(Element::row(content))
    .background(ground)
    .size(Some(window.width), Some(window.height));
    // Dialogs and toasts float over the whole window, whatever the page.
    pages::layers(&cx, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use limn::{UiEvent, UiTree};
    use std::collections::HashSet;

    #[test]
    fn the_gallery_keeps_english_library_labels() {
        let window = Size::new(700.0, 200.0);
        view(&State::starting_at("button"), window);
        assert_eq!(limn::labels().close, "Close");
        assert_eq!(limn::labels().cancel, "Cancel");
    }

    #[test]
    fn every_page_lays_out_in_both_themes_at_several_widths() {
        for page in pages::PAGES {
            for light in [false, true] {
                // The last is the page alone, at the size a site embeds it.
                for (width, height, embed) in [(640.0, 800.0, false), (1280.0, 800.0, false), (2200.0, 800.0, false), (700.0, 200.0, true)] {
                    let mut state = State::starting_at(page.id);
                    if light {
                        state.set("light", "on");
                    }
                    if embed {
                        state.set("embed", "on");
                    }
                    let window = Size::new(width, height);
                    let frame = UiTree::new(view(&state, window)).layout(window);
                    assert!(!frame.commands.is_empty(), "{} paints nothing", page.id);
                    // Two controls with one ID would share hover, fill and focus.
                    let mut ids = HashSet::new();
                    let mut previous = None;
                    for region in frame.regions.iter().filter(|region| region.kind == limn::HitKind::Button) {
                        // A Markdown link wrapped over two lines is one control in consecutive pieces.
                        if !(region.id.starts_with("markdown-") && previous == Some(&region.id)) {
                            assert!(ids.insert(&region.id), "{}: duplicate id {}", page.id, region.id);
                        }
                        previous = Some(&region.id);
                    }
                }
            }
        }
    }

    /// A site shows each example alone in a card: none may run out of it.
    #[test]
    fn every_example_alone_stays_inside_the_card_a_site_gives_it() {
        let window = Size::new(760.0, 320.0);
        let mut outside = Vec::new();
        // A diagram wider than its card is there to show that it scrolls sideways.
        for page in pages::PAGES.iter().filter(|page| page.id != "diagram") {
            let state = State::starting_at(page.id);
            let cx = pages::Cx { state: &state, palette: state.theme().palette(), width: window.width - 48.0 };
            for (section, variants) in page.variant_counts(&cx).into_iter().enumerate() {
                for variant in 0..variants {
                    let mut state = State::starting_at(page.id);
                    for (key, value) in [("embed", "on".to_owned()), ("embed-section", section.to_string()), ("embed-variant", variant.to_string())] {
                        state.set(key, &value);
                    }
                    let frame = UiTree::new(view(&state, window)).layout(window);
                    let stray = frame.commands.iter().any(|command| match command {
                        limn::DrawCommand::Text { position, .. } => position.x < 0.0 || position.x > window.width,
                        _ => false,
                    });
                    if stray {
                        outside.push(format!("{} section {section} variant {variant}", page.id));
                    }
                }
            }
        }
        assert!(outside.is_empty(), "text drawn outside the card:\n{}", outside.join("\n"));
    }

    /// The menu at a point opens where the secondary click was, and against
    /// its area with Enter or the menu key.
    #[test]
    fn the_menu_at_a_point_opens_from_the_pointer_and_the_keyboard() {
        let window = Size::new(1280.0, 800.0);
        let area = "overlay-point=area";
        let shown = |state: &State| {
            let frame = UiTree::new(view(state, window)).layout(window);
            frame
                .regions
                .iter()
                .any(|region| region.id == "overlay-items-notify")
        };
        let start = || {
            let state = State::starting_at("context-menu");
            let mut tree = UiTree::new(view(&state, window));
            tree.layout(window);
            (state, tree)
        };
        let (mut state, tree) = start();
        assert!(!shown(&state));
        let rect = tree
            .frame()
            .regions
            .iter()
            .find(|region| region.id == area)
            .unwrap()
            .rect;
        let inside = limn::Point::new(rect.x + 10.0, rect.y + 10.0);
        assert!(pages::context_menu(
            &mut state,
            &tree,
            &UiEvent::ContextMenu { position: inside }
        ));
        assert_eq!(
            state.text("overlay-point"),
            format!("{},{}", inside.x, inside.y)
        );
        assert!(shown(&state));
        let outside = limn::Point::new(rect.x - 10.0, rect.y - 10.0);
        assert!(!pages::context_menu(
            &mut start().0,
            &tree,
            &UiEvent::ContextMenu { position: outside }
        ));
        // Enter on the area, as a button, and the menu key open it against the area.
        let (mut state, mut tree) = start();
        assert!(tree.focus(area));
        for action in tree.handle_event(UiEvent::Key {
            key: limn::Key::Enter,
            shift: false,
            ctrl: false,
        }) {
            update(&mut state, &action);
        }
        assert_eq!(state.text("overlay-point"), "area");
        assert!(shown(&state));
        let (mut state, mut tree) = start();
        assert!(!pages::context_menu(
            &mut state,
            &tree,
            &UiEvent::ContextMenuKeyboard
        ));
        assert!(tree.focus(area));
        assert!(pages::context_menu(
            &mut state,
            &tree,
            &UiEvent::ContextMenuKeyboard
        ));
        assert!(shown(&state));
    }

    #[test]
    fn page_ids_are_unique_and_selectable() {
        let mut seen = HashSet::new();
        for page in pages::PAGES {
            assert!(seen.insert(page.id), "duplicate page id {}", page.id);
            let mut state = State::default();
            update(&mut state, &UiAction::Clicked { id: format!("page={}", page.id) });
            assert_eq!(pages::find(state.choice("page", "")).id, page.id);
        }
    }

    #[test]
    fn clicks_flip_flags_and_choose_within_groups() {
        let mut state = State::default();
        let click = |id: &str| UiAction::Clicked { id: id.into() };
        update(&mut state, &click("demo-switch"));
        assert!(state.flag("demo-switch"));
        update(&mut state, &click("demo-switch"));
        assert!(!state.flag("demo-switch"));
        update(&mut state, &click("demo-tabs=two"));
        assert_eq!(state.choice("demo-tabs", "one"), "two");
        // What a restart restores is exactly what was there, awkward text included.
        state.set("notes", "tab\there\nnew line \\ backslash ñ 東京");
        let restored = State::deserialize(&state.serialize());
        assert_eq!(restored.serialize(), state.serialize());
        assert_eq!(restored.text("notes"), "tab\there\nnew line \\ backslash ñ 東京");
        assert!(State::deserialize("no tab on this line\n").serialize().is_empty());

        update(&mut state, &click("language=rust&list=closed"));
        assert_eq!((state.text("language"), state.text("list")), ("rust", "closed"));
        update(&mut state, &UiAction::InputChanged { id: "name".into(), value: "ñ".into() });
        assert_eq!(state.text("name"), "ñ");
    }
}
