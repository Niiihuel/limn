//! What the diagrams drawn by the engine share: the error a source is
//! refused with, the reading of a Mermaid source into statements, the
//! viewport a chart pans in and the marks every kind paints.

use std::{
    fmt,
    sync::{Arc, Mutex, PoisonError},
};

use crate::{
    Color, Component, ComponentPalette, DrawCommand, Element, Flowchart, FontFamily, Point, SequenceDiagram,
};

/// A longer source is not read: its host shows it as written.
pub(crate) const MAX_SOURCE_BYTES: usize = 64 * 1024;
/// The length of an arrowhead; a line stops half of it short of the tip.
pub(crate) const ARROW: f32 = 7.0;
const CORNER: f32 = 6.0;

/// Why a source is not a diagram the engine can draw.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagramError(String);

impl fmt::Display for DiagramError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DiagramError {}

pub(crate) fn error<T>(message: impl Into<String>) -> Result<T, DiagramError> {
    Err(DiagramError(message.into()))
}

/// The statements of a Mermaid source: its lines, trimmed, without the
/// blank ones, a front matter at the start or `%%` comments.
pub(crate) fn statements(source: &str) -> impl Iterator<Item = &str> {
    let mut lines = source.lines().map(str::trim).filter(|line| !line.is_empty()).peekable();
    if lines.peek() == Some(&"---") {
        lines.next();
        lines.by_ref().find(|line| *line == "---");
    }
    lines.filter(|line| !line.starts_with("%%"))
}

/// A label as Mermaid writes it: `<br/>` and `\n` break the line and the
/// common entities are decoded.
pub(crate) fn unescape(text: &str) -> String {
    let mut text = text.to_owned();
    for (from, to) in [
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("<br>", "\n"),
        ("<BR/>", "\n"),
        ("<BR>", "\n"),
        ("\\n", "\n"),
        ("#quot;", "\""),
        ("&quot;", "\""),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
    ] {
        text = text.replace(from, to);
    }
    text.lines().map(str::trim).collect::<Vec<_>>().join("\n")
}

/// Odd sizes put the middle of a side on a half pixel, where a one-pixel
/// line is sharp.
pub(crate) fn odd(value: f32) -> f32 {
    let value = value.ceil();
    if value % 2.0 == 0.0 {
        value + 1.0
    } else {
        value
    }
}

/// The layout for one set of measurements; a new font replaces it.
#[derive(Debug)]
pub(crate) struct LayoutCache<L>(Mutex<Option<(u64, Arc<L>)>>);

impl<L> LayoutCache<L> {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub(crate) fn get(&self, key: u64, build: impl FnOnce() -> L) -> Arc<L> {
        let mut cached = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match &*cached {
            Some((cached_key, layout)) if *cached_key == key => layout.clone(),
            _ => {
                let layout = Arc::new(build());
                *cached = Some((key, layout.clone()));
                layout
            }
        }
    }
}

/// `chart` in a horizontal viewport that fills the width it is given and
/// pans when the chart is wider, with a fade at a cut edge and a bar that
/// shows where it sits.
pub(crate) fn pannable(chart: Element, id: impl Into<String>, backdrop: Color, bar: Color) -> Element {
    // Room under the chart for the bar.
    let chart = Element::column(vec![chart.flex(1.0), Element::spacer().size(None, Some(8.0))]);
    Element::horizontal_scroll_2d(id, chart)
        .horizontal_edge_fade(backdrop, 16.0)
        .foreground(bar)
        .drag_to_pan()
}

pub(crate) fn text(commands: &mut Vec<DrawCommand>, value: &str, x: f32, y: f32, size: f32, color: Color) {
    commands.push(DrawCommand::Text {
        position: Point::new(x.round(), y.round()),
        text: value.to_owned(),
        color,
        font_size: size,
        font_family: FontFamily::Sans,
    });
}

/// A filled head whose point is `tip`, on a line that arrives along the
/// unit vector `(dx, dy)`.
pub(crate) fn arrowhead(commands: &mut Vec<DrawCommand>, tip: Point, (dx, dy): (f32, f32), color: Color) {
    let base = Point::new(tip.x - dx * ARROW, tip.y - dy * ARROW);
    commands.push(DrawCommand::FillTriangle {
        points: [
            tip,
            Point::new(base.x - dy * 3.5, base.y + dx * 3.5),
            Point::new(base.x + dy * 3.5, base.y - dx * 3.5),
        ],
        color,
    });
}

/// Drop repeated and collinear points, then turn each corner into a short
/// curve so a right-angle connector reads as one line.
pub(crate) fn rounded(path: &[Point]) -> Vec<Point> {
    let mut corners: Vec<Point> = Vec::with_capacity(path.len());
    for &point in path {
        if corners.last().is_some_and(|last| (last.x - point.x).abs() < 0.01 && (last.y - point.y).abs() < 0.01) {
            continue;
        }
        if let [.., before, last] = corners[..] {
            let straight = ((before.x - last.x).abs() < 0.01 && (last.x - point.x).abs() < 0.01)
                || ((before.y - last.y).abs() < 0.01 && (last.y - point.y).abs() < 0.01);
            if straight {
                corners.pop();
            }
        }
        corners.push(point);
    }
    let mut points = Vec::with_capacity(corners.len() * 4);
    for (index, &corner) in corners.iter().enumerate() {
        let (Some(&before), Some(&after)) =
            (index.checked_sub(1).and_then(|at| corners.get(at)), corners.get(index + 1))
        else {
            points.push(corner);
            continue;
        };
        let toward = |other: Point, radius: f32| {
            let length = (other.x - corner.x).hypot(other.y - corner.y);
            Point::new(
                corner.x + (other.x - corner.x) / length * radius,
                corner.y + (other.y - corner.y) / length * radius,
            )
        };
        let radius = CORNER
            .min((before.x - corner.x).hypot(before.y - corner.y) / 2.0)
            .min((after.x - corner.x).hypot(after.y - corner.y) / 2.0);
        let (from, to) = (toward(before, radius), toward(after, radius));
        for step in 0..=4 {
            let t = step as f32 / 4.0;
            let blend = |from: f32, corner: f32, to: f32| {
                (1.0 - t) * (1.0 - t) * from + 2.0 * (1.0 - t) * t * corner + t * t * to
            };
            points.push(Point::new(blend(from.x, corner.x, to.x), blend(from.y, corner.y, to.y)));
        }
    }
    points
}

/// A Mermaid diagram of any kind the engine draws, for a host that is
/// handed a source without knowing which it is.
#[derive(Clone, Debug)]
pub enum Diagram {
    /// A `flowchart` or a `graph`. `parse` reads as one every source that does not
    /// start with `sequenceDiagram`.
    Flowchart(Flowchart),
    /// A source whose first statement is `sequenceDiagram`.
    Sequence(SequenceDiagram),
}

impl Diagram {
    /// Read a `flowchart`, a `graph` or a `sequenceDiagram`. Another kind,
    /// a source over 64 KiB or one outside what its kind supports is an
    /// error, for the host to show the source instead.
    pub fn parse(source: &str, palette: ComponentPalette) -> Result<Self, DiagramError> {
        if source.len() > MAX_SOURCE_BYTES {
            return error("too large to draw");
        }
        match statements(source).next().and_then(|header| header.split_whitespace().next()) {
            Some(header) if header.trim_end_matches(';') == "sequenceDiagram" => {
                SequenceDiagram::parse(source, palette).map(Self::Sequence)
            }
            _ => Flowchart::parse(source, palette).map(Self::Flowchart),
        }
    }

    /// The fill the diagram sits on, painted behind the labels on its lines.
    pub fn backdrop(self, color: Color) -> Self {
        match self {
            Self::Flowchart(chart) => Self::Flowchart(chart.backdrop(color)),
            Self::Sequence(chart) => Self::Sequence(chart.backdrop(color)),
        }
    }

    /// The diagram in a horizontal viewport that fills the width it is
    /// given and pans when the diagram is wider.
    pub fn scrollable(self, id: impl Into<String>) -> Element {
        match self {
            Self::Flowchart(chart) => chart.scrollable(id),
            Self::Sequence(chart) => chart.scrollable(id),
        }
    }

    pub(crate) fn palette(&self) -> ComponentPalette {
        match self {
            Self::Flowchart(chart) => chart.palette(),
            Self::Sequence(chart) => chart.palette(),
        }
    }
}

impl From<Flowchart> for Diagram {
    fn from(chart: Flowchart) -> Self {
        Self::Flowchart(chart)
    }
}

impl From<SequenceDiagram> for Diagram {
    fn from(chart: SequenceDiagram) -> Self {
        Self::Sequence(chart)
    }
}

impl Component for Diagram {
    fn render(self) -> Element {
        match self {
            Self::Flowchart(chart) => chart.render(),
            Self::Sequence(chart) => chart.render(),
        }
    }
}
