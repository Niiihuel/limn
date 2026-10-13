use super::*;
use crate::{HitKind, PopoverPlacement, Size, UiTree};

fn fixture() -> (EditorController, CodeEditor) {
    let text = (0..120)
        .map(|row| match row {
            0 => "fn outer() {".into(),
            3 => "    fn inner() {".into(),
            6 => "        if condition {".into(),
            9 => "            while running {".into(),
            _ => format!("                source_line_{row}();"),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let controller = EditorController::new(TextBuffer::new(&text).unwrap());
    let snapshot = controller.snapshot().unwrap();
    let editor = CodeEditor::new("source", controller.clone())
        .language(CodeLanguage::Rust)
        .structure(
            snapshot.document_id(),
            snapshot.revision(),
            [
                CodeFoldCandidate {
                    start_row: 0,
                    end_row: 110,
                    action_id: "fold:0".into(),
                },
                CodeFoldCandidate {
                    start_row: 3,
                    end_row: 100,
                    action_id: "fold:3".into(),
                },
            ]
            .to_vec(),
            [0, 3, 6, 9]
                .into_iter()
                .map(|row| CodeStickyScope {
                    start_row: row,
                    end_row: 90 - row,
                    action_id: format!("scope:{row}"),
                })
                .collect::<Vec<_>>(),
        )
        .breakpoints(
            snapshot.document_id(),
            snapshot.revision(),
            "breakpoint",
            vec![3],
        );
    (controller, editor)
}
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 8.0
}
fn paint(view: &mut EditorView, width: f32) -> Vec<DrawCommand> {
    let rect = Rect::new(0.0, 0.0, width, 300.0);
    view.paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
}
fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}
fn hit(tree: &UiTree, id: &str) -> Rect {
    tree.frame()
        .regions
        .iter()
        .find(|region| region.id == id && region.kind == HitKind::Button)
        .unwrap_or_else(|| panic!("missing control {id}"))
        .rect
}
fn click(tree: &mut UiTree, id: &str) -> Vec<UiAction> {
    let point = middle(hit(tree, id));
    tree.handle_event(UiEvent::PointerDown(point));
    tree.handle_event(UiEvent::PointerUp(point))
}

#[test]
fn structure_gutter_lanes_are_disjoint_and_keep_numbers_aligned_after_folding_or_stale_analysis() {
    for width in [320.0, 900.0] {
        let (controller, editor) = fixture();
        let mut view = EditorView::new(editor, Instant::now());
        let commands = paint(&mut view, width);
        let body = view.viewport.body;
        let fold = view
            .structure_controls()
            .iter()
            .find(|control| control.action_id == "fold:0")
            .unwrap()
            .rect;
        let breakpoint = view
            .structure_controls()
            .iter()
            .find(|control| control.action_id == "breakpoint:0")
            .unwrap()
            .rect;
        assert_eq!(breakpoint.width, 14.0);
        assert!(breakpoint.x + breakpoint.width <= fold.x);
        assert!(fold.x + fold.width <= body.x);
        let number_x = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "1" => Some(position.x),
                _ => None,
            })
            .unwrap();
        assert!(number_x >= breakpoint.x + breakpoint.width);
        assert!(number_x + measure("1") < fold.x);
        view.editor.folds = vec![0..110].into();
        paint(&mut view, width);
        assert_eq!(
            view.viewport.body, body,
            "folding must not shift source columns"
        );
        assert!(view
            .structure_controls()
            .iter()
            .any(|control| control.action_id == "fold:0"));
        controller
            .execute(EditorCommand::InsertText("// changed\n".into()))
            .unwrap();
        paint(&mut view, width);
        assert_eq!(view.viewport.body.x, body.x);
        assert!(
            view.structure_controls().is_empty(),
            "stale analysis and breakpoint revisions are passive"
        );
    }
}

#[test]
fn sticky_scopes_use_real_source_are_bounded_and_keep_caret_and_find_surface_clear() {
    let (controller, editor) = fixture();
    let mut view = EditorView::new(editor.reveal_top_inset(32.0), Instant::now());
    paint(&mut view, 320.0);
    view.viewport.scroll(0.0, -400.0);
    let commands = paint(&mut view, 320.0);
    let sticky: Vec<_> = view
        .structure_controls()
        .iter()
        .filter(|control| control.action_id.starts_with("scope:"))
        .collect();
    assert_eq!(sticky.len(), 3);
    assert_eq!(sticky[0].rect.y, view.viewport.body.y + 32.0);
    assert_eq!(sticky[2].rect.y + LINE_HEIGHT, view.content_visible().y);
    assert!(commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text {text, ..} if text.contains("outer"))));
    assert!(sticky
        .iter()
        .all(|control| control.rect.height == LINE_HEIGHT));
    let snapshot = controller.snapshot().unwrap();
    let offset = snapshot.line_range(20).unwrap().start;
    controller
        .execute(EditorCommand::SetSelection(Selection::caret(offset)))
        .unwrap();
    view.viewport.reveal_caret = true;
    paint(&mut view, 320.0);
    let (ime_body, caret) = view
        .ime_rects()
        .expect("caret remains visible below fixed headers");
    assert!(caret.y >= view.content_visible().y);
    assert!(ime_body.y >= view.content_visible().y);
    assert!(caret.y + caret.height <= view.viewport.body.y + view.viewport.body.height);
    view.viewport.scroll(0.0, -100_000.0);
    paint(&mut view, 320.0);
    assert!(
        !view
            .structure_controls()
            .iter()
            .any(|control| control.action_id.starts_with("scope:")),
        "completed scopes do not remain pinned"
    );
}

#[test]
fn collapsed_scopes_and_wrapped_continuations_do_not_create_hidden_or_duplicate_controls() {
    let (_, editor) = fixture();
    let mut view = EditorView::new(editor.folds(vec![3..100]), Instant::now());
    paint(&mut view, 320.0);
    view.viewport.scroll(0.0, -120.0);
    paint(&mut view, 320.0);
    assert!(!view
        .structure_controls()
        .iter()
        .any(|control| control.action_id == "scope:6" || control.action_id == "scope:9"));
    let (_, editor) = fixture();
    let mut tree = UiTree::new(editor.soft_wrap(true).render());
    tree.layout(Size::new(180.0, 300.0));
    for id in ["fold:0", "fold:3", "breakpoint:0", "breakpoint:3"] {
        assert_eq!(
            tree.frame()
                .regions
                .iter()
                .filter(|region| region.id == id && region.kind == HitKind::Button)
                .count(),
            1,
            "{id}"
        );
    }
}

#[test]
fn structure_buttons_dispatch_without_editing_and_wheel_respects_popover_above_them() {
    let (controller, editor) = fixture();
    let mut tree = UiTree::new(editor.clone().render());
    tree.set_reduced_motion(true);
    let size = Size::new(320.0, 300.0);
    tree.layout(size);
    let before = controller.snapshot().unwrap().selection();
    assert_eq!(
        click(&mut tree, "fold:0"),
        vec![UiAction::Clicked {
            id: "fold:0".into()
        }]
    );
    assert_eq!(controller.snapshot().unwrap().selection(), before);
    assert_eq!(
        click(&mut tree, "breakpoint:3"),
        vec![UiAction::Clicked {
            id: "breakpoint:3".into()
        }]
    );
    let gutter = middle(hit(&tree, "breakpoint:3"));
    tree.handle_event(UiEvent::ScrollDirect {
        position: gutter,
        delta_y: -400.0,
    });
    tree.layout(size);
    let sticky = middle(hit(&tree, "scope:0"));
    assert_eq!(
        click(&mut tree, "scope:0"),
        vec![UiAction::Clicked {
            id: "scope:0".into()
        }]
    );
    let first_row = |tree: &UiTree| {
        tree.frame()
            .regions
            .iter()
            .filter_map(|region| region.id.strip_prefix("breakpoint:")?.parse::<usize>().ok())
            .min()
            .unwrap()
    };
    let before_row = first_row(&tree);
    tree.handle_event(UiEvent::ScrollDirect {
        position: sticky,
        delta_y: -80.0,
    });
    tree.layout(size);
    assert!(
        first_row(&tree) > before_row,
        "wheel over sticky header scrolls this editor"
    );
    let fade = Color::rgb(25, 25, 25);
    let menu = Element::popover(
        Element::button("menu_anchor", "Menu").size(Some(120.0), Some(20.0)),
        Element::button("menu_item", "Overlay owns wheel").size(Some(240.0), Some(150.0)),
        "dismiss",
        PopoverPlacement::BottomStart,
    );
    tree.set_root(Element::overlay(editor.render().top_fade(fade, 12.0), menu));
    tree.layout(size);
    let before_row = first_row(&tree);
    let menu_point = middle(hit(&tree, "menu_item"));
    tree.handle_event(UiEvent::ScrollDirect {
        position: menu_point,
        delta_y: -100.0,
    });
    tree.layout(size);
    assert_eq!(
        first_row(&tree),
        before_row,
        "an overlapping menu cannot scroll the editor below it"
    );
    let fixed_bottom = tree
        .frame()
        .regions
        .iter()
        .filter(|region| region.id.starts_with("scope:") && region.kind == HitKind::Button)
        .map(|region| region.rect.y + region.rect.height)
        .fold(0.0, f32::max);
    assert!(fixed_bottom > 0.0);
    assert!(
        tree.frame().commands.iter().any(
            |command| matches!(command, DrawCommand::FillVerticalGradient {rect,top,..}
        if *top == fade && rect.y >= fixed_bottom)
        ),
        "fade begins below sticky text and controls"
    );
}

#[test]
fn structure_cache_bounds_inputs_and_rejects_foreign_identity_or_invalid_ranges() {
    let (_, mut editor) = fixture();
    let snapshot = editor.controller.snapshot().unwrap();
    editor.fold_candidates = vec![CodeFoldCandidate {
        start_row: 0,
        end_row: usize::MAX,
        action_id: "invalid".into(),
    }]
    .into();
    editor.sticky_scopes = (0..20_000)
        .map(|row| CodeStickyScope {
            start_row: row,
            end_row: row + 2,
            action_id: format!("scope:{row}"),
        })
        .collect::<Vec<_>>()
        .into();
    let mut cache = structure::StructureCache::default();
    cache.update(&editor, &snapshot);
    assert!(cache.fold_at(0).is_none());
    assert!(cache.scopes_at(0, usize::MAX).len() <= snapshot.line_count());
    editor.structure_document_id = Some(snapshot.document_id().wrapping_add(1));
    cache.update(&editor, &snapshot);
    assert!(cache.scopes_at(0, usize::MAX).is_empty());
}

#[test]
fn symbol_and_block_on_the_same_source_line_share_one_sticky_header() {
    let (_, mut editor) = fixture();
    let mut scopes = editor.sticky_scopes.to_vec();
    scopes.push(CodeStickyScope {
        start_row: 0,
        end_row: 89,
        action_id: "scope:same_line_block".into(),
    });
    editor.sticky_scopes = scopes.into();
    let mut view = EditorView::new(editor, Instant::now());
    paint(&mut view, 900.0);
    view.viewport.scroll(0.0, -400.0);
    paint(&mut view, 900.0);
    let headers: Vec<_> = view
        .structure_controls()
        .iter()
        .filter(|control| control.action_id.starts_with("scope:"))
        .collect();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0].action_id, "scope:0");
    assert_eq!(headers[1].action_id, "scope:3");
    assert_eq!(headers[2].action_id, "scope:6");
}

#[test]
fn floating_find_inset_preserves_source_above_sticky_without_exposing_hidden_controls() {
    let (_, editor) = fixture();
    let mut view = EditorView::new(editor.reveal_top_inset(60.0), Instant::now());
    paint(&mut view, 900.0);
    view.viewport.scroll(0.0, -400.0);
    let commands = paint(&mut view, 900.0);
    let header = view
        .structure_controls()
        .iter()
        .find(|control| control.action_id == "scope:0")
        .unwrap();
    assert_eq!(header.rect.y, view.viewport.body.y + 60.0);
    assert!(commands.iter().any(|command| matches!(command,
        DrawCommand::Text {text,position,..} if text.contains("source_line_20") && position.y < header.rect.y)),
        "the floating find panel does not erase source on its left");
    for control in view
        .structure_controls()
        .iter()
        .filter(|control| !control.action_id.starts_with("scope:"))
    {
        let visible = control.rect.intersection(control.clip).unwrap();
        assert!(
            visible.y + visible.height <= header.rect.y || visible.y >= view.content_visible().y
        );
    }
}

#[test]
fn debugger_acknowledgements_are_distinct_and_update_without_editing_the_document() {
    let (controller, editor) = fixture();
    let snapshot = controller.snapshot().unwrap();
    let markers = vec![
        CodeBreakpoint {
            row: 0,
            state: CodeBreakpointState::Pending,
            message: None,
        },
        CodeBreakpoint {
            row: 1,
            state: CodeBreakpointState::Verified,
            message: Some("Adaptador confirmó la línea".into()),
        },
        CodeBreakpoint {
            row: 2,
            state: CodeBreakpointState::Rejected,
            message: Some("No hay código ejecutable en esta línea".into()),
        },
        CodeBreakpoint {
            row: 3,
            state: CodeBreakpointState::Disabled,
            message: None,
        },
    ];
    let mut editor = editor.debugger(
        snapshot.document_id(),
        snapshot.revision(),
        markers.clone(),
        None,
    );
    let mut tree = UiTree::new(editor.clone().render());
    tree.set_reduced_motion(true);
    let now = Instant::now();
    tree.layout_at(Size::new(360.0, 300.0), now);
    let verified = hit(&tree, "breakpoint:1");
    assert!(tree.frame().commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect {rect,color,radius} if color.r == 206 && *radius == 4.0
            && rect.y >= verified.y && rect.y + rect.height <= verified.y + verified.height)));
    assert!(tree.frame().commands.iter().any(|command| matches!(
        command,
        DrawCommand::Icon {
            icon: crate::Icon::WarningTriangle,
            ..
        }
    )));
    tree.handle_event_at(
        UiEvent::PointerMove(middle(hit(&tree, "breakpoint:2"))),
        now,
    );
    let frame = tree.layout_at(
        Size::new(360.0, 300.0),
        now + std::time::Duration::from_secs(1),
    );
    assert!(frame.commands.iter().any(
        |command| matches!(command, DrawCommand::Text {text,..} if text.contains("rejected"))
    ));
    assert!(frame.commands.iter().any(|command| matches!(command, DrawCommand::Text {text,..} if text.contains("código ejecutable"))));
    let mut changed = markers;
    changed[1].state = CodeBreakpointState::Disabled;
    editor.debugger_breakpoints = changed.into();
    tree.set_root(editor.render());
    tree.handle_event_at(
        UiEvent::PointerMove(Point::new(350.0, 290.0)),
        now + std::time::Duration::from_secs(2),
    );
    tree.layout_at(
        Size::new(360.0, 300.0),
        now + std::time::Duration::from_secs(2),
    );
    assert!(
        !tree.frame().commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect {rect,color,radius} if color.r == 206 && *radius == 4.0
            && rect.y >= verified.y && rect.y + rect.height <= verified.y + verified.height))
    );
    let after = controller.snapshot().unwrap();
    assert_eq!(after.revision(), snapshot.revision());
    assert_eq!(after.selection(), snapshot.selection());
    assert_eq!(
        after.text_range(0..after.char_count()).unwrap(),
        snapshot.text_range(0..snapshot.char_count()).unwrap()
    );
}

#[test]
fn debugger_execution_is_passive_revision_scoped_and_keeps_gutter_geometry_stable() {
    let (controller, editor) = fixture();
    let snapshot = controller.snapshot().unwrap();
    let mut view = EditorView::new(editor.clone(), Instant::now());
    paint(&mut view, 360.0);
    let body = view.viewport.body;
    view.update(editor.debugger(
        snapshot.document_id(),
        snapshot.revision(),
        Vec::<CodeBreakpoint>::new(),
        Some(CodeExecutionLine {
            row: 2,
            primary: true,
        }),
    ));
    let commands = paint(&mut view, 360.0);
    assert_eq!(view.viewport.body, body);
    assert!(commands.iter().any(
        |command| matches!(command, DrawCommand::FillRect {rect,color,..}
        if color.r == 232 && color.a == 38 && rect.y == body.y + 2.0 * LINE_HEIGHT)
    ));
    assert_eq!(
        commands
            .iter()
            .filter(
                |command| matches!(command, DrawCommand::FillTriangle {color,..} if color.r == 232)
            )
            .count(),
        1
    );
    assert_eq!(
        controller.snapshot().unwrap().selection(),
        snapshot.selection()
    );
    controller
        .execute(EditorCommand::InsertText("changed".into()))
        .unwrap();
    let commands = paint(&mut view, 360.0);
    assert!(!commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillRect {color,..} if color.r == 232)));
    let replacement =
        EditorController::new(TextBuffer::new("same revision, different document").unwrap());
    view.editor.controller = replacement;
    let commands = paint(&mut view, 360.0);
    assert!(!commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillRect {color,..} if color.r == 232)));
}

fn breakpoint_previews(frame: &crate::Frame) -> Vec<Rect> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect {
                rect,
                color,
                radius,
            } if color.r == 206 && color.g == 113 && color.a == 90 && *radius == 4.0 => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn empty_breakpoint_hover_previews_a_centered_point_without_shifting_source_or_editing() {
    let (controller, mut editor) = fixture();
    let snapshot = controller.snapshot().unwrap();
    let root = |editor: CodeEditor| {
        Element::row(vec![
            Element::spacer().size(Some(70.0), Some(300.0)),
            editor.render().size(Some(320.0), Some(300.0)),
        ])
    };
    let mut tree = UiTree::new(root(editor.clone()));
    tree.set_reduced_motion(true);
    let size = Size::new(390.0, 300.0);
    tree.layout(size);
    let lane = hit(&tree, "breakpoint:0");
    assert_eq!((lane.x, lane.width), (70.0, structure::BREAKPOINT_LANE));
    assert!(breakpoint_previews(tree.frame()).is_empty());
    let number = |frame: &crate::Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "1" => Some(*position),
                _ => None,
            })
            .unwrap()
    };
    let original_number = number(tree.frame());
    tree.handle_event(UiEvent::PointerMove(middle(lane)));
    tree.layout(size);
    let previews = breakpoint_previews(tree.frame());
    assert_eq!(previews.len(), 1);
    assert_eq!(middle(previews[0]), middle(lane));
    assert!(
        previews[0].x >= lane.x + 3.0
            && previews[0].x + previews[0].width <= lane.x + lane.width - 3.0
    );
    assert_eq!(number(tree.frame()), original_number);
    assert_eq!(
        click(&mut tree, "breakpoint:0"),
        vec![UiAction::Clicked {
            id: "breakpoint:0".into()
        }]
    );
    editor.breakpoint_rows = vec![0, 3].into();
    tree.set_root(root(editor.clone()));
    tree.layout(size);
    assert!(
        breakpoint_previews(tree.frame()).is_empty(),
        "a pending outline must replace the preview"
    );
    assert!(tree.frame().commands.iter().any(|command| matches!(command,
        DrawCommand::StrokeRect {rect,color,radius,..} if color.r == 206 && *radius == 4.0 && lane.contains(middle(*rect)))));
    assert_eq!(number(tree.frame()), original_number);
    assert_eq!(
        click(&mut tree, "breakpoint:0"),
        vec![UiAction::Clicked {
            id: "breakpoint:0".into()
        }]
    );
    editor.breakpoint_rows = vec![3].into();
    tree.set_root(root(editor));
    tree.handle_event(UiEvent::PointerMove(Point::new(350.0, 285.0)));
    tree.layout(size);
    assert!(breakpoint_previews(tree.frame()).is_empty());
    let after = controller.snapshot().unwrap();
    assert_eq!(
        (after.revision(), after.selection()),
        (snapshot.revision(), snapshot.selection())
    );
}

#[test]
fn breakpoint_preview_follows_visible_rows_and_is_hidden_under_an_overlay() {
    let (_, editor) = fixture();
    let mut tree = UiTree::new(editor.clone().render());
    tree.set_reduced_motion(true);
    let size = Size::new(320.0, 300.0);
    tree.layout(size);
    let point = middle(hit(&tree, "breakpoint:5"));
    tree.handle_event(UiEvent::PointerMove(point));
    tree.layout(size);
    assert_eq!(breakpoint_previews(tree.frame()).len(), 1);
    tree.handle_event(UiEvent::ScrollDirect {
        position: point,
        delta_y: -2.0 * LINE_HEIGHT,
    });
    tree.layout(size);
    let previews = breakpoint_previews(tree.frame());
    assert_eq!(previews.len(), 1);
    let visible = tree
        .frame()
        .regions
        .iter()
        .find(|region| region.id.starts_with("breakpoint:") && region.rect.contains(point))
        .unwrap();
    assert_ne!(visible.id, "breakpoint:5");
    assert!(visible.rect.contains(middle(previews[0])));
    tree.set_root(Element::overlay(
        editor.render(),
        Element::button("overlay", "Overlay").size(Some(160.0), Some(180.0)),
    ));
    tree.layout(size);
    tree.handle_event(UiEvent::PointerMove(point));
    tree.layout(size);
    assert!(breakpoint_previews(tree.frame()).is_empty());
}

#[test]
fn breakpoint_and_execution_lanes_are_disjoint_and_omitting_debug_keeps_folding_only() {
    let (controller, editor) = fixture();
    let snapshot = controller.snapshot().unwrap();
    let mut view = EditorView::new(
        editor.clone().debugger(
            snapshot.document_id(),
            snapshot.revision(),
            vec![CodeBreakpoint {
                row: 2,
                state: CodeBreakpointState::Verified,
                message: None,
            }],
            Some(CodeExecutionLine {
                row: 2,
                primary: true,
            }),
        ),
        Instant::now(),
    );
    let commands = paint(&mut view, 320.0);
    let body_x = view.viewport.body.x;
    let lane = view
        .structure_controls()
        .iter()
        .find(|control| control.action_id == "breakpoint:2")
        .unwrap()
        .rect;
    let arrow = commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillTriangle { points, color } if color.r == 232 => Some(points),
            _ => None,
        })
        .unwrap();
    assert!(arrow
        .iter()
        .all(|point| point.x > lane.x + lane.width && point.x < body_x));
    let mut ordinary = editor;
    ordinary.breakpoint_action_prefix = None;
    ordinary.breakpoint_document_id = None;
    ordinary.breakpoint_revision = None;
    view.update(ordinary);
    view.set_hovered_control(Some("breakpoint:2"), Some(middle(lane)));
    paint(&mut view, 320.0);
    assert_eq!(view.viewport.body.x, body_x - structure::BREAKPOINT_LANE);
    assert!(view
        .structure_controls()
        .iter()
        .all(|control| !control.action_id.starts_with("breakpoint:")));
    assert!(view
        .structure_controls()
        .iter()
        .any(|control| control.action_id == "fold:0"));
}
