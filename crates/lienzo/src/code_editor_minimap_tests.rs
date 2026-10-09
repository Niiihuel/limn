use super::*;
use crate::{Size, UiTree};

fn fixture(rows: usize) -> (EditorController, EditorView) {
    let source = (0..rows)
        .map(|row| format!("    let value_{row} = source_{row}.read().map(|entry| entry.value()).unwrap_or_default();"))
        .collect::<Vec<_>>()
        .join("\n");
    let controller = EditorController::new(TextBuffer::new(&source).unwrap());
    let editor = CodeEditor::new("source", controller.clone()).minimap(true);
    (controller, EditorView::new(editor, Instant::now()))
}

fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 8.0
}

fn paint(view: &mut EditorView, width: f32) -> Vec<DrawCommand> {
    let rect = Rect::new(0.0, 0.0, width, 300.0);
    view.paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
}

fn selection_blocks(commands: &[DrawCommand]) -> Vec<Rect> {
    commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == selection::COLOR => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn minimap_selection_tracks_unicode_columns_without_rebuilding_code_density() {
    let controller = EditorController::new(TextBuffer::new("\t東京e\u{301} tail\nnext").unwrap());
    let mut view = EditorView::new(
        CodeEditor::new("selected-minimap", controller.clone()).minimap(true),
        Instant::now(),
    );
    paint(&mut view, 700.0);
    let builds = view.minimap_cache.builds;
    let code_before: Vec<_> = view.minimap_cache.runs.iter().map(|run| run.rect).collect();
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 1,
            head: 5,
        }))
        .unwrap();
    let selected = paint(&mut view, 700.0);
    let layout = view.viewport.minimap.unwrap();
    assert_eq!(
        selection_blocks(&selected),
        vec![Rect::new(
            layout.rect.x + 4.0 + 4.0 * CELL_WIDTH,
            layout.rect.y,
            5.0 * CELL_WIDTH,
            ROW_HEIGHT,
        )]
    );
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 2,
            head: 3,
        }))
        .unwrap();
    let moved = paint(&mut view, 700.0);
    assert_eq!(
        selection_blocks(&moved),
        vec![Rect::new(
            layout.rect.x + 4.0 + 6.0 * CELL_WIDTH,
            layout.rect.y,
            2.0 * CELL_WIDTH,
            ROW_HEIGHT,
        )]
    );
    assert_eq!(view.minimap_cache.builds, builds);
    controller
        .execute(EditorCommand::InsertText("界界".into()))
        .unwrap();
    let edited = paint(&mut view, 700.0);
    assert!(selection_blocks(&edited).is_empty());
    assert_eq!(view.minimap_cache.builds, builds + 1);
    let code_after: Vec<_> = view.minimap_cache.runs.iter().map(|run| run.rect).collect();
    assert_ne!(
        code_before, code_after,
        "editing refreshes the miniature code"
    );
}

#[test]
fn minimap_selection_uses_visual_rows_after_wrap_and_fold() {
    let (controller, mut view) = fixture(150);
    view.editor.soft_wrap = true;
    view.editor.folds = vec![2..120].into();
    let snapshot = controller.snapshot().unwrap();
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 0,
            head: snapshot.line_range(130).unwrap().start + 12,
        }))
        .unwrap();
    let commands = paint(&mut view, 480.0);
    let layout = view.viewport.minimap.unwrap();
    let map = &view.visual_map.as_ref().unwrap().4;
    assert!(!map.rows.iter().any(|row| row.physical_row == 80));
    let blocks = selection_blocks(&commands);
    assert!(!blocks.is_empty());
    let continuation = map.rows.iter().position(|row| row.continuation).unwrap();
    assert!(blocks
        .iter()
        .any(|rect| (rect.y - layout.rect.y - layout.row_y(continuation)).abs() < 0.01));
    assert!(blocks.iter().all(|rect| rect.x >= layout.rect.x + 4.0
        && rect.x + rect.width <= layout.rect.x + layout.rect.width - 4.0
        && rect.y >= layout.rect.y
        && rect.y + rect.height <= layout.rect.y + layout.content_height + 0.01));
}

#[test]
fn minimap_reserves_space_without_changing_source_metrics_and_hides_on_narrow_views() {
    let (_, mut view) = fixture(200);
    let source_editor = view.editor.clone().minimap(false);
    let mut source = EditorView::new(source_editor, Instant::now());
    paint(&mut source, 700.0);
    let commands = paint(&mut view, 700.0);
    let mini = view.viewport.minimap.unwrap();
    assert_eq!(mini.rect.width, WIDTH);
    assert_eq!(view.viewport.body.width + WIDTH, source.viewport.body.width);
    assert_eq!(view.viewport.body.x, source.viewport.body.x);
    assert_eq!(mini.rect.x, view.viewport.body.x + view.viewport.body.width);
    assert_eq!(mini.rect.x + mini.rect.width, 692.0);
    assert!(commands.iter().any(|command| matches!(command,
        DrawCommand::Text {text, font_size, ..} if text.contains("value_0") && *font_size == FONT_SIZE
    )));
    paint(&mut view, 360.0);
    paint(&mut source, 360.0);
    assert!(view.viewport.minimap.is_none());
    assert_eq!(view.viewport.body, source.viewport.body);
    assert!(!CodeEditor::new("default", view.editor.controller.clone()).minimap);
}

#[test]
fn minimap_click_drag_and_modifiers_scroll_without_editing_or_moving_selection() {
    let (controller, mut view) = fixture(500);
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 2,
            head: 10,
        }))
        .unwrap();
    let before = controller.snapshot().unwrap();
    paint(&mut view, 700.0);
    let mini = view.viewport.minimap.unwrap();
    let target = Point::new(mini.rect.x + 20.0, mini.rect.y + mini.content_height * 0.6);
    assert!(view
        .pointer_modified(target, true, true, true, 3, true, &measure)
        .is_empty());
    assert!(view.viewport.scroll_y > view.viewport.max_y * 0.5);
    assert!(view.viewport.minimap_drag);
    assert!(!view.viewport.reveal_caret);
    let end = Point::new(target.x, mini.rect.y + mini.content_height + 100.0);
    assert!(view.point(end, true, true, &measure).is_empty());
    assert_eq!(view.viewport.scroll_y, view.viewport.max_y);
    view.point(end, true, false, &measure);
    assert!(view.viewport.scrollbar_drag.is_none());
    assert!(!view.viewport.minimap_drag);
    let after = controller.snapshot().unwrap();
    assert_eq!(after.revision(), before.revision());
    assert_eq!(after.selection(), before.selection());
    assert_eq!(after.is_dirty(), before.is_dirty());
    // A source navigation still reveals the actual caret after minimap scrolling.
    view.viewport.reveal_caret = true;
    paint(&mut view, 700.0);
    assert_eq!(view.viewport.scroll_y, 0.0);
}

#[test]
fn minimap_thumb_grab_does_not_jump_and_wheel_and_right_scrollbar_still_work() {
    let (controller, mut view) = fixture(600);
    paint(&mut view, 700.0);
    view.viewport.scroll(0.0, -1200.0);
    let layout = view.viewport.minimap.unwrap();
    let thumb = layout.thumb(&view.viewport);
    let point = Point::new(thumb.x + 12.0, thumb.y + thumb.height * 0.3);
    let previous = view.viewport.scroll_y;
    view.pointer_modified(point, false, false, false, 1, true, &measure);
    assert!((view.viewport.scroll_y - previous).abs() < 0.01);
    view.point(point, true, false, &measure);
    let before = controller.snapshot().unwrap().selection();
    let scrollbar = Point::new(698.0, 260.0);
    view.point(scrollbar, false, true, &measure);
    assert!(!view.viewport.minimap_drag);
    assert!(view.viewport.scroll_y > previous);
    view.point(scrollbar, true, false, &measure);
    assert_eq!(controller.snapshot().unwrap().selection(), before);
    let previous = view.viewport.scroll_y;
    view.viewport.scroll(0.0, 100.0);
    assert!(view.viewport.scroll_y < previous);
}

#[test]
fn minimap_uses_actual_wrap_and_fold_rows_and_invalidates_when_layout_changes() {
    let (controller, mut view) = fixture(150);
    view.editor.soft_wrap = true;
    view.editor.folds = vec![2..120].into();
    paint(&mut view, 480.0);
    let map = &view.visual_map.as_ref().unwrap().4;
    assert!(!map.rows.iter().any(|row| row.physical_row == 80));
    assert_eq!(view.viewport.minimap.unwrap().rows, map.rows.len());
    assert!(map.rows.iter().any(|row| row.continuation));
    let builds = view.minimap_cache.builds;
    let offset = controller.snapshot().unwrap().line_range(80).unwrap().start;
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(offset)))
        .unwrap();
    paint(&mut view, 480.0);
    let map = &view.visual_map.as_ref().unwrap().4;
    assert!(map.rows.iter().any(|row| row.physical_row == 80));
    assert_eq!(view.viewport.minimap.unwrap().rows, map.rows.len());
    assert_eq!(view.minimap_cache.builds, builds + 1);
    let mini = view.viewport.minimap.unwrap();
    // Drag beyond the edge to cover the final visual row.
    view.pointer_modified(
        Point::new(mini.rect.x + 10.0, mini.rect.y + mini.content_height - 1.0),
        false,
        false,
        false,
        1,
        true,
        &measure,
    );
    view.point(Point::new(mini.rect.x + 10.0, 500.0), true, false, &measure);
    assert_eq!(view.viewport.scroll_y, view.viewport.max_y);
}

#[test]
fn minimap_cache_and_sampling_remain_bounded_for_large_and_pathological_documents() {
    let source = format!(
        "{}\n{}",
        "x".repeat(1_000_000),
        "word word word word word word word word word\n".repeat(100_000)
    );
    let controller = EditorController::new(TextBuffer::new(&source).unwrap());
    let editor = CodeEditor::new("large", controller.clone()).minimap(true);
    let mut view = EditorView::new(editor, Instant::now());
    paint(&mut view, 700.0);
    assert!(view.minimap_cache.runs.len() <= MAX_ROWS * MAX_RUNS);
    assert!(view.minimap_cache.sampled_scalars <= MAX_ROWS * MAX_SCALARS);
    let builds = view.minimap_cache.builds;
    let selected_end = controller.snapshot().unwrap().char_count();
    controller
        .execute(EditorCommand::SetSelection(Selection {
            anchor: 0,
            head: selected_end,
        }))
        .unwrap();
    let selection = selection_blocks(&paint(&mut view, 700.0));
    assert!(!selection.is_empty());
    assert!(selection.len() <= MAX_ROWS);
    assert_eq!(view.minimap_cache.builds, builds);
    for offset in [50.0, 2000.0, 70_000.0] {
        view.viewport.scroll_y = offset;
        paint(&mut view, 700.0);
    }
    assert_eq!(
        view.minimap_cache.builds, builds,
        "scrolling reuses the cached density"
    );
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(3)))
        .unwrap();
    paint(&mut view, 700.0);
    assert_eq!(
        view.minimap_cache.builds, builds,
        "selection is an uncached overlay"
    );
    controller
        .execute(EditorCommand::InsertText("new".into()))
        .unwrap();
    paint(&mut view, 700.0);
    assert_eq!(view.minimap_cache.builds, builds + 1);
}

#[test]
fn minimap_paints_only_current_syntax_and_preserves_unicode_tabs_and_indentation() {
    let controller =
        EditorController::new(TextBuffer::new("\t東京e\u{301} first second\nnext").unwrap());
    let snapshot = controller.snapshot().unwrap();
    let tint = Color::rgb(240, 50, 130);
    let editor = CodeEditor::new("unicode", controller.clone())
        .minimap(true)
        .highlights(
            snapshot.document_id(),
            snapshot.revision(),
            vec![CodeHighlight {
                range: 1..6,
                color: tint,
            }],
        );
    let mut view = EditorView::new(editor, Instant::now());
    paint(&mut view, 700.0);
    assert!(view
        .minimap_cache
        .runs
        .iter()
        .any(|run| run.color == with_alpha(tint, 145)));
    assert_eq!(view.minimap_cache.runs[0].rect.x, 4.0 + CELL_WIDTH * 4.0);
    controller
        .execute(EditorCommand::InsertText("changed".into()))
        .unwrap();
    paint(&mut view, 700.0);
    assert!(!view
        .minimap_cache
        .runs
        .iter()
        .any(|run| run.color == with_alpha(tint, 145)));
}

#[test]
fn retained_tree_dispatches_minimap_capture_and_releases_after_pointer_up() {
    let (controller, view) = fixture(300);
    let mut tree = UiTree::new(view.editor.height(300.0).render());
    tree.layout(Size::new(700.0, 300.0));
    let before = controller.snapshot().unwrap().selection();
    tree.handle_event(UiEvent::PointerDown(Point::new(650.0, 140.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(500.0, 280.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(500.0, 280.0)));
    let frame = tree.layout(Size::new(700.0, 300.0));
    assert_eq!(controller.snapshot().unwrap().selection(), before);
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text {text, ..} if text == "1")));
    tree.handle_event(UiEvent::PointerMove(Point::new(650.0, 5.0)));
    assert_eq!(controller.snapshot().unwrap().selection(), before);
}

#[test]
fn dragging_a_source_selection_across_minimap_does_not_start_scrollbar_navigation() {
    let (_, mut view) = fixture(300);
    paint(&mut view, 700.0);
    let source = Point::new(view.viewport.body.x + 15.0, view.viewport.body.y + 10.0);
    view.pointer_modified(source, false, false, false, 1, true, &measure);
    let mini = view.viewport.minimap.unwrap();
    let across = Point::new(mini.rect.x + 10.0, mini.rect.y + 100.0);
    view.point(across, true, true, &measure);
    assert!(view.viewport.dragging);
    assert!(!view.viewport.minimap_drag);
    assert!(view.viewport.scrollbar_drag.is_none());
    view.point(across, true, false, &measure);
    assert!(!view.viewport.dragging);
}

#[test]
fn editor_scrollbars_share_scrollarea_ink_and_shape_without_changing_drag_geometry() {
    let appearance = crate::ScrollbarAppearance::default();
    let ink = Color::rgba(136, 136, 136, 200);
    let mut viewport = CodeViewport {
        rect: Rect::new(10.0, 20.0, 500.0, 240.0),
        body: Rect::new(60.0, 24.0, 442.0, 228.0),
        scroll_x: 70.0,
        scroll_y: 400.0,
        max_x: 500.0,
        max_y: 1500.0,
        ..Default::default()
    };
    let mut idle = Vec::new();
    paint_scrollbars(&mut idle, &viewport, ink);
    assert_eq!(idle.len(), 2, "idle tracks stay transparent");
    let thumbs: Vec<Rect> = idle
        .iter()
        .map(|command| {
            let DrawCommand::FillRect {
                rect,
                color,
                radius,
            } = command
            else {
                panic!("scrollbar ink")
            };
            assert_eq!(*color, appearance.thumb_color(ink, false, false));
            assert_eq!(*radius, appearance.thumb_width / 2.0);
            *rect
        })
        .collect();
    assert_eq!(thumbs[0].width, appearance.thumb_width);
    assert_eq!(thumbs[1].height, appearance.thumb_width);
    let expected_height = (228.0_f32 * 228.0 / (228.0 + 1500.0)).max(16.0);
    assert_eq!(thumbs[0].height, expected_height);
    let thumb_point = Point::new(thumbs[0].x + 2.0, thumbs[0].y + thumbs[0].height / 2.0);
    viewport.set_hovered_point(Some(thumb_point));
    let mut hovered = Vec::new();
    paint_scrollbars(&mut hovered, &viewport, ink);
    assert_eq!(hovered.len(), 3);
    assert!(hovered.iter().any(
        |command| matches!(command, DrawCommand::FillRect {color, rect, ..}
        if *color == appearance.track_color(ink) && rect.width == 8.0)
    ));
    assert!(hovered.iter().any(
        |command| matches!(command, DrawCommand::FillRect {color, rect, ..}
        if *color == appearance.thumb_color(ink, true, false) && *rect == thumbs[0])
    ));
    let scroll_before = viewport.scroll_y;
    assert!(viewport.begin_scrollbar(thumb_point));
    assert!((viewport.scroll_y - scroll_before).abs() < 0.01);
    let mut active = Vec::new();
    paint_scrollbars(&mut active, &viewport, ink);
    assert!(active.iter().any(
        |command| matches!(command, DrawCommand::FillRect {color, ..}
        if *color == appearance.thumb_color(ink, true, true))
    ));
    viewport.drag_scrollbar(Point::new(thumb_point.x, 1000.0), false);
    assert_eq!(viewport.scroll_y, viewport.max_y);
    assert!(viewport.scrollbar_drag.is_none());
}
