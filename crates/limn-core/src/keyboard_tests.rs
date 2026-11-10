use super::*;

fn press(key: Key, shift: bool, ctrl: bool) -> UiEvent {
    UiEvent::Key { key, shift, ctrl, alt: false }
}

#[test]
fn every_key_reads_back_from_its_name_its_variant_and_its_character() {
    let mut names = std::collections::HashSet::new();
    for &key in Key::ALL {
        assert!(names.insert(key.name()), "{key:?} shares its name");
        assert_eq!(Key::from_name(key.name()), Some(key));
        assert_eq!(Key::from_name(&format!("{key:?}")), Some(key));
    }
    assert_eq!(Key::ALL.len(), 105);
    for (name, key) in [
        ("/", Key::Slash),
        ("-", Key::Minus),
        ("'", Key::Quote),
        ("\\", Key::Backslash),
        (" ", Key::Space),
        ("1", Key::Num1),
        ("a", Key::A),
        ("up", Key::ArrowUp),
        ("F35", Key::F35),
    ] {
        assert_eq!(Key::from_name(name), Some(key), "{name:?}");
    }
    for nonsense in ["", "Nothing", "Shift", "F36", "<<"] {
        assert_eq!(Key::from_name(nonsense), None, "{nonsense:?}");
    }
    assert_eq!((Key::ArrowDown.name(), Key::Num0.name(), Key::Backtick.name()), ("Down", "0", "Backtick"));
}

#[test]
fn a_key_is_held_from_its_press_to_its_release_and_the_modifiers_follow_the_events() {
    let mut tree = UiTree::new(Element::button("save", "Save"));
    tree.layout(Size::new(200.0, 80.0));
    assert!(!tree.key_held(Key::T));
    tree.handle_event(press(Key::T, false, true));
    assert!(tree.key_held(Key::T));
    assert_eq!(tree.modifiers(), Modifiers { ctrl: true, ..Default::default() });
    // A repeat keeps it held once; its release lets go of it alone.
    tree.handle_event(press(Key::T, false, true));
    tree.handle_event(press(Key::Num1, false, true));
    assert!(tree.handle_event(UiEvent::KeyReleased { key: Key::T }).is_empty());
    assert!(!tree.key_held(Key::T) && tree.key_held(Key::Num1));
    // Shift alone has no key of its own: its change says it.
    let shift = Modifiers { shift: true, ..Default::default() };
    assert!(tree.handle_event(UiEvent::ModifiersChanged(shift)).is_empty());
    assert_eq!(tree.modifiers(), shift);
    tree.handle_event(UiEvent::ModifiersChanged(Modifiers::default()));
    assert_eq!(tree.modifiers(), Modifiers::default());
}

#[test]
fn losing_the_window_focus_lets_go_of_every_key_and_modifier() {
    let mut tree = UiTree::new(Element::button("save", "Save"));
    tree.layout(Size::new(200.0, 80.0));
    tree.handle_event(press(Key::F13, true, false));
    tree.set_window_focused(false);
    assert!(!tree.key_held(Key::F13));
    assert_eq!(tree.modifiers(), Modifiers::default());
    // Back in focus, nothing is held until pressed again.
    tree.set_window_focused(true);
    assert!(!tree.key_held(Key::F13));
}

#[test]
fn a_key_the_host_takes_first_is_still_known_as_held() {
    let mut tree = UiTree::new(Element::button("save", "Save"));
    tree.layout(Size::new(200.0, 80.0));
    let space = press(Key::Space, false, false);
    tree.observe_keys(&space);
    assert!(tree.key_held(Key::Space));
    // Noting it again, or handling it after, changes nothing.
    tree.observe_keys(&space);
    tree.observe_keys(&UiEvent::KeyReleased { key: Key::Space });
    assert!(!tree.key_held(Key::Space));
    tree.observe_keys(&UiEvent::KeyReleased { key: Key::Space });
    assert!(!tree.key_held(Key::Space));
}

#[test]
fn a_release_or_a_change_of_modifiers_activates_nothing() {
    let mut tree = UiTree::new(Element::button("save", "Save"));
    tree.layout(Size::new(200.0, 80.0));
    tree.focus("save");
    for event in [
        UiEvent::KeyReleased { key: Key::Enter },
        UiEvent::KeyReleased { key: Key::Space },
        UiEvent::ModifiersChanged(Modifiers { shift: true, ctrl: true, alt: true }),
    ] {
        assert!(tree.handle_event(event).is_empty());
    }
    assert_eq!(tree.focused_id(), Some("save"));
    // A new key reaches the focused control as any key does: a button
    // answers only Enter and Space.
    assert!(tree.handle_event(press(Key::Num5, false, false)).is_empty());
    assert_eq!(tree.handle_event(press(Key::Enter, false, false)), vec![UiAction::Clicked { id: "save".into() }]);
}

#[test]
fn a_focus_asked_for_before_its_control_is_laid_out_lands_in_the_layout_that_adds_it() {
    let size = Size::new(320.0, 200.0);
    let now = Instant::now();
    let closed = || Element::column(vec![Element::button("open", "Open")]);
    let open = || Element::column(vec![Element::button("open", "Open"), Element::text_input("search", "", "Search")]);

    // The host's update opens the launcher and focuses its field before laying it out.
    let mut tree = UiTree::new(closed());
    tree.layout_at(size, now);
    assert!(!tree.focus("search"));
    assert_eq!(tree.focused_id(), None);
    tree.set_root(open());
    let deferred = tree.layout_at(size, now);
    assert_eq!(tree.focused_id(), Some("search"));
    assert!(tree.has_keyboard_focus());

    // The same layout paints it as a focus given after a layout does.
    let mut eager = UiTree::new(open());
    eager.layout_at(size, now);
    assert!(eager.focus("search"));
    assert_eq!(eager.layout_at(size, now).commands, deferred.commands);
}

#[test]
fn a_focus_request_lasts_one_layout_and_yields_to_a_later_focus_or_clear() {
    let size = Size::new(320.0, 200.0);
    let with = |ids: &[&str]| Element::column(ids.iter().map(|id| Element::button(*id, *id)).collect());
    let mut tree = UiTree::new(with(&["a"]));
    tree.layout(size);

    assert!(!tree.focus("ghost"));
    tree.layout(size);
    tree.set_root(with(&["a", "ghost"]));
    tree.layout(size);
    assert_eq!(tree.focused_id(), None, "a layout without it drops the request");

    tree.set_root(with(&["a"]));
    tree.layout(size);
    assert!(!tree.focus("later"));
    assert!(tree.focus("a"));
    tree.set_root(with(&["a", "later"]));
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("a"), "a focus that landed replaces the request");

    assert!(!tree.focus("next"));
    tree.clear_focus();
    tree.set_root(with(&["a", "next"]));
    tree.layout(size);
    assert_eq!(tree.focused_id(), None, "clear_focus drops it too");
}

#[test]
fn a_focus_asked_for_in_a_virtual_list_row_lands_once_the_row_is_placed() {
    let size = Size::new(320.0, 200.0);
    let list = || Element::uniform_list("rows", 50, 24.0, |index| Element::button(format!("row:{index}"), "Row"));
    let mut tree = UiTree::new(Element::spacer());
    tree.layout(size);
    assert!(!tree.focus("row:2"));
    tree.set_root(list());
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("row:2"));
}

#[test]
fn a_pointer_only_clickable_answers_a_press_but_not_tab_focus_or_the_platform() {
    let size = Size::new(320.0, 240.0);
    let root = Element::column(vec![
        Element::button("close", "Close"),
        Element::clickable("picture", Element::spacer().size(Some(200.0), Some(120.0)))
            .pointer_only()
            .dispatch_as("close"),
        Element::button("next", "Next"),
    ]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(size);
    assert!(frame.duplicate_ids().is_empty());
    assert!(frame.access.iter().all(|node| node.id != "picture"), "the platform is not told of it");

    let picture = frame.regions.iter().find(|region| region.id == "picture").unwrap().rect;
    let inside = Point::new(picture.x + 20.0, picture.y + 20.0);
    tree.handle_event(UiEvent::PointerMove(inside));
    assert_eq!(tree.hovered_id(), Some("picture"), "its hover is its own");
    assert!(tree.focus("close"));
    tree.handle_event(UiEvent::PointerDown(inside));
    assert_eq!(tree.focused_id(), None, "a press takes the focus away, as one on the page does");
    assert_eq!(tree.handle_event(UiEvent::PointerUp(inside)), vec![UiAction::Clicked { id: "close".into() }]);

    tree.layout(size);
    assert!(tree.focus("close"));
    tree.handle_event(press(Key::Tab, false, false));
    assert_eq!(tree.focused_id(), Some("next"), "Tab goes past it");
}
