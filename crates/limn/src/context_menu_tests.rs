use super::*;
use crate::{
    CodeEditor, Component, ContextMenu, ContextMenuEntry, ContextMenuItem, EditorController, Modal,
};

#[test]
fn controlled_context_requests_the_hosts_action_without_opening_a_menu_or_clicking_the_row() {
    let mut tree = UiTree::new(ContextMenu::new(
        "song-context",
        Element::button("play:song", "Song").size(Some(240.0), Some(60.0)),
        Vec::new(),
        ComponentPalette::neutral_dark(),
    ).open_action("more:song").render());
    tree.layout(Size::new(320.0, 100.0));
    let point = Point::new(20.0, 20.0);
    assert_eq!(tree.frame.context_menu_at(point), Some("song-context"));
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(tree.handle_event(UiEvent::ContextMenu { position: point }), vec![UiAction::Clicked { id: "more:song".into() }]);
    assert!(!tree.context_menu_open());
    assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
    tree.focus("play:song");
    assert_eq!(tree.handle_event(UiEvent::ContextMenuKeyboard), vec![UiAction::Clicked { id: "more:song".into() }]);
    assert!(!tree.context_menu_open());
}

#[test]
fn context_hit_testing_does_not_reach_through_a_modal_backdrop() {
    let palette = ComponentPalette::neutral_dark();
    let row = ContextMenu::new("song-context", Element::button("song", "Song"), Vec::new(), palette).open_action("more:song").render();
    let root = Modal::new(row, true, "Actions", "", "close", palette).body(Element::button("action", "Action")).render();
    let frame = UiTree::new(root).layout(Size::new(320.0, 480.0));
    assert_eq!(frame.context_menu_at(Point::new(5.0, 5.0)), None);
}

fn key(key: Key, ctrl: bool) -> UiEvent {
    UiEvent::Key {
        key,
        ctrl,
        shift: false,
    }
}
fn input() -> Element {
    Element::text_input("draft", "ñandú 東京", "").size(Some(260.0), Some(32.0))
}
fn open(tree: &mut UiTree, point: Point) {
    assert!(tree
        .handle_event(UiEvent::ContextMenu { position: point })
        .is_empty());
    assert!(tree.context_menu_open());
    tree.layout(Size::new(300.0, 240.0));
}
fn item_enabled(tree: &UiTree, action: &str) -> bool {
    tree.context_menu.as_ref().unwrap().entries.iter().any(|entry| matches!(entry, ContextMenuEntry::Item(item) if item.action_id == action && item.enabled))
}

#[test]
fn menu_geometry_is_fixed_bounded_and_last_action_reachable_with_keyboard_and_wheel() {
    for size in [
        Size::new(360.0, 240.0),
        Size::new(520.0, 720.0),
        Size::new(1280.0, 900.0),
    ] {
        let entries = (0..30)
            .map(|index| {
                ContextMenuEntry::Item(
                    ContextMenuItem::new(format!("action:{index}"), format!("Acción {index}"))
                        .shortcut("Ctrl+Shift+Z"),
                )
            })
            .collect();
        let mut tree = UiTree::new(
            ContextMenu::new(
                "owner",
                Element::button("owner", "Editor").size(Some(180.0), Some(32.0)),
                entries,
                ComponentPalette::neutral_dark(),
            )
            .render(),
        );
        tree.set_reduced_motion(true);
        tree.layout(size);
        tree.focus("owner");
        tree.handle_event(UiEvent::ContextMenuKeyboard);
        let frame = tree.layout(size);
        let menu = frame
            .regions
            .iter()
            .find(|region| region.id == "__context_menu:surface")
            .unwrap()
            .rect;
        assert_eq!(
            menu.width,
            crate::context_menu::WIDTH.min(size.width - 16.0)
        );
        assert!(menu.height <= crate::context_menu::MAX_HEIGHT);
        assert!(menu.height <= size.height * 0.75);
        assert!(menu.x >= 8.0 && menu.y >= 8.0);
        assert!(menu.x + menu.width <= size.width - 8.0);
        assert!(menu.y + menu.height <= size.height - 8.0);
        tree.handle_event(key(Key::End, false));
        let frame = tree.layout(size);
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == crate::context_menu::row_id(29)));
        assert_eq!(
            tree.handle_event(key(Key::Enter, false)),
            vec![UiAction::Clicked {
                id: "action:29".into()
            }]
        );
        tree.layout(size);
        tree.handle_event(UiEvent::ContextMenuKeyboard);
        let frame = tree.layout(size);
        let menu = frame
            .regions
            .iter()
            .find(|region| region.id == "__context_menu:surface")
            .unwrap()
            .rect;
        tree.handle_event(UiEvent::ScrollDirect {
            position: Point::new(menu.x + 20.0, menu.y + 20.0),
            delta_y: -100_000.0,
        });
        let frame = tree.layout(size);
        let last = frame
            .regions
            .iter()
            .find(|region| region.id == crate::context_menu::row_id(29))
            .unwrap()
            .rect;
        let point = Point::new(last.x + last.width / 2.0, last.y + last.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "action:29".into()
            }]
        );
    }
}

#[test]
fn compact_menu_shortcuts_share_tooltip_keycaps_without_partial_chords() {
    for width in [180.0, 520.0] {
        let root = ContextMenu::new(
            "owner",
            Element::button("owner", "Editor").size(None, Some(32.0)),
            vec![
                ContextMenuEntry::Item(
                    ContextMenuItem::new("undo", "Deshacer").shortcut("Ctrl+Shift+Z"),
                ),
                ContextMenuEntry::Item(ContextMenuItem::new("plain", "Sin atajo").shortcut("")),
            ],
            ComponentPalette::neutral_dark(),
        )
        .render();
        let mut tree = UiTree::new(root);
        // Geometry is checked settled, not partway through the entrance.
        tree.set_reduced_motion(true);
        tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 9.0);
        let size = Size::new(width, 260.0);
        tree.layout(size);
        tree.focus("owner");
        tree.handle_event(UiEvent::ContextMenuKeyboard);
        let frame = tree.layout(size);
        let caps: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. }
                    if *radius == crate::shortcut::radius()
                        && rect.height == crate::shortcut::HEIGHT =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .collect();
        let chord_width =
            crate::shortcut::width("Ctrl+Shift+Z", |key| key.chars().count() as f32 * 9.0);
        let available = crate::context_menu::WIDTH.min(width - 16.0)
            - crate::context_menu::INSET * 2.0
            - crate::menu::action_style::HORIZONTAL_PADDING * 2.0;
        assert_eq!(
            caps.len(),
            if chord_width + crate::menu::action_style::GAP + 40.0 <= available {
                3
            } else {
                0
            }
        );
        if !caps.is_empty() {
            let row = frame
                .regions
                .iter()
                .find(|region| region.id == crate::context_menu::row_id(0))
                .unwrap()
                .rect;
            for cap in caps {
                assert!(cap.x >= row.x);
                assert!(cap.x + cap.width <= row.x + row.width - 7.9);
                assert!(cap.y >= row.y && cap.y + cap.height <= row.y + row.height);
            }
        }
    }
}

#[test]
fn modified_native_click_activates_menu_items_and_dismisses_outside() {
    let mut tree = UiTree::new(input());
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(15.0, 15.0));
    let index = tree
        .context_menu
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .position(|entry| matches!(entry, ContextMenuEntry::Item(item) if item.action_id == "copy"))
        .unwrap();
    let id = crate::context_menu::row_id(index);
    let rect = tree
        .frame()
        .regions
        .iter()
        .find(|region| region.id == id)
        .unwrap()
        .rect;
    let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    let press = |position| UiEvent::PointerDownEditor {
        position,
        shift: false,
        alt: false,
        ctrl: true,
        click_count: 2,
    };
    tree.handle_event(press(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::CopyRequested {
            text: "ñandú 東京".into()
        }]
    );
    assert!(!tree.context_menu_open());
    open(&mut tree, Point::new(15.0, 15.0));
    let outside = Point::new(296.0, 236.0);
    tree.handle_event(press(outside));
    assert!(!tree.context_menu_open());
    assert!(tree.handle_event(UiEvent::PointerUp(outside)).is_empty());
    assert_eq!(tree.input_value("draft"), Some("ñandú 東京"));
}

#[test]
fn text_context_copy_and_cut_keep_unicode_selection_and_focus() {
    let mut tree = UiTree::new(input());
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(tree.focused_id(), Some("draft"));
    assert_eq!(tree.input_selection("draft"), Some((0, 8)));
    assert_eq!(
        tree.handle_event(key(Key::C, true)),
        vec![UiAction::CopyRequested {
            text: "ñandú 東京".into()
        }]
    );
    assert_eq!(tree.input_value("draft"), Some("ñandú 東京"));
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(key(Key::X, true)),
        vec![
            UiAction::CopyRequested {
                text: "ñandú 東京".into()
            },
            UiAction::InputChanged {
                id: "draft".into(),
                value: String::new()
            }
        ]
    );
    assert_eq!(tree.focused_id(), Some("draft"));
}

#[test]
fn text_context_paste_is_explicit_and_rejects_late_focus_changes() {
    let mut tree = UiTree::new(Element::column(vec![
        input(),
        Element::text_input("other", "untouched", "").size(None, Some(32.0)),
    ]));
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(key(Key::V, true)),
        vec![UiAction::PasteRequested { id: "draft".into() }]
    );
    tree.focus("other");
    assert!(tree
        .handle_event(UiEvent::PasteTo {
            id: "draft".into(),
            text: "incorrect".into()
        })
        .is_empty());
    assert_eq!(tree.input_value("other"), Some("untouched"));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    assert_eq!(
        tree.handle_event(UiEvent::PasteTo {
            id: "draft".into(),
            text: "correcto".into()
        }),
        vec![UiAction::InputChanged {
            id: "draft".into(),
            value: "correcto".into()
        }]
    );
}

#[test]
fn context_password_and_readonly_editor_offer_only_permitted_operations() {
    let mut tree = UiTree::new(input().password(true));
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(15.0, 15.0));
    assert!(!item_enabled(&tree, "copy"));
    assert!(!item_enabled(&tree, "cut"));
    assert!(item_enabled(&tree, "paste"));
    assert!(tree.handle_event(key(Key::C, true)).is_empty());
    let controller =
        EditorController::new(limn_editor::TextBuffer::new("const value = 7;").unwrap());
    let mut tree = UiTree::new(
        CodeEditor::new("editor", controller)
            .read_only(true)
            .render(),
    );
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("editor");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(80.0, 30.0));
    assert!(item_enabled(&tree, "copy"));
    assert!(!item_enabled(&tree, "cut"));
    assert!(!item_enabled(&tree, "paste"));
    assert_eq!(
        tree.handle_event(key(Key::C, true)),
        vec![UiAction::CopyRequested {
            text: "const value = 7;".into()
        }]
    );
}

#[test]
fn context_static_text_preserves_selection_and_cannot_cross_modal_layers() {
    let mut tree = UiTree::new(Element::text("Copiar ñ 東京").size(Some(260.0), Some(32.0)));
    tree.layout(Size::new(300.0, 240.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 8.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(90.0, 8.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(90.0, 8.0)));
    let selected = tree.selected_text().unwrap().to_owned();
    open(&mut tree, Point::new(10.0, 8.0));
    assert_eq!(
        tree.handle_event(key(Key::C, true)),
        vec![UiAction::CopyRequested { text: selected }]
    );
    tree.set_root(
        Modal::new(
            Element::text("Copiar ñ 東京"),
            true,
            "Modal",
            "",
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .body(Element::text("Interior"))
        .width(220.0)
        .height(160.0)
        .render(),
    );
    tree.layout(Size::new(300.0, 240.0));
    tree.handle_event(UiEvent::ContextMenu {
        position: Point::new(1.0, 8.0),
    });
    assert!(!tree.context_menu_open());
}

#[test]
fn custom_context_keyboard_skips_disabled_and_outside_click_never_activates_base() {
    let root = ContextMenu::new(
        "custom",
        Element::button("base", "Owner").size(Some(280.0), Some(32.0)),
        vec![
            ContextMenuEntry::Item(
                ContextMenuItem::new("disabled", "No disponible").enabled(false),
            ),
            ContextMenuEntry::Item(ContextMenuItem::new("first", "Primera").shortcut("Ctrl+C")),
            ContextMenuEntry::Separator,
            ContextMenuEntry::Item(ContextMenuItem::new("last", "Última")),
        ],
        ComponentPalette::neutral_dark(),
    )
    .render();
    let mut tree = UiTree::new(root);
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("base");
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    tree.layout(Size::new(300.0, 240.0));
    assert!(tree.context_menu_open());
    assert_eq!(tree.focused_id(), Some("base"));
    tree.handle_event(key(Key::ArrowDown, false));
    assert_eq!(
        tree.handle_event(key(Key::Enter, false)),
        vec![UiAction::Clicked { id: "last".into() }]
    );
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(260.0, 15.0));
    let menu = tree
        .frame
        .regions
        .iter()
        .find(|region| region.id == "__context_menu:surface")
        .unwrap()
        .rect;
    assert!(menu.x >= 0.0 && menu.x + menu.width <= 300.0 && menu.y + menu.height <= 240.0);
    assert!(tree
        .handle_event(UiEvent::PointerDown(Point::new(2.0, 2.0)))
        .is_empty());
    assert!(!tree.context_menu_open());
    tree.layout(Size::new(300.0, 240.0));
    assert!(tree
        .handle_event(UiEvent::PointerUp(Point::new(2.0, 2.0)))
        .is_empty());
    assert_eq!(tree.focused_id(), Some("base"));
}

#[test]
fn context_menu_closes_if_target_disappears_and_dismiss_does_not_clear_selection() {
    let mut tree = UiTree::new(input());
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(15.0, 15.0));
    tree.dismiss_context_menu();
    assert_eq!(tree.input_selection("draft"), Some((0, 8)));
    open(&mut tree, Point::new(15.0, 15.0));
    tree.set_root(Element::text("Replaced"));
    tree.layout(Size::new(300.0, 240.0));
    assert!(!tree.context_menu_open());
    assert!(!tree
        .frame
        .regions
        .iter()
        .any(|region| region.id == "__context_menu:surface"));
}

#[test]
fn context_static_select_all_does_not_copy_a_previously_focused_input() {
    let mut tree = UiTree::new(Element::column(vec![
        input(),
        Element::text("Respuesta para copiar").size(Some(260.0), Some(32.0)),
    ]));
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(20.0, 40.0));
    tree.handle_event(key(Key::A, true));
    assert_eq!(tree.focused_id(), None);
    assert_eq!(
        tree.handle_event(key(Key::C, true)),
        vec![UiAction::CopyRequested {
            text: "Respuesta para copiar".into()
        }]
    );
    assert!(!tree.dismiss_context_menu());
}

#[test]
fn context_native_paste_uses_payload_once_without_another_clipboard_request() {
    let mut tree = UiTree::new(input());
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("draft");
    tree.handle_event(key(Key::A, true));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(UiEvent::Paste("pegado ñ".into())),
        vec![UiAction::InputChanged {
            id: "draft".into(),
            value: "pegado ñ".into()
        }]
    );
    assert!(!tree.context_menu_open());
    assert_eq!(tree.input_value("draft"), Some("pegado ñ"));
}

#[test]
fn context_native_paste_rejects_replacement_document_with_same_editor_id() {
    let original = EditorController::new(limn_editor::TextBuffer::new("original").unwrap());
    let mut tree = UiTree::new(CodeEditor::new("editor", original.clone()).render());
    tree.layout(Size::new(300.0, 240.0));
    tree.focus("editor");
    open(&mut tree, Point::new(80.0, 30.0));
    let replacement =
        EditorController::new(limn_editor::TextBuffer::new("replacement").unwrap());
    tree.set_root(CodeEditor::new("editor", replacement.clone()).render());
    tree.layout(Size::new(300.0, 240.0));
    assert!(tree.context_menu_open());
    assert_eq!(tree.focused_id(), Some("editor"));
    assert!(tree
        .handle_event(UiEvent::Paste("must not enter either document".into()))
        .is_empty());
    assert!(!tree.context_menu_open());
    for (controller, expected) in [(original, "original"), (replacement, "replacement")] {
        let snapshot = controller.snapshot().unwrap();
        assert_eq!(
            snapshot.text_range(0..snapshot.char_count()).unwrap(),
            expected
        );
        assert_eq!(snapshot.revision(), 0);
    }
}

#[test]
fn custom_context_shortcuts_match_modifiers_and_native_paste_dispatches_owner_action() {
    let root = ContextMenu::new(
        "terminal:7",
        Element::button("terminal_screen", "Terminal").size(Some(280.0), Some(32.0)),
        vec![
            ContextMenuEntry::Item(
                ContextMenuItem::new("terminal:7:copy_selection", "Copiar selección")
                    .shortcut("Ctrl+Shift+C"),
            ),
            ContextMenuEntry::Item(
                ContextMenuItem::new("terminal:7:paste", "Pegar").shortcut("Ctrl+Shift+V"),
            ),
            ContextMenuEntry::Item(
                ContextMenuItem::new("terminal:7:clear", "Limpiar").shortcut("Ctrl+L"),
            ),
        ],
        ComponentPalette::neutral_dark(),
    )
    .render();
    let mut tree = UiTree::new(root);
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert!(tree.handle_event(key(Key::C, true)).is_empty());
    assert!(tree.context_menu_open());
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::C,
            ctrl: true,
            shift: true
        }),
        vec![UiAction::Clicked {
            id: "terminal:7:copy_selection".into()
        }]
    );
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::V,
            ctrl: true,
            shift: true
        }),
        vec![UiAction::Clicked {
            id: "terminal:7:paste".into()
        }]
    );
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(UiEvent::Paste("native payload".into())),
        vec![UiAction::Clicked {
            id: "terminal:7:paste".into()
        }]
    );
    assert_eq!(tree.focused_id(), Some("terminal_screen"));
    tree.layout(Size::new(300.0, 240.0));
    open(&mut tree, Point::new(15.0, 15.0));
    assert_eq!(
        tree.handle_event(key(Key::L, true)),
        vec![UiAction::Clicked {
            id: "terminal:7:clear".into()
        }]
    );
}

#[test]
fn action_labels_have_priority_over_accelerators_and_wrap_without_losing_text() {
    let labels = [
        "Copiar salida completa",
        "Guardar todos los cambios del archivo seleccionado en la carpeta de trabajo",
        "Abrir documentación de la herramienta en el navegador predeterminado",
    ];
    for width in [180.0, 520.0] {
        let entries = labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                ContextMenuEntry::Item(
                    ContextMenuItem::new(format!("long:{index}"), *label)
                        .shortcut("Ctrl+Shift+Z")
                        .icon(Icon::File),
                )
            })
            .collect();
        let mut tree = UiTree::new(
            ContextMenu::new(
                "owner",
                Element::button("owner", "Editor").size(Some(120.0), Some(32.0)),
                entries,
                ComponentPalette::neutral_dark(),
            )
            .render(),
        );
        tree.set_text_measurer(|text, size, _| text.chars().count() as f32 * size * 0.58);
        tree.set_reduced_motion(true);
        let size = Size::new(width, 900.0);
        tree.layout(size);
        tree.focus("owner");
        tree.handle_event(UiEvent::ContextMenuKeyboard);
        let frame = tree.layout(size);
        let row = frame
            .regions
            .iter()
            .find(|region| region.id == "__context_menu:item:1")
            .unwrap()
            .rect;
        assert!(row.height > context_menu::ROW_HEIGHT);
        let label_lines = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    text, font_size, ..
                } if *font_size == context_menu::LABEL_FONT_SIZE => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(label_lines.iter().all(|line| !line.contains('…')));
        let painted = label_lines.join("");
        for label in labels {
            assert!(
                painted.contains(label),
                "lost label at {width}px: {painted:?}"
            );
        }
        assert_eq!(tree.focused_id(), Some("owner"));
        tree.handle_event(key(Key::End, false));
        tree.layout(size);
        assert_eq!(
            tree.handle_event(key(Key::Enter, false)),
            vec![UiAction::Clicked {
                id: "long:2".into()
            }]
        );
    }
}

#[test]
fn keyboard_reveals_the_start_of_a_wrapped_action_taller_than_the_menu() {
    let entries = vec![
        ContextMenuEntry::Item(ContextMenuItem::new("first", "Primera")),
        ContextMenuEntry::Item(ContextMenuItem::new(
            "long",
            "Guardar todos los cambios en la carpeta de trabajo. ".repeat(8),
        )),
    ];
    let mut tree = UiTree::new(
        ContextMenu::new(
            "owner",
            Element::button("owner", "Editor").size(Some(120.0), Some(32.0)),
            entries,
            ComponentPalette::neutral_dark(),
        )
        .render(),
    );
    tree.set_reduced_motion(true);
    let size = Size::new(180.0, 180.0);
    tree.layout(size);
    tree.focus("owner");
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    tree.layout(size);
    tree.handle_event(key(Key::ArrowDown, false));
    let frame = tree.layout(size);
    let target = frame
        .scroll_focus_targets
        .get("__context_menu:item:1")
        .unwrap();
    assert!(target.rect.height > target.viewport.height);
    assert!(
        (target.rect.y - target.viewport.y).abs() < 0.01,
        "keyboard navigation must reveal the beginning of a tall label: {target:?}"
    );
    let viewport = target.viewport;
    let top = tree.scroll_offset(context_menu::SCROLL_ID);
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(viewport.x + 10.0, viewport.y + 10.0),
        delta_y: -10_000.0,
    });
    let frame = tree.layout(size);
    assert!(tree.scroll_offset(context_menu::SCROLL_ID) > top);
    let row = frame
        .regions
        .iter()
        .find(|region| region.id == "__context_menu:item:1")
        .unwrap()
        .rect;
    let point = Point::new(row.x + row.width / 2.0, row.y + row.height / 2.0);
    assert_eq!(frame.hit_test(point), Some("__context_menu:item:1"));
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked { id: "long".into() }]
    );
}

#[test]
fn pointer_and_keyboard_share_one_context_menu_active_row() {
    let palette = ComponentPalette::neutral_dark();
    let mut tree = UiTree::new(
        ContextMenu::new(
            "owner",
            Element::button("owner", "Editor").size(Some(180.0), Some(32.0)),
            (0..3)
                .map(|index| {
                    ContextMenuEntry::Item(ContextMenuItem::new(
                        format!("action:{index}"),
                        format!("Acción {index}"),
                    ))
                })
                .collect(),
            palette,
        )
        .render(),
    );
    tree.set_reduced_motion(true);
    let size = Size::new(400.0, 300.0);
    tree.layout(size);
    tree.focus("owner");
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    let frame = tree.layout(size);
    let rows: Vec<_> = (0..3)
        .map(|index| {
            frame
                .regions
                .iter()
                .find(|region| region.id == crate::context_menu::row_id(index))
                .unwrap()
                .rect
        })
        .collect();
    let point = Point::new(rows[1].x + 20.0, rows[1].y + rows[1].height / 2.0);
    tree.handle_event(UiEvent::PointerMove(point));
    assert_eq!(tree.context_menu.as_ref().unwrap().selected, Some(1));
    tree.layout(size);
    tree.handle_event(key(Key::ArrowDown, false));
    assert_eq!(tree.context_menu.as_ref().unwrap().selected, Some(2));
    assert!(
        tree.hovered.is_none(),
        "keyboard navigation clears the stale pointer hover"
    );
    let frame = tree.layout(size);
    let active: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if rows.contains(rect) && *color == palette.hover =>
            {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(active, vec![rows[2]]);
    tree.handle_event(UiEvent::PointerMove(point));
    assert_eq!(tree.context_menu.as_ref().unwrap().selected, Some(1));
    assert_eq!(
        tree.handle_event(key(Key::Enter, false)),
        vec![UiAction::Clicked {
            id: "action:1".into()
        }]
    );
}
