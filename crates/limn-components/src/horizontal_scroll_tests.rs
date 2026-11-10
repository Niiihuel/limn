//! The horizontal offsets kept for code viewports. This test exercises the
//! engine's retained horizontal scroll state through `CodeBlock` and
//! `Markdown`.
use crate::*;

#[test]
fn rebuilding_anonymous_code_viewports_bounds_state_and_preserves_explicit_ids() {
    use crate::{CodeBlock, Component, ComponentPalette, Markdown};
    let palette = ComponentPalette::neutral_dark();
    let source = "```\nlong code line with enough text to scroll horizontally\n```\n\n```\nlong code line with enough text to scroll horizontally\n```";
    let root = || Element::column(vec![
        CodeBlock::new("long code line with enough text to scroll horizontally", palette)
            .id("stable-code").word_wrap(false).render(),
        CodeBlock::new("anonymous first block", palette).word_wrap(false).render(),
        CodeBlock::new("anonymous second block", palette).word_wrap(false).render(),
        Markdown::new(source, palette).width(140.0).code_word_wrap(false).render().element,
        Markdown::new(source, palette).id("stable-message").width(140.0)
            .code_word_wrap(false).render().element,
    ]);
    let mut tree = UiTree::new(Element::column(vec![
        Element::horizontal_scroll_2d("unrelated", Element::spacer().size(Some(1000.0), Some(20.0))),
        root(),
    ]));
    let size = Size::new(140.0, 900.0);
    let frame = tree.layout(size);
    let region = frame.regions.iter().find(|region| region.id == "stable-code").unwrap().rect;
    tree.handle_event(UiEvent::Scroll2D {
        position: Point::new(region.x + 10.0, region.y + 10.0),
        delta_x: -40.0, delta_y: 0.0, shift: false, direct: true,
    });
    // Both offsets are kept already: writing them adds no entry.
    let kept = tree.horizontal_scroll_count();
    tree.set_horizontal_scroll_offset("unrelated", 80.0);
    let stable_message = "markdown:stable-message:code:0";
    tree.set_horizontal_scroll_offset(stable_message, 20.0);
    assert_eq!(tree.horizontal_scroll_count(), kept);
    for _ in 0..100 {
        tree.set_root(root());
        tree.layout(size);
        assert_eq!(tree.horizontal_scroll_count(), 8, "anonymous viewport state must stay bounded");
        assert_eq!(tree.horizontal_scroll_offset("stable-code"), 40.0);
        assert_eq!(tree.horizontal_scroll_offset(stable_message), 20.0);
        assert_eq!(tree.horizontal_scroll_offset("unrelated"), 80.0);
    }
}
