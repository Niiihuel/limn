//! Source positions let hosts apply each native prefix before capturing a reply.

use super::{events, events_indexed, normalize_keyboard_input, IndexedUiEvent};
use eframe::egui::{self, Event, Modifiers};
use limn::{Element, Key, PasteTarget, Point, Size, UiAction, UiEvent, UiTree};

const VIEWPORT: Size = Size::new(400.0, 240.0);

fn key(key: egui::Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

fn shortcut(keycode: egui::Key, command: bool, shift: bool) -> Event {
    key(
        keycode,
        true,
        Modifiers {
            ctrl: !command,
            command,
            shift,
            ..Default::default()
        },
    )
}

fn ui_key(key: Key, ctrl: bool, shift: bool) -> UiEvent {
    UiEvent::Key { key, ctrl, shift, alt: false }
}

fn frame(
    ctx: &egui::Context,
    input: egui::RawInput,
    origin: egui::Pos2,
) -> (Vec<Event>, Vec<IndexedUiEvent>) {
    let mut observed = None;
    let mut output = ctx.run_ui(input, |ui| {
        normalize_keyboard_input(ui.ctx());
        let source = ui.ctx().input(|input| input.events.clone());
        let indexed = events_indexed(ui.ctx(), origin);
        let projection = indexed
            .iter()
            .map(|item| item.event.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            events(ui.ctx(), origin),
            projection,
            "legacy output is the same converter's projection"
        );
        assert_eq!(
            ui.ctx().input(|input| input.events.clone()),
            source,
            "translation preserves the host snapshot"
        );
        let indices = indexed
            .iter()
            .filter_map(|item| item.source_index)
            .collect::<Vec<_>>();
        assert!(indices.windows(2).all(|pair| pair[0] <= pair[1]));
        assert!(indices.iter().all(|index| *index < source.len()));
        observed = Some((source, indexed));
    });
    output.textures_delta.clear();
    observed.unwrap()
}

fn input(source: Vec<Event>) -> (Vec<Event>, Vec<IndexedUiEvent>) {
    frame(
        &egui::Context::default(),
        egui::RawInput {
            events: source,
            ..Default::default()
        },
        egui::Pos2::ZERO,
    )
}

fn mapped(items: &[IndexedUiEvent]) -> Vec<(Option<usize>, UiEvent)> {
    items
        .iter()
        .map(|item| (item.source_index, item.event.clone()))
        .collect()
}

/// The host visits the source, including indices which translate to no UiEvent.
/// Returning true consumes a host shortcut before its ordinary limn translation.
fn walk(
    tree: &mut UiTree,
    source: &[Event],
    items: Vec<IndexedUiEvent>,
    mut host: impl FnMut(usize, &Event, &mut UiTree, &[UiAction]) -> bool,
) -> Vec<UiAction> {
    let mut items = items.into_iter().peekable();
    let mut actions = Vec::new();
    for (index, native) in source.iter().enumerate() {
        let consumed = host(index, native, tree, &actions);
        while items
            .peek()
            .is_some_and(|item| item.source_index == Some(index))
        {
            let event = items.next().unwrap().event;
            if !consumed {
                actions.extend(tree.handle_event(event));
            }
        }
    }
    // Frame-derived file metadata trails every indexed translation.
    for item in items {
        assert!(
            item.source_index.is_none(),
            "every translation was associated with its source or the derived suffix"
        );
        actions.extend(tree.handle_event(item.event));
    }
    actions
}

fn fields() -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![
        Element::text_input("a", "alpha", "").size(None, Some(32.0)),
        Element::text_input("b", "beta", "").size(None, Some(32.0)),
    ]));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    tree
}

fn area() -> UiTree {
    let mut tree = UiTree::new(Element::text_area("a", "alpha", "").size(None, Some(100.0)));
    tree.set_reduced_motion(true);
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    tree
}

fn select_all(tree: &mut UiTree) {
    assert!(tree.handle_event(ui_key(Key::A, true, false)).is_empty());
    assert_eq!(tree.input_selection("a"), Some((0, 5)));
}

fn changed(value: &str) -> UiAction {
    UiAction::InputChanged {
        id: "a".into(),
        value: value.into(),
    }
}

fn copy(value: &str) -> UiAction {
    UiAction::CopyRequested { text: value.into() }
}

fn deliver(tree: &mut UiTree, target: PasteTarget, text: &str) -> Vec<UiAction> {
    tree.handle_event(UiEvent::PasteCaptured {
        target,
        text: text.into(),
    })
}

#[test]
fn indexed_input_uses_post_normalization_indices_and_keeps_native_key_metadata() {
    let ctx = egui::Context::default();
    let alt = Modifiers {
        alt: true,
        ..Default::default()
    };
    let fallback = |pressed| Event::Key {
        key: egui::Key::Enter,
        physical_key: Some(egui::Key::Enter),
        pressed,
        repeat: false,
        modifiers: Modifiers::default(),
    };
    let mut down = key(egui::Key::ArrowDown, true, alt);
    if let Event::Key { physical_key, .. } = &mut down {
        *physical_key = Some(egui::Key::ArrowDown);
    }
    let (source, items) = frame(
        &ctx,
        egui::RawInput {
            events: vec![
                fallback(true),
                Event::Text(">".into()),
                Event::Zoom(1.25),
                down.clone(),
                down,
                key(egui::Key::ArrowDown, false, alt),
            ],
            ..Default::default()
        },
        egui::Pos2::ZERO,
    );
    assert_eq!(source.len(), 5);
    assert_eq!(source[0], Event::Text(">".into()));
    assert_eq!(source[1], Event::Zoom(1.25));
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), UiEvent::Text(">".into())),
            // Alt+Down: the translation carries Alt; repeat stays native.
            (Some(2), UiEvent::Key { key: Key::ArrowDown, ctrl: false, shift: false, alt: true }),
            (Some(3), UiEvent::Key { key: Key::ArrowDown, ctrl: false, shift: false, alt: true }),
        ]
    );
    for (index, expected_repeat) in [(2, false), (3, true)] {
        assert!(matches!(&source[index], Event::Key {
            key: egui::Key::ArrowDown,
            physical_key: Some(egui::Key::ArrowDown), pressed: true, repeat, modifiers,
        } if *repeat == expected_repeat && modifiers.alt));
    }
    assert!(
        matches!(&source[4], Event::Key { key: egui::Key::ArrowDown, pressed: false, modifiers, .. } if modifiers.alt)
    );
    assert!(!source.iter().any(|event| matches!(
        event,
        Event::Key {
            key: egui::Key::Enter,
            ..
        }
    )));
    let (next, items) = frame(
        &ctx,
        egui::RawInput {
            events: vec![fallback(false), Event::Zoom(1.0), Event::Text("界".into())],
            ..Default::default()
        },
        egui::Pos2::ZERO,
    );
    assert_eq!(next, vec![Event::Zoom(1.0), Event::Text("界".into())]);
    assert_eq!(mapped(&items), vec![(Some(1), UiEvent::Text("界".into()))]);
}

#[test]
fn indexed_input_host_visits_unknown_markers_before_or_after_navigation() {
    let alt = Modifiers {
        alt: true,
        ..Default::default()
    };
    for (native, captured_id, final_id, accepted) in [
        (
            vec![Event::Zoom(1.0), key(egui::Key::ArrowDown, true, alt)],
            "a",
            "b",
            false,
        ),
        (
            vec![key(egui::Key::ArrowDown, true, alt), Event::Zoom(1.0)],
            "b",
            "b",
            true,
        ),
        (
            vec![
                Event::Zoom(1.0),
                key(egui::Key::ArrowDown, true, alt),
                key(egui::Key::ArrowUp, true, alt),
            ],
            "a",
            "a",
            false,
        ),
    ] {
        let (source, items) = input(native);
        let mut tree = fields();
        let mut visited = Vec::new();
        let mut captured = None;
        let actions = walk(&mut tree, &source, items, |index, event, tree, _| {
            visited.push(index);
            match event {
                Event::Zoom(_) => {
                    assert_eq!(tree.focused_id(), Some(captured_id));
                    captured = Some(
                        tree.capture_paste_target()
                            .expect("current laid-out editor"),
                    );
                    false
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.alt && matches!(key, egui::Key::ArrowDown | egui::Key::ArrowUp) => {
                    assert!(tree.focus(if *key == egui::Key::ArrowDown {
                        "b"
                    } else {
                        "a"
                    }));
                    tree.layout(VIEWPORT);
                    true
                }
                _ => false,
            }
        });
        assert_eq!(visited, (0..source.len()).collect::<Vec<_>>());
        assert!(actions.is_empty());
        assert_eq!(tree.focused_id(), Some(final_id));
        let target = captured.expect("the unknown marker was visited");
        let actions = deliver(&mut tree, target, "!");
        if accepted {
            assert_eq!(
                actions,
                vec![UiAction::InputChanged {
                    id: "b".into(),
                    value: "beta!".into()
                }]
            );
            assert_eq!(tree.input_value("b"), Some("beta!"));
        } else {
            assert!(actions.is_empty());
            assert_eq!(tree.input_value("b"), Some("beta"));
            let fresh = tree.capture_paste_target().unwrap();
            assert!(!deliver(&mut tree, fresh, "!").is_empty());
        }
        assert_eq!(
            tree.input_value("a"),
            Some(if final_id == "a" { "alpha!" } else { "alpha" })
        );
    }
}

#[test]
fn indexed_input_deduplicates_only_forward_adjacent_nonoverlapping_pairs() {
    for newline in ["\n", "\r", "\r\n"] {
        let (source, items) = input(vec![
            key(egui::Key::Enter, true, Modifiers::default()),
            Event::Text(newline.into()),
        ]);
        assert_eq!(
            mapped(&items),
            vec![(Some(0), ui_key(Key::Enter, false, false))]
        );
        let mut tree = area();
        assert_eq!(
            walk(&mut tree, &source, items, |_, _, _, _| false),
            vec![changed("alpha\n")]
        );
        assert_eq!(tree.input_selection("a"), Some((6, 6)));
    }
    for command in [false, true] {
        let (source, items) = input(vec![shortcut(egui::Key::C, command, true), Event::Copy]);
        assert_eq!(mapped(&items), vec![(Some(0), ui_key(Key::C, true, true))]);
        let mut tree = area();
        select_all(&mut tree);
        assert_eq!(
            walk(&mut tree, &source, items, |_, _, _, _| false),
            vec![copy("alpha")]
        );
        assert_eq!(tree.input_value("a"), Some("alpha"));
        assert_eq!(tree.input_selection("a"), Some((0, 5)));

        let (source, items) = input(vec![shortcut(egui::Key::X, command, true), Event::Cut]);
        assert_eq!(mapped(&items), vec![(Some(0), ui_key(Key::X, true, true))]);
        let mut tree = area();
        select_all(&mut tree);
        assert_eq!(
            walk(&mut tree, &source, items, |_, _, _, _| false),
            vec![copy("alpha"), changed("")]
        );
        assert_eq!(tree.input_selection("a"), Some((0, 0)));

        let payload = "ñ 東京\r\n";
        let (source, items) = input(vec![
            shortcut(egui::Key::V, command, false),
            Event::Paste(payload.into()),
        ]);
        assert_eq!(
            mapped(&items),
            vec![(Some(1), UiEvent::Paste(payload.into()))]
        );
        let mut tree = area();
        select_all(&mut tree);
        assert_eq!(
            walk(&mut tree, &source, items, |_, _, _, _| false),
            vec![changed("ñ 東京\n")]
        );
        assert_eq!(tree.input_value("a"), Some("ñ 東京\n"));
        assert_eq!(tree.input_selection("a"), Some((5, 5)));
    }

    let (source, items) = input(vec![
        Event::Text("\n".into()),
        Event::Zoom(1.0),
        key(egui::Key::Enter, true, Modifiers::default()),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), UiEvent::Text("\n".into())),
            (Some(2), ui_key(Key::Enter, false, false))
        ]
    );
    let mut tree = area();
    let mut prefix = None;
    let mut captured = None;
    let actions = walk(&mut tree, &source, items, |_, native, tree, actions| {
        if matches!(native, Event::Zoom(_)) {
            prefix = Some((tree.input_value("a").unwrap().to_owned(), actions.to_vec()));
            captured = tree.capture_paste_target();
        }
        false
    });
    assert_eq!(prefix, Some(("alpha\n".into(), vec![changed("alpha\n")])));
    assert_eq!(actions, vec![changed("alpha\n"), changed("alpha\n\n")]);
    assert!(deliver(&mut tree, captured.unwrap(), "late").is_empty());
    assert_eq!(tree.input_value("a"), Some("alpha\n\n"));

    let (source, items) = input(vec![
        Event::Copy,
        Event::Zoom(1.0),
        shortcut(egui::Key::C, false, true),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::C, true, false)),
            (Some(2), ui_key(Key::C, true, true))
        ]
    );
    let mut tree = area();
    select_all(&mut tree);
    let mut prefix = None;
    let actions = walk(&mut tree, &source, items, |_, native, tree, actions| {
        if matches!(native, Event::Zoom(_)) {
            prefix = Some((tree.input_selection("a"), actions.to_vec()));
        }
        false
    });
    assert_eq!(prefix, Some((Some((0, 5)), vec![copy("alpha")])));
    assert_eq!(actions, vec![copy("alpha"), copy("alpha")]);

    let (source, items) = input(vec![
        shortcut(egui::Key::C, false, true),
        Event::Copy,
        shortcut(egui::Key::C, false, false),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::C, true, true)),
            (Some(2), ui_key(Key::C, true, false))
        ]
    );
    let mut tree = area();
    select_all(&mut tree);
    assert_eq!(
        walk(&mut tree, &source, items, |_, _, _, _| false),
        vec![copy("alpha"), copy("alpha")]
    );

    // Reversed pairs and mismatches retain both translations.
    let (source, items) = input(vec![
        Event::Text("\n".into()),
        key(egui::Key::Enter, true, Modifiers::default()),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), UiEvent::Text("\n".into())),
            (Some(1), ui_key(Key::Enter, false, false))
        ]
    );
    assert_eq!(
        walk(&mut area(), &source, items, |_, _, _, _| false),
        vec![changed("alpha\n"), changed("alpha\n\n")]
    );

    let (source, items) = input(vec![Event::Copy, shortcut(egui::Key::C, false, true)]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::C, true, false)),
            (Some(1), ui_key(Key::C, true, true))
        ]
    );
    let mut tree = area();
    select_all(&mut tree);
    assert_eq!(
        walk(&mut tree, &source, items, |_, _, _, _| false),
        vec![copy("alpha"), copy("alpha")]
    );

    let (source, items) = input(vec![
        Event::Paste("界".into()),
        shortcut(egui::Key::V, false, false),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), UiEvent::Paste("界".into())),
            (Some(1), ui_key(Key::V, true, false))
        ]
    );
    let mut tree = area();
    assert_eq!(
        walk(&mut tree, &source, items, |_, _, _, _| false),
        vec![changed("alpha界")]
    );

    let (source, items) = input(vec![
        key(egui::Key::C, true, Modifiers::default()),
        Event::Copy,
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::C, false, false)),
            (Some(1), ui_key(Key::C, true, false))
        ]
    );
    let mut tree = area();
    select_all(&mut tree);
    assert_eq!(
        walk(&mut tree, &source, items, |_, _, _, _| false),
        vec![copy("alpha")]
    );

    let (source, items) = input(vec![shortcut(egui::Key::C, false, false), Event::Cut]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::C, true, false)),
            (Some(1), ui_key(Key::X, true, false))
        ]
    );
    let mut tree = area();
    select_all(&mut tree);
    assert_eq!(
        walk(&mut tree, &source, items, |_, _, _, _| false),
        vec![copy("alpha"), copy("alpha"), changed("")]
    );

    let (source, items) = input(vec![
        shortcut(egui::Key::V, false, false),
        Event::Zoom(1.0),
        Event::Paste("界".into()),
    ]);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), ui_key(Key::V, true, false)),
            (Some(2), UiEvent::Paste("界".into()))
        ]
    );
    let mut tree = area();
    let mut prefix = None;
    let actions = walk(&mut tree, &source, items, |_, native, tree, actions| {
        if matches!(native, Event::Zoom(_)) {
            prefix = Some((tree.input_value("a").unwrap().to_owned(), actions.to_vec()));
        }
        false
    });
    assert_eq!(prefix, Some(("alpha".into(), vec![])));
    assert_eq!(actions, vec![changed("alpha界")]);
}

#[test]
fn indexed_input_preserves_mixed_source_order_and_marks_frame_derived_files() {
    let ctx = egui::Context::default();
    let native = vec![
        Event::PointerMoved(egui::Pos2::new(110.0, 60.0)),
        Event::Text("é".into()),
        Event::Zoom(1.0),
        Event::Ime(egui::ImeEvent::Preedit {
            text: "仮".into(),
            active_range_chars: Some(0..1),
        }),
        Event::Ime(egui::ImeEvent::Commit("界".into())),
        Event::WindowFocused(false),
        Event::PointerGone,
        Event::WindowFocused(true),
        Event::Text("!".into()),
    ];
    let (source, items) = frame(
        &ctx,
        egui::RawInput {
            events: native.clone(),
            ..Default::default()
        },
        egui::Pos2::new(100.0, 50.0),
    );
    assert_eq!(source, native);
    assert_eq!(
        mapped(&items),
        vec![
            (Some(0), UiEvent::PointerMove(Point::new(10.0, 10.0))),
            (Some(1), UiEvent::Text("é".into())),
            (Some(3), UiEvent::ImePreedit("仮".into())),
            (Some(4), UiEvent::ImeCommit("界".into())),
            (Some(6), UiEvent::PointerLeave),
            (Some(8), UiEvent::Text("!".into())),
        ]
    );
    let mut tree = fields();
    let mut visited = Vec::new();
    let mut focus_events = Vec::new();
    let mut target = None;
    let actions = walk(&mut tree, &source, items, |index, native, tree, _| {
        visited.push(index);
        match native {
            Event::WindowFocused(focused) => {
                focus_events.push((index, *focused));
                tree.set_window_focused(*focused);
            }
            Event::Zoom(_) => target = tree.capture_paste_target(),
            _ => {}
        }
        false
    });
    assert_eq!(visited, (0..9).collect::<Vec<_>>());
    assert_eq!(focus_events, vec![(5, false), (7, true)]);
    assert_eq!(
        actions,
        vec![changed("alphaé"), changed("alphaé界"), changed("alphaé界!")]
    );
    assert_eq!(tree.input_value("a"), Some("alphaé界!"));
    assert!(deliver(&mut tree, target.unwrap(), "late").is_empty());
    let fresh = tree.capture_paste_target().unwrap();
    assert_eq!(deliver(&mut tree, fresh, "+"), vec![changed("alphaé界!+")]);

    let position = Point::new(10.0, 20.0);
    let native = vec![
        Event::Zoom(1.0),
        Event::PointerMoved(egui::Pos2::new(110.0, 70.0)),
    ];
    let (_, hovered) = frame(
        &egui::Context::default(),
        egui::RawInput {
            events: native.clone(),
            hovered_files: vec![egui::HoveredFile {
                path: Some("/metadata/hover.txt".into()),
                ..Default::default()
            }],
            ..Default::default()
        },
        egui::Pos2::new(100.0, 50.0),
    );
    assert_eq!(
        mapped(&hovered),
        vec![
            (Some(1), UiEvent::PointerMove(position)),
            (None, UiEvent::FilesHovered { position })
        ]
    );

    #[cfg(not(target_arch = "wasm32"))]
    {
        #[derive(Debug)]
        struct MetadataFile(std::path::PathBuf);
        impl egui::DroppedFile for MetadataFile {
            fn path(&self) -> &std::path::Path {
                &self.0
            }
            fn bytes(&self) -> Result<Vec<u8>, String> {
                panic!("event translation must not read file contents")
            }
        }
        let path = std::path::PathBuf::from("/metadata/drop.txt");
        let (_, dropped) = frame(
            &egui::Context::default(),
            egui::RawInput {
                events: native,
                dropped_files: vec![std::sync::Arc::new(MetadataFile(path.clone()))],
                hovered_files: vec![egui::HoveredFile {
                    path: Some("/metadata/hover.txt".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            egui::Pos2::new(100.0, 50.0),
        );
        assert_eq!(
            mapped(&dropped),
            vec![
                (Some(1), UiEvent::PointerMove(position)),
                (
                    None,
                    UiEvent::FilesDropped {
                        position,
                        paths: vec![path]
                    }
                )
            ]
        );
    }
}
