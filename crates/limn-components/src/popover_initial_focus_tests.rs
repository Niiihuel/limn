//! Initial focus is a one-time transition, not an effect of every layout.
//! These tests exercise the engine's popover and modal focus scopes through
//! `DropdownMenu`, `Select` and `Modal`.
use crate::*;

const VIEWPORT: Size = Size::new(460.0, 320.0);

#[derive(Clone, Copy)]
enum Surface {
    Rename,
}

fn root(surface: Surface, preferred: Option<&str>) -> Element {
    let selector = Element::button("selector", "Terminal").size(Some(160.0), Some(32.0));
    let more = Element::button("more", "Actions").size(Some(90.0), Some(32.0));
    let (selector, more) = match surface {
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
    let scopes = tree.frame().popover_focus_dismiss_ids();
    let modal = scopes
        .iter()
        .position(|dismiss_id| dismiss_id == "modal_dismiss")
        .unwrap();
    let behind = scopes
        .iter()
        .position(|dismiss_id| dismiss_id == "behind_child_dismiss")
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
    let scopes = tree.frame().popover_focus_dismiss_ids();
    let choice = scopes
        .iter()
        .position(|dismiss_id| dismiss_id == "choice_dismiss")
        .unwrap();
    let behind = scopes
        .iter()
        .position(|dismiss_id| dismiss_id == "behind_grandchild_dismiss")
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
    assert!(!tree.frame().regions.iter().any(|region| region.id == "leaf"));
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

/// A button behind a modal that `open` shows, which hands the button its focus back.
fn modal_over_button(open: bool) -> Element {
    use crate::{Component, ComponentPalette, Modal};
    Modal::new(
        Element::button("behind", "Background"),
        open,
        "Edit",
        "",
        "modal_dismiss",
        ComponentPalette::neutral_dark(),
    )
    .width(320.0)
    .height(200.0)
    .body(Element::button("inside", "Inside"))
    .return_focus("behind")
    .render()
}

#[test]
fn a_modal_hands_back_the_focus_as_the_pointer_or_the_keyboard_gave_it() {
    // A press opened it: the Esc that closes it does not make that focus the keyboard's.
    let mut tree = UiTree::new(modal_over_button(false));
    tree.set_reduced_motion(true);
    let frame = tree.layout(VIEWPORT);
    let behind = frame.regions.iter().find(|r| r.id == "behind").expect("the button").rect;
    let at = Point::new(behind.x + 4.0, behind.y + 4.0);
    tree.handle_event(UiEvent::PointerDown(at));
    tree.handle_event(UiEvent::PointerUp(at));
    assert_eq!(tree.focused_id(), Some("behind"));
    assert!(!tree.has_keyboard_focus());
    tree.set_root(modal_over_button(true));
    tree.layout(VIEWPORT);
    assert_eq!(tree.handle_event(key(Key::Escape)), [UiAction::Clicked { id: "modal_dismiss".into() }]);
    tree.set_root(modal_over_button(false));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("behind"));
    assert!(!tree.has_keyboard_focus(), "the pointer's focus comes back as the pointer's");
    // The keyboard gave it: it comes back as the keyboard's.
    assert!(tree.focus("behind"));
    tree.set_root(modal_over_button(true));
    tree.layout(VIEWPORT);
    tree.handle_event(key(Key::Escape));
    tree.set_root(modal_over_button(false));
    tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("behind"));
    assert!(tree.has_keyboard_focus(), "the keyboard's focus comes back as the keyboard's");
}
