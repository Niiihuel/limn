use limn::{CodeBlock, CodeDiff, CodeDiffLine, CodeDiffLineKind, CodeEditor, Component,
    ComponentPalette, DrawCommand, EditorController, Element, FontFamily, Markdown,
    CodeReviewHunk, Point, Size, TextSpan, UiEvent, UiTree};
use limn_editor::TextBuffer;

fn tree(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_text_measurer(|text, size, _| text.chars().count() as f32 * size / 2.0);
    tree.set_text_height_measurer(|size, _| size);
    tree
}

fn text(frame: &limn::Frame, expected: &str) -> (Point, f32) {
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { position, text, font_size, .. } if text == expected =>
            Some((*position, *font_size)),
        _ => None,
    }).unwrap()
}

#[test]
fn code_font_size_controls_paint_row_height_and_pointer_selection_per_editor() {
    let controller = EditorController::new(TextBuffer::new("abcde\nfghij\nklmno").unwrap());
    let root = |size| CodeEditor::new("code", controller.clone()).font_size(size).render();
    let mut tree = tree(root(26.0));
    let frame = tree.layout(Size::new(400.0, 320.0));
    let (first, size) = text(&frame, "abcde");
    let (second, _) = text(&frame, "fghij");
    assert_eq!(size, 26.0);
    assert_eq!(second.y - first.y, 40.0);
    tree.handle_event(UiEvent::PointerDown(Point::new(second.x + 2.1 * 13.0, second.y + 2.0)));
    let snapshot = controller.snapshot().unwrap();
    let position = snapshot.position_of(snapshot.selection().head).unwrap();
    assert_eq!((position.row, position.column), (1, 2));
    tree.set_root(root(13.0));
    let small = tree.layout(Size::new(400.0, 320.0));
    assert_eq!(text(&small, "fghij").0.y - text(&small, "abcde").0.y, 20.0);
    assert_eq!(text(&small, "abcde").1, 13.0);
}

#[test]
fn diff_size_and_background_switch_keep_change_markers() {
    let lines = vec![CodeDiffLine { kind: CodeDiffLineKind::Added, old_line: None,
        new_line: Some(1), text: "abcde".into() },
        CodeDiffLine { kind: CodeDiffLineKind::Removed, old_line: Some(2),
        new_line: None, text: "fghij".into() }];
    let mut tree = tree(CodeDiff::new("diff", lines).font_size(26.0).diff_backgrounds(false).render());
    let frame = tree.layout(Size::new(400.0, 320.0));
    assert_eq!(text(&frame, "abcde").1, 26.0);
    assert_eq!(text(&frame, "fghij").0.y - text(&frame, "abcde").0.y, 40.0);
    assert_eq!(text(&frame, "+").1, 26.0);
    assert!(!frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect {rect, color, ..} if rect.width > 2.0 && (color.a == 20 || color.a == 28))));
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect {rect, ..} if rect.width == 2.0 && rect.height == 40.0)));
}

#[test]
fn markdown_code_size_and_wrapping_are_independent_of_body_and_cache() {
    let palette = ComponentPalette::neutral_dark();
    let source = "Body\n\n```rust\nlet a_long_variable = 123456789;\n```";
    let root = |wrap| Markdown::new(source, palette).width(150.0)
        .font_size(20.0).code_font_size(24.0).code_word_wrap(wrap).render().element;
    let mut tree = tree(root(true));
    let wrapped = tree.layout(Size::new(150.0, 600.0));
    assert_eq!(text(&wrapped, "Body").1, 20.0);
    // Highlighted code is a run per colored span; rows tell wrapping apart.
    let rows = |frame: &limn::Frame| {
        let code = frame
            .text_layouts()
            .into_iter()
            .find(|layout| layout.source.contains("a_long_variable"))
            .unwrap();
        code.runs
            .iter()
            .map(|run| run.rect.y.to_bits())
            .collect::<std::collections::HashSet<_>>()
            .len()
    };
    assert!(rows(&wrapped) > 1);
    assert!(wrapped.commands.iter().any(|command| matches!(command,
        DrawCommand::Text {font_size, font_family, text, ..}
        if *font_size == 24.0 && *font_family == FontFamily::Monospace && text.contains("let"))));
    tree.set_root(root(false));
    let unwrapped = tree.layout(Size::new(150.0, 600.0));
    assert_eq!(rows(&unwrapped), 1);
    assert_eq!(text(&unwrapped, "Body").1, 20.0);
}

#[test]
fn code_block_custom_size_keeps_explicit_span_colors() {
    let palette = ComponentPalette::neutral_dark();
    let mut tree = tree(CodeBlock::new("plain", palette)
        .spans(vec![TextSpan::new("abcde", palette.muted, 12.0, FontFamily::Monospace)])
        .font_size(22.0).render());
    let frame = tree.layout(Size::new(400.0, 320.0));
    assert_eq!(text(&frame, "abcde").1, 22.0);
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text {text, color, ..} if text == "abcde" && *color == palette.muted)));
}

#[test]
fn font_sizes_reject_invalid_numbers_and_bound_finite_values() {
    let controller = EditorController::new(TextBuffer::new("code").unwrap());
    for invalid in [f32::NAN, f32::INFINITY, 0.0, -2.0] {
        assert_eq!(CodeEditor::new("editor", controller.clone()).font_size(invalid).font_size, 13.0);
        assert_eq!(CodeDiff::new("diff", Vec::<CodeDiffLine>::new()).font_size(invalid).font_size, 13.0);
    }
    assert_eq!(CodeEditor::new("editor", controller.clone()).font_size(0.5).font_size, 1.0);
    assert_eq!(CodeEditor::new("editor", controller).font_size(1000.0).font_size, 96.0);
}

#[test]
fn review_removed_zones_and_current_rows_share_configured_geometry() {
    let controller = EditorController::new(TextBuffer::new("kept\nnew\nend").unwrap());
    let snapshot = controller.snapshot().unwrap();
    let mut tree = tree(CodeEditor::new("review", controller.clone())
        .font_size(26.0).diff_backgrounds(false)
        .review(snapshot.document_id(), snapshot.revision(), vec![CodeReviewHunk {
            new_rows: 1..2, old_start_row: 1, removed_lines: vec!["old".into()].into(),
        }]).render());
    let frame = tree.layout(Size::new(400.0, 320.0));
    let old = text(&frame, "old");
    let new = text(&frame, "new");
    assert_eq!(old.1, 26.0);
    assert_eq!(old.0.y - text(&frame, "kept").0.y, 40.0);
    assert_eq!(new.0.y - old.0.y, 40.0);
    assert!(!frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect {rect, color, ..} if rect.width > 2.0 && color.a == 28)));
    tree.handle_event(UiEvent::PointerDown(Point::new(new.0.x + 26.0, new.0.y + 2.0)));
    let snapshot = controller.snapshot().unwrap();
    let position = snapshot.position_of(snapshot.selection().head).unwrap();
    assert_eq!((position.row, position.column), (1, 2));
    assert_eq!(snapshot.revision(), 0);
}

#[test]
fn pointer_uses_target_editors_font_even_when_another_editor_holds_focus() {
    let large = EditorController::new(TextBuffer::new("large").unwrap());
    let small = EditorController::new(TextBuffer::new("small").unwrap());
    let mut tree = tree(Element::column(vec![
        CodeEditor::new("large", large.clone()).font_size(26.0).height(120.0).render(),
        CodeEditor::new("small", small.clone()).font_size(13.0).height(120.0).render(),
    ]));
    let frame = tree.layout(Size::new(400.0, 320.0));
    assert!(tree.focus("large"));
    let (position, _) = text(&frame, "small");
    tree.handle_event(UiEvent::PointerDown(Point::new(position.x + 13.0, position.y + 2.0)));
    assert_eq!(small.snapshot().unwrap().selection().head, 2);
    assert_eq!(large.snapshot().unwrap().selection().head, 0);
}

#[test]
fn equal_code_blocks_and_cached_markdown_previews_have_independent_scrolls() {
    let palette = ComponentPalette::neutral_dark();
    let code = "very long code line repeated exactly in every viewport";
    let source = format!("```\n{code}\n```\n\n```\n{code}\n```");
    let preview = |id: &str| Markdown::new(&source, palette).id(id).width(150.0)
        .code_word_wrap(false).render().element;
    let root = || Element::column(vec![
        CodeBlock::new(code, palette).id("standalone-a").word_wrap(false).render(),
        CodeBlock::new(code, palette).id("standalone-b").word_wrap(false).render(),
        preview("message-a"), preview("message-b"),
    ]);
    let mut tree = tree(root());
    let frame = tree.layout(Size::new(150.0, 800.0));
    let viewports = frame.regions.iter().filter(|region| region.kind == limn::HitKind::Scroll)
        .map(|region| (region.id.clone(), region.rect)).collect::<Vec<_>>();
    assert_eq!(viewports.len(), 6);
    assert_eq!(viewports.iter().map(|(id, _)| id).collect::<std::collections::HashSet<_>>().len(), 6);
    let target = &viewports[2];
    tree.handle_event(UiEvent::Scroll2D {
        position: Point::new(target.1.x + 10.0, target.1.y + 10.0),
        delta_x: -50.0, delta_y: 0.0, shift: false, direct: true,
    });
    assert!(tree.horizontal_scroll_offset(&target.0) > 0.0);
    for (id, _) in &viewports {
        if id != &target.0 { assert_eq!(tree.horizontal_scroll_offset(id), 0.0); }
    }
    tree.set_root(root());
    tree.layout(Size::new(150.0, 800.0));
    assert!(tree.horizontal_scroll_offset(&target.0) > 0.0, "stable IDs survive rebuilding cached previews");
    let anonymous = || Markdown::new(&source, palette).width(150.0).code_word_wrap(false).render().element;
    tree.set_root(Element::column(vec![anonymous(), anonymous(),
        CodeBlock::new(code, palette).word_wrap(false).render(),
        CodeBlock::new(code, palette).word_wrap(false).render()]));
    let frame = tree.layout(Size::new(150.0, 800.0));
    let ids = frame.regions.iter().filter(|region| region.kind == limn::HitKind::Scroll)
        .map(|region| &region.id).collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 6, "anonymous instances never share mutable state");
}

#[test]
fn an_editor_paints_its_built_in_syntax_in_the_hosts_inks() {
    let controller = EditorController::new(TextBuffer::new("fn main() {}\n").unwrap());
    let keyword = limn::Color::rgb(1, 2, 3);
    let palette = ComponentPalette::neutral_dark();
    let colors = limn::CodeSyntaxColors {
        keyword,
        ..limn::CodeSyntaxColors::for_palette(palette)
    };
    let root = CodeEditor::new("code", controller)
        .palette(palette)
        .language(limn::CodeLanguage::Rust)
        .syntax_colors(colors)
        .render();
    let mut tree = tree(root);
    // The colours are worked out beside the tree: lay it out until they come.
    let inked = (0..400).any(|_| {
        let frame = tree.layout(Size::new(400.0, 200.0));
        let found = frame.commands.iter().any(|command| {
            matches!(command, DrawCommand::Text { text, color, .. } if text == "fn" && *color == keyword)
        });
        if !found {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        found
    });
    assert!(inked, "the keyword is never painted in the host's ink");
}

#[test]
fn change_marks_draw_bars_beside_their_rows_and_wedges_where_lines_went() {
    let buffer = TextBuffer::new("a\nb\nc\nd\n").unwrap();
    let document = buffer.document_id();
    let controller = EditorController::new(buffer);
    let (green, red) = (limn::Color::rgb(0, 200, 0), limn::Color::rgb(200, 0, 0));
    let marks = vec![
        limn::CodeChangeMark { rows: 1..3, color: green },
        limn::CodeChangeMark { rows: 3..3, color: red },
    ];
    let editor = |document| {
        CodeEditor::new("code", controller.clone())
            .change_marks(document, marks.clone())
            .render()
    };
    let fills = |frame: &limn::Frame, ink| {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == ink => Some(*rect),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let frame = tree(editor(document)).layout(Size::new(400.0, 200.0));
    let bars = fills(&frame, green);
    assert_eq!(bars.len(), 2);
    assert!(bars.iter().all(|bar| bar.width == 3.0));
    let wedge = fills(&frame, red);
    assert_eq!(wedge.len(), 1);
    assert_eq!(wedge[0].y + 3.0, bars[1].y + bars[1].height, "between the last bar and row 3");
    // Marks of another document are not drawn.
    let frame = tree(editor(document + 1)).layout(Size::new(400.0, 200.0));
    assert!(fills(&frame, green).is_empty());
}
