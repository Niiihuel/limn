use super::{caption, row, section, Cx};
use limn::{
    Button, ButtonVariant, Card, CardDensity, CodeBlock, CodeBlockSizing, Component, Element,
    FontFamily, ListRow, OutputBlock, RadiusRole, Separator, Text, TextRole, TextSpan,
};

fn card_copy(cx: &Cx, title: &str, detail: &str) -> Element {
    Element::column(vec![
        Text::new(title, cx.palette).render(),
        Text::new(detail, cx.palette).role(TextRole::Small).muted().render(),
    ])
    .gap(6.0)
}

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let width = cx.width.min(520.0);
    let densities = [
        ("Compact: 8", CardDensity::Compact),
        ("Regular: 10", CardDensity::Regular),
        ("Roomy: 12", CardDensity::Roomy),
    ];
    let log: String = (1..=4000).map(|n| format!("test suite::case_{n:04} ... ok\n")).collect();
    let source = "fn main() {\n    let total: u32 = (1..=10).sum();\n    println!(\"{total}\");\n}";
    let spans = vec![
        TextSpan::new("fn ", palette.muted, 13.0, FontFamily::Monospace),
        TextSpan::new("main", palette.text, 13.0, FontFamily::Monospace),
        TextSpan::new("() {}  ", palette.muted, 13.0, FontFamily::Monospace),
        TextSpan::new("// coloreado por el host", palette.muted, 13.0, FontFamily::Monospace),
    ];
    Element::column(vec![
        section(cx, "Tarjeta", vec![
            row(densities
                .into_iter()
                .map(|(name, density)| {
                    Card::new(card_copy(cx, name, "Borde y radio"), palette)
                        .density(density)
                        .width(128.0)
                        .render()
                })
                .collect()),
            row(vec![
                Card::new(card_copy(cx, "Sin borde", "Radio de superficie"), palette)
                    .border(false)
                    .radius(RadiusRole::Surface)
                    .width(160.0)
                    .render(),
                Card::new(card_copy(cx, "Radio de panel", "Para un contenedor grande"), palette)
                    .radius(RadiusRole::Panel)
                    .width(200.0)
                    .render(),
            ]),
            row(vec![
                Card::new(
                    Element::column(vec![
                        ListRow::new("blocks-card-flush", "Flush: fila hasta el borde", palette).render(),
                        Separator::horizontal(palette).render(),
                        card_copy(cx, "Sin relleno", "El contenido trae el suyo").padding(8.0),
                    ])
                    .clip_children(),
                    palette,
                )
                .density(CardDensity::Flush)
                .width(200.0)
                .render(),
            ]),
            Card::new(
                Element::column(vec![
                    card_copy(cx, "Plan propuesto", "Una tarjeta sin ancho ocupa el que recibe; el contenido es del host, incluidas sus acciones."),
                    row(vec![Button::new("blocks-card-build", "Construir", palette)
                        .variant(ButtonVariant::Outline)
                        .render()]),
                ])
                .gap(10.0),
                palette,
            )
            .density(CardDensity::Roomy)
            .render()
            .size(Some(width), None),
        ]),
        section(cx, "Bloque de salida", vec![
            caption(cx, "4000 líneas: sólo se construyen las visibles, y la nota avisa del recorte"),
            OutputBlock::new("blocks-output-long", log, palette)
                .width(width)
                .max_height(136.0)
                .truncation_note("Vista parcial · resultado demasiado extenso")
                .render(),
            caption(cx, "Salida corta, con una tinta del host y una línea más ancha que el bloque"),
            OutputBlock::new(
                "blocks-output-short",
                "error[E0308]: mismatched types\n\n  expected `u32`, found `&str` in a line that runs well past the right edge of the block and is clipped there",
                palette,
            )
            .ink(palette.muted)
            .width(width)
            .render(),
            caption(cx, "Vacío"),
            OutputBlock::new("blocks-output-empty", "", palette).width(width).render(),
        ]),
        section(cx, "Bloque de código", vec![
            caption(cx, "CodeBlockSizing::CHAT"),
            CodeBlock::new(source, palette).label("RUST").width(width).render(),
            caption(cx, "CodeBlockSizing::DOCUMENT, con tramos coloreados y sin etiqueta"),
            CodeBlock::new(source, palette)
                .label("rust")
                .sizing(CodeBlockSizing::DOCUMENT)
                .width(width)
                .render(),
            CodeBlock::new("", palette)
                .spans(spans)
                .sizing(CodeBlockSizing::DOCUMENT)
                .width(width)
                .render(),
            caption(cx, "Angosto: el código se parte dentro del bloque"),
            CodeBlock::new("cargo test -p limn -p limn-egui -p limn-gallery", palette)
                .label("SH")
                .width(180.0)
                .render(),
        ]),
    ])
    .gap(32.0)
}
