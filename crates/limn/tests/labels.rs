//! Labels are process-wide, so their changes run in one test in a separate
//! binary, leaving the library's parallel unit tests in their own language.

use limn::{
    labels, set_labels, Alert, AlertDialog, AlertTone, Breadcrumb, BreadcrumbItem, Component, ComponentPalette,
    DatePicker, DrawCommand, Element, FindLabels, Frame, Labels, MarkdownLabels, Size, UiTree,
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

    // What a component says when its host gives it nothing follows the labels too.
    set_labels(Labels::SPANISH);
    let drawn = |element: Element| {
        let frame = UiTree::new(element).layout(size);
        texts(&frame).into_iter().map(str::to_owned).collect::<Vec<_>>()
    };
    assert!(drawn(Alert::new("", "Disk almost full", palette).tone(AlertTone::Warning).render()).contains(&"Advertencia".to_owned()));
    assert!(drawn(DatePicker::new("date", "", palette).render()).contains(&"AAAA-MM-DD".to_owned()));
    assert_eq!((FindLabels::default().query.as_str(), FindLabels::default().dismiss.as_str()), ("Buscar", "Cerrar"));
    assert_eq!((MarkdownLabels::default().expand, MarkdownLabels::DEFAULT.expand), ("Ampliar", "Expand"));
    set_labels(original);
}
