//! Bounded Unicode searches against immutable revisions; replacements commit once.
use crate::{BufferSnapshot, EditKind, EditorError, Selection, TextBuffer, TextEdit};
use regex::{Regex, RegexBuilder};
use std::{fmt, ops::Range};

const MATCH_LIMIT: usize = 4096;
/// How `search` and the replace methods read a query. The default is a case-insensitive
/// literal search of the whole document.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchOptions {
    /// Match letter case exactly. When false, the default, case is ignored.
    pub case_sensitive: bool,
    /// Reject a match that has a letter, digit or underscore directly before or after it.
    pub whole_word: bool,
    /// Read the query as a regular expression instead of literal text.
    pub regex: bool,
    /// Character offsets in the captured snapshot, not bytes.
    pub scope: Option<Range<usize>>,
}
/// The matches of one search and the snapshot revision they were found in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchResults {
    /// `document_id` of the snapshot that was searched.
    pub document_id: u64,
    /// Revision of that snapshot; the ranges are only valid for it.
    pub revision: u64,
    /// Character ranges in document order.
    pub matches: Vec<Range<usize>>,
    /// True when the search stopped at its limit of 4096 matches.
    pub truncated: bool,
}
/// Why a search or a replacement failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchError {
    /// The query did not compile as a regular expression, or compiles past the 2 MiB size
    /// limit. Carries the message of the `regex` crate.
    InvalidPattern(String),
    /// `scope` starts after its end or ends past the document.
    InvalidScope,
    /// A replacement was refused because the search found more than 4096 matches.
    TooManyMatches,
    /// The buffer refused an offset or the replacement edit.
    Edit(EditorError),
}
impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPattern(e) => write!(f, "Invalid search pattern: {e}"),
            Self::InvalidScope => write!(f, "Search selection is outside this document"),
            Self::TooManyMatches => write!(
                f,
                "Replacement exceeds the limit of {MATCH_LIMIT} matches; narrow the search"
            ),
            Self::Edit(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for SearchError {}
impl From<EditorError> for SearchError {
    fn from(e: EditorError) -> Self {
        Self::Edit(e)
    }
}
fn pattern(query: &str, options: &SearchOptions) -> Result<Regex, SearchError> {
    let pattern = if options.regex {
        query.to_owned()
    } else {
        regex::escape(query)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!options.case_sensitive)
        .multi_line(true)
        .size_limit(2 * 1024 * 1024)
        .dfa_size_limit(2 * 1024 * 1024)
        .build()
        .map_err(|e| SearchError::InvalidPattern(e.to_string()))
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || unicode_width::UnicodeWidthChar::width(c) == Some(0)
}
fn allowed(text: &str, start: usize, end: usize, options: &SearchOptions) -> bool {
    !options.whole_word
        || (!text[..start].chars().next_back().is_some_and(word)
            && !text[end..].chars().next().is_some_and(word))
}
impl BufferSnapshot {
    /// Find `query` in this snapshot. An empty query finds nothing, at most 4096 matches are
    /// returned (`truncated` reports the cut) and a match that would split a grapheme is dropped.
    pub fn search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<SearchResults, SearchError> {
        let mut result = SearchResults {
            document_id: self.document_id(),
            revision: self.revision(),
            matches: Vec::new(),
            truncated: false,
        };
        let scope = options.scope.clone().unwrap_or(0..self.char_count());
        if scope.start > scope.end || scope.end > self.char_count() {
            return Err(SearchError::InvalidScope);
        }
        if query.is_empty() {
            return Ok(result);
        }
        let regex = pattern(query, options)?;
        // Match the complete text so ^/$/word boundaries retain document semantics
        // even when a selection constrains which matches may be returned.
        let text = self.text_range(0..self.char_count())?;
        let first = self.char_to_byte(scope.start)?;
        let last = self.char_to_byte(scope.end)?;
        for found in regex.find_iter(&text) {
            if found.start() < first
                || found.end() > last
                || !allowed(&text, found.start(), found.end(), options)
            {
                continue;
            }
            if result.matches.len() == MATCH_LIMIT {
                result.truncated = true;
                break;
            }
            let range = self.byte_to_char(found.start())?..self.byte_to_char(found.end())?;
            if self.is_grapheme_boundary(range.start)? && self.is_grapheme_boundary(range.end)? {
                result.matches.push(range);
            }
        }
        Ok(result)
    }
}
impl TextBuffer {
    /// Replace every match in one undo transaction and return how many were replaced. In regex
    /// mode `replacement` may refer to capture groups. Fails with `TooManyMatches` rather than
    /// replace only some of them.
    pub fn replace_all(
        &mut self,
        query: &str,
        replacement: &str,
        options: &SearchOptions,
        timestamp: u64,
    ) -> Result<usize, SearchError> {
        self.replace_search(query, replacement, options, None, timestamp)
    }
    /// Replace a selected search match; only an exact current match is eligible.
    pub fn replace_match(
        &mut self,
        query: &str,
        replacement: &str,
        options: &SearchOptions,
        range: Range<usize>,
        timestamp: u64,
    ) -> Result<usize, SearchError> {
        self.replace_search(query, replacement, options, Some(range), timestamp)
    }
    fn replace_search(
        &mut self,
        query: &str,
        replacement: &str,
        options: &SearchOptions,
        only: Option<Range<usize>>,
        timestamp: u64,
    ) -> Result<usize, SearchError> {
        let snapshot = self.snapshot();
        let results = snapshot.search(query, options)?;
        if results.truncated {
            return Err(SearchError::TooManyMatches);
        }
        if results.matches.is_empty() {
            return Ok(0);
        }
        let regex = pattern(query, options)?;
        let text = snapshot.text_range(0..snapshot.char_count())?;
        let mut edits = Vec::new();
        for captures in regex.captures_iter(&text) {
            let found = captures.get(0).expect("full capture");
            let range =
                snapshot.byte_to_char(found.start())?..snapshot.byte_to_char(found.end())?;
            if only.as_ref().is_some_and(|only| *only != range)
                || results
                    .matches
                    .binary_search_by_key(&range.start, |r| r.start)
                    .is_err()
            {
                continue;
            }
            let mut value = String::new();
            if options.regex {
                captures.expand(replacement, &mut value);
            } else {
                value.push_str(replacement);
            }
            edits.push(TextEdit {
                range,
                replacement: value,
            });
        }
        let count = edits.len();
        if let Some(last) = edits.last() {
            let delta: isize = edits[..count - 1]
                .iter()
                .map(|e| e.replacement.chars().count() as isize - e.range.len() as isize)
                .sum();
            let end =
                last.range.start.saturating_add_signed(delta) + last.replacement.chars().count();
            self.apply_transaction(&edits, Selection::caret(end), EditKind::Replace, timestamp)?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_regex_scope_case_words_and_atomic_replacement() {
        let mut b = TextBuffer::new("café Café cafeteria\r\ncafé 東京42").unwrap();
        let o = SearchOptions {
            whole_word: true,
            ..Default::default()
        };
        assert_eq!(b.snapshot().search("café", &o).unwrap().matches.len(), 3);
        let scope = SearchOptions {
            scope: Some(20..b.snapshot().char_count()),
            ..o.clone()
        };
        assert_eq!(
            b.snapshot().search("café", &scope).unwrap().matches.len(),
            1
        );
        let o = SearchOptions {
            regex: true,
            ..Default::default()
        };
        b.replace_all(r"(café)|(東京)(\d+)", "$2${3}X", &o, 1)
            .unwrap();
        assert_eq!(b.snapshot().line_text(1).unwrap(), "X 東京42X");
        assert_eq!(b.history_transaction_count(), 1);
        b.undo().unwrap();
        assert_eq!(b.snapshot().line_text(1).unwrap(), "café 東京42");
        assert!(b.snapshot().search("(", &o).is_err());
    }
    #[test]
    fn zero_width_matches_terminate_and_large_replacement_is_rejected_whole() {
        let mut b = TextBuffer::new("é\r\n東京").unwrap();
        let o = SearchOptions {
            regex: true,
            ..Default::default()
        };
        assert_eq!(b.replace_all("(?m)^", ">", &o, 0).unwrap(), 2);
        b.undo().unwrap();
        assert_eq!(b.snapshot().line_text(0).unwrap(), "é");
        let mut many = TextBuffer::new(&"a ".repeat(4097)).unwrap();
        assert!(
            many.snapshot()
                .search("a", &SearchOptions::default())
                .unwrap()
                .truncated
        );
        assert_eq!(
            many.replace_all("a", "b", &SearchOptions::default(), 0),
            Err(SearchError::TooManyMatches)
        );
        assert_eq!(many.revision(), 0);
    }
}
