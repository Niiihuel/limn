//! Landmark regions: F6 moves between them, and each knows its controls.
use super::*;
use crate::{Access, PopoverPlacement, Role};

const VIEWPORT: Size = Size::new(600.0, 400.0);

fn region(id: &str, name: &str, children: Vec<Element>) -> Element {
    Element::column(children).accessible(Access::new(Role::Region).id(id).name(name))
}

fn button(id: &str) -> Element {
    Element::button(id, id).size(Some(80.0), Some(28.0))
}

fn window() -> Element {
    Element::row(vec![
        region("rail", "Servers", vec![button("home"), button("server:1")]),
        // Nothing to focus: F6 passes over it.
        region("banner", "Banner", vec![Element::text("Welcome")]),
        region(
            "channels",
            "Channels",
            vec![
                button("channel:1"),
                region("voice", "Voice", vec![button("mute")]),
                button("channel:2"),
            ],
        ),
        button("outside"),
    ])
}

fn f6(tree: &mut UiTree, shift: bool) -> Vec<UiAction> {
    tree.handle_event(UiEvent::Key { key: Key::F6, shift, ctrl: false, alt: false })
}

#[test]
fn a_region_is_a_named_landmark_that_knows_its_controls_in_focus_order() {
    let mut tree = UiTree::new(window());
    let frame = tree.layout(VIEWPORT);
    let node = frame.access.iter().find(|node| node.id == "channels").expect("the region is published");
    assert_eq!((node.role, node.name.as_str(), node.actionable), (Role::Region, "Channels", false));
    assert_eq!(tree.focusables_within("rail"), ["home", "server:1"]);
    // A nested region's controls belong to the region around it too.
    assert_eq!(tree.focusables_within("channels"), ["channel:1", "mute", "channel:2"]);
    assert_eq!(tree.focusables_within("voice"), ["mute"]);
    assert!(tree.focusables_within("banner").is_empty());
    assert!(tree.focusables_within("home").is_empty(), "a control is not a region");
}

#[test]
fn f6_walks_the_regions_with_controls_and_shift_f6_walks_back() {
    let mut tree = UiTree::new(window());
    tree.layout(VIEWPORT);
    let mut reached = Vec::new();
    for _ in 0..4 {
        assert!(f6(&mut tree, false).is_empty(), "moving the focus reports nothing");
        reached.push(tree.focused_id().unwrap().to_owned());
    }
    // The banner has nothing to focus; the walk wraps to the first region.
    assert_eq!(reached, ["home", "channel:1", "mute", "home"]);
    f6(&mut tree, true);
    assert_eq!(tree.focused_id(), Some("mute"));
    assert!(tree.has_keyboard_focus());
    // From the innermost region the focus is in, not from the one around it.
    assert_eq!(tree.focus_next_region(true).as_deref(), Some("channels"));
    assert_eq!(tree.focused_id(), Some("channel:1"));
    // Focus outside every region starts from the first, or the last going back.
    assert!(tree.focus("outside"));
    assert_eq!(tree.focus_next_region(false).as_deref(), Some("rail"));
    assert!(tree.focus("outside"));
    assert_eq!(tree.focus_next_region(true).as_deref(), Some("voice"));
}

#[test]
fn f6_does_nothing_without_regions_or_while_a_popover_holds_the_focus() {
    let mut tree = UiTree::new(Element::row(vec![button("a"), button("b")]));
    tree.layout(VIEWPORT);
    assert!(tree.focus("a"));
    assert_eq!(tree.focus_next_region(false), None);
    assert_eq!(tree.focused_id(), Some("a"));

    let open = Element::popover(window(), button("ok"), "dismiss", PopoverPlacement::Center);
    let mut tree = UiTree::new(open);
    tree.layout(VIEWPORT);
    assert!(tree.focus("ok"));
    f6(&mut tree, false);
    assert_eq!(tree.focused_id(), Some("ok"));
    assert_eq!(tree.focus_next_region(false), None);
}
