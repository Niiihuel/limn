// Staged by the crate split, not yet a module of this crate: these tests
// build their trees with components that live here now.
//
// Came from `crates/limn-core/src/tree.rs`, its `mod tests`, unchanged:
// - `popover_arrows_skip_secondary_actions_but_tab_and_activation_keep_them`,
//   `popover_secondary_actions_preserve_offscreen_primary_focus_and_reveal` and
//   `unmarked_popover_buttons_keep_the_existing_arrow_order`, with their helper
//   `menu_with_secondary_actions` (`DropdownMenu`, `DropdownMenuEntry`, `MenuItem`).
// - `intrinsic_width_cap_keeps_breadcrumb_stats_adjacent_and_actions_in_bounds` (`Breadcrumb`).
// - `menu_entrance_has_one_surface_and_only_the_hovered_row_gets_a_fill` (`DropdownMenu`).
// - `modal_blocks_outside_wheel_but_its_body_still_scrolls` (`Modal`).
// Their `use super::*` (the engine's `tree` module) is replaced by
// `use crate::*` and the clock; `DropdownMenu` and `DropdownMenuItem` came
// from a `use` among the engine's tests.
//
// Engine items they use that `limn-core` does not make public:
// - `Frame::scroll_focus_targets` and `Frame::scroll_focus_candidates`: private
//   fields, read by `popover_secondary_actions_preserve_offscreen_primary_focus_and_reveal`.
// - `UiTree::measurement_cache` (private field, `RefCell<MeasurementCache>` of a
//   private module) and `MeasurementCache::disabled`, which is `pub(super)` and
//   exists only under `cfg(test)` of `limn-core`, so not at all when the engine
//   is a dependency. Written by
//   `intrinsic_width_cap_keeps_breadcrumb_stats_adjacent_and_actions_in_bounds`.
mod tests {
    use crate::*;
    use web_time::{Duration, Instant};

    fn menu_with_secondary_actions(mark_secondary: bool, visible_rows: usize) -> Element {
        use crate::{
            Component, ComponentPalette, DropdownMenu, DropdownMenuEntry, DropdownMenuItem,
            MenuItem, Tooltip,
        };

        let palette = ComponentPalette::neutral_dark();
        let mut entries = vec![DropdownMenuItem::new("normal", "New terminal").into()];
        for index in 1..=2 {
            let close_id = format!("close:{index}");
            let mut close = Element::icon_button(&close_id, Icon::Close, "")
                .size(Some(24.0), Some(24.0))
                .dispatch_as(format!("terminal_close:{index}"));
            if mark_secondary {
                close = close.menu_secondary_action();
            }
            entries.push(DropdownMenuEntry::custom(
                Element::row(vec![
                    MenuItem::new(
                        format!("main:{index}"),
                        format!("Terminal {index}"),
                        palette,
                    )
                    .width(180.0)
                    .height(28.0)
                    .render(),
                    Tooltip::new(close, close_id, "Close terminal", palette).render(),
                ])
                .gap(4.0)
                .size(None, Some(28.0)),
            ));
        }
        DropdownMenu::new(
            Element::button("trigger", "Terminals").size(Some(100.0), Some(28.0)),
            entries,
            true,
            "dismiss",
            palette,
        )
        .width(240.0)
        .max_visible_items(visible_rows)
        .render()
    }

    #[test]
    fn popover_arrows_skip_secondary_actions_but_tab_and_activation_keep_them() {
        let viewport = Size::new(320.0, 240.0);
        let mut tree = UiTree::new(menu_with_secondary_actions(true, 8));
        tree.set_reduced_motion(true);
        tree.layout(viewport);
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        for (from, direction, expected) in [
            ("main:1", Key::ArrowDown, "main:2"),
            ("main:2", Key::ArrowUp, "main:1"),
            ("close:1", Key::ArrowDown, "main:2"),
            ("close:2", Key::ArrowUp, "main:2"),
            ("close:2", Key::ArrowDown, "normal"),
            ("normal", Key::ArrowUp, "main:2"),
            ("close:2", Key::Home, "normal"),
            ("normal", Key::End, "main:2"),
            ("trigger", Key::ArrowUp, "main:2"),
            ("trigger", Key::ArrowDown, "normal"),
        ] {
            assert!(tree.focus(from));
            assert!(tree.handle_event(key(direction)).is_empty());
            assert_eq!(tree.focused_id(), Some(expected), "{from}: {direction:?}");
        }
        assert!(tree.focus("main:1"));
        assert!(tree.handle_event(key(Key::Tab)).is_empty());
        assert_eq!(tree.focused_id(), Some("close:1"));
        for activate in [Key::Enter, Key::Space] {
            assert_eq!(
                tree.handle_event(key(activate)),
                vec![UiAction::Clicked {
                    id: "terminal_close:1".into(),
                }]
            );
        }
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: true,
                ctrl: false,
                alt: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("main:1"));

        let frame = tree.layout(viewport);
        let close = frame
            .regions
            .iter()
            .find(|region| region.id == "close:2")
            .unwrap()
            .rect;
        let point = Point::new(close.x + close.width / 2.0, close.y + close.height / 2.0);
        assert_eq!(frame.hit_test(point), Some("close:2"));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "terminal_close:2".into(),
            }]
        );
        assert_eq!(
            tree.handle_event(key(Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss".into(),
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn popover_secondary_actions_preserve_offscreen_primary_focus_and_reveal() {
        let viewport = Size::new(320.0, 240.0);
        let mut tree = UiTree::new(menu_with_secondary_actions(true, 1));
        tree.set_reduced_motion(true);
        let before = tree.layout(viewport);
        assert!(!before.regions.iter().any(|region| region.id == "main:2"));
        assert!(!before.scroll_focus_targets.contains_key("main:2"));
        assert!(before.scroll_focus_candidates.contains("main:2"));
        assert!(tree.focus("trigger"));
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::End,
                shift: false,
                ctrl: false,
                alt: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("main:2"));
        let frame = tree.layout(viewport);
        let options = frame
            .regions
            .iter()
            .find(|region| region.id == "dismiss:options" && region.kind == HitKind::Scroll)
            .unwrap()
            .rect;
        let main = frame
            .regions
            .iter()
            .find(|region| region.id == "main:2")
            .unwrap()
            .rect;
        assert_eq!(main.height, 28.0);
        assert!(main.y >= options.y && main.y + main.height <= options.y + options.height);
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(tree
            .handle_event(UiEvent::Key {
                key: Key::Tab,
                shift: false,
                ctrl: false,
                alt: false,
            })
            .is_empty());
        assert_eq!(tree.focused_id(), Some("close:2"));
    }

    #[test]
    fn unmarked_popover_buttons_keep_the_existing_arrow_order() {
        let mut tree = UiTree::new(menu_with_secondary_actions(false, 8));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(320.0, 240.0));
        assert!(tree.focus("main:1"));
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowDown,
            shift: false,
            ctrl: false,
            alt: false,
        });
        assert_eq!(tree.focused_id(), Some("close:1"));
        tree.handle_event(UiEvent::Key {
            key: Key::End,
            shift: false,
            ctrl: false,
            alt: false,
        });
        assert_eq!(tree.focused_id(), Some("close:2"));
    }

    #[test]
    fn intrinsic_width_cap_keeps_breadcrumb_stats_adjacent_and_actions_in_bounds() {
        use crate::{Breadcrumb, BreadcrumbItem, Component, ComponentPalette};

        let measured_width = |text: &str| {
            text.chars()
                .map(|character| match character {
                    'W' => 12.0,
                    'i' => 3.0,
                    _ => 6.0,
                })
                .sum::<f32>()
        };
        for cache_disabled in [false, true] {
            for label in [
                "README.md",
                "Wider_resource_with_a_long_filename_for_this_header.md",
            ] {
                let budget = 140.0;
                let path = Breadcrumb::new(
                    "path",
                    vec![BreadcrumbItem::new(label).action("open_path")],
                    ComponentPalette::neutral_dark(),
                )
                .width(budget)
                .height(24.0)
                .render()
                .size(None, Some(24.0))
                .max_width(budget);
                let mut tree = UiTree::new(
                    Element::row(vec![
                        path,
                        Element::text("Local +62 −5")
                            .font_family(FontFamily::Sans)
                            .size(Some(80.0), Some(24.0)),
                        Element::spacer().flex(1.0),
                        Element::icon_button("refresh", Icon::Refresh, "")
                            .size(Some(24.0), Some(24.0)),
                    ])
                    .gap(8.0),
                );
                tree.measurement_cache.get_mut().disabled = cache_disabled;
                tree.set_reduced_motion(true);
                tree.set_text_measurer(move |text, _, _| measured_width(text));
                let size = Size::new(320.0, 24.0);
                let frame = tree.layout(size);
                let path_rect = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "open_path" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                let expected_width = (measured_width(label) + 4.0).min(budget);
                assert_eq!(path_rect, Rect::new(0.0, 0.0, expected_width, 24.0));
                let stats_x = frame
                    .commands
                    .iter()
                    .find_map(|command| match command {
                        DrawCommand::Text { text, position, .. } if text == "Local +62 −5" => {
                            Some(position.x)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert_eq!(stats_x, expected_width + 8.0);
                let refresh_rect = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "refresh" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                assert_eq!(refresh_rect, Rect::new(296.0, 0.0, 24.0, 24.0));
                assert!(stats_x + 80.0 < refresh_rect.x);
                assert_ne!(
                    frame.hit_test(Point::new(expected_width + 2.0, 12.0)),
                    Some("open_path"),
                    "the resource action must not extend into its trailing gap"
                );
                for (id, rect) in [("open_path", path_rect), ("refresh", refresh_rect)] {
                    let point = Point::new(rect.x + rect.width / 2.0, 12.0);
                    tree.handle_event(UiEvent::PointerDown(point));
                    assert_eq!(
                        tree.handle_event(UiEvent::PointerUp(point)),
                        vec![UiAction::Clicked { id: id.into() }],
                        "clipped filename and toolbar keep independent actions"
                    );
                }
            }
        }
    }

    #[test]
    fn menu_entrance_has_one_surface_and_only_the_hovered_row_gets_a_fill() {
        let mut palette = ComponentPalette::neutral_dark();
        palette.surface = Color::rgb(30, 30, 30);
        palette.raised = palette.surface;
        palette.hover = Color::rgb(48, 48, 48);
        let menu = DropdownMenu::new(
            Element::button("trigger", "Abrir").size(Some(90.0), Some(30.0)),
            vec![
                DropdownMenuItem::new("first", "Primera").into(),
                DropdownMenuItem::new("second", "Segunda").into(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(220.0)
        .render();
        let start = Instant::now();
        let viewport = Size::new(320.0, 220.0);
        let mut tree = UiTree::new(menu);
        tree.layout_at(viewport, start);
        let entering = tree.layout_at(viewport, start + Duration::from_millis(70));
        let surface = entering.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if rect.width == 220.0 && *color == Color::rgba(30, 30, 30, color.a) =>
            {
                Some(*color)
            }
            _ => None,
        });
        assert!(surface.is_some_and(|color| color.a > 0 && color.a < 255));
        assert!(!entering.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, .. } if rect.width == 208.0 && rect.height == 36.0
        )));

        let first = entering
            .regions
            .iter()
            .find(|region| region.id == "first")
            .unwrap();
        let point = Point::new(first.rect.x + 8.0, first.rect.y + 8.0);
        tree.handle_event(UiEvent::PointerMove(point));
        tree.set_reduced_motion(true);
        let hovered = tree.layout_at(viewport, start + Duration::from_millis(80));
        assert_eq!(hovered.hit_test(point), Some("first"));
        assert_eq!(
            hovered
                .commands
                .iter()
                .filter(|command| matches!(command,
                    DrawCommand::FillRect { rect, color, .. }
                        if rect.width == 208.0 && rect.height == 36.0 && *color == palette.hover
                ))
                .count(),
            1,
        );
    }

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
}
