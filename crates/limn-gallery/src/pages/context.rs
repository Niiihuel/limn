use super::{caption, row, section, Cx};
use limn::{
    AlertColors, AlertTone, Button, ButtonVariant, Color, Component, ContextItem, ContextShare,
    ContextShareAction, ContextTones, ContextWindow, Element,
};

const ID: &str = "context";
const WINDOW: u64 = 200_000;
const FILES: [(&str, &str, u64); 3] = [
    ("esquema", "esquema.sql", 9_400),
    ("facturacion", "facturacion.rs", 21_300),
    ("notas", "notas-de-reunion.md", 15_400),
];

pub fn view(cx: &Cx) -> Element {
    let palette = cx.palette;
    let accent = cx.state.theme().accent;
    let tone = |tone| AlertColors::for_tone(tone, palette).accent;
    let faded = |color: Color, alpha| Color::rgba(color.r, color.g, color.b, alpha);
    let files: Vec<ContextItem> = FILES
        .into_iter()
        .filter(|(id, ..)| !cx.state.flag(&format!("{ID}:remove:archivos:{id}")))
        .map(|(id, label, tokens)| ContextItem { id: id.into(), label: label.into(), tokens })
        .collect();
    let compacted = cx.state.flag("context:action:conversacion");
    let busy = cx.state.flag("context-busy") && !compacted;
    let shares = vec![
        ContextShare::new("sistema", "Sistema", 4_200, palette.text),
        ContextShare::new("herramientas", "Herramientas", 12_800, palette.muted),
        ContextShare::new("archivos", "Archivos", files.iter().map(|file| file.tokens).sum(), faded(palette.muted, 150))
            .items(files),
        ContextShare::new("conversacion", "Conversación", if compacted { 30_400 } else { 108_600 }, accent).action(
            if compacted {
                ContextShareAction::Done("Compactada".into())
            } else if busy {
                ContextShareAction::Busy("Compactando".into())
            } else {
                ContextShareAction::Offer("Compactar".into())
            },
        ),
        ContextShare::new("memoria", "Memoria", 3_900, faded(accent, 120)),
    ];
    let used: u64 = shares.iter().map(|share| share.tokens).sum();
    // The host mirrors the control under the pointer, as the tree reports it.
    let focused = ContextWindow::share_under(ID, cx.state.text("hovered"));
    let in_focus = focused.and_then(|id| shares.iter().find(|share| share.id == id));
    let status = match in_focus {
        Some(share) => format!("{} · {}% de la ventana", share.label, (share.tokens * 100 + WINDOW / 2) / WINDOW),
        None if busy => "Compactando…".into(),
        None if compacted => "78.2K tokens liberados".into(),
        None => format!("{} libres", ContextWindow::tokens_text(WINDOW.saturating_sub(used) as f32)),
    };
    let saved = compacted && in_focus.is_none();
    let in_focus_label = in_focus.map(|share| share.label.clone());
    let mut card = ContextWindow::new(
        ID,
        "Ventana de contexto",
        shares,
        WINDOW,
        palette,
        ContextTones {
            warning: tone(AlertTone::Warning),
            danger: tone(AlertTone::Error),
            success: tone(AlertTone::Success),
        },
    )
    .badge("claude-opus-5-5")
    .caption("de 200K tokens")
    .focused(focused)
    .expanded(cx.state.flag("context:share:archivos").then_some("archivos"))
    .header_icon("context-copy", limn::Icon::Copy, "Copiar diagnóstico")
    .footer_action("context-new", "Nuevo chat");
    if let Some(share) = in_focus_label.as_deref() {
        card = card.status_lead(share);
    }
    card = if saved { card.status_done(status) } else { card.status(status) };
    let reset = FILES.iter().fold(String::from("context:action:conversacion=off&context-busy=off"), |reset, (id, ..)| {
        format!("{reset}&{ID}:remove:archivos:{id}=off")
    });
    Element::column(vec![section(cx, "Ventana de contexto", vec![
        caption(cx, "Pasá el cursor por una categoría, abrí Archivos y quitá uno, o compactá la conversación: las cifras cuentan y las celdas se vacían solas."),
        row(vec![Element::column(vec![card.render()])
            .background(palette.surface)
            .border(palette.border, 1.0)
            .radius(16.0)
            .clip_children()]),
        row(vec![
            Button::new("context-busy", if busy { "Dejar de compactar" } else { "Simular compactación en curso" }, palette)
                .variant(ButtonVariant::Outline)
                .render(),
            Button::new(reset, "Restablecer", palette).variant(ButtonVariant::Outline).render(),
        ]),
    ])])
}
