use super::*;

const SIZE: Size = Size::new(200.0, 390.0);

/// A uniform list of 50 rows of 20 px, each built by `row`.
fn list(row: impl Fn(usize) -> Element + Send + Sync + 'static) -> Element {
    Element::uniform_list("rows", 50, 20.0, move |index| row(index).size(None, Some(20.0)))
}

fn duplicates(root: Element) -> Vec<String> {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(SIZE).duplicate_ids().to_vec()
}

fn link(index: usize) -> Element {
    Element::formatted_text(vec![TextSpan::new(format!("docs {index}"), Color::rgb(0, 0, 0), 14.0, FontFamily::Sans).action("open:docs")])
}

#[test]
fn a_frame_lists_an_id_taken_twice_in_a_row_across_rows_or_with_the_tree_around_the_list() {
    let inside = list(|index| {
        let again = if index == 2 { "open:2".to_owned() } else { format!("more:{index}") };
        Element::row(vec![Element::button(format!("open:{index}"), "Open"), Element::button(again, "Again")])
    });
    assert_eq!(duplicates(inside), ["open:2"]);
    let across = list(|index| Element::button(if index % 2 == 0 { "even".to_owned() } else { format!("open:{index}") }, "Open"));
    assert_eq!(duplicates(across), ["even"]);
    let around = Element::column(vec![
        Element::button("save", "Save"),
        Element::button("save", "Save again"),
        list(|index| Element::button(if index == 1 { "close".to_owned() } else { format!("open:{index}") }, "Open")).flex(1.0),
        Element::button("close", "Close"),
        Element::text_input("open:3", "", "Name"),
    ]);
    // What the walk found, in tree order, then what placing the rows found.
    assert_eq!(duplicates(around), ["save", "close", "open:3"]);
    let measured = Element::measured_list("rows", 50, 20.0, |index| index as u64, |index| {
        Element::clickable(if index == 3 { "pick:0".to_owned() } else { format!("pick:{index}") }, Element::text("Pick")).size(None, Some(20.0))
    });
    assert_eq!(duplicates(measured), ["pick:0"]);
}

#[test]
fn a_frame_does_not_list_a_row_control_laid_out_once() {
    let rows = |index: usize| Element::row(vec![Element::text(format!("Row {index}")), Element::button(format!("open:{index}"), "Open")]);
    assert!(duplicates(list(rows)).is_empty());
    // A link linked again, and a second target for the pointer alone.
    let repeated = list(|index| {
        Element::row(vec![link(index), Element::clickable("cover", Element::text("Cover")).pointer_only(), Element::button(format!("open:{index}"), "Open")])
    });
    assert!(duplicates(Element::column(vec![link(99), repeated.flex(1.0)])).is_empty());

    // A pass that runs again places the same rows again.
    let mut tree = UiTree::new(Element::measured_list("rows", 50, 20.0, |index| index as u64, move |index| rows(index).size(None, Some(20.0))));
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree.reveal("open:19");
    let frame = tree.layout(SIZE);
    assert!(tree.scroll_offset("rows") > 0.0, "the reveal moved the list, so the pass ran again");
    assert!(frame.duplicate_ids().is_empty());

    // A glimpse of the list, inert, takes no focus, however its rows are made.
    let twice = |index: usize| Element::row(vec![Element::button(format!("open:{index}"), "Open"), Element::button(format!("open:{index}"), "Again")]);
    let glimpse = Element::uniform_list("glimpse", 50, 20.0, move |index| twice(index).size(None, Some(20.0)));
    assert!(duplicates(Element::column(vec![list(rows).size(None, Some(100.0)), glimpse.size(None, Some(100.0)).inert()])).is_empty());

    // A sticky header, pinned over the rows, holding a list.
    let header = Element::column(vec![
        Element::text("Tabs").size(None, Some(20.0)),
        Element::uniform_list("tabs", 3, 20.0, |index| Element::button(format!("tab:{index}"), "Tab").size(None, Some(20.0))).size(None, Some(60.0)),
    ])
    .sticky_top();
    let mut children = vec![header];
    children.extend((0..20).map(|index| Element::button(format!("item:{index}"), "Item").size(None, Some(20.0))));
    let mut tree = UiTree::new(Element::scroll("page", Element::column(children)));
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree.set_scroll_offset("page", 30.0);
    let frame = tree.layout(SIZE);
    let pinned = frame.regions.iter().filter(|region| region.id == "tab:1").map(|region| region.rect.y).collect::<Vec<_>>();
    assert_eq!(pinned, [40.0], "pinned at the top, and not in its slot as well");
    assert!(frame.duplicate_ids().is_empty());
}
