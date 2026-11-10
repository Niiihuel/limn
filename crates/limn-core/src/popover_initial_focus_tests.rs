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
fn a_modals_first_control_shows_no_tooltip_until_the_keyboard_moves_there() {
    use crate::Component as _;
    let page = || Element::text_input("background", "draft", "Background input").size(Some(200.0), Some(32.0));
    let mut tree = UiTree::new(page());
    tree.layout(VIEWPORT);
    // The host last put the focus somewhere itself, which counts as the keyboard's.
    tree.layout(VIEWPORT);
    assert!(tree.focus("background"));
    assert_eq!(tree.focused_id(), Some("background"));
    let close = crate::Tooltip::new(
        Element::button("close", "Close").size(Some(80.0), Some(32.0)),
        "close",
        "Close image",
        crate::ComponentPalette::neutral_dark(),
    )
    .render();
    // A modal over the page, its close control in a surface of its own beside it, as an image
    // viewer's.
    let actions = Element::popover(
        Element::spacer().size(Some(120.0), Some(80.0)),
        close,
        "actions",
        PopoverPlacement::RightStart,
    )
    .popover_non_modal()
    .popover_initial_focus("close");
    let modal = Element::popover(page(), actions, "dismiss", PopoverPlacement::Center)
        .popover_scrim(Color::rgba(0, 0, 0, 160));
    tree.set_root(modal);
    let frame = tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("close"));
    // A tooltip shows its pointer, a triangle; nothing else here draws one.
    let shown = |frame: &Frame| frame.commands.iter().any(|command| matches!(command, DrawCommand::FillTriangle { .. }));
    assert!(!shown(&frame), "the opening focus raises no tooltip");
    // Once the keyboard moves to it, it says what it is.
    tree.handle_event(key(Key::Tab));
    tree.handle_event(UiEvent::Key { key: Key::Tab, shift: true, ctrl: false, alt: false });
    let frame = tree.layout(VIEWPORT);
    assert_eq!(tree.focused_id(), Some("close"));
    assert!(shown(&frame), "the keyboard's focus does");
}

#[test]
fn a_backdrop_that_answers_to_a_controls_id_raises_no_tooltip() {
    use crate::Component as _;
    // An image viewer's: its close control and its backdrop report the same id.
    let close = crate::Tooltip::new(
        Element::button("close", "Close").size(Some(80.0), Some(32.0)),
        "close",
        "Close image",
        crate::ComponentPalette::neutral_dark(),
    )
    .render();
    let page = Element::text_input("background", "draft", "Background input").size(Some(200.0), Some(32.0));
    let mut tree = UiTree::new(
        Element::popover(page, close, "close", PopoverPlacement::Center)
            .popover_scrim(Color::rgba(0, 0, 0, 160))
            .popover_backdrop_dismiss(true),
    );
    tree.layout(VIEWPORT);
    tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
    let since = tree.tooltip_hover_since.as_ref().map(|(_, since)| *since).unwrap_or_else(Instant::now);
    let frame = tree.layout_at(VIEWPORT, since + Duration::from_secs(2));
    let shown = frame.commands.iter().any(|command| matches!(command, DrawCommand::FillTriangle { .. }));
    assert!(!shown, "the pointer on the backdrop is not on the close control");
}
