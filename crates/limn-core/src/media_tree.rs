//! How the tree places pictures and what moves on its own: raster images,
//! surfaces, icons, drawings, progress rings, records, spinners and scribbles.

use super::*;

impl LayoutBuilder<'_> {
    pub(super) fn place_raster_image(&mut self, element: &Element, content: Rect, style: Style) {
        let ElementKind::RasterImage(image) = &element.kind else { unreachable!() };
        self.frame.commands.push(DrawCommand::RasterImage {
            rect: content,
            image: image.clone(),
            opacity: 1.0,
            radius: style.radius,
        });
    }

    pub(super) fn place_surface(&mut self, element: &Element, content: Rect, style: Style) {
        let ElementKind::Surface {
            id,
            label,
            fit,
            mirrored,
        } = &element.kind else { unreachable!() };
        self.frame.commands.push(DrawCommand::Surface {
            rect: content,
            id: id.clone(),
            label: label.clone(),
            opacity: 1.0,
            fit: *fit,
            radius: style.radius,
            mirrored: *mirrored,
        });
    }

    pub(super) fn place_icon(&mut self, element: &Element, content: Rect, style: Style) {
        let ElementKind::Icon(icon) = &element.kind else { unreachable!() };
        let side = style
            .icon_side
            .unwrap_or(ICON_SIZE)
            .min(content.width)
            .min(content.height)
            .max(0.0);
        let icon_rect = Rect::new(
            content.x + (content.width - side) / 2.0,
            content.y + (content.height - side) / 2.0,
            side,
            side,
        );
        self.draw_icon(*icon, icon_rect, style);
    }

    pub(super) fn place_drawing(&mut self, element: &Element, content: Rect) {
        let ElementKind::Drawing(drawing) = &element.kind else { unreachable!() };
        self.frame.commands.push(DrawCommand::PushClip(content));
        // The drawing's own clips stay balanced, so none outlives it.
        let mut open = 0;
        for command in drawing.draw(Size::new(content.width, content.height)) {
            match command {
                DrawCommand::PushClip(_) => open += 1,
                DrawCommand::PopClip if open == 0 => continue,
                DrawCommand::PopClip => open -= 1,
                _ => {}
            }
            self.frame.commands.push(command.moved(content.x, content.y, 1.0));
        }
        self.frame.commands.extend(std::iter::repeat_n(DrawCommand::PopClip, open + 1));
    }

    pub(super) fn place_progress_ring(&mut self, element: &Element, content: Rect) {
        let ElementKind::ProgressRing {
            fraction,
            track,
            fill,
        } = &element.kind else { unreachable!() };
        self.frame.commands.push(DrawCommand::ProgressRing {
            rect: content,
            fraction: *fraction,
            track: *track,
            fill: *fill,
        });
    }

    pub(super) fn place_record(&mut self, element: &Element, rect: Rect, style: Style) {
        let ElementKind::Record { id, image, out, spinning, peek } = &element.kind else { unreachable!() };
        let now = self.now;
        let peeked = peek.as_deref().is_some_and(|control| self.hovered == Some(control));
        let target = if *out || peeked { 1.0 } else { 0.0 };
        let mut states = states_mut!(self);
        let motion = states.touch(id, now, || RecordMotion {
            out: ScalarTransition::new(target, now),
            turned: 0.0,
            spinning_since: None,
        });
        if motion.out.target() != target {
            let travel = Duration::from_millis(u64::from(crate::tokens().motion.move_ms));
            motion.out.retarget_with_motion(target, now, travel, self.reduced_motion);
        }
        motion.spin(*spinning && !self.reduced_motion, now);
        if rect.width > 0.0 && rect.height > 0.0 {
            self.frame.commands.push(DrawCommand::Record {
                rect,
                image: image.clone(),
                fill: style.background.unwrap_or(Color::TRANSPARENT),
                out: motion.out.value(now),
                angle: motion.angle(now),
                radius: style.radius,
                opacity: 1.0,
            });
        }
    }

    pub(super) fn place_spinner(&mut self, element: &Element, rect: Rect) {
        let ElementKind::Spinner { id, color, variant } = &element.kind else { unreachable!() };
        // Under reduced motion a spinner starts on every frame: it does not turn.
        let started = if self.reduced_motion {
            self.now
        } else {
            states_mut!(self).touch(id, self.now, || SpinnerStart(self.now)).0
        };
        paint_spinner(
            &mut self.frame.commands,
            rect,
            *color,
            *variant,
            started,
            self.now,
            self.reduced_motion,
        );
    }

    pub(super) fn place_scribble(&mut self, element: &Element, rect: Rect) {
        let ElementKind::Scribble { id, color, stroke, drawing } = &element.kind else { unreachable!() };
        let started = states_mut!(self).touch(id, self.now, || SpinnerStart(self.now)).0;
        let (commands, moving) = crate::scribble::paint(
            rect,
            *color,
            *stroke,
            drawing.as_deref(),
            started,
            self.now,
            self.reduced_motion,
        );
        // A drawing written once stops asking for frames when its pen has gone; its
        // start is still kept, so it is not written again.
        if !moving {
            self.settled_spinners.insert(id.clone());
        }
        self.frame.commands.extend(commands);
    }
}
