//! A flowchart drawn by the engine: the common subset of Mermaid's
//! `flowchart` syntax, a layered layout and the element that paints it.
//!
//! The host parses a source with [`FlowchartGraph::parse`] and falls back to
//! showing the source itself on an error. Nodes are ranked by longest path,
//! ordered within each rank by barycenter and joined by right-angle
//! connectors that leave and arrive at the middle of a side.

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{
    diagram::{arrowhead, error, odd, pannable, rounded, statements, text, unescape, LayoutCache, ARROW},
    typography::wrap_measured,
    Color, Component, ComponentPalette, DrawCommand, Element, Point, RadiusRole, Rect, Size, TextRole,
};

const MAX_NODES: usize = 120;
const MAX_EDGES: usize = 240;
const NODE_TEXT: f32 = TextRole::Label.size();
const EDGE_TEXT: f32 = TextRole::Small.size();
const NODE_PAD_X: f32 = 14.0;
const NODE_PAD_Y: f32 = 9.0;
const MAX_LABEL_WIDTH: f32 = 220.0;
const MAX_DECISION_LABEL_WIDTH: f32 = 150.0;
const NODE_GAP: f32 = 24.0;
const DUMMY_GAP: f32 = 14.0;
/// From a node's side to the first bend, between bends, and from the last
/// bend to the next node, along the direction of the flow.
const LEAD: f32 = 18.0;
const TRACK: f32 = 10.0;
const TAIL: f32 = 22.0;
/// How far off its node a neighbour may sit and still be joined by a straight line.
const SNAP: f32 = 16.0;
const GROUP_PAD: f32 = 10.0;
const MARGIN: f32 = 2.0;

/// Where the flow runs: `TD`/`TB`, `BT`, `LR` and `RL`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowDirection {
    Down,
    Up,
    Right,
    Left,
}

/// `A[rect]`, `A(rounded)`, `A([stadium])`, `A{decision}` and `A((circle))`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowNodeShape {
    Rect,
    Rounded,
    Stadium,
    Decision,
    Circle,
}

/// `-->`, `-.->` and `==>`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowEdgeLine {
    Solid,
    Dotted,
    Thick,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowNode {
    pub id: String,
    /// Lines are separated by `\n`.
    pub label: String,
    pub shape: FlowNodeShape,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowEdge {
    /// Indexes into [`FlowchartGraph::nodes`].
    pub from: usize,
    pub to: usize,
    /// Empty when the edge has none.
    pub label: String,
    pub line: FlowEdgeLine,
    /// `---` joins two nodes without pointing at either.
    pub arrow: bool,
}

/// A `subgraph … end`: a titled box around its nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowGroup {
    pub title: String,
    pub nodes: Vec<usize>,
}

/// Why a source is not a flowchart this module can draw.
pub type FlowchartError = crate::DiagramError;

/// A parsed flowchart: what to draw, before any measurement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowchartGraph {
    pub direction: FlowDirection,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub groups: Vec<FlowGroup>,
}

struct Parser {
    graph: FlowchartGraph,
    ids: HashMap<String, usize>,
    open_groups: Vec<usize>,
}

// Longest opener first; a pair that does not close falls through to the next.
const SHAPES: [(&str, &str, FlowNodeShape); 8] = [
    ("([", "])", FlowNodeShape::Stadium),
    ("((", "))", FlowNodeShape::Circle),
    ("[[", "]]", FlowNodeShape::Rect),
    ("[(", ")]", FlowNodeShape::Rounded),
    ("{{", "}}", FlowNodeShape::Rounded),
    ("(", ")", FlowNodeShape::Rounded),
    ("[", "]", FlowNodeShape::Rect),
    ("{", "}", FlowNodeShape::Decision),
];

fn clean_label(text: &str) -> String {
    unescape(text.trim().trim_matches('"'))
}

impl Parser {
    fn node(&mut self, id: &str, shaped: Option<(String, FlowNodeShape)>) -> usize {
        let index = *self.ids.entry(id.to_owned()).or_insert_with(|| {
            self.graph.nodes.push(FlowNode {
                id: id.to_owned(),
                label: id.to_owned(),
                shape: FlowNodeShape::Rect,
            });
            let index = self.graph.nodes.len() - 1;
            // A group holds the nodes of the groups nested in it too.
            for group in &self.open_groups {
                self.graph.groups[*group].nodes.push(index);
            }
            index
        });
        if let Some((label, shape)) = shaped {
            let node = &mut self.graph.nodes[index];
            if !label.is_empty() {
                node.label = label;
            }
            node.shape = shape;
        }
        index
    }

    fn parse_node(&mut self, rest: &mut &str) -> Result<usize, FlowchartError> {
        let text = rest.trim_start();
        let mut end = 0;
        let mut chars = text.char_indices().peekable();
        while let Some((at, character)) = chars.next() {
            let inner_dash = character == '-'
                && at > 0
                && chars.peek().is_some_and(|(_, next)| next.is_alphanumeric());
            if !(character.is_alphanumeric() || character == '_' || inner_dash) {
                break;
            }
            end = at + character.len_utf8();
        }
        if end == 0 {
            return error(format!("expected a node at `{text}`"));
        }
        let id = &text[..end];
        let mut after = &text[end..];
        let mut shaped = None;
        if after.starts_with(['(', '[', '{']) {
            for (open, close, shape) in SHAPES {
                let Some(body) = after.strip_prefix(open) else {
                    continue;
                };
                let found = match body.trim_start().strip_prefix('"') {
                    Some(quoted) => quoted.find('"').and_then(|end| {
                        Some((&quoted[..end], quoted[end + 1..].trim_start().strip_prefix(close)?))
                    }),
                    None => body.find(close).map(|end| (&body[..end], &body[end + close.len()..])),
                };
                if let Some((label, tail)) = found {
                    shaped = Some((clean_label(label), shape));
                    after = tail;
                    break;
                }
            }
            if shaped.is_none() {
                return error(format!("unclosed node shape at `{after}`"));
            }
        }
        if let Some(class) = after.strip_prefix(":::") {
            after = class.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_' || c == '-');
        }
        *rest = after;
        Ok(self.node(id, shaped))
    }

    fn parse_group(&mut self, rest: &mut &str) -> Result<Vec<usize>, FlowchartError> {
        let mut nodes = vec![self.parse_node(rest)?];
        while let Some(more) = rest.trim_start().strip_prefix('&') {
            *rest = more;
            nodes.push(self.parse_node(rest)?);
        }
        Ok(nodes)
    }

    fn parse_statement(&mut self, mut rest: &str) -> Result<(), FlowchartError> {
        let mut sources = self.parse_group(&mut rest)?;
        loop {
            rest = rest.trim_start();
            if let Some(next) = rest.strip_prefix(';') {
                rest = next.trim_start();
                if rest.is_empty() {
                    return Ok(());
                }
                sources = self.parse_group(&mut rest)?;
                continue;
            }
            if rest.is_empty() {
                return Ok(());
            }
            let (line, arrow, label) = parse_edge(&mut rest)?;
            let targets = self.parse_group(&mut rest)?;
            for &from in &sources {
                for &to in &targets {
                    self.graph.edges.push(FlowEdge {
                        from,
                        to,
                        label: label.clone(),
                        line,
                        arrow,
                    });
                }
            }
            sources = targets;
        }
    }
}

/// The run of connector characters at the start of `text`, up to its arrow.
fn connector_len(text: &str) -> usize {
    let mut length = 0;
    for byte in text.bytes() {
        if !matches!(byte, b'-' | b'.' | b'=' | b'>') {
            break;
        }
        length += 1;
        if byte == b'>' {
            break;
        }
    }
    length
}

fn parse_edge(rest: &mut &str) -> Result<(FlowEdgeLine, bool, String), FlowchartError> {
    let text = *rest;
    let opener = &text[..connector_len(text)];
    let mut label = String::new();
    let (connector, mut after) = if matches!(opener, "--" | "-." | "==") {
        // `-- text -->`: the label sits inside the connector.
        let closer = match opener {
            "--" => "--",
            "-." => ".-",
            _ => "==",
        };
        let body = &text[2..];
        let Some(end) = body.find(closer) else {
            return error(format!("unsupported connector at `{text}`"));
        };
        label = clean_label(&body[..end]);
        let tail = &body[end..];
        let length = connector_len(tail);
        (format!("{opener}{}", &tail[..length]), &tail[length..])
    } else if opener.len() >= 3 && opener.starts_with(['-', '=']) {
        (opener.to_owned(), &text[opener.len()..])
    } else {
        return error(format!("unsupported connector at `{text}`"));
    };
    if let Some(body) = after.trim_start().strip_prefix('|') {
        let Some(end) = body.find('|') else {
            return error("unclosed edge label");
        };
        label = clean_label(&body[..end]);
        after = &body[end + 1..];
    }
    *rest = after;
    let line = if connector.contains('.') {
        FlowEdgeLine::Dotted
    } else if connector.contains('=') {
        FlowEdgeLine::Thick
    } else {
        FlowEdgeLine::Solid
    };
    Ok((line, connector.ends_with('>'), label.replace('\n', " ")))
}

impl FlowchartGraph {
    /// Read a Mermaid `flowchart` or `graph`. Anything outside the supported
    /// subset is an error, never a guess: the host shows the source instead.
    pub fn parse(source: &str) -> Result<Self, FlowchartError> {
        let mut parser = Parser {
            graph: Self {
                direction: FlowDirection::Down,
                nodes: Vec::new(),
                edges: Vec::new(),
                groups: Vec::new(),
            },
            ids: HashMap::new(),
            open_groups: Vec::new(),
        };
        let mut lines = statements(source);
        let header = lines.next().unwrap_or_default().trim_end_matches(';');
        let mut words = header.split_whitespace();
        if !matches!(words.next(), Some("flowchart" | "graph")) {
            return error("not a flowchart");
        }
        parser.graph.direction = match words.next() {
            None | Some("TD" | "TB") => FlowDirection::Down,
            Some("BT") => FlowDirection::Up,
            Some("LR") => FlowDirection::Right,
            Some("RL") => FlowDirection::Left,
            Some(other) => return error(format!("unknown direction `{other}`")),
        };
        if words.next().is_some() {
            return error("unexpected text after the direction");
        }
        for line in lines {
            let (keyword, tail) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
            match keyword.trim_end_matches(';') {
                // Presentation that the host's own design replaces.
                "style" | "classDef" | "class" | "linkStyle" | "click" | "direction" => {}
                "subgraph" => {
                    let tail = tail.trim();
                    let title = match tail.split_once('[') {
                        Some((_, title)) => title.trim_end_matches(']'),
                        None => tail,
                    };
                    parser.open_groups.push(parser.graph.groups.len());
                    parser.graph.groups.push(FlowGroup {
                        title: clean_label(title).replace('\n', " "),
                        nodes: Vec::new(),
                    });
                }
                "end" if tail.is_empty() => {
                    if parser.open_groups.pop().is_none() {
                        return error("`end` without a subgraph");
                    }
                }
                _ => parser.parse_statement(line)?,
            }
            if parser.graph.nodes.len() > MAX_NODES || parser.graph.edges.len() > MAX_EDGES {
                return error("too large to draw");
            }
        }
        if parser.graph.nodes.is_empty() {
            return error("no nodes");
        }
        Ok(parser.graph)
    }

    /// Place every node, connector and label. `measure` gives the width of
    /// one line at a font size and `line_height` the height of a line at
    /// that size, both as the painter will draw them.
    pub fn layout(
        &self,
        measure: &dyn Fn(&str, f32) -> f32,
        line_height: &dyn Fn(f32) -> f32,
    ) -> FlowchartLayout {
        let horizontal = matches!(self.direction, FlowDirection::Right | FlowDirection::Left);
        let node_line = line_height(NODE_TEXT).ceil();
        let label_line = line_height(EDGE_TEXT).ceil();
        let count = self.nodes.len();

        let mut node_lines = Vec::with_capacity(count);
        let mut sizes = Vec::with_capacity(count);
        for node in &self.nodes {
            let limit = if node.shape == FlowNodeShape::Decision {
                MAX_DECISION_LABEL_WIDTH
            } else {
                MAX_LABEL_WIDTH
            };
            let lines: Vec<String> = wrap_measured(&node.label, limit, |part| measure(part, NODE_TEXT))
                .into_iter()
                .map(|line| line.trim().to_owned())
                .collect();
            let text_width = lines.iter().map(|line| measure(line, NODE_TEXT)).fold(0.0, f32::max);
            let text_height = lines.len().max(1) as f32 * node_line;
            let (width, height) = match node.shape {
                FlowNodeShape::Decision => {
                    // The text box touches all four sides of the diamond.
                    let half_height = text_height / 2.0 + 12.0;
                    let half_width =
                        (text_width / 2.0 + 4.0) / (1.0 - text_height / 2.0 / half_height);
                    (half_width * 2.0, half_height * 2.0)
                }
                FlowNodeShape::Circle => {
                    let diameter = text_width.hypot(text_height) + 14.0;
                    (diameter, diameter)
                }
                FlowNodeShape::Stadium => {
                    (text_width + 2.0 * NODE_PAD_X + 8.0, text_height + 2.0 * NODE_PAD_Y)
                }
                FlowNodeShape::Rect | FlowNodeShape::Rounded => {
                    (text_width + 2.0 * NODE_PAD_X, text_height + 2.0 * NODE_PAD_Y)
                }
            };
            node_lines.push(lines);
            sizes.push(Size::new(odd(width.max(40.0)), odd(height)));
        }

        // An edge that closes a cycle is laid out as if it ran the other way.
        let mut outgoing = vec![Vec::new(); count];
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.from != edge.to {
                outgoing[edge.from].push(index);
            }
        }
        let mut reversed = vec![false; self.edges.len()];
        let mut state = vec![0u8; count];
        let mut finished = Vec::with_capacity(count);
        for root in 0..count {
            if state[root] != 0 {
                continue;
            }
            state[root] = 1;
            let mut stack = vec![(root, 0usize)];
            while let Some((node, next)) = stack.last_mut() {
                let node = *node;
                let Some(&edge) = outgoing[node].get(*next) else {
                    state[node] = 2;
                    finished.push(node);
                    stack.pop();
                    continue;
                };
                *next += 1;
                let target = self.edges[edge].to;
                match state[target] {
                    0 => {
                        state[target] = 1;
                        stack.push((target, 0));
                    }
                    1 => reversed[edge] = true,
                    _ => {}
                }
            }
        }
        let ends = |index: usize| {
            let edge = &self.edges[index];
            if reversed[index] {
                (edge.to, edge.from)
            } else {
                (edge.from, edge.to)
            }
        };
        let flowing: Vec<usize> =
            (0..self.edges.len()).filter(|&index| self.edges[index].from != self.edges[index].to).collect();

        // Longest path from the sources; a source then drops next to its
        // nearest successor so its connector is not needlessly long.
        let mut rank = vec![0usize; count];
        for &node in finished.iter().rev() {
            for &edge in &flowing {
                let (from, to) = ends(edge);
                if from == node {
                    rank[to] = rank[to].max(rank[from] + 1);
                }
            }
        }
        for node in 0..count {
            let has_input = flowing.iter().any(|&edge| ends(edge).1 == node);
            let nearest = flowing
                .iter()
                .filter(|&&edge| ends(edge).0 == node)
                .map(|&edge| rank[ends(edge).1])
                .min();
            if let (false, Some(nearest)) = (has_input, nearest) {
                rank[node] = nearest - 1;
            }
        }

        // A connector across several ranks passes through one invisible,
        // sizeless vertex per rank, so it is routed around the nodes there.
        struct Vertex {
            rank: usize,
            cross: f32,
            main: f32,
            position: f32,
        }
        let mut vertices: Vec<Vertex> = (0..count)
            .map(|node| Vertex {
                rank: rank[node],
                cross: if horizontal { sizes[node].height } else { sizes[node].width },
                main: if horizontal { sizes[node].width } else { sizes[node].height },
                position: 0.0,
            })
            .collect();
        let label_size = |edge: &FlowEdge| {
            (!edge.label.is_empty())
                .then(|| Size::new(odd(measure(&edge.label, EDGE_TEXT) + 12.0), odd(label_line + 4.0)))
        };
        let mut above: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut below: Vec<Vec<usize>> = vec![Vec::new(); count];
        let mut chains: Vec<Vec<usize>> = vec![Vec::new(); self.edges.len()];
        for &edge in &flowing {
            let (from, to) = ends(edge);
            let label = label_size(&self.edges[edge]);
            let mut chain = vec![from];
            for step in rank[from] + 1..rank[to] {
                let labelled = label.filter(|_| step == rank[from] + 1);
                vertices.push(Vertex {
                    rank: step,
                    cross: labelled.map_or(1.0, |size| if horizontal { size.height } else { size.width }),
                    main: 0.0,
                    position: 0.0,
                });
                above.push(Vec::new());
                below.push(Vec::new());
                chain.push(vertices.len() - 1);
            }
            chain.push(to);
            for pair in chain.windows(2) {
                below[pair[0]].push(pair[1]);
                above[pair[1]].push(pair[0]);
            }
            chains[edge] = chain;
        }

        // Order each rank: depth-first to start, then barycenter sweeps,
        // keeping the order with the fewest crossings.
        let rank_count = vertices.iter().map(|vertex| vertex.rank).max().unwrap_or(0) + 1;
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); rank_count];
        let mut seen = vec![false; vertices.len()];
        for root in 0..count {
            let mut stack = vec![root];
            while let Some(vertex) = stack.pop() {
                if std::mem::replace(&mut seen[vertex], true) {
                    continue;
                }
                layers[vertices[vertex].rank].push(vertex);
                stack.extend(below[vertex].iter().rev());
            }
        }
        let mut index = vec![0usize; vertices.len()];
        let reindex = |layers: &[Vec<usize>], index: &mut [usize]| {
            for layer in layers {
                for (place, &vertex) in layer.iter().enumerate() {
                    index[vertex] = place;
                }
            }
        };
        reindex(&layers, &mut index);
        // shortcut: quadratic per rank, skipped above 400 vertices; count with a
        // Fenwick tree if graphs that large must also be untangled.
        let countable = vertices.len() <= 400;
        let crossings = |layers: &[Vec<usize>], index: &[usize]| {
            let mut total = 0usize;
            for layer in layers.iter().skip(1).filter(|_| countable) {
                let segments: Vec<(usize, usize)> = layer
                    .iter()
                    .flat_map(|&vertex| above[vertex].iter().map(move |&upper| (index[upper], index[vertex])))
                    .collect();
                for (at, first) in segments.iter().enumerate() {
                    total += segments[at + 1..]
                        .iter()
                        .filter(|second| {
                            (first.0 < second.0 && first.1 > second.1)
                                || (first.0 > second.0 && first.1 < second.1)
                        })
                        .count();
                }
            }
            total
        };
        let mut best = (crossings(&layers, &index), layers.clone());
        for round in 0..8 {
            let downward = round % 2 == 0;
            for step in 1..rank_count {
                let layer = if downward { step } else { rank_count - 1 - step };
                let neighbours = if downward { &above } else { &below };
                let mut keyed: Vec<(f32, usize)> = layers[layer]
                    .iter()
                    .enumerate()
                    .map(|(place, &vertex)| {
                        let near = &neighbours[vertex];
                        let key = if near.is_empty() {
                            place as f32
                        } else {
                            near.iter().map(|&other| index[other] as f32).sum::<f32>() / near.len() as f32
                        };
                        (key, vertex)
                    })
                    .collect();
                keyed.sort_by(|first, second| first.0.total_cmp(&second.0));
                layers[layer] = keyed.into_iter().map(|(_, vertex)| vertex).collect();
                reindex(&layers[layer..=layer], &mut index);
            }
            let crossed = crossings(&layers, &index);
            if crossed < best.0 {
                best = (crossed, layers.clone());
            }
        }
        let layers = best.1;

        // Across the flow: pack each rank, then pull every vertex toward the
        // mean of its neighbours while keeping the order and the gaps.
        let separation = |vertices: &[Vertex], first: usize, second: usize| {
            (vertices[first].cross + vertices[second].cross) / 2.0
                + if first >= count || second >= count { DUMMY_GAP } else { NODE_GAP }
        };
        for layer in &layers {
            let mut previous: Option<usize> = None;
            for &vertex in layer {
                vertices[vertex].position = match previous {
                    Some(previous) => {
                        vertices[previous].position + separation(&vertices, previous, vertex)
                    }
                    None => vertices[vertex].cross / 2.0,
                };
                previous = Some(vertex);
            }
        }
        // The last sweep runs with the flow, so a chain hangs straight under
        // its head and a node with several inputs sits between them.
        for round in 0..=20 {
            let downward = round % 2 == 0;
            for step in 0..rank_count {
                let layer = &layers[if downward { step } else { rank_count - 1 - step }];
                let wanted: Vec<f32> = layer
                    .iter()
                    .map(|&vertex| {
                        let near = if downward { &above[vertex] } else { &below[vertex] };
                        let (sum, amount) = near.iter().fold((0.0, 0.0), |(sum, amount), &other| {
                            (sum + vertices[other].position, amount + 1.0)
                        });
                        if amount == 0.0 {
                            vertices[vertex].position
                        } else {
                            sum / amount
                        }
                    })
                    .collect();
                // Pushing right and pushing left both keep the gaps, so does
                // their mean, and the mean leans to neither side.
                let mut pushed_right = wanted.clone();
                for place in 1..layer.len() {
                    pushed_right[place] = pushed_right[place].max(
                        pushed_right[place - 1] + separation(&vertices, layer[place - 1], layer[place]),
                    );
                }
                let mut pushed_left = wanted;
                for place in (0..layer.len().saturating_sub(1)).rev() {
                    pushed_left[place] = pushed_left[place].min(
                        pushed_left[place + 1] - separation(&vertices, layer[place], layer[place + 1]),
                    );
                }
                for (place, &vertex) in layer.iter().enumerate() {
                    vertices[vertex].position = (pushed_right[place] + pushed_left[place]) / 2.0;
                }
            }
        }
        for vertex in &mut vertices {
            vertex.position = vertex.position.round() + 0.5;
        }
        // A pair joined only to each other lines up when the neighbours of
        // the one that moves leave room: first toward the successor, so a
        // wide rank below spreads the rank above it, then toward the
        // predecessor.
        for upward in [true, false] {
            for step in 0..rank_count {
                let layer = &layers[if upward { rank_count - 1 - step } else { step }];
                for (place, &vertex) in layer.iter().enumerate() {
                    let (toward, back) = if upward { (&below, &above) } else { (&above, &below) };
                    let [other] = toward[vertex][..] else {
                        continue;
                    };
                    if back[other].len() != 1 {
                        continue;
                    }
                    let target = vertices[other].position;
                    let clear_left = place.checked_sub(1).is_none_or(|left| {
                        vertices[layer[left]].position + separation(&vertices, layer[left], vertex) <= target
                    });
                    let clear_right = layer.get(place + 1).is_none_or(|&right| {
                        target + separation(&vertices, vertex, right) <= vertices[right].position
                    });
                    if clear_left && clear_right {
                        vertices[vertex].position = target;
                    }
                }
            }
        }

        // Along the flow: each rank is as deep as its deepest node, and each
        // gap has one track per group of bends that would otherwise overlap.
        let mut tracks: Vec<HashMap<usize, usize>> = vec![HashMap::new(); rank_count];
        let mut track_counts = vec![0usize; rank_count];
        for (layer_index, layer) in layers.iter().enumerate() {
            let mut bends: Vec<(f32, f32, usize)> = layer
                .iter()
                .filter_map(|&vertex| {
                    let own = vertices[vertex].position;
                    let reach = below[vertex].iter().map(|&other| vertices[other].position);
                    let low = reach.clone().fold(own, f32::min);
                    let high = reach.fold(own, f32::max);
                    (high - low >= 0.5).then_some((low, high, vertex))
                })
                .collect();
            bends.sort_by(|first, second| first.0.total_cmp(&second.0));
            // The far end of the last bend on each track, and whether it is
            // where a connector arrives rather than where one leaves.
            let mut ends_at: Vec<(f32, bool)> = Vec::new();
            for (low, high, vertex) in bends {
                let own = vertices[vertex].position;
                let free = ends_at.iter().position(|&(end, arrives)| {
                    end + 8.0 < low || ((end - low).abs() < 0.5 && arrives && own > low + 0.5)
                });
                let track = free.unwrap_or(ends_at.len());
                if track == ends_at.len() {
                    ends_at.push((high, own < high - 0.5));
                } else {
                    ends_at[track] = (high, own < high - 0.5);
                }
                tracks[layer_index].insert(vertex, track);
            }
            track_counts[layer_index] = ends_at.len();
        }
        let mut label_zone = vec![0.0f32; rank_count];
        for &edge in &flowing {
            if let Some(size) = label_size(&self.edges[edge]) {
                let zone = &mut label_zone[vertices[chains[edge][0]].rank];
                *zone = zone.max(if horizontal { size.width } else { size.height } + 8.0);
            }
        }
        let mut start = vec![0.0f32; rank_count];
        let mut depth = vec![0.0f32; rank_count];
        for vertex in &vertices {
            depth[vertex.rank] = depth[vertex.rank].max(vertex.main);
        }
        let mut cursor = 0.0;
        for layer in 0..rank_count {
            start[layer] = cursor;
            cursor += depth[layer];
            if layer + 1 < rank_count {
                cursor += LEAD
                    + track_counts[layer].saturating_sub(1) as f32 * TRACK
                    + label_zone[layer]
                    + TAIL;
            }
        }
        let total = cursor;
        let middle = |vertex: usize| start[vertices[vertex].rank] + depth[vertices[vertex].rank] / 2.0;
        let track_at = |layer: usize, track: usize| start[layer] + depth[layer] + LEAD + track as f32 * TRACK + 0.5;
        let to_screen = |cross: f32, main: f32| match self.direction {
            FlowDirection::Down => Point::new(cross, main),
            FlowDirection::Up => Point::new(cross, total - main),
            FlowDirection::Right => Point::new(main, cross),
            FlowDirection::Left => Point::new(total - main, cross),
        };

        let mut nodes: Vec<FlowNodeLayout> = (0..count)
            .map(|node| {
                let center = to_screen(vertices[node].position, middle(node));
                FlowNodeLayout {
                    rect: Rect::new(
                        center.x - sizes[node].width / 2.0,
                        center.y - sizes[node].height / 2.0,
                        sizes[node].width,
                        sizes[node].height,
                    ),
                    lines: std::mem::take(&mut node_lines[node]),
                }
            })
            .collect();
        // A neighbour a few pixels off its node would bend the connector
        // twice within those pixels. The node's port moves under or over
        // that neighbour instead, so the line runs straight; every other
        // connector of the node shares the port.
        let snaps = |from: f32, to: f32| (0.5..SNAP).contains(&(from - to).abs());
        let mut leaves: Vec<f32> = (0..count).map(|node| vertices[node].position).collect();
        let mut arrives = leaves.clone();
        for chain in chains.iter().filter(|chain| chain.len() > 1) {
            let (first, next) = (chain[0], chain[1]);
            if snaps(vertices[first].position, vertices[next].position)
                && leaves[first] == vertices[first].position
            {
                leaves[first] = vertices[next].position;
            }
        }
        for chain in chains.iter().filter(|chain| chain.len() > 1) {
            let (last, before) = (chain[chain.len() - 1], chain[chain.len() - 2]);
            let from = if chain.len() == 2 { leaves[before] } else { vertices[before].position };
            // An arrowhead must touch the outline: only a box takes it off center.
            let boxed = !matches!(self.nodes[last].shape, FlowNodeShape::Decision | FlowNodeShape::Circle);
            if boxed && snaps(from, vertices[last].position) && arrives[last] == vertices[last].position {
                arrives[last] = from;
            }
        }
        let mut edges: Vec<FlowEdgeLayout> = (0..self.edges.len())
            .map(|edge| {
                let chain = &chains[edge];
                let Some(&first) = chain.first() else {
                    // shortcut: an edge from a node to itself is not drawn;
                    // route a loop beside the node if it is ever needed.
                    return FlowEdgeLayout { points: Vec::new(), label: None };
                };
                // Off center, a diamond or a circle has no outline to leave
                // from: the line starts under the node, which is drawn over it.
                let pointed = matches!(self.nodes[first].shape, FlowNodeShape::Decision | FlowNodeShape::Circle);
                // A connector drawn against the flow has its head at this end.
                let mut cross = if pointed && reversed[edge] { vertices[first].position } else { leaves[first] };
                let covered = pointed && cross != vertices[first].position;
                let mut path = vec![to_screen(
                    cross,
                    middle(first) + if covered { 0.0 } else { vertices[first].main / 2.0 },
                )];
                let mut label_cross = cross;
                for (step, pair) in chain.windows(2).enumerate() {
                    let (from, to) = (&vertices[pair[0]], &vertices[pair[1]]);
                    let target = if step + 2 == chain.len() { arrives[pair[1]] } else { to.position };
                    if (cross - target).abs() >= 0.5 {
                        // A bend that the node's own track cannot give falls on the first.
                        let track = track_at(from.rank, tracks[from.rank].get(&pair[0]).copied().unwrap_or(0));
                        path.push(to_screen(cross, track));
                        path.push(to_screen(target, track));
                        cross = target;
                    }
                    if step == 0 {
                        label_cross = cross;
                    }
                    path.push(to_screen(cross, middle(pair[1]) - to.main / 2.0));
                }
                if reversed[edge] {
                    path.reverse();
                }
                let label = label_size(&self.edges[edge]).map(|size| {
                    let layer = vertices[first].rank;
                    let along = track_at(layer, track_counts[layer].saturating_sub(1))
                        + 3.5
                        + if horizontal { size.width } else { size.height } / 2.0;
                    let center = to_screen(label_cross, along);
                    (
                        Rect::new(
                            center.x - size.width / 2.0,
                            center.y - size.height / 2.0,
                            size.width,
                            size.height,
                        ),
                        self.edges[edge].label.clone(),
                    )
                });
                FlowEdgeLayout { points: rounded(&path), label }
            })
            .collect();

        // A box that would cover a node outside its group is left out: the
        // layout does not keep a group together.
        let title_strip = label_line + 6.0;
        let mut groups: Vec<FlowGroupLayout> = self
            .groups
            .iter()
            .filter_map(|group| {
                let mut members = group.nodes.iter().map(|&node| nodes[node].rect);
                let first = members.next()?;
                let (left, top, right, bottom) = members.fold(
                    (first.x, first.y, first.x + first.width, first.y + first.height),
                    |(left, top, right, bottom), rect| {
                        (
                            left.min(rect.x),
                            top.min(rect.y),
                            right.max(rect.x + rect.width),
                            bottom.max(rect.y + rect.height),
                        )
                    },
                );
                let title_width = measure(&group.title, EDGE_TEXT) + 2.0 * GROUP_PAD;
                let rect = Rect::new(
                    left - GROUP_PAD,
                    top - GROUP_PAD - title_strip,
                    (right - left + 2.0 * GROUP_PAD).max(title_width),
                    bottom - top + 2.0 * GROUP_PAD + title_strip,
                );
                let covers_stranger = nodes.iter().enumerate().any(|(node, layout)| {
                    !group.nodes.contains(&node) && rect.intersection(layout.rect).is_some()
                });
                (!covers_stranger).then(|| FlowGroupLayout { rect, title: group.title.clone() })
            })
            .collect();

        let boxes = nodes
            .iter()
            .map(|node| node.rect)
            .chain(groups.iter().map(|group| group.rect))
            .chain(edges.iter().filter_map(|edge| edge.label.as_ref().map(|label| label.0)));
        let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for rect in boxes {
            left = left.min(rect.x);
            top = top.min(rect.y);
            right = right.max(rect.x + rect.width);
            bottom = bottom.max(rect.y + rect.height);
        }
        for point in edges.iter().flat_map(|edge| &edge.points) {
            left = left.min(point.x);
            top = top.min(point.y);
            right = right.max(point.x);
            bottom = bottom.max(point.y);
        }
        let (dx, dy) = ((MARGIN - left).round(), (MARGIN - top).round());
        let shift = |rect: &mut Rect| {
            rect.x += dx;
            rect.y += dy;
        };
        nodes.iter_mut().for_each(|node| shift(&mut node.rect));
        groups.iter_mut().for_each(|group| shift(&mut group.rect));
        for edge in &mut edges {
            for point in &mut edge.points {
                *point = Point::new(point.x + dx, point.y + dy);
            }
            if let Some((rect, _)) = &mut edge.label {
                shift(rect);
            }
        }
        FlowchartLayout {
            size: Size::new((right + dx + MARGIN).ceil(), (bottom + dy + MARGIN).ceil()),
            nodes,
            edges,
            groups,
        }
    }
}

/// One node's box and its wrapped label.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowNodeLayout {
    pub rect: Rect,
    pub lines: Vec<String>,
}

/// One connector, from the side of its source to the side of its target. An
/// edge from a node to itself has no points.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowEdgeLayout {
    pub points: Vec<Point>,
    /// The chip that carries the label, and its text.
    pub label: Option<(Rect, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowGroupLayout {
    pub rect: Rect,
    pub title: String,
}

/// Where everything in a [`FlowchartGraph`] goes, from a zero origin. Nodes
/// and edges keep the graph's order; a group that could not be boxed without
/// covering another node is absent.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowchartLayout {
    pub size: Size,
    pub nodes: Vec<FlowNodeLayout>,
    pub edges: Vec<FlowEdgeLayout>,
    pub groups: Vec<FlowGroupLayout>,
}

#[derive(Debug)]
struct Shared {
    graph: FlowchartGraph,
    layout: LayoutCache<FlowchartLayout>,
}

/// A flowchart at its natural size, in the palette's own inks: nodes on
/// `surface` with a `border` outline, connectors and their labels in
/// `muted`. Wider than its container, [`Flowchart::scrollable`] pans it.
#[derive(Clone, Debug)]
pub struct Flowchart {
    shared: Arc<Shared>,
    palette: ComponentPalette,
    backdrop: Color,
}

impl Flowchart {
    /// `graph` drawn on the palette's `raised` fill, as inside a card.
    pub fn new(graph: FlowchartGraph, palette: ComponentPalette) -> Self {
        Self {
            shared: Arc::new(Shared { graph, layout: LayoutCache::new() }),
            palette,
            backdrop: palette.raised,
        }
    }

    /// Parse a Mermaid `flowchart` source and draw it.
    pub fn parse(source: &str, palette: ComponentPalette) -> Result<Self, FlowchartError> {
        Ok(Self::new(FlowchartGraph::parse(source)?, palette))
    }

    /// The fill the chart sits on, painted behind edge labels so a
    /// connector never runs through their text.
    pub fn backdrop(mut self, color: Color) -> Self {
        self.backdrop = color;
        self
    }

    pub fn graph(&self) -> &FlowchartGraph {
        &self.shared.graph
    }

    /// The chart in a horizontal viewport that fills the width it is given
    /// and pans when the chart is wider, with a fade at a cut edge.
    pub fn scrollable(self, id: impl Into<String>) -> Element {
        let (backdrop, bar) = (self.backdrop, self.palette.muted);
        pannable(self.render(), id, backdrop, bar)
    }

    pub(crate) fn palette(&self) -> ComponentPalette {
        self.palette
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        let graph = &self.shared.graph;
        graph.nodes.iter().map(|node| node.id.len() + node.label.len() + 64).sum::<usize>()
            + graph.edges.iter().map(|edge| edge.label.len() + 64).sum::<usize>()
    }

    pub(crate) fn layout(
        &self,
        measure: &dyn Fn(&str, f32) -> f32,
        line_height: &dyn Fn(f32) -> f32,
    ) -> Arc<FlowchartLayout> {
        let graph = &self.shared.graph;
        let mut key = DefaultHasher::new();
        for node in &graph.nodes {
            measure(&node.label, NODE_TEXT).to_bits().hash(&mut key);
        }
        for edge in &graph.edges {
            measure(&edge.label, EDGE_TEXT).to_bits().hash(&mut key);
        }
        (line_height(NODE_TEXT).to_bits(), line_height(EDGE_TEXT).to_bits()).hash(&mut key);
        self.shared.layout.get(key.finish(), || graph.layout(measure, line_height))
    }

    /// Paint into `content`, centered where it is larger than the chart.
    pub(crate) fn paint(
        &self,
        content: Rect,
        measure: &dyn Fn(&str, f32) -> f32,
        line_height: &dyn Fn(f32) -> f32,
        commands: &mut Vec<DrawCommand>,
    ) {
        let layout = self.layout(measure, line_height);
        let origin = Point::new(
            content.x + ((content.width - layout.size.width) / 2.0).max(0.0).floor(),
            content.y + ((content.height - layout.size.height) / 2.0).max(0.0).floor(),
        );
        let graph = &self.shared.graph;
        let palette = self.palette;
        let at = |point: Point| Point::new(origin.x + point.x, origin.y + point.y);
        let moved = |rect: Rect| Rect::new(origin.x + rect.x, origin.y + rect.y, rect.width, rect.height);
        let label_line = line_height(EDGE_TEXT).ceil();
        for group in &layout.groups {
            let rect = moved(group.rect);
            commands.push(DrawCommand::StrokeRect {
                rect,
                color: palette.border,
                width: 1.0,
                radius: RadiusRole::Card.px(),
            });
            text(commands, &group.title, rect.x + GROUP_PAD, rect.y + 6.0, EDGE_TEXT, palette.muted);
        }
        for (edge, path) in graph.edges.iter().zip(&layout.edges) {
            let [.., before, tip] = path.points[..] else {
                continue;
            };
            let length = (tip.x - before.x).hypot(tip.y - before.y).max(0.001);
            let (dx, dy) = ((tip.x - before.x) / length, (tip.y - before.y) / length);
            let mut points: Vec<Point> = path.points.iter().map(|point| at(*point)).collect();
            if edge.arrow {
                // The line stops inside the head so the tip stays pointed.
                let last = points.len() - 1;
                points[last] = at(Point::new(tip.x - dx * ARROW / 2.0, tip.y - dy * ARROW / 2.0));
                arrowhead(commands, at(tip), (dx, dy), palette.muted);
            }
            commands.push(DrawCommand::Polyline {
                points: points.into(),
                color: palette.muted,
                width: if edge.line == FlowEdgeLine::Thick { 2.0 } else { 1.0 },
                dashed: edge.line == FlowEdgeLine::Dotted,
            });
        }
        for path in &layout.edges {
            if let Some((rect, label)) = &path.label {
                let rect = moved(*rect);
                commands.push(DrawCommand::FillRect {
                    rect,
                    color: self.backdrop,
                    radius: RadiusRole::Control.px(),
                });
                text(
                    commands,
                    label,
                    rect.x + (rect.width - measure(label, EDGE_TEXT)) / 2.0,
                    rect.y + (rect.height - label_line) / 2.0,
                    EDGE_TEXT,
                    palette.muted,
                );
            }
        }
        let node_line = line_height(NODE_TEXT).ceil();
        for (node, placed) in graph.nodes.iter().zip(&layout.nodes) {
            let rect = moved(placed.rect);
            if node.shape == FlowNodeShape::Decision {
                // A decision is told apart by its outline, not by a color.
                let (middle_x, middle_y) = (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                let corners = [
                    Point::new(middle_x, rect.y),
                    Point::new(rect.x + rect.width, middle_y),
                    Point::new(middle_x, rect.y + rect.height),
                    Point::new(rect.x, middle_y),
                ];
                commands.push(DrawCommand::FillTriangle {
                    points: [corners[0], corners[1], corners[3]],
                    color: palette.surface,
                });
                commands.push(DrawCommand::FillTriangle {
                    points: [corners[2], corners[3], corners[1]],
                    color: palette.surface,
                });
                commands.push(DrawCommand::Polyline {
                    points: [corners[0], corners[1], corners[2], corners[3], corners[0]].into(),
                    color: palette.border,
                    width: 1.0,
                    dashed: false,
                });
            } else {
                let radius = match node.shape {
                    FlowNodeShape::Rect => RadiusRole::Control.px(),
                    FlowNodeShape::Rounded => RadiusRole::Panel.px(),
                    _ => rect.height / 2.0,
                }
                .min(rect.height / 2.0);
                commands.push(DrawCommand::FillRect { rect, color: palette.surface, radius });
                commands.push(DrawCommand::StrokeRect { rect, color: palette.border, width: 1.0, radius });
            }
            let top = rect.y + (rect.height - placed.lines.len() as f32 * node_line) / 2.0;
            for (row, line) in placed.lines.iter().enumerate() {
                text(
                    commands,
                    line,
                    rect.x + (rect.width - measure(line, NODE_TEXT)) / 2.0,
                    top + row as f32 * node_line,
                    NODE_TEXT,
                    palette.text,
                );
            }
        }
    }
}

impl Component for Flowchart {
    fn render(self) -> Element {
        Element::flowchart(self)
    }
}

#[cfg(test)]
#[path = "flowchart_tests.rs"]
mod tests;
