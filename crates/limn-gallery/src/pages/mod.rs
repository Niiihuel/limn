//! One page per component. A module draws the sections of one component or
//! of several that are shown together; [`PAGES`] gives each component its
//! page. To add one: write its sections in a module, as
//! `fn view(&Cx) -> Vec<Section>`, and list the component in [`PAGES`].

use crate::app::State;
use limn::{ComponentPalette, Element, ElementKind, FontFamily, UiAction};

mod blocks;
mod buttons;
mod chrome;
mod context;
mod diagrams;
mod editor;
mod empty;
mod feedback;
pub mod glass;
mod inputs;
mod lists;
mod markdown;
mod menus;
mod messages;
mod navigation;
mod overlays;
mod page;
mod panels;
mod preferences;
mod scheme;
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
    Page { id: "tokens", group: "Sistema", title: "Tokens", source: "tokens", view: tokens::view, sections: &[] },
    Page { id: "scheme", group: "Sistema", title: "Scheme", source: "scheme", view: scheme::view, sections: &[] },
    Page { id: "text", group: "Sistema", title: "Text", source: "typography", view: typography::view, sections: &["Escala", "Fuera de la escala", "Tinta y familia", "Pesos", "Texto largo", "Fuera de foco"] },
    Page { id: "separator", group: "Sistema", title: "Separator", source: "typography", view: typography::view, sections: &["Separador"] },
    Page { id: "icon", group: "Sistema", title: "Icon", source: "navigation", view: navigation::view, sections: &["Icono del elemento elegido"] },
    Page { id: "button", group: "Acciones", title: "Button", source: "buttons", view: buttons::view, sections: &["Variantes", "Deshabilitados", "Cargando", "Tamaños", "Perfil de la aplicación", "Con icono", "Barra de herramientas", "Tinta, separación y radio", "Activo"] },
    Page { id: "button-group", group: "Acciones", title: "ButtonGroup", source: "buttons", view: buttons::view, sections: &["Grupo de botones"] },
    Page { id: "clipboard", group: "Acciones", title: "Clipboard", source: "buttons", view: buttons::view, sections: &["Portapapeles"] },
    Page { id: "link", group: "Acciones", title: "Link", source: "navigation", view: navigation::view, sections: &["Enlaces"] },
    Page { id: "kbd", group: "Acciones", title: "Kbd", source: "menus", view: menus::view, sections: &["Teclas con leyenda"] },
    Page { id: "message-actions", group: "Acciones", title: "MessageActions", source: "messages", view: messages::view, sections: &["Mensaje del asistente", "Mensaje del usuario", "Conmutadores y ramas"] },
    Page { id: "text-field", group: "Entradas", title: "TextField", source: "inputs", view: inputs::view, sections: &["Campo de texto"] },
    Page { id: "form-field", group: "Entradas", title: "FormField", source: "inputs", view: inputs::view, sections: &["Campo de formulario"] },
    Page { id: "text-area", group: "Entradas", title: "TextArea", source: "inputs", view: inputs::view, sections: &["Área de texto"] },
    Page { id: "search-field", group: "Entradas", title: "SearchField", source: "inputs", view: inputs::view, sections: &["Búsqueda"] },
    Page { id: "select", group: "Entradas", title: "Select", source: "inputs", view: inputs::view, sections: &["Selector"] },
    Page { id: "switch", group: "Entradas", title: "Switch", source: "inputs", view: inputs::view, sections: &["Interruptor"] },
    Page { id: "slider", group: "Entradas", title: "Slider", source: "inputs", view: inputs::view, sections: &["Deslizador"] },
    Page { id: "editor", group: "Entradas", title: "Editor", source: "editor", view: editor::view, sections: &[] },
    Page { id: "tabs", group: "Navegación", title: "Tabs", source: "navigation", view: navigation::view, sections: &["Pestañas", "Pestañas de recursos"] },
    Page { id: "segmented-control", group: "Navegación", title: "SegmentedControl", source: "navigation", view: navigation::view, sections: &["Control segmentado"] },
    Page { id: "stepper", group: "Navegación", title: "Stepper", source: "navigation", view: navigation::view, sections: &["Pasos"] },
    Page { id: "breadcrumb", group: "Navegación", title: "Breadcrumb", source: "navigation", view: navigation::view, sections: &["Ruta"] },
    Page { id: "collapsible", group: "Navegación", title: "Collapsible", source: "navigation", view: navigation::view, sections: &["Plegable"] },
    Page { id: "chat-minimap", group: "Navegación", title: "ChatMinimap", source: "messages", view: messages::view, sections: &["Minimapa de la conversación"] },
    Page { id: "pager", group: "Navegación", title: "Pager", source: "panels", view: panels::view, sections: &["Paginador"] },
    Page { id: "menu-bar", group: "Navegación", title: "MenuBar", source: "chrome", view: chrome::view, sections: &["Barra de menús"] },
    Page { id: "status-bar", group: "Navegación", title: "StatusBar", source: "chrome", view: chrome::view, sections: &["Barra de estado"] },
    Page { id: "glass", group: "Navegación", title: "NavigationBar", source: "glass", view: glass::view, sections: &[] },
    Page { id: "list-row", group: "Datos", title: "ListRow", source: "lists", view: lists::view, sections: &[] },
    Page { id: "table", group: "Datos", title: "Table", source: "table", view: table::view, sections: &[] },
    Page { id: "markdown", group: "Datos", title: "Markdown", source: "markdown", view: markdown::view, sections: &[] },
    Page { id: "diagram", group: "Datos", title: "Diagram", source: "diagrams", view: diagrams::view, sections: &[] },
    Page { id: "code-block", group: "Datos", title: "CodeBlock", source: "blocks", view: blocks::view, sections: &["Bloque de código"] },
    Page { id: "output-block", group: "Datos", title: "OutputBlock", source: "blocks", view: blocks::view, sections: &["Bloque de salida"] },
    Page { id: "card", group: "Estructura", title: "Card", source: "blocks", view: blocks::view, sections: &["Tarjeta"] },
    Page { id: "media-tile", group: "Estructura", title: "MediaTile", source: "blocks", view: blocks::view, sections: &["Mosaico de medios"] },
    Page { id: "media-card", group: "Estructura", title: "MediaCard", source: "blocks", view: blocks::view, sections: &["Tarjeta de pista"] },
    Page { id: "page-header", group: "Estructura", title: "PageHeader", source: "page", view: page::view, sections: &["Encabezado de página"] },
    Page { id: "section-header", group: "Estructura", title: "SectionHeader", source: "page", view: page::view, sections: &["Encabezados de sección"] },
    Page { id: "page", group: "Estructura", title: "Page", source: "page", view: page::view, sections: &["Rejilla de tarjetas", "Columna de página"] },
    Page { id: "panel-header", group: "Estructura", title: "PanelHeader", source: "panels", view: panels::view, sections: &["Cabecera de panel", "Volver, cerrar y título largo", "Un campo en lugar del título", "Un tinte detrás del encabezado"] },
    Page { id: "sidebar-panel", group: "Estructura", title: "SidebarPanel", source: "panels", view: panels::view, sections: &["Panel lateral"] },
    Page { id: "resize-handle", group: "Estructura", title: "ResizeHandle", source: "panels", view: panels::view, sections: &["Tirador entre paneles"] },
    Page { id: "preferences-group", group: "Estructura", title: "PreferencesGroup", source: "preferences", view: preferences::view, sections: &[] },
    Page { id: "empty", group: "Estados", title: "Empty", source: "empty", view: empty::view, sections: &["Con icono y acción", "Dos acciones", "Compacto", "Enmarcado", "Sólo título"] },
    Page { id: "notice", group: "Estados", title: "Notice", source: "empty", view: empty::view, sections: &["Aviso en línea", "Aviso apilado", "Aviso compacto", "Aviso de alto fijo"] },
    Page { id: "alert", group: "Estados", title: "Alert", source: "feedback", view: feedback::view, sections: &["Avisos", "Avisos con acción"] },
    Page { id: "progress", group: "Estados", title: "Progress", source: "feedback", view: feedback::view, sections: &["Progreso"] },
    Page { id: "spinner", group: "Estados", title: "Spinner", source: "feedback", view: feedback::view, sections: &["Carga"] },
    Page { id: "marker", group: "Estados", title: "Marker", source: "feedback", view: feedback::view, sections: &["Marcadores"] },
    Page { id: "ticker", group: "Estados", title: "Ticker", source: "feedback", view: feedback::view, sections: &["Teletipo"] },
    Page { id: "collapse", group: "Estados", title: "Collapse", source: "feedback", view: feedback::view, sections: &["Hueco que se cierra"] },
    Page { id: "chip", group: "Estados", title: "Chip", source: "feedback", view: feedback::view, sections: &["Etiquetas"] },
    Page { id: "context-window", group: "Estados", title: "ContextWindow", source: "context", view: context::view, sections: &[] },
    Page { id: "popover", group: "Superposiciones", title: "Popover", source: "overlays", view: overlays::view, sections: &["Popover"] },
    Page { id: "dropdown-list", group: "Superposiciones", title: "DropdownList", source: "overlays", view: overlays::view, sections: &["Lista desplegable"] },
    Page { id: "dropdown-menu", group: "Superposiciones", title: "DropdownMenu", source: "overlays", view: overlays::view, sections: &["Menú"] },
    Page { id: "context-menu", group: "Superposiciones", title: "ContextMenu", source: "overlays", view: overlays::view, sections: &["Menú contextual"] },
    Page { id: "dialog", group: "Superposiciones", title: "Dialog", source: "overlays", view: overlays::view, sections: &["Diálogos"] },
    Page { id: "toast", group: "Superposiciones", title: "Toast", source: "overlays", view: overlays::view, sections: &["Toast"] },
    Page { id: "picker", group: "Superposiciones", title: "Picker", source: "menus", view: menus::view, sections: &["Selector y catálogo", "Filas de menú", "Panel en popover"] },
    Page { id: "suggestion-list", group: "Superposiciones", title: "SuggestionList", source: "menus", view: menus::view, sections: &["Lista de sugerencias"] },
    Page { id: "tooltip", group: "Superposiciones", title: "Tooltip", source: "tooltip", view: tooltip::view, sections: &[] },
    Page { id: "tour", group: "Superposiciones", title: "Tour", source: "tour", view: tour::view, sections: &[] },
];

/// The page with this ID, or the first one for an ID that no longer exists
/// (a hot patch may remove the page being shown).
pub fn find(id: &str) -> &'static Page {
    PAGES.iter().find(|page| page.id == id).unwrap_or(&PAGES[0])
}

/// What a demo keeps that a flag or a choice cannot hold.
pub fn update(state: &mut State, action: &UiAction) {
    table::update(state, action);
}

/// Wrap the window in the layers the pages control.
pub fn layers(cx: &Cx, base: Element) -> Element {
    tour::layer(cx, overlays::layers(cx, base))
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
    /// demo with the captions that follow it being one variant.
    fn showcase(self, cx: &Cx) -> Element {
        let mut variants: Vec<Vec<Element>> = Vec::new();
        for demo in self.demos {
            match variants.last_mut() {
                // A caption before any demo opens the first variant.
                Some(variant) if is_caption(&demo) || variant.iter().all(is_caption) => variant.push(demo),
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
        for page in PAGES {
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
                assert!(
                    section.title.is_empty() || seen.contains(section.title.as_str()),
                    "{}.rs: section {:?} is on no page",
                    page.source,
                    section.title
                );
            }
        }
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
