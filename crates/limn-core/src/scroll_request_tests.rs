use super::*;

const SIZE: Size = Size::new(200.0, 100.0);

/// A list of 50 rows of 20 px, or the loading state that stands in for it.
fn results(shown: bool) -> Element {
    if shown {
        Element::uniform_list("results", 50, 20.0, |index| Element::text(format!("row {index}")))
    } else {
        Element::text("Searching…")
    }
}

#[test]
fn a_scroll_asked_of_a_list_not_shown_yet_lands_when_a_later_layout_places_it() {
    let mut tree = UiTree::new(results(false));
    tree.scroll_to_index("results", 40);
    tree.scroll_to_end("transcript");
    for _ in 0..3 {
        tree.layout(SIZE);
    }
    tree.set_root(results(true));
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("results"), 41.0 * 20.0 - SIZE.height);
    assert!(tree.scroll_to_index.remove("results").is_none(), "the layout that placed it took it");
    tree.set_root(Element::scroll("transcript", Element::column((0..20).map(|_| Element::text("line").size(None, Some(20.0))).collect())));
    tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("transcript"), 20.0 * 20.0 - SIZE.height);
}

#[test]
fn scrolls_asked_of_ids_never_laid_out_are_kept_only_for_the_most_recent() {
    let mut tree = UiTree::new(results(false));
    for index in 0..200 {
        tree.scroll_to_end(&format!("gone:{index}"));
        tree.scroll_to_index(&format!("gone:{index}:list"), index);
        tree.layout(SIZE);
    }
    assert_eq!(tree.scroll_to_end.pending.len(), DORMANT_KEPT);
    assert_eq!(tree.scroll_to_index.pending.len(), DORMANT_KEPT);
    assert!(tree.scroll_to_end.remove("gone:199"));
    assert!(tree.scroll_to_end.remove("gone:136"));
    assert!(!tree.scroll_to_end.remove("gone:135"));
    assert_eq!(tree.scroll_to_index.remove("gone:136:list"), Some(136));
    assert_eq!(tree.scroll_to_index.remove("gone:135:list"), None);
    // Asking again counts as asking now.
    tree.scroll_to_end("again");
    for index in 0..DORMANT_KEPT - 1 {
        tree.scroll_to_end(&format!("other:{index}"));
    }
    tree.scroll_to_end("again");
    tree.scroll_to_end("last");
    assert!(tree.scroll_to_end.remove("again"));
}
