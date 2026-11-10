//! A menu enters as one surface. This test exercises the engine's popover
//! entrance and hover fills through `DropdownMenu`.
use crate::*;
use web_time::{Duration, Instant};

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
