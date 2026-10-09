//! CommonMark/GFM source analysis, with the same versioned coordinates as code grammars.
//! Markdown block/inline scopes mirror the native Zed grammars and VS Code TextMate
//! grammar. Fenced languages reuse our existing parsers; no HTML or URL is executed.
use super::*;
use lienzo_editor::TextBuffer;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser as MarkdownParser, Tag, TagEnd};
use std::collections::BTreeSet;

struct Span {
    bytes: Range<usize>,
    kind: HighlightKind,
    priority: u8,
}
struct Spans {
    values: Vec<Span>,
    limit: usize,
    truncated: bool,
}
impl Spans {
    fn push(&mut self, bytes: Range<usize>, kind: HighlightKind, priority: u8) {
        if bytes.is_empty() {
            return;
        }
        if self.values.len() >= self.limit {
            self.truncated = true;
            return;
        }
        self.values.push(Span {
            bytes,
            kind,
            priority,
        });
    }
}
struct Heading {
    level: u8,
    bytes: Range<usize>,
    name: String,
    selection: Option<Range<usize>>,
}
struct Code {
    language: Language,
    bytes: Range<usize>,
    content: Option<Range<usize>>,
    text: String,
}
struct Folds {
    values: Vec<Fold>,
    limit: usize,
    truncated: bool,
}
impl Folds {
    fn push(&mut self, rows: Range<usize>) {
        if rows.end <= rows.start.saturating_add(1) {
            return;
        }
        if self.values.len() >= self.limit {
            self.truncated = true;
        } else {
            self.values.push(Fold { rows });
        }
    }
}

pub(super) fn analyze(
    engine: &mut ParserEngine,
    snapshot: &BufferSnapshot,
    source: &str,
    cancellation: &Cancellation,
) -> AnalysisSnapshot {
    let limits = engine.limits;
    let deadline = Instant::now() + limits.timeout;
    let mut result = AnalysisSnapshot::empty(snapshot, Language::Markdown, false);
    let mut spans = Spans {
        values: Vec::new(),
        limit: limits.max_spans,
        truncated: false,
    };
    let mut headings = Vec::<Heading>::new();
    let mut current_heading = None::<Heading>;
    let mut code = None::<Code>;
    let mut folds = Folds {
        values: Vec::new(),
        limit: limits.max_folds,
        truncated: false,
    };
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = MarkdownParser::new_ext(source, options);
    for (_, definition) in parser.reference_definitions().iter() {
        if spans.truncated || cancelled(cancellation, deadline) {
            result.truncated = true;
            break;
        }
        spans.push(definition.span.clone(), HighlightKind::Function, 3);
        if let Some(raw) = source.get(definition.span.clone()) {
            if let Some(colon) = raw.find("]:") {
                spans.push(
                    definition.span.start + colon + 2..definition.span.end,
                    HighlightKind::String,
                    4,
                );
            }
        }
    }
    for (count, (event, bytes)) in parser.into_offset_iter().enumerate() {
        if count >= limits.max_nodes || spans.truncated || cancelled(cancellation, deadline) {
            result.truncated = true;
            break;
        }
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                spans.push(bytes.clone(), HighlightKind::Type, 1);
                current_heading = Some(Heading {
                    level: level as u8,
                    bytes,
                    name: String::new(),
                    selection: None,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(heading) = current_heading.take() {
                    if headings.len() < 4096 {
                        headings.push(heading);
                    } else {
                        result.truncated = true;
                    }
                }
            }
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough) => {
                spans.push(bytes, HighlightKind::Property, 2);
            }
            Event::Start(Tag::Link { .. } | Tag::Image { .. }) => {
                // The parser determines whether this is actually a link (including
                // reference links); only its source spelling is colored.
                spans.push(bytes.clone(), HighlightKind::Function, 3);
                if let Some(raw) = source.get(bytes.clone()) {
                    if let Some(destination) = raw.rfind("](") {
                        spans.push(
                            bytes.start + destination + 1..bytes.end,
                            HighlightKind::String,
                            4,
                        );
                    } else if raw.starts_with('<') {
                        spans.push(bytes, HighlightKind::String, 4);
                    }
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => Language::from_code_fence(&info),
                    CodeBlockKind::Indented => Language::Plain,
                };
                spans.push(bytes.clone(), HighlightKind::Keyword, 5);
                add_fold(snapshot, bytes.clone(), &mut folds);
                code = Some(Code {
                    language,
                    bytes,
                    content: None,
                    text: String::new(),
                });
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code.take() {
                    highlight_code(
                        engine,
                        snapshot,
                        source,
                        code,
                        cancellation,
                        deadline,
                        &mut spans,
                    );
                }
            }
            Event::Start(Tag::BlockQuote(_) | Tag::List(_)) => {
                add_fold(snapshot, bytes, &mut folds);
            }
            Event::Start(Tag::Table(_)) => {
                if let Some(raw) = source.get(bytes.clone()) {
                    for (offset, ch) in raw.char_indices() {
                        if ch == '|' {
                            spans.push(
                                bytes.start + offset..bytes.start + offset + 1,
                                HighlightKind::Keyword,
                                3,
                            );
                        }
                    }
                }
            }
            Event::Text(text) => {
                if let Some(code) = &mut code {
                    code.content =
                        Some(code.content.as_ref().map_or(bytes.start, |r| r.start)..bytes.end);
                    code.text.push_str(&text);
                } else if let Some(heading) = &mut current_heading {
                    append_heading(heading, &text, bytes);
                }
            }
            Event::Code(text) => {
                spans.push(bytes.clone(), HighlightKind::String, 6);
                if let Some(heading) = &mut current_heading {
                    append_heading(heading, &text, bytes);
                }
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                spans.push(bytes, HighlightKind::Comment, 7);
            }
            Event::Rule | Event::TaskListMarker(_) | Event::FootnoteReference(_) => {
                spans.push(bytes, HighlightKind::Keyword, 3);
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = &mut current_heading {
                    heading.name.push(' ');
                }
            }
            _ => {}
        }
    }
    // Prefix punctuation is intentionally independent of inline content. Code spans
    // have a higher priority, so a '*' or '>' inside code remains literal code.
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if spans.truncated || cancelled(cancellation, deadline) {
            result.truncated = true;
            break;
        }
        prefix_markers(line, offset, &mut spans);
        offset += line.len();
    }
    let mut symbols = Vec::<Symbol>::new();
    let mut section_stack = Vec::<(u8, usize, usize)>::new();
    for heading in headings {
        let Some(range) = char_range(snapshot, heading.bytes) else {
            continue;
        };
        let Some(selection) = heading
            .selection
            .and_then(|range| char_range(snapshot, range))
        else {
            continue;
        };
        let row = snapshot
            .position_of(range.start)
            .map_or(0, |position| position.row);
        while section_stack
            .last()
            .is_some_and(|(level, _, _)| *level >= heading.level)
        {
            let (_, start, symbol) = section_stack.pop().unwrap();
            symbols[symbol].range.end = range.start;
            folds.push(start..row);
        }
        section_stack.push((heading.level, row, symbols.len()));
        symbols.push(Symbol {
            name: heading.name,
            kind: SymbolKind::Module,
            range,
            selection,
        });
    }
    for (_, start, symbol) in section_stack {
        symbols[symbol].range.end = snapshot.char_count();
        folds.push(start..snapshot.line_count());
    }
    folds
        .values
        .sort_by_key(|fold| (fold.rows.start, fold.rows.end));
    folds.values.dedup();
    result.truncated |= spans.truncated || folds.truncated || cancelled(cancellation, deadline);
    let (highlights, truncated) = flatten(
        snapshot,
        spans.values,
        limits.max_spans,
        cancellation,
        deadline,
    );
    result.truncated |= truncated;
    result.highlights = highlights.into();
    result.symbols = symbols.into();
    result.folds = folds.values.into();
    result
}

fn cancelled(cancellation: &Cancellation, deadline: Instant) -> bool {
    cancellation.is_cancelled() || Instant::now() >= deadline
}

fn append_heading(heading: &mut Heading, text: &str, bytes: Range<usize>) {
    // Symbols are small labels, never a second unbounded copy of a document.
    let remaining = 1024usize.saturating_sub(heading.name.chars().count());
    heading.name.extend(text.chars().take(remaining));
    heading.selection =
        Some(heading.selection.as_ref().map_or(bytes.start, |r| r.start)..bytes.end);
}

fn add_fold(snapshot: &BufferSnapshot, bytes: Range<usize>, folds: &mut Folds) {
    let Some(range) = char_range(snapshot, bytes) else {
        return;
    };
    let start = snapshot
        .position_of(range.start)
        .map_or(0, |position| position.row);
    let end = snapshot
        .position_of(range.end.saturating_sub(1))
        .map_or(start, |position| position.row);
    folds.push(start..end + 1);
}

fn prefix_markers(line: &str, offset: usize, spans: &mut Spans) {
    let mut index = line.len() - line.trim_start_matches([' ', '\t']).len();
    let bytes = line.as_bytes();
    while bytes.get(index) == Some(&b'>') {
        spans.push(
            offset + index..offset + index + 1,
            HighlightKind::Keyword,
            3,
        );
        index += 1;
        while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
    }
    let start = index;
    if bytes
        .get(index)
        .is_some_and(|byte| matches!(byte, b'-' | b'*' | b'+'))
    {
        index += 1;
    } else {
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start
            || !bytes
                .get(index)
                .is_some_and(|byte| matches!(byte, b'.' | b')'))
        {
            return;
        }
        index += 1;
    }
    if bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        spans.push(offset + start..offset + index, HighlightKind::Keyword, 3);
    }
}

fn highlight_code(
    engine: &mut ParserEngine,
    snapshot: &BufferSnapshot,
    source: &str,
    code: Code,
    cancellation: &Cancellation,
    deadline: Instant,
    spans: &mut Spans,
) {
    let Some(content) = code.content else { return };
    // A base scope covers code including punctuation in unknown languages.
    spans.push(content.clone(), HighlightKind::String, 8);
    if matches!(code.language, Language::Plain | Language::Markdown)
        || cancelled(cancellation, deadline)
    {
        return;
    }
    // Offset events can remove list/quote prefixes. Only inject a contiguous verbatim
    // slice; otherwise retain the literal scope instead of emitting incorrect offsets.
    let Some(raw) = source.get(content.clone()) else {
        return;
    };
    if raw != code.text || content.start < code.bytes.start || content.end > code.bytes.end {
        return;
    }
    let Ok(buffer) = TextBuffer::new(raw) else {
        return;
    };
    let original_timeout = engine.limits.timeout;
    engine.limits.timeout = deadline.saturating_duration_since(Instant::now());
    let analysis = engine.analyze(&buffer.snapshot(), code.language, cancellation);
    engine.limits.timeout = original_timeout;
    let Ok(analysis) = analysis else { return };
    spans.truncated |= analysis.truncated;
    let embedded = buffer.snapshot();
    for highlight in analysis.highlights.iter() {
        let (Ok(start), Ok(end)) = (
            embedded.char_to_byte(highlight.range.start),
            embedded.char_to_byte(highlight.range.end),
        ) else {
            continue;
        };
        // The outer snapshot's conversion is performed once after overlap resolution.
        if content.start + end <= snapshot.byte_count() {
            spans.push(
                content.start + start..content.start + end,
                highlight.kind,
                9,
            );
        }
    }
}

/// Produce disjoint spans: inline code must win over the surrounding heading/link.
/// A sweep bounds overlap resolution by O(n log n), even for heavily nested Markdown.
fn flatten(
    snapshot: &BufferSnapshot,
    spans: Vec<Span>,
    limit: usize,
    cancellation: &Cancellation,
    deadline: Instant,
) -> (Vec<Highlight>, bool) {
    let mut edges = Vec::with_capacity(spans.len() * 2);
    for (index, span) in spans.iter().enumerate() {
        edges.push((span.bytes.start, true, index));
        edges.push((span.bytes.end, false, index));
    }
    edges.sort_unstable();
    let mut active = BTreeSet::new();
    let mut output = Vec::<Highlight>::new();
    let mut previous = 0;
    for (offset, entering, index) in edges {
        if output.len() >= limit || cancelled(cancellation, deadline) {
            return (output, true);
        }
        if offset > previous {
            if let Some(&(_, selected)) = active.last() {
                let selected: &Span = &spans[selected];
                if let Some(range) = char_range(snapshot, previous..offset) {
                    match output.last_mut() {
                        Some(last)
                            if last.kind == selected.kind && last.range.end == range.start =>
                        {
                            last.range.end = range.end
                        }
                        _ => output.push(Highlight {
                            range,
                            kind: selected.kind,
                        }),
                    }
                }
            }
        }
        let key = (spans[index].priority, index);
        if entering {
            active.insert(key);
        } else {
            active.remove(&key);
        }
        previous = offset;
    }
    (output, false)
}

#[cfg(test)]
#[path = "markdown_syntax_tests.rs"]
mod tests;
