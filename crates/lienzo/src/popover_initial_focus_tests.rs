//! Initial focus is a one-time transition, not an effect of every layout.
use super::*;

const VIEWPORT: Size = Size::new(460.0, 320.0);

#[derive(Clone, Copy)]
enum Surface {
    Closed,
    Menu,
    Rename,
}

fn root(surface: Surface, preferred: Option<&str>) -> Element {
    let selector = Element::button("selector", "Terminal").size(Some(160.0), Some(32.0));
    let more = Element::button("more", "Actions").size(Some(90.0), Some(32.0));
    let (selector, more) = match surface {
        Surface::Closed => (selector, more),
        Surface::Menu => (
            selector,
            Element::popover(
                more,
                Element::button("rename", "Rename").size(Some(180.0), Some(32.0)),
                "menu_dismiss",
                PopoverPlacement::BottomEnd,
            ),
        ),
        Surface::Rename => {
            let mut form = Element::popover(
                selector,
                Element::column(vec![
                    Element::text_input("name", "shell", "Terminal name").size(None, Some(32.0)),
                    Element::button("confirm", "Rename").size(None, Some(32.0)),
                    Element::button("cancel", "Cancel").size(None, Some(32.0)),
                ])
                .gap(6.0)
                .padding(8.0)
                .size(Some(230.0), None),
                "rename_dismiss",
                PopoverPlacement::BottomStart,
            )
            .popover_return_focus("selector");
            if let Some(preferred) = preferred {
                form = form.popover_initial_focus(preferred);
            }
            (form, more)
        }
    };
    Element::column(vec![
        Element::row(vec![selector, more]).gap(8.0),
        Element::text_input("background", "draft", "Background input").size(None, Some(32.0)),
    ])
    .gap(8.0)
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
    }
}

fn click(tree: &mut UiTree, id: &str) {
    let frame = tree.layout(VIEWPORT);
    let region = frame
        .regions
        .iter()
        .rev()
        .find(|region| region.id == id)
        .unwrap();
    let point = Point::new(
        region.rect.x + region.rect.width / 2.0,
        region.rect.y + region.rect.height / 2.0,
    );
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked { id: id.into() }]
    );
}

fn enter_rename_from_menu(tree: &mut UiTree, preferred: Option<&str>) {
    click(tree, "more");
    tree.set_root(root(Surface::Menu, None));
    click(tree, "rename");
    assert_eq!(tree.focused_id(), Some("rename"));
    tree.set_root(root(Surface::Rename, preferred));
    tree.layout(VIEWPORT);
}

#[test]
fn menu_to_nonmodal_form_focuses_before_typing_and_restores_after_submit_or_escape() {
    let mut tree = UiTree::new(root(Surface::Closed, None));
    tree.set_reduced_motion(true);
    enter_rename_from_menu(&mut tree, Some("name"));
    assert_eq!(tree.focused_id(), Some("name"));
    assert!(!tree.frame.popover_focus.last().unwrap().modal);
    assert!(tree
        .handle_event(UiEvent::Text(" 2".into()))
        .contains(&UiAction::InputChanged {
            id: "name".into(),
            value: "shell 2".into(),
        }));
    assert_eq!(
        tree.handle_event(key(Key::Enter)),
        vec![UiAction::Submitted {
            id: "name".into(),
            value: "shell 2".into(),
        }]
    );
    tree.set_root(root(Surface::Closed, None));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("selector"));

    // A genuinely reopened scope should apply its initial focus again.
    enter_rename_from_menu(&mut tree, Some("name"));
    assert_eq!(tree.focused_id(), Some("name"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        vec![UiAction::Clicked {
            id: "rename_dismiss".into(),
        }]
    );
    tree.set_root(root(Surface::Closed, None));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("selector"));
}

#[test]
fn nonmodal_initial_focus_does_not_reset_caret_or_steal_button_focus_on_rebuild() {
    let mut tree = UiTree::new(root(Surface::Closed, None));
    tree.set_reduced_motion(true);
    enter_rename_from_menu(&mut tree, Some("name"));
    tree.handle_event(key(Key::Home));
    tree.set_root(root(Surface::Rename, Some("name")));
    tree.layout(VIEWPORT);
    assert!(tree
        .handle_event(UiEvent::Text("new ".into()))
        .contains(&UiAction::InputChanged {
            id: "name".into(),
            value: "new shell".into(),
        }));
    tree.handle_event(key(Key::Tab));
    assert_eq!(tree.focused_id(), Some("confirm"));
    for _ in 0..3 {
        tree.set_root(root(Surface::Rename, Some("name")));
        tree.layout(VIEWPORT);
        assert_eq!(tree.focused_id(), Some("confirm"));
    }
    assert_eq!(
        tree.handle_event(key(Key::Enter)),
        vec![UiAction::Clicked {
            id: "confirm".into(),
        }]
    );
}

#[test]
fn nonmodal_form_without_valid_preferred_keeps_restored_menu_trigger() {
    for preferred in [None, Some("missing"), Some("background")] {
        let mut tree = UiTree::new(root(Surface::Closed, None));
        tree.set_reduced_motion(true);
        enter_rename_from_menu(&mut tree, preferred);
        assert_eq!(tree.focused_id(), Some("more"), "{preferred:?}");
        assert!(tree
            .handle_event(UiEvent::Text("must not edit".into()))
            .is_empty());
        assert_eq!(tree.input_value("background"), Some("draft"));
    }
}

#[test]
fn modal_close_restores_before_new_nonmodal_form_initial_focus() {
    let modal = Element::popover(
        Element::button("selector", "Terminal"),
        Element::button("modal_next", "Rename"),
        "modal_dismiss",
        PopoverPlacement::BottomStart,
    )
    .popover_scrim(Color::rgba(0, 0, 0, 100))
    .popover_return_focus("selector");
    let mut tree = UiTree::new(modal);
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("modal_next"));
    tree.set_root(root(Surface::Rename, Some("name")));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("name"));
}

#[test]
fn leaving_action_menu_row_clears_activation_without_stealing_form_input_focus() {
    let palette = crate::ComponentPalette::neutral_dark();
    let menu = crate::DropdownMenu::new(
        Element::button("actions", "Actions"),
        vec![
            crate::DropdownMenuItem::new("first", "First").into(),
            crate::DropdownMenuItem::new("second", "Second").into(),
        ],
        true,
        "dismiss",
        palette,
    )
    .context_menu_style()
    .render();
    use crate::Component;
    let mut tree = UiTree::new(menu);
    tree.set_reduced_motion(true);
    let frame = tree.layout(VIEWPORT);
    let row = frame
        .regions
        .iter()
        .find(|r| r.id == "second")
        .unwrap()
        .rect;
    tree.handle_event(UiEvent::PointerMove(Point::new(row.x + 8.0, row.y + 8.0)));
    assert_eq!(tree.focused_id(), Some("second"));
    tree.handle_event(UiEvent::PointerMove(Point::new(450.0, 300.0)));
    assert_eq!(tree.focused_id(), Some("actions"));
    assert!(!tree
        .handle_event(key(Key::Enter))
        .iter()
        .any(|action| matches!(action, UiAction::Clicked { id } if id == "second")));

    // Moving across a floating form never retargets its text input.
    tree.set_root(root(Surface::Rename, Some("name")));
    let frame = tree.layout(VIEWPORT);
    let row = frame
        .regions
        .iter()
        .find(|r| r.id == "confirm")
        .unwrap()
        .rect;
    tree.handle_event(UiEvent::PointerMove(Point::new(row.x + 8.0, row.y + 8.0)));
    assert_eq!(tree.focused_id(), Some("name"));
}

#[test]
fn nested_action_menu_left_and_escape_close_child_without_focus_reopening_it() {
    use crate::{Component, ComponentPalette, DropdownMenu, DropdownMenuItem};
    let menu = DropdownMenu::new(
        Element::button("menu", "Menu"),
        vec![DropdownMenuItem::new("category", "File")
            .submenu(vec![DropdownMenuItem::new("new_file", "New File").into()])
            .into()],
        true,
        "dismiss",
        ComponentPalette::neutral_dark(),
    )
    .context_menu_style()
    .render();
    let mut tree = UiTree::new(menu);
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    tree.focus("category");
    tree.layout(VIEWPORT);
    tree.handle_event(key(Key::ArrowRight));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("new_file"));
    assert!(tree.handle_event(key(Key::ArrowLeft)).is_empty());
    let frame = tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("category"));
    assert!(!frame.regions.iter().any(|r| r.id == "new_file"));
    // Reopen explicitly, then Esc closes only the child; the next Esc closes parent.
    tree.handle_event(key(Key::ArrowRight));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("new_file"));
    assert!(tree.handle_event(key(Key::Escape)).is_empty());
    let frame = tree.layout(VIEWPORT);
    assert!(!frame.regions.iter().any(|r| r.id == "new_file"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
}
