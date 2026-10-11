use super::{caption, section, Cx};
use limn::{Element, Flowchart, Markdown, RadiusRole};

const DOWN: &str = "flowchart TD
    A[Se inicializa la demo] --> B[workspace-preview.js prepara la experiencia]
    B --> C[workspace-dock.js gestiona paneles y pestañas]
    C --> D[Archivos, vistas y terminales de ejemplo]
    A --> E[Recorrido guiado automático o interacción manual]
    E --> F[El usuario explora una solicitud preparada]
    F --> G[Recorre planificación, aprobaciones y resultado de ejemplo]
    D --> H[El usuario cambia archivos o vistas del ejemplo]
    G --> I[La experiencia muestra un resultado simulado]
    H --> I";

const RIGHT: &str = "flowchart LR
    A[Contenido en docs/sections/] --> B[docs/content.js organiza guías y navegación]
    B --> C[scripts/build-docs.mjs]
    C --> D[Genera páginas HTML estáticas]
    C --> E[Genera índice de búsqueda local]
    D --> F[Salida dentro de dist/]
    E --> F
    F --> G[El navegador muestra la documentación]";

const SHAPES: &str = "flowchart TD
    start([Inicio]) --> check{¿Hay cambios?}
    check -->|Sí| build[Compilar]
    check -->|No| done((Fin))
    subgraph Verificación
        build -.-> test(Probar)
    end
    test ==>|pasa| done
    test -- falla --> check";

fn chart(cx: &Cx, id: &str, source: &str) -> Element {
    let palette = cx.palette;
    match Flowchart::parse(source, palette) {
        Ok(chart) => Element::panel(chart.scrollable(id))
            .padding(12.0)
            .background(palette.raised)
            .border(palette.border, 1.0)
            .radius(RadiusRole::Card.px()),
        Err(error) => caption(cx, &error.to_string()),
    }
}

pub fn view(cx: &Cx) -> Element {
    let fence = format!("Un bloque `mermaid` cerrado se dibuja:\n\n```mermaid\n{RIGHT}\n```\n");
    Element::column(vec![
        section(cx, "De arriba abajo", vec![
            caption(cx, "Flowchart::parse(fuente, palette)?.scrollable(id): rangos por camino más largo"),
            chart(cx, "diagram-down", DOWN),
        ]),
        section(cx, "De izquierda a derecha", vec![
            caption(cx, "Más ancho que su contenedor se desplaza en horizontal"),
            chart(cx, "diagram-right", RIGHT),
        ]),
        section(cx, "Formas, etiquetas y grupos", vec![
            caption(cx, "Estadio, decisión, círculo, redondeado; líneas punteada y gruesa; un ciclo"),
            chart(cx, "diagram-shapes", SHAPES),
        ]),
        section(cx, "Dentro de Markdown", vec![
            caption(cx, "Lo que no se entiende, o sigue abierto, queda como bloque de código"),
            Markdown::new(&fence, cx.palette).width(cx.width.min(560.0)).render().element,
        ]),
    ])
    .gap(28.0)
}
