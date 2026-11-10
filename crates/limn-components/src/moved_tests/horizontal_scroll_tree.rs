// Staged by the crate split, not yet a module of this crate: this test
// builds its tree with `CodeBlock` and `Markdown`, which live here now.
//
// Came from `crates/limn-core/src/horizontal_scroll_tree.rs` (a child module
// of `tree` there), its `mod tests`, unchanged:
// - `rebuilding_anonymous_code_viewports_bounds_state_and_preserves_explicit_ids`
// Its `use super::*` (that module, and through it the engine's `tree`
// module) is replaced by `use crate::*`.
//
// Engine items it uses that `limn-core` does not make public:
// - `UiTree::horizontal_scrolls`: private field, a map of the `pub(super)`
//   type `HorizontalScrollState`, whose `offset` field is private. The test
//   counts the map's entries and writes two offsets through `get_mut().unwrap()`.
//   The public `UiTree::set_horizontal_scroll_offset` is close to that write
//   but not the same (it creates a missing entry and stops a glide), so the
//   test is left as it was.
mod tests {
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
        tree.horizontal_scrolls.get_mut("unrelated").unwrap().offset = 80.0;
        let stable_message = "markdown:stable-message:code:0";
        tree.horizontal_scrolls.get_mut(stable_message).unwrap().offset = 20.0;
        for _ in 0..100 {
            tree.set_root(root());
            tree.layout(size);
            assert_eq!(tree.horizontal_scrolls.len(), 8, "anonymous viewport state must stay bounded");
            assert_eq!(tree.horizontal_scroll_offset("stable-code"), 40.0);
            assert_eq!(tree.horizontal_scroll_offset(stable_message), 20.0);
            assert_eq!(tree.horizontal_scroll_offset("unrelated"), 80.0);
        }
    }
}
