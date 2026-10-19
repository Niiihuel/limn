use crate::*;

fn key(tree: &mut UiTree, key: Key, shift: bool, ctrl: bool) -> Vec<UiAction> {
    tree.handle_event(UiEvent::Key { key, shift, ctrl })
}

fn editor(value: &str, preserve: bool) -> Element {
    TextArea::new("raw", value, "", ComponentPalette::neutral_dark())
        .preserve_line_endings(preserve)
        .height(140.0)
        .width(320.0)
        .render()
        .font_family(FontFamily::Monospace)
        .font_size(14.0)
}

#[test]
fn raw_textarea_paste_copy_cut_rebuild_and_backspace_retain_original_utf8_bytes() {
    let raw = "  ñ\r\n🦀\r終\t\0";
    let mut tree = UiTree::new(editor("", true));
    tree.layout(Size::new(320.0, 140.0));
    assert!(tree.focus("raw"));
    assert_eq!(
        tree.handle_event(UiEvent::Text(raw.into())),
        vec![UiAction::InputChanged {
            id: "raw".into(),
            value: raw.into()
        }]
    );
    tree.set_root(editor(raw, true));
    tree.layout(Size::new(320.0, 140.0));
    assert_eq!(tree.input_value("raw").unwrap().as_bytes(), raw.as_bytes());
    key(&mut tree, Key::A, false, true);
    assert_eq!(
        key(&mut tree, Key::C, false, true),
        vec![UiAction::CopyRequested { text: raw.into() }]
    );
    assert_eq!(
        key(&mut tree, Key::X, false, true),
        vec![
            UiAction::CopyRequested { text: raw.into() },
            UiAction::InputChanged {
                id: "raw".into(),
                value: String::new()
            }
        ]
    );
    assert_eq!(tree.input_value("raw"), Some(""));
    tree.handle_event(UiEvent::Text(raw.into()));
    assert_eq!(
        key(&mut tree, Key::Backspace, false, false),
        vec![UiAction::InputChanged {
            id: "raw".into(),
            value: "  ñ\r\n🦀\r終\t".into()
        }]
    );
    key(&mut tree, Key::Backspace, false, false);
    key(&mut tree, Key::Backspace, false, false);
    assert_eq!(tree.input_value("raw"), Some("  ñ\r\n🦀\r"));
}

#[test]
fn raw_textarea_visual_rows_pointer_and_selection_use_original_crlf_offsets() {
    let raw = "ñ\r\n🦀\r終";
    let mut tree = UiTree::new(editor(raw, true)).with_metrics(FontMetrics {
        char_width: 8.0,
        line_height: 20.0,
    });
    let frame = tree.layout(Size::new(320.0, 140.0));
    let painted: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(painted, ["ñ", "🦀", "終"]);
    // One visual line break consumes the original CR and LF positions. A
    // click at the start of the second line inserts after both raw bytes.
    let point = Point::new(8.0, 38.0);
    tree.handle_event(UiEvent::PointerDown(point));
    tree.handle_event(UiEvent::PointerUp(point));
    assert_eq!(
        tree.handle_event(UiEvent::Text("X".into())),
        vec![UiAction::InputChanged {
            id: "raw".into(),
            value: "ñ\r\nX🦀\r終".into()
        }]
    );
    tree.set_input_value("raw", raw);
    tree.layout(Size::new(320.0, 140.0));
    key(&mut tree, Key::Home, false, true);
    key(&mut tree, Key::ArrowRight, false, false);
    // CR and LF are one grapheme: one step selects the whole break.
    key(&mut tree, Key::ArrowRight, true, false);
    assert_eq!(
        key(&mut tree, Key::C, false, true),
        vec![UiAction::CopyRequested {
            text: "\r\n".into()
        }]
    );
    assert_eq!(
        key(&mut tree, Key::X, false, true),
        vec![
            UiAction::CopyRequested {
                text: "\r\n".into()
            },
            UiAction::InputChanged {
                id: "raw".into(),
                value: "ñ🦀\r終".into()
            }
        ]
    );
    tree.set_input_value("raw", raw);
    key(&mut tree, Key::Home, false, true);
    key(&mut tree, Key::ArrowDown, false, false);
    assert_eq!(
        tree.handle_event(UiEvent::Text("Y".into())),
        vec![UiAction::InputChanged {
            id: "raw".into(),
            value: "ñ\r\nY🦀\r終".into()
        }]
    );
}

#[test]
fn raw_textarea_element_builder_and_auto_grow_count_crlf_and_cr_as_visual_breaks() {
    let raw = "ñ\r\n🦀\r終";
    let element = Element::text_area("raw", raw, "")
        .text_area_preserve_line_endings(true)
        .text_area_auto_grow(20.0, 200.0)
        .padding(8.0)
        .size(Some(320.0), None);
    // A root fills its viewport. Measure the auto-growing child in a column.
    let mut tree = UiTree::new(Element::column(vec![element])).with_metrics(FontMetrics {
        char_width: 8.0,
        line_height: 20.0,
    });
    let frame = tree.layout(Size::new(320.0, 200.0));
    let region = frame
        .regions
        .iter()
        .find(|region| region.id == "raw")
        .unwrap();
    assert_eq!(region.rect.height, 76.0);
    assert_eq!(tree.input_value("raw"), Some(raw));
    assert!(tree.focus("raw"));
    // The arrows step over CR and LF together, as one grapheme; the host
    // can still put the cursor between them.
    tree.set_input_cursor("raw", 2);
    // The cursor lies between CR and LF. Painting uses the preceding line's
    // end without indexing past its displayed text; deleting edits the CR.
    tree.layout(Size::new(320.0, 200.0));
    assert_eq!(
        key(&mut tree, Key::Backspace, false, false),
        vec![UiAction::InputChanged {
            id: "raw".into(),
            value: "ñ\n🦀\r終".into()
        }]
    );
}

#[test]
fn default_textarea_keeps_existing_lf_normalization_and_submit_contract() {
    let mut tree = UiTree::new(editor("ñ\r\n🦀\r終", false));
    tree.layout(Size::new(320.0, 140.0));
    assert_eq!(tree.input_value("raw"), Some("ñ\n🦀\n終"));
    assert!(tree.focus("raw"));
    tree.handle_event(UiEvent::Text("\r\nA\rB".into()));
    assert_eq!(tree.input_value("raw"), Some("ñ\n🦀\n終\nA\nB"));
    assert_eq!(
        key(&mut tree, Key::Enter, false, true),
        vec![UiAction::Submitted {
            id: "raw".into(),
            value: "ñ\n🦀\n終\nA\nB".into()
        }]
    );
    key(&mut tree, Key::A, false, true);
    assert_eq!(
        key(&mut tree, Key::C, false, true),
        vec![UiAction::CopyRequested {
            text: "ñ\n🦀\n終\nA\nB".into()
        }]
    );
}
