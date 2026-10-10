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

#[test]
#[ignore = "prints timings; run with --profile desktop -- --ignored --nocapture"]
fn measure_analysis_time_by_document_size() {
    for functions in [40usize, 700, 7_000, 20_000] {
        let source: String = (0..functions)
            .map(|index| {
                format!(
                    "/// Item {index}.\nfn item_{index}(input: &str) -> usize {{\n    let total = input.len() + {index};\n    if total > 10 {{\n        return total * 2;\n    }}\n    total\n}}\n\n"
                )
            })
            .collect();
        let mut buffer = lienzo_editor::TextBuffer::new(&source).unwrap();
        let cancel = Cancellation::default();
        let report = |engine: &mut ParserEngine, snapshot: &BufferSnapshot| {
            let started = Instant::now();
            let analysis = engine.analyze(snapshot, Language::Rust, &cancel).unwrap();
            format!(
                "{:.2?} ({} spans, {} folds{})",
                started.elapsed(),
                analysis.highlights.len(),
                analysis.folds.len(),
                if analysis.truncated { ", truncated" } else { "" }
            )
        };
        let mut shipped = ParserEngine::default();
        let mut patient = ParserEngine::new(ParseLimits {
            timeout: Duration::from_secs(60),
            max_spans: usize::MAX,
            max_nodes: usize::MAX,
            max_folds: usize::MAX,
            ..ParseLimits::default()
        });
        let opened = buffer.snapshot();
        let first = report(&mut shipped, &opened);
        let unbounded = report(&mut patient, &opened);
        let at = opened.line_range(opened.line_count() / 2).unwrap().start;
        buffer
            .edit(
                at..at,
                "x",
                lienzo_editor::Selection::caret(at + 1),
                lienzo_editor::EditKind::Typing,
                0,
            )
            .unwrap();
        let typed = buffer.snapshot();
        println!(
            "{:>7} lines | open {first} | unbounded {unbounded} | after a key {} | unbounded {}",
            opened.line_count(),
            report(&mut shipped, &typed),
            report(&mut patient, &typed),
        );
    }
}

#[test]
fn a_cut_analysis_keeps_its_tree_for_the_next_attempt() {
    let source: String = (0..4_000)
        .map(|index| format!("fn item_{index}() {{\n    let value = {index};\n}}\n"))
        .collect();
    let mut buffer = TextBuffer::new(&source).unwrap();
    let cancel = Cancellation::default();

    // Stopped while collecting structure: the finished tree is still kept.
    let mut engine = ParserEngine::new(ParseLimits {
        max_nodes: 50,
        ..ParseLimits::default()
    });
    let cut = engine
        .analyze(&buffer.snapshot(), Language::Rust, &cancel)
        .unwrap();
    assert!(cut.truncated && !cut.highlights.is_empty());
    assert_eq!(engine.trees.len(), 1);

    // Stopped while parsing: the earlier tree follows the new text.
    let mut engine = ParserEngine::default();
    let whole = engine
        .analyze(&buffer.snapshot(), Language::Rust, &cancel)
        .unwrap();
    assert!(!whole.truncated);
    assert_eq!(whole.folds.len(), 4_000);
    buffer
        .edit(0..0, "// first\n", Selection::caret(9), EditKind::Typing, 0)
        .unwrap();
    let edited = buffer.snapshot();
    engine.limits.timeout = Duration::ZERO;
    let stopped = engine.analyze(&edited, Language::Rust, &cancel).unwrap();
    assert!(stopped.truncated && stopped.highlights.is_empty());
    assert_eq!(engine.trees.len(), 1);
    assert!(engine.trees[0].text.starts_with("// first\n"));

    engine.limits.timeout = ParseLimits::default().timeout;
    let resumed = engine.analyze(&edited, Language::Rust, &cancel).unwrap();
    let fresh = ParserEngine::default()
        .analyze(&edited, Language::Rust, &cancel)
        .unwrap();
    assert!(!resumed.truncated);
    assert_eq!(resumed.highlights, fresh.highlights);
    assert_eq!(resumed.folds, fresh.folds);
}

#[test]
fn highlights_reused_across_edits_equal_a_full_query_of_the_same_tree() {
    let sources = [
        (
            Language::Rust,
            "/// Doc ñ.\nfn item(input: &str) -> usize {\n    let total = input.len() + 1; // nota\n    if total > 10 {\n        return call(total, \"text\");\n    }\n    CONSTANT + total\n}\n\nstruct Thing<T> {\n    field: Vec<T>,\n}\n\n",
        ),
        (
            Language::TypeScript,
            "interface Thing {\n  name: string;\n}\nexport function greet(x: Thing): string {\n  const label = `hola ${x.name}`; /* nota */\n  return require('fs').readFileSync(label) + 1;\n}\n\n",
        ),
        (
            Language::Python,
            "class Thing:\n    \"\"\"Doc.\"\"\"\n    def run(self, value=1):\n        total = len(self.items) + value  # nota\n        return f\"{total}\"\n\n",
        ),
        (
            Language::Json,
            "{\n  \"name\": \"ñ\",\n  \"items\": [1, 2.5, true, null],\n  \"nested\": {\"a\": \"b\"}\n}\n",
        ),
        (
            Language::Css,
            "main > .item:hover {\n  color: #ff0000;\n  width: calc(100% - 12px); /* nota */\n}\n\n",
        ),
    ];
    let pieces = [
        "", "x", " ", "\n", "\"", "'", "{", "}", "(", ")", "/*", "*/", "//", "#", "fn f() {}\n",
        "let a = 1;\n", "ñ", "`", ": 1,", "\n\n",
    ];
    let mut seed = 0x2545_f491_u64;
    let mut random = |bound: usize| {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 33) as usize % bound.max(1)
    };
    let cancel = Cancellation::default();
    for (language, unit) in sources {
        let mut buffer = TextBuffer::new(&unit.repeat(12)).unwrap();
        let mut engine = ParserEngine::default();
        engine.analyze(&buffer.snapshot(), language, &cancel).unwrap();
        // Parses the same edits incrementally but always queries the whole tree.
        let mut whole = ParserEngine::default();
        whole.analyze(&buffer.snapshot(), language, &cancel).unwrap();
        let mut reused = 0;
        for step in 0..120 {
            let length = buffer.snapshot().char_count();
            let start = random(length + 1);
            let end = (start + random(3) * random(12)).min(length);
            let piece = pieces[random(pieces.len())];
            let caret = Selection::caret(start + piece.chars().count());
            buffer.edit(start..end, piece, caret, EditKind::Typing, 0).unwrap();
            let snapshot = buffer.snapshot();
            reused += usize::from(engine.trees.front().is_some_and(|tree| tree.highlights.is_some()));
            let incremental = engine.analyze(&snapshot, language, &cancel).unwrap();
            whole.trees[0].highlights = None;
            let fresh = whole.analyze(&snapshot, language, &cancel).unwrap();
            let differs = incremental
                .highlights
                .iter()
                .zip(fresh.highlights.iter())
                .position(|(left, right)| left != right)
                .or_else(|| {
                    (incremental.highlights.len() != fresh.highlights.len())
                        .then(|| incremental.highlights.len().min(fresh.highlights.len()))
                });
            if let Some(index) = differs {
                let around = |spans: &[Highlight]| {
                    spans[index.saturating_sub(2)..(index + 3).min(spans.len())].to_vec()
                };
                let at = fresh.highlights.get(index).map_or(0, |span| span.range.start);
                panic!(
                    "{language:?} step {step} edit {start}..{end} {piece:?}: span {index}\n reused {:?}\n fresh  {:?}\n text {:?}",
                    around(&incremental.highlights),
                    around(&fresh.highlights),
                    snapshot
                        .text_range(at.saturating_sub(40)..(at + 40).min(snapshot.char_count()))
                        .unwrap()
                );
            }
            assert_eq!(incremental.folds, fresh.folds, "{language:?} step {step}");
        }
        assert!(reused > 100, "{language:?} reused {reused}");
    }
}
