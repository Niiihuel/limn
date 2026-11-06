use super::*;
use crate::{Frame, HitKind, Key, Rect, Role, Size, UiEvent, UiTree};

fn sheet(open: bool) -> Element {
    BottomSheet::new(
        Element::button("behind", "Open actions"),
        open,
        "Song actions",
        "Choose what to do with this song.",
        "dismiss",
        ComponentPalette::neutral_dark(),
    )
    .max_height(400.0)
    .body(Element::column(vec![
        Element::text_input("first", "", "Name"),
        Element::spacer().size(None, Some(600.0)),
        Element::text_input("last", "", "Last field"),
    ]))
    .footer(Element::button("save", "Save").size(None, Some(44.0)))
    .initial_focus("first")
    .render()
}

fn bounds(frame: &Frame, id: &str) -> Rect {
    frame
        .regions
        .iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("missing {id}"))
        .rect
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
        alt: false,
    }
}

fn click(tree: &mut UiTree, point: Point) -> Vec<UiAction> {
    let mut actions = tree.handle_event(UiEvent::PointerDown(point));
    actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
    actions
}

#[test]
fn sheet_meets_bottom_at_phone_widths_and_has_named_touch_targets() {
    let mut tree = UiTree::new(sheet(true));
    tree.set_reduced_motion(true);
    for width in [320.0, 390.0, 430.0, 900.0] {
        let frame = tree.layout(Size::new(width, 700.0));
        let card = bounds(&frame, "popover_surface:dismiss");
        assert_eq!(
            card,
            Rect::new(
                (width - width.min(560.0)) / 2.0,
                300.0,
                width.min(560.0),
                400.0
            )
        );
        let dialog = frame
            .access
            .iter()
            .find(|a| a.id == "dismiss:dialog")
            .unwrap();
        assert_eq!(
            (dialog.role, dialog.name.as_str()),
            (Role::Dialog, "Song actions")
        );
        assert_eq!(
            dialog.description.as_deref(),
            Some("Choose what to do with this song.")
        );
        for id in ["dismiss:close", "dismiss:drag"] {
            let target = frame.access.iter().find(|a| a.id == id).unwrap();
            assert_eq!(
                (target.role, target.name.as_str()),
                (Role::Button, crate::labels().close)
            );
            assert!(target.rect.width >= 44.0 && target.rect.height >= 44.0);
        }
    }
}

#[test]
fn short_content_fits_while_long_body_scrolls_below_fixed_header_and_above_footer() {
    let mut tree = UiTree::new(sheet(true));
    tree.set_reduced_motion(true);
    let size = Size::new(320.0, 300.0);
    let before = tree.layout(size);
    let card = bounds(&before, "popover_surface:dismiss");
    assert_eq!(card.y, 8.0);
    let close = bounds(&before, "dismiss:close");
    let footer = bounds(&before, "save");
    assert!(close.y >= card.y && footer.y + footer.height <= 300.0);
    let scroll = before
        .regions
        .iter()
        .find(|r| r.id == "modal_body:dismiss" && r.kind == HitKind::Scroll)
        .unwrap()
        .rect;
    assert!(scroll.y >= close.y + close.height && scroll.y + scroll.height <= footer.y);
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(scroll.x + 10.0, scroll.y + 10.0),
        delta_y: -160.0,
    });
    let after = tree.layout(size);
    assert!(tree.scroll_offset("modal_body:dismiss") > 0.0);
    assert_eq!(bounds(&after, "dismiss:close"), close);
    assert_eq!(bounds(&after, "save"), footer);

    tree.set_root(
        BottomSheet::new(
            Element::spacer(),
            true,
            "Short",
            "",
            "short",
            ComponentPalette::neutral_dark(),
        )
        .body(Element::text("One action"))
        .max_height(400.0)
        .render(),
    );
    let frame = tree.layout(Size::new(390.0, 700.0));
    let card = bounds(&frame, "popover_surface:short");
    assert!(card.height < 400.0);
    assert_eq!(card.y + card.height, 700.0);
}

#[test]
fn sheet_traps_and_restores_focus_and_reveals_offscreen_fields() {
    let size = Size::new(390.0, 700.0);
    let mut tree = UiTree::new(sheet(false));
    tree.set_reduced_motion(true);
    tree.layout(size);
    assert!(tree.focus("behind"));
    tree.set_root(sheet(true));
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("first"));
    tree.handle_event(key(Key::Tab));
    assert_eq!(tree.focused_id(), Some("last"));
    let frame = tree.layout(size);
    assert!(tree.scroll_offset("modal_body:dismiss") > 0.0);
    assert!(frame.regions.iter().any(|r| r.id == "last"));
    for _ in 0..8 {
        tree.handle_event(key(Key::Tab));
        assert_ne!(tree.focused_id(), Some("behind"));
    }
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
    tree.set_root(sheet(false));
    let frame = tree.layout(size);
    assert_eq!(tree.focused_id(), Some("behind"));
    assert!(!frame.regions.iter().any(|r| r.id == "dismiss:close"));
}

#[test]
fn scrim_blocks_underlying_clicks_and_obeys_dismiss_policy() {
    for dismiss in [false, true] {
        let mut tree = UiTree::new(
            BottomSheet::new(
                Element::button("behind", "Behind"),
                true,
                "Actions",
                "",
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .body(Element::text("One action"))
            .backdrop_dismiss(dismiss)
            .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(390.0, 700.0));
        assert_ne!(frame.hit_test(Point::new(8.0, 8.0)), Some("behind"));
        let actions = click(&mut tree, Point::new(8.0, 8.0));
        assert_eq!(
            actions,
            if dismiss {
                vec![UiAction::Clicked {
                    id: "dismiss".into(),
                }]
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn safe_area_padding_keeps_footer_above_home_indicator_without_moving_bottom_edge() {
    let mut tree = UiTree::new(
        BottomSheet::new(
            Element::spacer(),
            true,
            "Actions",
            "",
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .body(Element::spacer().size(None, Some(800.0)))
        .footer(Element::button("save", "Save").size(None, Some(44.0)))
        .max_height(400.0)
        .safe_area_bottom(34.0)
        .render(),
    );
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(390.0, 700.0));
    let card = bounds(&frame, "popover_surface:dismiss");
    let footer = bounds(&frame, "save");
    assert_eq!(card.y + card.height, 700.0);
    assert!(footer.y + footer.height <= 700.0 - 34.0);
}

#[test]
fn small_requested_heights_and_short_safe_viewports_keep_the_footer_and_close_usable() {
    for height in [700.0, 230.0, 200.0] {
        let mut tree = UiTree::new(
            BottomSheet::new(
                Element::spacer(),
                true,
                "Actions",
                "A short description.",
                "dismiss",
                ComponentPalette::neutral_dark(),
            )
            .body(Element::spacer().size(None, Some(800.0)))
            .footer(Element::button("save", "Save").size(None, Some(44.0)))
            .max_height(160.0)
            .safe_area_bottom(34.0)
            .render(),
        );
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(390.0, height));
        let close = bounds(&frame, "dismiss:close");
        let footer = bounds(&frame, "save");
        assert_eq!(close.height, 44.0, "height {height}");
        assert_eq!(footer.height, 44.0, "height {height}");
        assert!(close.y + close.height <= footer.y, "height {height}");
        assert!(footer.y + footer.height <= height - 34.0, "height {height}");
    }
}

#[test]
fn drag_handle_requests_close_once_on_downward_release_and_clicks_remain_accessible() {
    let mut tree = UiTree::new(sheet(true));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(390.0, 700.0));
    let handle = bounds(&frame, "dismiss:drag");
    let start = Point::new(
        handle.x + handle.width / 2.0,
        handle.y + handle.height / 2.0,
    );
    // A slow drag past a third of the sheet, at most 180 points, closes it.
    let end = Point::new(start.x, start.y + 200.0);
    let mut drag = BottomSheetDrag::new("dismiss");
    assert!(tree.handle_event(UiEvent::PointerDown(start)).is_empty());
    let actions = tree.handle_event(UiEvent::PointerMove(end));
    assert!(actions
        .iter()
        .any(|a| matches!(a, UiAction::DragStarted { id, .. } if id == "dismiss:drag")));
    assert!(actions.iter().all(|a| !drag.handle_action(a)));
    let release = tree.handle_event(UiEvent::PointerUp(end));
    assert_eq!(release.iter().filter(|a| drag.handle_action(a)).count(), 1);
    assert!(
        !drag.handle_action(&release[0]),
        "a release is consumed once"
    );
    assert_eq!(
        click(&mut tree, start),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
    assert!(tree.focus("dismiss:drag"));
    assert_eq!(
        tree.handle_event(key(Key::Enter)),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
}

#[test]
fn short_upward_sideways_cancelled_and_unrelated_drags_do_not_close() {
    let start = Point::new(100.0, 200.0);
    for end in [
        Point::new(100.0, 263.0),
        Point::new(100.0, 100.0),
        Point::new(240.0, 280.0),
    ] {
        let mut drag = BottomSheetDrag::new("dismiss");
        assert!(!drag.handle_action(&UiAction::DragStarted {
            id: "dismiss:drag".into(),
            position: start
        }));
        assert!(!drag.handle_action(&UiAction::Dropped {
            id: "dismiss:drag".into(),
            position: end,
            target: None
        }));
    }
    for cancel in [
        UiAction::DragCancelled {
            id: "dismiss:drag".into(),
        },
        UiAction::Clicked {
            id: "dismiss".into(),
        },
    ] {
        let mut drag = BottomSheetDrag::new("dismiss");
        drag.handle_action(&UiAction::DragStarted {
            id: "dismiss:drag".into(),
            position: start,
        });
        drag.handle_action(&cancel);
        assert!(!drag.handle_action(&UiAction::Dropped {
            id: "dismiss:drag".into(),
            position: Point::new(100.0, 300.0),
            target: None
        }));
    }
    let mut drag = BottomSheetDrag::new("dismiss");
    drag.handle_action(&UiAction::DragStarted {
        id: "body".into(),
        position: start,
    });
    assert!(!drag.handle_action(&UiAction::Dropped {
        id: "body".into(),
        position: Point::new(100.0, 300.0),
        target: None
    }));
}

#[test]
fn interrupted_sheet_transitions_keep_opacity_and_closed_sheets_have_no_hit_targets() {
    use std::time::{Duration, Instant};
    let palette = ComponentPalette::neutral_dark();
    let open = || {
        BottomSheet::new(Element::spacer(), true, "Actions", "", "dismiss", palette)
            .body(Element::text("One action"))
            .animation_ms(200)
            .render()
    };
    let alpha = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                crate::DrawCommand::FillRect { color, .. }
                | crate::DrawCommand::FillCornerRect { color, .. }
                    if (color.r, color.g, color.b)
                        == (palette.surface.r, palette.surface.g, palette.surface.b) =>
                {
                    Some(color.a)
                }
                _ => None,
            })
            .expect("sheet paint")
    };
    let start = Instant::now();
    let size = Size::new(390.0, 700.0);
    let mut tree = UiTree::new(open());
    tree.layout_at(size, start);
    let entering = tree.layout_at(size, start + Duration::from_millis(40));
    let visible = alpha(&entering);
    assert!(visible > 0 && visible < 255);
    tree.set_root(Element::spacer());
    let closing = tree.layout_at(size, start + Duration::from_millis(40));
    assert_eq!(alpha(&closing), visible);
    assert!(!closing.regions.iter().any(|r| r.id.starts_with("dismiss")));
    let fading = tree.layout_at(size, start + Duration::from_millis(80));
    assert!(alpha(&fading) < visible);
    tree.set_root(open());
    let reopened = tree.layout_at(size, start + Duration::from_millis(80));
    assert_eq!(alpha(&reopened), alpha(&fading));
    assert!(reopened.regions.iter().any(|r| r.id == "dismiss:close"));
    tree.set_reduced_motion(true);
    assert_eq!(
        alpha(&tree.layout_at(size, start + Duration::from_millis(81))),
        255
    );
}

#[test]
fn explicit_return_focus_and_close_button_work_without_drag_handler_state() {
    let size = Size::new(390.0, 700.0);
    let mut tree = UiTree::new(Element::button("temporary", "Menu item"));
    tree.set_reduced_motion(true);
    tree.layout(size);
    assert!(tree.focus("temporary"));
    tree.set_root(
        BottomSheet::new(
            Element::button("opener", "Open actions"),
            true,
            "Actions",
            "",
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .body(Element::text("One action"))
        .return_focus("opener")
        .render(),
    );
    let frame = tree.layout(size);
    assert_eq!(tree.focused_id(), Some("dismiss:close"));
    let close = bounds(&frame, "dismiss:close");
    assert_eq!(
        click(&mut tree, Point::new(close.x + 2.0, close.y + 2.0)),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
    tree.set_root(Element::button("opener", "Open actions"));
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("opener"));
}

#[test]
fn glass_material_belongs_to_the_sheet_shell_and_keeps_close_action() {
    let palette = ComponentPalette::neutral_dark();
    let material = crate::GlassMaterial::new(palette.surface);
    let mut tree = UiTree::new(BottomSheet::new(
        Element::button("base", "Page"), true, "Sheet", "", "dismiss", palette,
    ).glass(material).body(Element::text("Sharp content")).render());
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(400.0, 600.0));
    let shell = frame.access.iter().find(|node| node.id == "dismiss:dialog").unwrap().rect;
    let glass: Vec<_> = frame.commands.iter().filter_map(|command| match command {
        crate::DrawCommand::Glass { rect, material: actual, .. } => Some((*rect, *actual)), _ => None,
    }).collect();
    assert_eq!(glass, vec![(shell, material)]);
    let close = frame.regions.iter().find(|region| region.id == "dismiss:close").unwrap().rect;
    let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: "dismiss".into() }]);
}

#[test]
fn a_glass_sheet_with_an_origin_pops_out_of_it_as_a_drop() {
    use crate::{Component, DrawCommand, Element, GlassMaterial, Rect, Size, UiTree};
    let palette = crate::ComponentPalette::neutral_dark();
    let button = Rect::new(170.0, 700.0, 50.0, 50.0);
    let sheet = crate::BottomSheet::new(Element::spacer(), true, "Output", "", "output:close", palette)
        .body(Element::spacer().size(None, Some(300.0))).glass(GlassMaterial::new(palette.surface)).render()
        .popover_morph(button, 25.0).popover_animation_ms(crate::GLASS_POP_OPEN_MS);
    let now = web_time::Instant::now();
    let mut tree = UiTree::new(sheet);
    let pane = |frame: &crate::Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::Glass { rect, .. } => Some(*rect), _ => None,
    }).expect("a pane of glass");
    tree.layout_at(Size::new(390.0, 844.0), now);
    let early = pane(&tree.layout_at(Size::new(390.0, 844.0), now + web_time::Duration::from_millis(40)));
    assert!(early.width < 100.0, "a drop over the button first: {early:?}");
    let open = pane(&tree.layout_at(Size::new(390.0, 844.0), now + web_time::Duration::from_millis(700)));
    assert!(open.width > 300.0, "then the sheet: {open:?}");
}

#[test]
fn a_held_sheet_follows_the_finger_springs_back_when_let_go_short_and_leaves_when_flung() {
    let mut tree = UiTree::new(sheet(true));
    let at = std::time::Instant::now();
    let ms = |n: u64| at + std::time::Duration::from_millis(n);
    let size = Size::new(390.0, 700.0);
    tree.layout_at(size, ms(0));
    // Entered, it stands in its place.
    let frame = tree.layout_at(size, ms(2000));
    let handle = bounds(&frame, "dismiss:drag");
    let rest = bounds(&frame, "popover_surface:dismiss").y;
    let start = Point::new(handle.x + handle.width / 2.0, handle.y + handle.height / 2.0);
    let top = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        crate::DrawCommand::PushClip(rect) if rect.width == bounds(frame, "popover_surface:dismiss").width => Some(rect.y),
        _ => None,
    });
    // Held 80 points down slowly, it follows the finger; let go, it springs back.
    tree.handle_event_at(UiEvent::PointerDown(start), ms(2000));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(start.x, start.y + 40.0)), ms(2100));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(start.x, start.y + 80.0)), ms(2300));
    let held = tree.layout_at(size, ms(2310));
    assert!((top(&held).unwrap() - (rest + 80.0)).abs() < 0.5, "it follows the finger: {:?} from {rest}", top(&held));
    let release = tree.handle_event_at(UiEvent::PointerUp(Point::new(start.x, start.y + 80.0)), ms(2500));
    assert_eq!(release, vec![UiAction::DragCancelled { id: "dismiss:drag".into() }], "let go short and still");
    // The frame after the release sets the spring off; the next shows it on its way.
    tree.layout_at(size, ms(2510));
    let back = tree.layout_at(size, ms(2580));
    let between = top(&back).expect("on its way back");
    assert!(between > rest && between < rest + 80.0, "it springs back: {between}");
    let settled = tree.layout_at(size, ms(4000));
    assert!(top(&settled).is_none(), "back in its place");
    // Pulled up, it stretches a little and no more.
    tree.handle_event_at(UiEvent::PointerDown(start), ms(4000));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(start.x, start.y - 300.0)), ms(4100));
    let stretched = top(&tree.layout_at(size, ms(4110))).unwrap();
    assert!(stretched < rest && stretched >= rest - 48.0, "a rubber band up: {stretched}");
    tree.handle_event_at(UiEvent::PointerUp(Point::new(start.x, start.y - 300.0)), ms(4600));
    // Flung down, however short, it leaves; its dismissal is where it was dropped.
    tree.layout_at(size, ms(6000));
    tree.handle_event_at(UiEvent::PointerDown(start), ms(6000));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(start.x, start.y + 10.0)), ms(6010));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(start.x, start.y + 40.0)), ms(6030));
    let flung = tree.handle_event_at(UiEvent::PointerUp(Point::new(start.x, start.y + 40.0)), ms(6040));
    assert_eq!(flung, vec![UiAction::Dropped { id: "dismiss:drag".into(), position: Point::new(start.x, start.y + 40.0), target: Some("dismiss".into()) }]);
    let mut drag = BottomSheetDrag::new("dismiss");
    drag.handle_action(&UiAction::DragStarted { id: "dismiss:drag".into(), position: start });
    assert!(drag.handle_action(&flung[0]), "the host closes it");
}

#[test]
fn a_sheet_whose_content_grows_rises_to_its_new_height_on_a_spring_and_one_let_go_leaves_down() {
    let sheet = |tall: f32, open: bool| BottomSheet::new(Element::spacer(), open, "Import", "", "dismiss", ComponentPalette::neutral_dark())
        .max_height(700.0)
        .body(Element::spacer().size(None, Some(tall)))
        .render();
    let at = std::time::Instant::now();
    let ms = |n: u64| at + std::time::Duration::from_millis(n);
    let size = Size::new(390.0, 800.0);
    let mut tree = UiTree::new(sheet(120.0, true));
    tree.layout_at(size, ms(0));
    let short = bounds(&tree.layout_at(size, ms(2000)), "popover_surface:dismiss").y;
    tree.set_root(sheet(320.0, true));
    tree.layout_at(size, ms(2000));
    let frame = tree.layout_at(size, ms(2080));
    let tall = bounds(&frame, "popover_surface:dismiss").y;
    assert!(tall < short - 150.0, "laid out taller at once: {tall} from {short}");
    let shown = frame.commands.iter().find_map(|command| match command {
        crate::DrawCommand::PushClip(rect) if rect.width == 390.0 && rect.y > tall + 1.0 => Some(rect.y),
        _ => None,
    }).expect("its top on its way");
    assert!(shown < short && shown > tall, "it rises rather than jumps: {shown} between {short} and {tall}");
    // Held down and closed there, it leaves down from where it was, not from its place.
    let start = Point::new(195.0, tall + 22.0);
    tree.layout_at(size, ms(4000));
    tree.handle_event_at(UiEvent::PointerDown(start), ms(4000));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(195.0, start.y + 150.0)), ms(4200));
    tree.layout_at(size, ms(4210));
    tree.set_root(sheet(320.0, false));
    let leaving = tree.layout_at(size, ms(4230));
    let lowest = leaving.commands.iter().filter_map(|command| match command {
        crate::DrawCommand::FillCornerRect { rect, .. } | crate::DrawCommand::FillRect { rect, .. } if rect.width == 390.0 && rect.height < 700.0 => Some(rect.y),
        _ => None,
    }).fold(f32::MIN, f32::max);
    assert!(lowest >= tall + 150.0, "it goes on from the finger: {lowest} from {tall}");
}
