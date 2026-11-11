use super::*;

#[test]
fn glass_selection_captures_labels_without_feedback_and_preserves_hit_targets() {
    let palette = ComponentPalette::neutral_dark();
    // Two tabs of 80 on a track 2 pixels around them, under a glass selection.
    let view = |selected: &str| {
        let tab = |id: &str, label: &str| {
            let active = id == selected;
            let (ink, hover) = if active { (palette.text, crate::lighter(palette.text, 12)) } else { (palette.muted, palette.text) };
            Element::button(id, label)
                .background(Color::TRANSPARENT)
                .hover_foreground(hover)
                .pressed_foreground(hover)
                .foreground(ink)
                .font_family(FontFamily::Sans)
                .font_size(11.5)
                .padding(4.0)
                .gap(4.0)
                .radius(crate::control_radius())
                .center_content()
                .size(Some(80.0), Some(24.0))
                .accessible(crate::Access::new(crate::Role::Tab).selected(active))
        };
        Element::tab_strip(vec![tab("home", "Home"), tab("browse", "Browse")], vec!["home".into(), "browse".into()])
            .gap(2.0)
            .padding(2.0)
            .size(None, Some(28.0))
            .background(palette.surface)
            .radius(crate::surface_radius())
            .tab_selection("glass-tabs", selected, palette.selected, crate::control_radius(), crate::tokens().motion.move_ms)
            .glass_selection(GlassMaterial::new(palette.surface))
    };
    let size = Size::new(200.0, 40.0);
    let now = Instant::now();
    let mut tree = UiTree::new(view("home"));
    let initial = tree.layout_at(size, now);
    let lens = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::GlassSelection { rect, backdrop, .. } => Some((*rect, backdrop.clone())), _ => None,
    }).unwrap();
    let (start, backdrop) = lens(&initial);
    assert!(initial.commands.iter().any(|command| matches!(command, DrawCommand::GlassSelection { opacity, .. } if *opacity == 0.0)));
    assert!(backdrop.iter().any(|command| matches!(command, DrawCommand::Text { .. } | DrawCommand::Glyphs(_))));
    assert!(!backdrop.iter().any(|command| matches!(command, DrawCommand::GlassSelection { .. })));
    let target_region = initial.regions.iter().find(|region| region.id == "browse").unwrap().rect;
    tree.set_root(view("browse"));
    tree.layout_at(size, now + Duration::from_millis(10));
    let midway = tree.layout_at(size, now + Duration::from_millis(70));
    let (moving, _) = lens(&midway);
    assert!(moving.x > start.x);
    assert!(moving.width > start.width, "travel stretches the lens");
    assert!(moving.height > start.height && moving.y < start.y, "and lifts it past its strip: {moving:?} from {start:?}");
    let corner = |frame: &Frame| frame.commands.iter().find_map(|command| match command { DrawCommand::GlassSelection { radius, .. } => Some(*radius), _ => None }).unwrap();
    assert!((corner(&midway) / moving.height - corner(&initial) / start.height).abs() < 0.01,
        "its corners grow with it, so a capsule lifted is a capsule still: {} on {moving:?}", corner(&midway));
    assert!(midway.commands.iter().any(|command| matches!(command, DrawCommand::GlassSelection { opacity, .. } if *opacity > 0.1)));
    assert_eq!(midway.regions.iter().find(|region| region.id == "browse").unwrap().rect, target_region);
    tree.set_reduced_motion(true);
    let (snapped, _) = lens(&tree.layout_at(size, now + Duration::from_millis(80)));
    assert_eq!(snapped.x, target_region.x);
    assert_eq!(snapped.width, target_region.width);
}

#[test]
fn a_held_tab_bar_lens_follows_the_finger_and_lands_on_the_tab_it_is_let_go_over() {
    let view = |selected: &str| {
        let cell = |id: &str| Element::clickable(id, Element::spacer()).size(Some(100.0), Some(40.0)).indicator_slot(id);
        Element::tab_strip(vec![cell("a"), cell("b"), cell("c")], vec!["a".into(), "b".into(), "c".into()])
            .tab_selection("bar", selected, Color::rgb(60, 60, 60), 20.0, 200)
            .glass_selection(GlassMaterial::new(Color::rgba(255, 255, 255, 0)))
    };
    let size = Size::new(300.0, 40.0);
    let mut tree = UiTree::new(view("a"));
    tree.set_reduced_motion(true);
    tree.layout(size);
    let lens = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::GlassSelection { rect, .. } => Some(*rect), _ => None,
    }).unwrap();
    tree.handle_event(UiEvent::PointerMove(Point::new(50.0, 20.0)));
    tree.handle_event(UiEvent::PointerDown(Point::new(50.0, 20.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(230.0, 20.0)));
    let held = lens(&tree.layout(size));
    assert!((held.x + held.width / 2.0 - 230.0).abs() < 0.5, "the lens stands under the finger: {held:?}");
    assert_eq!(tree.handle_event(UiEvent::PointerUp(Point::new(230.0, 20.0))), vec![UiAction::Clicked { id: "c".into() }]);
    // Dragged past the bar's end, it stops at its last tab.
    tree.set_root(view("c"));
    tree.layout(size);
    tree.handle_event(UiEvent::PointerDown(Point::new(250.0, 20.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(400.0, 20.0)));
    assert_eq!(lens(&tree.layout(size)).x, 200.0);
}

#[test]
fn what_a_host_shows_again_stands_at_once_instead_of_entering_anew() {
    let ink = Color::rgb(200, 100, 50);
    let view = |shown: bool| Element::column(if shown {
        vec![Element::panel(Element::spacer()).background(ink).size(Some(80.0), Some(40.0)).animate_enter("shelf")]
    } else { vec![] });
    let alpha = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { color, .. } if color.r == ink.r && color.g == ink.g => Some(color.a), _ => None,
    });
    let now = Instant::now();
    let mut tree = UiTree::new(view(true));
    assert!(alpha(&tree.layout_at(Size::new(200.0, 100.0), now)).unwrap_or(0) < 255, "a first appearance enters");
    tree.set_root(view(false));
    tree.layout_at(Size::new(200.0, 100.0), now + Duration::from_millis(500));
    tree.set_root(view(true));
    tree.settle_entrances();
    assert_eq!(alpha(&tree.layout_at(Size::new(200.0, 100.0), now + Duration::from_millis(510))), Some(255), "shown again, it stands");
}

#[test]
fn a_segmented_knob_lifts_for_half_its_way_and_lands_in_its_slot_as_auroras() {
    let lens = GlassMaterial::new(Color::rgba(255, 255, 255, 40)).blur(0.0);
    let view = |selected: &str| {
        let cell = |id: &str| Element::clickable(id, Element::text(id)).size(None, Some(44.0)).flex(1.0).indicator_slot(id);
        Element::tab_strip(vec![cell("all"), cell("songs"), cell("artists"), cell("albums")],
            ["all", "songs", "artists", "albums"].map(String::from).to_vec())
            .tab_selection("segments", selected, Color::rgb(99, 99, 102), 15.0, 200)
            .tab_selection_height(30.0).glass_selection(lens).selection_lift(1.3)
    };
    let lens_at = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::GlassSelection { rect, opacity, .. } => Some((*rect, *opacity)), _ => None,
    }).unwrap();
    let size = Size::new(400.0, 44.0);
    let now = Instant::now();
    let mut tree = UiTree::new(view("all"));
    let (rest, hidden) = lens_at(&tree.layout_at(size, now));
    assert!((rest.height - 30.0).abs() < 0.01 && (rest.y - 7.0).abs() < 0.01 && hidden == 0.0, "a 30 knob in a 44 slot: {rest:?}");
    tree.set_root(view("albums"));
    tree.layout_at(size, now);
    let (lifted, seen) = lens_at(&tree.layout_at(size, now + Duration::from_millis(60)));
    assert!(lifted.height > 34.0 && seen > 0.5, "lifted into a lens on its way: {lifted:?}");
    for ms in (100..1500).step_by(16) { tree.layout_at(size, now + Duration::from_millis(ms)); }
    let (landed, gone) = lens_at(&tree.layout_at(size, now + Duration::from_millis(1500)));
    assert!((landed.height - 30.0).abs() < 0.5 && landed.x > 280.0 && gone < 0.05, "landed in its slot, flat again: {landed:?}");
}

#[test]
fn glass_switch_lifts_on_press_and_tracks_a_drag_without_moving_hit_regions() {
    let p = ComponentPalette::neutral_dark();
    // A glass switch of 64 by 28, off: its 22-pixel thumb is the selection of
    // a strip with an "off" and an "on" slot, 3 pixels inside the track.
    let view = || {
        let slot = |name: &str| Element::spacer().size(Some(22.0), Some(22.0)).indicator_slot(name);
        let thumb = Element::tab_strip(vec![slot("off"), Element::spacer().size(Some(14.0), Some(22.0)), slot("on")], Vec::new())
            .tab_selection("switch:thumb", "off", p.text, 11.0, crate::tokens().motion.move_ms)
            .glass_selection(GlassMaterial::new(p.surface))
            .selection_press("switch");
        Element::clickable("switch", thumb)
            .hover_background(crate::lighter(p.selected, 10))
            .pressed_background(crate::lighter(p.selected, 16))
            .transition_ms(crate::hover_ms())
            .background(p.selected)
            .padding(3.0)
            .radius(14.0)
            .size(Some(64.0), Some(28.0))
            .accessible(crate::Access::new(crate::Role::Switch).id("switch").toggled(false).disabled(false))
    };
    let mut tree = UiTree::new(view());
    let now = Instant::now();
    let size = Size::new(80.0, 40.0);
    let lens = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::GlassSelection { rect, .. } => Some(*rect), _ => None,
    }).unwrap();
    let first = tree.layout_at(size, now);
    let first_lens = lens(&first);
    let bounds = first.regions.iter().find(|region| region.id == "switch").unwrap().rect;
    let point = Point::new(first_lens.x + first_lens.width / 2.0, first_lens.y + first_lens.height / 2.0);
    tree.handle_event(UiEvent::PointerDown(point));
    tree.layout_at(size, now + Duration::from_millis(1));
    let pressed = tree.layout_at(size, now + Duration::from_millis(81));
    assert!(lens(&pressed).width > first_lens.width * 1.1);
    tree.handle_event(UiEvent::PointerMove(Point::new(point.x + 24.0, point.y)));
    let dragged = tree.layout_at(size, now + Duration::from_millis(97));
    assert!(lens(&dragged).x > first_lens.x + 15.0);
    assert_eq!(dragged.regions.iter().find(|region| region.id == "switch").unwrap().rect, bounds);
}

#[test]
fn glass_switch_drag_sets_the_endpoint_while_taps_toggle_and_cancel_is_inert() {
    let palette = ComponentPalette::neutral_dark();
    let make = |checked: bool| {
        // A glass switch of 64 by 28 with a 40-pixel thumb 2 pixels inside the
        // track: the thumb is the selection of a strip of two 30-pixel slots.
        let track = if checked { palette.text } else { palette.selected };
        let slot = |name: &str| Element::spacer().size(Some(30.0), Some(24.0)).indicator_slot(name);
        let thumb = Element::tab_strip(vec![slot("off"), slot("on")], Vec::new())
            .tab_selection("switch:thumb", if checked { "on" } else { "off" }, palette.text, 12.0, crate::tokens().motion.move_ms)
            .selection_width(40.0)
            .glass_selection(GlassMaterial::new(palette.surface))
            .selection_press("switch");
        let switch = Element::clickable("switch", thumb)
            .hover_background(crate::lighter(track, 10))
            .pressed_background(crate::lighter(track, 16))
            .transition_ms(crate::hover_ms())
            .background(track)
            .padding(2.0)
            .radius(14.0)
            .size(Some(64.0), Some(28.0))
            .accessible(crate::Access::new(crate::Role::Switch).id("switch").toggled(checked).disabled(false));
        let mut tree = UiTree::new(switch);
        tree.layout(Size::new(80.0, 40.0));
        tree
    };
    let clicked = vec![UiAction::Clicked { id: "switch".into() }];
    for checked in [false, true] {
        let measured = make(checked);
        let bounds = measured.frame().regions.iter().find(|region| region.id == "switch").unwrap().rect;
        let point = |fraction: f32| Point::new(bounds.x + bounds.width * fraction, bounds.y + bounds.height / 2.0);
        let origin = point(if checked { 0.75 } else { 0.25 });
        let other = point(if checked { 0.25 } else { 0.75 });
        let same = point(if checked { 0.625 } else { 0.375 });
        let mut tap = make(checked);
        tap.handle_event(UiEvent::PointerDown(origin));
        assert_eq!(tap.handle_event(UiEvent::PointerUp(origin)), clicked);
        let mut change = make(checked);
        change.handle_event(UiEvent::PointerDown(origin));
        change.handle_event(UiEvent::PointerMove(other));
        assert_eq!(change.handle_event(UiEvent::PointerUp(other)), clicked);
        let mut keep = make(checked);
        keep.handle_event(UiEvent::PointerDown(origin));
        keep.handle_event(UiEvent::PointerMove(same));
        assert!(keep.handle_event(UiEvent::PointerUp(same)).is_empty());
        let mut reverse = make(checked);
        reverse.handle_event(UiEvent::PointerDown(origin));
        reverse.handle_event(UiEvent::PointerMove(other));
        reverse.handle_event(UiEvent::PointerMove(origin));
        assert!(reverse.handle_event(UiEvent::PointerUp(origin)).is_empty());
        let mut cancel = make(checked);
        cancel.handle_event(UiEvent::PointerDown(origin));
        cancel.handle_event(UiEvent::PointerMove(other));
        cancel.handle_event(UiEvent::PointerLeave);
        assert!(cancel.handle_event(UiEvent::PointerUp(other)).is_empty());
    }
}

#[test]
fn pressed_interactive_glass_grows_and_lights_up_where_it_is_held_then_settles_back() {
    let material = GlassMaterial::new(Color::rgba(38, 38, 40, 122));
    let view = || Element::panel(Element::button("go", "Go").size(Some(56.0), Some(56.0)))
        .glass(material).glass_press("go").round().size(Some(56.0), Some(56.0));
    let pane = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::Glass { rect, material, .. } => Some((*rect, *material)), _ => None,
    }).unwrap();
    let now = Instant::now();
    let mut tree = UiTree::new(view());
    let (rest, _) = pane(&tree.layout_at(Size::new(200.0, 200.0), now));
    let at = Point::new(rest.x + 40.0, rest.y + 20.0);
    tree.handle_event(UiEvent::PointerMove(at));
    tree.handle_event(UiEvent::PointerDown(at));
    tree.layout_at(Size::new(200.0, 200.0), now);
    let (held, lit) = pane(&tree.layout_at(Size::new(200.0, 200.0), now + Duration::from_millis(250)));
    assert!(held.width > rest.width * 1.05 && held.width < rest.width * 1.1, "grown about 7.5%: {held:?}");
    assert!(lit.glow > 0.9 && (lit.glow_at.0 - 40.0 / rest.width).abs() < 0.01 && (lit.glow_at.1 - 20.0 / rest.height).abs() < 0.01,
        "lit where held: {lit:?}");
    tree.handle_event(UiEvent::PointerUp(at));
    tree.layout_at(Size::new(200.0, 200.0), now + Duration::from_millis(260));
    let (back, _) = pane(&tree.layout_at(Size::new(200.0, 200.0), now + Duration::from_millis(1200)));
    assert!((back.width - rest.width).abs() < 0.5, "back to its size: {back:?}");
}

#[test]
fn glass_follows_drag_entrance_and_popover_shell_geometry() {
    let material = GlassMaterial::new(Color::rgba(100, 120, 140, 60));
    let mut command = DrawCommand::Glass {
        rect: Rect::new(10.0, 20.0, 200.0, 60.0),
        radius: 30.0,
        material,
        opacity: 0.8,
    }.moved(5.0, -10.0, 0.5);
    assert_eq!(command, DrawCommand::Glass {
        rect: Rect::new(15.0, 10.0, 200.0, 60.0), radius: 30.0, material, opacity: 0.4,
    });
    animate_popover_command(&mut command, 0.5, 4.0, 0.5, Point::new(15.0, 10.0));
    assert_eq!(command, DrawCommand::Glass {
        rect: Rect::new(15.0, 14.0, 100.0, 30.0), radius: 15.0, material, opacity: 0.2,
    });
    assert!(popover_shell_command(&command, Rect::new(15.0, 14.0, 100.0, 30.0)));
    let target = Rect::new(20.0, 30.0, 120.0, 80.0);
    morph_popover_shell(&mut command, target);
    assert!(matches!(command, DrawCommand::Glass { rect, .. } if rect == target));
}

#[test]
fn max_width_paragraph_measures_its_wrapped_height_before_the_next_sibling() {
    let description = "A paragraph whose natural width is longer than its maximum must keep every wrapped line above the following control.";
    for width in [160.0, 480.0] {
        let paragraph = Element::text(description)
            .max_width(120.0)
            .shrink_to_fit()
            .centered_x();
        let mut tree = UiTree::new(
            Element::column(vec![
                paragraph,
                Element::button("next", "Continue").size(None, Some(32.0)),
            ])
            .gap(12.0),
        );
        let frame = tree.layout(Size::new(width, 600.0));
        let blocks = frame.text_layouts();
        let paragraph = blocks
            .iter()
            .find(|block| block.source == description)
            .unwrap();
        assert!(
            paragraph.runs.len() > 1,
            "the capped paragraph must wrap at {width}"
        );
        let bottom = paragraph
            .runs
            .iter()
            .map(|run| run.rect.y + run.rect.height)
            .fold(0.0_f32, f32::max);
        let next = frame
            .regions
            .iter()
            .find(|region| region.id == "next")
            .unwrap();
        assert!(next.rect.y >= bottom + 12.0 - 0.1,
            "wrapped paragraph consumed its following gap at {width}: bottom={bottom}, next={:?}", next.rect);
    }
}

#[test]
fn intrinsic_width_cap_lays_out_a_truncated_link_and_its_trailing_controls_alike_with_the_cache_off() {
    use crate::{Access, Component, ComponentPalette, Role, Tooltip};

    let measured_width = |text: &str| {
        text.chars()
            .map(|character| match character {
                'W' => 12.0,
                'i' => 3.0,
                _ => 6.0,
            })
            .sum::<f32>()
    };
    let palette = ComponentPalette::neutral_dark();
    let budget = 140.0;
    for label in [
        "README.md",
        "Wider_resource_with_a_long_filename_for_this_header.md",
    ] {
        // The row a breadcrumb of one linked resource renders to: a faded
        // label in a link that a stretching slot hugs, the row capped in
        // width, and a stat and a button after it.
        let layout = |cache_disabled: bool| {
            let visual = Element::row(vec![Element::faded_text(label, 16.0)
                .font_family(FontFamily::Sans)
                .font_size(11.5)
                .foreground(palette.text)
                .size(None, Some(20.0))
                .min_size(0.0, 0.0)
                .flex(1.0)])
            .gap(4.0)
            .clip_children();
            let link = Element::clickable("open_path", visual)
                .underline_on_hover()
                .accessible(Access::new(Role::Link))
                .padding(2.0)
                .size(None, Some(24.0));
            let tooltip = Tooltip::new(link, "open_path", label, palette).max_width(320.0);
            let tooltip = if label.chars().count() > 40 {
                tooltip.description(label)
            } else {
                tooltip
            };
            let path = Element::row(vec![Element::row(vec![
                tooltip.render(),
                Element::spacer().flex(1.0),
            ])
            .clip_children()
            .flex(1.0)])
            .gap(4.0)
            .size(Some(budget), Some(24.0))
            .clip_children()
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
            tree.layout(Size::new(320.0, 24.0))
        };
        let cached = layout(false);
        let rect = |id: &str| {
            cached
                .regions
                .iter()
                .find(|region| region.id == id && region.kind == HitKind::Button)
                .unwrap()
                .rect
        };
        let expected_width = (measured_width(label) + 4.0).min(budget);
        assert_eq!(rect("open_path"), Rect::new(0.0, 0.0, expected_width, 24.0));
        assert_eq!(rect("refresh"), Rect::new(296.0, 0.0, 24.0, 24.0));
        assert_eq!(layout(true), cached, "{label}");
    }
}

#[test]
fn intrinsic_width_cap_preserves_explicit_minimums_and_anchor_wrappers() {
    use crate::{Component, ComponentPalette, Tooltip};

    for cache_disabled in [false, true] {
        for minimum in [0.0_f32, 96.0] {
            for wrapper in 0..3 {
                let anchor = Element::button("bounded", "Bounded")
                    .size(Some(160.0), Some(24.0))
                    .min_size(minimum, 0.0)
                    .max_width(64.0);
                let wrapped = match wrapper {
                    0 => anchor,
                    1 => Tooltip::new(
                        anchor,
                        "bounded",
                        "Bounded action",
                        ComponentPalette::neutral_dark(),
                    )
                    .render(),
                    _ => Element::popover(
                        anchor,
                        Element::spacer(),
                        "dismiss",
                        PopoverPlacement::BottomStart,
                    ),
                };
                let mut tree = UiTree::new(Element::row(vec![
                    wrapped,
                    Element::button("after", "After").size(Some(48.0), Some(24.0)),
                ]));
                tree.measurement_cache.get_mut().disabled = cache_disabled;
                tree.set_reduced_motion(true);
                let frame = tree.layout(Size::new(300.0, 80.0));
                let bounded = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "bounded" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                let after = frame
                    .regions
                    .iter()
                    .find(|region| region.id == "after" && region.kind == HitKind::Button)
                    .unwrap()
                    .rect;
                assert_eq!(bounded.width, minimum.max(64.0));
                assert_eq!(after.x, bounded.x + bounded.width);
            }
        }
    }
}

#[test]
fn intrinsic_height_cap_keeps_short_content_compact_and_long_content_scrollable() {
    for cache_disabled in [false, true] {
        for content_height in [20.0, 200.0] {
            let mut tree = UiTree::new(Element::column(vec![
                Element::scroll("capped", Element::spacer().size(None, Some(content_height)))
                    .max_height(48.0),
                Element::button("after", "After").size(Some(100.0), Some(24.0)),
            ]));
            tree.measurement_cache.get_mut().disabled = cache_disabled;
            let frame = tree.layout(Size::new(220.0, 300.0));
            let after = frame
                .regions
                .iter()
                .find(|region| region.id == "after")
                .unwrap();
            assert_eq!(after.rect.y, content_height.min(48.0));
            assert_eq!(
                frame.scroll_extents.get("capped").copied().unwrap_or(0.0),
                (content_height - 48.0).max(0.0)
            );
        }
        let mut tree = UiTree::new(Element::column(vec![
            Element::button("fixed", "Fixed")
                .size(Some(100.0), Some(80.0))
                .max_height(32.0),
            Element::button("after", "After").size(Some(100.0), Some(24.0)),
        ]));
        tree.measurement_cache.get_mut().disabled = cache_disabled;
        let frame = tree.layout(Size::new(220.0, 300.0));
        assert_eq!(
            frame
                .regions
                .iter()
                .find(|region| region.id == "fixed")
                .unwrap()
                .rect
                .height,
            32.0
        );
        assert_eq!(
            frame
                .regions
                .iter()
                .find(|region| region.id == "after")
                .unwrap()
                .rect
                .y,
            32.0
        );
    }
}

#[test]
fn narrow_rows_omit_whole_shortcuts_before_consuming_the_labels_minimum_width() {
    for width in [352.0, 120.0] {
        let mut tree = UiTree::new(Element::column(vec![
            Element::clickable(
                "action",
                Element::row(vec![
                    Element::text("Todas las coincidencias")
                        .min_size(40.0, 22.0)
                        .flex(1.0),
                    Element::keyboard_shortcut("Ctrl+Shift+L", Color::rgb(0, 0, 0), None),
                ])
                .gap(8.0),
            )
            .min_size(0.0, 26.0),
            Element::button("after", "After").size(None, Some(24.0)),
        ]));
        let frame = tree.layout(Size::new(width, 300.0));
        let action = frame
            .regions
            .iter()
            .find(|region| region.id == "action")
            .unwrap();
        let after = frame
            .regions
            .iter()
            .find(|region| region.id == "after")
            .unwrap();
        assert_eq!(after.rect.y, action.rect.y + action.rect.height);
        let has_ctrl = frame.commands.iter().any(|command| {
            matches!(command,
            DrawCommand::Text { text, .. } if text == "Ctrl")
        });
        assert_eq!(
            has_ctrl,
            width > 300.0,
            "the complete chord must yield in narrow rows"
        );
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text.contains("Todas"))));
    }
}

#[test]
fn intrinsic_faded_text_uses_its_ancestor_clip_as_the_visible_edge() {
    let text = "a_long_filename_with_an_intrinsic_width.rs";
    let mut tree = UiTree::new(Element::column(vec![Element::row(vec![
        Element::faded_text(text, 20.0).size(Some(400.0), Some(24.0)),
    ])
    .size(Some(100.0), Some(24.0))
    .clip_children()]));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
    let frame = tree.layout(Size::new(300.0, 100.0));
    let painted: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                text: value,
                color,
                position,
                ..
            } if value == text => Some((*position, color.a)),
            _ => None,
        })
        .collect();
    assert_eq!(painted.len(), 17, "solid text plus sixteen fade bands");
    assert!(painted.windows(2).all(|pair| pair[0].0 == pair[1].0));
    assert_eq!(painted[0].1, 255);
    assert!(painted.last().unwrap().1 < 16);
    assert!(!tree.is_animating());
}

#[test]
fn independent_corners_preserve_hover_press_focus_and_action_geometry() {
    let hover = Color::rgb(66, 66, 66);
    let pressed = Color::rgb(76, 76, 76);
    for width in [101.0, 360.0, 900.0] {
        let mut tree = UiTree::new(
            Element::clickable("rounded-row", Element::text("Provider"))
                .size(Some(width), Some(51.0))
                .hover_background(hover)
                .pressed_background(pressed)
                .corner_radii(CornerRadii::top(12.0)),
        );
        tree.set_reduced_motion(true);
        let initial = tree.layout(Size::new(width, 100.0));
        let rect = initial
            .regions
            .iter()
            .find(|region| region.id == "rounded-row")
            .unwrap()
            .rect;
        let point = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerMove(point));
        let hovered = tree.layout(Size::new(width, 100.0));
        let radii = |frame: &Frame, color| {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillCornerRect {
                        rect: painted,
                        color: ink,
                        radii,
                    } if *painted == rect && *ink == color => Some(*radii),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(radii(&hovered, hover), [CornerRadii::top(12.0)]);
        assert_eq!(hovered.hit_test(point), Some("rounded-row"));
        tree.handle_event(UiEvent::PointerDown(point));
        let down = tree.layout(Size::new(width, 100.0));
        assert_eq!(radii(&down, pressed), [CornerRadii::top(12.0)]);
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            [UiAction::Clicked {
                id: "rounded-row".into()
            }]
        );
        assert!(tree.focus("rounded-row"));
        let focused = tree.layout(Size::new(width, 100.0));
        assert_eq!(
            focused
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::StrokeCornerRect {
                        rect: painted,
                        width: stroke,
                        radii,
                        ..
                    } if *painted == rect && *stroke == 2.0 => Some(*radii),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [CornerRadii::top(12.0)]
        );
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
                alt: false
            }),
            [UiAction::Clicked {
                id: "rounded-row".into()
            }]
        );
        let mut clips = 0_i32;
        for command in focused.commands {
            match command {
                DrawCommand::PushClip(_) => clips += 1,
                DrawCommand::PopClip => clips -= 1,
                _ => {}
            }
            assert!(clips >= 0);
        }
        assert_eq!(clips, 0);
    }
}
use crate::{Component, ComponentPalette, Tooltip};

#[test]
fn row_measure_wraps_flex_column_at_assigned_width() {
    let title_ink = Color::rgb(191, 210, 221);
    let description_ink = Color::rgb(112, 140, 165);
    let note_surface = Color::rgb(32, 39, 43);
    for width in [180.0, 360.0, 900.0] {
        let note = Element::row(vec![
            Element::icon(Icon::Settings).size(Some(16.0), Some(16.0)),
            Element::column(vec![
                Element::text("Authentication belongs to the selected provider connection")
                    .font_family(FontFamily::Sans)
                    .foreground(title_ink),
                Element::text("Connect the provider with its own credential, then refresh the catalog to inspect the models declared by that account. A listed model does not prove that generation has been accepted.")
                    .font_family(FontFamily::Sans)
                    .foreground(description_ink),
            ])
            .gap(5.0)
            .flex(1.0),
        ])
        .gap(10.0)
        .padding(16.0)
        .background(note_surface);
        let mut tree = UiTree::new(
            Element::column(vec![
                note,
                Element::button("following-row", "Following row").size(None, Some(28.0)),
            ])
            .gap(8.0),
        );
        let frame = tree.layout(Size::new(width, 800.0));
        let note_rect = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == note_surface => {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        let last_copy_bottom = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    position, color, ..
                } if *color == description_ink => {
                    Some(position.y + FontMetrics::default().line_height)
                }
                _ => None,
            })
            .fold(0.0_f32, f32::max);
        let following = frame
            .regions
            .iter()
            .find(|region| region.id == "following-row")
            .unwrap();
        assert!(
            last_copy_bottom <= note_rect.y + note_rect.height - 16.0 + 0.1,
            "wrapped copy escapes its measured note at {width}: bottom={last_copy_bottom}, note={note_rect:?}"
        );
        assert!(
            following.rect.y >= last_copy_bottom + 16.0 + 8.0 - 0.1,
            "next row overlaps wrapped note at {width}"
        );
    }
}

#[test]
fn row_measure_keeps_inline_switch_and_next_setting_below_wrapped_copy() {
    // A switch, off: a 36 by 20 track around a strip whose selection is its
    // 14-pixel thumb.
    let switch = || {
        let p = ComponentPalette::neutral_dark();
        let slot = |name: &str| Element::spacer().size(Some(14.0), Some(14.0)).indicator_slot(name);
        let thumb = Element::tab_strip(vec![slot("off"), Element::spacer().size(Some(2.0), Some(14.0)), slot("on")], Vec::new())
            .tab_selection("approval-switch:thumb", "off", p.text, 7.0, crate::tokens().motion.move_ms);
        Element::clickable("approval-switch", thumb)
            .hover_background(crate::lighter(p.selected, 10))
            .pressed_background(crate::lighter(p.selected, 16))
            .transition_ms(crate::hover_ms())
            .background(p.selected)
            .padding(3.0)
            .radius(10.0)
            .size(Some(36.0), Some(20.0))
            .accessible(crate::Access::new(crate::Role::Switch).id("approval-switch").toggled(false).disabled(false))
    };

    let description_ink = Color::rgb(121, 135, 147);
    let row_surface = Color::rgb(28, 32, 35);
    for width in [180.0, 360.0, 900.0] {
        // This is the shared Settings row shape: flexible label and
        // description column, fixed-width inline switch, padding and gap.
        let setting = Element::row(vec![
            Element::column(vec![
                Element::text("Require approval before running a workspace command")
                    .font_family(FontFamily::Sans)
                    .font_size(14.0),
                Element::text("Every command requires a persisted permission decision before it is sent to the native execution service. This preference applies to the current agent profile.")
                    .font_family(FontFamily::Sans)
                    .font_size(12.0)
                    .foreground(description_ink),
            ])
            .gap(6.0)
            .flex(1.0),
            switch(),
        ])
        .gap(24.0)
        .min_size(0.0, 54.0)
        .padding(16.0)
        .background(row_surface);
        let mut tree = UiTree::new(
            Element::column(vec![
                setting,
                Element::button("next-setting", "Next setting").size(None, Some(28.0)),
            ])
            .gap(1.0),
        );
        let frame = tree.layout(Size::new(width, 800.0));
        let row_rect = frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == row_surface => {
                    Some(*rect)
                }
                _ => None,
            })
            .unwrap();
        let switch = frame
            .regions
            .iter()
            .find(|region| region.id == "approval-switch")
            .unwrap();
        let following = frame
            .regions
            .iter()
            .find(|region| region.id == "next-setting")
            .unwrap();
        let last_copy_bottom = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text {
                    position,
                    color,
                    font_size,
                    ..
                } if *color == description_ink => {
                    Some(position.y + FontMetrics::default().at_size(*font_size).line_height)
                }
                _ => None,
            })
            .fold(0.0_f32, f32::max);
        assert_eq!(switch.rect.width, 36.0);
        assert!(switch.rect.x + switch.rect.width <= row_rect.x + row_rect.width - 16.0 + 0.1);
        assert!(
            following.rect.y >= last_copy_bottom + 16.0 + 1.0 - 0.1,
            "next setting overlaps wrapped copy at {width}"
        );
        let switch_point = Point::new(switch.rect.x + 18.0, switch.rect.y + 10.0);
        tree.handle_event(UiEvent::PointerDown(switch_point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(switch_point)),
            vec![UiAction::Clicked {
                id: "approval-switch".into()
            }]
        );
    }
}

#[test]
fn effort_slider_previews_pointer_capture_and_snaps_to_dynamic_stops() {
    let mut tree =
        UiTree::new(Element::effort_slider("effort", 0.0, 5).size(Some(220.0), Some(42.0)));
    let frame = tree.layout(Size::new(220.0, 42.0));
    assert!(frame.regions.iter().any(|region| region.id == "effort"));
    let down = tree.handle_event(UiEvent::PointerDown(Point::new(12.0, 21.0)));
    assert_eq!(
        down,
        vec![UiAction::SliderChanged {
            id: "effort".into(),
            fraction: 0.0,
            committed: false,
        }]
    );
    let preview = tree.handle_event(UiEvent::PointerMove(Point::new(170.0, 21.0)));
    assert!(matches!(
        preview.as_slice(),
        [UiAction::SliderChanged {
            id,
            fraction,
            committed: false,
        }] if id == "effort" && *fraction > 0.75 && *fraction < 0.85
    ));
    let release = tree.handle_event(UiEvent::PointerUp(Point::new(170.0, 21.0)));
    assert_eq!(
        release,
        vec![UiAction::SliderChanged {
            id: "effort".into(),
            fraction: 0.75,
            committed: true,
        }]
    );
    tree.handle_event(UiEvent::PointerDown(Point::new(170.0, 21.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(500.0, 21.0))),
        vec![UiAction::SliderChanged {
            id: "effort".into(),
            fraction: 1.0,
            committed: true,
        }]
    );
    assert!(tree.is_animating());
}

#[test]
fn effort_slider_magnetism_uses_the_advertised_stop_count() {
    let rect = Rect::new(0.0, 0.0, 220.0, 38.0);
    let five = SliderGeometry {
        rect,
        steps: 5,
        fraction: 0.0,
        ultra_available: true,
        inset: SLIDER_THUMB_INSET,
        animated: true,
        y: None,
        vertical: false,
    };
    let three = SliderGeometry { steps: 3, ..five };
    let x = SLIDER_THUMB_INSET + 0.3 * (rect.width - 2.0 * SLIDER_THUMB_INSET);
    let x = Point::new(x, 0.0);
    assert!(five.fraction_at(x) < 0.3, "five stops attract toward 0.25");
    assert!(three.fraction_at(x) > 0.3, "three stops attract toward 0.5");
    assert_eq!(five.snap(five.fraction_at(x)), 0.25);
    assert_eq!(three.snap(three.fraction_at(x)), 0.5);
}

#[test]
fn effort_slider_spring_settles_before_ultra_celebration() {
    let start = Instant::now();
    let mut motion = SliderMotion::new(start);
    motion.sample(start, 0.5, None, 5, true, false);
    motion.animate_to(1.0, start, true);
    let (_, near_end, _, burst) =
        motion.sample(start + Duration::from_millis(82), 1.0, None, 5, true, false);
    assert!(near_end > 1.0, "the knob briefly overshoots its final stop");
    assert!(burst.is_none());
    let (_, settled, _, burst) =
        motion.sample(start + SLIDER_SETTLE_DURATION, 1.0, None, 5, true, false);
    assert_eq!(settled, 1.0);
    assert_eq!(burst, Some(0.0));
    assert!(motion.is_fast(start + SLIDER_SETTLE_DURATION));
    let (_, _, _, burst) = motion.sample(
        start + SLIDER_SETTLE_DURATION + SLIDER_BURST_DURATION,
        1.0,
        None,
        5,
        true,
        false,
    );
    assert!(burst.is_none());

    let mut reduced = SliderMotion::new(start);
    reduced.sample(start, 0.5, None, 5, true, true);
    reduced.animate_to(1.0, start, true);
    let (_, position, _, burst) =
        reduced.sample(start + Duration::from_millis(82), 1.0, None, 5, true, true);
    assert_eq!(position, 1.0);
    assert!(burst.is_none());

    let mut limited_model = SliderMotion::new(start);
    limited_model.sample(start, 0.5, None, 3, false, false);
    limited_model.animate_to(1.0, start, false);
    let (_, _, ultra_mix, burst) =
        limited_model.sample(start + SLIDER_SETTLE_DURATION, 1.0, None, 3, false, false);
    assert_eq!(
        ultra_mix, 0.0,
        "a model ending at High has no Ultra gradient"
    );
    assert!(burst.is_none(), "a model ending at High has no Ultra burst");
}

#[test]
fn effort_slider_reduced_motion_freezes_effects_but_keeps_input() {
    let mut tree =
        UiTree::new(Element::effort_slider("effort", 0.5, 3).size(Some(220.0), Some(42.0)));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(220.0, 42.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::EffortSlider {
            reduced_motion: true,
            phase,
            ..
        } if *phase == 0.0
    )));
    tree.handle_event(UiEvent::PointerDown(Point::new(110.0, 21.0)));
    assert!(!tree.is_animating());
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(165.0, 21.0))),
        vec![UiAction::SliderChanged {
            id: "effort".into(),
            fraction: 1.0,
            committed: true,
        }]
    );
}

#[test]
fn effort_slider_keyboard_steps_follow_model_level_count() {
    let mut tree =
        UiTree::new(Element::effort_slider("effort", 0.5, 5).size(Some(220.0), Some(42.0)));
    tree.layout(Size::new(220.0, 42.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(110.0, 21.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(110.0, 21.0)));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            shift: false,
            ctrl: false,
            alt: false,
        }),
        vec![UiAction::SliderChanged {
            id: "effort".into(),
            fraction: 0.75,
            committed: true,
        }]
    );
}

#[test]
fn effort_slider_animates_only_while_visible_and_freezes_for_reduced_motion() {
    let slider = || Element::effort_slider("effort", 0.5, 4).size(Some(220.0), Some(42.0));
    let mut tree = UiTree::new(slider());
    let viewport = Size::new(220.0, 42.0);
    let start = Instant::now();
    let phase = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::EffortSlider { phase, .. } => Some(*phase),
            _ => None,
        })
    };
    let initial = phase(&tree.layout_at(viewport, start)).unwrap();
    assert_eq!(
        tree.animation_repaint_after(),
        Some(Duration::from_millis(33))
    );
    assert_eq!(
        phase(&tree.layout_at(viewport, start + Duration::from_millis(16))),
        Some(initial)
    );
    assert!(
        phase(&tree.layout_at(viewport, start + Duration::from_millis(34))).unwrap() > initial
    );

    tree.set_root(Element::spacer());
    tree.layout_at(viewport, start + Duration::from_millis(50));
    assert_eq!(tree.animation_repaint_after(), None);

    tree.set_reduced_motion(true);
    tree.set_root(slider());
    let frozen = phase(&tree.layout_at(viewport, start + Duration::from_millis(84))).unwrap();
    assert_eq!(
        phase(&tree.layout_at(viewport, start + Duration::from_millis(134))),
        Some(frozen)
    );
    assert_eq!(tree.animation_repaint_after(), None);
}

#[test]
fn a_frame_lists_the_ids_two_controls_took_but_not_a_link_linked_twice() {
    let link = || Element::formatted_text(vec![TextSpan::new("docs", Color::rgb(0, 0, 0), 14.0, FontFamily::Sans).action("open:docs")]);
    let root = Element::column(vec![
        Element::button("save", "Save"),
        Element::clickable("save", Element::text("Save again")),
        Element::text_input("name", "", "Name"),
        Element::text_input("name", "", "Name again"),
        Element::button("save", "Third"),
        Element::button("cancel", "Cancel"),
        link(),
        link(),
    ]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(Size::new(400.0, 400.0));
    assert_eq!(frame.duplicate_ids(), ["save", "name"]);
    tree.set_root(Element::column(vec![Element::button("save", "Save"), Element::button("cancel", "Cancel")]));
    assert!(tree.layout(Size::new(400.0, 400.0)).duplicate_ids().is_empty());
}

#[test]
fn state_out_of_sight_is_kept_only_for_the_ids_laid_out_most_recently() {
    let viewport = Size::new(300.0, 200.0);
    let page = |id: &str| {
        Element::column(vec![
            Element::scroll(id, Element::column((0..20).map(|_| Element::text("row")).collect()))
                .size(None, Some(100.0)),
            Element::text_input(format!("{id}:field"), "draft", "Reply"),
        ])
    };
    let mut tree = UiTree::new(page("tab"));
    tree.layout(viewport);
    tree.set_scroll_offset("tab", 40.0);
    tree.layout(viewport);
    // A tab switched away from and back keeps its place.
    for index in 0..10 {
        tree.set_root(page(&format!("other:{index}")));
        tree.layout(viewport);
    }
    tree.set_root(page("tab"));
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("tab"), 40.0);
    // IDs that come and go, one per message, do not pile up.
    for index in 0..200 {
        tree.set_root(page(&format!("message:{index}")));
        tree.layout(viewport);
    }
    assert_eq!(tree.scroll_offsets.len(), DORMANT_KEPT + 1);
    assert_eq!(tree.inputs.len(), DORMANT_KEPT + 1);
    assert!(tree.scroll_offsets.contains_key("message:199"));
    assert!(tree.scroll_offsets.contains_key("message:136"));
    assert!(!tree.scroll_offsets.contains_key("tab"));
}

#[test]
fn clip_children_limits_descendant_hitboxes_without_clipping_parent_surface() {
    let parent_color = Color::rgb(29, 31, 34);
    let clipped = Element::panel(Element::row(vec![
        Element::button("wide_child", "Action").size(Some(150.0), Some(28.0))
    ]))
    .background(parent_color)
    .border(Color::rgb(70, 70, 70), 1.0)
    .padding(4.0)
    .clip_children()
    .size(Some(80.0), Some(40.0));
    let frame = UiTree::new(clipped).layout(Size::new(80.0, 40.0));
    let child = frame
        .regions
        .iter()
        .find(|region| region.id == "wide_child")
        .unwrap();
    assert_eq!(child.rect, Rect::new(4.0, 4.0, 72.0, 28.0));
    assert_eq!(frame.hit_test(Point::new(75.0, 16.0)), Some("wide_child"));
    assert_eq!(frame.hit_test(Point::new(90.0, 16.0)), None);
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. }
            if *rect == Rect::new(0.0, 0.0, 80.0, 40.0) && *color == parent_color
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::PushClip(rect) if *rect == Rect::new(4.0, 4.0, 72.0, 32.0)
    )));
}

#[test]
fn tab_indicator_moves_without_moving_click_regions_and_reduced_motion_snaps() {
    let palette = ComponentPalette::neutral_dark();
    // Tabs of 80 and 96 on a track 2 pixels around them, under a selection
    // that slides between them.
    let make_tabs = |active: &str| {
        let tab = |id: &str, label: &str, width: f32| {
            let chosen = id == active;
            let (ink, hover) = if chosen { (palette.text, crate::lighter(palette.text, 12)) } else { (palette.muted, palette.text) };
            Element::button(id, label)
                .background(Color::TRANSPARENT)
                .hover_foreground(hover)
                .pressed_foreground(hover)
                .foreground(ink)
                .font_family(FontFamily::Sans)
                .font_size(11.5)
                .padding(4.0)
                .gap(4.0)
                .radius(crate::control_radius())
                .center_content()
                .size(Some(width), Some(24.0))
                .accessible(crate::Access::new(crate::Role::Tab).selected(chosen))
        };
        Element::tab_strip(vec![tab("group", "Grupo", 80.0), tab("project", "Proyecto", 96.0)], vec!["group".into(), "project".into()])
            .gap(2.0)
            .padding(2.0)
            .size(None, Some(28.0))
            .background(palette.surface)
            .radius(crate::surface_radius())
            .tab_selection("session-grouping", active, palette.selected, crate::control_radius(), crate::tokens().motion.move_ms)
    };
    let start = Instant::now();
    let mut tree = UiTree::new(make_tabs("group"));
    let viewport = Size::new(240.0, 32.0);
    let initial = tree.layout_at(viewport, start);
    let indicator = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == palette.selected => {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("selected tab must have a painted indicator")
    };
    let start_rect = indicator(&initial);
    let project_region = initial
        .regions
        .iter()
        .find(|region| region.id == "project")
        .unwrap()
        .rect;
    tree.set_root(make_tabs("project"));
    let switched = tree.layout_at(viewport, start + Duration::from_millis(10));
    assert_eq!(indicator(&switched).x, start_rect.x);
    let halfway = tree.layout_at(viewport, start + Duration::from_millis(95));
    assert!(indicator(&halfway).x > start_rect.x);
    assert!(indicator(&halfway).x < project_region.x);
    assert_eq!(
        halfway.hit_test(Point::new(project_region.x + 5.0, 16.0)),
        Some("project")
    );
    // A spring covers 99% of the way in the duration and then settles.
    let arriving = tree.layout_at(viewport, start + Duration::from_millis(180));
    assert!((indicator(&arriving).x - project_region.x).abs() < 1.5);
    let complete = tree.layout_at(viewport, start + Duration::from_millis(600));
    assert_eq!(indicator(&complete).x, project_region.x);
    tree.set_reduced_motion(true);
    tree.set_root(make_tabs("group"));
    assert_eq!(
        indicator(&tree.layout_at(viewport, start + Duration::from_millis(190))).x,
        start_rect.x
    );
}

#[test]
fn popover_entrance_fades_paint_but_keeps_stable_targets() {
    let popup_color = Color::rgb(71, 75, 81);
    let make_open = || {
        Element::popover(
            Element::button("trigger", "Abrir").size(Some(92.0), Some(30.0)),
            Element::panel(Element::button("option", "Opción").size(Some(112.0), Some(34.0)))
                .background(popup_color)
                .size(Some(124.0), Some(42.0)),
            "dismiss",
            PopoverPlacement::BottomStart,
        )
    };
    let start = Instant::now();
    let viewport = Size::new(300.0, 200.0);
    let mut tree = UiTree::new(make_open());
    let first = tree.layout_at(viewport, start);
    let target = first
        .regions
        .iter()
        .find(|region| region.id == "option")
        .unwrap()
        .rect;
    let popup_fill = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. }
                    if (color.r, color.g, color.b)
                        == (popup_color.r, popup_color.g, popup_color.b) =>
                {
                    Some((*rect, *color))
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(popup_fill(&first).1.a, 0);
    let halfway = tree.layout_at(viewport, start + Duration::from_millis(70));
    let (animated_rect, animated_color) = popup_fill(&halfway);
    assert!(animated_color.a > 0 && animated_color.a < 255);
    assert_eq!(
        halfway.hit_test(Point::new(target.x + 8.0, target.y + 8.0)),
        Some("option")
    );
    let complete = tree.layout_at(viewport, start + Duration::from_millis(140));
    assert_eq!(popup_fill(&complete).1.a, 255);
    assert!((animated_rect.y - popup_fill(&complete).0.y).abs() > 0.1);
    assert!(tree.focus("option"));
    tree.set_root(Element::button("trigger", "Abrir"));
    let closed = tree.layout_at(viewport, start + Duration::from_millis(150));
    assert!(tree.popover_transitions.is_empty());
    assert_eq!(tree.focused_id(), Some("trigger"));
    assert_eq!(popup_fill(&closed).1.a, 255);
    assert!(!closed.regions.iter().any(|region| {
        region.id == "option"
            || region.id == "dismiss"
            || region.id == "popover_surface:dismiss"
    }));
    assert_ne!(
        closed.hit_test(Point::new(target.x + 8.0, target.y + 8.0)),
        Some("option")
    );
    assert!(tree
        .handle_event(UiEvent::Key {
            key: Key::Escape,
            shift: false,
            ctrl: false,
            alt: false,
        })
        .is_empty());
    let exiting = tree.layout_at(viewport, start + Duration::from_millis(205));
    assert!(popup_fill(&exiting).1.a > 0 && popup_fill(&exiting).1.a < 255);
    let gone = tree.layout_at(viewport, start + Duration::from_millis(260));
    assert!(!gone.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. }
            if (color.r, color.g, color.b)
                == (popup_color.r, popup_color.g, popup_color.b)
    )));
    tree.set_reduced_motion(true);
    tree.set_root(make_open());
    let reduced = tree.layout_at(viewport, start + Duration::from_millis(270));
    assert_eq!(popup_fill(&reduced).1.a, 255);
    tree.set_root(Element::button("trigger", "Abrir"));
    let reduced_close = tree.layout_at(viewport, start + Duration::from_millis(280));
    assert!(!reduced_close.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. }
            if (color.r, color.g, color.b)
                == (popup_color.r, popup_color.g, popup_color.b)
    )));
}

#[test]
fn opt_in_popover_scale_and_close_duration_never_move_hit_targets() {
    let ink = Color::rgb(86, 89, 92);
    let open = || {
        Element::popover(
            Element::button("trigger", "Open").size(Some(90.0), Some(32.0)),
            Element::panel(Element::button("choice", "Choose"))
                .background(ink)
                .size(Some(160.0), Some(64.0)),
            "dismiss",
            PopoverPlacement::BottomStart,
        )
        .popover_animation_ms(200)
        .popover_close_animation_ms(150)
        .popover_open_scale(0.96)
    };
    let surface = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. }
                if (color.r, color.g, color.b) == (ink.r, ink.g, ink.b) =>
            {
                Some((*rect, color.a))
            }
            _ => None,
        })
    };
    let start = Instant::now();
    let viewport = Size::new(400.0, 260.0);
    let mut tree = UiTree::new(open());
    let first = tree.layout_at(viewport, start);
    let choice = first
        .regions
        .iter()
        .find(|region| region.id == "choice")
        .unwrap();
    let choice_point = Point::new(choice.rect.x + 4.0, choice.rect.y + 4.0);
    assert_eq!(first.hit_test(choice_point), Some("choice"));
    let middle = tree.layout_at(viewport, start + Duration::from_millis(100));
    let (scaled_rect, alpha) = surface(&middle).unwrap();
    assert!(alpha > 0 && alpha < 255);
    assert!(scaled_rect.width < 160.0);
    assert_eq!(middle.hit_test(choice_point), Some("choice"));
    tree.layout_at(viewport, start + Duration::from_millis(200));
    tree.set_root(Element::button("trigger", "Open"));
    let closed = tree.layout_at(viewport, start + Duration::from_millis(210));
    assert!(surface(&closed).is_some());
    assert_ne!(closed.hit_test(choice_point), Some("choice"));
    assert!(!closed.regions.iter().any(|region| region.id == "choice"));
    let fading = tree.layout_at(viewport, start + Duration::from_millis(330));
    assert!(surface(&fading).is_some_and(|(_, alpha)| alpha > 0));
    let gone = tree.layout_at(viewport, start + Duration::from_millis(361));
    assert!(surface(&gone).is_none());
}

#[test]
fn keyed_popover_crossfades_and_morphs_without_old_hit_regions() {
    let surface_color = Color::rgb(71, 74, 79);
    let view = |key: &str, id: &str, label: &str, height: f32| {
        Element::popover(
            Element::button("trigger", "Open").size(Some(90.0), Some(32.0)),
            Element::panel(Element::button(id, label))
                .background(surface_color)
                .size(Some(180.0), Some(height)),
            "dismiss",
            PopoverPlacement::BottomStart,
        )
        .popover_animation_ms(0)
        .popover_view_key(key)
    };
    let shell = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == surface_color => Some(*rect),
            _ => None,
        })
    };
    let start = Instant::now();
    let viewport = Size::new(420.0, 320.0);
    let mut tree = UiTree::new(view("overview", "old", "Old", 70.0));
    let old = tree.layout_at(viewport, start);
    assert_eq!(shell(&old).unwrap().height, 70.0);
    tree.set_root(view("details", "new", "New", 130.0));
    let switching = tree.layout_at(viewport, start + Duration::from_millis(10));
    assert_eq!(shell(&switching).unwrap().height, 70.0);
    assert!(!switching.regions.iter().any(|region| region.id == "old"));
    let new_region = switching
        .regions
        .iter()
        .find(|region| region.id == "new")
        .unwrap();
    assert_eq!(
        switching.hit_test(Point::new(new_region.rect.x + 4.0, new_region.rect.y + 4.0)),
        Some("new")
    );
    let middle = tree.layout_at(viewport, start + Duration::from_millis(140));
    let height = shell(&middle).unwrap().height;
    assert!(height > 70.0 && height < 130.0);
    assert!(middle.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, color, .. } if text == "Old" && color.a > 0 && color.a < 255
    )));
    assert!(middle.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, color, .. } if text == "New" && color.a > 0 && color.a < 255
    )));
    let finished = tree.layout_at(viewport, start + Duration::from_millis(271));
    assert_eq!(shell(&finished).unwrap().height, 130.0);
    assert!(!finished.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "Old"
    )));
    tree.set_reduced_motion(true);
    tree.set_root(view("overview", "old", "Old", 70.0));
    let reduced = tree.layout_at(viewport, start + Duration::from_millis(280));
    assert_eq!(shell(&reduced).unwrap().height, 70.0);
    assert!(!reduced.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "New"
    )));
}

#[test]
fn closing_during_popover_entrance_preserves_current_opacity() {
    let start = Instant::now();
    let viewport = Size::new(300.0, 200.0);
    let popup_color = Color::rgb(111, 112, 113);
    let open = || {
        Element::popover(
            Element::button("trigger", "Open"),
            Element::panel(Element::text("Item"))
                .background(popup_color)
                .size(Some(100.0), Some(40.0)),
            "dismiss",
            PopoverPlacement::BottomStart,
        )
    };
    let mut tree = UiTree::new(open());
    tree.layout_at(viewport, start);
    let entering = tree.layout_at(viewport, start + Duration::from_millis(40));
    let opacity = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { color, .. }
                if (color.r, color.g, color.b)
                    == (popup_color.r, popup_color.g, popup_color.b) =>
            {
                Some(color.a)
            }
            _ => None,
        })
    };
    let visible = opacity(&entering).unwrap();
    assert!(visible > 0 && visible < 255);
    tree.set_root(Element::button("trigger", "Open"));
    let closing = tree.layout_at(viewport, start + Duration::from_millis(40));
    assert_eq!(opacity(&closing), Some(visible));
    let fading = tree.layout_at(viewport, start + Duration::from_millis(80));
    let fading_alpha = opacity(&fading).unwrap();
    assert!(fading_alpha < visible);
    tree.set_root(open());
    let reopened = tree.layout_at(viewport, start + Duration::from_millis(80));
    assert_eq!(opacity(&reopened), Some(fading_alpha));
    assert!(tree.popover_exits.is_empty());
}

#[test]
fn keyboard_shortcut_caps_use_proportional_measurement_and_remain_passive() {
    let fill = Color::rgba(235, 235, 235, 25);
    let ink = Color::rgb(235, 235, 235);
    let mut tree = UiTree::new(Element::row(vec![
        Element::keyboard_shortcut("WW+ii", fill, None)
            .foreground(ink)
            .size(None, Some(shortcut::HEIGHT)),
        Element::button("next", "Next").size(Some(50.0), Some(30.0)),
    ]));
    tree.set_text_measurer(|text, size, family| {
        if size == shortcut::FONT_SIZE {
            assert_eq!(family, FontFamily::Sans);
            text.chars()
                .map(|character| if character == 'W' { 12.0 } else { 2.0 })
                .sum()
        } else {
            text.chars().count() as f32 * 7.0
        }
    });
    let frame = tree.layout(Size::new(160.0, 30.0));
    let caps: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect {
                rect,
                color,
                radius,
            } if *color == fill => {
                assert_eq!(*radius, shortcut::radius());
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(caps.len(), 2);
    assert_eq!(caps[0].width, shortcut::cap_width(24.0));
    assert_eq!(caps[1].width, shortcut::cap_width(4.0));
    assert_eq!(
        caps[1].x - caps[0].x - caps[0].width,
        shortcut::key_gap(|separator| separator.len() as f32 * 2.0)
    );
    let labels: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                position,
                text,
                color,
                font_family,
                ..
            } if *color == ink => {
                assert_eq!(*font_family, FontFamily::Sans);
                Some((*position, text.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        labels.iter().map(|(_, label)| *label).collect::<Vec<_>>(),
        ["WW", "ii"]
    );
    let line_height = tree.sans_metrics.at_size(shortcut::FONT_SIZE).line_height;
    for ((position, _), (cap, text_width)) in labels.iter().zip(caps.iter().zip([24.0, 4.0])) {
        assert!((position.x + text_width / 2.0 - cap.x - cap.width / 2.0).abs() < 0.01);
        assert!((position.y + line_height / 2.0 - cap.y - cap.height / 2.0).abs() < 0.01);
    }
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, color, .. }
            if text == "+" && *color == shortcut::separator_ink(ink))));
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::StrokeRect { rect, color, .. }
            if *rect == caps[0] && *color == shortcut::cap_border(ink))));
    assert_eq!(frame.regions.len(), 1);
    assert_eq!(frame.regions[0].id, "next");
    assert_eq!(frame.regions[0].rect.x, caps[1].x + caps[1].width);
    assert_eq!(frame.hit_test(Point::new(5.0, 15.0)), None);
    assert!(
        frame.text_blocks.is_empty(),
        "decorative caps are not selectable text"
    );
}

#[test]
fn keyboard_shortcut_omits_whole_chord_when_bounded_and_keeps_literal_plus() {
    let fill = Color::rgba(235, 235, 235, 25);
    let shortcut = |value: &str, max_width| {
        Element::keyboard_shortcut(value, fill, max_width).size(None, Some(shortcut::HEIGHT))
    };
    let mut tree = UiTree::new(Element::row(vec![
        shortcut("Ctrl++", Some(1.0)),
        Element::button("next", "Next").size(Some(50.0), Some(30.0)),
    ]));
    let bounded = tree.layout(Size::new(160.0, 30.0));
    assert_eq!(bounded.regions[0].rect.x, 0.0);
    assert!(!bounded.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { color, .. } if *color == fill
    )));
    tree.set_root(Element::column(vec![shortcut("Ctrl++", None)]));
    let narrow = tree.layout(Size::new(20.0, 30.0));
    assert!(!narrow.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { color, .. } if *color == fill
    )));
    tree.set_root(Element::column(vec![shortcut("Ctrl++", None)]));
    let wide = tree.layout(Size::new(160.0, 30.0));
    let labels: Vec<_> = wide
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Ctrl", "+", "+"], "the joining plus, then the literal key");
    tree.set_root(shortcut("   ", None));
    let empty = tree.layout(Size::new(160.0, 30.0));
    assert!(empty.commands.is_empty());
    assert!(empty.regions.is_empty());
}

#[test]
fn input_shortcut_preserves_text_space_and_uses_text_ink() {
    let ink = Color::rgb(235, 235, 235);
    let muted = Color::rgb(120, 120, 120);
    let search = |width| {
        Element::text_input("search", "", "Find")
            .leading_icon(Icon::Search)
            .leading_icon_tint(muted)
            .input_shortcut("Ctrl+K")
            .foreground(ink)
            .size(Some(width), Some(34.0))
    };
    let mut tree = UiTree::new(search(200.0));
    let wide = tree.layout(Size::new(200.0, 34.0));
    for key in ["Ctrl", "K"] {
        assert!(wide.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, color, font_family, .. }
                if text == key && *color == ink && *font_family == FontFamily::Sans
        )));
    }
    assert!(wide.commands.iter().any(|command| matches!(command,
        DrawCommand::Icon { icon: Icon::Search, tint, .. } if *tint == muted
    )));
    assert!(wide
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == shortcut::cap_fill(ink) =>
                Some(rect),
            _ => None,
        })
        .all(|rect| rect.height == shortcut::HEIGHT
            && rect.y >= 0.0
            && rect.y + rect.height <= 34.0));
    let wide_text = tree.inputs["search"].content_width;
    assert!(wide_text >= 24.0);
    tree.set_root(search(80.0));
    let narrow = tree.layout(Size::new(80.0, 34.0));
    assert!(!narrow.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K"
    )));
    assert_eq!(tree.inputs["search"].content_width, 40.0);
    assert_eq!(narrow.regions.len(), 1);
    assert_eq!(narrow.regions[0].id, "search");
    assert!(tree.focus("search"));
    let focused = tree.layout(Size::new(80.0, 34.0));
    assert!(!focused.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "Ctrl" || text == "K"
    )));
}

#[test]
fn a_control_without_a_ring_shows_its_focus_border_under_the_keyboard_only() {
    let mark = Color::rgb(1, 2, 3);
    let marked = |frame: &Frame| {
        frame.commands.iter().any(|command| {
            matches!(command, DrawCommand::StrokeRect { color, width, .. }
                if *color == mark && *width == 1.5)
        })
    };
    let button = |ring: bool| {
        Element::button("go", "Go")
            .border(Color::rgb(9, 9, 9), 1.0)
            .focus_border(mark, 1.5)
            .focus_ring(ring)
            .size(Some(80.0), Some(28.0))
    };
    let window = Size::new(80.0, 28.0);
    let mut tree = UiTree::new(button(false));
    assert!(!marked(&tree.layout(window)));
    assert!(tree.focus_from_pointer("go"));
    assert!(!marked(&tree.layout(window)));
    assert!(tree.focus("go"));
    assert!(marked(&tree.layout(window)));
    // With its ring, a control keeps the ring and its own border.
    tree.set_root(button(true));
    assert!(!marked(&tree.layout(window)));
}

#[test]
fn a_field_brightens_its_border_under_the_pointer_and_its_focus_wins() {
    let (rest, over, focus) = (Color::rgb(60, 60, 60), Color::rgb(180, 180, 180), Color::rgb(255, 255, 255));
    let stroke = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
        DrawCommand::StrokeRect { color, .. } => Some(*color),
        _ => None,
    });
    let field = Element::text_input("name", "", "")
        .border(rest, 1.0)
        .hover_border(over, 1.0)
        .focus_border(focus, 1.5)
        .size(Some(200.0), Some(48.0));
    let window = Size::new(200.0, 48.0);
    let mut tree = UiTree::new(field);
    tree.set_reduced_motion(true);
    assert_eq!(stroke(&tree.layout(window)), Some(rest));
    tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 24.0)));
    assert_eq!(stroke(&tree.layout(window)), Some(over));
    assert!(tree.focus("name"));
    assert_eq!(stroke(&tree.layout(window)), Some(focus));
}

#[test]
fn input_trailing_space_keeps_the_end_free_of_text_caret_and_hint() {
    let field = |trailing: f32| {
        Element::text_input("search", "", "Find")
            .leading_icon(Icon::Search)
            .input_shortcut("K")
            .input_trailing_space(trailing)
            .size(Some(200.0), Some(34.0))
    };
    let hint_right = |frame: &Frame| {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, .. } if text == "K" => Some(position.x),
                _ => None,
            })
            .next()
            .expect("the hint")
    };
    let mut tree = UiTree::new(field(0.0));
    let plain = tree.layout(Size::new(200.0, 34.0));
    let (plain_x, plain_width) = (
        tree.inputs["search"].content_x,
        tree.inputs["search"].content_width,
    );
    tree.set_root(field(24.0));
    let kept = tree.layout(Size::new(200.0, 34.0));
    // The start is where it was; the text, and the hint at its end, give up the room.
    assert_eq!(tree.inputs["search"].content_x, plain_x);
    assert_eq!(tree.inputs["search"].content_width, plain_width - 24.0);
    assert_eq!(hint_right(&kept), hint_right(&plain) - 24.0);
    assert_eq!(
        kept.regions[0].rect, plain.regions[0].rect,
        "the input's own box is unchanged"
    );
    // More than the field has leaves no room, never a negative one.
    tree.set_root(field(500.0));
    tree.layout(Size::new(200.0, 34.0));
    assert_eq!(tree.inputs["search"].content_width, 0.0);
    // Not a number, or another kind of element: nothing is kept.
    for element in [
        field(f32::NAN),
        Element::spacer().input_trailing_space(24.0),
    ] {
        assert!(element.kept.is_none());
    }
}

#[test]
fn persistent_input_placeholder_survives_focus_edits_and_refocus() {
    use crate::{Component, ComponentPalette, TextField};

    let palette = ComponentPalette::neutral_dark();
    let root = || {
        Element::column(vec![
            TextField::new("replace", "", "Reemplazar", palette)
                .persistent_placeholder(true)
                .render(),
            Element::button("other", "Other").size(None, Some(32.0)),
        ])
    };
    let viewport = Size::new(240.0, 80.0);
    let placeholder = |frame: &Frame| {
        frame
            .commands
            .iter()
            .enumerate()
            .find_map(|(index, command)| match command {
                DrawCommand::Text {
                    text,
                    position,
                    color,
                    ..
                } if text == "Reemplazar" => Some((index, *position, *color)),
                _ => None,
            })
    };
    let mut tree = UiTree::new(root());
    tree.set_reduced_motion(true);
    assert!(placeholder(&tree.layout(viewport)).is_some());
    assert!(tree.focus("replace"));
    let focused = tree.layout(viewport);
    let (label_index, label_position, label_color) =
        placeholder(&focused).expect("focused empty field keeps its purpose visible");
    assert_eq!(label_color.a, 110);
    assert!(
        focused.commands[label_index + 1..].iter().any(|command| {
            matches!(command,
                DrawCommand::FillRect { rect, color, .. }
                    if rect.x == label_position.x && rect.y == label_position.y
                        && rect.width == 1.5 && color.a > label_color.a
            )
        }),
        "caret must remain visible in front of the placeholder"
    );

    tree.handle_event(UiEvent::Text("ñ".into()));
    let typed = tree.layout(viewport);
    assert!(placeholder(&typed).is_none());
    assert_eq!(tree.input_value("replace"), Some("ñ"));
    assert!(typed.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, color, .. }
            if text == "ñ" && *color == palette.text
    )));
    tree.handle_event(UiEvent::Key {
        key: Key::Backspace,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert!(placeholder(&tree.layout(viewport)).is_some());
    assert!(tree.focus("other"));
    assert!(placeholder(&tree.layout(viewport)).is_some());
    assert!(tree.focus("replace"));
    tree.set_root(root());
    assert!(placeholder(&tree.layout(viewport)).is_some());
    assert_eq!(tree.focused_id(), Some("replace"));
    assert_eq!(tree.input_value("replace"), Some(""));
}

#[test]
fn input_placeholder_still_hides_on_focus_by_default() {
    let mut tree =
        UiTree::new(Element::text_input("default", "", "Placeholder").size(None, Some(32.0)));
    let viewport = Size::new(240.0, 32.0);
    assert!(tree
        .layout(viewport)
        .commands
        .iter()
        .any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Placeholder"
        )));
    assert!(tree.focus("default"));
    assert!(!tree
        .layout(viewport)
        .commands
        .iter()
        .any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Placeholder"
        )));
}

#[test]
fn a_text_area_keeps_its_placeholder_while_focused_only_when_asked() {
    use crate::{Component, ComponentPalette, TextArea};

    let viewport = Size::new(240.0, 120.0);
    for persistent in [false, true] {
        let area = TextArea::new("notes", "", "Notes", ComponentPalette::neutral_dark())
            .persistent_placeholder(persistent)
            .render();
        let mut tree = UiTree::new(area);
        tree.set_reduced_motion(true);
        tree.layout(viewport);
        assert!(tree.focus("notes"));
        let frame = tree.layout(viewport);
        let placeholder = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Text { text, position, color, .. } if text == "Notes" => Some((*position, color.a)),
            _ => None,
        });
        let caret = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, .. } if rect.width == 1.5 => Some(*rect),
            _ => None,
        });
        match placeholder {
            // Faint, with the caret before it, where the first key goes.
            Some((at, alpha)) => {
                assert!(persistent, "a focused area hides its placeholder by default");
                assert_eq!(alpha, 110);
                assert_eq!(caret.map(|caret| (caret.x, caret.y)), Some((at.x, at.y)));
            }
            None => assert!(!persistent, "the focused empty area keeps its placeholder"),
        }
        tree.handle_event(UiEvent::Text("a".into()));
        assert!(!tree.layout(viewport).commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Notes"
        )));
    }
}

#[test]
fn tooltip_preserves_anchor_bounds_and_has_no_hit_region() {
    let palette = ComponentPalette::neutral_dark();
    let anchor =
        Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0));
    let baseline = UiTree::new(anchor.clone()).layout(Size::new(300.0, 100.0));
    let wrapped = Tooltip::new(anchor, "toggle", "Toggle sidebar", palette)
        .shortcut("Ctrl+B")
        .render();
    let mut tree = UiTree::new(wrapped);
    let frame = tree.layout(Size::new(300.0, 100.0));
    assert_eq!(frame.regions, baseline.regions);
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    assert!(tree.focus("toggle"));
    let focused = tree.layout(Size::new(300.0, 100.0));
    assert_eq!(focused.regions, baseline.regions);
    assert_eq!(
        focused
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::FillTriangle { .. }))
            .count(),
        2,
    );
    // The library's pointer is the outlined pair of triangles, with no stroke round it.
    assert!(!focused
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Polyline { .. })));
    assert!(focused.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Text { text, .. } if text == "Toggle sidebar"
    )));
    let tooltip_ink = focused
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { text, color, .. } if text == "Toggle sidebar" => Some(*color),
            _ => None,
        })
        .unwrap();
    for key in ["Ctrl", "B"] {
        assert!(focused.commands.iter().any(|command| matches!(
            command,
            DrawCommand::Text { text, color, font_family, .. }
                if text == key && *color == tooltip_ink && *font_family == FontFamily::Sans
        )));
    }
}

#[test]
fn tooltip_omits_a_shortcut_that_would_take_the_title_space() {
    let palette = ComponentPalette::neutral_dark();
    let mut tree = UiTree::new(
        Tooltip::new(
            Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
            "toggle",
            "A",
            palette,
        )
        .shortcut("Ctrl+Shift+B")
        .max_width(64.0)
        .render(),
    );
    tree.layout(Size::new(200.0, 100.0));
    assert!(tree.focus("toggle"));
    let frame = tree.layout(Size::new(200.0, 100.0));
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if text == "A"
    )));
    assert!(!frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, .. } if ["Ctrl", "Shift", "B"].contains(&text.as_str())
    )));
    assert_eq!(frame.regions.len(), 1);
}

#[test]
fn a_centered_hint_draws_its_picture_over_its_lines_in_the_middle() {
    let palette = ComponentPalette::neutral_dark();
    let line = |text: &str, size: f32| TextSpan::new(text, Color::WHITE, size, FontFamily::SansMedium);
    let window = Size::new(400.0, 400.0);
    let mut tree = UiTree::new(Element::column(vec![
        Element::spacer().size(None, Some(250.0)),
        Tooltip::new(
            Element::icon_button("badge", Icon::PanelLeft, "").size(Some(20.0), Some(20.0)),
            "badge",
            "",
            palette,
        )
        .picture(inline_picture(), Size::new(96.0, 64.0))
        .lines([line("Account age", 14.0), line("BLOOM", 18.0), line("5 years", 14.0)])
        .centered()
        .render(),
    ]));
    // Every character 8 wide at any size, so widths are known.
    tree.set_text_measurer(|text: &str, _: f32, _: FontFamily| text.chars().count() as f32 * 8.0);
    tree.set_reduced_motion(true);
    tree.layout(window);
    assert!(tree.focus("badge"));
    let frame = tree.layout(window);
    let picture = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::RasterImage { rect, .. } => Some(*rect),
            _ => None,
        })
        .expect("the picture");
    let lines: Vec<(String, Point, f32, f32)> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, position, font_size, .. } => {
                Some((text.clone(), *position, *font_size, text.chars().count() as f32 * 8.0))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        lines.iter().map(|(text, ..)| text.as_str()).collect::<Vec<_>>(),
        ["Account age", "BLOOM", "5 years"]
    );
    assert_eq!((picture.width, picture.height), (96.0, 64.0));
    // Every line is centered on the picture's middle.
    let middle = picture.x + picture.width / 2.0;
    for (text, position, _, width) in &lines {
        assert!((position.x + width / 2.0 - middle).abs() < 1.0, "{text} at {position:?}, {width} wide");
    }
    // The picture stands 12 over the lines, which follow each other.
    assert_eq!(lines[0].1.y, picture.y + picture.height + 12.0);
    assert!(lines[1].1.y > lines[0].1.y && lines[2].1.y > lines[1].1.y);
}

#[test]
fn tooltip_hover_waits_and_keyboard_focus_is_immediate() {
    let palette = ComponentPalette::neutral_dark();
    let mut tree = UiTree::new(
        Tooltip::new(
            Element::icon_button("toggle", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
            "toggle",
            "Toggle sidebar",
            palette,
        )
        .render(),
    );
    tree.layout(Size::new(200.0, 100.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(16.0, 16.0)));
    let started = tree.tooltip_hover_since.as_ref().unwrap().1;
    let early = tree.layout_at(Size::new(200.0, 100.0), started + TOOLTIP_HOVER_DELAY / 2);
    assert!(!early
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    let late = tree.layout_at(Size::new(200.0, 100.0), started + TOOLTIP_HOVER_DELAY);
    assert!(late
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 80.0)));
    let left = tree.layout(Size::new(200.0, 100.0));
    assert!(!left
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    let focused = tree.layout(Size::new(200.0, 100.0));
    assert!(focused
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
}

#[test]
fn a_hint_takes_its_own_delay_title_pointer_and_fill() {
    let palette = ComponentPalette::neutral_dark();
    let fill = Color::rgb(10, 10, 12);
    let hint = |tooltip: Tooltip| {
        let mut tree = UiTree::new(tooltip.render());
        tree.set_reduced_motion(true);
        tree.layout(Size::new(240.0, 120.0));
        tree.handle_event(UiEvent::PointerMove(Point::new(16.0, 16.0)));
        let started = tree.tooltip_hover_since.as_ref().unwrap().1;
        (tree, started)
    };
    let anchor = || Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0));
    let body = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if rect.x > 32.0 => Some((*rect, *color)),
            _ => None,
        })
    };
    // The default waits, is 24 tall, has a pointer and the palette's fill.
    let (mut plain, started) = hint(Tooltip::new(anchor(), "mark", "Server", palette).placement(TooltipPlacement::Right));
    assert!(body(&plain.layout_at(Size::new(240.0, 120.0), started)).is_none());
    assert_eq!(plain.animation_repaint_after_at(started), Some(TOOLTIP_HOVER_DELAY));
    let shown = plain.layout_at(Size::new(240.0, 120.0), started + TOOLTIP_HOVER_DELAY);
    let (rect, color) = body(&shown).unwrap();
    assert_eq!((rect.height, color), (TOOLTIP_HEIGHT, palette.tooltip_style().background));
    assert_eq!(shown.commands.iter().filter(|command| matches!(command, DrawCommand::FillTriangle { .. })).count(), 2);
    // No wait, a larger medium title, no pointer and a fill of its own.
    let (mut quick, started) = hint(
        Tooltip::new(anchor(), "mark", "Server", palette)
            .placement(TooltipPlacement::Right)
            .delay_ms(0)
            .title_font_size(16.0)
            .title_font_size(f32::NAN)
            .title_font_size(-1.0)
            .title_font_family(FontFamily::SansMedium)
            .arrow(false)
            .background(fill),
    );
    assert_eq!(quick.animation_repaint_after_at(started), None);
    let shown = quick.layout_at(Size::new(240.0, 120.0), started);
    let (rect, color) = body(&shown).unwrap();
    let grown = FontMetrics::default().at_size(16.0).line_height - FontMetrics::default().at_size(12.0).line_height;
    assert!((rect.height - (TOOLTIP_HEIGHT + grown)).abs() < 0.01, "{rect:?}");
    assert_eq!(color, fill);
    assert!(!shown.commands.iter().any(|command| matches!(command, DrawCommand::FillTriangle { .. })));
    assert!(shown.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, font_size, font_family, .. }
            if text == "Server" && *font_size == 16.0 && *font_family == FontFamily::SansMedium
    )));
    // A delay of its own is what the tree waits for.
    let (mut slow, started) = hint(Tooltip::new(anchor(), "mark", "Server", palette).placement(TooltipPlacement::Right).delay_ms(800));
    assert_eq!(slow.animation_repaint_after_at(started), Some(Duration::from_millis(800)));
    assert!(body(&slow.layout_at(Size::new(240.0, 120.0), started + TOOLTIP_HOVER_DELAY)).is_none());
}

#[test]
fn a_shaped_hint_takes_its_radius_padding_shadow_rounded_pointer_and_entrance() {
    let palette = ComponentPalette::neutral_dark();
    let style = palette.tooltip_style();
    let shape = crate::TooltipShape {
        radius: 8.0,
        padding_x: 12.0,
        padding_y: 8.0,
        shadow_offset: Point::new(0.0, 12.0),
        shadow_blur: 24.0,
        caret: Size::new(16.0, 10.0),
    };
    let view = Size::new(320.0, 160.0);
    let tree = |reduced: bool| {
        let mut element = Tooltip::new(
            Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
            "mark",
            "Server",
            palette,
        )
        .placement(TooltipPlacement::Below)
        .delay_ms(0)
        .render();
        let ElementKind::Tooltip { options, .. } = &mut element.kind else {
            unreachable!()
        };
        options.shape = Some(shape);
        options.open_scale = 0.9;
        options.enter_ms = Some(100);
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![Element::spacer().flex(1.0), element, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(reduced);
        tree
    };
    let body = |frame: &Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, radius }
                if Color { a: style.background.a, ..*color } == style.background && rect.height > 20.0 =>
            {
                Some((*rect, *radius))
            }
            _ => None,
        })
    };
    let mut settled = tree(true);
    settled.layout(view);
    assert!(settled.focus("mark"));
    let frame = settled.layout(view);
    let anchor = frame.regions.iter().find(|region| region.id == "mark").unwrap().rect;
    let (rect, radius) = body(&frame).expect("the body");
    // One line and 8 above and below; rounded 8; the pointer's depth and 2 from the anchor.
    let line = FontMetrics::default().at_size(12.0).line_height;
    assert!((rect.height - (line + 16.0)).abs() < 0.01, "{rect:?}");
    assert_eq!(radius, 8.0);
    assert!((rect.y - (anchor.y + anchor.height + 12.0)).abs() < 0.01, "{rect:?} {anchor:?}");
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, position, .. } if text == "Server" && (position.x - (rect.x + 12.0)).abs() < 0.01
    )));
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Shadow { shadow, .. } if shadow.offset == Point::new(0.0, 12.0) && shadow.blur == 24.0
    )));
    // The pointer: two triangles and a round tip, all in the fill, the outline round it.
    let pointer: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillTriangle { points, color } => Some((points.to_vec(), *color)),
            _ => None,
        })
        .collect();
    assert_eq!(pointer.len(), 2);
    assert!(pointer.iter().all(|(_, color)| *color == style.background));
    let points: Vec<Point> = pointer.iter().flat_map(|(points, _)| points.clone()).collect();
    let (low, high) = points.iter().fold((f32::MAX, f32::MIN), |(low, high), point| {
        (low.min(point.x), high.max(point.x))
    });
    // 16 wide 1 px inside the body, its sides going on 1 px further in.
    assert!((high - low - 16.0 * 12.0 / 11.0).abs() < 0.01, "16 wide: {points:?}");
    let tip = frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect: tip, color, radius } if *color == style.background && *radius > 0.0 && tip.height < 8.0 => Some(*tip),
        _ => None,
    });
    let tip = tip.expect("a rounded tip");
    assert!(tip.y >= anchor.y + anchor.height + 1.0 && tip.y < rect.y, "{tip:?} {rect:?}");
    // Coming in: smaller and faded, about the pointer's tip, for its own duration.
    let mut moving = tree(false);
    let start = Instant::now();
    moving.layout_at(view, start);
    assert!(moving.focus("mark"));
    let early = moving.layout_at(view, start + Duration::from_millis(20));
    let (small, _) = body(&early).expect("the body, coming in");
    assert!(small.width < rect.width && small.height < rect.height, "{small:?} {rect:?}");
    // It began on the frame it was first painted in, 20 ms in.
    let done = moving.layout_at(view, start + Duration::from_millis(120));
    let (whole, _) = body(&done).expect("the body, in");
    assert!((whole.width - rect.width).abs() < 0.01 && (whole.height - rect.height).abs() < 0.01);
}

#[test]
fn a_shaped_hints_outline_goes_on_round_its_pointer_on_every_side() {
    let palette = ComponentPalette::neutral_dark();
    let shape = crate::TooltipShape {
        radius: 8.0,
        padding_x: 12.0,
        padding_y: 8.0,
        shadow_offset: Point::new(0.0, 12.0),
        shadow_blur: 24.0,
        caret: Size::new(16.0, 10.0),
    };
    let view = Size::new(400.0, 300.0);
    let hint = |placement: TooltipPlacement, border: Option<Color>| {
        let mut element = Tooltip::new(
            Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
            "mark",
            "Server",
            palette,
        )
        .placement(placement)
        .delay_ms(0)
        .render();
        let ElementKind::Tooltip { options, colors, .. } = &mut element.kind else {
            unreachable!()
        };
        options.shape = Some(shape);
        if let Some(border) = border {
            colors.border = border;
        }
        let style = *colors;
        let row = Element::row(vec![Element::spacer().flex(1.0), element, Element::spacer().flex(1.0)]);
        let spacer = || Element::spacer().flex(1.0);
        let mut tree = UiTree::new(Element::column(vec![spacer(), row, spacer()]));
        tree.set_reduced_motion(true);
        tree.layout(view);
        assert!(tree.focus("mark"));
        (tree.layout(view), style)
    };
    let ring = Color::rgba(149, 149, 162, 51);
    for placement in [
        TooltipPlacement::Above,
        TooltipPlacement::Below,
        TooltipPlacement::Left,
        TooltipPlacement::Right,
    ] {
        let (frame, style) = hint(placement, Some(ring));
        let index = |find: &dyn Fn(&DrawCommand) -> bool| frame.commands.iter().position(find).unwrap();
        let outline = index(&|command| matches!(command, DrawCommand::StrokeRect { color, .. } if *color == ring));
        let DrawCommand::StrokeRect { rect: body, .. } = frame.commands[outline] else { unreachable!() };
        let fill = index(&|command| matches!(command, DrawCommand::FillTriangle { .. }));
        let stroke = index(&|command| matches!(command, DrawCommand::Polyline { .. }));
        let DrawCommand::Polyline { points, color, width, dashed } = &frame.commands[stroke] else { unreachable!() };
        // The outline as it shows over the fill, so it adds nothing where it meets the body's.
        assert_eq!(*color, mix(style.background, Color { a: style.background.a, ..ring }, 51.0 / 255.0));
        assert_eq!((*width, *dashed), (1.0, false));
        // The body and its outline first, the pointer's fill over the outline at the join,
        // then the pointer's outline.
        assert!(outline < fill && fill < stroke, "{placement:?}");
        // Each point as (along the facing side, out from it).
        let local = |point: Point| match placement {
            TooltipPlacement::Above => (point.x, point.y - (body.y + body.height)),
            TooltipPlacement::Below => (point.x, body.y - point.y),
            TooltipPlacement::Left => (point.y, point.x - (body.x + body.width)),
            TooltipPlacement::Right => (point.y, body.x - point.x),
        };
        let tip = frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, radius, .. } if *radius > 0.0 && rect.height < 8.0 => Some(*rect),
            _ => None,
        });
        let tip = tip.expect("the rounded tip");
        let (axis, _) = local(Point::new(tip.x + tip.width / 2.0, tip.y + tip.height / 2.0));
        let far = [Point::new(tip.x, tip.y), Point::new(tip.x + tip.width, tip.y + tip.height)]
            .map(|corner| local(corner).1)
            .into_iter()
            .fold(f32::MIN, f32::max);
        let path: Vec<(f32, f32)> = points.iter().map(|point| local(*point)).collect();
        // The pointer's fill starts 2 px inside the body, over all of its outline's pixels
        // across the base: its sides, 16 apart 1 px in, go on 1 px further.
        let DrawCommand::FillTriangle { points: base, .. } = frame.commands[fill] else { unreachable!() };
        let base: Vec<_> = base.iter().map(|point| local(*point)).filter(|(_, out)| (out + 2.0).abs() < 0.01).collect();
        assert_eq!(base.len(), 2, "{placement:?}");
        assert!(((base[0].0 - base[1].0).abs() - 16.0 * 12.0 / 11.0).abs() < 0.01, "{placement:?}");
        // It starts and ends on the middle of the body's outline, beyond the pointer's
        // base on either side, so the two outlines overlap and join.
        let (first, last) = (path[0], path[path.len() - 1]);
        for (along, out) in [first, last] {
            assert!((out + 0.5).abs() < 0.01, "{placement:?} {path:?}");
            assert!((along - axis).abs() > 8.0, "{placement:?} {path:?}");
        }
        assert!((first.0 - axis).signum() != (last.0 - axis).signum(), "{placement:?}");
        // Never deeper in the body than that line; out to half a pixel inside the tip.
        assert!(path.iter().all(|(_, out)| *out > -0.51), "{placement:?} {path:?}");
        let deepest = path.iter().map(|(_, out)| *out).fold(f32::MIN, f32::max);
        assert!((deepest - (far - 0.5)).abs() < 0.01, "{placement:?} {deepest} {far}");
        // No stroke across the base: the line crosses the pointer's middle only at its tip.
        for pair in path.windows(2) {
            let ((a, a_out), (b, b_out)) = (pair[0], pair[1]);
            if (a - axis) * (b - axis) <= 0.0 {
                assert!(a_out > deepest - 1.0 && b_out > deepest - 1.0, "{placement:?} {path:?}");
            }
        }
        // The two sides mirror each other about the pointer's middle.
        for (left, right) in path.iter().zip(path.iter().rev()) {
            assert!((left.0 - axis + (right.0 - axis)).abs() < 0.01 && (left.1 - right.1).abs() < 0.01);
        }
        // Without an outline, the pointer has none either.
        let (bare, _) = hint(placement, Some(Color::TRANSPARENT));
        assert!(!bare.commands.iter().any(|command| matches!(command, DrawCommand::Polyline { .. })));
    }
}

#[test]
fn a_hint_on_a_spring_grows_past_full_size_and_back_its_fade_over_early() {
    let palette = ComponentPalette::neutral_dark();
    let style = palette.tooltip_style();
    let view = Size::new(320.0, 160.0);
    // The body's width and opacity on each frame, from the first it is painted in.
    let frames = |easing: Easing, reduced: bool| {
        let mut element = Tooltip::new(
            Element::icon_button("mark", Icon::PanelLeft, "").size(Some(32.0), Some(32.0)),
            "mark",
            "Server",
            palette,
        )
        .placement(TooltipPlacement::Below)
        .delay_ms(0)
        .render();
        let ElementKind::Tooltip { options, .. } = &mut element.kind else {
            unreachable!()
        };
        options.open_scale = 0.95;
        options.enter_ms = Some(200);
        options.enter_easing = easing;
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![Element::spacer().flex(1.0), element, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(reduced);
        let start = Instant::now();
        tree.layout_at(view, start);
        assert!(tree.focus("mark"));
        [0, 60, 75, 150, 200, 260].map(|ms| {
            let frame = tree.layout_at(view, start + Duration::from_millis(ms));
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. }
                        if Color { a: style.background.a, ..*color } == style.background
                            && rect.height > 20.0 =>
                    {
                        Some((rect.width, color.a))
                    }
                    _ => None,
                })
                .expect("the body")
        })
    };
    let full = style.background.a;
    let [_, _, _, _, _, (width, _)] = frames(Easing::OutCubic, true);
    // At once under reduced motion, whichever the curve.
    assert!(frames(Easing::Spring, true).iter().all(|&frame| frame == (width, full)));
    // The default curve grows to full size and stops there, its fade still going at 60 ms.
    assert_eq!(TooltipOptions::DEFAULT.enter_easing, Easing::OutCubic);
    let eased = frames(Easing::OutCubic, false);
    assert!(eased.iter().all(|&(w, _)| w <= width + 0.001), "{eased:?}");
    assert!(eased[1].1 < full && eased[5] == (width, full), "{eased:?}");
    // The spring: opaque by 60 ms, past full size at 75, a little short at 150, settled at 200.
    let sprung = frames(Easing::Spring, false);
    assert!(sprung[0].0 < width && sprung[0].1 < full, "{sprung:?}");
    assert_eq!((sprung[1].1, sprung[2].1), (full, full), "{sprung:?}");
    assert!(sprung[2].0 > width * 1.005, "{sprung:?} {width}");
    assert!(sprung[3].0 < width - 0.01, "{sprung:?} {width}");
    assert_eq!(&sprung[4..], &[(width, full); 2], "{sprung:?}");
}

#[test]
fn a_small_icon_is_drawn_and_measured_at_its_side_between_8_and_16() {
    for (asked, side) in [(12.0, 12.0), (4.0, crate::SMALL_ICON_SIZE), (30.0, ICON_SIZE), (f32::NAN, ICON_SIZE)] {
        let icon = Element::icon(Icon::Volume).small_icon(asked);
        // Natural size in a row, centered in a larger box in a column.
        let frame = UiTree::new(Element::column(vec![
            Element::row(vec![icon.clone(), Element::spacer().flex(1.0)]),
            Element::panel(icon).size(Some(20.0), Some(20.0)),
            Element::spacer().flex(1.0),
        ]))
        .layout(Size::new(100.0, 100.0));
        let rects: Vec<Rect> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { rect, icon: Icon::Volume, .. } => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(rects.len(), 2, "{asked}");
        assert_eq!(rects[0], Rect::new(0.0, 0.0, side, side), "{asked}");
        assert_eq!(rects[1], Rect::new((20.0 - side) / 2.0, side + (20.0 - side) / 2.0, side, side), "{asked}");
    }
}

#[test]
fn a_hint_for_a_control_inside_its_anchor_points_at_that_control() {
    let palette = ComponentPalette::neutral_dark();
    // A link at the start of a wide line, hinted on the whole line.
    let line = Element::row(vec![
        Element::clickable("link", Element::text("link")).size(Some(40.0), Some(20.0)),
        Element::spacer().flex(1.0),
    ])
    .size(None, Some(20.0));
    let size = Size::new(400.0, 200.0);
    let mut tree = UiTree::new(Element::column(vec![
        Element::spacer().size(None, Some(100.0)),
        Tooltip::new(line, "link", "Where it goes", palette).render(),
    ]));
    tree.layout(size);
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 110.0)));
    let started = tree.tooltip_hover_since.as_ref().unwrap().1;
    let frame = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY);
    let tip = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillTriangle { points, .. } => Some(*points),
            _ => None,
        })
        .expect("the hint is open");
    // Its arrow stands over the link, not over the middle of the line.
    let center = tip.iter().map(|point| point.x).sum::<f32>() / 3.0;
    assert!(center > 0.0 && center < 40.0, "{center}");
}

#[test]
fn pointer_hover_and_keyboard_focus_do_not_show_two_tooltips() {
    let palette = ComponentPalette::neutral_dark();
    let tooltip = |id, title| {
        Tooltip::new(
            Element::icon_button(id, Icon::Add, "").size(Some(32.0), Some(32.0)),
            id,
            title,
            palette,
        )
        .render()
    };
    let mut tree = UiTree::new(Element::row(vec![
        tooltip("first", "First tooltip"),
        tooltip("second", "Second tooltip"),
    ]));
    tree.layout(Size::new(160.0, 80.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(48.0, 16.0)));
    tree.tooltip_hover_since = Some(("second".into(), Instant::now() - TOOLTIP_HOVER_DELAY));
    assert!(tree.focus("first"));
    let frame = tree.layout(Size::new(160.0, 80.0));
    let titles: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. }
                if matches!(text.as_str(), "First tooltip" | "Second tooltip") =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(titles, vec!["First tooltip"]);

    tree.handle_event(UiEvent::PointerMove(Point::new(49.0, 16.0)));
    let frame = tree.layout(Size::new(160.0, 80.0));
    let titles: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. }
                if matches!(text.as_str(), "First tooltip" | "Second tooltip") =>
            {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(titles, vec!["Second tooltip"]);
}

#[test]
fn tooltip_flips_from_top_and_clamps_to_viewport() {
    let viewport = Rect::new(0.0, 0.0, 180.0, 100.0);
    let anchor = Rect::new(154.0, 4.0, 24.0, 24.0);
    let (body, actual) = place_tooltip(
        anchor,
        Size::new(140.0, 32.0),
        viewport,
        TooltipPlacement::Above,
        TOOLTIP_GAP,
    );
    assert_eq!(actual, TooltipPlacement::Below);
    assert!(body.x >= 8.0);
    assert!(body.x + body.width <= viewport.width - 8.0);
    assert!(body.y >= anchor.y + anchor.height);
    assert!(body.y + body.height <= viewport.height - 8.0);
}

#[test]
fn tooltips_prefer_the_requested_side_and_avoid_the_anchor_at_viewport_edges() {
    for (viewport, anchor, desired, preferred, expected) in [
        (
            Rect::new(0.0, 0.0, 400.0, 300.0),
            Rect::new(6.0, 100.0, 24.0, 24.0),
            Size::new(120.0, 36.0),
            TooltipPlacement::Left,
            TooltipPlacement::Right,
        ),
        (
            Rect::new(0.0, 0.0, 400.0, 300.0),
            Rect::new(370.0, 100.0, 24.0, 24.0),
            Size::new(120.0, 36.0),
            TooltipPlacement::Right,
            TooltipPlacement::Left,
        ),
        (
            Rect::new(0.0, 0.0, 500.0, 90.0),
            Rect::new(200.0, 34.0, 24.0, 24.0),
            Size::new(150.0, 50.0),
            TooltipPlacement::Above,
            TooltipPlacement::Right,
        ),
        (
            Rect::new(0.0, 0.0, 300.0, 400.0),
            Rect::new(100.0, 150.0, 100.0, 24.0),
            Size::new(140.0, 44.0),
            TooltipPlacement::Left,
            TooltipPlacement::Below,
        ),
        (
            Rect::new(0.0, 0.0, 400.0, 300.0),
            Rect::new(200.0, 260.0, 24.0, 24.0),
            Size::new(140.0, 36.0),
            TooltipPlacement::Below,
            TooltipPlacement::Above,
        ),
    ] {
        let (body, actual) = place_tooltip(anchor, desired, viewport, preferred, TOOLTIP_GAP);
        assert_eq!(actual, expected);
        assert!(
            body.intersection(anchor).is_none(),
            "hint covers trigger: {body:?} / {anchor:?}"
        );
        assert!(body.x >= viewport.x + 8.0 && body.y >= viewport.y + 8.0);
        assert!(body.x + body.width <= viewport.x + viewport.width - 8.0);
        assert!(body.y + body.height <= viewport.y + viewport.height - 8.0);
    }
}

#[test]
fn lateral_menu_tooltip_falls_back_on_narrow_viewports_without_losing_hover_or_actions() {
    let palette = ComponentPalette::neutral_dark();
    for width in [360.0, 900.0] {
        let hint = Tooltip::new(
            Element::button("exact-model", "Exact model").size(Some(328.0), Some(42.0)),
            "exact-model",
            "Exact Messages model",
            palette,
        )
        .placement(TooltipPlacement::Left)
        .description("Entrada máxima: 100000\nSalida máxima: 8192\nContexto: no informado")
        .footer("Proveedor propio · ID exacto")
        .render();
        let popup = Element::column(vec![hint])
            .padding(8.0)
            .size(Some(344.0), Some(74.0));
        let root = Element::column(vec![
            Element::spacer().size(None, Some(140.0)),
            Element::popover(
                Element::button("models", "Modelos").size(Some(100.0), Some(30.0)),
                popup,
                "dismiss-models",
                PopoverPlacement::BottomStart,
            ),
        ]);
        let mut tree = UiTree::new(root);
        tree.set_reduced_motion(true);
        let start = Instant::now();
        let viewport = Size::new(width, 640.0);
        let initial = tree.layout_at(viewport, start);
        let target = initial
            .regions
            .iter()
            .find(|region| region.id == "exact-model")
            .unwrap()
            .rect;
        let point = Point::new(
            target.x + target.width / 2.0,
            target.y + target.height / 2.0,
        );
        tree.handle_event_at(UiEvent::PointerMove(point), start);
        for elapsed in [
            TOOLTIP_HOVER_DELAY,
            TOOLTIP_HOVER_DELAY + Duration::from_millis(100),
        ] {
            let frame = tree.layout_at(viewport, start + elapsed);
            assert_eq!(frame.hit_test(point), Some("exact-model"));
            assert!(
                frame.commands.iter().any(|command| matches!(command,
                    DrawCommand::Text { text, .. } if text == "Entrada máxima: 100000"
                )),
                "missing exact metadata at {width}px"
            );
            let body = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Shadow { rect, .. } => Some(*rect),
                    _ => None,
                })
                .expect("tooltip is painted");
            assert!(body.x >= 8.0 && body.y >= 8.0);
            assert!(body.x + body.width <= width - 8.0);
            assert!(body.y + body.height <= viewport.height - 8.0);
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .filter(|region| region.id == "exact-model")
                    .count(),
                1
            );
        }
        assert!(tree.focus("exact-model"));
        let focused = tree.layout_at(viewport, start + Duration::from_secs(1));
        assert!(focused.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, .. } if text == "Proveedor propio · ID exacto"
        )));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Enter,
                shift: false,
                ctrl: false,
                alt: false
            }),
            vec![UiAction::Clicked {
                id: "exact-model".into()
            }]
        );
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: "exact-model".into()
            }]
        );
    }
}

#[test]
fn popover_container_exception_still_blocks_nested_and_context_menus() {
    let panel = |nested| {
        let field = Element::text_input("field", "editable", "").size(Some(180.0), Some(32.0));
        let contents = if nested {
            Element::column(vec![
                field,
                Element::popover(
                    Element::button("nested_trigger", "Menu"),
                    Element::button("nested_action", "Action").size(Some(120.0), Some(32.0)),
                    "nested_dismiss",
                    PopoverPlacement::BottomStart,
                ),
            ])
        } else {
            field
        };
        Element::popover(
            Element::button("panel_trigger", "Panel").size(Some(80.0), Some(32.0)),
            contents.size(Some(220.0), Some(160.0)),
            "panel_dismiss",
            PopoverPlacement::BottomStart,
        )
        .popover_initial_focus("field")
    };
    let size = Size::new(600.0, 440.0);
    let mut tree = UiTree::new(panel(false));
    tree.set_reduced_motion(true);
    tree.layout(size);
    assert!(tree.has_open_popover());
    assert!(!tree.has_open_popover_except("panel_dismiss"));
    assert!(tree.has_open_popover_except("other_container"));
    assert!(tree.focus("field"));
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    assert!(tree.context_menu_open());
    assert!(tree.has_open_popover_except("panel_dismiss"));
    tree.dismiss_context_menu();
    tree.set_root(panel(true));
    tree.layout(size);
    assert!(tree.has_open_popover_except("panel_dismiss"));
}

#[test]
fn root_popover_positions_below_the_explicit_trigger_not_its_viewport_allocation() {
    let mut tree = UiTree::new(Element::popover(
        Element::button("trigger", "Abrir").size(Some(100.0), Some(30.0)),
        Element::button("option", "Opción").size(Some(160.0), Some(28.0)),
        "dismiss",
        PopoverPlacement::BottomStart,
    ));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(400.0, 300.0));
    let trigger = frame
        .regions
        .iter()
        .find(|region| region.id == "trigger")
        .unwrap()
        .rect;
    let surface = frame
        .regions
        .iter()
        .find(|region| region.id == "popover_surface:dismiss")
        .unwrap()
        .rect;
    assert_eq!(trigger.width, 100.0);
    assert_eq!(trigger.height, 30.0);
    assert!(surface.y >= trigger.y + trigger.height);
    assert_eq!(surface.height, 28.0);
    assert!(frame.regions.iter().any(|region| region.id == "option"));
}

#[test]
fn end_aligned_popover_uses_the_whole_composite_anchor() {
    for controls in [1, 2] {
        let mut header = vec![
            Element::button("breadcrumb", "src").size(Some(40.0), Some(30.0)),
            Element::spacer().flex(1.0),
        ];
        if controls == 2 {
            header.push(Element::button("more", "…").size(Some(30.0), Some(30.0)));
        }
        let mut tree = UiTree::new(Element::row(vec![
            Element::spacer().size(Some(311.0), None),
            Element::popover(
                Element::row(header).size(Some(240.0), Some(30.0)),
                Element::text_input("search", "", "Buscar").size(Some(232.0), Some(76.0)),
                "search_close",
                PopoverPlacement::BottomEnd,
            )
            .popover_non_modal(),
        ]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(560.0, 300.0));
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:search_close")
            .unwrap()
            .rect;
        assert_eq!(surface.x, 319.0, "{controls} controls: {surface:?}");
        assert_eq!(surface.x + surface.width, 551.0);
        assert!(surface.y >= 30.0);
    }
}

#[test]
fn viewport_wide_popover_anchor_keeps_a_usable_bounded_surface() {
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let rect = place_popover(
        viewport,
        Size::new(200.0, 180.0),
        viewport,
        PopoverPlacement::BottomStart,
        4.0,
    );
    assert_eq!(rect.height, 180.0);
    assert!(rect.y >= 8.0 && rect.y + rect.height <= 292.0);
}

#[test]
fn bottom_popover_ignores_anchor_and_gap_and_meets_the_safe_viewport_edge() {
    let viewport = Rect::new(12.0, 24.0, 390.0, 700.0);
    for anchor in [viewport, Rect::new(80.0, 400.0, 40.0, 30.0)] {
        for gap in [0.0, 90.0] {
            assert_eq!(
                place_popover(anchor, Size::new(560.0, 300.0), viewport, PopoverPlacement::Bottom, gap),
                Rect::new(12.0, 424.0, 390.0, 300.0),
            );
            assert_eq!(
                place_popover(anchor, Size::new(240.0, 900.0), viewport, PopoverPlacement::Bottom, gap),
                Rect::new(87.0, 32.0, 240.0, 692.0),
            );
        }
    }
}

#[test]
fn bottom_popover_measures_wrapped_content_at_full_phone_width() {
    let mut tree = UiTree::new(Element::popover(
        Element::spacer(),
        Element::button("sheet_body", "Actions").size(Some(560.0), Some(120.0)),
        "sheet_close",
        PopoverPlacement::Bottom,
    ));
    tree.set_reduced_motion(true);
    for width in [320.0, 390.0, 430.0] {
        let frame = tree.layout(Size::new(width, 700.0));
        let surface = frame.regions.iter().find(|r| r.id == "popover_surface:sheet_close").unwrap().rect;
        let body = frame.regions.iter().find(|r| r.id == "sheet_body").unwrap().rect;
        assert_eq!(surface, Rect::new(0.0, 580.0, width, 120.0));
        assert_eq!(body, surface);
    }
}

#[test]
fn anchored_popover_flips_and_keeps_its_own_surface_interactive() {
    let trigger = Element::button("trigger", "Abrir").size(Some(100.0), Some(26.0));
    let popup =
        Element::panel(Element::button("option", "Opción").size(Some(100.0), Some(28.0)))
            .padding(8.0)
            .size(Some(130.0), Some(48.0))
            .background(Color::rgb(40, 40, 40));
    let root = Element::column(vec![
        Element::spacer().size(None, Some(145.0)),
        Element::popover(trigger, popup, "dismiss", PopoverPlacement::BottomStart),
    ]);
    let mut tree = UiTree::new(root);
    let frame = tree.layout(Size::new(240.0, 190.0));
    let anchor = frame
        .regions
        .iter()
        .find(|region| region.id == "trigger")
        .unwrap();
    let option = frame
        .regions
        .iter()
        .find(|region| region.id == "option")
        .unwrap();
    assert!(
        option.rect.y < anchor.rect.y,
        "must flip above viewport edge"
    );
    assert_eq!(frame.hit_test(Point::new(228.0, 176.0)), Some("dismiss"));
    assert_eq!(
        frame.hit_test(Point::new(anchor.rect.x + 10.0, anchor.rect.y + 10.0)),
        Some("dismiss"),
        "an open popover's backdrop closes before the anchor can retrigger"
    );
    let popup_surface = frame
        .regions
        .iter()
        .find(|region| region.id == "popover_surface:dismiss")
        .unwrap();
    assert_eq!(
        frame.hit_test(Point::new(
            popup_surface.rect.x + 3.0,
            popup_surface.rect.y + 3.0
        )),
        Some("popover_surface:dismiss")
    );
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Escape,
            shift: false,
            ctrl: false,
            alt: false,
        }),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
}

#[test]
fn clickable_session_row_keeps_nested_menu_action_and_escape_focus() {
    let row = |open| {
        let menu = Element::icon_button("more", Icon::More, "").size(Some(24.0), Some(28.0));
        let menu = if open {
            Element::popover(
                menu,
                Element::column(vec![
                    Element::button("pin", "Fijar").size(Some(90.0), Some(28.0)),
                    Element::button("archive", "Archivar").size(Some(90.0), Some(28.0)),
                ])
                .size(Some(90.0), Some(56.0)),
                "dismiss",
                PopoverPlacement::BottomEnd,
            )
        } else {
            menu
        };
        Element::clickable(
            "session",
            Element::row(vec![
                Element::truncated_text("Tarea").size(Some(130.0), Some(28.0)),
                menu,
            ])
            .size(Some(160.0), Some(28.0)),
        )
        .size(Some(160.0), Some(28.0))
    };
    let mut tree = UiTree::new(row(false));
    let frame = tree.layout(Size::new(240.0, 180.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 14.0)), Some("session"));
    assert_eq!(frame.hit_test(Point::new(142.0, 14.0)), Some("more"));
    for (point, expected) in [
        (Point::new(5.0, 14.0), "session"),
        (Point::new(142.0, 14.0), "more"),
    ] {
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked {
                id: expected.into()
            }]
        );
    }
    assert!(tree.focus("more"));
    tree.set_root(row(true));
    tree.layout(Size::new(240.0, 180.0));
    let key = |key| UiEvent::Key {
        key,
        shift: false,
        ctrl: false,
        alt: false,
    };
    tree.handle_event(key(Key::ArrowDown));
    assert_eq!(tree.focused_id(), Some("pin"));
    tree.set_root(row(false));
    tree.layout(Size::new(240.0, 180.0));
    assert_eq!(tree.focused_id(), Some("more"));
    tree.set_root(row(true));
    tree.layout(Size::new(240.0, 180.0));
    tree.handle_event(key(Key::End));
    assert_eq!(tree.focused_id(), Some("archive"));
    assert_eq!(
        tree.handle_event(key(Key::Escape)),
        vec![UiAction::Clicked {
            id: "dismiss".into()
        }]
    );
    assert_eq!(tree.focused_id(), Some("more"));
    tree.set_root(row(false));
    tree.layout(Size::new(240.0, 180.0));
    assert_eq!(tree.focused_id(), Some("more"));
}

#[test]
fn anchored_popover_escapes_scroll_clip() {
    let trigger = Element::button("trigger", "Abrir").size(Some(80.0), Some(26.0));
    let popup = Element::button("option", "Elegir").size(Some(95.0), Some(30.0));
    let scrolled = Element::scroll(
        "list",
        Element::column(vec![
            Element::spacer().size(None, Some(77.0)),
            Element::popover(trigger, popup, "dismiss", PopoverPlacement::BottomStart),
        ]),
    )
    .size(Some(160.0), Some(105.0));
    let mut tree = UiTree::new(Element::column(vec![scrolled]));
    let frame = tree.layout(Size::new(220.0, 200.0));
    let option = frame
        .regions
        .iter()
        .find(|region| region.id == "option")
        .unwrap();
    assert!(option.rect.y > 105.0, "floating option escapes scroll clip");
    assert_eq!(
        frame.hit_test(Point::new(option.rect.x + 10.0, option.rect.y + 10.0)),
        Some("option")
    );
}

#[test]
fn button_labels_truncate_at_painted_width_without_splitting_graphemes() {
    let mut tree = UiTree::new(Element::column(vec![
        Element::button("plain", "WWii").size(Some(30.0), Some(28.0)),
        Element::icon_button("with_icon", Icon::Folder, "👩‍💻WWii").size(Some(50.0), Some(28.0)),
    ]));
    tree.set_text_measurer(|text, _, _| {
        text.graphemes(true)
            .map(|part| match part {
                "W" => 10.0,
                "i" => 2.0,
                "…" => 4.0,
                "👩‍💻" => 14.0,
                _ => 5.0,
            })
            .sum()
    });
    let frame = tree.layout(Size::new(70.0, 70.0));
    let labels: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(labels, vec!["W…", "👩‍💻…"]);
}

#[test]
fn formatted_spans_use_painter_width_across_font_families() {
    let spans = vec![
        TextSpan::new("ii ", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("W", Color::rgb(250, 100, 100), 14.0, FontFamily::Monospace),
        TextSpan::new("i", Color::WHITE, 14.0, FontFamily::Sans),
    ];
    let mut tree = UiTree::new(Element::formatted_text(spans.clone()).size(Some(9.0), None));
    tree.set_text_measurer(|part, _, family| {
        part.chars()
            .map(|character| match (character, family) {
                ('W', FontFamily::Monospace) => 5.0,
                ('i', _) | (' ', _) => 1.0,
                _ => 2.0,
            })
            .sum()
    });
    let frame = tree.layout(Size::new(9.0, 100.0));
    let runs: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text {
                position,
                text,
                font_family,
                color,
                ..
            } => Some((position.x, position.y, text.as_str(), *font_family, *color)),
            _ => None,
        })
        .collect();
    assert_eq!(runs.iter().map(|run| run.2).collect::<String>(), "ii Wi");
    assert!(runs.iter().all(|run| run.1 == 0.0));
    assert!(runs.iter().any(|run| run.0 == 3.0
        && run.2 == "W"
        && run.3 == FontFamily::Monospace
        && run.4 == Color::rgb(250, 100, 100)));
    assert!(runs.iter().any(|run| run.0 == 8.0 && run.2 == "i"));

    tree.set_root(Element::formatted_text(spans).size(Some(6.0), None));
    let frame = tree.layout(Size::new(6.0, 100.0));
    let w_y = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { position, text, .. } if text == "W" => Some(position.y),
            _ => None,
        })
        .unwrap();
    assert!(
        w_y > 0.0,
        "narrow layout should wrap the measured wide glyph"
    );
}

fn inline_picture() -> RasterImage {
    RasterImage::new(
        "b".repeat(64),
        crate::RasterImageFormat::Png,
        4,
        Arc::from("AAAA"),
        "wave",
    )
    .unwrap()
}

#[test]
fn inline_pictures_are_unsplit_squares_painted_in_place_of_their_text() {
    let spans = vec![
        TextSpan::new("hi ", Color::WHITE, 16.0, FontFamily::Sans),
        TextSpan::new(":wave:", Color::WHITE, 16.0, FontFamily::Sans).image(inline_picture()),
        TextSpan::new(" ok", Color::WHITE, 16.0, FontFamily::Sans),
    ];
    let mut tree = UiTree::new(Element::formatted_text(spans.clone()));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
    let frame = tree.layout(Size::new(200.0, 100.0));
    let pictures: Vec<Rect> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::RasterImage { rect, image, .. } if image.sha256() == "b".repeat(64) => {
                Some(*rect)
            }
            _ => None,
        })
        .collect();
    assert_eq!(pictures.len(), 1);
    // 1.375 times the 16 px text, after the 12 px of "hi ".
    assert_eq!((pictures[0].x, pictures[0].width, pictures[0].height), (12.0, 22.0, 22.0));
    let painted: String = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(painted, "hi  ok", "the alternative text is not painted");
    let block = &frame.text_layouts()[0];
    assert_eq!(block.source, "hi :wave: ok");
    let after = block.runs.iter().find(|run| run.text == " ok").unwrap();
    assert_eq!(after.rect.x, 34.0, "the text goes on after the square");

    // Too narrow for the square beside "hi ": it moves down whole.
    tree.set_root(Element::formatted_text(spans).size(Some(30.0), None));
    let frame = tree.layout(Size::new(30.0, 100.0));
    let picture = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::RasterImage { rect, .. } => Some(*rect),
            _ => None,
        })
        .unwrap();
    assert_eq!((picture.x, picture.width), (0.0, 22.0));
    assert!(picture.y > 0.0);
}

#[test]
fn an_inline_picture_is_selected_and_copied_whole_as_its_text() {
    let mut tree = UiTree::new(Element::formatted_text(vec![
        TextSpan::new("ab", Color::WHITE, 16.0, FontFamily::Sans),
        TextSpan::new(":wave:", Color::WHITE, 16.0, FontFamily::Sans).image(inline_picture()),
        TextSpan::new("cd", Color::WHITE, 16.0, FontFamily::Sans),
    ]));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
    tree.layout(Size::new(200.0, 100.0));
    // From the start of the text to a third into the 22 px square, at 8.
    tree.handle_event(UiEvent::PointerDown(Point::new(0.0, 4.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(15.0, 4.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("ab"));
    tree.handle_event(UiEvent::PointerMove(Point::new(21.0, 4.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("ab:wave:"));
    tree.handle_event(UiEvent::PointerUp(Point::new(21.0, 4.0)));
    let frame = tree.layout(Size::new(200.0, 100.0));
    let highlight = frame.commands.iter().find_map(|command| match command {
        DrawCommand::FillRect { rect, color, .. }
            if *color == Color::rgba(180, 180, 180, 84) && rect.x == 8.0 =>
        {
            Some(*rect)
        }
        _ => None,
    });
    assert_eq!(highlight.map(|rect| rect.width), Some(22.0), "the whole square is selected");
}

#[test]
fn an_inline_surface_paints_the_hosts_pixels_fitted_in_the_square() {
    let mut tree = UiTree::new(Element::formatted_text(vec![TextSpan::new(
        ":dance:",
        Color::WHITE,
        14.0,
        FontFamily::Sans,
    )
    .surface("animation:dance")
    .action("emoji:dance")]));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 4.0);
    let frame = tree.layout(Size::new(200.0, 100.0));
    let surface = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Surface {
            rect, id, label, fit, ..
        } => Some((*rect, id.clone(), label.clone(), *fit)),
        _ => None,
    });
    let (rect, id, label, fit) = surface.unwrap();
    assert_eq!((rect.width, rect.height), (19.25, 19.25));
    assert_eq!(
        (id.as_str(), label.as_str(), fit),
        ("animation:dance", ":dance:", crate::SurfaceFit::Contain)
    );
    assert_eq!(
        frame.hit_test(Point::new(rect.x + 5.0, rect.y + 5.0)),
        Some("emoji:dance")
    );
    assert_eq!(frame.text_layouts()[0].source, ":dance:");
}

#[test]
fn inline_actions_follow_measured_wrapped_fragments_and_preserve_plain_selection() {
    let link = "WW ii WW";
    let mut tree = UiTree::new(Element::formatted_text(vec![
        TextSpan::new("ii ", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new(link, Color::WHITE, 14.0, FontFamily::Sans).action("link"),
        TextSpan::new(" fin", Color::WHITE, 14.0, FontFamily::Sans),
    ]));
    tree.set_text_measurer(|text, _, _| {
        text.chars()
            .map(|ch| if ch == 'W' { 9.0 } else { 2.0 })
            .sum()
    });
    let viewport = Size::new(30.0, 200.0);
    let frame = tree.layout(viewport);
    let regions: Vec<_> = frame
        .regions
        .iter()
        .filter(|r| r.id == "link")
        .cloned()
        .collect();
    assert_eq!(regions.len(), 2, "one target per painted link fragment");
    let runs = &frame.text_layouts()[0].runs;
    let linked_runs: Vec<_> = runs
        .iter()
        .filter(|run| regions.iter().any(|r| r.rect == run.rect))
        .collect();
    assert_eq!(
        linked_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        link
    );
    assert_eq!(
        linked_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<Vec<_>>(),
        ["WW ii", " WW"],
        "word and whitespace tokens fill the exact measured 30 px row"
    );
    assert_eq!(frame.hit_test(Point::new(1.0, 2.0)), None);
    for region in &regions {
        let point = Point::new(region.rect.x + 1.0, region.rect.y + 2.0);
        assert_eq!(frame.hit_test(point), Some("link"));
    }
    let point = Point::new(regions[1].rect.x + 1.0, regions[1].rect.y + 2.0);
    tree.handle_event(UiEvent::PointerDown(point));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked { id: "link".into() }]
    );
    assert_eq!(tree.selected_text(), None);

    // Selection starting in ordinary prose still crosses actionable spans.
    tree.handle_event(UiEvent::PointerDown(Point::new(0.0, 2.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(
        regions[0].rect.x + regions[0].rect.width,
        2.0,
    )));
    assert_eq!(tree.selected_text().as_deref(), Some("ii WW ii"));
}

#[test]
fn inline_actions_keep_distinct_targets_and_invalidate_cached_destinations() {
    let root = |target: &str| {
        Element::formatted_text(vec![
            TextSpan::new("one", Color::WHITE, 14.0, FontFamily::Sans).action(target),
            TextSpan::new("two", Color::WHITE, 14.0, FontFamily::Sans).action("second"),
        ])
    };
    let mut tree = UiTree::new(root("old"));
    tree.set_text_measurer(|text, _, _| text.len() as f32 * 8.0);
    tree.layout(Size::new(100.0, 40.0));
    tree.set_root(root("new"));
    let frame = tree.layout(Size::new(100.0, 40.0));
    assert_eq!(frame.hit_test(Point::new(4.0, 4.0)), Some("new"));
    assert_eq!(frame.hit_test(Point::new(28.0, 4.0)), Some("second"));
    assert!(!frame.regions.iter().any(|region| region.id == "old"));
    assert_eq!(frame.text_layouts()[0].source, "onetwo");
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("new"));
    for key in [Key::Enter, Key::Space] {
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key,
                shift: false,
                ctrl: false,
                alt: false
            }),
            vec![UiAction::Clicked { id: "new".into() }]
        );
    }
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("second"));
}

#[test]
fn inline_link_keyboard_focus_reveals_offscreen_paragraph_and_clips_hit_regions() {
    let paragraph = Element::formatted_text(vec![TextSpan::new(
        "wrapped link text",
        Color::WHITE,
        14.0,
        FontFamily::Sans,
    )
    .action("link")]);
    let mut tree = UiTree::new(
        Element::scroll(
            "document",
            Element::column(vec![
                Element::button("before", "Before").size(None, Some(24.0)),
                Element::spacer().size(None, Some(180.0)),
                paragraph,
                Element::button("after", "After").size(None, Some(24.0)),
            ]),
        )
        .size(None, Some(42.0)),
    );
    tree.set_reduced_motion(true);
    tree.set_text_measurer(|text, _, _| text.len() as f32 * 8.0);
    let viewport = Size::new(70.0, 42.0);
    assert!(!tree.layout(viewport).regions.iter().any(|r| r.id == "link"));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("before"));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("link"));
    let frame = tree.layout(viewport);
    let regions: Vec<_> = frame.regions.iter().filter(|r| r.id == "link").collect();
    assert!(
        !regions.is_empty(),
        "Tab reveals the measured first link fragment"
    );
    assert!(regions
        .iter()
        .all(|r| r.rect.y >= 0.0 && r.rect.y + r.rect.height <= 42.0));
    assert!(frame
        .commands
        .iter()
        .any(|cmd| matches!(cmd, DrawCommand::StrokeRect { width, .. } if *width == 2.0)));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(
        tree.focused_id(),
        Some("after"),
        "a wrapped link has only one Tab stop"
    );
}

#[test]
fn formatted_leading_keeps_cache_selection_and_inline_code_geometry_consistent() {
    let fill = Color::rgb(45, 46, 47);
    let paragraph = |height| {
        Element::formatted_text(vec![TextSpan::new(
            "one\ntwo",
            Color::WHITE,
            14.0,
            FontFamily::Sans,
        )
        .background(fill)])
        .font_size(14.0)
        .font_family(FontFamily::Sans)
        .line_height(height)
        .size(Some(160.0), None)
    };
    let mut tree = UiTree::new(paragraph(24.0));
    for height in [24.0, 32.0, 24.0] {
        tree.set_root(paragraph(height));
        let frame = tree.layout(Size::new(160.0, 200.0));
        let block = &frame.text_layouts()[0];
        assert_eq!(block.source, "one\ntwo");
        assert_eq!(block.runs.len(), 2);
        assert_eq!(block.runs[0].rect.height, height);
        assert_eq!(block.runs[1].rect.y - block.runs[0].rect.y, height);
        let painted: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { position, .. } => Some(position.y),
                _ => None,
            })
            .collect();
        assert_eq!(painted[1] - painted[0], height);
        let backgrounds: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == fill => Some(rect),
                _ => None,
            })
            .collect();
        assert_eq!(backgrounds.len(), 2);
        assert!(
            backgrounds[0].height < height,
            "code chips must not fill the leading"
        );
        assert!(backgrounds[0].y > block.runs[0].rect.y);
    }
    assert_eq!(paragraph(f32::NAN).style.line_height, None);
    assert_eq!(paragraph(-1.0).style.line_height, None);
}

#[test]
fn formatted_wrap_keeps_graphemes_and_explicit_line_breaks() {
    let source = "a👩‍💻e\u{301}b\n🦀";
    let mut tree = UiTree::new(
        Element::formatted_text(vec![TextSpan::new(
            source,
            Color::WHITE,
            14.0,
            FontFamily::Sans,
        )])
        .size(Some(5.0), None),
    );
    tree.set_text_measurer(|part, _, _| {
        part.graphemes(true)
            .map(|part| if part == "👩‍💻" { 5.0 } else { 1.0 })
            .sum()
    });
    let frame = tree.layout(Size::new(5.0, 100.0));
    let runs: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { position, text, .. } => Some((position.y, text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        runs.iter().map(|(_, text)| *text).collect::<String>(),
        source.replace('\n', "")
    );
    assert!(runs.iter().any(|(_, text)| *text == "👩‍💻"));
    assert!(runs.iter().any(|(_, text)| text.contains("e\u{301}")));
    assert!(runs.last().unwrap().0 > runs.first().unwrap().0);
}

#[test]
fn static_selection_crosses_styled_spans_without_splitting_emoji() {
    let spans = vec![
        TextSpan::new("hi ", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("👩‍💻", Color::rgb(180, 180, 180), 14.0, FontFamily::Sans),
        TextSpan::new(" fin", Color::WHITE, 14.0, FontFamily::Sans),
    ];
    let mut tree = UiTree::new(Element::formatted_text(spans).size(Some(200.0), None));
    tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
    tree.layout(Size::new(200.0, 40.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(43.0, 5.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(43.0, 5.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("hi 👩‍💻"));
    let frame = tree.layout(Size::new(200.0, 40.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. }
            if *color == Color::rgba(180, 180, 180, 84)
                && (rect.x - 30.0).abs() < 0.01
                && (rect.width - 10.0).abs() < 0.01
    )));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::C,
            shift: false,
            ctrl: true,
            alt: false,
        }),
        vec![UiAction::CopyRequested {
            text: "hi 👩‍💻".into(),
        }]
    );
}

#[test]
fn pointer_leave_finishes_static_selection_drag() {
    let mut tree = UiTree::new(Element::text("abcd").size(Some(100.0), None));
    tree.layout(Size::new(100.0, 30.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(18.0, 5.0)));
    let before_leave = tree.selected_text();
    assert!(before_leave.is_some());
    tree.handle_event(UiEvent::PointerLeave);
    assert!(!tree.static_selection.as_ref().unwrap().dragging);
    tree.handle_event(UiEvent::PointerMove(Point::new(80.0, 5.0)));
    assert_eq!(tree.selected_text(), before_leave);
}

#[test]
fn pointer_leave_releases_editor_minimap_selection_and_diff_scrollbar_capture() {
    let controller = crate::EditorController::new(
        limn_editor::TextBuffer::new(&"source code\n".repeat(400)).unwrap(),
    );
    for point in [
        Point::new(650.0, 140.0),
        Point::new(698.0, 140.0),
        Point::new(120.0, 24.0),
    ] {
        let mut tree = UiTree::new(
            crate::CodeEditor::new("editor", controller.clone())
                .minimap(true)
                .height(300.0)
                .render(),
        );
        tree.layout(Size::new(700.0, 300.0));
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerMove(Point::new(point.x, 240.0)));
        let before = tree.code_editors["editor"].viewport.scroll_y;
        let selection = controller.snapshot().unwrap().selection();
        tree.handle_event(UiEvent::PointerLeave);
        assert!(!tree.code_editors["editor"].viewport.dragging);
        assert!(tree.code_editors["editor"]
            .viewport
            .scrollbar_drag
            .is_none());
        tree.handle_event(UiEvent::PointerMove(Point::new(point.x, 5.0)));
        assert_eq!(tree.code_editors["editor"].viewport.scroll_y, before);
        assert_eq!(controller.snapshot().unwrap().selection(), selection);
    }
    let lines = (0..400)
        .map(|row| crate::CodeDiffLine {
            kind: crate::CodeDiffLineKind::Context,
            old_line: Some(row + 1),
            new_line: Some(row + 1),
            text: "source code".into(),
        })
        .collect::<Vec<_>>();
    let mut tree = UiTree::new(crate::CodeDiff::new("diff", lines).height(300.0).render());
    tree.layout(Size::new(700.0, 300.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(698.0, 150.0)));
    assert!(tree.code_diffs["diff"].viewport.scrollbar_drag.is_some());
    let before = tree.code_diffs["diff"].viewport.scroll_y;
    tree.handle_event(UiEvent::PointerLeave);
    tree.handle_event(UiEvent::PointerMove(Point::new(698.0, 5.0)));
    assert!(tree.code_diffs["diff"].viewport.scrollbar_drag.is_none());
    assert_eq!(tree.code_diffs["diff"].viewport.scroll_y, before);
}

#[test]
fn static_selection_preserves_source_newline_and_soft_wraps() {
    let mut tree = UiTree::new(Element::formatted_text(vec![
        TextSpan::new("α\n", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("β", Color::WHITE, 14.0, FontFamily::Sans),
    ]));
    tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
    tree.layout(Size::new(100.0, 50.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(10.0, 23.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("α\nβ"));

    tree.set_root(Element::text("abcd").size(Some(20.0), None));
    tree.layout(Size::new(20.0, 50.0));
    assert_eq!(tree.selected_text(), None);
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(20.0, 23.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("abcd"));
}

#[test]
fn selection_does_not_cut_a_grapheme_across_span_boundaries() {
    let mut tree = UiTree::new(Element::formatted_text(vec![
        TextSpan::new("e", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("\u{301}", Color::rgb(180, 180, 180), 14.0, FontFamily::Sans),
        TextSpan::new("x", Color::WHITE, 14.0, FontFamily::Sans),
    ]));
    tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 10.0);
    tree.layout(Size::new(100.0, 30.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(1.0, 5.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(20.0, 5.0)));
    assert_eq!(tree.selected_text().as_deref(), Some("e\u{301}"));
}

#[test]
fn wrapping_prefers_word_boundaries_and_preserves_newlines() {
    let lines = wrap_lines("El agente trabaja en este proyecto.\nSiguiente", 22.0, 1.0);
    assert_eq!(
        lines,
        ["El agente trabaja en ", "este proyecto.", "Siguiente"]
    );
}

#[test]
fn painter_measurement_controls_proportional_wrapping_and_font_size() {
    let root = Element::text("iiii WWWW")
        .font_family(FontFamily::Sans)
        .font_size(28.0);
    let mut tree = UiTree::new(root);
    tree.set_text_measurer(|part, size, _| {
        part.chars()
            .map(|ch| if ch == 'W' { 7.0 } else { 2.0 })
            .sum::<f32>()
            * size
            / 14.0
    });
    let frame = tree.layout(Size::new(40.0, 100.0));
    let lines = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(lines, ["iiii ", "WW", "WW"]);
}

#[test]
fn input_caret_and_selection_follow_painter_advances() {
    let mut tree = UiTree::new(
        Element::text_input("text", "iW", "")
            .font_family(FontFamily::Sans)
            .padding(0.0)
            .size(Some(120.0), Some(30.0)),
    );
    tree.set_text_measurer(|part, _, _| {
        part.chars()
            .map(|ch| if ch == 'W' { 20.0 } else { 2.0 })
            .sum()
    });
    tree.layout(Size::new(120.0, 30.0));
    assert!(tree.focus("text"));
    let focused = tree.layout(Size::new(120.0, 30.0));
    assert!(focused.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, .. } if (rect.x - 22.0).abs() < 0.01 && rect.width == 1.5
    )));
    tree.handle_event(UiEvent::PointerDown(Point::new(4.0, 4.0)));
    let clicked = tree.layout(Size::new(120.0, 30.0));
    assert!(clicked.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, .. } if (rect.x - 2.0).abs() < 0.01 && rect.width == 1.5
    )));
    tree.handle_event(UiEvent::Key {
        key: Key::A,
        shift: false,
        ctrl: true,
        alt: false,
    });
    let selected = tree.layout(Size::new(120.0, 30.0));
    assert!(selected.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. }
            if *color == Color::rgba(75, 130, 210, 130)
                && (rect.width - 22.0).abs() < 0.01
    )));
}

#[test]
fn text_area_uses_measured_wraps_for_paint_and_vertical_navigation() {
    let mut tree = UiTree::new(
        Element::text_area("area", "iWWW", "")
            .font_family(FontFamily::Sans)
            .padding(0.0)
            .size(Some(20.0), Some(60.0)),
    );
    tree.set_text_measurer(|part, _, _| {
        part.chars()
            .map(|ch| if ch == 'W' { 10.0 } else { 2.0 })
            .sum()
    });
    let frame = tree.layout(Size::new(20.0, 60.0));
    let lines = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(lines, ["iW", "WW"]);
    assert!(tree.focus("area"));
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowLeft,
        shift: false,
        ctrl: false,
        alt: false,
    });
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowUp,
        shift: false,
        ctrl: false,
        alt: false,
    });
    let moved = tree.layout(Size::new(20.0, 60.0));
    assert!(moved.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, .. }
            if (rect.x - 2.0).abs() < 0.01 && rect.y == 0.0 && rect.width == 1.5
    )));
}

#[test]
fn flex_layout_distributes_remaining_height() {
    let root = Element::column(vec![
        Element::text("header").size(None, Some(20.0)),
        Element::panel(Element::text("body")).flex(1.0),
        Element::text("footer").size(None, Some(10.0)),
    ]);
    let frame = UiTree::new(root).layout(Size::new(300.0, 100.0));
    let body = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { position, text, .. } if text == "body" => Some(position.y),
        _ => None,
    });
    let footer = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { position, text, .. } if text == "footer" => Some(position.y),
        _ => None,
    });
    assert_eq!(body, Some(20.0));
    assert_eq!(footer, Some(90.0));
}

#[test]
fn typography_and_border_affect_scene_without_changing_hit_bounds() {
    let accent = Color::rgb(34, 145, 210);
    let root = Element::column(vec![
        Element::text("Title")
            .font_family(FontFamily::Sans)
            .font_size(28.0),
        Element::button("open", "Open")
            .background(Color::rgb(20, 20, 20))
            .border(accent, 1.0)
            .radius(8.0)
            .size(None, Some(32.0)),
    ]);
    let mut tree = UiTree::new(root);
    tree.set_sans_metrics(FontMetrics {
        char_width: 9.0,
        line_height: 20.0,
    });
    let frame = tree.layout(Size::new(200.0, 100.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Text {
            text,
            position,
            font_size: 28.0,
            font_family: FontFamily::Sans,
            ..
        } if text == "Title" && position.y == 0.0
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::StrokeRect {
            color,
            width: 1.0,
            radius: 8.0,
            ..
        } if *color == accent
    )));
    // The title's calibrated line height is 40 at 28px.
    assert_eq!(frame.hit_test(Point::new(5.0, 41.0)), Some("open"));
    assert_eq!(frame.hit_test(Point::new(5.0, 39.0)), None);
}

#[test]
fn shadow_precedes_surface_and_does_not_expand_hit_bounds() {
    let ink = Color::rgba(0, 0, 0, 90);
    let surface = Color::rgb(35, 35, 35);
    let frame = UiTree::new(Element::column(vec![Element::button("card", "Card")
        .background(surface)
        .shadow(ink, Point::new(0.0, 5.0), 16.0, 2.0)
        .radius(12.0)
        .size(Some(120.0), Some(42.0))]))
    .layout(Size::new(200.0, 80.0));
    assert!(matches!(
        frame.commands.first(),
        Some(DrawCommand::Shadow {
            rect: Rect { width: 120.0, height: 42.0, .. },
            shadow: BoxShadow { color, blur: 16.0, spread: 2.0, .. },
            radius: 12.0,
        }) if *color == ink
    ));
    assert!(matches!(
        frame.commands.get(1),
        Some(DrawCommand::FillRect { color, .. }) if *color == surface
    ));
    assert_eq!(frame.hit_test(Point::new(5.0, 40.0)), Some("card"));
    assert_eq!(frame.hit_test(Point::new(5.0, 45.0)), None);
}

#[test]
fn invalid_shadow_measurements_are_sanitized() {
    let element = Element::panel(Element::spacer()).shadow(
        Color::rgba(0, 0, 0, 80),
        Point::new(f32::NAN, f32::INFINITY),
        -5.0,
        f32::INFINITY,
    );
    assert_eq!(
        element.style.shadow,
        Some(BoxShadow::new(
            Color::rgba(0, 0, 0, 80),
            Point::new(0.0, 0.0),
            0.0,
            0.0,
        ))
    );
}

#[test]
fn overlay_paints_after_base_and_owns_topmost_hit() {
    let frame = UiTree::new(Element::overlay(
        Element::button("base", "base").size(None, Some(40.0)),
        Element::button("floating", "floating").size(None, Some(40.0)),
    ))
    .layout(Size::new(120.0, 40.0));
    let labels = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["base", "floating"]);
    assert_eq!(frame.hit_test(Point::new(8.0, 8.0)), Some("floating"));
}

#[test]
fn overlay_escapes_scroll_clip_but_stays_inside_viewport() {
    let scroll_contents = Element::column(vec![
        Element::spacer().size(None, Some(35.0)),
        Element::overlay(
            Element::button("anchor", "anchor").size(None, Some(50.0)),
            Element::button("popup", "popup")
                .background(Color::rgb(40, 40, 40))
                .size(None, Some(50.0)),
        ),
    ]);
    let root = Element::column(vec![
        Element::scroll("list", scroll_contents).size(None, Some(40.0)),
        Element::spacer().flex(1.0),
    ]);
    let frame = UiTree::new(root).layout(Size::new(120.0, 100.0));
    let pop_clip = frame
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::PopClip))
        .unwrap();
    let popup = frame
        .commands
        .iter()
        .position(
            |command| matches!(command, DrawCommand::Text { text, .. } if text == "popup"),
        )
        .unwrap();
    assert!(popup > pop_clip);
    assert_eq!(frame.hit_test(Point::new(8.0, 60.0)), Some("popup"));
    assert_eq!(frame.hit_test(Point::new(8.0, 98.0)), None);
}

#[test]
fn icon_button_uses_one_theme_tint_and_keeps_button_interaction() {
    let tint = Color::rgb(219, 234, 254);
    let mut tree = UiTree::new(
        Element::icon_button("new", Icon::Add, "Nuevo")
            .icon_size(20.0)
            .foreground(tint)
            .size(Some(100.0), Some(36.0)),
    );
    let frame = tree.layout(Size::new(100.0, 36.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Icon {
            rect: Rect {
                width: 16.0,
                height: 16.0,
                ..
            },
            ..
        }
    )));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Icon {
            icon: Icon::Add,
            tint: actual,
            ..
        } if *actual == tint
    )));
    assert_eq!(frame.hit_test(Point::new(12.0, 12.0)), Some("new"));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: false,
            alt: false,
        }),
        vec![UiAction::Clicked { id: "new".into() }]
    );
}

#[test]
fn trailing_icon_shares_one_click_target_and_leaves_room_for_label() {
    let mut tree = UiTree::new(
        Element::icon_button("backend", Icon::Chat, "Codex agent")
            .trailing_icon(Icon::ArrowDown)
            .size(Some(120.0), Some(32.0)),
    );
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
    let frame = tree.layout(Size::new(120.0, 32.0));
    let icons: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Icon { icon, rect, .. } => Some((*icon, *rect)),
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), 2);
    assert_eq!(icons[0].0, Icon::Chat);
    assert_eq!(icons[1].0, Icon::ArrowDown);
    assert!(icons[0].1.x + icons[0].1.width < icons[1].1.x);
    assert_eq!(frame.hit_test(Point::new(110.0, 16.0)), Some("backend"));
    let label = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { text, .. } => Some(text.as_str()),
        _ => None,
    });
    assert!(label.unwrap().ends_with('…'));
}

#[test]
fn icon_only_button_fits_canonical_icon_in_compact_target() {
    let mut tree = UiTree::new(
        Element::icon_button("close", Icon::Close, "").size(Some(24.0), Some(24.0)),
    );
    let frame = tree.layout(Size::new(24.0, 24.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Icon {
            rect: Rect {
                width: 16.0,
                height: 16.0,
                ..
            },
            ..
        }
    )));
}

#[test]
fn icon_only_glyph_and_hover_stay_centered_in_square_hit_target() {
    let hover = Color::rgb(41, 44, 47);
    let pressed = Color::rgb(57, 60, 63);
    for size in [24.0, 26.0, 28.0, 32.0, 36.0] {
        let control = Element::icon_button("toolbar", Icon::Settings, "")
            .padding(4.0)
            .background(Color::TRANSPARENT)
            .hover_background(hover)
            .pressed_background(pressed)
            .transition_ms(0)
            .size(Some(size), Some(size));
        let mut tree = UiTree::new(control);
        let viewport = Size::new(size, size);
        let base = tree.layout(viewport);
        let target = base
            .regions
            .iter()
            .find(|region| region.id == "toolbar")
            .expect("icon-only hit region")
            .rect;
        assert_eq!(target, Rect::new(0.0, 0.0, size, size));
        let glyph = base
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::Icon {
                    icon: Icon::Settings,
                    rect,
                    ..
                } => Some(*rect),
                _ => None,
            })
            .expect("icon glyph");
        assert_eq!(
            glyph,
            Rect::new((size - 16.0) / 2.0, (size - 16.0) / 2.0, 16.0, 16.0)
        );
        tree.handle_event(UiEvent::PointerMove(Point::new(size / 2.0, size / 2.0)));
        let hovered = tree.layout(viewport);
        assert_eq!(hovered.regions, base.regions);
        assert!(hovered.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. } if *rect == target && *color == hover
        )));
        tree.handle_event(UiEvent::PointerDown(Point::new(size / 2.0, size / 2.0)));
        let pressed_frame = tree.layout(viewport);
        assert_eq!(pressed_frame.regions, base.regions);
        assert!(pressed_frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. } if *rect == target && *color == pressed
        )));
    }
}

#[test]
fn text_area_edits_lines_and_submits_only_on_ctrl_enter() {
    let area = || Element::text_area("prompt", "stale", "Ask").size(None, Some(88.0));
    let mut tree = UiTree::new(area());
    tree.layout(Size::new(240.0, 88.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
    tree.set_input_value("prompt", "");
    tree.handle_event(UiEvent::Text("uno".into()));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: false,
            alt: false,
        }),
        vec![UiAction::InputChanged {
            id: "prompt".into(),
            value: "uno\n".into(),
        }]
    );
    tree.handle_event(UiEvent::Text("dos".into()));
    tree.set_root(area());
    tree.layout(Size::new(240.0, 88.0));
    assert_eq!(tree.input_value("prompt"), Some("uno\ndos"));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: true,
            alt: false,
        }),
        vec![UiAction::Submitted {
            id: "prompt".into(),
            value: "uno\ndos".into(),
        }]
    );
    assert_eq!(tree.input_value("prompt"), Some("uno\ndos"));
}

#[test]
fn set_input_selection_selects_characters_and_typing_replaces_them() {
    let area = || Element::text_area("prompt", "a **ñandú** b", "Ask").size(None, Some(88.0));
    let mut tree = UiTree::new(area());
    tree.layout(Size::new(240.0, 88.0));
    tree.set_input_selection("prompt", 4, 9);
    assert_eq!(tree.focused_id(), None, "selecting does not focus");
    assert_eq!(tree.input_selection("prompt"), Some((4, 9)));
    assert_eq!(tree.input_cursor("prompt"), Some(9));
    // The caret stays at `end`, before the anchor here.
    tree.set_input_selection("prompt", 9, 4);
    assert_eq!(tree.input_selection("prompt"), Some((4, 9)));
    assert_eq!(tree.input_cursor("prompt"), Some(4));
    // Both ends are kept within the 13 characters.
    tree.set_input_selection("prompt", 11, 99);
    assert_eq!(tree.input_selection("prompt"), Some((11, 13)));
    tree.set_input_selection("prompt", 99, 99);
    assert_eq!(tree.input_selection("prompt"), Some((13, 13)));
    tree.set_input_selection("prompt", 2, 2);
    assert_eq!(tree.input_selection("prompt"), Some((2, 2)));
    tree.set_input_selection("missing", 0, 1);
    assert_eq!(tree.input_selection("missing"), None);

    assert!(tree.focus("prompt"));
    tree.set_input_selection("prompt", 4, 9);
    assert_eq!(tree.focused_id(), Some("prompt"));
    assert_eq!(tree.input_value("prompt"), Some("a **ñandú** b"));
    assert_eq!(
        tree.handle_event(UiEvent::Text("emú".into())),
        vec![UiAction::InputChanged {
            id: "prompt".into(),
            value: "a **emú** b".into(),
        }]
    );
    assert_eq!(tree.input_selection("prompt"), Some((7, 7)));
}

#[test]
fn button_uses_hover_and_pressed_fill_without_changing_hit_region() {
    let base = Color::rgb(20, 20, 20);
    let hover = Color::rgb(40, 40, 40);
    let pressed = Color::rgb(60, 60, 60);
    let button = Element::icon_button("action", Icon::Add, "Action")
        .background(base)
        .hover_background(hover)
        .pressed_background(pressed)
        .transition_ms(0)
        .size(Some(120.0), Some(32.0));
    let mut tree = UiTree::new(button);
    tree.layout(Size::new(120.0, 32.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
    let frame = tree.layout(Size::new(120.0, 32.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == hover
    )));
    tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
    let frame = tree.layout(Size::new(120.0, 32.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == pressed
    )));
    assert_eq!(frame.hit_test(Point::new(10.0, 10.0)), Some("action"));
}

#[test]
fn pointer_focus_does_not_draw_keyboard_ring() {
    let mut tree =
        UiTree::new(Element::button("action", "Action").size(Some(100.0), Some(28.0)));
    tree.layout(Size::new(100.0, 28.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
    assert_eq!(tree.focused_id(), Some("action"));
    assert!(!tree
        .layout(Size::new(100.0, 28.0))
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));

    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert!(tree
        .layout(Size::new(100.0, 28.0))
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
    // Pointer motion changes hover, but it is not a focus change.
    tree.handle_event(UiEvent::PointerMove(Point::new(101.0, 5.0)));
    assert!(tree
        .layout(Size::new(100.0, 28.0))
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::StrokeRect { .. })));
}

#[test]
fn stationary_pointer_retargets_hover_after_tree_rebuild() {
    let left_hover = Color::rgb(90, 10, 10);
    let right_hover = Color::rgb(10, 90, 10);
    let row = |first: bool| {
        let left = Element::button("left", "Left")
            .hover_background(left_hover)
            .transition_ms(0)
            .size(Some(50.0), Some(28.0));
        let right = Element::button("right", "Right")
            .hover_background(right_hover)
            .transition_ms(0)
            .size(Some(50.0), Some(28.0));
        Element::row(if first {
            vec![left, right]
        } else {
            vec![right, left]
        })
    };
    let mut tree = UiTree::new(row(true));
    tree.layout(Size::new(100.0, 28.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));
    assert_eq!(tree.hovered.as_deref(), Some("left"));

    tree.set_root(row(false));
    let frame = tree.layout(Size::new(100.0, 28.0));
    assert_eq!(tree.hovered.as_deref(), Some("right"));
    assert!(
        frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, rect, .. }
                if *color == right_hover && rect.x == 0.0
        )),
        "commands={:?}; regions={:?}; hovered={:?}",
        frame.commands,
        frame.regions,
        tree.hovered
    );
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == left_hover
    )));
}

#[test]
fn pointer_leave_clears_hover_and_nested_icon_has_priority() {
    let row_hover = Color::rgb(50, 50, 50);
    let icon_hover = Color::rgb(90, 90, 90);
    let row = Element::clickable(
        "row",
        Element::row(vec![
            Element::spacer().size(Some(72.0), Some(28.0)),
            Element::icon_button("more", Icon::More, "")
                .hover_background(icon_hover)
                .transition_ms(0)
                .size(Some(28.0), Some(28.0)),
        ]),
    )
    .hover_background(row_hover)
    .transition_ms(0)
    .size(Some(100.0), Some(28.0));
    let mut tree = UiTree::new(row);
    tree.layout(Size::new(100.0, 28.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(86.0, 10.0)));
    let frame = tree.layout(Size::new(100.0, 28.0));
    assert_eq!(tree.hovered.as_deref(), Some("more"));
    assert!(
        frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == icon_hover
        )),
        "commands={:?}; regions={:?}; hovered={:?}",
        frame.commands,
        frame.regions,
        tree.hovered
    );
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == row_hover
    )));

    tree.handle_event(UiEvent::PointerLeave);
    let frame = tree.layout(Size::new(100.0, 28.0));
    assert_eq!(tree.hovered, None);
    assert!(!frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == icon_hover
    )));
}

#[test]
fn nested_action_can_share_parent_hover_surface_without_sharing_click_target() {
    let row_hover = Color::rgb(50, 50, 50);
    let row = Element::clickable(
        "row",
        Element::row(vec![
            Element::spacer().size(Some(72.0), Some(28.0)),
            Element::icon_button("more", Icon::More, "")
                .background(Color::TRANSPARENT)
                .hover_background(Color::TRANSPARENT)
                .transition_ms(0)
                .size(Some(28.0), Some(28.0)),
        ]),
    )
    .hover_with("more")
    .hover_background(row_hover)
    .transition_ms(0)
    .size(Some(100.0), Some(28.0));
    let mut tree = UiTree::new(row);
    tree.layout(Size::new(100.0, 28.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(86.0, 10.0)));
    let frame = tree.layout(Size::new(100.0, 28.0));
    assert_eq!(frame.hit_test(Point::new(86.0, 10.0)), Some("more"));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, rect, .. }
            if *color == row_hover && rect.width == 100.0
    )));
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 10.0)));
    let frame = tree.layout(Size::new(100.0, 28.0));
    assert_eq!(frame.hit_test(Point::new(20.0, 10.0)), Some("row"));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, rect, .. }
            if *color == row_hover && rect.width == 100.0
    )));
}

#[test]
fn a_shared_surface_stays_lit_in_the_gap_between_its_actions() {
    let hover = Color::rgb(70, 70, 70);
    let row = Element::row(vec![
        Element::clickable("open", Element::spacer()).size(Some(60.0), Some(28.0)),
        Element::spacer().size(Some(10.0), None),
        Element::clickable("more", Element::spacer()).size(Some(24.0), Some(24.0)),
    ])
    .size(Some(100.0), Some(28.0))
    .hover_surface("row-surface", ["open", "more"])
    .hover_background(hover);
    let mut tree = UiTree::new(Element::column(vec![row, Element::spacer().flex(1.0)]));
    tree.set_reduced_motion(true);
    let viewport = Size::new(100.0, 80.0);
    tree.layout(viewport);
    let lit = |tree: &mut UiTree, point: Point| {
        tree.handle_event(UiEvent::PointerMove(point));
        tree.layout(viewport).commands.iter().any(
            |command| matches!(command, DrawCommand::FillRect { color, .. } if *color == hover),
        )
    };
    // In the 10 px gap no control is under the pointer; the band holds.
    assert_eq!(tree.layout(viewport).hit_test(Point::new(65.0, 14.0)), None);
    assert!(lit(&mut tree, Point::new(65.0, 14.0)));
    // Off the row it goes dark.
    assert!(!lit(&mut tree, Point::new(65.0, 60.0)));
}

#[test]
fn a_control_hovered_on_a_lit_shared_surface_stands_out_from_it() {
    let hover = Color::rgb(70, 70, 70);
    let build = || {
        Element::row(vec![
            Element::clickable("open", Element::spacer())
                .hover_background(Color::TRANSPARENT)
                .size(Some(72.0), Some(28.0)),
            Element::clickable("more", Element::spacer())
                .hover_background(hover)
                .foreground(Color::rgb(230, 230, 230))
                .size(Some(24.0), Some(24.0)),
        ])
        .size(Some(100.0), Some(28.0))
        .hover_surface("row-surface", ["open", "more"])
        .hover_background(hover)
    };
    let fills = |point: Point| {
        let mut tree = UiTree::new(build());
        tree.set_reduced_motion(true);
        let viewport = Size::new(100.0, 28.0);
        tree.layout(viewport);
        tree.handle_event(UiEvent::PointerMove(point));
        let frame = tree.layout(viewport);
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if color.a > 0 => Some((rect.width, *color)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // Over the main part: the surface alone is lit.
    assert_eq!(fills(Point::new(20.0, 14.0)), vec![(100.0, hover)]);
    // Over the control: the surface stays lit and the control is lighter.
    let over = fills(Point::new(84.0, 12.0));
    assert_eq!(over[0], (100.0, hover));
    let (width, lifted) = over[1];
    assert_eq!(width, 24.0);
    assert!(lifted.r > hover.r && lifted.g > hover.g && lifted.b > hover.b, "{lifted:?}");
}

#[test]
fn passive_hover_surface_keeps_one_transition_across_independent_actions() {
    let base = Color::rgb(20, 20, 20);
    let hover = Color::rgb(70, 70, 70);
    let pressed = Color::rgb(100, 100, 100);
    let row = Element::row(vec![
        Element::clickable("open", Element::spacer()).size(Some(72.0), Some(28.0)),
        Element::clickable("stage", Element::spacer()).size(Some(28.0), Some(28.0)),
    ])
    .size(Some(100.0), Some(28.0))
    .hover_surface("row-surface", ["open", "stage"])
    .background(base)
    .hover_background(hover)
    .pressed_background(pressed)
    .transition_ms(120);
    let mut tree = UiTree::new(row);
    let viewport = Size::new(100.0, 28.0);
    let now = Instant::now();
    let initial = tree.layout_at(viewport, now);
    let surface_color = |frame: &Frame| {
        frame
            .commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::FillRect {
                    rect,
                    color,
                    radius,
                } if *rect == Rect::new(0.0, 0.0, 100.0, 28.0) && *radius == 0.0 => {
                    Some(*color)
                }
                _ => None,
            })
            .unwrap()
    };
    assert_eq!(surface_color(&initial), base);
    assert!(!tree.focus("row-surface"));
    tree.handle_event_at(UiEvent::PointerMove(Point::new(10.0, 14.0)), now);
    tree.layout_at(viewport, now);
    let halfway = now + Duration::from_millis(60);
    let middle = tree.layout_at(viewport, halfway);
    let color = surface_color(&middle);
    assert!(color.r > base.r && color.r < hover.r);
    let stage = Point::new(86.0, 14.0);
    tree.handle_event_at(UiEvent::PointerMove(stage), halfway);
    let switched = tree.layout_at(viewport, halfway);
    assert_eq!(surface_color(&switched), color);
    assert_eq!(switched.regions, initial.regions);
    let end = now + Duration::from_millis(120);
    assert_eq!(surface_color(&tree.layout_at(viewport, end)), hover);
    tree.set_reduced_motion(true);
    tree.handle_event_at(UiEvent::PointerDown(stage), end);
    assert_eq!(surface_color(&tree.layout_at(viewport, end)), pressed);
    assert_eq!(
        tree.handle_event_at(UiEvent::PointerUp(stage), end),
        vec![UiAction::Clicked { id: "stage".into() }]
    );
    tree.handle_event_at(UiEvent::PointerLeave, end);
    assert_eq!(surface_color(&tree.layout_at(viewport, end)), base);
}

#[test]
fn hover_fill_animates_without_moving_the_button() {
    let start = Instant::now();
    let base = Color::rgb(20, 20, 20);
    let hover = Color::rgb(80, 80, 80);
    let button = Element::button("action", "Action")
        .background(base)
        .hover_background(hover)
        .size(Some(120.0), Some(32.0));
    let mut tree = UiTree::new(button);
    tree.layout_at(Size::new(120.0, 32.0), start);
    tree.handle_event(UiEvent::PointerMove(Point::new(10.0, 10.0)));

    let first = tree.layout_at(Size::new(120.0, 32.0), start);
    assert!(first.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == base
    )));
    let middle = tree.layout_at(Size::new(120.0, 32.0), start + Duration::from_millis(60));
    assert!(middle.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if color.r > base.r && color.r < hover.r
    )));
    let end = tree.layout_at(Size::new(120.0, 32.0), start + Duration::from_millis(120));
    assert!(end.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == hover
    )));
    assert_eq!(first.regions, end.regions);

    tree.set_reduced_motion(true);
    tree.handle_event(UiEvent::PointerMove(Point::new(121.0, 10.0)));
    let reduced = tree.layout_at(Size::new(120.0, 32.0), start);
    assert!(reduced.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { color, .. } if *color == base
    )));
}

#[test]
fn focus_by_id_accepts_text_and_preserves_focus_on_unknown_id() {
    let mut tree = UiTree::new(Element::text_input("query", "foo", "Search"));
    tree.layout(Size::new(240.0, 38.0));
    assert!(tree.focus("query"));
    assert!(!tree.focus("missing"));
    assert_eq!(tree.focused_id(), Some("query"));
    assert_eq!(
        tree.handle_event(UiEvent::Text("bar".into())),
        vec![UiAction::InputChanged {
            id: "query".into(),
            value: "foobar".into(),
        }]
    );
}

#[test]
fn selecting_all_in_an_empty_input_preserves_the_first_typed_character() {
    for multiline in [false, true] {
        let field = |value: &str| {
            if multiline {
                Element::text_area("query", value, "Search")
            } else {
                crate::SearchField::new(
                    "query",
                    value,
                    "Search",
                    crate::ComponentPalette::neutral_dark(),
                )
                .render()
            }
        };
        let viewport = Size::new(320.0, 88.0);
        let mut tree = UiTree::new(field(""));
        let frame = tree.layout(viewport);
        let rect = frame
            .regions
            .iter()
            .find(|region| region.id == "query")
            .unwrap()
            .rect;
        let point = Point::new(rect.x + 16.0, rect.y + rect.height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point));
        assert_eq!(tree.focused_id(), Some("query"));
        tree.handle_event(UiEvent::Key {
            key: Key::A,
            shift: false,
            ctrl: true,
            alt: false,
        });
        let mut expected = String::new();
        for character in "fixture ñ🦀".chars() {
            expected.push(character);
            assert_eq!(
                tree.handle_event(UiEvent::Text(character.to_string())),
                vec![UiAction::InputChanged {
                    id: "query".into(),
                    value: expected.clone(),
                }],
                "every native Text event must append, including the first two"
            );
            tree.set_root(field(&expected));
            tree.layout(viewport);
            assert_eq!(tree.input_value("query"), Some(expected.as_str()));
        }
        // Replacing a nonempty selection remains a single replacement;
        // subsequent native character events then append normally.
        tree.handle_event(UiEvent::Key {
            key: Key::A,
            shift: false,
            ctrl: true,
            alt: false,
        });
        tree.handle_event(UiEvent::Text("n".into()));
        tree.handle_event(UiEvent::Text("a".into()));
        assert_eq!(tree.input_value("query"), Some("na"));
    }
}

#[test]
fn typing_after_a_zero_length_shift_selection_does_not_select_inserted_text() {
    let mut tree = UiTree::new(Element::text_input("query", "", "Search"));
    tree.layout(Size::new(240.0, 38.0));
    assert!(tree.focus("query"));
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowLeft,
        shift: true,
        ctrl: false,
        alt: false,
    });
    tree.handle_event(UiEvent::Paste("ñ".into()));
    tree.handle_event(UiEvent::Text("🦀".into()));
    assert_eq!(tree.input_value("query"), Some("ñ🦀"));
    assert!(
        tree.handle_event(UiEvent::Key {
            key: Key::C,
            shift: false,
            ctrl: true,
            alt: false,
        })
        .is_empty(),
        "typing must leave no active selection"
    );
}

#[test]
fn text_area_selection_replaces_line_breaks_and_moves_vertically() {
    let mut tree = UiTree::new(Element::text_area("prompt", "one\ntwo", ""));
    tree.layout(Size::new(240.0, 88.0));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    tree.handle_event(UiEvent::Key {
        key: Key::A,
        shift: false,
        ctrl: true,
        alt: false,
    });
    let selected = tree.layout(Size::new(240.0, 88.0));
    assert_eq!(
        selected
            .commands
            .iter()
            .filter(|command| matches!(command, DrawCommand::FillRect { color, .. } if *color == Color::rgba(75, 130, 210, 130)))
            .count(),
        2
    );
    tree.handle_event(UiEvent::Text("x\ny".into()));
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowUp,
        shift: true,
        ctrl: false,
        alt: false,
    });
    tree.handle_event(UiEvent::Text("!".into()));
    assert_eq!(tree.input_value("prompt"), Some("x!"));
}

#[test]
fn text_area_wheel_scrolls_wrapped_lines_inside_its_clip() {
    let mut tree =
        UiTree::new(Element::text_area("prompt", "abcdefghi", "").size(Some(40.0), Some(52.0)));
    let bottom = tree.layout(Size::new(40.0, 52.0));
    assert!(bottom
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "ghi")));
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: 18.0,
    });
    let top = tree.layout(Size::new(40.0, 52.0));
    assert!(top
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "abc")));
    assert!(!top
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "ghi")));
}

#[test]
fn hit_testing_respects_scroll_clipping() {
    let child = Element::column(vec![
        Element::button("one", "one").size(None, Some(30.0)),
        Element::button("two", "two").size(None, Some(30.0)),
    ]);
    let mut tree = UiTree::new(Element::scroll("list", child));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(100.0, 30.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("one"));
    assert_eq!(frame.hit_test(Point::new(5.0, 31.0)), None);
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(5.0, 5.0),
        delta_y: -30.0,
    });
    let frame = tree.layout(Size::new(100.0, 30.0));
    assert_eq!(frame.hit_test(Point::new(5.0, 5.0)), Some("two"));
}

#[test]
fn fixed_width_column_in_row_keeps_wrapped_action_scrollable() {
    let copy = "A long description with enough words to wrap into several lines inside a narrow column";
    let content = Element::row(vec![
        Element::spacer().flex(1.0),
        Element::column(vec![
            Element::text(copy),
            Element::button("last_action", "Open").size(None, Some(24.0)),
        ])
        .size(Some(100.0), None),
        Element::spacer().flex(1.0),
    ]);
    let mut tree = UiTree::new(Element::scroll("list", content));
    let initial = tree.layout(Size::new(240.0, 56.0));
    assert!(!initial
        .regions
        .iter()
        .any(|region| region.id == "last_action"));
    assert!(initial.scroll_extents["list"] > 0.0);

    tree.scroll_to_end("list");
    let bottom = tree.layout(Size::new(240.0, 56.0));
    assert!(bottom
        .regions
        .iter()
        .any(|region| region.id == "last_action"));
}

fn scroll_rows(count: usize) -> Element {
    Element::scroll(
        "list",
        Element::column(
            (0..count)
                .map(|index| {
                    Element::button(format!("row:{index}"), "Row").size(None, Some(20.0))
                })
                .collect(),
        ),
    )
}

fn rect_center(rect: Rect) -> Point {
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn assert_offset(tree: &UiTree, id: &str, expected: f32) {
    assert!(
        (tree.scroll_offset(id) - expected).abs() < 0.01,
        "{id}: expected {expected}, got {}",
        tree.scroll_offset(id)
    );
}

#[test]
fn scroll_buttons_share_one_rail_target_and_stop_at_content_bounds() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    for root in [
        scroll_rows(10),
        Element::uniform_list("list", 10, 20.0, |index| {
            Element::button(format!("row:{index}"), "Row")
        }),
    ] {
        let mut tree = UiTree::new(root);
        tree.set_reduced_motion(true);
        let frame = tree.layout_at(viewport, start);
        let bar = frame.scrollbars["list"];
        let up = rect_center(bar.up_button.expect("default up button"));
        let down = rect_center(bar.down_button.expect("default down button"));
        assert_eq!(bar.rail.width, 14.0);
        for row in frame
            .regions
            .iter()
            .filter(|hit| hit.kind == HitKind::Button)
        {
            assert!(
                row.rect.x + row.rect.width <= bar.rail.x,
                "default row hover and hit geometry must clear the rail"
            );
        }
        assert_eq!(
            frame
                .regions
                .iter()
                .filter(|hit| hit.kind == HitKind::Scrollbar)
                .count(),
            1
        );
        for point in [up, down, rect_center(bar.thumb)] {
            assert_eq!(
                frame.hit_region(point).map(|hit| (&*hit.id, hit.kind)),
                Some(("list", HitKind::Scrollbar))
            );
        }
        tree.handle_event_at(UiEvent::PointerDown(up), start);
        tree.handle_event_at(UiEvent::PointerUp(up), start);
        assert_offset(&tree, "list", 0.0);
        tree.handle_event_at(UiEvent::PointerDown(down), start);
        tree.handle_event_at(UiEvent::PointerUp(down), start);
        assert_offset(&tree, "list", 48.0);
        tree.layout_at(viewport, start);
        tree.handle_event_at(UiEvent::PointerDown(up), start);
        tree.handle_event_at(UiEvent::PointerUp(up), start);
        assert_offset(&tree, "list", 0.0);
        tree.scroll_to_end("list");
        let frame = tree.layout_at(viewport, start);
        assert_offset(&tree, "list", 100.0);
        let down = rect_center(frame.scrollbars["list"].down_button.unwrap());
        tree.handle_event_at(UiEvent::PointerDown(down), start);
        tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 100.0);
        tree.handle_event_at(UiEvent::PointerUp(down), start + Duration::from_secs(1));
        assert!(
            !tree.is_animating(),
            "disabled arrows do not keep requesting repaint"
        );
    }
}

#[test]
fn scrollbar_buttons_and_thumb_stay_bounded_in_tiny_viewports() {
    for uniform in [false, true] {
        for height in [0.25, 0.5, 1.0, 12.0, 24.0, 40.0] {
            let root = if uniform {
                Element::uniform_list("list", 10, 20.0, |_| Element::text("Row"))
            } else {
                scroll_rows(10)
            };
            let mut tree = UiTree::new(root);
            let frame = tree.layout(Size::new(12.0, height));
            let bar = frame.scrollbars.get("list").expect("overflow has a rail");
            for rect in [
                Some(bar.rail),
                Some(bar.track),
                Some(bar.thumb),
                bar.up_button,
                bar.down_button,
            ]
            .into_iter()
            .flatten()
            {
                assert!(rect.width > 0.0 && rect.height > 0.0);
                assert!(rect.x >= 0.0 && rect.y >= 0.0);
                assert!(rect.x + rect.width <= 12.0 + 0.001);
                assert!(rect.y + rect.height <= height + 0.001);
            }
            if let Some(up) = bar.up_button {
                assert!(up.y + up.height <= bar.track.y + 0.001);
            }
            if let Some(down) = bar.down_button {
                assert!(bar.track.y + bar.track.height <= down.y + 0.001);
            }
        }
    }
    let mut tree = UiTree::new(scroll_rows(1));
    assert!(!tree
        .layout(Size::new(100.0, 100.0))
        .scrollbars
        .contains_key("list"));
}

#[test]
fn wheel_motion_accumulates_and_reverses_without_display_jumps_across_rebuilds() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(scroll_rows(25));
    tree.layout_at(viewport, start);
    let wheel = |delta_y| UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y,
    };
    tree.handle_event_at(wheel(-100.0), start);
    assert_offset(&tree, "list", 0.0);
    assert!(tree.is_animating());
    tree.layout_at(viewport, start + Duration::from_millis(70));
    assert_offset(&tree, "list", 87.5);
    tree.handle_event_at(wheel(-40.0), start + Duration::from_millis(70));
    assert_offset(&tree, "list", 87.5);
    tree.set_root(scroll_rows(25));
    tree.layout_at(viewport, start + Duration::from_millis(70));
    assert_offset(&tree, "list", 87.5);
    tree.layout_at(viewport, start + Duration::from_millis(140));
    assert_offset(&tree, "list", 133.4375);
    tree.handle_event_at(wheel(100.0), start + Duration::from_millis(140));
    assert_offset(&tree, "list", 133.4375);
    tree.layout_at(viewport, start + Duration::from_millis(210));
    assert_offset(&tree, "list", 51.679_688);
    tree.layout_at(viewport, start + Duration::from_millis(280));
    assert_offset(&tree, "list", 40.0);
    assert!(!tree.is_animating());
}

#[test]
fn direct_scroll_cancels_wheel_motion_and_uses_displayed_position() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(scroll_rows(25));
    tree.layout_at(viewport, start);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        },
        start,
    );
    tree.layout_at(viewport, start + Duration::from_millis(70));
    tree.handle_event_at(
        UiEvent::ScrollDirect {
            position: Point::new(10.0, 10.0),
            delta_y: 20.0,
        },
        start + Duration::from_millis(70),
    );
    assert_offset(&tree, "list", 67.5);
    tree.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&tree, "list", 67.5);
    assert!(!tree.is_animating());
}

#[test]
fn nested_smooth_wheel_consumes_pending_targets_before_bubbling() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 40.0);
    let inner = Element::scroll(
        "inner",
        Element::column(
            (0..10)
                .map(|index| {
                    Element::button(format!("inner:{index}"), "Row").size(None, Some(20.0))
                })
                .collect(),
        ),
    )
    .size(None, Some(80.0));
    let mut tree = UiTree::new(Element::scroll(
        "outer",
        Element::column(vec![inner, Element::spacer().size(None, Some(200.0))]),
    ));
    tree.layout_at(viewport, start);
    for delta_y in [-50.0, -100.0] {
        tree.handle_event_at(
            UiEvent::Scroll {
                position: Point::new(10.0, 10.0),
                delta_y,
            },
            start,
        );
    }
    assert_offset(&tree, "inner", 0.0);
    assert_offset(&tree, "outer", 0.0);
    assert!(tree.is_scroll_at_end("inner", 0.0));
    assert!(!tree.is_scroll_at_end("outer", 0.0));
    tree.layout_at(viewport, start + Duration::from_millis(140));
    assert_offset(&tree, "inner", 120.0);
    assert_offset(&tree, "outer", 30.0);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: 150.0,
        },
        start + Duration::from_millis(140),
    );
    tree.layout_at(viewport, start + Duration::from_millis(280));
    assert_offset(&tree, "inner", 0.0);
    assert_offset(&tree, "outer", 0.0);
}

#[test]
fn track_page_click_animates_without_turning_pointer_motion_into_a_drag() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(
        scroll_rows(25)
            .scrollbar_appearance(crate::ScrollbarAppearance::reserved().buttons(false)),
    );
    let frame = tree.layout_at(viewport, start);
    let bar = frame.scrollbars["list"];
    assert!(bar.up_button.is_none() && bar.down_button.is_none());
    assert_eq!(bar.track, bar.rail);
    tree.handle_event_at(UiEvent::PointerDown(Point::new(94.0, 80.0)), start);
    assert_offset(&tree, "list", 0.0);
    tree.handle_event_at(UiEvent::PointerMove(Point::new(94.0, 10.0)), start);
    tree.handle_event_at(UiEvent::PointerUp(Point::new(94.0, 10.0)), start);
    tree.layout_at(viewport, start + Duration::from_millis(140));
    assert_offset(&tree, "list", 90.0);
}

#[test]
fn thumb_drag_cancels_pending_motion_and_tracks_pointer_directly() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(
        scroll_rows(25).scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
    );
    tree.layout_at(viewport, start);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        },
        start,
    );
    let middle = start + Duration::from_millis(35);
    let frame = tree.layout_at(viewport, middle);
    let bar = frame.scrollbars["list"];
    let grab = rect_center(bar.thumb);
    let displayed = tree.scroll_offset("list");
    tree.handle_event_at(UiEvent::PointerDown(grab), middle);
    assert_offset(&tree, "list", displayed);
    let end = Point::new(
        grab.x,
        bar.track.y + bar.track.height - bar.thumb.height / 2.0,
    );
    tree.handle_event_at(UiEvent::PointerMove(end), middle);
    assert_offset(&tree, "list", 400.0);
    tree.handle_event_at(UiEvent::PointerUp(end), middle);
    tree.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&tree, "list", 400.0);
    assert!(!tree.is_animating());
}

#[test]
fn programmatic_end_index_and_focus_override_pending_wheel_motion() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let wheel = || UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: -100.0,
    };
    let mut end = UiTree::new(scroll_rows(25));
    end.layout_at(viewport, start);
    end.handle_event_at(wheel(), start);
    end.scroll_to_end("list");
    end.layout_at(viewport, start + Duration::from_millis(35));
    assert_offset(&end, "list", 400.0);
    end.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&end, "list", 400.0);

    let mut index = UiTree::new(Element::uniform_list("list", 25, 20.0, |index| {
        Element::button(format!("row:{index}"), "Row")
    }));
    index.layout_at(viewport, start);
    index.handle_event_at(wheel(), start);
    index.scroll_to_index("list", 20);
    index.layout_at(viewport, start + Duration::from_millis(35));
    assert_offset(&index, "list", 320.0);
    index.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&index, "list", 320.0);

    let mut focus = UiTree::new(scroll_rows(25));
    focus.layout_at(viewport, start);
    focus.handle_event_at(wheel(), start);
    focus.layout_at(viewport, start + Duration::from_millis(35));
    assert!(focus.focus("row:0"));
    let revealed = focus.layout_at(viewport, start + Duration::from_millis(35));
    assert_offset(&focus, "list", 0.0);
    assert!(revealed.regions.iter().any(|hit| hit.id == "row:0"));
    focus.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&focus, "list", 0.0);
}

#[test]
fn shrinking_content_and_reduced_motion_stop_pending_scroll_animation() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(scroll_rows(25));
    tree.layout_at(viewport, start);
    let wheel = || UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: -200.0,
    };
    tree.handle_event_at(wheel(), start);
    tree.layout_at(viewport, start + Duration::from_millis(70));
    assert_offset(&tree, "list", 175.0);
    tree.set_root(scroll_rows(7));
    let frame = tree.layout_at(viewport, start + Duration::from_millis(70));
    assert_eq!(frame.scroll_extents["list"], 40.0);
    assert_offset(&tree, "list", 40.0);
    tree.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&tree, "list", 40.0);
    tree.set_root(scroll_rows(2));
    let frame = tree.layout_at(viewport, start + Duration::from_secs(1));
    assert_offset(&tree, "list", 0.0);
    assert!(!frame.scrollbars.contains_key("list"));
    assert!(!tree.is_animating());

    tree.set_root(scroll_rows(25));
    tree.layout_at(viewport, start + Duration::from_secs(2));
    tree.handle_event_at(wheel(), start + Duration::from_secs(2));
    tree.layout_at(
        viewport,
        start + Duration::from_secs(2) + Duration::from_millis(70),
    );
    tree.set_reduced_motion(true);
    tree.layout_at(
        viewport,
        start + Duration::from_secs(2) + Duration::from_millis(70),
    );
    assert_offset(&tree, "list", 200.0);
    assert!(!tree.is_animating());
    tree.handle_event_at(wheel(), start + Duration::from_secs(3));
    assert_offset(&tree, "list", 400.0);
}

#[test]
fn scrollbar_arrow_hold_repeats_after_delay_and_stops_on_release_or_leave() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(scroll_rows(25));
    let frame = tree.layout_at(viewport, start);
    let down = rect_center(frame.scrollbars["list"].down_button.unwrap());
    tree.handle_event_at(UiEvent::PointerDown(down), start);
    tree.layout_at(viewport, start + Duration::from_millis(140));
    assert_offset(&tree, "list", 48.0);
    tree.layout_at(viewport, start + Duration::from_millis(349));
    assert_offset(&tree, "list", 48.0);
    assert!(
        tree.is_animating(),
        "a held arrow needs another repaint after its repeat delay"
    );
    tree.layout_at(viewport, start + Duration::from_millis(350));
    tree.layout_at(viewport, start + Duration::from_millis(410));
    tree.handle_event_at(UiEvent::PointerUp(down), start + Duration::from_millis(411));
    tree.layout_at(viewport, start + Duration::from_millis(600));
    assert_offset(&tree, "list", 144.0);
    assert!(!tree.is_animating());
    tree.layout_at(viewport, start + Duration::from_secs(2));
    assert_offset(&tree, "list", 144.0);

    for cancel in [UiEvent::PointerUp(down), UiEvent::PointerLeave] {
        let mut tree = UiTree::new(scroll_rows(25));
        tree.layout_at(viewport, start);
        tree.handle_event_at(UiEvent::PointerDown(down), start);
        tree.handle_event_at(cancel, start + Duration::from_millis(100));
        tree.layout_at(viewport, start + Duration::from_secs(1));
        assert_offset(&tree, "list", 48.0);
        assert!(!tree.is_animating());
    }
}

#[test]
fn pending_wheel_target_preserves_stream_follow_end_intention() {
    let start = Instant::now();
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(scroll_rows(10));
    tree.layout_at(viewport, start);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        },
        start,
    );
    assert_offset(&tree, "list", 0.0);
    assert!(
        tree.is_scroll_at_end("list", 0.0),
        "follow-end uses intended target during interpolation"
    );
    tree.set_root(scroll_rows(15));
    tree.scroll_to_end("list");
    tree.layout_at(viewport, start + Duration::from_millis(30));
    assert_offset(&tree, "list", 200.0);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: 50.0,
        },
        start + Duration::from_millis(30),
    );
    assert!(
        !tree.is_scroll_at_end("list", 0.0),
        "scrolling away immediately releases follow-end"
    );
}

#[test]
fn animated_uniform_list_remains_lazy_at_each_intermediate_frame() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let start = Instant::now();
    let viewport = Size::new(160.0, 100.0);
    let built = Arc::new(AtomicUsize::new(0));
    let counter = built.clone();
    let mut tree = UiTree::new(Element::uniform_list("list", 1_000, 20.0, move |index| {
        counter.fetch_add(1, Ordering::SeqCst);
        Element::button(format!("row:{index}"), "Row")
    }));
    tree.layout_at(viewport, start);
    assert_eq!(built.load(Ordering::SeqCst), 5);
    tree.handle_event_at(
        UiEvent::Scroll {
            position: Point::new(10.0, 10.0),
            delta_y: -100.0,
        },
        start,
    );
    for milliseconds in [35, 70, 105, 140] {
        let before = built.load(Ordering::SeqCst);
        let frame = tree.layout_at(viewport, start + Duration::from_millis(milliseconds));
        let after = built.load(Ordering::SeqCst);
        assert!(
            (5..=12).contains(&(after - before)),
            "only intersecting rows plus one hover repaint are built at {milliseconds}ms"
        );
        assert!(
            frame
                .regions
                .iter()
                .filter(|hit| hit.kind == HitKind::Button)
                .count()
                <= 6
        );
    }
    assert_offset(&tree, "list", 100.0);
}

#[test]
fn scrollbar_tracks_overflow_and_preserves_child_hit_regions() {
    let mut tree = UiTree::new(
        scroll_rows(10).scrollbar_appearance(crate::ScrollbarAppearance::overlay()),
    );
    let frame = tree.layout(Size::new(100.0, 100.0));
    let bar = frame.scrollbars["list"];
    assert_eq!(bar.rail, Rect::new(86.0, 0.0, 14.0, 100.0));
    assert_eq!(bar.track, Rect::new(86.0, 14.0, 14.0, 72.0));
    assert_eq!(bar.up_button, Some(Rect::new(86.0, 0.0, 14.0, 14.0)));
    assert_eq!(bar.down_button, Some(Rect::new(86.0, 86.0, 14.0, 14.0)));
    assert_eq!(bar.thumb.height, 36.0);
    assert_eq!(bar.max_offset, 100.0);
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "row:0")
            .unwrap()
            .rect
            .width,
        100.0,
        "an overlay must not shrink the content when it overflows"
    );
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { rect, .. } if *rect == bar.thumb
    )));
    assert!(
        frame.commands.iter().all(|command| !matches!(command,
            DrawCommand::FillRect { rect, .. } if *rect == bar.track
        )),
        "the idle track must be transparent"
    );
    assert_eq!(frame.hit_test(Point::new(82.0, 10.0)), Some("row:0"));
    assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("list"));
    assert_eq!(
        frame.hit_region(Point::new(94.0, 10.0)).map(|hit| hit.kind),
        Some(HitKind::Scrollbar)
    );
    tree.handle_event(UiEvent::PointerDown(Point::new(82.0, 10.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(82.0, 10.0))),
        vec![UiAction::Clicked { id: "row:0".into() }]
    );

    tree.set_root(scroll_rows(2).scrollbar_appearance(crate::ScrollbarAppearance::overlay()));
    let frame = tree.layout(Size::new(100.0, 100.0));
    assert!(!frame.scrollbars.contains_key("list"));
    assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("row:0"));
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "row:0")
            .unwrap()
            .rect
            .width,
        100.0,
        "content width must stay stable when overflow disappears"
    );
}

#[test]
fn overlay_rail_leaves_a_trailing_action_clickable() {
    let rows = Element::column(
        (0..10)
            .map(|index| {
                Element::row(vec![
                    Element::spacer().flex(1.0),
                    Element::button(format!("action:{index}"), "···")
                        .padding(0.0)
                        .size(Some(20.0), Some(20.0)),
                ])
                .size(None, Some(20.0))
            })
            .collect(),
    );
    let mut tree = UiTree::new(
        Element::scroll("list", rows).scrollbar_appearance(
            crate::ScrollbarAppearance::overlay()
                .buttons(false)
                .rail_width(8.0),
        ),
    );
    let frame = tree.layout(Size::new(100.0, 100.0));
    assert_eq!(frame.hit_test(Point::new(90.0, 10.0)), Some("action:0"));
    assert_eq!(
        frame.hit_test(Point::new(96.0, 70.0)),
        Some("action:3"),
        "the transparent track must not take clicks from a trailing action"
    );
    assert_eq!(
        frame.hit_test(Point::new(96.0, 10.0)),
        Some("list"),
        "the visible thumb remains draggable"
    );
    tree.handle_event(UiEvent::PointerDown(Point::new(90.0, 10.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(90.0, 10.0))),
        vec![UiAction::Clicked {
            id: "action:0".into()
        }]
    );
    tree.handle_event(UiEvent::PointerDown(Point::new(96.0, 70.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(96.0, 70.0))),
        vec![UiAction::Clicked {
            id: "action:3".into()
        }]
    );
    tree.handle_event(UiEvent::PointerDown(Point::new(96.0, 25.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(96.0, 50.0)));
    assert_eq!(tree.scroll_offset("list"), 50.0);
}

#[test]
fn focusing_nested_offscreen_action_reveals_it_without_offscreen_hits() {
    let rows = Element::column(
        (0..10)
            .map(|index| {
                Element::panel(
                    Element::row(vec![
                        Element::spacer().flex(1.0),
                        Element::button(format!("action:{index}"), "Open")
                            .size(Some(40.0), Some(20.0)),
                    ])
                    .size(None, Some(20.0)),
                )
                .size(None, Some(20.0))
            })
            .collect(),
    );
    let mut tree = UiTree::new(Element::scroll("list", rows));
    let initial = tree.layout(Size::new(100.0, 60.0));
    assert!(!initial.regions.iter().any(|hit| hit.id == "action:6"));
    assert!(!initial.scroll_focus_targets.contains_key("action:6"));

    assert!(tree.focus("action:6"));
    let revealed = tree.layout(Size::new(100.0, 60.0));
    assert_eq!(tree.scroll_offset("list"), 80.0);
    assert!(revealed.regions.iter().any(|hit| hit.id == "action:6"));
    assert!(!revealed.regions.iter().any(|hit| hit.id == "action:0"));
    assert_eq!(revealed.hit_test(Point::new(80.0, 50.0)), Some("action:6"));
}

/// Rows of uneven heights, so no row's position follows from its index.
fn uneven_rows() -> Element {
    Element::column(
        (0..10)
            .map(|index| {
                Element::button(format!("row:{index}"), "Row")
                    .size(None, Some(20.0 + 10.0 * (index % 3) as f32))
            })
            .collect(),
    )
}

#[test]
fn reveal_scrolls_a_control_of_any_height_into_view_with_a_margin() {
    let viewport = Size::new(100.0, 100.0);
    let mut tree = UiTree::new(Element::scroll("list", uneven_rows()));
    tree.set_reduced_motion(true);
    let initial = tree.layout(viewport);
    assert!(!initial.regions.iter().any(|hit| hit.id == "row:7"));

    // Rows 0..=6 are 200 tall and row 7 is 30: its end, with the margin
    // under it, lands on the viewport's end.
    tree.reveal("row:7");
    let revealed = tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), 230.0 + REVEAL_MARGIN - 100.0);
    let row = revealed.regions.iter().find(|hit| hit.id == "row:7").unwrap().rect;
    assert_eq!((row.y + row.height, row.height), (100.0 - REVEAL_MARGIN, 30.0));
    assert_eq!(tree.focused_id(), None, "revealing does not move focus");

    // A control above the viewport comes down to the margin under its top.
    tree.reveal("row:2");
    let above = tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), 50.0 - REVEAL_MARGIN);
    let row = above.regions.iter().find(|hit| hit.id == "row:2").unwrap().rect;
    assert_eq!(row.y, REVEAL_MARGIN);

    // In view already, or not in the tree: nothing moves, nothing lingers.
    let offset = tree.scroll_offset("list");
    for id in ["row:3", "missing"] {
        tree.reveal(id);
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("list"), offset, "{id}");
    }
    tree.set_root(Element::scroll("list", uneven_rows()));
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), offset);

    // A margin of the host's own; one too large for the viewport shrinks
    // to center the control, and a bad one falls back to the default.
    tree.reveal_with_margin("row:7", 30.0);
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), 230.0 + 30.0 - 100.0);
    tree.reveal_with_margin("row:2", 500.0);
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), 50.0 - (100.0 - 40.0) / 2.0);
    tree.reveal_with_margin("row:7", f32::NAN);
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("list"), 230.0 + REVEAL_MARGIN - 100.0);
}

#[test]
fn reveal_scrolls_every_area_around_the_control_and_yields_to_focus() {
    let viewport = Size::new(100.0, 100.0);
    let root = || {
        Element::scroll(
            "page",
            Element::column(vec![
                Element::button("top", "Top").size(None, Some(20.0)),
                Element::spacer().size(None, Some(300.0)),
                Element::scroll("inner", uneven_rows()).size(None, Some(60.0)),
            ]),
        )
    };
    let mut tree = UiTree::new(root());
    tree.set_reduced_motion(true);
    tree.layout(viewport);
    // A focus request in the same frame is located first; the reveal
    // still resolves in that layout call.
    assert!(tree.focus("top"));
    tree.reveal("row:9");
    let frame = tree.layout(viewport);
    assert_eq!(tree.focused_id(), Some("top"));
    assert!(tree.scroll_offset("page") > 0.0);
    assert!(tree.scroll_offset("inner") > 0.0);
    let row = frame.regions.iter().find(|hit| hit.id == "row:9").unwrap().rect;
    assert!(row.y >= 0.0 && row.y + row.height <= viewport.height, "{row:?}");
    assert_eq!(row.height, 20.0);
}

#[test]
fn reserved_rail_remains_available_for_dense_right_edge_content() {
    let rows = Element::column(
        (0..10)
            .map(|index| Element::button(format!("row:{index}"), "Row").size(None, Some(20.0)))
            .collect(),
    );
    let mut tree = UiTree::new(
        Element::scroll("list", rows)
            .scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
    );
    let frame = tree.layout(Size::new(100.0, 100.0));
    assert_eq!(frame.scrollbars["list"].rail.width, 14.0);
    assert_eq!(frame.hit_test(Point::new(94.0, 10.0)), Some("list"));
    assert_eq!(frame.hit_test(Point::new(82.0, 10.0)), Some("row:0"));
}

#[test]
fn reserved_scrollbar_drag_and_track_click_move_retained_scroll_offset() {
    let mut tree = UiTree::new(
        scroll_rows(10).scrollbar_appearance(crate::ScrollbarAppearance::reserved()),
    );
    tree.set_reduced_motion(true);
    tree.layout(Size::new(100.0, 100.0));
    assert!(tree
        .handle_event(UiEvent::PointerDown(Point::new(94.0, 32.0)))
        .is_empty());
    tree.handle_event(UiEvent::PointerMove(Point::new(94.0, 50.0)));
    assert_eq!(tree.scroll_offset("list"), 50.0);
    assert!(tree
        .handle_event(UiEvent::PointerUp(Point::new(94.0, 50.0)))
        .is_empty());
    let frame = tree.layout(Size::new(100.0, 100.0));
    assert_eq!(frame.scrollbars["list"].thumb.y, 32.0);

    tree.handle_event(UiEvent::PointerDown(Point::new(94.0, 80.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(94.0, 80.0)));
    assert_eq!(tree.scroll_offset("list"), 100.0);
    tree.set_root(scroll_rows(15).scrollbar_appearance(crate::ScrollbarAppearance::reserved()));
    let frame = tree.layout(Size::new(100.0, 100.0));
    assert_eq!(tree.scroll_offset("list"), 100.0);
    assert_eq!(frame.scrollbars["list"].max_offset, 200.0);
}

#[test]
fn scrollbar_is_bounded_in_narrow_and_clipped_nested_viewports() {
    let mut tree = UiTree::new(scroll_rows(4));
    let frame = tree.layout(Size::new(12.0, 40.0));
    let bar = frame.scrollbars["list"];
    assert_eq!(bar.rail, Rect::new(8.0, 0.0, 4.0, 40.0));
    assert!(bar.track.y >= bar.rail.y);
    assert!(bar.track.y + bar.track.height <= bar.rail.y + bar.rail.height);
    assert!(bar.thumb.width > 0.0 && bar.thumb.height > 0.0);
    assert_eq!(frame.hit_test(Point::new(2.0, 10.0)), Some("row:0"));
    assert_eq!(frame.hit_test(Point::new(10.0, 10.0)), Some("list"));

    let inner = Element::scroll(
        "inner",
        Element::column(
            (0..10)
                .map(|index| {
                    Element::button(format!("inner:{index}"), "I").size(None, Some(20.0))
                })
                .collect(),
        ),
    )
    .size(None, Some(80.0));
    let mut nested = UiTree::new(Element::scroll("outer", inner));
    let frame = nested.layout(Size::new(100.0, 40.0));
    assert!(frame.scrollbars.contains_key("inner"));
    assert_eq!(frame.hit_test(Point::new(82.0, 41.0)), None);
    assert_eq!(
        frame.hit_region(Point::new(96.0, 10.0)).map(|hit| hit.kind),
        Some(HitKind::Scrollbar)
    );
}

#[test]
fn exhausted_floating_menu_does_not_scroll_covered_background() {
    let viewport = Size::new(320.0, 240.0);
    let menu = Element::scroll(
        "menu",
        Element::column(
            (0..12)
                .map(|index| {
                    Element::button(format!("menu:{index}"), "Item").size(None, Some(20.0))
                })
                .collect(),
        ),
    )
    .size(Some(160.0), Some(80.0));
    let mut tree = UiTree::new(Element::popover(
        Element::scroll("background", Element::spacer().size(None, Some(600.0))),
        menu,
        "dismiss_menu",
        PopoverPlacement::BottomStart,
    ));
    tree.set_reduced_motion(true);
    tree.scroll_to_end("menu");
    let frame = tree.layout(viewport);
    let menu = frame
        .regions
        .iter()
        .find(|region| region.kind == HitKind::Scroll && region.id == "menu")
        .unwrap();
    let point = Point::new(menu.rect.x + 20.0, menu.rect.y + 20.0);
    assert!(
        frame.regions.iter().any(|region| {
            region.kind == HitKind::Scroll
                && region.id == "background"
                && region.rect.contains(point)
        }),
        "the background scroll is covered by this menu"
    );
    let menu_end = tree.scroll_offset("menu");
    assert!(menu_end > 0.0);
    for event in [
        UiEvent::Scroll {
            position: point,
            delta_y: -40.0,
        },
        UiEvent::ScrollDirect {
            position: point,
            delta_y: -40.0,
        },
    ] {
        tree.handle_event(event);
        tree.layout(viewport);
        assert_eq!(tree.scroll_offset("menu"), menu_end);
        assert_eq!(tree.scroll_offset("background"), 0.0);
    }
    tree.handle_event(UiEvent::Scroll {
        position: point,
        delta_y: 40.0,
    });
    tree.layout(viewport);
    assert_eq!(tree.scroll_offset("menu"), menu_end - 40.0);
    assert_eq!(tree.scroll_offset("background"), 0.0);
}

#[test]
fn nested_wheel_bubbles_when_inner_scroll_reaches_its_end() {
    let inner = Element::scroll(
        "inner",
        Element::column(
            (0..10)
                .map(|index| {
                    Element::button(format!("inner:{index}"), "Row").size(None, Some(20.0))
                })
                .collect(),
        ),
    )
    .size(None, Some(80.0));
    let outer = Element::scroll(
        "outer",
        Element::column(vec![inner, Element::spacer().size(None, Some(200.0))]),
    );
    let mut tree = UiTree::new(outer);
    tree.set_reduced_motion(true);
    tree.layout(Size::new(100.0, 40.0));
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: -50.0,
    });
    assert_eq!(tree.scroll_offset("inner"), 50.0);
    assert_eq!(tree.scroll_offset("outer"), 0.0);
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: -100.0,
    });
    assert_eq!(tree.scroll_offset("inner"), 120.0);
    assert_eq!(tree.scroll_offset("outer"), 30.0);
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: -30.0,
    });
    assert_eq!(tree.scroll_offset("outer"), 60.0);
}

fn nested_diff_scroll_tree(rows: usize, height: f32, long_lines: bool) -> UiTree {
    use crate::Component;
    let lines: Vec<_> = (0..rows)
        .map(|index| crate::CodeDiffLine {
            kind: crate::CodeDiffLineKind::Context,
            old_line: Some(index + 1),
            new_line: Some(index + 1),
            text: if long_lines {
                "long line ".repeat(100)
            } else {
                format!("line {index}")
            },
        })
        .collect();
    let diff = crate::CodeDiff::new("review", lines)
        .height(height)
        .render();
    UiTree::new(Element::scroll(
        "outer",
        Element::column(vec![diff, Element::spacer().size(None, Some(400.0))]),
    ))
}

#[test]
fn nested_short_diff_bubbles_direct_and_smooth_wheel_to_parent() {
    let now = Instant::now();
    for direct in [false, true] {
        let mut tree = nested_diff_scroll_tree(2, 80.0, false);
        tree.layout_at(Size::new(360.0, 100.0), now);
        let position = Point::new(120.0, 30.0);
        assert_eq!(
            tree.frame.hit_region(position).unwrap().kind,
            HitKind::CodeDiff
        );
        assert_eq!(tree.code_diffs["review"].viewport.max_y, 0.0);
        tree.handle_event_at(
            if direct {
                UiEvent::ScrollDirect {
                    position,
                    delta_y: -50.0,
                }
            } else {
                UiEvent::Scroll {
                    position,
                    delta_y: -50.0,
                }
            },
            now,
        );
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
        assert_eq!(tree.scroll_target("outer"), 50.0);
        tree.layout_at(Size::new(360.0, 100.0), now + Duration::from_secs(1));
        assert_eq!(tree.scroll_offset("outer"), 50.0);
    }
}

#[test]
fn nested_long_diff_bubbles_only_vertical_residual_at_each_boundary() {
    let now = Instant::now();
    for direct in [false, true] {
        let mut tree = nested_diff_scroll_tree(10, 140.0, false);
        tree.layout_at(Size::new(360.0, 100.0), now);
        let position = Point::new(120.0, 30.0);
        let max_y = tree.code_diffs["review"].viewport.max_y;
        assert!(max_y > 30.0 && max_y < 110.0);
        let scroll = |tree: &mut UiTree, delta_y| {
            tree.handle_event_at(
                if direct {
                    UiEvent::ScrollDirect { position, delta_y }
                } else {
                    UiEvent::Scroll { position, delta_y }
                },
                now,
            )
        };
        scroll(&mut tree, -30.0);
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 30.0);
        assert_eq!(tree.scroll_target("outer"), 0.0);
        scroll(&mut tree, -80.0);
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, max_y);
        let first_residual = 110.0 - max_y;
        assert_eq!(tree.scroll_target("outer"), first_residual);
        scroll(&mut tree, -30.0);
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, max_y);
        assert_eq!(tree.scroll_target("outer"), first_residual + 30.0);
        // Reversing direction first moves the diff to its start; only the
        // remaining delta moves the enclosing transcript back upward.
        scroll(&mut tree, 100.0);
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
        assert_eq!(tree.scroll_target("outer"), 40.0);
    }
}

#[test]
fn nested_diff_shift_wheel_keeps_horizontal_intent_and_bubbles_diagonal_y() {
    let now = Instant::now();
    for direct in [false, true] {
        let mut tree = nested_diff_scroll_tree(2, 80.0, true);
        tree.layout_at(Size::new(360.0, 100.0), now);
        let position = Point::new(120.0, 30.0);
        assert!(tree.code_diffs["review"].viewport.max_x > 80.0);
        tree.handle_event_at(
            UiEvent::Scroll2D {
                position,
                delta_x: 0.0,
                delta_y: -60.0,
                direct,
                shift: true,
            },
            now,
        );
        assert_eq!(tree.code_diffs["review"].viewport.scroll_x, 60.0);
        assert_eq!(tree.scroll_target("outer"), 0.0);
        tree.handle_event_at(
            UiEvent::Scroll2D {
                position,
                delta_x: -20.0,
                delta_y: -40.0,
                direct,
                shift: false,
            },
            now,
        );
        assert_eq!(tree.code_diffs["review"].viewport.scroll_x, 80.0);
        assert_eq!(tree.code_diffs["review"].viewport.scroll_y, 0.0);
        assert_eq!(tree.scroll_target("outer"), 40.0);
    }
}

#[test]
fn shifted_two_axis_wheel_outside_code_preserves_legacy_vertical_contract() {
    let now = Instant::now();
    let position = Point::new(10.0, 10.0);
    for direct in [false, true] {
        let mut legacy = UiTree::new(scroll_rows(10));
        let mut two_axis = UiTree::new(scroll_rows(10));
        for tree in [&mut legacy, &mut two_axis] {
            tree.layout_at(Size::new(100.0, 40.0), now);
        }
        legacy.handle_event_at(
            if direct {
                UiEvent::ScrollDirect {
                    position,
                    delta_y: -75.0,
                }
            } else {
                UiEvent::Scroll {
                    position,
                    delta_y: -75.0,
                }
            },
            now,
        );
        two_axis.handle_event_at(
            UiEvent::Scroll2D {
                position,
                delta_x: 0.0,
                delta_y: -75.0,
                direct,
                shift: true,
            },
            now,
        );
        assert_eq!(legacy.scroll_target("list"), 75.0);
        assert_eq!(two_axis.scroll_target("list"), legacy.scroll_target("list"));
        let size = Size::new(100.0, 40.0);
        let settled = now + Duration::from_secs(1);
        legacy.layout_at(size, settled);
        two_axis.layout_at(size, settled);
        assert_eq!(two_axis.scroll_offset("list"), legacy.scroll_offset("list"));
    }
}

#[test]
fn invalid_wheel_delta_does_not_change_scroll_position() {
    let mut tree = UiTree::new(scroll_rows(10));
    tree.layout(Size::new(100.0, 40.0));
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: f32::NAN,
    });
    assert_eq!(tree.scroll_offset("list"), 0.0);
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(10.0, 10.0),
        delta_y: f32::INFINITY,
    });
    assert_eq!(tree.scroll_offset("list"), 0.0);
}

#[test]
fn input_state_survives_tree_rebuild_and_edits_unicode() {
    let mut tree = UiTree::new(Element::text_input("prompt", "", "Ask"));
    tree.layout(Size::new(200.0, 40.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(10.0, 10.0)));
    assert_eq!(
        tree.handle_event(UiEvent::Text("a🦀b".into())),
        vec![UiAction::InputChanged {
            id: "prompt".into(),
            value: "a🦀b".into()
        }]
    );
    tree.handle_event(UiEvent::Key {
        key: Key::ArrowLeft,
        shift: true,
        ctrl: false,
        alt: false,
    });
    tree.handle_event(UiEvent::Key {
        key: Key::Backspace,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.input_value("prompt"), Some("a🦀"));
    tree.set_root(Element::text_input("prompt", "stale", "Ask"));
    tree.layout(Size::new(200.0, 40.0));
    assert_eq!(tree.focused_id(), Some("prompt"));
    assert_eq!(tree.input_value("prompt"), Some("a🦀"));
}

#[test]
fn click_requires_release_inside_same_button() {
    let mut tree = UiTree::new(Element::button("send", "Send"));
    tree.layout(Size::new(100.0, 40.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
    assert!(tree
        .handle_event(UiEvent::PointerUp(Point::new(120.0, 5.0)))
        .is_empty());
    tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(Point::new(5.0, 5.0))),
        vec![UiAction::Clicked { id: "send".into() }]
    );
}

#[test]
fn scroll_to_end_uses_new_content_height() {
    let rows = |count: usize| {
        Element::scroll(
            "transcript",
            Element::column(
                (0..count)
                    .map(|index| Element::text(index.to_string()).size(None, Some(20.0)))
                    .collect(),
            ),
        )
    };
    let mut tree = UiTree::new(rows(2));
    tree.layout(Size::new(100.0, 20.0));
    tree.set_root(rows(4));
    tree.scroll_to_end("transcript");
    let frame = tree.layout(Size::new(100.0, 20.0));
    assert_eq!(tree.scroll_offset("transcript"), 60.0);
    assert!(frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "3")));
    assert!(!frame
        .commands
        .iter()
        .any(|command| matches!(command, DrawCommand::Text { text, .. } if text == "0")));
}

#[test]
fn uniform_list_builds_only_visible_rows_and_reaches_last_of_one_thousand() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let built = Arc::new(AtomicUsize::new(0));
    let list = |built: Arc<AtomicUsize>| {
        Element::uniform_list("sessions", 1_000, 20.0, move |index| {
            built.fetch_add(1, Ordering::SeqCst);
            Element::button(format!("session:{index}"), format!("Session {index}"))
        })
    };
    let mut tree = UiTree::new(list(built.clone()));
    let frame = tree.layout(Size::new(160.0, 100.0));
    assert_eq!(built.load(Ordering::SeqCst), 5);
    assert_eq!(frame.scroll_extents["sessions"], 19_900.0);
    assert_eq!(
        frame
            .regions
            .iter()
            .filter(|hit| hit.id.starts_with("session:"))
            .count(),
        5
    );
    assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:4"));
    assert_eq!(frame.hit_test(Point::new(30.0, 101.0)), None);
    assert_eq!(frame.hit_test(Point::new(140.0, 90.0)), Some("session:4"));
    assert_eq!(frame.hit_test(Point::new(155.0, 90.0)), Some("sessions"));
    assert_eq!(frame.hit_test(Point::new(155.0, 10.0)), Some("sessions"));
    assert_eq!(
        frame
            .regions
            .iter()
            .find(|region| region.id == "session:4")
            .unwrap()
            .rect
            .width,
        146.0,
        "default virtualized rows reserve a separate scrollbar rail"
    );
    let rail = frame.scrollbars["sessions"].rail;
    for row in frame
        .regions
        .iter()
        .filter(|hit| hit.id.starts_with("session:"))
    {
        assert!(
            row.rect.x + row.rect.width <= rail.x,
            "row hover and hit geometry must not touch the scrollbar rail"
        );
    }

    tree.scroll_to_index("sessions", 999);
    let frame = tree.layout(Size::new(160.0, 100.0));
    assert_eq!(tree.scroll_offset("sessions"), 19_900.0);
    assert_eq!(built.load(Ordering::SeqCst), 10);
    assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:999"));
    assert!(!frame.regions.iter().any(|hit| hit.id == "session:0"));

    tree.set_root(list(built.clone()));
    tree.layout(Size::new(160.0, 100.0));
    assert_eq!(tree.scroll_offset("sessions"), 19_900.0);
    assert_eq!(built.load(Ordering::SeqCst), 15);
}

#[test]
fn a_row_of_a_uniform_list_opens_content_under_itself_and_the_list_stays_lazy() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    const SIZE: Size = Size::new(160.0, 100.0);
    let built = Arc::new(AtomicUsize::new(0));
    let list = |built: Arc<AtomicUsize>, open: Option<(usize, f32)>| {
        let list = Element::uniform_list("sessions", 1_000, 20.0, move |index| {
            built.fetch_add(1, Ordering::SeqCst);
            Element::button(format!("session:{index}"), format!("Session {index}"))
        });
        match open {
            Some((index, height)) => list.list_expansion(
                index,
                Element::button("detail", "Detail").size(None, Some(height)),
            ),
            None => list,
        }
    };
    let rect = |frame: &Frame, id: &str| {
        frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
    };
    let mut tree = UiTree::new(list(built.clone(), Some((1, 50.0))));
    let frame = tree.layout(SIZE);
    // Under its row, as wide as a row and as tall as it asked to be; the
    // rows after it start where it ends.
    let (row, detail, next) = (
        rect(&frame, "session:1").unwrap(),
        rect(&frame, "detail").unwrap(),
        rect(&frame, "session:2").unwrap(),
    );
    assert_eq!((detail.y, detail.height), (row.y + row.height, 50.0));
    assert_eq!((detail.x, detail.width), (row.x, row.width));
    assert_eq!(next.y, detail.y + detail.height);
    // The list is that much longer, and builds only the rows that show.
    assert_eq!(frame.scroll_extents["sessions"], 19_900.0 + 50.0);
    assert_eq!(built.load(Ordering::SeqCst), 3);
    assert_eq!(frame.hit_test(Point::new(30.0, 60.0)), Some("detail"));
    assert_eq!(frame.hit_test(Point::new(30.0, 95.0)), Some("session:2"));

    // Scrolled into it, its row is still built with it and nothing above.
    tree.handle_event(UiEvent::ScrollDirect { position: Point::new(30.0, 50.0), delta_y: -60.0 });
    let frame = tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("sessions"), 60.0);
    // Its part in view: the last thirty of its fifty.
    let detail = rect(&frame, "detail").unwrap();
    assert_eq!((detail.y, detail.height), (0.0, 30.0));
    assert!(rect(&frame, "session:0").is_none());
    assert_eq!(rect(&frame, "session:2").unwrap().y, 30.0);
    assert_eq!(frame.hit_test(Point::new(30.0, 10.0)), Some("detail"));

    // Far from it, neither it nor its row is built, and a row is revealed
    // where it stands under what was opened.
    tree.scroll_to_index("sessions", 500);
    let frame = tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("sessions"), 501.0 * 20.0 + 50.0 - 100.0);
    assert!(rect(&frame, "detail").is_none() && rect(&frame, "session:1").is_none());
    assert_eq!(rect(&frame, "session:500").unwrap().y, 80.0);
    tree.scroll_to_index("sessions", 999);
    let frame = tree.layout(SIZE);
    assert_eq!(tree.scroll_offset("sessions"), 19_950.0);
    assert_eq!(frame.hit_test(Point::new(30.0, 90.0)), Some("session:999"));

    // Closed again, the list is the rows alone.
    tree.set_root(list(built.clone(), None));
    let frame = tree.layout(SIZE);
    assert_eq!(frame.scroll_extents["sessions"], 19_900.0);
    assert!(rect(&frame, "detail").is_none());
    // An index past the rows opens nothing.
    tree.set_root(list(built, Some((1_000, 50.0))));
    assert_eq!(tree.layout(SIZE).scroll_extents["sessions"], 19_900.0);
}

#[test]
fn the_keyboard_walks_from_a_row_into_what_it_opened_and_on_to_the_next() {
    let list = Element::uniform_list("sessions", 10, 20.0, |index| {
        Element::button(format!("row:{index}"), format!("Task {index}"))
    })
    .list_expansion(
        3,
        Element::column(vec![
            Element::button("detail:first", "First").size(None, Some(20.0)),
            Element::button("detail:last", "Last").size(None, Some(20.0)),
        ]),
    );
    let tab = |tree: &mut UiTree, shift: bool| {
        tree.handle_event(UiEvent::Key { key: Key::Tab, shift, ctrl: false, alt: false });
        tree.layout(Size::new(120.0, 60.0));
    };
    let mut tree = UiTree::new(list);
    tree.layout(Size::new(120.0, 60.0));
    // Row 3 is under the viewport: reaching it brings what it opened too.
    assert!(tree.focus("row:2"));
    for expected in ["row:3", "detail:first", "detail:last", "row:4"] {
        tab(&mut tree, false);
        assert_eq!(tree.focused_id(), Some(expected));
    }
    // And back, from the row after it into its last control.
    for expected in ["detail:last", "detail:first", "row:3", "row:2"] {
        tab(&mut tree, true);
        assert_eq!(tree.focused_id(), Some(expected));
    }
}

#[test]
fn what_a_row_opens_can_open_on_the_spring_and_the_rows_follow_it() {
    let list = |open: bool| {
        Element::uniform_list("sessions", 100, 20.0, |index| {
            Element::button(format!("row:{index}"), format!("Task {index}"))
        })
        .list_expansion(
            0,
            Element::column(
                open.then(|| Element::button("detail:body", "Body").size(None, Some(60.0))).into_iter().collect(),
            )
            .animate_height("detail", true),
        )
    };
    let size = Size::new(160.0, 200.0);
    let next = |frame: &Frame| {
        frame.regions.iter().find(|region| region.id == "row:1").unwrap().rect.y
    };
    let start = Instant::now() - Duration::from_secs(10);
    let mut tree = UiTree::new(list(false));
    assert_eq!(next(&tree.layout_at(size, start)), 20.0);
    tree.set_root(list(true));
    tree.layout_at(size, start);
    let settle = Duration::from_millis(u64::from(crate::tokens().motion.move_ms) * 3 / 2);
    let midway = next(&tree.layout_at(size, start + settle / 2));
    assert!(20.0 < midway && midway < 80.0, "{midway}");
    let open = tree.layout_at(size, start + settle * 3);
    assert_eq!(next(&open), 80.0);
    assert_eq!(open.scroll_extents["sessions"], 100.0 * 20.0 + 60.0 - 200.0);
    assert!(!tree.is_animating());
}

#[test]
fn uniform_list_tab_focus_continues_across_viewport_boundary() {
    let list = Element::uniform_list("sessions", 10, 20.0, |index| {
        Element::clickable(
            format!("row:{index}"),
            Element::row(vec![
                Element::truncated_text(format!("Task {index}")).size(Some(58.0), Some(20.0)),
                Element::button(format!("menu:{index}"), "⋯").size(Some(20.0), Some(20.0)),
            ]),
        )
    });
    let mut tree = UiTree::new(list);
    tree.layout(Size::new(120.0, 60.0));
    assert!(tree.focus("menu:2"));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("row:3"));
    let frame = tree.layout(Size::new(120.0, 60.0));
    assert_eq!(tree.scroll_offset("sessions"), 20.0);
    assert!(frame.regions.iter().any(|hit| hit.id == "row:3"));

    assert!(tree.focus("row:1"));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: true,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("menu:0"));
    let frame = tree.layout(Size::new(120.0, 60.0));
    assert_eq!(tree.scroll_offset("sessions"), 0.0);
    assert!(frame.regions.iter().any(|hit| hit.id == "menu:0"));
}

#[test]
fn uniform_list_visible_rows_dispatch_scoped_aliases_and_drop_recycled_aliases() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let built = Arc::new(AtomicUsize::new(0));
    let rows = |scope: Option<&'static str>, built: Arc<AtomicUsize>| {
        Element::uniform_list("command_results", 1_000, 20.0, move |index| {
            built.fetch_add(1, Ordering::SeqCst);
            let row = Element::clickable(
                format!("command_result:{index}"),
                Element::text(format!("Result {index}")),
            )
            .size(None, Some(20.0));
            Element::column(vec![if let Some(scope) = scope {
                row.dispatch_as(format!("command_pick:{index}:{scope}"))
            } else {
                row
            }])
        })
    };
    let click = |tree: &mut UiTree, point| {
        tree.handle_event(UiEvent::PointerDown(point));
        tree.handle_event(UiEvent::PointerUp(point))
    };
    let size = Size::new(200.0, 60.0);
    let mut tree = UiTree::new(rows(Some("workspace-a/query-a"), built.clone()));
    tree.layout(size);
    assert_eq!(
        built.load(Ordering::SeqCst),
        3,
        "alias collection remains lazy"
    );
    assert_eq!(tree.action_aliases.len(), 3);
    assert_eq!(
        click(&mut tree, Point::new(20.0, 10.0)),
        vec![UiAction::Clicked {
            id: "command_pick:0:workspace-a/query-a".into(),
        }]
    );
    tree.scroll_to_index("command_results", 999);
    tree.layout(size);
    assert!(!tree.action_aliases.contains_key("command_result:0"));
    assert_eq!(tree.action_aliases.len(), 3);
    assert_eq!(
        click(&mut tree, Point::new(20.0, 50.0)),
        vec![UiAction::Clicked {
            id: "command_pick:999:workspace-a/query-a".into(),
        }]
    );
    tree.set_root(rows(Some("workspace-b/query-b"), built.clone()));
    tree.scroll_to_start("command_results");
    tree.layout(size);
    assert!(!tree.action_aliases.contains_key("command_result:999"));
    assert!(tree
        .action_aliases
        .values()
        .all(|action| action.ends_with("workspace-b/query-b")));
    assert!(tree.focus("command_result:0"));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: false,
            alt: false
        }),
        vec![UiAction::Clicked {
            id: "command_pick:0:workspace-b/query-b".into(),
        }]
    );
    tree.set_root(rows(None, built));
    tree.layout(size);
    assert!(
        tree.action_aliases.is_empty(),
        "removing dispatch_as removes the previous alias too"
    );
    assert_eq!(
        click(&mut tree, Point::new(20.0, 10.0)),
        vec![UiAction::Clicked {
            id: "command_result:0".into(),
        }]
    );
}

#[test]
fn virtual_alias_refresh_preserves_focus_and_cancels_only_retargeted_pointer_presses() {
    let rows = |scope: &'static str| {
        Element::uniform_list("results", 10, 20.0, move |index| {
            Element::clickable(format!("result:{index}"), Element::text("Result"))
                .size(None, Some(20.0))
                .dispatch_as(format!("pick:{scope}:{index}"))
        })
    };
    let size = Size::new(200.0, 60.0);
    let point = Point::new(20.0, 10.0);
    let mut tree = UiTree::new(rows("old"));
    tree.layout(size);
    tree.handle_event(UiEvent::PointerDown(point));
    tree.set_root(rows("new"));
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("result:0"));
    assert!(
        tree.handle_event(UiEvent::PointerUp(point)).is_empty(),
        "held press must not activate a replacement result"
    );
    tree.handle_event(UiEvent::PointerDown(point));
    tree.set_root(rows("new"));
    tree.layout(size);
    assert_eq!(tree.focused_id(), Some("result:0"));
    assert_eq!(
        tree.handle_event(UiEvent::PointerUp(point)),
        vec![UiAction::Clicked {
            id: "pick:new:0".into(),
        }],
        "unchanged rebuilds keep their original pointer capture"
    );
    tree.handle_event(UiEvent::PointerDown(point));
    tree.scroll_to_index("results", 9);
    tree.layout(size);
    assert!(
        tree.handle_event(UiEvent::PointerUp(point)).is_empty(),
        "scrolling away cannot click another row at the same position"
    );
}

#[test]
fn empty_uniform_list_has_no_rows_or_scrollbar() {
    let mut tree = UiTree::new(Element::uniform_list("empty", 0, 20.0, |_| {
        panic!("empty list must never build a row")
    }));
    tree.scroll_to_index("empty", 42);
    let frame = tree.layout(Size::new(100.0, 60.0));
    assert!(!frame.scrollbars.contains_key("empty"));
    assert_eq!(frame.scroll_extents["empty"], 0.0);
    assert!(tree.focus_order.is_empty());
}

#[test]
fn selected_text_can_be_replaced() {
    let mut tree = UiTree::new(Element::text_input("input", "hello", ""));
    tree.layout(Size::new(200.0, 40.0));
    tree.handle_event(UiEvent::PointerDown(Point::new(48.0, 10.0)));
    tree.handle_event(UiEvent::Key {
        key: Key::A,
        shift: false,
        ctrl: true,
        alt: false,
    });
    tree.handle_event(UiEvent::Text("rust".into()));
    assert_eq!(tree.input_value("input"), Some("rust"));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::Enter,
            shift: false,
            ctrl: false,
            alt: false
        }),
        vec![UiAction::Submitted {
            id: "input".into(),
            value: "rust".into()
        }]
    );
}

#[test]
fn tab_navigation_wraps_and_survives_tree_rebuild() {
    let controls = || {
        Element::row(vec![
            Element::button("first", "First"),
            Element::text_input("prompt", "", "Prompt"),
            Element::button("last", "Last"),
        ])
    };
    let mut tree = UiTree::new(controls());
    tree.layout(Size::new(400.0, 40.0));
    let tab = |shift| UiEvent::Key {
        key: Key::Tab,
        shift,
        ctrl: false,
        alt: false,
    };
    tree.handle_event(tab(false));
    assert_eq!(tree.focused_id(), Some("first"));
    tree.handle_event(tab(false));
    assert_eq!(tree.focused_id(), Some("prompt"));

    tree.set_root(controls());
    tree.layout(Size::new(400.0, 40.0));
    assert_eq!(tree.focused_id(), Some("prompt"));
    tree.handle_event(tab(false));
    assert_eq!(tree.focused_id(), Some("last"));
    tree.handle_event(tab(false));
    assert_eq!(tree.focused_id(), Some("first"));
    tree.handle_event(tab(true));
    assert_eq!(tree.focused_id(), Some("last"));
}

#[test]
fn enter_and_space_activate_focused_button() {
    let mut tree = UiTree::new(Element::button("send", "Send"));
    tree.layout(Size::new(100.0, 40.0));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    for key in [Key::Enter, Key::Space] {
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked { id: "send".into() }]
        );
    }
    let frame = tree.layout(Size::new(100.0, 40.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::StrokeRect { color, width: 2.0, .. }
            if *color == Color::rgb(170, 170, 170)
    )));
}

#[test]
fn removing_focused_control_clears_focus() {
    let mut tree = UiTree::new(Element::button("old", "Old"));
    tree.layout(Size::new(100.0, 40.0));
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
        alt: false,
    });
    tree.set_root(Element::button("new", "New"));
    tree.layout(Size::new(100.0, 40.0));
    assert_eq!(tree.focused_id(), None);
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: true,
        ctrl: false,
        alt: false,
    });
    assert_eq!(tree.focused_id(), Some("new"));
}

#[test]
fn decorative_top_fade_keeps_legacy_panel_behavior() {
    let content = || {
        Element::panel(Element::scroll(
            "scroll",
            Element::button("row", "A scrollable row").size(None, Some(36.0)),
        ))
        .size(None, Some(60.0))
    };
    let plain = UiTree::new(content()).layout(Size::new(220.0, 60.0));
    let faded = UiTree::new(content().top_fade(Color::rgb(20, 20, 20), 24.0))
        .layout(Size::new(220.0, 60.0));
    assert_eq!(plain.regions, faded.regions);
    let fade_index = faded
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. }))
        .unwrap();
    assert!(faded.commands[..fade_index].iter().any(|command| matches!(
        command,
        DrawCommand::Text { text, .. } if text == "A scrollable row"
    )));
    assert!(matches!(
        faded.commands[fade_index],
        DrawCommand::FillVerticalGradient {
            rect: Rect {
                y: 0.0,
                height: 24.0,
                ..
            },
            top: Color { a: 255, .. },
            bottom: Color { a: 0, .. },
        }
    ));
}

#[test]
fn a_top_background_fade_is_painted_behind_the_children_from_the_top_edge_down() {
    let tint = Color::rgb(90, 40, 120);
    let content = || Element::column(vec![Element::text("Título")]).size(None, Some(200.0));
    let plain = UiTree::new(content()).layout(Size::new(220.0, 200.0));
    let washed = UiTree::new(content().top_background_fade(tint, 500.0)).layout(Size::new(220.0, 200.0));
    assert_eq!(plain.regions, washed.regions);
    let wash = washed.commands.iter().position(|command| matches!(command, DrawCommand::FillVerticalGradient { .. })).unwrap();
    let text = washed.commands.iter().position(|command| matches!(command, DrawCommand::Text { .. })).unwrap();
    assert!(wash < text, "behind what the element holds");
    assert!(matches!(
        washed.commands[wash],
        // No taller than the element it is behind.
        DrawCommand::FillVerticalGradient { rect: Rect { y: 0.0, height: 200.0, .. }, top, bottom: Color { a: 0, .. } } if top == tint
    ));
}

#[test]
fn spinner_phase_moves_and_reduced_motion_is_static() {
    let start = Instant::now();
    let area = Rect::new(0.0, 0.0, 32.0, 32.0);
    let ink = Color::rgb(220, 220, 220);
    let colors = |at, reduced| {
        let mut commands = Vec::new();
        paint_spinner(
            &mut commands,
            area,
            ink,
            SpinnerVariant::Circle,
            start,
            at,
            reduced,
        );
        commands
            .into_iter()
            .filter_map(|command| match command {
                DrawCommand::Spinner { angle, .. } => Some(angle),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_ne!(
        colors(start, false),
        colors(start + Duration::from_millis(225), false)
    );
    assert_eq!(
        colors(start, true),
        colors(start + Duration::from_secs(1), true)
    );
}

#[test]
fn rounded_dashed_border_paints_all_four_corners_inside_bounds() {
    let area = Rect::new(10.0, 20.0, 320.0, 240.0);
    let mut commands = Vec::new();
    paint_dashed_corners(&mut commands, area, 14.0, 1.0, Color::WHITE);
    let mut corners = [false; 4];
    for command in commands {
        let DrawCommand::FillRect { rect, .. } = command else {
            panic!("corner must use a rounded fill");
        };
        assert!(rect.x >= area.x && rect.y >= area.y);
        assert!(rect.x + rect.width <= area.x + area.width);
        assert!(rect.y + rect.height <= area.y + area.height);
        let left = rect.x < area.x + 14.0;
        let top = rect.y < area.y + 14.0;
        corners[match (left, top) {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        }] = true;
    }
    assert_eq!(corners, [true; 4]);
}
