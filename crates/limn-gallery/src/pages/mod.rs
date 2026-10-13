//! One page per component. A module draws the sections of one component or
//! of several that are shown together; [`PAGES`] gives each component its
//! page. To add one: write its sections in a module, as
//! `fn view(&Cx) -> Vec<Section>`, and list the component in [`PAGES`].

use crate::app::State;
use limn::{ComponentPalette, Element, ElementKind, FontFamily, UiAction};

mod blocks;
mod buttons;
mod charts;
mod chrome;
mod code;
mod context;
mod dates;
mod diagrams;
mod editor;
mod empty;
mod feedback;
pub mod glass;
mod inputs;
mod lists;
mod loading;
mod markdown;
mod media;
mod menus;
mod messages;
mod navigation;
mod overlays;
mod page;
mod panels;
mod preferences;
mod scheme;
mod surfaces;
mod table;
mod tokens;
mod tooltip;
mod tour;
mod typography;

/// One page for each component. `title` is the name of its type, and `id`
/// that name in kebab-case: the address of its page on the site and the name
/// of its guide, `docs/components/<id>.md`.
pub struct Page {
    pub id: &'static str,
    pub group: &'static str,
    pub title: &'static str,
    /// The module in this directory that draws it. Only `--pages` reads it.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub source: &'static str,
    /// Every section of that module, some of them this page's.
    pub view: fn(&Cx) -> Vec<Section>,
    /// The titles of this page's sections; none means the whole module.
    pub sections: &'static [&'static str],
}

impl Page {
    /// This page's sections, one under another.
    pub fn render(&self, cx: &Cx) -> Element {
        let mut sections: Vec<Section> = (self.view)(cx)
            .into_iter()
            .filter(|section| self.sections.is_empty() || self.sections.contains(&section.title.as_str()))
            .collect();
        // Embedded under the component's name, a lone section's title repeats it.
        if cx.state.flag("embed") && sections.len() == 1 {
            sections[0].title.clear();
        }
        Element::column(sections.into_iter().map(|section| section.render(cx)).collect()).gap(32.0)
    }
}

pub const PAGES: &[Page] = &[
    Page { id: "tokens", group: "System", title: "Tokens", source: "tokens", view: tokens::view, sections: &[] },
    Page { id: "scheme", group: "System", title: "Scheme", source: "scheme", view: scheme::view, sections: &[] },
    Page { id: "text", group: "System", title: "Text", source: "typography", view: typography::view, sections: &["Scale", "Outside the scale", "Ink and family", "Weights", "Long text", "Soft focus"] },
    Page { id: "separator", group: "System", title: "Separator", source: "typography", view: typography::view, sections: &["Separator"] },
    Page { id: "icon", group: "System", title: "Icon", source: "navigation", view: navigation::view, sections: &["Selected item icon"] },
    Page { id: "button", group: "Actions", title: "Button", source: "buttons", view: buttons::view, sections: &["Variants", "Disabled", "Loading", "Sizes", "Application sizing", "With icon", "Toolbar", "Ink, spacing and radius", "Active"] },
    Page { id: "button-group", group: "Actions", title: "ButtonGroup", source: "buttons", view: buttons::view, sections: &["Button group"] },
    Page { id: "clipboard", group: "Actions", title: "Clipboard", source: "buttons", view: buttons::view, sections: &["Clipboard"] },
    Page { id: "link", group: "Actions", title: "Link", source: "navigation", view: navigation::view, sections: &["Links"] },
    Page { id: "kbd", group: "Actions", title: "Kbd", source: "menus", view: menus::view, sections: &["Keys with captions"] },
    Page { id: "message-actions", group: "Actions", title: "MessageActions", source: "messages", view: messages::view, sections: &["Assistant message", "User message", "Toggles and branches"] },
    Page { id: "text-field", group: "Inputs", title: "TextField", source: "inputs", view: inputs::view, sections: &["Text field"] },
    Page { id: "form-field", group: "Inputs", title: "FormField", source: "inputs", view: inputs::view, sections: &["Form field"] },
    Page { id: "text-area", group: "Inputs", title: "TextArea", source: "inputs", view: inputs::view, sections: &["Text area"] },
    Page { id: "search-field", group: "Inputs", title: "SearchField", source: "inputs", view: inputs::view, sections: &["Search"] },
    Page { id: "select", group: "Inputs", title: "Select", source: "inputs", view: inputs::view, sections: &["Select"] },
    Page { id: "switch", group: "Inputs", title: "Switch", source: "inputs", view: inputs::view, sections: &["Switch"] },
    Page { id: "slider", group: "Inputs", title: "Slider", source: "inputs", view: inputs::view, sections: &["Slider"] },
    Page { id: "date-picker", group: "Inputs", title: "DatePicker", source: "dates", view: dates::view, sections: &["Date picker"] },
    Page { id: "time-picker", group: "Inputs", title: "TimePicker", source: "dates", view: dates::view, sections: &["Time picker"] },
    Page { id: "date-time-picker", group: "Inputs", title: "DateTimePicker", source: "dates", view: dates::view, sections: &["Date and time picker"] },
    Page { id: "editor", group: "Inputs", title: "Editing tools", source: "editor", view: editor::view, sections: &[] },
    Page { id: "tabs", group: "Navigation", title: "Tabs", source: "navigation", view: navigation::view, sections: &["Tabs", "Resource tabs"] },
    Page { id: "segmented-control", group: "Navigation", title: "SegmentedControl", source: "navigation", view: navigation::view, sections: &["Segmented control"] },
    Page { id: "stepper", group: "Navigation", title: "Stepper", source: "navigation", view: navigation::view, sections: &["Steps"] },
    Page { id: "breadcrumb", group: "Navigation", title: "Breadcrumb", source: "navigation", view: navigation::view, sections: &["Path"] },
    Page { id: "collapsible", group: "Navigation", title: "Collapsible", source: "navigation", view: navigation::view, sections: &["Collapsible"] },
    Page { id: "chat-minimap", group: "Navigation", title: "ChatMinimap", source: "messages", view: messages::view, sections: &["Conversation minimap"] },
    Page { id: "pager", group: "Navigation", title: "Pager", source: "panels", view: panels::view, sections: &["Pager"] },
    Page { id: "menu-bar", group: "Navigation", title: "MenuBar", source: "chrome", view: chrome::view, sections: &["Menu bar"] },
    Page { id: "status-bar", group: "Navigation", title: "StatusBar", source: "chrome", view: chrome::view, sections: &["Status bar"] },
    Page { id: "glass", group: "Navigation", title: "NavigationBar", source: "glass", view: glass::view, sections: &[] },
    Page { id: "list-row", group: "Data", title: "ListRow", source: "lists", view: lists::view, sections: &[] },
    Page { id: "tree-row", group: "Data", title: "TreeRow", source: "surfaces", view: surfaces::view, sections: &["Tree rows"] },
    Page { id: "table", group: "Data", title: "Table", source: "table", view: table::view, sections: &[] },
    Page { id: "markdown", group: "Data", title: "Markdown", source: "markdown", view: markdown::view, sections: &[] },
    Page { id: "diagram", group: "Data", title: "Diagram", source: "diagrams", view: diagrams::view, sections: &[] },
    Page { id: "code-block", group: "Data", title: "CodeBlock", source: "blocks", view: blocks::view, sections: &["Code block"] },
    Page { id: "output-block", group: "Data", title: "OutputBlock", source: "blocks", view: blocks::view, sections: &["Output block"] },
    Page { id: "code-editor", group: "Data", title: "CodeEditor", source: "code", view: code::view, sections: &["Code editor"] },
    Page { id: "code-diff", group: "Data", title: "CodeDiff", source: "code", view: code::view, sections: &["Code diff"] },
    Page { id: "charts", group: "Data", title: "Charts", source: "charts", view: charts::view, sections: &[] },
    Page { id: "attachment", group: "Data", title: "Attachment", source: "media", view: media::view, sections: &["Attachment"] },
    Page { id: "card", group: "Layout", title: "Card", source: "blocks", view: blocks::view, sections: &["Card"] },
    Page { id: "framed-card", group: "Layout", title: "FramedCard", source: "surfaces", view: surfaces::view, sections: &["Framed card"] },
    Page { id: "scroll-area", group: "Layout", title: "ScrollArea", source: "surfaces", view: surfaces::view, sections: &["Scroll area"] },
    Page { id: "media-tile", group: "Layout", title: "MediaTile", source: "blocks", view: blocks::view, sections: &["Media tile"] },
    Page { id: "media-card", group: "Layout", title: "MediaCard", source: "blocks", view: blocks::view, sections: &["Track card"] },
    Page { id: "page-header", group: "Layout", title: "PageHeader", source: "page", view: page::view, sections: &["Page header"] },
    Page { id: "section-header", group: "Layout", title: "SectionHeader", source: "page", view: page::view, sections: &["Section headers"] },
    Page { id: "page", group: "Layout", title: "PageColumn / CardGrid", source: "page", view: page::view, sections: &["Card grid", "Page column"] },
    Page { id: "panel-header", group: "Layout", title: "PanelHeader", source: "panels", view: panels::view, sections: &["Panel header", "Back, close, and long title", "A field in place of the title", "A tint behind the header"] },
    Page { id: "sidebar-panel", group: "Layout", title: "SidebarPanel", source: "panels", view: panels::view, sections: &["Sidebar panel"] },
    Page { id: "resize-handle", group: "Layout", title: "ResizeHandle", source: "panels", view: panels::view, sections: &["Resize handle between panels"] },
    Page { id: "preferences-group", group: "Layout", title: "PreferencesGroup", source: "preferences", view: preferences::view, sections: &[] },
    Page { id: "empty", group: "States", title: "Empty", source: "empty", view: empty::view, sections: &["With icon and action", "Two actions", "Compact", "Framed", "Title only"] },
    Page { id: "notice", group: "States", title: "Notice", source: "empty", view: empty::view, sections: &["Inline notice", "Stacked notice", "Compact notice", "Fixed-height notice"] },
    Page { id: "alert", group: "States", title: "Alert", source: "feedback", view: feedback::view, sections: &["Alerts", "Alerts with actions"] },
    Page { id: "progress", group: "States", title: "Progress", source: "feedback", view: feedback::view, sections: &["Progress"] },
    Page { id: "spinner", group: "States", title: "Spinner", source: "feedback", view: feedback::view, sections: &["Loading"] },
    Page { id: "skeleton", group: "States", title: "Skeleton", source: "loading", view: loading::view, sections: &["Skeleton"] },
    Page { id: "text-shimmer", group: "States", title: "TextShimmer", source: "loading", view: loading::view, sections: &["Text shimmer"] },
    Page { id: "marker", group: "States", title: "Marker", source: "feedback", view: feedback::view, sections: &["Markers"] },
    Page { id: "ticker", group: "States", title: "Ticker", source: "feedback", view: feedback::view, sections: &["Ticker"] },
    Page { id: "collapse", group: "States", title: "Collapse", source: "feedback", view: feedback::view, sections: &["Closing slot"] },
    Page { id: "chip", group: "States", title: "Chip", source: "feedback", view: feedback::view, sections: &["Labels"] },
    Page { id: "avatar", group: "States", title: "Avatar", source: "media", view: media::view, sections: &["Avatar"] },
    Page { id: "context-window", group: "States", title: "ContextWindow", source: "context", view: context::view, sections: &[] },
    Page { id: "popover", group: "Overlays", title: "Popover", source: "overlays", view: overlays::view, sections: &["Popover"] },
    Page { id: "dropdown-list", group: "Overlays", title: "DropdownList", source: "overlays", view: overlays::view, sections: &["Dropdown list"] },
    Page { id: "dropdown-menu", group: "Overlays", title: "DropdownMenu", source: "overlays", view: overlays::view, sections: &["Menu"] },
    Page { id: "context-menu", group: "Overlays", title: "ContextMenu", source: "overlays", view: overlays::view, sections: &["Context menu"] },
    Page { id: "dialog", group: "Overlays", title: "Modal / AlertDialog", source: "overlays", view: overlays::view, sections: &["Dialogs"] },
    Page { id: "toast", group: "Overlays", title: "Toast", source: "overlays", view: overlays::view, sections: &["Toast"] },
    Page { id: "image-viewer", group: "Overlays", title: "ImageViewer", source: "media", view: media::view, sections: &["Image viewer"] },
    Page { id: "picker", group: "Overlays", title: "Catalog menus", source: "menus", view: menus::view, sections: &["Select and catalog", "Menu rows", "Popover panel"] },
    Page { id: "suggestion-list", group: "Overlays", title: "SuggestionList", source: "menus", view: menus::view, sections: &["Suggestion list"] },
    Page { id: "tooltip", group: "Overlays", title: "Tooltip", source: "tooltip", view: tooltip::view, sections: &[] },
    Page { id: "tour", group: "Overlays", title: "Tour", source: "tour", view: tour::view, sections: &[] },
];

/// The components as JSON, one page each: what a site needs to give every
/// component its address. `site/src/data/pages.json` is this, kept by a test.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn pages_json() -> String {
    let quote = |text: &str| format!("\"{}\"", text.replace('"', "\\\""));
    let pages: Vec<String> = PAGES
        .iter()
        .map(|page| {
            format!(
                "  {{\"id\": {}, \"group\": {}, \"title\": {}, \"source\": {}}}",
                quote(page.id),
                quote(page.group),
                quote(page.title),
                quote(page.source),
            )
        })
        .collect();
    format!("[\n{}\n]\n", pages.join(",\n"))
}

/// The page with this ID, or the first one for an ID that no longer exists
/// (a hot patch may remove the page being shown).
pub fn find(id: &str) -> &'static Page {
    PAGES.iter().find(|page| page.id == id).unwrap_or(&PAGES[0])
}

/// What a demo keeps that a flag or a choice cannot hold.
pub fn update(state: &mut State, action: &UiAction) {
    table::update(state, action);
    dates::update(state, action);
}

/// Wrap the window in the layers the pages control.
pub fn layers(cx: &Cx, base: Element) -> Element {
    tour::layer(cx, overlays::layers(cx, media::layer(cx, base)))
}

/// What a page needs to draw itself.
pub struct Cx<'a> {
    pub state: &'a State,
    pub palette: ComponentPalette,
    /// Width available to the page's content.
    pub width: f32,
}

pub fn caption(cx: &Cx, text: &str) -> Element {
    Element::text(text)
        .font_family(FontFamily::Sans)
        .font_size(CAPTION_SIZE)
        .foreground(cx.palette.muted)
}

/// A titled group of demos: one aspect of a component, or one component of
/// several a module draws.
pub struct Section {
    pub title: String,
    pub demos: Vec<Element>,
}

impl Section {
    fn render(self, cx: &Cx) -> Element {
        if cx.state.flag("embed") {
            return self.showcase(cx);
        }
        let body = Element::column(self.demos).gap(12.0);
        if self.title.is_empty() {
            body
        } else {
            titled(cx, &self.title, vec![body])
        }
    }

    /// As a site shows it: each variant apart from the next and centered, a
    /// demo with its captions being one variant.
    fn showcase(self, cx: &Cx) -> Element {
        // A section says each demo's caption before it or after it; the first
        // item tells which.
        let leading = self.demos.first().is_some_and(is_caption);
        let mut variants: Vec<Vec<Element>> = Vec::new();
        for demo in self.demos {
            let caption = is_caption(&demo);
            match variants.last_mut() {
                // After it: a caption joins the demo before it.
                Some(variant) if !leading && caption => variant.push(demo),
                // Before it: a demo joins the captions that announced it, and
                // a caption that follows a demo opens the next variant.
                Some(variant) if leading && (!caption || variant.iter().all(is_caption)) => variant.push(demo),
                _ => variants.push(vec![demo]),
            }
        }
        let mut rows = Vec::new();
        if !self.title.is_empty() {
            rows.push(titled(cx, &self.title, Vec::new()).centered_x());
        }
        for (index, variant) in variants.into_iter().enumerate() {
            if index > 0 {
                rows.push(Element::panel(Element::spacer()).background(cx.palette.border).size(None, Some(1.0)));
            }
            rows.push(Element::column(variant.into_iter().map(|demo| natural(demo).centered_x()).collect()).gap(10.0));
        }
        Element::column(rows).gap(20.0)
    }
}

const CAPTION_SIZE: f32 = 12.0;

fn is_caption(element: &Element) -> bool {
    matches!(element.kind, ElementKind::Text(_)) && element.style.font_size == CAPTION_SIZE
}

/// A `row` without the spacer that keeps it at the left, so it can be centered.
fn natural(mut element: Element) -> Element {
    if let ElementKind::Row(children) = &mut element.kind {
        if matches!(children.last().map(|child| &child.kind), Some(ElementKind::Spacer)) {
            children.pop();
        }
    }
    element
}

/// What a module shows without a title of its own.
impl From<Element> for Section {
    fn from(demo: Element) -> Self {
        Section { title: String::new(), demos: vec![demo] }
    }
}

pub fn section(_cx: &Cx, title: &str, demos: Vec<Element>) -> Section {
    Section { title: title.to_owned(), demos }
}

/// Demos under a title, as one element.
pub fn titled(cx: &Cx, title: &str, demos: Vec<Element>) -> Element {
    let mut children = vec![Element::text(title)
        .font_family(FontFamily::Sans)
        .font_size(15.0)
        .foreground(cx.palette.text)];
    children.extend(demos);
    Element::column(children).gap(12.0)
}

/// Demos side by side at their natural widths.
pub fn row(children: Vec<Element>) -> Element {
    let mut children = children;
    children.push(Element::spacer().flex(1.0));
    Element::row(children).gap(12.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn every_section_of_a_module_is_on_a_page_and_every_page_names_real_sections() {
        let state = State::default();
        let cx = Cx { state: &state, palette: ComponentPalette::neutral_dark(), width: 800.0 };
        let mut claimed: HashMap<&str, Option<HashSet<&str>>> = HashMap::new();
        // The name of a module and its function go together: the site links to the one and shows the other.
        let mut views: HashMap<&str, usize> = HashMap::new();
        for page in PAGES {
            let view = page.view as usize;
            assert_eq!(*views.entry(page.source).or_insert(view), view, "{}: {}.rs is not its view", page.id, page.source);
            let titles: Vec<String> = (page.view)(&cx).into_iter().map(|section| section.title).collect();
            for wanted in page.sections {
                assert!(titles.iter().any(|title| title == wanted), "{}: no section {wanted:?} in {}.rs", page.id, page.source);
            }
            // A page without a list takes the whole module.
            let all = claimed.entry(page.source).or_insert_with(|| Some(HashSet::new()));
            match (page.sections.is_empty(), all.as_mut()) {
                (true, _) => *all = None,
                (false, Some(seen)) => seen.extend(page.sections),
                (false, None) => {}
            }
        }
        for page in PAGES {
            let Some(Some(seen)) = claimed.get(page.source) else { continue };
            for section in (page.view)(&cx) {
                // Without a title a section belongs to no page that names its sections.
                assert!(
                    seen.contains(section.title.as_str()),
                    "{}.rs: section {:?} is on no page",
                    page.source,
                    section.title
                );
            }
        }
    }

    /// The site is built without the gallery, from the list it last printed.
    #[test]
    fn the_site_has_the_list_of_pages_as_it_is() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../site/src/data/pages.json");
        assert_eq!(
            std::fs::read_to_string(path).unwrap_or_default(),
            pages_json(),
            "run `cargo run -p limn-gallery -- --pages > site/src/data/pages.json`"
        );
    }

    /// The convention of a component's guide: it is named after a page, and
    /// it says how to import the component and how to use it.
    #[test]
    fn every_component_guide_belongs_to_a_page_and_follows_the_outline() {
        let guides = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/components");
        let Ok(guides) = std::fs::read_dir(guides) else { return };
        for guide in guides.flatten() {
            let path = guide.path();
            let id = path.file_stem().unwrap().to_string_lossy().into_owned();
            let page = PAGES.iter().find(|page| page.id == id).unwrap_or_else(|| panic!("docs/components/{id}.md has no page"));
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.starts_with(&format!("# {}\n", page.title)), "{id}.md must start with `# {}`", page.title);
            for heading in ["\n## Import\n", "\n## Usage\n"] {
                assert!(text.contains(heading), "{id}.md needs {}", heading.trim());
            }
        }
    }
}
