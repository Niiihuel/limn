//! The bounded Markdown subset behind [`crate::Markdown`]: blocks, inline
//! runs and web destinations. Nothing here builds an element.

use std::ops::Range;

use crate::Icon;

pub(crate) const MAX_SOURCE_CHARS: usize = 20_000;
pub(crate) const MAX_BLOCKS: usize = 160;
const MAX_MARKUP_LOOKAHEAD: usize = 2_048;
const MAX_URL_BYTES: usize = 2_048;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ListMarker {
    Bullet,
    Number(String),
    Done,
    Todo,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Block {
    Heading(u8, String),
    Paragraph(String),
    ListItem(usize, ListMarker, String),
    Quote(String),
    Code(String, String),
    /// A fence the source has not closed: its content can still grow.
    OpenCode(String, String),
    Rule,
}

impl Block {
    pub(crate) fn heap_bytes(&self) -> usize {
        match self {
            Self::Heading(_, text) | Self::Paragraph(text) | Self::Quote(text) => text.capacity(),
            Self::ListItem(_, marker, text) => {
                text.capacity()
                    + match marker {
                        ListMarker::Number(number) => number.capacity(),
                        _ => 0,
                    }
            }
            Self::Code(language, content) | Self::OpenCode(language, content) => {
                language.capacity() + content.capacity()
            }
            Self::Rule => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InlineKind {
    Normal,
    Strong,
    Emphasis,
    Code,
    Link,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InlineRun {
    pub(crate) kind: InlineKind,
    pub(crate) text: String,
    pub(crate) destination: Option<String>,
}

pub(crate) fn push_run(runs: &mut Vec<InlineRun>, kind: InlineKind, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = runs
        .last_mut()
        .filter(|last| last.kind == kind && last.destination.is_none())
    {
        last.text.push_str(text);
    } else {
        runs.push(InlineRun {
            kind,
            text: text.to_owned(),
            destination: None,
        });
    }
}

pub(crate) fn flush_paragraph(blocks: &mut Vec<Block>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    let mut content = String::new();
    for line in paragraph.drain(..) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push(' ');
        }
        content.push_str(&line);
    }
    blocks.push(Block::Paragraph(content));
}

pub(crate) fn heading(line: &str) -> Option<(u8, &str)> {
    let count = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (1..=3)
        .contains(&count)
        .then(|| line.get(count..))
        .flatten()
        .and_then(|rest| rest.strip_prefix(' ').map(|text| (count as u8, text)))
}

pub(crate) fn list_item(line: &str) -> Option<(usize, ListMarker, &str)> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    let body = line.get(indent..)?;
    let depth = (indent / 2).min(4);
    if let Some(text) = body.strip_prefix("- ").or_else(|| body.strip_prefix("* ")) {
        if let Some(text) = text
            .strip_prefix("[x] ")
            .or_else(|| text.strip_prefix("[X] "))
        {
            return Some((depth, ListMarker::Done, text));
        }
        if let Some(text) = text.strip_prefix("[ ] ") {
            return Some((depth, ListMarker::Todo, text));
        }
        return Some((depth, ListMarker::Bullet, text));
    }
    let number_len = body.chars().take_while(char::is_ascii_digit).count();
    if (1..=5).contains(&number_len) {
        let number = body.get(..number_len)?;
        let text = body.get(number_len..)?.strip_prefix(". ")?;
        return Some((depth, ListMarker::Number(number.to_owned()), text));
    }
    None
}

pub(crate) fn bounded_source(source: &str, max_chars: usize) -> (&str, bool) {
    match source.char_indices().nth(max_chars) {
        Some((end, _)) => (&source[..end], true),
        None => (source, false),
    }
}

#[cfg(test)]
pub(crate) fn parse(source: &str) -> (Vec<Block>, bool) {
    let (source, truncated) = bounded_source(source, MAX_SOURCE_CHARS);
    parse_bounded(source, truncated)
}

#[derive(Clone, Copy)]
pub(crate) struct CodeFence {
    pub(crate) marker: u8,
    pub(crate) length: usize,
}

pub(crate) fn fence_line(line: &str) -> Option<(CodeFence, &str)> {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }
    let body = &line[indent..];
    let marker = *body.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let length = body.bytes().take_while(|byte| *byte == marker).count();
    (length >= 3).then(|| (CodeFence { marker, length }, &body[length..]))
}

pub(crate) fn pending_fence_line(line: &str) -> bool {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    indent <= 3 && matches!(line[indent..].trim_end(), "`" | "``" | "~" | "~~")
}

#[cfg(test)]
pub(crate) fn parse_bounded(source: &str, truncated: bool) -> (Vec<Block>, bool) {
    parse_bounded_mode(source, truncated, false, MAX_BLOCKS)
}

pub(crate) fn parse_bounded_mode(
    source: &str,
    mut truncated: bool,
    streaming: bool,
    max_blocks: usize,
) -> (Vec<Block>, bool) {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut code = Vec::new();
    let mut language = String::new();
    let mut fence: Option<CodeFence> = None;
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        if blocks.len() >= max_blocks {
            truncated = true;
            break;
        }
        let trimmed = line.trim_end();
        if let Some(open) = fence {
            let closes = fence_line(trimmed).is_some_and(|(close, suffix)| {
                close.marker == open.marker
                    && close.length >= open.length
                    && suffix.trim().is_empty()
            });
            if closes {
                blocks.push(Block::Code(std::mem::take(&mut language), code.join("\n")));
                code.clear();
                fence = None;
            } else {
                code.push(line.to_owned());
            }
            continue;
        }
        if let Some((opening, info)) = fence_line(trimmed) {
            // Backticks in an info string make this an ordinary text line,
            // and another language/fence inside a code block is code itself.
            if opening.marker != b'`' || !info.contains('`') {
                flush_paragraph(&mut blocks, &mut paragraph);
                language = info
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(24)
                    .collect();
                fence = Some(opening);
                continue;
            }
        }
        // A delimiter arriving one token at a time must not flash as prose
        // just before it becomes a fence. Only the unfinished final line is
        // provisional; final answers always retain literal source semantics.
        if streaming
            && !source.ends_with('\n')
            && lines.peek().is_none()
            && pending_fence_line(line)
        {
            continue;
        }
        if trimmed.trim().is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
        } else if let Some((level, text)) = heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(Block::Heading(level, text.to_owned()));
        } else if let Some((depth, marker, text)) = list_item(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(Block::ListItem(depth, marker, text.to_owned()));
        } else if let Some(text) = trimmed.trim_start().strip_prefix("> ") {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(Block::Quote(text.to_owned()));
        } else if matches!(trimmed.trim(), "---" | "***" | "___") {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(Block::Rule);
        } else {
            let hard_break = line.ends_with("  ") || line.ends_with('\\');
            let mut value = trimmed.trim().to_owned();
            if hard_break {
                value = value.trim_end_matches('\\').to_owned();
                value.push('\n');
            }
            paragraph.push(value);
        }
    }
    if fence.is_some() {
        blocks.push(Block::OpenCode(language, code.join("\n")));
    }
    flush_paragraph(&mut blocks, &mut paragraph);
    if blocks.len() > max_blocks {
        blocks.truncate(max_blocks);
        truncated = true;
    }
    (blocks, truncated)
}

pub(crate) fn bounded_lookahead(value: &str) -> &str {
    let end = value
        .char_indices()
        .find(|(index, _)| *index >= MAX_MARKUP_LOOKAHEAD)
        .map_or(value.len(), |(index, _)| index);
    &value[..end]
}

pub(crate) fn web_url(value: &str) -> Option<url::Url> {
    if value.len() > MAX_URL_BYTES
        || value
            .chars()
            .any(|ch| ch.is_whitespace() || ch.is_control() || ch == '\\')
    {
        return None;
    }
    let url = url::Url::parse(value).ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then_some(url)
}

pub(crate) fn safe_url(value: &str) -> bool {
    web_url(value).is_some()
}

pub(crate) fn link_icon(url: &url::Url) -> Icon {
    match url.host_str() {
        Some("github.com" | "www.github.com") => Icon::Github,
        _ => Icon::Globe,
    }
}

pub(crate) fn url_token_len(value: &str) -> usize {
    let mut end = 0;
    let mut balance = [0i32; 3];
    for ch in value.chars() {
        if ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'' | '`') {
            break;
        }
        end += ch.len_utf8();
        if end > MAX_URL_BYTES {
            return 0;
        }
        match ch {
            '(' => balance[0] += 1,
            ')' => balance[0] -= 1,
            '[' => balance[1] += 1,
            ']' => balance[1] -= 1,
            '{' => balance[2] += 1,
            '}' => balance[2] -= 1,
            _ => {}
        }
    }
    let mut candidate = &value[..end];
    while let Some(last) = candidate.chars().last() {
        let closing = match last {
            ')' => Some(0),
            ']' => Some(1),
            '}' => Some(2),
            _ => None,
        };
        if matches!(last, '.' | ',' | ';' | ':' | '!' | '?')
            || closing.is_some_and(|index| balance[index] < 0)
        {
            if let Some(index) = closing {
                balance[index] += 1;
            }
            candidate = &candidate[..candidate.len() - last.len_utf8()];
        } else {
            break;
        }
    }
    candidate.len()
}

pub(crate) fn begins_web_url(value: &str) -> bool {
    value
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        || value
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
}

pub(crate) fn web_url_boundary(previous: Option<char>) -> bool {
    previous.is_none_or(|ch| !ch.is_alphanumeric() && !matches!(ch, '_' | '/' | '\\'))
}

/// Character ranges refer to the unchanged editable source, including any
/// preceding emoji or Markdown syntax. Work is bounded independently of a
/// potentially large pasted draft, and every URL is parsed without a fetch.
pub(crate) fn web_links(source: &str) -> Vec<(Range<usize>, url::Url)> {
    let (scan_source, _) = bounded_source(source, MAX_SOURCE_CHARS);
    let mut links = Vec::new();
    let mut cursor = scan_source.char_indices().enumerate().peekable();
    let mut previous = None;
    while let Some((offset, (byte, ch))) = cursor.next() {
        if web_url_boundary(previous) && begins_web_url(&source[byte..]) {
            let length = url_token_len(&source[byte..]);
            if let Some(url) = (length > 0)
                .then(|| web_url(&source[byte..byte + length]))
                .flatten()
            {
                let end = offset + source[byte..byte + length].chars().count();
                links.push((offset..end, url));
                while cursor
                    .peek()
                    .is_some_and(|(_, (next, _))| *next < byte + length)
                {
                    previous = cursor.next().map(|(_, (_, ch))| ch);
                }
                if links.len() >= 64 {
                    break;
                }
                continue;
            }
        }
        previous = Some(ch);
    }
    links
}

pub(crate) fn markdown_destination_end(value: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in bounded_lookahead(value).char_indices() {
        match ch {
            '(' => depth += 1,
            ')' if depth == 0 => return Some(index),
            ')' => depth -= 1,
            _ => {}
        }
    }
    None
}

pub(crate) fn flush_plain(runs: &mut Vec<InlineRun>, plain: &mut String) {
    push_run(runs, InlineKind::Normal, plain);
    plain.clear();
}

#[cfg(test)]
pub(crate) fn parse_inline(source: &str) -> Vec<InlineRun> {
    parse_inline_mode(source, false)
}

#[cfg(test)]
thread_local! {
    pub(crate) static INLINE_PARSE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn parse_inline_mode(source: &str, incomplete: bool) -> Vec<InlineRun> {
    parse_inline_with_tail(source, incomplete, false)
}

pub(crate) fn parse_inline_with_tail(
    source: &str,
    incomplete: bool,
    url_tail_complete: bool,
) -> Vec<InlineRun> {
    #[cfg(test)]
    INLINE_PARSE_COUNT.with(|count| count.set(count.get() + 1));
    let mut runs = Vec::new();
    let mut plain = String::new();
    let mut index = 0;
    while index < source.len() {
        let rest = &source[index..];
        if let Some(escaped) = rest.strip_prefix('\\') {
            if let Some(character) = escaped.chars().next().filter(|character| {
                matches!(character, '*' | '_' | '`' | '[' | ']' | '(' | ')' | '\\')
            }) {
                plain.push(character);
                index += 1 + character.len_utf8();
                continue;
            }
        }
        let delimiter = if rest.starts_with('`') {
            let count = rest.bytes().take_while(|byte| *byte == b'`').count();
            Some((&rest[..count], InlineKind::Code))
        } else if rest.starts_with("**") {
            Some(("**", InlineKind::Strong))
        } else if rest.starts_with('*') {
            Some(("*", InlineKind::Emphasis))
        } else {
            None
        };
        if let Some((delimiter, kind)) = delimiter {
            let body = &rest[delimiter.len()..];
            let lookahead = bounded_lookahead(body);
            if let Some(end) = lookahead.find(delimiter).filter(|end| *end > 0) {
                flush_plain(&mut runs, &mut plain);
                push_run(&mut runs, kind, &body[..end]);
                index += end + 2 * delimiter.len();
                continue;
            }
            if incomplete
                && lookahead.len() == body.len()
                && (body.is_empty() || !body.starts_with(char::is_whitespace))
            {
                flush_plain(&mut runs, &mut plain);
                // A single star can be the first half of a closing ** pair.
                let body = if delimiter == "**" {
                    body.strip_suffix('*').unwrap_or(body)
                } else {
                    body
                };
                push_run(&mut runs, kind, body);
                break;
            }
            if kind == InlineKind::Code {
                // Treat an unmatched delimiter run atomically. Rechecking a
                // 20,000-backtick run at each following byte is quadratic.
                plain.push_str(delimiter);
                index += delimiter.len();
                continue;
            }
        }
        if let Some(body) = rest.strip_prefix('[') {
            if let Some(label_end) = bounded_lookahead(body).find("](") {
                let tail = &body[label_end + 2..];
                if let Some(url_end) = markdown_destination_end(tail) {
                    let url = &tail[..url_end];
                    if !body[..label_end].is_empty() && safe_url(url) {
                        flush_plain(&mut runs, &mut plain);
                        runs.push(InlineRun {
                            kind: InlineKind::Link,
                            text: body[..label_end].to_owned(),
                            destination: Some(url.to_owned()),
                        });
                        index += label_end + url_end + 4;
                        continue;
                    }
                }
            }
        }
        if begins_web_url(rest) && web_url_boundary(source[..index].chars().next_back()) {
            let length = url_token_len(rest);
            let destination = &rest[..length];
            if length > 0 && safe_url(destination) {
                flush_plain(&mut runs, &mut plain);
                // A live token can still grow into a different host or path.
                // Color is provisional; navigation requires a delimiter or a
                // completed response, so github.co cannot open before its m.
                let after_token = rest[length..]
                    .chars()
                    .find(|ch| !matches!(ch, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}'));
                let complete = !incomplete
                    || url_tail_complete
                    || after_token.is_some_and(|ch| {
                        ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'' | '`')
                    });
                runs.push(InlineRun {
                    kind: InlineKind::Link,
                    text: destination.to_owned(),
                    destination: complete.then(|| destination.to_owned()),
                });
                index += length;
                continue;
            }
        }
        if incomplete {
            if let Some(body) = rest.strip_prefix('[') {
                let bounded = bounded_lookahead(body);
                if bounded.len() == body.len() {
                    let provisional_label = if let Some(label_end) = body.find("](") {
                        (!body[label_end + 2..].contains(')')).then_some(&body[..label_end])
                    } else if let Some(label) = body.strip_suffix(']') {
                        Some(label)
                    } else if !body.contains(']') {
                        Some(body)
                    } else {
                        None
                    };
                    if let Some(label) = provisional_label {
                        flush_plain(&mut runs, &mut plain);
                        push_run(&mut runs, InlineKind::Link, label);
                        break;
                    }
                }
            }
        }
        let character = rest.chars().next().expect("index is inside source");
        plain.push(character);
        index += character.len_utf8();
    }
    flush_plain(&mut runs, &mut plain);
    runs
}
