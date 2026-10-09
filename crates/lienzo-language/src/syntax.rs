use lienzo_editor::BufferSnapshot;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tree_sitter::{InputEdit, Parser, Point, Query, QueryCursor, StreamingIterator, Tree};

#[path = "markdown_syntax.rs"]
mod markdown;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Language {
    #[default]
    Plain,
    Rust,
    Json,
    JavaScript,
    TypeScript,
    Tsx,
    Python,
    Toml,
    Css,
    Html,
    Markdown,
}
impl Language {
    pub fn from_extension(extension: &str) -> Self {
        match extension
            .trim_start_matches('.')
            .to_ascii_lowercase()
            .as_str()
        {
            "rs" => Self::Rust,
            "json" | "jsonc" => Self::Json,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "py" | "pyi" => Self::Python,
            "toml" => Self::Toml,
            "css" | "scss" => Self::Css,
            "html" | "htm" | "xml" | "svg" => Self::Html,
            "md" | "markdown" | "mdown" | "mkd" | "mkdn" | "mdwn" => Self::Markdown,
            _ => Self::Plain,
        }
    }
    pub fn lsp_id(self) -> &'static str {
        match self {
            Self::Plain => "plaintext",
            Self::Rust => "rust",
            Self::Json => "json",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "typescriptreact",
            Self::Python => "python",
            Self::Toml => "toml",
            Self::Css => "css",
            Self::Html => "html",
            Self::Markdown => "markdown",
        }
    }
    /// Common fenced-code aliases; an unknown label stays plain and never runs a tool.
    pub fn from_code_fence(info: &str) -> Self {
        let label = info
            .trim_start_matches('{')
            .trim_start_matches('.')
            .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == '}')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match label.as_str() {
            "rust" => Self::Rust,
            "javascript" | "node" | "nodejs" => Self::JavaScript,
            "typescript" => Self::TypeScript,
            "python" | "python3" => Self::Python,
            "text" | "plaintext" | "plain" => Self::Plain,
            _ => Self::from_extension(&label),
        }
    }
    fn grammar(self) -> Option<(tree_sitter::Language, String)> {
        Some(match self {
            Self::Plain | Self::Markdown => return None,
            Self::Rust => (
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY.into(),
            ),
            Self::Json => (
                tree_sitter_json::LANGUAGE.into(),
                tree_sitter_json::HIGHLIGHTS_QUERY.into(),
            ),
            Self::JavaScript => (
                tree_sitter_javascript::LANGUAGE.into(),
                tree_sitter_javascript::HIGHLIGHT_QUERY.into(),
            ),
            Self::TypeScript => (
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                format!(
                    "{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
            ),
            Self::Tsx => (
                tree_sitter_typescript::LANGUAGE_TSX.into(),
                format!(
                    "{}\n{}\n{}",
                    tree_sitter_javascript::HIGHLIGHT_QUERY,
                    tree_sitter_javascript::JSX_HIGHLIGHT_QUERY,
                    tree_sitter_typescript::HIGHLIGHTS_QUERY
                ),
            ),
            Self::Python => (
                tree_sitter_python::LANGUAGE.into(),
                tree_sitter_python::HIGHLIGHTS_QUERY.into(),
            ),
            Self::Toml => (
                tree_sitter_toml_ng::LANGUAGE.into(),
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY.into(),
            ),
            Self::Css => (
                tree_sitter_css::LANGUAGE.into(),
                tree_sitter_css::HIGHLIGHTS_QUERY.into(),
            ),
            Self::Html => (
                tree_sitter_html::LANGUAGE.into(),
                tree_sitter_html::HIGHLIGHTS_QUERY.into(),
            ),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighlightKind {
    Comment,
    String,
    Keyword,
    Number,
    Function,
    Type,
    Property,
    Operator,
    Variable,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Highlight {
    pub range: Range<usize>,
    pub kind: HighlightKind,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Function,
    Type,
    Module,
    Variable,
    Property,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub range: Range<usize>,
    pub selection: Range<usize>,
}
/// Physical rows [start,end), with the first row kept visible by the consumer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fold {
    pub rows: Range<usize>,
}
#[derive(Clone, Debug)]
pub struct AnalysisSnapshot {
    pub document_id: u64,
    pub revision: u64,
    pub language: Language,
    pub highlights: Arc<[Highlight]>,
    pub symbols: Arc<[Symbol]>,
    pub folds: Arc<[Fold]>,
    pub has_errors: bool,
    /// Budget exhausted or giant input; never implies an error-free complete parse.
    pub truncated: bool,
}
impl AnalysisSnapshot {
    fn empty(snapshot: &BufferSnapshot, language: Language, truncated: bool) -> Self {
        Self {
            document_id: snapshot.document_id(),
            revision: snapshot.revision(),
            language,
            highlights: Arc::from([]),
            symbols: Arc::from([]),
            folds: Arc::from([]),
            has_errors: false,
            truncated,
        }
    }
}

#[derive(Clone)]
pub struct Cancellation {
    generation: Arc<AtomicU64>,
    expected: u64,
}
impl Default for Cancellation {
    fn default() -> Self {
        Self {
            generation: Arc::new(AtomicU64::new(0)),
            expected: 0,
        }
    }
}
impl Cancellation {
    pub fn for_generation(generation: Arc<AtomicU64>, expected: u64) -> Self {
        Self {
            generation,
            expected,
        }
    }
    pub fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.generation.load(Ordering::Acquire) != self.expected
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_bytes: usize,
    pub max_line_bytes: usize,
    pub max_spans: usize,
    pub max_nodes: usize,
    pub max_folds: usize,
    pub timeout: Duration,
}
impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_line_bytes: 256 * 1024,
            max_spans: 262_144,
            max_nodes: 524_288,
            max_folds: 16_384,
            timeout: Duration::from_millis(250),
        }
    }
}
struct CachedTree {
    document_id: u64,
    language: Language,
    text: String,
    tree: Tree,
}
struct Grammar {
    language: Language,
    parser: Parser,
    query: Query,
}
/// A worker owns this object. Four previous trees at most; no background thread per document.
pub struct ParserEngine {
    grammars: Vec<Grammar>,
    trees: VecDeque<CachedTree>,
    limits: ParseLimits,
}
impl Default for ParserEngine {
    fn default() -> Self {
        Self::new(ParseLimits::default())
    }
}
impl ParserEngine {
    pub fn new(limits: ParseLimits) -> Self {
        Self {
            grammars: Vec::new(),
            trees: VecDeque::new(),
            limits,
        }
    }
    pub fn analyze(
        &mut self,
        snapshot: &BufferSnapshot,
        language: Language,
        cancellation: &Cancellation,
    ) -> Result<AnalysisSnapshot, String> {
        let mut result = AnalysisSnapshot::empty(snapshot, language, false);
        if language == Language::Plain {
            return Ok(result);
        }
        if snapshot.byte_count() > self.limits.max_bytes || cancellation.is_cancelled() {
            result.truncated = true;
            return Ok(result);
        }
        // Inspect borrowed chunks, never allocate a giant line or a Vec<char> for it.
        let mut line_bytes = 0;
        for chunk in snapshot.chunks() {
            for byte in chunk.bytes() {
                if byte == b'\n' || byte == b'\r' {
                    line_bytes = 0;
                } else {
                    line_bytes += 1;
                }
                if line_bytes > self.limits.max_line_bytes {
                    result.truncated = true;
                    return Ok(result);
                }
            }
            if cancellation.is_cancelled() {
                result.truncated = true;
                return Ok(result);
            }
        }
        let text: String = snapshot.chunks().collect();
        if language == Language::Markdown {
            return Ok(markdown::analyze(self, snapshot, &text, cancellation));
        }
        if !self
            .grammars
            .iter()
            .any(|grammar| grammar.language == language)
        {
            let (grammar, query) = language.grammar().ok_or("missing grammar")?;
            let mut parser = Parser::new();
            parser
                .set_language(&grammar)
                .map_err(|error| error.to_string())?;
            let query = Query::new(&grammar, &query).map_err(|error| error.to_string())?;
            self.grammars.push(Grammar {
                language,
                parser,
                query,
            });
        }
        let mut previous = self
            .trees
            .iter()
            .position(|tree| {
                tree.document_id == snapshot.document_id() && tree.language == language
            })
            .and_then(|index| self.trees.remove(index));
        if let Some(previous) = &mut previous {
            edit_tree(&mut previous.tree, &previous.text, &text);
        }
        let grammar = self
            .grammars
            .iter_mut()
            .find(|grammar| grammar.language == language)
            .unwrap();
        let deadline = Instant::now() + self.limits.timeout;
        let mut cancelled =
            |_: &tree_sitter::ParseState| cancellation.is_cancelled() || Instant::now() >= deadline;
        let mut input = |offset: usize, _: Point| text.as_bytes().get(offset..).unwrap_or(&[]);
        let tree = grammar.parser.parse_with_options(
            &mut input,
            previous.as_ref().map(|previous| &previous.tree),
            Some(tree_sitter::ParseOptions::new().progress_callback(&mut cancelled)),
        );
        let Some(tree) = tree else {
            grammar.parser.reset();
            result.truncated = true;
            return Ok(result);
        };
        result.has_errors = tree.root_node().has_error();
        let mut cursor = QueryCursor::new();
        cursor.set_match_limit(4096);
        let mut query_cancelled = |_: &tree_sitter::QueryCursorState| {
            cancellation.is_cancelled() || Instant::now() >= deadline
        };
        let mut captures = cursor.captures_with_options(
            &grammar.query,
            tree.root_node(),
            text.as_bytes(),
            tree_sitter::QueryCursorOptions::new().progress_callback(&mut query_cancelled),
        );
        let mut highlights = Vec::new();
        while let Some((matched, index)) = captures.next() {
            let capture = matched.captures[*index];
            if let Some(kind) = capture_kind(grammar.query.capture_names()[capture.index as usize])
            {
                if let Some(range) = char_range(snapshot, capture.node.byte_range()) {
                    highlights.push(Highlight { range, kind });
                }
            }
            if highlights.len() >= self.limits.max_spans
                || cancellation.is_cancelled()
                || Instant::now() >= deadline
            {
                result.truncated = true;
                break;
            }
        }
        drop(captures);
        result.truncated |= cursor.did_exceed_match_limit()
            || cancellation.is_cancelled()
            || Instant::now() >= deadline;
        // Prefer the most specific capture at a shared offset; rendering resolves overlaps by
        // first containing range. No scanning parser subtrees on the UI thread.
        highlights.sort_by_key(|span| (span.range.start, span.range.len()));
        highlights.dedup();
        result.highlights = highlights.into();
        let mut symbols = Vec::new();
        let mut folds = Vec::new();
        let mut walk = tree.walk();
        let mut nodes = 0;
        loop {
            let node = walk.node();
            nodes += 1;
            if nodes > self.limits.max_nodes
                || cancellation.is_cancelled()
                || Instant::now() >= deadline
            {
                result.truncated = true;
                break;
            }
            if node.is_named() {
                if let Some(kind) = symbol_kind(node.kind()) {
                    let name = node
                        .child_by_field_name("name")
                        .or_else(|| node.child_by_field_name("key"))
                        .or_else(|| {
                            if matches!(
                                node.kind(),
                                "table" | "table_array_element" | "rule_set" | "element"
                            ) {
                                node.named_child(0)
                            } else {
                                None
                            }
                        });
                    if let Some(name) = name {
                        if name.end_byte().saturating_sub(name.start_byte()) <= 1024 {
                            if let (Some(range), Some(selection), Ok(label)) = (
                                char_range(snapshot, node.byte_range()),
                                char_range(snapshot, name.byte_range()),
                                name.utf8_text(text.as_bytes()),
                            ) {
                                if symbols.len() < 4096 {
                                    symbols.push(Symbol {
                                        name: label.into(),
                                        kind,
                                        range,
                                        selection,
                                    });
                                } else {
                                    result.truncated = true;
                                }
                            }
                        }
                    }
                }
                if fold_kind(node.kind()) {
                    // Tree-sitter rows count LF only; editor rows also represent CR and Unicode
                    // separators. Map node byte coordinates through the actual buffer snapshot.
                    let start = snapshot
                        .byte_to_char(node.start_byte())
                        .ok()
                        .and_then(|offset| snapshot.position_of(offset).ok())
                        .map_or(0, |position| position.row);
                    let end = snapshot
                        .byte_to_char(node.end_byte())
                        .ok()
                        .and_then(|offset| snapshot.position_of(offset).ok())
                        .map_or(start, |position| position.row);
                    if end > start + 1 {
                        if folds.len() < self.limits.max_folds {
                            folds.push(Fold {
                                rows: start..end + 1,
                            });
                        } else {
                            result.truncated = true;
                        }
                    }
                }
            }
            if walk.goto_first_child() {
                continue;
            }
            loop {
                if walk.goto_next_sibling() {
                    break;
                }
                if !walk.goto_parent() {
                    drop(walk);
                    result.symbols = symbols.into();
                    folds.sort_by_key(|fold| (fold.rows.start, fold.rows.end));
                    folds.dedup();
                    result.folds = folds.into();
                    self.trees.push_front(CachedTree {
                        document_id: snapshot.document_id(),
                        language,
                        text,
                        tree,
                    });
                    self.trees.truncate(4);
                    return Ok(result);
                }
            }
        }
        result.symbols = symbols.into();
        result.folds = folds.into();
        Ok(result)
    }
}
fn char_range(snapshot: &BufferSnapshot, bytes: Range<usize>) -> Option<Range<usize>> {
    Some(snapshot.byte_to_char(bytes.start).ok()?..snapshot.byte_to_char(bytes.end).ok()?)
}
fn capture_kind(name: &str) -> Option<HighlightKind> {
    let family = name.split('.').next()?;
    Some(match family {
        "comment" => HighlightKind::Comment,
        "string" | "character" | "escape" => HighlightKind::String,
        "keyword" | "boolean" | "constant" => HighlightKind::Keyword,
        "number" | "float" => HighlightKind::Number,
        "function" | "constructor" => HighlightKind::Function,
        "type" | "tag" => HighlightKind::Type,
        "property" | "attribute" => HighlightKind::Property,
        "operator" | "punctuation" => HighlightKind::Operator,
        "variable" => HighlightKind::Variable,
        _ => return None,
    })
}
fn symbol_kind(kind: &str) -> Option<SymbolKind> {
    Some(match kind {
        "function_item"
        | "function_declaration"
        | "function_definition"
        | "method_definition"
        | "function_signature_item" => SymbolKind::Function,
        "struct_item"
        | "enum_item"
        | "trait_item"
        | "class_declaration"
        | "class_definition"
        | "interface_declaration"
        | "type_alias_declaration"
        | "type_item" => SymbolKind::Type,
        "mod_item" | "internal_module" | "table" | "table_array_element" => SymbolKind::Module,
        "const_item" | "static_item" | "variable_declarator" => SymbolKind::Variable,
        "pair" | "declaration" | "rule_set" | "element" => SymbolKind::Property,
        _ => return None,
    })
}
fn fold_kind(kind: &str) -> bool {
    matches!(
        kind,
        "interface_declaration"
            | "object_type"
            | "function_definition"
            | "class_definition"
            | "function_declaration"
            | "class_declaration"
            | "function_item"
            | "block"
            | "statement_block"
            | "declaration_list"
            | "field_declaration_list"
            | "enum_variant_list"
            | "class_body"
            | "object"
            | "array"
            | "table"
            | "table_array_element"
            | "rule_set"
            | "element"
            | "comment"
            | "block_comment"
    )
}
fn point_at(text: &str, byte: usize) -> Point {
    let prefix = &text.as_bytes()[..byte];
    Point {
        row: prefix.iter().filter(|byte| **byte == b'\n').count(),
        column: prefix
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(byte, |index| byte - index - 1),
    }
}
fn edit_tree(tree: &mut Tree, previous: &str, next: &str) {
    if previous == next {
        return;
    }
    let mut start = previous
        .bytes()
        .zip(next.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !previous.is_char_boundary(start) || !next.is_char_boundary(start) {
        start -= 1;
    }
    let mut suffix = previous.as_bytes()[start..]
        .iter()
        .rev()
        .zip(next.as_bytes()[start..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    while !previous.is_char_boundary(previous.len() - suffix)
        || !next.is_char_boundary(next.len() - suffix)
    {
        suffix -= 1;
    }
    let old_end = previous.len() - suffix;
    let new_end = next.len() - suffix;
    tree.edit(&InputEdit {
        start_byte: start,
        old_end_byte: old_end,
        new_end_byte: new_end,
        start_position: point_at(previous, start),
        old_end_position: point_at(previous, old_end),
        new_end_position: point_at(next, new_end),
    });
}

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;
