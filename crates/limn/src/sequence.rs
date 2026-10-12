//! A sequence diagram drawn by the engine: the common subset of Mermaid's
//! `sequenceDiagram` syntax, one lifeline per participant with the messages
//! in the order written, and the element that paints it.
//!
//! The host parses a source with [`SequenceGraph::parse`] and falls back to
//! showing the source itself on an error. Lifelines stand as far apart as
//! the text between them needs, so no label, note or block meets a line
//! that is not its own.

use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    sync::Arc,
};

use crate::{
    diagram::{arrowhead, error, odd, pannable, rounded, statements, text, unescape, LayoutCache, ARROW},
    typography::wrap_measured,
    Color, Component, ComponentPalette, DiagramError, DrawCommand, Element, Point, RadiusRole, Rect, Size,
    TextRole,
};

const MAX_SOURCE_BYTES: usize = 32 * 1024;
const MAX_PARTICIPANTS: usize = 20;
const MAX_EVENTS: usize = 240;
const MAX_DEPTH: usize = 6;
const MAX_LABEL_CHARS: usize = 400;

const NAME_TEXT: f32 = TextRole::Label.size();
const TEXT: f32 = TextRole::Caption.size();
const SMALL_TEXT: f32 = TextRole::Small.size();
const NUMBER_TEXT: f32 = TextRole::Micro.size();
const MAX_NAME_WIDTH: f32 = 140.0;
const MAX_TEXT_WIDTH: f32 = 240.0;
const MAX_NOTE_WIDTH: f32 = 180.0;
const MAX_BLOCK_LABEL_WIDTH: f32 = 260.0;
const NAME_PAD_X: f32 = 14.0;
const NAME_PAD_Y: f32 = 9.0;
const MIN_HEAD_WIDTH: f32 = 64.0;
const HEAD_GAP: f32 = 28.0;
const FIGURE_WIDTH: f32 = 17.0;
const FIGURE_HEIGHT: f32 = 34.0;
const FIGURE_GAP: f32 = 2.0;
/// Between a message's text and the lifelines it runs between.
const TEXT_PAD: f32 = 12.0;
const NUMBERED_PAD: f32 = 16.0;
const CHIP_PAD: f32 = 3.0;
const ROW_GAP: f32 = 12.0;
const BAR: f32 = 9.0;
const BAR_STEP: f32 = 4.0;
const LOOP_WIDTH: f32 = 26.0;
const LOOP_HEIGHT: f32 = 18.0;
const NUMBER_SIZE: f32 = 15.0;
const NOTE_GAP: f32 = 10.0;
const NOTE_PAD_X: f32 = 8.0;
const NOTE_PAD_Y: f32 = 5.0;
/// How far a note over two participants reaches past their lifelines.
const NOTE_OVER: f32 = 14.0;
const BLOCK_PAD: f32 = 12.0;
const TAB_PAD_X: f32 = 7.0;
const TAB_PAD_Y: f32 = 3.0;
/// The least room between two things that do not belong together.
const CLEAR: f32 = 10.0;
const MARGIN: f32 = 2.0;

/// `participant` is drawn as a box and `actor` as a figure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceParticipant {
    pub id: String,
    /// The alias after `as`, or the id. Lines are separated by `\n`.
    pub label: String,
    pub actor: bool,
}

/// What a message's line ends in: `->>` a filled head, `->` nothing, `-x`
/// a cross and `-)` an open head.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceArrow {
    Filled,
    Plain,
    Cross,
    Open,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceMessage {
    /// Indexes into [`SequenceGraph::participants`]; the same for a message
    /// a participant sends itself.
    pub from: usize,
    pub to: usize,
    /// Empty when the message has none. Lines are separated by `\n`.
    pub text: String,
    /// A second dash (`-->>`) draws the line dotted.
    pub dotted: bool,
    pub arrow: SequenceArrow,
    /// `+` after the arrow starts an activation of the receiver.
    pub activates: bool,
    /// `-` after the arrow ends one of the sender.
    pub deactivates: bool,
    /// Its number under `autonumber`.
    pub number: Option<u32>,
}

/// Beside one participant, or over one or two (the lower index first).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceNotePlace {
    LeftOf(usize),
    RightOf(usize),
    Over(usize, usize),
}

/// `loop`, `alt`, `opt`, `par`, `critical`, `break` and `rect`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SequenceBlockKind {
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    /// A highlighted region: no keyword and no label.
    Rect,
}

impl SequenceBlockKind {
    fn keyword(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Alt => "alt",
            Self::Opt => "opt",
            Self::Par => "par",
            Self::Critical => "critical",
            Self::Break => "break",
            Self::Rect => "",
        }
    }
}

/// One statement, in the order written.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SequenceEvent {
    Message(SequenceMessage),
    Note { place: SequenceNotePlace, text: String },
    Activate(usize),
    Deactivate(usize),
    /// A block opens; `label` is the text after its keyword.
    Open { kind: SequenceBlockKind, label: String },
    /// `else`, `and` or `option`: the next section of the open block.
    Section { label: String },
    /// `end`.
    Close,
}

/// A parsed sequence diagram: what to draw, before any measurement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SequenceGraph {
    /// Empty when the source gives none.
    pub title: String,
    /// In the order declared or first named.
    pub participants: Vec<SequenceParticipant>,
    pub events: Vec<SequenceEvent>,
}

struct Parser {
    graph: SequenceGraph,
    ids: HashMap<String, usize>,
    /// Each open block, and whether a message or a note is inside it yet.
    open: Vec<(SequenceBlockKind, bool)>,
    active: Vec<usize>,
    /// The next number and the step between two, under `autonumber`.
    number: Option<(u32, u32)>,
}

fn label(text: &str) -> Result<String, DiagramError> {
    let text = unescape(text.trim());
    if text.chars().count() > MAX_LABEL_CHARS {
        return error("a label is too long to draw");
    }
    Ok(text)
}

/// A participant's name, or `None` for text that can only be another
/// statement read wrong.
fn name(text: &str) -> Option<&str> {
    let text = text.trim();
    let sound = !text.is_empty()
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains([':', ';', ',', '<', '>', '+', '{', '}'])
        && text.chars().count() <= MAX_LABEL_CHARS;
    sound.then_some(text)
}

impl Parser {
    fn participant(&mut self, id: &str) -> usize {
        *self.ids.entry(id.to_owned()).or_insert_with(|| {
            self.graph.participants.push(SequenceParticipant {
                id: id.to_owned(),
                label: id.to_owned(),
                actor: false,
            });
            self.active.push(0);
            self.graph.participants.len() - 1
        })
    }

    fn named(&mut self, text: &str) -> Result<usize, DiagramError> {
        match name(text) {
            Some(id) => Ok(self.participant(id)),
            None => error(format!("`{}` is not a participant", text.trim())),
        }
    }

    fn content(&mut self, event: SequenceEvent) {
        if let Some((_, filled)) = self.open.last_mut() {
            *filled = true;
        }
        self.graph.events.push(event);
    }

    fn message(&mut self, line: &str) -> Result<(), DiagramError> {
        // Longest first, so `-->>` is not read as `-->` and a stray `>`.
        const ARROWS: [(&str, bool, SequenceArrow); 8] = [
            ("-->>", true, SequenceArrow::Filled),
            ("->>", false, SequenceArrow::Filled),
            ("--x", true, SequenceArrow::Cross),
            ("--)", true, SequenceArrow::Open),
            ("-->", true, SequenceArrow::Plain),
            ("-x", false, SequenceArrow::Cross),
            ("-)", false, SequenceArrow::Open),
            ("->", false, SequenceArrow::Plain),
        ];
        let Some((head, words)) = line.split_once(':') else {
            return error(format!("unsupported statement `{line}`"));
        };
        // A name may hold a dash (`api-xml->>db`): the arrow is the first
        // place that leaves a sound name on both sides.
        for (at, _) in head.match_indices('-') {
            let Some(&(token, dotted, arrow)) = ARROWS.iter().find(|(token, ..)| head[at..].starts_with(token))
            else {
                continue;
            };
            let after = head[at + token.len()..].trim_start();
            let (activates, deactivates, to) = match after.as_bytes().first() {
                Some(b'+') => (true, false, &after[1..]),
                Some(b'-') => (false, true, &after[1..]),
                _ => (false, false, after),
            };
            let (Some(from), Some(to)) = (name(&head[..at]), name(to)) else {
                continue;
            };
            let text = label(words)?;
            let (from, to) = (self.participant(from), self.participant(to));
            if deactivates {
                if self.active[from] == 0 {
                    return error("`-` ends an activation that never started");
                }
                self.active[from] -= 1;
            }
            if activates {
                self.active[to] += 1;
            }
            let number = self.number.map(|(next, _)| next);
            if let Some((next, step)) = &mut self.number {
                *next = next.saturating_add(*step);
            }
            self.content(SequenceEvent::Message(SequenceMessage {
                from,
                to,
                text,
                dotted,
                arrow,
                activates,
                deactivates,
                number,
            }));
            return Ok(());
        }
        error(format!("unsupported statement `{line}`"))
    }

    fn note(&mut self, tail: &str) -> Result<(), DiagramError> {
        let Some((who, words)) = tail.split_once(':') else {
            return error("a note without text");
        };
        let who = who.trim();
        // ASCII lowercase keeps every byte where it was.
        let lower = who.to_ascii_lowercase();
        let place = if lower.starts_with("right of ") {
            SequenceNotePlace::RightOf(self.named(&who[9..])?)
        } else if lower.starts_with("left of ") {
            SequenceNotePlace::LeftOf(self.named(&who[8..])?)
        } else if lower.starts_with("over ") {
            let mut names = who[5..].split(',');
            let first = self.named(names.next().unwrap_or_default())?;
            let second = match names.next() {
                Some(other) => self.named(other)?,
                None => first,
            };
            if names.next().is_some() {
                return error("a note over more than two participants");
            }
            SequenceNotePlace::Over(first.min(second), first.max(second))
        } else {
            return error(format!("unsupported note `{who}`"));
        };
        let text = label(words)?;
        if text.is_empty() {
            return error("a note without text");
        }
        self.content(SequenceEvent::Note { place, text });
        Ok(())
    }

    fn statement(&mut self, line: &str) -> Result<(), DiagramError> {
        let (keyword, tail) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let tail = tail.trim();
        // Mermaid reads its keywords in any case.
        let keyword = keyword.to_ascii_lowercase();
        let block = match keyword.as_str() {
            "loop" => Some(SequenceBlockKind::Loop),
            "alt" => Some(SequenceBlockKind::Alt),
            "opt" => Some(SequenceBlockKind::Opt),
            "par" => Some(SequenceBlockKind::Par),
            "critical" => Some(SequenceBlockKind::Critical),
            "break" => Some(SequenceBlockKind::Break),
            "rect" => Some(SequenceBlockKind::Rect),
            _ => None,
        };
        if let Some(kind) = block {
            if self.open.len() == MAX_DEPTH {
                return error("blocks nested too deep to draw");
            }
            self.open.push((kind, false));
            // A `rect` names a color, which the host's own design replaces.
            let label = match kind {
                SequenceBlockKind::Rect => String::new(),
                _ => label(tail)?.replace('\n', " "),
            };
            self.graph.events.push(SequenceEvent::Open { kind, label });
            return Ok(());
        }
        match keyword.as_str() {
            "participant" | "actor" => {
                let (id, alias) = match tail.split_once(" as ") {
                    Some((id, alias)) => (id, Some(alias)),
                    None => (tail, None),
                };
                let index = self.named(id)?;
                let alias = alias.map(label).transpose()?;
                let participant = &mut self.graph.participants[index];
                participant.actor = keyword == "actor";
                if let Some(alias) = alias.filter(|alias| !alias.is_empty()) {
                    participant.label = alias;
                }
            }
            "title" | "title:" => self.graph.title = label(tail.trim_start_matches(':'))?.replace('\n', " "),
            "autonumber" => {
                let mut values = tail.split_whitespace().map(str::parse::<u32>);
                self.number = match (values.next(), values.next(), values.next()) {
                    (None, ..) => Some((1, 1)),
                    (Some(Ok(start)), None, _) => Some((start, 1)),
                    (Some(Ok(start)), Some(Ok(step)), None) => Some((start, step)),
                    _ if tail.eq_ignore_ascii_case("off") => None,
                    _ => return error(format!("unsupported autonumber `{tail}`")),
                };
            }
            "activate" => {
                let index = self.named(tail)?;
                self.active[index] += 1;
                self.graph.events.push(SequenceEvent::Activate(index));
            }
            "deactivate" => {
                let index = self.named(tail)?;
                if self.active[index] == 0 {
                    return error("`deactivate` ends an activation that never started");
                }
                self.active[index] -= 1;
                self.graph.events.push(SequenceEvent::Deactivate(index));
            }
            "note" => self.note(tail)?,
            "else" | "and" | "option" => {
                let section = match self.open.last() {
                    Some((SequenceBlockKind::Alt, _)) => "else",
                    Some((SequenceBlockKind::Par, _)) => "and",
                    Some((SequenceBlockKind::Critical, _)) => "option",
                    _ => "",
                };
                if keyword != section {
                    return error(format!("`{keyword}` outside its block"));
                }
                let label = label(tail)?.replace('\n', " ");
                self.graph.events.push(SequenceEvent::Section { label });
            }
            "end" if tail.is_empty() => {
                match self.open.pop() {
                    None => return error("`end` without a block"),
                    Some((_, false)) => return error("a block with nothing in it"),
                    Some(_) => {}
                }
                if let Some((_, filled)) = self.open.last_mut() {
                    *filled = true;
                }
                self.graph.events.push(SequenceEvent::Close);
            }
            _ => self.message(line)?,
        }
        Ok(())
    }
}

impl SequenceGraph {
    /// Read a Mermaid `sequenceDiagram`. Anything outside the supported
    /// subset is an error, never a guess: the host shows the source instead.
    pub fn parse(source: &str) -> Result<Self, DiagramError> {
        if source.len() > MAX_SOURCE_BYTES {
            return error("too large to draw");
        }
        let mut lines = statements(source);
        if lines.next().map(|header| header.trim_end_matches(';')) != Some("sequenceDiagram") {
            return error("not a sequence diagram");
        }
        let mut parser = Parser {
            graph: Self { title: String::new(), participants: Vec::new(), events: Vec::new() },
            ids: HashMap::new(),
            open: Vec::new(),
            active: Vec::new(),
            number: None,
        };
        for line in lines {
            parser.statement(line)?;
            if parser.graph.participants.len() > MAX_PARTICIPANTS || parser.graph.events.len() > MAX_EVENTS {
                return error("too large to draw");
            }
        }
        if !parser.open.is_empty() {
            return error("a block is never closed");
        }
        if parser.graph.participants.is_empty() {
            return error("no participants");
        }
        Ok(parser.graph)
    }

    fn messages(&self) -> impl Iterator<Item = &SequenceMessage> {
        self.events.iter().filter_map(|event| match event {
            SequenceEvent::Message(message) => Some(message),
            _ => None,
        })
    }

    /// Place every participant, message, note and block. `measure` gives
    /// the width of one line at a font size and `line_height` the height of
    /// a line at that size, both as the painter will draw them.
    pub fn layout(
        &self,
        measure: &dyn Fn(&str, f32) -> f32,
        line_height: &dyn Fn(f32) -> f32,
    ) -> SequenceLayout {
        let count = self.participants.len();
        if count == 0 {
            return SequenceLayout::default();
        }
        // A measurer that fails must not send anything off the chart.
        let measure = |value: &str, size: f32| {
            let width = measure(value, size);
            if width.is_finite() {
                width.max(0.0)
            } else {
                0.0
            }
        };
        let line = |size: f32| {
            let height = line_height(size);
            if height.is_finite() {
                height.ceil().max(1.0)
            } else {
                size.ceil()
            }
        };
        let (name_line, text_line, small_line) = (line(NAME_TEXT), line(TEXT), line(SMALL_TEXT));
        let wrap = |value: &str, limit: f32, size: f32| -> (Vec<String>, f32) {
            if value.is_empty() {
                return (Vec::new(), 0.0);
            }
            let lines: Vec<String> = wrap_measured(value, limit, |part| measure(part, size))
                .into_iter()
                .map(|line| line.trim().to_owned())
                .collect();
            let width = lines.iter().map(|line| measure(line, size)).fold(0.0, f32::max).ceil();
            (lines, width)
        };
        // A graph built by hand may name a participant it does not have.
        let at = |participant: usize| participant.min(count - 1);

        let mut names = Vec::with_capacity(count);
        let mut sizes = Vec::with_capacity(count);
        for participant in &self.participants {
            let (lines, width) = wrap(&participant.label, MAX_NAME_WIDTH, NAME_TEXT);
            let text_height = lines.len().max(1) as f32 * name_line;
            // A figure's name ends level with the names in the boxes.
            sizes.push(if participant.actor {
                Size::new(odd(width.max(FIGURE_WIDTH)), FIGURE_HEIGHT + FIGURE_GAP + text_height + NAME_PAD_Y)
            } else {
                Size::new(odd((width + 2.0 * NAME_PAD_X).max(MIN_HEAD_WIDTH)), text_height + 2.0 * NAME_PAD_Y)
            });
            names.push(lines);
        }
        let head_height = sizes.iter().map(|size| size.height).fold(0.0, f32::max);

        // The widest an activation bar ever gets on each lifeline.
        let mut depth = vec![0usize; count];
        let mut deepest = vec![0usize; count];
        for event in &self.events {
            let (starts, ends) = match event {
                SequenceEvent::Message(message) => (
                    message.activates.then_some(message.to),
                    message.deactivates.then_some(message.from),
                ),
                SequenceEvent::Activate(participant) => (Some(*participant), None),
                SequenceEvent::Deactivate(participant) => (None, Some(*participant)),
                _ => (None, None),
            };
            if let Some(participant) = ends {
                depth[at(participant)] = depth[at(participant)].saturating_sub(1);
            }
            if let Some(participant) = starts {
                depth[at(participant)] += 1;
                deepest[at(participant)] = deepest[at(participant)].max(depth[at(participant)]);
            }
        }
        let bar_left: Vec<f32> = deepest.iter().map(|&depth| if depth == 0 { 0.0 } else { BAR / 2.0 }).collect();
        let bar_right: Vec<f32> = deepest
            .iter()
            .map(|&depth| if depth == 0 { 0.0 } else { BAR / 2.0 + (depth - 1) as f32 * BAR_STEP })
            .collect();

        // Across: what every row needs between two lifelines, or between
        // one and an edge of the chart. Index 0 is the left edge, then the
        // participants, then the right edge.
        let mut gaps: Vec<(usize, usize, f32)> = Vec::new();
        for (index, size) in sizes.iter().enumerate() {
            let before = if index == 0 { MARGIN } else { sizes[index - 1].width / 2.0 + HEAD_GAP };
            gaps.push((index, index + 1, size.width / 2.0 + before));
        }
        gaps.push((count, count + 1, sizes[count - 1].width / 2.0 + MARGIN));
        // A row stays clear of the lifeline before it and the one after.
        let hold = |gaps: &mut Vec<(usize, usize, f32)>, span: Span| {
            let before = if span.lo == 0 { MARGIN } else { bar_right[span.lo - 1] + CLEAR };
            let after = if span.hi + 1 == count { MARGIN } else { bar_left[span.hi + 1] + CLEAR };
            gaps.push((span.lo, span.lo + 1, span.left + before));
            gaps.push((span.hi + 1, span.hi + 2, span.right + after));
        };
        let mut wrapped: Vec<(Vec<String>, f32)> = Vec::with_capacity(self.events.len());
        // Every block in the order they open: what it holds, then itself.
        let mut spans: Vec<Option<Span>> = Vec::new();
        let mut closed: Vec<bool> = Vec::new();
        // The open blocks and the width their labels need.
        let mut stack: Vec<(usize, f32)> = Vec::new();
        for event in &self.events {
            let mut words = (Vec::new(), 0.0);
            let span = match event {
                SequenceEvent::Message(message) => {
                    words = wrap(&message.text, MAX_TEXT_WIDTH, TEXT);
                    let (lo, hi) = (at(message.from.min(message.to)), at(message.from.max(message.to)));
                    if lo == hi {
                        let beside = if words.0.is_empty() { 0.0 } else { 6.0 + words.1 + CHIP_PAD };
                        Some(Span { lo, hi, left: bar_left[lo], right: bar_right[lo] + LOOP_WIDTH + beside })
                    } else {
                        let pad = if message.number.is_some() { NUMBERED_PAD } else { TEXT_PAD };
                        gaps.push((lo + 1, hi + 1, bar_right[lo] + bar_left[hi] + words.1 + 2.0 * pad));
                        Some(Span { lo, hi, left: bar_left[lo], right: bar_right[hi] })
                    }
                }
                SequenceEvent::Note { place, text } => {
                    words = wrap(text, MAX_NOTE_WIDTH, TEXT);
                    let width = odd(words.1 + 2.0 * NOTE_PAD_X);
                    Some(match *place {
                        SequenceNotePlace::LeftOf(participant) => {
                            let lo = at(participant);
                            Span { lo, hi: lo, left: bar_left[lo] + NOTE_GAP + width, right: bar_right[lo] }
                        }
                        SequenceNotePlace::RightOf(participant) => {
                            let lo = at(participant);
                            Span { lo, hi: lo, left: bar_left[lo], right: bar_right[lo] + NOTE_GAP + width }
                        }
                        SequenceNotePlace::Over(first, second) => {
                            let (lo, hi) = (at(first.min(second)), at(first.max(second)));
                            if lo == hi {
                                let half = width / 2.0;
                                Span { lo, hi, left: half.max(bar_left[lo]), right: half.max(bar_right[lo]) }
                            } else {
                                gaps.push((lo + 1, hi + 1, width - 2.0 * NOTE_OVER));
                                Span { lo, hi, left: NOTE_OVER, right: NOTE_OVER.max(bar_right[hi]) }
                            }
                        }
                    })
                }
                SequenceEvent::Open { kind, label } => {
                    words = wrap(&bracketed(label), MAX_BLOCK_LABEL_WIDTH, SMALL_TEXT);
                    let tab = tab_width(*kind, &measure);
                    let beside = if words.0.is_empty() { 0.0 } else { words.1 + 2.0 * TAB_PAD_X };
                    // One more for the pixel a side may lose when it is rounded.
                    stack.push((spans.len(), tab + beside + 1.0));
                    spans.push(None);
                    closed.push(false);
                    None
                }
                SequenceEvent::Section { label } => {
                    words = wrap(&bracketed(label), MAX_BLOCK_LABEL_WIDTH, SMALL_TEXT);
                    if let Some((_, need)) = stack.last_mut() {
                        *need = need.max(words.1 + 2.0 * TAB_PAD_X + 1.0);
                    }
                    None
                }
                SequenceEvent::Close => stack.pop().and_then(|(block, need)| {
                    let inner = spans[block]?;
                    let mut span = Span { left: inner.left + BLOCK_PAD, right: inner.right + BLOCK_PAD, ..inner };
                    // Its labels fit inside it: the lifelines move apart,
                    // or a block on one lifeline grows to the right.
                    if span.lo == span.hi {
                        span.right = span.right.max(need - span.left);
                    } else {
                        gaps.push((span.lo + 1, span.hi + 1, need - span.left - span.right));
                    }
                    spans[block] = Some(span);
                    closed[block] = true;
                    Some(span)
                }),
                SequenceEvent::Activate(_) | SequenceEvent::Deactivate(_) => None,
            };
            wrapped.push(words);
            if let Some(span) = span {
                hold(&mut gaps, span);
                if let Some(&(block, _)) = stack.last() {
                    spans[block] = Some(spans[block].map_or(span, |known| known.join(span)));
                }
            }
        }
        // Every gap runs left to right, so one pass places each lifeline
        // as far right as its widest need. A lifeline sits on a half pixel.
        let mut xs = vec![0.0f32; count + 2];
        for to in 1..count + 2 {
            let needs = gaps.iter().filter(|gap| gap.1 == to && gap.0 < to);
            let x = needs.map(|gap| xs[gap.0] + gap.2).fold(0.0, f32::max);
            xs[to] = if to <= count { x.ceil() + 0.5 } else { x.ceil() };
        }
        let title_width = measure(&self.title, NAME_TEXT).ceil();
        let width = xs[count + 1].max(title_width + 2.0 * MARGIN);
        let shift = ((width - xs[count + 1]) / 2.0).floor();
        let x = |participant: usize| xs[participant + 1] + shift;

        // Down: each statement takes the rows its text needs.
        let mut y = MARGIN;
        let title = (!self.title.is_empty()).then(|| {
            let start = Point::new(((width - title_width) / 2.0).round(), y);
            y += name_line + 10.0;
            start
        });
        let head_top = y;
        y += head_height + 6.0;
        let mut layout = SequenceLayout { title, ..SequenceLayout::default() };
        // The bars still open on each lifeline, as indexes into the layout's.
        let mut active: Vec<Vec<usize>> = vec![Vec::new(); count];
        // Where an `activate` on a line of its own starts: at the last arrow.
        let mut now = y;
        let mut open: Vec<usize> = Vec::new();
        let mut opened = 0;
        // Where an arrow meets a lifeline: on it, or on the side of its bars.
        let side = |active: &[Vec<usize>], participant: usize, right: bool, ahead: usize| {
            match active[participant].len() + ahead {
                0 => x(participant),
                _ if !right => x(participant) - BAR / 2.0,
                depth => x(participant) + BAR / 2.0 + (depth - 1) as f32 * BAR_STEP,
            }
        };
        // A bar reaches past the arrow that starts it and the one that ends
        // it, so the head of the first and the tail of the last meet its
        // side, not a corner. An arrow is on a half pixel; nothing else is.
        fn reach(y: f32) -> f32 {
            if y.fract() == 0.0 {
                0.0
            } else {
                BAR / 2.0
            }
        }
        fn start(layout: &mut SequenceLayout, active: &mut [Vec<usize>], participant: usize, x: f32, y: f32) {
            let left = x - BAR / 2.0 + active[participant].len() as f32 * BAR_STEP;
            active[participant].push(layout.activations.len());
            layout.activations.push(Rect::new(left, y - reach(y), BAR, 0.0));
        }
        fn end(layout: &mut SequenceLayout, active: &mut [Vec<usize>], participant: usize, y: f32) {
            if let Some(bar) = active[participant].pop() {
                let bar = &mut layout.activations[bar];
                bar.height = (y + reach(y) - bar.y).max(BAR);
            }
        }
        for (index, (event, (lines, text_width))) in self.events.iter().zip(wrapped).enumerate() {
            let text_height = lines.len() as f32 * text_line;
            match event {
                SequenceEvent::Message(message) => {
                    let (from, to) = (at(message.from), at(message.to));
                    y += ROW_GAP;
                    // An `activate` on the next line starts its bar at this arrow.
                    let ahead = usize::from(matches!(
                        self.events.get(index + 1),
                        Some(SequenceEvent::Activate(next)) if at(*next) == to && !message.activates
                    ));
                    let (points, label, number_y);
                    if from == to {
                        let height = text_height.max(LOOP_HEIGHT + 2.0);
                        let out = y + ((height - LOOP_HEIGHT) / 2.0).floor() + 0.5;
                        let back = out + LOOP_HEIGHT;
                        let leave = side(&active, from, true, 0);
                        if message.activates {
                            start(&mut layout, &mut active, to, x(to), out);
                        }
                        let arrive = side(&active, to, true, ahead);
                        let turn = leave.max(arrive) + LOOP_WIDTH;
                        points = rounded(&[
                            Point::new(leave, out),
                            Point::new(turn, out),
                            Point::new(turn, back),
                            Point::new(arrive, back),
                        ]);
                        label = (!lines.is_empty()).then(|| {
                            Rect::new(
                                (turn + 6.0).round() - CHIP_PAD,
                                y + ((height - text_height) / 2.0).floor(),
                                text_width + 2.0 * CHIP_PAD,
                                text_height,
                            )
                        });
                        if message.deactivates {
                            end(&mut layout, &mut active, from, back);
                        }
                        (number_y, now) = (out, back);
                        y += height + 4.0;
                    } else {
                        let label_top = y;
                        y += text_height + if lines.is_empty() { 4.0 } else { 3.0 };
                        let line_y = y + 0.5;
                        let rightward = to > from;
                        let leave = side(&active, from, rightward, 0);
                        if message.activates {
                            start(&mut layout, &mut active, to, x(to), line_y);
                        }
                        let arrive = side(&active, to, !rightward, ahead);
                        if message.deactivates {
                            end(&mut layout, &mut active, from, line_y);
                        }
                        points = vec![Point::new(leave, line_y), Point::new(arrive, line_y)];
                        label = (!lines.is_empty()).then(|| {
                            Rect::new(
                                ((leave + arrive - text_width) / 2.0).round() - CHIP_PAD,
                                label_top,
                                text_width + 2.0 * CHIP_PAD,
                                text_height,
                            )
                        });
                        (number_y, now) = (line_y, line_y);
                        y += 5.0;
                    }
                    let number = message.number.map(|number| {
                        let width = odd((measure(&number.to_string(), NUMBER_TEXT) + 8.0).max(NUMBER_SIZE));
                        Rect::new(x(from) - width / 2.0, number_y - NUMBER_SIZE / 2.0, width, NUMBER_SIZE)
                    });
                    layout.messages.push(SequenceMessageLayout {
                        points,
                        label: label.map(|rect| (rect, lines)),
                        number,
                    });
                }
                SequenceEvent::Note { place, .. } => {
                    y += ROW_GAP;
                    let mut width = odd(text_width + 2.0 * NOTE_PAD_X);
                    let left = match *place {
                        SequenceNotePlace::LeftOf(participant) => {
                            x(at(participant)) - bar_left[at(participant)] - NOTE_GAP - width
                        }
                        SequenceNotePlace::RightOf(participant) => {
                            x(at(participant)) + bar_right[at(participant)] + NOTE_GAP
                        }
                        SequenceNotePlace::Over(first, second) if at(first) == at(second) => {
                            x(at(first)) - width / 2.0
                        }
                        SequenceNotePlace::Over(first, second) => {
                            let (lo, hi) = (at(first.min(second)), at(first.max(second)));
                            width = x(hi) - x(lo) + 2.0 * NOTE_OVER;
                            x(lo) - NOTE_OVER
                        }
                    };
                    let height = lines.len().max(1) as f32 * text_line + 2.0 * NOTE_PAD_Y;
                    let rect = Rect::new(left.round(), y, width.round(), height);
                    y += rect.height;
                    now = y;
                    layout.notes.push(SequenceNoteLayout { rect, lines });
                }
                SequenceEvent::Activate(participant) => {
                    start(&mut layout, &mut active, at(*participant), x(at(*participant)), now);
                }
                SequenceEvent::Deactivate(participant) => end(&mut layout, &mut active, at(*participant), now),
                SequenceEvent::Open { kind, .. } => {
                    let block = opened;
                    opened += 1;
                    y += ROW_GAP;
                    // A block left open, or with nothing in it, is not drawn.
                    let span = spans[block].filter(|_| closed[block]);
                    let left = span.map_or(0.0, |span| (x(span.lo) - span.left).round());
                    let right = span.map_or(0.0, |span| (x(span.hi) + span.right).round());
                    let tab = match kind {
                        SequenceBlockKind::Rect => Rect::new(left, y, 0.0, 0.0),
                        _ => Rect::new(left, y, tab_width(*kind, &measure), small_line + 2.0 * TAB_PAD_Y),
                    };
                    let label_height = lines.len() as f32 * small_line;
                    let label = Rect::new(tab.x + tab.width + TAB_PAD_X, y + TAB_PAD_Y, text_width, label_height);
                    open.push(layout.blocks.len());
                    layout.blocks.push(SequenceBlockLayout {
                        kind: *kind,
                        rect: Rect::new(left, y, right - left, 0.0),
                        tab,
                        labels: if lines.is_empty() { Vec::new() } else { vec![(label, lines)] },
                        dividers: Vec::new(),
                    });
                    y += tab.height.max(if label.height > 0.0 { label.height + 2.0 * TAB_PAD_Y } else { 0.0 });
                    now = y;
                }
                SequenceEvent::Section { .. } => {
                    y += ROW_GAP;
                    if let Some(&block) = open.last() {
                        let block = &mut layout.blocks[block];
                        block.dividers.push(y + 0.5);
                        if !lines.is_empty() {
                            let height = lines.len() as f32 * small_line;
                            let left = block.rect.x + TAB_PAD_X;
                            block.labels.push((Rect::new(left, y + 1.0 + TAB_PAD_Y, text_width, height), lines));
                            y += height + TAB_PAD_Y;
                        }
                    }
                    y += 1.0;
                    now = y;
                }
                SequenceEvent::Close => {
                    if let Some(block) = open.pop() {
                        y += BLOCK_PAD;
                        layout.blocks[block].rect.height = y - layout.blocks[block].rect.y;
                        now = y;
                    }
                }
            }
        }
        y += 16.0;
        for participant in 0..count {
            while !active[participant].is_empty() {
                end(&mut layout, &mut active, participant, y - 6.0);
            }
        }
        layout.blocks.retain(|block| block.rect.width > 0.0 && block.rect.height > 0.0);
        layout.participants = names
            .into_iter()
            .zip(&sizes)
            .enumerate()
            .map(|(index, (lines, size))| {
                // Each stands on its lifeline, whatever the height of the others.
                let left = x(index) - size.width / 2.0;
                SequenceParticipantLayout {
                    head: Rect::new(left, head_top + head_height - size.height, size.width, size.height),
                    foot: Rect::new(left, y, size.width, size.height),
                    x: x(index),
                    lines,
                }
            })
            .collect();
        layout.size = Size::new(width, y + head_height + MARGIN);
        layout
    }
}

/// From `left` before the lifeline of `lo` to `right` past the lifeline of
/// `hi`: what a row covers, before the lifelines are placed.
#[derive(Clone, Copy)]
struct Span {
    lo: usize,
    hi: usize,
    left: f32,
    right: f32,
}

impl Span {
    /// The span that holds both. A row never reaches the lifeline before
    /// its own, so the lower index is always the further left.
    fn join(self, other: Self) -> Self {
        let (lo, left) = match self.lo.cmp(&other.lo) {
            std::cmp::Ordering::Less => (self.lo, self.left),
            std::cmp::Ordering::Greater => (other.lo, other.left),
            std::cmp::Ordering::Equal => (self.lo, self.left.max(other.left)),
        };
        let (hi, right) = match self.hi.cmp(&other.hi) {
            std::cmp::Ordering::Greater => (self.hi, self.right),
            std::cmp::Ordering::Less => (other.hi, other.right),
            std::cmp::Ordering::Equal => (self.hi, self.right.max(other.right)),
        };
        Self { lo, hi, left, right }
    }
}

/// A block's condition as Mermaid shows it.
fn bracketed(label: &str) -> String {
    if label.is_empty() {
        String::new()
    } else {
        format!("[{label}]")
    }
}

fn tab_width(kind: SequenceBlockKind, measure: &dyn Fn(&str, f32) -> f32) -> f32 {
    match kind {
        SequenceBlockKind::Rect => 0.0,
        _ => measure(kind.keyword(), SMALL_TEXT).ceil() + 2.0 * TAB_PAD_X,
    }
}

/// One participant: its box over the lifeline and the same box under it.
#[derive(Clone, Debug, PartialEq)]
pub struct SequenceParticipantLayout {
    pub head: Rect,
    pub foot: Rect,
    /// The lifeline, from the bottom of `head` to the top of `foot`.
    pub x: f32,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceMessageLayout {
    /// From the sender to the receiver: two points, or a turn out and back
    /// for a message a participant sends itself.
    pub points: Vec<Point>,
    /// The chip that carries the text, and its lines.
    pub label: Option<(Rect, Vec<String>)>,
    /// The chip of its number, on the sender's lifeline.
    pub number: Option<Rect>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceNoteLayout {
    pub rect: Rect,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SequenceBlockLayout {
    pub kind: SequenceBlockKind,
    pub rect: Rect,
    /// The corner that names the kind; empty for a `rect`.
    pub tab: Rect,
    /// The condition of each section that has one: its box and its lines.
    pub labels: Vec<(Rect, Vec<String>)>,
    /// The line over each section after the first.
    pub dividers: Vec<f32>,
}

/// Where everything in a [`SequenceGraph`] goes, from a zero origin.
/// Participants and messages keep the graph's order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SequenceLayout {
    pub size: Size,
    /// Where the title starts; `None` without one.
    pub title: Option<Point>,
    pub participants: Vec<SequenceParticipantLayout>,
    pub messages: Vec<SequenceMessageLayout>,
    pub notes: Vec<SequenceNoteLayout>,
    /// An outer block before the ones inside it.
    pub blocks: Vec<SequenceBlockLayout>,
    /// The bar of each activation, an outer one before those on top of it.
    pub activations: Vec<Rect>,
}

#[derive(Debug)]
struct Shared {
    graph: SequenceGraph,
    layout: LayoutCache<SequenceLayout>,
}

/// A sequence diagram at its natural size, in the palette's own inks:
/// participants and activations on `surface` with a `border` outline,
/// lifelines and blocks in `border`, arrows in `muted` and what they say in
/// `text`. Wider than its container, [`SequenceDiagram::scrollable`] pans it.
#[derive(Clone, Debug)]
pub struct SequenceDiagram {
    shared: Arc<Shared>,
    palette: ComponentPalette,
    backdrop: Color,
}

impl SequenceDiagram {
    /// `graph` drawn on the palette's `raised` fill, as inside a card.
    pub fn new(graph: SequenceGraph, palette: ComponentPalette) -> Self {
        Self {
            shared: Arc::new(Shared { graph, layout: LayoutCache::new() }),
            palette,
            backdrop: palette.raised,
        }
    }

    /// Parse a Mermaid `sequenceDiagram` source and draw it.
    pub fn parse(source: &str, palette: ComponentPalette) -> Result<Self, DiagramError> {
        Ok(Self::new(SequenceGraph::parse(source)?, palette))
    }

    /// The fill the diagram sits on, painted behind the text of a message
    /// so a lifeline never runs through it.
    pub fn backdrop(mut self, color: Color) -> Self {
        self.backdrop = color;
        self
    }

    pub fn graph(&self) -> &SequenceGraph {
        &self.shared.graph
    }

    /// The diagram in a horizontal viewport that fills the width it is
    /// given and pans when the diagram is wider, with a fade at a cut edge.
    pub fn scrollable(self, id: impl Into<String>) -> Element {
        let (backdrop, bar) = (self.backdrop, self.palette.muted);
        pannable(self.render(), id, backdrop, bar)
    }

    pub(crate) fn palette(&self) -> ComponentPalette {
        self.palette
    }

    pub(crate) fn heap_bytes(&self) -> usize {
        let graph = &self.shared.graph;
        graph.title.len()
            + graph.participants.iter().map(|one| one.id.len() + one.label.len() + 64).sum::<usize>()
            + graph
                .events
                .iter()
                .map(|event| match event {
                    SequenceEvent::Message(message) => message.text.len() + 96,
                    SequenceEvent::Note { text, .. } => text.len() + 96,
                    SequenceEvent::Open { label, .. } | SequenceEvent::Section { label } => label.len() + 96,
                    _ => 96,
                })
                .sum::<usize>()
    }

    pub(crate) fn layout(
        &self,
        measure: &dyn Fn(&str, f32) -> f32,
        line_height: &dyn Fn(f32) -> f32,
    ) -> Arc<SequenceLayout> {
        let graph = &self.shared.graph;
        let mut key = DefaultHasher::new();
        measure(&graph.title, NAME_TEXT).to_bits().hash(&mut key);
        for participant in &graph.participants {
            measure(&participant.label, NAME_TEXT).to_bits().hash(&mut key);
        }
        for event in &graph.events {
            match event {
                SequenceEvent::Message(message) => measure(&message.text, TEXT).to_bits().hash(&mut key),
                SequenceEvent::Note { text, .. } => measure(text, TEXT).to_bits().hash(&mut key),
                SequenceEvent::Open { label, .. } | SequenceEvent::Section { label } => {
                    measure(label, SMALL_TEXT).to_bits().hash(&mut key);
                }
                _ => {}
            }
        }
        [NAME_TEXT, TEXT, SMALL_TEXT, NUMBER_TEXT].map(|size| line_height(size).to_bits()).hash(&mut key);
        measure("0", NUMBER_TEXT).to_bits().hash(&mut key);
        self.shared.layout.get(key.finish(), || graph.layout(measure, line_height))
    }

    /// Paint into `content`, centered where it is larger than the diagram.
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
        let line = |size: f32| line_height(size).ceil();
        let stroke = |commands: &mut Vec<DrawCommand>, points: Vec<Point>, color: Color, dashed: bool| {
            commands.push(DrawCommand::Polyline { points: points.into(), color, width: 1.0, dashed });
        };
        // `lines` centered in `rect`, each on its own row.
        let centered = |commands: &mut Vec<DrawCommand>, lines: &[String], rect: Rect, size: f32, color: Color| {
            let top = rect.y + (rect.height - lines.len() as f32 * line(size)) / 2.0;
            for (row, value) in lines.iter().enumerate() {
                text(
                    commands,
                    value,
                    rect.x + (rect.width - measure(value, size)) / 2.0,
                    top + row as f32 * line(size),
                    size,
                    color,
                );
            }
        };
        let radius = RadiusRole::Control.px();
        // What a label hides the lines under it with: the fill it sits on.
        let highlighted: Vec<Rect> = layout
            .blocks
            .iter()
            .filter(|block| block.kind == SequenceBlockKind::Rect)
            .map(|block| moved(block.rect))
            .collect();
        let under = |rect: Rect| {
            if highlighted.iter().any(|block| block.intersection(rect).is_some()) {
                palette.hover
            } else {
                self.backdrop
            }
        };

        for rect in &highlighted {
            commands.push(DrawCommand::FillRect { rect: *rect, color: palette.hover, radius });
        }
        for participant in &layout.participants {
            let (top, bottom) = (participant.head.y + participant.head.height, participant.foot.y);
            stroke(
                commands,
                vec![at(Point::new(participant.x, top)), at(Point::new(participant.x, bottom))],
                palette.border,
                false,
            );
        }
        // A lifeline does not run through a text: each has the fill it sits
        // on behind it. A bar is drawn over that fill, so no text cuts it.
        let chip = |label: Rect| Rect::new(label.x - CHIP_PAD, label.y, label.width + 2.0 * CHIP_PAD, label.height);
        let conditions = layout.blocks.iter().flat_map(|block| &block.labels).map(|(label, _)| chip(moved(*label)));
        let texts = layout.messages.iter().filter_map(|placed| placed.label.as_ref()).map(|(rect, _)| moved(*rect));
        for rect in conditions.chain(texts) {
            commands.push(DrawCommand::FillRect { rect, color: under(rect), radius });
        }
        for bar in &layout.activations {
            let rect = moved(*bar);
            let radius = radius.min(2.0);
            commands.push(DrawCommand::FillRect { rect, color: palette.surface, radius });
            commands.push(DrawCommand::StrokeRect { rect, color: palette.border, width: 1.0, radius });
        }
        // After the bars, so none runs through the keyword of a block.
        for block in layout.blocks.iter().filter(|block| block.kind != SequenceBlockKind::Rect) {
            let rect = moved(block.rect);
            for divider in &block.dividers {
                let y = origin.y + divider;
                let ends = vec![Point::new(rect.x, y), Point::new(rect.x + rect.width, y)];
                stroke(commands, ends, palette.border, true);
            }
            commands.push(DrawCommand::FillRect { rect: moved(block.tab), color: palette.selected, radius });
            commands.push(DrawCommand::StrokeRect { rect, color: palette.border, width: 1.0, radius });
            text(
                commands,
                block.kind.keyword(),
                rect.x + TAB_PAD_X,
                rect.y + TAB_PAD_Y,
                SMALL_TEXT,
                palette.text,
            );
            for (label, lines) in &block.labels {
                let label = moved(*label);
                for (row, value) in lines.iter().enumerate() {
                    let y = label.y + row as f32 * line(SMALL_TEXT);
                    text(commands, value, label.x, y, SMALL_TEXT, palette.muted);
                }
            }
        }
        for (message, placed) in graph.messages().zip(&layout.messages) {
            let [.., before, tip] = placed.points[..] else {
                continue;
            };
            let length = (tip.x - before.x).hypot(tip.y - before.y).max(0.001);
            let (dx, dy) = ((tip.x - before.x) / length, (tip.y - before.y) / length);
            let mut points: Vec<Point> = placed.points.iter().map(|point| at(*point)).collect();
            // A point `along` the line back from the tip and `across` it.
            let back = |along: f32, across: f32| {
                at(Point::new(tip.x - dx * along - dy * across, tip.y - dy * along + dx * across))
            };
            match message.arrow {
                SequenceArrow::Filled => {
                    // The line stops inside the head so the tip stays pointed.
                    let last = points.len() - 1;
                    points[last] = back(ARROW / 2.0, 0.0);
                    arrowhead(commands, at(tip), (dx, dy), palette.muted);
                }
                SequenceArrow::Open => {
                    stroke(commands, vec![back(ARROW, 4.0), at(tip), back(ARROW, -4.0)], palette.muted, false);
                }
                SequenceArrow::Cross => {
                    stroke(commands, vec![back(2.0, 4.0), back(10.0, -4.0)], palette.muted, false);
                    stroke(commands, vec![back(2.0, -4.0), back(10.0, 4.0)], palette.muted, false);
                }
                SequenceArrow::Plain => {}
            }
            stroke(commands, points, palette.muted, message.dotted);
        }
        for (message, placed) in graph.messages().zip(&layout.messages) {
            if let Some((rect, lines)) = &placed.label {
                centered(commands, lines, moved(*rect), TEXT, palette.text);
            }
            if let (Some(rect), Some(number)) = (placed.number, message.number) {
                let rect = moved(rect);
                commands.push(DrawCommand::FillRect { rect, color: palette.muted, radius: rect.height / 2.0 });
                centered(commands, &[number.to_string()], rect, NUMBER_TEXT, self.backdrop);
            }
        }
        for note in &layout.notes {
            let rect = moved(note.rect);
            commands.push(DrawCommand::FillRect { rect, color: palette.hover, radius });
            commands.push(DrawCommand::StrokeRect { rect, color: palette.border, width: 1.0, radius });
            centered(commands, &note.lines, rect, TEXT, palette.text);
        }
        for (participant, placed) in graph.participants.iter().zip(&layout.participants) {
            for rect in [placed.head, placed.foot] {
                let rect = moved(rect);
                if !participant.actor {
                    commands.push(DrawCommand::FillRect { rect, color: palette.surface, radius });
                    commands.push(DrawCommand::StrokeRect { rect, color: palette.border, width: 1.0, radius });
                    centered(commands, &placed.lines, rect, NAME_TEXT, palette.text);
                    continue;
                }
                // A figure: head, body, arms and legs, with the name under it.
                let (middle, top) = (origin.x + placed.x, rect.y);
                commands.push(DrawCommand::StrokeRect {
                    rect: Rect::new(middle - 5.5, top, 11.0, 11.0),
                    color: palette.muted,
                    width: 1.0,
                    radius: 5.5,
                });
                let at = |x: f32, y: f32| Point::new(middle + x, top + y);
                for limb in [
                    vec![at(0.0, 11.0), at(0.0, 23.0)],
                    vec![at(-8.0, 15.5), at(8.0, 15.5)],
                    vec![at(-7.0, 33.0), at(0.0, 23.0), at(7.0, 33.0)],
                ] {
                    stroke(commands, limb, palette.muted, false);
                }
                let below = FIGURE_HEIGHT + FIGURE_GAP;
                let name = Rect::new(rect.x, top + below, rect.width, rect.height - below - NAME_PAD_Y);
                centered(commands, &placed.lines, name, NAME_TEXT, palette.text);
            }
        }
        if let Some(start) = layout.title {
            text(commands, &graph.title, origin.x + start.x, origin.y + start.y, NAME_TEXT, palette.text);
        }
    }
}

impl Component for SequenceDiagram {
    fn render(self) -> Element {
        Element::sequence_diagram(self)
    }
}

#[cfg(test)]
#[path = "sequence_tests.rs"]
mod tests;
