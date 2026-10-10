use super::{caption, row, section, Cx};
use limn::{
    AlertDialog, AlertTone, Button, ButtonVariant, Color, Component, ContextMenu, ContextMenuEntry,
    ContextMenuItem, DropdownList, DropdownMenu, DropdownMenuEntry, DropdownMenuItem, DropdownOption,
    Element, FontFamily, Icon, Modal, Popover, TextField, Toast,
};

const LANGUAGES: [(&str, &str); 3] = [("rust", "Rust"), ("python", "Python"), ("typescript", "TypeScript")];

fn trigger(cx: &Cx, id: &str, label: &str) -> Element {
    Button::new(id, label, cx.palette).variant(ButtonVariant::Outline).render()
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let open = |group: &str| state.text(group) == "open";

    let popover = Popover::new(
        trigger(cx, "overlay-popover=open", "Abrir popover"),
        Element::column(vec![
            Element::text("Compartir enlace")
                .font_family(FontFamily::Sans)
                .font_size(13.0)
                .foreground(palette.text),
            caption(cx, "Cualquiera con el enlace puede ver el documento."),
            Button::new("overlay-popover=copied", "Copiar enlace", palette).render(),
        ])
        .gap(8.0)
        .padding(12.0),
        open("overlay-popover"),
        "overlay-popover=closed",
        palette,
    )
    .width(260.0)
    .render();

    let language = state.choice("overlay-language", LANGUAGES[0].0);
    let label = LANGUAGES.iter().find(|(id, _)| *id == language).map_or(LANGUAGES[0].1, |(_, label)| label);
    let list = DropdownList::new(
        Button::new("overlay-list=open", label, palette)
            .variant(ButtonVariant::Outline)
            .trailing_icon(Icon::ChevronDown)
            .render(),
        LANGUAGES
            .iter()
            .map(|(id, label)| {
                DropdownOption::new(format!("overlay-language={id}&overlay-list=closed"), *label)
                    .selected(*id == language)
            })
            .collect(),
        open("overlay-list"),
        "overlay-list=closed",
        palette,
    )
    .width(200.0)
    .render();

    let menu = DropdownMenu::new(
        trigger(cx, "overlay-menu=open", "Abrir menú"),
        vec![
            DropdownMenuEntry::Heading("Archivo".into()),
            DropdownMenuItem::new("overlay-menu=rename", "Renombrar").icon(Icon::Edit).shortcut("F2").into(),
            DropdownMenuItem::new("overlay-menu=archive", "Archivar").icon(Icon::Archive).into(),
            DropdownMenuItem::new("overlay-menu=pinned", "Fijar").icon(Icon::Pin).disabled().into(),
            DropdownMenuEntry::Separator,
            DropdownMenuItem::new("overlay-menu=delete", "Eliminar")
                .icon(Icon::Trash)
                .shortcut("Supr")
                .destructive(Color::rgb(229, 83, 75))
                .into(),
        ],
        open("overlay-menu"),
        "overlay-menu=closed",
        palette,
    )
    .width(220.0)
    .render();

    let context = ContextMenu::new(
        "overlay-context",
        Element::panel(caption(cx, "Clic derecho aquí"))
            .padding(24.0)
            .border(palette.border, 1.0)
            .radius(limn::tokens().radius.card),
        vec![
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=copy", "Copiar").icon(Icon::File).shortcut("Ctrl+C")),
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=paste", "Pegar").enabled(false)),
            ContextMenuEntry::Separator,
            ContextMenuEntry::Item(ContextMenuItem::new("overlay-context-action=delete", "Eliminar").icon(Icon::Trash)),
        ],
        palette,
    )
    .render();

    Element::column(vec![
        section(cx, "Popover", vec![row(vec![popover])]),
        section(cx, "Lista desplegable", vec![row(vec![list])]),
        section(cx, "Menú", vec![row(vec![menu]), caption(cx, &format!("Elegido: {}", state.text("overlay-menu")))]),
        section(cx, "Menú contextual", vec![row(vec![context])]),
        section(cx, "Diálogos", vec![
            row(vec![
                trigger(cx, "overlay-modal=open", "Abrir modal"),
                trigger(cx, "overlay-alert=open", "Abrir confirmación"),
            ]),
            caption(cx, &format!("Confirmación: {}", state.text("overlay-alert"))),
        ]),
        section(cx, "Toast", vec![row(vec![
            trigger(cx, "overlay-toast=shown", "Mostrar toasts"),
            trigger(cx, "overlay-toast=hidden", "Ocultar"),
        ])]),
    ])
    .gap(32.0)
}

/// Wrap the window in the layers this page controls.
pub fn layers(cx: &Cx, base: Element) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let base = Modal::new(
        base,
        state.text("overlay-modal") == "open",
        "Nuevo proyecto",
        "El nombre se puede cambiar más tarde.",
        "overlay-modal=closed",
        palette,
    )
    .width(420.0)
    .body(
        TextField::new("overlay-modal-name", state.text("overlay-modal-name"), "Nombre", palette).render(),
    )
    .footer(Element::row(vec![
        Element::spacer().flex(1.0),
        Button::new("overlay-modal=cancelled", "Cancelar", palette).variant(ButtonVariant::Outline).render(),
        Button::new("overlay-modal=created", "Crear", palette).render(),
    ])
    .gap(8.0))
    .initial_focus("overlay-modal-name")
    .backdrop_dismiss(true)
    .render();
    let base = AlertDialog::new(
        base,
        state.text("overlay-alert") == "open",
        "¿Eliminar el proyecto?",
        "Esta acción no se puede deshacer.",
        "overlay-alert=confirmed",
        "overlay-alert=cancelled",
        palette,
    )
    .confirm_label("Eliminar")
    .cancel_label("Cancelar")
    .destructive(Color::rgb(161, 67, 71))
    .render();
    let toasts = if state.text("overlay-toast") == "shown" {
        vec![
            Toast::new("Cambios guardados", palette)
                .description("limn/src/empty.rs")
                .tone(AlertTone::Success)
                .render(),
            Toast::new("Sin conexión", palette)
                .description("Se reintentará en unos segundos.")
                .tone(AlertTone::Error)
                .dismissible("overlay-toast=hidden")
                .render(),
        ]
    } else {
        Vec::new()
    };
    Toast::layer(base, toasts, 16.0)
}
