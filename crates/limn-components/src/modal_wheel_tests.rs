//! A modal keeps the wheel from what it covers. This test exercises the
//! engine's scroll routing through `Modal`.
use crate::*;

#[test]
fn modal_blocks_outside_wheel_but_its_body_still_scrolls() {
    let viewport = Size::new(500.0, 340.0);
    let mut tree = UiTree::new(
        crate::Modal::new(
            Element::scroll("background", Element::spacer().size(None, Some(800.0))),
            true,
            "Scrollable dialog",
            "",
            "dismiss_modal",
            ComponentPalette::neutral_dark(),
        )
        .width(240.0)
        .height(180.0)
        .body(Element::spacer().size(None, Some(600.0)))
        // A press on the backdrop closes it; a wheel there never does.
        .backdrop_dismiss(true)
        .render(),
    );
    tree.set_reduced_motion(true);
    let frame = tree.layout(viewport);
    let outside = Point::new(10.0, 10.0);
    let surface = frame
        .regions
        .iter()
        .find(|region| region.id == "popover_surface:dismiss_modal")
        .unwrap();
    assert!(!surface.rect.contains(outside));
    let body = frame
        .regions
        .iter()
        .find(|region| {
            region.kind == HitKind::Scroll && region.id == "modal_body:dismiss_modal"
        })
        .unwrap();
    let inside = Point::new(body.rect.x + 20.0, body.rect.y + 20.0);
    for event in [
        UiEvent::Scroll {
            position: outside,
            delta_y: -40.0,
        },
        UiEvent::ScrollDirect {
            position: outside,
            delta_y: -40.0,
        },
    ] {
        assert!(tree.handle_event(event).is_empty(), "a wheel never closes a modal");
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("background"), 0.0);
        assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 0.0);
    }
    tree.handle_event(UiEvent::PointerDown(outside));
    assert_eq!(tree.handle_event(UiEvent::PointerUp(outside)), vec![UiAction::Clicked { id: "dismiss_modal".into() }]);
    tree.handle_event(UiEvent::Scroll {
        position: inside,
        delta_y: -40.0,
    });
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 40.0);
    assert_eq!(tree.scroll_offset("background"), 0.0);
    tree.handle_event(UiEvent::ScrollDirect {
        position: inside,
        delta_y: -20.0,
    });
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("modal_body:dismiss_modal"), 60.0);
    assert_eq!(tree.scroll_offset("background"), 0.0);
}
