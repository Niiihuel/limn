use super::*;
use crate::{HitKind, Size, UiTree};

fn actions(count: usize, token: &str) -> Arc<[CodeReviewHunkActions]> {
    (0..count)
        .map(|index| CodeReviewHunkActions {
            accept_id: format!("editor_review:accept_hunk:{token}:{index}"),
            reject_id: format!("editor_review:reject_hunk:{token}:{index}"),
            previous_id: Some(format!("editor_review:previous:{token}:{index}")),
            next_id: Some(format!("editor_review:next:{token}:{index}")),
            enabled: true,
            reject_enabled: true,
            disabled_reason: None,
        })
        .collect::<Vec<_>>()
        .into()
}

fn measure(text: &str) -> f32 {
    UnicodeWidthStr::width(text) as f32 * 8.0
}

fn fixture(text: &str, hunks: Vec<CodeReviewHunk>) -> (EditorController, EditorView) {
    let controller = EditorController::new(TextBuffer::new(text).unwrap());
    let snapshot = controller.snapshot().unwrap();
    let editor = CodeEditor::new("review", controller.clone()).review(
        snapshot.document_id(),
        snapshot.revision(),
        hunks,
    );
    (controller, EditorView::new(editor, Instant::now()))
}

fn hunk(rows: Range<usize>, old: usize, removed: &[&str]) -> CodeReviewHunk {
    CodeReviewHunk {
        new_rows: rows,
        old_start_row: old,
        removed_lines: removed
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .into(),
    }
}

fn paint(view: &mut EditorView, width: f32, height: f32) -> Vec<DrawCommand> {
    let rect = Rect::new(0.0, 0.0, width, height);
    view.paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
}

fn text(controller: &EditorController) -> String {
    let snapshot = controller.snapshot().unwrap();
    snapshot.text_range(0..snapshot.char_count()).unwrap()
}

fn fills(commands: &[DrawCommand], expected: Color) -> Vec<Rect> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == expected => Some(*rect),
            _ => None,
        })
        .collect()
}

fn key(view: &mut EditorView, key: Key, ctrl: bool) -> Vec<UiAction> {
    view.input(
        &UiEvent::Key {
            key,
            shift: false,
            ctrl,
            alt: false,
        },
        Instant::now(),
        &measure,
    )
}

#[test]
fn exact_review_renders_old_and_current_rows_without_mutating_crlf_unicode_source() {
    let source = "head\r\n\t東京e\u{301}\r\ntail";
    let (controller, mut view) = fixture(
        source,
        vec![hunk(1..2, 1, &["obsolete α\r\n", "second old\n"])],
    );
    let commands = paint(&mut view, 500.0, 240.0);
    let red = fills(&commands, REMOVED_FILL);
    let green = fills(&commands, ADDED_FILL);
    assert_eq!(red.len(), 2);
    assert_eq!(green.len(), 1);
    assert_eq!(red[0].y, view.viewport.body.y + LINE_HEIGHT);
    assert_eq!(green[0].y, view.viewport.body.y + 3.0 * LINE_HEIGHT);
    assert!(commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "−2")));
    assert!(commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "2")));
    assert_eq!(text(&controller), source);
    assert_eq!(controller.snapshot().unwrap().revision(), 0);
}

#[test]
fn pointer_and_clipboard_never_treat_deleted_view_zones_as_editable_text() {
    let (controller, mut view) =
        fixture("head\nnew\ntail", vec![hunk(1..2, 1, &["OLD A", "OLD B"])]);
    paint(&mut view, 500.0, 240.0);
    let body = view.viewport.body;
    let old = Point::new(body.x + 16.0, body.y + LINE_HEIGHT + 5.0);
    let original = controller.snapshot().unwrap().selection();
    assert!(view.point(old, false, true, &measure).is_empty());
    assert!(view
        .pointer_modified(old, false, true, false, 2, true, &measure)
        .is_empty());
    assert!(view.context_point(old, &measure).is_empty());
    assert_eq!(controller.snapshot().unwrap().selection(), original);
    let current = Point::new(body.x + 8.0, body.y + 3.0 * LINE_HEIGHT + 5.0);
    view.point(current, false, false, &measure);
    assert_eq!(controller.snapshot().unwrap().selection().head, 6);
    view.point(Point::new(body.x, body.y + 4.0), false, true, &measure);
    view.point(
        Point::new(body.x + 32.0, body.y + 4.0 * LINE_HEIGHT + 4.0),
        true,
        true,
        &measure,
    );
    assert_eq!(
        key(&mut view, Key::C, true),
        vec![UiAction::CopyRequested {
            text: "head\nnew\ntail".into()
        }]
    );
    assert_eq!(text(&controller), "head\nnew\ntail");
}

#[test]
fn navigation_ignores_removed_lines_and_any_edit_invalidates_snapshot_decorations() {
    let (controller, mut view) = fixture("first\nnew\nlast", vec![hunk(1..2, 1, &["a", "b", "c"])]);
    paint(&mut view, 400.0, 140.0);
    key(&mut view, Key::ArrowDown, false);
    assert_eq!(
        controller
            .snapshot()
            .unwrap()
            .position_of(controller.snapshot().unwrap().selection().head)
            .unwrap()
            .row,
        1
    );
    key(&mut view, Key::ArrowUp, false);
    assert_eq!(controller.snapshot().unwrap().selection().head, 0);
    let actions = view.input(&UiEvent::Text("X".into()), Instant::now(), &measure);
    assert!(actions
        .iter()
        .any(|action| matches!(action, UiAction::EditorChanged { .. })));
    let commands = paint(&mut view, 400.0, 140.0);
    assert!(fills(&commands, REMOVED_FILL).is_empty());
    assert!(fills(&commands, ADDED_FILL).is_empty());
    assert_eq!(
        view.review_layout.rows,
        controller.snapshot().unwrap().line_count()
    );
    assert_eq!(text(&controller), "Xfirst\nnew\nlast");
    assert!(!view.reveal_review_hunk(0));
}

#[test]
fn eof_deletion_has_scroll_extent_and_explicit_reveal_without_fake_caret() {
    let old: Vec<String> = (0..120).map(|row| format!("removed {row}")).collect();
    let (controller, mut view) = fixture(
        "kept",
        vec![CodeReviewHunk {
            new_rows: 1..1,
            old_start_row: 1,
            removed_lines: old.into(),
        }],
    );
    paint(&mut view, 420.0, 160.0);
    let selection = controller.snapshot().unwrap().selection();
    assert!(view.viewport.max_y > 2000.0);
    assert!(view.reveal_review_hunk(0));
    let commands = paint(&mut view, 420.0, 160.0);
    assert!(!fills(&commands, REMOVED_FILL).is_empty());
    assert_eq!(view.viewport.scroll_y, LINE_HEIGHT);
    assert_eq!(controller.snapshot().unwrap().selection(), selection);
    view.viewport.scroll_y = view.viewport.max_y;
    paint(&mut view, 420.0, 160.0);
    let body = view.viewport.body;
    view.point(
        Point::new(body.x + 20.0, body.y + 20.0),
        true,
        true,
        &measure,
    );
    assert_eq!(controller.snapshot().unwrap().selection().head, 4);
    assert_eq!(text(&controller), "kept");
}

#[test]
fn huge_deleted_zone_draws_only_visible_rows_and_minimap_uses_presentation_positions() {
    let (controller, mut view) = fixture(
        "before\nafter",
        vec![CodeReviewHunk {
            new_rows: 1..1,
            old_start_row: 1,
            removed_lines: (0..10_000)
                .map(|row| format!("old {row}"))
                .collect::<Vec<_>>()
                .into(),
        }],
    );
    view.editor.minimap = true;
    let commands = paint(&mut view, 700.0, 200.0);
    assert!(fills(&commands, REMOVED_FILL).len() <= 11);
    assert!(commands.len() < 500);
    view.viewport.scroll_y = view.viewport.max_y;
    let commands = paint(&mut view, 700.0, 200.0);
    assert!(commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "after")));
    let body = view.viewport.body;
    let after_y =
        body.y + view.review_layout.display_row(1) as f32 * LINE_HEIGHT - view.viewport.scroll_y;
    view.point(Point::new(body.x, after_y + 5.0), false, false, &measure);
    assert_eq!(controller.snapshot().unwrap().selection().head, 7);
}

#[test]
fn removed_soft_wrap_preserves_graphemes_tabs_and_real_row_mapping() {
    let old = "\t東京e\u{301} ".repeat(8);
    let (controller, mut view) = fixture(
        "head\nreplacement\ntail",
        vec![hunk(1..2, 1, &[old.as_str()])],
    );
    view.editor.soft_wrap = true;
    paint(&mut view, 160.0, 300.0);
    let zone = &view.review_layout.zones[0];
    assert!(zone.fragments.len() > 1);
    assert_eq!(
        zone.fragments
            .iter()
            .map(|fragment| &old[fragment.bytes.clone()])
            .collect::<String>(),
        old
    );
    for fragment in &zone.fragments {
        let exact = &old[fragment.bytes.clone()];
        let rendered = DisplayLine::window_at(
            exact,
            4,
            fragment.column as f32 * 8.0,
            500.0,
            fragment.column,
            &measure,
        );
        assert!(rendered
            .clusters
            .iter()
            .all(|cluster| cluster.x >= fragment.column as f32 * 8.0));
    }
    let end = zone.end;
    assert_eq!(view.review_layout.visual_row(end), 1);
    assert_eq!(view.review_layout.display_row(1), end);
    assert_eq!(view.viewport.max_x, 0.0);
    assert_eq!(text(&controller), "head\nreplacement\ntail");
    let initial_rows = view.review_layout.rows;
    paint(&mut view, 360.0, 300.0);
    assert!(view.review_layout.rows < initial_rows);
}

#[test]
fn folded_hidden_review_zones_restore_after_unfold_and_cache_tracks_arc_identity() {
    let (_, mut view) = fixture("zero\none\ntwo\nthree\nfour", vec![hunk(2..3, 2, &["OLD"])]);
    view.editor.folds = vec![1..4].into();
    paint(&mut view, 420.0, 200.0);
    assert!(view.review_layout.zones.is_empty());
    view.editor.folds = Arc::from([]);
    paint(&mut view, 420.0, 200.0);
    assert_eq!(view.review_layout.zones.len(), 1);
    let generation = view.review_layout.generation;
    view.editor.review_hunks = vec![hunk(2..3, 2, &["OLD", "ANOTHER"])].into();
    paint(&mut view, 420.0, 200.0);
    assert!(view.review_layout.generation > generation);
    assert_eq!(view.review_layout.rows, 7);
}

#[test]
fn invalid_or_foreign_decorations_are_hidden_as_one_atomic_overlay() {
    let (_, mut view) = fixture(
        "one\ntwo",
        vec![hunk(0..1, 0, &["old"]), hunk(9..10, 0, &["invalid"])],
    );
    let commands = paint(&mut view, 420.0, 180.0);
    assert!(fills(&commands, REMOVED_FILL).is_empty());
    assert!(fills(&commands, ADDED_FILL).is_empty());
    view.editor.review_hunks = vec![hunk(0..1, 0, &["old"])].into();
    view.editor.review_document_id = Some(u64::MAX);
    let commands = paint(&mut view, 420.0, 180.0);
    assert!(fills(&commands, REMOVED_FILL).is_empty());
    assert_eq!(view.review_layout.rows, 2);
}

#[test]
fn page_navigation_crosses_visible_review_rows_without_counting_them_as_source_lines() {
    let (_, mut view) = fixture(
        "a\nb\nc\nd\ne\nf\ng\nh\ni\nj",
        vec![hunk(2..3, 2, &["old a", "old b", "old c"])],
    );
    paint(&mut view, 420.0, 180.0);
    assert_eq!(view.review_page_target(0, 6, false, 10), 3);
    assert_eq!(view.review_page_target(3, 5, true, 10), 1);
    assert_eq!(view.review_page_target(2, 2, true, 10), 1);
}

#[test]
fn selection_contours_leave_deleted_zones_unselected_and_copy_only_current_document() {
    let (controller, mut view) =
        fixture("head\nnew\ntail", vec![hunk(1..2, 1, &["OLD A", "OLD B"])]);
    paint(&mut view, 420.0, 200.0);
    key(&mut view, Key::A, true);
    let commands = paint(&mut view, 420.0, 200.0);
    let old_top = view.viewport.body.y + LINE_HEIGHT;
    let old_bottom = old_top + 2.0 * LINE_HEIGHT;
    let contours: Vec<_> = commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::SelectionContour { points, .. } => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(
        contours.len(),
        2,
        "the old zone splits the real selection's paint contour"
    );
    for points in contours {
        let top = points
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        let bottom = points
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(bottom <= old_top || top >= old_bottom);
    }
    assert_eq!(
        key(&mut view, Key::C, true),
        vec![UiAction::CopyRequested {
            text: text(&controller)
        }]
    );
}

#[test]
fn review_zones_preserve_current_scope_and_breakpoint_hit_regions() {
    let source = (0..40)
        .map(|row| format!("line_{row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (controller, mut view) = fixture(&source, vec![hunk(2..3, 2, &["old a", "old b"])]);
    let snapshot = controller.snapshot().unwrap();
    view.editor = view
        .editor
        .clone()
        .structure(
            snapshot.document_id(),
            snapshot.revision(),
            Vec::<CodeFoldCandidate>::new(),
            vec![CodeStickyScope {
                start_row: 0,
                end_row: 35,
                action_id: "scope:0".into(),
            }],
        )
        .breakpoints(
            snapshot.document_id(),
            snapshot.revision(),
            "breakpoint",
            vec![2],
        );
    paint(&mut view, 420.0, 200.0);
    let old_top = view.viewport.body.y + 2.0 * LINE_HEIGHT;
    let old_bottom = old_top + 2.0 * LINE_HEIGHT;
    assert!(view
        .structure_controls
        .iter()
        .all(|control| control.rect.y < old_top || control.rect.y >= old_bottom));
    let current = view
        .structure_controls
        .iter()
        .find(|control| control.action_id.ends_with(":2"))
        .unwrap();
    assert_eq!(current.rect.y, old_bottom);
    view.viewport.scroll_y = 6.0 * LINE_HEIGHT;
    paint(&mut view, 420.0, 200.0);
    assert!(view
        .structure_controls
        .iter()
        .any(|control| control.action_id == "scope:0"));
    view.editor.reveal_top_inset = 18.0;
    assert!(view.reveal_review_hunk(0));
    let revealed = paint(&mut view, 420.0, 200.0);
    let red = fills(&revealed, REMOVED_FILL);
    assert_eq!(red.len(), 2);
    assert!(
        red[0].y >= view.content_visible.y,
        "sticky and floating headers must not hide the first removed line"
    );
    assert_eq!(controller.snapshot().unwrap().selection().head, 0);
}

#[test]
fn active_hunk_buttons_are_real_clipped_controls_and_never_edit_or_drag_the_source() {
    for width in [240.0, 360.0, 900.0] {
        let (controller, mut view) = fixture("head\nnew\ntail", vec![hunk(1..2, 1, &["old"])]);
        view.editor.review_actions = actions(1, "owner-version");
        let mut tree = UiTree::new(view.editor.clone().height(180.0).render());
        let frame = tree.layout(Size::new(width, 180.0));
        let source = frame
            .regions
            .iter()
            .find(|region| region.id == "review")
            .unwrap()
            .rect;
        let before = controller.snapshot().unwrap().selection();
        for (operation, label) in [("reject_hunk", "Undo"), ("accept_hunk", "Keep")] {
            let id = format!("editor_review:{operation}:owner-version:0");
            let region = frame
                .regions
                .iter()
                .find(|region| region.id == id && region.kind == HitKind::Button)
                .unwrap();
            assert_eq!(region.rect.width, 80.0, "both decisions use the same width");
            assert_eq!(region.rect.height, ControlSize::Dense.height());
            assert!(source.contains(Point::new(region.rect.x, region.rect.y)));
            assert!(source.contains(Point::new(
                region.rect.x + region.rect.width - 0.1,
                region.rect.y + region.rect.height - 0.1
            )));
            assert!(frame
                .commands
                .iter()
                .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == label)));
            let point = Point::new(
                region.rect.x + region.rect.width * 0.5,
                region.rect.y + region.rect.height * 0.5,
            );
            tree.handle_event(UiEvent::PointerDown(point));
            tree.handle_event(UiEvent::PointerMove(Point::new(point.x + 1.0, point.y)));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id }]
            );
        }
        assert_eq!(controller.snapshot().unwrap().selection(), before);
        assert_eq!(controller.snapshot().unwrap().revision(), 0);
        assert_eq!(text(&controller), "head\nnew\ntail");
    }
}

#[test]
fn inline_stepper_is_scoped_clipped_and_preserves_decisions_in_narrow_editors() {
    let (controller, mut view) = fixture(
        "one\ntwo\nthree\nfour\nfive",
        vec![hunk(0..1, 0, &[]), hunk(2..3, 2, &[]), hunk(4..5, 4, &[])],
    );
    view.editor.review_actions = actions(3, "owner-version");
    let mut tree = UiTree::new(view.editor.clone().height(220.0).render());
    tree.layout(Size::new(420.0, 220.0));
    assert!(tree.reveal_code_review_hunk("review", 1));
    let before = controller.snapshot().unwrap().selection();
    let frame = tree.layout(Size::new(420.0, 220.0));
    assert!(frame.regions.iter().all(|region| !region.id.is_empty()));
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "2/3")));
    for operation in ["previous", "next"] {
        let id = format!("editor_review:{operation}:owner-version:1");
        let control = frame.regions.iter().find(|region| region.id == id).unwrap();
        let point = Point::new(control.rect.x + 12.0, control.rect.y + 12.0);
        assert_eq!(control.rect.height, ControlSize::Dense.height());
        assert_eq!(frame.hit_test(point), Some(id.as_str()));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id }]
        );
    }
    assert_eq!(controller.snapshot().unwrap().selection(), before);
    assert_eq!(text(&controller), "one\ntwo\nthree\nfour\nfive");

    let narrow = tree.layout(Size::new(240.0, 220.0));
    let operations: Vec<_> = narrow
        .regions
        .iter()
        .filter(|region| region.kind == HitKind::Button && region.id.starts_with("editor_review:"))
        .map(|region| region.id.as_str())
        .collect();
    assert_eq!(
        operations,
        [
            "editor_review:reject_hunk:owner-version:1",
            "editor_review:accept_hunk:owner-version:1",
        ]
    );
}

#[test]
fn inline_stepper_disables_ends_and_omits_invalid_or_absent_navigation_ids() {
    let (_, mut view) = fixture(
        "one\ntwo\nthree",
        vec![hunk(0..1, 0, &[]), hunk(2..3, 2, &[])],
    );
    view.editor.review_actions = actions(2, "v1");
    let mut tree = UiTree::new(view.editor.clone().height(220.0).render());
    let first = tree.layout(Size::new(420.0, 220.0));
    let navigation = |frame: &crate::Frame| {
        frame
            .regions
            .iter()
            .filter(|region| {
                region.kind == HitKind::Button
                    && (region.id.starts_with("editor_review:previous:")
                        || region.id.starts_with("editor_review:next:"))
            })
            .map(|region| region.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(navigation(&first), ["editor_review:next:v1:0"]);
    assert!(tree.reveal_code_review_hunk("review", 1));
    let last = tree.layout(Size::new(420.0, 220.0));
    assert_eq!(navigation(&last), ["editor_review:previous:v1:1"]);

    for invalid in [
        None,
        Some("editor_review:accept_hunk:v1:0"),
        Some("invalid\nnav"),
    ] {
        let mut values = actions(2, "v1").to_vec();
        values[0].previous_id = invalid.map(str::to_owned);
        view.editor.review_actions = values.into();
        let mut tree = UiTree::new(view.editor.clone().height(220.0).render());
        let frame = tree.layout(Size::new(420.0, 220.0));
        assert!(navigation(&frame).is_empty());
        assert!(frame.regions.iter().any(|region| {
            region.kind == HitKind::Button && region.id == "editor_review:accept_hunk:v1:0"
        }));
    }
}

#[test]
fn active_hunk_counter_follows_scroll_caret_and_pending_explicit_navigation() {
    let source = (0..100)
        .map(|row| format!("line{row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (controller, mut view) = fixture(
        &source,
        [2, 20, 40, 70]
            .into_iter()
            .map(|row| hunk(row..row + 1, row, &[]))
            .collect(),
    );
    view.editor.review_actions = actions(4, "v1");
    let mut tree = UiTree::new(view.editor.clone().height(200.0).render());
    tree.layout(Size::new(500.0, 200.0));
    assert_eq!(tree.code_review_current_hunk("review").unwrap().index, 0);
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(100.0, 100.0),
        delta_y: -1400.0,
    });
    tree.layout(Size::new(500.0, 200.0));
    let position = tree.code_review_current_hunk("review").unwrap();
    assert_eq!(
        position.index, 3,
        "the visible fourth block must not retain the first block's count"
    );
    assert_eq!(position.accept_id, "editor_review:accept_hunk:v1:3");
    assert_eq!(
        controller.snapshot().unwrap().selection().head,
        0,
        "scroll never moves the caret"
    );
    assert!(tree.reveal_code_review_hunk("review", 1));
    assert_eq!(
        tree.code_review_current_hunk("review").unwrap().index,
        1,
        "a poll before layout cannot undo explicit Next"
    );
    tree.layout(Size::new(500.0, 200.0));
    assert_eq!(tree.code_review_current_hunk("review").unwrap().index, 1);
    let row = controller.snapshot().unwrap().line_range(40).unwrap();
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(row.start)))
        .unwrap();
    tree.reveal_code_caret("review");
    tree.layout(Size::new(500.0, 200.0));
    assert_eq!(tree.code_review_current_hunk("review").unwrap().index, 2);
}

#[test]
fn block_widget_hover_requires_intent_and_stationary_hover_cannot_override_new_caret() {
    let (controller, mut view) = fixture(
        "zero\none\ntwo\nthree\nfour",
        vec![hunk(0..1, 0, &[]), hunk(3..4, 3, &[])],
    );
    view.editor.review_actions = actions(2, "v1");
    paint(&mut view, 500.0, 200.0);
    let now = Instant::now();
    let pointer = Point::new(
        view.viewport.body.x + 3.0,
        view.viewport.body.y + 3.0 * LINE_HEIGHT + 3.0,
    );
    view.set_review_pointer(Some("review"), Some(pointer), now);
    paint(&mut view, 500.0, 200.0);
    assert_eq!(view.current_review_hunk().unwrap().index, 0);
    assert!(view.review_interaction.hover_pending());
    view.set_review_pointer(
        Some("review"),
        Some(pointer),
        now + Duration::from_millis(1100),
    );
    paint(&mut view, 500.0, 200.0);
    assert_eq!(view.current_review_hunk().unwrap().index, 1);
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(1)))
        .unwrap();
    paint(&mut view, 500.0, 200.0);
    assert_eq!(view.current_review_hunk().unwrap().index, 0);
    assert!(!view.review_interaction.hover_pending());
}

#[test]
fn stale_review_has_no_block_actions_and_disabled_rejection_cannot_dispatch() {
    let (controller, mut view) = fixture("new", vec![hunk(0..1, 0, &["old"])]);
    let mut action = actions(1, "v1")[0].clone();
    action.reject_enabled = false;
    action.disabled_reason = Some("El archivo cambió.".into());
    view.editor.review_actions = vec![action].into();
    let mut tree = UiTree::new(view.editor.clone().height(160.0).render());
    let frame = tree.layout(Size::new(400.0, 160.0));
    assert!(!frame
        .regions
        .iter()
        .any(|region| region.id == "editor_review:reject_hunk:v1:0"
            && region.kind == HitKind::Button));
    controller
        .execute(EditorCommand::InsertText("local ".into()))
        .unwrap();
    let frame = tree.layout(Size::new(400.0, 160.0));
    assert!(tree.code_review_current_hunk("review").is_none());
    assert!(!frame
        .regions
        .iter()
        .any(|region| region.id.starts_with("editor_review:")));
    assert_eq!(text(&controller), "local new");
}

#[test]
fn wheel_over_block_button_scrolls_the_owning_editor_and_clips_fallback_widget() {
    let text = (0..60)
        .map(|row| format!("row{row}"))
        .collect::<Vec<_>>()
        .join("\n");
    let (_, mut view) = fixture(&text, vec![hunk(0..50, 0, &[])]);
    view.editor.review_actions = actions(1, "v1");
    let mut tree = UiTree::new(view.editor.clone().height(180.0).render());
    let frame = tree.layout(Size::new(360.0, 180.0));
    let button = frame
        .regions
        .iter()
        .find(|region| region.id == "editor_review:accept_hunk:v1:0")
        .unwrap()
        .rect;
    assert!(button.y >= 0.0 && button.y + button.height <= 180.0);
    let point = Point::new(button.x + 5.0, button.y + 5.0);
    tree.handle_event(UiEvent::ScrollDirect {
        position: point,
        delta_y: -400.0,
    });
    let frame = tree.layout(Size::new(360.0, 180.0));
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "row20")));
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "row0")));
}
