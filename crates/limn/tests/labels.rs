//! Labels are process-wide, so their changes run in one test in a separate
//! binary, leaving the library's parallel unit tests in their own language.

use limn::{
    labels, set_labels, AlertDialog, Breadcrumb, BreadcrumbItem, Component, ComponentPalette,
    DrawCommand, Element, Frame, Labels, Size, UiTree,
};

fn texts(frame: &Frame) -> Vec<&str> {
    frame.commands.iter().filter_map(|command| match command {
        DrawCommand::Text { text, .. } => Some(text.as_str()),
        _ => None,
    }).collect()
}

fn dialog(palette: ComponentPalette) -> AlertDialog {
    AlertDialog::new(Element::spacer(), true, "Delete file?", "", "confirm", "cancel", palette)
}

#[test]
fn replacing_labels_updates_rebuilt_components_and_preserves_explicit_copy() {
    let palette = ComponentPalette::neutral_dark();
    let size = Size::new(800.0, 600.0);
    let original = labels();
    let mut tree = UiTree::new(dialog(palette).render());
    tree.set_reduced_motion(true);
    let english = tree.layout(size);
    assert!(texts(&english).contains(&"Continue") && texts(&english).contains(&"Cancel"));

    // Existing elements keep their copy until the host rebuilds them.
    set_labels(Labels::SPANISH);
    let unchanged = tree.layout(size);
    assert_eq!(texts(&unchanged), texts(&english));
    tree.set_root(dialog(palette).render());
    let spanish = tree.layout(size);
    assert!(texts(&spanish).contains(&"Continuar") && texts(&spanish).contains(&"Cancelar"));
    assert!(!texts(&spanish).contains(&"Continue") && !texts(&spanish).contains(&"Cancel"));

    set_labels(Labels { confirm: "Confirmer", search: "Rechercher", ..Labels::ENGLISH });
    tree.set_root(dialog(palette).confirm_label("Delete permanently").render());
    let custom = tree.layout(size);
    assert!(texts(&custom).contains(&"Delete permanently") && texts(&custom).contains(&"Cancel"));
    assert!(!texts(&custom).contains(&"Confirmer"));

    let path = Breadcrumb::new("path", vec![BreadcrumbItem::new("main.rs")], palette)
        .search_action("find")
        .render();
    let mut path = UiTree::new(path);
    path.set_reduced_motion(true);
    path.layout(size);
    assert!(path.focus("find"));
    let hint = path.layout(size);
    assert!(texts(&hint).contains(&"Rechercher"));
    assert!(!texts(&hint).contains(&"Search"));
    set_labels(original);
}
