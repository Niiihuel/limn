use super::{caption, section, Cx};
use limn::{Diagram, Element, Markdown, RadiusRole};

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

const PURCHASE: &str = "sequenceDiagram
    autonumber
    actor C as Cliente
    participant W as Tienda web
    participant P as Pagos
    participant D as Depósito
    C->>W: Confirma el carrito
    W->>+P: Cobra el total
    Note right of P: Valida la tarjeta<br/>con el banco
    alt Pago aprobado
        P-->>W: Aprobado
        W->>D: Reserva los productos
        loop Por cada producto
            D->>D: Descuenta del stock
        end
        D-->>W: Reserva confirmada
    else Pago rechazado
        P-->>-W: Rechazado
        W-->>C: Pide otro medio de pago
    end
    opt El cliente dejó su correo
        W-)C: Envía el comprobante
    end";

const TURN: &str = "sequenceDiagram
    title Un turno del agente
    participant V as Ventana
    participant M as Motor
    participant H as Herramientas
    V->>+M: Inicia el turno
    Note over V,M: La ventana solo dibuja
    par Lectura
        M->>+H: read_file
        H-->>-M: Contenido
    and Búsqueda
        M->>+H: grep
        H-->>-M: Coincidencias
    end
    critical Guardar la sesión
        M->>M: Escribe el registro
    option Disco lleno
        M--xV: Avisa del error
    end
    break Si el usuario cancela
        V-xM: Cancela el turno
    end
    rect rgb(0, 0, 0)
        M-->>-V: Turno terminado
    end";

fn chart(cx: &Cx, id: &str, source: &str) -> Element {
    let palette = cx.palette;
    match Diagram::parse(source, palette) {
        Ok(chart) => Element::panel(chart.scrollable(id))
            .padding(12.0)
            .background(palette.raised)
            .border(palette.border, 1.0)
            .radius(RadiusRole::Card.px()),
        Err(error) => caption(cx, &error.to_string()),
    }
}

pub fn view(cx: &Cx) -> Element {
    let fence = format!(
        "Un bloque `mermaid` cerrado se dibuja:\n\n```mermaid\n{RIGHT}\n```\n\nTambién una secuencia:\n\n\
         ```mermaid\nsequenceDiagram\n    Editor->>Vista previa: Cambia el texto\n    Vista previa-->>Editor: Se redibuja\n```\n"
    );
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
        section(cx, "Secuencia", vec![
            caption(cx, "SequenceDiagram::parse(fuente, palette)?.scrollable(id): una línea de vida por participante"),
            chart(cx, "diagram-sequence", PURCHASE),
        ]),
        section(cx, "Secuencia: bloques, notas y finales de flecha", vec![
            caption(cx, "par, critical y break; cruz y flecha abierta; un mensaje a sí mismo; rect como zona resaltada"),
            chart(cx, "diagram-sequence-blocks", TURN),
        ]),
        section(cx, "Dentro de Markdown", vec![
            caption(cx, "Lo que no se entiende, o sigue abierto, queda como bloque de código"),
            Markdown::new(&fence, cx.palette).width(cx.width.min(560.0)).render().element,
        ]),
    ])
    .gap(28.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sample_is_drawn_and_none_falls_back_to_its_error() {
        for source in [DOWN, RIGHT, SHAPES, PURCHASE, TURN] {
            let drawn = Diagram::parse(source, limn::ComponentPalette::neutral_dark());
            assert!(drawn.is_ok(), "{:?}: {source}", drawn.err());
        }
    }
}
