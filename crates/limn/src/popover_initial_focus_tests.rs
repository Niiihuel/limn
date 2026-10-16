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
        alt: false,
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

fn modal_with_select(open: bool, background_menu: bool) -> Element {
    use crate::{Component, ComponentPalette, DropdownOption, Modal, Select};
    let background = Element::button("behind", "Background");
    let background = if background_menu {
        // This child's deferred scope is painted after the modal, even
        // though its trigger belongs to the covered background tree.
        Element::popover(
            background,
            Element::popover(
                Element::button("behind_trigger", "Background menu"),
                Element::popover(
                    Element::button("behind_second_trigger", "Background submenu"),
                    Element::button("behind_action", "Background action"),
                    "behind_grandchild_dismiss",
                    PopoverPlacement::BottomStart,
                )
                .popover_initial_focus("behind_action"),
                "behind_child_dismiss",
                PopoverPlacement::BottomStart,
            ),
            "behind_dismiss",
            PopoverPlacement::BottomStart,
        )
    } else {
        background
    };
    let select = Select::new(
        "choice",
        "First",
        ["first", "second", "third"]
            .into_iter()
            .map(|id| DropdownOption::new(id, id))
            .collect(),
        open,
        "choice_dismiss",
        ComponentPalette::neutral_dark(),
    )
    .width(180.0)
    .menu_width(220.0)
    .render()
    .popover_initial_focus("first")
    .popover_return_focus("choice");
    Modal::new(
        background,
        true,
        "Edit",
        "",
        "modal_dismiss",
        ComponentPalette::neutral_dark(),
    )
    .width(320.0)
    .height(260.0)
    .body(
        Element::column(vec![
            Element::text_input("form_name", "draft", "Name"),
            select,
        ])
        .gap(8.0),
    )
    .footer(Element::button("save", "Save"))
    .initial_focus("form_name")
    .return_focus("behind")
    .render()
}

#[test]
fn modal_select_escape_closes_the_select_before_the_modal() {
    let mut tree = UiTree::new(modal_with_select(false, false));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("form_name"));
    assert!(tree.focus("choice"));
    tree.set_root(modal_with_select(true, false));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("first"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "choice_dismiss".into()
        }]
    );
    tree.set_root(modal_with_select(false, false));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("choice"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "modal_dismiss".into()
        }]
    );
}

#[test]
fn modal_select_tab_and_arrows_stay_in_the_open_child() {
    let mut tree = UiTree::new(modal_with_select(true, false));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("first"));
    for expected in ["second", "third", "first"] {
        assert!(tree.handle_event(key(Key::Tab)).is_empty());
        assert_eq!(tree.focused_id(), Some(expected));
    }
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: true,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("third"));
    for (key_value, expected) in [
        (Key::Home, "first"),
        (Key::ArrowUp, "third"),
        (Key::ArrowDown, "first"),
        (Key::End, "third"),
    ] {
        assert!(tree.handle_event(key(key_value)).is_empty());
        assert_eq!(tree.focused_id(), Some(expected));
    }
    assert_eq!(
        tree.handle_event(key(Key::Enter)),
        [UiAction::Clicked { id: "third".into() }]
    );
}

#[test]
fn modal_masks_background_scopes_even_when_they_are_deferred_after_it() {
    let mut opened = UiTree::new(modal_with_select(true, true));
    opened.set_reduced_motion(true);
    opened.layout(VIEWPORT);
    assert_eq!(
        opened.focused_id(),
        Some("first"),
        "a later background target cannot hide the valid child initial focus"
    );
    let mut tree = UiTree::new(modal_with_select(false, true));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    let scopes = &tree.frame.popover_focus;
    let modal = scopes
        .iter()
        .position(|scope| scope.dismiss_id == "modal_dismiss")
        .unwrap();
    let behind = scopes
        .iter()
        .position(|scope| scope.dismiss_id == "behind_child_dismiss")
        .unwrap();
    assert!(
        behind > modal,
        "fixture must cover a later background scope"
    );
    for _ in 0..10 {
        tree.handle_event(key(Key::Tab));
        assert!(!tree.focused_id().unwrap().starts_with("behind"));
    }
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "modal_dismiss".into()
        }]
    );
    tree.set_root(modal_with_select(true, true));
    tree.layout(VIEWPORT);
    let scopes = &tree.frame.popover_focus;
    let choice = scopes
        .iter()
        .position(|scope| scope.dismiss_id == "choice_dismiss")
        .unwrap();
    let behind = scopes
        .iter()
        .position(|scope| scope.dismiss_id == "behind_grandchild_dismiss")
        .unwrap();
    assert!(
        behind > choice,
        "fixture must cover a later background initial focus"
    );
    assert_eq!(tree.focused_id(), Some("first"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "choice_dismiss".into()
        }]
    );
}

#[test]
fn modal_menu_keeps_nested_hover_submenus_reachable_with_the_keyboard() {
    use crate::{Component, ComponentPalette, DropdownMenu, DropdownMenuItem, Modal};
    let palette = ComponentPalette::neutral_dark();
    let menu = DropdownMenu::new(
        Element::button("menu", "Menu"),
        vec![DropdownMenuItem::new("category", "Category")
            .submenu(vec![DropdownMenuItem::new("subcategory", "Subcategory")
                .submenu(vec![DropdownMenuItem::new("leaf", "Leaf").into()])
                .into()])
            .into()],
        true,
        "menu_dismiss",
        palette,
    )
    .context_menu_style()
    .render();
    let mut tree = UiTree::new(
        Modal::new(
            Element::button("behind", "Behind"),
            true,
            "Edit",
            "",
            "modal_dismiss",
            palette,
        )
        .width(280.0)
        .height(200.0)
        .body(menu)
        .initial_focus("category")
        .render(),
    );
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("category"));
    for expected in ["subcategory", "leaf"] {
        tree.handle_event(key(Key::ArrowRight));
        tree.layout(VIEWPORT);
        assert_eq!(tree.focused_id(), Some(expected));
    }
    assert!(tree.handle_event(key(Key::Escape)).is_empty());
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("subcategory"));
    assert!(!tree.frame.regions.iter().any(|region| region.id == "leaf"));
    assert!(tree.handle_event(key(Key::Escape)).is_empty());
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("category"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        [UiAction::Clicked {
            id: "menu_dismiss".into()
        }]
    );
}

#[test]
fn clicking_blank_space_in_an_open_modal_blurs_instead_of_refocusing_its_first_field() {
    let mut tree = UiTree::new(modal_with_select(false, false));
    tree.set_reduced_motion(true);
    let frame = tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("form_name"));
    // Inside the modal's body, under the select and above the footer.
    let blank = Point::new(300.0, 190.0);
    assert_eq!(
        frame.hit_test(blank),
        Some("modal_body:modal_dismiss"),
        "the point is blank body space"
    );
    tree.handle_event(UiEvent::PointerDown(blank));
    tree.handle_event(UiEvent::PointerUp(blank));
    tree.layout(VIEWPORT);
    assert_eq!(
        tree.focused_id(),
        None,
        "a pointer blur is not sent to the first field"
    );
    // The keyboard still starts inside the modal.
    tree.handle_event(key(Key::Tab));
    let focused = tree.focused_id().map(str::to_owned);
    assert!(
        matches!(
            focused.as_deref(),
            Some("form_name" | "choice" | "save" | "modal_dismiss:close")
        ),
        "{focused:?}"
    );
}
