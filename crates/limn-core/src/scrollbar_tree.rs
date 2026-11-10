use super::*;

#[cfg(test)]
#[path = "scrollbar_look_tests.rs"]
mod scrollbar_look_tests;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ScrollbarGeometry {
    pub(super) rail: Rect,
    pub(super) track: Rect,
    pub(super) thumb: Rect,
    pub(super) up_button: Option<Rect>,
    pub(super) down_button: Option<Rect>,
    pub(super) max_offset: f32,
    pub(super) placement: crate::ScrollbarPlacement,
}

impl ScrollbarGeometry {
    pub(super) fn new(
        viewport: Rect,
        max_offset: f32,
        offset: f32,
        appearance: crate::ScrollbarAppearance,
    ) -> Option<Self> {
        if viewport.width <= 0.0 || viewport.height <= 0.0 || max_offset <= 0.0 {
            return None;
        }
        let gutter = appearance.effective_rail_width(viewport.width);
        let rail = Rect::new(
            viewport.x + viewport.width - gutter,
            viewport.y,
            gutter,
            viewport.height,
        );
        // Very short viewports keep a useful thumb instead of squeezing it
        // between two buttons. The buttons never reduce content height.
        let button_height = if appearance.buttons && viewport.height >= gutter * 4.0 {
            gutter
        } else {
            0.0
        };
        let up_button =
            (button_height > 0.0).then_some(Rect::new(rail.x, rail.y, gutter, button_height));
        let down_button = (button_height > 0.0).then_some(Rect::new(
            rail.x,
            rail.y + rail.height - button_height,
            gutter,
            button_height,
        ));
        let track = Rect::new(
            rail.x,
            rail.y + button_height,
            gutter,
            (rail.height - button_height * 2.0).max(0.0),
        );
        let fraction = viewport.height / (viewport.height + max_offset);
        let minimum = (if appearance.min_thumb_length.is_finite() {
            appearance.min_thumb_length
        } else {
            crate::ScrollbarAppearance::default().min_thumb_length
        })
        .max(1.0)
        .min((track.height / 2.0).max(1.0))
        .min(track.height);
        let thumb_height = (track.height * fraction).clamp(minimum, track.height);
        let travel = track.height - thumb_height;
        let thumb_width = (if appearance.thumb_width.is_finite() {
            appearance.thumb_width
        } else {
            crate::ScrollbarAppearance::default().thumb_width
        })
        .max(1.0)
        .min((gutter - 2.0).max(1.0))
        .min(gutter);
        let thumb = Rect::new(
            track.x + (gutter - thumb_width) / 2.0,
            track.y + offset.clamp(0.0, max_offset) / max_offset * travel,
            thumb_width,
            thumb_height,
        );
        Some(Self {
            rail,
            track,
            thumb,
            up_button,
            down_button,
            max_offset,
            placement: appearance.placement,
        })
    }

    pub(super) fn button_at(self, point: Point) -> Option<i8> {
        if self.up_button.is_some_and(|rect| rect.contains(point)) {
            Some(-1)
        } else if self.down_button.is_some_and(|rect| rect.contains(point)) {
            Some(1)
        } else {
            None
        }
    }

    pub(super) fn offset_at(self, pointer_y: f32, grab_y: f32) -> f32 {
        let travel = self.track.height - self.thumb.height;
        if travel <= 0.0 {
            return 0.0;
        }
        ((pointer_y - self.track.y - grab_y) / travel).clamp(0.0, 1.0) * self.max_offset
    }
}

pub(super) struct ScrollDrag {
    pub(super) id: String,
    pub(super) grab_y: f32,
}

pub(super) struct ScrollButtonHold {
    pub(super) id: String,
    pub(super) direction: i8,
    pub(super) repeat_at: Instant,
}

impl LayoutBuilder<'_> {
    pub(super) fn paint_scrollbar(
        &mut self,
        id: &str,
        geometry: ScrollbarGeometry,
        content: Rect,
        clip: Option<Rect>,
        ink: Color,
        appearance: crate::ScrollbarAppearance,
        visibility: crate::ScrollbarVisibility,
    ) {
        let visible = clip
            .and_then(|outer| outer.intersection(content))
            .or_else(|| clip.is_none().then_some(content));
        let Some(visible) = visible else {
            return;
        };
        let active = self.dragging_scrollbar == Some(id);
        let hovered = self.hovered_scrollbar == Some(id);
        let offset = self.scroll_offsets.get(id).copied().unwrap_or(0.0);
        let interacting = active || hovered || self.scroll_button_hold.is_some_and(|hold| hold.id == id);
        let opacity = self.scrollbar_opacity(id, offset, interacting, visibility, visible);
        if opacity <= 0.0 { return; }
        let ink = appearance.tint.unwrap_or(ink);
        let ink = Color { a: (f32::from(ink.a) * opacity).round() as u8, ..ink };
        let appearance = crate::ScrollbarAppearance { tint: None, ..appearance };
        let alpha = |opacity: u8| {
            Color::rgba(
                ink.r,
                ink.g,
                ink.b,
                ((u16::from(ink.a) * u16::from(opacity)) / 255) as u8,
            )
        };
        self.frame.commands.push(DrawCommand::PushClip(visible));
        // By default an overlay rail never paints a permanent gray stripe
        // behind content: its track appears only while interacting with it.
        let track = match appearance.track {
            crate::ScrollbarTrack::WhenActive => active || hovered,
            crate::ScrollbarTrack::Always => true,
            crate::ScrollbarTrack::Never => false,
        };
        if track {
            self.frame.commands.push(DrawCommand::FillRect {
                rect: geometry.track,
                color: appearance.track_color(ink),
                radius: appearance.corner(geometry.track.width),
            });
        }
        self.frame.commands.push(DrawCommand::FillRect {
            rect: geometry.thumb,
            color: appearance.thumb_color(ink, hovered, active),
            radius: appearance.corner(geometry.thumb.width),
        });
        let target = self
            .scroll_transitions
            .get(id)
            .map(ScalarTransition::target)
            .unwrap_or_else(|| self.scroll_offsets.get(id).copied().unwrap_or(0.0));
        for (direction, button) in [(-1, geometry.up_button), (1, geometry.down_button)] {
            let Some(button) = button else {
                continue;
            };
            let enabled = if direction < 0 {
                target > 0.001
            } else {
                target < geometry.max_offset - 0.001
            };
            let over = enabled && self.pointer.is_some_and(|point| button.contains(point));
            let pressed = over
                && self
                    .scroll_button_hold
                    .is_some_and(|hold| hold.id == id && hold.direction == direction);
            if over {
                self.frame.commands.push(DrawCommand::FillRect {
                    rect: button.inset(appearance.button_fill_inset()),
                    color: alpha(appearance.hover_track_opacity.saturating_mul(if pressed {
                        3
                    } else {
                        2
                    })),
                    radius: appearance.button_fill_radius(),
                });
            }
            let x = button.x + button.width / 2.0;
            let y = button.y + button.height / 2.0;
            let arrow = appearance.arrow_box();
            let half_width = (button.width * 0.25).min(arrow.width / 2.0);
            let half_height = (button.height * 0.15).min(arrow.height / 2.0);
            let direction = f32::from(direction);
            self.frame.commands.push(DrawCommand::FillTriangle {
                points: [
                    Point::new(x - half_width, y - direction * half_height),
                    Point::new(x + half_width, y - direction * half_height),
                    Point::new(x, y + direction * half_height),
                ],
                color: alpha(if !enabled {
                    appearance.idle_thumb_opacity / 3
                } else if pressed {
                    appearance.active_thumb_opacity
                } else if over {
                    appearance.hover_thumb_opacity
                } else {
                    appearance.idle_thumb_opacity
                }),
            });
        }
        self.frame.commands.push(DrawCommand::PopClip);
        self.add_region(id, geometry.rail, HitKind::Scrollbar, Some(visible));
        self.frame.scrollbars.insert(id.to_owned(), geometry);
    }
}
