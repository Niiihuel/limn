//! A dropdown's anchor keeps its place while its menu is open. This test
//! exercises the engine's popover layout and focus return through
//! `DropdownMenu`.
use crate::*;

#[test]
fn dropdown_keeps_its_compact_anchor_geometry_when_open_in_a_tall_row() {
    let palette = ComponentPalette::neutral_dark();
    let build = |open| {
        let anchor = Button::new("add_origin", "Agregar origen", palette)
            .icon(Icon::Add)
            .size(ControlSize::Compact)
            .render();
        let menu = DropdownMenu::new(
            anchor,
            vec![
                crate::DropdownMenuItem::new("origin:local", "Carpeta local").into(),
                crate::DropdownMenuItem::new("origin:github", "Repositorio GitHub").into(),
            ],
            open,
            "dismiss_origin",
            palette,
        )
        .width(200.0)
        .render();
        Element::row(vec![Element::spacer().flex(1.0), menu]).size(None, Some(50.0))
    };
    let viewport = Size::new(420.0, 260.0);
    let mut tree = UiTree::new(build(false));
    tree.set_reduced_motion(true);
    let closed = tree.layout(viewport);
    let anchor = closed
        .regions
        .iter()
        .find(|region| region.id == "add_origin")
        .expect("closed anchor")
        .rect;
    assert_eq!(anchor.height, 28.0);
    assert!(tree.focus("add_origin"));
    tree.set_root(build(true));
    let open = tree.layout(viewport);
    assert_eq!(
        open.regions
            .iter()
            .find(|region| region.id == "add_origin")
            .unwrap()
            .rect,
        anchor
    );
    for id in ["origin:local", "origin:github"] {
        assert!(open.regions.iter().any(|region| region.id == id));
    }
    assert!(tree.focus("origin:local"));
    let key = |key| UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
        alt: false,
    };
    assert_eq!(
        tree.handle_event(key(crate::Key::Enter)),
        vec![UiAction::Clicked {
            id: "origin:local".into()
        }]
    );
    assert_eq!(
        tree.handle_event(key(crate::Key::Escape)),
        vec![UiAction::Clicked {
            id: "dismiss_origin".into()
        }]
    );
    tree.set_root(build(false));
    let closed_again = tree.layout(viewport);
    assert_eq!(tree.focused_id(), Some("add_origin"));
    assert_eq!(
        closed_again
            .regions
            .iter()
            .find(|region| region.id == "add_origin")
            .unwrap()
            .rect,
        anchor
    );
}
