use super::*;
use crate::{Diagram, Flowchart, UiTree};

const LOGIN: &str = "sequenceDiagram
    participant U as Usuario
    participant A as App
    participant S as Servidor
    U->>A: Ingresa sus credenciales
    A->>+S: POST /login
    S-->>-A: 200 OK con el token
    A-->>U: Muestra el panel";

const CHECKOUT: &str = "sequenceDiagram
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

const AGENT: &str = "sequenceDiagram
    title Una tarea del agente, de punta a punta
    actor U as Usuario
    participant V as Ventana
    participant M as Motor
    participant H as Herramientas
    participant L as Modelo
    U->>V: Escribe la tarea
    V->>+M: Inicia el turno
    Note over V,M: La ventana solo dibuja,<br/>el motor decide
    loop Hasta terminar la tarea
        M->>+L: Envía el contexto
        L-->>-M: Responde con texto o una llamada
        alt Pide una herramienta
            M->>V: Solicita aprobación
            V-->>M: Aprobada
            par Lectura de archivos
                M->>+H: read_file
                H-->>-M: Contenido
            and Búsqueda
                M->>+H: grep
                H-->>-M: Coincidencias
            end
        else Responde al usuario
            M-->>V: Muestra la respuesta
        end
    end
    critical Guardar la sesión
        M->>M: Escribe el registro
    option Disco lleno
        M--xV: Avisa del error
    end
    break Si el usuario cancela
        V-xM: Cancela el turno
    end
    M-->>-V: Turno terminado
    V-->>U: Muestra el resultado";

const EXPORT: &str = "sequenceDiagram
    participant navegador as Navegador del usuario
    participant puerta as Puerta de enlace de la API
    participant cola as Cola de trabajos en segundo plano
    navegador->>puerta: Solicita la exportación completa del historial de conversaciones en formato comprimido
    puerta->>cola: Encola el trabajo
    Note left of navegador: El usuario puede cerrar la pestaña: recibirá un correo cuando termine
    cola-->>navegador: Notifica por correo con el enlace de descarga, válido durante veinticuatro horas
    Note over puerta: Registra la auditoría
    rect rgb(40, 40, 60)
        puerta->>puerta: Limpia los archivos temporales
        activate cola
        cola->>cola: Comprime
        deactivate cola
    end";

const REALISTIC: [&str; 4] = [LOGIN, CHECKOUT, AGENT, EXPORT];

fn measured(graph: &SequenceGraph, scale: f32) -> SequenceLayout {
    graph.layout(&|text, size| (text.chars().count() as f32 * size * scale).round(), &|size| size * 1.3)
}

fn layout(source: &str) -> (SequenceGraph, SequenceLayout) {
    let graph = SequenceGraph::parse(source).unwrap();
    let layout = measured(&graph, 0.55);
    (graph, layout)
}

fn right(rect: Rect) -> f32 {
    rect.x + rect.width
}

fn bottom(rect: Rect) -> f32 {
    rect.y + rect.height
}

/// `inner` lies within `outer`.
fn within(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x && inner.y >= outer.y && right(inner) <= right(outer) && bottom(inner) <= bottom(outer)
}

/// Nothing leaves the chart, no text shares its place with other text, a
/// note or a box, no label or note meets a lifeline that is not its own,
/// and a block holds what it covers whole.
fn assert_sound(source: &str, graph: &SequenceGraph, layout: &SequenceLayout) {
    let chart = Rect::new(0.0, 0.0, layout.size.width, layout.size.height);
    assert_eq!(layout.participants.len(), graph.participants.len(), "{source}");
    assert_eq!(layout.messages.len(), graph.messages().count(), "{source}");
    let lifelines: Vec<f32> = layout.participants.iter().map(|participant| participant.x).collect();
    assert!(lifelines.windows(2).all(|pair| pair[0] < pair[1]), "{source}: lifelines out of order");
    let (rows_top, rows_bottom) = (bottom(layout.participants[0].head), layout.participants[0].foot.y);

    // Text, notes and boxes: each with a name for the failure.
    let mut solid: Vec<(String, Rect)> = Vec::new();
    for (index, participant) in layout.participants.iter().enumerate() {
        assert!(!participant.lines.is_empty(), "{source}: participant {index} has no name");
        assert!((participant.x - (participant.head.x + participant.head.width / 2.0)).abs() < 0.01);
        assert_eq!(bottom(participant.head), rows_top, "{source}: heads end level");
        assert_eq!(participant.foot.y, rows_bottom);
        solid.push((format!("head {index}"), participant.head));
        solid.push((format!("foot {index}"), participant.foot));
    }
    for (index, (message, placed)) in graph.messages().zip(&layout.messages).enumerate() {
        let (first, last) = (placed.points[0], placed.points[placed.points.len() - 1]);
        assert!(placed.points.iter().all(|point| point.x.is_finite() && point.y.is_finite()));
        assert!(first.y > rows_top && last.y < rows_bottom, "{source}: message {index} leaves the rows");
        // An end is on its lifeline or on the side of the bar there.
        for (point, participant) in [(first, message.from), (last, message.to)] {
            let reach = (point.x - lifelines[participant]).abs();
            assert!(reach <= BAR / 2.0 + 3.0 * BAR_STEP, "{source}: message {index} ends {reach} off its lifeline");
        }
        assert_eq!(placed.label.is_some(), !message.text.is_empty());
        assert_eq!(placed.number.is_some(), message.number.is_some());
        if let Some((chip, lines)) = &placed.label {
            assert!(!lines.is_empty());
            let (lo, hi) = (message.from.min(message.to), message.from.max(message.to));
            for (participant, &x) in lifelines.iter().enumerate() {
                let crossed = chip.x <= x && x <= right(*chip);
                // Between two lifelines the text may cover the ones it passes.
                let passes = lo < participant && participant < hi;
                assert!(!crossed || passes, "{source}: the text of message {index} meets lifeline {participant}");
            }
            solid.push((format!("text of message {index}"), *chip));
        }
    }
    let notes = graph.events.iter().filter_map(|event| match event {
        SequenceEvent::Note { place, .. } => Some(*place),
        _ => None,
    });
    for (index, (place, note)) in notes.zip(&layout.notes).enumerate() {
        assert!(!note.lines.is_empty());
        let over = match place {
            SequenceNotePlace::Over(lo, hi) => lo..=hi,
            // Never met: a note beside a participant is over none.
            _ => 1..=0,
        };
        for (participant, &x) in lifelines.iter().enumerate() {
            let crossed = note.rect.x <= x && x <= right(note.rect);
            assert_eq!(crossed, over.contains(&participant), "{source}: note {index} and lifeline {participant}");
        }
        solid.push((format!("note {index} {place:?}"), note.rect));
    }
    for (index, block) in layout.blocks.iter().enumerate() {
        assert!(block.rect.width > 0.0 && block.rect.height > 0.0);
        assert!(within(block.tab, block.rect), "{source}: block {index} loses its tab");
        for (label, lines) in &block.labels {
            assert!(!lines.is_empty());
            assert!(within(*label, block.rect), "{source}: block {index} loses a label at {label:?}");
            // A measurer that gives every text no width leaves nothing to overlap.
            if label.width > 0.0 {
                solid.push((format!("label of block {index}"), *label));
            }
        }
        if block.tab.width > 0.0 {
            solid.push((format!("tab of block {index}"), block.tab));
        }
        for divider in &block.dividers {
            assert!(*divider > block.rect.y && *divider < bottom(block.rect));
        }
        // Its sides run clear of every lifeline and of the bars on them.
        for side in [block.rect.x, right(block.rect)] {
            for &x in &lifelines {
                assert!((side - x).abs() >= BAR / 2.0 + 4.0, "{source}: block {index} runs along a lifeline");
            }
        }
        for (name, rect) in &solid {
            let apart = block.rect.intersection(*rect).is_none();
            assert!(apart || within(*rect, block.rect), "{source}: block {index} cuts {name}");
        }
        for other in &layout.blocks[index + 1..] {
            let apart = block.rect.intersection(other.rect).is_none();
            assert!(apart || within(other.rect, block.rect), "{source}: blocks {index} cut each other");
        }
    }
    for (index, (name, rect)) in solid.iter().enumerate() {
        assert!(rect.width > 0.0 && rect.height > 0.0, "{source}: {name} is empty");
        assert!(within(*rect, chart), "{source}: {name} leaves the chart: {rect:?} in {chart:?}");
        for (other_name, other) in &solid[index + 1..] {
            assert!(rect.intersection(*other).is_none(), "{source}: {name} overlaps {other_name}");
        }
    }
    for bar in &layout.activations {
        assert!(bar.height > 0.0 && within(*bar, chart), "{source}: a bar leaves the chart");
        assert!(bar.y >= rows_top && bottom(*bar) <= rows_bottom, "{source}: a bar leaves the rows");
        assert!(lifelines.iter().any(|x| (bar.x + BAR / 2.0 - x).abs() <= 3.0 * BAR_STEP));
        // A note over a lifeline covers the bar there; one beside it does not.
        for (name, rect) in solid.iter().filter(|(name, _)| name.contains("Of(")) {
            assert!(bar.intersection(*rect).is_none(), "{source}: a bar under {name}");
        }
    }
}

#[test]
fn participants_arrows_and_activations_are_read() {
    let graph = SequenceGraph::parse(
        "sequenceDiagram\n  participant A as Alicia\n  actor B\n  A->>B: uno\n  B-->>A: dos<br/>líneas\n  A->C: tres\n  \
         C-->A:\n  A-xB: cinco\n  B--xA: seis\n  A-)+B: siete\n  B--)-A: ocho\n  Servicio web->>api-xml: nueve\n  \
         PARTICIPANT C as Tercero",
    )
    .unwrap();
    let participants: Vec<_> =
        graph.participants.iter().map(|one| (one.id.as_str(), one.label.as_str(), one.actor)).collect();
    assert_eq!(
        participants,
        [
            ("A", "Alicia", false),
            ("B", "B", true),
            ("C", "Tercero", false),
            ("Servicio web", "Servicio web", false),
            ("api-xml", "api-xml", false),
        ]
    );
    let messages: Vec<_> = graph
        .messages()
        .map(|one| (one.from, one.to, one.text.as_str(), one.dotted, one.arrow, one.activates, one.deactivates))
        .collect();
    assert_eq!(
        messages,
        [
            (0, 1, "uno", false, SequenceArrow::Filled, false, false),
            (1, 0, "dos\nlíneas", true, SequenceArrow::Filled, false, false),
            (0, 2, "tres", false, SequenceArrow::Plain, false, false),
            (2, 0, "", true, SequenceArrow::Plain, false, false),
            (0, 1, "cinco", false, SequenceArrow::Cross, false, false),
            (1, 0, "seis", true, SequenceArrow::Cross, false, false),
            (0, 1, "siete", false, SequenceArrow::Open, true, false),
            (1, 0, "ocho", true, SequenceArrow::Open, false, true),
            (3, 4, "nueve", false, SequenceArrow::Filled, false, false),
        ]
    );
    assert!(graph.messages().all(|message| message.number.is_none()));
}

#[test]
fn notes_blocks_numbers_and_the_title_are_read() {
    let graph = SequenceGraph::parse(
        "---\nconfig: x\n---\n%% un comentario\nsequenceDiagram;\n  title: El título\n  autonumber 10 5\n  A->>B: a\n  \
         Note right of A: derecha\n  note left of B: izquierda\n  NOTE OVER A: sobre\n  Note over B,A: ambos\n  \
         activate B\n  alt uno\n    A->>B: b\n  else dos\n    opt\n      A->>A: c\n    end\n  end\n  deactivate B\n  \
         par p\n    A->>B: d\n  and q\n    critical r\n      A->>B: e\n    option s\n      break t\n        A->>B: f\n      end\n    end\n  end\n  \
         rect rgb(0, 0, 0)\n    loop siempre\n      A->>B: g\n    end\n  end\n  autonumber off\n  A->>B: h",
    )
    .unwrap();
    assert_eq!(graph.title, "El título");
    assert_eq!(graph.messages().map(|message| message.number).collect::<Vec<_>>(), [
        Some(10),
        Some(15),
        Some(20),
        Some(25),
        Some(30),
        Some(35),
        Some(40),
        None
    ]);
    let shape: Vec<String> = graph
        .events
        .iter()
        .filter_map(|event| match event {
            SequenceEvent::Note { place, text } => Some(format!("{place:?} {text}")),
            SequenceEvent::Open { kind, label } => Some(format!("{kind:?}({label})")),
            SequenceEvent::Section { label } => Some(format!("|{label}")),
            SequenceEvent::Close => Some("end".into()),
            SequenceEvent::Activate(participant) => Some(format!("+{participant}")),
            SequenceEvent::Deactivate(participant) => Some(format!("-{participant}")),
            SequenceEvent::Message(_) => None,
        })
        .collect();
    assert_eq!(
        shape.join(" "),
        "RightOf(0) derecha LeftOf(1) izquierda Over(0, 0) sobre Over(0, 1) ambos +1 Alt(uno) |dos Opt() end end -1 \
         Par(p) |q Critical(r) |s Break(t) end end end Rect() Loop(siempre) end end"
    );
}

#[test]
fn what_is_not_understood_is_an_error() {
    for source in [
        "",
        "sequenceDiagram",
        "flowchart TD\n  A --> B",
        "sequenceDiagram extra\n  A->>B: x",
        "sequenceDiagram\n  A->>B",
        "sequenceDiagram\n  A=>>B: x",
        "sequenceDiagram\n  A<<->>B: x",
        "sequenceDiagram\n  ->>B: x",
        "sequenceDiagram\n  A->>: x",
        "sequenceDiagram\n  A->>B: x\n  end",
        "sequenceDiagram\n  loop x\n  A->>B: x",
        "sequenceDiagram\n  loop x\n  end\n  A->>B: x",
        "sequenceDiagram\n  A->>B: x\n  else y",
        "sequenceDiagram\n  loop x\n  A->>B: x\n  else y\n  end",
        "sequenceDiagram\n  alt x\n  A->>B: x\n  and y\n  end",
        "sequenceDiagram\n  A->>B: x\n  deactivate B",
        "sequenceDiagram\n  A->>B: x\n  B-->>-A: y",
        "sequenceDiagram\n  Note over A,B,C: x",
        "sequenceDiagram\n  Note above A: x",
        "sequenceDiagram\n  Note over A",
        "sequenceDiagram\n  Note over A:",
        "sequenceDiagram\n  participant",
        "sequenceDiagram\n  participant A@{ \"type\": \"boundary\" }",
        "sequenceDiagram\n  box Grupo\n  participant A\n  end\n  A->>B: x",
        "sequenceDiagram\n  create participant B\n  A->>B: x",
        "sequenceDiagram\n  A->>B: x\n  destroy B",
        "sequenceDiagram\n  links A: {\"a\": \"b\"}",
        "sequenceDiagram\n  autonumber uno\n  A->>B: x",
        "sequenceDiagram\n  autonumber 1 2 3\n  A->>B: x",
    ] {
        assert!(SequenceGraph::parse(source).is_err(), "{source:?} was accepted");
        let either = Diagram::parse(source, crate::ComponentPalette::neutral_dark());
        assert!(either.is_err() || source.starts_with("flowchart"));
    }
}

#[test]
fn the_examples_of_mermaids_own_documentation_are_drawn() {
    for body in [
        "Alice->>John: Hello John, how are you?\nJohn-->>Alice: Great!\nAlice-)John: See you later!",
        "actor Alice\nactor Bob\nAlice->>Bob: Hi Bob\nBob->>Alice: Hi Alice",
        "participant A as Alice\nparticipant J as John\nA->>J: Hello John, how are you?\nJ->>A: Great!",
        "Alice->>John: Hello John, how are you?\nactivate John\nJohn-->>Alice: Great!\ndeactivate John",
        "Alice->>+John: Hello John, how are you?\nAlice->>+John: John, can you hear me?\n\
         John-->>-Alice: Hi Alice, I can hear you!\nJohn-->>-Alice: I feel great!",
        "participant John\nNote right of John: Text in note",
        "Alice->John: Hello John, how are you?\nNote over Alice,John: A typical interaction<br/>But now in two lines",
        "Alice->John: Hello John, how are you?\nloop Every minute\n    John-->Alice: Great!\nend",
        "Alice->>Bob: Hello Bob, how are you?\nalt is sick\n    Bob->>Alice: Not so good :(\nelse is well\n    \
         Bob->>Alice: Feeling fresh like a daisy\nend\nopt Extra response\n    Bob->>Alice: Thanks for asking\nend",
        "par Alice to Bob\n    Alice->>Bob: Go help John\nand Alice to John\n    Alice->>John: I want this done today\n    \
         par John to Charlie\n        John->>Charlie: Can we do this today?\n    and John to Diana\n        \
         John->>Diana: Can you help us today?\n    end\nend",
        "critical Establish a connection to the DB\n    Service-->DB: connect\noption Network timeout\n    \
         Service-->Service: Log error\noption Credentials rejected\n    Service-->Service: Log different error\nend",
        "Consumer-->API: Book something\nAPI-->BookingService: Start booking process\n\
         break when the booking process fails\n    API-->Consumer: show failure\nend\nAPI-->BillingService: Start billing process",
        "participant Alice\nparticipant John\n\nrect rgb(191, 223, 255)\nnote right of Alice: Alice calls John.\n\
         Alice->>+John: Hello John, how are you?\nrect rgb(200, 150, 255)\nAlice->>+John: John, can you hear me?\n\
         John-->>-Alice: Hi Alice, I can hear you!\nend\nJohn-->>-Alice: I feel great!\nend\n\
         Alice ->>+ John: Did you want to go to the game tonight?\nJohn -->>- Alice: Yeah! See you there.",
        "Alice->>John: Hello John, how are you?\n%% this is a comment\nJohn-->>Alice: Great!",
        "autonumber\nAlice->>John: Hello John, how are you?\nloop HealthCheck\n    John->>John: Fight against hypochondria\nend\n\
         Note right of John: Rational thoughts!\nJohn-->>Alice: Great!\nJohn->>Bob: How about you?\nBob-->>John: Jolly good!",
    ] {
        let source = format!("sequenceDiagram\n{body}");
        let graph = SequenceGraph::parse(&source).unwrap_or_else(|error| panic!("{error}: {source}"));
        assert_sound(&source, &graph, &measured(&graph, 0.55));
    }
    // What it documents and this module does not draw is refused, not guessed.
    for body in [
        "box Aqua Group Description\nparticipant A\nend\nA->>B: x",
        "Alice->>Bob: Hello Bob, how are you ?\ncreate participant Carl\nAlice->>Carl: Hi Carl!",
        "Alice->>Bob: x\ndestroy Bob\nBob->>Alice: y",
        "Alice<<->>Bob: x",
        "participant Alice@{ \"type\" : \"boundary\" }\nAlice->>Bob: x",
        "participant Alice\nlink Alice: Dashboard @ https://dashboard.contoso.com/alice",
        "participant Alice\nlinks Alice: {\"Dashboard\": \"https://dashboard.contoso.com/alice\"}",
    ] {
        assert!(SequenceGraph::parse(&format!("sequenceDiagram\n{body}")).is_err(), "{body:?} was accepted");
    }
}

#[test]
fn a_source_past_its_bounds_is_refused_whole() {
    let chain =
        |count: usize| (0..count).map(|index| format!("\n P{index}->>P{}: x", index + 1)).collect::<String>();
    assert!(SequenceGraph::parse(&format!("sequenceDiagram{}", chain(MAX_PARTICIPANTS - 1))).is_ok());
    assert!(SequenceGraph::parse(&format!("sequenceDiagram{}", chain(MAX_PARTICIPANTS))).is_err());
    let many = |count: usize| format!("sequenceDiagram{}", "\n A->>B: x".repeat(count));
    assert!(SequenceGraph::parse(&many(MAX_EVENTS)).is_ok());
    assert!(SequenceGraph::parse(&many(MAX_EVENTS + 1)).is_err());
    let nested = |depth: usize| {
        format!("sequenceDiagram{}\n A->>B: x{}", "\n loop x".repeat(depth), "\n end".repeat(depth))
    };
    assert!(SequenceGraph::parse(&nested(MAX_DEPTH)).is_ok());
    assert!(SequenceGraph::parse(&nested(MAX_DEPTH + 1)).is_err());
    let long = |length: usize| format!("sequenceDiagram\n A->>B: {}", "ñ".repeat(length));
    assert!(SequenceGraph::parse(&long(MAX_LABEL_CHARS)).is_ok());
    assert!(SequenceGraph::parse(&long(MAX_LABEL_CHARS + 1)).is_err());
    let named = format!("sequenceDiagram\n {}->>B: x", "a".repeat(MAX_LABEL_CHARS + 1));
    assert!(SequenceGraph::parse(&named).is_err());
    let padded = format!("sequenceDiagram\n A->>B: x\n%% {}", "x".repeat(MAX_SOURCE_BYTES));
    assert!(SequenceGraph::parse(&padded).is_err());
    let numbered = format!("sequenceDiagram\n autonumber {0} {0}\n A->>B: x\n B->>A: y", u32::MAX);
    assert_eq!(
        SequenceGraph::parse(&numbered).unwrap().messages().map(|message| message.number).collect::<Vec<_>>(),
        [Some(u32::MAX), Some(u32::MAX)],
        "a number past the last one stays there"
    );
}

#[test]
fn no_cut_garbled_or_hostile_source_panics_the_parser_or_the_layout() {
    let hostile = [
        "sequenceDiagram\n\u{0}->>\u{0}: \u{0}",
        "sequenceDiagram\n -->>--x-)->: ::::",
        "sequenceDiagram\n A->>B: \u{202e}texto al revés\u{200b}\u{feff}",
        "sequenceDiagram\n 👩‍💻->>e\u{301}: 日本語 🧪<br/><br/><br/>",
        "sequenceDiagram\n Note over ,: x\n Note right of : y\n Note over A,: z",
        "sequenceDiagram\n participant  as \n actor as as as",
        "sequenceDiagram\n A->>+A: a\n A->>+A: b\n A->>+A: c\n A->>+A: d\n A-->>-A: e",
        "sequenceDiagram\r\n A->>B: uno\r\n\r\n B-->>A: dos\r\n",
        "sequenceDiagram\n\tA->>B:\tcon\ttabuladores",
        "sequenceDiagram\n title\n title:\n autonumber 0 0\n A->>B: x",
    ];
    let mut sources: Vec<String> = hostile.iter().map(|source| (*source).to_owned()).collect();
    for source in REALISTIC {
        // Every way a diagram can be cut while it is being written.
        sources.extend(source.char_indices().map(|(end, _)| source[..end].to_owned()));
        // Every line on its own, without, twice and out of order.
        let lines: Vec<&str> = source.lines().collect();
        for skip in 1..lines.len() {
            let mut without = lines.clone();
            let line = without.remove(skip);
            sources.push(without.join("\n"));
            sources.push(format!("sequenceDiagram\n{line}"));
            without.insert(1, line);
            sources.push(without.join("\n"));
            without.insert(1, line);
            sources.push(without.join("\n"));
        }
        // One character swapped for each of the ones the syntax turns on.
        for (at, character) in source.char_indices().step_by(7) {
            for swap in ["-", ">", ":", "+", ",", "\n", "x", ")", " "] {
                sources.push(format!("{}{swap}{}", &source[..at], &source[at + character.len_utf8()..]));
            }
        }
    }
    let mut drawn = 0;
    for source in &sources {
        let Ok(graph) = SequenceGraph::parse(source) else {
            continue;
        };
        drawn += 1;
        for scale in [0.0, 0.55, 40.0] {
            assert_sound(source, &graph, &measured(&graph, scale));
        }
        // A measurer that fails still leaves every number finite.
        let broken = graph.layout(&|_, _| f32::NAN, &|_| f32::INFINITY);
        assert!(broken.size.width.is_finite() && broken.size.height.is_finite());
        assert_sound(source, &graph, &broken);
    }
    assert!(drawn > 200, "only {drawn} of {} sources were drawn", sources.len());
}

#[test]
fn a_graph_built_by_hand_with_loose_ends_is_still_laid_out() {
    let message = |from, to| {
        SequenceEvent::Message(SequenceMessage {
            from,
            to,
            text: "hola".into(),
            dotted: false,
            arrow: SequenceArrow::Filled,
            activates: true,
            deactivates: true,
            number: Some(u32::MAX),
        })
    };
    let graph = SequenceGraph {
        title: String::new(),
        participants: vec![SequenceParticipant { id: "a".into(), label: String::new(), actor: true }],
        events: vec![
            SequenceEvent::Close,
            SequenceEvent::Section { label: "suelta".into() },
            SequenceEvent::Deactivate(9),
            message(7, 3),
            SequenceEvent::Note { place: SequenceNotePlace::Over(5, 2), text: String::new() },
            SequenceEvent::Open { kind: SequenceBlockKind::Alt, label: "abierta".into() },
            message(0, 0),
            SequenceEvent::Open { kind: SequenceBlockKind::Loop, label: String::new() },
            SequenceEvent::Close,
            SequenceEvent::Activate(4),
        ],
    };
    let layout = measured(&graph, 0.55);
    assert!(layout.size.width > 0.0 && layout.size.height > 0.0);
    assert_eq!(layout.messages.len(), 2);
    // Neither the block left open nor the one with nothing in it is drawn.
    assert!(layout.blocks.is_empty());
    let empty = SequenceGraph { title: "t".into(), participants: Vec::new(), events: vec![message(0, 0)] };
    assert_eq!(measured(&empty, 0.55), SequenceLayout::default());
}

#[test]
fn nothing_overlaps_on_realistic_diagrams_at_any_text_width() {
    for source in REALISTIC {
        let graph = SequenceGraph::parse(source).unwrap();
        for scale in [0.47, 0.5, 0.55, 0.61, 0.9] {
            let layout = measured(&graph, scale);
            assert_sound(source, &graph, &layout);
            assert_eq!(layout, measured(&graph, scale), "{source}: the layout depends on more than its source");
        }
    }
}

#[test]
fn lifelines_stand_as_far_apart_as_the_text_between_them_needs() {
    let (_, short) = layout("sequenceDiagram\n A->>B: x\n B->>C: y");
    let gap = |layout: &SequenceLayout, first: usize, second: usize| {
        layout.participants[second].x - layout.participants[first].x
    };
    assert_eq!(gap(&short, 0, 1), gap(&short, 1, 2));
    let (_, long) = layout("sequenceDiagram\n A->>B: x\n B->>C: un texto bastante más largo que el otro");
    assert_eq!(gap(&long, 0, 1), gap(&short, 0, 1));
    assert!(gap(&long, 1, 2) > gap(&short, 1, 2) + 100.0);
    // Past its limit a text wraps instead of pushing further.
    let (_, wrapped) = layout(EXPORT);
    let (chip, lines) = wrapped.messages[0].label.as_ref().unwrap();
    assert!(lines.len() > 1 && chip.width <= MAX_TEXT_WIDTH + 2.0 * CHIP_PAD);
    let note = &wrapped.notes[0];
    assert!(note.lines.len() > 1 && note.rect.width <= MAX_NOTE_WIDTH + 2.0 * NOTE_PAD_X + 1.0);
    // A note beside the first participant makes room at the chart's edge.
    assert!(note.rect.x >= MARGIN && right(note.rect) < wrapped.participants[0].x);
    // A lifeline falls on a half pixel, where its one-pixel line is sharp.
    assert!(wrapped.participants.iter().all(|participant| participant.x.fract() == 0.5));
    assert!(wrapped.messages.iter().all(|message| message.points[0].y.fract() == 0.5));
}

#[test]
fn rows_follow_the_source_from_top_to_bottom() {
    let (graph, chart) = layout(AGENT);
    let rows: Vec<f32> = chart.messages.iter().map(|message| message.points[0].y).collect();
    assert!(rows.windows(2).all(|pair| pair[0] < pair[1]), "{rows:?}");
    assert!(chart.title.is_some_and(|title| title.y < chart.participants[0].head.y));
    // A message runs from its sender to its receiver, and one a
    // participant sends itself leaves and comes back on the right.
    for (message, placed) in graph.messages().zip(&chart.messages) {
        let (first, last) = (placed.points[0], placed.points[placed.points.len() - 1]);
        if message.from == message.to {
            let lifeline = chart.participants[message.from].x;
            assert!(last.y > first.y && placed.points.iter().all(|point| point.x >= lifeline));
        } else {
            assert_eq!(first.y, last.y);
            assert_eq!(last.x > first.x, message.to > message.from);
        }
    }
    // loop, alt and par nest; critical and break follow them.
    let kinds: Vec<_> = chart.blocks.iter().map(|block| block.kind.keyword()).collect();
    assert_eq!(kinds, ["loop", "alt", "par", "critical", "break"]);
    assert!(within(chart.blocks[1].rect, chart.blocks[0].rect));
    assert!(within(chart.blocks[2].rect, chart.blocks[1].rect));
    assert!(chart.blocks[3].rect.y > bottom(chart.blocks[0].rect));
    assert_eq!(chart.blocks[1].dividers.len(), 1);
    assert_eq!(chart.blocks[1].labels.iter().map(|(_, lines)| lines.join(" ")).collect::<Vec<_>>(), [
        "[Pide una herramienta]",
        "[Responde al usuario]"
    ]);
}

#[test]
fn an_activation_runs_between_its_arrows_and_takes_their_ends() {
    let (_, chart) = layout(
        "sequenceDiagram\n A->>+B: pide\n B->>+B: piensa\n B-->>-B: listo\n B-->>-A: responde\n A->>B: sigue",
    );
    let [outer, inner] = chart.activations[..] else { panic!("{:?}", chart.activations) };
    let (ask, answer) = (&chart.messages[0], &chart.messages[3]);
    // It reaches half its width past each arrow, so both meet its side.
    assert_eq!(outer.y, ask.points[0].y - BAR / 2.0);
    assert_eq!(bottom(outer), answer.points[0].y + BAR / 2.0);
    assert!(inner.y > outer.y && bottom(inner) < bottom(outer));
    assert_eq!(inner.x, outer.x + BAR_STEP, "a second bar sits on the first, a step to the right");
    // The arrow that starts the bar ends on its side, and so does the one
    // that leaves it; without a bar an arrow reaches the lifeline.
    assert_eq!(ask.points[1].x, outer.x);
    assert_eq!(answer.points[0].x, outer.x);
    assert_eq!(chart.messages[4].points[1].x, chart.participants[1].x);

    let (_, lines) = layout("sequenceDiagram\n A->>B: pide\n activate B\n B-->>A: responde\n deactivate B");
    let [bar] = lines.activations[..] else { panic!("{:?}", lines.activations) };
    let (asked, answered) = (lines.messages[0].points[0].y, lines.messages[1].points[0].y);
    assert_eq!((bar.y, bottom(bar)), (asked - BAR / 2.0, answered + BAR / 2.0));
    assert_eq!(lines.messages[0].points[1].x, bar.x, "`activate` after an arrow starts its bar at that arrow");
    // One left open ends with the diagram.
    let (_, open) = layout("sequenceDiagram\n A->>+B: pide\n B-->>A: responde");
    assert!(bottom(open.activations[0]) > open.messages[1].points[0].y);
}

#[test]
fn the_tree_paints_the_diagram_and_a_narrow_container_pans_instead_of_overflowing() {
    let palette = crate::ComponentPalette::neutral_dark();
    let chart = SequenceDiagram::parse(CHECKOUT, palette).unwrap();
    let natural = UiTree::new(chart.clone().render()).layout(Size::new(4_000.0, 4_000.0));
    let texts: Vec<&str> = natural
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let expected = ["Cliente", "Tienda web", "Confirma el carrito", "con el banco", "alt", "[Pago rechazado]", "loop", "9"];
    for expected in expected {
        assert!(texts.contains(&expected), "{expected} is not painted in {texts:?}");
    }
    // Each participant is named over its lifeline and again under it.
    assert_eq!(texts.iter().filter(|text| **text == "Pagos").count(), 2);
    let lines: Vec<bool> = natural
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Polyline { dashed, .. } => Some(*dashed),
            _ => None,
        })
        .collect();
    let dotted = chart.graph().messages().filter(|message| message.dotted).count();
    // The dotted messages and the line between the two branches of `alt`.
    assert_eq!(lines.iter().filter(|dashed| **dashed).count(), dotted + 1);
    assert!(natural.commands.iter().any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    assert!(natural.commands.iter().all(|command| match command {
        DrawCommand::Text { position, .. } => position.x >= 0.0 && position.y >= 0.0,
        _ => true,
    }));

    let narrow = UiTree::new(Element::column(vec![chart.scrollable("chart")])).layout(Size::new(240.0, 900.0));
    let clip = narrow
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::PushClip(rect) => Some(*rect),
            _ => None,
        })
        .expect("a wide diagram is clipped by its viewport");
    assert!(clip.x >= 0.0 && clip.x + clip.width <= 240.0);
}

#[test]
fn either_kind_is_read_from_its_header_and_an_oversized_source_is_refused() {
    let palette = crate::ComponentPalette::neutral_dark();
    assert!(matches!(Diagram::parse(LOGIN, palette), Ok(Diagram::Sequence(_))));
    assert!(matches!(Diagram::parse(&format!("%% nota\n{LOGIN}"), palette), Ok(Diagram::Sequence(_))));
    assert!(matches!(Diagram::parse("graph LR\n A --> B", palette), Ok(Diagram::Flowchart(_))));
    assert!(Diagram::parse("classDiagram\n A <|-- B", palette).is_err());
    let padded = format!("flowchart TD\n A --> B\n%% {}", "x".repeat(64 * 1024));
    assert!(Flowchart::parse(&padded, palette).is_ok() && Diagram::parse(&padded, palette).is_err());
}
