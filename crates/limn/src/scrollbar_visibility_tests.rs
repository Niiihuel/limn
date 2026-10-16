use super::*;
use crate::{Component, ScrollArea, ScrollbarAppearance, ScrollbarVisibility};

fn root(visibility: ScrollbarVisibility) -> Element {
    ScrollArea::new(
        "rows",
        Element::column(
            (0..20)
                .map(|index| Element::button(format!("row:{index}"), "Row").size(None, Some(32.0)))
                .collect(),
        ),
    )
    .appearance(ScrollbarAppearance::overlay().buttons(false))
    .visibility(visibility)
    .render()
}

#[test]
fn scrollbars_auto_hide_until_activity_without_hiding_the_content() {
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    let frame = tree.layout_at(size, Instant::now());
    assert!(frame.regions.iter().any(|region| region.id == "row:0"));
    assert!(!frame
        .regions
        .iter()
        .any(|region| region.kind == HitKind::Scrollbar));
    assert_eq!(frame.hit_test(Point::new(215.0, 16.0)), Some("row:0"));
}

fn thumb_alpha(frame: &Frame) -> Option<u8> {
    let thumb = frame.scrollbars.get("rows")?.thumb;
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, .. } if *rect == thumb => Some(color.a),
        _ => None,
    })
}

fn scroll(tree: &mut UiTree, now: Instant) {
    tree.handle_event_at(
        UiEvent::ScrollDirect {
            position: Point::new(20.0, 60.0),
            delta_y: -32.0,
        },
        now,
    );
}

#[test]
fn scrolling_holds_then_fades_and_reschedules_only_until_hidden() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    tree.layout_at(size, now);
    assert_eq!(tree.animation_repaint_after_at(now), None);
    scroll(&mut tree, now);
    assert_eq!(thumb_alpha(&tree.layout_at(size, now)), Some(72));
    assert_eq!(
        tree.animation_repaint_after_at(now),
        Some(Duration::from_millis(600))
    );
    let half_hold = now + Duration::from_millis(300);
    assert_eq!(thumb_alpha(&tree.layout_at(size, half_hold)), Some(72));
    assert_eq!(
        tree.animation_repaint_after_at(half_hold),
        Some(Duration::from_millis(300))
    );
    let fade = now + Duration::from_millis(700);
    let alpha = thumb_alpha(&tree.layout_at(size, fade)).unwrap();
    assert!(
        alpha > 0 && alpha < 72,
        "the bar fades rather than disappearing at the deadline"
    );
    assert_eq!(tree.animation_repaint_after_at(fade), Some(Duration::ZERO));
    let end = now + Duration::from_millis(800);
    assert_eq!(thumb_alpha(&tree.layout_at(size, end)), None);
    assert_eq!(tree.animation_repaint_after_at(end), None);
    scroll(&mut tree, end);
    assert_eq!(thumb_alpha(&tree.layout_at(size, end)), Some(72));
}

#[test]
fn more_scroll_activity_restarts_the_hold_during_a_fade() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    tree.layout_at(size, now);
    scroll(&mut tree, now);
    tree.layout_at(size, now);
    let fade = now + Duration::from_millis(700);
    assert!(thumb_alpha(&tree.layout_at(size, fade)).unwrap() < 72);
    scroll(&mut tree, fade);
    assert_eq!(thumb_alpha(&tree.layout_at(size, fade)), Some(72));
    assert_eq!(
        tree.animation_repaint_after_at(fade),
        Some(Duration::from_millis(600))
    );
}

#[test]
fn reduced_motion_keeps_the_hold_but_skips_the_fade() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    tree.set_reduced_motion(true);
    tree.layout_at(size, now);
    scroll(&mut tree, now);
    assert_eq!(thumb_alpha(&tree.layout_at(size, now)), Some(72));
    assert_eq!(
        tree.animation_repaint_after_at(now),
        Some(Duration::from_millis(600))
    );
    let end = now + Duration::from_millis(600);
    assert_eq!(thumb_alpha(&tree.layout_at(size, end)), None);
    assert_eq!(tree.animation_repaint_after_at(end), None);
}

#[test]
fn scrollbar_drag_stays_visible_and_holds_after_the_pointer_leaves() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    tree.layout_at(size, now);
    scroll(&mut tree, now);
    let frame = tree.layout_at(size, now);
    let rect = frame.scrollbars["rows"].thumb;
    let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
    tree.handle_event_at(UiEvent::PointerDown(point), now);
    let held = now + Duration::from_secs(3);
    assert_eq!(thumb_alpha(&tree.layout_at(size, held)), Some(164));
    assert_eq!(tree.animation_repaint_after_at(held), None);
    tree.handle_event_at(UiEvent::PointerLeave, held);
    assert_eq!(thumb_alpha(&tree.layout_at(size, held)), Some(72));
    assert_eq!(
        tree.animation_repaint_after_at(held),
        Some(Duration::from_millis(600))
    );
}

#[test]
fn visibility_preserves_defaults_and_applies_to_virtual_lists() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut normal = UiTree::new(root(ScrollbarVisibility::Always));
    assert_eq!(thumb_alpha(&normal.layout_at(size, now)), Some(72));
    for measured in [false, true] {
        let row = |index| Element::button(format!("row:{index}"), "Row").size(None, Some(32.0));
        let list = if measured {
            ScrollArea::measured_list("rows", 20, 32.0, |index| index as u64, row)
        } else {
            ScrollArea::uniform_list("rows", 20, 32.0, row)
        };
        let mut tree = UiTree::new(
            list.appearance(ScrollbarAppearance::overlay().buttons(false))
                .visibility(ScrollbarVisibility::AutoHide {
                    hold_ms: 600,
                    fade_ms: 200,
                })
                .render(),
        );
        assert_eq!(thumb_alpha(&tree.layout_at(size, now)), None);
        scroll(&mut tree, now);
        assert_eq!(thumb_alpha(&tree.layout_at(size, now)), Some(72));
    }
}

#[test]
fn removing_a_scroll_surface_removes_its_pending_wakeup() {
    let now = Instant::now();
    let size = Size::new(220.0, 160.0);
    let mut tree = UiTree::new(root(ScrollbarVisibility::AutoHide {
        hold_ms: 600,
        fade_ms: 200,
    }));
    tree.layout_at(size, now);
    scroll(&mut tree, now);
    tree.layout_at(size, now);
    assert!(tree.animation_repaint_after_at(now).is_some());
    tree.set_root(Element::spacer());
    tree.layout_at(size, now);
    assert_eq!(tree.animation_repaint_after_at(now), None);
}
