use super::*;
use lienzo_editor::{EditKind, Selection, TextBuffer};

#[test]
fn actual_grammars_highlight_all_supported_languages_and_report_version() {
    let mut parser = ParserEngine::default();
    for (language, text) in [
        (Language::Rust, "fn café() {\n let value = \"😀\";\n}\n"),
        (
            Language::JavaScript,
            "function example() {\n return {x: 1};\n}\n",
        ),
        (
            Language::TypeScript,
            "interface Thing {\n name: string;\n}\nfunction greet(x: Thing) { return x.name; }",
        ),
        (
            Language::Python,
            "def café():\n    value = \"😀\"\n    return value\n",
        ),
        (Language::Json, "{\n \"name\": \"😀\",\n \"count\": 2\n}"),
        (
            Language::Toml,
            "[package]\nname = \"example\"\nversion = 1\n",
        ),
        (Language::Css, "main {\n color: red;\n width: 12px;\n}"),
        (Language::Html, "<main>\n <span>hello</span>\n</main>"),
    ] {
        let snapshot = TextBuffer::new(text).unwrap().snapshot();
        let result = parser
            .analyze(&snapshot, language, &Cancellation::default())
            .unwrap();
        assert_eq!(
            (result.document_id, result.revision),
            (snapshot.document_id(), snapshot.revision())
        );
        assert!(!result.highlights.is_empty(), "{language:?}");
        assert!(!result.truncated, "{language:?}");
        assert!(!result.has_errors, "{language:?}");
        assert!(result
            .highlights
            .iter()
            .all(|span| span.range.end <= snapshot.char_count()));
        assert!(
            !result.symbols.is_empty() || language == Language::Json,
            "symbols: {language:?}"
        );
        assert!(!result.folds.is_empty(), "folds: {language:?}");
    }
}

#[test]
fn incremental_tree_reuses_document_but_never_stale_unicode_coordinates() {
    let mut parser = ParserEngine::default();
    let mut buffer = TextBuffer::new("fn primero() {\n let s = \"😀\";\n}\n").unwrap();
    let first = parser
        .analyze(&buffer.snapshot(), Language::Rust, &Cancellation::default())
        .unwrap();
    assert_eq!(first.symbols[0].name, "primero");
    buffer
        .edit(3..10, "niño", Selection::caret(7), EditKind::Replace, 0)
        .unwrap();
    let snapshot = buffer.snapshot();
    let second = parser
        .analyze(&snapshot, Language::Rust, &Cancellation::default())
        .unwrap();
    assert!(second.revision > first.revision);
    assert_eq!(second.symbols[0].name, "niño");
    assert_eq!(
        snapshot
            .text_range(second.symbols[0].selection.clone())
            .unwrap(),
        "niño"
    );
    assert_eq!(parser.trees.len(), 1);
}

#[test]
fn giant_lines_cancellation_and_tree_retention_are_bounded() {
    let mut parser = ParserEngine::new(ParseLimits {
        max_line_bytes: 64,
        ..ParseLimits::default()
    });
    let giant = TextBuffer::new(&"x".repeat(1_000_000)).unwrap().snapshot();
    let result = parser
        .analyze(&giant, Language::Rust, &Cancellation::default())
        .unwrap();
    assert!(result.truncated && result.highlights.is_empty());
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let snapshot = TextBuffer::new("fn main() {}").unwrap().snapshot();
    assert!(
        parser
            .analyze(&snapshot, Language::Rust, &cancellation)
            .unwrap()
            .truncated
    );
    for _ in 0..12 {
        let snapshot = TextBuffer::new("fn main() {}").unwrap().snapshot();
        parser
            .analyze(&snapshot, Language::Rust, &Cancellation::default())
            .unwrap();
    }
    assert_eq!(parser.trees.len(), 4);
}

#[test]
fn typescript_angle_assertions_and_tsx_elements_use_their_own_grammars() {
    let mut parser = ParserEngine::default();
    for (extension, language, text) in [
        ("ts", Language::TypeScript, "const value = <number>source;"),
        (
            "tsx",
            Language::Tsx,
            "const view = <main className=\"app\"><span>Hi</span></main>;",
        ),
    ] {
        assert_eq!(Language::from_extension(extension), language);
        let snapshot = TextBuffer::new(text).unwrap().snapshot();
        let result = parser
            .analyze(&snapshot, language, &Cancellation::default())
            .unwrap();
        assert!(!result.has_errors, "{language:?}");
        assert!(!result.highlights.is_empty());
    }
    assert_eq!(Language::Tsx.lsp_id(), "typescriptreact");
}

#[test]
fn fold_rows_follow_the_buffer_for_cr_and_unicode_separators() {
    let mut parser = ParserEngine::default();
    for text in [
        "fn main() {\r let value=\"a\u{2028}b\u{85}c\";\r}",
        "fn main() {\r\n let value=1;\r\n}",
    ] {
        let snapshot = TextBuffer::new(text).unwrap().snapshot();
        let result = parser
            .analyze(&snapshot, Language::Rust, &Cancellation::default())
            .unwrap();
        let expected = snapshot.position_of(snapshot.char_count()).unwrap().row + 1;
        assert!(
            result.folds.iter().any(|fold| fold.rows == (0..expected)),
            "{:?} expected end {expected}",
            result.folds
        );
    }
}

#[test]
fn analysis_worker_is_last_wins_and_results_keep_their_owner() {
    let worker = crate::AnalysisWorker::new();
    let old = TextBuffer::new(&"fn old() {}\n".repeat(5000))
        .unwrap()
        .snapshot();
    assert!(worker.submit(old, Language::Rust));
    let current = TextBuffer::new("fn current() {}\n").unwrap().snapshot();
    assert!(worker.submit(current.clone(), Language::Rust));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = worker.try_recv() {
            let result = result.unwrap();
            assert_eq!(result.document_id, current.document_id());
            assert_eq!(result.symbols[0].name, "current");
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
}
