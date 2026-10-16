//! Opt-in vertical shrinking leaves ordinary columns and row widths unchanged.

use limn::{
    CodeEditor, Component, EditorController, Element, Frame, HitKind, Point, Rect, ScrollArea,
    Size, Style, UiTree,
};
use limn_editor::TextBuffer;

fn block(id: &str, height: f32) -> Element {
    Element::button(id, id).size(None, Some(height))
}

fn rect(frame: &Frame, id: &str) -> Rect {
    frame
        .regions
        .iter()
        .find(|region| region.id == id)
        .unwrap_or_else(|| panic!("missing {id}"))
        .rect
}

fn tree(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree
}

fn viewport(shrink: bool) -> Element {
    let notice = block("notice", 120.0);
    Element::column(vec![
        block("body", 0.0).min_size(0.0, 20.0).flex(1.0),
        if shrink {
            notice.shrink_height_to_fit()
        } else {
            notice
        },
        block("footer", 40.0),
    ])
    .gap(8.0)
}

#[test]
fn column_shrink_is_independent_of_width_shrinking_and_is_opt_in() {
    assert!(!Style::default().shrink_height);
    let vertical = block("notice", 120.0).shrink_height_to_fit();
    assert!(vertical.style.shrink_height && !vertical.style.shrink);
    let horizontal = block("notice", 120.0).shrink_to_fit();
    assert!(horizontal.style.shrink && !horizontal.style.shrink_height);
    let mut ordinary = tree(Element::column(vec![
        block("notice", 120.0),
        block("footer", 40.0),
    ]));
    let mut width_only = tree(Element::column(vec![horizontal, block("footer", 40.0)]));
    let window = Size::new(240.0, 100.0);
    assert_eq!(ordinary.layout(window), width_only.layout(window));
    let mut row = tree(Element::row(vec![
        block("notice", 120.0),
        block("footer", 40.0),
    ]));
    let mut flagged_row = tree(Element::row(vec![vertical, block("footer", 40.0)]));
    assert_eq!(row.layout(window), flagged_row.layout(window));
}

#[test]
fn column_shrink_keeps_its_natural_height_when_there_is_room() {
    let frame = tree(viewport(true)).layout(Size::new(240.0, 260.0));
    assert_eq!(rect(&frame, "notice").height, 120.0);
    assert_eq!(rect(&frame, "body").height, 84.0);
    assert_eq!(rect(&frame, "footer"), Rect::new(0.0, 220.0, 240.0, 40.0));
}

#[test]
fn column_shrink_reserves_the_fixed_footer_gaps_and_flexible_minimum() {
    let frame = tree(viewport(true)).layout(Size::new(240.0, 160.0));
    assert_eq!(rect(&frame, "notice").height, 84.0);
    assert_eq!(rect(&frame, "body").height, 20.0);
    assert_eq!(rect(&frame, "footer"), Rect::new(0.0, 120.0, 240.0, 40.0));
}

#[test]
fn column_shrink_uses_ordered_minimum_then_zero_passes_for_multiple_children() {
    for (height, first, second) in [
        (200.0, 80.0, 70.0),
        (150.0, 50.0, 50.0),
        (120.0, 30.0, 40.0),
        (60.0, 0.0, 10.0),
    ] {
        let frame = tree(
            Element::column(vec![
                block("first", 80.0)
                    .min_size(0.0, 50.0)
                    .shrink_height_to_fit(),
                block("second", 70.0)
                    .min_size(0.0, 40.0)
                    .shrink_height_to_fit(),
                block("footer", 30.0),
            ])
            .gap(10.0),
        )
        .layout(Size::new(240.0, height));
        assert_eq!(rect(&frame, "first").height, first, "viewport {height}");
        assert_eq!(rect(&frame, "second").height, second, "viewport {height}");
        let footer = rect(&frame, "footer");
        assert!(
            footer.y + footer.height <= height,
            "{footer:?} escapes {height}"
        );
    }
}

#[test]
fn column_shrink_preserves_each_flexible_minimum_instead_of_only_their_sum() {
    let frame = tree(
        Element::column(vec![
            block("small", 0.0).min_size(0.0, 10.0).flex(1.0),
            block("large", 0.0).min_size(0.0, 70.0).flex(1.0),
            block("notice", 120.0).shrink_height_to_fit(),
            block("footer", 40.0),
        ])
        .gap(10.0),
    )
    .layout(Size::new(240.0, 180.0));
    assert_eq!(rect(&frame, "notice").height, 30.0);
    assert_eq!(rect(&frame, "small").height, 10.0);
    assert_eq!(rect(&frame, "large").height, 70.0);
    assert_eq!(rect(&frame, "footer").y + 40.0, 180.0);
}

#[test]
fn column_shrink_keeps_weighted_flex_shares_until_a_minimum_actually_binds() {
    for (height, small, large) in [
        (390.0, 100.0, 100.0),
        (290.0, 30.0, 70.0),
        (270.0, 10.0, 70.0),
    ] {
        let frame = tree(
            Element::column(vec![
                block("small", 0.0).min_size(0.0, 10.0).flex(1.0),
                block("large", 0.0).min_size(0.0, 70.0).flex(1.0),
                block("notice", 120.0).shrink_height_to_fit(),
                block("footer", 40.0),
            ])
            .gap(10.0),
        )
        .layout(Size::new(240.0, height));
        assert_eq!(rect(&frame, "notice").height, 120.0);
        assert_eq!(rect(&frame, "small").height, small, "viewport {height}");
        assert_eq!(rect(&frame, "large").height, large, "viewport {height}");
        assert_eq!(rect(&frame, "footer").y + 40.0, height);
    }
}

#[test]
fn column_shrink_waterfill_does_not_overconstrain_a_later_flexible_child() {
    let frame = tree(
        Element::column(vec![
            block("large", 0.0).min_size(0.0, 70.0).flex(1.0),
            block("small", 0.0).min_size(0.0, 20.0).flex(1.0),
            block("notice", 120.0).shrink_height_to_fit(),
            block("footer", 40.0),
        ])
        .gap(10.0),
    )
    .layout(Size::new(240.0, 290.0));
    assert_eq!(rect(&frame, "large").height, 70.0);
    assert_eq!(rect(&frame, "small").height, 30.0);
    assert_eq!(rect(&frame, "footer").y + 40.0, 290.0);
}

#[test]
fn column_shrink_does_not_treat_a_flexible_flagged_child_as_fixed_content() {
    let frame = tree(
        Element::column(vec![
            block("body", 500.0)
                .min_size(0.0, 20.0)
                .flex(1.0)
                .shrink_height_to_fit(),
            block("notice", 120.0).shrink_height_to_fit(),
            block("footer", 40.0),
        ])
        .gap(8.0),
    )
    .layout(Size::new(240.0, 160.0));
    assert_eq!(rect(&frame, "body").height, 20.0);
    assert_eq!(rect(&frame, "notice").height, 84.0);
    assert_eq!(rect(&frame, "footer").y + 40.0, 160.0);
}

#[test]
fn column_shrink_measures_wrapped_content_at_its_real_width_and_keeps_the_end_reachable() {
    for width in [300.0, 180.0] {
        let notice = Element::row(vec![
            Element::spacer().flex(1.0),
            ScrollArea::new(
                "notices",
                Element::column(vec![
                    Element::text("Readable wrapped content. ".repeat(40)),
                    block("last", 24.0),
                ]),
            )
            .render()
            .size(Some((width - 20.0_f32).min(250.0)), None)
            .max_height(140.0),
        ])
        .shrink_height_to_fit();
        let mut tree = tree(
            Element::column(vec![
                block("body", 0.0).min_size(0.0, 10.0).flex(1.0),
                notice,
                block("footer", 40.0),
            ])
            .gap(8.0),
        );
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 6.0);
        let window = Size::new(width, 180.0);
        let frame = tree.layout(window);
        let scroll = frame
            .regions
            .iter()
            .find(|region| region.id == "notices" && region.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        assert_eq!(scroll.height, 114.0);
        assert_eq!(rect(&frame, "footer").y + 40.0, 180.0);
        tree.scroll_to_end("notices");
        let frame = tree.layout(window);
        let last = rect(&frame, "last");
        assert!(last.y >= scroll.y && last.y + last.height <= scroll.y + scroll.height + 0.01);
        assert_eq!(
            frame.hit_test(Point::new(
                last.x + last.width / 2.0,
                last.y + last.height / 2.0
            )),
            Some("last")
        );
    }
}

#[test]
fn column_shrink_honors_a_nested_finite_panel_and_its_padding() {
    let frame = tree(Element::column(vec![
        block("header", 20.0),
        Element::panel(
            Element::column(vec![
                block("notice", 120.0).shrink_height_to_fit(),
                block("footer", 40.0),
            ])
            .gap(8.0),
        )
        .padding(10.0)
        .flex(1.0),
    ]))
    .layout(Size::new(240.0, 140.0));
    assert_eq!(rect(&frame, "notice").height, 52.0);
    assert_eq!(rect(&frame, "footer"), Rect::new(10.0, 90.0, 220.0, 40.0));
}

#[test]
fn column_shrink_leaves_an_unbounded_scroll_content_at_its_natural_height() {
    let mut tree = tree(
        ScrollArea::new(
            "outer",
            Element::column(vec![
                block("notice", 120.0).shrink_height_to_fit(),
                block("footer", 40.0),
            ])
            .gap(8.0),
        )
        .render(),
    );
    let window = Size::new(240.0, 80.0);
    tree.layout(window);
    tree.scroll_to_end("outer");
    let frame = tree.layout(window);
    assert_eq!(tree.scroll_offset("outer"), 88.0);
    assert_eq!(rect(&frame, "footer").y + 40.0, 80.0);
}

#[test]
fn column_shrink_toggling_the_flag_invalidates_the_previous_layout_measurement() {
    let mut tree = tree(viewport(false));
    let window = Size::new(240.0, 160.0);
    assert_eq!(rect(&tree.layout(window), "notice").height, 120.0);
    tree.set_root(viewport(true));
    assert_eq!(rect(&tree.layout(window), "notice").height, 84.0);
    tree.set_root(viewport(false));
    assert_eq!(rect(&tree.layout(window), "notice").height, 120.0);
}

#[test]
fn column_shrink_bounds_an_editor_with_no_intrinsic_height_to_the_finite_column() {
    let editor = CodeEditor::new(
        "editor",
        EditorController::new(TextBuffer::new("source").unwrap()),
    )
    .render()
    .size(None, None)
    .shrink_height_to_fit();
    let frame =
        tree(Element::column(vec![editor, block("footer", 40.0)])).layout(Size::new(240.0, 160.0));
    assert_eq!(rect(&frame, "editor").height, 120.0);
    assert_eq!(rect(&frame, "footer"), Rect::new(0.0, 120.0, 240.0, 40.0));
}

#[test]
fn column_shrink_keeps_allocation_finite_after_an_unbounded_editor() {
    let editor = CodeEditor::new(
        "editor",
        EditorController::new(TextBuffer::new("source").unwrap()),
    )
    .render()
    .size(None, None)
    .shrink_height_to_fit();
    let frame = tree(Element::column(vec![
        editor,
        block("notice", 120.0).shrink_height_to_fit(),
        block("footer", 40.0),
    ]))
    .layout(Size::new(240.0, 160.0));
    assert_eq!(rect(&frame, "editor").height, 0.0);
    assert_eq!(rect(&frame, "notice").height, 120.0);
    assert_eq!(rect(&frame, "footer"), Rect::new(0.0, 120.0, 240.0, 40.0));
}
