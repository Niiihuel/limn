// Staged by the crate split, not yet a module of this crate: these tests
// build their trees with `DropdownList`, `DropdownOption`, `DropdownMenu` and
// `Theme`, which live here now.
//
// Came from `crates/limn-core/src/components.rs`, its `mod tests`, unchanged:
// - `DropdownList` and `DropdownOption`, which moved to `dropdown_list.rs`:
//   `a_dropdown_list_says_which_option_is_the_chosen_one`,
//   `dropdown_uses_shared_popover_and_keeps_item_ids`,
//   `dropdown_list_sizing_preserves_compact_rows_and_keyboard_scroll`,
//   `selected_dropdown_option_keeps_selected_hover_color`,
//   `dropdown_rows_keep_one_hit_target_and_stable_icon_columns`,
//   `dropdown_keyboard_stays_in_menu_skips_disabled_and_returns_to_trigger`,
//   `long_dropdown_reveals_options_reached_by_keyboard`,
//   `clicking_outside_dropdown_returns_focus_before_dismiss_action`,
//   `disabled_options_are_visible_without_click_regions`.
// - `DropdownMenu`: `dropdown_keeps_its_compact_anchor_geometry_when_open_in_a_tall_row`.
// - `Theme` (and `chip::contrast`, which is the engine's `contrast` re-exported):
//   `the_accent_variant_keeps_a_light_thumb_on_the_link_color_or_the_ink_and_on_the_muted_color_off`,
//   with the helper `switch_paint`, copied: the engine's other `Switch`
//   tests use it too.
// Their `use super::*` (the engine's `components` module) is replaced by
// `use crate::*`.
//
// Engine items they use that `limn-core` does not make public:
// - `lighter(Color, u8)`: a `pub(crate) const fn` in the engine's
//   `components.rs`, called by `selected_dropdown_option_keeps_selected_hover_color`.
// Otherwise they read public fields (`Frame::regions`, `Frame::commands`,
// `Frame::access`, `Element::kind`, `Element::style`) and public `UiTree` methods.
mod tests {
    use crate::*;

    /// A switch at its own size, settled: its track's rectangle and color,
    /// then its thumb's.
    fn switch_paint(switch: Switch) -> ((crate::Rect, Color), (crate::Rect, Color)) {
        let mut tree = UiTree::new(Element::row(vec![switch.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 40.0));
        let fills: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        // The track is the pill the others stand in; the thumb is the circle inside it.
        let [(track, track_color, _), (thumb, thumb_color, thumb_radius)] = fills.as_slice() else {
            panic!("a track and a thumb, got {fills:?}");
        };
        assert_eq!(*thumb_radius, thumb.width / 2.0);
        ((*track, *track_color), (*thumb, *thumb_color))
    }

    #[test]
    fn a_dropdown_list_says_which_option_is_the_chosen_one() {
        let palette = ComponentPalette::neutral_dark();
        let list = DropdownList::new(
            Button::new("open", "Shell", palette).render(),
            vec![
                DropdownOption::new("bash", "Bash").selected(true),
                DropdownOption::new("zsh", "Zsh"),
                DropdownOption::new("off", "Off").disabled(),
            ],
            true,
            "dismiss",
            palette,
        );
        let mut tree = UiTree::new(Element::column(vec![list.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let nodes = tree.layout(Size::new(480.0, 320.0)).access;
        let said = |id: &str| {
            let node = nodes.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id} among {nodes:?}"));
            (node.role, node.name.as_str(), node.selected, node.disabled)
        };
        assert_eq!(said("bash"), (Role::MenuItem, "Bash", Some(true), false));
        assert_eq!(said("zsh"), (Role::MenuItem, "Zsh", Some(false), false));
        assert_eq!(said("off"), (Role::MenuItem, "Off", Some(false), true));
    }

    #[test]
    fn dropdown_keeps_its_compact_anchor_geometry_when_open_in_a_tall_row() {
        let palette = ComponentPalette::neutral_dark();
        let build = |open| {
            let anchor = Button::new("add_origin", "Agregar origen", palette)
                .icon(Icon::Add)
                .size(ControlSize::Compact)
                .render();
            let menu = DropdownMenu::new(
                anchor,
                vec![
                    crate::DropdownMenuItem::new("origin:local", "Carpeta local").into(),
                    crate::DropdownMenuItem::new("origin:github", "Repositorio GitHub").into(),
                ],
                open,
                "dismiss_origin",
                palette,
            )
            .width(200.0)
            .render();
            Element::row(vec![Element::spacer().flex(1.0), menu]).size(None, Some(50.0))
        };
        let viewport = Size::new(420.0, 260.0);
        let mut tree = UiTree::new(build(false));
        tree.set_reduced_motion(true);
        let closed = tree.layout(viewport);
        let anchor = closed
            .regions
            .iter()
            .find(|region| region.id == "add_origin")
            .expect("closed anchor")
            .rect;
        assert_eq!(anchor.height, 28.0);
        assert!(tree.focus("add_origin"));
        tree.set_root(build(true));
        let open = tree.layout(viewport);
        assert_eq!(
            open.regions
                .iter()
                .find(|region| region.id == "add_origin")
                .unwrap()
                .rect,
            anchor
        );
        for id in ["origin:local", "origin:github"] {
            assert!(open.regions.iter().any(|region| region.id == id));
        }
        assert!(tree.focus("origin:local"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        assert_eq!(
            tree.handle_event(key(crate::Key::Enter)),
            vec![UiAction::Clicked {
                id: "origin:local".into()
            }]
        );
        assert_eq!(
            tree.handle_event(key(crate::Key::Escape)),
            vec![UiAction::Clicked {
                id: "dismiss_origin".into()
            }]
        );
        tree.set_root(build(false));
        let closed_again = tree.layout(viewport);
        assert_eq!(tree.focused_id(), Some("add_origin"));
        assert_eq!(
            closed_again
                .regions
                .iter()
                .find(|region| region.id == "add_origin")
                .unwrap()
                .rect,
            anchor
        );
    }

    #[test]
    fn the_accent_variant_keeps_a_light_thumb_on_the_link_color_or_the_ink_and_on_the_muted_color_off() {
        let dark = ComponentPalette::neutral_dark();
        let light = ComponentPalette {
            surface: Color::rgb(255, 255, 255),
            raised: Color::rgb(250, 250, 250),
            hover: Color::rgb(235, 235, 235),
            selected: Color::rgb(220, 220, 220),
            border: Color::rgb(205, 205, 205),
            text: Color::rgb(32, 32, 32),
            muted: Color::rgb(100, 100, 100),
            shadow: Color::rgba(0, 0, 0, 40),
        };
        let ink = Color::rgb(88, 101, 242);
        // A theme's text and muted color are translucent; its thumb is the foreground at full strength.
        let theme = crate::Theme::DARK.palette();
        // The thumb is the palette's lightest color: its text on a dark one, its surface on a light one.
        for (palette, thumb) in [(dark, dark.text), (light, light.surface), (theme, Color::rgb(255, 255, 255))] {
            let accent = |checked| Switch::new("s", checked, palette).variant(SwitchVariant::Accent);
            let paint = |switch: Switch| {
                let ((_, track), (_, thumb)) = switch_paint(switch);
                (track, thumb)
            };
            // A pale ink, such as the gallery theme's accent, is darkened like the link color.
            let pale = Color::rgb(25, 170, 216);
            for switch in [accent(true), accent(false), accent(true).ink(pale)] {
                let (track, drawn) = paint(switch);
                assert_eq!(drawn, thumb);
                let contrast = crate::chip::contrast(thumb, track);
                assert!(contrast >= 3.0, "{palette:?}: {track:?} at {contrast}");
            }
            for checked in [true, false] {
                assert_eq!(paint(accent(checked).disabled()), paint(accent(checked)), "disabled, it keeps its paint");
            }
            assert_eq!(paint(accent(true).ink(ink)), (ink, thumb), "an ink that reads is kept");
            // An off ink is the host's, kept as given, and leaves the switch on as it was.
            let pale_gray = Color::rgb(206, 206, 206);
            assert_eq!(paint(accent(false).off_ink(pale_gray)), (pale_gray, thumb));
            assert_eq!(paint(accent(true).off_ink(pale_gray)), paint(accent(true)));
            assert_eq!(paint(accent(false).ink(ink)), paint(accent(false)));
        }
        // Where the link and muted colors already read, they are the track.
        let paint = |checked| switch_paint(Switch::new("s", checked, light).variant(SwitchVariant::Accent)).0 .1;
        assert_eq!((paint(true), paint(false)), (light.link(), light.muted));
        // The default variant takes the ink for its track and keeps its own thumb.
        let paint = |checked| {
            let ((_, track), (_, thumb)) = switch_paint(Switch::new("s", checked, dark).ink(ink));
            (track, thumb)
        };
        assert_eq!((paint(true), paint(false)), ((ink, dark.surface), (dark.selected, dark.text)));
    }

    #[test]
    fn dropdown_uses_shared_popover_and_keeps_item_ids() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("backend:codex", "Codex").selected(true),
                DropdownOption::new("backend:claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        let frame = tree.layout(Size::new(400.0, 300.0));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:codex"));
        assert!(frame
            .regions
            .iter()
            .any(|region| region.id == "backend:claude"));
        assert_eq!(frame.hit_test(Point::new(390.0, 290.0)), Some("dismiss"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Escape,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn dropdown_list_sizing_preserves_compact_rows_and_keyboard_scroll() {
        let sizing = crate::SelectorSizing {
            max_visible_items: 2,
            ..crate::SelectorSizing::COMPACT
        };
        let dropdown = DropdownList::new(
            Element::button("trigger", "Choice").size(Some(160.0), Some(36.0)),
            (0..5)
                .map(|index| {
                    DropdownOption::new(format!("choice:{index}"), format!("Choice {index}"))
                        .icon(Icon::Folder)
                        .selected(index == 0)
                })
                .collect(),
            true,
            "dismiss",
            ComponentPalette::neutral_dark(),
        )
        .sizing(sizing)
        .width(220.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(360.0, 280.0));
        let surface = frame
            .regions
            .iter()
            .find(|region| region.id == "popover_surface:dismiss")
            .unwrap();
        assert_eq!(surface.rect.width, 220.0);
        let first = frame
            .regions
            .iter()
            .find(|region| region.id == "choice:0")
            .unwrap();
        assert_eq!(first.rect.height, sizing.row_height);
        assert!(frame.commands.iter().any(|command| matches!(command,
            DrawCommand::Text { text, font_size, .. }
                if text == "Choice 0" && *font_size == sizing.label_size
        )));
        assert!(tree.focus("choice:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::End,
            shift: false,
            ctrl: false,
            alt: false,
        });
        let final_frame = tree.layout(Size::new(360.0, 280.0));
        assert_eq!(tree.focused_id(), Some("choice:4"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        assert!(final_frame
            .regions
            .iter()
            .any(|region| region.id == "choice:4"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Enter,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked {
                id: "choice:4".into()
            }]
        );
    }

    #[test]
    fn selected_dropdown_option_keeps_selected_hover_color() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agent"),
            vec![
                DropdownOption::new("codex", "Codex").selected(true),
                DropdownOption::new("claude", "Claude"),
            ],
            true,
            "dismiss",
            palette,
        )
        .render();
        let ElementKind::Popover { floating, .. } = dropdown.kind else {
            panic!("open dropdown must have a popover");
        };
        let ElementKind::Panel(content) = floating.kind else {
            panic!("popover must contain a panel");
        };
        let ElementKind::Column(options) = content.kind else {
            panic!("short dropdown must contain an option column");
        };
        assert_eq!(
            options[0].style.hover_background,
            Some(lighter(palette.selected, 8))
        );
        assert_eq!(options[1].style.hover_background, Some(palette.hover));
    }

    #[test]
    fn dropdown_rows_keep_one_hit_target_and_stable_icon_columns() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![
                DropdownOption::new("codex", "Codex")
                    .icon(Icon::Chat)
                    .selected(true),
                DropdownOption::new("native", "Nativo"),
                DropdownOption::new("offline", "No instalado").disabled(),
            ],
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(400.0, 300.0));
        let rows: Vec<_> = frame
            .regions
            .iter()
            .filter(|region| region.id == "codex" || region.id == "native")
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].rect.width, rows[1].rect.width);
        assert_eq!(rows[0].rect.height, 36.0);
        assert_eq!(rows[1].rect.height, 36.0);
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
        assert_eq!(
            frame
                .commands
                .iter()
                .filter(|command| matches!(
                    command,
                    DrawCommand::Icon {
                        icon: Icon::Check,
                        ..
                    }
                ))
                .count(),
            1
        );
        tree.handle_event(UiEvent::PointerMove(Point::new(
            rows[1].rect.x + 4.0,
            rows[1].rect.y + 4.0,
        )));
        let hovered = tree.layout(Size::new(400.0, 300.0));
        assert!(hovered.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, color, .. }
                if *rect == rows[1].rect && *color == palette.hover
        )));
    }

    #[test]
    fn dropdown_keyboard_stays_in_menu_skips_disabled_and_returns_to_trigger() {
        let palette = ComponentPalette::neutral_dark();
        let make_dropdown = || {
            DropdownList::new(
                Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
                vec![
                    DropdownOption::new("codex", "Codex"),
                    DropdownOption::new("offline", "No instalado").disabled(),
                    DropdownOption::new("claude", "Claude"),
                ],
                true,
                "dismiss",
                palette,
            )
            .width(170.0)
            .render()
        };
        let key = |key, shift| UiEvent::Key {
            key,
            shift,
            ctrl: false,
            alt: false,
        };
        let mut tree = UiTree::new(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("trigger"));

        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::ArrowDown, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::End, false));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Home, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        tree.handle_event(key(crate::Key::Tab, true));
        assert_eq!(tree.focused_id(), Some("claude"));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Enter, false)),
            vec![UiAction::Clicked { id: "codex".into() }]
        );

        tree.set_root(Element::button("trigger", "Agente"));
        tree.layout(Size::new(400.0, 300.0));
        assert_eq!(tree.focused_id(), Some("trigger"));

        tree.set_root(make_dropdown());
        tree.layout(Size::new(400.0, 300.0));
        tree.handle_event(key(crate::Key::Tab, false));
        assert_eq!(tree.focused_id(), Some("codex"));
        assert_eq!(
            tree.handle_event(key(crate::Key::Escape, false)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
        assert_eq!(tree.focused_id(), Some("trigger"));
    }

    #[test]
    fn long_dropdown_reveals_options_reached_by_keyboard() {
        let palette = ComponentPalette::neutral_dark();
        let options = (0..12)
            .map(|index| {
                let option =
                    DropdownOption::new(format!("option:{index}"), format!("Opción {index}"));
                if index == 5 {
                    option.disabled()
                } else {
                    option
                }
            })
            .collect();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            options,
            true,
            "dismiss",
            palette,
        )
        .width(180.0)
        .render();
        let mut tree = UiTree::new(dropdown);
        let viewport = Size::new(400.0, 400.0);
        tree.layout(viewport);
        assert!(tree.focus("trigger"));
        let key = |key| UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        };
        for _ in 0..9 {
            tree.handle_event(key(crate::Key::ArrowDown));
        }
        assert_eq!(tree.focused_id(), Some("option:9"));
        assert!(tree.scroll_offset("dismiss:options") > 0.0);
        let scrolled = tree.layout(viewport);
        assert!(scrolled
            .regions
            .iter()
            .any(|region| region.id == "option:9"));
        tree.handle_event(key(crate::Key::End));
        assert_eq!(tree.focused_id(), Some("option:11"));
        let end = tree.layout(viewport);
        assert!(end.regions.iter().any(|region| region.id == "option:11"));
        tree.handle_event(key(crate::Key::Home));
        assert_eq!(tree.focused_id(), Some("option:0"));
        assert_eq!(tree.scroll_offset("dismiss:options"), 0.0);
        let home = tree.layout(viewport);
        assert!(home.regions.iter().any(|region| region.id == "option:0"));
        tree.handle_event(UiEvent::Key {
            key: crate::Key::Tab,
            shift: true,
            ctrl: false,
            alt: false,
        });
        assert_eq!(tree.focused_id(), Some("option:11"));
        let wrapped = tree.layout(viewport);
        assert!(wrapped
            .regions
            .iter()
            .any(|region| region.id == "option:11"));
    }

    #[test]
    fn clicking_outside_dropdown_returns_focus_before_dismiss_action() {
        let palette = ComponentPalette::neutral_dark();
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente").size(Some(100.0), Some(30.0)),
            vec![DropdownOption::new("codex", "Codex")],
            true,
            "dismiss",
            palette,
        )
        .render();
        let mut tree = UiTree::new(dropdown);
        tree.layout(Size::new(400.0, 300.0));
        assert!(tree.focus("codex"));
        let outside = Point::new(390.0, 290.0);
        tree.handle_event(UiEvent::PointerDown(outside));
        assert_eq!(tree.focused_id(), Some("trigger"));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(outside)),
            vec![UiAction::Clicked {
                id: "dismiss".into()
            }]
        );
    }

    #[test]
    fn disabled_options_are_visible_without_click_regions() {
        let palette = ComponentPalette::neutral_dark();
        let tabs = Tabs::new(
            vec![Tab::new("soon", "Próximamente", 140.0).disabled()],
            palette,
        )
        .render();
        let frame = UiTree::new(tabs).layout(Size::new(180.0, 40.0));
        assert!(!frame.regions.iter().any(|region| region.id == "soon"));
        let dropdown = DropdownList::new(
            Element::button("trigger", "Agente"),
            vec![DropdownOption::new("offline", "No instalado").disabled()],
            true,
            "dismiss",
            palette,
        )
        .render();
        let frame = UiTree::new(dropdown).layout(Size::new(300.0, 200.0));
        assert!(!frame.regions.iter().any(|region| region.id == "offline"));
    }
}
