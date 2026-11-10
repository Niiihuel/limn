//! A menu row's secondary action, such as a close button beside the row,
//! stays out of the arrow keys' order and in Tab's. These tests exercise the
//! engine's popover focus through `DropdownMenu`, `MenuItem` and `Tooltip`.
use crate::*;

fn menu_with_secondary_actions(mark_secondary: bool, visible_rows: usize) -> Element {
    use crate::{
        Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
        MenuItem, Tooltip,
    };

    let palette = ComponentPalette::neutral_dark();
    let mut entries = vec![DropdownMenuItem::new("normal", "New terminal").into()];
    for index in 1..=2 {
        let close_id = format!("close:{index}");
        let mut close = Element::icon_button(&close_id, Icon::Close, "")
            .size(Some(24.0), Some(24.0))
            .dispatch_as(format!("terminal_close:{index}"));
        if mark_secondary {
            close = close.menu_secondary_action();
        }
        entries.push(DropdownMenuEntry::custom(
            Element::row(vec![
                MenuItem::new(
                    format!("main:{index}"),
                    format!("Terminal {index}"),
                    palette,
                )
                .width(180.0)
                .height(28.0)
                .render(),
                Tooltip::new(close, close_id, "Close terminal", palette).render(),
            ])
            .gap(4.0)
            .size(None, Some(28.0)),
        ));
    }
    DropdownMenu::new(
        Element::button("trigger", "Terminals").size(Some(100.0), Some(28.0)),
        entries,
        true,
        "dismiss",
        palette,
    )
    .width(240.0)
    .max_visible_items(visible_rows)
    .render()
}

#[test]
fn popover_arrows_skip_secondary_actions_but_tab_and_activation_keep_them() {
    let viewport = Size::new(320.0, 240.0);
    let mut tree = UiTree::new(menu_with_secondary_actions(true, 8));
    tree.set_reduced_motion(true);
    tree.layout(viewport);
    let key = |key| UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
        alt: false,
    };
    for (from, direction, expected) in [
        ("main:1", Key::ArrowDown, "main:2"),
        ("main:2", Key::ArrowUp, "main:1"),
        ("close:1", Key::ArrowDown, "main:2"),
        ("close:2", Key::ArrowUp, "main:2"),
        ("close:2", Key::ArrowDown, "normal"),
        ("normal", Key::ArrowUp, "main:2"),
        ("close:2", Key::Home, "normal"),
        ("normal", Key::End, "main:2"),
        ("trigger", Key::ArrowUp, "main:2"),
        ("trigger", Key::ArrowDown, "normal"),
    ] {
        assert!(tree.focus(from));
        assert!(tree.handle_event(key(direction)).is_empty());
        assert_eq!(tree.focused_id(), Some(expected), "{from}: {direction:?}");
    }
    assert!(tree.focus("main:1"));
    assert!(tree.handle_event(key(Key::Tab)).is_empty());
    assert_eq!(tree.focused_id(), Some("close:1"));
    for activate in [Key::Enter, Key::Space] {
        assert_eq!(
            tree.handle_event(key(activate)),
            vec![UiAction::Clicked {
                id: "terminal_close:1".into(),
            }]
        );
    }
    assert!(tree
        .handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: true,
            ctrl: false,
            alt: false,
        })
        .is_empty());
    assert_eq!(tree.focused_id(), Some("main:1"));

    let frame = tree.layout(viewport);
    let close = frame
        .regions
        .iter()
        .find(|region| region.id == "close:2")
        .unwrap()
        .rect;
    let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
    assert_eq!(frame.hit_test(point), Some("close:2"));
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked {
            id: "terminal_close:2".into(),
        }]
    );
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        vec![UiAction::Clicked {
            id: "dismiss".into(),
        }]
    );
    assert_eq!(tree.focused_id(), Some("trigger"));
}

#[test]
fn popover_secondary_actions_preserve_offscreen_primary_focus_and_reveal() {
    let viewport = Size::new(320.0, 240.0);
    let mut tree = UiTree::new(menu_with_secondary_actions(true, 1));
    tree.set_reduced_motion(true);
    let before = tree.layout(viewport);
    assert!(!before.regions.iter().any(|region| region.id == "main:2"));
    assert!(!before.has_scroll_focus_target("main:2"));
    assert!(before.has_scroll_focus_candidate("main:2"));
    assert!(tree.focus("trigger"));
    assert!(tree
        .handle_event(UiEvent::Key {
            key: Key::End,
            shift: false,
            ctrl: false,
            alt: false,
        })
        .is_empty());
    assert_eq!(tree.focused_id(), Some("main:2"));
    let frame = tree.layout(viewport);
    let options = frame
        .regions
        .iter()
        .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
        .unwrap()
        .rect;
    let main = frame
        .regions
        .iter()
        .find(|region| region.id == "main:2")
        .unwrap()
        .rect;
    assert_eq!(main.height, 28.0);
    assert!(main.y >= options.y && main.y + main.height <= options.y + options.height);
    assert!(tree.scroll_offset("dismiss:options") > 0.0);
    assert!(tree
        .handle_event(UiEvent::Key {
            key: Key::Tab,
            shift: false,
            ctrl: false,
            alt: false,
        })
        .is_empty());
    assert_eq!(tree.focused_id(), Some("close:2"));
}

#[test]
fn unmarked_popover_buttons_keep_the_existing_arrow_order() {
    let mut tree = UiTree::new(menu_with_secondary_actions(false, 8));
    tree.set_reduced_motion(true);
    tree.layout(Size::new(320.0, 240.0));
    assert!(tree.focus("main:1"));
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowDown,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("close:1"));
    tree.handle_event(UiEvent::Key {
        key: Key::End,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("close:2"));
}
