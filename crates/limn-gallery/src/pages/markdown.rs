use super::{caption, section, Cx};
use limn::{Component, Element, Markdown, MarkdownLabels, MarkdownLimits, Text};

const COMPLETE: &str = "# Resumen de la revisión\n\nSe corrigió **la sesión** en `src/main.rs`; el detalle está en la [guía](https://example.com/docs) y en https://github.com/rust-lang/rust.\n\n## Pasos\n\n1. Compilar el proyecto\n2. Ejecutar las pruebas\n   - [x] Unitarias\n   - [ ] De integración\n- Un punto suelto con un texto lo bastante largo como para partirse en dos líneas dentro de la columna de lectura.\n\n> Una cita mantiene su barra y su fondo aunque ocupe más de una línea de la columna.\n\n```rust\nfn main() {\n    println!(\"hola\");\n}\n```\n\n```\nsin lenguaje\n```\n\n---\n\n### Cierre\n\nTexto *con énfasis* y un salto  \nde línea forzado.";

const STREAMING: &str = "El resultado llega por partes.\n\n- Primer punto ya **cerrado**\n- Segundo punto con un enlace a medias https://example.com/doc\n\nLa última línea sigue **abierta y con `código";

const PLAIN: &str = "**Esto no es negrita**: revisa https://github.com/rust-lang/rust y https://example.com/docs, no `src/main.rs`.";

fn link(cx: &Cx, written: &str) -> Element {
    let palette = cx.palette;
    let address = Markdown::web_link(written);
    Element::row(vec![
        Element::icon(Markdown::link_icon(written)).foreground(palette.muted),
        Text::new(written, palette).mono().muted().render(),
        Text::new(address.as_deref().unwrap_or("no se abre"), palette).mono().truncate().render().flex(1.0),
    ])
    .gap(8.0)
    .size(Some(cx.width.min(560.0)), None)
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(560.0);
    let labels = MarkdownLabels {
        code_fallback: "CÓDIGO",
        truncated_note: "Texto extenso: vista limitada a los primeros bloques.",
        ..MarkdownLabels::DEFAULT
    };
    let page = Markdown::source_page(COMPLETE, 0);
    Element::column(vec![
        section(cx, "Markdown completo", vec![
            caption(cx, "Encabezados, listas, cita, código, regla y enlaces, con las etiquetas del host"),
            Markdown::new(COMPLETE, palette)
                .width(width)
                .labels(labels)
                .link_action_prefix("markdown-complete:")
                .render()
                .element,
        ]),
        section(cx, "En curso", vec![
            caption(cx, ".streaming(true): la cola sin cerrar ya tiene estilo y su dirección aún no abre nada"),
            Markdown::new(STREAMING, palette)
                .width(width)
                .streaming(true)
                .link_action_prefix("markdown-streaming:")
                .render()
                .element,
        ]),
        section(cx, "Límites", vec![
            caption(cx, ".limits de 3 bloques: el resto queda para la vista de la fuente"),
            Markdown::new(COMPLETE, palette)
                .width(width)
                .labels(labels)
                .limits(MarkdownLimits { source_chars: 20_000, blocks: 3 })
                .link_action_prefix("markdown-limited:")
                .render()
                .element,
            caption(cx, "Columna de 220"),
            Markdown::new(COMPLETE, palette)
                .width(220.0)
                .labels(labels)
                .limits(MarkdownLimits { source_chars: 20_000, blocks: 6 })
                .link_action_prefix("markdown-narrow:")
                .render()
                .element
                .size(Some(220.0), None),
        ]),
        section(cx, "Texto literal con enlaces", vec![
            caption(cx, ".plain_with_autolinks(21.0): solo las direcciones completas son enlaces"),
            Markdown::new(PLAIN, palette)
                .plain_with_autolinks(21.0)
                .link_action_prefix("markdown-plain:")
                .render()
                .element
                .size(Some(width), None),
        ]),
        section(cx, "Direcciones para el host", vec![
            caption(cx, "Markdown::web_link normaliza o rechaza; Markdown::link_icon da el glifo"),
            link(cx, "https://GitHub.com/limn"),
            link(cx, "https://example.com/docs"),
            link(cx, "file:///etc/passwd"),
        ]),
        section(cx, "Fuente por páginas", vec![
            caption(cx, "Markdown::source_page(..).render(width, palette)"),
            page.render(width, palette),
        ]),
    ])
    .gap(28.0)
}
