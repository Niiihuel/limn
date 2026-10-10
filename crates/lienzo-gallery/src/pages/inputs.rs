use super::{caption, row, section, Cx};
use lienzo::{Component, Element, SearchField, Switch, TextArea, TextField};

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let state = cx.state;
    let width = cx.width.min(360.0);
    Element::column(vec![
        section(cx, "Campo de texto", vec![
            TextField::new("input-name", state.text("input-name"), "Nombre del proyecto", palette)
                .width(width)
                .render(),
            caption(cx, &format!("Valor: {}", state.text("input-name"))),
            TextField::new("input-password", state.text("input-password"), "Contraseña", palette)
                .password(true)
                .width(width)
                .render(),
            TextField::new("input-disabled", "No editable", "", palette).disabled().width(width).render(),
        ]),
        section(cx, "Búsqueda", vec![
            SearchField::new("input-search", state.text("input-search"), "Buscar", palette)
                .shortcut("Ctrl+K")
                .width(width)
                .render(),
        ]),
        section(cx, "Área de texto", vec![
            TextArea::new("input-notes", state.text("input-notes"), "Notas", palette)
                .width(width)
                .height(120.0)
                .render(),
        ]),
        section(cx, "Interruptor", vec![row(vec![
            Switch::new("input-switch", state.flag("input-switch"), palette).render(),
            Switch::new("input-switch-disabled", true, palette).disabled().render(),
        ])]),
    ])
    .gap(32.0)
}
