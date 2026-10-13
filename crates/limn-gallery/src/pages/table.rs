use super::{caption, section, Cx};
use crate::app::State;
use limn::{
    ButtonVariant, Component, ComponentPalette, ControlSize, Element, Empty, Icon, IconButton, RadiusRole,
    ScrollArea, ScrollbarAppearance, SearchField, Table, TableColumn, TableRow, TableSizing, TableSort, Text,
    TextRole, UiAction,
};

const TRACKS: &str = "demo-table";
const NARROW: &str = "demo-table-narrow";
const FILES: &str = "demo-table-files";
const COVER: f32 = 40.0;
// What the demo lets a dragged column be.
const COLUMN_LIMITS: (f32, f32) = (80.0, 320.0);

struct Track {
    title: &'static str,
    artist: &'static str,
    album: &'static str,
    added: &'static str,
    seconds: u32,
}

const fn track(title: &'static str, artist: &'static str, album: &'static str, added: &'static str, seconds: u32) -> Track {
    Track { title, artist, album, added, seconds }
}

const SONGS: [Track; 12] = [
    track("Cuando pase el temblor", "Soda Stereo", "Nada personal", "2026-03-14", 229),
    track("Seminare", "Serú Girán", "Serú Girán", "2026-01-02", 207),
    track("Muchacha (ojos de papel)", "Almendra", "Almendra", "2025-11-23", 186),
    track("Un título demasiado largo para caber en la columna que le toca", "Una banda de nombre igual de largo que su canción", "Un disco que tampoco cabe", "2026-05-30", 3723),
    track("Flaca", "Andrés Calamaro", "Alta suciedad", "2026-02-19", 277),
    track("Lamento boliviano", "Los Enanitos Verdes", "Big Bang", "2025-12-08", 222),
    track("El amor después del amor", "Fito Páez", "El amor después del amor", "2026-04-01", 310),
    track("Rasguña las piedras", "Sui Generis", "Confesiones de invierno", "2025-10-17", 191),
    track("Persiana americana", "Soda Stereo", "Signos", "2026-03-14", 293),
    track("Mariposa tecknicolor", "Fito Páez", "Circo beat", "2026-06-11", 224),
    track("La balsa", "Los Gatos", "Los Gatos", "2025-09-05", 177),
    track("Ji ji ji", "Patricio Rey y sus Redonditos de Ricota", "Oktubre", "2026-02-27", 327),
];

const DOCUMENTS: [(&str, &str, &str, u32); 4] = [
    ("table.rs", "Rust", "hoy", 31),
    ("components.md", "Markdown", "ayer", 46),
    ("Un nombre de archivo demasiado largo para su columna.toml", "TOML", "hace 3 días", 2),
    ("lienzo.png", "Imagen", "hace 2 semanas", 512),
];

fn sort(state: &State, table: &str) -> Option<TableSort> {
    let value = state.text(&format!("{table}:sorted"));
    let key = value.get(1..).filter(|key| !key.is_empty())?;
    Some(TableSort { key: key.to_owned(), descending: value.starts_with('-') })
}

fn column_width(state: &State, table: &str, key: &str) -> f32 {
    let initial = if key == "album" { 180.0 } else { 110.0 };
    state.text(&format!("{table}:width:{key}")).parse().unwrap_or(initial)
}

/// What the table demos keep that a flag cannot hold: a sort that cycles
/// and a width that follows a drag.
pub fn update(state: &mut State, action: &UiAction) {
    let ours = |id: &str, part: &str| {
        let (table, key) = id.split_once(part).filter(|(table, _)| table.starts_with(TRACKS))?;
        Some((table.to_owned(), key.to_owned()))
    };
    match action {
        UiAction::Clicked { id } => {
            if let Some((table, key)) = ours(id, ":sort:") {
                let value = TableSort::toggled(sort(state, &table), &key)
                    .map_or(String::new(), |sort| format!("{}{}", if sort.descending { '-' } else { '+' }, sort.key));
                state.set(&format!("{table}:sorted"), &value);
            }
        }
        UiAction::DragStarted { id, position } => {
            if let Some((table, key)) = ours(id, ":resize:") {
                let start = format!("{} {}", position.x, column_width(state, &table, &key));
                state.set("demo-table-drag", &start);
            }
        }
        UiAction::DragMoved { id, position, .. } => {
            let start = state.text("demo-table-drag").split_once(' ').and_then(|(from, width)| {
                Some((from.parse::<f32>().ok()?, width.parse::<f32>().ok()?))
            });
            if let (Some((table, key)), Some((from, width))) = (ours(id, ":resize:"), start) {
                // Both stand after the flexible title: wider as the handle goes left.
                let width = (width - (position.x - from)).clamp(COLUMN_LIMITS.0, COLUMN_LIMITS.1);
                state.set(&format!("{table}:width:{key}"), &width.to_string());
            }
        }
        _ => {}
    }
}

fn clock(seconds: u32) -> String {
    match seconds / 3600 {
        0 => format!("{}:{:02}", seconds / 60, seconds % 60),
        hours => format!("{hours}:{:02}:{:02}", seconds / 60 % 60, seconds % 60),
    }
}

/// A list of tracks the way a player shows it. Each click on an optional
/// column flips its flag, so a flag set means the column is hidden.
fn tracks(cx: &Cx, id: &str, width: f32, query: &str, palette: ComponentPalette) -> Element {
    let state = cx.state;
    let sort = sort(state, id);
    let shown = |key: &str| !state.flag(&format!("{id}:column:{key}"));
    let columns = vec![
        TableColumn::fixed("index", "#", 24.0).align_end(),
        TableColumn::flexible("title", "Título", 160.0).sortable(),
        TableColumn::fixed("album", "Álbum", column_width(state, id, "album"))
            .sortable()
            .resizable(COLUMN_LIMITS.0)
            .optional(shown("album"))
            .hide_below(460.0),
        TableColumn::fixed("added", "Agregada", column_width(state, id, "added"))
            .sortable()
            .resizable(COLUMN_LIMITS.0)
            .optional(shown("added"))
            .hide_below(620.0),
        TableColumn::fixed("duration", "Duración", 56.0).icon(Icon::Clock).sortable().align_end(),
        TableColumn::fixed("more", "", ControlSize::Dense.height()),
    ];
    let needle = query.to_lowercase();
    let mut found: Vec<(usize, &Track)> = SONGS
        .iter()
        .enumerate()
        .filter(|(_, track)| [track.title, track.artist, track.album].iter().any(|text| text.to_lowercase().contains(&needle)))
        .collect();
    if let Some(sort) = &sort {
        found.sort_by(|(_, a), (_, b)| {
            let order = match sort.key.as_str() {
                "title" => a.title.cmp(b.title),
                "album" => a.album.cmp(b.album),
                "added" => a.added.cmp(b.added),
                _ => a.seconds.cmp(&b.seconds),
            };
            if sort.descending { order.reverse() } else { order }
        });
    }
    let playing = state.choice(&format!("{id}:row"), "2");
    let muted = |text: String| Text::new(text, palette).role(TextRole::Label).muted().truncate().render();
    let rows = found
        .iter()
        .enumerate()
        .map(|(place, (index, track))| {
            let cover = Element::panel(Element::icon(Icon::MusicNote).foreground(palette.muted).centered())
                .background(palette.selected)
                .radius(RadiusRole::Control.px())
                .size(Some(COVER), Some(COVER));
            let copy = Element::column(vec![
                Text::new(track.title, palette).role(TextRole::Body).truncate().render(),
                Text::new(track.artist, palette).role(TextRole::Caption).muted().truncate().render(),
            ])
            .gap(2.0);
            let more = format!("{id}:more:{index}");
            let cells = vec![
                muted((place + 1).to_string()),
                // Flexible, so the copy is cut at the column's edge.
                Element::row(vec![cover, copy.flex(1.0).centered_y()]).gap(12.0).flex(1.0),
                muted(track.album.to_owned()),
                muted(track.added.to_owned()),
                muted(clock(track.seconds)),
                IconButton::new(&more, Icon::More, palette)
                    .variant(ButtonVariant::Ghost)
                    .size(ControlSize::Dense)
                    .tooltip("Opciones")
                    .render(),
            ];
            TableRow::new(format!("{id}:row={index}"), cells)
                .selected(playing == index.to_string())
                .controls([more])
        })
        .collect();
    let empty = found.is_empty();
    let menu = format!("{id}:columns");
    // The trigger and the dismissal are two IDs, and each click flips one.
    let open = state.flag(&menu) != state.flag(&format!("{menu}:dismiss"));
    let table = Table::new(id, columns, width, palette)
        .rows(rows)
        .sort(sort)
        .sticky_header()
        .columns_menu(open, "Columnas")
        .menu_palette(cx.palette)
        .radius(RadiusRole::Surface)
        .sizing(TableSizing::ROOMY)
        .render();
    if empty {
        let none = Empty::new("Sin resultados", "Ninguna canción coincide con la búsqueda.", palette)
            .icon(Icon::Search)
            .width(width)
            .render();
        Element::column(vec![table, none])
    } else {
        table
    }
}

fn documents(cx: &Cx, width: f32, palette: ComponentPalette) -> Element {
    let state = cx.state;
    let sort = sort(state, FILES);
    let columns = vec![
        TableColumn::flexible("name", "Nombre", 120.0).sortable(),
        TableColumn::fixed("kind", "Tipo", 90.0).optional(!state.flag(&format!("{FILES}:column:kind"))),
        TableColumn::fixed("changed", "Modificado", 120.0).hide_below(420.0),
        TableColumn::fixed("size", "Tamaño", 72.0).sortable().align_end(),
    ];
    let mut files = DOCUMENTS.to_vec();
    if let Some(sort) = &sort {
        files.sort_by(|a, b| {
            let order = if sort.key == "name" { a.0.cmp(b.0) } else { a.3.cmp(&b.3) };
            if sort.descending { order.reverse() } else { order }
        });
    }
    let chosen = state.choice(&format!("{FILES}:row"), "");
    let rows = files
        .iter()
        .map(|(name, kind, changed, size)| {
            let cell = |text: &str, strong: bool| {
                let text = Text::new(text, palette).truncate();
                if strong { text } else { text.muted() }.render()
            };
            let cells = vec![cell(name, true), cell(kind, false), cell(changed, false), cell(&format!("{size} KB"), false)];
            TableRow::new(format!("{FILES}:row={name}"), cells).selected(chosen == *name)
        })
        .collect();
    Table::new(FILES, columns, width, palette).rows(rows).sort(sort).menu_palette(cx.palette).render()
}

pub fn view(cx: &Cx) -> Element {
    let width = cx.width.min(760.0);
    // A table sits on its card: the header pinned over the rows takes the card's fill.
    let palette = ComponentPalette { surface: cx.palette.raised, ..cx.palette };
    let card = |width: f32, content: Element| {
        Element::panel(content).padding(8.0).background(palette.surface).radius(RadiusRole::Card.px()).size(Some(width), None)
    };
    let query = cx.state.text("demo-table-query");
    let list = ScrollArea::new("demo-table-scroll", tracks(cx, TRACKS, width - 16.0, query, palette))
        // Over the rows, so the table keeps the whole width it was given.
        .appearance(ScrollbarAppearance::overlay())
        .render()
        .size(None, Some(360.0));
    let narrow = width.min(380.0);
    Element::column(vec![
        section(cx, "Lista de canciones", vec![
            caption(cx, "El encabezado queda fijo mientras las filas pasan por debajo. Un clic en una etiqueta ordena: ascendente, descendente y sin orden. Los límites de Álbum y Agregada se arrastran, y el botón del final, o un clic secundario en el encabezado, elige las columnas."),
            SearchField::new("demo-table-query", query, "Filtrar canciones", cx.palette).width(narrow).render(),
            card(width, list),
            caption(cx, "El anfitrión filtra y ordena: la tabla muestra las filas que recibe. La fila elegida queda rellena y el botón Opciones conserva su propio destino."),
        ]),
        section(cx, "Angosta", vec![
            caption(cx, "Cada columna dice bajo qué ancho de la tabla se deja afuera: primero Agregada, después Álbum. Lo que queda nunca se sale."),
            card(narrow, tracks(cx, NARROW, narrow - 16.0, "soda", palette)),
        ]),
        section(cx, "Compacta", vec![
            caption(cx, "El perfil de un panel: filas de 28. La columna Tipo se oculta con un clic secundario en el encabezado."),
            card(width.min(560.0), documents(cx, width.min(560.0) - 16.0, palette)),
        ]),
        section(cx, "Sin filas", vec![
            caption(cx, "Sin filas queda el encabezado; el anfitrión pone debajo su estado vacío."),
            card(width, tracks(cx, "demo-table-empty", width - 16.0, "no existe", palette)),
        ]),
    ])
    .gap(32.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app;
    use limn::{Frame, Point, Rect, Size, UiEvent, UiTree};

    const WINDOW: Size = Size::new(1280.0, 900.0);

    fn region(frame: &Frame, id: &str) -> Option<Rect> {
        frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
    }

    // What the gallery's host does with an event: update, rebuild, lay out.
    fn send(tree: &mut UiTree, state: &mut State, event: UiEvent) -> Frame {
        for action in tree.handle_event(event) {
            app::update(state, &action);
        }
        tree.set_root(app::view(state, WINDOW));
        tree.layout(WINDOW)
    }

    fn click(tree: &mut UiTree, state: &mut State, frame: &Frame, id: &str) -> Frame {
        let rect = region(frame, id).unwrap_or_else(|| panic!("no region {id}"));
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        send(tree, state, UiEvent::PointerDown(point));
        send(tree, state, UiEvent::PointerUp(point))
    }

    #[test]
    fn the_track_list_sorts_resizes_and_hides_columns_as_its_host_answers() {
        let mut state = State::starting_at("table");
        let mut tree = UiTree::new(app::view(&state, WINDOW));
        tree.set_reduced_motion(true);
        let frame = tree.layout(WINDOW);
        let row = |frame: &Frame, index: usize| region(frame, &format!("demo-table:row={index}")).unwrap().y;
        let step = row(&frame, 1) - row(&frame, 0);
        assert_eq!(step, TableSizing::ROOMY.row_height + TableSizing::ROOMY.row_gap);

        // By title: "Cuando…" stays first and "El amor…" follows it; then the other way round.
        let frame = click(&mut tree, &mut state, &frame, "demo-table:sort:title");
        assert_eq!(row(&frame, 6), row(&frame, 0) + step);
        let frame = click(&mut tree, &mut state, &frame, "demo-table:sort:title");
        assert_eq!(row(&frame, 1), row(&frame, 3) + step);
        let frame = click(&mut tree, &mut state, &frame, "demo-table:sort:title");
        assert_eq!((sort(&state, TRACKS), row(&frame, 1)), (None, row(&frame, 0) + step));

        // The album's boundary follows the pointer to the left.
        let x = |frame: &Frame, key: &str| region(frame, &format!("demo-table:sort:{key}")).map(|rect| rect.x);
        let (title, start) = (x(&frame, "title").unwrap(), x(&frame, "album").unwrap());
        let handle = region(&frame, "demo-table:resize:album").unwrap();
        let at = |travel: f32| Point::new(handle.x + handle.width / 2.0 + travel, handle.y + handle.height / 2.0);
        send(&mut tree, &mut state, UiEvent::PointerDown(at(0.0)));
        let frame = send(&mut tree, &mut state, UiEvent::PointerMove(at(-20.0)));
        assert_eq!((x(&frame, "album"), column_width(&state, TRACKS, "album")), (Some(start - 20.0), 200.0));
        // Past the demo's limit the width stops, and the title keeps its minimum.
        let frame = send(&mut tree, &mut state, UiEvent::PointerMove(at(-900.0)));
        assert_eq!(column_width(&state, TRACKS, "album"), COLUMN_LIMITS.1);
        assert_eq!(x(&frame, "album"), Some(title + 160.0 + TableSizing::ROOMY.gap));
        let frame = send(&mut tree, &mut state, UiEvent::PointerUp(at(-900.0)));

        // The columns button opens the list; a click on a column hides it.
        assert!(region(&frame, "demo-table:column:added").is_none());
        let frame = click(&mut tree, &mut state, &frame, "demo-table:columns");
        let frame = click(&mut tree, &mut state, &frame, "demo-table:column:added");
        assert_eq!(x(&frame, "added"), None);
        assert!(x(&frame, "album").is_some() && region(&frame, "demo-table:resize:added").is_none());
    }
}
