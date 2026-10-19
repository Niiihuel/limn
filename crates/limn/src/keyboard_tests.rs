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
