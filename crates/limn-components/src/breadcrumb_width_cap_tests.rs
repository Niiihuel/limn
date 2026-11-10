//! A row capped in width keeps a breadcrumb's long label inside the cap and
//! what follows beside it. This test exercises the engine's intrinsic
//! measurement through `Breadcrumb`.
use crate::*;

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
