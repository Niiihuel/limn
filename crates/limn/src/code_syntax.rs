//! Lazy, shared background syntax analysis. Buffers use native language parsers;
//! diff fragments use a bounded lexer with independent old/new structural states.
use crate::{CodeHighlight, Color};
use limn_editor::BufferSnapshot;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, OnceLock, Weak};

#[cfg(feature = "syntax")]
pub use limn_language::Language as CodeLanguage;

/// Same variants as `limn_language::Language`, for builds without grammars.
#[cfg(not(feature = "syntax"))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum CodeLanguage {
    /// Text in no language: nothing is colored. The default.
    #[default]
    Plain,
    /// Rust source.
    Rust,
    /// JSON.
    Json,
    /// JavaScript, with or without JSX.
    JavaScript,
    /// TypeScript.
    TypeScript,
    /// TypeScript with JSX.
    Tsx,
    /// Python source.
    Python,
    /// TOML.
    Toml,
    /// A CSS style sheet.
    Css,
    /// HTML markup.
    Html,
    /// Markdown.
    Markdown,
}

#[cfg(not(feature = "syntax"))]
impl CodeLanguage {
    /// Without the `syntax` feature every code fence is plain text.
    pub fn from_code_fence(_info: &str) -> Self {
        Self::Plain
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct LexState {
    block_depth: usize,
    quote: Option<char>,
    triple: bool,
    raw_hashes: Option<usize>,
    markdown_fence: Option<(char, usize, CodeLanguage)>,
}
const COMMENT: Color = Color::rgb(126, 151, 119);
const STRING: Color = Color::rgb(206, 163, 127);
const KEYWORD: Color = Color::rgb(184, 143, 216);
const NUMBER: Color = Color::rgb(151, 193, 169);

enum Source {
    Buffer(BufferSnapshot),
    Diff(Arc<[crate::CodeDiffLine]>),
}
impl Source {
    fn key(&self, language: CodeLanguage) -> (u64, u64, CodeLanguage) {
        match self {
            Self::Buffer(snapshot) => (snapshot.document_id(), snapshot.revision(), language),
            Self::Diff(lines) => (
                Arc::as_ptr(lines) as *const crate::CodeDiffLine as usize as u64,
                lines.len() as u64,
                language,
            ),
        }
    }
    fn count(&self) -> usize {
        match self {
            Self::Buffer(snapshot) => snapshot.line_count(),
            Self::Diff(lines) => lines.len(),
        }
        .min(100_000)
    }
}
struct Job {
    source: Source,
    language: CodeLanguage,
    generation: u64,
    previous: Option<Arc<ResultRows>>,
    invalid_from: usize,
    current: Arc<AtomicU64>,
    result_slot: Weak<Mutex<Option<Arc<ResultRows>>>>,
}
struct ResultRows {
    document_id: u64,
    revision: u64,
    language: CodeLanguage,
    rows: Arc<[Vec<CodeHighlight>]>,
    end_states: Arc<[LexState]>,
    _diff_identity: Option<Arc<[crate::CodeDiffLine]>>,
}
pub(crate) struct SyntaxCache {
    results: Arc<Mutex<Option<Arc<ResultRows>>>>,
    generation: Arc<AtomicU64>,
    requested: Option<(u64, u64, CodeLanguage)>,
    desired: Option<(u64, u64, CodeLanguage)>,
    ready: Option<Arc<ResultRows>>,
    invalid_from: Option<usize>,
}
impl SyntaxCache {
    /// An empty cache with nothing requested.
    pub fn new() -> Self {
        let results = Arc::new(Mutex::new(None));
        let generation = Arc::new(AtomicU64::new(0));
        Self {
            results,
            generation,
            requested: None,
            desired: None,
            ready: None,
            invalid_from: None,
        }
    }
    /// Ask for the highlights of this buffer snapshot in `language`. A job is sent to the
    /// background analyser only when the document, revision or language differs from the
    /// last one sent; `Plain` cancels any analysis instead.
    pub fn update(&mut self, snapshot: &BufferSnapshot, language: CodeLanguage) {
        self.request(Source::Buffer(snapshot.clone()), language);
    }
    /// As `update`, for the lines of a diff. Two diffs are told apart by their allocation
    /// and their length.
    pub fn update_diff(&mut self, lines: Arc<[crate::CodeDiffLine]>, language: CodeLanguage) {
        self.request(Source::Diff(lines), language);
    }
    /// Abandons any analysis in flight; the caller has another source of rows.
    pub fn release(&mut self) {
        if self.desired.take().is_some() {
            self.generation.fetch_add(1, Ordering::Release);
            self.requested = None;
        }
    }
    /// Record the first row that changed. The lowest row recorded is sent with the next
    /// analysis, next to the previous result.
    pub fn invalidate_from(&mut self, row: usize) {
        self.invalid_from = Some(self.invalid_from.map_or(row, |previous| previous.min(row)));
    }
    fn request(&mut self, source: Source, language: CodeLanguage) {
        if let Ok(mut slot) = self.results.lock() {
            if let Some(result) = slot.take() {
                self.ready = Some(result);
            }
        }
        if language == CodeLanguage::Plain {
            self.generation.fetch_add(1, Ordering::Release);
            self.requested = None;
            self.desired = None;
            return;
        }
        let key = source.key(language);
        self.desired = Some(key);
        if self
            .ready
            .as_ref()
            .is_some_and(|ready| (ready.document_id, ready.revision, ready.language) == key)
        {
            self.invalid_from = None;
        }
        if self.requested != Some(key) {
            let generation = self
                .generation
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
            if syntax_pool()
                .0
                .try_send(Job {
                    previous: if matches!(&source, Source::Buffer(_)) {
                        self.ready.clone()
                    } else {
                        None
                    },
                    invalid_from: self.invalid_from.unwrap_or(0),
                    source,
                    language,
                    generation,
                    current: self.generation.clone(),
                    result_slot: Arc::downgrade(&self.results),
                })
                .is_ok()
            {
                self.requested = Some(key);
                #[cfg(target_arch = "wasm32")]
                work(&syntax_pool().1, false);
            } else {
                self.requested = None;
            }
        }
    }
    /// The highlights of `row` in this snapshot. Empty while the ready result is for
    /// another document, revision or language.
    pub fn line(
        &self,
        snapshot: &BufferSnapshot,
        language: CodeLanguage,
        row: usize,
    ) -> &[CodeHighlight] {
        self.line_key((snapshot.document_id(), snapshot.revision(), language), row)
    }
    /// As `line`, for a row of a diff.
    pub fn diff_line(
        &self,
        lines: &Arc<[crate::CodeDiffLine]>,
        language: CodeLanguage,
        row: usize,
    ) -> &[CodeHighlight] {
        self.line_key(
            (
                Arc::as_ptr(lines) as *const crate::CodeDiffLine as usize as u64,
                lines.len() as u64,
                language,
            ),
            row,
        )
    }
    fn line_key(&self, key: (u64, u64, CodeLanguage), row: usize) -> &[CodeHighlight] {
        self.ready
            .as_ref()
            .filter(|ready| (ready.document_id, ready.revision, ready.language) == key)
            .and_then(|ready| ready.rows.get(row))
            .map_or(&[], Vec::as_slice)
    }
    /// Whether a requested analysis has not produced its result yet.
    pub fn pending(&self) -> bool {
        self.desired.is_some_and(|key| {
            self.ready
                .as_ref()
                .is_none_or(|ready| (ready.document_id, ready.revision, ready.language) != key)
        })
    }
}
impl Drop for SyntaxCache {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::Release);
    }
}

#[cfg(feature = "syntax")]
fn highlight_color(kind: limn_language::HighlightKind) -> Color {
    use limn_language::HighlightKind;
    match kind {
        HighlightKind::Comment => COMMENT,
        HighlightKind::String => STRING,
        HighlightKind::Keyword => KEYWORD,
        HighlightKind::Number => NUMBER,
        HighlightKind::Function => Color::rgb(143, 185, 218),
        HighlightKind::Type => Color::rgb(225, 196, 131),
        HighlightKind::Property => Color::rgb(160, 191, 217),
        HighlightKind::Variable | HighlightKind::Operator => Color::rgb(205, 205, 205),
    }
}
/// Works the queue until it closes or, when not waiting, until it is empty.
fn work(jobs: &Mutex<mpsc::Receiver<Job>>, wait: bool) {
        #[cfg(feature = "syntax")]
        let mut parser = limn_language::ParserEngine::default();
        loop {
            let job = {
                let jobs = jobs.lock().expect("syntax job queue poisoned");
                if wait { jobs.recv().ok() } else { jobs.try_recv().ok() }
            };
            let Some(job) = job else {
                break;
            };
            let current = &job.current;
            let Some(result_slot) = job.result_slot.upgrade() else {
                continue;
            };
            if current.load(Ordering::Acquire) != job.generation {
                continue;
            }
            #[cfg(feature = "syntax")]
            if let Source::Buffer(snapshot) = &job.source {
                let cancel = limn_language::Cancellation::for_generation(
                    job.current.clone(),
                    job.generation,
                );
                let analysis = parser.analyze(snapshot, job.language, &cancel);
                if cancel.is_cancelled() {
                    continue;
                }
                let count = job.source.count();
                let mut rows = vec![Vec::new(); count];
                if let Ok(analysis) = analysis {
                    for span in analysis.highlights.iter() {
                        let Ok(start) = snapshot.position_of(span.range.start) else {
                            continue;
                        };
                        let Ok(end) = snapshot.position_of(span.range.end) else {
                            continue;
                        };
                        for (row, highlights) in rows
                            .iter_mut()
                            .enumerate()
                            .take(end.row.saturating_add(1))
                            .skip(start.row)
                        {
                            let Ok(range) = snapshot.line_range(row) else {
                                continue;
                            };
                            let clipped = span.range.start.max(range.start)
                                ..span.range.end.min(range.end);
                            if clipped.start < clipped.end {
                                highlights.push(CodeHighlight {
                                    range: clipped,
                                    color: highlight_color(span.kind),
                                });
                            }
                        }
                    }
                }
                if let Ok(mut slot) = result_slot.lock() {
                    if !cancel.is_cancelled() {
                        *slot = Some(Arc::new(ResultRows {
                            document_id: snapshot.document_id(),
                            revision: snapshot.revision(),
                            language: job.language,
                            rows: rows.into(),
                            end_states: Arc::from([]),
                            _diff_identity: None,
                        }));
                    }
                }
                continue;
            }
            let key = job.source.key(job.language);
            let previous = job.previous.as_ref().filter(|previous| {
                previous.document_id == key.0 && previous.language == key.2
            });
            let prefix = previous.map_or(0, |previous| {
                job.invalid_from
                    .min(previous.rows.len())
                    .min(job.source.count())
            });
            let mut state = previous
                .and_then(|previous| previous.end_states.get(prefix.saturating_sub(1)))
                .copied()
                .filter(|_| prefix > 0)
                .unwrap_or_default();
            let mut old_state = LexState::default();
            let mut rows = previous
                .map_or_else(Vec::new, |previous| previous.rows[..prefix].to_vec());
            let mut end_states = previous.map_or_else(Vec::new, |previous| {
                previous.end_states[..prefix].to_vec()
            });
            let mut spans = rows.iter().map(Vec::len).sum::<usize>();
            for row in prefix..job.source.count() {
                if current.load(Ordering::Acquire) != job.generation {
                    break;
                }
                let line = match &job.source {
                    // Without grammars a buffer is colored a line at a time, as a diff is.
                    Source::Buffer(snapshot) => match (snapshot.line_text(row), snapshot.line_range(row)) {
                        (Ok(text), Ok(range)) => highlight_line(&text, range.start, job.language, &mut state),
                        _ => Vec::new(),
                    },
                    Source::Diff(lines) => {
                        let line = &lines[row];
                        let text = line.text.trim_end_matches(['\r', '\n']);
                        if line.kind == crate::CodeDiffLineKind::Folded
                            || (line.old_line.is_none() && line.new_line.is_none())
                        {
                            state = LexState::default();
                            old_state = LexState::default();
                            rows.push(Vec::new());
                            end_states.push(state);
                            continue;
                        }
                        match line.kind {
                            crate::CodeDiffLineKind::Added => {
                                highlight_line(text, 0, job.language, &mut state)
                            }
                            crate::CodeDiffLineKind::Removed => {
                                highlight_line(text, 0, job.language, &mut old_state)
                            }
                            // Folded rows restart the state above.
                            crate::CodeDiffLineKind::Context | crate::CodeDiffLineKind::Folded => {
                                let _ = highlight_line(
                                    text,
                                    0,
                                    job.language,
                                    &mut old_state,
                                );
                                highlight_line(text, 0, job.language, &mut state)
                            }
                        }
                    }
                };
                spans = spans.saturating_add(line.len());
                // Fixed limits independent of input token density. Remaining
                // rows stay plain; structural state has still been parsed.
                rows.push(if spans <= 262_144 { line } else { Vec::new() });
                end_states.push(state);
            }
            if current.load(Ordering::Acquire) == job.generation
                && rows.len() == job.source.count()
            {
                let result = ResultRows {
                    document_id: key.0,
                    revision: key.1,
                    language: job.language,
                    rows: Arc::from(rows),
                    end_states: Arc::from(end_states),
                    _diff_identity: match &job.source {
                        Source::Diff(lines) => Some(lines.clone()),
                        Source::Buffer(_) => None,
                    },
                };
                // A single result slot bounds memory. Never block a cancelled
                // worker behind a view that is no longer rendered.
                if let Ok(mut slot) = result_slot.lock() {
                    *slot = Some(Arc::new(result));
                }
            }
        }
}

type Pool = (mpsc::SyncSender<Job>, Arc<Mutex<mpsc::Receiver<Job>>>);

fn syntax_pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let (sender, jobs) = mpsc::sync_channel::<Job>(8);
        let jobs = Arc::new(Mutex::new(jobs));
        // A browser has no threads to give: there the caller works the queue.
        #[cfg(not(target_arch = "wasm32"))]
        for index in 0..2 {
            let jobs = jobs.clone();
            std::thread::Builder::new()
                .name(format!("code-syntax-{index}"))
                .spawn(move || work(&jobs, true))
                .expect("syntax worker thread creation failed");
        }
        (sender, jobs)
    })
}

fn highlight_line(
    text: &str,
    start: usize,
    language: CodeLanguage,
    state: &mut LexState,
) -> Vec<CodeHighlight> {
    if language == CodeLanguage::Plain {
        return Vec::new();
    }
    // A minified or generated line must not allocate millions of chars on a worker.
    // Reset state because the skipped suffix cannot be interpreted safely.
    if text.len() > 256 * 1024 {
        *state = LexState::default();
        return Vec::new();
    }
    if language == CodeLanguage::Markdown {
        return highlight_markdown_line(text, start, state);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let begin = index;
        let color;
        if state.block_depth > 0 {
            while index < chars.len() {
                if chars.get(index) == Some(&'/')
                    && chars.get(index + 1) == Some(&'*')
                    && language == CodeLanguage::Rust
                {
                    state.block_depth = state.block_depth.saturating_add(1);
                    index += 2;
                } else if chars.get(index) == Some(&'*') && chars.get(index + 1) == Some(&'/') {
                    state.block_depth -= 1;
                    index += 2;
                    if state.block_depth == 0 {
                        break;
                    }
                } else {
                    index += 1;
                }
            }
            color = COMMENT;
        } else if let Some(hashes) = state.raw_hashes {
            while index < chars.len() {
                if chars[index] == '"'
                    && (0..hashes).all(|n| chars.get(index + 1 + n) == Some(&'#'))
                {
                    index += 1 + hashes;
                    state.raw_hashes = None;
                    break;
                }
                index += 1;
            }
            color = STRING;
        } else if let Some(quote) = state.quote {
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                if chars[index] == quote
                    && (!state.triple
                        || (chars.get(index + 1) == Some(&quote)
                            && chars.get(index + 2) == Some(&quote)))
                {
                    index += if state.triple { 3 } else { 1 };
                    state.quote = None;
                    state.triple = false;
                    break;
                }
                index += 1;
            }
            color = STRING;
        } else if chars.get(index) == Some(&'/')
            && chars.get(index + 1) == Some(&'*')
            && matches!(
                language,
                CodeLanguage::Rust
                    | CodeLanguage::JavaScript
                    | CodeLanguage::TypeScript
                    | CodeLanguage::Tsx
                    | CodeLanguage::Css
            )
        {
            state.block_depth = 1;
            index += 2;
            if spans.len() < 262_144 {
                spans.push(CodeHighlight {
                    range: start + begin..start + index,
                    color: COMMENT,
                });
            }
            continue;
        } else if (chars.get(index) == Some(&'/')
            && chars.get(index + 1) == Some(&'/')
            && matches!(
                language,
                CodeLanguage::Rust
                    | CodeLanguage::JavaScript
                    | CodeLanguage::TypeScript
                    | CodeLanguage::Tsx
                    | CodeLanguage::Json
            ))
            || (chars[index] == '#'
                && matches!(language, CodeLanguage::Python | CodeLanguage::Toml))
        {
            index = chars.len();
            color = COMMENT;
        } else if chars[index] == 'r'
            && language == CodeLanguage::Rust
            && (chars.get(index + 1) == Some(&'"') || chars.get(index + 1) == Some(&'#'))
        {
            let mut end = index + 1;
            while chars.get(end) == Some(&'#') {
                end += 1;
            }
            if chars.get(end) == Some(&'"') {
                state.raw_hashes = Some(end - index - 1);
                index = end + 1;
                if spans.len() < 262_144 {
                    spans.push(CodeHighlight {
                        range: start + begin..start + index,
                        color: STRING,
                    });
                }
                continue;
            } else {
                index += 1;
                continue;
            }
        } else if matches!(chars[index], '"' | '\'' | '`') {
            let quote = chars[index];
            // A Rust lifetime is not a character/string literal.
            if language == CodeLanguage::Rust
                && quote == '\''
                && chars
                    .get(index + 1)
                    .is_some_and(|character| character.is_alphabetic())
                && chars.get(index + 2) != Some(&'\'')
            {
                index += 1;
                continue;
            }
            state.quote = Some(quote);
            state.triple = matches!(language, CodeLanguage::Python | CodeLanguage::Toml)
                && chars.get(index + 1) == Some(&quote)
                && chars.get(index + 2) == Some(&quote);
            index += if state.triple { 3 } else { 1 };
            if spans.len() < 262_144 {
                spans.push(CodeHighlight {
                    range: start + begin..start + index,
                    color: STRING,
                });
            }
            continue;
        } else if chars[index].is_ascii_digit() {
            index += 1;
            while chars.get(index).is_some_and(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '.')
            }) {
                index += 1;
            }
            color = NUMBER;
        } else if chars[index].is_alphabetic() || chars[index] == '_' {
            index += 1;
            while chars
                .get(index)
                .is_some_and(|character| character.is_alphanumeric() || *character == '_')
            {
                index += 1;
            }
            let word: String = chars[begin..index].iter().collect();
            if !is_keyword(&word, language) {
                continue;
            }
            color = KEYWORD;
        } else {
            index += 1;
            continue;
        }
        if index > begin && spans.len() < 262_144 {
            spans.push(CodeHighlight {
                range: start + begin..start + index,
                color,
            });
        }
    }
    // Ordinary single-line strings cannot color an unrelated following line.
    if !state.triple
        && state.quote != Some('`')
        && !(language == CodeLanguage::Rust && state.quote == Some('"'))
    {
        state.quote = None;
    }
    spans
}
fn highlight_markdown_line(text: &str, start: usize, state: &mut LexState) -> Vec<CodeHighlight> {
    let trimmed = text.trim_start_matches(' ');
    let indent = text.len() - trimmed.len();
    let delimiter = trimmed.chars().next().filter(|ch| matches!(ch, '`' | '~'));
    let length = delimiter.map_or(0, |delimiter| {
        trimmed.chars().take_while(|ch| *ch == delimiter).count()
    });
    let fence = indent <= 3 && length >= 3;
    if let Some((marker, minimum, language)) = state.markdown_fence {
        if fence
            && delimiter == Some(marker)
            && length >= minimum
            && trimmed[length..].trim().is_empty()
        {
            *state = LexState::default();
            return vec![CodeHighlight {
                range: start..start + text.chars().count(),
                color: KEYWORD,
            }];
        }
        if matches!(language, CodeLanguage::Plain | CodeLanguage::Markdown) {
            return vec![CodeHighlight {
                range: start..start + text.chars().count(),
                color: STRING,
            }];
        }
        // Reuse the bounded code lexer and preserve its multiline state inside a fence.
        return highlight_line(text, start, language, state);
    }
    if fence {
        let language = CodeLanguage::from_code_fence(&trimmed[length..]);
        *state = LexState {
            markdown_fence: Some((delimiter.unwrap(), length, language)),
            ..LexState::default()
        };
        return vec![CodeHighlight {
            range: start..start + text.chars().count(),
            color: KEYWORD,
        }];
    }
    // Diff fragments have no whole-document snapshot. Reuse the same inline parser
    // instead of maintaining a second, subtly different Markdown regular expression.
    #[cfg(not(feature = "syntax"))]
    return Vec::new();
    #[cfg(feature = "syntax")]
    {
    let Ok(buffer) = limn_editor::TextBuffer::new(text) else {
        return Vec::new();
    };
    limn_language::ParserEngine::default()
        .analyze(
            &buffer.snapshot(),
            CodeLanguage::Markdown,
            &Default::default(),
        )
        .map(|analysis| {
            analysis
                .highlights
                .iter()
                .map(|highlight| CodeHighlight {
                    range: start + highlight.range.start..start + highlight.range.end,
                    color: highlight_color(highlight.kind),
                })
                .collect()
        })
        .unwrap_or_default()
    }
}

fn is_keyword(word: &str, language: CodeLanguage) -> bool {
    let words=match language {
        CodeLanguage::Rust=>"as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while",
        CodeLanguage::Json=>"true false null",
        CodeLanguage::Python=>"and as assert async await break class continue def del elif else except False finally for from global if import in is lambda None nonlocal not or pass raise return True try while with yield",
        CodeLanguage::JavaScript|CodeLanguage::TypeScript|CodeLanguage::Tsx=>"async await break case catch class const continue debugger default delete do else export extends false finally for from function if import in instanceof interface let new null of private protected public return static super switch this throw true try type typeof undefined var void while yield",
        CodeLanguage::Toml=>"true false", _=>"",
    };
    words.split_whitespace().any(|candidate| candidate == word)
}

#[cfg(all(test, feature = "syntax"))]
mod tests {
    use super::*;
    use limn_editor::{EditKind, Selection, TextBuffer};
    fn wait(cache: &mut SyntaxCache, snapshot: &BufferSnapshot, language: CodeLanguage) {
        let until = web_time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            cache.update(snapshot, language);
            if !cache.pending() {
                break;
            }
            assert!(
                web_time::Instant::now() < until,
                "lexical worker exceeded bounded test wait"
            );
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn multiline_comments_raw_strings_and_triple_quotes_carry_state() {
        for (language, lines) in [
            (
                CodeLanguage::Rust,
                [
                    "/* begin",
                    "still comment */ let x = r###\"raw",
                    "raw tail\"###; fn done() {}",
                ],
            ),
            (
                CodeLanguage::Python,
                ["x = \"\"\"start", "still string", "end\"\"\"; return True"],
            ),
        ] {
            let mut state = LexState::default();
            let first = highlight_line(lines[0], 0, language, &mut state);
            assert!(!first.is_empty());
            let second = highlight_line(lines[1], 100, language, &mut state);
            assert!(!second.is_empty());
            assert_eq!(second[0].range.start, 100);
            let final_line = highlight_line(lines[2], 200, language, &mut state);
            assert!(!final_line.is_empty());
            assert!(state.block_depth == 0 && state.quote.is_none() && state.raw_hashes.is_none());
        }
    }
    #[test]
    fn unicode_ranges_and_lifetimes_preserve_source_coordinates() {
        let mut state = LexState::default();
        let spans = highlight_line(
            "let café = \"🧠\"; &'a str",
            20,
            CodeLanguage::Rust,
            &mut state,
        );
        assert_eq!(spans[0].range, 20..23);
        assert_eq!(state.quote, None);
        assert!(spans
            .iter()
            .all(|span| span.range.end <= 20 + "let café = \"🧠\"; &'a str".chars().count()));
    }
    #[test]
    fn background_lexing_uses_predecessor_state_for_scrolled_viewports_and_invalidates_from_edit() {
        let mut buffer = TextBuffer::new("let keep=1;\n/* open\nbody\n*/ let tail=3;\n").unwrap();
        let mut cache = SyntaxCache::new();
        let first = buffer.snapshot();
        wait(&mut cache, &first, CodeLanguage::Rust);
        assert_eq!(cache.line(&first, CodeLanguage::Rust, 2)[0].color, COMMENT);
        let preserved = cache.line(&first, CodeLanguage::Rust, 0)[0].range.clone();
        let range = first.line_range(1).unwrap();
        buffer
            .edit(
                range.clone(),
                "let now=2;",
                Selection {
                    anchor: range.start,
                    head: range.start,
                },
                EditKind::Replace,
                0,
            )
            .unwrap();
        cache.invalidate_from(1);
        let second = buffer.snapshot();
        cache.update(&second, CodeLanguage::Rust);
        assert!(cache.line(&second, CodeLanguage::Rust, 2).is_empty());
        wait(&mut cache, &second, CodeLanguage::Rust);
        assert!(cache
            .line(&second, CodeLanguage::Rust, 2)
            .iter()
            .all(|span| span.color != COMMENT));
        assert_eq!(
            cache.line(&second, CodeLanguage::Rust, 0)[0].range,
            preserved
        );
        assert!(
            cache.ready.as_ref().unwrap().end_states.is_empty(),
            "buffers use the parser tree, not lexical predecessor state"
        );
    }
    #[test]
    fn background_lexing_rejects_other_documents_and_old_revisions_without_stopping_poll() {
        let mut buffer = TextBuffer::new("let old=1;\n".repeat(5000).as_str()).unwrap();
        let mut cache = SyntaxCache::new();
        let first = buffer.snapshot();
        cache.update(&first, CodeLanguage::Rust);
        for index in 0..8 {
            buffer.insert("x", EditKind::Typing, index).unwrap();
            cache.invalidate_from(0);
            cache.update(&buffer.snapshot(), CodeLanguage::Rust);
            assert!(cache.pending());
        }
        let final_snapshot = buffer.snapshot();
        wait(&mut cache, &final_snapshot, CodeLanguage::Rust);
        assert!(cache.line(&first, CodeLanguage::Rust, 0).is_empty());
        let other = TextBuffer::new("let other=9;").unwrap().snapshot();
        assert!(cache.line(&other, CodeLanguage::Rust, 0).is_empty());
        cache.update(&other, CodeLanguage::Rust);
        wait(&mut cache, &other, CodeLanguage::Rust);
        assert!(!cache.line(&other, CodeLanguage::Rust, 0).is_empty());
    }
    #[test]
    fn diff_lexing_keeps_old_new_states_and_resets_on_omitted_hunk_boundaries() {
        use crate::{CodeDiffLine, CodeDiffLineKind as Kind};
        let lines: Arc<[CodeDiffLine]> = vec![
            CodeDiffLine {
                kind: Kind::Removed,
                old_line: Some(1),
                new_line: None,
                text: "/* old".into(),
            },
            CodeDiffLine {
                kind: Kind::Added,
                old_line: None,
                new_line: Some(1),
                text: "let new=1;".into(),
            },
            CodeDiffLine {
                kind: Kind::Removed,
                old_line: Some(2),
                new_line: None,
                text: "body */".into(),
            },
            CodeDiffLine {
                kind: Kind::Context,
                old_line: None,
                new_line: None,
                text: "@@ -90 +90 @@".into(),
            },
            CodeDiffLine {
                kind: Kind::Context,
                old_line: Some(90),
                new_line: Some(90),
                text: "let after=2;".into(),
            },
        ]
        .into();
        let mut cache = SyntaxCache::new();
        let until = web_time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            cache.update_diff(lines.clone(), CodeLanguage::Rust);
            if !cache.pending() {
                break;
            }
            assert!(web_time::Instant::now() < until);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(
            cache.diff_line(&lines, CodeLanguage::Rust, 2)[0].color,
            COMMENT
        );
        assert_eq!(
            cache.diff_line(&lines, CodeLanguage::Rust, 1)[0].color,
            KEYWORD
        );
        assert!(cache.diff_line(&lines, CodeLanguage::Rust, 3).is_empty());
        assert_eq!(
            cache.diff_line(&lines, CodeLanguage::Rust, 4)[0].color,
            KEYWORD
        );
        assert!(Arc::strong_count(&lines) >= 2);
    }
    #[test]
    fn regular_rust_multiline_strings_preserve_quote_state() {
        let mut state = LexState::default();
        highlight_line("let x=\"first", 0, CodeLanguage::Rust, &mut state);
        assert_eq!(state.quote, Some('"'));
        let next = highlight_line("second\"; let y=3;", 30, CodeLanguage::Rust, &mut state);
        assert_eq!(next[0].color, STRING);
        assert_eq!(state.quote, None);
    }

    #[test]
    fn markdown_diff_fences_keep_language_and_multiline_state_separate_from_prose() {
        let mut old = LexState::default();
        let mut new = LexState::default();
        highlight_line("```rust", 0, CodeLanguage::Markdown, &mut old);
        highlight_line("/* viejo", 0, CodeLanguage::Markdown, &mut old);
        let new_heading = highlight_line("# Nuevo 🧠", 50, CodeLanguage::Markdown, &mut new);
        assert_eq!(
            new_heading[0].color,
            highlight_color(limn_language::HighlightKind::Type)
        );
        assert_eq!(new_heading[0].range, 50..59);
        let old_comment = highlight_line("comentario */", 0, CodeLanguage::Markdown, &mut old);
        assert_eq!(old_comment[0].color, COMMENT);
        highlight_line("```", 0, CodeLanguage::Markdown, &mut old);
        assert!(old.markdown_fence.is_none());
        assert!(
            highlight_line("Texto sin formato", 0, CodeLanguage::Markdown, &mut old).is_empty()
        );
        let inline = highlight_line(
            "Café `🧠` y **fuerte**",
            10,
            CodeLanguage::Markdown,
            &mut old,
        );
        assert_eq!(inline[0].range, 15..18);
        assert_eq!(inline[0].color, STRING);
    }

    #[test]
    fn markdown_background_cache_rejects_old_outline_and_recomputes_after_edits() {
        let mut buffer = TextBuffer::new("# Café\n\n```rust\nlet niño = 1;\n```\n").unwrap();
        let mut cache = SyntaxCache::new();
        let before = buffer.snapshot();
        wait(&mut cache, &before, CodeLanguage::Markdown);
        assert_eq!(
            cache.line(&before, CodeLanguage::Markdown, 0)[0].color,
            highlight_color(limn_language::HighlightKind::Type)
        );
        assert!(cache
            .line(&before, CodeLanguage::Markdown, 3)
            .iter()
            .any(|span| span.color == KEYWORD));
        buffer
            .edit(
                0..before.char_count(),
                "# Nuevo 🦀\n\nCafé **fuerte**\n",
                Selection::caret(0),
                EditKind::Replace,
                0,
            )
            .unwrap();
        cache.invalidate_from(0);
        let after = buffer.snapshot();
        cache.update(&after, CodeLanguage::Markdown);
        assert!(cache.line(&after, CodeLanguage::Markdown, 0).is_empty());
        wait(&mut cache, &after, CodeLanguage::Markdown);
        assert!(cache.line(&before, CodeLanguage::Markdown, 0).is_empty());
        assert!(cache
            .line(&after, CodeLanguage::Markdown, 2)
            .iter()
            .any(|span| span.color == highlight_color(limn_language::HighlightKind::Property)));
    }
}
