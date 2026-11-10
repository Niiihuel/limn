//! Bounded passive corner finishing, independent of retained child count.
use super::{paint_corner_rect, CornerRadii, DrawCommand, Rect, Style};

pub(super) fn finish_rounded_surface(
    commands: &mut Vec<DrawCommand>,
    rect: Rect,
    clip: Option<Rect>,
    style: Style,
) {
    let Some(color) = style.corner_mask else {
        return;
    };
    if rect.width <= 0.0
        || rect.height <= 0.0
        || ![rect.x, rect.y, rect.width, rect.height]
            .into_iter()
            .all(f32::is_finite)
    {
        return;
    }
    let Some(visible) = clip.map_or(Some(rect), |clip| clip.intersection(rect)) else {
        return;
    };
    let limit = rect.width.min(rect.height) / 2.0;
    let clean = |radius: f32| {
        if radius.is_finite() {
            radius.clamp(0.0, limit)
        } else {
            0.0
        }
    };
    let radii = style
        .corner_radii
        .unwrap_or(CornerRadii::same(style.radius));
    let radii = CornerRadii {
        top_left: clean(radii.top_left),
        top_right: clean(radii.top_right),
        bottom_right: clean(radii.bottom_right),
        bottom_left: clean(radii.bottom_left),
    };
    commands.push(DrawCommand::PushClip(visible));
    if color.a > 0
        && [
            radii.top_left,
            radii.top_right,
            radii.bottom_right,
            radii.bottom_left,
        ]
        .into_iter()
        .any(|radius| radius > 0.0)
    {
        commands.push(DrawCommand::CornerMask { rect, color, radii });
    }
    if let Some(border) = style.border.filter(|_| !style.dashed_border) {
        paint_corner_rect(
            commands,
            rect,
            border.color,
            Some(border.width),
            0.0,
            Some(radii),
        );
    }
    commands.push(DrawCommand::PopClip);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, Element, Point, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn mask_finishes_after_opaque_content_without_changing_hits_or_default_paint() {
        let child_color = Color::rgb(90, 70, 50);
        let mask_color = Color::rgb(30, 31, 32);
        let root = || {
            Element::panel(Element::button("child", "Document").background(child_color))
                .border(Color::WHITE, 1.0)
                .radius(12.0)
        };
        let baseline = UiTree::new(root()).layout(Size::new(240.0, 100.0));
        let mut tree = UiTree::new(root().corner_mask(mask_color).clip_children());
        let frame = tree.layout(Size::new(240.0, 100.0));
        assert_eq!(frame.regions, baseline.regions);
        let child = frame
            .commands
            .iter()
            .position(|command| {
                matches!(command,
            DrawCommand::FillRect { color, .. } if *color == child_color)
            })
            .unwrap();
        let mask = frame
            .commands
            .iter()
            .position(|command| {
                matches!(command,
            DrawCommand::CornerMask { color, .. } if *color == mask_color)
            })
            .unwrap();
        let border = frame
            .commands
            .iter()
            .position(|command| {
                matches!(
                    command,
                    DrawCommand::StrokeRect {
                        color: Color::WHITE,
                        ..
                    }
                )
            })
            .unwrap();
        assert!(child < mask && mask < border);
        assert!(!baseline.commands.iter().any(|command| matches!(command,
            DrawCommand::CornerMask { color, .. } if *color == mask_color)));
        tree.handle_event(UiEvent::PointerDown(Point::new(120.0, 50.0)));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(Point::new(120.0, 50.0))),
            vec![UiAction::Clicked { id: "child".into() }]
        );
    }

    #[test]
    fn mask_is_clamped_clipped_and_uses_the_existing_overlay_opacity_transform() {
        let rect = Rect::new(10.0, 20.0, 30.0, 8.0);
        let clip = Rect::new(12.0, 20.0, 20.0, 8.0);
        let ink = Color::rgba(20, 30, 40, 128);
        let style = Element::spacer().radius(200.0).corner_mask(ink).style;
        let mut commands = Vec::new();
        finish_rounded_surface(&mut commands, rect, Some(clip), style);
        assert!(matches!(commands.first(), Some(DrawCommand::PushClip(actual)) if *actual == clip));
        assert_eq!(
            commands.len(),
            3,
            "one continuous mask plus clip boundaries"
        );
        for command in &mut commands {
            if let DrawCommand::CornerMask {
                rect: painted,
                color,
                radii,
            } = command
            {
                assert_eq!(*color, ink);
                assert_eq!(*painted, rect);
                assert_eq!(*radii, CornerRadii::same(4.0));
            }
            super::super::animate_popover_command(command, 0.5, 0.0, 1.0, Point::new(0.0, 0.0));
            if let DrawCommand::CornerMask { color, .. } = command {
                assert_eq!(color.a, 64);
            }
        }
        commands.clear();
        finish_rounded_surface(
            &mut commands,
            rect,
            Some(Rect::new(100.0, 100.0, 1.0, 1.0)),
            style,
        );
        assert!(commands.is_empty());
    }
}
