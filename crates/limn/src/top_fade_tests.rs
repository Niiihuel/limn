use super::*;
use crate::{CodeDiff, CodeDiffLine, CodeDiffLineKind, CodeEditor, Component, EditorController};
use limn_editor::TextBuffer;

fn fades(frame: &Frame) -> Vec<Rect> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillVerticalGradient { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect()
}

#[test]
fn scroll_top_fade_requires_hidden_content_and_leaves_headers_and_scrollbars_clear() {
    for virtualized in [false, true] {
        let body = if virtualized {
            Element::uniform_list("rows", 20, 36.0, |index| {
                Element::button(format!("row:{index}"), format!("Row {index}"))
                    .size(None, Some(36.0))
            })
        } else {
            Element::scroll(
                "rows",
                Element::column(
                    (0..20)
                        .map(|index| {
                            Element::button(format!("row:{index}"), format!("Row {index}"))
                                .size(None, Some(36.0))
                        })
                        .collect(),
                ),
            )
        };
        let root = |body| {
            Element::column(vec![
                Element::button("header", "Header").size(None, Some(32.0)),
                body,
                Element::button("footer", "Footer").size(None, Some(32.0)),
            ])
        };
        let mut plain = UiTree::new(root(body.clone().flex(1.0)));
        let mut tree = UiTree::new(root(body.top_fade(Color::rgb(20, 20, 20), 12.0).flex(1.0)));
        plain.set_reduced_motion(true);
        tree.set_reduced_motion(true);
        let size = Size::new(220.0, 180.0);
        let initial = tree.layout(size);
        assert!(
            fades(&initial).is_empty(),
            "the first row is fully visible at the top"
        );
        assert_eq!(initial.regions, plain.layout(size).regions);
        let point = Point::new(30.0, 75.0);
        for surface in [&mut plain, &mut tree] {
            surface.handle_event(UiEvent::ScrollDirect {
                position: point,
                delta_y: -18.0,
            });
        }
        let scrolled = tree.layout(size);
        assert_eq!(scrolled.regions, plain.layout(size).regions);
        let gradients = fades(&scrolled);
        assert_eq!(gradients.len(), 1);
        let fade = gradients[0];
        assert_eq!(fade.y, 32.0);
        assert_eq!(fade.height, 12.0);
        assert!(fade.x + fade.width <= scrolled.scrollbars["rows"].rail.x);
        for id in ["header", "footer"] {
            let region = scrolled
                .regions
                .iter()
                .find(|region| region.id == id)
                .unwrap();
            assert!(region.rect.intersection(fade).is_none());
            assert!(tree.focus(id));
            assert_eq!(
                tree.handle_event(UiEvent::Key {
                    key: Key::Enter,
                    shift: false,
                    ctrl: false
                }),
                vec![UiAction::Clicked { id: id.into() }]
            );
        }
        tree.handle_event(UiEvent::ScrollDirect {
            position: point,
            delta_y: 100_000.0,
        });
        assert!(fades(&tree.layout(size)).is_empty());
    }
}

#[test]
fn scroll_bottom_fade_overlays_only_hidden_content_without_intercepting_input() {
    for virtualized in [false, true] {
        let body = if virtualized {
            Element::uniform_list("rows", 20, 36.0, |index| {
                Element::button(format!("row:{index}"), format!("Row {index}"))
                    .size(None, Some(36.0))
            })
        } else {
            Element::scroll(
                "rows",
                Element::column(
                    (0..20)
                        .map(|index| {
                            Element::button(format!("row:{index}"), format!("Row {index}"))
                                .size(None, Some(36.0))
                        })
                        .collect(),
                ),
            )
        };
        let root = |body| {
            Element::column(vec![
                Element::button("header", "Header").size(None, Some(32.0)),
                body,
                Element::button("composer", "Send").size(None, Some(32.0)),
            ])
        };
        let mut plain = UiTree::new(root(body.clone().flex(1.0)));
        let mut tree = UiTree::new(root(
            body.bottom_fade(Color::rgb(20, 20, 20), 12.0).flex(1.0),
        ));
        plain.set_reduced_motion(true);
        tree.set_reduced_motion(true);
        let size = Size::new(220.0, 180.0);
        let initial = tree.layout(size);
        assert_eq!(initial.regions, plain.layout(size).regions);
        let gradients = fades(&initial);
        assert_eq!(gradients.len(), 1);
        let fade = gradients[0];
        assert_eq!((fade.y, fade.height), (136.0, 12.0));
        assert!(fade.x + fade.width <= initial.scrollbars["rows"].rail.x);
        let gradient_index = initial
            .commands
            .iter()
            .position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. }))
            .unwrap();
        assert!(
            initial.commands[..gradient_index]
                .iter()
                .any(|command| matches!(
                    command, DrawCommand::Text { text, .. } if text.starts_with("Row ")
                )),
            "the scrim is painted above, not behind, the scrolling content"
        );
        for id in ["header", "composer"] {
            let region = initial
                .regions
                .iter()
                .find(|region| region.id == id)
                .unwrap();
            assert!(region.rect.intersection(fade).is_none());
        }
        // Even the portion of a row inside the scrim keeps its original target.
        for (id, point) in [
            ("row:3", Point::new(20.0, 144.0)),
            ("composer", Point::new(20.0, 160.0)),
        ] {
            assert_eq!(initial.hit_test(point), Some(id));
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: id.into() }]
            );
        }
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(30.0, 75.0),
            delta_y: -100_000.0,
        });
        let end = tree.layout(size);
        assert!(
            fades(&end).is_empty(),
            "the final row is never faded at the end"
        );
        let final_row = end
            .regions
            .iter()
            .find(|region| region.id == "row:19")
            .unwrap();
        assert_eq!(final_row.rect.height, 36.0);
        assert_eq!(final_row.rect.y + final_row.rect.height, 148.0);
        assert!(tree.focus("row:19"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::Clicked {
                id: "row:19".into()
            }]
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(30.0, 75.0),
            delta_y: 100_000.0,
        });
        assert_eq!(fades(&tree.layout(size)).len(), 1);
        // A resize that fits all rows removes the scrim without a stale extent.
        assert!(fades(&tree.layout(Size::new(220.0, 900.0))).is_empty());
    }
}

#[test]
fn bottom_fade_respects_a_clipped_ancestor_and_has_no_hit_region() {
    let body = Element::scroll(
        "rows",
        Element::column(vec![
            Element::button("first", "First").size(None, Some(40.0)),
            Element::button("last", "Last").size(None, Some(200.0)),
        ]),
    )
    .size(None, Some(160.0));
    let root = |body| {
        Element::column(vec![body])
            .size(None, Some(80.0))
            .clip_children()
    };
    let plain = UiTree::new(root(body.clone())).layout(Size::new(220.0, 80.0));
    let frame = UiTree::new(root(body.bottom_fade(Color::rgb(20, 20, 20), 16.0)))
        .layout(Size::new(220.0, 80.0));
    assert!(
        fades(&frame).is_empty(),
        "a viewport edge outside the ancestor clip must not paint"
    );
    assert_eq!(frame.regions, plain.regions);
}

#[test]
fn editor_and_diff_top_fades_stay_inside_the_scrolling_code_body() {
    for diff in [false, true] {
        let code = if diff {
            CodeDiff::new(
                "code",
                (0..100)
                    .map(|index| CodeDiffLine {
                        kind: CodeDiffLineKind::Context,
                        old_line: Some(index + 1),
                        new_line: Some(index + 1),
                        text: format!("line {index}"),
                    })
                    .collect::<Vec<_>>(),
            )
            .render()
            .size(None, None)
        } else {
            CodeEditor::new(
                "code",
                EditorController::new(TextBuffer::new(&"line\n".repeat(100)).unwrap()),
            )
            .status_bar(true)
            .render()
        };
        let mut tree = UiTree::new(Element::column(vec![
            Element::button("header", "Document").size(None, Some(32.0)),
            code.top_fade(Color::rgb(20, 20, 20), 12.0).flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        let size = Size::new(320.0, 240.0);
        assert!(fades(&tree.layout(size)).is_empty());
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(180.0, 120.0),
            delta_y: -60.0,
        });
        let frame = tree.layout(size);
        let viewport = if diff {
            &tree.code_diffs["code"].viewport
        } else {
            &tree.code_editors["code"].viewport
        };
        assert!(viewport.scroll_y > 0.0);
        let gradients = fades(&frame);
        assert_eq!(gradients.len(), 1);
        assert_eq!(
            gradients[0],
            Rect::new(viewport.body.x, viewport.body.y, viewport.body.width, 12.0)
        );
        assert!(gradients[0].y >= 32.0);
        assert!(
            gradients[0].x > viewport.rect.x,
            "folding controls and the gutter stay clear"
        );
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(180.0, 120.0),
            delta_y: 100_000.0,
        });
        assert!(fades(&tree.layout(size)).is_empty());
    }
}
