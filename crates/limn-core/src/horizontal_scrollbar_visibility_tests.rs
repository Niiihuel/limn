use super::*;
use crate::{ScrollbarAppearance, ScrollbarVisibility};

const INK: Color = Color::rgb(17, 37, 57);
const SIZE: Size = Size::new(200.0, 60.0);
const AUTO: ScrollbarVisibility = ScrollbarVisibility::AutoHide {
    hold_ms: 600,
    fade_ms: 200,
};

fn strip(two_dimensional: bool, visibility: ScrollbarVisibility) -> Element {
    let content = Element::row(vec![
        Element::button("child", "Child").size(Some(80.0), Some(44.0)),
        Element::spacer().size(Some(520.0), None),
    ]);
    let strip = if two_dimensional {
        Element::horizontal_scroll_2d("strip", content)
    } else {
        Element::horizontal_scroll("strip", content)
    };
    strip
        .drag_to_pan()
        .foreground(INK)
        .scrollbar_visibility(visibility)
}

fn indicator(frame: &Frame) -> Option<u8> {
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect {
            rect,
            color,
            radius,
        } if color.r == INK.r
            && color.g == INK.g
            && color.b == INK.b
            && rect.height == 3.0
            && *radius == 1.5 =>
        {
            Some(color.a)
        }
        _ => None,
    })
}

fn scroll(tree: &mut UiTree, two_dimensional: bool, now: Instant) {
    let position = Point::new(120.0, 30.0);
    tree.handle_event_at(
        if two_dimensional {
            UiEvent::Scroll2D {
                position,
                delta_x: -40.0,
                delta_y: 0.0,
                direct: true,
                shift: false,
            }
        } else {
            UiEvent::ScrollDirect {
                position,
                delta_y: -40.0,
            }
        },
        now,
    );
}

#[test]
fn horizontal_activity_shows_then_holds_and_fades_with_a_repaint_deadline() {
    let now = Instant::now();
    for two_dimensional in [false, true] {
        let mut tree = UiTree::new(strip(two_dimensional, AUTO));
        let first = tree.layout_at(SIZE, now);
        assert_eq!(indicator(&first), None);
        assert_eq!(tree.animation_repaint_after_at(now), None);
        scroll(&mut tree, two_dimensional, now);
        assert_eq!(tree.horizontal_scroll_offset("strip"), 40.0);
        assert_eq!(indicator(&tree.layout_at(SIZE, now)), Some(114));
        assert_eq!(
            tree.animation_repaint_after_at(now),
            Some(Duration::from_millis(600))
        );
        let fade = now + Duration::from_millis(700);
        let alpha = indicator(&tree.layout_at(SIZE, fade)).unwrap();
        assert!(alpha > 0 && alpha < 114);
        assert_eq!(tree.animation_repaint_after_at(fade), Some(Duration::ZERO));
        let end = now + Duration::from_millis(800);
        assert_eq!(indicator(&tree.layout_at(SIZE, end)), None);
        assert_eq!(tree.animation_repaint_after_at(end), None);
        scroll(&mut tree, two_dimensional, end);
        assert_eq!(indicator(&tree.layout_at(SIZE, end)), Some(114));
    }
}

#[test]
fn an_invisible_horizontal_indicator_keeps_its_viewport_pan_and_child_actions() {
    let now = Instant::now();
    let mut tree = UiTree::new(strip(true, AUTO));
    let first = tree.layout_at(SIZE, now);
    assert_eq!(indicator(&first), None);
    assert_eq!(
        first.horizontal_viewports["strip"],
        Rect::new(0.0, 0.0, 200.0, 60.0)
    );
    assert_eq!(first.hit_test(Point::new(180.0, 30.0)), Some("strip"));
    tree.handle_event_at(UiEvent::PointerDown(Point::new(180.0, 30.0)), now);
    tree.handle_event_at(UiEvent::PointerMove(Point::new(140.0, 30.0)), now);
    assert_eq!(tree.horizontal_scroll_offset("strip"), 40.0);
    assert_eq!(indicator(&tree.layout_at(SIZE, now)), Some(114));
    tree.handle_event_at(UiEvent::PointerUp(Point::new(140.0, 30.0)), now);
    tree.set_horizontal_scroll_offset("strip", 0.0);
    tree.layout_at(SIZE, now);
    let idle = now + Duration::from_secs(1);
    let hidden = tree.layout_at(SIZE, idle);
    assert_eq!(indicator(&hidden), None);
    let child = Point::new(20.0, 20.0);
    assert_eq!(hidden.hit_test(child), Some("child"));
    tree.handle_event_at(UiEvent::PointerDown(child), idle);
    assert_eq!(
        tree.handle_event_at(UiEvent::PointerUp(child), idle),
        vec![UiAction::Clicked { id: "child".into() }]
    );
}

#[test]
fn a_horizontal_pan_holds_the_indicator_until_release_even_without_more_motion() {
    let now = Instant::now();
    let mut tree = UiTree::new(strip(false, AUTO));
    assert_eq!(indicator(&tree.layout_at(SIZE, now)), None);
    let point = Point::new(180.0, 30.0);
    tree.handle_event_at(UiEvent::PointerDown(point), now);
    let held = now + Duration::from_secs(3);
    assert_eq!(indicator(&tree.layout_at(SIZE, held)), Some(114));
    assert_eq!(tree.animation_repaint_after_at(held), None);
    tree.handle_event_at(UiEvent::PointerUp(point), held);
    assert_eq!(indicator(&tree.layout_at(SIZE, held)), Some(114));
    assert_eq!(
        tree.animation_repaint_after_at(held),
        Some(Duration::from_millis(600))
    );
    let end = held + Duration::from_millis(800);
    assert_eq!(indicator(&tree.layout_at(SIZE, end)), None);
    assert_eq!(tree.animation_repaint_after_at(end), None);
}

#[test]
fn reduced_motion_keeps_the_horizontal_hold_and_skips_the_fade() {
    let now = Instant::now();
    let mut tree = UiTree::new(strip(true, AUTO));
    tree.set_reduced_motion(true);
    assert_eq!(indicator(&tree.layout_at(SIZE, now)), None);
    scroll(&mut tree, true, now);
    assert_eq!(indicator(&tree.layout_at(SIZE, now)), Some(114));
    let end = now + Duration::from_millis(600);
    assert_eq!(indicator(&tree.layout_at(SIZE, end)), None);
    assert_eq!(tree.animation_repaint_after_at(end), None);
}

#[test]
fn horizontal_defaults_and_policy_changes_keep_geometry_and_remove_stale_wakeups() {
    let now = Instant::now();
    let mut tree = UiTree::new(strip(true, ScrollbarVisibility::Always));
    let plain = tree.layout_at(SIZE, now);
    assert_eq!(indicator(&plain), Some(114));
    tree.set_root(strip(true, AUTO));
    let hidden = tree.layout_at(SIZE, now);
    assert_eq!(indicator(&hidden), None);
    assert_eq!(hidden.regions, plain.regions);
    assert_eq!(hidden.access, plain.access);
    scroll(&mut tree, true, now);
    tree.layout_at(SIZE, now);
    assert_eq!(
        tree.animation_repaint_after_at(now),
        Some(Duration::from_millis(600))
    );
    tree.set_root(strip(true, ScrollbarVisibility::Always));
    assert_eq!(indicator(&tree.layout_at(SIZE, now)), Some(114));
    assert_eq!(tree.animation_repaint_after_at(now), None);
    tree.set_root(strip(true, AUTO));
    tree.layout_at(SIZE, now);
    scroll(&mut tree, true, now);
    tree.layout_at(SIZE, now);
    assert!(tree.animation_repaint_after_at(now).is_some());
    tree.set_root(Element::spacer());
    tree.layout_at(SIZE, now);
    assert_eq!(tree.animation_repaint_after_at(now), None);
}

/// The pan bar of a strip inked `ink` and given `appearance`, if any: its
/// rect, color and corner.
fn pan_bar(appearance: Option<ScrollbarAppearance>, ink: Color) -> (Rect, Color, f32) {
    let strip = strip(false, ScrollbarVisibility::Always).foreground(ink);
    let strip = match appearance {
        Some(appearance) => strip.horizontal_scrollbar_appearance(appearance),
        None => strip,
    };
    let frame = UiTree::new(strip).layout(SIZE);
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, radius } if rect.y + rect.height == SIZE.height && rect.width < SIZE.width => Some((*rect, *color, *radius)),
        _ => None,
    }).expect("a pan bar")
}

#[test]
fn a_pan_bar_given_an_appearance_takes_its_thickness_corner_tint_and_opacity() {
    let red = Color::rgb(200, 0, 0);
    let own = ScrollbarAppearance::pan_bar().thumb_width(6.0).thumb_radius(0.0).tint(red).idle_thumb_opacity(200);
    let (rect, color, radius) = pan_bar(Some(own), INK);
    assert_eq!((rect.y, rect.height, radius, color), (54.0, 6.0, 0.0, Color::rgba(200, 0, 0, 200)));
    // `own` with one field set on it, past what a builder allows.
    let changed = |set: fn(&mut ScrollbarAppearance)| {
        let mut appearance = own;
        set(&mut appearance);
        Some(appearance)
    };
    // Untinted, it takes the viewport's foreground, as a vertical bar does.
    assert_eq!(pan_bar(changed(|own| own.tint = None), INK).1, Color::rgba(INK.r, INK.g, INK.b, 200));
    // A thickness that is not finite is the pan bar's; one taller than the
    // viewport is held to it.
    assert_eq!(pan_bar(changed(|own| own.thumb_width = f32::NAN), INK).0.height, 3.0);
    assert_eq!(pan_bar(changed(|own| own.thumb_width = 500.0), INK).0.height, SIZE.height);
}

#[test]
fn the_default_pan_bar_keeps_its_look_and_pan_bar_describes_it() {
    // Without an appearance: 3 tall, 1.5 round, at 0.45 of the ink's alpha
    // by a float multiply, where an opacity of 114 out of 255 gives 89.
    let (rect, color, radius) = pan_bar(None, Color::rgba(INK.r, INK.g, INK.b, 200));
    assert_eq!((rect.height, radius, color.a), (3.0, 1.5, 90));
    // Given explicitly, `pan_bar()` paints the same bar for an opaque ink.
    assert_eq!(pan_bar(None, INK).1.a, 114);
    assert_eq!(pan_bar(Some(ScrollbarAppearance::pan_bar()), INK), pan_bar(None, INK));
}
