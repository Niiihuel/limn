use super::*;
use lienzo_editor::{EditKind, Selection};

fn analyzed(source: &str) -> (BufferSnapshot, AnalysisSnapshot) {
    let snapshot = TextBuffer::new(source).unwrap().snapshot();
    let analysis = ParserEngine::default()
        .analyze(&snapshot, Language::Markdown, &Cancellation::default())
        .unwrap();
    assert!(!analysis.truncated);
    assert!(!analysis.has_errors);
    assert!(analysis
        .highlights
        .windows(2)
        .all(|pair| pair[0].range.end <= pair[1].range.start));
    assert!(analysis
        .highlights
        .iter()
        .all(|span| span.range.end <= snapshot.char_count()));
    (snapshot, analysis)
}

fn kind_at(source: &str, analysis: &AnalysisSnapshot, needle: &str) -> Option<HighlightKind> {
    let byte = source.find(needle).unwrap();
    let character = source[..byte].chars().count();
    analysis
        .highlights
        .iter()
        .find(|span| span.range.contains(&character))
        .map(|span| span.kind)
}

#[test]
fn markdown_extensions_and_fenced_aliases_are_real_languages() {
    for extension in ["md", ".MD", "markdown", "mdown", "mkd", "mkdn", "mdwn"] {
        assert_eq!(Language::from_extension(extension), Language::Markdown);
    }
    assert_eq!(Language::Markdown.lsp_id(), "markdown");
    assert_eq!(Language::from_code_fence("rust,ignore"), Language::Rust);
    assert_eq!(Language::from_code_fence("{.python}"), Language::Python);
    assert_eq!(
        Language::from_code_fence("javascript title=sample"),
        Language::JavaScript
    );
    assert_eq!(Language::from_code_fence("unknown"), Language::Plain);
}

#[test]
fn markdown_source_scopes_preserve_unicode_and_nested_inline_precedence() {
    let source = "# Café 🧠 y `código`\n\n> Cita **fuerte** y *énfasis*.\n\n- [x] tarea [guía](./guía.md) con ~~tachado~~\n\n![imagen](assets/niño.png)\n";
    let (snapshot, analysis) = analyzed(source);
    assert_eq!(
        kind_at(source, &analysis, "Café"),
        Some(HighlightKind::Type)
    );
    assert_eq!(
        kind_at(source, &analysis, "código"),
        Some(HighlightKind::String)
    );
    assert_eq!(
        kind_at(source, &analysis, ">"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "fuerte"),
        Some(HighlightKind::Property)
    );
    assert_eq!(
        kind_at(source, &analysis, "énfasis"),
        Some(HighlightKind::Property)
    );
    assert_eq!(
        kind_at(source, &analysis, "[x]"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "guía]"),
        Some(HighlightKind::Function)
    );
    assert_eq!(
        kind_at(source, &analysis, "./guía.md"),
        Some(HighlightKind::String)
    );
    assert_eq!(
        kind_at(source, &analysis, "tachado"),
        Some(HighlightKind::Property)
    );
    assert_eq!(
        kind_at(source, &analysis, "assets/niño.png"),
        Some(HighlightKind::String)
    );
    assert_eq!(analysis.symbols[0].name, "Café 🧠 y código");
    assert_eq!(
        snapshot
            .text_range(analysis.symbols[0].selection.clone())
            .unwrap(),
        "Café 🧠 y `código`"
    );
}

#[test]
fn markdown_multiline_scopes_tables_comments_and_section_folds() {
    let source = "Título\n======\n\nTexto **en\ndos líneas**.\n\n| Nombre | Valor |\n| --- | --- |\n| uno | `dos` |\n\n<!-- comentario\nmultilínea -->\n\n## Segundo\nFinal.\n";
    let (snapshot, analysis) = analyzed(source);
    assert_eq!(
        analysis
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["Título", "Segundo"]
    );
    assert_eq!(
        kind_at(source, &analysis, "dos líneas"),
        Some(HighlightKind::Property)
    );
    assert_eq!(
        kind_at(source, &analysis, "| Nombre"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "`dos`"),
        Some(HighlightKind::String)
    );
    assert_eq!(
        kind_at(source, &analysis, "multilínea"),
        Some(HighlightKind::Comment)
    );
    assert!(analysis
        .folds
        .iter()
        .any(|fold| fold.rows == (0..snapshot.line_count())));
}

#[test]
fn markdown_fences_delegate_code_without_leaking_into_following_prose() {
    let source = "😀 antes\n\n```rust\nfn café() {\n    let texto = \"🧠\";\n}\n```\n\nDespués *normal*.\n\n~~~python\ndef niño():\n    return 42\n~~~\n";
    let (_, analysis) = analyzed(source);
    assert_eq!(
        kind_at(source, &analysis, "```rust"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "fn café"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "café()"),
        Some(HighlightKind::Function)
    );
    assert_eq!(
        kind_at(source, &analysis, "\"🧠\""),
        Some(HighlightKind::String)
    );
    assert_eq!(kind_at(source, &analysis, "Después"), None);
    assert_eq!(
        kind_at(source, &analysis, "normal"),
        Some(HighlightKind::Property)
    );
    assert_eq!(
        kind_at(source, &analysis, "return 42"),
        Some(HighlightKind::Keyword)
    );
    assert_eq!(
        kind_at(source, &analysis, "42"),
        Some(HighlightKind::Number)
    );
    assert!(analysis.folds.iter().any(|fold| fold.rows == (2..7)));
}

#[test]
fn markdown_unknown_and_nested_fences_keep_literal_ranges() {
    let source = "```unknown\n**literal**\n```\n\n> ```rust\n> let café = 1;\n> café\n> ```\n";
    let (_, analysis) = analyzed(source);
    assert_eq!(
        kind_at(source, &analysis, "**literal**"),
        Some(HighlightKind::String)
    );
    assert_eq!(
        kind_at(source, &analysis, "let café"),
        Some(HighlightKind::String)
    );
    assert!(analysis
        .highlights
        .iter()
        .all(|span| span.range.end <= source.chars().count()));
}

#[test]
fn markdown_edits_recompute_fences_and_versioned_outline_without_stale_ranges() {
    let mut parser = ParserEngine::default();
    let mut buffer = TextBuffer::new("# Viejo\n\n```rust\nlet café = 1;\n```\n").unwrap();
    let old = parser
        .analyze(
            &buffer.snapshot(),
            Language::Markdown,
            &Cancellation::default(),
        )
        .unwrap();
    let end = buffer.snapshot().char_count();
    let source = "😀 cambiado\n\n## Nuevo `🦀`\n\n**café**\n";
    buffer
        .edit(0..end, source, Selection::caret(0), EditKind::Replace, 0)
        .unwrap();
    let new = parser
        .analyze(
            &buffer.snapshot(),
            Language::Markdown,
            &Cancellation::default(),
        )
        .unwrap();
    assert_eq!(new.document_id, old.document_id);
    assert!(new.revision > old.revision);
    assert_eq!(new.symbols[0].name, "Nuevo 🦀");
    assert_eq!(kind_at(source, &new, "café"), Some(HighlightKind::Property));
    assert_eq!(kind_at(source, &new, "🦀"), Some(HighlightKind::String));
    assert!(new
        .highlights
        .iter()
        .all(|span| span.range.end <= buffer.snapshot().char_count()));
}

#[test]
fn markdown_analysis_honors_cancelled_and_small_parse_budgets() {
    let snapshot = TextBuffer::new("# Title\n\n**strong** `code` [link](target)\n")
        .unwrap()
        .snapshot();
    let mut parser = ParserEngine::new(ParseLimits {
        max_spans: 2,
        ..Default::default()
    });
    assert!(
        parser
            .analyze(&snapshot, Language::Markdown, &Cancellation::default())
            .unwrap()
            .truncated
    );
    let cancelled = Cancellation::default();
    cancelled.cancel();
    let result = parser
        .analyze(&snapshot, Language::Markdown, &cancelled)
        .unwrap();
    assert!(result.truncated);
    assert!(result.highlights.is_empty());
}

#[test]
fn markdown_reference_definitions_and_resolved_uses_share_link_scopes() {
    let source = "Una [guía][manual] y [manual].\n\n[manual]: ./guía.md \"Título\"\n";
    let (_, analysis) = analyzed(source);
    assert_eq!(
        kind_at(source, &analysis, "guía]"),
        Some(HighlightKind::Function)
    );
    assert_eq!(
        kind_at(source, &analysis, "[manual]:"),
        Some(HighlightKind::Function)
    );
    assert_eq!(
        kind_at(source, &analysis, "./guía.md"),
        Some(HighlightKind::String)
    );
}

#[test]
fn markdown_heading_symbols_cover_their_section_for_breadcrumb_navigation() {
    let source = "# Primero\ntexto\n\n## Hijo 🧠\ncuerpo\n\n# Segundo\nfinal\n";
    let (snapshot, analysis) = analyzed(source);
    let sibling = source[..source.find("# Segundo").unwrap()].chars().count();
    let body = source[..source.find("cuerpo").unwrap()].chars().count();
    assert_eq!(analysis.symbols[0].range.end, sibling);
    assert_eq!(analysis.symbols[1].range.end, sibling);
    assert!(analysis.symbols[0].range.contains(&body));
    assert!(analysis.symbols[1].range.contains(&body));
    assert!(!analysis.symbols[1].range.contains(&sibling));
    assert_eq!(analysis.symbols[2].range.end, snapshot.char_count());
}

#[test]
fn markdown_folds_share_one_budget_and_report_discarded_code_or_heading_ranges() {
    for source in [
        "```text\na\nb\n```\n\n```text\nc\nd\n```\n",
        "# One\na\nb\n\n# Two\nc\nd\n",
        "# One\n\n```text\na\nb\n```\n\n# Two\nc\nd\n",
    ] {
        let snapshot = TextBuffer::new(source).unwrap().snapshot();
        let mut parser = ParserEngine::new(ParseLimits {
            max_folds: 1,
            timeout: Duration::from_secs(2),
            ..Default::default()
        });
        let result = parser
            .analyze(&snapshot, Language::Markdown, &Cancellation::default())
            .unwrap();
        assert_eq!(result.folds.len(), 1, "{source}");
        assert!(
            result.truncated,
            "discarding a code or heading fold must be reported"
        );
        assert!(
            !result.highlights.is_empty(),
            "fold budget must not suppress syntax"
        );
    }
}

#[test]
fn native_code_grammars_obey_the_same_configurable_fold_budget() {
    let snapshot = TextBuffer::new("fn one() {\n let x = 1;\n}\nfn two() {\n let y = 2;\n}\n")
        .unwrap()
        .snapshot();
    let mut parser = ParserEngine::new(ParseLimits {
        max_folds: 1,
        ..Default::default()
    });
    let result = parser
        .analyze(&snapshot, Language::Rust, &Cancellation::default())
        .unwrap();
    assert_eq!(result.folds.len(), 1);
    assert!(result.truncated);
}
