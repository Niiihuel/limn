//! One module per page. To add a page: write `fn view(&Cx) -> Element` in a
//! new module and list it in [`PAGES`].

use crate::app::State;
use limn::{ComponentPalette, Element, FontFamily};

mod blocks;
mod buttons;
mod chrome;
mod context;
mod diagrams;
mod editor;
mod empty;
mod feedback;
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
mod tokens;
mod tooltip;
mod tour;
mod typography;

pub struct Page {
    pub id: &'static str,
    pub group: &'static str,
    pub title: &'static str,
    pub view: fn(&Cx) -> Element,
}

pub const PAGES: &[Page] = &[
    Page { id: "tokens", group: "Sistema", title: "Forma y movimiento", view: tokens::view },
    Page { id: "scheme", group: "Sistema", title: "Esquema de color", view: scheme::view },
    Page { id: "typography", group: "Sistema", title: "Texto y separadores", view: typography::view },
    Page { id: "buttons", group: "Controles", title: "Botones", view: buttons::view },
    Page { id: "inputs", group: "Controles", title: "Entradas", view: inputs::view },
    Page { id: "navigation", group: "Controles", title: "Navegación", view: navigation::view },
    Page { id: "lists", group: "Controles", title: "Listas", view: lists::view },
    Page { id: "preferences", group: "Controles", title: "Preferencias", view: preferences::view },
    Page { id: "page", group: "Controles", title: "Página y secciones", view: page::view },
    Page { id: "panels", group: "Controles", title: "Paneles y paginador", view: panels::view },
    Page { id: "blocks", group: "Controles", title: "Tarjetas y bloques", view: blocks::view },
    Page { id: "markdown", group: "Controles", title: "Markdown", view: markdown::view },
    Page { id: "diagrams", group: "Controles", title: "Diagramas", view: diagrams::view },
    Page { id: "messages", group: "Controles", title: "Acciones de mensaje", view: messages::view },
    Page { id: "chrome", group: "Controles", title: "Barras de menús y de estado", view: chrome::view },
    Page { id: "editor", group: "Controles", title: "Editor y compositor", view: editor::view },
    Page { id: "empty", group: "Estados", title: "Estado vacío", view: empty::view },
    Page { id: "feedback", group: "Estados", title: "Avisos y carga", view: feedback::view },
    Page { id: "context", group: "Estados", title: "Ventana de contexto", view: context::view },
    Page { id: "overlays", group: "Superposiciones", title: "Menús y diálogos", view: overlays::view },
    Page { id: "menus", group: "Superposiciones", title: "Selectores y sugerencias", view: menus::view },
    Page { id: "tooltip", group: "Superposiciones", title: "Tooltip", view: tooltip::view },
    Page { id: "tour", group: "Superposiciones", title: "Recorrido guiado", view: tour::view },
];

/// The page with this ID, or the first one for an ID that no longer exists
/// (a hot patch may remove the page being shown).
pub fn find(id: &str) -> &'static Page {
    PAGES.iter().find(|page| page.id == id).unwrap_or(&PAGES[0])
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
        .font_size(12.0)
        .foreground(cx.palette.muted)
}

/// A titled group of demos.
pub fn section(cx: &Cx, title: &str, demos: Vec<Element>) -> Element {
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
