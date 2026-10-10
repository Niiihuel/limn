use super::{caption, row, section, Cx};
use limn::{Color, Component, ComponentPalette, Element, FontFamily, Scheme, Surface, Text, TextRole};

fn swatch(cx: &Cx, name: &str, color: Color) -> Element {
    Element::row(vec![
        Element::spacer()
            .background(color)
            .border(cx.palette.border, 1.0)
            .radius(limn::tokens().radius.control)
            .size(Some(28.0), Some(28.0)),
        Text::new(name, cx.palette).mono().render().centered_y(),
    ])
    .gap(10.0)
    .size(Some(180.0), None)
}

/// A surface painted with one palette, holding an action that hovers and
/// presses with that palette's own colors.
fn sample(cx: &Cx, id: &str, name: &str, note: &str, palette: ComponentPalette) -> Element {
    let action = palette.interactive_surface(
        Element::button(format!("scheme-{id}"), "Acción")
            .font_family(FontFamily::Sans)
            .foreground(palette.text),
    );
    Element::column(vec![
        Text::new(name, palette).role(TextRole::Label).mono().render(),
        Text::new(note, palette).muted().render(),
        row(vec![action]),
    ])
    .gap(8.0)
    .padding(12.0)
    .background(palette.surface)
    .border(cx.palette.border, 1.0)
    .radius(limn::tokens().radius.card)
    .size(Some(cx.width.min(248.0)), None)
}

fn on(scheme: Scheme, id: &str, label: &str, surface: Surface, ink: Color) -> Element {
    let palette = scheme.on(surface, ink);
    palette.interactive_surface(
        Element::button(format!("scheme-surface-{id}"), label)
            .font_family(FontFamily::Sans)
            .background(palette.surface)
            .foreground(palette.text),
    )
}

pub fn view(cx: &Cx) -> Element {
    let theme = cx.state.theme();
    let scheme = theme.scheme();
    let wrap = |items: Vec<Element>, item_width: f32| {
        let per_row = ((cx.width / item_width) as usize).clamp(1, 4);
        let mut rows = Vec::new();
        let mut items = items.into_iter().peekable();
        while items.peek().is_some() {
            rows.push(row(items.by_ref().take(per_row).collect()));
        }
        Element::column(rows).gap(10.0)
    };
    Element::column(vec![
        caption(cx, "Un Scheme nombra cada color por su papel. Ningún componente lo recibe: sólo produce paletas."),
        section(cx, "Papeles", vec![wrap(vec![
            swatch(cx, "canvas", scheme.canvas),
            swatch(cx, "panel", scheme.panel),
            swatch(cx, "raised", scheme.raised),
            swatch(cx, "hover", scheme.hover),
            swatch(cx, "selected", scheme.selected),
            swatch(cx, "border", scheme.border),
            swatch(cx, "text", scheme.text),
            swatch(cx, "muted", scheme.muted),
            swatch(cx, "muted_on_raised", scheme.muted_on_raised),
            swatch(cx, "menu", scheme.menu),
            swatch(cx, "menu_hover", scheme.menu_hover),
            swatch(cx, "menu_muted", scheme.menu_muted),
            swatch(cx, "control.surface", scheme.control.surface),
            swatch(cx, "control.hover", scheme.control.hover),
            swatch(cx, "control.pressed", scheme.control.pressed),
            swatch(cx, "control.border", scheme.control.border),
        ], 192.0)]),
        section(cx, "Paletas", vec![wrap(vec![
            sample(cx, "palette", "palette()", "Sobre el lienzo.", scheme.palette()),
            sample(cx, "card", "card()", "Contenido sobre una tarjeta.", scheme.card()),
            sample(cx, "menu", "menu()", "Un menú flotante.", scheme.menu()),
            sample(cx, "on-raised", "on_raised()", "Un control relleno sobre una tarjeta.", scheme.on_raised()),
        ], 260.0)]),
        section(cx, "Superficie de una acción", vec![
            row(vec![
                on(scheme, "flat", "Flat", Surface::Flat(scheme.canvas), scheme.text),
                on(scheme, "raised", "Raised", Surface::Raised(scheme.raised), scheme.text),
                on(scheme, "solid", "Solid", Surface::Solid(theme.accent_surface_strong()), scheme.text),
            ]),
            caption(cx, "Flat usa el hover del esquema, Raised da un paso desde su propio color y Solid se mantiene."),
            row(vec![
                swatch(cx, "raised", scheme.raised),
                swatch(cx, "step(raised)", scheme.step(scheme.raised)),
            ]),
        ]),
    ])
    .gap(32.0)
}
