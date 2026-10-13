use super::{caption, section, Cx, Section};
use limn::{Diagram, Element, Markdown, RadiusRole};

const DOWN: &str = "flowchart TD
    A[The demo initializes] --> B[workspace-preview.js prepares the experience]
    B --> C[workspace-dock.js manages panels and tabs]
    C --> D[Sample files, views and terminals]
    A --> E[Automatic guided tour or manual interaction]
    E --> F[The user explores a prepared request]
    F --> G[Walks through planning, approvals and a sample result]
    D --> H[The user changes sample files or views]
    G --> I[The experience shows a simulated result]
    H --> I";

const RIGHT: &str = "flowchart LR
    A[Content in docs/sections/] --> B[docs/content.js organizes guides and navigation]
    B --> C[scripts/build-docs.mjs]
    C --> D[Generates static HTML pages]
    C --> E[Generates a local search index]
    D --> F[Output inside dist/]
    E --> F
    F --> G[The browser shows the documentation]";

const SHAPES: &str = "flowchart TD
    start([Start]) --> check{Are there changes?}
    check -->|Yes| build[Build]
    check -->|No| done((End))
    subgraph Verificación[Verification]
        build -.-> test(Test)
    end
    test ==>|pass| done
    test -- fail --> check";

const PURCHASE: &str = "sequenceDiagram
    autonumber
    actor C as Customer
    participant W as Web store
    participant P as Payments
    participant D as Warehouse
    C->>W: Confirms the cart
    W->>+P: Charges the total
    Note right of P: Validates the card<br/>with the bank
    alt Payment approved
        P-->>W: Approved
        W->>D: Reserves the products
        loop For each product
            D->>D: Deducts from stock
        end
        D-->>W: Reservation confirmed
    else Payment declined
        P-->>-W: Declined
        W-->>C: Requests another payment method
    end
    opt The customer provided an email
        W-)C: Sends the receipt
    end";

const TURN: &str = "sequenceDiagram
    title An agent turn
    participant V as Window
    participant M as Engine
    participant H as Tools
    V->>+M: Starts the turn
    Note over V,M: The window only renders
    par Reading
        M->>+H: read_file
        H-->>-M: Content
    and Search
        M->>+H: grep
        H-->>-M: Matches
    end
    critical Save the session
        M->>M: Writes the log
    option Disk full
        M--xV: Reports the error
    end
    break If the user cancels
        V-xM: Cancels the turn
    end
    rect rgb(0, 0, 0)
        M-->>-V: Turn finished
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

pub fn view(cx: &Cx) -> Vec<Section> {
    let fence = format!(
        "A closed `mermaid` block is rendered:\n\n```mermaid\n{RIGHT}\n```\n\nA sequence too:\n\n\
         ```mermaid\nsequenceDiagram\n    participant Editor\n    participant Vista previa as Preview\n    Editor->>Vista previa: Changes the text\n    Vista previa-->>Editor: Redraws\n```\n"
    );
    vec![
        section(cx, "Top to bottom", vec![
            caption(cx, "Flowchart::parse(source, palette)?.scrollable(id): ranks by longest path"),
            chart(cx, "diagram-down", DOWN),
        ]),
        section(cx, "Left to right", vec![
            caption(cx, "A chart wider than its container scrolls horizontally"),
            chart(cx, "diagram-right", RIGHT),
        ]),
        section(cx, "Shapes, labels and groups", vec![
            caption(cx, "Stadium, decision, circle, rounded; dotted and thick lines; a cycle"),
            chart(cx, "diagram-shapes", SHAPES),
        ]),
        section(cx, "Sequence", vec![
            caption(cx, "SequenceDiagram::parse(source, palette)?.scrollable(id): one lifeline per participant"),
            chart(cx, "diagram-sequence", PURCHASE),
        ]),
        section(cx, "Sequence: blocks, notes and arrowheads", vec![
            caption(cx, "par, critical and break; cross and open arrow; a self-message; rect as a highlighted region"),
            chart(cx, "diagram-sequence-blocks", TURN),
        ]),
        section(cx, "Inside Markdown", vec![
            caption(cx, "Unsupported or unfinished diagrams remain code blocks"),
            Markdown::new(&fence, cx.palette).width(cx.width.min(560.0)).render().element,
        ]),
    ]
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
