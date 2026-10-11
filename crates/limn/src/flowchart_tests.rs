use super::*;
use crate::{Size, UiTree};

const DEMO: &str = "flowchart TD
    A[Se inicializa la demo] --> B[workspace-preview.js prepara la experiencia]
    B --> C[workspace-dock.js gestiona paneles y pestañas]
    C --> D[Archivos, vistas y terminales de ejemplo]
    A --> E[Recorrido guiado automático o interacción manual]
    E --> F[El usuario explora una solicitud preparada]
    F --> G[Recorre planificación, aprobaciones y resultado de ejemplo]
    D --> H[El usuario cambia archivos o vistas del ejemplo]
    G --> I[La experiencia muestra un resultado simulado]
    H --> I";

const DOCS: &str = "flowchart LR
    A[Contenido en docs/sections/] --> B[docs/content.js organiza guías y navegación]
    B --> C[scripts/build-docs.mjs]
    C --> D[Genera páginas HTML estáticas]
    C --> E[Genera índice de búsqueda local]
    D --> F[Salida dentro de dist/]
    E --> F
    F --> G[El navegador muestra la documentación]";

const DECISION: &str = "graph TD
    Start([Inicio]) --> Check{¿Hay cambios?}
    Check -->|Sí| Build[Compilar]
    Check -- No --> Done((Fin))
    Build -.-> Test(Probar)
    Test ==> Done
    Test -->|falla| Check";

fn layout(source: &str) -> (FlowchartGraph, FlowchartLayout) {
    let graph = FlowchartGraph::parse(source).unwrap();
    let layout = graph.layout(&|text, size| text.chars().count() as f32 * size * 0.55, &|size| size * 1.3);
    (graph, layout)
}

fn on_border(point: Point, rect: Rect) -> bool {
    let inside = point.x >= rect.x - 0.6
        && point.x <= rect.x + rect.width + 0.6
        && point.y >= rect.y - 0.6
        && point.y <= rect.y + rect.height + 0.6;
    let on_side = (point.x - rect.x).abs() < 0.6
        || (point.x - rect.x - rect.width).abs() < 0.6
        || (point.y - rect.y).abs() < 0.6
        || (point.y - rect.y - rect.height).abs() < 0.6;
    inside && on_side
}

/// Every node is placed inside the chart, no two boxes overlap, and each
/// connector runs from the side of its source to the side of its target.
fn assert_sound(source: &str) -> FlowchartLayout {
    let (graph, layout) = layout(source);
    assert_eq!(layout.nodes.len(), graph.nodes.len());
    assert_eq!(layout.edges.len(), graph.edges.len());
    for (index, node) in layout.nodes.iter().enumerate() {
        let rect = node.rect;
        assert!(rect.width > 0.0 && rect.height > 0.0, "{source}: node {index} is empty");
        assert!(
            rect.x >= 0.0
                && rect.y >= 0.0
                && rect.x + rect.width <= layout.size.width
                && rect.y + rect.height <= layout.size.height,
            "{source}: node {index} leaves the chart"
        );
        assert!(!node.lines.is_empty());
        for other in &layout.nodes[index + 1..] {
            assert!(rect.intersection(other.rect).is_none(), "{source}: {rect:?} overlaps {:?}", other.rect);
        }
    }
    for (edge, path) in graph.edges.iter().zip(&layout.edges) {
        if edge.from == edge.to {
            continue;
        }
        let (first, last) = (path.points[0], path.points[path.points.len() - 1]);
        assert!(on_border(first, layout.nodes[edge.from].rect), "{source}: {first:?} leaves no side");
        assert!(on_border(last, layout.nodes[edge.to].rect), "{source}: {last:?} reaches no side");
        assert!(path.points.iter().all(|point| point.x.is_finite() && point.y.is_finite()));
    }
    layout
}

#[test]
fn the_two_reference_charts_parse_whole() {
    let demo = FlowchartGraph::parse(DEMO).unwrap();
    assert_eq!(demo.direction, FlowDirection::Down);
    assert_eq!((demo.nodes.len(), demo.edges.len()), (9, 9));
    assert_eq!(demo.nodes[1].label, "workspace-preview.js prepara la experiencia");
    assert!(demo.nodes.iter().all(|node| node.shape == FlowNodeShape::Rect));
    let docs = FlowchartGraph::parse(DOCS).unwrap();
    assert_eq!(docs.direction, FlowDirection::Right);
    assert_eq!((docs.nodes.len(), docs.edges.len()), (7, 7));
    assert_eq!(docs.nodes[0].label, "Contenido en docs/sections/");
}

#[test]
fn shapes_connectors_and_labels_are_read() {
    let graph = FlowchartGraph::parse(DECISION).unwrap();
    let shapes: Vec<_> = graph.nodes.iter().map(|node| (node.id.as_str(), node.shape)).collect();
    assert_eq!(
        shapes,
        [
            ("Start", FlowNodeShape::Stadium),
            ("Check", FlowNodeShape::Decision),
            ("Build", FlowNodeShape::Rect),
            ("Done", FlowNodeShape::Circle),
            ("Test", FlowNodeShape::Rounded),
        ]
    );
    let edges: Vec<_> =
        graph.edges.iter().map(|edge| (edge.from, edge.to, edge.label.as_str(), edge.line, edge.arrow)).collect();
    assert_eq!(
        edges,
        [
            (0, 1, "", FlowEdgeLine::Solid, true),
            (1, 2, "Sí", FlowEdgeLine::Solid, true),
            (1, 3, "No", FlowEdgeLine::Solid, true),
            (2, 4, "", FlowEdgeLine::Dotted, true),
            (4, 3, "", FlowEdgeLine::Thick, true),
            (4, 1, "falla", FlowEdgeLine::Solid, true),
        ]
    );
}

#[test]
fn chains_fan_outs_groups_and_ignored_lines() {
    let graph = FlowchartGraph::parse(
        "---\ntitle: x\n---\n%%{init: {}}%%\nflowchart RL;\n  %% a comment\n  a --> b --> c\n  a & b --- d-1\n  \
         subgraph one [\"Grupo uno\"]\n    direction LR\n    x[\"Con [corchetes]<br/>y salto\"]:::warm -. lento .-> y\n  end\n  \
         classDef warm fill:#f96\n  class x warm\n  style a fill:#f9f\n  linkStyle 0 stroke:#333\n  click a callback\n  \
         lone; d-1 == fuerte ==> x",
    )
    .unwrap();
    assert_eq!(graph.direction, FlowDirection::Left);
    let ids: Vec<_> = graph.nodes.iter().map(|node| node.id.as_str()).collect();
    assert_eq!(ids, ["a", "b", "c", "d-1", "x", "y", "lone"]);
    assert_eq!(graph.nodes[4].label, "Con [corchetes]\ny salto");
    let pairs: Vec<_> = graph.edges.iter().map(|edge| (edge.from, edge.to, edge.arrow)).collect();
    assert_eq!(pairs, [(0, 1, true), (1, 2, true), (0, 3, false), (1, 3, false), (4, 5, true), (3, 4, true)]);
    assert_eq!((graph.edges[4].label.as_str(), graph.edges[4].line), ("lento", FlowEdgeLine::Dotted));
    assert_eq!((graph.edges[5].label.as_str(), graph.edges[5].line), ("fuerte", FlowEdgeLine::Thick));
    assert_eq!(graph.groups, [FlowGroup { title: "Grupo uno".into(), nodes: vec![4, 5] }]);
}

#[test]
fn what_is_not_understood_is_an_error() {
    for source in [
        "",
        "sequenceDiagram\n  A->>B: hola",
        "flowchart XY\n  A --> B",
        "flowchart TD",
        "flowchart TD\n  A --o B",
        "flowchart TD\n  A[sin cerrar --> B",
        "flowchart TD\n  A --> B\n  end",
        "flowchart TD\n  A -->|sin cerrar B",
        "flowchart TD\n  A --> B extra",
    ] {
        assert!(FlowchartGraph::parse(source).is_err(), "{source:?} was accepted");
    }
    let many: String = (0..200).map(|index| format!("\n n{index} --> n{}", index + 1)).collect();
    assert!(FlowchartGraph::parse(&format!("flowchart TD{many}")).is_err());
}

#[test]
fn nodes_never_overlap_and_connectors_join_their_sides() {
    for source in [DEMO, DOCS, DECISION] {
        for direction in ["TD", "BT", "LR", "RL"] {
            let (header, body) = source.split_once('\n').unwrap();
            let kind = header.split_whitespace().next().unwrap();
            assert_sound(&format!("{kind} {direction}\n{body}"));
        }
    }
    assert_sound("flowchart TD\n a --> b --> c --> a\n c --> c\n d");
    assert_sound("flowchart LR\n a --> b & c & d & e --> f\n a --> f\n subgraph g\n b\n c\n end");
}

#[test]
fn a_rank_runs_across_the_flow_and_the_flow_follows_its_direction() {
    let (_, down) = layout(DEMO);
    // B and E both follow A: side by side, below it.
    assert_eq!(down.nodes[1].rect.y, down.nodes[4].rect.y);
    assert!(down.nodes[1].rect.y > down.nodes[0].rect.y + down.nodes[0].rect.height);
    let (_, right) = layout(DOCS);
    let middle = |node: &FlowNodeLayout| node.rect.x + node.rect.width / 2.0;
    assert_eq!(middle(&right.nodes[3]), middle(&right.nodes[4]));
    assert!(right.nodes[6].rect.x > right.nodes[5].rect.x + right.nodes[5].rect.width);
    let (_, up) = layout(&DEMO.replace("TD", "BT"));
    assert!(up.nodes[1].rect.y < up.nodes[0].rect.y);
}

#[test]
fn long_labels_wrap_and_edge_labels_get_a_chip_clear_of_the_nodes() {
    let (_, chart) = layout(DEMO);
    assert!(chart.nodes[6].lines.len() > 1);
    assert!(chart.nodes.iter().all(|node| node.rect.width <= MAX_LABEL_WIDTH + 2.0 * NODE_PAD_X + 2.0));
    let (graph, chart) = layout(DECISION);
    for (edge, path) in graph.edges.iter().zip(&chart.edges) {
        assert_eq!(path.label.is_some(), !edge.label.is_empty());
        if let Some((chip, text)) = &path.label {
            assert_eq!(text, &edge.label);
            assert!(chart.nodes.iter().all(|node| node.rect.intersection(*chip).is_none()));
        }
    }
}

#[test]
fn a_group_is_boxed_only_when_the_box_holds_no_stranger() {
    let (graph, chart) = layout("flowchart TD\n subgraph Etapa\n a --> b\n end\n b --> c");
    let [group] = &chart.groups[..] else { panic!("{:?}", chart.groups) };
    assert_eq!(group.title, "Etapa");
    for (index, node) in chart.nodes.iter().enumerate() {
        let inside = group.rect.intersection(node.rect) == Some(node.rect);
        assert_eq!(inside, graph.groups[0].nodes.contains(&index));
    }
    // a and c are grouped but b sits between them.
    let (_, split) = layout("flowchart TD\n a --> b --> c\n subgraph g\n a\n c\n end");
    assert!(split.groups.is_empty());
}

#[test]
fn the_tree_paints_every_node_and_a_narrow_container_pans_instead_of_overflowing() {
    let palette = crate::ComponentPalette::neutral_dark();
    let chart = Flowchart::parse(DOCS, palette).unwrap();
    let natural = UiTree::new(chart.clone().render()).layout(Size::new(4_000.0, 2_000.0));
    let texts = |frame: &crate::Frame| {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    for node in &chart.graph().nodes {
        let first_word = node.label.split_whitespace().next().unwrap();
        assert!(texts(&natural).contains(first_word), "{first_word} is not painted");
    }
    let lines =
        natural.commands.iter().filter(|command| matches!(command, DrawCommand::Polyline { .. })).count();
    assert_eq!(lines, chart.graph().edges.len());
    assert!(natural.commands.iter().any(
        |command| matches!(command, DrawCommand::StrokeRect { color, width, .. } if *color == palette.border && *width == 1.0)
    ));

    let narrow = UiTree::new(Element::column(vec![chart.scrollable("chart")])).layout(Size::new(320.0, 600.0));
    let clip = narrow
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::PushClip(rect) => Some(*rect),
            _ => None,
        })
        .expect("a wide chart is clipped by its viewport");
    assert!(clip.x >= 0.0 && clip.x + clip.width <= 320.0);
}
